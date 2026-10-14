//! The integrated server's mobs. Entities generation created (structure
//! mobs and the SPAWN step's creatures, saved as `ProtoChunk.getEntities`
//! keeps them) join the entity world when their chunk arrives, as vanilla
//! loads a FULL chunk's proto entities (`EntityType.loadEntitiesRecursive`),
//! and leave with it. They tick only inside the entity-ticking range.

use crate::item_nbt::{components_tag, json_tag, saved_components, tag_json};
use crate::scene::Block;
use crate::terrain::BlockStates;
use glam::DVec3;
use minecraftoss_entities::zombie::ZombieKind;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{BlockStateId, ChunkPos};
use minecraftoss_entities::chicken::{Chicken, ChickenSoundVariant, ChickenVariant};
use minecraftoss_entities::cow::{Cow, CowSoundVariant, CowVariant};
use minecraftoss_entities::mooshroom::{MushroomCow, MushroomVariant};
use minecraftoss_entities::pig::{Pig, PigSoundVariant, PigVariant};
use minecraftoss_entities::sheep::Sheep;
use minecraftoss_entities::villager::Villager;
use minecraftoss_entities::world::{EntityWorld, Rider};
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_core::block::{flags, FluidKind};
use minecraftoss_core::tags::TagId;
use minecraftoss_core::Registries;
use minecraftoss_player::path_type::PathType;
use minecraftoss_player::{Block as PlayerBlock, World as PlayerWorld};
use std::cell::RefCell;
use minecraftoss_world::level::{update, Level};

/// Per block state, what mobs look up often: the name form (none for air)
/// and the walk path type.
pub struct MobTables {
    pub names: Vec<Option<PlayerBlock>>,
    pub path_types: Vec<PathType>,
    /// `PoiTypes.forState`.
    pub pois: Vec<Option<minecraftoss_entities::poi::PoiType>>,
    pub block_tags: std::sync::Mutex<std::collections::HashMap<String, Option<TagId>>>,
}

impl MobTables {
    pub fn new(level: &Level<'static>, states: &BlockStates) -> Self {
        let registries = level.registries();
        let count = registries.blocks.state_count();
        let tags = PathTags::new(registries);
        let names: Vec<Option<PlayerBlock>> = (0..count).map(|i| name_of(level, states, BlockStateId(i as u16))).collect();
        let pois = names.iter().map(|b| b.as_ref().and_then(minecraftoss_entities::poi::PoiType::of_block)).collect();
        Self {
            names,
            path_types: (0..count).map(|i| path_type_of_state(registries, &tags, BlockStateId(i as u16))).collect(),
            pois,
            block_tags: Default::default(),
        }
    }
}

/// The block tags `getPathTypeFromState` consults.
struct PathTags {
    trapdoors: Option<TagId>,
    speleothems: Option<TagId>,
    fire: Option<TagId>,
    campfires: Option<TagId>,
    fences: Option<TagId>,
    walls: Option<TagId>,
}

impl PathTags {
    fn new(registries: &Registries) -> Self {
        let tag = |name: &str| registries.block_tags.id(name);
        Self {
            trapdoors: tag("minecraft:trapdoors"),
            speleothems: tag("minecraft:speleothems"),
            fire: tag("minecraft:fire"),
            campfires: tag("minecraft:campfires"),
            fences: tag("minecraft:fences"),
            walls: tag("minecraft:walls"),
        }
    }
}

/// `WalkNodeEvaluator.getPathTypeFromState` for a block state.
fn path_type_of_state(registries: &Registries, tags: &PathTags, state: BlockStateId) -> PathType {
    let blocks = &registries.blocks;
    if blocks.is_air(state) {
        return PathType::Open;
    }
    let info = blocks.block(blocks.block_of(state));
    let name = info.name.as_str();
    let is = |tag: Option<TagId>| tag.is_some_and(|t| registries.block_in_tag(state, t));
    let class = |wanted: &str| info.classes().iter().any(|c| &**c == wanted);
    let prop = |key: &str| blocks.property(state, key) == Some("true");
    if is(tags.trapdoors) || name == "minecraft:lily_pad" || name == "minecraft:big_dripleaf" {
        return PathType::Trapdoor;
    }
    match name {
        "minecraft:powder_snow" => return PathType::PowderSnow,
        "minecraft:cactus" | "minecraft:sweet_berry_bush" => return PathType::Damaging,
        "minecraft:honey_block" => return PathType::StickyHoney,
        "minecraft:cocoa" => return PathType::Cocoa,
        _ => {}
    }
    if name == "minecraft:wither_rose" || is(tags.speleothems) {
        return PathType::DamageCautious;
    }
    let fluid = blocks.state(state).fluid.map(|f| f.kind);
    if fluid == Some(FluidKind::Lava) {
        return PathType::Lava;
    }
    // `NodeEvaluator.isBurningBlock`.
    let lit_campfire = is(tags.campfires) && prop("lit");
    if is(tags.fire) || matches!(name, "minecraft:lava" | "minecraft:magma_block" | "minecraft:lava_cauldron") || lit_campfire {
        return PathType::Fire;
    }
    if class("DoorBlock") {
        return if prop("open") {
            PathType::DoorOpen
        } else if name == "minecraft:iron_door" {
            PathType::DoorIronClosed
        } else {
            PathType::DoorWoodClosed
        };
    }
    if class("BaseRailBlock") {
        return PathType::Rail;
    }
    if class("LeavesBlock") {
        return PathType::Leaves;
    }
    if is(tags.fences) || is(tags.walls) || (class("FenceGateBlock") && !prop("open")) {
        return PathType::Fence;
    }
    if !blocks.is(state, flags::PATHFINDABLE_LAND) {
        return PathType::Blocked;
    }
    if fluid == Some(FluidKind::Water) {
        PathType::Water
    } else {
        PathType::Open
    }
}

/// The level as mobs read and write it: blocks by name, air as none, and
/// the catalog's shapes, path types and flags. The level sits in a cell
/// because reading light may solve it.
pub struct MobWorld<'a> {
    pub level: RefCell<&'a mut Level<'static>>,
    pub states: &'a BlockStates,
    pub tables: &'a MobTables,
}

impl MobWorld<'_> {
    fn state(&self, (x, y, z): (i32, i32, i32)) -> BlockStateId {
        self.level.borrow().block(minecraftoss_core::BlockPos::new(x, y, z))
    }
}

/// A level stack's components as JSON: the JSON text the server keeps for
/// stacks it made, or saved NBT.
fn stack_components(stack: &minecraftoss_core::item::ItemStack) -> Option<serde_json::Value> {
    match stack.components.as_ref()? {
        Tag::String(json) => serde_json::from_str(json).ok(),
        tag => Some(tag_json(tag)),
    }
}

fn world_item(id: i32, pos: [f64; 3], stack: &minecraftoss_core::item::ItemStack, pickup_delay: i32) -> minecraftoss_player::WorldItem {
    minecraftoss_player::WorldItem { id, position: DVec3::from_array(pos), item: stack.id.clone(), count: stack.count, components: stack_components(stack), pickup_delay }
}

impl PlayerWorld for MobWorld<'_> {
    fn items_in(&self, min: DVec3, max: DVec3) -> Vec<minecraftoss_player::WorldItem> {
        self.level.borrow().items_touching(min.to_array(), max.to_array()).iter().map(|(id, pos, stack, delay)| world_item(*id, *pos, stack, *delay)).collect()
    }

    fn item(&self, id: i32) -> Option<minecraftoss_player::WorldItem> {
        self.level.borrow().item_entity(id).map(|(pos, stack, delay)| world_item(id, pos, &stack, delay))
    }

    fn take_item(&mut self, id: i32, count: i32) {
        self.level.borrow_mut().take_item(id, count);
    }

    fn spawn_item(&mut self, position: DVec3, stack: &minecraftoss_player::inventory::ItemStack, velocity: DVec3, pickup_delay: i32) {
        let mut level_stack = minecraftoss_core::item::ItemStack::new(&stack.id, i32::from(stack.count));
        level_stack.components = stack.components.as_ref().and_then(components_tag);
        self.level.borrow_mut().spawn_item_with(position.to_array(), level_stack, velocity.to_array(), pickup_delay, 0);
    }

    fn schedule_block_tick(&mut self, (x, y, z): (i32, i32, i32), delay: i32) {
        let pos = minecraftoss_core::BlockPos::new(x, y, z);
        let mut level = self.level.borrow_mut();
        let state = level.block(pos);
        level.schedule_block_tick(pos, state, delay);
    }

    fn trample_farmland(&mut self, (x, y, z): (i32, i32, i32), random: &mut minecraftoss_player::rng::LegacyRandom) {
        let Ok(id) = crate::pack::ResourceId::parse("minecraft:dirt") else { return };
        let Some(dirt) = self.states.state_of(&Block { id, properties: Default::default() }) else { return };
        let mut level = self.level.borrow_mut();
        // The level's own updates break what grew there, drawing from the
        // level random where the entity world has it.
        let gaussian = match &mut level.random {
            minecraftoss_core::random::AnyRandom::Legacy(r) => {
                let gaussian = r.gaussian_cache();
                r.set_seed((random.raw_state() ^ 0x5DEE_CE66D) as i64);
                Some(gaussian)
            }
            _ => None,
        };
        level.set_block(minecraftoss_core::BlockPos::new(x, y, z), dirt, update::ALL, update::LIMIT);
        if let (Some(gaussian), minecraftoss_core::random::AnyRandom::Legacy(r)) = (gaussian, &mut level.random) {
            *random = minecraftoss_player::rng::LegacyRandom::from_raw_state(r.state());
            r.set_gaussian_cache(gaussian);
        }
    }

    fn spawn_popped_item(&mut self, position: DVec3, stack: &minecraftoss_player::inventory::ItemStack) {
        let mut level_stack = minecraftoss_core::item::ItemStack::new(&stack.id, i32::from(stack.count));
        level_stack.components = stack.components.as_ref().and_then(components_tag);
        self.level.borrow_mut().spawn_popped_item(position.to_array(), level_stack);
    }

    fn block_drops(&self, (x, y, z): (i32, i32, i32), block: &PlayerBlock) -> Vec<minecraftoss_player::inventory::ItemStack> {
        let Ok(id) = crate::pack::ResourceId::parse(&block.id) else { return Vec::new() };
        let Some(state) = self.states.state_of(&Block { id, properties: block.properties.clone() }) else { return Vec::new() };
        let mut level = self.level.borrow_mut();
        let drops = level.mob_block_drops(state, minecraftoss_core::BlockPos::new(x, y, z));
        drops
            .into_iter()
            .map(|s| {
                let max = level.lib.registries.items.max_stack(&s.id).clamp(1, 99) as u8;
                minecraftoss_player::inventory::ItemStack { id: s.id.clone(), count: s.count.clamp(0, 255) as u8, max, components: stack_components(&s) }
            })
            .collect()
    }

    fn biome(&self, (x, y, z): (i32, i32, i32)) -> Option<String> {
        let level = self.level.borrow();
        let biome = minecraftoss_generator::feature::World::biome(&**level, x, y, z)?;
        Some(level.lib.registries.biomes.get(biome).name.as_str().to_owned())
    }

    fn block(&self, pos: (i32, i32, i32)) -> Option<PlayerBlock> {
        self.tables.names.get(usize::from(self.state(pos).0)).cloned().flatten()
    }

    fn set_block(&mut self, (x, y, z): (i32, i32, i32), block: Option<PlayerBlock>) {
        let state = match block {
            None => BlockStateId::AIR,
            Some(block) => {
                let Ok(id) = crate::pack::ResourceId::parse(&block.id) else { return };
                let Some(state) = self.states.state_of(&Block { id, properties: block.properties }) else { return };
                state
            }
        };
        self.level.borrow_mut().set_block(minecraftoss_core::BlockPos::new(x, y, z), state, update::ALL, update::LIMIT);
    }

    fn block_in_tag(&self, pos: (i32, i32, i32), tag: &str) -> bool {
        let state = self.state(pos);
        let level = self.level.borrow();
        let registries = level.registries();
        let cached = self.tables.block_tags.lock().ok().and_then(|tags| tags.get(tag).copied());
        let id = cached.unwrap_or_else(|| {
            let id = registries.block_tags.id(tag);
            if let Ok(mut tags) = self.tables.block_tags.lock() {
                tags.insert(tag.to_owned(), id);
            }
            id
        });
        match id {
            Some(id) => registries.block_in_tag(state, id),
            None => self.block(pos).is_some_and(|block| minecraftoss_player::authored_block_in_tag(&block, tag, &self.collision_boxes(pos))),
        }
    }

    fn collision_boxes(&self, pos: (i32, i32, i32)) -> Vec<[f64; 6]> {
        let state = self.state(pos);
        self.level.borrow().registries().blocks.collision_boxes(state)
    }

    fn path_type_from_state(&self, pos: (i32, i32, i32)) -> PathType {
        self.tables.path_types.get(usize::from(self.state(pos).0)).copied().unwrap_or(PathType::Blocked)
    }

    fn floatable_fluid(&self, pos: (i32, i32, i32)) -> bool {
        let state = self.state(pos);
        self.level.borrow().registries().blocks.state(state).fluid.is_some_and(|f| f.kind == FluidKind::Water)
    }

    fn pathfindable(&self, pos: (i32, i32, i32)) -> bool {
        let state = self.state(pos);
        self.level.borrow().registries().blocks.is(state, flags::PATHFINDABLE_LAND)
    }

    fn solid(&self, pos: (i32, i32, i32)) -> bool {
        let state = self.state(pos);
        self.level.borrow().registries().blocks.is(state, flags::LEGACY_SOLID)
    }

    /// `BlockState.isSuffocating`, from the catalog.
    fn suffocating(&self, pos: (i32, i32, i32)) -> bool {
        let state = self.state(pos);
        self.level.borrow().registries().blocks.state(state).suffocating
    }

    fn solid_render(&self, pos: (i32, i32, i32)) -> bool {
        let state = self.state(pos);
        self.level.borrow().registries().blocks.is(state, flags::SOLID_RENDER)
    }

    fn difficulty_inputs(&self, (x, _, z): (i32, i32, i32)) -> (i64, i64, f32) {
        self.level.borrow().difficulty_inputs(x, z)
    }

    fn sturdy_underside(&self, pos: (i32, i32, i32)) -> bool {
        let state = self.state(pos);
        self.level.borrow().registries().blocks.is_face_sturdy(state, minecraftoss_core::pos::Direction::Down, minecraftoss_core::block::SupportType::Full)
    }

    /// The block's fall sound: the catalog's sound type, else the step's
    /// family.
    fn fall_sound(&self, pos: (i32, i32, i32)) -> Option<(String, f32, f32)> {
        let state = self.state(pos);
        if let Some(sound) = self.level.borrow().registries().blocks.sound_type(state) {
            return Some((sound.fall.to_string(), sound.volume, sound.pitch));
        }
        let (step, volume, pitch) = self.step_sound(pos)?;
        Some((format!("{}.fall", step.strip_suffix(".step")?), volume, pitch))
    }

    /// The block's step sound: the catalog's sound type, else its measured
    /// sound type, else the family its name suggests at volume and pitch 1.
    fn step_sound(&self, pos: (i32, i32, i32)) -> Option<(String, f32, f32)> {
        let state = self.state(pos);
        if let Some(sound) = self.level.borrow().registries().blocks.sound_type(state) {
            return Some((sound.step.to_string(), sound.volume, sound.pitch));
        }
        let block = self.block(pos)?;
        let (family, volume, pitch) = match crate::audio::measured_block_sound_profile(&block.id) {
            Some(profile) => (profile.family, profile.volume, profile.pitch),
            None => (crate::audio::block_sound_family(&block.id), 1.0, 1.0),
        };
        Some((format!("block.{family}.step"), volume, pitch))
    }

    /// `getLightLevelDependentMagicValue - 0.5`: the raw brightness (sky
    /// light less the sky's darkening, or block light) on vanilla's curve,
    /// lifted by the dimension's ambient light.
    fn can_see_sky(&self, (x, y, z): (i32, i32, i32)) -> bool {
        self.level.borrow_mut().sky_light(minecraftoss_core::BlockPos::new(x, y, z)) >= 15
    }

    fn rain_at(&self, (x, y, z): (i32, i32, i32)) -> bool {
        self.level.borrow_mut().is_raining_at(minecraftoss_core::BlockPos::new(x, y, z))
    }

    fn snow_golem_melts(&self, (x, y, z): (i32, i32, i32)) -> bool {
        self.level.borrow().snow_golem_melts_at(x, y, z)
    }

    fn block_survives(&self, (x, y, z): (i32, i32, i32), block: &PlayerBlock) -> bool {
        let Ok(id) = crate::pack::ResourceId::parse(&block.id) else { return false };
        let Some(state) = self.states.state_of(&Block { id, properties: block.properties.clone() }) else { return false };
        self.level.borrow().can_survive_at(state, minecraftoss_core::BlockPos::new(x, y, z))
    }

    fn light_path_cost(&self, (x, y, z): (i32, i32, i32)) -> f32 {
        let mut level = self.level.borrow_mut();
        let brightness = level.max_local_raw_brightness(minecraftoss_core::BlockPos::new(x, y, z));
        let v = brightness as f32 / 15.0;
        let curved = v / (4.0 - 3.0 * v);
        // `Mth.lerp(ambientLight, curvedV, 1)`.
        let magic = curved + level.ambient_light() * (1.0 - curved);
        magic - 0.5
    }

    fn min_y(&self) -> i32 {
        self.level.borrow().min_y()
    }

    fn max_y(&self) -> i32 {
        self.level.borrow().max_y()
    }

    fn motion_blocking_no_leaves_height(&self, x: i32, z: i32) -> i32 {
        let level = self.level.borrow();
        minecraftoss_generator::feature::World::height_at(&**level, minecraftoss_core::chunk::HeightmapKind::MotionBlockingNoLeaves, x, z)
    }
}

fn name_of(level: &Level<'static>, states: &BlockStates, state: BlockStateId) -> Option<PlayerBlock> {
    if level.registries().blocks.is_air(state) {
        return None;
    }
    states.block(state).map(|b| PlayerBlock { id: b.id.key(), properties: b.properties.clone() })
}



fn position(tag: &Tag) -> Option<DVec3> {
    let pos = tag.get("Pos")?.as_list()?;
    Some(DVec3::new(pos.first()?.as_f64()?, pos.get(1)?.as_f64()?, pos.get(2)?.as_f64()?))
}

fn yaw(tag: &Tag) -> f32 {
    tag.get("Rotation").and_then(Tag::as_list).and_then(|r| r.first()).and_then(Tag::as_f64).unwrap_or(0.0) as f32
}

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|v| v as i32)
}

fn text<'a>(tag: &'a Tag, key: &str) -> Option<&'a str> {
    tag.get(key).and_then(Tag::as_str)
}

/// A seed for the entity's own random from its UUID: vanilla seeds entity
/// randoms from the clock, so any seed is as good; this one repeats.
fn seed(tag: &Tag) -> u64 {
    let uuid = tag.get("UUID").and_then(Tag::as_ints).unwrap_or_default();
    uuid.iter().fold(0x9E37_79B9_7F4A_7C15u64, |h, &w| (h ^ u64::from(w as u32)).wrapping_mul(0x100_0000_01B3))
}

pub const SIMULATED_KINDS: &[&str] = &[
    "minecraft:bat",
    "minecraft:blaze",
    "minecraft:bogged",
    "minecraft:cat",
    "minecraft:cave_spider",
    "minecraft:chicken",
    "minecraft:cow",
    "minecraft:creeper",
    "minecraft:donkey",
    "minecraft:drowned",
    "minecraft:end_crystal",
    "minecraft:ender_dragon",
    "minecraft:enderman",
    "minecraft:evoker",
    "minecraft:ghast",
    "minecraft:glow_item_frame",
    "minecraft:hoglin",
    "minecraft:horse",
    "minecraft:husk",
    "minecraft:iron_golem",
    "minecraft:item_frame",
    "minecraft:llama",
    "minecraft:magma_cube",
    "minecraft:mooshroom",
    "minecraft:mule",
    "minecraft:ocelot",
    "minecraft:parched",
    "minecraft:pig",
    "minecraft:piglin",
    "minecraft:piglin_brute",
    "minecraft:pillager",
    "minecraft:sheep",
    "minecraft:shulker",
    "minecraft:skeleton",
    "minecraft:slime",
    "minecraft:snow_golem",
    "minecraft:spider",
    "minecraft:stray",
    "minecraft:strider",
    "minecraft:trader_llama",
    "minecraft:vex",
    "minecraft:villager",
    "minecraft:vindicator",
    "minecraft:wandering_trader",
    "minecraft:witch",
    "minecraft:wither_skeleton",
    "minecraft:wolf",
    "minecraft:zoglin",
    "minecraft:zombie",
    "minecraft:zombie_villager",
    "minecraft:zombified_piglin",
];

