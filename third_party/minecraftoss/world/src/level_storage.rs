use crate::game_rules::{GAME_RULES, GameRules};
pub use crate::settings::WorldSettings;
use crate::storage::ChunkStorage;
use minecraftoss_core::Registries;
use minecraftoss_core::chunk_nbt::DATA_VERSION;
use minecraftoss_core::nbt::{self, Tag};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

pub const LEVEL_DATA_FILE: &str = "level.dat";
pub const LOCK_FILE: &str = "session.lock";
pub const DATA_FOLDER: &str = "data";
pub const DIMENSIONS_FOLDER: &str = "dimensions";
pub const DIMENSION_FOLDERS: [&str; 4] = ["region", "entities", "poi", "data"];
pub const STORAGE_VERSION: i32 = 19133;
pub const VERSION_NAME: &str = "26.3";
pub const VERSION_SERIES: &str = "main";
pub const SERVER_BRAND: &str = "iw4l";
pub const WORLD_CLOCKS: &str = "world_clocks";
pub const WEATHER: &str = "weather";
pub const PLAYERS_FOLDER: &str = "players";
const LOCK_CONTENT: &str = "\u{2603}";

#[derive(Clone, Debug, PartialEq)]
pub struct RespawnData {
    pub dimension: String,
    pub pos: (i32, i32, i32),
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for RespawnData {
    fn default() -> Self {
        Self {
            dimension: "minecraft:overworld".to_owned(),
            pos: (0, 0, 0),
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LevelData {
    pub level_name: String,
    pub game_time: i64,
    pub spawn: RespawnData,
    pub difficulty: String,
    pub hardcore: bool,
    pub initialized: bool,
    pub allow_commands: bool,
    pub game_type: i32,
    pub singleplayer_uuid: Option<String>,
}

impl LevelData {
    pub fn new(level_name: &str) -> Self {
        Self {
            level_name: level_name.to_owned(),
            game_time: 0,
            spawn: RespawnData::default(),
            difficulty: "normal".to_owned(),
            hardcore: false,
            initialized: false,
            allow_commands: true,
            game_type: 0,
            singleplayer_uuid: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockState {
    pub total_ticks: i64,
    pub partial_tick: f32,
    pub rate: f32,
    pub paused: bool,
}

impl ClockState {
    pub fn at(ticks: f64) -> Self {
        let whole = ticks.floor();
        Self {
            total_ticks: whole as i64,
            partial_tick: (ticks - whole) as f32,
            rate: 1.0,
            paused: false,
        }
    }

    pub fn ticks(&self) -> f64 {
        self.total_ticks as f64 + f64::from(self.partial_tick)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeatherData {
    pub clear_weather_time: i32,
    pub rain_time: i32,
    pub thunder_time: i32,
    pub raining: bool,
    pub thundering: bool,
}

pub struct LevelStorage {
    root: PathBuf,
    _lock: File,
}

impl LevelStorage {
    pub fn open(root: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|e| format!("{}: {e}", root.display()))?;
        let path = root.join(LOCK_FILE);
        let mut lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(format!("{} is already in use", root.display()));
            }
            Err(TryLockError::Error(e)) => return Err(format!("{}: {e}", path.display())),
        }
        let mark = |lock: &mut File| -> std::io::Result<()> {
            lock.set_len(0)?;
            lock.write_all(LOCK_CONTENT.as_bytes())?;
            lock.sync_all()
        };
        mark(&mut lock).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self {
            root: root.to_path_buf(),
            _lock: lock,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn dimension_path(&self, dimension: &str) -> PathBuf {
        dimension_path(&self.root, dimension)
    }

    pub fn create_dimension(&self, dimension: &str) -> Result<PathBuf, String> {
        let base = self.dimension_path(dimension);
        for folder in DIMENSION_FOLDERS {
            let dir = base.join(folder);
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        Ok(base)
    }

    pub fn chunk_storage(
        &self,
        dimension: &str,
        registries: Arc<Registries>,
        min_y: i32,
        height: i32,
    ) -> ChunkStorage {
        ChunkStorage::new(&self.root, dimension, registries, min_y, height)
    }

    pub fn saved_data_path(&self, id: &str) -> PathBuf {
        let (namespace, path) = split_id(id);
        self.root
            .join(DATA_FOLDER)
            .join(namespace)
            .join(format!("{path}.dat"))
    }

    pub fn read_saved(&self, id: &str) -> Result<Option<Tag>, String> {
        read_saved_at(&self.saved_data_path(id))
    }

    pub fn write_saved(&self, id: &str, data: Tag) -> Result<(), String> {
        write_saved_at(&self.saved_data_path(id), data)
    }

    pub fn dimension_data_path(&self, dimension: &str, id: &str) -> PathBuf {
        let (namespace, path) = split_id(id);
        self.dimension_path(dimension)
            .join(DATA_FOLDER)
            .join(namespace)
            .join(format!("{path}.dat"))
    }

    pub fn read_saved_in(&self, dimension: &str, id: &str) -> Result<Option<Tag>, String> {
        read_saved_at(&self.dimension_data_path(dimension, id))
    }

    pub fn write_saved_in(&self, dimension: &str, id: &str, data: Tag) -> Result<(), String> {
        write_saved_at(&self.dimension_data_path(dimension, id), data)
    }

    pub fn read_world_settings(&self) -> Result<Option<WorldSettings>, String> {
        WorldSettings::read(&self.root)
    }

    pub fn write_world_settings(&self, settings: &WorldSettings) -> Result<(), String> {
        settings.write(&self.root)
    }

    pub fn level_data_path(&self) -> PathBuf {
        self.root.join(LEVEL_DATA_FILE)
    }

    pub fn read_level_data(&self) -> Result<Option<LevelData>, String> {
        let Some(root) = read_nbt_or_old(&self.level_data_path())? else {
            return Ok(None);
        };
        let data = root.get("Data").ok_or("level.dat lacks Data")?;
        let text = |tag: Option<&Tag>, default: &str| {
            tag.and_then(Tag::as_str).unwrap_or(default).to_owned()
        };
        let flag = |tag: Option<&Tag>| tag.and_then(Tag::as_i64).is_some_and(|v| v != 0);
        let float = |tag: Option<&Tag>| tag.and_then(Tag::as_f64).unwrap_or(0.0) as f32;
        let spawn = data
            .get("spawn")
            .map_or_else(RespawnData::default, |spawn| {
                let pos = spawn
                    .get("pos")
                    .and_then(Tag::as_ints)
                    .unwrap_or_else(Vec::new);
                RespawnData {
                    dimension: text(spawn.get("dimension"), "minecraft:overworld"),
                    pos: match pos.as_slice() {
                        [x, y, z] => (*x, *y, *z),
                        _ => (0, 0, 0),
                    },
                    yaw: float(spawn.get("yaw")),
                    pitch: float(spawn.get("pitch")),
                }
            });
        let difficulty = data.get("difficulty_settings");
        Ok(Some(LevelData {
            level_name: text(data.get("LevelName"), ""),
            game_time: data.get("Time").and_then(Tag::as_i64).unwrap_or(0),
            spawn,
            difficulty: text(difficulty.and_then(|d| d.get("difficulty")), "normal"),
            hardcore: flag(difficulty.and_then(|d| d.get("hardcore"))),
            initialized: flag(data.get("initialized")),
            allow_commands: flag(data.get("allowCommands")),
            game_type: data.get("GameType").and_then(Tag::as_i64).unwrap_or(0) as i32,
            singleplayer_uuid: data
                .get("singleplayer_uuid")
                .and_then(Tag::as_ints)
                .and_then(|ints| uuid_from_ints(&ints)),
        }))
    }

    pub fn write_level_data(&self, level: &LevelData) -> Result<(), String> {
        let path = self.level_data_path();
        let previous = read_nbt_or_old(&path).ok().flatten();
        let mut data = previous
            .as_ref()
            .and_then(|root| root.get("Data"))
            .and_then(Tag::as_compound)
            .cloned()
            .unwrap_or_else(BTreeMap::new);
        let mut brands: Vec<Tag> = data
            .get("ServerBrands")
            .and_then(Tag::as_list)
            .map_or_else(Vec::new, <[Tag]>::to_vec);
        if !brands.iter().any(|b| b.as_str() == Some(SERVER_BRAND)) {
            brands.push(Tag::String(SERVER_BRAND.to_owned()));
        }
        let mut history: Vec<Tag> = data
            .get("version_history")
            .and_then(Tag::as_list)
            .map_or_else(Vec::new, <[Tag]>::to_vec);
        if history.last().and_then(Tag::as_i64) != Some(i64::from(DATA_VERSION)) {
            history.push(Tag::Int(DATA_VERSION));
        }
        let last_played = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let (x, y, z) = level.spawn.pos;
        let entries = [
            ("ServerBrands", Tag::List(brands)),
            ("WasModded", Tag::Byte(1)),
            (
                "Version",
                compound(vec![
                    ("Id", Tag::Int(DATA_VERSION)),
                    ("Name", string(VERSION_NAME)),
                    ("Series", string(VERSION_SERIES)),
                    ("Snapshot", Tag::Byte(0)),
                ]),
            ),
            ("version_history", Tag::List(history)),
            ("DataVersion", Tag::Int(DATA_VERSION)),
            (
                "spawn",
                compound(vec![
                    ("dimension", string(&level.spawn.dimension)),
                    ("pos", Tag::IntArray(vec![x, y, z])),
                    ("yaw", Tag::Float(level.spawn.yaw)),
                    ("pitch", Tag::Float(level.spawn.pitch)),
                ]),
            ),
            ("Time", Tag::Long(level.game_time)),
            ("LastPlayed", Tag::Long(last_played)),
            ("LevelName", string(&level.level_name)),
            ("version", Tag::Int(STORAGE_VERSION)),
            ("initialized", Tag::Byte(i8::from(level.initialized))),
            ("allowCommands", Tag::Byte(i8::from(level.allow_commands))),
            ("GameType", Tag::Int(level.game_type)),
            (
                "difficulty_settings",
                compound(vec![
                    ("difficulty", string(&level.difficulty)),
                    ("hardcore", Tag::Byte(i8::from(level.hardcore))),
                    ("locked", Tag::Byte(0)),
                ]),
            ),
        ];
        for (key, value) in entries {
            data.insert(key.to_owned(), value);
        }
        if let Some(ints) = level.singleplayer_uuid.as_deref().and_then(uuid_to_ints) {
            data.insert("singleplayer_uuid".to_owned(), Tag::IntArray(ints.to_vec()));
        }
        let defaults = [
            (
                "DataPacks",
                compound(vec![
                    ("Enabled", Tag::List(vec![string("vanilla")])),
                    ("Disabled", Tag::List(Vec::new())),
                ]),
            ),
            (
                "enabled_features",
                Tag::List(vec![string("minecraft:vanilla")]),
            ),
        ];
        for (key, value) in defaults {
            data.entry(key.to_owned()).or_insert(value);
        }
        let root = compound(vec![("Data", Tag::Compound(data))]);
        safe_replace(&path, &nbt::write_gzip(&root, ""))
    }

    pub fn read_clocks(&self) -> Result<BTreeMap<String, ClockState>, String> {
        let Some(data) = self.read_saved(WORLD_CLOCKS)? else {
            return Ok(BTreeMap::new());
        };
        let Some(entries) = data.as_compound() else {
            return Err("world_clocks is not a compound".to_owned());
        };
        Ok(entries
            .iter()
            .filter_map(|(id, state)| {
                Some((
                    id.clone(),
                    ClockState {
                        total_ticks: state.get("total_ticks").and_then(Tag::as_i64)?,
                        partial_tick: state
                            .get("partial_tick")
                            .and_then(Tag::as_f64)
                            .unwrap_or(0.0) as f32,
                        rate: state.get("rate").and_then(Tag::as_f64).unwrap_or(1.0) as f32,
                        paused: state
                            .get("paused")
                            .and_then(Tag::as_i64)
                            .is_some_and(|v| v != 0),
                    },
                ))
            })
            .collect())
    }

    pub fn write_clocks(&self, clocks: &BTreeMap<String, ClockState>) -> Result<(), String> {
        let data = clocks
            .iter()
            .map(|(id, state)| {
                (
                    id.clone(),
                    compound(vec![
                        ("total_ticks", Tag::Long(state.total_ticks)),
                        ("partial_tick", Tag::Float(state.partial_tick)),
                        ("rate", Tag::Float(state.rate)),
                        ("paused", Tag::Byte(i8::from(state.paused))),
                    ]),
                )
            })
            .collect();
        self.write_saved(WORLD_CLOCKS, Tag::Compound(data))
    }

    pub fn read_weather(&self) -> Result<Option<WeatherData>, String> {
        let Some(data) = self.read_saved(WEATHER)? else {
            return Ok(None);
        };
        let number = |key: &str| {
            data.get(key)
                .and_then(Tag::as_i64)
                .ok_or_else(|| format!("weather lacks {key}"))
        };
        Ok(Some(WeatherData {
            clear_weather_time: number("clear_weather_time")? as i32,
            rain_time: number("rain_time")? as i32,
            thunder_time: number("thunder_time")? as i32,
            raining: number("raining")? != 0,
            thundering: number("thundering")? != 0,
        }))
    }

    pub fn write_weather(&self, weather: &WeatherData) -> Result<(), String> {
        let data = compound(vec![
            ("clear_weather_time", Tag::Int(weather.clear_weather_time)),
            ("rain_time", Tag::Int(weather.rain_time)),
            ("thunder_time", Tag::Int(weather.thunder_time)),
            ("raining", Tag::Byte(i8::from(weather.raining))),
            ("thundering", Tag::Byte(i8::from(weather.thundering))),
        ]);
        self.write_saved(WEATHER, data)
    }

    pub fn read_game_rules(&self) -> Result<GameRules, String> {
        let Some(data) = self.read_saved(GAME_RULES)? else {
            return Ok(GameRules::default());
        };
        if data.as_compound().is_none() {
            return Err("game_rules is not a compound".to_owned());
        }
        Ok(GameRules::from_tag(&data))
    }

    pub fn write_game_rules(&self, rules: &GameRules) -> Result<(), String> {
        self.write_saved(GAME_RULES, rules.to_tag())
    }

    pub fn player_data_path(&self, uuid: &str) -> PathBuf {
        self.root
            .join(PLAYERS_FOLDER)
            .join(DATA_FOLDER)
            .join(format!("{uuid}.dat"))
    }

    pub fn read_player_data(&self, uuid: &str) -> Result<Option<Tag>, String> {
        read_nbt_or_old(&self.player_data_path(uuid))
    }

    pub fn write_player_data(&self, uuid: &str, data: &Tag) -> Result<(), String> {
        let mut entries = data
            .as_compound()
            .cloned()
            .ok_or("player data is not a compound")?;
        entries.insert("DataVersion".to_owned(), Tag::Int(DATA_VERSION));
        safe_replace(
            &self.player_data_path(uuid),
            &nbt::write_gzip(&Tag::Compound(entries), ""),
        )
    }
}

pub fn uuid_to_ints(uuid: &str) -> Option<[i32; 4]> {
    let hex: String = uuid.chars().filter(|&c| c != '-').collect();
    if hex.len() != 32 {
        return None;
    }
    let value = u128::from_str_radix(&hex, 16).ok()?;
    Some([96, 64, 32, 0].map(|shift| (value >> shift) as u32 as i32))
}

pub fn uuid_from_ints(ints: &[i32]) -> Option<String> {
    let [a, b, c, d] = ints else {
        return None;
    };
    let hex = format!(
        "{:08x}{:08x}{:08x}{:08x}",
        *a as u32, *b as u32, *c as u32, *d as u32
    );
    Some(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

pub fn dimension_path(world: &Path, dimension: &str) -> PathBuf {
    let (namespace, path) = split_id(dimension);
    world.join(DIMENSIONS_FOLDER).join(namespace).join(path)
}

pub fn old_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map_or_else(std::ffi::OsString::new, std::ffi::OsStr::to_os_string);
    name.push("_old");
    path.with_file_name(name)
}

pub fn safe_replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let temp = dir.join(format!(
        "{name}-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let written = File::create(&temp)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()))
        .map_err(|e| format!("{}: {e}", temp.display()));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    if path.exists() {
        let old = old_path(path);
        let _ = std::fs::remove_file(&old);
        std::fs::rename(path, &old).map_err(|e| format!("{}: {e}", old.display()))?;
    }
    std::fs::rename(&temp, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn split_id(id: &str) -> (&str, &str) {
    id.split_once(':').unwrap_or(("minecraft", id))
}

fn read_nbt(path: &Path) -> Result<Option<Tag>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    nbt::read(&bytes)
        .map(Some)
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn read_nbt_or_old(path: &Path) -> Result<Option<Tag>, String> {
    match read_nbt(path) {
        Ok(Some(tag)) => Ok(Some(tag)),
        current => match read_nbt(&old_path(path)) {
            Ok(Some(tag)) => Ok(Some(tag)),
            _ => current,
        },
    }
}

fn read_saved_at(path: &Path) -> Result<Option<Tag>, String> {
    let Some(root) = read_nbt_or_old(path)? else {
        return Ok(None);
    };
    Ok(root.get("data").cloned())
}

fn write_saved_at(path: &Path, data: Tag) -> Result<(), String> {
    let root = compound(vec![
        ("data", data),
        ("DataVersion", Tag::Int(DATA_VERSION)),
    ]);
    safe_replace(path, &nbt::write_gzip(&root, ""))
}

fn compound(entries: Vec<(&str, Tag)>) -> Tag {
    Tag::Compound(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

fn string(s: &str) -> Tag {
    Tag::String(s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir(PathBuf);

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_dir(name: &str) -> TestDir {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "minecraftoss-level-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        TestDir(dir)
    }

    fn level() -> LevelData {
        LevelData {
            level_name: "world".to_owned(),
            game_time: 123_456,
            spawn: RespawnData {
                dimension: "minecraft:overworld".to_owned(),
                pos: (-17, 71, 302),
                yaw: 0.0,
                pitch: 0.0,
            },
            difficulty: "normal".to_owned(),
            hardcore: false,
            initialized: true,
            allow_commands: true,
            game_type: 0,
            singleplayer_uuid: None,
        }
    }

    #[test]
    fn dimensions_live_under_dimensions_namespace_path() {
        let world = Path::new("saves").join("world");
        assert_eq!(
            dimension_path(&world, "minecraft:the_nether"),
            world
                .join("dimensions")
                .join("minecraft")
                .join("the_nether")
        );
        assert_eq!(
            dimension_path(&world, "minecraft:overworld"),
            world.join("dimensions").join("minecraft").join("overworld")
        );
        assert_eq!(
            dimension_path(&world, "iw4l:arena"),
            world.join("dimensions").join("iw4l").join("arena")
        );
    }

    #[test]
    fn opening_creates_the_lock_and_dimension_folders() {
        let dir = test_dir("open");
        let storage = LevelStorage::open(&dir.0).unwrap();
        let base = storage.create_dimension("minecraft:the_end").unwrap();
        for folder in ["region", "entities", "poi", "data"] {
            assert!(base.join(folder).is_dir(), "{folder}");
        }
        assert_eq!(
            base,
            dir.0.join("dimensions").join("minecraft").join("the_end")
        );
        drop(storage);
        assert_eq!(
            std::fs::read(dir.0.join("session.lock")).unwrap(),
            "\u{2603}".as_bytes()
        );
    }

    #[test]
    fn a_locked_world_cannot_be_opened_twice() {
        let dir = test_dir("lock");
        let first = LevelStorage::open(&dir.0).unwrap();
        assert!(LevelStorage::open(&dir.0).is_err());
        drop(first);
        assert!(LevelStorage::open(&dir.0).is_ok());
    }

    #[test]
    fn level_data_round_trips_in_the_vanilla_layout() {
        let dir = test_dir("level");
        let storage = LevelStorage::open(&dir.0).unwrap();
        assert_eq!(storage.read_level_data().unwrap(), None);
        let mut level = level();
        storage.write_level_data(&level).unwrap();
        assert_eq!(storage.read_level_data().unwrap(), Some(level.clone()));
        let root = nbt::read(&std::fs::read(dir.0.join("level.dat")).unwrap()).unwrap();
        let data = root.get("Data").unwrap();
        assert_eq!(data.get("version"), Some(&Tag::Int(19133)));
        assert_eq!(data.get("DataVersion"), Some(&Tag::Int(DATA_VERSION)));
        assert_eq!(DATA_VERSION, 5023);
        assert_eq!(
            data.get("spawn").and_then(|s| s.get("pos")),
            Some(&Tag::IntArray(vec![-17, 71, 302]))
        );
        assert_eq!(
            data.get("Version").and_then(|v| v.get("Name")),
            Some(&Tag::String("26.3".to_owned()))
        );
        assert!(!dir.0.join("level.dat_old").exists());
        level.game_time += 6000;
        storage.write_level_data(&level).unwrap();
        assert_eq!(storage.read_level_data().unwrap(), Some(level));
        let old = nbt::read(&std::fs::read(dir.0.join("level.dat_old")).unwrap()).unwrap();
        assert_eq!(
            old.get("Data").and_then(|d| d.get("Time")),
            Some(&Tag::Long(123_456))
        );
        let history = root.get("Data").and_then(|d| d.get("version_history"));
        assert_eq!(history, Some(&Tag::List(vec![Tag::Int(DATA_VERSION)])));
    }

    #[test]
    fn rewriting_level_data_keeps_fields_it_does_not_know() {
        let dir = test_dir("keep");
        let storage = LevelStorage::open(&dir.0).unwrap();
        let foreign = compound(vec![(
            "Data",
            compound(vec![
                ("WanderingTraderSpawnDelay", Tag::Int(24000)),
                ("singleplayer_uuid", Tag::IntArray(vec![1, 2, 3, 4])),
                ("ServerBrands", Tag::List(vec![string("vanilla")])),
                ("version_history", Tag::List(vec![Tag::Int(4000)])),
            ]),
        )]);
        safe_replace(&storage.level_data_path(), &nbt::write_gzip(&foreign, "")).unwrap();
        storage.write_level_data(&level()).unwrap();
        let root = nbt::read(&std::fs::read(storage.level_data_path()).unwrap()).unwrap();
        let data = root.get("Data").unwrap();
        assert_eq!(
            data.get("WanderingTraderSpawnDelay"),
            Some(&Tag::Int(24000))
        );
        assert_eq!(
            data.get("singleplayer_uuid"),
            Some(&Tag::IntArray(vec![1, 2, 3, 4]))
        );
        assert_eq!(
            data.get("ServerBrands"),
            Some(&Tag::List(vec![string("vanilla"), string("iw4l")]))
        );
        assert_eq!(
            data.get("version_history"),
            Some(&Tag::List(vec![Tag::Int(4000), Tag::Int(DATA_VERSION)]))
        );
    }

    #[test]
    fn allow_commands_and_game_type_round_trip() {
        let dir = test_dir("commands");
        let storage = LevelStorage::open(&dir.0).unwrap();
        assert!(LevelData::new("world").allow_commands);
        assert_eq!(LevelData::new("world").game_type, 0);
        let old = compound(vec![(
            "Data",
            compound(vec![
                ("allowCommands", Tag::Byte(0)),
                ("GameType", Tag::Int(1)),
            ]),
        )]);
        safe_replace(&storage.level_data_path(), &nbt::write_gzip(&old, "")).unwrap();
        let read = storage.read_level_data().unwrap().unwrap();
        assert!(!read.allow_commands);
        assert_eq!(read.game_type, 1);
        storage
            .write_level_data(&LevelData {
                allow_commands: true,
                game_type: 2,
                ..read
            })
            .unwrap();
        let root = nbt::read(&std::fs::read(storage.level_data_path()).unwrap()).unwrap();
        let data = root.get("Data").unwrap();
        assert_eq!(data.get("allowCommands"), Some(&Tag::Byte(1)));
        assert_eq!(data.get("GameType"), Some(&Tag::Int(2)));
        let again = storage.read_level_data().unwrap().unwrap();
        assert!(again.allow_commands);
        assert_eq!(again.game_type, 2);
    }

    #[test]
    fn player_files_live_in_players_data_uuid_dat() {
        let dir = test_dir("players");
        let storage = LevelStorage::open(&dir.0).unwrap();
        let uuid = "b50ad385-829d-3141-a216-7e7d7539ba7f";
        let path = dir
            .0
            .join("players")
            .join("data")
            .join(format!("{uuid}.dat"));
        assert_eq!(storage.player_data_path(uuid), path);
        assert_eq!(storage.read_player_data(uuid).unwrap(), None);
        let first = compound(vec![("playerGameType", Tag::Int(1))]);
        storage.write_player_data(uuid, &first).unwrap();
        let read = storage.read_player_data(uuid).unwrap().unwrap();
        assert_eq!(read.get("playerGameType"), Some(&Tag::Int(1)));
        assert_eq!(read.get("DataVersion"), Some(&Tag::Int(DATA_VERSION)));
        let second = compound(vec![("playerGameType", Tag::Int(3))]);
        storage.write_player_data(uuid, &second).unwrap();
        std::fs::write(&path, b"broken").unwrap();
        let fallback = storage.read_player_data(uuid).unwrap().unwrap();
        assert_eq!(fallback.get("playerGameType"), Some(&Tag::Int(1)));
        assert!(storage.write_player_data(uuid, &Tag::Int(2)).is_err());
    }

    #[test]
    fn uuids_are_stored_as_four_ints_like_uuid_util() {
        let uuid = "b50ad385-829d-3141-a216-7e7d7539ba7f";
        let ints = uuid_to_ints(uuid).unwrap();
        assert_eq!(
            ints,
            [
                0xb50a_d385_u32 as i32,
                0x829d_3141_u32 as i32,
                0xa216_7e7d_u32 as i32,
                0x7539_ba7f
            ]
        );
        assert_eq!(uuid_from_ints(&ints).as_deref(), Some(uuid));
        assert_eq!(uuid_to_ints("not-a-uuid"), None);
        assert_eq!(uuid_from_ints(&[1, 2, 3]), None);
        let dir = test_dir("host");
        let storage = LevelStorage::open(&dir.0).unwrap();
        let level = LevelData {
            singleplayer_uuid: Some(uuid.to_owned()),
            ..level()
        };
        storage.write_level_data(&level).unwrap();
        let root = nbt::read(&std::fs::read(storage.level_data_path()).unwrap()).unwrap();
        assert_eq!(
            root.get("Data").and_then(|d| d.get("singleplayer_uuid")),
            Some(&Tag::IntArray(ints.to_vec()))
        );
        assert_eq!(storage.read_level_data().unwrap(), Some(level));
    }

    #[test]
    fn saved_data_lives_in_data_namespace_id_dat() {
        let dir = test_dir("saved");
        let storage = LevelStorage::open(&dir.0).unwrap();
        for id in ["weather", "world_clocks", "world_gen_settings"] {
            assert_eq!(
                storage.saved_data_path(id),
                dir.0
                    .join("data")
                    .join("minecraft")
                    .join(format!("{id}.dat"))
            );
        }
        assert_eq!(storage.read_saved("weather").unwrap(), None);
        let weather = compound(vec![("raining", Tag::Byte(1))]);
        storage.write_saved("weather", weather.clone()).unwrap();
        assert_eq!(
            storage.read_saved("minecraft:weather").unwrap(),
            Some(weather)
        );
        let root = nbt::read(&std::fs::read(storage.saved_data_path("weather")).unwrap()).unwrap();
        assert_eq!(root.get("DataVersion"), Some(&Tag::Int(DATA_VERSION)));
        let settings = WorldSettings::new(-9_876_543_210);
        storage.write_world_settings(&settings).unwrap();
        assert!(storage.saved_data_path("world_gen_settings").is_file());
        assert_eq!(storage.read_world_settings().unwrap(), Some(settings));
        let reseeded = WorldSettings::new(5);
        storage.write_world_settings(&reseeded).unwrap();
        assert!(old_path(&storage.saved_data_path("world_gen_settings")).is_file());
    }

    #[test]
    fn clocks_round_trip_with_their_partial_tick() {
        let dir = test_dir("clocks");
        let storage = LevelStorage::open(&dir.0).unwrap();
        assert!(storage.read_clocks().unwrap().is_empty());
        let mut clocks = BTreeMap::new();
        clocks.insert("minecraft:overworld".to_owned(), ClockState::at(30_250.5));
        clocks.insert("minecraft:the_end".to_owned(), ClockState::at(12.0));
        storage.write_clocks(&clocks).unwrap();
        let back = storage.read_clocks().unwrap();
        assert_eq!(back, clocks);
        assert_eq!(back["minecraft:overworld"].total_ticks, 30_250);
        assert_eq!(back["minecraft:overworld"].ticks(), 30_250.5);
        let data = storage.read_saved("world_clocks").unwrap().unwrap();
        let overworld = data.get("minecraft:overworld").unwrap();
        assert_eq!(overworld.get("total_ticks"), Some(&Tag::Long(30_250)));
        assert_eq!(overworld.get("rate"), Some(&Tag::Float(1.0)));
    }

    #[test]
    fn weather_lives_in_data_minecraft_weather_dat_with_its_five_keys() {
        let dir = test_dir("weather");
        let storage = LevelStorage::open(&dir.0).unwrap();
        assert_eq!(storage.read_weather().unwrap(), None);
        let stormy = WeatherData {
            clear_weather_time: 0,
            rain_time: 4_321,
            thunder_time: 987,
            raining: true,
            thundering: true,
        };
        storage.write_weather(&stormy).unwrap();
        assert_eq!(storage.read_weather().unwrap(), Some(stormy));
        let path = dir.0.join("data").join("minecraft").join("weather.dat");
        let root = nbt::read(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(root.get("DataVersion"), Some(&Tag::Int(DATA_VERSION)));
        assert_eq!(
            root.get("data"),
            Some(&compound(vec![
                ("clear_weather_time", Tag::Int(0)),
                ("rain_time", Tag::Int(4_321)),
                ("thunder_time", Tag::Int(987)),
                ("raining", Tag::Byte(1)),
                ("thundering", Tag::Byte(1)),
            ]))
        );
        let clear = WeatherData {
            clear_weather_time: 12_000,
            ..WeatherData::default()
        };
        storage.write_weather(&clear).unwrap();
        assert_eq!(storage.read_weather().unwrap(), Some(clear));
        std::fs::write(&path, b"broken").unwrap();
        assert_eq!(storage.read_weather().unwrap(), Some(stormy));
        storage
            .write_saved(WEATHER, compound(vec![("raining", Tag::Byte(1))]))
            .unwrap();
        assert!(storage.read_weather().is_err());
    }

    #[test]
    fn a_dimension_keeps_its_own_saved_data_under_its_data_folder() {
        let dir = test_dir("dimension-data");
        let storage = LevelStorage::open(&dir.0).unwrap();
        let fight = compound(vec![("DragonKilled", Tag::Byte(1))]);
        assert_eq!(
            storage
                .read_saved_in("minecraft:the_end", "ender_dragon_fight")
                .unwrap(),
            None
        );
        storage
            .write_saved_in("minecraft:the_end", "ender_dragon_fight", fight.clone())
            .unwrap();
        let path = dir
            .0
            .join("dimensions")
            .join("minecraft")
            .join("the_end")
            .join("data")
            .join("minecraft")
            .join("ender_dragon_fight.dat");
        assert_eq!(
            storage.dimension_data_path("minecraft:the_end", "ender_dragon_fight"),
            path
        );
        let root = nbt::read(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(root.get("DataVersion"), Some(&Tag::Int(DATA_VERSION)));
        assert_eq!(root.get("data"), Some(&fight));
        assert_eq!(
            storage
                .read_saved_in("minecraft:the_end", "ender_dragon_fight")
                .unwrap(),
            Some(fight.clone())
        );
        assert_eq!(
            storage
                .read_saved_in("minecraft:the_nether", "ender_dragon_fight")
                .unwrap(),
            None
        );
        assert_eq!(storage.read_saved("ender_dragon_fight").unwrap(), None);
        std::fs::write(&path, b"broken").unwrap();
        assert!(storage
            .read_saved_in("minecraft:the_end", "ender_dragon_fight")
            .is_err());
        storage
            .write_saved_in("minecraft:the_end", "ender_dragon_fight", fight.clone())
            .unwrap();
        storage
            .write_saved_in("minecraft:the_end", "ender_dragon_fight", Tag::Int(0))
            .unwrap();
        std::fs::write(&path, b"broken").unwrap();
        assert_eq!(
            storage
                .read_saved_in("minecraft:the_end", "ender_dragon_fight")
                .unwrap(),
            Some(fight)
        );
    }

    #[test]
    fn game_rules_live_in_data_minecraft_game_rules_dat() {
        let dir = test_dir("rules");
        let storage = LevelStorage::open(&dir.0).unwrap();
        let mut rules = GameRules::default();
        rules.set("keep_inventory", "true").unwrap();
        rules.set("random_tick_speed", "7").unwrap();
        storage.write_game_rules(&rules).unwrap();
        let path = dir.0.join("data").join("minecraft").join("game_rules.dat");
        let root = nbt::read(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(root.get("DataVersion"), Some(&Tag::Int(DATA_VERSION)));
        let data = root.get("data").unwrap();
        assert_eq!(data.get("minecraft:keep_inventory"), Some(&Tag::Byte(1)));
        assert_eq!(data.get("minecraft:random_tick_speed"), Some(&Tag::Int(7)));
        assert_eq!(storage.read_game_rules().unwrap(), rules);
    }

    #[test]
    fn missing_game_rules_read_as_defaults() {
        let dir = test_dir("norules");
        let storage = LevelStorage::open(&dir.0).unwrap();
        assert_eq!(storage.read_game_rules().unwrap(), GameRules::default());
        storage.write_saved(GAME_RULES, Tag::Int(3)).unwrap();
        assert!(storage.read_game_rules().is_err());
    }

    #[test]
    fn a_missing_or_broken_file_is_read_from_its_old_copy() {
        let dir = test_dir("fallback");
        let storage = LevelStorage::open(&dir.0).unwrap();
        let mut level = level();
        storage.write_level_data(&level).unwrap();
        level.game_time += 20;
        storage.write_level_data(&level).unwrap();
        let path = storage.level_data_path();
        std::fs::write(&path, b"not nbt").unwrap();
        assert_eq!(
            storage.read_level_data().unwrap().unwrap().game_time,
            123_456
        );
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            storage.read_level_data().unwrap().unwrap().game_time,
            123_456
        );
        let mut clocks = BTreeMap::new();
        clocks.insert("minecraft:overworld".to_owned(), ClockState::at(500.0));
        storage.write_clocks(&clocks).unwrap();
        clocks.insert("minecraft:overworld".to_owned(), ClockState::at(900.0));
        storage.write_clocks(&clocks).unwrap();
        std::fs::write(storage.saved_data_path(WORLD_CLOCKS), b"").unwrap();
        assert_eq!(
            storage.read_clocks().unwrap()["minecraft:overworld"].total_ticks,
            500
        );
        std::fs::write(old_path(&storage.saved_data_path(WORLD_CLOCKS)), b"").unwrap();
        assert!(storage.read_clocks().is_err());
    }

    #[test]
    fn safe_replace_keeps_the_previous_copy() {
        let dir = test_dir("replace");
        let path = dir.0.join("nested").join("file.dat");
        safe_replace(&path, b"first").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        assert!(!old_path(&path).exists());
        safe_replace(&path, b"second").unwrap();
        safe_replace(&path, b"third").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"third");
        assert_eq!(std::fs::read(old_path(&path)).unwrap(), b"second");
        assert_eq!(old_path(&path).file_name().unwrap(), "file.dat_old");
        let leftovers = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .is_ok_and(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            })
            .count();
        assert_eq!(leftovers, 0);
    }
}
