use crate::death::{CombatTracker, DamageCause, DeathLine, FallLocation};
use glam::DVec3;
use minecraftoss_entities::inside_blocks::{BlockEffect, Hazard, block_effects};
use minecraft_terrain::potions::PlayerPotion;
use minecraftoss_entities::movement::Body;
use minecraftoss_player::GameMode;
use minecraftoss_player::World;
use minecraftoss_player::collision::find_supporting_block;
use minecraftoss_player::food::{FoodUse, FoodUseTick, apply_consumed_food_effects};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::survival::{Armor, Difficulty, EffectKind, SurvivalStatus};

const HURT_EXHAUSTION: f32 = 0.1;
const BODY_WIDTH: f32 = 0.6;
const MW2_GROUND_REACH: f64 = 0.01;
const FIRE_IMMUNE_TICKS: i32 = 20;
const FIRE_IGNITE_TICKS: i32 = 160;
const LAVA_IGNITE_TICKS: i32 = 300;
const LAVA_DAMAGE: f32 = 4.0;
const OUT_OF_WORLD_DAMAGE: f32 = 4.0;
const OUT_OF_WORLD_DEPTH: i32 = 64;
const BURN_DAMAGE: f32 = 1.0;
const BURN_INTERVAL: i32 = 20;
pub const ATTACK_EXHAUSTION: f32 = 0.1;
pub const MINE_EXHAUSTION: f32 = 0.005;
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
pub struct Motion {
    pub feet: [f64; 3],
    pub on_ground: bool,
    pub sprinting: bool,
    pub jumping: bool,
    pub in_water: bool,
    pub eye_in_water: bool,
    pub in_lava: bool,
    pub eye_in_lava: bool,
    pub in_rain: bool,
    pub crouching: bool,
    pub height: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vitals {
    pub air: i32,
    pub max_air: i32,
    pub eye_in_water: bool,
    pub health: f32,
    pub max_health: f32,
    pub absorption: f32,
    pub food: u8,
    pub saturation: f32,
    pub poisoned: bool,
    pub withered: bool,
    pub hunger: bool,
    pub regen_index: i32,
    pub blink: bool,
    pub display_health: i32,
    pub tick: u64,
    pub experience_level: u32,
    pub experience_progress: f32,
    pub on_fire: bool,
    pub eye_in_lava: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurvivalRules {
    pub natural_regeneration: bool,
    pub fall_damage: bool,
    pub fire_damage: bool,
    pub drowning_damage: bool,
    pub difficulty: Difficulty,
}

impl Default for SurvivalRules {
    fn default() -> Self {
        Self {
            natural_regeneration: true,
            fall_damage: true,
            fire_damage: true,
            drowning_damage: true,
            difficulty: Difficulty::Normal,
        }
    }
}

impl SurvivalRules {
    pub fn of(rules: &minecraft_terrain::game_rules::GameRules, difficulty: Difficulty) -> Self {
        Self {
            natural_regeneration: rules.bool("natural_health_regeneration"),
            fall_damage: rules.bool("fall_damage"),
            fire_damage: rules.bool("fire_damage"),
            drowning_damage: rules.bool("drowning_damage"),
            difficulty,
        }
    }

    fn allows(&self, cause: DamageCause) -> bool {
        match cause {
            DamageCause::Fall | DamageCause::EnderPearl => self.fall_damage,
            DamageCause::Hazard(hazard) if hazard.is_fire() => self.fire_damage,
            DamageCause::Drown => self.drowning_damage,
            _ => true,
        }
    }
}

#[derive(Default)]
pub struct Survival {
    pub status: SurvivalStatus,
    pub score: i32,
    pub rules: SurvivalRules,
    killed: bool,
    pub eye_in_water: bool,
    eye_in_lava: bool,
    stuck: bool,
    fire_ticks: i32,
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
    level_up_at: Option<u64>,
    combat: CombatTracker,
    climbable: Option<FallLocation>,
    in_water: bool,
    invulnerable: bool,
    creative: bool,
    may_fly: bool,
    no_physics: bool,
    impulse: Option<f64>,
    impulse_grace: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Breath {
    pub drowned: bool,
    pub popped: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerSound {
    pub event: &'static str,
    pub volume: f32,
    pub pitch: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Bite {
    Nothing,
    Chew(String),
    Done {
        sound: String,
        overflow: Option<ItemStack>,
        fed: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Settle {
    pub damage: Option<(i32, Option<[f64; 3]>)>,
    pub set: Option<i32>,
}

fn scale(max_health: i32) -> f32 {
    max_health.max(1) as f32 / 20.0
}

impl Survival {
    pub fn respawn(&mut self) {
        *self = Self {
            blocked_sprint: self.blocked_sprint,
            rules: self.rules,
            invulnerable: self.invulnerable,
            creative: self.creative,
            may_fly: self.may_fly,
            no_physics: self.no_physics,
            ..Self::default()
        };
    }

    pub fn respawn_keeping_experience(&mut self) {
        let level = self.status.experience_level;
        let progress = self.status.experience_progress;
        let total = self.status.total_experience;
        let score = self.score;
        self.respawn();
        self.status.experience_level = level;
        self.status.experience_progress = progress;
        self.status.total_experience = total;
        self.score = score;
    }

    pub fn restore(&mut self, status: SurvivalStatus, fire_ticks: i32, score: i32) {
        self.status = status;
        self.fire_ticks = fire_ticks;
        self.score = score;
        self.synced = Some(0);
        self.last_health = status.health.ceil() as i32;
        self.display_health = self.last_health;
    }

    pub fn fire_ticks(&self) -> i32 {
        self.fire_ticks
    }

    pub fn set_game_mode(&mut self, mode: &minecraft_terrain::game_mode::PlayerMode) {
        self.invulnerable = mode.invulnerable();
        self.creative = mode.mode == GameMode::Creative;
        self.may_fly = mode.abilities.may_fly;
        self.no_physics = mode.noclip();
    }

    fn food_mode(&self) -> GameMode {
        if self.creative { GameMode::Creative } else { GameMode::Survival }
    }

    pub fn sprint_change(&mut self, blocked: bool) -> Option<bool> {
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

    pub fn observe(&mut self, health: i32, max_health: i32) {
        let scale = scale(max_health);
        let Some(synced) = self.synced else {
            self.synced = Some(health);
            self.status.health = (health as f32 / scale).min(self.status.max_health);
            self.last_health = self.status.health.ceil() as i32;
            self.display_health = self.last_health;
            return;
        };
        if health < synced && !self.invulnerable {
            let mut lost = (synced - health) as f32 / scale;
            let absorbed = lost.min(self.status.absorption);
            self.status.absorption -= absorbed;
            lost -= absorbed;
            self.status.health = (self.status.health - lost).max(0.0);
            self.status.food.add_exhaustion(HURT_EXHAUSTION);
            self.synced = Some(health);
        }
    }

    pub fn tick(&mut self, motion: &Motion) -> Breath {
        self.tick += 1;
        self.impulse_grace = self.impulse_grace.saturating_sub(1);
        if self.impulse_grace == 0 && (motion.on_ground || motion.in_water) {
            self.impulse = None;
        }
        self.in_water = motion.in_water;
        self.combat.tick(self.tick);
        let before = self.status.health;
        let withering = self.status.effect(EffectKind::Wither).is_some_and(|effect| {
            effect.duration % (40_u32 >> effect.amplifier.min(31)).max(1) == 0
        });
        self.status.tick_effects();
        if self.invulnerable {
            self.status.health = self.status.health.max(before);
        }
        let cause = if withering {
            DamageCause::Wither
        } else {
            DamageCause::Magic
        };
        self.record(before, cause, 0.0);
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
        let before = self.status.health;
        if self.invulnerable {
            self.status.food.exhaustion = 0.0;
        }
        self.status
            .tick_with_rules(self.rules.natural_regeneration, self.rules.difficulty, self.tick);
        if self.invulnerable {
            self.status.health = self.status.health.max(before);
        }
        self.record(before, DamageCause::Starve, 0.0);
        self.update_blink();
        breath
    }

    pub fn breathe(&mut self, eye_in_water: bool) -> Breath {
        self.eye_in_water = eye_in_water;
        let before = self.status.health;
        let drowned = if eye_in_water && !self.rules.drowning_damage && !self.invulnerable {
            self.status.air -= 1;
            if self.status.air <= -20 {
                self.status.air = 0;
            }
            false
        } else {
            self.status.tick_air(eye_in_water, self.invulnerable)
        };
        self.record(before, DamageCause::Drown, 0.0);
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

    pub fn fall(&mut self, distance: f64, landed_on: Option<&str>, crouching: bool) -> i32 {
        let damage = if self.rules.fall_damage && !self.invulnerable {
            landing_damage(distance, landed_on, crouching)
        } else {
            0
        };
        let before = self.status.health;
        if damage > 0
            && self
                .status
                .hurt(damage as f32, Armor::default(), 0.0)
                .is_some()
        {
            self.record(before, DamageCause::Fall, distance as f32);
            self.impulse = None;
            damage
        } else {
            0
        }
    }

    pub fn start_impulse(&mut self, feet_y: f64) {
        self.impulse = Some(feet_y);
        self.impulse_grace = self.impulse_grace.max(crate::impulse::FALL_GRACE_TICKS);
    }

    pub fn impulse_fall(&mut self, distance: f64, feet_y: f64) -> f64 {
        let Some(impact) = self.impulse else {
            return distance;
        };
        let counted = distance.min(impact - feet_y);
        if counted <= 0.0 || self.impulse_grace == 0 {
            self.impulse = None;
        }
        counted.max(0.0)
    }

    fn record(&mut self, before: f32, cause: DamageCause, fall_distance: f32) {
        let dealt = before - self.status.health;
        if dealt > 0.0 {
            let location = self
                .climbable
                .or(self.in_water.then_some(FallLocation::Water));
            self.combat
                .record(self.tick, cause, dealt, location, fall_distance);
        }
    }

    pub fn set_climbable(&mut self, climbable: Option<FallLocation>) {
        self.climbable = climbable;
    }

    pub fn death_line(
        &self,
        language: &std::collections::HashMap<String, String>,
        present: impl Fn(u64) -> bool,
    ) -> DeathLine {
        self.combat.death_line(language, present)
    }

    pub fn last_feet(&self) -> Option<[f64; 3]> {
        self.last_feet
    }

    pub fn stuck(&self) -> bool {
        self.stuck
    }

    pub fn hazards(
        &mut self,
        world: &impl World,
        motion: &Motion,
        from: [f64; 3],
        armor: Armor,
        mut random: impl FnMut() -> f32,
    ) -> Vec<PlayerSound> {
        let mut heard = Vec::new();
        self.eye_in_lava = motion.eye_in_lava;
        if self.fire_ticks > 0 {
            if self.fire_ticks % BURN_INTERVAL == 0 && !motion.in_lava {
                self.hurt_aloud(Hazard::OnFire, BURN_DAMAGE, armor, &mut random, &mut heard);
            }
            self.fire_ticks -= 1;
        }
        if motion.feet[1] < f64::from(world.min_y() - OUT_OF_WORLD_DEPTH) {
            self.fall_out_of_world(&mut random, &mut heard);
        }
        let stride = (0..3)
            .map(|k| (motion.feet[k] - from[k]).powi(2))
            .sum::<f64>()
            .sqrt();
        let from = if stride < MAX_STRIDE {
            from
        } else {
            motion.feet
        };
        let feet = DVec3::from_array(motion.feet);
        let mut body = Body::new(feet, BODY_WIDTH, motion.height);
        body.on_ground = motion.on_ground;
        if motion.on_ground {
            body.supporting_block = standing_on(world, feet);
        }
        let effects = if self.no_physics { Vec::new() } else { block_effects(world, &body, DVec3::from_array(from), &[]) };
        let before = self.fire_ticks;
        self.stuck = false;
        for effect in effects {
            if self.status.health <= 0.0 {
                break;
            }
            match effect {
                BlockEffect::Hurt(Hazard::HotFloor, _) if motion.crouching => {}
                BlockEffect::Hurt(hazard, damage) => {
                    self.hurt_aloud(hazard, damage, armor, &mut random, &mut heard);
                }
                BlockEffect::FireHurt(damage) => {
                    self.hurt_aloud(Hazard::InFire, damage, armor, &mut random, &mut heard);
                }
                BlockEffect::LavaHurt => {
                    if self.hurt_aloud(Hazard::Lava, LAVA_DAMAGE, armor, &mut random, &mut heard) {
                        let pitch = 2.0 + random() * 0.4;
                        heard.push(PlayerSound {
                            event: "minecraft:entity.generic.burn",
                            volume: 0.4,
                            pitch,
                        });
                    }
                }
                BlockEffect::FireIgnite => {
                    self.fire_ticks += if self.fire_ticks < 0 || random() < 0.5 {
                        1
                    } else {
                        2
                    };
                    if self.fire_ticks >= 0 {
                        self.fire_ticks = self.fire_ticks.max(FIRE_IGNITE_TICKS);
                    }
                }
                BlockEffect::LavaIgnite => self.fire_ticks = self.fire_ticks.max(LAVA_IGNITE_TICKS),
                BlockEffect::Extinguish => self.fire_ticks = self.fire_ticks.min(0),
                BlockEffect::Stuck(_) => self.stuck = true,
                BlockEffect::ClearFreeze => {}
            }
        }
        if self.invulnerable {
            self.fire_ticks = self.fire_ticks.min(1);
        }
        if motion.in_rain {
            self.fire_ticks = self.fire_ticks.min(0);
        }
        if before > 0 && self.fire_ticks <= 0 {
            let (a, b) = (random(), random());
            heard.push(PlayerSound {
                event: "minecraft:entity.generic.extinguish_fire",
                volume: 0.7,
                pitch: 1.6 + (a - b) * 0.4,
            });
        }
        if self.fire_ticks <= 0 && self.fire_ticks <= before {
            self.fire_ticks = -FIRE_IMMUNE_TICKS;
        }
        heard
    }

    fn hurt_aloud(
        &mut self,
        hazard: Hazard,
        damage: f32,
        armor: Armor,
        random: &mut impl FnMut() -> f32,
        heard: &mut Vec<PlayerSound>,
    ) -> bool {
        if self.invulnerable {
            return false;
        }
        if hazard.is_fire() && (self.status.effect(EffectKind::FireResistance).is_some() || !self.rules.fire_damage) {
            return false;
        }
        let armor = if hazard.bypasses_armor() {
            Armor::default()
        } else {
            armor
        };
        let exhaustion = if hazard == Hazard::OnFire {
            0.0
        } else {
            HURT_EXHAUSTION
        };
        let before = self.status.health;
        let landed = self.status.hurt(damage, armor, exhaustion);
        self.record(before, DamageCause::Hazard(hazard), 0.0);
        if landed == Some(true) && self.status.health > 0.0 {
            heard.push(PlayerSound {
                event: hurt_sound(hazard),
                volume: 1.0,
                pitch: (random() - random()) * 0.2 + 1.0,
            });
        }
        landed.is_some()
    }

    fn fall_out_of_world(
        &mut self,
        random: &mut impl FnMut() -> f32,
        heard: &mut Vec<PlayerSound>,
    ) {
        let before = self.status.health;
        let resistance = self.status.effect(EffectKind::Resistance);
        self.status.remove_effect(EffectKind::Resistance);
        let landed = self.status.hurt(OUT_OF_WORLD_DAMAGE, Armor::default(), 0.0);
        if let Some(effect) = resistance {
            self.status
                .add_effect(EffectKind::Resistance, effect.duration, effect.amplifier);
        }
        self.record(before, DamageCause::OutOfWorld, 0.0);
        if landed.is_some() && self.status.health <= 0.0 {
            self.killed = true;
        } else if landed == Some(true) {
            heard.push(PlayerSound {
                event: "minecraft:entity.player.hurt",
                volume: 1.0,
                pitch: (random() - random()) * 0.2 + 1.0,
            });
        }
    }

    pub fn hurt(
        &mut self,
        damage: f32,
        armor: Armor,
        from: Option<[f64; 3]>,
        cause: DamageCause,
    ) -> bool {
        if !self.rules.allows(cause) || (self.invulnerable && cause != DamageCause::Kill) {
            return false;
        }
        if cause.burns() && (self.status.effect(EffectKind::FireResistance).is_some() || !self.rules.fire_damage) {
            return false;
        }
        let before = self.status.health;
        let exhaustion = if cause == DamageCause::FlyIntoWall {
            0.0
        } else {
            HURT_EXHAUSTION
        };
        let landed = self.status.hurt(damage, armor, exhaustion).is_some();
        if landed {
            self.combat.hurt_by(self.tick, cause);
            self.record(before, cause, 0.0);
            if let Some((kind, ticks)) = cause.melee_effect() {
                self.status.add_effect(kind, ticks, 0);
            }
            if let Some(ticks) = cause.ignites() {
                self.fire_ticks = self.fire_ticks.max(ticks);
            }
        }
        if landed && from.is_some() {
            self.source = from;
        }
        landed
    }

    pub fn take_potion(
        &mut self,
        potion: &PlayerPotion,
        from: Option<[f64; 3]>,
        mut random: impl FnMut() -> f32,
    ) -> Option<PlayerSound> {
        match potion {
            PlayerPotion::Harm { amount, .. } => {
                self.hurt_magic(*amount, from);
                None
            }
            PlayerPotion::Extinguish => {
                let burning = self.fire_ticks > 0;
                self.fire_ticks = self.fire_ticks.min(0);
                burning.then(|| {
                    let (a, b) = (random(), random());
                    PlayerSound {
                        event: "minecraft:entity.generic.extinguish_fire",
                        volume: 0.7,
                        pitch: 1.6 + (a - b) * 0.4,
                    }
                })
            }
            other => {
                let _ = minecraft_terrain::potions::apply_to_player(&mut self.status, other);
                None
            }
        }
    }

    fn hurt_magic(&mut self, damage: f32, from: Option<[f64; 3]>) {
        if self.invulnerable {
            return;
        }
        let before = self.status.health;
        let landed = self.status.hurt(damage, Armor::default(), 0.0).is_some();
        self.record(before, DamageCause::Magic, 0.0);
        if landed && from.is_some() {
            self.source = from;
        }
    }

    pub fn kill(&mut self, cause: DamageCause) {
        let before = self.status.health;
        self.status.absorption = 0.0;
        self.status.health = 0.0;
        self.killed = true;
        self.record(before, cause, 0.0);
    }

    pub fn killed(&self) -> bool {
        self.killed
    }

    pub fn hurt_blocked(&mut self, cause: DamageCause) {
        if self.status.hurt(0.0, Armor::default(), 0.0).is_some() {
            self.combat.hurt_by(self.tick, cause);
        }
    }

    pub fn exhaust(&mut self, amount: f32) {
        self.status.food.add_exhaustion(amount);
    }

    pub fn drink_potion(&mut self, item: &str, components: Option<&serde_json::Value>) {
        use minecraft_terrain::potions::{PotionContents, drink_hits, duration_scale};
        let contents = PotionContents::from_components(components);
        for potion in drink_hits(&contents, duration_scale(item, components)) {
            self.take_potion(&potion, None, || 0.0);
        }
    }

    pub fn spend_levels(&mut self, levels: u32) {
        let status = &mut self.status;
        if status.experience_level < levels {
            status.experience_level = 0;
            status.experience_progress = 0.0;
            status.total_experience = 0;
        } else {
            status.experience_level -= levels;
        }
    }

    pub fn gain_experience(&mut self, points: u32) -> Option<f32> {
        if points == 0 {
            return None;
        }
        let before = self.status.experience_level;
        let gained = i32::try_from(points).unwrap_or(i32::MAX);
        self.score = self.score.saturating_add(gained);
        self.status.give_experience_points(points);
        let level = self.status.experience_level;
        if level > before && level.is_multiple_of(5) {
            self.level_up_chime()
        } else {
            None
        }
    }

    pub fn level_up_chime(&mut self) -> Option<f32> {
        let level = self.status.experience_level;
        let rested = self
            .level_up_at
            .is_none_or(|at| self.tick.saturating_sub(at) > 100);
        if !rested {
            return None;
        }
        self.level_up_at = Some(self.tick);
        Some(if level > 30 { 1.0 } else { level as f32 / 30.0 } * 0.75)
    }

    pub fn can_sprint(&self) -> bool {
        self.may_fly || self.status.food.level > SPRINT_FOOD
    }

    pub fn settle(&mut self, health: i32, max_health: i32) -> Settle {
        let base = self.synced.unwrap_or(health);
        let mut out = Settle::default();
        let mut current = health;
        if health > base {
            out.set = Some(base.max(1));
            current = base.max(1);
        }
        if self.killed || self.status.health <= 0.0 {
            self.status.health = 0.0;
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

    pub fn eating(&self) -> Option<&FoodUse> {
        self.eating.as_ref()
    }

    pub fn start_eating(&mut self, slot: usize, stack: &ItemStack) -> bool {
        if self.eating.is_none() {
            self.eating = FoodUse::start(slot, stack, &self.status.food, self.food_mode());
        }
        self.eating.is_some()
    }

    pub fn start_using(&mut self, using: FoodUse) {
        if self.eating.is_none() {
            self.eating = Some(using);
        }
    }

    pub fn stop_eating(&mut self) {
        self.eating = None;
    }

    pub fn chew(
        &mut self,
        inventory: &mut Inventory,
        selected: usize,
        mut random: impl FnMut() -> f32,
    ) -> Bite {
        let mode = self.food_mode();
        let Some(using) = self.eating.as_mut() else {
            return Bite::Nothing;
        };
        if using.slot != selected && using.slot != crate::offhand::OFFHAND {
            self.eating = None;
            return Bite::Nothing;
        }
        let sound = format!("minecraft:{}", using.info.sound);
        match using.tick(inventory, &mut self.status.food, mode) {
            FoodUseTick::Continuing { emit_sound } => {
                if emit_sound {
                    Bite::Chew(sound)
                } else {
                    Bite::Nothing
                }
            }
            FoodUseTick::Finished { overflow } => {
                let id = using.stack.id.clone();
                let components = using.stack.components.clone();
                self.eating = None;
                apply_consumed_food_effects(&id, &mut self.status, &mut random);
                self.drink_potion(&id, components.as_ref());
                if id == minecraft_terrain::bottle::MILK_BUCKET {
                    self.clear_effects();
                }
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

    pub fn protect(&mut self, inventory: &mut Inventory, selected: usize) -> bool {
        if self.killed {
            return false;
        }
        let Some(slot) = crate::offhand::totem_slot(inventory, selected) else {
            return false;
        };
        if let Some(totem) = inventory.slots[slot].as_mut() {
            totem.count -= 1;
            if totem.count == 0 {
                inventory.slots[slot] = None;
            }
        }
        self.status.health = 1.0;
        self.clear_effects();
        self.status.add_effect(EffectKind::Regeneration, 900, 1);
        self.status.add_effect(EffectKind::Absorption, 100, 1);
        self.status.add_effect(EffectKind::FireResistance, 800, 0);
        true
    }

    pub fn clear_effects(&mut self) {
        for kind in [
            EffectKind::Hunger,
            EffectKind::Poison,
            EffectKind::Regeneration,
            EffectKind::Absorption,
            EffectKind::Resistance,
            EffectKind::FireResistance,
            EffectKind::Nausea,
            EffectKind::Slowness,
            EffectKind::Weakness,
            EffectKind::Wither,
        ] {
            self.status.remove_effect(kind);
        }
    }

    pub fn vitals(&self) -> Vitals {
        let regen = self.status.effect(EffectKind::Regeneration).is_some();
        let max = self.status.max_health;
        Vitals {
            air: self.status.air,
            max_air: self.status.max_air,
            eye_in_water: self.eye_in_water,
            health: self.status.health,
            max_health: max,
            absorption: self.status.absorption,
            food: self.status.food.level,
            saturation: self.status.food.saturation,
            poisoned: self.status.effect(EffectKind::Poison).is_some(),
            withered: self.status.effect(EffectKind::Wither).is_some(),
            hunger: self.status.effect(EffectKind::Hunger).is_some(),
            regen_index: if regen {
                (self.tick % (max + 5.0).ceil() as u64) as i32
            } else {
                -1
            },
            blink: self.blink_until > self.tick && (self.blink_until - self.tick) / 3 % 2 == 1,
            display_health: self.display_health,
            tick: self.tick,
            experience_level: self.status.experience_level,
            experience_progress: self.status.experience_progress,
            on_fire: self.fire_ticks > 0,
            eye_in_lava: self.eye_in_lava,
        }
    }
}

fn standing_on(world: &impl World, feet: DVec3) -> Option<(i32, i32, i32)> {
    let half = f64::from(BODY_WIDTH) / 2.0;
    find_supporting_block(
        world,
        feet,
        DVec3::new(feet.x - half, feet.y - MW2_GROUND_REACH, feet.z - half),
        DVec3::new(feet.x + half, feet.y, feet.z + half),
    )
}

fn hurt_sound(hazard: Hazard) -> &'static str {
    if hazard.is_fire() {
        "minecraft:entity.player.hurt_on_fire"
    } else if hazard == Hazard::SweetBerryBush {
        "minecraft:entity.player.hurt_sweet_berry_bush"
    } else {
        "minecraft:entity.player.hurt"
    }
}

pub fn attack_strength(ticks_since_attack: u32, attack_speed: f64) -> f32 {
    let delay = 20.0 / attack_speed.max(1.0e-4);
    ((f64::from(ticks_since_attack) + 0.5) / delay).clamp(0.0, 1.0) as f32
}

pub fn fall_damage(distance: f64, multiplier: f32) -> i32 {
    ((distance + 1.0e-6 - SAFE_FALL) * f64::from(multiplier))
        .floor()
        .max(0.0) as i32
}

pub fn breaks_fall(block: &str) -> bool {
    matches!(
        block,
        "minecraft:ladder"
            | "minecraft:vine"
            | "minecraft:scaffolding"
            | "minecraft:cobweb"
            | "minecraft:powder_snow"
            | "minecraft:sweet_berry_bush"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
            | "minecraft:water"
            | "minecraft:bubble_column"
    )
}

pub fn fall_sound(damage: i32) -> &'static str {
    if damage > 4 {
        "minecraft:entity.player.big_fall"
    } else {
        "minecraft:entity.player.small_fall"
    }
}

pub fn landing_damage(distance: f64, block: Option<&str>, crouching: bool) -> i32 {
    match block {
        Some("minecraft:slime_block") if !crouching => 0,
        Some(soft) if breaks_fall(soft) => 0,
        Some("minecraft:hay_block" | "minecraft:honey_block") => fall_damage(distance, 0.2),
        Some(soft) if soft.ends_with("_bed") || soft == "minecraft:shelf_mushroom" => {
            fall_damage(distance * 0.5, 1.0)
        }
        _ => fall_damage(distance, 1.0),
    }
}

pub fn armor(inventory: &Inventory) -> Armor {
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

pub fn bubble_count(air: i32, max_air: i32) -> i32 {
    ((air * 10) as f32 / max_air.max(1) as f32).ceil() as i32
}

pub fn pop_sound(empty: i32) -> (f32, f32) {
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
    fn a_wither_skeleton_s_blow_withers_and_wither_hurts_every_two_seconds_to_the_end() {
        let mut survival = fresh();
        let blow = DamageCause::mob(
            crate::death::MobAttack::Melee,
            Some(4),
            Some("minecraft:wither_skeleton"),
        );
        assert!(survival.hurt(8.0, Armor::default(), None, blow));
        assert_eq!(
            survival.status.effect(EffectKind::Wither).map(|e| e.duration),
            Some(200)
        );
        assert!(survival.vitals().withered);
        let mut status = SurvivalStatus::default();
        status.health = 3.0;
        status.add_effect(EffectKind::Wither, 200, 0);
        for _ in 0..120 {
            status.tick_effects();
        }
        assert_eq!(status.health, 0.0, "unlike poison, wither kills");
        let mut fast = SurvivalStatus::default();
        fast.add_effect(EffectKind::Wither, 40, 1);
        for _ in 0..40 {
            fast.tick_effects();
        }
        assert_eq!(fast.health, 18.0, "every 20 ticks at level II");
    }

    #[test]
    fn fire_resistance_and_the_fire_damage_rule_keep_fireballs_off() {
        let fireball = DamageCause::mob(crate::death::MobAttack::Fireball, Some(4), Some("minecraft:ghast"));
        let mut survival = fresh();
        assert!(survival.hurt(6.0, Armor::default(), None, fireball));
        assert_eq!(survival.status.health, 14.0);
        let mut resistant = fresh();
        resistant.status.add_effect(EffectKind::FireResistance, 600, 0);
        assert!(!resistant.hurt(6.0, Armor::default(), None, fireball));
        assert_eq!(resistant.status.health, 20.0);
        let mut ruled = fresh();
        ruled.rules.fire_damage = false;
        assert!(!ruled.hurt(6.0, Armor::default(), None, fireball));
        let blast = DamageCause::mob(crate::death::MobAttack::Explosion, Some(4), Some("minecraft:ghast"));
        assert!(resistant.hurt(4.0, Armor::default(), None, blast), "its blast is no fire");
    }

    #[test]
    fn a_blaze_fireball_sets_the_player_alight_only_when_it_hurts() {
        let small = DamageCause::mob(crate::death::MobAttack::SmallFireball, Some(3), Some("minecraft:blaze"));
        let mut survival = fresh();
        assert!(survival.hurt(5.0, Armor::default(), None, small));
        assert_eq!((survival.status.health, survival.fire_ticks()), (15.0, 100));
        let mut resistant = fresh();
        resistant.status.add_effect(EffectKind::FireResistance, 600, 0);
        assert!(!resistant.hurt(5.0, Armor::default(), None, small));
        assert_eq!(resistant.fire_ticks(), 0);
        let large = DamageCause::mob(crate::death::MobAttack::Fireball, Some(4), Some("minecraft:ghast"));
        let mut ghast_hit = fresh();
        assert!(ghast_hit.hurt(6.0, Armor::default(), None, large));
        assert_eq!(ghast_hit.fire_ticks(), 0, "a ghast's fireball does not set the player alight");
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
        assert_eq!(survival.fall(10.0, Some("minecraft:stone"), false), 7);
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
    fn a_self_frag_lowers_hearts_even_with_minecraft_damage_queued_in_the_same_tick() {
        let mut survival = fresh();
        assert_eq!(survival.fall(10.0, Some("minecraft:stone"), false), 7);
        assert_eq!(survival.settle(100, 100).damage, Some((35, None)));
        survival.observe(100 - 35 - 61, 100);
        assert!((survival.status.health - (13.0 - 61.0 / 5.0)).abs() < 1e-5);
        assert_eq!(survival.settle(4, 100).damage, None);
    }

    #[test]
    fn own_grenades_cost_the_same_hearts_after_every_respawn() {
        let mut survival = Survival::default();
        for life in 0..4 {
            survival.respawn();
            survival.observe(100, 100);
            assert_eq!(survival.fall(10.0, Some("minecraft:stone"), false), 7);
            assert_eq!(survival.settle(100, 100).damage, Some((35, None)));
            survival.observe(65, 100);
            assert_eq!(survival.settle(65, 100), Settle::default(), "life {life}");
            survival.observe(65 - 61, 100);
            assert!(
                (survival.status.health - (13.0 - 61.0 / 5.0)).abs() < 1e-5,
                "life {life}"
            );
            assert_eq!(survival.settle(4, 100).damage, None, "life {life}");
            survival.observe(0, 100);
            assert!(survival.status.health < 1e-5, "life {life}");
        }
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
        survival.hurt(
            10.0,
            Armor::default(),
            Some([1.0, 2.0, 3.0]),
            DamageCause::Generic,
        );
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
    fn a_wind_charge_jump_only_counts_the_fall_below_where_it_burst() {
        let mut survival = fresh();
        survival.start_impulse(64.0);
        assert_eq!(survival.impulse_fall(9.0, 64.0), 0.0);
        assert_eq!(survival.impulse_fall(9.0, 64.0), 9.0);
        survival.start_impulse(64.0);
        assert_eq!(survival.impulse_fall(12.0, 60.0), 4.0);
        assert_eq!(survival.fall(4.0, None, false), 1);
        assert_eq!(survival.impulse_fall(12.0, 60.0), 12.0);
    }

    #[test]
    fn a_short_drop_after_a_wind_charge_keeps_the_grace_for_two_seconds() {
        let mut survival = fresh();
        survival.start_impulse(70.0);
        assert_eq!(survival.impulse_fall(5.0, 68.0), 2.0);
        assert_eq!(survival.fall(2.0, None, false), 0);
        assert_eq!(survival.impulse_fall(10.0, 67.0), 3.0);
        for _ in 0..40 {
            survival.tick(&Motion {
                feet: [0.5, 67.0, 0.5],
                ..Motion::default()
            });
        }
        assert_eq!(survival.impulse_fall(10.0, 66.0), 4.0);
        assert_eq!(survival.impulse_fall(10.0, 66.0), 10.0);
    }

    #[test]
    fn a_wind_charge_grace_ends_on_the_ground_or_in_water_once_two_seconds_are_up() {
        let mut survival = fresh();
        survival.start_impulse(64.0);
        assert_eq!(survival.impulse_fall(1.0, 63.0), 1.0);
        for _ in 0..39 {
            survival.tick(&standing([0.5, 63.0, 0.5]));
        }
        assert_eq!(survival.impulse_fall(3.0, 60.0), 3.0);
        survival.tick(&standing([0.5, 60.0, 0.5]));
        assert_eq!(survival.impulse_fall(10.0, 70.0), 10.0);
        survival.start_impulse(64.0);
        for _ in 0..40 {
            survival.tick(&Motion {
                feet: [0.5, 50.0, 0.5],
                in_water: true,
                ..Motion::default()
            });
        }
        assert_eq!(survival.impulse_fall(10.0, 70.0), 10.0);
    }

    #[test]
    fn falls_hurt_one_per_block_past_three() {
        assert_eq!(fall_damage(3.0, 1.0), 0);
        assert_eq!(fall_damage(3.9, 1.0), 0);
        assert_eq!(fall_damage(4.0, 1.0), 1);
        assert_eq!(fall_damage(23.5, 1.0), 20);
        assert_eq!(landing_damage(13.0, Some("minecraft:hay_block"), false), 2);
        assert_eq!(
            landing_damage(13.0, Some("minecraft:honey_block"), false),
            2
        );
        assert_eq!(landing_damage(13.0, Some("minecraft:red_bed"), false), 3);
        assert_eq!(landing_damage(7.0, Some("minecraft:red_bed"), false), 0);
        assert_eq!(
            landing_damage(13.0, Some("minecraft:slime_block"), false),
            0
        );
        assert_eq!(
            landing_damage(13.0, Some("minecraft:slime_block"), true),
            10
        );
        assert_eq!(landing_damage(13.0, None, false), 10);
    }

    #[test]
    fn sprinting_spends_saturation_and_then_food() {
        let mut runner = fresh();
        for i in 0..1000 {
            runner.tick(&Motion {
                feet: [f64::from(i) * 0.25, 0.0, 0.0],
                on_ground: true,
                sprinting: true,
                ..Motion::default()
            });
        }
        assert_eq!(runner.status.food.saturation, 0.0);
        assert_eq!(runner.status.food.level, 19);
        let mut walker = fresh();
        for i in 0..1000 {
            walker.tick(&standing([f64::from(i) * 0.25, 0.0, 0.0]));
        }
        assert_eq!(walker.status.food.saturation, 5.0);
        assert_eq!(walker.status.food.exhaustion, 0.0);
    }

    #[test]
    fn jumps_swims_hits_and_wounds_all_cost_food() {
        let mut jumper = fresh();
        jumper.status.food.saturation = 0.0;
        for i in 0..126 {
            jumper.tick(&Motion {
                on_ground: true,
                sprinting: true,
                jumping: i % 2 == 0,
                ..Motion::default()
            });
        }
        assert_eq!(jumper.status.food.level, 17);
        let mut hopper = fresh();
        for i in 0..10 {
            hopper.tick(&Motion {
                on_ground: true,
                jumping: i % 2 == 0,
                ..Motion::default()
            });
        }
        assert!((hopper.status.food.exhaustion - 0.25).abs() < 1e-5);
        let mut swimmer = fresh();
        for i in 0..100 {
            swimmer.tick(&Motion {
                feet: [f64::from(i) * 0.2, 0.0, 0.0],
                in_water: true,
                eye_in_water: true,
                ..Motion::default()
            });
        }
        assert!((swimmer.status.food.exhaustion - 0.198).abs() < 1e-4);
        let mut fighter = fresh();
        fighter.exhaust(ATTACK_EXHAUSTION);
        fighter.exhaust(MINE_EXHAUSTION);
        fighter.hurt(2.0, Armor::default(), None, DamageCause::Generic);
        assert!((fighter.status.food.exhaustion - 0.205).abs() < 1e-6);
        fighter.observe(80, 100);
        assert!((fighter.status.food.exhaustion - 0.305).abs() < 1e-6);
    }

    fn drink(survival: &mut Survival, potion: &str) {
        let components = serde_json::json!({"minecraft:potion_contents": {"potion": potion}});
        survival.drink_potion("minecraft:potion", Some(&components));
    }

    #[test]
    fn potions_apply_their_vanilla_effects() {
        let mut survival = fresh();
        survival.status.health = 10.0;
        drink(&mut survival, "minecraft:strong_healing");
        assert!((survival.status.health - 18.0).abs() < 1e-6);
        drink(&mut survival, "minecraft:harming");
        assert!((survival.status.health - 12.0).abs() < 1e-6);
        drink(&mut survival, "minecraft:long_poison");
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Poison)
                .map(|e| (e.duration, e.amplifier)),
            Some((1800, 0))
        );
        drink(&mut survival, "minecraft:strong_turtle_master");
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Slowness)
                .map(|e| (e.duration, e.amplifier)),
            Some((400, 5))
        );
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Resistance)
                .map(|e| e.amplifier),
            Some(3)
        );
        drink(&mut survival, "minecraft:long_fire_resistance");
        assert_eq!(
            survival
                .status
                .effect(EffectKind::FireResistance)
                .map(|e| e.duration),
            Some(9600)
        );
        drink(&mut survival, "minecraft:water");
        drink(&mut survival, "minecraft:swiftness");
    }

    #[test]
    fn a_drunk_potion_gives_its_custom_effects_and_every_strong_and_long_form() {
        let mut survival = fresh();
        let components = serde_json::json!({"minecraft:potion_contents": {
            "potion": "minecraft:strong_poison",
            "custom_effects": [{"id": "minecraft:regeneration", "amplifier": 1, "duration": 100}]
        }});
        survival.drink_potion("minecraft:potion", Some(&components));
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Poison)
                .map(|e| (e.duration, e.amplifier)),
            Some((432, 1))
        );
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Regeneration)
                .map(|e| (e.duration, e.amplifier)),
            Some((100, 1))
        );
        drink(&mut survival, "minecraft:long_weakness");
        assert_eq!(
            survival.status.effect(EffectKind::Weakness).map(|e| e.duration),
            Some(4800)
        );
        drink(&mut survival, "minecraft:long_turtle_master");
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Resistance)
                .map(|e| (e.duration, e.amplifier)),
            Some((800, 2))
        );
    }

    #[test]
    fn a_splash_gives_its_effect_heals_and_harms_past_armour() {
        use minecraft_terrain::potions::PotionEffect;
        let mut survival = fresh();
        survival.status.health = 10.0;
        let poison = PlayerPotion::Effect(PotionEffect::new("minecraft:poison", 450, 1));
        assert_eq!(survival.take_potion(&poison, None, || 0.5), None);
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Poison)
                .map(|e| (e.duration, e.amplifier)),
            Some((450, 1))
        );
        survival.take_potion(&PlayerPotion::Heal(4.0), None, || 0.5);
        assert!((survival.status.health - 14.0).abs() < 1e-6);
        let exhaustion = survival.status.food.exhaustion;
        let harm = PlayerPotion::Harm {
            amount: 6.0,
            by: Some(0),
        };
        survival.take_potion(&harm, Some([1.0, 64.0, 2.0]), || 0.5);
        assert!((survival.status.health - 8.0).abs() < 1e-6);
        assert_eq!(survival.status.food.exhaustion, exhaustion);
        assert_eq!(survival.source, Some([1.0, 64.0, 2.0]));
        assert_eq!(
            survival.combat.death_line(&std::collections::HashMap::new(), |_| false).message,
            "death.attack.magic"
        );
    }

    #[test]
    fn a_creative_player_takes_splash_effects_but_no_harm() {
        use minecraft_terrain::potions::PotionEffect;
        let mut survival = fresh();
        survival.set_game_mode(&minecraft_terrain::game_mode::PlayerMode::new(GameMode::Creative));
        let harm = PlayerPotion::Harm {
            amount: 12.0,
            by: None,
        };
        survival.take_potion(&harm, None, || 0.5);
        assert_eq!(survival.status.health, 20.0);
        let slow = PlayerPotion::Effect(PotionEffect::new("minecraft:slowness", 100, 0));
        survival.take_potion(&slow, None, || 0.5);
        assert!(survival.status.effect(EffectKind::Slowness).is_some());
    }

    #[test]
    fn splash_water_puts_out_a_burning_player_with_the_hiss() {
        let mut survival = fresh();
        assert_eq!(
            survival.take_potion(&PlayerPotion::Extinguish, None, || 0.5),
            None
        );
        survival.fire_ticks = 120;
        assert!(survival.vitals().on_fire);
        let sound = survival.take_potion(&PlayerPotion::Extinguish, None, || 0.5);
        assert_eq!(
            sound,
            Some(PlayerSound {
                event: "minecraft:entity.generic.extinguish_fire",
                volume: 0.7,
                pitch: 1.6
            })
        );
        assert!(!survival.vitals().on_fire);
        survival.fire_ticks = -20;
        survival.take_potion(&PlayerPotion::Extinguish, None, || 0.5);
        assert_eq!(survival.fire_ticks, -20);
    }

    #[test]
    fn experience_levels_up_and_chimes_every_fifth_level() {
        let mut survival = fresh();
        assert_eq!(survival.gain_experience(7), None);
        assert_eq!(survival.status.experience_level, 1);
        let chime = survival.gain_experience(48).expect("level five chimes");
        assert_eq!(survival.status.experience_level, 5);
        assert!((chime - 0.125).abs() < 1e-6);
        assert_eq!(
            survival.gain_experience(105),
            None,
            "not twice within five seconds"
        );
        assert_eq!(survival.status.experience_level, 10);
        assert_eq!(survival.vitals().experience_level, 10);
        survival.spend_levels(3);
        assert_eq!(survival.status.experience_level, 7);
        survival.spend_levels(30);
        assert_eq!(survival.status.experience_level, 0);
        assert_eq!(survival.status.total_experience, 0);
    }

    #[test]
    fn an_eight_block_drop_onto_grass_costs_five() {
        let grass = Some("minecraft:grass_block");
        assert_eq!(landing_damage(8.0, grass, false), 5);
        for soft in [
            "minecraft:water",
            "minecraft:scaffolding",
            "minecraft:ladder",
            "minecraft:vine",
            "minecraft:cobweb",
            "minecraft:powder_snow",
        ] {
            assert_eq!(landing_damage(8.0, Some(soft), false), 0, "{soft}");
        }
        let mut survival = fresh();
        assert_eq!(survival.fall(8.0, grass, false), 5);
        assert!((survival.status.health - 15.0).abs() < 1e-6);
        assert_eq!(survival.settle(100, 100).damage.map(|d| d.0), Some(25));
        assert_eq!(fall_sound(5), "minecraft:entity.player.big_fall");
        assert_eq!(fall_sound(4), "minecraft:entity.player.small_fall");
        assert!(!breaks_fall("minecraft:grass_block"));
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
        survival.set_game_mode(&minecraft_terrain::game_mode::PlayerMode::new(GameMode::Creative));
        assert!(survival.can_sprint());
        survival.set_game_mode(&minecraft_terrain::game_mode::PlayerMode::new(GameMode::Adventure));
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
    fn respawn_keeping_experience_keeps_levels_and_resets_food() {
        let mut survival = fresh();
        survival.status.experience_level = 12;
        survival.status.experience_progress = 0.5;
        survival.status.total_experience = 300;
        survival.status.food.level = 3;
        survival.status.food.saturation = 0.0;
        survival.status.health = 4.0;
        survival.respawn_keeping_experience();
        assert_eq!(survival.status.experience_level, 12);
        assert_eq!(survival.status.experience_progress, 0.5);
        assert_eq!(survival.status.total_experience, 300);
        assert_eq!(survival.status.food.level, 20);
        assert_eq!(survival.status.food.saturation, 5.0);
        assert_eq!(survival.status.health, 20.0);
        let mut plain = fresh();
        plain.status.experience_level = 12;
        plain.status.total_experience = 300;
        plain.respawn();
        assert_eq!(plain.status.experience_level, 0);
        assert_eq!(plain.status.total_experience, 0);
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
    fn milk_clears_every_effect_when_finished_without_a_burp() {
        let mut survival = fresh();
        survival.status.add_effect(EffectKind::Poison, 400, 1);
        survival.status.add_effect(EffectKind::Absorption, 2400, 0);
        let mut inventory = Inventory::default();
        inventory.slots[40] = Some(ItemStack::new(minecraft_terrain::bottle::MILK_BUCKET, 1));
        let milk = inventory.slots[40].clone().expect("milk");
        survival.start_using(minecraft_terrain::bottle::drink_milk(40, &milk));
        let using = survival.eating().expect("drinking");
        assert!(using.info.animation.eq_ignore_ascii_case("drink"));
        assert!(!using.info.particles);
        for _ in 1..32 {
            assert!(!matches!(survival.chew(&mut inventory, 4, || 0.5), Bite::Done { .. }));
        }
        assert!(survival.status.effect(EffectKind::Poison).is_some());
        let done = survival.chew(&mut inventory, 4, || 0.5);
        assert_eq!(done, Bite::Done { sound: "minecraft:entity.generic.drink".to_owned(), overflow: None, fed: false });
        assert_eq!(survival.status.effect(EffectKind::Poison), None);
        assert_eq!(survival.status.absorption, 0.0);
        assert_eq!(inventory.slots[40].as_ref().map(|s| s.id.as_str()), Some("minecraft:bucket"));
    }

    #[test]
    fn eating_and_drinking_report_their_animation_and_crumbs() {
        let mut survival = fresh();
        survival.status.food.level = 4;
        let bread = ItemStack::new("minecraft:bread", 1);
        assert!(survival.start_eating(0, &bread));
        let using = survival.eating().expect("eating");
        assert!(using.info.animation.eq_ignore_ascii_case("eat"));
        assert!(using.info.particles);
        survival.stop_eating();
        let honey = ItemStack::new("minecraft:honey_bottle", 1);
        assert!(survival.start_eating(0, &honey));
        let using = survival.eating().expect("drinking");
        assert!(using.info.animation.eq_ignore_ascii_case("drink"));
        assert!(!using.info.particles);
        assert_eq!(using.info.consume_ticks, 40);
        survival.stop_eating();
        let kelp = ItemStack::new("minecraft:dried_kelp", 1);
        assert!(survival.start_eating(0, &kelp));
        assert_eq!(survival.eating().expect("eating").info.consume_ticks, 16);
    }

    #[test]
    fn offhand_food_keeps_eating_when_the_hotbar_selection_changes() {
        let mut survival = fresh();
        survival.status.food.level = 10;
        let mut inventory = Inventory::default();
        inventory.slots[40] = Some(ItemStack::new("minecraft:bread", 2));
        let bread = inventory.slots[40].clone().expect("bread");
        assert!(survival.start_eating(40, &bread));
        let mut done = false;
        for tick in 0..32 {
            done |= matches!(survival.chew(&mut inventory, tick % 9, || 0.5), Bite::Done { .. });
        }
        assert!(done);
        assert_eq!(inventory.slots[40].as_ref().map(|s| s.count), Some(1));
    }

    #[test]
    fn a_totem_keeps_one_health_and_gives_its_effects() {
        let mut survival = fresh();
        let mut inventory = Inventory::default();
        inventory.slots[3] = Some(ItemStack::new("minecraft:totem_of_undying", 1));
        inventory.slots[40] = Some(ItemStack::new("minecraft:totem_of_undying", 1));
        survival.status.add_effect(EffectKind::Poison, 400, 1);
        survival.status.add_effect(EffectKind::Weakness, 400, 0);
        survival.status.health = 0.0;
        assert!(survival.protect(&mut inventory, 3));
        assert_eq!(survival.status.health, 1.0);
        assert!(inventory.slots[3].is_none());
        assert!(inventory.slots[40].is_some());
        assert!(survival.status.effect(EffectKind::Poison).is_none());
        assert!(survival.status.effect(EffectKind::Weakness).is_none());
        let effect = |kind| survival.status.effect(kind).map(|e| (e.duration, e.amplifier));
        assert_eq!(effect(EffectKind::Regeneration), Some((900, 1)));
        assert_eq!(effect(EffectKind::Absorption), Some((100, 1)));
        assert_eq!(effect(EffectKind::FireResistance), Some((800, 0)));
        assert_eq!(survival.status.absorption, 8.0);
        assert!(survival.protect(&mut inventory, 3));
        assert!(inventory.slots[40].is_none());
        assert!(!survival.protect(&mut inventory, 3));
    }

    #[test]
    fn generic_kill_has_the_vanilla_message_and_bypasses_the_totem() {
        let mut survival = fresh();
        let mut inventory = Inventory::default();
        inventory.slots[40] = Some(ItemStack::new("minecraft:totem_of_undying", 1));
        survival.status.absorption = 4.0;
        survival.status.add_effect(EffectKind::Resistance, 400, 4);
        survival.kill(DamageCause::Kill);
        assert_eq!(survival.status.health, 0.0);
        assert_eq!(survival.status.absorption, 0.0);
        assert!(survival.killed());
        assert!(!survival.protect(&mut inventory, 0));
        assert!(inventory.slots[40].is_some());
        let language: std::collections::HashMap<String, String> = [("death.attack.genericKill", "%1$s was killed")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        assert_eq!(survival.death_line(&language, |_| false).message, "%1$s was killed");
        assert_eq!(survival.settle(100, 100).damage, Some((100, None)));
        survival.respawn();
        assert!(!survival.killed());
        survival.status.health = 0.0;
        assert!(survival.protect(&mut inventory, 0));
    }

    #[test]
    fn a_kill_sticks_through_healing_until_it_is_settled() {
        let mut survival = fresh();
        survival.status.health = 12.0;
        survival.status.food.level = 20;
        survival.status.food.saturation = 5.0;
        survival.status.add_effect(EffectKind::Regeneration, 400, 1);
        survival.kill(DamageCause::Kill);
        for _ in 0..40 {
            survival.tick(&standing([0.0, 64.0, 0.0]));
        }
        assert!(survival.status.health > 0.0);
        assert_eq!(survival.settle(100, 100).damage.map(|d| d.0), Some(100));
        assert_eq!(survival.status.health, 0.0);
        assert!(survival.killed());
    }

    #[test]
    fn game_rules_turn_fall_fire_drowning_and_regeneration_off() {
        let mut rules = minecraft_terrain::game_rules::GameRules::default();
        assert_eq!(SurvivalRules::of(&rules, Difficulty::Normal), SurvivalRules::default());
        for rule in ["fall_damage", "fire_damage", "drowning_damage", "natural_health_regeneration"] {
            rules.set(rule, "false").unwrap();
        }
        let off = SurvivalRules::of(&rules, Difficulty::Hard);
        assert_eq!(off.difficulty, Difficulty::Hard);
        let mut survival = fresh();
        survival.rules = off;
        assert_eq!(survival.fall(20.0, None, false), 0);
        assert!(!survival.hurt(3.0, Armor::default(), None, DamageCause::EnderPearl));
        assert!(survival.hurt(3.0, Armor::default(), None, DamageCause::Generic));
        assert_eq!(survival.status.health, 17.0);
        for _ in 0..400 {
            assert!(!survival.breathe(true).drowned);
        }
        assert_eq!(survival.status.health, 17.0);
        for _ in 0..200 {
            survival.tick(&standing([0.0, 64.0, 0.0]));
        }
        assert_eq!(survival.status.health, 17.0);
        let mut heard = Vec::new();
        assert!(!survival.hurt_aloud(Hazard::Lava, 4.0, Armor::default(), &mut || 0.5, &mut heard));
        assert_eq!(survival.status.health, 17.0);
        survival.respawn();
        assert_eq!(survival.rules, off);
        survival.rules = SurvivalRules::default();
        assert_eq!(survival.fall(20.0, None, false), 17);
    }

    #[derive(Default)]
    struct Blocks(std::collections::HashMap<(i32, i32, i32), minecraftoss_player::Block>);

    impl World for Blocks {
        fn block(&self, pos: (i32, i32, i32)) -> Option<minecraftoss_player::Block> {
            self.0.get(&pos).cloned()
        }

        fn set_block(&mut self, pos: (i32, i32, i32), block: Option<minecraftoss_player::Block>) {
            match block {
                Some(block) => {
                    self.0.insert(pos, block);
                }
                None => {
                    self.0.remove(&pos);
                }
            }
        }
    }

    fn ground(blocks: &[((i32, i32, i32), minecraftoss_player::Block)]) -> Blocks {
        let mut world = Blocks::default();
        for x in -2..=2 {
            for z in -2..=2 {
                world.set_block(
                    (x, 0, z),
                    Some(minecraftoss_player::Block::new("minecraft:stone")),
                );
            }
        }
        for (pos, block) in blocks {
            world.set_block(*pos, Some(block.clone()));
        }
        world
    }

    fn block(id: &str) -> minecraftoss_player::Block {
        minecraftoss_player::Block::new(id)
    }

    fn lava_pool() -> Blocks {
        ground(&[
            ((0, 1, 0), block("minecraft:lava")),
            ((0, 2, 0), block("minecraft:lava")),
        ])
    }

    fn unfed() -> Survival {
        let mut survival = fresh();
        survival.status.food.level = 10;
        survival.status.food.saturation = 0.0;
        survival
    }

    fn standing_in(feet: [f64; 3]) -> Motion {
        Motion {
            feet,
            on_ground: true,
            height: 1.8,
            ..Motion::default()
        }
    }

    fn live(
        survival: &mut Survival,
        world: &Blocks,
        motion: &Motion,
        armor: Armor,
        ticks: u32,
    ) -> Vec<&'static str> {
        let mut heard = Vec::new();
        for _ in 0..ticks {
            let from = survival.last_feet().unwrap_or(motion.feet);
            survival.tick(motion);
            let sounds = survival.hazards(world, motion, from, armor, || 0.25);
            heard.extend(sounds.into_iter().map(|sound| sound.event));
        }
        heard
    }

    fn count(heard: &[&str], event: &str) -> usize {
        heard.iter().filter(|e| **e == event).count()
    }

    fn through_bush(block: minecraftoss_player::Block, stride: f64) -> (f32, usize, usize) {
        let world = ground(&[((0, 1, 0), block)]);
        let mut survival = unfed();
        let mut heard = Vec::new();
        let mut stuck = 0;
        for tick in 0..40 {
            let x = if tick % 2 == 0 { 0.5 } else { 0.5 + stride };
            let motion = standing_in([x, 1.0, 0.5]);
            let from = survival.last_feet().unwrap_or(motion.feet);
            survival.tick(&motion);
            let sounds = survival.hazards(&world, &motion, from, Armor::default(), || 0.25);
            heard.extend(sounds.into_iter().map(|sound| sound.event));
            stuck += usize::from(survival.stuck());
        }
        let pricks = count(&heard, "minecraft:entity.player.hurt_sweet_berry_bush");
        (survival.status.health, pricks, stuck)
    }

    #[test]
    fn a_grown_berry_bush_holds_and_pricks_a_moving_player_every_ten_ticks() {
        let bush = |age: &str| block("minecraft:sweet_berry_bush").with("age", age);
        let stuck_step = 0.098 * 0.8;
        let (health, pricks, stuck) = through_bush(bush("2"), stuck_step);
        assert_eq!((pricks, stuck), (4, 40));
        assert!((health - 16.0).abs() < 1e-5, "{health}");
        let (health, pricks, stuck) = through_bush(bush("3"), 0.0031);
        assert_eq!((pricks, stuck, health), (4, 40, 16.0));
        let (health, pricks, stuck) = through_bush(bush("3"), 0.002);
        assert_eq!((pricks, stuck, health), (0, 40, 20.0), "too slow to prick");
        let (health, pricks, stuck) = through_bush(bush("0"), stuck_step);
        assert_eq!((pricks, stuck, health), (0, 40, 20.0), "a sapling bush only holds");
        let (health, pricks, stuck) = through_bush(block("minecraft:cobweb"), stuck_step);
        assert_eq!((pricks, stuck, health), (0, 40, 20.0));
        let (_, _, stuck) = through_bush(block("minecraft:stone"), 0.0);
        assert_eq!(stuck, 0);
    }

    #[test]
    fn lava_burns_four_every_ten_ticks_through_armor_and_sets_the_player_alight() {
        let in_lava = Motion {
            in_lava: true,
            ..standing_in([0.5, 1.0, 0.5])
        };
        let mut bare = unfed();
        let heard = live(&mut bare, &lava_pool(), &in_lava, Armor::default(), 40);
        assert!(
            (bare.status.health - 4.0).abs() < 1e-5,
            "{}",
            bare.status.health
        );
        assert_eq!(count(&heard, "minecraft:entity.generic.burn"), 4);
        assert_eq!(count(&heard, "minecraft:entity.player.hurt_on_fire"), 4);
        assert!(bare.vitals().on_fire);
        assert_eq!(bare.fire_ticks, 300);
        let mut armored = unfed();
        let iron = Armor {
            value: Some((15.0, 0.0)),
        };
        live(&mut armored, &lava_pool(), &in_lava, iron, 40);
        assert!(
            (armored.status.health - (20.0 - 4.0 * 4.0 * 0.48)).abs() < 1e-4,
            "{}",
            armored.status.health
        );
    }

    #[test]
    fn the_player_burns_one_a_second_for_fifteen_seconds_after_the_lava() {
        let mut survival = unfed();
        survival.fire_ticks = 300;
        let diamond = Armor {
            value: Some((20.0, 8.0)),
        };
        let heard = live(
            &mut survival,
            &ground(&[]),
            &standing_in([0.5, 1.0, 0.5]),
            diamond,
            300,
        );
        assert!(
            (survival.status.health - 5.0).abs() < 1e-5,
            "{}",
            survival.status.health
        );
        assert_eq!(count(&heard, "minecraft:entity.player.hurt_on_fire"), 15);
        assert_eq!(count(&heard, "minecraft:entity.generic.extinguish_fire"), 0);
        assert_eq!(survival.status.food.exhaustion, 0.0);
        assert!(!survival.vitals().on_fire);
        assert_eq!(survival.fire_ticks, -FIRE_IMMUNE_TICKS);
    }

    #[test]
    fn fire_resistance_turns_the_damage_away_but_not_the_flames() {
        let in_lava = Motion {
            in_lava: true,
            ..standing_in([0.5, 1.0, 0.5])
        };
        let mut survival = unfed();
        survival
            .status
            .add_effect(EffectKind::FireResistance, 3600, 0);
        let heard = live(&mut survival, &lava_pool(), &in_lava, Armor::default(), 40);
        assert!(heard.is_empty(), "{heard:?}");
        assert_eq!(survival.status.health, 20.0);
        assert!(survival.vitals().on_fire);
        live(
            &mut survival,
            &ground(&[]),
            &standing_in([0.5, 1.0, 0.5]),
            Armor::default(),
            300,
        );
        assert_eq!(survival.status.health, 20.0);
    }

    #[test]
    fn water_and_rain_put_the_fire_out_with_a_hiss() {
        let feet = [0.5, 1.0, 0.5];
        let mut swimmer = unfed();
        swimmer.fire_ticks = 299;
        let pool = ground(&[((0, 1, 0), block("minecraft:water"))]);
        let heard = live(&mut swimmer, &pool, &standing_in(feet), Armor::default(), 1);
        assert_eq!(heard, vec!["minecraft:entity.generic.extinguish_fire"]);
        assert_eq!(swimmer.fire_ticks, -FIRE_IMMUNE_TICKS);
        let rain = Motion {
            in_rain: true,
            ..standing_in(feet)
        };
        let mut walker = unfed();
        walker.fire_ticks = 299;
        let heard = live(&mut walker, &ground(&[]), &rain, Armor::default(), 1);
        assert_eq!(heard, vec!["minecraft:entity.generic.extinguish_fire"]);
        assert!(live(&mut walker, &ground(&[]), &rain, Armor::default(), 5).is_empty());
        assert_eq!(walker.fire_ticks, -FIRE_IMMUNE_TICKS);
    }

    #[test]
    fn fire_hurts_at_once_but_sets_the_player_alight_a_second_later() {
        let motion = standing_in([0.5, 1.0, 0.5]);
        let fire = ground(&[((0, 1, 0), block("minecraft:fire"))]);
        let mut survival = unfed();
        live(&mut survival, &ground(&[]), &motion, Armor::default(), 1);
        assert_eq!(survival.fire_ticks, -FIRE_IMMUNE_TICKS);
        let heard = live(&mut survival, &fire, &motion, Armor::default(), 19);
        assert!(!survival.vitals().on_fire);
        assert!(
            (survival.status.health - 18.0).abs() < 1e-5,
            "{}",
            survival.status.health
        );
        assert_eq!(count(&heard, "minecraft:entity.player.hurt_on_fire"), 2);
        live(&mut survival, &fire, &motion, Armor::default(), 1);
        assert_eq!(survival.fire_ticks, FIRE_IGNITE_TICKS);
        let mut soul = unfed();
        let soul_fire = ground(&[((0, 1, 0), block("minecraft:soul_fire"))]);
        live(&mut soul, &soul_fire, &motion, Armor::default(), 1);
        assert!(
            (soul.status.health - 18.0).abs() < 1e-5,
            "{}",
            soul.status.health
        );
    }

    #[test]
    fn magma_burns_only_players_who_do_not_sneak() {
        let magma = ground(&[((0, 0, 0), block("minecraft:magma_block"))]);
        let motion = standing_in([0.5, 1.0, 0.5]);
        let mut walker = unfed();
        let heard = live(&mut walker, &magma, &motion, Armor::default(), 20);
        assert!(
            (walker.status.health - 18.0).abs() < 1e-5,
            "{}",
            walker.status.health
        );
        assert_eq!(count(&heard, "minecraft:entity.player.hurt_on_fire"), 2);
        assert!(!walker.vitals().on_fire);
        let mut sneaker = unfed();
        let sneaking = Motion {
            crouching: true,
            ..motion
        };
        assert!(live(&mut sneaker, &magma, &sneaking, Armor::default(), 20).is_empty());
        assert_eq!(sneaker.status.health, 20.0);
    }

    #[test]
    fn magma_burns_a_player_on_its_edge_over_a_drop_but_not_beside_it() {
        let mut ledge = ground(&[((0, 0, 0), block("minecraft:magma_block"))]);
        ledge.set_block((1, 0, 0), None);
        let mut overhanging = unfed();
        let hovering = standing_in([1.25, 1.005, 0.5]);
        live(&mut overhanging, &ledge, &hovering, Armor::default(), 20);
        assert!((overhanging.status.health - 18.0).abs() < 1e-5);
        let beside = ground(&[((1, 0, 0), block("minecraft:magma_block"))]);
        let mut on_stone = unfed();
        let mostly_on_stone = standing_in([0.95, 1.0, 0.5]);
        live(
            &mut on_stone,
            &beside,
            &mostly_on_stone,
            Armor::default(),
            20,
        );
        assert_eq!(on_stone.status.health, 20.0);
    }

    #[test]
    fn cauldrons_bubble_columns_kelp_and_powder_snow_put_the_fire_out() {
        let (inside, wading) = ([0.5, 1.25, 0.5], [0.5, 1.0, 0.5]);
        for (put_out, feet) in [
            (block("minecraft:water_cauldron").with("level", "1"), inside),
            (
                block("minecraft:powder_snow_cauldron").with("level", "3"),
                inside,
            ),
            (block("minecraft:bubble_column"), wading),
            (block("minecraft:kelp_plant"), wading),
            (block("minecraft:powder_snow"), wading),
        ] {
            let id = put_out.id.clone();
            let mut survival = unfed();
            survival.fire_ticks = 299;
            let world = ground(&[((0, 1, 0), put_out)]);
            let heard = live(
                &mut survival,
                &world,
                &standing_in(feet),
                Armor::default(),
                1,
            );
            assert_eq!(
                heard,
                vec!["minecraft:entity.generic.extinguish_fire"],
                "{id}"
            );
            assert_eq!(survival.fire_ticks, -FIRE_IMMUNE_TICKS, "{id}");
        }
        let mut dry = unfed();
        dry.fire_ticks = 299;
        let empty = ground(&[((0, 1, 0), block("minecraft:cauldron"))]);
        assert!(live(&mut dry, &empty, &standing_in(inside), Armor::default(), 1).is_empty());
        assert_eq!(dry.fire_ticks, 298);
    }

    #[test]
    fn a_lava_cauldron_burns_like_lava() {
        let cauldron = ground(&[((0, 1, 0), block("minecraft:lava_cauldron"))]);
        let mut survival = unfed();
        let inside = standing_in([0.5, 1.25, 0.5]);
        let heard = live(&mut survival, &cauldron, &inside, Armor::default(), 1);
        assert!((survival.status.health - 16.0).abs() < 1e-5);
        assert_eq!(count(&heard, "minecraft:entity.generic.burn"), 1);
        assert_eq!(survival.fire_ticks, LAVA_IGNITE_TICKS);
    }

    #[test]
    fn lit_campfires_burn_one_and_soul_campfires_two() {
        let on_top = standing_in([0.5, 1.0 + 7.0 / 16.0, 0.5]);
        for (id, lit, damage) in [
            ("minecraft:campfire", "true", 1.0),
            ("minecraft:soul_campfire", "true", 2.0),
            ("minecraft:campfire", "false", 0.0),
        ] {
            let camp = ground(&[((0, 1, 0), block(id).with("lit", lit))]);
            let mut survival = unfed();
            live(&mut survival, &camp, &on_top, Armor::default(), 1);
            assert!(
                (survival.status.health - (20.0 - damage)).abs() < 1e-5,
                "{id} {lit}"
            );
            assert!(!survival.vitals().on_fire);
        }
    }

    #[test]
    fn dying_in_lava_respawns_without_fire() {
        let in_lava = Motion {
            in_lava: true,
            ..standing_in([0.5, 1.0, 0.5])
        };
        let mut survival = unfed();
        live(&mut survival, &lava_pool(), &in_lava, Armor::default(), 60);
        assert_eq!(survival.status.health, 0.0);
        assert!(survival.vitals().on_fire);
        survival.respawn();
        assert!(!survival.vitals().on_fire);
        assert_eq!(survival.last_feet(), None);
    }

    #[test]
    fn hearts_blink_after_damage() {
        let mut survival = fresh();
        survival.tick(&standing([0.0; 3]));
        survival.hurt(4.0, Armor::default(), None, DamageCause::Generic);
        let blinks: Vec<bool> = (0..20)
            .map(|_| {
                survival.tick(&standing([0.0; 3]));
                survival.vitals().blink
            })
            .collect();
        assert!(blinks.iter().any(|b| *b) && blinks.iter().any(|b| !*b));
    }

    fn death_language() -> std::collections::HashMap<String, String> {
        [
            ("death.attack.lava", "%1$s tried to swim in lava"),
            (
                "death.attack.lava.player",
                "%1$s tried to swim in lava to escape %2$s",
            ),
            ("death.attack.drown", "%1$s drowned"),
            ("death.attack.magic", "%1$s was killed by magic"),
            ("death.attack.generic", "%1$s died"),
            ("death.attack.outOfWorld", "%1$s fell out of the world"),
            (
                "death.attack.outOfWorld.player",
                "%1$s didn't want to live in the same world as %2$s",
            ),
            ("death.fell.accident.ladder", "%1$s fell off a ladder"),
            ("entity.minecraft.zombie", "Zombie"),
            ("block.minecraft.lava", "Lava"),
        ]
        .into_iter()
        .map(|(key, text)| (key.to_owned(), text.to_owned()))
        .collect()
    }

    fn death(survival: &Survival) -> (String, String) {
        let line = survival.death_line(&death_language(), |_| true);
        (line.message, line.killer)
    }

    fn said(message: &str, killer: &str) -> (String, String) {
        (message.to_owned(), killer.to_owned())
    }

    #[test]
    fn dying_in_lava_tries_to_swim_in_lava_and_escapes_a_zombie_that_hit_first() {
        let in_lava = Motion {
            in_lava: true,
            ..standing_in([0.5, 1.0, 0.5])
        };
        let mut alone = unfed();
        live(&mut alone, &lava_pool(), &in_lava, Armor::default(), 60);
        assert_eq!(alone.status.health, 0.0);
        assert_eq!(death(&alone), said("%1$s tried to swim in lava", "Lava"));
        let mut chased = unfed();
        let zombie = DamageCause::mob(
            crate::death::MobAttack::Melee,
            Some(4),
            Some("minecraft:zombie"),
        );
        assert!(chased.hurt(2.0, Armor::default(), None, zombie));
        live(&mut chased, &lava_pool(), &in_lava, Armor::default(), 60);
        assert_eq!(chased.status.health, 0.0);
        assert_eq!(
            death(&chased),
            said("%1$s tried to swim in lava to escape Zombie", "Zombie")
        );
    }

    #[test]
    fn a_zombie_hit_the_shield_blocks_still_gets_the_kill_credit() {
        let in_lava = Motion {
            in_lava: true,
            ..standing_in([0.5, 1.0, 0.5])
        };
        let zombie = DamageCause::mob(
            crate::death::MobAttack::Melee,
            Some(4),
            Some("minecraft:zombie"),
        );
        let mut blocker = unfed();
        blocker.hurt_blocked(zombie);
        assert_eq!(blocker.status.health, 20.0);
        live(&mut blocker, &lava_pool(), &in_lava, Armor::default(), 60);
        assert_eq!(blocker.status.health, 0.0);
        assert_eq!(
            death(&blocker),
            said("%1$s tried to swim in lava to escape Zombie", "Zombie")
        );
        let mut recovering = unfed();
        assert!(recovering.hurt(1.0, Armor::default(), None, DamageCause::Generic));
        recovering.hurt_blocked(zombie);
        live(&mut recovering, &lava_pool(), &in_lava, Armor::default(), 60);
        assert_eq!(
            death(&recovering),
            said("%1$s tried to swim in lava", "Lava")
        );
    }

    #[test]
    fn a_long_fall_after_climbing_falls_off_the_ladder() {
        let mut survival = fresh();
        survival.set_climbable(Some(FallLocation::Ladder));
        assert_eq!(survival.fall(30.0, Some("minecraft:stone"), false), 27);
        assert_eq!(survival.status.health, 0.0);
        assert_eq!(
            death(&survival),
            said("%1$s fell off a ladder", "fell off a ladder")
        );
    }

    #[test]
    fn running_out_of_air_drowns() {
        let mut survival = fresh();
        for _ in 0..1000 {
            survival.status.tick_effects();
            survival.breathe(true);
        }
        assert_eq!(survival.status.health, 0.0);
        assert_eq!(death(&survival), said("%1$s drowned", "drowned"));
    }

    #[test]
    fn a_potion_of_harming_kills_by_magic_and_a_respawn_forgets_it() {
        let mut survival = fresh();
        survival.status.health = 5.0;
        drink(&mut survival, "minecraft:harming");
        assert_eq!(survival.status.health, 0.0);
        assert_eq!(
            death(&survival),
            said("%1$s was killed by magic", "was killed by magic")
        );
        survival.respawn();
        assert_eq!(death(&survival), said("%1$s died", "died"));
    }

    fn creative() -> Survival {
        let mut survival = fresh();
        survival.set_game_mode(&minecraft_terrain::game_mode::PlayerMode::new(GameMode::Creative));
        survival
    }

    #[test]
    fn creative_and_spectator_players_are_invulnerable_and_survival_and_adventure_ones_are_not() {
        use minecraft_terrain::game_mode::{MODES, PlayerMode};
        for mode in MODES {
            let mut survival = fresh();
            survival.set_game_mode(&PlayerMode::new(mode));
            let hurt = survival.hurt(3.0, Armor::default(), None, DamageCause::Generic);
            assert_eq!(hurt, !mode.may_fly(), "{mode:?}");
        }
    }

    #[test]
    fn an_invulnerable_player_takes_no_hits_falls_lava_fire_poison_or_harming() {
        let mut survival = creative();
        assert!(!survival.hurt(6.0, Armor::default(), Some([1.0, 2.0, 3.0]), DamageCause::Generic));
        assert_eq!(survival.fall(30.0, None, false), 0);
        drink(&mut survival, "minecraft:harming");
        drink(&mut survival, "minecraft:strong_poison");
        let in_lava = Motion {
            in_lava: true,
            ..standing_in([0.5, 1.0, 0.5])
        };
        let heard = live(&mut survival, &lava_pool(), &in_lava, Armor::default(), 200);
        assert_eq!(survival.status.health, 20.0);
        assert!(survival.status.effect(EffectKind::Poison).is_some());
        assert_eq!(count(&heard, "minecraft:entity.player.hurt_on_fire"), 0);
        assert_eq!(count(&heard, "minecraft:entity.generic.burn"), 0);
        assert!(survival.fire_ticks <= 1, "{}", survival.fire_ticks);
        assert!(survival.vitals().on_fire);
        live(&mut survival, &ground(&[]), &standing_in([0.5, 1.0, 0.5]), Armor::default(), 40);
        assert!(survival.fire_ticks <= 0, "{}", survival.fire_ticks);
        assert!(!survival.vitals().on_fire);
        assert_eq!(survival.settle(100, 100), Settle::default());
    }

    #[test]
    fn a_spectator_touches_no_lava_fire_or_cobweb_and_a_burn_from_before_goes_out() {
        let spectator = || {
            let mut survival = fresh();
            survival.set_game_mode(&minecraft_terrain::game_mode::PlayerMode::new(GameMode::Spectator));
            survival
        };
        let in_lava = Motion {
            in_lava: true,
            ..standing_in([0.5, 1.0, 0.5])
        };
        let mut swimmer = spectator();
        let heard = live(&mut swimmer, &lava_pool(), &in_lava, Armor::default(), 200);
        assert!(heard.is_empty(), "{heard:?}");
        assert!(swimmer.fire_ticks <= 0, "{}", swimmer.fire_ticks);
        assert!(!swimmer.vitals().on_fire);
        let mut burning = spectator();
        burning.fire_ticks = 300;
        live(&mut burning, &ground(&[((0, 1, 0), block("minecraft:fire"))]), &standing_in([0.5, 1.0, 0.5]), Armor::default(), 2);
        assert!(burning.fire_ticks <= 0, "{}", burning.fire_ticks);
        assert!(!burning.vitals().on_fire);
        let web = ground(&[((0, 1, 0), block("minecraft:cobweb"))]);
        let mut ghost = spectator();
        live(&mut ghost, &web, &standing_in([0.5, 1.0, 0.5]), Armor::default(), 1);
        assert!(!ghost.stuck());
        ghost.respawn();
        live(&mut ghost, &web, &standing_in([0.5, 1.0, 0.5]), Armor::default(), 1);
        assert!(!ghost.stuck());
        let mut creative = creative();
        live(&mut creative, &web, &standing_in([0.5, 1.0, 0.5]), Armor::default(), 1);
        assert!(creative.stuck());
    }

    #[test]
    fn an_invulnerable_player_never_gets_hungry() {
        let mut runner = creative();
        let saturation = runner.status.food.saturation;
        runner.status.add_effect(EffectKind::Hunger, 600, 2);
        for i in 0..1000 {
            runner.tick(&Motion {
                feet: [f64::from(i) * 0.25, 0.0, 0.0],
                on_ground: true,
                sprinting: true,
                jumping: i % 2 == 0,
                ..Motion::default()
            });
            runner.exhaust(ATTACK_EXHAUSTION);
        }
        assert_eq!(runner.status.food.level, 20);
        assert_eq!(runner.status.food.saturation, saturation);
        let mut starving = creative();
        starving.status.food.level = 0;
        starving.status.food.saturation = 0.0;
        for _ in 0..400 {
            starving.tick(&standing([0.0, 64.0, 0.0]));
        }
        assert_eq!(starving.status.health, 20.0);
    }

    #[test]
    fn an_invulnerable_player_never_runs_out_of_air() {
        let mut diver = creative();
        for _ in 0..400 {
            assert!(!diver.breathe(true).drowned);
        }
        assert_eq!(diver.status.air, diver.status.max_air);
        assert_eq!(diver.status.health, 20.0);
        diver.rules.drowning_damage = false;
        diver.breathe(true);
        assert_eq!(diver.status.air, diver.status.max_air);
    }

    #[test]
    fn mw2_damage_reaching_an_invulnerable_player_is_healed_back() {
        let mut survival = creative();
        survival.observe(60, 100);
        assert_eq!(survival.status.health, 20.0);
        assert_eq!(survival.settle(60, 100).set, Some(100));
    }

    #[test]
    fn kill_still_kills_an_invulnerable_player_and_a_respawn_keeps_the_game_mode() {
        let mut survival = creative();
        survival.kill(DamageCause::Kill);
        assert_eq!(survival.status.health, 0.0);
        assert_eq!(survival.settle(100, 100).damage, Some((100, None)));
        survival.respawn();
        assert!(survival.invulnerable && survival.creative);
        assert!(!survival.hurt(3.0, Armor::default(), None, DamageCause::Generic));
        survival.respawn_keeping_experience();
        assert!(survival.invulnerable);
        survival.set_game_mode(&minecraft_terrain::game_mode::PlayerMode::new(GameMode::Survival));
        assert!(survival.hurt(3.0, Armor::default(), None, DamageCause::Generic));
    }

    #[test]
    fn creative_players_eat_when_full_and_keep_their_food() {
        let mut survival = creative();
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:cooked_beef", 3));
        let stack = inventory.slots[0].clone().unwrap();
        assert!(survival.start_eating(0, &stack));
        let mut done = false;
        for _ in 0..40 {
            if let Bite::Done { .. } = survival.chew(&mut inventory, 0, || 0.5) {
                done = true;
                break;
            }
        }
        assert!(done);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3));
        let mut full = fresh();
        assert!(!full.start_eating(0, &stack));
    }

    struct Floor(i32);

    impl World for Floor {
        fn block(&self, _: (i32, i32, i32)) -> Option<minecraftoss_player::Block> {
            None
        }

        fn set_block(&mut self, _: (i32, i32, i32), _: Option<minecraftoss_player::Block>) {}

        fn min_y(&self) -> i32 {
            self.0
        }
    }

    fn falling(
        survival: &mut Survival,
        world: &Floor,
        y: f64,
        armor: Armor,
        ticks: u32,
    ) -> Vec<&'static str> {
        let motion = Motion {
            feet: [0.5, y, 0.5],
            height: 1.8,
            ..Motion::default()
        };
        let mut heard = Vec::new();
        for _ in 0..ticks {
            let from = survival.last_feet().unwrap_or(motion.feet);
            survival.tick(&motion);
            let sounds = survival.hazards(world, &motion, from, armor, || 0.25);
            heard.extend(sounds.into_iter().map(|sound| sound.event));
        }
        heard
    }

