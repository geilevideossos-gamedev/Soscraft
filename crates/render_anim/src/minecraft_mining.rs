//! Breaking the Minecraft world with MW2 weapons. Bullets mine the block they
//! strike the way a tool does in survival: each adds progress in proportion
//! to the damage the bullet would do and in inverse proportion to the
//! block's hardness, the destroy stages show it, and at full
//! progress the block breaks with MinecraftOSS's break particles.
use std::collections::HashMap;

use minecraft_terrain::block_particles::BlockParticles;
use minecraft_terrain::mesh::{Atlas, ChunkMesh, SectionVertex};
use minecraft_terrain::model::ResolvedModel;
use minecraft_terrain::pack::{PackStack, ResourceId};
use minecraft_terrain::scene::{HandcraftedScene, Scene};
use minecraft_terrain::terrain::{BlockStates, TerrainStream};
use minecraftoss_core::BlockStateId;
use minecraftoss_core::registries::Registries;

type BlockPos = (i32, i32, i32);

/// Bullet damage that mines a block of hardness 1 in four shots: a 40-damage
/// bullet (an ACR up close) breaks dirt, of hardness 0.5, on the second.
const DAMAGE_PER_HARDNESS: f32 = sim::voxel::DAMAGE_PER_HARDNESS;
/// Progress on a block no longer being shot is forgotten after this long.
const PROGRESS_SECONDS: f64 = 6.0;
const TICK_SECONDS: f64 = 1.0 / 20.0;

pub(crate) struct Mining {
    /// Progress in 0..1 of each block being shot, and when it was last hit.
    progress: HashMap<BlockPos, (f32, f64)>,
    particles: Option<BlockParticles>,
    tick_clock: f64,
    models: HashMap<BlockStateId, Option<Vec<(ResolvedModel, u32)>>>,
}

impl Default for Mining {
    fn default() -> Self {
        Self {
            progress: HashMap::new(),
            particles: None,
            tick_clock: 0.0,
            models: HashMap::new(),
        }
    }
}

/// The world pieces mining reads and edits.
pub(crate) struct WorldRefs<'a> {
    pub stream: &'a mut TerrainStream,
    pub scene: &'a mut HandcraftedScene,
    pub packs: &'a PackStack,
    pub atlas: &'a Atlas,
    pub registries: &'a Registries,
}

impl Mining {
    /// Applies this tick's shots and explosions; the blocks they broke, each
    /// with the power of the blast that broke it.
    pub(crate) fn apply(
        &mut self,
        events: Vec<sim::voxel::VoxelEvent>,
        world: &mut WorldRefs<'_>,
        now: f64,
    ) -> Vec<(BlockPos, minecraft_terrain::scene::Block, Option<f32>)> {
        if self.particles.is_none() {
            self.particles = BlockParticles::new(world.packs).ok();
        }
        let mut broken: Vec<(BlockPos, Option<f32>)> = Vec::new();
        // Blocks broken by this batch: the rest of a shotgun's pellets into
        // one pass through, rather than starting a crack on a block that is
        // about to go.
        let mut gone = std::collections::HashSet::new();
        for event in events {
            match event {
                sim::voxel::VoxelEvent::Shot { block, damage } => {
                    let pos = (block[0], block[1], block[2]);
                    if gone.contains(&pos) {
                        continue;
                    }
                    let Some(hardness) = hardness(world, pos) else {
                        continue;
                    };
                    let entry = self.progress.entry(pos).or_insert((0.0, now));
                    entry.0 += if hardness <= 0.0 { 1.0 } else { damage / DAMAGE_PER_HARDNESS / hardness };
                    entry.1 = now;
                    if entry.0 >= 1.0 {
                        self.progress.remove(&pos);
                        if let Some(block) = Scene::block(&*world.scene, pos).cloned()
                            && let Some(particles) = self.particles.as_mut()
                        {
                            let _ = particles.spawn(world.packs, &*world.scene, pos, &block, world.atlas);
                        }
                        broken.push((pos, None));
                        gone.insert(pos);
                    }
                }
                sim::voxel::VoxelEvent::Ray { from, to, damage } => {
                    // Blocks without collision the bullet passed through.
                    for pos in blocks_on_segment(from, to) {
                        if gone.contains(&pos) || !passable(world, pos) {
                            continue;
                        }
                        let Some(hardness) = hardness(world, pos) else {
                            continue;
                        };
                        let entry = self.progress.entry(pos).or_insert((0.0, now));
                        entry.0 += if hardness <= 0.0 { 1.0 } else { damage / DAMAGE_PER_HARDNESS / hardness };
                        entry.1 = now;
                        if entry.0 >= 1.0 {
                            self.progress.remove(&pos);
                            if let Some(block) = Scene::block(&*world.scene, pos).cloned()
                                && let Some(particles) = self.particles.as_mut()
                            {
                                let _ = particles.spawn(world.packs, &*world.scene, pos, &block, world.atlas);
                            }
                            broken.push((pos, None));
                            gone.insert(pos);
                        }
                    }
                }
                sim::voxel::VoxelEvent::Explosion { .. } | sim::voxel::VoxelEvent::MobShot { .. } | sim::voxel::VoxelEvent::MobBlast { .. } => {}
            }
        }
        // No crack outlives its block, however it went.
        self.progress
            .retain(|pos, (_, at)| now - *at < PROGRESS_SECONDS && !gone.contains(pos) && Scene::block(&*world.scene, *pos).is_some());
        remove_broken(broken, world)
    }

