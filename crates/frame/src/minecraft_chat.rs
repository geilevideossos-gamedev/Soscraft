use std::collections::VecDeque;
use std::ops::Range;

pub const MC_CHAT_HISTORY: usize = 100;
pub const MC_CHAT_INPUT_LIMIT: usize = 256;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McChatLine {
    pub text: String,
    pub color: u32,
    pub spans: Vec<(Range<usize>, u32)>,
    pub italic: bool,
}

#[derive(Clone, Debug, Default)]
pub struct McChat {
    pub open: bool,
    pub input: String,
    pub cursor: usize,
    pub highlight: usize,
    pub history: Vec<String>,
    pub history_pos: usize,
    pub draft: String,
    pub lines: VecDeque<(McChatLine, f64)>,
    pub scroll: usize,
    pub new_since_scroll: bool,
    pub received: Vec<McChatLine>,
    pub sent: Vec<String>,
    pub opened_at: f64,
}

impl McChat {
    pub fn close(&mut self) {
        self.open = false;
        self.input.clear();
        self.cursor = 0;
        self.highlight = 0;
        self.draft.clear();
        self.scroll = 0;
        self.new_since_scroll = false;
    }

    pub fn clear_messages(&mut self) {
        self.close();
        self.lines.clear();
        self.received.clear();
        self.sent.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clearing_messages_keeps_the_sent_history() {
        let mut chat = McChat {
            open: true,
            input: "/se".into(),
            cursor: 3,
            highlight: 1,
            history: vec!["hello".into()],
            scroll: 4,
            ..McChat::default()
        };
        chat.lines.push_front((McChatLine::default(), 1.0));
        chat.received.push(McChatLine::default());
        chat.sent.push("hi".into());
        chat.clear_messages();
        assert!(!chat.open);
        assert!(chat.input.is_empty() && chat.cursor == 0 && chat.scroll == 0);
        assert_eq!(chat.highlight, 0);
        assert!(chat.lines.is_empty() && chat.received.is_empty() && chat.sent.is_empty());
        assert_eq!(chat.history, vec!["hello".to_owned()]);
    }
}
