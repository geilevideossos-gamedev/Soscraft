use std::collections::BTreeMap;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Instant;

use minecraft_commands::CommandServer;
use minecraft_commands::server::Profile;
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::game_mode::{GameMode, PlayerMode, from_id, parse};
use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::level_storage::{RespawnData, WeatherData};
use minecraft_terrain::server::{ClockTicks, OVERWORLD_CLOCK, THE_END_CLOCK};
use minecraftoss_core::Registries;
use minecraftoss_core::random::LegacyRandom;
use minecraftoss_player::Difficulty;
use minecraftoss_player::crafting::RecipeBook;

use crate::data::GameData;
use crate::entities::Entities;
use crate::level::{Level, Terrain};
use crate::player::{Player, joining_inventory};
use crate::player_nbt::{Place, PlayerData};
use crate::save::{WorldClocks, WorldSave};
use crate::survival::Survival;

pub struct Server {
    pub globals: Globals,
    pub levels: BTreeMap<Dimension, Level>,
    pub opening: BTreeMap<Dimension, LevelLoader>,
    pub players: BTreeMap<PlayerUuid, Player>,
}

pub struct Globals {
    pub seed: i64,
    pub registries: Arc<Registries>,
    pub save: Option<WorldSave>,
    pub game_rules: GameRules,
    pub difficulty: Difficulty,
    pub spawn: RespawnData,
    pub day: DayCycle,
    pub end_clock: DayCycle,
    pub weather: Option<WeatherData>,
    pub weather_random: LegacyRandom,
    pub chat: CommandServer,
    pub data: GameData,
    pub level_writer: Option<JoinHandle<()>>,
    pub autosave_ticks: u32,
    pub level_ticks: u32,
    pub closing: Vec<(Dimension, JoinHandle<()>)>,
}

impl Globals {
    pub fn clock_ticks(&self) -> ClockTicks {
        ClockTicks {
            overworld: self.day.ticks.floor() as i64,
            the_end: self.end_clock.ticks.floor() as i64,
        }
    }

    pub fn default_clock_ticks(&self, dimension: Dimension) -> i64 {
        let clocks = self.clock_ticks();
        match default_clock(dimension) {
            Some(THE_END_CLOCK) => clocks.the_end,
            _ => clocks.overworld,
        }
    }

    pub fn advance_weather(&mut self, ticking: &[Dimension], ticks: u32) {
        if ticks == 0
            || ticking.iter().any(|&dimension| has_weather(dimension))
            || !self.game_rules.bool("advance_weather")
        {
            return;
        }
        let mut weather = self.weather.unwrap_or(NEW_WORLD_WEATHER);
        for _ in 0..ticks {
            minecraft_terrain::server::advance_weather_timers(&mut weather, &mut self.weather_random);
        }
        self.weather = Some(weather);
    }

    pub fn close_level(&mut self, mut level: Level) {
        let dimension = level.dimension;
        let closing = std::thread::Builder::new()
            .name("minecraft-level-save".into())
            .spawn(move || {
                let started = Instant::now();
                if let Some(entities) = level.entities.as_mut() {
                    crate::level::save_server_block_entities(entities, &mut level.terrain, true);
                }
                crate::level::save_block_entities(&mut level.containers, &mut level.terrain);
                drop(level.entities.take());
                drop(level);
                diag::info!(
                    World,
                    "Minecraft {} level saved and closed in {} ms",
                    dimension.dimension_type(),
                    started.elapsed().as_millis()
                );
            });
        match closing {
            Ok(closing) => self.closing.push((dimension, closing)),
            Err(error) => diag::warn!(World, "Minecraft level save thread not started: {error}"),
        }
    }

    pub fn advance_clocks(&mut self, seconds: f64) {
        let advancing = self.game_rules.bool("advance_time");
        if advancing && !self.day.paused {
            self.day.advance(seconds);
        } else if let Some(save) = self.save.as_mut() {
            save.shift_day(-seconds * 20.0);
        }
        if advancing {
            self.end_clock.advance(seconds);
        }
    }
}

pub const EMPTY_TIME: u32 = 300;

const NEW_WORLD_WEATHER: WeatherData = WeatherData {
    clear_weather_time: 0,
    rain_time: 0,
    thunder_time: 0,
    raining: false,
    thundering: false,
};

pub fn has_weather(dimension: Dimension) -> bool {
    dimension == Dimension::Overworld
}

pub fn weather_random(seed: i64) -> LegacyRandom {
    LegacyRandom::new(seed ^ 0x0057_4541_5448_4552)
}

pub fn default_clock(dimension: Dimension) -> Option<&'static str> {
    match dimension {
        Dimension::Overworld => Some(OVERWORLD_CLOCK),
        Dimension::Nether => None,
        Dimension::End => Some(THE_END_CLOCK),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arrival {
    pub feet: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub motion: [f64; 3],
}

impl Arrival {
    pub fn at(feet: [f64; 3]) -> Self {
        Self {
            feet,
            yaw: 0.0,
            pitch: 0.0,
            motion: [0.0; 3],
        }
    }

    pub fn of(place: &Place) -> Self {
        Self {
            feet: place.feet,
            yaw: place.yaw,
            pitch: place.pitch,
            motion: place.motion,
        }
    }

    pub fn place(self, dimension: Dimension) -> Place {
        Place {
            motion: self.motion,
            on_ground: self.motion == [0.0; 3],
            ..Place::standing(dimension, self.feet, self.yaw, self.pitch)
        }
    }
}

type Opened = Result<(Terrain, Arrival), String>;

pub struct LevelLoader {
    result: mpsc::Receiver<Opened>,
    thread: Option<JoinHandle<()>>,
}

impl LevelLoader {
    pub fn open<F>(
        globals: &mut Globals,
        dimension: Dimension,
        view_distance: i32,
        known_spawn: Option<(i32, i32, i32)>,
        prepare: F,
    ) -> Self
    where
        F: FnOnce(&mut Terrain) -> Arrival + Send + 'static,
    {
        let registries = globals.registries.clone();
        let seed = globals.seed;
        let save = globals.save.clone();
        let (closing, open): (Vec<_>, Vec<_>) = std::mem::take(&mut globals.closing)
            .into_iter()
            .partition(|(closed, _)| *closed == dimension);
        globals.closing = open;
        let (send, result) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("minecraft-level-load".into())
            .spawn(move || {
                for (_, save) in closing {
                    let _ = save.join();
                }
                let started = Instant::now();
                let opened = Terrain::open(
                    registries,
                    seed,
                    view_distance,
                    dimension,
                    save.as_ref(),
                    known_spawn,
                )
                .map(|mut terrain| {
                    let arrival = prepare(&mut terrain);
                    (terrain, arrival)
                });
                if opened.is_ok() {
                    diag::info!(
                        World,
                        "Minecraft {} level opened in {} ms",
                        dimension.dimension_type(),
                        started.elapsed().as_millis()
                    );
                }
                let _ = send.send(opened);
            })
            .map_err(|error| diag::warn!(World, "Minecraft level thread not started: {error}"))
            .ok();
        Self { result, thread }
    }

    pub fn poll(&mut self) -> Option<Opened> {
        match self.result.try_recv() {
            Ok(opened) => Some(opened),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("the level loader stopped".to_owned()))
            }
        }
    }
}

