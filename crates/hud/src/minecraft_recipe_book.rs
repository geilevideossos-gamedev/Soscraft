use assets::PreparedWeapons;
use bevy::input::keyboard::Key;
use frame::minecraft_creative::MC_SEARCH_LIMIT;
use frame::minecraft_recipe_book::{MC_RECIPES_PER_PAGE, cycle};
use frame::{McClick, McFont, McGameMode, McRecipeBook, McScreen, McSlot, McStack, MinecraftUi};

use crate::minecraft_chat::{ChatClipboard, Mods, Typed};
use crate::minecraft_edit::Edit;
use crate::minecraft_gui::{GRAY, Painter, TextLine, WHITE};
use crate::minecraft_screens::Layout;

pub(crate) const SIZE: [i32; 2] = [147, 166];
const SHEET: &str = "recipe_book";
const OFFSET: i32 = 86;
const NARROW: i32 = 379;
const BUTTON: [i32; 2] = [20, 18];
const TAB: [i32; 2] = [35, 27];
const SEARCH: [i32; 4] = [25, 13, 81, 14];
const MAGNIFIER: [i32; 4] = [8, 13, 25, 14];
const FILTER: [i32; 4] = [110, 12, 26, 16];
const FORWARD: [i32; 4] = [93, 137, 12, 17];
const BACKWARD: [i32; 4] = [38, 137, 12, 17];
const CELL: i32 = 25;
const OPTION: i32 = 24;
const GHOST: u32 = 0x30FF_0000;
const GHOST_COVER: u32 = 0x30FF_FFFF;
const TICKS_PER_SECOND: f64 = 20.0;
const BLINK_MILLIS: f64 = 300.0;
const SELECTION: u32 = 0xFF00_00FF;
const CARET: u32 = 0xFFD0_D0D0;

fn inside([x, y]: [i32; 2], [rx, ry, w, h]: [i32; 4]) -> bool {
    x >= rx && y >= ry && x < rx + w && y < ry + h
}

fn at([px, py]: [i32; 2], [x, y, w, h]: [i32; 4]) -> [i32; 4] {
    [px + x, py + y, w, h]
}

pub(crate) fn ticks(now: f64) -> f64 {
    now * TICKS_PER_SECOND
}

pub(crate) fn has_book(screen: McScreen) -> bool {
    matches!(screen, McScreen::Inventory | McScreen::CraftingTable)
}

pub(crate) fn narrow(gui: [i32; 2]) -> bool {
    gui[0] < NARROW
}

pub(crate) fn origin(layout: &Layout, gui: [i32; 2], open: bool) -> [i32; 2] {
    let mut origin = layout.origin(gui);
    if open && has_book(layout.screen) && !narrow(gui) {
        origin[0] = 177 + (gui[0] - layout.size[0] - 200) / 2;
    }
    origin
}

pub(crate) fn covered(ui: &MinecraftUi, gui: [i32; 2]) -> bool {
    has_book(ui.screen) && ui.recipe_book.open && narrow(gui)
}

pub(crate) fn panel(gui: [i32; 2]) -> [i32; 2] {
    let offset = if narrow(gui) { 0 } else { OFFSET };
    [(gui[0] - SIZE[0]) / 2 - offset, (gui[1] - SIZE[1]) / 2]
}

pub(crate) fn button(screen: McScreen, origin: [i32; 2], gui: [i32; 2]) -> Option<[i32; 4]> {
    let [x, y] = match screen {
        McScreen::Inventory => [origin[0] + 104, gui[1] / 2 - 22],
        McScreen::CraftingTable => [origin[0] + 5, gui[1] / 2 - 49],
        _ => return None,
    };
    Some([x, y, BUTTON[0], BUTTON[1]])
}

pub(crate) fn tabs(book: &McRecipeBook, panel: [i32; 2]) -> Vec<(usize, [i32; 4])> {
    book.tabs
        .iter()
        .enumerate()
        .filter(|(_, visible)| **visible)
        .enumerate()
        .map(|(row, (tab, _))| {
            (
                tab,
                [
                    panel[0] - 30,
                    panel[1] + 3 + 27 * row as i32,
                    TAB[0],
                    TAB[1],
                ],
            )
        })
        .collect()
}

pub(crate) fn recipe_rect(panel: [i32; 2], index: usize) -> [i32; 4] {
    [
        panel[0] + 11 + CELL * (index % 5) as i32,
        panel[1] + 31 + CELL * (index / 5) as i32,
        CELL,
        CELL,
    ]
}

fn page(book: &McRecipeBook) -> usize {
    if book.page >= book.pages {
        0
    } else {
        book.page
    }
}

