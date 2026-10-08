use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use frame::minecraft_chat::{MC_CHAT_HISTORY, MC_CHAT_INPUT_LIMIT};
use frame::{McAction, McChat, McChatLine, McFont, MinecraftUi};
use minecraft_commands::protocol::{allowed_in_chat, normalize_chat};

use crate::minecraft_gui::{Painter, TextLine};

const CHAT_WIDTH: i32 = 320;
const LINE_HEIGHT: i32 = 9;
const TEXT_ABOVE_BOTTOM: i32 = 8;
const BOTTOM_MARGIN: i32 = 40;
const FOCUSED_LINES: usize = 20;
const UNFOCUSED_LINES: usize = 10;
const FADE_TICKS: f64 = 200.0;
const TEXT_OPACITY: f64 = 1.0;
const BACKGROUND_OPACITY: f64 = 0.5;
const INPUT_BACKGROUND: u32 = 0x8000_0000;
const INPUT_TEXT: u32 = 0xFFE0_E0E0;
const CURSOR: u32 = 0xFFD0_D0D0;
const SCROLL_NOTCH: f32 = 7.0;
const BLINK_MILLIS: f64 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opener {
    Chat,
    Command,
}

pub(crate) fn alpha(age_ticks: f64, focused: bool) -> f64 {
    if focused {
        return 1.0;
    }
    let left = ((1.0 - age_ticks / FADE_TICKS) * 10.0).clamp(0.0, 1.0);
    left * left
}

pub(crate) fn visible_lines(focused: bool) -> usize {
    if focused {
        FOCUSED_LINES
    } else {
        UNFOCUSED_LINES
    }
}

pub(crate) fn chat_bottom(height: i32) -> i32 {
    height - BOTTOM_MARGIN
}

pub(crate) fn line_rect(chat_bottom: i32, i: usize) -> [i32; 4] {
    let bottom = chat_bottom - i as i32 * LINE_HEIGHT;
    [0, bottom - LINE_HEIGHT, CHAT_WIDTH + 12, bottom]
}

pub(crate) fn scrollbar(
    chat_bottom: i32,
    total: usize,
    drawn: usize,
    scroll: usize,
) -> Option<([i32; 2], u32)> {
    let (total, drawn, scroll) = (total as i32, drawn as i32, scroll as i32);
    if total <= 0 {
        return None;
    }
    let shown = drawn * LINE_HEIGHT;
    let all = total * LINE_HEIGHT;
    if shown == all {
        return None;
    }
    let offset = scroll * shown / total - chat_bottom;
    let height = shown * shown / all;
    let alpha = if offset > 0 { 170 } else { 96 };
    Some(([-offset - height, -offset], alpha))
}

fn with_alpha(rgb: u32, alpha: f64) -> u32 {
    (((alpha.clamp(0.0, 1.0) * 255.0).floor() as u32) << 24) | (rgb & 0x00FF_FFFF)
}

pub(crate) fn opener(
    chat: bool,
    command: bool,
    command_bound: bool,
    typed: &[(Key, Option<String>)],
) -> Option<Opener> {
    let slash_typed = typed.iter().any(|(_, text)| text.as_deref() == Some("/"));
    if command {
        Some(Opener::Command)
    } else if chat {
        Some(Opener::Chat)
    } else if command_bound && slash_typed {
        Some(Opener::Command)
    } else {
        None
    }
}

pub(crate) fn open(chat: &mut McChat, opener: Opener, now: f64) {
    chat.open = true;
    chat.input = match opener {
        Opener::Chat => String::new(),
        Opener::Command => "/".to_owned(),
    };
    chat.cursor = chat.input.len();
    chat.history_pos = chat.history.len();
    chat.draft.clear();
    chat.scroll = 0;
    chat.new_since_scroll = false;
    chat.opened_at = now;
}

