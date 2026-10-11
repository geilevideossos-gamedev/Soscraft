//! Streamed, generated terrain: the client half of the integrated server.
//!
//! `minecraftoss_world::ChunkMap` decides which chunks exist and sends them
//! nearest-first. This module applies those packets to the client scene,
//! lights each chunk once its neighbors are present, and builds 16x16x16
//! render sections on worker threads in the order vanilla 26.3 would (see
//! `sections.rs`): the occlusion graph grows from the camera as sections
//! compile, only frustum-visible dirty sections are built, nearest first,
//! and each new section fades in from fog color.
//!
//! Meshing reads a dense copy of a section and its one-block border and
//! resolves models once per block state, then emits faces through the same
//! `mesh::append_block` as the authored scene, so both look identical.

use crate::frame_spans::span;
use crate::lighting::SkyLight;
use crate::mesh::{self, Atlas, BiomeTint, ChunkMesh};
use crate::model::{resolve_block_variants, ResolvedModel};
use crate::pack::{PackStack, ResourceId};
use crate::scene::{BiomeSample, Block, BlockPos, ChunkPos, HandcraftedScene, Scene};
use crate::sections::{CompileQueue, CullCamera, SectionPos, Sections, VisGraph, VisibilitySet};
use anyhow::{anyhow, Result};
use glam::DVec3;
use minecraftoss_core::block::flags;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::{BiomeId, BlockStateId, Chunk, Registries};
use minecraftoss_generator::terrain::TerrainGenerator;
use minecraftoss_generator::zoom;
use minecraftoss_world::chunk_map::WorldGen;
use minecraftoss_world::storage::ChunkStorage;
use minecraftoss_world::{spawn, ChunkEvent, ChunkMap};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

/// Presentation data for every block state and biome of a world.
pub struct BlockStates {
    registries: Arc<Registries>,
    blocks: Vec<Option<Block>>,
    /// Per-state light properties for the vanilla light solver.
    light_table: minecraftoss_core::light::LightTable,
    /// `DimensionType.hasSkyLight`.
    sky_light: bool,
    nether_shade: bool,
    /// `BlockState.isSolidRender`, which closes a cell in `VisGraph`.
    solid_render: Vec<bool>,
    face_culling: crate::face_culling::FaceCulling,
    /// Per state, what section building asks of its block's name and
    /// properties: whether it darkens a corner's ambient occlusion (opaque
    /// or leaves), fluid, holds_fluid, tint.
    ao_occluder: Vec<bool>,
    fluid: Vec<bool>,
    holds_fluid: Vec<bool>,
    fluid_overlay: Vec<bool>,
    tint_kind: Vec<mesh::TintKind>,
    /// Per state, the catalog `FluidState` as a `FluidCell` and `fluid::full_collision`.
    fluid_cell: Vec<Option<crate::fluid::FluidCell>>,
    full_collision: Vec<bool>,
    biomes: Vec<BiomeSample>,
    plains: BiomeSample,
    zoom_seed: i64,
    min_y: i32,
    height: i32,
}

impl BlockStates {
    pub fn new(registries: Arc<Registries>, seed: i64, min_y: i32, height: i32) -> Result<Self> {
        let count = registries.blocks.state_count();
        let mut blocks = Vec::with_capacity(count);
        let mut solid_render = Vec::with_capacity(count);
        let (mut ao_occluder, mut fluid, mut holds_fluid, mut tint_kind) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let (mut fluid_cell, mut full_collision, mut fluid_overlay) = (Vec::new(), Vec::new(), Vec::new());
        for index in 0..count {
            let state = BlockStateId(index as u16);
            let block = if registries.blocks.is_air(state) {
                None
            } else {
                Some(block_from_state(&registries.blocks.state_to_string(state))?)
            };
            solid_render.push(registries.blocks.is(state, flags::SOLID_RENDER));
            let path = block.as_ref().map_or("", |b| b.id.path.as_str());
            ao_occluder.push(block.as_ref().is_some_and(|b| shade_darkens(&registries.blocks, state, b)));
            let cell = registries.blocks.state(state).fluid.map(catalog_fluid_cell).or_else(|| block.as_ref().and_then(crate::fluid::FluidCell::from_block));
            fluid.push(matches!(path, "water" | "lava"));
            holds_fluid.push(cell.is_some() && !matches!(path, "water" | "lava"));
            let class = registries.blocks.block(registries.blocks.block_of(state));
            fluid_overlay.push(class.is_a("HalfTransparentBlock") || class.is_a("LeavesBlock"));
            tint_kind.push(block.as_ref().map_or(mesh::TintKind::None, mesh::TintKind::of));
            fluid_cell.push(cell);
            full_collision.push(crate::fluid::full_collision(block.as_ref()));
            blocks.push(block);
        }
        let biomes = registries
            .biomes
            .iter()
            .map(|(_, info)| biome_sample(info))
            .collect::<Vec<_>>();
        let light_table = minecraftoss_core::light::LightTable::new(&registries.blocks);
        let plains = registries
            .biomes
            .id("minecraft:plains")
            .map_or(BiomeSample::THE_VOID, |id| biomes[usize::from(id.0)]);
        let face_culling = crate::face_culling::FaceCulling::new(&registries);
        Ok(Self {
            registries,
            blocks,
            light_table,
            sky_light: true,
            nether_shade: false,
            solid_render,
            face_culling,
            ao_occluder,
            fluid,
            holds_fluid,
            fluid_overlay,
            tint_kind,
            fluid_cell,
            full_collision,
            biomes,
            plains,
            zoom_seed: zoom::zoom_seed(seed),
            min_y,
            height,
        })
    }

    pub fn registries(&self) -> &Arc<Registries> {
        &self.registries
    }

    pub fn face_hidden(&self, own: BlockStateId, neighbor: BlockStateId, side: Direction) -> bool {
        self.face_culling.hidden(own, neighbor, side)
    }

    pub fn fluid_face_hidden(&self, neighbor: BlockStateId, side: Direction, height: f32) -> bool {
        self.face_culling.fluid_face_hidden(neighbor, side, height)
    }

    pub fn fluid_cell(&self, state: BlockStateId) -> Option<crate::fluid::FluidCell> {
        self.fluid_cell[usize::from(state.0)]
    }

    pub fn holds_fluid(&self, state: BlockStateId) -> bool {
        self.holds_fluid[usize::from(state.0)]
    }

    pub fn solid_render(&self, state: BlockStateId) -> bool {
        self.solid_render[usize::from(state.0)]
    }

    pub fn fluid_overlay(&self, state: BlockStateId) -> bool {
        self.fluid_overlay[usize::from(state.0)]
    }

    /// Whether a block darkens the corners of faces beside it
    /// (`shade_darkens`).
    pub fn shade_darkens(&self, block: &Block) -> bool {
        self.state_of(block).map_or_else(|| block.is_opaque() || block.id.path.ends_with("_leaves"), |state| self.ao_occluder[usize::from(state.0)])
    }

    /// The scene block of a state; air states have none.
    pub fn block(&self, state: BlockStateId) -> Option<&Block> {
        self.blocks[usize::from(state.0)].as_ref()
    }

    /// The state an edited block stands for. Unlisted properties keep the
    /// block's defaults; an unknown block or value reads as its default state.
    pub fn state_of(&self, block: &Block) -> Option<BlockStateId> {
        let registry = &self.registries.blocks;
        let name = block.id.key();
        let id = registry.block_by_name(&name)?;
        let mut state = registry.block(id).default_state();
        for (key, value) in &block.properties {
            if let Some(next) = registry.with_property(state, key, value) {
                state = next;
            }
        }
        Some(state)
    }

    /// `BlockState.getSoundType()` of a block's state, when the catalog
    /// records sound types.
    pub fn sound_type(&self, block: &Block) -> Option<&minecraftoss_core::block::SoundType> {
        let state = self.state_of(block)?;
        self.registries.blocks.sound_type(state)
    }

    /// The sound type of the block named `id` in its default state.
    pub fn sound_type_of(&self, id: &str) -> Option<&minecraftoss_core::block::SoundType> {
        let registry = &self.registries.blocks;
        let block = registry.block_by_name(id)?;
        registry.sound_type(registry.block(block).default_state())
    }

    pub fn vertical_range(&self) -> std::ops::Range<i32> {
        self.min_y..self.min_y + self.height
    }

    fn section_range(&self) -> (i32, i32) {
        (self.min_y >> 4, (self.min_y + self.height - 1) >> 4)
    }

    /// `BiomeManager.getBiome`: the zoomed noise biome at a block. Missing
    /// chunks read as plains, as on the vanilla client.
    pub fn biome_at<'c>(
        &self,
        (x, y, z): BlockPos,
        chunk: impl Fn(ChunkPos) -> Option<&'c Chunk>,
    ) -> BiomeSample {
        let [qx, qy, qz] = zoom::quart_for_block(self.zoom_seed, x, y, z);
        chunk((qx >> 2, qz >> 2)).map_or(self.plains, |chunk| {
            let id = chunk.biome((qx & 3) as usize, qy, (qz & 3) as usize);
            self.biome(id)
        })
    }

    pub fn biome(&self, id: BiomeId) -> BiomeSample {
        self.biomes[usize::from(id.0)]
    }

    /// `BiomeManager.getNoiseBiomeAtQuart` on the client: the chunk's noise
    /// biome, or plains where no chunk is loaded.
    pub fn noise_biome<'c>(&self, (qx, qy, qz): (i32, i32, i32), chunk: impl Fn(ChunkPos) -> Option<&'c Chunk>) -> BiomeId {
        chunk((qx >> 2, qz >> 2)).map_or_else(
            || self.registries.biomes.id("minecraft:plains").unwrap_or(BiomeId(0)),
            |chunk| chunk.biome((qx & 3) as usize, qy, (qz & 3) as usize),
        )
    }
}

fn catalog_fluid_cell(fluid: minecraftoss_core::block::FluidInfo) -> crate::fluid::FluidCell {
    crate::fluid::FluidCell {
        kind: match fluid.kind {
            minecraftoss_core::block::FluidKind::Water => crate::fluid::FluidKind::Water,
            minecraftoss_core::block::FluidKind::Lava => crate::fluid::FluidKind::Lava,
        },
        amount: fluid.amount,
        source: fluid.source,
        falling: fluid.falling,
    }
}

fn block_from_state(text: &str) -> Result<Block> {
    let (name, properties) = match text.split_once('[') {
        Some((name, rest)) => (name, rest.trim_end_matches(']')),
        None => (text, ""),
    };
    let mut block = Block {
        id: ResourceId::parse(name)?,
        properties: BTreeMap::new(),
    };
    for pair in properties.split(',').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| anyhow!("bad block state {text}"))?;
        block.properties.insert(key.into(), value.into());
    }
    Ok(block)
}

/// `#rrggbb` strings or packed integers, as 26.3 biome codecs accept.
fn color(value: &serde_json::Value) -> Option<[u8; 3]> {
    let packed = match value {
        serde_json::Value::String(text) => {
            u32::from_str_radix(text.strip_prefix('#').unwrap_or(text), 16).ok()?
        }
        serde_json::Value::Number(number) => number.as_i64()? as u32,
        _ => return None,
    };
    Some([(packed >> 16) as u8, (packed >> 8) as u8, packed as u8])
}

/// The presentation sample of `minecraft:plains`.
pub fn plains_sample(registries: &Registries) -> BiomeSample {
    registries.biomes.id("minecraft:plains").map_or(BiomeSample::THE_VOID, |id| biome_sample(registries.biomes.get(id)))
}

fn biome_sample(info: &minecraftoss_core::biome::BiomeInfo) -> BiomeSample {
    let effect = |key: &str| info.effects.get(key).and_then(color);
    let attribute = |key: &str| info.attributes.get(key).and_then(color);
    BiomeSample {
        temperature: info.temperature,
        downfall: info.downfall,
        has_precipitation: info.has_precipitation,
        grass_color: effect("grass_color"),
        foliage_color: effect("foliage_color"),
        dry_foliage_color: effect("dry_foliage_color"),
        water_color: effect("water_color").unwrap_or(BiomeSample::THE_VOID.water_color),
        sky_color: attribute("minecraft:visual/sky_color"),
        fog_color: attribute("minecraft:visual/fog_color"),
    }
}

/// `BlockState.getShadeBrightness` is 0.2 rather than 1: the block darkens
/// the corners of faces beside it. A full-block collision shape does
/// (`BlockBehaviour`), except where a block says otherwise: glass, stained
/// and tinted glass and copper grates (`TransparentBlock`), barriers, light
/// blocks and structure voids never do; mud, soul sand and a full stack of
/// snow always do. Plants, with no collision, never darken the ground.
fn shade_darkens(registry: &minecraftoss_core::block::BlockRegistry, state: BlockStateId, block: &Block) -> bool {
    use minecraftoss_core::block::FaceShape;
    let path = block.id.path.as_str();
    match path {
        "glass" | "tinted_glass" | "barrier" | "light" | "structure_void" => return false,
        _ if path.ends_with("_stained_glass") || path.ends_with("copper_grate") => return false,
        "mud" | "soul_sand" => return true,
        "snow" => return block.properties.get("layers").is_some_and(|layers| layers == "8"),
        _ => {}
    }
    match registry.collision_shape(state) {
        Some(FaceShape::Full) => true,
        Some(FaceShape::Boxes(boxes)) => boxes.len() == 1 && boxes[0] == [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        Some(FaceShape::Empty) => false,
        // A catalog without shapes: the older name-based guess.
        None => block.is_opaque() || path.ends_with("_leaves"),
    }
}

/// A chunk and its eight neighbors, with their edits, as one job sees them.
pub struct Neighborhood {
    pub center: ChunkPos,
    chunks: [Arc<Chunk>; 9],
    placed: [Option<Arc<BTreeMap<BlockPos, Block>>>; 9],
    cleared: [Option<Arc<BTreeSet<BlockPos>>>; 9],
}

impl Neighborhood {
    /// `None` until all nine chunks are loaded.
    pub fn of(scene: &HandcraftedScene, center: ChunkPos) -> Option<Self> {
        let at = |i: usize| (center.0 + i as i32 % 3 - 1, center.1 + i as i32 / 3 - 1);
        let chunks = (0..9)
            .map(|i| scene.generated_chunk(at(i)).cloned())
            .collect::<Option<Vec<_>>>()?;
        let edits = |i: usize| scene.chunk_edits(at(i));
        Some(Self {
            center,
            chunks: chunks.try_into().ok()?,
            placed: std::array::from_fn(|i| edits(i).0.cloned()),
            cleared: std::array::from_fn(|i| edits(i).1.cloned()),
        })
    }

    fn chunk(&self, (cx, cz): ChunkPos) -> Option<&Chunk> {
        let (dx, dz) = (cx - self.center.0, cz - self.center.1);
        ((-1..=1).contains(&dx) && (-1..=1).contains(&dz))
            .then(|| &*self.chunks[((dz + 1) * 3 + dx + 1) as usize])
    }
}

/// Dense block states of a box inside a neighborhood, readable as a `Scene`.
struct View<'a> {
    states: &'a BlockStates,
    hood: &'a Neighborhood,
    min: BlockPos,
    size: (usize, usize, usize),
    cells: Vec<BlockStateId>,
}

