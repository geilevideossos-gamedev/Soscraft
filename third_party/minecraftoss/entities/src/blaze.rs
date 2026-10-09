use crate::health::DamageState;
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const MAX_HEALTH: f32 = 20.0;
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.53;
pub const ATTACK_DAMAGE: f32 = 6.0;
pub const MOVEMENT_SPEED: f64 = 0.23000000417232513;
pub const FOLLOW_RANGE: f64 = 48.0;
pub const EXPERIENCE: i32 = 10;

#[derive(Clone, Debug)]
pub struct Blaze {
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub persistence_required: bool,
    pub charged: bool,
    pub allowed_height_offset: f32,
    pub next_height_offset_change_tick: i32,
}

impl Blaze {
    pub fn new(position: DVec3) -> Self {
        Self {
            body: Body::new(position, WIDTH, HEIGHT),
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            persistence_required: false,
            charged: false,
            allowed_height_offset: 0.5,
            next_height_offset_change_tick: 0,
        }
    }

    pub fn slow_fall(&mut self) {
        if !self.body.on_ground && self.body.velocity.y < 0.0 {
            self.body.velocity.y *= 0.6;
        }
    }

    pub fn rise_towards(&mut self, target_eye: Option<f64>, random: &mut LegacyRandom) {
        self.next_height_offset_change_tick -= 1;
        if self.next_height_offset_change_tick <= 0 {
            self.next_height_offset_change_tick = 100;
            self.allowed_height_offset = triangle(random, 0.5, 6.891) as f32;
        }
        let eye = self.body.position.y + f64::from(EYE_HEIGHT);
        if target_eye.is_some_and(|target| target > eye + f64::from(self.allowed_height_offset)) {
            let lift = f64::from(0.3_f32);
            self.body.velocity.y += (lift - self.body.velocity.y) * lift;
            self.body.needs_sync = true;
        }
    }
}

pub fn triangle(random: &mut LegacyRandom, mode: f64, deviation: f64) -> f64 {
    mode + deviation * (random.next_double() - random.next_double())
}

pub fn small_fireball_aim(delta: DVec3, distance_squared: f64, random: &mut LegacyRandom) -> DVec3 {
    let spread = distance_squared.sqrt().sqrt() * 0.5;
    let x = triangle(random, delta.x, 2.297 * spread);
    let z = triangle(random, delta.z, 2.297 * spread);
    DVec3::new(x, delta.y, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blaze_falls_slowly_and_rises_towards_a_target_above_it() {
        let mut blaze = Blaze::new(DVec3::new(0.5, 70.0, 0.5));
        blaze.body.velocity.y = -0.5;
        blaze.slow_fall();
        assert!((blaze.body.velocity.y + 0.3).abs() < 1e-12);
        blaze.body.on_ground = true;
        blaze.slow_fall();
        assert!(
            (blaze.body.velocity.y + 0.3).abs() < 1e-12,
            "on the ground nothing changes"
        );
        let mut random = LegacyRandom::new(3);
        blaze.body.velocity.y = 0.0;
        blaze.rise_towards(Some(80.0), &mut random);
        assert_eq!(blaze.next_height_offset_change_tick, 100);
        assert!((0.5 - 6.891..=0.5 + 6.891).contains(&f64::from(blaze.allowed_height_offset)));
        let expected =
            if 80.0 > 70.0 + f64::from(EYE_HEIGHT) + f64::from(blaze.allowed_height_offset) {
                f64::from(0.3_f32) * f64::from(0.3_f32)
            } else {
                0.0
            };
        assert!((blaze.body.velocity.y - expected).abs() < 1e-12);
        blaze.allowed_height_offset = 0.5;
        blaze.body.velocity.y = 0.0;
        blaze.rise_towards(Some(70.0), &mut random);
        assert_eq!(blaze.body.velocity.y, 0.0, "a target below leaves it be");
        let aim = small_fireball_aim(DVec3::new(0.0, 1.0, 16.0), 256.0, &mut LegacyRandom::new(9));
        assert_eq!(aim.y, 1.0);
        assert!(aim.x.abs() <= 2.297 * 2.0 && (aim.z - 16.0).abs() <= 2.297 * 2.0);
    }
}
