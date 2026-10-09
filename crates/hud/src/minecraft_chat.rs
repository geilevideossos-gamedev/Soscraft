use std::ops::Range;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use frame::minecraft_chat::{MC_CHAT_HISTORY, MC_CHAT_INPUT_LIMIT};
use frame::{McAction, McChat, McChatLine, McFont, MinecraftUi};
use minecraft_commands::protocol::normalize_chat;

use crate::minecraft_edit::{Edit, previous_char};
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
const SELECTION: u32 = 0xFFC0_C0FF;
const SELECTED_TEXT: u32 = 0xFF1F_1FFF;
const MODIFIERS: [KeyCode; 6] = [
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
    KeyCode::AltLeft,
    KeyCode::AltRight,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opener {
    Chat,
    Command,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Mods {
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
    pub(crate) alt: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Typed {
    pub(crate) key: Key,
    pub(crate) code: KeyCode,
    pub(crate) text: Option<String>,
    pub(crate) mods: Mods,
}

pub(crate) fn held_before(pressed: bool, just_pressed: bool, just_released: bool) -> bool {
    if just_pressed == just_released {
        pressed
    } else {
        !pressed
    }
}

pub(crate) fn fold_typed(mut held: [bool; 6], events: &[KeyboardInput]) -> Vec<Typed> {
    let mut typed = Vec::new();
    for event in events {
        if let Some(i) = MODIFIERS.iter().position(|code| *code == event.key_code) {
            held[i] = event.state.is_pressed();
        } else if event.state.is_pressed() {
            typed.push(Typed {
                key: event.logical_key.clone(),
                code: event.key_code,
                text: event.text.as_ref().map(ToString::to_string),
                mods: Mods {
                    ctrl: held[0] || held[1],
                    shift: held[2] || held[3],
                    alt: held[4] || held[5],
                },
            });
        }
    }
    typed
}

fn shortcut(typed: &Typed) -> Option<char> {
    let Mods { ctrl, shift, alt } = typed.mods;
    if !ctrl || shift || alt {
        return None;
    }
    let mut chars = match &typed.key {
        Key::Character(text) => text.chars(),
        _ => "".chars(),
    };
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii() => Some(c.to_ascii_lowercase()),
        (Some(_), None) => match typed.code {
            KeyCode::KeyA => Some('a'),
            KeyCode::KeyC => Some('c'),
            KeyCode::KeyV => Some('v'),
            KeyCode::KeyX => Some('x'),
            _ => None,
        },
        _ => None,
    }
}

fn edit(chat: &mut McChat) -> Edit<'_> {
    Edit::new(
        &mut chat.input,
        &mut chat.cursor,
        &mut chat.highlight,
        MC_CHAT_INPUT_LIMIT,
    )
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
    typed: &[Typed],
) -> Option<Opener> {
    let slash_typed = typed.iter().any(|typed| typed.text.as_deref() == Some("/"));
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
    chat.highlight = chat.cursor;
    chat.history_pos = chat.history.len();
    chat.draft.clear();
    chat.scroll = 0;
    chat.new_since_scroll = false;
    chat.opened_at = now;
}

pub(crate) fn handle_frame(chat: &mut McChat, opener: Option<Opener>, typed: &[Typed], now: f64) {
    if !chat.open {
        if let Some(opener) = opener {
            open(chat, opener, now);
        }
        return;
    }
    for typed in typed {
        if !chat.open {
            break;
        }
        apply_typed(chat, typed);
    }
}

pub(crate) fn apply_typed(chat: &mut McChat, typed: &Typed) {
    let Mods { ctrl, shift, alt } = typed.mods;
    match typed.key {
        Key::Escape => chat.close(),
        Key::Enter => {
            send(chat);
            chat.close();
        }
        Key::ArrowUp => move_in_history(chat, -1),
        Key::ArrowDown => move_in_history(chat, 1),
        Key::PageUp => scroll_chat(chat, FOCUSED_LINES as i32 - 1),
        Key::PageDown => scroll_chat(chat, 1 - FOCUSED_LINES as i32),
        Key::Backspace => edit(chat).delete(-1, ctrl),
        Key::Delete => edit(chat).delete(1, ctrl),
        Key::ArrowLeft => edit(chat).step(-1, ctrl, shift),
        Key::ArrowRight => edit(chat).step(1, ctrl, shift),
        Key::Home => edit(chat).move_to(0, shift),
        Key::End => edit(chat).move_to(usize::MAX, shift),
        _ => match shortcut(typed) {
            Some('a') => edit(chat).select_all(),
            _ => {
                if let Some(text) = typed.text.as_deref().filter(|_| !ctrl || alt) {
                    edit(chat).insert(text);
                }
            }
        },
    }
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
    chat.highlight = chat.cursor;
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

pub(crate) fn selection_box(
    shown: &str,
    [cursor, highlight]: [usize; 2],
    insert: bool,
    inner: i32,
    width: impl Fn(&str) -> i32,
) -> Option<([i32; 2], Range<usize>)> {
    if cursor == highlight {
        return None;
    }
    let cursor_x = if shown.is_empty() {
        4
    } else {
        4 + width(&shown[..cursor]) + i32::from(!insert)
    };
    let highlight_x = 4 + width(&shown[..highlight]) - 1;
    let right = 4 + inner;
    Some((
        [cursor_x.min(right), highlight_x.min(right)],
        cursor.min(highlight)..cursor.max(highlight),
    ))
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
    let events: Vec<KeyboardInput> = typed.read().cloned().collect();
    let held = MODIFIERS.map(|code| {
        held_before(
            keys.pressed(code),
            keys.just_pressed(code),
            keys.just_released(code),
        )
    });
    let events = fold_typed(held, &events);
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
    let was_open = ui.chat.open;
    handle_frame(&mut ui.chat, opener, &events, now);
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
    let insert = cursor < chat.input.len() || chat.input.chars().count() >= MC_CHAT_INPUT_LIMIT;
    let ends = [cursor, chat.highlight].map(|at| at.saturating_sub(start).min(shown_text.len()));
    if let Some(([x0, x1], selected)) =
        selection_box(&shown_text, ends, insert, inner, |text| font.width(text))
    {
        p.fill(x0, y - 1, x1, y + 10, SELECTION);
        let x = 4 + font.width(&shown_text[..selected.start]);
        p.text(&shown_text[selected], x, y, SELECTED_TEXT, false);
    }
    if ((now - chat.opened_at) * 1000.0 / BLINK_MILLIS) as i64 % 2 != 0 {
        return;
    }
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
    use bevy::input::keyboard::NativeKeyCode;
    use frame::McGlyph;

    const NO_CODE: KeyCode = KeyCode::Unidentified(NativeKeyCode::Unidentified);
    const CTRL: Mods = Mods {
        ctrl: true,
        shift: false,
        alt: false,
    };
    const SHIFT: Mods = Mods {
        ctrl: false,
        shift: true,
        alt: false,
    };
    const CTRL_SHIFT: Mods = Mods {
        ctrl: true,
        shift: true,
        alt: false,
    };

    fn key(c: &str) -> Typed {
        Typed {
            key: Key::Character(c.into()),
            code: NO_CODE,
            text: Some(c.to_owned()),
            mods: Mods::default(),
        }
    }

    fn with(key: Key, text: Option<&str>, mods: Mods) -> Typed {
        Typed {
            key,
            code: NO_CODE,
            text: text.map(str::to_owned),
            mods,
        }
    }

    fn apply_key(chat: &mut McChat, key: &Key, text: Option<&str>, ctrl: bool) {
        let mods = Mods {
            ctrl,
            ..Mods::default()
        };
        apply_typed(chat, &with(key.clone(), text, mods));
    }

    fn typed(chat: &mut McChat, text: &str) {
        for c in text.chars() {
            apply_typed(chat, &key(&c.to_string()));
        }
    }

    fn event(code: KeyCode, key: Key, text: Option<&str>, pressed: bool) -> KeyboardInput {
        KeyboardInput {
            key_code: code,
            logical_key: key,
            state: if pressed {
                bevy::input::ButtonState::Pressed
            } else {
                bevy::input::ButtonState::Released
            },
            text: text.map(Into::into),
            repeat: false,
            window: Entity::PLACEHOLDER,
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
        handle_frame(&mut chat, opener(true, false, true, &t), &t, 2.0);
        assert!(chat.open);
        assert_eq!(chat.input, "");
        assert_eq!(chat.opened_at, 2.0);
        let mut chat = McChat::default();
        let slash = [key("/")];
        handle_frame(&mut chat, opener(false, true, true, &slash), &slash, 0.0);
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
            0.0,
        );
        assert_eq!(chat.input, "/");
        assert_eq!(opener(false, false, true, &[key("t")]), None);
        let mut chat = McChat::default();
        handle_frame(&mut chat, None, &[key("t")], 0.0);
        assert!(!chat.open && chat.input.is_empty());
        handle_frame(&mut chat, Some(Opener::Chat), &[], 0.0);
        handle_frame(
            &mut chat,
            opener(true, false, true, &[key("t")]),
            &[key("t")],
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
        let k = Key::Character("a".into());
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

    #[test]
    fn modifiers_apply_to_the_keys_pressed_while_they_were_down() {
        let ctrl = |pressed| event(KeyCode::ControlLeft, Key::Control, None, pressed);
        let v = event(KeyCode::KeyV, Key::Character("v".into()), Some("v"), true);
        let typed = fold_typed([false; 6], &[ctrl(true), v.clone(), ctrl(false)]);
        assert_eq!(typed.len(), 1);
        assert_eq!(typed[0].mods, CTRL);
        assert_eq!(typed[0].code, KeyCode::KeyV);
        let typed = fold_typed([false; 6], &[v.clone(), ctrl(true)]);
        assert_eq!(typed[0].mods, Mods::default());
        let mut held = [false; 6];
        held[1] = true;
        let typed = fold_typed(held, &[v.clone(), ctrl(false)]);
        assert_eq!(typed[0].mods, CTRL);
        let alt_gr = event(KeyCode::AltRight, Key::AltGraph, None, true);
        let at = event(KeyCode::KeyQ, Key::Character("@".into()), Some("@"), true);
        let typed = fold_typed([false; 6], &[alt_gr, at]);
        assert!(typed[0].mods.alt && !typed[0].mods.ctrl);
        let released = event(KeyCode::KeyV, Key::Character("v".into()), None, false);
        assert!(fold_typed([false; 6], &[released]).is_empty());
    }

    #[test]
    fn a_modifier_is_held_before_the_frame_unless_this_frame_changed_it() {
        assert!(!held_before(false, false, false));
        assert!(held_before(true, false, false));
        assert!(!held_before(true, true, false));
        assert!(held_before(false, false, true));
        assert!(!held_before(false, true, true));
        assert!(held_before(true, true, true));
    }

    #[test]
    fn ctrl_a_selects_everything_and_typing_replaces_it() {
        let mut chat = open_chat();
        typed(&mut chat, "hello big world");
        apply_typed(
            &mut chat,
            &with(Key::Character("a".into()), Some("a"), CTRL),
        );
        assert_eq!((chat.cursor, chat.highlight), (15, 0));
        typed(&mut chat, "x");
        assert_eq!(chat.input, "x");
        typed(&mut chat, "yz");
        apply_typed(
            &mut chat,
            &with(Key::Character("A".into()), Some("A"), CTRL),
        );
        assert_eq!((chat.cursor, chat.highlight), (3, 0));
        apply_typed(&mut chat, &with(Key::End, None, Mods::default()));
        let shifted = with(Key::Character("A".into()), Some("A"), CTRL_SHIFT);
        apply_typed(&mut chat, &shifted);
        assert_eq!((chat.input.as_str(), chat.highlight), ("xyz", 3));
        let mut cyrillic = with(Key::Character("ф".into()), Some("ф"), CTRL);
        cyrillic.code = KeyCode::KeyA;
        apply_typed(&mut chat, &cyrillic);
        assert_eq!((chat.input.as_str(), chat.highlight), ("xyz", 0));
    }

    #[test]
    fn ctrl_types_nothing_but_alt_gr_still_types() {
        let mut chat = open_chat();
        apply_typed(
            &mut chat,
            &with(Key::Character("b".into()), Some("b"), CTRL),
        );
        assert_eq!(chat.input, "");
        let ctrl_alt = Mods {
            ctrl: true,
            shift: false,
            alt: true,
        };
        let at = Key::Character("@".into());
        apply_typed(&mut chat, &with(at.clone(), Some("@"), ctrl_alt));
        let alt_gr = Mods {
            alt: true,
            ..Mods::default()
        };
        apply_typed(&mut chat, &with(at, Some("@"), alt_gr));
        assert_eq!(chat.input, "@@");
    }

    #[test]
    fn shift_selects_and_a_plain_arrow_steps_from_the_cursor() {
        let mut chat = open_chat();
        typed(&mut chat, "hello big world");
        apply_typed(&mut chat, &with(Key::ArrowLeft, None, CTRL_SHIFT));
        assert_eq!(&chat.input[chat.cursor..chat.highlight], "world");
        apply_typed(&mut chat, &with(Key::Home, None, SHIFT));
        assert_eq!((chat.cursor, chat.highlight), (0, 15));
        apply_typed(&mut chat, &with(Key::ArrowRight, None, SHIFT));
        assert_eq!((chat.cursor, chat.highlight), (1, 15));
        apply_typed(&mut chat, &with(Key::ArrowLeft, None, Mods::default()));
        assert_eq!((chat.cursor, chat.highlight), (0, 0));
        apply_typed(&mut chat, &with(Key::End, None, SHIFT));
        apply_key(&mut chat, &Key::Backspace, None, true);
        assert_eq!((chat.input.as_str(), chat.cursor), ("", 0));
    }

    #[test]
    fn opening_and_walking_the_history_drop_the_selection() {
        let mut chat = McChat {
            history: vec!["one".into()],
            highlight: 7,
            ..McChat::default()
        };
        open(&mut chat, Opener::Command, 0.0);
        assert_eq!((chat.cursor, chat.highlight), (1, 1));
        typed(&mut chat, "abc");
        apply_typed(&mut chat, &with(Key::Home, None, SHIFT));
        apply_key(&mut chat, &Key::ArrowUp, None, false);
        assert_eq!((chat.input.as_str(), chat.highlight), ("one", 3));
        apply_typed(&mut chat, &with(Key::Home, None, SHIFT));
        apply_key(&mut chat, &Key::ArrowDown, None, false);
        assert_eq!((chat.input.as_str(), chat.highlight), ("/abc", 4));
    }

    #[test]
    fn the_selection_box_spans_the_selected_text_like_vanilla() {
        let font = font();
        let width = |text: &str| font.width(text);
        assert_eq!(selection_box("hello", [2, 2], true, 316, width), None);
        assert_eq!(
            selection_box("hello", [5, 0], false, 316, width),
            Some(([35, 3], 0..5))
        );
        assert_eq!(
            selection_box("hello", [0, 5], true, 316, width),
            Some(([4, 33], 0..5))
        );
        assert_eq!(
            selection_box("hello", [5, 0], false, 20, width),
            Some(([24, 3], 0..5))
        );
    }
}
