use minecraft_terrain::crafting::{self, Grid};
use minecraft_terrain::fork_items::{self, ForkItem, Kind};
use minecraft_terrain::gun_mods::{self, AKIMBO, GunRules};
use minecraftoss_player::inventory::{Inventory, ItemStack};

use crate::gun_names::GunContent;
use crate::guns::HOTBAR;

pub const ATTACH_HINT: &str = "Craft together with a gun to attach it";

pub struct Rules<'a, C>(pub &'a C);

impl<C: GunContent> Rules<'_, C> {
    fn index(&self, id: &str, attachments: &[String]) -> Option<u32> {
        let row = fork_items::get(id).filter(|row| row.kind == Kind::Gun)?;
        self.0.configuration(row.weapon?, attachments)
    }
}

impl<C: GunContent> GunRules for Rules<'_, C> {
    fn accepts(&self, id: &str, attachments: &[String]) -> bool {
        self.index(id, attachments).is_some()
    }

    fn clip_size(&self, id: &str, attachments: &[String]) -> Option<i32> {
        self.index(id, attachments)
            .and_then(|index| self.0.clip_size(index))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Craft {
    pub result: ItemStack,
    pub cells: Vec<Option<ItemStack>>,
    pub rounds: u32,
    pub ammo: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Cursor,
    Inventory,
    Hotbar(usize),
    Ground,
}

pub fn craft(cells: &[Option<ItemStack>], rules: &dyn GunRules) -> Option<Craft> {
    let filled: Vec<usize> = (0..cells.len()).filter(|&i| cells[i].is_some()).collect();
    let guns: Vec<usize> = filled
        .iter()
        .copied()
        .filter(|&i| cells[i].as_ref().is_some_and(gun_mods::is_gun))
        .collect();
    match (guns.as_slice(), filled.len()) {
        ([gun], 1) => split(cells, *gun, rules),
        ([gun], _) => attach(cells, *gun, &filled, rules),
        ([first, second], 2) => pair(cells, (*first, *second), rules),
        _ => None,
    }
}

fn single(cells: &[Option<ItemStack>], index: usize) -> Option<&ItemStack> {
    cells[index].as_ref().filter(|stack| stack.count == 1)
}

fn used(cells: &[Option<ItemStack>], taken: &[usize]) -> Vec<Option<ItemStack>> {
    let mut left = cells.to_vec();
    for &index in taken {
        if let Some(stack) = left[index].as_mut() {
            stack.count = stack.count.saturating_sub(1);
            if stack.count == 0 {
                left[index] = None;
            }
        }
    }
    left
}

fn ammo_of(gun: &ItemStack) -> Option<&'static str> {
    fork_items::get(&gun.id).and_then(|row| row.ammo)
}

fn choices(rows: &[&ForkItem]) -> Vec<Vec<String>> {
    rows.iter().fold(vec![Vec::new()], |sets, row| {
        sets.iter()
            .flat_map(|set| {
                row.aliases.iter().map(move |alias| {
                    let mut next = set.clone();
                    next.push((*alias).to_owned());
                    next
                })
            })
            .collect()
    })
}

fn attach(
    cells: &[Option<ItemStack>],
    gun: usize,
    filled: &[usize],
    rules: &dyn GunRules,
) -> Option<Craft> {
    let stack = single(cells, gun)?;
    let mut rows: Vec<&ForkItem> = Vec::new();
    for &index in filled.iter().filter(|&&index| index != gun) {
        let row = cells[index]
            .as_ref()
            .and_then(|item| fork_items::get(&item.id))
            .filter(|row| row.kind == Kind::Attachment)?;
        if rows.iter().any(|known| known.id == row.id) {
            return None;
        }
        rows.push(row);
    }
    let current = gun_mods::attachments(stack);
    if rows.is_empty() || current.len() + rows.len() > 2 {
        return None;
    }
    let doubled = current.iter().any(|name| {
        fork_items::by_alias(name).is_some_and(|on| rows.iter().any(|row| row.id == on.id))
    });
    if doubled {
        return None;
    }
    let chosen = choices(&rows)
        .into_iter()
        .map(|names| {
            let mut all = current.clone();
            all.extend(names);
            all
        })
        .find(|all| rules.accepts(&stack.id, all))?;
    let refit = gun_mods::refit(stack, &chosen, rules);
    Some(Craft {
        result: refit.stack,
        cells: used(cells, filled),
        rounds: refit.rounds,
        ammo: ammo_of(stack),
    })
}

