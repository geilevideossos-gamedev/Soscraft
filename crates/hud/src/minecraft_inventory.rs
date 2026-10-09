use assets::{PreparedLocalizedStrings, PreparedWeapons};
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use frame::minecraft_ui::MC_HOTBAR;
use frame::{McAction, McClick, McFont, McGameMode, McScreen, McSlot, McStack, McTabKind, MinecraftUi, PromptStyle};

use crate::gaps::HudPresentationGaps;
use crate::gpu_list::{HudTessPass, TessJob};
use crate::images::HudImages;
use crate::minecraft_chat::MODIFIERS;
use crate::minecraft_clicks::{Clicks, Menu, Target};
use crate::minecraft_gui::{ANIMATED_ICON_MATERIAL, DARK_GREEN, GLINT_MATERIAL, GRAY, GUI_MATERIAL, GuiSpace, ICON_MATERIAL, Painter, TextLine, WHITE, YELLOW};
use crate::minecraft_screens::{
    Cycler, Layout, PUPPET_BOX, RECIPE_COLUMNS, RECIPE_ROWS, Screen, SlotSpec, Station, drag_offset, enchant_row_at, hidden_rows,
    over_scroller, recipe_at, smithing_icons, smithing_templates, start_index,
};

const REPEAT_DELAY: f64 = 0.4;
const REPEAT_EVERY: f64 = 0.12;
const STICK: f32 = 0.6;
const TICK: f64 = 0.05;
const NAME_LIMIT: usize = 50;
const NAME_WIDTH: i32 = 103;

fn left_name(ui: &MinecraftUi) -> String {
    ui.container.first().cloned().flatten().map_or_else(String::new, |left| {
        left.name.clone().or_else(|| ui.names.get(&left.id).cloned()).unwrap_or_else(|| left.id.clone())
    })
}

pub(crate) fn scroll_to(font: &McFont, value: &str, display: usize, pos: usize) -> usize {
    scroll_within(font, value, display, pos, NAME_WIDTH)
}

pub(crate) fn scroll_within(font: &McFont, value: &str, display: usize, pos: usize, width: i32) -> usize {
    let chars: Vec<char> = value.chars().collect();
    let len = chars.len();
    let mut display = display.min(len);
    let rest: String = chars[display..].iter().collect();
    let end = font.head_by_width(&rest, width).chars().count() + display;
    if pos == display {
        let mut fits = 0;
        let mut left = width as f32;
        for &c in chars.iter().rev() {
            left -= font.advance(c);
            if left < 0.0 {
                break;
            }
            fits += 1;
        }
        display = display.saturating_sub(fits);
    }
    if pos > end {
        display += pos - end;
    } else if pos <= display {
        display = pos;
    }
    display.min(len)
}

#[derive(Component)]
pub(crate) struct MinecraftRaster;

#[derive(Default)]
pub(crate) struct ScreenInput {
    open: bool,
    screen: McScreen,
    clicks: Clicks,
    focus: Option<McSlot>,
    repeat: Option<(Nav, f64)>,
    pad_pointer: bool,
    last_pointer: Option<[f64; 2]>,
    name: String,
    name_for: Option<String>,
    name_from: usize,
    name_since: f64,
    scroll: f32,
    scrolling: bool,
    choice_focus: usize,
    cyclers: [Cycler; 3],
    cycle_clock: f64,
    highlight: crate::minecraft_hotbar::Highlight,
    creative: crate::minecraft_creative::CreativeInput,
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
    held: [bool; 2],
    quick: bool,
}

struct MenuView<'a> {
    ui: &'a MinecraftUi,
    layout: &'a Layout,
}

impl Menu for MenuView<'_> {
    fn slots(&self) -> &[SlotSpec] {
        &self.layout.slots
    }

    fn stack(&self, slot: McSlot) -> Option<&McStack> {
        stack_at(self.ui, slot)
    }

    fn carried(&self) -> Option<&McStack> {
        self.ui.cursor.as_ref()
    }

    fn capacity(&self, slot: McSlot) -> Option<u8> {
        self.ui.droppable.iter().find(|(s, _)| *s == slot).map(|&(_, capacity)| capacity)
    }
}

