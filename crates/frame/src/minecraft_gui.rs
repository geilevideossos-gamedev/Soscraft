use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McScaling {
    Stretch,
    Tile {
        width: u32,
        height: u32,
    },
    NineSlice {
        width: u32,
        height: u32,
        border: [u32; 4],
        stretch_inner: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McSprite {
    pub rect: [u32; 4],
    pub scaling: McScaling,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct McGlyph {
    pub rect: [u32; 4],
    pub scale: f32,
    pub ascent: i32,
    pub advance: f32,
}

pub const MC_MISSING_ADVANCE: f32 = 6.0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McFont {
    pub glyphs: HashMap<char, McGlyph>,
    pub spaces: HashMap<char, f32>,
}

impl McFont {
    pub fn advance(&self, c: char) -> f32 {
        if let Some(&space) = self.spaces.get(&c) {
            return space;
        }
        self.glyphs
            .get(&c)
            .map_or(MC_MISSING_ADVANCE, |glyph| glyph.advance)
    }

    pub fn width(&self, text: &str) -> i32 {
        text.chars().map(|c| self.advance(c)).sum::<f32>().ceil() as i32
    }

    pub fn head_by_width(&self, text: &str, max_width: i32) -> String {
        let mut left = max_width as f32;
        let mut out = String::new();
        for c in text.chars() {
            left -= self.advance(c);
            if left < 0.0 {
                break;
            }
            out.push(c);
        }
        out
    }

    pub fn split(&self, text: &str, max_width: i32) -> Vec<String> {
        let max = max_width as f32;
        let mut lines = Vec::new();
        let mut line = String::new();
        let mut width = 0.0;
        let mut drawn = false;
        let mut last_space: Option<usize> = None;
        for c in text.chars() {
            if c == '\n' {
                lines.push(std::mem::take(&mut line));
                width = 0.0;
                drawn = false;
                last_space = None;
                continue;
            }
            if c == ' ' {
                last_space = Some(line.len());
            }
            let advance = self.advance(c);
            if drawn && width + advance > max {
                if let Some(at) = last_space.take() {
                    let mut rest = line.split_off(at);
                    lines.push(std::mem::take(&mut line));
                    if !rest.is_empty() {
                        rest.remove(0);
                    }
                    line = rest;
                    width = line.chars().map(|c| self.advance(c)).sum();
                    drawn = width != 0.0;
                    if c == ' ' {
                        continue;
                    }
                }
                if drawn && width + advance > max {
                    lines.push(std::mem::take(&mut line));
                    width = 0.0;
                    drawn = false;
                }
            }
            line.push(c);
            width += advance;
            drawn |= advance != 0.0;
        }
        if !line.is_empty() || lines.is_empty() {
            lines.push(line);
        }
        lines
    }
}

#[derive(Clone, Debug, Default)]
pub struct McGui {
    pub image: Handle<Image>,
    pub size: [u32; 2],
    pub sheets: HashMap<String, [u32; 2]>,
    pub sprites: HashMap<String, McSprite>,
    pub white: [u32; 2],
    pub font: McFont,
    pub alt_font: McFont,
    pub texts: HashMap<String, String>,
}

impl McGui {
    pub fn text(&self, key: &str) -> String {
        self.texts
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn format(&self, key: &str, args: &[&str]) -> String {
        self.format_spans(key, args).0
    }

    pub fn format_spans(&self, key: &str, args: &[&str]) -> (String, Vec<std::ops::Range<usize>>) {
        let pattern = self.text(key);
        let mut out = String::new();
        let mut spans = Vec::new();
        let mut next = 0;
        let mut chars = pattern.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            let mut index = None;
            let mut digits = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    digits.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if !digits.is_empty() && chars.peek() == Some(&'$') {
                chars.next();
                index = digits.parse::<usize>().ok().map(|i| i.saturating_sub(1));
            } else if !digits.is_empty() {
                out.push('%');
                out.push_str(&digits);
                continue;
            }
            match chars.next() {
                Some('s') | Some('d') => {
                    let at = index.unwrap_or_else(|| {
                        next += 1;
                        next - 1
                    });
                    let start = out.len();
                    out.push_str(args.get(at).copied().unwrap_or(""));
                    spans.push(start..out.len());
                }
                Some('%') => out.push('%'),
                Some(other) => {
                    out.push('%');
                    out.push(other);
                }
                None => out.push('%'),
            }
        }
        (out, spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> McFont {
        let glyph = |advance: f32| McGlyph {
            rect: [0, 0, 8, 8],
            scale: 1.0,
            ascent: 7,
            advance,
        };
        let mut font = McFont::default();
        for c in 'a'..='z' {
            font.glyphs.insert(c, glyph(6.0));
        }
        font.glyphs.insert('i', glyph(2.0));
        font.spaces.insert(' ', 4.0);
        font
    }

    #[test]
    fn width_sums_advances_and_spaces() {
        let font = font();
        assert_eq!(font.width("ab"), 12);
        assert_eq!(font.width("a i"), 12);
        assert_eq!(font.width(""), 0);
        assert_eq!(font.width("?"), 6);
    }

    #[test]
    fn head_by_width_keeps_what_fits() {
        let font = font();
        assert_eq!(font.head_by_width("abcd", 18), "abc");
        assert_eq!(font.head_by_width("abcd", 17), "ab");
        assert_eq!(font.head_by_width("abcd", 100), "abcd");
    }

    #[test]
    fn split_breaks_at_spaces() {
        let font = font();
        assert_eq!(font.split("ab cd ef", 28), vec!["ab cd", "ef"]);
        assert_eq!(font.split("ab cd ef", 22), vec!["ab", "cd", "ef"]);
        assert_eq!(font.split("abcdef", 12), vec!["ab", "cd", "ef"]);
        assert_eq!(font.split("", 12), vec![""]);
    }

    #[test]
    fn format_fills_positional_and_indexed_arguments() {
        let mut gui = McGui::default();
        gui.texts
            .insert("cost".into(), "Enchantment Cost: %1$s".into());
        gui.texts.insert("clue".into(), "%s . . . ?".into());
        assert_eq!(gui.format("cost", &["7"]), "Enchantment Cost: 7");
        assert_eq!(
            gui.format("clue", &["Sharpness II"]),
            "Sharpness II . . . ?"
        );
        assert_eq!(gui.text("missing"), "missing");
        gui.texts.insert("pair".into(), "%2$s é %1$s".into());
        let (text, spans) = gui.format_spans("pair", &["a", "bc"]);
        assert_eq!(text, "bc é a");
        assert_eq!(spans, vec![0..2, 6..7]);
        assert_eq!(&text[spans[1].clone()], "a");
    }
}
