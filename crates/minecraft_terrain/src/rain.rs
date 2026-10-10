use crate::ambient_particles::Kind;
use crate::display_ticks::{AmbientSound, DisplayTick};
use crate::fluid::{FluidCell, FluidKind};
use crate::scene::{Block, BlockPos, HandcraftedScene, Scene};
use glam::DVec3;
use minecraftoss_core::block::flags;
use minecraftoss_generator::temperature::BiomeTemperature;
use minecraftoss_player::{Block as PlayerBlock, collision_shape_boxes, rng::LegacyRandom};

pub const RAIN_SOUND: &str = "minecraft:weather.rain";
pub const RAIN_ABOVE_SOUND: &str = "minecraft:weather.rain.above";
pub const WEATHER_RADIUS: i32 = 10;
pub const OVERWORLD_SEA_LEVEL: i32 = 63;
const PARTICLE_DENSITY: f32 = 0.225;
const HEIGHT_REACH: i32 = 10;
const WARM_ENOUGH_TO_RAIN: f32 = 0.15;

pub struct RainWorld<'a> {
    pub height: &'a dyn Fn(i32, i32) -> i32,
    pub min_y: i32,
    pub rains_at: &'a dyn Fn(BlockPos) -> bool,
}

pub fn shape_top(block: Option<&Block>, x: f64, z: f64) -> f64 {
    let Some(block) = block else {
        return 0.0;
    };
    let shape = PlayerBlock {
        id: block.id.key(),
        properties: block.properties.clone(),
    };
    collision_shape_boxes(&shape)
        .into_iter()
        .filter(|(min, max)| min[0] <= x && x <= max[0] && min[2] <= z && z <= max[2])
        .map(|(_, max)| max[1])
        .fold(0.0, f64::max)
}

pub fn surface<S: Scene + ?Sized>(scene: &S, pos: BlockPos, x: f64, z: f64) -> f64 {
    let fluid = scene.fluid_at(pos).map_or(0.0, |cell| {
        let above = scene.fluid_at((pos.0, pos.1 + 1, pos.2));
        if above.is_some_and(|above| above.kind == cell.kind) {
            1.0
        } else {
            f64::from(cell.amount) / 9.0
        }
    });
    shape_top(scene.block(pos), x, z).max(fluid)
}

fn smokes(block: Option<&Block>, fluid: Option<FluidCell>) -> bool {
    fluid.is_some_and(|cell| cell.kind == FluidKind::Lava)
        || block.is_some_and(|block| {
            block.id.namespace == "minecraft" && block.id.path == "magma_block"
                || crate::campfire::is_lit(block)
        })
}

pub fn rain_tick<S: Scene + ?Sized>(
    scene: &S,
    world: &RainWorld,
    camera: DVec3,
    rain_level: f32,
    random: &mut LegacyRandom,
    sound_time: &mut i32,
) -> DisplayTick {
    let mut out = DisplayTick::default();
    if rain_level <= 0.0 {
        return out;
    }
    let cam = (
        camera.x.floor() as i32,
        camera.y.floor() as i32,
        camera.z.floor() as i32,
    );
    let size = WEATHER_RADIUS * 2 + 1;
    let count = (PARTICLE_DENSITY * (size * size) as f32 * rain_level * rain_level) as i32;
    let mut sound_at = None;
    for _ in 0..count {
        let dx = random.next_int(size as u32) as i32 - WEATHER_RADIUS;
        let dz = random.next_int(size as u32) as i32 - WEATHER_RADIUS;
        let (x, z) = (cam.0 + dx, cam.2 + dz);
        let y = (world.height)(x, z);
        if y <= world.min_y || y > cam.1 + HEIGHT_REACH || y < cam.1 - HEIGHT_REACH {
            continue;
        }
        if !(world.rains_at)((x, y, z)) {
            continue;
        }
        let below = (x, y - 1, z);
        sound_at = Some(below);
        let (rx, rz) = (random.next_double(), random.next_double());
        let block = scene.block(below);
        let fluid = scene.fluid_at(below);
        let top = surface(scene, below, rx, rz);
        let kind = if smokes(block, fluid) {
            Kind::Smoke
        } else {
            Kind::Rain
        };
        let at = DVec3::new(
            f64::from(x) + rx,
            f64::from(below.1) + top,
            f64::from(z) + rz,
        );
        out.particles.push((kind, at, DVec3::ZERO));
    }
    if let Some(at) = sound_at {
        let due = (random.next_int(3) as i32) < *sound_time;
        *sound_time += 1;
        if due {
            *sound_time = 0;
            let overhead = (world.height)(cam.0, cam.2) > camera.y.floor() as i32;
            let (event, volume, pitch) = if at.1 > cam.1 + 1 && overhead {
                (RAIN_ABOVE_SOUND, 0.1, 0.5)
            } else {
                (RAIN_SOUND, 0.2, 1.0)
            };
            out.sounds.push(AmbientSound {
                event,
                at: [
                    f64::from(at.0) + 0.5,
                    f64::from(at.1) + 0.5,
                    f64::from(at.2) + 0.5,
                ],
                volume,
                pitch,
            });
        }
    }
    out
}