const VEHICLES: &[&str] = &["minecraft:strider", "minecraft:horse", "minecraft:donkey", "minecraft:mule", "minecraft:llama", "minecraft:trader_llama"];

fn carries(tag: &Tag) -> bool {
    text(tag, "id").is_some_and(|id| VEHICLES.contains(&id))
        && tag.get("Passengers").and_then(Tag::as_list).is_some_and(|riders| {
            riders.iter().all(|rider| {
                let kind = text(rider, "id").unwrap_or("");
                SIMULATED_KINDS.contains(&kind) && !matches!(kind, "minecraft:end_crystal" | "minecraft:ender_dragon") && (rider.get("Passengers").is_none() || carries(rider))
            })
        })
}

pub fn spawn_saved_all(world: &mut EntityWorld, tag: &Tag) -> Vec<(u64, Tag)> {
    let (Tag::Compound(map), true) = (tag, carries(tag)) else {
        return spawn_saved(world, tag).into_iter().map(|id| (id, tag.clone())).collect();
    };
    let mut bare = map.clone();
    let riders = bare.remove("Passengers").and_then(|riders| riders.as_list().map(<[Tag]>::to_vec)).unwrap_or_else(Vec::new);
    let Some(id) = spawn_saved(world, &Tag::Compound(bare)) else { return Vec::new() };
    let mut out = vec![(id, tag.clone())];
    for rider in &riders {
        let spawned = spawn_saved_all(world, rider);
        if let Some(&(rider_id, _)) = spawned.first() {
            world.force_riding(Rider::Mob(rider_id), id);
        }
        out.extend(spawned);
    }
    out
}

pub fn spawn_unmounted(world: &mut EntityWorld, tag: &Tag) -> Vec<(u64, Tag)> {
    let Tag::Compound(map) = tag else { return Vec::new() };
    if !map.contains_key("Passengers") || carries(tag) {
        return spawn_saved_all(world, tag);
    }
    let mut bare = map.clone();
    let riders = bare.remove("Passengers").and_then(|riders| riders.as_list().map(<[Tag]>::to_vec)).unwrap_or_else(Vec::new);
    let bare = Tag::Compound(bare);
    let Some(id) = spawn_saved(world, &bare) else { return Vec::new() };
    let mut out = vec![(id, bare)];
    for rider in &riders {
        out.extend(spawn_unmounted(world, rider));
    }
    out
}

fn nest_passengers(world: &EntityWorld, out: &mut Vec<(u64, Tag)>) {
    fn build(world: &EntityWorld, tags: &mut std::collections::HashMap<u64, Tag>, id: u64) -> Option<Tag> {
        let mut tag = tags.remove(&id)?;
        let riders: Vec<Tag> = world
            .passengers_of(id)
            .iter()
            .filter_map(|rider| match rider {
                Rider::Mob(rider) => build(world, tags, *rider),
                Rider::Player(_) => None,
            })
            .collect();
        if let Tag::Compound(map) = &mut tag {
            if riders.is_empty() {
                map.remove("Passengers");
            } else {
                map.insert("Passengers".to_owned(), Tag::List(riders));
            }
        }
        Some(tag)
    }
    let order: Vec<u64> = out.iter().map(|(id, _)| *id).collect();
    let present: std::collections::HashSet<u64> = order.iter().copied().collect();
    let mut tags: std::collections::HashMap<u64, Tag> = std::mem::take(out).into_iter().collect();
    for id in order {
        if world.vehicle_of(Rider::Mob(id)).is_some_and(|vehicle| present.contains(&vehicle)) {
            continue;
        }
        if let Some(tag) = build(world, &mut tags, id) {
            out.push((id, tag));
        }
    }
}

