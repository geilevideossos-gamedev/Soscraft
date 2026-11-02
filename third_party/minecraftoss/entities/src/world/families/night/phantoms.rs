mod goals;
#[cfg(test)]
mod tests;

pub use self::goals::{AttackPhase, GOAL_NAMES, PhantomGoals, TARGET_NAMES, block_of};
use crate::fluid::FluidFrame;
use crate::monster_ai::{MonsterAi, MonsterKind, Target};
use crate::movement::Body;
use crate::tempt::PlayerCandidate;
use crate::wither::wrap_degrees;
use crate::world::families::{Family, FamilyMob, MobCore, MonsterSpec};
use crate::world::{EntityWorld, PlayerHitKind, Voice};
use glam::DVec3;
use minecraftoss_player::{World, mth};

pub const KIND: &str = "minecraft:phantom";
pub const WIDTH: f32 = 0.9;
pub const HEIGHT: f32 = 0.5;
pub const EYE_HEIGHT: f32 = 0.175;
pub const MAX_HEALTH: f32 = 20.0;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const ATTACK_DAMAGE: f32 = 2.0;
pub const MAX_SIZE: i32 = 64;
pub const START_SPEED: f32 = 0.1;
pub const CRUISE_SPEED: f32 = 0.2;
pub const TOP_SPEED: f32 = 1.8;
pub const TURN: f32 = 4.0;
pub const TRAVEL_SPEED: f32 = 0.2;
pub const SWOOP_SOUND: &str = "entity.phantom.swoop";
pub const SWOOP_VOLUME: f32 = 10.0;
pub const BITE_SOUND: &str = "entity.phantom.bite";
pub const BITE_VOLUME: f32 = 0.3;

#[derive(Clone)]
pub struct Phantom {
    pub size: i32,
    pub attack_damage: f32,
    pub speed: f32,
    pub ai: Box<MonsterAi>,
}

impl Phantom {
    pub fn goals(&self) -> Option<&PhantomGoals> {
        self.ai.state.extra.get::<PhantomGoals>()
    }

    pub fn goals_mut(&mut self) -> &mut PhantomGoals {
        goals::shared(&mut self.ai.state.extra)
    }
}

pub fn scale(size: i32) -> f32 {
    1.0 + 0.15 * size as f32
}

pub fn set_size(mob: &mut FamilyMob<Phantom>, size: i32) {
    let size = size.clamp(0, MAX_SIZE);
    if size == mob.state.size {
        return;
    }
    let scale = scale(size);
    mob.state.size = size;
    mob.state.attack_damage = (6 + size) as f32;
    mob.core.body.width = WIDTH * scale;
    mob.core.body.height = HEIGHT * scale;
    mob.core.eye_height = EYE_HEIGHT * scale;
}

fn approach(from: f32, to: f32, step: f32) -> f32 {
    let step = step.abs();
    if from < to {
        (from + step).clamp(from, to)
    } else {
        (from - step).clamp(to, from)
    }
}

fn degrees_difference(from: f32, to: f32) -> f32 {
    wrap_degrees(to - from)
}

fn approach_degrees(from: f32, to: f32, step: f32) -> f32 {
    approach(from, from + degrees_difference(from, to), step)
}

pub fn move_control(speed: &mut f32, yaw: &mut f32, x_rot: &mut f32, body: &mut Body, to: DVec3) {
    if body.horizontal_collision {
        *yaw += 180.0;
        *speed = START_SPEED;
    }
    let p = body.position;
    let (mut dx, dy, mut dz) = (to.x - p.x, to.y - p.y, to.z - p.z);
    let mut flat = (dx * dx + dz * dz).sqrt();
    if flat.abs() <= f64::from(1.0e-5_f32) {
        return;
    }
    let squash = 1.0 - (dy * f64::from(0.7_f32)).abs() / flat;
    dx *= squash;
    dz *= squash;
    flat = (dx * dx + dz * dz).sqrt();
    let length = (dx * dx + dz * dz + dy * dy).sqrt();
    let old = *yaw;
    let heading = mth::atan2(dz, dx) as f32;
    let from = wrap_degrees(*yaw + 90.0);
    let toward = wrap_degrees(heading * 57.295776);
    *yaw = approach_degrees(from, toward, TURN) - 90.0;
    *speed = if degrees_difference(old, *yaw).abs() < 3.0 {
        approach(*speed, TOP_SPEED, 0.005 * (TOP_SPEED / *speed))
    } else {
        approach(*speed, CRUISE_SPEED, 0.025)
    };
    let pitch = -(mth::atan2(-dy, flat) * mth::RAD_TO_DEG) as f32;
    *x_rot = pitch.clamp(-90.0, 90.0);
    let turn = f64::from((*yaw + 90.0) * 0.017453292);
    let wanted = DVec3::new(
        f64::from(*speed * mth::cos(turn)) * (dx / length).abs(),
        f64::from(*speed * mth::sin(f64::from(pitch * 0.017453292))) * (dy / length).abs(),
        f64::from(*speed * mth::sin(turn)) * (dz / length).abs(),
    );
    body.velocity += (wanted - body.velocity) * 0.2;
}

