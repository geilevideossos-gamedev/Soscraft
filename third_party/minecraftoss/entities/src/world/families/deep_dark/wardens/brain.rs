use super::anger::AngerLevel;
use super::{Pose, Warden};
use crate::brain_behaviors::{
    Living, MoveSink, Pos, Track, WalkGoal, Walker, block_of, find, shuffled,
};
use crate::monster_ai::Target;
use crate::villager_brain::Slot;
use crate::world::families::{FamilyMob, MobCore};
use crate::world::{Voice, voice_pitch};
use glam::DVec3;
use minecraftoss_player::World;

pub const EMERGE_DURATION: i64 = 134;
pub const DIGGING_DURATION: i64 = 100;
pub const ROAR_DURATION: i64 = 84;
pub const SNIFFING_DURATION: i64 = 84;
pub const DIGGING_COOLDOWN: i64 = 1200;
pub const DISTURBANCE_TICKS: i64 = 100;
pub const SNIFF_COOLDOWN_MIN: i64 = 100;
pub const SNIFF_COOLDOWN_MAX: i64 = 200;
pub const VIBRATION_COOLDOWN: i64 = 40;
pub const RECENT_PROJECTILE: i64 = 100;
pub const TOUCH_COOLDOWN: i64 = 20;
pub const SCAN_RATE: i32 = 20;
pub const BEHAVIOR_TICKS: i64 = 60;
pub const SWIM_CHANCE: f32 = 0.8;
pub const LOOK_MIN: i64 = 45;
pub const LOOK_MAX: i64 = 90;
pub const STROLL_SPEED: f32 = 0.5;
pub const INVESTIGATE_SPEED: f32 = 0.7;
pub const INVESTIGATE_CLOSE: i32 = 2;
pub const IDLE_MIN: i64 = 30;
pub const IDLE_MAX: i64 = 60;
pub const SNIFF_ANGER_XZ: f64 = 6.0;
pub const SNIFF_ANGER_Y: f64 = 20.0;
pub const DEFAULT_ANGER: i32 = 35;
pub const LISTENING_VOLUME: f32 = 10.0;
pub const BEHAVIOR_VOLUME: f32 = 5.0;
pub const FLUID_JUMP_THRESHOLD: f64 = 0.4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activity {
    Emerge,
    Dig,
    Roar,
    Fight,
    Investigate,
    Sniff,
    #[default]
    Idle,
}

#[derive(Clone, Debug, Default)]
pub struct Memories {
    pub dig_cooldown: Slot<()>,
    pub is_emerging: Slot<()>,
    pub is_sniffing: Slot<()>,
    pub sniff_cooldown: Slot<()>,
    pub roar_target: Slot<Target>,
    pub attack_target: Slot<Target>,
    pub disturbance_location: Slot<Pos>,
    pub recent_projectile: Slot<()>,
    pub vibration_cooldown: Slot<()>,
    pub touch_cooldown: Slot<()>,
    pub nearest_attackable: Slot<Target>,
    pub walk_target: Slot<WalkGoal>,
    pub look_target: Slot<Track>,
    pub cant_reach_walk_target_since: Slot<i64>,
}

impl Memories {
    fn tick(&mut self) {
        self.dig_cooldown.tick();
        self.is_emerging.tick();
        self.is_sniffing.tick();
        self.sniff_cooldown.tick();
        self.roar_target.tick();
        self.attack_target.tick();
        self.disturbance_location.tick();
        self.recent_projectile.tick();
        self.vibration_cooldown.tick();
        self.touch_cooldown.tick();
        self.nearest_attackable.tick();
        self.walk_target.tick();
        self.look_target.tick();
        self.cant_reach_walk_target_since.tick();
    }

