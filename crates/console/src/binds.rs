use input_iw4::{ClientInput, command_id_lookup, command_name, key_event};

use std::collections::{HashMap, HashSet};

use bevy::input::ButtonInput;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseButton, MouseScrollUnit};
use bevy::prelude::Resource;

pub const DEFAULT_CONTROLS: &str = include_str!("../assets/default_controls.cfg");

mod key_names;

pub use key_names::{
    BINDABLE_KEYS, display_button, host_keynum, parse_button_name, parse_key_name,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindButton {
    Key(KeyCode),
    Mouse(MouseButton),
    WheelUp,
    WheelDown,
    Pad(PadButton),
}

impl BindButton {
    pub fn is_pad(self) -> bool {
        matches!(self, Self::Pad(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Select,
    Start,
}

impl PadButton {
    pub const ALL: [Self; 16] = [
        Self::South,
        Self::East,
        Self::West,
        Self::North,
        Self::LeftBumper,
        Self::RightBumper,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::LeftStick,
        Self::RightStick,
        Self::DpadUp,
        Self::DpadDown,
        Self::DpadLeft,
        Self::DpadRight,
        Self::Select,
        Self::Start,
    ];

    pub const fn gamepad_button(self) -> GamepadButton {
        match self {
            Self::South => GamepadButton::South,
            Self::East => GamepadButton::East,
            Self::West => GamepadButton::West,
            Self::North => GamepadButton::North,
            Self::LeftBumper => GamepadButton::LeftTrigger,
            Self::RightBumper => GamepadButton::RightTrigger,
            Self::LeftTrigger => GamepadButton::LeftTrigger2,
            Self::RightTrigger => GamepadButton::RightTrigger2,
            Self::LeftStick => GamepadButton::LeftThumb,
            Self::RightStick => GamepadButton::RightThumb,
            Self::DpadUp => GamepadButton::DPadUp,
            Self::DpadDown => GamepadButton::DPadDown,
            Self::DpadLeft => GamepadButton::DPadLeft,
            Self::DpadRight => GamepadButton::DPadRight,
            Self::Select => GamepadButton::Select,
            Self::Start => GamepadButton::Start,
        }
    }

    pub fn from_gamepad_button(button: GamepadButton) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|pad| pad.gamepad_button() == button)
    }

    pub const fn console_name(self) -> &'static str {
        match self {
            Self::South => "BUTTON_A",
            Self::East => "BUTTON_B",
            Self::West => "BUTTON_X",
            Self::North => "BUTTON_Y",
            Self::LeftBumper => "BUTTON_LSHLDR",
            Self::RightBumper => "BUTTON_RSHLDR",
            Self::LeftTrigger => "BUTTON_LTRIG",
            Self::RightTrigger => "BUTTON_RTRIG",
            Self::LeftStick => "BUTTON_LSTICK",
            Self::RightStick => "BUTTON_RSTICK",
            Self::DpadUp => "DPAD_UP",
            Self::DpadDown => "DPAD_DOWN",
            Self::DpadLeft => "DPAD_LEFT",
            Self::DpadRight => "DPAD_RIGHT",
            Self::Select => "BUTTON_BACK",
            Self::Start => "BUTTON_START",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::South => "A",
            Self::East => "B",
            Self::West => "X",
            Self::North => "Y",
            Self::LeftBumper => "LB",
            Self::RightBumper => "RB",
            Self::LeftTrigger => "LT",
            Self::RightTrigger => "RT",
            Self::LeftStick => "LS",
            Self::RightStick => "RS",
            Self::DpadUp => "D-UP",
            Self::DpadDown => "D-DOWN",
            Self::DpadLeft => "D-LEFT",
            Self::DpadRight => "D-RIGHT",
            Self::Select => "BACK",
            Self::Start => "START",
        }
    }

    pub const fn prompt(self, style: frame::PromptStyle) -> &'static str {
        use frame::PromptStyle::*;
        match (style, self) {
            (Xbox, _) => self.label(),
            (PlayStation, Self::South) => "×",
            (PlayStation, Self::East) => "○",
            (PlayStation, Self::West) => "□",
            (PlayStation, Self::North) => "△",
            (PlayStation, Self::LeftBumper) => "L1",
            (PlayStation, Self::RightBumper) => "R1",
            (PlayStation, Self::LeftTrigger) => "L2",
            (PlayStation, Self::RightTrigger) => "R2",
            (PlayStation, Self::LeftStick) => "L3",
            (PlayStation, Self::RightStick) => "R3",
            (PlayStation, Self::Select) => "SHARE",
            (PlayStation, Self::Start) => "OPTIONS",
            (Generic, Self::South) => "SOUTH",
            (Generic, Self::East) => "EAST",
            (Generic, Self::West) => "WEST",
            (Generic, Self::North) => "NORTH",
            (Generic, Self::LeftBumper) => "L-BUMPER",
            (Generic, Self::RightBumper) => "R-BUMPER",
            (Generic, Self::LeftTrigger) => "L-TRIGGER",
            (Generic, Self::RightTrigger) => "R-TRIGGER",
            (Generic, Self::LeftStick) => "L-STICK",
            (Generic, Self::RightStick) => "R-STICK",
            (Generic, Self::Select) => "SELECT",
            _ => self.label(),
        }
    }

    const fn keynum(self) -> usize {
        190 + self as usize
    }
}