/// Adds a saved entity the entity world simulates; its ID, or none for a
/// type it does not simulate yet.
pub fn spawn_saved(world: &mut EntityWorld, tag: &Tag) -> Option<u64> {
    let kind = text(tag, "id")?;
    let pos = position(tag)?;
    let yaw = yaw(tag);
    let age = int(tag, "Age").unwrap_or(0);
    let health = tag.get("Health").and_then(Tag::as_f64).map(|h| h as f32);
    let persistent = int(tag, "PersistenceRequired").unwrap_or(0) != 0;
    let random = LegacyRandom::new(seed(tag));
    let variant = text(tag, "variant").unwrap_or("minecraft:temperate");
    let fire = tag.get("Fire").and_then(Tag::as_i64).map_or(0, |f| f as i32);
    // `Mob.readAdditionalSaveData`: a mob without AI stands where it is.
    let no_ai = int(tag, "NoAI").unwrap_or(0) != 0;
    let id = match kind {
        "minecraft:cow" | "minecraft:mooshroom" => {
            let mut cow = Cow::new(pos);
            cow.yaw = yaw;
            cow.age.set(age);
            cow.persistence_required = persistent;
            if let Some(health) = health {
                cow.health = health;
            }
            cow.variant = match variant {
                "minecraft:warm" => CowVariant::Warm,
                "minecraft:cold" => CowVariant::Cold,
                _ => CowVariant::Temperate,
            };
            cow.sound_variant = if text(tag, "sound_variant") == Some("minecraft:moody") { CowSoundVariant::Moody } else { CowSoundVariant::Classic };
            let id = if kind == "minecraft:mooshroom" {
                let mushroom = if text(tag, "Type") == Some("brown") { MushroomVariant::Brown } else { MushroomVariant::Red };
                let mut mooshroom = MushroomCow::new(pos, mushroom);
                mooshroom.cow = cow;
                world.spawn_mooshroom(mooshroom, no_ai)
            } else {
                world.spawn_cow(cow, no_ai)
            };
            world.cow_mut(id)?.random = random;
            id
        }
        "minecraft:horse" | "minecraft:donkey" | "minecraft:mule" | "minecraft:llama" | "minecraft:trader_llama" if tag.get("Passengers").is_none() => {
            use minecraftoss_entities::horse::{HorseKind, HorseState};
            let mut cow = Cow::new(pos);
            cow.yaw = yaw;
            cow.age.set(age);
            cow.persistence_required = persistent;
            if let Some(health) = health {
                cow.health = health;
            }
            // `AbstractHorse.readAdditionalSaveData` (and `Horse`'s variant,
            // `AbstractChestedHorse`'s chest); its attributes as saved.
            let mut horse = HorseState::new(HorseKind::from_type_id(kind)?);
            horse.variant = int(tag, "Variant").unwrap_or(0);
            if horse.kind.is_llama() {
                horse.variant = horse.variant.clamp(0, 3);
                horse.set_strength(int(tag, "Strength").unwrap_or(0));
                horse.despawn_delay = int(tag, "DespawnDelay").unwrap_or(minecraftoss_entities::horse::TRADER_DESPAWN_DELAY);
            }
            horse.body_item = tag.get("equipment").and_then(|e| e.get("body")).and_then(|b| b.get("id")).and_then(Tag::as_str).map(str::to_owned);
            horse.saddled = horse.kind.saddleable() && tag.get("equipment").and_then(|e| e.get("saddle")).and_then(|b| b.get("id")).and_then(Tag::as_str) == Some("minecraft:saddle");
            horse.owner = tag.get("Owner").and_then(Tag::as_ints).and_then(|ints| <[i32; 4]>::try_from(ints).ok()).map(minecraftoss_entities::gossip::uuid_from_ints);
            horse.chest = int(tag, "ChestedHorse").unwrap_or(0) != 0;
            horse.tame = int(tag, "Tame").unwrap_or(0) != 0;
            horse.bred = int(tag, "Bred").unwrap_or(0) != 0;
            horse.temper = int(tag, "Temper").unwrap_or(0);
            horse.eating = int(tag, "EatingHaystack").unwrap_or(0) != 0;
            if let Some(max_health) = attribute_base(tag, "minecraft:max_health") {
                horse.max_health = max_health as f32;
            }
            if let Some(speed) = attribute_base(tag, "minecraft:movement_speed") {
                horse.movement_speed = speed;
            }
            if let Some(jump) = attribute_base(tag, "minecraft:jump_strength") {
                horse.jump_strength = jump;
            }
            cow.health = health.unwrap_or(horse.max_health).min(horse.max_health);
            horse.resize_inventory();
            if horse.chest {
                for saved in tag.get("Items").and_then(Tag::as_list).into_iter().flatten() {
                    let Some(item) = text(saved, "id") else { continue };
                    let Some(slot) = int(saved, "Slot").and_then(|slot| usize::try_from(slot).ok()).filter(|&slot| slot < horse.inventory.len()) else { continue };
                    let max = world.item_max_stack(item).clamp(1, 99) as u8;
                    let count = int(saved, "count").unwrap_or(1).clamp(1, i32::from(max)) as u8;
                    let components = saved.get("components").map(tag_json);
                    horse.inventory[slot] = Some(minecraftoss_player::inventory::ItemStack { id: item.to_owned(), count, max, components });
                }
            }
            let id = world.spawn_horse(cow, horse, no_ai);
            world.cow_mut(id)?.random = random;
            id
        }
        "minecraft:pig" => {
            let mut pig = Pig::new(pos);
            pig.yaw = yaw;
            pig.age.set(age);
            pig.persistence_required = persistent;
            if let Some(health) = health {
                pig.health = health;
            }
            pig.variant = match variant {
                "minecraft:warm" => PigVariant::Warm,
                "minecraft:cold" => PigVariant::Cold,
                _ => PigVariant::Temperate,
            };
            pig.sound_variant = match text(tag, "sound_variant") {
                Some("minecraft:mini") => PigSoundVariant::Mini,
                Some("minecraft:big") => PigSoundVariant::Big,
                _ => PigSoundVariant::Classic,
            };
            // `Mob.equipment`'s saddle slot.
            pig.saddled = tag.get("equipment").and_then(|e| e.get("saddle")).and_then(|s| s.get("id")).and_then(Tag::as_str) == Some("minecraft:saddle");
            let id = world.spawn_pig(pig, no_ai);
            world.pig_mut(id)?.random = random;
            id
        }
        "minecraft:strider" if tag.get("Passengers").is_none() => {
            let mut strider = minecraftoss_entities::strider::Strider::new(pos);
            strider.yaw = yaw;
            strider.age.set(age);
            strider.in_love = int(tag, "InLove").unwrap_or(0);
            strider.persistence_required = persistent;
            if let Some(health) = health {
                strider.health = health;
            }
            strider.saddled = tag.get("equipment").and_then(|e| e.get("saddle")).and_then(|s| s.get("id")).and_then(Tag::as_str) == Some("minecraft:saddle");
            let id = world.spawn_strider(strider, no_ai);
            let entity = world.strider_mut(id)?;
            entity.random = random;
            entity.look_control.head_yaw = yaw;
            entity.body_rotation.body_yaw = yaw;
            id
        }
        "minecraft:piglin" | "minecraft:piglin_brute" if tag.get("Passengers").is_none() => {
            let kind = minecraftoss_entities::piglin::PiglinKind::from_type(kind)?;
            let saved = crate::piglin_save::Saved { pos, yaw, health, persistent, no_ai, random };
            crate::piglin_save::spawn(world, tag, kind, saved)?
        }
        "minecraft:pillager" | "minecraft:vindicator" | "minecraft:evoker" if tag.get("Passengers").is_none() => {
            let kind = minecraftoss_entities::illager::IllagerKind::from_type(kind)?;
            let saved = crate::illager_save::Saved { pos, yaw, health, persistent, no_ai, random };
            crate::illager_save::spawn(world, tag, kind, saved)?
        }
        "minecraft:vex" if tag.get("Passengers").is_none() => {
            let saved = crate::vex_save::Saved { pos, yaw, health, persistent, no_ai, random };
            crate::vex_save::spawn(world, tag, saved)?
        }
        "minecraft:hoglin" | "minecraft:zoglin" => {
            use minecraftoss_entities::hoglin::{Hoglin, HoglinKind};
            let kind = HoglinKind::from_type(kind)?;
            let mut hoglin = Hoglin::new(kind, pos);
            hoglin.yaw = yaw;
            hoglin.persistence_required = persistent;
            if let Some(health) = health {
                hoglin.health = health;
            }
            match kind {
                HoglinKind::Hoglin => {
                    hoglin.age.set(age);
                    hoglin.age.forced = int(tag, "ForcedAge").unwrap_or(0);
                    hoglin.in_love = int(tag, "InLove").unwrap_or(0);
                    hoglin.immune_to_zombification = int(tag, "IsImmuneToZombification").unwrap_or(0) != 0;
                    hoglin.time_in_overworld = int(tag, "TimeInOverworld").unwrap_or(0);
                    hoglin.cannot_be_hunted = int(tag, "CannotBeHunted").unwrap_or(0) != 0;
                }
                HoglinKind::Zoglin => hoglin.baby = int(tag, "IsBaby").unwrap_or(0) != 0,
            }
            let id = world.spawn_hoglin(hoglin, no_ai);
            let entity = world.hoglin_mut(id)?;
            entity.random = random;
            entity.look_control.head_yaw = yaw;
            entity.body_rotation.body_yaw = yaw;
            id
        }
        "minecraft:chicken" => {
            let mut chicken = Chicken::new(pos);
            chicken.yaw = yaw;
            chicken.age.set(age);
            chicken.persistence_required = persistent;
            if let Some(health) = health {
                chicken.health = health;
            }
            chicken.variant = match variant {
                "minecraft:warm" => ChickenVariant::Warm,
                "minecraft:cold" => ChickenVariant::Cold,
                _ => ChickenVariant::Temperate,
            };
            chicken.sound_variant = if text(tag, "sound_variant") == Some("minecraft:picky") { ChickenSoundVariant::Picky } else { ChickenSoundVariant::Classic };
            if let Some(egg_time) = int(tag, "EggLayTime") {
                chicken.egg_time = egg_time;
            }
            let id = world.spawn_chicken(chicken, no_ai);
            world.chicken_mut(id)?.random = random;
            id
        }
        "minecraft:sheep" => {
            let mut sheep = Sheep::default();
            sheep.age.set(age);
            sheep.persistence_required = persistent;
            sheep.wool.set_color((int(tag, "Color").unwrap_or(0) & 15) as u8);
            sheep.wool.set_sheared(int(tag, "Sheared").unwrap_or(0) != 0);
            let id = world.spawn_sheep(sheep, pos, no_ai);
            let entity = world.sheep_mut(id)?;
            entity.random = random;
            // `Entity.load`: the head and body face the saved yaw too.
            entity.yaw = yaw;
            entity.previous_yaw = yaw;
            entity.look_control.head_yaw = yaw;
            entity.body_rotation.body_yaw = yaw;
            if let Some(health) = health {
                entity.health = health;
            }
            id
        }
        // Monsters and bats natural spawning makes. Riders (jockeys) wait
        // dormant with their vehicle.
        "minecraft:zombie" | "minecraft:husk" | "minecraft:drowned" | "minecraft:zombie_villager" | "minecraft:zombified_piglin" if tag.get("Passengers").is_none() => {
            let mut zombie = minecraftoss_entities::zombie::Zombie::new(pos);
            zombie.kind = match kind {
                "minecraft:husk" => ZombieKind::Husk,
                "minecraft:drowned" => ZombieKind::Drowned,
                "minecraft:zombie_villager" => ZombieKind::ZombieVillager,
                "minecraft:zombified_piglin" => ZombieKind::ZombifiedPiglin,
                _ => ZombieKind::Zombie,
            };
            if zombie.kind == ZombieKind::Drowned {
                zombie.body.step_height = 1.0;
            }
            if zombie.kind == ZombieKind::ZombieVillager {
                let data = tag.get("VillagerData");
                let field = |key: &str, default: &str| data.and_then(|d| text(d, key)).unwrap_or(default).to_owned();
                zombie.villager = Some((field("type", "minecraft:plains"), field("profession", "minecraft:none")));
                zombie.villager_xp = int(tag, "Xp").unwrap_or(0);
                if let Some(time) = int(tag, "ConversionTime").filter(|time| *time != -1) {
                    zombie.villager_conversion_time = time.max(0);
                    zombie.conversion_starter = tag.get("ConversionPlayer").and_then(Tag::as_ints).filter(|i| i.len() == 4).map(|i| minecraftoss_entities::gossip::uuid_from_ints([i[0], i[1], i[2], i[3]]));
                }
            }
            zombie.head_item = head_item(tag);
            zombie.main_hand = tag.get("equipment").and_then(|e| e.get("mainhand")).and_then(|i| i.get("id")).and_then(Tag::as_str).map(str::to_owned);
            zombie.off_hand = tag.get("equipment").and_then(|e| e.get("offhand")).and_then(|i| i.get("id")).and_then(Tag::as_str).map(str::to_owned);
            for (chance, slot) in zombie.hand_drop_chances.iter_mut().zip(["mainhand", "offhand"]) {
                if let Some(saved) = tag.get("drop_chances").and_then(|d| d.get(slot)).and_then(Tag::as_f64) {
                    *chance = saved as f32;
                }
            }
            zombie.armor = armor_slots(tag);
            zombie.set_baby(int(tag, "IsBaby").unwrap_or(0) != 0);
            zombie.can_break_doors = int(tag, "CanBreakDoors").unwrap_or(0) != 0;
            zombie.persistence_required = persistent;
            if let Some(health) = health {
                zombie.health = health;
            }
            let id = if no_ai { world.spawn_zombie(zombie, true) } else if zombie.kind == ZombieKind::Drowned { world.spawn_drowned(zombie) } else { world.spawn_zombie_active(zombie, yaw) };
            let entity = world.zombie_mut(id)?;
            entity.random = random;
            // `Entity.load` turns the body and head to the saved yaw.
            entity.yaw = yaw;
            entity.body_rotation.body_yaw = yaw;
            entity.look_control.head_yaw = yaw;
            if let (Some(range), Some(ai)) = (attribute_value(tag, "minecraft:follow_range"), entity.ai.as_deref_mut()) {
                ai.state.follow_range = range;
            }
            if let (Some(end), Some(ai)) = (tag.get("anger_end_time").and_then(Tag::as_i64), entity.ai.as_deref_mut()) {
                ai.state.enderman.anger_end_time = end;
            }
            id
        }
        "minecraft:skeleton" | "minecraft:stray" | "minecraft:bogged" | "minecraft:parched" | "minecraft:wither_skeleton" if tag.get("Passengers").is_none() => {
            use minecraftoss_entities::skeleton::SkeletonKind;
            let kind = match kind {
                "minecraft:stray" => SkeletonKind::Stray,
                "minecraft:bogged" => SkeletonKind::Bogged,
                "minecraft:parched" => SkeletonKind::Parched,
                "minecraft:wither_skeleton" => SkeletonKind::WitherSkeleton,
                _ => SkeletonKind::Skeleton,
            };
            let mut skeleton = minecraftoss_entities::skeleton::Skeleton::of_kind(kind, pos);
            // `Bogged.readAdditionalSaveData`.
            skeleton.sheared = int(tag, "sheared").unwrap_or(0) != 0;
            skeleton.head_item = head_item(tag);
            // Its bow is the main hand's (a summoned skeleton has none; a
            // natural spawn's `finalizeSpawn` gave it one).
            skeleton.holds_bow = tag.get("equipment").and_then(|e| e.get("mainhand")).and_then(|i| i.get("id")).and_then(Tag::as_str) == Some("minecraft:bow");
            skeleton.main_hand = tag.get("equipment").and_then(|e| e.get("mainhand")).and_then(|i| i.get("id")).and_then(Tag::as_str).filter(|item| *item != "minecraft:bow").map(str::to_owned);
            skeleton.armor = armor_slots(tag);
            skeleton.persistence_required = persistent;
            if let Some(health) = health {
                skeleton.health = health;
            }
            let id = if no_ai { world.spawn_skeleton(skeleton, true) } else { world.spawn_skeleton_active(skeleton, yaw) };
            let entity = world.skeleton_mut(id)?;
            entity.random = random;
            entity.yaw = yaw;
            entity.body_rotation.body_yaw = yaw;
            entity.look_control.head_yaw = yaw;
            if let (Some(range), Some(ai)) = (attribute_value(tag, "minecraft:follow_range"), entity.ai.as_deref_mut()) {
                ai.state.follow_range = range;
            }
            id
        }
        "minecraft:creeper" => {
            let mut creeper = minecraftoss_entities::creeper::Creeper::new(pos);
            creeper.yaw = yaw;
            creeper.persistence_required = persistent;
            if let Some(health) = health {
                creeper.health = health;
            }
            creeper.powered = int(tag, "powered").unwrap_or(0) != 0;
            creeper.max_swell = int(tag, "Fuse").unwrap_or(30);
            creeper.explosion_radius = int(tag, "ExplosionRadius").unwrap_or(3);
            creeper.ignited = int(tag, "ignited").unwrap_or(0) != 0;
            let id = world.spawn_creeper(creeper, no_ai);
            let entity = world.creeper_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:spider" | "minecraft:cave_spider" if tag.get("Passengers").is_none() => {
            let mut spider = if kind == "minecraft:cave_spider" { minecraftoss_entities::spider::Spider::cave(pos) } else { minecraftoss_entities::spider::Spider::new(pos) };
            spider.yaw = yaw;
            spider.persistence_required = persistent;
            if let Some(health) = health {
                spider.health = health;
            }
            let id = world.spawn_spider(spider, no_ai);
            let entity = world.spider_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:slime" | "minecraft:magma_cube" => {
            // `AbstractCubeMob.readAdditionalSaveData`: `Size` is one short
            // of the size, and the landing state carries over.
            let size = int(tag, "Size").unwrap_or(0) + 1;
            let mut slime = if kind == "minecraft:magma_cube" { minecraftoss_entities::slime::Slime::magma(pos, size) } else { minecraftoss_entities::slime::Slime::new(pos, size) };
            slime.yaw = yaw;
            slime.persistence_required = persistent;
            slime.was_on_ground = int(tag, "wasOnGround").unwrap_or(0) != 0;
            if let Some(health) = health {
                slime.health = health;
            }
            let id = world.spawn_slime(slime, no_ai);
            let entity = world.slime_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:enderman" => {
            let mut enderman = minecraftoss_entities::enderman::Enderman::new(pos);
            enderman.yaw = yaw;
            enderman.persistence_required = persistent;
            if let Some(health) = health {
                enderman.health = health;
            }
            let id = world.spawn_enderman(enderman, no_ai);
            let entity = world.enderman_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            // `readAdditionalSaveData`: the carried block (air carries
            // nothing) and the anger's end (who it was angry at is not kept).
            entity.ai.state.enderman.carried = tag.get("carriedBlockState").and_then(|state| {
                let name = text(state, "Name")?;
                (name != "minecraft:air").then(|| {
                    let mut block = PlayerBlock::new(name);
                    if let Some(Tag::Compound(properties)) = state.get("Properties") {
                        for (key, value) in properties {
                            if let Some(value) = value.as_str() {
                                block = block.with(key, value);
                            }
                        }
                    }
                    block
                })
            });
            if let Some(end) = tag.get("anger_end_time").and_then(Tag::as_i64) {
                entity.ai.state.enderman.anger_end_time = end;
            }
            id
        }
        "minecraft:witch" => {
            // A witch keeps no drinking state (`DATA_USING_ITEM` is not
            // saved): it wakes up not drinking.
            let mut witch = minecraftoss_entities::witch::Witch::new(pos);
            witch.yaw = yaw;
            witch.persistence_required = persistent;
            if let Some(health) = health {
                witch.health = health;
            }
            let id = world.spawn_witch(witch, no_ai);
            let entity = world.witch_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:iron_golem" => {
            // `IronGolem.readAdditionalSaveData`: built by a player or not,
            // and the anger's end (whom it was angry at is not kept).
            let mut golem = minecraftoss_entities::iron_golem::IronGolem::new(pos);
            golem.yaw = yaw;
            golem.persistence_required = persistent;
            golem.player_created = int(tag, "PlayerCreated").unwrap_or(0) != 0;
            if let Some(health) = health {
                golem.health = health;
            }
            let id = world.spawn_iron_golem(golem, no_ai);
            let entity = world.iron_golem_mut(id)?;
            entity.random = random;
            if let Some(end) = tag.get("anger_end_time").and_then(Tag::as_i64) {
                entity.ai.state.enderman.anger_end_time = end;
            }
            id
        }
        "minecraft:snow_golem" => {
            use minecraftoss_entities::snow_golem::{SnowGolem, MAX_HEALTH};
            let mut golem = SnowGolem::new(pos);
            golem.yaw = yaw;
            golem.persistence_required = persistent;
            golem.pumpkin = int(tag, "Pumpkin").unwrap_or(1) != 0;
            if let Some(health) = health {
                golem.health = health.min(MAX_HEALTH);
            }
            let id = world.spawn_snow_golem(golem, no_ai);
            let entity = world.snow_golem_mut(id)?;
            entity.random = random;
            entity.ai.body_rotation.body_yaw = yaw;
            id
        }
        "minecraft:wolf" => {
            // `Wolf.readAdditionalSaveData` and `TamableAnimal`'s: variant,
            // sound variant, collar, owner (tame with one), sitting, and the
            // anger's end (whom it was angry at is not kept yet).
            let mut wolf = minecraftoss_entities::wolf::Wolf::new(pos);
            wolf.yaw = yaw;
            wolf.persistence_required = persistent;
            wolf.set_age(age);
            let short = |key: &str, default: &str| text(tag, key).map_or(default, |v| v.trim_start_matches("minecraft:")).to_owned();
            wolf.variant = short("variant", "pale");
            wolf.sound_variant = short("sound_variant", "classic");
            wolf.collar = int(tag, "CollarColor").map_or(minecraftoss_entities::wolf::DEFAULT_COLLAR, |c| c as u8);
            wolf.owner = tag.get("Owner").and_then(Tag::as_ints).and_then(|ints| <[i32; 4]>::try_from(ints).ok()).map(minecraftoss_entities::gossip::uuid_from_ints);
            wolf.tame = wolf.owner.is_some();
            wolf.ordered_to_sit = int(tag, "Sitting").unwrap_or(0) != 0;
            wolf.sitting = wolf.ordered_to_sit;
            wolf.in_love = int(tag, "InLove").unwrap_or(0);
            // Taming raised the attribute's base to 40 (loading tame does not).
            wolf.max_health_base = attribute_base(tag, "minecraft:max_health").map_or(minecraftoss_entities::wolf::MAX_HEALTH, |b| b as f32);
            if let Some(health) = health {
                wolf.health = health;
            }
            let id = world.spawn_wolf(wolf, no_ai);
            let entity = world.wolf_mut(id)?;
            entity.random = random;
            if let Some(end) = tag.get("anger_end_time").and_then(Tag::as_i64) {
                entity.ai.state.enderman.anger_end_time = end;
            }
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:cat" | "minecraft:ocelot" => {
            let kind = minecraftoss_entities::feline::FelineKind::of_type(kind)?;
            let mut feline = minecraftoss_entities::feline::Feline::new(kind, pos);
            feline.yaw = yaw;
            feline.persistence_required = persistent;
            feline.set_age(age);
            let short = |key: &str, default: &str| text(tag, key).map_or(default, |v| v.trim_start_matches("minecraft:")).to_owned();
            feline.variant = short("variant", minecraftoss_entities::feline::DEFAULT_CAT_VARIANT);
            feline.sound_variant = short("sound_variant", minecraftoss_entities::feline::CAT_SOUND_VARIANTS[0]);
            feline.collar = int(tag, "CollarColor").map_or(minecraftoss_entities::feline::DEFAULT_COLLAR, |c| c as u8);
            feline.owner = tag.get("Owner").and_then(Tag::as_ints).and_then(|ints| <[i32; 4]>::try_from(ints).ok()).map(minecraftoss_entities::gossip::uuid_from_ints);
            feline.tame = kind == minecraftoss_entities::feline::FelineKind::Cat && feline.owner.is_some();
            feline.ordered_to_sit = int(tag, "Sitting").unwrap_or(0) != 0;
            feline.sitting = feline.ordered_to_sit;
            feline.trusting = int(tag, "Trusting").unwrap_or(0) != 0;
            feline.in_love = int(tag, "InLove").unwrap_or(0);
            if let Some(health) = health {
                feline.health = health;
            }
            let id = world.spawn_feline(feline, no_ai);
            let entity = world.feline_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:ghast" => {
            let mut ghast = minecraftoss_entities::ghast::Ghast::new(pos);
            ghast.yaw = yaw;
            ghast.persistence_required = persistent;
            ghast.explosion_power = int(tag, "ExplosionPower").unwrap_or(1);
            if let Some(health) = health {
                ghast.health = health;
            }
            let id = world.spawn_ghast(ghast, no_ai);
            let entity = world.ghast_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:blaze" => {
            let mut blaze = minecraftoss_entities::blaze::Blaze::new(pos);
            blaze.yaw = yaw;
            blaze.persistence_required = persistent;
            if let Some(health) = health {
                blaze.health = health;
            }
            let id = world.spawn_blaze(blaze, no_ai);
            let entity = world.blaze_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:shulker" => {
            use minecraftoss_entities::shulker;
            let mut entity = shulker::Shulker::new(pos);
            entity.yaw = yaw;
            entity.persistence_required = persistent;
            entity.attach_face = shulker::face_of_id(tag.get("AttachFace").and_then(Tag::as_i64).unwrap_or(0));
            entity.raw_peek = tag.get("Peek").and_then(Tag::as_i64).map_or(0, |peek| i32::from(peek as i8));
            entity.color = tag.get("Color").and_then(Tag::as_i64).map(|color| color as i8).filter(|color| (0..16).contains(color)).map(|color| color as u8);
            entity.covered = has_modifier(tag, "minecraft:armor", COVERED_MODIFIER);
            if let Some(health) = health {
                entity.health = health;
            }
            let id = world.spawn_shulker(entity, no_ai);
            let entity = world.shulker_mut(id)?;
            entity.random = random;
            if let Some(range) = attribute_value(tag, "minecraft:follow_range") {
                entity.ai.state.follow_range = range;
            }
            id
        }
        "minecraft:fireball" | "minecraft:small_fireball" => {
            let motion = tag.get("Motion").and_then(Tag::as_list).map(|m| DVec3::new(m.first().and_then(Tag::as_f64).unwrap_or(0.0), m.get(1).and_then(Tag::as_f64).unwrap_or(0.0), m.get(2).and_then(Tag::as_f64).unwrap_or(0.0))).unwrap_or(DVec3::ZERO);
            let kind = if kind == "minecraft:small_fireball" { minecraftoss_entities::fireball::FireballKind::Small } else { minecraftoss_entities::fireball::FireballKind::Large { explosion_power: int(tag, "ExplosionPower").unwrap_or(1) } };
            let mut fireball = minecraftoss_entities::fireball::Fireball::shoot(kind, minecraftoss_entities::fireball::Shooter::Mob(0), pos, motion);
            fireball.owner = None;
            fireball.velocity = motion;
            fireball.acceleration_power = tag.get("acceleration_power").and_then(Tag::as_f64).unwrap_or(minecraftoss_entities::fireball::ACCELERATION_POWER);
            fireball.left_owner = int(tag, "LeftOwner").unwrap_or(1) != 0;
            return Some(world.spawn_fireball(fireball));
        }
        "minecraft:shulker_bullet" => {
            let double = |key: &str| tag.get(key).and_then(Tag::as_f64).unwrap_or(0.0);
            let uuid = |key: &str| tag.get(key).and_then(Tag::as_ints).and_then(|ints| <[i32; 4]>::try_from(ints).ok()).map(minecraftoss_entities::gossip::uuid_from_ints);
            let mut bullet = minecraftoss_entities::shulker_bullet::ShulkerBullet::new(pos);
            bullet.velocity = tag.get("Motion").and_then(Tag::as_list).map(|m| DVec3::new(m.first().and_then(Tag::as_f64).unwrap_or(0.0), m.get(1).and_then(Tag::as_f64).unwrap_or(0.0), m.get(2).and_then(Tag::as_f64).unwrap_or(0.0))).unwrap_or(DVec3::ZERO);
            bullet.steps = int(tag, "Steps").unwrap_or(0);
            bullet.target_delta = DVec3::new(double("TXD"), double("TYD"), double("TZD"));
            bullet.direction = tag.get("Dir").and_then(Tag::as_i64).map(minecraftoss_entities::shulker::face_of_id);
            bullet.owner_uuid = uuid("Owner");
            bullet.target_uuid = uuid("Target");
            let id = world.spawn_shulker_bullet(bullet);
            if let Some(own) = uuid("UUID") {
                world.set_uuid(id, own);
            }
            return Some(id);
        }
        "minecraft:llama_spit" => {
            let uuid = |key: &str| tag.get(key).and_then(Tag::as_ints).and_then(|ints| <[i32; 4]>::try_from(ints).ok()).map(minecraftoss_entities::gossip::uuid_from_ints);
            let motion = tag.get("Motion").and_then(Tag::as_list).map(|m| DVec3::new(m.first().and_then(Tag::as_f64).unwrap_or(0.0), m.get(1).and_then(Tag::as_f64).unwrap_or(0.0), m.get(2).and_then(Tag::as_f64).unwrap_or(0.0))).unwrap_or(DVec3::ZERO);
            let pitch = tag.get("Rotation").and_then(Tag::as_list).and_then(|r| r.get(1)).and_then(Tag::as_f64).map_or(0.0, |p| p as f32);
            let spit = minecraftoss_entities::llama_spit::LlamaSpit {
                position: pos,
                velocity: motion,
                yaw,
                pitch,
                yaw_o: yaw,
                pitch_o: pitch,
                owner: None,
                owner_uuid: uuid("Owner"),
                left_owner: int(tag, "LeftOwner").unwrap_or(0) != 0,
                tick_count: 0,
                alive: true,
            };
            let id = world.spawn_llama_spit(spit);
            if let Some(own) = uuid("UUID") {
                world.set_uuid(id, own);
            }
            return Some(id);
        }
        "minecraft:evoker_fangs" => {
            let owner = tag.get("Owner").and_then(Tag::as_ints).and_then(|ints| <[i32; 4]>::try_from(ints).ok()).map(minecraftoss_entities::gossip::uuid_from_ints);
            let mut fangs = minecraftoss_entities::evoker_fangs::EvokerFangs::new(pos, 0.0, int(tag, "Warmup").unwrap_or(0), None);
            fangs.yaw = yaw;
            fangs.owner_uuid = owner;
            let id = world.spawn_evoker_fangs(fangs);
            if let Some(own) = tag.get("UUID").and_then(Tag::as_ints).and_then(|ints| <[i32; 4]>::try_from(ints).ok()).map(minecraftoss_entities::gossip::uuid_from_ints) {
                world.set_uuid(id, own);
            }
            return Some(id);
        }
        "minecraft:splash_potion" => {
            // A potion in flight (`ThrowableItemProjectile.Item`), its thrower
            // forgotten.
            let potion = tag
                .get("Item")
                .and_then(|item| item.get("components"))
                .and_then(|c| c.get("minecraft:potion_contents"))
                .and_then(|c| c.get("potion").or(Some(c)))
                .and_then(Tag::as_str)
                .and_then(minecraftoss_entities::potion::Potion::from_id)?;
            let motion = tag.get("Motion").and_then(Tag::as_list).map(|m| DVec3::new(m.first().and_then(Tag::as_f64).unwrap_or(0.0), m.get(1).and_then(Tag::as_f64).unwrap_or(0.0), m.get(2).and_then(Tag::as_f64).unwrap_or(0.0))).unwrap_or(DVec3::ZERO);
            let mut thrown = minecraftoss_entities::potion::ThrownPotion::shoot(potion, 0, pos, motion, 1.0, 0.0, &mut LegacyRandom::new(0));
            thrown.owner = None;
            thrown.velocity = motion;
            thrown.yaw = yaw;
            thrown.left_owner = int(tag, "LeftOwner").unwrap_or(1) != 0;
            return Some(world.spawn_potion(thrown));
        }
        "minecraft:bat" => {
            let mut bat = minecraftoss_entities::bat::Bat::new(pos);
            bat.persistence_required = persistent;
            bat.resting = int(tag, "BatFlags").unwrap_or(0) & 1 != 0;
            if let Some(health) = health {
                bat.health = health;
            }
            let id = world.spawn_bat(bat, no_ai);
            let entity = world.bat_mut(id)?;
            entity.random = random;
            entity.yaw = yaw;
            entity.look_control.head_yaw = yaw;
            entity.body_rotation.body_yaw = yaw;
            id
        }
        "minecraft:wandering_trader" => {
            use minecraftoss_entities::wandering_trader::WanderingTrader;
            let mut trader = WanderingTrader::new(pos);
            trader.yaw = yaw;
            trader.persistence_required = persistent;
            trader.age = age.max(0);
            trader.despawn_delay = int(tag, "DespawnDelay").unwrap_or(0);
            let block = |key: &str| tag.get(key).and_then(Tag::as_ints).and_then(|ints| <[i32; 3]>::try_from(ints).ok()).map(|[x, y, z]| (x, y, z));
            trader.wander_target = block("wander_target");
            trader.home = int(tag, "home_radius").filter(|radius| *radius >= 0).map(|radius| (block("home_pos").unwrap_or((0, 0, 0)), radius));
            if let Some(health) = health {
                trader.health = health.min(minecraftoss_entities::wandering_trader::MAX_HEALTH);
            }
            let id = world.spawn_wandering_trader(trader, no_ai);
            let entity = world.wandering_trader_mut(id)?;
            entity.random = random;
            entity.offers = tag.get("Offers").and_then(offers_of);
            id
        }
        "minecraft:villager" => {
            let mut villager = Villager::new(pos);
            villager.set_age(age);
            villager.persistence_required = persistent;
            // `VillagerData` and `Xp`.
            let data = tag.get("VillagerData");
            if let Some(kind) = data.and_then(|d| text(d, "type")) {
                villager.kind = kind.to_owned();
            }
            if let Some(profession) = data.and_then(|d| text(d, "profession")).and_then(minecraftoss_entities::villager::Profession::from_id) {
                villager.profession = profession;
            }
            if let Some(level) = data.and_then(|d| int(d, "level")) {
                villager.level = level;
            }
            villager.xp = int(tag, "Xp").unwrap_or(0);
            if let Some(health) = health {
                villager.health = health;
            }
            let id = world.spawn_villager(villager, true);
            // The saved stacks' sizes, as the item catalog knows them.
            let maxes: Vec<u8> = tag.get("Inventory").and_then(Tag::as_list).into_iter().flatten().map(|s| world.item_max_stack(text(s, "id").unwrap_or("")).clamp(1, 99) as u8).collect();
            let entity = world.villager_mut(id)?;
            entity.random = random;
            entity.yaw = yaw;
            // `AbstractVillager.readAdditionalSaveData`: the offers it had
            // made (none are made until needed), and `Villager`'s restock
            // clock.
            entity.offers = tag.get("Offers").and_then(offers_of);
            // `InventoryCarrier.readInventoryFromTag` (`fromItemList`: each
            // stack added) and `FoodLevel`.
            for (saved, max) in tag.get("Inventory").and_then(Tag::as_list).into_iter().flatten().zip(maxes) {
                if let (Some(item), Some(count)) = (text(saved, "id"), int(saved, "count").or(Some(1))) {
                    let components = saved.get("components").map(tag_json);
                    let stack = minecraftoss_player::inventory::ItemStack { id: item.to_owned(), count: count.clamp(1, 99) as u8, max, components };
                    entity.inventory.add(stack);
                }
            }
            entity.food_level = int(tag, "FoodLevel").unwrap_or(0);
            // What it holds is its main hand.
            entity.held_item = tag.get("equipment").and_then(|e| e.get("mainhand")).and_then(|item| {
                let id = text(item, "id")?.to_owned();
                let components = item.get("components").map(tag_json).and_then(|v| v.as_object().cloned()).unwrap_or_default();
                Some(minecraftoss_entities::trading::TradeItem { id, count: int(item, "count").unwrap_or(1), components })
            });
            // `Mob.readAdditionalSaveData`: false unless saved (village
            // templates save true).
            entity.can_pick_up_loot = int(tag, "CanPickUpLoot").is_some_and(|b| b != 0);
            entity.last_restock = tag.get("LastRestock").and_then(Tag::as_i64).unwrap_or(0);
            entity.restocks_today = int(tag, "RestocksToday").unwrap_or(0);
            // `Villager.readAdditionalSaveData`: its gossip (the codec's
            // container, then `clear` and `putAll`) and decay time.
            let saved: Vec<_> = tag
                .get("Gossips")
                .and_then(Tag::as_list)
                .into_iter()
                .flatten()
                .filter_map(|g| {
                    let ints = g.get("Target").and_then(Tag::as_ints)?;
                    let target = minecraftoss_entities::gossip::uuid_from_ints([*ints.first()?, *ints.get(1)?, *ints.get(2)?, *ints.get(3)?]);
                    let kind = minecraftoss_entities::gossip::GossipType::from_id(text(g, "Type")?)?;
                    Some((target, kind, int(g, "Value")?))
                })
                .collect();
            std::sync::Arc::make_mut(&mut entity.gossips).replace_with(&minecraftoss_entities::gossip::Gossips::from_entries(&saved));
            entity.last_gossip_decay = tag.get("LastGossipDecay").and_then(Tag::as_i64).unwrap_or(0);
            // Its brain (idle, play and panic, with no points of interest
            // yet), unless it has no AI.
            if !no_ai {
                world.activate_villager(id, yaw);
            }
            id
        }
        _ => return None,
    };
    // `Entity.load`: the UUID gossip about it is kept by.
    if let Some(ints) = tag.get("UUID").and_then(Tag::as_ints).filter(|i| i.len() == 4) {
        world.set_uuid(id, minecraftoss_entities::gossip::uuid_from_ints([ints[0], ints[1], ints[2], ints[3]]));
    }
    if let Some(leash) = tag.get("leash").and_then(saved_leash_of) {
        world.load_leash(id, leash);
    }
    // `LivingEntity.readAdditionalSaveData`: its effects as saved.
    if let (Some(list), Some((effects, _, _))) = (tag.get("active_effects").and_then(Tag::as_list), world.mob_effects_mut(id)) {
        for saved in list {
            if let Some(effect) = effect_of(saved) {
                effects.insert(effect);
            }
        }
    }
    // `Entity.load`: the motion, ground contact and fire it was saved with.
    if let Some(body) = world.body_mut(id) {
        if let Some(motion) = tag.get("Motion").and_then(Tag::as_list) {
            let at = |i: usize| motion.get(i).and_then(Tag::as_f64).unwrap_or(0.0);
            body.velocity = DVec3::new(at(0), at(1), at(2));
        }
        body.on_ground = int(tag, "OnGround").unwrap_or(0) != 0;
        body.fire_ticks = fire;
    }
    Some(id)
}

