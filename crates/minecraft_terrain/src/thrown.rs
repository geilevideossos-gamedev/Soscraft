use glam::{DVec3, Vec2, Vec3};
use minecraftoss_entities::health::DamageResult;
use minecraftoss_entities::monster_ai::Target;
use minecraftoss_entities::movement::Body;
use minecraftoss_entities::potion::{Potion, PotionTarget, ThrownPotion};
use minecraftoss_entities::world::{DamageSourceKind, EntityWorld};
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;

pub const GRAVITY: f64 = 0.03;
pub const POWER: f32 = 1.5;
pub const INACCURACY: f32 = 1.0;
pub const PEARL_DAMAGE: f32 = 5.0;
pub const PEARL_COOLDOWN: u32 = 20;
const OWNER: u64 = u64::MAX;
const KNOCKBACK: f64 = 0.4;
const THROWN: &str = "minecraft:thrown";
const PLAYER_TARGET: u64 = u64::MAX / 2;
const HIDDEN_TICKS: i32 = 2;
const HIDDEN_NEAR: f64 = 12.25;
const CHICK_NUDGE: f64 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Throwable {
    Snowball,
    Egg,
    EnderPearl,
}

impl Throwable {
    pub fn of(id: &str) -> Option<Self> {
        match id {
            "minecraft:snowball" => Some(Self::Snowball),
            "minecraft:egg" | "minecraft:blue_egg" | "minecraft:brown_egg" => Some(Self::Egg),
            "minecraft:ender_pearl" => Some(Self::EnderPearl),
            _ => None,
        }
    }

    pub fn sound(self) -> &'static str {
        match self {
            Self::Snowball => "minecraft:entity.snowball.throw",
            Self::Egg => "minecraft:entity.egg.throw",
            Self::EnderPearl => "minecraft:entity.ender_pearl.throw",
        }
    }
}

pub fn throw_pitch(random: f32) -> f32 {
    0.4 / (random * 0.4 + 0.8)
}

pub fn chick_variant(item: &str) -> &'static str {
    match item {
        "minecraft:blue_egg" => "minecraft:cold",
        "minecraft:brown_egg" => "minecraft:warm",
        _ => "minecraft:temperate",
    }
}

pub fn chicks(random: &mut LegacyRandom) -> usize {
    if random.next_int(8) != 0 {
        0
    } else if random.next_int(32) == 0 {
        4
    } else {
        1
    }
}

pub fn shot_direction(yaw: f32, pitch: f32) -> DVec3 {
    let (y, x) = (yaw.to_radians(), pitch.to_radians());
    DVec3::new(
        f64::from(-y.sin() * x.cos()),
        f64::from(-x.sin()),
        f64::from(y.cos() * x.cos()),
    )
}

#[derive(Clone, Debug)]
pub struct Thrown {
    pub item: String,
    pub kind: Throwable,
    pub previous: DVec3,
    flight: ThrownPotion,
    random: LegacyRandom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landed {
    pub at: DVec3,
    pub before: DVec3,
    pub mob: Option<u64>,
    pub velocity: DVec3,
}

impl Thrown {
    pub fn throw(
        item: &str,
        eye: DVec3,
        yaw: f32,
        pitch: f32,
        motion: DVec3,
        on_ground: bool,
        random: &mut LegacyRandom,
    ) -> Option<Self> {
        let kind = Throwable::of(item)?;
        let start = eye - DVec3::new(0.0, 0.1, 0.0);
        let direction = shot_direction(yaw, pitch);
        let mut flight = ThrownPotion::shoot(
            Potion::Healing,
            OWNER,
            start,
            direction,
            POWER,
            INACCURACY,
            random,
        );
        flight.velocity += DVec3::new(motion.x, if on_ground { 0.0 } else { motion.y }, motion.z);
        Some(Self {
            item: item.to_owned(),
            kind,
            previous: start,
            flight,
            random: LegacyRandom::new(random.next_long()),
        })
    }

