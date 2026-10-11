use crate::{
    fluid::{FluidCell, FluidKind},
    lighting::SkyLight,
    model::{resolve_block_variants, ResolvedModel},
    pack::{PackStack, ResourceId},
    scene::{BiomeSample, Block, BlockPos, ChunkPos, Scene},
    texture_mips,
};
use anyhow::{anyhow, Context, Result};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use image::{imageops::FilterType, RgbaImage};
use minecraftoss_core::pos::Direction;
use minecraftoss_player::items::WorldItems;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub sky_light: f32,
    pub block_light: f32,
}
impl Vertex {
}
/// A terrain section vertex as the GPU stores it: 28 bytes, the size of
/// vanilla's block vertex. Colours are bytes as `ARGB.colorFromFloat`
/// makes them (`floor(value * 255)`), and light keeps vanilla's lightmap
/// coordinate precision (a level in sixteenths: smooth lighting averages
/// four levels, so quarter levels are exact).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SectionVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [u8; 4],
    /// Sky then block light, times 16.
    pub light: [u8; 2],
    pub pad: [u8; 2],
}

impl SectionVertex {

    pub fn from_vertex(v: &Vertex) -> Self {
        // `ARGB.as8BitChannel`.
        let channel = |value: f32| (value * 255.0).floor().clamp(0.0, 255.0) as u8;
        let light = |level: f32| (level * 16.0).round().clamp(0.0, 255.0) as u8;
        Self {
            position: v.position,
            uv: v.uv,
            color: v.color.map(channel),
            light: [light(v.sky_light), light(v.block_light)],
            pad: [0; 2],
        }
    }
}

/// A terrain section's mesh in the GPU vertex format.
#[derive(Default)]
pub struct SectionMesh {
    pub vertices: Vec<SectionVertex>,
    pub indices: Vec<u32>,
    pub transparent_start: Option<u32>,
}

impl From<ChunkMesh> for SectionMesh {
    fn from(mesh: ChunkMesh) -> Self {
        Self {
            vertices: mesh.vertices.iter().map(SectionVertex::from_vertex).collect(),
            indices: mesh.indices,
            transparent_start: mesh.transparent_start,
        }
    }
}

#[derive(Default)]
pub struct ChunkMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub transparent_start: Option<u32>,
    pub faces: usize,
    pub glint: Vec<std::ops::Range<u32>>,
    pub armor_glint: Vec<std::ops::Range<u32>>,
}

pub const GLINT_VERTEX: u8 = 255;
pub const ARMOR_GLINT_VERTEX: u8 = 128;

pub fn section_vertices(mesh: &ChunkMesh) -> Vec<SectionVertex> {
    let mut vertices: Vec<SectionVertex> = mesh.vertices.iter().map(SectionVertex::from_vertex).collect();
    let len = vertices.len();
    for (ranges, flag) in [(&mesh.armor_glint, ARMOR_GLINT_VERTEX), (&mesh.glint, GLINT_VERTEX)] {
        for range in ranges {
            let end = (range.end as usize).min(len);
            let start = (range.start as usize).min(end);
            for vertex in &mut vertices[start..end] {
                vertex.pad[0] = flag;
            }
        }
    }
    vertices
}
pub const WHITE_PATH: &str = "iw4l/white";
pub const WHITE_TEXTURE: &str = "minecraft:iw4l/white";
pub struct Atlas {
    pub pixels: RgbaImage,
    pub mipmaps: Vec<RgbaImage>,
    slots: HashMap<ResourceId, [f32; 4]>,
    pub missing: Vec<ResourceId>,
    pub animated: Vec<ResourceId>,
    pub animated_tiles: Vec<AnimatedTile>,
    pub translucent: HashMap<ResourceId, TranslucentPixels>,
}
pub struct TranslucentPixels {
    width: u32,
    height: u32,
    pixels: Vec<bool>,
}
impl TranslucentPixels {
    fn of(image: &RgbaImage, animation: Option<&serde_json::Value>) -> Option<Self> {
        let (width, height) = if animation.is_some() {
            let size = image.width().min(image.height());
            (size, size)
        } else {
            image.dimensions()
        };
        if width == 0 || height == 0 {
            return None;
        }
        let columns = (image.width() / width).max(1);
        let count = columns * (image.height() / height).max(1);
        let frames: Vec<u32> = animation
            .and_then(|animation| animation.get("frames"))
            .and_then(|frames| frames.as_array())
            .map(|frames| {
                frames
                    .iter()
                    .filter_map(|frame| frame.as_u64().or_else(|| frame.get("index").and_then(|index| index.as_u64())))
                    .filter_map(|index| u32::try_from(index).ok().filter(|&index| index < count))
                    .collect()
            })
            .unwrap_or_else(|| (0..count).collect());
        let mut pixels = vec![false; (width * height) as usize];
        for frame in frames {
            let (left, top) = ((frame % columns) * width, (frame / columns) * height);
            for y in 0..height {
                for x in 0..width {
                    let alpha = image.get_pixel(left + x, top + y)[3];
                    if alpha != 0 && alpha != 255 {
                        pixels[(y * width + x) as usize] = true;
                    }
                }
            }
        }
        pixels.contains(&true).then_some(Self { width, height, pixels })
    }

