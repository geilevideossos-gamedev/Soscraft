use crate::sight;
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

pub const SIZE: f64 = 0.25;
pub const DAMAGE: f32 = 1.0;
pub const GRAVITY: f64 = 0.06;
pub const DRAG: f64 = 0.99_f32 as f64;
pub const POWER: f32 = 1.5;
pub const UNCERTAINTY: f32 = 10.0;

#[derive(Clone, Copy, Debug)]
pub struct SpitTarget {
    pub id: u64,
    pub min: DVec3,
    pub max: DVec3,
    pub owner: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpitHit {
    Entity(u64),
    Block,
}

#[derive(Clone, Debug)]
pub struct LlamaSpit {
    pub position: DVec3,
    pub velocity: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub yaw_o: f32,
    pub pitch_o: f32,
    pub owner: Option<u64>,
    pub owner_uuid: Option<u128>,
    pub left_owner: bool,
    pub tick_count: i32,
    pub alive: bool,
}

fn atan2_degrees(y: f64, x: f64) -> f32 {
    (y.atan2(x) * 57.295_776_367_187_5) as f32
}

fn lerp_rotation(previous: f32, current: f32) -> f32 {
    let mut previous = previous;
    while current - previous < -180.0 {
        previous -= 360.0;
    }
    while current - previous >= 180.0 {
        previous += 360.0;
    }
    previous + 0.2 * (current - previous)
}

fn triangle(random: &mut LegacyRandom, deviation: f64) -> f64 {
    deviation * (random.next_double() - random.next_double())
}

pub fn spat_from(position: DVec3, width: f32, body_yaw: f32, eye_y: f64) -> DVec3 {
    let reach = f64::from(width + 1.0) * 0.5;
    let turn = f64::from(body_yaw * 0.017_453_292);
    DVec3::new(
        position.x - reach * f64::from(minecraftoss_player::mth::sin(turn)),
        eye_y - 0.10000000149011612,
        position.z + reach * f64::from(minecraftoss_player::mth::cos(turn)),
    )
}

impl LlamaSpit {
    pub fn spit(owner: u64, from: DVec3, aim: DVec3, random: &mut LegacyRandom) -> Self {
        let dx = aim.x - from.x;
        let dz = aim.z - from.z;
        let lift = (dx * dx + dz * dz).sqrt() * 0.20000000298023224;
        let direction = DVec3::new(dx, aim.y - from.y + lift, dz).normalize_or_zero();
        let spread = 0.0172275 * f64::from(UNCERTAINTY);
        let velocity = DVec3::new(
            direction.x + triangle(random, spread),
            direction.y + triangle(random, spread),
            direction.z + triangle(random, spread),
        ) * f64::from(POWER);
        let horizontal = (velocity.x * velocity.x + velocity.z * velocity.z).sqrt();
        let yaw = atan2_degrees(velocity.x, velocity.z);
        let pitch = atan2_degrees(velocity.y, horizontal);
        Self {
            position: from,
            velocity,
            yaw,
            pitch,
            yaw_o: yaw,
            pitch_o: pitch,
            owner: Some(owner),
            owner_uuid: None,
            left_owner: false,
            tick_count: 0,
            alive: true,
        }
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        bounds_at(self.position)
    }

    fn margin(&self) -> f64 {
        f64::from(0.0_f32.max(0.3_f32.min((self.tick_count - 2) as f32 / 20.0)))
    }

