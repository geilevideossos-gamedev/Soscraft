use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use minecraft_commands::text::{GREEN, YELLOW};
use minecraft_commands::{Clientbound, CommandServer, Outgoing, PlayerId, Text};
use minecraft_net::codec::{BATCH_CUT, BULK_LIMIT};
use minecraft_net::messages::{
    BookView, BossBarColor, Lying, BossBarFlags, BossBarOperation, BossBarOverlay, BossBarUpdate, ChunkBlob,
    GearView, HandEvent, HandEventKind, HandState, HelloRefusal, Login, Nbt, PlayerView, Rules,
    Target, check_hello,
};
use minecraft_net::{ChunkBatchBlob, Dimension, PROTOCOL, PlayerUuid, StateFrame, ToClient};
use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::lectern::{BookStack, LecternAction, LecternView, TAKE_BOOK};
use minecraft_terrain::server::{OVERWORLD_CLOCK, THE_END_CLOCK};
use minecraft_terrain::view::{MIN_VIEW_DISTANCE, TrackingView};
use minecraftoss_core::{BlockStateId, Chunk, ChunkPos, Registries};

use crate::actions::Hands;
use crate::level::{Level, Terrain};
use crate::player::Player;
use crate::server::{Globals, Left, Server};
use crate::the_end::dragon_fight::DragonFight;

pub type ChunkXz = (i32, i32);
type BlockPos = (i32, i32, i32);

pub const BATCH_CHUNKS: usize = 9;
pub const RETIRE_AFTER: Duration = Duration::from_secs(1);
pub const NAME_TAKEN: &str = "A player with that name is already in the world";
pub const TIME_EVERY: Duration = Duration::from_secs(1);
pub const HELD_FOR: Duration = Duration::from_millis(500);
pub const HAND_EVENTS: usize = 256;
pub const BULLETS: u32 = u32::MAX;
pub const MOB_BREAKER: u32 = 0x4000_0000;
pub const CRACK_RANGE: f64 = 32.0;
pub const BREAK_EVENT: i32 = 2001;
pub const DRAGON_BAR: u64 = 1;
pub const DRAGON_NAME: &str = "entity.minecraft.ender_dragon";

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
    pub dimension: Dimension,
    pub epoch: u32,
    pub chunks: ChunkSender,
    pub outbox: Vec<ToClient>,
    pub hands: HandQueue,
    pub menu_inputs: Vec<crate::menus::MenuInput>,
    pub mobs: crate::mob_sender::MobSender,
    pub lectern: Option<BlockPos>,
    pub book_settings: Option<(bool, bool)>,
    pub seen_recipes: Vec<String>,
    book: Option<(usize, usize, [bool; 8])>,
    gear: BTreeMap<u32, GearView>,
    command_level: Option<u8>,
    view: Option<PlayerView>,
    spawn: Option<(Dimension, [i32; 3])>,
    cracks: BTreeMap<(u32, BlockPos), u8>,
    boss_bar: Option<f32>,
    raid_bars: BTreeMap<i32, minecraft_terrain::raids::RaidBar>,
    wither_bars: BTreeMap<u64, f32>,
    elden_bars: BTreeMap<u64, f32>,
    credits: bool,
    sleepers: BTreeMap<u32, Lying>,
}

impl Session {
    pub fn new(client: u32, uuid: PlayerUuid, name: &str, view_distance: i32) -> Self {
        Self {
            client,
            uuid,
            name: name.to_owned(),
            dimension: Dimension::Overworld,
            epoch: 0,
            chunks: ChunkSender::new(view_distance),
            outbox: Vec::new(),
            hands: HandQueue::default(),
            menu_inputs: Vec::new(),
            mobs: crate::mob_sender::MobSender::default(),
            lectern: None,
            book_settings: None,
            seen_recipes: Vec::new(),
            book: None,
            gear: BTreeMap::new(),
            command_level: None,
            view: None,
            spawn: None,
            cracks: BTreeMap::new(),
            boss_bar: None,
            raid_bars: BTreeMap::new(),
            wither_bars: BTreeMap::new(),
            elden_bars: BTreeMap::new(),
            credits: false,
            sleepers: BTreeMap::new(),
        }
    }

    pub fn send_sleepers(&mut self, now: BTreeMap<u32, Lying>, timer: i32) {
        let me = self.client;
        let own = |client: u32| if client == me { timer } else { 0 };
        let gone: Vec<u32> = self
            .sleepers
            .keys()
            .filter(|client| !now.contains_key(client))
            .copied()
            .collect();
        for client in gone {
            let timer = own(client);
            self.outbox.push(ToClient::Sleeping {
                client,
                bed: None,
                timer,
            });
        }
        for (&client, &lying) in &now {
            if self.sleepers.get(&client) != Some(&lying) {
                let timer = own(client);
                self.outbox.push(ToClient::Sleeping {
                    client,
                    bed: Some(lying),
                    timer,
                });
            }
        }
        self.sleepers = now;
    }

    pub fn send_boss_bar(&mut self, bar: Option<f32>) {
        let operation = match (self.boss_bar, bar) {
            (None, Some(progress)) => dragon_bar(progress),
            (Some(shown), Some(progress)) if shown != progress => {
                BossBarOperation::Progress(progress)
            }
            (Some(_), None) => BossBarOperation::Remove,
            _ => return,
        };
        self.boss_bar = bar;
        self.outbox.push(ToClient::BossBar(BossBarUpdate {
            id: DRAGON_BAR,
            operation,
        }));
    }

    pub fn send_raid_bars(&mut self, bars: &[minecraft_terrain::raids::RaidBar]) {
        let now: BTreeMap<i32, minecraft_terrain::raids::RaidBar> = bars.iter().map(|bar| (bar.id, *bar)).collect();
        for &id in self.raid_bars.keys().filter(|id| !now.contains_key(id)) {
            self.outbox.push(ToClient::BossBar(BossBarUpdate { id: raid_bar_id(id), operation: BossBarOperation::Remove }));
        }
        for (&id, bar) in &now {
            let id_sent = raid_bar_id(id);
            match self.raid_bars.get(&id) {
                None => self.outbox.push(ToClient::BossBar(BossBarUpdate { id: id_sent, operation: raid_bar(bar) })),
                Some(shown) => {
                    if shown.name != bar.name {
                        self.outbox.push(ToClient::BossBar(BossBarUpdate { id: id_sent, operation: BossBarOperation::Name(raid_bar_name(bar.name)) }));
                    }
                    if shown.progress != bar.progress {
                        self.outbox.push(ToClient::BossBar(BossBarUpdate { id: id_sent, operation: BossBarOperation::Progress(bar.progress) }));
                    }
                }
            }
        }
        self.raid_bars = now;
    }