    fn within(&self, [a, b, c, d]: [f32; 4]) -> bool {
        let (width, height) = (self.width as f32, self.height as f32);
        let left = ((a.min(c) * width).floor().max(0.0) as u32).min(self.width);
        let top = ((b.min(d) * height).floor().max(0.0) as u32).min(self.height);
        let right = ((a.max(c) * width).ceil().max(0.0) as u32).min(self.width);
        let bottom = ((b.max(d) * height).ceil().max(0.0) as u32).min(self.height);
        (top..bottom).any(|y| (left..right).any(|x| self.pixels[(y * self.width + x) as usize]))
    }
}
pub struct AnimatedTile {
    pub origin: (u32, u32),
    pub frames: Vec<Vec<RgbaImage>>,
    pub sequence: Vec<(usize, u32)>,
    pub interpolate: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationFrameState {
    pub current: usize,
    pub next: usize,
    pub progress_millis: u16,
}
impl AnimatedTile {
    pub fn state_at(&self, tick: u64) -> AnimationFrameState {
        let total: u64 = self
            .sequence
            .iter()
            .map(|(_, duration)| *duration as u64)
            .sum();
        let mut remaining = tick % total;
        for (position, &(frame, duration)) in self.sequence.iter().enumerate() {
            if remaining < duration as u64 {
                let next = self.sequence[(position + 1) % self.sequence.len()].0;
                let progress_millis = ((remaining as f32 / duration as f32) * 1000.0_f32) as u16;
                return AnimationFrameState {
                    current: frame,
                    next,
                    progress_millis,
                };
            }
            remaining -= duration as u64;
        }
        unreachable!("valid animation sequence has a positive duration")
    }
    pub fn frame_at(&self, tick: u64) -> usize {
        self.state_at(tick).current
    }
    pub fn image_at(
        &self,
        state: AnimationFrameState,
        level: usize,
    ) -> std::borrow::Cow<'_, RgbaImage> {
        let current = &self.frames[state.current][level];
        if !self.interpolate || state.progress_millis == 0 || state.current == state.next {
            return std::borrow::Cow::Borrowed(current);
        }
        let next = &self.frames[state.next][level];
        let progress = state.progress_millis as f32 / 1000.0;
        let mut image = RgbaImage::new(current.width(), current.height());
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let a = current.get_pixel(x, y);
            let b = next.get_pixel(x, y);
            for channel in 0..4 {
                let first = a[channel] as f32 / 255.0;
                let second = b[channel] as f32 / 255.0;
                pixel[channel] = ((first * (1.0 - progress) + second * progress) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
        std::borrow::Cow::Owned(image)
    }
}
pub struct AtlasPatch {
    pub origin: (u32, u32),
    pub levels: Vec<RgbaImage>,
}
#[derive(Default)]
pub struct AtlasAnimation {
    ticks: u64,
    shown: Vec<AnimationFrameState>,
}
impl AtlasAnimation {
    pub fn new(atlas: &Atlas) -> Self {
        Self {
            ticks: 0,
            shown: atlas
                .animated_tiles
                .iter()
                .map(|tile| tile.state_at(0))
                .collect(),
        }
    }
    pub fn ticks(&self) -> u64 {
        self.ticks
    }
    pub fn tick(&mut self, atlas: &Atlas) -> Vec<AtlasPatch> {
        self.ticks += 1;
        let ticks = self.ticks;
        if self.shown.len() != atlas.animated_tiles.len() {
            self.shown = atlas
                .animated_tiles
                .iter()
                .map(|tile| tile.state_at(0))
                .collect();
        }
        atlas
            .animated_tiles
            .iter()
            .zip(self.shown.iter_mut())
            .filter_map(|(tile, shown)| {
                let state = tile.state_at(ticks);
                let unchanged = if tile.interpolate {
                    *shown == state
                } else {
                    shown.current == state.current
                };
                *shown = state;
                if unchanged {
                    return None;
                }
                Some(AtlasPatch {
                    origin: tile.origin,
                    levels: (0..tile.frames[state.current].len())
                        .map(|level| tile.image_at(state, level).into_owned())
                        .collect(),
                })
            })
            .collect()
    }
}
impl Atlas {
    pub fn region(&self, id: &ResourceId) -> [f32; 4] {
        self.slots
            .get(id)
            .copied()
            .or_else(|| {
                self.slots
                    .get(&ResourceId::parse("minecraft:missingno").unwrap())
                    .copied()
            })
            .unwrap_or([0.0, 0.0, 1.0, 1.0])
    }
    /// Entity models address individual pixels inside a skin. The regular
    /// region is inset by half a texel to protect whole block sprites from
    /// neighbouring atlas tiles; scaling that inset over a skin moves inner
    /// UV boundaries into the adjacent body-part texture.
    /// Entity skins need their exact pixel boundaries; see `region_exact`.
    pub fn entity_region(&self, id: &ResourceId) -> [f32; 4] {
        self.region_exact(id)
    }
    pub fn contains(&self, id: &ResourceId) -> bool {
        self.slots.contains_key(id)
    }
    pub fn translucent(&self, face: &crate::model::Face) -> bool {
        face.force_translucent || !entity_face(face) && self.translucent.get(&face.texture).is_some_and(|pixels| pixels.within(face.uv))
    }
    fn region_exact(&self, id: &ResourceId) -> [f32; 4] {
        let [u0, v0, u1, v1] = self.region(id);
        let half_texel = 0.5 / self.pixels.width() as f32;
        [
            u0 - half_texel,
            v0 - half_texel,
            u1 + half_texel,
            v1 + half_texel,
        ]
    }
}
pub struct Build {
    pub atlas: Arc<Atlas>,
    pub chunks: BTreeMap<ChunkPos, ChunkMesh>,
    pub sky_light: Option<Arc<SkyLight>>,
    pub scene_revision: u64,
    pub pack_generation: u64,
}
pub struct BiomeTint {
    grass: Option<RgbaImage>,
    foliage: Option<RgbaImage>,
    dry_foliage: Option<RgbaImage>,
}
impl BiomeTint {
    pub fn from_pack(packs: &PackStack) -> Result<Self> {
        let mut tint = Self {
            grass: None,
            foliage: None,
            dry_foliage: None,
        };
        for (name, output) in [("grass", &mut tint.grass), ("foliage", &mut tint.foliage), ("dry_foliage", &mut tint.dry_foliage)] {
            let id = ResourceId::parse(&format!("minecraft:colormap/{name}"))?;
            if let Some(bytes) = packs.texture(&id)? {
                let map = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?
                    .to_rgba8();
                if map.width() >= 256 && map.height() >= 256 {
                    *output = Some(map);
                }
            }
        }
        Ok(tint)
    }
    fn color(
        map: &Option<RgbaImage>,
        override_color: Option<[u8; 3]>,
        biome: BiomeSample,
        fallback: [u8; 3],
    ) -> [f32; 3] {
        let rgb = override_color
            .or_else(|| {
                map.as_ref().map(|map| {
                    // 26.3 ColorMapColorUtil: rainfall is multiplied by temperature.
                    let temp = biome.temperature.clamp(0.0, 1.0);
                    let rain = biome.downfall.clamp(0.0, 1.0) * temp;
                    let pixel =
                        map.get_pixel(((1.0 - temp) * 255.0) as u32, ((1.0 - rain) * 255.0) as u32);
                    [pixel[0], pixel[1], pixel[2]]
                })
            })
            .unwrap_or(fallback);
        rgb.map(|channel| channel as f32 / 255.0)
    }
    pub(crate) fn grass(&self, biome: BiomeSample) -> [f32; 3] {
        Self::color(&self.grass, biome.grass_color, biome, [142, 185, 113])
    }
    pub(crate) fn foliage(&self, biome: BiomeSample) -> [f32; 3] {
        Self::color(&self.foliage, biome.foliage_color, biome, [113, 167, 77])
    }
    pub(crate) fn dry_foliage(&self, biome: BiomeSample) -> [f32; 3] {
        Self::color(&self.dry_foliage, biome.dry_foliage_color, biome, [92, 60, 50])
    }
}
/// RedstoneWireBlock.COLORS, including ARGB.colorFromFloat's floor-to-byte step.
pub fn redstone_wire_color_argb(power: u8) -> u32 {
    let level = power.min(15) as f32 / 15.0;
    let red = level * 0.6 + if level > 0.0 { 0.4 } else { 0.3 };
    let green = (level * level * 0.7 - 0.5).clamp(0.0, 1.0);
    let blue = (level * level * 0.6 - 0.7).clamp(0.0, 1.0);
    let byte = |channel: f32| (channel * 255.0).floor() as u32;
    0xff00_0000 | byte(red) << 16 | byte(green) << 8 | byte(blue)
}
pub fn build<S: Scene>(scene: &S, packs: &PackStack) -> Result<Build> {
    build_internal(scene, packs, true)
}
pub fn build_minimal<S: Scene>(scene: &S, packs: &PackStack) -> Result<Build> {
    build_internal(scene, packs, false)
}
pub fn build_item_model(packs: &PackStack, item: &ResourceId) -> Result<Option<Build>> {
    let Some(model) = crate::model::resolve_item_model(packs, item)? else {
        return Ok(None);
    };
    if model.elements.is_empty() {
        return Ok(None);
    }
    let tints = packs
        .item_definition(item)?
        .map(|value| crate::interface::item_model_tints(packs, &value["model"]))
        .transpose()?
        .unwrap_or_default();
    let textures = model
        .elements
        .iter()
        .flat_map(|element| element.faces.iter())
        .map(|face| face.texture.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let atlas = make_atlas(packs, textures)?;
    let mut mesh = ChunkMesh::default();
    for element in &model.elements {
        for face in &element.faces {
            let corners = element_corners(element, &face.direction)?;
            let [u0, v0, u1, v1] = atlas.region(&face.texture);
            let uv = face.corner_uvs().map(|[s, t]| [u0 + (u1 - u0) * s, v0 + (v1 - v0) * t]);
            let tint = face
                .tint_index
                .and_then(|index| tints.get(index))
                .copied()
                .unwrap_or([255; 3]);
            let shade = match face.direction.as_str() {
                "up" => 1.0,
                "down" => 0.5,
                "north" | "south" => 0.8,
                _ => 0.6,
            };
            let start = mesh.vertices.len() as u32;
            for (position, uv) in corners.into_iter().zip(uv) {
                mesh.vertices.push(Vertex {
                    position,
                    uv,
                    color: [
                        shade * tint[0] as f32 / 255.0,
                        shade * tint[1] as f32 / 255.0,
                        shade * tint[2] as f32 / 255.0,
                        1.0,
                    ],
                    sky_light: 15.0,
                    block_light: 0.0,
                });
            }
            mesh.indices.extend_from_slice(&[
                start,
                start + 1,
                start + 2,
                start,
                start + 2,
                start + 3,
            ]);
            mesh.faces += 1;
        }
    }
    Ok(Some(Build {
        atlas: Arc::new(atlas),
        chunks: BTreeMap::from([((0, 0), mesh)]),
        sky_light: None,
        scene_revision: 0,
        pack_generation: packs.generation,
    }))
}
fn build_internal<S: Scene>(scene: &S, packs: &PackStack, preload_blocks: bool) -> Result<Build> {
    let mut models: HashMap<String, Vec<(ResolvedModel, u32)>> = HashMap::new();
    let mut textures = BTreeMap::<ResourceId, ()>::new();
    // A broken chest can remain as a dropped item after its block mesh is gone.
    textures.insert(ResourceId::parse("minecraft:entity/chest/normal")?, ());
    for id in crate::special_models::textures()? {
        textures.insert(id, ());
    }
    textures.insert(ResourceId::parse("minecraft:entity/bat/bat")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/zombie/zombie")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/player/wide/steve")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/skeleton/skeleton")?, ());
    for skin in ["stray", "stray_overlay", "bogged", "bogged_overlay", "parched", "wither_skeleton"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/skeleton/{skin}"))?, ());
    }
    textures.insert(ResourceId::parse("minecraft:entity/creeper/creeper")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/experience/experience_orb")?, ());
    textures.insert(ResourceId::parse(crate::trident::TEXTURE)?, ());
    textures.insert(ResourceId::parse("minecraft:entity/spider/spider")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/slime/slime")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/slime/magmacube")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/ghast/ghast")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/ghast/ghast_shooting")?, ());
    textures.insert(ResourceId::parse(crate::ghast_render::FIRE_CHARGE)?, ());
    textures.insert(ResourceId::parse("minecraft:entity/blaze/blaze")?, ());
    for texture in crate::strider_render::TEXTURES {
        textures.insert(ResourceId::parse(texture)?, ());
    }
    for texture in crate::hoglin_render::TEXTURES {
        textures.insert(ResourceId::parse(texture)?, ());
    }
    textures.insert(ResourceId::parse(crate::end_crystal_render::TEXTURE)?, ());
    textures.insert(ResourceId::parse(crate::end_crystal_render::BEAM_TEXTURE)?, ());
    textures.insert(ResourceId::parse(crate::dragon_render::TEXTURE)?, ());
    textures.insert(ResourceId::parse(crate::dragon_render::EYES)?, ());
    textures.insert(ResourceId::parse(crate::dragon_fireball::TEXTURE)?, ());
    textures.insert(ResourceId::parse(WHITE_TEXTURE)?, ());
    textures.insert(ResourceId::parse("minecraft:entity/enderman/enderman")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/enderman/enderman_eyes")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/witch/witch")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/iron_golem/iron_golem")?, ());
    for cracks in ["low", "medium", "high"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/iron_golem/iron_golem_crackiness_{cracks}"))?, ());
    }
    // Wolves: each variant's wild, angry and tame looks, adult and baby,
    // and the collars.
    for variant in minecraftoss_entities::wolf::VARIANTS {
        let name = if variant == "pale" { "wolf".to_owned() } else { format!("wolf_{variant}") };
        for state in ["", "_angry", "_tame"] {
            for age in ["", "_baby"] {
                textures.insert(ResourceId::parse(&format!("minecraft:entity/wolf/{name}{state}{age}"))?, ());
            }
        }
    }
    textures.insert(ResourceId::parse("minecraft:entity/wolf/wolf_collar")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/wolf/wolf_collar_baby")?, ());
    // Thrown splash potions: the bottle over its tinted contents.
    textures.insert(ResourceId::parse("minecraft:item/splash_potion")?, ());
    textures.insert(ResourceId::parse("minecraft:item/potion_overlay")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/zombie/husk")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/zombie/husk_baby")?, ());
    for skin in ["zombified_piglin", "zombified_piglin_baby"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/piglin/{skin}"))?, ());
    }
    textures.insert(ResourceId::parse("minecraft:entity/zombie_villager/zombie_villager")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/zombie_villager/zombie_villager_baby")?, ());
    for kind in ["desert", "jungle", "plains", "savanna", "snow", "swamp", "taiga"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/zombie_villager/type/{kind}"))?, ());
        textures.insert(ResourceId::parse(&format!("minecraft:entity/zombie_villager/baby/{kind}"))?, ());
    }
    for profession in ["armorer", "butcher", "cartographer", "cleric", "farmer", "fisherman", "fletcher", "leatherworker", "librarian", "mason", "nitwit", "shepherd", "toolsmith", "weaponsmith"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/zombie_villager/profession/{profession}"))?, ());
    }
    textures.insert(ResourceId::parse("minecraft:entity/spider/spider_eyes")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/zombie/drowned")?, ());
    textures.insert(
        ResourceId::parse("minecraft:entity/zombie/drowned_baby")?,
        (),
    );
    for path in ["villager", "villager_baby"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/villager/{path}"))?, ());
    }
    for kind in ["desert", "jungle", "plains", "savanna", "snow", "swamp", "taiga"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/villager/type/{kind}"))?, ());
        textures.insert(ResourceId::parse(&format!("minecraft:entity/villager/baby/{kind}"))?, ());
    }
    for profession in ["armorer", "butcher", "cartographer", "cleric", "farmer", "fisherman", "fletcher", "leatherworker", "librarian", "mason", "nitwit", "shepherd", "toolsmith", "weaponsmith"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/villager/profession/{profession}"))?, ());
    }
    for level in ["stone", "iron", "gold", "emerald", "diamond"] {
        textures.insert(ResourceId::parse(&format!("minecraft:entity/villager/profession_level/{level}"))?, ());
    }
    textures.insert(
        ResourceId::parse("minecraft:entity/zombie/zombie_baby")?,
        (),
    );
    for kind in ["temperate", "warm", "cold"] {
        for suffix in ["", "_baby"] {
            textures.insert(
                ResourceId::parse(&format!("minecraft:entity/cow/cow_{kind}{suffix}"))?,
                (),
            );
            textures.insert(
                ResourceId::parse(&format!("minecraft:entity/pig/pig_{kind}{suffix}"))?,
                (),
            );
            textures.insert(
                ResourceId::parse(&format!("minecraft:entity/chicken/chicken_{kind}{suffix}"))?,
                (),
            );
        }
    }
    for kind in ["red", "brown"] {
        for suffix in ["", "_baby"] {
            textures.insert(
                ResourceId::parse(&format!("minecraft:entity/cow/mooshroom_{kind}{suffix}"))?,
                (),
            );
        }
        textures.insert(
            ResourceId::parse(&format!("minecraft:block/{kind}_mushroom"))?,
            (),
        );
    }
    for name in [
        "sheep",
        "sheep_baby",
        "sheep_wool",
        "sheep_wool_baby",
        "sheep_wool_undercoat",
    ] {
        textures.insert(
            ResourceId::parse(&format!("minecraft:entity/sheep/{name}"))?,
            (),
        );
    }
    textures.insert(
        ResourceId::parse("minecraft:entity/equipment/pig_saddle/saddle")?,
        (),
    );
    for id in crate::horse_render::all_textures() {
        textures.insert(id, ());
    }
    // Humanoid armour (`EquipmentLayerRenderer`'s humanoid layers).
    for layer in ["humanoid", "humanoid_leggings"] {
        for material in crate::armor_render::MATERIALS.iter().copied().chain(["leather_overlay"]) {
            let id = ResourceId::parse(&format!("minecraft:entity/equipment/{layer}/{material}"))?;
            if packs.texture(&id)?.is_some() {
                textures.insert(id, ());
            }
        }
    }
    for chunk in scene.chunks() {
        for x in chunk.0 * 16..chunk.0 * 16 + 16 {
            for z in chunk.1 * 16..chunk.1 * 16 + 16 {
                for y in scene.vertical_range() {
                    let Some(block) = scene.block((x, y, z)) else {
                        continue;
                    };
                    let key = model_key(block);
                    if !models.contains_key(&key) {
                        let variants = resolve_block_variants(packs, block)
                            .with_context(|| format!("block {} at {x},{y},{z}", block.id.key()))?;
                        for (model, _) in &variants {
                            for element in &model.elements {
                                for face in &element.faces {
                                    textures.insert(face.texture.clone(), ());
                                }
                            }
                        }
                        models.insert(key, variants);
                    }
                }
            }
        }
    }
    // Reserve block and item sprites up front. Neither placing a new block
    // nor tossing an item should resize/reupload the atlas during gameplay.
    if preload_blocks {
        for prefix in ["textures/block/", "textures/item/"] {
            for path in packs.list("minecraft", prefix)? {
                if let Some(name) = path
                    .strip_prefix("assets/minecraft/textures/")
                    .and_then(|name| name.strip_suffix(".png"))
                {
                    textures.insert(ResourceId::parse(&format!("minecraft:{name}"))?, ());
                }
            }
        }
        for index in 0..12 {
            let id = ResourceId::parse(&format!("minecraft:particle/leaf_{index}"))?;
            textures.insert(id, ());
        }
        for index in 0..4 {
            textures.insert(
                ResourceId::parse(&format!("minecraft:particle/splash_{index}"))?,
                (),
            );
        }
        for index in 0..8 {
            textures.insert(
                ResourceId::parse(&format!("minecraft:particle/generic_{index}"))?,
                (),
            );
        }
        textures.insert(ResourceId::parse("minecraft:particle/note")?, ());
        textures.insert(ResourceId::parse("minecraft:particle/glow")?, ());
        for id in crate::spell_particles::textures() {
            textures.insert(id, ());
        }
        for id in crate::ambient_particles::textures() {
            textures.insert(id, ());
        }
        textures.insert(ResourceId::parse("minecraft:misc/shadow")?, ());
    }
    let atlas = make_atlas(packs, textures.into_keys().collect())?;
    let tint = BiomeTint::from_pack(packs)?;
    let light = SkyLight::build(scene);
    let mut chunks = BTreeMap::new();
    for chunk in scene.chunks() {
        chunks.insert(
            chunk,
            mesh_chunk(scene, chunk, &models, &atlas, &tint, &light)?,
        );
    }
    Ok(Build {
        atlas: Arc::new(atlas),
        chunks,
        sky_light: Some(Arc::new(light)),
        scene_revision: scene.revision(),
        pack_generation: packs.generation,
    })
}
/// Rebuild the chunks whose face visibility or 15-block skylight neighborhood
/// can change after a single block edit, using the existing texture atlas.
/// A newly introduced texture requests an explicit atlas rebuild instead of
/// silently drawing a missing-texture face.
pub fn rebuild_near<S: Scene>(
    scene: &S,
    packs: &PackStack,
    build: &Build,
    pos: BlockPos,
) -> Result<BTreeMap<ChunkPos, ChunkMesh>> {
    rebuild_near_many_with_previous_light(
        scene,
        packs,
        &build.atlas,
        &build.chunks.keys().copied().collect::<Vec<_>>(),
        &[pos],
        build.sky_light.as_deref(),
    )
    .map(|(chunks, _)| chunks)
}
pub fn rebuild_near_with_atlas<S: Scene>(
    scene: &S,
    packs: &PackStack,
    atlas: &Atlas,
    old_chunks: &[ChunkPos],
    pos: BlockPos,
) -> Result<(BTreeMap<ChunkPos, ChunkMesh>, SkyLight)> {
    rebuild_near_many_with_atlas(scene, packs, atlas, old_chunks, &[pos])
}
pub fn rebuild_near_many_with_atlas<S: Scene>(
    scene: &S,
    packs: &PackStack,
    atlas: &Atlas,
    old_chunks: &[ChunkPos],
    positions: &[BlockPos],
) -> Result<(BTreeMap<ChunkPos, ChunkMesh>, SkyLight)> {
    rebuild_near_many_with_previous_light(scene, packs, atlas, old_chunks, positions, None)
}

pub fn rebuild_near_many_with_previous_light<S: Scene>(
    scene: &S,
    packs: &PackStack,
    atlas: &Atlas,
    old_chunks: &[ChunkPos],
    positions: &[BlockPos],
    previous_light: Option<&SkyLight>,
) -> Result<(BTreeMap<ChunkPos, ChunkMesh>, SkyLight)> {
    let mut chunks = scene.chunks();
    chunks.extend(old_chunks.iter().copied());
    chunks.sort_unstable();
    chunks.dedup();
    let affected = chunks
        .into_iter()
        .filter(|&(cx, cz)| {
            positions.iter().any(|pos| {
                let min_x = cx * 16;
                let min_z = cz * 16;
                min_x <= pos.0 + 15
                    && min_x + 15 >= pos.0 - 15
                    && min_z <= pos.2 + 15
                    && min_z + 15 >= pos.2 - 15
            })
        })
        .collect::<Vec<_>>();
    let mut models = HashMap::new();
    for &(cx, cz) in &affected {
        for x in cx * 16..cx * 16 + 16 {
            for z in cz * 16..cz * 16 + 16 {
                for y in scene.vertical_range() {
                    let Some(block) = scene.block((x, y, z)) else {
                        continue;
                    };
                    let key = model_key(block);
                    if models.contains_key(&key) {
                        continue;
                    }
                    let variants = resolve_block_variants(packs, block)
                        .with_context(|| format!("block {} at {x},{y},{z}", block.id.key()))?;
                    for (model, _) in &variants {
                        for element in &model.elements {
                            for face in &element.faces {
                                if !atlas.contains(&face.texture) {
                                    return Err(anyhow!("atlas missing {}", face.texture.key()));
                                }
                            }
                        }
                    }
                    models.insert(key, variants);
                }
            }
        }
    }
    let tint = BiomeTint::from_pack(packs)?;
    let light = previous_light.map_or_else(
        || SkyLight::build(scene),
        |old| old.updated(scene, positions),
    );
    let rebuilt: BTreeMap<ChunkPos, ChunkMesh> = affected
        .into_iter()
        .map(|chunk| {
            Ok((
                chunk,
                mesh_chunk(scene, chunk, &models, atlas, &tint, &light)?,
            ))
        })
        .collect::<Result<_>>()?;
    Ok((rebuilt, light))
}

/// Pack-resolved item models are cached, while entity positions, bob, and
/// rotation are rebuilt at frame rate from interpolated 20 Hz physics state.
#[derive(Default)]
pub struct ItemVisuals {
    models: HashMap<String, ItemVisual>,
    block_models: HashMap<String, Vec<(ResolvedModel, u32)>>,
    tint: Option<BiomeTint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MovingBlock {
    pub feet: glam::DVec3,
    pub block: Block,
    pub seed: BlockPos,
}

struct MovingBlockScene<'a, S: Scene> {
    level: &'a S,
    block: &'a Block,
    at: BlockPos,
}

impl<S: Scene> Scene for MovingBlockScene<'_, S> {
    fn block(&self, pos: BlockPos) -> Option<&Block> {
        (pos == self.at).then_some(self.block)
    }
    fn chunks(&self) -> Vec<ChunkPos> {
        Vec::new()
    }
    fn vertical_range(&self) -> std::ops::Range<i32> {
        self.level.vertical_range()
    }
    fn revision(&self) -> u64 {
        0
    }
    fn biome_at(&self, pos: BlockPos) -> BiomeSample {
        self.level.biome_at(pos)
    }
}

const MOVING_BLOCK_HEIGHT: f64 = 0.98;
struct ItemVisual {
    model: ResolvedModel,
    hand: Option<ResolvedModel>,
    ground: Mat4,
    /// The `thirdperson_righthand` display transform.
    right_hand: Mat4,
    left_hand: Mat4,
    fixed: Mat4,
    min_y: f32,
    tints: Vec<[f32; 3]>,
    flat: bool,
}
fn face_normal(direction: &str) -> Result<Vec3> {
    let (x, y, z) = self::direction(direction)?;
    Ok(Vec3::new(x as f32, y as f32, z as f32))
}
fn level_item_shade(normal: Vec3) -> f32 {
    // entity.vsh uses Lighting.Entry.LEVEL and minecraft_mix_light for
    // dropped item quads, in addition to the sampled world lightmap.
    crate::cow_render::entity_shade(normal)
}
impl ItemVisuals {
    pub fn clear(&mut self) {
        self.models.clear();
        self.block_models.clear();
        self.tint = None;
    }
    fn model<'a>(&'a mut self, id: &str, packs: &PackStack) -> Result<Option<&'a ItemVisual>> {
        if !self.models.contains_key(id) {
            let (key, dial) = crate::dials::split(id);
            let resource = ResourceId::parse(key)?;
            let resolved = match dial {
                Some(leaf) => Some(crate::model::resolve_item_leaf(packs, &resource, leaf)?),
                None => crate::model::resolve_item_model(packs, &resource)?,
            };
            let Some(model) = resolved else {
                return Ok(None);
            };
            if model.elements.is_empty() {
                return Ok(None);
            }
            let definition = packs.item_definition(&resource)?;
            let hand = match definition.as_ref().map(|value| &value["model"]) {
                Some(value) if crate::special_models::uses_item_geometry_in(value, Some("thirdperson_righthand")) && !crate::special_models::uses_item_geometry(value) => {
                    crate::model::resolve_item_model_in(packs, &resource, Some("thirdperson_righthand"))?.filter(|hand| !hand.elements.is_empty())
                }
                _ => None,
            };
            let ground = crate::model::item_display_transform(packs, &resource, "ground")?;
            let right_hand = crate::model::item_display_transform(packs, &resource, "thirdperson_righthand")?;
            let left_hand = crate::model::item_third_person_transform(packs, &resource, true)?;
            let fixed = crate::model::item_display_transform(packs, &resource, "fixed")?;
            let tints = definition
                .as_ref()
                .map(|value| crate::interface::item_model_tints(packs, &value["model"]))
                .transpose()?
                .unwrap_or_default()
                .into_iter()
                .map(|rgb| rgb.map(|component| component as f32 / 255.0))
                .collect();
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            for element in &model.elements {
                for face in &element.faces {
                    for corner in element_corners(element, &face.direction)? {
                        let point = ground.transform_point3(Vec3::from_array(corner));
                        min = min.min(point);
                        max = max.max(point);
                    }
                }
            }
            self.models.insert(
                id.into(),
                ItemVisual {
                    model,
                    hand,
                    ground,
                    right_hand,
                    left_hand,
                    fixed,
                    min_y: min.y,
                    tints,
                    flat: max.z - min.z <= 0.0625,
                },
            );
        }
        Ok(self.models.get(id))
    }
    pub fn mesh(
        &mut self,
        items: &WorldItems,
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
        partial_tick: f32,
    ) -> Result<ChunkMesh> {
        let mut mesh = ChunkMesh::default();
        for (entity, pickup_position) in items.entities.iter().map(|entity| (entity, None)).chain(
            items
                .pickup_effects
                .iter()
                .map(|effect| (&effect.item, Some(effect.position(partial_tick)))),
        ) {
            let Some(visual) = self.model(&entity.stack.id, packs)? else {
                continue;
            };
            let partial = partial_tick.clamp(0.0, 1.0);
            let pos = pickup_position
                .unwrap_or_else(|| {
                    entity
                        .previous_position
                        .lerp(entity.position, partial as f64)
                })
                .as_vec3();
            let light_pos = (
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            );
            let sky = sky_light.get(light_pos) as f32;
            let age = entity.age.saturating_sub(1) as f32 + partial;
            let bob = (age / 10.0 + entity.bob_offset).sin() * 0.1 + 0.1;
            let spin = age / 20.0 + entity.bob_offset;
            let copies = match entity.stack.count {
                0 | 1 => 1,
                2..=16 => 2,
                17..=32 => 3,
                33..=48 => 4,
                _ => 5,
            };
            let first = mesh.vertices.len() as u32;
            for copy in 0..copies {
                let offset = if copy == 0 {
                    Vec3::ZERO
                } else {
                    let seed = (entity.bob_offset.to_bits() as u64)
                        .wrapping_add((copy as u64).wrapping_mul(0x9e3779b97f4a7c15));
                    let random = |shift: u32| {
                        ((seed.rotate_left(shift) as u32) as f32 / u32::MAX as f32 * 2.0 - 1.0)
                            * 0.15
                    };
                    Vec3::new(
                        random(7),
                        random(19),
                        if visual.flat { 0.0 } else { random(31) },
                    )
                };
                let pose = Mat4::from_translation(pos + Vec3::Y * (bob - visual.min_y + 0.0625))
                    * Mat4::from_rotation_y(spin)
                    * Mat4::from_translation(offset)
                    * visual.ground;
                let normal_pose = pose.inverse().transpose();
                for element in &visual.model.elements {
                    for face in &element.faces {
                        if !atlas.contains(&face.texture) {
                            continue;
                        }
                        let corners = element_corners(element, &face.direction)?;
                        let entity = entity_face(face);
                        let [u0, v0, u1, v1] = if entity { atlas.entity_region(&face.texture) } else { atlas.region(&face.texture) };
                        let uv = face.corner_uvs().map(|[s, t]| [u0 + (u1 - u0) * s, v0 + (v1 - v0) * t]);
                        let tint = face
                            .tint_index
                            .and_then(|index| visual.tints.get(index))
                            .copied()
                            .unwrap_or([1.0; 3]);
                        let rotated_normal = if entity {
                            quad_normal(&corners).unwrap_or(Vec3::Y)
                        } else {
                            face_normal(quad_direction(element, &face.direction, &corners)?)?
                        };
                        let shade = level_item_shade(
                            normal_pose.transform_vector3(rotated_normal).normalize(),
                        );
                        let alpha = entity_face_alpha(face);
                        let start = mesh.vertices.len() as u32;
                        for (corner, uv) in corners.into_iter().zip(uv) {
                            mesh.vertices.push(Vertex {
                                position: pose
                                    .transform_point3(Vec3::from_array(corner))
                                    .to_array(),
                                uv,
                                color: [tint[0] * shade, tint[1] * shade, tint[2] * shade, alpha],
                                sky_light: sky,
                                block_light: sky_light.get_block(light_pos) as f32,
                            });
                        }
                        mesh.indices.extend_from_slice(&[
                            start,
                            start + 1,
                            start + 2,
                            start,
                            start + 2,
                            start + 3,
                        ]);
                        mesh.faces += 1;
                    }
                }
            }
            if crate::glint::has_glint(&entity.stack) {
                mesh.glint.push(first..mesh.vertices.len() as u32);
            }
        }
        Ok(mesh)
    }
}

impl ItemVisuals {
    /// `TntRenderer`: primed TNT as the TNT block, swelling over the last
    /// ten ticks of its fuse and flashing white on alternate five-tick
    /// windows. Each entry is an interpolated position and the remaining
    /// fuse (`fuse - partialTick + 1`). The white overlay mixes 75% white
    /// into the texture in vanilla; here the vertex colour brightens it.
    pub fn append_primed_tnt(
        &mut self,
        mesh: &mut ChunkMesh,
        tnt: &[(Vec3, f32)],
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
    ) -> Result<()> {
        for &(pos, fuse) in tnt {
            let mut scale = 1.0;
            if fuse < 10.0 {
                let g = (1.0 - fuse / 10.0).clamp(0.0, 1.0);
                let g = g * g;
                scale = 1.0 + g * g * 0.3;
            }
            let white = (fuse as i32) / 5 % 2 == 0;
            let pose = Mat4::from_translation(pos + Vec3::Y * 0.5)
                * Mat4::from_scale(Vec3::splat(scale))
                * Mat4::from_rotation_y((-90.0f32).to_radians())
                * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.5))
                * Mat4::from_rotation_y(90.0f32.to_radians());
            self.append_block_model(mesh, "minecraft:tnt", pos, pose, if white { 4.0 } else { 1.0 }, packs, atlas, sky_light)?;
        }
        Ok(())
    }

    pub fn append_moving_blocks<S: Scene>(&mut self, mesh: &mut ChunkMesh, blocks: &[MovingBlock], level: &S, packs: &PackStack, atlas: &Atlas, sky_light: &SkyLight) -> Result<()> {
        if self.tint.is_none() {
            self.tint = Some(BiomeTint::from_pack(packs)?);
        }
        for moving in blocks {
            let key = model_key(&moving.block);
            if !self.block_models.contains_key(&key) {
                let variants = resolve_block_variants(packs, &moving.block).unwrap_or_else(|_| Vec::new());
                self.block_models.insert(key.clone(), variants);
            }
            let (Some(variants), Some(tint)) = (self.block_models.get(&key), self.tint.as_ref()) else {
                continue;
            };
            if variants.is_empty() {
                continue;
            }
            let chosen = variant_index(variants, moving.seed);
            let at = (moving.feet.x.floor() as i32, (moving.feet.y + MOVING_BLOCK_HEIGHT).floor() as i32, moving.feet.z.floor() as i32);
            let scene = MovingBlockScene { level, block: &moving.block, at };
            let mut own = ChunkMesh::default();
            let mut transparent = Vec::new();
            append_block(&scene, at, &moving.block, || Ok(&variants[chosen..=chosen]), |_, _| false, atlas, tint, sky_light, &mut own, &mut transparent)?;
            let offset = [moving.feet.x - 0.5 - f64::from(at.0), moving.feet.y - f64::from(at.1), moving.feet.z - 0.5 - f64::from(at.2)];
            let base = mesh.vertices.len() as u32;
            mesh.vertices.extend(own.vertices.into_iter().map(|mut vertex| {
                for (component, shift) in vertex.position.iter_mut().zip(offset) {
                    *component += shift as f32;
                }
                vertex
            }));
            mesh.indices.extend(own.indices.into_iter().chain(transparent).map(|index| index + base));
            mesh.faces += own.faces;
        }
        Ok(())
    }

    pub fn append_open_lids<S: Scene>(&mut self, mesh: &mut ChunkMesh, lids: &[crate::lids::PosedLid], level: &S, packs: &PackStack, atlas: &Atlas, sky_light: &SkyLight) -> Result<()> {
        if self.tint.is_none() {
            self.tint = Some(BiomeTint::from_pack(packs)?);
        }
        let Some(tint) = self.tint.as_ref() else {
            return Ok(());
        };
        for lid in lids {
            let Some(model) = crate::lids::lid_model(&lid.block, lid.progress)? else {
                continue;
            };
            let variants = [(model, 1)];
            let mut dry = lid.block.clone();
            dry.properties.remove("waterlogged");
            let scene = MovingBlockScene { level, block: &dry, at: lid.cell };
            let mut transparent = Vec::new();
            append_block(&scene, lid.cell, &dry, || Ok(variants.as_slice()), |_, _| false, atlas, tint, sky_light, mesh, &mut transparent)?;
            mesh.indices.extend(transparent);
        }
        Ok(())
    }

    /// Items mobs hold (`ItemInHandLayer`, `CrossedArmsItemLayer`): each
    /// under its holder's pose, then its display transform, tinted as its
    /// item definition says, lit at its point.
    pub fn append_held_items(&mut self, mesh: &mut ChunkMesh, items: &[HeldItem], packs: &PackStack, atlas: &Atlas, sky_light: &SkyLight) -> Result<()> {
        for item in items {
            let Some((display, mut tints)) = self.model(&item.id, packs)?.map(|visual| {
                (match item.display { HeldDisplay::RightHand => visual.right_hand, HeldDisplay::LeftHand => visual.left_hand, HeldDisplay::Ground => visual.ground, HeldDisplay::Fixed => visual.fixed }, visual.tints.clone())
            }) else {
                continue;
            };
            if let Some(color) = item.first_tint {
                if tints.is_empty() {
                    tints.push(color);
                } else {
                    tints[0] = color;
                }
            }
            let hand = matches!(item.display, HeldDisplay::RightHand | HeldDisplay::LeftHand);
            self.append_model(mesh, &item.id, item.light, item.pose * display, 1.0, Some(&tints), hand, packs, atlas, sky_light)?;
        }
        Ok(())
    }

    /// Block models under whole poses (an enderman's carried block), each lit
    /// at its point.
    pub fn append_posed_blocks(&mut self, mesh: &mut ChunkMesh, blocks: &[(Mat4, Vec3, String)], packs: &PackStack, atlas: &Atlas, sky_light: &SkyLight) -> Result<()> {
        if self.tint.is_none() {
            self.tint = Some(BiomeTint::from_pack(packs)?);
        }
        for (pose, light, id) in blocks {
            let tints = match (ResourceId::parse(id), self.tint.as_ref()) {
                (Ok(id), Some(source)) => {
                    let kind = TintKind::of(&Block { id, properties: BTreeMap::new() });
                    vec![kind.in_hand(0, source), kind.in_hand(1, source)]
                }
                _ => Vec::new(),
            };
            self.append_model(mesh, id, *light, *pose, 1.0, Some(&tints), false, packs, atlas, sky_light)?;
        }
        Ok(())
    }

    pub fn append_posed_items(&mut self, mesh: &mut ChunkMesh, items: &[(Mat4, Vec3, String)], packs: &PackStack, atlas: &Atlas, sky_light: &SkyLight) -> Result<()> {
        for (pose, light, id) in items {
            let Some(tints) = self.model(id, packs)?.map(|visual| visual.tints.clone()) else {
                continue;
            };
            self.append_model(mesh, id, *light, *pose, 1.0, Some(&tints), true, packs, atlas, sky_light)?;
        }
        Ok(())
    }

    /// One item model placed with a pose, lit at `pos`, untinted.
    #[allow(clippy::too_many_arguments)]
    fn append_block_model(&mut self, mesh: &mut ChunkMesh, id: &str, pos: Vec3, pose: Mat4, brightness: f32, packs: &PackStack, atlas: &Atlas, sky_light: &SkyLight) -> Result<()> {
        self.append_model(mesh, id, pos, pose, brightness, None, false, packs, atlas, sky_light)
    }

    /// One item model placed with a pose, lit at `pos`, its faces with a
    /// tint index coloured from `tints` when given.
    #[allow(clippy::too_many_arguments)]
    fn append_model(&mut self, mesh: &mut ChunkMesh, id: &str, pos: Vec3, pose: Mat4, brightness: f32, tints: Option<&[[f32; 3]]>, hand: bool, packs: &PackStack, atlas: &Atlas, sky_light: &SkyLight) -> Result<()> {
        let Some(visual) = self.model(id, packs)? else {
            return Ok(());
        };
        {
            let white = brightness > 1.0;
            let light_pos = (pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32);
            let sky = sky_light.get(light_pos) as f32;
            let block = sky_light.get_block(light_pos) as f32;
            let model = if hand { visual.hand.as_ref().unwrap_or(&visual.model) } else { &visual.model };
            for element in &model.elements {
                for face in &element.faces {
                    if !atlas.contains(&face.texture) {
                        continue;
                    }
                    let corners = element_corners(element, &face.direction)?;
                    let entity = entity_face(face);
                    let [u0, v0, u1, v1] = if entity { atlas.entity_region(&face.texture) } else { atlas.region(&face.texture) };
                    let uv = face.corner_uvs().map(|[s, t]| [u0 + (u1 - u0) * s, v0 + (v1 - v0) * t]);
                    let local_normal = if entity { quad_normal(&corners).unwrap_or(Vec3::Y) } else { face_normal(&face.direction)? };
                    let normal = pose.transform_vector3(local_normal).normalize();
                    let shade = level_item_shade(normal) * if white { 4.0 } else { 1.0 };
                    let tint = tints.zip(face.tint_index).and_then(|(tints, index)| tints.get(index)).copied().unwrap_or([1.0; 3]);
                    let alpha = entity_face_alpha(face);
                    let start = mesh.vertices.len() as u32;
                    for (corner, uv) in corners.into_iter().zip(uv) {
                        mesh.vertices.push(Vertex {
                            position: pose.transform_point3(Vec3::from_array(corner)).to_array(),
                            uv,
                            color: [tint[0] * shade, tint[1] * shade, tint[2] * shade, alpha],
                            sky_light: sky,
                            block_light: block,
                        });
                    }
                    mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
                    mesh.faces += 1;
                }
            }
        }
        Ok(())
    }
}

/// Which display transform a held item takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldDisplay {
    /// `THIRD_PERSON_RIGHT_HAND` (`ItemInHandLayer`).
    RightHand,
    LeftHand,
    /// `GROUND` (`CrossedArmsItemLayer`, a witch's potion).
    Ground,
    Fixed,
}

/// An item a mob holds: its holder's pose, where it is lit, the item, its
/// display and a colour for its first tint (a potion's).
#[derive(Clone, Debug)]
pub struct HeldItem {
    pub pose: Mat4,
    pub light: Vec3,
    pub id: String,
    pub display: HeldDisplay,
    pub first_tint: Option<[f32; 3]>,
}

/// What `EntityRenderer.extractShadow` reads of one entity.
#[derive(Clone, Copy, Debug)]
pub struct ShadowCaster {
    /// The render state's position: the interpolated feet.
    pub position: glam::DVec3,
    /// `distanceToCameraSq`: from the entity's current position.
    pub distance_squared: f64,
    /// `getShadowRadius`, before the 32-block cap.
    pub radius: f32,
    /// `getShadowStrength`.
    pub strength: f32,
}

/// Dropped items' shadows (`ItemEntityRenderer`: radius 0.15, strength
/// 0.75).
pub fn item_shadow_casters(items: &WorldItems, camera_position: glam::DVec3, partial_tick: f32) -> Vec<ShadowCaster> {
    items
        .entities
        .iter()
        .map(|entity| ShadowCaster {
            position: entity.previous_position.lerp(entity.position, f64::from(partial_tick.clamp(0.0, 1.0))),
            distance_squared: entity.position.distance_squared(camera_position),
            radius: 0.15,
            strength: 0.75,
        })
        .collect()
}