pub fn blocks_motion(scene: &HandcraftedScene, block: &Block) -> bool {
    if FluidCell::held_by(block).is_some() {
        return true;
    }
    let ignored = block.id.namespace == "minecraft"
        && matches!(block.id.path.as_str(), "cobweb" | "bamboo_sapling");
    !ignored
        && scene.states().is_some_and(|states| {
            states
                .state_of(block)
                .is_some_and(|state| states.registries().blocks.is(state, flags::LEGACY_SOLID))
        })
}

pub fn motion_blocking_height(scene: &HandcraftedScene, x: i32, z: i32) -> i32 {
    scene
        .column_top(x, z, |block| blocks_motion(scene, block))
        .map_or(scene.vertical_range().start, |y| y + 1)
}

pub fn rains_at(
    scene: &HandcraftedScene,
    temperature: &BiomeTemperature,
    (x, y, z): BlockPos,
) -> bool {
    let Some(states) = scene.states() else {
        return false;
    };
    let Some(biome) = scene.noise_biome((x >> 2, y >> 2, z >> 2)) else {
        return false;
    };
    let info = states.registries().biomes.get(biome);
    info.has_precipitation
        && temperature.at(
            info.temperature,
            info.frozen_temperature_modifier,
            x,
            y,
            z,
            OVERWORLD_SEA_LEVEL,
        ) >= WARM_ENOUGH_TO_RAIN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::ChunkPos;
    use std::collections::HashMap;

    struct Blocks(HashMap<BlockPos, Block>);

    impl Scene for Blocks {
        fn block(&self, pos: BlockPos) -> Option<&Block> {
            self.0.get(&pos)
        }

        fn chunks(&self) -> Vec<ChunkPos> {
            Vec::new()
        }

        fn vertical_range(&self) -> std::ops::Range<i32> {
            -64..320
        }

        fn revision(&self) -> u64 {
            0
        }
    }

    fn floor_at(y: i32) -> Blocks {
        let mut blocks = HashMap::new();
        for x in -12..=12 {
            for z in -12..=12 {
                blocks.insert((x, y, z), Block::new("minecraft:stone"));
            }
        }
        Blocks(blocks)
    }

    fn tick(scene: &Blocks, ground: i32, rain: f32, seed: u64, raining: bool) -> DisplayTick {
        let height = move |_: i32, _: i32| ground + 1;
        let rains = move |_: BlockPos| raining;
        let world = RainWorld {
            height: &height,
            min_y: -64,
            rains_at: &rains,
        };
        let mut time = 0;
        rain_tick(
            scene,
            &world,
            DVec3::new(0.5, 65.0, 0.5),
            rain,
            &mut LegacyRandom::new(seed),
            &mut time,
        )
    }

    #[test]
    fn full_rain_drops_ninety_nine_splashes_on_the_ground_around_the_camera() {
        let scene = floor_at(63);
        let splashes = tick(&scene, 63, 1.0, 1, true).particles;
        assert_eq!(splashes.len(), 99);
        for (kind, at, motion) in splashes {
            assert_eq!((kind, motion), (Kind::Rain, DVec3::ZERO));
            assert_eq!(at.y, 64.0);
            assert!((-10.0..11.0).contains(&at.x) && (-10.0..11.0).contains(&at.z));
        }
        assert_eq!(tick(&scene, 63, 0.5, 1, true).particles.len(), 24);
        assert!(tick(&scene, 63, 0.0, 1, true).particles.is_empty());
    }

    #[test]
    fn no_splashes_on_snow_or_ground_far_above_or_below() {
        let scene = floor_at(63);
        assert!(tick(&scene, 63, 1.0, 2, false).particles.is_empty());
        let deep = floor_at(40);
        assert!(tick(&deep, 40, 1.0, 2, true).particles.is_empty());
        let high = floor_at(80);
        assert!(tick(&high, 80, 1.0, 2, true).particles.is_empty());
    }

    #[test]
    fn rain_on_lava_magma_or_a_lit_campfire_smokes_and_slabs_catch_it_halfway() {
        let mut blocks = HashMap::new();
        for x in -12..=12 {
            for z in -12..=12 {
                blocks.insert((x, 63, z), Block::new("minecraft:lava"));
            }
        }
        let lava = tick(&Blocks(blocks), 63, 1.0, 3, true);
        assert!(
            lava.particles
                .iter()
                .all(|(kind, _, _)| *kind == Kind::Smoke)
        );
        assert!(
            lava.particles
                .iter()
                .all(|(_, at, _)| (at.y - (63.0 + 8.0 / 9.0)).abs() < 1e-6)
        );
        let mut slabs = HashMap::new();
        for x in -12..=12 {
            for z in -12..=12 {
                slabs.insert(
                    (x, 63, z),
                    Block::new("minecraft:stone_slab").with("type", "bottom"),
                );
            }
        }
        let caught = tick(&Blocks(slabs), 63, 1.0, 3, true);
        assert!(
            caught
                .particles
                .iter()
                .all(|(kind, at, _)| *kind == Kind::Rain && at.y == 63.5)
        );
    }

    #[test]
    fn the_rain_sound_comes_every_few_ticks_from_the_last_splash_spot() {
        let scene = floor_at(63);
        let height = |_: i32, _: i32| 64;
        let rains = |_: BlockPos| true;
        let world = RainWorld {
            height: &height,
            min_y: -64,
            rains_at: &rains,
        };
        let mut random = LegacyRandom::new(4);
        let mut time = 0;
        let mut heard = Vec::new();
        for _ in 0..300 {
            let out = rain_tick(
                &scene,
                &world,
                DVec3::new(0.5, 65.0, 0.5),
                1.0,
                &mut random,
                &mut time,
            );
            heard.extend(out.sounds);
        }
        assert!((80..200).contains(&heard.len()), "{}", heard.len());
        for sound in &heard {
            assert_eq!(
                (sound.event, sound.volume, sound.pitch),
                (RAIN_SOUND, 0.2, 1.0)
            );
            assert_eq!(sound.at[1], 63.5);
        }
    }

    #[test]
    fn rain_heard_on_a_roof_above_the_camera_is_muffled() {
        let scene = floor_at(70);
        let height = |_: i32, _: i32| 71;
        let rains = |_: BlockPos| true;
        let world = RainWorld {
            height: &height,
            min_y: -64,
            rains_at: &rains,
        };
        let mut random = LegacyRandom::new(5);
        let mut time = 0;
        let mut heard = Vec::new();
        for _ in 0..100 {
            let out = rain_tick(
                &scene,
                &world,
                DVec3::new(0.5, 65.0, 0.5),
                1.0,
                &mut random,
                &mut time,
            );
            heard.extend(out.sounds);
        }
        assert!(!heard.is_empty());
        assert!(
            heard
                .iter()
                .all(|sound| (sound.event, sound.volume, sound.pitch)
                    == (RAIN_ABOVE_SOUND, 0.1, 0.5))
        );
    }

    #[test]
    fn the_heightmap_finds_the_top_motion_blocking_block() {
        let mut scene = HandcraftedScene::default();
        scene.set(
            (3, 70, 3),
            Some(Block::new("minecraft:water").with("level", "0")),
        );
        scene.set((3, 60, 3), Some(Block::new("minecraft:cobweb")));
        assert_eq!(motion_blocking_height(&scene, 3, 3), 71);
        assert_eq!(
            motion_blocking_height(&scene, 5, 5),
            scene.vertical_range().start
        );
    }
}
