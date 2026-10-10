use glam::{DVec3, Vec2, Vec3};
use minecraftoss_entities::fireball::{Fireball, FireballKind, FireballTarget, Shooter, bounds_at};
use minecraftoss_player::World;

use crate::effect_cloud::{Breath, EffectCloud};
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use crate::potions::PotionEffect;

pub const TEXTURE: &str = "minecraft:entity/enderdragon/dragon_fireball";
pub const EXPLODE_SOUND: &str = "minecraft:entity.dragon_fireball.explode";
pub const KIND: FireballKind = FireballKind::Large { explosion_power: 0 };
pub const SPLASH_RANGE: f64 = 4.0;
pub const CLOUD_RADIUS: f32 = 3.0;
pub const CLOUD_MAX_RADIUS: f32 = 7.0;
pub const CLOUD_DURATION: i32 = 600;
const SCALE: f32 = 2.0;

#[derive(Clone, Debug)]
pub struct DragonFireball {
    pub id: u64,
    pub owner: u64,
    pub fireball: Fireball,
    pub previous_position: DVec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireballView {
    pub id: u64,
    pub position: DVec3,
    pub previous_position: DVec3,
}

impl DragonFireball {
    pub fn shoot(id: u64, owner: u64, position: DVec3, direction: DVec3) -> Self {
        Self {
            id,
            owner,
            fireball: Fireball::shoot(KIND, Shooter::Mob(owner), position, direction),
            previous_position: position,
        }
    }

    pub fn tick(&mut self, world: &impl World, targets: &[FireballTarget]) -> Option<DVec3> {
        self.previous_position = self.fireball.position;
        self.fireball.tick(world, targets, None)?;
        self.fireball.alive = false;
        Some(self.fireball.position)
    }

    pub fn view(&self) -> FireballView {
        FireballView {
            id: self.id,
            position: self.fireball.position,
            previous_position: self.previous_position,
        }
    }
}

pub fn cloud_position(at: DVec3, living: &[FireballTarget]) -> DVec3 {
    let (min, max) = bounds_at(KIND, at);
    let grow = DVec3::new(SPLASH_RANGE, 2.0, SPLASH_RANGE);
    let (low, high) = (min - grow, max + grow);
    living
        .iter()
        .filter(|target| {
            target.min.x < high.x
                && target.max.x > low.x
                && target.min.y < high.y
                && target.max.y > low.y
                && target.min.z < high.z
                && target.max.z > low.z
        })
        .map(|target| {
            DVec3::new(
                (target.min.x + target.max.x) / 2.0,
                target.min.y,
                (target.min.z + target.max.z) / 2.0,
            )
        })
        .find(|feet| feet.distance_squared(at) < SPLASH_RANGE * SPLASH_RANGE)
        .unwrap_or(at)
}

pub fn breath_cloud(at: DVec3) -> EffectCloud {
    EffectCloud::dragon_breath(
        at,
        CLOUD_RADIUS,
        CLOUD_DURATION,
        (CLOUD_MAX_RADIUS - CLOUD_RADIUS) / CLOUD_DURATION as f32,
        PotionEffect::new("minecraft:instant_damage", 1, 1),
        Breath::Fireball,
    )
}

pub fn explode_pitch(random: f32) -> f32 {
    random * 0.1 + 0.9
}

#[derive(Default)]
pub struct DragonFireballs {
    pub list: Vec<DragonFireball>,
    next_id: u64,
    bursts: Vec<DVec3>,
}

impl DragonFireballs {
    pub fn shoot(&mut self, owner: u64, position: DVec3, direction: DVec3) -> u64 {
        self.next_id += 1;
        self.list.push(DragonFireball::shoot(
            self.next_id,
            owner,
            position,
            direction,
        ));
        self.next_id
    }

    pub fn burst(&mut self, at: DVec3) {
        self.bursts.push(at);
    }

    pub fn take_bursts(&mut self) -> Vec<DVec3> {
        std::mem::take(&mut self.bursts)
    }