/// `EntityRenderer.extractShadow`/`extractShadowPiece` and
/// `ShadowFeatureRenderer`: each block column under the entity within its
/// radius, down to a depth its strength allows, gets the shadow texture
/// on top of a full-collision, visible block below a spot lit above 3,
/// fading with depth and darkening with that light (`Lightmap.
/// getBrightness`). `raw_brightness` is `getMaxLocalRawBrightness`; the
/// texture clamps outside the circle, so each quad is cut to the radius.
pub fn entity_shadows<S: Scene>(
    casters: &[ShadowCaster],
    scene: &S,
    atlas: &Atlas,
    raw_brightness: &dyn Fn(BlockPos) -> u8,
    ambient_light: f32,
) -> Result<ChunkMesh> {
    let mut mesh = ChunkMesh::default();
    let sprite = ResourceId::parse("minecraft:misc/shadow")?;
    if !atlas.contains(&sprite) {
        return Ok(mesh);
    }
    let [u0, v0, u1, v1] = atlas.region(&sprite);
    for caster in casters {
        let radius = caster.radius.min(32.0);
        if radius <= 0.0 {
            continue;
        }
        let pow = ((1.0 - caster.distance_squared / 256.0) * f64::from(caster.strength)) as f32;
        if pow <= 0.0 {
            continue;
        }
        let state = caster.position;
        let r = f64::from(radius);
        let depth = (pow / 0.5 - 1.0).min(radius);
        let (x0, x1) = ((state.x - r).floor() as i32, (state.x + r).floor() as i32);
        let (z0, z1) = ((state.z - r).floor() as i32, (state.z + r).floor() as i32);
        let (y0, y1) = ((state.y - f64::from(depth)).floor() as i32, state.y.floor() as i32);
        for z in z0..=z1 {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    let power_at_depth = pow - (state.y - f64::from(y)) as f32 * 0.5;
                    let below = (x, y - 1, z);
                    // RenderShape.INVISIBLE: air and barriers.
                    if scene.block(below).is_none_or(|b| matches!(b.id.path.as_str(), "air" | "cave_air" | "void_air" | "barrier" | "structure_void" | "light" | "moving_piston")) {
                        continue;
                    }
                    let brightness = raw_brightness((x, y, z));
                    if brightness <= 3 || !scene.full_collision_at(below) {
                        continue;
                    }
                    let v = f32::from(brightness) / 15.0;
                    let curved = v / (4.0 - 3.0 * v);
                    let light = ambient_light * (1.0 - curved) + curved;
                    let alpha = (power_at_depth * 0.5 * light).clamp(0.0, 1.0);
                    // The piece's corners relative to the entity, as floats.
                    let (rx, ry, rz) = ((f64::from(x) - state.x) as f32, (f64::from(y) - state.y) as f32, (f64::from(z) - state.z) as f32);
                    let (x_min, x_max) = (rx.max(-radius), (rx + 1.0).min(radius));
                    let (z_min, z_max) = (rz.max(-radius), (rz + 1.0).min(radius));
                    if x_min >= x_max || z_min >= z_max {
                        continue;
                    }
                    let origin = state.as_vec3();
                    let start = mesh.vertices.len() as u32;
                    for (px, pz) in [(x_min, z_min), (x_min, z_max), (x_max, z_max), (x_max, z_min)] {
                        let su = -px / 2.0 / radius + 0.5;
                        let sv = -pz / 2.0 / radius + 0.5;
                        mesh.vertices.push(Vertex {
                            // Lifted a thousandth off the block top so it
                            // never loses the depth test to it.
                            position: (origin + Vec3::new(px, ry + 0.001, pz)).to_array(),
                            uv: [u0 + (u1 - u0) * su, v0 + (v1 - v0) * sv],
                            color: [1.0, 1.0, 1.0, alpha],
                            sky_light: 15.0,
                            block_light: 0.0,
                        });
                    }
                    mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
                    mesh.faces += 1;
                }
            }
        }
    }
    Ok(mesh)
}
fn model_key(block: &Block) -> String {
    format!("{}{:?}", block.id.key(), block.properties)
}
/// BlockBehaviour.getSeed -> Mth.getSeed -> SingleThreadedRandomSource.nextInt.
/// WeightedVariants consumes the first draw for the selected model.
fn variant_index(variants: &[(ResolvedModel, u32)], (x, y, z): BlockPos) -> usize {
    if variants.len() == 1 {
        return 0;
    }
    let total: u32 = variants.iter().map(|(_, weight)| *weight).sum();
    if total == 0 {
        return 0;
    }
    let seed = x.wrapping_mul(3_129_871) as i64 ^ (z as i64).wrapping_mul(116_129_781) ^ y as i64;
    let seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16;
    let mut random = ((seed as u64) ^ 0x5deece66d) & ((1u64 << 48) - 1);
    let mut next = || {
        random = random.wrapping_mul(0x5deece66d).wrapping_add(11) & ((1u64 << 48) - 1);
        (random >> 17) as u32
    };
    let mut selection = if total.is_power_of_two() {
        ((total as u64 * next() as u64) >> 31) as u32
    } else {
        loop {
            let bits = next();
            let value = bits % total;
            if (bits as i32)
                .wrapping_sub(value as i32)
                .wrapping_add(total as i32 - 1)
                >= 0
            {
                break value;
            }
        }
    };
    for (index, (_, weight)) in variants.iter().enumerate() {
        if selection < *weight {
            return index;
        }
        selection -= *weight;
    }
    variants.len() - 1
}
/// A face's corners as `FaceBakery.bakeVertex` places them: the element's
/// own rotation about its origin, then the blockstate's.
pub(crate) fn element_corners(element: &crate::model::Element, face: &str) -> Result<[[f32; 3]; 4]> {
    let mut quad = corners(face, element.from, element.to)?;
    // An unrotated sheet with no thickness (leaf litter, ladders, vines) has
    // its two opposite faces in one plane, textured differently. The terrain is drawn without back-face
    // culling, so both would fight for the same pixels; each is pushed a
    // hair out along its own normal so the one facing the viewer wins.
    let (axis, sign) = match face {
        "down" => (1, -1.0),
        "up" => (1, 1.0),
        "north" => (2, -1.0),
        "south" => (2, 1.0),
        "west" => (0, -1.0),
        _ => (0, 1.0),
    };
    if element.rotation.is_none() && element.from[axis] == element.to[axis] {
        for corner in &mut quad {
            corner[axis] += sign * SHEET_FACE_OFFSET;
        }
    }
    Ok(quad.map(|corner| blockstate_turned(element.rotation.map_or(corner, |r| r.apply(corner)), element)))
}
pub(crate) fn blockstate_turned(corner: [f32; 3], element: &crate::model::Element) -> [f32; 3] {
    rotate_y(rotate_x(corner, element.rotation_x), element.rotation_y)
}

/// Blocks; well under a pixel of a block, well over depth precision.
const SHEET_FACE_OFFSET: f32 = 0.002;

pub(crate) fn entity_face(face: &crate::model::Face) -> bool {
    face.texture.path.starts_with("entity/")
}

fn entity_face_alpha(face: &crate::model::Face) -> f32 {
    if let Some(alpha) = crate::end_portal_look::face_alpha(&face.texture) {
        alpha
    } else if crate::special_models::unmipped_sheet(&face.texture) {
        crate::special_models::ENTITY_SHEET_ALPHA
    } else {
        1.0
    }
}

pub(crate) fn quad_normal(corners: &[[f32; 3]; 4]) -> Option<Vec3> {
    let [v0, v1, v2, _] = corners.map(Vec3::from_array);
    let normal = (v1 - v0).cross(v2 - v0);
    (normal.length_squared() > 1e-12).then(|| normal.normalize())
}

pub(crate) fn model_face_normal(element: &crate::model::Element, face: &crate::model::Face, corners: &[[f32; 3]; 4]) -> Option<Vec3> {
    let geometric = quad_normal(corners)?;
    let (linear, _) = crate::special_models::element_affine(element);
    let expected = linear * face_normal(&face.direction).ok()?;
    Some(if geometric.dot(expected) < 0.0 { -geometric } else { geometric })
}

/// The baked quad's direction (`FaceBakery.calculateFacing`): the face's
/// own, turned with the blockstate, or for a rotated element the direction
/// closest to its normal (`GeometryUtils.normal` of the first three
/// corners; the first of equals in `Direction` order).
fn quad_direction(element: &crate::model::Element, face: &str, corners: &[[f32; 3]; 4]) -> Result<&'static str> {
    if element.rotation.is_none() {
        let dir = turned_direction(face, element)?;
        return Ok(["down", "up", "north", "south", "west", "east"].into_iter().find(|d| *d == dir).unwrap_or("up"));
    }
    let [v0, v1, v2, _] = *corners;
    let mut n = [
        (v1[1] - v0[1]) * (v2[2] - v0[2]) - (v1[2] - v0[2]) * (v2[1] - v0[1]),
        (v1[2] - v0[2]) * (v2[0] - v0[0]) - (v1[0] - v0[0]) * (v2[2] - v0[2]),
        (v1[0] - v0[0]) * (v2[1] - v0[1]) - (v1[1] - v0[1]) * (v2[0] - v0[0]),
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if !(length > 0.0) || !length.is_finite() {
        return Ok("up");
    }
    for v in &mut n {
        *v /= length;
    }
    let mut best = "up";
    let mut closest = 0.0f32;
    for (name, [x, y, z]) in [("down", [0.0, -1.0, 0.0]), ("up", [0.0, 1.0, 0.0]), ("north", [0.0, 0.0, -1.0]), ("south", [0.0, 0.0, 1.0]), ("west", [-1.0, 0.0, 0.0]), ("east", [1.0, 0.0, 0.0])] {
        let product = n[0] * x + n[1] * y + n[2] * z;
        if product >= 0.0 && product > closest {
            closest = product;
            best = name;
        }
    }
    Ok(best)
}

/// `BlockModelLighter.prepareQuadShape`'s `faceCubic`: the quad lies flat
/// on the block's side it faces (a full collision block's quads count as
/// on their side too; not modelled).
fn face_cubic(dir: &str, corners: &[[f32; 3]; 4]) -> bool {
    let axis = match dir {
        "down" | "up" => 1,
        "north" | "south" => 2,
        _ => 0,
    };
    let (min, max) = corners.iter().fold((32.0f32, -32.0f32), |(lo, hi), c| (lo.min(c[axis]), hi.max(c[axis])));
    min == max && if matches!(dir, "down" | "north" | "west") { min < 1.0e-4 } else { max > 0.9999 }
}

fn rotate_y([x, y, z]: [f32; 3], degrees: u16) -> [f32; 3] {
    match degrees {
        90 => [1.0 - z, y, x],
        180 => [1.0 - x, y, 1.0 - z],
        270 => [z, y, 1.0 - x],
        _ => [x, y, z],
    }
}
fn rotated_direction(dir: &str, degrees: u16) -> Result<&str> {
    let i = match dir {
        "north" => 0,
        "east" => 1,
        "south" => 2,
        "west" => 3,
        "up" | "down" => return Ok(dir),
        _ => return Err(anyhow!("unsupported face direction {dir}")),
    };
    Ok(["north", "east", "south", "west"][(i + degrees as usize / 90) % 4])
}
fn rotate_x([x, y, z]: [f32; 3], degrees: u16) -> [f32; 3] {
    match degrees {
        90 => [x, z, 1.0 - y],
        180 => [x, 1.0 - y, 1.0 - z],
        270 => [x, 1.0 - z, y],
        _ => [x, y, z],
    }
}
fn rotated_direction_x(dir: &str, degrees: u16) -> Result<&'static str> {
    let i = match dir {
        "up" => 0,
        "north" => 1,
        "down" => 2,
        "south" => 3,
        "east" => return Ok("east"),
        "west" => return Ok("west"),
        _ => return Err(anyhow!("unsupported face direction {dir}")),
    };
    Ok(["up", "north", "down", "south"][(i + degrees as usize / 90) % 4])
}
fn turned_direction(dir: &str, element: &crate::model::Element) -> Result<&'static str> {
    rotated_direction(rotated_direction_x(dir, element.rotation_x)?, element.rotation_y)
}
pub fn make_atlas(packs: &PackStack, mut textures: Vec<ResourceId>) -> Result<Atlas> {
    textures.insert(0, ResourceId::parse("minecraft:missingno")?);
    let tile = 64u32;
    // Entity skins keep their own resolution, as vanilla binds each one on
    // its own: one larger than a cell spans several cells, a texel to a
    // pixel. Everything else fills one cell.
    let mut images = Vec::with_capacity(textures.len());
    let mut spans = Vec::with_capacity(textures.len());
    for id in &textures {
        let bytes = if id.path == "missingno" || id.path == WHITE_PATH { None } else { packs.texture(id)? };
        let span = match &bytes {
            Some(bytes) if id.path.starts_with("entity/") && packs.animation(id)?.is_none() => {
                let (width, height) = image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png).into_dimensions()?;
                (width.div_ceil(tile).max(1), height.div_ceil(tile).max(1))
            }
            _ => (1, 1),
        };
        images.push(bytes);
        spans.push(span);
    }
    let cells: u32 = spans.iter().map(|&(w, h)| w * h).sum();
    let mut side = (cells as f32).sqrt().ceil() as u32;
    let positions = loop {
        if let Some(positions) = place_cells(&spans, side) {
            break positions;
        }
        side += 1;
    };
    let mut pixels = RgbaImage::new(side * tile, side * tile);
    let mut mipmaps = (1..=4)
        .map(|level| RgbaImage::new(side * (tile >> level), side * (tile >> level)))
        .collect::<Vec<_>>();
    let mut slots = HashMap::new();
    let mut missing = Vec::new();
    let mut animated = Vec::new();
    let mut animated_tiles = Vec::new();
    let mut translucent = HashMap::new();
    for (i, id) in textures.into_iter().enumerate() {
        let (col, row) = positions[i];
        let (span_w, span_h) = spans[i];
        let image = images[i].take();
        let mut dark_cutout = false;
        let mut native_leaf_mips = None;
        let tile_image = if let Some(bytes) = image {
            let decoded =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
            let meta = packs.animation(&id)?;
            if let Some(pixels) = TranslucentPixels::of(&decoded, meta.as_ref().and_then(|meta| meta.get("animation"))) {
                translucent.insert(id.clone(), pixels);
            }
            let frame = if let Some(meta) = meta {
                dark_cutout = meta
                    .get("texture")
                    .and_then(|texture| texture.get("mipmap_strategy"))
                    .and_then(|value| value.as_str())
                    == Some("dark_cutout");
                if let Some(animation) = meta.get("animation") {
                    animated.push(id.clone());
                    if let Some(tile) = animated_tile(&decoded, animation, (col * tile, row * tile))
                    {
                        animated_tiles.push(tile);
                    }
                    let index = animation
                        .get("frames")
                        .and_then(|f| f.as_array())
                        .and_then(|a| a.first())
                        .and_then(|v| {
                            v.as_u64()
                                .or_else(|| v.get("index").and_then(|x| x.as_u64()))
                        })
                        .unwrap_or(0) as u32;
                    let size = decoded.width().min(decoded.height());
                    image::imageops::crop_imm(
                        &decoded,
                        0,
                        (index * size).min(decoded.height() - size),
                        size,
                        size,
                    )
                    .to_image()
                } else {
                    decoded
                }
            } else {
                decoded
            };
            if dark_cutout && frame.width() == 16 && frame.height() == 16 {
                native_leaf_mips = Some(texture_mips::dark_cutout(frame, 5));
                image::imageops::resize(
                    &native_leaf_mips.as_ref().unwrap()[0],
                    tile,
                    tile,
                    FilterType::Nearest,
                )
            } else {
                image::imageops::resize(&frame, span_w * tile, span_h * tile, FilterType::Nearest)
            }
        } else if id.path == WHITE_PATH {
            RgbaImage::from_pixel(tile, tile, image::Rgba([255; 4]))
        } else {
            if id.path != "missingno" {
                missing.push(id.clone());
            }
            let mut checker = RgbaImage::new(tile, tile);
            for y in 0..tile {
                for x in 0..tile {
                    checker.put_pixel(
                        x,
                        y,
                        if (x / 8 + y / 8) % 2 == 0 {
                            image::Rgba([255, 0, 255, 255])
                        } else {
                            image::Rgba([0, 0, 0, 255])
                        },
                    );
                }
            }
            checker
        };
        image::imageops::replace(
            &mut pixels,
            &tile_image,
            (col * tile) as i64,
            (row * tile) as i64,
        );
        let tile_mips = if let Some(native) = native_leaf_mips {
            // Keep the 16-pixel foliage cutout chain intact while the 64-pixel
            // atlas also preserves the native 64-pixel chest entity texture.
            let mut levels = vec![tile_image];
            levels.push(image::imageops::resize(
                &native[0],
                32,
                32,
                FilterType::Nearest,
            ));
            levels.extend(native.into_iter().take(3));
            levels
        } else if dark_cutout {
            texture_mips::dark_cutout(tile_image, 5)
        } else {
            (0..5)
                .map(|level| {
                    image::imageops::resize(
                        &tile_image,
                        (span_w * tile) >> level,
                        (span_h * tile) >> level,
                        FilterType::Triangle,
                    )
                })
                .collect::<Vec<_>>()
        };
        // The dark-cutout source also alters transparent RGB at level zero.
        if dark_cutout {
            image::imageops::replace(
                &mut pixels,
                &tile_mips[0],
                (col * tile) as i64,
                (row * tile) as i64,
            );
        }
        for (level, mip) in mipmaps.iter_mut().enumerate() {
            let mip_tile = tile >> (level + 1);
            image::imageops::replace(
                mip,
                &tile_mips[level + 1],
                (col * mip_tile) as i64,
                (row * mip_tile) as i64,
            );
        }
        let width = (side * tile) as f32;
        slots.insert(
            id,
            [
                (col * tile) as f32 / width + 0.5 / width,
                (row * tile) as f32 / width + 0.5 / width,
                ((col + span_w) * tile) as f32 / width - 0.5 / width,
                ((row + span_h) * tile) as f32 / width - 0.5 / width,
            ],
        );
    }
    Ok(Atlas {
        pixels,
        mipmaps,
        slots,
        missing,
        animated,
        animated_tiles,
        translucent,
    })
}
/// Grid cells for each texture's span (columns, rows) in a `side`-cell
/// square: the multi-cell ones first, then the rest in order, each at the
/// first free spot; `None` when they do not fit.
fn place_cells(spans: &[(u32, u32)], side: u32) -> Option<Vec<(u32, u32)>> {
    let mut used = vec![false; (side * side) as usize];
    let mut positions = vec![(0, 0); spans.len()];
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(spans[i].0 * spans[i].1));
    let mut first_free = 0;
    for i in order {
        let (w, h) = spans[i];
        let fits = |col: u32, row: u32, used: &[bool]| {
            col + w <= side && row + h <= side && (row..row + h).all(|r| (col..col + w).all(|c| !used[(r * side + c) as usize]))
        };
        let start = if (w, h) == (1, 1) { first_free } else { 0 };
        let cell = (start..side * side).find(|&cell| fits(cell % side, cell / side, &used))?;
        let (col, row) = (cell % side, cell / side);
        for r in row..row + h {
            for c in col..col + w {
                used[(r * side + c) as usize] = true;
            }
        }
        if (w, h) == (1, 1) {
            first_free = cell + 1;
        }
        positions[i] = (col, row);
    }
    Some(positions)
}
fn animated_tile(
    sheet: &RgbaImage,
    metadata: &serde_json::Value,
    origin: (u32, u32),
) -> Option<AnimatedTile> {
    animated_frames(sheet, metadata, origin, |frame| {
        let tile = image::imageops::resize(&frame, 64, 64, FilterType::Nearest);
        (0..5)
            .map(|level| {
                image::imageops::resize(&tile, 64 >> level, 64 >> level, FilterType::Triangle)
            })
            .collect()
    })
}
pub(crate) fn animation_frame(
    sheet: &RgbaImage,
    metadata: &serde_json::Value,
    tick: u64,
) -> Option<RgbaImage> {
    let tile = animated_frames(sheet, metadata, (0, 0), |frame| vec![frame])?;
    let image = tile.image_at(tile.state_at(tick), 0).into_owned();
    Some(image)
}
fn animated_frames(
    sheet: &RgbaImage,
    metadata: &serde_json::Value,
    origin: (u32, u32),
    levels: impl Fn(RgbaImage) -> Vec<RgbaImage>,
) -> Option<AnimatedTile> {
    let square = sheet.width().min(sheet.height());
    let width = metadata
        .get("width")
        .and_then(|v| v.as_u64())
        .unwrap_or(square as u64) as u32;
    let height = metadata
        .get("height")
        .and_then(|v| v.as_u64())
        .unwrap_or(square as u64) as u32;
    if width == 0 || height == 0 || sheet.width() % width != 0 || sheet.height() % height != 0 {
        return None;
    }
    let columns = sheet.width() / width;
    let count = columns * (sheet.height() / height);
    let frametime = metadata
        .get("frametime")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    let entries = metadata.get("frames").and_then(|v| v.as_array());
    let sequence = if let Some(entries) = entries {
        entries
            .iter()
            .filter_map(|entry| {
                let index = entry.as_u64().or_else(|| entry.get("index")?.as_u64())? as usize;
                let time = entry
                    .get("time")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(frametime as u64) as u32;
                (index < count as usize && time > 0).then_some((index, time))
            })
            .collect::<Vec<_>>()
    } else {
        (0..count as usize)
            .map(|index| (index, frametime))
            .collect()
    };
    if sequence.len() <= 1 || frametime == 0 {
        return None;
    }
    let frames = (0..count)
        .map(|index| {
            let frame = image::imageops::crop_imm(
                sheet,
                index % columns * width,
                index / columns * height,
                width,
                height,
            )
            .to_image();
            levels(frame)
        })
        .collect();
    Some(AnimatedTile {
        origin,
        frames,
        sequence,
        interpolate: metadata.get("interpolate").and_then(|v| v.as_bool()) == Some(true),
    })
}
fn mesh_chunk<S: Scene>(
    scene: &S,
    chunk: ChunkPos,
    models: &HashMap<String, Vec<(ResolvedModel, u32)>>,
    atlas: &Atlas,
    tint_source: &BiomeTint,
    light: &SkyLight,
) -> Result<ChunkMesh> {
    let mut mesh = ChunkMesh::default();
    let mut transparent_indices = Vec::new();
    let full_cube_models: HashMap<_, _> = models
        .iter()
        .map(|(key, variants)| (key.clone(), all_full_cubes(variants)))
        .collect();
    for x in chunk.0 * 16..chunk.0 * 16 + 16 {
        for z in chunk.1 * 16..chunk.1 * 16 + 16 {
            for y in scene.vertical_range() {
                let Some(block) = scene.block((x, y, z)) else {
                    continue;
                };
                append_block(
                    scene,
                    (x, y, z),
                    block,
                    || {
                        models
                            .get(&model_key(block))
                            .map(Vec::as_slice)
                            .ok_or_else(|| anyhow!("uncached model"))
                    },
                    |neighbor, side| {
                        scene.face_hidden((x, y, z), neighbor, side).unwrap_or_else(|| {
                            scene.block(neighbor).is_some_and(|b| {
                                hides_shared_face(block, b)
                                    || (b.is_opaque()
                                        && full_cube_models
                                            .get(&model_key(b))
                                            .copied()
                                            .unwrap_or(false))
                            })
                        })
                    },
                    atlas,
                    tint_source,
                    light,
                    &mut mesh,
                    &mut transparent_indices,
                )?;
            }
        }
    }
    Ok(finish_mesh(mesh, transparent_indices))
}
/// Whether every variant has a full-cube element, so an opaque block with
/// these models hides the faces of its neighbors.
pub(crate) fn all_full_cubes(variants: &[(ResolvedModel, u32)]) -> bool {
    variants.iter().all(|(model, _)| {
        model
            .elements
            .iter()
            .any(|element| element.from == [0.0; 3] && element.to == [1.0; 3])
    })
}
/// Moves the translucent faces after the opaque ones.
/// One model face baked for a block state's variant (vanilla `BakedQuad`):
/// everything [`append_block`] works out per face that does not depend on
/// where the block stands, so a section build only culls, shades, lights
/// and emits it.
#[derive(Clone, Debug)]
pub(crate) struct BakedQuad {
    /// The neighbour offset whose block may hide it, for cullable faces.
    cull: Option<(i32, i32, i32)>,
    /// The neighbour its light comes from.
    light_offset: (i32, i32, i32),
    /// For a full cube's face, the two axes its corner shade samples span.
    ao_axes: Option<[usize; 2]>,
    corners: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    shade: f32,
    tint_index: Option<usize>,
    transparent: bool,
    alpha: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TintKind {
    None,
    Grass,
    GrassBlock,
    GrassOfLowerHalf,
    SugarCane,
    WhiteThenGrass,
    Foliage,
    DryFoliage,
    Water,
    WaterParticles,
    Constant([f32; 3]),
    LilyPad,
    Dye([f32; 3]),
}

fn rgb_of(color: u32) -> [f32; 3] {
    [((color >> 16) & 255) as f32 / 255.0, ((color >> 8) & 255) as f32 / 255.0, (color & 255) as f32 / 255.0]
}

impl TintKind {
    pub(crate) fn of(block: &Block) -> Self {
        if let Some(dye) = crate::special_models::banner_dye(block) {
            return Self::Dye(dye);
        }
        let number = |key: &str| block.properties.get(key).and_then(|value| value.parse::<u32>().ok()).unwrap_or(0);
        match block.id.path.as_str() {
            "tall_grass" | "large_fern" if block.properties.get("half").is_some_and(|half| half == "upper") => Self::GrassOfLowerHalf,
            "tall_grass" | "large_fern" | "fern" | "short_grass" | "potted_fern" | "bush" => Self::Grass,
            "sugar_cane" => Self::SugarCane,
            "grass_block" => Self::GrassBlock,
            "pink_petals" | "wildflowers" => Self::WhiteThenGrass,
            "spruce_leaves" => Self::Constant(rgb_of(0x619961)),
            "birch_leaves" => Self::Constant(rgb_of(0x80a755)),
            "oak_leaves" | "jungle_leaves" | "acacia_leaves" | "dark_oak_leaves" | "vine" | "mangrove_leaves" => Self::Foliage,
            "leaf_litter" => Self::DryFoliage,
            "water_cauldron" => Self::Water,
            "water" | "bubble_column" => Self::WaterParticles,
            "redstone_wire" => Self::Constant(rgb_of(redstone_wire_color_argb(number("power").min(15) as u8))),
            "attached_melon_stem" | "attached_pumpkin_stem" => Self::Constant(rgb_of(0xe0c71c)),
            "melon_stem" | "pumpkin_stem" => {
                let age = number("age").min(7);
                Self::Constant(rgb_of(((age * 32) << 16) | ((255 - age * 8) << 8) | (age * 4)))
            }
            "lily_pad" => Self::LilyPad,
            _ => Self::None,
        }
    }

