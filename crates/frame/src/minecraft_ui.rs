//! The Minecraft map's inventory and hotbar, between the world that owns the
//! vanilla inventory (`render_anim`) and the HUD that draws it and takes the
//! player's clicks (`hud`).
use std::collections::HashMap;

use bevy::prelude::*;

/// Slots of the vanilla player inventory: 0..9 hotbar, 9..36 main, 36..40
/// armor (feet to head), 40 offhand.
pub const MC_INVENTORY_SLOTS: usize = 41;
pub const MC_HOTBAR: usize = 9;
pub const GLINT_STRENGTH: f32 = 0.75;
pub const GLINT_SPEED: f64 = 0.5;
pub const ITEM_GLINT_SCALE: f32 = 8.0;
pub const GLINT_ATLAS_CELLS: f32 = 32.0;

pub fn glint_shift(millis: u64) -> [f32; 2] {
    let t = (millis as f64 * GLINT_SPEED * 8.0) as i64;
    [
        -((t % 110_000) as f32 / 110_000.0),
        (t % 30_000) as f32 / 30_000.0,
    ]
}

pub fn glint_uv(shift: [f32; 2], point: [f32; 2], scale: f32) -> [f32; 2] {
    let (s, c) = 0.174_532_92_f32.sin_cos();
    let [x, y] = [point[0] * scale, point[1] * scale];
    [shift[0] + c * x - s * y, shift[1] + s * x + c * y]
}

/// One stack as the HUD shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct McStack {
    /// The item's id (`minecraft:dirt`), or `iw4:weapon/<index>` for a gun.
    pub id: String,
    pub count: u8,
    /// The MW2 weapon this item is, for a gun.
    pub weapon: Option<u32>,
    pub damage: Option<(u32, u32)>,
    pub max: u8,
    pub named: bool,
    pub name: Option<String>,
    pub attack: Option<[f32; 2]>,
    pub enchantments: Vec<String>,
    pub contents: Vec<String>,
    pub more: Option<String>,
    pub glint: bool,
}

