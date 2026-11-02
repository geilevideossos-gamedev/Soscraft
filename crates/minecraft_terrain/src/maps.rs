use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use minecraft_net::PlayerUuid;
use minecraft_net::messages::{MapMark, MapPatch, MapUpdate};
use minecraftoss_core::loot::Exploring;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::{BlockStateId, ChunkPos, SupportType};
use minecraftoss_generator::zoom;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_world::chunk_map::WorldGen;
use minecraftoss_world::level_storage::LevelStorage;
use serde_json::Value;

use crate::block_map_colors::BlockMapColors;
use crate::fluid::FluidKind;
use crate::locate::{self, Search};
use crate::scene::{HandcraftedScene, Scene};

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
pub const EMPTY_MAP: &str = "minecraft:map";
pub const FILLED_MAP: &str = "minecraft:filled_map";
pub const TAKE_RESULT_SOUND: &str = "minecraft:ui.cartography_table.take_result";
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
pub const COLOR_DIRT: u8 = 10;
pub const COLOR_STONE: u8 = 11;
pub const COLOR_WATER: u8 = 12;
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

    pub fn fill_in(
        &self,
        (x, z): (f64, f64),
        step: i32,
        surface: &dyn MapSurface,
    ) -> Vec<(usize, usize, u8)> {
        let scale = 1i32 << self.scale;
        let holder_x = (x - f64::from(self.x_center)).floor() as i32 / scale + 64;
        let holder_z = (z - f64::from(self.z_center)).floor() as i32 / scale + 64;
        let ceiling = surface.ceiling();
        let mut radius = 128 / scale;
        if ceiling {
            radius /= 2;
        }
        let size = MAP_SIZE as i32;
        let mut painted = Vec::new();
        let mut changed = false;
        for col in holder_x - radius + 1..holder_x + radius {
            if col & 15 != step & 15 && !changed {
                continue;
            }
            changed = false;
            let mut previous = 0.0;
            for row in holder_z - radius - 1..holder_z + radius {
                if col < 0 || row < -1 || col >= size || row >= size {
                    continue;
                }
                let distance = (col - holder_x).pow(2) + (row - holder_z).pow(2);
                let ring = distance > (radius - 2) * (radius - 2);
                let world_x = (self.x_center / scale + col - 64) * scale;
                let world_z = (self.z_center / scale + row - 64) * scale;
                if !surface.loaded(world_x, world_z) {
                    continue;
                }
                let (color, depth, height) = if ceiling {
                    (speckle(world_x, world_z), 0, 100.0)
                } else {
                    surface_colour(surface, (world_x, world_z), scale)
                };
                let odd = f64::from((col + row) & 1);
                let brightness = if color == COLOR_WATER {
                    let shade = f64::from(depth) * 0.1 + odd * 0.2;
                    if shade < 0.5 {
                        HIGH
                    } else if shade > 0.9 {
                        LOW
                    } else {
                        NORMAL
                    }
                } else {
                    let shade =
                        (height - previous) * 4.0 / f64::from(scale + 4) + (odd - 0.5) * 0.4;
                    if shade > 0.6 {
                        HIGH
                    } else if shade < -0.6 {
                        LOW
                    } else {
                        NORMAL
                    }
                };
                previous = height;
                if row < 0 || distance >= radius * radius || (ring && (col + row) & 1 == 0) {
                    continue;
                }
                let packed = packed(color, brightness);
                if self.colors[col as usize + row as usize * MAP_SIZE] != packed {
                    changed = true;
                    painted.push((col as usize, row as usize, packed));
                }
            }
        }
        painted
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapBlock {
    pub color: u8,
    pub fluid: bool,
    pub shown: u8,
}

pub trait MapSurface {
    fn ceiling(&self) -> bool;
    fn min_y(&self) -> i32;
    fn loaded(&self, x: i32, z: i32) -> bool;
    fn height(&self, x: i32, z: i32) -> i32;
    fn block(&self, pos: [i32; 3]) -> MapBlock;
}

pub fn speckle(x: i32, z: i32) -> u8 {
    let seed = x.wrapping_add(z.wrapping_mul(231_871));
    let hash = seed
        .wrapping_mul(seed)
        .wrapping_mul(31_287_121)
        .wrapping_add(seed.wrapping_mul(11));
    if (hash >> 20) & 1 == 0 {
        COLOR_DIRT
    } else {
        COLOR_STONE
    }
}

fn surface_colour(surface: &dyn MapSurface, (x, z): (i32, i32), scale: i32) -> (u8, i32, f64) {
    let min_y = surface.min_y();
    let mut tally: Vec<(u8, i32)> = Vec::new();
    let mut depth = 0;
    let mut height = 0.0;
    for dx in 0..scale {
        for dz in 0..scale {
            let (bx, bz) = (x + dx, z + dz);
            let mut y = surface.height(bx, bz) + 1;
            let color = if y > min_y {
                y -= 1;
                let mut block = surface.block([bx, y, bz]);
                while block.color == 0 && y > min_y {
                    y -= 1;
                    block = surface.block([bx, y, bz]);
                }
                if y > min_y && block.fluid {
                    let mut below = y - 1;
                    loop {
                        let under = surface.block([bx, below, bz]);
                        below -= 1;
                        depth += 1;
                        if below <= min_y || !under.fluid {
                            break;
                        }
                    }
                    block.shown
                } else {
                    block.color
                }
            } else {
                COLOR_STONE
            };
            height += f64::from(y) / f64::from(scale * scale);
            match tally.iter_mut().find(|(seen, _)| *seen == color) {
                Some((_, count)) => *count += 1,
                None => tally.push((color, 1)),
            }
        }
    }
    let mut color = 0;
    let mut most = 0;
    for &(seen, count) in &tally {
        if count > most {
            (color, most) = (seen, count);
        }
    }
    (color, depth / (scale * scale), height)
}

pub struct SceneSurface<'a> {
    scene: &'a HandcraftedScene,
    colors: &'a BlockMapColors,
    ceiling: bool,
    fluids: [u8; 2],
}