/// A saved villager's offers (`MerchantOffers.CODEC`: `{Recipes: [...]}`,
/// an empty list when absent); none when one does not read, as vanilla
/// then makes them afresh.
fn offers_of(tag: &Tag) -> Option<Vec<minecraftoss_entities::trading::MerchantOffer>> {
    let Some(list) = tag.get("Recipes") else { return Some(Vec::new()) };
    list.as_list()?.iter().map(|offer| minecraftoss_entities::trading::MerchantOffer::from_json(&tag_json(offer))).collect()
}

/// Offers as `MerchantOffers.CODEC` writes them to NBT: whole numbers as
/// ints, `rewardExp` as a byte and `priceMultiplier` as a float.
fn offers_tag(offers: &[minecraftoss_entities::trading::MerchantOffer]) -> Tag {
    let recipes = offers
        .iter()
        .map(|offer| {
            let mut tag = json_tag(&offer.to_json());
            if let Tag::Compound(map) = &mut tag {
                if let Some(Tag::Double(m)) = map.get("priceMultiplier") {
                    let m = *m as f32;
                    map.insert("priceMultiplier".to_owned(), Tag::Float(m));
                }
            }
            tag
        })
        .collect();
    Tag::Compound([("Recipes".to_owned(), Tag::List(recipes))].into_iter().collect())
}

/// A saved effect (`MobEffectInstance.CODEC`: its ID, level, time, flags
/// and the effect hidden beneath), none for one this port lacks.
fn effect_of(tag: &Tag) -> Option<minecraftoss_entities::effects::EffectInstance> {
    let effect = minecraftoss_entities::effects::MobEffect::from_id(text(tag, "id")?)?;
    Some(details_of(tag, effect))
}

fn details_of(tag: &Tag, effect: minecraftoss_entities::effects::MobEffect) -> minecraftoss_entities::effects::EffectInstance {
    let flag = |key: &str| tag.get(key).and_then(Tag::as_i64).map(|v| v != 0);
    let visible = flag("show_particles").unwrap_or(true);
    minecraftoss_entities::effects::EffectInstance {
        effect,
        duration: int(tag, "duration").unwrap_or(0),
        amplifier: int(tag, "amplifier").unwrap_or(0) & 255,
        ambient: flag("ambient").unwrap_or(false),
        visible,
        show_icon: flag("show_icon").unwrap_or(visible),
        hidden: tag.get("hidden_effect").map(|hidden| Box::new(details_of(hidden, effect))),
    }
}

/// An effect as `MobEffectInstance.CODEC` writes it.
fn effect_tag(effect: &minecraftoss_entities::effects::EffectInstance, with_id: bool) -> Tag {
    let mut map = std::collections::BTreeMap::new();
    if with_id {
        map.insert("id".to_owned(), Tag::String(effect.effect.id().to_owned()));
    }
    map.insert("amplifier".to_owned(), Tag::Byte(effect.amplifier as u8 as i8));
    map.insert("duration".to_owned(), Tag::Int(effect.duration));
    map.insert("ambient".to_owned(), Tag::Byte(i8::from(effect.ambient)));
    map.insert("show_particles".to_owned(), Tag::Byte(i8::from(effect.visible)));
    map.insert("show_icon".to_owned(), Tag::Byte(i8::from(effect.show_icon)));
    if let Some(hidden) = &effect.hidden {
        map.insert("hidden_effect".to_owned(), effect_tag(hidden, false));
    }
    Tag::Compound(map)
}

pub fn saved_leash_of(tag: &Tag) -> Option<minecraftoss_entities::world::SavedLeash> {
    use minecraftoss_entities::world::SavedLeash;
    if let Some(ints) = tag.as_ints() {
        let [x, y, z] = <[i32; 3]>::try_from(ints).ok()?;
        return Some(SavedLeash::Knot((x, y, z)));
    }
    let ints = <[i32; 4]>::try_from(tag.get("UUID").and_then(Tag::as_ints)?).ok()?;
    Some(SavedLeash::Uuid(minecraftoss_entities::gossip::uuid_from_ints(ints)))
}

pub fn leash_tag(leash: minecraftoss_entities::world::SavedLeash) -> Tag {
    match leash {
        minecraftoss_entities::world::SavedLeash::Knot((x, y, z)) => Tag::IntArray(vec![x, y, z]),
        minecraftoss_entities::world::SavedLeash::Uuid(uuid) => Tag::Compound([("UUID".to_owned(), Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(uuid).to_vec()))].into_iter().collect()),
    }
}

/// Doubles as an NBT list.
fn doubles(values: [f64; 3]) -> Tag {
    Tag::List(values.into_iter().map(Tag::Double).collect())
}

/// A UUID for an entity saved without one, from its ID, position and the
/// session (vanilla's are random).
fn fresh_uuid(id: u64, position: DVec3) -> Tag {
    static SALT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    let salt = *SALT.get_or_init(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64));
    let mix = |mut z: u64| {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let a = mix(salt ^ id ^ position.x.to_bits().rotate_left(17) ^ position.y.to_bits() ^ position.z.to_bits().rotate_left(41));
    let b = mix(a);
    Tag::IntArray(vec![(a >> 32) as i32, a as i32, (b >> 32) as i32, b as i32])
}

fn body_equipment(original: Option<&Tag>, body: Option<&str>) -> Option<Tag> {
    let mut slots = match original.and_then(|tag| tag.get("equipment")) {
        Some(Tag::Compound(slots)) => slots.clone(),
        _ if body.is_none() => return None,
        _ => std::collections::BTreeMap::new(),
    };
    match body {
        Some(item) => {
            let kept = slots.remove("body").filter(|b| b.get("id").and_then(Tag::as_str) == Some(item));
            let worn = kept.unwrap_or_else(|| Tag::Compound([("id".to_owned(), Tag::String(item.to_owned())), ("count".to_owned(), Tag::Int(1))].into_iter().collect()));
            slots.insert("body".to_owned(), worn);
        }
        None => {
            slots.remove("body");
        }
    }
    Some(Tag::Compound(slots))
}

fn saddle_equipment(original: Option<&Tag>, saddled: bool) -> Option<Tag> {
    let mut slots = match original.and_then(|tag| tag.get("equipment")) {
        Some(Tag::Compound(slots)) => slots.clone(),
        _ if !saddled => return None,
        _ => std::collections::BTreeMap::new(),
    };
    if saddled {
        let saddle = slots.remove("saddle").filter(|s| s.get("id").and_then(Tag::as_str) == Some("minecraft:saddle"));
        let saddle = saddle.unwrap_or_else(|| Tag::Compound([("id".to_owned(), Tag::String("minecraft:saddle".to_owned())), ("count".to_owned(), Tag::Int(1))].into_iter().collect()));
        slots.insert("saddle".to_owned(), saddle);
    } else {
        slots.remove("saddle");
    }
    Some(Tag::Compound(slots))
}

/// The live mobs whose feet `keep` accepts, as saved entity tags
/// (`Entity.saveWithoutId` with the type's fields), each from the tag it
/// came from when it has one: what the entity world simulates overwrites
/// it; the rest (equipment, attributes, UUID, villager data) stays as
/// loaded. `spawn_saved` reads them back.
pub fn mob_tags(world: &EntityWorld, keep: impl Fn(DVec3) -> bool, originals: &std::collections::HashMap<u64, Tag>) -> Vec<(u64, Tag)> {
    mob_tags_where(world, |_, at| keep(at), originals, true)
}