fn arrows(book: &McRecipeBook) -> [bool; 2] {
    let page = page(book);
    [
        book.pages > 1 && page + 1 < book.pages,
        book.pages > 1 && page > 0,
    ]
}

pub(crate) fn over(book: &McRecipeBook, gui: [i32; 2], point: [i32; 2]) -> bool {
    let panel = panel(gui);
    inside(point, [panel[0], panel[1], SIZE[0], SIZE[1]])
        || tabs(book, panel)
            .iter()
            .any(|(_, rect)| inside(point, *rect))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Overlay {
    pub origin: [i32; 2],
    pub columns: usize,
    pub rows: usize,
    pub count: usize,
}

impl Overlay {
    pub(crate) fn new(count: usize, button: [i32; 2], panel: [i32; 2]) -> Self {
        let columns = if count <= 16 { 4 } else { 5 };
        let rows = count.div_ceil(columns);
        let [mut x, mut y] = button;
        let centre = [panel[0] + SIZE[0] / 2, panel[1] + 13 + SIZE[1] / 2];
        let width = CELL as f32;
        let right = x + count.min(columns) as i32 * CELL;
        let limit = centre[0] + 50;
        if right > limit {
            x = (x as f32 - width * ((right - limit) as f32 / width) as i32 as f32) as i32;
        }
        let bottom = y + rows as i32 * CELL;
        let limit = centre[1] + 50;
        if bottom > limit {
            y = (y as f32 - width * ((bottom - limit) as f32 / width).ceil()) as i32;
        }
        let limit = centre[1] - 100;
        if y < limit {
            y = (y as f32 - width * ((y - limit) as f32 / width).ceil()) as i32;
        }
        Self {
            origin: [x, y],
            columns,
            rows,
            count,
        }
    }

    pub(crate) fn frame(&self) -> [i32; 4] {
        [
            self.origin[0],
            self.origin[1],
            self.count.min(self.columns) as i32 * CELL + 8,
            self.rows as i32 * CELL + 8,
        ]
    }

    pub(crate) fn option(&self, index: usize) -> [i32; 4] {
        [
            self.origin[0] + 4 + CELL * (index % self.columns) as i32,
            self.origin[1] + 5 + CELL * (index / self.columns) as i32,
            OPTION,
            OPTION,
        ]
    }
}

fn overlay_of(book: &McRecipeBook, panel: [i32; 2]) -> Option<(usize, Overlay)> {
    let index = book.overlay?;
    let button = book.buttons.get(index)?;
    let count = button.alternatives(book.filtering).len();
    let [x, y, _, _] = recipe_rect(panel, index);
    Some((index, Overlay::new(count, [x, y], panel)))
}

#[derive(Default)]
pub(crate) struct BookInput {
    pub(crate) since: f64,
    display: usize,
}

pub(crate) struct BookFrame<'a> {
    pub mouse: Option<[i32; 2]>,
    pub pressed: [bool; 2],
    pub typed: &'a [Typed],
    pub chat_key: bool,
    pub now: f64,
    pub gui: [i32; 2],
    pub origin: [i32; 2],
    pub shift: bool,
}

pub(crate) fn input(
    ui: &mut MinecraftUi,
    input: &mut BookInput,
    frame: BookFrame<'_>,
    font: Option<&McFont>,
    clipboard: &mut dyn ChatClipboard,
) -> [bool; 2] {
    if !has_book(ui.screen) || ui.game_mode == McGameMode::Spectator {
        return [false; 2];
    }
    let toggle = button(ui.screen, frame.origin, frame.gui);
    let MinecraftUi {
        recipe_book: book,
        clicks,
        ..
    } = ui;
    if book.page >= book.pages {
        book.page = 0;
    }
    let mut consumed = [false; 2];
    if let Some(mouse) = frame.mouse.filter(|_| frame.pressed.contains(&true)) {
        if frame.pressed[0] && toggle.is_some_and(|rect| inside(mouse, rect)) {
            book.open = !book.open;
            book.overlay = None;
            book.focused = false;
            return [true, false];
        }
        if book.open {
            let pick = Pick {
                mouse,
                pressed: frame.pressed,
                gui: frame.gui,
                ticks: ticks(frame.now),
                shift: frame.shift,
            };
            consumed = click(book, pick, clicks);
            if book.focused && consumed[0] {
                input.since = frame.now;
            }
        }
    }
    if !book.open {
        return consumed;
    }
    if book.focused {
        for typed in frame.typed {
            if type_query(book, typed, clipboard) {
                input.since = frame.now;
            }
        }
    } else if frame.chat_key {
        book.focused = true;
        input.since = frame.now;
    }
    if let Some(font) = font {
        let pos = book.query[..book.cursor.min(book.query.len())]
            .chars()
            .count();
        input.display = crate::minecraft_inventory::scroll_within(
            font,
            &book.query,
            input.display,
            pos,
            SEARCH[2] - 8,
        );
    }
    consumed
}

