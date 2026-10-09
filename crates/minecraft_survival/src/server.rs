use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

use minecraft_commands::CommandServer;
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::game_mode::{GameMode, PlayerMode, from_id, parse};
use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::level_storage::WeatherData;
use minecraftoss_core::Registries;
use minecraftoss_player::Difficulty;

use crate::data::GameData;
use crate::level::{Level, Terrain};
use crate::player::Player;
use crate::save::WorldSave;

pub struct Server {
    pub globals: Globals,
    pub levels: BTreeMap<Dimension, Level>,
    pub players: BTreeMap<PlayerUuid, Player>,
}

pub struct Globals {
    pub seed: i64,
    pub registries: Arc<Registries>,
    pub save: Option<WorldSave>,
    pub game_rules: GameRules,
    pub difficulty: Difficulty,
    pub day: DayCycle,
    pub weather: Option<WeatherData>,
    pub chat: CommandServer,
    pub data: GameData,
    pub level_writer: Option<JoinHandle<()>>,
    pub autosave_ticks: u32,
}

impl Server {
    pub fn save_and_close(mut self, local: Option<&PlayerUuid>, started: Instant) {
        let save = self.globals.save.take();
        if let Some(writer) = self.globals.level_writer.take() {
            let _ = writer.join();
        }
        let Some(spawn) = self
            .levels
            .get(&Dimension::Overworld)
            .map(|level| level.terrain.stream.world_spawn)
        else {
            return;
        };
        let rules = std::mem::take(&mut self.globals.game_rules);
        let difficulty = self.globals.difficulty;
        let day_ticks = self.globals.day.ticks;
        let weather = self.globals.weather;
        let player_mode = local
            .and_then(|uuid| self.players.get(uuid))
            .map(|player| player.mode);
        drop(self);
        let Some(save) = save else {
            return;
        };
        if let Err(error) = save.write(spawn, Some(day_ticks), &rules, difficulty) {
            diag::warn!(World, "Minecraft level.dat not written: {error}");
        }
        if let Some(weather) = weather
            && let Err(error) = save.write_weather(&weather)
        {
            diag::warn!(World, "Minecraft weather.dat not written: {error}");
        }
        if let Some(player_mode) = player_mode
            && let Err(error) = save.write_host(&player_mode)
        {
            diag::warn!(World, "Minecraft player file not written: {error}");
        }
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

pub fn save_player_mode(globals: &mut Globals, mode: PlayerMode) {
    let Some(save) = globals.save.clone() else {
        return;
    };
    if let Some(previous) = globals.level_writer.take() {
        let _ = previous.join();
    }
    globals.level_writer = std::thread::Builder::new()
        .name("minecraft-player-save".into())
        .spawn(move || {
            if let Err(error) = save.write_host(&mode) {
                diag::warn!(World, "Minecraft player file not written: {error}");
            }
        })
        .map_err(|error| diag::warn!(World, "Minecraft player file thread not started: {error}"))
        .ok();
}

pub fn autosave(globals: &mut Globals, terrain: &mut Terrain, mode: PlayerMode) {
    let Some(save) = globals.save.clone() else {
        return;
    };
    let started = Instant::now();
    let chunks = terrain.stream.queue_save();
    if let Some(previous) = globals.level_writer.take() {
        let _ = previous.join();
    }
    let path = save.path().display().to_string();
    let spawn = terrain.stream.world_spawn;
    let rules = globals.game_rules.clone();
    let difficulty = globals.difficulty;
    let day_ticks = globals.day.ticks;
    let weather = globals.weather;
    globals.level_writer = std::thread::Builder::new()
        .name("minecraft-level-save".into())
        .spawn(move || {
            if let Err(error) = save.write(spawn, Some(day_ticks), &rules, difficulty) {
                diag::warn!(World, "Minecraft autosave: level.dat not written: {error}");
            }
            if let Some(weather) = weather
                && let Err(error) = save.write_weather(&weather)
            {
                diag::warn!(
                    World,
                    "Minecraft autosave: weather.dat not written: {error}"
                );
            }
            if let Err(error) = save.write_host(&mode) {
                diag::warn!(
                    World,
                    "Minecraft autosave: player file not written: {error}"
                );
            }
        })
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
}
