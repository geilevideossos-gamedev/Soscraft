use std::ops::Range;

use serde::{Deserialize, Serialize};

pub const RED: u32 = 0xFF5555;
pub const GRAY: u32 = 0xAAAAAA;
pub const GREEN: u32 = 0x55FF55;
pub const YELLOW: u32 = 0xFFFF55;
pub const AQUA: u32 = 0x55FFFF;
pub const LIGHT_PURPLE: u32 = 0xFF55FF;
pub const GOLD: u32 = 0xFFAA00;
pub const WHITE: u32 = 0xFFFFFF;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Content {
    Literal(String),
    Translate { key: String, args: Vec<Text> },
    ItemName(String),
    EntityName(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub content: Content,
    pub color: Option<u32>,
    pub italic: bool,
    pub underlined: bool,
    pub extra: Vec<Text>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolved {
    pub text: String,
    pub color: u32,
    pub spans: Vec<(Range<usize>, u32)>,
    pub italic: bool,
}

impl Text {
    fn of(content: Content) -> Self {
        Self {
            content,
            color: None,
            italic: false,
            underlined: false,
            extra: Vec::new(),
        }
    }

    pub fn literal(text: impl Into<String>) -> Self {
        Self::of(Content::Literal(text.into()))
    }

    pub fn translate(key: impl Into<String>, args: Vec<Text>) -> Self {
        Self::of(Content::Translate {
            key: key.into(),
            args,
        })
    }

    pub fn item(id: impl Into<String>) -> Self {
        Self::of(Content::ItemName(id.into()))
    }

    pub fn entity(id: impl Into<String>) -> Self {
        Self::of(Content::EntityName(id.into()))
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn underlined(mut self) -> Self {
        self.underlined = true;
        self
    }

    pub fn append(mut self, text: Text) -> Self {
        self.extra.push(text);
        self
    }

    pub fn resolve(&self, lang: &dyn Fn(&str) -> Option<String>) -> Resolved {
        let base = self.color.unwrap_or(WHITE);
        let mut text = String::new();
        let mut pieces = Vec::new();
        self.emit(lang, base, &mut text, &mut pieces);
        let mut spans: Vec<(Range<usize>, u32)> = Vec::new();
        for (range, color) in pieces {
            if color == base || range.is_empty() {
                continue;
            }
            match spans.last_mut() {
                Some((last, last_color)) if *last_color == color && last.end == range.start => {
                    last.end = range.end;
                }
                _ => spans.push((range, color)),
            }
        }
        Resolved {
            text,
            color: base,
            spans,
            italic: self.italic,
        }
    }

    fn emit(
        &self,
        lang: &dyn Fn(&str) -> Option<String>,
        inherited: u32,
        out: &mut String,
        pieces: &mut Vec<(Range<usize>, u32)>,
    ) {
        let color = self.color.unwrap_or(inherited);
        match &self.content {
            Content::Literal(text) => push(out, pieces, text, color),
            Content::Translate { key, args } => {
                let pattern = lang(key).unwrap_or_else(|| key.clone());
                fill(&pattern, args, lang, color, out, pieces);
            }
            Content::ItemName(id) => {
                let (namespace, path) = split_id(id);
                let name = lang(&format!("item.{namespace}.{path}"))
                    .or_else(|| lang(&format!("block.{namespace}.{path}")))
                    .unwrap_or_else(|| id.clone());
                push(out, pieces, &format!("[{name}]"), color);
            }
            Content::EntityName(id) => {
                let (namespace, path) = split_id(id);
                let name =
                    lang(&format!("entity.{namespace}.{path}")).unwrap_or_else(|| id.clone());
                push(out, pieces, &name, color);
            }
        }
        for child in &self.extra {
            child.emit(lang, color, out, pieces);
        }
    }
}

fn split_id(id: &str) -> (&str, &str) {
    id.split_once(':').unwrap_or(("minecraft", id))
}

fn push(out: &mut String, pieces: &mut Vec<(Range<usize>, u32)>, text: &str, color: u32) {
    let start = out.len();
    out.push_str(text);
    pieces.push((start..out.len(), color));
}

fn fill(
    pattern: &str,
    args: &[Text],
    lang: &dyn Fn(&str) -> Option<String>,
    color: u32,
    out: &mut String,
    pieces: &mut Vec<(Range<usize>, u32)>,
) {
    let mut literal = String::new();
    let mut next = 0;
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            literal.push(c);
            continue;
        }
        let mut digits = String::new();
        while let Some(&d) = chars.peek() {
            if !d.is_ascii_digit() {
                break;
            }
            digits.push(d);
            chars.next();
        }
        let mut index = None;
        if !digits.is_empty() {
            if chars.peek() == Some(&'$') {
                chars.next();
                index = digits.parse::<usize>().ok().map(|i| i.saturating_sub(1));
            } else {
                literal.push('%');
                literal.push_str(&digits);
                continue;
            }
        }
        match chars.next() {
            Some('s' | 'd') => {
                push(out, pieces, &std::mem::take(&mut literal), color);
                let at = index.unwrap_or_else(|| {
                    next += 1;
                    next - 1
                });
                if let Some(arg) = args.get(at) {
                    arg.emit(lang, color, out, pieces);
                }
            }
            Some('%') => literal.push('%'),
            Some(other) => {
                literal.push('%');
                literal.push(other);
            }
            None => literal.push('%'),
        }
    }
    push(out, pieces, &literal, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lang(entries: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = entries
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    #[test]
    fn translate_fills_positional_and_indexed_args() {
        let lang = lang(&[
            ("chat.type.text", "<%s> %s"),
            ("swap", "%2$s before %1$s, 100%%"),
            ("gameMode.creative", "Creative Mode"),
            ("outer", "[%s]"),
        ]);
        let chat = Text::translate(
            "chat.type.text",
            vec![Text::literal("Steve"), Text::literal("hello")],
        );
        assert_eq!(chat.resolve(&lang).text, "<Steve> hello");
        let swapped = Text::translate("swap", vec![Text::literal("a"), Text::literal("b")]);
        assert_eq!(swapped.resolve(&lang).text, "b before a, 100%");
        let nested = Text::translate(
            "outer",
            vec![Text::translate("gameMode.creative", Vec::new())],
        );
        assert_eq!(nested.resolve(&lang).text, "[Creative Mode]");
        let missing = Text::translate("not.a.key", vec![Text::literal("x")]);
        assert_eq!(missing.resolve(&lang).text, "not.a.key");
        let short = Text::translate("chat.type.text", vec![Text::literal("only")]);
        assert_eq!(short.resolve(&lang).text, "<only> ");
    }

    #[test]
    fn spans_cover_coloured_parts() {
        let lang = lang(&[("here", "<--[HERE]")]);
        let context = Text::literal("")
            .color(GRAY)
            .append(Text::literal("tp "))
            .append(Text::literal("frob").color(RED).underlined())
            .append(Text::translate("here", Vec::new()).color(RED).italic());
        let resolved = context.resolve(&lang);
        assert_eq!(resolved.text, "tp frob<--[HERE]");
        assert_eq!(resolved.color, GRAY);
        assert_eq!(resolved.spans, vec![(3..16, RED)]);
        let plain = Text::literal("hi").append(Text::literal(" there"));
        let resolved = plain.resolve(&lang);
        assert_eq!(resolved.color, WHITE);
        assert!(resolved.spans.is_empty());
        let mixed = Text::literal("a")
            .color(RED)
            .append(Text::literal("b").color(GREEN))
            .append(Text::literal("c"));
        assert_eq!(mixed.resolve(&lang).spans, vec![(1..2, GREEN)]);
    }

    #[test]
    fn item_names_try_item_then_block_keys_in_brackets() {
        let lang = lang(&[
            ("item.minecraft.diamond", "Diamond"),
            ("block.minecraft.stone", "Stone"),
            ("entity.minecraft.cow", "Cow"),
        ]);
        assert_eq!(
            Text::item("minecraft:diamond").resolve(&lang).text,
            "[Diamond]"
        );
        assert_eq!(Text::item("stone").resolve(&lang).text, "[Stone]");
        assert_eq!(
            Text::item("minecraft:nothing").resolve(&lang).text,
            "[minecraft:nothing]"
        );
        assert_eq!(Text::entity("minecraft:cow").resolve(&lang).text, "Cow");
        assert_eq!(Text::entity("mod:thing").resolve(&lang).text, "mod:thing");
    }
}
