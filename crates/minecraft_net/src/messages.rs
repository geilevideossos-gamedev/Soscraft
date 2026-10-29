use minecraft_commands::Text;
use minecraft_commands::protocol::{Clientbound, Serverbound};
use serde::{Deserialize, Serialize};

use crate::{Dimension, PlayerUuid};

pub const PROTOCOL: u16 = 6;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nbt(pub Vec<u8>);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blob(pub Vec<u8>);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkBlob(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToServerFrame {
    pub frame: u32,
    pub ack: u32,
    pub messages: Vec<ToServer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToClientFrame {
    pub frame: u32,
    pub ack: u32,
    pub messages: Vec<ToClient>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToServer {
    Hello {
        protocol: u16,
        view_distance: u8,
        registry: u64,
    },
    Ack,
    ChunkBatchReceived {
        seq: u32,
    },
    Hand(Vec<HandEvent>),
    SelectSlot(u8),
    Click(Click),
    RenameItem(String),
    CloseMenu {
        menu: u8,
    },
    Chat(Serverbound),
    CreditsDone,
    Lectern {
        pos: [i32; 3],
        button: Option<i32>,
    },
    LeaveBed,
    EditBook {
        slot: u8,
        pages: Vec<String>,
        title: Option<String>,
    },
    CreativeSlot {
        slot: CreativeTarget,
        stack: Option<Nbt>,
    },
    RecipeBookSettings {
        open: bool,
        filtering: bool,
    },
    RecipeSeen(Vec<String>),
    OpenMountInventory,
    Skate(SkateReport),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreativeTarget {
    Menu(u8),
    Carried,
    Drop,
}

impl ToServer {
    pub fn hello(view_distance: u8, registry: u64) -> Self {
        Self::Hello {
            protocol: PROTOCOL,
            view_distance,
            registry,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HandState {
    pub tick: u32,
    pub attack: bool,
    pub use_item: bool,
    pub target: Target,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkateReport {
    pub tick: u32,
    pub on: bool,
    pub airborne: bool,
    pub origin: [f32; 3],
    pub yaw: f32,
    pub pose: Option<Box<SkatePose>>,
}

pub const SKATE_BONES: usize = 23;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkatePose {
    pub root: [f32; 12],
    pub bones: [[f32; 12]; SKATE_BONES],
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skaters(pub Vec<(u32, SkatePose)>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StateUp {
    Hand(HandState),
    Skate(SkateReport),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HandEvent {
    pub seq: u32,
    pub kind: HandEventKind,
    pub target: Target,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandEventKind {
    AttackPressed,
    AttackReleased,
    UsePressed,
    UseReleased,
    SwapHands,
    Drop { whole_stack: bool },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Target {
    #[default]
    Miss,
    Block {
        pos: [i32; 3],
        face: u8,
        hit: [f32; 3],
    },
    Mob {
        id: u64,
    },
    Player {
        client: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Click {
    pub menu: u8,
    pub state: u16,
    pub action: MenuAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuSlot {
    Inventory(u8),
    Crafting(u8),
    Result,
    Workbench(u8),
    WorkbenchResult,
    Container(u16),
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuAction {
    Slot {
        slot: MenuSlot,
        right: bool,
        shift: bool,
    },
    Outside {
        right: bool,
    },
    Gather {
        slot: MenuSlot,
        right: bool,
    },
    Swap {
        slot: MenuSlot,
        hotbar: u8,
    },
    Spread {
        slots: Vec<MenuSlot>,
        right: bool,
    },
    Throw {
        slot: MenuSlot,
        whole: bool,
    },
    Choose(u16),
    PlaceRecipe {
        recipe: String,
        all: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToClient {
    Login(Login),
    Rejected {
        reason: Text,
    },
    ChunkBatch {
        seq: u32,
    },
    ChunkForget {
        pos: [i32; 2],
    },
    BlockUpdates {
        changes: Vec<([i32; 3], u32)>,
    },
    BlockEntity {
        pos: [i32; 3],
        tag: Nbt,
    },
    BlockEvent {
        pos: [i32; 3],
        kind: u8,
        param: i32,
    },
    Crack {
        breaker: u32,
        pos: [i32; 3],
        stage: i8,
    },
    LevelEvent {
        event: i32,
        pos: [i32; 3],
        data: i32,
    },
    Sound {
        event: String,
        pos: [f64; 3],
        volume: f32,
        pitch: f32,
        seed: i64,
    },
    Explosion {
        center: [f64; 3],
        radius: f32,
        blocks: u32,
        interacts: bool,
    },
    MobAdd {
        id: u64,
        tag: Nbt,
    },
    MobLook {
        id: u64,
        look: Blob,
    },
    MobRemove {
        ids: Vec<u64>,
    },
    Player(Box<PlayerView>),
    MenuOpen(MenuView),
    MenuSlots {
        menu: u8,
        state: u16,
        slots: Vec<(i16, Option<Nbt>)>,
        carried: Option<Option<Nbt>>,
        data: Vec<(u8, i32)>,
    },
    MenuClose {
        menu: u8,
    },
    Equipment {
        client: u32,
        gear: GearView,
    },
    Chat(Clientbound),
    PlayerList {
        added: Vec<(u32, String, PlayerUuid)>,
        removed: Vec<u32>,
    },
    Time {
        game_time: i64,
        clocks: Vec<(String, f64, bool)>,
    },
    Weather {
        rain: f32,
        thunder: f32,
    },
    Dimension {
        epoch: u32,
        dimension: Dimension,
        origin: [f64; 3],
        feet: [f64; 3],
    },
    BossBar(BossBarUpdate),
    KilledBy {
        killer: Option<String>,
        line: Text,
    },
    Credits,
    PlayerEvent {
        client: u32,
        event: u8,
    },
    Bursts {
        entities: Blob,
    },
    Lectern(Option<BookView>),
    Sleeping {
        client: u32,
        bed: Option<Lying>,
        timer: i32,
    },
    OpenBook {
        slot: u8,
    },
    RecipeBook(RecipeBookView),
    Animate {
        client: u32,
        offhand: bool,
    },
    SkateRefused {
        tick: u32,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBookView {
    pub recipes: Vec<String>,
    pub highlight: Vec<String>,
    pub open: bool,
    pub filtering: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lying {
    pub feet: [f64; 3],
    pub facing: [i32; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookView {
    pub pos: [i32; 3],
    pub pages: Vec<String>,
    pub page: u32,
    pub take: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Login {
    pub client: u32,
    pub uuid: PlayerUuid,
    pub seed: i64,
    pub dimension: Dimension,
    pub origins: Vec<(Dimension, [f64; 3])>,
    pub feet: [f64; 3],
    pub view_distance: u8,
    pub rules: Rules,
    pub registry: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    pub pvp: bool,
    pub show_death_messages: bool,
    pub keep_inventory: bool,
    pub natural_health_regeneration: bool,
    pub reduced_debug_info: bool,
    pub do_immediate_respawn: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            pvp: true,
            show_death_messages: true,
            keep_inventory: false,
            natural_health_regeneration: true,
            reduced_debug_info: false,
            do_immediate_respawn: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateFrame {
    pub tick: u32,
    pub epoch: u32,
    pub mobs: Blob,
    pub entities: Blob,
    pub skaters: Blob,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Abilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub may_fly: bool,
    pub instabuild: bool,
    pub may_build: bool,
    pub fly_speed: f32,
    pub walk_speed: f32,
}

impl Default for Abilities {
    fn default() -> Self {
        Self {
            invulnerable: false,
            flying: false,
            may_fly: false,
            instabuild: false,
            may_build: true,
            fly_speed: 0.05,
            walk_speed: 0.1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectView {
    pub kind: String,
    pub amplifier: u8,
    pub duration: i32,
    pub ambient: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CooldownView {
    pub group: String,
    pub remaining: i32,
    pub total: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hand {
    #[default]
    Main,
    Off,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseKind {
    Eat {
        duration: i32,
    },
    #[default]
    Block,
    Charge,
    Scope,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsingView {
    pub hand: Hand,
    pub ticks: i32,
    pub kind: UseKind,
}

pub const PLAYER_SLOTS: usize = 45;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerView {
    pub health: f32,
    pub max_health: f32,
    pub absorption: f32,
    pub food: i32,
    pub saturation: f32,
    pub air: i32,
    pub fire_ticks: i32,
    pub frozen: i32,
    pub armor: i32,
    pub xp_level: i32,
    pub xp_progress: f32,
    pub xp_total: i32,
    pub game_mode: GameMode,
    pub abilities: Abilities,
    pub effects: Vec<EffectView>,
    pub slots: Vec<Option<Nbt>>,
    pub selected: u8,
    pub cooldowns: Vec<CooldownView>,
    pub using: Option<UsingView>,
    pub jump_bar: Option<f32>,
    pub vehicle_health: Option<[f32; 2]>,
}

impl Default for PlayerView {
    fn default() -> Self {
        Self {
            health: 20.0,
            max_health: 20.0,
            absorption: 0.0,
            food: 20,
            saturation: 5.0,
            air: 300,
            fire_ticks: 0,
            frozen: 0,
            armor: 0,
            xp_level: 0,
            xp_progress: 0.0,
            xp_total: 0,
            game_mode: GameMode::Survival,
            abilities: Abilities::default(),
            effects: Vec::new(),
            slots: vec![None; PLAYER_SLOTS],
            selected: 0,
            cooldowns: Vec::new(),
            using: None,
            jump_bar: None,
            vehicle_health: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuKind {
    Inventory,
    CraftingTable,
    Chest { rows: u8 },
    Furnace,
    Smoker,
    BlastFurnace,
    BrewingStand,
    Enchanting,
    Anvil,
    Grindstone,
    Smithing,
    Stonecutter,
    Loom,
    Cartography,
    EnderChest,
    ShulkerBox,
    Hopper,
    Dispenser,
    Dropper,
    Beacon,
    Merchant,
    Mount,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum MenuExtras {
    #[default]
    None,
    Enchanting {
        seed: i32,
        shelves: i32,
    },
    Anvil {
        cost: i32,
        name: Option<String>,
    },
    Merchant {
        villager: u64,
        profession: String,
        level: i32,
        xp: i32,
        future_xp: i32,
        show_progress: bool,
        selected: Option<u16>,
        offers: Vec<NetOffer>,
    },
    Mount {
        mount: u64,
        columns: u8,
        saddle: bool,
        armor: bool,
        llama: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetOffer {
    pub cost_a: Nbt,
    pub cost_b: Option<Nbt>,
    pub result: Nbt,
    pub uses: i32,
    pub max_uses: i32,
    pub xp: i32,
    pub special_price: i32,
    pub demand: i32,
    pub price_multiplier: f32,
    pub reward_exp: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MenuView {
    pub menu: u8,
    pub kind: MenuKind,
    pub title: Text,
    pub state: u16,
    pub slots: Vec<Option<Nbt>>,
    pub carried: Option<Nbt>,
    pub data: Vec<(u8, i32)>,
    pub extras: MenuExtras,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GearItem {
    pub id: String,
    pub glint: bool,
    pub dye: Option<u32>,
    pub potion: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GearView {
    pub main: Option<GearItem>,
    pub off: Option<GearItem>,
    pub armor: [Option<GearItem>; 4],
    pub hides_gun: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    #[default]
    Purple,
    White,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossBarOverlay {
    #[default]
    Progress,
    Notched6,
    Notched10,
    Notched12,
    Notched20,
}

impl BossBarColor {
    pub fn sprite_name(self) -> &'static str {
        match self {
            BossBarColor::Pink => "pink",
            BossBarColor::Blue => "blue",
            BossBarColor::Red => "red",
            BossBarColor::Green => "green",
            BossBarColor::Yellow => "yellow",
            BossBarColor::Purple => "purple",
            BossBarColor::White => "white",
        }
    }
}

impl BossBarOverlay {
    pub fn sprite_name(self) -> &'static str {
        match self {
            BossBarOverlay::Progress => "progress",
            BossBarOverlay::Notched6 => "notched_6",
            BossBarOverlay::Notched10 => "notched_10",
            BossBarOverlay::Notched12 => "notched_12",
            BossBarOverlay::Notched20 => "notched_20",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BossBarFlags {
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BossBarUpdate {
    pub id: u64,
    pub operation: BossBarOperation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BossBarOperation {
    Add {
        name: Text,
        progress: f32,
        color: BossBarColor,
        overlay: BossBarOverlay,
        flags: BossBarFlags,
    },
    Remove,
    Progress(f32),
    Name(Text),
    Style {
        color: BossBarColor,
        overlay: BossBarOverlay,
    },
    Flags(BossBarFlags),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelloRefusal {
    Protocol { theirs: u16 },
    Registry,
}

impl HelloRefusal {
    pub fn reason(self) -> Text {
        match self {
            Self::Protocol { theirs } => Text::translate(
                if theirs < PROTOCOL {
                    "multiplayer.disconnect.outdated_client"
                } else {
                    "multiplayer.disconnect.outdated_server"
                },
                vec![Text::literal(format!(
                    "the host's build (Minecraft multiplayer protocol {PROTOCOL})"
                ))],
            ),
            Self::Registry => Text::literal(
                "Your Minecraft 26.3 files differ from the host's. Delete iw4l-artifacts\\minecraft-26.3 and start the game again to download them.",
            ),
        }
    }
}

pub fn check_hello(protocol: u16, registry: u64, host_registry: u64) -> Result<(), HelloRefusal> {
    if protocol != PROTOCOL {
        return Err(HelloRefusal::Protocol { theirs: protocol });
    }
    if registry != host_registry {
        return Err(HelloRefusal::Registry);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_carries_the_protocol() {
        assert_eq!(
            ToServer::hello(8, 42),
            ToServer::Hello {
                protocol: PROTOCOL,
                view_distance: 8,
                registry: 42,
            }
        );
    }

    #[test]
    fn hello_is_checked_for_protocol_then_registry() {
        assert_eq!(check_hello(PROTOCOL, 7, 7), Ok(()));
        assert_eq!(
            check_hello(PROTOCOL + 1, 7, 7),
            Err(HelloRefusal::Protocol {
                theirs: PROTOCOL + 1
            })
        );
        assert_eq!(
            check_hello(PROTOCOL + 1, 7, 8),
            Err(HelloRefusal::Protocol {
                theirs: PROTOCOL + 1
            })
        );
        assert_eq!(check_hello(PROTOCOL, 7, 8), Err(HelloRefusal::Registry));
    }

    #[test]
    fn refusals_explain_themselves() {
        let key = |refusal: HelloRefusal| match refusal.reason().content {
            minecraft_commands::Content::Translate { key, .. } => key,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            key(HelloRefusal::Protocol { theirs: 0 }),
            "multiplayer.disconnect.outdated_client"
        );
        assert_eq!(
            key(HelloRefusal::Protocol {
                theirs: PROTOCOL + 1
            }),
            "multiplayer.disconnect.outdated_server"
        );
        assert!(matches!(
            HelloRefusal::Registry.reason().content,
            minecraft_commands::Content::Literal(line) if line.contains("26.3")
        ));
    }

    #[test]
    fn a_new_player_view_has_every_slot() {
        let view = PlayerView::default();
        assert_eq!(PLAYER_SLOTS, 45);
        assert_eq!(view.slots.len(), PLAYER_SLOTS);
        assert_eq!(view.health, 20.0);
        assert_eq!(view.food, 20);
        assert!(Rules::default().pvp);
    }
}
