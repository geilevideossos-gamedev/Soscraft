use crate::scene::{BlockPos, Scene};
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const PORTAL_AMBIENT: &str = "minecraft:block.portal.ambient";
const SAMPLES: usize = 667;
const RADII: [u32; 2] = [16, 32];
const PORTAL_PARTICLES: usize = 4;
const AMBIENT_CHANCE: u32 = 100;
const AMBIENT_VOLUME: f32 = 0.5;
const AMBIENT_PITCH: f32 = 0.8;
const AMBIENT_PITCH_SPREAD: f32 = 0.4;

#[derive(Clone, Debug, PartialEq)]
pub struct AmbientSound {
    pub event: &'static str,
    pub at: [f64; 3],
    pub volume: f32,
    pub pitch: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayTick {
    pub portal_particles: Vec<(DVec3, DVec3)>,
    pub sounds: Vec<AmbientSound>,
}

pub fn animate_tick<S: Scene + ?Sized>(
    scene: &S,
    (x, y, z): BlockPos,
    random: &mut LegacyRandom,
) -> DisplayTick {
    let mut out = DisplayTick::default();
    for _ in 0..SAMPLES {
        for radius in RADII {
            let dx = spread(random, radius);
            let dy = spread(random, radius);
            let dz = spread(random, radius);
            animate_block(scene, (x + dx, y + dy, z + dz), random, &mut out);
        }
    }
    out
}

fn spread(random: &mut LegacyRandom, radius: u32) -> i32 {
    let ahead = random.next_int(radius) as i32;
    ahead - random.next_int(radius) as i32
}

fn is_portal<S: Scene + ?Sized>(scene: &S, pos: BlockPos) -> bool {
    scene
        .block(pos)
        .is_some_and(|block| block.id.namespace == "minecraft" && block.id.path == "nether_portal")
}

pub fn animate_block<S: Scene + ?Sized>(
    scene: &S,
    pos: BlockPos,
    random: &mut LegacyRandom,
    out: &mut DisplayTick,
) {
    if is_portal(scene, pos) {
        nether_portal(scene, pos, random, out);
    }
}

fn nether_portal<S: Scene + ?Sized>(
    scene: &S,
    pos: BlockPos,
    random: &mut LegacyRandom,
    out: &mut DisplayTick,
) {
    let (bx, by, bz) = (f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    if random.next_int(AMBIENT_CHANCE) == 0 {
        out.sounds.push(AmbientSound {
            event: PORTAL_AMBIENT,
            at: [bx + 0.5, by + 0.5, bz + 0.5],
            volume: AMBIENT_VOLUME,
            pitch: random.next_float() * AMBIENT_PITCH_SPREAD + AMBIENT_PITCH,
        });
    }
    let along_z =
        is_portal(scene, (pos.0 - 1, pos.1, pos.2)) || is_portal(scene, (pos.0 + 1, pos.1, pos.2));
    for _ in 0..PORTAL_PARTICLES {
        let mut start = DVec3::new(
            bx + random.next_double(),
            by + random.next_double(),
            bz + random.next_double(),
        );
        let mut motion = DVec3::new(
            (f64::from(random.next_float()) - 0.5) * 0.5,
            (f64::from(random.next_float()) - 0.5) * 0.5,
            (f64::from(random.next_float()) - 0.5) * 0.5,
        );
        let side = random.next_int(2) as i32 * 2 - 1;
        let outward = f64::from(random.next_float() * 2.0 * side as f32);
        if along_z {
            start.z = bz + 0.5 + 0.25 * f64::from(side);
            motion.z = outward;
        } else {
            start.x = bx + 0.5 + 0.25 * f64::from(side);
            motion.x = outward;
        }
        out.portal_particles.push((start, motion));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Block, ChunkPos};
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

    fn portal(axis: &str) -> Block {
        let mut block = Block::new("minecraft:nether_portal");
        block.properties.insert("axis".to_owned(), axis.to_owned());
        block
    }

    #[test]
    fn a_portal_sheds_four_particles_out_of_its_faces() {
        let scene = Blocks(HashMap::from([((0, 64, 0), portal("x"))]));
        let mut out = DisplayTick::default();
        let mut random = LegacyRandom::new(7);
        animate_block(&scene, (0, 64, 0), &mut random, &mut out);
        assert_eq!(out.portal_particles.len(), 4);
        for (start, motion) in &out.portal_particles {
            assert!((start.x - 0.25).abs() < 1e-9 || (start.x - 0.75).abs() < 1e-9);
            assert!((64.0..65.0).contains(&start.y));
            assert!((0.0..1.0).contains(&start.z));
            assert!(motion.x.abs() <= 2.0 && motion.x * (start.x - 0.5) >= 0.0);
            assert!(motion.y.abs() <= 0.25 && motion.z.abs() <= 0.25);
        }
    }

    #[test]
    fn a_sheet_along_x_sheds_them_along_z() {
        let scene = Blocks(HashMap::from([
            ((0, 64, 0), portal("x")),
            ((1, 64, 0), portal("x")),
        ]));
        let mut out = DisplayTick::default();
        let mut random = LegacyRandom::new(3);
        animate_block(&scene, (0, 64, 0), &mut random, &mut out);
        for (start, motion) in &out.portal_particles {
            assert!((start.z - 0.25).abs() < 1e-9 || (start.z - 0.75).abs() < 1e-9);
            assert!((0.0..1.0).contains(&start.x));
            assert!(motion.z * (start.z - 0.5) >= 0.0);
        }
    }

    #[test]
    fn the_ambient_hum_comes_once_in_a_hundred() {
        let scene = Blocks(HashMap::from([((0, 64, 0), portal("x"))]));
        let mut out = DisplayTick::default();
        let mut random = LegacyRandom::new(11);
        for _ in 0..10_000 {
            animate_block(&scene, (0, 64, 0), &mut random, &mut out);
        }
        assert!(
            (50..150).contains(&out.sounds.len()),
            "{}",
            out.sounds.len()
        );
        for sound in &out.sounds {
            assert_eq!(sound.event, PORTAL_AMBIENT);
            assert_eq!(sound.at, [0.5, 64.5, 0.5]);
            assert_eq!(sound.volume, 0.5);
            assert!((0.8..1.2).contains(&sound.pitch));
        }
    }

    #[test]
    fn sampling_reaches_blocks_near_the_player_and_not_beyond_the_far_radius() {
        let scene = Blocks(HashMap::from([
            ((2, 65, -1), portal("x")),
            ((40, 64, 0), portal("x")),
        ]));
        let mut random = LegacyRandom::new(5);
        let mut near = 0;
        for _ in 0..200 {
            let tick = animate_tick(&scene, (0, 64, 0), &mut random);
            near += tick.portal_particles.len();
            assert!(
                tick.portal_particles
                    .iter()
                    .all(|(start, _)| start.x < 39.0)
            );
        }
        assert!(near > 0);
    }

    #[test]
    fn other_blocks_do_nothing() {
        let scene = Blocks(HashMap::from([(
            (0, 64, 0),
            Block::new("minecraft:obsidian"),
        )]));
        let mut out = DisplayTick::default();
        animate_block(&scene, (0, 64, 0), &mut LegacyRandom::new(1), &mut out);
        assert_eq!(out, DisplayTick::default());
    }
}
