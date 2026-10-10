use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

use crate::scene::{Block, BlockPos};

pub const DRAGON_EGG: &str = "minecraft:dragon_egg";
pub const TRIES: usize = 1000;
pub const HORIZONTAL_SPREAD: i32 = 16;
pub const VERTICAL_SPREAD: i32 = 8;
pub const PARTICLES: usize = 128;
pub const BORDER: i32 = 29_999_984;

pub fn is_egg(block: &Block) -> bool {
    block.id.namespace == "minecraft" && block.id.path == "dragon_egg"
}

pub fn offset(next_int: &mut impl FnMut(i32) -> i32) -> (i32, i32, i32) {
    let x = next_int(HORIZONTAL_SPREAD) - next_int(HORIZONTAL_SPREAD);
    let y = next_int(VERTICAL_SPREAD) - next_int(VERTICAL_SPREAD);
    let z = next_int(HORIZONTAL_SPREAD) - next_int(HORIZONTAL_SPREAD);
    (x, y, z)
}

pub fn within_border(pos: BlockPos) -> bool {
    (-BORDER..BORDER).contains(&pos.0) && (-BORDER..BORDER).contains(&pos.2)
}

pub fn teleport_target(
    from: BlockPos,
    next_int: &mut impl FnMut(i32) -> i32,
    fits: impl Fn(BlockPos) -> bool,
) -> Option<BlockPos> {
    for _ in 0..TRIES {
        let (dx, dy, dz) = offset(next_int);
        let to = (from.0 + dx, from.1 + dy, from.2 + dz);
        if fits(to) {
            return Some(to);
        }
    }
    None
}

fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

pub fn teleport_particles(
    from: BlockPos,
    to: BlockPos,
    random: &mut LegacyRandom,
) -> Vec<(DVec3, DVec3)> {
    let corner = |pos: BlockPos| DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    let (from, to) = (corner(from), corner(to));
    (0..PARTICLES)
        .map(|_| {
            let t = random.next_double();
            let xa = f64::from((random.next_float() - 0.5) * 0.2);
            let ya = f64::from((random.next_float() - 0.5) * 0.2);
            let za = f64::from((random.next_float() - 0.5) * 0.2);
            let x = lerp(t, to.x, from.x) + (random.next_double() - 0.5) + 0.5;
            let y = lerp(t, to.y, from.y) + random.next_double() - 0.5;
            let z = lerp(t, to.z, from.z) + (random.next_double() - 0.5) + 0.5;
            (DVec3::new(x, y, z), DVec3::new(xa, ya, za))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::ResourceId;

    #[test]
    fn only_the_dragon_egg_teleports() {
        let block = |id: &str| Block {
            id: ResourceId::parse(id).unwrap(),
            properties: Default::default(),
        };
        assert!(is_egg(&block(DRAGON_EGG)));
        assert!(!is_egg(&block("minecraft:end_stone")));
    }

    #[test]
    fn an_offset_draws_two_horizontal_two_vertical_and_two_horizontal_ints() {
        let mut bounds = Vec::new();
        let mut values = [15, 0, 0, 7, 3, 3].into_iter();
        let mut next = |bound: i32| {
            bounds.push(bound);
            values.next().unwrap()
        };
        assert_eq!(offset(&mut next), (15, -7, 0));
        assert_eq!(bounds, [16, 16, 8, 8, 16, 16]);
        let mut random = LegacyRandom::new(42);
        for _ in 0..2000 {
            let (x, y, z) = offset(&mut |bound| random.next_int(bound as u32) as i32);
            assert!((-15..=15).contains(&x) && (-7..=7).contains(&y) && (-15..=15).contains(&z));
        }
    }

    #[test]
    fn the_egg_lands_on_the_first_free_spot_drawn_or_stays_after_a_thousand_tries() {
        let mut random = LegacyRandom::new(7);
        let mut draws = 0;
        let ground = 60;
        let fits = |to: BlockPos| to.1 == ground + 1;
        let to = teleport_target(
            (0, 64, 0),
            &mut |bound| {
                draws += 1;
                random.next_int(bound as u32) as i32
            },
            fits,
        )
        .unwrap();
        assert_eq!(to.1, ground + 1);
        assert!(to.0.abs() <= 15 && to.2.abs() <= 15);
        assert_eq!(draws % 6, 0);
        let mut tries = 0;
        let mut random = LegacyRandom::new(7);
        let stuck = teleport_target(
            (0, 64, 0),
            &mut |bound| {
                tries += 1;
                random.next_int(bound as u32) as i32
            },
            |_| false,
        );
        assert_eq!(stuck, None, "an egg in a full box stays put");
        assert_eq!(tries, TRIES * 6);
        assert!(within_border((29_999_983, 0, -29_999_984)));
        assert!(!within_border((29_999_984, 0, 0)));
    }

    #[test]
    fn the_portal_particles_stream_between_the_two_spots() {
        let mut random = LegacyRandom::new(3);
        let particles = teleport_particles((0, 64, 0), (10, 60, -4), &mut random);
        assert_eq!(particles.len(), PARTICLES);
        for (start, motion) in &particles {
            assert!((-0.5..=11.5).contains(&start.x), "{start:?}");
            assert!((59.5..=64.5).contains(&start.y), "{start:?}");
            assert!((-4.5..=1.5).contains(&start.z), "{start:?}");
            assert!(motion.abs().max_element() <= 0.1);
        }
        let mut again = LegacyRandom::new(3);
        let t = again.next_double();
        let (xa, _, _) = (again.next_float(), again.next_float(), again.next_float());
        let x = 10.0 + t * -10.0 + (again.next_double() - 0.5) + 0.5;
        assert_eq!(particles[0].0.x, x);
        assert_eq!(particles[0].1.x, f64::from((xa - 0.5) * 0.2));
    }
}
