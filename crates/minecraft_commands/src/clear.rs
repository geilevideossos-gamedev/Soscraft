use minecraftoss_player::inventory::{Inventory, ItemStack};

use crate::args::{Arg, ArgKind, ItemPredicate};
use crate::permissions::GAMEMASTERS;
use crate::reader::{CommandError, literal};
use crate::server::{Ctx, registered};
use crate::text::Text;
use crate::tracker::Tracker;
use crate::tree::Node;

pub const PLAYER_SLOTS: usize = 45;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemFilter {
    Any,
    Items(Vec<String>),
}

impl ItemFilter {
    pub fn matches(&self, id: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Items(ids) => ids.iter().any(|known| known == id),
        }
    }
}

pub fn clear_stack(
    slot: &mut Option<ItemStack>,
    filter: &ItemFilter,
    max: i32,
    dry_run: bool,
) -> i32 {
    let Some(stack) = slot.as_mut() else {
        return 0;
    };
    if stack.count == 0 || !filter.matches(&stack.id) {
        return 0;
    }
    let count = i32::from(stack.count);
    if dry_run {
        return count;
    }
    let removed = if max < 0 { count } else { max.min(count) };
    stack.count -= removed as u8;
    if stack.count == 0 {
        *slot = None;
    }
    removed
}

pub fn clear_stacks<'a>(
    slots: impl IntoIterator<Item = &'a mut Option<ItemStack>>,
    filter: &ItemFilter,
    max: i32,
    dry_run: bool,
) -> i32 {
    let mut count = 0;
    for slot in slots {
        count += clear_stack(slot, filter, max - count, dry_run);
    }
    count
}

pub fn clear_inventory(inventory: &mut Inventory, filter: &ItemFilter, max: i32) -> i32 {
    let dry_run = max == 0;
    let slots = inventory.slots.iter_mut().take(PLAYER_SLOTS);
    let mut count = clear_stacks(slots, filter, max, dry_run);
    count += clear_stacks(inventory.crafting.iter_mut(), filter, max - count, dry_run);
    count += clear_stack(&mut inventory.cursor, filter, max - count, dry_run);
    count
}

pub(crate) fn node() -> Node {
    let players = ArgKind::Players { single: false };
    let max_count = ArgKind::Int {
        min: 0,
        max: i32::MAX,
    };
    Node::literal("clear")
        .requires(GAMEMASTERS)
        .executes(clear)
        .then(
            Node::argument("targets", players).executes(clear).then(
                Node::argument("item", ArgKind::ItemPredicate)
                    .executes(clear)
                    .then(Node::argument("maxCount", max_count).executes(clear)),
            ),
        )
}

pub(crate) fn item_filter(ctx: &Ctx, name: &str) -> Result<ItemFilter, CommandError> {
    let Some(Arg::ItemPredicate { predicate, at }) = ctx.arg(name) else {
        return Ok(ItemFilter::Any);
    };
    match predicate {
        ItemPredicate::Any => Ok(ItemFilter::Any),
        ItemPredicate::Item(id) if registered(&ctx.server.registries().items, id) => {
            Ok(ItemFilter::Items(vec![id.clone()]))
        }
        ItemPredicate::Item(id) => {
            Err(CommandError::new("argument.item.id.invalid", vec![literal(id)]).at(*at))
        }
        ItemPredicate::Tag(id) => match ctx.server.data().item_tags.get(id) {
            Some(members) => Ok(ItemFilter::Items(members.clone())),
            None => Err(CommandError::new("arguments.item.tag.unknown", vec![literal(id)]).at(*at)),
        },
    }
}

