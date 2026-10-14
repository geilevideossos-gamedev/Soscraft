use crate::health::{DamageResult, DamageState, damage_after_armor};
use crate::movement::Body;
use glam::DVec3;

pub const TYPE_ID: &str = "minecraft:wither";
pub const WIDTH: f32 = 0.9;
pub const HEIGHT: f32 = 3.5;
pub const EYE_HEIGHT: f32 = HEIGHT * 0.85;
pub const MAX_HEALTH: f32 = 300.0;
pub const SPEED: f64 = 0.600_000_023_841_857_9;
pub const FOLLOW_RANGE: f64 = 40.0;
pub const ARMOR: f32 = 4.0;
pub const EXPERIENCE: i32 = 50;
pub const AMBIENT_INTERVAL: i32 = 80;
pub const INVULNERABLE_TICKS: i32 = 220;
pub const CHARGE_HEAL_INTERVAL: i32 = 10;
pub const CHARGE_HEAL: f32 = 10.0;
pub const HEAL_INTERVAL: i32 = 20;
pub const HEAL: f32 = 1.0;
pub const BLAST_RADIUS: f32 = 7.0;
pub const SPAWN_SOUND: &str = "entity.wither.spawn";
pub const SPAWN_EVENT: i32 = 1023;
pub const SIDE_HEAD_REACH: f64 = 1.3;
pub const CENTER_HEAD_HEIGHT: f32 = 3.0;
pub const SIDE_HEAD_HEIGHT: f32 = 2.2;
pub const HEAD_YAW_TURN: f32 = 10.0;
pub const HEAD_PITCH_TURN: f32 = 40.0;
pub const SIDE_TARGET_RANGE_SQR: f64 = 900.0;
pub const SIDE_SEARCH: DVec3 = DVec3::new(20.0, 8.0, 20.0);
pub const SIDE_SEARCH_RANGE: f64 = 20.0;
pub const IDLE_HEAD_LIMIT: i32 = 15;
pub const IDLE_SHOT_SPREAD: DVec3 = DVec3::new(10.0, 5.0, 10.0);
pub const HEAD_UPDATE_BASE: i32 = 10;
pub const HEAD_UPDATE_SPREAD: u32 = 10;
pub const SHOT_UPDATE_BASE: i32 = 40;
pub const SHOT_UPDATE_SPREAD: u32 = 20;
pub const HURT_IDLE_BONUS: i32 = 3;
pub const DESTROY_BLOCKS_DELAY: i32 = 20;
pub const VERTICAL_DRAG: f64 = 0.6;
pub const CHASE_LIFT: f64 = 0.3;
pub const CHASE_PULL: f64 = 0.3;
pub const CHASE_DAMPING: f64 = 0.600_000_023_841_857_9;
pub const HOVER_ABOVE: f64 = 5.0;
pub const CHASE_MIN_SQR: f64 = 9.0;
pub const FACING_MIN_SQR: f64 = 0.05;
pub const FLIGHT_YAW_TURN: f32 = 90.0;
pub const NEAR_WANTED_SQR: f64 = 2.500_000_277_905_201E-7;
pub const RANGED_SPEED: f64 = 1.0;
pub const RANGED_INTERVAL: i32 = 40;
pub const RANGED_RADIUS: f32 = 20.0;
pub const STROLL_SPEED: f64 = 1.0;
pub const STROLL_INTERVAL: u32 = 60;
pub const LOOK_RANGE: f64 = 8.0;
pub const DANGEROUS_CHANCE: f32 = 0.001;
pub const SMOKE_SPREAD: f32 = 0.3;
pub const POWERED_COLOR: [f32; 3] = [0.7, 0.7, 0.5];
pub const CHARGE_COLOR: [f32; 3] = [0.7, 0.7, 0.9];
pub const CHARGE_PARTICLE_HEIGHT: f32 = 3.3;

