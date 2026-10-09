use std::ops::Range;

use bevy::input::keyboard::Key;
use frame::McChat;
use frame::minecraft_chat::{MC_CHAT_INPUT_LIMIT, McSuggestionList};
use minecraft_commands::complete;
use minecraft_commands::suggest::suggestion_suffix;

use crate::minecraft_chat::Typed;
use crate::minecraft_gui::{Painter, TextLine, opaque};

pub(crate) const LINE_LIMIT: usize = 10;
const LINE_HEIGHT: i32 = 12;
const USAGE_OFFSET_FROM_BOTTOM: i32 = 27;
const TEXT_X: i32 = 4;
const FILL: u32 = 0xD000_0000;
const SELECTED: u32 = 0xFFFF_FF00;
const UNSELECTED: u32 = 0xFFAA_AAAA;
const SCROLL_DOTS: u32 = 0xFFFF_FFFF;
pub(crate) const INLINE: u32 = 0xFF80_8080;

pub(crate) fn update(chat: &mut McChat) {
    let completion = complete(&chat.input, chat.cursor, chat.command_level, &chat.hints);
    let suggestions = &mut chat.suggestions;
    if !suggestions.keep {
        suggestions.inline = None;
        suggestions.list = None;
    }
    suggestions.usage.clear();
    suggestions.spans = completion.spans.clone();
    let mut show_now = false;
    if completion.command {
        if chat.cursor >= 1 && (suggestions.list.is_none() || !suggestions.keep) {
            suggestions.usage = completion.usage.clone();
            suggestions.usage_start = completion.usage_start;
            suggestions.pending = Some(completion);
            suggestions.list = None;
            show_now = suggestions.allow;
        }
    } else if chat.input.trim().is_empty() {
        suggestions.pending = None;
    } else {
        suggestions.usage_start = 0;
        suggestions.pending = Some(completion);
    }
    if show_now {
        show(chat);
    }
}

pub(crate) fn edited(chat: &mut McChat) {
    chat.suggestions.allow = true;
    update(chat);
}

pub(crate) fn browsed(chat: &mut McChat) {
    edited(chat);
    chat.suggestions.allow = false;
    chat.suggestions.list = None;
}

pub(crate) fn show(chat: &mut McChat) {
    let Some(pending) = chat.suggestions.pending.as_ref() else {
        return;
    };
    if pending.list.is_empty() {
        return;
    }
    chat.suggestions.list = Some(McSuggestionList {
        list: pending.list.clone(),
        original: chat.input.clone(),
        ..McSuggestionList::default()
    });
    select(chat, 0);
}

fn select(chat: &mut McChat, index: isize) {
    let suggestions = &mut chat.suggestions;
    let Some(list) = suggestions.list.as_mut() else {
        return;
    };
    let len = list.list.len() as isize;
    if len == 0 {
        return;
    }
    list.current = index.rem_euclid(len) as usize;
    let applied = list.list[list.current].apply(&list.original);
    suggestions.inline = suggestion_suffix(&chat.input, &applied).map(str::to_owned);
}

fn cycle(chat: &mut McChat, dir: isize) {
    let Some(current) = chat.suggestions.list.as_ref().map(|list| list.current) else {
        return;
    };
    select(chat, current as isize + dir);
    let Some(list) = chat.suggestions.list.as_mut() else {
        return;
    };
    let most = list.list.len().saturating_sub(LINE_LIMIT);
    if list.current < list.offset {
        list.offset = list.current.min(most);
    } else if list.current > list.offset + LINE_LIMIT - 1 {
        list.offset = (list.current + 1).saturating_sub(LINE_LIMIT).min(most);
    }
}

fn use_suggestion(chat: &mut McChat) {
    let Some(list) = chat.suggestions.list.as_ref() else {
        return;
    };
    let current = list.current;
    let suggestion = list.list[current].clone();
    let applied = suggestion.apply(&list.original);
    chat.suggestions.keep = true;
    chat.input = applied.chars().take(MC_CHAT_INPUT_LIMIT).collect();
    chat.cursor = chat.input.len();
    chat.highlight = chat.cursor;
    edited(chat);
    let mut cursor = suggestion.cursor().min(chat.input.len());
    while !chat.input.is_char_boundary(cursor) {
        cursor -= 1;
    }
    chat.cursor = cursor;
    chat.highlight = cursor;
    select(chat, current as isize);
    chat.suggestions.keep = false;
    if let Some(list) = chat.suggestions.list.as_mut() {
        list.tab_cycles = true;
    }
}

