use crate::shulker::{block_of, is_air, overlaps};
use crate::sight;
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Face, Pos, World};

pub const SIZE: f64 = 0.3125;
pub const DAMAGE: f32 = 4.0;
pub const LEVITATION_TICKS: u32 = 200;
pub const SPEED: f64 = 0.15;
pub const GRAVITY: f64 = 0.04;
pub const HOMING: f64 = 1.025;
pub const STEER: f64 = 0.2;
pub const DIRECTIONS: [Face; 6] = [
    Face::Down,
    Face::Up,
    Face::North,
    Face::South,
    Face::West,
    Face::East,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

pub fn axis_of(face: Face) -> Axis {
    match face {
        Face::West | Face::East => Axis::X,
        Face::Down | Face::Up => Axis::Y,
        Face::North | Face::South => Axis::Z,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aim {
    pub position: DVec3,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BulletTarget {
    pub id: u64,
    pub min: DVec3,
    pub max: DVec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BulletHit {
    pub location: DVec3,
    pub entity: Option<u64>,
    pub block: Option<Pos>,
}

#[derive(Clone, Debug)]
pub struct ShulkerBullet {
    pub position: DVec3,
    pub velocity: DVec3,
    pub direction: Option<Face>,
    pub steps: i32,
    pub target_delta: DVec3,
    pub owner: Option<u64>,
    pub target: Option<u64>,
    pub owner_uuid: Option<u128>,
    pub target_uuid: Option<u128>,
    pub tick_count: i32,
    pub alive: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub yaw_o: f32,
    pub pitch_o: f32,
}

impl ShulkerBullet {
    pub fn new(position: DVec3) -> Self {
        Self {
            position,
            velocity: DVec3::ZERO,
            direction: None,
            steps: 0,
            target_delta: DVec3::ZERO,
            owner: None,
            target: None,
            owner_uuid: None,
            target_uuid: None,
            tick_count: 0,
            alive: true,
            yaw: 0.0,
            pitch: 0.0,
            yaw_o: 0.0,
            pitch_o: 0.0,
        }
    }

    pub fn trail(&self) -> DVec3 {
        self.position - self.velocity + DVec3::new(0.0, 0.15, 0.0)
    }

    pub fn turn(&self, partial: f32) -> (f32, f32) {
        (
            self.yaw_o + (self.yaw - self.yaw_o) * partial,
            self.pitch_o + (self.pitch - self.pitch_o) * partial,
        )
    }

    pub fn rotate_towards_movement(&mut self) {
        let v = self.velocity;
        if v.length_squared() == 0.0 {
            return;
        }
        let across = (v.x * v.x + v.z * v.z).sqrt();
        let degrees = 57.2957763671875;
        self.yaw = (minecraftoss_player::mth::atan2(v.z, v.x) * degrees) as f32 + 90.0;
        self.pitch = (minecraftoss_player::mth::atan2(across, v.y) * degrees) as f32 - 90.0;
        while self.pitch - self.pitch_o < -180.0 {
            self.pitch_o -= 360.0;
        }
        while self.pitch - self.pitch_o >= 180.0 {
            self.pitch_o += 360.0;
        }
        while self.yaw - self.yaw_o < -180.0 {
            self.yaw_o -= 360.0;
        }
        while self.yaw - self.yaw_o >= 180.0 {
            self.yaw_o += 360.0;
        }
        self.pitch = self.pitch_o + (self.pitch - self.pitch_o) * 0.5;
        self.yaw = self.yaw_o + (self.yaw - self.yaw_o) * 0.5;
    }

    pub fn shoot(
        world: &dyn World,
        owner: u64,
        position: DVec3,
        target: Option<(u64, Aim)>,
        axis: Axis,
        random: &mut LegacyRandom,
    ) -> Self {
        let mut bullet = Self::new(position);
        bullet.owner = Some(owner);
        bullet.target = target.map(|(id, _)| id);
        bullet.direction = Some(Face::Up);
        bullet.select_next_move_direction(world, Some(axis), target.map(|(_, aim)| aim), random);
        bullet
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        bounds_at(self.position)
    }

    pub fn block(&self) -> Pos {
        block_of(self.position)
    }

    pub fn margin(&self) -> f64 {
        f64::from(0.0_f32.max(0.3_f32.min((self.tick_count - 2) as f32 / 20.0)))
    }

    pub fn select_next_move_direction(
        &mut self,
        world: &dyn World,
        axis: Option<Axis>,
        target: Option<Aim>,
        random: &mut LegacyRandom,
    ) {
        let (aim_block, half) = match target {
            None => {
                let here = self.block();
                ((here.0, here.1 - 1, here.2), 0.5)
            }
            Some(aim) => {
                let half = f64::from(aim.height) * 0.5;
                (block_of(aim.position + DVec3::new(0.0, half, 0.0)), half)
            }
        };
        let mut aim = DVec3::new(
            f64::from(aim_block.0) + 0.5,
            f64::from(aim_block.1) + half,
            f64::from(aim_block.2) + 0.5,
        );
        let mut direction = None;
        if !closer_to_center_than(aim_block, self.position, 2.0) {
            let here = self.block();
            let empty = |face: Face| is_air(world, offset(here, face));
            let mut options = Vec::new();
            if axis != Some(Axis::X) {
                if here.0 < aim_block.0 && empty(Face::East) {
                    options.push(Face::East);
                } else if here.0 > aim_block.0 && empty(Face::West) {
                    options.push(Face::West);
                }
            }
            if axis != Some(Axis::Y) {
                if here.1 < aim_block.1 && empty(Face::Up) {
                    options.push(Face::Up);
                } else if here.1 > aim_block.1 && empty(Face::Down) {
                    options.push(Face::Down);
                }
            }
            if axis != Some(Axis::Z) {
                if here.2 < aim_block.2 && empty(Face::South) {
                    options.push(Face::South);
                } else if here.2 > aim_block.2 && empty(Face::North) {
                    options.push(Face::North);
                }
            }
            let mut chosen = random_direction(random);
            if options.is_empty() {
                let mut tries = 5;
                while !empty(chosen) && tries > 0 {
                    chosen = random_direction(random);
                    tries -= 1;
                }
            } else {
                chosen = options[random.next_int(options.len() as u32) as usize];
            }
            aim = self.position + step(chosen);
            direction = Some(chosen);
        }
        self.direction = direction;
        let delta = aim - self.position;
        let length = delta.length();
        self.target_delta = if length == 0.0 {
            DVec3::ZERO
        } else {
            delta / length * SPEED
        };
        self.steps = 10 + random.next_int(5) as i32 * 10;
    }

    pub fn tick(
        &mut self,
        world: &dyn World,
        targets: &[BulletTarget],
        target: Option<Aim>,
        homing: bool,
        random: &mut LegacyRandom,
    ) -> Option<BulletHit> {
        self.tick_count += 1;
        self.yaw_o = self.yaw;
        self.pitch_o = self.pitch;
        if self.position.y < f64::from(world.min_y() - 64) {
            self.alive = false;
            return None;
        }
        if homing {
            self.target_delta = DVec3::new(
                (self.target_delta.x * HOMING).clamp(-1.0, 1.0),
                (self.target_delta.y * HOMING).clamp(-1.0, 1.0),
                (self.target_delta.z * HOMING).clamp(-1.0, 1.0),
            );
            self.velocity += (self.target_delta - self.velocity) * STEER;
        } else {
            self.velocity.y -= GRAVITY;
        }
        let hit = self.hit_on_move(world, targets);
        self.position += self.velocity;
        self.rotate_towards_movement();
        if let Some(aim) = target {
            self.steer(world, aim, random);
        }
        hit
    }

    fn steer(&mut self, world: &dyn World, aim: Aim, random: &mut LegacyRandom) {
        if self.steps > 0 {
            self.steps -= 1;
            if self.steps == 0 {
                let axis = self.direction.map(axis_of);
                self.select_next_move_direction(world, axis, Some(aim), random);
            }
        }
        let Some(direction) = self.direction else {
            return;
        };
        let here = self.block();
        let axis = axis_of(direction);
        if world.sturdy_face(offset(here, direction), Face::Up) {
            self.select_next_move_direction(world, Some(axis), Some(aim), random);
            return;
        }
        let there = block_of(aim.position);
        let lined_up = match axis {
            Axis::X => here.0 == there.0,
            Axis::Y => here.1 == there.1,
            Axis::Z => here.2 == there.2,
        };
        if lined_up {
            self.select_next_move_direction(world, Some(axis), Some(aim), random);
        }
    }

    fn hit_on_move(&self, world: &dyn World, targets: &[BulletTarget]) -> Option<BulletHit> {
        let from = self.position;
        let movement = self.velocity;
        let mut to = from + movement;
        let block = sight::clip(world, from, to);
        if let Some((_, at)) = block {
            to = at;
        }
        let (min, max) = self.bounds();
        let (search_min, search_max) = sweep(min, max, movement);
        let search = (search_min - DVec3::ONE, search_max + DVec3::ONE);
        let grow = DVec3::splat(self.margin());
        let mut nearest = f64::MAX;
        let mut entity = None;
        for target in targets {
            if !overlaps((target.min, target.max), search) {
                continue;
            }
            if let Some(at) = sight::clip_box(target.min - grow, target.max + grow, from, to) {
                let distance = from.distance_squared(at);
                if distance < nearest {
                    nearest = distance;
                    entity = Some((target.id, at));
                }
            }
        }
        match (entity, block) {
            (Some((id, location)), _) => Some(BulletHit {
                location,
                entity: Some(id),
                block: None,
            }),
            (None, Some((pos, location))) => Some(BulletHit {
                location,
                entity: None,
                block: Some(pos),
            }),
            _ => None,
        }
    }
}

pub fn bounds_at(position: DVec3) -> (DVec3, DVec3) {
    let half = SIZE / 2.0;
    (
        DVec3::new(position.x - half, position.y, position.z - half),
        DVec3::new(position.x + half, position.y + SIZE, position.z + half),
    )
}

pub fn random_direction(random: &mut LegacyRandom) -> Face {
    DIRECTIONS[random.next_int(6) as usize]
}

pub fn closer_to_center_than(pos: Pos, position: DVec3, distance: f64) -> bool {
    let center = DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    );
    center.distance_squared(position) < distance * distance
}

fn offset(pos: Pos, face: Face) -> Pos {
    let (x, y, z) = face.offset();
    (pos.0 + x, pos.1 + y, pos.2 + z)
}

fn step(face: Face) -> DVec3 {
    let (x, y, z) = face.offset();
    DVec3::new(f64::from(x), f64::from(y), f64::from(z))
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

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::Block;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Blocks(HashMap<Pos, &'static str>);

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.0.get(&pos).map(|id| Block::new(*id))
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
        fn collision_boxes(&self, pos: Pos) -> Vec<[f64; 6]> {
            match self.0.get(&pos) {
                Some(_) => vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
                None => Vec::new(),
            }
        }
        fn min_y(&self) -> i32 {
            0
        }
    }

    fn floor(y: i32) -> Blocks {
        let mut blocks = Blocks::default();
        for x in -20..=20 {
            for z in -20..=20 {
                blocks.0.insert((x, y, z), "minecraft:end_stone");
            }
        }
        blocks
    }

    fn player(at: DVec3) -> Aim {
        Aim {
            position: at,
            height: 1.8,
        }
    }

    #[test]
    fn it_steps_along_one_axis_toward_the_target_but_never_along_its_shulker_s() {
        let world = Blocks::default();
        let at = DVec3::new(0.5, 10.5, 0.5);
        let target = player(DVec3::new(8.5, 15.0, 6.5));
        let mut chosen = Vec::new();
        for seed in 0..64 {
            let mut random = LegacyRandom::new(seed);
            let bullet =
                ShulkerBullet::shoot(&world, 1, at, Some((7, target)), Axis::Y, &mut random);
            let face = bullet.direction.expect("far from the target it steps");
            assert!(matches!(face, Face::East | Face::South), "{face:?}");
            assert!((bullet.target_delta - step(face) * SPEED).length() < 1e-12);
            assert!((10..=50).contains(&bullet.steps) && bullet.steps % 10 == 0);
            chosen.push(face);
        }
        assert!(chosen.contains(&Face::East) && chosen.contains(&Face::South));
        let mut bullet = ShulkerBullet::new(at);
        let mut ups = 0;
        for seed in 0..64 {
            let mut random = LegacyRandom::new(seed);
            bullet.select_next_move_direction(&world, None, Some(target), &mut random);
            ups += usize::from(bullet.direction == Some(Face::Up));
        }
        assert!(ups > 0, "without an axis kept out it may also climb");
    }

    #[test]
    fn near_the_target_it_aims_straight_at_it() {
        let world = Blocks::default();
        let target = player(DVec3::new(1.5, 10.0, 0.5));
        let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 10.5, 0.5));
        bullet.select_next_move_direction(
            &world,
            Some(Axis::Y),
            Some(target),
            &mut LegacyRandom::new(1),
        );
        assert_eq!(bullet.direction, None);
        let want = (DVec3::new(1.5, 10.9, 0.5) - bullet.position).normalize() * SPEED;
        assert!(
            (bullet.target_delta - want).length() < 1e-6,
            "{:?}",
            bullet.target_delta
        );
    }

    #[test]
    fn walled_in_it_picks_a_random_way_out() {
        let mut world = Blocks::default();
        for face in [Face::Up, Face::North, Face::South, Face::West, Face::East] {
            world
                .0
                .insert(offset((0, 10, 0), face), "minecraft:purpur_block");
        }
        let target = player(DVec3::new(9.5, 10.0, 0.5));
        let mut downs = 0;
        for seed in 0..40 {
            let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 10.5, 0.5));
            bullet.select_next_move_direction(
                &world,
                Some(Axis::Y),
                Some(target),
                &mut LegacyRandom::new(seed),
            );
            downs += usize::from(bullet.direction == Some(Face::Down));
            assert!(bullet.direction.is_some());
        }
        assert!(
            downs > 15,
            "it keeps drawing until it finds the open side: {downs}"
        );
    }

    #[test]
    fn it_homes_in_on_a_still_target_and_strikes_it() {
        let world = floor(9);
        let feet = DVec3::new(6.5, 10.0, 9.5);
        let aim = player(feet);
        let body = BulletTarget {
            id: 7,
            min: feet - DVec3::new(0.3, 0.0, 0.3),
            max: feet + DVec3::new(0.3, 1.8, 0.3),
        };
        for seed in 0..8 {
            let mut random = LegacyRandom::new(seed);
            let mut bullet = ShulkerBullet::shoot(
                &world,
                1,
                DVec3::new(0.5, 10.5, 0.5),
                Some((7, aim)),
                Axis::Y,
                &mut random,
            );
            let mut struck = None;
            for _ in 0..600 {
                if let Some(hit) = bullet.tick(&world, &[body], Some(aim), true, &mut random) {
                    struck = Some(hit);
                    break;
                }
            }
            let hit = struck.expect("it reaches the target");
            assert_eq!(hit.entity, Some(7), "seed {seed}: {hit:?}");
        }
    }

    #[test]
    fn it_turns_when_the_block_ahead_is_solid_or_it_lines_up() {
        let mut world = Blocks::default();
        world.0.insert((2, 10, 0), "minecraft:purpur_block");
        let aim = player(DVec3::new(9.5, 10.0, 6.5));
        let mut bullet = ShulkerBullet::new(DVec3::new(1.5, 10.5, 0.5));
        bullet.direction = Some(Face::East);
        bullet.steps = 30;
        bullet.steer(&world, aim, &mut LegacyRandom::new(5));
        assert_ne!(
            bullet.direction,
            Some(Face::East),
            "a solid block ahead turns it"
        );
        assert_ne!(bullet.direction.map(axis_of), Some(Axis::X));
        let world = Blocks::default();
        let mut bullet = ShulkerBullet::new(DVec3::new(9.5, 10.5, 0.5));
        bullet.direction = Some(Face::East);
        bullet.steps = 30;
        bullet.steer(&world, aim, &mut LegacyRandom::new(5));
        assert_eq!(
            bullet.direction,
            Some(Face::South),
            "in line with the target it turns toward it"
        );
        let mut bullet = ShulkerBullet::new(DVec3::new(3.5, 10.5, 0.5));
        bullet.direction = Some(Face::East);
        bullet.steps = 30;
        bullet.steer(&world, aim, &mut LegacyRandom::new(5));
        assert_eq!((bullet.direction, bullet.steps), (Some(Face::East), 29));
    }

    #[test]
    fn without_a_target_it_falls_and_pops_on_the_ground() {
        let world = floor(9);
        let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 14.0, 0.5));
        let mut random = LegacyRandom::new(1);
        bullet.tick(&world, &[], None, false, &mut random);
        assert!((bullet.velocity.y + GRAVITY).abs() < 1e-12);
        let mut hit = None;
        for _ in 0..40 {
            hit = bullet.tick(&world, &[], None, false, &mut random);
            if hit.is_some() {
                break;
            }
        }
        let hit = hit.expect("it lands");
        assert_eq!((hit.entity, hit.block), (None, Some((0, 9, 0))));
    }

    #[test]
    fn it_turns_halfway_toward_its_motion_each_tick_and_trails_behind() {
        let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 10.0, 0.5));
        bullet.velocity = DVec3::new(0.0, 0.0, 0.2);
        bullet.rotate_towards_movement();
        assert!((bullet.yaw - 270.0).abs() < 1e-3, "{}", bullet.yaw);
        let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 10.0, 0.5));
        bullet.velocity = DVec3::new(0.2, 0.0, 0.0);
        bullet.rotate_towards_movement();
        assert!((bullet.yaw - 45.0).abs() < 1e-3, "{}", bullet.yaw);
        assert!(bullet.pitch.abs() < 1e-3, "{}", bullet.pitch);
        bullet.yaw_o = bullet.yaw;
        bullet.rotate_towards_movement();
        assert!((bullet.yaw - 67.5).abs() < 1e-3, "{}", bullet.yaw);
        assert!((bullet.turn(0.5).0 - 56.25).abs() < 1e-3);
        bullet.velocity = DVec3::new(0.0, -0.3, 0.0);
        bullet.pitch_o = 0.0;
        bullet.rotate_towards_movement();
        assert!((bullet.pitch - 45.0).abs() < 1e-3, "{}", bullet.pitch);
        assert!((bullet.trail() - DVec3::new(0.5, 10.45, 0.5)).length() < 1e-9);
    }

    #[test]
    fn a_fresh_bullet_flies_out_of_its_own_shulker() {
        let world = Blocks::default();
        let shulker = BulletTarget {
            id: 1,
            min: DVec3::new(0.0, 10.0, 0.0),
            max: DVec3::new(1.0, 12.0, 1.0),
        };
        let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 11.0, 0.5));
        bullet.velocity = DVec3::new(0.15, 0.0, 0.0);
        bullet.target_delta = DVec3::new(0.15, 0.0, 0.0);
        let mut random = LegacyRandom::new(1);
        for _ in 0..20 {
            assert_eq!(
                bullet.tick(&world, &[shulker], None, true, &mut random),
                None
            );
        }
        assert!(bullet.position.x > 2.0);
    }
}
