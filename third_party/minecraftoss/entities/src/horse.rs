//! Horses and donkeys (26.3 `AbstractHorse`, `Horse`, `AbstractChestedHorse`,
//! `Donkey`): their flags, counters and animation values, grazing and
//! rearing, and the attributes their base definitions give (summoned with
//! a tag; `finalizeSpawn` rolls a horse's health, speed and jump). They
//! live in the entity world as farm animals with this state beside them,
//! on the horse goal set (`horse_ai`).
use crate::walk_path::WalkProfile;

/// Which `AbstractHorse` it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorseKind {
    Horse,
    Donkey,
    Mule,
    Llama,
    TraderLlama,
}

impl HorseKind {
    pub fn type_id(self) -> &'static str {
        match self {
            HorseKind::Horse => "minecraft:horse",
            HorseKind::Donkey => "minecraft:donkey",
            HorseKind::Mule => "minecraft:mule",
            HorseKind::Llama => "minecraft:llama",
            HorseKind::TraderLlama => "minecraft:trader_llama",
        }
    }

    pub fn from_type_id(id: &str) -> Option<HorseKind> {
        match id {
            "minecraft:horse" => Some(HorseKind::Horse),
            "minecraft:donkey" => Some(HorseKind::Donkey),
            "minecraft:mule" => Some(HorseKind::Mule),
            "minecraft:llama" => Some(HorseKind::Llama),
            "minecraft:trader_llama" => Some(HorseKind::TraderLlama),
            _ => None,
        }
    }

    pub fn is_llama(self) -> bool {
        matches!(self, HorseKind::Llama | HorseKind::TraderLlama)
    }

    pub fn is_chested(self) -> bool {
        self != HorseKind::Horse
    }

    pub fn can_rear(self) -> bool {
        !self.is_llama()
    }

    pub fn can_eat_grass(self) -> bool {
        !self.is_llama()
    }

    pub fn max_temper(self) -> i32 {
        if self.is_llama() { 30 } else { 100 }
    }

    pub fn can_mate_with(self, other: HorseKind) -> bool {
        match self {
            HorseKind::Horse | HorseKind::Donkey => matches!(other, HorseKind::Horse | HorseKind::Donkey),
            HorseKind::Mule => false,
            HorseKind::Llama | HorseKind::TraderLlama => other.is_llama(),
        }
    }

    pub fn offspring(self, partner: HorseKind) -> HorseKind {
        match (self, partner) {
            (HorseKind::Horse, HorseKind::Donkey) | (HorseKind::Donkey, HorseKind::Horse) => HorseKind::Mule,
            (kind, _) => kind,
        }
    }

    pub fn food_tag(self) -> &'static str {
        if self.is_llama() { "minecraft:llama_food" } else { "minecraft:horse_food" }
    }

    pub fn tempt_tag(self) -> &'static str {
        if self.is_llama() { "minecraft:llama_tempt_items" } else { "minecraft:horse_tempt_items" }
    }

    pub fn eats(self, item: &str) -> bool {
        if self.is_llama() {
            matches!(item, "minecraft:wheat" | "minecraft:hay_block")
        } else {
            matches!(
                item,
                "minecraft:wheat" | "minecraft:sugar" | "minecraft:hay_block" | "minecraft:apple" | "minecraft:carrot" | "minecraft:golden_carrot" | "minecraft:golden_apple" | "minecraft:enchanted_golden_apple"
            )
        }
    }

    pub fn sound(self, baby: bool, event: &str) -> String {
        format!("entity.{}.{event}", self.sound_family(baby))
    }

    /// Adult width and height (`EntityTypes`), and a baby's scale
    /// (`Horse.BABY_DIMENSIONS` 0.7; `AbstractChestedHorse` 0.5).
    pub fn dimensions(self) -> (f32, f32, f32) {
        match self {
            HorseKind::Horse => (1.3964844, 1.6, 0.7),
            HorseKind::Donkey => (1.3964844, 1.5, 0.5),
            HorseKind::Mule => (1.3964844, 1.6, 0.5),
            HorseKind::Llama | HorseKind::TraderLlama => (0.9, 1.87, 0.5),
        }
    }

    /// `EntityTypes` eye height, scaled with a baby's dimensions.
    pub fn eye_height(self, baby: bool) -> f32 {
        let (_, _, scale) = self.dimensions();
        let eye = match self {
            HorseKind::Horse => 1.52,
            HorseKind::Donkey => 1.425,
            HorseKind::Mule => 1.52,
            HorseKind::Llama | HorseKind::TraderLlama => 1.7765,
        };
        if baby {
            eye * scale
        } else {
            eye
        }
    }

    /// `createBaseHorseAttributes` (and `createBaseChestedHorseAttributes`
    /// for donkeys): max health, movement speed and jump strength.
    pub fn base_attributes(self) -> (f32, f64, f64) {
        match self {
            HorseKind::Horse => (53.0, f64::from(0.225_f32), 0.7),
            HorseKind::Donkey | HorseKind::Mule | HorseKind::Llama | HorseKind::TraderLlama => (53.0, f64::from(0.175_f32), 0.5),
        }
    }

    /// Its sound family: a baby horse has its own (`entity.baby_horse.*`).
    pub fn sound_family(self, baby: bool) -> &'static str {
        match self {
            HorseKind::Horse if baby => "baby_horse",
            HorseKind::Horse => "horse",
            HorseKind::Donkey => "donkey",
            HorseKind::Mule => "mule",
            HorseKind::Llama | HorseKind::TraderLlama => "llama",
        }
    }
}