/// A slot of the inventory screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McSlot {
    Inventory(usize),
    Crafting(usize),
    Result,
    Workbench(usize),
    WorkbenchResult,
    Container(usize),
    Output,
    Creative(usize),
    Trash,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum McScreen {
    #[default]
    Inventory,
    CraftingTable,
    Chest {
        rows: u8,
    },
    ShulkerBox,
    Furnace(McOven),
    Brewing,
    Stonecutter,
    Smithing,
    Grindstone,
    Anvil,
    Enchanting,
    Creative(crate::McTabKind),
    Lectern,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum McOven {
    #[default]
    Furnace,
    Smoker,
    BlastFurnace,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McBook {
    pub pages: Vec<String>,
    pub page: usize,
    pub take: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McChoice {
    pub icon: Option<McStack>,
    pub label: String,
    pub cost: u32,
    pub enabled: bool,
    pub runes: String,
}

/// What the player did on the inventory screen, applied with vanilla's
/// container click rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McClick {
    /// A click on a slot; shift moves the stack across.
    Slot { slot: McSlot, right: bool, shift: bool },
    /// A click outside the window: throws the carried stack (or one of it).
    Outside { right: bool },
    /// A double click: gathers matching stacks onto the carried one.
    Gather { slot: McSlot, right: bool },
    /// A number key over a slot: swaps it with that hotbar slot.
    Swap { slot: McSlot, hotbar: usize },
    /// A drag across slots with a carried stack: shares it out.
    Spread { slots: Vec<McSlot>, right: bool },
    Throw { slot: McSlot, whole: bool },
    /// The screen closed: the carried stack and the crafting grid go back.
    Close,
    Choose(usize),
    Rename(String),
    Clone(McSlot),
}

#[derive(Resource, Default)]
pub struct MinecraftUi {
    /// A Minecraft world is in play and the player is alive.
    pub active: bool,
    pub board_allowed: bool,
    /// The match's Minecraft world is still being generated; the loading
    /// screen holds until it is in, so its setup never lands mid-match.
    pub loading_world: bool,
    /// The inventory screen is open (the HUD's to change).
    pub inventory_open: bool,
    pub screen: McScreen,
    pub screen_opened: bool,
    /// The selected hotbar slot.
    pub selected: usize,
    pub slots: Vec<Option<McStack>>,
    pub crafting: [Option<McStack>; 4],
    pub result: Option<McStack>,
    pub workbench: [Option<McStack>; 9],
    pub workbench_result: Option<McStack>,
    pub container: Vec<Option<McStack>>,
    pub output: Option<McStack>,
    pub data: [i32; 4],
    pub droppable: Vec<(McSlot, u8)>,
    pub screen_title: String,
    pub choices: Vec<McChoice>,
    pub chosen: Option<usize>,
    pub cost: Option<u32>,
    pub name: Option<String>,
    /// The stack on the cursor.
    pub cursor: Option<McStack>,
    /// Item icons: one atlas, and each item's rectangle in it (s0 t0 s1 t1).
    pub icons: Option<Handle<Image>>,
    pub icon_rects: HashMap<String, [f32; 4]>,
    pub animated_icons: Option<Handle<Image>>,
    pub animated_icon_rects: HashMap<String, [f32; 4]>,
    pub glint: Option<Handle<Image>>,
    pub gui: Option<std::sync::Arc<crate::McGui>>,
    pub gui_scale: u32,
    pub armor: u8,
    pub cooldowns: Vec<(String, f32)>,
    /// Display names by item id.
    pub names: HashMap<String, String>,
    /// Clicks the HUD took this frame, for the inventory's owner.
    pub clicks: Vec<McClick>,
    /// A hotbar slot the player picked (scroll or number key).
    pub select: Option<usize>,
    /// A drop of the selected stack (`Q`; with the whole stack on control).
    pub drop_selected: Option<bool>,
    pub puppet_turn: [f32; 3],
    /// The character box in window pixels (centre x and y, width, height),
    /// for placing the character.
    pub character_box: Option<[f32; 4]>,
    /// The MW2 gun the hotbar selection asks the player to raise.
    pub weapon_request: Option<u32>,
    /// The selection is not a gun: the hand or a held item shows, and the
    /// gun neither fires nor aims.
    pub holding_item: bool,
    /// The selected slot is empty: MW2's bare hands show, without the gun.
    pub empty_hand: bool,
    /// How far through its swing the hand is, 0 to 1.
    pub hand_swing: f32,
    pub swap_hands: bool,
    pub offhand_usable: bool,
    pub offhand_key: Option<bool>,
    pub offhand_busy: bool,
    pub using_item: bool,
    pub scoping: bool,
    /// The minimap's picture of the world and the map points of its
    /// north-west and south-east corners.
    pub minimap: Option<(Handle<Image>, [f32; 2], [f32; 2])>,
    pub vitals: Option<McVitals>,
    pub game_mode: McGameMode,
    pub fire_frames: Option<(Handle<Image>, u32)>,
    pub killed_by: Option<(u32, String)>,
    pub chat: crate::McChat,
    pub chat_only: bool,
    pub creative: crate::McCreative,
    pub level_loading: Option<McLevelLoading>,
    pub portal_frames: Option<(Handle<Image>, u32)>,
    pub portal_overlay: Option<McPortalOverlay>,
    pub freeze_outline: Option<Handle<Image>>,
    pub spyglass_scope: Option<Handle<Image>>,
    pub overlay_message: Option<McOverlayMessage>,
    pub sleep: Option<McSleep>,
    pub boss_bars: Vec<McBossBar>,
    pub credits: Option<crate::minecraft_credits::McCredits>,
    pub book: Option<McBook>,
    pub ui_sounds: Vec<&'static str>,
    pub pumpkin_blur: Option<Handle<Image>>,
    pub camera_overlay: bool,
}

impl MinecraftUi {
    pub fn chat_usable(&self) -> bool {
        self.active || self.chat_only
    }

    pub fn typing(&self) -> bool {
        self.chat_usable() && self.chat.open
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McBossBar {
    pub name: String,
    pub from: f32,
    pub target: f32,
    pub elapsed: f32,
    pub world_fog: bool,
    pub music: bool,
}

impl McBossBar {
    pub const LERP_SECONDS: f32 = 0.1;

    pub fn new(name: String, progress: f32, world_fog: bool, music: bool) -> Self {
        Self {
            name,
            from: progress,
            target: progress,
            elapsed: Self::LERP_SECONDS,
            world_fog,
            music,
        }
    }

    pub fn progress(&self) -> f32 {
        let t = (self.elapsed / Self::LERP_SECONDS).clamp(0.0, 1.0);
        self.from + t * (self.target - self.from)
    }

    pub fn set_progress(&mut self, progress: f32) {
        if progress != self.target {
            self.from = self.progress();
            self.target = progress;
            self.elapsed = 0.0;
        }
    }

    pub fn advance(&mut self, seconds: f32) {
        self.elapsed += seconds;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct McPortalOverlay {
    pub alpha: f32,
    pub frame: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct McSleep {
    pub timer: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct McSleepCamera {
    pub eye: [f32; 3],
    pub origin: [f32; 3],
    pub angles: [f32; 3],
}

#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct McSleepers {
    pub bodies: Vec<(u32, Mat4)>,
    pub camera: Option<McSleepCamera>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McOverlayMessage {
    pub text: String,
    pub ticks: f32,
}

impl McOverlayMessage {
    pub const TICKS: f32 = 60.0;

    pub fn alpha(&self) -> u32 {
        ((self.ticks * 255.0 / 20.0) as i32).clamp(0, 255) as u32
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum McLoadingReason {
    NetherPortal,
    EndPortal,
    #[default]
    Other,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McLevelLoading {
    pub reason: McLoadingReason,
    pub background: Option<Handle<Image>>,
    pub background_st: [f32; 4],
    pub menu_background: Option<Handle<Image>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum McGameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl McGameMode {
    pub const fn shows_vitals(self) -> bool {
        matches!(self, Self::Survival | Self::Adventure)
    }

    pub const fn shows_hotbar(self) -> bool {
        !matches!(self, Self::Spectator)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McVitals {
    pub air: i32,
    pub max_air: i32,
    pub eye_in_water: bool,
    pub health: f32,
    pub max_health: f32,
    pub absorption: f32,
    pub food: u8,
    pub saturation: f32,
    pub poisoned: bool,
    pub hunger: bool,
    pub regen_index: i32,
    pub blink: bool,
    pub display_health: i32,
    pub tick: u64,
    pub experience_level: u32,
    pub experience_progress: f32,
    pub on_fire: bool,
    pub eye_in_lava: bool,
    pub withered: bool,
    pub frozen: f32,
    pub fully_frozen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum McAction {
    Inventory,
    Drop,
    SwapHands,
    Chat,
    Command,
    Perspective,
    Skate,
}

impl McAction {
    pub const ALL: [Self; 7] = [
        Self::Inventory,
        Self::Drop,
        Self::SwapHands,
        Self::Chat,
        Self::Command,
        Self::Perspective,
        Self::Skate,
    ];

    pub const fn command(self) -> &'static str {
        match self {
            Self::Inventory => "mc_inventory",
            Self::Drop => "mc_drop",
            Self::SwapHands => "mc_swaphands",
            Self::Chat => "mc_chat",
            Self::Command => "mc_command",
            Self::Perspective => "mc_perspective",
            Self::Skate => "mc_skate",
        }
    }

    pub fn from_command(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.command() == name)
    }
}

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct McActionInput {
    pressed: u8,
    bound: u8,
    held: u8,
}

pub fn offhand_key_latch(latched: Option<bool>, held: bool, usable: bool) -> Option<bool> {
    match latched {
        _ if !held => None,
        None => Some(usable),
        latched => latched,
    }
}

impl McActionInput {
    pub fn press(&mut self, action: McAction) {
        self.pressed |= 1 << action as u8;
    }

    pub fn just_pressed(&self, action: McAction) -> bool {
        self.pressed & (1 << action as u8) != 0
    }

    pub fn hold(&mut self, action: McAction) {
        self.held |= 1 << action as u8;
    }

    pub fn is_held(&self, action: McAction) -> bool {
        self.held & (1 << action as u8) != 0
    }

    pub fn bind(&mut self, action: McAction) {
        self.bound |= 1 << action as u8;
    }

    pub fn is_bound(&self, action: McAction) -> bool {
        self.bound & (1 << action as u8) != 0
    }

    pub fn clear(&mut self) {
        self.pressed = 0;
        self.bound = 0;
        self.held = 0;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum McCameraType {
    #[default]
    FirstPerson,
    ThirdPersonBack,
    ThirdPersonFront,
}

impl McCameraType {
    pub const fn cycle(self) -> Self {
        match self {
            Self::FirstPerson => Self::ThirdPersonBack,
            Self::ThirdPersonBack => Self::ThirdPersonFront,
            Self::ThirdPersonFront => Self::FirstPerson,
        }
    }

    pub const fn is_first_person(self) -> bool {
        matches!(self, Self::FirstPerson)
    }

    pub const fn is_mirrored(self) -> bool {
        matches!(self, Self::ThirdPersonFront)
    }
}

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct McCamera {
    pub camera_type: McCameraType,
    pub detached: bool,
}

/// The player's own MW2 body, drawn standing in the inventory's character
/// box and looking towards the mouse.
#[derive(Resource, Default, Clone)]
pub struct InventoryPuppet {
    pub active: bool,
    pub client: u32,
    /// Where the body stands, in world space (in front of the camera).
    pub root: Mat4,
    /// The aim pitch its upper body and head take, in degrees.
    pub pitch: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
    }

    #[test]
    fn every_minecraft_action_has_its_own_command() {
        for action in McAction::ALL {
            assert_eq!(McAction::from_command(action.command()), Some(action));
        }
        assert_eq!(McAction::from_command("mc_skate"), Some(McAction::Skate));
        assert_eq!(McAction::from_command("+frag"), None);
        assert_eq!(McAction::from_command(""), None);
    }

    #[test]
    fn minecraft_action_presses_last_until_cleared() {
        let mut input = McActionInput::default();
        assert!(McAction::ALL.iter().all(|a| !input.just_pressed(*a)));
        input.press(McAction::SwapHands);
        input.press(McAction::Perspective);
        input.bind(McAction::Command);
        assert!(input.is_bound(McAction::Command) && !input.is_bound(McAction::Chat));
        assert!(!input.just_pressed(McAction::Command));
        for action in McAction::ALL {
            assert_eq!(
                input.just_pressed(action),
                matches!(action, McAction::SwapHands | McAction::Perspective)
            );
        }
        input.clear();
        assert_eq!(input, McActionInput::default());
    }

    #[test]
    fn a_held_minecraft_key_is_held_apart_from_its_press() {
        let mut input = McActionInput::default();
        input.hold(McAction::SwapHands);
        assert!(input.is_held(McAction::SwapHands));
        assert!(!input.just_pressed(McAction::SwapHands));
        assert!(McAction::ALL
            .iter()
            .filter(|a| **a != McAction::SwapHands)
            .all(|a| !input.is_held(*a)));
        input.clear();
        assert_eq!(input, McActionInput::default());
    }

    #[test]
    fn the_offhand_key_latches_what_it_could_do_when_pressed() {
        assert_eq!(offhand_key_latch(None, false, true), None);
        assert_eq!(offhand_key_latch(None, true, true), Some(true));
        assert_eq!(offhand_key_latch(Some(true), true, false), Some(true));
        assert_eq!(offhand_key_latch(Some(true), false, true), None);
        assert_eq!(offhand_key_latch(None, true, false), Some(false));
        assert_eq!(offhand_key_latch(Some(false), true, true), Some(false));
        assert_eq!(offhand_key_latch(Some(false), false, true), None);
    }

    #[test]
    fn only_spectators_have_no_hotbar_and_only_survival_and_adventure_show_vitals() {
        let modes = [
            McGameMode::Survival,
            McGameMode::Creative,
            McGameMode::Adventure,
            McGameMode::Spectator,
        ];
        assert_eq!(modes.map(McGameMode::shows_hotbar), [true, true, true, false]);
        assert_eq!(modes.map(McGameMode::shows_vitals), [true, false, true, false]);
    }

    #[test]
    fn the_camera_cycles_first_back_front_like_vanilla() {
        let first = McCameraType::default();
        assert_eq!(first, McCameraType::FirstPerson);
        assert_eq!(first.cycle(), McCameraType::ThirdPersonBack);
        assert_eq!(first.cycle().cycle(), McCameraType::ThirdPersonFront);
        assert_eq!(first.cycle().cycle().cycle(), first);
        for camera in [
            McCameraType::FirstPerson,
            McCameraType::ThirdPersonBack,
            McCameraType::ThirdPersonFront,
        ] {
            assert_eq!(
                camera.is_first_person(),
                camera == McCameraType::FirstPerson
            );
            assert_eq!(
                camera.is_mirrored(),
                camera == McCameraType::ThirdPersonFront
            );
        }
        assert!(!McCamera::default().detached);
    }

    #[test]
    fn the_glint_scrolls_at_vanilla_speed() {
        assert_eq!(glint_shift(0), [-0.0, 0.0]);
        assert!(near(
            glint_shift(1000),
            [-4000.0 / 110_000.0, 4000.0 / 30_000.0]
        ));
        assert!(near(glint_shift(27_500), [0.0, 20_000.0 / 30_000.0]));
    }

    #[test]
    fn a_boss_bar_slides_to_its_new_progress_over_a_tenth_of_a_second() {
        let mut bar = McBossBar::new("Ender Dragon".to_owned(), 1.0, true, true);
        assert!(bar.world_fog && bar.music);
        assert_eq!(bar.progress(), 1.0);
        bar.set_progress(0.5);
        assert_eq!(bar.progress(), 1.0);
        bar.advance(0.05);
        assert!((bar.progress() - 0.75).abs() < 1e-6);
        bar.set_progress(0.5);
        assert!((bar.progress() - 0.75).abs() < 1e-6, "the same value does not restart it");
        bar.advance(0.2);
        assert_eq!(bar.progress(), 0.5);
        bar.set_progress(0.25);
        bar.advance(0.05);
        assert!((bar.progress() - 0.375).abs() < 1e-6);
    }

    #[test]
    fn the_glint_turns_ten_degrees_anticlockwise_after_scaling() {
        assert!(near(
            glint_uv([0.0, 0.0], [1.0, 0.0], 8.0),
            [7.8785, 1.3892]
        ));
        assert!(near(
            glint_uv([0.0, 0.0], [0.0, 1.0], 8.0),
            [-1.3892, 7.8785]
        ));
        assert!(near(glint_uv([0.25, -0.5], [0.0, 0.0], 8.0), [0.25, -0.5]));
        assert!(near(
            glint_uv([0.25, -0.5], [1.0, 0.0], 8.0),
            [0.25 + 7.8785, -0.5 + 1.3892]
        ));
    }
}