impl<'a> SceneSurface<'a> {
    pub fn new(scene: &'a HandcraftedScene, colors: &'a BlockMapColors, ceiling: bool) -> Self {
        let fluid = |name: &str| {
            let states = scene.states()?;
            let blocks = &states.registries().blocks;
            let block = blocks.block_by_name(name)?;
            Some(colors.id(blocks.block(block).default_state()))
        };
        Self {
            scene,
            colors,
            ceiling,
            fluids: [
                fluid("minecraft:water").unwrap_or(0),
                fluid("minecraft:lava").unwrap_or(0),
            ],
        }
    }
}

impl MapSurface for SceneSurface<'_> {
    fn ceiling(&self) -> bool {
        self.ceiling
    }

    fn min_y(&self) -> i32 {
        self.scene.vertical_range().start
    }

    fn loaded(&self, x: i32, z: i32) -> bool {
        self.scene.generated_chunk((x >> 4, z >> 4)).is_some()
    }

    fn height(&self, x: i32, z: i32) -> i32 {
        self.scene
            .column_top(x, z, |_| true)
            .unwrap_or(self.min_y() - 1)
    }

    fn block(&self, [x, y, z]: [i32; 3]) -> MapBlock {
        let state = self.scene.state_at((x, y, z)).unwrap_or(BlockStateId::AIR);
        let color = self.colors.id(state);
        let held = self
            .scene
            .states()
            .and_then(|states| Some((states, states.fluid_cell(state)?)));
        let Some((states, cell)) = held else {
            return MapBlock {
                color,
                fluid: false,
                shown: color,
            };
        };
        let blocks = &states.registries().blocks;
        let shown = if blocks.is_face_sturdy(state, Direction::Up, SupportType::Full) {
            color
        } else {
            match cell.kind {
                FluidKind::Water => self.fluids[0],
                FluidKind::Lava => self.fluids[1],
            }
        };
        MapBlock {
            color,
            fluid: true,
            shown,
        }
    }
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

pub fn use_empty_map(
    store: &mut MapStore,
    inventory: &mut Inventory,
    slot: usize,
    (feet, dimension): ([f64; 3], &str),
    infinite: bool,
) -> Option<ItemStack> {
    let held = inventory.slots.get_mut(slot)?;
    let empty = held.as_mut().filter(|stack| stack.id == EMPTY_MAP)?;
    if !infinite {
        empty.count -= 1;
        if empty.count == 0 {
            *held = None;
        }
    }
    let at = (feet[0].floor() as i32, feet[2].floor() as i32);
    let made = store.create(at, 0, dimension, true, false);
    if held.is_none() {
        *held = Some(made);
        return None;
    }
    inventory.add_item(made, slot)
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
    pub hand: bool,
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
            hand: false,
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
    dirty_data: bool,
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
    dirty_decorations: bool,
    tick: i32,
    step: i32,
}

impl Holding {
    fn new(uuid: &PlayerUuid) -> Self {
        Self {
            uuid: uuid.clone(),
            dirty_data: true,
            min_x: 0,
            min_y: 0,
            max_x: MAP_SIZE - 1,
            max_y: MAP_SIZE - 1,
            dirty_decorations: true,
            tick: 0,
            step: 0,
        }
    }

    fn mark_colors_dirty(&mut self, x: usize, y: usize) {
        if self.dirty_data {
            self.min_x = self.min_x.min(x);
            self.min_y = self.min_y.min(y);
            self.max_x = self.max_x.max(x);
            self.max_y = self.max_y.max(y);
        } else {
            self.dirty_data = true;
            (self.min_x, self.min_y, self.max_x, self.max_y) = (x, y, x, y);
        }
    }

    fn patch(&self, data: &MapData) -> MapPatch {
        let width = self.max_x + 1 - self.min_x;
        let height = self.max_y + 1 - self.min_y;
        let mut colors = vec![0; width * height];
        for i in 0..width {
            for j in 0..height {
                colors[i + j * width] = data.colors[self.min_x + i + (self.min_y + j) * MAP_SIZE];
            }
        }
        MapPatch {
            x: self.min_x as u8,
            y: self.min_y as u8,
            width: width as u8,
            height: height as u8,
            colors,
        }
    }