pub(crate) fn stack_at(ui: &MinecraftUi, slot: McSlot) -> Option<&McStack> {
    match slot {
        McSlot::Inventory(i) => ui.slots.get(i)?.as_ref(),
        McSlot::Crafting(i) => ui.crafting.get(i)?.as_ref(),
        McSlot::Result => ui.result.as_ref(),
        McSlot::Workbench(i) => ui.workbench.get(i)?.as_ref(),
        McSlot::WorkbenchResult => ui.workbench_result.as_ref(),
        McSlot::Container(i) => ui.container.get(i)?.as_ref(),
        McSlot::Output => ui.output.as_ref(),
        McSlot::Creative(i) => ui.creative.items.get(i)?.as_ref(),
        McSlot::Trash => None,
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

fn title(ui: &MinecraftUi, gui: &frame::McGui) -> String {
    match ui.screen {
        McScreen::Inventory | McScreen::CraftingTable => gui.text("container.crafting"),
        _ => ui.screen_title.clone(),
    }
}

type Typing<'w, 's> = (
    Res<'w, ButtonInput<KeyCode>>,
    Res<'w, ButtonInput<MouseButton>>,
    MessageReader<'w, 's, MouseWheel>,
    MessageReader<'w, 's, bevy::input::keyboard::KeyboardInput>,
    MessageReader<'w, 's, bevy::window::CursorMoved>,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_minecraft_hud(
    surface: Res<crate::surface::Hud2dSurface>,
    ui: Option<ResMut<MinecraftUi>>,
    (weapons, strings): (Option<Res<PreparedWeapons>>, Option<Res<PreparedLocalizedStrings>>),
    (keys, buttons, mut wheel, mut typed, mut moves): Typing,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    time: Res<Time>,
    mut input: Local<ScreenInput>,
    mut hud_images: ResMut<HudImages>,
    mut gaps: ResMut<HudPresentationGaps>,
    mut pass: ResMut<HudTessPass>,
    (gamepads, active_pad, devices): (Query<&Gamepad>, Option<Res<frame::ActivePad>>, Option<Res<frame::InputDevices>>),
    (menus, hud_input, mut settings): (
        Res<crate::menus::ScriptMenus>,
        Option<Res<frame::HudInputView>>,
        Option<ResMut<frame::GameSettings>>,
    ),
    (mc_keys, identity, mut clipboard): (Option<Res<frame::McActionInput>>, Option<Res<frame::LaunchIdentity>>, Option<ResMut<bevy::clipboard::Clipboard>>),
) {
    let Some(mut ui) = ui else {
        pass.minecraft = TessJob::Hide;
        return;
    };
    let opened_by_world = std::mem::take(&mut ui.screen_opened);
    let swept: Vec<Vec2> = moves.read().map(|moved| moved.position).collect();
    let window = windows.single().ok();
    let in_world = !ui.loading_world && identity.as_ref().is_some_and(|identity| assets::minecraft_map::is_minecraft_load(&identity.zone));
    let (hud_shown, chat_shown) = minecraft_layers(ui.active, in_world);
    let Some(window) = window.filter(|_| hud_shown && surface.is_ready()) else {
        pass.minecraft = TessJob::Hide;
        ui.character_box = None;
        for _ in wheel.read() {}
        input.open = false;
        input.clicks.reset();
        if let Some((window, gui)) = window.zip(ui.gui.clone()).filter(|_| chat_shown && surface.is_ready()) {
            let space = GuiSpace::new([window.physical_width(), window.physical_height()], [surface.width(), surface.height()], [window.width(), window.height()]);
            hud_images.insert_runtime(GUI_MATERIAL, gui.image.clone());
            let mut painter = Painter::new(&gui, space);
            crate::minecraft_chat::draw(&mut painter, &ui.chat, time.elapsed_secs_f64());
            if !painter.quads.is_empty() {
                pass.minecraft = TessJob::Quads(painter.quads);
            }
        }
        return;
    };
    let now = time.elapsed_secs_f64();
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
    let mc = |action: McAction| mc_keys.as_deref().is_some_and(|keys| keys.just_pressed(action));

    let open_before = ui.inventory_open;
    let hotbar = ui.game_mode.shows_hotbar();
    let searching = ui.inventory_open && ui.screen == McScreen::Creative(McTabKind::Search);
    let typing = searching || (ui.inventory_open && ui.screen == McScreen::Anvil && ui.container.first().is_some_and(Option::is_some));
    let events: Vec<bevy::input::keyboard::KeyboardInput> = typed.read().cloned().collect();
    let text: String = events.iter().filter(|event| event.state.is_pressed()).filter_map(|event| event.text.as_ref()).flat_map(|text| text.chars()).filter(|c| !c.is_control()).collect();
    if !blocked && ((mc(McAction::Inventory) && !typing && !ui.chat.open && (hotbar || ui.inventory_open)) || (ui.inventory_open && keys.just_pressed(KeyCode::Escape))) {
        ui.inventory_open = !ui.inventory_open && !keys.just_pressed(KeyCode::Escape);
    } else if ui.inventory_open && (menus.captures_input() || pad_just(GamepadButton::East)) {
        ui.inventory_open = false;
    } else if !ui.inventory_open && !ui.chat.open && hotbar && pad_just(GamepadButton::DPadUp) {
        ui.inventory_open = true;
    }
    if ui.inventory_open && !open_before {
        ui.screen = McScreen::Inventory;
        if ui.game_mode == McGameMode::Creative
            && let Some(screen) = crate::minecraft_creative::screen_of(&ui.creative)
        {
            crate::minecraft_creative::reopen(&mut ui.creative);
            ui.screen = screen;
        }
    }
    if open_before && !ui.inventory_open {
        ui.clicks.push(McClick::Close);
    }
    let opened = ui.inventory_open && (!input.open || input.screen != ui.screen || opened_by_world);
    if ui.inventory_open != input.open || ui.screen != input.screen {
        input.clicks.reset();
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
    let pressed_digit = digits.iter().position(|k| keys.just_pressed(*k));
    let digit = pressed_digit.filter(|_| !typing);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let mut wheel_steps = 0i32;
    for event in wheel.read() {
        wheel_steps += if event.y < 0.0 { 1 } else if event.y > 0.0 { -1 } else { 0 };
    }
    let dt = time.delta_secs();

    if !ui.inventory_open && !ui.chat.open {
        if wheel_steps != 0 && hotbar {
            ui.select = Some((ui.selected as i32 + wheel_steps).rem_euclid(9) as usize);
        }
        if let Some(digit) = digit.filter(|_| hotbar) {
            ui.select = Some(digit);
        }
        if mc(McAction::Drop) && hotbar {
            ui.drop_selected = Some(ctrl);
        }
        if hotbar && ((!blocked && mc(McAction::SwapHands)) || pad_just(GamepadButton::DPadDown)) {
            ui.swap_hands = true;
        }
        if !blocked
            && keys.just_pressed(KeyCode::KeyM)
            && let Some(settings) = settings.as_mut()
        {
            settings.minecraft_minimap = !settings.minecraft_minimap;
            settings.touch();
        }
        ui.character_box = None;
    }

    let Some(gui) = ui.gui.clone() else {
        pass.minecraft = TessJob::Hide;
        ui.character_box = None;
        return;
    };
    let space = GuiSpace::new(
        [window.physical_width(), window.physical_height()],
        [surface.width(), surface.height()],
        [window.width(), window.height()],
    );
    ui.gui_scale = space.scale;
    hud_images.insert_runtime(GUI_MATERIAL, gui.image.clone());
    if let Some(icons) = ui.icons.clone() {
        hud_images.insert_runtime(ICON_MATERIAL, icons);
    }
    if let Some(icons) = ui.animated_icons.clone() {
        hud_images.insert_runtime(ANIMATED_ICON_MATERIAL, icons);
    }
    if let Some(glint) = ui.glint.clone() {
        hud_images.insert_runtime(GLINT_MATERIAL, glint);
    }
    let weapons = weapons.as_deref();
    let strings = strings.as_deref();
    let mut painter = Painter::new(&gui, space);
    painter.glint_shift = frame::minecraft_ui::glint_shift(time.elapsed().as_millis() as u64);
    let highlighted = ui.slots.get(ui.selected).and_then(Option::as_ref).filter(|stack| stack.weapon.is_none());
    let highlighted = highlighted.map(|stack| (stack.id.clone(), stack_name(stack, &ui, weapons, strings, &mut gaps)));
    input.highlight.advance(f64::from(dt), highlighted);
    crate::minecraft_hotbar::draw_hud(&mut painter, &ui, weapons, input.highlight.shown());
    crate::minecraft_chat::draw(&mut painter, &ui.chat, now);

    if ui.inventory_open {
        let layout = match ui.screen {
            McScreen::Creative(kind) => crate::minecraft_creative::layout(kind, ui.creative.first_shown()),
            screen => crate::minecraft_screens::layout(screen),
        };
        let origin = layout.origin([space.width, space.height]);
        let mouse = window.physical_cursor_position().map(|m| space.mouse([m.x, m.y]));
        let rel_mouse = mouse.map(|[x, y]| [x - f64::from(origin[0]), y - f64::from(origin[1])]);
        if opened {
            input.focus = None;
            input.repeat = held.map(|nav| (nav, f64::INFINITY));
            input.scroll = 0.0;
            input.scrolling = false;
            input.choice_focus = 0;
            input.cyclers = Default::default();
        }
        let mouse_hovered = rel_mouse.and_then(|rel| layout.slot_at(rel));
        if pad_pointer && !input.pad_pointer {
            input.focus = mouse_hovered.or(input.focus);
        }
        input.pad_pointer = pad_pointer;
        if !input.focus.is_some_and(|focus| layout.spec(focus).is_some()) {
            input.focus = Some(McSlot::Inventory(ui.selected.min(MC_HOTBAR - 1)));
        }
        if let Some(nav) = repeat_step(&mut input.repeat, held, now)
            && let Some(from) = input.focus
        {
            input.focus = neighbour(&layout.slots, from, nav).or(Some(from));
        }
        let pointer: Option<[f64; 2]> = if pad_pointer {
            input.focus.and_then(|focus| layout.spec(focus)).map(|spec| [f64::from(spec.x + 12), f64::from(spec.y + 12)])
        } else {
            rel_mouse
        };
        let hovered = if pad_pointer { input.focus } else { mouse_hovered };
        let target = match (hovered, pointer) {
            (Some(slot), _) => Target::Slot(slot),
            (None, Some(rel)) if layout.outside(rel) && !crate::minecraft_creative::over_tab(&ui, rel) => Target::Outside,
            _ => Target::Nothing,
        };
        let screen_buttons = screen_buttons(&buttons, pad);
        input.cycle_clock += f64::from(dt);
        while input.cycle_clock >= TICK {
            input.cycle_clock -= TICK;
            if ui.screen == McScreen::Smithing {
                let [bases, additions] = smithing_icons(ui.container.first().and_then(Option::as_ref));
                input.cyclers[0].tick(smithing_templates());
                input.cyclers[1].tick(bases);
                input.cyclers[2].tick(additions);
            }
        }
        if opened || blocked {
            input.clicks.reset();
        } else {
            let pad_choice = [pad_just(GamepadButton::LeftTrigger), pad_just(GamepadButton::RightTrigger), pad_just(GamepadButton::RightTrigger2)];
            let creative_consumed = if matches!(ui.screen, McScreen::Creative(_)) {
                let held = MODIFIERS.map(|code| crate::minecraft_chat::held_before(keys.pressed(code), keys.just_pressed(code), keys.just_released(code)));
                let search_swap = searching && pressed_digit.is_some() && hovered.and_then(|slot| stack_at(&ui, slot)).is_some() && matches!(hovered, Some(McSlot::Creative(_)));
                let typed: Vec<crate::minecraft_chat::Typed> = crate::minecraft_chat::fold_typed(held, &events)
                    .into_iter()
                    .filter(|typed| !(search_swap && digits.contains(&typed.code)))
                    .collect();
                if search_swap
                    && let (Some(slot), Some(hotbar)) = (hovered, pressed_digit)
                {
                    ui.clicks.push(McClick::Swap { slot, hotbar });
                }
                let mut none = ();
                let clipboard: &mut dyn crate::minecraft_chat::ChatClipboard = match clipboard.as_deref_mut() {
                    Some(clipboard) => clipboard,
                    None => &mut none,
                };
                let frame = crate::minecraft_creative::CreativeFrame {
                    mouse: pointer.filter(|_| !pad_pointer),
                    pressed: screen_buttons.pressed,
                    released: screen_buttons.released,
                    held: screen_buttons.held,
                    wheel: wheel_steps,
                    typed: &typed,
                    chat_key: mc(McAction::Chat),
                    now,
                };
                crate::minecraft_creative::input(&mut ui, &mut input.creative, frame, Some(&gui.font), clipboard)
            } else {
                [false; 2]
            };
            let consumed = station_input(
                &mut ui,
                &mut input,
                &gui,
                pointer.filter(|_| !pad_pointer),
                screen_buttons,
                pad_choice,
                (&text, keys.just_pressed(KeyCode::Backspace)),
                (wheel_steps, opened, now),
            );
            let view = MenuView { ui: &ui, layout: &layout };
            let moved = pointer != input.last_pointer;
            let mut clicks = Vec::new();
            if input.clicks.drag.is_some() {
                let scale = window.scale_factor();
                for at in swept.iter().filter(|_| !pad_pointer) {
                    let [x, y] = space.mouse([at.x * scale, at.y * scale]);
                    if let Some(slot) = layout.slot_at([x - f64::from(origin[0]), y - f64::from(origin[1])]) {
                        input.clicks.drag_over(&view, Target::Slot(slot));
                    }
                }
                if moved {
                    input.clicks.drag_over(&view, target);
                }
            }
            if screen_buttons.quick
                && input.clicks.drag.is_none()
                && let Some(slot) = hovered
            {
                clicks.push(McClick::Slot { slot, right: false, shift: true });
            }
            if buttons.just_pressed(MouseButton::Middle)
                && ui.game_mode == McGameMode::Creative
                && ui.cursor.is_none()
                && let Some(slot) = hovered.filter(|&slot| stack_at(&ui, slot).is_some())
            {
                clicks.push(McClick::Clone(slot));
            }
            for (button, taken) in consumed.into_iter().enumerate() {
                let taken = taken || creative_consumed[button];
                if screen_buttons.pressed[button] && !taken {
                    let idle = input.clicks.drag.is_none();
                    clicks.extend(input.clicks.press(&view, target, button, shift, now));
                    if pad_pointer && idle {
                        input.clicks.drag_over(&view, target);
                    }
                }
                if screen_buttons.released[button] {
                    clicks.extend(input.clicks.release(&view, target, button, shift));
                }
            }
            if !typing {
                let swap = digit.or_else(|| mc(McAction::SwapHands).then_some(40));
                let throw = mc(McAction::Drop).then_some(ctrl);
                clicks.extend(input.clicks.key(&view, hovered, swap, throw));
            }
            ui.clicks.extend(clicks);
        }
        input.last_pointer = pointer;

        let pointer_gui = pointer.map_or([i32::MIN / 2; 2], |[x, y]| [x.floor() as i32 + origin[0], y.floor() as i32 + origin[1]]);
        let choices = ui.choices.len();
        let start = start_index(input.scroll, choices);
        let hovered_choice = rel_mouse.filter(|_| !pad_pointer).and_then(|rel| match ui.screen {
            McScreen::Enchanting => enchant_row_at(rel),
            _ => None,
        });
        let focused_choice = pad_pointer.then_some(input.choice_focus).filter(|_| matches!(ui.screen, McScreen::Enchanting | McScreen::Stonecutter));
        let level = ui.vitals.as_ref().map_or(0, |v| v.experience_level);
        let cursor_on = ((now - input.name_since) * 1000.0 / 300.0) as i64 % 2 == 0 && typing;
        let screen_title = title(&ui, &gui);
        let drag = input.clicks.drag.clone();
        let view = MenuView { ui: &ui, layout: &layout };
        let screen = Screen {
            ui: &ui,
            weapons,
            layout: &layout,
            menu: &view,
            origin,
            mouse: pointer_gui,
            hovered,
            drag: drag.as_ref(),
            title: &screen_title,
            station: Station {
                hovered_choice,
                focused_choice,
                scroll: input.scroll,
                start,
                name: &input.name,
                name_from: input.name_from,
                cursor: cursor_on,
                cyclers: &input.cyclers,
                partial: (input.cycle_clock / TICK) as f32,
                level,
            },
        };
        let puppet_box = match ui.screen {
            McScreen::Inventory => Some(PUPPET_BOX),
            McScreen::Creative(McTabKind::Inventory) => Some(crate::minecraft_creative::PUPPET_BOX),
            _ => None,
        };
        let hole = puppet_box.map(|[x0, y0, x1, y1]| [origin[0] + x0, origin[1] + y0, origin[0] + x1, origin[1] + y1]);
        painter.gradient([0, 0, space.width, space.height], hole, 0xC010_1010, 0xD010_1010);
        if matches!(ui.screen, McScreen::Creative(_)) {
            crate::minecraft_creative::draw_background(&mut painter, &ui, weapons, origin, &input.creative, now);
            crate::minecraft_creative::draw_labels(&mut painter, &ui, origin);
        } else {
            crate::minecraft_screens::draw_background(&mut painter, &screen);
            crate::minecraft_screens::draw_labels(&mut painter, &screen);
        }
        if ui.screen == McScreen::Anvil {
            crate::minecraft_screens::anvil_cost(&mut painter, &screen);
        }
        crate::minecraft_screens::draw_highlight(&mut painter, &screen, false);
        crate::minecraft_screens::draw_slots(&mut painter, &screen);
        crate::minecraft_screens::draw_highlight(&mut painter, &screen, true);
        if pointer.is_some() {
            crate::minecraft_screens::draw_carried(&mut painter, &screen, pointer_gui);
        }
        let item_tooltip = hovered
            .filter(|_| ui.cursor.is_none())
            .and_then(|slot| stack_at(&ui, slot))
            .map(|stack| (tooltip_lines(stack, &ui, weapons, strings, &mut gaps), true));
        let recipe_tooltip = crate::minecraft_screens::hovered_recipe(&screen)
            .and_then(|index| ui.choices.get(index))
            .and_then(|choice| choice.icon.as_ref())
            .map(|stack| (tooltip_lines(stack, &ui, weapons, strings, &mut gaps), true));
        let creative_tooltip = || {
            let rel = rel_mouse.filter(|_| matches!(ui.screen, McScreen::Creative(_)))?;
            crate::minecraft_creative::tooltip(&painter, &ui, rel, hovered).map(|lines| (lines, false))
        };
        let tooltip = item_tooltip.or(recipe_tooltip).or_else(|| crate::minecraft_screens::station_tooltip(&painter, &screen)).or_else(creative_tooltip);
        if let (Some((lines, gap)), Some(_)) = (tooltip, pointer) {
            painter.tooltip(&lines, pointer_gui, gap);
        }
        if let Some(style) = prompts {
            draw_pad_legend(&mut painter, &layout, origin, style, ui.cursor.is_some(), hovered.is_some_and(is_result));
        }
        if let Some(puppet) = puppet_box {
            let [bx0, by0, bx1, by1] = puppet;
            let height = if puppet == PUPPET_BOX { (by1 - by0) as f32 } else { crate::minecraft_creative::PUPPET_HEIGHT };
            let middle = (by0 + by1) as f32 * 0.5;
            let [x0, y0] = space.window((origin[0] + bx0) as f32, origin[1] as f32 + middle - height * 0.5);
            let [x1, y1] = space.window((origin[0] + bx1) as f32, origin[1] as f32 + middle + height * 0.5);
            ui.character_box = Some([(x0 + x1) * 0.5, (y0 + y1) * 0.5, x1 - x0, y1 - y0]);
            if pointer.is_some() {
                ui.puppet_turn = puppet_turn_in(puppet, origin, pointer_gui);
            }
        } else {
            ui.character_box = None;
        }
    }
    pass.minecraft = if painter.quads.is_empty() { TessJob::Hide } else { TessJob::Quads(painter.quads) };
}

pub(crate) fn puppet_turn_in(puppet: [i32; 4], origin: [i32; 2], mouse: [i32; 2]) -> [f32; 3] {
    let [bx0, by0, bx1, by1] = puppet;
    let cx = (origin[0] + bx0 + origin[0] + bx1) as f32 / 2.0;
    let cy = (origin[1] + by0 + origin[1] + by1) as f32 / 2.0;
    let across = ((cx - mouse[0] as f32) / 40.0).atan();
    let down = ((cy - mouse[1] as f32) / 40.0).atan();
    [across * 20.0, -down * 20.0, down * 20.0]
}

#[allow(clippy::too_many_arguments)]
fn station_input(
    ui: &mut MinecraftUi,
    input: &mut ScreenInput,
    gui: &frame::McGui,
    mouse: Option<[f64; 2]>,
    buttons: Buttons,
    [previous, next, confirm]: [bool; 3],
    (text, backspace): (&str, bool),
    (wheel, opened, now): (i32, bool, f64),
) -> [bool; 2] {
    let choices = ui.choices.len();
    let mut consumed = [false; 2];
    let pressed = buttons.pressed.contains(&true);
    match ui.screen {
        McScreen::Enchanting => {
            if pressed
                && let Some(index) = mouse.and_then(enchant_row_at)
                && ui.choices.get(index).is_some_and(|choice| choice.enabled)
            {
                ui.clicks.push(McClick::Choose(index));
                consumed = buttons.pressed;
            }
            if choices > 0 {
                let step = i32::from(next) - i32::from(previous);
                input.choice_focus = (input.choice_focus as i32 + step).rem_euclid(choices.min(3) as i32) as usize;
                if confirm && ui.choices.get(input.choice_focus).is_some_and(|choice| choice.enabled) {
                    ui.clicks.push(McClick::Choose(input.choice_focus));
                }
            }
        }
        McScreen::Stonecutter => {
            let active = choices > RECIPE_COLUMNS * RECIPE_ROWS;
            let start = start_index(input.scroll, choices);
            if pressed && let Some(rel) = mouse {
                if let Some(index) = recipe_at(rel, start, choices) {
                    ui.clicks.push(McClick::Choose(index));
                    input.choice_focus = index;
                    consumed = buttons.pressed;
                } else if active && over_scroller(rel) {
                    input.scrolling = true;
                }
            }
            if buttons.released.contains(&true) || !buttons.held.contains(&true) {
                input.scrolling = false;
            }
            if input.scrolling
                && active
                && let Some(rel) = mouse
            {
                input.scroll = drag_offset(rel[1]);
            }
            if wheel != 0 && active {
                input.scroll = (input.scroll + wheel as f32 / hidden_rows(choices) as f32).clamp(0.0, 1.0);
            }
            if choices > 0 {
                let step = i32::from(next) - i32::from(previous);
                if step != 0 {
                    input.choice_focus = (input.choice_focus as i32 + step).rem_euclid(choices as i32) as usize;
                    ui.clicks.push(McClick::Choose(input.choice_focus));
                    let hidden = hidden_rows(choices);
                    if hidden > 0 {
                        let row = input.choice_focus / RECIPE_COLUMNS;
                        let first = (start_index(input.scroll, choices) / RECIPE_COLUMNS).clamp(row.saturating_sub(RECIPE_ROWS - 1), row).min(hidden);
                        input.scroll = first as f32 / hidden as f32;
                    }
                }
            } else {
                input.scroll = 0.0;
            }
        }
        _ => {}
    }
    if ui.screen != McScreen::Anvil {
        input.name_for = None;
        return consumed;
    }
    let left = ui.container.first().cloned().flatten().map(|stack| stack.id);
    if opened || left != input.name_for {
        input.name = if left.is_some() { ui.name.clone().unwrap_or_else(|| left_name(ui)) } else { String::new() };
        input.name_for = left.clone();
        input.name_from = scroll_to(&gui.font, &input.name, input.name_from, input.name.chars().count());
        input.name_since = now;
    }
    if left.is_none() {
        return consumed;
    }
    let mut changed = false;
    for c in text.chars() {
        if input.name.chars().count() < NAME_LIMIT {
            input.name.push(c);
            changed = true;
        }
    }
    if backspace && input.name.pop().is_some() {
        changed = true;
    }
    if changed {
        input.name_from = scroll_to(&gui.font, &input.name, input.name_from, input.name.chars().count());
        input.name_since = now;
        ui.clicks.push(McClick::Rename(input.name.clone()));
    }
    consumed
}

fn tooltip_lines(
    stack: &McStack,
    ui: &MinecraftUi,
    weapons: Option<&PreparedWeapons>,
    strings: Option<&PreparedLocalizedStrings>,
    gaps: &mut HudPresentationGaps,
) -> Vec<TextLine> {
    let name = stack_name(stack, ui, weapons, strings, gaps);
    let mut lines = vec![TextLine { italic: stack.named, ..TextLine::new(name, crate::minecraft_hotbar::name_color(Some(stack))) }];
    lines.extend(stack.contents.iter().map(|line| TextLine::new(line.clone(), WHITE)));
    lines.extend(stack.more.iter().map(|line| TextLine { italic: true, ..TextLine::new(line.clone(), WHITE) }));
    lines.extend(stack.enchantments.iter().map(|name| TextLine::new(name.clone(), GRAY)));
    let extra = match (stack.weapon, stack.attack) {
        (Some(weapon), _) => weapons
            .and_then(|reg| reg.0.facts_of(weapon))
            .map_or_else(Vec::new, |facts| crate::minecraft_tooltip::gun_lines(&facts)),
        (None, Some(attack)) => crate::minecraft_tooltip::attack_lines(attack),
        (None, None) => Vec::new(),
    };
    if !extra.is_empty() {
        lines.push(TextLine::new("", WHITE));
        lines.extend(extra.into_iter().map(|(text, tone)| {
            let color = match tone {
                crate::minecraft_tooltip::Tone::Value => DARK_GREEN,
                crate::minecraft_tooltip::Tone::Header | crate::minecraft_tooltip::Tone::Plain => GRAY,
            };
            TextLine::new(text, color)
        }));
    }
    lines
}

fn neighbour(slots: &[SlotSpec], from: McSlot, nav: Nav) -> Option<McSlot> {
    let centre = |spec: &SlotSpec| Vec2::new(spec.x as f32 + 8.0, spec.y as f32 + 8.0);
    let origin = centre(slots.iter().find(|spec| spec.slot == from)?);
    let dir = match nav {
        Nav::Up => Vec2::NEG_Y,
        Nav::Down => Vec2::Y,
        Nav::Left => Vec2::NEG_X,
        Nav::Right => Vec2::X,
    };
    slots
        .iter()
        .filter(|spec| spec.slot != from)
        .filter_map(|spec| {
            let delta = centre(spec) - origin;
            let along = delta.dot(dir);
            let across = delta.perp_dot(dir).abs();
            (along > 1.0 && across <= along * 1.5).then_some((along + 2.0 * across, spec.slot))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, slot)| slot)
}

pub(crate) fn minecraft_layers(active: bool, in_world: bool) -> (bool, bool) {
    (active, active || in_world)
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
    let held = |button: GamepadButton| pad.is_some_and(|pad| pad.pressed(button));
    Buttons {
        pressed: [
            mouse.just_pressed(MouseButton::Left) || pressed(GamepadButton::South),
            mouse.just_pressed(MouseButton::Right) || pressed(GamepadButton::West),
        ],
        released: [
            mouse.just_released(MouseButton::Left) || released(GamepadButton::South),
            mouse.just_released(MouseButton::Right) || released(GamepadButton::West),
        ],
        held: [
            mouse.pressed(MouseButton::Left) || held(GamepadButton::South),
            mouse.pressed(MouseButton::Right) || held(GamepadButton::West),
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

fn draw_pad_legend(p: &mut Painter<'_>, layout: &Layout, origin: [i32; 2], style: PromptStyle, carrying: bool, on_result: bool) {
    let entries = [
        (GamepadButton::South, if carrying && !on_result { "Place" } else { "Take" }),
        (GamepadButton::West, if carrying { "Place One" } else { "Split" }),
        (GamepadButton::North, if on_result { "Craft All" } else { "Quick Move" }),
        (GamepadButton::East, "Close"),
    ];
    let parts: Vec<(&str, &str)> = entries.iter().map(|&(button, action)| (pad_label(style, button), action)).collect();
    let total: i32 = parts.iter().map(|(button, action)| p.width(button) + 4 + p.width(action)).sum::<i32>() + 12 * (parts.len() as i32 - 1);
    let mut x = (p.space.width - total) / 2;
    let y = origin[1] + layout.size[1] + 6;
    for (button, action) in parts {
        p.text(button, x, y, YELLOW, true);
        x += p.width(button) + 4;
        p.text(action, x, y, WHITE, true);
        x += p.width(action) + 12;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_chat_shows_while_dead_in_a_minecraft_world() {
        assert_eq!(minecraft_layers(true, true), (true, true));
        assert_eq!(minecraft_layers(false, true), (false, true));
        assert_eq!(minecraft_layers(false, false), (false, false));
    }

    fn font() -> McFont {
        let glyph = frame::McGlyph { rect: [0, 0, 8, 8], scale: 1.0, ascent: 7, advance: 6.0 };
        let mut font = McFont::default();
        for c in 'a'..='z' {
            font.glyphs.insert(c, glyph);
        }
        font
    }

    #[test]
    fn the_name_field_scrolls_to_keep_the_cursor_in_view() {
        let font = font();
        assert_eq!(scroll_to(&font, "abcdefghijklmnopq", 0, 17), 0);
        let long = "abcdefghijklmnopqrstuvwxyz";
        assert_eq!(scroll_to(&font, long, 0, 26), 9);
        assert_eq!(scroll_to(&font, long, 9, 26), 9);
        assert_eq!(scroll_to(&font, "abc", 9, 3), 0);
    }

    #[test]
    fn the_puppet_turns_toward_the_mouse_like_vanilla() {
        let [yaw, pitch, tilt] = puppet_turn_in(PUPPET_BOX, [0, 0], [50, 43]);
        assert!(yaw.abs() < 0.5 && pitch.abs() < 1e-6 && tilt.abs() < 1e-6);
        let [yaw, _, _] = puppet_turn_in(PUPPET_BOX, [0, 0], [90, 43]);
        assert!((yaw - (-39.5f32 / 40.0).atan() * 20.0).abs() < 1e-4);
        let [_, pitch, tilt] = puppet_turn_in(PUPPET_BOX, [0, 0], [50, 3]);
        assert!((tilt - 1.0f32.atan() * 20.0).abs() < 1e-4);
        assert!((pitch + tilt).abs() < 1e-6);
    }

    #[test]
    fn the_pad_moves_to_the_nearest_slot_in_a_direction() {
        let layout = crate::minecraft_screens::layout(McScreen::Inventory);
        assert_eq!(neighbour(&layout.slots, McSlot::Inventory(9), Nav::Right), Some(McSlot::Inventory(10)));
        assert_eq!(neighbour(&layout.slots, McSlot::Inventory(9), Nav::Down), Some(McSlot::Inventory(18)));
        assert_eq!(neighbour(&layout.slots, McSlot::Inventory(0), Nav::Down), None);
    }

    #[test]
    fn a_shulker_box_tooltip_lists_its_stacks_before_the_italic_rest() {
        let stack = McStack {
            id: "minecraft:shulker_box".to_owned(),
            count: 1,
            weapon: None,
            damage: None,
            max: 1,
            named: false,
            name: Some("Shulker Box".to_owned()),
            attack: None,
            enchantments: vec!["Unbreaking I".to_owned()],
            contents: (1..=5).map(|count| format!("{count} x Stone")).collect(),
            more: Some("and 2 more...".to_owned()),
            glint: false,
        };
        let lines = tooltip_lines(&stack, &MinecraftUi::default(), None, None, &mut HudPresentationGaps::default());
        let shown: Vec<(&str, bool)> = lines.iter().map(|line| (line.text.as_str(), line.italic)).collect();
        assert_eq!(
            shown,
            [
                ("Shulker Box", false),
                ("1 x Stone", false),
                ("2 x Stone", false),
                ("3 x Stone", false),
                ("4 x Stone", false),
                ("5 x Stone", false),
                ("and 2 more...", true),
                ("Unbreaking I", false),
            ]
        );
        assert!(lines[1..7].iter().all(|line| line.color == WHITE));
    }
}