#[derive(Clone, Copy)]
pub(crate) struct Pick {
    pub mouse: [i32; 2],
    pub pressed: [bool; 2],
    pub gui: [i32; 2],
    pub ticks: f64,
    pub shift: bool,
}

pub(crate) fn click(book: &mut McRecipeBook, pick: Pick, clicks: &mut Vec<McClick>) -> [bool; 2] {
    let Pick {
        mouse,
        pressed,
        gui,
        ticks,
        shift,
    } = pick;
    let panel = panel(gui);
    if let Some((index, overlay)) = overlay_of(book, panel) {
        if pressed[0] {
            let options = book.buttons[index].alternatives(book.filtering);
            if let Some(chosen) = (0..options.len())
                .find(|&option| inside(mouse, overlay.option(option)))
                .map(|chosen| options[chosen].clone())
            {
                clicks.extend(book.place(&chosen, shift));
                if narrow(gui) {
                    book.open = false;
                }
            }
        }
        book.overlay = None;
        return pressed;
    }
    book.overlay = None;
    let [forward, backward] = arrows(book);
    if pressed[0] && forward && inside(mouse, at(panel, FORWARD)) {
        book.page = page(book);
        book.turn(true);
        return pressed;
    }
    if pressed[0] && backward && inside(mouse, at(panel, BACKWARD)) {
        book.page = page(book);
        book.turn(false);
        return pressed;
    }
    if let Some(index) = (0..book.buttons.len().min(MC_RECIPES_PER_PAGE))
        .find(|&index| inside(mouse, recipe_rect(panel, index)))
    {
        let button = &book.buttons[index];
        if pressed[0] {
            if let Some(shown) = button.shown(book.filtering, ticks).cloned() {
                clicks.extend(book.place(&shown, shift));
                if narrow(gui) {
                    book.open = false;
                }
            }
        } else if pressed[1] && button.selected(book.filtering).len() > 1 {
            book.overlay = Some(index);
        }
        book.focused = false;
        return pressed;
    }
    if pressed[0] && (inside(mouse, at(panel, SEARCH)) || inside(mouse, at(panel, MAGNIFIER))) {
        book.focused = true;
        return [true, false];
    }
    book.focused = false;
    if pressed[0] && inside(mouse, at(panel, FILTER)) {
        book.filtering = !book.filtering;
        return [true, false];
    }
    if pressed[0]
        && let Some((tab, _)) = tabs(book, panel)
            .into_iter()
            .find(|(_, rect)| inside(mouse, *rect))
    {
        book.select_tab(tab);
        return [true, false];
    }
    [false; 2]
}

pub(crate) fn type_query(
    book: &mut McRecipeBook,
    typed: &Typed,
    clipboard: &mut dyn ChatClipboard,
) -> bool {
    let Mods { ctrl, shift, alt } = typed.mods;
    let before = book.query.clone();
    let mut edit = Edit::new(
        &mut book.query,
        &mut book.cursor,
        &mut book.highlight,
        MC_SEARCH_LIMIT,
    );
    match typed.key {
        Key::Backspace => edit.delete(-1, ctrl),
        Key::Delete => edit.delete(1, ctrl),
        Key::ArrowLeft => edit.step(-1, ctrl, shift),
        Key::ArrowRight => edit.step(1, ctrl, shift),
        Key::Home => edit.move_to(0, shift),
        Key::End => edit.move_to(usize::MAX, shift),
        _ => match crate::minecraft_chat::shortcut(typed) {
            Some('a') => edit.select_all(),
            Some('c') => {
                let text = edit.highlighted().to_owned();
                if !text.is_empty() {
                    clipboard.set(&text);
                }
            }
            Some('v') => {
                let text = clipboard.get();
                edit.insert(&text);
            }
            Some('x') => {
                let text = edit.highlighted().to_owned();
                if !text.is_empty() {
                    clipboard.set(&text);
                }
                edit.insert("");
            }
            _ => {
                if let Some(text) = typed.text.as_deref().filter(|_| !ctrl || alt) {
                    edit.insert(text);
                }
            }
        },
    }
    if book.query != before {
        book.overlay = None;
        true
    } else {
        false
    }
}

pub(crate) fn clears_ghost(slot: McSlot) -> bool {
    matches!(
        slot,
        McSlot::Crafting(_) | McSlot::Result | McSlot::Workbench(_) | McSlot::WorkbenchResult
    )
}