fn pair(
    cells: &[Option<ItemStack>],
    (first, second): (usize, usize),
    rules: &dyn GunRules,
) -> Option<Craft> {
    let (right, left) = (single(cells, first)?, single(cells, second)?);
    let current = gun_mods::attachments(right);
    let sorted = |stack: &ItemStack| {
        let mut names = gun_mods::attachments(stack);
        names.sort();
        names
    };
    if right.id != left.id
        || sorted(right) != sorted(left)
        || current.len() >= 2
        || current.iter().any(|name| name == AKIMBO)
    {
        return None;
    }
    let mut paired = current.clone();
    paired.push(AKIMBO.to_owned());
    if !rules.accepts(&right.id, &paired) {
        return None;
    }
    let [clip, _, alt_clip] = gun_mods::clips(right);
    let both = gun_mods::with_state(
        right,
        &current,
        [clip, gun_mods::clip(left, "clip"), alt_clip],
    );
    let refit = gun_mods::refit(&both, &paired, rules);
    Some(Craft {
        result: refit.stack,
        cells: used(cells, &[first, second]),
        rounds: refit.rounds,
        ammo: ammo_of(right),
    })
}

fn split(cells: &[Option<ItemStack>], gun: usize, rules: &dyn GunRules) -> Option<Craft> {
    let stack = single(cells, gun)?;
    let current = gun_mods::attachments(stack);
    if !current.iter().any(|name| name == AKIMBO) {
        return None;
    }
    let kept: Vec<String> = current.into_iter().filter(|name| name != AKIMBO).collect();
    if !rules.accepts(&stack.id, &kept) {
        return None;
    }
    let [clip, clip_left, alt_clip] = gun_mods::clips(stack);
    let mut left = cells.to_vec();
    left[gun] = Some(gun_mods::with_state(stack, &kept, [clip_left, None, None]));
    Some(Craft {
        result: gun_mods::with_state(stack, &kept, [clip, None, alt_clip]),
        cells: left,
        rounds: 0,
        ammo: ammo_of(stack),
    })
}

fn cells_mut(inventory: &mut Inventory, grid: Grid) -> &mut [Option<ItemStack>] {
    match grid {
        Grid::Inventory => &mut inventory.crafting[..],
        Grid::Table => &mut inventory.workbench[..],
    }
}

pub fn result(inventory: &Inventory, grid: Grid, rules: &dyn GunRules) -> Option<ItemStack> {
    let cells = match grid {
        Grid::Inventory => &inventory.crafting[..],
        Grid::Table => &inventory.workbench[..],
    };
    craft(cells, rules).map(|craft| craft.result)
}

