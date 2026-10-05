//! The Minecraft map's hotbar and inventory, drawn as MW2 HUD chrome: MW2's
//! fonts, materials and colours over vanilla's slots. The inventory itself
//! (vanilla's container rules) lives with the world; this draws what it
//! publishes in `frame::MinecraftUi` and hands back the player's clicks.
use std::collections::HashMap;

use asset_core::{AssetNamespace, AssetRef};
use asset_game::{FontDef, MenuCatalog};
use assets::{PreparedLocalizedStrings, PreparedWeapons};
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use frame::minecraft_ui::MC_HOTBAR;
use frame::{McClick, McScreen, McSlot, McStack, MinecraftUi, PromptStyle};
use hud_iw4::{normalized_text_scale as r_normalized_text_scale, ui_text_height};

use crate::chrome::ui_text_width;
use crate::draw2d::{Draw2dCmd, Draw2dList, Draw2dOp, Draw2dProvenance, tessellate_fonts};
use crate::gaps::HudPresentationGaps;
use crate::gpu_list::{HudTessPass, TessJob};
use crate::images::{HUD_CHROME_NAMESPACE, HudImages};

/// The item icon atlas, drawn as a HUD material.
const ICONS: &str = "mc_item_icons";

const CENTER: i32 = 2;
const MAX: i32 = 3;

// MW2's theme: the rust orange of its menus and smoke, the pale green-white
// of its logo, on warm dark glass.
const PANEL: [f32; 4] = [0.046, 0.040, 0.034, 0.90];
const HEADER: [f32; 4] = [0.085, 0.068, 0.050, 0.97];
const EDGE: [f32; 4] = [1.0, 0.92, 0.82, 0.14];
const ACCENT: [f32; 4] = [0.91, 0.53, 0.18, 1.0];
const HIGHLIGHT: [f32; 4] = [0.86, 0.96, 0.82, 1.0];
const TEXT: [f32; 4] = [0.93, 0.93, 0.90, 1.0];
const TEXT_DIM: [f32; 4] = [0.66, 0.62, 0.56, 1.0];
const SLOT: [f32; 4] = [0.0, 0.0, 0.0, 0.48];
const SLOT_EDGE: [f32; 4] = [1.0, 0.90, 0.78, 0.10];
const SLOT_HOVER: [f32; 4] = [0.91, 0.53, 0.18, 0.20];
/// The inventory's size over its virtual layout.
const K: f32 = 1.3;

/// Slot size and pitch in virtual pixels.
const S: f32 = 20.0;
const P: f32 = 22.0;
/// The panel, about the screen's centre.
const PANEL_X: f32 = -118.0;
const PANEL_Y: f32 = -128.0;
const PANEL_W: f32 = 236.0;
const PANEL_H: f32 = 238.0;
const HEADER_H: f32 = 20.0;
const GRID_X: f32 = -98.0;
/// The character window.
const BOX: [f32; 4] = [-73.0, -100.0, 62.0, 86.0];
const TABLE_BOX: [f32; 4] = [-74.0, -93.0, 144.0, 78.0];
const TABLE_GRID: [f32; 2] = [-66.0, -86.0];
const TABLE_RESULT: [f32; 3] = [38.0, -66.0, 24.0];
const FURNACE_INPUT: [f32; 2] = [-50.0, -92.0];
const FURNACE_OUTPUT: [f32; 2] = [26.0, -72.0];
const XP_GREEN: [f32; 4] = [0.50, 1.0, 0.13, 1.0];
const REPEAT_DELAY: f64 = 0.4;

const CHOICE_CELL: f32 = 18.0;
const CHOICE_COLUMNS: usize = 4;
const CHOICE_ROWS: usize = 3;
const OFFER_BOX: [f32; 4] = [-36.0, -98.0, 132.0, 20.0];
const NAME_BOX: [f32; 4] = [-62.0, -98.0, 156.0, 14.0];

fn station_slots(screen: McScreen) -> Option<Vec<(McSlot, f32, f32, f32)>> {
    let c = McSlot::Container;
    Some(match screen {
        McScreen::Brewing => vec![
            (c(0), -40.0, -40.0, S),
            (c(1), -10.0, -32.0, S),
            (c(2), 20.0, -40.0, S),
            (c(3), -10.0, -96.0, S),
            (c(4), -82.0, -96.0, S),
        ],
        McScreen::Stonecutter => vec![(c(0), -88.0, -78.0, S), (McSlot::Output, 52.0, -80.0, 24.0)],
        McScreen::Smithing => vec![
            (c(0), -84.0, -62.0, S),
            (c(1), -60.0, -62.0, S),
            (c(2), -36.0, -62.0, S),
            (McSlot::Output, 40.0, -64.0, 24.0),
        ],
        McScreen::Grindstone => vec![(c(0), -50.0, -92.0, S), (c(1), -50.0, -58.0, S), (McSlot::Output, 40.0, -76.0, 24.0)],
        McScreen::Anvil => vec![(c(0), -84.0, -66.0, S), (c(1), -36.0, -66.0, S), (McSlot::Output, 40.0, -68.0, 24.0)],
        McScreen::Enchanting => vec![(c(0), -88.0, -80.0, S), (c(1), -64.0, -80.0, S)],
        _ => return None,
    })
}

