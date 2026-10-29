use crate::bucket::{Change, ItemUse};
use crate::trial_spawner::{center, stack_tag, tag_stack, uuid_list, uuids};
use minecraftoss_core::item::ItemStack;
use minecraftoss_core::loot::{Enchanting, LootParams};
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::random::RandomSource;
use minecraftoss_core::{BlockPos, Chunk};
use minecraftoss_entities::world::MobSound;
use minecraftoss_world::level::Level;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub const ID: &str = "minecraft:vault";
pub const STATE: &str = "vault_state";
pub const OMINOUS: &str = "ominous";
pub const TRIAL_KEY: &str = "minecraft:trial_key";
pub const OMINOUS_TRIAL_KEY: &str = "minecraft:ominous_trial_key";
pub const REWARD: &str = "minecraft:chests/trial_chambers/reward";
pub const ACTIVATE: i32 = 3015;
pub const DEACTIVATE: i32 = 3016;
pub const EJECT_PARTICLES: i32 = 3017;
pub const ACTIVATE_SOUND: &str = "minecraft:block.vault.activate";
pub const DEACTIVATE_SOUND: &str = "minecraft:block.vault.deactivate";
pub const INSERT_ITEM_SOUND: &str = "minecraft:block.vault.insert_item";
pub const INSERT_ITEM_FAIL_SOUND: &str = "minecraft:block.vault.insert_item_fail";
pub const REJECT_REWARDED_PLAYER_SOUND: &str = "minecraft:block.vault.reject_rewarded_player";
pub const OPEN_SHUTTER_SOUND: &str = "minecraft:block.vault.open_shutter";
pub const CLOSE_SHUTTER_SOUND: &str = "minecraft:block.vault.close_shutter";
pub const EJECT_ITEM_SOUND: &str = "minecraft:block.vault.eject_item";
const ACTIVATION_RANGE: f64 = 4.0;
const DEACTIVATION_RANGE: f64 = 4.5;
const MAX_REWARDED_PLAYERS: usize = 128;
const STATE_UPDATE_TICKS: i64 = 20;
const UNLOCKING_DELAY_TICKS: i64 = 14;
const DISPLAY_CYCLE_TICK_RATE: i64 = 20;
const INSERT_FAIL_SOUND_BUFFER_TICKS: i64 = 15;
const EJECT_HEIGHT: f64 = 1.2;
const EJECT_ACCURACY: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultState {
    Inactive,
    Active,
    Unlocking,
    Ejecting,
}

impl VaultState {
    const ALL: [Self; 4] = [
        Self::Inactive,
        Self::Active,
        Self::Unlocking,
        Self::Ejecting,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Active => "active",
            Self::Unlocking => "unlocking",
            Self::Ejecting => "ejecting",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.name() == name)
    }
}

fn stack(tag: Option<&Tag>) -> Option<ItemStack> {
    let mut stack = tag_stack(tag?)?;
    if stack
        .components
        .as_ref()
        .is_some_and(|components| components.as_compound().is_none_or(BTreeMap::is_empty))
    {
        stack.components = None;
    }
    (!stack.is_empty()).then_some(stack)
}

#[derive(Clone, Debug, PartialEq)]
pub struct VaultConfig {
    pub loot_table: String,
    pub activation_range: f64,
    pub deactivation_range: f64,
    pub key_item: Option<ItemStack>,
    pub override_loot_table_to_display: Option<String>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            loot_table: REWARD.to_owned(),
            activation_range: ACTIVATION_RANGE,
            deactivation_range: DEACTIVATION_RANGE,
            key_item: Some(ItemStack::new(TRIAL_KEY, 1)),
            override_loot_table_to_display: None,
        }
    }
}

impl VaultConfig {
    pub fn read(tag: Option<&Tag>) -> Self {
        let Some(tag) = tag.filter(|tag| tag.as_compound().is_some()) else {
            return Self::default();
        };
        let text = |key: &str| tag.get(key).and_then(Tag::as_str).map(str::to_owned);
        let range = |key: &str, default: f64| tag.get(key).and_then(Tag::as_f64).unwrap_or(default);
        let config = Self {
            loot_table: text("loot_table").unwrap_or_else(|| REWARD.to_owned()),
            activation_range: range("activation_range", ACTIVATION_RANGE),
            deactivation_range: range("deactivation_range", DEACTIVATION_RANGE),
            key_item: stack(tag.get("key_item")),
            override_loot_table_to_display: text("override_loot_table_to_display"),
        };
        if config.activation_range > config.deactivation_range {
            Self::default()
        } else {
            config
        }
    }

    pub fn display_table(&self) -> &str {
        self.override_loot_table_to_display
            .as_deref()
            .unwrap_or(&self.loot_table)
    }