pub fn take(
    inventory: &mut Inventory,
    grid: Grid,
    rules: &dyn GunRules,
    into: Place,
) -> Option<Vec<ItemStack>> {
    let craft = craft(cells_mut(inventory, grid), rules)?;
    let mut thrown = Vec::new();
    let placed = match into {
        Place::Cursor if inventory.cursor.is_none() => {
            inventory.cursor = Some(craft.result.clone());
            true
        }
        Place::Inventory => {
            let mut stack = craft.result.clone();
            crafting::move_into(&mut inventory.slots, &mut stack, &crafting::result_order());
            stack.count == 0
        }
        Place::Hotbar(slot) if slot < HOTBAR && inventory.slots[slot].is_none() => {
            inventory.slots[slot] = Some(craft.result.clone());
            true
        }
        Place::Ground if inventory.cursor.is_none() => {
            thrown.push(craft.result.clone());
            true
        }
        _ => false,
    };
    if !placed {
        return Some(thrown);
    }
    cells_mut(inventory, grid).clone_from_slice(&craft.cells);
    inventory.notice_item_changed(&craft.result);
    if let Some(ammo) = craft.ammo {
        let max = u32::from(inventory.recipes.max_stack(ammo).max(1));
        let mut rounds = craft.rounds;
        while rounds > 0 {
            let count = rounds.min(max);
            rounds -= count;
            let stack = inventory.recipes.stack(ammo, count as u8);
            thrown.extend(inventory.add_item(stack, 0));
        }
    }
    Some(thrown)
}

