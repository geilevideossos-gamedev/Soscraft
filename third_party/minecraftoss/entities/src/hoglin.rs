use crate::age::Age;
use crate::health::{DamageResult, DamageState};
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const MAX_HEALTH: f32 = 40.0;
pub const MOVEMENT_SPEED: f64 = 0.30000001192092896;
pub const KNOCKBACK_RESISTANCE: f64 = 0.6000000238418579;
pub const ATTACK_KNOCKBACK: f64 = 1.0;
pub const ATTACK_DAMAGE: f32 = 6.0;
pub const BABY_ATTACK_DAMAGE: f32 = 0.5;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const WIDTH: f32 = 1.3964844;
pub const HEIGHT: f32 = 1.4;
pub const EYE_HEIGHT: f32 = HEIGHT * 0.85;
pub const BABY_WIDTH: f32 = 0.75;
pub const BABY_HEIGHT: f32 = 0.85;
pub const BABY_EYE_HEIGHT: f32 = 0.625;
pub const EXPERIENCE: i32 = 5;
pub const BABY_EXPERIENCE: i32 = 3;
pub const CONVERSION_TIME: i32 = 300;
pub const ATTACK_ANIMATION_DURATION: i32 = 10;
pub const BABY_CHANCE: f32 = 0.2;
pub const AMBIENT_INTERVAL: i32 = 80;
pub const STEP_VOLUME: f32 = 0.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoglinKind {
    Hoglin,
    Zoglin,
}