    pub(crate) fn biome_pos(self, (x, y, z): BlockPos) -> BlockPos {
        if self == Self::GrassOfLowerHalf {
            (x, y - 1, z)
        } else {
            (x, y, z)
        }
    }

    pub(crate) fn tint(self, tint_index: Option<usize>, biome: BiomeSample, source: &BiomeTint) -> [f32; 3] {
        match (self, tint_index) {
            (_, None) => [1.0; 3],
            (Self::WhiteThenGrass, Some(1)) => source.grass(biome),
            (_, Some(index)) if index > 0 => [1.0; 3],
            (Self::Grass | Self::GrassBlock | Self::GrassOfLowerHalf | Self::SugarCane, _) => source.grass(biome),
            (Self::Foliage, _) => source.foliage(biome),
            (Self::DryFoliage, _) => source.dry_foliage(biome),
            (Self::Water, _) => biome.water_color.map(|channel| channel as f32 / 255.0),
            (Self::Constant(color) | Self::Dye(color), _) => color,
            (Self::LilyPad, _) => rgb_of(0x208030),
            (Self::None | Self::WhiteThenGrass | Self::WaterParticles, _) => [1.0; 3],
        }
    }

    pub(crate) fn in_hand(self, tint_index: usize, source: &BiomeTint) -> [f32; 3] {
        let default_grass = || source.grass(BiomeSample { temperature: 0.5, downfall: 1.0, grass_color: None, ..BiomeSample::THE_VOID });
        match (self, tint_index) {
            (Self::WhiteThenGrass, 1) => default_grass(),
            (_, index) if index > 0 => [1.0; 3],
            (Self::Grass | Self::GrassBlock | Self::GrassOfLowerHalf, _) => default_grass(),
            (Self::Foliage, _) => rgb_of(0x48b518),
            (Self::DryFoliage, _) => rgb_of(0x5c3c32),
            (Self::Constant(color) | Self::Dye(color), _) => color,
            (Self::LilyPad, _) => rgb_of(0x71c35c),
            (Self::None | Self::SugarCane | Self::WhiteThenGrass | Self::Water | Self::WaterParticles, _) => [1.0; 3],
        }
    }

