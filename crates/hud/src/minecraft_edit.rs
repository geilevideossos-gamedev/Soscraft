use std::ops::Range;

use minecraft_commands::protocol::allowed_in_chat;

pub(crate) struct Edit<'a> {
    text: &'a mut String,
    cursor: &'a mut usize,
    highlight: &'a mut usize,
    limit: usize,
}

impl<'a> Edit<'a> {
    pub(crate) fn new(
        text: &'a mut String,
        cursor: &'a mut usize,
        highlight: &'a mut usize,
        limit: usize,
    ) -> Self {
        *cursor = boundary(text, *cursor);
        *highlight = boundary(text, *highlight);
        Self {
            text,
            cursor,
            highlight,
            limit,
        }
    }

    pub(crate) fn selection(&self) -> Range<usize> {
        let (a, b) = (*self.cursor, *self.highlight);
        a.min(b)..a.max(b)
    }

    pub(crate) fn move_to(&mut self, pos: usize, extend: bool) {
        *self.cursor = boundary(self.text, pos);
        if !extend {
            *self.highlight = *self.cursor;
        }
    }

    pub(crate) fn step(&mut self, dir: i32, words: bool, extend: bool) {
        let to = target(self.text, *self.cursor, dir, words);
        self.move_to(to, extend);
    }

    pub(crate) fn select_all(&mut self) {
        *self.cursor = self.text.len();
        *self.highlight = 0;
    }

    pub(crate) fn insert(&mut self, text: &str) {
        let range = self.selection();
        let len = self.text.chars().count();
        let selected = self.text[range.clone()].chars().count();
        let room = (self.limit + selected).saturating_sub(len);
        if room == 0 {
            return;
        }
        let added: String = text
            .chars()
            .filter(|c| allowed_in_chat(*c))
            .take(room)
            .collect();
        self.text.replace_range(range.clone(), &added);
        *self.cursor = range.start + added.len();
        *self.highlight = *self.cursor;
    }

    pub(crate) fn delete(&mut self, dir: i32, words: bool) {
        if self.text.is_empty() {
            return;
        }
        if *self.cursor != *self.highlight {
            self.insert("");
            return;
        }
        let to = target(self.text, *self.cursor, dir, words);
        let range = to.min(*self.cursor)..to.max(*self.cursor);
        if range.is_empty() {
            return;
        }
        self.text.replace_range(range.clone(), "");
        *self.cursor = range.start;
        *self.highlight = range.start;
    }
}

fn boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn target(text: &str, at: usize, dir: i32, words: bool) -> usize {
    if words {
        word_position(text, at, dir)
    } else if dir < 0 {
        previous_char(text, at)
    } else {
        next_char(text, at)
    }
}

pub(crate) fn previous_char(text: &str, at: usize) -> usize {
    text[..at]
        .chars()
        .next_back()
        .map_or(0, |c| at - c.len_utf8())
}

pub(crate) fn next_char(text: &str, at: usize) -> usize {
    text[at..]
        .chars()
        .next()
        .map_or(text.len(), |c| at + c.len_utf8())
}

