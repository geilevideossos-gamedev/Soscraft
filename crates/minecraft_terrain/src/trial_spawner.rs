use crate::item_nbt::json_tag;
use crate::raids::{ints_of_uuid, uuid_of_ints};
use minecraftoss_core::datapack::DataPack;
use minecraftoss_core::loot::LootParams;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::random::RandomSource;
use minecraftoss_core::{BlockPos, Chunk, Identifier};
use minecraftoss_world::level::Level;
use minecraftoss_world::level::burning::LevelSound;
use minecraftoss_world::level::physics::{ClipBlocks, ClipFluids};
use std::collections::{BTreeMap, HashMap};

pub const ID: &str = "minecraft:trial_spawner";
pub const STATE: &str = "trial_spawner_state";
pub const OMINOUS: &str = "ominous";
const CONFIGS: &str = "trial_spawner";
const REQUIRED_PLAYER_RANGE: i32 = 14;
const TARGET_COOLDOWN_LENGTH: i32 = 36000;
const CONSUMABLES: &str = "minecraft:spawners/trial_chamber/consumables";
const KEY: &str = "minecraft:spawners/trial_chamber/key";
const ITEMS_TO_DROP_WHEN_OMINOUS: &str =
    "minecraft:spawners/trial_chamber/items_to_drop_when_ominous";
const MAX_MOB_TRACKING_DISTANCE: i32 = 47;
const TIME_BETWEEN_EACH_EJECTION: f32 = 30.0;
const SHUTTER_DELAY: f32 = 40.0;
const DETECTION_PERIOD: i64 = 20;
const DETECTION_SPAWN_DELAY: i64 = 40;
const EJECT_HEIGHT: f64 = 1.2;
const EJECT_ACCURACY: i32 = 2;
pub const SPAWN_PARTICLES: i32 = 3011;
pub const SPAWN_MOB: i32 = 3012;
pub const DETECT_PLAYER: i32 = 3013;
pub const EJECT_ITEM: i32 = 3014;
pub const DETECT_PLAYER_OMINOUS: i32 = 3019;
pub const BECOME_OMINOUS: i32 = 3020;
pub const SPAWN_ITEM: i32 = 3021;
pub const SPAWN_MOB_SOUND: &str = "minecraft:block.trial_spawner.spawn_mob";
pub const SPAWN_ITEM_SOUND: &str = "minecraft:block.trial_spawner.spawn_item";
pub const DETECT_PLAYER_SOUND: &str = "minecraft:block.trial_spawner.detect_player";
pub const OMINOUS_ACTIVATE_SOUND: &str = "minecraft:block.trial_spawner.ominous_activate";
pub const EJECT_ITEM_SOUND: &str = "minecraft:block.trial_spawner.eject_item";
pub const OPEN_SHUTTER: &str = "minecraft:block.trial_spawner.open_shutter";
pub const CLOSE_SHUTTER: &str = "minecraft:block.trial_spawner.close_shutter";
const RESET_STATE_DATA: [&str; 6] = [
    "registered_players",
    "current_mobs",
    "cooldown_ends_at",
    "next_mob_spawns_at",
    "total_mobs_spawned",
    "spawn_data",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emission {
    None,
    SmallFlames,
    FlamesAndSmoke,
    SmokeInsideAndTopFace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialState {
    Inactive,
    WaitingForPlayers,
    Active,
    WaitingForRewardEjection,
    EjectingReward,
    Cooldown,
}

impl TrialState {
    pub const ALL: [Self; 6] = [
        Self::Inactive,
        Self::WaitingForPlayers,
        Self::Active,
        Self::WaitingForRewardEjection,
        Self::EjectingReward,
        Self::Cooldown,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::WaitingForPlayers => "waiting_for_players",
            Self::Active => "active",
            Self::WaitingForRewardEjection => "waiting_for_reward_ejection",
            Self::EjectingReward => "ejecting_reward",
            Self::Cooldown => "cooldown",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.name() == name)
    }

    pub fn spin_speed(self) -> f64 {
        match self {
            Self::WaitingForPlayers => 200.0,
            Self::Active => 1000.0,
            _ => -1.0,
        }
    }

    pub fn has_spinning_mob(self) -> bool {
        self.spin_speed() >= 0.0
    }

    pub fn emission(self) -> Emission {
        match self {
            Self::Inactive => Emission::None,
            Self::WaitingForPlayers | Self::WaitingForRewardEjection | Self::EjectingReward => {
                Emission::SmallFlames
            }
            Self::Active => Emission::FlamesAndSmoke,
            Self::Cooldown => Emission::SmokeInsideAndTopFace,
        }
    }

    pub fn can_spawn(self) -> bool {
        matches!(self, Self::WaitingForPlayers | Self::Active)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrialConfig {
    pub spawn_range: i32,
    pub total_mobs: f32,
    pub simultaneous_mobs: f32,
    pub total_mobs_added_per_player: f32,
    pub simultaneous_mobs_added_per_player: f32,
    pub ticks_between_spawn: i32,
    pub spawn_potentials: Vec<(Tag, i32)>,
    pub loot_tables_to_eject: Vec<(String, i32)>,
    pub items_to_drop_when_ominous: String,
}

impl Default for TrialConfig {
    fn default() -> Self {
        Self {
            spawn_range: 4,
            total_mobs: 6.0,
            simultaneous_mobs: 2.0,
            total_mobs_added_per_player: 2.0,
            simultaneous_mobs_added_per_player: 1.0,
            ticks_between_spawn: 40,
            spawn_potentials: Vec::new(),
            loot_tables_to_eject: vec![(CONSUMABLES.to_owned(), 1), (KEY.to_owned(), 1)],
            items_to_drop_when_ominous: ITEMS_TO_DROP_WHEN_OMINOUS.to_owned(),
        }
    }
}

fn int(tag: &Tag, key: &str, default: i32) -> i32 {
    tag.get(key)
        .and_then(Tag::as_f64)
        .map_or(default, |value| value as i32)
}

fn float(tag: &Tag, key: &str, default: f32) -> f32 {
    tag.get(key)
        .and_then(Tag::as_f64)
        .map_or(default, |value| value as f32)
}

fn weighted<T>(tag: &Tag, key: &str, read: impl Fn(&Tag) -> Option<T>) -> Option<Vec<(T, i32)>> {
    let list = tag.get(key)?.as_list()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let weight = int(entry, "weight", 0);
                if weight <= 0 {
                    return None;
                }
                Some((read(entry.get("data")?)?, weight))
            })
            .collect(),
    )
}

fn weighted_tag<T>(entries: &[(T, i32)], write: impl Fn(&T) -> Tag) -> Tag {
    Tag::List(
        entries
            .iter()
            .map(|(data, weight)| {
                let mut entry = BTreeMap::new();
                entry.insert("data".to_owned(), write(data));
                entry.insert("weight".to_owned(), Tag::Int(*weight));
                Tag::Compound(entry)
            })
            .collect(),
    )
}

fn pick<'a, T>(entries: &'a [(T, i32)], random: &mut dyn RandomSource) -> Option<&'a T> {
    let total: i32 = entries.iter().map(|(_, weight)| *weight).sum();
    if total <= 0 {
        return None;
    }
    let mut selection = random.next_i32_bound(total);
    for (data, weight) in entries {
        if selection < *weight {
            return Some(data);
        }
        selection -= weight;
    }
    None
}

pub fn empty_spawn_data() -> Tag {
    let mut data = BTreeMap::new();
    data.insert("entity".to_owned(), Tag::Compound(BTreeMap::new()));
    Tag::Compound(data)
}

