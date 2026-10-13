use minecraftoss_player::inventory::{Inventory, ItemStack};
use serde_json::{Map, Value, json};

pub const WRITTEN_BOOK: &str = "minecraft:written_book";
pub const WRITABLE_BOOK: &str = "minecraft:writable_book";
pub const WRITTEN_BOOK_CONTENT: &str = "minecraft:written_book_content";
pub const WRITABLE_BOOK_CONTENT: &str = "minecraft:writable_book_content";
pub const MAX_PAGES: usize = 100;
pub const PAGE_LENGTH: usize = 1024;
pub const TITLE_LENGTH: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookUse {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenBook {
    pub slot: usize,
    pub kind: BookUse,
}

pub fn is_book(stack: &ItemStack) -> bool {
    stack.id == WRITTEN_BOOK || stack.id == WRITABLE_BOOK
}

pub fn book_use(stack: &ItemStack) -> Option<BookUse> {
    match stack.id.as_str() {
        WRITABLE_BOOK => Some(BookUse::Write),
        WRITTEN_BOOK
            if stack
                .components
                .as_ref()
                .is_some_and(|components| components.get(WRITTEN_BOOK_CONTENT).is_some()) =>
        {
            Some(BookUse::Read)
        }
        _ => None,
    }
}

pub fn edit_book(
    inventory: &mut Inventory,
    slot: usize,
    pages: &[String],
    title: Option<&str>,
    author: &str,
) -> bool {
    if !(slot <= 8 || slot == crate::offhand::OFFHAND)
        || pages.len() > MAX_PAGES
        || pages.iter().any(|page| page.chars().count() > PAGE_LENGTH)
        || title.is_some_and(|title| title.chars().count() > TITLE_LENGTH)
    {
        return false;
    }
    let recipes = inventory.recipes.clone();
    let Some(stack) = inventory
        .slots
        .get_mut(slot)
        .and_then(Option::as_mut)
        .filter(|stack| stack.id == WRITABLE_BOOK)
    else {
        return false;
    };
    let pages: Vec<Value> = pages
        .iter()
        .map(|page| json!({"raw": filter_text(page)}))
        .collect();
    let mut components = match stack.components.take() {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    components.remove(WRITABLE_BOOK_CONTENT);
    match title {
        None => {
            if !pages.is_empty() {
                components.insert(WRITABLE_BOOK_CONTENT.to_owned(), json!({"pages": pages}));
            }
        }
        Some(title) => {
            let mut content = Map::new();
            content.insert("title".to_owned(), json!({"raw": filter_text(title)}));
            content.insert("author".to_owned(), Value::from(author));
            if !pages.is_empty() {
                content.insert("pages".to_owned(), Value::Array(pages));
            }
            content.insert("resolved".to_owned(), Value::Bool(true));
            components.insert(WRITTEN_BOOK_CONTENT.to_owned(), Value::Object(content));
            stack.id = WRITTEN_BOOK.to_owned();
            stack.max = recipes.stack(WRITTEN_BOOK, 1).max;
        }
    }
    stack.components = (!components.is_empty()).then_some(Value::Object(components));
    true
}

fn filter_text(text: &str) -> String {
    text.chars()
        .filter(|&c| c == '\n' || (c != '\u{a7}' && c >= ' ' && c != '\u{7f}'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_book_reads_and_a_quill_writes() {
        let mut written = ItemStack::new(WRITTEN_BOOK, 1);
        assert_eq!(book_use(&written), None, "a written book without content");
        assert!(is_book(&written));
        written.components = Some(json!({
            WRITTEN_BOOK_CONTENT: {"title": {"raw": "Test"}, "author": "Owner"}
        }));
        assert_eq!(book_use(&written), Some(BookUse::Read));
        let mut quill = ItemStack::new(WRITABLE_BOOK, 1);
        assert_eq!(book_use(&quill), Some(BookUse::Write));
        quill.components = Some(json!({WRITABLE_BOOK_CONTENT: {"pages": [{"raw": "a"}]}}));
        assert_eq!(book_use(&quill), Some(BookUse::Write));
        for other in [
            "minecraft:book",
            "minecraft:enchanted_book",
            "minecraft:stone",
        ] {
            let stack = ItemStack::new(other, 1);
            assert_eq!(book_use(&stack), None, "{other}");
            assert!(!is_book(&stack), "{other}");
        }
    }

    fn catalog_recipes() -> Option<std::sync::Arc<minecraftoss_player::crafting::RecipeBook>> {
        let root = std::env::var_os("MINECRAFTOSS_ROOT")?;
        let path = std::path::Path::new(&root).join(minecraft_terrain::crafting::ITEM_CATALOG);
        let catalog = minecraftoss_player::item_catalog::ItemCatalog::from_path(&path).ok()?;
        Some(std::sync::Arc::new(
            minecraftoss_player::crafting::RecipeBook::default()
                .with_item_catalog(std::sync::Arc::new(catalog)),
        ))
    }

    fn pages(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    fn quill(components: Option<Value>) -> ItemStack {
        let mut stack = ItemStack::new(WRITABLE_BOOK, 1);
        stack.max = 1;
        stack.components = components;
        stack
    }

    #[test]
    fn editing_a_quill_stores_its_pages_and_an_empty_one_has_no_component() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(quill(Some(json!({"minecraft:custom_name": "Draft"}))));
        assert!(edit_book(
            &mut inventory,
            0,
            &pages(&["one", "two\nthree"]),
            None,
            "Steve"
        ));
        let stack = inventory.slots[0].clone().expect("the quill");
        assert_eq!(stack.id, WRITABLE_BOOK);
        assert_eq!(
            stack.components,
            Some(json!({
                "minecraft:custom_name": "Draft",
                WRITABLE_BOOK_CONTENT: {"pages": [{"raw": "one"}, {"raw": "two\nthree"}]}
            }))
        );
        assert!(edit_book(&mut inventory, 0, &[], None, "Steve"));
        assert_eq!(
            inventory.slots[0]
                .as_ref()
                .and_then(|stack| stack.components.clone()),
            Some(json!({"minecraft:custom_name": "Draft"})),
            "no pages means no component, and the name stays"
        );
        inventory.slots[1] = Some(quill(None));
        assert!(edit_book(&mut inventory, 1, &pages(&["a"]), None, "Steve"));
        assert!(edit_book(&mut inventory, 1, &[], None, "Steve"));
        assert_eq!(
            inventory.slots[1]
                .as_ref()
                .map(|stack| stack.components.clone()),
            Some(None)
        );
    }

    #[test]
    fn signing_turns_a_quill_into_an_original_written_book() {
        let mut inventory = Inventory::default();
        let catalog = catalog_recipes();
        if let Some(recipes) = catalog.clone() {
            inventory.recipes = recipes;
        }
        let old = json!({
            "minecraft:custom_name": "Diary",
            WRITABLE_BOOK_CONTENT: {"pages": [{"raw": "old"}]}
        });
        inventory.slots[40] = Some(quill(Some(old)));
        assert!(edit_book(
            &mut inventory,
            40,
            &pages(&["Dear diary", "", ""]),
            Some("Diary"),
            "Steve"
        ));
        let stack = inventory.slots[40].clone().expect("the signed book");
        assert_eq!(stack.id, WRITTEN_BOOK);
        assert_eq!(stack.count, 1);
        assert_eq!(stack.max, inventory.recipes.max_stack(WRITTEN_BOOK));
        if catalog.is_some() {
            assert_eq!(stack.max, 16);
        }
        assert_eq!(
            stack.components,
            Some(json!({
                "minecraft:custom_name": "Diary",
                WRITTEN_BOOK_CONTENT: {
                    "title": {"raw": "Diary"},
                    "author": "Steve",
                    "pages": [{"raw": "Dear diary"}, {"raw": ""}, {"raw": ""}],
                    "resolved": true
                }
            }))
        );
        assert_eq!(book_use(&stack), Some(BookUse::Read));
        assert!(
            !edit_book(&mut inventory, 40, &pages(&["again"]), None, "Steve"),
            "a written book cannot be edited"
        );
    }

    #[test]
    fn book_edits_follow_vanilla_slot_and_size_rules() {
        let mut inventory = Inventory::default();
        for slot in [9, 39] {
            inventory.slots[slot] = Some(quill(None));
            assert!(!edit_book(&mut inventory, slot, &pages(&["a"]), None, "S"));
            assert_eq!(inventory.slots[slot], Some(quill(None)), "slot {slot}");
        }
        for slot in [0, 8, 40] {
            inventory.slots[slot] = Some(quill(None));
            assert!(edit_book(&mut inventory, slot, &pages(&["a"]), None, "S"));
        }
        assert!(!edit_book(&mut inventory, 5, &pages(&["a"]), None, "S"));
        inventory.slots[1] = Some(ItemStack::new(WRITTEN_BOOK, 1));
        inventory.slots[2] = Some(ItemStack::new("minecraft:stone", 1));
        assert!(!edit_book(&mut inventory, 1, &pages(&["a"]), None, "S"));
        assert!(!edit_book(
            &mut inventory,
            2,
            &pages(&["a"]),
            Some("T"),
            "S"
        ));
        assert_eq!(
            inventory.slots[2].as_ref().map(|stack| stack.id.as_str()),
            Some("minecraft:stone")
        );
        inventory.slots[3] = Some(quill(None));
        let refused = [
            (vec![String::new(); 101], None),
            (vec!["x".repeat(1025)], None),
            (pages(&["a"]), Some("t".repeat(33))),
        ];
        for (texts, title) in refused {
            assert!(!edit_book(&mut inventory, 3, &texts, title.as_deref(), "S"));
            assert_eq!(inventory.slots[3], Some(quill(None)));
        }
        assert!(edit_book(
            &mut inventory,
            3,
            &vec![String::new(); 100],
            None,
            "S"
        ));
        assert!(edit_book(&mut inventory, 3, &["é".repeat(1024)], None, "S"));
        assert!(edit_book(
            &mut inventory,
            3,
            &pages(&["a\u{a7}lb\u{1}c\nd\u{7f}e\tf"]),
            None,
            "S"
        ));
        assert_eq!(
            inventory.slots[3]
                .as_ref()
                .and_then(|stack| stack.components.clone()),
            Some(json!({WRITABLE_BOOK_CONTENT: {"pages": [{"raw": "albc\ndef"}]}}))
        );
        let title = format!("{}\u{a7}\u{0}", "t".repeat(30));
        assert!(edit_book(
            &mut inventory,
            3,
            &pages(&["p"]),
            Some(&title),
            "S"
        ));
        let signed = inventory.slots[3]
            .as_ref()
            .and_then(|stack| stack.components.as_ref())
            .and_then(|components| components.get(WRITTEN_BOOK_CONTENT))
            .and_then(|content| content.pointer("/title/raw"))
            .cloned();
        assert_eq!(signed, Some(Value::from("t".repeat(30))));
    }

    #[test]
    fn a_signed_book_survives_the_item_nbt_round_trip() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(quill(None));
        assert!(edit_book(
            &mut inventory,
            0,
            &pages(&["first", "second\nline"]),
            Some("Diary"),
            "Steve"
        ));
        let stack = inventory.slots[0].clone().expect("the signed book");
        let tag = minecraft_terrain::item_nbt::stack_tag(&stack).expect("a saved stack");
        let back = minecraft_terrain::item_nbt::stack_of(&tag, &inventory.recipes)
            .expect("a loaded stack");
        assert_eq!(back.id, WRITTEN_BOOK);
        assert_eq!(
            back.components,
            Some(json!({
                WRITTEN_BOOK_CONTENT: {
                    "title": {"raw": "Diary"},
                    "author": "Steve",
                    "pages": [{"raw": "first"}, {"raw": "second\nline"}],
                    "resolved": 1
                }
            }))
        );
        assert_eq!(book_use(&back), Some(BookUse::Read));
        assert_eq!(
            minecraft_terrain::lectern::pages_text(back.components.as_ref()),
            ["first", "second\nline"]
        );
    }

    fn view_bytes(player: &crate::player::Player) -> usize {
        let view = minecraft_net::ToClient::Player(Box::new(crate::views::player_view(player)));
        minecraft_net::encode(minecraft_net::Lane::Reliable, &view)
            .expect("a player view fits the reliable lane")
            .len()
    }

    #[test]
    fn a_player_view_with_a_full_book_is_measured() {
        let profile = minecraft_commands::server::Profile {
            id: minecraft_commands::PlayerId(2),
            name: "Alex".to_owned(),
            uuid: "00000000-0000-0000-0000-000000000002".to_owned(),
        };
        let mut player = crate::player::Player::new(
            &profile,
            Default::default(),
            Default::default(),
            Inventory::default(),
        );
        player.inventory.slots[0] = Some(ItemStack::new(WRITABLE_BOOK, 1));
        let empty = view_bytes(&player);
        let mut sizes = Vec::new();
        for letter in ['a', '\u{e9}', '\u{4e2d}'] {
            let page: String = std::iter::repeat_n(letter, PAGE_LENGTH).collect();
            let pages = vec![page; MAX_PAGES];
            player.inventory.slots[0] = Some(ItemStack::new(WRITABLE_BOOK, 1));
            assert!(edit_book(&mut player.inventory, 0, &pages, None, "Alex"));
            sizes.push(view_bytes(&player) - empty);
        }
        let text = MAX_PAGES * PAGE_LENGTH;
        for (width, &size) in (1..).zip(&sizes) {
            assert!((width * text..width * text + 2048).contains(&size));
        }
        assert!(empty + sizes[2] < minecraft_net::codec::RELIABLE_LIMIT / 10);
    }
}
