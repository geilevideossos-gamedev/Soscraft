use serde_json::Value;

pub use minecraftoss_world::book::{has_content, page_count};
pub use minecraftoss_world::level::lectern::{BOOKS, signal as comparator_signal};

pub const PAGE_TURN_SOUND: &str = "minecraft:item.book.page_turn";

pub fn is_lectern_book(id: &str) -> bool {
    BOOKS.contains(&id)
}

pub fn book_item_use(pos: crate::scene::BlockPos) -> crate::bucket::ItemUse {
    crate::bucket::ItemUse {
        change: crate::bucket::Change::Lectern(pos),
        result: None,
        wear: 0,
        consumed: true,
        sounds: Vec::new(),
    }
}

pub fn has_book(block: &crate::scene::Block) -> bool {
    block
        .properties
        .get("has_book")
        .is_some_and(|value| value == "true")
}

pub fn is_lectern(block: &crate::scene::Block) -> bool {
    block.id.namespace == "minecraft" && block.id.path == "lectern"
}

pub fn pages_text(components: Option<&Value>) -> Vec<String> {
    components
        .and_then(minecraftoss_world::book::pages)
        .into_iter()
        .flatten()
        .map(|page| {
            let mut text = String::new();
            flatten(minecraftoss_world::book::raw_page(page), &mut text);
            text
        })
        .collect()
}

fn flatten(component: &Value, out: &mut String) {
    match component {
        Value::String(text) => out.push_str(text),
        Value::Array(parts) => {
            for part in parts {
                flatten(part, out);
            }
        }
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                out.push_str(text);
            } else if let Some(key) = map.get("translate").and_then(Value::as_str) {
                let shown = map.get("fallback").and_then(Value::as_str).unwrap_or(key);
                out.push_str(shown);
            } else if let Some(key) = map.get("keybind").and_then(Value::as_str) {
                out.push_str(key);
            }
            if let Some(extra) = map.get("extra") {
                flatten(extra, out);
            }
        }
        Value::Number(number) => out.push_str(&number.to_string()),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Null => {}
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BookStack {
    pub id: String,
    pub components: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LecternAction {
    PlaceBook(BookStack),
    Open,
    Button(i32),
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LecternView {
    pub player: u64,
    pub pos: crate::scene::BlockPos,
    pub book: Option<BookStack>,
    pub page: i32,
    pub taken: Option<BookStack>,
    pub closed: bool,
}

pub const PREVIOUS_PAGE: i32 = 1;
pub const NEXT_PAGE: i32 = 2;
pub const TAKE_BOOK: i32 = 3;
pub const GO_TO_PAGE: i32 = 100;

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::nbt::Tag;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn book_tag(id: &str, pages: usize, content: bool) -> Tag {
        let mut map = BTreeMap::from([
            ("id".to_owned(), Tag::String(id.to_owned())),
            ("count".to_owned(), Tag::Int(1)),
        ]);
        if content {
            let key = if id == "minecraft:written_book" {
                "minecraft:written_book_content"
            } else {
                "minecraft:writable_book_content"
            };
            let pages = (0..pages)
                .map(|n| {
                    Tag::Compound(BTreeMap::from([(
                        "raw".to_owned(),
                        Tag::String(format!("{n}")),
                    )]))
                })
                .collect();
            let content = BTreeMap::from([("pages".to_owned(), Tag::List(pages))]);
            map.insert(
                "components".to_owned(),
                Tag::Compound(BTreeMap::from([(key.to_owned(), Tag::Compound(content))])),
            );
        }
        Tag::Compound(map)
    }

    #[test]
    fn the_comparator_reads_page_steps_like_vanilla() {
        let three = book_tag("minecraft:written_book", 3, true);
        let signals: Vec<i32> = (0..3)
            .map(|page| comparator_signal(Some(&three), page))
            .collect();
        assert_eq!(signals, [1, 8, 15]);
        let one = book_tag("minecraft:written_book", 1, true);
        assert_eq!(comparator_signal(Some(&one), 0), 15);
        let quill = book_tag("minecraft:writable_book", 0, false);
        assert_eq!(comparator_signal(Some(&quill), 0), 15);
        let bare = book_tag("minecraft:written_book", 0, false);
        assert_eq!(comparator_signal(Some(&bare), 0), 14);
        assert_eq!(comparator_signal(None, 0), 0);
        let fifteen = book_tag("minecraft:writable_book", 15, true);
        assert_eq!(comparator_signal(Some(&fifteen), 7), 8);
    }

    #[test]
    fn written_pages_flatten_to_text() {
        let components = json!({"minecraft:written_book_content": {
            "title": {"raw": "T"},
            "author": "A",
            "pages": [
                {"raw": "Plain"},
                {"raw": {"text": "Red ", "color": "red", "extra": ["and ", {"text": "bold", "bold": true}]}},
                {"raw": [{"translate": "block.minecraft.stone"}, " ", {"translate": "x.y", "fallback": "Fallback"}]},
                {"raw": "Raw", "filtered": "Filtered"}
            ]
        }});
        assert_eq!(
            pages_text(Some(&components)),
            [
                "Plain",
                "Red and bold",
                "block.minecraft.stone Fallback",
                "Raw"
            ]
        );
        let writable = json!({"minecraft:writable_book_content": {"pages": [{"raw": "draft"}]}});
        assert_eq!(pages_text(Some(&writable)), ["draft"]);
        assert_eq!(page_count(Some(&writable)), 1);
        assert!(pages_text(None).is_empty());
        assert!(is_lectern_book("minecraft:writable_book"));
        assert!(!is_lectern_book("minecraft:book"));
    }
}
