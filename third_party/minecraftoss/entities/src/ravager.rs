use crate::health::DamageState;
use crate::illager::Raider;
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const TYPE_ID: &str = "minecraft:ravager";
pub const WIDTH: f32 = 1.95;
pub const HEIGHT: f32 = 2.2;
pub const EYE_HEIGHT: f32 = HEIGHT * 0.85;
pub const MAX_HEALTH: f32 = 100.0;
pub const BASE_SPEED: f64 = 0.3;
pub const ATTACK_SPEED: f64 = 0.35;
pub const SPEED_LERP: f64 = 0.1;
pub const KNOCKBACK_RESISTANCE: f64 = 0.75;
pub const ATTACK_DAMAGE: f32 = 12.0;
pub const ATTACK_KNOCKBACK: f64 = 1.5;
pub const FOLLOW_RANGE: f64 = 32.0;
pub const STEP_HEIGHT: f32 = 1.0;
pub const EXPERIENCE: i32 = 20;
pub const AMBIENT_INTERVAL: i32 = 80;
pub const MAX_HEAD_Y_ROT: f32 = 45.0;
pub const ATTACK_DURATION: i32 = 10;
pub const STUN_DURATION: i32 = 40;
pub const ROAR_DURATION: i32 = 20;
pub const ROAR_AT: i32 = 10;
pub const ROAR_RANGE: f64 = 4.0;
pub const ROAR_DAMAGE: f32 = 6.0;
pub const STUN_CHANCE: f64 = 0.5;
pub const STUN_EFFECT_ROLL: u32 = 6;
pub const STUNNED_COLOR: [f32; 3] = [0.498_039_22, 0.513_725_5, 0.572_549_05];
pub const LEAF_REACH: f64 = 0.2;
pub const ATTACK_REACH_SHRINK: f64 = 0.05;
pub const PASSENGER_ATTACHMENT: (f32, f32) = (2.2625, -0.0625);
pub const MELEE_SPEED: f64 = 1.0;
pub const STROLL_SPEED: f64 = 0.4;
pub const PLAYER_LOOK_RANGE: f64 = 6.0;
pub const MOB_LOOK_RANGE: f64 = 8.0;
pub const LOOK_PROBABILITY: f32 = 0.02;
pub const ATTACK_SOUND: &str = "entity.ravager.attack";
pub const STUNNED_SOUND: &str = "entity.ravager.stunned";
pub const ROAR_SOUND: &str = "entity.ravager.roar";
pub const STEP_SOUND: &str = "entity.ravager.step";
pub const ROAR_PARTICLES: usize = 40;
pub const ROAR_SPREAD: f64 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    Ignored,
    Stunned,
    Thrown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timers {
    pub roars: bool,
    pub roar_sound: bool,
    pub stun_effect: bool,
}

#[derive(Clone, Debug)]
pub struct Ravager {
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub persistence_required: bool,
    pub left_handed: bool,
    pub raider: Raider,
    pub attack_tick: i32,
    pub stunned_tick: i32,
    pub roar_tick: i32,
    pub speed: f64,
}

impl Ravager {
    pub fn new(position: DVec3) -> Self {
        let mut body = Body::new(position, WIDTH, HEIGHT);
        body.step_height = STEP_HEIGHT;
        Self {
            body,
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            persistence_required: false,
            left_handed: false,
            raider: Raider::default(),
            attack_tick: 0,
            stunned_tick: 0,
            roar_tick: 0,
            speed: BASE_SPEED,
        }
    }

    pub fn is_immobile(&self) -> bool {
        self.health <= 0.0 || self.attack_tick > 0 || self.stunned_tick > 0 || self.roar_tick > 0
    }

    pub fn update_speed(&mut self, has_target: bool) {
        if self.is_immobile() {
            self.speed = 0.0;
        } else {
            let wanted = if has_target { ATTACK_SPEED } else { BASE_SPEED };
            self.speed += SPEED_LERP * (wanted - self.speed);
        }
    }

    pub fn attacked(&mut self) {
        self.attack_tick = ATTACK_DURATION;
    }

    pub fn blocked_by_item(&mut self, random: &mut LegacyRandom) -> Blocked {
        if self.roar_tick != 0 {
            return Blocked::Ignored;
        }
        if random.next_double() < STUN_CHANCE {
            self.stunned_tick = STUN_DURATION;
            Blocked::Stunned
        } else {
            Blocked::Thrown
        }
    }

