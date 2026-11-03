use crate::control::{MoveControl, MoveOperation, minecraft_atan2, rotlerp};
use crate::look::rotate_towards;
use crate::navigation::WaterNav;
use crate::panic::manhattan_ordered;
use crate::stroll::{
    amphibious_random_position, default_random_position, water_bound_random_position,
};
use crate::walk_path::{WalkProfile, WalkTarget, pathfindable_in_water};
use crate::world::families::MobCore;
use crate::world::{Voice, voice_pitch};
use glam::DVec3;
use minecraftoss_player::mth;
use minecraftoss_player::path_type::PathType;
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Pos, World};

pub const RAD_TO_DEG: f64 = 57.2957763671875;
pub const DEG_TO_RAD: f32 = 0.017453292;
pub const CLOSE_ENOUGH: f64 = 2.500000277905201E-7;
pub const FISH_SPEED_LERP: f32 = 0.125;
pub const FISH_TURN: f32 = 90.0;
pub const RISE_IN_WATER: f64 = 0.005;
pub const PITCH_STEP: f32 = 5.0;
pub const WATER: &str = "minecraft:water";
pub const SWIM_TIERS: [(u32, u32); 6] = [(1, 1), (3, 3), (5, 5), (6, 5), (7, 7), (10, 7)];
pub const SWIMMABLE_RETRIES: u32 = 10;
pub const FIND_LIQUID_WAIT: i64 = 22;
pub const FIND_LIQUID_INTERVAL: i64 = 40;
pub const FLOP_LIFT: f64 = 0.4000000059604645;
pub const FLOP_SPREAD: f32 = 0.05;
pub const FLOP_VOLUME: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwimFrame {
    pub yaw: f32,
    pub pitch: f32,
    pub body_yaw: f32,
    pub head_yaw: f32,
    pub speed: f32,
    pub sideways: f32,
    pub up: f32,
    pub forward: f32,
}