    pub fn views(&self) -> Vec<FireballView> {
        self.list.iter().map(DragonFireball::view).collect()
    }
}

pub fn append_fireballs(
    mesh: &mut ChunkMesh,
    fireballs: &[FireballView],
    atlas: &Atlas,
    forward: Vec3,
    partial: f32,
) {
    let Ok(sprite) = ResourceId::parse(TEXTURE) else {
        return;
    };
    if !atlas.contains(&sprite) {
        return;
    }
    let [u0, v0, u1, v1] = atlas.region(&sprite);
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    for fireball in fireballs {
        let at = fireball
            .previous_position
            .lerp(fireball.position, f64::from(partial))
            .as_vec3();
        let start = mesh.vertices.len() as u32;
        for (corner, uv) in [
            (Vec2::new(-0.5, -0.25), [u0, v1]),
            (Vec2::new(0.5, -0.25), [u1, v1]),
            (Vec2::new(0.5, 0.75), [u1, v0]),
            (Vec2::new(-0.5, 0.75), [u0, v0]),
        ] {
            let point = at + (right * corner.x + up * corner.y) * SCALE;
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                uv,
                color: [1.0; 4],
                sky_light: 15.0,
                block_light: 15.0,
            });
        }
        mesh.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect_cloud::{BREATH_COLOR, Visitor};

    fn target(id: u64, feet: DVec3) -> FireballTarget {
        FireballTarget {
            id,
            min: feet - DVec3::new(0.3, 0.0, 0.3),
            max: feet + DVec3::new(0.3, 1.8, 0.3),
            owner: false,
        }
    }

    #[test]
    fn a_dragon_fireball_flies_like_a_hurting_projectile_from_its_aim() {
        let fireball = DragonFireball::shoot(1, 9, DVec3::new(0.0, 80.0, 0.0), DVec3::X * 3.0);
        assert_eq!(fireball.fireball.velocity, DVec3::new(0.1, 0.0, 0.0));
        assert_eq!(fireball.fireball.owner, Some(Shooter::Mob(9)));
        assert_eq!(KIND.size(), 1.0, "a 1 by 1 box");
    }

    #[test]
    fn the_breath_cloud_lands_on_a_living_entity_within_four_blocks() {
        let at = DVec3::new(10.0, 64.0, 10.0);
        assert_eq!(cloud_position(at, &[]), at);
        let near = DVec3::new(12.0, 64.0, 11.0);
        assert_eq!(cloud_position(at, &[target(1, near)]), near);
        let far = DVec3::new(14.5, 64.0, 10.0);
        assert_eq!(
            cloud_position(at, &[target(1, far)]),
            at,
            "inside the box but 4.5 blocks off"
        );
        let first = DVec3::new(9.0, 65.0, 10.0);
        assert_eq!(
            cloud_position(at, &[target(1, far), target(2, first), target(3, near)]),
            first
        );
    }

    #[test]
    fn the_fireball_cloud_harms_for_6_and_grows_from_3_to_7_over_600_ticks() {
        let mut cloud = breath_cloud(DVec3::new(0.5, 64.0, 0.5));
        assert_eq!(cloud.color, BREATH_COLOR);
        assert_eq!(cloud.breath, Some(Breath::Fireball));
        assert_eq!(cloud.owner, None);
        let effect = &cloud.effects[0];
        assert_eq!(
            crate::potions::instant_amount(effect, false, crate::effect_cloud::INSTANT_SCALE),
            Some(crate::potions::PlayerPotion::Harm {
                amount: 6.0,
                by: None
            }),
            "Harming II at half strength"
        );
        let feet = DVec3::new(1.5, 64.0, 0.5);
        let me = [Visitor {
            key: 7,
            feet,
            min: feet - DVec3::new(0.3, 0.0, 0.3),
            max: feet + DVec3::new(0.3, 1.8, 0.3),
            affected: true,
        }];
        let mut hits = Vec::new();
        while cloud.alive {
            if !cloud.tick(&me).is_empty() {
                hits.push(cloud.tick_count);
            }
        }
        assert_eq!(&hits[..3], &[20, 40, 60], "after 20 ticks, every 20 ticks");
        assert_eq!(cloud.tick_count, 620);
        assert!(
            (cloud.radius - 7.0).abs() < 0.01,
            "it never shrinks on use: {}",
            cloud.radius
        );
    }
}