pub(crate) fn gameplay_binding(button: BindButton, command: u32, akimbo: bool) -> u32 {
    if !akimbo || !button.is_pad() {
        return command;
    }
    let mapped = match command_name(command) {
        Some("+attack") => "+speed_throw",
        Some("+speed_throw") => "+attack",
        _ => return command,
    };
    command_id_lookup(mapped).expect("built-in controller action")
}

pub fn pad_layout(layout: usize) -> Vec<(PadButton, &'static str)> {
    use PadButton::*;
    let mut binds = vec![
        (RightTrigger, "+attack"),
        (LeftTrigger, "+speed_throw"),
        (RightBumper, "+frag"),
        (LeftBumper, "+smoke"),
        (South, "+gostand"),
        (East, "+stance"),
        (West, "+usereload"),
        (North, "weapnext"),
        (LeftStick, "+breath_sprint"),
        (RightStick, "+melee"),
        (DpadUp, "+actionslot 1"),
        (DpadDown, "+actionslot 2"),
        (DpadLeft, "+actionslot 3"),
        (DpadRight, "+actionslot 4"),
        (Select, "+scores"),
    ];
    let mut set = |button: PadButton, command: &'static str| {
        binds.retain(|(b, _)| *b != button);
        binds.push((button, command));
    };
    let tactical = |set: &mut dyn FnMut(PadButton, &'static str)| {
        set(East, "+melee");
        set(RightStick, "+stance");
    };
    match layout {
        1 => tactical(&mut set),
        2 => {
            set(LeftTrigger, "+attack");
            set(RightTrigger, "+speed_throw");
            set(LeftBumper, "+frag");
            set(RightBumper, "+smoke");
        }
        3 | 4 => {
            set(LeftBumper, "+gostand");
            set(South, "+smoke");
            if layout == 4 {
                tactical(&mut set);
            }
        }
        _ => {}
    }
    binds
}

pub(crate) fn wheel_button(y: f32) -> Option<BindButton> {
    if y > 0.0 {
        Some(BindButton::WheelUp)
    } else if y < 0.0 {
        Some(BindButton::WheelDown)
    } else {
        None
    }
}

pub(crate) fn wheel_detents(unit: MouseScrollUnit, y: f32, carry: &mut f32) -> i32 {
    if !y.is_finite() {
        return 0;
    }
    let delta = match unit {
        MouseScrollUnit::Line => y,
        MouseScrollUnit::Pixel => y / 100.0,
    };
    *carry = (*carry + delta).clamp(-32.0, 32.0);
    let detents = carry.trunc() as i32;
    *carry -= detents as f32;
    detents
}

pub struct BindInputs<'a> {
    pub keys: &'a ButtonInput<KeyCode>,
    pub mouse: &'a ButtonInput<MouseButton>,
    pub pad: Option<&'a Gamepad>,
}

impl<'a> BindInputs<'a> {
    pub fn new(keys: &'a ButtonInput<KeyCode>, mouse: &'a ButtonInput<MouseButton>) -> Self {
        Self {
            keys,
            mouse,
            pad: None,
        }
    }

    pub fn with_pad(mut self, pad: Option<&'a Gamepad>) -> Self {
        self.pad = pad;
        self
    }