impl SwimFrame {
    fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
        self.forward = speed;
    }

    fn stop(&mut self) {
        self.set_speed(0.0);
        self.sideways = 0.0;
        self.up = 0.0;
        self.forward = 0.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothSwimmingMoveControl {
    pub max_turn_x: i32,
    pub max_turn_y: i32,
    pub in_water: f32,
    pub outside: f32,
    pub gravity: bool,
}

impl SmoothSwimmingMoveControl {
    pub const fn new(
        max_turn_x: i32,
        max_turn_y: i32,
        in_water: f32,
        outside: f32,
        gravity: bool,
    ) -> Self {
        Self {
            max_turn_x,
            max_turn_y,
            in_water,
            outside,
            gravity,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &self,
        control: &MoveControl,
        navigation_done: bool,
        position: DVec3,
        in_water: bool,
        movement_speed: f64,
        frame: &mut SwimFrame,
        velocity: &mut DVec3,
    ) {
        if self.gravity && in_water {
            velocity.y += RISE_IN_WATER;
        }
        if control.operation != MoveOperation::MoveTo || navigation_done {
            frame.stop();
            return;
        }
        let delta = control.wanted - position;
        if delta.length_squared() < CLOSE_ENOUGH {
            frame.forward = 0.0;
            return;
        }
        let target_yaw = (minecraft_atan2(delta.z, delta.x) * RAD_TO_DEG) as f32 - 90.0;
        frame.yaw = rotlerp(frame.yaw, target_yaw, self.max_turn_y as f32);
        frame.body_yaw = frame.yaw;
        frame.head_yaw = frame.yaw;
        let speed = (control.speed_modifier * movement_speed) as f32;
        if in_water {
            frame.set_speed(speed * self.in_water);
            let horizontal = (delta.x * delta.x + delta.z * delta.z).sqrt();
            let tiny = f64::from(1.0e-5_f32);
            if delta.y.abs() > tiny || horizontal.abs() > tiny {
                let wanted = -((minecraft_atan2(delta.y, horizontal) * RAD_TO_DEG) as f32);
                let max = self.max_turn_x as f32;
                let wanted = wrap_degrees(wanted).clamp(-max, max);
                frame.pitch = rotate_towards(frame.pitch, wanted, PITCH_STEP);
            }
            let radians = f64::from(frame.pitch * DEG_TO_RAD);
            frame.forward = mth::cos(radians) * speed;
            frame.up = -mth::sin(radians) * speed;
        } else {
            let turn = wrap_degrees(frame.yaw - target_yaw).abs();
            let factor = 1.0 - ((turn - 10.0) / 50.0).clamp(0.0, 1.0);
            frame.set_speed(speed * self.outside * factor);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FishMoveControl;

impl FishMoveControl {
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &self,
        control: &MoveControl,
        navigation_done: bool,
        position: DVec3,
        eyes_in_water: bool,
        movement_speed: f64,
        frame: &mut SwimFrame,
        velocity: &mut DVec3,
    ) {
        if eyes_in_water {
            velocity.y += RISE_IN_WATER;
        }
        if control.operation != MoveOperation::MoveTo || navigation_done {
            frame.set_speed(0.0);
            return;
        }
        let wanted = (control.speed_modifier * movement_speed) as f32;
        frame.set_speed(frame.speed + FISH_SPEED_LERP * (wanted - frame.speed));
        let delta = control.wanted - position;
        if delta.y != 0.0 {
            let distance = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
            velocity.y += f64::from(frame.speed) * (delta.y / distance) * 0.1;
        }
        if delta.x != 0.0 || delta.z != 0.0 {
            let target = (minecraft_atan2(delta.z, delta.x) * RAD_TO_DEG) as f32 - 90.0;
            frame.yaw = rotlerp(frame.yaw, target, FISH_TURN);
            frame.body_yaw = frame.yaw;
        }
    }
}

pub fn water_walk(profile: &mut WalkProfile) {
    profile.set_malus(PathType::Water, 0.0);
    profile.walk_target = WalkTarget::Zero;
    profile.can_float = false;
}

#[allow(clippy::too_many_arguments)]
pub fn random_position(
    world: &dyn World,
    profile: &WalkProfile,
    water: WaterNav,
    position: DVec3,
    horizontal: u32,
    vertical: u32,
    home: Option<(Pos, i32)>,
    random: &mut LegacyRandom,
) -> Option<DVec3> {
    match water {
        WaterNav::Bound { .. } => water_bound_random_position(
            world, profile, position, horizontal, vertical, home, random,
        ),
        WaterNav::Amphibious { .. } => {
            amphibious_random_position(world, profile, position, horizontal, vertical, random)
        }
        WaterNav::Ground => {
            default_random_position(world, profile, position, horizontal, vertical, random)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn random_swimmable_position(
    world: &dyn World,
    profile: &WalkProfile,
    water: WaterNav,
    position: DVec3,
    horizontal: u32,
    vertical: u32,
    home: Option<(Pos, i32)>,
    random: &mut LegacyRandom,
) -> Option<DVec3> {
    let draw = |random: &mut LegacyRandom| {
        random_position(
            world, profile, water, position, horizontal, vertical, home, random,
        )
    };
    let mut spot = draw(random);
    let mut retries = 0;
    while let Some(at) = spot {
        if retries >= SWIMMABLE_RETRIES || pathfindable_in_water(world, block_of(at)) {
            break;
        }
        retries += 1;
        spot = draw(random);
    }
    spot
}

pub fn swim_stroll_target(
    world: &dyn World,
    profile: &WalkProfile,
    water: WaterNav,
    position: DVec3,
    home: Option<(Pos, i32)>,
    random: &mut LegacyRandom,
) -> Option<DVec3> {
    let mut found: Option<DVec3> = None;
    let mut candidate = None;
    for (horizontal, vertical) in SWIM_TIERS {
        candidate = match found {
            None => random_swimmable_position(
                world, profile, water, position, horizontal, vertical, home, random,
            ),
            Some(last) => {
                let (h, v) = (f64::from(horizontal), f64::from(vertical));
                Some(position + normalize(last - position) * DVec3::new(h, v, h))
            }
        };
        let restricted = |spot: DVec3| {
            home.is_some_and(|(center, radius)| {
                let reach = f64::from(radius) + f64::from(horizontal) + 1.0;
                let near = centre_of(center).distance_squared(position) < reach * reach;
                near && !within_home(center, radius, block_of(spot))
            })
        };
        let Some(spot) =
            candidate.filter(|&spot| holds_fluid(world, block_of(spot)) && !restricted(spot))
        else {
            return found;
        };
        found = Some(spot);
    }
    candidate
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TryFindLiquid {
    pub next: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquidSearch {
    InLiquid,
    Waited,
    Searched(Option<Pos>),
}

impl TryFindLiquid {
    pub fn run(
        &mut self,
        world: &dyn World,
        position: DVec3,
        range: i32,
        time: i64,
    ) -> LiquidSearch {
        let feet = block_of(position);
        if world.floatable_fluid(feet) {
            return LiquidSearch::InLiquid;
        }
        if time < self.next {
            self.next = time + FIND_LIQUID_WAIT;
            return LiquidSearch::Waited;
        }
        let mut found = None;
        for pos in manhattan_ordered(feet, range, range, range, range * 3) {
            let own_column = pos.0 == feet.0 && pos.2 == feet.2;
            if own_column || !is_water_block(world, pos) {
                continue;
            }
            if is_air(world, (pos.0, pos.1 + 1, pos.2)) {
                found = Some(pos);
                break;
            }
            if found.is_none() && centre_of(pos).distance_squared(position) >= 1.5 * 1.5 {
                found = Some(pos);
            }
        }
        self.next = time + FIND_LIQUID_INTERVAL;
        LiquidSearch::Searched(found)
    }
}

pub fn flop(core: &mut MobCore, sound: &'static str) -> bool {
    let body = &core.body;
    if body.touching_water || !body.on_ground || !body.vertical_collision {
        return false;
    }
    let x = (core.random.next_float() * 2.0 - 1.0) * FLOP_SPREAD;
    let z = (core.random.next_float() * 2.0 - 1.0) * FLOP_SPREAD;
    let body = &mut core.body;
    body.velocity += DVec3::new(f64::from(x), FLOP_LIFT, f64::from(z));
    body.on_ground = false;
    body.needs_sync = true;
    let pitch = voice_pitch(&mut core.random, core.baby);
    let at = core.body.position;
    core.voices
        .push((Voice::Event(sound, FLOP_VOLUME, pitch), at));
    true
}

pub fn block_of(at: DVec3) -> Pos {
    (
        at.x.floor() as i32,
        at.y.floor() as i32,
        at.z.floor() as i32,
    )
}

fn centre_of((x, y, z): Pos) -> DVec3 {
    DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5)
}

fn within_home((x, y, z): Pos, radius: i32, pos: Pos) -> bool {
    let d = DVec3::new(
        f64::from(x - pos.0),
        f64::from(y - pos.1),
        f64::from(z - pos.2),
    );
    d.length_squared() < f64::from(radius * radius)
}

fn normalize(v: DVec3) -> DVec3 {
    let length = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    if length < f64::from(1.0e-5_f32) {
        DVec3::ZERO
    } else {
        DVec3::new(v.x / length, v.y / length, v.z / length)
    }
}

fn holds_fluid(world: &dyn World, pos: Pos) -> bool {
    world.floatable_fluid(pos) || world.fluid_is_lava(pos)
}

fn is_water_block(world: &dyn World, pos: Pos) -> bool {
    world.block(pos).is_some_and(|block| block.id == WATER)
}

fn is_air(world: &dyn World, pos: Pos) -> bool {
    world.block(pos).is_none_or(|block| {
        matches!(
            block.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

fn wrap_degrees(angle: f32) -> f32 {
    let mut angle = angle % 360.0;
    if angle >= 180.0 {
        angle -= 360.0;
    }
    if angle < -180.0 {
        angle += 360.0;
    }
    angle
}

#[cfg(test)]
mod tests;