    pub(crate) fn break_now(&mut self, positions: &[BlockPos], world: &mut WorldRefs<'_>) -> Vec<(BlockPos, minecraft_terrain::scene::Block, Option<f32>)> {
        if self.particles.is_none() {
            self.particles = BlockParticles::new(world.packs).ok();
        }
        let breakable = breakable_now(&*world.scene, positions);
        for &pos in &breakable {
            self.progress.remove(&pos);
            if let Some(block) = Scene::block(&*world.scene, pos).cloned()
                && let Some(particles) = self.particles.as_mut()
            {
                let _ = particles.spawn(world.packs, &*world.scene, pos, &block, world.atlas);
            }
        }
        remove_broken(breakable.into_iter().map(|pos| (pos, None)).collect(), world)
    }

    pub(crate) fn burst(&mut self, packs: &PackStack, scene: &HandcraftedScene, atlas: &Atlas, pos: BlockPos, block: &minecraft_terrain::scene::Block) {
        if self.particles.is_none() {
            self.particles = BlockParticles::new(packs).ok();
        }
        if let Some(particles) = self.particles.as_mut() {
            let _ = particles.spawn(packs, scene, pos, block, atlas);
        }
    }

    pub(crate) fn eat_burst(&mut self, packs: &PackStack, eye: glam::Vec3, yaw: f32, pitch: f32, count: u32, texture: &ResourceId) {
        if self.particles.is_none() {
            self.particles = BlockParticles::new(packs).ok();
        }
        if let Some(particles) = self.particles.as_mut() {
            particles.spawn_eating(eye, yaw, pitch, count, texture);
        }
    }

    pub(crate) fn item_crumbs(&mut self, packs: &PackStack, at: glam::Vec3, velocities: &[glam::Vec3], texture: &ResourceId) {
        if self.particles.is_none() {
            self.particles = BlockParticles::new(packs).ok();
        }
        if let Some(particles) = self.particles.as_mut() {
            particles.spawn_item_crumbs(at, velocities, texture);
        }
    }

    pub(crate) fn potion_crumbs(&mut self, packs: &PackStack, center: glam::Vec3, texture: &ResourceId) {
        if self.particles.is_none() {
            self.particles = BlockParticles::new(packs).ok();
        }
        if let Some(particles) = self.particles.as_mut() {
            particles.spawn_potion_crumbs(center, texture);
        }
    }

    /// Steps the break particles at 20 ticks a second.
    pub(crate) fn tick(&mut self, scene: &HandcraftedScene, dt: f64) -> f32 {
        self.tick_clock += dt;
        while self.tick_clock >= TICK_SECONDS {
            self.tick_clock -= TICK_SECONDS;
            if let Some(particles) = self.particles.as_mut() {
                particles.tick(scene);
            }
        }
        (self.tick_clock / TICK_SECONDS) as f32
    }

    /// The break particles as section vertices, facing `forward`.
    pub(crate) fn particle_mesh(
        &self,
        atlas: &Atlas,
        forward: glam::Vec3,
        partial: f32,
        light: &minecraft_terrain::lighting::SkyLight,
    ) -> (Vec<u8>, Vec<u32>) {
        let Some(particles) = self.particles.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let mut mesh = ChunkMesh::default();
        particles.append_mesh(&mut mesh, atlas, forward, partial, light);
        let vertices: Vec<SectionVertex> = mesh.vertices.iter().map(SectionVertex::from_vertex).collect();
        (bytemuck::cast_slice(&vertices).to_vec(), mesh.indices)
    }