pub(crate) fn touches_grid(click: &McClick) -> bool {
    match click {
        McClick::Slot { slot, .. }
        | McClick::Gather { slot, .. }
        | McClick::Swap { slot, .. }
        | McClick::Throw { slot, .. } => clears_ghost(*slot),
        McClick::Spread { slots, .. } => slots.iter().copied().any(clears_ghost),
        _ => false,
    }
}

pub(crate) struct Look<'a> {
    pub ui: &'a MinecraftUi,
    pub weapons: Option<&'a PreparedWeapons>,
    pub origin: [i32; 2],
    pub gui: [i32; 2],
    pub mouse: Option<[i32; 2]>,
    pub now: f64,
}

impl Look<'_> {
    fn hovered(&self, rect: [i32; 4]) -> bool {
        self.mouse.is_some_and(|mouse| inside(mouse, rect))
    }
}

pub(crate) fn draw_button(p: &mut Painter<'_>, look: &Look<'_>) {
    if look.ui.game_mode == McGameMode::Spectator {
        return;
    }
    let Some(rect) = button(look.ui.screen, look.origin, look.gui) else {
        return;
    };
    let sprite = if look.hovered(rect) {
        "recipe_book/button_highlighted"
    } else {
        "recipe_book/button"
    };
    p.sprite(sprite, rect[0], rect[1], rect[2], rect[3]);
}

pub(crate) fn draw_ghost(p: &mut Painter<'_>, look: &Look<'_>, layout: &Layout) {
    let ticks = ticks(look.now);
    for (slot, stacks) in &look.ui.recipe_book.ghost_cells {
        if crate::minecraft_inventory::stack_at(look.ui, *slot).is_some() {
            continue;
        }
        let (Some(spec), Some(index)) = (layout.spec(*slot), cycle(ticks, stacks.len())) else {
            continue;
        };
        let (x, y) = (look.origin[0] + spec.x, look.origin[1] + spec.y);
        let result = matches!(slot, McSlot::Result | McSlot::WorkbenchResult);
        if result && *slot == McSlot::WorkbenchResult {
            p.fill(x - 4, y - 4, x + 20, y + 20, GHOST);
        } else {
            p.fill(x, y, x + 16, y + 16, GHOST);
        }
        let stack = &stacks[index];
        p.item(look.ui, look.weapons, stack, x, y);
        p.fill(x, y, x + 16, y + 16, GHOST_COVER);
        if result {
            p.decorations(look.ui, stack, [x, y], None);
        }
    }
}

