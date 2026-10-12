use crate::health::DamageState;
use crate::look::{LookControl, rotate_towards};
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Face, Pos, World};

pub const MAX_HEALTH: f32 = 30.0;
pub const EYE_HEIGHT: f32 = 0.5;
pub const EXPERIENCE: i32 = 5;
pub const COVERED_ARMOR: f32 = 20.0;
pub const TELEPORT_STEPS: i32 = 6;
pub const NO_COLOR: u8 = 16;
pub const PEEK_PER_TICK: f32 = 0.05;
pub const AMBIENT_SOUND_INTERVAL: i32 = 120;
pub const DOES_NOT_TELEPORT_TO: &str = "minecraft:shulker_does_not_teleport_to";
pub const FACES: [Face; 6] = [
    Face::Down,
    Face::Up,
    Face::North,
    Face::South,
    Face::West,
    Face::East,
];

pub type Aabb = (DVec3, DVec3);

#[derive(Clone, Debug)]
pub struct Shulker {
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub attach_face: Face,
    pub raw_peek: i32,
    pub peek: f32,
    pub peek_o: f32,
    pub color: Option<u8>,
    pub covered: bool,
    pub teleport_from: Option<Pos>,
    pub teleport_ticks: i32,
    pub persistence_required: bool,
}

impl Shulker {
    pub fn new(position: DVec3) -> Self {
        let mut body = Body::new(snap(position), 1.0, 1.0);
        body.gravity = 0.0;
        Self {
            body,
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            attach_face: Face::Down,
            raw_peek: 0,
            peek: 0.0,
            peek_o: 0.0,
            color: None,
            covered: false,
            teleport_from: None,
            teleport_ticks: 0,
            persistence_required: false,
        }
    }

    pub fn block(&self) -> Pos {
        block_of(self.body.position)
    }

    pub fn opening(&self) -> Face {
        opposite(self.attach_face)
    }

    pub fn bounding_box(&self) -> Aabb {
        progress_box(self.opening(), physical_peek(self.peek), self.body.position)
    }

    pub fn closed(&self) -> bool {
        self.raw_peek == 0
    }

    pub fn eye_height(&self) -> f32 {
        EYE_HEIGHT * self.body.height
    }

    pub fn client_peek(&self, partial: f32) -> f32 {
        self.peek_o + partial * (self.peek - self.peek_o)
    }

    pub fn set_raw_peek(&mut self, amount: i32) -> &'static str {
        self.covered = amount == 0;
        self.raw_peek = amount;
        if amount == 0 {
            "entity.shulker.close"
        } else {
            "entity.shulker.open"
        }
    }

    pub fn update_peek(&mut self) -> bool {
        self.peek_o = self.peek;
        let target = self.raw_peek as f32 * 0.01;
        if self.peek == target {
            return false;
        }
        self.peek = if self.peek > target {
            mth_clamp(self.peek - PEEK_PER_TICK, target, 1.0)
        } else {
            mth_clamp(self.peek + PEEK_PER_TICK, 0.0, target)
        };
        true
    }

    pub fn refresh_dimensions(&mut self) {
        self.body.height = if self.attach_face == Face::Down && self.peek > 0.0 {
            1.0 + self.peek
        } else {
            1.0
        };
    }

    pub fn peek_growth(&self) -> Option<(Aabb, DVec3)> {
        let (now, before) = (physical_peek(self.peek), physical_peek(self.peek_o));
        let growth = now - before;
        if growth <= 0.0 {
            return None;
        }
        let opening = self.opening();
        let area = progress_delta_box(opening, before, now, self.body.position);
        Some((area, step(opening) * f64::from(growth)))
    }

    pub fn move_to(&mut self, to: Pos, face: Face) {
        let from = self.block();
        self.attach_face = face;
        self.body.position = bottom_centre(to);
        if to != from {
            self.raw_peek = 0;
            self.teleport_from = Some(from);
            self.teleport_ticks = TELEPORT_STEPS;
        }
        self.refresh_dimensions();
    }

    pub fn tick_teleport_lerp(&mut self) {
        if self.teleport_ticks > 0 {
            self.teleport_ticks -= 1;
        } else {
            self.teleport_from = None;
        }
    }

    pub fn render_offset(&self, partial: f32) -> DVec3 {
        let Some(from) = self.teleport_from.filter(|_| self.teleport_ticks > 0) else {
            return DVec3::ZERO;
        };
        let t = f64::from(self.teleport_ticks as f32 - partial) / f64::from(TELEPORT_STEPS);
        let to = self.block();
        let moved = DVec3::new(
            f64::from(to.0 - from.0),
            f64::from(to.1 - from.1),
            f64::from(to.2 - from.2),
        );
        -moved * (t * t)
    }
}

