use std::collections::BTreeMap;

use minecraft_commands::{Clientbound, Text};
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::game_mode::GameMode;
use minecraft_terrain::level_storage::WeatherData;
use minecraft_terrain::scene::Scene;
use minecraftoss_core::{BlockStateId, Registries};

use crate::dimension::DimensionRules;
use crate::entities::Entities;
use crate::level::{Level, Terrain};
use crate::player::Player;
use crate::player_nbt::Place;
use crate::respawn_block::{
    Bed, BlockView, Pos, STRAW_BED_BREAK_SOUND, bed_stand_up, centre, horizontal, is_bed,
    look_at_yaw, step,
};
use crate::server::{Globals, Server, default_clock};

pub const ZOMBIFIED_PIGLIN: &str = "minecraft:zombified_piglin";
pub const DEEP_SLEEP_TICKS: i32 = 100;
pub const WAKE_UP_TICKS: i32 = 110;
pub const NOT_POSSIBLE: &str = "sleep.not_possible";
pub const PLAYERS_SLEEPING: &str = "sleep.players_sleeping";
pub const SKIPPING_NIGHT: &str = "sleep.skipping_night";
const RAINING_LEVEL: f32 = 0.2;

const MONSTERS: [&str; 33] = [
    "minecraft:zombie",
    "minecraft:husk",
    "minecraft:drowned",
    "minecraft:zombie_villager",
    "minecraft:giant",
    "minecraft:skeleton",
    "minecraft:stray",
    "minecraft:wither_skeleton",
    "minecraft:bogged",
    "minecraft:parched",
    "minecraft:creeper",
    "minecraft:spider",
    "minecraft:cave_spider",
    "minecraft:enderman",
    "minecraft:endermite",
    "minecraft:silverfish",
    "minecraft:witch",
    "minecraft:pillager",
    "minecraft:vindicator",
    "minecraft:evoker",
    "minecraft:illusioner",
    "minecraft:ravager",
    "minecraft:blaze",
    "minecraft:guardian",
    "minecraft:elder_guardian",
    "minecraft:vex",
    "minecraft:zoglin",
    "minecraft:breeze",
    "minecraft:creaking",
    "minecraft:warden",
    "minecraft:wither",
    "minecraft:piglin",
    "minecraft:piglin_brute",
];

pub fn dark_outside(sky_light: f32, fixed_time: bool) -> bool {
    !fixed_time && (15.0 - sky_light) as i32 >= 4
}

pub fn monster_box((x, y, z): Pos) -> [f64; 6] {
    let (x, y, z) = (f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5);
    [x - 8.0, y - 5.0, z - 8.0, x + 8.0, y + 5.0, z + 8.0]
}

pub fn prevents_rest(kind: &str, angry_at_sleeper: bool) -> bool {
    if kind == ZOMBIFIED_PIGLIN {
        return angry_at_sleeper;
    }
    MONSTERS.contains(&kind)
}

pub fn intersects(a: [f64; 6], b: [f64; 6]) -> bool {
    (0..3).all(|i| a[i] < b[i + 3] && a[i + 3] > b[i])
}

