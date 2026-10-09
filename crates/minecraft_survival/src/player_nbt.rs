use std::collections::BTreeMap;

use minecraft_terrain::game_mode::PlayerMode;
use minecraft_terrain::item_nbt::{components_tag, saved_components, tag_json};
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::ItemStack;

use crate::guns::{HOTBAR, WEAPON_PREFIX};

pub const STORAGE: usize = 36;
pub const SLOTS: usize = 43;
const MAX_COUNT: u8 = 99;
pub const EQUIPMENT: [(usize, &str); 7] = [
    (36, "feet"),
    (37, "legs"),
    (38, "chest"),
    (39, "head"),
    (40, "offhand"),
    (41, "body"),
    (42, "saddle"),
];
const OWNED: [&str; 6] = [
    "playerGameType",
    "previousPlayerGameType",
    "abilities",
    "Inventory",
    "equipment",
    "SelectedItemSlot",
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerData {
    pub mode: Option<PlayerMode>,
    pub inventory: Option<Vec<Option<ItemStack>>>,
    pub selected: usize,
}

impl PlayerData {
    pub fn read(tag: &Tag, recipes: &RecipeBook) -> Self {
        let inventory = tag.get("Inventory").map(|list| {
            let mut slots = slots(list, STORAGE, recipes);
            slots.resize(SLOTS, None);
            for (slot, name) in EQUIPMENT {
                slots[slot] = tag
                    .get("equipment")
                    .and_then(|equipment| equipment.get(name))
                    .and_then(|stack| stack_of(stack, recipes));
            }
            slots
        });
        Self {
            mode: PlayerMode::from_tag(tag),
            inventory,
            selected: tag
                .get("SelectedItemSlot")
                .and_then(Tag::as_i64)
                .and_then(|slot| usize::try_from(slot).ok())
                .filter(|slot| *slot < HOTBAR)
                .unwrap_or(0),
        }
    }

    pub fn entries(&self) -> BTreeMap<String, Tag> {
        let mut entries = BTreeMap::new();
        if let Some(Tag::Compound(mode)) = self.mode.map(|mode| mode.to_tag()) {
            entries.extend(mode);
        }
        if let Some(inventory) = &self.inventory {
            entries.insert(
                "Inventory".to_owned(),
                slot_list(inventory.iter().take(STORAGE)),
            );
            let equipment: BTreeMap<String, Tag> = EQUIPMENT
                .iter()
                .filter_map(|&(slot, name)| {
                    let stack = stack_tag(inventory.get(slot)?.as_ref()?)?;
                    Some((name.to_owned(), stack))
                })
                .collect();
            if !equipment.is_empty() {
                entries.insert("equipment".to_owned(), Tag::Compound(equipment));
            }
        }
        entries.insert(
            "SelectedItemSlot".to_owned(),
            Tag::Int(self.selected as i32),
        );
        entries
    }

    pub fn merged(&self, previous: Option<Tag>) -> Tag {
        let mut merged = match previous {
            Some(Tag::Compound(map)) => map,
            _ => BTreeMap::new(),
        };
        for key in OWNED {
            merged.remove(key);
        }
        merged.extend(self.entries());
        Tag::Compound(merged)
    }
}

pub fn stack_tag(stack: &ItemStack) -> Option<Tag> {
    if stack.count == 0 || stack.id.starts_with(WEAPON_PREFIX) {
        return None;
    }
    let mut map = BTreeMap::new();
    map.insert("id".to_owned(), Tag::String(stack.id.clone()));
    map.insert(
        "count".to_owned(),
        Tag::Int(i32::from(stack.count.min(MAX_COUNT))),
    );
    if let Some(components) = stack.components.as_ref().and_then(components_tag) {
        map.insert("components".to_owned(), components);
    }
    Some(Tag::Compound(map))
}

pub fn stack_of(tag: &Tag, recipes: &RecipeBook) -> Option<ItemStack> {
    let id = tag.get("id")?.as_str()?;
    let count = tag.get("count").map_or(Some(1), Tag::as_i64)?;
    if count <= 0 {
        return None;
    }
    let mut stack = recipes.stack(id, count.min(i64::from(u8::MAX)) as u8);
    stack.components = tag
        .get("components")
        .and_then(saved_components)
        .map(|components| tag_json(&components));
    Some(stack)
}

pub fn slot_list<'a>(cells: impl Iterator<Item = &'a Option<ItemStack>>) -> Tag {
    Tag::List(
        cells
            .enumerate()
            .filter_map(|(slot, cell)| {
                let Tag::Compound(mut map) = stack_tag(cell.as_ref()?)? else {
                    return None;
                };
                map.insert("Slot".to_owned(), Tag::Byte(slot as u8 as i8));
                Some(Tag::Compound(map))
            })
            .collect(),
    )
}