fn choice_rects(screen: McScreen, count: usize, scroll: usize) -> Vec<(usize, [f32; 4])> {
    match screen {
        McScreen::Stonecutter => (0..CHOICE_COLUMNS * CHOICE_ROWS)
            .map(|cell| (scroll * CHOICE_COLUMNS + cell, cell))
            .filter(|(index, _)| *index < count)
            .map(|(index, cell)| {
                let (col, row) = ((cell % CHOICE_COLUMNS) as f32, (cell / CHOICE_COLUMNS) as f32);
                (index, [-54.0 + col * CHOICE_CELL, -96.0 + row * CHOICE_CELL, CHOICE_CELL, CHOICE_CELL])
            })
            .collect(),
        McScreen::Enchanting => (0..count.min(3))
            .map(|index| {
                let [x, y, w, h] = OFFER_BOX;
                (index, [x, y + index as f32 * (h + 2.0), w, h])
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn hovered_choice(surface: &crate::surface::Hud2dSurface, mouse: Vec2, screen: McScreen, count: usize, scroll: usize) -> Option<usize> {
    choice_rects(screen, count, scroll).into_iter().find_map(|(index, [x, y, w, h])| {
        let r = surface.apply_rect(x * K, y * K, w * K, h * K, CENTER, CENTER);
        (mouse.x >= r.x && mouse.x < r.x + r.w && mouse.y >= r.y && mouse.y < r.y + r.h).then_some(index)
    })
}

struct StationView {
    scroll: usize,
    focus: Option<usize>,
    hovered: Option<usize>,
    name: String,
}

fn left_name(ui: &MinecraftUi) -> String {
    ui.container.first().cloned().flatten().map_or_else(String::new, |left| {
        left.name.clone().or_else(|| ui.names.get(&left.id).cloned()).unwrap_or_else(|| left.id.clone())
    })
}

#[allow(clippy::too_many_arguments)]
fn station_input(
    ui: &mut MinecraftUi,
    input: &mut ScreenInput,
    surface: &crate::surface::Hud2dSurface,
    mouse: Option<Vec2>,
    clicked: bool,
    [previous, next, confirm]: [bool; 3],
    text: &str,
    backspace: bool,
    opened: bool,
) {
    let choices = ui.choices.len();
    let max_scroll = choices.div_ceil(CHOICE_COLUMNS).saturating_sub(CHOICE_ROWS);
    if clicked
        && let Some(index) = mouse.and_then(|m| hovered_choice(surface, m, ui.screen, choices, input.scroll))
        && ui.choices.get(index).is_some_and(|choice| choice.enabled)
    {
        ui.clicks.push(McClick::Choose(index));
    }
    if choices > 0 {
        let step = i32::from(next) - i32::from(previous);
        if step != 0 {
            input.choice_focus = (input.choice_focus as i32 + step).rem_euclid(choices as i32) as usize;
            if ui.screen == McScreen::Stonecutter {
                ui.clicks.push(McClick::Choose(input.choice_focus));
                let row = input.choice_focus / CHOICE_COLUMNS;
                input.scroll = input.scroll.clamp(row.saturating_sub(CHOICE_ROWS - 1), row).min(max_scroll);
            }
        }
        if confirm && ui.screen == McScreen::Enchanting {
            ui.clicks.push(McClick::Choose(input.choice_focus.min(choices - 1)));
        }
    }
    if ui.screen != McScreen::Anvil {
        input.name_for = None;
        return;
    }
    let left = ui.container.first().cloned().flatten().map(|stack| stack.id);
    if opened || left != input.name_for {
        input.name = ui.name.clone().unwrap_or_else(|| left_name(ui));
        input.name_for = left.clone();
    }
    if left.is_none() {
        return;
    }
    let mut changed = false;
    for c in text.chars() {
        if input.name.chars().count() < 50 {
            input.name.push(c);
            changed = true;
        }
    }
    if backspace && input.name.pop().is_some() {
        changed = true;
    }
    if changed {
        ui.clicks.push(McClick::Rename(input.name.clone()));
    }
}

fn chest_top(rows: u8) -> f32 {
    -86.0 - f32::from(rows.saturating_sub(3)) * P
}

fn panel_top(screen: McScreen) -> f32 {
    match screen {
        McScreen::Chest { rows } => PANEL_Y - f32::from(rows.saturating_sub(3)) * P,
        _ => PANEL_Y,
    }
}
const REPEAT_EVERY: f64 = 0.12;
const STICK: f32 = 0.6;

/// Where each slot of the screen sits (x, y, size).
fn layout(screen: McScreen) -> Vec<(McSlot, f32, f32, f32)> {
    let mut out = Vec::new();
    if screen == McScreen::CraftingTable {
        for i in 0..9 {
            out.push((McSlot::Workbench(i), TABLE_GRID[0] + (i % 3) as f32 * P, TABLE_GRID[1] + (i / 3) as f32 * P, S));
        }
        out.push((McSlot::WorkbenchResult, TABLE_RESULT[0], TABLE_RESULT[1], TABLE_RESULT[2]));
    } else if let McScreen::Chest { rows } = screen {
        let top = chest_top(rows);
        for i in 0..usize::from(rows) * 9 {
            out.push((McSlot::Container(i), GRID_X + (i % 9) as f32 * P, top + (i / 9) as f32 * P, S));
        }
    } else if screen == McScreen::Furnace {
        out.push((McSlot::Container(0), FURNACE_INPUT[0], FURNACE_INPUT[1], S));
        out.push((McSlot::Container(1), FURNACE_INPUT[0], FURNACE_INPUT[1] + 44.0, S));
        out.push((McSlot::Output, FURNACE_OUTPUT[0], FURNACE_OUTPUT[1], 24.0));
    } else if let Some(slots) = station_slots(screen) {
        out.extend(slots);
    } else {
        // Armor, head to feet down the left (slots 39..36).
        for (row, slot) in [39usize, 38, 37, 36].into_iter().enumerate() {
            out.push((McSlot::Inventory(slot), GRID_X, -100.0 + row as f32 * P, S));
        }
        // The offhand beside the character window.
        out.push((McSlot::Inventory(40), BOX[0] + BOX[2] + 3.0, -34.0, S));
        // The 2x2 crafting grid and its result.
        for i in 0..4 {
            out.push((McSlot::Crafting(i), 22.0 + (i % 2) as f32 * P, -86.0 + (i / 2) as f32 * P, S));
        }
        out.push((McSlot::Result, 88.0, -77.0, 24.0));
    }
    // The backpack, three rows of nine, then the hotbar.
    for row in 0..3 {
        for col in 0..9 {
            out.push((McSlot::Inventory(9 + row * 9 + col), GRID_X + col as f32 * P, 6.0 + row as f32 * P, S));
        }
    }
    for col in 0..9 {
        out.push((McSlot::Inventory(col), GRID_X + col as f32 * P, 80.0, S));
    }
    out
}

#[derive(Component)]
pub(crate) struct MinecraftRaster;

/// The input state the screen keeps between frames.
#[derive(Default)]
pub(crate) struct ScreenInput {
    last_click: Option<(McSlot, f64)>,
    /// A drag of the carried stack: the button and the slots crossed.
    drag: Option<(bool, Vec<McSlot>)>,
    open: bool,
    screen: McScreen,
    focus: Option<McSlot>,
    repeat: Option<(Nav, f64)>,
    pad_pointer: bool,
    name: String,
    name_for: Option<String>,
    scroll: usize,
    choice_focus: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nav {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Default)]
struct Buttons {
    pressed: [bool; 2],
    released: [bool; 2],
    quick: bool,
}

struct Canvas<'a> {
    surface: &'a crate::surface::Hud2dSurface,
    cmds: Vec<Draw2dCmd>,
    fonts: HashMap<String, &'a FontDef>,
    /// The scale virtual coordinates are drawn at.
    k: f32,
}

impl Canvas<'_> {
    fn quad(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4], material: &str, st: [f32; 4], align: (i32, i32)) {
        self.quad_ns(x, y, w, h, color, material, HUD_CHROME_NAMESPACE, st, align);
    }

    #[allow(clippy::too_many_arguments)]
    fn quad_ns(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        material: &str,
        namespace: AssetNamespace,
        st: [f32; 4],
        align: (i32, i32),
    ) {
        let k = self.k;
        let r = self.surface.apply_rect(x * k, y * k, w * k, h * k, align.0, align.1);
        self.cmds.push(Draw2dCmd {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
            s0: st[0],
            t0: st[1],
            s1: st[2],
            t1: st[3],
            color,
            material: material.to_owned(),
            material_namespace: namespace,
            op: Draw2dOp::StretchPic,
            provenance: Draw2dProvenance::CgDraw { site: "minecraft_inventory" },
            layer: 1,
        });
    }

    fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4], align: (i32, i32)) {
        self.quad(x, y, w, h, color, "white", [0.0, 0.0, 1.0, 1.0], align);
    }

    /// A hairline frame.
    fn frame(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4], align: (i32, i32)) {
        let t = 0.5;
        self.fill(x, y, w, t, color, align);
        self.fill(x, y + h - t, w, t, color, align);
        self.fill(x, y, t, h, color, align);
        self.fill(x + w - t, y, t, h, color, align);
    }

    /// MW2's corner brackets.
    fn brackets(&mut self, x: f32, y: f32, w: f32, h: f32, len: f32, color: [f32; 4], align: (i32, i32)) {
        let t = 1.0;
        for (cx, cy, dx, dy) in [(x, y, 1.0, 1.0), (x + w, y, -1.0, 1.0), (x, y + h, 1.0, -1.0), (x + w, y + h, -1.0, -1.0)] {
            let hx = if dx > 0.0 { cx } else { cx - len };
            let vy = if dy > 0.0 { cy } else { cy - len };
            self.fill(hx, if dy > 0.0 { cy } else { cy - t }, len, t, color, align);
            self.fill(if dx > 0.0 { cx } else { cx - t }, vy, t, len, color, align);
        }
    }

    /// Text in a font `px` virtual pixels tall, its top-left at (x, y);
    /// `right` aligns its right edge to x instead. Returns its width.
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, font: &str, x: f32, y: f32, px: f32, color: [f32; 4], text: &str, right: bool, align: (i32, i32)) -> f32 {
        let Some(def) = self.fonts.get(font).copied() else {
            return 0.0;
        };
        let text_scale = px / ui_text_height(1.0);
        let width = ui_text_width(def, text, text_scale);
        let x = if right { x - width } else { x };
        let k = self.k;
        let nscale = r_normalized_text_scale(def.pixel_height, text_scale * k);
        let r = self.surface.apply_rect(x * k, (y + px) * k, nscale, nscale, align.0, align.1);
        self.cmds.push(Draw2dCmd {
            x: (r.x + 0.5).floor(),
            y: (r.y + 0.5).floor(),
            w: r.w,
            h: r.h,
            s0: 0.0,
            t0: 0.0,
            s1: 1.0,
            t1: 1.0,
            color,
            material: AssetRef::bare_name(&def.material).to_owned(),
            material_namespace: HUD_CHROME_NAMESPACE,
            op: Draw2dOp::TextRun {
                font: font.to_owned(),
                scale: nscale,
                text: text.to_owned(),
                loc_key: String::new(),
                style: crate::draw2d::TEXT_STYLE_HUDELEM,
                fx: None,
                glow: None,
            },
            provenance: Draw2dProvenance::CgDraw { site: "minecraft_inventory" },
            layer: 1,
        });
        width
    }
}

const FONT_TITLE: &str = "fonts/objectivefont";
const FONT_SMALL: &str = "fonts/smallfont";
const FONT_COUNT: &str = "fonts/hudsmallfont";

