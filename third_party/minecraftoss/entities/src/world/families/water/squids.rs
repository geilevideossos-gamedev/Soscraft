use crate::age::Age;
use crate::health::DamageResult;
use crate::monster_ai::Target;
use crate::tempt::PlayerCandidate;
use crate::world::emissions::play_movement;
use crate::world::families::{Breath, Family, FamilyMob, Hurt, MobCore, movement_sounds};
use crate::world::living::fall_damage;
use crate::world::{DamageSourceKind, EntityWorld, Voice, voice_pitch};
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Block, World, mth};
use std::f32::consts::{PI, TAU};

pub const SQUID: &str = "minecraft:squid";
pub const GLOW_SQUID: &str = "minecraft:glow_squid";
pub const SIZE: f32 = 0.8;
pub const EYE_HEIGHT: f32 = 0.4;
pub const BABY_SIZE: f32 = 0.5;
pub const BABY_EYE_HEIGHT: f32 = 0.37;
pub const MAX_HEALTH: f32 = 10.0;
pub const AMBIENT_INTERVAL: i32 = 120;
pub const VOLUME: f32 = 0.4;
pub const SQUIRT: &str = "entity.squid.squirt";
pub const GLOW_SQUIRT: &str = "entity.glow_squid.squirt";
pub const DARK_TICKS: i32 = 100;
pub const IDLE_TICKS: i32 = 100;
pub const FLEE_RANGE: f64 = 10.0;
pub const INK_SPECKS: usize = 30;
pub const INK_SPEED: f32 = 0.1;
const TURN_CHANCE: u32 = 25;
const STILL: f64 = 1.0e-5_f32 as f64;
const DEGREES: f32 = mth::RAD_TO_DEG as f32;
const RADIANS: f32 = 0.017_453_292;

#[derive(Clone, Debug)]
pub struct Squid {
    pub glow: bool,
    pub age: Age,
    pub x_body_rot: f32,
    pub x_body_rot_o: f32,
    pub z_body_rot: f32,
    pub z_body_rot_o: f32,
    pub y_body_rot_o: f32,
    pub tentacle_movement: f32,
    pub tentacle_movement_old: f32,
    pub tentacle_angle: f32,
    pub tentacle_angle_old: f32,
    pub tentacle_speed: f32,
    pub rotate_speed: f32,
    pub movement_vector: DVec3,
    pub fleeing: Option<i32>,
    pub ink_spurts: u32,
    pub dark_ticks: i32,
}

impl Squid {
    fn new(glow: bool) -> Self {
        Self {
            glow,
            age: Age::default(),
            x_body_rot: 0.0,
            x_body_rot_o: 0.0,
            z_body_rot: 0.0,
            z_body_rot_o: 0.0,
            y_body_rot_o: 0.0,
            tentacle_movement: 0.0,
            tentacle_movement_old: 0.0,
            tentacle_angle: 0.0,
            tentacle_angle_old: 0.0,
            tentacle_speed: 0.0,
            rotate_speed: 0.0,
            movement_vector: DVec3::ZERO,
            fleeing: None,
            ink_spurts: 0,
            dark_ticks: 0,
        }
    }

    pub fn has_movement_vector(&self) -> bool {
        self.movement_vector.length_squared() > STILL
    }
}

pub fn tentacle_speed(random: &mut LegacyRandom) -> f32 {
    1.0 / (random.next_float() + 1.0) * 0.2
}

pub fn set_baby(mob: &mut FamilyMob<Squid>, baby: bool) {
    let (size, eye) = if baby {
        (BABY_SIZE, BABY_EYE_HEIGHT)
    } else {
        (SIZE, EYE_HEIGHT)
    };
    let core = &mut mob.core;
    core.baby = baby;
    core.body.width = size;
    core.body.height = size;
    core.eye_height = eye;
}

pub fn rotate_vector(v: DVec3, x_body_rot: f32, y_body_rot: f32) -> DVec3 {
    let pitch = f64::from(x_body_rot * RADIANS);
    let (cos, sin) = (f64::from(mth::cos(pitch)), f64::from(mth::sin(pitch)));
    let v = DVec3::new(v.x, v.y * cos + v.z * sin, v.z * cos - v.y * sin);
    let yaw = f64::from(-y_body_rot * RADIANS);
    let (cos, sin) = (f64::from(mth::cos(yaw)), f64::from(mth::sin(yaw)));
    DVec3::new(v.x * cos + v.z * sin, v.y, v.z * cos - v.x * sin)
}

pub fn ink(
    random: &mut LegacyRandom,
    x_body_rot: f32,
    y_body_rot: f32,
    baby: bool,
    position: DVec3,
) -> Vec<(DVec3, DVec3)> {
    let below = rotate_vector(DVec3::NEG_Y, x_body_rot, y_body_rot) + position;
    let origin = below + DVec3::new(0.0, 0.5, 0.0);
    let reach = if baby { 0.1_f32 } else { 0.3 };
    (0..INK_SPECKS)
        .map(|_| {
            let x = f64::from(random.next_float()) * 0.6 - 0.3;
            let z = f64::from(random.next_float()) * 0.6 - 0.3;
            let out = rotate_vector(DVec3::new(x, -1.0, z), x_body_rot, y_body_rot);
            let d = out * f64::from(reach + random.next_float() * 2.0);
            let speed = |part: f64| f64::from(part as f32 * INK_SPEED);
            (origin, DVec3::new(speed(d.x), speed(d.y), speed(d.z)))
        })
        .collect()
}