    pub fn send_wither_bars(&mut self, bars: &[minecraft_terrain::wither_bar::WitherBar]) {
        let now: BTreeMap<u64, f32> = bars.iter().map(|bar| (bar.id, bar.progress)).collect();
        for &id in self.wither_bars.keys().filter(|id| !now.contains_key(id)) {
            self.outbox.push(ToClient::BossBar(BossBarUpdate { id: wither_bar_id(id), operation: BossBarOperation::Remove }));
        }
        for (&id, &progress) in &now {
            let operation = match self.wither_bars.get(&id) {
                None => wither_bar(progress),
                Some(&shown) if shown != progress => BossBarOperation::Progress(progress),
                Some(_) => continue,
            };
            self.outbox.push(ToClient::BossBar(BossBarUpdate { id: wither_bar_id(id), operation }));
        }
        self.wither_bars = now;
    }

    pub fn send_elden_bars(&mut self, bars: &[minecraft_terrain::elden_bar::EldenBar]) {
        let now: BTreeMap<u64, f32> = bars.iter().map(|bar| (bar.id, bar.progress)).collect();
        for &id in self.elden_bars.keys().filter(|id| !now.contains_key(id)) {
            self.outbox.push(ToClient::BossBar(BossBarUpdate { id: elden_bar_id(id), operation: BossBarOperation::Remove }));
        }
        for bar in bars {
            let operation = match self.elden_bars.get(&bar.id) {
                None => elden_bar(bar),
                Some(&shown) if shown != bar.progress => BossBarOperation::Progress(bar.progress),
                Some(_) => continue,
            };
            self.outbox.push(ToClient::BossBar(BossBarUpdate { id: elden_bar_id(bar.id), operation }));
        }
        self.elden_bars = now;
    }

