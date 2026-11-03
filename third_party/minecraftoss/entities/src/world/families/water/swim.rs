use crate::control::{MoveControl, MoveOperation, minecraft_atan2, rotlerp};
use crate::look::rotate_towards;
use crate::walk_path::{WalkProfile, WalkTarget};
use glam::DVec3;
use minecraftoss_player::mth;
use minecraftoss_player::path_type::PathType;

pub const RAD_TO_DEG: f64 = 57.2957763671875;
pub const DEG_TO_RAD: f32 = 0.017453292;
pub const CLOSE_ENOUGH: f64 = 2.500000277905201E-7;
pub const FISH_SPEED_LERP: f32 = 0.125;
pub const FISH_TURN: f32 = 90.0;
pub const RISE_IN_WATER: f64 = 0.005;
pub const PITCH_STEP: f32 = 5.0;

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