/// `AbstractHorse`'s state beside the animal it is.
#[derive(Clone, Debug)]
pub struct HorseState {
    pub kind: HorseKind,
    /// `Horse`'s type variant: the coat in the low byte, the markings in
    /// the next.
    pub variant: i32,
    /// `AbstractChestedHorse.hasChest`.
    pub chest: bool,
    /// `DATA_ID_FLAGS`.
    pub tame: bool,
    pub bred: bool,
    pub eating: bool,
    pub standing: bool,
    pub mouth_open: bool,
    pub eating_counter: i32,
    pub mouth_counter: i32,
    pub stand_counter: i32,
    pub tail_counter: i32,
    pub sprint_counter: i32,
    pub temper: i32,
    pub eat_anim: f32,
    pub eat_anim_o: f32,
    pub stand_anim: f32,
    pub stand_anim_o: f32,
    pub mouth_anim: f32,
    pub mouth_anim_o: f32,
    pub allow_stand_sliding: bool,
    /// `RandomStandGoal.nextStand`.
    pub next_stand: i32,
    pub max_health: f32,
    pub movement_speed: f64,
    pub jump_strength: f64,
    pub strength: i32,
    pub body_item: Option<String>,
    pub inventory: Vec<minecraftoss_player::inventory::ItemStack>,
    pub leashed: bool,
    pub despawn_delay: i32,
    pub fight: LlamaFight,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LlamaFight {
    pub target: Option<crate::monster_ai::Target>,
    pub hurt_by: Option<(crate::monster_ai::Target, i32)>,
    pub hurt_seen: i32,
    pub did_spit: bool,
    pub see_time: i32,
    pub attack_time: i32,
    pub unseen_ticks: i32,
    pub caravan_head: Option<u64>,
    pub caravan_tail: Option<u64>,
    pub caravan_speed: f64,
    pub dist_check: i32,
}

impl Default for LlamaFight {
    fn default() -> Self {
        Self {
            target: None,
            hurt_by: None,
            hurt_seen: 0,
            did_spit: false,
            see_time: 0,
            attack_time: -1,
            unseen_ticks: 0,
            caravan_head: None,
            caravan_tail: None,
            caravan_speed: CARAVAN_SPEED,
            dist_check: 0,
        }
    }
}

pub const CARAVAN_SPEED: f64 = 2.0999999046325684;
pub const TRADER_DESPAWN_DELAY: i32 = 47999;
pub const MAX_STRENGTH: i32 = 5;
pub const MIN_HEALTH: f64 = 15.0;
pub const MAX_HEALTH: f64 = 30.0;
pub const MIN_JUMP_STRENGTH: f64 = 0.4_f32 as f64;
pub const MAX_JUMP_STRENGTH: f64 = 1.0;
pub const MIN_MOVEMENT_SPEED: f64 = 0.1125_f32 as f64;
pub const MAX_MOVEMENT_SPEED: f64 = 0.3375_f32 as f64;

pub fn offspring_attribute(a: f64, b: f64, min: f64, max: f64, random: &mut minecraftoss_player::rng::LegacyRandom) -> f64 {
    let a = a.clamp(min, max);
    let b = b.clamp(min, max);
    let margin = 0.15 * (max - min);
    let spread = (a - b).abs() + margin * 2.0;
    let average = (a + b) / 2.0;
    let roll = (random.next_double() + random.next_double() + random.next_double()) / 3.0 - 0.5;
    let value = average + spread * roll;
    if value > max {
        max - (value - max)
    } else if value < min {
        min + (min - value)
    } else {
        value
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Feeding {
    pub heal: f32,
    pub age_up: i32,
    pub temper: i32,
    pub love: bool,
}

impl Feeding {
    pub fn of(kind: HorseKind, item: &str) -> Option<Feeding> {
        let (heal, age_up, temper, love) = if kind.is_llama() {
            match item {
                "minecraft:wheat" => (2.0, 10, 3, false),
                "minecraft:hay_block" => (10.0, 90, 6, true),
                _ => return None,
            }
        } else {
            match item {
                "minecraft:wheat" => (2.0, 20, 3, false),
                "minecraft:sugar" => (1.0, 30, 3, false),
                "minecraft:hay_block" => (20.0, 180, 0, false),
                "minecraft:apple" | "minecraft:carrot" => (3.0, 60, 3, false),
                "minecraft:red_mushroom" => (3.0, 0, 3, false),
                "minecraft:golden_carrot" => (4.0, 60, 5, true),
                "minecraft:golden_apple" | "minecraft:enchanted_golden_apple" => (10.0, 240, 10, true),
                _ => return None,
            }
        };
        Some(Feeding { heal, age_up, temper, love })
    }
}

impl HorseState {
    pub fn new(kind: HorseKind) -> Self {
        let (max_health, movement_speed, jump_strength) = kind.base_attributes();
        Self {
            kind,
            variant: 0,
            chest: false,
            tame: false,
            bred: false,
            eating: false,
            standing: false,
            mouth_open: false,
            eating_counter: 0,
            mouth_counter: 0,
            stand_counter: 0,
            tail_counter: 0,
            sprint_counter: 0,
            temper: 0,
            eat_anim: 0.0,
            eat_anim_o: 0.0,
            stand_anim: 0.0,
            stand_anim_o: 0.0,
            mouth_anim: 0.0,
            mouth_anim_o: 0.0,
            allow_stand_sliding: false,
            // `RandomStandGoal`'s constructor: `-getAmbientStandInterval()`.
            next_stand: -AMBIENT_SOUND_INTERVAL,
            max_health,
            movement_speed,
            jump_strength,
            strength: 0,
            body_item: None,
            inventory: Vec::new(),
            leashed: false,
            despawn_delay: TRADER_DESPAWN_DELAY,
            fight: LlamaFight::default(),
        }
    }

    pub fn set_strength(&mut self, strength: i32) {
        self.strength = strength.clamp(1, MAX_STRENGTH);
    }

    pub fn inventory_columns(&self) -> i32 {
        match self.kind {
            _ if !self.chest => 0,
            HorseKind::Llama | HorseKind::TraderLlama => self.strength,
            _ => 5,
        }
    }

    pub fn modify_temper(&mut self, by: i32) -> i32 {
        self.temper = (self.temper + by).clamp(0, self.kind.max_temper());
        self.temper
    }

    pub fn in_caravan(&self) -> bool {
        self.fight.caravan_head.is_some()
    }

    pub fn has_caravan_tail(&self) -> bool {
        self.fight.caravan_tail.is_some()
    }

    /// `isImmobile`: grazing or rearing, it neither moves nor thinks.
    pub fn immobile(&self) -> bool {
        self.eating || self.standing
    }

    /// `setStanding(ticks)`.
    pub fn set_standing(&mut self, ticks: i32) {
        self.eating = false;
        self.standing = true;
        self.stand_counter = ticks;
    }

    /// `clearStanding`.
    pub fn clear_standing(&mut self) {
        self.standing = false;
        self.stand_counter = 0;
    }

    /// `standIfPossible` (every horse and donkey can rear): twenty ticks.
    pub fn stand_if_possible(&mut self) {
        if self.kind.can_rear() {
            self.set_standing(20);
        }
    }

    /// The rest of `AbstractHorse.tick`, after the living tick: the mouth,
    /// rearing, tail and sprint counters, then the eating, standing and
    /// mouth animations ease towards their flags.
    pub fn tick_animation(&mut self) {
        if self.mouth_counter > 0 {
            self.mouth_counter += 1;
            if self.mouth_counter > 30 {
                self.mouth_counter = 0;
                self.mouth_open = false;
            }
        }
        if self.stand_counter > 0 {
            self.stand_counter -= 1;
            if self.stand_counter <= 0 {
                self.clear_standing();
            }
        }
        if self.tail_counter > 0 {
            self.tail_counter += 1;
            if self.tail_counter > 8 {
                self.tail_counter = 0;
            }
        }
        if self.sprint_counter > 0 {
            self.sprint_counter += 1;
            if self.sprint_counter > 300 {
                self.sprint_counter = 0;
            }
        }
        self.eat_anim_o = self.eat_anim;
        if self.eating {
            self.eat_anim += (1.0 - self.eat_anim) * 0.4 + 0.05;
            if self.eat_anim > 1.0 {
                self.eat_anim = 1.0;
            }
        } else {
            self.eat_anim += (0.0 - self.eat_anim) * 0.4 - 0.05;
            if self.eat_anim < 0.0 {
                self.eat_anim = 0.0;
            }
        }
        self.stand_anim_o = self.stand_anim;
        if self.standing {
            self.eat_anim = 0.0;
            self.eat_anim_o = self.eat_anim;
            self.stand_anim += (1.0 - self.stand_anim) * 0.4 + 0.05;
            if self.stand_anim > 1.0 {
                self.stand_anim = 1.0;
            }
        } else {
            self.allow_stand_sliding = false;
            self.stand_anim += (0.8 * self.stand_anim * self.stand_anim * self.stand_anim - self.stand_anim) * 0.6 - 0.05;
            if self.stand_anim < 0.0 {
                self.stand_anim = 0.0;
            }
        }
        self.mouth_anim_o = self.mouth_anim;
        if self.mouth_open {
            self.mouth_anim += (1.0 - self.mouth_anim) * 0.7 + 0.05;
            if self.mouth_anim > 1.0 {
                self.mouth_anim = 1.0;
            }
        } else {
            self.mouth_anim += (0.0 - self.mouth_anim) * 0.7 - 0.05;
            if self.mouth_anim < 0.0 {
                self.mouth_anim = 0.0;
            }
        }
    }

    /// Its walk search: an animal's, stepping up a whole block
    /// (`STEP_HEIGHT` 1).
    pub fn walk_profile(width: f32, height: f32) -> WalkProfile {
        let mut profile = WalkProfile::animal(width, height);
        profile.max_up_step = 1.0;
        profile
    }
}

/// `AbstractHorse.getAmbientSoundInterval` (and its stand interval).
pub const AMBIENT_SOUND_INTERVAL: i32 = 400;
/// `AbstractHorse.getSoundVolume`.
pub const SOUND_VOLUME: f32 = 0.8;
/// `SAFE_FALL_DISTANCE` and `FALL_DAMAGE_MULTIPLIER`.
pub const SAFE_FALL_DISTANCE: f64 = 6.0;
pub const FALL_DAMAGE_MULTIPLIER: f64 = 0.5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grazing_eases_in_and_rearing_clears_it() {
        let mut horse = HorseState::new(HorseKind::Horse);
        horse.eating = true;
        horse.tick_animation();
        assert_eq!(horse.eat_anim, 0.4_f32 + 0.05);
        horse.stand_if_possible();
        assert!(!horse.eating && horse.standing);
        horse.tick_animation();
        assert_eq!(horse.eat_anim, 0.0);
        for _ in 0..19 {
            horse.tick_animation();
        }
        assert!(!horse.standing, "twenty ticks of rearing");
    }
}