pub(crate) fn draw(p: &mut Painter<'_>, look: &Look<'_>, input: &BookInput) {
    let book = &look.ui.recipe_book;
    if !book.open || !has_book(look.ui.screen) || look.ui.game_mode == McGameMode::Spectator {
        return;
    }
    let panel = panel(look.gui);
    let ticks = ticks(look.now);
    p.blit(SHEET, panel[0], panel[1], 1, 1, SIZE[0], SIZE[1]);
    draw_search(p, look, panel, input);
    let selected = book.shown_tab();
    for (tab, [x, y, w, h]) in tabs(book, panel) {
        let chosen = tab == selected;
        let shift = if chosen { -2 } else { 0 };
        p.sprite(
            if chosen {
                "recipe_book/tab_selected"
            } else {
                "recipe_book/tab"
            },
            x + shift,
            y,
            w,
            h,
        );
        let icons = book.tab_icons.get(tab).map_or(&[][..], Vec::as_slice);
        match icons {
            [one] => p.item(look.ui, look.weapons, one, x + 9 + shift, y + 5),
            [first, second, ..] => {
                p.item(look.ui, look.weapons, first, x + 3 + shift, y + 5);
                p.item(look.ui, look.weapons, second, x + 14 + shift, y + 5);
            }
            [] => {}
        }
    }
    let filter = at(panel, FILTER);
    let sprite = match (book.filtering, look.hovered(filter)) {
        (true, false) => "recipe_book/filter_enabled",
        (true, true) => "recipe_book/filter_enabled_highlighted",
        (false, false) => "recipe_book/filter_disabled",
        (false, true) => "recipe_book/filter_disabled_highlighted",
    };
    p.sprite(sprite, filter[0], filter[1], filter[2], filter[3]);
    if book.pages > 1 {
        let text = p.gui.format(
            "gui.recipebook.page",
            &[&(page(book) + 1).to_string(), &book.pages.to_string()],
        );
        let width = p.width(&text);
        p.text(
            &text,
            panel[0] - width / 2 + 73,
            panel[1] + 141,
            WHITE,
            true,
        );
    }
    for (index, button) in book.buttons.iter().take(MC_RECIPES_PER_PAGE).enumerate() {
        let [x, y, w, h] = recipe_rect(panel, index);
        let many = button.selected(book.filtering).len() > 1;
        let sprite = match (button.craftable(), many) {
            (true, true) => "recipe_book/slot_many_craftable",
            (true, false) => "recipe_book/slot_craftable",
            (false, true) => "recipe_book/slot_many_uncraftable",
            (false, false) => "recipe_book/slot_uncraftable",
        };
        p.sprite(sprite, x, y, w, h);
        let Some(shown) = button.shown(book.filtering, ticks) else {
            continue;
        };
        let mut offset = 4;
        if many && button.same_result(book.filtering) {
            p.item(
                look.ui,
                look.weapons,
                &shown.result,
                x + offset + 1,
                y + offset + 1,
            );
            offset -= 1;
        }
        p.item(look.ui, look.weapons, &shown.result, x + offset, y + offset);
    }
    let [forward, backward] = arrows(book);
    for (visible, rect, name) in [
        (forward, at(panel, FORWARD), "recipe_book/page_forward"),
        (backward, at(panel, BACKWARD), "recipe_book/page_backward"),
    ] {
        if visible {
            let name = if look.hovered(rect) {
                format!("{name}_highlighted")
            } else {
                name.to_owned()
            };
            p.sprite(&name, rect[0], rect[1], rect[2], rect[3]);
        }
    }
    if let Some((index, overlay)) = overlay_of(book, panel) {
        let [x, y, w, h] = overlay.frame();
        p.sprite("recipe_book/overlay_recipe", x, y, w, h);
        for (option, recipe) in book.buttons[index]
            .alternatives(book.filtering)
            .into_iter()
            .enumerate()
        {
            let rect = overlay.option(option);
            let sprite = match (recipe.craftable, look.hovered(rect)) {
                (true, false) => "recipe_book/crafting_overlay",
                (true, true) => "recipe_book/crafting_overlay_highlighted",
                (false, false) => "recipe_book/crafting_overlay_disabled",
                (false, true) => "recipe_book/crafting_overlay_disabled_highlighted",
            };
            p.sprite(sprite, rect[0], rect[1], rect[2], rect[3]);
            for (cell, stacks) in recipe.cells.iter().enumerate() {
                let Some(stacks) = stacks else { continue };
                let Some(pick) = cycle(ticks, stacks.len()) else {
                    continue;
                };
                let centre = [
                    rect[0] + 5 + 7 * (cell % 3) as i32,
                    rect[1] + 5 + 7 * (cell / 3) as i32,
                ];
                p.item_sized(
                    look.ui,
                    look.weapons,
                    &stacks[pick],
                    [centre[0] as f32 - 3.0, centre[1] as f32 - 3.0],
                    6.0,
                    WHITE,
                );
            }
        }
    }
}

fn draw_search(p: &mut Painter<'_>, look: &Look<'_>, panel: [i32; 2], input: &BookInput) {
    let book = &look.ui.recipe_book;
    let [x, y, w, h] = at(panel, SEARCH);
    p.sprite(
        if book.focused {
            "widget/text_field_highlighted"
        } else {
            "widget/text_field"
        },
        x,
        y,
        w,
        h,
    );
    let (tx, ty, width) = (x + 4, y + (h - 8) / 2, w - 8);
    if book.query.is_empty() && !book.focused {
        let hint = p.gui.text("gui.recipebook.search_hint");
        p.styled_text(false, &hint, tx, ty, GRAY, true, true);
        return;
    }
    let chars: Vec<char> = book.query.chars().collect();
    let char_at = |byte: usize| book.query[..byte.min(book.query.len())].chars().count();
    let display = input.display.min(chars.len());
    let rest: String = chars[display..].iter().collect();
    let shown: Vec<char> = p.gui.font.head_by_width(&rest, width).chars().collect();
    let cursor = char_at(book.cursor) as i64 - display as i64;
    let in_view = cursor >= 0 && cursor <= shown.len() as i64;
    let blink = book.focused
        && ((look.now - input.since) * 1000.0 / BLINK_MILLIS) as i64 % 2 == 0
        && in_view;
    let not_at_end = char_at(book.cursor) < chars.len() || chars.len() >= MC_SEARCH_LIMIT;
    let head: String = if in_view {
        shown[..cursor as usize].iter().collect()
    } else {
        shown.iter().collect()
    };
    let mut end = tx;
    if !shown.is_empty() {
        p.text(&head, tx, ty, WHITE, true);
        end = tx + p.width(&head) + 1;
    }
    let mut caret = end;
    if !in_view {
        caret = if cursor > 0 { tx + width } else { tx };
    } else if not_at_end {
        caret = end - 1;
        end -= 1;
    }
    if !shown.is_empty() && in_view && (cursor as usize) < shown.len() {
        let tail: String = shown[cursor as usize..].iter().collect();
        p.text(&tail, end, ty, WHITE, true);
    }
    let highlight = (char_at(book.highlight) as i64 - display as i64).clamp(0, shown.len() as i64);
    if book.focused && highlight != cursor {
        let upto: String = shown[..highlight as usize].iter().collect();
        let other = tx + p.width(&upto);
        p.fill(
            caret.min(other),
            ty - 1,
            caret.max(other) - 1,
            ty + 1 + 9,
            SELECTION,
        );
    }
    if blink {
        if not_at_end {
            p.fill(caret, ty - 1, caret + 1, ty + 1 + 9, CARET);
        } else {
            p.text("_", caret, ty, WHITE, true);
        }
    }
}