const UNDEAD: [&str; 17] = [
    "minecraft:skeleton",
    "minecraft:stray",
    "minecraft:wither_skeleton",
    "minecraft:skeleton_horse",
    "minecraft:bogged",
    "minecraft:parched",
    "minecraft:zombie_horse",
    "minecraft:camel_husk",
    "minecraft:zombie",
    "minecraft:zombie_villager",
    "minecraft:zombified_piglin",
    "minecraft:zoglin",
    "minecraft:drowned",
    "minecraft:husk",
    "minecraft:zombie_nautilus",
    "minecraft:wither",
    "minecraft:phantom",
];

pub fn is_undead(kind: &str) -> bool {
    UNDEAD.contains(&kind)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkullShot {
    pub head: usize,
    pub target: DVec3,
    pub dangerous: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChargeStep {
    pub heal: bool,
    pub explode: bool,
}

#[derive(Clone, Debug)]
pub struct Wither {
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub body_yaw: f32,
    pub persistence_required: bool,
    pub invulnerable_ticks: i32,
    pub head_x_rots: [f32; 2],
    pub head_y_rots: [f32; 2],
    pub next_head_update: [i32; 2],
    pub idle_head_updates: [i32; 2],
    pub destroy_blocks_tick: i32,
    pub forward: f32,
    pub vertical: f32,
}

pub fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

pub fn rotlerp(from: f32, to: f32, max: f32) -> f32 {
    from + wrap_degrees(to - from).clamp(-max, max)
}

pub fn head_position(position: DVec3, body_yaw: f32, head: usize) -> DVec3 {
    if head == 0 {
        return DVec3::new(
            position.x,
            position.y + f64::from(CENTER_HEAD_HEIGHT),
            position.z,
        );
    }
    let angle = (body_yaw + (180 * (head as i32 - 1)) as f32) * 0.017_453_292;
    DVec3::new(
        position.x + f64::from(minecraftoss_player::mth::cos(f64::from(angle))) * SIDE_HEAD_REACH,
        position.y + f64::from(SIDE_HEAD_HEIGHT),
        position.z + f64::from(minecraftoss_player::mth::sin(f64::from(angle))) * SIDE_HEAD_REACH,
    )
}

pub fn head_angles(head: DVec3, target_eye: DVec3) -> (f32, f32) {
    let dx = target_eye.x - head.x;
    let dy = target_eye.y - head.y;
    let dz = target_eye.z - head.z;
    let horizontal = (dx * dx + dz * dz).sqrt();
    let yaw = (minecraftoss_player::mth::atan2(dz, dx) * 57.295_776_367_187_5) as f32 - 90.0;
    let pitch = (-(minecraftoss_player::mth::atan2(dy, horizontal) * 57.295_776_367_187_5)) as f32;
    (yaw, pitch)
}

pub fn chase(velocity: DVec3, position: DVec3, target: Option<DVec3>, powered: bool) -> DVec3 {
    let mut v = DVec3::new(velocity.x, velocity.y * VERTICAL_DRAG, velocity.z);
    let Some(target) = target else {
        return v;
    };
    if position.y < target.y || (!powered && position.y < target.y + HOVER_ABOVE) {
        let up = v.y.max(0.0);
        v.y = up + (CHASE_LIFT - up * CHASE_DAMPING);
    }
    let towards = DVec3::new(target.x - position.x, 0.0, target.z - position.z);
    if towards.x * towards.x + towards.z * towards.z > CHASE_MIN_SQR {
        let n = towards.normalize();
        v += DVec3::new(
            n.x * CHASE_PULL - v.x * VERTICAL_DRAG,
            0.0,
            n.z * CHASE_PULL - v.z * VERTICAL_DRAG,
        );
    }
    v
}

pub fn facing(velocity: DVec3) -> Option<f32> {
    (velocity.x * velocity.x + velocity.z * velocity.z > FACING_MIN_SQR)
        .then(|| minecraftoss_player::mth::atan2(velocity.z, velocity.x) as f32 * 57.295_776 - 90.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    pub yaw: f32,
    pub speed: f32,
    pub forward: f32,
    pub vertical: f32,
    pub no_gravity: bool,
}

pub fn fly(
    wanted: Option<(DVec3, f64)>,
    position: DVec3,
    yaw: f32,
    forward: f32,
    vertical: f32,
) -> Flight {
    let Some((wanted, modifier)) = wanted else {
        return Flight {
            yaw,
            speed: 0.0,
            forward: 0.0,
            vertical: 0.0,
            no_gravity: false,
        };
    };
    let d = wanted - position;
    if d.length_squared() < NEAR_WANTED_SQR {
        return Flight {
            yaw,
            speed: forward,
            forward: 0.0,
            vertical: 0.0,
            no_gravity: true,
        };
    }
    let turn = (minecraftoss_player::mth::atan2(d.z, d.x) * 57.295_776_367_187_5) as f32 - 90.0;
    let yaw = crate::control::rotlerp(yaw, turn, FLIGHT_YAW_TURN);
    let speed = (modifier * SPEED) as f32;
    let horizontal = (d.x * d.x + d.z * d.z).sqrt();
    let vertical = if d.y.abs() > 1.0E-5_f32 as f64 || horizontal.abs() > 1.0E-5_f32 as f64 {
        if d.y > 0.0 { speed } else { -speed }
    } else {
        vertical
    };
    Flight {
        yaw,
        speed,
        forward: speed,
        vertical,
        no_gravity: true,
    }
}

impl Wither {
    pub fn new(position: DVec3) -> Self {
        Self {
            body: Body::new(position, WIDTH, HEIGHT),
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            body_yaw: 0.0,
            persistence_required: false,
            invulnerable_ticks: 0,
            head_x_rots: [0.0; 2],
            head_y_rots: [0.0; 2],
            next_head_update: [0; 2],
            idle_head_updates: [0; 2],
            destroy_blocks_tick: 0,
            forward: 0.0,
            vertical: 0.0,
        }
    }

    pub fn built(position: DVec3, yaw: f32) -> Self {
        let mut wither = Self::new(position);
        wither.yaw = yaw;
        wither.body_yaw = yaw;
        wither.make_invulnerable();
        wither
    }

    pub fn make_invulnerable(&mut self) {
        self.invulnerable_ticks = INVULNERABLE_TICKS;
        self.health = MAX_HEALTH / 3.0;
    }

    pub fn powered(&self) -> bool {
        self.health <= MAX_HEALTH / 2.0
    }

    pub fn charging(&self) -> bool {
        self.invulnerable_ticks > 0
    }

    pub fn eye(&self) -> DVec3 {
        self.body.position + DVec3::new(0.0, f64::from(EYE_HEIGHT), 0.0)
    }

    pub fn head(&self, head: usize) -> DVec3 {
        head_position(self.body.position, self.body_yaw, head)
    }

    pub fn heal(&mut self, amount: f32) {
        if self.health > 0.0 {
            self.health = (self.health + amount).min(MAX_HEALTH);
        }
    }

    pub fn charge_step(&mut self, tick_count: i32) -> ChargeStep {
        let left = self.invulnerable_ticks - 1;
        self.invulnerable_ticks = left;
        let heal = tick_count % CHARGE_HEAL_INTERVAL == 0;
        if heal {
            self.heal(CHARGE_HEAL);
        }
        ChargeStep {
            heal,
            explode: left <= 0,
        }
    }

    pub fn turn_heads(&mut self, targets: [Option<DVec3>; 2]) {
        for (i, target) in targets.into_iter().enumerate() {
            match target {
                Some(eye) => {
                    let (yaw, pitch) = head_angles(self.head(i + 1), eye);
                    self.head_x_rots[i] = rotlerp(self.head_x_rots[i], pitch, HEAD_PITCH_TURN);
                    self.head_y_rots[i] = rotlerp(self.head_y_rots[i], yaw, HEAD_YAW_TURN);
                }
                None => {
                    self.head_y_rots[i] =
                        rotlerp(self.head_y_rots[i], self.body_yaw, HEAD_YAW_TURN);
                }
            }
        }
    }

    pub fn refuses(
        &self,
        damage_type: &str,
        by_undead: bool,
        bypasses_invulnerability: bool,
    ) -> bool {
        damage_type == "minecraft:drown"
            || by_undead
            || (self.charging() && !bypasses_invulnerability)
    }

    pub fn hurt_armored(&mut self, amount: f32) -> DamageResult {
        self.damage
            .hurt_absorbed(&mut self.health, MAX_HEALTH, amount, |dealt| {
                damage_after_armor(dealt, ARMOR, 0.0)
            })
    }

    pub fn hurt_unarmored(&mut self, amount: f32) -> DamageResult {
        self.damage
            .hurt_generic(&mut self.health, MAX_HEALTH, amount)
    }

    pub fn note_hurt(&mut self) {
        if self.destroy_blocks_tick <= 0 {
            self.destroy_blocks_tick = DESTROY_BLOCKS_DELAY;
        }
        for idle in &mut self.idle_head_updates {
            *idle += HURT_IDLE_BONUS;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wither_is_a_tall_three_hundred_health_armoured_boss() {
        let wither = Wither::new(DVec3::new(0.5, 64.0, 0.5));
        assert_eq!(
            (wither.body.width, wither.body.height, wither.health),
            (0.9, 3.5, 300.0)
        );
        assert!((EYE_HEIGHT - 2.975).abs() < 1e-6);
        assert!(!wither.powered() && !wither.charging());
        assert_eq!(SPEED as f32, 0.6);
        assert!((wither.eye().y - 66.975).abs() < 1e-6);
        assert!(is_undead("minecraft:wither_skeleton") && is_undead("minecraft:wither"));
        assert!(!is_undead("minecraft:creeper") && !is_undead("minecraft:player"));
    }

    #[test]
    fn a_built_wither_charges_for_220_ticks_at_a_third_of_its_health() {
        let mut wither = Wither::built(DVec3::ZERO, 90.0);
        assert_eq!((wither.invulnerable_ticks, wither.health), (220, 100.0));
        assert!(wither.powered() && wither.charging());
        assert_eq!((wither.yaw, wither.body_yaw), (90.0, 90.0));
        let mut explodes = Vec::new();
        let mut heals = 0;
        for tick in 1..=230 {
            if !wither.charging() {
                break;
            }
            let step = wither.charge_step(tick);
            heals += i32::from(step.heal);
            if step.explode {
                explodes.push(tick);
            }
        }
        assert_eq!(explodes, vec![220]);
        assert_eq!(heals, 22);
        assert_eq!(
            wither.health, 300.0,
            "ten health every ten ticks fills it up"
        );
        assert!(!wither.charging());
    }

    #[test]
    fn its_side_heads_sit_one_and_a_third_blocks_out_and_lower() {
        let at = DVec3::new(0.0, 64.0, 0.0);
        assert_eq!(head_position(at, 0.0, 0), DVec3::new(0.0, 67.0, 0.0));
        let right = head_position(at, 0.0, 1);
        let left = head_position(at, 0.0, 2);
        assert!(
            (right - DVec3::new(1.3, 66.2, 0.0)).length() < 1e-5,
            "{right}"
        );
        assert!(
            (left - DVec3::new(-1.3, 66.2, 0.0)).length() < 1e-5,
            "{left}"
        );
        let turned = head_position(at, 90.0, 1);
        assert!(
            (turned - DVec3::new(0.0, 66.2, 1.3)).length() < 1e-5,
            "{turned}"
        );
    }

    #[test]
    fn a_side_head_turns_ten_degrees_a_tick_towards_its_target() {
        let mut wither = Wither::new(DVec3::new(0.0, 64.0, 0.0));
        let east = Some(wither.head(1) + DVec3::new(10.0, 0.0, 0.0));
        wither.turn_heads([east, None]);
        assert_eq!(wither.head_y_rots[0], -10.0);
        for _ in 0..20 {
            wither.turn_heads([east, None]);
        }
        assert!((wither.head_y_rots[0] + 90.0).abs() < 1e-4);
        assert_eq!(wither.head_y_rots[1], 0.0);
        let below = Some(wither.head(1) + DVec3::new(10.0, -10.0, 0.0));
        wither.turn_heads([below, None]);
        assert!(
            (wither.head_x_rots[0] - 40.0).abs() < 1e-4,
            "pitch turns 40 a tick"
        );
        wither.body_yaw = 30.0;
        wither.turn_heads([None, None]);
        assert!((wither.head_y_rots[1] - 10.0).abs() < 1e-4);
        assert_eq!(rotlerp(170.0, -170.0, 10.0), 180.0);
    }

    #[test]
    fn it_rises_above_and_closes_on_its_target() {
        let v = chase(DVec3::new(0.0, -0.5, 0.0), DVec3::ZERO, None, false);
        assert!((v.y + 0.3).abs() < 1e-9);
        let target = DVec3::new(10.0, 2.0, 0.0);
        let v = chase(DVec3::ZERO, DVec3::ZERO, Some(target), false);
        assert!((v.y - 0.3).abs() < 1e-9);
        assert!((v.x - 0.3).abs() < 1e-9 && v.z.abs() < 1e-9);
        let above = chase(
            DVec3::ZERO,
            DVec3::new(0.0, 4.0, 0.0),
            Some(DVec3::new(1.0, 0.0, 0.0)),
            false,
        );
        assert!(
            (above.y - 0.3).abs() < 1e-9,
            "unpowered it keeps five blocks up"
        );
        let powered = chase(
            DVec3::ZERO,
            DVec3::new(0.0, 4.0, 0.0),
            Some(DVec3::new(1.0, 0.0, 0.0)),
            true,
        );
        assert_eq!(powered.y, 0.0, "powered it comes down to its target");
        assert_eq!(powered.x, 0.0, "within three blocks it does not pull");
        assert_eq!(facing(DVec3::new(0.1, 0.0, 0.1)), None);
        assert!((facing(DVec3::new(0.0, 0.0, 1.0)).unwrap()).abs() < 1e-4);
    }

    #[test]
    fn its_flight_turns_up_to_ninety_degrees_and_floats_without_gravity() {
        let idle = fly(None, DVec3::ZERO, 10.0, 0.6, 0.6);
        assert!(!idle.no_gravity && idle.forward == 0.0 && idle.vertical == 0.0);
        let up = fly(
            Some((DVec3::new(0.0, 5.0, 10.0), 1.0)),
            DVec3::ZERO,
            90.0,
            0.0,
            0.0,
        );
        assert!(up.no_gravity);
        assert_eq!(up.speed, 0.6);
        assert_eq!((up.forward, up.vertical), (0.6, 0.6));
        assert!(up.yaw.abs() < 1e-4, "{}", up.yaw);
        let down = fly(
            Some((DVec3::new(-10.0, -1.0, 0.0), 1.0)),
            DVec3::ZERO,
            0.0,
            0.0,
            0.0,
        );
        assert_eq!(down.vertical, -0.6);
        assert!((down.yaw - 90.0).abs() < 1e-4, "{}", down.yaw);
    }

    #[test]
    fn it_takes_no_hits_while_charging_and_none_from_the_undead_or_drowning() {
        let mut wither = Wither::built(DVec3::ZERO, 0.0);
        assert!(wither.refuses("minecraft:player_attack", false, false));
        assert!(!wither.refuses("minecraft:generic_kill", false, true));
        wither.invulnerable_ticks = 0;
        assert!(!wither.refuses("minecraft:player_attack", false, false));
        assert!(wither.refuses("minecraft:drown", false, false));
        assert!(wither.refuses("minecraft:mob_attack", true, false));
        let hit = wither.hurt_armored(10.0);
        assert!((hit.dealt - damage_after_armor(10.0, 4.0, 0.0)).abs() < 1e-6);
        wither.note_hurt();
        assert_eq!(wither.destroy_blocks_tick, 20);
        assert_eq!(wither.idle_head_updates, [3, 3]);
    }
}