impl<'a> View<'a> {
    fn new(states: &'a BlockStates, hood: &'a Neighborhood, min: BlockPos, size: (usize, usize, usize)) -> Self {
        let (sx, sy, sz) = size;
        let mut cells = vec![BlockStateId::AIR; sx * sy * sz];
        let world = states.vertical_range();
        let (y0, y1) = (min.1.max(world.start), (min.1 + sy as i32).min(world.end));
        for dx in 0..sx {
            for dz in 0..sz {
                let (x, z) = (min.0 + dx as i32, min.2 + dz as i32);
                let Some(chunk) = hood.chunk((x >> 4, z >> 4)) else {
                    continue;
                };
                let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
                let base = (dx * sz + dz) * sy;
                let mut y = y0;
                while y < y1 {
                    let section = &chunk.sections()[((y >> 4) - chunk.min_section_y()) as usize];
                    let end = ((y >> 4) + 1) * 16;
                    let end = end.min(y1);
                    match section.blocks.single() {
                        Some(state) => {
                            let from = base + (y - min.1) as usize;
                            cells[from..from + (end - y) as usize].fill(state);
                        }
                        None => {
                            for yy in y..end {
                                cells[base + (yy - min.1) as usize] =
                                    section.block(lx, (yy & 15) as usize, lz);
                            }
                        }
                    }
                    y = end;
                }
            }
        }
        let mut view = Self {
            states,
            hood,
            min,
            size,
            cells,
        };
        for i in 0..9 {
            for &pos in hood.cleared[i].iter().flat_map(|set| set.iter()) {
                if let Some(index) = view.index(pos) {
                    view.cells[index] = BlockStateId::AIR;
                }
            }
            for (&pos, block) in hood.placed[i].iter().flat_map(|map| map.iter()) {
                if let (Some(index), Some(state)) = (view.index(pos), states.state_of(block)) {
                    view.cells[index] = state;
                }
            }
        }
        view
    }

    fn index(&self, (x, y, z): BlockPos) -> Option<usize> {
        let (dx, dy, dz) = (x - self.min.0, y - self.min.1, z - self.min.2);
        let (sx, sy, sz) = self.size;
        if dx < 0 || dy < 0 || dz < 0 || dx as usize >= sx || dy as usize >= sy || dz as usize >= sz {
            return None;
        }
        Some((dx as usize * sz + dz as usize) * sy + dy as usize)
    }

    fn state(&self, pos: BlockPos) -> BlockStateId {
        self.index(pos).map_or(BlockStateId::AIR, |i| self.cells[i])
    }
}

impl Scene for View<'_> {
    fn block(&self, pos: BlockPos) -> Option<&Block> {
        self.states.block(self.state(pos))
    }
    fn chunks(&self) -> Vec<ChunkPos> {
        vec![self.hood.center]
    }
    fn vertical_range(&self) -> std::ops::Range<i32> {
        self.states.vertical_range()
    }
    fn revision(&self) -> u64 {
        0
    }
    fn biome_at(&self, pos: BlockPos) -> BiomeSample {
        self.states.biome_at(pos, |chunk| self.hood.chunk(chunk))
    }
    fn fluid_at(&self, pos: BlockPos) -> Option<crate::fluid::FluidCell> {
        self.states.fluid_cell[usize::from(self.state(pos).0)]
    }
    fn full_collision_at(&self, pos: BlockPos) -> bool {
        self.states.full_collision[usize::from(self.state(pos).0)]
    }
    fn solid_render_at(&self, pos: BlockPos) -> bool {
        self.states.solid_render[usize::from(self.state(pos).0)]
    }
    fn fluid_overlay_at(&self, pos: BlockPos) -> bool {
        self.states.fluid_overlay[usize::from(self.state(pos).0)]
    }
    fn fluid_face_hidden(&self, neighbor: BlockPos, side: Direction, height: f32) -> bool {
        self.states.fluid_face_hidden(self.state(neighbor), side, height)
    }
    fn shade_darkens_at(&self, pos: BlockPos) -> bool {
        self.states.ao_occluder[usize::from(self.state(pos).0)]
    }
}

/// Lights a chunk from its neighborhood with the vanilla light solver
/// (`minecraftoss_core::light`), edits included. The result covers the
/// chunk and a one-block border, everything its sections' faces sample.
fn light_chunk(states: &BlockStates, hood: &Neighborhood) -> SkyLight {
    let (cx, cz) = hood.center;
    let height = states.height as usize;
    let view = View::new(states, hood, (cx * 16 - 16, states.min_y, cz * 16 - 16), (48, height, 48));
    // The solver's layout: y from one section below the build range.
    let region_height = height + 32;
    let mut cells = vec![BlockStateId::AIR; 48 * 48 * region_height];
    let blocks = &states.registries.blocks;
    let sections = height / 16;
    let mut non_empty = vec![false; 9 * sections];
    for x in 0..48 {
        for z in 0..48 {
            for y in 0..height {
                let state = view.cells[(x * 48 + z) * height + y];
                cells[((y + 16) * 48 + z) * 48 + x] = state;
                if !blocks.is(state, minecraftoss_core::block::flags::AIR) {
                    non_empty[((z / 16) * 3 + x / 16) * sections + y / 16] = true;
                }
            }
        }
    }
    let min_section = states.min_y >> 4;
    let region = minecraftoss_core::light::light_region(
        blocks,
        &states.light_table,
        min_section,
        sections,
        cells,
        |dx, dz, sy| {
            let s = sy - min_section;
            (-1..=1).contains(&dx) && (-1..=1).contains(&dz) && (0..sections as i32).contains(&s) && non_empty[((dz + 1) * 3 + dx + 1) as usize * sections + s as usize]
        },
        states.sky_light,
    );
    // Keep columns up to the highest cell below full sky light or with
    // block light; lookups above return those defaults.
    let mut top = 0;
    for x in 15..33 {
        for z in 15..33 {
            if let Some(y) = (0..region_height).rev().find(|&y| {
                let (block, sky) = region.get(x, y, z);
                sky < 15 || block > 0
            }) {
                top = top.max(y + 1);
            }
        }
    }
    let (mut sky, mut block) = (Vec::with_capacity(18 * 18 * top), Vec::with_capacity(18 * 18 * top));
    for x in 15..33 {
        for z in 15..33 {
            for y in 0..top {
                let (b, s) = region.get(x, y, z);
                block.push(b);
                sky.push(s);
            }
        }
    }
    SkyLight::from_levels((cx * 16 - 1, states.min_y - 16, cz * 16 - 1), (18, top, 18), sky, block)
}

/// A chunk's light as the server sent it, as [`light_chunk`] lays it out:
/// the column and a one-block border from the nine chunks' own light, as
/// vanilla's client takes light from the server. `None` when a chunk has
/// no light or the neighborhood has edits the server light misses.
fn server_light(states: &BlockStates, hood: &Neighborhood) -> Option<SkyLight> {
    if hood.placed.iter().any(Option::is_some) || hood.cleared.iter().any(Option::is_some) {
        return None;
    }
    let lights: Vec<&minecraftoss_core::light::ChunkLight> = hood.chunks.iter().map(|chunk| chunk.light.as_deref()).collect::<Option<_>>()?;
    let (cx, cz) = hood.center;
    let region_min_y = states.min_y - 16;
    let region_height = states.height as usize + 32;
    // Each border column's (sky, block) levels from the bottom of the region.
    let mut columns: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(18 * 18);
    let mut top = 0;
    for x in cx * 16 - 1..=cx * 16 + 16 {
        for z in cz * 16 - 1..=cz * 16 + 16 {
            let light = lights[(((z >> 4) - cz + 1) * 3 + (x >> 4) - cx + 1) as usize];
            let (mut sky, mut block) = (Vec::with_capacity(region_height), Vec::with_capacity(region_height));
            for y in 0..region_height {
                let world_y = region_min_y + y as i32;
                // Without skylight the solver's sky levels are all 0.
                let s = if states.sky_light { light.sky_at(x, world_y, z) } else { 0 };
                let b = light.block_at(x, world_y, z);
                sky.push(s as u8);
                block.push(b as u8);
                if s < 15 || b > 0 {
                    top = top.max(y + 1);
                }
            }
            columns.push((sky, block));
        }
    }
    let (mut sky, mut block) = (Vec::with_capacity(18 * 18 * top), Vec::with_capacity(18 * 18 * top));
    for (column_sky, column_block) in &columns {
        sky.extend_from_slice(&column_sky[..top]);
        block.extend_from_slice(&column_block[..top]);
    }
    Some(SkyLight::from_levels((cx * 16 - 1, region_min_y, cz * 16 - 1), (18, top, 18), sky, block))
}

enum Slot {
    Unresolved,
    Missing,
    Model {
        variants: Vec<(ResolvedModel, u32)>,
        /// Each variant's faces baked; `None` if a face could not be.
        baked: Option<Vec<Vec<mesh::BakedQuad>>>,
    },
}

/// Resolved models per block state, for one pack generation.
#[derive(Default)]
struct ModelCache {
    slots: Vec<Slot>,
}

impl ModelCache {
    fn prepare(&mut self, states: &BlockStates, packs: &PackStack, atlas: &Atlas, view: &View) {
        if self.slots.len() != states.blocks.len() {
            self.slots = (0..states.blocks.len()).map(|_| Slot::Unresolved).collect();
        }
        for &state in &view.cells {
            let slot = &mut self.slots[usize::from(state.0)];
            if !matches!(slot, Slot::Unresolved) {
                continue;
            }
            let Some(block) = states.block(state) else {
                *slot = Slot::Missing;
                continue;
            };
            *slot = match resolve_block_variants(packs, block) {
                Ok(variants) => Slot::Model {
                    baked: variants.iter().map(|(model, _)| mesh::bake_quads(block, model, atlas)).collect::<Result<Vec<_>>>().ok(),
                    variants,
                },
                Err(e) => {
                    eprintln!("no model for {}: {e:#}", block.id.key());
                    Slot::Missing
                }
            };
        }
    }

    fn face_hidden(&self, states: &BlockStates, own: BlockStateId, neighbor: BlockStateId, side: Direction) -> bool {
        states.face_hidden(own, neighbor, side)
    }
}

/// Profiling: summed time per section compile phase over the process.
pub mod compile_profile {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;
    pub const VIEW: usize = 0;
    pub const MODELS: usize = 1;
    pub const BLOCKS: usize = 2;
    pub const CONVERT: usize = 3;
    const NAMES: [&str; 4] = ["compile: view", "compile: models", "compile: blocks", "compile: gpu format"];
    static MICROS: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
    static COUNTS: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
    pub fn add(phase: usize, since: Instant) {
        MICROS[phase].fetch_add(since.elapsed().as_micros() as u64, Ordering::Relaxed);
        COUNTS[phase].fetch_add(1, Ordering::Relaxed);
    }
    pub fn report() -> String {
        let mut out = String::new();
        for (i, name) in NAMES.iter().enumerate() {
            let (micros, count) = (MICROS[i].load(Ordering::Relaxed), COUNTS[i].load(Ordering::Relaxed));
            out += &format!("    {name:32} {:8.2}s {count:7} x {:8.0} us\n", micros as f64 / 1e6, micros.checked_div(count).unwrap_or(0));
        }
        out
    }
}

fn nether_cardinal_light(registries: &Registries, dimension_type: &str) -> bool {
    minecraftoss_core::Identifier::parse(dimension_type)
        .and_then(|id| registries.datapack.read_json("dimension_type", &id))
        .is_ok_and(|json| json.get("cardinal_light").and_then(|light| light.as_str()) == Some("nether"))
}

/// `SectionCompiler.compile`: one section's faces and visibility set.
#[allow(clippy::too_many_arguments)]
fn compile_section(
    states: &BlockStates,
    hood: &Neighborhood,
    (sx, sy, sz): SectionPos,
    light: &SkyLight,
    models: &mut ModelCache,
    packs: &PackStack,
    atlas: &Atlas,
    tint: &BiomeTint,
    apart: &[BlockPos],
) -> Result<(ChunkMesh, VisibilitySet)> {
    let origin = (sx * 16, sy * 16, sz * 16);
    mesh::set_section_nether_shade(states.nether_shade);
    let started = Instant::now();
    let view = View::new(states, hood, (origin.0 - 1, origin.1 - 1, origin.2 - 1), (18, 18, 18));
    compile_profile::add(compile_profile::VIEW, started);
    let started = Instant::now();
    models.prepare(states, packs, atlas, &view);
    compile_profile::add(compile_profile::MODELS, started);
    let started = Instant::now();
    let models = &*models;
    let built = build_section_blocks(states, &view, origin, light, models, atlas, tint, true, apart)?;
    if std::env::var_os("MINECRAFTOSS_MESH_CHECK").is_some() {
        // The baked faces must give exactly the unbaked build's mesh.
        mesh::FLUID_FULL_PATH.with(|full| full.set(true));
        let slow = build_section_blocks(states, &view, origin, light, models, atlas, tint, false, apart);
        mesh::FLUID_FULL_PATH.with(|full| full.set(false));
        let slow = slow?;
        let bits = |m: &ChunkMesh| -> Vec<u32> {
            m.vertices.iter().flat_map(|v| v.position.iter().chain(&v.uv).chain(&v.color).chain([&v.sky_light, &v.block_light]).map(|f| f.to_bits()).collect::<Vec<_>>()).collect()
        };
        assert!(
            bits(&built.0) == bits(&slow.0) && built.0.indices == slow.0.indices && built.0.faces == slow.0.faces && built.1 == slow.1,
            "baked section {:?} differs",
            (sx, sy, sz)
        );
    }
    let (mesh, transparent, vis) = (built.0, built.1, built.2);
    compile_profile::add(compile_profile::BLOCKS, started);
    Ok((mesh::finish_mesh(mesh, transparent), vis.resolve()))
}

/// The block loop of a section build: its faces (opaque, then the
/// translucent indices apart) and its visibility graph. `baked` uses the
/// baked faces where a block has them.
#[allow(clippy::too_many_arguments)]
fn build_section_blocks(states: &BlockStates, view: &View, origin: BlockPos, light: &SkyLight, models: &ModelCache, atlas: &Atlas, tint: &BiomeTint, baked: bool, apart: &[BlockPos]) -> Result<(ChunkMesh, Vec<u32>, VisGraph)> {
    let mut mesh = ChunkMesh::default();
    let mut transparent = Vec::new();
    let mut vis = VisGraph::default();
    // BlockPos.betweenClosed order: X fastest, then Y, then Z.
    for z in 0..16 {
        for y in 0..16 {
            for x in 0..16 {
                let pos = (origin.0 + x, origin.1 + y, origin.2 + z);
                let state = view.state(pos);
                let Some(block) = states.block(state) else {
                    continue;
                };
                if states.solid_render[usize::from(state.0)] {
                    vis.set_opaque(x as usize, y as usize, z as usize);
                }
                let index = usize::from(state.0);
                if apart.contains(&pos) {
                    mesh::append_baked(view, pos, &[], states.tint_kind[index], states.holds_fluid[index], |_, _| false, |_| false, atlas, tint, light, &mut mesh, &mut transparent)?;
                    continue;
                }
                let slot = &models.slots[index];
                let fluid = states.fluid[index];
                if !fluid && !matches!(slot, Slot::Model { .. }) {
                    continue;
                }
                if let Slot::Model { variants, baked: Some(quads), .. } = slot {
                    if baked && !fluid {
                        mesh::append_baked(
                            view,
                            pos,
                            &quads[mesh::variant_for(variants, pos)],
                            states.tint_kind[index],
                            states.holds_fluid[index],
                            |neighbor, side| models.face_hidden(states, state, view.state(neighbor), side),
                            |at| states.ao_occluder[usize::from(view.state(at).0)],
                            atlas,
                            tint,
                            light,
                            &mut mesh,
                            &mut transparent,
                        )?;
                        continue;
                    }
                }
                mesh::append_block(
                    view,
                    pos,
                    block,
                    || match slot {
                        Slot::Model { variants, .. } => Ok(variants.as_slice()),
                        _ => Err(anyhow!("unresolved model")),
                    },
                    |neighbor, side| models.face_hidden(states, state, view.state(neighbor), side),
                    atlas,
                    tint,
                    light,
                    &mut mesh,
                    &mut transparent,
                )?;
            }
        }
    }
    Ok((mesh, transparent, vis))
}