pub fn mob_tags_where(world: &EntityWorld, keep: impl Fn(u64, DVec3) -> bool, originals: &std::collections::HashMap<u64, Tag>, nest: bool) -> Vec<(u64, Tag)> {
    use minecraftoss_entities::movement::Body;
    let mut out = Vec::new();
    let mut add = |id: u64, kind: &str, body: &Body, yaw: f32, health: f32, persistent: bool, fields: Vec<(&str, Tag)>| {
        if !keep(id, body.position) || health <= 0.0 {
            return;
        }
        let mut map = match originals.get(&id) {
            Some(Tag::Compound(map)) => map.clone(),
            _ => std::collections::BTreeMap::new(),
        };
        let p = body.position;
        map.insert("id".into(), Tag::String(kind.to_owned()));
        map.insert("Pos".into(), doubles([p.x, p.y, p.z]));
        map.insert("Motion".into(), doubles([body.velocity.x, body.velocity.y, body.velocity.z]));
        map.insert("Rotation".into(), Tag::List(vec![Tag::Float(yaw), Tag::Float(0.0)]));
        map.insert("OnGround".into(), Tag::Byte(i8::from(body.on_ground)));
        map.insert("fall_distance".into(), Tag::Double(body.fall_distance));
        map.insert("Fire".into(), Tag::Short(body.fire_ticks.clamp(-32768, 32767) as i16));
        map.insert("Health".into(), Tag::Float(health));
        map.insert("PersistenceRequired".into(), Tag::Byte(i8::from(persistent)));
        // The UUID gossip about it is kept by.
        map.entry("UUID".into()).or_insert_with(|| Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(world.uuid_of(id)).to_vec()));
        for (key, value) in fields {
            map.insert(key.to_owned(), value);
        }
        match world.saved_leash(id) {
            Some(leash) => {
                map.insert("leash".into(), leash_tag(leash));
            }
            None => {
                map.remove("leash");
            }
        }
        out.push((id, Tag::Compound(map)));
    };
    let age = |age: &minecraftoss_entities::age::Age| vec![("Age", Tag::Int(age.ticks)), ("ForcedAge", Tag::Int(age.forced))];
    let text_tag = |s: &str| Tag::String(s.to_owned());
    for e in world.bats() {
        add(e.id, "minecraft:bat", &e.bat.body, e.yaw, e.bat.health, e.bat.persistence_required, vec![("BatFlags", Tag::Byte(i8::from(e.bat.resting)))]);
    }
    for e in world.zombies() {
        let kind = e.zombie.kind.type_id();
        let mut fields = vec![("IsBaby", Tag::Byte(i8::from(e.zombie.baby))), ("CanBreakDoors", Tag::Byte(i8::from(e.zombie.can_break_doors)))];
        if let (ZombieKind::ZombifiedPiglin, Some(ai)) = (e.zombie.kind, e.ai.as_deref()) {
            fields.push(("anger_end_time", Tag::Long(ai.state.enderman.anger_end_time)));
        }
        if e.zombie.kind == ZombieKind::ZombieVillager {
            fields.push(("ConversionTime", Tag::Int(if e.zombie.curing() { e.zombie.villager_conversion_time } else { -1 })));
            if let Some(starter) = e.zombie.conversion_starter.filter(|_| e.zombie.curing()) {
                fields.push(("ConversionPlayer", Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(starter).to_vec())));
            }
        }
        add(e.id, kind, &e.zombie.body, e.yaw, e.zombie.health, e.zombie.persistence_required, fields);
    }
    for e in world.skeletons() {
        let mut fields = Vec::new();
        if e.skeleton.kind == minecraftoss_entities::skeleton::SkeletonKind::Bogged {
            fields.push(("sheared", Tag::Byte(i8::from(e.skeleton.sheared))));
        }
        add(e.id, e.skeleton.kind.type_id(), &e.skeleton.body, e.yaw, e.skeleton.health, e.skeleton.persistence_required, fields);
    }
    for e in world.endermen() {
        let mut fields = vec![("anger_end_time", Tag::Long(e.ai.state.enderman.anger_end_time))];
        if let Some(block) = e.carried() {
            let mut state: std::collections::BTreeMap<String, Tag> = [("Name".to_owned(), Tag::String(block.id.clone()))].into_iter().collect();
            if !block.properties.is_empty() {
                let properties = block.properties.iter().map(|(k, v)| (k.clone(), Tag::String(v.clone()))).collect();
                state.insert("Properties".to_owned(), Tag::Compound(properties));
            }
            fields.push(("carriedBlockState", Tag::Compound(state)));
        }
        add(e.id, "minecraft:enderman", &e.enderman.body, e.enderman.yaw, e.enderman.health, e.enderman.persistence_required, fields);
    }
    for e in world.witches() {
        let w = &e.witch;
        add(e.id, "minecraft:witch", &w.body, w.yaw, w.health, w.persistence_required, Vec::new());
    }
    for e in world.ghasts() {
        let g = &e.ghast;
        add(e.id, "minecraft:ghast", &g.body, g.yaw, g.health, g.persistence_required, vec![("ExplosionPower", Tag::Byte(g.explosion_power.clamp(-128, 127) as i8))]);
    }
    for e in world.blazes() {
        let b = &e.blaze;
        add(e.id, "minecraft:blaze", &b.body, b.yaw, b.health, b.persistence_required, Vec::new());
    }
    for e in world.shulkers() {
        let s = &e.shulker;
        let fields = vec![
            ("AttachFace", Tag::Byte(minecraftoss_entities::shulker::face_id(s.attach_face))),
            ("Peek", Tag::Byte(s.raw_peek as i8)),
            ("Color", Tag::Byte(s.color.unwrap_or(minecraftoss_entities::shulker::NO_COLOR) as i8)),
            ("attributes", with_modifier(originals.get(&e.id), "minecraft:armor", COVERED_MODIFIER, s.covered.then_some(f64::from(minecraftoss_entities::shulker::COVERED_ARMOR)))),
        ];
        add(e.id, "minecraft:shulker", &s.body, s.yaw, s.health, s.persistence_required, fields);
    }
    for e in world.iron_golems() {
        let g = &e.golem;
        let fields = vec![("PlayerCreated", Tag::Byte(i8::from(g.player_created))), ("anger_end_time", Tag::Long(e.ai.state.enderman.anger_end_time))];
        add(e.id, "minecraft:iron_golem", &g.body, g.yaw, g.health, g.persistence_required, fields);
    }
    for e in world.snow_golems() {
        let g = &e.golem;
        let fields = vec![("Pumpkin", Tag::Byte(i8::from(g.pumpkin)))];
        add(e.id, minecraftoss_entities::snow_golem::TYPE_ID, &g.body, g.yaw, g.health, g.persistence_required, fields);
    }
    for e in world.wolves() {
        let w = &e.wolf;
        let mut fields = vec![
            ("variant", text_tag(&format!("minecraft:{}", w.variant))),
            ("sound_variant", text_tag(&format!("minecraft:{}", w.sound_variant))),
            ("CollarColor", Tag::Byte(w.collar as i8)),
            ("Sitting", Tag::Byte(i8::from(w.ordered_to_sit))),
            ("anger_end_time", Tag::Long(e.ai.state.enderman.anger_end_time)),
        ];
        fields.extend(age(&w.age));
        fields.push(("InLove", Tag::Int(w.in_love)));
        fields.push(("attributes", with_attribute_base(originals.get(&e.id), "minecraft:max_health", f64::from(w.max_health_base))));
        if let Some(owner) = w.owner {
            fields.push(("Owner", Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(owner).to_vec())));
        }
        add(e.id, "minecraft:wolf", &w.body, w.yaw, w.health, w.persistence_required, fields);
    }
    for e in world.felines() {
        let f = &e.feline;
        let mut fields = age(&f.age);
        fields.push(("InLove", Tag::Int(f.in_love)));
        match f.kind {
            minecraftoss_entities::feline::FelineKind::Cat => {
                fields.push(("variant", text_tag(&format!("minecraft:{}", f.variant))));
                fields.push(("sound_variant", text_tag(&format!("minecraft:{}", f.sound_variant))));
                fields.push(("CollarColor", Tag::Byte(f.collar as i8)));
                fields.push(("Sitting", Tag::Byte(i8::from(f.ordered_to_sit))));
                if let Some(owner) = f.owner {
                    fields.push(("Owner", Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(owner).to_vec())));
                }
            }
            minecraftoss_entities::feline::FelineKind::Ocelot => fields.push(("Trusting", Tag::Byte(i8::from(f.trusting)))),
        }
        add(e.id, f.kind.type_id(), &f.body, f.yaw, f.health, f.persistence_required, fields);
    }
    for e in world.slimes() {
        let slime = &e.slime;
        let fields = vec![("Size", Tag::Int(slime.size - 1)), ("wasOnGround", Tag::Byte(i8::from(slime.was_on_ground)))];
        add(e.id, slime.type_id(), &slime.body, slime.yaw, slime.health, slime.persistence_required, fields);
    }
    for e in world.creepers() {
        let c = &e.creeper;
        if c.exploded {
            continue;
        }
        let fields = vec![
            ("powered", Tag::Byte(i8::from(c.powered))),
            ("Fuse", Tag::Short(c.max_swell as i16)),
            ("ExplosionRadius", Tag::Byte(c.explosion_radius as i8)),
            ("ignited", Tag::Byte(i8::from(c.ignited))),
        ];
        add(e.id, "minecraft:creeper", &c.body, e.ai.yaw, c.health, c.persistence_required, fields);
    }
    for e in world.spiders() {
        add(e.id, e.spider.type_id(), &e.spider.body, e.spider.yaw, e.spider.health, e.spider.persistence_required, Vec::new());
    }
    for e in world.wandering_traders() {
        let t = &e.trader;
        let mut fields = vec![("DespawnDelay", Tag::Int(t.despawn_delay)), ("Age", Tag::Int(t.age))];
        if let Some((x, y, z)) = t.wander_target {
            fields.push(("wander_target", Tag::IntArray(vec![x, y, z])));
        }
        if let Some(((x, y, z), radius)) = t.home {
            fields.push(("home_pos", Tag::IntArray(vec![x, y, z])));
            fields.push(("home_radius", Tag::Int(radius)));
        }
        if let Some(offers) = &e.offers {
            fields.push(("Offers", offers_tag(offers)));
        }
        if let Some(drink) = t.held {
            let mut stack: std::collections::BTreeMap<String, Tag> = [("id".to_owned(), text_tag(drink.item())), ("count".to_owned(), Tag::Int(1))].into_iter().collect();
            if let Some(components) = drink.components() {
                stack.insert("components".to_owned(), json_tag(&components));
            }
            fields.push(("equipment", Tag::Compound([("mainhand".to_owned(), Tag::Compound(stack))].into_iter().collect())));
        }
        add(e.id, minecraftoss_entities::wandering_trader::TYPE_ID, &t.body, t.yaw, t.health, t.persistence_required, fields);
    }
    for e in world.villagers() {
        let v = &e.villager;
        let mut fields = age(&v.age);
        let mut data = std::collections::BTreeMap::new();
        data.insert("type".to_owned(), text_tag(&v.kind));
        data.insert("profession".to_owned(), text_tag(v.profession.id()));
        data.insert("level".to_owned(), Tag::Int(v.level));
        fields.push(("VillagerData", Tag::Compound(data)));
        fields.push(("Xp", Tag::Int(v.xp)));
        if let Some(item) = &e.held_item {
            let mut stack: std::collections::BTreeMap<String, Tag> = [("id".to_owned(), text_tag(&item.id)), ("count".to_owned(), Tag::Int(item.count))].into_iter().collect();
            if !item.components.is_empty() {
                stack.insert("components".to_owned(), json_tag(&serde_json::Value::Object(item.components.clone())));
            }
            fields.push(("equipment", Tag::Compound([("mainhand".to_owned(), Tag::Compound(stack))].into_iter().collect())));
        }
        add(e.id, "minecraft:villager", &v.body, e.ai.as_deref().map_or(e.yaw, |ai| ai.yaw), v.health, v.persistence_required, fields);
    }
    for e in world.cows() {
        let cow = &e.cow;
        let mut fields = age(&cow.age);
        fields.push(("variant", text_tag(match cow.variant { CowVariant::Warm => "minecraft:warm", CowVariant::Cold => "minecraft:cold", CowVariant::Temperate => "minecraft:temperate" })));
        fields.push(("sound_variant", text_tag(match cow.sound_variant { CowSoundVariant::Moody => "minecraft:moody", CowSoundVariant::Classic => "minecraft:classic" })));
        if let Some(horse) = &e.horse {
            use minecraftoss_entities::horse::HorseKind;
            let mut fields = age(&cow.age);
            fields.push(("EatingHaystack", Tag::Byte(i8::from(horse.eating))));
            fields.push(("Bred", Tag::Byte(i8::from(horse.bred))));
            fields.push(("Temper", Tag::Int(horse.temper)));
            fields.push(("Tame", Tag::Byte(i8::from(horse.tame))));
            match horse.kind {
                HorseKind::Horse => fields.push(("Variant", Tag::Int(horse.variant))),
                _ => fields.push(("ChestedHorse", Tag::Byte(i8::from(horse.chest)))),
            }
            if horse.kind.is_llama() {
                fields.push(("Variant", Tag::Int(horse.variant)));
                fields.push(("Strength", Tag::Int(horse.strength)));
            }
            if let Some(owner) = horse.owner {
                fields.push(("Owner", Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(owner).to_vec())));
            }
            if horse.chest {
                let items = horse
                    .inventory
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, stack)| {
                        let stack = stack.as_ref()?;
                        let mut item: std::collections::BTreeMap<String, Tag> = [("Slot".to_owned(), Tag::Byte(slot as i8)), ("id".to_owned(), text_tag(&stack.id)), ("count".to_owned(), Tag::Int(i32::from(stack.count)))].into_iter().collect();
                        if let Some(components) = stack.components.as_ref() {
                            item.insert("components".to_owned(), json_tag(components));
                        }
                        Some(Tag::Compound(item))
                    })
                    .collect();
                fields.push(("Items", Tag::List(items)));
            }
            let worn = body_equipment(originals.get(&e.id), horse.body_item.as_deref()).map(|slots| Tag::Compound([("equipment".to_owned(), slots)].into_iter().collect()));
            if let Some(equipment) = saddle_equipment(worn.as_ref().or(originals.get(&e.id)), horse.saddled) {
                fields.push(("equipment", equipment));
            }
            if horse.kind == HorseKind::TraderLlama {
                fields.push(("DespawnDelay", Tag::Int(horse.despawn_delay)));
            }
            add(e.id, horse.kind.type_id(), &cow.body, cow.yaw, cow.health, cow.persistence_required, fields);
            continue;
        }
        let kind = match &e.mooshroom {
            Some(state) => {
                fields.push(("Type", text_tag(if state.variant == MushroomVariant::Brown { "brown" } else { "red" })));
                "minecraft:mooshroom"
            }
            None => "minecraft:cow",
        };
        add(e.id, kind, &cow.body, cow.yaw, cow.health, cow.persistence_required, fields);
    }
    for e in world.sheep() {
        let mut fields = age(&e.sheep.age);
        fields.push(("Color", Tag::Byte((e.sheep.wool.data() & 15) as i8)));
        fields.push(("Sheared", Tag::Byte(i8::from(e.sheep.wool.sheared()))));
        add(e.id, "minecraft:sheep", &e.body, e.yaw, e.health, e.sheep.persistence_required, fields);
    }
    for e in world.pigs() {
        let pig = &e.pig;
        let mut fields = age(&pig.age);
        fields.push(("variant", text_tag(match pig.variant { PigVariant::Warm => "minecraft:warm", PigVariant::Cold => "minecraft:cold", PigVariant::Temperate => "minecraft:temperate" })));
        fields.push(("sound_variant", text_tag(match pig.sound_variant { PigSoundVariant::Mini => "minecraft:mini", PigSoundVariant::Big => "minecraft:big", PigSoundVariant::Classic => "minecraft:classic" })));
        if pig.saddled {
            // `Mob.equipment`'s saddle slot.
            let saddle = Tag::Compound([("id".to_owned(), text_tag("minecraft:saddle")), ("count".to_owned(), Tag::Int(1))].into_iter().collect());
            fields.push(("equipment", Tag::Compound([("saddle".to_owned(), saddle)].into_iter().collect())));
        }
        add(e.id, "minecraft:pig", &pig.body, pig.yaw, pig.health, pig.persistence_required, fields);
    }
    for e in world.striders() {
        let strider = &e.strider;
        let mut fields = age(&strider.age);
        fields.push(("InLove", Tag::Int(strider.in_love)));
        if let Some(equipment) = saddle_equipment(originals.get(&e.id), strider.saddled) {
            fields.push(("equipment", equipment));
        }
        add(e.id, "minecraft:strider", &strider.body, strider.yaw, strider.health, strider.persistence_required, fields);
    }
    for e in world.hoglins() {
        use minecraftoss_entities::hoglin::HoglinKind;
        let hoglin = &e.hoglin;
        let fields = match hoglin.kind {
            HoglinKind::Hoglin => {
                let mut fields = age(&hoglin.age);
                fields.push(("InLove", Tag::Int(hoglin.in_love)));
                fields.push(("IsImmuneToZombification", Tag::Byte(i8::from(hoglin.immune_to_zombification))));
                fields.push(("TimeInOverworld", Tag::Int(hoglin.time_in_overworld)));
                fields.push(("CannotBeHunted", Tag::Byte(i8::from(hoglin.cannot_be_hunted))));
                fields
            }
            HoglinKind::Zoglin => vec![("IsBaby", Tag::Byte(i8::from(hoglin.baby)))],
        };
        add(e.id, hoglin.kind.type_id(), &hoglin.body, hoglin.yaw, hoglin.health, hoglin.persistence_required, fields);
    }
    for e in world.piglins() {
        let piglin = &e.piglin;
        add(e.id, piglin.kind.type_id(), &piglin.body, piglin.yaw, piglin.health, piglin.persistence_required, crate::piglin_save::fields(e));
    }
    for e in world.illagers() {
        let illager = &e.illager;
        add(e.id, illager.kind.type_id(), &illager.body, illager.yaw, illager.health, illager.persistence_required, crate::illager_save::fields(e));
    }
    for e in world.vexes() {
        let vex = &e.vex;
        add(e.id, minecraftoss_entities::vex::TYPE_ID, &vex.body, vex.yaw, vex.health, vex.persistence_required, crate::vex_save::fields(world, e));
    }
    for e in world.chickens() {
        let chicken = &e.chicken;
        let mut fields = age(&chicken.age);
        fields.push(("variant", text_tag(match chicken.variant { ChickenVariant::Warm => "minecraft:warm", ChickenVariant::Cold => "minecraft:cold", ChickenVariant::Temperate => "minecraft:temperate" })));
        fields.push(("sound_variant", text_tag(match chicken.sound_variant { ChickenSoundVariant::Picky => "minecraft:picky", ChickenSoundVariant::Classic => "minecraft:classic" })));
        fields.push(("EggLayTime", Tag::Int(chicken.egg_time)));
        add(e.id, "minecraft:chicken", &chicken.body, chicken.yaw, chicken.health, chicken.persistence_required, fields);
    }
    // `AbstractVillager.addAdditionalSaveData`: the offers once made, and
    // `Villager`'s restock clock.
    for (id, tag) in &mut out {
        if let (Some(e), Tag::Compound(map)) = (world.villagers().iter().find(|v| v.id == *id), tag) {
            match &e.offers {
                Some(offers) => map.insert("Offers".to_owned(), offers_tag(offers)),
                None => map.remove("Offers"),
            };
            map.insert("LastRestock".to_owned(), Tag::Long(e.last_restock));
            // `writeInventoryToTag` (the stacks in slot order) and `FoodLevel`.
            let inventory = e
                .inventory
                .slots
                .iter()
                .flatten()
                .map(|s| {
                    let mut stack = vec![("id".to_owned(), Tag::String(s.id.clone())), ("count".to_owned(), Tag::Int(i32::from(s.count)))];
                    if let Some(components) = &s.components {
                        stack.push(("components".to_owned(), json_tag(components)));
                    }
                    Tag::Compound(stack.into_iter().collect())
                })
                .collect();
            map.insert("Inventory".to_owned(), Tag::List(inventory));
            map.insert("FoodLevel".to_owned(), Tag::Byte(e.food_level as i8));
            map.insert("CanPickUpLoot".to_owned(), Tag::Byte(i8::from(e.can_pick_up_loot)));
            map.insert("RestocksToday".to_owned(), Tag::Int(e.restocks_today));
            // `Villager.addAdditionalSaveData`: gossip as `GossipContainer.CODEC`
            // writes it, and its decay time.
            let gossips = e
                .gossips
                .unpack()
                .into_iter()
                .map(|(target, kind, value)| {
                    let entry = [
                        ("Target".to_owned(), Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(target).to_vec())),
                        ("Type".to_owned(), Tag::String(kind.id().to_owned())),
                        ("Value".to_owned(), Tag::Int(value)),
                    ];
                    Tag::Compound(entry.into_iter().collect())
                })
                .collect();
            map.insert("Gossips".to_owned(), Tag::List(gossips));
            map.insert("LastGossipDecay".to_owned(), Tag::Long(e.last_gossip_decay));
        }
    }
    // `LivingEntity.addAdditionalSaveData`: the effects, when there are any.
    for (id, tag) in &mut out {
        if let (Some(effects), Tag::Compound(map)) = (world.mob_effects(*id), tag) {
            if effects.is_empty() {
                map.remove("active_effects");
            } else {
                map.insert("active_effects".to_owned(), Tag::List(effects.iter().map(|e| effect_tag(e, true)).collect()));
            }
        }
    }
    if nest {
        nest_passengers(world, &mut out);
    } else {
        for (_, tag) in &mut out {
            if let Tag::Compound(map) = tag {
                map.remove("Passengers");
            }
        }
    }
    for f in world.fireballs() {
        let fireball = &f.fireball;
        if !keep(f.id, fireball.position) || !fireball.alive {
            continue;
        }
        let mut map = std::collections::BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(fireball.kind.type_id().to_owned()));
        map.insert("Pos".to_owned(), doubles(fireball.position.to_array()));
        map.insert("Motion".to_owned(), doubles(fireball.velocity.to_array()));
        map.insert("Rotation".to_owned(), Tag::List(vec![Tag::Float(0.0), Tag::Float(0.0)]));
        map.insert("acceleration_power".to_owned(), Tag::Double(fireball.acceleration_power));
        map.insert("LeftOwner".to_owned(), Tag::Byte(i8::from(fireball.left_owner)));
        if let minecraftoss_entities::fireball::FireballKind::Large { explosion_power } = fireball.kind {
            map.insert("ExplosionPower".to_owned(), Tag::Byte(explosion_power.clamp(-128, 127) as i8));
        }
        map.insert("UUID".to_owned(), fresh_uuid(f.id, fireball.position));
        out.push((f.id, Tag::Compound(map)));
    }
    for b in world.shulker_bullets() {
        let bullet = &b.bullet;
        if !keep(b.id, bullet.position) || !bullet.alive {
            continue;
        }
        let uuid = |uuid: u128| Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(uuid).to_vec());
        let mut map = std::collections::BTreeMap::new();
        map.insert("id".to_owned(), Tag::String("minecraft:shulker_bullet".to_owned()));
        map.insert("Pos".to_owned(), doubles(bullet.position.to_array()));
        map.insert("Motion".to_owned(), doubles(bullet.velocity.to_array()));
        map.insert("Rotation".to_owned(), Tag::List(vec![Tag::Float(0.0), Tag::Float(0.0)]));
        map.insert("UUID".to_owned(), uuid(world.uuid_of(b.id)));
        map.insert("Steps".to_owned(), Tag::Int(bullet.steps));
        map.insert("TXD".to_owned(), Tag::Double(bullet.target_delta.x));
        map.insert("TYD".to_owned(), Tag::Double(bullet.target_delta.y));
        map.insert("TZD".to_owned(), Tag::Double(bullet.target_delta.z));
        if let Some(direction) = bullet.direction {
            map.insert("Dir".to_owned(), Tag::Byte(minecraftoss_entities::shulker::face_id(direction)));
        }
        if let Some(owner) = bullet.owner.map(|o| world.uuid_of(o)).or(bullet.owner_uuid) {
            map.insert("Owner".to_owned(), uuid(owner));
        }
        if let Some(target) = bullet.target.map(|t| world.uuid_of(t)).or(bullet.target_uuid) {
            map.insert("Target".to_owned(), uuid(target));
        }
        out.push((b.id, Tag::Compound(map)));
    }
    for l in world.llama_spits() {
        let spit = &l.spit;
        if !keep(l.id, spit.position) || !spit.alive {
            continue;
        }
        let uuid = |uuid: u128| Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(uuid).to_vec());
        let mut map = std::collections::BTreeMap::new();
        map.insert("id".to_owned(), Tag::String("minecraft:llama_spit".to_owned()));
        map.insert("Pos".to_owned(), doubles(spit.position.to_array()));
        map.insert("Motion".to_owned(), doubles(spit.velocity.to_array()));
        map.insert("Rotation".to_owned(), Tag::List(vec![Tag::Float(spit.yaw), Tag::Float(spit.pitch)]));
        map.insert("UUID".to_owned(), uuid(world.uuid_of(l.id)));
        map.insert("LeftOwner".to_owned(), Tag::Byte(i8::from(spit.left_owner)));
        if let Some(owner) = spit.owner.map(|o| world.uuid_of(o)).or(spit.owner_uuid) {
            map.insert("Owner".to_owned(), uuid(owner));
        }
        out.push((l.id, Tag::Compound(map)));
    }
    for f in world.evoker_fangs() {
        let fangs = &f.fangs;
        if !keep(f.id, fangs.position) || !fangs.alive {
            continue;
        }
        let uuid = |uuid: u128| Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(uuid).to_vec());
        let mut map = std::collections::BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(minecraftoss_entities::evoker_fangs::TYPE_ID.to_owned()));
        map.insert("Pos".to_owned(), doubles(fangs.position.to_array()));
        map.insert("Rotation".to_owned(), Tag::List(vec![Tag::Float(fangs.yaw), Tag::Float(0.0)]));
        map.insert("UUID".to_owned(), uuid(world.uuid_of(f.id)));
        map.insert("Warmup".to_owned(), Tag::Int(fangs.warmup));
        if let Some(owner) = fangs.owner.map(|o| world.uuid_of(o)).or(fangs.owner_uuid) {
            map.insert("Owner".to_owned(), uuid(owner));
        }
        out.push((f.id, Tag::Compound(map)));
    }
    // Splash potions in flight (`ThrowableItemProjectile`).
    for p in world.potions() {
        let potion = &p.potion;
        if !keep(p.id, potion.position) || !potion.alive {
            continue;
        }
        let mut map = std::collections::BTreeMap::new();
        map.insert("id".to_owned(), Tag::String("minecraft:splash_potion".to_owned()));
        map.insert("Pos".to_owned(), doubles(potion.position.to_array()));
        map.insert("Motion".to_owned(), doubles(potion.velocity.to_array()));
        map.insert("Rotation".to_owned(), Tag::List(vec![Tag::Float(potion.yaw), Tag::Float(potion.pitch)]));
        map.insert("LeftOwner".to_owned(), Tag::Byte(i8::from(potion.left_owner)));
        let contents = Tag::Compound([("potion".to_owned(), Tag::String(potion.potion.id().to_owned()))].into_iter().collect());
        let components = Tag::Compound([("minecraft:potion_contents".to_owned(), contents)].into_iter().collect());
        let item = [("id".to_owned(), Tag::String("minecraft:splash_potion".to_owned())), ("count".to_owned(), Tag::Int(1)), ("components".to_owned(), components)];
        map.insert("Item".to_owned(), Tag::Compound(item.into_iter().collect()));
        map.insert("UUID".to_owned(), fresh_uuid(p.id, potion.position));
        out.push((p.id, Tag::Compound(map)));
    }
    for e in world.wandering_traders() {
        let Some(Tag::Compound(map)) = out.iter_mut().find(|(id, _)| *id == e.id).map(|(_, tag)| tag) else { continue };
        if e.trader.wander_target.is_none() {
            map.remove("wander_target");
        }
        if e.trader.home.is_none() {
            map.remove("home_pos");
            map.remove("home_radius");
        }
        if e.trader.held.is_none() {
            if let Some(Tag::Compound(equipment)) = map.get_mut("equipment") {
                equipment.remove("mainhand");
            }
        }
    }
    out
}

/// The level's items and experience orbs whose position `keep` accepts, as
/// saved entity tags (`ItemEntity`, `ExperienceOrb.addAdditionalSaveData`).
pub fn level_entity_tags(level: &Level<'static>, keep: impl Fn(DVec3) -> bool) -> Vec<Tag> {
    let mut out = Vec::new();
    for e in &level.entities {
        let p = DVec3::from_array(e.pos);
        if e.removed || !keep(p) {
            continue;
        }
        let mut map: std::collections::BTreeMap<String, Tag> = std::collections::BTreeMap::new();
        map.insert("Pos".into(), doubles(e.pos));
        map.insert("Motion".into(), doubles(e.delta));
        map.insert("OnGround".into(), Tag::Byte(i8::from(e.on_ground)));
        map.insert("UUID".into(), fresh_uuid(e.id as u64, p));
        if let Some(item) = e.item_data() {
            if item.stack.is_empty() || item.stack.id.starts_with(crate::item_nbt::KILLSTREAK_PREFIX) {
                continue;
            }
            map.insert("id".into(), Tag::String("minecraft:item".into()));
            let mut stack = std::collections::BTreeMap::new();
            let (id, components) = crate::item_nbt::to_disk(&item.stack.id, item.stack.components.as_ref().and_then(saved_components));
            stack.insert("id".to_owned(), Tag::String(id));
            stack.insert("count".to_owned(), Tag::Int(item.stack.count));
            if let Some(components) = components {
                stack.insert("components".to_owned(), components);
            }
            map.insert("Item".into(), Tag::Compound(stack));
            map.insert("Age".into(), Tag::Short(item.age.clamp(-32768, 32767) as i16));
            map.insert("PickupDelay".into(), Tag::Short(item.pickup_delay.clamp(-32768, 32767) as i16));
            map.insert("Health".into(), Tag::Short(item.health as i16));
            map.insert("Fire".into(), Tag::Short(e.remaining_fire_ticks.clamp(-32768, 32767) as i16));
        } else if let Some(orb) = e.orb_data() {
            map.insert("id".into(), Tag::String("minecraft:experience_orb".into()));
            map.insert("Value".into(), Tag::Short(orb.value as i16));
            map.insert("Count".into(), Tag::Int(orb.count));
            map.insert("Age".into(), Tag::Short(orb.age as i16));
            map.insert("Health".into(), Tag::Short(orb.health as i16));
        } else {
            continue;
        }
        out.push(Tag::Compound(map));
    }
    out
}

/// A saved item or experience orb joins the level; false for other types.
pub fn load_level_entity(level: &mut Level<'static>, tag: &Tag) -> bool {
    let Some(pos) = position(tag) else { return false };
    let motion = tag.get("Motion").and_then(Tag::as_list).map_or([0.0; 3], |m| [0, 1, 2].map(|i| m.get(i).and_then(Tag::as_f64).unwrap_or(0.0)));
    match text(tag, "id") {
        Some("minecraft:item") => {
            let Some(item) = tag.get("Item") else { return true };
            let (Some(id), count) = (text(item, "id"), int(item, "count").unwrap_or(1)) else { return true };
            let (id, components) = crate::item_nbt::from_disk(id, item.get("components").and_then(saved_components));
            let mut stack = minecraftoss_core::item::ItemStack::new(&id, count);
            stack.components = components;
            let id = level.spawn_item_with(pos.to_array(), stack, motion, int(tag, "PickupDelay").unwrap_or(0), int(tag, "Age").unwrap_or(0));
            if let Some(e) = level.entities.iter_mut().find(|e| e.id == id) {
                e.remaining_fire_ticks = int(tag, "Fire").unwrap_or(0);
                if let minecraftoss_world::level::entity::EntityKind::Item(data) = &mut e.kind {
                    data.health = int(tag, "Health").unwrap_or(5);
                }
            }
            true
        }
        Some("minecraft:experience_orb") => {
            let mut orb = minecraftoss_world::level::entity::Entity::experience_orb(pos.to_array(), int(tag, "Value").unwrap_or(0));
            orb.delta = motion;
            if let Some(data) = orb.orb_data_mut() {
                data.count = int(tag, "Count").unwrap_or(1).max(1);
                data.age = int(tag, "Age").unwrap_or(0);
                data.health = int(tag, "Health").unwrap_or(5);
            }
            level.add_entity(orb);
            true
        }
        _ => false,
    }
}

