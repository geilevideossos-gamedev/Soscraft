use std::collections::BTreeMap;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Instant;

use minecraft_commands::{CommandServer, LocateRequest, PlayerId};
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
use crate::tick_rate::{TickBatch, TickRate};

pub struct Server {
    pub globals: Globals,
    pub levels: BTreeMap<Dimension, Level>,
    pub opening: BTreeMap<Dimension, LevelLoader>,
    pub players: BTreeMap<PlayerUuid, Player>,
    pub published: bool,
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
    pub maps: std::sync::Mutex<minecraft_terrain::maps::MapStore>,
    pub weather_random: LegacyRandom,
    pub forced_weather: Option<String>,
    pub chat: CommandServer,
    pub data: GameData,
    pub writer: SaveWriter,
    pub autosave_due: bool,
    pub level_ticks: u32,
    pub closing: Vec<(Dimension, JoinHandle<()>)>,
    pub waiting_locates: Vec<(Dimension, PlayerId, LocateRequest)>,
    pub tick_rate: TickRate,
}

impl Globals {
    pub fn game_time(&self) -> i64 {
        self.save.as_ref().map_or(self.day.ticks.floor() as i64, |save| {
            save.game_time(Some(self.day.ticks))
        })
    }

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

    pub fn set_clock(&mut self, clock: &str, total: i64) {
        set_clock(&mut self.day, &mut self.end_clock, self.save.as_mut(), clock, total);
    }

    pub fn advance_clocks(&mut self, ticks: u32) {
        if ticks == 0 {
            return;
        }
        let advancing = self.game_rules.bool("advance_time");
        if advancing && !self.day.paused {
            self.day.advance_ticks(ticks);
        } else if let Some(save) = self.save.as_mut() {
            save.shift_day(-f64::from(ticks));
        }
        if advancing {
            self.end_clock.advance_ticks(ticks);
        }
    }
}

pub const EMPTY_TIME: u32 = 300;

pub fn set_clock(
    day: &mut DayCycle,
    end_clock: &mut DayCycle,
    save: Option<&mut WorldSave>,
    clock: &str,
    total: i64,
) {
    if clock == THE_END_CLOCK {
        end_clock.set(total as f64);
        return;
    }
    let ticks = total as f64;
    if let Some(save) = save {
        save.shift_day(ticks - day.ticks);
    }
    day.set(ticks);
}

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
    pub requested: Option<Place>,
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
        Self {
            result,
            thread,
            requested: None,
        }
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

pub fn resolve_rings_in_background(
    stream: &minecraft_terrain::terrain::TerrainStream,
) -> Option<JoinHandle<usize>> {
    let world = stream.world_gen();
    std::thread::Builder::new()
        .name("Stronghold rings".to_owned())
        .spawn(move || {
            if !minecraft_terrain::locate::has_rings(&world) {
                return 0;
            }
            let started = Instant::now();
            let rings = minecraft_terrain::locate::resolve_rings(&world);
            diag::info!(
                World,
                "Minecraft stronghold rings: {rings} worked out in {} ms",
                started.elapsed().as_millis()
            );
            rings
        })
        .ok()
}