impl Drop for LevelLoader {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn start_level(globals: &Globals, level: &mut Level) {
    let mut entities = Entities::new(
        &level.terrain.stream,
        level.dimension,
        globals.seed,
        &globals.data,
        globals.weather.as_ref(),
    );
    entities.set_game_rules(&globals.game_rules);
    level.containers.set_book(globals.data.stations.clone());
    level.entities = Some(entities);
}

fn block_of(feet: [f64; 3]) -> (i32, i32, i32) {
    (
        feet[0].floor() as i32,
        feet[1].floor() as i32,
        feet[2].floor() as i32,
    )
}

fn spawn_place(spawn: &RespawnData) -> Place {
    let (x, y, z) = spawn.pos;
    Place::standing(
        crate::save::spawn_dimension(spawn),
        [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5],
        spawn.yaw,
        spawn.pitch,
    )
}

impl Server {
    pub fn known_spawn(&self, dimension: Dimension, arrival: Option<Place>) -> Option<(i32, i32, i32)> {
        match dimension {
            Dimension::Overworld => (crate::save::spawn_dimension(&self.globals.spawn)
                == Dimension::Overworld)
                .then_some(self.globals.spawn.pos),
            _ => arrival
                .filter(|place| place.dimension == dimension)
                .map(|place| block_of(place.feet)),
        }
    }

    pub fn prepare_respawns(&mut self) {
        for player in self.players.values_mut() {
            if player.dead && player.respawning.is_none() {
                let point = crate::respawn::respawn_point(player);
                player.respawning = Some(crate::respawn::respawn_dimension(
                    &self.globals.spawn,
                    point.as_ref(),
                ));
            }
        }
    }

    pub fn open_levels(&mut self, view_distance: i32) {
        let respawns = self.players.values().filter_map(|player| {
            let dimension = player.respawning?;
            let point = crate::respawn::respawn_point(player)
                .filter(|point| point.at.dimension == dimension)
                .map(|point| {
                    let (x, y, z) = point.at.pos;
                    Place::standing(
                        dimension,
                        [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5],
                        0.0,
                        0.0,
                    )
                });
            Some((dimension, point))
        });
        let waiting: Vec<(Dimension, Option<Place>)> = self
            .players
            .values()
            .map(|player| {
                let arrival = player.arrival.filter(|place| place.dimension == player.dimension);
                (player.dimension, arrival)
            })
            .chain(respawns)
            .filter(|(dimension, _)| !self.levels.contains_key(dimension))
            .collect();
        for (dimension, arrival) in waiting {
            if self.opening.contains_key(&dimension) {
                continue;
            }
            let known_spawn = self.known_spawn(dimension, arrival);
            let loader = LevelLoader::open(
                &mut self.globals,
                dimension,
                view_distance,
                known_spawn,
                move |terrain| match arrival {
                    Some(place) => Arrival::of(&place),
                    None => {
                        let (x, y, z) = terrain.stream.player_spawn;
                        Arrival::at([x, y, z])
                    }
                },
            );
            self.opening.insert(dimension, loader);
        }
    }

    pub fn finish_opening(&mut self) -> Vec<(Dimension, Result<(), String>)> {
        let ready: Vec<(Dimension, Opened)> = self
            .opening
            .iter_mut()
            .filter_map(|(&dimension, loader)| loader.poll().map(|opened| (dimension, opened)))
            .collect();
        let mut finished = Vec::new();
        for (dimension, opened) in ready {
            self.opening.remove(&dimension);
            let waiting = self
                .players
                .values_mut()
                .filter(|player| player.dimension == dimension);
            match opened {
                Ok((terrain, arrival)) => {
                    let mut level = Level::new(dimension, terrain);
                    start_level(&self.globals, &mut level);
                    self.levels.insert(dimension, level);
                    for player in waiting {
                        if player.place.is_none_or(|place| place.dimension != dimension) {
                            player.arrive(arrival.place(dimension));
                        }
                    }
                    finished.push((dimension, Ok(())));
                }
                Err(error) => {
                    diag::warn!(
                        World,
                        "Minecraft {} level failed to open: {error}",
                        dimension.dimension_type()
                    );
                    let spawn = spawn_place(&self.globals.spawn);
                    for player in waiting {
                        let back = player
                            .place
                            .filter(|place| place.dimension != dimension)
                            .unwrap_or(spawn);
                        player.arrive(back);
                        if let Some(switch) = player.switching.as_mut() {
                            switch.to = back.dimension;
                            switch.arrival = Some(back);
                        }
                    }
                    finished.push((dimension, Err(error)));
                }
            }
        }
        finished
    }

    pub fn follow_weather_commands(&mut self) {
        let mut commanded = None;
        for (&dimension, level) in &mut self.levels {
            if let Some(weather) = level
                .entities
                .as_mut()
                .and_then(|entities| entities.world_weather.take())
            {
                commanded = Some((dimension, weather));
            }
        }
        let Some((from, weather)) = commanded else {
            return;
        };
        self.globals.weather = Some(weather);
        for (&dimension, level) in &mut self.levels {
            if dimension != from
                && has_weather(dimension)
                && let Some(entities) = level.entities.as_mut()
            {
                entities.set_weather_data(weather);
            }
        }
    }

    pub fn close_empty_levels(&mut self, ticks: u32) -> Vec<Dimension> {
        let occupied: Vec<Dimension> = self
            .players
            .values()
            .flat_map(|player| std::iter::once(player.dimension).chain(player.respawning))
            .collect();
        let keeps_empty_levels = self.globals.save.is_none();
        let mut closing = Vec::new();
        for (&dimension, level) in &mut self.levels {
            if occupied.contains(&dimension) {
                if level.empty_ticks > 0
                    && has_weather(dimension)
                    && let (Some(entities), Some(weather)) =
                        (level.entities.as_mut(), self.globals.weather)
                {
                    entities.set_weather_data(weather);
                }
                level.empty_ticks = 0;
                continue;
            }
            level.empty_ticks = level.empty_ticks.saturating_add(ticks);
            if level.empty_ticks >= EMPTY_TIME && !keeps_empty_levels {
                closing.push(dimension);
            }
        }
        for dimension in &closing {
            if let Some(level) = self.levels.remove(dimension) {
                self.globals.close_level(level);
            }
        }
        closing
    }

    pub fn save_and_close(mut self, started: Instant) {
        let save = self.globals.save.take();
        if let Some(writer) = self.globals.level_writer.take() {
            let _ = writer.join();
        }
        for (_, closing) in self.globals.closing.drain(..) {
            let _ = closing.join();
        }
        let files = WorldFiles::of(&self.globals, &self.players);
        for level in self.levels.values_mut() {
            if let Some(entities) = level.entities.as_mut() {
                crate::level::save_server_block_entities(entities, &mut level.terrain, true);
            }
            crate::level::save_block_entities(&mut level.containers, &mut level.terrain);
        }
        drop(self);
        let Some(save) = save else {
            return;
        };
        files.write(&save, "Minecraft");
        diag::info!(
            World,
            "Minecraft world saved in {} ms: {}",
            started.elapsed().as_millis(),
            save.path().display()
        );
    }
}

pub fn starting_game_mode(saved: Option<PlayerMode>, chosen: Option<&str>) -> PlayerMode {
    let chosen = chosen.and_then(|name| {
        let name = name.trim().to_ascii_lowercase();
        parse(&name).or_else(|| name.parse().ok().and_then(from_id))
    });
    let mut mode = saved.unwrap_or_else(|| PlayerMode::new(GameMode::Survival));
    if let Some(chosen) = chosen {
        mode.set(chosen);
    }
    mode
}

pub struct Joining<'a> {
    pub profile: &'a Profile,
    pub spawn: [f64; 3],
    pub chosen_mode: Option<&'a str>,
    pub kit: Option<&'a str>,
    pub carried: Option<Survival>,
}

