use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::level_storage::{
    ClockState, LevelData, LevelStorage, RespawnData, WorldSettings,
};

pub(crate) const DEFAULT_WORLD: &str = "world";
pub(crate) const TEMPORARY_WORLD: &str = "temp";
pub(crate) const AUTOSAVE_TICKS: u32 = 6000;
const OVERWORLD: &str = "minecraft:overworld";

pub(crate) fn saves_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("iw4l-artifacts")
        .join("saves")
}

pub(crate) fn random_seed() -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    (nanos as i64) ^ 0x5DEE_CE66_D1CE_4E5B
}

pub(crate) fn world_name(value: Option<&str>) -> Result<String, String> {
    let name = value.map_or("", str::trim);
    if name.is_empty() {
        return Ok(DEFAULT_WORLD.to_owned());
    }
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Ok(name.to_owned())
    } else {
        Err(format!(
            "IW4L_MINECRAFT_WORLD `{name}` may only use letters, digits, _ and -"
        ))
    }
}

pub(crate) fn requested_seed(value: Option<&str>) -> Option<i64> {
    value.and_then(|seed| seed.trim().parse().ok())
}

pub(crate) fn choose_seed(
    stored: Option<i64>,
    requested: Option<i64>,
    random: i64,
    name: &str,
) -> (i64, Option<String>) {
    match (stored, requested) {
        (Some(stored), Some(requested)) if requested != stored => (
            stored,
            Some(format!(
                "IW4L_MINECRAFT_SEED ignored: world '{name}' has seed {stored}"
            )),
        ),
        (Some(stored), _) => (stored, None),
        (None, Some(requested)) => (requested, None),
        (None, None) => (random, None),
    }
}

#[derive(Clone)]
pub(crate) struct WorldSave {
    storage: Arc<LevelStorage>,
    level: LevelData,
    day_at_start: Option<f64>,
}

impl WorldSave {
    pub(crate) fn path(&self) -> &Path {
        self.storage.root()
    }

    pub(crate) fn started(&mut self, day_ticks: f64) {
        self.day_at_start = Some(day_ticks);
    }

    pub(crate) fn write(
        &self,
        spawn: (i32, i32, i32),
        day_ticks: Option<f64>,
        game_rules: &GameRules,
    ) -> Result<(), String> {
        let elapsed = match (day_ticks, self.day_at_start) {
            (Some(now), Some(start)) => (now - start).max(0.0) as i64,
            _ => 0,
        };
        let level = LevelData {
            game_time: self.level.game_time + elapsed,
            spawn: RespawnData {
                dimension: OVERWORLD.to_owned(),
                pos: spawn,
                ..self.level.spawn.clone()
            },
            initialized: true,
            ..self.level.clone()
        };
        self.storage.write_level_data(&level)?;
        self.storage.write_game_rules(game_rules)?;
        let Some(day_ticks) = day_ticks else {
            return Ok(());
        };
        let mut clocks = self
            .storage
            .read_clocks()
            .unwrap_or_else(|_| BTreeMap::new());
        clocks.insert(OVERWORLD.to_owned(), ClockState::at(day_ticks));
        self.storage.write_clocks(&clocks)
    }
}