pub(crate) fn handle_frame(
    chat: &mut McChat,
    opener: Option<Opener>,
    typed: &[(Key, Option<String>)],
    ctrl: bool,
    now: f64,
) {
    if !chat.open {
        if let Some(opener) = opener {
            open(chat, opener, now);
        }
        return;
    }
    for (key, text) in typed {
        if !chat.open {
            break;
        }
        apply_key(chat, key, text.as_deref(), ctrl);
    }
}

pub(crate) fn apply_key(chat: &mut McChat, key: &Key, text: Option<&str>, ctrl: bool) {
    match key {
        Key::Escape => chat.close(),
        Key::Enter => {
            send(chat);
            chat.close();
        }
        Key::ArrowUp => move_in_history(chat, -1),
        Key::ArrowDown => move_in_history(chat, 1),
        Key::PageUp => scroll_chat(chat, FOCUSED_LINES as i32 - 1),
        Key::PageDown => scroll_chat(chat, 1 - FOCUSED_LINES as i32),
        Key::Backspace => {
            let to = if ctrl {
                word_position(&chat.input, chat.cursor, -1)
            } else {
                previous_char(&chat.input, chat.cursor)
            };
            chat.input.replace_range(to..chat.cursor, "");
            chat.cursor = to;
        }
        Key::Delete => {
            let to = if ctrl {
                word_position(&chat.input, chat.cursor, 1)
            } else {
                next_char(&chat.input, chat.cursor)
            };
            chat.input.replace_range(chat.cursor..to, "");
        }
        Key::ArrowLeft => {
            chat.cursor = if ctrl {
                word_position(&chat.input, chat.cursor, -1)
            } else {
                previous_char(&chat.input, chat.cursor)
            };
        }
        Key::ArrowRight => {
            chat.cursor = if ctrl {
                word_position(&chat.input, chat.cursor, 1)
            } else {
                next_char(&chat.input, chat.cursor)
            };
        }
        Key::Home => chat.cursor = 0,
        Key::End => chat.cursor = chat.input.len(),
        _ => {
            if let Some(text) = text {
                insert(chat, text);
            }
        }
    }
}

fn insert(chat: &mut McChat, text: &str) {
    let room = MC_CHAT_INPUT_LIMIT.saturating_sub(chat.input.chars().count());
    let added: String = text
        .chars()
        .filter(|c| allowed_in_chat(*c))
        .take(room)
        .collect();
    chat.input.insert_str(chat.cursor, &added);
    chat.cursor += added.len();
}

fn send(chat: &mut McChat) {
    let message = normalize_chat(&chat.input);
    if message.is_empty() {
        return;
    }
    if chat.history.last() != Some(&message) {
        if chat.history.len() >= MC_CHAT_HISTORY {
            chat.history.remove(0);
        }
        chat.history.push(message.clone());
    }
    chat.sent.push(message);
}

fn move_in_history(chat: &mut McChat, dir: i32) {
    let len = chat.history.len();
    let target = (chat.history_pos as i32 + dir).clamp(0, len as i32) as usize;
    if target == chat.history_pos {
        return;
    }
    if target == len {
        chat.input = chat.draft.clone();
    } else {
        if chat.history_pos == len {
            chat.draft = chat.input.clone();
        }
        chat.input = chat.history[target].clone();
    }
    chat.history_pos = target;
    chat.cursor = chat.input.len();
}

pub(crate) fn scroll_chat(chat: &mut McChat, delta: i32) {
    let max = chat.lines.len().saturating_sub(FOCUSED_LINES) as i32;
    let pos = (chat.scroll as i32 + delta).min(max);
    if pos <= 0 {
        chat.scroll = 0;
        chat.new_since_scroll = false;
    } else {
        chat.scroll = pos as usize;
    }
}

fn wheel_delta(y: f32, shift: bool) -> i32 {
    let y = y.clamp(-1.0, 1.0);
    (if shift { y } else { y * SCROLL_NOTCH }) as i32
}

