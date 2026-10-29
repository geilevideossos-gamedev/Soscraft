use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use minecraft_net::PlayerUuid;
use minecraftoss_core::ChunkPos;
use minecraftoss_core::loot::Exploring;
use minecraftoss_core::nbt::Tag;
use minecraftoss_generator::zoom;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_world::chunk_map::WorldGen;
use minecraftoss_world::level_storage::LevelStorage;
use serde_json::Value;

use crate::locate::{self, Search};

pub const MAP_SIZE: usize = 128;
pub const COLORS: usize = MAP_SIZE * MAP_SIZE;
pub const MAX_SCALE: u8 = 4;
pub const LAST_ID: &str = "minecraft:maps/last_id";
pub const MAP_CLAIMS: &str = "iw4l:map_claims";
pub const MAX_REFERENCES: i32 = 1;
pub const DIMENSIONS: [&str; 3] = [
    "minecraft:overworld",
    "minecraft:the_nether",
    "minecraft:the_end",
];
pub const PLAYER: &str = "minecraft:player";
pub const PLAYER_OFF_MAP: &str = "minecraft:player_off_map";
pub const PLAYER_OFF_LIMITS: &str = "minecraft:player_off_limits";
pub const MAP_ID: &str = "minecraft:map_id";
pub const MAP_DECORATIONS: &str = "minecraft:map_decorations";
pub const MAP_INVISIBILITY_EQUIPMENT: &str = "minecraft:map_invisibility_equipment";
pub const MAP_ITEMS: [&str; 17] = [
    "minecraft:filled_map",
    "minecraft:ocean_monument_map",
    "minecraft:woodland_mansion_map",
    "minecraft:buried_trial_chambers_map",
    "minecraft:jungle_pyramid_map",
    "minecraft:swamp_hut_map",
    "minecraft:desert_village_map",
    "minecraft:plains_village_map",
    "minecraft:savanna_village_map",
    "minecraft:snowy_village_map",
    "minecraft:taiga_village_map",
    "minecraft:buried_treasure_map",
    "minecraft:buried_ancient_city_map",
    "minecraft:buried_mineshaft_map",
    "minecraft:desert_pyramid_map",
    "minecraft:abandoned_camp_map",
    "minecraft:warm_ocean_ruins_map",
];
const WORN_SLOTS: [usize; 6] = [36, 37, 38, 39, 41, 42];
pub const SEARCH_TIME: Duration = Duration::from_secs(2);
pub const WATER_ON_MAP_OUTLINES: &str = "minecraft:water_on_map_outlines";
pub const COLOR_ORANGE: u8 = 15;
pub const COLOR_BROWN: u8 = 26;
pub const LOW: u8 = 0;
pub const NORMAL: u8 = 1;
pub const HIGH: u8 = 2;
pub const LOWEST: u8 = 3;
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
        let list = |key: &str| tag.get(key).and_then(Tag::as_list).unwrap_or(&[]).to_vec();
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

pub fn packed(color: u8, brightness: u8) -> u8 {
    (color << 2) | (brightness & 3)
}

pub fn biome_preview(map: &mut MapData, mut watery: impl FnMut(i32, i32) -> bool) {
    let step = 1i32 << map.scale;
    let left = map.x_center / step - 64;
    let top = map.z_center / step - 64;
    let mut wet = vec![false; COLORS];
    for z in 0..MAP_SIZE {
        for x in 0..MAP_SIZE {
            wet[z * MAP_SIZE + x] = watery((left + x as i32) * step, (top + z as i32) * step);
        }
    }
    for x in 1..MAP_SIZE - 1 {
        for z in 1..MAP_SIZE - 1 {
            let mut around = 0;
            for dx in 0..3 {
                for dz in 0..3 {
                    if (dx, dz) != (1, 1) && wet[(z + dz - 1) * MAP_SIZE + x + dx - 1] {
                        around += 1;
                    }
                }
            }
            let shade = if wet[z * MAP_SIZE + x] {
                if around > 7 && z % 2 == 0 {
                    let ripple =
                        (minecraftoss_generator::providers::sin(f64::from(z as f32)) * 7.0) as i32;
                    let band = match (x as i32 + ripple) / 8 % 5 {
                        0 | 4 => LOW,
                        1 | 3 => NORMAL,
                        2 => HIGH,
                        _ => LOWEST,
                    };
                    Some((COLOR_ORANGE, band))
                } else if around > 7 {
                    None
                } else if around > 5 {
                    Some((COLOR_ORANGE, NORMAL))
                } else if around > 1 {
                    Some((COLOR_ORANGE, LOW))
                } else {
                    Some((COLOR_ORANGE, LOWEST))
                }
            } else if around > 3 {
                Some((COLOR_BROWN, NORMAL))
            } else if around > 0 {
                Some((COLOR_BROWN, LOWEST))
            } else {
                None
            };
            if let Some((color, brightness)) = shade {
                map.colors[z * MAP_SIZE + x] = packed(color, brightness);
            }
        }
    }
}