/// A slot's item: its icon, a gun's MW2 kill icon, and its count.
fn draw_stack(
    canvas: &mut Canvas<'_>,
    ui: &MinecraftUi,
    weapons: Option<&PreparedWeapons>,
    stack: &McStack,
    x: f32,
    y: f32,
    size: f32,
    align: (i32, i32),
) {
    let inset = size * 0.1;
    if let Some(weapon) = stack.weapon {
        let Some(reg) = weapons else { return };
        let namespace = reg.0.namespace_of(weapon).unwrap_or(HUD_CHROME_NAMESPACE);
        let Some(icon) = reg.0.kill_icon_image_of(weapon).or_else(|| reg.0.kill_icon_of(weapon)) else {
            return;
        };
        let facts = reg.0.facts_of(weapon);
        let ratio = match facts.map(|f| f.kill_icon_ratio) {
            Some(1) => 2.0,
            Some(2) => 4.0,
            _ => 1.0,
        };
        let flip = facts.is_some_and(|f| f.flip_kill_icon);
        // Guns sit across the slot, as wide as it allows.
        let w = size * 1.18;
        let h = (w / ratio).min(size - inset);
        let st = if flip { [1.0, 0.0, 0.0, 1.0] } else { [0.0, 0.0, 1.0, 1.0] };
        canvas.quad_ns(x + (size - w) * 0.5, y + (size - h) * 0.5, w, h, TEXT, icon, namespace, st, align);
        return;
    }
    if let Some(rect) = ui.icon_rects.get(&stack.id) {
        canvas.quad(x + inset, y + inset, size - inset * 2.0, size - inset * 2.0, [1.0; 4], ICONS, *rect, align);
    }
    if stack.count > 1 {
        let label = stack.count.to_string();
        canvas.text(FONT_COUNT, x + size - 1.0, y + size - 8.5, 7.5, TEXT, &label, true, align);
    }
}

fn stack_at<'a>(ui: &'a MinecraftUi, slot: McSlot) -> Option<&'a McStack> {
    match slot {
        McSlot::Inventory(i) => ui.slots.get(i)?.as_ref(),
        McSlot::Crafting(i) => ui.crafting.get(i)?.as_ref(),
        McSlot::Result => ui.result.as_ref(),
        McSlot::Workbench(i) => ui.workbench.get(i)?.as_ref(),
        McSlot::WorkbenchResult => ui.workbench_result.as_ref(),
        McSlot::Container(i) => ui.container.get(i)?.as_ref(),
        McSlot::Output => ui.output.as_ref(),
    }
}

fn is_result(slot: McSlot) -> bool {
    matches!(slot, McSlot::Result | McSlot::WorkbenchResult | McSlot::Output)
}

fn stack_name(
    stack: &McStack,
    ui: &MinecraftUi,
    weapons: Option<&PreparedWeapons>,
    strings: Option<&PreparedLocalizedStrings>,
    gaps: &mut HudPresentationGaps,
) -> String {
    match stack.weapon {
        Some(weapon) => crate::weapon_name::localized_weapon_name(weapon, weapons, strings, gaps)
            .unwrap_or_else(|| "Weapon".to_owned()),
        None => stack.name.clone().or_else(|| ui.names.get(&stack.id).cloned()).unwrap_or_else(|| stack.id.clone()),
    }
}

