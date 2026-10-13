use minecraft_terrain::game_mode::PlayerMode;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use serde_json::Value;

pub const MENU_SLOTS: usize = 46;
pub const HOTBAR_MENU: usize = 36;
pub const DROP_STEP: u32 = 20;
pub const DROP_THRESHOLD: u32 = 1480;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DropThrottle {
    pub count: u32,
}

impl DropThrottle {
    pub fn tick(&mut self, ticks: u32) {
        self.count = self.count.saturating_sub(ticks);
    }

    pub fn allow(&mut self, name: &str) -> bool {
        if self.count >= DROP_THRESHOLD {
            diag::warn!(
                World,
                "Player {name} was dropping items too fast in creative mode, ignoring."
            );
            return false;
        }
        self.count += DROP_STEP;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreativeSlot {
    Menu(usize),
    Carried,
    Drop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    NotCreative,
    BadSlot(usize),
    UnknownItem(String),
    TooMany { count: u8, max: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Slot(usize),
    Crafting(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridClick {
    Pick { right: bool, shift: bool },
    Swap(usize),
    Throw { whole: bool },
    Clone,
}

pub type SetSlot = (CreativeSlot, Option<ItemStack>);

pub fn menu_place(menu: usize) -> Option<Place> {
    match menu {
        1..=4 => Some(Place::Crafting(menu - 1)),
        5..=8 => Some(Place::Slot(44 - menu)),
        9..=35 => Some(Place::Slot(menu)),
        36..=44 => Some(Place::Slot(menu - HOTBAR_MENU)),
        45 => Some(Place::Slot(40)),
        _ => None,
    }
}

pub fn menu_index(slot: usize) -> Option<usize> {
    match slot {
        0..=8 => Some(HOTBAR_MENU + slot),
        9..=35 => Some(slot),
        36..=39 => Some(44 - slot),
        40 => Some(45),
        _ => None,
    }
}

pub fn max_stack(recipes: &RecipeBook, stack: &ItemStack) -> u8 {
    stack
        .components
        .as_ref()
        .and_then(|components| components.get("minecraft:max_stack_size"))
        .and_then(Value::as_u64)
        .map_or_else(
            || recipes.max_stack(&stack.id),
            |max| max.clamp(1, 99) as u8,
        )
}

pub fn set_slot(
    mode: &PlayerMode,
    inventory: &mut Inventory,
    slot: CreativeSlot,
    stack: Option<ItemStack>,
    known: &dyn Fn(&str) -> bool,
) -> Result<Option<ItemStack>, Refused> {
    if mode.consumes_items() {
        return Err(Refused::NotCreative);
    }
    let stack = match stack.filter(|stack| stack.count > 0) {
        Some(mut stack) => {
            if !known(&stack.id) {
                return Err(Refused::UnknownItem(stack.id));
            }
            let max = max_stack(&inventory.recipes, &stack);
            if stack.count > max {
                return Err(Refused::TooMany {
                    count: stack.count,
                    max,
                });
            }
            stack.max = max;
            Some(stack)
        }
        None => None,
    };
    match slot {
        CreativeSlot::Menu(index) => {
            match menu_place(index).ok_or(Refused::BadSlot(index))? {
                Place::Slot(crate::offhand::OFFHAND)
                    if stack.as_ref().is_some_and(crate::guns::is_gun) =>
                {
                    return Err(Refused::BadSlot(index));
                }
                Place::Slot(i) => inventory.slots[i] = stack,
                Place::Crafting(i) => inventory.crafting[i] = stack,
            }
            Ok(None)
        }
        CreativeSlot::Carried => {
            inventory.cursor = stack;
            Ok(None)
        }
        CreativeSlot::Drop => Ok(stack),
    }
}

fn with_count(stack: &ItemStack, count: u8) -> ItemStack {
    ItemStack {
        count,
        ..stack.clone()
    }
}

pub fn grid_click(
    carried: Option<&ItemStack>,
    item: Option<&ItemStack>,
    click: GridClick,
    max: &dyn Fn(&ItemStack) -> u8,
) -> Vec<SetSlot> {
    match click {
        GridClick::Swap(slot) => menu_index(slot)
            .zip(item)
            .map(|(index, item)| (CreativeSlot::Menu(index), Some(with_count(item, max(item)))))
            .into_iter()
            .collect(),
        GridClick::Clone => item
            .filter(|_| carried.is_none())
            .map(|item| (CreativeSlot::Carried, Some(with_count(item, max(item)))))
            .into_iter()
            .collect(),
        GridClick::Throw { whole } => item
            .map(|item| {
                let count = if whole { max(item) } else { 1 };
                (CreativeSlot::Drop, Some(with_count(item, count)))
            })
            .into_iter()
            .collect(),
        GridClick::Pick { right, shift } => pick(carried, item, right, shift, max),
    }
}

fn pick(
    carried: Option<&ItemStack>,
    item: Option<&ItemStack>,
    right: bool,
    shift: bool,
    max: &dyn Fn(&ItemStack) -> u8,
) -> Vec<SetSlot> {
    let shrunk = |carried: &ItemStack| {
        let count = carried.count.saturating_sub(1);
        (count > 0).then(|| with_count(carried, count))
    };
    let next = match (carried, item) {
        (Some(carried), Some(item)) if carried.same_item(item) => {
            let limit = max(carried);
            if right {
                shrunk(carried)
            } else if shift {
                Some(with_count(carried, limit))
            } else if carried.count < limit {
                Some(with_count(carried, carried.count + 1))
            } else {
                return Vec::new();
            }
        }
        (None, Some(item)) => {
            let count = if shift { max(item) } else { item.count };
            Some(with_count(item, count))
        }
        (Some(carried), _) if right => shrunk(carried),
        (Some(_), _) => None,
        (None, None) => return Vec::new(),
    };
    vec![(CreativeSlot::Carried, next)]
}

pub fn trash_click(shift: bool) -> Vec<SetSlot> {
    if shift {
        (1..MENU_SLOTS)
            .map(|index| (CreativeSlot::Menu(index), None))
            .collect()
    } else {
        vec![(CreativeSlot::Carried, None)]
    }
}

pub fn hotbar_quick_move(slot: usize) -> Vec<SetSlot> {
    menu_index(slot)
        .filter(|_| slot < 9)
        .map(|index| (CreativeSlot::Menu(index), None))
        .into_iter()
        .collect()
}

pub fn load_hotbar(stacks: &[Option<ItemStack>]) -> Vec<SetSlot> {
    (0..9)
        .map(|i| {
            (
                CreativeSlot::Menu(HOTBAR_MENU + i),
                stacks.get(i).cloned().flatten(),
            )
        })
        .collect()
}

pub fn apply(
    mode: &PlayerMode,
    inventory: &mut Inventory,
    sets: Vec<SetSlot>,
    known: &dyn Fn(&str) -> bool,
) -> (Vec<ItemStack>, Vec<Refused>) {
    let mut dropped = Vec::new();
    let mut refused = Vec::new();
    for (slot, stack) in sets {
        match set_slot(mode, inventory, slot, stack, known) {
            Ok(drop) => dropped.extend(drop),
            Err(why) => refused.push(why),
        }
    }
    (dropped, refused)
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::game_mode::GameMode;

    fn creative() -> PlayerMode {
        PlayerMode::new(GameMode::Creative)
    }

    fn stone(count: u8) -> ItemStack {
        ItemStack::new("minecraft:stone", count)
    }

    fn any(_: &str) -> bool {
        true
    }

    fn sixty_four(_: &ItemStack) -> u8 {
        64
    }

    #[test]
    fn menu_slots_map_onto_the_inventory_like_the_inventory_menu() {
        assert_eq!(menu_place(0), None);
        assert_eq!(menu_place(1), Some(Place::Crafting(0)));
        assert_eq!(menu_place(4), Some(Place::Crafting(3)));
        assert_eq!(menu_place(5), Some(Place::Slot(39)));
        assert_eq!(menu_place(8), Some(Place::Slot(36)));
        assert_eq!(menu_place(9), Some(Place::Slot(9)));
        assert_eq!(menu_place(35), Some(Place::Slot(35)));
        assert_eq!(menu_place(36), Some(Place::Slot(0)));
        assert_eq!(menu_place(44), Some(Place::Slot(8)));
        assert_eq!(menu_place(45), Some(Place::Slot(40)));
        assert_eq!(menu_place(46), None);
        for slot in 0..=40 {
            assert_eq!(
                menu_index(slot).and_then(menu_place),
                Some(Place::Slot(slot))
            );
        }
        assert_eq!(menu_index(41), None);
    }

    #[test]
    fn the_server_sets_slots_only_for_creative_players_with_known_items_that_fit() {
        let mut inventory = Inventory::default();
        let survival = PlayerMode::new(GameMode::Survival);
        let known = |id: &str| id == "minecraft:stone";
        assert_eq!(
            set_slot(
                &survival,
                &mut inventory,
                CreativeSlot::Menu(36),
                Some(stone(1)),
                &known
            ),
            Err(Refused::NotCreative)
        );
        assert_eq!(
            set_slot(
                &creative(),
                &mut inventory,
                CreativeSlot::Menu(36),
                Some(ItemStack::new("minecraft:nope", 1)),
                &known
            ),
            Err(Refused::UnknownItem("minecraft:nope".to_owned()))
        );
        assert_eq!(
            set_slot(
                &creative(),
                &mut inventory,
                CreativeSlot::Menu(36),
                Some(stone(65)),
                &known
            ),
            Err(Refused::TooMany { count: 65, max: 64 })
        );
        assert_eq!(
            set_slot(
                &creative(),
                &mut inventory,
                CreativeSlot::Menu(0),
                Some(stone(1)),
                &known
            ),
            Err(Refused::BadSlot(0))
        );
        assert!(inventory.slots.iter().all(Option::is_none));
        let set = |inventory: &mut Inventory, slot, stack| {
            set_slot(&creative(), inventory, slot, stack, &known)
        };
        assert_eq!(
            set(&mut inventory, CreativeSlot::Menu(36), Some(stone(64))),
            Ok(None)
        );
        assert_eq!(inventory.slots[0], Some(stone(64)));
        assert_eq!(
            set(&mut inventory, CreativeSlot::Menu(5), Some(stone(1))),
            Ok(None)
        );
        assert_eq!(inventory.slots[39], Some(stone(1)));
        assert_eq!(
            set(&mut inventory, CreativeSlot::Menu(2), Some(stone(2))),
            Ok(None)
        );
        assert_eq!(inventory.crafting[1], Some(stone(2)));
        assert_eq!(
            set(&mut inventory, CreativeSlot::Carried, Some(stone(3))),
            Ok(None)
        );
        assert_eq!(inventory.cursor, Some(stone(3)));
        assert_eq!(
            set(&mut inventory, CreativeSlot::Drop, Some(stone(7))),
            Ok(Some(stone(7)))
        );
        assert_eq!(
            set(&mut inventory, CreativeSlot::Menu(36), Some(stone(0))),
            Ok(None)
        );
        assert_eq!(inventory.slots[0], None);
        assert_eq!(set(&mut inventory, CreativeSlot::Drop, None), Ok(None));
    }

    #[test]
    fn a_max_stack_size_component_caps_the_stack() {
        let mut inventory = Inventory::default();
        let mut sword = ItemStack::new("minecraft:stone", 2);
        sword.components = Some(serde_json::json!({"minecraft:max_stack_size": 1}));
        assert_eq!(
            set_slot(
                &creative(),
                &mut inventory,
                CreativeSlot::Menu(9),
                Some(sword),
                &any
            ),
            Err(Refused::TooMany { count: 2, max: 1 })
        );
    }

    #[test]
    fn a_gun_never_goes_into_the_creative_offhand() {
        let mut inventory = Inventory::default();
        for gun in ["iw4l:m4", "iw4l:riotshield"] {
            let stack = Some(ItemStack::new(gun, 1));
            let set = set_slot(&creative(), &mut inventory, CreativeSlot::Menu(45), stack, &any);
            assert_eq!(set, Err(Refused::BadSlot(45)));
            let stack = Some(ItemStack::new(gun, 1));
            let set = set_slot(&creative(), &mut inventory, CreativeSlot::Menu(36), stack, &any);
            assert_eq!(set, Ok(None));
        }
        assert!(inventory.slots[40].is_none());
        let grenade = Some(ItemStack::new("iw4l:frag_grenade", 3));
        let set = set_slot(&creative(), &mut inventory, CreativeSlot::Menu(45), grenade, &any);
        assert_eq!(set, Ok(None));
        assert_eq!(inventory.slots[40].as_ref().map(|s| s.count), Some(3));
    }

    #[test]
    fn clicking_the_grid_picks_one_and_shift_picks_a_stack_like_vanilla() {
        let one = stone(1);
        let pick = |carried: Option<&ItemStack>, right, shift| {
            grid_click(
                carried,
                Some(&one),
                GridClick::Pick { right, shift },
                &sixty_four,
            )
        };
        assert_eq!(
            pick(None, false, false),
            vec![(CreativeSlot::Carried, Some(stone(1)))]
        );
        assert_eq!(
            pick(None, true, false),
            vec![(CreativeSlot::Carried, Some(stone(1)))]
        );
        assert_eq!(
            pick(None, false, true),
            vec![(CreativeSlot::Carried, Some(stone(64)))]
        );
        assert_eq!(
            pick(Some(&stone(5)), false, false),
            vec![(CreativeSlot::Carried, Some(stone(6)))]
        );
        assert!(pick(Some(&stone(64)), false, false).is_empty());
        assert_eq!(
            pick(Some(&stone(5)), false, true),
            vec![(CreativeSlot::Carried, Some(stone(64)))]
        );
        assert_eq!(
            pick(Some(&stone(5)), true, false),
            vec![(CreativeSlot::Carried, Some(stone(4)))]
        );
        assert_eq!(
            pick(Some(&stone(1)), true, false),
            vec![(CreativeSlot::Carried, None)]
        );
        let dirt = ItemStack::new("minecraft:dirt", 12);
        assert_eq!(
            pick(Some(&dirt), false, false),
            vec![(CreativeSlot::Carried, None)]
        );
        assert_eq!(
            pick(Some(&dirt), true, false),
            vec![(
                CreativeSlot::Carried,
                Some(ItemStack::new("minecraft:dirt", 11))
            )]
        );
        let empty = |carried: Option<&ItemStack>, right| {
            grid_click(
                carried,
                None,
                GridClick::Pick {
                    right,
                    shift: false,
                },
                &sixty_four,
            )
        };
        assert_eq!(
            empty(Some(&dirt), false),
            vec![(CreativeSlot::Carried, None)]
        );
        assert!(empty(None, false).is_empty());
    }

    #[test]
    fn number_keys_throws_and_the_middle_button_give_whole_stacks() {
        let one = stone(1);
        assert_eq!(
            grid_click(None, Some(&one), GridClick::Swap(3), &sixty_four),
            vec![(CreativeSlot::Menu(39), Some(stone(64)))]
        );
        assert_eq!(
            grid_click(None, Some(&one), GridClick::Swap(40), &sixty_four),
            vec![(CreativeSlot::Menu(45), Some(stone(64)))]
        );
        assert!(grid_click(None, None, GridClick::Swap(3), &sixty_four).is_empty());
        assert_eq!(
            grid_click(
                Some(&stone(5)),
                Some(&one),
                GridClick::Throw { whole: false },
                &sixty_four
            ),
            vec![(CreativeSlot::Drop, Some(stone(1)))]
        );
        assert_eq!(
            grid_click(
                None,
                Some(&one),
                GridClick::Throw { whole: true },
                &sixty_four
            ),
            vec![(CreativeSlot::Drop, Some(stone(64)))]
        );
        assert_eq!(
            grid_click(None, Some(&one), GridClick::Clone, &sixty_four),
            vec![(CreativeSlot::Carried, Some(stone(64)))]
        );
        assert!(grid_click(Some(&one), Some(&one), GridClick::Clone, &sixty_four).is_empty());
    }

    #[test]
    fn the_trash_deletes_the_carried_stack_and_shift_empties_the_inventory() {
        assert_eq!(trash_click(false), vec![(CreativeSlot::Carried, None)]);
        let all = trash_click(true);
        assert_eq!(all.len(), 45);
        assert_eq!(all[0], (CreativeSlot::Menu(1), None));
        assert_eq!(all[44], (CreativeSlot::Menu(45), None));
        let mut inventory = Inventory::default();
        for slot in inventory.slots.iter_mut().take(41) {
            *slot = Some(stone(3));
        }
        inventory.crafting[0] = Some(stone(1));
        inventory.cursor = Some(stone(9));
        let (dropped, refused) = apply(&creative(), &mut inventory, all, &any);
        assert!(dropped.is_empty() && refused.is_empty());
        assert!(inventory.slots.iter().take(41).all(Option::is_none));
        assert_eq!(inventory.crafting[0], None);
        assert_eq!(inventory.cursor, Some(stone(9)));
    }

    #[test]
    fn shift_on_the_hotbar_of_an_item_tab_deletes_the_stack() {
        assert_eq!(hotbar_quick_move(2), vec![(CreativeSlot::Menu(38), None)]);
        assert!(hotbar_quick_move(9).is_empty());
    }

    #[test]
    fn loading_a_hotbar_sets_all_nine_slots() {
        let mut inventory = Inventory::default();
        inventory.slots[4] = Some(stone(5));
        let mut saved = vec![None; 9];
        saved[0] = Some(stone(64));
        let (_, refused) = apply(&creative(), &mut inventory, load_hotbar(&saved), &any);
        assert!(refused.is_empty());
        assert_eq!(inventory.slots[0], Some(stone(64)));
        assert_eq!(inventory.slots[4], None);
    }
}
