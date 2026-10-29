use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use minecraft_commands::{Clientbound, Text};
use minecraft_net::messages::{
    BookView, BossBarColor, BossBarFlags, BossBarOperation, BossBarOverlay, BossBarUpdate, GearView,
    Login, Lying, MenuView, Nbt, PlayerView, Rules,
};
use minecraft_net::{ChunkBatchBlob, Dimension, Lane, PlayerUuid, StateFrame, ToClient};
use minecraft_terrain::ChunkEvent;
use minecraft_terrain::client_mobs::ServerMob;
use minecraft_terrain::server::EntitySnapshot;
use minecraft_terrain::server::{OVERWORLD_CLOCK, THE_END_CLOCK};
use minecraft_terrain::terrain::BlockStates;
use minecraftoss_core::{BlockStateId, Chunk, ChunkPos, Registries};
use minecraftoss_player::crafting::RecipeBook;

use crate::level::Terrain;

pub const QUEUE_LIMIT: usize = 64 * 1024 * 1024;
pub const LOST_STREAM: &str = "Lost the world stream";

type BlockPos = (i32, i32, i32);

pub struct RemoteLevel {
    pub dimension: Dimension,
    pub epoch: u32,
    pub origin: [f64; 3],
    pub feet: [f64; 3],
    pub terrain: Terrain,
    pub block_entities: BTreeMap<BlockPos, Nbt>,
    pub mobs: BTreeMap<u64, Nbt>,
    pub view: crate::entity_view::EntityView,
    pub synced_game_time: Option<i64>,
}

impl RemoteLevel {
    fn add_mob(&mut self, id: u64, tag: Nbt) {
        let world = &mut self.view.world;
        let spawned = minecraftoss_core::nbt::parse(&tag.0)
            .is_ok_and(|parsed| minecraft_terrain::mob_tracking::spawn_tracked(world, id, &parsed));
        if !spawned {
            diag::warn!(World, "Minecraft mob {id} from the host not shown");
        }
        self.mobs.insert(id, tag);
    }

