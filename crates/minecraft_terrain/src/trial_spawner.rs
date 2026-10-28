use crate::item_nbt::json_tag;
use minecraftoss_core::datapack::DataPack;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::RandomSource;
use minecraftoss_core::{BlockPos, Chunk, Identifier};
use minecraftoss_world::level::Level;
use std::collections::BTreeMap;

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
        let config = |key: &str| match tag.get(key) {
            None => Some(TrialConfig::default()),
            Some(Tag::String(id)) => registry_config(pack, id),
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
}

pub fn is_trial_spawner(tag: &Tag) -> bool {
    tag.get("id").and_then(Tag::as_str) == Some(ID)
}

pub fn wake_spawners(level: &mut Level, simulated: impl Fn(&str) -> bool) {
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
    for key in spawners {
        wake(level, key, &simulated);
    }
}

fn wake(level: &mut Level, key: (i32, i32, i32), simulated: &impl Fn(&str) -> bool) {
    let pos = BlockPos::new(key.0, key.1, key.2);
    let lib = level.lib;
    let blocks = &lib.registries.blocks;
    let state = level.block(pos);
    if blocks.property(state, STATE) != Some(TrialState::Inactive.name()) {
        return;
    }
    let ominous = blocks.property(state, OMINOUS) == Some("true");
    let Some(waiting) = blocks.with_property(state, STATE, TrialState::WaitingForPlayers.name())
    else {
        return;
    };
    let Some(tag) = level
        .chunk(pos.chunk())
        .and_then(|chunk| chunk.block_entities.entities.get(&key))
        .cloned()
    else {
        return;
    };
    let spawn_data = match tag.get("spawn_data") {
        Some(data) => data.clone(),
        None => {
            let config = FullConfig::read(&lib.registries.datapack, &tag);
            let data = config
                .active(ominous)
                .pick_spawn_data(&mut level.random)
                .unwrap_or_else(empty_spawn_data);
            level.set_block_entity_value(pos, "spawn_data", data.clone());
            data
        }
    };
    if spawn_kind(&spawn_data).is_some_and(simulated) {
        level.set_block_and_update(pos, waiting);
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
}
