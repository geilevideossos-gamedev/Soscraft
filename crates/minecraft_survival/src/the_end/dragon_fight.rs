use std::any::Any;
use std::collections::BTreeMap;
use std::sync::mpsc;

use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::server::{DragonFightScan, Job, ServerSim};
use minecraftoss_core::ChunkPos;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::LegacyRandom;

use crate::entities::Entities;
use crate::level::Level;
use crate::player::Player;
use crate::save::WorldSave;

pub const SAVED_DATA: &str = "ender_dragon_fight";
pub const FIGHT_ORIGIN: (i32, i32, i32) = (0, 0, 0);
pub const DRAGON_SPAWN_Y: i32 = 128;
pub const PLAYER_RANGE: f64 = 192.0;
pub const ARENA_SIZE_CHUNKS: i32 = 8;
pub const ARENA_TICKET_LEVEL: i32 = 9;
pub const GATEWAY_COUNT: i32 = 20;
pub const TIME_BETWEEN_PLAYER_SCANS: u32 = 20;
pub const TICK_SECONDS: f64 = 0.05;

#[derive(Clone, Debug, PartialEq)]
pub struct FightData {
    pub needs_state_scanning: bool,
    pub dragon_killed: bool,
    pub previously_killed: bool,
    pub respawn_stage: Option<String>,
    pub respawn_time: i32,
    pub dragon_uuid: Option<[i32; 4]>,
    pub exit_portal_location: Option<(i32, i32, i32)>,
    pub gateways: Vec<i32>,
    pub respawn_crystals: Vec<Tag>,
}

impl Default for FightData {
    fn default() -> Self {
        Self {
            needs_state_scanning: true,
            dragon_killed: false,
            previously_killed: false,
            respawn_stage: None,
            respawn_time: 0,
            dragon_uuid: None,
            exit_portal_location: None,
            gateways: Vec::new(),
            respawn_crystals: Vec::new(),
        }
    }
}

fn ints(tag: Option<&Tag>) -> Option<Vec<i32>> {
    match tag? {
        Tag::IntArray(values) => Some(values.clone()),
        Tag::List(values) => values
            .iter()
            .map(|value| value.as_i64().map(|value| value as i32))
            .collect(),
        _ => None,
    }
}

impl FightData {
    pub fn read(tag: &Tag) -> Self {
        let flag = |key: &str, default: bool| {
            tag.get(key)
                .and_then(Tag::as_i64)
                .map_or(default, |value| value != 0)
        };
        Self {
            needs_state_scanning: flag("needs_state_scanning", true),
            dragon_killed: flag("dragon_killed", false),
            previously_killed: flag("previously_killed", false),
            respawn_stage: tag
                .get("respawn_stage")
                .and_then(Tag::as_str)
                .map(str::to_owned),
            respawn_time: tag
                .get("respawn_time")
                .and_then(Tag::as_i64)
                .map_or(0, |value| value as i32),
            dragon_uuid: ints(tag.get("dragon_uuid")).and_then(|uuid| uuid.try_into().ok()),
            exit_portal_location: ints(tag.get("exit_portal_location"))
                .filter(|pos| pos.len() == 3)
                .map(|pos| (pos[0], pos[1], pos[2])),
            gateways: ints(tag.get("gateways")).into_iter().flatten().collect(),
            respawn_crystals: match tag.get("respawn_crystals") {
                Some(Tag::List(crystals)) => crystals.clone(),
                _ => Vec::new(),
            },
        }
    }