pub(crate) fn key(chat: &mut McChat, typed: &Typed) -> bool {
    if let Some(tab_cycles) = chat.suggestions.list.as_ref().map(|list| list.tab_cycles) {
        match typed.key {
            Key::ArrowUp | Key::ArrowDown => {
                let dir = if typed.key == Key::ArrowUp { -1 } else { 1 };
                cycle(chat, dir);
                if let Some(list) = chat.suggestions.list.as_mut() {
                    list.tab_cycles = false;
                }
                return true;
            }
            Key::Tab => {
                if tab_cycles {
                    cycle(chat, if typed.mods.shift { -1 } else { 1 });
                }
                use_suggestion(chat);
                return true;
            }
            Key::Escape => {
                chat.suggestions.list = None;
                chat.suggestions.inline = None;
                return true;
            }
            _ => {}
        }
    }
    if typed.key == Key::Tab {
        show(chat);
        return true;
    }
    false
}

pub(crate) fn screen_x(width: impl Fn(&str) -> i32, input: &str, at: usize) -> i32 {
    input.get(..at).map_or(TEXT_X, |head| TEXT_X + width(head))
}

fn clamp(value: i32, low: i32, high: i32) -> i32 {
    value.max(low).min(high)
}

pub(crate) fn list_rect(
    width: impl Fn(&str) -> i32,
    list: &McSuggestionList,
    height: i32,
    inner: i32,
) -> [i32; 4] {
    let widest = list
        .list
        .iter()
        .map(|suggestion| width(&suggestion.text))
        .max()
        .unwrap_or(0);
    let start = list
        .list
        .first()
        .map_or(0, |suggestion| suggestion.range.start);
    let x = clamp(
        screen_x(&width, &list.original, start),
        0,
        TEXT_X + inner - widest,
    );
    let lines = list.list.len().min(LINE_LIMIT) as i32 * LINE_HEIGHT;
    [x - 1, height - LINE_HEIGHT - 3 - lines, widest + 1, lines]
}

pub(crate) fn usage_position(
    width: impl Fn(&str) -> i32,
    input: &str,
    start: usize,
    widest: i32,
    inner: i32,
) -> i32 {
    clamp(screen_x(width, input, start), 0, TEXT_X + inner - widest)
}

pub(crate) fn usage_top(height: i32, line: usize) -> i32 {
    height - USAGE_OFFSET_FROM_BOTTOM - LINE_HEIGHT * line as i32
}

pub(crate) fn input_accents(
    spans: &[(Range<usize>, u32)],
    from: usize,
    shown: usize,
) -> Vec<(Range<usize>, u32)> {
    let end = from + shown;
    spans
        .iter()
        .filter_map(|(range, color)| {
            let (start, stop) = (range.start.max(from), range.end.min(end));
            (start < stop).then(|| (start - from..stop - from, opaque(*color)))
        })
        .collect()
}