pub fn start_level(globals: &mut Globals, level: &mut Level) {
    let _ = resolve_rings_in_background(&level.terrain.stream);
    let forced_weather = if has_weather(level.dimension) {
        globals.forced_weather.take()
    } else {
        None
    };
    let mut entities = Entities::new(
        &level.terrain.stream,
        level.dimension,
        globals.seed,
        &globals.data,
        globals.weather.as_ref(),
        forced_weather.as_deref(),
    );
    entities.set_game_rules(&globals.game_rules);
    entities.set_game_time(globals.game_time());
    let (waiting, others) = std::mem::take(&mut globals.waiting_locates)
        .into_iter()
        .partition(|(dimension, _, _)| *dimension == level.dimension);
    globals.waiting_locates = others;
    for (_, source, request) in waiting {
        let job = crate::commands::locate_job(source, request, &level.terrain.stream);
        entities.run_job(job);
    }
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
    pub fn plan_ticks(&mut self, now: Instant, ticking: &[Dimension]) -> TickBatch {
        let mut holders = Vec::new();
        let mut ready = true;
        let mut finished: Vec<u64> = Vec::new();
        for dimension in ticking {
            let Some(level) = self.levels.get_mut(dimension) else {
                continue;
            };
            let waiting = level.pending_batch.is_none();
            let Some(entities) = level.entities.as_mut() else {
                continue;
            };
            holders.push(*dimension);
            ready &= entities.ready_for_ticks() && waiting;
            for (index, nanos) in entities.take_finished_ticks().into_iter().enumerate() {
                match finished.get_mut(index) {
                    Some(slowest) => *slowest = (*slowest).max(nanos),
                    None => finished.push(nanos),
                }
            }
        }
        for (dimension, level) in &mut self.levels {
            if !holders.contains(dimension) {
                level.pending_batch = None;
            }
        }
        self.globals.tick_rate.record(&finished);
        let batch = self.globals.tick_rate.plan(now, ready);
        let normal = batch.normal_count();
        self.globals.advance_clocks(normal);
        self.globals.advance_weather(ticking, normal);
        self.globals.level_ticks = self.globals.level_ticks.saturating_add(batch.len() as u32);
        if self.globals.tick_rate.count_autosave(batch.len()) {
            self.globals.autosave_due = true;
        }
        if !holders.is_empty() && !batch.is_empty() {
            for dimension in &holders {
                if let Some(level) = self.levels.get_mut(dimension) {
                    level.container_ticks = level.container_ticks.saturating_add(normal);
                    level.pending_batch = Some(batch.clone());
                }
            }
        }
        batch
    }

    pub fn take_batch(&mut self, dimension: Dimension) -> TickBatch {
        self.levels
            .get_mut(&dimension)
            .and_then(|level| level.pending_batch.take())
            .unwrap_or(TickBatch {
                normal: Vec::new(),
                overload: None,
            })
    }

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
            .chain(
                self.globals
                    .waiting_locates
                    .iter()
                    .map(|(dimension, _, _)| (*dimension, None)),
            )
            .filter(|(dimension, _)| !self.levels.contains_key(dimension))
            .collect();
        for (dimension, arrival) in waiting {
            if self.opening.contains_key(&dimension) {
                continue;
            }
            let travel = match arrival {
                Some(_) => None,
                None => crate::portal_forcer::travel_into(
                    &mut self.players,
                    dimension,
                    &self.globals.registries,
                ),
            };
            let entering = arrival.or(travel.map(|travel| travel.target_place()));
            let known_spawn = self.known_spawn(dimension, entering);
            let mut loader = LevelLoader::open(
                &mut self.globals,
                dimension,
                view_distance,
                known_spawn,
                move |terrain| match (arrival, travel) {
                    (Some(place), _) => Arrival::of(&place),
                    (None, Some(travel)) => crate::portal_forcer::arrival(terrain, &travel),
                    (None, None) => {
                        let (x, y, z) = terrain.stream.player_spawn;
                        Arrival::at([x, y, z])
                    }
                },
            );
            loader.requested = arrival;
            self.opening.insert(dimension, loader);
        }
    }

    pub fn finish_opening(&mut self) -> Vec<(Dimension, Result<(), String>)> {
        let ready: Vec<(Dimension, Opened, Option<Place>)> = self
            .opening
            .iter_mut()
            .filter_map(|(&dimension, loader)| {
                loader
                    .poll()
                    .map(|opened| (dimension, opened, loader.requested))
            })
            .collect();
        let mut finished = Vec::new();
        for (dimension, opened, requested) in ready {
            self.opening.remove(&dimension);
            let waiting = self
                .players
                .values_mut()
                .filter(|player| player.dimension == dimension);
            match opened {
                Ok((terrain, arrival)) => {
                    let mut level = Level::new(dimension, terrain);
                    start_level(&mut self.globals, &mut level);
                    self.levels.insert(dimension, level);
                    for player in waiting {
                        let own = player.arrival.filter(|place| place.dimension == dimension);
                        if player.place.is_none_or(|place| place.dimension != dimension)
                            && own.is_none_or(|own| Some(own) == requested)
                            && !crate::portal_forcer::waits_for_portal(player, dimension)
                        {
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
                    self.globals
                        .waiting_locates
                        .retain(|(waiting, _, _)| *waiting != dimension);
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
            let searching = level.entities.as_ref().is_some_and(Entities::runs_jobs);
            if level.empty_ticks >= EMPTY_TIME && !keeps_empty_levels && !searching {
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

    pub fn take_vehicles(&mut self) {
        refresh_vehicles(&mut self.levels, &mut self.players, true);
    }

    pub fn save_and_close(mut self, started: Instant) {
        self.take_vehicles();
        let save = self.globals.save.take();
        self.globals.writer.finish();
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

    pub fn join_friend(
        &mut self,
        id: PlayerId,
        name: &str,
        kit: Option<&str>,
    ) -> Option<PlayerUuid> {
        self.globals.chat.join(id, name);
        let profile = self.globals.chat.profile(id)?.clone();
        let uuid = PlayerUuid(profile.uuid.clone());
        let spawn = match self.levels.get_mut(&Dimension::Overworld) {
            Some(level) => crate::respawn::level_spawn(&self.globals, level).feet,
            None => spawn_place(&self.globals.spawn).feet,
        };
        let joining = Joining {
            profile: &profile,
            spawn,
            chosen_mode: None,
            kit,
            carried: None,
        };
        let globals = &mut self.globals;
        let mut player = match globals.writer.recently_left(&uuid) {
            Some(saved) => join_from(
                saved,
                globals.save.as_ref(),
                &globals.data.recipes,
                &globals.game_rules,
                joining,
            ),
            None => join(
                globals.save.as_ref(),
                &globals.data.recipes,
                &globals.game_rules,
                joining,
            ),
        };
        if player.arrival.is_none() {
            player.arrival = player
                .place
                .filter(|place| place.dimension == player.dimension);
        }
        self.players.insert(uuid.clone(), player);
        Some(uuid)
    }

    pub fn leave(&mut self, uuid: &PlayerUuid) -> Option<Left> {
        let mut player = self.players.remove(uuid)?;
        crate::the_end::credits_done(&mut player);
        let woke = match self.levels.get_mut(&player.dimension) {
            Some(level) => crate::sleep::wake_leaving(&self.globals, level, &mut player),
            None => Vec::new(),
        };
        let feet = player
            .place
            .filter(|place| place.dimension == player.dimension)
            .map(|place| place.feet);
        let level = self.levels.get_mut(&player.dimension);
        let dropped = match level {
            Some(level) => {
                if let Some(entities) = level.entities.as_mut() {
                    if player.riding.is_some() {
                        player.saved_vehicle = entities.root_vehicle(u64::from(player.id.0), true);
                    }
                    if let Some(feet) = feet {
                        crate::menus::close_merchant_now(&mut player, entities, feet);
                    }
                    let experience = entities.leave(
                        u64::from(player.id.0),
                        &mut player.inventory,
                        player.selected,
                        feet,
                    );
                    let _ = player.survival.gain_experience(experience);
                }
                let dropped = player.close_menu();
                if let (Some(entities), Some(feet)) = (level.entities.as_mut(), feet) {
                    for stack in &dropped {
                        entities.drop_given(stack, feet);
                    }
                }
                crate::level::leave(level, uuid);
                dropped
            }
            None => player.close_menu(),
        };
        self.globals.chat.leave(player.id);
        save_leaving(&mut self.globals, &player);
        Some(Left {
            player,
            dropped,
            woke,
        })
    }
}

pub struct Left {
    pub player: Player,
    pub dropped: Vec<minecraftoss_player::inventory::ItemStack>,
    pub woke: Vec<(i32, i32, i32)>,
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
    join_from(
        saved,
        save,
        recipes,
        rules,
        Joining {
            profile,
            spawn,
            chosen_mode,
            kit,
            carried,
        },
    )
}

pub fn join_from(
    saved: PlayerData,
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
    let mode = starting_game_mode(
        saved.mode.or_else(|| save.map(WorldSave::default_mode)),
        chosen_mode,
    );
    let mut inventory = joining_inventory(recipes.clone(), saved.inventory, kit);
    let recipe_book = crate::recipe_book::Book::restore(saved.recipe_book.as_ref(), &mut inventory);
    crate::recipe_book::notice_carried(&mut inventory);
    let dead = saved.stats.is_some_and(|stats| stats.status.health <= 0.0);
    let survival = match (saved.stats, carried) {
        (Some(stats), _) => {
            let mut survival = Survival::default();
            survival.restore(stats.status, stats.fire, stats.frozen, stats.score);
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
    if let Some(seed) = saved.xp_seed {
        player.enchantment_seed = seed;
    }
    player.reattach = saved.root_vehicle.filter(|_| !dead);
    player.sleep_timer = saved.sleep_timer.unwrap_or(0);
    player.time_since_rest = saved.time_since_rest.unwrap_or(0);
    player.saved_in_bed = saved.sleeping_pos;
    player.survival.raid_omen_position = saved.raid_omen_position;
    player.tags = saved.tags.into_iter().flatten().collect();
    player.recipe_book = recipe_book;
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
    maps: minecraft_terrain::maps::MapWrites,
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
            maps: globals
                .maps
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take_writes(),
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
        if !self.maps.is_empty()
            && let Err(error) = save.write_maps(&self.maps)
        {
            diag::warn!(World, "{prefix} map files not written: {error}");
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
    let file = [(player.uuid.0.clone(), player.data())];
    globals
        .writer
        .queue(move || write_players(&save, &file, "Minecraft player file not written"));
}

pub fn save_leaving(globals: &mut Globals, player: &Player) {
    let data = player.data();
    let Some(save) = globals.save.clone() else {
        globals.writer.keep_left(player.uuid.clone(), 0, data);
        return;
    };
    let file = [(player.uuid.0.clone(), data.clone())];
    let ticket = globals
        .writer
        .queue(move || write_players(&save, &file, "Minecraft player file not written"));
    globals.writer.keep_left(player.uuid.clone(), ticket, data);
}

type SaveJob = Box<dyn FnOnce() + Send>;

#[derive(Default)]
pub struct SaveWriter {
    jobs: Option<mpsc::Sender<(u64, SaveJob)>>,
    written: Option<mpsc::Receiver<u64>>,
    thread: Option<JoinHandle<()>>,
    queued: u64,
    done: u64,
    recently_left: BTreeMap<PlayerUuid, (u64, PlayerData)>,
}

impl SaveWriter {
    fn start(&mut self) {
        let (jobs, receive) = mpsc::channel::<(u64, SaveJob)>();
        let (report, written) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("minecraft-world-save".into())
            .spawn(move || {
                for (ticket, job) in receive {
                    job();
                    let _ = report.send(ticket);
                }
            });
        match thread {
            Ok(thread) => {
                self.jobs = Some(jobs);
                self.written = Some(written);
                self.thread = Some(thread);
            }
            Err(error) => diag::warn!(World, "Minecraft save thread not started: {error}"),
        }
    }

    pub fn queue(&mut self, job: impl FnOnce() + Send + 'static) -> u64 {
        self.queued += 1;
        let ticket = self.queued;
        if self.jobs.is_none() {
            self.start();
        }
        let job: SaveJob = Box::new(job);
        let refused = match self.jobs.as_ref() {
            Some(jobs) => jobs.send((ticket, job)).err().map(|refused| refused.0.1),
            None => Some(job),
        };
        if let Some(job) = refused {
            job();
            self.done = self.done.max(ticket);
        }
        ticket
    }

    pub fn poll(&mut self) {
        if let Some(written) = self.written.as_ref() {
            while let Ok(ticket) = written.try_recv() {
                self.done = self.done.max(ticket);
            }
        }
        let done = self.done;
        self.recently_left
            .retain(|_, (ticket, _)| *ticket == 0 || *ticket > done);
    }

    pub fn written(&mut self, ticket: u64) -> bool {
        self.poll();
        ticket <= self.done
    }

    pub fn keep_left(&mut self, uuid: PlayerUuid, ticket: u64, data: PlayerData) {
        self.recently_left.insert(uuid, (ticket, data));
    }

    pub fn recently_left(&mut self, uuid: &PlayerUuid) -> Option<PlayerData> {
        self.poll();
        self.recently_left.get(uuid).map(|(_, data)| data.clone())
    }

    pub fn finish(&mut self) {
        drop(self.jobs.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.poll();
        self.written = None;
    }
}

pub fn refresh_vehicles(
    levels: &mut BTreeMap<Dimension, Level>,
    players: &mut BTreeMap<PlayerUuid, Player>,
    remove: bool,
) {
    for player in players.values_mut().filter(|player| player.riding.is_some()) {
        if let Some(entities) = levels
            .get_mut(&player.dimension)
            .and_then(|level| level.entities.as_mut())
        {
            player.saved_vehicle = entities.root_vehicle(u64::from(player.id.0), remove);
        }
    }
}

pub fn autosave(
    globals: &mut Globals,
    levels: &mut BTreeMap<Dimension, Level>,
    players: &mut BTreeMap<PlayerUuid, Player>,
) {
    let Some(save) = globals.save.clone() else {
        return;
    };
    refresh_vehicles(levels, players, false);
    let started = Instant::now();
    let chunks: usize = levels
        .values_mut()
        .map(|level| {
            crate::level::save_block_entities(&mut level.containers, &mut level.terrain);
            level.terrain.stream.queue_save()
        })
        .sum();
    let path = save.path().display().to_string();
    let files = WorldFiles::of(globals, players);
    globals
        .writer
        .queue(move || files.write(&save, "Minecraft autosave:"));
    diag::info!(
        World,
        "Minecraft autosave: {chunks} chunks queued for {path} in {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

#[cfg(test)]
pub(crate) mod tests {
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

    pub(crate) struct TestDir(std::path::PathBuf);

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn world(name: &str) -> (TestDir, WorldSave) {
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
    fn a_saved_sleeper_joins_with_its_bed_timer_and_time_since_rest() {
        let (_dir, save) = world("sleeper");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!((player.sleep_timer, player.time_since_rest, player.saved_in_bed), (0, 0, None));
        player.saved_in_bed = Some((4, 70, -9));
        player.sleep_timer = 100;
        player.time_since_rest = 72_001;
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let again = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(again.saved_in_bed, Some((4, 70, -9)));
        assert!(again.sleeping.is_none(), "it stands up once its bed's chunk is in");
        assert_eq!((again.sleep_timer, again.time_since_rest), (100, 72_001));
    }

    #[test]
    fn a_joining_player_keeps_the_enchanting_seed_their_file_saved() {
        let (_dir, save) = world("xp_seed");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mode_only = PlayerData {
            mode: Some(PlayerMode::new(GameMode::Survival)),
            ..PlayerData::default()
        };
        save.write_player(&steve.uuid, &mode_only).unwrap();
        let mut player = join(Some(&save), &recipes, &rules, joining(&steve));
        let derived = crate::level::opener_key(&player.uuid) as i32;
        assert_eq!(player.enchantment_seed, derived, "a file without XpSeed keeps the seed the UUID gives");
        player.enchantment_seed = -123_456_789;
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let again = join(Some(&save), &recipes, &rules, joining(&steve));
        assert_eq!(again.enchantment_seed, -123_456_789);
        let alex = profile("Alex");
        let alex = join(Some(&save), &recipes, &rules, joining(&alex));
        assert_eq!(alex.enchantment_seed, crate::level::opener_key(&alex.uuid) as i32, "each player has their own");
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

    pub(crate) fn registries() -> Option<Arc<Registries>> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        Registries::load(&paths).ok().map(Arc::new)
    }

    pub(crate) fn test_globals(registries: Arc<Registries>, save: Option<WorldSave>) -> Globals {
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
            maps: Default::default(),
            weather_random: weather_random(1),
            forced_weather: None,
            chat: CommandServer::new(crate::commands::HOST),
            data: GameData::default(),
            writer: SaveWriter::default(),
            autosave_due: false,
            level_ticks: 0,
            closing: Vec::new(),
            waiting_locates: Vec::new(),
            tick_rate: TickRate::default(),
        }
    }

    fn friend_server(name: &str) -> Option<(TestDir, WorldSave, Server, mpsc::Sender<()>)> {
        let registries = registries()?;
        let (dir, save) = world(name);
        let mut server = Server {
            globals: test_globals(registries, Some(save.clone())),
            levels: BTreeMap::new(),
            opening: BTreeMap::new(),
            players: BTreeMap::new(),
            published: false,
        };
        let (open, gate) = mpsc::channel::<()>();
        server.globals.writer.queue(move || {
            let _ = gate.recv_timeout(std::time::Duration::from_secs(60));
        });
        Some((dir, save, server, open))
    }

    #[test]
    fn a_leaving_friend_folds_their_menu_back_drops_the_rest_and_saves_off_the_main_thread() {
        let Some((_dir, save, mut server, open)) = friend_server("friend-leave") else {
            return;
        };
        let alex = PlayerId(1);
        let uuid = server
            .join_friend(alex, "Alex", None)
            .unwrap();
        let player = server.players.get_mut(&uuid).unwrap();
        for slot in 0..35 {
            player.inventory.slots[slot] = Some(ItemStack::new("minecraft:stone", 64));
        }
        player.inventory.crafting[0] = Some(ItemStack::new("minecraft:stick", 1));
        player.inventory.cursor = Some(ItemStack::new("minecraft:diamond", 2));
        let started = Instant::now();
        let left = server.leave(&uuid).expect("Alex was in the world");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "the leave never waits for the writer"
        );
        let dropped: Vec<_> = left
            .dropped
            .iter()
            .map(|stack| (stack.id.as_str(), stack.count))
            .collect();
        assert_eq!(dropped, vec![("minecraft:stick", 1)]);
        assert_eq!(held(&left.player, 35), Some(("minecraft:diamond", 2)));
        assert!(server.players.is_empty());
        assert!(server.globals.chat.profile(alex).is_none());
        let recipes = server.globals.data.recipes.clone();
        assert_eq!(
            save.read_player(&uuid.0, &recipes).unwrap(),
            None,
            "the file waits behind the writer"
        );
        open.send(()).unwrap();
        server.globals.writer.finish();
        let saved = save
            .read_player(&uuid.0, &recipes)
            .unwrap()
            .expect("the leave wrote the file");
        let slots = saved.inventory.unwrap();
        assert_eq!(slots[35].as_ref().map(|stack| stack.count), Some(2));
        assert!(server.globals.writer.recently_left(&uuid).is_none());
    }

    #[test]
    fn a_friend_who_quits_in_bed_gets_up_at_once_and_frees_the_bed() {
        let Some((_dir, _save, mut server, open)) = friend_server("friend-bed-leave") else {
            return;
        };
        let registries = server.globals.registries.clone();
        let Ok(terrain) = Terrain::open(registries.clone(), 0, 2, Dimension::Overworld, None, None)
        else {
            return;
        };
        let mut level = Level::new(Dimension::Overworld, terrain);
        let (foot, head) = ((0, 64, 0), (0, 64, 1));
        for (pos, part) in [(foot, "foot"), (head, "head")] {
            let text = format!("minecraft:red_bed[facing=south,part={part},occupied=false]");
            let state = registries.blocks.parse_state(&text).unwrap();
            let block = level.terrain.stream.states.block(state).cloned();
            level.terrain.scene.set(pos, block);
        }
        server.levels.insert(Dimension::Overworld, level);
        let uuid = server.join_friend(PlayerId(1), "Alex", None).unwrap();
        let bed = crate::respawn_block::Bed {
            head,
            facing: (0, 1),
            occupied: false,
            straw: false,
        };
        let level = server.levels.get_mut(&Dimension::Overworld).unwrap();
        crate::sleep::start(&server.globals, level, &mut server.players, &uuid, &bed, false);
        let occupied = |server: &Server, pos| {
            let level = &server.levels[&Dimension::Overworld];
            let block = minecraft_terrain::scene::Scene::block(&level.terrain.scene, pos)?;
            let state = level.terrain.stream.states.state_of(block)?;
            registries.blocks.property(state, "occupied").map(str::to_owned)
        };
        assert!(server.players[&uuid].sleeping.is_some());
        assert_eq!(occupied(&server, head).as_deref(), Some("true"));
        let left = server.leave(&uuid).expect("Alex was in the world");
        assert!(left.player.sleeping.is_none(), "saved awake, as vanilla's disconnect");
        assert_eq!(left.player.sleep_timer, 0, "woken at once");
        assert_eq!(occupied(&server, head).as_deref(), Some("false"));
        assert_eq!(occupied(&server, foot).as_deref(), Some("false"));
        assert!(left.woke.contains(&head) && left.woke.contains(&foot));
        let feet = left.player.place.unwrap().feet;
        assert!(feet[1] >= 64.0, "standing beside or on the bed: {feet:?}");
        open.send(()).unwrap();
        server.globals.writer.finish();
    }

    #[test]
    fn a_quick_rejoin_reads_the_player_the_writer_has_not_written_yet() {
        let Some((_dir, save, mut server, open)) = friend_server("friend-rejoin") else {
            return;
        };
        let uuid = server
            .join_friend(PlayerId(1), "Alex", None)
            .unwrap();
        let place = Place::standing(Dimension::Overworld, [30.5, 71.0, -9.5], 90.0, 0.0);
        let player = server.players.get_mut(&uuid).unwrap();
        player.inventory.slots[0] = Some(ItemStack::new("minecraft:diamond", 5));
        player.place = Some(place);
        server.leave(&uuid).unwrap();
        let again = server
            .join_friend(PlayerId(2), "Alex", Some("dirt"))
            .unwrap();
        assert_eq!(again, uuid);
        let alex = &server.players[&again];
        assert_eq!(alex.id, PlayerId(2));
        assert_eq!(held(alex, 0), Some(("minecraft:diamond", 5)));
        assert_eq!(held(alex, 9), None, "no kit for a player who has a file");
        assert_eq!(alex.arrival, Some(place));
        let recipes = server.globals.data.recipes.clone();
        assert_eq!(save.read_player(&uuid.0, &recipes).unwrap(), None);
        open.send(()).unwrap();
        server.globals.writer.finish();
        assert!(server.globals.writer.recently_left(&uuid).is_none());
        let saved = save.read_player(&uuid.0, &recipes).unwrap().unwrap();
        assert_eq!(saved.place, Some(place));
    }

    #[test]
    fn a_friend_saved_in_another_dimension_joins_there() {
        let Some((_dir, save, mut server, open)) = friend_server("friend-elsewhere") else {
            return;
        };
        let profile = {
            server.globals.chat.join(PlayerId(1), "Alex");
            server.globals.chat.profile(PlayerId(1)).unwrap().clone()
        };
        let nether = PlayerData {
            place: Some(Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], 0.0, 0.0)),
            ..PlayerData::default()
        };
        save.write_player(&profile.uuid, &nether).unwrap();
        let uuid = server.join_friend(PlayerId(1), "Alex", None).unwrap();
        let alex = &server.players[&uuid];
        assert_eq!(alex.dimension, Dimension::Nether);
        assert_eq!(
            alex.arrival.map(|place| (place.dimension, place.feet)),
            Some((Dimension::Nether, [5.5, 70.0, 5.5]))
        );
        assert!(!server.levels.contains_key(&Dimension::Nether));
        server.open_levels(8);
        assert!(server.opening.contains_key(&Dimension::Nether), "the friend's level opens for them");
        server.leave(&uuid).unwrap();
        open.send(()).unwrap();
        server.globals.writer.finish();
        let uuid = server.join_friend(PlayerId(2), "Alex", None).unwrap();
        let alex = &server.players[&uuid];
        assert_eq!(alex.dimension, Dimension::Nether, "a rejoin comes back where they left");
    }

    #[test]
    fn a_new_friend_arrives_within_the_radius_and_a_returning_one_where_they_left() {
        let Some((_dir, _save, mut server, open)) = friend_server("friend-spread") else {
            return;
        };
        let registries = server.globals.registries.clone();
        let Ok(terrain) = Terrain::open(registries, 0, 2, Dimension::Overworld, None, None) else {
            return;
        };
        server.levels.insert(Dimension::Overworld, Level::new(Dimension::Overworld, terrain));
        let (x, _, z) = server.levels[&Dimension::Overworld].terrain.stream.world_spawn;
        let (x, z) = (f64::from(x) + 0.5, f64::from(z) + 0.5);
        let uuid = server.join_friend(PlayerId(1), "Alex", None).unwrap();
        let arrival = server.players[&uuid].arrival.expect("a new friend arrives at the spawn");
        assert_eq!(arrival.dimension, Dimension::Overworld);
        assert!(
            (arrival.feet[0] - x).abs() <= 10.0 && (arrival.feet[2] - z).abs() <= 10.0,
            "within the respawn radius of the world spawn: {arrival:?}"
        );
        let left_at = Place::standing(Dimension::Overworld, [x + 30.0, 80.0, z - 7.0], 45.0, 0.0);
        server.players.get_mut(&uuid).unwrap().place = Some(left_at);
        server.leave(&uuid).unwrap();
        let again = server.join_friend(PlayerId(2), "Alex", None).unwrap();
        assert_eq!(server.players[&again].arrival, Some(left_at), "a returning friend comes back exactly where they left");
        server.globals.game_rules.set("respawn_radius", "0").unwrap();
        let steve = server.join_friend(PlayerId(3), "Steve", None).unwrap();
        let feet = server.players[&steve].arrival.map(|place| (place.feet[0], place.feet[2]));
        assert_eq!(feet, Some((x, z)), "radius 0 is the spawn column");
        open.send(()).unwrap();
        server.globals.writer.finish();
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
        globals.advance_clocks(20);
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
        globals.advance_clocks(20);
        assert_eq!(
            globals.clock_ticks(),
            ClockTicks {
                overworld: 1040,
                the_end: 5
            }
        );
        globals.end_clock.paused = false;
        globals.game_rules.set("advance_time", "false").unwrap();
        globals.advance_clocks(20);
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
            published: false,
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
    fn autosaves_and_closing_write_new_maps_into_the_vanilla_map_files() {
        let Some(registries) = registries() else {
            return;
        };
        let (dir, save) = world("maps");
        let mut globals = test_globals(registries, Some(save));
        let treasure = minecraft_terrain::maps::MapData::create_fresh(1000.0, -300.0, 1, "minecraft:overworld", true, true);
        {
            let mut maps = globals.maps.lock().unwrap();
            let id = maps.free_id();
            maps.insert(id, treasure.clone());
        }
        autosave(&mut globals, &mut BTreeMap::new(), &mut BTreeMap::new());
        globals.writer.finish();
        let folder = dir.0.join("world/data/minecraft/maps");
        assert!(folder.join("0.dat").is_file() && folder.join("last_id.dat").is_file());
        let _ = std::fs::remove_file(folder.join("0.dat"));
        let mut plain = minecraft_terrain::maps::MapData::create_fresh(5.0, 5.0, 0, "minecraft:the_nether", true, false);
        plain.colors[100] = 9;
        {
            let mut maps = globals.maps.lock().unwrap();
            let id = maps.free_id();
            maps.insert(id, plain.clone());
        }
        let server = Server {
            globals,
            levels: BTreeMap::new(),
            opening: BTreeMap::new(),
            players: BTreeMap::new(),
            published: false,
        };
        server.save_and_close(Instant::now());
        assert!(!folder.join("0.dat").exists(), "a map written once is not written again unchanged");
        let opened = crate::save::open_world(&dir.0, None, None, 2);
        assert!(opened.warnings.is_empty(), "{:?}", opened.warnings);
        assert_eq!(opened.maps.last_id(), 1);
        assert_eq!(opened.maps.get(1), Some(&plain));
        assert_eq!(opened.maps.get(0), None);
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
            published: false,
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
        let host = crate::commands::HOST;
        server.globals.waiting_locates = vec![
            (Dimension::End, host, biome_search(Dimension::End, &[])),
            (
                Dimension::Nether,
                host,
                biome_search(Dimension::Nether, &[]),
            ),
        ];
        let finished = wait_for_levels(&mut server);
        assert_eq!(finished.len(), 1);
        assert!(finished[0].1.is_err(), "{finished:?}");
        assert!(server.levels.is_empty());
        let waiting: Vec<Dimension> = server
            .globals
            .waiting_locates
            .iter()
            .map(|(dimension, _, _)| *dimension)
            .collect();
        assert_eq!(
            waiting,
            vec![Dimension::Nether],
            "a search in a level that failed to open is dropped"
        );
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

    fn biome_search(dimension: Dimension, targets: &[&str]) -> LocateRequest {
        LocateRequest {
            kind: minecraft_commands::LocateKind::Biome,
            printable: "#minecraft:is_nether".to_owned(),
            tag: true,
            targets: targets.iter().map(|target| (*target).to_owned()).collect(),
            dimension: dimension.dimension_type().to_owned(),
            origin: [0, 64, 0],
        }
    }

    #[test]
    fn a_search_in_a_closed_level_opens_it_and_keeps_it_open_until_answered() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("search-level");
        let mut server = server_with(test_globals(registries, Some(save)), Vec::new());
        let host = crate::commands::HOST;
        let nether_biomes = [
            "minecraft:nether_wastes",
            "minecraft:crimson_forest",
            "minecraft:warped_forest",
            "minecraft:soul_sand_valley",
            "minecraft:basalt_deltas",
        ];
        let request = biome_search(Dimension::Nether, &nether_biomes);
        server
            .globals
            .waiting_locates
            .push((Dimension::Nether, host, request.clone()));
        server.open_levels(2);
        assert_eq!(
            server.opening.keys().copied().collect::<Vec<_>>(),
            vec![Dimension::Nether]
        );
        assert_eq!(
            wait_for_levels(&mut server),
            vec![(Dimension::Nether, Ok(()))]
        );
        assert!(
            server.globals.waiting_locates.is_empty(),
            "the level took its search"
        );
        let entities = server
            .levels
            .get_mut(&Dimension::Nether)
            .unwrap()
            .entities
            .as_mut()
            .unwrap();
        assert!(entities.runs_jobs());
        assert!(
            server.close_empty_levels(EMPTY_TIME).is_empty(),
            "a level still searching stays open"
        );
        let started = Instant::now();
        let answer = loop {
            let entities = server
                .levels
                .get_mut(&Dimension::Nether)
                .unwrap()
                .entities
                .as_mut()
                .unwrap();
            entities.poll_jobs();
            if let Some(job) = entities.command_jobs.pop() {
                break job;
            }
            assert!(
                started.elapsed().as_secs() < 600,
                "no answer from the Nether"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let Ok(answer) = answer.downcast::<crate::commands::CommandJob>() else {
            panic!("a command job");
        };
        let crate::commands::CommandJob::Located {
            source,
            request: asked,
            search,
        } = *answer
        else {
            panic!("a located answer");
        };
        assert_eq!((source, asked), (host, request));
        let minecraft_terrain::locate::Search::Found(_, biome) = search else {
            panic!("a Nether biome near 0, 0: {search:?}");
        };
        assert!(nether_biomes.contains(&biome.as_str()), "{biome}");
        assert!(
            !server.levels[&Dimension::Nether]
                .entities
                .as_ref()
                .unwrap()
                .runs_jobs()
        );
        assert_eq!(server.close_empty_levels(1), vec![Dimension::Nether]);
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
    fn players_switching_into_one_level_each_arrive_where_they_were_sent() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("switch-together");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let roofs = [
            Place::standing(Dimension::Nether, [100.5, 128.0, -200.5], 30.0, 0.0),
            Place::standing(Dimension::Nether, [-60.5, 128.0, 40.5], -90.0, 0.0),
        ];
        let mut players = Vec::new();
        for (name, roof) in ["Steve", "Alex"].into_iter().zip(roofs) {
            let mut player = join(Some(&save), &recipes, &rules, joining(&profile(name)));
            player.arrival = None;
            player.switch_to(Dimension::Nether, Some(roof), crate::switch::LoadingReason::NetherPortal);
            players.push(player);
        }
        let mut carl = join(Some(&save), &recipes, &rules, joining(&profile("Carl")));
        carl.dimension = Dimension::Nether;
        carl.arrival = None;
        players.push(carl);
        let ids: Vec<PlayerUuid> = players.iter().map(|player| player.uuid.clone()).collect();
        let mut server = server_with(test_globals(registries, Some(save)), players);
        server.open_levels(2);
        assert_eq!(server.opening.len(), 1, "one level for all of them");
        let requested = server.opening[&Dimension::Nether].requested;
        assert!(requested.is_none_or(|place| roofs.contains(&place)));
        assert_eq!(wait_for_levels(&mut server), vec![(Dimension::Nether, Ok(()))]);
        for (id, roof) in ids.iter().zip(roofs) {
            assert_eq!(
                server.players[id].arrival,
                Some(roof),
                "each player keeps their own arrival"
            );
        }
        let (x, y, z) = server.levels[&Dimension::Nether].terrain.stream.player_spawn;
        let opened = requested.map_or(Arrival::at([x, y, z]), |place| Arrival::of(&place));
        assert_eq!(
            server.players[&ids[2]].arrival,
            Some(opened.place(Dimension::Nether)),
            "a player with no arrival of their own takes the level's"
        );
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
            crate::level::stream_tick(terrain, entities, containers, &[block], Some(block));
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
    fn the_weather_variable_starts_the_overworld_once_and_not_again_when_it_reopens() {
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("forced-weather");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let cave = Place::standing(Dimension::Nether, [5.5, 70.0, 5.5], 0.0, 0.0);
        let home = Place::standing(Dimension::Overworld, [8.5, 64.0, -3.5], 0.0, 0.0);
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        steve.arrive(cave);
        let id = steve.uuid.clone();
        let globals = Globals {
            spawn: crate::save::world_spawn(Dimension::Overworld, (8, 64, -4)),
            forced_weather: Some("thunder".to_owned()),
            ..test_globals(registries, Some(save))
        };
        let mut server = server_with(globals, vec![steve]);
        let thundering = |server: &Server| {
            server.levels[&Dimension::Overworld]
                .entities
                .as_ref()
                .unwrap()
                .weather_data
                .map(|weather| weather.thundering)
        };
        server.open_levels(2);
        wait_for_levels(&mut server);
        assert_eq!(
            server.globals.forced_weather.as_deref(),
            Some("thunder"),
            "the Nether has no weather to force"
        );
        server.players.get_mut(&id).unwrap().arrive(home);
        server.open_levels(2);
        wait_for_levels(&mut server);
        assert_eq!(thundering(&server), Some(true));
        assert_eq!(server.globals.forced_weather, None);
        server.players.get_mut(&id).unwrap().arrive(cave);
        assert_eq!(
            server.close_empty_levels(EMPTY_TIME),
            vec![Dimension::Overworld]
        );
        server.globals.weather = minecraft_terrain::server::weather_parameters("clear", 900);
        server.players.get_mut(&id).unwrap().arrive(home);
        server.open_levels(2);
        wait_for_levels(&mut server);
        assert_eq!(
            thundering(&server),
            Some(false),
            "a reopened overworld goes on with the world's weather"
        );
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
        assert!(!respawn(&server.globals, server.levels.get_mut(&Dimension::Nether).unwrap(), steve));
        assert_eq!(steve.respawning, None);
        let switch = steve.switching.expect("the respawn switches dimension");
        assert_eq!(
            (switch.from, switch.to, switch.reason, switch.respawn, switch.arrival),
            (Dimension::Nether, Dimension::Overworld, LoadingReason::Other, true, None)
        );
        assert!(!respawn_in(&server.globals, server.levels.get_mut(&Dimension::Overworld).unwrap(), steve, |_| None));
        let (x, _, z) = server.levels[&Dimension::Overworld].terrain.stream.world_spawn;
        let arrival = steve.arrival.expect("a bedless respawn arrives at the world spawn");
        assert_eq!(
            (arrival.dimension, arrival.yaw, arrival.pitch),
            (Dimension::Overworld, 90.0, 0.0),
            "the world spawn is in the overworld, facing the spawn angle"
        );
        assert!(
            (arrival.feet[0] - f64::from(x) - 0.5).abs() <= 10.0 && (arrival.feet[2] - f64::from(z) - 0.5).abs() <= 10.0,
            "within the respawn radius of the world spawn: {arrival:?}"
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
        assert!(!respawn(&server.globals, server.levels.get_mut(&Dimension::Overworld).unwrap(), steve));
        assert_eq!(steve.switching.map(|switch| (switch.to, switch.respawn)), Some((Dimension::Nether, true)));
        assert!(!respawn_in(&server.globals, server.levels.get_mut(&Dimension::Nether).unwrap(), steve, |_| None));
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
            respawn_in(&server.globals, server.levels.get_mut(&Dimension::Nether).unwrap(), steve, |_| None),
            "a point in the bedrock floor is missing"
        );
        assert_eq!(steve.respawn, None);
        assert_eq!(steve.switching.map(|switch| (switch.to, switch.arrival)), Some((Dimension::Overworld, None)));
    }

    #[test]
    fn a_nether_death_respawns_beside_a_charged_anchor_and_spends_a_charge() {
        use crate::player_nbt::{GlobalPos, Respawn};
        use crate::respawn::respawn;
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("anchor-respawn");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        steve.arrive(Place::standing(Dimension::Nether, [5.5, 100.0, 5.5], 0.0, 0.0));
        let id = steve.uuid.clone();
        let mut server = server_with(test_globals(registries, Some(save)), vec![steve]);
        server.open_levels(2);
        wait_for_levels(&mut server);
        let nether = server.levels.get_mut(&Dimension::Nether).unwrap();
        load_chunks_around(nether, (5, 100, 5));
        let states = nether.terrain.stream.states.clone();
        let block = |text: &str| {
            let state = states.registries().blocks.parse_state(text).unwrap();
            states.block(state).cloned()
        };
        let mut written = Vec::new();
        for x in 2..=8 {
            for z in 2..=8 {
                for y in 99..=104 {
                    let placed = match y {
                        99 => block("minecraft:obsidian"),
                        _ => None,
                    };
                    nether.terrain.scene.set((x, y, z), placed);
                    written.push((x, y, z));
                }
            }
        }
        nether
            .terrain
            .scene
            .set((5, 100, 5), block("minecraft:respawn_anchor[charges=2]"));
        nether.terrain.stream.record_edits(&nether.terrain.scene, &written);
        let anchor = Respawn {
            at: GlobalPos {
                dimension: Dimension::Nether,
                pos: (5, 100, 5),
            },
            yaw: 0.0,
            pitch: 0.0,
            forced: false,
        };
        let steve = server.players.get_mut(&id).unwrap();
        steve.set_respawn(Some(anchor));
        steve.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        assert!(steve.observe_life(0));
        let nether = server.levels.get_mut(&Dimension::Nether).unwrap();
        assert!(!respawn(&server.globals, nether, steve), "the anchor is there");
        assert!(steve.switching.is_none(), "the anchor is in the level the player died in");
        let arrival = steve.arrival.unwrap();
        assert_eq!((arrival.dimension, arrival.feet), (Dimension::Nether, [5.5, 100.0, 4.5]), "north of the anchor");
        assert!(arrival.yaw.abs() < 1.0e-4 && arrival.pitch == 0.0, "facing it");
        assert_eq!(steve.respawn, Some(anchor));
        let charged = |level: &Level| {
            minecraft_terrain::scene::Scene::block(&level.terrain.scene, (5, 100, 5))
                .and_then(|block| block.properties.get("charges").cloned())
        };
        assert_eq!(charged(nether), Some("1".to_owned()), "a charge is spent");
        steve.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        assert!(steve.observe_life(0));
        assert!(!respawn(&server.globals, nether, steve));
        assert_eq!(charged(nether), Some("0".to_owned()));
        steve.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        assert!(steve.observe_life(0));
        steve.arrival = None;
        assert!(
            respawn(&server.globals, nether, steve),
            "an empty anchor is missing"
        );
        assert_eq!(steve.respawn, None);
        assert_eq!(
            steve.switching.map(|switch| switch.to),
            Some(Dimension::Overworld),
            "and the player goes to the overworld's spawn"
        );
    }

    #[test]
    fn the_end_exit_portal_takes_a_player_beside_their_anchor_without_spending_a_charge() {
        use crate::player_nbt::{GlobalPos, Respawn};
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("anchor-end-exit");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        steve.arrive(Place::standing(Dimension::Nether, [5.5, 100.0, 5.5], 0.0, 0.0));
        let id = steve.uuid.clone();
        let mut server = server_with(test_globals(registries, Some(save)), vec![steve]);
        server.open_levels(2);
        wait_for_levels(&mut server);
        let nether = server.levels.get_mut(&Dimension::Nether).unwrap();
        load_chunks_around(nether, (5, 100, 5));
        let states = nether.terrain.stream.states.clone();
        let block = |text: &str| {
            let state = states.registries().blocks.parse_state(text).unwrap();
            states.block(state).cloned()
        };
        let mut written = Vec::new();
        for x in 2..=8 {
            for z in 2..=8 {
                for y in 99..=104 {
                    let placed = match y {
                        99 => block("minecraft:obsidian"),
                        _ => None,
                    };
                    nether.terrain.scene.set((x, y, z), placed);
                    written.push((x, y, z));
                }
            }
        }
        nether
            .terrain
            .scene
            .set((5, 100, 5), block("minecraft:respawn_anchor[charges=2]"));
        nether.terrain.stream.record_edits(&nether.terrain.scene, &written);
        let anchor = Respawn {
            at: GlobalPos {
                dimension: Dimension::Nether,
                pos: (5, 100, 5),
            },
            yaw: 0.0,
            pitch: 0.0,
            forced: false,
        };
        let steve = server.players.get_mut(&id).unwrap();
        steve.set_respawn(Some(anchor));
        steve.dimension = Dimension::End;
        crate::the_end::leave_the_end(steve, Dimension::Nether);
        let switch = steve.switching.expect("the exit portal switches");
        assert_eq!((switch.to, switch.respawn, switch.keep_charge), (Dimension::Nether, true, true));
        assert!(!crate::respawn_block::return_to_block(&server.globals, nether, steve, !switch.keep_charge));
        let arrival = steve.arrival.unwrap();
        assert_eq!((arrival.dimension, arrival.feet), (Dimension::Nether, [5.5, 100.0, 4.5]), "beside the anchor");
        assert_eq!(steve.respawn, Some(anchor));
        let charges = minecraft_terrain::scene::Scene::block(&nether.terrain.scene, (5, 100, 5))
            .and_then(|block| block.properties.get("charges").cloned());
        assert_eq!(charges, Some("2".to_owned()), "the exit portal spends no charge");
    }

    #[test]
    fn a_portal_trip_builds_its_exit_and_the_next_trip_finds_it() {
        use crate::portal::{PortalAxis, PortalKind, PortalTravel, portal_axis_in, set_off};
        let Some(registries) = registries() else {
            return;
        };
        let (_dir, save) = world("portal-trip");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let travel = PortalTravel {
            kind: PortalKind::Nether,
            to: Dimension::Nether,
            target: (100, 70, -200),
            entry_axis: PortalAxis::Z,
            relative: [0.5, 0.0, 0.0],
            yaw: 30.0,
            pitch: 5.0,
            motion: [0.0; 3],
            creates: true,
        };
        let mut steve = join(Some(&save), &recipes, &rules, joining(&profile("Steve")));
        steve.arrive(Place::standing(Dimension::Overworld, [800.5, 70.0, -1599.5], 30.0, 5.0));
        set_off(&mut steve, travel);
        let steve_id = steve.uuid.clone();
        let mut server = server_with(test_globals(registries, Some(save.clone())), vec![steve]);
        server.open_levels(2);
        assert_eq!(server.opening.keys().copied().collect::<Vec<_>>(), vec![Dimension::Nether]);
        assert_eq!(server.players[&steve_id].portal.travel, None, "the loader took the trip");
        assert_eq!(wait_for_levels(&mut server), vec![(Dimension::Nether, Ok(()))]);
        assert_eq!(server.levels[&Dimension::Nether].terrain.stream.player_spawn, (100.5, 70.0, -199.5));
        let block = |feet: [f64; 3]| (feet[0].floor() as i32, feet[1].floor() as i32, feet[2].floor() as i32);
        let first = server.players[&steve_id].arrival.expect("arrived");
        assert_eq!((first.dimension, first.yaw, first.pitch), (Dimension::Nether, 30.0, 5.0));
        let terrain = &server.levels[&Dimension::Nether].terrain;
        assert_eq!(portal_axis_in(terrain, block(first.feet)), Some(PortalAxis::Z), "{first:?}");
        assert!((first.feet[0] - 100.5).abs() <= 18.0 && (first.feet[2] + 199.5).abs() <= 18.0);
        let mut alex = join(Some(&save), &recipes, &rules, joining(&profile("Alex")));
        alex.arrive(Place::standing(Dimension::Overworld, [810.5, 70.0, -1590.5], 0.0, 0.0));
        set_off(&mut alex, PortalTravel { target: (101, 70, -199), entry_axis: PortalAxis::X, yaw: 0.0, ..travel });
        let alex_id = alex.uuid.clone();
        server.players.insert(alex_id.clone(), alex);
        server.open_levels(2);
        assert!(server.opening.is_empty(), "the open Nether takes the trip on the main thread");
        crate::portal_forcer::arrive_through_portals(&mut server);
        assert!(server.players[&alex_id].arrival.is_none(), "it waits until the player has left");
        server.players.get_mut(&alex_id).unwrap().switching.as_mut().unwrap().left = true;
        crate::portal_forcer::arrive_through_portals(&mut server);
        let second = server.players[&alex_id].arrival.expect("arrived");
        let terrain = &server.levels[&Dimension::Nether].terrain;
        assert_eq!(portal_axis_in(terrain, block(second.feet)), Some(PortalAxis::Z));
        assert_eq!(second.yaw, 90.0, "an X portal leads into a Z portal turned by 90");
        assert_eq!(block(second.feet).0, block(first.feet).0, "the same portal, not a new one");
        assert!((second.feet[2] - first.feet[2]).abs() < 2.0 && second.feet[1] == first.feet[1]);
        assert_eq!(server.players[&alex_id].portal.travel, None);
        let mut debug = join(Some(&save), &recipes, &rules, joining(&profile("Herobrine")));
        debug.arrive(Place::standing(Dimension::Overworld, [-40.0, 64.0, 80.0], 0.0, 0.0));
        start_in(&mut debug, Dimension::Nether, SPAWN);
        let mut players = BTreeMap::from([(debug.uuid.clone(), debug)]);
        let start = crate::portal_forcer::travel_into(&mut players, Dimension::Nether, &server.globals.registries).expect("a debug start comes through a portal");
        assert_eq!((start.target, start.entry_axis, start.relative), ((-5, 64, 10), PortalAxis::X, [0.5, 0.0, 0.0]));
        assert!(crate::portal_forcer::travel_into(&mut players, Dimension::End, &server.globals.registries).is_none());
    }

    #[test]
    fn the_stronghold_rings_are_worked_out_on_their_own_thread() {
        let Some(stream) = crate::commands::tests::test_stream() else {
            return;
        };
        let handle = resolve_rings_in_background(&stream).expect("a thread");
        assert_eq!(handle.thread().name(), Some("Stronghold rings"));
        assert_eq!(handle.join().ok(), Some(128));
        let started = Instant::now();
        assert_eq!(
            minecraft_terrain::locate::resolve_rings(&stream.world_gen()),
            128
        );
        assert!(
            started.elapsed() < std::time::Duration::from_millis(200),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_batch_left_by_an_early_return_is_sent_on_the_next_frame() {
        let Some(mut level) =
            crate::commands::tests::test_stream().and_then(crate::commands::tests::test_level)
        else {
            return;
        };
        let registries = level.terrain.stream.states.registries().clone();
        level.entities = Some(Entities::new(
            &level.terrain.stream,
            Dimension::Overworld,
            0,
            &GameData::default(),
            None,
            None,
        ));
        let mut server = Server {
            globals: test_globals(registries, None),
            levels: BTreeMap::from([(Dimension::Overworld, level)]),
            opening: BTreeMap::new(),
            players: BTreeMap::new(),
            published: false,
        };
        let base = Instant::now();
        let after = |millis| base + std::time::Duration::from_millis(millis);
        assert_eq!(server.plan_ticks(base, &[]).len(), 1);
        assert!(
            server.levels[&Dimension::Overworld].pending_batch.is_none(),
            "no ticking level holds it"
        );
        let planned = server.plan_ticks(after(150), &[Dimension::Overworld]);
        assert_eq!(planned.len(), 3);
        assert_eq!(server.levels[&Dimension::Overworld].container_ticks, 3);
        assert!(server.plan_ticks(after(300), &[Dimension::Overworld]).is_empty());
        assert_eq!(server.levels[&Dimension::Overworld].container_ticks, 3);
        assert_eq!(server.take_batch(Dimension::Overworld), planned);
        assert!(server.take_batch(Dimension::Overworld).is_empty());
        assert_eq!(server.plan_ticks(after(300), &[Dimension::Overworld]).len(), 3);
        assert_eq!(server.plan_ticks(after(450), &[]).len(), 3);
        assert!(
            server.levels[&Dimension::Overworld].pending_batch.is_none(),
            "a level nobody ticks any more lets its batch go"
        );
    }

    #[test]
    fn two_levels_each_tick_once_per_server_tick_and_the_clocks_advance_once() {
        let Some((overworld, nether)) = crate::commands::tests::test_stream()
            .and_then(crate::commands::tests::test_level)
            .zip(
                crate::commands::tests::test_stream()
                    .and_then(crate::commands::tests::test_level),
            )
        else {
            return;
        };
        let registries = overworld.terrain.stream.states.registries().clone();
        let mut levels = BTreeMap::new();
        for (dimension, mut level) in [(Dimension::Overworld, overworld), (Dimension::Nether, nether)] {
            level.dimension = dimension;
            level.entities = Some(Entities::new(
                &level.terrain.stream,
                dimension,
                0,
                &GameData::default(),
                None,
                None,
            ));
            levels.insert(dimension, level);
        }
        let mut server = Server {
            globals: test_globals(registries, None),
            levels,
            opening: BTreeMap::new(),
            players: BTreeMap::new(),
            published: false,
        };
        let base = Instant::now();
        let after = |millis| base + std::time::Duration::from_millis(millis);
        let both = [Dimension::Overworld, Dimension::Nether];
        server.plan_ticks(base, &both);
        let _ = (server.take_batch(Dimension::Overworld), server.take_batch(Dimension::Nether));
        let clocks = server.globals.clock_ticks();
        let ticks = server.globals.level_ticks;
        let planned = server.plan_ticks(after(150), &both);
        assert_eq!(planned.normal_count(), 3);
        for dimension in both {
            assert_eq!(server.levels[&dimension].pending_batch.as_ref(), Some(&planned), "{dimension:?}");
            assert_eq!(server.levels[&dimension].container_ticks, 4, "{dimension:?}");
        }
        assert_eq!(
            server.globals.clock_ticks(),
            ClockTicks {
                overworld: clocks.overworld + 3,
                the_end: clocks.the_end + 3,
            },
            "the clocks advance once for both levels"
        );
        assert_eq!(server.globals.level_ticks, ticks + 3);
        assert!(
            server.plan_ticks(after(300), &both).is_empty(),
            "no new ticks while a level still holds its batch"
        );
        assert_eq!(server.take_batch(Dimension::Nether), planned);
        assert!(server.plan_ticks(after(300), &both).is_empty());
        assert_eq!(server.take_batch(Dimension::Overworld), planned);
        assert_eq!(server.plan_ticks(after(300), &both).normal_count(), 3);
        assert_eq!(server.globals.clock_ticks().overworld, clocks.overworld + 6);
    }

    #[test]
    fn frozen_ticks_move_no_clock_and_no_weather_but_count_for_the_autosave_and_empty_levels() {
        let Some(registries) = registries() else {
            return;
        };
        let rain = WeatherData {
            rain_time: 100,
            raining: true,
            ..WeatherData::default()
        };
        let mut server = Server {
            globals: Globals {
                weather: Some(rain),
                ..test_globals(registries, None)
            },
            levels: BTreeMap::new(),
            opening: BTreeMap::new(),
            players: BTreeMap::new(),
            published: false,
        };
        let base = Instant::now();
        let after = |millis| base + std::time::Duration::from_millis(millis);
        let rain_time = |server: &Server| server.globals.weather.map(|weather| weather.rain_time);
        assert_eq!(server.plan_ticks(base, &[Dimension::Nether]).normal_count(), 1);
        assert_eq!(server.globals.clock_ticks(), ClockTicks { overworld: 1001, the_end: 1 });
        assert_eq!(rain_time(&server), Some(99));
        assert_eq!(server.globals.level_ticks, 1);
        server.globals.tick_rate.set_frozen(true);
        let frozen = server.plan_ticks(after(500), &[Dimension::Nether]);
        assert_eq!((frozen.len(), frozen.normal_count()), (10, 0));
        assert_eq!(server.globals.clock_ticks(), ClockTicks { overworld: 1001, the_end: 1 });
        assert_eq!(rain_time(&server), Some(99));
        assert_eq!(server.globals.level_ticks, 11);
        assert_eq!(server.globals.tick_rate.ticks_until_autosave(), 5989);
        assert!(!server.globals.autosave_due);
        for step in 1..=5989 {
            server.plan_ticks(after(500 + 50 * step), &[Dimension::Nether]);
        }
        assert!(server.globals.autosave_due, "frozen ticks count down to the autosave");
        assert_eq!(server.globals.tick_rate.ticks_until_autosave(), 6000);
        assert_eq!(server.globals.clock_ticks(), ClockTicks { overworld: 1001, the_end: 1 });
        server.globals.tick_rate.set_frozen(false);
        assert_eq!(server.plan_ticks(after(500 + 50 * 5990), &[Dimension::Nether]).normal_count(), 1);
        assert_eq!(server.globals.clock_ticks(), ClockTicks { overworld: 1002, the_end: 2 });
        assert_eq!(rain_time(&server), Some(98));
    }
}
