use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use minecraft_commands::text::YELLOW;
use minecraft_commands::{Clientbound, CommandServer, Outgoing, PlayerId, Text};
use minecraft_net::codec::{BATCH_CUT, BULK_LIMIT};
use minecraft_net::messages::{
    ChunkBlob, HandEvent, HandEventKind, HandState, HelloRefusal, Login, Nbt, PlayerView, Rules,
    Target, check_hello,
};
use minecraft_net::{ChunkBatchBlob, Dimension, PROTOCOL, PlayerUuid, ToClient};
use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::server::{OVERWORLD_CLOCK, THE_END_CLOCK};
use minecraft_terrain::view::{MIN_VIEW_DISTANCE, TrackingView};
use minecraftoss_core::{BlockStateId, Chunk, ChunkPos, Registries};

use crate::actions::Hands;
use crate::level::{Level, Terrain};
use crate::player::Player;
use crate::server::{Globals, Left, Server};

pub type ChunkXz = (i32, i32);
type BlockPos = (i32, i32, i32);

pub const BATCH_CHUNKS: usize = 9;
pub const RETIRE_AFTER: Duration = Duration::from_secs(1);
pub const NAME_TAKEN: &str = "A player with that name is already in the world";
pub const TRAVEL_OFF: &str = "Dimension travel is off while friends are online (not built yet)";
pub const TIME_EVERY: Duration = Duration::from_secs(1);
pub const HELD_FOR: Duration = Duration::from_millis(500);
pub const HAND_EVENTS: usize = 256;
pub const BULLETS: u32 = u32::MAX;
pub const CRACK_RANGE: f64 = 32.0;
pub const BREAK_EVENT: i32 = 2001;

#[derive(Debug, Default)]
pub struct ChunkSender {
    pub level: Option<Dimension>,
    view: Option<TrackingView>,
    view_distance: i32,
    sent: HashSet<ChunkXz>,
    pending: HashSet<ChunkXz>,
    in_flight: BTreeSet<u32>,
    acknowledged: u32,
    next_seq: u32,
}

impl ChunkSender {
    pub fn new(view_distance: i32) -> Self {
        Self {
            view_distance,
            next_seq: 1,
            ..Self::default()
        }
    }

    pub fn follow(&mut self, level: Dimension, center: ChunkXz, out: &mut Vec<ToClient>) {
        if self.level != Some(level) {
            self.level = Some(level);
            self.view = None;
            self.sent.clear();
            self.pending.clear();
            self.in_flight.clear();
        }
        let next = TrackingView::new(ChunkPos::new(center.0, center.1), self.view_distance);
        let (mut entered, mut left) = (Vec::new(), Vec::new());
        TrackingView::difference(
            self.view,
            Some(next),
            |pos| entered.push((pos.x, pos.z)),
            |pos| left.push((pos.x, pos.z)),
        );
        self.view = Some(next);
        for pos in left {
            if !self.pending.remove(&pos) && self.sent.remove(&pos) {
                out.push(ToClient::ChunkForget {
                    pos: [pos.0, pos.1],
                });
            }
        }
        for pos in entered {
            if !self.sent.contains(&pos) {
                self.pending.insert(pos);
            }
        }
    }

    pub fn window(&self) -> usize {
        if self.acknowledged >= 2 { 2 } else { 1 }
    }

    pub fn next_batch(
        &mut self,
        loaded: impl Fn(ChunkXz) -> bool,
        out: &mut Vec<ToClient>,
    ) -> Option<(u32, Vec<ChunkXz>)> {
        if self.in_flight.len() >= self.window() {
            return None;
        }
        let center = self.view?.center;
        let mut ready: Vec<ChunkXz> = self
            .pending
            .iter()
            .copied()
            .filter(|&pos| loaded(pos))
            .collect();
        if ready.is_empty() {
            return None;
        }
        ready.sort_by_key(|&(x, z)| {
            let (dx, dz) = (i64::from(x - center.x), i64::from(z - center.z));
            (dx * dx + dz * dz, x, z)
        });
        ready.truncate(BATCH_CHUNKS);
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1).max(1);
        for pos in &ready {
            self.pending.remove(pos);
            self.sent.insert(*pos);
        }
        self.in_flight.insert(seq);
        out.push(ToClient::ChunkBatch { seq });
        Some((seq, ready))
    }

    pub fn received(&mut self, seq: u32) -> bool {
        let known = self.in_flight.remove(&seq);
        if known {
            self.acknowledged = self.acknowledged.saturating_add(1);
        }
        known
    }

    pub fn cut(&mut self, rest: &[ChunkXz]) {
        for &pos in rest {
            let in_view = self
                .view
                .is_some_and(|view| view.contains(ChunkPos::new(pos.0, pos.1)));
            if self.sent.remove(&pos) && in_view {
                self.pending.insert(pos);
            }
        }
    }

    pub fn has(&self, pos: ChunkXz) -> bool {
        self.sent.contains(&pos)
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }
}

pub struct Session {
    pub client: u32,
    pub uuid: PlayerUuid,
    pub name: String,
    pub chunks: ChunkSender,
    pub outbox: Vec<ToClient>,
    pub hands: HandQueue,
    pub menu_inputs: Vec<crate::menus::MenuInput>,
    command_level: Option<u8>,
    view: Option<PlayerView>,
    cracks: BTreeMap<(u32, BlockPos), u8>,
}

impl Session {
    pub fn new(client: u32, uuid: PlayerUuid, name: &str, view_distance: i32) -> Self {
        Self {
            client,
            uuid,
            name: name.to_owned(),
            chunks: ChunkSender::new(view_distance),
            outbox: Vec::new(),
            hands: HandQueue::default(),
            menu_inputs: Vec::new(),
            command_level: None,
            view: None,
            cracks: BTreeMap::new(),
        }
    }

    pub fn send_cracks(&mut self, cracks: BTreeMap<(u32, BlockPos), u8>) {
        let cracks: BTreeMap<_, _> = cracks
            .into_iter()
            .filter(|((_, pos), _)| self.chunks.has((pos.0 >> 4, pos.2 >> 4)))
            .collect();
        for (&(breaker, pos), &stage) in &cracks {
            if self.cracks.get(&(breaker, pos)) != Some(&stage) {
                self.outbox.push(ToClient::Crack {
                    breaker,
                    pos: [pos.0, pos.1, pos.2],
                    stage: i8::try_from(stage).unwrap_or(9),
                });
            }
        }
        for &(breaker, pos) in self.cracks.keys() {
            if !cracks.contains_key(&(breaker, pos)) {
                self.outbox.push(ToClient::Crack {
                    breaker,
                    pos: [pos.0, pos.1, pos.2],
                    stage: -1,
                });
            }
        }
        self.cracks = cracks;
    }

    pub fn send_breaks(&mut self, breaks: &[([i32; 3], u32)]) {
        for &(pos, state) in breaks {
            if self.chunks.has((pos[0] >> 4, pos[2] >> 4)) {
                self.outbox.push(ToClient::LevelEvent {
                    event: BREAK_EVENT,
                    pos,
                    data: i32::try_from(state).unwrap_or_default(),
                });
            }
        }
    }

    pub fn push_menu(&mut self, input: crate::menus::MenuInput) {
        if self.menu_inputs.len() < crate::menus::MENU_INPUTS {
            self.menu_inputs.push(input);
        }
    }

    pub fn send_view(&mut self, player: &Player) {
        let view = crate::views::player_view(player);
        let key = crate::views::sent_key(&view);
        if self.view.as_ref() == Some(&key) {
            return;
        }
        self.view = Some(key);
        self.outbox.push(ToClient::Player(Box::new(view)));
    }

