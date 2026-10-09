use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::JoinHandle;

use minecraft_commands::CommandServer;
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::game_rules::GameRules;
use minecraftoss_core::Registries;
use minecraftoss_player::Difficulty;

use crate::data::GameData;
use crate::level::Level;
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
    pub chat: CommandServer,
    pub data: GameData,
    pub level_writer: Option<JoinHandle<()>>,
    pub autosave_ticks: u32,
}