pub(crate) fn draw(p: &mut Painter, chat: &McChat, inner: i32) {
    let height = p.space.height;
    let font = &p.gui.font;
    let width = |text: &str| font.width(text);
    if let Some(list) = chat.suggestions.list.as_ref() {
        let [x, y, w, h] = list_rect(width, list, height, inner);
        let shown = list.list.len().min(LINE_LIMIT);
        let above = list.offset > 0;
        let below = list.list.len() > list.offset + shown;
        if above || below {
            p.fill(x, y - 1, x + w, y, FILL);
            p.fill(x, y + h, x + w, y + h + 1, FILL);
            for dot in (0..w).step_by(2) {
                if above {
                    p.fill(x + dot, y - 1, x + dot + 1, y, SCROLL_DOTS);
                }
                if below {
                    p.fill(x + dot, y + h, x + dot + 1, y + h + 1, SCROLL_DOTS);
                }
            }
        }
        for line in 0..shown {
            let index = line + list.offset;
            let top = y + LINE_HEIGHT * line as i32;
            p.fill(x, top, x + w, top + LINE_HEIGHT, FILL);
            let color = if index == list.current {
                SELECTED
            } else {
                UNSELECTED
            };
            p.text(&list.list[index].text, x + 1, top + 2, color, true);
        }
        return;
    }
    let lang = |key: &str| chat.language.get(key).cloned();
    let lines: Vec<_> = chat
        .suggestions
        .usage
        .iter()
        .map(|text| text.resolve(&lang))
        .collect();
    let Some(widest) = lines.iter().map(|line| font.width(&line.text)).max() else {
        return;
    };
    let x = usage_position(
        width,
        &chat.input,
        chat.suggestions.usage_start,
        widest,
        inner,
    );
    for (index, line) in lines.into_iter().enumerate() {
        let top = usage_top(height, index);
        p.fill(x - 1, top, x + widest + 1, top + LINE_HEIGHT, FILL);
        let text = TextLine {
            text: line.text,
            color: opaque(line.color),
            italic: line.italic,
            accents: line
                .spans
                .into_iter()
                .map(|(range, color)| (range, opaque(color)))
                .collect(),
        };
        p.line(false, &text, x, top + 2, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft_chat::{Mods, Opener, apply_typed, open};
    use bevy::input::keyboard::{KeyCode, NativeKeyCode};
    use minecraft_commands::CommandHints;
    use minecraft_commands::text::{AQUA, GRAY, RED};

    const NO_CODE: KeyCode = KeyCode::Unidentified(NativeKeyCode::Unidentified);
    const SHIFT: Mods = Mods {
        ctrl: false,
        shift: true,
        alt: false,
    };

    fn press(chat: &mut McChat, key: Key, mods: Mods) {
        let typed = Typed {
            key,
            code: NO_CODE,
            text: None,
            mods,
        };
        apply_typed(chat, &typed, &mut ());
    }

    fn tab(chat: &mut McChat) {
        press(chat, Key::Tab, Mods::default());
    }

    fn type_text(chat: &mut McChat, text: &str) {
        for c in text.chars() {
            let typed = Typed {
                key: Key::Character(c.to_string().into()),
                code: NO_CODE,
                text: Some(c.to_string()),
                mods: Mods::default(),
            };
            apply_typed(chat, &typed, &mut ());
        }
    }

    fn owned(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    fn opened(opener: Opener) -> McChat {
        let mut chat = McChat {
            command_level: 4,
            hints: CommandHints {
                players: owned(&["Steve", "Alex"]),
                items: owned(&[
                    "minecraft:stone",
                    "minecraft:diamond_pickaxe",
                    "minecraft:iron_pickaxe",
                ]),
                ..CommandHints::default()
            },
            history: owned(&["/seed", "hello"]),
            ..McChat::default()
        };
        open(&mut chat, opener, 0.0);
        chat
    }

    fn shown(chat: &McChat) -> Vec<&str> {
        chat.suggestions
            .list
            .iter()
            .flat_map(|list| list.list.iter().map(|item| item.text.as_str()))
            .collect()
    }

    fn current(chat: &McChat) -> Option<usize> {
        chat.suggestions.list.as_ref().map(|list| list.current)
    }

    fn width(text: &str) -> i32 {
        text.chars().count() as i32 * 6
    }

    fn many(count: usize) -> McChat {
        let mut chat = opened(Opener::Command);
        chat.suggestions.list = Some(McSuggestionList {
            list: (0..count)
                .map(|i| minecraft_commands::Suggestion {
                    range: 1..1,
                    text: format!("c{i:02}"),
                })
                .collect(),
            original: "/".into(),
            ..McSuggestionList::default()
        });
        chat
    }

    #[test]
    fn typing_a_command_shows_the_list_and_the_grey_rest_of_the_first() {
        let mut chat = opened(Opener::Command);
        assert!(chat.suggestions.list.is_none());
        assert!(chat.suggestions.pending.is_some());
        type_text(&mut chat, "ga");
        assert_eq!(shown(&chat), ["gamemode", "gamerule"]);
        assert_eq!(current(&chat), Some(0));
        assert_eq!(chat.suggestions.inline.as_deref(), Some("memode"));
        assert_eq!(chat.suggestions.spans, vec![(0..1, GRAY), (1..3, RED)]);
    }

    #[test]
    fn tab_writes_the_selection_then_cycles_and_shift_tab_goes_back() {
        let mut chat = opened(Opener::Command);
        type_text(&mut chat, "ga");
        tab(&mut chat);
        assert_eq!((chat.input.as_str(), chat.cursor), ("/gamemode", 9));
        assert_eq!(shown(&chat), ["gamemode", "gamerule"]);
        assert_eq!(chat.suggestions.inline.as_deref(), Some(""));
        tab(&mut chat);
        assert_eq!((chat.input.as_str(), chat.cursor), ("/gamerule", 9));
        assert_eq!(current(&chat), Some(1));
        tab(&mut chat);
        assert_eq!(chat.input, "/gamemode");
        press(&mut chat, Key::Tab, SHIFT);
        assert_eq!(chat.input, "/gamerule");
        assert_eq!(chat.suggestions.spans, vec![(0..9, GRAY)]);
        type_text(&mut chat, " ");
        let list = chat.suggestions.list.as_ref().unwrap();
        assert_eq!((list.original.as_str(), list.current), ("/gamerule ", 0));
        assert!(!list.tab_cycles);
    }

    #[test]
    fn up_and_down_move_in_the_list_without_writing_and_reset_the_cycle() {
        let mut chat = opened(Opener::Command);
        type_text(&mut chat, "ga");
        press(&mut chat, Key::ArrowDown, Mods::default());
        assert_eq!(current(&chat), Some(1));
        assert_eq!(chat.input, "/ga");
        assert_eq!(chat.suggestions.inline.as_deref(), Some("merule"));
        press(&mut chat, Key::ArrowDown, Mods::default());
        assert_eq!(current(&chat), Some(0));
        press(&mut chat, Key::ArrowUp, Mods::default());
        assert_eq!(current(&chat), Some(1));
        tab(&mut chat);
        assert_eq!(chat.input, "/gamerule");
        tab(&mut chat);
        assert_eq!(chat.input, "/gamemode");
        press(&mut chat, Key::ArrowUp, Mods::default());
        tab(&mut chat);
        assert_eq!(chat.input, "/gamerule");
        assert_eq!(chat.history_pos, chat.history.len());
    }

    #[test]
    fn escape_hides_the_list_first_and_closes_the_chat_second() {
        let mut chat = opened(Opener::Command);
        type_text(&mut chat, "give @s pick");
        assert_eq!(
            shown(&chat),
            ["minecraft:diamond_pickaxe", "minecraft:iron_pickaxe"]
        );
        press(&mut chat, Key::Escape, Mods::default());
        assert!(chat.open && chat.suggestions.list.is_none());
        assert_eq!(chat.suggestions.inline, None);
        assert_eq!(chat.input, "/give @s pick");
        press(&mut chat, Key::Escape, Mods::default());
        assert!(!chat.open);
    }

    #[test]
    fn up_and_down_walk_the_history_only_while_the_list_is_closed() {
        let mut chat = opened(Opener::Chat);
        press(&mut chat, Key::ArrowUp, Mods::default());
        assert_eq!(chat.input, "hello");
        press(&mut chat, Key::ArrowUp, Mods::default());
        assert_eq!(chat.input, "/seed");
        assert!(chat.suggestions.list.is_none());
        assert!(!chat.suggestions.allow);
        assert_eq!(chat.suggestions.spans, vec![(0..5, GRAY)]);
        tab(&mut chat);
        assert!(chat.suggestions.list.is_none());
        type_text(&mut chat, " ");
        press(&mut chat, Key::ArrowUp, Mods::default());
        assert_eq!(chat.input, "/seed ");
        let mut list = opened(Opener::Command);
        type_text(&mut list, "s");
        assert!(list.suggestions.list.is_some());
        press(&mut list, Key::ArrowUp, Mods::default());
        assert_eq!(list.input, "/s");
    }

    #[test]
    fn tab_opens_the_list_where_typing_does_not() {
        let mut chat = opened(Opener::Command);
        tab(&mut chat);
        assert!(shown(&chat).contains(&"gamemode"));
        assert_eq!(chat.input, "/");
        let mut talk = opened(Opener::Chat);
        type_text(&mut talk, "hi st");
        assert!(talk.suggestions.list.is_none());
        tab(&mut talk);
        assert_eq!(shown(&talk), ["Steve"]);
        tab(&mut talk);
        assert_eq!((talk.input.as_str(), talk.cursor), ("hi Steve", 8));
        assert!(talk.suggestions.spans.is_empty());
        let mut blank = opened(Opener::Chat);
        type_text(&mut blank, "  ");
        tab(&mut blank);
        assert!(blank.suggestions.pending.is_none() && blank.suggestions.list.is_none());
    }

    #[test]
    fn usage_shows_grey_when_nothing_matches_and_the_error_when_parsing_fails() {
        let mut chat = opened(Opener::Command);
        type_text(&mut chat, "give ");
        assert_eq!(chat.suggestions.usage_start, 6);
        assert!(chat.suggestions.list.is_some());
        press(&mut chat, Key::Escape, Mods::default());
        let usage: Vec<String> = chat
            .suggestions
            .usage
            .iter()
            .map(|line| line.resolve(&|_| None).text)
            .collect();
        assert_eq!(usage, ["<targets> <item> [<count>]"]);
        let mut wrong = opened(Opener::Command);
        type_text(&mut wrong, "gamemode god");
        assert!(wrong.suggestions.list.is_none());
        let lang = |key: &str| {
            (key == "command.context.parse_error").then(|| "%s at position %s: %s".to_owned())
        };
        let line = wrong.suggestions.usage[0].resolve(&lang);
        assert!(
            line.text
                .ends_with(" at position 13: ...memode god<--[HERE]"),
            "{}",
            line.text
        );
        assert_eq!(wrong.suggestions.spans.last(), Some(&(10..13, RED)));
    }

    #[test]
    fn arguments_take_their_vanilla_colours() {
        let mut chat = opened(Opener::Command);
        type_text(&mut chat, "kill Steve");
        assert_eq!(chat.suggestions.spans, vec![(0..6, GRAY), (6..11, AQUA)]);
        assert_eq!(
            input_accents(&chat.suggestions.spans, 3, 6),
            vec![(0..3, opaque(GRAY)), (3..6, opaque(AQUA))]
        );
        assert!(input_accents(&chat.suggestions.spans, 11, 0).is_empty());
    }

    #[test]
    fn the_list_scrolls_ten_lines_at_a_time_and_wraps() {
        let mut chat = many(14);
        let offset = |chat: &McChat| chat.suggestions.list.as_ref().map(|list| list.offset);
        for _ in 0..9 {
            cycle(&mut chat, 1);
        }
        assert_eq!((current(&chat), offset(&chat)), (Some(9), Some(0)));
        cycle(&mut chat, 1);
        assert_eq!((current(&chat), offset(&chat)), (Some(10), Some(1)));
        for _ in 0..3 {
            cycle(&mut chat, 1);
        }
        assert_eq!((current(&chat), offset(&chat)), (Some(13), Some(4)));
        cycle(&mut chat, 1);
        assert_eq!((current(&chat), offset(&chat)), (Some(0), Some(0)));
        cycle(&mut chat, -1);
        assert_eq!((current(&chat), offset(&chat)), (Some(13), Some(4)));
    }

    #[test]
    fn the_list_sits_above_the_input_at_the_word_it_completes() {
        let mut chat = opened(Opener::Command);
        type_text(&mut chat, "give @s pick");
        let list = chat.suggestions.list.clone().unwrap();
        let start = TEXT_X + width("/give @s ");
        let widest = width("minecraft:diamond_pickaxe");
        assert_eq!(
            list_rect(width, &list, 240, 316),
            [start - 1, 240 - 15 - 24, widest + 1, 24]
        );
        assert_eq!(
            list_rect(width, &list, 240, 100)[0],
            TEXT_X + 100 - widest - 1
        );
        let many = many(14).suggestions.list.unwrap();
        assert_eq!(
            list_rect(width, &many, 240, 316)[1..],
            [240 - 15 - 120, 19, 120]
        );
        assert_eq!(usage_position(width, "/give ", 6, 120, 316), TEXT_X + 36);
        assert_eq!(
            usage_position(width, "/give ", 6, 400, 316),
            TEXT_X + 316 - 400
        );
        assert_eq!(screen_x(width, "/give", 9), TEXT_X);
        assert_eq!((usage_top(240, 0), usage_top(240, 1)), (213, 201));
    }

    #[test]
    fn closing_the_chat_forgets_the_suggestions() {
        let mut chat = opened(Opener::Command);
        type_text(&mut chat, "ga");
        chat.close();
        assert!(chat.suggestions.list.is_none() && chat.suggestions.pending.is_none());
        open(&mut chat, Opener::Chat, 0.0);
        assert!(chat.suggestions.spans.is_empty() && chat.suggestions.inline.is_none());
    }
}