    pub fn write(&self) -> Tag {
        let flag = |value: bool| Tag::Byte(i8::from(value));
        let mut map = BTreeMap::from([
            (
                "needs_state_scanning".to_owned(),
                flag(self.needs_state_scanning),
            ),
            ("dragon_killed".to_owned(), flag(self.dragon_killed)),
            ("previously_killed".to_owned(), flag(self.previously_killed)),
            ("respawn_time".to_owned(), Tag::Int(self.respawn_time)),
        ]);
        if let Some(stage) = &self.respawn_stage {
            map.insert("respawn_stage".to_owned(), Tag::String(stage.clone()));
        }
        if let Some(uuid) = self.dragon_uuid {
            map.insert("dragon_uuid".to_owned(), Tag::IntArray(uuid.to_vec()));
        }
        if let Some((x, y, z)) = self.exit_portal_location {
            map.insert(
                "exit_portal_location".to_owned(),
                Tag::IntArray(vec![x, y, z]),
            );
        }
        if !self.gateways.is_empty() {
            map.insert("gateways".to_owned(), Tag::IntArray(self.gateways.clone()));
        }
        if !self.respawn_crystals.is_empty() {
            map.insert(
                "respawn_crystals".to_owned(),
                Tag::List(self.respawn_crystals.clone()),
            );
        }
        Tag::Compound(map)
    }

    pub fn init(&mut self, seed: i64) -> bool {
        if !self.gateways.is_empty() {
            return false;
        }
        let mut gateways: Vec<i32> = (0..GATEWAY_COUNT).collect();
        let mut random = LegacyRandom::new(seed);
        for i in (2..=gateways.len()).rev() {
            let j = random.next_i32_bound(i as i32) as usize;
            gateways.swap(i - 1, j);
        }
        self.gateways = gateways;
        true
    }

    pub fn apply_scan(&mut self, scan: DragonFightScan) {
        self.previously_killed = scan.previously_killed;
        self.dragon_killed = scan.dragon_killed;
        if scan.dragon_seen {
            self.dragon_uuid = scan.dragon_uuid;
        }
        if scan.exit_portal_location.is_some() {
            self.exit_portal_location = scan.exit_portal_location;
        }
        self.needs_state_scanning = false;
    }
}

pub struct DragonFight {
    pub data: FightData,
    pub clock: f64,
    pub ticks_since_last_player_scan: u32,
    pub players: Vec<PlayerUuid>,
    pub scanning: Option<mpsc::Receiver<DragonFightScan>>,
    pub dirty: bool,
}

impl Default for DragonFight {
    fn default() -> Self {
        Self::new(FightData::default())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FightEvents {
    pub loaded: Option<Result<bool, String>>,
    pub arena: Option<usize>,
    pub scanned: Option<DragonFightScan>,
    pub saved: Option<Result<(), String>>,
}

pub fn arena_ticket() -> (ChunkPos, i32) {
    let (x, _, z) = FIGHT_ORIGIN;
    (
        ChunkPos::new(x.div_euclid(16), z.div_euclid(16)),
        ARENA_TICKET_LEVEL - 1,
    )
}

pub fn valid_player(feet: [f64; 3], alive: bool) -> bool {
    let (x, y, z) = FIGHT_ORIGIN;
    let centre = [f64::from(x), f64::from(DRAGON_SPAWN_Y + y), f64::from(z)];
    let distance: f64 = feet
        .iter()
        .zip(centre)
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    alive && distance <= PLAYER_RANGE * PLAYER_RANGE
}

pub fn arena_loaded(loaded: impl Fn((i32, i32)) -> bool) -> bool {
    let (x, _, z) = FIGHT_ORIGIN;
    let (cx, cz) = (x.div_euclid(16), z.div_euclid(16));
    let row = (cx - ARENA_SIZE_CHUNKS..=cx + ARENA_SIZE_CHUNKS)
        .all(|x| loaded((x, cz + ARENA_SIZE_CHUNKS)));
    let around = (cx - 1..=cx + 1).all(|x| (cz - 1..=cz + 1).all(|z| loaded((x, z))));
    row && around
}

pub fn scan_job(
    exit_portal_location: Option<(i32, i32, i32)>,
    send: mpsc::Sender<DragonFightScan>,
) -> Job {
    Box::new(move |sim: &mut ServerSim| -> Box<dyn Any + Send> {
        let _ = send.send(sim.scan_dragon_fight(FIGHT_ORIGIN, exit_portal_location));
        Box::new(())
    })
}

impl DragonFight {
    pub fn new(data: FightData) -> Self {
        Self {
            data,
            clock: 0.0,
            ticks_since_last_player_scan: TIME_BETWEEN_PLAYER_SCANS + 1,
            players: Vec::new(),
            scanning: None,
            dirty: false,
        }
    }

    pub fn load(save: Option<&WorldSave>, seed: i64) -> (Self, Option<Result<bool, String>>) {
        let read = save.map(|save| save.read_saved_in(Dimension::End, SAVED_DATA));
        let data = match &read {
            Some(Ok(Some(tag))) => FightData::read(tag),
            _ => FightData::default(),
        };
        let mut fight = Self::new(data);
        fight.dirty = fight.data.init(seed);
        (fight, read.map(|read| read.map(|tag| tag.is_some())))
    }

    pub fn ticks(&mut self, seconds: f64) -> u32 {
        self.clock += seconds.max(0.0);
        let ticks = (self.clock / TICK_SECONDS).floor();
        self.clock -= ticks * TICK_SECONDS;
        ticks.min(f64::from(u32::MAX)) as u32
    }

    pub fn tick(
        &mut self,
        valid: impl Fn() -> Vec<PlayerUuid>,
        arena_loaded: bool,
    ) -> (Vec<(ChunkPos, i32)>, bool) {
        self.ticks_since_last_player_scan += 1;
        if self.ticks_since_last_player_scan >= TIME_BETWEEN_PLAYER_SCANS {
            self.players = valid();
            self.ticks_since_last_player_scan = 0;
        }
        if self.players.is_empty() {
            return (Vec::new(), false);
        }
        let scan = arena_loaded && self.data.needs_state_scanning && self.scanning.is_none();
        (vec![arena_ticket()], scan)
    }

    pub fn start_scan(&mut self, entities: &mut Entities) {
        let (send, answer) = mpsc::channel();
        entities.run_job(scan_job(self.data.exit_portal_location, send));
        self.scanning = Some(answer);
    }

    pub fn receive_scan(&mut self) -> Option<DragonFightScan> {
        let scan = match self.scanning.as_ref()?.try_recv() {
            Ok(scan) => scan,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.scanning = None;
                return None;
            }
        };
        self.scanning = None;
        self.data.apply_scan(scan);
        self.dirty = true;
        Some(scan)
    }

    pub fn save(&mut self, save: Option<&WorldSave>) -> Option<Result<(), String>> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        save.map(|save| save.write_saved_in(Dimension::End, SAVED_DATA, self.data.write()))
    }
}

