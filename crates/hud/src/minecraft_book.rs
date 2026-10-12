use frame::{McBook, McClick, McFont, MinecraftUi};

use crate::minecraft_gui::{Painter, WHITE};

pub(crate) const BOOK_SIZE: i32 = 192;
pub(crate) const TEXT_WIDTH: i32 = 114;
pub(crate) const LINE_HEIGHT: i32 = 9;
pub(crate) const TEXT_LINES: usize = (128 / LINE_HEIGHT) as usize;
pub(crate) const PAGE_TURN_SOUND: &str = "minecraft:item.book.page_turn";
pub(crate) const BUTTON_SOUND: &str = "minecraft:ui.button.click";
const INK: u32 = 0xFF00_0000;
const PAGE_BUTTON: [i32; 2] = [23, 13];
const BUTTON_HEIGHT: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    Back,
    Forward,
    Done,
    Take,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BookLayout {
    pub left: i32,
    pub top: i32,
    pub text: [i32; 2],
    pub indicator: [i32; 2],
    pub back: [i32; 4],
    pub forward: [i32; 4],
    pub done: [i32; 4],
    pub take: Option<[i32; 4]>,
}

pub(crate) fn layout(width: i32, take: bool) -> BookLayout {
    let left = (width - BOOK_SIZE) / 2;
    let top = 2;
    let controls = top + BOOK_SIZE + 2;
    let middle = width / 2;
    let [pw, ph] = PAGE_BUTTON;
    let (done, take) = if take {
        (
            [middle - 98 - 2, controls, 98, BUTTON_HEIGHT],
            Some([middle + 2, controls, 98, BUTTON_HEIGHT]),
        )
    } else {
        ([(width - 200) / 2, controls, 200, BUTTON_HEIGHT], None)
    };
    BookLayout {
        left,
        top,
        text: [left + 36, top + 30],
        indicator: [left + 148, top + 16],
        back: [left + 43, top + 157, pw, ph],
        forward: [left + 116, top + 157, pw, ph],
        done,
        take,
    }
}

pub(crate) fn page_count(book: &McBook) -> usize {
    book.pages.len()
}

pub(crate) fn visible(book: &McBook) -> (bool, bool) {
    let count = page_count(book) as i64;
    let page = book.page as i64;
    (page > 0, page < count - 1)
}

pub(crate) fn page_lines(font: &McFont, text: &str) -> Vec<String> {
    let mut lines = font.split(text, TEXT_WIDTH);
    lines.truncate(TEXT_LINES);
    lines
}

fn inside([x, y, w, h]: [i32; 4], [mx, my]: [f64; 2]) -> bool {
    mx >= f64::from(x) && mx < f64::from(x + w) && my >= f64::from(y) && my < f64::from(y + h)
}