    pub fn set_dig_cooldown(&mut self) {
        if self.dig_cooldown.present() {
            self.dig_cooldown.set_for((), DIGGING_COOLDOWN);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct WardenBrain {
    pub memories: Memories,
    pub activity: Activity,
    pub started: bool,
    pub scan: i32,
    pub swim_until: Option<i64>,
    pub look_until: Option<i64>,
    pub emerging_until: Option<i64>,
    pub digging_until: Option<i64>,
    pub sniffing_until: Option<i64>,
    pub idle_until: Option<i64>,
    pub walk: MoveSink,
}

impl WardenBrain {
    pub fn update_activity(&mut self) {
        let memories = &self.memories;
        let next = if memories.is_emerging.present() {
            Activity::Emerge
        } else if !memories.roar_target.present() && !memories.dig_cooldown.present() {
            Activity::Dig
        } else if memories.roar_target.present() {
            Activity::Roar
        } else if memories.attack_target.present() {
            Activity::Fight
        } else if memories.disturbance_location.present() {
            Activity::Investigate
        } else if memories.is_sniffing.present() {
            Activity::Sniff
        } else {
            Activity::Idle
        };
        if next == self.activity {
            return;
        }
        let memories = &mut self.memories;
        match self.activity {
            Activity::Emerge => memories.is_emerging.erase(),
            Activity::Roar => memories.roar_target.erase(),
            Activity::Fight => memories.attack_target.erase(),
            Activity::Investigate => memories.disturbance_location.erase(),
            Activity::Sniff => memories.is_sniffing.erase(),
            Activity::Dig | Activity::Idle => {}
        }
        self.activity = next;
    }
}

pub struct Ctx<'a> {
    pub others: &'a [Living],
    pub uuids: &'a [(Target, u128)],
    pub world: &'a dyn World,
    pub time: i64,
    pub passenger: bool,
    pub look_at: Option<DVec3>,
    pub jump: bool,
    pub unmount: bool,
}

impl Ctx<'_> {
    pub fn uuid_of(&self, target: Target) -> u128 {
        self.uuids
            .iter()
            .find(|(known, _)| *known == target)
            .map_or(0, |(_, uuid)| *uuid)
    }
}

pub fn can_target(living: &Living) -> bool {
    living.alive
        && !living.spectator
        && living.attackable
        && living.kind != super::KIND
        && living.kind != "minecraft:armor_stand"
}

pub fn targetable(others: &[Living], target: Target) -> bool {
    find(others, target).is_some_and(can_target)
}

fn play(core: &mut MobCore, event: &'static str, volume: f32, pitch: f32) {
    let at = core.body.position;
    core.voices.push((Voice::Event(event, volume, pitch), at));
}

pub fn active_anger(warden: &Warden) -> i32 {
    let target = warden.brain.memories.attack_target.get().copied();
    warden.anger.active_anger(target)
}

pub fn anger_level(warden: &Warden) -> AngerLevel {
    AngerLevel::by_anger(active_anger(warden))
}

pub fn angry_at(warden: &Warden, others: &[Living]) -> Option<Target> {
    if !anger_level(warden).is_angry() {
        return None;
    }
    warden.anger.top_suspect(|t| targetable(others, t))
}

pub fn set_pose(mob: &mut FamilyMob<Warden>, pose: Pose) {
    if mob.state.pose == pose {
        return;
    }
    mob.state.pose = pose;
    mob.state.pose_age = 0;
    super::fit(&mut mob.core, pose);
}

pub fn increase_anger_at(
    mob: &mut FamilyMob<Warden>,
    others: &[Living],
    target: Target,
    uuid: u128,
    amount: i32,
    listening: bool,
) {
    if mob.core.no_ai || !targetable(others, target) {
        return;
    }
    let memories = &mut mob.state.brain.memories;
    memories.set_dig_cooldown();
    let not_at_player = !matches!(memories.attack_target.get(), Some(Target::Player(_)));
    let anger = mob.state.anger.increase(target, uuid, amount);
    let player = matches!(target, Target::Player(_));
    if player && not_at_player && AngerLevel::by_anger(anger).is_angry() {
        mob.state.brain.memories.attack_target.erase();
    }
    if listening && mob.state.pose != Pose::Roaring {
        let sound = anger_level(&mob.state).listening_sound();
        let pitch = voice_pitch(&mut mob.core.random, false);
        play(&mut mob.core, sound, LISTENING_VOLUME, pitch);
    }
}

pub fn set_disturbance(mob: &mut FamilyMob<Warden>, others: &[Living], pos: Pos) {
    if angry_at(&mob.state, others).is_some() || mob.state.brain.memories.attack_target.present() {
        return;
    }
    let memories = &mut mob.state.brain.memories;
    memories.set_dig_cooldown();
    memories.sniff_cooldown.set_for((), DISTURBANCE_TICKS);
    memories
        .look_target
        .set_for(Track::Block(pos), DISTURBANCE_TICKS);
    memories
        .disturbance_location
        .set_for(pos, DISTURBANCE_TICKS);
    memories.walk_target.erase();
}

fn in_swimmable_fluid(mob: &FamilyMob<Warden>) -> bool {
    let fluid = mob.state.fluid;
    fluid.in_water() && fluid.water_height > FLUID_JUMP_THRESHOLD || fluid.in_lava()
}

pub fn think(mob: &mut FamilyMob<Warden>, ctx: &mut Ctx) {
    if !mob.state.brain.started {
        mob.state.brain.started = true;
        mob.state.brain.scan = mob.core.random.next_int(SCAN_RATE as u32) as i32;
    }
    mob.state.brain.memories.tick();
    sense(mob, ctx);
    start(mob, ctx);
    run(mob, ctx);
}

fn sense(mob: &mut FamilyMob<Warden>, ctx: &Ctx) {
    let brain = &mut mob.state.brain;
    brain.scan -= 1;
    if brain.scan > 0 {
        return;
    }
    brain.scan = SCAN_RATE;
    let body = &mob.core.body;
    let (p, half, range) = (
        body.position,
        f64::from(body.width) / 2.0,
        super::FOLLOW_RANGE,
    );
    let min = p - DVec3::new(half + range, range, half + range);
    let max = p + DVec3::new(half + range, f64::from(body.height) + range, half + range);
    let mut near: Vec<(&Living, f64)> = ctx
        .others
        .iter()
        .filter(|other| other.alive)
        .filter(|other| {
            let (q, h) = (other.position, f64::from(other.width) / 2.0);
            q.x - h < max.x
                && q.x + h > min.x
                && q.y < max.y
                && q.y + f64::from(other.height) > min.y
                && q.z - h < max.z
                && q.z + h > min.z
        })
        .map(|other| (other, other.position.distance_squared(p)))
        .collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1));
    let first = |player: bool| {
        near.iter()
            .map(|(other, _)| *other)
            .find(|other| other.is_player() == player && can_target(other))
            .map(|other| other.target)
    };
    let attackable = first(true).or_else(|| first(false));
    brain.memories.nearest_attackable.set_or_erase(attackable);
}