    fn show_state(&mut self, frame: &StateFrame, recipes: &RecipeBook) {
        if let Ok(mobs) = minecraft_net::decode::<Vec<ServerMob>>(Lane::State, &frame.mobs.0) {
            let mobs: Vec<ServerMob> = mobs
                .into_iter()
                .filter(|mob| self.mobs.contains_key(&mob.id))
                .collect();
            for mob in &mobs {
                minecraft_terrain::mob_tracking::show_on(&mut self.view.world, mob);
            }
            let world = &self.view.world;
            let mobs: Vec<ServerMob> = mobs
                .into_iter()
                .filter(|mob| !minecraft_terrain::mob_tracking::is_projectile(world, mob.id))
                .collect();
            self.view.show_mobs(mobs);
        }
        if let Ok(mut snapshot) =
            minecraft_net::decode::<EntitySnapshot>(Lane::State, &frame.entities.0)
        {
            self.view.receive(&mut snapshot, &[]);
            self.view
                .show_items(std::mem::take(&mut snapshot.items), recipes);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossBar {
    pub name: Text,
    pub progress: f32,
    pub color: BossBarColor,
    pub overlay: BossBarOverlay,
    pub flags: BossBarFlags,
}

#[derive(Default)]
pub struct Shown {
    pub level: bool,
    pub loaded: Vec<Arc<Chunk>>,
    pub forgotten: Vec<ChunkPos>,
    pub blocks: Vec<(BlockPos, BlockStateId)>,
}

pub struct DecodeJob {
    pub seq: u32,
    pub epoch: u32,
    pub blob: Vec<u8>,
    pub states: Arc<BlockStates>,
}

impl DecodeJob {
    pub fn decode(&self) -> Result<Vec<Arc<Chunk>>, String> {
        minecraft_terrain::chunk_wire::decode_batch(&self.blob, &self.states)
            .map(|(_, chunks)| chunks)
            .map_err(|error| format!("{error:#}"))
    }
}

struct HeldBatch {
    blob: Vec<u8>,
    decoding: Option<u32>,
    chunks: Option<(u32, Vec<Arc<Chunk>>)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Clock {
    pub ticks: f64,
    pub paused: bool,
}

impl Clock {
    fn advance(&mut self, seconds: f64) {
        if !self.paused {
            self.ticks += seconds * 20.0;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Clocks {
    pub overworld: Clock,
    pub the_end: Clock,
}

impl Clocks {
    pub fn advance(&mut self, seconds: f64) {
        self.overworld.advance(seconds);
        self.the_end.advance(seconds);
    }

    pub fn ticks(&self, dimension: Dimension) -> i64 {
        match crate::server::default_clock(dimension) {
            Some(THE_END_CLOCK) => self.the_end.ticks.floor() as i64,
            _ => self.overworld.ticks.floor() as i64,
        }
    }

    fn set(&mut self, name: &str, ticks: f64, paused: bool) {
        let clock = match name {
            OVERWORLD_CLOCK => &mut self.overworld,
            THE_END_CLOCK => &mut self.the_end,
            _ => return,
        };
        *clock = Clock { ticks, paused };
    }
}

pub struct RemoteWorld {
    registries: Arc<Registries>,
    pub login: Option<Login>,
    pub level: Option<RemoteLevel>,
    pub rejected: Option<Text>,
    pub failed: Option<String>,
    pub players: BTreeMap<u32, (String, PlayerUuid)>,
    pub chat: Vec<Clientbound>,
    pub command_level: u8,
    pub player: Option<PlayerView>,
    pub menu: Option<MenuView>,
    pub inventory_menu: Option<MenuView>,
    pub gear: BTreeMap<u32, GearView>,
    pub rules: Rules,
    pub game_time: i64,
    pub clocks: Clocks,
    pub weather: [f32; 2],
    pub spawn: Option<(Dimension, [i32; 3])>,
    pub boss_bars: BTreeMap<u64, BossBar>,
    pub lecterns: Vec<Option<BookView>>,
    pub open_books: Vec<u8>,
    pub sleepers: BTreeMap<u32, Lying>,
    pub sleep_timer: i32,
    sleep_clock: f64,
    pub effects: Vec<ToClient>,
    pub state: Option<StateFrame>,
    pub recipes: Arc<RecipeBook>,
    shown: Shown,
    reliable: VecDeque<(ToClient, usize)>,
    batches: BTreeMap<u32, HeldBatch>,
    applied_seq: u32,
    held: usize,
    acks: Vec<u32>,
}

impl RemoteWorld {
    pub fn new(registries: Arc<Registries>) -> Self {
        Self {
            registries,
            login: None,
            level: None,
            rejected: None,
            failed: None,
            players: BTreeMap::new(),
            chat: Vec::new(),
            command_level: 0,
            player: None,
            menu: None,
            inventory_menu: None,
            gear: BTreeMap::new(),
            rules: Rules::default(),
            game_time: 0,
            clocks: Clocks::default(),
            weather: [0.0; 2],
            spawn: None,
            boss_bars: BTreeMap::new(),
            lecterns: Vec::new(),
            open_books: Vec::new(),
            sleepers: BTreeMap::new(),
            sleep_timer: 0,
            sleep_clock: 0.0,
            effects: Vec::new(),
            state: None,
            recipes: Arc::new(RecipeBook::default()),
            shown: Shown::default(),
            reliable: VecDeque::new(),
            batches: BTreeMap::new(),
            applied_seq: 0,
            held: 0,
            acks: Vec::new(),
        }
    }

    pub fn held_bytes(&self) -> usize {
        self.held
    }

    pub fn waiting(&self) -> usize {
        self.reliable.len()
    }

    pub fn take_shown(&mut self) -> Shown {
        std::mem::take(&mut self.shown)
    }

    pub fn dial_context(&self, feet: [f64; 3], yaw: f32) -> Option<minecraft_terrain::dials::DialContext> {
        let level = self.level.as_ref()?;
        let point = |(dimension, [x, y, z]): (Dimension, [i32; 3])| (dimension, (x, y, z));
        Some(minecraft_terrain::dials::DialContext {
            dimension: level.dimension,
            feet,
            yaw,
            spawn: self.spawn.map(point),
            last_death: self.player.as_ref().and_then(|view| view.last_death).map(point),
            sun_angle: level.terrain.environment.sun_angle_degrees(1.0),
        })
    }

    pub fn take_acks(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.acks)
    }

    pub fn take_chat(&mut self) -> Vec<Clientbound> {
        std::mem::take(&mut self.chat)
    }

    pub fn take_effects(&mut self) -> Vec<ToClient> {
        std::mem::take(&mut self.effects)
    }

    pub fn apply(&mut self, message: ToClient) {
        if self.failed.is_some() {
            return;
        }
        let waiting = if self.reliable.is_empty() {
            self.try_apply(message)
        } else {
            Some(message)
        };
        if let Some(message) = waiting {
            let size = minecraft_net::encode(Lane::Reliable, &message)
                .map_or(1, |bytes| bytes.len().max(1));
            self.held += size;
            self.reliable.push_back((message, size));
            self.check_limit();
            self.pump();
        }
    }

    pub fn apply_state(&mut self, frame: StateFrame) -> bool {
        let Some(level) = self.level.as_ref() else {
            return false;
        };
        let older = self
            .state
            .as_ref()
            .is_some_and(|last| frame.tick <= last.tick);
        if frame.epoch != level.epoch || older {
            return false;
        }
        if let Some(level) = self.level.as_mut() {
            level.show_state(&frame, &self.recipes);
        }
        self.state = Some(frame);
        true
    }

    pub fn receive_batch(&mut self, blob: Vec<u8>) {
        if self.failed.is_some() {
            return;
        }
        let Some(seq) = ChunkBatchBlob::peek_seq(&blob) else {
            diag::warn!(World, "Minecraft chunk batch without a header dropped");
            return;
        };
        if seq <= self.applied_seq {
            self.acks.push(seq);
            return;
        }
        if self.batches.contains_key(&seq) {
            return;
        }
        self.held += blob.len();
        self.batches.insert(
            seq,
            HeldBatch {
                blob,
                decoding: None,
                chunks: None,
            },
        );
        self.check_limit();
    }

    pub fn take_decode_jobs(&mut self) -> Vec<DecodeJob> {
        let Some(level) = self.level.as_ref() else {
            return Vec::new();
        };
        let (epoch, states) = (level.epoch, level.terrain.stream.states.clone());
        self.batches
            .iter_mut()
            .filter(|(_, held)| {
                held.decoding.is_none()
                    && held
                        .chunks
                        .as_ref()
                        .is_none_or(|(decoded, _)| *decoded != epoch)
            })
            .map(|(&seq, held)| {
                held.decoding = Some(epoch);
                held.chunks = None;
                DecodeJob {
                    seq,
                    epoch,
                    blob: held.blob.clone(),
                    states: states.clone(),
                }
            })
            .collect()
    }

    pub fn decoded(&mut self, seq: u32, epoch: u32, result: Result<Vec<Arc<Chunk>>, String>) {
        if let Some(held) = self.batches.get_mut(&seq)
            && held.decoding == Some(epoch)
        {
            held.decoding = None;
            let chunks = result.unwrap_or_else(|error| {
                diag::warn!(World, "Minecraft chunk batch {seq} not read: {error}");
                Vec::new()
            });
            held.chunks = Some((epoch, chunks));
        }
        self.pump();
    }

    pub fn decode_now(&mut self) {
        for job in self.take_decode_jobs() {
            let result = job.decode();
            self.decoded(job.seq, job.epoch, result);
        }
    }

    pub fn advance_clocks(&mut self, seconds: f64) {
        self.clocks.advance(seconds);
        self.sleep_clock = (self.sleep_clock + seconds.max(0.0) * 20.0).min(20.0);
        let sleeping = self.lying().is_some();
        while self.sleep_clock >= 1.0 {
            self.sleep_clock -= 1.0;
            self.sleep_timer = crate::sleep::next_sleep_timer(self.sleep_timer, sleeping);
        }
    }

    pub fn lying(&self) -> Option<Lying> {
        let client = self.login.as_ref()?.client;
        self.sleepers.get(&client).copied()
    }

    fn check_limit(&mut self) {
        if self.held > QUEUE_LIMIT && self.failed.is_none() {
            diag::warn!(
                World,
                "Minecraft stream holds {} bytes behind a missing chunk batch",
                self.held
            );
            self.failed = Some(LOST_STREAM.to_owned());
        }
    }

    fn pump(&mut self) {
        while self.failed.is_none()
            && let Some((message, size)) = self.reliable.pop_front()
        {
            self.held = self.held.saturating_sub(size);
            if let Some(waiting) = self.try_apply(message) {
                self.held += size;
                self.reliable.push_front((waiting, size));
                break;
            }
        }
    }

    fn try_apply(&mut self, message: ToClient) -> Option<ToClient> {
        match message {
            ToClient::Rejected { reason } => self.rejected = Some(reason),
            ToClient::Login(login) => self.start(login),
            message if self.login.is_none() => return Some(message),
            ToClient::ChunkBatch { seq } => {
                if !self.apply_batch(seq) {
                    return Some(ToClient::ChunkBatch { seq });
                }
            }
            ToClient::ChunkForget { pos } => self.forget(ChunkPos::new(pos[0], pos[1])),
            ToClient::BlockUpdates { changes } => self.set_blocks(&changes),
            ToClient::BlockEntity { pos, tag } => {
                if let Some(level) = self.level.as_mut() {
                    let at = (pos[0], pos[1], pos[2]);
                    if let Ok(parsed) = minecraftoss_core::nbt::parse(&tag.0) {
                        level.view.vaults.block_entities(&[(at, Some(parsed.clone()))]);
                        level.view.spawners.block_entities(&[(at, Some(parsed))]);
                    }
                    level.block_entities.insert(at, tag);
                }
            }
            ToClient::MobAdd { id, tag } => {
                if let Some(level) = self.level.as_mut() {
                    level.add_mob(id, tag);
                }
            }
            ToClient::MobLook { id, look } => {
                if let Some(level) = self.level.as_mut()
                    && level.mobs.contains_key(&id)
                {
                    level.add_mob(id, Nbt(look.0));
                }
            }
            ToClient::MobRemove { ids } => {
                if let Some(level) = self.level.as_mut() {
                    for id in &ids {
                        level.mobs.remove(id);
                    }
                    level.view.world.remove_mobs(&ids);
                }
            }
            ToClient::Player(view) => self.player = Some(*view),
            ToClient::MenuOpen(view) if view.menu == crate::menus::INVENTORY_MENU => {
                self.inventory_menu = Some(view);
            }
            ToClient::MenuOpen(view) => self.menu = Some(view),
            ToClient::MenuSlots {
                menu,
                state,
                slots,
                carried,
                data,
            } => {
                let view = if menu == crate::menus::INVENTORY_MENU {
                    self.inventory_menu.as_mut()
                } else {
                    self.menu.as_mut().filter(|view| view.menu == menu)
                };
                if let Some(view) = view {
                    crate::menus::apply_slots(view, state, slots, carried, data);
                }
            }
            ToClient::Equipment { client, gear } => {
                self.gear.insert(client, gear);
            }
            ToClient::MenuClose { menu } => {
                if self.menu.as_ref().is_some_and(|view| view.menu == menu) {
                    self.menu = None;
                }
            }
            ToClient::Chat(Clientbound::Commands { level }) => self.command_level = level,
            ToClient::Chat(line) => self.chat.push(line),
            ToClient::PlayerList { added, removed } => {
                for client in removed {
                    self.players.remove(&client);
                }
                for (client, name, uuid) in added {
                    self.players.insert(client, (name, uuid));
                }
            }
            ToClient::Time { game_time, clocks } => {
                self.game_time = game_time;
                for (name, ticks, paused) in clocks {
                    self.clocks.set(&name, ticks, paused);
                }
            }
            ToClient::Weather { rain, thunder } => self.weather = [rain, thunder],
            ToClient::SpawnPosition { dimension, pos } => self.spawn = Some((dimension, pos)),
            ToClient::Dimension {
                epoch,
                dimension,
                origin,
                feet,
            } => self.open_level(dimension, epoch, origin, feet),
            ToClient::BossBar(update) => self.boss_bar(update),
            ToClient::Lectern(book) => self.lecterns.push(book),
            ToClient::OpenBook { slot } => self.open_books.push(slot),
            ToClient::Sleeping { client, bed, timer } => {
                if self.login.as_ref().is_some_and(|login| login.client == client) {
                    self.sleep_timer = timer;
                }
                match bed {
                    Some(lying) => {
                        self.sleepers.insert(client, lying);
                    }
                    None => {
                        self.sleepers.remove(&client);
                    }
                }
            }
            ToClient::Bursts { entities } => {
                let bursts = minecraft_net::decode::<EntitySnapshot>(Lane::Reliable, &entities.0);
                if let (Some(level), Ok(mut bursts)) = (self.level.as_mut(), bursts) {
                    level.view.bursts(&mut bursts);
                }
            }
            other => self.effects.push(other),
        }
        None
    }

    fn boss_bar(&mut self, update: BossBarUpdate) {
        let BossBarUpdate { id, operation } = update;
        let operation = match operation {
            BossBarOperation::Add {
                name,
                progress,
                color,
                overlay,
                flags,
            } => {
                let bar = BossBar {
                    name,
                    progress,
                    color,
                    overlay,
                    flags,
                };
                self.boss_bars.insert(id, bar);
                return;
            }
            BossBarOperation::Remove => {
                self.boss_bars.remove(&id);
                return;
            }
            operation => operation,
        };
        let Some(bar) = self.boss_bars.get_mut(&id) else {
            return;
        };
        match operation {
            BossBarOperation::Progress(progress) => bar.progress = progress,
            BossBarOperation::Name(name) => bar.name = name,
            BossBarOperation::Style { color, overlay } => {
                bar.color = color;
                bar.overlay = overlay;
            }
            BossBarOperation::Flags(flags) => bar.flags = flags,
            BossBarOperation::Add { .. } | BossBarOperation::Remove => {}
        }
    }

    fn start(&mut self, login: Login) {
        if self.login.is_some() {
            diag::warn!(World, "Minecraft host sent a second login, ignored");
            return;
        }
        let origin = login
            .origins
            .iter()
            .find(|(dimension, _)| *dimension == login.dimension)
            .map_or(login.feet, |(_, origin)| *origin);
        let (dimension, feet) = (login.dimension, login.feet);
        self.rules = login.rules;
        self.login = Some(login);
        self.open_level(dimension, 0, origin, feet);
    }

    fn open_level(&mut self, dimension: Dimension, epoch: u32, origin: [f64; 3], feet: [f64; 3]) {
        let Some(login) = self.login.as_ref() else {
            return;
        };
        let view_distance = i32::from(login.view_distance);
        self.level = None;
        self.state = None;
        self.sleepers.clear();
        self.shown = Shown {
            level: true,
            ..Shown::default()
        };
        match Terrain::remote(
            self.registries.clone(),
            login.seed,
            view_distance,
            dimension,
        ) {
            Ok(mut terrain) => {
                terrain.stream.player_spawn = (feet[0], feet[1], feet[2]);
                self.level = Some(RemoteLevel {
                    dimension,
                    epoch,
                    origin,
                    feet,
                    terrain,
                    block_entities: BTreeMap::new(),
                    mobs: BTreeMap::new(),
                    view: crate::entity_view::EntityView::new(login.seed),
                    synced_game_time: None,
                });
                diag::info!(
                    World,
                    "Minecraft level {} from the host, origin {origin:?}, feet {feet:?}",
                    dimension.dimension_type()
                );
            }
            Err(error) => {
                diag::warn!(
                    World,
                    "Minecraft level {} not built: {error}",
                    dimension.dimension_type()
                );
                self.failed = Some(error);
            }
        }
    }

    fn apply_batch(&mut self, seq: u32) -> bool {
        if seq <= self.applied_seq {
            return true;
        }
        let Some(level) = self.level.as_mut() else {
            return false;
        };
        let ready = self
            .batches
            .get(&seq)
            .and_then(|held| held.chunks.as_ref())
            .is_some_and(|(epoch, _)| *epoch == level.epoch);
        if !ready {
            return false;
        }
        let Some(held) = self.batches.remove(&seq) else {
            return false;
        };
        self.held = self.held.saturating_sub(held.blob.len());
        let chunks = held.chunks.map_or_else(Vec::new, |(_, chunks)| chunks);
        let terrain = &mut level.terrain;
        let (loaded, forgotten) = terrain
            .stream
            .apply(chunks.into_iter().map(ChunkEvent::Load), &mut terrain.scene);
        terrain.scene.take_changed();
        for chunk in &loaded {
            level.view.spawners.load_chunk(chunk);
            level.view.vaults.load_chunk(chunk);
        }
        for pos in &forgotten {
            level.view.spawners.unload_chunk(*pos);
            level.view.vaults.unload_chunk(*pos);
        }
        self.shown.loaded.extend(loaded);
        self.shown.forgotten.extend(forgotten);
        self.applied_seq = seq;
        self.acks.push(seq);
        true
    }

    fn forget(&mut self, pos: ChunkPos) {
        let Some(level) = self.level.as_mut() else {
            return;
        };
        let terrain = &mut level.terrain;
        let (_, forgotten) = terrain
            .stream
            .apply([ChunkEvent::Forget(pos)], &mut terrain.scene);
        terrain.scene.take_changed();
        level
            .block_entities
            .retain(|&(x, _, z), _| (x >> 4, z >> 4) != (pos.x, pos.z));
        for pos in &forgotten {
            level.view.spawners.unload_chunk(*pos);
            level.view.vaults.unload_chunk(*pos);
        }
        self.shown.forgotten.extend(forgotten);
    }

    fn set_blocks(&mut self, changes: &[([i32; 3], u32)]) {
        let Some(level) = self.level.as_mut() else {
            return;
        };
        let count = self.registries.blocks.state_count();
        let states = level.terrain.stream.states.clone();
        let mut positions = Vec::new();
        for &(pos, state) in changes {
            let Some(state) = u16::try_from(state)
                .ok()
                .filter(|&state| usize::from(state) < count)
                .map(BlockStateId)
            else {
                diag::warn!(World, "Minecraft block update with state {state} dropped");
                continue;
            };
            let at = (pos[0], pos[1], pos[2]);
            if level
                .terrain
                .scene
                .generated_chunk((at.0 >> 4, at.2 >> 4))
                .is_none()
            {
                continue;
            }
            let block = states.block(state).cloned();
            if !block.as_ref().is_some_and(minecraft_terrain::spawner_display::is_displayed_block) {
                level.view.spawners.block_entities(&[(at, None)]);
            }
            if !block.as_ref().is_some_and(minecraft_terrain::vault::is_vault_block) {
                level.view.vaults.block_entities(&[(at, None)]);
            }
            level.terrain.scene.set(at, block);
            positions.push(at);
            self.shown.blocks.push((at, state));
        }
        level.terrain.scene.take_changed();
        if !positions.is_empty() {
            level
                .terrain
                .stream
                .mark_edited(&level.terrain.scene, &positions);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::Level;
    use crate::server::Server;
    use crate::server::tests::{registries, test_globals, world};
    use crate::session::{Answer, Hello, JoinContext, Sessions, encode_batch};
    use minecraft_net::PROTOCOL;
    use minecraft_net::messages::{MenuExtras, MenuKind};
    use minecraftoss_core::BiomeId;
    use minecraftoss_core::chunk::ChunkStatus;
    use std::time::{Duration, Instant};

    #[test]
    fn a_body_moving_in_powder_snow_makes_snowflakes_on_any_game() {
        struct Snow;
        impl minecraftoss_player::World for Snow {
            fn block(&self, pos: (i32, i32, i32)) -> Option<minecraftoss_player::Block> {
                let snow = pos.1 == 1 || pos.1 == 2;
                snow.then(|| minecraftoss_player::Block::new("minecraft:powder_snow"))
            }

            fn set_block(&mut self, _: (i32, i32, i32), _: Option<minecraftoss_player::Block>) {}
        }
        let mut inside = crate::entity_view::EntityView::new(7);
        let mut above = crate::entity_view::EntityView::new(7);
        for _ in 0..40 {
            inside.snowflakes(&Snow, [0.5, 1.2, 0.5], 1.8);
            above.snowflakes(&Snow, [0.5, 3.5, 0.5], 1.8);
        }
        let made = inside.ambient.len();
        assert!((20..=60).contains(&made), "half of 80 rolls, two blocks a tick: {made}");
        assert_eq!(above.ambient.len(), 0, "the feet must be in the snow");
    }

    fn login(dimension: Dimension) -> Login {
        Login {
            client: 1,
            uuid: PlayerUuid("friend".into()),
            seed: 7,
            dimension,
            origins: vec![(dimension, [8.5, 64.0, 8.5])],
            feet: [8.5, 64.0, 8.5],
            view_distance: 2,
            rules: Rules::default(),
            registry: 42,
        }
    }

    fn stone_chunk(registries: &Registries, x: i32, z: i32, min_y: i32, height: i32) -> Arc<Chunk> {
        let plains = registries
            .biomes
            .id("minecraft:plains")
            .unwrap_or(BiomeId(0));
        let stone = registries.blocks.parse_state("minecraft:stone").unwrap();
        let mut chunk = Chunk::new(ChunkPos::new(x, z), min_y, height, plains);
        chunk.status = ChunkStatus::Full;
        for x in 0..16 {
            for z in 0..16 {
                for y in min_y..min_y + 8 {
                    chunk.set_block_raw(x, y, z, stone, registries);
                }
            }
        }
        Arc::new(chunk)
    }

    fn blob(registries: &Registries, seq: u32, chunks: &[Arc<Chunk>]) -> Vec<u8> {
        minecraft_terrain::chunk_wire::encode_batch(seq, chunks, registries).unwrap()
    }

    fn state(world: &RemoteWorld, pos: BlockPos) -> Option<BlockStateId> {
        world.level.as_ref()?.terrain.scene.state_at(pos)
    }

    #[test]
    fn a_block_update_behind_a_missing_batch_waits_and_applies_after_it() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries.clone());
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        friend.apply(ToClient::ChunkBatch { seq: 1 });
        friend.apply(ToClient::BlockUpdates {
            changes: vec![([3, -64, 3], 0)],
        });
        friend.apply(ToClient::Chat(Clientbound::System {
            text: Text::literal("after"),
            overlay: false,
        }));
        assert_eq!(state(&friend, (3, -64, 3)), None, "no chunk yet");
        assert_eq!(friend.waiting(), 3);
        assert!(friend.chat.is_empty(), "the chat line waits too");
        friend.receive_batch(blob(
            &registries,
            1,
            &[stone_chunk(&registries, 0, 0, -64, 384)],
        ));
        assert!(friend.take_acks().is_empty(), "acknowledged once applied");
        friend.decode_now();
        assert_eq!(friend.waiting(), 0);
        assert_eq!(state(&friend, (3, -64, 3)), Some(BlockStateId::AIR));
        assert_ne!(state(&friend, (4, -64, 3)), Some(BlockStateId::AIR));
        assert_eq!(friend.chat.len(), 1);
        assert_eq!(friend.take_acks(), vec![1]);
        let shown = friend.take_shown();
        assert!(shown.level);
        assert_eq!(shown.loaded.len(), 1);
        assert_eq!(shown.blocks, vec![((3, -64, 3), BlockStateId::AIR)]);
        assert_eq!(friend.held_bytes(), 0);
    }

    #[test]
    fn the_friends_dials_know_the_world_spawn_and_its_own_last_death() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        assert_eq!(friend.dial_context([0.0; 3], 0.0), None, "no level yet");
        friend.apply(ToClient::Login(login(Dimension::Nether)));
        let context = friend.dial_context([1.5, 64.0, 2.5], 90.0).expect("a level");
        assert_eq!(context.spawn, None, "no world spawn yet: the needle spins");
        friend.apply(ToClient::SpawnPosition {
            dimension: Dimension::Overworld,
            pos: [8, 70, -8],
        });
        let context = friend.dial_context([1.5, 64.0, 2.5], 90.0).unwrap();
        assert_eq!(
            (context.dimension, context.feet, context.yaw),
            (Dimension::Nether, [1.5, 64.0, 2.5], 90.0)
        );
        assert_eq!(context.spawn, Some((Dimension::Overworld, (8, 70, -8))));
        assert_eq!(context.last_death, None);
        friend.apply(ToClient::Player(Box::new(PlayerView {
            last_death: Some((Dimension::Nether, [-3, 40, 9])),
            ..PlayerView::default()
        })));
        friend.apply(ToClient::SpawnPosition {
            dimension: Dimension::Overworld,
            pos: [100, 64, 100],
        });
        let context = friend.dial_context([1.5, 64.0, 2.5], 90.0).unwrap();
        assert_eq!(context.last_death, Some((Dimension::Nether, (-3, 40, 9))));
        assert_eq!(context.spawn, Some((Dimension::Overworld, (100, 64, 100))), "a new world spawn");
    }

    #[test]
    fn the_friend_sees_who_lies_in_a_bed_and_wakes_with_the_hosts_fade() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        let lying = Lying {
            feet: [8.5, 64.6875, 8.5],
            facing: [1, 0],
        };
        let sleeping = |client, bed, timer| ToClient::Sleeping { client, bed, timer };
        friend.apply(sleeping(5, Some(lying), 0));
        assert_eq!((friend.lying(), friend.sleepers.len()), (None, 1));
        friend.apply(sleeping(1, Some(lying), 0));
        assert_eq!(friend.lying(), Some(lying));
        for _ in 0..120 {
            friend.advance_clocks(0.05);
        }
        assert_eq!(friend.sleep_timer, crate::sleep::DEEP_SLEEP_TICKS);
        friend.apply(sleeping(1, None, crate::sleep::DEEP_SLEEP_TICKS));
        assert_eq!(friend.lying(), None);
        for _ in 0..9 {
            friend.advance_clocks(0.05);
        }
        assert_eq!(friend.sleep_timer, crate::sleep::WAKE_UP_TICKS - 1);
        friend.advance_clocks(0.05);
        assert_eq!(friend.sleep_timer, 0, "the wake-up fade is over");
        friend.apply(ToClient::Dimension {
            epoch: 1,
            dimension: Dimension::Nether,
            origin: [8.5, 64.0, 8.5],
            feet: [8.5, 64.0, 8.5],
        });
        assert!(friend.sleepers.is_empty(), "a new level has its own sleepers");
    }

    #[test]
    fn a_spawner_streamed_to_the_friend_shows_its_cage_until_the_block_or_chunk_goes() {
        use minecraftoss_core::nbt::Tag;
        let Some(registries) = registries() else {
            return;
        };
        let spawner = |(x, y, z): BlockPos, mob: &str| {
            let entity = BTreeMap::from([("id".to_owned(), Tag::String(mob.to_owned()))]);
            let data = BTreeMap::from([("entity".to_owned(), Tag::Compound(entity))]);
            Tag::Compound(BTreeMap::from([
                ("id".to_owned(), Tag::String("minecraft:mob_spawner".to_owned())),
                ("x".to_owned(), Tag::Int(x)),
                ("y".to_owned(), Tag::Int(y)),
                ("z".to_owned(), Tag::Int(z)),
                ("SpawnData".to_owned(), Tag::Compound(data)),
            ]))
        };
        let cages = |friend: &RemoteWorld| friend.level.as_ref().map_or(0, |level| level.view.spawners.len());
        let at: BlockPos = (3, -60, 4);
        let Ok(mut chunk) = Arc::try_unwrap(stone_chunk(&registries, 0, 0, -64, 384)) else {
            panic!("a fresh chunk has one owner");
        };
        let state = registries.blocks.parse_state("minecraft:spawner").unwrap();
        chunk.set_block_raw(3, at.1, 4, state, &registries);
        chunk
            .block_entities
            .entities
            .insert(at, spawner(at, "minecraft:zombie"));
        let mut friend = RemoteWorld::new(registries.clone());
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        friend.apply(ToClient::ChunkBatch { seq: 1 });
        friend.receive_batch(blob(&registries, 1, &[Arc::new(chunk)]));
        friend.decode_now();
        assert_eq!(cages(&friend), 1, "the spawner came with its chunk");
        friend.apply(ToClient::BlockUpdates {
            changes: vec![([at.0, at.1, at.2], u32::from(state.0))],
        });
        assert_eq!(cages(&friend), 1, "still a spawner");
        friend.apply(ToClient::BlockUpdates {
            changes: vec![([at.0, at.1, at.2], 0)],
        });
        assert_eq!(cages(&friend), 0, "mined away");
        let placed = (5, -60, 6);
        let tag = Nbt(minecraftoss_core::nbt::write(&spawner(placed, "minecraft:pig"), ""));
        friend.apply(ToClient::BlockEntity {
            pos: [placed.0, placed.1, placed.2],
            tag,
        });
        assert_eq!(cages(&friend), 1, "a spawner's data from the host");
        friend.apply(ToClient::ChunkForget { pos: [0, 0] });
        assert_eq!(cages(&friend), 0, "its chunk was forgotten");
    }

    #[test]
    fn a_vault_streamed_to_the_friend_shows_its_reward_item_until_the_block_or_chunk_goes() {
        use minecraft_terrain::vault::SharedData;
        use minecraftoss_core::nbt::Tag;
        let Some(registries) = registries() else {
            return;
        };
        let vault = |(x, y, z): BlockPos, item: Option<&str>| {
            let shared = SharedData {
                display_item: item.map(|id| minecraftoss_core::item::ItemStack::new(id, 1)),
                ..SharedData::default()
            };
            Tag::Compound(BTreeMap::from([
                ("id".to_owned(), Tag::String(minecraft_terrain::vault::ID.to_owned())),
                ("x".to_owned(), Tag::Int(x)),
                ("y".to_owned(), Tag::Int(y)),
                ("z".to_owned(), Tag::Int(z)),
                ("shared_data".to_owned(), shared.to_tag()),
            ]))
        };
        let shown = |friend: &RemoteWorld| {
            friend.level.as_ref().map_or_else(Vec::new, |level| {
                level.view.vaults.items(glam::DVec3::new(4.0, -59.0, 8.0), 0.0).into_iter().map(|view| view.item).collect::<Vec<_>>()
            })
        };
        let at: BlockPos = (3, -60, 4);
        let Ok(mut chunk) = Arc::try_unwrap(stone_chunk(&registries, 0, 0, -64, 384)) else {
            panic!("a fresh chunk has one owner");
        };
        let dim = registries.blocks.parse_state("minecraft:vault").unwrap();
        let lit = registries.blocks.parse_state("minecraft:vault[vault_state=active]").unwrap();
        chunk.set_block_raw(3, at.1, 4, dim, &registries);
        chunk.block_entities.entities.insert(at, vault(at, Some("minecraft:emerald")));
        let mut friend = RemoteWorld::new(registries.clone());
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        friend.apply(ToClient::ChunkBatch { seq: 1 });
        friend.receive_batch(blob(&registries, 1, &[Arc::new(chunk)]));
        friend.decode_now();
        assert_eq!(shown(&friend), ["minecraft:emerald"], "the vault came with its chunk");
        friend.apply(ToClient::BlockUpdates { changes: vec![([at.0, at.1, at.2], u32::from(lit.0))] });
        assert_eq!(shown(&friend), ["minecraft:emerald"], "it lit up and is still a vault");
        let tag = |item: Option<&str>| Nbt(minecraftoss_core::nbt::write(&vault(at, item), ""));
        friend.apply(ToClient::BlockEntity { pos: [at.0, at.1, at.2], tag: tag(Some("minecraft:diamond")) });
        assert_eq!(shown(&friend), ["minecraft:diamond"], "the host turned its item");
        friend.apply(ToClient::BlockEntity { pos: [at.0, at.1, at.2], tag: tag(None) });
        assert!(shown(&friend).is_empty(), "dim again: nothing shown");
        friend.apply(ToClient::BlockUpdates { changes: vec![([at.0, at.1, at.2], 0)] });
        assert_eq!(friend.level.as_ref().map(|level| level.view.vaults.len()), Some(0), "mined away");
        friend.apply(ToClient::BlockEntity { pos: [at.0, at.1, at.2], tag: tag(Some("minecraft:bow")) });
        assert_eq!(friend.level.as_ref().map(|level| level.view.vaults.len()), Some(1));
        friend.apply(ToClient::ChunkForget { pos: [0, 0] });
        assert_eq!(friend.level.as_ref().map(|level| level.view.vaults.len()), Some(0), "its chunk was forgotten");
    }

    #[test]
    fn a_dimension_behind_a_missing_batch_keeps_later_mob_adds_from_the_old_level() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries.clone());
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        friend.apply(ToClient::ChunkBatch { seq: 1 });
        friend.apply(ToClient::Dimension {
            epoch: 1,
            dimension: Dimension::End,
            origin: [100.5, 49.0, 0.5],
            feet: [100.5, 49.0, 0.5],
        });
        friend.apply(ToClient::MobAdd {
            id: 7,
            tag: Nbt(vec![10, 0, 0, 0]),
        });
        let level = friend.level.as_ref().unwrap();
        assert_eq!((level.dimension, level.epoch), (Dimension::Overworld, 0));
        assert!(level.mobs.is_empty());
        friend.receive_batch(blob(
            &registries,
            1,
            &[stone_chunk(&registries, 0, 0, -64, 384)],
        ));
        let jobs = friend.take_decode_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].epoch, 0);
        for job in jobs {
            let result = job.decode();
            friend.decoded(job.seq, job.epoch, result);
        }
        let level = friend.level.as_ref().unwrap();
        assert_eq!((level.dimension, level.epoch), (Dimension::End, 1));
        assert_eq!(level.mobs.keys().copied().collect::<Vec<_>>(), vec![7]);
        assert_eq!(level.origin, [100.5, 49.0, 0.5]);
        assert!(
            level.terrain.scene.generated_chunk((0, 0)).is_none(),
            "the old level's chunk went with it"
        );
        let shown = friend.take_shown();
        assert!(shown.level);
        assert!(shown.loaded.is_empty());
        assert_eq!(friend.take_acks(), vec![1]);
    }

