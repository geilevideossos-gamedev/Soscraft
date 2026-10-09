use bevy::input::keyboard::Key;
use frame::minecraft_creative::{
    MC_SEARCH_LIMIT, can_scroll, scroll_after_wheel, scroll_for_drag, scroller_y,
};
use frame::{McCreative, McCreativeTab, McFont, McScreen, McSlot, McStack, McTabKind, MinecraftUi};

use crate::minecraft_chat::{ChatClipboard, Mods, Typed};
use crate::minecraft_edit::Edit;
use crate::minecraft_gui::{LABEL, Painter, TextLine, WHITE};
use crate::minecraft_screens::{Group, Layout, SlotSpec};

pub(crate) const SIZE: [i32; 2] = [195, 136];
pub(crate) const PUPPET_BOX: [i32; 4] = [73, 6, 105, 49];
pub(crate) const PUPPET_HEIGHT: f32 = 70.0 * 20.0 / 30.0;
const SEARCH_BOX: [i32; 3] = [82, 6, 80];
const HIGHLIGHT: u32 = 0xFF00_00FF;
const BLINK_MILLIS: f64 = 300.0;
const BLUE: u32 = 0xFF55_55FF;
const DARK_PURPLE: u32 = 0xFFAA_00AA;

fn spec(slot: McSlot, x: i32, y: i32, group: Group, empty: Option<&'static str>) -> SlotSpec {
    SlotSpec {
        slot,
        x,
        y,
        group,
        empty,
    }
}

pub(crate) fn layout(kind: McTabKind, first: usize) -> Layout {
    let mut slots = Vec::new();
    if kind == McTabKind::Inventory {
        let armor = [
            (39, 54, 6, "container/slot/helmet"),
            (38, 54, 33, "container/slot/chestplate"),
            (37, 108, 6, "container/slot/leggings"),
            (36, 108, 33, "container/slot/boots"),
        ];
        for (slot, x, y, icon) in armor {
            slots.push(spec(
                McSlot::Inventory(slot),
                x,
                y,
                Group::Player,
                Some(icon),
            ));
        }
        for slot in 9..36 {
            let i = (slot - 9) as i32;
            slots.push(spec(
                McSlot::Inventory(slot),
                9 + i % 9 * 18,
                54 + i / 9 * 18,
                Group::Player,
                None,
            ));
        }
        slots.push(spec(
            McSlot::Inventory(40),
            35,
            20,
            Group::Player,
            Some("container/slot/shield"),
        ));
    } else {
        for i in 0..45 {
            slots.push(spec(
                McSlot::Creative(first + i),
                9 + (i % 9) as i32 * 18,
                18 + (i / 9) as i32 * 18,
                Group::Cells,
                None,
            ));
        }
    }
    for i in 0..9 {
        slots.push(spec(
            McSlot::Inventory(i),
            9 + i as i32 * 18,
            112,
            Group::Player,
            None,
        ));
    }
    if kind == McTabKind::Inventory {
        slots.push(spec(McSlot::Trash, 173, 112, Group::Output, None));
    }
    Layout {
        screen: McScreen::Creative(kind),
        size: SIZE,
        slots,
    }
}

pub(crate) fn tab_x(tab: &McCreativeTab) -> i32 {
    let column = i32::from(tab.column);
    if tab.aligned_right {
        SIZE[0] - 27 * (7 - column) + 1
    } else {
        column * 27
    }
}

pub(crate) fn tab_y(tab: &McCreativeTab) -> i32 {
    if tab.top { -32 } else { SIZE[1] }
}

pub(crate) fn tab_at(tabs: &[McCreativeTab], [mx, my]: [f64; 2]) -> Option<usize> {
    tabs.iter().position(|tab| {
        let (x, y) = (f64::from(tab_x(tab)), f64::from(tab_y(tab)));
        mx >= x && mx <= x + 26.0 && my >= y && my <= y + 32.0
    })
}

pub(crate) fn tab_hovered(tabs: &[McCreativeTab], [mx, my]: [f64; 2]) -> Option<usize> {
    tabs.iter().position(|tab| {
        let (x, y) = (f64::from(tab_x(tab) + 3), f64::from(tab_y(tab) + 3));
        mx >= x - 1.0 && mx < x + 21.0 + 1.0 && my >= y - 1.0 && my < y + 27.0 + 1.0
    })
}