type Typing<'w, 's> = (
    Res<'w, ButtonInput<KeyCode>>,
    Res<'w, ButtonInput<MouseButton>>,
    MessageReader<'w, 's, MouseWheel>,
    MessageReader<'w, 's, bevy::input::keyboard::KeyboardInput>,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_minecraft_hud(
    surface: Res<crate::surface::Hud2dSurface>,
    catalog: Option<Res<MenuCatalog>>,
    ui: Option<ResMut<MinecraftUi>>,
    (weapons, strings): (Option<Res<PreparedWeapons>>, Option<Res<PreparedLocalizedStrings>>),
    (keys, buttons, mut wheel, mut typed): Typing,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    time: Res<Time>,
    mut input: Local<ScreenInput>,
    (mut hud_images, mut images): (ResMut<HudImages>, ResMut<Assets<Image>>),
    mut gaps: ResMut<HudPresentationGaps>,
    mut pass: ResMut<HudTessPass>,
    (gamepads, active_pad, devices): (Query<&Gamepad>, Option<Res<frame::ActivePad>>, Option<Res<frame::InputDevices>>),
    (menus, hud_input, mut settings): (
        Res<crate::menus::ScriptMenus>,
        Option<Res<frame::HudInputView>>,
        Option<ResMut<frame::GameSettings>>,
    ),
) {
    let Some(mut ui) = ui else {
        pass.minecraft = TessJob::Hide;
        return;
    };
    let opened_by_world = std::mem::take(&mut ui.screen_opened);
    if !ui.active || !surface.is_ready() {
        pass.minecraft = TessJob::Hide;
        ui.character_box = None;
        for _ in wheel.read() {}
        input.open = false;
        input.drag = None;
        return;
    }
    if let Some(icons) = ui.icons.clone() {
        hud_images.insert_runtime(ICONS, icons);
    }
    let now = time.elapsed_secs_f64();
    let window = windows.single().ok();
    let mouse = window.and_then(Window::cursor_position);
    let blocked = menus.captures_input() || hud_input.as_ref().is_some_and(|input| input.console_open);
    let pad = active_pad
        .as_deref()
        .and_then(|active| active.0)
        .and_then(|entity| gamepads.get(entity).ok())
        .filter(|_| !blocked && devices.as_ref().is_none_or(|devices| devices.focused));
    let prompts = devices.as_ref().filter(|devices| devices.pad_prompts && pad.is_some()).map(|devices| devices.style);
    let pad_pointer = prompts.is_some();
    let held = pad.and_then(pad_direction);
    let pad_just = |button: GamepadButton| pad.is_some_and(|pad| pad.just_pressed(button));

    // Opening and closing, and the hotbar's keys.
    let open_before = ui.inventory_open;
    let typing = ui.inventory_open && ui.screen == McScreen::Anvil;
    let text: String = typed.read().filter(|event| event.state.is_pressed()).filter_map(|event| event.text.as_ref()).flat_map(|text| text.chars()).filter(|c| !c.is_control()).collect();
    if !blocked && ((keys.just_pressed(KeyCode::KeyE) && !typing) || (ui.inventory_open && keys.just_pressed(KeyCode::Escape))) {
        ui.inventory_open = !ui.inventory_open && !keys.just_pressed(KeyCode::Escape);
    } else if ui.inventory_open && (menus.captures_input() || pad_just(GamepadButton::East)) {
        ui.inventory_open = false;
    } else if !ui.inventory_open && pad_just(GamepadButton::DPadUp) {
        ui.inventory_open = true;
    }
    if ui.inventory_open && !open_before {
        ui.screen = McScreen::Inventory;
    }
    if open_before && !ui.inventory_open {
        ui.clicks.push(McClick::Close);
    }
    let opened = ui.inventory_open && (!input.open || input.screen != ui.screen || opened_by_world);
    if ui.inventory_open != input.open || ui.screen != input.screen {
        input.drag = None;
        input.last_click = None;
    }
    input.open = ui.inventory_open;
    input.screen = ui.screen;
    let digits = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    let digit = digits.iter().position(|k| keys.just_pressed(*k)).filter(|_| !typing);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    let fonts: HashMap<String, &FontDef> = catalog
        .as_deref()
        .map(|catalog| {
            [FONT_TITLE, FONT_SMALL, FONT_COUNT]
                .into_iter()
                .filter_map(|name| catalog.font(name).map(|font| (name.to_owned(), font)))
                .collect()
        })
        .unwrap_or_default();
    let mut canvas = Canvas { surface: &surface, cmds: Vec::new(), fonts, k: 1.0 };
    let weapons = weapons.as_deref();
    let strings = strings.as_deref();

    if !ui.inventory_open {
        // The hotbar: scroll and number keys pick, Q throws.
        let mut scroll = 0i32;
        for event in wheel.read() {
            scroll += if event.y < 0.0 { 1 } else if event.y > 0.0 { -1 } else { 0 };
        }
        if scroll != 0 {
            ui.select = Some((ui.selected as i32 + scroll).rem_euclid(9) as usize);
        }
        if let Some(digit) = digit {
            ui.select = Some(digit);
        }
        if keys.just_pressed(KeyCode::KeyQ) {
            ui.drop_selected = Some(ctrl);
        }
        if !blocked
            && keys.just_pressed(KeyCode::KeyM)
            && let Some(settings) = settings.as_mut()
        {
            settings.minecraft_minimap = !settings.minecraft_minimap;
            settings.touch();
        }
        canvas.k = 1.15;
        draw_hotbar(&mut canvas, &mut ui, weapons, strings, &mut gaps, time.delta_secs());
        ui.character_box = None;
    } else {
        let mut scroll_step = 0i32;
        for event in wheel.read() {
            scroll_step += if event.y < 0.0 { 1 } else if event.y > 0.0 { -1 } else { 0 };
        }
        let choices = ui.choices.len();
        let max_scroll = choices.div_ceil(CHOICE_COLUMNS).saturating_sub(CHOICE_ROWS);
        if opened {
            input.scroll = 0;
            input.choice_focus = 0;
        }
        input.scroll = (input.scroll as i32 + scroll_step).clamp(0, max_scroll as i32) as usize;
        let slots = layout(ui.screen);
        let mouse_hovered = mouse.and_then(|m| hovered_slot(&surface, m, ui.screen));
        if opened {
            input.focus = None;
            input.repeat = held.map(|nav| (nav, f64::INFINITY));
        }
        if pad_pointer && !input.pad_pointer {
            input.focus = mouse_hovered.or(input.focus);
        }
        input.pad_pointer = pad_pointer;
        if !input.focus.is_some_and(|focus| slots.iter().any(|slot| slot.0 == focus)) {
            input.focus = Some(McSlot::Inventory(ui.selected.min(MC_HOTBAR - 1)));
        }
        if let Some(nav) = repeat_step(&mut input.repeat, held, now)
            && let Some(from) = input.focus
        {
            input.focus = neighbour(&slots, from, nav).or(Some(from));
        }
        let hovered = if pad_pointer { input.focus } else { mouse_hovered };
        let inside = pad_pointer || mouse.is_some_and(|m| over_panel(&surface, m, ui.screen));
        if opened || blocked {
            input.drag = None;
        } else {
            handle_clicks(&mut ui, &mut input, screen_buttons(&buttons, pad), hovered, inside, shift, digit, now);
            station_input(&mut ui, &mut input, &surface, mouse.filter(|_| !pad_pointer), buttons.just_pressed(MouseButton::Left), [pad_just(GamepadButton::LeftTrigger), pad_just(GamepadButton::RightTrigger), pad_just(GamepadButton::RightTrigger2)], &text, keys.just_pressed(KeyCode::Backspace), opened);
        }
        let pointer = if pad_pointer {
            hovered.and_then(|focus| slots.iter().find(|slot| slot.0 == focus)).map(|&(_, x, y, size)| {
                let r = surface.apply_rect((x + size) * K, y * K, 0.0, 0.0, CENTER, CENTER);
                Vec2::new(r.x, r.y)
            })
        } else {
            mouse
        };
        canvas.k = K;
        let view = StationView {
            scroll: input.scroll,
            focus: pad_pointer.then_some(input.choice_focus),
            hovered: mouse.filter(|_| !pad_pointer).and_then(|m| hovered_choice(&surface, m, ui.screen, choices, input.scroll)),
            name: input.name.clone(),
        };
        draw_inventory(&mut canvas, &mut ui, weapons, strings, &mut gaps, hovered, pointer, prompts, &view);
        if ui.screen != McScreen::Inventory {
            ui.character_box = None;
        } else {
            // Where the character stands, and the mouse it follows.
            let b = surface.apply_rect(BOX[0] * K, BOX[1] * K, BOX[2] * K, BOX[3] * K, CENTER, CENTER);
            let centre = [b.x + b.w * 0.5, b.y + b.h * 0.36];
            ui.character_box = Some([b.x + b.w * 0.5, b.y + b.h * 0.5, b.w, b.h]);
            if let Some(m) = pointer {
                ui.gaze = [(m.x - centre[0]) / (b.h * 0.5), (m.y - centre[1]) / (b.h * 0.5)];
            }
        }
    }

    let list = Draw2dList { cmds: canvas.cmds };
    let fonts = canvas.fonts;
    let (quads, _) = tessellate_fonts(&list, &fonts);
    for font in fonts.values() {
        let _ = hud_images.get(HUD_CHROME_NAMESPACE, AssetRef::bare_name(&font.material), &mut images);
    }
    pass.minecraft = if quads.is_empty() { TessJob::Hide } else { TessJob::Quads(quads) };
}

fn hovered_slot(surface: &crate::surface::Hud2dSurface, mouse: Vec2, screen: McScreen) -> Option<McSlot> {
    layout(screen).into_iter().find_map(|(slot, x, y, size)| {
        let r = surface.apply_rect(x * K, y * K, size * K, size * K, CENTER, CENTER);
        (mouse.x >= r.x && mouse.x < r.x + r.w && mouse.y >= r.y && mouse.y < r.y + r.h).then_some(slot)
    })
}

fn neighbour(slots: &[(McSlot, f32, f32, f32)], from: McSlot, nav: Nav) -> Option<McSlot> {
    let centre = |&(_, x, y, size): &(McSlot, f32, f32, f32)| Vec2::new(x + size * 0.5, y + size * 0.5);
    let origin = centre(slots.iter().find(|slot| slot.0 == from)?);
    let dir = match nav {
        Nav::Up => Vec2::NEG_Y,
        Nav::Down => Vec2::Y,
        Nav::Left => Vec2::NEG_X,
        Nav::Right => Vec2::X,
    };
    slots
        .iter()
        .filter(|slot| slot.0 != from)
        .filter_map(|slot| {
            let delta = centre(slot) - origin;
            let along = delta.dot(dir);
            let across = delta.perp_dot(dir).abs();
            (along > 1.0 && across <= along * 1.5).then_some((along + 2.0 * across, slot.0))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, slot)| slot)
}

fn pad_direction(pad: &Gamepad) -> Option<Nav> {
    let dpad = [
        (GamepadButton::DPadUp, Nav::Up),
        (GamepadButton::DPadDown, Nav::Down),
        (GamepadButton::DPadLeft, Nav::Left),
        (GamepadButton::DPadRight, Nav::Right),
    ];
    if let Some(&(_, nav)) = dpad.iter().find(|(button, _)| pad.pressed(*button)) {
        return Some(nav);
    }
    let stick = pad.left_stick();
    if stick.x.abs().max(stick.y.abs()) < STICK {
        return None;
    }
    Some(if stick.x.abs() > stick.y.abs() {
        if stick.x > 0.0 { Nav::Right } else { Nav::Left }
    } else if stick.y > 0.0 {
        Nav::Up
    } else {
        Nav::Down
    })
}

fn repeat_step(repeat: &mut Option<(Nav, f64)>, held: Option<Nav>, now: f64) -> Option<Nav> {
    let Some(nav) = held else {
        *repeat = None;
        return None;
    };
    match *repeat {
        Some((last, next)) if last == nav => {
            if now < next {
                return None;
            }
            *repeat = Some((nav, now + REPEAT_EVERY));
        }
        _ => *repeat = Some((nav, now + REPEAT_DELAY)),
    }
    Some(nav)
}

fn screen_buttons(mouse: &ButtonInput<MouseButton>, pad: Option<&Gamepad>) -> Buttons {
    let pressed = |button: GamepadButton| pad.is_some_and(|pad| pad.just_pressed(button));
    let released = |button: GamepadButton| pad.is_some_and(|pad| pad.just_released(button));
    Buttons {
        pressed: [
            mouse.just_pressed(MouseButton::Left) || pressed(GamepadButton::South),
            mouse.just_pressed(MouseButton::Right) || pressed(GamepadButton::West),
        ],
        released: [
            mouse.just_released(MouseButton::Left) || released(GamepadButton::South),
            mouse.just_released(MouseButton::Right) || released(GamepadButton::West),
        ],
        quick: pressed(GamepadButton::North),
    }
}

fn pad_label(style: PromptStyle, button: GamepadButton) -> &'static str {
    match (style, button) {
        (PromptStyle::Xbox, GamepadButton::South) => "A",
        (PromptStyle::Xbox, GamepadButton::East) => "B",
        (PromptStyle::Xbox, GamepadButton::West) => "X",
        (PromptStyle::Xbox, GamepadButton::North) => "Y",
        (PromptStyle::PlayStation, GamepadButton::South) => "×",
        (PromptStyle::PlayStation, GamepadButton::East) => "○",
        (PromptStyle::PlayStation, GamepadButton::West) => "□",
        (PromptStyle::PlayStation, GamepadButton::North) => "△",
        (PromptStyle::Generic, GamepadButton::South) => "SOUTH",
        (PromptStyle::Generic, GamepadButton::East) => "EAST",
        (PromptStyle::Generic, GamepadButton::West) => "WEST",
        (PromptStyle::Generic, GamepadButton::North) => "NORTH",
        _ => "",
    }
}