pub(crate) fn word_position(text: &str, at: usize, dir: i32) -> usize {
    let bytes = text.as_bytes();
    let mut i = at;
    if dir < 0 {
        while i > 0 && bytes[i - 1] == b' ' {
            i -= 1;
        }
        while i > 0 && bytes[i - 1] != b' ' {
            i -= 1;
        }
    } else {
        i = text[at..].find(' ').map_or(text.len(), |space| at + space);
        while i < text.len() && bytes[i] == b' ' {
            i += 1;
        }
    }
    boundary(text, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Line {
        text: String,
        cursor: usize,
        highlight: usize,
    }

    impl Line {
        fn new(text: &str) -> Self {
            Self {
                text: text.to_owned(),
                cursor: text.len(),
                highlight: text.len(),
            }
        }

        fn edit(&mut self) -> Edit<'_> {
            Edit::new(&mut self.text, &mut self.cursor, &mut self.highlight, 256)
        }

        fn selected(&mut self) -> String {
            let range = self.edit().selection();
            self.text[range].to_owned()
        }
    }

    #[test]
    fn shift_with_the_arrows_home_and_end_extends_the_selection() {
        let mut line = Line::new("hello big world");
        line.edit().step(-1, false, true);
        line.edit().step(-1, false, true);
        assert_eq!(line.selected(), "ld");
        line.edit().move_to(0, true);
        assert_eq!(line.selected(), "hello big world");
        assert_eq!((line.cursor, line.highlight), (0, 15));
        line.edit().move_to(15, true);
        assert_eq!(line.selected(), "");
        line.edit().move_to(6, false);
        line.edit().move_to(15, true);
        assert_eq!(line.selected(), "big world");
    }

    #[test]
    fn a_plain_arrow_drops_the_selection_and_steps_from_the_cursor() {
        let mut line = Line::new("hello big world");
        line.edit().move_to(6, true);
        line.edit().step(-1, false, false);
        assert_eq!((line.cursor, line.highlight), (5, 5));
        line.edit().move_to(10, true);
        line.edit().step(1, false, false);
        assert_eq!((line.cursor, line.highlight), (11, 11));
        line.edit().move_to(0, false);
        line.edit().move_to(5, true);
        line.edit().move_to(15, false);
        assert_eq!((line.cursor, line.highlight), (15, 15));
    }

    #[test]
    fn ctrl_shift_left_selects_a_word() {
        let mut line = Line::new("hello big world");
        line.edit().step(-1, true, true);
        assert_eq!(line.selected(), "world");
        line.edit().step(-1, true, true);
        assert_eq!(line.selected(), "big world");
        line.edit().step(1, true, true);
        assert_eq!(line.selected(), "world");
        line.edit().move_to(0, false);
        line.edit().step(1, true, true);
        assert_eq!(line.selected(), "hello ");
    }

    #[test]
    fn select_all_puts_the_cursor_at_the_end() {
        let mut line = Line::new("abc");
        line.cursor = 1;
        line.highlight = 1;
        line.edit().select_all();
        assert_eq!((line.cursor, line.highlight), (3, 0));
        assert_eq!(line.selected(), "abc");
    }

    #[test]
    fn deleting_eats_the_selection_first() {
        let mut line = Line::new("hello big world");
        line.edit().step(-1, true, true);
        line.edit().delete(-1, false);
        assert_eq!((line.text.as_str(), line.cursor), ("hello big ", 10));
        line.edit().move_to(6, true);
        line.edit().delete(1, false);
        assert_eq!((line.text.as_str(), line.cursor), ("hello ", 6));
        line.edit().move_to(2, true);
        line.edit().delete(-1, true);
        assert_eq!((line.text.as_str(), line.cursor), ("he", 2));
        assert_eq!(line.highlight, 2);
        line.edit().select_all();
        line.edit().delete(1, true);
        assert_eq!(
            (line.text.as_str(), line.cursor, line.highlight),
            ("", 0, 0)
        );
        line.edit().delete(-1, true);
        assert_eq!(line.text, "");
    }

    #[test]
    fn words_delete_like_vanilla() {
        let mut line = Line::new("say  hello there");
        line.edit().delete(-1, true);
        assert_eq!(line.text, "say  hello ");
        line.edit().delete(-1, true);
        assert_eq!(line.text, "say  ");
        line.edit().move_to(0, false);
        line.edit().delete(1, true);
        assert_eq!((line.text.as_str(), line.cursor), ("", 0));
        let mut line = Line::new("a bc d");
        line.edit().move_to(3, false);
        line.edit().delete(1, true);
        assert_eq!((line.text.as_str(), line.cursor), ("a bd", 3));
    }

    #[test]
    fn typing_replaces_the_selection() {
        let mut line = Line::new("hello big world");
        line.edit().step(-1, true, true);
        line.edit().insert("x");
        assert_eq!(
            (line.text.as_str(), line.cursor, line.highlight),
            ("hello big x", 11, 11)
        );
        line.edit().select_all();
        line.edit().insert("é");
        assert_eq!((line.text.as_str(), line.cursor), ("é", 2));
    }

    #[test]
    fn insert_room_counts_the_selected_length() {
        let mut line = Line::new(&"a".repeat(256));
        line.edit().insert("b");
        assert_eq!(line.text.len(), 256);
        assert!(!line.text.contains('b'));
        line.edit().move_to(246, true);
        line.edit().insert(&"c".repeat(20));
        assert_eq!(line.text.chars().count(), 256);
        assert_eq!(&line.text[246..], "c".repeat(10));
        assert_eq!(line.cursor, 256);
    }

    #[test]
    fn inserted_text_loses_section_signs_and_control_characters() {
        let mut line = Line::new("");
        line.edit().insert("a\u{a7}b\r\n\tc\u{7f}");
        assert_eq!(line.text, "abc");
    }

    #[test]
    fn positions_never_split_a_character() {
        let mut line = Line::new("café");
        line.cursor = 4;
        line.highlight = 4;
        assert_eq!(line.edit().selection(), 3..3);
        line.edit().step(1, false, true);
        assert_eq!(line.selected(), "é");
        assert_eq!(word_position("aé b", 3, -1), 0);
    }
}
