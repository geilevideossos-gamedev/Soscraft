use std::ops::Range;

use bevy::input::keyboard::Key;
use frame::{McBookEdit, McClick, McFont, McScreen, MinecraftUi};
use minecraft_commands::protocol::allowed_in_chat;

use crate::minecraft_book::{BOOK_SIZE, BUTTON_SOUND, LINE_HEIGHT, PAGE_TURN_SOUND, TEXT_WIDTH};
use crate::minecraft_chat::{ChatClipboard, Mods, Typed, shortcut};
use crate::minecraft_edit::{Edit, next_char, previous_char};
use crate::minecraft_gui::{Painter, WHITE};

pub(crate) const PAGE_LIMIT: usize = 1024;
pub(crate) const LINE_LIMIT: usize = (126 / LINE_HEIGHT) as usize;
pub(crate) const PAGE_COUNT_LIMIT: usize = 100;
pub(crate) const TITLE_LIMIT: usize = 15;
const BLINK_MILLIS: f64 = 300.0;
const INK: u32 = 0xFF00_0000;
const DARK_GRAY: u32 = 0xFF55_5555;
const DISABLED_TEXT: u32 = 0xFFA0_A0A0;
const SELECTION: u32 = 0xFF0A_14FF;
const PAGE_BUTTON: [i32; 2] = [23, 13];
const BUTTON: [i32; 2] = [98, 20];
const AREA: [i32; 2] = [122, 134];
const PADDING: i32 = 4;
const TITLE_BOX: [i32; 2] = [114, 20];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    Text,
    Back,
    Forward,
    Sign,
    Done,
    Finalize,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EditLayout {
    pub left: i32,
    pub top: i32,
    pub area: [i32; 4],
    pub text: [i32; 2],
    pub indicator: [i32; 2],
    pub back: [i32; 4],
    pub forward: [i32; 4],
    pub sign: [i32; 4],
    pub done: [i32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SignLayout {
    pub left: i32,
    pub title: [i32; 4],
    pub warning: [i32; 2],
    pub finalize: [i32; 4],
    pub cancel: [i32; 4],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Press {
    pub mouse: Option<[f64; 2]>,
    pub click: bool,
    pub shift: bool,
    pub page_up: bool,
    pub page_down: bool,
}

pub(crate) fn edit_layout(width: i32) -> EditLayout {
    let left = (width - BOOK_SIZE) / 2;
    let top = 2;
    let controls = top + BOOK_SIZE + 2;
    let middle = width / 2;
    let [pw, ph] = PAGE_BUTTON;
    let [bw, bh] = BUTTON;
    let area = [(width - TEXT_WIDTH) / 2 - 8, 28, AREA[0], AREA[1]];
    EditLayout {
        left,
        top,
        area,
        text: [area[0] + PADDING, area[1] + PADDING],
        indicator: [left + 148, top + 16],
        back: [left + 43, top + 157, pw, ph],
        forward: [left + 116, top + 157, pw, ph],
        sign: [middle - bw - 2, controls, bw, bh],
        done: [middle + 2, controls, bw, bh],
    }
}

pub(crate) fn sign_layout(width: i32) -> SignLayout {
    let left = (width - BOOK_SIZE) / 2;
    let middle = width / 2;
    let [bw, bh] = BUTTON;
    let [tw, th] = TITLE_BOX;
    SignLayout {
        left,
        title: [(width - TEXT_WIDTH) / 2 - 3, 50, tw, th],
        warning: [left + 36, 82],
        finalize: [middle - 100, 196, bw, bh],
        cancel: [middle + 2, 196, bw, bh],
    }
}

pub(crate) fn captures_typing(ui: &MinecraftUi) -> bool {
    ui.inventory_open && matches!(ui.screen, McScreen::BookEdit | McScreen::BookSign)
}

pub(crate) fn display_lines(font: &McFont, text: &str) -> Vec<Range<usize>> {
    let mut lines = font.split_ranges(text, TEXT_WIDTH);
    if text.ends_with('\n') {
        lines.push(text.len()..text.len());
    }
    lines
}

fn overflows(font: &McFont, text: &str) -> bool {
    display_lines(font, text).len() > LINE_LIMIT
}

fn line_at(lines: &[Range<usize>], at: usize) -> usize {
    lines
        .iter()
        .position(|line| line.start <= at && at <= line.end)
        .unwrap_or(lines.len().saturating_sub(1))
}

fn first_shown(lines: &[Range<usize>], cursor: usize) -> usize {
    line_at(lines, cursor).saturating_sub(LINE_LIMIT - 1)
}

fn boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn inside([x, y, w, h]: [i32; 4], [mx, my]: [f64; 2]) -> bool {
    mx >= f64::from(x) && mx < f64::from(x + w) && my >= f64::from(y) && my < f64::from(y + h)
}

fn is_space(text: &str, at: usize) -> Option<bool> {
    text[at..].chars().next().map(char::is_whitespace)
}

fn previous_word(text: &str, at: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let mut i = at.min(previous_char(text, text.len()));
    while i > 0 && is_space(text, previous_char(text, i)) == Some(true) {
        i = previous_char(text, i);
    }
    while i > 0 && is_space(text, previous_char(text, i)) == Some(false) {
        i = previous_char(text, i);
    }
    i
}

fn next_word(text: &str, at: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let mut i = at.min(previous_char(text, text.len()));
    while is_space(text, i) == Some(false) {
        i = next_char(text, i);
    }
    while is_space(text, i) == Some(true) {
        i = next_char(text, i);
    }
    i
}

fn settle(edit: &mut McBookEdit) {
    if edit.pages.is_empty() {
        edit.pages.push(String::new());
    }
    edit.page = edit.page.min(edit.pages.len() - 1);
    let text = &edit.pages[edit.page];
    edit.cursor = boundary(text, edit.cursor);
    edit.highlight = boundary(text, edit.highlight);
}

fn page_edit(edit: &mut McBookEdit) -> Edit<'_> {
    settle(edit);
    Edit::new(
        &mut edit.pages[edit.page],
        &mut edit.cursor,
        &mut edit.highlight,
        PAGE_LIMIT,
    )
    .multiline()
}

fn title_edit(edit: &mut McBookEdit) -> Edit<'_> {
    Edit::new(
        &mut edit.title,
        &mut edit.title_cursor,
        &mut edit.title_highlight,
        TITLE_LIMIT,
    )
}

fn insert(edit: &mut McBookEdit, font: &McFont, text: &str) {
    settle(edit);
    if text.is_empty() && edit.cursor == edit.highlight {
        return;
    }
    let page = &edit.pages[edit.page];
    let room = PAGE_LIMIT.saturating_sub(page.chars().count());
    let added: String = text
        .chars()
        .filter(|&c| allowed_in_chat(c) || c == '\n')
        .take(room)
        .collect();
    let mut value = page.clone();
    let (mut cursor, mut highlight) = (edit.cursor, edit.highlight);
    Edit::new(&mut value, &mut cursor, &mut highlight, PAGE_LIMIT)
        .multiline()
        .insert(&added);
    if overflows(font, &value) {
        return;
    }
    edit.pages[edit.page] = value;
    edit.cursor = cursor;
    edit.highlight = highlight;
}

fn delete(edit: &mut McBookEdit, font: &McFont, dir: i32, words: bool) {
    settle(edit);
    if edit.cursor == edit.highlight {
        let text = &edit.pages[edit.page];
        let at = edit.cursor;
        edit.highlight = match (dir < 0, words) {
            (true, true) => previous_word(text, at),
            (false, true) => next_word(text, at),
            (true, false) => previous_char(text, at),
            (false, false) => next_char(text, at),
        };
    }
    insert(edit, font, "");
}

fn seek_line(edit: &mut McBookEdit, font: &McFont, dir: i64, extend: bool) {
    settle(edit);
    let text = &edit.pages[edit.page];
    let lines = display_lines(font, text);
    let line = line_at(&lines, edit.cursor);
    let x = font.width(&text[lines[line].start..edit.cursor]) + 2;
    let target = lines[(line as i64 + dir).clamp(0, lines.len() as i64 - 1) as usize].clone();
    let to = target.start + font.head_by_width(&text[target], x).len();
    page_edit(edit).move_to(to, extend);
}

fn seek_point(edit: &mut McBookEdit, font: &McFont, [x, y]: [f64; 2], extend: bool) {
    settle(edit);
    let text = &edit.pages[edit.page];
    let lines = display_lines(font, text);
    let first = first_shown(&lines, edit.cursor) as i64;
    let row = (y / f64::from(LINE_HEIGHT)).floor() as i64 + first;
    let line = lines[row.clamp(0, lines.len() as i64 - 1) as usize].clone();
    let to = line.start + font.head_by_width(&text[line], x.floor() as i32).len();
    page_edit(edit).move_to(to, extend);
}

fn line_edge(edit: &mut McBookEdit, font: &McFont, end: bool, whole: bool, extend: bool) {
    settle(edit);
    let text = &edit.pages[edit.page];
    let to = if whole {
        if end { text.len() } else { 0 }
    } else {
        let lines = display_lines(font, text);
        let line = &lines[line_at(&lines, edit.cursor)];
        if end { line.end } else { line.start }
    };
    page_edit(edit).move_to(to, extend);
}

fn show_page(edit: &mut McBookEdit, page: usize) {
    edit.page = page;
    settle(edit);
    let text = &mut edit.pages[edit.page];
    if let Some((cut, _)) = text.char_indices().nth(PAGE_LIMIT) {
        text.truncate(cut);
    }
    edit.cursor = text.len();
    edit.highlight = edit.cursor;
}

pub(crate) fn page_back(edit: &mut McBookEdit) {
    if edit.page > 0 {
        show_page(edit, edit.page - 1);
    }
}

pub(crate) fn page_forward(edit: &mut McBookEdit) {
    settle(edit);
    if edit.page + 1 >= edit.pages.len() && edit.pages.len() < PAGE_COUNT_LIMIT {
        edit.pages.push(String::new());
    }
    if edit.page + 1 < edit.pages.len() {
        show_page(edit, edit.page + 1);
    }
}

pub(crate) fn trimmed_pages(pages: &[String]) -> Vec<String> {
    let kept = pages
        .iter()
        .rposition(|page| !page.is_empty())
        .map_or(0, |last| last + 1);
    pages[..kept].to_vec()
}

fn finish(ui: &mut MinecraftUi, title: Option<String>) {
    if let Some(edit) = ui.book_edit.as_ref() {
        let pages = match title {
            Some(_) => edit.pages.clone(),
            None => trimmed_pages(&edit.pages),
        };
        ui.clicks.push(McClick::EditBook { pages, title });
    }
    ui.clicks.push(McClick::Close);
    ui.inventory_open = false;
}

fn can_sign(edit: &McBookEdit) -> bool {
    !edit.title.trim().is_empty()
}

pub(crate) fn control_at(
    screen: McScreen,
    edit: &McBookEdit,
    width: i32,
    mouse: [f64; 2],
) -> Option<Control> {
    if screen == McScreen::BookSign {
        let l = sign_layout(width);
        return [(Control::Finalize, l.finalize), (Control::Cancel, l.cancel)]
            .into_iter()
            .find_map(|(control, rect)| inside(rect, mouse).then_some(control));
    }
    let l = edit_layout(width);
    [
        (Control::Text, Some(l.area)),
        (Control::Back, (edit.page > 0).then_some(l.back)),
        (Control::Forward, Some(l.forward)),
        (Control::Sign, Some(l.sign)),
        (Control::Done, Some(l.done)),
    ]
    .into_iter()
    .find_map(|(control, rect)| rect.filter(|&rect| inside(rect, mouse)).map(|_| control))
}

fn copy(clipboard: &mut dyn ChatClipboard, text: &str) {
    if !text.is_empty() {
        clipboard.set(text);
    }
}

pub(crate) fn input(
    ui: &mut MinecraftUi,
    font: &McFont,
    width: i32,
    press: Press,
    typed: &[Typed],
    clipboard: &mut dyn ChatClipboard,
) {
    match ui.screen {
        McScreen::BookEdit => edit_input(ui, font, width, press, typed, clipboard),
        McScreen::BookSign => sign_input(ui, width, press, typed, clipboard),
        _ => {}
    }
}

fn edit_input(
    ui: &mut MinecraftUi,
    font: &McFont,
    width: i32,
    press: Press,
    typed: &[Typed],
    clipboard: &mut dyn ChatClipboard,
) {
    let Some(edit) = ui.book_edit.as_mut() else {
        return;
    };
    let clicked = press
        .mouse
        .filter(|_| press.click)
        .and_then(|mouse| control_at(McScreen::BookEdit, edit, width, mouse).map(|c| (c, mouse)));
    match clicked {
        Some((Control::Text, [mx, my])) => {
            let [x, y, ..] = edit_layout(width).area;
            let at = [mx - f64::from(x + PADDING), my - f64::from(y + PADDING)];
            seek_point(edit, font, at, press.shift);
        }
        Some((Control::Back, _)) => {
            page_back(edit);
            ui.ui_sounds.push(PAGE_TURN_SOUND);
        }
        Some((Control::Forward, _)) => {
            page_forward(edit);
            ui.ui_sounds.push(PAGE_TURN_SOUND);
        }
        Some((Control::Sign, _)) => {
            edit.title_cursor = edit.title.len();
            edit.title_highlight = edit.title_cursor;
            ui.screen = McScreen::BookSign;
            ui.ui_sounds.push(BUTTON_SOUND);
            return;
        }
        Some((Control::Done, _)) => {
            ui.ui_sounds.push(BUTTON_SOUND);
            finish(ui, None);
            return;
        }
        _ => {}
    }
    if press.page_up {
        page_back(edit);
    }
    if press.page_down {
        page_forward(edit);
    }
    for typed in typed {
        let Mods { ctrl, shift, alt } = typed.mods;
        match typed.key {
            Key::Escape | Key::PageUp | Key::PageDown | Key::Tab => {}
            Key::Enter => insert(edit, font, "\n"),
            Key::Backspace => delete(edit, font, -1, ctrl),
            Key::Delete => delete(edit, font, 1, ctrl),
            Key::ArrowLeft | Key::ArrowRight => {
                let dir = if typed.key == Key::ArrowLeft { -1 } else { 1 };
                if ctrl {
                    settle(edit);
                    let text = &edit.pages[edit.page];
                    let to = if dir < 0 {
                        previous_word(text, edit.cursor)
                    } else {
                        next_word(text, edit.cursor)
                    };
                    page_edit(edit).move_to(to, shift);
                } else {
                    page_edit(edit).step(dir, false, shift);
                }
            }
            Key::ArrowUp if !ctrl => seek_line(edit, font, -1, shift),
            Key::ArrowDown if !ctrl => seek_line(edit, font, 1, shift),
            Key::Home => line_edge(edit, font, false, ctrl, shift),
            Key::End => line_edge(edit, font, true, ctrl, shift),
            _ => match shortcut(typed) {
                Some('a') => page_edit(edit).select_all(),
                Some('c') => copy(clipboard, page_edit(edit).highlighted()),
                Some('v') => {
                    let text = clipboard.get();
                    insert(edit, font, &text);
                }
                Some('x') => {
                    copy(clipboard, page_edit(edit).highlighted());
                    insert(edit, font, "");
                }
                _ => {
                    if let Some(text) = typed.text.as_deref().filter(|_| !ctrl || alt) {
                        insert(edit, font, text);
                    }
                }
            },
        }
    }
}

fn sign_input(
    ui: &mut MinecraftUi,
    width: i32,
    press: Press,
    typed: &[Typed],
    clipboard: &mut dyn ChatClipboard,
) {
    let Some(edit) = ui.book_edit.as_mut() else {
        return;
    };
    let clicked = press
        .mouse
        .filter(|_| press.click)
        .and_then(|mouse| control_at(McScreen::BookSign, edit, width, mouse));
    match clicked {
        Some(Control::Finalize) if can_sign(edit) => {
            let title = edit.title.trim().to_owned();
            ui.ui_sounds.push(BUTTON_SOUND);
            finish(ui, Some(title));
            return;
        }
        Some(Control::Cancel) => {
            show_page(edit, edit.page);
            ui.screen = McScreen::BookEdit;
            ui.ui_sounds.push(BUTTON_SOUND);
            return;
        }
        _ => {}
    }
    for typed in typed {
        let Mods { ctrl, shift, alt } = typed.mods;
        match typed.key {
            Key::Enter => {
                if !edit.title.is_empty() {
                    let title = edit.title.trim().to_owned();
                    finish(ui, Some(title));
                    return;
                }
            }
            Key::Escape
            | Key::PageUp
            | Key::PageDown
            | Key::Tab
            | Key::ArrowUp
            | Key::ArrowDown => {}
            Key::Backspace => title_edit(edit).delete(-1, ctrl),
            Key::Delete => title_edit(edit).delete(1, ctrl),
            Key::ArrowLeft => title_edit(edit).step(-1, ctrl, shift),
            Key::ArrowRight => title_edit(edit).step(1, ctrl, shift),
            Key::Home => title_edit(edit).move_to(0, shift),
            Key::End => title_edit(edit).move_to(usize::MAX, shift),
            _ => match shortcut(typed) {
                Some('a') => title_edit(edit).select_all(),
                Some('c') => copy(clipboard, title_edit(edit).highlighted()),
                Some('v') => {
                    let text = clipboard.get();
                    title_edit(edit).insert(&text);
                }
                Some('x') => {
                    copy(clipboard, title_edit(edit).highlighted());
                    title_edit(edit).insert("");
                }
                _ => {
                    if let Some(text) = typed.text.as_deref().filter(|_| !ctrl || alt) {
                        title_edit(edit).insert(text);
                    }
                }
            },
        }
    }
}

fn cursor_shown(edit: &McBookEdit, now: f64) -> bool {
    ((now - edit.since).max(0.0) * 1000.0 / BLINK_MILLIS) as i64 % 2 == 0
}

fn button(p: &mut Painter<'_>, rect: [i32; 4], label: &str, hovered: bool, active: bool) {
    let [x, y, w, h] = rect;
    let (sprite, color) = match (active, hovered) {
        (false, _) => ("widget/button_disabled", DISABLED_TEXT),
        (true, true) => ("widget/button_highlighted", WHITE),
        (true, false) => ("widget/button", WHITE),
    };
    p.sprite(sprite, x, y, w, h);
    let width = p.width(label);
    p.text(
        label,
        x + w / 2 - width / 2,
        (2 * y + h - 9) / 2 + 1,
        color,
        true,
    );
}

pub(crate) fn draw(
    p: &mut Painter<'_>,
    screen: McScreen,
    edit: &McBookEdit,
    mouse: Option<[f64; 2]>,
    now: f64,
) {
    match screen {
        McScreen::BookSign => draw_sign(p, edit, mouse, now),
        _ => draw_edit(p, edit, mouse, now),
    }
}

fn draw_edit(p: &mut Painter<'_>, edit: &McBookEdit, mouse: Option<[f64; 2]>, now: f64) {
    let l = edit_layout(p.space.width);
    let gui = p.gui;
    let font = &gui.font;
    p.blit("book", l.left, l.top, 0, 0, BOOK_SIZE, BOOK_SIZE);
    let indicator = gui.format(
        "book.pageIndicator",
        &[
            &(edit.page + 1).to_string(),
            &edit.pages.len().max(1).to_string(),
        ],
    );
    let [ix, iy] = l.indicator;
    p.text(&indicator, ix - p.width(&indicator), iy, INK, false);
    let text = edit.pages.get(edit.page).map_or("", String::as_str);
    let cursor = boundary(text, edit.cursor);
    let highlight = boundary(text, edit.highlight);
    let lines = display_lines(font, text);
    let first = first_shown(&lines, cursor);
    let shown = &lines[first..(first + LINE_LIMIT).min(lines.len())];
    let [tx, ty] = l.text;
    let row = |i: usize| ty + i as i32 * LINE_HEIGHT;
    for (i, line) in shown.iter().enumerate() {
        p.text(&text[line.clone()], tx, row(i), INK, false);
    }
    if cursor_shown(edit, now) {
        if cursor < text.len() {
            if let Some(i) = shown
                .iter()
                .position(|line| line.start <= cursor && cursor <= line.end)
            {
                let x = tx + font.width(&text[shown[i].start..cursor]);
                p.fill(x, row(i) - 1, x + 1, row(i) + LINE_HEIGHT + 1, INK);
            }
        } else if let Some(i) = shown.len().checked_sub(1) {
            let x = tx + font.width(&text[shown[i].clone()]);
            p.text("_", x, row(i), INK, false);
        }
    }
    let (from, to) = (cursor.min(highlight), cursor.max(highlight));
    if from != to {
        for (i, line) in shown.iter().enumerate() {
            if from > line.end || line.start > to {
                continue;
            }
            let (start, end) = (from.max(line.start), to.min(line.end));
            let x0 = tx + font.width(&text[line.start..start]);
            let x1 = if to > line.end {
                tx + AREA[0] - PADDING
            } else {
                tx + font.width(&text[line.start..end])
            };
            p.fill(x0, row(i), x1, row(i) + LINE_HEIGHT, SELECTION);
            p.text(&text[start..end], x0, row(i), WHITE, false);
        }
    }
    let hovered =
        mouse.and_then(|mouse| control_at(McScreen::BookEdit, edit, p.space.width, mouse));
    for (shown, control, rect, name) in [
        (edit.page > 0, Control::Back, l.back, "widget/page_backward"),
        (true, Control::Forward, l.forward, "widget/page_forward"),
    ] {
        if shown {
            let [x, y, w, h] = rect;
            let sprite = if hovered == Some(control) {
                format!("{name}_highlighted")
            } else {
                name.to_owned()
            };
            p.sprite(&sprite, x, y, w, h);
        }
    }
    let sign = gui.text("book.signButton");
    button(p, l.sign, &sign, hovered == Some(Control::Sign), true);
    let done = gui.text("gui.done");
    button(p, l.done, &done, hovered == Some(Control::Done), true);
}

fn draw_sign(p: &mut Painter<'_>, edit: &McBookEdit, mouse: Option<[f64; 2]>, now: f64) {
    let l = sign_layout(p.space.width);
    let gui = p.gui;
    let font = &gui.font;
    p.blit("book", l.left, 2, 0, 0, BOOK_SIZE, BOOK_SIZE);
    let centred = |text: &str| l.left + 36 + (TEXT_WIDTH - font.width(text)) / 2;
    let label = gui.text("book.editTitle");
    p.text(&label, centred(&label), 34, INK, false);
    let [bx, by, bw, _] = l.title;
    let title = edit.title.as_str();
    let cursor = boundary(title, edit.title_cursor);
    let highlight = boundary(title, edit.title_highlight);
    let text_x = bx + (bw - font.width(title)) / 2;
    p.text(title, text_x, by, INK, false);
    let insert = cursor < title.len() || title.chars().count() >= TITLE_LIMIT;
    let mut cursor_x = text_x;
    if !title.is_empty() {
        cursor_x += font.width(&title[..cursor]) + 1 - i32::from(insert);
    }
    if cursor != highlight {
        let (from, to) = (cursor.min(highlight), cursor.max(highlight));
        let highlight_x = text_x + font.width(&title[..highlight]) - 1;
        p.fill(
            cursor_x,
            by - 1,
            highlight_x,
            by + LINE_HEIGHT + 1,
            SELECTION,
        );
        p.text(
            &title[from..to],
            text_x + font.width(&title[..from]),
            by,
            WHITE,
            false,
        );
    }
    if cursor_shown(edit, now) {
        if insert {
            p.fill(cursor_x, by - 1, cursor_x + 1, by + LINE_HEIGHT + 1, INK);
        } else {
            p.text("_", cursor_x, by, INK, false);
        }
    }
    let author = gui.format("book.byAuthor", &[&edit.author]);
    p.text(&author, centred(&author), 60, DARK_GRAY, false);
    let [wx, wy] = l.warning;
    for (i, line) in font
        .split(&gui.text("book.finalizeWarning"), TEXT_WIDTH)
        .iter()
        .enumerate()
    {
        p.text(line, wx, wy + i as i32 * LINE_HEIGHT, INK, false);
    }
    let hovered =
        mouse.and_then(|mouse| control_at(McScreen::BookSign, edit, p.space.width, mouse));
    let finalize = gui.text("book.finalizeButton");
    button(
        p,
        l.finalize,
        &finalize,
        hovered == Some(Control::Finalize),
        can_sign(edit),
    );
    let cancel = gui.text("gui.cancel");
    button(p, l.cancel, &cancel, hovered == Some(Control::Cancel), true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::keyboard::{KeyCode, NativeKeyCode};

    const NO_CODE: KeyCode = KeyCode::Unidentified(NativeKeyCode::Unidentified);
    const WIDTH: i32 = 427;

    fn font() -> McFont {
        let glyph = |advance: f32| frame::McGlyph {
            rect: [0, 0, 8, 8],
            scale: 1.0,
            ascent: 7,
            advance,
        };
        let mut font = McFont::default();
        for c in 'a'..='z' {
            font.glyphs.insert(c, glyph(6.0));
        }
        for c in '0'..='9' {
            font.glyphs.insert(c, glyph(6.0));
        }
        font.glyphs.insert('.', glyph(1.0));
        font.spaces.insert(' ', 4.0);
        font
    }

    fn open(pages: &[&str]) -> MinecraftUi {
        let pages: Vec<String> = pages.iter().map(|page| (*page).to_owned()).collect();
        let cursor = pages.first().map_or(0, String::len);
        MinecraftUi {
            inventory_open: true,
            screen: McScreen::BookEdit,
            book_edit: Some(McBookEdit {
                pages,
                cursor,
                highlight: cursor,
                author: "Owner".to_owned(),
                ..McBookEdit::default()
            }),
            ..MinecraftUi::default()
        }
    }

    fn key(key: Key, mods: Mods) -> Typed {
        Typed {
            key,
            code: NO_CODE,
            text: None,
            mods,
        }
    }

    fn letter(text: &str) -> Typed {
        Typed {
            key: Key::Character(text.into()),
            code: NO_CODE,
            text: Some(text.to_owned()),
            mods: Mods::default(),
        }
    }

    fn shift() -> Mods {
        Mods {
            shift: true,
            ..Mods::default()
        }
    }

    fn ctrl() -> Mods {
        Mods {
            ctrl: true,
            ..Mods::default()
        }
    }

    fn keys(ui: &mut MinecraftUi, typed: &[Typed]) {
        input(ui, &font(), WIDTH, Press::default(), typed, &mut ());
    }

    fn type_text(ui: &mut MinecraftUi, text: &str) {
        let typed: Vec<Typed> = text.chars().map(|c| letter(&c.to_string())).collect();
        keys(ui, &typed);
    }

    fn click(ui: &mut MinecraftUi, rect: [i32; 4]) {
        let press = Press {
            mouse: Some([
                f64::from(rect[0]) + f64::from(rect[2]) / 2.0,
                f64::from(rect[1]) + f64::from(rect[3]) / 2.0,
            ]),
            click: true,
            ..Press::default()
        };
        input(ui, &font(), WIDTH, press, &[], &mut ());
    }

    fn page(ui: &MinecraftUi) -> &str {
        let edit = ui.book_edit.as_ref().unwrap();
        &edit.pages[edit.page]
    }

    fn cursor(ui: &MinecraftUi) -> (usize, usize) {
        let edit = ui.book_edit.as_ref().unwrap();
        (edit.cursor, edit.highlight)
    }

    #[test]
    fn edit_layout_matches_vanilla() {
        let l = edit_layout(WIDTH);
        assert_eq!((l.left, l.top), (117, 2));
        assert_eq!(l.area, [148, 28, 122, 134]);
        assert_eq!(l.text, [152, 32]);
        assert_eq!(l.indicator, [265, 18]);
        assert_eq!(l.back, [160, 159, 23, 13]);
        assert_eq!(l.forward, [233, 159, 23, 13]);
        assert_eq!(l.sign, [113, 196, 98, 20]);
        assert_eq!(l.done, [215, 196, 98, 20]);
        let s = sign_layout(WIDTH);
        assert_eq!(s.title, [153, 50, 114, 20]);
        assert_eq!(s.warning, [153, 82]);
        assert_eq!(s.finalize, [113, 196, 98, 20]);
        assert_eq!(s.cancel, [215, 196, 98, 20]);
        assert_eq!(LINE_LIMIT, 14);
        let edit = McBookEdit::default();
        assert_eq!(
            control_at(McScreen::BookEdit, &edit, WIDTH, [170.0, 165.0]),
            None
        );
        assert_eq!(
            control_at(McScreen::BookEdit, &edit, WIDTH, [170.0, 160.0]),
            Some(Control::Text)
        );
        assert_eq!(
            control_at(McScreen::BookEdit, &edit, WIDTH, [240.0, 165.0]),
            Some(Control::Forward)
        );
        let second = McBookEdit {
            page: 1,
            ..McBookEdit::default()
        };
        assert_eq!(
            control_at(McScreen::BookEdit, &second, WIDTH, [170.0, 165.0]),
            Some(Control::Back)
        );
    }

    #[test]
    fn typing_stops_at_14_lines_and_1024_chars() {
        let mut ui = open(&[""]);
        let enter = key(Key::Enter, Mods::default());
        for _ in 0..20 {
            keys(&mut ui, std::slice::from_ref(&enter));
        }
        assert_eq!(page(&ui), "\n".repeat(13));
        type_text(&mut ui, &"a".repeat(25));
        assert_eq!(page(&ui), format!("{}{}", "\n".repeat(13), "a".repeat(19)));
        assert_eq!(display_lines(&font(), page(&ui)).len(), 14);
        let mut ui = open(&[""]);
        type_text(&mut ui, "ab");
        let mut board = Board("a".repeat(300));
        input(
            &mut ui,
            &font(),
            WIDTH,
            Press::default(),
            &[Typed {
                mods: ctrl(),
                ..letter("v")
            }],
            &mut board,
        );
        assert_eq!(
            page(&ui),
            "ab",
            "a paste that overflows the lines is refused whole"
        );
        board.0 = ".".repeat(2000);
        input(
            &mut ui,
            &font(),
            WIDTH,
            Press::default(),
            &[Typed {
                mods: ctrl(),
                ..letter("v")
            }],
            &mut board,
        );
        assert_eq!(page(&ui).chars().count(), PAGE_LIMIT);
        assert!(page(&ui).starts_with("ab."));
        type_text(&mut ui, "a");
        assert_eq!(page(&ui).chars().count(), PAGE_LIMIT);
        assert!(!page(&ui).ends_with('a'));
        keys(&mut ui, &[key(Key::Backspace, Mods::default())]);
        assert_eq!(page(&ui).chars().count(), PAGE_LIMIT - 1);
        type_text(&mut ui, "a\u{a7}\t");
        assert!(page(&ui).ends_with('a'));
        assert_eq!(page(&ui).chars().count(), PAGE_LIMIT);
    }

    #[test]
    fn enter_inserts_a_newline_and_up_down_move_by_display_line() {
        let mut ui = open(&[""]);
        type_text(&mut ui, "ab");
        keys(&mut ui, &[key(Key::Enter, Mods::default())]);
        type_text(&mut ui, "cd");
        assert_eq!(page(&ui), "ab\ncd");
        assert_eq!(cursor(&ui), (5, 5));
        keys(&mut ui, &[key(Key::ArrowUp, Mods::default())]);
        assert_eq!(cursor(&ui), (2, 2));
        keys(&mut ui, &[key(Key::ArrowDown, Mods::default())]);
        assert_eq!(cursor(&ui), (5, 5));
        keys(&mut ui, &[key(Key::ArrowUp, shift())]);
        assert_eq!(cursor(&ui), (2, 5));
        keys(&mut ui, &[key(Key::ArrowUp, ctrl())]);
        assert_eq!(cursor(&ui), (2, 5), "ctrl with up does nothing");
        keys(&mut ui, &[key(Key::Enter, Mods::default())]);
        assert_eq!(page(&ui), "ab\n");
        let mut ui = open(&[&"a".repeat(25)]);
        keys(&mut ui, &[key(Key::ArrowUp, Mods::default())]);
        assert_eq!(cursor(&ui), (6, 6));
        keys(&mut ui, &[key(Key::ArrowDown, Mods::default())]);
        assert_eq!(cursor(&ui), (25, 25));
        keys(&mut ui, &[key(Key::Home, Mods::default())]);
        assert_eq!(cursor(&ui), (19, 19));
        keys(&mut ui, &[key(Key::End, Mods::default())]);
        assert_eq!(
            cursor(&ui),
            (19, 19),
            "19 ends the first line, as in vanilla"
        );
        keys(&mut ui, &[key(Key::Home, ctrl())]);
        assert_eq!(cursor(&ui), (0, 0));
        keys(&mut ui, &[key(Key::End, ctrl())]);
        assert_eq!(cursor(&ui), (25, 25));
        let mut ui = open(&["one two\nthree"]);
        keys(&mut ui, &[key(Key::ArrowLeft, ctrl())]);
        assert_eq!(cursor(&ui), (8, 8));
        keys(&mut ui, &[key(Key::ArrowLeft, ctrl())]);
        assert_eq!(cursor(&ui), (4, 4));
        keys(&mut ui, &[key(Key::ArrowRight, ctrl())]);
        assert_eq!(cursor(&ui), (8, 8));
        keys(&mut ui, &[key(Key::Backspace, ctrl())]);
        assert_eq!(page(&ui), "one three");
        let mut keypad = key(Key::Enter, Mods::default());
        keypad.code = KeyCode::NumpadEnter;
        keys(&mut ui, &[keypad]);
        assert_eq!(page(&ui), "one \nthree");
        let mut ui = open(&["ab\ncd"]);
        let area = edit_layout(WIDTH).area;
        let press = Press {
            mouse: Some([
                f64::from(area[0] + PADDING) + 7.0,
                f64::from(area[1] + PADDING) + 1.0,
            ]),
            click: true,
            ..Press::default()
        };
        input(&mut ui, &font(), WIDTH, press, &[], &mut ());
        assert_eq!(cursor(&ui), (1, 1));
    }

    #[test]
    fn forward_on_the_last_page_adds_one_up_to_100() {
        let mut ui = open(&["first"]);
        let forward = edit_layout(WIDTH).forward;
        click(&mut ui, forward);
        let edit = ui.book_edit.as_ref().unwrap();
        assert_eq!((edit.pages.len(), edit.page), (2, 1));
        assert_eq!(ui.ui_sounds, [PAGE_TURN_SOUND]);
        for _ in 0..150 {
            input(
                &mut ui,
                &font(),
                WIDTH,
                Press {
                    page_down: true,
                    ..Press::default()
                },
                &[],
                &mut (),
            );
        }
        let edit = ui.book_edit.as_ref().unwrap();
        assert_eq!((edit.pages.len(), edit.page), (PAGE_COUNT_LIMIT, 99));
        assert_eq!(ui.ui_sounds.len(), 1, "page keys turn without a sound");
        click(&mut ui, edit_layout(WIDTH).back);
        let edit = ui.book_edit.as_ref().unwrap();
        assert_eq!((edit.pages.len(), edit.page), (100, 98));
        let mut ui = open(&["first", "second"]);
        click(&mut ui, forward);
        let edit = ui.book_edit.as_ref().unwrap();
        assert_eq!((edit.pages.len(), edit.page), (2, 1));
        assert_eq!((edit.cursor, edit.highlight), (6, 6));
        input(
            &mut ui,
            &font(),
            WIDTH,
            Press {
                page_up: true,
                ..Press::default()
            },
            &[],
            &mut (),
        );
        assert_eq!(ui.book_edit.as_ref().unwrap().page, 0);
        click(&mut ui, edit_layout(WIDTH).back);
        assert_eq!(ui.book_edit.as_ref().unwrap().page, 0);
    }

    #[test]
    fn done_trims_trailing_empty_pages_and_esc_discards() {
        let mut ui = open(&["one", "", "two", "", ""]);
        keys(&mut ui, &[key(Key::Escape, Mods::default())]);
        assert!(ui.clicks.is_empty());
        assert!(ui.inventory_open);
        click(&mut ui, edit_layout(WIDTH).done);
        assert_eq!(
            ui.clicks,
            [
                McClick::EditBook {
                    pages: vec!["one".to_owned(), String::new(), "two".to_owned()],
                    title: None
                },
                McClick::Close
            ]
        );
        assert!(!ui.inventory_open);
        assert_eq!(ui.ui_sounds, [BUTTON_SOUND]);
        let mut ui = open(&["", ""]);
        click(&mut ui, edit_layout(WIDTH).done);
        assert_eq!(
            ui.clicks[0],
            McClick::EditBook {
                pages: Vec::new(),
                title: None
            }
        );
    }

    #[test]
    fn sign_and_close_waits_for_a_title_and_cancel_keeps_it() {
        let mut ui = open(&["abc", ""]);
        keys(&mut ui, &[key(Key::Home, Mods::default())]);
        click(&mut ui, edit_layout(WIDTH).sign);
        assert_eq!(ui.screen, McScreen::BookSign);
        let s = sign_layout(WIDTH);
        click(&mut ui, s.finalize);
        keys(&mut ui, &[key(Key::Enter, Mods::default())]);
        assert!(ui.clicks.is_empty(), "no title yet");
        type_text(&mut ui, "  ");
        click(&mut ui, s.finalize);
        assert!(ui.clicks.is_empty(), "a blank title cannot be signed");
        keys(&mut ui, &[letter("a")]);
        keys(
            &mut ui,
            &[Typed {
                mods: ctrl(),
                ..letter("a")
            }],
        );
        keys(&mut ui, &[key(Key::Backspace, Mods::default())]);
        type_text(&mut ui, "my diary of twenty");
        assert_eq!(ui.book_edit.as_ref().unwrap().title, "my diary of twe");
        click(&mut ui, s.cancel);
        assert_eq!(ui.screen, McScreen::BookEdit);
        assert!(ui.inventory_open);
        assert_eq!(
            cursor(&ui),
            (3, 3),
            "the page comes back with the cursor at its end"
        );
        click(&mut ui, edit_layout(WIDTH).sign);
        let edit = ui.book_edit.as_ref().unwrap();
        assert_eq!(edit.title, "my diary of twe");
        assert_eq!(edit.title_cursor, edit.title.len());
        for _ in 0..10 {
            keys(&mut ui, &[key(Key::Backspace, Mods::default())]);
        }
        type_text(&mut ui, " ");
        assert_eq!(ui.book_edit.as_ref().unwrap().title, "my di ");
        keys(&mut ui, &[key(Key::Enter, Mods::default())]);
        assert_eq!(
            ui.clicks,
            [
                McClick::EditBook {
                    pages: vec!["abc".to_owned(), String::new()],
                    title: Some("my di".to_owned())
                },
                McClick::Close
            ]
        );
        assert!(!ui.inventory_open);
        let mut ui = open(&["x"]);
        click(&mut ui, edit_layout(WIDTH).sign);
        type_text(&mut ui, "Diary");
        click(&mut ui, sign_layout(WIDTH).finalize);
        assert_eq!(
            ui.clicks[0],
            McClick::EditBook {
                pages: vec!["x".to_owned()],
                title: Some("Diary".to_owned())
            }
        );
        assert_eq!(ui.ui_sounds, [BUTTON_SOUND, BUTTON_SOUND]);
    }

    #[test]
    fn typing_in_a_book_does_not_toggle_the_inventory() {
        let mut ui = open(&[""]);
        assert!(captures_typing(&ui));
        type_text(&mut ui, "eq1");
        assert_eq!(page(&ui), "eq1");
        ui.screen = McScreen::BookSign;
        assert!(captures_typing(&ui));
        type_text(&mut ui, "e2");
        assert_eq!(ui.book_edit.as_ref().unwrap().title, "e2");
        ui.screen = McScreen::Lectern;
        assert!(!captures_typing(&ui));
        ui.screen = McScreen::BookEdit;
        ui.inventory_open = false;
        assert!(!captures_typing(&ui));
    }

    #[test]
    fn copy_cut_and_select_all_use_the_clipboard() {
        let mut ui = open(&["hello world"]);
        let mut board = Board(String::new());
        let with_ctrl = |c: &str| Typed {
            mods: ctrl(),
            ..letter(c)
        };
        keys(
            &mut ui,
            &[key(
                Key::ArrowLeft,
                Mods {
                    ctrl: true,
                    shift: true,
                    alt: false,
                },
            )],
        );
        input(
            &mut ui,
            &font(),
            WIDTH,
            Press::default(),
            &[with_ctrl("x")],
            &mut board,
        );
        assert_eq!((page(&ui), board.0.as_str()), ("hello ", "world"));
        input(
            &mut ui,
            &font(),
            WIDTH,
            Press::default(),
            &[with_ctrl("a")],
            &mut board,
        );
        assert_eq!(cursor(&ui), (6, 0));
        input(
            &mut ui,
            &font(),
            WIDTH,
            Press::default(),
            &[with_ctrl("c")],
            &mut board,
        );
        assert_eq!(board.0, "hello ");
        input(
            &mut ui,
            &font(),
            WIDTH,
            Press::default(),
            &[with_ctrl("v")],
            &mut board,
        );
        assert_eq!(page(&ui), "hello ");
        keys(&mut ui, &[key(Key::Delete, Mods::default())]);
        assert_eq!(page(&ui), "hello ");
        keys(&mut ui, &[key(Key::Home, Mods::default())]);
        keys(&mut ui, &[key(Key::Delete, ctrl())]);
        assert_eq!(page(&ui), "");
    }

    struct Board(String);

    impl ChatClipboard for Board {
        fn get(&mut self) -> String {
            self.0.clone()
        }

        fn set(&mut self, text: &str) {
            self.0 = text.to_owned();
        }
    }
}
