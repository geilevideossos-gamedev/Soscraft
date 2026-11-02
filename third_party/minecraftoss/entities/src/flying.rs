use crate::control::{MoveControl, MoveOperation, rotlerp};
use crate::fluid::FluidFrame;
use crate::movement::Body;
use crate::navigation::{GroundNavigation, PlannedPath, adopt_path};
use crate::path_search::NodePos;
use crate::walk_path::{WalkProfile, Walker, find_fly_path_to_any};
use glam::DVec3;
use minecraftoss_player::World;

pub const NEAR_WANTED_SQR: f64 = 2.500000277905201e-7;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stable {
    #[default]
    StandOn,
    NotAboveAir,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flight {
    pub stable: Stable,
    pub blocked_by_fluids: bool,
}

impl Default for Flight {
    fn default() -> Self {
        Self {
            stable: Stable::StandOn,
            blocked_by_fluids: true,
        }
    }
}

impl Flight {
    pub fn is_stable_destination(self, world: &dyn World, (x, y, z): NodePos) -> bool {
        match self.stable {
            Stable::StandOn => top_face_full(&world.collision_boxes((x, y, z))),
            Stable::NotAboveAir => world.block((x, y - 1, z)).is_some_and(|b| {
                !matches!(
                    b.id.as_str(),
                    "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
                )
            }),
        }
    }

    pub fn can_move_directly(self, world: &dyn World, from: DVec3, to: DVec3, height: f32) -> bool {
        is_clear_for_movement_between(world, from, to, height, self.blocked_by_fluids)
    }
}

pub fn fly(profile: &mut WalkProfile, flight: Flight) {
    profile.flying = Some(flight);
}

pub fn require_path_length(profile: &mut WalkProfile, length: f32) {
    let longest = profile
        .max_path_length
        .unwrap_or(profile.follow_range)
        .max(length);
    profile.max_path_length = Some(longest);
    profile.max_visited = Some((longest * 16.0).floor() as i32);
}

pub fn can_update_path(profile: &WalkProfile, fluid: FluidFrame, passenger: bool) -> bool {
    (profile.can_float && (fluid.in_water() || fluid.in_lava())) || !passenger
}

pub fn plan_path<W: World + ?Sized>(
    body: &Body,
    navigation: &mut GroundNavigation,
    world: &W,
    profile: &WalkProfile,
    fluid: FluidFrame,
    targets: &[NodePos],
    reach_range: i32,
) -> Option<PlannedPath> {
    if targets.is_empty() || body.position.y < f64::from(world.min_y()) {
        return None;
    }
    if !navigation.is_done() && navigation.target_pos.is_some_and(|t| targets.contains(&t)) {
        return Some(PlannedPath::of(navigation));
    }
    let walker = Walker {
        position: body.position,
        on_ground: body.on_ground,
        in_floatable_fluid: fluid.in_water(),
    };
    let random = &mut navigation.fly_random;
    let path = find_fly_path_to_any(world, profile, walker, targets, reach_range, random)?;
    Some(adopt_path(navigation, path, reach_range))
}

pub fn tick_path(
    navigation: &mut GroundNavigation,
    world: &dyn World,
    body: &Body,
    profile: &WalkProfile,
    fluid: FluidFrame,
    passenger: bool,
    speed: f32,
) -> Option<(DVec3, f64)> {
    let flight = profile.flying.unwrap_or_default();
    let can_update = can_update_path(profile, fluid, passenger);
    let (position, height) = (body.position, body.height);
    navigation.tick_free(position, body.width, can_update, speed, |to| {
        flight.can_move_directly(world, position, to, height)
    })
}

pub fn is_clear_for_movement_between(
    world: &dyn World,
    from: DVec3,
    to: DVec3,
    height: f32,
    blocked_by_fluids: bool,
) -> bool {
    let to = DVec3::new(to.x, to.y + f64::from(height) * 0.5, to.z);
    !crate::sight::blocked(world, from, to)
        && !(blocked_by_fluids && fluid_in_the_way(world, from, to))
}

fn top_face_full(boxes: &[[f64; 6]]) -> bool {
    let cut = 1.0 - 1.0e-7;
    let slice: Vec<[f64; 6]> = boxes
        .iter()
        .copied()
        .filter(|b| b[1] < cut && b[4] > cut)
        .collect();
    let mut xs = vec![0.0, 1.0];
    let mut zs = vec![0.0, 1.0];
    for b in &slice {
        xs.extend([b[0].clamp(0.0, 1.0), b[3].clamp(0.0, 1.0)]);
        zs.extend([b[2].clamp(0.0, 1.0), b[5].clamp(0.0, 1.0)]);
    }
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    zs.sort_by(f64::total_cmp);
    zs.dedup();
    xs.windows(2).all(|x| {
        zs.windows(2).all(|z| {
            let (cx, cz) = ((x[0] + x[1]) / 2.0, (z[0] + z[1]) / 2.0);
            slice
                .iter()
                .any(|b| b[0] <= cx && cx <= b[3] && b[2] <= cz && cz <= b[5])
        })
    })
}

fn fluid_height(world: &dyn World, pos: NodePos) -> Option<f64> {
    let block = world.block(pos)?;
    let water = world.floatable_fluid(pos);
    if !water && !world.fluid_is_lava(pos) {
        return None;
    }
    let level = if block.property("waterlogged") == Some("true") {
        0
    } else {
        block
            .property("level")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let above = (pos.0, pos.1 + 1, pos.2);
    let same_above = if water {
        world.floatable_fluid(above)
    } else {
        world.fluid_is_lava(above)
    };
    let amount = if level == 0 || level >= 8 {
        8
    } else {
        8 - level
    };
    Some(if same_above {
        1.0
    } else {
        f64::from(amount as f32 / 9.0)
    })
}

fn fluid_hit(world: &dyn World, pos: NodePos, from: DVec3, to: DVec3) -> bool {
    let Some(height) = fluid_height(world, pos) else {
        return false;
    };
    let d = to - from;
    if d.length_squared() < 1.0e-7 {
        return false;
    }
    let origin = DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    let test = DVec3::new(
        from.x + d.x * 0.001,
        from.y + d.y * 0.001,
        from.z + d.z * 0.001,
    ) - origin;
    if (0.0..1.0).contains(&test.x)
        && (0.0..height).contains(&test.y)
        && (0.0..1.0).contains(&test.z)
    {
        return true;
    }
    let top = origin + DVec3::new(1.0, height, 1.0);
    crate::sight::clip_box(origin, top, from, to).is_some()
}

fn fluid_in_the_way(world: &dyn World, from: DVec3, to: DVec3) -> bool {
    if from == to {
        return false;
    }
    let lerp = |t: f64, a: f64, b: f64| a + t * (b - a);
    let end = DVec3::new(
        lerp(-1.0e-7, to.x, from.x),
        lerp(-1.0e-7, to.y, from.y),
        lerp(-1.0e-7, to.z, from.z),
    );
    let start = DVec3::new(
        lerp(-1.0e-7, from.x, to.x),
        lerp(-1.0e-7, from.y, to.y),
        lerp(-1.0e-7, from.z, to.z),
    );
    let mut block = (
        start.x.floor() as i32,
        start.y.floor() as i32,
        start.z.floor() as i32,
    );
    if fluid_hit(world, block, from, to) {
        return true;
    }
    let d = end - start;
    let sign = |v: f64| {
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let (sx, sy, sz) = (sign(d.x), sign(d.y), sign(d.z));
    let delta = |s: i32, v: f64| if s == 0 { f64::MAX } else { f64::from(s) / v };
    let (dx, dy, dz) = (delta(sx, d.x), delta(sy, d.y), delta(sz, d.z));
    let frac = |v: f64| v - v.floor();
    let first = |s: i32, v: f64, t: f64| t * if s > 0 { 1.0 - frac(v) } else { frac(v) };
    let (mut tx, mut ty, mut tz) = (
        first(sx, start.x, dx),
        first(sy, start.y, dy),
        first(sz, start.z, dz),
    );
    while tx <= 1.0 || ty <= 1.0 || tz <= 1.0 {
        if tx < ty {
            if tx < tz {
                block.0 += sx;
                tx += dx;
            } else {
                block.2 += sz;
                tz += dz;
            }
        } else if ty < tz {
            block.1 += sy;
            ty += dy;
        } else {
            block.2 += sz;
            tz += dz;
        }
        if fluid_hit(world, block, from, to) {
            return true;
        }
    }
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlyingMoveControl {
    pub max_turn: i32,
    pub hovers_in_place: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlyingMove {
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub forward: f32,
    pub vertical: f32,
    pub no_gravity: bool,
}

fn set_x_rot(pitch: f32) -> f32 {
    (pitch % 360.0).clamp(-90.0, 90.0)
}

impl FlyingMoveControl {
    pub const fn new(max_turn: i32, hovers_in_place: bool) -> Self {
        Self {
            max_turn,
            hovers_in_place,
        }
    }

    pub fn tick(
        self,
        control: &mut MoveControl,
        now: FlyingMove,
        position: DVec3,
        on_ground: bool,
        movement_speed: f64,
        flying_speed: f64,
    ) -> FlyingMove {
        let mut next = now;
        if control.operation != MoveOperation::MoveTo {
            if !self.hovers_in_place {
                next.no_gravity = false;
            }
            next.vertical = 0.0;
            next.forward = 0.0;
            return next;
        }
        control.operation = MoveOperation::Wait;
        next.no_gravity = true;
        let d = control.wanted - position;
        if d.x * d.x + d.y * d.y + d.z * d.z < NEAR_WANTED_SQR {
            next.vertical = 0.0;
            next.forward = 0.0;
            return next;
        }
        let turn = (minecraftoss_player::mth::atan2(d.z, d.x) * 57.295_776_367_187_5) as f32 - 90.0;
        next.yaw = rotlerp(now.yaw, turn, 90.0);
        let attribute = if on_ground {
            movement_speed
        } else {
            flying_speed
        };
        next.speed = (control.speed_modifier * attribute) as f32;
        next.forward = next.speed;
        let horizontal = (d.x * d.x + d.z * d.z).sqrt();
        let least = f64::from(1.0e-5_f32);
        if d.y.abs() > least || horizontal.abs() > least {
            let pitch =
                (-(minecraftoss_player::mth::atan2(d.y, horizontal) * 57.295_776_367_187_5)) as f32;
            next.pitch = set_x_rot(rotlerp(now.pitch, pitch, self.max_turn as f32));
            next.vertical = if d.y > 0.0 { next.speed } else { -next.speed };
        }
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain_behaviors::{MoveSink, Track, WalkGoal};
    use crate::navigation::{navigate_walk_to, plan_walk_path};
    use crate::villager_brain::Slot;
    use minecraftoss_player::Block;
    use minecraftoss_player::rng::LegacyRandom;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Scene(BTreeMap<NodePos, Block>);

    impl World for Scene {
        fn block(&self, pos: NodePos) -> Option<Block> {
            self.0.get(&pos).cloned()
        }

        fn set_block(&mut self, pos: NodePos, block: Option<Block>) {
            match block {
                Some(block) => self.0.insert(pos, block),
                None => self.0.remove(&pos),
            };
        }
    }

    impl Scene {
        fn fill(&mut self, low: NodePos, high: NodePos, id: &str) {
            for x in low.0..=high.0 {
                for y in low.1..=high.1 {
                    for z in low.2..=high.2 {
                        self.0.insert((x, y, z), Block::new(id));
                    }
                }
            }
        }
    }

    fn flyer(width: f32, height: f32) -> WalkProfile {
        let mut profile = WalkProfile::animal(width, height);
        fly(&mut profile, Flight::default());
        profile
    }

    fn floor_and_wall() -> Scene {
        let mut scene = Scene::default();
        scene.fill((-3, 63, -9), (13, 63, 9), "minecraft:stone");
        scene.fill((5, 64, -9), (5, 66, 9), "minecraft:stone");
        scene
    }

    fn wall_with_a_gap() -> Scene {
        let mut scene = Scene::default();
        scene.fill((-3, 63, -9), (13, 63, 9), "minecraft:stone");
        scene.fill((-3, 71, -9), (13, 71, 9), "minecraft:stone");
        scene.fill((5, 64, -9), (5, 70, 9), "minecraft:stone");
        scene.0.remove(&(5, 66, 0));
        scene
    }

    fn plan(
        scene: &Scene,
        profile: &WalkProfile,
        from: DVec3,
        to: DVec3,
    ) -> (GroundNavigation, PlannedPath) {
        let body = Body::new(from, profile.width, profile.height);
        let mut navigation = GroundNavigation::default();
        let path = plan_walk_path(
            &body,
            &mut navigation,
            scene,
            profile,
            FluidFrame::default(),
            to,
            0,
        )
        .expect("a flyer plans in the air");
        (navigation, path)
    }

    fn steps_are_three_dimensional_neighbours(nodes: &[NodePos]) {
        for pair in nodes.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let step = ((b.0 - a.0).abs(), (b.1 - a.1).abs(), (b.2 - a.2).abs());
            assert!(
                step.0 <= 1 && step.1 <= 1 && step.2 <= 1 && step != (0, 0, 0),
                "{a:?} to {b:?}"
            );
        }
    }

    #[test]
    fn a_flying_path_goes_over_a_wall_and_through_a_gap_in_three_dimensions() {
        let profile = flyer(0.5, 0.9);
        let mut bee = profile.clone();
        require_path_length(&mut bee, 48.0);
        let (from, to) = (DVec3::new(0.5, 64.0, 0.5), DVec3::new(10.5, 64.0, 0.5));
        let scene = floor_and_wall();
        let (navigation, path) = plan(&scene, &bee, from, to);
        let nodes = path.nodes();
        assert!(path.reached());
        assert_eq!(nodes.first(), Some(&(0, 64, 0)));
        assert_eq!(nodes.last(), Some(&(10, 64, 0)));
        assert_eq!(navigation.target_pos, Some((10, 64, 0)));
        steps_are_three_dimensional_neighbours(nodes);
        assert!(nodes.iter().all(|&(x, y, _)| x != 5 || y >= 67));
        assert!(nodes.iter().any(|&(_, y, _)| y >= 67));
        assert!(
            nodes
                .windows(2)
                .any(|p| p[0].1 != p[1].1 && (p[0].0 != p[1].0 || p[0].2 != p[1].2))
        );
        let scene = wall_with_a_gap();
        let (_, path) = plan(&scene, &profile, from, to);
        let nodes = path.nodes();
        assert!(path.reached());
        assert_eq!(nodes.last(), Some(&(10, 64, 0)));
        steps_are_three_dimensional_neighbours(nodes);
        assert!(nodes.contains(&(5, 66, 0)), "{nodes:?}");
        assert!(nodes.iter().all(|&(x, y, z)| x != 5 || (y, z) == (66, 0)));
        let walker = Body::new(from, profile.width, profile.height);
        let mut ground = GroundNavigation::default();
        let walk = WalkProfile::animal(0.5, 0.9);
        let fluid = FluidFrame::default();
        assert!(plan_walk_path(&walker, &mut ground, &scene, &walk, fluid, to, 0).is_none());
    }

    #[test]
    fn a_flying_path_prefers_the_air_to_the_ground_and_keeps_off_fire() {
        let mut scene = Scene::default();
        scene.fill((-3, 63, -3), (12, 63, 3), "minecraft:stone");
        scene.0.insert((3, 64, 0), Block::new("minecraft:fire"));
        let mob = (0, 64, 0);
        assert_eq!(
            crate::walk_path::fly_path_type(&scene, (1, 64, 0), mob),
            minecraftoss_player::path_type::PathType::Walkable
        );
        assert_eq!(
            crate::walk_path::fly_path_type(&scene, (1, 65, 0), mob),
            minecraftoss_player::path_type::PathType::Open
        );
        assert_eq!(
            crate::walk_path::fly_path_type(&scene, (3, 65, 0), mob),
            minecraftoss_player::path_type::PathType::Fire
        );
        assert_eq!(
            crate::walk_path::fly_path_type(&scene, (2, 64, 0), mob),
            minecraftoss_player::path_type::PathType::FireInNeighbor
        );
        let profile = flyer(0.5, 0.9);
        let (_, path) = plan(
            &scene,
            &profile,
            DVec3::new(0.5, 64.0, 0.5),
            DVec3::new(8.5, 64.0, 0.5),
        );
        let nodes = path.nodes();
        assert!(path.reached());
        assert!(!nodes.contains(&(3, 65, 0)) && !nodes.contains(&(3, 64, 0)));
        assert!(nodes[1..nodes.len() - 1].iter().any(|&(_, y, _)| y >= 65));
    }

    #[test]
    fn a_flying_path_ends_on_a_stable_spot() {
        let mut scene = Scene::default();
        scene.fill((-12, 63, -12), (12, 63, 12), "minecraft:stone");
        scene.0.insert((0, 100, 0), Block::new("minecraft:chest"));
        scene
            .0
            .insert((1, 100, 0), Block::new("minecraft:stone_slab"));
        let top_slab = Block::new("minecraft:stone_slab").with("type", "top");
        scene.0.insert((2, 100, 0), top_slab);
        scene
            .0
            .insert((3, 100, 0), Block::new("minecraft:oak_stairs"));
        let upside_down = Block::new("minecraft:oak_stairs").with("half", "top");
        scene.0.insert((4, 100, 0), upside_down);
        let stand_on = Flight::default();
        assert!(stand_on.is_stable_destination(&scene, (0, 63, 0)));
        assert!(!stand_on.is_stable_destination(&scene, (0, 64, 0)));
        assert!(!stand_on.is_stable_destination(&scene, (0, 100, 0)));
        assert!(!stand_on.is_stable_destination(&scene, (1, 100, 0)));
        assert!(stand_on.is_stable_destination(&scene, (2, 100, 0)));
        assert!(!stand_on.is_stable_destination(&scene, (3, 100, 0)));
        assert!(stand_on.is_stable_destination(&scene, (4, 100, 0)));
        let bee = Flight {
            stable: Stable::NotAboveAir,
            ..Flight::default()
        };
        assert!(bee.is_stable_destination(&scene, (0, 64, 0)));
        assert!(bee.is_stable_destination(&scene, (1, 101, 0)));
        assert!(!bee.is_stable_destination(&scene, (0, 66, 0)));
        let mut profile = WalkProfile::animal(0.55, 0.5);
        fly(&mut profile, bee);
        let position = DVec3::new(0.5, 66.0, 0.5);
        let mut found = 0;
        for seed in 0..200 {
            let mut random = LegacyRandom::new(seed);
            let spot = crate::stroll::default_random_position(
                &scene,
                &profile,
                position,
                10,
                7,
                &mut random,
            );
            if let Some(spot) = spot {
                found += 1;
                let block = (
                    spot.x.floor() as i32,
                    spot.y.floor() as i32,
                    spot.z.floor() as i32,
                );
                assert!(bee.is_stable_destination(&scene, block), "{spot:?}");
                assert_eq!(block.1, 64, "{spot:?}");
            }
        }
        assert!(found > 50, "{found}");
        let mut profile = WalkProfile::animal(0.5, 0.9);
        fly(&mut profile, stand_on);
        for seed in 0..50 {
            let mut random = LegacyRandom::new(seed);
            let spot = crate::stroll::default_random_position(
                &scene,
                &profile,
                position,
                10,
                7,
                &mut random,
            );
            assert_eq!(spot, None);
        }
    }

    #[test]
    fn flying_move_control_turns_and_climbs_like_vanilla() {
        let position = DVec3::new(0.0, 64.0, 0.0);
        let now = FlyingMove {
            pitch: 30.0,
            ..FlyingMove::default()
        };
        let climb = |max_turn: i32, now: FlyingMove, on_ground: bool| {
            let mut control = MoveControl::default();
            control.set_wanted_position(position + DVec3::new(0.0, 5.0, -5.0), 1.5);
            let moved = FlyingMoveControl::new(max_turn, false).tick(
                &mut control,
                now,
                position,
                on_ground,
                0.2,
                0.4,
            );
            assert_eq!(control.operation, MoveOperation::Wait);
            moved
        };
        let parrot = climb(10, now, false);
        assert_eq!(parrot.yaw, 270.0);
        assert_eq!(parrot.pitch, 20.0);
        assert_eq!(parrot.speed, (1.5_f64 * 0.4) as f32);
        assert_eq!(parrot.forward, parrot.speed);
        assert_eq!(parrot.vertical, parrot.speed);
        assert!(parrot.no_gravity);
        let bee = climb(20, now, false);
        assert_eq!(bee.pitch, 10.0);
        let grounded = climb(10, now, true);
        assert_eq!(grounded.speed, (1.5_f64 * 0.2) as f32);
        let level = FlyingMove {
            pitch: 5.0,
            ..FlyingMove::default()
        };
        assert_eq!(climb(10, level, false).pitch, 90.0);
        let mut control = MoveControl::default();
        control.set_wanted_position(position + DVec3::new(3.0, -2.0, 0.0), 1.0);
        let dive = FlyingMoveControl::new(10, false).tick(
            &mut control,
            FlyingMove::default(),
            position,
            false,
            0.2,
            0.4,
        );
        assert_eq!(dive.vertical, -dive.speed);
        assert_eq!(dive.yaw, 270.0);
        for (hovers, gravity_back) in [(false, true), (true, false)] {
            let flying = FlyingMove {
                no_gravity: true,
                vertical: 0.3,
                forward: 0.3,
                ..FlyingMove::default()
            };
            let waited = FlyingMoveControl::new(20, hovers).tick(
                &mut control,
                flying,
                position,
                false,
                0.2,
                0.4,
            );
            assert_eq!(waited.no_gravity, !gravity_back);
            assert_eq!((waited.vertical, waited.forward), (0.0, 0.0));
        }
        control.set_wanted_position(position + DVec3::new(0.0001, 0.0, 0.0), 1.0);
        let near = FlyingMove {
            yaw: 45.0,
            speed: 0.3,
            forward: 0.3,
            vertical: 0.3,
            ..FlyingMove::default()
        };
        let arrived =
            FlyingMoveControl::new(20, true).tick(&mut control, near, position, false, 0.2, 0.4);
        assert_eq!((arrived.yaw, arrived.speed), (45.0, 0.3));
        assert_eq!((arrived.vertical, arrived.forward), (0.0, 0.0));
        assert!(arrived.no_gravity);
    }

    #[test]
    fn a_flying_path_gives_up_when_stuck() {
        let scene = Scene::default();
        let profile = flyer(0.5, 0.9);
        let fluid = FluidFrame::default();
        let from = DVec3::new(0.5, 64.0, 0.5);
        let to = DVec3::new(12.5, 64.0, 0.5);
        let (mut navigation, path) = plan(&scene, &profile, from, to);
        assert!(navigation.move_to(Some(path), 1.0, from));
        let body = Body::new(from, profile.width, profile.height);
        for tick in 1..=100 {
            let wanted = tick_path(&mut navigation, &scene, &body, &profile, fluid, false, 0.4);
            assert!(wanted.is_some(), "{tick}");
        }
        assert!(tick_path(&mut navigation, &scene, &body, &profile, fluid, false, 0.4).is_none());
        assert!(navigation.is_stuck() && navigation.is_done());
        let (mut navigation, path) = plan(&scene, &profile, from, to);
        assert!(navigation.move_to(Some(path), 1.0, from));
        let mut body = Body::new(from, profile.width, profile.height);
        for tick in 1..=150 {
            body.position.x = from.x + f64::from(tick) * 0.05;
            let wanted = tick_path(&mut navigation, &scene, &body, &profile, fluid, false, 0.4);
            let (wanted, speed) = wanted.expect("still on its way");
            assert_eq!(speed, 1.0);
            assert_eq!(wanted.y, 64.0);
        }
        assert!(!navigation.is_stuck());
        let (mut navigation, path) = plan(&scene, &profile, from, to);
        assert!(navigation.move_to(Some(path), 1.0, from));
        let riding = Body::new(from, profile.width, profile.height);
        for tick in 1..=150 {
            let wanted = tick_path(&mut navigation, &scene, &riding, &profile, fluid, true, 0.4);
            assert!(wanted.is_some(), "{tick}");
        }
        assert_eq!(navigation.next, 1);
        assert!(!navigation.is_stuck());
    }

    #[test]
    fn a_spawn_bonus_keeps_a_flyer_s_required_path_length() {
        let body = Body::new(DVec3::new(0.5, 64.0, 0.5), 0.55, 0.5);
        let kind = crate::monster_ai::MonsterKind::Creeper;
        let mut ai = crate::monster_ai::MonsterAi::of_kind(kind, &body, 0.0);
        ai.state.follow_range = 17.5;
        assert_eq!(ai.state.profile().max_path_length, Some(17.5));
        fly(&mut ai.state.walk, Flight::default());
        require_path_length(&mut ai.state.walk, 48.0);
        assert_eq!(ai.state.profile().max_path_length, Some(48.0));
        assert_eq!(ai.state.profile().max_visited_nodes(), 768);
        ai.state.follow_range = 64.0;
        assert_eq!(ai.state.profile().max_path_length, Some(64.0));
    }

    #[test]
    fn a_flyer_moves_straight_only_through_clear_air() {
        let mut scene = Scene::default();
        let (from, to) = (DVec3::new(0.5, 64.0, 0.5), DVec3::new(4.5, 64.0, 0.5));
        assert!(is_clear_for_movement_between(&scene, from, to, 0.9, true));
        scene.0.insert((2, 64, 0), Block::new("minecraft:water"));
        assert!(!is_clear_for_movement_between(&scene, from, to, 0.9, true));
        assert!(is_clear_for_movement_between(&scene, from, to, 0.9, false));
        scene.0.insert((2, 64, 0), Block::new("minecraft:stone"));
        assert!(!is_clear_for_movement_between(&scene, from, to, 0.9, false));
        let ghastling = Flight {
            blocked_by_fluids: false,
            ..Flight::default()
        };
        scene.0.insert((2, 64, 0), Block::new("minecraft:water"));
        assert!(ghastling.can_move_directly(&scene, from, to, 0.9));
        assert!(!Flight::default().can_move_directly(&scene, from, to, 0.9));
    }

    #[test]
    fn a_flyer_follows_its_path_through_a_gap_in_a_wall() {
        let scene = wall_with_a_gap();
        let mut profile = flyer(0.35, 0.6);
        require_path_length(&mut profile, 48.0);
        assert_eq!(
            (profile.max_path_length, profile.max_visited_nodes()),
            (Some(48.0), 768)
        );
        let from = DVec3::new(0.5, 64.0, 0.5);
        let target = DVec3::new(10.5, 64.0, 0.5);
        let mut body = Body::new(from, profile.width, profile.height);
        let mut navigation = GroundNavigation::default();
        let fluid = FluidFrame::default();
        assert_eq!(
            navigate_walk_to(
                &body,
                &mut navigation,
                &scene,
                &profile,
                fluid,
                target,
                1.0,
                0
            ),
            Some(true)
        );
        let flight = FlyingMoveControl::new(20, true);
        let mut control = MoveControl::default();
        let mut moved = FlyingMove::default();
        let mut through = false;
        for _ in 0..600 {
            if navigation.is_done() {
                break;
            }
            if let Some((wanted, speed)) = tick_path(
                &mut navigation,
                &scene,
                &body,
                &profile,
                fluid,
                false,
                moved.speed,
            ) {
                control.set_wanted_position(wanted, speed);
            }
            moved = flight.tick(&mut control, moved, body.position, body.on_ground, 0.1, 0.1);
            let input = DVec3::new(0.0, f64::from(moved.vertical), f64::from(moved.forward));
            body.travel_flying(&scene, input, moved.speed, moved.yaw, fluid);
            through |= body.position.x > 5.0 && body.position.x < 6.0;
            assert!(body.position.y < 70.5);
        }
        assert!(navigation.is_done() && !navigation.is_stuck());
        assert!(through);
        assert!(body.position.distance(target) < 1.5, "{:?}", body.position);
    }

    #[test]
    fn the_brain_move_sink_flies_a_flyer_to_its_walk_target() {
        let scene = floor_and_wall();
        let mut profile = flyer(0.35, 0.6);
        require_path_length(&mut profile, 48.0);
        let body = Body::new(DVec3::new(0.5, 64.0, 0.5), profile.width, profile.height);
        let mut navigation = GroundNavigation::default();
        let mut random = LegacyRandom::new(1);
        let mut walk_target = Slot::default();
        walk_target.set(WalkGoal {
            target: Track::Block((10, 64, 0)),
            speed: 1.0,
            close_enough: 0,
        });
        let mut cant_reach = Slot::default();
        let mut sink = MoveSink::default();
        let mut walker = crate::brain_behaviors::Walker {
            body: &body,
            others: &[],
            world: &scene,
            time: 0,
            random: &mut random,
            navigation: &mut navigation,
            profile: &profile,
            fluid: FluidFrame::default(),
        };
        sink.try_start(&mut walk_target, &mut cant_reach, &mut walker);
        assert!(sink.running());
        assert!(!cant_reach.present());
        assert!(!navigation.is_done());
        assert_eq!(navigation.nodes.last(), Some(&(10, 64, 0)));
        assert!(navigation.nodes.iter().any(|&(_, y, _)| y >= 67));
    }
}