pub(crate) fn over_scrollbar([mx, my]: [f64; 2]) -> bool {
    (175.0..189.0).contains(&mx) && (18.0..130.0).contains(&my)
}

pub(crate) fn tab_sprite(tab: &McCreativeTab, selected: bool) -> String {
    format!(
        "container/creative_inventory/tab_{}_{}_{}",
        if tab.top { "top" } else { "bottom" },
        if selected { "selected" } else { "unselected" },
        tab.column.min(6) + 1
    )
}

pub(crate) fn select(creative: &mut McCreative, index: usize) {
    let Some(tab) = creative.tabs.get(index) else {
        return;
    };
    let same = creative.selected_index() == Some(index);
    let id = tab.id.clone();
    if !(same && tab.kind == McTabKind::Search) {
        creative.query.clear();
        creative.cursor = 0;
        creative.highlight = 0;
    }
    creative.selected = Some(id);
    creative.scroll = 0.0;
}

pub(crate) fn reopen(creative: &mut McCreative) {
    creative.query.clear();
    creative.cursor = 0;
    creative.highlight = 0;
    creative.scroll = 0.0;
}

pub(crate) fn screen_of(creative: &McCreative) -> Option<McScreen> {
    creative
        .selected_tab()
        .map(|tab| McScreen::Creative(tab.kind))
}

pub(crate) fn view_start(font: &McFont, value: &str, display: usize, pos: usize) -> usize {
    crate::minecraft_inventory::scroll_within(font, value, display, pos, SEARCH_BOX[2])
}

fn shortcut(typed: &Typed) -> Option<char> {
    crate::minecraft_chat::shortcut(typed)
}