pub fn spawn_kind(spawn_data: &Tag) -> Option<&str> {
    spawn_data.get("entity")?.get("id")?.as_str()
}

impl TrialConfig {
    pub fn from_tag(tag: &Tag) -> Self {
        let base = Self::default();
        Self {
            spawn_range: int(tag, "spawn_range", base.spawn_range),
            total_mobs: float(tag, "total_mobs", base.total_mobs),
            simultaneous_mobs: float(tag, "simultaneous_mobs", base.simultaneous_mobs),
            total_mobs_added_per_player: float(
                tag,
                "total_mobs_added_per_player",
                base.total_mobs_added_per_player,
            ),
            simultaneous_mobs_added_per_player: float(
                tag,
                "simultaneous_mobs_added_per_player",
                base.simultaneous_mobs_added_per_player,
            ),
            ticks_between_spawn: int(tag, "ticks_between_spawn", base.ticks_between_spawn),
            spawn_potentials: weighted(tag, "spawn_potentials", |data| {
                data.get("entity").is_some().then(|| data.clone())
            })
            .unwrap_or(base.spawn_potentials),
            loot_tables_to_eject: weighted(tag, "loot_tables_to_eject", |data| {
                data.as_str().map(str::to_owned)
            })
            .unwrap_or(base.loot_tables_to_eject),
            items_to_drop_when_ominous: tag
                .get("items_to_drop_when_ominous")
                .and_then(Tag::as_str)
                .map_or(base.items_to_drop_when_ominous, str::to_owned),
        }
    }

    pub fn to_tag(&self) -> Tag {
        let base = Self::default();
        let mut map = BTreeMap::new();
        if self.spawn_range != base.spawn_range {
            map.insert("spawn_range".to_owned(), Tag::Int(self.spawn_range));
        }
        for (key, value, default) in [
            ("total_mobs", self.total_mobs, base.total_mobs),
            (
                "simultaneous_mobs",
                self.simultaneous_mobs,
                base.simultaneous_mobs,
            ),
            (
                "total_mobs_added_per_player",
                self.total_mobs_added_per_player,
                base.total_mobs_added_per_player,
            ),
            (
                "simultaneous_mobs_added_per_player",
                self.simultaneous_mobs_added_per_player,
                base.simultaneous_mobs_added_per_player,
            ),
        ] {
            if value != default {
                map.insert(key.to_owned(), Tag::Float(value));
            }
        }
        if self.ticks_between_spawn != base.ticks_between_spawn {
            map.insert(
                "ticks_between_spawn".to_owned(),
                Tag::Int(self.ticks_between_spawn),
            );
        }
        if !self.spawn_potentials.is_empty() {
            map.insert(
                "spawn_potentials".to_owned(),
                weighted_tag(&self.spawn_potentials, Tag::clone),
            );
        }
        if self.loot_tables_to_eject != base.loot_tables_to_eject {
            map.insert(
                "loot_tables_to_eject".to_owned(),
                weighted_tag(&self.loot_tables_to_eject, |table| {
                    Tag::String(table.clone())
                }),
            );
        }
        if self.items_to_drop_when_ominous != base.items_to_drop_when_ominous {
            map.insert(
                "items_to_drop_when_ominous".to_owned(),
                Tag::String(self.items_to_drop_when_ominous.clone()),
            );
        }
        Tag::Compound(map)
    }

    pub fn target_total_mobs(&self, additional_players: i32) -> i32 {
        f64::from(self.total_mobs + self.total_mobs_added_per_player * additional_players as f32)
            .floor() as i32
    }

    pub fn target_simultaneous_mobs(&self, additional_players: i32) -> i32 {
        f64::from(
            self.simultaneous_mobs
                + self.simultaneous_mobs_added_per_player * additional_players as f32,
        )
        .floor() as i32
    }

    pub fn pick_spawn_data(&self, random: &mut dyn RandomSource) -> Option<Tag> {
        pick(&self.spawn_potentials, random).cloned()
    }

    pub fn with_spawning(&self, kind: &str) -> Self {
        let mut entity = BTreeMap::new();
        entity.insert("id".to_owned(), Tag::String(kind.to_owned()));
        let mut data = BTreeMap::new();
        data.insert("entity".to_owned(), Tag::Compound(entity));
        Self {
            spawn_potentials: vec![(Tag::Compound(data), 1)],
            ..self.clone()
        }
    }
}

pub fn registry_config(pack: &DataPack, id: &str) -> Option<TrialConfig> {
    let id = Identifier::parse(id).ok()?;
    let json = pack.read_json(CONFIGS, &id).ok()?;
    Some(TrialConfig::from_tag(&json_tag(&json)))
}

#[derive(Clone, Debug, PartialEq)]
pub struct FullConfig {
    pub normal: TrialConfig,
    pub ominous: TrialConfig,
    pub required_player_range: i32,
    pub target_cooldown_length: i32,
}

impl Default for FullConfig {
    fn default() -> Self {
        Self {
            normal: TrialConfig::default(),
            ominous: TrialConfig::default(),
            required_player_range: REQUIRED_PLAYER_RANGE,
            target_cooldown_length: TARGET_COOLDOWN_LENGTH,
        }
    }
}

impl FullConfig {
    pub fn read(pack: &DataPack, tag: &Tag) -> Self {
        Self::read_with(tag, &mut |id| registry_config(pack, id))
    }

    pub fn read_with(tag: &Tag, registry: &mut dyn FnMut(&str) -> Option<TrialConfig>) -> Self {
        let mut config = |key: &str| match tag.get(key) {
            None => Some(TrialConfig::default()),
            Some(Tag::String(id)) => registry(id),
            Some(inline) => Some(TrialConfig::from_tag(inline)),
        };
        let (Some(normal), Some(ominous)) = (config("normal_config"), config("ominous_config"))
        else {
            return Self::default();
        };
        Self {
            normal,
            ominous,
            required_player_range: int(tag, "required_player_range", REQUIRED_PLAYER_RANGE),
            target_cooldown_length: int(tag, "target_cooldown_length", TARGET_COOLDOWN_LENGTH),
        }
    }

    pub fn active(&self, ominous: bool) -> &TrialConfig {
        if ominous { &self.ominous } else { &self.normal }
    }

    pub fn write(&self, map: &mut BTreeMap<String, Tag>) {
        map.insert("normal_config".to_owned(), self.normal.to_tag());
        map.insert("ominous_config".to_owned(), self.ominous.to_tag());
        for (key, value, default) in [
            (
                "required_player_range",
                self.required_player_range,
                REQUIRED_PLAYER_RANGE,
            ),
            (
                "target_cooldown_length",
                self.target_cooldown_length,
                TARGET_COOLDOWN_LENGTH,
            ),
        ] {
            if value == default {
                map.remove(key);
            } else {
                map.insert(key.to_owned(), Tag::Int(value));
            }
        }
    }
}

pub fn override_entity(pack: &DataPack, tag: &Tag, kind: &str) -> Tag {
    let full = FullConfig::read(pack, tag);
    let full = FullConfig {
        normal: full.normal.with_spawning(kind),
        ominous: full.ominous.with_spawning(kind),
        ..full
    };
    let mut map = tag.as_compound().cloned().unwrap_or_default();
    for key in RESET_STATE_DATA {
        map.remove(key);
    }
    full.write(&mut map);
    Tag::Compound(map)
}