fn squirt(mob: &mut FamilyMob<Squid>) {
    let FamilyMob { core, state } = mob;
    let pitch = voice_pitch(&mut core.random, core.baby);
    let event = if state.glow { GLOW_SQUIRT } else { SQUIRT };
    let position = core.body.position;
    core.voices
        .push((Voice::Event(event, VOLUME, pitch), position));
    let (x, y) = (state.x_body_rot_o, state.y_body_rot_o);
    let _ = ink(&mut core.random, x, y, core.baby, position);
    state.ink_spurts = state.ink_spurts.wrapping_add(1);
    if state.glow {
        state.dark_ticks = DARK_TICKS;
    }
}

fn is_air(block: Option<&Block>) -> bool {
    block.is_none_or(|block| {
        matches!(
            block.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

fn attacker_position(world: &EntityWorld, id: u64, players: &[PlayerCandidate]) -> Option<DVec3> {
    let (target, _) = world.family_core(id)?.1.hurt_by?;
    match target {
        Target::Player(player) => players
            .iter()
            .find(|candidate| candidate.id == player && candidate.alive)
            .map(|candidate| candidate.position),
        Target::Villager(mob) | Target::Mob(mob) => {
            let alive = world.damage_state(mob).is_none_or(|damage| !damage.dead);
            world
                .body_of(mob)
                .filter(|_| alive)
                .map(|body| body.position)
        }
    }
}

fn wander(core: &mut MobCore, state: &mut Squid) {
    if core.no_action_time > IDLE_TICKS {
        state.movement_vector = DVec3::ZERO;
        return;
    }
    let turn = core.random.next_int(TURN_CHANCE) == 0;
    if turn || !core.body.touching_water || !state.has_movement_vector() {
        let angle = core.random.next_float() * TAU;
        let rise = -0.1_f32 + core.random.next_float() * 0.2;
        state.movement_vector = DVec3::new(
            f64::from(mth::cos(f64::from(angle)) * 0.2),
            f64::from(rise),
            f64::from(mth::sin(f64::from(angle)) * 0.2),
        );
    }
}

fn flee<W: World>(state: &mut Squid, level: &W, position: DVec3, attacker: DVec3) {
    let mut away = position - attacker;
    let ahead = position + away;
    let cell = (
        ahead.x.floor() as i32,
        ahead.y.floor() as i32,
        ahead.z.floor() as i32,
    );
    let air = is_air(level.block(cell).as_ref());
    if !level.floatable_fluid(cell) && !air {
        return;
    }
    let length = away.length();
    if length > 0.0 {
        let mut scale = 3.0;
        if length > 5.0 {
            scale -= (length - 5.0) / 5.0;
        }
        if scale > 0.0 {
            away *= scale;
        }
    }
    if air {
        away.y = 0.0;
    }
    state.movement_vector = away / 20.0;
}

fn swim_goals<W: World>(mob: &mut FamilyMob<Squid>, level: &W, attacker: Option<DVec3>) {
    let FamilyMob { core, state } = mob;
    core.no_action_time += 1;
    let full = core.tick_count <= 1 || (core.tick_count + core.id as i32) % 2 == 0;
    let position = core.body.position;
    if full {
        let near =
            attacker.is_some_and(|at| position.distance_squared(at) < FLEE_RANGE * FLEE_RANGE);
        if !(core.body.touching_water && near) {
            state.fleeing = None;
        } else if state.fleeing.is_none() {
            state.fleeing = Some(0);
        }
        wander(core, state);
    }
    if let Some(ticks) = state.fleeing {
        state.fleeing = Some(ticks + 1);
        if let Some(attacker) = attacker {
            flee(state, level, position, attacker);
        }
    }
}

fn travel<W: World>(mob: &mut FamilyMob<Squid>, level: &W) {
    let body = &mut mob.core.body;
    if body.levitation.is_some() || body.slow_falling {
        body.fall_distance = 0.0;
    }
    let landed = body.move_self(level);
    let core = &mut mob.core;
    let fall =
        landed.and_then(|fallen| fall_damage(&core.body, level, fallen, false, &mut core.voices));
    if let Some(damage) = fall {
        let hurt = Hurt::new(damage, "minecraft:fall", DamageSourceKind::Generic).bypassing_armor();
        Squid::hurt(mob, &hurt);
    }
    let sounds = movement_sounds::<Squid>(mob.core.kind);
    let core = &mut mob.core;
    play_movement(
        &mut core.body,
        core.tick_count,
        &mut core.random,
        &mut core.voices,
        sounds,
    );
}

fn grow(mob: &mut FamilyMob<Squid>) {
    let alive = mob.core.alive();
    let _ = mob.state.age.tick(alive);
    let baby = mob.state.age.baby();
    if baby != mob.core.baby {
        set_baby(mob, baby);
    }
}

fn stroke(mob: &mut FamilyMob<Squid>) {
    let FamilyMob { core, state } = mob;
    state.y_body_rot_o = core.yaw;
    state.x_body_rot_o = state.x_body_rot;
    state.z_body_rot_o = state.z_body_rot;
    state.tentacle_movement_old = state.tentacle_movement;
    state.tentacle_angle_old = state.tentacle_angle;
    state.tentacle_movement += state.tentacle_speed;
    if f64::from(state.tentacle_movement) > std::f64::consts::TAU {
        state.tentacle_movement -= TAU;
        if core.random.next_int(10) == 0 {
            state.tentacle_speed = tentacle_speed(&mut core.random);
        }
    }
    let body = &mut core.body;
    if body.touching_water {
        if state.tentacle_movement < PI {
            let stroke = state.tentacle_movement / PI;
            state.tentacle_angle = mth::sin(f64::from(stroke * stroke * PI)) * PI * 0.25;
            if f64::from(stroke) > 0.75 {
                body.velocity = state.movement_vector;
                state.rotate_speed = 1.0;
            } else {
                state.rotate_speed *= 0.8;
            }
        } else {
            state.tentacle_angle = 0.0;
            body.velocity *= 0.9;
            state.rotate_speed *= 0.99;
        }
        let v = body.velocity;
        let horizontal = (v.x * v.x + v.z * v.z).sqrt();
        let heading = -(mth::atan2(v.x, v.z) as f32) * DEGREES;
        core.yaw += (heading - core.yaw) * 0.1;
        state.z_body_rot += PI * state.rotate_speed * 1.5;
        let tilt = -(mth::atan2(horizontal, v.y) as f32) * DEGREES;
        state.x_body_rot += (tilt - state.x_body_rot) * 0.1;
    } else {
        state.tentacle_angle = mth::sin(f64::from(state.tentacle_movement)).abs() * PI * 0.25;
        let rise = match body.levitation {
            Some(amplifier) => 0.05 * (f64::from(amplifier) + 1.0),
            None => body.velocity.y - body.gravity,
        };
        body.velocity = DVec3::new(0.0, rise * f64::from(0.98_f32), 0.0);
        state.x_body_rot += (-90.0 - state.x_body_rot) * 0.02;
    }
    core.look.head_yaw = core.yaw;
}

fn glow(mob: &mut FamilyMob<Squid>) {
    if mob.state.dark_ticks > 0 {
        mob.state.dark_ticks -= 1;
    }
    let random = &mut mob.core.random;
    let _ = (
        random.next_double(),
        random.next_double(),
        random.next_double(),
    );
}

impl Family for Squid {
    const KEY: u8 = 64;
    const KINDS: &'static [&'static str] = &[SQUID, GLOW_SQUID];
    const OWN_DYING_TRAVEL: bool = true;

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let mut core = MobCore::new(kind, position, SIZE, SIZE, EYE_HEIGHT, MAX_HEALTH);
        core.ambient_interval = AMBIENT_INTERVAL;
        let state = Squid::new(kind == GLOW_SQUID);
        Some(FamilyMob { core, state })
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        if let Some(mob) = world
            .family_mob_mut::<Self>(id)
            .filter(|mob| mob.core.tick_count == 0)
        {
            mob.state.tentacle_speed = tentacle_speed(&mut mob.core.random);
        }
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        let attacker = attacker_position(world, id, players);
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            mob.core.body.trim_small_velocity();
            if base.alive && !base.no_ai {
                swim_goals(mob, &*level, attacker);
            }
            if !base.no_ai {
                travel(mob, &*level);
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            grow(mob);
            stroke(mob);
            if mob.state.glow {
                glow(mob);
            }
        }
    }

    fn category(kind: &str) -> &'static str {
        if kind == GLOW_SQUID {
            "underground_water_creature"
        } else {
            "water_creature"
        }
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        let blamed = hurt.attacker.is_some() || mob.core.hurt_by.is_some();
        let result = mob.core.hurt_plain(hurt.amount);
        if result.applied && blamed {
            squirt(mob);
        }
        result
    }

    fn experience(mob: &mut FamilyMob<Self>) -> i32 {
        if mob.core.baby {
            0
        } else {
            1 + mob.core.random.next_int(3) as i32
        }
    }

    fn breathes_water(_mob: &FamilyMob<Self>) -> bool {
        true
    }

    fn breath(_mob: &FamilyMob<Self>) -> Breath {
        Breath::Gills
    }

    fn leashable(_mob: &FamilyMob<Self>) -> bool {
        true
    }

    fn voice(mob: &FamilyMob<Self>) -> (&'static str, f32, &'static str) {
        let family = if mob.state.glow {
            "glow_squid"
        } else {
            "squid"
        };
        (family, VOLUME, "neutral_volume")
    }

    fn emits_sounds(_kind: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests;