pub(crate) enum Tip {
    Item(Box<McStack>, bool),
    Text(String),
}

pub(crate) fn tooltip(
    p: &Painter<'_>,
    look: &Look<'_>,
    layout: &Layout,
    hovered: Option<McSlot>,
) -> Option<Tip> {
    let book = &look.ui.recipe_book;
    if !has_book(look.ui.screen) || look.ui.game_mode == McGameMode::Spectator {
        return None;
    }
    let ticks = ticks(look.now);
    let mouse = look.mouse?;
    if book.open {
        let panel = panel(look.gui);
        if book.overlay.is_none()
            && let Some(index) = (0..book.buttons.len().min(MC_RECIPES_PER_PAGE))
                .find(|&index| inside(mouse, recipe_rect(panel, index)))
        {
            let button = &book.buttons[index];
            let shown = button.shown(book.filtering, ticks)?;
            return Some(Tip::Item(
                Box::new(shown.result.clone()),
                button.selected(book.filtering).len() > 1,
            ));
        }
        if inside(mouse, at(panel, FILTER)) {
            let key = if book.filtering {
                "gui.recipebook.toggleRecipes.craftable"
            } else {
                "gui.recipebook.toggleRecipes.all"
            };
            return Some(Tip::Text(p.gui.text(key)));
        }
        let [forward, backward] = arrows(book);
        if forward && inside(mouse, at(panel, FORWARD)) {
            return Some(Tip::Text(p.gui.text("gui.recipebook.next_page")));
        }
        if backward && inside(mouse, at(panel, BACKWARD)) {
            return Some(Tip::Text(p.gui.text("gui.recipebook.previous_page")));
        }
    }
    let slot = hovered.filter(|slot| {
        layout.spec(*slot).is_some()
            && crate::minecraft_inventory::stack_at(look.ui, *slot).is_none()
    })?;
    let (_, stacks) = book.ghost_cells.iter().find(|(ghost, _)| *ghost == slot)?;
    let index = cycle(ticks, stacks.len())?;
    Some(Tip::Item(Box::new(stacks[index].clone()), false))
}