    pub fn tick_timers(&mut self) -> Timers {
        let mut timers = Timers::default();
        if self.roar_tick > 0 {
            self.roar_tick -= 1;
            timers.roars = self.roar_tick == ROAR_AT;
        }
        if self.attack_tick > 0 {
            self.attack_tick -= 1;
        }
        if self.stunned_tick > 0 {
            self.stunned_tick -= 1;
            timers.stun_effect = true;
            if self.stunned_tick == 0 {
                timers.roar_sound = true;
                self.roar_tick = ROAR_DURATION;
            }
        }
        timers
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        let p = self.body.position;
        let half = f64::from(self.body.width / 2.0);
        (
            DVec3::new(p.x - half, p.y, p.z - half),
            DVec3::new(p.x + half, p.y + f64::from(self.body.height), p.z + half),
        )
    }

    pub fn leaf_cells(&self) -> Vec<(i32, i32, i32)> {
        let (min, max) = self.bounds();
        let grow = DVec3::splat(LEAF_REACH);
        let (min, max) = (min - grow, max + grow);
        let mut cells = Vec::new();
        for x in min.x.floor() as i32..=max.x.floor() as i32 {
            for y in min.y.floor() as i32..=max.y.floor() as i32 {
                for z in min.z.floor() as i32..=max.z.floor() as i32 {
                    cells.push((x, y, z));
                }
            }
        }
        cells
    }

    pub fn attack_throw(&self) -> DVec3 {
        let power = ATTACK_KNOCKBACK * 0.5;
        let angle = f64::from(self.yaw * 0.017_453_292);
        let (sin, cos) = (
            f64::from(minecraftoss_player::mth::sin(angle)),
            f64::from(minecraftoss_player::mth::cos(angle)),
        );
        DVec3::new(-sin * power, power.min(0.4), cos * power)
    }

    pub fn stun_particle(&self, random: &mut LegacyRandom, body_yaw: f32) -> Option<DVec3> {
        if random.next_int(STUN_EFFECT_ROLL) != 0 {
            return None;
        }
        let p = self.body.position;
        let width = f64::from(self.body.width);
        let angle = f64::from(body_yaw * 0.017_453_292);
        let x = p.x - width * angle.sin() + (random.next_double() * 0.6 - 0.3);
        let y = p.y + f64::from(self.body.height) - 0.3;
        let z = p.z + width * angle.cos() + (random.next_double() * 0.6 - 0.3);
        Some(DVec3::new(x, y, z))
    }
}

pub fn is_leaves(id: &str) -> bool {
    id.ends_with("_leaves")
}

pub fn strong_knockback(ravager: DVec3, other: DVec3) -> DVec3 {
    let (dx, dz) = (other.x - ravager.x, other.z - ravager.z);
    let distance = (dx * dx + dz * dz).max(0.001);
    DVec3::new(dx / distance * ROAR_RANGE, 0.2, dz / distance * ROAR_RANGE)
}

pub fn roar_hurts(kind: &str) -> bool {
    !crate::illager::is_illager(kind)
}