fn mth_clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min { min } else { value.min(max) }
}

pub fn snap(position: DVec3) -> DVec3 {
    DVec3::new(
        position.x.floor() + 0.5,
        (position.y + 0.5).floor(),
        position.z.floor() + 0.5,
    )
}

pub fn block_of(position: DVec3) -> Pos {
    (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

pub fn bottom_centre(pos: Pos) -> DVec3 {
    DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1),
        f64::from(pos.2) + 0.5,
    )
}

pub fn opposite(face: Face) -> Face {
    match face {
        Face::Down => Face::Up,
        Face::Up => Face::Down,
        Face::North => Face::South,
        Face::South => Face::North,
        Face::West => Face::East,
        Face::East => Face::West,
    }
}

pub fn step(face: Face) -> DVec3 {
    let (x, y, z) = face.offset();
    DVec3::new(f64::from(x), f64::from(y), f64::from(z))
}

pub fn face_id(face: Face) -> i8 {
    FACES.iter().position(|&f| f == face).unwrap_or(0) as i8
}

pub fn face_of_id(id: i64) -> Face {
    FACES[(id % 6).unsigned_abs() as usize]
}

fn offset(pos: Pos, face: Face) -> Pos {
    let (x, y, z) = face.offset();
    (pos.0 + x, pos.1 + y, pos.2 + z)
}

pub fn physical_peek(peek: f32) -> f32 {
    0.5 - minecraftoss_player::mth::sin(f64::from((0.5 + peek) * std::f32::consts::PI)) * 0.5
}

pub fn progress_delta_box(opening: Face, from: f32, to: f32, position: DVec3) -> Aabb {
    let max = f64::from(from.max(to));
    let min = f64::from(from.min(to));
    let s = step(opening);
    let (grow, shrink) = (s * max, -s * (1.0 + min));
    let mut lo = DVec3::new(-0.5, 0.0, -0.5);
    let mut hi = DVec3::new(0.5, 1.0, 0.5);
    for axis in 0..3 {
        if grow[axis] < 0.0 {
            lo[axis] += grow[axis];
        } else if grow[axis] > 0.0 {
            hi[axis] += grow[axis];
        }
        if shrink[axis] < 0.0 {
            lo[axis] -= shrink[axis];
        } else if shrink[axis] > 0.0 {
            hi[axis] -= shrink[axis];
        }
    }
    (lo + position, hi + position)
}

pub fn progress_box(opening: Face, peek: f32, position: DVec3) -> Aabb {
    progress_delta_box(opening, -1.0, peek, position)
}

pub fn overlaps(a: Aabb, b: Aabb) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
}

fn deflated((lo, hi): Aabb) -> Aabb {
    (lo + DVec3::splat(1.0e-6), hi - DVec3::splat(1.0e-6))
}

