use minecraftoss_player::inventory::ItemStack;

pub const WRITTEN_BOOK: &str = "minecraft:written_book";
pub const WRITABLE_BOOK: &str = "minecraft:writable_book";
pub const WRITTEN_BOOK_CONTENT: &str = "minecraft:written_book_content";
pub const WRITABLE_BOOK_CONTENT: &str = "minecraft:writable_book_content";

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