pub(crate) fn more_line(p: &Painter<'_>) -> TextLine {
    TextLine::new(p.gui.text("gui.recipebook.moreRecipes"), WHITE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::keyboard::KeyCode;
    use frame::{McRecipeButton, McRecipeOption};

    fn stack(id: &str) -> McStack {
        McStack {
            id: id.to_owned(),
            count: 1,
            weapon: None,
            damage: None,
            max: 64,
            named: false,
            name: None,
            attack: None,
            enchantments: Vec::new(),
            contents: Vec::new(),
            more: None,
            glint: false,
            icon: id.to_owned(),
            lore: Vec::new(),
            clip: None,
            ..Default::default()
        }
    }

    fn option(id: &str, craftable: bool) -> McRecipeOption {
        McRecipeOption {
            id: id.to_owned(),
            result: stack(id),
            craftable,
            cells: Vec::new(),
        }
    }

    fn book() -> McRecipeBook {
        McRecipeBook {
            open: true,
            tabs: [true, true, false, true, false],
            pages: 2,
            buttons: vec![
                McRecipeButton {
                    recipes: vec![
                        option("minecraft:oak_planks", true),
                        option("minecraft:birch_planks", false),
                    ],
                },
                McRecipeButton {
                    recipes: vec![option("minecraft:stick", false)],
                },
            ],
            ..McRecipeBook::default()
        }
    }

    const GUI: [i32; 2] = [427, 240];

    fn tap(
        book: &mut McRecipeBook,
        sent: &mut Vec<McClick>,
        mouse: [i32; 2],
        pressed: [bool; 2],
        gui: [i32; 2],
        ticks: f64,
    ) -> [bool; 2] {
        let pick = Pick {
            mouse,
            pressed,
            gui,
            ticks,
            shift: false,
        };
        click(book, pick, sent)
    }

    fn placed(recipe: &str, all: bool) -> McClick {
        McClick::PlaceRecipe {
            recipe: recipe.to_owned(),
            all,
        }
    }

    #[test]
    fn the_book_opens_left_of_the_screen_and_shifts_it_like_vanilla() {
        let inventory = crate::minecraft_screens::layout(McScreen::Inventory);
        assert_eq!(inventory.origin(GUI), [125, 37]);
        assert_eq!(origin(&inventory, GUI, false), [125, 37]);
        assert_eq!(origin(&inventory, GUI, true), [202, 37]);
        assert_eq!(panel(GUI), [54, 37]);
        assert_eq!(
            button(McScreen::Inventory, [125, 37], GUI),
            Some([229, 98, 20, 18])
        );
        assert_eq!(
            button(McScreen::Inventory, [202, 37], GUI),
            Some([306, 98, 20, 18])
        );
        let table = crate::minecraft_screens::layout(McScreen::CraftingTable);
        let shifted = origin(&table, GUI, true);
        assert_eq!(
            button(McScreen::CraftingTable, shifted, GUI),
            Some([shifted[0] + 5, 71, 20, 18])
        );
        assert_eq!(button(McScreen::Anvil, [0, 0], GUI), None);
        let anvil = crate::minecraft_screens::layout(McScreen::Anvil);
        assert_eq!(origin(&anvil, GUI, true), anvil.origin(GUI));
        let small = [320, 240];
        assert!(narrow(small) && !narrow(GUI));
        assert_eq!(origin(&inventory, small, true), inventory.origin(small));
        assert_eq!(panel(small), [86, 37]);
    }

    #[test]
    fn tabs_stack_down_the_left_edge_and_recipes_fill_five_columns() {
        let tabs = tabs(&book(), [54, 37]);
        assert_eq!(
            tabs,
            [
                (0, [24, 40, 35, 27]),
                (1, [24, 67, 35, 27]),
                (3, [24, 94, 35, 27])
            ]
        );
        assert_eq!(recipe_rect([54, 37], 0), [65, 68, 25, 25]);
        assert_eq!(recipe_rect([54, 37], 19), [165, 143, 25, 25]);
        assert!(over(&book(), GUI, [30, 50]));
        assert!(over(&book(), GUI, [200, 202]));
        assert!(!over(&book(), GUI, [30, 130]));
        assert!(!over(&book(), GUI, [201, 50]));
    }

    #[test]
    fn the_alternatives_overlay_stays_near_the_middle_of_the_book() {
        let small = Overlay::new(3, [65, 68], [54, 37]);
        assert_eq!(small.origin, [65, 68]);
        assert_eq!(small.frame(), [65, 68, 83, 33]);
        assert_eq!(small.option(2), [119, 73, 24, 24]);
        let wide = Overlay::new(4, [165, 143], [54, 37]);
        assert_eq!((wide.columns, wide.rows), (4, 1));
        assert_eq!(wide.origin, [90, 143]);
        let many = Overlay::new(17, [65, 143], [54, 37]);
        assert_eq!((many.columns, many.rows), (5, 4));
        assert_eq!(many.origin, [65, 68]);
        assert_eq!(Overlay::new(17, [65, 68], [54, 37]).origin, [65, 68]);
    }

    #[test]
    fn clicks_open_tabs_turn_pages_pick_ghosts_and_open_alternatives() {
        let mut book = book();
        let mut sent = Vec::new();
        let panel = panel(GUI);
        assert_eq!(
            tap(&mut book, &mut sent, [30, 70], [true, false], GUI, 0.0),
            [true, false]
        );
        assert_eq!((book.tab, book.page), (1, 0));
        assert_eq!(
            tap(
                &mut book,
                &mut sent,
                [panel[0] + 95, panel[1] + 140],
                [true, false],
                GUI,
                0.0
            ),
            [true, false]
        );
        assert_eq!(book.page, 1);
        tap(
            &mut book,
            &mut sent,
            [panel[0] + 40, panel[1] + 140],
            [true, false],
            GUI,
            0.0,
        );
        assert_eq!(book.page, 0);
        tap(
            &mut book,
            &mut sent,
            [panel[0] + 40, panel[1] + 140],
            [true, false],
            GUI,
            0.0,
        );
        assert_eq!(book.page, 0);
        let first = recipe_rect(panel, 0);
        tap(
            &mut book,
            &mut sent,
            [first[0] + 2, first[1] + 2],
            [true, false],
            GUI,
            31.0,
        );
        assert_eq!(book.ghost.as_deref(), Some("minecraft:birch_planks"));
        assert_eq!(sent, [placed("minecraft:birch_planks", false)]);
        let second = recipe_rect(panel, 1);
        tap(
            &mut book,
            &mut sent,
            [second[0] + 2, second[1] + 2],
            [false, true],
            GUI,
            0.0,
        );
        assert_eq!(book.overlay, None, "one recipe has no alternatives");
        assert_eq!(
            tap(
                &mut book,
                &mut sent,
                [first[0] + 2, first[1] + 2],
                [false, true],
                GUI,
                0.0
            ),
            [false, true]
        );
        assert_eq!(book.overlay, Some(0));
        let overlay = Overlay::new(2, [first[0], first[1]], panel).option(1);
        tap(
            &mut book,
            &mut sent,
            [overlay[0] + 1, overlay[1] + 1],
            [true, false],
            GUI,
            0.0,
        );
        assert_eq!(book.ghost.as_deref(), Some("minecraft:birch_planks"));
        assert_eq!(book.overlay, None);
        assert_eq!(
            sent.len(),
            1,
            "the same missing recipe is not asked for again"
        );
        book.overlay = Some(0);
        let oak = Overlay::new(2, [first[0], first[1]], panel).option(0);
        let pick = Pick {
            mouse: [oak[0] + 1, oak[1] + 1],
            pressed: [true, false],
            gui: GUI,
            ticks: 0.0,
            shift: true,
        };
        click(&mut book, pick, &mut sent);
        assert_eq!(sent[1..], [placed("minecraft:oak_planks", true)]);
        assert_eq!(book.ghost, None);
        book.overlay = Some(0);
        assert_eq!(
            tap(&mut book, &mut sent, [5, 5], [true, false], GUI, 0.0),
            [true, false],
            "a click beside the alternatives only closes them"
        );
        assert_eq!(book.overlay, None);
        assert_eq!(
            tap(
                &mut book,
                &mut sent,
                [panel[0] + 30, panel[1] + 16],
                [true, false],
                GUI,
                0.0
            ),
            [true, false]
        );
        assert!(book.focused);
        tap(
            &mut book,
            &mut sent,
            [panel[0] + 112, panel[1] + 14],
            [true, false],
            GUI,
            0.0,
        );
        assert!(book.filtering && !book.focused);
        assert_eq!(
            tap(&mut book, &mut sent, [5, 5], [true, false], GUI, 0.0),
            [false, false]
        );
        book.filtering = false;
        book.open = true;
        let narrow_first = recipe_rect(super::panel([320, 240]), 0);
        tap(
            &mut book,
            &mut sent,
            [narrow_first[0] + 2, narrow_first[1] + 2],
            [true, false],
            [320, 240],
            0.0,
        );
        assert!(!book.open, "a narrow screen closes the book after a pick");
    }

    #[test]
    fn the_book_button_toggles_the_book_and_typing_needs_the_search_box() {
        let mut ui = MinecraftUi {
            screen: McScreen::Inventory,
            ..MinecraftUi::default()
        };
        let mut input = BookInput::default();
        let origin = [125, 37];
        let typed = [Typed {
            key: Key::Character("s".into()),
            code: KeyCode::KeyS,
            text: Some("s".to_owned()),
            mods: Mods::default(),
        }];
        let frame = |mouse, pressed, chat_key| BookFrame {
            mouse,
            pressed,
            typed: &typed,
            chat_key,
            now: 1.0,
            gui: GUI,
            origin,
            shift: false,
        };
        assert_eq!(
            input_of(
                &mut ui,
                &mut input,
                frame(Some([230, 100]), [true, false], false)
            ),
            [true, false]
        );
        assert!(ui.recipe_book.open);
        input_of(&mut ui, &mut input, frame(None, [false; 2], false));
        assert_eq!(ui.recipe_book.query, "");
        input_of(&mut ui, &mut input, frame(None, [false; 2], true));
        assert!(ui.recipe_book.focused);
        input_of(&mut ui, &mut input, frame(None, [false; 2], false));
        assert_eq!(ui.recipe_book.query, "s");
        ui.screen = McScreen::Anvil;
        assert_eq!(
            input_of(
                &mut ui,
                &mut input,
                frame(Some([230, 100]), [true, false], false)
            ),
            [false, false]
        );
        assert!(clears_ghost(McSlot::WorkbenchResult) && !clears_ghost(McSlot::Inventory(3)));
        assert!(touches_grid(&McClick::Slot {
            slot: McSlot::Crafting(2),
            right: false,
            shift: false
        }));
        assert!(touches_grid(&McClick::Spread {
            slots: vec![McSlot::Inventory(4), McSlot::Workbench(0)],
            right: true
        }));
        assert!(!touches_grid(&McClick::Swap {
            slot: McSlot::Inventory(10),
            hotbar: 2
        }));
        assert!(!touches_grid(&McClick::Outside { right: false }));
    }

    fn input_of(ui: &mut MinecraftUi, state: &mut BookInput, frame: BookFrame<'_>) -> [bool; 2] {
        input(ui, state, frame, None, &mut ())
    }
}