    pub fn send_credits(&mut self, holding: bool) {
        if holding && !self.credits {
            self.outbox.push(ToClient::Credits);
        }
        self.credits = holding;
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
                    data: i32::try_from(state).unwrap_or(0),
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

    pub fn send_spawn(&mut self, spawn: &minecraft_terrain::level_storage::RespawnData) {
        let dimension = crate::save::spawn_dimension(spawn);
        let (x, y, z) = spawn.pos;
        let now = (dimension, [x, y, z]);
        if self.spawn != Some(now) {
            self.spawn = Some(now);
            self.outbox.push(ToClient::SpawnPosition { dimension, pos: now.1 });
        }
    }

    pub fn send_recipe_book(&mut self, player: &Player) {
        let book = &player.recipe_book;
        let key = (player.inventory.unlocked_recipes.len(), book.highlight.len(), book.settings);
        if self.book != Some(key) {
            self.book = Some(key);
            self.outbox.push(ToClient::RecipeBook(book.view(&player.inventory)));
        }
    }

    pub fn send_gear(&mut self, gears: &[(u32, GearView)]) {
        self.gear
            .retain(|client, _| gears.iter().any(|(known, _)| known == client));
        for (client, gear) in gears {
            if self.gear.get(client) != Some(gear) {
                self.gear.insert(*client, gear.clone());
                self.outbox.push(ToClient::Equipment {
                    client: *client,
                    gear: gear.clone(),
                });
            }
        }
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
    for (mob, pos, stage) in level.entities.iter().flat_map(|entities| entities.view.world.door_cracks()) {
        cracks.insert((mob_breaker(mob), pos), stage);
    }
    cracks
}

pub fn mob_breaker(mob: u64) -> u32 {
    MOB_BREAKER | (mob as u32 & (MOB_BREAKER - 1))
}

fn near(feet: [f64; 3], pos: BlockPos, range: f64) -> bool {
    let centre = [
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    ];
    (0..3).map(|k| (centre[k] - feet[k]).powi(2)).sum::<f64>() < range * range
}

#[derive(Debug, Default, PartialEq)]
pub struct LecternAnswer {
    pub actions: Vec<(BlockPos, LecternAction)>,
    pub books: Vec<Option<BookView>>,
    pub taken: Vec<BookStack>,
}

pub fn friend_lectern(
    open: &mut Option<BlockPos>,
    clicks: Vec<([i32; 3], Option<i32>)>,
    views: Vec<LecternView>,
    eye: [f64; 3],
    may_build: bool,
) -> LecternAnswer {
    let mut answer = LecternAnswer::default();
    for (pos, button) in clicks {
        let at = (pos[0], pos[1], pos[2]);
        if *open != Some(at) {
            continue;
        }
        match button {
            None => {
                *open = None;
                answer.actions.push((at, LecternAction::Close));
            }
            Some(TAKE_BOOK) if !may_build => {}
            Some(button) => answer.actions.push((at, LecternAction::Button(button))),
        }
    }
    if let Some(at) = *open
        && !minecraft_terrain::crafting::within_table_reach(eye, at)
    {
        *open = None;
        answer.actions.push((at, LecternAction::Close));
        answer.books.push(None);
    }
    for view in views {
        answer.taken.extend(view.taken);
        match view.book.filter(|_| !view.closed) {
            Some(book) => {
                *open = Some(view.pos);
                let components = book
                    .components
                    .as_deref()
                    .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok());
                let (x, y, z) = view.pos;
                answer.books.push(Some(BookView {
                    pos: [x, y, z],
                    pages: minecraft_terrain::lectern::pages_text(components.as_ref()),
                    page: u32::try_from(view.page).unwrap_or(0),
                    take: may_build,
                }));
            }
            None => {
                if !view.closed {
                    answer.actions.push((view.pos, LecternAction::Close));
                }
                if *open == Some(view.pos) {
                    *open = None;
                }
                answer.books.push(None);
            }
        }
    }
    answer
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
    write(&minecraft_terrain::fork_items::table_hash().to_be_bytes());
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
        game_time: globals.game_time(),
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

pub fn join_code_line(code: &str) -> Clientbound {
    let bracketed = Text::translate(
        "chat.square_brackets",
        vec![Text::literal(code).color(GREEN)],
    );
    system_line(Text::literal("Friends join with the code ").append(bracketed))
}

pub const RAID_BARS: u64 = 0x100;
pub const WITHER_BARS: u64 = 1 << 40;
pub const ELDEN_BARS: u64 = 1 << 41;

pub fn wither_bar_id(wither: u64) -> u64 {
    WITHER_BARS + wither
}

pub fn wither_bar(progress: f32) -> BossBarOperation {
    BossBarOperation::Add {
        name: Text::translate(minecraft_terrain::wither_bar::NAME, Vec::new()),
        progress,
        color: BossBarColor::Purple,
        overlay: BossBarOverlay::Progress,
        flags: BossBarFlags {
            darken_screen: true,
            play_music: false,
            create_world_fog: false,
        },
    }
}

pub fn elden_bar_id(boss: u64) -> u64 {
    ELDEN_BARS + boss
}

pub fn elden_bar(bar: &minecraft_terrain::elden_bar::EldenBar) -> BossBarOperation {
    BossBarOperation::Add {
        name: Text::literal(bar.name),
        progress: bar.progress,
        color: BossBarColor::Red,
        overlay: BossBarOverlay::Progress,
        flags: BossBarFlags {
            darken_screen: false,
            play_music: true,
            create_world_fog: false,
        },
    }
}

pub fn raid_bar_id(raid: i32) -> u64 {
    RAID_BARS + u64::from(raid as u32)
}

pub fn raid_bar_name(name: minecraft_terrain::raids::BarName) -> Text {
    use minecraft_terrain::raids::{BarName, DEFEAT_KEY, RAIDERS_REMAINING_KEY, RAID_KEY, VICTORY_KEY};
    match name {
        BarName::Raid => Text::translate(RAID_KEY, Vec::new()),
        BarName::RaidersRemaining(count) => Text::translate(RAID_KEY, Vec::new())
            .append(Text::literal(" - "))
            .append(Text::translate(RAIDERS_REMAINING_KEY, vec![Text::literal(count.to_string())])),
        BarName::Victory => Text::translate(VICTORY_KEY, Vec::new()),
        BarName::Defeat => Text::translate(DEFEAT_KEY, Vec::new()),
    }
}

pub fn raid_bar(bar: &minecraft_terrain::raids::RaidBar) -> BossBarOperation {
    BossBarOperation::Add {
        name: raid_bar_name(bar.name),
        progress: bar.progress,
        color: BossBarColor::Red,
        overlay: BossBarOverlay::Notched10,
        flags: BossBarFlags::default(),
    }
}

pub fn dragon_bar(progress: f32) -> BossBarOperation {
    BossBarOperation::Add {
        name: Text::translate(DRAGON_NAME, Vec::new()),
        progress,
        color: BossBarColor::Pink,
        overlay: BossBarOverlay::Progress,
        flags: BossBarFlags {
            darken_screen: false,
            play_music: true,
            create_world_fog: true,
        },
    }
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
        let Some(uuid) = server.join_friend(PlayerId(client), name, context.kit) else {
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
        session.dimension = player.dimension;
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

    pub fn follow_levels(
        &mut self,
        players: &BTreeMap<PlayerUuid, Player>,
        origins: &[(Dimension, [f64; 3])],
    ) {
        for session in self.sessions.values_mut() {
            let Some(player) = players
                .get(&session.uuid)
                .filter(|player| player.dimension != session.dimension)
            else {
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
            let Some(origin) = origins
                .iter()
                .find(|(known, _)| *known == dimension)
                .map(|(_, origin)| *origin)
            else {
                continue;
            };
            session.dimension = dimension;
            session.epoch = session.epoch.wrapping_add(1);
            session.outbox.push(ToClient::Dimension {
                epoch: session.epoch,
                dimension,
                origin,
                feet,
            });
            let center = ((feet[0].floor() as i32) >> 4, (feet[2].floor() as i32) >> 4);
            session
                .chunks
                .follow(dimension, center, &mut session.outbox);
            session.mobs.follow(Some(dimension));
            session.cracks.clear();
            session.sleepers.clear();
            diag::info!(
                World,
                "Minecraft {} (client {}) goes to {} at {feet:?}",
                session.name,
                session.client,
                dimension.dimension_type()
            );
        }
    }

    pub fn send_time(&mut self, globals: &Globals, levels: &BTreeMap<Dimension, Level>) {
        let time = time_of(globals);
        for session in self.sessions.values_mut() {
            let entities = session
                .chunks
                .level
                .and_then(|dimension| levels.get(&dimension))
                .and_then(|level| level.entities.as_ref());
            let mut time = time.clone();
            if let (ToClient::Time { game_time, .. }, Some(entities)) = (&mut time, entities) {
                *game_time = entities.game_time();
            }
            session.outbox.push(time);
            let weather = entities.map(|entities| entities.weather);
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
                .unwrap_or_else(Vec::new);
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
                    .map_or_else(BTreeMap::new, |level| level_cracks(dimension, level, players))
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
                session.send_recipe_book(player);
            }
        }
    }

    pub fn send_spawns(&mut self, globals: &Globals) {
        for session in self.sessions.values_mut() {
            session.send_spawn(&globals.spawn);
        }
    }

    pub fn send_gear(&mut self, gears: &[(u32, GearView)]) {
        for session in self.sessions.values_mut() {
            session.send_gear(gears);
        }
    }

    pub fn send_sleepers(
        &mut self,
        players: &BTreeMap<PlayerUuid, Player>,
        (host, host_client): (&PlayerUuid, u32),
    ) {
        for session in self.sessions.values_mut() {
            let Some(me) = players.get(&session.uuid) else {
                continue;
            };
            let now = players
                .iter()
                .filter(|(_, player)| player.dimension == session.dimension)
                .filter(|(_, player)| player.switching.is_none() && !player.dead)
                .filter_map(|(uuid, player)| {
                    let sleeping = player.sleeping?;
                    let client = if uuid == host { host_client } else { player.id.0 };
                    let (x, z) = sleeping.facing;
                    let feet = sleeping.feet();
                    let facing = [x, z];
                    Some((client, Lying { feet, facing }))
                })
                .collect();
            session.send_sleepers(now, me.sleep_timer);
        }
    }

    pub fn send_boss_bars(
        &mut self,
        players: &BTreeMap<PlayerUuid, Player>,
        fight: Option<&DragonFight>,
    ) {
        for session in self.sessions.values_mut() {
            let bar = players.get(&session.uuid).and_then(|player| {
                crate::the_end::dragon_fight::shown_bar(fight, player)
            });
            session.send_boss_bar(bar);
        }
    }

    pub fn send_raid_bars(&mut self, players: &BTreeMap<PlayerUuid, Player>) {
        for session in self.sessions.values_mut() {
            let bars = players.get(&session.uuid).map_or(&[][..], |player| player.raid_bars.as_slice());
            session.send_raid_bars(bars);
        }
    }

    pub fn send_wither_bars(&mut self, players: &BTreeMap<PlayerUuid, Player>) {
        for session in self.sessions.values_mut() {
            let bars = players.get(&session.uuid).map_or(&[][..], |player| player.wither_bars.as_slice());
            session.send_wither_bars(bars);
        }
    }

    pub fn send_elden_bars(&mut self, players: &BTreeMap<PlayerUuid, Player>) {
        for session in self.sessions.values_mut() {
            let bars = players.get(&session.uuid).map_or(&[][..], |player| player.elden_bars.as_slice());
            session.send_elden_bars(bars);
        }
    }

    pub fn send_credits(&mut self, players: &BTreeMap<PlayerUuid, Player>) {
        for session in self.sessions.values_mut() {
            let holding = players
                .get(&session.uuid)
                .is_some_and(crate::the_end::holding_for_credits);
            session.send_credits(holding);
        }
    }

    pub fn credits_done(
        &mut self,
        players: &mut BTreeMap<PlayerUuid, Player>,
        client: u32,
    ) -> bool {
        let Some(session) = self.sessions.get(&client) else {
            return false;
        };
        let Some(player) = players
            .get_mut(&session.uuid)
            .filter(|player| crate::the_end::holding_for_credits(player))
        else {
            return false;
        };
        crate::the_end::credits_done(player);
        diag::info!(
            World,
            "Minecraft credits closed for {} (client {client})",
            session.name
        );
        true
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

    pub fn send_mobs(
        &mut self,
        players: &BTreeMap<PlayerUuid, Player>,
        levels: &mut BTreeMap<Dimension, Level>,
        view_distance: i32,
    ) -> Vec<(u32, StateFrame)> {
        let mut frames = Vec::new();
        for (&dimension, level) in levels.iter_mut() {
            let Some(entities) = level.entities.as_mut() else {
                continue;
            };
            let tracked = entities.take_tracked();
            let mut snapshot = entities.take_friend_snapshot();
            let bursts = snapshot.as_mut().map(crate::mob_sender::take_bursts);
            for session in self.sessions.values_mut() {
                session.mobs.follow(session.chunks.level);
                if session.chunks.level != Some(dimension) {
                    continue;
                }
                let Some(player) = players.get(&session.uuid) else {
                    continue;
                };
                let id = u64::from(player.id.0);
                let moves = tracked.moves_of(id);
                let chunks = &session.chunks;
                session.mobs.events(
                    tracked.events_of(id),
                    moves,
                    |pos| chunks.has(pos),
                    &mut session.outbox,
                );
                let Some(feet) = player
                    .arrival
                    .or(player.place)
                    .filter(|place| place.dimension == dimension)
                    .map(|place| glam::DVec3::from_array(place.feet))
                else {
                    continue;
                };
                let (around, has) = ((feet, view_distance), |pos| chunks.has(pos));
                let near = bursts.as_ref().and_then(|bursts| {
                    crate::mob_sender::bursts_message(bursts, around, has)
                });
                session.outbox.extend(near);
                let epoch = session.epoch;
                let frame = session.mobs.frame(epoch, moves, snapshot.as_ref(), around, has);
                if let Some(frame) = frame {
                    frames.push((session.client, frame));
                }
            }
        }
        frames
    }

    pub fn send_heard(
        &mut self,
        players: &BTreeMap<PlayerUuid, Player>,
        levels: &mut BTreeMap<Dimension, Level>,
    ) {
        for (&dimension, level) in levels.iter_mut() {
            let Some(entities) = level.entities.as_mut() else {
                continue;
            };
            let watched = self
                .sessions
                .values()
                .any(|session| session.chunks.level == Some(dimension));
            let said = entities.heard.listen(watched);
            self.tell(dimension, &said, |uuid| {
                let player = players.get(uuid)?;
                let place = player.arrival.or(player.place)?;
                (place.dimension == dimension).then_some(place.feet)
            });
            let spawners = std::mem::take(&mut entities.spawner_tags);
            self.send_spawners(dimension, &spawners);
        }
    }

    pub fn send_spawners(
        &mut self,
        dimension: Dimension,
        spawners: &[((i32, i32, i32), minecraftoss_core::nbt::Tag)],
    ) {
        for session in self.sessions.values_mut() {
            if session.chunks.level != Some(dimension) {
                continue;
            }
            for (pos, tag) in spawners {
                let (x, y, z) = *pos;
                if session.chunks.has((x >> 4, z >> 4)) {
                    let tag = Nbt(minecraftoss_core::nbt::write(tag, ""));
                    let pos = [x, y, z];
                    session.outbox.push(ToClient::BlockEntity { pos, tag });
                }
            }
        }
    }

    pub fn tell(
        &mut self,
        dimension: Dimension,
        said: &[crate::heard::Said],
        feet_of: impl Fn(&PlayerUuid) -> Option<[f64; 3]>,
    ) {
        for session in self.sessions.values_mut() {
            if session.chunks.level != Some(dimension) {
                continue;
            }
            let Some(feet) = feet_of(&session.uuid) else {
                continue;
            };
            let (client, view_distance) = (session.client, session.chunks.view_distance);
            session.outbox.extend(
                said.iter()
                    .filter(|said| said.reaches(client, feet, view_distance))
                    .map(|said| said.heard.message()),
            );
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

    #[test]
    fn the_join_code_line_shows_the_code_in_green_brackets_like_the_seed() {
        let Clientbound::System { text, overlay } =
            join_code_line("00ff00ff00ff00ff00ff00ff00ff00ff")
        else {
            panic!("the join code is a system line");
        };
        assert!(!overlay);
        let lang = |key: &str| (key == "chat.square_brackets").then(|| "[%s]".to_owned());
        let resolved = text.resolve(&lang);
        assert_eq!(
            resolved.text,
            "Friends join with the code [00ff00ff00ff00ff00ff00ff00ff00ff]"
        );
        let start = resolved.text.find('[').unwrap() + 1;
        assert_eq!(resolved.spans, vec![(start..start + 32, GREEN)]);
    }

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
        session.dimension = level;
        session.chunks.follow(level, (0, 0), &mut session.outbox);
        session
            .chunks
            .next_batch(|pos| pos == (0, 0), &mut session.outbox)
            .unwrap();
        session.outbox.clear();
        session
    }

    #[test]
    fn what_is_heard_reaches_friends_in_range_in_that_level_and_never_across_levels() {
        use crate::heard::{Heard, Said};
        let mut sessions = Sessions::default();
        let levels = [
            (1, Dimension::Overworld),
            (2, Dimension::Overworld),
            (3, Dimension::Nether),
            (4, Dimension::Overworld),
        ];
        for (client, level) in levels {
            sessions
                .sessions
                .insert(client, session_with(client, level));
        }
        let chest = Heard::sound("minecraft:block.chest.open", [0.5, 64.5, 0.5], 0.5, 1.0);
        let fizz = Heard::Event {
            event: 1501,
            pos: [40, 64, 0],
            data: 0,
        };
        let dig = Heard::sound("minecraft:block.stone.hit", [0.5, 64.5, 0.5], 0.25, 0.5);
        let said = vec![
            Said::all(chest.clone()),
            Said::all(fizz.clone()),
            Said::to(2, dig.clone()),
        ];
        let feet = |uuid: &PlayerUuid| match uuid.0.as_str() {
            "friend-1" => Some([-30.0, 64.0, 0.0]),
            "friend-2" => Some([10.0, 64.0, 0.0]),
            "friend-3" => Some([0.0, 64.0, 0.0]),
            _ => None,
        };
        sessions.tell(Dimension::Overworld, &said, feet);
        let heard = |client| sessions.sessions[&client].outbox.clone();
        assert!(
            heard(1).is_empty(),
            "30 blocks from the chest, 70 from the fizz"
        );
        assert_eq!(
            heard(2),
            vec![chest.message(), fizz.message(), dig.message()]
        );
        assert!(heard(3).is_empty(), "another level hears nothing");
        assert!(
            heard(4).is_empty(),
            "a friend with no place yet hears nothing"
        );
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
    fn a_spawners_new_data_reaches_friends_in_that_level_with_that_chunk() {
        use minecraftoss_core::nbt::Tag;
        let mut sessions = Sessions::default();
        sessions
            .sessions
            .insert(1, session_with(1, Dimension::Overworld));
        sessions
            .sessions
            .insert(2, session_with(2, Dimension::Nether));
        let tag = Tag::Compound(BTreeMap::from([(
            "id".to_owned(),
            Tag::String("minecraft:mob_spawner".to_owned()),
        )]));
        let spawners = vec![((2, 30, 3), tag.clone()), ((40, 30, 40), tag.clone())];
        sessions.send_spawners(Dimension::Overworld, &spawners);
        assert_eq!(
            sessions.sessions[&1].outbox,
            vec![ToClient::BlockEntity {
                pos: [2, 30, 3],
                tag: Nbt(minecraftoss_core::nbt::write(&tag, "")),
            }],
            "the spawner in a chunk the friend has, not the one outside"
        );
        assert!(sessions.sessions[&2].outbox.is_empty());
    }

    #[test]
    fn friends_learn_who_lies_in_a_bed_and_their_own_wake_up_timer() {
        let mut session = session_with(1, Dimension::Overworld);
        let lying = Lying {
            feet: [0.5, 64.6875, 0.5],
            facing: [0, 1],
        };
        let sleeping = |client, bed, timer| ToClient::Sleeping { client, bed, timer };
        let both = BTreeMap::from([(1, lying), (5, lying)]);
        session.send_sleepers(both.clone(), 0);
        assert_eq!(
            std::mem::take(&mut session.outbox),
            vec![sleeping(1, Some(lying), 0), sleeping(5, Some(lying), 0)]
        );
        session.send_sleepers(both, 40);
        assert!(session.outbox.is_empty(), "nothing changed");
        session.send_sleepers(BTreeMap::from([(5, lying)]), 100);
        assert_eq!(
            std::mem::take(&mut session.outbox),
            vec![sleeping(1, None, 100)],
            "the friend's own wake-up fades from deep sleep"
        );
        session.send_sleepers(BTreeMap::new(), 7);
        assert_eq!(
            std::mem::take(&mut session.outbox),
            vec![sleeping(5, None, 0)],
            "another player's timer is not theirs"
        );
    }

    #[test]
    fn a_friends_lectern_turns_pages_closes_and_gives_the_book_only_to_a_builder() {
        use minecraft_terrain::lectern::NEXT_PAGE;
        let at = (10, 64, 10);
        let eye = [10.5, 65.6, 12.0];
        let book = BookStack {
            id: "minecraft:written_book".to_owned(),
            components: None,
        };
        let view = |page: i32, closed: bool, taken: Option<BookStack>| LecternView {
            player: 1,
            pos: at,
            book: (!closed).then(|| book.clone()),
            page,
            taken,
            closed,
        };
        let shown = |page| BookView {
            pos: [10, 64, 10],
            pages: Vec::new(),
            page,
            take: true,
        };
        let mut open = None;
        let next = vec![([10, 64, 10], Some(NEXT_PAGE))];
        let opened = friend_lectern(&mut open, next.clone(), vec![view(0, false, None)], eye, true);
        assert!(opened.actions.is_empty(), "a click before the screen opened is dropped");
        assert_eq!((open, opened.books), (Some(at), vec![Some(shown(0))]));
        let turned = friend_lectern(&mut open, next, vec![view(1, false, None)], eye, true);
        assert_eq!(turned.actions, vec![(at, LecternAction::Button(NEXT_PAGE))]);
        assert_eq!(turned.books, vec![Some(shown(1))]);
        let take = vec![([10, 64, 10], Some(TAKE_BOOK))];
        let refused = friend_lectern(&mut open, take.clone(), Vec::new(), eye, false);
        assert!(refused.actions.is_empty(), "a player who may not build keeps their hands off");
        let closing = vec![view(0, true, Some(book.clone()))];
        let taken = friend_lectern(&mut open, take, closing, eye, true);
        assert_eq!(taken.actions, vec![(at, LecternAction::Button(TAKE_BOOK))]);
        assert_eq!((taken.taken, taken.books), (vec![book.clone()], vec![None]));
        assert_eq!(open, None);
        friend_lectern(&mut open, Vec::new(), vec![view(2, false, None)], eye, true);
        let far = friend_lectern(&mut open, Vec::new(), Vec::new(), [30.0, 65.0, 30.0], true);
        assert_eq!((far.actions, far.books), (vec![(at, LecternAction::Close)], vec![None]));
        assert_eq!(open, None, "walking away closes it");
        friend_lectern(&mut open, Vec::new(), vec![view(2, false, None)], eye, true);
        let close = vec![([10, 64, 10], None)];
        let closed = friend_lectern(&mut open, close, Vec::new(), eye, true);
        assert_eq!(closed.actions, vec![(at, LecternAction::Close)]);
        assert!(closed.books.is_empty() && open.is_none());
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
    fn a_friends_recipe_book_is_sent_once_for_each_change() {
        let mut player = alex();
        let mut session = Session::new(1, player.uuid.clone(), "Alex", 2);
        let books = |session: &mut Session| {
            std::mem::take(&mut session.outbox)
                .into_iter()
                .filter_map(|message| match message {
                    ToClient::RecipeBook(book) => Some(book),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        session.send_recipe_book(&player);
        session.send_recipe_book(&player);
        assert_eq!(books(&mut session).len(), 1);
        player.recipe_book.set_crafting(true, true);
        player.inventory.unlocked_recipes.insert("minecraft:stick".to_owned());
        player.inventory.unlocked_recipes.insert("iw4l:m4".to_owned());
        player.recipe_book.update(&player.inventory);
        session.send_recipe_book(&player);
        session.send_recipe_book(&player);
        let sent = books(&mut session);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].recipes, ["minecraft:stick"]);
        assert_eq!(sent[0].highlight, ["minecraft:stick"]);
        assert!(sent[0].open && sent[0].filtering);
        player.recipe_book.seen(&["minecraft:stick".to_owned()]);
        session.send_recipe_book(&player);
        assert!(books(&mut session)[0].highlight.is_empty());
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

    #[test]
    fn gear_is_sent_once_for_each_change() {
        let mut sessions = Sessions::default();
        for (client, name) in [(1, "Alex"), (2, "Bo")] {
            let uuid = PlayerUuid(name.to_owned());
            sessions
                .sessions
                .insert(client, Session::new(client, uuid, name, 2));
        }
        let sword = GearView {
            main: Some(minecraft_net::messages::GearItem {
                id: "minecraft:iron_sword".to_owned(),
                glint: false,
                dye: None,
                potion: None,
            }),
            hides_gun: true,
            ..GearView::default()
        };
        let sent = |sessions: &mut Sessions| {
            let mut sent: Vec<(u32, u32, bool)> = sessions
                .take_outgoing()
                .into_iter()
                .filter_map(|(to, message)| match message {
                    ToClient::Equipment { client, gear } => Some((to, client, gear.hides_gun)),
                    _ => None,
                })
                .collect();
            sent.sort_unstable();
            sent
        };
        let mut gears = vec![(0, GearView::default()), (2, sword.clone())];
        sessions.send_gear(&gears);
        assert_eq!(
            sent(&mut sessions),
            vec![(1, 0, false), (1, 2, true), (2, 0, false), (2, 2, true)]
        );
        sessions.send_gear(&gears);
        assert!(sent(&mut sessions).is_empty(), "nothing changed");
        gears[0].1 = sword.clone();
        sessions.send_gear(&gears);
        assert_eq!(sent(&mut sessions), vec![(1, 0, true), (2, 0, true)]);
        sessions.send_gear(&gears[..1]);
        assert!(sent(&mut sessions).is_empty(), "a player who left");
        sessions.send_gear(&gears);
        assert_eq!(
            sent(&mut sessions),
            vec![(1, 2, true), (2, 2, true)],
            "a player who came back is sent again"
        );
    }

    fn named(client: u32, name: &str, dimension: Dimension) -> Player {
        let mut chat = CommandServer::new(HOST);
        chat.join(PlayerId(client), name);
        let mut player = Player::new(
            chat.profile(PlayerId(client)).unwrap(),
            minecraft_terrain::game_mode::PlayerMode::default(),
            crate::survival::Survival::default(),
            minecraftoss_player::inventory::Inventory::default(),
        );
        player.dimension = dimension;
        player
    }

    fn sessions_of(players: &BTreeMap<PlayerUuid, Player>) -> Sessions {
        let mut sessions = Sessions::default();
        for player in players.values() {
            let (client, uuid) = (player.id.0, player.uuid.clone());
            let session = Session::new(client, uuid, &player.name, 2);
            sessions.sessions.insert(client, session);
        }
        sessions
    }

    #[test]
    fn a_friend_gets_a_purple_darkening_bar_for_each_wither_it_tracks() {
        use minecraft_terrain::wither_bar::WitherBar;
        let mut session = Session::new(2, PlayerUuid("f".to_owned()), "Alex", 2);
        let sent = |session: &mut Session| -> Vec<BossBarUpdate> {
            std::mem::take(&mut session.outbox)
                .into_iter()
                .filter_map(|message| match message {
                    ToClient::BossBar(update) => Some(update),
                    _ => None,
                })
                .collect()
        };
        session.send_wither_bars(&[WitherBar { id: 7, progress: 0.0 }]);
        let added = sent(&mut session);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].id, WITHER_BARS + 7);
        let BossBarOperation::Add { name, progress, color, overlay, flags } = &added[0].operation else {
            panic!("{added:?}");
        };
        assert_eq!((*progress, *color, *overlay), (0.0, BossBarColor::Purple, BossBarOverlay::Progress));
        assert!(flags.darken_screen && !flags.play_music && !flags.create_world_fog);
        assert_eq!(*name, Text::translate("entity.minecraft.wither", Vec::new()));
        session.send_wither_bars(&[WitherBar { id: 7, progress: 0.0 }]);
        assert!(sent(&mut session).is_empty(), "nothing changed");
        session.send_wither_bars(&[WitherBar { id: 7, progress: 0.4 }, WitherBar { id: 8, progress: 1.0 }]);
        let changed = sent(&mut session);
        assert_eq!((changed[0].id, changed[0].operation.clone()), (WITHER_BARS + 7, BossBarOperation::Progress(0.4)));
        assert!(matches!(changed[1].operation, BossBarOperation::Add { progress, .. } if progress == 1.0));
        session.send_wither_bars(&[WitherBar { id: 8, progress: 1.0 }]);
        let removed = sent(&mut session);
        assert_eq!((removed.len(), removed[0].id, removed[0].operation.clone()), (1, WITHER_BARS + 7, BossBarOperation::Remove));
        assert!(wither_bar_id(u64::from(u32::MAX)) > raid_bar_id(i32::MAX) && raid_bar_id(i32::MIN) > DRAGON_BAR);
    }

    #[test]
    fn the_elden_bar_is_red_with_his_name_and_asks_for_music() {
        use minecraft_terrain::elden_bar::EldenBar;
        let mut session = Session::new(2, PlayerUuid("f".to_owned()), "Alex", 2);
        let sent = |session: &mut Session| -> Vec<BossBarUpdate> {
            std::mem::take(&mut session.outbox)
                .into_iter()
                .filter_map(|message| match message {
                    ToClient::BossBar(update) => Some(update),
                    _ => None,
                })
                .collect()
        };
        let margit = |progress| EldenBar { id: 7, name: "Margit, the Fell Omen", progress };
        session.send_elden_bars(&[margit(1.0)]);
        let added = sent(&mut session);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].id, ELDEN_BARS + 7);
        let BossBarOperation::Add { name, progress, color, overlay, flags } = &added[0].operation else {
            panic!("{added:?}");
        };
        assert_eq!((*progress, *color, *overlay), (1.0, BossBarColor::Red, BossBarOverlay::Progress));
        assert!(flags.play_music && !flags.darken_screen && !flags.create_world_fog);
        assert_eq!(*name, Text::literal("Margit, the Fell Omen"));
        session.send_elden_bars(&[margit(1.0)]);
        assert!(sent(&mut session).is_empty(), "nothing changed");
        session.send_elden_bars(&[margit(0.5)]);
        let changed = sent(&mut session);
        assert_eq!((changed.len(), changed[0].operation.clone()), (1, BossBarOperation::Progress(0.5)));
        session.send_elden_bars(&[]);
        let removed = sent(&mut session);
        assert_eq!((removed.len(), removed[0].id, removed[0].operation.clone()), (1, ELDEN_BARS + 7, BossBarOperation::Remove));
    }

    #[test]
    fn margits_bar_ids_never_meet_wither_raid_or_dragon_ids() {
        let entity = u64::from(u32::MAX);
        assert_eq!(elden_bar_id(7), (1 << 41) + 7);
        assert!(elden_bar_id(0) > wither_bar_id(entity), "any entity id below 2^40 keeps them apart");
        assert!(wither_bar_id(0) > raid_bar_id(i32::MAX) && raid_bar_id(i32::MIN) > DRAGON_BAR);
        assert!(elden_bar_id(0) > DRAGON_BAR && elden_bar_id(0) > raid_bar_id(i32::MAX));
    }

    #[test]
    fn a_friend_gets_the_raid_bar_red_and_notched_with_its_name_and_progress() {
        use minecraft_terrain::raids::{BarName, RaidBar};
        let mut session = Session::new(2, PlayerUuid("f".to_owned()), "Alex", 2);
        let bar = |name, progress| RaidBar { id: 3, name, progress };
        let sent = |session: &mut Session| -> Vec<BossBarUpdate> {
            std::mem::take(&mut session.outbox)
                .into_iter()
                .filter_map(|message| match message {
                    ToClient::BossBar(update) => Some(update),
                    _ => None,
                })
                .collect()
        };
        session.send_raid_bars(&[bar(BarName::Raid, 0.25)]);
        let added = sent(&mut session);
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].id, RAID_BARS + 3);
        let BossBarOperation::Add { name, progress, color, overlay, flags } = &added[0].operation else {
            panic!("{added:?}");
        };
        assert_eq!((*progress, *color, *overlay, *flags), (0.25, BossBarColor::Red, BossBarOverlay::Notched10, BossBarFlags::default()));
        assert_eq!(*name, Text::translate(minecraft_terrain::raids::RAID_KEY, Vec::new()));
        session.send_raid_bars(&[bar(BarName::Raid, 0.25)]);
        assert!(sent(&mut session).is_empty(), "nothing changed");
        session.send_raid_bars(&[bar(BarName::RaidersRemaining(2), 0.1)]);
        let changed: Vec<BossBarOperation> = sent(&mut session).into_iter().map(|update| update.operation).collect();
        assert_eq!(changed, [BossBarOperation::Name(raid_bar_name(BarName::RaidersRemaining(2))), BossBarOperation::Progress(0.1)]);
        session.send_raid_bars(&[]);
        let removed = sent(&mut session);
        assert_eq!((removed[0].id, removed[0].operation.clone()), (RAID_BARS + 3, BossBarOperation::Remove));
        let resolved = raid_bar_name(BarName::RaidersRemaining(2)).resolve(&|key: &str| match key {
            "event.minecraft.raid" => Some("Raid".to_owned()),
            "event.minecraft.raid.raiders_remaining" => Some("Raiders Remaining: %s".to_owned()),
            _ => None,
        });
        assert_eq!(resolved.text, "Raid - Raiders Remaining: 2");
    }

    fn bars(sessions: &mut Sessions) -> Vec<(u32, BossBarOperation)> {
        sessions
            .take_outgoing()
            .into_iter()
            .filter_map(|(client, message)| match message {
                ToClient::BossBar(update) if update.id == DRAGON_BAR => {
                    Some((client, update.operation))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_dragon_bar_reaches_only_the_players_near_the_fight_and_follows_its_health() {
        let near = named(1, "Alex", Dimension::End);
        let far = named(2, "Bo", Dimension::End);
        let home = named(3, "Cy", Dimension::Overworld);
        let mut fight = DragonFight {
            players: vec![near.uuid.clone(), home.uuid.clone()],
            ..Default::default()
        };
        let mut players: BTreeMap<PlayerUuid, Player> = [near, far, home]
            .into_iter()
            .map(|player| (player.uuid.clone(), player))
            .collect();
        let mut sessions = sessions_of(&players);
        sessions.send_boss_bars(&players, Some(&fight));
        assert_eq!(bars(&mut sessions), vec![(1, dragon_bar(1.0))]);
        sessions.send_boss_bars(&players, Some(&fight));
        assert!(bars(&mut sessions).is_empty(), "nothing changed");
        fight.update_dragon(150.0);
        sessions.send_boss_bars(&players, Some(&fight));
        sessions.send_boss_bars(&players, Some(&fight));
        assert_eq!(
            bars(&mut sessions),
            vec![(1, BossBarOperation::Progress(0.75))]
        );
        let alex = players.values_mut().find(|player| player.id.0 == 1).unwrap();
        alex.switch_to(Dimension::Overworld, None, crate::switch::LoadingReason::EndPortal);
        sessions.send_boss_bars(&players, Some(&fight));
        assert_eq!(bars(&mut sessions), vec![(1, BossBarOperation::Remove)]);
        sessions.send_boss_bars(&players, None);
        assert!(bars(&mut sessions).is_empty(), "removed once");
        let BossBarOperation::Add { name, flags, .. } = dragon_bar(1.0) else {
            panic!("the dragon bar is added");
        };
        assert_eq!(name, Text::translate(DRAGON_NAME, Vec::new()));
        assert!(flags.play_music && flags.create_world_fog && !flags.darken_screen);
    }

    #[test]
    fn the_credits_roll_once_per_player_and_credits_done_releases_only_that_player() {
        let mut players: BTreeMap<PlayerUuid, Player> = [
            named(1, "Alex", Dimension::Overworld),
            named(2, "Bo", Dimension::Overworld),
            named(3, "Cy", Dimension::Overworld),
        ]
        .into_iter()
        .map(|player| (player.uuid.clone(), player))
        .collect();
        let mut sessions = sessions_of(&players);
        let credits = |sessions: &mut Sessions| {
            sessions
                .take_outgoing()
                .into_iter()
                .filter(|(_, message)| *message == ToClient::Credits)
                .map(|(client, _)| client)
                .collect::<Vec<_>>()
        };
        for player in players.values_mut().filter(|player| player.id.0 != 3) {
            assert!(crate::the_end::show_credits(player));
        }
        sessions.send_credits(&players);
        sessions.send_credits(&players);
        assert_eq!(credits(&mut sessions), vec![1, 2], "once each");
        assert!(!sessions.credits_done(&mut players, 3), "Cy has no credits to close");
        assert!(!sessions.credits_done(&mut players, 9), "not a session");
        assert!(sessions.credits_done(&mut players, 1));
        assert!(!sessions.credits_done(&mut players, 1), "closed once");
        let holding = |players: &BTreeMap<PlayerUuid, Player>, client: u32| {
            players
                .values()
                .find(|player| player.id.0 == client)
                .is_some_and(crate::the_end::holding_for_credits)
        };
        assert!(!holding(&players, 1));
        assert!(holding(&players, 2), "Bo still watches");
        sessions.send_credits(&players);
        assert!(credits(&mut sessions).is_empty());
        let alex = players.values_mut().find(|player| player.id.0 == 1).unwrap();
        assert!(!crate::the_end::show_credits(alex), "seen once, never again");
        sessions.send_credits(&players);
        assert!(credits(&mut sessions).is_empty());
    }

    fn host_server(name: &str) -> Option<(TestDir, Server)> {
        let registries = registries()?;
        let (dir, save) = world(name);
        let mut server = Server {
            globals: test_globals(registries, Some(save)),
            levels: BTreeMap::new(),
            opening: BTreeMap::new(),
            players: BTreeMap::new(),
            published: false,
        };
        let uuid = server.join_friend(HOST, "Steve", None).unwrap();
        server.players.get_mut(&uuid).unwrap().arrival = None;
        Some((dir, server))
    }

    fn context(origins: &[(Dimension, [f64; 3])]) -> JoinContext<'_> {
        JoinContext {
            registry: 42,
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
    fn each_friend_gets_the_world_spawn_after_its_login_and_again_when_it_moves() {
        let Some((_dir, mut server)) = host_server("session-spawn") else {
            return;
        };
        let origins = [(Dimension::Overworld, [0.5, 64.0, 0.5])];
        let mut sessions = Sessions::default();
        let now = Instant::now();
        sessions.hello(&mut server, 1, "Alex", HELLO, &context(&origins), now);
        server.globals.spawn = crate::save::world_spawn(Dimension::Overworld, (12, 70, -4));
        sessions.send_spawns(&server.globals);
        let spawns = |sessions: &mut Sessions| -> Vec<(u32, ToClient)> {
            sessions.take_outgoing().into_iter().filter(|(_, message)| matches!(message, ToClient::SpawnPosition { .. })).collect()
        };
        let sent = spawns(&mut sessions);
        assert_eq!(sent, [(1, ToClient::SpawnPosition { dimension: Dimension::Overworld, pos: [12, 70, -4] })]);
        sessions.send_spawns(&server.globals);
        assert!(spawns(&mut sessions).is_empty(), "sent once");
        sessions.hello(&mut server, 2, "Bob", HELLO, &context(&origins), now);
        let out = sessions.take_outgoing();
        assert!(matches!(out.iter().find(|(client, _)| *client == 2), Some((_, ToClient::Login(_)))), "the login comes first");
        sessions.send_spawns(&server.globals);
        assert_eq!(spawns(&mut sessions), [(2, ToClient::SpawnPosition { dimension: Dimension::Overworld, pos: [12, 70, -4] })]);
        server.globals.spawn = crate::save::world_spawn(Dimension::Overworld, (0, 64, 0));
        sessions.send_spawns(&server.globals);
        let moved = ToClient::SpawnPosition { dimension: Dimension::Overworld, pos: [0, 64, 0] };
        assert_eq!(spawns(&mut sessions), [(1, moved.clone()), (2, moved)], "/setworldspawn reaches everyone");
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

    #[test]
    fn a_friend_leaving_during_the_credits_is_released_and_saved_as_having_seen_them() {
        let Some((_dir, mut server)) = host_server("session-credits-leave") else {
            return;
        };
        let origins = [];
        let mut sessions = Sessions::default();
        let now = Instant::now();
        sessions.hello(&mut server, 1, "Alex", HELLO, &context(&origins), now);
        sessions.hello(&mut server, 2, "Bob", HELLO, &context(&origins), now);
        for player in server.players.values_mut() {
            crate::the_end::show_credits(player);
        }
        sessions.send_credits(&server.players);
        sessions.take_outgoing();
        let left = sessions.leave(&mut server, 1).expect("Alex was in the world");
        assert!(!crate::the_end::holding_for_credits(&left.player));
        assert_eq!(left.player.data().seen_credits, Some(true));
        assert!(!sessions.credits_done(&mut server.players, 1), "no session any more");
        let bob = server
            .players
            .values()
            .find(|player| player.id.0 == 2)
            .unwrap();
        assert!(crate::the_end::holding_for_credits(bob), "Bob still watches");
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
    fn a_friends_portal_arrival_sends_dimension_with_the_next_epoch_and_starts_the_chunks_again() {
        let Some((_dir, mut server)) = host_server("session-travel") else {
            return;
        };
        let origins = [
            (Dimension::Overworld, [0.5, 64.0, 0.5]),
            (Dimension::Nether, [0.0, 1314.0, 0.0]),
        ];
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
        let session = sessions.sessions.get_mut(&1).unwrap();
        assert_eq!((session.dimension, session.epoch), (Dimension::Overworld, 0));
        session
            .chunks
            .follow(Dimension::Overworld, (0, 0), &mut session.outbox);
        session
            .chunks
            .next_batch(|pos| pos == (0, 0), &mut session.outbox)
            .unwrap();
        session.outbox.clear();
        let uuid = session.uuid.clone();
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
        for player in server.players.values_mut() {
            crate::portal::set_off(player, travel);
        }
        assert!(
            server
                .players
                .values()
                .all(|player| player.dimension == Dimension::Nether && player.switching.is_some()),
            "travel is no longer refused while a friend is online"
        );
        sessions.follow_levels(&server.players, &origins);
        assert!(
            sessions.take_outgoing().is_empty(),
            "nothing until the arrival is known"
        );
        let arrival = crate::player_nbt::Place::standing(Dimension::Nether, [3.5, 70.0, -20.5], 0.0, 0.0);
        server.players.get_mut(&uuid).unwrap().arrival = Some(arrival);
        sessions.follow_levels(&server.players, &origins);
        let sent = sessions.take_outgoing();
        assert_eq!(
            sent.first(),
            Some(&(
                1,
                ToClient::Dimension {
                    epoch: 1,
                    dimension: Dimension::Nether,
                    origin: [0.0, 1314.0, 0.0],
                    feet: [3.5, 70.0, -20.5],
                }
            ))
        );
        assert!(
            sent.iter().all(|(_, message)| !matches!(message, ToClient::ChunkForget { .. })),
            "the new level needs no forgets"
        );
        let session = &sessions.sessions[&1];
        assert_eq!((session.dimension, session.epoch), (Dimension::Nether, 1));
        assert_eq!(session.chunks.level, Some(Dimension::Nether));
        assert!(!session.chunks.has((0, 0)), "the sent set starts again");
        assert_eq!(session.chunks.in_flight(), 0);
        sessions.follow_levels(&server.players, &origins);
        assert!(sessions.take_outgoing().is_empty(), "told once");
        let alex = server.players.get_mut(&uuid).unwrap();
        alex.arrive(crate::player_nbt::Place::standing(Dimension::Overworld, [8.5, 64.0, 8.5], 0.0, 0.0));
        sessions.follow_levels(&server.players, &origins);
        assert!(matches!(
            sessions.take_outgoing().first(),
            Some((1, ToClient::Dimension { epoch: 2, dimension: Dimension::Overworld, .. }))
        ));
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
            game_time: server.globals.game_time(),
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

    #[test]
    fn a_friend_sees_a_zombies_door_crack_grow_and_clear() {
        let Some(mut level) =
            crate::commands::tests::test_stream().and_then(crate::commands::tests::test_level)
        else {
            return;
        };
        let mut entities = crate::entities::Entities::new(
            &level.terrain.stream,
            Dimension::Overworld,
            0,
            &crate::data::GameData::default(),
            None,
            None,
        );
        let feet = glam::DVec3::new(1.5, 64.0, 0.5);
        let zombie = minecraftoss_entities::zombie::Zombie::new(feet);
        let id = entities.view.world.spawn_zombie_active(zombie, 0.0);
        level.entities = Some(entities);
        let door = (1, 65, 1);
        let mut session = session_with(1, Dimension::Overworld);
        let mut crack = |level: &mut Level, stage: Option<u8>| {
            let world = &mut level.entities.as_mut().unwrap().view.world;
            let ai = world.zombie_mut(id).unwrap().ai.as_deref_mut().unwrap();
            ai.state.door.crack = stage.map(|stage| (door, stage));
            let cracks = level_cracks(Dimension::Overworld, level, &BTreeMap::new());
            session.send_cracks(cracks);
        };
        crack(&mut level, Some(0));
        crack(&mut level, Some(0));
        crack(&mut level, Some(5));
        crack(&mut level, None);
        let breaker = mob_breaker(id);
        assert!(breaker != BULLETS && breaker >= MOB_BREAKER);
        let shown = |stage| ToClient::Crack {
            breaker,
            pos: [1, 65, 1],
            stage,
        };
        assert_eq!(session.outbox, vec![shown(0), shown(5), shown(-1)]);
    }
}