pub fn spawn_egg(level: &mut Level, pos: BlockPos, kind: &str) -> bool {
    let key = (pos.x, pos.y, pos.z);
    let Some(tag) = level
        .chunk(pos.chunk())
        .and_then(|chunk| chunk.block_entities.entities.get(&key))
        .filter(|tag| is_trial_spawner(tag))
        .cloned()
    else {
        return false;
    };
    let lib = level.lib;
    level.set_block_entity_tag(pos, override_entity(&lib.registries.datapack, &tag, kind));
    let state = level.block(pos);
    if let Some(inactive) =
        lib.registries
            .blocks
            .with_property(state, STATE, TrialState::Inactive.name())
    {
        level.set_block_and_update(pos, inactive);
    }
    true
}

pub fn is_trial_spawner(tag: &Tag) -> bool {
    tag.get("id").and_then(Tag::as_str) == Some(ID)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateData {
    pub detected_players: Vec<u128>,
    pub current_mobs: Vec<u128>,
    pub cooldown_ends_at: i64,
    pub next_mob_spawns_at: i64,
    pub total_mobs_spawned: i32,
    pub spawn_data: Option<Tag>,
    pub ejecting_loot_table: Option<String>,
}

fn uuid_of_tag(tag: &Tag) -> Option<u128> {
    match tag {
        Tag::IntArray(ints) => match ints[..] {
            [a, b, c, d] => Some(uuid_of_ints([a, b, c, d])),
            _ => None,
        },
        _ => None,
    }
}

fn uuids(tag: &Tag, key: &str) -> Vec<u128> {
    let mut out = Vec::new();
    for uuid in tag
        .get(key)
        .and_then(Tag::as_list)
        .into_iter()
        .flatten()
        .filter_map(uuid_of_tag)
    {
        if !out.contains(&uuid) {
            out.push(uuid);
        }
    }
    out
}

fn uuid_list(uuids: &[u128]) -> Tag {
    Tag::List(
        uuids
            .iter()
            .map(|uuid| Tag::IntArray(ints_of_uuid(*uuid).to_vec()))
            .collect(),
    )
}

fn long(tag: &Tag, key: &str) -> i64 {
    tag.get(key).and_then(Tag::as_i64).unwrap_or(0)
}

impl StateData {
    pub fn read(tag: &Tag) -> Self {
        Self {
            detected_players: uuids(tag, "registered_players"),
            current_mobs: uuids(tag, "current_mobs"),
            cooldown_ends_at: long(tag, "cooldown_ends_at"),
            next_mob_spawns_at: long(tag, "next_mob_spawns_at"),
            total_mobs_spawned: int(tag, "total_mobs_spawned", 0).max(0),
            spawn_data: tag
                .get("spawn_data")
                .filter(|data| data.as_compound().is_some())
                .cloned(),
            ejecting_loot_table: tag
                .get("ejecting_loot_table")
                .and_then(Tag::as_str)
                .map(str::to_owned),
        }
    }

    pub fn write(&self, map: &mut BTreeMap<String, Tag>) {
        for key in RESET_STATE_DATA {
            map.remove(key);
        }
        map.remove("ejecting_loot_table");
        if !self.detected_players.is_empty() {
            map.insert(
                "registered_players".to_owned(),
                uuid_list(&self.detected_players),
            );
        }
        if !self.current_mobs.is_empty() {
            map.insert("current_mobs".to_owned(), uuid_list(&self.current_mobs));
        }
        for (key, value) in [
            ("cooldown_ends_at", self.cooldown_ends_at),
            ("next_mob_spawns_at", self.next_mob_spawns_at),
        ] {
            if value != 0 {
                map.insert(key.to_owned(), Tag::Long(value));
            }
        }
        if self.total_mobs_spawned != 0 {
            map.insert(
                "total_mobs_spawned".to_owned(),
                Tag::Int(self.total_mobs_spawned),
            );
        }
        if let Some(data) = &self.spawn_data {
            map.insert("spawn_data".to_owned(), data.clone());
        }
        if let Some(table) = &self.ejecting_loot_table {
            map.insert("ejecting_loot_table".to_owned(), Tag::String(table.clone()));
        }
    }

    fn reset_statistics(&mut self) {
        self.detected_players.clear();
        self.total_mobs_spawned = 0;
        self.next_mob_spawns_at = 0;
        self.cooldown_ends_at = 0;
    }

    fn reset(&mut self) {
        self.current_mobs.clear();
        self.spawn_data = None;
        self.reset_statistics();
    }

    fn additional_players(&self) -> i32 {
        (self.detected_players.len() as i32 - 1).max(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrialPlayer {
    pub uuid: u128,
    pub block: BlockPos,
    pub eye: [f64; 3],
    pub detectable: bool,
}

pub trait TrialWorld {
    fn now(&self) -> i64;
    fn players(&self) -> &[TrialPlayer];
    fn sees(&self, from: [f64; 3], to: [f64; 3]) -> bool;
    fn mob_block(&self, uuid: u128) -> Option<BlockPos>;
    fn can_spawn(&self) -> bool;
    fn simulated(&self, kind: &str) -> bool;
    fn random(&mut self) -> &mut dyn RandomSource;
    fn spawn_mob(
        &mut self,
        spawner: BlockPos,
        spawn_data: &Tag,
        at: [f64; 3],
    ) -> Option<(u128, BlockPos)>;
    fn eject(&mut self, spawner: BlockPos, table: &str);
    fn sound(&mut self, spawner: BlockPos, event: &'static str);
    fn level_event(&mut self, event: i32, pos: BlockPos, data: i32);
}

pub fn center(pos: BlockPos) -> [f64; 3] {
    [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    ]
}

fn block_at(at: [f64; 3]) -> BlockPos {
    BlockPos::new(
        at[0].floor() as i32,
        at[1].floor() as i32,
        at[2].floor() as i32,
    )
}

fn saved_pos(entity: &Tag) -> Option<[f64; 3]> {
    let list = entity.get("Pos")?.as_list()?;
    if list.len() != 3 {
        return None;
    }
    Some([list[0].as_f64()?, list[1].as_f64()?, list[2].as_f64()?])
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrialSpawner {
    pub config: FullConfig,
    pub data: StateData,
}

impl TrialSpawner {
    pub fn tick(
        &mut self,
        pos: BlockPos,
        state: TrialState,
        ominous: &mut bool,
        world: &mut impl TrialWorld,
    ) -> TrialState {
        let now = world.now();
        let between = i64::from(self.config.active(*ominous).ticks_between_spawn);
        let reach = f64::from(MAX_MOB_TRACKING_DISTANCE * MAX_MOB_TRACKING_DISTANCE);
        let tracked = self.data.current_mobs.len();
        self.data.current_mobs.retain(|uuid| {
            world
                .mob_block(*uuid)
                .is_some_and(|at| at.dist_sqr(pos) <= reach)
        });
        if self.data.current_mobs.len() != tracked {
            self.data.next_mob_spawns_at = now + between;
        }
        let cooldown = i64::from(self.config.target_cooldown_length);
        match state {
            TrialState::Inactive => {
                if self.has_mob_to_spawn(*ominous, world) {
                    TrialState::WaitingForPlayers
                } else {
                    state
                }
            }
            TrialState::WaitingForPlayers => {
                if !world.can_spawn() {
                    self.data.reset_statistics();
                    return state;
                }
                if !self.has_mob_to_spawn(*ominous, world) {
                    return TrialState::Inactive;
                }
                self.try_detect_players(pos, state, *ominous, world);
                if self.data.detected_players.is_empty() {
                    state
                } else {
                    TrialState::Active
                }
            }
            TrialState::Active => {
                if !world.can_spawn() {
                    self.data.reset_statistics();
                    return TrialState::WaitingForPlayers;
                }
                if !self.has_mob_to_spawn(*ominous, world) {
                    return TrialState::Inactive;
                }
                let additional = self.data.additional_players();
                self.try_detect_players(pos, state, *ominous, world);
                let config = self.config.active(*ominous).clone();
                if self.data.total_mobs_spawned >= config.target_total_mobs(additional) {
                    if self.data.current_mobs.is_empty() {
                        self.data.cooldown_ends_at = now + cooldown;
                        self.data.total_mobs_spawned = 0;
                        self.data.next_mob_spawns_at = 0;
                        return TrialState::WaitingForRewardEjection;
                    }
                } else if now >= self.data.next_mob_spawns_at
                    && (self.data.current_mobs.len() as i32)
                        < config.target_simultaneous_mobs(additional)
                {
                    if let Some(uuid) = self.spawn_mob(pos, *ominous, world) {
                        self.data.current_mobs.push(uuid);
                        self.data.total_mobs_spawned += 1;
                        self.data.next_mob_spawns_at = now + i64::from(config.ticks_between_spawn);
                        if let Some(next) = config.pick_spawn_data(world.random()) {
                            self.data.spawn_data = Some(next);
                        }
                    }
                }
                state
            }
            TrialState::WaitingForRewardEjection => {
                let start = self.data.cooldown_ends_at - cooldown;
                if now as f32 >= start as f32 + SHUTTER_DELAY {
                    world.sound(pos, OPEN_SHUTTER);
                    TrialState::EjectingReward
                } else {
                    state
                }
            }
            TrialState::EjectingReward => {
                let start = self.data.cooldown_ends_at - cooldown;
                if (now - start) as f32 % TIME_BETWEEN_EACH_EJECTION != 0.0 {
                    return state;
                }
                if self.data.detected_players.is_empty() {
                    world.sound(pos, CLOSE_SHUTTER);
                    self.data.ejecting_loot_table = None;
                    return TrialState::Cooldown;
                }
                if self.data.ejecting_loot_table.is_none() {
                    let tables = &self.config.active(*ominous).loot_tables_to_eject;
                    self.data.ejecting_loot_table = pick(tables, world.random()).cloned();
                }
                if let Some(table) = self.data.ejecting_loot_table.clone() {
                    world.eject(pos, &table);
                }
                self.data.detected_players.remove(0);
                state
            }
            TrialState::Cooldown => {
                self.try_detect_players(pos, state, *ominous, world);
                if !self.data.detected_players.is_empty() {
                    self.data.total_mobs_spawned = 0;
                    self.data.next_mob_spawns_at = 0;
                    return TrialState::Active;
                }
                if now >= self.data.cooldown_ends_at {
                    *ominous = false;
                    self.data.reset();
                    return TrialState::WaitingForPlayers;
                }
                state
            }
        }
    }

    fn next_spawn_data(&mut self, ominous: bool, world: &mut impl TrialWorld) -> Tag {
        if let Some(data) = &self.data.spawn_data {
            return data.clone();
        }
        let data = self
            .config
            .active(ominous)
            .pick_spawn_data(world.random())
            .unwrap_or_else(empty_spawn_data);
        self.data.spawn_data = Some(data.clone());
        data
    }

    fn has_mob_to_spawn(&mut self, ominous: bool, world: &mut impl TrialWorld) -> bool {
        let data = self.next_spawn_data(ominous, world);
        spawn_kind(&data).is_some_and(|kind| world.simulated(kind))
    }

    fn detect(&self, pos: BlockPos, sight: bool, world: &impl TrialWorld) -> Vec<u128> {
        let range = f64::from(self.config.required_player_range);
        let eye_of_spawner = center(pos);
        world
            .players()
            .iter()
            .filter(|player| {
                player.detectable
                    && player.block.dist_sqr(pos) < range * range
                    && (!sight || world.sees(eye_of_spawner, player.eye))
            })
            .map(|player| player.uuid)
            .collect()
    }

    fn try_detect_players(
        &mut self,
        pos: BlockPos,
        state: TrialState,
        ominous: bool,
        world: &mut impl TrialWorld,
    ) {
        let now = world.now();
        if pos.pack().wrapping_add(now) % DETECTION_PERIOD != 0 {
            return;
        }
        if state == TrialState::Cooldown && ominous {
            return;
        }
        let sighted = self.detect(pos, true, world);
        if state == TrialState::Cooldown {
            return;
        }
        let found = if self.data.detected_players.is_empty() {
            sighted
        } else {
            self.detect(pos, false, world)
        };
        let before = self.data.detected_players.len();
        for uuid in found {
            if !self.data.detected_players.contains(&uuid) {
                self.data.detected_players.push(uuid);
            }
        }
        let detected = self.data.detected_players.len();
        if detected > before {
            self.data.next_mob_spawns_at =
                (now + DETECTION_SPAWN_DELAY).max(self.data.next_mob_spawns_at);
            let event = if ominous {
                DETECT_PLAYER_OMINOUS
            } else {
                DETECT_PLAYER
            };
            world.level_event(event, pos, detected as i32);
        }
    }

    fn spawn_mob(
        &mut self,
        pos: BlockPos,
        ominous: bool,
        world: &mut impl TrialWorld,
    ) -> Option<u128> {
        let data = self.next_spawn_data(ominous, world);
        spawn_kind(&data)?;
        let at = match data.get("entity").and_then(saved_pos) {
            Some(at) => at,
            None => {
                let range = f64::from(self.config.active(ominous).spawn_range);
                let random = world.random();
                let x = f64::from(pos.x) + (random.next_f64() - random.next_f64()) * range + 0.5;
                let y = f64::from(pos.y + random.next_i32_bound(3) - 1);
                let z = f64::from(pos.z) + (random.next_f64() - random.next_f64()) * range + 0.5;
                [x, y, z]
            }
        };
        let (uuid, mob) = world.spawn_mob(pos, &data, at)?;
        let flame = i32::from(ominous);
        world.level_event(SPAWN_PARTICLES, pos, flame);
        world.level_event(SPAWN_MOB, mob, flame);
        Some(uuid)
    }
}

pub struct LevelTrials<'a, 'l> {
    pub level: &'a mut Level<'l>,
    pub players: &'a [TrialPlayer],
    pub mobs: &'a HashMap<u128, BlockPos>,
    pub simulated: &'a dyn Fn(&str) -> bool,
}

impl TrialWorld for LevelTrials<'_, '_> {
    fn now(&self) -> i64 {
        self.level.game_time
    }

    fn players(&self) -> &[TrialPlayer] {
        self.players
    }

    fn sees(&self, from: [f64; 3], to: [f64; 3]) -> bool {
        self.level
            .clip_first_hit(to, from, ClipBlocks::Visual, ClipFluids::None)
            .is_none_or(|hit| hit == block_at(from))
    }

    fn mob_block(&self, uuid: u128) -> Option<BlockPos> {
        self.mobs.get(&uuid).copied()
    }

    fn can_spawn(&self) -> bool {
        self.level
            .natural_spawning
            .as_ref()
            .is_some_and(|natural| natural.spawner_blocks_work && natural.spawn_mobs)
            && self.level.difficulty != 0
    }

    fn simulated(&self, kind: &str) -> bool {
        (self.simulated)(kind)
    }

    fn random(&mut self) -> &mut dyn RandomSource {
        &mut self.level.random
    }

    fn spawn_mob(
        &mut self,
        spawner: BlockPos,
        spawn_data: &Tag,
        at: [f64; 3],
    ) -> Option<(u128, BlockPos)> {
        if !self.sees(center(spawner), at) {
            return None;
        }
        let entity = spawn_data.get("entity")?;
        let rules = spawn_data.get("custom_spawn_rules");
        let mob = self.level.summon_trial_mob(entity, rules, at)?;
        let uuid = mob.get("UUID").and_then(uuid_of_tag)?;
        let feet = saved_pos(&mob).unwrap_or(at);
        Some((uuid, block_at(feet)))
    }

    fn eject(&mut self, spawner: BlockPos, table: &str) {
        let registries = self.level.lib.registries.clone();
        let params = LootParams::default();
        let level = &mut *self.level;
        let Ok(stacks) = registries.loot.roll(
            &registries,
            table,
            &params,
            &mut level.random_sequences,
            &mut level.random,
        ) else {
            return;
        };
        if stacks.is_empty() {
            return;
        }
        let at = [
            f64::from(spawner.x) + 0.5,
            f64::from(spawner.y) + EJECT_HEIGHT,
            f64::from(spawner.z) + 0.5,
        ];
        for stack in stacks {
            level.spawn_dispensed_item(at, Direction::Up, EJECT_ACCURACY, stack);
        }
        level.level_events.push((EJECT_ITEM, spawner, 0));
    }

    fn sound(&mut self, spawner: BlockPos, event: &'static str) {
        self.level.sounds.push(LevelSound {
            event,
            pos: center(spawner),
            volume: 1.0,
            pitch: 1.0,
        });
    }

    fn level_event(&mut self, event: i32, pos: BlockPos, data: i32) {
        self.level.level_events.push((event, pos, data));
    }
}

pub fn ticking_spawners(level: &Level) -> Vec<(i32, i32, i32)> {
    let ticking: Vec<&Chunk> = match &level.ticking_chunks {
        Some(list) => list.iter().filter_map(|pos| level.chunk(*pos)).collect(),
        None => level.chunks().collect(),
    };
    let mut spawners: Vec<(i32, i32, i32)> = ticking
        .into_iter()
        .flat_map(|chunk| {
            chunk
                .block_entities
                .entities
                .iter()
                .filter(|(_, tag)| is_trial_spawner(tag))
                .map(|(key, _)| *key)
        })
        .collect();
    spawners.sort_unstable();
    spawners
}

pub struct Trials<'a> {
    pub players: &'a [TrialPlayer],
    pub mobs: &'a HashMap<u128, BlockPos>,
    pub simulated: &'a dyn Fn(&str) -> bool,
}

pub fn tick_spawners(
    level: &mut Level,
    spawners: &[(i32, i32, i32)],
    trials: &Trials,
    configs: &mut HashMap<String, Option<TrialConfig>>,
) {
    for key in spawners {
        tick_at(level, *key, trials, configs);
    }
}

fn tick_at(
    level: &mut Level,
    key: (i32, i32, i32),
    trials: &Trials,
    configs: &mut HashMap<String, Option<TrialConfig>>,
) {
    let pos = BlockPos::new(key.0, key.1, key.2);
    let lib = level.lib;
    let blocks = &lib.registries.blocks;
    let block = level.block(pos);
    let Some(state) = blocks
        .property(block, STATE)
        .and_then(TrialState::from_name)
    else {
        return;
    };
    let was_ominous = blocks.property(block, OMINOUS) == Some("true");
    let Some(tag) = level
        .chunk(pos.chunk())
        .and_then(|chunk| chunk.block_entities.entities.get(&key))
        .cloned()
    else {
        return;
    };
    let pack = &lib.registries.datapack;
    let config = FullConfig::read_with(&tag, &mut |id| {
        configs
            .entry(id.to_owned())
            .or_insert_with(|| registry_config(pack, id))
            .clone()
    });
    let mut spawner = TrialSpawner {
        config,
        data: StateData::read(&tag),
    };
    let before = spawner.data.clone();
    let mut ominous = was_ominous;
    let next = spawner.tick(
        pos,
        state,
        &mut ominous,
        &mut LevelTrials {
            level,
            players: trials.players,
            mobs: trials.mobs,
            simulated: trials.simulated,
        },
    );
    if spawner.data != before {
        let mut map = tag.as_compound().cloned().unwrap_or_default();
        spawner.data.write(&mut map);
        level.set_block_entity_tag(pos, Tag::Compound(map));
    }
    if next != state || ominous != was_ominous {
        let block = level.block(pos);
        let changed = blocks
            .with_property(block, STATE, next.name())
            .and_then(|block| {
                blocks.with_property(block, OMINOUS, if ominous { "true" } else { "false" })
            });
        if let Some(changed) = changed {
            level.set_block_and_update(pos, changed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::random::LegacyRandom;

    fn pack() -> Option<DataPack> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        DataPack::open(&paths.datapack).ok()
    }

    fn entity(id: &str) -> Tag {
        let mut entity = BTreeMap::new();
        entity.insert("id".to_owned(), Tag::String(id.to_owned()));
        let mut data = BTreeMap::new();
        data.insert("entity".to_owned(), Tag::Compound(entity));
        Tag::Compound(data)
    }

    #[test]
    fn the_six_states_carry_vanillas_spin_particles_and_spawning() {
        let rows: Vec<_> = TrialState::ALL
            .into_iter()
            .map(|state| {
                (
                    state.name(),
                    state.spin_speed(),
                    state.has_spinning_mob(),
                    state.emission(),
                    state.can_spawn(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("inactive", -1.0, false, Emission::None, false),
                (
                    "waiting_for_players",
                    200.0,
                    true,
                    Emission::SmallFlames,
                    true
                ),
                ("active", 1000.0, true, Emission::FlamesAndSmoke, true),
                (
                    "waiting_for_reward_ejection",
                    -1.0,
                    false,
                    Emission::SmallFlames,
                    false
                ),
                ("ejecting_reward", -1.0, false, Emission::SmallFlames, false),
                (
                    "cooldown",
                    -1.0,
                    false,
                    Emission::SmokeInsideAndTopFace,
                    false
                ),
            ]
        );
        for state in TrialState::ALL {
            assert_eq!(TrialState::from_name(state.name()), Some(state));
        }
        assert_eq!(TrialState::from_name("asleep"), None);
    }

    #[test]
    fn an_empty_config_has_vanillas_defaults_and_targets() {
        let config = TrialConfig::from_tag(&Tag::Compound(BTreeMap::new()));
        assert_eq!(config, TrialConfig::default());
        assert_eq!(config.spawn_range, 4);
        assert_eq!(config.ticks_between_spawn, 40);
        assert_eq!(
            config.loot_tables_to_eject,
            [(CONSUMABLES.to_owned(), 1), (KEY.to_owned(), 1)]
        );
        assert_eq!(
            config.items_to_drop_when_ominous,
            ITEMS_TO_DROP_WHEN_OMINOUS
        );
        assert_eq!(
            (
                config.target_total_mobs(0),
                config.target_simultaneous_mobs(0)
            ),
            (6, 2)
        );
        assert_eq!(
            (
                config.target_total_mobs(1),
                config.target_simultaneous_mobs(1)
            ),
            (8, 3)
        );
        assert_eq!(
            config.to_tag(),
            Tag::Compound(BTreeMap::new()),
            "defaults are left out"
        );
        assert_eq!(config.pick_spawn_data(&mut LegacyRandom::new(1)), None);
    }

    #[test]
    fn an_inline_config_reads_and_writes_back_the_same() {
        let mut map = BTreeMap::new();
        map.insert("spawn_range".to_owned(), Tag::Int(6));
        map.insert("total_mobs".to_owned(), Tag::Float(3.0));
        map.insert("simultaneous_mobs".to_owned(), Tag::Double(4.0));
        map.insert("ticks_between_spawn".to_owned(), Tag::Int(160));
        let mut potentials = Vec::new();
        for (id, weight) in [("minecraft:zombie", 3), ("minecraft:husk", 1)] {
            let mut entry = BTreeMap::new();
            entry.insert("data".to_owned(), entity(id));
            entry.insert("weight".to_owned(), Tag::Int(weight));
            potentials.push(Tag::Compound(entry));
        }
        let mut broken = BTreeMap::new();
        broken.insert("data".to_owned(), entity("minecraft:skeleton"));
        potentials.push(Tag::Compound(broken));
        map.insert("spawn_potentials".to_owned(), Tag::List(potentials));
        let mut key = BTreeMap::new();
        key.insert("data".to_owned(), Tag::String(KEY.to_owned()));
        key.insert("weight".to_owned(), Tag::Int(2));
        map.insert(
            "loot_tables_to_eject".to_owned(),
            Tag::List(vec![Tag::Compound(key)]),
        );
        let config = TrialConfig::from_tag(&Tag::Compound(map));
        assert_eq!(config.spawn_range, 6);
        assert_eq!((config.total_mobs, config.simultaneous_mobs), (3.0, 4.0));
        assert_eq!(config.ticks_between_spawn, 160);
        assert_eq!(
            config.spawn_potentials,
            [
                (entity("minecraft:zombie"), 3),
                (entity("minecraft:husk"), 1)
            ]
        );
        assert_eq!(config.loot_tables_to_eject, [(KEY.to_owned(), 2)]);
        assert_eq!(TrialConfig::from_tag(&config.to_tag()), config);
        let written = config.to_tag();
        assert_eq!(written.get("simultaneous_mobs"), Some(&Tag::Float(4.0)));
        assert!(written.get("simultaneous_mobs_added_per_player").is_none());
        let mut random = LegacyRandom::new(9);
        let mut zombies = 0;
        for _ in 0..400 {
            let data = config.pick_spawn_data(&mut random).unwrap();
            if spawn_kind(&data) == Some("minecraft:zombie") {
                zombies += 1;
            }
        }
        assert!((260..340).contains(&zombies), "{zombies}");
    }

    #[test]
    fn every_trial_chamber_config_in_the_data_reads() {
        let Some(pack) = pack() else {
            return;
        };
        let ids = pack.list(CONFIGS).unwrap();
        assert_eq!(ids.len(), 28, "{ids:?}");
        for id in &ids {
            let config =
                registry_config(&pack, &id.to_string()).unwrap_or_else(|| panic!("{id} reads"));
            assert!(!config.spawn_potentials.is_empty(), "{id}");
            assert!(
                config
                    .spawn_potentials
                    .iter()
                    .all(|(data, _)| spawn_kind(data).is_some()),
                "{id}"
            );
        }
        let read = |id: &str| registry_config(&pack, id).unwrap();
        let slime = read("minecraft:trial_chamber/small_melee/slime/normal");
        let sizes: Vec<_> = slime
            .spawn_potentials
            .iter()
            .map(|(data, weight)| {
                let size = data.get("entity").and_then(|e| e.get("Size"));
                (size.and_then(Tag::as_f64), *weight)
            })
            .collect();
        assert_eq!(sizes, [(Some(1.0), 3), (Some(2.0), 1)]);
        let zombie = read("minecraft:trial_chamber/melee/zombie/ominous");
        let equipment = zombie.spawn_potentials[0].0.get("equipment").unwrap();
        assert_eq!(
            equipment.get("loot_table").and_then(Tag::as_str),
            Some("minecraft:equipment/trial_chamber_melee")
        );
        assert_eq!(
            equipment.get("slot_drop_chances").and_then(Tag::as_f64),
            Some(0.0)
        );
        assert_eq!(
            zombie.loot_tables_to_eject,
            [
                ("minecraft:spawners/ominous/trial_chamber/key".to_owned(), 3),
                (
                    "minecraft:spawners/ominous/trial_chamber/consumables".to_owned(),
                    7
                ),
            ]
        );
        let targets = |config: &TrialConfig| {
            [0, 1].map(|extra| {
                (
                    config.target_total_mobs(extra),
                    config.target_simultaneous_mobs(extra),
                )
            })
        };
        let melee = read("minecraft:trial_chamber/melee/zombie/normal");
        assert_eq!(targets(&melee), [(6, 3), (8, 3)]);
        assert_eq!(melee.ticks_between_spawn, 20);
        let slow = read("minecraft:trial_chamber/slow_ranged/skeleton/normal");
        assert_eq!(targets(&slow), [(6, 4), (8, 6)]);
        assert_eq!(slow.ticks_between_spawn, 160);
        let breeze = read("minecraft:trial_chamber/breeze/normal");
        assert_eq!(targets(&breeze), [(2, 1), (3, 1)]);
        assert_eq!(
            registry_config(&pack, "minecraft:trial_chamber/nothing"),
            None
        );
    }

    #[test]
    fn an_egg_makes_both_configs_spawn_its_mob_and_resets_the_trial() {
        let Some(pack) = pack() else {
            return;
        };
        let normal = "minecraft:trial_chamber/melee/zombie/normal";
        let ominous = "minecraft:trial_chamber/melee/zombie/ominous";
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(ID.to_owned()));
        map.insert("normal_config".to_owned(), Tag::String(normal.to_owned()));
        map.insert("ominous_config".to_owned(), Tag::String(ominous.to_owned()));
        map.insert("target_cooldown_length".to_owned(), Tag::Int(600));
        map.insert("total_mobs_spawned".to_owned(), Tag::Int(4));
        map.insert("next_mob_spawns_at".to_owned(), Tag::Long(900));
        map.insert("cooldown_ends_at".to_owned(), Tag::Long(36900));
        for key in ["registered_players", "current_mobs"] {
            map.insert(
                key.to_owned(),
                Tag::List(vec![Tag::IntArray(vec![1, 2, 3, 4])]),
            );
        }
        map.insert("spawn_data".to_owned(), entity("minecraft:zombie"));
        map.insert(
            "ejecting_loot_table".to_owned(),
            Tag::String(KEY.to_owned()),
        );
        let tag = override_entity(&pack, &Tag::Compound(map), "minecraft:skeleton");
        let keys: Vec<&str> = tag
            .as_compound()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "ejecting_loot_table",
                "id",
                "normal_config",
                "ominous_config",
                "target_cooldown_length"
            ]
        );
        let full = FullConfig::read(&pack, &tag);
        let skeleton = vec![(entity("minecraft:skeleton"), 1)];
        assert_eq!(
            full.normal,
            TrialConfig {
                spawn_potentials: skeleton.clone(),
                ..registry_config(&pack, normal).unwrap()
            }
        );
        assert_eq!(
            full.ominous,
            TrialConfig {
                spawn_potentials: skeleton,
                ..registry_config(&pack, ominous).unwrap()
            }
        );
        assert_eq!(
            (full.required_player_range, full.target_cooldown_length),
            (14, 600)
        );
        let written = tag.get("normal_config").unwrap();
        assert_eq!(written.get("simultaneous_mobs"), Some(&Tag::Float(3.0)));
        assert!(written.get("total_mobs").is_none());
        let mut placed = BTreeMap::new();
        placed.insert("id".to_owned(), Tag::String(ID.to_owned()));
        let fresh = override_entity(&pack, &Tag::Compound(placed), "minecraft:zombie");
        let full = FullConfig::read(&pack, &fresh);
        assert_eq!(
            full.normal,
            TrialConfig::default().with_spawning("minecraft:zombie")
        );
        assert_eq!(full.ominous, full.normal);
        assert!(fresh.get("target_cooldown_length").is_none());
    }

    #[test]
    fn a_full_config_reads_registry_ids_inline_configs_and_falls_back_whole() {
        let Some(pack) = pack() else {
            return;
        };
        let mut inline = BTreeMap::new();
        inline.insert("total_mobs".to_owned(), Tag::Float(1.0));
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(ID.to_owned()));
        map.insert(
            "normal_config".to_owned(),
            Tag::String("minecraft:trial_chamber/breeze/normal".to_owned()),
        );
        map.insert("ominous_config".to_owned(), Tag::Compound(inline));
        map.insert("required_player_range".to_owned(), Tag::Int(9));
        let full = FullConfig::read(&pack, &Tag::Compound(map.clone()));
        assert_eq!(
            full.normal,
            registry_config(&pack, "minecraft:trial_chamber/breeze/normal").unwrap()
        );
        assert_eq!(full.ominous.total_mobs, 1.0);
        assert_eq!(full.active(true).total_mobs, 1.0);
        assert_eq!(full.active(false).total_mobs, 2.0);
        assert_eq!(
            (full.required_player_range, full.target_cooldown_length),
            (9, 36000)
        );
        map.insert(
            "normal_config".to_owned(),
            Tag::String("minecraft:trial_chamber/nothing".to_owned()),
        );
        assert_eq!(
            FullConfig::read(&pack, &Tag::Compound(map)),
            FullConfig::default()
        );
    }

    struct Mock {
        now: i64,
        players: Vec<TrialPlayer>,
        walls: bool,
        mobs: HashMap<u128, BlockPos>,
        can_spawn: bool,
        random: LegacyRandom,
        made: u128,
        ejected: Vec<String>,
        sounds: Vec<&'static str>,
        events: Vec<(i32, BlockPos, i32)>,
    }

    impl Mock {
        fn new() -> Self {
            Self {
                now: 0,
                players: Vec::new(),
                walls: false,
                mobs: HashMap::new(),
                can_spawn: true,
                random: LegacyRandom::new(5),
                made: 0,
                ejected: Vec::new(),
                sounds: Vec::new(),
                events: Vec::new(),
            }
        }
    }

    impl TrialWorld for Mock {
        fn now(&self) -> i64 {
            self.now
        }

        fn players(&self) -> &[TrialPlayer] {
            &self.players
        }

        fn sees(&self, _from: [f64; 3], to: [f64; 3]) -> bool {
            !self.walls || to[0] < 0.0
        }

        fn mob_block(&self, uuid: u128) -> Option<BlockPos> {
            self.mobs.get(&uuid).copied()
        }

        fn can_spawn(&self) -> bool {
            self.can_spawn
        }

        fn simulated(&self, kind: &str) -> bool {
            kind != "minecraft:breeze"
        }

        fn random(&mut self) -> &mut dyn RandomSource {
            &mut self.random
        }

        fn spawn_mob(
            &mut self,
            _spawner: BlockPos,
            _spawn_data: &Tag,
            at: [f64; 3],
        ) -> Option<(u128, BlockPos)> {
            self.made += 1;
            self.mobs.insert(self.made, block_at(at));
            Some((self.made, block_at(at)))
        }

        fn eject(&mut self, _spawner: BlockPos, table: &str) {
            self.ejected.push(table.to_owned());
        }

        fn sound(&mut self, _spawner: BlockPos, event: &'static str) {
            self.sounds.push(event);
        }

        fn level_event(&mut self, event: i32, pos: BlockPos, data: i32) {
            self.events.push((event, pos, data));
        }
    }

    const AT: BlockPos = BlockPos::new(0, 64, 0);

    fn player(uuid: u128, x: i32, detectable: bool) -> TrialPlayer {
        TrialPlayer {
            uuid,
            block: BlockPos::new(x, 64, 0),
            eye: [f64::from(x) + 0.5, 65.62, 0.5],
            detectable,
        }
    }

    fn spawner_of(kind: &str) -> TrialSpawner {
        TrialSpawner {
            config: FullConfig {
                normal: TrialConfig::default().with_spawning(kind),
                ominous: TrialConfig::default().with_spawning(kind),
                ..FullConfig::default()
            },
            data: StateData::default(),
        }
    }

    struct Run {
        spawner: TrialSpawner,
        state: TrialState,
        world: Mock,
    }

    impl Run {
        fn of(kind: &str) -> Self {
            Self {
                spawner: spawner_of(kind),
                state: TrialState::Inactive,
                world: Mock::new(),
            }
        }

        fn step(&mut self) {
            self.world.now += 1;
            let mut ominous = false;
            self.state = self
                .spawner
                .tick(AT, self.state, &mut ominous, &mut self.world);
        }

        fn until(&mut self, limit: i64, done: impl Fn(&Self) -> bool) -> i64 {
            let start = self.world.now;
            while !done(self) {
                assert!(
                    self.world.now - start < limit,
                    "{:?} after {limit} ticks",
                    self.state
                );
                self.step();
            }
            self.world.now - start
        }

        fn active(&mut self) {
            self.until(60, |run| run.state == TrialState::Active);
        }
    }

    #[test]
    fn a_waiting_spawner_detects_a_survival_player_in_range_and_sight_on_its_twenty_tick_phase() {
        let mut run = Run::of("minecraft:zombie");
        run.step();
        assert_eq!(run.state, TrialState::WaitingForPlayers);
        assert_eq!(
            run.spawner.data.spawn_data.as_ref().and_then(spawn_kind),
            Some("minecraft:zombie")
        );
        run.world.players = vec![
            player(1, 13, false),
            player(2, 14, true),
            player(3, -5, false),
        ];
        for _ in 0..60 {
            run.step();
        }
        assert_eq!(
            run.state,
            TrialState::WaitingForPlayers,
            "creative, spectating or 14 blocks away"
        );
        run.world.walls = true;
        run.world.players.push(player(4, 5, true));
        for _ in 0..60 {
            run.step();
        }
        assert_eq!(run.state, TrialState::WaitingForPlayers, "out of sight");
        assert!(run.world.events.is_empty());
        run.world.players.push(player(5, -13, true));
        run.until(21, |run| run.state == TrialState::Active);
        assert_eq!((AT.pack() + run.world.now) % 20, 0, "only on its phase");
        assert_eq!(run.spawner.data.detected_players, [5]);
        assert_eq!(run.spawner.data.next_mob_spawns_at, run.world.now + 40);
        assert_eq!(run.world.events, [(DETECT_PLAYER, AT, 1)]);
        assert_eq!(
            run.until(21, |run| run.spawner.data.detected_players.len() == 2),
            20
        );
        assert_eq!(
            run.spawner.data.detected_players,
            [5, 4],
            "once one is seen, the others need no sight"
        );
        assert_eq!(run.world.events.last(), Some(&(DETECT_PLAYER, AT, 2)));
    }

    #[test]
    fn a_trial_spawns_its_mobs_in_turn_ejects_one_reward_per_player_and_cools_down() {
        let mut run = Run::of("minecraft:zombie");
        run.world.players = vec![player(7, 3, true), player(8, -3, true)];
        run.active();
        let detected = run.world.now;
        assert_eq!(run.spawner.data.detected_players, [7, 8]);
        let (mut spawns, mut most) = (Vec::new(), 0);
        while run.state == TrialState::Active {
            assert!(run.world.now - detected < 3000);
            let made = run.world.made;
            run.step();
            if run.world.made > made {
                spawns.push(run.world.now);
                let flames: Vec<i32> = run.world.events[run.world.events.len() - 2..]
                    .iter()
                    .map(|event| event.0)
                    .collect();
                assert_eq!(flames, [SPAWN_PARTICLES, SPAWN_MOB]);
            }
            let current = run.spawner.data.current_mobs.clone();
            most = most.max(current.len());
            if current.len() == 3 || run.spawner.data.total_mobs_spawned == 8 {
                if let Some(oldest) = current.first() {
                    run.world.mobs.remove(oldest);
                }
            }
        }
        assert_eq!(spawns.len(), 8, "6 and 2 more for the second player");
        assert_eq!(most, 3, "2 at once and 1 more for the second player");
        assert_eq!(spawns[0] - detected, 40);
        assert!(spawns.windows(2).all(|pair| pair[1] - pair[0] >= 40));
        for at in run.world.mobs.values() {
            assert!((at.x.abs() <= 4) && (at.z.abs() <= 4) && (63..=65).contains(&at.y));
        }
        let end = run.world.now;
        assert_eq!(run.state, TrialState::WaitingForRewardEjection);
        assert_eq!(run.spawner.data.cooldown_ends_at, end + 36000);
        assert_eq!(
            (
                run.spawner.data.total_mobs_spawned,
                run.spawner.data.next_mob_spawns_at
            ),
            (0, 0)
        );
        assert_eq!(
            run.until(100, |run| run.state == TrialState::EjectingReward),
            40
        );
        assert_eq!(run.world.sounds, [OPEN_SHUTTER]);
        run.until(100, |run| run.state == TrialState::Cooldown);
        assert_eq!(run.world.now - end, 120);
        assert_eq!(run.world.ejected.len(), 2, "one reward per player");
        assert_eq!(run.world.ejected[0], run.world.ejected[1]);
        assert!([CONSUMABLES, KEY].contains(&run.world.ejected[0].as_str()));
        assert_eq!(run.world.sounds, [OPEN_SHUTTER, CLOSE_SHUTTER]);
        assert_eq!(run.spawner.data.ejecting_loot_table, None);
        assert!(run.spawner.data.detected_players.is_empty());
        let waited = run.until(36000, |run| run.state == TrialState::WaitingForPlayers);
        assert_eq!(run.world.now - end, 36000, "{waited}");
        assert_eq!(run.spawner.data, StateData::default());
        run.active();
    }

    #[test]
    fn a_mob_farther_than_47_blocks_is_untracked_and_replaced_after_ticks_between_spawn() {
        let mut run = Run::of("minecraft:zombie");
        run.world.players = vec![player(7, 3, true)];
        run.active();
        run.until(200, |run| run.spawner.data.current_mobs.len() == 2);
        let first = run.spawner.data.current_mobs[0];
        run.world.mobs.insert(first, BlockPos::new(47, 64, 0));
        for _ in 0..80 {
            run.step();
        }
        assert_eq!(
            run.spawner.data.current_mobs.len(),
            2,
            "47 blocks away is kept"
        );
        run.world.mobs.insert(first, BlockPos::new(47, 65, 0));
        run.step();
        assert_eq!(run.spawner.data.current_mobs.len(), 1);
        assert_eq!(run.spawner.data.next_mob_spawns_at, run.world.now + 40);
        assert_eq!(
            run.until(100, |run| run.spawner.data.current_mobs.len() == 2),
            40
        );
        assert_eq!(run.spawner.data.total_mobs_spawned, 3);
    }

    #[test]
    fn peaceful_or_spawning_rules_off_send_a_trial_back_to_waiting() {
        let mut run = Run::of("minecraft:zombie");
        run.world.players = vec![player(7, 3, true)];
        run.active();
        run.until(200, |run| run.spawner.data.current_mobs.len() == 2);
        run.world.can_spawn = false;
        run.step();
        assert_eq!(run.state, TrialState::WaitingForPlayers);
        let data = &run.spawner.data;
        assert!(data.detected_players.is_empty());
        assert_eq!(
            (
                data.total_mobs_spawned,
                data.next_mob_spawns_at,
                data.current_mobs.len()
            ),
            (0, 0, 2)
        );
        for _ in 0..60 {
            run.step();
        }
        assert_eq!(run.state, TrialState::WaitingForPlayers);
        run.world.can_spawn = true;
        run.active();
    }

    #[test]
    fn a_spawner_of_a_mob_the_game_does_not_simulate_stays_inactive() {
        let mut run = Run::of("minecraft:breeze");
        run.world.players = vec![player(7, 3, true)];
        for _ in 0..100 {
            run.step();
        }
        assert_eq!(run.state, TrialState::Inactive);
        assert_eq!(
            run.spawner.data.spawn_data.as_ref().and_then(spawn_kind),
            Some("minecraft:breeze")
        );
        let mut empty = Run::of("minecraft:zombie");
        empty.spawner.config = FullConfig::default();
        empty.step();
        assert_eq!(empty.state, TrialState::Inactive);
        assert_eq!(empty.spawner.data.spawn_data, Some(empty_spawn_data()));
        let mut waiting = Run::of("minecraft:zombie");
        waiting.state = TrialState::WaitingForPlayers;
        waiting.spawner.data.spawn_data = Some(entity("minecraft:breeze"));
        waiting.step();
        assert_eq!(waiting.state, TrialState::Inactive);
    }

    #[test]
    fn the_state_data_round_trips_every_key() {
        let data = StateData {
            detected_players: vec![uuid_of_ints([1, 2, 3, 4]), uuid_of_ints([-1, 0, 0, 9])],
            current_mobs: vec![uuid_of_ints([5, 6, 7, 8])],
            cooldown_ends_at: 72000,
            next_mob_spawns_at: 36040,
            total_mobs_spawned: 3,
            spawn_data: Some(entity("minecraft:husk")),
            ejecting_loot_table: Some(KEY.to_owned()),
        };
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(ID.to_owned()));
        data.write(&mut map);
        assert_eq!(
            map.get("registered_players"),
            Some(&Tag::List(vec![
                Tag::IntArray(vec![1, 2, 3, 4]),
                Tag::IntArray(vec![-1, 0, 0, 9])
            ]))
        );
        assert_eq!(map.get("cooldown_ends_at"), Some(&Tag::Long(72000)));
        assert_eq!(map.get("total_mobs_spawned"), Some(&Tag::Int(3)));
        assert_eq!(StateData::read(&Tag::Compound(map.clone())), data);
        StateData::default().write(&mut map);
        assert_eq!(map.keys().collect::<Vec<_>>(), ["id"]);
        assert_eq!(StateData::read(&Tag::Compound(map)), StateData::default());
    }

    #[test]
    fn the_trial_chamber_reward_tables_roll_keys_and_consumables() {
        use minecraftoss_core::loot::{LootParams, RandomSequences};
        let Some(registries) = minecraftoss_core::registries::DataPaths::discover()
            .ok()
            .and_then(|paths| minecraftoss_core::Registries::load(&paths).ok())
        else {
            return;
        };
        let mut sequences = RandomSequences::new(3);
        let mut random = LegacyRandom::new(3);
        let mut roll = |table: &str| {
            registries
                .loot
                .roll(
                    &registries,
                    table,
                    &LootParams::default(),
                    &mut sequences,
                    &mut random,
                )
                .unwrap()
        };
        let key = roll(KEY);
        assert_eq!(key.len(), 1);
        assert_eq!(
            (key[0].id.as_str(), key[0].count),
            ("minecraft:trial_key", 1)
        );
        let ominous = roll("minecraft:spawners/ominous/trial_chamber/key");
        assert_eq!(ominous[0].id, "minecraft:ominous_trial_key");
        let mut kinds = std::collections::BTreeSet::new();
        for _ in 0..100 {
            let items = roll(CONSUMABLES);
            assert!(!items.is_empty());
            kinds.extend(items.into_iter().map(|item| item.id));
        }
        assert!(kinds.len() >= 4, "{kinds:?}");
    }
}