impl HoglinKind {
    pub fn type_id(self) -> &'static str {
        match self {
            Self::Hoglin => "minecraft:hoglin",
            Self::Zoglin => "minecraft:zoglin",
        }
    }

    pub fn family(self) -> &'static str {
        match self {
            Self::Hoglin => "hoglin",
            Self::Zoglin => "zoglin",
        }
    }

    pub fn from_type(id: &str) -> Option<Self> {
        match id {
            "minecraft:hoglin" => Some(Self::Hoglin),
            "minecraft:zoglin" => Some(Self::Zoglin),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Hoglin {
    pub kind: HoglinKind,
    pub body: Body,
    pub yaw: f32,
    pub health: f32,
    pub damage: DamageState,
    pub age: Age,
    pub baby: bool,
    pub in_love: i32,
    pub attack_animation: i32,
    pub time_in_overworld: i32,
    pub immune_to_zombification: bool,
    pub cannot_be_hunted: bool,
    pub persistence_required: bool,
    pub converting: bool,
}

impl Hoglin {
    pub fn new(kind: HoglinKind, position: DVec3) -> Self {
        Self {
            kind,
            body: Body::new(position, WIDTH, HEIGHT),
            yaw: 0.0,
            health: MAX_HEALTH,
            damage: DamageState::default(),
            age: Age::default(),
            baby: false,
            in_love: 0,
            attack_animation: 0,
            time_in_overworld: 0,
            immune_to_zombification: false,
            cannot_be_hunted: false,
            persistence_required: false,
            converting: false,
        }
    }

    pub fn is_baby(&self) -> bool {
        match self.kind {
            HoglinKind::Hoglin => self.age.baby(),
            HoglinKind::Zoglin => self.baby,
        }
    }

    pub fn set_baby(&mut self, baby: bool) {
        match self.kind {
            HoglinKind::Hoglin => {
                self.age.set(if baby { Age::BABY_START } else { 0 });
            }
            HoglinKind::Zoglin => self.baby = baby,
        }
        self.sync_dimensions();
    }

    pub fn dimensions(baby: bool) -> (f32, f32) {
        if baby {
            (BABY_WIDTH, BABY_HEIGHT)
        } else {
            (WIDTH, HEIGHT)
        }
    }

    pub fn sync_dimensions(&mut self) {
        let (width, height) = Self::dimensions(self.is_baby());
        self.body.width = width;
        self.body.height = height;
    }

    pub fn eye_height(&self) -> f32 {
        if self.is_baby() {
            BABY_EYE_HEIGHT
        } else {
            EYE_HEIGHT
        }
    }

    pub fn attack_damage(&self) -> f32 {
        if self.is_baby() {
            BABY_ATTACK_DAMAGE
        } else {
            ATTACK_DAMAGE
        }
    }

    pub fn experience(&self) -> i32 {
        if self.kind == HoglinKind::Hoglin && self.is_baby() {
            BABY_EXPERIENCE
        } else {
            EXPERIENCE
        }
    }

    pub fn fire_immune(&self) -> bool {
        self.kind == HoglinKind::Zoglin
    }

    pub fn hurt_generic(&mut self, amount: f32) -> DamageResult {
        self.damage
            .hurt_generic(&mut self.health, MAX_HEALTH, amount)
    }

    pub fn tick_attack_animation(&mut self) {
        if self.attack_animation > 0 {
            self.attack_animation -= 1;
        }
    }
}

pub fn damage_roll(attack_damage: f32, baby: bool, random: &mut LegacyRandom) -> f32 {
    if !baby && attack_damage as i32 > 0 {
        attack_damage / 2.0 + random.next_int(attack_damage as i32 as u32) as f32
    } else {
        attack_damage
    }
}

pub fn throw_push(
    attacker: DVec3,
    target: DVec3,
    target_resistance: f64,
    random: &mut LegacyRandom,
) -> DVec3 {
    let k = ATTACK_KNOCKBACK - target_resistance;
    if k <= 0.0 {
        return DVec3::ZERO;
    }
    let (dx, dz) = (target.x - attacker.x, target.z - attacker.z);
    let turn = (random.next_int(21) as i32 - 10) as f32;
    let scale = k * f64::from(random.next_float() * 0.5 + 0.2);
    let length = (dx * dx + dz * dz).sqrt();
    let flat = if length < f64::from(1.0e-5_f32) {
        DVec3::ZERO
    } else {
        DVec3::new(dx / length, 0.0, dz / length) * scale
    };
    let cos = f64::from(minecraftoss_player::mth::cos(f64::from(turn)));
    let sin = f64::from(minecraftoss_player::mth::sin(f64::from(turn)));
    let x = flat.x * cos + flat.z * sin;
    let z = flat.z * cos - flat.x * sin;
    let up = k * f64::from(random.next_float()) * 0.5;
    DVec3::new(x, up, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_adult_rolls_three_to_eight_and_a_baby_half_a_heart() {
        let mut random = LegacyRandom::new(3);
        let rolls: Vec<f32> = (0..400)
            .map(|_| damage_roll(ATTACK_DAMAGE, false, &mut random))
            .collect();
        assert!(rolls.iter().all(|d| (3.0..=8.0).contains(d)));
        assert!(rolls.contains(&3.0) && rolls.contains(&8.0));
        assert_eq!(damage_roll(BABY_ATTACK_DAMAGE, true, &mut random), 0.5);
    }

    #[test]
    fn the_throw_lifts_at_most_half_a_block_and_spares_full_resistance() {
        let mut random = LegacyRandom::new(9);
        for _ in 0..200 {
            let push = throw_push(DVec3::ZERO, DVec3::new(0.0, 0.0, 2.0), 0.0, &mut random);
            assert!((0.0..=0.5).contains(&push.y), "{push}");
            let flat = (push.x * push.x + push.z * push.z).sqrt();
            assert!((0.2 - 1.0e-3..=0.7 + 1.0e-3).contains(&flat), "{flat}");
        }
        assert_eq!(
            throw_push(DVec3::ZERO, DVec3::Z, 1.0, &mut random),
            DVec3::ZERO
        );
        let hoglin = throw_push(DVec3::ZERO, DVec3::Z, KNOCKBACK_RESISTANCE, &mut random);
        assert!(hoglin.y <= 0.2 + 1.0e-6);
    }

    #[test]
    fn a_hoglin_baby_is_aged_and_a_zoglin_baby_is_flagged() {
        let mut hoglin = Hoglin::new(HoglinKind::Hoglin, DVec3::ZERO);
        hoglin.set_baby(true);
        assert!(hoglin.is_baby() && hoglin.age.ticks == Age::BABY_START);
        assert_eq!((hoglin.body.width, hoglin.body.height), (0.75, 0.85));
        assert_eq!(hoglin.experience(), 3);
        let mut zoglin = Hoglin::new(HoglinKind::Zoglin, DVec3::ZERO);
        zoglin.set_baby(true);
        assert!(zoglin.is_baby() && zoglin.age.ticks == 0);
        assert_eq!(zoglin.experience(), 5);
        assert_eq!(zoglin.attack_damage(), 0.5);
    }
}
