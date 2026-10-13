use crate::health::DamageState;
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::Pos;

pub const TYPE_ID: &str = "minecraft:snow_golem";
pub const WIDTH: f32 = 0.7;
pub const HEIGHT: f32 = 1.9;
pub const EYE_HEIGHT: f32 = 1.7;
pub const MAX_HEALTH: f32 = 4.0;
pub const MOVEMENT_SPEED: f64 = 0.200_000_002_980_232_24;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const AMBIENT_INTERVAL: i32 = 120;
pub const ATTACK_SPEED: f64 = 1.25;
pub const ATTACK_INTERVAL: i32 = 20;
pub const ATTACK_RADIUS: f32 = 10.0;
pub const STROLL_SPEED: f64 = 1.0;
pub const STROLL_PROBABILITY: f32 = 1.000_000_1e-5;
pub const LOOK_RANGE: f64 = 6.0;
pub const TARGET_INTERVAL: u32 = 5;
pub const SNOWBALL: &str = "minecraft:snowball";
pub const SNOWBALL_POWER: f32 = 1.6;
pub const SNOWBALL_UNCERTAINTY: f32 = 12.0;
pub const SNOWBALL_BLAZE_DAMAGE: f32 = 3.0;
pub const SNOW: &str = "minecraft:snow";
pub const SHEAR_LOOT: &str = "minecraft:shearing/snow_golem";
pub const AMBIENT_SOUND: &str = "entity.snow_golem.ambient";
pub const HURT_SOUND: &str = "entity.snow_golem.hurt";
pub const DEATH_SOUND: &str = "entity.snow_golem.death";
pub const SHOOT_SOUND: &str = "entity.snow_golem.shoot";
pub const SHEAR_SOUND: &str = "entity.snow_golem.shear";
pub const WATER_DAMAGE: f32 = 1.0;
pub const MELT_DAMAGE: f32 = 1.0;

#[derive(Clone, Debug)]
pub struct SnowGolem {
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub persistence_required: bool,
    pub pumpkin: bool,
}

impl SnowGolem {
    pub fn new(position: DVec3) -> Self {
        Self {
            body: Body::new(position, WIDTH, HEIGHT),
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            persistence_required: false,
            pumpkin: true,
        }
    }

    pub fn ready_for_shearing(&self) -> bool {
        self.pumpkin
    }

    pub fn snowball_start(&self) -> DVec3 {
        let p = self.body.position;
        DVec3::new(p.x, p.y + f64::from(EYE_HEIGHT) - f64::from(0.1_f32), p.z)
    }

    pub fn snowball_aim(&self, target: DVec3, target_eye_height: f32) -> DVec3 {
        let p = self.body.position;
        let dx = target.x - p.x;
        let eye = target.y + f64::from(target_eye_height) - 1.100_000_023_841_858;
        let dz = target.z - p.z;
        let lift = (dx * dx + dz * dz).sqrt() * 0.200_000_002_980_232_24;
        DVec3::new(dx, eye + lift - self.snowball_start().y, dz)
    }

    pub fn shoot_pitch(random: f32) -> f32 {
        1.0 / (random * 0.4 + 0.8)
    }

    pub fn trail_cells(&self) -> [Pos; 4] {
        let p = self.body.position;
        let y = p.y.floor() as i32;
        let cell = |i: i32| {
            let x = p.x + f64::from(((i % 2 * 2 - 1) as f32) * 0.25);
            let z = p.z + f64::from(((i / 2 % 2 * 2 - 1) as f32) * 0.25);
            (x.floor() as i32, y, z.floor() as i32)
        };
        [cell(0), cell(1), cell(2), cell(3)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snow_golem_is_small_and_frail_and_wears_its_pumpkin() {
        let golem = SnowGolem::new(DVec3::new(0.5, 64.0, 0.5));
        assert_eq!((golem.body.width, golem.body.height), (0.7, 1.9));
        assert_eq!(golem.health, 4.0);
        assert!(golem.pumpkin && golem.ready_for_shearing());
    }

    #[test]
    fn the_trail_covers_the_four_blocks_under_its_corners() {
        let mut golem = SnowGolem::new(DVec3::new(10.1, 64.0, -3.9));
        assert_eq!(
            golem.trail_cells(),
            [(9, 64, -5), (10, 64, -5), (9, 64, -4), (10, 64, -4)]
        );
        golem.body.position = DVec3::new(10.5, 64.0, 0.5);
        assert_eq!(
            golem.trail_cells(),
            [(10, 64, 0), (10, 64, 0), (10, 64, 0), (10, 64, 0)]
        );
    }

    #[test]
    fn a_snowball_leaves_under_its_eyes_and_aims_above_the_target_with_distance() {
        let golem = SnowGolem::new(DVec3::new(0.5, 64.0, 0.5));
        let start = golem.snowball_start();
        assert!((start.y - (64.0 + 1.7 - 0.1)).abs() < 1e-6);
        let aim = golem.snowball_aim(DVec3::new(0.5, 64.0, 10.5), 1.74);
        assert_eq!((aim.x, aim.z), (0.0, 10.0));
        let expected = 64.0 + 1.74_f32 as f64 - 1.100_000_023_841_858
            + 10.0 * 0.200_000_002_980_232_24
            - start.y;
        assert!((aim.y - expected).abs() < 1e-9);
        assert!((SnowGolem::shoot_pitch(0.0) - 1.25).abs() < 1e-6);
        assert!((SnowGolem::shoot_pitch(1.0) - 1.0 / 1.2).abs() < 1e-6);
    }
}