pub fn join(
    save: Option<&WorldSave>,
    recipes: &Arc<RecipeBook>,
    rules: &GameRules,
    joining: Joining,
) -> Player {
    let Joining {
        profile,
        spawn,
        chosen_mode,
        kit,
        carried,
    } = joining;
    let saved = match save.map(|save| save.read_player(&profile.uuid, recipes)) {
        Some(Ok(Some(saved))) => saved,
        Some(Err(error)) => {
            diag::warn!(
                World,
                "Minecraft player file unreadable, starting it again: {error}"
            );
            PlayerData::default()
        }
        Some(Ok(None)) | None => PlayerData::default(),
    };
    let mode = starting_game_mode(
        saved.mode.or_else(|| save.map(WorldSave::default_mode)),
        chosen_mode,
    );
    let inventory = joining_inventory(recipes.clone(), saved.inventory, kit);
    let dead = saved.stats.is_some_and(|stats| stats.status.health <= 0.0);
    let survival = match (saved.stats, carried) {
        (Some(stats), _) => {
            let mut survival = Survival::default();
            survival.restore(stats.status, stats.fire, stats.score);
            if dead {
                if crate::death::keeps_belongings(rules.keep_inventory(), mode.mode) {
                    survival.respawn_keeping_experience();
                } else {
                    survival.respawn();
                }
            }
            survival
        }
        (None, Some(carried)) => carried,
        (None, None) => Survival::default(),
    };
    let mut player = Player::new(profile, mode, survival, inventory);
    player.selected = saved.selected;
    if let Some(ender) = saved.ender {
        player.ender = ender;
    }
    player.respawn = saved.respawn;
    player.last_death = saved.last_death;
    player.portal_cooldown = saved.portal_cooldown.unwrap_or(0);
    player.seen_credits = saved.seen_credits.unwrap_or(false);
    player.dead = dead;
    player.place = Some(Place::standing(Dimension::Overworld, spawn, 0.0, 0.0));
    match saved.place {
        Some(place) if !dead => player.arrive(place),
        _ => {}
    }
    player
}

pub fn debug_dimension(value: &str) -> Result<Option<Dimension>, String> {
    let name = value.trim().to_ascii_lowercase();
    if name.is_empty() {
        return Ok(None);
    }
    Dimension::parse(&name).map(Some).ok_or_else(|| {
        format!(
            "Minecraft dimension `{}` unknown: IW4L_MINECRAFT_DIMENSION takes overworld, the_nether or the_end",
            value.trim()
        )
    })
}

pub fn start_in(player: &mut Player, dimension: Dimension, overworld_spawn: [f64; 3]) {
    if player.dimension == dimension {
        return;
    }
    if dimension == Dimension::Overworld {
        player.arrive(Place::standing(dimension, overworld_spawn, 0.0, 0.0));
        return;
    }
    player.dimension = dimension;
    player.arrival = None;
}

struct WorldFiles {
    spawn: RespawnData,
    clocks: WorldClocks,
    rules: GameRules,
    difficulty: Difficulty,
    weather: Option<WeatherData>,
    players: Vec<(String, PlayerData)>,
}

impl WorldFiles {
    fn of(globals: &Globals, players: &BTreeMap<PlayerUuid, Player>) -> Self {
        Self {
            spawn: globals.spawn.clone(),
            clocks: WorldClocks {
                overworld: globals.day,
                the_end: globals.end_clock,
            },
            rules: globals.game_rules.clone(),
            difficulty: globals.difficulty,
            weather: globals.weather,
            players: players
                .values()
                .map(|player| (player.uuid.0.clone(), player.data()))
                .collect(),
        }
    }

    fn write(&self, save: &WorldSave, prefix: &str) {
        if let Err(error) = save.write(&self.spawn, Some(self.clocks), &self.rules, self.difficulty) {
            diag::warn!(World, "{prefix} level.dat not written: {error}");
        }
        if let Some(weather) = self.weather
            && let Err(error) = save.write_weather(&weather)
        {
            diag::warn!(World, "{prefix} weather.dat not written: {error}");
        }
        write_players(
            save,
            &self.players,
            &format!("{prefix} player file not written"),
        );
    }
}

fn write_players(save: &WorldSave, players: &[(String, PlayerData)], failed: &str) {
    for (uuid, data) in players {
        if let Err(error) = save.write_player(uuid, data) {
            diag::warn!(World, "{failed}: {error}");
        }
    }
}

pub fn save_player(globals: &mut Globals, player: &Player) {
    let Some(save) = globals.save.clone() else {
        return;
    };
    if let Some(previous) = globals.level_writer.take() {
        let _ = previous.join();
    }
    let file = [(player.uuid.0.clone(), player.data())];
    globals.level_writer = std::thread::Builder::new()
        .name("minecraft-player-save".into())
        .spawn(move || write_players(&save, &file, "Minecraft player file not written"))
        .map_err(|error| diag::warn!(World, "Minecraft player file thread not started: {error}"))
        .ok();
}