pub fn monster_in<'a>(
    area: [f64; 6],
    mobs: impl IntoIterator<Item = (&'a str, bool, [f64; 6])>,
) -> bool {
    mobs.into_iter()
        .any(|(kind, angry, bounds)| intersects(area, bounds) && prevents_rest(kind, angry))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SleepStatus {
    pub active: i32,
    pub sleeping: i32,
}

impl SleepStatus {
    pub fn update(&mut self, players: impl IntoIterator<Item = (bool, bool)>) -> bool {
        let before = *self;
        *self = Self::default();
        for (spectator, sleeping) in players {
            if spectator {
                continue;
            }
            self.active += 1;
            if sleeping {
                self.sleeping += 1;
            }
        }
        (before.sleeping > 0 || self.sleeping > 0) && before != *self
    }

    pub fn sleepers_needed(&self, percentage: i32) -> i32 {
        let share = (self.active * percentage) as f32 / 100.0;
        1.max(share.ceil() as i32)
    }

    pub fn enough_sleeping(&self, percentage: i32) -> bool {
        self.sleeping >= self.sleepers_needed(percentage)
    }

    pub fn enough_deep_sleeping(&self, percentage: i32, deep: i32) -> bool {
        deep >= self.sleepers_needed(percentage)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sleeping {
    pub head: Pos,
    pub facing: (i32, i32),
    pub straw: bool,
    pub hurts: u64,
    pub height: f64,
}

impl Sleeping {
    pub fn feet(&self) -> [f64; 3] {
        bed_feet(self.head, self.height)
    }
}

pub fn bed_feet((x, y, z): Pos, height: f64) -> [f64; 3] {
    [
        f64::from(x) + 0.5,
        f64::from(y) + height + 0.125,
        f64::from(z) + 0.5,
    ]
}

#[derive(Debug, Default, PartialEq)]
pub struct SleepEvents {
    pub messages: Vec<(PlayerUuid, Clientbound)>,
    pub written: Vec<Pos>,
}

impl SleepEvents {
    fn extend(&mut self, other: SleepEvents) {
        self.messages.extend(other.messages);
        self.written.extend(other.written);
    }
}

pub trait Bedroom: BlockView {
    fn rules(&self) -> &DimensionRules;
    fn write(&mut self, writes: Vec<(Pos, Option<BlockStateId>)>);
    fn sound(&mut self, event: &'static str, at: [f64; 3]);
    fn loaded(&self, _pos: Pos) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BedAction {
    Lie(Bed),
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Night {
    pub dark: bool,
    pub raining: bool,
    pub percentage: i32,
    pub advance_time: bool,
    pub advance_weather: bool,
    pub clock: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Morning {
    pub clock: Option<i64>,
    pub clear_weather: bool,
}

pub fn sleep_height(room: &mut impl BlockView, registries: &Registries, bed: &Bed) -> Option<f64> {
    let at = if bed.straw { bed.foot() } else { bed.head };
    let top = registries
        .blocks
        .collision_boxes(room.state_at(at))
        .iter()
        .map(|b| b[4])
        .fold(f64::NEG_INFINITY, f64::max);
    top.is_finite().then_some(top)
}

fn mark_occupied(room: &mut impl Bedroom, registries: &Registries, head: Pos, occupied: bool) {
    let blocks = &registries.blocks;
    let state = room.state_at(head);
    if !is_bed(registries, state) {
        return;
    }
    let value = if occupied { "true" } else { "false" };
    let mut writes = Vec::new();
    writes.extend(
        blocks
            .with_property(state, "occupied", value)
            .map(|state| (head, Some(state))),
    );
    if let Some(facing) = blocks.property(state, "facing").and_then(horizontal) {
        let foot = step(head, (-facing.0, -facing.1));
        let other = room.state_at(foot);
        if blocks.block_of(other) == blocks.block_of(state)
            && blocks.property(other, "part") == Some("foot")
        {
            writes.extend(
                blocks
                    .with_property(other, "occupied", value)
                    .map(|state| (foot, Some(state))),
            );
        }
    }
    room.write(writes);
}

pub fn lie_down(
    room: &mut impl Bedroom,
    registries: &Registries,
    player: &mut Player,
    bed: &Bed,
) -> bool {
    if player.sleeping.is_some() || player.dead {
        return false;
    }
    let Some(height) = sleep_height(room, registries, bed) else {
        return false;
    };
    mark_occupied(room, registries, bed.head, true);
    let feet = bed_feet(bed.head, height);
    let (yaw, pitch) = player
        .place
        .map_or((0.0, 0.0), |place| (place.yaw, place.pitch));
    player.arrive(Place::standing(player.dimension, feet, yaw, pitch));
    player.sleeping = Some(Sleeping {
        head: bed.head,
        facing: bed.facing,
        straw: bed.straw,
        hurts: player.survival.hurts(),
        height,
    });
    player.sleep_timer = 0;
    player.time_since_rest = 0;
    true
}

pub fn get_up(
    room: &mut impl Bedroom,
    registries: &Registries,
    player: &mut Player,
    wake_at_once: bool,
) {
    let Some(sleeping) = player.sleeping.take() else {
        return;
    };
    let head = sleeping.head;
    let state = room.state_at(head);
    if is_bed(registries, state) {
        let blocks = &registries.blocks;
        let facing = blocks
            .property(state, "facing")
            .and_then(horizontal)
            .unwrap_or(sleeping.facing);
        mark_occupied(room, registries, head, false);
        let straw = blocks.block(blocks.block_of(state)).is_a("StrawBedBlock");
        let rules = room.rules();
        let leaves = if straw {
            rules.straw_bed_rule.destroy_on_leave
        } else {
            rules.bed_rule.destroy_on_leave
        };
        if leaves {
            let foot = step(head, (-facing.0, -facing.1));
            let mut removed = vec![(head, None)];
            if blocks.block_of(room.state_at(foot)) == blocks.block_of(state) {
                removed.push((foot, None));
            }
            room.write(removed);
            room.sound(STRAW_BED_BREAK_SOUND, centre(head));
        }
        if !player.dead {
            let yaw = player.place.map_or(0.0, |place| place.yaw);
            let (x, y, z) = head;
            let feet = bed_stand_up(room, registries, head, facing, yaw).unwrap_or([
                f64::from(x) + 0.5,
                f64::from(y) + 1.1,
                f64::from(z) + 0.5,
            ]);
            let yaw = look_at_yaw(feet, head);
            player.arrive(Place::standing(player.dimension, feet, yaw, 0.0));
        }
    }
    player.sleep_timer = if wake_at_once { 0 } else { DEEP_SLEEP_TICKS };
}

pub fn wake_saved_sleeper(
    room: &mut impl Bedroom,
    registries: &Registries,
    player: &mut Player,
) -> bool {
    let Some(head) = player.saved_in_bed.filter(|head| room.loaded(*head)) else {
        return false;
    };
    player.saved_in_bed = None;
    let state = room.state_at(head);
    if player.sleeping.is_some() || !is_bed(registries, state) {
        return false;
    }
    let blocks = &registries.blocks;
    let Some(facing) = blocks.property(state, "facing").and_then(horizontal) else {
        return false;
    };
    let straw = blocks.block(blocks.block_of(state)).is_a("StrawBedBlock");
    let bed = Bed {
        head,
        facing,
        occupied: true,
        straw,
    };
    let height = sleep_height(room, registries, &bed).unwrap_or(0.0);
    player.sleeping = Some(Sleeping {
        head,
        facing,
        straw,
        hurts: player.survival.hurts(),
        height,
    });
    get_up(room, registries, player, true);
    true
}

pub fn wake_for_teleport(
    room: &mut impl Bedroom,
    registries: &Registries,
    player: &mut Player,
) -> bool {
    if player.sleeping.is_none() {
        return false;
    }
    let (place, arrival) = (player.place, player.arrival);
    get_up(room, registries, player, true);
    player.place = place;
    player.arrival = arrival;
    true
}

pub fn cleared_weather(data: WeatherData) -> WeatherData {
    WeatherData {
        rain_time: 0,
        raining: false,
        thunder_time: 0,
        thundering: false,
        ..data
    }
}

fn overlay(text: Text) -> Clientbound {
    Clientbound::System {
        text,
        overlay: true,
    }
}

fn update_status(
    status: &mut SleepStatus,
    sleepers: &[&mut Player],
    percentage: i32,
    published: bool,
    events: &mut SleepEvents,
) {
    let changed = status.update(sleepers.iter().map(|player| {
        (
            player.mode.mode == GameMode::Spectator,
            player.sleeping.is_some(),
        )
    }));
    if !changed || !published || percentage > 100 {
        return;
    }
    let text = if status.enough_sleeping(percentage) {
        Text::translate(SKIPPING_NIGHT, Vec::new())
    } else {
        Text::translate(
            PLAYERS_SLEEPING,
            vec![
                Text::literal(status.sleeping.to_string()),
                Text::literal(status.sleepers_needed(percentage).to_string()),
            ],
        )
    };
    for player in sleepers {
        events
            .messages
            .push((player.uuid.clone(), overlay(text.clone())));
    }
}

pub fn tick_sleepers(
    room: &mut impl Bedroom,
    registries: &Registries,
    status: &mut SleepStatus,
    sleepers: &mut [&mut Player],
    night: &Night,
    published: bool,
    events: &mut SleepEvents,
) -> Morning {
    let percentage = night.percentage;
    update_status(status, sleepers, percentage, published, events);
    let deep = sleepers
        .iter()
        .filter(|player| player.sleeping.is_some() && player.sleep_timer >= DEEP_SLEEP_TICKS)
        .count() as i32;
    let mut morning = Morning::default();
    if status.enough_sleeping(percentage) && status.enough_deep_sleeping(percentage, deep) {
        morning.clock = night
            .clock
            .filter(|_| night.advance_time)
            .and_then(|total| minecraft_commands::commands::ticks_to_marker(total, 0));
        status.sleeping = 0;
        for player in sleepers.iter_mut() {
            get_up(room, registries, player, false);
        }
        morning.clear_weather = night.advance_weather && night.raining;
    }
    let mut woke = false;
    for player in sleepers.iter_mut() {
        woke |= wake_saved_sleeper(room, registries, player);
        if player.sleeping.is_some() {
            player.sleep_timer = (player.sleep_timer + 1).min(DEEP_SLEEP_TICKS);
            if !room.rules().bed_rule.can_sleep.holds(night.dark) {
                get_up(room, registries, player, false);
                woke = true;
            }
        } else if player.sleep_timer > 0 {
            player.sleep_timer += 1;
            if player.sleep_timer >= WAKE_UP_TICKS {
                player.sleep_timer = 0;
            }
        }
        if let Some(sleeping) = player.sleeping {
            let bed_gone = !is_bed(registries, room.state_at(sleeping.head));
            if bed_gone || player.dead || player.survival.hurts() != sleeping.hurts {
                get_up(room, registries, player, true);
                woke = true;
            }
        }
        if player.dead {
            player.time_since_rest = 0;
        } else if player.sleeping.is_none() {
            player.time_since_rest = player.time_since_rest.saturating_add(1);
        }
    }
    if woke {
        update_status(status, sleepers, percentage, published, events);
    }
    morning
}

struct LevelRoom<'a> {
    terrain: &'a mut Terrain,
    entities: Option<&'a mut Entities>,
    written: &'a mut Vec<Pos>,
}

impl BlockView for LevelRoom<'_> {
    fn state_at(&mut self, pos: Pos) -> BlockStateId {
        Scene::block(&self.terrain.scene, pos)
            .and_then(|block| self.terrain.stream.states.state_of(block))
            .unwrap_or(BlockStateId::AIR)
    }
}

impl Bedroom for LevelRoom<'_> {
    fn rules(&self) -> &DimensionRules {
        &self.terrain.rules
    }

    fn write(&mut self, writes: Vec<(Pos, Option<BlockStateId>)>) {
        let terrain = &mut *self.terrain;
        let writes: Vec<_> = writes
            .into_iter()
            .map(|(pos, state)| {
                let block = state.and_then(|state| terrain.stream.states.block(state).cloned());
                (pos, block)
            })
            .collect();
        self.written.extend(writes.iter().map(|(pos, _)| *pos));
        match self.entities.as_deref_mut() {
            Some(entities) => crate::level::write_blocks(terrain, entities, writes),
            None => {
                let positions: Vec<Pos> = writes.iter().map(|(pos, _)| *pos).collect();
                for (pos, block) in writes {
                    terrain.scene.set(pos, block);
                }
                terrain.stream.record_edits(&terrain.scene, &positions);
                terrain.stream.mark_edited(&terrain.scene, &positions);
            }
        }
    }

    fn loaded(&self, (x, _, z): Pos) -> bool {
        self.terrain
            .scene
            .generated_chunk((x >> 4, z >> 4))
            .is_some()
    }

    fn sound(&mut self, event: &'static str, at: [f64; 3]) {
        if let Some(entities) = self.entities.as_deref_mut() {
            entities
                .sounds
                .push((event.to_owned(), glam::DVec3::from_array(at), 1.0, 1.0));
        }
    }
}

fn in_level(players: &mut BTreeMap<PlayerUuid, Player>, dimension: Dimension) -> Vec<&mut Player> {
    players
        .values_mut()
        .filter(|player| player.dimension == dimension && player.switching.is_none())
        .collect()
}

pub fn start(
    globals: &Globals,
    level: &mut Level,
    players: &mut BTreeMap<PlayerUuid, Player>,
    uuid: &PlayerUuid,
    bed: &Bed,
    published: bool,
) -> SleepEvents {
    let mut events = SleepEvents::default();
    let percentage = globals.game_rules.int("players_sleeping_percentage");
    let Level {
        terrain,
        entities,
        sleep_status,
        dimension,
        ..
    } = level;
    let Some(player) = players.get_mut(uuid) else {
        return events;
    };
    if player.riding.take().is_some()
        && let Some(entities) = entities.as_mut()
    {
        entities.dismount(u64::from(player.id.0));
    }
    let mut room = LevelRoom {
        terrain,
        entities: entities.as_mut(),
        written: &mut events.written,
    };
    if !lie_down(&mut room, &globals.registries, player, bed) {
        return events;
    }
    if percentage > 100 {
        events.messages.push((
            uuid.clone(),
            overlay(Text::translate(NOT_POSSIBLE, Vec::new())),
        ));
    }
    let sleepers = in_level(players, *dimension);
    update_status(sleep_status, &sleepers, percentage, published, &mut events);
    events
}

pub fn stop(
    globals: &Globals,
    level: &mut Level,
    players: &mut BTreeMap<PlayerUuid, Player>,
    uuid: &PlayerUuid,
    wake_at_once: bool,
    published: bool,
) -> SleepEvents {
    let mut events = SleepEvents::default();
    let percentage = globals.game_rules.int("players_sleeping_percentage");
    let Level {
        terrain,
        entities,
        sleep_status,
        dimension,
        ..
    } = level;
    let Some(player) = players
        .get_mut(uuid)
        .filter(|player| player.sleeping.is_some())
    else {
        return events;
    };
    let mut room = LevelRoom {
        terrain,
        entities: entities.as_mut(),
        written: &mut events.written,
    };
    get_up(&mut room, &globals.registries, player, wake_at_once);
    let sleepers = in_level(players, *dimension);
    update_status(sleep_status, &sleepers, percentage, published, &mut events);
    events
}

pub fn wake_teleported(
    globals: &Globals,
    levels: &mut BTreeMap<Dimension, Level>,
    players: &mut BTreeMap<PlayerUuid, Player>,
    teleported: &[PlayerUuid],
    published: bool,
) -> SleepEvents {
    let mut events = SleepEvents::default();
    let percentage = globals.game_rules.int("players_sleeping_percentage");
    for uuid in teleported {
        let Some(player) = players
            .get_mut(uuid)
            .filter(|player| player.sleeping.is_some())
        else {
            continue;
        };
        let dimension = player.dimension;
        let Some(Level {
            terrain,
            entities,
            sleep_status,
            ..
        }) = levels.get_mut(&dimension)
        else {
            continue;
        };
        let mut room = LevelRoom {
            terrain,
            entities: entities.as_mut(),
            written: &mut events.written,
        };
        wake_for_teleport(&mut room, &globals.registries, player);
        let sleepers = in_level(players, dimension);
        update_status(sleep_status, &sleepers, percentage, published, &mut events);
    }
    events
}

pub fn tick(
    globals: &mut Globals,
    level: &mut Level,
    players: &mut BTreeMap<PlayerUuid, Player>,
    published: bool,
) -> SleepEvents {
    let mut events = SleepEvents::default();
    let clock = default_clock(level.dimension);
    let night = Night {
        dark: level.dark_outside(globals),
        raining: level
            .entities
            .as_ref()
            .is_some_and(|entities| entities.weather.rain_level > RAINING_LEVEL),
        percentage: globals.game_rules.int("players_sleeping_percentage"),
        advance_time: globals.game_rules.bool("advance_time"),
        advance_weather: globals.game_rules.bool("advance_weather"),
        clock: clock.map(|_| globals.default_clock_ticks(level.dimension)),
    };
    let Level {
        terrain,
        entities,
        sleep_status,
        dimension,
        ..
    } = level;
    let mut sleepers = in_level(players, *dimension);
    let mut written = Vec::new();
    let mut room = LevelRoom {
        terrain,
        entities: entities.as_mut(),
        written: &mut written,
    };
    let morning = tick_sleepers(
        &mut room,
        &globals.registries,
        sleep_status,
        &mut sleepers,
        &night,
        published,
        &mut events,
    );
    events.written.extend(written);
    if let (Some(total), Some(clock)) = (morning.clock, clock) {
        globals.set_clock(clock, total);
    }
    if morning.clear_weather
        && let Some(weather) = entities
            .as_mut()
            .and_then(|entities| entities.reset_weather_cycle())
    {
        globals.weather = Some(weather);
    }
    events
}

pub fn serve(
    globals: &mut Globals,
    levels: &mut BTreeMap<Dimension, Level>,
    players: &mut BTreeMap<PlayerUuid, Player>,
    uuid: &PlayerUuid,
    action: Option<BedAction>,
    ticks: u32,
    published: bool,
) -> SleepEvents {
    let mut events = SleepEvents::default();
    let Some(dimension) = players
        .get(uuid)
        .filter(|player| player.switching.is_none())
        .map(|player| player.dimension)
    else {
        return events;
    };
    let Some(level) = levels.get_mut(&dimension) else {
        return events;
    };
    match action {
        Some(BedAction::Lie(bed)) => {
            events.extend(start(globals, level, players, uuid, &bed, published));
        }
        Some(BedAction::Leave) => {
            events.extend(stop(globals, level, players, uuid, false, published));
        }
        None => {}
    }
    for _ in 0..ticks {
        events.extend(tick(globals, level, players, published));
    }
    events
}

impl Server {
    pub fn tick_sleep(&mut self, dimension: Dimension) -> SleepEvents {
        match self.levels.get_mut(&dimension) {
            Some(level) => tick(&mut self.globals, level, &mut self.players, self.published),
            None => SleepEvents::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::environment::DimensionEnvironment;
    use minecraftoss_core::Registries;

    fn registries() -> Option<Registries> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        Registries::load(&paths).ok()
    }

    #[test]
    fn the_sky_is_dark_at_light_eleven_or_less() {
        assert!(dark_outside(11.0, false));
        assert!(!dark_outside(11.01, false));
        assert!(!dark_outside(15.0, false));
        assert!(dark_outside(4.0, false));
        for light in [0.0, 4.0, 11.0, 15.0] {
            assert!(!dark_outside(light, true));
        }
    }

    #[test]
    fn the_overworld_night_runs_from_tick_12523_to_23477_in_clear_weather() {
        let Some(registries) = registries() else {
            return;
        };
        let overworld = DimensionEnvironment::load(&registries, "minecraft:overworld").unwrap();
        let dark = |ticks: i64| dark_outside(overworld.level_sky_light(ticks, 0.0, 0.0), false);
        let night: Vec<i64> = (0..24000).filter(|&ticks| dark(ticks)).collect();
        assert_eq!(
            (night.first(), night.last(), night.len()),
            (Some(&12523), Some(&23477), 23477 - 12523 + 1)
        );
        assert!(dark(12523 + 24000 * 3) && !dark(12522 + 24000 * 3));
        let nether = DimensionEnvironment::load(&registries, "minecraft:the_nether").unwrap();
        assert_eq!(nether.level_sky_light(18000, 0.0, 0.0), 4.0);
    }

    #[test]
    fn a_thunderstorm_darkens_noon_but_rain_alone_does_not() {
        let Some(registries) = registries() else {
            return;
        };
        let overworld = DimensionEnvironment::load(&registries, "minecraft:overworld").unwrap();
        let rain = overworld.level_sky_light(6000, 1.0, 0.0);
        assert!((rain - 11.5625).abs() < 1.0e-4, "{rain}");
        assert!(!dark_outside(rain, false));
        let thunder = overworld.level_sky_light(6000, 1.0, 1.0);
        assert!(thunder <= 9.2 + 1.0e-4, "{thunder}");
        assert!(dark_outside(thunder, false));
    }

    #[test]
    fn sleepers_needed_rounds_up_and_needs_at_least_one() {
        let status = |active| SleepStatus {
            active,
            sleeping: 0,
        };
        assert_eq!(status(0).sleepers_needed(100), 1);
        assert_eq!(status(1).sleepers_needed(100), 1);
        assert_eq!(status(4).sleepers_needed(50), 2);
        assert_eq!(status(3).sleepers_needed(50), 2);
        assert_eq!(status(4).sleepers_needed(0), 1);
        assert_eq!(status(2).sleepers_needed(101), 3);
        let all = SleepStatus {
            active: 2,
            sleeping: 2,
        };
        assert!(!all.enough_sleeping(101) && !all.enough_deep_sleeping(101, 2));
        assert!(all.enough_sleeping(100) && all.enough_deep_sleeping(100, 2));
        assert!(!all.enough_deep_sleeping(100, 1));
    }

    #[test]
    fn the_sleep_count_changes_only_when_someone_sleeps() {
        let mut status = SleepStatus::default();
        assert!(!status.update([(false, false), (false, false)]));
        assert_eq!(
            status,
            SleepStatus {
                active: 2,
                sleeping: 0
            }
        );
        assert!(!status.update([(false, false)]), "nobody sleeps");
        assert!(status.update([(false, true), (false, false)]));
        assert!(!status.update([(false, true), (false, false)]), "no change");
        assert!(
            !status.update([(false, true), (false, false), (true, true)]),
            "a spectator does not count"
        );
        assert!(status.update([(false, true), (false, false), (false, false)]));
        assert!(status.update([(false, false), (false, false), (false, false)]));
        assert_eq!(status.sleeping, 0);
    }

    #[test]
    fn monsters_count_in_an_eight_by_five_box_around_the_head() {
        let area = monster_box((0, 64, 0));
        assert_eq!(area, [-7.5, 59.0, -7.5, 8.5, 69.0, 8.5]);
        let mob = |dx: f64, dy: f64, half: f64, height: f64| {
            let x = 0.5 + dx;
            [
                x - half,
                64.0 + dy,
                0.5 - half,
                x + half,
                64.0 + dy + height,
                0.5 + half,
            ]
        };
        let zombie = |dx, dy| mob(dx, dy, 0.3, 1.95);
        let near = |kind: &'static str, angry, bounds| monster_in(area, [(kind, angry, bounds)]);
        assert!(near("minecraft:zombie", false, zombie(8.2, 0.0)));
        assert!(!near("minecraft:zombie", false, zombie(8.4, 0.0)));
        assert!(near("minecraft:zombie", false, zombie(-8.2, 0.0)));
        assert!(near("minecraft:zombie", false, zombie(0.0, 4.99)));
        assert!(!near("minecraft:zombie", false, zombie(0.0, 5.0)));
        assert!(near("minecraft:zombie", false, zombie(0.0, -6.9)));
        assert!(!near("minecraft:zombie", false, zombie(0.0, -7.0)));
        for kind in [
            "minecraft:slime",
            "minecraft:magma_cube",
            "minecraft:ghast",
            "minecraft:phantom",
            "minecraft:shulker",
            "minecraft:hoglin",
            "minecraft:strider",
            "minecraft:iron_golem",
            "minecraft:wolf",
        ] {
            assert!(!near(kind, true, zombie(1.0, 0.0)), "{kind}");
        }
        for kind in [
            "minecraft:skeleton",
            "minecraft:creeper",
            "minecraft:spider",
            "minecraft:enderman",
            "minecraft:witch",
            "minecraft:blaze",
            "minecraft:zoglin",
            "minecraft:piglin",
            "minecraft:piglin_brute",
            "minecraft:wither_skeleton",
        ] {
            assert!(near(kind, false, zombie(1.0, 0.0)), "{kind}");
        }
        assert!(!near(ZOMBIFIED_PIGLIN, false, zombie(1.0, 0.0)));
        assert!(near(ZOMBIFIED_PIGLIN, true, zombie(1.0, 0.0)));
        assert!(monster_in(
            area,
            [
                ("minecraft:slime", false, zombie(1.0, 0.0)),
                ("minecraft:husk", false, zombie(-3.0, 1.0)),
            ]
        ));
    }
}

#[cfg(test)]
mod night_tests {
    use super::*;
    use crate::commands::HOST;
    use crate::death::DamageCause;
    use crate::survival::Survival;
    use minecraft_commands::{CommandServer, PlayerId};
    use minecraft_terrain::game_mode::PlayerMode;
    use minecraftoss_player::inventory::Inventory;
    use std::collections::HashMap;

    struct Room<'a> {
        registries: &'a Registries,
        placed: HashMap<Pos, BlockStateId>,
        rules: DimensionRules,
        sounds: Vec<(&'static str, [f64; 3])>,
        unloaded: bool,
    }

    impl<'a> Room<'a> {
        fn new(registries: &'a Registries) -> Self {
            let mut room = Self {
                registries,
                placed: HashMap::new(),
                rules: DimensionRules::load(registries, Dimension::Overworld).unwrap(),
                sounds: Vec::new(),
                unloaded: false,
            };
            for x in -6..=6 {
                for z in -6..=6 {
                    room.set((x, 63, z), "minecraft:stone");
                }
            }
            room
        }

        fn set(&mut self, pos: Pos, text: &str) {
            let state = self.registries.blocks.parse_state(text).unwrap();
            self.placed.insert(pos, state);
        }

        fn bed(&mut self, id: &str, foot: Pos, facing: &str) -> Bed {
            let (dx, dz) = horizontal(facing).unwrap();
            let head = step(foot, (dx, dz));
            self.set(
                foot,
                &format!("{id}[facing={facing},part=foot,occupied=false]"),
            );
            self.set(
                head,
                &format!("{id}[facing={facing},part=head,occupied=false]"),
            );
            Bed {
                head,
                facing: (dx, dz),
                occupied: false,
                straw: id == "minecraft:straw_bed",
            }
        }

        fn occupied(&self, pos: Pos) -> Option<&str> {
            let state = *self.placed.get(&pos)?;
            self.registries.blocks.property(state, "occupied")
        }
    }

    impl BlockView for Room<'_> {
        fn state_at(&mut self, pos: Pos) -> BlockStateId {
            self.placed.get(&pos).copied().unwrap_or(BlockStateId::AIR)
        }
    }

    impl Bedroom for Room<'_> {
        fn rules(&self) -> &DimensionRules {
            &self.rules
        }

        fn write(&mut self, writes: Vec<(Pos, Option<BlockStateId>)>) {
            for (pos, state) in writes {
                match state {
                    Some(state) => self.placed.insert(pos, state),
                    None => self.placed.remove(&pos),
                };
            }
        }

        fn sound(&mut self, event: &'static str, at: [f64; 3]) {
            self.sounds.push((event, at));
        }

        fn loaded(&self, _pos: Pos) -> bool {
            !self.unloaded
        }
    }

    fn registries() -> Option<Registries> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        Registries::load(&paths).ok()
    }

    fn player(id: u32, feet: [f64; 3]) -> Player {
        let mut chat = CommandServer::new(HOST);
        chat.join(PlayerId(id), format!("Steve{id}"));
        let mut player = Player::new(
            chat.profile(PlayerId(id)).unwrap(),
            PlayerMode::new(GameMode::Survival),
            Survival::default(),
            Inventory::default(),
        );
        player.place = Some(Place::standing(Dimension::Overworld, feet, 30.0, 10.0));
        player
    }

    fn night(percentage: i32) -> Night {
        Night {
            dark: true,
            raining: false,
            percentage,
            advance_time: true,
            advance_weather: true,
            clock: Some(13000 + 24000 * 2),
        }
    }

    fn tick(
        room: &mut Room,
        status: &mut SleepStatus,
        players: &mut [&mut Player],
        night: &Night,
    ) -> (Morning, SleepEvents) {
        let registries = room.registries;
        let mut events = SleepEvents::default();
        let morning = tick_sleepers(room, registries, status, players, night, true, &mut events);
        (morning, events)
    }

    #[test]
    fn lying_down_marks_the_bed_occupied_and_puts_the_feet_on_it() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let red = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        steve.time_since_rest = 500;
        steve.sleep_timer = 7;
        assert!(lie_down(&mut room, &registries, &mut steve, &red));
        let place = steve.arrival.unwrap();
        assert_eq!(place.feet, [0.5, 64.6875, 1.5]);
        assert_eq!(
            (place.yaw, place.pitch, place.motion),
            (30.0, 10.0, [0.0; 3])
        );
        assert_eq!(steve.place, Some(place));
        assert_eq!(room.occupied((0, 64, 1)), Some("true"));
        assert_eq!(room.occupied((0, 64, 0)), Some("true"));
        assert_eq!(
            steve.sleeping,
            Some(Sleeping {
                head: (0, 64, 1),
                facing: (0, 1),
                straw: false,
                hurts: 0,
                height: 0.5625,
            })
        );
        assert_eq!((steve.sleep_timer, steve.time_since_rest), (0, 0));
        assert!(
            !lie_down(&mut room, &registries, &mut steve, &red),
            "already asleep"
        );
        let straw = room.bed("minecraft:straw_bed", (3, 64, 0), "east");
        let mut alex = player(1, [3.5, 64.0, 2.5]);
        assert!(lie_down(&mut room, &registries, &mut alex, &straw));
        assert_eq!(alex.arrival.unwrap().feet, [4.5, 64.375, 0.5]);
        let mut dead = player(2, [0.5, 64.0, 3.5]);
        dead.dead = true;
        assert!(!lie_down(&mut room, &registries, &mut dead, &red));
    }

    #[test]
    fn the_sleep_timer_counts_to_100_and_the_wake_fade_to_110() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let bed = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        let mut status = SleepStatus::default();
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        for expected in [1, 2, 3] {
            tick(&mut room, &mut status, &mut [&mut steve], &night(101));
            assert_eq!(steve.sleep_timer, expected);
        }
        for _ in 0..200 {
            tick(&mut room, &mut status, &mut [&mut steve], &night(101));
        }
        assert_eq!(steve.sleep_timer, 100);
        get_up(&mut room, &registries, &mut steve, false);
        assert_eq!(steve.sleep_timer, 100);
        for expected in [101, 102, 103, 104, 105, 106, 107, 108, 109, 0, 0] {
            tick(&mut room, &mut status, &mut [&mut steve], &night(101));
            assert_eq!(steve.sleep_timer, expected);
        }
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        tick(&mut room, &mut status, &mut [&mut steve], &night(101));
        get_up(&mut room, &registries, &mut steve, true);
        assert_eq!(steve.sleep_timer, 0, "waking at once skips the fade");
    }

    #[test]
    fn the_night_skips_to_the_next_morning_once_everyone_has_slept_100_ticks() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let first = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let second = room.bed("minecraft:blue_bed", (3, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        let mut alex = player(1, [3.5, 64.0, 3.5]);
        let mut status = SleepStatus::default();
        assert!(lie_down(&mut room, &registries, &mut steve, &first));
        assert!(lie_down(&mut room, &registries, &mut alex, &second));
        for _ in 0..100 {
            let (morning, _) = tick(
                &mut room,
                &mut status,
                &mut [&mut steve, &mut alex],
                &night(100),
            );
            assert_eq!(morning, Morning::default());
        }
        assert!(steve.sleeping.is_some() && alex.sleeping.is_some());
        let (morning, events) = tick(
            &mut room,
            &mut status,
            &mut [&mut steve, &mut alex],
            &night(100),
        );
        assert_eq!(
            morning,
            Morning {
                clock: Some(72000),
                clear_weather: false,
            }
        );
        assert!(steve.sleeping.is_none() && alex.sleeping.is_none());
        assert_eq!(
            (steve.sleep_timer, alex.sleep_timer),
            (101, 101),
            "woken at 100, the fade's first tick follows in the same tick"
        );
        assert_eq!(room.occupied((0, 64, 1)), Some("false"));
        assert_eq!(room.occupied((3, 64, 0)), Some("false"));
        assert_eq!(status.sleeping, 0);
        assert!(events.messages.is_empty(), "the count was already cleared");
        let (morning, _) = tick(
            &mut room,
            &mut status,
            &mut [&mut steve, &mut alex],
            &night(100),
        );
        assert_eq!(morning, Morning::default());
        assert!(lie_down(&mut room, &registries, &mut steve, &first));
        let frozen = Night {
            advance_time: false,
            ..night(100)
        };
        for _ in 0..100 {
            tick(&mut room, &mut status, &mut [&mut steve], &frozen);
        }
        let (morning, _) = tick(&mut room, &mut status, &mut [&mut steve], &frozen);
        assert_eq!(morning.clock, None, "advance_time off keeps the clock");
        assert!(steve.sleeping.is_none(), "but the sleepers still wake");
        assert_eq!(
            minecraft_commands::commands::ticks_to_marker(24000 * 3, 0),
            None,
            "a morning on the dot does not move"
        );
    }

    #[test]
    fn a_skip_clears_rain_and_thunder_only_when_it_rains() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let bed = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let skip = |room: &mut Room, night: Night| {
            let mut steve = player(0, [0.5, 64.0, 3.5]);
            let mut status = SleepStatus::default();
            assert!(lie_down(room, &registries, &mut steve, &bed));
            (0..=100)
                .map(|_| tick(room, &mut status, &mut [&mut steve], &night).0)
                .last()
                .unwrap()
        };
        let rain = Night {
            raining: true,
            ..night(100)
        };
        assert!(skip(&mut room, rain).clear_weather);
        assert!(!skip(&mut room, night(100)).clear_weather);
        let fixed = Night {
            advance_weather: false,
            ..rain
        };
        assert!(!skip(&mut room, fixed).clear_weather);
        let stormy = WeatherData {
            clear_weather_time: 0,
            rain_time: 4000,
            thunder_time: 900,
            raining: true,
            thundering: true,
        };
        assert_eq!(
            cleared_weather(stormy),
            WeatherData {
                clear_weather_time: 0,
                rain_time: 0,
                thunder_time: 0,
                raining: false,
                thundering: false,
            }
        );
        let waiting = WeatherData {
            clear_weather_time: 1200,
            ..stormy
        };
        assert_eq!(cleared_weather(waiting).clear_weather_time, 1200);
    }

    #[test]
    fn half_the_players_are_enough_at_fifty_percent_and_none_at_101() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let first = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let second = room.bed("minecraft:blue_bed", (3, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        let mut alex = player(1, [3.5, 64.0, 3.5]);
        let mut status = SleepStatus::default();
        assert!(lie_down(&mut room, &registries, &mut steve, &first));
        let (_, events) = tick(
            &mut room,
            &mut status,
            &mut [&mut steve, &mut alex],
            &night(100),
        );
        let players_sleeping = Clientbound::System {
            text: Text::translate(
                PLAYERS_SLEEPING,
                vec![Text::literal("1"), Text::literal("2")],
            ),
            overlay: true,
        };
        assert_eq!(
            events.messages,
            vec![
                (steve.uuid.clone(), players_sleeping.clone()),
                (alex.uuid.clone(), players_sleeping)
            ]
        );
        for _ in 0..200 {
            let (morning, _) = tick(
                &mut room,
                &mut status,
                &mut [&mut steve, &mut alex],
                &night(100),
            );
            assert_eq!(morning.clock, None, "one of two is not enough at 100");
        }
        get_up(&mut room, &registries, &mut steve, true);
        tick(
            &mut room,
            &mut status,
            &mut [&mut steve, &mut alex],
            &night(50),
        );
        assert!(lie_down(&mut room, &registries, &mut steve, &first));
        let (_, events) = tick(
            &mut room,
            &mut status,
            &mut [&mut steve, &mut alex],
            &night(50),
        );
        let skipping = Clientbound::System {
            text: Text::translate(SKIPPING_NIGHT, Vec::new()),
            overlay: true,
        };
        assert_eq!(events.messages[0], (steve.uuid.clone(), skipping));
        let mut last = Morning::default();
        for _ in 0..100 {
            last = tick(
                &mut room,
                &mut status,
                &mut [&mut steve, &mut alex],
                &night(50),
            )
            .0;
        }
        assert_eq!(last.clock, Some(72000), "one of two at 50");
        assert!(lie_down(&mut room, &registries, &mut steve, &first));
        assert!(lie_down(&mut room, &registries, &mut alex, &second));
        for _ in 0..300 {
            let (morning, events) = tick(
                &mut room,
                &mut status,
                &mut [&mut steve, &mut alex],
                &night(101),
            );
            assert_eq!(morning.clock, None, "no amount of rest at 101");
            assert!(events.messages.is_empty(), "and no announcement");
        }
    }

    #[test]
    fn leaving_the_bed_stands_up_beside_it_facing_it() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let bed = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        get_up(&mut room, &registries, &mut steve, false);
        let place = steve.arrival.unwrap();
        assert!(steve.sleeping.is_none());
        assert_eq!(place.feet[1], 64.0);
        let off_bed = (place.feet[0] - 0.5).abs() >= 1.0 || (place.feet[2] - 1.0).abs() > 1.0;
        assert!(off_bed, "{:?}", place.feet);
        assert_eq!(
            place.yaw,
            crate::respawn_block::look_at_yaw(place.feet, (0, 64, 1))
        );
        assert_eq!(place.pitch, 0.0);
        assert_eq!(room.occupied((0, 64, 1)), Some("false"));
        assert_eq!(room.occupied((0, 64, 0)), Some("false"));
        assert!(room.sounds.is_empty());
        for x in -1..=1 {
            for z in -1..=2 {
                if (x, z) != (0, 0) && (x, z) != (0, 1) {
                    room.set((x, 64, z), "minecraft:stone");
                    room.set((x, 65, z), "minecraft:stone");
                }
            }
        }
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        get_up(&mut room, &registries, &mut steve, false);
        let feet = steve.arrival.unwrap().feet;
        assert!(
            feet[0] == 0.5 && feet[1] > 64.5 && (0.0..2.0).contains(&feet[2]),
            "walled in, the sleeper stands on the bed: {feet:?}"
        );
    }

    #[test]
    fn a_straw_bed_breaks_when_left() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let straw = room.bed("minecraft:straw_bed", (0, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        assert!(lie_down(&mut room, &registries, &mut steve, &straw));
        get_up(&mut room, &registries, &mut steve, false);
        assert!(!room.placed.contains_key(&(0, 64, 0)));
        assert!(!room.placed.contains_key(&(0, 64, 1)));
        assert_eq!(room.sounds, vec![(STRAW_BED_BREAK_SOUND, [0.5, 64.5, 1.5])]);
    }

    #[test]
    fn a_teleport_wakes_the_sleeper_at_once_and_leaves_it_where_it_went() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let bed = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        assert!(!wake_for_teleport(&mut room, &registries, &mut steve));
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        let mut status = SleepStatus::default();
        for _ in 0..30 {
            tick(&mut room, &mut status, &mut [&mut steve], &night(100));
        }
        assert_eq!(steve.sleep_timer, 30);
        let gone = Place::standing(Dimension::Overworld, [40.5, 70.0, -9.5], 90.0, 0.0);
        steve.place = Some(gone);
        steve.arrival = None;
        assert!(wake_for_teleport(&mut room, &registries, &mut steve));
        assert!(steve.sleeping.is_none());
        assert_eq!(steve.sleep_timer, 0, "no wake-up fade");
        assert_eq!((steve.place, steve.arrival), (Some(gone), None));
        assert_eq!(room.occupied((0, 64, 1)), Some("false"));
        assert_eq!(room.occupied((0, 64, 0)), Some("false"));
        let straw = room.bed("minecraft:straw_bed", (3, 64, 0), "east");
        let mut alex = player(1, [3.5, 64.0, 2.5]);
        assert!(lie_down(&mut room, &registries, &mut alex, &straw));
        assert!(wake_for_teleport(&mut room, &registries, &mut alex));
        assert!(
            !room.placed.contains_key(&straw.head),
            "a left straw bed breaks"
        );
    }

    #[test]
    fn a_broken_bed_or_daylight_or_a_hit_wakes_the_sleeper() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let bed = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        let mut status = SleepStatus::default();
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        tick(&mut room, &mut status, &mut [&mut steve], &night(100));
        room.placed.remove(&(0, 64, 1));
        room.placed.remove(&(0, 64, 0));
        tick(&mut room, &mut status, &mut [&mut steve], &night(100));
        assert!(steve.sleeping.is_none());
        assert_eq!(steve.sleep_timer, 0, "a broken bed wakes at once");
        let bed = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        let day = Night {
            dark: false,
            ..night(100)
        };
        tick(&mut room, &mut status, &mut [&mut steve], &day);
        assert!(steve.sleeping.is_none());
        assert_eq!(steve.sleep_timer, 100, "daylight wakes with the fade");
        tick(&mut room, &mut status, &mut [&mut steve], &night(100));
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        tick(&mut room, &mut status, &mut [&mut steve], &night(100));
        assert!(steve.sleeping.is_some());
        steve.survival.observe(20, 20);
        assert!(
            steve
                .survival
                .hurt(2.0, Default::default(), None, DamageCause::Generic)
        );
        tick(&mut room, &mut status, &mut [&mut steve], &night(100));
        assert!(steve.sleeping.is_none());
        assert_eq!(steve.sleep_timer, 0, "a hit wakes at once");
        assert_eq!(room.occupied((0, 64, 1)), Some("false"));
    }

    #[test]
    fn time_since_rest_counts_while_awake_and_resets_on_sleep_and_death() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let bed = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.0, 3.5]);
        let mut status = SleepStatus::default();
        for _ in 0..30 {
            tick(&mut room, &mut status, &mut [&mut steve], &night(101));
        }
        assert_eq!(steve.time_since_rest, 30);
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        for _ in 0..30 {
            tick(&mut room, &mut status, &mut [&mut steve], &night(101));
        }
        assert_eq!(steve.time_since_rest, 0, "it does not count in bed");
        get_up(&mut room, &registries, &mut steve, false);
        for _ in 0..5 {
            tick(&mut room, &mut status, &mut [&mut steve], &night(101));
        }
        assert_eq!(steve.time_since_rest, 5);
        steve.dead = true;
        tick(&mut room, &mut status, &mut [&mut steve], &night(101));
        assert_eq!(steve.time_since_rest, 0);
        steve.dead = false;
        steve.time_since_rest = i32::MAX;
        tick(&mut room, &mut status, &mut [&mut steve], &night(101));
        assert_eq!(steve.time_since_rest, i32::MAX);
    }

    #[test]
    fn an_occupied_bed_with_a_sleeping_villager_wakes_the_villager_instead_of_complaining() {
        use minecraftoss_entities::villager::Villager;
        use minecraftoss_entities::world::EntityWorld;
        let mut world = EntityWorld::default();
        let id = world.spawn_villager(Villager::new(glam::DVec3::new(0.5, 64.6875, 1.5)), true);
        assert!(!crate::entities::villager_asleep_in(&world, (0, 64, 1)));
        assert!(!world.wake_villager_in((0, 64, 1)), "an awake villager");
        world.villager_mut(id).unwrap().sleeping = Some((0, 64, 1));
        assert!(crate::entities::villager_asleep_in(&world, (0, 64, 1)));
        assert!(!crate::entities::villager_asleep_in(&world, (3, 64, 1)));
        assert!(world.wake_villager_in((0, 64, 1)));
        assert!(world.villagers()[0].wake_pending);
    }

    #[test]
    fn a_player_saved_in_bed_wakes_beside_it_on_join() {
        let Some(registries) = registries() else {
            return;
        };
        let mut room = Room::new(&registries);
        let bed = room.bed("minecraft:red_bed", (0, 64, 0), "south");
        let mut steve = player(0, [0.5, 64.6875, 1.5]);
        assert!(lie_down(&mut room, &registries, &mut steve, &bed));
        let saved = steve.data();
        assert_eq!(saved.sleeping_pos, Some((0, 64, 1)));
        let mut joined = player(0, [0.5, 64.6875, 1.5]);
        joined.saved_in_bed = saved.sleeping_pos;
        joined.sleep_timer = 40;
        let mut status = SleepStatus::default();
        room.unloaded = true;
        tick(&mut room, &mut status, &mut [&mut joined], &night(100));
        assert_eq!(
            joined.saved_in_bed,
            Some((0, 64, 1)),
            "it waits for the bed's chunk"
        );
        assert_eq!(
            joined.data().sleeping_pos,
            Some((0, 64, 1)),
            "and is saved again meanwhile"
        );
        assert_eq!(room.occupied((0, 64, 1)), Some("true"));
        room.unloaded = false;
        joined.arrival = None;
        let (_, events) = tick(&mut room, &mut status, &mut [&mut joined], &night(100));
        assert!(events.messages.is_empty());
        assert_eq!((joined.saved_in_bed, joined.sleeping), (None, None));
        assert_eq!(joined.sleep_timer, 0);
        assert_eq!(room.occupied((0, 64, 1)), Some("false"));
        assert_eq!(room.occupied((0, 64, 0)), Some("false"));
        let place = joined.arrival.unwrap();
        assert_eq!(place.feet[1], 64.0);
        assert!(
            (place.feet[0] - 0.5).abs() >= 1.0 || (place.feet[2] - 1.0).abs() > 1.0,
            "{:?}",
            place.feet
        );
        assert_eq!(
            place.yaw,
            crate::respawn_block::look_at_yaw(place.feet, (0, 64, 1))
        );
        assert_eq!(joined.data().sleeping_pos, None);
        let straw = room.bed("minecraft:straw_bed", (3, 64, 0), "east");
        let mut alex = player(1, [4.5, 64.375, 0.5]);
        alex.saved_in_bed = Some(straw.head);
        tick(&mut room, &mut status, &mut [&mut alex], &night(100));
        assert_eq!(
            room.state_at(straw.head),
            BlockStateId::AIR,
            "a left straw bed breaks"
        );
        let mut gone = player(2, [9.5, 64.0, 9.5]);
        gone.saved_in_bed = Some((9, 64, 9));
        tick(&mut room, &mut status, &mut [&mut gone], &night(100));
        assert_eq!(
            (gone.saved_in_bed, gone.arrival),
            (None, None),
            "no bed: nobody moves"
        );
    }
}