pub(crate) struct OpenedWorld {
    pub(crate) seed: i64,
    pub(crate) save: Option<WorldSave>,
    pub(crate) spawn: Option<(i32, i32, i32)>,
    pub(crate) day_ticks: Option<f64>,
    pub(crate) game_rules: GameRules,
    pub(crate) notes: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

impl OpenedWorld {
    fn temporary(seed: i64, warnings: Vec<String>) -> Self {
        Self {
            seed,
            save: None,
            spawn: None,
            day_ticks: None,
            game_rules: GameRules::default(),
            notes: vec![format!(
                "Minecraft world: temporary, deleted when it closes, seed {seed}"
            )],
            warnings,
        }
    }
}

pub(crate) fn open_world(
    saves: &Path,
    name: Option<&str>,
    seed: Option<&str>,
    random: i64,
) -> OpenedWorld {
    let requested = requested_seed(seed);
    let name = match world_name(name) {
        Ok(name) => name,
        Err(error) => return OpenedWorld::temporary(requested.unwrap_or(random), vec![error]),
    };
    if name == TEMPORARY_WORLD {
        return OpenedWorld::temporary(requested.unwrap_or(random), Vec::new());
    }
    let path = saves.join(&name);
    open_save(&path, &name, requested, random).unwrap_or_else(|error| {
        OpenedWorld::temporary(
            requested.unwrap_or(random),
            vec![format!(
                "Minecraft world {} not opened, playing a temporary world instead: {error}",
                path.display()
            )],
        )
    })
}

fn open_save(
    path: &Path,
    name: &str,
    requested: Option<i64>,
    random: i64,
) -> Result<OpenedWorld, String> {
    let storage = LevelStorage::open(path)?;
    let stored = storage.read_world_settings()?;
    let mut warnings = Vec::new();
    let level = storage.read_level_data().unwrap_or_else(|error| {
        warnings.push(format!(
            "Minecraft level.dat unreadable, starting it again: {error}"
        ));
        None
    });
    let clocks = storage.read_clocks().unwrap_or_else(|error| {
        warnings.push(format!(
            "Minecraft world_clocks unreadable, time of day reset: {error}"
        ));
        BTreeMap::new()
    });
    let game_rules = storage.read_game_rules().unwrap_or_else(|error| {
        warnings.push(format!(
            "Minecraft game_rules unreadable, using the defaults: {error}"
        ));
        GameRules::default()
    });
    let (seed, warning) = choose_seed(stored.map(|s| s.seed), requested, random, name);
    warnings.extend(warning);
    if stored.is_none() {
        storage.write_world_settings(&WorldSettings::new(seed))?;
    }
    storage.create_dimension(OVERWORLD)?;
    let spawn = level
        .as_ref()
        .filter(|level| level.initialized && level.spawn.dimension == OVERWORLD)
        .map(|level| level.spawn.pos);
    let mut level = level.unwrap_or_else(|| LevelData::new(name));
    if level.level_name.is_empty() {
        level.level_name = name.to_owned();
    }
    let verb = if stored.is_some() {
        "opened"
    } else {
        "created"
    };
    Ok(OpenedWorld {
        seed,
        spawn,
        day_ticks: clocks.get(OVERWORLD).map(ClockState::ticks),
        game_rules,
        notes: vec![format!(
            "Minecraft world: {verb} {} seed {seed}",
            path.display()
        )],
        warnings,
        save: Some(WorldSave {
            storage: Arc::new(storage),
            level,
            day_at_start: None,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TestDir(PathBuf);

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn saves() -> TestDir {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "render_anim-saves-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        TestDir(dir)
    }

    #[test]
    fn world_names_stay_inside_the_saves_folder() {
        assert_eq!(world_name(None).unwrap(), "world");
        assert_eq!(world_name(Some("  ")).unwrap(), "world");
        assert_eq!(world_name(Some(" test_2-b ")).unwrap(), "test_2-b");
        for bad in [
            "..",
            "..\\x",
            "../x",
            "a/b",
            "a b",
            "C:",
            "w\u{f6}rld",
            "x.y",
        ] {
            assert!(world_name(Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_stored_seed_wins_and_a_different_request_is_reported() {
        assert_eq!(choose_seed(None, None, 9, "world"), (9, None));
        assert_eq!(choose_seed(None, Some(123), 9, "world"), (123, None));
        assert_eq!(choose_seed(Some(5), None, 9, "world"), (5, None));
        assert_eq!(choose_seed(Some(5), Some(5), 9, "world"), (5, None));
        let (seed, warning) = choose_seed(Some(5), Some(123), 9, "world");
        assert_eq!(seed, 5);
        assert_eq!(
            warning.as_deref(),
            Some("IW4L_MINECRAFT_SEED ignored: world 'world' has seed 5")
        );
        assert_eq!(requested_seed(Some(" -42 ")), Some(-42));
        assert_eq!(requested_seed(Some("abc")), None);
    }

    #[test]
    fn a_new_world_keeps_its_seed_spawn_and_time_when_opened_again() {
        let dir = saves();
        let first = open_world(&dir.0, None, Some("123"), 9);
        assert_eq!(first.seed, 123);
        assert!(first.warnings.is_empty());
        assert!(first.notes[0].starts_with("Minecraft world: created "));
        assert!(first.notes[0].ends_with(" seed 123"));
        assert_eq!(first.spawn, None);
        assert_eq!(first.day_ticks, None);
        let world = dir.0.join("world");
        assert!(
            world
                .join("data/minecraft/world_gen_settings.dat")
                .is_file()
        );
        for folder in ["region", "entities", "poi", "data"] {
            assert!(
                world
                    .join("dimensions/minecraft/overworld")
                    .join(folder)
                    .is_dir()
            );
        }
        let mut save = first.save.expect("a real world");
        assert_eq!(save.path(), world);
        let rules = GameRules::default();
        save.write((12, 70, -4), None, &rules).unwrap();
        save.started(1000.0);
        save.write((12, 70, -4), Some(19_000.25), &rules).unwrap();
        drop(save);
        let again = open_world(&dir.0, Some("world"), Some("77"), 9);
        assert_eq!(again.seed, 123);
        assert_eq!(
            again.warnings,
            ["IW4L_MINECRAFT_SEED ignored: world 'world' has seed 123"]
        );
        assert!(again.notes[0].starts_with("Minecraft world: opened "));
        assert_eq!(again.spawn, Some((12, 70, -4)));
        assert_eq!(again.day_ticks, Some(19_000.25));
        let level = again.save.as_ref().unwrap().level.clone();
        assert_eq!(level.game_time, 18_000);
        assert_eq!(level.level_name, "world");
        let other = open_world(&dir.0, Some("other"), None, 31);
        assert_eq!(other.seed, 31);
    }

    #[test]
    fn a_world_in_use_falls_back_to_a_temporary_one() {
        let dir = saves();
        let holder = open_world(&dir.0, None, None, 4);
        assert!(holder.save.is_some());
        let second = open_world(&dir.0, None, Some("8"), 5);
        assert!(second.save.is_none());
        assert_eq!(second.seed, 8);
        assert!(
            second.warnings[0].contains("temporary world"),
            "{:?}",
            second.warnings
        );
        drop(holder);
        let reopened = open_world(&dir.0, None, None, 6);
        assert_eq!(reopened.seed, 4);
    }

    #[test]
    fn unreadable_level_data_and_clocks_keep_the_world_and_its_seed() {
        let dir = saves();
        let first = open_world(&dir.0, None, Some("55"), 9);
        first
            .save
            .as_ref()
            .unwrap()
            .write((1, 64, 2), Some(700.0), &GameRules::default())
            .unwrap();
        drop(first);
        let world = dir.0.join("world");
        for file in [
            "level.dat",
            "level.dat_old",
            "data/minecraft/world_clocks.dat",
            "data/minecraft/world_clocks.dat_old",
        ] {
            let _ = std::fs::write(world.join(file), b"broken");
        }
        let again = open_world(&dir.0, None, None, 9);
        assert!(again.save.is_some());
        assert_eq!(again.seed, 55);
        assert_eq!(again.spawn, None);
        assert_eq!(again.day_ticks, None);
        assert_eq!(again.warnings.len(), 2, "{:?}", again.warnings);
    }

    #[test]
    fn game_rules_survive_reopening_a_world() {
        let dir = saves();
        let first = open_world(&dir.0, None, Some("8"), 9);
        assert!(!first.game_rules.keep_inventory());
        let mut rules = first.game_rules.clone();
        rules.set("keep_inventory", "true").unwrap();
        first
            .save
            .as_ref()
            .unwrap()
            .write((0, 64, 0), None, &rules)
            .unwrap();
        drop(first);
        let world = dir.0.join("world");
        assert!(world.join("data/minecraft/game_rules.dat").is_file());
        let again = open_world(&dir.0, None, None, 9);
        assert!(again.game_rules.keep_inventory());
        assert!(again.warnings.is_empty(), "{:?}", again.warnings);
        drop(again);
        for file in [
            "data/minecraft/game_rules.dat",
            "data/minecraft/game_rules.dat_old",
        ] {
            let _ = std::fs::write(world.join(file), b"broken");
        }
        let broken = open_world(&dir.0, None, None, 9);
        assert!(broken.save.is_some());
        assert!(!broken.game_rules.keep_inventory());
        assert_eq!(broken.warnings.len(), 1, "{:?}", broken.warnings);
        assert!(broken.warnings[0].contains("game_rules"));
        let temp = open_world(&dir.0, Some("temp"), None, 9);
        assert_eq!(temp.game_rules, GameRules::default());
    }

    #[test]
    fn temp_and_invalid_names_play_without_a_folder() {
        let dir = saves();
        let temp = open_world(&dir.0, Some("temp"), Some("3"), 9);
        assert!(temp.save.is_none());
        assert_eq!(temp.seed, 3);
        assert!(temp.warnings.is_empty());
        let bad = open_world(&dir.0, Some("../escape"), None, 9);
        assert!(bad.save.is_none());
        assert_eq!(bad.seed, 9);
        assert_eq!(bad.warnings.len(), 1);
        assert!(!dir.0.exists());
    }
}