pub fn valid_players<'a>(
    players: impl Iterator<Item = &'a Player>,
    dimension: Dimension,
) -> Vec<PlayerUuid> {
    players
        .filter(|player| player.dimension == dimension && player.switching.is_none())
        .filter_map(|player| {
            let place = player.place.filter(|place| place.dimension == dimension)?;
            valid_player(place.feet, !player.dead).then(|| player.uuid.clone())
        })
        .collect()
}

pub fn tick_level<'a>(
    level: &mut Level,
    players: impl Iterator<Item = &'a Player> + Clone,
    save: Option<&WorldSave>,
    seed: i64,
    seconds: f64,
) -> FightEvents {
    let mut events = FightEvents::default();
    if level.dimension != Dimension::End {
        return events;
    }
    let dimension = level.dimension;
    let Level {
        dragon_fight,
        terrain,
        entities,
        ..
    } = level;
    let fight = dragon_fight.get_or_insert_with(|| {
        let (fight, loaded) = DragonFight::load(save, seed);
        events.loaded = loaded;
        fight
    });
    events.scanned = fight.receive_scan();
    let loaded =
        entities.is_some() && arena_loaded(|chunk| terrain.scene.generated_chunk(chunk).is_some());
    let before = fight.players.len();
    let mut tickets = None;
    for _ in 0..fight.ticks(seconds) {
        let (ticket, scan) = fight.tick(|| valid_players(players.clone(), dimension), loaded);
        tickets = Some(ticket);
        if let Some(entities) = entities.as_mut().filter(|_| scan) {
            fight.start_scan(entities);
        }
    }
    let after = fight.players.len();
    if let Some(tickets) = tickets {
        terrain.stream.set_tickets(&tickets);
    }
    events.arena = (before.min(1) != after.min(1)).then_some(after);
    events.saved = fight.save(save);
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(name: &str) -> PlayerUuid {
        PlayerUuid(name.to_owned())
    }

    #[test]
    fn the_arena_ticket_keeps_chunks_minus_eight_to_eight_ticking() {
        assert_eq!(arena_ticket(), (ChunkPos::new(0, 0), 8));
    }

    #[test]
    fn players_count_within_192_blocks_of_the_dragon_spawn_while_alive() {
        assert!(valid_player([100.5, 49.0, 0.5], true));
        assert!(valid_player([0.0, 128.0, 192.0], true));
        assert!(!valid_player([0.0, 128.0, 192.01], true));
        assert!(!valid_player([150.0, 0.0, 0.0], true));
        assert!(!valid_player([0.0, 128.0, 0.0], false));
    }

    #[test]
    fn the_ticket_follows_the_players_scanned_every_twenty_ticks() {
        let mut fight = DragonFight::default();
        let steve = vec![uuid("steve")];
        assert_eq!(
            fight.tick(|| steve.clone(), false),
            (vec![arena_ticket()], false),
            "the first tick scans"
        );
        assert_eq!(fight.players, steve);
        for _ in 0..19 {
            assert_eq!(
                fight.tick(Vec::new, false).0,
                vec![arena_ticket()],
                "players are kept between scans"
            );
        }
        assert_eq!(
            fight.tick(Vec::new, false).0,
            Vec::new(),
            "the twentieth tick scans again"
        );
        assert!(fight.players.is_empty());
    }

    #[test]
    fn the_fight_clock_counts_whole_ticks() {
        let mut fight = DragonFight::default();
        assert_eq!(fight.ticks(0.12), 2);
        assert_eq!(fight.ticks(0.04), 1);
    }

    #[test]
    fn the_state_scan_waits_for_a_player_and_the_arena_and_runs_once() {
        let mut fight = DragonFight::default();
        let steve = vec![uuid("steve")];
        assert_eq!(fight.tick(Vec::new, true), (Vec::new(), false));
        fight.ticks_since_last_player_scan = TIME_BETWEEN_PLAYER_SCANS;
        assert!(!fight.tick(|| steve.clone(), false).1);
        assert!(fight.tick(|| steve.clone(), true).1);
        let (_send, answer) = mpsc::channel();
        fight.scanning = Some(answer);
        assert!(!fight.tick(|| steve.clone(), true).1, "one at a time");
        fight.scanning = None;
        fight.data.needs_state_scanning = false;
        assert!(!fight.tick(|| steve.clone(), true).1);
    }

    #[test]
    fn the_arena_is_loaded_with_the_far_row_and_the_three_by_three_around_the_origin() {
        assert!(arena_loaded(|_| true));
        assert!(!arena_loaded(|chunk| chunk != (-8, 8)));
        assert!(!arena_loaded(|chunk| chunk != (1, -1)));
        assert!(
            arena_loaded(|chunk| chunk != (-8, -8) && chunk != (5, 0)),
            "vanilla's isArenaLoaded only walks z = 8"
        );
    }

    #[test]
    fn a_new_fight_needs_scanning_and_shuffles_its_twenty_gateways_by_the_seed() {
        let mut data = FightData::default();
        assert!(data.needs_state_scanning);
        assert!(data.init(0));
        assert_eq!(
            data.gateways,
            [
                6, 1, 15, 17, 4, 18, 12, 11, 14, 7, 19, 3, 16, 9, 8, 10, 2, 13, 5, 0
            ]
        );
        assert!(!data.init(1), "saved gateways stay");
        let mut other = FightData::default();
        other.init(1);
        assert_eq!(
            other.gateways,
            [
                2, 17, 6, 15, 11, 13, 19, 8, 7, 12, 14, 10, 9, 0, 4, 3, 16, 1, 18, 5
            ]
        );
    }

    #[test]
    fn the_fight_is_saved_with_vanilla_s_keys_and_read_back() {
        let data = FightData {
            needs_state_scanning: false,
            dragon_killed: true,
            previously_killed: true,
            respawn_stage: None,
            respawn_time: 0,
            dragon_uuid: Some([1, -2, 3, -4]),
            exit_portal_location: Some((0, 62, 0)),
            gateways: vec![4, 9, 1],
            respawn_crystals: Vec::new(),
        };
        let tag = data.write();
        let keys: Vec<&str> = match &tag {
            Tag::Compound(map) => map.keys().map(String::as_str).collect(),
            _ => Vec::new(),
        };
        assert_eq!(
            keys,
            [
                "dragon_killed",
                "dragon_uuid",
                "exit_portal_location",
                "gateways",
                "needs_state_scanning",
                "previously_killed",
                "respawn_time"
            ]
        );
        assert_eq!(tag.get("needs_state_scanning"), Some(&Tag::Byte(0)));
        assert_eq!(
            tag.get("exit_portal_location"),
            Some(&Tag::IntArray(vec![0, 62, 0]))
        );
        assert_eq!(FightData::read(&tag), data);
        let fresh = FightData::default().write();
        assert_eq!(fresh.get("needs_state_scanning"), Some(&Tag::Byte(1)));
        assert_eq!(fresh.get("gateways"), None, "an empty list is left out");
        let listed = Tag::Compound(BTreeMap::from([
            (
                "gateways".to_owned(),
                Tag::List(vec![Tag::Int(3), Tag::Int(7)]),
            ),
            ("respawn_stage".to_owned(), Tag::String("start".to_owned())),
        ]));
        let read = FightData::read(&listed);
        assert_eq!(read.gateways, [3, 7]);
        assert_eq!(read.respawn_stage.as_deref(), Some("start"));
        assert!(
            read.needs_state_scanning,
            "a missing flag takes its default"
        );
        assert_eq!(FightData::read(&read.write()), read);
    }

    #[test]
    fn a_scan_records_the_exit_portal_and_keeps_an_unseen_dragon_s_uuid() {
        let mut data = FightData {
            dragon_uuid: Some([5, 6, 7, 8]),
            ..FightData::default()
        };
        data.apply_scan(DragonFightScan {
            previously_killed: true,
            dragon_killed: true,
            dragon_seen: false,
            dragon_uuid: None,
            exit_portal_location: Some((0, 60, 0)),
        });
        assert!(!data.needs_state_scanning);
        assert!(data.previously_killed && data.dragon_killed);
        assert_eq!(data.dragon_uuid, Some([5, 6, 7, 8]));
        assert_eq!(data.exit_portal_location, Some((0, 60, 0)));
        data.apply_scan(DragonFightScan {
            previously_killed: false,
            dragon_killed: false,
            dragon_seen: true,
            dragon_uuid: None,
            exit_portal_location: None,
        });
        assert_eq!(data.dragon_uuid, None, "a discarded dragon is forgotten");
        assert_eq!(data.exit_portal_location, Some((0, 60, 0)));
    }

    fn stream_until(level: &mut Level, at: (i32, i32, i32), done: impl Fn(&Level) -> bool) {
        let started = std::time::Instant::now();
        while !done(level) {
            assert!(
                started.elapsed().as_secs() < 600,
                "the stream never got there"
            );
            let Level {
                terrain,
                entities,
                containers,
                ..
            } = level;
            crate::level::stream_tick(terrain, entities, containers, at);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn end_level(save: Option<&WorldSave>) -> Option<Level> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        let terrain = crate::level::Terrain::open(
            std::sync::Arc::new(registries),
            0,
            2,
            Dimension::End,
            save,
            None,
        )
        .ok()?;
        Some(Level::new(Dimension::End, terrain))
    }

    fn standing(at: [f64; 3]) -> Player {
        let mut steve = crate::the_end::tests::player(Dimension::End);
        steve.place = Some(crate::player_nbt::Place::standing(
            Dimension::End,
            at,
            90.0,
            0.0,
        ));
        steve
    }

    #[test]
    fn the_arena_ticket_loads_the_arena_beside_the_view_and_lets_it_go() {
        let Some(mut level) = end_level(None) else {
            return;
        };
        let platform = (100, 49, 0);
        let has =
            |level: &Level, chunk: (i32, i32)| level.terrain.scene.generated_chunk(chunk).is_some();
        stream_until(&mut level, platform, |level| has(level, (6, 0)));
        assert!(!has(&level, (0, 0)), "the island centre is out of view");
        let mut steve = standing([100.5, 49.0, 0.5]);
        let tick = |level: &mut Level, steve: &Player| {
            tick_level(level, [steve].into_iter(), None, 0, 0.05).arena
        };
        assert_eq!(tick(&mut level, &steve), Some(1));
        assert_eq!(tick(&mut level, &steve), None);
        let corners = [(-8, -8), (8, -8), (-8, 8), (8, 8), (0, 0)];
        stream_until(&mut level, platform, |level| {
            corners.iter().all(|&chunk| has(level, chunk))
        });
        for chunk in [(-9, 0), (0, 9), (-9, -9)] {
            assert!(!has(&level, chunk), "{chunk:?} is outside the ticket");
        }
        steve = standing([100.5, 49.0, 200.5]);
        for _ in 0..18 {
            assert_eq!(tick(&mut level, &steve), None, "until the next scan");
        }
        assert_eq!(tick(&mut level, &steve), Some(0));
        stream_until(&mut level, platform, |level| {
            corners.iter().all(|&chunk| !has(level, chunk))
        });
        assert!(has(&level, (6, 0)), "the view stays");
    }

    #[test]
    fn the_first_visit_scans_once_builds_the_podium_and_saves_the_fight() {
        let dir = std::env::temp_dir().join(format!("dragon-fight-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let save = crate::save::open_world(&dir, Some("fight"), Some("0"), 0)
            .save
            .expect("a world save in the temporary folder");
        let Some(mut level) = end_level(Some(&save)) else {
            let _ = std::fs::remove_dir_all(&dir);
            return;
        };
        let data = crate::data::GameData::default();
        level.entities = Some(Entities::new(
            &level.terrain.stream,
            Dimension::End,
            0,
            &data,
            None,
            None,
        ));
        let steve = standing([100.5, 49.0, 0.5]);
        let first = tick_level(&mut level, [&steve].into_iter(), Some(&save), 0, 0.05);
        assert_eq!(first.loaded, Some(Ok(false)), "no fight file yet");
        assert_eq!(first.saved, Some(Ok(())), "the shuffled gateways are saved");
        let written = save
            .read_saved_in(Dimension::End, SAVED_DATA)
            .expect("readable")
            .expect("written");
        assert_eq!(FightData::read(&written).gateways.len(), 20);
        let started = std::time::Instant::now();
        let scan = loop {
            assert!(started.elapsed().as_secs() < 600, "no scan");
            {
                let Level {
                    terrain,
                    entities,
                    containers,
                    ..
                } = &mut level;
                crate::level::stream_tick(terrain, entities, containers, (100, 49, 0));
            }
            let events = tick_level(&mut level, [&steve].into_iter(), Some(&save), 0, 0.05);
            if let Some(scan) = events.scanned {
                assert_eq!(events.saved, Some(Ok(())));
                break scan;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let at = scan.exit_portal_location.expect("an exit portal location");
        assert_eq!((at.0, at.2), (0, 0));
        assert!(!scan.previously_killed && !scan.dragon_killed);
        let saved = FightData::read(
            &save
                .read_saved_in(Dimension::End, SAVED_DATA)
                .expect("readable")
                .expect("written"),
        );
        assert!(!saved.needs_state_scanning);
        assert_eq!(saved.exit_portal_location, Some(at));
        assert!(
            level
                .dragon_fight
                .as_ref()
                .is_some_and(|fight| fight.scanning.is_none())
        );
        for _ in 0..40 {
            let events = tick_level(&mut level, [&steve].into_iter(), Some(&save), 0, 0.05);
            assert_eq!(events.scanned, None);
            assert_eq!(events.saved, None);
            assert!(
                level
                    .dragon_fight
                    .as_ref()
                    .is_some_and(|fight| fight.scanning.is_none()),
                "never scanned twice"
            );
        }
        let (reloaded, loaded) = DragonFight::load(Some(&save), 0);
        assert_eq!(loaded, Some(Ok(true)));
        assert_eq!(reloaded.data, saved);
        assert!(!reloaded.dirty);
        drop(level);
        drop(save);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