#[derive(Default)]
struct Flight {
    strike: Option<Target>,
    at: DVec3,
    swoop: Option<f32>,
    bit: bool,
}

fn fly<W: World>(
    world: &mut EntityWorld,
    id: u64,
    level: &W,
    players: &[PlayerCandidate],
) -> Flight {
    let (game_time, difficulty, sea_level) = (world.game_time, world.difficulty, world.sea_level);
    let Some(mob) = world.family_mob_mut::<Phantom>(id) else {
        return Flight::default();
    };
    let FamilyMob { core, state } = mob;
    let position = core.body.position;
    if players
        .iter()
        .any(|p| p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0)
    {
        core.no_action_time = 0;
    }
    core.no_action_time += 1;
    core.body.trim_small_velocity();
    let body = &core.body;
    let fluid = FluidFrame::sample(level, body.position, body.width, body.height);
    let ai = &mut state.ai;
    ai.yaw = core.yaw;
    ai.state.body = core.body.clone();
    ai.state.health = core.health;
    ai.state.max_health = core.max_health;
    ai.state.eye_height = core.eye_height;
    ai.state.game_time = game_time;
    ai.state.difficulty = difficulty;
    ai.state.players = players.to_vec();
    ai.state.no_action_time = core.no_action_time;
    ai.state.tick_count = core.tick_count;
    ai.state.hurt_by = core.hurt_by;
    ai.state.fluid = fluid;
    ai.state.random = std::mem::take(&mut core.random);
    let goals = goals::shared(&mut ai.state.extra);
    goals.sea_level = sea_level;
    goals.hurt_recently = core.damage.hurt_ticks > 0;
    let full = core.tick_count <= 1 || (core.tick_count + core.id as i32) % 2 == 0;
    ai.tick_goals(level, full);
    core.random = std::mem::take(&mut ai.state.random);
    let strike = ai.state.attack.take();
    let goals = state.goals_mut();
    let flight = Flight {
        strike,
        at: core.body.position,
        swoop: goals.swoop.take(),
        bit: std::mem::take(&mut goals.bit),
    };
    let to = goals.move_target;
    move_control(
        &mut state.speed,
        &mut core.yaw,
        &mut core.look.pitch,
        &mut core.body,
        to,
    );
    core.look.head_yaw = core.yaw;
    state.ai.yaw = core.yaw;
    state.ai.body_rotation.body_yaw = core.yaw;
    core.body
        .travel_flying(level, DVec3::ZERO, TRAVEL_SPEED, core.yaw, fluid);
    flight
}

fn push_voice(world: &mut EntityWorld, id: u64, voice: Voice, at: DVec3) {
    if let Some(mob) = world.family_mob_mut::<Phantom>(id) {
        mob.core.voices.push((voice, at));
    }
}

impl Family for Phantom {
    const KEY: u8 = 80;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let mut ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        ai.state.max_health = MAX_HEALTH;
        ai.state.extra.set(PhantomGoals::default());
        Some(FamilyMob {
            core,
            state: Phantom {
                size: 0,
                attack_damage: ATTACK_DAMAGE,
                speed: START_SPEED,
                ai: Box::new(ai),
            },
        })
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        if base.alive && !base.no_ai {
            let flight = fly(world, id, &*level, players);
            if let Some(pitch) = flight.swoop {
                let swoop = Voice::Event(SWOOP_SOUND, SWOOP_VOLUME, pitch);
                push_voice(world, id, swoop, flight.at);
            }
            if let Some(target) = flight.strike {
                let damage = world
                    .family::<Self>()
                    .iter()
                    .find(|mob| mob.core.id == id)
                    .map_or(ATTACK_DAMAGE, |mob| mob.state.attack_damage);
                let kind = PlayerHitKind::Melee {
                    attacker: flight.at,
                    hunger_ticks: 0,
                    lift: 0.0,
                    throw: DVec3::ZERO,
                    disable_shield_seconds: 0.0,
                };
                world.family_strike(id, target, damage, kind);
            }
            if flight.bit {
                let pitch = world.level_random.next_float() * 0.1 + 0.9;
                let (x, y, z) = block_of(flight.at);
                let centre = DVec3::new(f64::from(x), f64::from(y), f64::from(z)) + 0.5;
                push_voice(
                    world,
                    id,
                    Voice::Event(BITE_SOUND, BITE_VOLUME, pitch),
                    centre,
                );
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn enemy(_kind: &str) -> bool {
        true
    }

    fn monster(kind: &str) -> Option<&'static MonsterSpec> {
        (kind == KIND).then_some(&goals::SPEC)
    }
}