    #[test]
    fn the_void_hurts_four_every_ten_ticks_through_armor_and_resistance_below_the_floor() {
        let iron = Armor {
            value: Some((15.0, 0.0)),
        };
        let mut above = unfed();
        assert!(falling(&mut above, &Floor(-64), -128.0, iron, 40).is_empty());
        assert_eq!(above.status.health, 20.0);
        let mut below = unfed();
        below.status.add_effect(EffectKind::Resistance, 400, 4);
        let heard = falling(&mut below, &Floor(-64), -128.01, iron, 40);
        assert_eq!(below.status.health, 4.0);
        assert_eq!(count(&heard, "minecraft:entity.player.hurt"), 4);
        let resistance = below.status.effect(EffectKind::Resistance);
        assert_eq!(resistance.map(|effect| effect.amplifier), Some(4));
        let mut nether = unfed();
        falling(&mut nether, &Floor(0), -63.9, Armor::default(), 40);
        assert_eq!(nether.status.health, 20.0);
        falling(&mut nether, &Floor(0), -64.1, Armor::default(), 40);
        assert_eq!(nether.status.health, 4.0);
    }

    #[test]
    fn creative_and_spectator_players_fall_out_of_the_world_past_a_totem() {
        use minecraft_terrain::game_mode::PlayerMode;
        for mode in [GameMode::Creative, GameMode::Spectator] {
            let mut survival = unfed();
            survival.set_game_mode(&PlayerMode::new(mode));
            let mut inventory = Inventory::default();
            inventory.slots[40] = Some(ItemStack::new("minecraft:totem_of_undying", 1));
            let heard = falling(&mut survival, &Floor(-64), -200.0, Armor::default(), 41);
            assert_eq!(survival.status.health, 0.0, "{mode:?}");
            assert_eq!(count(&heard, "minecraft:entity.player.hurt"), 4);
            assert!(survival.killed());
            assert!(!survival.protect(&mut inventory, 0));
            assert!(inventory.slots[40].is_some());
            assert_eq!(
                death(&survival),
                said("%1$s fell out of the world", "fell out of the world")
            );
            assert_eq!(survival.settle(100, 100).damage, Some((100, None)));
        }
    }

    #[test]
    fn falling_out_of_the_world_after_a_zombie_hit_names_the_zombie() {
        let mut survival = unfed();
        let zombie = DamageCause::mob(
            crate::death::MobAttack::Melee,
            Some(4),
            Some("minecraft:zombie"),
        );
        assert!(survival.hurt(2.0, Armor::default(), None, zombie));
        falling(&mut survival, &Floor(-64), -130.0, Armor::default(), 60);
        assert_eq!(survival.status.health, 0.0);
        assert_eq!(
            death(&survival),
            said(
                "%1$s didn't want to live in the same world as Zombie",
                "Zombie"
            )
        );
    }
}