    pub(crate) fn crack_mesh(&mut self, scene: &HandcraftedScene, states: &BlockStates, packs: &PackStack) -> (Vec<[f32; 6]>, Vec<u32>) {
        let (mut vertices, mut indices) = (Vec::new(), Vec::new());
        for (&pos, &(progress, _)) in &self.progress {
            let Some(block) = Scene::block(scene, pos) else {
                continue;
            };
            if matches!(block.id.path.as_str(), "water" | "lava") {
                continue;
            }
            let Some(state) = states.state_of(block) else {
                continue;
            };
            let variants = self
                .models
                .entry(state)
                .or_insert_with(|| states.block(state).and_then(|block| minecraft_terrain::model::resolve_block_variants(packs, block).ok()));
            let Some(quads) = variants.as_deref().and_then(|variants| minecraft_terrain::mesh::crumbling_quads(variants, pos).ok()) else {
                continue;
            };
            let stage = ((progress * 10.0) as i32).clamp(0, 9) as f32;
            for quad in quads {
                let first = vertices.len() as u32;
                vertices.extend(quad.map(|[x, y, z, u, v]| [x, y, z, u, v, stage]));
                indices.extend_from_slice(&[first, first + 1, first + 2, first, first + 2, first + 3]);
            }
        }
        (vertices, indices)
    }
}

fn remove_broken(mut broken: Vec<(BlockPos, Option<f32>)>, world: &mut WorldRefs<'_>) -> Vec<(BlockPos, minecraft_terrain::scene::Block, Option<f32>)> {
    if broken.is_empty() {
        return Vec::new();
    }
    broken.sort_unstable_by_key(|(pos, _)| *pos);
    broken.dedup_by_key(|(pos, _)| *pos);
    let mut out = Vec::with_capacity(broken.len());
    for &(pos, blast) in &broken {
        if let Some(block) = Scene::block(&*world.scene, pos).cloned() {
            out.push((pos, block, blast));
        }
        world.scene.set(pos, None);
        sim::voxel::set_block_shape(pos.0, pos.1, pos.2, 0);
    }
    let positions: Vec<BlockPos> = broken.iter().map(|(pos, _)| *pos).collect();
    world.stream.record_edits(world.scene, &positions);
    world.stream.mark_edited(world.scene, &positions);
    out
}

fn breakable_now(scene: &impl Scene, positions: &[BlockPos]) -> Vec<BlockPos> {
    let mut breakable: Vec<BlockPos> = positions
        .iter()
        .copied()
        .filter(|&pos| Scene::block(scene, pos).is_some_and(|block| !matches!(block.id.path.as_str(), "water" | "lava" | "air" | "cave_air" | "void_air")))
        .collect();
    breakable.sort_unstable();
    breakable.dedup();
    breakable
}

/// The destroy speed of a breakable block, or `None` for air, fluids and
/// unbreakable blocks.
/// A block bullets pass through: one with no collision shape.
fn passable(world: &WorldRefs<'_>, pos: BlockPos) -> bool {
    Scene::block(&*world.scene, pos)
        .and_then(|block| world.stream.states.state_of(block))
        .is_some_and(|state| world.registries.blocks.collision_boxes(state).is_empty())
}

/// The blocks a segment passes through, in order (Amanatides–Woo).
fn blocks_on_segment(from: [f64; 3], to: [f64; 3]) -> Vec<BlockPos> {
    let d: [f64; 3] = std::array::from_fn(|k| to[k] - from[k]);
    let mut cell: [i32; 3] = std::array::from_fn(|k| from[k].floor() as i32);
    let end: [i32; 3] = std::array::from_fn(|k| to[k].floor() as i32);
    let step: [i32; 3] = std::array::from_fn(|k| if d[k] > 0.0 { 1 } else { -1 });
    let delta: [f64; 3] = std::array::from_fn(|k| if d[k] == 0.0 { f64::INFINITY } else { 1.0 / d[k].abs() });
    let mut next: [f64; 3] = std::array::from_fn(|k| {
        if d[k] == 0.0 {
            f64::INFINITY
        } else if d[k] > 0.0 {
            (f64::from(cell[k]) + 1.0 - from[k]) * delta[k]
        } else {
            (from[k] - f64::from(cell[k])) * delta[k]
        }
    });
    let mut out = vec![(cell[0], cell[1], cell[2])];
    // A shot reaches a few hundred blocks at most.
    while cell != end && out.len() < 512 {
        let k = if next[0] <= next[1] && next[0] <= next[2] {
            0
        } else if next[1] <= next[2] {
            1
        } else {
            2
        };
        if next[k] > 1.0 {
            break;
        }
        cell[k] += step[k];
        next[k] += delta[k];
        out.push((cell[0], cell[1], cell[2]));
    }
    out
}

fn hardness(world: &WorldRefs<'_>, pos: BlockPos) -> Option<f32> {
    let block = Scene::block(&*world.scene, pos)?;
    if matches!(block.id.path.as_str(), "water" | "lava" | "air" | "cave_air" | "void_air") {
        return None;
    }
    let state = world.stream.states.state_of(block)?;
    let speed = world.registries.blocks.state(state).destroy_speed;
    (speed >= 0.0).then_some(speed)
}