pub fn unmapped<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out: Vec<String> = names
        .into_iter()
        .filter(|name| *name != AKIMBO && fork_items::by_alias(name).is_none())
        .map(str::to_owned)
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    struct Fake;

    const WEAPONS: [(&str, &[&str], i32); 13] = [
        ("iw4:weapon/m4_mp", &[], 30),
        ("iw4:weapon/m4_mp", &["reflex"], 30),
        ("iw4:weapon/m4_mp", &["acog"], 30),
        ("iw4:weapon/m4_mp", &["fmj"], 30),
        ("iw4:weapon/m4_mp", &["silencer"], 20),
        ("iw4:weapon/m4_mp", &["reflex", "fmj"], 30),
        ("iw4:weapon/m4_mp", &["reflex", "silencer"], 20),
        ("iw4:weapon/m4_mp", &["xmags"], 45),
        ("iw4:weapon/usp_mp", &[], 12),
        ("iw4:weapon/usp_mp", &["akimbo"], 12),
        ("iw4:weapon/usp_mp", &["fmj"], 12),
        ("iw4:weapon/usp_mp", &["akimbo", "fmj"], 12),
        ("t5:weapon/galil_mp", &["elbit"], 35),
    ];

    impl GunContent for Fake {
        fn configuration(&self, weapon: &str, attachments: &[String]) -> Option<u32> {
            let mut wanted = attachments.to_vec();
            wanted.sort();
            let index = WEAPONS.iter().position(|(key, with, _)| {
                let mut with: Vec<String> = with.iter().map(|name| name.to_string()).collect();
                with.sort();
                *key == weapon && with == wanted
            })?;
            u32::try_from(index).ok()
        }

        fn selection(&self, _index: u32) -> Option<(String, Vec<String>)> {
            None
        }

        fn clip_size(&self, index: u32) -> Option<i32> {
            WEAPONS.get(index as usize).map(|(_, _, clip)| *clip)
        }
    }

    fn gun(id: &str, state: Value) -> ItemStack {
        let mut stack = ItemStack::new(id, 1);
        stack.max = 1;
        stack.components = Some(json!({"minecraft:custom_data": {"iw4l": state}}));
        stack
    }

    fn item(id: &str, count: u8) -> Option<ItemStack> {
        Some(ItemStack::new(id, count))
    }

    fn grid(cells: &[(usize, Option<ItemStack>)]) -> Vec<Option<ItemStack>> {
        let mut grid = vec![None; 9];
        for (index, stack) in cells {
            grid[*index] = stack.clone();
        }
        grid
    }

    fn state(stack: &ItemStack) -> Value {
        stack.components.as_ref().unwrap()["minecraft:custom_data"]["iw4l"].clone()
    }

    #[test]
    fn attaching_keeps_the_clip_the_name_and_the_other_attachments() {
        let mut named = gun("iw4l:m4", json!({"attachments": ["reflex"], "clip": 27}));
        named.components.as_mut().unwrap()["minecraft:custom_name"] = json!("Old Faithful");
        let cells = grid(&[(0, Some(named)), (4, item("iw4l:fmj", 3))]);
        let craft = craft(&cells, &Rules(&Fake)).unwrap();
        assert_eq!(
            craft.result.components,
            Some(json!({
                "minecraft:custom_name": "Old Faithful",
                "minecraft:custom_data": {"iw4l": {"attachments": ["reflex", "fmj"], "clip": 27}}
            }))
        );
        assert_eq!((craft.result.count, craft.rounds), (1, 0));
        assert_eq!(craft.cells[0], None);
        assert_eq!(craft.cells[4].as_ref().map(|stack| stack.count), Some(2));
        let bare = grid(&[
            (8, Some(gun("iw4l:m4", json!({})))),
            (2, item("iw4l:red_dot_sight", 1)),
        ]);
        let craft = super::craft(&bare, &Rules(&Fake)).unwrap();
        assert_eq!(state(&craft.result), json!({"attachments": ["reflex"]}));
        assert!(craft.cells.iter().all(Option::is_none));
    }

    #[test]
    fn a_smaller_clip_gives_the_extra_rounds_back_as_ammo() {
        let mut inventory = Inventory::default();
        inventory.crafting[0] = Some(gun("iw4l:m4", json!({"clip": 28})));
        inventory.crafting[3] = item("iw4l:silencer", 1);
        let thrown = take(
            &mut inventory,
            Grid::Inventory,
            &Rules(&Fake),
            Place::Cursor,
        )
        .unwrap();
        assert!(thrown.is_empty());
        let carried = inventory.cursor.clone().unwrap();
        assert_eq!(
            state(&carried),
            json!({"attachments": ["silencer"], "clip": 20})
        );
        assert!(inventory.crafting.iter().all(Option::is_none));
        let ammo: Vec<(String, u8)> = inventory
            .slots
            .iter()
            .flatten()
            .map(|stack| (stack.id.clone(), stack.count))
            .collect();
        assert_eq!(ammo, [("iw4l:rifle_ammo".to_owned(), 8)]);
    }

    #[test]
    fn two_sights_a_third_attachment_or_the_same_one_twice_give_nothing() {
        let rules = Rules(&Fake);
        let m4 = |state| Some(gun("iw4l:m4", state));
        let two_sights = grid(&[
            (0, m4(json!({}))),
            (1, item("iw4l:red_dot_sight", 1)),
            (2, item("iw4l:acog_scope", 1)),
        ]);
        assert_eq!(craft(&two_sights, &rules), None);
        let third = grid(&[
            (0, m4(json!({"attachments": ["reflex", "fmj"]}))),
            (1, item("iw4l:silencer", 1)),
        ]);
        assert_eq!(craft(&third, &rules), None);
        let three = grid(&[
            (0, m4(json!({}))),
            (1, item("iw4l:red_dot_sight", 1)),
            (2, item("iw4l:fmj", 1)),
            (3, item("iw4l:silencer", 1)),
        ]);
        assert_eq!(craft(&three, &rules), None);
        let again = grid(&[
            (0, m4(json!({"attachments": ["reflex"]}))),
            (1, item("iw4l:red_dot_sight", 1)),
        ]);
        assert_eq!(craft(&again, &rules), None);
        let twice = grid(&[
            (0, m4(json!({}))),
            (1, item("iw4l:fmj", 1)),
            (2, item("iw4l:fmj", 1)),
        ]);
        assert_eq!(craft(&twice, &rules), None);
        let dirt = grid(&[
            (0, m4(json!({}))),
            (1, item("iw4l:fmj", 1)),
            (2, item("minecraft:dirt", 1)),
        ]);
        assert_eq!(craft(&dirt, &rules), None);
        let alone = grid(&[(0, m4(json!({"clip": 5})))]);
        assert_eq!(craft(&alone, &rules), None);
    }

    #[test]
    fn a_t5_gun_takes_the_attachment_under_its_own_name() {
        let cells = grid(&[
            (0, Some(gun("iw4l:t5_galil", json!({})))),
            (1, item("iw4l:red_dot_sight", 1)),
        ]);
        let craft = craft(&cells, &Rules(&Fake)).unwrap();
        assert_eq!(state(&craft.result), json!({"attachments": ["elbit"]}));
    }

    #[test]
    fn two_identical_guns_make_an_akimbo_pair_with_both_clips() {
        let rules = Rules(&Fake);
        let usp = |state| Some(gun("iw4l:usp", state));
        let cells = grid(&[
            (1, usp(json!({"attachments": ["fmj"], "clip": 10}))),
            (5, usp(json!({"attachments": ["fmj"], "clip": 7}))),
        ]);
        let craft = craft(&cells, &rules).unwrap();
        assert_eq!(
            state(&craft.result),
            json!({"attachments": ["fmj", "akimbo"], "clip": 10, "clip_left": 7})
        );
        assert!(craft.cells.iter().all(Option::is_none));
        let full = grid(&[(0, usp(json!({"clip": -1}))), (1, usp(json!({})))]);
        assert_eq!(
            state(&super::craft(&full, &rules).unwrap().result),
            json!({"attachments": ["akimbo"], "clip": 12})
        );
        let different = grid(&[
            (0, usp(json!({"attachments": ["fmj"]}))),
            (1, usp(json!({}))),
        ]);
        assert_eq!(super::craft(&different, &rules), None);
        let m4s = grid(&[
            (0, Some(gun("iw4l:m4", json!({})))),
            (1, Some(gun("iw4l:m4", json!({})))),
        ]);
        assert_eq!(
            super::craft(&m4s, &rules),
            None,
            "the M4A1 offers no akimbo"
        );
        let mixed = grid(&[(0, usp(json!({}))), (1, Some(gun("iw4l:m4", json!({}))))]);
        assert_eq!(super::craft(&mixed, &rules), None);
    }

    #[test]
    fn an_akimbo_gun_alone_splits_and_the_second_gun_stays_in_its_cell() {
        let mut inventory = Inventory::default();
        let pair = gun(
            "iw4l:usp",
            json!({"attachments": ["akimbo", "fmj"], "clip": 10, "clip_left": 7}),
        );
        inventory.workbench[4] = Some(pair);
        let thrown = take(&mut inventory, Grid::Table, &Rules(&Fake), Place::Cursor).unwrap();
        assert!(thrown.is_empty());
        let taken = inventory.cursor.clone().unwrap();
        assert_eq!(state(&taken), json!({"attachments": ["fmj"], "clip": 10}));
        let left = inventory.workbench[4].clone().unwrap();
        assert_eq!(state(&left), json!({"attachments": ["fmj"], "clip": 7}));
        assert_eq!(result(&inventory, Grid::Table, &Rules(&Fake)), None);
    }

    #[test]
    fn the_result_goes_to_the_hotbar_the_inventory_or_the_ground_and_never_onto_a_full_cursor() {
        let rules = Rules(&Fake);
        let ready = || {
            let mut inventory = Inventory::default();
            inventory.crafting[0] = Some(gun("iw4l:m4", json!({})));
            inventory.crafting[1] = item("iw4l:fmj", 1);
            inventory
        };
        let mut busy = ready();
        busy.cursor = item("minecraft:dirt", 1);
        assert_eq!(
            take(&mut busy, Grid::Inventory, &rules, Place::Cursor),
            Some(Vec::new())
        );
        assert!(busy.crafting[0].is_some(), "nothing was crafted");
        let mut hotbar = ready();
        take(&mut hotbar, Grid::Inventory, &rules, Place::Hotbar(3)).unwrap();
        assert_eq!(
            hotbar.slots[3].as_ref().map(state),
            Some(json!({"attachments": ["fmj"]}))
        );
        let mut offhand = ready();
        take(&mut offhand, Grid::Inventory, &rules, Place::Hotbar(40)).unwrap();
        assert!(offhand.slots[40].is_none() && offhand.crafting[0].is_some());
        let mut shifted = ready();
        take(&mut shifted, Grid::Inventory, &rules, Place::Inventory).unwrap();
        assert!(
            shifted.slots[8].is_some(),
            "shift-click fills the hotbar from the right like vanilla"
        );
        let mut thrown = ready();
        let dropped = take(&mut thrown, Grid::Inventory, &rules, Place::Ground).unwrap();
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].id, "iw4l:m4");
        let mut plain = Inventory::default();
        plain.crafting[0] = item("minecraft:oak_log", 1);
        assert_eq!(
            take(&mut plain, Grid::Inventory, &rules, Place::Cursor),
            None
        );
    }

    #[test]
    fn a_click_on_the_result_crafts_through_the_menu() {
        use minecraft_net::messages::{MenuAction, MenuSlot};
        use minecraft_terrain::containers::Containers;
        use minecraft_terrain::game_mode::PlayerMode;
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let id = minecraft_commands::PlayerId(0);
        chat.join(id, "Steve");
        let mut player = crate::player::Player::new(
            chat.profile(id).unwrap(),
            PlayerMode::default(),
            crate::survival::Survival::default(),
            Inventory::default(),
        );
        player.inventory.crafting[2] = Some(gun("iw4l:m4", json!({"clip": 30})));
        player.inventory.crafting[3] = item("iw4l:silencer", 2);
        let stations = minecraft_terrain::stations::StationBook::default();
        let rules = Rules(&Fake);
        let books = crate::menus::Books {
            stations: &stations,
            enchantments: None,
            guns: Some(&rules),
        };
        let click = MenuAction::Slot {
            slot: MenuSlot::Result,
            right: false,
            shift: false,
        };
        let mut containers = Containers::default();
        let clicked = crate::menus::apply(&mut player, &mut containers, books, &click, 0.5);
        assert!(clicked.thrown.is_empty());
        let carried = player.inventory.cursor.clone().unwrap();
        assert_eq!(
            state(&carried),
            json!({"attachments": ["silencer"], "clip": 20})
        );
        assert_eq!(
            player.inventory.crafting[3]
                .as_ref()
                .map(|stack| stack.count),
            Some(1)
        );
        let rounds: u32 = player
            .inventory
            .slots
            .iter()
            .flatten()
            .filter(|stack| stack.id == "iw4l:rifle_ammo")
            .map(|stack| u32::from(stack.count))
            .sum();
        assert_eq!(rounds, 10);
        let without = crate::menus::Books {
            guns: None,
            ..books
        };
        player.inventory.cursor = None;
        player.inventory.crafting[2] = Some(gun("iw4l:m4", json!({})));
        crate::menus::apply(&mut player, &mut containers, without, &click, 0.5);
        assert!(
            player.inventory.cursor.is_none(),
            "no MW2 content, no special recipe"
        );
    }

    #[test]
    fn content_attachment_names_without_an_item_are_listed() {
        let presets = include_str!("../../frame/src/class_presets.rs");
        let names: Vec<&str> = presets
            .lines()
            .filter(|line| line.contains("_attachments: &["))
            .flat_map(|line| line.split('"').skip(1).step_by(2))
            .collect();
        assert!(names.contains(&"rf") && names.contains(&"hamrhybrid"));
        let missing = unmapped(names.iter().copied());
        eprintln!("class preset attachments with no Minecraft item: {missing:?}");
        for mapped in [
            "rf",
            "rof",
            "auto",
            "reflex",
            "hamrhybrid",
            "silencer03",
            "akimbo",
        ] {
            assert!(!missing.iter().any(|name| name == mapped), "{mapped}");
        }
    }
}