fn draw_pad_legend(canvas: &mut Canvas<'_>, style: PromptStyle, carrying: bool, on_result: bool) {
    let c = (CENTER, CENTER);
    let px = 5.5;
    let entries = [
        (GamepadButton::South, if carrying && !on_result { "PLACE" } else { "TAKE" }),
        (GamepadButton::West, if carrying { "PLACE ONE" } else { "SPLIT" }),
        (GamepadButton::North, if on_result { "CRAFT ALL" } else { "QUICK MOVE" }),
        (GamepadButton::East, "CLOSE"),
    ];
    let width = |canvas: &Canvas<'_>, text: &str| {
        canvas.fonts.get(FONT_SMALL).map_or(0.0, |def| ui_text_width(def, text, px / ui_text_height(1.0)))
    };
    let total = entries
        .iter()
        .map(|&(button, action)| width(canvas, pad_label(style, button)) + 3.0 + width(canvas, action) + 10.0)
        .sum::<f32>()
        + 6.0;
    let (x, y) = (-total * 0.5, PANEL_Y + PANEL_H + 6.0);
    canvas.fill(x, y, total, 12.0, [0.02, 0.025, 0.028, 0.90], c);
    canvas.frame(x, y, total, 12.0, EDGE, c);
    canvas.fill(x, y, 2.0, 12.0, ACCENT, c);
    let mut at = x + 8.0;
    for (button, action) in entries {
        at += canvas.text(FONT_SMALL, at, y + 3.25, px, ACCENT, pad_label(style, button), false, c) + 3.0;
        at += canvas.text(FONT_SMALL, at, y + 3.25, px, TEXT, action, false, c) + 10.0;
    }
}

fn over_panel(surface: &crate::surface::Hud2dSurface, mouse: Vec2, screen: McScreen) -> bool {
    let top = panel_top(screen);
    let r = surface.apply_rect(PANEL_X * K, top * K, PANEL_W * K, (PANEL_H + PANEL_Y - top) * K, CENTER, CENTER);
    mouse.x >= r.x && mouse.x < r.x + r.w && mouse.y >= r.y && mouse.y < r.y + r.h
}

/// Vanilla's container input: a click picks up, puts down or swaps; with
/// shift it moves the stack across; a double click gathers; dragging a
/// carried stack over slots shares it out; a number key swaps with the
/// hotbar; a click outside throws.
#[allow(clippy::too_many_arguments)]
fn handle_clicks(
    ui: &mut MinecraftUi,
    input: &mut ScreenInput,
    buttons: Buttons,
    hovered: Option<McSlot>,
    inside: bool,
    shift: bool,
    digit: Option<usize>,
    now: f64,
) {
    if let (Some(digit), Some(slot)) = (digit, hovered) {
        ui.clicks.push(McClick::Swap { slot, hotbar: digit });
    }
    // A drag keeps collecting the slots it crosses.
    if let Some((_, slots)) = input.drag.as_mut()
        && let Some(slot) = hovered
        && !is_result(slot)
        && !slots.contains(&slot)
    {
        slots.push(slot);
    }
    if buttons.quick
        && input.drag.is_none()
        && let Some(slot) = hovered
    {
        ui.clicks.push(McClick::Slot { slot, right: false, shift: true });
        input.last_click = None;
    }
    for (button, right) in [(0, false), (1, true)] {
        if buttons.pressed[button] {
            let Some(slot) = hovered else {
                if !inside {
                    ui.clicks.push(McClick::Outside { right });
                }
                continue;
            };
            if !right
                && !shift
                && !is_result(slot)
                && input.last_click.is_some_and(|(last, at)| last == slot && now - at < 0.25)
                && ui.cursor.is_some()
            {
                ui.clicks.push(McClick::Gather { slot, right: false });
                input.last_click = None;
                continue;
            }
            input.last_click = Some((slot, now));
            match slot {
                McSlot::Inventory(_) | McSlot::Crafting(_) | McSlot::Workbench(_) | McSlot::Container(_) if ui.cursor.is_some() && !shift => {
                    input.drag = Some((right, vec![slot]));
                }
                _ => ui.clicks.push(McClick::Slot { slot, right, shift }),
            }
        }
        if buttons.released[button]
            && let Some((drag_right, slots)) = input.drag.take()
        {
            if drag_right != right {
                input.drag = Some((drag_right, slots));
                continue;
            }
            if slots.len() > 1 {
                ui.clicks.push(McClick::Spread { slots, right });
            } else if let Some(&slot) = slots.first() {
                ui.clicks.push(McClick::Slot { slot, right, shift: false });
            }
        }
    }
}

/// The hotbar along the bottom of the screen, between MW2's score and ammo.
fn draw_hotbar(
    canvas: &mut Canvas<'_>,
    ui: &mut MinecraftUi,
    weapons: Option<&PreparedWeapons>,
    strings: Option<&PreparedLocalizedStrings>,
    gaps: &mut HudPresentationGaps,
    dt: f32,
) {
    let align = (CENTER, MAX);
    let size = 22.0;
    let pitch = 25.0;
    let width = pitch * 9.0 - (pitch - size);
    let x0 = -width * 0.5;
    let y = -size - 3.0;
    // The strip behind it.
    canvas.fill(x0 - 3.0, y - 3.0, width + 6.0, size + 6.0, [0.04, 0.03, 0.02, 0.50], align);
    canvas.quad(x0 - 3.0, y - 3.0, width + 6.0, size + 6.0, [ACCENT[0], ACCENT[1], ACCENT[2], 0.10], "gradient_fadein_fadebottom", [0.0, 0.0, 1.0, 1.0], align);
    canvas.frame(x0 - 3.0, y - 3.0, width + 6.0, size + 6.0, EDGE, align);
    for i in 0..9 {
        let selected = i == ui.selected;
        let x = x0 + i as f32 * pitch;
        let lift = if selected { -2.0 } else { 0.0 };
        if selected {
            canvas.quad(x - 7.0, y - 9.0 + lift, size + 14.0, size + 16.0, [ACCENT[0], ACCENT[1], ACCENT[2], 0.55], "mockup_bg_glow", [0.0, 0.0, 1.0, 1.0], align);
        }
        canvas.fill(x, y + lift, size, size, if selected { [0.16, 0.09, 0.03, 0.80] } else { SLOT }, align);
        canvas.frame(x, y + lift, size, size, if selected { HIGHLIGHT } else { SLOT_EDGE }, align);
        if selected {
            canvas.fill(x, y + lift + size - 1.5, size, 1.5, ACCENT, align);
        }
        canvas.text(FONT_SMALL, x + 1.5, y + lift + 0.5, 5.5, if selected { HIGHLIGHT } else { TEXT_DIM }, &(i + 1).to_string(), false, align);
        if let Some(stack) = ui.slots.get(i).and_then(Option::as_ref).cloned() {
            draw_stack(canvas, ui, weapons, &stack, x, y + lift, size, align);
        }
    }
    // The selected item's name, as vanilla shows it over the hotbar.
    let name = ui.slots.get(ui.selected).and_then(Option::as_ref).map(|stack| {
        stack_name(stack, ui, weapons, strings, gaps)
    });
    if let Some((shown, age)) = ui.selected_name.as_mut() {
        *age += dt;
        let fade = (1.0 - ((*age - 1.6) / 0.5).clamp(0.0, 1.0)).clamp(0.0, 1.0);
        if fade > 0.0 {
            let text = name.as_deref().unwrap_or(shown).to_uppercase();
            let px = 9.0;
            let w = canvas.fonts.get(FONT_TITLE).map_or(0.0, |def| {
                ui_text_width(def, &text, px / ui_text_height(1.0))
            });
            canvas.text(FONT_TITLE, -w * 0.5, vitals_rows(y).2, px, [HIGHLIGHT[0], HIGHLIGHT[1], HIGHLIGHT[2], fade], &text, false, align);
        }
    }
    draw_vitals(canvas, ui, x0, width, y, align);
}

const VITALS_ICON: f32 = 11.0;
const VITALS_PITCH: f32 = 9.75;

fn vitals_rows(hotbar_y: f32) -> (f32, f32, f32) {
    let first = hotbar_y - 5.0 - VITALS_ICON;
    let second = first - VITALS_ICON - 2.0;
    (first, second, second - 13.0)
}

fn vitals_backing(canvas: &mut Canvas<'_>, x: f32, y: f32, w: f32, align: (i32, i32)) {
    canvas.fill(x - 2.0, y - 1.5, w + 4.0, VITALS_ICON + 3.0, [0.04, 0.03, 0.02, 0.45], align);
    canvas.frame(x - 2.0, y - 1.5, w + 4.0, VITALS_ICON + 3.0, EDGE, align);
}