/// The ten destroy stages side by side, 16 pixels each.
pub(crate) fn crack_strip(packs: &PackStack) -> anyhow::Result<image::RgbaImage> {
    let mut strip = image::RgbaImage::new(16 * 10, 16);
    for stage in 0..10u32 {
        let id = ResourceId::parse(&format!("minecraft:block/destroy_stage_{stage}"))?;
        if let Some(bytes) = packs.texture(&id)? {
            let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
            let tile = image::imageops::resize(&img, 16, 16, image::imageops::FilterType::Nearest);
            image::imageops::replace(&mut strip, &tile, i64::from(stage) * 16, 0);
        }
    }
    Ok(strip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::scene::Block;

    #[test]
    fn a_creative_break_takes_any_block_even_bedrock_but_no_air_or_fluid() {
        let mut scene = HandcraftedScene::new();
        let (stone, bedrock, water, lava, cave_air, empty) = ((0, 64, 0), (1, 64, 0), (2, 64, 0), (3, 64, 0), (4, 64, 0), (5, 64, 0));
        scene.set(stone, Some(Block::new("minecraft:stone")));
        scene.set(bedrock, Some(Block::new("minecraft:bedrock")));
        scene.set(water, Some(Block::new("minecraft:water")));
        scene.set(lava, Some(Block::new("minecraft:lava")));
        scene.set(cave_air, Some(Block::new("minecraft:cave_air")));
        assert_eq!(breakable_now(&scene, &[empty, bedrock, water, stone, lava, cave_air, stone]), vec![stone, bedrock]);
        assert!(breakable_now(&scene, &[]).is_empty());
    }

    #[test]
    fn real_blocks_crack_on_their_own_model_shapes() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let states = std::sync::Arc::new(BlockStates::new(std::sync::Arc::new(registries), 0, -64, 384).unwrap());
        let packs = PackStack::open(vec![pack]).unwrap();
        let mut scene = HandcraftedScene::streamed(states.clone());
        let blocks = [
            Some(Block::new("minecraft:ladder").with("facing", "north")),
            Some(Block::new("minecraft:vine").with("north", "true")),
            Some(Block::new("minecraft:torch")),
            Some(Block::new("minecraft:oak_slab").with("type", "bottom")),
            Some(Block::new("minecraft:poppy")),
            Some(Block::new("minecraft:stone")),
            Some(Block::new("minecraft:water")),
            None,
        ];
        let mut mining = Mining::default();
        for (stage, block) in blocks.into_iter().enumerate() {
            let pos = (stage as i32 * 2, 100, 0);
            scene.set(pos, block);
            mining.progress.insert(pos, (0.05 + stage as f32 * 0.1, 0.0));
        }
        let (vertices, indices) = mining.crack_mesh(&scene, &states, &packs);
        assert_eq!(indices.len() * 4, vertices.len() * 6);
        let local = |stage: usize| -> Vec<[f32; 3]> {
            vertices.iter().filter(|vertex| vertex[5] == stage as f32).map(|vertex| [vertex[0] - stage as f32 * 2.0, vertex[1] - 100.0, vertex[2]]).collect()
        };
        assert!(vertices.iter().all(|vertex| (0.0..=9.0).contains(&vertex[5])));
        let ladder = local(0);
        assert!(ladder.len() >= 8 && ladder.iter().all(|p| (p[2] - 15.2 / 16.0).abs() <= 0.0021), "{ladder:?}");
        let vine = local(1);
        assert!(vine.len() >= 8 && vine.iter().all(|p| (p[2] - 0.8 / 16.0).abs() <= 0.0021), "{vine:?}");
        let torch = local(2);
        let inside = |value: f32| (7.0 / 16.0 - 1e-4..=9.0 / 16.0 + 1e-4).contains(&value);
        assert!(torch.len() == 24 && torch.iter().all(|p| inside(p[0]) && inside(p[2]) && p[1] <= 10.0 / 16.0 + 1e-5), "{torch:?}");
        let slab = local(3);
        assert!(slab.len() == 24 && slab.iter().all(|p| p[1] <= 0.5 + 1e-5), "{slab:?}");
        let poppy = local(4);
        assert!(poppy.len() == 16 && poppy.iter().all(|p| ((p[0] - 0.5).abs() - (p[2] - 0.5).abs()).abs() < 1e-3), "{poppy:?}");
        let stone = local(5);
        assert!(stone.len() == 24 && stone.iter().flatten().all(|&value| value == 0.0 || value == 1.0), "{stone:?}");
        assert!(local(6).is_empty() && local(7).is_empty());
    }
}
