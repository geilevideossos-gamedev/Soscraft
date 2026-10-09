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
use crate::player_nbt::{Place, PlayerData};
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
    player.place = Some(Place::standing(Dimension::Overworld, spawn, 0.0, 0.0));
    match saved.place {
        Some(_) if dead => {}
        Some(place) if place.dimension == Dimension::Overworld => player.arrive(place),
        Some(place) => diag::info!(
            World,
            "Minecraft player {} was saved in {}, which cannot be entered yet: back at the world spawn",
            profile.name,
            place.dimension.dimension_type()
        ),
        None => {}
    }
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
        let again = join(Some(&save), &recipes, &rules, carrying);
        assert_eq!(again.survival.status.health, 20.0);
        assert_eq!(again.survival.status.experience_level, 0);
        assert!(!again.dead);
        assert_eq!(again.arrival, None);
        assert_eq!(again.place.map(|place| place.feet), Some(SPAWN));
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
        assert_eq!(back.arrival, None);
        assert_eq!(back.dimension, Dimension::Overworld);
        assert_eq!(back.place.map(|place| place.feet), Some(SPAWN));
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
}
