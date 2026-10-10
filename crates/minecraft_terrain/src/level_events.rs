use crate::ambient_particles::Kind;
use crate::display_ticks::{AmbientSound, DisplayTick};
use crate::scene::BlockPos;
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const LAVA_FIZZ: i32 = 1501;
pub const LAVA_EXTINGUISH: &str = "minecraft:block.lava.extinguish";
const FIZZ_SMOKE: usize = 8;
const EVAPORATION_SMOKE: usize = 8;

fn corner((x, y, z): BlockPos) -> DVec3 {
    DVec3::new(f64::from(x), f64::from(y), f64::from(z))
}

pub fn level_event(id: i32, pos: BlockPos, _data: i32, random: &mut LegacyRandom) -> DisplayTick {
    let mut out = DisplayTick::default();
    if id == LAVA_FIZZ {
        let base = corner(pos);
        out.sounds.push(AmbientSound {
            event: LAVA_EXTINGUISH,
            at: (base + DVec3::splat(0.5)).to_array(),
            volume: 0.5,
            pitch: 2.6 + (random.next_float() - random.next_float()) * 0.8,
        });
        for _ in 0..FIZZ_SMOKE {
            let x = base.x + random.next_double();
            let z = base.z + random.next_double();
            let at = DVec3::new(x, base.y + 1.2, z);
            out.particles.push((Kind::LargeSmoke, at, DVec3::ZERO));
        }
    }
    out
}

pub fn evaporation(pos: BlockPos, random: &mut LegacyRandom) -> DisplayTick {
    let mut out = DisplayTick::default();
    let base = corner(pos);
    for _ in 0..EVAPORATION_SMOKE {
        let x = base.x + random.next_double();
        let y = base.y + random.next_double();
        let z = base.z + random.next_double();
        out.particles
            .push((Kind::LargeSmoke, DVec3::new(x, y, z), DVec3::ZERO));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_fizzes_with_a_hiss_and_eight_puffs_over_the_block() {
        let mut random = LegacyRandom::new(1);
        let fizz = level_event(LAVA_FIZZ, (3, 10, -2), 0, &mut random);
        assert_eq!(fizz.sounds.len(), 1);
        let hiss = &fizz.sounds[0];
        assert_eq!(
            (hiss.event, hiss.at, hiss.volume),
            (LAVA_EXTINGUISH, [3.5, 10.5, -1.5], 0.5)
        );
        assert!((1.8..3.4).contains(&hiss.pitch));
        assert_eq!(fizz.particles.len(), 8);
        for (kind, at, motion) in fizz.particles {
            assert_eq!((kind, motion), (Kind::LargeSmoke, DVec3::ZERO));
            assert_eq!(at.y, 11.2);
            assert!((3.0..4.0).contains(&at.x) && (-2.0..-1.0).contains(&at.z));
        }
    }

    #[test]
    fn other_level_events_do_nothing_yet() {
        let mut random = LegacyRandom::new(2);
        assert_eq!(
            level_event(1009, (0, 0, 0), 0, &mut random),
            DisplayTick::default()
        );
    }

    #[test]
    fn evaporating_water_puffs_eight_smokes_inside_the_block() {
        let mut random = LegacyRandom::new(3);
        let steam = evaporation((0, 40, 0), &mut random);
        assert!(steam.sounds.is_empty());
        assert_eq!(steam.particles.len(), 8);
        for (kind, at, _) in steam.particles {
            assert_eq!(kind, Kind::LargeSmoke);
            assert!((0.0..1.0).contains(&at.x) && (40.0..41.0).contains(&at.y));
        }
    }
}