/// A saved attribute's value (`AttributeInstance.calculateValue`): the
/// base, its added values, then the multipliers of the base and the total,
/// within the attribute's range (follow range: 0 to 2048).
/// A saved attribute's base.
fn attribute_base(tag: &Tag, id: &str) -> Option<f64> {
    tag.get("attributes")?.as_list()?.iter().find(|a| a.get("id").and_then(Tag::as_str) == Some(id))?.get("base")?.as_f64()
}

/// A saved entity's attributes with one base set (the entry added when
/// missing), the rest as loaded.
fn with_attribute_base(original: Option<&Tag>, id: &str, base: f64) -> Tag {
    let mut list = original.and_then(|t| t.get("attributes")).and_then(Tag::as_list).map(<[Tag]>::to_vec).unwrap_or_default();
    match list.iter_mut().find(|a| a.get("id").and_then(Tag::as_str) == Some(id)) {
        Some(Tag::Compound(entry)) => {
            entry.insert("base".to_owned(), Tag::Double(base));
        }
        _ => {
            let entry: std::collections::BTreeMap<String, Tag> = [("id".to_owned(), Tag::String(id.to_owned())), ("base".to_owned(), Tag::Double(base))].into_iter().collect();
            list.push(Tag::Compound(entry));
        }
    }
    Tag::List(list)
}

const COVERED_MODIFIER: &str = "minecraft:covered";

fn has_modifier(tag: &Tag, attribute: &str, modifier: &str) -> bool {
    let Some(entry) = tag.get("attributes").and_then(Tag::as_list).and_then(|list| list.iter().find(|a| a.get("id").and_then(Tag::as_str) == Some(attribute))) else {
        return false;
    };
    entry.get("modifiers").and_then(Tag::as_list).is_some_and(|list| list.iter().any(|m| m.get("id").and_then(Tag::as_str) == Some(modifier)))
}

fn with_modifier(original: Option<&Tag>, attribute: &str, modifier: &str, amount: Option<f64>) -> Tag {
    let mut list = original.and_then(|t| t.get("attributes")).and_then(Tag::as_list).map_or_else(Vec::new, <[Tag]>::to_vec);
    let index = match list.iter().position(|a| a.get("id").and_then(Tag::as_str) == Some(attribute)) {
        Some(index) => index,
        None if amount.is_none() => return Tag::List(list),
        None => {
            let entry: std::collections::BTreeMap<String, Tag> = [("id".to_owned(), Tag::String(attribute.to_owned())), ("base".to_owned(), Tag::Double(0.0))].into_iter().collect();
            list.push(Tag::Compound(entry));
            list.len() - 1
        }
    };
    if let Some(Tag::Compound(entry)) = list.get_mut(index) {
        let mut modifiers = entry.get("modifiers").and_then(Tag::as_list).map_or_else(Vec::new, <[Tag]>::to_vec);
        modifiers.retain(|m| m.get("id").and_then(Tag::as_str) != Some(modifier));
        if let Some(amount) = amount {
            let added: std::collections::BTreeMap<String, Tag> = [
                ("id".to_owned(), Tag::String(modifier.to_owned())),
                ("amount".to_owned(), Tag::Double(amount)),
                ("operation".to_owned(), Tag::String("add_value".to_owned())),
            ]
            .into_iter()
            .collect();
            modifiers.push(Tag::Compound(added));
        }
        if modifiers.is_empty() {
            entry.remove("modifiers");
        } else {
            entry.insert("modifiers".to_owned(), Tag::List(modifiers));
        }
    }
    Tag::List(list)
}

fn attribute_value(tag: &Tag, id: &str) -> Option<f64> {
    let entry = tag.get("attributes")?.as_list()?.iter().find(|a| a.get("id").and_then(Tag::as_str) == Some(id))?;
    let mut base = entry.get("base")?.as_f64()?;
    let modifiers: Vec<(&str, f64)> = entry
        .get("modifiers")
        .and_then(Tag::as_list)
        .map(|list| list.iter().filter_map(|m| Some((m.get("operation")?.as_str()?, m.get("amount")?.as_f64()?))).collect())
        .unwrap_or_default();
    for (_, amount) in modifiers.iter().filter(|(op, _)| *op == "add_value") {
        base += amount;
    }
    let mut value = base;
    for (_, amount) in modifiers.iter().filter(|(op, _)| *op == "add_multiplied_base") {
        value += base * amount;
    }
    for (_, amount) in modifiers.iter().filter(|(op, _)| *op == "add_multiplied_total") {
        value *= 1.0 + amount;
    }
    Some(value.clamp(0.0, 2048.0))
}

/// The head slot's item, when there is one: whether it can be damaged
/// (armor can; pumpkins and heads cannot).
pub fn converted_zombie_tag(original: &Tag) -> Tag {
    const KEPT: [&str; 14] = ["equipment", "drop_chances", "CanPickUpLoot", "LeftHanded", "NoAI", "PersistenceRequired", "CustomName", "CustomNameVisible", "Invulnerable", "NoGravity", "PortalCooldown", "Silent", "Tags", "leash"];
    let Tag::Compound(map) = original else { return Tag::Compound(std::collections::BTreeMap::new()) };
    Tag::Compound(map.iter().filter(|(key, _)| KEPT.contains(&key.as_str())).map(|(key, value)| (key.clone(), value.clone())).collect())
}

pub fn infected_villager_tag(world: &EntityWorld, villager: u64, original: Option<&Tag>) -> Option<Tag> {
    let e = world.villagers().iter().find(|v| v.id == villager)?;
    let v = &e.villager;
    let Tag::Compound(mut map) = original.map_or_else(|| Tag::Compound(std::collections::BTreeMap::new()), converted_zombie_tag) else { return None };
    let text_tag = |s: &str| Tag::String(s.to_owned());
    let body = &v.body;
    map.insert("id".to_owned(), text_tag("minecraft:zombie_villager"));
    map.insert("Pos".to_owned(), doubles(body.position.to_array()));
    map.insert("Motion".to_owned(), doubles(body.velocity.to_array()));
    map.insert("Rotation".to_owned(), Tag::List(vec![Tag::Float(e.ai.as_deref().map_or(e.yaw, |ai| ai.yaw)), Tag::Float(0.0)]));
    map.insert("OnGround".to_owned(), Tag::Byte(i8::from(body.on_ground)));
    map.insert("fall_distance".to_owned(), Tag::Double(body.fall_distance));
    map.insert("IsBaby".to_owned(), Tag::Byte(i8::from(v.age.baby())));
    map.insert("PersistenceRequired".to_owned(), Tag::Byte(i8::from(v.persistence_required)));
    map.insert("CanPickUpLoot".to_owned(), Tag::Byte(i8::from(e.can_pick_up_loot)));
    let data = [("type".to_owned(), text_tag(&v.kind)), ("profession".to_owned(), text_tag(v.profession.id())), ("level".to_owned(), Tag::Int(v.level))];
    map.insert("VillagerData".to_owned(), Tag::Compound(data.into_iter().collect()));
    map.insert("Xp".to_owned(), Tag::Int(v.xp));
    if let Some(offers) = &e.offers {
        map.insert("Offers".to_owned(), offers_tag(offers));
    }
    let gossips = e
        .gossips
        .unpack()
        .into_iter()
        .map(|(target, kind, value)| {
            let entry = [
                ("Target".to_owned(), Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(target).to_vec())),
                ("Type".to_owned(), Tag::String(kind.id().to_owned())),
                ("Value".to_owned(), Tag::Int(value)),
            ];
            Tag::Compound(entry.into_iter().collect())
        })
        .collect();
    map.insert("Gossips".to_owned(), Tag::List(gossips));
    if let Some(item) = &e.held_item {
        let mut stack: std::collections::BTreeMap<String, Tag> = [("id".to_owned(), text_tag(&item.id)), ("count".to_owned(), Tag::Int(item.count))].into_iter().collect();
        if !item.components.is_empty() {
            stack.insert("components".to_owned(), json_tag(&serde_json::Value::Object(item.components.clone())));
        }
        map.insert("equipment".to_owned(), Tag::Compound([("mainhand".to_owned(), Tag::Compound(stack))].into_iter().collect()));
    }
    if let Some(effects) = world.mob_effects(villager).filter(|effects| !effects.is_empty()) {
        map.insert("active_effects".to_owned(), Tag::List(effects.iter().map(|e| effect_tag(e, true)).collect()));
    }
    Some(Tag::Compound(map))
}

pub fn cured_villager_tag(zombie: &Tag) -> (Tag, Vec<(String, i32, Option<String>)>) {
    const KEPT: [&str; 21] = [
        "Pos",
        "Motion",
        "Rotation",
        "OnGround",
        "fall_distance",
        "active_effects",
        "LeftHanded",
        "NoAI",
        "PersistenceRequired",
        "CustomName",
        "CustomNameVisible",
        "Invulnerable",
        "NoGravity",
        "PortalCooldown",
        "Silent",
        "Tags",
        "leash",
        "VillagerData",
        "Gossips",
        "Offers",
        "Xp",
    ];
    let Tag::Compound(map) = zombie else { return (Tag::Compound(std::collections::BTreeMap::new()), Vec::new()) };
    let mut out: std::collections::BTreeMap<String, Tag> = map.iter().filter(|(key, _)| KEPT.contains(&key.as_str())).map(|(key, value)| (key.clone(), value.clone())).collect();
    out.insert("id".to_owned(), Tag::String("minecraft:villager".to_owned()));
    let finalized = int(zombie, "VillagerDataFinalized").is_some_and(|f| f != 0) || zombie.get("VillagerData").is_some();
    out.insert("VillagerDataFinalized".to_owned(), Tag::Byte(i8::from(finalized)));
    let baby = int(zombie, "IsBaby").is_some_and(|b| b != 0);
    out.insert("Age".to_owned(), Tag::Int(if baby { -24000 } else { 0 }));
    let mut dropped = Vec::new();
    for slot in ["mainhand", "offhand", "feet", "legs", "chest", "head", "body", "saddle"] {
        let chance = zombie.get("drop_chances").and_then(|d| d.get(slot)).and_then(Tag::as_f64).unwrap_or(0.0);
        let Some(item) = zombie.get("equipment").and_then(|e| e.get(slot)).filter(|_| chance > 1.0) else { continue };
        let Some(id) = text(item, "id") else { continue };
        let components = item.get("components").map(|c| crate::item_nbt::tag_json(c).to_string());
        dropped.push((id.to_owned(), int(item, "count").unwrap_or(1), components));
    }
    (Tag::Compound(out), dropped)
}

/// The head, chest, legs and feet slots' item IDs from `equipment`.
fn armor_slots(tag: &Tag) -> [Option<String>; 4] {
    ["head", "chest", "legs", "feet"].map(|slot| tag.get("equipment")?.get(slot)?.get("id")?.as_str().map(str::to_owned))
}

fn head_item(tag: &Tag) -> Option<bool> {
    let id = tag.get("equipment")?.get("head")?.get("id")?.as_str()?;
    Some(id.ends_with("_helmet"))
}

