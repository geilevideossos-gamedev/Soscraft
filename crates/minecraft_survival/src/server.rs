use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

use minecraft_commands::CommandServer;
use minecraft_commands::server::Profile;
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::game_mode::{GameMode, PlayerMode, from_id, parse};
use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::level_storage::WeatherData;
use minecraftoss_core::Registries;
use minecraftoss_player::Difficulty;
use minecraftoss_player::crafting::RecipeBook;

use crate::data::GameData;
use crate::level::Level;
use crate::player::{Player, joining_inventory};
use crate::player_nbt::PlayerData;
use crate::save::WorldSave;
use crate::survival::Survival;

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
    pub fn save_and_close(mut self, started: Instant) {
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
        let day = self.globals.day;
        let weather = self.globals.weather;
        let players = player_files(&self.players);
        drop(self);
        let Some(save) = save else {
            return;
        };
        if let Err(error) = save.write(spawn, Some(day), &rules, difficulty) {
            diag::warn!(World, "Minecraft level.dat not written: {error}");
        }
        if let Some(weather) = weather
            && let Err(error) = save.write_weather(&weather)
        {
            diag::warn!(World, "Minecraft weather.dat not written: {error}");
        }
        write_players(&save, &players, "Minecraft player file not written");
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

pub fn join(
    save: Option<&WorldSave>,
    recipes: &Arc<RecipeBook>,
    rules: &GameRules,
    profile: &Profile,
    chosen_mode: Option<&str>,
    kit: Option<&str>,
    carried: Option<Survival>,
) -> Player {
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
    let survival = match (saved.stats, carried) {
        (Some(stats), _) => {
            let mut survival = Survival::default();
            survival.restore(stats.status, stats.fire, stats.score);
            if stats.status.health <= 0.0 {
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
    player
}

fn player_files(players: &BTreeMap<PlayerUuid, Player>) -> Vec<(String, PlayerData)> {
    players
        .values()
        .map(|player| (player.uuid.0.clone(), player.data()))
        .collect()
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
    let Some(spawn) = levels
        .get(&Dimension::Overworld)
        .map(|level| level.terrain.stream.world_spawn)
    else {
        return;
    };
    let started = Instant::now();
    let chunks: usize = levels
        .values_mut()
        .map(|level| level.terrain.stream.queue_save())
        .sum();
    if let Some(previous) = globals.level_writer.take() {
        let _ = previous.join();
    }
    let path = save.path().display().to_string();
    let rules = globals.game_rules.clone();
    let difficulty = globals.difficulty;
    let day = globals.day;
    let weather = globals.weather;
    let players = player_files(players);
    globals.level_writer = std::thread::Builder::new()
        .name("minecraft-level-save".into())
        .spawn(move || {
            if let Err(error) = save.write(spawn, Some(day), &rules, difficulty) {
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
            write_players(
                &save,
                &players,
                "Minecraft autosave: player file not written",
            );
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

    #[test]
    fn a_joining_player_gets_the_inventory_and_mode_their_file_saved() {
        let (_dir, save) = world("join");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, &steve, None, None, None);
        assert_eq!(player.mode, PlayerMode::new(GameMode::Survival));
        assert!(player.inventory.slots.iter().all(Option::is_none));
        player.mode = PlayerMode::new(GameMode::Creative);
        player.inventory.slots[2] = Some(ItemStack::new("minecraft:diamond", 3));
        player.inventory.slots[38] = Some(ItemStack::new("minecraft:iron_chestplate", 1));
        player.selected = 2;
        save.write_player(&player.uuid.0, &player.data()).unwrap();
        let again = join(Some(&save), &recipes, &rules, &steve, None, None, None);
        assert_eq!(again.mode, PlayerMode::new(GameMode::Creative));
        assert_eq!(again.selected, 2);
        assert_eq!(held(&again, 2), Some(("minecraft:diamond", 3)));
        assert_eq!(held(&again, 38), Some(("minecraft:iron_chestplate", 1)));
        assert!(Arc::ptr_eq(&again.inventory.recipes, &recipes));
        let alex = join(
            Some(&save),
            &recipes,
            &rules,
            &profile("Alex"),
            Some("adventure"),
            None,
            None,
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
        let first = join(
            Some(&save),
            &recipes,
            &rules,
            &steve,
            None,
            Some("dirt*5"),
            None,
        );
        assert_eq!(first.mode.mode, GameMode::Creative);
        assert_eq!(held(&first, 9), Some(("minecraft:dirt", 5)));
        let mut emptied = first;
        emptied.inventory.slots.fill(None);
        save.write_player(&steve.uuid, &emptied.data()).unwrap();
        let again = join(
            Some(&save),
            &recipes,
            &rules,
            &steve,
            None,
            Some("dirt*5"),
            None,
        );
        assert!(again.inventory.slots.iter().all(Option::is_none));
        let alex = join(
            Some(&save),
            &recipes,
            &rules,
            &profile("Alex"),
            None,
            Some("dirt*5"),
            None,
        );
        assert_eq!(held(&alex, 9), Some(("minecraft:dirt", 5)));
    }

    #[test]
    fn a_joining_player_gets_back_the_health_hunger_and_experience_they_saved() {
        let (_dir, save) = world("stats");
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let steve = profile("Steve");
        let mut player = join(Some(&save), &recipes, &rules, &steve, None, None, None);
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
        let mut again = join(
            Some(&save),
            &recipes,
            &rules,
            &steve,
            None,
            None,
            Some(carried),
        );
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
        let mut player = join(Some(&save), &recipes, &rules, &steve, None, None, None);
        player.survival.status.experience_level = 12;
        player.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        save.write_player(&steve.uuid, &player.data()).unwrap();
        let mut carried = Survival::default();
        carried.status.health = 2.0;
        let again = join(
            Some(&save),
            &recipes,
            &rules,
            &steve,
            None,
            None,
            Some(carried),
        );
        assert_eq!(again.survival.status.health, 20.0);
        assert_eq!(again.survival.status.experience_level, 0);
        assert!(!again.dead);
        let mut keeping = GameRules::default();
        keeping.set("keep_inventory", "true").unwrap();
        let kept = join(Some(&save), &recipes, &keeping, &steve, None, None, None);
        assert_eq!(kept.survival.status.health, 20.0);
        assert_eq!(kept.survival.status.experience_level, 12);
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
        let player = join(
            Some(&save),
            &recipes,
            &rules,
            &steve,
            None,
            Some("dirt"),
            None,
        );
        assert_eq!(player.mode, PlayerMode::new(GameMode::Survival));
        assert_eq!(held(&player, 9), Some(("minecraft:dirt", 1)));
    }

    #[test]
    fn a_temporary_world_gives_the_kit_and_the_survival_carried_over() {
        let recipes = Arc::new(RecipeBook::default());
        let rules = GameRules::default();
        let mut carried = Survival::default();
        carried.observe(100, 100);
        carried.status.health = 7.0;
        let player = join(
            None,
            &recipes,
            &rules,
            &profile("Steve"),
            Some("creative"),
            Some("dirt*2"),
            Some(carried),
        );
        assert_eq!(player.survival.status.health, 7.0);
        assert_eq!(player.mode.mode, GameMode::Creative);
        assert_eq!(held(&player, 9), Some(("minecraft:dirt", 2)));
    }
}