/// What a job needs besides its own inputs.
#[derive(Clone)]
struct JobContext {
    states: Arc<BlockStates>,
    atlas: Arc<Atlas>,
    pack_sources: Vec<PathBuf>,
    pack_generation: u64,
}

struct LightJob {
    chunk: ChunkPos,
    epoch: u64,
    hood: Neighborhood,
}

struct CompileJob {
    serial: u64,
    hood: Neighborhood,
    light: Arc<SkyLight>,
    context: JobContext,
    apart: Vec<BlockPos>,
}

struct WorkQueue {
    light: Vec<(LightJob, Arc<BlockStates>)>,
    compile: CompileQueue<CompileJob>,
    eye: DVec3,
    /// Jobs a worker has taken but not finished.
    active: usize,
    stop: bool,
}

enum Job {
    Light(LightJob, Arc<BlockStates>),
    Compile(SectionPos, CompileJob),
}

enum Done {
    Light {
        chunk: ChunkPos,
        epoch: u64,
        light: Arc<SkyLight>,
        micros: u64,
    },
    Compile {
        section: SectionPos,
        serial: u64,
        pack_generation: u64,
        micros: u64,
        outcome: Result<(mesh::SectionMesh, VisibilitySet, Option<crate::translucency::TranslucentQuads>)>,
    },
}

type Work = Arc<(Mutex<WorkQueue>, Condvar)>;

/// Lighting gates compiling, so a worker takes the nearest light job first,
/// then the section `SectionTaskDynamicQueue` would pick.
fn next_job(queue: &mut WorkQueue) -> Option<Job> {
    let eye = queue.eye;
    let nearest = queue
        .light
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let d = |c: ChunkPos| {
                let (x, z) = (f64::from(c.0 * 16 + 8) - eye.x, f64::from(c.1 * 16 + 8) - eye.z);
                x * x + z * z
            };
            d(a.0.chunk).total_cmp(&d(b.0.chunk))
        })
        .map(|(i, _)| i);
    if let Some(i) = nearest {
        let (job, states) = queue.light.swap_remove(i);
        return Some(Job::Light(job, states));
    }
    queue.compile.poll(eye).map(|(pos, job)| Job::Compile(pos, job))
}

fn worker(work: &Work, done: &mpsc::Sender<Done>) {
    let mut packs: Option<(Vec<PathBuf>, u64, PackStack, BiomeTint)> = None;
    let mut models = ModelCache::default();
    loop {
        let (job, eye) = {
            let (lock, ready) = &**work;
            let mut queue = lock.lock().expect("terrain work queue");
            loop {
                if queue.stop {
                    return;
                }
                if let Some(job) = next_job(&mut queue) {
                    queue.active += 1;
                    break (job, queue.eye);
                }
                queue = ready.wait(queue).expect("terrain work queue");
            }
        };
        let started = Instant::now();
        let result = match job {
            Job::Light(job, states) => Done::Light {
                chunk: job.chunk,
                epoch: job.epoch,
                light: Arc::new(server_light(&states, &job.hood).unwrap_or_else(|| light_chunk(&states, &job.hood))),
                micros: started.elapsed().as_micros() as u64,
            },
            Job::Compile(section, job) => {
                let context = &job.context;
                if packs.as_ref().is_none_or(|(sources, generation, _, _)| {
                    sources != &context.pack_sources || *generation != context.pack_generation
                }) {
                    models = ModelCache::default();
                    packs = PackStack::open(context.pack_sources.clone())
                        .and_then(|stack| Ok((BiomeTint::from_pack(&stack)?, stack)))
                        .map(|(tint, stack)| (context.pack_sources.clone(), context.pack_generation, stack, tint))
                        .map_err(|e| eprintln!("section compiler cannot open packs: {e:#}"))
                        .ok();
                }
                let outcome = match &packs {
                    Some((_, _, stack, tint)) => compile_section(
                        &context.states,
                        &job.hood,
                        section,
                        &job.light,
                        &mut models,
                        stack,
                        &context.atlas,
                        tint,
                        &job.apart,
                    ),
                    None => Err(anyhow!("resource packs unavailable")),
                }
                // The GPU format is made here, off the render thread.
                .map(|(mesh, visibility)| {
                    let started = Instant::now();
                    let mut mesh = mesh::SectionMesh::from(mesh);
                    let mut quads = crate::translucency::TranslucentQuads::of(&mesh);
                    if let Some(quads) = quads.as_mut() {
                        quads.sort_mesh(&mut mesh, eye, section);
                    }
                    compile_profile::add(compile_profile::CONVERT, started);
                    (mesh, visibility, quads)
                });
                Done::Compile {
                    section,
                    serial: job.serial,
                    pack_generation: context.pack_generation,
                    micros: started.elapsed().as_micros() as u64,
                    outcome,
                }
            }
        };
        let sent = done.send(result);
        work.0.lock().expect("terrain work queue").active -= 1;
        if sent.is_err() {
            return;
        }
    }
}

/// `PlayerSpawnFinder` starts at an unseeded random candidate.
fn unseeded() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.subsec_nanos())
}

/// Renderer changes from one frame of terrain work.
#[derive(Default)]
pub struct FrameUpdate {
    /// New or rebuilt section meshes; an empty mesh removes the section.
    pub uploads: Vec<(SectionPos, mesh::SectionMesh)>,
    pub removed: Vec<SectionPos>,
    /// Chunk light columns to set or clear in the world light.
    pub lights: Vec<(ChunkPos, Option<Arc<SkyLight>>)>,
    /// Sections to draw, nearest first, with their fade-in visibility.
    pub visible: Vec<(SectionPos, f32)>,
}

#[derive(Default)]
struct Stats {
    lit: u64,
    light_micros: u64,
    compiled: u64,
    compile_micros: u64,
}

/// The integrated server's chunk map and the client's section rendering.
pub struct TerrainStream {
    pub states: Arc<BlockStates>,
    /// Vanilla's world spawn block (`MinecraftServer.setInitialSpawn`).
    pub world_spawn: (i32, i32, i32),
    /// Where the player first appears, within the respawn radius of it.
    pub player_spawn: (f64, f64, f64),
    server: ChunkMap,
    /// Dropped after `server`, whose drop saves into it.
    _session: Option<SessionDir>,
    sections: Sections,
    work: Work,
    done: mpsc::Receiver<Done>,
    workers: Vec<JoinHandle<()>>,
    lights: HashMap<ChunkPos, Arc<SkyLight>>,
    /// Bumped whenever a chunk must be (re)lit; stale light results drop.
    light_epochs: HashMap<ChunkPos, u64>,
    lights_pending: HashSet<ChunkPos>,
    /// Bumped whenever a section compile is scheduled; stale results drop.
    serials: HashMap<SectionPos, u64>,
    next_serial: u64,
    /// Edited sections and relit chunks the player is waiting on.
    player_sections: HashSet<SectionPos>,
    player_relights: HashSet<ChunkPos>,
    removed: Vec<SectionPos>,
    light_changes: Vec<(ChunkPos, Option<Arc<SkyLight>>)>,
    scheduled_last_frame: usize,
    clock: Instant,
    stats: Stats,
    apart: crate::lids::ApartCells,
    translucent: crate::translucency::TranslucentSorting,
}

pub use minecraft_net::Dimension;

const NETHER_FALLBACK_SPAWN: (i32, i32, i32) = (0, 64, 0);

fn block_centre((x, y, z): (i32, i32, i32)) -> (f64, f64, f64) {
    (f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5)
}

impl TerrainStream {
    pub fn new(registries: Arc<Registries>, seed: i64, view_distance: i32) -> Result<Self> {
        Self::for_dimension(registries, seed, view_distance, Dimension::Overworld, None)
    }

    /// A streamed dimension; with `world`, chunks load from and save to that
    /// world directory's region files.
    pub fn for_dimension(registries: Arc<Registries>, seed: i64, view_distance: i32, dimension: Dimension, world: Option<&std::path::Path>) -> Result<Self> {
        Self::for_dimension_at(registries, seed, view_distance, dimension, world, None)
    }