fn draw_vitals(canvas: &mut Canvas<'_>, ui: &MinecraftUi, x0: f32, width: f32, hotbar_y: f32, align: (i32, i32)) {
    let Some(vitals) = ui.vitals.as_ref() else {
        return;
    };
    let (first, second, _) = vitals_rows(hotbar_y);
    let row_width = VITALS_PITCH * 9.0 + VITALS_ICON;
    let right = x0 + width;
    canvas.fill(x0 - 3.0, hotbar_y - 4.5, width + 6.0, 1.5, [0.0, 0.0, 0.0, 0.55], align);
    let filled = vitals.experience_progress.clamp(0.0, 1.0) * (width + 6.0);
    if filled > 0.0 {
        canvas.fill(x0 - 3.0, hotbar_y - 4.5, filled, 1.5, XP_GREEN, align);
    }
    if vitals.experience_level > 0 {
        let label = vitals.experience_level.to_string();
        let px = 7.5;
        let w = canvas.fonts.get(FONT_COUNT).map_or(0.0, |def| ui_text_width(def, &label, px / ui_text_height(1.0)));
        canvas.text(FONT_COUNT, x0 + width * 0.5 - w * 0.5, first + 1.5, px, XP_GREEN, &label, false, align);
    }
    let px = VITALS_ICON / 9.0;
    let (rows, row_height) = crate::minecraft_vitals::heart_rows(vitals);
    let stacked = (rows.saturating_sub(1)) as f32 * row_height * px;
    vitals_backing(canvas, x0, first - stacked, row_width, align);
    if stacked > 0.0 {
        canvas.fill(x0 - 2.0, first - stacked + VITALS_ICON + 1.5, row_width + 4.0, stacked, [0.04, 0.03, 0.02, 0.45], align);
    }
    for heart in crate::minecraft_vitals::hearts(vitals) {
        let x = x0 + heart.column as f32 * VITALS_PITCH;
        let y = first - heart.row as f32 * row_height * px - heart.lift * px;
        for layer in &heart.layers {
            if let Some(rect) = ui.icon_rects.get(*layer) {
                canvas.quad(x, y, VITALS_ICON, VITALS_ICON, [1.0; 4], ICONS, *rect, align);
            }
        }
    }
    vitals_backing(canvas, right - row_width, first, row_width, align);
    for (j, drumstick) in crate::minecraft_vitals::drumsticks(vitals).iter().enumerate() {
        let x = right - VITALS_ICON - j as f32 * VITALS_PITCH;
        let y = first + drumstick.shake * px;
        for layer in std::iter::once(drumstick.empty).chain(drumstick.fill) {
            if let Some(rect) = ui.icon_rects.get(layer) {
                canvas.quad(x, y, VITALS_ICON, VITALS_ICON, [1.0; 4], ICONS, *rect, align);
            }
        }
    }
    let saturation = crate::minecraft_vitals::saturation_share(vitals) * (row_width + 4.0);
    if saturation > 0.0 {
        canvas.fill(right + 2.0 - saturation, first + VITALS_ICON + 0.5, saturation, 1.0, ACCENT, align);
    }
    if let Some(row) = crate::minecraft_vitals::bubbles(vitals.air, vitals.max_air, vitals.eye_in_water) {
        vitals_backing(canvas, right - row_width, second, row_width, align);
        for (i, bubble) in row.iter().enumerate() {
            if let Some(rect) = bubble.sprite().and_then(|key| ui.icon_rects.get(key)) {
                let x = right - VITALS_ICON - i as f32 * VITALS_PITCH;
                canvas.quad(x, second, VITALS_ICON, VITALS_ICON, [1.0; 4], ICONS, *rect, align);
            }
        }
    }
}