pub fn slots(list: &Tag, size: usize, recipes: &RecipeBook) -> Vec<Option<ItemStack>> {
    let mut slots = vec![None; size];
    for entry in list.as_list().unwrap_or(&[]) {
        let Some(slot) = entry
            .get("Slot")
            .and_then(Tag::as_i64)
            .map(|slot| usize::from(slot as u8))
        else {
            continue;
        };
        if slot < size {
            slots[slot] = stack_of(entry, recipes);
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::game_mode::GameMode;
    use serde_json::json;

    fn stack(id: &str, count: u8) -> ItemStack {
        ItemStack::new(id, count)
    }

    fn compound(entries: Vec<(&str, Tag)>) -> Tag {
        Tag::Compound(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    fn filled() -> Vec<Option<ItemStack>> {
        let mut slots = vec![None; SLOTS];
        slots[0] = Some(stack("minecraft:cobblestone", 64));
        let mut sword = stack("minecraft:diamond_sword", 1);
        sword.max = 1;
        sword.components = Some(json!({
            "minecraft:enchantments": {"minecraft:sharpness": 5},
            "minecraft:damage": 12
        }));
        slots[8] = Some(sword);
        slots[35] = Some(stack("minecraft:torch", 3));
        slots[36] = Some(stack("minecraft:diamond_boots", 1));
        slots[37] = Some(stack("minecraft:diamond_leggings", 1));
        slots[38] = Some(stack("minecraft:diamond_chestplate", 1));
        slots[39] = Some(stack("minecraft:diamond_helmet", 1));
        slots[40] = Some(stack("minecraft:shield", 1));
        slots
    }

    #[test]
    fn storage_slots_go_to_inventory_with_a_slot_byte_and_armour_to_equipment() {
        let data = PlayerData {
            inventory: Some(filled()),
            selected: 4,
            ..PlayerData::default()
        };
        let entries = data.entries();
        let inventory = entries["Inventory"].as_list().unwrap();
        assert_eq!(inventory.len(), 3);
        assert_eq!(
            inventory[0],
            compound(vec![
                ("Slot", Tag::Byte(0)),
                ("count", Tag::Int(64)),
                ("id", Tag::String("minecraft:cobblestone".to_owned())),
            ])
        );
        assert_eq!(inventory[1].get("Slot"), Some(&Tag::Byte(8)));
        assert_eq!(
            inventory[1]
                .get("components")
                .and_then(|c| c.get("minecraft:enchantments"))
                .and_then(|e| e.get("minecraft:sharpness")),
            Some(&Tag::Int(5))
        );
        assert_eq!(inventory[2].get("Slot"), Some(&Tag::Byte(35)));
        let equipment = entries["equipment"].as_compound().unwrap();
        let names: Vec<&str> = equipment.keys().map(String::as_str).collect();
        assert_eq!(names, ["chest", "feet", "head", "legs", "offhand"]);
        assert_eq!(
            equipment["head"].get("id"),
            Some(&Tag::String("minecraft:diamond_helmet".to_owned()))
        );
        assert_eq!(equipment["offhand"].get("count"), Some(&Tag::Int(1)));
        assert_eq!(entries["SelectedItemSlot"], Tag::Int(4));
    }

    #[test]
    fn the_inventory_comes_back_from_its_file_in_the_same_slots() {
        let mut mode = PlayerMode::new(GameMode::Creative);
        mode.abilities.flying = true;
        let mut slots = filled();
        slots[41] = Some(stack("minecraft:diamond_horse_armor", 1));
        slots[42] = Some(stack("minecraft:saddle", 1));
        let data = PlayerData {
            mode: Some(mode),
            inventory: Some(slots),
            selected: 8,
        };
        let tag = data.merged(None);
        let recipes = RecipeBook::default();
        let read = PlayerData::read(&tag, &recipes);
        assert_eq!(read.mode, Some(mode));
        assert_eq!(read.selected, 8);
        let restored = read.inventory.unwrap();
        assert_eq!(restored.len(), SLOTS);
        for (slot, (had, back)) in data.inventory.unwrap().iter().zip(&restored).enumerate() {
            assert_eq!(
                had.as_ref().map(|s| (&s.id, s.count, &s.components)),
                back.as_ref().map(|s| (&s.id, s.count, &s.components)),
                "slot {slot}"
            );
        }
        assert_eq!(
            tag.get("equipment")
                .and_then(|e| e.get("saddle"))
                .and_then(|s| s.get("id")),
            Some(&Tag::String("minecraft:saddle".to_owned()))
        );
        assert_eq!(
            tag.get("equipment")
                .and_then(|e| e.get("body"))
                .and_then(|s| s.get("id")),
            Some(&Tag::String("minecraft:diamond_horse_armor".to_owned()))
        );
    }

    #[test]
    fn guns_and_empty_stacks_are_not_saved() {
        let mut slots = vec![None; SLOTS];
        slots[0] = Some(crate::guns::weapon_item(7));
        slots[1] = Some(stack("minecraft:dirt", 0));
        slots[2] = Some(stack("minecraft:dirt", 2));
        let entries = PlayerData {
            inventory: Some(slots),
            ..PlayerData::default()
        }
        .entries();
        let inventory = entries["Inventory"].as_list().unwrap();
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].get("Slot"), Some(&Tag::Byte(2)));
        assert!(!entries.contains_key("equipment"));
    }

    #[test]
    fn an_empty_inventory_is_still_written_and_read_back_as_present() {
        let data = PlayerData {
            inventory: Some(vec![None; SLOTS]),
            ..PlayerData::default()
        };
        let tag = data.merged(None);
        assert_eq!(tag.get("Inventory"), Some(&Tag::List(Vec::new())));
        let read = PlayerData::read(&tag, &RecipeBook::default());
        assert_eq!(read.inventory, Some(vec![None; SLOTS]));
        let missing = PlayerData::read(&compound(Vec::new()), &RecipeBook::default());
        assert_eq!(missing.inventory, None);
        assert_eq!(missing.mode, None);
        assert_eq!(missing.selected, 0);
    }

    #[test]
    fn merging_keeps_unknown_keys_and_drops_armour_that_was_taken_off() {
        let previous = compound(vec![
            (
                "recipeBook",
                compound(vec![("recipes", Tag::List(Vec::new()))]),
            ),
            ("seenCredits", Tag::Byte(1)),
            (
                "equipment",
                compound(vec![(
                    "head",
                    compound(vec![
                        ("id", Tag::String("minecraft:iron_helmet".to_owned())),
                        ("count", Tag::Int(1)),
                    ]),
                )]),
            ),
            ("SelectedItemSlot", Tag::Int(3)),
        ]);
        let data = PlayerData {
            mode: Some(PlayerMode::new(GameMode::Survival)),
            inventory: Some(vec![None; SLOTS]),
            selected: 1,
        };
        let merged = data.merged(Some(previous.clone()));
        assert_eq!(merged.get("recipeBook"), previous.get("recipeBook"));
        assert_eq!(merged.get("seenCredits"), Some(&Tag::Byte(1)));
        assert_eq!(merged.get("equipment"), None);
        assert_eq!(merged.get("SelectedItemSlot"), Some(&Tag::Int(1)));
        assert_eq!(merged.get("playerGameType"), Some(&Tag::Int(0)));
        assert_eq!(
            merged.get("abilities").and_then(|a| a.get("mayfly")),
            Some(&Tag::Byte(0))
        );
    }

    #[test]
    fn slots_read_as_unsigned_bytes_and_bad_entries_are_skipped() {
        let list = Tag::List(vec![
            compound(vec![
                ("Slot", Tag::Byte(-1)),
                ("id", Tag::String("minecraft:stone".to_owned())),
            ]),
            compound(vec![
                ("Slot", Tag::Byte(5)),
                ("id", Tag::String("minecraft:stone".to_owned())),
            ]),
            compound(vec![("Slot", Tag::Byte(6)), ("count", Tag::Int(2))]),
            compound(vec![
                ("Slot", Tag::Byte(7)),
                ("id", Tag::String("minecraft:stone".to_owned())),
                ("count", Tag::Int(0)),
            ]),
            compound(vec![("id", Tag::String("minecraft:stone".to_owned()))]),
        ]);
        let read = slots(&list, STORAGE, &RecipeBook::default());
        assert_eq!(read.iter().flatten().count(), 1);
        assert_eq!(
            read[5].as_ref().map(|s| (s.id.as_str(), s.count)),
            Some(("minecraft:stone", 1))
        );
        let list = Tag::List(vec![compound(vec![
            ("Slot", Tag::Byte(-56)),
            ("id", Tag::String("minecraft:stone".to_owned())),
        ])]);
        assert!(slots(&list, 256, &RecipeBook::default())[200].is_some());
    }

    #[test]
    fn counts_are_ints_capped_at_vanilla_s_limit_and_old_string_components_load() {
        let tag = stack_tag(&stack("minecraft:stone", 120)).unwrap();
        assert_eq!(tag.get("count"), Some(&Tag::Int(99)));
        let old = compound(vec![
            ("id", Tag::String("minecraft:iron_sword".to_owned())),
            (
                "components",
                Tag::String(r#"{"minecraft:damage":3}"#.to_owned()),
            ),
        ]);
        let read = stack_of(&old, &RecipeBook::default()).unwrap();
        assert_eq!(read.count, 1);
        assert_eq!(read.components, Some(json!({"minecraft:damage": 3})));
        let written = stack_tag(&read).unwrap();
        assert_eq!(
            written.get("components"),
            Some(&compound(vec![("minecraft:damage", Tag::Int(3))]))
        );
    }

    #[test]
    fn a_selected_slot_outside_the_hotbar_reads_as_the_first() {
        let tag = compound(vec![("SelectedItemSlot", Tag::Int(12))]);
        assert_eq!(PlayerData::read(&tag, &RecipeBook::default()).selected, 0);
        let tag = compound(vec![("SelectedItemSlot", Tag::Int(-2))]);
        assert_eq!(PlayerData::read(&tag, &RecipeBook::default()).selected, 0);
    }
}