    pub fn for_dimension_at(registries: Arc<Registries>, seed: i64, view_distance: i32, dimension: Dimension, world: Option<&std::path::Path>, known_spawn: Option<(i32, i32, i32)>) -> Result<Self> {
        let started = Instant::now();
        let generator = match dimension {
            Dimension::Overworld => TerrainGenerator::overworld(registries.clone(), seed),
            Dimension::Nether => TerrainGenerator::nether(registries.clone(), seed),
            Dimension::End => TerrainGenerator::end(registries.clone(), seed),
        }
        .map_err(|e| anyhow!(e))?;
        let mut states = BlockStates::new(registries, seed, generator.chunk_min_y, generator.chunk_height)?;
        states.sky_light = dimension != Dimension::Nether;
        states.nether_shade = nether_cardinal_light(states.registries(), dimension.dimension_type());
        let states = Arc::new(states);
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        // Generation and section building run at background priority, so
        // they may ask for more threads than there are cores: the render
        // and main threads still come first. Three quarters of the hardware
        // threads generate (a fast flight at 32 chunks needs about 400
        // chunks a second) and half less one build sections.
        // MINECRAFTOSS_GEN_THREADS and MINECRAFTOSS_SECTION_THREADS override.
        let env = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<usize>().ok()).filter(|&n| n > 0);
        let generation_threads = env("MINECRAFTOSS_GEN_THREADS").unwrap_or((threads * 3 / 4).max(1));
        let section_threads = env("MINECRAFTOSS_SECTION_THREADS").unwrap_or((threads / 2).saturating_sub(1).max(1));
        // Chunks leaving the loaded area are always stored, as vanilla saves
        // them: without a world directory, in a session directory removed
        // when the stream closes. A chunk that comes back is loaded as it
        // was, never generated a second time over finished neighbours.
        let session = world.is_none().then(SessionDir::new).transpose()?;
        let world_path: &Path = match (world, &session) {
            (Some(w), _) => w,
            (None, Some(session)) => &session.0,
            (None, None) => unreachable!("a session directory exists without a world"),
        };
        let storage = Some(ChunkStorage::new(world_path, dimension.dimension_type(), states.registries().clone(), generator.chunk_min_y, generator.chunk_height));
        let worldgen = WorldGen::for_dimension(Arc::new(generator), dimension.dimension_type()).map_err(|e| anyhow!(e))?;
        eprintln!("world load: generator, features and structures in {:.2}s", started.elapsed().as_secs_f64());
        let mut server = ChunkMap::with_storage(Arc::new(worldgen), view_distance, generation_threads, storage);
        let searching = Instant::now();
        let (world_spawn, player_spawn) = match dimension {
            Dimension::Overworld => {
                let world_spawn = known_spawn.unwrap_or_else(|| spawn::world_spawn(&mut server));
                (world_spawn, spawn::player_spawn(&mut server, world_spawn, spawn::DEFAULT_RESPAWN_RADIUS, unseeded()))
            }
            Dimension::Nether => {
                let known = known_spawn.unwrap_or(NETHER_FALLBACK_SPAWN);
                (known, block_centre(known))
            }
            // ServerLevel.END_SPAWN_POINT: the obsidian platform at 100, 49, 0.
            Dimension::End => {
                let known = known_spawn.unwrap_or((100, 49, 0));
                (known, block_centre(known))
            }
        };
        eprintln!("spawn found in {:.2}s ({} chunks generated)", searching.elapsed().as_secs_f64(), server.stats().generated);
        let (min_section, max_section) = states.section_range();
        let work: Work = Arc::new((
            Mutex::new(WorkQueue {
                light: Vec::new(),
                compile: CompileQueue::default(),
                eye: DVec3::ZERO,
                active: 0,
                stop: false,
            }),
            Condvar::new(),
        ));
        let (sender, done) = mpsc::channel();
        let workers = (0..section_threads)
            .map(|index| {
                let (work, sender) = (work.clone(), sender.clone());
                std::thread::Builder::new()
                    .name(format!("section-builder-{index}"))
                    .spawn(move || {
                        minecraftoss_core::thread_priority::background();
                        worker(&work, &sender)
                    })
                    .map_err(Into::into)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            states,
            world_spawn,
            player_spawn,
            sections: Sections::new(server.view_distance(), min_section, max_section),
            server,
            _session: session,
            work,
            done,
            workers,
            lights: HashMap::new(),
            light_epochs: HashMap::new(),
            lights_pending: HashSet::new(),
            serials: HashMap::new(),
            next_serial: 0,
            player_sections: HashSet::new(),
            player_relights: HashSet::new(),
            removed: Vec::new(),
            light_changes: Vec::new(),
            scheduled_last_frame: 0,
            clock: Instant::now(),
            stats: Stats::default(),
            apart: Default::default(),
            translucent: Default::default(),
        })
    }

    pub fn view_distance(&self) -> i32 {
        self.server.view_distance()
    }

    /// A new render distance from the options screen: the server tracks
    /// the new area and every section is rebuilt on a new grid, as vanilla's
    /// `LevelRenderer.allChanged`. Whether it changed (the renderer then
    /// drops its sections).
    pub fn set_view_distance(&mut self, scene: &HandcraftedScene, view_distance: i32) -> bool {
        self.server.set_view_distance(view_distance);
        let view_distance = self.server.view_distance();
        if view_distance == self.sections.view_distance() {
            return false;
        }
        let (min_section, max_section) = self.states.section_range();
        self.sections = Sections::new(view_distance, min_section, max_section);
        for pos in scene.generated_chunks().collect::<Vec<_>>() {
            let chunk = scene.generated_chunk(pos).expect("listed").clone();
            let empty = self.empty_sections(scene, &chunk);
            self.sections.chunk_loaded(pos, empty);
        }
        // Builds queued for the old grid are dropped.
        self.serials.clear();
        self.player_sections.clear();
        self.removed.clear();
        self.work.0.lock().expect("terrain work queue").compile.retain(|_, _| false);
        true
    }

    /// `PlayerSpawnFinder.findSpawn` again, for a respawn.
    pub fn respawn_position(&mut self) -> (f64, f64, f64) {
        spawn::player_spawn(&mut self.server, self.world_spawn, spawn::DEFAULT_RESPAWN_RADIUS, unseeded())
    }

    pub fn reshow(&mut self, scene: &HandcraftedScene) -> Vec<Arc<Chunk>> {
        let (min_section, max_section) = self.states.section_range();
        self.sections = Sections::new(self.sections.view_distance(), min_section, max_section);
        let mut chunks = Vec::new();
        for pos in scene.generated_chunks().collect::<Vec<_>>() {
            let mut chunk = scene.generated_chunk(pos).expect("listed").clone();
            let empty = self.empty_sections(scene, &chunk);
            self.sections.chunk_loaded(pos, empty);
            self.apply_scene_edits(scene, &mut chunk);
            chunks.push(chunk);
        }
        self.serials.clear();
        self.player_sections.clear();
        self.removed.clear();
        self.work.0.lock().expect("terrain work queue").compile.retain(|_, _| false);
        self.light_changes = self.lights.iter().map(|(&pos, light)| (pos, Some(light.clone()))).collect();
        chunks
    }

    fn apply_scene_edits(&self, scene: &HandcraftedScene, chunk: &mut Arc<Chunk>) {
        let (placed, cleared) = scene.chunk_edits((chunk.pos.x, chunk.pos.z));
        if placed.is_none() && cleared.is_none() {
            return;
        }
        let registries = self.states.registries().clone();
        let chunk = Arc::make_mut(chunk);
        let placed = placed.into_iter().flat_map(|placed| placed.iter()).map(|(&pos, block)| (pos, self.states.state_of(block).unwrap_or(BlockStateId::AIR)));
        let cleared = cleared.into_iter().flat_map(|cleared| cleared.iter()).map(|&pos| (pos, BlockStateId::AIR));
        for ((x, y, z), state) in placed.chain(cleared) {
            chunk.set_block_raw((x & 15) as usize, y, (z & 15) as usize, state, &registries);
        }
    }

    fn now(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }

    pub fn section_compiled_and_visible(&self, (x, y, z): BlockPos, fade_millis: u64) -> bool {
        self.sections.compiled_and_visible((x >> 4, y >> 4, z >> 4), self.now(), fade_millis)
    }

    fn empty_sections(&self, scene: &HandcraftedScene, chunk: &Chunk) -> Vec<i32> {
        let placed = scene.chunk_edits((chunk.pos.x, chunk.pos.z)).0;
        chunk
            .sections()
            .iter()
            .enumerate()
            .map(|(i, section)| (chunk.min_section_y() + i as i32, section))
            .filter(|(y, section)| {
                section.is_empty()
                    && !placed.is_some_and(|placed| placed.keys().any(|pos| pos.1 >> 4 == *y))
            })
            .map(|(y, _)| y)
            .collect()
    }

    /// Queues a (re)light of a chunk once its neighborhood is loaded.
    fn request_light(&mut self, scene: &HandcraftedScene, chunk: ChunkPos) {
        let Some(hood) = Neighborhood::of(scene, chunk) else {
            return;
        };
        let epoch = self.light_epochs.entry(chunk).or_default();
        *epoch += 1;
        let job = LightJob {
            chunk,
            epoch: *epoch,
            hood,
        };
        self.lights_pending.insert(chunk);
        let (lock, ready) = &*self.work;
        let mut queue = lock.lock().expect("terrain work queue");
        queue.light.retain(|(queued, _)| queued.chunk != chunk);
        queue.light.push((job, self.states.clone()));
        ready.notify_one();
    }

    /// The world generation data the chunk map runs.
    pub fn world_gen(&self) -> Arc<minecraftoss_world::chunk_map::WorldGen> {
        self.server.world_gen().clone()
    }

    pub fn queue_save(&mut self) -> usize {
        self.server.queue_unsaved()
    }

    /// The storage chunks are saved to, which the level's entities share.
    pub fn storage(&self) -> Option<Arc<minecraftoss_world::storage::ChunkStorage>> {
        self.server.storage()
    }

    pub fn loaded_chunk(&mut self, pos: minecraftoss_core::ChunkPos) -> Option<Arc<Chunk>> {
        self.server.chunk(pos).cloned()
    }

    pub fn load_chunk_now(&mut self, pos: minecraftoss_core::ChunkPos) -> Arc<Chunk> {
        self.server.load_now(pos)
    }

    /// One server tick: runs the chunk map for the player and applies what it
    /// sends to the scene. Returns the chunks it loaded and the positions it
    /// forgot, for the world simulation.
    pub fn server_tick(&mut self, player: BlockPos, scene: &mut HandcraftedScene) -> (Vec<Arc<Chunk>>, Vec<minecraftoss_core::ChunkPos>) {
        let player_chunk = minecraftoss_core::ChunkPos::new(player.0 >> 4, player.2 >> 4);
        let (mut loaded, mut forgotten) = (Vec::new(), Vec::new());
        // Columns whose queued work is cancelled, in one pass at the end.
        let mut cancelled: crate::fast_hash::FxHashSet<ChunkPos> = Default::default();
        let events = {
            let _span = span("chunk_map.tick");
            self.server.tick(player_chunk)
        };
        let [tracking, collecting, lock_wait, scheduling, sending, locked, evicting] = self.server.last_tick_ms;
        crate::frame_spans::record("  chunk_map: schedule under lock", locked);
        crate::frame_spans::record("  chunk_map: evict", evicting);
        crate::frame_spans::record("  chunk_map: tracking", tracking);
        crate::frame_spans::record("  chunk_map: collect", collecting);
        crate::frame_spans::record("  chunk_map: world lock wait", lock_wait);
        crate::frame_spans::record("  chunk_map: schedule", scheduling);
        crate::frame_spans::record("  chunk_map: send", sending);
        for event in events {
            match event {
                ChunkEvent::Center(_) => {}
                ChunkEvent::Load(chunk) => {
                    let _span = span("chunk load (client)");
                    loaded.push(chunk.clone());
                    let pos = (chunk.pos.x, chunk.pos.z);
                    let empty = {
                        let _span = span("  empty_sections");
                        self.empty_sections(scene, &chunk)
                    };
                    {
                        let _span = span("  scene.insert_chunk");
                        scene.insert_chunk(chunk);
                    }
                    self.sections.chunk_loaded(pos, empty);
                    for x in pos.0 - 1..=pos.0 + 1 {
                        for z in pos.1 - 1..=pos.1 + 1 {
                            if !self.lights.contains_key(&(x, z)) && !self.lights_pending.contains(&(x, z)) {
                                self.request_light(scene, (x, z));
                            }
                        }
                    }
                }
                ChunkEvent::Forget(pos) => {
                    let _span = span("chunk forget (client)");
                    forgotten.push(pos);
                    let pos = (pos.x, pos.z);
                    if scene.generated_chunk(pos).is_none() {
                        continue;
                    }
                    scene.remove_chunk(pos);
                    self.removed.extend(self.sections.chunk_unloaded(pos));
                    *self.light_epochs.entry(pos).or_default() += 1;
                    self.lights_pending.remove(&pos);
                    self.player_relights.remove(&pos);
                    self.player_sections.retain(|s| (s.0, s.2) != pos);
                    if self.lights.remove(&pos).is_some() {
                        self.light_changes.push((pos, None));
                    }
                    cancelled.insert(pos);
                }
            }
        }
        if !cancelled.is_empty() {
            let _span = span("  cancel queued work");
            let (lock, _) = &*self.work;
            let mut queue = lock.lock().expect("terrain work queue");
            queue.light.retain(|(job, _)| !cancelled.contains(&job.chunk));
            queue.compile.retain(|section, _| !cancelled.contains(&(section.0, section.2)));
        }
        (loaded, forgotten)
    }

    /// Hands edited positions to the integrated server, whose chunks are
    /// the ones saved: each takes the block the scene now shows there.
    pub fn record_edits(&mut self, scene: &HandcraftedScene, positions: &[BlockPos]) {
        let edits: Vec<(minecraftoss_core::BlockPos, minecraftoss_core::BlockStateId)> = positions
            .iter()
            .map(|&(x, y, z)| {
                let state = Scene::block(scene, (x, y, z)).and_then(|b| self.states.state_of(b)).unwrap_or(minecraftoss_core::BlockStateId::AIR);
                (minecraftoss_core::BlockPos::new(x, y, z), state)
            })
            .collect();
        let _span = span("  edits: chunk map");
        self.server.set_blocks(&edits);
    }

    pub fn record_block_entities(&mut self, changes: &[(BlockPos, Option<minecraftoss_core::nbt::Tag>)]) {
        if changes.is_empty() {
            return;
        }
        let changes: Vec<_> = changes
            .iter()
            .map(|((x, y, z), tag)| (minecraftoss_core::BlockPos::new(*x, *y, *z), tag.clone()))
            .collect();
        self.server.set_block_entities(&changes);
    }

    /// An edit at these positions: nearby sections rebuild for the player
    /// (`LevelRenderer.setBlockDirty`), and nearby chunks relight; sections
    /// whose light changed rebuild when that finishes.
    pub fn mark_edited(&mut self, scene: &HandcraftedScene, positions: &[BlockPos]) {
        let mut sections = BTreeSet::new();
        let mut chunks = BTreeSet::new();
        let mut touched: BTreeMap<crate::sections::SectionPos, bool> = BTreeMap::new();
        for &(x, y, z) in positions {
            for sx in (x - 1) >> 4..=(x + 1) >> 4 {
                for sy in (y - 1) >> 4..=(y + 1) >> 4 {
                    for sz in (z - 1) >> 4..=(z + 1) >> 4 {
                        sections.insert((sx, sy, sz));
                    }
                }
            }
            for cx in (x - 15) >> 4..=(x + 15) >> 4 {
                for cz in (z - 15) >> 4..=(z + 15) >> 4 {
                    chunks.insert((cx, cz));
                }
            }
            // A section holding any changed non-air block is not empty.
            let section = (x >> 4, y >> 4, z >> 4);
            let placed = Scene::block(scene, (x, y, z)).is_some();
            let entry = touched.entry(section).or_insert(false);
            *entry |= placed;
        }
        // Whether each touched section now holds only air, checked once.
        let empty_span = span("  edits: emptiness");
        for (section, placed) in touched {
            let empty = !placed && scene.section_is_empty(section);
            self.sections.set_empty(section, empty);
        }
        drop(empty_span);
        for pos in sections {
            if self.sections.section(pos).is_some() {
                self.sections.set_dirty(pos, true);
                self.player_sections.insert(pos);
            }
        }
        let _span = span("  edits: relight requests");
        for chunk in chunks {
            if self.lights.contains_key(&chunk) {
                self.player_relights.insert(chunk);
                self.request_light(scene, chunk);
            }
        }
    }

    /// Rebuilds every section, after a resource reload.
    pub fn remesh_all(&mut self, scene: &HandcraftedScene) {
        for chunk in scene.generated_chunks().collect::<Vec<_>>() {
            self.sections.set_range_dirty(chunk, false);
        }
    }

    /// Whether every edit-triggered rebuild has finished.
    pub fn edits_settled(&self) -> bool {
        self.player_sections.is_empty() && self.player_relights.is_empty()
    }

    pub fn draw_apart(&mut self, pos: BlockPos, apart: bool) {
        if let Some(section) = self.apart.set(pos, apart) {
            if self.sections.section(section).is_some() {
                self.sections.set_dirty(section, true);
                self.player_sections.insert(section);
                self.serials.remove(&section);
            }
        }
    }

    pub fn cell_settled(&self, (x, y, z): BlockPos) -> bool {
        !self.player_sections.contains(&(x >> 4, y >> 4, z >> 4))
    }

    pub fn set_tickets(&mut self, tickets: &[(minecraftoss_core::ChunkPos, i32)]) {
        self.server.set_tickets(tickets);
    }

    /// Everything in view is generated, sent, lit and built.
    pub fn is_settled(&self) -> bool {
        let queue = self.work.0.lock().expect("terrain work queue");
        self.server.is_idle()
            && queue.light.is_empty()
            && queue.compile.is_empty()
            && queue.active == 0
            && self.lights_pending.is_empty()
            && self.scheduled_last_frame == 0
    }

    /// One frame: collect finished work, update the section graph from the
    /// camera, schedule what vanilla would build next, and report renderer
    /// changes.
    pub fn frame(
        &mut self,
        scene: &HandcraftedScene,
        camera: &CullCamera,
        fade_millis: u64,
        atlas: &Arc<Atlas>,
        packs: &PackStack,
    ) -> FrameUpdate {
        let now = self.now();
        let mut update = FrameUpdate {
            removed: std::mem::take(&mut self.removed),
            ..FrameUpdate::default()
        };
        let drain_span = span("terrain.frame drain");
        while let Ok(done) = self.done.try_recv() {
            match done {
                Done::Light { chunk, epoch, light, micros } => {
                    let _span = span("  drain light");
                    if self.light_epochs.get(&chunk) != Some(&epoch) || scene.generated_chunk(chunk).is_none() {
                        continue;
                    }
                    self.stats.lit += 1;
                    self.stats.light_micros += micros;
                    self.lights_pending.remove(&chunk);
                    let from_player = self.player_relights.remove(&chunk);
                    if let Some(old) = self.lights.insert(chunk, light.clone()) {
                        let _span = span("  drain light differs");
                        // A light section update rebuilds that section and its
                        // neighbors (`setSectionDirtyWithNeighbors`).
                        let (min_section, max_section) = self.states.section_range();
                        for sy in old.differing_sections(&light, chunk, min_section..=max_section) {
                            {
                                for dx in -1..=1 {
                                    for dy in -1..=1 {
                                        for dz in -1..=1 {
                                            let pos = (chunk.0 + dx, sy + dy, chunk.1 + dz);
                                            self.sections.set_dirty(pos, from_player);
                                            if from_player && self.sections.section(pos).is_some() {
                                                self.player_sections.insert(pos);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    self.light_changes.push((chunk, Some(light)));
                }
                Done::Compile { section, serial, pack_generation, micros, outcome } => {
                    let _span = span("  drain compile");
                    if self.serials.get(&section) != Some(&serial) || pack_generation != packs.generation {
                        continue;
                    }
                    self.serials.remove(&section);
                    self.player_sections.remove(&section);
                    self.stats.compiled += 1;
                    self.stats.compile_micros += micros;
                    match outcome {
                        Ok((mesh, visibility, quads)) => {
                            if self.sections.section(section).is_some() {
                                self.sections.compiled(section, visibility, now);
                                self.translucent.compiled(section, quads);
                                update.uploads.push((section, mesh));
                            }
                        }
                        Err(e) => eprintln!("section {section:?} could not build: {e:#}"),
                    }
                }
            }
        }
        drop(drain_span);
        update.lights = std::mem::take(&mut self.light_changes);
        {
            let _span = span("sections.reposition");
            update.removed.extend(self.sections.reposition(camera));
        }
        for &section in &update.removed {
            self.translucent.removed(section);
        }
        let lights = &self.lights;
        let update_span = span("sections.update");
        let scheduled = self.sections.update(camera, now, |(x, _, z)| {
            lights.contains_key(&(x, z))
                && (-1..=1).all(|dx| (-1..=1).all(|dz| scene.generated_chunk((x + dx, z + dz)).is_some()))
        });
        drop(update_span);
        let _schedule_span = span("terrain.frame schedule");
        let context = JobContext {
            states: self.states.clone(),
            atlas: atlas.clone(),
            pack_sources: packs.sources().to_vec(),
            pack_generation: packs.generation,
        };
        let mut jobs = Vec::new();
        for (pos, recompile) in scheduled.compile {
            let chunk = (pos.0, pos.2);
            let (Some(hood), Some(light)) = (Neighborhood::of(scene, chunk), self.lights.get(&chunk)) else {
                // Wait for the missing neighbor or light.
                self.sections.set_dirty(pos, false);
                continue;
            };
            self.next_serial += 1;
            self.serials.insert(pos, self.next_serial);
            jobs.push((
                pos,
                recompile,
                CompileJob {
                    serial: self.next_serial,
                    hood,
                    light: light.clone(),
                    context: context.clone(),
                    apart: self.apart.in_section(pos),
                },
            ));
        }
        // Sections still waiting on a neighbour or light are not work.
        self.scheduled_last_frame = jobs.len();
        {
            let (lock, ready) = &*self.work;
            let mut queue = lock.lock().expect("terrain work queue");
            queue.eye = camera.position;
            // Scheduling a section cancels its earlier task.
            if !jobs.is_empty() {
                let rescheduled: crate::fast_hash::FxHashSet<SectionPos> = jobs.iter().map(|(pos, _, _)| *pos).collect();
                queue.compile.retain(|queued, _| !rescheduled.contains(&queued));
            }
            for (pos, recompile, job) in jobs {
                queue.compile.push(pos, recompile, job);
            }
            if !queue.compile.is_empty() || !queue.light.is_empty() {
                ready.notify_all();
            }
        }
        update.visible = self
            .sections
            .visible()
            .iter()
            .filter_map(|&pos| {
                let section = self.sections.section(pos)?;
                matches!(section.mesh, crate::sections::MeshState::Compiled(_))
                    .then(|| (pos, section.visibility(now, fade_millis)))
            })
            .collect();
        let _resort_span = span("terrain.frame resort translucency");
        update.uploads.extend(self.translucent.resorts(camera.position, &update.visible));
        update
    }

    /// Diagnostics (F9): every chunk column within the view distance, by
    /// where it stands in the pipeline, nearest examples first.
    pub fn hole_report(&self, scene: &HandcraftedScene, gpu_has: &dyn Fn(SectionPos) -> bool) -> String {
        use std::collections::BTreeMap;
        let (cx, _, cz) = self.sections.camera_section();
        let vd = self.server.view_distance();
        let mut columns: BTreeMap<&'static str, Vec<(i32, ChunkPos, String)>> = BTreeMap::new();
        let mut sections: BTreeMap<&'static str, Vec<(i32, SectionPos)>> = BTreeMap::new();
        let (min_section, max_section) = self.states.section_range();
        for x in cx - vd..=cx + vd {
            for z in cz - vd..=cz + vd {
                let d = (x - cx).abs().max((z - cz).abs());
                let pos = (x, z);
                let category = if scene.generated_chunk(pos).is_none() {
                    "not in scene (server has not sent it)"
                } else if !self.lights.contains_key(&pos) {
                    if self.lights_pending.contains(&pos) { "no light (pending)" } else { "no light (never requested)" }
                } else {
                    "in scene and lit"
                };
                let server = if category.starts_with("not in scene") { self.server.debug_slot(minecraftoss_core::ChunkPos::new(x, z)) } else { String::new() };
                columns.entry(category).or_default().push((d, pos, server));
                if category != "in scene and lit" {
                    continue;
                }
                for y in min_section..=max_section {
                    let section = (x, y, z);
                    let label = match self.sections.debug_section(section) {
                        None if self.sections.is_empty_section(section) => continue,
                        None => "no section yet (not reached: occluded or pending)",
                        Some((mesh, dirty, in_graph, reached, visible)) => match mesh {
                            crate::sections::MeshState::Empty => continue,
                            crate::sections::MeshState::Uncompiled if self.serials.contains_key(&section) => "uncompiled, compile queued",
                            crate::sections::MeshState::Uncompiled if dirty && !in_graph => "uncompiled, not reached by graph",
                            crate::sections::MeshState::Uncompiled if dirty && !visible => "uncompiled, reached but not in frustum",
                            crate::sections::MeshState::Uncompiled if dirty => "uncompiled, visible and dirty (not ready?)",
                            crate::sections::MeshState::Uncompiled => "uncompiled, not dirty",
                            crate::sections::MeshState::Compiled(_) if !reached => "compiled, not reached by graph",
                            crate::sections::MeshState::Compiled(_) if visible && !gpu_has(section) => "compiled and visible, not on the GPU",
                            crate::sections::MeshState::Compiled(_) => continue,
                        },
                    };
                    sections.entry(label).or_default().push((d, section));
                }
            }
        }
        let mut out = format!("hole report at camera section {:?}, view distance {vd}\n{}\n", self.sections.camera_section(), self.server.debug_queue());
        for (category, mut list) in columns {
            list.sort();
            out += &format!("  columns {category}: {}\n", list.len());
            if category != "in scene and lit" {
                for (d, pos, server) in list.iter().take(12) {
                    out += &format!("      d{d} {pos:?} {server}\n");
                }
            }
        }
        for (label, mut list) in sections {
            list.sort();
            out += &format!("  sections {label}: {}\n", list.len());
            for (d, pos) in list.iter().take(12) {
                out += &format!("      d{d} {pos:?}\n");
            }
        }
        out
    }

    /// Loading progress around `center` for the load profile: how many
    /// rings are complete at each stage, and the pipeline's counters.
    pub fn load_metrics(&self, scene: &HandcraftedScene, center: ChunkPos) -> LoadMetrics {
        let vd = self.server.view_distance();
        // The tracked area is a circle (`ChunkTrackingView`); a column can
        // be lit once its whole 3x3 is tracked.
        let view = minecraftoss_world::TrackingView::new(minecraftoss_core::ChunkPos::new(center.0, center.1), vd);
        let tracked = |x: i32, z: i32| view.contains(minecraftoss_core::ChunkPos::new(x, z));
        let (mut columns, mut missing_sent, mut missing_lit) = (0, 0, 0);
        let (mut sent_radius, mut lit_radius) = (f64::from(vd + 1), f64::from(vd + 1));
        for x in center.0 - vd - 1..=center.0 + vd + 1 {
            for z in center.1 - vd - 1..=center.1 + vd + 1 {
                if !tracked(x, z) {
                    continue;
                }
                columns += 1;
                let distance = f64::from((x - center.0).pow(2) + (z - center.1).pow(2)).sqrt();
                let sent = scene.generated_chunk((x, z)).is_some();
                if !sent {
                    missing_sent += 1;
                    sent_radius = sent_radius.min(distance);
                }
                let litable = (-1..=1).all(|dx| (-1..=1).all(|dz| tracked(x + dx, z + dz)));
                if litable && !(sent && self.lights.contains_key(&(x, z))) {
                    missing_lit += 1;
                    lit_radius = lit_radius.min(distance);
                }
            }
        }
        let visible = self.sections.visible();
        let visible_compiled = visible
            .iter()
            .filter(|&&pos| self.sections.section(pos).is_some_and(|s| matches!(s.mesh, crate::sections::MeshState::Compiled(_))))
            .count();
        let stats = self.server.stats();
        let queue = self.work.0.lock().expect("terrain work queue");
        LoadMetrics {
            columns,
            missing_sent,
            missing_lit,
            sent_radius,
            lit_radius,
            visible: visible.len(),
            visible_compiled,
            generated: stats.generated,
            decorated: stats.decorated,
            sent: scene.generated_chunks().count(),
            lit: self.stats.lit,
            compiled: self.stats.compiled,
            light_queue: queue.light.len(),
            compile_queue: queue.compile.len(),
            generation_micros: stats.mean_generation_micros,
            decoration_micros: stats.mean_decoration_micros,
            light_micros: self.stats.light_micros.checked_div(self.stats.lit).unwrap_or(0),
            compile_micros: self.stats.compile_micros.checked_div(self.stats.compiled).unwrap_or(0),
        }
    }

    /// Detailed state for the diagnostics log.
    pub fn diag(&self) -> String {
        format!(
            "{} | {} | lights {} pending {} | player sections {} relights {}",
            self.debug_line(),
            self.sections.diag(),
            self.lights.len(),
            self.lights_pending.len(),
            self.player_sections.len(),
            self.player_relights.len(),
        )
    }

    /// The F3 chunk lines: server and client progress.
    pub fn debug_line(&self) -> String {
        let stats = self.server.stats();
        let queue = self.work.0.lock().expect("terrain work queue");
        format!(
            "Chunks: {} loaded, {} queued, {} generating ({} us/chunk); light {} queued ({} us); sections {} visible, {} queued, {} building ({} us/section)",
            stats.loaded,
            stats.queued,
            stats.generating,
            stats.mean_generation_micros,
            queue.light.len(),
            self.stats.light_micros.checked_div(self.stats.lit).unwrap_or(0),
            self.sections.visible().len(),
            queue.compile.len(),
            queue.active,
            self.stats.compile_micros.checked_div(self.stats.compiled).unwrap_or(0),
        )
    }
}

/// Loading progress for the load profile (`TerrainStream::load_metrics`).
#[derive(Clone, Copy, Debug, Default)]
pub struct LoadMetrics {
    /// Tracked columns (a circle around the player), and how many of them
    /// are not sent yet, or not lit (of those whose 3x3 is tracked).
    pub columns: usize,
    pub missing_sent: usize,
    pub missing_lit: usize,
    /// The distance in chunks to the nearest tracked column not sent, and
    /// not lit (one more than the view distance when there is none).
    pub sent_radius: f64,
    pub lit_radius: f64,
    /// Sections the graph reaches in the frustum, and how many are built.
    pub visible: usize,
    pub visible_compiled: usize,
    pub generated: u64,
    pub decorated: u64,
    pub sent: usize,
    pub lit: u64,
    pub compiled: u64,
    pub light_queue: usize,
    pub compile_queue: usize,
    pub generation_micros: u64,
    pub decoration_micros: u64,
    pub light_micros: u64,
    pub compile_micros: u64,
}

/// A temporary world directory for a session without one.
struct SessionDir(std::path::PathBuf);

impl SessionDir {
    fn new() -> Result<Self> {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("minecraftoss-session-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }
}

impl Drop for SessionDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Drop for TerrainStream {
    fn drop(&mut self) {
        self.work.0.lock().expect("terrain work queue").stop = true;
        self.work.1.notify_all();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test --release -p minecraftoss-viewer --lib section_mesh -- --ignored --nocapture`
    #[test]
    #[ignore = "needs the local data pack, block catalog and resource pack; slow in debug"]
    fn section_meshes_match_the_scene_mesher() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let pack = paths
            .datapack
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|root| root.join("resourcepacks/local/minecraft-26.3"));
        let Some(pack) = pack.filter(|p| p.is_dir() && paths.block_catalog.is_file()) else {
            eprintln!("skipping: local data not restored");
            return;
        };
        let registries = Arc::new(Registries::load(&paths).unwrap());
        let seed = 1234;
        let generator = Arc::new(TerrainGenerator::overworld(registries.clone(), seed).unwrap());
        let states = Arc::new(
            BlockStates::new(registries, seed, generator.min_y, generator.height).unwrap(),
        );
        let mut server = ChunkMap::new(generator, 2, 4);
        let origin = minecraftoss_core::ChunkPos::new(0, 0);
        while {
            server.tick(origin);
            !server.is_idle()
        } {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut scene = HandcraftedScene::streamed(states.clone());
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = server.chunk(minecraftoss_core::ChunkPos::new(x, z)).unwrap();
                scene.insert_chunk(chunk.clone());
            }
        }
        // An edit shows through both paths.
        scene.set((3, 70, 4), Some(Block::new("minecraft:glass")));
        let packs = PackStack::open(vec![pack]).unwrap();
        let reference = mesh::build(&scene, &packs).unwrap();
        let tint = BiomeTint::from_pack(&packs).unwrap();
        let hood = Neighborhood::of(&scene, (0, 0)).unwrap();
        let started = Instant::now();
        let light = light_chunk(&states, &hood);
        eprintln!("chunk light: {:?}", started.elapsed());
        let global = reference.sky_light.as_ref().unwrap();
        for x in -1..17 {
            for z in -1..17 {
                for y in -65..330 {
                    assert_eq!(light.get((x, y, z)), global.get((x, y, z)), "sky {x} {y} {z}");
                    assert_eq!(light.get_block((x, y, z)), global.get_block((x, y, z)), "block {x} {y} {z}");
                }
            }
        }
        let mut models = ModelCache::default();
        let started = Instant::now();
        let mut vertices = Vec::new();
        let mut opaque = 0;
        for sy in -4..20 {
            let (mesh, _) =
                compile_section(&states, &hood, (0, sy, 0), &light, &mut models, &packs, &reference.atlas, &tint, &[]).unwrap();
            opaque += mesh.transparent_start.unwrap_or(mesh.indices.len() as u32) as usize / 6;
            vertices.extend(mesh.vertices);
        }
        eprintln!("24 sections: {:?}", started.elapsed());
        let expected = &reference.chunks[&(0, 0)];
        // Sections emit the same faces in a different order.
        let key = |v: &mesh::Vertex| format!("{v:?}");
        let mut got: Vec<String> = vertices.iter().map(key).collect();
        let mut want: Vec<String> = expected.vertices.iter().map(key).collect();
        got.sort();
        want.sort();
        assert_eq!(got.len(), want.len());
        assert!(got == want, "section vertices differ from the chunk mesher");
        assert_eq!(opaque, expected.transparent_start.unwrap_or(expected.indices.len() as u32) as usize / 6);
    }

    #[test]
    fn block_states_parse_into_scene_blocks() {
        let block = block_from_state("minecraft:grass_block[snowy=false]").unwrap();
        assert_eq!(block, Block::new("minecraft:grass_block").with("snowy", "false"));
        assert_eq!(color(&serde_json::json!("#3f76e4")), Some([0x3f, 0x76, 0xe4]));
        assert_eq!(color(&serde_json::json!(4159204)), Some([0x3f, 0x76, 0xe4]));
    }

    #[test]
    fn swamp_leaf_litter_reads_the_biome_dry_foliage_colour() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let registries = Registries::load(&paths).unwrap();
        let swamp = registries.biomes.id("minecraft:swamp").map(|id| biome_sample(registries.biomes.get(id))).unwrap();
        assert_eq!(swamp.dry_foliage_color, Some([0x7b, 0x53, 0x34]));
        assert_eq!(plains_sample(&registries).dry_foliage_color, None);
    }

    #[test]
    fn only_full_collision_blocks_shade_corners() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let registries = Registries::load(&paths).unwrap();
        let blocks = &registries.blocks;
        let darkens = |text: &str| {
            let state = blocks.parse_state(text).unwrap();
            shade_darkens(blocks, state, &block_from_state(&blocks.state_to_string(state)).unwrap())
        };
        for solid in ["minecraft:grass_block", "minecraft:stone", "minecraft:oak_leaves", "minecraft:mud", "minecraft:snow[layers=8]"] {
            assert!(darkens(solid), "{solid}");
        }
        for open in ["minecraft:short_grass", "minecraft:poppy", "minecraft:tall_grass", "minecraft:glass", "minecraft:red_stained_glass", "minecraft:snow[layers=2]", "minecraft:oak_slab", "minecraft:barrier"] {
            assert!(!darkens(open), "{open}");
        }
    }

    #[test]
    fn block_entities_signs_and_beds_bake_into_sections_like_the_slow_path() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Some(pack) = paths.datapack.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(|root| root.join("resourcepacks/local/minecraft-26.3")).filter(|p| p.is_dir()) else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let packs = PackStack::open(vec![pack]).unwrap();
        let states = Arc::new(BlockStates::new(Arc::new(registries), 1, -64, 384).unwrap());
        let plains = states.registries().biomes.id("minecraft:plains").unwrap_or(BiomeId(0));
        let mut scene = HandcraftedScene::streamed(states.clone());
        for x in -1..=1 {
            for z in -1..=1 {
                scene.insert_chunk(Arc::new(Chunk::new(minecraftoss_core::ChunkPos::new(x, z), -64, 384, plains)));
            }
        }
        let blocks = vec![
            Block::new("minecraft:chest").with("facing", "east"),
            Block::new("minecraft:trapped_chest").with("facing", "north"),
            Block::new("minecraft:ender_chest").with("facing", "west"),
            Block::new("minecraft:waxed_oxidized_copper_chest").with("facing", "south"),
            Block::new("minecraft:shulker_box").with("facing", "up"),
            Block::new("minecraft:red_shulker_box").with("facing", "north"),
            Block::new("minecraft:blue_shulker_box").with("facing", "down"),
            Block::new("minecraft:white_banner").with("rotation", "3"),
            Block::new("minecraft:red_wall_banner").with("facing", "east"),
            Block::new("minecraft:skeleton_skull").with("rotation", "5"),
            Block::new("minecraft:wither_skeleton_wall_skull").with("facing", "south"),
            Block::new("minecraft:zombie_head").with("rotation", "0"),
            Block::new("minecraft:creeper_head").with("rotation", "12"),
            Block::new("minecraft:dragon_head").with("rotation", "8"),
            Block::new("minecraft:piglin_wall_head").with("facing", "west"),
            Block::new("minecraft:player_head").with("rotation", "2"),
            Block::new("minecraft:conduit"),
            Block::new("minecraft:decorated_pot").with("facing", "west"),
            Block::new("minecraft:bell").with("attachment", "floor").with("facing", "north"),
            Block::new("minecraft:enchanting_table"),
            Block::new("minecraft:lectern").with("facing", "east").with("has_book", "true"),
            Block::new("minecraft:red_bed").with("facing", "north").with("part", "head"),
            Block::new("minecraft:red_bed").with("facing", "north").with("part", "foot"),
            Block::new("minecraft:oak_sign").with("rotation", "7"),
            Block::new("minecraft:spruce_wall_sign").with("facing", "north"),
            Block::new("minecraft:birch_hanging_sign").with("rotation", "4").with("attached", "true"),
            Block::new("minecraft:crimson_wall_hanging_sign").with("facing", "east"),
        ];
        let at = |i: usize| ((i % 7) as i32 * 2 + 1, 65, (i / 7) as i32 * 3 + 1);
        for (i, block) in blocks.iter().enumerate() {
            scene.set(at(i), Some(block.clone()));
        }
        scene.set((14, 65, 14), Some(Block::new("minecraft:chest").with("facing", "south").with("type", "left")));
        scene.set((13, 65, 14), Some(Block::new("minecraft:chest").with("facing", "south").with("type", "right")));
        let mut textures = std::collections::BTreeSet::new();
        for block in &blocks {
            let variants = resolve_block_variants(&packs, block).unwrap();
            for (model, _) in &variants {
                assert!(!model.elements.is_empty(), "{} has no geometry", block.id.key());
                textures.extend(model.elements.iter().flat_map(|e| e.faces.iter().map(|f| f.texture.clone())));
            }
        }
        textures.extend(crate::special_models::textures().unwrap());
        let atlas = mesh::make_atlas(&packs, textures.into_iter().collect()).unwrap();
        assert!(atlas.missing.is_empty(), "{:?}", atlas.missing);
        let hood = Neighborhood::of(&scene, (0, 0)).unwrap();
        let light = light_chunk(&states, &hood);
        let view = View::new(&states, &hood, (-1, 63, -1), (18, 18, 18));
        let mut models = ModelCache::default();
        models.prepare(&states, &packs, &atlas, &view);
        let tint = BiomeTint::from_pack(&packs).unwrap();
        let baked = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, true, &[]).unwrap();
        let slow = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, false, &[]).unwrap();
        let bits = |m: &ChunkMesh| -> Vec<u32> { m.vertices.iter().flat_map(|v| v.position.iter().chain(&v.uv).chain(&v.color).chain([&v.sky_light, &v.block_light]).map(|f| f.to_bits()).collect::<Vec<_>>()).collect() };
        assert!(bits(&baked.0) == bits(&slow.0) && baked.0.indices == slow.0.indices && baked.0.faces == slow.0.faces && baked.1 == slow.1);
        for (i, block) in blocks.iter().enumerate() {
            let (x, y, z) = at(i);
            let inside = baked.0.vertices.iter().filter(|v| {
                let [vx, vy, vz] = v.position;
                (x as f32..x as f32 + 1.0).contains(&vx) && (y as f32 - 0.1..y as f32 + 2.0).contains(&vy) && (z as f32..z as f32 + 1.0).contains(&vz)
            });
            assert!(inside.count() >= 4, "{} draws nothing", block.id.key());
        }
        let entity_faces = baked.0.vertices.iter().filter(|v| v.color[3] == crate::special_models::ENTITY_SHEET_ALPHA).count();
        assert!(entity_faces > 0);
    }

    #[test]
    fn sand_under_a_cactus_keeps_its_top_face_in_both_section_paths() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Some(pack) = paths.datapack.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(|root| root.join("resourcepacks/local/minecraft-26.3")).filter(|p| p.is_dir()) else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let packs = PackStack::open(vec![pack]).unwrap();
        let states = Arc::new(BlockStates::new(Arc::new(registries), 1, -64, 384).unwrap());
        let plains = states.registries().biomes.id("minecraft:plains").unwrap_or(BiomeId(0));
        let mut scene = HandcraftedScene::streamed(states.clone());
        for x in -1..=1 {
            for z in -1..=1 {
                scene.insert_chunk(Arc::new(Chunk::new(minecraftoss_core::ChunkPos::new(x, z), -64, 384, plains)));
            }
        }
        let column = [(1, "minecraft:sand"), (2, "minecraft:cactus"), (3, "minecraft:cactus"), (4, "minecraft:stone")];
        for (y, id) in column {
            scene.set((5, 64 + y, 5), Some(Block::new(id)));
        }
        scene.set((6, 66, 5), Some(Block::new("minecraft:ice")));
        scene.set((7, 66, 5), Some(Block::new("minecraft:ice")));
        assert_eq!(Scene::face_hidden(&scene, (5, 65, 5), (5, 66, 5), Direction::Up), Some(false));
        assert_eq!(Scene::face_hidden(&scene, (5, 66, 5), (5, 65, 5), Direction::Down), Some(true));
        assert_eq!(Scene::face_hidden(&HandcraftedScene::new(), (0, 0, 0), (0, 1, 0), Direction::Up), None);
        let mut textures = std::collections::BTreeSet::new();
        for id in ["minecraft:sand", "minecraft:cactus", "minecraft:stone", "minecraft:ice"] {
            for (model, _) in &resolve_block_variants(&packs, &Block::new(id)).unwrap() {
                textures.extend(model.elements.iter().flat_map(|e| e.faces.iter().map(|f| f.texture.clone())));
            }
        }
        let atlas = mesh::make_atlas(&packs, textures.into_iter().collect()).unwrap();
        let hood = Neighborhood::of(&scene, (0, 0)).unwrap();
        let light = light_chunk(&states, &hood);
        let view = View::new(&states, &hood, (-1, 63, -1), (18, 18, 18));
        let mut models = ModelCache::default();
        models.prepare(&states, &packs, &atlas, &view);
        let tint = BiomeTint::from_pack(&packs).unwrap();
        let baked = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, true, &[]).unwrap();
        let slow = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, false, &[]).unwrap();
        let bits = |m: &ChunkMesh| -> Vec<u32> { m.vertices.iter().flat_map(|v| v.position.iter().chain(&v.uv).chain(&v.color).chain([&v.sky_light, &v.block_light]).map(|f| f.to_bits()).collect::<Vec<_>>()).collect() };
        assert!(bits(&baked.0) == bits(&slow.0) && baked.0.indices == slow.0.indices && baked.0.faces == slow.0.faces && baked.1 == slow.1);
        let flat_faces_at = |plane: f32| {
            baked.0.vertices.chunks(4).filter(|quad| quad.iter().all(|v| v.position[1] == plane && (5.0..=6.0).contains(&v.position[0]) && (5.0..=6.0).contains(&v.position[2]))).count()
        };
        assert_eq!(flat_faces_at(66.0), 1, "the sand top shows around the cactus");
        assert_eq!(flat_faces_at(67.0), 0, "stacked cacti hide their shared faces");
        assert_eq!(flat_faces_at(68.0), 1, "the stone bottom shows around the cactus");
        let side_faces_at = |plane: f32| baked.0.vertices.chunks(4).filter(|quad| quad.iter().all(|v| v.position[0] == plane && (66.0..=67.0).contains(&v.position[1]))).count();
        assert_eq!(side_faces_at(6.0), 1, "the ice shows beside the cactus");
        assert_eq!(side_faces_at(7.0), 0, "ice hides the face it shares with ice");
    }

    #[test]
    fn blocks_turned_about_x_cull_with_their_vanilla_occlusion_shapes() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Some(pack) = paths.datapack.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(|root| root.join("resourcepacks/local/minecraft-26.3")).filter(|p| p.is_dir()) else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let packs = PackStack::open(vec![pack]).unwrap();
        let states = Arc::new(BlockStates::new(Arc::new(registries), 1, -64, 384).unwrap());
        let plains = states.registries().biomes.id("minecraft:plains").unwrap_or(BiomeId(0));
        let mut scene = HandcraftedScene::streamed(states.clone());
        for x in -1..=1 {
            for z in -1..=1 {
                scene.insert_chunk(Arc::new(Chunk::new(minecraftoss_core::ChunkPos::new(x, z), -64, 384, plains)));
            }
        }
        let top_stair = Block::new("minecraft:oak_stairs").with("facing", "north").with("half", "top").with("shape", "straight").with("waterlogged", "false");
        let log_along_x = Block::new("minecraft:oak_log").with("axis", "x");
        scene.set((5, 65, 5), Some(top_stair.clone()));
        scene.set((5, 66, 5), Some(Block::new("minecraft:oak_planks")));
        scene.set((5, 64, 5), Some(Block::new("minecraft:stone")));
        scene.set((9, 65, 5), Some(log_along_x.clone()));
        scene.set((10, 65, 5), Some(log_along_x.clone()));
        let mut textures = std::collections::BTreeSet::new();
        for block in [top_stair, log_along_x, Block::new("minecraft:oak_planks"), Block::new("minecraft:stone")] {
            for (model, _) in &resolve_block_variants(&packs, &block).unwrap() {
                textures.extend(model.elements.iter().flat_map(|e| e.faces.iter().map(|f| f.texture.clone())));
            }
        }
        let atlas = mesh::make_atlas(&packs, textures.into_iter().collect()).unwrap();
        let hood = Neighborhood::of(&scene, (0, 0)).unwrap();
        let light = light_chunk(&states, &hood);
        let view = View::new(&states, &hood, (-1, 63, -1), (18, 18, 18));
        let mut models = ModelCache::default();
        models.prepare(&states, &packs, &atlas, &view);
        let tint = BiomeTint::from_pack(&packs).unwrap();
        for baked in [true, false] {
            let (mesh, _, _) = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, baked, &[]).unwrap();
            let planks_bottom = mesh.vertices.chunks(4).filter(|quad| quad.iter().all(|v| v.position[1] == 66.0 && (5.0..=6.0).contains(&v.position[0]) && (5.0..=6.0).contains(&v.position[2]))).count();
            assert_eq!(planks_bottom, 0, "a top stair's full top and the planks over it hide each other");
            let full_square = |quad: &[mesh::Vertex], plane: f32| quad.iter().all(|v| v.position[1] == plane) && [0, 2].iter().all(|&axis| {
                let (low, high) = quad.iter().fold((f32::MAX, f32::MIN), |(low, high), v| (low.min(v.position[axis]), high.max(v.position[axis])));
                low == 5.0 && high == 6.0
            });
            let stone_top = mesh.vertices.chunks(4).filter(|quad| full_square(quad, 65.0)).count();
            assert_eq!(stone_top, 1, "the stone under a top stair's half-open bottom keeps its top");
            let between_logs = mesh.vertices.chunks(4).filter(|quad| quad.iter().all(|v| v.position[0] == 10.0 && (65.0..=66.0).contains(&v.position[1]))).count();
            assert_eq!(between_logs, 0, "logs lying side by side still hide their shared faces");
        }
    }

    #[test]
    fn water_plants_and_waterlogged_blocks_hold_water_in_both_section_paths() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Some(pack) = paths.datapack.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(|root| root.join("resourcepacks/local/minecraft-26.3")).filter(|p| p.is_dir()) else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let packs = PackStack::open(vec![pack]).unwrap();
        let states = Arc::new(BlockStates::new(Arc::new(registries), 1, -64, 384).unwrap());
        let state = |text: &str| states.registries().blocks.parse_state(text).unwrap();
        let source = Some(crate::fluid::FluidCell::source(crate::fluid::FluidKind::Water));
        for plant in ["minecraft:seagrass", "minecraft:tall_seagrass[half=upper]", "minecraft:kelp[age=3]", "minecraft:kelp_plant", "minecraft:bubble_column[drag=true]"] {
            assert_eq!(states.fluid_cell(state(plant)), source, "{plant}");
            assert!(states.holds_fluid(state(plant)), "{plant}");
        }
        assert!(states.holds_fluid(state("minecraft:oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=true]")));
        for dry in ["minecraft:water[level=0]", "minecraft:lava[level=0]", "minecraft:stone", "minecraft:oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=false]"] {
            assert!(!states.holds_fluid(state(dry)), "{dry}");
        }
        for fluid in ["minecraft:water[level=0]", "minecraft:water[level=3]", "minecraft:water[level=9]", "minecraft:lava[level=0]", "minecraft:lava[level=6]"] {
            assert_eq!(states.fluid_cell(state(fluid)), crate::fluid::FluidCell::from_block(states.block(state(fluid)).unwrap()), "{fluid}");
        }
        for (block, overlay) in [("minecraft:glass", true), ("minecraft:white_stained_glass", true), ("minecraft:tinted_glass", true), ("minecraft:ice", true), ("minecraft:slime_block", true), ("minecraft:oak_leaves", true), ("minecraft:stone", false), ("minecraft:glass_pane", false), ("minecraft:kelp", false)] {
            assert_eq!(states.fluid_overlay(state(block)), overlay, "{block}");
        }
        let plains = states.registries().biomes.id("minecraft:plains").unwrap_or(BiomeId(0));
        let mut scene = HandcraftedScene::streamed(states.clone());
        for x in -1..=1 {
            for z in -1..=1 {
                scene.insert_chunk(Arc::new(Chunk::new(minecraftoss_core::ChunkPos::new(x, z), -64, 384, plains)));
            }
        }
        for x in 2..=8 {
            for z in 2..=8 {
                scene.set((x, 64, z), Some(Block::new("minecraft:stone")));
            }
        }
        for x in 3..=7 {
            for z in 3..=7 {
                for y in 65..=66 {
                    scene.set((x, y, z), Some(Block::new("minecraft:water").with("level", "0")));
                }
            }
        }
        let stair = Block::new("minecraft:oak_stairs").with("facing", "east").with("half", "bottom").with("shape", "straight").with("waterlogged", "true");
        let placed = [
            ((5, 65, 5), Block::new("minecraft:kelp_plant")),
            ((5, 66, 5), Block::new("minecraft:kelp").with("age", "3")),
            ((4, 65, 4), Block::new("minecraft:seagrass")),
            ((6, 65, 4), Block::new("minecraft:tall_seagrass").with("half", "lower")),
            ((6, 66, 4), Block::new("minecraft:tall_seagrass").with("half", "upper")),
            ((7, 65, 6), stair.clone()),
        ];
        for (pos, block) in &placed {
            scene.set(*pos, Some(block.clone()));
        }
        assert!(states.fluid_face_hidden(states.state_of(&stair).unwrap(), Direction::West, 1.0), "the stair's back is a full face");
        assert_eq!(Scene::fluid_at(&scene, (5, 66, 5)), source);
        assert!(Scene::solid_render_at(&scene, (2, 64, 2)));
        assert!(!Scene::solid_render_at(&scene, (5, 66, 5)));
        let mut textures = std::collections::BTreeSet::new();
        for block in placed.iter().map(|(_, block)| block).chain([&Block::new("minecraft:stone")]) {
            for (model, _) in &resolve_block_variants(&packs, block).unwrap() {
                textures.extend(model.elements.iter().flat_map(|e| e.faces.iter().map(|f| f.texture.clone())));
            }
        }
        for name in ["water_still", "water_flow", "water_overlay"] {
            textures.insert(ResourceId::parse(&format!("minecraft:block/{name}")).unwrap());
        }
        let atlas = mesh::make_atlas(&packs, textures.into_iter().collect()).unwrap();
        let hood = Neighborhood::of(&scene, (0, 0)).unwrap();
        let light = light_chunk(&states, &hood);
        let view = View::new(&states, &hood, (-1, 63, -1), (18, 18, 18));
        let mut models = ModelCache::default();
        models.prepare(&states, &packs, &atlas, &view);
        let tint = BiomeTint::from_pack(&packs).unwrap();
        let baked = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, true, &[]).unwrap();
        let slow = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, false, &[]).unwrap();
        let bits = |m: &ChunkMesh| -> Vec<u32> { m.vertices.iter().flat_map(|v| v.position.iter().chain(&v.uv).chain(&v.color).chain([&v.sky_light, &v.block_light]).map(|f| f.to_bits()).collect::<Vec<_>>()).collect() };
        assert!(bits(&baked.0) == bits(&slow.0) && baked.0.indices == slow.0.indices && baked.0.faces == slow.0.faces && baked.1 == slow.1);
        let water: Vec<[[f32; 3]; 4]> = baked.1.chunks(6).filter(|six| six[1] == six[0] + 1).map(|six| std::array::from_fn(|i| baked.0.vertices[six[0] as usize + i].position)).collect();
        let centre = |quad: &[[f32; 3]; 4]| [0, 1, 2].map(|axis| quad.iter().map(|v| v[axis]).sum::<f32>() / 4.0);
        for quad in &water {
            let [x, y, z] = centre(quad);
            assert!(x < 3.01 || x > 7.99 || z < 3.01 || z > 7.99 || y > 66.5, "a water face inside the pool at {quad:?}");
        }
        let surface = 66.0 + 8.0 / 9.0 - 0.001;
        let tops: Vec<&[[f32; 3]; 4]> = water.iter().filter(|quad| quad.iter().all(|v| v[1] > 66.5)).collect();
        assert_eq!(tops.len(), 25, "every column has its surface, over the plants too");
        for v in tops.iter().flat_map(|quad| quad.iter()) {
            if (4.0..=7.0).contains(&v[0]) && (4.0..=7.0).contains(&v[2]) {
                assert!((v[1] - surface).abs() < 1e-4, "the surface stays level beside the plants: {v:?}");
            }
        }
        let east_side = |z: f32| water.iter().filter(|quad| quad.iter().all(|v| (v[0] - 7.999).abs() < 1e-4 && (z..=z + 1.0).contains(&v[2]) && v[1] < 66.01)).count();
        assert_eq!(east_side(5.0), 1, "open water shows its side");
        assert_eq!(east_side(6.0), 0, "the stair's back hides its own water");
        let preview = mesh::block_preview(&scene, &packs, &atlas, &light, &tint, (5, 66, 5)).unwrap();
        let start = preview.transparent_start.unwrap() as usize;
        assert!(start > 0 && preview.indices.len() > start, "the kelp preview shows the kelp and its water");
    }

    #[test]
    fn an_apart_shulker_box_leaves_its_cell_empty_and_every_other_face_alone() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Some(pack) = paths.datapack.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(|root| root.join("resourcepacks/local/minecraft-26.3")).filter(|p| p.is_dir()) else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let packs = PackStack::open(vec![pack]).unwrap();
        let states = Arc::new(BlockStates::new(Arc::new(registries), 1, -64, 384).unwrap());
        let plains = states.registries().biomes.id("minecraft:plains").unwrap_or(BiomeId(0));
        let mut scene = HandcraftedScene::streamed(states.clone());
        for x in -1..=1 {
            for z in -1..=1 {
                scene.insert_chunk(Arc::new(Chunk::new(minecraftoss_core::ChunkPos::new(x, z), -64, 384, plains)));
            }
        }
        let cell = (5, 66, 5);
        let box_block = Block::new("minecraft:red_shulker_box").with("facing", "up");
        scene.set(cell, Some(box_block.clone()));
        for (dx, dy, dz) in [(-1, 0, 0), (1, 0, 0), (0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1)] {
            scene.set((cell.0 + dx, cell.1 + dy, cell.2 + dz), Some(Block::new("minecraft:stone")));
        }
        let mut textures = std::collections::BTreeSet::new();
        for block in [box_block, Block::new("minecraft:stone")] {
            for (model, _) in &resolve_block_variants(&packs, &block).unwrap() {
                textures.extend(model.elements.iter().flat_map(|e| e.faces.iter().map(|f| f.texture.clone())));
            }
        }
        textures.extend(crate::special_models::textures().unwrap());
        let atlas = mesh::make_atlas(&packs, textures.into_iter().collect()).unwrap();
        let hood = Neighborhood::of(&scene, (0, 0)).unwrap();
        let light = light_chunk(&states, &hood);
        let view = View::new(&states, &hood, (-1, 63, -1), (18, 18, 18));
        let mut models = ModelCache::default();
        models.prepare(&states, &packs, &atlas, &view);
        let tint = BiomeTint::from_pack(&packs).unwrap();
        let bits = |m: &ChunkMesh| -> Vec<Vec<u32>> { m.vertices.iter().map(|v| v.position.iter().chain(&v.uv).chain(&v.color).chain([&v.sky_light, &v.block_light]).map(|f| f.to_bits()).collect()).collect() };
        let low = [cell.0 as f32, cell.1 as f32, cell.2 as f32];
        let within = |v: &mesh::Vertex| (0..3).all(|axis| v.position[axis] > low[axis] && v.position[axis] < low[axis] + 1.0);
        let inside = |m: &ChunkMesh| m.vertices.iter().filter(|v| within(v)).count();
        for baked in [true, false] {
            let whole = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, baked, &[]).unwrap();
            let apart = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, baked, &[cell]).unwrap();
            assert!(inside(&whole.0) > 0, "the box is drawn in its section");
            assert_eq!(inside(&apart.0), 0, "an apart box leaves its cell empty");
            let kept: Vec<Vec<u32>> = bits(&whole.0).into_iter().zip(&whole.0.vertices).filter(|(_, v)| !within(v)).map(|(b, _)| b).collect();
            assert_eq!(bits(&apart.0), kept, "every other face is the same");
            assert_eq!(apart.0.faces, whole.0.faces - inside(&whole.0) / 4);
        }
        let baked = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, true, &[cell]).unwrap();
        let slow = build_section_blocks(&states, &view, (0, 64, 0), &light, &models, &atlas, &tint, false, &[cell]).unwrap();
        assert!(bits(&baked.0) == bits(&slow.0) && baked.0.indices == slow.0.indices && baked.0.faces == slow.0.faces && baked.1 == slow.1);
    }

    #[test]
    fn a_saved_world_reopens_at_its_spawn_with_the_edits_an_autosave_queued() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let dir = crate::test_dir::tempdir().unwrap();
        let level = crate::level_storage::LevelStorage::open(dir.path()).unwrap();
        let spawn = (40, 90, -24);
        let mut stream = TerrainStream::for_dimension_at(registries.clone(), 77, 2, Dimension::Overworld, Some(level.root()), Some(spawn)).unwrap();
        assert_eq!(stream.world_spawn, spawn, "a known spawn is kept, not searched again");
        let (px, _, pz) = stream.player_spawn;
        assert!((px - 40.5).abs() <= 11.0 && (pz + 23.5).abs() <= 11.0, "the player spawns around it: {:?}", stream.player_spawn);
        let column = minecraftoss_core::ChunkPos::new(2, -2);
        stream.server.load_now(column);
        let gold = registries.blocks.parse_state("minecraft:gold_block").unwrap();
        let at = minecraftoss_core::BlockPos::new(40, 300, -24);
        stream.server.set_block(at, gold);
        assert!(stream.queue_save() >= 1, "the edited chunk is queued");
        assert_eq!(stream.queue_save(), 0, "and only once");
        let until = Instant::now() + std::time::Duration::from_secs(60);
        let saved = loop {
            let reader = level.chunk_storage("minecraft:overworld", registries.clone(), -64, 384);
            let found = reader.load(column).unwrap().map(|chunk| chunk.block(8, 300, 8));
            if found == Some(gold) || Instant::now() > until {
                break found;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert_eq!(saved, Some(gold), "the autosave reached the region file while the world stays open");
        drop(stream);
        let reopened = level.chunk_storage("minecraft:overworld", registries, -64, 384).load(column).unwrap().unwrap();
        assert_eq!(reopened.block(8, 300, 8), gold);
        assert!(dir.path().join("dimensions/minecraft/overworld/region/r.0.-1.mca").is_file());
    }

    fn count_blocks(registries: &Registries, chunk: &Chunk, counts: &mut HashMap<String, u32>) {
        let blocks = &registries.blocks;
        let min_y = chunk.min_section_y() * 16;
        let max_y = min_y + chunk.sections().len() as i32 * 16;
        for y in min_y..max_y {
            for z in 0..16usize {
                for x in 0..16usize {
                    let name = blocks.block(blocks.block_of(chunk.block(x, y, z))).name.to_string();
                    *counts.entry(name).or_default() += 1;
                }
            }
        }
    }

    fn count(counts: &HashMap<String, u32>, name: &str) -> u32 {
        counts.get(name).copied().unwrap_or(0)
    }

    #[test]
    fn nether_chunks_generate_light_and_tick_around_the_origin() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let dir = crate::test_dir::tempdir().unwrap();
        let level = crate::level_storage::LevelStorage::open(dir.path()).unwrap();
        let mut stream = TerrainStream::for_dimension_at(registries.clone(), 77, 2, Dimension::Nether, Some(level.root()), None).unwrap();
        assert!(!stream.states.sky_light);
        assert!(stream.states.nether_shade, "the Nether's cardinal lighting");
        assert_eq!(stream.states.section_range(), (0, 15), "the Nether is 256 blocks high from 0");
        assert_eq!(stream.player_spawn, (0.5, 64.0, 0.5));
        assert_eq!(stream.server.stats().generated, 0, "no stand-in spawn is searched");
        let blocks = &registries.blocks;
        let bedrock = blocks.block_by_name("minecraft:bedrock").unwrap();
        let mut counts = HashMap::new();
        let mut scene = HandcraftedScene::streamed(stream.states.clone());
        for cx in -2..=2 {
            for cz in -2..=2 {
                let chunk = stream.server.load_now(minecraftoss_core::ChunkPos::new(cx, cz));
                for zz in 0..16usize {
                    for xx in 0..16usize {
                        assert_eq!(blocks.block_of(chunk.block(xx, 0, zz)), bedrock, "bedrock floor at chunk {cx},{cz}");
                        assert_eq!(blocks.block_of(chunk.block(xx, 127, zz)), bedrock, "bedrock roof at chunk {cx},{cz}");
                        assert!(blocks.is_air(chunk.block(xx, 200, zz)), "nothing above the roof at chunk {cx},{cz}");
                    }
                }
                count_blocks(&registries, &chunk, &mut counts);
                scene.insert_chunk(chunk);
            }
        }
        for name in ["minecraft:netherrack", "minecraft:lava"] {
            assert!(count(&counts, name) > 0, "no {name} in {counts:?}");
        }
        for name in ["minecraft:stone", "minecraft:dirt", "minecraft:grass_block", "minecraft:water", "minecraft:end_stone"] {
            assert_eq!(count(&counts, name), 0, "{name} in the Nether");
        }
        assert!(count(&counts, "minecraft:netherrack") > count(&counts, "minecraft:lava"), "{counts:?}");
        let hood = Neighborhood::of(&scene, (0, 0)).expect("the chunks around 0,0 are loaded");
        light_chunk(&stream.states, &hood);
        let mut sim = crate::server::ServerSim::new(stream.world_gen(), stream.states.clone(), "minecraft:the_nether");
        for cx in -2..=2 {
            for cz in -2..=2 {
                sim.load_chunk(&stream.server.load_now(minecraftoss_core::ChunkPos::new(cx, cz)));
            }
        }
        sim.set_clocks(crate::server::ClockTicks { overworld: 6000, the_end: 0 });
        for _ in 0..40 {
            sim.tick();
        }
        assert_eq!(sim.weather_data(), None, "the Nether has no weather");
        drop(sim);
        drop(stream);
        assert!(dir.path().join("dimensions/minecraft/the_nether/region").is_dir(), "the Nether saves under its own dimension folder");
        let known = TerrainStream::for_dimension_at(registries.clone(), 77, 2, Dimension::Nether, Some(level.root()), Some((5, 70, -3))).unwrap();
        assert_eq!(known.world_spawn, (5, 70, -3), "a known Nether spawn is kept");
        assert_eq!(known.player_spawn, (5.5, 70.0, -2.5));
        assert_eq!(known.server.stats().generated, 0, "and nothing is searched");
    }

    #[test]
    fn end_chunks_generate_the_main_island_around_the_origin() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let dir = crate::test_dir::tempdir().unwrap();
        let level = crate::level_storage::LevelStorage::open(dir.path()).unwrap();
        let mut stream = TerrainStream::for_dimension_at(registries.clone(), 77, 2, Dimension::End, Some(level.root()), None).unwrap();
        assert_eq!(stream.world_spawn, (100, 49, 0), "ServerLevel.END_SPAWN_POINT");
        assert_eq!(stream.player_spawn, (100.5, 49.0, 0.5));
        assert_eq!(stream.server.stats().generated, 0, "the End needs no search");
        assert!(stream.states.sky_light);
        assert!(!stream.states.nether_shade);
        assert_eq!(stream.states.section_range(), (0, 15), "the End is 256 blocks high from 0");
        let blocks = &registries.blocks;
        let end_stone = blocks.block_by_name("minecraft:end_stone").unwrap();
        let mut counts = HashMap::new();
        let mut scene = HandcraftedScene::streamed(stream.states.clone());
        for cx in -2..=2 {
            for cz in -2..=2 {
                let chunk = stream.server.load_now(minecraftoss_core::ChunkPos::new(cx, cz));
                count_blocks(&registries, &chunk, &mut counts);
                scene.insert_chunk(chunk);
            }
        }
        let centre = stream.server.load_now(minecraftoss_core::ChunkPos::new(0, 0));
        assert!((0..256).any(|y| blocks.block_of(centre.block(8, y, 8)) == end_stone), "the main island covers 0,0");
        assert!(count(&counts, "minecraft:end_stone") > 10_000, "{counts:?}");
        assert!(count(&counts, "minecraft:obsidian") > 0, "the end spikes stand on the island: {counts:?}");
        for name in ["minecraft:stone", "minecraft:netherrack", "minecraft:dirt", "minecraft:water", "minecraft:lava"] {
            assert_eq!(count(&counts, name), 0, "{name} in the End around 0,0");
        }
        let hood = Neighborhood::of(&scene, (0, 0)).expect("the chunks around 0,0 are loaded");
        light_chunk(&stream.states, &hood);
        let mut sim = crate::server::ServerSim::new(stream.world_gen(), stream.states.clone(), "minecraft:the_end");
        for cx in -2..=2 {
            for cz in -2..=2 {
                sim.load_chunk(&stream.server.load_now(minecraftoss_core::ChunkPos::new(cx, cz)));
            }
        }
        for _ in 0..40 {
            sim.tick();
        }
        assert_eq!(sim.weather_data(), None, "the End has no weather");
        drop(sim);
        drop(stream);
        assert!(dir.path().join("dimensions/minecraft/the_end/region").is_dir(), "the End saves under its own dimension folder");
        let far = TerrainStream::for_dimension_at(registries.clone(), 77, 2, Dimension::End, Some(level.root()), Some((1200, 60, 0))).unwrap();
        assert_eq!(far.world_spawn, (1200, 60, 0), "a known End spawn is kept");
        assert_eq!(far.player_spawn, (1200.5, 60.0, 0.5));
    }

    #[test]
    fn nether_fortresses_bastions_and_every_nether_feature_generate() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let dir = crate::test_dir::tempdir().unwrap();
        let level = crate::level_storage::LevelStorage::open(dir.path()).unwrap();
        let mut stream = TerrainStream::for_dimension_at(registries.clone(), 77, 2, Dimension::Nether, Some(level.root()), Some((0, 64, 0))).unwrap();
        let gen = stream.world_gen();
        for name in ["minecraft:fortress", "minecraft:bastion_remnant", "minecraft:nether_fossil", "minecraft:ruined_portal_nether"] {
            let structure = gen.structures.defs.iter().find(|def| def.name == name).unwrap_or_else(|| panic!("{name} is registered"));
            assert!(!format!("{:?}", structure.kind).starts_with("Unsupported"), "{name} generates: {:?}", structure.kind);
        }
        let mut unsupported = Vec::new();
        for biome in ["nether_wastes", "crimson_forest", "warped_forest", "soul_sand_valley", "basalt_deltas"] {
            let id = minecraftoss_core::Identifier::parse(&format!("minecraft:{biome}")).unwrap();
            let json = registries.datapack.read_json("worldgen/biome", &id).unwrap();
            let names: Vec<String> = json["features"].as_array().into_iter().flatten().flat_map(|step| step.as_array().into_iter().flatten()).filter_map(|name| name.as_str().map(str::to_owned)).collect();
            assert!(!names.is_empty(), "{biome} has features");
            for name in names {
                let placed = gen.library.placed_by_name(&name).or_else(|| gen.library.placed_by_name(name.trim_start_matches("minecraft:"))).unwrap_or_else(|| panic!("{biome} feature {name} is loaded"));
                if let minecraftoss_generator::feature::kinds::Feature::Unsupported(kind) = gen.library.feature(gen.library.placed(placed).feature) {
                    unsupported.push(format!("{biome} {name}: {kind}"));
                }
            }
        }
        assert!(unsupported.is_empty(), "unsupported Nether features: {unsupported:?}");
        let deadline = Instant::now() + std::time::Duration::from_secs(120);
        let blocks = &registries.blocks;
        let mut sim = crate::server::ServerSim::new(gen.clone(), stream.states.clone(), "minecraft:the_nether");
        let mut found = Vec::new();
        for (structure, own, block_entity) in [
            ("minecraft:fortress", ["minecraft:nether_bricks", "minecraft:nether_brick_fence", "minecraft:spawner"], "minecraft:blaze"),
            ("minecraft:bastion_remnant", ["minecraft:polished_blackstone_bricks", "minecraft:blackstone", "minecraft:gilded_blackstone"], "minecraft:chests/bastion"),
        ] {
            let at = match crate::locate::nearest_structure(&gen, &[structure.to_owned()], [0, 64, 0], crate::locate::STRUCTURE_RADIUS, deadline) {
                crate::locate::Search::Found(at, _) => at,
                other => panic!("no {structure} near the origin of seed 77: {other:?}"),
            };
            let (cx, cz) = (at[0] >> 4, at[2] >> 4);
            let mut counts = HashMap::new();
            let mut tags = Vec::new();
            let mut mobs = Vec::new();
            for x in cx - 2..=cx + 2 {
                for z in cz - 2..=cz + 2 {
                    let chunk = stream.server.load_now(minecraftoss_core::ChunkPos::new(x, z));
                    for xx in 0..16usize {
                        for zz in 0..16usize {
                            assert!(blocks.is_air(chunk.block(xx, 200, zz)), "nothing above the roof at chunk {x},{z}");
                        }
                    }
                    count_blocks(&registries, &chunk, &mut counts);
                    tags.extend(chunk.block_entities.pending.values().chain(chunk.block_entities.entities.values()).map(|tag| format!("{tag:?}")));
                    mobs.extend(chunk.generation.entities.iter().flat_map(minecraftoss_world::natural_spawner::tick::census_of));
                    sim.load_chunk(&chunk);
                }
            }
            for name in own {
                assert!(count(&counts, name) > 0, "no {name} around the {structure} at {at:?}: {counts:?}");
            }
            assert!(tags.iter().any(|tag| tag.contains(block_entity)), "no block entity with {block_entity} around the {structure}: {tags:?}");
            for name in ["minecraft:stone", "minecraft:grass_block", "minecraft:water", "minecraft:end_stone"] {
                assert_eq!(count(&counts, name), 0, "{name} around the {structure}");
            }
            let kinds: Vec<(&str, bool)> = mobs.iter().map(|mob| (mob.kind.as_str(), mob.persistent)).collect();
            if structure == "minecraft:bastion_remnant" {
                assert!(kinds.contains(&("minecraft:piglin", true)) && kinds.contains(&("minecraft:piglin_brute", true)), "a bastion holds piglins and brutes: {kinds:?}");
            }
            assert!(kinds.iter().filter(|(kind, _)| *kind != "minecraft:strider").all(|(_, persistent)| *persistent), "the {structure}'s mobs never despawn: {kinds:?}");
            found.push((structure, at));
        }
        sim.set_clocks(crate::server::ClockTicks { overworld: 6000, the_end: 0 });
        for _ in 0..40 {
            sim.tick();
        }
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn a_bastion_s_piglins_wake_up() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let dir = crate::test_dir::tempdir().unwrap();
        let level = crate::level_storage::LevelStorage::open(dir.path()).unwrap();
        let mut stream = TerrainStream::for_dimension_at(registries.clone(), 77, 2, Dimension::Nether, Some(level.root()), Some((0, 64, 0))).unwrap();
        let gen = stream.world_gen();
        let deadline = Instant::now() + std::time::Duration::from_secs(300);
        let mut sim = crate::server::ServerSim::new(gen.clone(), stream.states.clone(), "minecraft:the_nether");
        let at = match crate::locate::nearest_structure(&gen, &["minecraft:bastion_remnant".to_owned()], [0, 64, 0], crate::locate::STRUCTURE_RADIUS, deadline) {
            crate::locate::Search::Found(at, _) => at,
            other => panic!("no bastion near the origin of seed 77: {other:?}"),
        };
        let (cx, cz) = (at[0] >> 4, at[2] >> 4);
        let mut piglins = Vec::new();
        for x in cx - 3..=cx + 3 {
            for z in cz - 3..=cz + 3 {
                let chunk = stream.server.load_now(minecraftoss_core::ChunkPos::new(x, z));
                piglins.extend(chunk.generation.entities.iter().flat_map(minecraftoss_world::natural_spawner::tick::census_of).filter(|mob| matches!(mob.kind.as_str(), "minecraft:piglin" | "minecraft:piglin_brute")));
                sim.load_chunk(&chunk);
            }
        }
        assert!(!piglins.is_empty(), "the bastion holds piglins");
        let alive = sim.tracked_mobs([f64::from(at[0]), f64::from(at[1]), f64::from(at[2])], 112.0);
        assert_eq!(alive.piglins().len(), piglins.len(), "every bastion piglin and brute comes alive");
        assert!(alive.piglins().iter().all(|e| e.piglin.persistence_required));
        assert!(alive.piglins().iter().any(|e| e.piglin.kind == minecraftoss_entities::piglin::PiglinKind::Brute));
        for brute in alive.piglins().iter().filter(|e| e.piglin.is_brute()) {
            assert_eq!(brute.piglin.main_hand.as_ref().map(|s| s.id.as_str()), Some("minecraft:golden_axe"));
            let home = brute.brain.memories.home.get().expect("a bastion brute has a home");
            assert_eq!(home.dimension, "minecraft:the_nether");
            let (x, y, z) = home.pos;
            assert!(brute.position().distance(glam::DVec3::new(f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5)) < 2.0);
        }
        for _ in 0..20 {
            sim.tick();
        }
    }

    #[test]
    fn bastion_hoglins_come_alive_and_stay_persistent() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let dir = crate::test_dir::tempdir().unwrap();
        let level = crate::level_storage::LevelStorage::open(dir.path()).unwrap();
        let mut stream = TerrainStream::for_dimension_at(registries.clone(), 77, 2, Dimension::Nether, Some(level.root()), Some((0, 64, 0))).unwrap();
        let gen = stream.world_gen();
        let deadline = Instant::now() + std::time::Duration::from_secs(300);
        let mut sim = crate::server::ServerSim::new(gen.clone(), stream.states.clone(), "minecraft:the_nether");
        let mut seen = Vec::new();
        for (ox, oz) in [(0, 0), (1500, 0), (0, 1500), (-1500, 0), (0, -1500), (1500, 1500), (-1500, -1500), (1500, -1500), (-1500, 1500)] {
            let at = match crate::locate::nearest_structure(&gen, &["minecraft:bastion_remnant".to_owned()], [ox, 64, oz], crate::locate::STRUCTURE_RADIUS, deadline) {
                crate::locate::Search::Found(at, _) => at,
                _ => continue,
            };
            if seen.contains(&at) {
                continue;
            }
            seen.push(at);
            let (cx, cz) = (at[0] >> 4, at[2] >> 4);
            let mut hoglins = Vec::new();
            for x in cx - 3..=cx + 3 {
                for z in cz - 3..=cz + 3 {
                    let chunk = stream.server.load_now(minecraftoss_core::ChunkPos::new(x, z));
                    hoglins.extend(chunk.generation.entities.iter().flat_map(minecraftoss_world::natural_spawner::tick::census_of).filter(|mob| mob.kind == "minecraft:hoglin"));
                    sim.load_chunk(&chunk);
                }
            }
            if hoglins.is_empty() {
                continue;
            }
            assert!(hoglins.iter().all(|mob| mob.persistent), "{hoglins:?}");
            let alive = sim.tracked_mobs([f64::from(at[0]), f64::from(at[1]), f64::from(at[2])], 96.0);
            assert_eq!(alive.hoglins().len(), hoglins.len(), "every stable hoglin comes alive");
            assert!(alive.hoglins().iter().all(|e| e.hoglin.persistence_required && e.hoglin.kind == minecraftoss_entities::hoglin::HoglinKind::Hoglin));
            return;
        }
        panic!("no hoglin stables among the bastions of seed 77 at {seen:?}");
    }

    #[test]
    fn a_level_shown_again_sends_its_chunks_and_light_again() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let mut stream = TerrainStream::for_dimension_at(registries, 77, 2, Dimension::End, None, None).unwrap();
        let mut scene = HandcraftedScene::streamed(stream.states.clone());
        let started = Instant::now();
        let mut sent = 0;
        while scene.generated_chunk((6, 0)).is_none() || !stream.server.is_idle() {
            assert!(started.elapsed().as_secs() < 600, "no chunks around the End platform");
            sent += stream.server_tick((100, 49, 0), &mut scene).0.len();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let generated: HashSet<ChunkPos> = scene.generated_chunks().collect();
        assert_eq!(sent, generated.len());
        let light = Arc::new(SkyLight::streamed());
        stream.lights.insert((6, 0), light.clone());
        stream.light_changes.clear();
        let registries = stream.states.registries().clone();
        let blocks = &registries.blocks;
        let solid = generated
            .iter()
            .flat_map(|&(cx, cz)| (0..256).flat_map(move |y| (0..16).map(move |x| (cx * 16 + x, y, cz * 16))))
            .find(|&(x, y, z)| !blocks.is_air(scene.generated_chunk((x >> 4, z >> 4)).unwrap().block((x & 15) as usize, y, 0)))
            .expect("some end stone near the platform");
        scene.set(solid, None);
        scene.set((100, 200, 0), Some(Block::new("minecraft:obsidian")));
        let again = stream.reshow(&scene);
        let shown: HashSet<ChunkPos> = again.iter().map(|chunk| (chunk.pos.x, chunk.pos.z)).collect();
        assert_eq!(shown, generated, "every chunk the scene holds is sent again for collision");
        let sent_again = |pos: (i32, i32)| again.iter().find(|chunk| (chunk.pos.x, chunk.pos.z) == pos).unwrap();
        let broken = sent_again((solid.0 >> 4, solid.2 >> 4)).block((solid.0 & 15) as usize, solid.1, 0);
        assert!(blocks.is_air(broken), "a block broken before the trip stays broken for collision");
        let placed = sent_again((6, 0)).block(4, 200, 0);
        assert_eq!(blocks.block(blocks.block_of(placed)).name.as_str(), "minecraft:obsidian", "a block placed before the trip stays solid");
        assert!(!blocks.is_air(scene.generated_chunk((solid.0 >> 4, solid.2 >> 4)).unwrap().block((solid.0 & 15) as usize, solid.1, 0)), "the scene's own chunk is left alone");
        assert_eq!(stream.light_changes.len(), 1);
        assert!(stream.light_changes[0].0 == (6, 0) && stream.light_changes[0].1.as_ref().is_some_and(|l| Arc::ptr_eq(l, &light)));
        assert!(stream.server_tick((100, 49, 0), &mut scene).0.is_empty(), "the server does not send them twice");
    }
}