    pub fn tick(
        &mut self,
        world: &impl World,
        targets: &[SpitTarget],
        owner_box: Option<(DVec3, DVec3)>,
    ) -> Option<SpitHit> {
        self.tick_count += 1;
        if !self.left_owner {
            self.left_owner = match owner_box {
                Some((owner_min, owner_max)) => {
                    let (min, max) = self.bounds();
                    let (min, max) = sweep(min, max, self.velocity);
                    !overlaps(owner_min, owner_max, min - DVec3::ONE, max + DVec3::ONE)
                }
                None => true,
            };
        }
        let from = self.position;
        let movement = self.velocity;
        let mut to = from + movement;
        let block = sight::clip(world, from, to);
        if let Some((_, at)) = block {
            to = at;
        }
        let (min, max) = self.bounds();
        let (search_min, search_max) = sweep(min, max, movement);
        let (search_min, search_max) = (search_min - DVec3::ONE, search_max + DVec3::ONE);
        let grow = DVec3::splat(self.margin());
        let mut nearest = f64::MAX;
        let mut struck = None;
        for target in targets {
            if target.owner && !self.left_owner {
                continue;
            }
            if !overlaps(target.min, target.max, search_min, search_max) {
                continue;
            }
            if let Some(at) = sight::clip_box(target.min - grow, target.max + grow, from, to) {
                let d = from.distance_squared(at);
                if d < nearest {
                    nearest = d;
                    struck = Some(target.id);
                }
            }
        }
        let hit = match (struck, block) {
            (Some(id), _) => Some(SpitHit::Entity(id)),
            (None, Some((pos, _))) if !is_air(world, pos) => {
                self.alive = false;
                Some(SpitHit::Block)
            }
            _ => None,
        };
        let next = from + movement;
        let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
        self.pitch_o = self.pitch;
        self.yaw_o = self.yaw;
        self.pitch = lerp_rotation(self.pitch_o, atan2_degrees(movement.y, horizontal));
        self.yaw = lerp_rotation(self.yaw_o, atan2_degrees(movement.x, movement.z));
        if !self.alive {
            return hit;
        }
        if !touches_air(world, min, max) {
            self.alive = false;
            return hit;
        }
        if crate::fluid::FluidFrame::sample(world, from, SIZE as f32, SIZE as f32).in_water() {
            self.alive = false;
            return hit;
        }
        self.velocity = movement * DRAG - DVec3::new(0.0, GRAVITY, 0.0);
        self.position = next;
        if self.position.y < f64::from(world.min_y() - 64) {
            self.alive = false;
        }
        hit
    }
}

pub fn bounds_at(position: DVec3) -> (DVec3, DVec3) {
    let half = SIZE / 2.0;
    (
        DVec3::new(position.x - half, position.y, position.z - half),
        DVec3::new(position.x + half, position.y + SIZE, position.z + half),
    )
}

fn is_air(world: &impl World, pos: (i32, i32, i32)) -> bool {
    world.block(pos).is_none_or(|b| {
        matches!(
            b.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

fn touches_air(world: &impl World, min: DVec3, max: DVec3) -> bool {
    let low = min.floor();
    let high = max.floor();
    for x in low.x as i32..=high.x as i32 {
        for y in low.y as i32..=high.y as i32 {
            for z in low.z as i32..=high.z as i32 {
                if is_air(world, (x, y, z)) {
                    return true;
                }
            }
        }
    }
    false
}

fn sweep(min: DVec3, max: DVec3, by: DVec3) -> (DVec3, DVec3) {
    let mut min = min;
    let mut max = max;
    for axis in 0..3 {
        if by[axis] < 0.0 {
            min[axis] += by[axis];
        } else if by[axis] > 0.0 {
            max[axis] += by[axis];
        }
    }
    (min, max)
}

fn overlaps(a_min: DVec3, a_max: DVec3, b_min: DVec3, b_max: DVec3) -> bool {
    a_min.x < b_max.x
        && a_max.x > b_min.x
        && a_min.y < b_max.y
        && a_max.y > b_min.y
        && a_min.z < b_max.z
        && a_max.z > b_min.z
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::Block;
    use std::collections::HashMap;

    struct Floor(HashMap<(i32, i32, i32), Block>);

    impl World for Floor {
        fn block(&self, pos: (i32, i32, i32)) -> Option<Block> {
            Some(
                self.0
                    .get(&pos)
                    .cloned()
                    .unwrap_or_else(|| Block::new("minecraft:air")),
            )
        }

        fn set_block(&mut self, pos: (i32, i32, i32), block: Option<Block>) {
            match block {
                Some(block) => self.0.insert(pos, block),
                None => self.0.remove(&pos),
            };
        }
    }

    fn wall_at(z: i32) -> Floor {
        let mut blocks = HashMap::new();
        for x in -3..=3 {
            for y in 60..=70 {
                blocks.insert((x, y, z), Block::new("minecraft:stone"));
            }
        }
        Floor(blocks)
    }

    #[test]
    fn spit_flies_at_one_and_a_half_blocks_a_tick_and_falls() {
        let mut random = LegacyRandom::new(5);
        let from = DVec3::new(0.5, 64.0, 0.5);
        let mut spit = LlamaSpit::spit(1, from, DVec3::new(0.5, 64.0, 10.5), &mut random);
        let speed = spit.velocity.length();
        assert!((speed - 1.5).abs() < 0.2, "{speed}");
        assert!(spit.velocity.z > 1.3 && spit.velocity.y > 0.0);
        assert!(spit.yaw.abs() < 15.0, "{}", spit.yaw);
        let world = wall_at(100);
        let before = spit.velocity;
        assert_eq!(spit.tick(&world, &[], None), None);
        assert!((spit.velocity.y - (before.y * DRAG - GRAVITY)).abs() < 1.0e-9);
        assert_eq!(spit.position, from + before);
    }

    #[test]
    fn spit_hurts_what_it_meets_and_ends_on_a_wall() {
        let mut random = LegacyRandom::new(9);
        let from = DVec3::new(0.5, 64.0, 0.5);
        let mut spit = LlamaSpit::spit(1, from, DVec3::new(0.5, 64.2, 6.5), &mut random);
        let owner = SpitTarget {
            id: 1,
            min: DVec3::new(0.05, 62.5, -0.5),
            max: DVec3::new(0.95, 64.4, 0.4),
            owner: true,
        };
        let player = SpitTarget {
            id: 7,
            min: DVec3::new(-0.5, 63.0, 3.0),
            max: DVec3::new(1.5, 65.5, 4.0),
            owner: false,
        };
        let world = wall_at(8);
        let mut hits = Vec::new();
        for _ in 0..20 {
            if let Some(hit) = spit.tick(&world, &[owner, player], None) {
                hits.push(hit);
            }
            if !spit.alive {
                break;
            }
        }
        assert_eq!(hits.first(), Some(&SpitHit::Entity(7)), "{hits:?}");
        assert!(!hits.contains(&SpitHit::Entity(1)), "never its llama");
        assert_eq!(hits.last(), Some(&SpitHit::Block));
        assert!(!spit.alive);
    }

    #[test]
    fn spit_ends_in_water() {
        let mut blocks = HashMap::new();
        for x in -2..=2 {
            for y in 60..=70 {
                for z in 0..=6 {
                    blocks.insert((x, y, z), Block::new("minecraft:water"));
                }
            }
        }
        let world = Floor(blocks);
        let mut random = LegacyRandom::new(3);
        let mut spit = LlamaSpit::spit(
            1,
            DVec3::new(0.5, 64.0, 0.5),
            DVec3::new(0.5, 64.0, 5.5),
            &mut random,
        );
        spit.tick(&world, &[], None);
        assert!(!spit.alive);
    }
}