fn clear(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let filter = item_filter(ctx, "item")?;
    let max = match ctx.arg("maxCount") {
        Some(Arg::Int(max)) => *max,
        _ => -1,
    };
    let mut tracker = Tracker::default();
    for player in ctx.targets("targets")? {
        let count = ctx.world.clear(player, &filter, max).unwrap_or(0);
        tracker.track(player, count);
    }
    if tracker.total == 0 {
        return Err(match tracker.single(false) {
            Some(only) => CommandError::new("clear.failed.single", vec![ctx.display_name(only)]),
            None => CommandError::new("clear.failed.multiple", vec![literal(tracker.count(false))]),
        });
    }
    let (single, multiple) = if max == 0 {
        ("commands.clear.test.single", "commands.clear.test.multiple")
    } else {
        (
            "commands.clear.success.single",
            "commands.clear.success.multiple",
        )
    };
    let text = match tracker.single(true) {
        Some(only) => Text::translate(single, vec![literal(tracker.total), ctx.display_name(only)]),
        None => Text::translate(
            multiple,
            vec![literal(tracker.total), literal(tracker.count(true))],
        ),
    };
    ctx.success(text, true);
    Ok(tracker.total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Recipient;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};
    use crate::text::RED;

    fn stack(id: &str, count: u8) -> Option<ItemStack> {
        Some(ItemStack::new(id, count))
    }

    fn filter(ids: &[&str]) -> ItemFilter {
        ItemFilter::Items(ids.iter().map(|id| (*id).to_owned()).collect())
    }

    fn inventory() -> Inventory {
        let mut inventory = Inventory::default();
        inventory.slots[0] = stack("minecraft:stone", 64);
        inventory.slots[5] = stack("minecraft:oak_log", 10);
        inventory.slots[20] = stack("minecraft:stone", 30);
        inventory.slots[36] = stack("minecraft:iron_boots", 1);
        inventory.slots[40] = stack("minecraft:stone", 2);
        inventory.slots[42] = stack("minecraft:stone", 9);
        inventory.crafting[1] = stack("minecraft:stone", 3);
        inventory.cursor = stack("minecraft:stone", 4);
        inventory
    }

    fn world_with(player: crate::protocol::PlayerId, inventory: Inventory) -> FakeWorld {
        FakeWorld {
            inventories: vec![(player, inventory)],
            ..FakeWorld::default()
        }
    }

    #[test]
    fn stacks_clear_in_order_up_to_the_maximum() {
        let stone = filter(&["minecraft:stone"]);
        let mut slots = [
            stack("minecraft:stone", 5),
            None,
            stack("minecraft:dirt", 7),
            stack("minecraft:stone", 6),
        ];
        assert_eq!(clear_stacks(slots.iter_mut(), &stone, 0, true), 11);
        assert_eq!(slots[0].as_ref().map(|s| s.count), Some(5));
        assert_eq!(clear_stacks(slots.iter_mut(), &stone, 8, false), 8);
        assert_eq!(slots[0], None);
        assert_eq!(slots[3].as_ref().map(|s| s.count), Some(3));
        assert_eq!(
            clear_stacks(slots.iter_mut(), &ItemFilter::Any, -1, false),
            10
        );
        assert!(slots.iter().all(Option::is_none));
    }

    #[test]
    fn the_inventory_clears_storage_armour_offhand_crafting_grid_and_cursor() {
        let stone = filter(&["minecraft:stone"]);
        let mut counted = inventory();
        assert_eq!(clear_inventory(&mut counted, &stone, 0), 112);
        assert_eq!(counted.slots[0].as_ref().map(|s| s.count), Some(64));
        let mut some = inventory();
        assert_eq!(clear_inventory(&mut some, &stone, 70), 70);
        assert_eq!(some.slots[0], None);
        assert_eq!(some.slots[20].as_ref().map(|s| s.count), Some(24));
        let mut all = inventory();
        assert_eq!(clear_inventory(&mut all, &stone, -1), 112);
        assert_eq!((all.cursor.clone(), all.crafting[1].clone()), (None, None));
        assert_eq!(all.slots[42], None);
        assert_eq!(all.slots[5].as_ref().map(|s| s.count), Some(10));
        let mut everything = inventory();
        assert_eq!(clear_inventory(&mut everything, &ItemFilter::Any, -1), 123);
        assert!(everything.slots[36].is_none());
    }

    #[test]
    fn clear_empties_the_grenade_slots() {
        let mut inventory = Inventory::default();
        inventory.slots[43] = stack("iw4l:frag_grenade", 32);
        inventory.slots[44] = stack("iw4l:flash_grenade", 20);
        let frags = filter(&["iw4l:frag_grenade"]);
        assert_eq!(clear_inventory(&mut inventory, &frags, 0), 32);
        assert_eq!(clear_inventory(&mut inventory, &frags, 5), 5);
        assert_eq!(inventory.slots[43].as_ref().map(|s| s.count), Some(27));
        assert_eq!(clear_inventory(&mut inventory, &ItemFilter::Any, -1), 47);
        assert!(inventory.slots.iter().all(Option::is_none));
    }

    #[test]
    fn clear_counts_with_zero_and_removes_otherwise() {
        let mut server = server();
        let mut world = world_with(HOST, inventory());
        let out = run(&mut server, &mut world, HOST, "clear @s minecraft:stone 0");
        assert_eq!(
            texts(&out),
            vec!["Found 112 matching item(s) on player Steve"]
        );
        let out = run(&mut server, &mut world, HOST, "clear @s stone 100");
        assert_eq!(texts(&out), vec!["Removed 100 item(s) from player Steve"]);
        let out = run(&mut server, &mut world, HOST, "clear Steve stone");
        assert_eq!(texts(&out), vec!["Removed 12 item(s) from player Steve"]);
        let out = run(&mut server, &mut world, HOST, "clear @s stone");
        assert_eq!(texts(&out), vec!["No items were found on player Steve"]);
        assert_eq!(out[0].2, RED);
        let out = run(&mut server, &mut world, HOST, "clear");
        assert_eq!(texts(&out), vec!["Removed 11 item(s) from player Steve"]);
        let out = run(&mut server, &mut world, HOST, "clear @s * 0");
        assert_eq!(texts(&out), vec!["No items were found on player Steve"]);
    }

    #[test]
    fn clear_reports_every_player_and_tags() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        server
            .data_mut()
            .item_tags
            .insert("minecraft:logs".into(), vec!["minecraft:oak_log".into()]);
        let mut world = world_with(HOST, inventory());
        world.inventories.push((FRIEND, Inventory::default()));
        let out = run(&mut server, &mut world, HOST, "clear @a #minecraft:logs 0");
        assert_eq!(
            texts(&out),
            vec!["Found 10 matching item(s) on player Steve"]
        );
        world.inventories[1].1.slots[3] = stack("minecraft:oak_log", 6);
        let out = run(&mut server, &mut world, HOST, "clear @a #logs");
        assert_eq!(texts(&out), vec!["Removed 16 item(s) from 2 players"]);
        assert_eq!(out[0].0, Recipient::Player(HOST));
        let out = run(&mut server, &mut world, HOST, "clear @a #logs");
        assert_eq!(texts(&out), vec!["No items were found on 2 players"]);
        let out = run(&mut server, &mut world, HOST, "clear @s #minecraft:nope");
        assert_eq!(
            texts(&out),
            vec![
                "Unknown item tag 'minecraft:nope'",
                "clear @s #minecraft:nope<--[HERE]"
            ]
        );
        let out = run(&mut server, &mut world, HOST, "clear Herobrine");
        assert_eq!(texts(&out), vec!["No player was found"]);
        let out = run(&mut server, &mut world, FRIEND, "clear");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
    }

    #[test]
    fn clear_checks_items_against_the_registry() {
        let mut server = server();
        server.registries_mut().items = vec!["minecraft:stone".into()];
        let mut world = world_with(HOST, inventory());
        let out = run(&mut server, &mut world, HOST, "clear @s notanitem");
        assert_eq!(
            texts(&out),
            vec![
                "Unknown item 'minecraft:notanitem'",
                "clear @s notanitem<--[HERE]"
            ]
        );
        let out = run(&mut server, &mut world, HOST, "clear @s stone[count=1]");
        assert_eq!(
            texts(&out)[0],
            "Expected whitespace to end one argument, but found trailing data"
        );
        let mut empty = FakeWorld::default();
        let out = run(&mut server, &mut empty, HOST, "clear");
        assert_eq!(texts(&out), vec!["No items were found on player Steve"]);
    }
}