pub fn coastline(gen: &WorldGen, map: &mut MapData) {
    let terrain = &gen.terrain;
    let registries = &terrain.registries;
    let Some(tag) = registries.biome_tags.id(WATER_ON_MAP_OUTLINES) else {
        return;
    };
    let zoom_seed = zoom::zoom_seed(terrain.seed);
    let mut quarts = HashMap::new();
    biome_preview(map, |x, z| {
        let quart = zoom::quart_for_block(zoom_seed, x, terrain.sea_level, z);
        *quarts.entry(quart).or_insert_with(|| {
            registries.biome_in_tag(terrain.biome_at_quart(quart[0], quart[1], quart[2]), tag)
        })
    });
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

pub fn is_map_item(id: &str) -> bool {
    MAP_ITEMS.contains(&id)
}

pub fn tracks_count(kind: &str) -> bool {
    matches!(
        kind,
        PLAYER
            | PLAYER_OFF_MAP
            | PLAYER_OFF_LIMITS
            | "minecraft:frame"
            | "minecraft:red_marker"
            | "minecraft:blue_marker"
    ) || kind.starts_with("minecraft:banner_")
}

pub fn wears_map_invisibility(inventory: &Inventory) -> bool {
    WORN_SLOTS
        .iter()
        .filter_map(|&slot| inventory.slots.get(slot)?.as_ref())
        .any(|stack| {
            inventory
                .recipes
                .item_in_tag(MAP_INVISIBILITY_EQUIPMENT, &stack.id)
        })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub kind: String,
    pub x: f64,
    pub z: f64,
    pub rotation: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CarriedMap {
    pub item: String,
    pub id: i32,
    pub marks: Vec<(String, Mark)>,
}

impl CarriedMap {
    pub fn of(stack: &ItemStack) -> Option<Self> {
        if !is_map_item(&stack.id) {
            return None;
        }
        let components = stack.components.as_ref()?;
        let id = i32::try_from(components.get(MAP_ID)?.as_i64()?).ok()?;
        let marks = components
            .get(MAP_DECORATIONS)
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(key, entry)| {
                let kind = entry.get("type")?.as_str()?;
                let mark = Mark {
                    kind: if kind.contains(':') {
                        kind.to_owned()
                    } else {
                        format!("minecraft:{kind}")
                    },
                    x: entry.get("x")?.as_f64()?,
                    z: entry.get("z")?.as_f64()?,
                    rotation: entry.get("rotation")?.as_f64()? as f32,
                };
                Some((key.clone(), mark))
            })
            .collect();
        Some(Self {
            item: stack.id.clone(),
            id,
            marks,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Carrier {
    pub uuid: PlayerUuid,
    pub dimension: String,
    pub x: f64,
    pub z: f64,
    pub yaw: f32,
    pub maps: Vec<CarriedMap>,
    pub hidden: bool,
}

impl Carrier {
    pub fn carries(&self, item: &str, id: i32) -> bool {
        self.maps.iter().any(|map| map.item == item && map.id == id)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Holding {
    uuid: PlayerUuid,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Held {
    decorations: Vec<(String, Decoration)>,
    tracked: i32,
    holders: Vec<Holding>,
}

impl Held {
    pub fn decorations(&self) -> impl Iterator<Item = &Decoration> {
        self.decorations.iter().map(|(_, decoration)| decoration)
    }

    pub fn decoration(&self, key: &str) -> Option<&Decoration> {
        self.decorations
            .iter()
            .find(|(shown, _)| shown == key)
            .map(|(_, decoration)| decoration)
    }

    pub fn tracked(&self) -> i32 {
        self.tracked
    }

    pub fn holders(&self) -> impl Iterator<Item = &PlayerUuid> {
        self.holders.iter().map(|holding| &holding.uuid)
    }

    fn add(
        &mut self,
        data: &MapData,
        (kind, key): (&str, &str),
        (x, z, rot): (f64, f64, f64),
        game_time: i64,
    ) {
        let Some(decoration) = data.decoration(kind, x, z, rot, Some(game_time)) else {
            self.remove(key);
            return;
        };
        let old = match self.decorations.iter_mut().find(|(shown, _)| shown == key) {
            Some((_, old)) => Some(std::mem::replace(old, decoration.clone())),
            None => {
                self.decorations.push((key.to_owned(), decoration.clone()));
                None
            }
        };
        if old.as_ref() != Some(&decoration) {
            if old.is_some_and(|old| tracks_count(&old.kind)) {
                self.tracked -= 1;
            }
            if tracks_count(&decoration.kind) {
                self.tracked += 1;
            }
        }
    }

    fn remove(&mut self, key: &str) {
        if let Some(index) = self.decorations.iter().position(|(shown, _)| shown == key) {
            let (_, old) = self.decorations.remove(index);
            if tracks_count(&old.kind) {
                self.tracked -= 1;
            }
        }
    }
}

#[derive(Debug)]
pub struct MapStore {
    maps: BTreeMap<i32, MapData>,
    last_id: i32,
    dirty: BTreeSet<i32>,
    index_dirty: bool,
    claims: BTreeMap<String, Claims>,
    claims_dirty: BTreeSet<String>,
    held: BTreeMap<i32, Held>,
}

impl Default for MapStore {
    fn default() -> Self {
        Self {
            maps: BTreeMap::new(),
            last_id: -1,
            dirty: BTreeSet::new(),
            index_dirty: false,
            claims: BTreeMap::new(),
            claims_dirty: BTreeSet::new(),
            held: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Claims(BTreeMap<(String, i32, i32), i32>);

impl Claims {
    pub fn try_claim(&mut self, structure: &str, chunk: ChunkPos) -> bool {
        let references = self
            .0
            .entry((structure.to_owned(), chunk.x, chunk.z))
            .or_insert(0);
        if *references >= MAX_REFERENCES {
            return false;
        }
        *references += 1;
        true
    }

    pub fn references(&self, structure: &str, chunk: ChunkPos) -> i32 {
        self.0
            .get(&(structure.to_owned(), chunk.x, chunk.z))
            .copied()
            .unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn to_tag(&self) -> Tag {
        let claims = self
            .0
            .iter()
            .map(|((structure, x, z), references)| {
                Tag::Compound(BTreeMap::from([
                    ("structure".to_owned(), Tag::String(structure.clone())),
                    ("chunk_x".to_owned(), Tag::Int(*x)),
                    ("chunk_z".to_owned(), Tag::Int(*z)),
                    ("references".to_owned(), Tag::Int(*references)),
                ]))
            })
            .collect();
        Tag::Compound(BTreeMap::from([("claims".to_owned(), Tag::List(claims))]))
    }

    pub fn from_tag(tag: &Tag) -> Self {
        let mut claims = BTreeMap::new();
        for entry in tag.get("claims").and_then(Tag::as_list).unwrap_or(&[]) {
            let int = |key: &str| entry.get(key).and_then(Tag::as_i64).map(|v| v as i32);
            let structure = entry.get("structure").and_then(Tag::as_str);
            let (Some(structure), Some(x), Some(z)) = (structure, int("chunk_x"), int("chunk_z"))
            else {
                continue;
            };
            claims.insert((structure.to_owned(), x, z), int("references").unwrap_or(1));
        }
        Self(claims)
    }
}

#[derive(Debug, Default)]
pub struct MapWrites {
    pub maps: Vec<(i32, Tag)>,
    pub last_id: Option<i32>,
    pub claims: Vec<(String, Tag)>,
}

impl MapWrites {
    pub fn is_empty(&self) -> bool {
        self.maps.is_empty() && self.last_id.is_none() && self.claims.is_empty()
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
        for (dimension, claims) in &self.claims {
            if let Err(error) = storage.write_saved_in(dimension, MAP_CLAIMS, claims.clone()) {
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
        for dimension in DIMENSIONS {
            match storage.read_saved_in(dimension, MAP_CLAIMS) {
                Ok(Some(tag)) => {
                    store
                        .claims
                        .insert(dimension.to_owned(), Claims::from_tag(&tag));
                }
                Ok(None) => {}
                Err(error) => warnings.push(format!(
                    "Minecraft map claims of {dimension} unreadable: {error}"
                )),
            }
        }
        (store, warnings)
    }

    pub fn claims(&self, dimension: &str) -> Claims {
        self.claims
            .get(dimension)
            .map_or_else(Claims::default, Claims::clone)
    }

    pub fn set_claims(&mut self, dimension: &str, claims: Claims) {
        if self.claims.get(dimension).unwrap_or(&Claims::default()) != &claims {
            self.claims.insert(dimension.to_owned(), claims);
            self.claims_dirty.insert(dimension.to_owned());
        }
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
        let claims = std::mem::take(&mut self.claims_dirty)
            .into_iter()
            .filter_map(|dimension| {
                let tag = self.claims.get(&dimension)?.to_tag();
                Some((dimension, tag))
            })
            .collect();
        MapWrites {
            maps,
            last_id,
            claims,
        }
    }

    pub fn held(&self, id: i32) -> Option<&Held> {
        self.held.get(&id)
    }

    pub fn tick_carried(&mut self, carriers: &[Carrier], game_time: i64) {
        for carrier in carriers {
            for map in &carrier.maps {
                self.tick_carried_by(carrier, map, carriers, game_time);
            }
        }
    }

    fn tick_carried_by(
        &mut self,
        carrier: &Carrier,
        map: &CarriedMap,
        carriers: &[Carrier],
        game_time: i64,
    ) {
        let Some(data) = self.maps.get(&map.id) else {
            return;
        };
        let held = self.held.entry(map.id).or_default();
        if !held
            .holders
            .iter()
            .any(|holding| holding.uuid == carrier.uuid)
        {
            held.holders.push(Holding {
                uuid: carrier.uuid.clone(),
            });
        }
        let mut index = 0;
        while index < held.holders.len() {
            let uuid = held.holders[index].uuid.clone();
            let holder = carriers.iter().find(|other| other.uuid == uuid);
            match holder {
                Some(other) if other.carries(&map.item, map.id) => {
                    if other.dimension == data.dimension && data.tracking_position {
                        let at = (other.x, other.z, f64::from(other.yaw));
                        held.add(data, (PLAYER, &uuid.0), at, game_time);
                    }
                }
                _ => {
                    held.holders.remove(index);
                    held.remove(&uuid.0);
                }
            }
            if holder.is_some_and(|other| other.uuid != carrier.uuid && other.hidden) {
                held.remove(&uuid.0);
            }
            index += 1;
        }
        for (key, mark) in &map.marks {
            if held.decoration(key).is_none() {
                let at = (mark.x, mark.z, f64::from(mark.rotation));
                held.add(data, (&mark.kind, key), at, game_time);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Explored {
    pub maps: Vec<(i32, MapData)>,
    pub timed_out: Vec<String>,
    pub claims: Option<Claims>,
}

pub struct LootExplorer {
    gen: Arc<WorldGen>,
    dimension: String,
    first_id: i32,
    known: Claims,
    search_time: Duration,
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
            known: Claims::default(),
            search_time: SEARCH_TIME,
            explored: Mutex::new(Explored::default()),
        }
    }

    pub fn with_search_time(mut self, search_time: Duration) -> Self {
        self.search_time = search_time;
        self
    }

    pub fn with_claims(mut self, claims: Claims) -> Self {
        self.known = claims;
        self
    }

    pub fn dimension(&self) -> &str {
        &self.dimension
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
        skip_known: bool,
        zoom: u8,
    ) -> Option<(i32, [i32; 3])> {
        let deadline = Instant::now() + self.search_time;
        let mut explored = self.explored();
        let search = if skip_known {
            let mut claims = explored.claims.take().unwrap_or_else(|| self.known.clone());
            let mut claim = |name: &str, chunk: ChunkPos| claims.try_claim(name, chunk);
            locate::claim_structure_at(&self.gen, destination, origin, &mut claim);
            let search = locate::nearest_unclaimed_structure(
                &self.gen,
                destination,
                origin,
                radius,
                deadline,
                Some(&mut claim),
            );
            explored.claims = Some(claims);
            search
        } else {
            locate::nearest_structure(&self.gen, destination, origin, radius, deadline)
        };
        let target = match search {
            Search::Found(target, _) => target,
            Search::NotFound => return None,
            Search::TimedOut => {
                explored.timed_out.push(destination.join(", "));
                return None;
            }
        };
        let mut map = MapData::create_fresh(
            f64::from(target[0]),
            f64::from(target[2]),
            zoom.min(MAX_SCALE),
            &self.dimension,
            true,
            true,
        );
        coastline(&self.gen, &mut map);
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
    fn the_coastline_preview_outlines_water_like_vanilla() {
        let mut map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, true);
        let mut asked = Vec::new();
        biome_preview(&mut map, |x, z| {
            asked.push((x, z));
            (x < 0 && (x, z) != (-30, 0)) || (x, z) == (36, 0)
        });
        assert_eq!(asked.len(), COLORS);
        assert_eq!(
            (asked[0], asked[1], asked[MAP_SIZE]),
            ((-64, -64), (-63, -64), (-64, -63))
        );
        let at = |x: usize, z: usize| map.colors[z * MAP_SIZE + x];
        assert_eq!(
            at(63, 40),
            packed(COLOR_ORANGE, LOW),
            "the water at the shore"
        );
        assert_eq!(
            at(64, 40),
            packed(COLOR_BROWN, LOWEST),
            "the land at the shore"
        );
        assert_eq!(at(65, 40), 0);
        assert_eq!(at(10, 20), packed(COLOR_ORANGE, HIGH));
        assert_eq!(at(30, 20), packed(COLOR_ORANGE, LOW));
        assert_eq!(at(10, 4), packed(COLOR_ORANGE, LOW));
        assert_eq!(at(13, 4), packed(COLOR_ORANGE, NORMAL));
        assert_eq!(at(10, 21), 0, "odd rows of open water stay clear");
        assert_eq!(
            at(33, 64),
            packed(COLOR_ORANGE, NORMAL),
            "water beside a dry spot"
        );
        assert_eq!(
            at(34, 64),
            packed(COLOR_BROWN, NORMAL),
            "a dry spot in the water"
        );
        assert_eq!(
            at(100, 64),
            packed(COLOR_ORANGE, LOWEST),
            "a pond on its own"
        );
        assert_eq!(at(101, 64), packed(COLOR_BROWN, LOWEST));
        assert_eq!(
            (at(0, 40), at(10, 0), at(127, 64)),
            (0, 0, 0),
            "the border is never drawn"
        );
        assert_eq!(
            (packed(COLOR_ORANGE, HIGH), packed(COLOR_BROWN, LOWEST)),
            (62, 107)
        );
        let mut wide = MapData::create_fresh(1000.0, -300.0, 1, "minecraft:overworld", true, true);
        let mut corners = Vec::new();
        biome_preview(&mut wide, |x, z| {
            corners.push((x, z));
            false
        });
        assert_eq!(
            (corners[0], corners[COLORS - 1]),
            ((960, -320), (1214, -66))
        );
        assert!(wide.colors.iter().all(|&c| c == 0));
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

    fn patient(gen: &Arc<WorldGen>, first_id: i32) -> LootExplorer {
        LootExplorer::new(gen.clone(), "minecraft:overworld", first_id)
            .with_search_time(Duration::from_secs(120))
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
        let explorer = Arc::new(patient(&gen, 5));
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
        let mut fresh = MapData::create_fresh(
            f64::from(target[0]),
            f64::from(target[1]),
            1,
            "minecraft:overworld",
            true,
            true,
        );
        coastline(&gen, &mut fresh);
        assert_eq!((*id, data), (5, &fresh));
        let colours: BTreeSet<u8> = data.colors.iter().map(|&c| c >> 2).collect();
        assert!(
            colours.is_subset(&BTreeSet::from([0, COLOR_ORANGE, COLOR_BROWN])),
            "{colours:?}"
        );
        assert!(
            colours.contains(&COLOR_ORANGE) && colours.contains(&COLOR_BROWN),
            "the treasure lies on a coast: {colours:?}"
        );
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
        let explorer = Arc::new(patient(&gen, 0));
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
        assert_eq!(
            LootExplorer::new(gen.clone(), "minecraft:overworld", 0).search_time,
            SEARCH_TIME
        );
        let late = LootExplorer::new(gen, "minecraft:overworld", 0)
            .with_search_time(Duration::from_millis(100));
        let villages = ["minecraft:village_plains".to_owned()];
        let started = Instant::now();
        let gave_up = late.explore(&villages, [40_000, 64, 40_000], 100, false, 2);
        assert!(started.elapsed() < Duration::from_secs(30));
        if gave_up.is_none() {
            assert_eq!(late.take().timed_out, ["minecraft:village_plains"]);
        }
    }

    #[test]
    fn claims_hold_one_reference_and_are_saved_beside_each_level() {
        let mut claims = Claims::default();
        let chunk = ChunkPos::new(3, -7);
        assert!(claims.try_claim("minecraft:buried_treasure", chunk));
        assert!(!claims.try_claim("minecraft:buried_treasure", chunk));
        assert!(claims.try_claim("minecraft:buried_treasure", ChunkPos::new(3, -6)));
        assert!(claims.try_claim("minecraft:village_plains", chunk));
        assert_eq!(claims.references("minecraft:buried_treasure", chunk), 1);
        assert_eq!(claims.references("minecraft:mansion", chunk), 0);
        assert_eq!(claims.len(), 3);
        assert_eq!(Claims::from_tag(&claims.to_tag()), claims);
        let dir = crate::test_dir::tempdir().unwrap();
        let world = dir.path().join("world");
        let storage = LevelStorage::open(&world).unwrap();
        let (mut store, _) = MapStore::load(&storage);
        store.set_claims("minecraft:overworld", claims.clone());
        store.set_claims("minecraft:the_end", Claims::default());
        let writes = store.take_writes();
        assert_eq!(writes.claims.len(), 1, "an empty claim list is not written");
        writes.write(&storage).unwrap();
        let file = world.join("dimensions/minecraft/overworld/data/iw4l/map_claims.dat");
        assert!(file.is_file());
        assert!(
            !world
                .join("dimensions/minecraft/the_end/data/iw4l/map_claims.dat")
                .exists()
        );
        store.set_claims("minecraft:overworld", claims.clone());
        assert!(
            store.take_writes().is_empty(),
            "unchanged claims are not written again"
        );
        drop(storage);
        let storage = LevelStorage::open(&world).unwrap();
        let (again, warnings) = MapStore::load(&storage);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(again.claims("minecraft:overworld"), claims);
        assert!(again.claims("minecraft:the_nether").is_empty());
    }

    fn explore(
        explorer: &LootExplorer,
        targets: &[String],
        origin: [i32; 3],
        radius: i32,
        skip: bool,
    ) -> [i32; 3] {
        explorer
            .explore(targets, origin, radius, skip, 1)
            .unwrap_or_else(|| {
                panic!(
                    "nothing found from {origin:?}: {:?}",
                    explorer.take().timed_out
                )
            })
            .1
    }

    #[test]
    fn treasure_maps_that_skip_known_treasures_each_point_to_another() {
        let Some(stream) = overworld_stream(77) else {
            return;
        };
        let gen = stream.world_gen();
        let deadline = Instant::now() + Duration::from_secs(120);
        let shipwrecks = ["minecraft:shipwreck".to_owned()];
        let wreck = found(locate::nearest_structure(
            &gen,
            &shipwrecks,
            [0, 64, 0],
            100,
            deadline,
        ));
        let origin = [wreck[0], 50, wreck[2]];
        let treasure = ["minecraft:buried_treasure".to_owned()];
        let nearest = found(locate::nearest_structure(
            &gen, &treasure, origin, 50, deadline,
        ));
        let explorer = patient(&gen, 0);
        let first = explore(&explorer, &treasure, origin, 50, true);
        let second = explore(&explorer, &treasure, origin, 50, true);
        assert_eq!(first, nearest);
        assert_ne!(second, first);
        assert_eq!(
            explore(&explorer, &treasure, origin, 50, false),
            nearest,
            "a map that keeps known treasures points to the nearest"
        );
        let explored = explorer.take();
        assert_eq!(
            explored.maps.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        let claims = explored
            .claims
            .expect("the skipping maps claimed their treasures");
        assert_eq!(claims.len(), 2);
        for target in [first, second] {
            let chunk = ChunkPos::new(target[0] >> 4, target[2] >> 4);
            assert_eq!(claims.references("minecraft:buried_treasure", chunk), 1);
        }
        let dir = crate::test_dir::tempdir().unwrap();
        let world = dir.path().join("world");
        let storage = LevelStorage::open(&world).unwrap();
        let (mut store, _) = MapStore::load(&storage);
        store.set_claims("minecraft:overworld", claims);
        store.take_writes().write(&storage).unwrap();
        drop(storage);
        let (loaded, _) = MapStore::load(&LevelStorage::open(&world).unwrap());
        let later = patient(&gen, 3).with_claims(loaded.claims("minecraft:overworld"));
        let third = explore(&later, &treasure, origin, 50, true);
        assert!(
            third != first && third != second,
            "{third:?} after {first:?} and {second:?}"
        );
        assert_eq!(later.take().claims.map(|claims| claims.len()), Some(3));
    }

    #[test]
    fn a_map_that_skips_known_structures_never_points_at_the_one_it_lies_in() {
        let Some(stream) = overworld_stream(77) else {
            return;
        };
        let gen = stream.world_gen();
        let deadline = Instant::now() + Duration::from_secs(120);
        let shipwrecks = ["minecraft:shipwreck".to_owned()];
        let home = found(locate::nearest_structure(
            &gen,
            &shipwrecks,
            [0, 64, 0],
            100,
            deadline,
        ));
        let chunk = ChunkPos::new(home[0] >> 4, home[2] >> 4);
        let wreck = gen.structures.id("minecraft:shipwreck").unwrap();
        let start = gen
            .structures
            .starts_in(&gen.library, &gen.terrain, chunk)
            .iter()
            .find(|start| start.structure == wreck)
            .cloned()
            .expect("the located chunk starts the shipwreck");
        let middle = start.bounding_box().center();
        let inside = [middle.x, middle.y, middle.z];
        let keeping = patient(&gen, 0);
        assert_eq!(explore(&keeping, &shipwrecks, inside, 100, false), home);
        assert!(
            keeping.take().claims.is_none(),
            "a map that keeps known structures claims nothing"
        );
        let skipping = patient(&gen, 0);
        let away = explore(&skipping, &shipwrecks, inside, 100, true);
        assert_ne!(away, home);
        let claims = skipping.take().claims.unwrap();
        assert_eq!(
            claims.references("minecraft:shipwreck", start.chunk),
            1,
            "the wreck it lies in"
        );
        assert_eq!(
            claims.references(
                "minecraft:shipwreck",
                ChunkPos::new(away[0] >> 4, away[2] >> 4)
            ),
            1
        );
        assert_eq!(claims.len(), 2);
        let outside = patient(&gen, 0);
        explore(
            &outside,
            &shipwrecks,
            [home[0] + 2000, 64, home[2]],
            100,
            true,
        );
        assert_eq!(
            outside.take().claims.map(|claims| claims.len()),
            Some(1),
            "outside any wreck only the target is claimed"
        );
    }

    fn carried(item: &str, id: i32, marks: &[(&str, &str, f64, f64)]) -> CarriedMap {
        CarriedMap {
            item: item.to_owned(),
            id,
            marks: marks
                .iter()
                .map(|&(key, kind, x, z)| {
                    let mark = Mark {
                        kind: kind.to_owned(),
                        x,
                        z,
                        rotation: 180.0,
                    };
                    (key.to_owned(), mark)
                })
                .collect(),
        }
    }

    fn holder(uuid: &str, (x, z, yaw): (f64, f64, f32), maps: Vec<CarriedMap>) -> Carrier {
        Carrier {
            uuid: PlayerUuid(uuid.to_owned()),
            dimension: "minecraft:overworld".to_owned(),
            x,
            z,
            yaw,
            maps,
            hidden: false,
        }
    }

    fn shown(store: &MapStore, id: i32) -> Vec<(&str, i8, i8, i8)> {
        store.held(id).map_or_else(Vec::new, |held| {
            held.decorations()
                .map(|d| (d.kind.as_str(), d.x, d.y, d.rot))
                .collect()
        })
    }

    fn store_with(maps: &[(i32, MapData)]) -> MapStore {
        let mut store = MapStore::default();
        for (id, map) in maps {
            store.insert(*id, map.clone());
        }
        store.take_writes();
        store
    }

    #[test]
    fn a_carried_map_marks_its_holder_and_its_target_like_vanilla() {
        let small = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        let mut store = store_with(&[(0, small.clone())]);
        let treasure = carried(
            "minecraft:buried_treasure_map",
            0,
            &[("+", "minecraft:red_x", 30.0, -50.0)],
        );
        let mut steve = holder("steve", (10.4, -20.0, 90.0), vec![treasure]);
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(
            shown(&store, 0),
            [(PLAYER, 21, -39, 4), ("minecraft:red_x", 60, -99, 8)],
            "the holder first, then the stack's mark"
        );
        let held = store.held(0).unwrap();
        assert_eq!(held.tracked(), 1, "players count, the red X does not");
        assert_eq!(
            held.holders().collect::<Vec<_>>(),
            [&PlayerUuid("steve".to_owned())]
        );
        steve.x = 300.0;
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(shown(&store, 0)[0], (PLAYER_OFF_MAP, 127, -39, 4));
        steve.x = 400.0;
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(
            shown(&store, 0),
            [("minecraft:red_x", 60, -99, 8)],
            "far from a plain map the holder has no marker"
        );
        assert_eq!(store.held(0).unwrap().tracked(), 0);
        let mut wide = small.clone();
        wide.unlimited_tracking = true;
        let mut store = store_with(&[(0, wide)]);
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(shown(&store, 0)[0], (PLAYER_OFF_LIMITS, 127, -39, 4));
        steve.x = 10.4;
        store.tick_carried(std::slice::from_ref(&steve), 0);
        steve.dimension = "minecraft:the_nether".to_owned();
        steve.x = -50.0;
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(
            shown(&store, 0)[0],
            (PLAYER, 21, -39, 4),
            "in another dimension the marker stays where it was"
        );
        let mut untracked = small.clone();
        untracked.tracking_position = false;
        let mut store = store_with(&[(0, untracked)]);
        steve.dimension = "minecraft:overworld".to_owned();
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(shown(&store, 0), [("minecraft:red_x", 60, -99, 8)]);
        let nether = MapData::create_fresh(0.0, 0.0, 0, "minecraft:the_nether", true, false);
        let mut store = store_with(&[(0, nether)]);
        steve.dimension = "minecraft:the_nether".to_owned();
        steve.x = 10.4;
        store.tick_carried(std::slice::from_ref(&steve), 1234);
        let spin = nether_rotation(1234);
        assert_eq!(
            shown(&store, 0),
            [(PLAYER, 21, -39, spin), ("minecraft:red_x", 60, -99, spin)],
            "every marker on a Nether map spins"
        );
        let mut store = store_with(&[]);
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert!(store.held(0).is_none(), "a map without data is not held");
    }

    #[test]
    fn every_holder_is_marked_until_they_drop_the_map_or_leave() {
        let map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        let mut store = store_with(&[(0, map)]);
        let treasure = || {
            carried(
                "minecraft:filled_map",
                0,
                &[("+", "minecraft:red_x", 0.0, 0.0)],
            )
        };
        let mut steve = holder("steve", (2.0, 2.0, 0.0), vec![treasure()]);
        let mut alex = holder("alex", (-2.0, 4.0, 180.0), vec![treasure()]);
        store.tick_carried(&[steve.clone(), alex.clone()], 0);
        assert_eq!(
            shown(&store, 0),
            [
                (PLAYER, 4, 4, 0),
                ("minecraft:red_x", 0, 0, 8),
                (PLAYER, -3, 8, 8)
            ]
        );
        assert_eq!(store.held(0).unwrap().tracked(), 2);
        alex.hidden = true;
        store.tick_carried(&[alex.clone(), steve.clone()], 0);
        assert_eq!(
            shown(&store, 0),
            [(PLAYER, 4, 4, 0), ("minecraft:red_x", 0, 0, 8)],
            "another holder's tick hides a pumpkin wearer"
        );
        store.tick_carried(&[steve.clone(), alex.clone()], 0);
        assert_eq!(
            shown(&store, 0).last(),
            Some(&(PLAYER, -3, 8, 8)),
            "the wearer's own tick shows them again"
        );
        alex.hidden = false;
        let other = carried("minecraft:buried_treasure_map", 0, &[]);
        steve.maps = vec![other];
        alex.x = 20.0;
        store.tick_carried(std::slice::from_ref(&alex), 0);
        assert_eq!(
            shown(&store, 0),
            [("minecraft:red_x", 0, 0, 8), (PLAYER, -3, 8, 8)],
            "a holder who no longer carries this map loses their marker"
        );
        assert_eq!(
            store.held(0).unwrap().holders().collect::<Vec<_>>(),
            [&PlayerUuid("alex".to_owned())],
            "a map of another kind with the same id is not this map"
        );
        assert_eq!(store.held(0).unwrap().tracked(), 1);
        store.tick_carried(std::slice::from_ref(&alex), 0);
        assert_eq!(
            shown(&store, 0)[1],
            (PLAYER, 40, 8, 8),
            "the holder after a dropped one moves on the next tick, as vanilla's loop skips it"
        );
        steve.maps = vec![treasure()];
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(
            shown(&store, 0),
            [("minecraft:red_x", 0, 0, 8)],
            "a holder who left the game loses their marker"
        );
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(
            shown(&store, 0),
            [("minecraft:red_x", 0, 0, 8), (PLAYER, 4, 4, 0)]
        );
    }

    #[test]
    fn carried_maps_read_their_id_and_marks_from_map_items_only() {
        let mut map = ItemStack::new("minecraft:filled_map", 1);
        map.components = Some(serde_json::json!({
            "minecraft:map_id": 3,
            "minecraft:map_decorations": {
                "+": {"type": "minecraft:red_x", "x": 1000.0, "z": -300, "rotation": 180.0},
                "camp": {"type": "abandoned_camp", "x": 5.5, "z": 6.5, "rotation": 90.0},
                "broken": {"type": "minecraft:red_x", "x": 1.0}
            }
        }));
        let read = CarriedMap::of(&map).unwrap();
        assert_eq!((read.item.as_str(), read.id), ("minecraft:filled_map", 3));
        assert_eq!(
            read.marks,
            [
                (
                    "+".to_owned(),
                    Mark {
                        kind: "minecraft:red_x".to_owned(),
                        x: 1000.0,
                        z: -300.0,
                        rotation: 180.0
                    }
                ),
                (
                    "camp".to_owned(),
                    Mark {
                        kind: "minecraft:abandoned_camp".to_owned(),
                        x: 5.5,
                        z: 6.5,
                        rotation: 90.0
                    }
                )
            ]
        );
        let mut explorer = map.clone();
        explorer.id = "minecraft:woodland_mansion_map".to_owned();
        assert!(CarriedMap::of(&explorer).is_some());
        for id in ["minecraft:map", "minecraft:stone", "minecraft:compass"] {
            let mut other = map.clone();
            other.id = id.to_owned();
            assert_eq!(CarriedMap::of(&other), None, "{id}");
        }
        assert_eq!(
            CarriedMap::of(&ItemStack::new("minecraft:filled_map", 1)),
            None
        );
        assert_eq!(MAP_ITEMS.len(), 17);
        assert!(MAP_ITEMS.iter().all(|id| is_map_item(id)));
        assert!(tracks_count("minecraft:banner_red") && tracks_count(PLAYER_OFF_LIMITS));
        assert!(!tracks_count("minecraft:red_x") && !tracks_count("minecraft:mansion"));
    }

    #[test]
    fn marks_of_carried_maps_are_never_saved() {
        let map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        let mut store = store_with(&[(0, map.clone())]);
        let steve = holder(
            "steve",
            (1.0, 1.0, 0.0),
            vec![carried("minecraft:filled_map", 0, &[])],
        );
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(shown(&store, 0).len(), 1);
        assert!(
            store.take_writes().is_empty(),
            "a marker leaves the map file alone"
        );
        assert_eq!(store.get(0), Some(&map));
    }

    #[test]
    fn a_carved_pumpkin_on_the_head_hides_its_wearer_from_maps() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = minecraftoss_player::crafting::RecipeBook::from_jar(&jar).unwrap();
        let mut inventory = Inventory::default().with_recipes(book);
        assert!(!wears_map_invisibility(&inventory));
        for slot in [0, 40] {
            inventory.slots[slot] = Some(ItemStack::new("minecraft:carved_pumpkin", 1));
        }
        assert!(
            !wears_map_invisibility(&inventory),
            "a pumpkin in a hand hides nobody"
        );
        inventory.slots[39] = Some(ItemStack::new("minecraft:carved_pumpkin", 1));
        assert!(wears_map_invisibility(&inventory));
        inventory.slots[39] = Some(ItemStack::new("minecraft:diamond_helmet", 1));
        assert!(!wears_map_invisibility(&inventory));
    }
}