    fn accepts(&self, key: &ItemStack) -> bool {
        self.key_item.as_ref().is_some_and(|wanted| {
            key.same_item_same_components(wanted) && key.count >= wanted.count
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServerData {
    pub rewarded_players: Vec<u128>,
    pub state_updating_resumes_at: i64,
    pub items_to_eject: Vec<ItemStack>,
    pub total_ejections_needed: i32,
}

impl ServerData {
    pub fn read(tag: Option<&Tag>) -> Self {
        let Some(tag) = tag else {
            return Self::default();
        };
        Self {
            rewarded_players: uuids(tag, "rewarded_players"),
            state_updating_resumes_at: tag
                .get("state_updating_resumes_at")
                .and_then(Tag::as_i64)
                .unwrap_or(0),
            items_to_eject: tag
                .get("items_to_eject")
                .and_then(Tag::as_list)
                .into_iter()
                .flatten()
                .filter_map(|item| stack(Some(item)))
                .collect(),
            total_ejections_needed: tag
                .get("total_ejections_needed")
                .and_then(Tag::as_i64)
                .map_or(0, |total| total as i32),
        }
    }

    pub fn to_tag(&self) -> Tag {
        let mut map = BTreeMap::new();
        if !self.rewarded_players.is_empty() {
            map.insert(
                "rewarded_players".to_owned(),
                uuid_list(&self.rewarded_players),
            );
        }
        if self.state_updating_resumes_at != 0 {
            map.insert(
                "state_updating_resumes_at".to_owned(),
                Tag::Long(self.state_updating_resumes_at),
            );
        }
        if !self.items_to_eject.is_empty() {
            map.insert(
                "items_to_eject".to_owned(),
                Tag::List(self.items_to_eject.iter().map(stack_tag).collect()),
            );
        }
        if self.total_ejections_needed != 0 {
            map.insert(
                "total_ejections_needed".to_owned(),
                Tag::Int(self.total_ejections_needed),
            );
        }
        Tag::Compound(map)
    }

    pub fn ejection_progress(&self) -> f32 {
        if self.total_ejections_needed == 1 {
            return 1.0;
        }
        let left = self.items_to_eject.len() as f32;
        1.0 - (left - 1.0) / (self.total_ejections_needed as f32 - 1.0)
    }

    fn reward(&mut self, player: u128) {
        if !self.rewarded_players.contains(&player) {
            self.rewarded_players.push(player);
        }
        if self.rewarded_players.len() > MAX_REWARDED_PLAYERS {
            self.rewarded_players.remove(0);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SharedData {
    pub display_item: Option<ItemStack>,
    pub connected_players: Vec<u128>,
    pub connected_particles_range: f64,
}

impl Default for SharedData {
    fn default() -> Self {
        Self {
            display_item: None,
            connected_players: Vec::new(),
            connected_particles_range: DEACTIVATION_RANGE,
        }
    }
}

impl SharedData {
    pub fn read(tag: Option<&Tag>) -> Self {
        let Some(tag) = tag else {
            return Self::default();
        };
        Self {
            display_item: stack(tag.get("display_item")),
            connected_players: uuids(tag, "connected_players"),
            connected_particles_range: tag
                .get("connected_particles_range")
                .and_then(Tag::as_f64)
                .unwrap_or(DEACTIVATION_RANGE),
        }
    }

    pub fn to_tag(&self) -> Tag {
        let mut map = BTreeMap::new();
        if let Some(item) = &self.display_item {
            map.insert("display_item".to_owned(), stack_tag(item));
        }
        if !self.connected_players.is_empty() {
            map.insert(
                "connected_players".to_owned(),
                uuid_list(&self.connected_players),
            );
        }
        if self.connected_particles_range != DEACTIVATION_RANGE {
            map.insert(
                "connected_particles_range".to_owned(),
                Tag::Double(self.connected_particles_range),
            );
        }
        Tag::Compound(map)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultPlayer {
    pub uuid: u128,
    pub block: BlockPos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultBlock {
    pub pos: BlockPos,
    pub state: VaultState,
    pub ominous: bool,
}

fn closer_than(a: BlockPos, b: BlockPos, range: f64) -> bool {
    let axis = |a: i32, b: i32| f64::from(a) - f64::from(b);
    let (x, y, z) = (axis(a.x, b.x), axis(a.y, b.y), axis(a.z, b.z));
    x * x + y * y + z * z < range * range
}

pub trait VaultWorld {
    fn now(&self) -> i64;
    fn players(&self) -> &[VaultPlayer];
    fn random(&mut self) -> &mut dyn RandomSource;
    fn display_items(&mut self, table: &str, vault: BlockPos) -> Vec<ItemStack>;
    fn rewards(&mut self, table: &str, vault: BlockPos, key: &ItemStack) -> Vec<ItemStack>;
    fn eject(&mut self, vault: BlockPos, stack: ItemStack);
    fn sound(&mut self, vault: BlockPos, event: &'static str, pitch: f32);
    fn level_event(&mut self, event: i32, vault: BlockPos, data: i32);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vault {
    pub config: VaultConfig,
    pub server: ServerData,
    pub shared: SharedData,
}

impl Vault {
    pub fn read(tag: &Tag) -> Self {
        Self {
            config: VaultConfig::read(tag.get("config")),
            server: ServerData::read(tag.get("server_data")),
            shared: SharedData::read(tag.get("shared_data")),
        }
    }

    pub fn write(&self, map: &mut BTreeMap<String, Tag>) {
        map.insert("server_data".to_owned(), self.server.to_tag());
        map.insert("shared_data".to_owned(), self.shared.to_tag());
    }

    pub fn tick(&mut self, block: VaultBlock, world: &mut impl VaultWorld) -> VaultState {
        let now = world.now();
        if now % DISPLAY_CYCLE_TICK_RATE == 0 && block.state == VaultState::Active {
            self.cycle_display_item(block.pos, block.state, world);
        }
        if now < self.server.state_updating_resumes_at {
            return block.state;
        }
        let next = self.tick_and_get_next(block.pos, block.state, world);
        if next != block.state {
            self.transition(block, next, world);
        }
        next
    }

    pub fn insert_key(
        &mut self,
        block: VaultBlock,
        player: u128,
        key: &ItemStack,
        last_fail: &mut i64,
        world: &mut impl VaultWorld,
    ) -> Option<VaultState> {
        if block.state != VaultState::Active || !self.can_eject_reward(block.state) {
            return None;
        }
        if !self.config.accepts(key) {
            fail_sound(block.pos, INSERT_ITEM_FAIL_SOUND, last_fail, world);
            return None;
        }
        if self.server.rewarded_players.contains(&player) {
            fail_sound(block.pos, REJECT_REWARDED_PLAYER_SOUND, last_fail, world);
            return None;
        }
        let items = world.rewards(&self.config.loot_table, block.pos, key);
        if items.is_empty() {
            return None;
        }
        self.server.total_ejections_needed = items.len() as i32;
        self.server.items_to_eject = items;
        self.shared.display_item = self.server.items_to_eject.last().cloned();
        self.server.state_updating_resumes_at = world.now() + UNLOCKING_DELAY_TICKS;
        self.transition(block, VaultState::Unlocking, world);
        self.server.reward(player);
        self.update_connected_players(block.pos, self.config.deactivation_range, world);
        Some(VaultState::Unlocking)
    }

    fn can_eject_reward(&self, state: VaultState) -> bool {
        self.config.key_item.is_some() && state != VaultState::Inactive
    }

    fn cycle_display_item(
        &mut self,
        pos: BlockPos,
        state: VaultState,
        world: &mut impl VaultWorld,
    ) {
        if !self.can_eject_reward(state) {
            self.shared.display_item = None;
            return;
        }
        let items = world.display_items(self.config.display_table(), pos);
        self.shared.display_item = if items.is_empty() {
            None
        } else {
            let index = world.random().next_i32_bound(items.len() as i32);
            items.into_iter().nth(index as usize)
        };
    }

    fn update_connected_players(&mut self, pos: BlockPos, range: f64, world: &impl VaultWorld) {
        let mut connected: Vec<u128> = world
            .players()
            .iter()
            .filter(|player| closer_than(player.block, pos, range))
            .map(|player| player.uuid)
            .filter(|uuid| !self.server.rewarded_players.contains(uuid))
            .collect();
        connected.dedup();
        let same = connected.len() == self.shared.connected_players.len()
            && connected
                .iter()
                .all(|uuid| self.shared.connected_players.contains(uuid));
        if !same {
            self.shared.connected_players = connected;
        }
    }

    fn update_state_for_connected_players(
        &mut self,
        pos: BlockPos,
        range: f64,
        world: &mut impl VaultWorld,
    ) -> VaultState {
        self.update_connected_players(pos, range, world);
        self.server.state_updating_resumes_at = world.now() + STATE_UPDATE_TICKS;
        if self.shared.connected_players.is_empty() {
            VaultState::Inactive
        } else {
            VaultState::Active
        }
    }

    fn tick_and_get_next(
        &mut self,
        pos: BlockPos,
        state: VaultState,
        world: &mut impl VaultWorld,
    ) -> VaultState {
        match state {
            VaultState::Inactive => {
                self.update_state_for_connected_players(pos, self.config.activation_range, world)
            }
            VaultState::Active => {
                self.update_state_for_connected_players(pos, self.config.deactivation_range, world)
            }
            VaultState::Unlocking => {
                self.server.state_updating_resumes_at = world.now() + STATE_UPDATE_TICKS;
                VaultState::Ejecting
            }
            VaultState::Ejecting => {
                if self.server.items_to_eject.is_empty() {
                    self.server.total_ejections_needed = 0;
                    return self.update_state_for_connected_players(
                        pos,
                        self.config.deactivation_range,
                        world,
                    );
                }
                let progress = self.server.ejection_progress();
                if let Some(item) = self.server.items_to_eject.pop() {
                    world.eject(pos, item);
                    world.level_event(EJECT_PARTICLES, pos, 0);
                    world.sound(pos, EJECT_ITEM_SOUND, 0.8 + 0.4 * progress);
                }
                self.shared.display_item = self.server.items_to_eject.last().cloned();
                self.server.state_updating_resumes_at = world.now() + STATE_UPDATE_TICKS;
                VaultState::Ejecting
            }
        }
    }

    fn transition(&mut self, block: VaultBlock, next: VaultState, world: &mut impl VaultWorld) {
        if block.state == VaultState::Ejecting {
            world.sound(block.pos, CLOSE_SHUTTER_SOUND, 1.0);
        }
        let ominous = i32::from(block.ominous);
        match next {
            VaultState::Inactive => {
                self.shared.display_item = None;
                world.level_event(DEACTIVATE, block.pos, ominous);
            }
            VaultState::Active => {
                if self.shared.display_item.is_none() {
                    self.cycle_display_item(block.pos, next, world);
                }
                world.level_event(ACTIVATE, block.pos, ominous);
            }
            VaultState::Unlocking => world.sound(block.pos, INSERT_ITEM_SOUND, 1.0),
            VaultState::Ejecting => world.sound(block.pos, OPEN_SHUTTER_SOUND, 1.0),
        }
    }
}

fn fail_sound(
    pos: BlockPos,
    event: &'static str,
    last_fail: &mut i64,
    world: &mut impl VaultWorld,
) {
    let now = world.now();
    if now >= *last_fail + INSERT_FAIL_SOUND_BUFFER_TICKS {
        world.sound(pos, event, 1.0);
        *last_fail = now;
    }
}

pub fn is_vault(tag: &Tag) -> bool {
    tag.get("id").and_then(Tag::as_str) == Some(ID)
}

pub fn is_vault_block(block: &crate::scene::Block) -> bool {
    block.id.namespace == "minecraft" && block.id.path == "vault"
}

pub fn key_item_use(pos: crate::scene::BlockPos) -> ItemUse {
    ItemUse {
        change: Change::Vault(pos),
        result: None,
        wear: 0,
        consumed: true,
        sounds: Vec::new(),
    }
}

pub fn key_use(
    pos: crate::scene::BlockPos,
    block: &crate::scene::Block,
    held: Option<&minecraftoss_player::inventory::ItemStack>,
) -> Option<ItemUse> {
    let active = is_vault_block(block)
        && block.properties.get(STATE).map(String::as_str) == Some(VaultState::Active.name());
    (active && held.is_some_and(|stack| stack.count > 0)).then(|| key_item_use(pos))
}

#[derive(Default)]
pub struct Vaults {
    pub enchanting: Option<Arc<dyn Enchanting>>,
    pub sounds: Vec<MobSound>,
    fail_times: HashMap<(i32, i32, i32), i64>,
}

pub struct LevelVaults<'a, 'l> {
    pub level: &'a mut Level<'l>,
    pub players: &'a [VaultPlayer],
    pub sounds: &'a mut Vec<MobSound>,
    pub enchanting: Option<&'a Arc<dyn Enchanting>>,
}

impl LevelVaults<'_, '_> {
    fn params(&self, vault: BlockPos) -> LootParams {
        LootParams {
            origin: Some(center(vault)),
            enchanting: self.enchanting.cloned(),
            ..LootParams::default()
        }
    }
}

impl VaultWorld for LevelVaults<'_, '_> {
    fn now(&self) -> i64 {
        self.level.game_time
    }

    fn players(&self) -> &[VaultPlayer] {
        self.players
    }

    fn random(&mut self) -> &mut dyn RandomSource {
        &mut self.level.random
    }

    fn display_items(&mut self, table: &str, vault: BlockPos) -> Vec<ItemStack> {
        let registries = self.level.lib.registries.clone();
        let params = self.params(vault);
        let level = &mut *self.level;
        let seed = level.random.next_i64();
        registries
            .loot
            .roll_seeded(
                &registries,
                table,
                &params,
                seed,
                &mut level.random_sequences,
                &mut level.random,
            )
            .unwrap_or_else(|_| Vec::new())
            .into_iter()
            .filter(|stack| !stack.is_empty())
            .collect()
    }

    fn rewards(&mut self, table: &str, vault: BlockPos, key: &ItemStack) -> Vec<ItemStack> {
        let registries = self.level.lib.registries.clone();
        let params = LootParams {
            tool: Some(key.clone()),
            this_entity: true,
            ..self.params(vault)
        };
        let level = &mut *self.level;
        registries
            .loot
            .roll(
                &registries,
                table,
                &params,
                &mut level.random_sequences,
                &mut level.random,
            )
            .unwrap_or_else(|_| Vec::new())
            .into_iter()
            .filter(|stack| !stack.is_empty())
            .collect()
    }

    fn eject(&mut self, vault: BlockPos, stack: ItemStack) {
        let at = [
            f64::from(vault.x) + 0.5,
            f64::from(vault.y) + EJECT_HEIGHT,
            f64::from(vault.z) + 0.5,
        ];
        self.level
            .spawn_dispensed_item(at, Direction::Up, EJECT_ACCURACY, stack);
    }

    fn sound(&mut self, vault: BlockPos, event: &'static str, pitch: f32) {
        self.sounds.push(MobSound {
            event: event.to_owned(),
            position: glam::DVec3::from_array(center(vault)),
            volume: 1.0,
            pitch,
            category: "blocks",
        });
    }

    fn level_event(&mut self, event: i32, vault: BlockPos, data: i32) {
        self.level.level_events.push((event, vault, data));
    }
}

pub fn ticking_vaults(level: &Level) -> Vec<(i32, i32, i32)> {
    let ticking: Vec<&Chunk> = match &level.ticking_chunks {
        Some(list) => list.iter().filter_map(|pos| level.chunk(*pos)).collect(),
        None => level.chunks().collect(),
    };
    let mut vaults: Vec<(i32, i32, i32)> = ticking
        .into_iter()
        .flat_map(|chunk| {
            chunk
                .block_entities
                .entities
                .iter()
                .filter(|(_, tag)| is_vault(tag))
                .map(|(key, _)| *key)
        })
        .collect();
    vaults.sort_unstable();
    vaults
}

fn placed(level: &Level, key: (i32, i32, i32)) -> Option<(VaultBlock, Tag)> {
    let pos = BlockPos::new(key.0, key.1, key.2);
    let blocks = &level.lib.registries.blocks;
    let block = level.block(pos);
    let state = blocks
        .property(block, STATE)
        .and_then(VaultState::from_name)?;
    let ominous = blocks.property(block, OMINOUS) == Some("true");
    let tag = level
        .chunk(pos.chunk())
        .and_then(|chunk| chunk.block_entities.entities.get(&key))
        .filter(|tag| is_vault(tag))
        .cloned()?;
    Some((
        VaultBlock {
            pos,
            state,
            ominous,
        },
        tag,
    ))
}

fn store(
    level: &mut Level,
    block: VaultBlock,
    tag: &Tag,
    changed: Option<&Vault>,
    next: VaultState,
) {
    if let Some(vault) = changed {
        let mut map = tag.as_compound().cloned().unwrap_or_else(BTreeMap::new);
        vault.write(&mut map);
        level.set_block_entity_tag(block.pos, Tag::Compound(map));
    }
    if next != block.state {
        let lib = level.lib;
        let state = level.block(block.pos);
        if let Some(state) = lib
            .registries
            .blocks
            .with_property(state, STATE, next.name())
        {
            level.set_block_and_update(block.pos, state);
        }
    }
}

pub fn tick_vaults(level: &mut Level, players: &[VaultPlayer], vaults: &mut Vaults) {
    for key in ticking_vaults(level) {
        let Some((block, tag)) = placed(level, key) else {
            continue;
        };
        let mut vault = Vault::read(&tag);
        let before = vault.clone();
        let next = vault.tick(
            block,
            &mut LevelVaults {
                level,
                players,
                sounds: &mut vaults.sounds,
                enchanting: vaults.enchanting.as_ref(),
            },
        );
        store(
            level,
            block,
            &tag,
            (vault != before).then_some(&vault),
            next,
        );
    }
}

pub fn insert_key(
    level: &mut Level,
    pos: BlockPos,
    player: u128,
    key: &ItemStack,
    players: &[VaultPlayer],
    vaults: &mut Vaults,
) -> bool {
    let Some((block, tag)) = placed(level, (pos.x, pos.y, pos.z)) else {
        return false;
    };
    let Vaults {
        enchanting,
        sounds,
        fail_times,
    } = vaults;
    let mut vault = Vault::read(&tag);
    let before = vault.clone();
    let last_fail = fail_times.entry((pos.x, pos.y, pos.z)).or_insert(0);
    let next = vault.insert_key(
        block,
        player,
        key,
        last_fail,
        &mut LevelVaults {
            level,
            players,
            sounds,
            enchanting: enchanting.as_ref(),
        },
    );
    store(
        level,
        block,
        &tag,
        (vault != before).then_some(&vault),
        next.unwrap_or(block.state),
    );
    next.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::random::LegacyRandom;

    struct World {
        now: i64,
        players: Vec<VaultPlayer>,
        random: LegacyRandom,
        rewards: Vec<ItemStack>,
        tables: Vec<String>,
        ejected: Vec<ItemStack>,
        sounds: Vec<(&'static str, f32)>,
        events: Vec<(i32, i32)>,
    }

    impl World {
        fn new() -> Self {
            Self {
                now: 1000,
                players: Vec::new(),
                random: LegacyRandom::new(7),
                rewards: vec![
                    ItemStack::new("minecraft:arrow", 8),
                    ItemStack::new("minecraft:emerald", 2),
                    ItemStack::new("minecraft:crossbow", 1),
                ],
                tables: Vec::new(),
                ejected: Vec::new(),
                sounds: Vec::new(),
                events: Vec::new(),
            }
        }

        fn at(&mut self, players: &[(u128, i32)]) {
            self.players = players
                .iter()
                .map(|&(uuid, x)| VaultPlayer {
                    uuid,
                    block: BlockPos::new(x, 64, 0),
                })
                .collect();
        }

        fn sounds(&mut self) -> Vec<(&'static str, f32)> {
            std::mem::take(&mut self.sounds)
        }
    }

    impl VaultWorld for World {
        fn now(&self) -> i64 {
            self.now
        }

        fn players(&self) -> &[VaultPlayer] {
            &self.players
        }

        fn random(&mut self) -> &mut dyn RandomSource {
            &mut self.random
        }

        fn display_items(&mut self, table: &str, _: BlockPos) -> Vec<ItemStack> {
            self.tables.push(format!("display {table}"));
            vec![
                ItemStack::new("minecraft:emerald", 3),
                ItemStack::new("minecraft:diamond", 1),
            ]
        }

        fn rewards(&mut self, table: &str, _: BlockPos, key: &ItemStack) -> Vec<ItemStack> {
            self.tables.push(format!("reward {table} {}", key.id));
            self.rewards.clone()
        }

        fn eject(&mut self, _: BlockPos, stack: ItemStack) {
            self.ejected.push(stack);
        }

        fn sound(&mut self, _: BlockPos, event: &'static str, pitch: f32) {
            self.sounds.push((event, pitch));
        }

        fn level_event(&mut self, event: i32, _: BlockPos, data: i32) {
            self.events.push((event, data));
        }
    }

    fn placed(state: VaultState) -> VaultBlock {
        VaultBlock {
            pos: BlockPos::new(0, 64, 0),
            state,
            ominous: false,
        }
    }

    fn plain() -> Vault {
        Vault {
            config: VaultConfig::default(),
            server: ServerData::default(),
            shared: SharedData::default(),
        }
    }

    fn run(
        vault: &mut Vault,
        block: &mut VaultBlock,
        world: &mut World,
        ticks: i64,
    ) -> Vec<(i64, VaultState)> {
        let mut changes = Vec::new();
        for _ in 0..ticks {
            let next = vault.tick(*block, world);
            if next != block.state {
                changes.push((world.now, next));
                block.state = next;
            }
            world.now += 1;
        }
        changes
    }

    fn key(id: &str) -> ItemStack {
        ItemStack::new(id, 1)
    }

    fn active(world: &mut World, players: &[(u128, i32)]) -> (Vault, VaultBlock) {
        let mut vault = plain();
        let mut block = placed(VaultState::Inactive);
        world.at(players);
        run(&mut vault, &mut block, world, 1);
        assert_eq!(block.state, VaultState::Active);
        world.sounds();
        world.events.clear();
        (vault, block)
    }

    #[test]
    fn a_vault_lights_for_any_player_within_four_blocks_and_dims_beyond_four_and_a_half() {
        let mut world = World::new();
        let mut vault = plain();
        let mut block = placed(VaultState::Inactive);
        world.at(&[(1, 4)]);
        assert!(run(&mut vault, &mut block, &mut world, 20).is_empty());
        assert_eq!(vault.server.state_updating_resumes_at, 1020);
        world.at(&[(1, 3)]);
        assert_eq!(
            run(&mut vault, &mut block, &mut world, 20),
            [(1020, VaultState::Active)]
        );
        assert_eq!(world.events, [(ACTIVATE, 0)]);
        assert_eq!(vault.shared.connected_players, [1]);
        assert!(vault.shared.display_item.is_some(), "it shows an item");
        world.at(&[(1, 4)]);
        assert!(run(&mut vault, &mut block, &mut world, 40).is_empty());
        world.at(&[(1, 5)]);
        assert_eq!(
            run(&mut vault, &mut block, &mut world, 20),
            [(1080, VaultState::Inactive)]
        );
        assert_eq!(world.events, [(ACTIVATE, 0), (DEACTIVATE, 0)]);
        assert_eq!(vault.shared.display_item, None);
        assert!(vault.shared.connected_players.is_empty());
        assert!(
            world.sounds.is_empty(),
            "lighting up plays nothing on the server"
        );
    }

    #[test]
    fn a_trial_key_unlocks_it_and_it_ejects_one_item_a_second_rising_in_pitch() {
        let mut world = World::new();
        let (mut vault, mut block) = active(&mut world, &[(1, 2)]);
        let mut fail = 0;
        let unlocked = vault.insert_key(block, 1, &key(TRIAL_KEY), &mut fail, &mut world);
        assert_eq!(unlocked, Some(VaultState::Unlocking));
        block.state = VaultState::Unlocking;
        assert_eq!(world.sounds(), [(INSERT_ITEM_SOUND, 1.0)]);
        assert_eq!(
            world.tables,
            [
                "display minecraft:chests/trial_chambers/reward",
                "reward minecraft:chests/trial_chambers/reward minecraft:trial_key"
            ]
        );
        assert_eq!(vault.server.state_updating_resumes_at, 1015);
        assert_eq!(vault.server.rewarded_players, [1]);
        assert_eq!(vault.server.total_ejections_needed, 3);
        assert!(
            vault.shared.connected_players.is_empty(),
            "a rewarded player is not connected"
        );
        assert_eq!(
            vault.shared.display_item,
            Some(ItemStack::new("minecraft:crossbow", 1))
        );
        let changes = run(&mut vault, &mut block, &mut world, 120);
        assert_eq!(
            changes,
            [(1015, VaultState::Ejecting), (1095, VaultState::Inactive)]
        );
        let ids: Vec<&str> = world
            .ejected
            .iter()
            .map(|stack| stack.id.as_str())
            .collect();
        assert_eq!(
            ids,
            ["minecraft:crossbow", "minecraft:emerald", "minecraft:arrow"]
        );
        assert_eq!(
            world.sounds(),
            [
                (OPEN_SHUTTER_SOUND, 1.0),
                (EJECT_ITEM_SOUND, 0.8),
                (EJECT_ITEM_SOUND, 1.0),
                (EJECT_ITEM_SOUND, 1.2),
                (CLOSE_SHUTTER_SOUND, 1.0)
            ]
        );
        assert_eq!(
            world.events,
            [
                (EJECT_PARTICLES, 0),
                (EJECT_PARTICLES, 0),
                (EJECT_PARTICLES, 0),
                (DEACTIVATE, 0)
            ]
        );
        assert_eq!(vault.server.total_ejections_needed, 0);
        assert_eq!(vault.shared.display_item, None);
        assert_eq!(
            vault.insert_key(block, 1, &key(TRIAL_KEY), &mut fail, &mut world),
            None
        );
        assert!(world.sounds.is_empty(), "an inactive vault ignores a key");
        assert!(
            run(&mut vault, &mut block, &mut world, 40).is_empty(),
            "it stays dim for its rewarded player"
        );
    }

    #[test]
    fn a_single_ejection_plays_at_the_highest_pitch() {
        let mut world = World::new();
        world.rewards = vec![ItemStack::new("minecraft:diamond", 1)];
        let (mut vault, mut block) = active(&mut world, &[(1, 2)]);
        vault.insert_key(block, 1, &key(TRIAL_KEY), &mut 0, &mut world);
        block.state = VaultState::Unlocking;
        run(&mut vault, &mut block, &mut world, 40);
        assert!(world.sounds().contains(&(EJECT_ITEM_SOUND, 1.2)));
    }

    #[test]
    fn a_rewarded_player_is_refused_while_another_keeps_it_lit() {
        let mut world = World::new();
        let (mut vault, mut block) = active(&mut world, &[(1, 2), (2, -2)]);
        let mut fail = 0;
        assert!(
            vault
                .insert_key(block, 1, &key(TRIAL_KEY), &mut fail, &mut world)
                .is_some()
        );
        assert_eq!(vault.shared.connected_players, [2]);
        block.state = VaultState::Unlocking;
        let changes = run(&mut vault, &mut block, &mut world, 120);
        assert_eq!(
            changes,
            [(1015, VaultState::Ejecting), (1095, VaultState::Active)]
        );
        let sounds = world.sounds();
        assert_eq!(sounds.last(), Some(&(CLOSE_SHUTTER_SOUND, 1.0)));
        assert_eq!(world.events.last(), Some(&(ACTIVATE, 0)));
        assert!(vault.shared.display_item.is_some());
        assert_eq!(
            vault.insert_key(block, 1, &key(TRIAL_KEY), &mut fail, &mut world),
            None
        );
        assert_eq!(world.sounds(), [(REJECT_REWARDED_PLAYER_SOUND, 1.0)]);
        assert_eq!(
            vault.insert_key(block, 2, &key(TRIAL_KEY), &mut fail, &mut world),
            Some(VaultState::Unlocking)
        );
        assert_eq!(vault.server.rewarded_players, [1, 2]);
    }

    #[test]
    fn a_wrong_key_fails_with_its_sound_at_most_once_in_fifteen_ticks() {
        let mut world = World::new();
        let (mut vault, block) = active(&mut world, &[(1, 2)]);
        let mut fail = 0;
        let mut named = key(TRIAL_KEY);
        let mut components = BTreeMap::new();
        components.insert(
            "minecraft:custom_name".to_owned(),
            Tag::String("Key".to_owned()),
        );
        named.components = Some(Tag::Compound(components));
        for (tick, stack, sounds) in [
            (1001, key(OMINOUS_TRIAL_KEY), 1),
            (1005, key(OMINOUS_TRIAL_KEY), 0),
            (1015, key("minecraft:stick"), 0),
            (1016, named.clone(), 1),
            (1020, ItemStack::new(TRIAL_KEY, 0), 0),
        ] {
            world.now = tick;
            assert_eq!(
                vault.insert_key(block, 1, &stack, &mut fail, &mut world),
                None,
                "{stack:?}"
            );
            assert_eq!(world.sounds().len(), sounds, "at {tick}");
        }
        assert!(vault.server.rewarded_players.is_empty());
        assert!(
            world
                .tables
                .iter()
                .all(|table| table.starts_with("display")),
            "nothing rolled"
        );
    }

    #[test]
    fn an_ominous_vault_wants_an_ominous_key_and_rolls_its_own_table() {
        let mut config = BTreeMap::new();
        config.insert(
            "loot_table".to_owned(),
            Tag::String("minecraft:chests/trial_chambers/reward_ominous".to_owned()),
        );
        config.insert("key_item".to_owned(), stack_tag(&key(OMINOUS_TRIAL_KEY)));
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(ID.to_owned()));
        map.insert("config".to_owned(), Tag::Compound(config));
        let mut vault = Vault::read(&Tag::Compound(map));
        let mut world = World::new();
        world.at(&[(1, 1)]);
        let mut block = VaultBlock {
            ominous: true,
            ..placed(VaultState::Inactive)
        };
        run(&mut vault, &mut block, &mut world, 1);
        assert_eq!(world.events, [(ACTIVATE, 1)]);
        let mut fail = 0;
        assert_eq!(
            vault.insert_key(block, 1, &key(TRIAL_KEY), &mut fail, &mut world),
            None
        );
        assert_eq!(world.sounds(), [(INSERT_ITEM_FAIL_SOUND, 1.0)]);
        assert!(
            vault
                .insert_key(block, 1, &key(OMINOUS_TRIAL_KEY), &mut fail, &mut world)
                .is_some()
        );
        assert_eq!(
            world.tables.last().map(String::as_str),
            Some(
                "reward minecraft:chests/trial_chambers/reward_ominous minecraft:ominous_trial_key"
            )
        );
        block.state = VaultState::Unlocking;
        run(&mut vault, &mut block, &mut world, 120);
        assert_eq!(world.events.last(), Some(&(DEACTIVATE, 1)));
    }

    #[test]
    fn the_display_turns_over_every_twenty_ticks_only_while_active() {
        let mut world = World::new();
        world.now = 990;
        let mut vault = plain();
        vault.config.override_loot_table_to_display = Some("minecraft:shown".to_owned());
        let mut block = placed(VaultState::Inactive);
        assert!(run(&mut vault, &mut block, &mut world, 30).is_empty());
        assert!(world.tables.is_empty(), "nothing shown while dim");
        world.at(&[(1, 0)]);
        run(&mut vault, &mut block, &mut world, 70);
        assert_eq!(block.state, VaultState::Active);
        assert_eq!(
            world.tables, ["display minecraft:shown"; 4],
            "once on lighting up, then at 1040, 1060 and 1080"
        );
        world.at(&[]);
        run(&mut vault, &mut block, &mut world, 40);
        assert_eq!(block.state, VaultState::Inactive);
        assert_eq!(
            world.tables.len(),
            4,
            "it goes dim at 1090 and turns nothing at 1100"
        );
        assert_eq!(vault.shared.display_item, None);
    }

    #[test]
    fn a_vault_without_a_key_shows_nothing_and_takes_nothing() {
        let mut map = BTreeMap::new();
        map.insert("config".to_owned(), Tag::Compound(BTreeMap::new()));
        let mut vault = Vault::read(&Tag::Compound(map));
        assert_eq!(vault.config.key_item, None);
        let mut world = World::new();
        world.at(&[(1, 1)]);
        let mut block = placed(VaultState::Inactive);
        run(&mut vault, &mut block, &mut world, 1);
        assert_eq!(block.state, VaultState::Active);
        assert_eq!(vault.shared.display_item, None);
        assert_eq!(
            vault.insert_key(block, 1, &key(TRIAL_KEY), &mut 0, &mut world),
            None
        );
        assert!(world.sounds.is_empty());
        let mut wrong = BTreeMap::new();
        wrong.insert("activation_range".to_owned(), Tag::Double(6.0));
        wrong.insert("key_item".to_owned(), stack_tag(&key(OMINOUS_TRIAL_KEY)));
        assert_eq!(
            VaultConfig::read(Some(&Tag::Compound(wrong))),
            VaultConfig::default(),
            "ranges the wrong way round"
        );
        assert_eq!(VaultConfig::read(None), VaultConfig::default());
    }

    #[test]
    fn the_newest_128_rewarded_players_are_kept() {
        let mut server = ServerData::default();
        for player in 0..129 {
            server.reward(player);
        }
        server.reward(5);
        assert_eq!(server.rewarded_players.len(), 128);
        assert_eq!(server.rewarded_players.first(), Some(&1));
        assert_eq!(server.rewarded_players.last(), Some(&128));
    }

    #[test]
    fn the_tag_round_trips_and_leaves_defaults_out() {
        let vault = Vault {
            config: VaultConfig::default(),
            server: ServerData {
                rewarded_players: vec![0x1234_5678_9abc_def0_0fed_cba9_8765_4321, 7],
                state_updating_resumes_at: 1_000_014,
                items_to_eject: vec![
                    ItemStack::new("minecraft:emerald", 3),
                    ItemStack::new("minecraft:bow", 1),
                ],
                total_ejections_needed: 3,
            },
            shared: SharedData {
                display_item: Some(ItemStack::new("minecraft:bow", 1)),
                connected_players: vec![9],
                connected_particles_range: 6.0,
            },
        };
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(ID.to_owned()));
        vault.write(&mut map);
        let tag = Tag::Compound(map);
        assert_eq!(Vault::read(&tag), vault);
        let mut empty = BTreeMap::new();
        plain().write(&mut empty);
        assert_eq!(
            empty.get("server_data"),
            Some(&Tag::Compound(BTreeMap::new()))
        );
        assert_eq!(
            empty.get("shared_data"),
            Some(&Tag::Compound(BTreeMap::new()))
        );
    }

    #[test]
    fn a_key_is_offered_only_to_an_active_vault_from_a_full_hand() {
        let vault = |state: &str| crate::scene::Block::new(ID).with(STATE, state);
        let held = minecraftoss_player::inventory::ItemStack::new(TRIAL_KEY, 1);
        let used =
            key_use((1, 2, 3), &vault("active"), Some(&held)).expect("an active vault takes a key");
        assert_eq!(used.change, Change::Vault((1, 2, 3)));
        assert!(used.consumed && used.result.is_none() && used.wear == 0);
        assert_eq!(
            key_use((1, 2, 3), &vault("active"), None),
            None,
            "an empty hand"
        );
        for state in ["inactive", "unlocking", "ejecting"] {
            assert_eq!(
                key_use((1, 2, 3), &vault(state), Some(&held)),
                None,
                "{state}"
            );
        }
        let stone = crate::scene::Block::new("minecraft:stone");
        assert_eq!(key_use((1, 2, 3), &stone, Some(&held)), None);
    }
}