    pub fn position(&self) -> DVec3 {
        self.flight.position
    }

    pub fn velocity(&self) -> DVec3 {
        self.flight.velocity
    }

    pub fn alive(&self) -> bool {
        self.flight.alive
    }

    pub fn yaw(&self) -> f32 {
        self.flight.yaw
    }

    pub fn age(&self) -> i32 {
        self.flight.tick_count
    }

    pub fn random(&mut self) -> &mut LegacyRandom {
        &mut self.random
    }

    pub fn tick(
        &mut self,
        world: &impl World,
        targets: &[PotionTarget],
        owner: Option<(DVec3, DVec3)>,
    ) -> Option<Landed> {
        self.previous = self.flight.position;
        self.flight.velocity.y += minecraftoss_entities::potion::GRAVITY - GRAVITY;
        let hit = self.flight.tick(world, targets, owner)?;
        Some(Landed {
            at: hit.location,
            before: self.previous,
            mob: hit.entity,
            velocity: self.flight.velocity,
        })
    }
}

fn target(id: u64, body: &Body) -> PotionTarget {
    let half = f64::from(body.width) / 2.0;
    let p = body.position;
    PotionTarget {
        id,
        min: DVec3::new(p.x - half, p.y, p.z - half),
        max: DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
        owner: false,
    }
}

pub fn targets(world: &EntityWorld) -> Vec<PotionTarget> {
    let mut out = Vec::new();
    out.extend(
        world
            .bats()
            .iter()
            .filter(|e| e.bat.health > 0.0)
            .map(|e| target(e.id, &e.bat.body)),
    );
    out.extend(
        world
            .zombies()
            .iter()
            .filter(|e| e.zombie.health > 0.0)
            .map(|e| target(e.id, &e.zombie.body)),
    );
    out.extend(
        world
            .skeletons()
            .iter()
            .filter(|e| e.skeleton.health > 0.0)
            .map(|e| target(e.id, &e.skeleton.body)),
    );
    out.extend(
        world
            .creepers()
            .iter()
            .filter(|e| e.creeper.health > 0.0 && !e.creeper.exploded)
            .map(|e| target(e.id, &e.creeper.body)),
    );
    out.extend(
        world
            .spiders()
            .iter()
            .filter(|e| e.spider.health > 0.0)
            .map(|e| target(e.id, &e.spider.body)),
    );
    out.extend(
        world
            .slimes()
            .iter()
            .filter(|e| e.slime.health > 0.0)
            .map(|e| target(e.id, &e.slime.body)),
    );
    out.extend(
        world
            .endermen()
            .iter()
            .filter(|e| e.enderman.health > 0.0)
            .map(|e| target(e.id, &e.enderman.body)),
    );
    out.extend(
        world
            .witches()
            .iter()
            .filter(|e| e.witch.health > 0.0)
            .map(|e| target(e.id, &e.witch.body)),
    );
    out.extend(
        world
            .iron_golems()
            .iter()
            .filter(|e| e.golem.health > 0.0)
            .map(|e| target(e.id, &e.golem.body)),
    );
    out.extend(
        world
            .wolves()
            .iter()
            .filter(|e| e.wolf.health > 0.0)
            .map(|e| target(e.id, &e.wolf.body)),
    );
    out.extend(
        world
            .villagers()
            .iter()
            .filter(|e| e.villager.health > 0.0)
            .map(|e| target(e.id, &e.villager.body)),
    );
    out.extend(
        world
            .cows()
            .iter()
            .filter(|e| e.cow.health > 0.0)
            .map(|e| target(e.id, &e.cow.body)),
    );
    out.extend(
        world
            .sheep()
            .iter()
            .filter(|e| e.health > 0.0)
            .map(|e| target(e.id, &e.body)),
    );
    out.extend(
        world
            .pigs()
            .iter()
            .filter(|e| e.pig.health > 0.0)
            .map(|e| target(e.id, &e.pig.body)),
    );
    out.extend(
        world
            .chickens()
            .iter()
            .filter(|e| e.chicken.health > 0.0)
            .map(|e| target(e.id, &e.chicken.body)),
    );
    out
}

fn knockback(body: &mut Body, random: &mut LegacyRandom, power: f64, mut xd: f64, mut zd: f64) {
    while xd * xd + zd * zd < f64::from(1.0e-5_f32) {
        xd = (random.next_double() - random.next_double()) * 0.01;
        zd = (random.next_double() - random.next_double()) * 0.01;
    }
    let length = (xd * xd + zd * zd).sqrt();
    let (dx, dz) = (xd / length * power, zd / length * power);
    let v = body.velocity;
    let y = if body.on_ground {
        (v.y / 2.0 + power).min(0.4)
    } else {
        v.y
    };
    body.velocity = DVec3::new(v.x / 2.0 - dx, y, v.z / 2.0 - dz);
    body.needs_sync = true;
}

pub fn hurt(
    world: &mut EntityWorld,
    id: u64,
    velocity: DVec3,
    player: u64,
    random: &mut LegacyRandom,
) -> bool {
    if let Some(e) = world.enderman_mut(id) {
        e.dodge_pending = true;
        return false;
    }
    let by = Some(Target::Player(player));
    let time = world.game_time();
    let projectile = DamageSourceKind::Projectile;
    let blame =
        |hit: &DamageResult, tick: i32| hit.applied.then_some((Target::Player(player), tick));
    let (hit, resists) = if let Some(e) = world.bat_mut(id) {
        (e.hurt(0.0), false)
    } else if let Some(e) = world.zombie_mut(id) {
        let hit = e.hurt(0.0);
        if let (Some(ai), Some(blamed)) = (e.ai.as_deref_mut(), blame(&hit, e.tick_count)) {
            ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.skeleton_mut(id) {
        let hit = e.hurt(0.0);
        if let (Some(ai), Some(blamed)) = (e.ai.as_deref_mut(), blame(&hit, e.tick_count)) {
            ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.creeper_mut(id) {
        let hit = e.hurt(0.0);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.spider_mut(id) {
        let hit = e.hurt(0.0);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.slime_mut(id) {
        let hit = e.hurt(0.0);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.witch_mut(id) {
        let hit = e.hurt(0.0, false, false);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.iron_golem_mut(id) {
        let hit = e.hurt(0.0);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, true)
    } else if let Some(e) = world.wolf_mut(id) {
        (e.hurt_from(0.0, THROWN, by, time), false)
    } else if let Some(e) = world.villager_mut(id) {
        (
            e.hurt_from(0.0, THROWN, Some(PLAYER_TARGET + player), time),
            false,
        )
    } else if let Some(e) = world.cow_mut(id) {
        (e.hurt(0.0, projectile), false)
    } else if let Some(e) = world.sheep_mut(id) {
        (e.hurt(0.0, projectile), false)
    } else if let Some(e) = world.pig_mut(id) {
        (e.hurt(0.0, projectile), false)
    } else if let Some(e) = world.chicken_mut(id) {
        (e.hurt_with_source(0.0, projectile), false)
    } else {
        return false;
    };
    if hit.applied {
        world.villager_hurt_by(id, PLAYER_TARGET + player);
    }
    if hit.applied && hit.full && !resists {
        if let Some(body) = world.body_mut(id) {
            knockback(body, random, KNOCKBACK, -velocity.x, -velocity.z);
        }
    }
    hit.applied
}

pub fn shown(age: i32, at: DVec3, camera: DVec3) -> bool {
    age >= HIDDEN_TICKS || at.distance_squared(camera) >= HIDDEN_NEAR
}

pub fn hatch_point(at: DVec3, before: DVec3) -> DVec3 {
    let back = before - at;
    if back.length_squared() < 1.0e-12 {
        at
    } else {
        at + back.normalize() * CHICK_NUDGE
    }
}

pub fn texture(item: &str) -> Option<ResourceId> {
    let path = item.strip_prefix("minecraft:")?;
    ResourceId::parse(&format!("minecraft:item/{path}")).ok()
}

pub fn append_sprites<'a>(
    mesh: &mut ChunkMesh,
    items: impl IntoIterator<Item = (&'a str, DVec3)>,
    atlas: &Atlas,
    forward: Vec3,
    light: &SkyLight,
) {
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    for (item, at) in items {
        let Some(id) = texture(item).filter(|id| atlas.contains(id)) else {
            continue;
        };
        let cell = (
            at.x.floor() as i32,
            at.y.floor() as i32,
            at.z.floor() as i32,
        );
        let (sky, block) = (light.get(cell) as f32, light.get_block(cell) as f32);
        let center = at.as_vec3() + up * 0.125;
        let [u0, v0, u1, v1] = atlas.region(&id);
        let start = mesh.vertices.len() as u32;
        for (corner, uv) in [
            (Vec2::new(-0.25, -0.25), [u0, v1]),
            (Vec2::new(0.25, -0.25), [u1, v1]),
            (Vec2::new(0.25, 0.25), [u1, v0]),
            (Vec2::new(-0.25, 0.25), [u0, v0]),
        ] {
            let point = center + right * corner.x + up * corner.y;
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                uv,
                color: [1.0; 4],
                sky_light: sky,
                block_light: block,
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

    fn throw(item: &str, pitch: f32) -> Thrown {
        let mut random = LegacyRandom::new(42);
        Thrown::throw(
            item,
            DVec3::new(0.5, 70.0, 0.5),
            0.0,
            pitch,
            DVec3::ZERO,
            true,
            &mut random,
        )
        .expect("a throwable")
    }

    #[test]
    fn only_snowballs_eggs_and_pearls_are_thrown() {
        assert_eq!(
            Throwable::of("minecraft:snowball"),
            Some(Throwable::Snowball)
        );
        assert_eq!(Throwable::of("minecraft:brown_egg"), Some(Throwable::Egg));
        assert_eq!(
            Throwable::of("minecraft:ender_pearl"),
            Some(Throwable::EnderPearl)
        );
        assert_eq!(Throwable::of("minecraft:stone"), None);
        assert_eq!(chick_variant("minecraft:blue_egg"), "minecraft:cold");
        assert_eq!(chick_variant("minecraft:egg"), "minecraft:temperate");
    }

    #[test]
    fn a_throw_leaves_from_under_the_eye_at_one_and_a_half_blocks_a_tick() {
        let thrown = throw("minecraft:snowball", 0.0);
        assert!((thrown.position().y - 69.9).abs() < 1e-9);
        let speed = thrown.velocity().length();
        assert!((speed - 1.5).abs() < 0.05, "{speed}");
        assert!(thrown.velocity().z > 1.4);
    }

    #[test]
    fn a_throw_carries_the_throwers_motion_but_not_its_fall_on_the_ground() {
        let mut random = LegacyRandom::new(1);
        let still = Thrown::throw(
            "minecraft:egg",
            DVec3::ZERO,
            0.0,
            0.0,
            DVec3::ZERO,
            false,
            &mut random.clone(),
        )
        .unwrap();
        let moving = Thrown::throw(
            "minecraft:egg",
            DVec3::ZERO,
            0.0,
            0.0,
            DVec3::new(0.2, -0.5, 0.0),
            true,
            &mut random,
        )
        .unwrap();
        assert!((moving.velocity().x - still.velocity().x - 0.2).abs() < 1e-9);
        assert!((moving.velocity().y - still.velocity().y).abs() < 1e-9);
    }

    #[test]
    fn thrown_items_fall_under_vanilla_gravity_and_drag() {
        let world = Blocks::default();
        let mut thrown = throw("minecraft:snowball", 0.0);
        let before = thrown.velocity();
        assert!(thrown.tick(&world, &[], None).is_none());
        let expected = (before.y - GRAVITY) * f64::from(0.99_f32);
        assert!((thrown.velocity().y - expected).abs() < 1e-9);
        assert!((thrown.velocity().z - before.z * f64::from(0.99_f32)).abs() < 1e-9);
    }

    #[test]
    fn a_pearl_lands_where_it_was_before_the_tick_it_struck() {
        let mut world = Blocks::default();
        for x in -2..=2 {
            for z in -2..=40 {
                world.set_block((x, 60, z), Some(Block::new("minecraft:stone")));
            }
        }
        let mut thrown = throw("minecraft:ender_pearl", 0.0);
        let mut landed = None;
        for _ in 0..200 {
            if let Some(hit) = thrown.tick(&world, &[], None) {
                landed = Some(hit);
                break;
            }
        }
        let landed = landed.expect("the pearl lands");
        assert!((landed.at.y - 61.0).abs() < 1e-6, "{landed:?}");
        assert!(landed.before.y > landed.at.y);
        assert!(!thrown.alive());
    }

    #[test]
    fn a_thrown_item_strikes_a_mob_in_its_way() {
        let world = Blocks::default();
        let mut thrown = throw("minecraft:snowball", 0.0);
        let cow = PotionTarget {
            id: 7,
            min: DVec3::new(0.0, 69.0, 3.0),
            max: DVec3::new(1.0, 71.0, 4.0),
            owner: false,
        };
        let mut landed = None;
        for _ in 0..10 {
            if let Some(hit) = thrown.tick(&world, &[cow], None) {
                landed = Some(hit);
                break;
            }
        }
        assert_eq!(landed.and_then(|l| l.mob), Some(7));
    }

    #[test]
    fn about_one_egg_in_eight_hatches_and_one_hatch_in_thirty_two_is_four_chicks() {
        let mut random = LegacyRandom::new(9);
        let rolls: Vec<usize> = (0..256_000).map(|_| chicks(&mut random)).collect();
        let hatched = rolls.iter().filter(|&&n| n > 0).count() as f64 / rolls.len() as f64;
        let fours = rolls.iter().filter(|&&n| n == 4).count() as f64 / rolls.len() as f64;
        assert!((hatched - 1.0 / 8.0).abs() < 0.005, "{hatched}");
        assert!((fours - 1.0 / 256.0).abs() < 0.001, "{fours}");
        assert!(rolls.iter().all(|&n| matches!(n, 0 | 1 | 4)));
    }

    #[test]
    fn a_fresh_throw_is_not_drawn_in_the_throwers_face() {
        let camera = DVec3::new(0.0, 70.0, 0.0);
        assert!(!shown(1, DVec3::new(0.0, 70.0, 0.5), camera));
        assert!(shown(2, DVec3::new(0.0, 70.0, 0.5), camera));
        assert!(shown(1, DVec3::new(0.0, 70.0, 4.0), camera));
    }

    #[test]
    fn chicks_hatch_off_the_face_the_egg_struck() {
        let at = DVec3::new(5.0, 64.5, 2.0);
        let hatched = hatch_point(at, DVec3::new(4.0, 64.5, 2.0));
        assert!((hatched.x - 4.75).abs() < 1e-9);
        assert_eq!(hatch_point(at, at), at);
    }

    #[test]
    fn the_throw_sound_is_pitched_down_at_random() {
        assert!((throw_pitch(0.0) - 0.5).abs() < 1e-6);
        assert!((throw_pitch(1.0) - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn sprites_come_from_the_item_textures() {
        assert_eq!(
            texture("minecraft:ender_pearl").map(|id| id.key()),
            Some("minecraft:item/ender_pearl".to_owned())
        );
    }
}