    pub fn pressed(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.pressed(key),
            BindButton::Mouse(btn) => self.mouse.pressed(btn),
            BindButton::WheelUp | BindButton::WheelDown => false,
            BindButton::Pad(btn) => self
                .pad
                .is_some_and(|pad| pad.pressed(btn.gamepad_button())),
        }
    }

    pub fn just_pressed(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.just_pressed(key),
            BindButton::Mouse(btn) => self.mouse.just_pressed(btn),
            BindButton::WheelUp | BindButton::WheelDown => false,
            BindButton::Pad(btn) => self
                .pad
                .is_some_and(|pad| pad.just_pressed(btn.gamepad_button())),
        }
    }

    pub fn just_released(&self, button: BindButton) -> bool {
        match button {
            BindButton::Key(key) => self.keys.just_released(key),
            BindButton::Mouse(btn) => self.mouse.just_released(btn),
            BindButton::WheelUp | BindButton::WheelDown => false,
            BindButton::Pad(btn) => self
                .pad
                .is_some_and(|pad| pad.just_released(btn.gamepad_button())),
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct KeyBinds {
    map: HashMap<BindButton, u32>,
    minecraft: HashMap<BindButton, u32>,
}

impl KeyBinds {
    pub fn apply_defaults(&mut self) {
        self.clear_all();
        let _ = self.apply_script(DEFAULT_CONTROLS);
        self.apply_pad_layout(0);
    }

    pub fn apply_minecraft_defaults(&mut self) {
        let mut defaults = KeyBinds::default();
        let _ = defaults.apply_config_script(DEFAULT_CONTROLS);
        for (button, id) in defaults.minecraft {
            if !self.minecraft.values().any(|bound| *bound == id) {
                self.minecraft.entry(button).or_insert(id);
            }
        }
    }

    pub fn move_old_grenade_keys(&mut self) -> bool {
        let f = BindButton::Key(KeyCode::KeyF);
        let g = BindButton::Key(KeyCode::KeyG);
        let b = BindButton::Key(KeyCode::KeyB);
        let (Some(frag), Some(smoke)) = (command_id_lookup("+frag"), command_id_lookup("+smoke"))
        else {
            return false;
        };
        let keys = |id: u32| -> Vec<BindButton> {
            self.map
                .iter()
                .filter(|(button, bound)| **bound == id && !button.is_pad())
                .map(|(button, _)| *button)
                .collect()
        };
        if keys(frag) != [f]
            || keys(smoke) != [g]
            || self.map.contains_key(&b)
            || self.minecraft.contains_key(&b)
        {
            return false;
        }
        self.map.remove(&f);
        self.map.insert(g, frag);
        self.map.insert(b, smoke);
        true
    }

    pub fn apply_script(&mut self, script: &str) -> Vec<String> {
        self.apply_script_inner(script, true)
    }

    pub(crate) fn apply_config_script(&mut self, script: &str) -> Vec<String> {
        self.apply_script_inner(script, false)
    }

    fn apply_script_inner(&mut self, script: &str, echo_success: bool) -> Vec<String> {
        let mut output = Vec::new();
        for raw in script.split([';', '\n']) {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let Some(command) = crate::ConsoleCommand::parse(line) else {
                continue;
            };
            match command.name.as_str() {
                "bind" => match self.cmd_bind(&command.args) {
                    Ok(Some(msg)) if echo_success => output.push(msg),
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(msg) => output.push(msg),
                },
                "unbind" => match self.cmd_unbind(&command.args) {
                    Ok(Some(msg)) if echo_success => output.push(msg),
                    Ok(Some(_)) => {}
                    Ok(None) => {}
                    Err(msg) => output.push(msg),
                },
                "unbindall" => {
                    self.clear_all();
                    if echo_success {
                        output.push("unbindall".into());
                    }
                }
                other => output.push(format!("unknown bind-script command `{other}`")),
            }
        }
        output
    }

    fn layer(&mut self, id: u32) -> &mut HashMap<BindButton, u32> {
        if input_iw4::is_minecraft_command(id) {
            &mut self.minecraft
        } else {
            &mut self.map
        }
    }

    pub fn set(&mut self, button: BindButton, id: u32) {
        self.layer(id).insert(button, id);
    }

    pub fn clear_button(&mut self, button: BindButton) -> bool {
        let minecraft = self.minecraft.remove(&button).is_some();
        self.map.remove(&button).is_some() || minecraft
    }

    pub fn clear_command(&mut self, id: u32) -> bool {
        let layer = self.layer(id);
        let before = layer.len();
        layer.retain(|_, bound| *bound != id);
        layer.len() != before
    }

    pub fn clear_all(&mut self) {
        self.map.clear();
        self.minecraft.clear();
    }

    pub fn clear_command_on(&mut self, id: u32, pad: bool) -> bool {
        let layer = self.layer(id);
        let before = layer.len();
        layer.retain(|button, bound| *bound != id || button.is_pad() != pad);
        layer.len() != before
    }

    pub fn apply_pad_layout(&mut self, layout: usize) {
        self.map.retain(|button, _| !button.is_pad());
        for (button, command) in pad_layout(layout) {
            if let Some(id) = command_id_lookup(command) {
                self.set(BindButton::Pad(button), id);
            }
        }
    }

    pub fn has_pad_binds(&self) -> bool {
        self.map.keys().any(|button| button.is_pad())
    }

    pub fn get(&self, button: BindButton) -> Option<u32> {
        self.map.get(&button).copied()
    }

    pub fn binding_name(&self, button: BindButton) -> Option<&'static str> {
        self.get(button).and_then(command_name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (BindButton, u32)> + '_ {
        self.map.iter().map(|(b, id)| (*b, *id))
    }

    pub fn iter_minecraft(&self) -> impl Iterator<Item = (BindButton, u32)> + '_ {
        self.minecraft.iter().map(|(b, id)| (*b, *id))
    }

    pub fn minecraft_binding(&self, button: BindButton) -> Option<&'static str> {
        self.minecraft.get(&button).copied().and_then(command_name)
    }

    pub fn list_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .map
            .iter()
            .chain(&self.minecraft)
            .filter_map(|(button, id)| {
                command_name(*id).map(|name| format!("bind {} {name}", display_button(*button)))
            })
            .collect();
        lines.sort();
        lines.dedup();
        lines
    }

    fn cmd_bind(&mut self, args: &[String]) -> Result<Option<String>, String> {
        match args {
            [] => Ok(None),
            [key] => {
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                let names: Vec<&str> = buttons
                    .iter()
                    .filter_map(|button| self.binding_name(*button))
                    .collect();
                if names.is_empty() {
                    Ok(Some(format!("`{key}` is unbound")))
                } else {
                    Ok(Some(format!("bind {key} {}", names[0])))
                }
            }
            [key, action @ ..] => {
                let action = action.join(" ");
                let id = command_id_lookup(&action)
                    .ok_or_else(|| format!("unknown command `{action}`"))?;
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                if input_iw4::is_minecraft_command(id)
                    && buttons
                        .iter()
                        .any(|button| matches!(button, BindButton::WheelUp | BindButton::WheelDown))
                {
                    return Err(format!("`{action}` can't be bound to the mouse wheel"));
                }
                for button in buttons {
                    self.set(button, id);
                }
                let name = command_name(id).unwrap_or(action.as_str());
                Ok(Some(format!("bind {key} {name}")))
            }
        }
    }

    fn cmd_unbind(&mut self, args: &[String]) -> Result<Option<String>, String> {
        match args {
            [key] => {
                let buttons =
                    parse_button_name(key).ok_or_else(|| format!("unknown key `{key}`"))?;
                let mut any = false;
                for button in buttons {
                    any |= self.clear_button(button);
                }
                if any {
                    Ok(Some(format!("unbind {key}")))
                } else {
                    Ok(Some(format!("`{key}` is unbound")))
                }
            }
            _ => Err("usage: unbind <key>".into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MinecraftKeyState {
    pub(crate) active: bool,
    pub(crate) use_hint: bool,
    pub(crate) screen_open: bool,
}

pub(crate) fn minecraft_presses(
    binds: &KeyBinds,
    state: MinecraftKeyState,
    just_pressed: impl Fn(BindButton) -> bool,
    held: impl Fn(BindButton) -> bool,
) -> (HashSet<BindButton>, frame::McActionInput) {
    let mut suppressed = HashSet::new();
    let mut presses = frame::McActionInput::default();
    if !state.active {
        return (suppressed, presses);
    }
    for (button, id) in binds.iter_minecraft() {
        let Some(action) = command_name(id).and_then(frame::McAction::from_command) else {
            continue;
        };
        presses.bind(action);
        let use_wins = !state.screen_open
            && state.use_hint
            && matches!(binds.binding_name(button), Some("+activate" | "+usereload"));
        let throw_keeps_key = !state.screen_open
            && matches!(binds.binding_name(button), Some("+frag" | "+smoke"));
        if use_wins || throw_keeps_key {
            continue;
        }
        suppressed.insert(button);
        if just_pressed(button) {
            presses.press(action);
        }
        if !state.screen_open && (held(button) || just_pressed(button)) {
            presses.hold(action);
        }
    }
    let dpad_down = BindButton::Pad(PadButton::DpadDown);
    if !state.screen_open && (held(dpad_down) || just_pressed(dpad_down)) {
        presses.hold(frame::McAction::SwapHands);
    }
    (suppressed, presses)
}

pub(crate) fn pulse_wheel_binding(
    binds: &KeyBinds,
    client: &mut ClientInput,
    button: BindButton,
    now_msec: i32,
    frame_msec: u32,
) {
    let Some(id) = binds.get(button) else { return };
    let key_num = host_keynum(button);
    client.keys[key_num].binding = id;
    key_event(client, key_num, true, now_msec, frame_msec);
    key_event(client, key_num, false, now_msec, frame_msec);
}

#[cfg(test)]
mod function_key_tests {
    use super::*;

    const FUNCTION_KEYS: [(KeyCode, &str); 12] = [
        (KeyCode::F1, "F1"),
        (KeyCode::F2, "F2"),
        (KeyCode::F3, "F3"),
        (KeyCode::F4, "F4"),
        (KeyCode::F5, "F5"),
        (KeyCode::F6, "F6"),
        (KeyCode::F7, "F7"),
        (KeyCode::F8, "F8"),
        (KeyCode::F9, "F9"),
        (KeyCode::F10, "F10"),
        (KeyCode::F11, "F11"),
        (KeyCode::F12, "F12"),
    ];

    #[test]
    fn function_keys_parse_in_either_case_and_display_upper_case() {
        for (key, name) in FUNCTION_KEYS {
            assert_eq!(parse_button_name(name), Some(vec![BindButton::Key(key)]));
            assert_eq!(
                parse_button_name(&name.to_ascii_lowercase()),
                Some(vec![BindButton::Key(key)])
            );
            assert_eq!(display_button(BindButton::Key(key)), name);
            assert!(BINDABLE_KEYS.contains(&name.to_ascii_lowercase().as_str()));
        }
    }

    #[test]
    fn every_keyboard_key_has_its_own_host_key_number() {
        let mut buttons: Vec<BindButton> = BINDABLE_KEYS
            .iter()
            .filter_map(|name| parse_button_name(name))
            .flatten()
            .filter(|button| matches!(button, BindButton::Key(_)))
            .collect();
        buttons.sort_by_key(|button| host_keynum(*button));
        buttons.dedup();
        for (key, _) in FUNCTION_KEYS {
            assert!(buttons.contains(&BindButton::Key(key)));
            assert_ne!(host_keynum(BindButton::Key(key)), 62);
        }
        for pair in buttons.windows(2) {
            assert_ne!(
                host_keynum(pair[0]),
                host_keynum(pair[1]),
                "{:?} and {:?} share a host key number",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn a_function_key_binding_is_listed_and_loads_back() {
        let mut binds = KeyBinds::default();
        let _ = binds.apply_script("bind F5 +scores");
        let lines = binds.list_lines();
        assert_eq!(lines, vec!["bind F5 +scores".to_owned()]);
        let mut loaded = KeyBinds::default();
        let _ = loaded.apply_config_script(&lines.join("\n"));
        assert_eq!(
            loaded.get(BindButton::Key(KeyCode::F5)),
            command_id_lookup("+scores")
        );
    }
}

#[cfg(test)]
mod minecraft_bind_tests {
    use super::*;
    use frame::{McAction, McActionInput};

    const E: BindButton = BindButton::Key(KeyCode::KeyE);
    const F: BindButton = BindButton::Key(KeyCode::KeyF);
    const G: BindButton = BindButton::Key(KeyCode::KeyG);
    const B: BindButton = BindButton::Key(KeyCode::KeyB);
    const X: BindButton = BindButton::Key(KeyCode::KeyX);

    fn id(name: &str) -> u32 {
        command_id_lookup(name).expect("known command")
    }

    fn defaults() -> KeyBinds {
        let mut binds = KeyBinds::default();
        binds.apply_defaults();
        binds
    }

    fn state(use_hint: bool, screen_open: bool) -> MinecraftKeyState {
        MinecraftKeyState {
            active: true,
            use_hint,
            screen_open,
        }
    }

    fn press(
        binds: &KeyBinds,
        state: MinecraftKeyState,
        key: BindButton,
    ) -> (HashSet<BindButton>, McActionInput) {
        minecraft_presses(binds, state, |button| button == key, |button| button == key)
    }

    #[test]
    fn the_defaults_bind_the_vanilla_minecraft_keys_beside_the_mw2_ones() {
        let binds = defaults();
        let expected = [
            (KeyCode::KeyE, "mc_inventory"),
            (KeyCode::KeyQ, "mc_drop"),
            (KeyCode::KeyF, "mc_swaphands"),
            (KeyCode::KeyT, "mc_chat"),
            (KeyCode::Slash, "mc_command"),
            (KeyCode::F5, "mc_perspective"),
        ];
        for (key, command) in expected {
            assert_eq!(binds.minecraft_binding(BindButton::Key(key)), Some(command));
        }
        assert_eq!(binds.iter_minecraft().count(), expected.len());
        assert_eq!(binds.binding_name(E), Some("+activate"));
        assert_eq!(binds.binding_name(F), None);
        assert!(binds.iter().all(|(_, id)| !input_iw4::is_minecraft_command(id)));
    }

    #[test]
    fn the_defaults_put_the_grenades_on_g_and_b_and_leave_f_to_swap_hands() {
        let binds = defaults();
        assert_eq!(binds.binding_name(G), Some("+frag"));
        assert_eq!(binds.binding_name(B), Some("+smoke"));
        assert_eq!(binds.minecraft_binding(G), None);
        assert_eq!(binds.minecraft_binding(B), None);
        assert_eq!(binds.binding_name(F), None);
        assert_eq!(binds.minecraft_binding(F), Some("mc_swaphands"));
        let keys = |command: &str| {
            let mut keys: Vec<BindButton> = binds
                .iter()
                .filter(|(button, bound)| *bound == id(command) && !button.is_pad())
                .map(|(button, _)| button)
                .collect();
            keys.sort_by_key(|button| host_keynum(*button));
            keys
        };
        assert_eq!(keys("+frag"), vec![G]);
        assert_eq!(keys("+smoke"), vec![B]);
        assert_eq!(
            binds.binding_name(BindButton::Pad(PadButton::RightBumper)),
            Some("+frag")
        );
        assert_eq!(
            binds.binding_name(BindButton::Pad(PadButton::LeftBumper)),
            Some("+smoke")
        );
        let mut moved = binds.clone();
        assert!(!moved.move_old_grenade_keys());
        assert_eq!(moved.list_lines(), binds.list_lines());
    }

    #[test]
    fn the_old_grenade_pair_moves_once_and_only_when_untouched() {
        let old = "bind f +frag\nbind g +smoke\nbind f mc_swaphands\nbind BUTTON_RSHLDR +frag";
        let mut binds = KeyBinds::default();
        let _ = binds.apply_config_script(old);
        assert!(binds.move_old_grenade_keys());
        assert_eq!(binds.binding_name(G), Some("+frag"));
        assert_eq!(binds.binding_name(B), Some("+smoke"));
        assert_eq!(binds.binding_name(F), None);
        assert_eq!(binds.minecraft_binding(F), Some("mc_swaphands"));
        assert_eq!(
            binds.binding_name(BindButton::Pad(PadButton::RightBumper)),
            Some("+frag")
        );
        let once = binds.list_lines();
        assert!(!binds.move_old_grenade_keys());
        assert_eq!(binds.list_lines(), once);
        for customised in [
            "bind mouse4 +frag",
            "bind b +melee",
            "bind b mc_drop",
            "bind h +smoke",
            "unbind g",
        ] {
            let mut binds = KeyBinds::default();
            let _ = binds.apply_config_script(old);
            let _ = binds.apply_config_script(customised);
            let before = binds.list_lines();
            assert!(!binds.move_old_grenade_keys(), "{customised}");
            assert_eq!(binds.list_lines(), before, "{customised}");
        }
    }

    #[test]
    fn rebinding_swap_hands_leaves_the_mw2_key_alone() {
        let mut binds = defaults();
        binds.set(F, id("+melee"));
        let swap = id("mc_swaphands");
        binds.clear_command_on(swap, false);
        binds.set(X, swap);
        assert_eq!(binds.minecraft_binding(X), Some("mc_swaphands"));
        assert_eq!(binds.minecraft_binding(F), None);
        assert_eq!(binds.binding_name(F), Some("+melee"));
    }

    #[test]
    fn unbinding_a_key_clears_both_layers() {
        let mut binds = defaults();
        let _ = binds.apply_script("unbind E");
        assert_eq!(binds.binding_name(E), None);
        assert_eq!(binds.minecraft_binding(E), None);
        let _ = binds.apply_script("unbindall");
        assert_eq!(binds.iter().count() + binds.iter_minecraft().count(), 0);
    }

    #[test]
    fn both_layers_are_listed_and_load_back() {
        let binds = defaults();
        let lines = binds.list_lines();
        assert!(lines.contains(&"bind e +activate".to_owned()));
        assert!(lines.contains(&"bind e mc_inventory".to_owned()));
        assert!(lines.contains(&"bind F5 mc_perspective".to_owned()));
        assert!(lines.contains(&"bind SLASH mc_command".to_owned()));
        let mut loaded = KeyBinds::default();
        let _ = loaded.apply_config_script(&lines.join("\n"));
        assert_eq!(loaded.list_lines(), lines);
        assert_eq!(loaded.binding_name(E), Some("+activate"));
        assert_eq!(loaded.minecraft_binding(E), Some("mc_inventory"));
    }

    #[test]
    fn minecraft_actions_refuse_the_mouse_wheel() {
        let mut binds = defaults();
        let output = binds.apply_script("bind MWHEELUP mc_drop");
        assert!(output.iter().any(|line| line.contains("mouse wheel")));
        assert_eq!(binds.minecraft_binding(BindButton::WheelUp), None);
        assert_eq!(binds.minecraft_binding(BindButton::Key(KeyCode::KeyQ)), Some("mc_drop"));
    }

    #[test]
    fn minecraft_defaults_fill_only_unbound_actions() {
        let mut binds = KeyBinds::default();
        let _ = binds.apply_config_script("bind e +activate\nbind x mc_swaphands");
        binds.apply_minecraft_defaults();
        assert_eq!(binds.minecraft_binding(X), Some("mc_swaphands"));
        assert_eq!(binds.minecraft_binding(F), None);
        assert_eq!(binds.minecraft_binding(E), Some("mc_inventory"));
        assert_eq!(binds.binding_name(E), Some("+activate"));
        assert_eq!(binds.iter_minecraft().count(), 6);
    }

    #[test]
    fn minecraft_keys_do_nothing_off_the_minecraft_map() {
        let binds = defaults();
        let off = MinecraftKeyState::default();
        let (suppressed, presses) = press(&binds, off, E);
        assert!(suppressed.is_empty());
        assert_eq!(presses, McActionInput::default());
    }

    #[test]
    fn the_inventory_key_takes_e_from_mw2_use_unless_a_use_hint_shows() {
        let binds = defaults();
        let (suppressed, presses) = press(&binds, state(false, false), E);
        assert!(suppressed.contains(&E));
        assert!(presses.just_pressed(McAction::Inventory));
        let (suppressed, presses) = press(&binds, state(true, false), E);
        assert!(!suppressed.contains(&E));
        assert!(!presses.just_pressed(McAction::Inventory));
        let (suppressed, presses) = press(&binds, state(true, true), E);
        assert!(suppressed.contains(&E));
        assert!(presses.just_pressed(McAction::Inventory));
    }

    #[test]
    fn a_grenade_key_keeps_its_throw_outside_minecraft_screens() {
        for grenade in ["+frag", "+smoke"] {
            let mut binds = defaults();
            binds.set(F, id(grenade));
            let takes = |use_hint, screen_open| {
                let (suppressed, presses) = press(&binds, state(use_hint, screen_open), F);
                assert_eq!(suppressed.contains(&F), presses.just_pressed(McAction::SwapHands));
                suppressed.contains(&F)
            };
            assert!(!takes(false, false), "{grenade}");
            assert!(!takes(true, false), "{grenade}");
            assert!(takes(false, true), "{grenade}");
            assert!(takes(true, true), "{grenade}");
            let (suppressed, _) = press(&binds, MinecraftKeyState::default(), F);
            assert!(suppressed.is_empty());
        }
        let mut binds = defaults();
        binds.set(F, id("+melee"));
        let (suppressed, presses) = press(&binds, state(false, false), F);
        assert!(suppressed.contains(&F));
        assert!(presses.just_pressed(McAction::SwapHands));
    }

    #[test]
    fn a_use_hint_keeps_swap_hands_off_a_key_bound_to_mw2_use() {
        let mut binds = defaults();
        binds.set(F, id("+activate"));
        let (suppressed, presses) = press(&binds, state(true, false), F);
        assert!(!suppressed.contains(&F));
        assert!(!presses.just_pressed(McAction::SwapHands));
        let (suppressed, presses) = press(&binds, state(true, true), F);
        assert!(suppressed.contains(&F));
        assert!(presses.just_pressed(McAction::SwapHands));
        let (suppressed, presses) = press(&binds, state(false, false), F);
        assert!(suppressed.contains(&F));
        assert!(presses.just_pressed(McAction::SwapHands));
    }

    #[test]
    fn a_rebound_swap_hands_key_follows_the_same_rule() {
        let mut binds = defaults();
        let swap = id("mc_swaphands");
        binds.clear_command_on(swap, false);
        binds.set(X, swap);
        let (suppressed, presses) = press(&binds, state(false, false), X);
        assert!(suppressed.contains(&X) && !suppressed.contains(&F));
        assert!(presses.just_pressed(McAction::SwapHands));
        binds.set(X, id("+smoke"));
        let (suppressed, presses) = press(&binds, state(false, false), X);
        assert!(!suppressed.contains(&X));
        assert!(!presses.just_pressed(McAction::SwapHands));
        let (suppressed, presses) = press(&binds, state(false, true), X);
        assert!(suppressed.contains(&X));
        assert!(presses.just_pressed(McAction::SwapHands));
    }

    #[test]
    fn only_just_pressed_keys_fire_their_action() {
        let binds = defaults();
        let (suppressed, presses) = minecraft_presses(&binds, state(false, false), |_| false, |_| false);
        assert_eq!(suppressed.len(), 6);
        assert!(suppressed.contains(&F) && !suppressed.contains(&G) && !suppressed.contains(&B));
        assert!(McAction::ALL.iter().all(|a| !presses.just_pressed(*a)));
        let q = BindButton::Key(KeyCode::KeyQ);
        let (_, presses) = press(&binds, state(false, false), q);
        assert!(presses.just_pressed(McAction::Drop));
        assert!(McAction::ALL
            .iter()
            .filter(|a| **a != McAction::Drop)
            .all(|a| !presses.just_pressed(*a)));
    }

    #[test]
    fn a_held_swap_hands_key_stays_held_until_released() {
        let binds = defaults();
        let holding = |binds: &KeyBinds, state, key: BindButton| {
            let (_, presses) = minecraft_presses(binds, state, |_| false, |button| button == key);
            assert!(!presses.just_pressed(McAction::SwapHands));
            presses.is_held(McAction::SwapHands)
        };
        assert!(holding(&binds, state(false, false), F));
        assert!(holding(&binds, state(true, false), F));
        assert!(!holding(&binds, state(false, false), B));
        assert!(!holding(&binds, state(false, false), G));
        assert!(!holding(&binds, state(false, true), F));
        assert!(!holding(&binds, MinecraftKeyState::default(), F));
        let (_, presses) = minecraft_presses(&binds, state(false, false), |_| false, |_| false);
        assert!(McAction::ALL.iter().all(|a| !presses.is_held(*a)));
        let mut thrown = defaults();
        thrown.set(F, id("+smoke"));
        assert!(!holding(&thrown, state(false, false), F));
    }

    #[test]
    fn holding_the_pads_d_pad_down_holds_swap_hands_but_its_tactical_button_does_not() {
        let binds = defaults();
        let held = |state, button: PadButton| {
            let pad = BindButton::Pad(button);
            let (suppressed, presses) =
                minecraft_presses(&binds, state, |_| false, |b| b == pad);
            assert!(!suppressed.contains(&pad));
            presses.is_held(McAction::SwapHands)
        };
        assert!(held(state(false, false), PadButton::DpadDown));
        assert!(!held(state(false, true), PadButton::DpadDown));
        assert!(!held(MinecraftKeyState::default(), PadButton::DpadDown));
        assert!(!held(state(false, false), PadButton::LeftBumper));
        assert!(!held(state(false, false), PadButton::RightBumper));
        assert_eq!(
            binds.binding_name(BindButton::Pad(PadButton::LeftBumper)),
            Some("+smoke")
        );
    }

    #[test]
    fn a_swap_hands_tap_inside_one_frame_still_holds_for_that_frame() {
        let binds = defaults();
        let tapped = |state, key: BindButton| {
            let (_, presses) = minecraft_presses(&binds, state, |button| button == key, |_| false);
            presses.is_held(McAction::SwapHands)
        };
        for key in [F, BindButton::Pad(PadButton::DpadDown)] {
            assert!(tapped(state(false, false), key));
            assert!(!tapped(state(false, true), key));
            assert!(!tapped(MinecraftKeyState::default(), key));
        }
        assert!(!tapped(state(false, false), B));
        assert!(!tapped(state(false, false), G));
    }

    #[test]
    fn only_minecraft_actions_with_a_key_count_as_bound() {
        let mut binds = defaults();
        let (_, presses) = minecraft_presses(&binds, state(false, false), |_| false, |_| false);
        assert!(McAction::ALL.iter().all(|a| presses.is_bound(*a)));
        binds.clear_command_on(id("mc_command"), false);
        let (_, presses) = minecraft_presses(&binds, state(false, false), |_| false, |_| false);
        assert!(!presses.is_bound(McAction::Command));
        assert!(presses.is_bound(McAction::Chat));
        let (_, presses) = press(&binds, MinecraftKeyState::default(), E);
        assert!(!presses.is_bound(McAction::Chat));
    }
}
