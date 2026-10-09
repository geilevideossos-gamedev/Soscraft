use std::collections::BTreeMap;
use std::path::Path;

use minecraftoss_core::chunk_nbt::DATA_VERSION;
use minecraftoss_core::nbt::{self, Tag};
use minecraftoss_player::inventory::ItemStack;

pub const FILE: &str = "hotbar.nbt";
pub const GROUPS: usize = 9;
pub const SIZE: usize = 9;

#[derive(Clone, Debug, PartialEq)]
pub struct Hotbars {
    pub rows: Vec<Vec<Option<ItemStack>>>,
}

impl Default for Hotbars {
    fn default() -> Self {
        Self {
            rows: vec![vec![None; SIZE]; GROUPS],
        }
    }
}

impl Hotbars {
    pub fn row(&self, index: usize) -> &[Option<ItemStack>] {
        self.rows.get(index).map_or(&[], Vec::as_slice)
    }

    pub fn is_empty(&self, index: usize) -> bool {
        self.row(index).iter().all(Option::is_none)
    }

    pub fn store(&mut self, index: usize, stacks: &[Option<ItemStack>]) {
        if let Some(row) = self.rows.get_mut(index) {
            for (slot, stack) in row
                .iter_mut()
                .zip(stacks.iter().chain(std::iter::repeat(&None)))
            {
                *slot = stack.clone().filter(|stack| stack.count > 0);
            }
        }
    }

    pub fn to_tag(&self) -> Tag {
        let mut root = BTreeMap::new();
        root.insert("DataVersion".to_owned(), Tag::Int(DATA_VERSION));
        for (index, row) in self.rows.iter().enumerate() {
            let items = row.iter().map(|stack| stack_tag(stack.as_ref())).collect();
            root.insert(index.to_string(), Tag::List(items));
        }
        Tag::Compound(root)
    }

    pub fn from_tag(tag: &Tag) -> Self {
        let mut hotbars = Self::default();
        for (index, row) in hotbars.rows.iter_mut().enumerate() {
            let Some(items) = tag.get(&index.to_string()).and_then(Tag::as_list) else {
                continue;
            };
            if items.len() != SIZE {
                continue;
            }
            for (slot, item) in row.iter_mut().zip(items) {
                *slot = stack_of(item);
            }
        }
        hotbars
    }

    pub fn read(path: &Path) -> Self {
        match std::fs::read(path) {
            Ok(bytes) => match nbt::read(&bytes) {
                Ok(tag) => Self::from_tag(&tag),
                Err(error) => {
                    eprintln!("Failed to load creative mode options: {error}");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, nbt::write(&self.to_tag(), ""))
    }
}

fn stack_tag(stack: Option<&ItemStack>) -> Tag {
    let mut out = BTreeMap::new();
    if let Some(stack) = stack {
        out.insert("id".to_owned(), Tag::String(stack.id.clone()));
        out.insert("count".to_owned(), Tag::Int(i32::from(stack.count)));
        if let Some(components) = stack
            .components
            .as_ref()
            .filter(|components| components.as_object().is_some_and(|map| !map.is_empty()))
        {
            out.insert(
                "components".to_owned(),
                crate::item_nbt::json_tag(components),
            );
        }
    }
    Tag::Compound(out)
}

fn stack_of(tag: &Tag) -> Option<ItemStack> {
    let id = tag.get("id")?.as_str()?;
    let count = tag
        .get("count")
        .and_then(Tag::as_i64)
        .unwrap_or(1)
        .clamp(1, 99) as u8;
    let mut stack = ItemStack::new(id, count);
    stack.components = tag.get("components").map(crate::item_nbt::tag_json);
    Some(stack)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sword() -> ItemStack {
        let mut sword = ItemStack::new("minecraft:diamond_sword", 1);
        sword.components = Some(serde_json::json!({
            "minecraft:enchantments": {"minecraft:sharpness": 5},
            "minecraft:custom_name": "Edge"
        }));
        sword
    }

    #[test]
    fn hotbars_start_empty_and_store_nine_stacks() {
        let mut hotbars = Hotbars::default();
        assert!((0..GROUPS).all(|index| hotbars.is_empty(index)));
        let mut stacks = vec![None; 9];
        stacks[0] = Some(ItemStack::new("minecraft:stone", 64));
        stacks[8] = Some(sword());
        hotbars.store(2, &stacks);
        assert!(!hotbars.is_empty(2));
        assert_eq!(
            hotbars.row(2)[0],
            Some(ItemStack::new("minecraft:stone", 64))
        );
        hotbars.store(2, &[]);
        assert!(hotbars.is_empty(2));
        assert!(hotbars.row(9).is_empty());
    }

    #[test]
    fn the_file_layout_is_vanillas() {
        let mut hotbars = Hotbars::default();
        let mut stacks = vec![None; 9];
        stacks[0] = Some(ItemStack::new("minecraft:stone", 64));
        stacks[1] = Some(sword());
        hotbars.store(0, &stacks);
        let tag = hotbars.to_tag();
        assert_eq!(tag.get("DataVersion"), Some(&Tag::Int(DATA_VERSION)));
        for index in 0..GROUPS {
            assert_eq!(
                tag.get(&index.to_string())
                    .and_then(Tag::as_list)
                    .map(<[Tag]>::len),
                Some(SIZE)
            );
        }
        let row = tag.get("0").and_then(Tag::as_list).unwrap();
        assert_eq!(
            row[0].get("id"),
            Some(&Tag::String("minecraft:stone".into()))
        );
        assert_eq!(row[0].get("count"), Some(&Tag::Int(64)));
        assert_eq!(row[0].get("components"), None);
        assert_eq!(
            row[1]
                .get("components")
                .and_then(|components| components.get("minecraft:enchantments"))
                .and_then(|enchantments| enchantments.get("minecraft:sharpness")),
            Some(&Tag::Int(5))
        );
        assert_eq!(row[2], Tag::Compound(BTreeMap::new()));
    }

    #[test]
    fn hotbars_survive_the_disk_and_bad_rows_load_empty() {
        let mut hotbars = Hotbars::default();
        let mut stacks = vec![None; 9];
        stacks[4] = Some(sword());
        stacks[5] = Some(ItemStack::new("minecraft:torch", 16));
        hotbars.store(7, &stacks);
        let dir = std::env::temp_dir().join(format!("iw4l-hotbars-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE);
        hotbars.write(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes[0], 10);
        assert_eq!(&bytes[1..3], &[0, 0]);
        assert_eq!(Hotbars::read(&path), hotbars);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Hotbars::read(&dir.join("missing.nbt")), Hotbars::default());
        let mut root = BTreeMap::new();
        root.insert(
            "3".to_owned(),
            Tag::List(vec![Tag::Compound(BTreeMap::new())]),
        );
        root.insert(
            "4".to_owned(),
            Tag::List(
                (0..9)
                    .map(|_| {
                        let mut item = BTreeMap::new();
                        item.insert("id".to_owned(), Tag::String("minecraft:dirt".into()));
                        Tag::Compound(item)
                    })
                    .collect(),
            ),
        );
        let loaded = Hotbars::from_tag(&Tag::Compound(root));
        assert!(loaded.is_empty(3));
        assert_eq!(loaded.row(4)[8], Some(ItemStack::new("minecraft:dirt", 1)));
    }
}