fn draw_station(canvas: &mut Canvas<'_>, ui: &MinecraftUi, weapons: Option<&PreparedWeapons>, view: &StationView, c: (i32, i32)) {
    match ui.screen {
        McScreen::Chest { rows } => {
            let (x, y) = (GRID_X - 3.0, chest_top(rows) - 3.0);
            let (w, h) = (9.0 * P + 4.0, f32::from(rows) * P + 4.0);
            canvas.frame(x, y, w, h, EDGE, c);
            canvas.brackets(x, y, w, h, 5.0, ACCENT, c);
            canvas.text(FONT_SMALL, x, y - 8.0, 6.0, TEXT_DIM, "STORAGE", false, c);
        }
        McScreen::Furnace => {
            let [tx, ty, tw, th] = [-74.0, -100.0, 144.0, 82.0];
            canvas.quad(tx, ty + th - 26.0, tw, 26.0, [0.0, 0.0, 0.0, 0.35], "gradient_fadein_fadebottom", [0.0, 0.0, 1.0, 1.0], c);
            canvas.frame(tx, ty, tw, th, EDGE, c);
            canvas.brackets(tx, ty, tw, th, 5.0, ACCENT, c);
            let [ix, iy] = FURNACE_INPUT;
            canvas.text(FONT_SMALL, ix - 22.0, iy + 7.0, 5.5, TEXT_DIM, "INPUT", false, c);
            canvas.text(FONT_SMALL, ix - 22.0, iy + 51.0, 5.5, TEXT_DIM, "FUEL", false, c);
            let [lit, cook] = ui.progress;
            if let Some(&[s0, t0, s1, t1]) = ui.icon_rects.get("gui:container/furnace/lit_progress") {
                let (fx, fy, size) = (ix + 2.0, iy + 24.0, 16.0);
                canvas.quad(fx, fy, size, size, [1.0, 1.0, 1.0, 0.12], ICONS, [s0, t0, s1, t1], c);
                if lit > 0.0 {
                    let shown = ((lit * 13.0).ceil() + 1.0) / 14.0;
                    let cut = 1.0 - shown;
                    canvas.quad(fx, fy + size * cut, size, size * shown, [1.0; 4], ICONS, [s0, t0 + (t1 - t0) * cut, s1, t1], c);
                }
            }
            if let Some(&[s0, t0, s1, t1]) = ui.icon_rects.get("gui:container/furnace/burn_progress") {
                let (ax, ay, w, h) = (-12.0, FURNACE_OUTPUT[1] + 4.0, 27.0, 18.0);
                canvas.quad(ax, ay, w, h, [1.0, 1.0, 1.0, 0.12], ICONS, [s0, t0, s1, t1], c);
                let shown = (cook * 24.0).ceil() / 24.0;
                if shown > 0.0 {
                    canvas.quad(ax, ay, w * shown, h, [1.0; 4], ICONS, [s0, t0, s0 + (s1 - s0) * shown, t1], c);
                }
            }
        }
        McScreen::Brewing => {
            work_box(canvas, [-92.0, -104.0, 140.0, 90.0], c);
            canvas.text(FONT_SMALL, -82.0, -104.0 + 2.5, 5.5, TEXT_DIM, "FUEL", false, c);
            canvas.text(FONT_SMALL, -16.0, -104.0 + 2.5, 5.5, TEXT_DIM, "INGREDIENT", false, c);
            let [fuel, brew] = ui.progress;
            canvas.fill(-82.0, -72.0, S, 2.5, [0.0, 0.0, 0.0, 0.55], c);
            canvas.fill(-82.0, -72.0, S * fuel, 2.5, ACCENT, c);
            canvas.fill(16.0, -96.0, 3.0, 46.0, [0.0, 0.0, 0.0, 0.55], c);
            canvas.fill(16.0, -96.0, 3.0, 46.0 * brew, HIGHLIGHT, c);
            for x in [-30.0, 0.0, 30.0] {
                canvas.fill(x, -58.0, 0.5, 10.0, EDGE, c);
            }
            canvas.fill(-30.0, -58.0, 60.5, 0.5, EDGE, c);
            canvas.fill(0.0, -76.0, 0.5, 18.0, EDGE, c);
        }
        McScreen::Stonecutter => {
            work_box(canvas, [-94.0, -102.0, 178.0, 66.0], c);
            canvas.text(FONT_TITLE, 32.0, -76.0, 11.0, TEXT_DIM, ">", false, c);
            for (index, [x, y, w, h]) in choice_rects(ui.screen, ui.choices.len(), view.scroll) {
                let lit = view.hovered == Some(index) || view.focus == Some(index);
                canvas.fill(x, y, w - 1.0, h - 1.0, if lit { SLOT_HOVER } else { SLOT }, c);
                let edge = if ui.chosen == Some(index) { ACCENT } else if lit { HIGHLIGHT } else { SLOT_EDGE };
                canvas.frame(x, y, w - 1.0, h - 1.0, edge, c);
                if let Some(icon) = ui.choices.get(index).and_then(|choice| choice.icon.clone()) {
                    draw_stack(canvas, ui, weapons, &icon, x, y, w - 1.0, c);
                }
            }
            let rows = ui.choices.len().div_ceil(CHOICE_COLUMNS);
            if rows > CHOICE_ROWS {
                let track = CHOICE_ROWS as f32 * CHOICE_CELL;
                let thumb = track * CHOICE_ROWS as f32 / rows as f32;
                let top = (track - thumb) * view.scroll as f32 / (rows - CHOICE_ROWS) as f32;
                canvas.fill(20.0, -96.0, 2.0, track, [0.0, 0.0, 0.0, 0.55], c);
                canvas.fill(20.0, -96.0 + top, 2.0, thumb, ACCENT, c);
            }
        }
        McScreen::Smithing => {
            work_box(canvas, [-92.0, -100.0, 176.0, 70.0], c);
            canvas.text(FONT_SMALL, -84.0, -74.0, 5.0, TEXT_DIM, "TEMPLATE", false, c);
            canvas.text(FONT_SMALL, -60.0, -36.0, 5.0, TEXT_DIM, "BASE", false, c);
            canvas.text(FONT_SMALL, -36.0, -74.0, 5.0, TEXT_DIM, "MATERIAL", false, c);
            canvas.text(FONT_TITLE, 12.0, -60.0, 11.0, TEXT_DIM, ">", false, c);
        }
        McScreen::Grindstone => {
            work_box(canvas, [-74.0, -100.0, 144.0, 72.0], c);
            canvas.fill(-40.0, -72.0, 0.5, 14.0, EDGE, c);
            canvas.text(FONT_TITLE, 12.0, -72.0, 11.0, TEXT_DIM, ">", false, c);
        }
        McScreen::Anvil => {
            work_box(canvas, [-92.0, -104.0, 190.0, 82.0], c);
            let [nx, ny, nw, nh] = NAME_BOX;
            canvas.fill(nx, ny, nw, nh, [0.0, 0.0, 0.0, 0.6], c);
            canvas.frame(nx, ny, nw, nh, if ui.container.first().is_some_and(Option::is_some) { HIGHLIGHT } else { EDGE }, c);
            let shown = if view.name.is_empty() { ui.name.clone().unwrap_or_else(|| left_name(ui)) } else { view.name.clone() };
            canvas.text(FONT_SMALL, nx + 4.0, ny + 4.0, 6.0, TEXT, &shown, false, c);
            canvas.text(FONT_TITLE, -60.0, -62.0, 11.0, TEXT_DIM, "+", false, c);
            canvas.text(FONT_TITLE, 12.0, -64.0, 11.0, TEXT_DIM, ">", false, c);
            if let Some(cost) = ui.cost {
                let level = ui.vitals.as_ref().map_or(0, |v| v.experience_level);
                let (label, color) = if cost >= 40 {
                    ("TOO EXPENSIVE!".to_owned(), VANILLA_RED)
                } else {
                    (format!("ENCHANTMENT COST: {cost}"), if level >= cost { VANILLA_GREEN } else { VANILLA_RED })
                };
                canvas.text(FONT_SMALL, 94.0, -34.0, 6.0, color, &label, true, c);
            }
        }
        McScreen::Enchanting => {
            work_box(canvas, [-94.0, -104.0, 192.0, 72.0], c);
            canvas.text(FONT_SMALL, -88.0, -58.0, 5.0, TEXT_DIM, "ITEM", false, c);
            canvas.text(FONT_SMALL, -64.0, -58.0, 5.0, TEXT_DIM, "LAPIS", false, c);
            let level = ui.vitals.as_ref().map_or(0, |v| v.experience_level);
            for (index, [x, y, w, h]) in choice_rects(ui.screen, ui.choices.len(), 0) {
                let Some(choice) = ui.choices.get(index) else { continue };
                let lit = choice.enabled && (view.hovered == Some(index) || view.focus == Some(index));
                canvas.fill(x, y, w, h, if lit { SLOT_HOVER } else { SLOT }, c);
                canvas.frame(x, y, w, h, if lit { HIGHLIGHT } else { SLOT_EDGE }, c);
                if choice.cost == 0 {
                    continue;
                }
                let tone = if choice.enabled { TEXT } else { TEXT_DIM };
                canvas.text(FONT_SMALL, x + 4.0, y + 3.0, 5.5, tone, &format!("{} LAPIS", index + 1), false, c);
                if !choice.label.is_empty() {
                    canvas.text(FONT_SMALL, x + 4.0, y + 11.0, 5.5, tone, &format!("{} . . . ?", choice.label), false, c);
                }
                let color = if level >= choice.cost { VANILLA_GREEN } else { VANILLA_RED };
                canvas.text(FONT_COUNT, x + w - 4.0, y + 6.0, 7.5, color, &choice.cost.to_string(), true, c);
            }
        }
        _ => {}
    }
}

fn work_box(canvas: &mut Canvas<'_>, [x, y, w, h]: [f32; 4], c: (i32, i32)) {
    canvas.quad(x, y + h - 26.0, w, 26.0, [0.0, 0.0, 0.0, 0.35], "gradient_fadein_fadebottom", [0.0, 0.0, 1.0, 1.0], c);
    canvas.frame(x, y, w, h, EDGE, c);
    canvas.brackets(x, y, w, h, 5.0, ACCENT, c);
}

const VANILLA_RED: [f32; 4] = [1.0, 0.33, 0.33, 1.0];