    fn next_update(
        &mut self,
        id: i32,
        data: &MapData,
        decorations: &[(String, Decoration)],
    ) -> Option<MapUpdate> {
        let patch = std::mem::take(&mut self.dirty_data).then(|| self.patch(data));
        let due = self.dirty_decorations && {
            let tick = self.tick;
            self.tick = tick.wrapping_add(1);
            tick % 5 == 0
        };
        let decorations = due.then(|| {
            self.dirty_decorations = false;
            decorations
                .iter()
                .map(|(_, decoration)| MapMark {
                    kind: decoration.kind.clone(),
                    x: decoration.x,
                    y: decoration.y,
                    rot: decoration.rot,
                })
                .collect()
        });
        (patch.is_some() || decorations.is_some()).then_some(MapUpdate {
            id,
            scale: data.scale,
            locked: data.locked,
            decorations,
            patch,
        })
    }
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
            self.decorations_dirty();
        }
    }

    fn remove(&mut self, key: &str) {
        if let Some(index) = self.decorations.iter().position(|(shown, _)| shown == key) {
            let (_, old) = self.decorations.remove(index);
            if tracks_count(&old.kind) {
                self.tracked -= 1;
            }
        }
        self.decorations_dirty();
    }

    fn decorations_dirty(&mut self) {
        for holding in &mut self.holders {
            holding.dirty_decorations = true;
        }
    }

    fn forget_absent(&mut self, carriers: &[Carrier]) {
        let gone: Vec<PlayerUuid> = self
            .holders
            .iter()
            .filter(|holding| !carriers.iter().any(|carrier| carrier.uuid == holding.uuid))
            .map(|holding| holding.uuid.clone())
            .collect();
        for uuid in gone {
            self.holders.retain(|holding| holding.uuid != uuid);
            self.remove(&uuid.0);
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
    pending: BTreeMap<PlayerUuid, Vec<MapUpdate>>,
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
            pending: BTreeMap::new(),
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

    pub fn create(
        &mut self,
        (x, z): (i32, i32),
        scale: u8,
        dimension: &str,
        tracking: bool,
        unlimited: bool,
    ) -> ItemStack {
        let data = MapData::create_fresh(
            f64::from(x),
            f64::from(z),
            scale,
            dimension,
            tracking,
            unlimited,
        );
        let id = self.free_id();
        self.insert(id, data);
        let mut components = serde_json::Map::new();
        components.insert(MAP_ID.to_owned(), Value::from(id));
        let mut stack = ItemStack::new(FILLED_MAP, 1);
        stack.components = Some(Value::Object(components));
        stack
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
        self.tick_carried_on(carriers, game_time, &[]);
    }

    pub fn tick_carried_on(
        &mut self,
        carriers: &[Carrier],
        game_time: i64,
        surfaces: &[(&str, &dyn MapSurface)],
    ) {
        for carrier in carriers {
            for map in &carrier.maps {
                self.tick_carried_by(carrier, map, carriers, game_time);
                if map.hand {
                    self.fill_in(carrier, map.id, surfaces);
                }
            }
            let updates = self.next_updates(carrier);
            if !updates.is_empty() {
                let pending = self.pending.entry(carrier.uuid.clone()).or_default();
                pending.extend(updates);
            }
        }
        for held in self.held.values_mut() {
            held.forget_absent(carriers);
        }
        self.pending
            .retain(|uuid, _| carriers.iter().any(|carrier| carrier.uuid == *uuid));
    }

    pub fn take_updates(&mut self, uuid: &PlayerUuid) -> Vec<MapUpdate> {
        self.pending.remove(uuid).unwrap_or_default()
    }

    pub fn update_color(&mut self, id: i32, x: usize, y: usize, color: u8) -> bool {
        let changed = self
            .maps
            .get(&id)
            .is_some_and(|data| data.colors[x + y * MAP_SIZE] != color);
        if changed {
            self.set_color(id, x, y, color);
        }
        changed
    }

    pub fn set_color(&mut self, id: i32, x: usize, y: usize, color: u8) {
        let Some(data) = self.maps.get_mut(&id) else {
            return;
        };
        data.colors[x + y * MAP_SIZE] = color;
        self.dirty.insert(id);
        if let Some(held) = self.held.get_mut(&id) {
            for holding in &mut held.holders {
                holding.mark_colors_dirty(x, y);
            }
        }
    }

    fn fill_in(&mut self, carrier: &Carrier, id: i32, surfaces: &[(&str, &dyn MapSurface)]) {
        let Some(data) = self.maps.get(&id) else {
            return;
        };
        let surface = surfaces
            .iter()
            .find(|(dimension, _)| *dimension == carrier.dimension);
        let Some(&(_, surface)) = surface.filter(|_| !data.locked) else {
            return;
        };
        if data.dimension != carrier.dimension {
            return;
        }
        let holders = &mut self.held.entry(id).or_default().holders;
        let holding = match holders.iter().position(|h| h.uuid == carrier.uuid) {
            Some(index) => &mut holders[index],
            None => {
                holders.push(Holding::new(&carrier.uuid));
                holders.last_mut().expect("just pushed")
            }
        };
        holding.step = holding.step.wrapping_add(1);
        let painted = data.fill_in((carrier.x, carrier.z), holding.step, surface);
        for (x, y, color) in painted {
            self.set_color(id, x, y, color);
        }
    }

    fn next_updates(&mut self, carrier: &Carrier) -> Vec<MapUpdate> {
        let mut updates = Vec::new();
        for map in &carrier.maps {
            let (Some(data), Some(held)) = (self.maps.get(&map.id), self.held.get_mut(&map.id))
            else {
                continue;
            };
            let Held {
                decorations,
                holders,
                ..
            } = held;
            let update = holders
                .iter_mut()
                .find(|holding| holding.uuid == carrier.uuid)
                .and_then(|holding| holding.next_update(map.id, data, decorations));
            updates.extend(update);
        }
        updates
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
            held.holders.push(Holding::new(&carrier.uuid));
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

#[derive(Clone, Debug, PartialEq)]
pub struct ClientMap {
    pub scale: u8,
    pub locked: bool,
    pub colors: Box<[u8; COLORS]>,
    pub decorations: Vec<MapMark>,
    pub version: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapCache {
    maps: BTreeMap<i32, ClientMap>,
}

impl MapCache {
    pub fn apply(&mut self, update: MapUpdate) {
        let map = self.maps.entry(update.id).or_insert_with(|| ClientMap {
            scale: update.scale,
            locked: update.locked,
            colors: Box::new([0; COLORS]),
            decorations: Vec::new(),
            version: 0,
        });
        if let Some(decorations) = update.decorations {
            map.decorations = decorations;
        }
        if let Some(patch) = update.patch {
            let (width, height) = (usize::from(patch.width), usize::from(patch.height));
            for i in 0..width {
                for j in 0..height {
                    let (x, y) = (usize::from(patch.x) + i, usize::from(patch.y) + j);
                    let inside = x < MAP_SIZE && y < MAP_SIZE;
                    if let Some(&color) = patch.colors.get(i + j * width).filter(|_| inside) {
                        map.colors[x + y * MAP_SIZE] = color;
                    }
                }
            }
        }
        map.version = map.version.wrapping_add(1);
    }

    pub fn get(&self, id: i32) -> Option<&ClientMap> {
        self.maps.get(&id)
    }

    pub fn len(&self) -> usize {
        self.maps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.maps.is_empty()
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
            hand: false,
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

    fn sent(store: &mut MapStore, uuid: &str) -> Vec<MapUpdate> {
        store.take_updates(&PlayerUuid(uuid.to_owned()))
    }

    #[test]
    fn a_holder_gets_the_whole_map_first_and_then_only_the_dirty_rectangle() {
        let mut map = MapData::create_fresh(1000.0, -300.0, 1, "minecraft:overworld", true, true);
        map.colors[5 + 7 * MAP_SIZE] = 9;
        map.colors[COLORS - 1] = 0xfe;
        let mut store = store_with(&[(0, map.clone())]);
        let treasure = carried(
            "minecraft:buried_treasure_map",
            0,
            &[("+", "minecraft:red_x", 1000.0, -300.0)],
        );
        let mut steve = holder("steve", (1000.0, -300.0, 0.0), vec![treasure.clone()]);
        store.tick_carried(std::slice::from_ref(&steve), 0);
        let first = sent(&mut store, "steve");
        assert_eq!(first.len(), 1);
        let update = &first[0];
        assert_eq!((update.id, update.scale, update.locked), (0, 1, false));
        let patch = update.patch.as_ref().expect("the whole map");
        assert_eq!(
            (patch.x, patch.y, patch.width, patch.height),
            (0, 0, 128, 128)
        );
        assert_eq!(patch.colors, map.colors.to_vec());
        let marks: Vec<_> = update
            .decorations
            .as_ref()
            .expect("the marks with it")
            .iter()
            .map(|mark| (mark.kind.as_str(), mark.x, mark.y, mark.rot))
            .collect();
        assert_eq!(
            marks,
            [(PLAYER, -87, -107, 0), ("minecraft:red_x", -87, -107, 8)]
        );
        store.tick_carried(std::slice::from_ref(&steve), 1);
        assert!(sent(&mut store, "steve").is_empty(), "nothing changed");
        steve.x = 1010.0;
        for tick in 2..6 {
            store.tick_carried(std::slice::from_ref(&steve), tick);
            assert!(
                sent(&mut store, "steve").is_empty(),
                "moved markers wait for every fifth try"
            );
        }
        store.tick_carried(std::slice::from_ref(&steve), 6);
        let moved = sent(&mut store, "steve");
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[0].patch, None);
        assert_eq!(
            moved[0].decorations.as_ref().map(|marks| marks[0].x),
            Some(-77)
        );
        store.set_color(0, 5, 7, 20);
        store.set_color(0, 9, 3, 21);
        assert!(
            !store.update_color(0, 9, 3, 21),
            "the same colour changes nothing"
        );
        assert!(store.update_color(0, 6, 4, 22));
        store.tick_carried(std::slice::from_ref(&steve), 7);
        let painted = sent(&mut store, "steve");
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].decorations, None);
        let patch = painted[0].patch.as_ref().expect("a patch");
        assert_eq!((patch.x, patch.y, patch.width, patch.height), (5, 3, 5, 5));
        let mut expected = vec![0; 25];
        expected[4 * 5] = 20;
        expected[4] = 21;
        expected[1 + 5] = 22;
        assert_eq!(patch.colors, expected);
        assert_eq!(
            store.take_writes().maps.len(),
            1,
            "a new colour is saved, a marker is not"
        );
        let alex = holder("alex", (1000.0, -300.0, 0.0), vec![treasure]);
        store.set_color(0, 0, 0, 1);
        store.tick_carried(&[steve.clone(), alex.clone()], 8);
        let alex_first = sent(&mut store, "alex");
        assert_eq!(
            alex_first[0].patch.as_ref().map(|patch| patch.colors.len()),
            Some(COLORS),
            "a new holder gets the whole map"
        );
        assert_eq!(
            sent(&mut store, "steve")[0]
                .patch
                .as_ref()
                .map(|patch| patch.colors.clone()),
            Some(vec![1])
        );
        store.set_color(0, 1, 1, 2);
        store.tick_carried(&[steve.clone(), alex.clone()], 9);
        store.tick_carried(std::slice::from_ref(&alex), 10);
        assert!(
            sent(&mut store, "steve").is_empty(),
            "the updates of a player who is gone are dropped"
        );
        assert_eq!(sent(&mut store, "alex").len(), 1);
    }

    #[test]
    fn a_holder_who_left_gets_the_whole_map_again_when_back() {
        let map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        let mut store = store_with(&[(0, map)]);
        let carried_map = carried(
            "minecraft:filled_map",
            0,
            &[("+", "minecraft:red_x", 0.0, 0.0)],
        );
        let steve = holder("steve", (2.0, 2.0, 0.0), vec![carried_map]);
        store.tick_carried(std::slice::from_ref(&steve), 0);
        assert_eq!(sent(&mut store, "steve").len(), 1);
        store.tick_carried(std::slice::from_ref(&steve), 1);
        assert!(sent(&mut store, "steve").is_empty());
        store.tick_carried(&[], 2);
        let held = store.held(0).expect("the map keeps its target");
        assert_eq!(held.holders().count(), 0, "a player who left holds nothing");
        assert_eq!(held.tracked(), 0);
        assert_eq!(
            shown(&store, 0),
            [("minecraft:red_x", 0, 0, 8)],
            "and leaves no marker behind"
        );
        store.tick_carried(std::slice::from_ref(&steve), 3);
        let back = sent(&mut store, "steve");
        assert_eq!(back.len(), 1);
        let patch = back[0].patch.as_ref().expect("the whole map again");
        assert_eq!(
            (patch.x, patch.y, patch.width, patch.height),
            (0, 0, 128, 128)
        );
        assert_eq!(back[0].decorations.as_ref().map(Vec::len), Some(2));
    }

    #[test]
    fn a_map_cache_takes_whole_maps_patches_and_marks_like_the_vanilla_client() {
        let mut map = MapData::create_fresh(0.0, 0.0, 2, "minecraft:overworld", true, false);
        for (index, color) in map.colors.iter_mut().enumerate() {
            *color = (index % 200) as u8;
        }
        let mut store = store_with(&[(3, map)]);
        let mut steve = holder(
            "steve",
            (100.0, 50.0, 45.0),
            vec![carried("minecraft:filled_map", 3, &[])],
        );
        let mut cache = MapCache::default();
        assert!(cache.is_empty());
        let follow = |store: &mut MapStore, cache: &mut MapCache, steve: &Carrier, tick| {
            store.tick_carried(std::slice::from_ref(steve), tick);
            for update in sent(store, "steve") {
                cache.apply(update);
            }
        };
        follow(&mut store, &mut cache, &steve, 0);
        let shown = cache.get(3).expect("the map");
        assert_eq!((shown.scale, shown.locked, shown.version), (2, false, 1));
        assert_eq!(shown.colors, store.get(3).unwrap().colors);
        assert_eq!(
            shown.decorations,
            [MapMark {
                kind: PLAYER.to_owned(),
                x: -45,
                y: -70,
                rot: 2
            }]
        );
        store.set_color(3, 127, 127, 0x33);
        store.set_color(3, 64, 100, 0x44);
        steve.x = 150.0;
        for tick in 1..6 {
            follow(&mut store, &mut cache, &steve, tick);
        }
        let shown = cache.get(3).unwrap();
        assert_eq!(shown.colors, store.get(3).unwrap().colors);
        assert_eq!(shown.decorations[0].x, -20);
        assert_eq!(cache.len(), 1);
        let mut later = MapUpdate {
            id: 3,
            scale: 4,
            locked: true,
            decorations: Some(Vec::new()),
            patch: Some(MapPatch {
                x: 127,
                y: 0,
                width: 3,
                height: 1,
                colors: vec![1, 2, 3],
            }),
        };
        cache.apply(later.clone());
        let shown = cache.get(3).unwrap();
        assert_eq!(
            (shown.scale, shown.locked),
            (2, false),
            "a known map keeps its scale and lock"
        );
        assert!(shown.decorations.is_empty());
        assert_eq!(shown.colors[127], 1, "a patch is cut at the map's edge");
        later.id = 9;
        later.patch = None;
        cache.apply(later);
        let fresh = cache.get(9).unwrap();
        assert_eq!((fresh.scale, fresh.locked, fresh.version), (4, true, 1));
        assert!(fresh.colors.iter().all(|&color| color == 0));
    }

    type Column = Box<dyn Fn(i32, i32) -> i32>;
    type Cell = Box<dyn Fn([i32; 3]) -> MapBlock>;

    struct Land {
        ceiling: bool,
        top: Column,
        block: Cell,
        loaded: Box<dyn Fn(i32, i32) -> bool>,
    }

    impl MapSurface for Land {
        fn ceiling(&self) -> bool {
            self.ceiling
        }

        fn min_y(&self) -> i32 {
            -64
        }

        fn loaded(&self, x: i32, z: i32) -> bool {
            (self.loaded)(x, z)
        }

        fn height(&self, x: i32, z: i32) -> i32 {
            (self.top)(x, z)
        }

        fn block(&self, pos: [i32; 3]) -> MapBlock {
            (self.block)(pos)
        }
    }

    fn ground(color: u8) -> MapBlock {
        MapBlock {
            color,
            fluid: false,
            shown: color,
        }
    }

    fn water() -> MapBlock {
        MapBlock {
            color: COLOR_WATER,
            fluid: true,
            shown: COLOR_WATER,
        }
    }

    fn land(
        top: impl Fn(i32, i32) -> i32 + 'static,
        block: impl Fn([i32; 3]) -> MapBlock + 'static,
    ) -> Land {
        Land {
            ceiling: false,
            top: Box::new(top),
            block: Box::new(block),
            loaded: Box::new(|_, _| true),
        }
    }

    fn flat(color: impl Fn(i32, i32) -> u8 + 'static) -> Land {
        land(
            |_, _| 64,
            move |[x, y, z]| match y {
                64 => ground(color(x, z)),
                ..64 => ground(COLOR_STONE),
                _ => MapBlock::default(),
            },
        )
    }

    fn paint(map: &mut MapData, painted: &[(usize, usize, u8)]) {
        for &(x, y, color) in painted {
            map.colors[x + y * MAP_SIZE] = color;
        }
    }

    fn pixel(painted: &[(usize, usize, u8)], at: (usize, usize)) -> Option<u8> {
        painted
            .iter()
            .find(|&&(x, y, _)| (x, y) == at)
            .map(|&(_, _, color)| color)
    }

    const GRASS: u8 = 1;
    const SAND: u8 = 2;

    #[test]
    fn a_held_map_fills_in_the_stripe_of_its_step_and_runs_on_while_columns_change() {
        let mut map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        let grass = packed(GRASS, NORMAL);
        let meadow = flat(|_, _| GRASS);
        let painted = map.fill_in((0.5, 0.5), 1, &meadow);
        let columns: BTreeSet<usize> = painted.iter().map(|&(x, _, _)| x).collect();
        assert_eq!(
            columns,
            (1..MAP_SIZE).collect(),
            "column 1 is step 1's and every column after it changes"
        );
        assert_eq!(painted.len(), 127 * MAP_SIZE);
        assert!(painted.iter().all(|&(_, _, color)| color == grass));
        paint(&mut map, &painted);
        assert!(map.fill_in((0.5, 0.5), 2, &meadow).is_empty());
        let first = map.fill_in((0.5, 0.5), 16, &meadow);
        assert_eq!(
            first,
            (0..MAP_SIZE).map(|y| (0, y, grass)).collect::<Vec<_>>(),
            "column 0 waits for step 16"
        );
        paint(&mut map, &first);
        let beaches = flat(|x, _| if x.rem_euclid(4) == 0 { SAND } else { GRASS });
        let sanded = map.fill_in((0.5, 0.5), 4, &beaches);
        let columns: BTreeSet<usize> = sanded.iter().map(|&(x, _, _)| x).collect();
        assert_eq!(
            columns,
            BTreeSet::from([4, 20, 36, 52, 68, 84, 100, 116]),
            "only step 4's stripe changes, and the unchanged column after each stops it"
        );
        assert!(
            sanded
                .iter()
                .all(|&(_, _, color)| color == packed(SAND, NORMAL))
        );
        let mut patchy = flat(|_, _| GRASS);
        patchy.loaded = Box::new(|x, _| x != 0);
        let holes = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false).fill_in(
            (0.5, 0.5),
            0,
            &patchy,
        );
        assert_eq!(pixel(&holes, (63, 5)), Some(grass));
        assert_eq!(
            pixel(&holes, (64, 5)),
            None,
            "unloaded chunks are left blank"
        );
        assert_eq!(
            pixel(&holes, (65, 5)),
            None,
            "a column with nothing changed stops the run"
        );
        assert_eq!(pixel(&holes, (80, 5)), Some(grass), "step 0's next column");
    }

    #[test]
    fn map_pixels_are_shaded_by_water_depth_and_slopes_like_vanilla() {
        let map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        let depth = |x: i32| match x {
            0 => 1,
            1 => 4,
            2 => 5,
            3 => 9,
            4 => 12,
            _ => 0,
        };
        let ramp = |z: i32| match z {
            0..=4 => 64 + z,
            5..=8 => 72 - z,
            9..=12 => 64 + 2 * (z - 8),
            _ => 64,
        };
        let shore = land(
            move |x, z| if x == 10 { ramp(z) } else { 64 },
            move |[x, y, z]| {
                let top = if x == 10 { ramp(z) } else { 64 };
                let deep = depth(x);
                if y > top {
                    MapBlock::default()
                } else if y > top - deep {
                    water()
                } else if y == top - deep && deep > 0 {
                    ground(SAND)
                } else {
                    ground(GRASS)
                }
            },
        );
        let painted = map.fill_in((0.5, 0.5), 0, &shore);
        let at = |x: i32, z: i32| pixel(&painted, ((x + 64) as usize, (z + 64) as usize));
        let water = |brightness| Some(packed(COLOR_WATER, brightness));
        assert_eq!((at(0, 0), at(0, 1)), (water(HIGH), water(HIGH)), "depth 1");
        assert_eq!(
            (at(1, 1), at(1, 0)),
            (water(HIGH), water(NORMAL)),
            "depth 4"
        );
        assert_eq!(
            (at(2, 0), at(2, 1)),
            (water(NORMAL), water(NORMAL)),
            "depth 5"
        );
        assert_eq!((at(3, 1), at(3, 0)), (water(NORMAL), water(LOW)), "depth 9");
        assert_eq!((at(4, 0), at(4, 1)), (water(LOW), water(LOW)), "depth 12");
        let grass = |brightness| Some(packed(GRASS, brightness));
        assert_eq!(
            (at(10, 1), at(10, 2)),
            (grass(HIGH), grass(HIGH)),
            "one block up"
        );
        assert_eq!(
            (at(10, 5), at(10, 6)),
            (grass(LOW), grass(LOW)),
            "one block down"
        );
        assert_eq!(at(10, 10), grass(HIGH), "two blocks up");
        assert_eq!(
            (at(11, 3), at(11, 4)),
            (grass(NORMAL), grass(NORMAL)),
            "flat ground beside the ramp"
        );
    }

    #[test]
    fn a_pixel_takes_its_most_common_block_and_the_first_on_a_tie() {
        let mixed = land(
            |x, _| if x == 1 { 66 } else { 64 },
            |[x, y, z]| match (y, x, z) {
                (65..=66, 1, _) => MapBlock::default(),
                (64, 0, 0) | (64, 1, 1) | (64, 0, 2) | (64, 1, 2) | (64, 0, 3) => ground(GRASS),
                (64, ..) => ground(SAND),
                _ => ground(COLOR_STONE),
            },
        );
        assert_eq!(
            surface_colour(&mixed, (0, 0), 2),
            (GRASS, 0, 64.0),
            "grass, sand, sand, grass: the first wins the tie, and the air above is looked through"
        );
        assert_eq!(
            surface_colour(&mixed, (0, 2), 2).0,
            GRASS,
            "three grass and one sand"
        );
        let lake = land(
            |_, _| 64,
            |[x, y, z]| match y {
                65.. => MapBlock::default(),
                _ if y > 64 - (2 * x + z + 1) => water(),
                _ => ground(SAND),
            },
        );
        assert_eq!(
            surface_colour(&lake, (0, 0), 2),
            (COLOR_WATER, (1 + 2 + 3 + 4) / 4, 64.0),
            "the depth is averaged over the pixel's columns"
        );
        let logged = land(
            |_, _| 64,
            |[_, y, _]| match y {
                64 => MapBlock {
                    color: COLOR_STONE,
                    fluid: true,
                    shown: COLOR_WATER,
                },
                63 => water(),
                ..63 => ground(SAND),
                _ => MapBlock::default(),
            },
        );
        assert_eq!(
            surface_colour(&logged, (0, 0), 1),
            (COLOR_WATER, 2, 64.0),
            "a waterlogged block shows as the water it holds"
        );
        let void = land(|_, _| -65, |_| MapBlock::default());
        assert_eq!(
            surface_colour(&void, (0, 0), 1),
            (COLOR_STONE, 0, -64.0),
            "an empty column shows bedrock at the floor"
        );
        let glass = land(|_, _| 0, |_| MapBlock::default());
        assert_eq!(
            surface_colour(&glass, (0, 0), 1),
            (0, 0, -64.0),
            "no colour all the way down to the floor"
        );
    }

    #[test]
    fn maps_of_dimensions_with_a_ceiling_show_the_vanilla_speckle() {
        assert_eq!(
            [(0, 0), (1, 0), (0, 1), (5, -3), (10, 20), (-1, 0), (0, -1)]
                .map(|(x, z)| speckle(x, z)),
            [
                COLOR_DIRT,
                COLOR_STONE,
                COLOR_STONE,
                COLOR_DIRT,
                COLOR_STONE,
                COLOR_STONE,
                COLOR_DIRT
            ]
        );
        let map = MapData::create_fresh(0.0, 0.0, 0, "minecraft:the_nether", true, false);
        let mut nether = flat(|_, _| GRASS);
        nether.ceiling = true;
        let painted = map.fill_in((0.5, 0.5), 1, &nether);
        let at = |x: i32, z: i32| pixel(&painted, ((x + 64) as usize, (z + 64) as usize));
        assert_eq!(at(0, 0), Some(packed(COLOR_DIRT, NORMAL)));
        assert_eq!(at(1, 0), Some(packed(COLOR_STONE, NORMAL)));
        assert_eq!(
            at(-64, 0),
            None,
            "half the radius: column 0 is out of reach"
        );
        assert_eq!(at(-63, 0), Some(packed(speckle(-63, 0), NORMAL)));
        assert_eq!(at(0, -63), Some(packed(speckle(0, -63), NORMAL)));
        assert_eq!(at(46, 46), None, "outside the circle");
        assert_eq!(at(62, 2), None, "the outer ring keeps only odd pixels");
        assert_eq!(at(62, 3), Some(packed(speckle(62, 3), NORMAL)));
        assert_eq!(at(62, 0), Some(packed(speckle(62, 0), NORMAL)));
    }

    #[test]
    fn a_map_in_hand_fills_in_and_its_holder_gets_the_painted_rectangle() {
        let fresh = MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false);
        let mut store = store_with(&[(0, fresh.clone())]);
        let meadow = flat(|_, _| GRASS);
        let overworld: [(&str, &dyn MapSurface); 1] = [("minecraft:overworld", &meadow)];
        let mut steve = holder(
            "steve",
            (0.5, 0.5, 0.0),
            vec![carried("minecraft:filled_map", 0, &[])],
        );
        store.tick_carried_on(std::slice::from_ref(&steve), 0, &overworld);
        assert!(
            store.get(0).unwrap().colors.iter().all(|&c| c == 0),
            "a map in the backpack does not fill in"
        );
        assert_eq!(sent(&mut store, "steve").len(), 1, "the whole blank map");
        steve.maps[0].hand = true;
        store.tick_carried_on(std::slice::from_ref(&steve), 0, &overworld);
        let colors = &store.get(0).unwrap().colors;
        assert_eq!((colors[0], colors[1]), (0, packed(GRASS, NORMAL)));
        let updates = sent(&mut store, "steve");
        let patch = updates[0].patch.as_ref().expect("the painted part");
        assert_eq!(
            (patch.x, patch.y, patch.width, patch.height),
            (1, 0, 127, 128)
        );
        assert_eq!(store.take_writes().maps.len(), 1, "a painted map is saved");
        for _ in 2..16 {
            store.tick_carried_on(std::slice::from_ref(&steve), 0, &overworld);
        }
        assert_eq!(store.get(0).unwrap().colors[0], 0);
        store.tick_carried_on(std::slice::from_ref(&steve), 0, &overworld);
        assert_eq!(
            store.get(0).unwrap().colors[0],
            packed(GRASS, NORMAL),
            "the sixteenth step reaches column 0"
        );
        let mut elsewhere = fresh.clone();
        elsewhere.dimension = "minecraft:the_nether".to_owned();
        store.insert(1, elsewhere);
        let mut locked = fresh.clone();
        locked.locked = true;
        store.insert(2, locked);
        store.insert(3, fresh.clone());
        let in_hand = |id| CarriedMap {
            hand: true,
            ..carried("minecraft:filled_map", id, &[])
        };
        steve.maps = vec![in_hand(1), in_hand(2)];
        store.tick_carried_on(std::slice::from_ref(&steve), 0, &overworld);
        steve.maps = vec![in_hand(3)];
        store.tick_carried_on(std::slice::from_ref(&steve), 0, &[]);
        for (id, why) in [
            (1, "a map of another dimension"),
            (2, "a locked map"),
            (3, "a map with no loaded level"),
        ] {
            assert!(
                store.get(id).unwrap().colors.iter().all(|&c| c == 0),
                "{why} stays as it is"
            );
        }
    }

    #[test]
    fn a_streamed_scene_shows_maps_its_blocks_fluids_and_loaded_chunks() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        if !jar.is_file() {
            return;
        }
        let registries = Arc::new(registries);
        let colors = BlockMapColors::read_jar(&jar, &registries.blocks).unwrap();
        let id = |text: &str| colors.id(registries.blocks.parse_state(text).unwrap());
        assert_eq!(
            [
                "minecraft:dirt",
                "minecraft:stone",
                "minecraft:bedrock",
                "minecraft:water"
            ]
            .map(id),
            [COLOR_DIRT, COLOR_STONE, COLOR_STONE, COLOR_WATER]
        );
        let states = crate::terrain::BlockStates::new(registries.clone(), 0, -64, 384).unwrap();
        let mut scene = HandcraftedScene::streamed(Arc::new(states));
        scene.insert_chunk(Arc::new(minecraftoss_core::Chunk::new(
            ChunkPos::new(0, 0),
            -64,
            384,
            minecraftoss_core::BiomeId(0),
        )));
        let slab = |half: &str| {
            crate::scene::Block::new("minecraft:stone_slab")
                .with("type", half)
                .with("waterlogged", "true")
        };
        let grass = crate::scene::Block::new("minecraft:grass_block").with("snowy", "false");
        for (pos, block) in [
            ((1, 70, 1), grass.clone()),
            (
                (2, 70, 2),
                crate::scene::Block::new("minecraft:water").with("level", "0"),
            ),
            ((3, 70, 3), slab("bottom")),
            ((4, 70, 4), slab("top")),
            (
                (5, 70, 5),
                crate::scene::Block::new("minecraft:lava").with("level", "0"),
            ),
            ((6, 70, 6), grass),
            ((6, 72, 6), crate::scene::Block::new("minecraft:glass")),
        ] {
            scene.set(pos, Some(block));
        }
        let land = SceneSurface::new(&scene, &colors, false);
        assert!(!land.ceiling() && SceneSurface::new(&scene, &colors, true).ceiling());
        assert_eq!(land.min_y(), -64);
        assert!(land.loaded(0, 0) && land.loaded(15, 15));
        assert!(!land.loaded(16, 0) && !land.loaded(-1, 0));
        assert_eq!(
            (land.height(1, 1), land.height(6, 6), land.height(7, 7)),
            (70, 72, -65),
            "the highest block that is not air"
        );
        let fire = id("minecraft:lava");
        let block = |x, z, y| land.block([x, y, z]);
        let seen = |color, fluid, shown| MapBlock {
            color,
            fluid,
            shown,
        };
        assert_eq!(block(1, 1, 70), seen(GRASS, false, GRASS));
        assert_eq!(block(2, 2, 70), seen(COLOR_WATER, true, COLOR_WATER));
        assert_eq!(
            block(3, 3, 70),
            seen(COLOR_STONE, true, COLOR_WATER),
            "a waterlogged bottom slab shows its water"
        );
        assert_eq!(
            block(4, 4, 70),
            seen(COLOR_STONE, true, COLOR_STONE),
            "a waterlogged top slab is sturdy on top and shows itself"
        );
        assert_eq!(block(5, 5, 70), seen(fire, true, fire));
        assert_eq!(block(6, 6, 72), MapBlock::default(), "glass has no colour");
        assert_eq!(
            block(20, 0, 70),
            MapBlock::default(),
            "nothing is loaded there"
        );
        let map = MapData::create_fresh(8.0, 8.0, 0, "minecraft:overworld", true, false);
        let painted = map.fill_in((8.5, 8.5), 1, &land);
        let at = |x: usize| pixel(&painted, (x + 64, x + 64));
        assert_eq!(
            at(1),
            Some(packed(GRASS, HIGH)),
            "high above the empty floor"
        );
        assert_eq!(at(2), Some(packed(COLOR_WATER, HIGH)));
        assert_eq!(at(3), Some(packed(COLOR_WATER, HIGH)));
        assert_eq!(at(4), Some(packed(COLOR_STONE, HIGH)));
        assert_eq!(at(5), Some(packed(fire, HIGH)));
        assert_eq!(at(6), Some(packed(GRASS, HIGH)), "seen through the glass");
        assert_eq!(at(7), Some(packed(COLOR_STONE, NORMAL)), "bedrock");
        assert_eq!(at(16), None, "the next chunk is not loaded");
    }

    #[test]
    fn an_empty_map_turns_into_a_map_of_where_its_user_stands() {
        let mut store = MapStore::default();
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new(EMPTY_MAP, 3));
        let here = ([1000.7, 64.0, -300.2], "minecraft:overworld");
        let dropped = use_empty_map(&mut store, &mut inventory, 0, here, false);
        assert_eq!(dropped, None);
        let count = |inventory: &Inventory, slot: usize| {
            inventory.slots[slot]
                .as_ref()
                .map(|stack| (stack.id.clone(), stack.count))
        };
        assert_eq!(
            count(&inventory, 0),
            Some((EMPTY_MAP.to_owned(), 2)),
            "one fewer empty map"
        );
        let made = inventory.slots[1].clone().expect("the new map");
        let read = CarriedMap::of(&made).expect("a map item with an id");
        assert_eq!((made.id.as_str(), made.count, read.id), (FILLED_MAP, 1, 0));
        let fresh = MapData::create_fresh(1000.0, -301.0, 0, "minecraft:overworld", true, false);
        assert_eq!(store.get(0), Some(&fresh));
        assert_eq!(
            (fresh.x_center, fresh.z_center, fresh.scale),
            (1024, -256, 0),
            "centred on the grid around the block the user stands on"
        );
        assert!(fresh.tracking_position && !fresh.unlimited_tracking && !fresh.locked);
        let writes = store.take_writes();
        assert_eq!(
            (
                writes.maps.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                writes.last_id
            ),
            (vec![0], Some(0))
        );
        inventory.slots[0] = Some(ItemStack::new(EMPTY_MAP, 1));
        let nether = ([-5.5, 70.0, 3.0], "minecraft:the_nether");
        assert_eq!(
            use_empty_map(&mut store, &mut inventory, 0, nether, false),
            None
        );
        let replaced = inventory.slots[0].clone().expect("the map in hand");
        assert_eq!(
            CarriedMap::of(&replaced).map(|map| map.id),
            Some(1),
            "the last empty map is replaced in the hand"
        );
        assert_eq!(store.get(1).unwrap().dimension, "minecraft:the_nether");
        inventory.slots[0] = Some(ItemStack::new(EMPTY_MAP, 1));
        use_empty_map(&mut store, &mut inventory, 0, here, true);
        assert_eq!(
            count(&inventory, 0),
            Some((EMPTY_MAP.to_owned(), 1)),
            "nothing is used up in creative"
        );
        assert_eq!(store.last_id(), 2);
        for slot in 0..36 {
            if slot != 0 {
                inventory.slots[slot] = Some(ItemStack::new("minecraft:stone", 64));
            }
        }
        inventory.slots[40] = Some(ItemStack::new("minecraft:stone", 64));
        inventory.slots[0] = Some(ItemStack::new(EMPTY_MAP, 2));
        let rest = use_empty_map(&mut store, &mut inventory, 0, here, false)
            .expect("a full inventory drops the new map");
        assert_eq!(CarriedMap::of(&rest).map(|map| map.id), Some(3));
        assert_eq!(
            use_empty_map(&mut store, &mut inventory, 1, here, false),
            None
        );
        assert_eq!(store.last_id(), 3, "only an empty map makes a map");
    }
}
