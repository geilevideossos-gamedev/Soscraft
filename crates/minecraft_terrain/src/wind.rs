use crate::thrown::{PlayerBody, face_normal};
use glam::DVec3;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_player::{Face, World};

pub const RADIUS: f32 = 1.2;
pub const KNOCKBACK: f64 = 1.22;
pub const BURST_SOUND: &str = "minecraft:entity.wind_charge.wind_burst";
pub const DAMAGE: f32 = 1.0;
pub const DAMAGE_TYPE: &str = "minecraft:wind_charge";
pub const COOLDOWN_TICKS: u32 = 10;
const EYE_SHARE: f64 = 0.85;
pub const BREEZE_RADIUS: f32 = 3.0;
pub const BREEZE_KNOCKBACK: f64 = 1.0;
pub const BREEZE_BURST_SOUND: &str = "minecraft:entity.breeze.wind_burst";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charge {
    Player,
    Breeze,
}

impl Charge {
    pub fn of(item: &str) -> Self {
        if item == crate::breeze_charge::ENTITY {
            Self::Breeze
        } else {
            Self::Player
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            Self::Player => RADIUS,
            Self::Breeze => BREEZE_RADIUS,
        }
    }

    pub fn knockback(self) -> f64 {
        match self {
            Self::Player => KNOCKBACK,
            Self::Breeze => BREEZE_KNOCKBACK,
        }
    }

    pub fn sound(self) -> &'static str {
        match self {
            Self::Player => BURST_SOUND,
            Self::Breeze => BREEZE_BURST_SOUND,
        }
    }

    pub fn triggers_blocks(self, mob_griefing: bool) -> bool {
        self == Self::Player || mob_griefing
    }
}

pub fn burst_point(at: DVec3, face: Option<Face>) -> DVec3 {
    face.map_or(at, |face| at + face_normal(face) * 0.25)
}

pub fn burst_pitch(a: f32, b: f32) -> f32 {
    (1.0 + (a - b) * 0.2) * 0.7
}

fn lerp(delta: f64, a: f64, b: f64) -> f64 {
    a + delta * (b - a)
}

pub fn exposure(world: &impl World, center: DVec3, min: DVec3, max: DVec3) -> f32 {
    let xs = 1.0 / ((max.x - min.x) * 2.0 + 1.0);
    let ys = 1.0 / ((max.y - min.y) * 2.0 + 1.0);
    let zs = 1.0 / ((max.z - min.z) * 2.0 + 1.0);
    let x_offset = (1.0 - (1.0 / xs).floor() * xs) / 2.0;
    let z_offset = (1.0 - (1.0 / zs).floor() * zs) / 2.0;
    if xs < 0.0 || ys < 0.0 || zs < 0.0 {
        return 0.0;
    }
    let (mut hits, mut count) = (0, 0);
    let mut xx = 0.0;
    while xx <= 1.0 {
        let mut yy = 0.0;
        while yy <= 1.0 {
            let mut zz = 0.0;
            while zz <= 1.0 {
                let sample = DVec3::new(
                    lerp(xx, min.x, max.x) + x_offset,
                    lerp(yy, min.y, max.y),
                    lerp(zz, min.z, max.z) + z_offset,
                );
                if !minecraftoss_entities::sight::blocked(world, sample, center) {
                    hits += 1;
                }
                count += 1;
                zz += zs;
            }
            yy += ys;
        }
        xx += xs;
    }
    hits as f32 / count as f32
}

pub fn push(
    charge: Charge,
    center: DVec3,
    feet: DVec3,
    eye: DVec3,
    exposure: f32,
) -> Option<DVec3> {
    let distance = feet.distance(center) / f64::from(charge.radius() * 2.0);
    if distance > 1.0 {
        return None;
    }
    let direction = eye - center;
    let length = direction.length();
    let direction = if length < f64::from(1.0e-5_f32) {
        DVec3::ZERO
    } else {
        direction / length
    };
    Some(direction * ((1.0 - distance) * f64::from(exposure) * charge.knockback()))
}

fn reach(charge: Charge, center: DVec3) -> (DVec3, DVec3) {
    let r = f64::from(charge.radius() * 2.0) + 1.0;
    (center - DVec3::splat(r), center + DVec3::splat(r))
}

fn meets(a: (DVec3, DVec3), b: (DVec3, DVec3)) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
}