pub(crate) fn control_at(book: &McBook, width: i32, mouse: [f64; 2]) -> Option<Control> {
    let l = layout(width, book.take);
    let (back, forward) = visible(book);
    [
        (Control::Back, Some(l.back).filter(|_| back)),
        (Control::Forward, Some(l.forward).filter(|_| forward)),
        (Control::Done, Some(l.done)),
        (Control::Take, l.take),
    ]
    .into_iter()
    .find_map(|(control, rect)| rect.filter(|&rect| inside(rect, mouse)).map(|_| control))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Press {
    pub mouse: Option<[f64; 2]>,
    pub click: bool,
    pub page_up: bool,
    pub page_down: bool,
}

pub(crate) fn pressed(book: &McBook, width: i32, press: Press) -> Option<(Control, bool)> {
    if press.click
        && let Some(control) = press.mouse.and_then(|mouse| control_at(book, width, mouse))
    {
        return Some((control, true));
    }
    if press.page_up {
        return Some((Control::Back, false));
    }
    if press.page_down {
        return Some((Control::Forward, false));
    }
    None
}

pub(crate) fn input(ui: &mut MinecraftUi, width: i32, press: Press) {
    let Some(book) = ui.book.as_ref() else {
        return;
    };
    let Some((control, clicked)) = pressed(book, width, press) else {
        return;
    };
    match control {
        Control::Back => ui.clicks.push(McClick::Choose(1)),
        Control::Forward => ui.clicks.push(McClick::Choose(2)),
        Control::Take => ui.clicks.push(McClick::Choose(3)),
        Control::Done => {
            ui.inventory_open = false;
            ui.clicks.push(McClick::Close);
        }
    }
    if clicked {
        ui.ui_sounds.push(match control {
            Control::Back | Control::Forward => PAGE_TURN_SOUND,
            Control::Done | Control::Take => BUTTON_SOUND,
        });
    }
}

fn button(p: &mut Painter<'_>, rect: [i32; 4], label: &str, hovered: bool) {
    let [x, y, w, h] = rect;
    let sprite = if hovered {
        "widget/button_highlighted"
    } else {
        "widget/button"
    };
    p.sprite(sprite, x, y, w, h);
    let width = p.width(label);
    p.text(
        label,
        x + w / 2 - width / 2,
        (2 * y + h - 9) / 2 + 1,
        WHITE,
        true,
    );
}

pub(crate) fn draw(p: &mut Painter<'_>, book: &McBook, mouse: Option<[f64; 2]>) {
    let l = layout(p.space.width, book.take);
    p.blit("book", l.left, l.top, 0, 0, BOOK_SIZE, BOOK_SIZE);
    let indicator = p.gui.format(
        "book.pageIndicator",
        &[
            &(book.page + 1).to_string(),
            &page_count(book).max(1).to_string(),
        ],
    );
    let [ix, iy] = l.indicator;
    p.text(&indicator, ix - p.width(&indicator), iy, INK, false);
    let text = book.pages.get(book.page).map_or("", String::as_str);
    let [tx, ty] = l.text;
    for (i, line) in page_lines(&p.gui.font, text).iter().enumerate() {
        p.text(line, tx, ty + i as i32 * LINE_HEIGHT, INK, false);
    }
    let hovered = mouse.and_then(|mouse| control_at(book, p.space.width, mouse));
    let (back, forward) = visible(book);
    for (shown, control, rect, name) in [
        (back, Control::Back, l.back, "widget/page_backward"),
        (forward, Control::Forward, l.forward, "widget/page_forward"),
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
    let done = p.gui.text("gui.done");
    button(p, l.done, &done, hovered == Some(Control::Done));
    if let Some(rect) = l.take {
        let take = p.gui.text("lectern.take_book");
        button(p, rect, &take, hovered == Some(Control::Take));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(pages: usize, page: usize, take: bool) -> McBook {
        McBook {
            pages: (0..pages).map(|n| format!("page {n}")).collect(),
            page,
            take,
        }
    }

    #[test]
    fn book_layout_matches_vanilla() {
        let l = layout(427, true);
        assert_eq!((l.left, l.top), (117, 2));
        assert_eq!(l.text, [153, 32]);
        assert_eq!(l.indicator, [265, 18]);
        assert_eq!(l.back, [160, 159, 23, 13]);
        assert_eq!(l.forward, [233, 159, 23, 13]);
        assert_eq!(l.done, [113, 196, 98, 20]);
        assert_eq!(l.take, Some([215, 196, 98, 20]));
        let l = layout(427, false);
        assert_eq!(l.done, [113, 196, 200, 20]);
        assert_eq!(l.take, None);
        assert_eq!(TEXT_LINES, 14);
    }

    #[test]
    fn page_buttons_hide_at_the_ends() {
        assert_eq!(visible(&book(3, 0, true)), (false, true));
        assert_eq!(visible(&book(3, 1, true)), (true, true));
        assert_eq!(visible(&book(3, 2, true)), (true, false));
        assert_eq!(visible(&book(1, 0, true)), (false, false));
        assert_eq!(visible(&book(0, 0, true)), (false, false));
        let first = book(3, 0, true);
        assert_eq!(control_at(&first, 427, [161.0, 160.0]), None);
        assert_eq!(
            control_at(&first, 427, [233.0, 159.0]),
            Some(Control::Forward)
        );
        assert_eq!(control_at(&first, 427, [256.0, 159.0]), None);
        let last = book(3, 2, false);
        assert_eq!(control_at(&last, 427, [182.9, 171.9]), Some(Control::Back));
        assert_eq!(control_at(&last, 427, [240.0, 165.0]), None);
        assert_eq!(control_at(&last, 427, [300.0, 200.0]), Some(Control::Done));
        assert_eq!(control_at(&last, 427, [215.0, 200.0]), Some(Control::Done));
        let taking = book(3, 2, true);
        assert_eq!(
            control_at(&taking, 427, [215.0, 200.0]),
            Some(Control::Take)
        );
        assert_eq!(control_at(&taking, 427, [212.0, 200.0]), None);
    }

    #[test]
    fn clicks_and_page_keys_press_the_vanilla_menu_buttons() {
        let mut ui = MinecraftUi {
            inventory_open: true,
            book: Some(book(3, 1, true)),
            ..MinecraftUi::default()
        };
        let click = |at: [f64; 2]| Press {
            mouse: Some(at),
            click: true,
            ..Press::default()
        };
        input(&mut ui, 427, click([165.0, 165.0]));
        input(&mut ui, 427, click([240.0, 165.0]));
        input(&mut ui, 427, click([250.0, 200.0]));
        input(
            &mut ui,
            427,
            Press {
                page_up: true,
                ..Press::default()
            },
        );
        input(
            &mut ui,
            427,
            Press {
                page_down: true,
                ..Press::default()
            },
        );
        input(&mut ui, 427, click([5.0, 5.0]));
        assert_eq!(
            ui.clicks,
            [
                McClick::Choose(1),
                McClick::Choose(2),
                McClick::Choose(3),
                McClick::Choose(1),
                McClick::Choose(2)
            ]
        );
        assert_eq!(
            ui.ui_sounds,
            [PAGE_TURN_SOUND, PAGE_TURN_SOUND, BUTTON_SOUND]
        );
        assert!(ui.inventory_open);
        input(&mut ui, 427, click([150.0, 200.0]));
        assert!(!ui.inventory_open);
        assert_eq!(ui.clicks.last(), Some(&McClick::Close));
        assert_eq!(ui.ui_sounds.last(), Some(&BUTTON_SOUND));
    }

    #[test]
    fn pages_wrap_at_114_pixels_and_14_lines() {
        let glyph = frame::McGlyph {
            rect: [0, 0, 8, 8],
            scale: 1.0,
            ascent: 7,
            advance: 6.0,
        };
        let mut font = McFont::default();
        for c in 'a'..='z' {
            font.glyphs.insert(c, glyph);
        }
        font.spaces.insert(' ', 4.0);
        let lines = page_lines(&font, "aaaaaaaaaa bbbbbbbbbb");
        assert_eq!(lines, ["aaaaaaaaaa", "bbbbbbbbbb"]);
        let nineteen = "a".repeat(19);
        assert_eq!(page_lines(&font, &nineteen), [nineteen.clone()]);
        let twenty = "a".repeat(20);
        assert_eq!(page_lines(&font, &twenty), [nineteen, "a".to_owned()]);
        let long = vec!["word"; 200].join(" ");
        let lines = page_lines(&font, &long);
        assert_eq!(lines.len(), 14);
        assert!(lines.iter().all(|line| font.width(line) <= TEXT_WIDTH));
        assert_eq!(page_lines(&font, "one\ntwo"), ["one", "two"]);
    }
}