/// The inventory screen.
#[allow(clippy::too_many_arguments)]
fn draw_inventory(
    canvas: &mut Canvas<'_>,
    ui: &mut MinecraftUi,
    weapons: Option<&PreparedWeapons>,
    strings: Option<&PreparedLocalizedStrings>,
    gaps: &mut HudPresentationGaps,
    hovered: Option<McSlot>,
    mouse: Option<Vec2>,
    prompts: Option<PromptStyle>,
    view: &StationView,
) {
    let c = (CENTER, CENTER);
    // Dim the world around the panel; the character window stays clear.
    let dim = [0.0, 0.0, 0.0, 0.55];
    let [bx, by, bw, bh] = BOX;
    let table = ui.screen == McScreen::CraftingTable;
    let station = ui.screen != McScreen::Inventory;
    let top = panel_top(ui.screen);
    let height = PANEL_H + PANEL_Y - top;
    let around = [
        (-2000.0, -2000.0, 4000.0, 2000.0 + by),
        (-2000.0, by + bh, 4000.0, 2000.0),
        (-2000.0, by, 2000.0 + bx, bh),
        (bx + bw, by, 2000.0, bh),
    ];
    let whole = [(-2000.0, -2000.0, 4000.0, 4000.0)];
    for &(x, y, w, h) in if station { &whole[..] } else { &around[..] } {
        canvas.fill(x, y, w, h, dim, c);
        canvas.quad(x, y, w, h, [ACCENT[0], ACCENT[1], ACCENT[2], 0.07], "gradient_fadein_fadebottom", [0.0, 0.0, 1.0, 1.0], c);
    }

    // The panel: glass, header, edges and brackets.
    let glass = [
        (PANEL_X, PANEL_Y + HEADER_H, PANEL_W, by - PANEL_Y - HEADER_H),
        (PANEL_X, by + bh, PANEL_W, PANEL_Y + PANEL_H - by - bh),
        (PANEL_X, by, bx - PANEL_X, bh),
        (bx + bw, by, PANEL_X + PANEL_W - bx - bw, bh),
    ];
    let whole_glass = [(PANEL_X, top + HEADER_H, PANEL_W, height - HEADER_H)];
    for &(x, y, w, h) in if station { &whole_glass[..] } else { &glass[..] } {
        canvas.fill(x, y, w, h, PANEL, c);
    }
    canvas.fill(PANEL_X, top, PANEL_W, HEADER_H, HEADER, c);
    canvas.quad(PANEL_X, top, PANEL_W, HEADER_H, [ACCENT[0], ACCENT[1], ACCENT[2], 0.22], "gradient_fadein_fadebottom", [0.0, 0.0, 1.0, 1.0], c);
    canvas.fill(PANEL_X, top + HEADER_H - 1.0, PANEL_W, 1.0, [ACCENT[0], ACCENT[1], ACCENT[2], 0.85], c);
    canvas.fill(PANEL_X, top, 3.0, HEADER_H, ACCENT, c);
    let title = if table { "CRAFTING TABLE".to_owned() } else if station { ui.screen_title.clone() } else { "INVENTORY".to_owned() };
    canvas.text(FONT_TITLE, PANEL_X + 9.0, top + 5.0, 10.0, HIGHLIGHT, &title, false, c);
    let close = prompts.map_or("E", |style| pad_label(style, GamepadButton::East));
    canvas.text(FONT_SMALL, PANEL_X + PANEL_W - 8.0, top + 7.5, 6.0, TEXT_DIM, &format!("SURVIVAL  //  {close} TO CLOSE"), true, c);
    canvas.frame(PANEL_X, top, PANEL_W, height, EDGE, c);
    canvas.brackets(PANEL_X - 2.0, top - 2.0, PANEL_W + 4.0, height + 4.0, 8.0, [ACCENT[0], ACCENT[1], ACCENT[2], 0.9], c);

    if station && !table {
        draw_station(canvas, ui, weapons, view, c);
    } else if table {
        let [tx, ty, tw, th] = TABLE_BOX;
        canvas.quad(tx, ty + th - 26.0, tw, 26.0, [0.0, 0.0, 0.0, 0.35], "gradient_fadein_fadebottom", [0.0, 0.0, 1.0, 1.0], c);
        canvas.frame(tx, ty, tw, th, EDGE, c);
        canvas.brackets(tx, ty, tw, th, 5.0, ACCENT, c);
        canvas.text(FONT_SMALL, tx, ty - 8.0, 6.0, TEXT_DIM, "CRAFTING", false, c);
        canvas.text(FONT_TITLE, 14.0, -62.5, 11.0, TEXT_DIM, ">", false, c);
    } else {
        // The character window: clear, with a frame and MW2 brackets.
        canvas.quad(bx, by + bh - 26.0, bw, 26.0, [0.0, 0.0, 0.0, 0.35], "gradient_fadein_fadebottom", [0.0, 0.0, 1.0, 1.0], c);
        canvas.frame(bx, by, bw, bh, EDGE, c);
        canvas.brackets(bx, by, bw, bh, 5.0, ACCENT, c);
        canvas.text(FONT_SMALL, bx + 4.0, by + bh - 9.0, 5.5, [HIGHLIGHT[0], HIGHLIGHT[1], HIGHLIGHT[2], 0.75], "OPERATOR", false, c);
        canvas.text(FONT_SMALL, 22.0, -100.0, 6.0, TEXT_DIM, "CRAFTING", false, c);
        canvas.fill(22.0, -92.5, 90.0, 0.5, EDGE, c);
        canvas.text(FONT_TITLE, 68.0, -73.5, 11.0, TEXT_DIM, ">", false, c);
    }

    // Section labels.
    canvas.text(FONT_SMALL, GRID_X, -6.5, 6.0, TEXT_DIM, "BACKPACK", false, c);
    canvas.fill(GRID_X + 38.0, -3.5, 158.0, 0.5, EDGE, c);
    canvas.fill(GRID_X, 75.0, 196.0, 0.5, [ACCENT[0], ACCENT[1], ACCENT[2], 0.45], c);

    // The slots.
    for (slot, x, y, size) in layout(ui.screen) {
        let hover = hovered == Some(slot);
        canvas.fill(x, y, size, size, if hover { SLOT_HOVER } else { SLOT }, c);
        canvas.frame(x, y, size, size, if hover { HIGHLIGHT } else if is_result(slot) { [ACCENT[0], ACCENT[1], ACCENT[2], 0.55] } else { SLOT_EDGE }, c);
        if hover && prompts.is_some() {
            canvas.brackets(x - 2.0, y - 2.0, size + 4.0, size + 4.0, 4.0, ACCENT, c);
        }
        if let McSlot::Inventory(i) = slot
            && i < 9
            && i == ui.selected
        {
            canvas.fill(x, y + size - 1.5, size, 1.5, ACCENT, c);
        }
        if let Some(stack) = stack_at(ui, slot).cloned() {
            draw_stack(canvas, ui, weapons, &stack, x, y, size, c);
        } else if let McSlot::Inventory(i) = slot
            && (36..=40).contains(&i)
        {
            // Empty armor and offhand slots name what they take.
            let label = match i {
                39 => "HEAD",
                38 => "CHEST",
                37 => "LEGS",
                36 => "FEET",
                _ => "OFF",
            };
            let px = 4.5;
            let w = canvas.fonts.get(FONT_SMALL).map_or(0.0, |def| ui_text_width(def, label, px / ui_text_height(1.0)));
            canvas.text(FONT_SMALL, x + (size - w) * 0.5, y + (size - px) * 0.5 - 0.5, px, [1.0, 1.0, 1.0, 0.22], label, false, c);
        }
    }

    if let Some(style) = prompts {
        draw_pad_legend(canvas, style, ui.cursor.is_some(), hovered.is_some_and(is_result));
    }

    // The carried stack follows the mouse; otherwise the hovered item's card.
    let Some(mouse) = mouse else { return };
    let placement = canvas.surface.placement();
    let [sx, sy] = placement.scale_virtual_to_real;
    let (w_real, h_real) = (canvas.surface.width(), canvas.surface.height());
    // The mouse in the panel's centred virtual space.
    let vx = (mouse.x - w_real * 0.5) / sx / K;
    let vy = (mouse.y - h_real * 0.5) / sy / K;
    if let Some(stack) = ui.cursor.clone() {
        draw_stack(canvas, ui, weapons, &stack, vx - 10.0, vy - 10.0, 20.0, c);
        return;
    }
    let Some(slot) = hovered else { return };
    let Some(stack) = stack_at(ui, slot).cloned() else { return };
    let name = stack_name(&stack, ui, weapons, strings, gaps).to_uppercase();
    let sub = if stack.weapon.is_some() { "PRIMARY WEAPON".to_owned() } else { stack.id.clone() };
    let lines = tooltip_lines(&stack, weapons);
    let title_px = 8.0;
    let sub_px = 5.5;
    let line_h = 7.0;
    let title_w = canvas.fonts.get(FONT_TITLE).map_or(60.0, |def| ui_text_width(def, &name, title_px / ui_text_height(1.0)));
    let small_w = |text: &str| canvas.fonts.get(FONT_SMALL).map_or(60.0, |def| ui_text_width(def, text, sub_px / ui_text_height(1.0)));
    let sub_w = small_w(&sub);
    let lines_w = lines.iter().map(|(line, _)| small_w(line)).fold(0.0, f32::max);
    let w = title_w.max(sub_w).max(lines_w) + 14.0;
    let h = 22.0 + if lines.is_empty() { 0.0 } else { 2.0 + lines.len() as f32 * line_h };
    let (x, y) = (vx + 10.0, vy - h);
    canvas.fill(x, y, w, h, [0.02, 0.025, 0.028, 0.94], c);
    canvas.frame(x, y, w, h, EDGE, c);
    canvas.fill(x, y, 2.0, h, ACCENT, c);
    canvas.text(FONT_TITLE, x + 7.0, y + 3.0, title_px, HIGHLIGHT, &name, false, c);
    canvas.text(FONT_SMALL, x + 7.0, y + 13.5, sub_px, TEXT_DIM, &sub, false, c);
    for (i, (line, tone)) in lines.iter().enumerate() {
        let color = match tone {
            crate::minecraft_tooltip::Tone::Plain => TEXT,
            crate::minecraft_tooltip::Tone::Header => TEXT_DIM,
            crate::minecraft_tooltip::Tone::Value => VANILLA_GREEN,
        };
        canvas.text(FONT_SMALL, x + 7.0, y + 22.0 + 2.0 + i as f32 * line_h, sub_px, color, line, false, c);
    }
}

const VANILLA_GREEN: [f32; 4] = [0.33, 0.85, 0.33, 1.0];

fn tooltip_lines(stack: &McStack, weapons: Option<&PreparedWeapons>) -> Vec<(String, crate::minecraft_tooltip::Tone)> {
    let mut lines: Vec<(String, crate::minecraft_tooltip::Tone)> =
        stack.enchantments.iter().map(|name| (name.clone(), crate::minecraft_tooltip::Tone::Header)).collect();
    lines.extend(match (stack.weapon, stack.attack) {
        (Some(weapon), _) => weapons
            .and_then(|reg| reg.0.facts_of(weapon))
            .map_or_else(Vec::new, |facts| crate::minecraft_tooltip::gun_lines(&facts)),
        (None, Some(attack)) => crate::minecraft_tooltip::attack_lines(attack),
        (None, None) => Vec::new(),
    });
    lines
}