/// Whether a position lies in a chunk.
pub fn in_chunk(position: DVec3, chunk: ChunkPos) -> bool {
    (position.x.floor() as i32) >> 4 == chunk.x && (position.z.floor() as i32) >> 4 == chunk.z
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strider_tag(fields: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(&format!("{{id:\"minecraft:strider\",Pos:[1.5d,64.5d,2.5d],Rotation:[90.0f,0.0f],{fields}}}")).unwrap()
    }

    #[test]
    fn a_strider_round_trips_with_its_saddle_age_and_love() {
        let tag = strider_tag("InLove:300,Health:13.0f,PersistenceRequired:1b,equipment:{saddle:{id:\"minecraft:saddle\",count:1},head:{id:\"minecraft:carved_pumpkin\",count:1}}");
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("striders load");
        let strider = &world.strider_mut(id).unwrap().strider;
        assert!(strider.saddled && strider.persistence_required);
        assert_eq!((strider.in_love, strider.health, strider.yaw), (300, 13.0, 90.0));
        assert_eq!(strider.body.position, DVec3::new(1.5, 64.5, 2.5));
        let originals: std::collections::HashMap<u64, Tag> = [(id, tag.clone())].into_iter().collect();
        let saved = mob_tags(&world, |_| true, &originals);
        let (_, back) = saved.iter().find(|(saved_id, _)| *saved_id == id).expect("the strider is saved");
        let equipment = back.get("equipment").expect("its equipment");
        assert_eq!(equipment.get("saddle").and_then(|s| s.get("id")).and_then(Tag::as_str), Some("minecraft:saddle"));
        assert_eq!(equipment.get("head").and_then(|s| s.get("id")).and_then(Tag::as_str), Some("minecraft:carved_pumpkin"), "other slots stay as loaded");
        assert_eq!(int(back, "InLove"), Some(300));
        let mut again = EntityWorld::default();
        let id_again = spawn_saved(&mut again, back).expect("it loads again");
        let strider = &again.strider_mut(id_again).unwrap().strider;
        assert!(strider.saddled);
        assert_eq!((strider.in_love, strider.health), (300, 13.0));
        world.strider_mut(id).unwrap().strider.saddled = false;
        let saved = mob_tags(&world, |_| true, &originals);
        let (_, back) = saved.iter().find(|(saved_id, _)| *saved_id == id).unwrap();
        assert!(back.get("equipment").and_then(|e| e.get("saddle")).is_none());
        assert!(back.get("equipment").and_then(|e| e.get("head")).is_some());
        let baby = strider_tag("Age:-1200");
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &baby).unwrap();
        let strider = &world.strider_mut(id).unwrap().strider;
        assert!(strider.age.baby() && !strider.saddled);
        assert_eq!((strider.body.width, strider.body.height), (0.45, 0.85));
        let saved = mob_tags(&world, |_| true, &std::collections::HashMap::new());
        assert_eq!(int(&saved[0].1, "Age"), Some(-1200));
        assert!(saved[0].1.get("equipment").is_none());
    }

    #[test]
    fn a_strider_jockey_round_trips_with_its_passenger() {
        let tag = strider_tag("UUID:[I;1,2,3,4],equipment:{saddle:{id:\"minecraft:saddle\",count:1}},Passengers:[{id:\"minecraft:zombified_piglin\",Pos:[1.5d,65.5d,2.5d],UUID:[I;5,6,7,8],equipment:{mainhand:{id:\"minecraft:warped_fungus_on_a_stick\",count:1}}},{id:\"minecraft:ghast\",Pos:[9.0d,70.0d,9.0d]}]");
        assert_eq!(spawn_saved(&mut EntityWorld::default(), &tag), None, "the strider alone refuses its riders");
        let mut world = EntityWorld::default();
        let spawned = spawn_saved_all(&mut world, &tag);
        assert_eq!(spawned.len(), 3);
        let (strider, rider, ghast) = (spawned[0].0, spawned[1].0, spawned[2].0);
        assert_eq!(world.passengers_of(strider), &[Rider::Mob(rider), Rider::Mob(ghast)]);
        assert_eq!(world.zombies().iter().find(|e| e.id == rider).unwrap().zombie.main_hand.as_deref(), Some("minecraft:warped_fungus_on_a_stick"));
        let seat = world.seat(Rider::Mob(rider)).unwrap();
        assert!((seat.y - 65.5).abs() < 1.0e-6, "{seat}");
        assert_eq!(world.body_mut(rider).unwrap().position, seat);
        let originals: std::collections::HashMap<u64, Tag> = spawned.into_iter().collect();
        let saved = mob_tags(&world, |_| true, &originals);
        assert_eq!(saved.len(), 1, "riders save inside their vehicle");
        let back = &saved[0].1;
        assert_eq!(text(back, "id"), Some("minecraft:strider"));
        let riders = back.get("Passengers").and_then(Tag::as_list).expect("its passengers");
        assert_eq!(riders.len(), 2);
        assert_eq!(text(&riders[0], "id"), Some("minecraft:zombified_piglin"));
        assert_eq!(riders[0].get("UUID").and_then(Tag::as_ints).map(|ints| ints.to_vec()), Some(vec![5, 6, 7, 8]));
        assert!(riders[0].get("Passengers").is_none());
        assert_eq!(riders[0].get("equipment").and_then(|e| e.get("mainhand")).and_then(|i| i.get("id")).and_then(Tag::as_str), Some("minecraft:warped_fungus_on_a_stick"));
        let mut again = EntityWorld::default();
        let reloaded = spawn_saved_all(&mut again, back);
        assert_eq!(reloaded.len(), 3);
        assert_eq!(again.vehicle_of(Rider::Mob(reloaded[1].0)), Some(reloaded[0].0));
        assert!(again.census().iter().all(|entry| entry.riding));
    }

    #[test]
    fn a_jockey_on_a_horse_loads_and_a_saddled_horse_keeps_its_saddle_armour_and_owner() {
        let horse = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:horse\",Pos:[1.5d,64.0d,2.5d],Tame:1b,Owner:[I;1,2,3,4],equipment:{saddle:{id:\"minecraft:saddle\",count:1},body:{id:\"minecraft:iron_horse_armor\",count:1}},Passengers:[{id:\"minecraft:skeleton\",Pos:[1.5d,65.0d,2.5d],UUID:[I;5,6,7,8]}]}").unwrap();
        assert!(carries(&horse), "a skeleton on a horse loads like a strider jockey");
        assert_eq!(spawn_saved(&mut EntityWorld::default(), &horse), None, "the horse alone refuses its rider");
        let mut world = EntityWorld::default();
        let spawned = spawn_saved_all(&mut world, &horse);
        assert_eq!(spawned.len(), 2);
        let (mount, rider) = (spawned[0].0, spawned[1].0);
        assert_eq!(world.entity_type(rider), Some("minecraft:skeleton"));
        assert_eq!(world.vehicle_of(Rider::Mob(rider)), Some(mount));
        assert_eq!(world.controlling_mob(mount), Some(rider));
        let h = world.cow_mut(mount).unwrap().horse.clone().unwrap();
        assert!(h.saddled && h.tame && h.body_item.as_deref() == Some("minecraft:iron_horse_armor"));
        assert_eq!(h.owner, Some(minecraftoss_entities::gossip::uuid_from_ints([1, 2, 3, 4])));
        let seat = world.seat(Rider::Mob(rider)).unwrap();
        assert!((seat.y - (64.0 + f64::from(1.44375_f32) - f64::from(0.7_f32))).abs() < 1.0e-6, "{seat}");
        {
            let h = world.cow_mut(mount).unwrap().horse.as_mut().unwrap();
            h.saddled = false;
            h.body_item = Some("minecraft:diamond_horse_armor".to_owned());
        }
        let originals: std::collections::HashMap<u64, Tag> = spawned.into_iter().collect();
        let saved = mob_tags(&world, |_| true, &originals);
        assert_eq!(saved.len(), 1, "the rider saves inside its horse");
        let back = &saved[0].1;
        let equipment = back.get("equipment").unwrap();
        assert!(equipment.get("saddle").is_none());
        assert_eq!(equipment.get("body").and_then(|b| b.get("id")).and_then(Tag::as_str), Some("minecraft:diamond_horse_armor"));
        assert_eq!(back.get("Owner").and_then(Tag::as_ints), Some(vec![1, 2, 3, 4]));
        assert_eq!(back.get("Passengers").and_then(Tag::as_list).map(|riders| riders.len()), Some(1));
        let llama = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:llama\",Pos:[1.5d,64.0d,2.5d],equipment:{saddle:{id:\"minecraft:saddle\",count:1}},Passengers:[{id:\"minecraft:zombie\",Pos:[1.5d,65.0d,2.5d]}]}").unwrap();
        let mut world = EntityWorld::default();
        let spawned = spawn_saved_all(&mut world, &llama);
        assert_eq!(spawned.len(), 2, "a zombie rides a llama too");
        assert!(!world.cow_mut(spawned[0].0).unwrap().horse.as_ref().unwrap().saddled, "a llama never takes a saddle");
    }

    #[test]
    fn a_spider_jockey_still_waits_dormant() {
        let spider = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:spider\",Pos:[1.5d,64.0d,2.5d],Passengers:[{id:\"minecraft:skeleton\"}]}").unwrap();
        assert!(spawn_saved_all(&mut EntityWorld::default(), &spider).is_empty());
        let strider = strider_tag("Passengers:[{id:\"minecraft:camel\"}]");
        assert!(spawn_saved_all(&mut EntityWorld::default(), &strider).is_empty(), "a rider it does not simulate keeps the whole group dormant");
    }

    #[test]
    fn a_hatched_jockey_stands_its_rider_beside_its_mount() {
        let spider = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:spider\",Pos:[1.5d,64.0d,2.5d],Passengers:[{id:\"minecraft:skeleton\",Pos:[1.5d,64.0d,2.5d]}]}").unwrap();
        let mut world = EntityWorld::default();
        let spawned = spawn_unmounted(&mut world, &spider);
        let kinds: Vec<_> = spawned.iter().map(|(id, _)| world.entity_type(*id)).collect();
        assert_eq!(kinds, [Some("minecraft:spider"), Some("minecraft:skeleton")]);
        assert!(spawned.iter().all(|(id, tag)| tag.get("Passengers").is_none() && world.vehicle_of(Rider::Mob(*id)).is_none()));
        let chicken = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:chicken\",Pos:[1.5d,64.0d,2.5d],Passengers:[{id:\"minecraft:zombie\",Pos:[1.5d,64.0d,2.5d],IsBaby:1b}]}").unwrap();
        let spawned = spawn_unmounted(&mut world, &chicken);
        let kinds: Vec<_> = spawned.iter().map(|(id, _)| world.entity_type(*id)).collect();
        assert_eq!(kinds, [Some("minecraft:chicken"), Some("minecraft:zombie")]);
        let strider = strider_tag("Passengers:[{id:\"minecraft:zombified_piglin\",Pos:[1.5d,65.5d,2.5d]}]");
        let spawned = spawn_unmounted(&mut world, &strider);
        assert_eq!(world.vehicle_of(Rider::Mob(spawned[1].0)), Some(spawned[0].0), "a strider still carries its rider");
        let horse = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:zombie_horse\",Pos:[1.5d,64.0d,2.5d],Passengers:[{id:\"minecraft:zombie\",Pos:[1.5d,64.0d,2.5d]}]}").unwrap();
        assert!(spawn_unmounted(&mut world, &horse).is_empty(), "without its mount the rider stays away too");
        let cave = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:cave_spider\",Pos:[1.5d,64.0d,2.5d],Health:7.0f}").unwrap();
        let spawned = spawn_unmounted(&mut world, &cave);
        assert_eq!(world.entity_type(spawned[0].0), Some("minecraft:cave_spider"));
        let saved = mob_tags(&world, |_| true, &spawned.into_iter().collect());
        let back = saved.iter().find(|(_, tag)| text(tag, "id") == Some("minecraft:cave_spider")).expect("it saves as a cave spider");
        assert_eq!(back.1.get("Health").and_then(Tag::as_f64), Some(7.0));
    }

    #[test]
    fn piglins_and_brutes_load_live_and_save_every_field() {
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:piglin\",Pos:[1.5d,64.0d,2.5d],Rotation:[45.0f,0.0f],IsBaby:1b,CannotHunt:1b,IsImmuneToZombification:1b,TimeInOverworld:120,CanPickUpLoot:0b,LeftHanded:1b,PersistenceRequired:1b,Health:12.0f,Inventory:[{id:\"minecraft:gold_nugget\",count:7}],equipment:{mainhand:{id:\"minecraft:crossbow\",count:1},offhand:{id:\"minecraft:gold_ingot\",count:1},head:{id:\"minecraft:golden_helmet\",count:1,components:{\"minecraft:damage\":5}}},drop_chances:{offhand:2.0f},Brain:{memories:{\"minecraft:hunted_recently\":{value:1b,ttl:1200L},\"minecraft:admiring_item\":{value:1b,ttl:90L},\"minecraft:admiring_disabled\":{value:1b,ttl:300L},\"minecraft:universal_anger\":{value:1b,ttl:500L},\"minecraft:angry_at\":{value:[I;1,2,3,4],ttl:550L}}},attributes:[{id:\"minecraft:max_health\",base:16.0d}]}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("piglins load");
        let entity = world.piglin_mut(id).unwrap();
        let piglin = &entity.piglin;
        assert!(piglin.is_baby() && piglin.cannot_hunt && piglin.immune_to_zombification && piglin.left_handed && piglin.persistence_required);
        assert!(!piglin.can_pick_up_loot);
        assert_eq!((piglin.time_in_overworld, piglin.health, piglin.yaw), (120, 12.0, 45.0));
        assert_eq!((piglin.body.width, piglin.body.height), (0.49, 0.98));
        assert_eq!(piglin.inventory[0].as_ref().map(|s| (s.id.as_str(), s.count)), Some(("minecraft:gold_nugget", 7)));
        assert_eq!(piglin.main_hand.as_ref().map(|s| s.id.as_str()), Some("minecraft:crossbow"));
        assert_eq!(piglin.offhand.as_ref().map(|s| s.id.as_str()), Some("minecraft:gold_ingot"));
        assert_eq!(piglin.armor[0].as_ref().and_then(|s| s.components.clone()), Some(serde_json::json!({"minecraft:damage": 5})));
        assert_eq!(piglin.drop_chances[minecraftoss_entities::piglin::OFFHAND], 2.0);
        let memories = &entity.brain.memories;
        assert_eq!(memories.hunted_recently.ttl(), Some(1200));
        assert_eq!(memories.admiring_item.ttl(), Some(90));
        assert_eq!(memories.admiring_disabled.ttl(), Some(300));
        assert_eq!(memories.universal_anger.ttl(), Some(500));
        assert_eq!((memories.angry_at.get().copied(), memories.angry_at.ttl()), (Some(minecraftoss_entities::gossip::uuid_from_ints([1, 2, 3, 4])), Some(550)));
        let originals: std::collections::HashMap<u64, Tag> = [(id, tag.clone())].into_iter().collect();
        let saved = mob_tags(&world, |_| true, &originals);
        let (_, back) = saved.iter().find(|(saved_id, _)| *saved_id == id).expect("the piglin is saved");
        assert_eq!(text(back, "id"), Some("minecraft:piglin"));
        assert_eq!((int(back, "IsBaby"), int(back, "CannotHunt"), int(back, "IsImmuneToZombification"), int(back, "TimeInOverworld"), int(back, "CanPickUpLoot")), (Some(1), Some(1), Some(1), Some(120), Some(0)));
        assert!(back.get("attributes").is_some(), "unknown keys stay as loaded");
        let memory = |key: &str| back.get("Brain").and_then(|b| b.get("memories")).and_then(|m| m.get(key)).and_then(|m| m.get("ttl")).and_then(Tag::as_i64);
        assert_eq!((memory("minecraft:hunted_recently"), memory("minecraft:angry_at")), (Some(1200), Some(550)));
        let mut again = EntityWorld::default();
        let id_again = spawn_saved(&mut again, back).expect("it loads again");
        let entity = again.piglin_mut(id_again).unwrap();
        assert_eq!(entity.piglin.inventory[0].as_ref().map(|s| s.count), Some(7));
        assert_eq!(entity.piglin.armor[0].as_ref().map(|s| s.id.as_str()), Some("minecraft:golden_helmet"));
        assert_eq!(entity.piglin.drop_chances[minecraftoss_entities::piglin::OFFHAND], 2.0);
        assert_eq!(entity.brain.memories.angry_at.ttl(), Some(550));
        assert_eq!(entity.brain.memories.admiring_item.ttl(), Some(90));
        let brute = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:piglin_brute\",Pos:[1.5d,64.0d,2.5d],equipment:{mainhand:{id:\"minecraft:golden_axe\",count:1}},Brain:{memories:{\"minecraft:home\":{value:{dimension:\"minecraft:the_nether\",pos:[I;3,64,5]}}}}}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &brute).expect("brutes load");
        let entity = world.piglin_mut(id).unwrap();
        assert_eq!(entity.piglin.kind, minecraftoss_entities::piglin::PiglinKind::Brute);
        assert_eq!(entity.piglin.health, 50.0);
        let home = entity.brain.memories.home.get().cloned().expect("its home");
        assert_eq!((home.dimension.as_str(), home.pos), ("minecraft:the_nether", (3, 64, 5)));
        let saved = mob_tags(&world, |_| true, &std::collections::HashMap::new());
        assert_eq!(text(&saved[0].1, "id"), Some("minecraft:piglin_brute"));
        assert!(saved[0].1.get("IsBaby").is_none() && saved[0].1.get("Inventory").is_none());
        let home = saved[0].1.get("Brain").and_then(|b| b.get("memories")).and_then(|m| m.get("minecraft:home")).and_then(|h| h.get("value")).expect("the home is saved");
        assert_eq!(home.get("pos").and_then(Tag::as_ints), Some(vec![3, 64, 5]));
        assert_eq!(saved[0].1.get("equipment").and_then(|e| e.get("mainhand")).and_then(|m| m.get("id")).and_then(Tag::as_str), Some("minecraft:golden_axe"));
        assert!(SIMULATED_KINDS.contains(&"minecraft:piglin") && SIMULATED_KINDS.contains(&"minecraft:piglin_brute"));
    }

    #[test]
    fn hoglins_and_zoglins_keep_their_age_conversion_clock_and_flags_through_a_save() {
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:hoglin\",Pos:[1.5d,64.0d,2.5d],Rotation:[45.0f,0.0f],Age:-1200,ForcedAge:7,InLove:0,IsImmuneToZombification:1b,TimeInOverworld:200,CannotBeHunted:1b,PersistenceRequired:1b,Health:31.0f,attributes:[{id:\"minecraft:attack_damage\",base:0.5d}]}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("hoglins load");
        let hoglin = &world.hoglin_mut(id).unwrap().hoglin;
        assert!(hoglin.is_baby() && hoglin.immune_to_zombification && hoglin.cannot_be_hunted && hoglin.persistence_required);
        assert_eq!((hoglin.age.ticks, hoglin.age.forced, hoglin.time_in_overworld, hoglin.health, hoglin.yaw), (-1200, 7, 200, 31.0, 45.0));
        assert_eq!((hoglin.body.width, hoglin.body.height), (0.75, 0.85));
        let originals: std::collections::HashMap<u64, Tag> = [(id, tag.clone())].into_iter().collect();
        let saved = mob_tags(&world, |_| true, &originals);
        let (_, back) = saved.iter().find(|(saved_id, _)| *saved_id == id).expect("the hoglin is saved");
        assert_eq!(text(back, "id"), Some("minecraft:hoglin"));
        assert_eq!((int(back, "Age"), int(back, "TimeInOverworld"), int(back, "IsImmuneToZombification"), int(back, "CannotBeHunted")), (Some(-1200), Some(200), Some(1), Some(1)));
        assert!(back.get("attributes").is_some(), "unknown keys stay as loaded");
        let mut again = EntityWorld::default();
        let id_again = spawn_saved(&mut again, back).expect("it loads again");
        let hoglin = &again.hoglin_mut(id_again).unwrap().hoglin;
        assert_eq!((hoglin.age.ticks, hoglin.time_in_overworld, hoglin.health), (-1200, 200, 31.0));
        assert!(hoglin.immune_to_zombification && hoglin.cannot_be_hunted);
        let zoglin = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:zoglin\",Pos:[1.5d,64.0d,2.5d],IsBaby:1b}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &zoglin).expect("zoglins load");
        assert!(world.hoglin_mut(id).unwrap().hoglin.is_baby());
        let saved = mob_tags(&world, |_| true, &std::collections::HashMap::new());
        assert_eq!(text(&saved[0].1, "id"), Some("minecraft:zoglin"));
        assert_eq!(int(&saved[0].1, "IsBaby"), Some(1));
        assert!(saved[0].1.get("Age").is_none() && saved[0].1.get("TimeInOverworld").is_none());
        assert!(SIMULATED_KINDS.contains(&"minecraft:hoglin") && SIMULATED_KINDS.contains(&"minecraft:zoglin"));
    }

    #[test]
    fn the_clock_survives_a_save() {
        struct Floor;
        impl PlayerWorld for Floor {
            fn block(&self, pos: (i32, i32, i32)) -> Option<PlayerBlock> {
                (pos.1 < 64).then(|| PlayerBlock::new("minecraft:netherrack"))
            }
            fn set_block(&mut self, _pos: (i32, i32, i32), _block: Option<PlayerBlock>) {}
        }
        let mut world = EntityWorld::default();
        world.set_piglins_zombify(true);
        let hoglin = minecraftoss_entities::hoglin::Hoglin::new(minecraftoss_entities::hoglin::HoglinKind::Hoglin, DVec3::new(0.5, 64.0, 0.5));
        let id = world.spawn_hoglin(hoglin, false);
        for _ in 0..200 {
            world.tick_with_players(&mut Floor, &[]);
        }
        let saved = mob_tags(&world, |_| true, &std::collections::HashMap::new());
        let (_, tag) = saved.iter().find(|(saved_id, _)| *saved_id == id).expect("the hoglin is saved");
        assert_eq!(int(tag, "TimeInOverworld"), Some(200));
        let mut again = EntityWorld::default();
        again.set_piglins_zombify(true);
        let id = spawn_saved(&mut again, tag).expect("it loads again");
        for _ in 0..100 {
            again.tick_with_players(&mut Floor, &[]);
        }
        assert!(again.hoglin_mut(id).is_some());
        again.tick_with_players(&mut Floor, &[]);
        assert!(again.hoglin_mut(id).is_none());
        assert_eq!(again.hoglins().len(), 1);
        assert_eq!(again.entity_type(again.hoglins()[0].id), Some("minecraft:zoglin"));
    }

    #[test]
    fn a_drowned_saves_and_loads_as_a_drowned() {
        let zombie = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:zombie\",Pos:[1.5d,64.0d,2.5d],UUID:[I;1,2,3,4],CustomName:\"Bob\",InWaterTime:600,attributes:[{id:\"minecraft:max_health\",base:20.0d}],equipment:{mainhand:{id:\"minecraft:trident\",count:1,components:{\"minecraft:damage\":7}},head:{id:\"minecraft:iron_helmet\",count:1}}}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &zombie).expect("zombies load");
        let drowned = world.convert_zombie_to_drowned(id);
        assert_eq!(world.take_zombie_conversions(), vec![(id, drowned)]);
        assert!(world.take_zombie_conversions().is_empty());
        world.zombie_mut(drowned).unwrap().zombie.health = 11.0;
        assert_eq!(world.zombie_mut(drowned).unwrap().zombie.armor[0].as_deref(), Some("minecraft:iron_helmet"));
        let originals: std::collections::HashMap<u64, Tag> = [(drowned, converted_zombie_tag(&zombie))].into_iter().collect();
        let saved = mob_tags_where(&world, |_, _| true, &originals, true);
        assert_eq!(saved.len(), 1);
        let tag = &saved[0].1;
        assert_eq!(text(tag, "id"), Some("minecraft:drowned"));
        assert_eq!(text(tag, "CustomName"), Some("Bob"));
        assert!(tag.get("InWaterTime").is_none() && tag.get("attributes").is_none());
        assert_ne!(tag.get("UUID").and_then(Tag::as_ints), Some(vec![1, 2, 3, 4]), "a conversion is a new entity");
        let hand = tag.get("equipment").and_then(|e| e.get("mainhand")).expect("the trident is saved");
        assert_eq!(text(hand, "id"), Some("minecraft:trident"));
        assert_eq!(hand.get("components").and_then(|c| c.get("minecraft:damage")).and_then(Tag::as_i64), Some(7));
        assert!(SIMULATED_KINDS.contains(&"minecraft:drowned"));
        let mut fresh = EntityWorld::default();
        let loaded = spawn_saved(&mut fresh, tag).expect("a drowned loads");
        assert_eq!(fresh.entity_type(loaded), Some("minecraft:drowned"));
        let e = fresh.zombie_mut(loaded).unwrap();
        assert_eq!(e.zombie.kind, ZombieKind::Drowned);
        assert_eq!(e.zombie.health, 11.0);
        assert_eq!(e.zombie.main_hand.as_deref(), Some("minecraft:trident"));
        assert_eq!(e.zombie.armor[0].as_deref(), Some("minecraft:iron_helmet"));
        assert_eq!(e.zombie.body.step_height, 1.0);
        assert!(e.attack_only && e.ai.is_none() && !e.no_ai, "a loaded drowned fights with its attack goal");
        assert!(e.drowned.is_some(), "and has its water, beach and trident goals");
    }

    #[test]
    fn a_cat_and_an_ocelot_save_and_load_with_variant_collar_owner_sitting_and_trust() {
        let cat = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:cat\",Pos:[1.5d,64.0d,2.5d],UUID:[I;5,6,7,8],variant:\"minecraft:siamese\",sound_variant:\"minecraft:royal\",CollarColor:3b,Owner:[I;1,2,3,4],Sitting:1b,Health:7.0f,Age:-1200,PersistenceRequired:1b}").unwrap();
        let ocelot = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:ocelot\",Pos:[4.5d,64.0d,2.5d],Trusting:1b}").unwrap();
        assert!(SIMULATED_KINDS.contains(&"minecraft:cat") && SIMULATED_KINDS.contains(&"minecraft:ocelot"));
        let mut world = EntityWorld::default();
        let cat_id = spawn_saved(&mut world, &cat).expect("cats load");
        let ocelot_id = spawn_saved(&mut world, &ocelot).expect("ocelots load");
        assert_eq!((world.entity_type(cat_id), world.entity_type(ocelot_id)), (Some("minecraft:cat"), Some("minecraft:ocelot")));
        let originals: std::collections::HashMap<u64, Tag> = [(cat_id, cat), (ocelot_id, ocelot)].into_iter().collect();
        let saved = mob_tags_where(&world, |_, _| true, &originals, true);
        let tag_of = |id: u64| saved.iter().find(|(saved_id, _)| *saved_id == id).map(|(_, tag)| tag.clone()).unwrap();
        let cat_tag = tag_of(cat_id);
        assert_eq!((text(&cat_tag, "variant"), text(&cat_tag, "sound_variant")), (Some("minecraft:siamese"), Some("minecraft:royal")));
        assert_eq!((int(&cat_tag, "CollarColor"), int(&cat_tag, "Sitting"), int(&cat_tag, "Age")), (Some(3), Some(1), Some(-1200)));
        assert_eq!(cat_tag.get("Owner").and_then(Tag::as_ints), Some(vec![1, 2, 3, 4]));
        assert_eq!(int(&tag_of(ocelot_id), "Trusting"), Some(1));
        let mut fresh = EntityWorld::default();
        let loaded = spawn_saved(&mut fresh, &cat_tag).expect("a saved cat loads");
        let e = &fresh.feline_mut(loaded).unwrap().feline;
        assert_eq!((e.variant.as_str(), e.sound_variant.as_str(), e.collar, e.health), ("siamese", "royal", 3, 7.0));
        assert!(e.tame && e.ordered_to_sit && e.sitting && e.baby() && e.persistence_required);
        assert_eq!(e.owner, Some(minecraftoss_entities::gossip::uuid_from_ints([1, 2, 3, 4])));
        let loaded = spawn_saved(&mut fresh, &tag_of(ocelot_id)).expect("a saved ocelot loads");
        let e = &fresh.feline_mut(loaded).unwrap().feline;
        assert!(e.trusting && !e.tame && e.kind == minecraftoss_entities::feline::FelineKind::Ocelot);
    }

    #[test]
    fn a_mule_a_llama_and_a_trader_llama_save_and_load_with_variant_strength_carpet_and_chest() {
        use minecraftoss_entities::horse::HorseKind;
        let mule = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:mule\",Pos:[1.5d,64.0d,2.5d],ChestedHorse:1b,Tame:1b,Temper:40,Items:[{Slot:2b,id:\"minecraft:diamond\",count:3}],attributes:[{id:\"minecraft:max_health\",base:22.0d}]}").unwrap();
        let llama = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:llama\",Pos:[4.5d,64.0d,2.5d],Variant:2,Strength:4,Tame:1b,ChestedHorse:0b,equipment:{body:{id:\"minecraft:red_carpet\",count:1},head:{id:\"minecraft:carved_pumpkin\",count:1}},Age:-500}").unwrap();
        let trader = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:trader_llama\",Pos:[7.5d,64.0d,2.5d],Variant:9,Strength:0,DespawnDelay:1200}").unwrap();
        for kind in ["minecraft:mule", "minecraft:llama", "minecraft:trader_llama"] {
            assert!(SIMULATED_KINDS.contains(&kind), "{kind}");
        }
        let mut world = EntityWorld::default();
        let ids: Vec<u64> = [&mule, &llama, &trader].iter().map(|tag| spawn_saved(&mut world, tag).expect("it loads")).collect();
        let types: Vec<_> = ids.iter().map(|&id| world.entity_type(id)).collect();
        assert_eq!(types, [Some("minecraft:mule"), Some("minecraft:llama"), Some("minecraft:trader_llama")]);
        let horse_of = |world: &mut EntityWorld, id: u64| world.cow_mut(id).unwrap().horse.clone().unwrap();
        let m = horse_of(&mut world, ids[0]);
        assert!(m.chest && m.tame && m.temper == 40 && m.max_health == 22.0);
        assert_eq!(m.inventory.iter().map(|s| s.as_ref().map(|s| (s.id.as_str(), s.count))).collect::<Vec<_>>()[..3], [None, None, Some(("minecraft:diamond", 3))]);
        assert_eq!(m.inventory.len(), 15);
        let l = horse_of(&mut world, ids[1]);
        assert_eq!((l.kind, l.variant, l.strength, l.body_item.as_deref()), (HorseKind::Llama, 2, 4, Some("minecraft:red_carpet")));
        let t = horse_of(&mut world, ids[2]);
        assert_eq!((t.variant, t.strength, t.despawn_delay), (3, 1, 1200));
        {
            let e = world.cow_mut(ids[1]).unwrap();
            let h = e.horse.as_mut().unwrap();
            h.chest = true;
            h.body_item = Some("minecraft:blue_carpet".to_owned());
        }
        world.cow_mut(ids[2]).unwrap().horse.as_mut().unwrap().body_item = Some("minecraft:lime_carpet".to_owned());
        world.cow_mut(ids[0]).unwrap().horse.as_mut().unwrap().inventory[4] = Some(minecraftoss_player::inventory::ItemStack::new("minecraft:apple", 5));
        let originals: std::collections::HashMap<u64, Tag> = ids.iter().copied().zip([mule, llama, trader]).collect();
        let saved = mob_tags_where(&world, |_, _| true, &originals, true);
        let tag_of = |id: u64| saved.iter().find(|(saved_id, _)| *saved_id == id).map(|(_, tag)| tag.clone()).unwrap();
        let mule_tag = tag_of(ids[0]);
        assert_eq!((int(&mule_tag, "ChestedHorse"), int(&mule_tag, "Temper"), int(&mule_tag, "Tame")), (Some(1), Some(40), Some(1)));
        let items = mule_tag.get("Items").and_then(Tag::as_list).expect("the chest's items stay");
        let slots: Vec<_> = items.iter().map(|item| (int(item, "Slot"), text(item, "id"), int(item, "count"))).collect();
        assert_eq!(slots, [(Some(2), Some("minecraft:diamond"), Some(3)), (Some(4), Some("minecraft:apple"), Some(5))]);
        let llama_tag = tag_of(ids[1]);
        assert_eq!((int(&llama_tag, "Variant"), int(&llama_tag, "Strength"), int(&llama_tag, "ChestedHorse"), int(&llama_tag, "Age")), (Some(2), Some(4), Some(1), Some(-500)));
        let equipment = llama_tag.get("equipment").unwrap();
        assert_eq!(equipment.get("body").and_then(|b| b.get("id")).and_then(Tag::as_str), Some("minecraft:blue_carpet"));
        assert_eq!(equipment.get("head").and_then(|b| b.get("id")).and_then(Tag::as_str), Some("minecraft:carved_pumpkin"), "other slots stay");
        let trader_tag = tag_of(ids[2]);
        assert_eq!((int(&trader_tag, "DespawnDelay"), int(&trader_tag, "Variant")), (Some(1200), Some(3)));
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, &llama_tag).expect("a saved llama loads");
        let l = fresh.cow_mut(back).unwrap();
        assert!(l.cow.age.baby());
        let l = l.horse.clone().unwrap();
        assert_eq!((l.variant, l.strength, l.chest, l.body_item.as_deref()), (2, 4, true, Some("minecraft:blue_carpet")));
        let back = spawn_saved(&mut fresh, &trader_tag).expect("a saved trader llama loads");
        assert_eq!(fresh.cow_mut(back).unwrap().horse.as_ref().unwrap().body_item.as_deref(), Some("minecraft:lime_carpet"));
    }

    #[test]
    fn llama_spit_in_flight_saves_and_loads_with_its_owner() {
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:llama_spit\",Pos:[1.5d,65.0d,2.5d],Motion:[0.0d,0.1d,1.2d],Rotation:[10.0f,-5.0f],Owner:[I;1,2,3,4],UUID:[I;9,9,9,9],LeftOwner:1b}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("spit loads");
        assert_eq!(world.entity_type(id), Some("minecraft:llama_spit"));
        let saved = mob_tags_where(&world, |_, _| true, &std::collections::HashMap::new(), true);
        let back = &saved.iter().find(|(saved_id, _)| *saved_id == id).unwrap().1;
        assert_eq!(back.get("Owner").and_then(Tag::as_ints), Some(vec![1, 2, 3, 4]));
        assert_eq!(back.get("UUID").and_then(Tag::as_ints), Some(vec![9, 9, 9, 9]));
        assert_eq!(int(back, "LeftOwner"), Some(1));
        let spit = &world.llama_spits()[0].spit;
        assert_eq!((spit.velocity.z, spit.yaw, spit.pitch), (1.2, 10.0, -5.0));
    }

    #[test]
    fn a_drowned_loads_its_nautilus_shell_with_its_guaranteed_drop() {
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:drowned\",Pos:[1.5d,64.0d,2.5d],equipment:{mainhand:{id:\"minecraft:fishing_rod\",count:1},offhand:{id:\"minecraft:nautilus_shell\",count:1}},drop_chances:{offhand:2.0f}}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("a drowned loads");
        let zombie = &world.zombie_mut(id).unwrap().zombie;
        assert_eq!((zombie.main_hand.as_deref(), zombie.off_hand.as_deref()), (Some("minecraft:fishing_rod"), Some("minecraft:nautilus_shell")));
        assert_eq!(zombie.hand_drop_chances, [0.085, 2.0]);
    }

    #[test]
    fn a_wandering_trader_saves_and_loads_its_stay_wander_target_home_offers_and_potion() {
        use minecraftoss_entities::effects::MobEffect;
        use minecraftoss_entities::wandering_trader::Drink;
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:wandering_trader\",Pos:[1.5d,64.0d,2.5d],UUID:[I;1,2,3,9],DespawnDelay:47000,wander_target:[I;10,64,-3],home_pos:[I;10,64,-3],home_radius:16,Health:15.0f,active_effects:[{id:\"minecraft:invisibility\",duration:900,amplifier:0b}],Offers:{Recipes:[{buy:{id:\"minecraft:emerald\",count:5},sell:{id:\"minecraft:blue_ice\",count:1},maxUses:6,uses:2,xp:1,priceMultiplier:0.05f}]}}").unwrap();
        assert!(SIMULATED_KINDS.contains(&"minecraft:wandering_trader"));
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("wandering traders load");
        assert_eq!(world.entity_type(id), Some("minecraft:wandering_trader"));
        {
            let e = world.wandering_trader_mut(id).unwrap();
            assert_eq!((e.trader.despawn_delay, e.trader.wander_target, e.trader.home, e.trader.health), (47000, Some((10, 64, -3)), Some(((10, 64, -3), 16)), 15.0));
            assert!(e.invisible(), "the potion it drank stays on");
            assert_eq!(e.offers.as_ref().map(Vec::len), Some(1));
            e.trader.held = Some(Drink::Milk);
        }
        let originals: std::collections::HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let saved = mob_tags_where(&world, |_, _| true, &originals, true);
        let saved = &saved[0].1;
        assert_eq!((int(saved, "DespawnDelay"), int(saved, "home_radius")), (Some(47000), Some(16)));
        assert_eq!(saved.get("wander_target").and_then(Tag::as_ints), Some(vec![10, 64, -3]));
        assert_eq!(saved.get("equipment").and_then(|e| e.get("mainhand")).and_then(|m| text(m, "id")), Some("minecraft:milk_bucket"));
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, saved).expect("a saved trader loads");
        let e = fresh.wandering_trader_mut(back).unwrap();
        assert_eq!((e.trader.despawn_delay, e.trader.wander_target, e.trader.home), (47000, Some((10, 64, -3)), Some(((10, 64, -3), 16))));
        assert!(e.effects.get(MobEffect::Invisibility).is_some());
        let offer = &e.offers.as_ref().unwrap()[0];
        assert_eq!((offer.sell.id.as_str(), offer.buy.count, offer.uses), ("minecraft:blue_ice", 5, 2));
        e.trader.wander_target = None;
        e.trader.home = None;
        e.trader.held = None;
        let originals: std::collections::HashMap<u64, Tag> = [(back, saved.clone())].into_iter().collect();
        let again = mob_tags_where(&fresh, |_, _| true, &originals, true);
        let again = &again[0].1;
        assert!(again.get("wander_target").is_none() && again.get("home_pos").is_none() && again.get("home_radius").is_none(), "{again:?}");
        assert!(again.get("equipment").and_then(|e| e.get("mainhand")).is_none(), "the empty hand saves no bucket");
    }

    #[test]
    fn a_vindicator_saves_and_loads_with_its_axe_and_stays_johnny() {
        assert!(SIMULATED_KINDS.contains(&"minecraft:vindicator"));
        let named = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:vindicator\",Pos:[1.5d,64.0d,2.5d],UUID:[I;4,5,6,9],Health:22.0f,CustomName:\"Johnny\",CanJoinRaid:1b,equipment:{mainhand:{id:\"minecraft:iron_axe\",count:1}}}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &named).expect("vindicators load");
        assert_eq!(world.entity_type(id), Some("minecraft:vindicator"));
        {
            let i = &world.illager_mut(id).unwrap().illager;
            assert!(i.johnny, "named Johnny");
            assert_eq!(i.health, 22.0);
            assert_eq!(i.main_hand().map(|s| s.id.as_str()), Some("minecraft:iron_axe"));
            assert!(!i.can_pick_up_loot);
        }
        let originals: std::collections::HashMap<u64, Tag> = [(id, named)].into_iter().collect();
        let saved = mob_tags_where(&world, |_, _| true, &originals, true);
        let saved = &saved[0].1;
        assert_eq!(saved.get("Johnny"), Some(&Tag::Byte(1)));
        let mut unnamed = saved.clone();
        if let Tag::Compound(map) = &mut unnamed {
            map.remove("CustomName");
        }
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, &unnamed).expect("a saved vindicator loads");
        assert!(fresh.illager_mut(back).unwrap().illager.johnny, "the Johnny flag is saved");
        let plain = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:vindicator\",Pos:[1.5d,64.0d,2.5d],CustomName:{text:\"Bob\"}}").unwrap();
        let id = spawn_saved(&mut fresh, &plain).unwrap();
        assert!(!fresh.illager_mut(id).unwrap().illager.johnny);
    }

    #[test]
    fn an_evoker_saves_and_loads_mid_spell() {
        assert!(SIMULATED_KINDS.contains(&"minecraft:evoker"));
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:evoker\",Pos:[1.5d,64.0d,2.5d],UUID:[I;4,5,6,10],Health:20.0f,SpellTicks:37,CanJoinRaid:1b}").unwrap();
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("evokers load");
        assert_eq!(world.entity_type(id), Some("minecraft:evoker"));
        let i = &world.illager_mut(id).unwrap().illager;
        assert_eq!((i.spell_casting_ticks, i.health), (37, 20.0));
        assert!(i.raider.can_join_raid);
        let saved = mob_tags_where(&world, |_, _| true, &Default::default(), true);
        assert_eq!(saved[0].1.get("SpellTicks"), Some(&Tag::Int(37)));
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, &saved[0].1).expect("a saved evoker loads");
        assert_eq!(fresh.illager_mut(back).unwrap().illager.spell_casting_ticks, 37);
    }

    #[test]
    fn evoker_fangs_save_and_load_with_their_warmup_and_owner() {
        let mut world = EntityWorld::default();
        let evoker = spawn_saved(&mut world, &minecraftoss_core::snbt::parse_compound("{id:\"minecraft:evoker\",Pos:[1.5d,64.0d,2.5d],UUID:[I;4,5,6,11]}").unwrap()).unwrap();
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:evoker_fangs\",Pos:[3.25d,64.0d,2.5d],Rotation:[45.0f,0.0f],Warmup:6,Owner:[I;4,5,6,11],UUID:[I;1,2,3,4]}").unwrap();
        let fangs = spawn_saved(&mut world, &tag).expect("fangs load");
        assert_eq!(world.entity_type(fangs), Some("minecraft:evoker_fangs"));
        let saved: Vec<_> = mob_tags_where(&world, |_, _| true, &Default::default(), true).into_iter().filter(|(id, _)| *id == fangs).collect();
        let saved = &saved[0].1;
        assert_eq!(saved.get("Warmup"), Some(&Tag::Int(6)));
        assert_eq!(saved.get("Owner"), Some(&Tag::IntArray(vec![4, 5, 6, 11])));
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, saved).expect("saved fangs load");
        let f = &fresh.evoker_fangs_mut(back).unwrap().fangs;
        assert_eq!((f.warmup, f.yaw), (6, 45.0));
        assert_eq!(f.owner_uuid, Some(world.uuid_of(evoker)));
    }

    #[test]
    fn a_summoned_vex_saves_and_loads_with_its_owner_origin_and_life() {
        assert!(SIMULATED_KINDS.contains(&"minecraft:vex"));
        let mut world = EntityWorld::default();
        let evoker = spawn_saved(&mut world, &minecraftoss_core::snbt::parse_compound("{id:\"minecraft:evoker\",Pos:[1.5d,64.0d,2.5d],UUID:[I;4,5,6,12]}").unwrap()).unwrap();
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:vex\",Pos:[2.5d,66.0d,2.5d],UUID:[I;4,5,6,13],Health:12.0f,LeftHanded:1b,bound_pos:[I;2,65,2],life_ticks:900,owner:[I;4,5,6,12],equipment:{mainhand:{id:\"minecraft:iron_sword\",count:1}},drop_chances:{mainhand:0.0f}}").unwrap();
        let id = spawn_saved(&mut world, &tag).expect("vexes load");
        assert_eq!(world.entity_type(id), Some("minecraft:vex"));
        {
            let owner = world.uuid_of(evoker);
            let v = &world.vex_mut(id).unwrap().vex;
            assert_eq!((v.health, v.life_ticks, v.bound_origin), (12.0, Some(900), Some((2, 65, 2))));
            assert!(v.left_handed);
            assert_eq!(v.owner_uuid, Some(owner));
            assert_eq!(v.main_hand().map(|s| s.id.as_str()), Some("minecraft:iron_sword"));
            assert_eq!(v.drop_chances[0], 0.0);
        }
        let saved: Vec<_> = mob_tags_where(&world, |_, _| true, &Default::default(), true).into_iter().filter(|(i, _)| *i == id).collect();
        let saved = &saved[0].1;
        assert_eq!(saved.get("life_ticks"), Some(&Tag::Int(900)));
        assert_eq!(saved.get("bound_pos"), Some(&Tag::IntArray(vec![2, 65, 2])));
        assert_eq!(saved.get("owner"), Some(&Tag::IntArray(vec![4, 5, 6, 12])));
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, saved).expect("a saved vex loads");
        assert_eq!(fresh.vex_mut(back).unwrap().vex.life_ticks, Some(900));
        let wild = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:vex\",Pos:[2.5d,66.0d,2.5d]}").unwrap();
        let id = spawn_saved(&mut fresh, &wild).unwrap();
        assert_eq!(fresh.vex_mut(id).unwrap().vex.life_ticks, None, "a vex from an egg lives on");
    }

    #[test]
    fn a_pillager_saves_and_loads_with_its_crossbow_and_raider_state() {
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:pillager\",Pos:[1.5d,64.0d,2.5d],UUID:[I;4,5,6,8],Health:20.0f,Rotation:[45.0f,0.0f],LeftHanded:1b,CanJoinRaid:1b,Wave:3,PatrolLeader:1b,Patrolling:1b,patrol_target:[I;10,64,-20],equipment:{mainhand:{id:\"minecraft:crossbow\",count:1}},drop_chances:{mainhand:0.5f},Inventory:[{id:\"minecraft:white_banner\",count:1}]}").unwrap();
        assert!(SIMULATED_KINDS.contains(&"minecraft:pillager"));
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("pillagers load");
        assert_eq!(world.entity_type(id), Some("minecraft:pillager"));
        {
            let e = world.illager_mut(id).unwrap();
            let i = &e.illager;
            assert_eq!((i.health, i.yaw, i.left_handed, i.can_pick_up_loot), (20.0, 45.0, true, true));
            assert!(i.holds_crossbow() && !i.crossbow_charged());
            assert_eq!(i.drop_chances[0], 0.5);
            assert_eq!(i.inventory[0].as_ref().map(|s| s.id.as_str()), Some("minecraft:white_banner"));
            let r = &i.raider;
            assert_eq!((r.wave, r.can_join_raid, r.patrol_leader, r.patrolling, r.patrol_target), (3, true, true, true, Some((10, 64, -20))));
            e.illager.set_crossbow_charged(true);
        }
        let originals: std::collections::HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let saved = mob_tags_where(&world, |_, _| true, &originals, true);
        let saved = &saved[0].1;
        assert_eq!(saved.get("PatrolLeader"), Some(&Tag::Byte(1)));
        assert_eq!(saved.get("patrol_target"), Some(&Tag::IntArray(vec![10, 64, -20])));
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, saved).expect("a saved pillager loads");
        let e = fresh.illager_mut(back).unwrap();
        assert!(e.illager.crossbow_charged(), "a loaded crossbow stays loaded");
        assert_eq!((e.illager.health, e.illager.raider.wave), (20.0, 3));
        assert_eq!(e.illager.drop_chances[0], 0.5);
    }

    #[test]
    fn a_snow_golem_saves_and_loads_with_its_pumpkin_and_health() {
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:snow_golem\",Pos:[1.5d,64.0d,2.5d],UUID:[I;4,5,6,7],Health:3.0f,Rotation:[90.0f,0.0f]}").unwrap();
        assert!(SIMULATED_KINDS.contains(&"minecraft:snow_golem"));
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("snow golems load");
        assert_eq!(world.entity_type(id), Some("minecraft:snow_golem"));
        {
            let e = world.snow_golem_mut(id).unwrap();
            assert!(e.golem.pumpkin, "a golem saved without the field wears its pumpkin");
            assert_eq!((e.golem.health, e.golem.yaw), (3.0, 90.0));
            e.golem.pumpkin = false;
        }
        let originals: std::collections::HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let saved = mob_tags_where(&world, |_, _| true, &originals, true);
        let saved = &saved[0].1;
        assert_eq!(saved.get("Pumpkin"), Some(&Tag::Byte(0)));
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, saved).expect("a saved snow golem loads");
        let e = fresh.snow_golem_mut(back).unwrap();
        assert!(!e.golem.pumpkin, "a sheared golem stays bare");
        assert_eq!(e.golem.health, 3.0);
    }

    #[test]
    fn a_leashed_cow_saves_and_loads_its_leash_to_a_player_a_mob_or_a_knot() {
        use minecraftoss_entities::world::{LeashHolder, SavedLeash};
        let to_knot = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:cow\",Pos:[1.5d,64.0d,2.5d],leash:[I;3,64,-2]}").unwrap();
        let to_player = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:cow\",Pos:[4.5d,64.0d,2.5d],leash:{UUID:[I;1,2,3,4]}}").unwrap();
        let loose = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:pig\",Pos:[6.5d,64.0d,2.5d],leash:{UUID:[I;9,9,9,9]}}").unwrap();
        let mut world = EntityWorld::default();
        let ids: Vec<u64> = [&to_knot, &to_player, &loose].iter().map(|tag| spawn_saved(&mut world, tag).expect("it loads")).collect();
        assert_eq!(world.saved_leash(ids[0]), Some(SavedLeash::Knot((3, 64, -2))));
        let player_uuid = minecraftoss_entities::gossip::uuid_from_ints([1, 2, 3, 4]);
        assert_eq!(world.saved_leash(ids[1]), Some(SavedLeash::Uuid(player_uuid)));
        world.set_leash(ids[1], LeashHolder::Mob(ids[0]));
        world.drop_leash(ids[2], false);
        world.set_leash(ids[2], LeashHolder::Mob(ids[0]));
        world.drop_leash(ids[2], false);
        let originals: std::collections::HashMap<u64, Tag> = ids.iter().copied().zip([to_knot, to_player, loose]).collect();
        let saved = mob_tags_where(&world, |_, _| true, &originals, true);
        let tag_of = |id: u64| saved.iter().find(|(saved_id, _)| *saved_id == id).map(|(_, tag)| tag.clone()).unwrap();
        assert_eq!(tag_of(ids[0]).get("leash"), Some(&Tag::IntArray(vec![3, 64, -2])));
        let to_mob = tag_of(ids[1]);
        let holder = to_mob.get("leash").and_then(|l| l.get("UUID")).and_then(Tag::as_ints).unwrap();
        assert_eq!(minecraftoss_entities::gossip::uuid_from_ints(<[i32; 4]>::try_from(holder).unwrap()), world.uuid_of(ids[0]));
        assert!(tag_of(ids[2]).get("leash").is_none(), "an untied pig saves no leash");
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, &tag_of(ids[0])).unwrap();
        assert_eq!(fresh.saved_leash(back), Some(SavedLeash::Knot((3, 64, -2))));
        assert_eq!(saved_leash_of(&leash_tag(SavedLeash::Uuid(player_uuid))), Some(SavedLeash::Uuid(player_uuid)));
    }
}