fn previous_char(text: &str, at: usize) -> usize {
    text[..at]
        .chars()
        .next_back()
        .map_or(0, |c| at - c.len_utf8())
}

fn next_char(text: &str, at: usize) -> usize {
    text[at..]
        .chars()
        .next()
        .map_or(text.len(), |c| at + c.len_utf8())
}

fn word_position(text: &str, at: usize, dir: i32) -> usize {
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
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

pub(crate) fn wrap(font: Option<&McFont>, line: &McChatLine) -> Vec<McChatLine> {
    let parts = font.map_or_else(
        || vec![line.text.clone()],
        |font| font.split(&line.text, CHAT_WIDTH),
    );
    let mut from = 0;
    parts
        .into_iter()
        .map(|part| {
            let at = line
                .text
                .get(from..)
                .and_then(|rest| rest.find(part.as_str()))
                .map_or(from, |found| from + found);
            let end = (at + part.len()).min(line.text.len());
            from = end;
            let spans = line
                .spans
                .iter()
                .filter_map(|(range, color)| {
                    let (start, stop) = (range.start.max(at), range.end.min(end));
                    (start < stop).then(|| (start - at..stop - at, *color))
                })
                .collect();
            McChatLine {
                text: part,
                color: line.color,
                spans,
                italic: line.italic,
            }
        })
        .collect()
}

pub(crate) fn receive(chat: &mut McChat, font: Option<&McFont>, now: f64) {
    for line in std::mem::take(&mut chat.received) {
        for part in wrap(font, &line) {
            if chat.open && chat.scroll > 0 {
                chat.new_since_scroll = true;
                scroll_chat(chat, 1);
            }
            chat.lines.push_front((part, now));
        }
    }
    chat.lines.truncate(MC_CHAT_HISTORY);
}

fn input_view(font: &McFont, input: &str, cursor: usize, inner: i32) -> usize {
    if font.width(&input[..cursor]) <= inner {
        return 0;
    }
    let mut start = cursor;
    while start > 0 {
        let before = previous_char(input, start);
        if font.width(&input[before..cursor]) > inner {
            break;
        }
        start = before;
    }
    start
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_chat(
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    mut wheel: MessageReader<MouseWheel>,
    ui: Option<ResMut<MinecraftUi>>,
    menus: Res<crate::menus::ScriptMenus>,
    hud_input: Option<Res<frame::HudInputView>>,
    mc_keys: Option<Res<frame::McActionInput>>,
    time: Res<Time>,
) {
    let events: Vec<(Key, Option<String>)> = typed
        .read()
        .filter(|event| event.state.is_pressed())
        .map(|event| {
            (
                event.logical_key.clone(),
                event.text.as_ref().map(ToString::to_string),
            )
        })
        .collect();
    let notches: Vec<f32> = wheel.read().map(|event| event.y).collect();
    let Some(mut ui) = ui else {
        return;
    };
    let now = time.elapsed_secs_f64();
    let ui = &mut *ui;
    receive(&mut ui.chat, ui.gui.as_deref().map(|gui| &gui.font), now);
    let blocked =
        menus.captures_input() || hud_input.as_ref().is_some_and(|input| input.console_open);
    if !ui.active || blocked {
        if ui.chat.open {
            ui.chat.close();
        }
        return;
    }
    let pressed = |action: McAction| {
        mc_keys
            .as_deref()
            .is_some_and(|keys| keys.just_pressed(action))
    };
    let command_bound = mc_keys
        .as_deref()
        .is_some_and(|keys| keys.is_bound(McAction::Command));
    let opener = opener(
        pressed(McAction::Chat),
        pressed(McAction::Command),
        command_bound,
        &events,
    )
    .filter(|_| !ui.inventory_open);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let was_open = ui.chat.open;
    handle_frame(&mut ui.chat, opener, &events, ctrl, now);
    if was_open && ui.chat.open {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        for y in notches {
            scroll_chat(&mut ui.chat, wheel_delta(y, shift));
        }
    }
}

pub(crate) fn draw(p: &mut Painter, chat: &McChat, now: f64) {
    let bottom = chat_bottom(p.space.height);
    let focused = chat.open;
    let shown = chat
        .lines
        .len()
        .saturating_sub(chat.scroll)
        .min(visible_lines(focused));
    let mut visible = Vec::new();
    for i in (0..shown).rev() {
        let (line, added) = &chat.lines[i + chat.scroll];
        let a = alpha((now - added) * 20.0, focused);
        if a > 1e-5 {
            visible.push((i, line, a));
        }
    }
    for &(i, _, a) in &visible {
        let [x0, y0, x1, y1] = line_rect(bottom, i);
        p.fill(x0, y0, x1, y1, with_alpha(0, a * BACKGROUND_OPACITY));
    }
    for &(i, line, a) in &visible {
        let [_, _, _, line_bottom] = line_rect(bottom, i);
        let opacity = a * TEXT_OPACITY;
        let text = TextLine {
            text: line.text.clone(),
            color: with_alpha(line.color, opacity),
            italic: line.italic,
            accents: line
                .spans
                .iter()
                .map(|(range, color)| (range.clone(), with_alpha(*color, opacity)))
                .collect(),
        };
        p.line(false, &text, 4, line_bottom - TEXT_ABOVE_BOTTOM, true);
    }
    if !focused {
        return;
    }
    if let Some(([top, bar_bottom], alpha)) =
        scrollbar(bottom, chat.lines.len(), visible.len(), chat.scroll)
    {
        let color = if chat.new_since_scroll {
            0x00CC_3333
        } else {
            0x0033_33AA
        };
        let x = CHAT_WIDTH + 8;
        p.fill(x, top, x + 2, bar_bottom, (alpha << 24) | color);
        p.fill(x + 1, top, x + 2, bar_bottom, (alpha << 24) | 0x00CC_CCCC);
    }
    let (width, height) = (p.space.width, p.space.height);
    p.fill(2, height - 14, width - 2, height - 2, INPUT_BACKGROUND);
    let gui = p.gui;
    let font = &gui.font;
    let inner = width - 4;
    let cursor = chat.cursor.min(chat.input.len());
    let start = input_view(font, &chat.input, cursor, inner);
    let shown_text = font.head_by_width(&chat.input[start..], inner);
    let before = &chat.input[start..cursor];
    let before_width = font.width(before);
    let y = height - 10;
    p.text(&shown_text, 4, y, INPUT_TEXT, true);
    if ((now - chat.opened_at) * 1000.0 / BLINK_MILLIS) as i64 % 2 != 0 {
        return;
    }
    let insert = cursor < chat.input.len() || chat.input.chars().count() >= MC_CHAT_INPUT_LIMIT;
    let end = if before.is_empty() {
        4
    } else {
        4 + before_width + 1
    };
    if insert {
        p.fill(end - 1, y - 1, end, y + 10, CURSOR);
    } else {
        p.text("_", end, y, INPUT_TEXT, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::McGlyph;

    fn key(c: &str) -> (Key, Option<String>) {
        (Key::Character(c.into()), Some(c.to_owned()))
    }

    fn typed(chat: &mut McChat, text: &str) {
        for c in text.chars() {
            let (k, t) = key(&c.to_string());
            apply_key(chat, &k, t.as_deref(), false);
        }
    }

    fn open_chat() -> McChat {
        let mut chat = McChat::default();
        open(&mut chat, Opener::Chat, 0.0);
        chat
    }

    fn font() -> McFont {
        let mut font = McFont::default();
        let glyph = McGlyph {
            rect: [0, 0, 5, 8],
            scale: 1.0,
            ascent: 7,
            advance: 6.0,
        };
        for c in 'a'..='z' {
            font.glyphs.insert(c, glyph);
        }
        font.spaces.insert(' ', 4.0);
        font
    }

    #[test]
    fn lines_stay_solid_nine_seconds_then_fade_quadratically() {
        assert_eq!(alpha(0.0, false), 1.0);
        assert!((alpha(180.0, false) - 1.0).abs() < 1e-9);
        assert!((alpha(150.0, false) - 1.0).abs() < 1e-9);
        assert!((alpha(190.0, false) - 0.25).abs() < 1e-9);
        assert_eq!(alpha(200.0, false), 0.0);
        assert_eq!(alpha(5000.0, false), 0.0);
        assert_eq!(alpha(5000.0, true), 1.0);
    }

    #[test]
    fn unfocused_shows_ten_lines_focused_twenty() {
        assert_eq!(visible_lines(false), 10);
        assert_eq!(visible_lines(true), 20);
    }

    #[test]
    fn line_backgrounds_span_0_to_332_and_text_sits_8_above_the_bottom() {
        let bottom = chat_bottom(240);
        assert_eq!(bottom, 200);
        assert_eq!(line_rect(bottom, 0), [0, 191, 332, 200]);
        assert_eq!(line_rect(bottom, 2), [0, 173, 332, 182]);
        assert_eq!(line_rect(bottom, 0)[3] - TEXT_ABOVE_BOTTOM, 192);
        assert_eq!(with_alpha(0, 0.5), 0x7F00_0000);
    }

    #[test]
    fn the_scrollbar_shows_only_when_lines_are_hidden() {
        assert_eq!(scrollbar(200, 15, 15, 0), None);
        assert_eq!(scrollbar(200, 40, 20, 0), Some(([110, 200], 96)));
        assert_eq!(scrollbar(200, 40, 20, 20), Some(([20, 110], 96)));
        assert_eq!(scrollbar(200, 0, 0, 0), None);
    }

    #[test]
    fn t_opens_empty_and_slash_opens_with_a_slash_without_typing_the_key() {
        let mut chat = McChat::default();
        let t = [key("t")];
        handle_frame(&mut chat, opener(true, false, true, &t), &t, false, 2.0);
        assert!(chat.open);
        assert_eq!(chat.input, "");
        assert_eq!(chat.opened_at, 2.0);
        let mut chat = McChat::default();
        let slash = [key("/")];
        handle_frame(
            &mut chat,
            opener(false, true, true, &slash),
            &slash,
            false,
            0.0,
        );
        assert_eq!((chat.input.as_str(), chat.cursor), ("/", 1));
        let mut chat = McChat::default();
        let shift_seven = [key("/")];
        assert_eq!(
            opener(false, false, true, &shift_seven),
            Some(Opener::Command)
        );
        handle_frame(
            &mut chat,
            opener(false, false, true, &shift_seven),
            &shift_seven,
            false,
            0.0,
        );
        assert_eq!(chat.input, "/");
        assert_eq!(opener(false, false, true, &[key("t")]), None);
        let mut chat = McChat::default();
        handle_frame(&mut chat, None, &[key("t")], false, 0.0);
        assert!(!chat.open && chat.input.is_empty());
        handle_frame(&mut chat, Some(Opener::Chat), &[], false, 0.0);
        handle_frame(
            &mut chat,
            opener(true, false, true, &[key("t")]),
            &[key("t")],
            false,
            0.0,
        );
        assert_eq!(chat.input, "t");
    }

    #[test]
    fn the_bound_keys_win_over_a_typed_slash() {
        let slash = [key("/")];
        assert_eq!(opener(true, false, true, &slash), Some(Opener::Chat));
        assert_eq!(opener(true, true, true, &slash), Some(Opener::Command));
        assert_eq!(opener(false, false, false, &slash), None);
        assert_eq!(opener(false, true, false, &slash), Some(Opener::Command));
        assert_eq!(opener(true, false, false, &slash), Some(Opener::Chat));
    }

    #[test]
    fn enter_sends_normalized_text_and_closes() {
        let mut chat = open_chat();
        typed(&mut chat, "  hi   there ");
        apply_key(&mut chat, &Key::Enter, Some("\r"), false);
        assert!(!chat.open);
        assert!(chat.input.is_empty());
        assert_eq!(chat.sent, vec!["hi there".to_owned()]);
        assert_eq!(chat.history, vec!["hi there".to_owned()]);
        let mut again = open_chat();
        again.history = chat.history.clone();
        typed(&mut again, "hi there");
        apply_key(&mut again, &Key::Enter, None, false);
        assert_eq!(again.history.len(), 1);
        let mut empty = open_chat();
        typed(&mut empty, "   ");
        apply_key(&mut empty, &Key::Enter, None, false);
        assert!(!empty.open && empty.sent.is_empty() && empty.history.is_empty());
        let mut full = open_chat();
        full.history = (0..MC_CHAT_HISTORY).map(|i| i.to_string()).collect();
        typed(&mut full, "new");
        apply_key(&mut full, &Key::Enter, None, false);
        assert_eq!(full.history.len(), MC_CHAT_HISTORY);
        assert_eq!(full.history.first().map(String::as_str), Some("1"));
        assert_eq!(full.history.last().map(String::as_str), Some("new"));
    }

    #[test]
    fn escape_closes_without_sending() {
        let mut chat = open_chat();
        typed(&mut chat, "secret");
        apply_key(&mut chat, &Key::Escape, None, false);
        assert!(!chat.open);
        assert!(chat.input.is_empty() && chat.sent.is_empty() && chat.history.is_empty());
    }

    #[test]
    fn up_and_down_walk_the_history_and_restore_the_draft() {
        let mut chat = McChat {
            history: vec!["one".into(), "two".into()],
            ..McChat::default()
        };
        open(&mut chat, Opener::Chat, 0.0);
        typed(&mut chat, "draft");
        apply_key(&mut chat, &Key::ArrowUp, None, false);
        assert_eq!((chat.input.as_str(), chat.cursor), ("two", 3));
        apply_key(&mut chat, &Key::ArrowUp, None, false);
        assert_eq!(chat.input, "one");
        apply_key(&mut chat, &Key::ArrowUp, None, false);
        assert_eq!(chat.input, "one");
        apply_key(&mut chat, &Key::ArrowDown, None, false);
        assert_eq!(chat.input, "two");
        apply_key(&mut chat, &Key::ArrowDown, None, false);
        assert_eq!((chat.input.as_str(), chat.cursor), ("draft", 5));
        apply_key(&mut chat, &Key::ArrowDown, None, false);
        assert_eq!(chat.input, "draft");
    }

    #[test]
    fn backspace_and_cursor_moves_respect_char_boundaries() {
        let mut chat = open_chat();
        typed(&mut chat, "café au");
        apply_key(&mut chat, &Key::ArrowLeft, None, false);
        apply_key(&mut chat, &Key::ArrowLeft, None, false);
        apply_key(&mut chat, &Key::ArrowLeft, None, false);
        assert_eq!(chat.cursor, "café".len());
        apply_key(&mut chat, &Key::Backspace, None, false);
        assert_eq!(chat.input, "caf au");
        apply_key(&mut chat, &Key::ArrowLeft, None, false);
        apply_key(&mut chat, &Key::Delete, None, false);
        assert_eq!(chat.input, "ca au");
        typed(&mut chat, "é");
        assert_eq!(chat.input, "caé au");
        apply_key(&mut chat, &Key::End, None, false);
        apply_key(&mut chat, &Key::Backspace, None, true);
        assert_eq!(chat.input, "caé ");
        apply_key(&mut chat, &Key::Backspace, None, true);
        assert_eq!(chat.input, "");
        typed(&mut chat, "ab cd");
        apply_key(&mut chat, &Key::Home, None, false);
        assert_eq!(chat.cursor, 0);
        apply_key(&mut chat, &Key::ArrowRight, None, true);
        assert_eq!(chat.cursor, 3);
        apply_key(&mut chat, &Key::ArrowLeft, None, false);
        apply_key(&mut chat, &Key::Backspace, None, false);
        apply_key(&mut chat, &Key::Home, None, false);
        apply_key(&mut chat, &Key::Backspace, None, false);
        assert_eq!((chat.input.as_str(), chat.cursor), ("a cd", 0));
        typed(&mut chat, "\u{8}\u{a7}x");
        assert_eq!(chat.input, "xa cd");
    }

    #[test]
    fn wrapped_lines_split_at_320_and_keep_at_most_100() {
        let font = font();
        let long = McChatLine {
            text: format!("{} {}", "a".repeat(40), "b".repeat(30)),
            color: 0xFFFFFF,
            spans: vec![(38..45, 0xFF5555)],
            italic: false,
        };
        let parts = wrap(Some(&font), &long);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].text, "a".repeat(40));
        assert_eq!(parts[1].text, "b".repeat(30));
        assert_eq!(parts[0].spans, vec![(38..40, 0xFF5555)]);
        assert_eq!(parts[1].spans, vec![(0..4, 0xFF5555)]);
        let mut chat = McChat::default();
        chat.received.push(long);
        receive(&mut chat, Some(&font), 1.0);
        assert_eq!(chat.lines[0].0.text, "b".repeat(30));
        assert_eq!(chat.lines[1].0.text, "a".repeat(40));
        for _ in 0..120 {
            chat.received.push(McChatLine {
                text: "x".into(),
                ..McChatLine::default()
            });
        }
        receive(&mut chat, Some(&font), 2.0);
        assert_eq!(chat.lines.len(), MC_CHAT_HISTORY);
    }

    #[test]
    fn scrolling_holds_the_view_while_new_lines_arrive() {
        let mut chat = open_chat();
        for _ in 0..40 {
            chat.received.push(McChatLine {
                text: "x".into(),
                ..McChatLine::default()
            });
        }
        receive(&mut chat, None, 0.0);
        apply_key(&mut chat, &Key::PageUp, None, false);
        assert_eq!(chat.scroll, 19);
        chat.received.push(McChatLine {
            text: "y".into(),
            ..McChatLine::default()
        });
        receive(&mut chat, None, 1.0);
        assert_eq!(chat.scroll, 20);
        assert!(chat.new_since_scroll);
        apply_key(&mut chat, &Key::PageUp, None, false);
        assert_eq!(chat.scroll, 21);
        scroll_chat(&mut chat, wheel_delta(-1.0, false));
        assert_eq!(chat.scroll, 14);
        scroll_chat(&mut chat, wheel_delta(-1.0, true));
        assert_eq!(chat.scroll, 13);
        scroll_chat(&mut chat, wheel_delta(-1.0, false) * 10);
        assert_eq!(chat.scroll, 0);
        assert!(!chat.new_since_scroll);
        assert_eq!(wheel_delta(3.0, false), 7);
    }

    #[test]
    fn input_is_capped_at_256_chars() {
        let mut chat = open_chat();
        let (k, _) = key("a");
        apply_key(&mut chat, &k, Some(&"é".repeat(300)), false);
        assert_eq!(chat.input.chars().count(), MC_CHAT_INPUT_LIMIT);
        typed(&mut chat, "z");
        assert_eq!(chat.input.chars().count(), MC_CHAT_INPUT_LIMIT);
        assert!(!chat.input.contains('z'));
    }

    #[test]
    fn the_input_view_follows_the_cursor() {
        let font = font();
        let text = "a".repeat(100);
        assert_eq!(input_view(&font, &text, 10, 316), 0);
        assert_eq!(input_view(&font, &text, 100, 316), 48);
        assert_eq!(input_view(&font, "abc", 3, 316), 0);
    }
}