    pub fn send_updates(&mut self, updates: &Updates) {
        let sent = |pos: &[i32; 3]| self.chunks.has((pos[0] >> 4, pos[2] >> 4));
        let changes: Vec<([i32; 3], u32)> = updates
            .states
            .iter()
            .filter(|(pos, _)| sent(pos))
            .copied()
            .collect();
        let entities: Vec<([i32; 3], Nbt)> = updates
            .entities
            .iter()
            .filter(|(pos, _)| sent(pos))
            .cloned()
            .collect();
        if !changes.is_empty() {
            self.outbox.push(ToClient::BlockUpdates { changes });
        }
        for (pos, tag) in entities {
            self.outbox.push(ToClient::BlockEntity { pos, tag });
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Held {
    down: bool,
    heard: Option<Instant>,
}

impl Held {
    fn press(&mut self, now: Instant) {
        self.down = true;
        self.heard = Some(now);
    }

    fn release(&mut self) {
        self.down = false;
        self.heard = None;
    }

    fn keep(&mut self, now: Instant) {
        if self.down {
            self.heard = Some(now);
        }
    }

    fn held(&self, now: Instant) -> bool {
        self.down
            && self
                .heard
                .is_some_and(|at| now.saturating_duration_since(at) < HELD_FOR)
    }
}

#[derive(Debug, Default)]
pub struct HandQueue {
    events: VecDeque<HandEvent>,
    last_seq: Option<u32>,
    state_tick: Option<u32>,
    attack: Held,
    using: Held,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HandTake {
    pub hands: Hands,
    pub swap: bool,
    pub drops: Vec<bool>,
}

impl HandQueue {
    pub fn push(&mut self, events: Vec<HandEvent>) {
        for event in events {
            if self.last_seq.is_some_and(|last| event.seq <= last) {
                continue;
            }
            self.last_seq = Some(event.seq);
            self.events.push_back(event);
            if self.events.len() > HAND_EVENTS {
                self.events.pop_front();
            }
        }
    }

    pub fn state(&mut self, state: HandState, now: Instant) {
        if self.state_tick.is_some_and(|tick| state.tick <= tick) {
            return;
        }
        self.state_tick = Some(state.tick);
        if state.attack {
            self.attack.keep(now);
        }
        if state.use_item {
            self.using.keep(now);
        }
    }

    pub fn take(&mut self, now: Instant, allowed: impl Fn(&Target) -> bool) -> HandTake {
        let mut taken = HandTake::default();
        for event in self.events.drain(..) {
            match event.kind {
                HandEventKind::AttackPressed if allowed(&event.target) => {
                    taken.hands.attack_pressed = true;
                    self.attack.press(now);
                }
                HandEventKind::UsePressed if allowed(&event.target) => {
                    taken.hands.use_pressed = true;
                    self.using.press(now);
                }
                HandEventKind::AttackPressed | HandEventKind::UsePressed => {}
                HandEventKind::AttackReleased => self.attack.release(),
                HandEventKind::UseReleased => self.using.release(),
                HandEventKind::SwapHands => taken.swap = true,
                HandEventKind::Drop { whole_stack } => taken.drops.push(whole_stack),
            }
        }
        taken.hands.attack_held = self.attack.held(now);
        taken.hands.use_held = self.using.held(now);
        taken
    }
}

pub fn level_cracks(
    dimension: Dimension,
    level: &Level,
    players: &BTreeMap<PlayerUuid, Player>,
) -> BTreeMap<(u32, BlockPos), u8> {
    let mut cracks: BTreeMap<_, _> = crate::level::shot_cracks(&level.shots)
        .into_iter()
        .map(|(pos, stage)| ((BULLETS, pos), stage))
        .collect();
    for player in players
        .values()
        .filter(|player| player.dimension == dimension)
    {
        if let Some((pos, stage)) = player.hand.dig.crack() {
            cracks.insert((player.id.0, pos), stage);
        }
    }
    cracks
}

fn near(feet: [f64; 3], pos: BlockPos, range: f64) -> bool {
    let centre = [
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    ];
    (0..3).map(|k| (centre[k] - feet[k]).powi(2)).sum::<f64>() < range * range
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Updates {
    pub states: Vec<([i32; 3], u32)>,
    pub entities: Vec<([i32; 3], Nbt)>,
}

pub fn block_updates(terrain: &mut Terrain, changed: &[BlockPos]) -> Updates {
    let changed: BTreeSet<BlockPos> = changed.iter().copied().collect();
    let states = changed
        .iter()
        .filter_map(|&(x, y, z)| {
            let state = terrain.scene.state_at((x, y, z))?;
            Some(([x, y, z], u32::from(state.0)))
        })
        .collect();
    let mut by_chunk: BTreeMap<ChunkXz, Vec<BlockPos>> = BTreeMap::new();
    for &pos in &changed {
        by_chunk
            .entry((pos.0 >> 4, pos.2 >> 4))
            .or_default()
            .push(pos);
    }
    let mut entities = Vec::new();
    for ((x, z), positions) in by_chunk {
        let Some(chunk) = terrain.stream.loaded_chunk(ChunkPos::new(x, z)) else {
            continue;
        };
        for pos in positions {
            if let Some(tag) = chunk.block_entities.entities.get(&pos) {
                let nbt = Nbt(minecraftoss_core::nbt::write(tag, ""));
                entities.push(([pos.0, pos.1, pos.2], nbt));
            }
        }
    }
    Updates { states, entities }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hello {
    pub protocol: u16,
    pub view_distance: u8,
    pub registry: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Hello(HelloRefusal),
    NameTaken,
}

impl Refusal {
    pub fn reason(self) -> Text {
        match self {
            Self::Hello(refusal) => refusal.reason(),
            Self::NameTaken => Text::literal(NAME_TAKEN),
        }
    }

    pub fn line(self) -> String {
        match self {
            Self::Hello(HelloRefusal::Protocol { .. }) => format!(
                "Incompatible build: use the host's build (Minecraft multiplayer protocol {PROTOCOL})"
            ),
            Self::Hello(HelloRefusal::Registry) => self.reason().resolve(&|_| None).text,
            Self::NameTaken => NAME_TAKEN.to_owned(),
        }
    }
}

pub fn check_join(
    hello: &Hello,
    host_registry: u64,
    name: &str,
    players: &BTreeMap<PlayerUuid, Player>,
) -> Result<(), Refusal> {
    check_hello(hello.protocol, hello.registry, host_registry).map_err(Refusal::Hello)?;
    if players
        .values()
        .any(|player| player.name.eq_ignore_ascii_case(name))
    {
        return Err(Refusal::NameTaken);
    }
    Ok(())
}

pub struct JoinContext<'a> {
    pub registry: u64,
    pub dimension: Dimension,
    pub origins: &'a [(Dimension, [f64; 3])],
    pub view_distance: i32,
    pub kit: Option<&'a str>,
}

#[derive(Debug, PartialEq)]
pub enum Answer {
    Joined(PlayerUuid),
    Refused(Refusal),
    Ignored,
}

pub struct BatchJob {
    pub client: u32,
    pub seq: u32,
    pub chunks: Vec<Arc<Chunk>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EncodedBatch {
    pub blob: Vec<u8>,
    pub rest: Vec<ChunkXz>,
}

pub fn encode_batch(seq: u32, chunks: &[Arc<Chunk>], registries: &Registries) -> EncodedBatch {
    let items: Vec<([i32; 2], &Arc<Chunk>)> = chunks
        .iter()
        .map(|chunk| ([chunk.pos.x, chunk.pos.z], chunk))
        .collect();
    cut_batch(seq, &items, |chunk| {
        minecraft_terrain::chunk_wire::encode_chunk(chunk, registries)
    })
}

pub fn cut_batch<T>(
    seq: u32,
    items: &[([i32; 2], T)],
    encode: impl Fn(&T) -> ChunkBlob,
) -> EncodedBatch {
    let mut batch = ChunkBatchBlob {
        seq,
        chunks: Vec::new(),
    };
    let mut rest = Vec::new();
    for (pos, item) in items {
        if !rest.is_empty() {
            rest.push((pos[0], pos[1]));
            continue;
        }
        batch.chunks.push((*pos, encode(item)));
        if batch.chunks.len() > 1 && batch.encoded_len() > BATCH_CUT {
            batch.chunks.pop();
            rest.push((pos[0], pos[1]));
        } else if batch.encoded_len() > BULK_LIMIT {
            batch.chunks.pop();
            diag::warn!(World, "Minecraft chunk {pos:?} is too large to send");
        }
    }
    let blob = batch
        .encode()
        .or_else(|_| {
            ChunkBatchBlob {
                seq,
                chunks: Vec::new(),
            }
            .encode()
        })
        .unwrap_or_else(|_| Vec::new());
    EncodedBatch { blob, rest }
}

pub fn registry_fingerprint(registries: &Registries) -> u64 {
    let blocks = &registries.blocks;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut write = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    write(&(blocks.state_count() as u64).to_be_bytes());
    for id in (0..blocks.state_count()).filter_map(|id| u16::try_from(id).ok()) {
        write(blocks.state_to_string(BlockStateId(id)).as_bytes());
        write(&[0]);
    }
    hash
}

pub fn rules_of(rules: &GameRules) -> Rules {
    Rules {
        pvp: rules.bool("pvp"),
        show_death_messages: rules.bool("show_death_messages"),
        keep_inventory: rules.keep_inventory(),
        natural_health_regeneration: rules.bool("natural_health_regeneration"),
        reduced_debug_info: rules.bool("reduced_debug_info"),
        do_immediate_respawn: rules.bool("immediate_respawn"),
    }
}

pub fn time_of(globals: &Globals) -> ToClient {
    let advancing = globals.game_rules.bool("advance_time");
    ToClient::Time {
        game_time: globals.day.ticks.floor() as i64,
        clocks: vec![
            (
                OVERWORLD_CLOCK.to_owned(),
                globals.day.ticks,
                !advancing || globals.day.paused,
            ),
            (
                THE_END_CLOCK.to_owned(),
                globals.end_clock.ticks,
                !advancing || globals.end_clock.paused,
            ),
        ],
    }
}

fn system_line(text: Text) -> Clientbound {
    Clientbound::System {
        text,
        overlay: false,
    }
}

pub fn joined_line(name: &str) -> Clientbound {
    system_line(
        Text::translate("multiplayer.player.joined", vec![Text::literal(name)]).color(YELLOW),
    )
}

pub fn left_line(name: &str) -> Clientbound {
    system_line(Text::translate("multiplayer.player.left", vec![Text::literal(name)]).color(YELLOW))
}

pub fn travel_off_line() -> Clientbound {
    system_line(Text::literal(TRAVEL_OFF))
}

fn listed(player: &Player) -> (u32, String, PlayerUuid) {
    (player.id.0, player.name.clone(), player.uuid.clone())
}

#[derive(Default)]
pub struct Sessions {
    pub sessions: BTreeMap<u32, Session>,
    pub host_lines: Vec<Clientbound>,
    refused: Vec<(u32, ToClient)>,
    retiring: Vec<(u32, String, Instant)>,
}

impl Sessions {
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn hello(
        &mut self,
        server: &mut Server,
        client: u32,
        name: &str,
        hello: Hello,
        context: &JoinContext,
        now: Instant,
    ) -> Answer {
        let retiring = self.retiring.iter().any(|(known, ..)| *known == client);
        if self.sessions.contains_key(&client) || retiring {
            return Answer::Ignored;
        }
        if let Err(refusal) = check_join(&hello, context.registry, name, &server.players) {
            let reason = refusal.reason();
            self.refused.push((client, ToClient::Rejected { reason }));
            self.retiring.push((client, refusal.line(), now));
            diag::info!(
                World,
                "Minecraft refused {name} (client {client}): {}",
                refusal.line()
            );
            return Answer::Refused(refusal);
        }
        let Some(uuid) = server.join_friend(PlayerId(client), name, context.kit, context.dimension)
        else {
            return Answer::Ignored;
        };
        let Some(player) = server.players.get(&uuid) else {
            return Answer::Ignored;
        };
        let view_distance = i32::from(hello.view_distance).clamp(
            MIN_VIEW_DISTANCE,
            context.view_distance.max(MIN_VIEW_DISTANCE),
        );
        let feet = player
            .arrival
            .or(player.place)
            .map_or([0.0; 3], |place| place.feet);
        let login = Login {
            client,
            uuid: uuid.clone(),
            seed: server.globals.seed,
            dimension: player.dimension,
            origins: context.origins.to_vec(),
            feet,
            view_distance: u8::try_from(view_distance).unwrap_or(u8::MAX),
            rules: rules_of(&server.globals.game_rules),
            registry: context.registry,
        };
        let newcomer = listed(player);
        let mut session = Session::new(client, uuid.clone(), name, view_distance);
        session.outbox.push(ToClient::Login(login));
        session.outbox.push(ToClient::PlayerList {
            added: server.players.values().map(listed).collect(),
            removed: Vec::new(),
        });
        for other in self.sessions.values_mut() {
            other.outbox.push(ToClient::PlayerList {
                added: vec![newcomer.clone()],
                removed: Vec::new(),
            });
        }
        self.sessions.insert(client, session);
        self.broadcast(joined_line(name));
        diag::info!(
            World,
            "Minecraft {name} (client {client}) joined in {} at {feet:?}",
            player.dimension.dimension_type()
        );
        Answer::Joined(uuid)
    }

    pub fn leave(&mut self, server: &mut Server, client: u32) -> Option<Left> {
        self.refused.retain(|(known, _)| *known != client);
        self.retiring.retain(|(known, ..)| *known != client);
        let session = self.sessions.remove(&client)?;
        let left = server.leave(&session.uuid);
        for other in self.sessions.values_mut() {
            other.outbox.push(ToClient::PlayerList {
                added: Vec::new(),
                removed: vec![client],
            });
        }
        self.broadcast(left_line(&session.name));
        diag::info!(World, "Minecraft {} (client {client}) left", session.name);
        left
    }

    fn broadcast(&mut self, line: Clientbound) {
        for session in self.sessions.values_mut() {
            session.outbox.push(ToClient::Chat(line.clone()));
        }
        self.host_lines.push(line);
    }

    pub fn deliver(&mut self, chat: &CommandServer, out: &[Outgoing], to_host: bool) {
        if out.is_empty() {
            return;
        }
        for session in self.sessions.values_mut() {
            let lines = crate::commands::reaching(chat, out, PlayerId(session.client));
            session.outbox.extend(lines.into_iter().map(ToClient::Chat));
        }
        if to_host {
            let lines = crate::commands::reaching(chat, out, crate::commands::HOST);
            self.host_lines.extend(lines);
        }
    }

    pub fn send_levels(&mut self, chat: &CommandServer) {
        for session in self.sessions.values_mut() {
            let level = chat.level(PlayerId(session.client));
            if session.command_level != Some(level) {
                session.command_level = Some(level);
                let commands = Clientbound::Commands { level };
                session.outbox.push(ToClient::Chat(commands));
            }
        }
    }

    pub fn take_retiring(&mut self, now: Instant) -> Vec<(u32, String)> {
        let (due, waiting) = std::mem::take(&mut self.retiring)
            .into_iter()
            .partition(|(_, _, since)| now.duration_since(*since) >= RETIRE_AFTER);
        self.retiring = waiting;
        due.into_iter()
            .map(|(client, reason, _)| (client, reason))
            .collect()
    }

    pub fn batch_received(&mut self, client: u32, seq: u32) -> bool {
        self.sessions
            .get_mut(&client)
            .is_some_and(|session| session.chunks.received(seq))
    }

    pub fn batch_encoded(&mut self, client: u32, rest: &[ChunkXz]) {
        if let Some(session) = self.sessions.get_mut(&client) {
            session.chunks.cut(rest);
        }
    }

    pub fn lock_travel(&mut self, players: &mut BTreeMap<PlayerUuid, Player>) {
        let locked = !self.sessions.is_empty();
        for player in players.values_mut() {
            player.travel_locked = locked;
            if !std::mem::take(&mut player.travel_refused) {
                continue;
            }
            match self.sessions.get_mut(&player.id.0) {
                Some(session) => session.outbox.push(ToClient::Chat(travel_off_line())),
                None => self.host_lines.push(travel_off_line()),
            }
        }
    }

    pub fn send_time(&mut self, globals: &Globals, levels: &BTreeMap<Dimension, Level>) {
        let time = time_of(globals);
        for session in self.sessions.values_mut() {
            session.outbox.push(time.clone());
            let weather = session
                .chunks
                .level
                .and_then(|dimension| levels.get(&dimension))
                .and_then(|level| level.entities.as_ref())
                .map(|entities| entities.weather);
            if let Some(weather) = weather {
                session.outbox.push(ToClient::Weather {
                    rain: weather.rain_level,
                    thunder: weather.thunder_level,
                });
            }
        }
    }

    pub fn send_updates(&mut self, dimension: Dimension, updates: &Updates) {
        for session in self.sessions.values_mut() {
            if session.chunks.level == Some(dimension) {
                session.send_updates(updates);
            }
        }
    }

    pub fn fan_out(&mut self, levels: &mut BTreeMap<Dimension, Level>) {
        for (&dimension, level) in levels.iter_mut() {
            let watched = self
                .sessions
                .values()
                .any(|session| session.chunks.level == Some(dimension));
            if !watched {
                level.terrain.friend_changes = None;
                level.terrain.friend_breaks = None;
                continue;
            }
            let breaks = level
                .terrain
                .friend_breaks
                .replace(Vec::new())
                .unwrap_or_default();
            for session in self.sessions.values_mut() {
                if session.chunks.level == Some(dimension) {
                    session.send_breaks(&breaks);
                }
            }
            let Some(changed) = level.terrain.friend_changes.replace(Vec::new()) else {
                continue;
            };
            if changed.is_empty() {
                continue;
            }
            let updates = block_updates(&mut level.terrain, &changed);
            self.send_updates(dimension, &updates);
        }
    }

    pub fn select_batches(&mut self, server: &mut Server) -> Vec<BatchJob> {
        let mut jobs = Vec::new();
        for session in self.sessions.values_mut() {
            let Some(player) = server.players.get(&session.uuid) else {
                continue;
            };
            let dimension = player.dimension;
            let Some(feet) = player
                .arrival
                .or(player.place)
                .filter(|place| place.dimension == dimension)
                .map(|place| place.feet)
            else {
                continue;
            };
            let center = ((feet[0].floor() as i32) >> 4, (feet[2].floor() as i32) >> 4);
            session
                .chunks
                .follow(dimension, center, &mut session.outbox);
            let Some(level) = server.levels.get_mut(&dimension) else {
                continue;
            };
            level.terrain.friend_changes.get_or_insert_with(Vec::new);
            level.terrain.friend_breaks.get_or_insert_with(Vec::new);
            let scene = &level.terrain.scene;
            let Some((seq, positions)) = session.chunks.next_batch(
                |pos| scene.generated_chunk(pos).is_some(),
                &mut session.outbox,
            ) else {
                continue;
            };
            let mut chunks = Vec::new();
            let mut missing = Vec::new();
            for (x, z) in positions {
                match level.terrain.stream.loaded_chunk(ChunkPos::new(x, z)) {
                    Some(chunk) => chunks.push(chunk),
                    None => missing.push((x, z)),
                }
            }
            session.chunks.cut(&missing);
            jobs.push(BatchJob {
                client: session.client,
                seq,
                chunks,
            });
        }
        jobs
    }

    pub fn send_cracks(
        &mut self,
        players: &BTreeMap<PlayerUuid, Player>,
        levels: &BTreeMap<Dimension, Level>,
    ) {
        for session in self.sessions.values_mut() {
            let feet = players.get(&session.uuid).and_then(|player| {
                player
                    .place
                    .filter(|place| place.dimension == player.dimension)
                    .map(|place| (player.dimension, place.feet))
            });
            let cracks = match (session.chunks.level, feet) {
                (Some(dimension), Some((standing, feet))) if dimension == standing => levels
                    .get(&dimension)
                    .map(|level| level_cracks(dimension, level, players))
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|((_, pos), _)| near(feet, *pos, CRACK_RANGE))
                    .collect(),
                _ => BTreeMap::new(),
            };
            session.send_cracks(cracks);
        }
    }

    pub fn send_views(&mut self, players: &BTreeMap<PlayerUuid, Player>) {
        for session in self.sessions.values_mut() {
            if let Some(player) = players.get(&session.uuid) {
                session.send_view(player);
            }
        }
    }

    pub fn send_menus(
        &mut self,
        players: &mut BTreeMap<PlayerUuid, Player>,
        levels: &mut BTreeMap<Dimension, Level>,
        books: crate::menus::Books<'_>,
    ) {
        for session in self.sessions.values_mut() {
            let Some(player) = players.get_mut(&session.uuid) else {
                continue;
            };
            let Some(level) = levels.get_mut(&player.dimension) else {
                continue;
            };
            let changes = crate::menus::changes(player, &mut level.containers, books);
            session.outbox.extend(changes);
        }
    }

    pub fn take_outgoing(&mut self) -> Vec<(u32, ToClient)> {
        let mut out = std::mem::take(&mut self.refused);
        for session in self.sessions.values_mut() {
            let client = session.client;
            out.extend(session.outbox.drain(..).map(|message| (client, message)));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::HOST;
    use crate::server::tests::{TestDir, registries, test_globals, world};

    fn all(_: ChunkXz) -> bool {
        true
    }

    fn markers(out: &[ToClient]) -> Vec<u32> {
        out.iter()
            .filter_map(|message| match message {
                ToClient::ChunkBatch { seq } => Some(*seq),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn one_batch_flies_until_two_are_acknowledged_then_two() {
        let mut sender = ChunkSender::new(4);
        let mut out = Vec::new();
        sender.follow(Dimension::Overworld, (0, 0), &mut out);
        let (seq, first) = sender.next_batch(all, &mut out).expect("a first batch");
        assert_eq!(seq, 1);
        assert_eq!(first.len(), BATCH_CHUNKS);
        assert_eq!(first[0], (0, 0), "the nearest chunk goes first");
        assert!(sender.next_batch(all, &mut out).is_none());
        assert!(sender.received(1));
        assert_eq!(
            sender.next_batch(all, &mut out).map(|(seq, _)| seq),
            Some(2)
        );
        assert!(sender.next_batch(all, &mut out).is_none());
        assert!(sender.received(2));
        assert_eq!(sender.window(), 2);
        assert_eq!(
            sender.next_batch(all, &mut out).map(|(seq, _)| seq),
            Some(3)
        );
        assert_eq!(
            sender.next_batch(all, &mut out).map(|(seq, _)| seq),
            Some(4)
        );
        assert!(sender.next_batch(all, &mut out).is_none());
        assert_eq!(markers(&out), vec![1, 2, 3, 4]);
    }

    #[test]
    fn an_unacknowledged_batch_holds_the_stream_and_stray_acks_do_not_open_it() {
        let mut sender = ChunkSender::new(4);
        let mut out = Vec::new();
        sender.follow(Dimension::Overworld, (0, 0), &mut out);
        let (seq, chunks) = sender.next_batch(all, &mut out).unwrap();
        for _ in 0..100 {
            assert!(sender.next_batch(all, &mut out).is_none());
        }
        assert!(!sender.received(seq + 7), "an unknown batch");
        assert!(sender.next_batch(all, &mut out).is_none());
        assert!(chunks.iter().all(|&pos| sender.has(pos)));
        assert!(sender.received(seq));
        assert!(!sender.received(seq), "a second ack for a resent batch");
        assert_eq!(sender.window(), 1);
        assert_eq!(sender.in_flight(), 0);
        assert!(sender.next_batch(all, &mut out).is_some());
    }

    #[test]
    fn only_loaded_chunks_are_sent_and_a_cut_chunk_is_sent_again() {
        let mut sender = ChunkSender::new(2);
        let mut out = Vec::new();
        sender.follow(Dimension::Overworld, (0, 0), &mut out);
        let loaded = |pos: ChunkXz| pos.0 >= 0;
        let (_, chunks) = sender.next_batch(loaded, &mut out).unwrap();
        assert!(chunks.iter().all(|pos| pos.0 >= 0));
        sender.cut(&chunks[3..]);
        assert!(chunks[..3].iter().all(|&pos| sender.has(pos)));
        assert!(chunks[3..].iter().all(|&pos| !sender.has(pos)));
        assert!(sender.received(1));
        let (_, again) = sender.next_batch(loaded, &mut out).unwrap();
        assert!(chunks[3..].iter().all(|pos| again.contains(pos)));
    }

    #[test]
    fn a_forget_after_a_load_stays_behind_the_marker() {
        let mut sender = ChunkSender::new(2);
        let mut out = Vec::new();
        sender.follow(Dimension::Overworld, (0, 0), &mut out);
        let (seq, sent) = sender.next_batch(all, &mut out).unwrap();
        sender.follow(Dimension::Overworld, (40, 40), &mut out);
        let marker = out
            .iter()
            .position(|message| *message == ToClient::ChunkBatch { seq })
            .unwrap();
        let forgets: Vec<usize> = out
            .iter()
            .enumerate()
            .filter_map(|(at, message)| match message {
                ToClient::ChunkForget { pos } => {
                    assert!(
                        sent.contains(&(pos[0], pos[1])),
                        "only sent chunks are forgotten"
                    );
                    Some(at)
                }
                _ => None,
            })
            .collect();
        assert_eq!(forgets.len(), sent.len());
        assert!(forgets.iter().all(|&at| at > marker));
        assert!(sent.iter().all(|&pos| !sender.has(pos)));
    }

    #[test]
    fn another_level_starts_the_chunk_stream_again() {
        let mut sender = ChunkSender::new(2);
        let mut out = Vec::new();
        sender.follow(Dimension::Overworld, (0, 0), &mut out);
        sender.next_batch(all, &mut out).unwrap();
        sender.follow(Dimension::Nether, (0, 0), &mut out);
        assert!(!sender.has((0, 0)));
        assert_eq!(sender.in_flight(), 0);
        assert!(sender.next_batch(all, &mut out).is_some());
    }

    fn session_with(client: u32, level: Dimension) -> Session {
        let mut session = Session::new(client, PlayerUuid(format!("friend-{client}")), "Alex", 2);
        session.chunks.follow(level, (0, 0), &mut session.outbox);
        session
            .chunks
            .next_batch(|pos| pos == (0, 0), &mut session.outbox)
            .unwrap();
        session.outbox.clear();
        session
    }

    #[test]
    fn block_updates_reach_only_friends_in_that_level_with_that_chunk() {
        let mut sessions = Sessions::default();
        sessions
            .sessions
            .insert(1, session_with(1, Dimension::Overworld));
        sessions
            .sessions
            .insert(2, session_with(2, Dimension::Nether));
        let chest = Nbt(vec![10, 0, 0, 0]);
        let updates = Updates {
            states: vec![([1, 64, 1], 7), ([40, 64, 40], 9)],
            entities: vec![([2, 64, 2], chest.clone()), ([41, 64, 41], chest.clone())],
        };
        sessions.send_updates(Dimension::Overworld, &updates);
        let overworld = &sessions.sessions[&1].outbox;
        assert_eq!(
            overworld,
            &vec![
                ToClient::BlockUpdates {
                    changes: vec![([1, 64, 1], 7)]
                },
                ToClient::BlockEntity {
                    pos: [2, 64, 2],
                    tag: chest
                },
            ]
        );
        assert!(sessions.sessions[&2].outbox.is_empty());
    }

    #[test]
    fn a_batch_is_cut_at_its_size_limit_and_keeps_at_least_one_chunk() {
        let items: Vec<([i32; 2], usize)> = vec![
            ([0, 0], 60 * 1024),
            ([1, 0], 60 * 1024),
            ([2, 0], 60 * 1024),
            ([3, 0], 1024),
        ];
        let blob = |size: &usize| ChunkBlob(vec![7; *size]);
        let cut = cut_batch(5, &items, blob);
        let batch = ChunkBatchBlob::decode(&cut.blob).unwrap();
        assert_eq!(batch.seq, 5);
        assert_eq!(
            batch.chunks.iter().map(|(pos, _)| *pos).collect::<Vec<_>>(),
            vec![[0, 0], [1, 0]]
        );
        assert_eq!(cut.rest, vec![(2, 0), (3, 0)]);
        let big = cut_batch(6, &[([0, 0], 200 * 1024), ([1, 0], 10)], blob);
        assert_eq!(ChunkBatchBlob::decode(&big.blob).unwrap().chunks.len(), 1);
        assert_eq!(big.rest, vec![(1, 0)]);
        let huge = cut_batch(7, &[([0, 0], 300 * 1024), ([1, 0], 10)], blob);
        let kept = ChunkBatchBlob::decode(&huge.blob).unwrap();
        assert_eq!(
            kept.chunks.iter().map(|(pos, _)| *pos).collect::<Vec<_>>(),
            vec![[1, 0]]
        );
        assert!(huge.rest.is_empty());
        let empty = cut_batch::<usize>(8, &[], blob);
        assert_eq!(ChunkBatchBlob::decode(&empty.blob).unwrap().seq, 8);
    }

    fn alex() -> Player {
        let mut chat = CommandServer::new(HOST);
        chat.join(PlayerId(1), "Alex");
        Player::new(
            chat.profile(PlayerId(1)).unwrap(),
            minecraft_terrain::game_mode::PlayerMode::default(),
            crate::survival::Survival::default(),
            minecraftoss_player::inventory::Inventory::default(),
        )
    }

    #[test]
    fn a_friend_respawns_only_after_a_life_has_been_seen() {
        let mut player = alex();
        let spectating = playerstate_iw4::PM_TYPE_SPECTATOR;
        assert!(!player.observe_life(spectating));
        assert!(!player.observe_life(0), "the first spawn is the join");
        assert!(!player.observe_life(0));
        assert!(!player.observe_life(playerstate_iw4::PM_TYPE_DEAD));
        assert!(player.observe_life(0));
    }

    #[test]
    fn a_player_view_is_sent_once_for_each_change() {
        let mut player = alex();
        let mut session = Session::new(1, player.uuid.clone(), "Alex", 2);
        let views = |session: &mut Session| {
            std::mem::take(&mut session.outbox)
                .into_iter()
                .filter_map(|message| match message {
                    ToClient::Player(view) => Some(*view),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        session.send_view(&player);
        session.send_view(&player);
        assert_eq!(views(&mut session).len(), 1);
        player.survival.status.food.level = 12;
        player.selected = 4;
        session.send_view(&player);
        session.send_view(&player);
        let sent = views(&mut session);
        assert_eq!(sent.len(), 1);
        assert_eq!((sent[0].food, sent[0].selected), (12, 4));
        player.survival.status.add_effect(
            minecraftoss_player::survival::EffectKind::Regeneration,
            100,
            0,
        );
        session.send_view(&player);
        assert_eq!(views(&mut session).len(), 1);
        player.survival.status.tick_effects();
        session.send_view(&player);
        assert!(
            views(&mut session).is_empty(),
            "a running effect is not sent again"
        );
    }

    fn host_server(name: &str) -> Option<(TestDir, Server)> {
        let registries = registries()?;
        let (dir, save) = world(name);
        let mut server = Server {
            globals: test_globals(registries, Some(save)),
            levels: BTreeMap::new(),
            opening: BTreeMap::new(),
            players: BTreeMap::new(),
            pending_batch: None,
        };
        let uuid = server
            .join_friend(HOST, "Steve", None, Dimension::Overworld)
            .unwrap();
        server.players.get_mut(&uuid).unwrap().arrival = None;
        Some((dir, server))
    }

    fn context(origins: &[(Dimension, [f64; 3])]) -> JoinContext<'_> {
        JoinContext {
            registry: 42,
            dimension: Dimension::Overworld,
            origins,
            view_distance: 8,
            kit: Some("dirt*3"),
        }
    }

    const HELLO: Hello = Hello {
        protocol: PROTOCOL,
        view_distance: 12,
        registry: 42,
    };

    #[test]
    fn a_hello_joins_the_friend_with_login_first() {
        let Some((_dir, mut server)) = host_server("session-join") else {
            return;
        };
        let origins = [(Dimension::Overworld, [0.5, 64.0, 0.5])];
        let mut sessions = Sessions::default();
        let now = Instant::now();
        let answer = sessions.hello(&mut server, 1, "Alex", HELLO, &context(&origins), now);
        let Answer::Joined(uuid) = answer else {
            panic!("{answer:?}");
        };
        let alex = &server.players[&uuid];
        assert_eq!(alex.id, PlayerId(1));
        assert!(alex.arrival.is_some(), "the first spawn is placed");
        assert_eq!(server.globals.chat.name(PlayerId(1)), Some("Alex"));
        let out = sessions.take_outgoing();
        let ToClient::Login(login) = &out[0].1 else {
            panic!("{out:?}");
        };
        assert_eq!(
            (login.client, login.view_distance, login.registry),
            (1, 8, 42)
        );
        assert_eq!(login.origins, origins.to_vec());
        assert!(matches!(&out[1].1, ToClient::PlayerList { added, .. } if added.len() == 2));
        assert!(matches!(&out[2].1, ToClient::Chat(_)));
        assert_eq!(sessions.host_lines, vec![joined_line("Alex")]);
        let again = sessions.hello(&mut server, 1, "Alex", HELLO, &context(&origins), now);
        assert_eq!(again, Answer::Ignored);
        assert!(sessions.take_outgoing().is_empty());
        let bob = sessions.hello(&mut server, 2, "Bob", HELLO, &context(&origins), now);
        assert!(matches!(bob, Answer::Joined(_)));
        let out = sessions.take_outgoing();
        assert!(out.iter().any(|(client, message)| *client == 1
            && matches!(message, ToClient::PlayerList { added, .. } if added.len() == 1)));
    }

    #[test]
    fn a_second_hello_with_a_name_in_the_world_is_refused_and_retired() {
        let Some((_dir, mut server)) = host_server("session-twice") else {
            return;
        };
        let origins = [];
        let mut sessions = Sessions::default();
        let now = Instant::now();
        let answer = sessions.hello(&mut server, 3, "steve", HELLO, &context(&origins), now);
        assert_eq!(answer, Answer::Refused(Refusal::NameTaken));
        assert_eq!(server.players.len(), 1);
        assert!(server.globals.chat.profile(PlayerId(3)).is_none());
        assert_eq!(
            sessions.take_outgoing(),
            vec![(
                3,
                ToClient::Rejected {
                    reason: Text::literal(NAME_TAKEN)
                }
            )]
        );
        assert_eq!(
            sessions.hello(&mut server, 3, "steve", HELLO, &context(&origins), now),
            Answer::Ignored
        );
        assert!(
            sessions.take_retiring(now).is_empty(),
            "the refusal goes out first"
        );
        assert_eq!(
            sessions.take_retiring(now + RETIRE_AFTER),
            vec![(3, NAME_TAKEN.to_owned())]
        );
        let old = Hello {
            protocol: PROTOCOL - 1,
            ..HELLO
        };
        assert!(matches!(
            sessions.hello(&mut server, 4, "Alex", old, &context(&origins), now),
            Answer::Refused(Refusal::Hello(HelloRefusal::Protocol { .. }))
        ));
        let other = Hello {
            registry: 41,
            ..HELLO
        };
        assert_eq!(
            sessions.hello(&mut server, 5, "Alex", other, &context(&origins), now),
            Answer::Refused(Refusal::Hello(HelloRefusal::Registry))
        );
        assert!(sessions.is_empty());
    }

    #[test]
    fn a_leaving_friend_is_listed_out_and_their_session_ends() {
        let Some((_dir, mut server)) = host_server("session-leave") else {
            return;
        };
        let origins = [];
        let mut sessions = Sessions::default();
        let now = Instant::now();
        sessions.hello(&mut server, 1, "Alex", HELLO, &context(&origins), now);
        sessions.hello(&mut server, 2, "Bob", HELLO, &context(&origins), now);
        sessions.take_outgoing();
        sessions.host_lines.clear();
        let left = sessions
            .leave(&mut server, 1)
            .expect("Alex was in the world");
        assert_eq!(left.player.name, "Alex");
        assert!(sessions.leave(&mut server, 1).is_none());
        assert_eq!(server.players.len(), 2);
        assert_eq!(sessions.host_lines, vec![left_line("Alex")]);
        let out = sessions.take_outgoing();
        assert_eq!(
            out,
            vec![
                (
                    2,
                    ToClient::PlayerList {
                        added: Vec::new(),
                        removed: vec![1]
                    }
                ),
                (2, ToClient::Chat(left_line("Alex"))),
            ]
        );
        server.globals.writer.finish();
    }

    fn said(line: &Clientbound) -> String {
        let lang = |key: &str| {
            let text = match key {
                "chat.type.text" => "<%s> %s",
                "chat.type.admin" => "[%s: %s]",
                "command.unknown.command" => "Unknown or incomplete command. See below for error",
                "command.context.here" => "<--[HERE]",
                "commands.message.display.incoming" => "%s whispers to you: %s",
                "commands.message.display.outgoing" => "You whisper to %s: %s",
                "commands.time.set.absolute" => "Set %s to %s tick(s)",
                "commands.op.success" => "Made %s a server operator",
                "argument.entity.notfound.player" => "No player was found",
                "multiplayer.player.joined" => "%s joined the game",
                "multiplayer.player.left" => "%s left the game",
                _ => return None,
            };
            Some(text.to_owned())
        };
        line.chat_text()
            .map_or_else(String::new, |text| text.resolve(&lang).text)
    }

    fn chat_of(out: &[(u32, ToClient)], client: u32) -> Vec<String> {
        out.iter()
            .filter(|(to, _)| *to == client)
            .filter_map(|(_, message)| match message {
                ToClient::Chat(line) => Some(said(line)),
                _ => None,
            })
            .collect()
    }

    fn say(
        server: &mut Server,
        sessions: &mut Sessions,
        who: &PlayerUuid,
        line: &str,
    ) -> Vec<String> {
        let lines = minecraft_commands::Serverbound::from_input(line)
            .into_iter()
            .collect();
        let Server {
            globals,
            levels,
            players,
            ..
        } = server;
        let host = players.get(who).is_some_and(|player| player.id == HOST);
        let handled = crate::commands::run(globals, levels, players, who, lines, Vec::new());
        sessions.deliver(&globals.chat, &handled.delivered, !host);
        let mut seen = std::mem::take(&mut sessions.host_lines);
        if host {
            seen = crate::commands::reaching(&globals.chat, &handled.delivered, HOST);
        }
        seen.iter().map(said).collect()
    }

    fn uuid_of(server: &Server, id: u32) -> PlayerUuid {
        server
            .players
            .values()
            .find(|player| player.id == PlayerId(id))
            .map(|player| player.uuid.clone())
            .unwrap()
    }

    #[test]
    fn chat_reaches_everyone_and_tell_reaches_one() {
        let Some((_dir, mut server)) = host_server("session-chat") else {
            return;
        };
        let origins = [];
        let mut sessions = Sessions::default();
        let now = Instant::now();
        sessions.hello(&mut server, 1, "Alex", HELLO, &context(&origins), now);
        sessions.hello(&mut server, 2, "Bob", HELLO, &context(&origins), now);
        sessions.take_outgoing();
        sessions.host_lines.clear();
        let (steve, alex) = (uuid_of(&server, 0), uuid_of(&server, 1));
        let seen = say(&mut server, &mut sessions, &alex, "  hello   all ");
        assert_eq!(seen, vec!["<Alex> hello all"]);
        let out = sessions.take_outgoing();
        assert_eq!(chat_of(&out, 1), vec!["<Alex> hello all"]);
        assert_eq!(chat_of(&out, 2), vec!["<Alex> hello all"]);
        let seen = say(&mut server, &mut sessions, &steve, "hi from the host");
        assert_eq!(seen, vec!["<Steve> hi from the host"]);
        let out = sessions.take_outgoing();
        assert_eq!(chat_of(&out, 1), vec!["<Steve> hi from the host"]);
        assert_eq!(chat_of(&out, 2), vec!["<Steve> hi from the host"]);
        let seen = say(&mut server, &mut sessions, &steve, "/tell bob psst");
        assert_eq!(seen, vec!["You whisper to Bob: psst"]);
        let out = sessions.take_outgoing();
        assert!(chat_of(&out, 1).is_empty());
        assert_eq!(chat_of(&out, 2), vec!["Steve whispers to you: psst"]);
        let seen = say(&mut server, &mut sessions, &alex, "/msg Steve back");
        assert_eq!(seen, vec!["Alex whispers to you: back"]);
        let out = sessions.take_outgoing();
        assert_eq!(chat_of(&out, 1), vec!["You whisper to Steve: back"]);
        assert!(chat_of(&out, 2).is_empty());
        server.globals.writer.finish();
    }

    #[test]
    fn an_op_only_command_is_refused_for_a_new_friend_and_allowed_for_the_host() {
        let Some((_dir, mut server)) = host_server("session-op") else {
            return;
        };
        let origins = [];
        let mut sessions = Sessions::default();
        sessions.hello(
            &mut server,
            1,
            "Alex",
            HELLO,
            &context(&origins),
            Instant::now(),
        );
        sessions.take_outgoing();
        sessions.host_lines.clear();
        let (steve, alex) = (uuid_of(&server, 0), uuid_of(&server, 1));
        assert_eq!(server.globals.chat.level(PlayerId(1)), 0);
        let level = |level| vec![(1, ToClient::Chat(Clientbound::Commands { level }))];
        sessions.send_levels(&server.globals.chat);
        assert_eq!(sessions.take_outgoing(), level(0));
        sessions.send_levels(&server.globals.chat);
        assert!(sessions.take_outgoing().is_empty());
        let seen = say(&mut server, &mut sessions, &alex, "/time set 2000");
        assert!(seen.is_empty());
        let out = sessions.take_outgoing();
        assert_eq!(
            chat_of(&out, 1).first().map(String::as_str),
            Some("Unknown or incomplete command. See below for error")
        );
        assert_ne!(server.globals.day.ticks, 2000.0);
        let seen = say(&mut server, &mut sessions, &steve, "/time set 2000");
        assert_eq!(seen, vec!["Set minecraft:overworld to 2000 tick(s)"]);
        assert_eq!(server.globals.day.ticks, 2000.0);
        assert!(chat_of(&sessions.take_outgoing(), 1).is_empty());
        let seen = say(&mut server, &mut sessions, &steve, "/op alex");
        assert_eq!(seen, vec!["Made Alex a server operator"]);
        assert_eq!(
            chat_of(&sessions.take_outgoing(), 1),
            vec!["[Steve: Made Alex a server operator]"]
        );
        sessions.send_levels(&server.globals.chat);
        assert_eq!(sessions.take_outgoing(), level(4));
        let seen = say(&mut server, &mut sessions, &alex, "/time set 3000");
        assert_eq!(
            seen,
            vec!["[Alex: Set minecraft:overworld to 3000 tick(s)]"]
        );
        assert_eq!(
            chat_of(&sessions.take_outgoing(), 1),
            vec!["Set minecraft:overworld to 3000 tick(s)"]
        );
        assert_eq!(server.globals.day.ticks, 3000.0);
        server.globals.writer.finish();
    }

    #[test]
    fn a_friend_is_named_in_chat_while_in_the_world_and_never_after() {
        let Some((_dir, mut server)) = host_server("session-names") else {
            return;
        };
        let origins = [];
        let mut sessions = Sessions::default();
        let now = Instant::now();
        sessions.hello(&mut server, 1, "Alex", HELLO, &context(&origins), now);
        sessions.hello(&mut server, 2, "Bob", HELLO, &context(&origins), now);
        let out = sessions.take_outgoing();
        assert_eq!(
            chat_of(&out, 1),
            vec!["Alex joined the game", "Bob joined the game"]
        );
        let joined: Vec<String> = sessions
            .host_lines
            .drain(..)
            .map(|line| said(&line))
            .collect();
        assert_eq!(joined, vec!["Alex joined the game", "Bob joined the game"]);
        let steve = uuid_of(&server, 0);
        let seen = say(
            &mut server,
            &mut sessions,
            &steve,
            "/tell Alex still there?",
        );
        assert_eq!(seen, vec!["You whisper to Alex: still there?"]);
        sessions.take_outgoing();
        sessions.leave(&mut server, 1);
        let left: Vec<String> = sessions
            .host_lines
            .drain(..)
            .map(|line| said(&line))
            .collect();
        assert_eq!(left, vec!["Alex left the game"]);
        assert_eq!(
            chat_of(&sessions.take_outgoing(), 2),
            vec!["Alex left the game"]
        );
        assert_eq!(server.globals.chat.name(PlayerId(1)), None);
        let seen = say(
            &mut server,
            &mut sessions,
            &steve,
            "/tell Alex still there?",
        );
        assert_eq!(seen, vec!["No player was found"]);
        assert!(sessions.take_outgoing().is_empty());
        server.globals.writer.finish();
    }

    #[test]
    fn travel_is_refused_while_a_friend_is_online() {
        let Some((_dir, mut server)) = host_server("session-travel") else {
            return;
        };
        let origins = [];
        let mut sessions = Sessions::default();
        sessions.lock_travel(&mut server.players);
        assert!(server.players.values().all(|player| !player.travel_locked));
        sessions.hello(
            &mut server,
            1,
            "Alex",
            HELLO,
            &context(&origins),
            Instant::now(),
        );
        sessions.take_outgoing();
        sessions.host_lines.clear();
        sessions.lock_travel(&mut server.players);
        for player in server.players.values_mut() {
            assert!(player.refuse_travel());
        }
        sessions.lock_travel(&mut server.players);
        assert_eq!(sessions.host_lines, vec![travel_off_line()]);
        assert_eq!(
            sessions.take_outgoing(),
            vec![(1, ToClient::Chat(travel_off_line()))]
        );
        let steve = server
            .players
            .values_mut()
            .find(|player| player.id == HOST)
            .unwrap();
        let travel = crate::portal::PortalTravel {
            kind: crate::portal::PortalKind::Nether,
            to: Dimension::Nether,
            target: (0, 70, 0),
            entry_axis: crate::portal::PortalAxis::X,
            relative: [0.5, 0.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
            motion: [0.0; 3],
            creates: true,
        };
        if !steve.refuse_travel() {
            crate::portal::set_off(steve, travel);
        }
        assert_eq!(steve.dimension, Dimension::Overworld);
        assert!(steve.switching.is_none());
        sessions.leave(&mut server, 1);
        sessions.lock_travel(&mut server.players);
        let steve = server.players.values_mut().next().unwrap();
        assert!(!steve.refuse_travel());
        server.globals.writer.finish();
    }

    #[test]
    fn every_friend_gets_the_time_of_both_clocks_with_their_pause() {
        let Some((_dir, mut server)) = host_server("session-time") else {
            return;
        };
        server.globals.day.set(6000.0);
        server.globals.end_clock.ticks = 1234.0;
        let origins = [];
        let mut sessions = Sessions::default();
        let now = Instant::now();
        sessions.hello(&mut server, 1, "Alex", HELLO, &context(&origins), now);
        sessions.hello(&mut server, 2, "Bob", HELLO, &context(&origins), now);
        sessions.take_outgoing();
        sessions.send_time(&server.globals, &server.levels);
        let time = ToClient::Time {
            game_time: 6000,
            clocks: vec![
                (OVERWORLD_CLOCK.to_owned(), 6000.0, false),
                (THE_END_CLOCK.to_owned(), 1234.0, false),
            ],
        };
        assert_eq!(sessions.take_outgoing(), vec![(1, time.clone()), (2, time)]);
        server.globals.day.paused = true;
        let ToClient::Time { clocks, .. } = time_of(&server.globals) else {
            panic!("a time message");
        };
        assert_eq!((clocks[0].2, clocks[1].2), (true, false));
        server
            .globals
            .game_rules
            .set("advance_time", "false")
            .unwrap();
        let ToClient::Time { clocks, .. } = time_of(&server.globals) else {
            panic!("a time message");
        };
        assert_eq!((clocks[0].2, clocks[1].2), (true, true));
        server.globals.writer.finish();
    }

    #[test]
    fn the_registry_fingerprint_is_stable_and_sees_every_state() {
        let Some(registries) = registries() else {
            return;
        };
        let fingerprint = registry_fingerprint(&registries);
        assert_eq!(fingerprint, registry_fingerprint(&registries));
        assert_ne!(fingerprint, 0xcbf2_9ce4_8422_2325);
    }

    fn hand(seq: u32, kind: HandEventKind) -> HandEvent {
        HandEvent {
            seq,
            kind,
            target: Target::Miss,
        }
    }

    fn held_state(tick: u32) -> HandState {
        HandState {
            tick,
            attack: false,
            use_item: true,
            target: Target::Miss,
        }
    }

    #[test]
    fn a_press_and_a_release_in_one_host_frame_both_act() {
        let now = Instant::now();
        let mut queue = HandQueue::default();
        queue.push(vec![
            hand(1, HandEventKind::UsePressed),
            hand(2, HandEventKind::UseReleased),
            hand(3, HandEventKind::AttackPressed),
            hand(4, HandEventKind::AttackReleased),
            hand(5, HandEventKind::SwapHands),
            hand(6, HandEventKind::Drop { whole_stack: true }),
        ]);
        let taken = queue.take(now, |_| true);
        assert!(taken.hands.use_pressed && !taken.hands.use_held);
        assert!(taken.hands.attack_pressed && !taken.hands.attack_held);
        assert!(taken.swap);
        assert_eq!(taken.drops, [true]);
        let Some((mut level, globals)) = crate::actions::tests::level_and_globals() else {
            return;
        };
        use crate::actions::tests::{Flat, actor, count, holding, moved, pitch_to};
        let mut player = holding("minecraft:cobblestone", 32);
        let looking = actor(0.0, pitch_to(actor(0.0, 0.0).eye, [4.5, 201.0, 6.5]));
        queue.push(vec![
            hand(7, HandEventKind::UsePressed),
            hand(8, HandEventKind::UseReleased),
        ]);
        for frame in 0..3 {
            let taken = queue.take(now, |_| true);
            assert_eq!(taken.hands.use_pressed, frame == 0);
            let _ = crate::actions::act(
                &mut player,
                &mut level,
                &globals,
                &mut Flat,
                &looking,
                taken.hands,
                &moved(),
                None,
                0.0,
            );
        }
        let placed = minecraft_terrain::scene::Scene::block(&level.terrain.scene, (4, 201, 6))
            .map(|b| b.id.key());
        assert_eq!(placed.as_deref(), Some("minecraft:cobblestone"));
        assert_eq!(count(&player), Some(31), "the click placed one block");
    }

    #[test]
    fn a_press_is_used_once_however_many_host_frames_pass() {
        let start = Instant::now();
        let mut queue = HandQueue::default();
        queue.push(vec![hand(1, HandEventKind::AttackPressed)]);
        queue.push(vec![hand(1, HandEventKind::AttackPressed)]);
        let first = queue.take(start, |_| true);
        assert!(first.hands.attack_pressed && first.hands.attack_held);
        for frame in 1..10 {
            let later = start + Duration::from_millis(frame * 10);
            let taken = queue.take(later, |_| true);
            assert!(!taken.hands.attack_pressed, "frame {frame}");
            assert!(taken.hands.attack_held, "frame {frame}");
        }
        queue.push(vec![hand(2, HandEventKind::AttackReleased)]);
        let released = queue.take(start + Duration::from_millis(100), |_| true);
        assert!(!released.hands.attack_pressed && !released.hands.attack_held);
    }

    #[test]
    fn a_held_use_without_a_fresh_hand_state_for_half_a_second_stops() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut queue = HandQueue::default();
        queue.push(vec![hand(1, HandEventKind::UsePressed)]);
        assert!(queue.take(start, |_| true).hands.use_held);
        queue.state(held_state(1), at(300));
        queue.state(held_state(1), at(600));
        assert!(queue.take(at(700), |_| true).hands.use_held);
        assert!(
            !queue.take(at(801), |_| true).hands.use_held,
            "a repeated tick is not a fresh state"
        );
        queue.state(held_state(2), at(900));
        assert!(queue.take(at(1300), |_| true).hands.use_held);
        assert!(!queue.take(at(1400), |_| true).hands.use_held);
        queue.push(vec![hand(2, HandEventKind::UseReleased)]);
        queue.state(held_state(3), at(1500));
        assert!(
            !queue.take(at(1500), |_| true).hands.use_held,
            "a state after the release does not hold the button again"
        );
    }

    #[test]
    fn a_press_at_a_target_out_of_reach_is_refused() {
        let now = Instant::now();
        let mut queue = HandQueue::default();
        queue.push(vec![
            hand(1, HandEventKind::UsePressed),
            hand(2, HandEventKind::AttackPressed),
        ]);
        let taken = queue.take(now, |_| false);
        assert_eq!(taken, HandTake::default());
        queue.state(held_state(1), now);
        assert!(!queue.take(now, |_| true).hands.use_held);
    }

    #[test]
    fn cracks_and_breaks_reach_a_friend_once_for_chunks_they_have() {
        let mut session = session_with(1, Dimension::Overworld);
        let cracks = |entries: &[(u32, BlockPos, u8)]| {
            entries
                .iter()
                .map(|&(breaker, pos, stage)| ((breaker, pos), stage))
                .collect::<BTreeMap<_, _>>()
        };
        session.send_cracks(cracks(&[(BULLETS, (1, 64, 1), 3), (5, (40, 64, 40), 2)]));
        assert_eq!(
            session.outbox,
            vec![ToClient::Crack {
                breaker: BULLETS,
                pos: [1, 64, 1],
                stage: 3
            }]
        );
        session.outbox.clear();
        session.send_cracks(cracks(&[(BULLETS, (1, 64, 1), 3)]));
        assert!(
            session.outbox.is_empty(),
            "an unchanged crack is not sent again"
        );
        session.send_cracks(cracks(&[(BULLETS, (1, 64, 1), 5), (2, (3, 64, 3), 0)]));
        session.send_cracks(cracks(&[(2, (3, 64, 3), 0)]));
        let crack = |breaker, pos, stage| ToClient::Crack {
            breaker,
            pos,
            stage,
        };
        assert_eq!(
            session.outbox,
            vec![
                crack(2, [3, 64, 3], 0),
                crack(BULLETS, [1, 64, 1], 5),
                crack(BULLETS, [1, 64, 1], -1),
            ]
        );
        session.outbox.clear();
        session.send_breaks(&[([1, 64, 1], 7), ([40, 64, 40], 9)]);
        assert_eq!(
            session.outbox,
            vec![ToClient::LevelEvent {
                event: BREAK_EVENT,
                pos: [1, 64, 1],
                data: 7
            }]
        );
        assert!(near([0.5, 64.0, 0.5], (20, 64, 20), CRACK_RANGE));
        assert!(!near([0.5, 64.0, 0.5], (30, 64, 30), CRACK_RANGE));
    }
}