fn start(mob: &mut FamilyMob<Warden>, ctx: &mut Ctx) {
    let time = ctx.time;
    if mob.state.brain.swim_until.is_none() && in_swimmable_fluid(mob) {
        mob.state.brain.swim_until = Some(time + BEHAVIOR_TICKS);
    }
    let activity = mob.state.brain.activity;
    if activity == Activity::Dig && ctx.passenger {
        ctx.unmount = true;
    }
    let memories = &mut mob.state.brain.memories;
    if !memories.attack_target.present() {
        let roar = memories.roar_target.get().copied();
        let roar =
            roar.and_then(|target| find(ctx.others, target).map(|other| block_of(other.position)));
        if let Some(pos) = roar.or_else(|| memories.disturbance_location.get().copied()) {
            memories.look_target.set(Track::Block(pos));
        }
    }
    if activity == Activity::Dig {
        start_digging(mob, time);
    }
    let brain = &mut mob.state.brain;
    if brain.look_until.is_none() && brain.memories.look_target.present() {
        let span = (LOOK_MAX - LOOK_MIN + 1) as u32;
        brain.look_until = Some(time + LOOK_MIN + i64::from(mob.core.random.next_int(span)));
    }
    if !mob.state.brain.walk.running() {
        let FamilyMob { core, state } = &mut *mob;
        let super::Warden {
            brain,
            navigation,
            profile,
            fluid,
            ..
        } = state;
        let memories = &mut brain.memories;
        brain.walk.try_start(
            &mut memories.walk_target,
            &mut memories.cant_reach_walk_target_since,
            &mut Walker {
                body: &core.body,
                others: ctx.others,
                world: ctx.world,
                time,
                random: &mut core.random,
                navigation,
                profile,
                fluid: *fluid,
            },
        );
    }
    match activity {
        Activity::Emerge => start_emerging(mob, time),
        Activity::Investigate => go_to_disturbance(mob),
        Activity::Sniff => start_sniffing(mob, time),
        Activity::Idle => {
            try_to_sniff(mob);
            idle(mob, ctx);
        }
        Activity::Dig | Activity::Roar | Activity::Fight => {}
    }
}

