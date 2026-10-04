use minecraftoss_player::GameMode;
use minecraftoss_player::food::{FoodUse, FoodUseTick, apply_consumed_food_effects};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::survival::{Armor, Difficulty, EffectKind, SurvivalStatus};

const HURT_EXHAUSTION: f32 = 0.1;
pub(crate) const ATTACK_EXHAUSTION: f32 = 0.1;
pub(crate) const MINE_EXHAUSTION: f32 = 0.005;
const JUMP_EXHAUSTION: f32 = 0.05;
const SPRINT_JUMP_EXHAUSTION: f32 = 0.2;
const SPRINT_EXHAUSTION: f32 = 0.1;
const SWIM_EXHAUSTION: f32 = 0.01;
const SPRINT_FOOD: u8 = 6;
const SAFE_FALL: f64 = 3.0;
const MAX_STRIDE: f64 = 2.0;
const DAMAGE_BLINK: u64 = 20;
const SETTLE_DISPLAY: u64 = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Motion {
    pub(crate) feet: [f64; 3],
    pub(crate) on_ground: bool,
    pub(crate) sprinting: bool,
    pub(crate) jumping: bool,
    pub(crate) in_water: bool,
    pub(crate) eye_in_water: bool,
}

#[derive(Default)]
pub(crate) struct Survival {
    pub(crate) status: SurvivalStatus,
    pub(crate) eye_in_water: bool,
    bubbles: i32,
    tick: u64,
    synced: Option<i32>,
    source: Option<[f64; 3]>,
    eating: Option<FoodUse>,
    last_feet: Option<[f64; 3]>,
    jumping: bool,
    last_health: i32,
    display_health: i32,
    changed_at: u64,
    blink_until: u64,
    blocked_sprint: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Breath {
    pub(crate) drowned: bool,
    pub(crate) popped: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Bite {
    Nothing,
    Chew(String),
    Done {
        sound: String,
        overflow: Option<ItemStack>,
        fed: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Settle {
    pub(crate) damage: Option<(i32, Option<[f64; 3]>)>,
    pub(crate) set: Option<i32>,
}

fn scale(max_health: i32) -> f32 {
    max_health.max(1) as f32 / 20.0
}

impl Survival {
    pub(crate) fn respawn(&mut self) {
        *self = Self {
            blocked_sprint: self.blocked_sprint,
            ..Self::default()
        };
    }

    pub(crate) fn sprint_change(&mut self, blocked: bool) -> Option<bool> {
        let hungry = !self.can_sprint();
        if hungry && !blocked {
            self.blocked_sprint = true;
            return Some(true);
        }
        if !hungry && blocked && self.blocked_sprint {
            self.blocked_sprint = false;
            return Some(false);
        }
        if !blocked {
            self.blocked_sprint = false;
        }
        None
    }

    pub(crate) fn observe(&mut self, health: i32, max_health: i32) {
        let scale = scale(max_health);
        let Some(synced) = self.synced else {
            self.synced = Some(health);
            self.status.health = (health as f32 / scale).min(self.status.max_health);
            self.last_health = self.status.health.ceil() as i32;
            self.display_health = self.last_health;
            return;
        };
        if health < synced {
            let mut lost = (synced - health) as f32 / scale;
            let absorbed = lost.min(self.status.absorption);
            self.status.absorption -= absorbed;
            lost -= absorbed;
            self.status.health = (self.status.health - lost).max(0.0);
            self.status.food.add_exhaustion(HURT_EXHAUSTION);
            self.synced = Some(health);
        }
    }

    pub(crate) fn tick(&mut self, motion: &Motion) -> Breath {
        self.tick += 1;
        self.status.tick_effects();
        if let Some(last) = self.last_feet {
            let d: [f64; 3] = std::array::from_fn(|k| motion.feet[k] - last[k]);
            let horizontal = d[0].hypot(d[2]);
            let full = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if full < MAX_STRIDE {
                if motion.in_water {
                    let swum = if motion.eye_in_water {
                        full
                    } else {
                        horizontal
                    };
                    self.status
                        .food
                        .add_exhaustion(SWIM_EXHAUSTION * swum as f32);
                } else if motion.on_ground && motion.sprinting {
                    self.status
                        .food
                        .add_exhaustion(SPRINT_EXHAUSTION * horizontal as f32);
                }
            }
        }
        self.last_feet = Some(motion.feet);
        if motion.jumping && !self.jumping {
            let cost = if motion.sprinting {
                SPRINT_JUMP_EXHAUSTION
            } else {
                JUMP_EXHAUSTION
            };
            self.status.food.add_exhaustion(cost);
        }
        self.jumping = motion.jumping;
        let breath = self.breathe(motion.eye_in_water);
        self.status
            .tick_with_rules(true, Difficulty::Normal, self.tick);
        self.update_blink();
        breath
    }

    pub(crate) fn breathe(&mut self, eye_in_water: bool) -> Breath {
        self.eye_in_water = eye_in_water;
        let drowned = self.status.tick_air(eye_in_water, false);
        let (air, max) = (
            self.status.air.clamp(0, self.status.max_air),
            self.status.max_air,
        );
        let bubbles = bubble_count(air - 2, max);
        let popped =
            (eye_in_water && bubbles < self.bubbles).then(|| 10 - bubble_count(air + 1, max));
        self.bubbles = bubbles;
        Breath { drowned, popped }
    }

    fn update_blink(&mut self) {
        let health = self.status.health.ceil() as i32;
        if health < self.last_health {
            self.blink_until = self.tick + DAMAGE_BLINK;
        }
        if health != self.last_health {
            self.changed_at = self.tick;
        }
        if self.tick.saturating_sub(self.changed_at) > SETTLE_DISPLAY {
            self.display_health = health;
        }
        self.last_health = health;
    }

    pub(crate) fn fall(&mut self, distance: f64, landed_on: Option<&str>) -> bool {
        let damage = landing_damage(distance, landed_on);
        damage > 0
            && self
                .status
                .hurt(damage as f32, Armor::default(), 0.0)
                .is_some()
    }

    pub(crate) fn hurt(&mut self, damage: f32, armor: Armor, from: Option<[f64; 3]>) -> bool {
        let landed = self.status.hurt(damage, armor, HURT_EXHAUSTION).is_some();
        if landed && from.is_some() {
            self.source = from;
        }
        landed
    }

    pub(crate) fn exhaust(&mut self, amount: f32) {
        self.status.food.add_exhaustion(amount);
    }

    pub(crate) fn can_sprint(&self) -> bool {
        self.status.food.level > SPRINT_FOOD
    }

    pub(crate) fn settle(&mut self, health: i32, max_health: i32) -> Settle {
        let base = self.synced.unwrap_or(health);
        let mut out = Settle::default();
        let mut current = health;
        if health > base {
            out.set = Some(base.max(1));
            current = base.max(1);
        }
        if self.status.health <= 0.0 {
            out.damage = Some((current.max(1), self.source.take()));
            return out;
        }
        let target =
            ((self.status.health * scale(max_health)).ceil() as i32).clamp(1, max_health.max(1));
        self.synced = Some(target);
        if target < current {
            out.damage = Some((current - target, self.source.take()));
        } else if target > current {
            out.set = Some(target);
        }
        out
    }

    pub(crate) fn eating(&self) -> Option<&FoodUse> {
        self.eating.as_ref()
    }

    pub(crate) fn start_eating(&mut self, slot: usize, stack: &ItemStack) -> bool {
        if self.eating.is_none() {
            self.eating = FoodUse::start(slot, stack, &self.status.food, GameMode::Survival);
        }
        self.eating.is_some()
    }

    pub(crate) fn start_using(&mut self, using: FoodUse) {
        if self.eating.is_none() {
            self.eating = Some(using);
        }
    }

    pub(crate) fn stop_eating(&mut self) {
        self.eating = None;
    }

    pub(crate) fn chew(
        &mut self,
        inventory: &mut Inventory,
        selected: usize,
        mut random: impl FnMut() -> f32,
    ) -> Bite {
        let Some(using) = self.eating.as_mut() else {
            return Bite::Nothing;
        };
        if using.slot != selected {
            self.eating = None;
            return Bite::Nothing;
        }
        let sound = format!("minecraft:{}", using.info.sound);
        match using.tick(inventory, &mut self.status.food, GameMode::Survival) {
            FoodUseTick::Continuing { emit_sound } => {
                if emit_sound {
                    Bite::Chew(sound)
                } else {
                    Bite::Nothing
                }
            }
            FoodUseTick::Finished { overflow } => {
                let id = using.stack.id.clone();
                self.eating = None;
                apply_consumed_food_effects(&id, &mut self.status, &mut random);
                let fed = minecraftoss_player::food::catalog().contains_key(&id);
                Bite::Done {
                    sound,
                    overflow,
                    fed,
                }
            }
            FoodUseTick::Cancelled => {
                self.eating = None;
                Bite::Nothing
            }
        }
    }

    pub(crate) fn vitals(&self) -> frame::McVitals {
        let regen = self.status.effect(EffectKind::Regeneration).is_some();
        let max = self.status.max_health;
        frame::McVitals {
            air: self.status.air,
            max_air: self.status.max_air,
            eye_in_water: self.eye_in_water,
            health: self.status.health,
            max_health: max,
            absorption: self.status.absorption,
            food: self.status.food.level,
            saturation: self.status.food.saturation,
            poisoned: self.status.effect(EffectKind::Poison).is_some(),
            hunger: self.status.effect(EffectKind::Hunger).is_some(),
            regen_index: if regen {
                (self.tick % (max + 5.0).ceil() as u64) as i32
            } else {
                -1
            },
            blink: self.blink_until > self.tick && (self.blink_until - self.tick) / 3 % 2 == 1,
            display_health: self.display_health,
            tick: self.tick,
        }
    }
}

pub(crate) fn attack_strength(ticks_since_attack: u32, attack_speed: f64) -> f32 {
    let delay = 20.0 / attack_speed.max(1.0e-4);
    ((f64::from(ticks_since_attack) + 0.5) / delay).clamp(0.0, 1.0) as f32
}

pub(crate) fn fall_damage(distance: f64, multiplier: f32) -> i32 {
    ((distance + 1.0e-6 - SAFE_FALL) * f64::from(multiplier))
        .floor()
        .max(0.0) as i32
}

pub(crate) fn landing_damage(distance: f64, block: Option<&str>) -> i32 {
    match block {
        Some("minecraft:slime_block") => 0,
        Some("minecraft:hay_block" | "minecraft:honey_block") => fall_damage(distance, 0.2),
        Some(soft) if soft.ends_with("_bed") || soft == "minecraft:shelf_mushroom" => {
            fall_damage(distance * 0.5, 1.0)
        }
        _ => fall_damage(distance, 1.0),
    }
}

pub(crate) fn armor(inventory: &Inventory) -> Armor {
    let recipes = &inventory.recipes;
    let (mut points, mut toughness) = (0.0, 0.0);
    for (slot, name) in [(36, "feet"), (37, "legs"), (38, "chest"), (39, "head")] {
        if let Some(stack) = inventory.slots.get(slot).and_then(Option::as_ref) {
            points += recipes.armor_points(stack, name);
            toughness += recipes.armor_toughness(stack, name).0;
        }
    }
    Armor {
        value: (points > 0.0).then_some((points as f32, toughness as f32)),
    }
}

pub(crate) fn bubble_count(air: i32, max_air: i32) -> i32 {
    ((air * 10) as f32 / max_air.max(1) as f32).ceil() as i32
}

pub(crate) fn pop_sound(empty: i32) -> (f32, f32) {
    let volume = 0.5 + 0.1 * (empty - 2).max(0) as f32;
    let pitch = 1.0 + 0.1 * (empty - 4).max(0) as f32;
    (volume, pitch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standing(feet: [f64; 3]) -> Motion {
        Motion {
            feet,
            on_ground: true,
            ..Motion::default()
        }
    }

    fn fresh() -> Survival {
        let mut survival = Survival::default();
        survival.observe(100, 100);
        survival
    }

    #[test]
    fn breath_runs_out_and_hurts_every_second_after_fifteen() {
        let mut survival = Survival::default();
        let mut drowned_at = Vec::new();
        for tick in 1..=320 {
            if survival.breathe(true).drowned {
                drowned_at.push(tick);
            }
        }
        assert_eq!(drowned_at, vec![320]);
        assert_eq!(survival.status.air, 0);
        for tick in 321..=340 {
            if survival.breathe(true).drowned {
                drowned_at.push(tick);
            }
        }
        assert_eq!(drowned_at, vec![320, 340]);
    }

    #[test]
    fn breath_comes_back_four_a_tick_above_water() {
        let mut survival = Survival::default();
        for _ in 0..100 {
            survival.breathe(true);
        }
        assert_eq!(survival.status.air, 200);
        survival.breathe(false);
        assert_eq!(survival.status.air, 204);
        for _ in 0..30 {
            survival.breathe(false);
        }
        assert_eq!(survival.status.air, 300);
    }

    #[test]
    fn bubbles_pop_one_by_one() {
        assert_eq!(bubble_count(298, 300), 10);
        assert_eq!(bubble_count(270, 300), 9);
        assert_eq!(bubble_count(0, 300), 0);
        assert_eq!(bubble_count(-2, 300), 0);
        let mut survival = Survival::default();
        let pops: Vec<i32> = (0..300)
            .filter_map(|_| survival.breathe(true).popped)
            .collect();
        assert_eq!(pops, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn later_pops_sound_louder_and_higher() {
        assert_eq!(pop_sound(0), (0.5, 1.0));
        let (volume, pitch) = pop_sound(9);
        assert!((volume - 1.2).abs() < 1e-6 && (pitch - 1.5).abs() < 1e-6);
    }

    #[test]
    fn mw2_bullets_take_hearts_away_after_absorption() {
        let mut survival = fresh();
        survival.status.absorption = 4.0;
        survival.observe(70, 100);
        assert_eq!(survival.status.absorption, 0.0);
        assert!((survival.status.health - 18.0).abs() < 1e-6);
        assert_eq!(
            survival.settle(70, 100),
            Settle {
                damage: None,
                set: Some(90)
            }
        );
    }

    #[test]
    fn minecraft_damage_reaches_mw2_health_five_to_a_heart_point() {
        let mut survival = fresh();
        assert!(survival.fall(10.0, Some("minecraft:stone")));
        assert!((survival.status.health - 13.0).abs() < 1e-6);
        assert_eq!(
            survival.settle(100, 100),
            Settle {
                damage: Some((35, None)),
                set: None
            }
        );
        assert_eq!(survival.settle(65, 100), Settle::default());
    }

    #[test]
    fn mw2_health_regeneration_is_undone() {
        let mut survival = fresh();
        survival.observe(60, 100);
        assert_eq!(survival.settle(60, 100), Settle::default());
        assert_eq!(survival.settle(75, 100).set, Some(60));
    }

    #[test]
    fn healing_raises_mw2_health() {
        let mut survival = fresh();
        survival.observe(50, 100);
        survival.status.heal(3.0);
        assert_eq!(
            survival.settle(50, 100),
            Settle {
                damage: None,
                set: Some(65)
            }
        );
    }

    #[test]
    fn a_lethal_hit_takes_what_is_left() {
        let mut survival = fresh();
        survival.observe(20, 100);
        survival.hurt(10.0, Armor::default(), Some([1.0, 2.0, 3.0]));
        assert_eq!(survival.status.health, 0.0);
        assert_eq!(
            survival.settle(20, 100).damage,
            Some((20, Some([1.0, 2.0, 3.0])))
        );
    }

    #[test]
    fn a_sword_recharges_in_twelve_and_a_half_ticks_and_a_fist_in_five() {
        assert!(attack_strength(0, 1.6) < 0.1);
        assert!(attack_strength(11, 1.6) < 1.0);
        assert_eq!(attack_strength(12, 1.6), 1.0);
        assert_eq!(attack_strength(5, 4.0), 1.0);
        assert!(attack_strength(4, 4.0) < 1.0);
    }

    #[test]
    fn falls_hurt_one_per_block_past_three() {
        assert_eq!(fall_damage(3.0, 1.0), 0);
        assert_eq!(fall_damage(3.9, 1.0), 0);
        assert_eq!(fall_damage(4.0, 1.0), 1);
        assert_eq!(fall_damage(23.5, 1.0), 20);
        assert_eq!(landing_damage(13.0, Some("minecraft:hay_block")), 2);
        assert_eq!(landing_damage(13.0, Some("minecraft:red_bed")), 3);
        assert_eq!(landing_damage(7.0, Some("minecraft:red_bed")), 0);
        assert_eq!(landing_damage(13.0, Some("minecraft:slime_block")), 0);
        assert_eq!(landing_damage(13.0, None), 10);
    }

    #[test]
    fn a_fed_player_heals_every_four_seconds() {
        let mut survival = fresh();
        survival.observe(50, 100);
        survival.status.food.saturation = 0.0;
        for _ in 0..79 {
            survival.tick(&standing([0.0; 3]));
        }
        assert!((survival.status.health - 10.0).abs() < 1e-6);
        survival.tick(&standing([0.0; 3]));
        assert!((survival.status.health - 11.0).abs() < 1e-6);
        assert!(survival.status.food.exhaustion > 5.9);
    }

    #[test]
    fn starving_hurts_down_to_half_a_heart() {
        let mut survival = fresh();
        survival.status.food.level = 0;
        survival.status.food.saturation = 0.0;
        survival.status.health = 3.0;
        for _ in 0..400 {
            survival.tick(&standing([0.0; 3]));
        }
        assert!((survival.status.health - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sprinting_and_jumping_tire_the_player() {
        let mut survival = fresh();
        let mut motion = Motion {
            sprinting: true,
            ..standing([0.0; 3])
        };
        survival.tick(&motion);
        motion.feet = [1.0, 0.0, 0.0];
        survival.tick(&motion);
        assert!((survival.status.food.exhaustion - 0.1).abs() < 1e-6);
        motion.jumping = true;
        survival.tick(&motion);
        survival.tick(&motion);
        assert!((survival.status.food.exhaustion - 0.3).abs() < 1e-6);
        motion.feet = [500.0, 0.0, 0.0];
        survival.tick(&motion);
        assert!((survival.status.food.exhaustion - 0.3).abs() < 1e-6);
    }

    #[test]
    fn swimming_tires_a_tenth_as_much() {
        let mut survival = fresh();
        let mut motion = Motion {
            in_water: true,
            eye_in_water: true,
            ..Motion::default()
        };
        survival.tick(&motion);
        motion.feet = [0.0, -1.0, 0.0];
        survival.tick(&motion);
        assert!((survival.status.food.exhaustion - 0.01).abs() < 1e-6);
    }

    #[test]
    fn hungry_players_cannot_sprint() {
        let mut survival = fresh();
        survival.status.food.level = 7;
        assert!(survival.can_sprint());
        survival.status.food.level = 6;
        assert!(!survival.can_sprint());
    }

    #[test]
    fn hunger_blocks_sprinting_until_fed_and_leaves_other_blocks_alone() {
        let mut survival = fresh();
        assert_eq!(survival.sprint_change(false), None);
        assert_eq!(survival.sprint_change(true), None);
        survival.status.food.level = 6;
        assert_eq!(survival.sprint_change(false), Some(true));
        assert_eq!(survival.sprint_change(true), None);
        survival.respawn();
        assert_eq!(survival.sprint_change(true), Some(false));
        assert_eq!(survival.sprint_change(false), None);
    }

    #[test]
    fn eating_takes_32_ticks_and_feeds() {
        let mut survival = fresh();
        survival.status.food.level = 10;
        let mut inventory = Inventory::default();
        inventory.slots[2] = Some(ItemStack::new("minecraft:bread", 3));
        let bread = inventory.slots[2].clone().expect("bread");
        assert!(survival.start_eating(2, &bread));
        let mut done = None;
        for tick in 1..=32 {
            if let Bite::Done { .. } = survival.chew(&mut inventory, 2, || 0.5) {
                done = Some(tick);
            }
        }
        assert_eq!(done, Some(32));
        assert_eq!(survival.status.food.level, 15);
        assert_eq!(inventory.slots[2].as_ref().map(|s| s.count), Some(2));
        assert!(survival.eating().is_none());
    }

    #[test]
    fn eating_stops_when_the_slot_changes_or_food_is_full() {
        let mut survival = fresh();
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:apple", 1));
        let apple = inventory.slots[0].clone().expect("apple");
        assert!(!survival.start_eating(0, &apple));
        survival.status.food.level = 5;
        assert!(survival.start_eating(0, &apple));
        assert_eq!(survival.chew(&mut inventory, 1, || 0.5), Bite::Nothing);
        assert!(survival.eating().is_none());
    }

    #[test]
    fn a_golden_apple_gives_absorption_hearts() {
        let mut survival = fresh();
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:golden_apple", 1));
        let apple = inventory.slots[0].clone().expect("apple");
        assert!(survival.start_eating(0, &apple));
        for _ in 0..32 {
            survival.chew(&mut inventory, 0, || 0.5);
        }
        assert_eq!(survival.status.absorption, 4.0);
        assert!(survival.vitals().regen_index >= 0);
    }

    #[test]
    fn hearts_blink_after_damage() {
        let mut survival = fresh();
        survival.tick(&standing([0.0; 3]));
        survival.hurt(4.0, Armor::default(), None);
        let blinks: Vec<bool> = (0..20)
            .map(|_| {
                survival.tick(&standing([0.0; 3]));
                survival.vitals().blink
            })
            .collect();
        assert!(blinks.iter().any(|b| *b) && blinks.iter().any(|b| !*b));
    }
}