pub(crate) fn type_search(
    creative: &mut McCreative,
    typed: &Typed,
    clipboard: &mut dyn ChatClipboard,
) -> bool {
    let Mods { ctrl, shift, alt } = typed.mods;
    let before = creative.query.clone();
    let mut edit = Edit::new(
        &mut creative.query,
        &mut creative.cursor,
        &mut creative.highlight,
        MC_SEARCH_LIMIT,
    );
    match typed.key {
        Key::Backspace => edit.delete(-1, ctrl),
        Key::Delete => edit.delete(1, ctrl),
        Key::ArrowLeft => edit.step(-1, ctrl, shift),
        Key::ArrowRight => edit.step(1, ctrl, shift),
        Key::Home => edit.move_to(0, shift),
        Key::End => edit.move_to(usize::MAX, shift),
        _ => match shortcut(typed) {
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
    if creative.query != before {
        creative.scroll = 0.0;
        true
    } else {
        false
    }
}

#[derive(Default)]
pub(crate) struct CreativeInput {
    scrolling: bool,
    pressed_tab: bool,
    pub(crate) since: f64,
    pub(crate) display: usize,
}

pub(crate) struct CreativeFrame<'a> {
    pub mouse: Option<[f64; 2]>,
    pub pressed: [bool; 2],
    pub released: [bool; 2],
    pub held: [bool; 2],
    pub wheel: i32,
    pub typed: &'a [Typed],
    pub chat_key: bool,
    pub now: f64,
}

pub(crate) fn input(
    ui: &mut MinecraftUi,
    input: &mut CreativeInput,
    frame: CreativeFrame<'_>,
    font: Option<&McFont>,
    clipboard: &mut dyn ChatClipboard,
) -> [bool; 2] {
    let mut consumed = [false; 2];
    let Some(index) = ui.creative.selected_index() else {
        return consumed;
    };
    let tab = ui.creative.tabs[index].clone();
    let items = ui.creative.items.len();
    let scrollable = tab.scroll_bar && can_scroll(items);
    if let Some(rel) = frame.mouse {
        if frame.pressed[0] && tab_at(&ui.creative.tabs, rel).is_some() {
            input.pressed_tab = true;
            consumed[0] = true;
        } else if frame.pressed[0] && tab.scroll_bar && over_scrollbar(rel) {
            input.scrolling = scrollable;
            consumed[0] = true;
        }
        if frame.released[0] {
            if let Some(clicked) = tab_at(&ui.creative.tabs, rel) {
                select(&mut ui.creative, clicked);
                consumed[0] = true;
            }
            input.pressed_tab = false;
        }
        if input.scrolling && frame.held[0] {
            ui.creative.scroll = scroll_for_drag(rel[1]);
        }
    }
    if frame.released[0] || !frame.held[0] {
        input.scrolling = false;
    }
    if frame.wheel != 0 && scrollable {
        ui.creative.scroll = scroll_after_wheel(ui.creative.scroll, -frame.wheel as f32, items);
    }
    if tab.kind == McTabKind::Search {
        for typed in frame.typed {
            if type_search(&mut ui.creative, typed, clipboard) {
                input.since = frame.now;
            }
        }
        if let Some(font) = font {
            let pos = ui.creative.query[..ui.creative.cursor.min(ui.creative.query.len())]
                .chars()
                .count();
            input.display = view_start(font, &ui.creative.query, input.display, pos);
        }
    } else if frame.chat_key
        && let Some(search) = ui
            .creative
            .tabs
            .iter()
            .position(|tab| tab.kind == McTabKind::Search)
    {
        select(&mut ui.creative, search);
        input.since = frame.now;
    }
    if let Some(screen) = screen_of(&ui.creative) {
        ui.screen = screen;
    }
    consumed
}

fn draw_tab(
    p: &mut Painter<'_>,
    ui: &MinecraftUi,
    weapons: Option<&assets::PreparedWeapons>,
    origin: [i32; 2],
    tab: &McCreativeTab,
    selected: bool,
) {
    let x = origin[0] + tab_x(tab);
    let y = origin[1] + if tab.top { -28 } else { SIZE[1] - 4 };
    p.sprite(&tab_sprite(tab, selected), x, y, 26, 32);
    if let Some(icon) = tab.icon.as_ref() {
        let icon_y = y + 16 - 8 + if tab.top { 1 } else { -1 };
        p.item(ui, weapons, icon, x + 13 - 8, icon_y);
    }
}

pub(crate) fn draw_background(
    p: &mut Painter<'_>,
    ui: &MinecraftUi,
    weapons: Option<&assets::PreparedWeapons>,
    origin: [i32; 2],
    input: &CreativeInput,
    now: f64,
) {
    let selected = ui.creative.selected_index();
    for (index, tab) in ui.creative.tabs.iter().enumerate() {
        if Some(index) != selected {
            draw_tab(p, ui, weapons, origin, tab, false);
        }
    }
    let Some(tab) = ui.creative.selected_tab() else {
        return;
    };
    let [left, top] = origin;
    let [w, h] = SIZE;
    let sheet = format!("creative_inventory/tab_{}", tab.background);
    if tab.kind == McTabKind::Inventory {
        let [bx0, by0, bx1, by1] = PUPPET_BOX;
        p.blit(&sheet, left, top, 0, 0, w, by0);
        p.blit(&sheet, left, top + by1, 0, by1, w, h - by1);
        p.blit(&sheet, left, top + by0, 0, by0, bx0, by1 - by0);
        p.blit(&sheet, left + bx1, top + by0, bx1, by0, w - bx1, by1 - by0);
    } else {
        p.blit(&sheet, left, top, 0, 0, w, h);
    }
    if tab.kind == McTabKind::Search {
        draw_search(p, &ui.creative, origin, input, now);
    }
    if tab.scroll_bar {
        let sprite = if can_scroll(ui.creative.items.len()) {
            "container/creative_inventory/scroller"
        } else {
            "container/creative_inventory/scroller_disabled"
        };
        p.sprite(
            sprite,
            left + 175,
            top + scroller_y(ui.creative.scroll),
            12,
            15,
        );
    }
    draw_tab(p, ui, weapons, origin, tab, true);
}

fn draw_search(
    p: &mut Painter<'_>,
    creative: &McCreative,
    origin: [i32; 2],
    input: &CreativeInput,
    now: f64,
) {
    let [bx, by, width] = SEARCH_BOX;
    let (x, y) = (origin[0] + bx, origin[1] + by);
    let chars: Vec<char> = creative.query.chars().collect();
    let char_at = |byte: usize| {
        creative.query[..byte.min(creative.query.len())]
            .chars()
            .count()
    };
    let display = input.display.min(chars.len());
    let rest: String = chars[display..].iter().collect();
    let shown: Vec<char> = p.gui.font.head_by_width(&rest, width).chars().collect();
    let cursor = char_at(creative.cursor) as i64 - display as i64;
    let in_view = cursor >= 0 && cursor <= shown.len() as i64;
    let blink = ((now - input.since) * 1000.0 / BLINK_MILLIS) as i64 % 2 == 0 && in_view;
    let not_at_end = char_at(creative.cursor) < chars.len() || chars.len() >= MC_SEARCH_LIMIT;
    let head: String = if in_view {
        shown[..cursor as usize].iter().collect()
    } else {
        shown.iter().collect()
    };
    let mut end = x;
    if !shown.is_empty() {
        p.text(&head, x, y, WHITE, true);
        end = x + p.width(&head) + 1;
    }
    let mut caret = end;
    if !in_view {
        caret = if cursor > 0 { x + width } else { x };
    } else if not_at_end {
        caret = end - 1;
        end -= 1;
    }
    if !shown.is_empty() && in_view && (cursor as usize) < shown.len() {
        let tail: String = shown[cursor as usize..].iter().collect();
        p.text(&tail, end, y, WHITE, true);
    }
    let highlight =
        (char_at(creative.highlight) as i64 - display as i64).clamp(0, shown.len() as i64);
    if highlight != cursor {
        let upto: String = shown[..highlight as usize].iter().collect();
        let other = x + p.width(&upto);
        p.fill(
            caret.min(other),
            y - 1,
            caret.max(other) - 1,
            y + 1 + 9,
            HIGHLIGHT,
        );
    }
    if blink {
        if not_at_end {
            p.fill(caret, y - 1, caret + 1, y + 1 + 9, 0xFFD0_D0D0);
        } else {
            p.text("_", caret, y, WHITE, true);
        }
    }
}

pub(crate) fn draw_labels(p: &mut Painter<'_>, ui: &MinecraftUi, origin: [i32; 2]) {
    if let Some(tab) = ui.creative.selected_tab().filter(|tab| tab.show_title) {
        p.text(&tab.title, origin[0] + 8, origin[1] + 6, LABEL, false);
    }
}

pub(crate) fn tooltip(
    p: &Painter<'_>,
    ui: &MinecraftUi,
    rel: [f64; 2],
    hovered: Option<McSlot>,
) -> Option<Vec<TextLine>> {
    if let Some(tab) =
        tab_hovered(&ui.creative.tabs, rel).and_then(|index| ui.creative.tabs.get(index))
    {
        return Some(vec![TextLine::new(tab.title.clone(), WHITE)]);
    }
    (hovered == Some(McSlot::Trash))
        .then(|| vec![TextLine::new(p.gui.text("inventory.binSlot"), WHITE)])
}

pub(crate) fn extra_lines(ui: &MinecraftUi, slot: McSlot, stack: &McStack) -> Vec<TextLine> {
    let Some(tab) = ui.creative.selected_tab() else {
        return Vec::new();
    };
    let grid = match slot {
        McSlot::Creative(index) => Some(index),
        _ => None,
    };
    if grid.is_some() && tab.kind == McTabKind::Category {
        return Vec::new();
    }
    let mut lines: Vec<TextLine> = ui
        .creative
        .tabs_of
        .get(&stack.id)
        .into_iter()
        .flatten()
        .map(|title| TextLine::new(title.clone(), BLUE))
        .collect();
    if let Some(index) = grid.filter(|_| tab.kind == McTabKind::Search) {
        lines.extend(
            ui.creative
                .tags
                .get(index)
                .into_iter()
                .flatten()
                .map(|tag| TextLine::new(format!("#{tag}"), DARK_PURPLE)),
        );
    }
    lines
}

pub(crate) fn over_tab(ui: &MinecraftUi, rel: [f64; 2]) -> bool {
    tab_at(&ui.creative.tabs, rel).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::KeyCode;

    fn tab(id: &str, kind: McTabKind, top: bool, column: u8, aligned_right: bool) -> McCreativeTab {
        McCreativeTab {
            id: id.to_owned(),
            title: id.to_owned(),
            top,
            column,
            aligned_right,
            kind,
            show_title: true,
            scroll_bar: kind != McTabKind::Inventory,
            background: "items".to_owned(),
            icon: None,
        }
    }

    fn tabs() -> McCreative {
        McCreative {
            tabs: vec![
                tab(
                    "minecraft:building_blocks",
                    McTabKind::Category,
                    true,
                    0,
                    false,
                ),
                tab(
                    "minecraft:colored_blocks",
                    McTabKind::Category,
                    true,
                    1,
                    false,
                ),
                tab("minecraft:hotbar", McTabKind::Hotbar, true, 5, true),
                tab("minecraft:search", McTabKind::Search, true, 6, true),
                tab(
                    "minecraft:tools_and_utilities",
                    McTabKind::Category,
                    false,
                    0,
                    false,
                ),
                tab("minecraft:inventory", McTabKind::Inventory, false, 6, true),
            ],
            ..McCreative::default()
        }
    }

    fn at(layout: &Layout, slot: McSlot) -> Option<(i32, i32)> {
        layout.spec(slot).map(|spec| (spec.x, spec.y))
    }

    #[test]
    fn item_tabs_show_a_nine_by_five_grid_over_the_hotbar() {
        let layout = layout(McTabKind::Category, 18);
        assert_eq!(layout.size, [195, 136]);
        assert_eq!(layout.slots.len(), 54);
        assert_eq!(at(&layout, McSlot::Creative(18)), Some((9, 18)));
        assert_eq!(at(&layout, McSlot::Creative(26)), Some((153, 18)));
        assert_eq!(at(&layout, McSlot::Creative(62)), Some((153, 90)));
        assert_eq!(at(&layout, McSlot::Creative(17)), None);
        assert_eq!(at(&layout, McSlot::Inventory(0)), Some((9, 112)));
        assert_eq!(at(&layout, McSlot::Inventory(8)), Some((153, 112)));
        assert_eq!(at(&layout, McSlot::Inventory(9)), None);
        assert_eq!(at(&layout, McSlot::Trash), None);
        assert_eq!(layout.slot_at([9.0, 18.0]), Some(McSlot::Creative(18)));
    }

    #[test]
    fn the_inventory_tab_places_armor_offhand_and_trash_like_vanilla() {
        let layout = layout(McTabKind::Inventory, 0);
        assert_eq!(layout.slots.len(), 4 + 27 + 1 + 9 + 1);
        assert_eq!(at(&layout, McSlot::Inventory(39)), Some((54, 6)));
        assert_eq!(at(&layout, McSlot::Inventory(38)), Some((54, 33)));
        assert_eq!(at(&layout, McSlot::Inventory(37)), Some((108, 6)));
        assert_eq!(at(&layout, McSlot::Inventory(36)), Some((108, 33)));
        assert_eq!(at(&layout, McSlot::Inventory(40)), Some((35, 20)));
        assert_eq!(at(&layout, McSlot::Inventory(9)), Some((9, 54)));
        assert_eq!(at(&layout, McSlot::Inventory(35)), Some((153, 90)));
        assert_eq!(at(&layout, McSlot::Inventory(4)), Some((81, 112)));
        assert_eq!(at(&layout, McSlot::Trash), Some((173, 112)));
        assert_eq!(
            layout
                .spec(McSlot::Inventory(39))
                .and_then(|spec| spec.empty),
            Some("container/slot/helmet")
        );
        assert!(
            !layout
                .slots
                .iter()
                .any(|spec| matches!(spec.slot, McSlot::Creative(_)))
        );
    }

    #[test]
    fn tabs_sit_in_vanilla_columns_above_and_below_the_window() {
        let creative = tabs();
        let xs: Vec<(i32, i32)> = creative
            .tabs
            .iter()
            .map(|tab| (tab_x(tab), tab_y(tab)))
            .collect();
        assert_eq!(
            xs,
            [
                (0, -32),
                (27, -32),
                (142, -32),
                (169, -32),
                (0, 136),
                (169, 136)
            ]
        );
        assert_eq!(tab_at(&creative.tabs, [10.0, -20.0]), Some(0));
        assert_eq!(tab_at(&creative.tabs, [26.0, -1.0]), Some(0));
        assert_eq!(tab_at(&creative.tabs, [170.0, 150.0]), Some(5));
        assert_eq!(tab_at(&creative.tabs, [100.0, -20.0]), None);
        assert_eq!(tab_at(&creative.tabs, [10.0, 20.0]), None);
        assert_eq!(tab_hovered(&creative.tabs, [2.0, -29.0]), Some(0));
        assert_eq!(tab_hovered(&creative.tabs, [1.0, -29.0]), None);
        assert_eq!(
            tab_sprite(&creative.tabs[3], true),
            "container/creative_inventory/tab_top_selected_7"
        );
        assert_eq!(
            tab_sprite(&creative.tabs[4], false),
            "container/creative_inventory/tab_bottom_unselected_1"
        );
        assert!(
            over_scrollbar([175.0, 18.0])
                && !over_scrollbar([189.0, 18.0])
                && !over_scrollbar([180.0, 130.0])
        );
    }

    #[test]
    fn selecting_a_tab_clears_the_search_and_scroll_unless_it_is_search_again() {
        let mut creative = tabs();
        assert_eq!(creative.selected_index(), Some(0));
        select(&mut creative, 3);
        creative.query = "wool".to_owned();
        creative.cursor = 4;
        creative.scroll = 0.5;
        select(&mut creative, 3);
        assert_eq!((creative.query.as_str(), creative.scroll), ("wool", 0.0));
        select(&mut creative, 1);
        assert_eq!(
            creative.selected.as_deref(),
            Some("minecraft:colored_blocks")
        );
        assert!(creative.query.is_empty() && creative.cursor == 0);
        assert_eq!(
            screen_of(&creative),
            Some(McScreen::Creative(McTabKind::Category))
        );
        select(&mut creative, 5);
        assert_eq!(
            screen_of(&creative),
            Some(McScreen::Creative(McTabKind::Inventory))
        );
        select(&mut creative, 3);
        creative.query = "wool".to_owned();
        reopen(&mut creative);
        assert!(creative.query.is_empty());
        assert_eq!(creative.selected.as_deref(), Some("minecraft:search"));
    }

    fn stone() -> McStack {
        McStack {
            id: "minecraft:stone".to_owned(),
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
        }
    }

    #[test]
    fn tooltips_name_the_tabs_and_shown_tags_outside_the_item_tabs() {
        let mut ui = MinecraftUi {
            creative: tabs(),
            ..MinecraftUi::default()
        };
        ui.creative.tabs_of.insert(
            "minecraft:stone".to_owned(),
            vec!["Building Blocks".to_owned(), "Natural Blocks".to_owned()],
        );
        ui.creative.tags = vec![vec!["minecraft:stone_tool_materials".to_owned()]];
        let texts = |ui: &MinecraftUi, slot| {
            extra_lines(ui, slot, &stone())
                .into_iter()
                .map(|line| (line.text, line.color))
                .collect::<Vec<_>>()
        };
        assert!(texts(&ui, McSlot::Creative(0)).is_empty());
        assert_eq!(
            texts(&ui, McSlot::Inventory(0)),
            [
                ("Building Blocks".to_owned(), BLUE),
                ("Natural Blocks".to_owned(), BLUE)
            ]
        );
        select(&mut ui.creative, 3);
        assert_eq!(
            texts(&ui, McSlot::Creative(0)).last(),
            Some(&("#minecraft:stone_tool_materials".to_owned(), DARK_PURPLE))
        );
        assert_eq!(texts(&ui, McSlot::Inventory(0)).len(), 2);
    }

    fn typed(key: Key, code: KeyCode, text: Option<&str>, ctrl: bool) -> Typed {
        Typed {
            key,
            code,
            text: text.map(str::to_owned),
            mods: Mods {
                ctrl,
                shift: false,
                alt: false,
            },
        }
    }

    #[test]
    fn the_search_box_types_deletes_and_caps_at_fifty_characters() {
        let mut creative = tabs();
        select(&mut creative, 3);
        creative.scroll = 0.3;
        let letter = |c: &str| typed(Key::Character(c.into()), KeyCode::KeyA, Some(c), false);
        assert!(type_search(&mut creative, &letter("w"), &mut ()));
        assert_eq!(creative.scroll, 0.0);
        for c in ["o", "o", "l"] {
            type_search(&mut creative, &letter(c), &mut ());
        }
        assert_eq!(creative.query, "wool");
        assert!(type_search(
            &mut creative,
            &typed(Key::Backspace, KeyCode::Backspace, None, false),
            &mut ()
        ));
        assert_eq!(creative.query, "woo");
        assert!(!type_search(
            &mut creative,
            &typed(Key::ArrowLeft, KeyCode::ArrowLeft, None, false),
            &mut ()
        ));
        assert_eq!(creative.cursor, 2);
        type_search(
            &mut creative,
            &typed(Key::Character("a".into()), KeyCode::KeyA, Some("a"), true),
            &mut (),
        );
        type_search(&mut creative, &letter("#"), &mut ());
        assert_eq!(creative.query, "#");
        for _ in 0..60 {
            type_search(&mut creative, &letter("x"), &mut ());
        }
        assert_eq!(creative.query.chars().count(), MC_SEARCH_LIMIT);
    }
}