fn start_digging(mob: &mut FamilyMob<Warden>, time: i64) {
    let memories = &mob.state.brain.memories;
    if mob.state.brain.digging_until.is_some()
        || memories.attack_target.present()
        || memories.walk_target.present()
    {
        return;
    }
    let in_liquid = mob.state.fluid.in_water() || mob.state.fluid.in_lava();
    if !mob.core.body.on_ground && !in_liquid {
        return;
    }
    mob.state.brain.digging_until = Some(time + DIGGING_DURATION);
    if mob.core.body.on_ground {
        set_pose(mob, Pose::Digging);
        play(&mut mob.core, "entity.warden.dig", BEHAVIOR_VOLUME, 1.0);
    } else {
        play(
            &mut mob.core,
            "entity.warden.agitated",
            BEHAVIOR_VOLUME,
            1.0,
        );
        mob.state.brain.digging_until = None;
        mob.core.removed = true;
    }
}

fn start_emerging(mob: &mut FamilyMob<Warden>, time: i64) {
    let memories = &mob.state.brain.memories;
    if mob.state.brain.emerging_until.is_some()
        || !memories.is_emerging.present()
        || memories.walk_target.present()
    {
        return;
    }
    mob.state.brain.emerging_until = Some(time + EMERGE_DURATION);
    set_pose(mob, Pose::Emerging);
    play(&mut mob.core, "entity.warden.emerge", BEHAVIOR_VOLUME, 1.0);
}

fn go_to_disturbance(mob: &mut FamilyMob<Warden>) {
    let memories = &mob.state.brain.memories;
    if memories.attack_target.present() || memories.walk_target.present() {
        return;
    }
    let Some(&target) = memories.disturbance_location.get() else {
        return;
    };
    let at = block_of(mob.core.body.position);
    if crate::brain_behaviors::closer_than(at, target, f64::from(INVESTIGATE_CLOSE)) {
        return;
    }
    let random = &mut mob.core.random;
    let dx = random.next_int(3) as i32 - 1;
    let dz = random.next_int(3) as i32 - 1;
    let pos = (target.0 + dx, target.1, target.2 + dz);
    let memories = &mut mob.state.brain.memories;
    memories.look_target.set(Track::Block(pos));
    memories.walk_target.set(WalkGoal {
        target: Track::Block(pos),
        speed: INVESTIGATE_SPEED,
        close_enough: INVESTIGATE_CLOSE,
    });
}

fn start_sniffing(mob: &mut FamilyMob<Warden>, time: i64) {
    let memories = &mob.state.brain.memories;
    if mob.state.brain.sniffing_until.is_some()
        || !memories.is_sniffing.present()
        || memories.attack_target.present()
        || memories.walk_target.present()
    {
        return;
    }
    mob.state.brain.sniffing_until = Some(time + SNIFFING_DURATION);
    play(&mut mob.core, "entity.warden.sniff", BEHAVIOR_VOLUME, 1.0);
}

fn try_to_sniff(mob: &mut FamilyMob<Warden>) {
    let memories = &mob.state.brain.memories;
    if memories.sniff_cooldown.present()
        || !memories.nearest_attackable.present()
        || memories.disturbance_location.present()
    {
        return;
    }
    let span = (SNIFF_COOLDOWN_MAX - SNIFF_COOLDOWN_MIN + 1) as u32;
    let cooldown = SNIFF_COOLDOWN_MIN + i64::from(mob.core.random.next_int(span));
    let memories = &mut mob.state.brain.memories;
    memories.is_sniffing.set(());
    memories.sniff_cooldown.set_for((), cooldown);
    memories.walk_target.erase();
    set_pose(mob, Pose::Sniffing);
}