pub fn passenger_point(yaw: f32) -> DVec3 {
    let (y, z) = PASSENGER_ATTACHMENT;
    let angle = f64::from(-yaw * 0.017_453_292_f32);
    DVec3::new(
        f64::from(z) * f64::from(minecraftoss_player::mth::sin(angle)),
        f64::from(y),
        f64::from(z) * f64::from(minecraftoss_player::mth::cos(angle)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ravager_is_a_big_hundred_heart_raider() {
        let ravager = Ravager::new(DVec3::new(0.5, 64.0, 0.5));
        assert_eq!(
            (ravager.body.width, ravager.body.height, ravager.health),
            (1.95, 2.2, 100.0)
        );
        assert!((EYE_HEIGHT - 1.87).abs() < 1e-5);
        assert_eq!(ravager.body.step_height, 1.0);
        assert_eq!(ravager.speed, BASE_SPEED);
        assert!(crate::illager::is_raider(TYPE_ID));
        assert!(!ravager.is_immobile());
    }

    #[test]
    fn a_blocked_ravager_is_stunned_for_forty_ticks_then_roars_ten_ticks_in() {
        let mut stuns = 0;
        let mut seeds = LegacyRandom::new(42);
        for _ in 0..20 {
            let seed = seeds.next_long() as u64;
            let mut ravager = Ravager::new(DVec3::ZERO);
            let mut random = LegacyRandom::new(seed);
            let mut check = LegacyRandom::new(seed);
            let stunned = check.next_double() < STUN_CHANCE;
            let blocked = ravager.blocked_by_item(&mut random);
            if !stunned {
                assert_eq!(blocked, Blocked::Thrown);
                assert_eq!(ravager.stunned_tick, 0);
                continue;
            }
            stuns += 1;
            assert_eq!(blocked, Blocked::Stunned);
            let mut ticks = Vec::new();
            for tick in 1..=70 {
                assert_eq!(ravager.is_immobile(), tick <= 60, "tick {tick}");
                ticks.push((tick, ravager.tick_timers()));
            }
            let stun_effects = ticks.iter().filter(|(_, t)| t.stun_effect).count();
            assert_eq!(stun_effects, 40);
            let sounds: Vec<i32> = ticks
                .iter()
                .filter(|(_, t)| t.roar_sound)
                .map(|(tick, _)| *tick)
                .collect();
            assert_eq!(sounds, vec![40]);
            let roars: Vec<i32> = ticks
                .iter()
                .filter(|(_, t)| t.roars)
                .map(|(tick, _)| *tick)
                .collect();
            assert_eq!(roars, vec![50]);
        }
        assert!(stuns > 3 && stuns < 17, "{stuns}");
    }

    #[test]
    fn a_roaring_ravager_ignores_a_block() {
        let mut ravager = Ravager::new(DVec3::ZERO);
        ravager.roar_tick = 5;
        let mut random = LegacyRandom::new(1);
        assert_eq!(ravager.blocked_by_item(&mut random), Blocked::Ignored);
        assert_eq!(ravager.stunned_tick, 0);
    }

    #[test]
    fn an_attack_holds_it_still_for_nine_more_ticks_and_slows_it() {
        let mut ravager = Ravager::new(DVec3::ZERO);
        ravager.attacked();
        ravager.update_speed(true);
        assert_eq!(ravager.speed, 0.0);
        let still = (0..20)
            .take_while(|_| {
                ravager.tick_timers();
                ravager.is_immobile()
            })
            .count();
        assert_eq!(still, 9);
        ravager.update_speed(true);
        assert!((ravager.speed - 0.035).abs() < 1e-9);
        for _ in 0..200 {
            ravager.update_speed(true);
        }
        assert!((ravager.speed - ATTACK_SPEED).abs() < 1e-6);
    }

    #[test]
    fn the_roar_flings_sideways_harder_the_closer_it_is() {
        let near = strong_knockback(DVec3::ZERO, DVec3::new(1.0, 0.0, 0.0));
        assert_eq!(near, DVec3::new(4.0, 0.2, 0.0));
        let far = strong_knockback(DVec3::ZERO, DVec3::new(0.0, 5.0, -4.0));
        assert!((far.z + 1.0).abs() < 1e-9 && far.x == 0.0);
        assert!(roar_hurts("minecraft:player") && roar_hurts("minecraft:witch"));
        assert!(!roar_hurts("minecraft:pillager"));
    }

    #[test]
    fn its_rider_sits_two_and_a_quarter_blocks_up_a_touch_behind() {
        let point = passenger_point(0.0);
        assert!((point - DVec3::new(0.0, 2.2625, -0.0625)).length() < 1e-6);
        let turned = passenger_point(90.0);
        assert!(
            (turned.x - 0.0625).abs() < 1e-6 && turned.z.abs() < 1e-6,
            "{turned}"
        );
    }

    #[test]
    fn its_hit_throws_forward_and_up() {
        let ravager = Ravager::new(DVec3::ZERO);
        let throw = ravager.attack_throw();
        assert!(
            (throw - DVec3::new(0.0, 0.4, 0.75)).length() < 1e-6,
            "{throw}"
        );
        assert!(is_leaves("minecraft:oak_leaves") && is_leaves("minecraft:azalea_leaves"));
        assert!(!is_leaves("minecraft:oak_log"));
        assert_eq!(ravager.leaf_cells().len(), 4 * 4 * 4);
    }
}
