use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use minecraft_commands::{Clientbound, Text};
use minecraft_net::messages::{GearView, Login, MenuView, Nbt, PlayerView, Rules};
use minecraft_net::{ChunkBatchBlob, Dimension, Lane, PlayerUuid, StateFrame, ToClient};
use minecraft_terrain::ChunkEvent;
use minecraft_terrain::server::{OVERWORLD_CLOCK, THE_END_CLOCK};
use minecraft_terrain::terrain::BlockStates;
use minecraftoss_core::{BlockStateId, Chunk, ChunkPos, Registries};

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
    pub effects: Vec<ToClient>,
    pub state: Option<StateFrame>,
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
            effects: Vec::new(),
            state: None,
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
                    level.block_entities.insert((pos[0], pos[1], pos[2]), tag);
                }
            }
            ToClient::MobAdd { id, tag } => {
                if let Some(level) = self.level.as_mut() {
                    level.mobs.insert(id, tag);
                }
            }
            ToClient::MobRemove { ids } => {
                if let Some(level) = self.level.as_mut() {
                    for id in ids {
                        level.mobs.remove(&id);
                    }
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
            ToClient::Dimension {
                epoch,
                dimension,
                origin,
                feet,
            } => self.open_level(dimension, epoch, origin, feet),
            other => self.effects.push(other),
        }
        None
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
            level.terrain.scene.set(at, states.block(state).cloned());
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
            pending_batch: None,
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
        let context = JoinContext {
            registry: 42,
            dimension: Dimension::End,
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