fn idle(mob: &mut FamilyMob<Warden>, ctx: &Ctx) {
    let brain = &mob.state.brain;
    if brain.idle_until.is_some() || brain.memories.is_sniffing.present() {
        return;
    }
    for choice in shuffled(&mut mob.core.random, &[2, 1]) {
        if choice == 0 {
            if mob.state.brain.memories.walk_target.present() {
                continue;
            }
            let spot = crate::stroll::land_random_position(
                ctx.world,
                &mob.state.profile,
                mob.core.body.position,
                10,
                7,
                &mut mob.core.random,
            );
            let goal = spot.map(|spot| WalkGoal {
                target: Track::Block(block_of(spot)),
                speed: STROLL_SPEED,
                close_enough: 0,
            });
            mob.state.brain.memories.walk_target.set_or_erase(goal);
            return;
        }
        let span = (IDLE_MAX - IDLE_MIN + 1) as u32;
        let wait = IDLE_MIN + i64::from(mob.core.random.next_int(span));
        mob.state.brain.idle_until = Some(ctx.time + wait);
        return;
    }
}

fn run(mob: &mut FamilyMob<Warden>, ctx: &mut Ctx) {
    let time = ctx.time;
    if let Some(until) = mob.state.brain.swim_until {
        if time > until || !in_swimmable_fluid(mob) {
            mob.state.brain.swim_until = None;
        } else if mob.core.random.next_float() < SWIM_CHANCE {
            ctx.jump = true;
        }
    }
    if let Some(until) = mob.state.brain.digging_until {
        if time > until || mob.core.removed {
            mob.state.brain.digging_until = None;
            mob.core.removed = true;
        }
    }
    if let Some(until) = mob.state.brain.look_until {
        let memories = &mut mob.state.brain.memories;
        let at = memories
            .look_target
            .get()
            .and_then(|track| track.position(ctx.others));
        match at {
            Some(at) if time <= until => ctx.look_at = Some(at),
            _ => {
                mob.state.brain.look_until = None;
                memories.look_target.erase();
            }
        }
    }
    {
        let FamilyMob { core, state } = &mut *mob;
        let super::Warden {
            brain,
            navigation,
            profile,
            fluid,
            ..
        } = state;
        let memories = &mut brain.memories;
        brain.walk.tick(
            &mut memories.walk_target,
            &mut memories.cant_reach_walk_target_since,
            &mut Walker {
                body: &core.body,
                others: ctx.others,
                world: ctx.world,
                time,
                random: &mut core.random,
                navigation,
                profile,
                fluid: *fluid,
            },
        );
    }
    if mob
        .state
        .brain
        .emerging_until
        .is_some_and(|until| time > until)
    {
        mob.state.brain.emerging_until = None;
        if mob.state.pose == Pose::Emerging {
            set_pose(mob, Pose::Standing);
        }
    }
    if mob
        .state
        .brain
        .sniffing_until
        .is_some_and(|until| time > until)
    {
        mob.state.brain.sniffing_until = None;
        stop_sniffing(mob, ctx);
    }
    if mob.state.brain.idle_until.is_some_and(|until| time > until) {
        mob.state.brain.idle_until = None;
    }
}

fn stop_sniffing(mob: &mut FamilyMob<Warden>, ctx: &Ctx) {
    if mob.state.pose == Pose::Sniffing {
        set_pose(mob, Pose::Standing);
    }
    mob.state.brain.memories.is_sniffing.erase();
    let nearest = mob.state.brain.memories.nearest_attackable.get().copied();
    let Some(other) = nearest
        .and_then(|target| find(ctx.others, target))
        .filter(|other| can_target(other))
        .copied()
    else {
        return;
    };
    let offset = other.position - mob.core.body.position;
    let close = offset.x * offset.x + offset.z * offset.z < SNIFF_ANGER_XZ * SNIFF_ANGER_XZ
        && offset.y * offset.y < SNIFF_ANGER_Y * SNIFF_ANGER_Y;
    if close {
        let uuid = ctx.uuid_of(other.target);
        increase_anger_at(mob, ctx.others, other.target, uuid, DEFAULT_ANGER, true);
    }
    if !mob.state.brain.memories.disturbance_location.present() {
        set_disturbance(mob, ctx.others, block_of(other.position));
    }
}