pub fn is_air(world: &dyn World, pos: Pos) -> bool {
    world.block(pos).is_none_or(|block| {
        matches!(
            block.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

pub fn no_collision(world: &dyn World, area: Aabb, others: &[Aabb]) -> bool {
    if others.iter().any(|&other| overlaps(area, other)) {
        return false;
    }
    let (lo, hi) = area;
    for x in lo.x.floor() as i32..=hi.x.floor() as i32 {
        for y in lo.y.floor() as i32 - 1..=hi.y.floor() as i32 {
            for z in lo.z.floor() as i32..=hi.z.floor() as i32 {
                let cell = DVec3::new(f64::from(x), f64::from(y), f64::from(z));
                for b in world.collision_boxes((x, y, z)) {
                    let solid = (
                        cell + DVec3::new(b[0], b[1], b[2]),
                        cell + DVec3::new(b[3], b[4], b[5]),
                    );
                    if overlaps(area, solid) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

fn blocked(world: &dyn World, pos: Pos, own: Pos) -> bool {
    if is_air(world, pos) {
        return false;
    }
    let moving_piston = world
        .block(pos)
        .is_some_and(|block| block.id == "minecraft:moving_piston");
    !(moving_piston && pos == own)
}

pub fn can_stay_at(world: &dyn World, pos: Pos, face: Face, own: Pos, others: &[Aabb]) -> bool {
    if blocked(world, pos, own) {
        return false;
    }
    if !world.sturdy_face(offset(pos, face), opposite(face)) {
        return false;
    }
    let open = progress_box(opposite(face), 1.0, bottom_centre(pos));
    no_collision(world, deflated(open), others)
}

pub fn find_attachable_surface(
    world: &dyn World,
    pos: Pos,
    own: Pos,
    others: &[Aabb],
) -> Option<Face> {
    FACES
        .into_iter()
        .find(|&face| can_stay_at(world, pos, face, own, others))
}

pub fn teleport_somewhere(
    world: &dyn World,
    random: &mut LegacyRandom,
    from: Pos,
    others: &[Aabb],
) -> Option<(Pos, Face)> {
    for _ in 0..5 {
        let dx = random.next_int(17) as i32 - 8;
        let dy = random.next_int(17) as i32 - 8;
        let dz = random.next_int(17) as i32 - 8;
        let to = (from.0 + dx, from.1 + dy, from.2 + dz);
        if to.1 <= world.min_y() || !is_air(world, to) {
            continue;
        }
        let cell = bottom_centre(to);
        let cube = (
            cell - DVec3::new(0.5, 0.0, 0.5),
            cell + DVec3::new(0.5, 1.0, 0.5),
        );
        if !no_collision(world, deflated(cube), others) {
            continue;
        }
        let Some(face) = find_attachable_surface(world, to, from, others) else {
            continue;
        };
        if world.block_in_tag(offset(to, face), DOES_NOT_TELEPORT_TO) {
            continue;
        }
        return Some((to, face));
    }
    None
}

pub fn look_axes(opening: Face) -> ([f32; 3], [f32; 3]) {
    match opening {
        Face::Up => ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        Face::Down => ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
        Face::North => ([0.0, -1.0, 0.0], [-1.0, 0.0, 0.0]),
        Face::South => ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
        Face::West => ([0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
        Face::East => ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
    }
}

pub fn head_yaw_toward(opening: Face, delta: DVec3) -> Option<f32> {
    let (forward, right) = look_axes(opening);
    let d = [delta.x as f32, delta.y as f32, delta.z as f32];
    let dot = |v: [f32; 3]| v[0] * d[0] + v[1] * d[1] + v[2] * d[2];
    let (across, along) = (dot(right), dot(forward));
    if across.abs() <= 1.0e-5 && along.abs() <= 1.0e-5 {
        return None;
    }
    let angle = minecraftoss_player::mth::atan2(f64::from(-across), f64::from(along));
    Some((angle * 57.2957763671875_f64) as f32)
}

pub fn look_tick(look: &mut LookControl, opening: Face, position: DVec3, eye_height: f32) {
    look.pitch = 0.0;
    if look.cooldown > 0 {
        look.cooldown -= 1;
        let eye = position + DVec3::new(0.0, f64::from(eye_height), 0.0);
        if let Some(yaw) = head_yaw_toward(opening, look.wanted - eye) {
            look.head_yaw = rotate_towards(look.head_yaw, yaw, look.y_max_rot_speed);
        }
    } else {
        look.head_yaw = rotate_towards(look.head_yaw, 0.0, 10.0);
    }
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
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => {
                    let id: &'static str = Box::leak(block.id.into_boxed_str());
                    self.0.insert(pos, id);
                }
                None => {
                    self.0.remove(&pos);
                }
            }
        }
        fn collision_boxes(&self, pos: Pos) -> Vec<[f64; 6]> {
            match self.0.get(&pos) {
                Some(&"minecraft:water") | None => Vec::new(),
                Some(_) => vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
            }
        }
        fn min_y(&self) -> i32 {
            0
        }
    }

    fn close(a: Aabb, b: Aabb) -> bool {
        (a.0 - b.0).length() < 1e-6 && (a.1 - b.1).length() < 1e-6
    }

    #[test]
    fn the_physical_peek_eases_in_and_out() {
        assert!(physical_peek(0.0).abs() < 1e-6);
        assert!((physical_peek(0.3) - 0.2061).abs() < 1e-3);
        assert!((physical_peek(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_box_stretches_toward_the_opening_side() {
        let at = DVec3::new(0.5, 10.0, 0.5);
        for face in FACES {
            assert!(
                close(
                    progress_box(face, 0.0, at),
                    (DVec3::new(0.0, 10.0, 0.0), DVec3::new(1.0, 11.0, 1.0))
                ),
                "{face:?} closed is the block"
            );
        }
        let open = |face| progress_box(face, 1.0, at);
        assert!(close(
            open(Face::Up),
            (DVec3::new(0.0, 10.0, 0.0), DVec3::new(1.0, 12.0, 1.0))
        ));
        assert!(close(
            open(Face::Down),
            (DVec3::new(0.0, 9.0, 0.0), DVec3::new(1.0, 11.0, 1.0))
        ));
        assert!(close(
            open(Face::North),
            (DVec3::new(0.0, 10.0, -1.0), DVec3::new(1.0, 11.0, 1.0))
        ));
        assert!(close(
            open(Face::South),
            (DVec3::new(0.0, 10.0, 0.0), DVec3::new(1.0, 11.0, 2.0))
        ));
        assert!(close(
            open(Face::West),
            (DVec3::new(-1.0, 10.0, 0.0), DVec3::new(1.0, 11.0, 1.0))
        ));
        assert!(close(
            open(Face::East),
            (DVec3::new(0.0, 10.0, 0.0), DVec3::new(2.0, 11.0, 1.0))
        ));
        assert!(close(
            progress_delta_box(Face::Up, 0.0, 0.25, at),
            (DVec3::new(0.0, 11.0, 0.0), DVec3::new(1.0, 11.25, 1.0))
        ));
    }

    #[test]
    fn a_shulker_clings_to_a_sturdy_face_with_room_to_open() {
        let mut world = Blocks::default();
        world.0.insert((0, 9, 0), "minecraft:end_stone");
        world.0.insert((1, 10, 0), "minecraft:purpur_block");
        assert!(can_stay_at(&world, (0, 10, 0), Face::Down, (0, 10, 0), &[]));
        assert_eq!(
            find_attachable_surface(&world, (0, 10, 0), (0, 10, 0), &[]),
            Some(Face::Down)
        );
        world.0.remove(&(0, 9, 0));
        assert_eq!(
            find_attachable_surface(&world, (0, 10, 0), (0, 10, 0), &[]),
            Some(Face::East),
            "with the floor gone it clings to the wall"
        );
        world.0.insert((0, 9, 0), "minecraft:end_stone");
        world.0.insert((0, 11, 0), "minecraft:end_stone");
        assert!(
            !can_stay_at(&world, (0, 10, 0), Face::Down, (0, 10, 0), &[]),
            "a block in front of its lid"
        );
        world.0.remove(&(0, 11, 0));
        world.0.insert((0, 10, 0), "minecraft:water");
        assert!(
            !can_stay_at(&world, (0, 10, 0), Face::Down, (0, 10, 0), &[]),
            "water in its cell"
        );
        world.0.remove(&(0, 10, 0));
        let other = progress_box(Face::Up, 0.0, DVec3::new(0.5, 11.0, 0.5));
        assert!(
            !can_stay_at(&world, (0, 10, 0), Face::Down, (0, 10, 0), &[other]),
            "another shulker sits where its lid opens"
        );
        world.0.insert((0, 9, 0), "minecraft:oak_leaves");
        world.0.remove(&(1, 10, 0));
        assert_eq!(
            find_attachable_surface(&world, (0, 10, 0), (0, 10, 0), &[]),
            None,
            "leaves are no sturdy face"
        );
    }

    #[test]
    fn teleports_land_within_eight_blocks_on_air_beside_a_sturdy_face() {
        let mut world = Blocks::default();
        for x in -12..=12 {
            for z in -12..=12 {
                world.0.insert((x, 19, z), "minecraft:end_stone");
            }
        }
        let mut random = LegacyRandom::new(7);
        let mut landed = 0;
        for _ in 0..200 {
            if let Some((to, face)) = teleport_somewhere(&world, &mut random, (0, 20, 0), &[]) {
                landed += 1;
                assert!((to.0).abs() <= 8 && (to.1 - 20).abs() <= 8 && (to.2).abs() <= 8);
                assert!(is_air(&world, to));
                assert!(
                    matches!((to.1, face), (20, Face::Down) | (18, Face::Up)),
                    "on the floor or under it: {to:?} {face:?}"
                );
            }
        }
        assert!(landed > 20, "{landed}");
        let mut bedrock = Blocks::default();
        for x in -12..=12 {
            for z in -12..=12 {
                bedrock.0.insert((x, 19, z), "minecraft:bedrock");
            }
        }
        let mut random = LegacyRandom::new(7);
        for _ in 0..200 {
            assert_eq!(
                teleport_somewhere(&bedrock, &mut random, (0, 20, 0), &[]),
                None,
                "never onto bedrock"
            );
        }
    }

    #[test]
    fn the_peek_moves_a_twentieth_a_tick_and_a_floor_shulker_grows() {
        let mut shulker = Shulker::new(DVec3::new(3.2, 9.7, -1.4));
        assert_eq!(shulker.body.position, DVec3::new(3.5, 10.0, -1.5));
        shulker.set_raw_peek(30);
        let mut ticks = 0;
        while shulker.update_peek() {
            ticks += 1;
            shulker.refresh_dimensions();
        }
        assert_eq!(ticks, 6);
        assert!((shulker.peek - 0.3).abs() < 1e-6);
        assert!((shulker.body.height - 1.3).abs() < 1e-6);
        assert!(!shulker.covered);
        assert_eq!(shulker.set_raw_peek(0), "entity.shulker.close");
        assert!(shulker.covered);
        shulker.raw_peek = 127;
        shulker.update_peek();
        assert!(
            (shulker.peek - 0.35).abs() < 1e-6,
            "a raw peek past 100 caps at 1"
        );
    }

    #[test]
    fn a_teleport_draws_from_the_old_block_over_six_ticks() {
        let mut shulker = Shulker::new(DVec3::new(0.5, 10.0, 0.5));
        shulker.raw_peek = 30;
        shulker.move_to((3, 12, 0), Face::West);
        assert_eq!((shulker.raw_peek, shulker.attach_face), (0, Face::West));
        assert_eq!(shulker.body.position, DVec3::new(3.5, 12.0, 0.5));
        assert_eq!(shulker.render_offset(0.0), DVec3::new(-3.0, -2.0, 0.0));
        for _ in 0..3 {
            shulker.tick_teleport_lerp();
        }
        assert!((shulker.render_offset(0.0) - DVec3::new(-0.75, -0.5, 0.0)).length() < 1e-9);
        for _ in 0..4 {
            shulker.tick_teleport_lerp();
        }
        assert_eq!(shulker.render_offset(0.5), DVec3::ZERO);
        assert_eq!(shulker.teleport_from, None);
    }

    #[test]
    fn the_head_turns_in_the_plane_of_its_face() {
        let floor = |d: DVec3| head_yaw_toward(Face::Up, d).unwrap();
        assert!(floor(DVec3::new(0.0, 0.0, 1.0)).abs() < 1e-3);
        assert!((floor(DVec3::new(1.0, 0.0, 0.0)) + 90.0).abs() < 1e-3);
        assert!((floor(DVec3::new(-1.0, 5.0, 0.0)) - 90.0).abs() < 1e-3);
        assert_eq!(head_yaw_toward(Face::Up, DVec3::new(0.0, 3.0, 0.0)), None);
        let wall = |d: DVec3| head_yaw_toward(Face::East, d).unwrap();
        assert!(wall(DVec3::new(0.0, -1.0, 0.0)).abs() < 1e-3);
        assert!((wall(DVec3::new(0.0, 0.0, -1.0)) + 90.0).abs() < 1e-3);
        let mut look = LookControl::new(0.0);
        look.set_look_at_with_limits(DVec3::new(10.5, 10.5, 0.5), 10.0, 180.0);
        look_tick(&mut look, Face::Up, DVec3::new(0.5, 10.0, 0.5), EYE_HEIGHT);
        assert!((look.head_yaw + 10.0).abs() < 1e-3, "{}", look.head_yaw);
        look.cooldown = 0;
        look_tick(&mut look, Face::Up, DVec3::new(0.5, 10.0, 0.5), EYE_HEIGHT);
        assert!(look.head_yaw.abs() < 1e-3, "back toward the body");
    }
}