pub fn burst(
    world: &impl World,
    mobs: &mut EntityWorld,
    center: DVec3,
    players: &[PlayerBody],
    charge: Charge,
) -> Vec<(u64, DVec3)> {
    let area = reach(charge, center);
    for target in crate::thrown::targets(mobs) {
        if !meets(area, (target.min, target.max)) {
            continue;
        }
        let feet = crate::thrown::target_feet(&target);
        let eye = feet + DVec3::Y * ((target.max.y - target.min.y) * EYE_SHARE);
        let seen = exposure(world, center, target.min, target.max);
        let Some(push) = push(charge, center, feet, eye, seen) else {
            continue;
        };
        if let Some(body) = mobs.body_mut(target.id) {
            body.velocity += push;
            body.needs_sync = true;
        }
    }
    players
        .iter()
        .filter(|p| p.targetable() && meets(area, p.bounds()))
        .filter_map(|p| {
            let (min, max) = p.bounds();
            let eye = p.feet + DVec3::Y * crate::trident::EYE_HEIGHT;
            push(charge, center, p.feet, eye, exposure(world, center, min, max))
                .map(|push| (p.id, push))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};
    use std::collections::HashMap;

    #[derive(Default)]
    struct Blocks(HashMap<Pos, Block>);

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.0.get(&pos).cloned()
        }

        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.0.insert(pos, block),
                None => self.0.remove(&pos),
            };
        }
    }

    #[test]
    fn a_burst_pushes_away_and_weakens_with_distance() {
        let center = DVec3::new(0.0, 64.0, 0.0);
        let near = push(
            Charge::Player,
            center,
            DVec3::new(1.0, 64.0, 0.0),
            DVec3::new(1.0, 65.62, 0.0),
            1.0,
        )
        .unwrap();
        let far = push(
            Charge::Player,
            center,
            DVec3::new(2.0, 64.0, 0.0),
            DVec3::new(2.0, 65.62, 0.0),
            1.0,
        )
        .unwrap();
        assert!(near.x > 0.0 && near.y > 0.0);
        assert!(near.length() > far.length());
        let expected = (1.0 - 1.0 / f64::from(RADIUS * 2.0)) * KNOCKBACK;
        assert!((near.length() - expected).abs() < 1e-9);
        assert_eq!(
            push(
                Charge::Player,
                center,
                DVec3::new(2.5, 64.0, 0.0),
                DVec3::ZERO,
                1.0
            ),
            None
        );
    }

    #[test]
    fn a_breeze_charge_reaches_further_and_pushes_less_than_a_players() {
        let center = DVec3::new(0.0, 64.0, 0.0);
        let feet = DVec3::new(3.0, 64.0, 0.0);
        let eye = DVec3::new(3.0, 65.62, 0.0);
        assert_eq!(push(Charge::Player, center, feet, eye, 1.0), None);
        let breeze = push(Charge::Breeze, center, feet, eye, 1.0).unwrap();
        let expected = (1.0 - 3.0 / f64::from(BREEZE_RADIUS * 2.0)) * BREEZE_KNOCKBACK;
        assert!((breeze.length() - expected).abs() < 1e-9);
        assert_eq!(Charge::Breeze.sound(), "minecraft:entity.breeze.wind_burst");
        assert!(Charge::Player.triggers_blocks(false));
        assert!(!Charge::Breeze.triggers_blocks(false));
        assert!(Charge::Breeze.triggers_blocks(true));
    }

    #[test]
    fn a_wall_shields_what_is_behind_it() {
        let mut world = Blocks::default();
        for y in 60..70 {
            for z in -3..=3 {
                world.set_block((1, y, z), Some(Block::new("minecraft:stone")));
            }
        }
        let center = DVec3::new(0.5, 64.0, 0.5);
        let behind = (DVec3::new(2.2, 64.0, 0.2), DVec3::new(2.8, 65.8, 0.8));
        let open = (DVec3::new(-1.8, 64.0, 0.2), DVec3::new(-1.2, 65.8, 0.8));
        assert_eq!(exposure(&world, center, behind.0, behind.1), 0.0);
        assert_eq!(exposure(&world, center, open.0, open.1), 1.0);
    }

    #[test]
    fn it_bursts_a_quarter_block_off_the_face_it_hit() {
        let at = DVec3::new(3.0, 64.5, 0.5);
        assert_eq!(
            burst_point(at, Some(Face::West)),
            DVec3::new(2.75, 64.5, 0.5)
        );
        assert_eq!(burst_point(at, None), at);
        assert!((burst_pitch(0.5, 0.5) - 0.7).abs() < 1e-6);
    }
}
