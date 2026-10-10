use bevy::prelude::*;

use crate::minecraft_gui::McFont;

pub const POEM_WIDTH: i32 = 256;
pub const LINE_HEIGHT: i32 = 12;
pub const POEM_SPEED: f32 = 0.5;
pub const SPEEDUP_FACTOR: f32 = 5.0;
pub const SPEEDUP_FACTOR_FAST: f32 = 15.0;
pub const POEM_SEED: u64 = 8_124_371;
pub const LOGO_TOP: i32 = 50;
pub const TEXT_TOP: i32 = 100;
pub const WHITE: u32 = 0xFFFF_FFFF;
pub const YELLOW: u32 = 0xFFFF_FF55;
pub const GRAY: u32 = 0xFFAA_AAAA;
const SECTION_SIGN: char = '\u{a7}';
const OBFUSCATE_TOKEN: &str = "\u{a7}f\u{a7}k\u{a7}a\u{a7}b";
const OBFUSCATED_WHITE: &str = "\u{a7}f\u{a7}k";
const OBFUSCATED_TEXT: &str = "XXXXXXXX";
const SECTION_HEADING: &str = "============";
const NAME_PREFIX: &str = "           ";
const POEM_TAIL: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McCreditSpan {
    pub text: String,
    pub color: u32,
    pub obfuscated: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct McCreditLine {
    pub spans: Vec<McCreditSpan>,
    pub centered: bool,
}

impl McCreditLine {
    pub fn plain(text: &str, color: u32, centered: bool) -> Self {
        Self {
            spans: vec![McCreditSpan {
                text: text.to_owned(),
                color,
                obfuscated: false,
            }],
            centered,
        }
    }

    pub fn text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct McCreditsTitle {
    pub title: String,
    pub names: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct McCreditsDiscipline {
    pub discipline: String,
    pub titles: Vec<McCreditsTitle>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct McCreditsSection {
    pub section: String,
    pub disciplines: Vec<McCreditsDiscipline>,
}

#[derive(Clone, Debug, Default)]
pub struct McCreditsText {
    pub poem: String,
    pub sections: Vec<McCreditsSection>,
    pub postcredits: String,
    pub logo: Option<Handle<Image>>,
    pub edition: Option<Handle<Image>>,
}

#[derive(Clone, Debug, Default)]
pub struct McCredits {
    pub lines: Vec<McCreditLine>,
    pub logo: Option<Handle<Image>>,
    pub edition: Option<Handle<Image>>,
    pub scroll: f32,
    pub speedup: bool,
    pub fast_keys: u32,
    pub reverse: bool,
    pub finished: bool,
}

struct JavaRandom(u64);

impl JavaRandom {
    const MASK: u64 = (1 << 48) - 1;

    fn new(seed: u64) -> Self {
        Self((seed ^ 0x5_DEEC_E66D) & Self::MASK)
    }

    fn next_int_power_of_two(&mut self, bound: i64) -> i32 {
        self.0 = (self.0.wrapping_mul(0x5_DEEC_E66D).wrapping_add(11)) & Self::MASK;
        let bits = (self.0 >> 17) as i64;
        ((bound * bits) >> 31) as i32
    }
}

pub fn legacy_color(code: char) -> Option<u32> {
    let rgb = match code.to_ascii_lowercase() {
        '0' => 0x000000,
        '1' => 0x0000AA,
        '2' => 0x00AA00,
        '3' => 0x00AAAA,
        '4' => 0xAA0000,
        '5' => 0xAA00AA,
        '6' => 0xFFAA00,
        '7' => 0xAAAAAA,
        '8' => 0x555555,
        '9' => 0x5555FF,
        'a' => 0x55FF55,
        'b' => 0x55FFFF,
        'c' => 0xFF5555,
        'd' => 0xFF55FF,
        'e' => 0xFFFF55,
        'f' => 0xFFFFFF,
        _ => return None,
    };
    Some(0xFF00_0000 | rgb)
}

pub fn styled(text: &str, color: u32) -> Vec<(char, u32, bool)> {
    let mut out = Vec::new();
    let (mut current, mut obfuscated) = (color, false);
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != SECTION_SIGN {
            out.push((c, current, obfuscated));
            continue;
        }
        let Some(code) = chars.next() else {
            break;
        };
        if let Some(rgb) = legacy_color(code) {
            current = rgb;
            obfuscated = false;
        } else if code.eq_ignore_ascii_case(&'k') {
            obfuscated = true;
        } else if code.eq_ignore_ascii_case(&'r') {
            current = color;
            obfuscated = false;
        }
    }
    out
}

fn spans(chars: &[(char, u32, bool)]) -> Vec<McCreditSpan> {
    let mut spans: Vec<McCreditSpan> = Vec::new();
    for &(c, color, obfuscated) in chars {
        match spans.last_mut() {
            Some(span) if span.color == color && span.obfuscated == obfuscated => span.text.push(c),
            _ => spans.push(McCreditSpan {
                text: c.to_string(),
                color,
                obfuscated,
            }),
        }
    }
    spans
}

pub fn wrap(chars: &[(char, u32, bool)], font: &McFont, max_width: i32) -> Vec<McCreditLine> {
    let max = max_width as f32;
    let mut lines = Vec::new();
    let mut line: Vec<(char, u32, bool)> = Vec::new();
    let mut width = 0.0;
    let mut last_space: Option<usize> = None;
    for &styled in chars {
        let (c, _, _) = styled;
        let advance = font.advance(c);
        if !line.is_empty() && width + advance > max {
            if let Some(at) = last_space.take() {
                let rest = line.split_off(at + 1);
                line.pop();
                lines.push(std::mem::take(&mut line));
                line = rest;
                width = line.iter().map(|&(c, _, _)| font.advance(c)).sum();
            }
            if !line.is_empty() && width + advance > max {
                lines.push(std::mem::take(&mut line));
                width = 0.0;
            }
        }
        if c == ' ' {
            last_space = Some(line.len());
        }
        line.push(styled);
        width += advance;
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
        .iter()
        .map(|line| McCreditLine {
            spans: spans(line),
            centered: false,
        })
        .collect()
}

pub fn poem_lines(text: &str, player: &str, font: &McFont) -> Vec<McCreditLine> {
    let mut random = JavaRandom::new(POEM_SEED);
    let mut lines = Vec::new();
    for raw in text.lines() {
        let mut line = raw.replace("PLAYERNAME", player);
        while let Some(at) = line.find(OBFUSCATE_TOKEN) {
            let count = random.next_int_power_of_two(4) as usize + 3;
            let rest = line[at + OBFUSCATE_TOKEN.len()..].to_owned();
            line = format!(
                "{}{OBFUSCATED_WHITE}{}{rest}",
                &line[..at],
                &OBFUSCATED_TEXT[..count]
            );
        }
        lines.extend(wrap(&styled(&line, WHITE), font, POEM_WIDTH));
        lines.push(McCreditLine::default());
    }
    lines.extend(std::iter::repeat_n(McCreditLine::default(), POEM_TAIL));
    lines
}

pub fn credits_lines(sections: &[McCreditsSection]) -> Vec<McCreditLine> {
    let mut lines = Vec::new();
    for section in sections {
        lines.push(McCreditLine::plain(SECTION_HEADING, WHITE, true));
        lines.push(McCreditLine::plain(&section.section, YELLOW, true));
        lines.push(McCreditLine::plain(SECTION_HEADING, WHITE, true));
        lines.push(McCreditLine::default());
        lines.push(McCreditLine::default());
        for discipline in &section.disciplines {
            if !discipline.discipline.is_empty() {
                lines.push(McCreditLine::plain(&discipline.discipline, YELLOW, true));
                lines.push(McCreditLine::default());
                lines.push(McCreditLine::default());
            }
            for title in &discipline.titles {
                lines.push(McCreditLine::plain(&title.title, GRAY, false));
                for name in &title.names {
                    lines.push(McCreditLine::plain(
                        &format!("{NAME_PREFIX}{name}"),
                        WHITE,
                        false,
                    ));
                }
                lines.push(McCreditLine::default());
                lines.push(McCreditLine::default());
            }
        }
    }
    lines
}

impl McCredits {
    pub fn new(text: &McCreditsText, player: &str, font: &McFont) -> Self {
        let mut lines = poem_lines(&text.poem, player, font);
        lines.extend(credits_lines(&text.sections));
        lines.extend(poem_lines(&text.postcredits, player, font));
        Self {
            lines,
            logo: text.logo.clone(),
            edition: text.edition.clone(),
            ..Self::default()
        }
    }

    pub fn total_scroll_length(&self) -> i32 {
        self.lines.len() as i32 * LINE_HEIGHT
    }

    pub fn speed(&self) -> f32 {
        let direction = if self.reverse { -1.0 } else { 1.0 };
        let factor = if self.speedup {
            SPEEDUP_FACTOR + self.fast_keys as f32 * SPEEDUP_FACTOR_FAST
        } else {
            1.0
        };
        POEM_SPEED * factor * direction
    }

    pub fn advance(&mut self, ticks: f32, height: i32) {
        self.scroll = (self.scroll + ticks * self.speed()).max(0.0);
        let end = (self.total_scroll_length() + height + height + 24) as f32;
        if self.scroll > end {
            self.finished = true;
        }
    }

    pub fn skip(&mut self) {
        self.finished = true;
    }

    pub fn natural_y(&self, index: usize, height: i32) -> f32 {
        (height + LOGO_TOP + TEXT_TOP + index as i32 * LINE_HEIGHT) as f32 - self.scroll
    }

    pub fn line_y(&self, index: usize, height: i32) -> f32 {
        let y = self.natural_y(index, height);
        let middle = (height / 2 - 6) as f32;
        if index + 1 == self.lines.len() {
            y.max(middle)
        } else {
            y
        }
    }

    pub fn logo_y(&self, height: i32) -> f32 {
        (height + LOGO_TOP) as f32 - self.scroll
    }

    pub fn visible(y: f32, height: i32) -> bool {
        y + LINE_HEIGHT as f32 + 8.0 > 0.0 && y < height as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> McFont {
        McFont::default()
    }

    #[test]
    fn colour_codes_set_the_colour_and_end_obfuscation_like_vanilla() {
        let chars = styled("\u{a7}3ab\u{a7}f\u{a7}kXY\u{a7}2z", WHITE);
        assert_eq!(
            chars,
            [
                ('a', 0xFF00_AAAA, false),
                ('b', 0xFF00_AAAA, false),
                ('X', WHITE, true),
                ('Y', WHITE, true),
                ('z', 0xFF00_AA00, false),
            ]
        );
        assert_eq!(
            styled("a\u{a7}rb", GRAY),
            [('a', GRAY, false), ('b', GRAY, false)]
        );
    }

    #[test]
    fn the_poem_names_the_player_and_hides_the_unspeakable_words() {
        let poem = "\u{a7}3A first line of verse.\n\n\u{a7}2PLAYERNAME?\n\u{a7}3a piece of the \u{a7}f\u{a7}k\u{a7}a\u{a7}b\u{a7}3, and";
        let lines = poem_lines(poem, "Steve", &font());
        assert_eq!(lines[0].text(), "A first line of verse.");
        assert_eq!(lines[0].spans[0].color, 0xFF00_AAAA);
        assert_eq!(
            lines[1],
            McCreditLine::default(),
            "each line is followed by an empty one"
        );
        assert_eq!(lines[2], McCreditLine::default());
        assert_eq!(lines[4].text(), "Steve?");
        assert_eq!(lines[4].spans[0].color, 0xFF00_AA00);
        let hidden = &lines[6];
        let obfuscated: Vec<&McCreditSpan> =
            hidden.spans.iter().filter(|span| span.obfuscated).collect();
        assert_eq!(obfuscated.len(), 1);
        assert_eq!(obfuscated[0].color, WHITE);
        assert!((3..=6).contains(&obfuscated[0].text.len()));
        assert!(obfuscated[0].text.chars().all(|c| c == 'X'));
        assert_eq!(hidden.spans.last().unwrap().text, ", and");
        assert_eq!(lines.len(), 8 + 8, "eight empty lines close the file");
        assert_eq!(
            poem_lines(poem, "Steve", &font()),
            lines,
            "the same hidden words every time"
        );
    }

    #[test]
    fn long_poem_lines_wrap_at_256_pixels_on_a_space() {
        let words = "word ".repeat(20);
        let lines = wrap(&styled(words.trim_end(), WHITE), &font(), POEM_WIDTH);
        assert_eq!(lines.len(), 3);
        assert!(
            lines
                .iter()
                .all(|line| font().width(&line.text()) <= POEM_WIDTH)
        );
        assert!(lines.iter().all(|line| !line.text().starts_with(' ')));
        assert_eq!(lines[0].text(), "word ".repeat(8).trim_end());
    }

    #[test]
    fn the_credits_list_sections_disciplines_titles_and_names_like_vanilla() {
        let sections = [McCreditsSection {
            section: "Mojang Studios".to_owned(),
            disciplines: vec![
                McCreditsDiscipline {
                    discipline: "Leadership".to_owned(),
                    titles: vec![McCreditsTitle {
                        title: "Studio Head".to_owned(),
                        names: vec!["A".to_owned(), "B".to_owned()],
                    }],
                },
                McCreditsDiscipline {
                    discipline: String::new(),
                    titles: vec![McCreditsTitle {
                        title: "Thanks".to_owned(),
                        names: vec!["C".to_owned()],
                    }],
                },
            ],
        }];
        let lines = credits_lines(&sections);
        let texts: Vec<String> = lines.iter().map(McCreditLine::text).collect();
        assert_eq!(
            texts,
            [
                "============",
                "Mojang Studios",
                "============",
                "",
                "",
                "Leadership",
                "",
                "",
                "Studio Head",
                "           A",
                "           B",
                "",
                "",
                "Thanks",
                "           C",
                "",
                ""
            ]
        );
        assert!(lines[0].centered && lines[1].centered && lines[5].centered);
        assert!(!lines[8].centered && !lines[9].centered);
        assert_eq!(
            (lines[1].spans[0].color, lines[8].spans[0].color),
            (YELLOW, GRAY)
        );
    }

    #[test]
    fn the_credits_scroll_half_a_pixel_a_tick_and_end_two_screens_past_the_last_line() {
        let text = McCreditsText {
            poem: "a\nb".to_owned(),
            postcredits: "c".to_owned(),
            ..McCreditsText::default()
        };
        let mut credits = McCredits::new(&text, "Steve", &font());
        assert_eq!(credits.lines.len(), 4 + 8 + 2 + 8);
        let height = 240;
        let end = credits.total_scroll_length() + height * 2 + 24;
        credits.advance(20.0, height);
        assert_eq!(credits.scroll, 10.0, "ten pixels a second");
        credits.speedup = true;
        assert_eq!(credits.speed(), 2.5);
        credits.fast_keys = 1;
        assert_eq!(credits.speed(), 10.0);
        credits.speedup = false;
        credits.reverse = true;
        credits.advance(100.0, height);
        assert_eq!(credits.scroll, 0.0, "it never scrolls back past the start");
        credits.reverse = false;
        credits.advance(end as f32 * 2.0, height);
        assert!(!credits.finished, "exactly at the end it still shows");
        credits.advance(1.0, height);
        assert!(credits.finished);
    }

    #[test]
    fn the_last_line_stops_in_the_middle_of_the_screen() {
        let text = McCreditsText {
            poem: "a".to_owned(),
            ..McCreditsText::default()
        };
        let mut credits = McCredits::new(&text, "Steve", &font());
        let height = 200;
        let last = credits.lines.len() - 1;
        assert_eq!(credits.logo_y(height), 250.0);
        assert_eq!(credits.line_y(0, height), 350.0);
        assert_eq!(
            last, 17,
            "the empty post-credits still close with eight lines"
        );
        assert_eq!(credits.line_y(last, height), 554.0);
        credits.scroll = 500.0;
        assert_eq!(credits.line_y(last, height), 94.0, "held in the middle");
        assert_eq!(credits.natural_y(last, height), 54.0);
        assert_eq!(credits.line_y(0, height), -150.0);
        credits.scroll = 10_000.0;
        assert_eq!(credits.line_y(last, height), 94.0);
        assert!(!McCredits::visible(credits.natural_y(last, height), height));
        assert!(McCredits::visible(-19.0, height) && !McCredits::visible(-20.0, height));
        assert!(!McCredits::visible(200.0, height));
    }
}