    pub(crate) fn particle_tint(self, biome: BiomeSample, source: &BiomeTint) -> [f32; 3] {
        match self {
            Self::GrassBlock | Self::Dye(_) => [1.0; 3],
            Self::WaterParticles => biome.water_color.map(|channel| channel as f32 / 255.0),
            _ => self.tint(Some(0), biome, source),
        }
    }
}

/// Bakes a resolved model's faces for `block`, as [`append_block`] would
/// place them.
pub(crate) fn bake_quads(block: &Block, model: &ResolvedModel, atlas: &Atlas) -> Result<Vec<BakedQuad>> {
    let mut quads = Vec::new();
    let mut sizes = Vec::new();
    for element in &model.elements {
        let size = (0..3).map(|axis| (element.to[axis] - element.from[axis]).abs()).product::<f32>();
        for face in &element.faces {
            let cull_dir = turned_direction(face.cullface.as_deref().unwrap_or(face.direction.as_str()), element)?;
            let cull_delta = direction(cull_dir)?;
            let corners = element_corners(element, &face.direction)?;
            let dir = quad_direction(element, &face.direction, &corners)?;
            let delta = direction(dir)?;
            let entity = entity_face(face);
            let [u0, v0, u1, v1] = if entity { atlas.entity_region(&face.texture) } else { atlas.region(&face.texture) };
            let uv = face.corner_uvs().map(|[s, t]| [u0 + (u1 - u0) * s, v0 + (v1 - v0) * t]);
            let shade = if entity {
                crate::cow_render::entity_shade(quad_normal(&corners).unwrap_or(Vec3::Y))
            } else {
                face_shade(element.shade_direction_override.as_deref().unwrap_or(dir))
            };
            let light_offset = if entity {
                (0, 0, 0)
            } else if face.cull {
                cull_delta
            } else if face_cubic(dir, &corners) {
                delta
            } else {
                (0, 0, 0)
            };
            let full_cube = element.from == [0.0; 3] && element.to == [1.0; 3] && element.rotation.is_none() && !entity;
            let ao_axes = full_cube.then(|| match dir {
                "up" | "down" => [0, 2],
                "north" | "south" => [0, 1],
                _ => [1, 2],
            });
            quads.push(BakedQuad {
                cull: face.cull.then_some(cull_delta),
                light_offset,
                ao_axes,
                corners,
                uv,
                shade,
                tint_index: face.tint_index,
                transparent: block.id.path == "water" || atlas.translucent(face),
                alpha: entity_face_alpha(face),
            });
            sizes.push(size);
        }
    }
    let mut sized: Vec<(f32, BakedQuad)> = sizes.into_iter().zip(quads).map(|(size, quad)| (if quad.transparent { size } else { 0.0 }, quad)).collect();
    sized.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(sized.into_iter().map(|(_, quad)| quad).collect())
}

/// [`append_block`] for a block whose faces are baked: the same vertices,
/// with `ao_occluder(pos)` standing for the block test in
/// [`ambient_vertex`] and `hidden_by` for the cull test.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_baked<S: Scene>(
    scene: &S,
    (x, y, z): BlockPos,
    quads: &[BakedQuad],
    tint_kind: TintKind,
    waterlogged: bool,
    hidden_by: impl Fn(BlockPos, Direction) -> bool,
    ao_occluder: impl Fn(BlockPos) -> bool,
    atlas: &Atlas,
    tint_source: &BiomeTint,
    light: &SkyLight,
    mesh: &mut ChunkMesh,
    transparent_indices: &mut Vec<u32>,
) -> Result<()> {
    let mut biome = None;
    for quad in quads {
        if quad.cull.is_some_and(|(dx, dy, dz)| hidden_by((x + dx, y + dy, z + dz), cull_side((dx, dy, dz)))) {
            continue;
        }
        let biome = *biome.get_or_insert_with(|| scene.biome_at(tint_kind.biome_pos((x, y, z))));
        let tint = tint_kind.tint(quad.tint_index, biome, tint_source);
        let start = mesh.vertices.len() as u32;
        let light_pos = (x + quad.light_offset.0, y + quad.light_offset.1, z + quad.light_offset.2);
        let block_light = light.get_block(light_pos) as f32;
        for (corner, uv) in quad.corners.iter().zip(quad.uv) {
            let (ao, sky_light) = match quad.ao_axes {
                Some(axes) => ambient_corner(light, light_pos, axes, *corner, &ao_occluder),
                None => (1.0, light.get(light_pos) as f32),
            };
            mesh.vertices.push(Vertex {
                position: [x as f32 + corner[0], y as f32 + corner[1], z as f32 + corner[2]],
                uv,
                color: [quad.shade * ao * tint[0], quad.shade * ao * tint[1], quad.shade * ao * tint[2], quad.alpha],
                sky_light,
                block_light,
            });
        }
        let indices = if quad.transparent { &mut *transparent_indices } else { &mut mesh.indices };
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
    if waterlogged {
        append_fluid(scene, (x, y, z), atlas, tint_source, light, mesh, transparent_indices)?;
    }
    Ok(())
}

/// [`ambient_vertex`] with its axes worked out and its block test given.
fn ambient_corner(light: &SkyLight, base: (i32, i32, i32), axes: [usize; 2], corner: [f32; 3], occluder: &impl Fn(BlockPos) -> bool) -> (f32, f32) {
    let mut side = [[0; 3]; 2];
    for (i, axis) in axes.into_iter().enumerate() {
        side[i][axis] = if corner[axis] < 0.5 { -1 } else { 1 };
    }
    let positions = [
        base,
        (base.0 + side[0][0], base.1 + side[0][1], base.2 + side[0][2]),
        (base.0 + side[1][0], base.1 + side[1][1], base.2 + side[1][2]),
        (base.0 + side[0][0] + side[1][0], base.1 + side[0][1] + side[1][1], base.2 + side[0][2] + side[1][2]),
    ];
    let center_light = light.get(base);
    let shade = positions.iter().map(|&p| if occluder(p) { 0.2 } else { 1.0 }).sum::<f32>() * 0.25;
    let sky = positions
        .iter()
        .map(|&p| {
            let level = light.get(p);
            if center_light > 2 && level == 0 { center_light as f32 } else { level as f32 }
        })
        .sum::<f32>()
        * 0.25;
    (shade, sky)
}

pub(crate) fn variant_for(variants: &[(ResolvedModel, u32)], pos: BlockPos) -> usize {
    variant_index(variants, pos)
}

pub fn sheeted_decal_uv(direction: &str, [x, y, z]: [f32; 3]) -> [f32; 2] {
    match direction {
        "down" => [x, -z],
        "up" => [x, z],
        "north" => [-x, -y],
        "south" => [x, -y],
        "west" => [-z, -y],
        _ => [z, -y],
    }
}

pub fn crumbling_quads(variants: &[(ResolvedModel, u32)], (x, y, z): BlockPos) -> Result<Vec<[[f32; 5]; 4]>> {
    let Some((model, _)) = variants.get(variant_index(variants, (x, y, z))) else {
        return Ok(Vec::new());
    };
    let mut quads = Vec::new();
    for element in &model.elements {
        for face in &element.faces {
            let corners = element_corners(element, &face.direction)?;
            let direction = quad_direction(element, &face.direction, &corners)?;
            quads.push(corners.map(|corner| {
                let [u, v] = sheeted_decal_uv(direction, corner);
                [x as f32 + corner[0], y as f32 + corner[1], z as f32 + corner[2], u, v]
            }));
        }
    }
    Ok(quads)
}

pub(crate) fn finish_mesh(mut mesh: ChunkMesh, transparent_indices: Vec<u32>) -> ChunkMesh {
    if !transparent_indices.is_empty() {
        mesh.transparent_start = Some(mesh.indices.len() as u32);
        mesh.indices.extend(transparent_indices);
    }
    mesh
}
/// Appends one block's faces. `variants` supplies its resolved models when
/// the block is not a fluid; `hidden_by(neighbor, side)` is whether the block at a
/// neighboring position hides a cullable face toward it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_block<'m, S: Scene>(
    scene: &S,
    (x, y, z): BlockPos,
    block: &Block,
    variants: impl FnOnce() -> Result<&'m [(ResolvedModel, u32)]>,
    hidden_by: impl Fn(BlockPos, Direction) -> bool,
    atlas: &Atlas,
    tint_source: &BiomeTint,
    light: &SkyLight,
    mesh: &mut ChunkMesh,
    transparent_indices: &mut Vec<u32>,
) -> Result<()> {
    if matches!(block.id.path.as_str(), "water" | "lava") {
        return append_fluid(
            scene,
            (x, y, z),
            atlas,
            tint_source,
            light,
            mesh,
            transparent_indices,
        );
    }
    let variants = variants()?;
    let model = &variants[variant_index(variants, (x, y, z))].0;
    let tint_kind = TintKind::of(block);
    let mut biome = None;
    for element in &model.elements {
        for face in &element.faces {
            // Culled quads go with their cullface (turned with the
            // blockstate), and take their light from that neighbour.
            let cull_dir = turned_direction(face.cullface.as_deref().unwrap_or(face.direction.as_str()), element)?;
            let cull_delta = direction(cull_dir)?;
            if face.cull && hidden_by((x + cull_delta.0, y + cull_delta.1, z + cull_delta.2), cull_side(cull_delta)) {
                continue;
            }
            let corners = element_corners(element, &face.direction)?;
            let dir = quad_direction(element, &face.direction, &corners)?;
            let delta = direction(dir)?;
            let entity = entity_face(face);
            let [u0, v0, u1, v1] = if entity { atlas.entity_region(&face.texture) } else { atlas.region(&face.texture) };
            let uv = face.corner_uvs().map(|[s, t]| [u0 + (u1 - u0) * s, v0 + (v1 - v0) * t]);
            let shade = if entity {
                crate::cow_render::entity_shade(quad_normal(&corners).unwrap_or(Vec3::Y))
            } else {
                face_shade(element.shade_direction_override.as_deref().unwrap_or(dir))
            };
            let biome = *biome.get_or_insert_with(|| scene.biome_at(tint_kind.biome_pos((x, y, z))));
            let tint = tint_kind.tint(face.tint_index, biome, tint_source);
            let alpha = entity_face_alpha(face);
            let start = mesh.vertices.len() as u32;
            // `ModelBlockRenderer.tesselateFlat` and `prepareQuadFlat`: a
            // culled quad is lit from its cullface's neighbour, another from
            // the neighbour it faces only when it lies on that side
            // (`faceCubic`), else from its own block.
            let light_pos = if entity {
                (x, y, z)
            } else if face.cull {
                (x + cull_delta.0, y + cull_delta.1, z + cull_delta.2)
            } else if face_cubic(dir, &corners) {
                (x + delta.0, y + delta.1, z + delta.2)
            } else {
                (x, y, z)
            };
            let full_cube = element.from == [0.0; 3] && element.to == [1.0; 3] && element.rotation.is_none() && !entity;
            for (corner, uv) in corners.iter().zip(uv) {
                let (ao, sky_light) = if full_cube {
                    ambient_vertex(scene, light, light_pos, dir, *corner)
                } else {
                    (1.0, light.get(light_pos) as f32)
                };
                mesh.vertices.push(Vertex {
                    position: [
                        x as f32 + corner[0],
                        y as f32 + corner[1],
                        z as f32 + corner[2],
                    ],
                    uv,
                    color: [
                        shade * ao * tint[0],
                        shade * ao * tint[1],
                        shade * ao * tint[2],
                        alpha,
                    ],
                    sky_light,
                    block_light: light.get_block(light_pos) as f32,
                });
            }
            let indices = if block.id.path == "water" || atlas.translucent(face) {
                &mut *transparent_indices
            } else {
                &mut mesh.indices
            };
            indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
    if FluidCell::at(scene, (x, y, z)).is_some() {
        append_fluid(
            scene,
            (x, y, z),
            atlas,
            tint_source,
            light,
            mesh,
            transparent_indices,
        )?;
    }
    Ok(())
}
fn fluid_render_height<S: Scene>(scene: &S, kind: FluidKind, pos: BlockPos) -> f32 {
    if let Some(cell) = FluidCell::at(scene, pos).filter(|cell| cell.kind == kind) {
        cell.height(scene, pos)
    } else if scene.full_collision_at(pos) {
        -1.0
    } else {
        0.0
    }
}
fn fluid_corner_height<S: Scene>(
    scene: &S,
    kind: FluidKind,
    own: f32,
    a: f32,
    b: f32,
    corner: BlockPos,
) -> f32 {
    if a >= 1.0 || b >= 1.0 {
        return 1.0;
    }
    let mut heights = vec![own, a, b];
    if a > 0.0 || b > 0.0 {
        heights.push(fluid_render_height(scene, kind, corner));
    }
    let (sum, weight) =
        heights
            .into_iter()
            .filter(|h| *h >= 0.0)
            .fold((0.0, 0.0), |(sum, weight), h| {
                let w = if h >= 0.8 { 10.0 } else { 1.0 };
                (sum + h * w, weight + w)
            });
    if weight == 0.0 {
        0.0
    } else {
        sum / weight
    }
}
fn fluid_quad(
    mesh: &mut ChunkMesh,
    transparent: &mut Vec<u32>,
    vertices: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    color: [f32; 4],
    sky: f32,
    block_light: f32,
    water: bool,
) {
    let start = mesh.vertices.len() as u32;
    for (position, uv) in vertices.into_iter().zip(uv) {
        mesh.vertices.push(Vertex {
            position,
            uv,
            color,
            sky_light: sky,
            block_light,
        });
    }
    let indices = if water {
        transparent
    } else {
        &mut mesh.indices
    };
    indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    if !water {
        indices.extend_from_slice(&[start, start + 2, start + 1, start, start + 3, start + 2]);
    }
    mesh.faces += 1;
}
thread_local! {
    static SECTION_NETHER_SHADE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}
pub(crate) fn set_section_nether_shade(nether: bool) {
    SECTION_NETHER_SHADE.with(|shade| shade.set(Some(nether)));
}
pub(crate) fn face_shade(direction: &str) -> f32 {
    let nether = SECTION_NETHER_SHADE.with(std::cell::Cell::get).unwrap_or_else(crate::cow_render::nether_entity_lighting);
    match direction {
        "up" | "down" if nether => 0.9,
        "up" => 1.0,
        "down" => 0.5,
        "north" | "south" => 0.8,
        _ => 0.6,
    }
}
thread_local! {
    /// Checks only: fluid with no faces showing goes the whole way through
    /// [`append_fluid`] instead of stopping early.
    pub(crate) static FLUID_FULL_PATH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn append_fluid<S: Scene>(
    scene: &S,
    pos: BlockPos,
    atlas: &Atlas,
    tint_source: &BiomeTint,
    light: &SkyLight,
    mesh: &mut ChunkMesh,
    transparent: &mut Vec<u32>,
) -> Result<()> {
    let cell = FluidCell::at(scene, pos).expect("fluid block has fluid state");
    let near = |d: (i32, i32, i32)| (pos.0 + d.0, pos.1 + d.1, pos.2 + d.2);
    let same = |p| FluidCell::at(scene, p).is_some_and(|other| other.kind == cell.kind);
    let up = near((0, 1, 0));
    let down = near((0, -1, 0));
    let top_open = !same(up);
    let self_hidden = |side: Direction| scene.fluid_face_hidden(pos, side.opposite(), 1.0);
    let bottom = !same(down) && !self_hidden(Direction::Down) && !scene.fluid_face_hidden(down, Direction::Down, 8.0 / 9.0);
    // The side faces, north, south, west and east: most fluid lies inside
    // a body of it and shows none of its faces.
    let sides = [(0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)].map(|d| {
        let other = near(d);
        !(same(other) || self_hidden(cull_side(d)) || scene.fluid_face_hidden(other, cull_side(d), 1.0))
    });
    if !top_open && !bottom && !sides.contains(&true) && !FLUID_FULL_PATH.with(std::cell::Cell::get) {
        return Ok(());
    }
    let water = cell.kind == FluidKind::Water;
    static TEXTURES: std::sync::OnceLock<[ResourceId; 5]> = std::sync::OnceLock::new();
    let [water_still, water_flow, lava_still, lava_flow, water_overlay] = TEXTURES.get_or_init(|| {
        ["water_still", "water_flow", "lava_still", "lava_flow", "water_overlay"].map(|name| ResourceId::parse(&format!("minecraft:block/{name}")).expect("static texture id"))
    });
    let (still, flow) = if water { (water_still.clone(), water_flow.clone()) } else { (lava_still.clone(), lava_flow.clone()) };
    let tint = if water {
        scene.biome_at(pos).water_color.map(|c| c as f32 / 255.0)
    } else {
        [1.0; 3]
    };
    let _ = tint_source;
    let [x, y, z] = [pos.0 as f32, pos.1 as f32, pos.2 as f32];
    let h = fluid_render_height(scene, cell.kind, pos);
    let nw;
    let ne;
    let sw;
    let se;
    if h >= 1.0 {
        nw = 1.0;
        ne = 1.0;
        sw = 1.0;
        se = 1.0;
    } else {
        let north = fluid_render_height(scene, cell.kind, near((0, 0, -1)));
        let south = fluid_render_height(scene, cell.kind, near((0, 0, 1)));
        let west = fluid_render_height(scene, cell.kind, near((-1, 0, 0)));
        let east = fluid_render_height(scene, cell.kind, near((1, 0, 0)));
        nw = fluid_corner_height(scene, cell.kind, h, north, west, near((-1, 0, -1)));
        ne = fluid_corner_height(scene, cell.kind, h, north, east, near((1, 0, -1)));
        sw = fluid_corner_height(scene, cell.kind, h, south, west, near((-1, 0, 1)));
        se = fluid_corner_height(scene, cell.kind, h, south, east, near((1, 0, 1)));
    }
    let top = top_open && !scene.fluid_face_hidden(up, Direction::Up, nw.min(ne).min(sw).min(se));
    let sky = light.get(pos).max(light.get(up)) as f32;
    let color = |shade: f32| [shade * tint[0], shade * tint[1], shade * tint[2], 1.0];
    let uv = |texture: &ResourceId, points: [[f32; 2]; 4]| {
        let [u0, v0, u1, v1] = atlas.region(texture);
        points.map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v])
    };
    if top {
        let mut dx = 0.0_f32;
        let mut dz = 0.0_f32;
        for d in [(0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)] {
            let target = near(d);
            let hh = fluid_render_height(scene, cell.kind, target);
            let delta = if hh > 0.0 {
                h - hh
            } else if !scene.full_collision_at(target) {
                let below = near((d.0, -1, d.2));
                let low = fluid_render_height(scene, cell.kind, below);
                if low > 0.0 {
                    h - (low - 8.0 / 9.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            dx += d.0 as f32 * delta;
            dz += d.2 as f32 * delta;
        }
        let (texture, coords) = if dx == 0.0 && dz == 0.0 {
            (&still, [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]])
        } else {
            let angle = dz.atan2(dx) - std::f32::consts::FRAC_PI_2;
            let (s, c) = (angle.sin() * 0.25, angle.cos() * 0.25);
            (
                &flow,
                [
                    [0.5 - c - s, 0.5 - c + s],
                    [0.5 - c + s, 0.5 + c + s],
                    [0.5 + c + s, 0.5 + c - s],
                    [0.5 + c - s, 0.5 - c - s],
                ],
            )
        };
        let texture = if atlas.contains(texture) {
            texture
        } else {
            &still
        };
        fluid_quad(
            mesh,
            transparent,
            [
                [x, y + nw - 0.001, z],
                [x, y + sw - 0.001, z + 1.0],
                [x + 1.0, y + se - 0.001, z + 1.0],
                [x + 1.0, y + ne - 0.001, z],
            ],
            uv(texture, coords),
            color(face_shade("up")),
            sky,
            light.get_block(pos) as f32,
            water,
        );
        if water && backward_up_face(scene, up, cell.kind) {
            backward_fluid_face(mesh, transparent);
        }
    }
    if bottom {
        fluid_quad(
            mesh,
            transparent,
            [
                [x, y + 0.001, z],
                [x + 1.0, y + 0.001, z],
                [x + 1.0, y + 0.001, z + 1.0],
                [x, y + 0.001, z + 1.0],
            ],
            uv(&still, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
            color(face_shade("down")),
            light.get(down) as f32,
            light.get_block(down) as f32,
            water,
        );
    }
    for (side, (delta, points, heights, shade)) in [
        (
            (0, 0, -1),
            [[x, y, z + 0.001], [x + 1.0, y, z + 0.001]],
            [nw, ne],
            0.8,
        ),
        (
            (0, 0, 1),
            [[x + 1.0, y, z + 0.999], [x, y, z + 0.999]],
            [se, sw],
            0.8,
        ),
        (
            (-1, 0, 0),
            [[x + 0.001, y, z + 1.0], [x + 0.001, y, z]],
            [sw, nw],
            0.6,
        ),
        (
            (1, 0, 0),
            [[x + 0.999, y, z], [x + 0.999, y, z + 1.0]],
            [ne, se],
            0.6,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let [ha, hb] = heights;
        if !sides[side] || scene.fluid_face_hidden(near(delta), cull_side(delta), ha.max(hb)) {
            continue;
        }
        let [[ax, _, az], [bx, _, bz]] = points;
        let overlay = water && scene.fluid_overlay_at(near(delta));
        let side_texture = if overlay && atlas.contains(water_overlay) { water_overlay } else { &flow };
        fluid_quad(
            mesh,
            transparent,
            [
                [ax, y + ha - 0.001, az],
                [bx, y + hb - 0.001, bz],
                [bx, y, bz],
                [ax, y, az],
            ],
            uv(
                side_texture,
                [
                    [0.0, (1.0 - ha) * 0.5],
                    [0.5, (1.0 - hb) * 0.5],
                    [0.5, 0.5],
                    [0.0, 0.5],
                ],
            ),
            color(face_shade("up") * shade),
            sky,
            light.get_block(pos) as f32,
            water,
        );
        if water && !overlay {
            backward_fluid_face(mesh, transparent);
        }
    }
    Ok(())
}
fn backward_up_face<S: Scene>(scene: &S, above: BlockPos, kind: FluidKind) -> bool {
    (-1..=1).any(|dx| {
        (-1..=1).any(|dz| {
            let at = (above.0 + dx, above.1, above.2 + dz);
            !FluidCell::at(scene, at).is_some_and(|other| other.kind == kind) && !scene.solid_render_at(at)
        })
    })
}
fn backward_fluid_face(mesh: &ChunkMesh, transparent: &mut Vec<u32>) {
    let start = mesh.vertices.len() as u32 - 4;
    transparent.extend_from_slice(&[start, start + 2, start + 1, start, start + 3, start + 2]);
}
/// Draw only a newly placed block while its neighboring chunks are rebuilt on
/// the worker. This uses the same model, atlas, tint, AO and face rules as the
/// chunk path, but samples the already available light map for this brief pose.
pub fn block_preview<S: Scene>(
    scene: &S,
    packs: &PackStack,
    atlas: &Atlas,
    light: &SkyLight,
    tint_source: &BiomeTint,
    pos: BlockPos,
) -> Result<ChunkMesh> {
    let Some(block) = scene.block(pos) else {
        return Ok(ChunkMesh::default());
    };
    if matches!(block.id.path.as_str(), "water" | "lava") {
        let mut mesh = ChunkMesh::default();
        let mut transparent = Vec::new();
        append_fluid(
            scene,
            pos,
            atlas,
            tint_source,
            light,
            &mut mesh,
            &mut transparent,
        )?;
        if !transparent.is_empty() {
            mesh.transparent_start = Some(mesh.indices.len() as u32);
            mesh.indices.extend(transparent);
        }
        return Ok(mesh);
    }
    let variants = resolve_block_variants(packs, block)?;
    let model = &variants[variant_index(&variants, pos)].0;
    let tint_kind = TintKind::of(block);
    let biome = scene.biome_at(tint_kind.biome_pos(pos));
    let mut mesh = ChunkMesh::default();
    let mut transparent_indices = Vec::new();
    let mut neighbor_cube = HashMap::new();
    for element in &model.elements {
        for face in &element.faces {
            if !atlas.contains(&face.texture) {
                return Err(anyhow!("atlas missing {}", face.texture.key()));
            }
            let dir = turned_direction(face.direction.as_str(), element)?;
            let delta = direction(dir)?;
            let neighbor_pos = (pos.0 + delta.0, pos.1 + delta.1, pos.2 + delta.2);
            let catalog_hidden = if face.cull { scene.face_hidden(pos, neighbor_pos, cull_side(delta)) } else { None };
            if catalog_hidden == Some(true) {
                continue;
            }
            if face.cull && catalog_hidden.is_none() {
                if let Some(neighbor) = scene.block(neighbor_pos) {
                    let full_cube = if neighbor.is_opaque() {
                        let key = model_key(neighbor);
                        if let Some(&cube) = neighbor_cube.get(&key) {
                            cube
                        } else {
                            let cube = resolve_block_variants(packs, neighbor)?.iter().all(
                                |(model, _)| {
                                    model
                                        .elements
                                        .iter()
                                        .any(|part| part.from == [0.0; 3] && part.to == [1.0; 3])
                                },
                            );
                            neighbor_cube.insert(key, cube);
                            cube
                        }
                    } else {
                        false
                    };
                    if hides_shared_face(block, neighbor) || (neighbor.is_opaque() && full_cube) {
                        continue;
                    }
                }
            }
            let corners = element_corners(element, &face.direction)?;
            let entity = entity_face(face);
            let [u0, v0, u1, v1] = if entity {
                // Chest ModelPart UVs address individual pixels in a 64x64
                // entity sheet. The terrain atlas' half-texel tile inset
                // distorts one-pixel lock faces and can sample alpha beside
                // them, so preserve the entity sheet's exact coordinate span.
                atlas.region_exact(&face.texture)
            } else {
                atlas.region(&face.texture)
            };
            let uv = face.corner_uvs().map(|[s, t]| [u0 + (u1 - u0) * s, v0 + (v1 - v0) * t]);
            let shade = if entity {
                crate::cow_render::entity_shade(quad_normal(&corners).unwrap_or(Vec3::Y))
            } else {
                face_shade(element.shade_direction_override.as_deref().unwrap_or(dir))
            };
            let tint = tint_kind.tint(face.tint_index, biome, tint_source);
            let alpha = entity_face_alpha(face);
            // ChestRenderer lights every ModelPart face from the block entity
            // position; a rotated lid must not retain its closed downward
            // face's dark floor-neighbor light.
            let light_pos = if entity {
                pos
            } else {
                neighbor_pos
            };
            let full_cube = element.from == [0.0; 3] && element.to == [1.0; 3] && !entity;
            let start = mesh.vertices.len() as u32;
            for (corner, uv) in corners.iter().zip(uv) {
                let (ao, sky_light) = if full_cube {
                    ambient_vertex(scene, light, light_pos, dir, *corner)
                } else {
                    (1.0, light.get(light_pos) as f32)
                };
                mesh.vertices.push(Vertex {
                    position: [
                        pos.0 as f32 + corner[0],
                        pos.1 as f32 + corner[1],
                        pos.2 as f32 + corner[2],
                    ],
                    uv,
                    color: [
                        shade * ao * tint[0],
                        shade * ao * tint[1],
                        shade * ao * tint[2],
                        alpha,
                    ],
                    sky_light,
                    block_light: light.get_block(light_pos) as f32,
                });
            }
            let indices = if block.id.path == "water" || atlas.translucent(face) {
                &mut transparent_indices
            } else {
                &mut mesh.indices
            };
            indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
    if FluidCell::at(scene, pos).is_some() {
        append_fluid(scene, pos, atlas, tint_source, light, &mut mesh, &mut transparent_indices)?;
    }
    if !transparent_indices.is_empty() {
        mesh.transparent_start = Some(mesh.indices.len() as u32);
        mesh.indices.extend(transparent_indices);
    }
    Ok(mesh)
}
/// HalfTransparentBlock skips its own neighboring face. Fancy cutout leaves
/// retain shared faces in 26.3; rasterizer back-face culling hides their backs.
pub(crate) fn hides_shared_face(block: &Block, neighbor: &Block) -> bool {
    if block.id != neighbor.id {
        return false;
    }
    let path = block.id.path.as_str();
    path == "water" || path == "glass" || path == "tinted_glass" || path.ends_with("_stained_glass")
}

/// Full-cube case of BlockModelLighter's four neighboring shade and light
/// samples. Keeping values per vertex produces the smooth block-edge fade.
fn ambient_vertex<S: Scene>(
    scene: &S,
    light: &SkyLight,
    base: (i32, i32, i32),
    dir: &str,
    corner: [f32; 3],
) -> (f32, f32) {
    let axes = match dir {
        "up" | "down" => [0, 2],
        "north" | "south" => [0, 1],
        _ => [1, 2],
    };
    let mut side = [[0; 3]; 2];
    for (i, axis) in axes.into_iter().enumerate() {
        side[i][axis] = if corner[axis] < 0.5 { -1 } else { 1 };
    }
    let positions = [
        base,
        (
            base.0 + side[0][0],
            base.1 + side[0][1],
            base.2 + side[0][2],
        ),
        (
            base.0 + side[1][0],
            base.1 + side[1][1],
            base.2 + side[1][2],
        ),
        (
            base.0 + side[0][0] + side[1][0],
            base.1 + side[0][1] + side[1][1],
            base.2 + side[0][2] + side[1][2],
        ),
    ];
    let center_light = light.get(base);
    let shade = positions
        .iter()
        .map(|&p| if scene.shade_darkens_at(p) { 0.2 } else { 1.0 })
        .sum::<f32>()
        * 0.25;
    let sky = positions
        .iter()
        .map(|&p| {
            let level = light.get(p);
            if center_light > 2 && level == 0 {
                center_light as f32
            } else {
                level as f32
            }
        })
        .sum::<f32>()
        * 0.25;
    (shade, sky)
}
fn direction(dir: &str) -> Result<(i32, i32, i32)> {
    Ok(match dir {
        "down" => (0, -1, 0),
        "up" => (0, 1, 0),
        "north" => (0, 0, -1),
        "south" => (0, 0, 1),
        "west" => (-1, 0, 0),
        "east" => (1, 0, 0),
        _ => return Err(anyhow!("unsupported face direction {dir}")),
    })
}
fn cull_side(delta: (i32, i32, i32)) -> Direction {
    match delta {
        (0, -1, 0) => Direction::Down,
        (0, 1, 0) => Direction::Up,
        (0, 0, -1) => Direction::North,
        (0, 0, 1) => Direction::South,
        (-1, 0, 0) => Direction::West,
        _ => Direction::East,
    }
}
fn corners(dir: &str, a: [f32; 3], b: [f32; 3]) -> Result<[[f32; 3]; 4]> {
    let [x, y, z] = a;
    let [xx, yy, zz] = b;
    Ok(match dir {
        "up" => [[x, yy, z], [x, yy, zz], [xx, yy, zz], [xx, yy, z]],
        "down" => [[x, y, zz], [x, y, z], [xx, y, z], [xx, y, zz]],
        "north" => [[xx, yy, z], [xx, y, z], [x, y, z], [x, yy, z]],
        "south" => [[x, yy, zz], [x, y, zz], [xx, y, zz], [xx, yy, zz]],
        "west" => [[x, yy, z], [x, y, z], [x, y, zz], [x, yy, zz]],
        "east" => [[xx, yy, zz], [xx, y, zz], [xx, y, z], [xx, yy, z]],
        _ => return Err(anyhow!("unsupported face direction {dir}")),
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    /// The `cross` model's planes: rotated 45° about the block's middle and
    /// rescaled, they run corner to corner; a plane in the middle of the
    /// block takes its light from its own cell, a face on the block's side
    /// from the neighbour (`BlockModelLighter.faceCubic`).
    #[test]
    fn cross_planes_turn_diagonal_and_light_from_their_own_cell() {
        let raw: serde_json::Value = serde_json::from_str(r#"{"rotation":{"origin":[8,8,8],"axis":"y","angle":45,"rescale":true}}"#).unwrap();
        let element = crate::model::Element {
            from: [0.8 / 16.0, 0.0, 0.5],
            to: [15.2 / 16.0, 1.0, 0.5],
            faces: Vec::new(),
            rotation_y: 0,
            rotation_x: 0,
            rotation: crate::model::ElementRotation::parse(&raw).unwrap(),
            shade_direction_override: None,
        };
        let corners = element_corners(&element, "north").unwrap();
        for (corner, (x, z)) in corners.iter().zip([(0.95, 0.05), (0.95, 0.05), (0.05, 0.95), (0.05, 0.95)]) {
            assert!((corner[0] - x).abs() < 1e-5 && (corner[2] - z).abs() < 1e-5, "{corner:?}");
        }
        // The normal points between north and west; north comes first.
        assert_eq!(quad_direction(&element, "north", &corners).unwrap(), "north");
        assert_eq!(quad_direction(&element, "south", &element_corners(&element, "south").unwrap()).unwrap(), "south");
        assert!(!face_cubic("north", &corners), "a diagonal plane is lit from its own cell");
        let cube = crate::model::Element { from: [0.0; 3], to: [1.0; 3], faces: Vec::new(), rotation_y: 0, rotation_x: 0, rotation: None, shade_direction_override: None };
        let top = element_corners(&cube, "up").unwrap();
        assert!(face_cubic("up", &top), "a cube's top lies on its side");
        let slab = crate::model::Element { from: [0.0; 3], to: [1.0, 0.5, 1.0], faces: Vec::new(), rotation_y: 0, rotation_x: 0, rotation: None, shade_direction_override: None };
        assert!(!face_cubic("up", &element_corners(&slab, "up").unwrap()), "a slab's top is inside the block");
        // A 22.5° torch tilt about z: no rescale.
        let raw: serde_json::Value = serde_json::from_str(r#"{"rotation":{"origin":[0,3.5,8],"axis":"z","angle":-22.5}}"#).unwrap();
        let tilt = crate::model::ElementRotation::parse(&raw).unwrap().unwrap();
        let [x, y, _] = tilt.apply([0.0, 3.5 / 16.0 + 1.0, 0.5]);
        assert!((x - (22.5f32).to_radians().sin()).abs() < 1e-5 && (y - (3.5 / 16.0 + (22.5f32).to_radians().cos())).abs() < 1e-5, "{x} {y}");
    }
    use crate::scene::{BlockPos, ChunkPos, HandcraftedScene};

    #[test]
    fn entity_sheet_region_restores_exact_pixel_coordinates() {
        let texture = ResourceId::parse("minecraft:entity/chest/normal_left").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(64, 64),
            mipmaps: vec![],
            slots: HashMap::from([(
                texture.clone(),
                [0.5 / 64.0, 0.5 / 64.0, 63.5 / 64.0, 63.5 / 64.0],
            )]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        assert_eq!(atlas.region_exact(&texture), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn entity_skin_uvs_keep_internal_pixel_boundaries_exact() {
        let id = ResourceId::parse("minecraft:entity/zombie/zombie").unwrap();
        let mut slots = HashMap::new();
        slots.insert(
            id.clone(),
            [64.5 / 128.0, 0.5 / 128.0, 127.5 / 128.0, 63.5 / 128.0],
        );
        let atlas = Atlas {
            pixels: RgbaImage::new(128, 128),
            mipmaps: Vec::new(),
            slots,
            missing: Vec::new(),
            animated: Vec::new(),
            animated_tiles: Vec::new(),
            translucent: HashMap::new(),
        };
        let [u0, v0, u1, v1] = atlas.entity_region(&id);
        assert_eq!([u0, v0, u1, v1], [0.5, 0.0, 1.0, 0.5]);
        // The zombie hand starts at skin pixel 48. With the regular inset,
        // that boundary lands at 47.75 and selects a shirt pixel instead.
        assert_eq!((u0 + (u1 - u0) * 48.0 / 64.0) * 128.0, 112.0);
        assert!(
            (atlas.region(&id)[0] + (atlas.region(&id)[2] - atlas.region(&id)[0]) * 48.0 / 64.0)
                * 128.0
                < 112.0
        );
    }

    #[test]
    fn wire_tint_uses_power_and_leaves_overlay_untinted() {
        let source = BiomeTint {
            grass: None,
            foliage: None,
            dry_foliage: None,
        };
        let wire = Block::new("minecraft:redstone_wire").with("power", "15");
        let tinted = TintKind::of(&wire).tint(Some(0), BiomeSample::THE_VOID, &source);
        assert_eq!(redstone_wire_color_argb(0), 0xff4c0000);
        assert_eq!(redstone_wire_color_argb(15), 0xffff3200);
        assert_eq!(tinted, [1.0, 50.0 / 255.0, 0.0]);
        assert_eq!(
            TintKind::of(&wire).tint(None, BiomeSample::THE_VOID, &source),
            [1.0; 3]
        );
    }

    fn colormap(rgb: [u8; 3]) -> Option<RgbaImage> {
        Some(RgbaImage::from_pixel(256, 256, image::Rgba([rgb[0], rgb[1], rgb[2], 255])))
    }

    #[test]
    fn block_tints_follow_the_vanilla_block_colors_table() {
        let source = BiomeTint { grass: colormap([10, 200, 10]), foliage: colormap([20, 150, 20]), dry_foliage: colormap([150, 100, 50]) };
        let biome = BiomeSample { water_color: [1, 2, 3], ..BiomeSample::THE_VOID };
        let grass = source.grass(biome);
        let foliage = source.foliage(biome);
        assert_eq!(grass, rgb_of(0x0ac80a));
        assert_eq!(foliage, rgb_of(0x149614));
        let tint = |id: &str, index: Option<usize>| TintKind::of(&Block::new(id)).tint(index, biome, &source);
        for untinted in ["minecraft:stonecutter", "minecraft:cherry_leaves", "minecraft:pale_oak_leaves", "minecraft:azalea_leaves", "minecraft:bamboo", "minecraft:bamboo_sapling", "minecraft:lava_cauldron", "minecraft:powder_snow_cauldron", "minecraft:water", "minecraft:bubble_column"] {
            assert_eq!(tint(untinted, Some(0)), [1.0; 3], "{untinted}");
        }
        for grassy in ["minecraft:short_grass", "minecraft:fern", "minecraft:potted_fern", "minecraft:bush", "minecraft:grass_block", "minecraft:sugar_cane", "minecraft:tall_grass", "minecraft:large_fern"] {
            assert_eq!(tint(grassy, Some(0)), grass, "{grassy}");
            assert_eq!(tint(grassy, None), [1.0; 3], "{grassy}");
            assert_eq!(tint(grassy, Some(1)), [1.0; 3], "{grassy}");
        }
        for leafy in ["minecraft:oak_leaves", "minecraft:jungle_leaves", "minecraft:acacia_leaves", "minecraft:dark_oak_leaves", "minecraft:mangrove_leaves", "minecraft:vine"] {
            assert_eq!(tint(leafy, Some(0)), foliage, "{leafy}");
        }
        assert_eq!(tint("minecraft:spruce_leaves", Some(0)), rgb_of(0x619961));
        assert_eq!(tint("minecraft:birch_leaves", Some(0)), rgb_of(0x80a755));
        assert_eq!(tint("minecraft:leaf_litter", Some(0)), rgb_of(0x966432));
        assert_eq!(tint("minecraft:water_cauldron", Some(0)), rgb_of(0x010203));
        assert_eq!(tint("minecraft:lily_pad", Some(0)), rgb_of(0x208030));
        assert_eq!(tint("minecraft:attached_pumpkin_stem", Some(0)), rgb_of(0xe0c71c));
        assert_eq!(tint("minecraft:attached_melon_stem", Some(0)), rgb_of(0xe0c71c));
        for petals in ["minecraft:pink_petals", "minecraft:wildflowers"] {
            assert_eq!(tint(petals, Some(0)), [1.0; 3], "{petals}");
            assert_eq!(tint(petals, Some(1)), grass, "{petals}");
            assert_eq!(tint(petals, Some(2)), [1.0; 3], "{petals}");
        }
        let stem = |age: &str| TintKind::of(&Block::new("minecraft:pumpkin_stem").with("age", age)).tint(Some(0), biome, &source);
        assert_eq!(stem("0"), rgb_of(0x00ff00));
        assert_eq!(stem("3"), rgb_of(0x60e70c));
        assert_eq!(stem("7"), rgb_of(0xe0c71c));
        let melon = TintKind::of(&Block::new("minecraft:melon_stem").with("age", "7"));
        assert_eq!(melon.tint(Some(0), biome, &source), rgb_of(0xe0c71c));
    }

    #[test]
    fn upper_tall_grass_takes_its_colour_from_the_lower_half() {
        let upper = TintKind::of(&Block::new("minecraft:tall_grass").with("half", "upper"));
        let lower = TintKind::of(&Block::new("minecraft:large_fern").with("half", "lower"));
        assert_eq!(upper.biome_pos((4, 70, -2)), (4, 69, -2));
        assert_eq!(lower.biome_pos((4, 70, -2)), (4, 70, -2));
        assert_eq!(TintKind::of(&Block::new("minecraft:stonecutter")).biome_pos((4, 70, -2)), (4, 70, -2));
    }

    #[test]
    fn terrain_particles_use_tint_source_zero_with_vanilla_overrides() {
        let source = BiomeTint { grass: colormap([10, 200, 10]), foliage: colormap([20, 150, 20]), dry_foliage: None };
        let biome = BiomeSample { water_color: [1, 2, 3], ..BiomeSample::THE_VOID };
        let particle = |id: &str| TintKind::of(&Block::new(id)).particle_tint(biome, &source);
        assert_eq!(particle("minecraft:grass_block"), [1.0; 3]);
        assert_eq!(particle("minecraft:short_grass"), source.grass(biome));
        assert_eq!(particle("minecraft:sugar_cane"), source.grass(biome));
        assert_eq!(particle("minecraft:water"), rgb_of(0x010203));
        assert_eq!(particle("minecraft:bubble_column"), rgb_of(0x010203));
        assert_eq!(particle("minecraft:pink_petals"), [1.0; 3]);
        assert_eq!(particle("minecraft:stonecutter"), [1.0; 3]);
        assert_eq!(particle("minecraft:leaf_litter"), rgb_of(0x5c3c32));
    }

    #[test]
    fn vanilla_stonecutter_saw_has_a_tintindex_but_no_tint_source() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let source = BiomeTint::from_pack(&packs).unwrap();
        assert!(source.dry_foliage.is_some());
        let block = Block::new("minecraft:stonecutter").with("facing", "north");
        let variants = resolve_block_variants(&packs, &block).unwrap();
        let indices = variants[0].0.elements.iter().flat_map(|element| &element.faces).filter_map(|face| face.tint_index).collect::<Vec<_>>();
        assert_eq!(indices, [0, 0]);
        let grassy = BiomeSample { grass_color: Some([0, 255, 0]), ..BiomeSample::THE_VOID };
        assert_eq!(TintKind::of(&block).tint(Some(0), grassy, &source), [1.0; 3]);
    }

    #[test]
    fn a_stonecutter_saw_with_a_tintindex_bakes_untinted() {
        let texture = ResourceId::parse("minecraft:block/stonecutter_saw").unwrap();
        let mut model = test_box_model(&texture, [0.0; 3], [1.0; 3]);
        for face in &mut model.elements[0].faces {
            face.tint = true;
            face.tint_index = Some(0);
        }
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture, [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let scene = Placed(HashMap::new());
        let light = SkyLight::build(&scene);
        let source = BiomeTint { grass: colormap([10, 200, 10]), foliage: None, dry_foliage: None };
        for id in ["minecraft:stonecutter", "minecraft:short_grass"] {
            let block = Block::new(id);
            let variants = vec![(model.clone(), 1)];
            let mut slow = ChunkMesh::default();
            append_block(&scene, (0, 0, 0), &block, || Ok(variants.as_slice()), |_, _| false, &atlas, &source, &light, &mut slow, &mut Vec::new()).unwrap();
            let quads = bake_quads(&block, &model, &atlas).unwrap();
            let mut baked = ChunkMesh::default();
            append_baked(&scene, (0, 0, 0), &quads, TintKind::of(&block), false, |_, _| false, |_| false, &atlas, &source, &light, &mut baked, &mut Vec::new()).unwrap();
            let expected = if id == "minecraft:stonecutter" { [1.0; 3] } else { source.grass(BiomeSample::THE_VOID) };
            for mesh in [&slow, &baked] {
                assert!(!mesh.vertices.is_empty());
                for vertex in &mesh.vertices {
                    let shade = [0.5, 0.6, 0.8, 1.0].into_iter().find(|shade| (vertex.color[0] / expected[0] - shade).abs() < 1e-5);
                    let shade = shade.unwrap_or_else(|| panic!("{id} {:?}", vertex.color));
                    for (drawn, wanted) in vertex.color.iter().zip(expected) {
                        assert!((drawn - shade * wanted).abs() < 1e-5, "{id} {:?}", vertex.color);
                    }
                }
            }
            assert_eq!(slow.vertices.iter().map(|vertex| vertex.color).collect::<Vec<_>>(), baked.vertices.iter().map(|vertex| vertex.color).collect::<Vec<_>>());
        }
    }

    #[test]
    fn blocks_outside_the_level_use_the_tint_sources_default_colour() {
        let source = BiomeTint { grass: colormap([10, 200, 10]), foliage: colormap([20, 150, 20]), dry_foliage: colormap([150, 100, 50]) };
        let hand = |block: Block, index: usize| TintKind::of(&block).in_hand(index, &source);
        let grass = rgb_of(0x0ac80a);
        for grassy in ["minecraft:grass_block", "minecraft:short_grass", "minecraft:fern", "minecraft:tall_grass"] {
            assert_eq!(hand(Block::new(grassy), 0), grass, "{grassy}");
            assert_eq!(hand(Block::new(grassy), 1), [1.0; 3], "{grassy}");
        }
        assert_eq!(hand(Block::new("minecraft:large_fern").with("half", "upper"), 0), grass);
        assert_eq!(hand(Block::new("minecraft:pink_petals"), 0), [1.0; 3]);
        assert_eq!(hand(Block::new("minecraft:pink_petals"), 1), grass);
        assert_eq!(hand(Block::new("minecraft:oak_leaves"), 0), rgb_of(0x48b518));
        assert_eq!(hand(Block::new("minecraft:leaf_litter"), 0), rgb_of(0x5c3c32));
        assert_eq!(hand(Block::new("minecraft:spruce_leaves"), 0), rgb_of(0x619961));
        assert_eq!(hand(Block::new("minecraft:lily_pad"), 0), rgb_of(0x71c35c));
        assert_eq!(hand(Block::new("minecraft:pumpkin_stem").with("age", "7"), 0), rgb_of(0xe0c71c));
        for white in ["minecraft:sugar_cane", "minecraft:water_cauldron", "minecraft:water", "minecraft:stonecutter"] {
            assert_eq!(hand(Block::new(white), 0), [1.0; 3], "{white}");
        }
        assert_eq!(TintKind::of(&Block::new("minecraft:sugar_cane")).tint(Some(0), BiomeSample::THE_VOID, &source), grass);
        assert_eq!(TintKind::of(&Block::new("minecraft:lily_pad")).tint(Some(0), BiomeSample::THE_VOID, &source), rgb_of(0x208030));
    }

    #[test]
    fn a_carried_grass_block_takes_the_default_grass_colour_and_a_stonecutter_stays_white() {
        let texture = ResourceId::parse("minecraft:block/grass_block_top").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let face = |direction: &str, tint_index: Option<usize>| crate::model::Face {
            direction: direction.into(),
            texture: texture.clone(),
            uv: [0.0, 0.0, 1.0, 1.0],
            cull: false,
            cullface: None,
            tint: tint_index.is_some(),
            tint_index,
            force_translucent: false,
            uv_rotation: 0,
        };
        let visual = || ItemVisual {
            hand: None,
            model: ResolvedModel {
                elements: vec![crate::model::Element {
                    from: [0.0; 3],
                    to: [1.0; 3],
                    faces: vec![face("up", Some(0)), face("north", None)],
                    rotation_y: 0,
                    rotation_x: 0,
                    rotation: None,
                    shade_direction_override: None,
                }],
            },
            ground: Mat4::IDENTITY,
            right_hand: Mat4::IDENTITY,
            left_hand: Mat4::IDENTITY,
            fixed: Mat4::IDENTITY,
            min_y: 0.0,
            tints: vec![[0.3, 0.6, 0.2]],
            flat: false,
        };
        let source = BiomeTint { grass: colormap([10, 200, 10]), foliage: None, dry_foliage: None };
        let grass = source.grass(BiomeSample { temperature: 0.5, downfall: 1.0, ..BiomeSample::THE_VOID });
        let mut visuals = ItemVisuals { tint: Some(source), ..ItemVisuals::default() };
        let sky_light = SkyLight::build(&HandcraftedScene::new());
        let packs = PackStack::open(vec![]).unwrap();
        for (id, expected) in [("minecraft:grass_block", grass), ("minecraft:stonecutter", [1.0; 3])] {
            visuals.models.insert(id.into(), visual());
            let mut mesh = ChunkMesh::default();
            visuals.append_posed_blocks(&mut mesh, &[(Mat4::IDENTITY, Vec3::new(5.5, 6.0, -6.5), id.into())], &packs, &atlas, &sky_light).unwrap();
            assert_eq!(mesh.vertices.len(), 8);
            let up = level_item_shade(Vec3::Y);
            let north = level_item_shade(Vec3::NEG_Z);
            for vertex in &mesh.vertices[..4] {
                assert_eq!(vertex.color, [expected[0] * up, expected[1] * up, expected[2] * up, 1.0], "{id}");
            }
            for vertex in &mesh.vertices[4..] {
                assert_eq!(vertex.color, [north, north, north, 1.0], "{id}");
            }
        }
    }

    #[test]
    fn discrete_frames_follow_pack_order_and_tick_durations() {
        let mut sheet = RgbaImage::new(2, 6);
        for frame in 0..3 {
            for y in frame * 2..frame * 2 + 2 {
                for x in 0..2 {
                    sheet.put_pixel(x, y, image::Rgba([frame as u8 * 40, 0, 0, 255]));
                }
            }
        }
        let metadata = serde_json::json!({
            "frametime": 2,
            "frames": [0, {"index": 2, "time": 3}, 1]
        });
        let tile = animated_tile(&sheet, &metadata, (64, 128)).unwrap();
        assert_eq!(tile.origin, (64, 128));
        assert_eq!(
            (0..7).map(|tick| tile.frame_at(tick)).collect::<Vec<_>>(),
            [0, 0, 2, 2, 2, 1, 1]
        );
        assert_eq!(tile.frame_at(7), 0);
        assert_eq!(tile.frames[2][0].get_pixel(0, 0).0, [80, 0, 0, 255]);
        assert_eq!(tile.frames[2][4].dimensions(), (4, 4));
    }

    #[test]
    fn interpolated_sprite_uses_pinned_thousandth_progress() {
        let mut sheet = RgbaImage::new(2, 4);
        for y in 2..4 {
            for x in 0..2 {
                sheet.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let tile = animated_tile(
            &sheet,
            &serde_json::json!({"frametime":8,"interpolate":true}),
            (0, 0),
        )
        .unwrap();
        let state = tile.state_at(4);
        assert_eq!(state.progress_millis, 500);
        assert_eq!((state.current, state.next), (0, 1));
        assert_eq!(tile.image_at(state, 0).get_pixel(0, 0).0, [128, 0, 0, 128]);
    }

    fn strip(colours: &[[u8; 4]]) -> RgbaImage {
        let mut sheet = RgbaImage::new(2, 2 * colours.len() as u32);
        for (frame, colour) in colours.iter().enumerate() {
            for y in frame as u32 * 2..frame as u32 * 2 + 2 {
                for x in 0..2 {
                    sheet.put_pixel(x, y, image::Rgba(*colour));
                }
            }
        }
        sheet
    }

    fn animated_atlas(tiles: Vec<AnimatedTile>) -> Atlas {
        Atlas {
            pixels: RgbaImage::new(128, 128),
            mipmaps: Vec::new(),
            slots: HashMap::new(),
            missing: Vec::new(),
            animated: Vec::new(),
            animated_tiles: tiles,
            translucent: HashMap::new(),
        }
    }

    #[test]
    fn stepped_animation_patches_only_when_its_frame_changes() {
        let sheet = strip(&[[10, 0, 0, 255], [20, 0, 0, 255], [30, 0, 0, 255]]);
        let tile = animated_tile(&sheet, &serde_json::json!({"frametime": 2}), (64, 0)).unwrap();
        let atlas = animated_atlas(vec![tile]);
        let mut animation = AtlasAnimation::new(&atlas);
        assert!(animation.tick(&atlas).is_empty());
        let patches = animation.tick(&atlas);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].origin, (64, 0));
        assert_eq!(
            patches[0]
                .levels
                .iter()
                .map(|level| level.dimensions())
                .collect::<Vec<_>>(),
            [(64, 64), (32, 32), (16, 16), (8, 8), (4, 4)]
        );
        assert!(patches[0]
            .levels
            .iter()
            .all(|level| level.get_pixel(0, 0).0 == [20, 0, 0, 255]));
        assert!(animation.tick(&atlas).is_empty());
        assert_eq!(animation.tick(&atlas)[0].levels[0].get_pixel(1, 1).0, [30, 0, 0, 255]);
        assert!(animation.tick(&atlas).is_empty());
        assert_eq!(animation.tick(&atlas)[0].levels[4].get_pixel(3, 3).0, [10, 0, 0, 255]);
        assert_eq!(animation.ticks(), 6);
    }

    #[test]
    fn repeated_frame_index_in_sequence_is_not_redrawn() {
        let sheet = strip(&[[10, 0, 0, 255], [20, 0, 0, 255]]);
        let tile = animated_tile(
            &sheet,
            &serde_json::json!({"frames": [0, 0, 1]}),
            (0, 0),
        )
        .unwrap();
        let atlas = animated_atlas(vec![tile]);
        let mut animation = AtlasAnimation::new(&atlas);
        let patched = (0..6)
            .map(|_| !animation.tick(&atlas).is_empty())
            .collect::<Vec<_>>();
        assert_eq!(patched, [false, true, true, false, true, true]);
    }

    #[test]
    fn interpolated_animation_patches_every_tick_with_blended_mips() {
        let sheet = strip(&[[0, 0, 0, 255], [200, 100, 0, 255]]);
        let tile = animated_tile(
            &sheet,
            &serde_json::json!({"frametime": 4, "interpolate": true}),
            (0, 64),
        )
        .unwrap();
        let stepped = animated_tile(&sheet, &serde_json::json!({"frametime": 40}), (64, 64)).unwrap();
        let atlas = animated_atlas(vec![tile, stepped]);
        let mut animation = AtlasAnimation::new(&atlas);
        for tick in 1..=8u64 {
            let patches = animation.tick(&atlas);
            assert_eq!(patches.len(), 1, "tick {tick}");
            assert_eq!(patches[0].origin, (0, 64));
            let progress = (tick % 4) as f32 / 4.0;
            let towards = if tick % 8 < 4 { progress } else { 1.0 - progress };
            let red = (200.0 * towards).round() as u8;
            for level in &patches[0].levels {
                assert_eq!(level.get_pixel(0, 0).0[0], red, "tick {tick}");
            }
        }
    }

    #[test]
    fn vanilla_animated_block_textures_tick_in_the_world_atlas() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let atlas = build(&HandcraftedScene::default(), &packs).unwrap().atlas;
        let width = atlas.pixels.width() as f32;
        let tile_of = |name: &str| {
            let [u0, v0, ..] = atlas.region(&ResourceId::parse(&format!("minecraft:block/{name}")).unwrap());
            let origin = ((u0 * width).floor() as u32, (v0 * width).floor() as u32);
            atlas
                .animated_tiles
                .iter()
                .position(|tile| tile.origin == origin)
                .unwrap_or_else(|| panic!("{name} is not animated"))
        };
        let expected = [
            ("fire_0", 32, 1, false),
            ("soul_fire_1", 32, 1, false),
            ("campfire_fire", 8, 2, false),
            ("campfire_log_lit", 4, 20, true),
            ("lava_still", 38, 2, false),
            ("lava_flow", 16, 3, false),
            ("water_still", 32, 2, false),
            ("water_flow", 32, 1, false),
            ("magma", 3, 8, true),
            ("nether_portal", 32, 1, false),
            ("sea_lantern", 5, 5, false),
            ("prismarine", 22, 300, true),
            ("stonecutter_saw", 3, 1, false),
            ("kelp", 20, 2, false),
            ("seagrass", 18, 2, false),
            ("lantern", 3, 8, false),
            ("sculk", 4, 20, true),
            ("command_block_front", 4, 10, true),
        ];
        let mut animation = AtlasAnimation::new(&atlas);
        let mut patched = vec![0; atlas.animated_tiles.len()];
        for _ in 0..40 {
            for patch in animation.tick(&atlas) {
                let index = atlas.animated_tiles.iter().position(|tile| tile.origin == patch.origin).unwrap();
                patched[index] += 1;
                assert_eq!(patch.levels.len(), 5);
            }
        }
        for (name, entries, time, interpolate) in expected {
            let tile = &atlas.animated_tiles[tile_of(name)];
            assert_eq!(tile.sequence.len(), entries, "{name}");
            assert!(tile.sequence.iter().all(|&(_, t)| t == time), "{name}");
            assert_eq!(tile.interpolate, interpolate, "{name}");
        }
        assert_eq!(patched[tile_of("fire_0")], 40);
        assert_eq!(patched[tile_of("water_still")], 20);
        assert_eq!(patched[tile_of("sea_lantern")], 8);
        assert_eq!(patched[tile_of("magma")], 40);
        assert_eq!(patched[tile_of("prismarine")], 40);
    }

    #[test]
    fn atlas_cell_starts_on_the_frame_the_animation_starts_from() {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let textures = root.join("assets/test/textures/block");
        std::fs::create_dir_all(&textures).unwrap();
        let mut sheet = RgbaImage::new(16, 48);
        for (index, pixel) in sheet.pixels_mut().enumerate() {
            let frame = index / (16 * 16);
            *pixel = image::Rgba([(index % 251) as u8, frame as u8 * 90, 40, 255]);
        }
        sheet.save(textures.join("glow.png")).unwrap();
        std::fs::write(
            textures.join("glow.png.mcmeta"),
            r#"{"animation":{"frametime":3,"frames":[2,0,1]}}"#,
        )
        .unwrap();
        sheet.save(textures.join("fade.png")).unwrap();
        std::fs::write(
            textures.join("fade.png.mcmeta"),
            r#"{"animation":{"frametime":5,"interpolate":true}}"#,
        )
        .unwrap();
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let ids = ["test:block/glow", "test:block/fade"].map(|id| ResourceId::parse(id).unwrap());
        let atlas = make_atlas(&packs, ids.to_vec()).unwrap();
        assert_eq!(atlas.animated_tiles.len(), 2);
        let mut levels = vec![atlas.pixels.clone()];
        levels.extend(atlas.mipmaps.iter().cloned());
        let cell = |levels: &[RgbaImage], origin: (u32, u32), level: usize| {
            let size = 64 >> level;
            image::imageops::crop_imm(&levels[level], origin.0 >> level, origin.1 >> level, size, size).to_image()
        };
        for tile in &atlas.animated_tiles {
            for level in 0..5 {
                assert_eq!(cell(&levels, tile.origin, level), *tile.image_at(tile.state_at(0), level));
            }
        }
        let mut animation = AtlasAnimation::new(&atlas);
        for _ in 0..23 {
            for patch in animation.tick(&atlas) {
                for (level, image) in patch.levels.iter().enumerate() {
                    image::imageops::replace(
                        &mut levels[level],
                        image,
                        (patch.origin.0 >> level) as i64,
                        (patch.origin.1 >> level) as i64,
                    );
                }
            }
        }
        for tile in &atlas.animated_tiles {
            for level in 0..5 {
                assert_eq!(cell(&levels, tile.origin, level), *tile.image_at(tile.state_at(23), level));
            }
        }
    }
    #[test]
    fn dropped_item_shadow_projects_on_ground() {
        let shadow = ResourceId::parse("minecraft:misc/shadow").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(32, 32),
            mipmaps: vec![],
            slots: HashMap::from([(shadow, [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let scene = HandcraftedScene::new();
        let mut items = WorldItems::default();
        items.spawn(
            minecraftoss_player::inventory::ItemStack::new("minecraft:stone", 1),
            glam::DVec3::new(0.5, 1.0, 0.5),
        );
        let casters = item_shadow_casters(&items, glam::DVec3::new(0.5, 2.62, 2.5), 1.0);
        let shadow_mesh = entity_shadows(&casters, &scene, &atlas, &|_| 15, 0.0).unwrap();
        assert!(shadow_mesh.faces >= 1);
        assert!(shadow_mesh
            .vertices
            .iter()
            .any(|vertex| vertex.position[1] > 1.0));
        assert!(shadow_mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color[3] > 0.0));
    }
    #[test]
    fn dropped_grass_uses_its_item_tint_face_lighting_and_local_skylight() {
        let texture = ResourceId::parse("minecraft:block/grass_block_top").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let face = |direction: &str, tint_index: Option<usize>| crate::model::Face {
            direction: direction.into(),
            texture: texture.clone(),
            uv: [0.0, 0.0, 1.0, 1.0],
            cull: false,
            cullface: None,
            tint: tint_index.is_some(),
            tint_index,
            force_translucent: false,
            uv_rotation: 0,
        };
        let mut visuals = ItemVisuals::default();
        visuals.models.insert(
            "minecraft:grass_block".into(),
            ItemVisual {
                hand: None,
                model: ResolvedModel {
                    elements: vec![crate::model::Element {
                        from: [0.0; 3],
                        to: [1.0; 3],
                        faces: vec![face("up", Some(0)), face("north", None)],
                        rotation_y: 0,
                        rotation_x: 0,
                        rotation: None,
                        shade_direction_override: None,
                    }],
                },
                ground: Mat4::IDENTITY,
                right_hand: Mat4::IDENTITY,
                left_hand: Mat4::IDENTITY,
                fixed: Mat4::IDENTITY,
                min_y: 0.0,
                tints: vec![[0.3, 0.6, 0.2]],
                flat: false,
            },
        );
        let mut items = WorldItems::default();
        items.spawn(
            minecraftoss_player::inventory::ItemStack::new("minecraft:grass_block", 1),
            glam::DVec3::new(5.5, 6.0, -6.5),
        );
        let sky_light = SkyLight::build(&HandcraftedScene::new());
        let mesh = visuals
            .mesh(
                &items,
                &PackStack::open(vec![]).unwrap(),
                &atlas,
                &sky_light,
                1.0,
            )
            .unwrap();
        assert_eq!(mesh.vertices.len(), 8);
        for vertex in &mesh.vertices[..4] {
            for (actual, expected) in vertex.color[..3].iter().zip([0.3, 0.6, 0.2]) {
                assert!((actual - expected).abs() < 0.0001);
            }
        }
        assert!(mesh.vertices[4].color[0] < 0.95);
        assert!(mesh.vertices.iter().all(|vertex| vertex.sky_light == 13.0));
    }
    #[test]
    fn section_vertices_flag_only_the_glint_ranges() {
        let mut mesh = ChunkMesh {
            vertices: vec![Vertex { position: [0.0; 3], uv: [0.0; 2], color: [1.0; 4], sky_light: 15.0, block_light: 0.0 }; 10],
            ..ChunkMesh::default()
        };
        mesh.glint = vec![2..4, 8..20];
        let vertices = section_vertices(&mesh);
        assert_eq!(vertices.len(), 10);
        let flags: Vec<u8> = vertices.iter().map(|vertex| vertex.pad[0]).collect();
        assert_eq!(flags, [0, 0, GLINT_VERTEX, GLINT_VERTEX, 0, 0, 0, 0, GLINT_VERTEX, GLINT_VERTEX]);
        assert!(vertices.iter().all(|vertex| vertex.pad[1] == 0));
        for (vertex, plain) in vertices.iter().zip(&mesh.vertices) {
            assert_eq!(SectionVertex { pad: [0; 2], ..*vertex }, SectionVertex::from_vertex(plain));
        }
        mesh.glint = vec![12..30, 40..50];
        assert!(section_vertices(&mesh).iter().all(|vertex| vertex.pad[0] == 0));
    }
    #[test]
    fn section_vertices_mark_armour_glint_apart_from_item_glint() {
        let mut mesh = ChunkMesh {
            vertices: vec![Vertex { position: [0.0; 3], uv: [0.0; 2], color: [1.0; 4], sky_light: 15.0, block_light: 0.0 }; 8],
            ..ChunkMesh::default()
        };
        mesh.armor_glint = vec![0..4, 7..20];
        mesh.glint.push(5..6);
        let flags: Vec<u8> = section_vertices(&mesh).iter().map(|vertex| vertex.pad[0]).collect();
        assert_eq!(flags, [ARMOR_GLINT_VERTEX, ARMOR_GLINT_VERTEX, ARMOR_GLINT_VERTEX, ARMOR_GLINT_VERTEX, 0, GLINT_VERTEX, 0, ARMOR_GLINT_VERTEX]);
    }
    #[test]
    fn dropped_enchanted_items_glint_on_every_copy() {
        let texture = ResourceId::parse("minecraft:item/diamond_pickaxe").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let visual = || ItemVisual {
            hand: None,
            model: ResolvedModel {
                elements: vec![crate::model::Element {
                    from: [0.0; 3],
                    to: [1.0, 1.0, 0.0],
                    faces: vec![crate::model::Face {
                        direction: "north".into(),
                        texture: texture.clone(),
                        uv: [0.0, 0.0, 1.0, 1.0],
                        cull: false,
                        cullface: None,
                        tint: false,
                        tint_index: None,
                        force_translucent: false,
                        uv_rotation: 0,
                    }],
                    rotation_y: 0,
                    rotation_x: 0,
                    rotation: None,
                    shade_direction_override: None,
                }],
            },
            ground: Mat4::IDENTITY,
            right_hand: Mat4::IDENTITY,
            left_hand: Mat4::IDENTITY,
            fixed: Mat4::IDENTITY,
            min_y: 0.0,
            tints: vec![],
            flat: true,
        };
        let mut visuals = ItemVisuals::default();
        for id in ["minecraft:diamond_pickaxe", "minecraft:nether_star"] {
            visuals.models.insert(id.into(), visual());
        }
        let stack = |id: &str, count: u8, components: Option<serde_json::Value>| minecraftoss_player::inventory::ItemStack {
            components,
            ..minecraftoss_player::inventory::ItemStack::new(id, count)
        };
        let silk_touch = serde_json::json!({"minecraft:enchantments": {"minecraft:silk_touch": 1}});
        let mut items = WorldItems::default();
        for item in [
            stack("minecraft:diamond_pickaxe", 1, None),
            stack("minecraft:diamond_pickaxe", 1, Some(silk_touch.clone())),
            stack("minecraft:diamond_pickaxe", 20, Some(silk_touch)),
            stack("minecraft:nether_star", 1, None),
        ] {
            items.spawn(item, glam::DVec3::new(0.5, 1.0, 0.5));
        }
        let mesh = visuals
            .mesh(&items, &PackStack::open(vec![]).unwrap(), &atlas, &SkyLight::build(&HandcraftedScene::new()), 1.0)
            .unwrap();
        assert_eq!(mesh.vertices.len(), 4 + 4 + 12 + 4);
        assert_eq!(mesh.glint, [4..8, 8..20, 20..24]);
    }
    #[test]
    #[ignore = "local performance diagnostic with the pinned resource pack"]
    fn local_block_edit_mesh_timing() {
        let Ok(path) = std::env::var("MINECRAFTOSS_PACK") else {
            return;
        };
        if !std::path::Path::new(&path).exists() {
            return;
        }
        let packs = PackStack::open(vec![path.into()]).unwrap();
        let mut scene = HandcraftedScene::new();
        let build = build(&scene, &packs).unwrap();
        scene.set((0, 2, 3), Some(Block::new("minecraft:glass")));
        let tint = BiomeTint::from_pack(&packs).unwrap();
        let mut preview_times = Vec::new();
        let mut times = Vec::new();
        let mut light_times = Vec::new();
        let mut clone_times = Vec::new();
        for _ in 0..5 {
            let preview_start = std::time::Instant::now();
            let preview = block_preview(
                &scene,
                &packs,
                &build.atlas,
                build.sky_light.as_deref().unwrap(),
                &tint,
                (0, 2, 3),
            )
            .unwrap();
            preview_times.push(preview_start.elapsed().as_secs_f64() * 1000.0);
            assert!(preview.faces > 0);
            let clone_start = std::time::Instant::now();
            let _ = scene.clone();
            clone_times.push(clone_start.elapsed().as_secs_f64() * 1000.0);
            let light_start = std::time::Instant::now();
            let _ = SkyLight::build(&scene);
            light_times.push(light_start.elapsed().as_secs_f64() * 1000.0);
            let start = std::time::Instant::now();
            let changed = rebuild_near(&scene, &packs, &build, (0, 2, 3)).unwrap();
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            assert!(!changed.is_empty());
        }
        eprintln!("nearby chunk rebuild ms: {times:?}");
        eprintln!("single block preview ms: {preview_times:?}");
        eprintln!("skylight rebuild ms: {light_times:?}");
        eprintln!("scene dispatch clone ms: {clone_times:?}");
        let near = rebuild_near(&scene, &packs, &build, (0, 2, 3)).unwrap();
        let full = super::build(&scene, &packs).unwrap();
        for (chunk, mesh) in near {
            let canonical = full.chunks.get(&chunk).unwrap();
            assert_eq!(mesh.indices, canonical.indices);
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&mesh.vertices),
                bytemuck::cast_slice::<_, u8>(&canonical.vertices)
            );
        }
    }
    struct Two {
        a: Block,
        b: Block,
    }
    impl Scene for Two {
        fn block(&self, p: BlockPos) -> Option<&Block> {
            match p {
                (15, 0, 0) => Some(&self.a),
                (16, 0, 0) => Some(&self.b),
                _ => None,
            }
        }
        fn chunks(&self) -> Vec<ChunkPos> {
            vec![(0, 0), (1, 0)]
        }
        fn vertical_range(&self) -> std::ops::Range<i32> {
            0..1
        }
        fn revision(&self) -> u64 {
            0
        }
    }
    fn test_box_model(texture: &ResourceId, from: [f32; 3], to: [f32; 3]) -> ResolvedModel {
        ResolvedModel {
            elements: vec![crate::model::Element {
                from,
                to,
                rotation_y: 0,
                rotation_x: 0,
                rotation: None,
                shade_direction_override: None,
                faces: ["down", "up", "north", "south", "west", "east"]
                    .map(|direction| crate::model::Face {
                        direction: direction.into(),
                        texture: texture.clone(),
                        uv: [0.0, 0.0, 1.0, 1.0],
                        cull: true,
                        cullface: None,
                        tint: false,
                        tint_index: None,
                        force_translucent: false,
                        uv_rotation: 0,
                    })
                    .to_vec(),
            }],
        }
    }

    struct Placed(HashMap<BlockPos, Block>);

    impl Scene for Placed {
        fn block(&self, p: BlockPos) -> Option<&Block> {
            self.0.get(&p)
        }
        fn chunks(&self) -> Vec<ChunkPos> {
            vec![(0, 0)]
        }
        fn vertical_range(&self) -> std::ops::Range<i32> {
            0..16
        }
        fn revision(&self) -> u64 {
            0
        }
    }

    #[test]
    fn nether_faces_are_shaded_from_above_and_below_alike() {
        let stone = ResourceId::parse("minecraft:block/stone").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(stone.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let tint = BiomeTint { grass: None, foliage: None, dry_foliage: None };
        let block = Block::new("minecraft:stone");
        let variants = vec![(test_box_model(&stone, [0.0; 3], [1.0; 3]), 1)];
        let scene = Placed(HashMap::from([((8, 6, 8), block.clone())]));
        let light = SkyLight::build(&scene);
        let shades = |nether: bool| {
            set_section_nether_shade(nether);
            let mut mesh = ChunkMesh::default();
            append_block(&scene, (8, 6, 8), &block, || Ok(variants.as_slice()), |_, _| false, &atlas, &tint, &light, &mut mesh, &mut Vec::new()).unwrap();
            mesh.vertices.iter().step_by(4).map(|v| v.color[0]).collect::<Vec<f32>>()
        };
        assert_eq!(shades(false), vec![0.5, 1.0, 0.8, 0.8, 0.6, 0.6], "CardinalLighting.DEFAULT");
        assert_eq!(shades(true), vec![0.9, 0.9, 0.8, 0.8, 0.6, 0.6], "CardinalLighting.NETHER");
        let quads = bake_quads(&block, &variants[0].0, &atlas).unwrap();
        assert_eq!(quads.len(), 6);
        assert_eq!(face_shade("up") * face_shade("north"), 0.9 * 0.8, "Nether lava sides are shaded by up times north");
        set_section_nether_shade(false);
        assert_eq!(face_shade("up") * face_shade("west"), 0.6);
    }

    #[test]
    fn a_moving_block_is_the_placed_block_with_its_variant_shading_and_neighbour_light() {
        let (first, second) = (ResourceId::parse("minecraft:block/sand").unwrap(), ResourceId::parse("minecraft:block/red_sand").unwrap());
        let atlas = Atlas {
            pixels: RgbaImage::new(32, 16),
            mipmaps: vec![],
            slots: HashMap::from([(first.clone(), [0.0, 0.0, 0.5, 1.0]), (second.clone(), [0.5, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let tint = BiomeTint { grass: None, foliage: None, dry_foliage: None };
        let sand = Block::new("minecraft:sand");
        let variants = vec![(test_box_model(&first, [0.0; 3], [1.0; 3]), 1), (test_box_model(&second, [0.0; 3], [1.0; 3]), 1)];
        let floor = (0..16).flat_map(|x| (0..16).map(move |z| ((x, 0, z), Block::new("minecraft:stone"))));
        let cell = (8, 6, 8);
        let mut placed = Placed(floor.collect());
        placed.0.insert(cell, sand.clone());
        let light = SkyLight::build(&placed);
        assert_eq!(light.get(cell), 0, "the placed block's own cell is dark");
        placed.0.remove(&cell);
        let terrain = |at: BlockPos| {
            let mut scene = Placed(placed.0.clone());
            scene.0.insert(at, sand.clone());
            let mut mesh = ChunkMesh::default();
            append_block(&scene, at, &sand, || Ok(variants.as_slice()), |_, _| false, &atlas, &tint, &light, &mut mesh, &mut Vec::new()).unwrap();
            mesh
        };
        let placed_mesh = terrain(cell);
        let mut visuals = ItemVisuals { tint: Some(BiomeTint { grass: None, foliage: None, dry_foliage: None }), ..ItemVisuals::default() };
        visuals.block_models.insert(model_key(&sand), variants.clone());
        let packs = PackStack::open(vec![]).unwrap();
        let mut moving = ChunkMesh::default();
        let falling = MovingBlock { feet: glam::DVec3::new(8.5, 5.96, 8.5), block: sand.clone(), seed: cell };
        visuals.append_moving_blocks(&mut moving, &[falling], &placed, &packs, &atlas, &light).unwrap();
        assert_eq!((moving.vertices.len(), moving.indices.len(), moving.faces), (placed_mesh.vertices.len(), placed_mesh.indices.len(), placed_mesh.faces));
        assert_eq!(moving.indices, placed_mesh.indices);
        for (drawn, block) in moving.vertices.iter().zip(&placed_mesh.vertices) {
            let lowered = [block.position[0], block.position[1] - 0.04, block.position[2]];
            assert!(drawn.position.iter().zip(lowered).all(|(a, b)| (a - b).abs() < 1e-5), "{drawn:?} {block:?}");
            assert_eq!((drawn.uv, drawn.color, drawn.sky_light, drawn.block_light), (block.uv, block.color, block.sky_light, block.block_light));
            assert!(drawn.sky_light >= 14.0, "lit from the open cells around it, not its own dark cell");
        }
        let shades: Vec<f32> = placed_mesh.vertices.iter().step_by(4).map(|v| v.color[0]).collect();
        assert_eq!(shades, vec![0.5, 1.0, 0.8, 0.8, 0.6, 0.6], "terrain face shading");
        let other = (0..64).map(|x| (x, 6, 8)).find(|&at| variant_index(&variants, at) != variant_index(&variants, cell)).unwrap();
        let mut reseeded = ChunkMesh::default();
        let falling = MovingBlock { feet: glam::DVec3::new(8.5, 5.96, 8.5), block: sand.clone(), seed: other };
        visuals.append_moving_blocks(&mut reseeded, &[falling], &placed, &packs, &atlas, &light).unwrap();
        let elsewhere = terrain(other);
        assert!(reseeded.vertices.iter().zip(&elsewhere.vertices).all(|(a, b)| a.uv == b.uv), "the variant follows the seed");
        assert!(reseeded.vertices.iter().zip(&moving.vertices).any(|(a, b)| a.uv != b.uv));
    }

    #[test]
    fn a_shut_lid_drawn_apart_is_its_section_block_and_leaves_the_water_to_the_section() {
        let sheets = ["minecraft:entity/chest/normal", "minecraft:entity/shulker/shulker_red"].map(|id| ResourceId::parse(id).unwrap());
        let atlas = Atlas {
            pixels: RgbaImage::new(128, 64),
            mipmaps: vec![],
            slots: HashMap::from([(sheets[0].clone(), [0.0, 0.0, 0.5, 1.0]), (sheets[1].clone(), [0.5, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let tint = BiomeTint { grass: None, foliage: None, dry_foliage: None };
        let packs = PackStack::open(vec![]).unwrap();
        let cell = (8, 6, 8);
        for (block, wet, faces) in [
            (Block::new("minecraft:chest").with("facing", "east").with("waterlogged", "true"), true, 18),
            (Block::new("minecraft:red_shulker_box").with("facing", "north"), false, 12),
        ] {
            let mut placed = Placed((0..16).flat_map(|x| (0..16).map(move |z| ((x, 0, z), Block::new("minecraft:stone")))).collect());
            placed.0.insert(cell, block.clone());
            placed.0.insert((9, 6, 8), Block::new("minecraft:glowstone"));
            let light = SkyLight::build(&placed);
            let variants = resolve_block_variants(&packs, &block).unwrap();
            let mut whole = ChunkMesh::default();
            let mut whole_transparent = Vec::new();
            append_block(&placed, cell, &block, || Ok(variants.as_slice()), |_, _| false, &atlas, &tint, &light, &mut whole, &mut whole_transparent).unwrap();
            let mut visuals = ItemVisuals { tint: Some(BiomeTint { grass: None, foliage: None, dry_foliage: None }), ..ItemVisuals::default() };
            let mut lid = ChunkMesh::default();
            let posed = crate::lids::PosedLid { cell, block: block.clone(), progress: 0.0 };
            visuals.append_open_lids(&mut lid, &[posed], &placed, &packs, &atlas, &light).unwrap();
            let mut left = ChunkMesh::default();
            let mut left_transparent = Vec::new();
            append_baked(&placed, cell, &[], TintKind::of(&block), wet, |_, _| false, |_| false, &atlas, &tint, &light, &mut left, &mut left_transparent).unwrap();
            let n = lid.vertices.len();
            assert_eq!(lid.faces, faces);
            assert_eq!((lid.faces + left.faces, n + left.vertices.len()), (whole.faces, whole.vertices.len()));
            assert_eq!(whole.vertices[..n], lid.vertices[..], "{}", block.id.path);
            assert_eq!(whole.vertices[n..], left.vertices[..]);
            assert_eq!(whole.indices, lid.indices.iter().copied().chain(left.indices.iter().map(|i| i + n as u32)).collect::<Vec<_>>());
            assert_eq!(whole_transparent, left_transparent.iter().map(|i| i + n as u32).collect::<Vec<_>>());
            assert_eq!(wet, left.faces > 0, "only a waterlogged chest leaves water behind");
            let mut open = ChunkMesh::default();
            let posed = crate::lids::PosedLid { cell, block: block.clone(), progress: 1.0 };
            visuals.append_open_lids(&mut open, &[posed], &placed, &packs, &atlas, &light).unwrap();
            assert_eq!(open.faces, lid.faces);
            assert!(open.vertices.iter().zip(&lid.vertices).any(|(a, b)| a.position != b.position), "{}", block.id.path);
        }
    }

    #[test]
    fn partial_neighbors_keep_the_full_block_face() {
        let texture = ResourceId::parse("minecraft:block/stone").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let tint = BiomeTint {
            grass: None,
            foliage: None,
            dry_foliage: None,
        };
        for (neighbor_id, neighbor_model, expected_faces) in [
            (
                "minecraft:oak_slab",
                test_box_model(&texture, [0.0; 3], [1.0, 0.5, 1.0]),
                6,
            ),
            (
                "minecraft:oak_stairs",
                test_box_model(&texture, [0.0; 3], [1.0, 0.5, 1.0]),
                6,
            ),
            (
                "minecraft:oak_slab",
                test_box_model(&texture, [0.0; 3], [1.0; 3]),
                5,
            ),
        ] {
            let scene = Two {
                a: Block::new("minecraft:stone"),
                b: Block::new(neighbor_id),
            };
            let models = HashMap::from([
                (
                    model_key(&scene.a),
                    vec![(test_box_model(&texture, [0.0; 3], [1.0; 3]), 1)],
                ),
                (model_key(&scene.b), vec![(neighbor_model, 1)]),
            ]);
            let light = SkyLight::build(&scene);
            assert_eq!(
                mesh_chunk(&scene, (0, 0), &models, &atlas, &tint, &light)
                    .unwrap()
                    .faces,
                expected_faces,
                "neighbor {neighbor_id}"
            );
        }
    }

    #[test]
    fn water_faces_use_the_translucent_range_and_hide_shared_faces() {
        let texture = ResourceId::parse("minecraft:block/water_still").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let tint = BiomeTint {
            grass: None,
            foliage: None,
            dry_foliage: None,
        };
        let water_model = test_box_model(&texture, [0.0; 3], [1.0, 8.0 / 9.0, 1.0]);
        let solid_model = test_box_model(&texture, [0.0; 3], [1.0; 3]);
        for neighbor in ["minecraft:water", "minecraft:stone"] {
            let scene = Two {
                a: Block::new("minecraft:water"),
                b: Block::new(neighbor),
            };
            let models = HashMap::from([
                (model_key(&scene.a), vec![(water_model.clone(), 1)]),
                (
                    model_key(&scene.b),
                    vec![(
                        if neighbor == "minecraft:water" {
                            water_model.clone()
                        } else {
                            solid_model.clone()
                        },
                        1,
                    )],
                ),
            ]);
            let light = SkyLight::build(&scene);
            let water = mesh_chunk(&scene, (0, 0), &models, &atlas, &tint, &light).unwrap();
            assert_eq!(water.faces, 5);
            assert_eq!(water.transparent_start, Some(0));
            assert_eq!(water.indices.len(), (water.faces + 4) * 6);
            if neighbor == "minecraft:stone" {
                let solid = mesh_chunk(&scene, (1, 0), &models, &atlas, &tint, &light).unwrap();
                assert_eq!(solid.faces, 6);
                assert_eq!(solid.transparent_start, None);
            }
        }
    }
    #[test]
    fn water_shows_its_faces_through_ice() {
        let texture = ResourceId::parse("minecraft:block/water_still").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let tint = BiomeTint {
            grass: None,
            foliage: None,
            dry_foliage: None,
        };
        for (neighbor, faces) in [("minecraft:ice", 6), ("minecraft:stone", 5)] {
            let scene = Two {
                a: Block::new("minecraft:water"),
                b: Block::new(neighbor),
            };
            let models = HashMap::from([(model_key(&scene.b), vec![(test_box_model(&texture, [0.0; 3], [1.0; 3]), 1)])]);
            let light = SkyLight::build(&scene);
            assert_eq!(mesh_chunk(&scene, (0, 0), &models, &atlas, &tint, &light).unwrap().faces, faces, "{neighbor}");
        }
    }
    #[test]
    fn forced_translucent_model_face_uses_blended_range() {
        let texture = ResourceId::parse("minecraft:block/redstone_dust_dot").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let mut model = test_box_model(&texture, [0.0; 3], [1.0; 3]);
        model.elements[0].faces[0].force_translucent = true;
        let mut scene = HandcraftedScene::default();
        scene.set((0, 0, 0), Some(Block::new("minecraft:redstone_wire")));
        let block = scene.block((0, 0, 0)).unwrap();
        let models = HashMap::from([(model_key(block), vec![(model, 1)])]);
        let mesh = mesh_chunk(
            &scene,
            (0, 0),
            &models,
            &atlas,
            &BiomeTint {
                grass: None,
                foliage: None,
                dry_foliage: None,
            },
            &SkyLight::build(&scene),
        )
        .unwrap();
        assert_eq!(mesh.transparent_start, Some(30));
        assert_eq!(mesh.indices.len(), 36);
    }
    #[test]
    fn faces_over_translucent_sprite_pixels_use_the_blended_range() {
        let texture = ResourceId::parse("minecraft:block/nether_portal").unwrap();
        let mut pixels = RgbaImage::from_pixel(4, 4, image::Rgba([90, 10, 200, 255]));
        pixels.put_pixel(0, 0, image::Rgba([90, 10, 200, 160]));
        pixels.put_pixel(3, 3, image::Rgba([0, 0, 0, 0]));
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::from([(texture.clone(), TranslucentPixels::of(&pixels, None).unwrap())]),
        };
        let mut scene = HandcraftedScene::default();
        scene.set((0, 0, 0), Some(Block::new("minecraft:nether_portal").with("axis", "x")));
        let block = scene.block((0, 0, 0)).unwrap();
        let tint = BiomeTint {
            grass: None,
            foliage: None,
            dry_foliage: None,
        };
        let whole = test_box_model(&texture, [0.0; 3], [1.0; 3]);
        let mut corner = whole.clone();
        for face in &mut corner.elements[0].faces {
            face.uv = [0.5, 1.0, 1.0, 0.5];
        }
        for (model, start) in [(whole, Some(0)), (corner, None)] {
            let models = HashMap::from([(model_key(block), vec![(model, 1)])]);
            let mesh = mesh_chunk(&scene, (0, 0), &models, &atlas, &tint, &SkyLight::build(&scene)).unwrap();
            assert_eq!(mesh.transparent_start, start);
            assert_eq!(mesh.indices.len(), 36);
        }
    }
    #[test]
    fn inner_translucent_elements_bake_before_the_shell_around_them() {
        let texture = ResourceId::parse("minecraft:block/honey_block_side").unwrap();
        let pixels = RgbaImage::from_pixel(4, 4, image::Rgba([250, 170, 30, 190]));
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::from([(texture.clone(), TranslucentPixels::of(&pixels, None).unwrap())]),
        };
        let mut model = test_box_model(&texture, [0.0; 3], [1.0; 3]);
        model.elements.extend(test_box_model(&texture, [1.0 / 16.0; 3], [15.0 / 16.0; 3]).elements);
        let quads = bake_quads(&Block::new("minecraft:honey_block"), &model, &atlas).unwrap();
        assert_eq!(quads.len(), 12);
        assert!(quads.iter().all(|quad| quad.transparent));
        let inner = |quad: &BakedQuad| quad.corners.iter().flatten().all(|c| (1.0 / 16.0..=15.0 / 16.0).contains(c));
        assert!(quads[..6].iter().all(inner));
        assert!(!quads[6..].iter().any(inner));
    }
    #[test]
    fn sprite_translucency_spans_the_frames_the_animation_shows() {
        let mut strip = RgbaImage::from_pixel(2, 6, image::Rgba([1, 2, 3, 255]));
        strip.put_pixel(1, 3, image::Rgba([1, 2, 3, 100]));
        assert!(TranslucentPixels::of(&strip, None).unwrap().within([0.5, 0.5, 1.0, 0.6]));
        let all = TranslucentPixels::of(&strip, Some(&serde_json::json!({}))).unwrap();
        assert!(all.within([0.5, 0.5, 1.0, 1.0]));
        assert!(!all.within([0.0, 0.0, 0.5, 1.0]));
        assert!(TranslucentPixels::of(&strip, Some(&serde_json::json!({"frames": [0, {"index": 2, "time": 4}]}))).is_none());
        assert!(TranslucentPixels::of(&strip, Some(&serde_json::json!({"frames": [1]}))).is_some());
        assert!(TranslucentPixels::of(&RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 0])), None).is_none());
    }
    #[test]
    fn flowing_surface_height_and_lava_light_follow_fluid_state() {
        let mut scene = HandcraftedScene::default();
        scene.set(
            (0, 0, 0),
            Some(Block::new("minecraft:water").with("level", "4")),
        );
        scene.set(
            (2, 0, 0),
            Some(Block::new("minecraft:lava").with("level", "0")),
        );
        let ids = ["water_still", "water_flow", "lava_still", "lava_flow"];
        let slots = ids
            .into_iter()
            .map(|name| {
                (
                    ResourceId::parse(&format!("minecraft:block/{name}")).unwrap(),
                    [0.0, 0.0, 1.0, 1.0],
                )
            })
            .collect();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots,
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let tint = BiomeTint {
            grass: None,
            foliage: None,
            dry_foliage: None,
        };
        let light = SkyLight::build(&scene);
        let mut water = ChunkMesh::default();
        let mut transparent = Vec::new();
        append_fluid(
            &scene,
            (0, 0, 0),
            &atlas,
            &tint,
            &light,
            &mut water,
            &mut transparent,
        )
        .unwrap();
        assert!(!transparent.is_empty());
        assert_eq!(transparent.len(), (water.faces + 5) * 6);
        assert!(water.vertices.iter().all(|v| v.position[1] <= 4.0 / 9.0));
        let mut lava = ChunkMesh::default();
        let mut transparent = Vec::new();
        append_fluid(
            &scene,
            (2, 0, 0),
            &atlas,
            &tint,
            &light,
            &mut lava,
            &mut transparent,
        )
        .unwrap();
        assert!(transparent.is_empty());
        assert!(lava.vertices.iter().all(|v| v.block_light >= 14.0));
        assert!(lava.vertices.iter().any(|v| v.block_light == 15.0));
        assert_eq!(lava.indices.len(), lava.faces * 12);
        for quad in lava.indices.chunks(12) {
            assert_eq!([quad[6], quad[7], quad[8]], [quad[0], quad[2], quad[1]]);
        }
    }
    fn fluid_test_atlas() -> Atlas {
        let slots = [
            ("minecraft:block/water_still", [0.0, 0.0, 0.25, 0.25]),
            ("minecraft:block/water_flow", [0.25, 0.0, 0.5, 0.25]),
            ("minecraft:block/water_overlay", [0.5, 0.0, 0.75, 0.25]),
            ("minecraft:block/lava_still", [0.0, 0.5, 0.25, 0.75]),
            ("minecraft:block/lava_flow", [0.25, 0.5, 0.5, 0.75]),
            ("test:block/plant", [0.75, 0.75, 1.0, 1.0]),
        ]
        .into_iter()
        .map(|(id, slot)| (ResourceId::parse(id).unwrap(), slot))
        .collect();
        Atlas { pixels: RgbaImage::new(16, 16), mipmaps: vec![], slots, missing: vec![], animated: vec![], animated_tiles: vec![], translucent: HashMap::new() }
    }
    fn untinted() -> BiomeTint {
        BiomeTint { grass: None, foliage: None, dry_foliage: None }
    }
    fn water_faces(mesh: &ChunkMesh, transparent: &[u32]) -> (Vec<[[f32; 3]; 4]>, usize) {
        let mut fronts = Vec::new();
        let mut backs = 0;
        for six in transparent.chunks(6) {
            if six[1] == six[0] + 1 {
                let start = six[0] as usize;
                fronts.push(std::array::from_fn(|i| mesh.vertices[start + i].position));
            } else {
                assert_eq!(six, [six[0], six[0] + 2, six[0] + 1, six[0], six[0] + 3, six[0] + 2]);
                backs += 1;
            }
        }
        (fronts, backs)
    }
    fn kelp_pool() -> (Placed, HashMap<String, Vec<(ResolvedModel, u32)>>) {
        let plant = ResourceId::parse("test:block/plant").unwrap();
        let mut pool = Placed(HashMap::new());
        for x in 0..3 {
            for y in 1..3 {
                for z in 0..3 {
                    pool.0.insert((x, y, z), Block::new("minecraft:water").with("level", "0"));
                }
            }
        }
        let plants = [((1, 1, 1), Block::new("minecraft:kelp_plant")), ((1, 2, 1), Block::new("minecraft:kelp").with("age", "3")), ((0, 1, 0), Block::new("minecraft:seagrass")), ((2, 1, 2), Block::new("minecraft:bubble_column").with("drag", "true"))];
        let mut models = HashMap::new();
        for (pos, block) in plants {
            models.insert(model_key(&block), vec![(test_box_model(&plant, [0.4, 0.0, 0.4], [0.6, 1.0, 0.6]), 1)]);
            pool.0.insert(pos, block);
        }
        (pool, models)
    }

    #[test]
    fn water_and_the_plants_that_hold_it_share_no_faces() {
        let (pool, models) = kelp_pool();
        let mesh = mesh_chunk(&pool, (0, 0), &models, &fluid_test_atlas(), &untinted(), &SkyLight::build(&pool)).unwrap();
        let start = mesh.transparent_start.unwrap() as usize;
        assert_eq!(start, 4 * 6 * 6, "the four plants keep their own faces");
        let (quads, _) = water_faces(&mesh, &mesh.indices[start..]);
        let on = |axis: usize, plane: f32| quads.iter().filter(|quad| quad.iter().all(|v| (v[axis] - plane).abs() < 0.01)).count();
        for plane in [1.0, 2.0] {
            assert_eq!(on(0, plane), 0, "no water face between columns at x {plane}");
            assert_eq!(on(2, plane), 0, "no water face between columns at z {plane}");
        }
        assert_eq!(on(1, 2.0), 0, "no surface inside the kelp column or the pool");
        let tops: Vec<_> = quads.iter().filter(|quad| quad.iter().all(|v| v[1] > 2.5)).collect();
        assert_eq!(tops.len(), 9, "every column has its surface, the kelp's too");
        let surface = 2.0 + 8.0 / 9.0 - 0.001;
        for v in tops.iter().flat_map(|quad| quad.iter()) {
            if (1.0..=2.0).contains(&v[0]) && (1.0..=2.0).contains(&v[2]) {
                assert!((v[1] - surface).abs() < 1e-5, "the surface stays level over the kelp: {v:?}");
            }
        }
    }

    #[test]
    fn water_is_as_opaque_as_vanilla() {
        let (pool, models) = kelp_pool();
        let mesh = mesh_chunk(&pool, (0, 0), &models, &fluid_test_atlas(), &untinted(), &SkyLight::build(&pool)).unwrap();
        let start = mesh.transparent_start.unwrap() as usize;
        let water: Vec<&Vertex> = mesh.indices[start..].iter().map(|&i| &mesh.vertices[i as usize]).collect();
        assert!(!water.is_empty());
        for v in water {
            assert_eq!(v.color[3], 1.0);
            assert_eq!(SectionVertex::from_vertex(v).color[3], 255);
        }
    }

    #[test]
    fn the_water_surface_has_a_back_only_where_vanilla_draws_one() {
        let atlas = fluid_test_atlas();
        let water_at = (0, 1, 0);
        let faces = |others: &[(BlockPos, &str)]| {
            let mut scene = Placed(HashMap::from([(water_at, Block::new("minecraft:water").with("level", "0"))]));
            for &(pos, id) in others {
                scene.0.insert(pos, Block::new(id));
            }
            let mut mesh = ChunkMesh::default();
            let mut transparent = Vec::new();
            append_fluid(&scene, water_at, &atlas, &untinted(), &SkyLight::build(&scene), &mut mesh, &mut transparent).unwrap();
            let (fronts, backs) = water_faces(&mesh, &transparent);
            (fronts, backs, mesh)
        };
        let (fronts, backs, _) = faces(&[]);
        assert_eq!((fronts.len(), backs), (6, 5), "under open sky the top and the four sides have backs, the bottom none");
        let roof: Vec<(BlockPos, &str)> = (-1..=1).flat_map(|x| (-1..=1).map(move |z| ((x, 2, z), "minecraft:stone"))).collect();
        let (fronts, backs, _) = faces(&roof);
        assert_eq!((fronts.len(), backs), (6, 4), "a full roof over the 3x3 above hides the surface's back");
        let (fronts, backs, _) = faces(&roof[..8]);
        assert_eq!((fronts.len(), backs), (6, 5), "one open cell diagonally above shows the surface's back");
        let (fronts, backs, mesh) = faces(&[((1, 1, 0), "minecraft:glass")]);
        assert_eq!((fronts.len(), backs), (6, 4), "the face against glass has no back");
        let overlay = atlas.region(&ResourceId::parse("minecraft:block/water_overlay").unwrap());
        let against_glass: Vec<&Vertex> = mesh.vertices.iter().filter(|v| (v.position[0] - 0.999).abs() < 1e-5).collect();
        assert_eq!(against_glass.len(), 4);
        assert!(against_glass.iter().all(|v| (overlay[0]..=overlay[2]).contains(&v.uv[0])), "the face against glass shows the overlay");
        let (fronts, backs, _) = faces(&[((1, 1, 0), "minecraft:oak_leaves")]);
        assert_eq!((fronts.len(), backs), (6, 4), "the face against leaves has no back");
    }

    #[test]
    fn a_waterlogged_block_draws_its_model_and_its_water() {
        use std::fs;
        let atlas = fluid_test_atlas();
        let plant = ResourceId::parse("test:block/plant").unwrap();
        let slab = Block::new("test:slab").with("waterlogged", "true");
        let at = (0, 1, 0);
        let mut scene = HandcraftedScene::default();
        scene.set(at, Some(slab.clone()));
        let light = SkyLight::build(&scene);
        let variants = vec![(test_box_model(&plant, [0.0; 3], [1.0, 0.5, 1.0]), 1)];
        let mut mesh = ChunkMesh::default();
        let mut transparent = Vec::new();
        append_block(&scene, at, &slab, || Ok(variants.as_slice()), |_, _| false, &atlas, &untinted(), &light, &mut mesh, &mut transparent).unwrap();
        assert_eq!(mesh.indices.len(), 36, "the slab's own faces");
        assert!(!transparent.is_empty(), "and its water");
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("pack.mcmeta"), r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#).unwrap();
        let states = root.join("assets/test/blockstates/slab.json");
        fs::create_dir_all(states.parent().unwrap()).unwrap();
        fs::write(states, r#"{"variants":{"":{"model":"test:block/slab"}}}"#).unwrap();
        let model = root.join("assets/test/models/block/slab.json");
        fs::create_dir_all(model.parent().unwrap()).unwrap();
        fs::write(model, r##"{"textures":{"all":"test:block/plant"},"elements":[{"from":[0,0,0],"to":[16,8,16],"faces":{"down":{"texture":"#all"},"up":{"texture":"#all"},"north":{"texture":"#all"},"south":{"texture":"#all"},"west":{"texture":"#all"},"east":{"texture":"#all"}}}]}"##).unwrap();
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let preview = block_preview(&scene, &packs, &atlas, &light, &untinted(), at).unwrap();
        assert_eq!(preview.transparent_start, Some(36), "the preview shows the slab before its water");
        assert_eq!(preview.indices.len(), 36 + transparent.len());
    }
    #[test]
    fn boundary_neighbors_hide_shared_face() {
        let id = ResourceId::parse("minecraft:block/stone").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(32, 32),
            mipmaps: vec![],
            slots: HashMap::from([(id.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let model = ResolvedModel {
            elements: vec![crate::model::Element {
                from: [0.0; 3],
                to: [1.0; 3],
                rotation_y: 0,
                rotation_x: 0,
                rotation: None,
                shade_direction_override: None,
                faces: ["down", "up", "north", "south", "west", "east"]
                    .map(|d| crate::model::Face {
                        direction: d.into(),
                        texture: id.clone(),
                        uv: [0.0, 0.0, 1.0, 1.0],
                        cull: true,
                        cullface: None,
                        tint: false,
                        tint_index: None,
                        force_translucent: false,
                        uv_rotation: 0,
                    })
                    .to_vec(),
            }],
        };
        let scene = Two {
            a: Block::new("minecraft:stone"),
            b: Block::new("minecraft:stone"),
        };
        let models = HashMap::from([(model_key(&scene.a), vec![(model, 1)])]);
        let light = SkyLight::build(&scene);
        assert_eq!(
            mesh_chunk(
                &scene,
                (0, 0),
                &models,
                &atlas,
                &BiomeTint {
                    grass: None,
                    foliage: None,
                    dry_foliage: None
                },
                &light
            )
            .unwrap()
            .faces,
            5
        );
        assert_eq!(
            mesh_chunk(
                &scene,
                (1, 0),
                &models,
                &atlas,
                &BiomeTint {
                    grass: None,
                    foliage: None,
                    dry_foliage: None
                },
                &light
            )
            .unwrap()
            .faces,
            5
        );
    }

    #[test]
    fn glass_hides_shared_faces_while_fancy_leaves_keep_them() {
        let glass = Block::new("minecraft:glass");
        let leaves = Block::new("minecraft:oak_leaves");
        assert!(hides_shared_face(&glass, &glass));
        assert!(!hides_shared_face(&leaves, &leaves));
        assert!(!hides_shared_face(
            &glass,
            &Block::new("minecraft:tinted_glass")
        ));
    }

    #[test]
    fn face_corners_blend_neighbor_shade() {
        let mut scene = HandcraftedScene::new();
        scene.set((-1, 1, -1), Some(Block::new("minecraft:stone")));
        let light = SkyLight::build(&scene);
        let shaded = ambient_vertex(&scene, &light, (0, 1, 0), "up", [0.0, 1.0, 0.0]);
        let open = ambient_vertex(&scene, &light, (0, 1, 0), "up", [1.0, 1.0, 1.0]);
        assert!(shaded.0 < open.0);
    }

    #[test]
    fn cube_vertices_follow_minecraft_faceinfo_order() {
        assert_eq!(
            corners("up", [0.0; 3], [1.0; 3]).unwrap()[0],
            [0.0, 1.0, 0.0]
        );
        assert_eq!(
            corners("down", [0.0; 3], [1.0; 3]).unwrap()[0],
            [0.0, 0.0, 1.0]
        );
        assert_eq!(
            corners("north", [0.0; 3], [1.0; 3]).unwrap()[0],
            [1.0, 1.0, 0.0]
        );
        assert_eq!(
            corners("south", [0.0; 3], [1.0; 3]).unwrap()[0],
            [0.0, 1.0, 1.0]
        );
        assert_eq!(
            corners("west", [0.0; 3], [1.0; 3]).unwrap()[0],
            [0.0, 1.0, 0.0]
        );
        assert_eq!(
            corners("east", [0.0; 3], [1.0; 3]).unwrap()[0],
            [1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn position_seed_selects_grass_rotations() {
        let variants = (0..4)
            .map(|_| (ResolvedModel { elements: vec![] }, 1))
            .collect::<Vec<_>>();
        assert_eq!(variant_index(&variants, (0, 1, 0)), 2);
        assert_eq!(variant_index(&variants, (0, 1, 1)), 3);
        assert_eq!(variant_index(&variants, (-1, 1, 0)), 1);
        assert_eq!(variant_index(&variants, (2_000_000_000, 1, 0)), 1);
        assert_eq!(rotate_y([0.0, 1.0, 0.0], 90), [1.0, 1.0, 0.0]);
        assert_eq!(rotated_direction("north", 90).unwrap(), "east");
    }

    fn turned_element(from: [f32; 3], to: [f32; 3], rotation_x: u16, rotation_y: u16) -> crate::model::Element {
        crate::model::Element { from, to, faces: Vec::new(), rotation_y, rotation_x, rotation: None, shade_direction_override: None }
    }

    #[test]
    fn blockstate_x_rotation_turns_geometry_and_cull_directions_before_y() {
        assert_eq!(rotate_x([0.5, 1.0, 0.5], 90), [0.5, 0.5, 0.0]);
        assert_eq!(rotate_x([0.5, 1.0, 0.5], 270), [0.5, 0.5, 1.0]);
        assert_eq!(rotate_x([0.5, 1.0, 0.25], 180), [0.5, 0.0, 0.75]);
        let log_along_x = turned_element([0.0; 3], [1.0; 3], 90, 90);
        assert_eq!(turned_direction("up", &log_along_x).unwrap(), "east");
        assert_eq!(turned_direction("down", &log_along_x).unwrap(), "west");
        assert_eq!(turned_direction("north", &log_along_x).unwrap(), "down");
        let cube_tipped_north = turned_element([0.0; 3], [1.0; 3], 90, 0);
        assert_eq!(turned_direction("up", &cube_tipped_north).unwrap(), "north");
        assert_eq!(turned_direction("east", &cube_tipped_north).unwrap(), "east");
        for (rotation_x, up_becomes) in [(0, "up"), (90, "north"), (180, "down"), (270, "south")] {
            let element = turned_element([0.0; 3], [1.0; 3], rotation_x, 0);
            let corners = element_corners(&element, "up").unwrap();
            assert_eq!(quad_direction(&element, "up", &corners).unwrap(), up_becomes);
            let normal = quad_normal(&corners).unwrap();
            assert!((normal - face_normal(up_becomes).unwrap()).length() < 1e-5, "x{rotation_x}: {normal}");
            let (linear, _) = crate::special_models::element_affine(&element);
            assert!((linear * Vec3::Y - normal).length() < 1e-5, "x{rotation_x}");
        }
        let upside_down_slab = turned_element([0.0; 3], [1.0, 0.5, 1.0], 180, 0);
        let bottom = element_corners(&upside_down_slab, "down").unwrap();
        assert!(bottom.iter().all(|corner| corner[1] == 1.0), "{bottom:?}");
        assert_eq!(quad_direction(&upside_down_slab, "down", &bottom).unwrap(), "up");
        assert!(face_cubic("up", &bottom), "the slab's bottom becomes the block's top");
        assert!(element_corners(&upside_down_slab, "up").unwrap().iter().all(|corner| corner[1] == 0.5));
    }

    #[test]
    fn a_ceiling_vine_sheet_lies_flat_under_the_block_above() {
        let sheet = turned_element([0.0, 0.0, 0.8 / 16.0], [1.0, 1.0, 0.8 / 16.0], 270, 0);
        let outer = element_corners(&sheet, "north").unwrap();
        assert!(outer.iter().all(|corner| (corner[1] - (15.2 / 16.0 + SHEET_FACE_OFFSET)).abs() < 1e-6), "{outer:?}");
        assert_eq!(quad_direction(&sheet, "north", &outer).unwrap(), "up");
        let inner = element_corners(&sheet, "south").unwrap();
        assert!(inner.iter().all(|corner| (corner[1] - (15.2 / 16.0 - SHEET_FACE_OFFSET)).abs() < 1e-6), "{inner:?}");
        assert_eq!(quad_direction(&sheet, "south", &inner).unwrap(), "down");
        let (low, high) = outer.iter().fold(([1.0f32; 3], [0.0f32; 3]), |(low, high), c| (std::array::from_fn(|k| low[k].min(c[k])), std::array::from_fn(|k| high[k].max(c[k]))));
        assert_eq!((low[0], low[2], high[0], high[2]), (0.0, 0.0, 1.0, 1.0), "it covers the whole ceiling");
        let east_sheet = turned_element([0.0, 0.0, 0.8 / 16.0], [1.0, 1.0, 0.8 / 16.0], 0, 90);
        assert!(element_corners(&east_sheet, "north").unwrap().iter().all(|corner| (corner[0] - (15.2 / 16.0 + SHEET_FACE_OFFSET)).abs() < 1e-6));
    }

    #[test]
    fn turned_cubes_cull_and_light_toward_the_side_each_face_was_turned_to() {
        let atlas = Atlas { pixels: RgbaImage::new(16, 16), mipmaps: Vec::new(), slots: HashMap::new(), missing: Vec::new(), animated: Vec::new(), animated_tiles: Vec::new(), translucent: HashMap::new() };
        for (rotation_x, rotation_y) in [(90, 0), (90, 90), (180, 0), (180, 270), (270, 180)] {
            let mut cube = crumble_element([0.0; 3], [1.0; 3], &["down", "up", "north", "south", "west", "east"], rotation_y, None);
            cube.rotation_x = rotation_x;
            for face in &mut cube.faces {
                face.cull = true;
            }
            let quads = bake_quads(&crate::scene::Block::new("minecraft:oak_log"), &ResolvedModel { elements: vec![cube] }, &atlas).unwrap();
            assert_eq!(quads.len(), 6);
            for quad in quads {
                let normal = quad_normal(&quad.corners).unwrap().round();
                let cull = quad.cull.unwrap();
                assert_eq!((cull.0 as f32, cull.1 as f32, cull.2 as f32), (normal.x, normal.y, normal.z), "x{rotation_x} y{rotation_y}");
                assert_eq!(quad.light_offset, cull);
            }
        }
    }

    fn crumble_texture() -> ResourceId {
        ResourceId::parse("minecraft:block/test_crumble").unwrap()
    }

    fn crumble_element(from: [f32; 3], to: [f32; 3], directions: &[&str], rotation_y: u16, rotation: Option<&str>) -> crate::model::Element {
        crate::model::Element {
            from,
            to,
            rotation_y,
            rotation_x: 0,
            rotation: rotation.and_then(|raw| crate::model::ElementRotation::parse(&serde_json::from_str(raw).unwrap()).unwrap()),
            shade_direction_override: None,
            faces: directions
                .iter()
                .map(|direction| crate::model::Face {
                    direction: (*direction).into(),
                    texture: crumble_texture(),
                    uv: [0.0, 0.0, 1.0, 1.0],
                    cull: false,
                    cullface: None,
                    tint: false,
                    tint_index: None,
                    force_translucent: false,
                    uv_rotation: 0,
                })
                .collect(),
        }
    }

    fn ladder_sheet() -> crate::model::Element {
        crumble_element([0.0, 0.0, 15.2 / 16.0], [1.0, 1.0, 15.2 / 16.0], &["north", "south"], 0, None)
    }

    fn terrain_positions(variants: &[(ResolvedModel, u32)], pos: BlockPos) -> Vec<[u32; 3]> {
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(crumble_texture(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
            translucent: HashMap::new(),
        };
        let block = Block::new("minecraft:test_crumble");
        let scene = Placed(HashMap::from([(pos, block.clone())]));
        let light = SkyLight::build(&scene);
        let mut mesh = ChunkMesh::default();
        append_block(&scene, pos, &block, || Ok(variants), |_, _| false, &atlas, &BiomeTint { grass: None, foliage: None, dry_foliage: None }, &light, &mut mesh, &mut Vec::new()).unwrap();
        mesh.vertices.iter().map(|vertex| vertex.position.map(f32::to_bits)).collect()
    }

    fn crack_positions(variants: &[(ResolvedModel, u32)], pos: BlockPos) -> Vec<[u32; 3]> {
        crumbling_quads(variants, pos).unwrap().iter().flatten().map(|corner| [corner[0].to_bits(), corner[1].to_bits(), corner[2].to_bits()]).collect()
    }

    #[test]
    fn a_ladder_cracks_on_its_sheet_not_on_a_cube() {
        let variants = vec![(ResolvedModel { elements: vec![ladder_sheet()] }, 1)];
        let quads = crumbling_quads(&variants, (3, 64, -2)).unwrap();
        assert_eq!(quads.len(), 2);
        for corner in quads.iter().flatten() {
            assert!((corner[2] - (-2.0 + 15.2 / 16.0)).abs() <= 0.002 + 1e-5, "{corner:?}");
            assert!((3.0..=4.0).contains(&corner[0]) && (64.0..=65.0).contains(&corner[1]), "{corner:?}");
            assert!(corner[..3].iter().zip([3.0, 64.0, -2.0]).all(|(value, base)| (value - (base - 0.002)).abs() > 1e-4 && (value - (base + 1.002)).abs() > 1e-4), "{corner:?}");
        }
        assert!(crumbling_quads(&[], (0, 0, 0)).unwrap().is_empty());
        assert!(crumbling_quads(&[(ResolvedModel { elements: vec![] }, 1)], (0, 0, 0)).unwrap().is_empty());
    }

    #[test]
    fn crack_corners_are_the_terrain_corners_bit_for_bit() {
        let cross = crumble_element([0.8 / 16.0, 0.0, 0.5], [15.2 / 16.0, 1.0, 0.5], &["north", "south"], 0, Some(r#"{"rotation":{"origin":[8,8,8],"axis":"y","angle":45,"rescale":true}}"#));
        let turned = crumble_element([0.0, 0.0, 0.0], [1.0, 0.5, 0.5], &["down", "up", "north", "south", "west", "east"], 90, None);
        let torch = crumble_element([7.0 / 16.0, 0.0, 7.0 / 16.0], [9.0 / 16.0, 10.0 / 16.0, 9.0 / 16.0], &["up", "north", "south", "west", "east"], 270, Some(r#"{"rotation":{"origin":[0,3.5,8],"axis":"z","angle":-22.5}}"#));
        for elements in [vec![ladder_sheet()], vec![cross.clone()], vec![turned.clone()], vec![torch.clone()], vec![ladder_sheet(), cross, turned, torch]] {
            let variants = vec![(ResolvedModel { elements }, 1)];
            let pos = (3, 5, 2);
            let crack = crack_positions(&variants, pos);
            assert!(!crack.is_empty());
            assert_eq!(crack, terrain_positions(&variants, pos));
        }
    }

    #[test]
    fn crack_uvs_are_vanillas_sheeted_decal_projection() {
        use std::f32::consts::{FRAC_PI_2, PI};
        let cube = crumble_element([0.0; 3], [1.0; 3], &["down", "up", "north", "south", "west", "east"], 0, None);
        let quads = crumbling_quads(&[(ResolvedModel { elements: vec![cube] }, 1)], (0, 0, 0)).unwrap();
        let uvs = |quad: &[[f32; 5]; 4]| quad.map(|corner| [corner[3], corner[4]]);
        assert_eq!(uvs(&quads[3]), [[0.0, -1.0], [0.0, 0.0], [1.0, 0.0], [1.0, -1.0]], "south");
        assert_eq!(uvs(&quads[2]), [[-1.0, -1.0], [-1.0, 0.0], [0.0, 0.0], [0.0, -1.0]], "north");
        for (quad, direction) in quads.iter().zip(["down", "up", "north", "south", "west", "east"]) {
            let facing = match direction {
                "down" => glam::Quat::from_rotation_x(PI),
                "up" => glam::Quat::IDENTITY,
                "north" => glam::Quat::from_rotation_x(FRAC_PI_2) * glam::Quat::from_rotation_z(PI),
                "south" => glam::Quat::from_rotation_x(FRAC_PI_2),
                "west" => glam::Quat::from_rotation_x(FRAC_PI_2) * glam::Quat::from_rotation_z(FRAC_PI_2),
                _ => glam::Quat::from_rotation_x(FRAC_PI_2) * glam::Quat::from_rotation_z(-FRAC_PI_2),
            };
            for corner in quad {
                let turned = facing * (glam::Quat::from_rotation_x(-FRAC_PI_2) * (glam::Quat::from_rotation_y(PI) * Vec3::new(corner[0], corner[1], corner[2])));
                assert!((corner[3] + turned.x).abs() < 1e-5 && (corner[4] + turned.y).abs() < 1e-5, "{direction} {corner:?} {turned}");
            }
        }
    }

    #[test]
    fn crack_takes_the_terrain_variant_at_each_position() {
        let variants = vec![
            (ResolvedModel { elements: vec![crumble_element([0.0; 3], [1.0; 3], &["down", "up", "north", "south", "west", "east"], 0, None)] }, 1),
            (ResolvedModel { elements: vec![ladder_sheet()] }, 1),
        ];
        let mut seen = [false; 2];
        for x in 0..16 {
            let pos = (x, 3, 7);
            seen[variant_for(&variants, pos)] = true;
            assert_eq!(crack_positions(&variants, pos), terrain_positions(&variants, pos), "{pos:?}");
            assert_eq!(crumbling_quads(&variants, pos).unwrap().len(), [6, 2][variant_for(&variants, pos)]);
        }
        assert_eq!(seen, [true, true]);
    }

    #[test]
    fn a_vine_cracks_only_on_its_attached_planes() {
        let north = crumble_element([0.0, 0.0, 0.8 / 16.0], [1.0, 1.0, 0.8 / 16.0], &["north", "south"], 0, None);
        let east = crumble_element([0.0, 0.0, 0.8 / 16.0], [1.0, 1.0, 0.8 / 16.0], &["north", "south"], 90, None);
        let quads = crumbling_quads(&[(ResolvedModel { elements: vec![north, east] }, 1)], (-5, 70, 9)).unwrap();
        assert_eq!(quads.len(), 4);
        for corner in quads[..2].iter().flatten() {
            assert!(corner[2] - 9.0 <= 0.05 + 0.002 + 1e-5 && corner[2] >= 9.0, "{corner:?}");
        }
        for corner in quads[2..].iter().flatten() {
            assert!(-4.0 - corner[0] <= 0.05 + 0.002 + 1e-5 && corner[0] <= -4.0, "{corner:?}");
        }
    }
}