    fn cow_at(id: u64, x: f64) -> ServerMob {
        ServerMob {
            id,
            position: glam::DVec3::new(x, 64.0, 8.5),
            y_rot: 0.0,
            x_rot: 0.0,
            y_head_rot: 0.0,
            on_ground: true,
            baby: false,
            eye_height: 1.3,
            max_head_y_rot: 75.0,
            update_interval: 3,
            sync: Default::default(),
            resting: false,
            hurts: 0,
            dead: false,
            swings: 0,
            size: (0.9, 1.4),
            vehicle: None,
            show: Default::default(),
        }
    }

    fn mobs_frame(tick: u32, mobs: &[ServerMob]) -> StateFrame {
        StateFrame {
            tick,
            epoch: 0,
            mobs: minecraft_net::messages::Blob(
                minecraft_net::encode(Lane::State, &mobs.to_vec()).unwrap(),
            ),
            entities: minecraft_net::messages::Blob(Vec::new()),
            skaters: minecraft_net::messages::Blob(Vec::new()),
        }
    }

    #[test]
    fn a_mob_is_added_moved_and_removed_and_moves_without_an_add_are_dropped() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        let tag = minecraftoss_core::snbt::parse_compound(
            "{id:\"minecraft:cow\",Pos:[8.5d,64.0d,8.5d],NoAI:1b}",
        )
        .unwrap();
        friend.apply(ToClient::MobAdd {
            id: 40,
            tag: Nbt(minecraftoss_core::nbt::write(&tag, "")),
        });
        let level = friend.level.as_ref().unwrap();
        assert_eq!(level.view.world.entity_type(40), Some("minecraft:cow"));
        let moved = [cow_at(40, 9.5), cow_at(41, 10.5)];
        assert!(friend.apply_state(mobs_frame(1, &moved)));
        let level = friend.level.as_ref().unwrap();
        assert!(level.view.client.get(40).is_some());
        assert!(
            level.view.client.get(41).is_none(),
            "a mob the friend was never told about is not drawn"
        );
        assert_eq!(level.view.world.cows()[0].cow.body.position.x, 9.5);
        let mut changed = tag.clone();
        if let minecraftoss_core::nbt::Tag::Compound(map) = &mut changed {
            map.insert(
                "id".to_owned(),
                minecraftoss_core::nbt::Tag::String("minecraft:pig".to_owned()),
            );
        }
        friend.apply(ToClient::MobLook {
            id: 40,
            look: minecraft_net::messages::Blob(minecraftoss_core::nbt::write(&changed, "")),
        });
        let level = friend.level.as_ref().unwrap();
        assert_eq!(level.view.world.entity_type(40), Some("minecraft:pig"));
        assert!(
            level.view.world.cows().is_empty(),
            "the look replaced the mob"
        );
        friend.apply(ToClient::MobRemove { ids: vec![40] });
        let level = friend.level.as_ref().unwrap();
        assert!(level.view.world.entity_type(40).is_none());
        assert!(level.mobs.is_empty());
        assert!(friend.apply_state(mobs_frame(2, &[cow_at(40, 9.5)])));
        assert!(
            friend.level.as_ref().unwrap().view.client.get(40).is_none(),
            "moves after the remove are dropped"
        );
    }

    #[test]
    fn a_batch_decoded_for_the_old_level_is_read_again_for_the_new_one() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries.clone());
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        friend.receive_batch(blob(
            &registries,
            2,
            &[stone_chunk(&registries, 6, 0, 0, 256)],
        ));
        let stale = friend.take_decode_jobs();
        assert_eq!(stale.len(), 1);
        friend.apply(ToClient::Dimension {
            epoch: 1,
            dimension: Dimension::End,
            origin: [100.5, 49.0, 0.5],
            feet: [100.5, 49.0, 0.5],
        });
        friend.apply(ToClient::ChunkBatch { seq: 2 });
        assert!(friend.take_decode_jobs().is_empty(), "one read at a time");
        for job in stale {
            let result = job.decode();
            friend.decoded(job.seq, job.epoch, result);
        }
        assert_eq!(
            friend.waiting(),
            1,
            "chunks read for the Overworld are not shown in the End"
        );
        let again = friend.take_decode_jobs();
        assert_eq!(
            again.iter().map(|job| job.epoch).collect::<Vec<_>>(),
            vec![1]
        );
        for job in again {
            let result = job.decode();
            friend.decoded(job.seq, job.epoch, result);
        }
        assert_eq!(friend.waiting(), 0);
        let stone = registries.blocks.parse_state("minecraft:stone").unwrap();
        assert_eq!(state(&friend, (100, 0, 3)), Some(stone));
    }

    #[test]
    fn a_batch_before_login_waits() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries.clone());
        friend.receive_batch(blob(
            &registries,
            1,
            &[stone_chunk(&registries, 0, 0, -64, 384)],
        ));
        assert!(friend.take_decode_jobs().is_empty());
        friend.apply(ToClient::ChunkBatch { seq: 1 });
        assert!(friend.level.is_none());
        assert_eq!(friend.waiting(), 1);
        friend.decode_now();
        assert_eq!(friend.waiting(), 1);
        let mut friend = RemoteWorld::new(registries.clone());
        friend.receive_batch(blob(
            &registries,
            1,
            &[stone_chunk(&registries, 0, 0, -64, 384)],
        ));
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        friend.apply(ToClient::ChunkBatch { seq: 1 });
        friend.decode_now();
        let stone = registries.blocks.parse_state("minecraft:stone").unwrap();
        assert_eq!(state(&friend, (5, -60, 5)), Some(stone));
        assert_eq!(friend.take_acks(), vec![1]);
    }

    #[test]
    fn menus_and_gear_reach_the_friend() {
        let Some(registries) = registries() else {
            return;
        };
        let view = |menu: u8, kind: MenuKind, cells: usize| MenuView {
            menu,
            kind,
            title: Text::literal("Chest"),
            state: 1,
            slots: vec![None; cells],
            carried: None,
            data: Vec::new(),
            extras: MenuExtras::None,
        };
        let mut friend = RemoteWorld::new(registries);
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        friend.apply(ToClient::MenuOpen(view(0, MenuKind::Inventory, 4)));
        friend.apply(ToClient::MenuOpen(view(5, MenuKind::Chest { rows: 3 }, 27)));
        friend.apply(ToClient::MenuSlots {
            menu: 5,
            state: 2,
            slots: vec![(26, Some(Nbt(vec![1])))],
            carried: Some(Some(Nbt(vec![2]))),
            data: vec![(0, 9)],
        });
        friend.apply(ToClient::MenuSlots {
            menu: 4,
            state: 9,
            slots: vec![(0, Some(Nbt(vec![3])))],
            carried: None,
            data: Vec::new(),
        });
        friend.apply(ToClient::MenuSlots {
            menu: 0,
            state: 3,
            slots: vec![(1, Some(Nbt(vec![4]))), (99, None)],
            carried: None,
            data: Vec::new(),
        });
        let chest = friend.menu.as_ref().unwrap();
        assert_eq!(chest.state, 2);
        assert_eq!(chest.slots[26], Some(Nbt(vec![1])));
        assert_eq!(chest.slots[0], None, "a change for another menu");
        assert_eq!(chest.carried, Some(Nbt(vec![2])));
        assert_eq!(chest.data, vec![(0, 9)]);
        let inventory = friend.inventory_menu.as_ref().unwrap();
        assert_eq!((inventory.state, inventory.slots.len()), (3, 4));
        assert_eq!(inventory.slots[1], Some(Nbt(vec![4])));
        friend.apply(ToClient::MenuClose { menu: 4 });
        assert!(friend.menu.is_some());
        friend.apply(ToClient::MenuClose { menu: 5 });
        assert!(friend.menu.is_none());
        assert!(friend.inventory_menu.is_some());
        let helmet = GearView {
            armor: [
                Some(minecraft_net::messages::GearItem {
                    id: "minecraft:iron_helmet".to_owned(),
                    glint: false,
                    dye: None,
                    potion: None,
                }),
                None,
                None,
                None,
            ],
            ..GearView::default()
        };
        friend.apply(ToClient::Equipment {
            client: 3,
            gear: GearView::default(),
        });
        friend.apply(ToClient::Equipment {
            client: 3,
            gear: helmet.clone(),
        });
        assert_eq!(friend.gear.get(&3), Some(&helmet));
        assert!(friend.take_effects().is_empty());
    }

    #[test]
    fn a_resent_batch_is_ignored() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries.clone());
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        let sent = blob(&registries, 1, &[stone_chunk(&registries, 0, 0, -64, 384)]);
        friend.apply(ToClient::ChunkBatch { seq: 1 });
        friend.receive_batch(sent.clone());
        friend.decode_now();
        friend.apply(ToClient::BlockUpdates {
            changes: vec![([1, -64, 1], 0)],
        });
        assert_eq!(friend.take_acks(), vec![1]);
        friend.take_shown();
        friend.receive_batch(sent);
        assert!(friend.take_decode_jobs().is_empty());
        assert_eq!(friend.take_acks(), vec![1], "acknowledged again");
        assert!(friend.take_shown().loaded.is_empty());
        assert_eq!(state(&friend, (1, -64, 1)), Some(BlockStateId::AIR));
        assert_eq!(friend.held_bytes(), 0);
    }

    #[test]
    fn a_state_frame_older_than_the_last_or_of_another_epoch_is_ignored() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        let frame = |tick, epoch| StateFrame {
            tick,
            epoch,
            ..StateFrame::default()
        };
        assert!(!friend.apply_state(frame(1, 0)), "no level yet");
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        assert!(friend.apply_state(frame(5, 0)));
        assert!(!friend.apply_state(frame(4, 0)));
        assert!(!friend.apply_state(frame(5, 0)));
        assert!(!friend.apply_state(frame(9, 1)));
        assert!(friend.apply_state(frame(6, 0)));
        friend.apply(ToClient::Dimension {
            epoch: 1,
            dimension: Dimension::Nether,
            origin: [0.5, 70.0, 0.5],
            feet: [0.5, 70.0, 0.5],
        });
        assert!(!friend.apply_state(frame(7, 0)));
        assert!(friend.apply_state(frame(2, 1)));
        assert_eq!(friend.state.as_ref().map(|state| state.tick), Some(2));
    }

    #[test]
    fn time_weather_players_and_rejections_reach_the_friend() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        friend.apply(ToClient::Rejected {
            reason: Text::literal("no"),
        });
        assert_eq!(friend.rejected, Some(Text::literal("no")));
        friend.apply(ToClient::Login(login(Dimension::End)));
        friend.apply(ToClient::PlayerList {
            added: vec![
                (0, "Steve".into(), PlayerUuid("a".into())),
                (1, "Alex".into(), PlayerUuid("b".into())),
            ],
            removed: Vec::new(),
        });
        friend.apply(ToClient::PlayerList {
            added: Vec::new(),
            removed: vec![0],
        });
        assert_eq!(friend.players.keys().copied().collect::<Vec<_>>(), vec![1]);
        friend.apply(ToClient::Time {
            game_time: 500,
            clocks: vec![
                (OVERWORLD_CLOCK.into(), 6000.0, false),
                (THE_END_CLOCK.into(), 100.0, true),
            ],
        });
        friend.apply(ToClient::Weather {
            rain: 1.0,
            thunder: 0.5,
        });
        friend.advance_clocks(1.0);
        assert_eq!(friend.clocks.ticks(Dimension::Overworld), 6020);
        assert_eq!(friend.clocks.ticks(Dimension::End), 100);
        assert_eq!(friend.clocks.ticks(Dimension::Nether), 6020);
        assert_eq!((friend.game_time, friend.weather), (500, [1.0, 0.5]));
        friend.apply(ToClient::Credits);
        assert_eq!(friend.take_effects(), vec![ToClient::Credits]);
    }

    #[test]
    fn the_boss_bar_follows_the_host_and_the_dragons_bursts_show_in_the_level() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        friend.apply(ToClient::Login(login(Dimension::End)));
        let bar = |operation| ToClient::BossBar(BossBarUpdate { id: 1, operation });
        friend.apply(bar(BossBarOperation::Progress(0.5)));
        assert!(friend.boss_bars.is_empty(), "no bar to move yet");
        friend.apply(bar(crate::session::dragon_bar(1.0)));
        friend.apply(bar(BossBarOperation::Progress(0.25)));
        assert_eq!(friend.boss_bars[&1].progress, 0.25);
        assert_eq!(friend.boss_bars[&1].color, BossBarColor::Pink);
        assert!(friend.boss_bars[&1].flags.create_world_fog);
        friend.apply(bar(BossBarOperation::Remove));
        assert!(friend.boss_bars.is_empty());
        assert!(friend.take_effects().is_empty(), "bars are not effects");
        let mut bursts = EntitySnapshot::default();
        bursts.eyes.shattered.push([0, 64, 0]);
        let entities = minecraft_net::encode(Lane::Reliable, &bursts).unwrap();
        friend.apply(ToClient::Bursts {
            entities: minecraft_net::messages::Blob(entities),
        });
        let level = friend.level.as_ref().unwrap();
        assert_eq!(level.view.item_crumbs.len(), 1, "the shattered eye's crumbs");
    }

    #[test]
    fn the_friend_s_dragon_glides_on_its_own_ticks_when_frames_come_late() {
        use minecraft_terrain::ender_dragon::{DragonView, HISTORY_SENT, Sample, part_box};
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        friend.apply(ToClient::Login(login(Dimension::End)));
        let dragon = |tick: u32| {
            let x = f64::from(tick);
            DragonView {
                id: minecraft_terrain::ender_dragon::FIRST_ID,
                uuid: [0; 4],
                position: glam::DVec3::new(x, 80.0, 0.0),
                previous_position: glam::DVec3::new(x - 1.0, 80.0, 0.0),
                y_rot: -90.0,
                flap_time: tick as f32 * 0.05,
                o_flap_time: tick as f32 * 0.05 - 0.05,
                history: vec![
                    Sample {
                        y: 80.0,
                        y_rot: -90.0
                    };
                    HISTORY_SENT
                ],
                health: 200.0,
                hurt_time: 0,
                death_time: 0,
                sitting: false,
                phase: 0,
                parts: std::array::from_fn(|part| glam::DVec3::new(x + part as f64, 80.0, 0.0)),
                tick_count: tick as i32,
                nearest_crystal: None,
            }
        };
        let frame = |tick: u32| {
            let snapshot = EntitySnapshot {
                dragons: vec![dragon(tick)],
                ..EntitySnapshot::default()
            };
            StateFrame {
                tick,
                epoch: 0,
                mobs: minecraft_net::messages::Blob(Vec::new()),
                entities: minecraft_net::messages::Blob(
                    minecraft_net::encode(Lane::State, &snapshot).unwrap(),
                ),
                skaters: minecraft_net::messages::Blob(Vec::new()),
            }
        };
        let timing = crate::tick_rate::ClientTiming::default();
        let mut tick = 0;
        let mut drawn = Vec::new();
        for frames in [1, 0, 2, 1, 1, 0, 2, 1, 1, 1].repeat(3) {
            for _ in 0..frames {
                tick += 1;
                assert!(friend.apply_state(frame(tick)));
            }
            for _ in 0..4 {
                let level = friend.level.as_mut().unwrap();
                level
                    .view
                    .advance(timing.tick_seconds / 4.0, timing, &[], glam::DVec3::ZERO);
                let view = level.view.drawn_dragons().remove(0);
                drawn.push(
                    view.previous_position
                        .lerp(view.position, f64::from(level.view.partial()))
                        .x,
                );
            }
        }
        for pair in drawn.windows(2) {
            assert!(pair[1] >= pair[0], "the drawn dragon went back: {pair:?}");
        }
        assert!(drawn.last().unwrap() > &20.0, "it flew on");
        let newest = dragon(tick);
        let level = friend.level.as_ref().unwrap();
        let parts: Vec<_> = level
            .view
            .shot_boxes(None)
            .into_iter()
            .filter(|(key, _)| crate::entities::dragon_part_key(newest.id, 0) >> 3 == key >> 3)
            .collect();
        assert_eq!(parts.len(), 8);
        for (part, &feet) in newest.parts.iter().enumerate() {
            let (low, high) = part_box(feet, part);
            let key = crate::entities::dragon_part_key(newest.id, part);
            assert!(
                parts.contains(&(key, [low.x, low.y, low.z, high.x, high.y, high.z])),
                "the host's newest box for part {part}"
            );
        }
    }

    #[test]
    fn chat_lines_show_and_the_command_level_is_kept() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        let line = Clientbound::Player {
            sender: minecraft_commands::PlayerId(0),
            name: "Steve".into(),
            message: "hi".into(),
        };
        friend.apply(ToClient::Chat(line.clone()));
        friend.apply(ToClient::Chat(Clientbound::Commands { level: 4 }));
        assert_eq!(friend.take_chat(), vec![line]);
        assert_eq!(friend.command_level, 4);
        friend.apply(ToClient::Chat(Clientbound::Commands { level: 0 }));
        assert_eq!(friend.command_level, 0);
        assert!(friend.take_chat().is_empty());
    }

    #[test]
    fn a_stream_held_past_its_limit_is_lost() {
        let Some(registries) = registries() else {
            return;
        };
        let mut friend = RemoteWorld::new(registries);
        friend.apply(ToClient::Login(login(Dimension::Overworld)));
        friend.apply(ToClient::ChunkBatch { seq: 1 });
        let line = Clientbound::System {
            text: Text::literal("x".repeat(1024 * 1024)),
            overlay: false,
        };
        for _ in 0..70 {
            friend.apply(ToClient::Chat(line.clone()));
        }
        assert_eq!(friend.failed.as_deref(), Some(LOST_STREAM));
        assert!(friend.held_bytes() > QUEUE_LIMIT);
    }

    #[test]
    fn a_friend_sees_the_hosts_blocks_in_order_and_their_leave_is_saved() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("remote-loopback");
        let mut server = Server {
            globals: test_globals(registries.clone(), Some(save.clone())),
            levels: BTreeMap::new(),
            opening: BTreeMap::new(),
            players: BTreeMap::new(),
            published: false,
        };
        let terrain = Terrain::open(
            registries.clone(),
            server.globals.seed,
            2,
            Dimension::End,
            Some(&save),
            None,
        )
        .unwrap();
        let (x, y, z) = terrain.stream.player_spawn;
        let origin = [x, y, z];
        server
            .levels
            .insert(Dimension::End, Level::new(Dimension::End, terrain));
        let origins = [(Dimension::End, origin)];
        server.globals.chat.join(minecraft_commands::PlayerId(1), "Alex");
        let profile = server.globals.chat.profile(minecraft_commands::PlayerId(1)).unwrap().clone();
        let in_the_end = crate::player_nbt::PlayerData {
            place: Some(crate::player_nbt::Place::standing(
                Dimension::End,
                origin,
                0.0,
                0.0,
            )),
            ..crate::player_nbt::PlayerData::default()
        };
        save.write_player(&profile.uuid, &in_the_end).unwrap();
        let context = JoinContext {
            registry: 42,
            origins: &origins,
            view_distance: 2,
            kit: None,
        };
        let hello = Hello {
            protocol: PROTOCOL,
            view_distance: 2,
            registry: 42,
        };
        let mut sessions = Sessions::default();
        let answer = sessions.hello(&mut server, 1, "Alex", hello, &context, Instant::now());
        let Answer::Joined(uuid) = answer else {
            panic!("{answer:?}");
        };
        let feet = server.players[&uuid].arrival.unwrap().feet;
        let block = (
            feet[0].floor() as i32,
            feet[1].floor() as i32,
            feet[2].floor() as i32,
        );
        let center = (block.0 >> 4, block.2 >> 4);
        let around: Vec<(i32, i32)> = (-1..=1)
            .flat_map(|dx| (-1..=1).map(move |dz| (center.0 + dx, center.1 + dz)))
            .collect();
        let mut friend = RemoteWorld::new(registries.clone());
        let exchange = |server: &mut Server, sessions: &mut Sessions, friend: &mut RemoteWorld| {
            let level = server.levels.get_mut(&Dimension::End).unwrap();
            crate::level::stream_tick(
                &mut level.terrain,
                &mut level.entities,
                &mut level.containers,
                &[block],
                Some(block),
            );
            sessions.fan_out(&mut server.levels);
            let mut blobs = Vec::new();
            for job in sessions.select_batches(server) {
                let batch = encode_batch(job.seq, &job.chunks, &registries);
                sessions.batch_encoded(job.client, &batch.rest);
                blobs.push(batch.blob);
            }
            for (client, message) in sessions.take_outgoing() {
                assert_eq!(client, 1);
                let bytes = minecraft_net::encode(Lane::Reliable, &message).unwrap();
                friend.apply(minecraft_net::decode(Lane::Reliable, &bytes).unwrap());
            }
            for blob in blobs {
                friend.receive_batch(blob);
            }
            friend.decode_now();
            for seq in friend.take_acks() {
                sessions.batch_received(1, seq);
            }
        };
        let started = Instant::now();
        while around.iter().any(|&chunk| {
            friend
                .level
                .as_ref()
                .is_none_or(|level| level.terrain.scene.generated_chunk(chunk).is_none())
        }) {
            assert!(
                started.elapsed() < Duration::from_secs(600),
                "the friend never got the chunks around the platform"
            );
            exchange(&mut server, &mut sessions, &mut friend);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(friend.failed.is_none());
        assert_eq!(
            friend.login.as_ref().map(|login| login.uuid.clone()),
            Some(uuid.clone())
        );
        assert_eq!(
            friend.players.get(&1).map(|(name, _)| name.as_str()),
            Some("Alex")
        );
        let host = &server.levels[&Dimension::End].terrain.scene;
        let seen = &friend.level.as_ref().unwrap().terrain.scene;
        let mut solid = 0;
        for &(cx, cz) in &around {
            for x in cx * 16..cx * 16 + 16 {
                for z in cz * 16..cz * 16 + 16 {
                    for y in 0..256 {
                        let host_state = host.state_at((x, y, z));
                        assert_eq!(seen.state_at((x, y, z)), host_state, "{:?}", (x, y, z));
                        solid +=
                            usize::from(host_state.is_some_and(|state| state != BlockStateId::AIR));
                    }
                }
            }
        }
        assert!(solid > 0, "the platform and the island are there");
        let broken = (0..256)
            .rev()
            .map(|y| (block.0, y, block.2))
            .find(|&pos| {
                host.state_at(pos)
                    .is_some_and(|state| state != BlockStateId::AIR)
            })
            .expect("a block under the feet");
        let placed = (broken.0 + 1, broken.1 + 1, broken.2);
        friend.take_shown();
        let level = server.levels.get_mut(&Dimension::End).unwrap();
        level.terrain.scene.set(
            placed,
            Some(minecraft_terrain::scene::Block::new("minecraft:stone")),
        );
        exchange(&mut server, &mut sessions, &mut friend);
        let level = server.levels.get_mut(&Dimension::End).unwrap();
        level.terrain.scene.set(broken, None);
        level.terrain.scene.set(placed, None);
        exchange(&mut server, &mut sessions, &mut friend);
        let stone = registries.blocks.parse_state("minecraft:stone").unwrap();
        let shown: Vec<_> = friend
            .take_shown()
            .blocks
            .into_iter()
            .filter(|(pos, _)| *pos == placed || *pos == broken)
            .collect();
        assert_eq!(
            shown,
            vec![
                (placed, stone),
                (broken, BlockStateId::AIR),
                (placed, BlockStateId::AIR),
            ]
        );
        assert_eq!(state(&friend, broken), Some(BlockStateId::AIR));
        assert_eq!(state(&friend, placed), Some(BlockStateId::AIR));
        let level = server.levels.get_mut(&Dimension::End).unwrap();
        let mut entities = crate::entities::Entities::new(
            &level.terrain.stream,
            Dimension::End,
            server.globals.seed,
            &crate::data::GameData::default(),
            None,
            None,
        );
        for &(x, z) in &around {
            if let Some(chunk) = level.terrain.stream.loaded_chunk(ChunkPos::new(x, z)) {
                entities.load_chunk(&chunk);
            }
        }
        let still = minecraftoss_core::snbt::parse_compound("{NoAI:1b}").unwrap();
        let cow_feet = [feet[0] + 1.0, feet[1], feet[2]];
        entities.summon(
            minecraft_commands::PlayerId(1),
            "minecraft:cow".to_owned(),
            cow_feet,
            Some(still),
        );
        level.entities = Some(entities);
        let watcher = minecraft_terrain::mob_tracking::Watcher {
            id: 1,
            feet,
            view_distance: 2,
        };
        let started = Instant::now();
        let mut frames = 0;
        let cow = loop {
            assert!(
                started.elapsed() < Duration::from_secs(300),
                "the friend never saw the cow"
            );
            let player = server.players.get_mut(&uuid).unwrap();
            let entities = server
                .levels
                .get_mut(&Dimension::End)
                .unwrap()
                .entities
                .as_mut()
                .unwrap();
            entities.set_watchers(vec![watcher], 2);
            let batch = crate::tick_rate::TickBatch {
                normal: if entities.ready_for_ticks() {
                    vec![true]
                } else {
                    Vec::new()
                },
                overload: None,
            };
            let view = crate::entities::PlayerView {
                feet,
                alive: true,
                attackable: true,
                spectator: false,
                picks_up: false,
                health: 20.0,
                yaw: 0.0,
                pitch: 0.0,
                main_hand: None,
                offhand: None,
                sneaking: false,
                hero: None,
                sleeping: None,
                sleep_timer: 0,
                forward: 0.0,
                strafe: 0.0,
                jumping: false,
                swirl: None,
                omens: Default::default(),
            };
            let mut actors = [crate::entities::TickActor {
                id: 1,
                view,
                inventory: &mut player.inventory,
                selected: 0,
            }];
            entities.tick(
                &mut actors,
                0.05,
                &batch,
                crate::tick_rate::ClientTiming::default(),
                minecraft_terrain::server::ClockTicks::default(),
                true,
            );
            for (client, frame) in sessions.send_mobs(&server.players, &mut server.levels, 2) {
                assert_eq!(client, 1);
                let bytes = minecraft_net::encode(Lane::State, &frame).unwrap();
                let frame = minecraft_net::decode(Lane::State, &bytes).unwrap();
                frames += usize::from(friend.apply_state(frame));
            }
            exchange(&mut server, &mut sessions, &mut friend);
            let level = friend.level.as_ref().unwrap();
            if let Some(cow) = level.view.world.cows().first().map(|cow| cow.id)
                && level.view.client.get(cow).is_some()
            {
                break cow;
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(frames > 0);
        let host = server.levels[&Dimension::End].entities.as_ref().unwrap();
        let host_cow = host
            .view
            .world
            .cows()
            .iter()
            .find(|each| each.id == cow)
            .expect("the friend's cow has the host's id");
        let seen = &friend.level.as_ref().unwrap().view.world.cows()[0];
        assert!(
            host_cow.cow.body.position.distance(seen.cow.body.position) < 0.5,
            "the friend's cow stands where the host's does"
        );
        let left = sessions
            .leave(&mut server, 1)
            .expect("Alex was in the world");
        assert_eq!(left.player.name, "Alex");
        server.globals.writer.finish();
        let recipes = server.globals.data.recipes.clone();
        assert!(
            save.read_player(&uuid.0, &recipes).unwrap().is_some(),
            "the leave wrote the player file"
        );
    }
}
