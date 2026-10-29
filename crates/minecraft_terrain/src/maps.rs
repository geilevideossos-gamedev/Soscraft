use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use minecraftoss_core::loot::Exploring;
use minecraftoss_core::nbt::Tag;
use minecraftoss_world::chunk_map::WorldGen;
use minecraftoss_world::level_storage::LevelStorage;

use crate::locate::{self, Search};

pub const MAP_SIZE: usize = 128;
pub const COLORS: usize = MAP_SIZE * MAP_SIZE;
pub const MAX_SCALE: u8 = 4;
pub const LAST_ID: &str = "minecraft:maps/last_id";
pub const PLAYER: &str = "minecraft:player";
pub const PLAYER_OFF_MAP: &str = "minecraft:player_off_map";
pub const PLAYER_OFF_LIMITS: &str = "minecraft:player_off_limits";
pub const SEARCH_TIME: Duration = Duration::from_secs(2);
const INSIDE: f32 = 63.0;
const OFF_MAP: f32 = 320.0;

pub fn map_key(id: i32) -> String {
    format!("minecraft:maps/{id}")
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapData {
    pub dimension: String,
    pub x_center: i32,
    pub z_center: i32,
    pub scale: u8,
    pub colors: Box<[u8; COLORS]>,
    pub tracking_position: bool,
    pub unlimited_tracking: bool,
    pub locked: bool,
    pub banners: Vec<Tag>,
    pub frames: Vec<Tag>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoration {
    pub kind: String,
    pub x: i8,
    pub y: i8,
    pub rot: i8,
}

impl MapData {
    pub fn create_fresh(
        x: f64,
        z: f64,
        scale: u8,
        dimension: &str,
        tracking: bool,
        unlimited: bool,
    ) -> Self {
        let size = 128 << scale;
        let center =
            |at: f64| ((at + 64.0) / f64::from(size)).floor() as i32 * size + size / 2 - 64;
        Self {
            dimension: dimension.to_owned(),
            x_center: center(x),
            z_center: center(z),
            scale,
            colors: Box::new([0; COLORS]),
            tracking_position: tracking,
            unlimited_tracking: unlimited,
            locked: false,
            banners: Vec::new(),
            frames: Vec::new(),
        }
    }

    pub fn map_units(&self, x: f64, z: f64) -> (f32, f32) {
        let step = (1i32 << self.scale) as f32;
        (
            (x - f64::from(self.x_center)) as f32 / step,
            (z - f64::from(self.z_center)) as f32 / step,
        )
    }

    pub fn decoration(
        &self,
        kind: &str,
        x: f64,
        z: f64,
        rot: f64,
        game_time: Option<i64>,
    ) -> Option<Decoration> {
        let (fx, fz) = self.map_units(x, z);
        let kind = if kind == PLAYER {
            player_marker(fx, fz, self.unlimited_tracking)?
        } else if inside(fx, fz) || self.unlimited_tracking {
            kind
        } else {
            return None;
        };
        let rot = match game_time {
            Some(time) if self.dimension == "minecraft:the_nether" => nether_rotation(time),
            _ => rotation_byte(rot),
        };
        Some(Decoration {
            kind: kind.to_owned(),
            x: clamp_coordinate(fx),
            y: clamp_coordinate(fz),
            rot,
        })
    }

    pub fn to_tag(&self) -> Tag {
        let mut map = BTreeMap::new();
        map.insert("dimension".to_owned(), Tag::String(self.dimension.clone()));
        map.insert("xCenter".to_owned(), Tag::Int(self.x_center));
        map.insert("zCenter".to_owned(), Tag::Int(self.z_center));
        if self.scale != 0 {
            map.insert("scale".to_owned(), Tag::Byte(self.scale as i8));
        }
        map.insert(
            "colors".to_owned(),
            Tag::ByteArray(self.colors.iter().map(|&c| c as i8).collect()),
        );
        for (key, value, default) in [
            ("trackingPosition", self.tracking_position, true),
            ("unlimitedTracking", self.unlimited_tracking, false),
            ("locked", self.locked, false),
        ] {
            if value != default {
                map.insert(key.to_owned(), Tag::Byte(i8::from(value)));
            }
        }
        for (key, list) in [("banners", &self.banners), ("frames", &self.frames)] {
            if !list.is_empty() {
                map.insert(key.to_owned(), Tag::List(list.clone()));
            }
        }
        Tag::Compound(map)
    }

    pub fn from_tag(tag: &Tag) -> Result<Self, String> {
        let dimension = tag
            .get("dimension")
            .and_then(Tag::as_str)
            .ok_or("map data lacks its dimension")?;
        let int = |key: &str| {
            tag.get(key)
                .and_then(Tag::as_i64)
                .map(|v| v as i32)
                .ok_or_else(|| format!("map data lacks {key}"))
        };
        let flag = |key: &str, default: bool| {
            tag.get(key)
                .and_then(Tag::as_i64)
                .map_or(default, |v| v != 0)
        };
        let list = |key: &str| {
            tag.get(key)
                .and_then(Tag::as_list)
                .unwrap_or_default()
                .to_vec()
        };
        let Some(Tag::ByteArray(bytes)) = tag.get("colors") else {
            return Err("map data lacks its colors".to_owned());
        };
        let mut colors = Box::new([0; COLORS]);
        if bytes.len() == COLORS {
            for (cell, &byte) in colors.iter_mut().zip(bytes) {
                *cell = byte as u8;
            }
        }
        let scale = tag.get("scale").and_then(Tag::as_i64).unwrap_or(0);
        Ok(Self {
            dimension: dimension.to_owned(),
            x_center: int("xCenter")?,
            z_center: int("zCenter")?,
            scale: scale.clamp(0, i64::from(MAX_SCALE)) as u8,
            colors,
            tracking_position: flag("trackingPosition", true),
            unlimited_tracking: flag("unlimitedTracking", false),
            locked: flag("locked", false),
            banners: list("banners"),
            frames: list("frames"),
        })
    }
}

pub fn inside(fx: f32, fz: f32) -> bool {
    fx >= -INSIDE && fz >= -INSIDE && fx <= INSIDE && fz <= INSIDE
}

pub fn clamp_coordinate(f: f32) -> i8 {
    if f <= -INSIDE {
        -128
    } else if f >= INSIDE {
        127
    } else {
        (f64::from(f * 2.0) + 0.5) as i32 as i8
    }
}

pub fn rotation_byte(rot: f64) -> i8 {
    let turned = if rot < 0.0 { rot - 8.0 } else { rot + 8.0 };
    (turned * 16.0 / 360.0) as i32 as i8
}

pub fn nether_rotation(game_time: i64) -> i8 {
    let t = (game_time / 10) as i32;
    let spin = t.wrapping_mul(t).wrapping_mul(34_187_121);
    ((spin.wrapping_add(t.wrapping_mul(121)) >> 15) & 15) as i8
}

pub fn player_marker(fx: f32, fz: f32, unlimited: bool) -> Option<&'static str> {
    if inside(fx, fz) {
        Some(PLAYER)
    } else if fx.abs() < OFF_MAP && fz.abs() < OFF_MAP {
        Some(PLAYER_OFF_MAP)
    } else if unlimited {
        Some(PLAYER_OFF_LIMITS)
    } else {
        None
    }
}

#[derive(Debug)]
pub struct MapStore {
    maps: BTreeMap<i32, MapData>,
    last_id: i32,
    dirty: BTreeSet<i32>,
    index_dirty: bool,
}

impl Default for MapStore {
    fn default() -> Self {
        Self {
            maps: BTreeMap::new(),
            last_id: -1,
            dirty: BTreeSet::new(),
            index_dirty: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct MapWrites {
    pub maps: Vec<(i32, Tag)>,
    pub last_id: Option<i32>,
}

impl MapWrites {
    pub fn is_empty(&self) -> bool {
        self.maps.is_empty() && self.last_id.is_none()
    }

    pub fn write(&self, storage: &LevelStorage) -> Result<(), String> {
        let mut failed = Vec::new();
        for (id, tag) in &self.maps {
            if let Err(error) = storage.write_saved(&map_key(*id), tag.clone()) {
                failed.push(error);
            }
        }
        if let Some(last_id) = self.last_id {
            let index = Tag::Compound(BTreeMap::from([("map".to_owned(), Tag::Int(last_id))]));
            if let Err(error) = storage.write_saved(LAST_ID, index) {
                failed.push(error);
            }
        }
        if failed.is_empty() {
            Ok(())
        } else {
            Err(failed.join("; "))
        }
    }
}

impl MapStore {
    pub fn load(storage: &LevelStorage) -> (Self, Vec<String>) {
        let mut store = Self::default();
        let mut warnings = Vec::new();
        match storage.read_saved(LAST_ID) {
            Ok(index) => {
                store.last_id = index
                    .as_ref()
                    .and_then(|index| index.get("map"))
                    .and_then(Tag::as_i64)
                    .map_or(-1, |id| id as i32);
            }
            Err(error) => warnings.push(format!("Minecraft maps/last_id.dat unreadable: {error}")),
        }
        let folder = storage.saved_data_path(LAST_ID);
        let entries = folder
            .parent()
            .and_then(|dir| std::fs::read_dir(dir).ok())
            .into_iter()
            .flatten()
            .flatten();
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(id) = name
                .strip_suffix(".dat")
                .and_then(|stem| stem.parse::<i32>().ok())
                .filter(|id| *id >= 0)
            else {
                continue;
            };
            match storage.read_saved(&map_key(id)) {
                Ok(Some(tag)) => match MapData::from_tag(&tag) {
                    Ok(data) => {
                        store.last_id = store.last_id.max(id);
                        store.maps.insert(id, data);
                    }
                    Err(error) => warnings.push(format!("Minecraft map {id} unreadable: {error}")),
                },
                Ok(None) => {}
                Err(error) => warnings.push(format!("Minecraft map {id} unreadable: {error}")),
            }
        }
        (store, warnings)
    }

    pub fn last_id(&self) -> i32 {
        self.last_id
    }

    pub fn next_id(&self) -> i32 {
        self.last_id + 1
    }

    pub fn free_id(&mut self) -> i32 {
        self.last_id += 1;
        self.index_dirty = true;
        self.last_id
    }

    pub fn get(&self, id: i32) -> Option<&MapData> {
        self.maps.get(&id)
    }

    pub fn get_mut(&mut self, id: i32) -> Option<&mut MapData> {
        let data = self.maps.get_mut(&id)?;
        self.dirty.insert(id);
        Some(data)
    }

    pub fn len(&self) -> usize {
        self.maps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.maps.is_empty()
    }

    pub fn insert(&mut self, id: i32, data: MapData) {
        if id > self.last_id {
            self.last_id = id;
            self.index_dirty = true;
        }
        self.maps.insert(id, data);
        self.dirty.insert(id);
    }

    pub fn take_writes(&mut self) -> MapWrites {
        let maps = std::mem::take(&mut self.dirty)
            .into_iter()
            .filter_map(|id| self.maps.get(&id).map(|data| (id, data.to_tag())))
            .collect();
        let last_id = std::mem::take(&mut self.index_dirty).then_some(self.last_id);
        MapWrites { maps, last_id }
    }
}

#[derive(Debug, Default)]
pub struct Explored {
    pub maps: Vec<(i32, MapData)>,
    pub timed_out: Vec<String>,
}

pub struct LootExplorer {
    gen: Arc<WorldGen>,
    dimension: String,
    first_id: i32,
    explored: Mutex<Explored>,
}

impl std::fmt::Debug for LootExplorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LootExplorer")
            .field("dimension", &self.dimension)
            .field("first_id", &self.first_id)
            .finish_non_exhaustive()
    }
}

impl LootExplorer {
    pub fn new(gen: Arc<WorldGen>, dimension: &str, first_id: i32) -> Self {
        Self {
            gen,
            dimension: dimension.to_owned(),
            first_id,
            explored: Mutex::new(Explored::default()),
        }
    }

    pub fn take(&self) -> Explored {
        std::mem::take(&mut *self.explored())
    }

    fn explored(&self) -> MutexGuard<'_, Explored> {
        self.explored.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Exploring for LootExplorer {
    fn explore(
        &self,
        destination: &[String],
        origin: [i32; 3],
        radius: i32,
        _skip_known: bool,
        zoom: u8,
    ) -> Option<(i32, [i32; 3])> {
        let deadline = Instant::now() + SEARCH_TIME;
        let target =
            match locate::nearest_structure(&self.gen, destination, origin, radius, deadline) {
                Search::Found(target, _) => target,
                Search::NotFound => return None,
                Search::TimedOut => {
                    self.explored().timed_out.push(destination.join(", "));
                    return None;
                }
            };
        let map = MapData::create_fresh(
            f64::from(target[0]),
            f64::from(target[2]),
            zoom.min(MAX_SCALE),
            &self.dimension,
            true,
            true,
        );
        let mut explored = self.explored();
        let id = self.first_id + explored.maps.len() as i32;
        explored.maps.push((id, map));
        Some((id, target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_map_snaps_its_centre_to_the_vanilla_grid() {
        let map = MapData::create_fresh(1000.0, -300.0, 1, "minecraft:overworld", true, true);
        assert_eq!((map.x_center, map.z_center), (1088, -192));
        let origin = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        assert_eq!((origin.x_center, origin.z_center), (0, 0));
        let edge = MapData::create_fresh(63.9, -64.1, 0, "minecraft:overworld", true, false);
        assert_eq!((edge.x_center, edge.z_center), (0, -128));
        let wide = MapData::create_fresh(-3000.0, 5000.0, 4, "minecraft:overworld", true, false);
        assert_eq!((wide.x_center, wide.z_center), (-3136, 5056));
        assert!(map.colors.iter().all(|&c| c == 0));
        assert!(!map.locked && map.tracking_position && map.unlimited_tracking);
    }

    #[test]
    fn a_target_is_placed_clamped_and_turned_like_vanilla() {
        let map = MapData::create_fresh(1000.0, -300.0, 1, "minecraft:overworld", true, true);
        let target = map.decoration("minecraft:red_x", 1000.0, -300.0, 180.0, None);
        assert_eq!(
            target,
            Some(Decoration {
                kind: "minecraft:red_x".to_owned(),
                x: -87,
                y: -107,
                rot: 8,
            })
        );
        assert_eq!(clamp_coordinate(-63.0), -128);
        assert_eq!(clamp_coordinate(63.0), 127);
        assert_eq!(clamp_coordinate(-62.9), -125);
        assert_eq!(clamp_coordinate(62.9), 126);
        assert_eq!(clamp_coordinate(0.0), 0);
        assert_eq!(clamp_coordinate(-0.4), 0);
        assert_eq!(rotation_byte(180.0), 8);
        assert_eq!(rotation_byte(0.0), 0);
        assert_eq!(rotation_byte(-90.0), -4);
        assert_eq!(rotation_byte(350.0), 15);
        assert!(inside(63.0, -63.0) && !inside(63.1, 0.0) && !inside(0.0, -63.1));
        let far = map
            .decoration("minecraft:red_x", 2000.0, -300.0, 180.0, None)
            .unwrap();
        assert_eq!(
            (far.x, far.y),
            (127, -107),
            "an unlimited map keeps far targets on its edge"
        );
        let small = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        assert_eq!(
            small.decoration("minecraft:red_x", 200.0, 0.0, 180.0, None),
            None
        );
    }

    #[test]
    fn the_holder_marker_leaves_the_map_like_vanilla() {
        assert_eq!(player_marker(10.0, -20.0, false), Some(PLAYER));
        assert_eq!(player_marker(300.0, 0.0, false), Some(PLAYER_OFF_MAP));
        assert_eq!(player_marker(0.0, -319.9, true), Some(PLAYER_OFF_MAP));
        assert_eq!(player_marker(400.0, 0.0, true), Some(PLAYER_OFF_LIMITS));
        assert_eq!(player_marker(400.0, 0.0, false), None);
        assert_eq!(player_marker(0.0, 320.0, false), None);
        let map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        let off = map.decoration(PLAYER, 300.0, 0.0, 90.0, Some(5)).unwrap();
        assert_eq!(
            (off.kind.as_str(), off.x, off.y, off.rot),
            (PLAYER_OFF_MAP, 127, 0, 4)
        );
        assert_eq!(map.decoration(PLAYER, 400.0, 0.0, 0.0, None), None);
        let nether = MapData::create_fresh(0.0, 0.0, 0, "minecraft:the_nether", true, false);
        let spun = nether
            .decoration(PLAYER, 0.0, 0.0, 90.0, Some(1234))
            .unwrap();
        assert_eq!(spun.rot, nether_rotation(1234));
        let t = 123i32;
        let spin = t
            .wrapping_mul(t)
            .wrapping_mul(34_187_121)
            .wrapping_add(t * 121);
        assert_eq!(nether_rotation(1234), ((spin >> 15) & 15) as i8);
        assert!((0..16).contains(&nether_rotation(987_654_321)));
    }

    #[test]
    fn map_data_keeps_the_vanilla_keys_and_skips_defaults() {
        let mut map = MapData::create_fresh(1000.0, -300.0, 1, "minecraft:overworld", true, true);
        map.colors[5] = 0x7a;
        map.colors[COLORS - 1] = 0xff;
        let tag = map.to_tag();
        let keys: Vec<&str> = tag
            .as_compound()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "colors",
                "dimension",
                "scale",
                "unlimitedTracking",
                "xCenter",
                "zCenter"
            ]
        );
        assert_eq!(tag.get("scale"), Some(&Tag::Byte(1)));
        assert_eq!(tag.get("unlimitedTracking"), Some(&Tag::Byte(1)));
        let Some(Tag::ByteArray(colors)) = tag.get("colors") else {
            panic!("colors are a byte array");
        };
        assert_eq!(
            (colors.len(), colors[5], colors[COLORS - 1]),
            (COLORS, 0x7a, -1)
        );
        assert_eq!(MapData::from_tag(&tag), Ok(map.clone()));
        let mut plain = MapData::create_fresh(0.0, 0.0, 0, "minecraft:the_nether", false, false);
        plain.locked = true;
        plain.banners.push(Tag::Compound(BTreeMap::from([(
            "color".to_owned(),
            Tag::String("red".to_owned()),
        )])));
        let tag = plain.to_tag();
        assert_eq!(tag.get("scale"), None);
        assert_eq!(tag.get("trackingPosition"), Some(&Tag::Byte(0)));
        assert_eq!(tag.get("locked"), Some(&Tag::Byte(1)));
        assert_eq!(tag.get("frames"), None);
        assert_eq!(MapData::from_tag(&tag), Ok(plain));
        let mut short = map.to_tag();
        if let Tag::Compound(entries) = &mut short {
            entries.insert("colors".to_owned(), Tag::ByteArray(vec![1; 10]));
        }
        assert!(
            MapData::from_tag(&short)
                .unwrap()
                .colors
                .iter()
                .all(|&c| c == 0)
        );
        let mut broken = map.to_tag();
        if let Tag::Compound(entries) = &mut broken {
            entries.remove("colors");
        }
        assert!(MapData::from_tag(&broken).is_err());
    }

    #[test]
    fn maps_are_saved_in_the_vanilla_files_and_come_back() {
        let dir = crate::test_dir::tempdir().unwrap();
        let world = dir.path().join("world");
        let storage = LevelStorage::open(&world).unwrap();
        let (mut store, warnings) = MapStore::load(&storage);
        assert!(warnings.is_empty() && store.is_empty());
        assert_eq!((store.last_id(), store.next_id()), (-1, 0));
        assert!(store.take_writes().is_empty());
        let first = store.free_id();
        assert_eq!(first, 0);
        let map = MapData::create_fresh(1000.0, -300.0, 1, "minecraft:overworld", true, true);
        store.insert(first, map.clone());
        store.take_writes().write(&storage).unwrap();
        assert!(world.join("data/minecraft/maps/0.dat").is_file());
        assert!(world.join("data/minecraft/maps/last_id.dat").is_file());
        let index = storage.read_saved(LAST_ID).unwrap().unwrap();
        assert_eq!(index.get("map"), Some(&Tag::Int(0)));
        assert!(
            store.take_writes().is_empty(),
            "nothing changed since the last write"
        );
        store.get_mut(first).unwrap().colors[0] = 4;
        let writes = store.take_writes();
        assert_eq!((writes.maps.len(), writes.last_id), (1, None));
        writes.write(&storage).unwrap();
        drop(storage);
        let storage = LevelStorage::open(&world).unwrap();
        let (again, warnings) = MapStore::load(&storage);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(again.last_id(), 0);
        assert_eq!(again.get(0).map(|data| data.colors[0]), Some(4));
        assert_eq!(
            again.get(0).map(|data| (data.x_center, data.z_center)),
            Some((1088, -192))
        );
    }

    #[test]
    fn a_map_file_beyond_the_index_is_never_overwritten() {
        let dir = crate::test_dir::tempdir().unwrap();
        let world = dir.path().join("world");
        let storage = LevelStorage::open(&world).unwrap();
        let map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        storage.write_saved(&map_key(3), map.to_tag()).unwrap();
        std::fs::write(world.join("data/minecraft/maps/7.dat"), b"broken").unwrap();
        std::fs::write(world.join("data/minecraft/maps/notes.dat"), b"other").unwrap();
        let (mut store, warnings) = MapStore::load(&storage);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("map 7"), "{warnings:?}");
        assert_eq!(store.len(), 1);
        assert_eq!(store.next_id(), 4);
        assert_eq!(store.free_id(), 4);
    }

    fn overworld_stream(seed: i64) -> Option<crate::terrain::TerrainStream> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = Arc::new(minecraftoss_core::Registries::load(&paths).ok()?);
        crate::terrain::TerrainStream::for_dimension_at(
            registries,
            seed,
            2,
            crate::terrain::Dimension::Overworld,
            None,
            Some((0, 64, 0)),
        )
        .ok()
    }

    fn chest(table: &str, seed: i64) -> Tag {
        Tag::Compound(BTreeMap::from([
            ("LootTable".to_owned(), Tag::String(table.to_owned())),
            ("LootTableSeed".to_owned(), Tag::Long(seed)),
        ]))
    }

    fn found(search: Search) -> [i32; 3] {
        match search {
            Search::Found(at, _) => at,
            other => panic!("nothing found: {other:?}"),
        }
    }

    #[test]
    fn a_shipwreck_map_chest_rolls_a_treasure_map_to_the_nearest_buried_treasure() {
        let Some(mut stream) = overworld_stream(77) else {
            return;
        };
        let gen = stream.world_gen();
        let registries = gen.terrain.registries.clone();
        let deadline = Instant::now() + Duration::from_secs(120);
        let shipwrecks = ["minecraft:shipwreck".to_owned()];
        let wreck = found(locate::nearest_structure(
            &gen,
            &shipwrecks,
            [0, 64, 0],
            100,
            deadline,
        ));
        let origin = (wreck[0], 50, wreck[2]);
        let explorer = Arc::new(LootExplorer::new(gen.clone(), "minecraft:overworld", 5));
        let recipes = minecraftoss_player::crafting::RecipeBook::default();
        let (items, _) = crate::containers::structure_loot_in(
            &registries,
            &chest("minecraft:chests/shipwreck_map", 9),
            origin,
            77,
            &recipes,
            None,
            None,
            Some(explorer.clone()),
        )
        .expect("the chest rolls");
        let map = items
            .iter()
            .find(|item| item.id == "minecraft:buried_treasure_map")
            .unwrap_or_else(|| panic!("a treasure map: {items:?}"));
        let components = map.components.as_ref().unwrap();
        assert_eq!(components["minecraft:map_id"], 5);
        let mark = &components["minecraft:map_decorations"]["+"];
        assert_eq!(mark["type"], "minecraft:red_x");
        assert_eq!(mark["rotation"], 180.0);
        let target = [
            mark["x"].as_f64().unwrap() as i32,
            mark["z"].as_f64().unwrap() as i32,
        ];
        let treasure = ["minecraft:buried_treasure".to_owned()];
        let nearest = found(locate::nearest_structure(
            &gen,
            &treasure,
            [origin.0, origin.1, origin.2],
            50,
            deadline,
        ));
        assert_eq!(target, [nearest[0], nearest[2]]);
        assert_eq!((target[0] & 15, target[1] & 15), (9, 9));
        let explored = explorer.take();
        assert!(explored.timed_out.is_empty());
        assert_eq!(explored.maps.len(), 1);
        let (id, data) = &explored.maps[0];
        let fresh = MapData::create_fresh(
            f64::from(target[0]),
            f64::from(target[1]),
            1,
            "minecraft:overworld",
            true,
            true,
        );
        assert_eq!((*id, data), (5, &fresh));
        let column = stream.load_chunk_now(minecraftoss_core::ChunkPos::new(
            target[0] >> 4,
            target[1] >> 4,
        ));
        let chests: Vec<_> = column
            .block_entities
            .pending
            .iter()
            .chain(column.block_entities.entities.iter())
            .filter(|(pos, _)| (pos.0, pos.2) == (target[0], target[1]))
            .filter_map(|(_, tag)| tag.get("LootTable").and_then(Tag::as_str))
            .collect();
        assert_eq!(chests, ["minecraft:chests/buried_treasure"]);
    }

    #[test]
    fn a_chest_without_maps_or_with_nothing_to_find_makes_no_map() {
        let Some(stream) = overworld_stream(77) else {
            return;
        };
        let gen = stream.world_gen();
        let registries = gen.terrain.registries.clone();
        let recipes = minecraftoss_player::crafting::RecipeBook::default();
        let explorer = Arc::new(LootExplorer::new(gen.clone(), "minecraft:overworld", 0));
        let (items, _) = crate::containers::structure_loot_in(
            &registries,
            &chest("minecraft:chests/simple_dungeon", 9),
            (0, 30, 0),
            77,
            &recipes,
            None,
            None,
            Some(explorer.clone()),
        )
        .unwrap();
        assert!(!items.is_empty());
        let none = ["minecraft:fortress".to_owned()];
        assert_eq!(explorer.explore(&none, [0, 64, 0], 50, false, 1), None);
        let explored = explorer.take();
        assert!(explored.maps.is_empty() && explored.timed_out.is_empty());
        let late = LootExplorer::new(gen, "minecraft:overworld", 0);
        let villages = ["minecraft:village_plains".to_owned()];
        let started = Instant::now();
        let gave_up = late.explore(&villages, [40_000, 64, 40_000], 100, false, 2);
        assert!(started.elapsed() < SEARCH_TIME + Duration::from_secs(2));
        if gave_up.is_none() {
            assert_eq!(late.take().timed_out, ["minecraft:village_plains"]);
        }
    }
}