pub fn autosave(
    globals: &mut Globals,
    levels: &mut BTreeMap<Dimension, Level>,
    players: &BTreeMap<PlayerUuid, Player>,
) {
    let Some(save) = globals.save.clone() else {
        return;
    };
    let started = Instant::now();
    let chunks: usize = levels
        .values_mut()
        .map(|level| {
            crate::level::save_block_entities(&mut level.containers, &mut level.terrain);
            level.terrain.stream.queue_save()
        })
        .sum();
    if let Some(previous) = globals.level_writer.take() {
        let _ = previous.join();
    }
    let path = save.path().display().to_string();
    let files = WorldFiles::of(globals, players);
    globals.level_writer = std::thread::Builder::new()
        .name("minecraft-level-save".into())
        .spawn(move || files.write(&save, "Minecraft autosave:"))
        .map_err(|error| {
            diag::warn!(
                World,
                "Minecraft autosave: level.dat thread not started: {error}"
            )
        })
        .ok();
    diag::info!(
        World,
        "Minecraft autosave: {chunks} chunks queued for {path} in {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::inventory::ItemStack;
    use minecraftoss_player::survival::EffectKind;

    #[test]
    fn a_player_starts_in_survival_or_in_the_mode_their_file_saved() {
        assert_eq!(
            starting_game_mode(None, None),
            PlayerMode::new(GameMode::Survival)
        );
        let mut saved = PlayerMode::new(GameMode::Creative);
        saved.abilities.flying = true;
        assert_eq!(starting_game_mode(Some(saved), None), saved);
        assert_eq!(starting_game_mode(Some(saved), Some("nonsense")), saved);
    }

    #[test]
    fn a_chosen_game_mode_by_name_or_id_overrides_the_saved_one() {
        assert_eq!(
            starting_game_mode(None, Some(" Creative ")).mode,
            GameMode::Creative
        );
        assert_eq!(
            starting_game_mode(None, Some("3")).mode,
            GameMode::Spectator
        );
        let chosen =
            starting_game_mode(Some(PlayerMode::new(GameMode::Creative)), Some("adventure"));
        assert_eq!(chosen.mode, GameMode::Adventure);
        assert_eq!(chosen.previous, Some(GameMode::Creative));
        assert_eq!(
            chosen.abilities,
            PlayerMode::new(GameMode::Adventure).abilities
        );
        let same = starting_game_mode(Some(PlayerMode::new(GameMode::Creative)), Some("1"));
        assert_eq!(same, PlayerMode::new(GameMode::Creative));
    }

    struct TestDir(std::path::PathBuf);

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn world(name: &str) -> (TestDir, WorldSave) {
        let dir = std::env::temp_dir().join(format!(
            "minecraft_survival-server-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let save = crate::save::open_world(&dir, None, Some("1"), 2)
            .save
            .expect("a world folder");
        (TestDir(dir), save)
    }

    fn profile(name: &str) -> Profile {
        let mut chat = CommandServer::new(crate::commands::HOST);
        chat.join(crate::commands::HOST, name);
        chat.profile(crate::commands::HOST).unwrap().clone()
    }

    fn held(player: &Player, slot: usize) -> Option<(&str, u8)> {
        player.inventory.slots[slot]
            .as_ref()
            .map(|stack| (stack.id.as_str(), stack.count))
    }

    const SPAWN: [f64; 3] = [8.5, 64.0, -3.5];

    fn joining(profile: &Profile) -> Joining<'_> {
        Joining {
            profile,
            spawn: SPAWN,
            chosen_mode: None,
            kit: None,
            carried: None,
        }
    }

    fn with_kit<'a>(profile: &'a Profile, kit: &'a str) -> Joining<'a> {
        Joining {
            kit: Some(kit),
            ..joining(profile)
        }
    }

    #[test]
    fn a_joining_player_gets_the_inventory_and_mode_their_file_saved() {
        let (_dir, save) = world("join");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(player.mode, PlayerMode::new(GameMode::Survival));
        assert!(player.inventory.slots.iter().all(Option::is_none));
        player.mode = PlayerMode::new(GameMode::Creative);
        player.inventory.slots[2] = Some(ItemStack::new("minecraft:diamond", 3));
        player.inventory.slots[38] = Some(ItemStack::new("minecraft:iron_chestplate", 1));
        player.selected = 2;
        save.write_player(&player.uuid.0, &player.data()).unwrap();
        let again = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(again.mode, PlayerMode::new(GameMode::Creative));
        assert_eq!(again.selected, 2);
        assert_eq!(held(&again, 2), Some(("minecraft:diamond", 3)));
        assert_eq!(held(&again, 38), Some(("minecraft:iron_chestplate", 1)));
        assert!(Arc::ptr_eq(&again.inventory.recipes, &recipes));
        let alex = profile("Alex");
        let alex = join(
            Some(&save),
            &recipes,
            &rules,
            Joining {
                chosen_mode: Some("adventure"),
                ..joining(&alex)
            },
        );
        assert_eq!(alex.mode.mode, GameMode::Adventure);
        assert!(alex.inventory.slots.iter().all(Option::is_none));
    }

    #[test]
    fn a_mode_only_player_file_still_gets_the_kit_and_a_saved_inventory_does_not() {
        let (_dir, save) = world("kit");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mode_only = PlayerData {
            mode: Some(PlayerMode::new(GameMode::Creative)),
            ..PlayerData::default()
        };
        save.write_player(&steve.uuid, &mode_only).unwrap();
        let first = join(Some(&save), &recipes, &rules, with_kit(&steve, "dirt*5"));
        assert_eq!(first.mode.mode, GameMode::Creative);
        assert_eq!(held(&first, 9), Some(("minecraft:dirt", 5)));
        let mut emptied = first;
        emptied.inventory.slots.fill(None);
        save.write_player(&steve.uuid, &emptied.data()).unwrap();
        let again = join(Some(&save), &recipes, &rules, with_kit(&steve, "dirt*5"));
        assert!(again.inventory.slots.iter().all(Option::is_none));
        let alex = profile("Alex");
        let alex = join(Some(&save), &recipes, &rules, with_kit(&alex, "dirt*5"));
        assert_eq!(held(&alex, 9), Some(("minecraft:dirt", 5)));
    }

    #[test]
    fn a_joining_player_gets_back_the_health_hunger_and_experience_they_saved() {
        let (_dir, save) = world("stats");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        player.survival.observe(100, 100);
        player.survival.status.health = 6.5;
        player.survival.status.food.level = 9;
        player.survival.status.experience_level = 12;
        player.survival.score = 250;
        player
            .survival
            .status
            .add_effect(EffectKind::Regeneration, 400, 1);
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let mut carried = Survival::default();
        carried.status.health = 2.0;
        let carrying = Joining {
            carried: Some(carried),
            ..joining(&steve)
        };
        let mut again = join(Some(&save), &recipes, &rules, carrying);
        assert_eq!(again.survival.status.health, 6.5);
        assert_eq!(again.survival.status.food.level, 9);
        assert_eq!(again.survival.status.experience_level, 12);
        assert_eq!(again.survival.score, 250);
        assert!(
            again
                .survival
                .status
                .effect(EffectKind::Regeneration)
                .is_some()
        );
        let settle = again.survival.settle(100, 100);
        assert_eq!((settle.set, settle.damage), (Some(33), None));
        assert_eq!(again.survival.vitals().display_health, 7);
    }

    #[test]
    fn a_player_saved_dead_comes_back_with_full_health_and_nothing_carried() {
        let (_dir, save) = world("dead");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        player.survival.status.experience_level = 12;
        player.arrive(Place::standing(
            Dimension::Overworld,
            [40.5, 12.0, 40.5],
            90.0,
            0.0,
        ));
        player.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let mut carried = Survival::default();
        carried.status.health = 2.0;
        let carrying = Joining {
            carried: Some(carried),
            ..joining(&steve)
        };
        let mut again = join(Some(&save), &recipes, &rules, carrying);
        assert_eq!(again.survival.status.health, 20.0);
        assert_eq!(again.survival.status.experience_level, 0);
        assert!(again.dead);
        assert_eq!(again.arrival, None);
        assert_eq!(again.place.map(|place| place.feet), Some(SPAWN));
        assert_eq!(again.last_death.map(|death| death.pos), Some((40, 12, 40)));
        assert!(again.observe_life(0));
        assert!(!again.dead);
        let mut keeping = GameRules::default();
        keeping.set("keep_inventory", "true").unwrap();
        let kept = join(Some(&save), &recipes, &keeping, joining(&steve));
        assert_eq!(kept.survival.status.health, 20.0);
        assert_eq!(kept.survival.status.experience_level, 12);
    }

    #[test]
    fn a_joining_player_arrives_where_they_were_saved_facing_the_same_way() {
        let (_dir, save) = world("place");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let fresh = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(fresh.arrival, None);
        assert_eq!(
            fresh.place,
            Some(Place::standing(Dimension::Overworld, SPAWN, 0.0, 0.0))
        );
        let mut player = fresh;
        let hill = Place {
            fall_distance: 1.5,
            on_ground: false,
            motion: [0.1, -0.2, 0.0],
            ..Place::standing(Dimension::Overworld, [120.25, 80.0, -64.75], -45.0, 30.0)
        };
        player.place = Some(hill);
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let again = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(again.arrival, Some(hill));
        assert_eq!(again.place, Some(hill));
        assert_eq!(again.dimension, Dimension::Overworld);
        let mut nether = again;
        nether.place = Some(Place::standing(
            Dimension::Nether,
            [5.0, 70.0, 5.0],
            0.0,
            0.0,
        ));
        save.write_player(&steve.uuid, &nether.data()).unwrap();
        let back = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(back.dimension, Dimension::Nether);
        assert_eq!(back.arrival, nether.place);
        assert_eq!(back.place, nether.place);
    }

    #[test]
    fn the_debug_dimension_names_a_dimension_with_or_without_its_namespace() {
        assert_eq!(debug_dimension("the_nether"), Ok(Some(Dimension::Nether)));
        assert_eq!(debug_dimension(" minecraft:the_end "), Ok(Some(Dimension::End)));
        assert_eq!(debug_dimension("Overworld"), Ok(Some(Dimension::Overworld)));
        assert_eq!(debug_dimension(""), Ok(None));
        let error = debug_dimension("the_moon").unwrap_err();
        assert!(error.contains("`the_moon`") && error.contains("IW4L_MINECRAFT_DIMENSION"), "{error}");
    }

    #[test]
    fn a_debug_start_keeps_a_saved_place_in_that_dimension_and_waits_for_its_level_otherwise() {
        let (_dir, save) = world("debug-start");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        let cave = Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], 90.0, 0.0);
        player.arrive(cave);
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let mut kept = join(Some(&save), &recipes, &rules, joining(&steve));
        start_in(&mut kept, Dimension::Nether, SPAWN);
        assert_eq!((kept.dimension, kept.arrival), (Dimension::Nether, Some(cave)));
        let mut end = join(Some(&save), &recipes, &rules, joining(&steve));
        start_in(&mut end, Dimension::End, SPAWN);
        assert_eq!((end.dimension, end.arrival), (Dimension::End, None));
        assert_eq!(end.place, Some(cave), "where to go back to if the End fails to open");
        let mut home = join(Some(&save), &recipes, &rules, joining(&steve));
        start_in(&mut home, Dimension::Overworld, SPAWN);
        let spawn = Place::standing(Dimension::Overworld, SPAWN, 0.0, 0.0);
        assert_eq!((home.dimension, home.arrival), (Dimension::Overworld, Some(spawn)));
    }

    #[test]
    fn the_respawn_point_comes_back_and_a_player_saved_dead_respawns_there() {
        let (_dir, save) = world("respawn");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        let point = crate::player_nbt::Respawn {
            at: crate::player_nbt::GlobalPos {
                dimension: Dimension::Overworld,
                pos: (30, 70, -30),
            },
            yaw: 45.0,
            pitch: 0.0,
            forced: true,
        };
        player.set_respawn(Some(point));
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let alive = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(alive.respawn, Some(point));
        assert!(!alive.dead);
        let mut dying = alive;
        dying.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        save.write_player(&steve.uuid, &dying.data()).unwrap();
        let mut back = join(Some(&save), &recipes, &rules, joining(&steve));
        assert!(back.dead);
        assert!(back.observe_life(0));
        assert!(!back.arrive_at_respawn(&[Dimension::Overworld], |_| Some(true), |_| None));
        assert_eq!(
            back.arrival.map(|place| (place.feet, place.yaw)),
            Some(([30.5, 70.0, -29.5], 45.0))
        );
    }

    #[test]
    fn each_player_gets_back_their_own_ender_chest() {
        let (_dir, save) = world("ender");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(player.ender, vec![None; crate::player_nbt::ENDER_SLOTS]);
        player.ender[3] = Some(ItemStack::new("minecraft:emerald", 9));
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let again = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(again.ender[3].as_ref().map(|stack| stack.count), Some(9));
        let alex = profile("Alex");
        let alex = join(Some(&save), &recipes, &rules, joining(&alex));
        assert!(alex.ender.iter().all(Option::is_none));
    }

    #[test]
    fn an_unreadable_player_file_starts_the_player_again() {
        let (_dir, save) = world("broken");
        let steve = profile("Steve");
        let path = save
            .path()
            .join("players")
            .join("data")
            .join(format!("{}.dat", steve.uuid));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"broken").unwrap();
        std::fs::write(minecraft_terrain::level_storage::old_path(&path), b"broken").unwrap();
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let kit = Joining {
            kit: Some("dirt"),
            ..joining(&steve)
        };
        let player = join(Some(&save), &recipes, &rules, kit);
        assert_eq!(player.mode, PlayerMode::new(GameMode::Survival));
        assert_eq!(held(&player, 9), Some(("minecraft:dirt", 1)));
    }

    fn registries() -> Option<Arc<Registries>> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        Registries::load(&paths).ok().map(Arc::new)
    }

    fn test_globals(registries: Arc<Registries>, save: Option<WorldSave>) -> Globals {
        Globals {
            seed: 1,
            registries,
            save,
            game_rules: GameRules::default(),
            difficulty: Difficulty::Normal,
            spawn: RespawnData::default(),
            day: DayCycle::default(),
            end_clock: crate::save::END_CLOCK_START,
            weather: None,
            weather_random: weather_random(1),
            chat: CommandServer::new(crate::commands::HOST),
            data: GameData::default(),
            level_writer: None,
            autosave_ticks: 0,
            level_ticks: 0,
            closing: Vec::new(),
        }
    }

    #[test]
    fn the_weather_keeps_changing_while_no_level_that_has_weather_ticks() {
        assert!(has_weather(Dimension::Overworld));
        assert!(!has_weather(Dimension::Nether));
        assert!(!has_weather(Dimension::End));
        let Some(registries) = registries() else {
            return;
        };
        let rain = WeatherData {
            rain_time: 3,
            raining: true,
            ..WeatherData::default()
        };
        let mut globals = Globals {
            weather: Some(rain),
            ..test_globals(registries.clone(), None)
        };
        let rain_left = |globals: &Globals| globals.weather.map(|w| (w.rain_time, w.raining));
        globals.advance_weather(&[Dimension::Overworld], 10);
        assert_eq!(globals.weather, Some(rain));
        globals.advance_weather(&[Dimension::Nether], 2);
        assert_eq!(rain_left(&globals), Some((1, true)));
        globals.game_rules.set("advance_weather", "false").unwrap();
        globals.advance_weather(&[Dimension::End], 5);
        assert_eq!(rain_left(&globals), Some((1, true)));
        globals.game_rules.set("advance_weather", "true").unwrap();
        globals.advance_weather(&[], 1);
        assert_eq!(rain_left(&globals), Some((0, false)));
        let mut fresh = test_globals(registries, None);
        fresh.advance_weather(&[Dimension::Nether, Dimension::End], 1);
        let drawn = fresh.weather.expect("a fresh world starts its weather");
        assert!(
            (12_000..=180_000).contains(&drawn.rain_time) && !drawn.raining,
            "{drawn:?}"
        );
    }

    #[test]
    fn the_end_clock_runs_beside_the_day_and_each_level_reads_its_default_clock() {
        assert_eq!(default_clock(Dimension::Overworld), Some("minecraft:overworld"));
        assert_eq!(default_clock(Dimension::Nether), None);
        assert_eq!(default_clock(Dimension::End), Some("minecraft:the_end"));
        let Some(registries) = registries() else {
            return;
        };
        let mut globals = test_globals(registries, None);
        assert_eq!(
            globals.clock_ticks(),
            ClockTicks {
                overworld: 1000,
                the_end: 0
            }
        );
        globals.advance_clocks(1.0);
        assert_eq!(
            globals.clock_ticks(),
            ClockTicks {
                overworld: 1020,
                the_end: 20
            }
        );
        assert_eq!(globals.default_clock_ticks(Dimension::Overworld), 1020);
        assert_eq!(globals.default_clock_ticks(Dimension::Nether), 1020);
        globals.end_clock.ticks = 5.0;
        assert_eq!(globals.default_clock_ticks(Dimension::End), 5);
        globals.end_clock.paused = true;
        globals.advance_clocks(1.0);
        assert_eq!(
            globals.clock_ticks(),
            ClockTicks {
                overworld: 1040,
                the_end: 5
            }
        );
        globals.end_clock.paused = false;
        globals.game_rules.set("advance_time", "false").unwrap();
        globals.advance_clocks(1.0);
        assert_eq!(
            globals.clock_ticks(),
            ClockTicks {
                overworld: 1040,
                the_end: 5
            }
        );
    }

    #[test]
    fn closing_with_no_overworld_loaded_still_writes_level_dat_and_the_player_files() {
        let Some(registries) = registries() else {
            return;
        };
        let (dir, save) = world("no-overworld");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        let nether = Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], 90.0, 0.0);
        player.place = Some(nether);
        player.inventory.slots[0] = Some(ItemStack::new("minecraft:netherrack", 7));
        let spawn = RespawnData {
            yaw: 45.0,
            ..crate::save::world_spawn(Dimension::Nether, (4, 70, 4))
        };
        let weather = WeatherData {
            rain_time: 900,
            raining: true,
            ..WeatherData::default()
        };
        let globals = Globals {
            difficulty: Difficulty::Hard,
            spawn: spawn.clone(),
            day: DayCycle {
                ticks: 13_000.0,
                paused: false,
            },
            end_clock: DayCycle {
                ticks: 77.0,
                paused: true,
            },
            weather: Some(weather),
            ..test_globals(registries, Some(save))
        };
        let uuid = player.uuid.clone();
        let server = Server {
            globals,
            levels: BTreeMap::new(),
            opening: BTreeMap::new(),
            players: BTreeMap::from([(uuid.clone(), player)]),
        };
        server.save_and_close(Instant::now());
        let opened = crate::save::open_world(&dir.0, None, None, 2);
        assert!(opened.warnings.is_empty(), "{:?}", opened.warnings);
        assert_eq!(opened.spawn, Some(spawn));
        assert_eq!(opened.day_ticks, Some(13_000.0));
        assert_eq!((opened.end_clock.ticks, opened.end_clock.paused), (77.0, true));
        assert_eq!(opened.difficulty, Difficulty::Hard);
        assert_eq!(opened.weather, Some(weather));
        let save = opened.save.unwrap();
        let saved = save.read_player(&uuid.0, &recipes).unwrap().unwrap();
        assert_eq!(saved.place, Some(nether));
        let back = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(held(&back, 0), Some(("minecraft:netherrack", 7)));
    }

    #[test]
    fn a_temporary_world_gives_the_kit_and_the_survival_carried_over() {
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut carried = Survival::default();
        carried.observe(100, 100);
        carried.status.health = 7.0;
        let steve = profile("Steve");
        let player = join(
            None,
            &recipes,
            &rules,
            Joining {
                chosen_mode: Some("creative"),
                kit: Some("dirt*2"),
                carried: Some(carried),
                ..joining(&steve)
            },
        );
        assert_eq!(player.survival.status.health, 7.0);
        assert_eq!(player.mode.mode, GameMode::Creative);
        assert_eq!(held(&player, 9), Some(("minecraft:dirt", 2)));
    }

    fn wait_for_levels(server: &mut Server) -> Vec<(Dimension, Result<(), String>)> {
        let started = Instant::now();
        let mut finished = Vec::new();
        while !server.opening.is_empty() {
            assert!(
                started.elapsed().as_secs() < 600,
                "levels still opening after ten minutes"
            );
            finished.extend(server.finish_opening());
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        finished
    }

    fn server_with(globals: Globals, players: Vec<Player>) -> Server {
        Server {
            globals,
            levels: BTreeMap::new(),
            opening: BTreeMap::new(),
            players: players
                .into_iter()
                .map(|player| (player.uuid.clone(), player))
                .collect(),
        }
    }

    #[test]
    fn a_level_opens_on_its_own_thread_for_the_players_in_its_dimension() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("open-levels");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        let cave = Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], 90.0, 10.0);
        steve.arrive(cave);
        let mut alex = join(Some(&save), &recipes, &rules, joining(&profile("Alex")));
        alex.dimension = Dimension::End;
        let (steve_id, alex_id) = (steve.uuid.clone(), alex.uuid.clone());
        let mut server = server_with(
            test_globals(registries, Some(save)),
            vec![steve, alex],
        );
        assert_eq!(server.known_spawn(Dimension::Nether, Some(cave)), Some((5, 70, 5)));
        assert_eq!(server.known_spawn(Dimension::End, Some(cave)), None);
        assert_eq!(
            server.known_spawn(Dimension::Overworld, Some(cave)),
            Some(server.globals.spawn.pos)
        );
        server.open_levels(2);
        assert_eq!(
            server.opening.keys().copied().collect::<Vec<_>>(),
            vec![Dimension::Nether, Dimension::End]
        );
        server.open_levels(2);
        assert_eq!(server.opening.len(), 2, "a level opening is not opened again");
        let mut finished = wait_for_levels(&mut server);
        finished.sort_by_key(|(dimension, _)| *dimension);
        assert_eq!(
            finished,
            vec![(Dimension::Nether, Ok(())), (Dimension::End, Ok(()))]
        );
        for dimension in [Dimension::Nether, Dimension::End] {
            let level = &server.levels[&dimension];
            assert_eq!(level.dimension, dimension);
            assert!(level.entities.is_some(), "{dimension:?} has its mobs");
        }
        assert!(!server.levels.contains_key(&Dimension::Overworld));
        assert_eq!(
            server.levels[&Dimension::Nether].terrain.stream.player_spawn,
            (5.5, 70.0, 5.5)
        );
        let steve = &server.players[&steve_id];
        assert_eq!(steve.arrival, Some(cave));
        assert_eq!(steve.place, Some(cave));
        let alex = &server.players[&alex_id];
        let platform = Place::standing(Dimension::End, [100.5, 49.0, 0.5], 0.0, 0.0);
        assert_eq!(alex.arrival, Some(platform), "ServerLevel.END_SPAWN_POINT");
        assert_eq!(alex.place, Some(platform));
        let dimensions = server.globals.save.as_ref().unwrap().path().join("dimensions/minecraft");
        for folder in ["the_nether", "the_end"] {
            assert!(dimensions.join(folder).join("region").is_dir(), "{folder}");
        }
        server.open_levels(2);
        assert!(server.opening.is_empty(), "an open level is not opened again");
    }

    #[test]
    fn a_level_that_fails_to_open_sends_its_players_back() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("failed-level");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        let home = steve.place.expect("a joining player stands somewhere");
        steve.dimension = Dimension::End;
        steve.arrival = None;
        let mut alex = join(Some(&save), &recipes, &rules, joining(&profile("Alex")));
        alex.arrive(Place::standing(Dimension::End, [3.5, 60.0, 3.5], 0.0, 0.0));
        let (steve_id, alex_id) = (steve.uuid.clone(), alex.uuid.clone());
        let spawn = RespawnData {
            yaw: 90.0,
            ..crate::save::world_spawn(Dimension::Overworld, (8, 64, -4))
        };
        let globals = Globals {
            spawn,
            ..test_globals(registries, Some(save))
        };
        let mut server = server_with(globals, vec![steve, alex]);
        let loader = LevelLoader::open(
            &mut server.globals,
            Dimension::End,
            2,
            None,
            |_terrain: &mut Terrain| -> Arrival { panic!("the arrival search fails") },
        );
        server.opening.insert(Dimension::End, loader);
        let finished = wait_for_levels(&mut server);
        assert_eq!(finished.len(), 1);
        assert!(finished[0].1.is_err(), "{finished:?}");
        assert!(server.levels.is_empty());
        let steve = &server.players[&steve_id];
        assert_eq!((steve.dimension, steve.arrival), (Dimension::Overworld, Some(home)));
        let alex = &server.players[&alex_id];
        assert_eq!(
            alex.arrival,
            Some(Place::standing(
                Dimension::Overworld,
                [8.5, 64.0, -3.5],
                90.0,
                0.0
            )),
            "with nowhere else to go, the world spawn"
        );
        assert_eq!(alex.dimension, Dimension::Overworld);
    }

    #[test]
    fn a_switching_player_opens_its_new_level_and_arrives_where_it_was_sent() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("switch");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        let home = steve.place.expect("a joining player stands somewhere");
        steve.arrival = None;
        let roof = Place::standing(Dimension::Nether, [100.5, 128.0, -200.5], 30.0, 0.0);
        steve.switch_to(Dimension::Nether, Some(roof), crate::switch::LoadingReason::NetherPortal);
        let steve_id = steve.uuid.clone();
        let mut server = server_with(test_globals(registries, Some(save)), vec![steve]);
        assert_eq!(server.known_spawn(Dimension::Nether, Some(roof)), Some((100, 128, -201)));
        server.open_levels(2);
        assert_eq!(server.opening.keys().copied().collect::<Vec<_>>(), vec![Dimension::Nether]);
        assert_eq!(wait_for_levels(&mut server), vec![(Dimension::Nether, Ok(()))]);
        let steve = &server.players[&steve_id];
        assert_eq!((steve.dimension, steve.arrival), (Dimension::Nether, Some(roof)));
        assert!(steve.switching.is_some(), "the switch ends when the player is let in");
        assert_eq!(server.levels[&Dimension::Nether].terrain.stream.player_spawn, (100.5, 128.0, -200.5));
        assert!(server.close_empty_levels(EMPTY_TIME).is_empty(), "the level the player switched into stays open");
        let player = server.players.get_mut(&steve_id).unwrap();
        player.place = Some(home);
        player.switch_to(Dimension::End, None, crate::switch::LoadingReason::EndPortal);
        let loader = LevelLoader::open(
            &mut server.globals,
            Dimension::End,
            2,
            None,
            |_terrain: &mut Terrain| -> Arrival { panic!("the End fails") },
        );
        server.opening.insert(Dimension::End, loader);
        let finished = wait_for_levels(&mut server);
        assert!(finished[0].1.is_err(), "{finished:?}");
        let steve = &server.players[&steve_id];
        assert_eq!((steve.dimension, steve.arrival), (Dimension::Overworld, Some(home)));
        let switch = steve.switching.expect("still switching, back home");
        assert_eq!((switch.to, switch.arrival), (Dimension::Overworld, Some(home)));
    }

    #[test]
    fn an_arrival_keeps_its_angles_and_motion() {
        let place = Place {
            motion: [0.2, 0.0, -0.1],
            on_ground: false,
            ..Place::standing(Dimension::End, [1.0, 2.0, 3.0], 30.0, -5.0)
        };
        let arrival = Arrival::of(&place);
        assert_eq!(arrival.place(Dimension::End), place);
        assert_eq!(
            Arrival::at([0.5, 64.0, 0.5]).place(Dimension::Nether),
            Place::standing(Dimension::Nether, [0.5, 64.0, 0.5], 0.0, 0.0)
        );
    }

    fn load_chunks_around(level: &mut Level, block: (i32, i32, i32)) {
        let started = Instant::now();
        let chunk = (block.0 >> 4, block.2 >> 4);
        while level.terrain.scene.generated_chunk(chunk).is_none() {
            assert!(started.elapsed().as_secs() < 600, "no chunk at {block:?}");
            let Level {
                terrain,
                entities,
                containers,
                ..
            } = level;
            crate::level::stream_tick(terrain, entities, containers, block);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn a_level_nobody_is_in_saves_and_closes_after_300_ticks() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("close-levels");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        steve.arrive(Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], 0.0, 0.0));
        let mut alex = join(Some(&save), &recipes, &rules, joining(&profile("Alex")));
        alex.dimension = Dimension::End;
        let alex_id = alex.uuid.clone();
        let mut server = server_with(test_globals(registries, Some(save)), vec![steve, alex]);
        server.open_levels(2);
        wait_for_levels(&mut server);
        let end = server.levels.get_mut(&Dimension::End).unwrap();
        load_chunks_around(end, (100, 49, 0));
        let empty = |server: &Server| server.levels[&Dimension::End].empty_ticks;
        server.players.get_mut(&alex_id).unwrap().dimension = Dimension::Nether;
        assert!(server.close_empty_levels(299).is_empty());
        assert_eq!(empty(&server), 299);
        assert_eq!(server.levels[&Dimension::Nether].empty_ticks, 0);
        server.players.get_mut(&alex_id).unwrap().dimension = Dimension::End;
        assert!(server.close_empty_levels(7).is_empty());
        assert_eq!(empty(&server), 0, "a player coming back resets the count");
        server.players.get_mut(&alex_id).unwrap().dimension = Dimension::Nether;
        assert!(server.close_empty_levels(150).is_empty());
        assert!(server.close_empty_levels(149).is_empty());
        assert_eq!(server.close_empty_levels(1), vec![Dimension::End]);
        assert!(!server.levels.contains_key(&Dimension::End));
        assert!(server.levels.contains_key(&Dimension::Nether));
        assert_eq!(server.globals.closing.len(), 1);
        assert_eq!(server.globals.closing[0].0, Dimension::End);
        let alex = server.players.get_mut(&alex_id).unwrap();
        alex.dimension = Dimension::End;
        alex.arrival = None;
        server.open_levels(2);
        assert!(
            server.globals.closing.is_empty(),
            "the loader waits for the closing save itself"
        );
        assert_eq!(wait_for_levels(&mut server), vec![(Dimension::End, Ok(()))]);
        let region = server
            .globals
            .save
            .as_ref()
            .unwrap()
            .path()
            .join("dimensions/minecraft/the_end/region");
        let saved = std::fs::read_dir(&region)
            .unwrap()
            .flatten()
            .any(|file| file.path().extension().is_some_and(|ext| ext == "mca"));
        assert!(saved, "the closed level saved its chunks before it opened again");
    }

    #[test]
    fn a_temporary_world_keeps_its_empty_levels_open() {
        let Some(registries) = registries() else {
            return;
        };
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(None, &recipes, &rules, joining(&profile("Steve")));
        steve.dimension = Dimension::End;
        let steve_id = steve.uuid.clone();
        let mut server = server_with(test_globals(registries, None), vec![steve]);
        server.open_levels(2);
        wait_for_levels(&mut server);
        server.players.get_mut(&steve_id).unwrap().dimension = Dimension::Nether;
        assert!(server.close_empty_levels(EMPTY_TIME * 4).is_empty());
        assert!(server.levels.contains_key(&Dimension::End));
        assert!(server.globals.closing.is_empty());
    }

    #[test]
    fn the_overworld_takes_the_world_s_weather_back_when_a_player_returns() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("weather-return");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        let steve_id = steve.uuid.clone();
        let globals = Globals {
            spawn: crate::save::world_spawn(Dimension::Overworld, (8, 64, -4)),
            ..test_globals(registries, Some(save))
        };
        let mut server = server_with(globals, vec![steve]);
        server.open_levels(2);
        wait_for_levels(&mut server);
        let overworld_weather = |server: &Server| {
            server.levels[&Dimension::Overworld]
                .entities
                .as_ref()
                .unwrap()
                .weather_data
        };
        let before = overworld_weather(&server);
        server.players.get_mut(&steve_id).unwrap().dimension = Dimension::Nether;
        server.close_empty_levels(20);
        let storm = WeatherData {
            thunder_time: 400,
            rain_time: 600,
            raining: true,
            thundering: true,
            ..WeatherData::default()
        };
        server.globals.weather = Some(storm);
        server.close_empty_levels(20);
        assert_eq!(overworld_weather(&server), before, "an empty level is left alone");
        server.players.get_mut(&steve_id).unwrap().dimension = Dimension::Overworld;
        server.close_empty_levels(0);
        assert_eq!(overworld_weather(&server), Some(storm));
        assert_eq!(server.levels[&Dimension::Overworld].empty_ticks, 0);
    }

    #[test]
    fn a_weather_command_in_any_dimension_sets_the_world_s_weather() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("weather-command");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        steve.arrive(Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], 0.0, 0.0));
        let alex = join(Some(&save), &recipes, &rules, joining(&profile("Alex")));
        let globals = Globals {
            spawn: crate::save::world_spawn(Dimension::Overworld, (8, 64, -4)),
            ..test_globals(registries, Some(save))
        };
        let mut server = server_with(globals, vec![steve, alex]);
        server.open_levels(2);
        wait_for_levels(&mut server);
        fn entities(server: &mut Server, dimension: Dimension) -> &mut Entities {
            server
                .levels
                .get_mut(&dimension)
                .unwrap()
                .entities
                .as_mut()
                .unwrap()
        }
        let thunder = minecraft_terrain::server::weather_parameters("thunder", 6000);
        entities(&mut server, Dimension::Nether).set_weather("thunder", 6000);
        server.follow_weather_commands();
        assert_eq!(server.globals.weather, thunder, "MinecraftServer.setWeatherParameters");
        assert_eq!(entities(&mut server, Dimension::Overworld).weather_data, thunder);
        assert_eq!(entities(&mut server, Dimension::Nether).world_weather, None);
        assert_eq!(entities(&mut server, Dimension::Nether).weather_data, None);
        let clear = minecraft_terrain::server::weather_parameters("clear", 900);
        entities(&mut server, Dimension::Overworld).set_weather("clear", 900);
        server.follow_weather_commands();
        assert_eq!(server.globals.weather, clear);
        assert_eq!(
            entities(&mut server, Dimension::Overworld).weather_data,
            thunder,
            "the overworld runs the command itself and gets no copy"
        );
        server.globals.weather = thunder;
        server.follow_weather_commands();
        assert_eq!(server.globals.weather, thunder, "no command, nothing changes");
    }

    #[test]
    fn a_death_in_the_nether_loads_the_respawn_level_at_once_and_the_respawn_switches_there() {
        use crate::respawn::{respawn, respawn_in};
        use crate::switch::LoadingReason;
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("respawn-switch");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        steve.arrive(Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], 0.0, 0.0));
        let id = steve.uuid.clone();
        let globals = Globals {
            spawn: RespawnData {
                yaw: 90.0,
                ..crate::save::world_spawn(Dimension::Overworld, (8, 64, -4))
            },
            ..test_globals(registries, Some(save))
        };
        let mut server = server_with(globals, vec![steve]);
        server.open_levels(2);
        wait_for_levels(&mut server);
        assert_eq!(server.levels.keys().copied().collect::<Vec<_>>(), vec![Dimension::Nether]);
        server.players.get_mut(&id).unwrap().observe_life(playerstate_iw4::PM_TYPE_DEAD);
        server.prepare_respawns();
        assert_eq!(server.players[&id].respawning, Some(Dimension::Overworld));
        server.open_levels(2);
        assert_eq!(wait_for_levels(&mut server), vec![(Dimension::Overworld, Ok(()))]);
        assert_eq!(server.players[&id].dimension, Dimension::Nether, "the dead player stays where they died");
        assert!(server.close_empty_levels(EMPTY_TIME).is_empty(), "the level the player respawns into stays open");

        let steve = server.players.get_mut(&id).unwrap();
        assert!(steve.observe_life(0));
        assert!(!respawn(&server.globals, &server.levels[&Dimension::Nether], steve));
        assert_eq!(steve.respawning, None);
        let switch = steve.switching.expect("the respawn switches dimension");
        assert_eq!(
            (switch.from, switch.to, switch.reason, switch.respawn, switch.arrival),
            (Dimension::Nether, Dimension::Overworld, LoadingReason::Other, true, None)
        );
        assert!(!respawn_in(&server.globals, &server.levels[&Dimension::Overworld], steve, |_| None));
        let (x, y, z) = server.levels[&Dimension::Overworld].terrain.stream.player_spawn;
        assert_eq!(
            steve.arrival,
            Some(Place::standing(Dimension::Overworld, [x, y, z], 90.0, 0.0)),
            "the world spawn is the overworld's own spawn, facing the spawn angle"
        );

        let anchor = crate::player_nbt::Respawn {
            at: crate::player_nbt::GlobalPos {
                dimension: Dimension::Nether,
                pos: (5, 70, 5),
            },
            yaw: -90.0,
            pitch: 0.0,
            forced: true,
        };
        steve.switching = None;
        steve.set_respawn(Some(anchor));
        steve.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        assert!(steve.observe_life(0));
        assert!(!respawn(&server.globals, &server.levels[&Dimension::Overworld], steve));
        assert_eq!(steve.switching.map(|switch| (switch.to, switch.respawn)), Some((Dimension::Nether, true)));
        assert!(!respawn_in(&server.globals, &server.levels[&Dimension::Nether], steve, |_| None));
        assert_eq!(
            steve.arrival,
            Some(Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], -90.0, 0.0))
        );

        load_chunks_around(server.levels.get_mut(&Dimension::Nether).unwrap(), (5, 0, 5));
        let steve = server.players.get_mut(&id).unwrap();
        steve.switching = None;
        steve.set_respawn(Some(crate::player_nbt::Respawn {
            at: crate::player_nbt::GlobalPos {
                dimension: Dimension::Nether,
                pos: (5, 0, 5),
            },
            ..anchor
        }));
        assert!(
            respawn_in(&server.globals, &server.levels[&Dimension::Nether], steve, |_| None),
            "a point in the bedrock floor is missing"
        );
        assert_eq!(steve.respawn, None);
        assert_eq!(steve.switching.map(|switch| (switch.to, switch.arrival)), Some((Dimension::Overworld, None)));
    }
}
