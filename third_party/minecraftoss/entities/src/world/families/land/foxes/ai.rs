use super::{FoxVariant, MAX_CROUCH_AMOUNT};
use crate::control::{MoveFrame, obstacle_top};
use crate::fluid::FluidFrame;
use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{
    CheckedMeleeAttackGoal, LeapGoal, LookAtPlayerGoal, MeleeAttackGoal, MonsterAi,
    MonsterGoalContext, NearestTargetGoal, Prey, StrollGoal, Target, TargetInfo,
};
use crate::movement::Body;
use crate::navigation::{
    PlannedPath, ground_view, navigate_walk_to, navigate_walk_to_entity, plan_walk_path,
};
use crate::stroll::{default_random_position_away, land_random_position_by};
use crate::tempt::PlayerCandidate;
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::emissions::play_movement;
use crate::world::families::MonsterSpec;
use crate::world::families::goals::{
    ClimbOnTopOfPowderSnowGoal, in_powder_snow, reduced_tick_delay,
};
use crate::world::families::land::animal_goals::{
    AnimalExtra, AnimalGoals, FollowParentGoal, PanicGoal,
};
use crate::world::living::landed_block;
use crate::world::{MonsterStep, Voice};
use glam::DVec3;
use minecraftoss_player::path_type::PathType;
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Pos, World, jmath};

pub const SITTING: u8 = 1;
pub const CROUCHING: u8 = 4;
pub const INTERESTED: u8 = 8;
pub const POUNCING: u8 = 16;
pub const SLEEPING: u8 = 32;
pub const FACEPLANTED: u8 = 64;
pub const DEFENDING: u8 = 128;
pub const FLOAT_DEPTH: f64 = 0.25;
pub const FLOAT_CHANCE: f32 = 0.8;
pub const PANIC_SPEED: f64 = 2.2;
pub const AVOID_PLAYER: f64 = 16.0;
pub const AVOID_WOLF: f64 = 8.0;
pub const AVOID_POLAR_BEAR: f64 = 8.0;
pub const AVOID_WALK: f64 = 1.6;
pub const AVOID_SPRINT: f64 = 1.4;
pub const AVOID_SPRINT_DISTANCE: f64 = 7.0;
pub const AVOID_HORIZONTAL: u32 = 16;
pub const AVOID_VERTICAL: u32 = 7;
pub const AVOID_HEIGHT: f64 = 3.0;
pub const SHELTER_SPEED: f64 = 1.25;
pub const SHELTER_INTERVAL: u32 = 100;
pub const SHELTER_TRIES: usize = 10;
pub const SLEEP_WAIT: u32 = 140;
pub const FOLLOW_PARENT_SPEED: f64 = 1.25;
pub const VILLAGE_INTERVAL: u32 = 200;
pub const VILLAGE_SECTIONS: i32 = 6;
pub const VILLAGE_HORIZONTAL: u32 = 15;
pub const VILLAGE_VERTICAL: u32 = 7;
pub const VILLAGE_STEP: f64 = 10.0;
pub const VILLAGE_PULL: f64 = 0.4;
pub const VILLAGE_WANDER: u32 = 16;
pub const STROLL_SPEED: f64 = 1.0;
pub const STROLL_PROBABILITY: f32 = 0.001;
pub const LOOK_RANGE: f64 = 24.0;
pub const PERCH_CHANCE: f32 = 0.02;
pub const PERCH_LOOKS: i32 = 2;
pub const PERCH_EXTRA_LOOKS: u32 = 3;
pub const PERCH_LOOK_TICKS: u32 = 80;
pub const PERCH_LOOK_SPREAD: u32 = 20;
pub const MAX_HEAD_Y_ROT: f32 = 75.0;
pub const MAX_HEAD_X_ROT: f32 = 40.0;
pub const MAX_PATH_NODES: i32 = 512;
pub const SAFE_FALL_BONUS: f64 = 2.0;
pub const STALK_DISTANCE: f64 = 6.0;
pub const STALK_SPEED: f64 = 1.5;
pub const POUNCE_PUSH: f64 = 0.8;
pub const POUNCE_LIFT: f64 = 0.9;
pub const POUNCE_LOOK_Y: f32 = 60.0;
pub const POUNCE_LOOK_X: f32 = 30.0;
pub const POUNCE_REACH: f32 = 2.0;
pub const POUNCE_RISE: f32 = 6.5;
pub const POUNCE_EASE: f32 = 0.2;
pub const POUNCE_SETTLE: f64 = 0.03;
pub const POUNCE_LANDED: f64 = 0.05;
pub const POUNCE_LEVEL: f32 = 15.0;
pub const FACEPLANT_PITCH: f32 = 60.0;
pub const FACEPLANT_TICKS: u32 = 40;
pub const PATH_STEPS: i32 = 6;
pub const PATH_HEIGHT: i32 = 3;
pub const MELEE_SPEED: f64 = 1.2000000476837158;
pub const LEAP_LIFT: f32 = 0.4;
pub const LAND_INTERVAL: u32 = 10;
pub const FISH_INTERVAL: u32 = 20;
pub const PLAYER: &str = "minecraft:player";
pub const WOLF: &str = "minecraft:wolf";
pub const POLAR_BEAR: &str = "minecraft:polar_bear";
pub const GRASS_BLOCK: &str = "minecraft:grass_block";
pub const SNOW_LAYER: &str = "minecraft:snow";
pub const REPLACEABLE: &str = "minecraft:replaceable";
pub const LAND_PREY: [&str; 2] = ["minecraft:chicken", "minecraft:rabbit"];
pub const FISH_PREY: [&str; 3] = [
    "minecraft:cod",
    "minecraft:salmon",
    "minecraft:tropical_fish",
];

pub const GOAL_NAMES: [&str; 18] = [
    "FoxFloatGoal",
    "ClimbOnTopOfPowderSnowGoal",
    "FaceplantGoal",
    "FoxPanicGoal",
    "AvoidEntityGoal",
    "AvoidEntityGoal",
    "AvoidEntityGoal",
    "StalkPreyGoal",
    "FoxPounceGoal",
    "SeekShelterGoal",
    "FoxMeleeAttackGoal",
    "SleepGoal",
    "FoxFollowParentGoal",
    "FoxStrollThroughVillageGoal",
    "LeapAtTargetGoal",
    "WaterAvoidingRandomStrollGoal",
    "FoxLookAtPlayerGoal",
    "PerchAndSearchGoal",
];
pub const TARGET_NAMES: [&str; 6] = ["NearestAttackableTargetGoal"; 6];

pub(super) static SPEC: MonsterSpec = MonsterSpec {
    goal_names: &GOAL_NAMES,
    target_names: &TARGET_NAMES,
    goals,
    targets,
    eye_height: super::EYE_HEIGHT,
    follow_range: super::FOLLOW_RANGE,
    walk,
};

#[derive(Clone, Debug, Default)]
pub struct FoxGoals {
    pub animal: AnimalGoals,
    pub flags: u8,
    pub baby: bool,
    pub alertable: bool,
    pub moving: bool,
    pub halt: bool,
    pub tame: Vec<u64>,
    pub unwary: Vec<u64>,
    pub hunts: Option<FoxVariant>,
    pub jumping: bool,
    pub crouch_amount: f32,
    pub uncrouched: bool,
    pub bit: bool,
}

impl FoxGoals {
    pub fn flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    pub fn set(&mut self, flag: u8, on: bool) {
        if on {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    pub fn clear_states(&mut self) {
        for flag in [
            INTERESTED,
            CROUCHING,
            SITTING,
            SLEEPING,
            DEFENDING,
            FACEPLANTED,
        ] {
            self.set(flag, false);
        }
    }

    pub fn can_move(&self) -> bool {
        !(self.flag(SLEEPING) || self.flag(SITTING) || self.flag(FACEPLANTED))
    }

    pub fn keeps_pitch(&self) -> bool {
        self.flag(POUNCING)
            || self.flag(CROUCHING)
            || self.flag(INTERESTED)
            || self.flag(FACEPLANTED)
    }
}

impl AnimalExtra for FoxGoals {
    fn animal(&self) -> &AnimalGoals {
        &self.animal
    }

    fn animal_mut(&mut self) -> &mut AnimalGoals {
        &mut self.animal
    }
}

fn goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(0, FoxFloatGoal);
    selector.add(0, ClimbOnTopOfPowderSnowGoal);
    selector.add(1, FaceplantGoal::default());
    selector.add(
        2,
        FoxPanicGoal(PanicGoal::new(PANIC_SPEED, crate::feline_ai::panic_cause)),
    );
    selector.add(4, FoxAvoidGoal::new(PLAYER, AVOID_PLAYER));
    selector.add(4, FoxAvoidGoal::new(WOLF, AVOID_WOLF));
    selector.add(4, FoxAvoidGoal::new(POLAR_BEAR, AVOID_POLAR_BEAR));
    selector.add(5, StalkPreyGoal::default());
    selector.add(6, FoxPounceGoal);
    selector.add(6, SeekShelterGoal::default());
    let melee = MeleeAttackGoal {
        hits: true,
        raises_arms: false,
        follow_unseen: true,
        gives_up_in_light: false,
        speed: MELEE_SPEED,
    };
    selector.add(7, FoxMeleeAttackGoal(melee.checked(bite)));
    selector.add(7, SleepGoal::default());
    selector.add(
        8,
        FoxFollowParentGoal(FollowParentGoal::new(FOLLOW_PARENT_SPEED)),
    );
    selector.add(9, FoxStrollThroughVillageGoal::default());
    selector.add(10, LeapGoal::new(LEAP_LIFT));
    selector.add(11, StrollGoal::new(STROLL_SPEED, STROLL_PROBABILITY));
    selector.add(
        12,
        FoxLookAtPlayerGoal(LookAtPlayerGoal { range: LOOK_RANGE }),
    );
    selector.add(13, PerchAndSearchGoal::default());
    selector
}

fn targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    let (red, snow) = (FoxVariant::Red, FoxVariant::Snow);
    let land = Prey::Kinds(&LAND_PREY);
    let fish = Prey::Kinds(&FISH_PREY);
    selector.add(4, FoxPreyGoal::new(red, land, LAND_INTERVAL));
    selector.add(4, FoxPreyGoal::new(red, Prey::Absent, LAND_INTERVAL));
    selector.add(6, FoxPreyGoal::new(red, fish, FISH_INTERVAL));
    selector.add(4, FoxPreyGoal::new(snow, fish, FISH_INTERVAL));
    selector.add(6, FoxPreyGoal::new(snow, land, LAND_INTERVAL));
    selector.add(6, FoxPreyGoal::new(snow, Prey::Absent, LAND_INTERVAL));
    selector
}

fn walk(walk: &mut WalkProfile) {
    walk.set_malus(PathType::FireInNeighbor, 16.0);
    walk.set_malus(PathType::Fire, -1.0);
    walk.set_malus(PathType::DamagingInNeighbor, 0.0);
    walk.set_malus(PathType::Damaging, 0.0);
    walk.can_float = true;
    walk.walk_target = WalkTarget::Animal;
    walk.max_visited = Some(MAX_PATH_NODES);
}

#[derive(Clone)]
struct FoxPreyGoal {
    variant: FoxVariant,
    goal: NearestTargetGoal,
}

impl FoxPreyGoal {
    fn new(variant: FoxVariant, prey: Prey, interval: u32) -> Self {
        let goal = NearestTargetGoal::every(prey, false, reduced_tick_delay(interval));
        Self { variant, goal }
    }
}

impl Goal<MonsterGoalContext> for FoxPreyGoal {
    fn controls(&self) -> Controls {
        self.goal.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let hunts = fox(ctx).and_then(|goals| goals.hunts);
        hunts == Some(self.variant) && self.goal.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.goal.can_continue(ctx, world)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.goal.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.goal.stop(ctx);
    }
}

pub(super) fn fox(ctx: &MonsterGoalContext) -> Option<&FoxGoals> {
    ctx.extra.get::<FoxGoals>()
}

fn fox_mut(ctx: &mut MonsterGoalContext) -> Option<&mut FoxGoals> {
    ctx.extra.get_mut::<FoxGoals>()
}

fn flag(ctx: &MonsterGoalContext, flag: u8) -> bool {
    fox(ctx).is_some_and(|goals| goals.flag(flag))
}

fn clear_states(ctx: &mut MonsterGoalContext) {
    if let Some(goals) = fox_mut(ctx) {
        goals.clear_states();
    }
}

pub fn held_still(ctx: &MonsterGoalContext) -> bool {
    fox(ctx).is_some_and(|goals| !goals.can_move())
}

pub(super) fn block_of(position: DVec3) -> Pos {
    (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

fn section(pos: Pos) -> Pos {
    (pos.0 >> 4, pos.1 >> 4, pos.2 >> 4)
}

fn bottom_centre(pos: Pos) -> DVec3 {
    DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1),
        f64::from(pos.2) + 0.5,
    )
}

pub fn walk_value(world: &dyn World, pos: Pos) -> f32 {
    let below = (pos.0, pos.1 - 1, pos.2);
    if world
        .block(below)
        .is_some_and(|block| block.id == GRASS_BLOCK)
    {
        10.0
    } else {
        world.light_path_cost(pos)
    }
}

pub fn has_shelter(world: &dyn World, body: &Body) -> bool {
    let p = body.position;
    let top = block_of(DVec3::new(p.x, p.y + f64::from(body.height), p.z));
    !world.can_see_sky(top) && walk_value(world, top) >= 0.0
}

fn is_village(ctx: &mut MonsterGoalContext, pos: Pos) -> bool {
    ctx.pois.as_mut().is_some_and(|pois| pois.is_village(pos))
}

fn held_target(ctx: &MonsterGoalContext) -> Option<TargetInfo> {
    ctx.target.and_then(|target| ctx.info(target))
}

fn live_target(ctx: &MonsterGoalContext) -> Option<TargetInfo> {
    held_target(ctx).filter(|target| target.alive)
}

fn eye_of(target: TargetInfo) -> DVec3 {
    target.position + DVec3::Y * f64::from(target.eye_height)
}

fn replaceable(world: &dyn World, pos: Pos) -> bool {
    world
        .block(pos)
        .is_none_or(|_| world.block_in_tag(pos, REPLACEABLE))
}

pub fn is_path_clear(world: &dyn World, from: DVec3, to: DVec3) -> bool {
    let dz = to.z - from.z;
    let dx = to.x - from.x;
    let slope = dz / dx;
    for step in 0..PATH_STEPS {
        let part = f64::from(step as f32 / PATH_STEPS as f32);
        let z = if slope == 0.0 { 0.0 } else { dz * part };
        let x = if slope == 0.0 { dx * part } else { z / slope };
        for up in 1..=PATH_HEIGHT {
            let at = DVec3::new(from.x + x, from.y + f64::from(up), from.z + z);
            if !replaceable(world, block_of(at)) {
                return false;
            }
        }
    }
    true
}

fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn rot_lerp(delta: f32, from: f32, to: f32) -> f32 {
    from + delta * wrap_degrees(to - from)
}

fn signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

fn distance_to(from: DVec3, to: DVec3) -> f32 {
    let x = (from.x - to.x) as f32;
    let y = (from.y - to.y) as f32;
    let z = (from.z - to.z) as f32;
    (x * x + y * y + z * z).sqrt()
}

fn normalize(vector: DVec3) -> DVec3 {
    let length = vector.length();
    if length < f64::from(1.0e-5_f32) {
        DVec3::ZERO
    } else {
        DVec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}

#[derive(Clone)]
struct FoxFloatGoal;

impl Goal<MonsterGoalContext> for FoxFloatGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Jump])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.fluid.floatable_height() > FLOAT_DEPTH || ctx.fluid.in_lava()
    }

    fn every_tick(&self) -> bool {
        true
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        clear_states(ctx);
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        if ctx.random.next_float() < FLOAT_CHANCE {
            ctx.jump = true;
        }
    }
}

#[derive(Clone, Default)]
struct FaceplantGoal {
    countdown: i32,
}

impl Goal<MonsterGoalContext> for FaceplantGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Look, Control::Jump, Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        flag(ctx, FACEPLANTED)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        flag(ctx, FACEPLANTED) && self.countdown > 0
    }

    fn start(&mut self, _ctx: &mut MonsterGoalContext) {
        self.countdown = reduced_tick_delay(FACEPLANT_TICKS) as i32;
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = fox_mut(ctx) {
            goals.set(FACEPLANTED, false);
        }
    }

    fn tick(&mut self, _ctx: &mut MonsterGoalContext) {
        self.countdown -= 1;
    }
}

#[derive(Clone)]
struct FoxPanicGoal(PanicGoal<FoxGoals>);

impl Goal<MonsterGoalContext> for FoxPanicGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        !flag(ctx, DEFENDING) && self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world)
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.0.start_world(ctx, world);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }
}

#[derive(Clone)]
struct FoxAvoidGoal {
    kind: &'static str,
    max_dist: f64,
    to_avoid: Option<Target>,
    path: Option<PlannedPath>,
}

impl FoxAvoidGoal {
    fn new(kind: &'static str, max_dist: f64) -> Self {
        Self {
            kind,
            max_dist,
            to_avoid: None,
            path: None,
        }
    }

    fn candidates(&self, ctx: &MonsterGoalContext) -> Vec<(Target, DVec3, f32, f32)> {
        let Some(goals) = fox(ctx) else {
            return Vec::new();
        };
        if self.kind == PLAYER {
            let players = ctx.players.iter();
            let wary = players.filter(|player| !goals.unwary.contains(&player.id));
            return wary
                .map(|player| (Target::Player(player.id), player.position, 0.6, 1.8))
                .collect();
        }
        let mobs = ctx.mobs.iter().filter(|mob| mob.kind == self.kind);
        mobs.filter(|mob| !goals.tame.contains(&mob.id))
            .map(|mob| (Target::Mob(mob.id), mob.position, mob.width, mob.height))
            .collect()
    }
}

impl Goal<MonsterGoalContext> for FoxAvoidGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if flag(ctx, DEFENDING) {
            return false;
        }
        let body = ctx.body.clone();
        let half = f64::from(body.width / 2.0);
        let p = body.position;
        let grown = DVec3::new(half + self.max_dist, AVOID_HEIGHT, half + self.max_dist);
        let min = p - grown;
        let max = p + grown + DVec3::Y * f64::from(body.height);
        let range = self.max_dist.max(2.0);
        let mut best: Option<(f64, TargetInfo)> = None;
        for (target, q, width, height) in self.candidates(ctx) {
            let half = f64::from(width / 2.0);
            let inside = q.x - half < max.x
                && q.x + half > min.x
                && q.y < max.y
                && q.y + f64::from(height) > min.y
                && q.z - half < max.z
                && q.z + half > min.z;
            let Some(info) = ctx.info(target).filter(|_| inside) else {
                continue;
            };
            let distance = p.distance_squared(info.position);
            if !ctx.can_attack(info) || distance > range * range || !ctx.sees(world, info) {
                continue;
            }
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, info));
            }
        }
        let Some((_, avoid)) = best else {
            return false;
        };
        let profile = ctx.profile();
        let away = default_random_position_away(
            world,
            &profile,
            p,
            AVOID_HORIZONTAL,
            AVOID_VERTICAL,
            avoid.position,
            &mut ctx.random,
        );
        let Some(pos) = away else {
            return false;
        };
        if avoid.position.distance_squared(pos) < avoid.position.distance_squared(p) {
            return false;
        }
        self.path = plan_walk_path(
            &body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            pos,
            0,
        );
        self.to_avoid = Some(avoid.target);
        self.path.is_some()
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let position = ctx.body.position;
        ctx.navigation
            .move_to_in(world, self.path.take(), AVOID_WALK, position);
    }

    fn stop(&mut self, _ctx: &mut MonsterGoalContext) {
        self.to_avoid = None;
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        let Some(avoid) = self.to_avoid.and_then(|target| ctx.info(target)) else {
            return;
        };
        let near = AVOID_SPRINT_DISTANCE * AVOID_SPRINT_DISTANCE;
        ctx.navigation.speed_modifier = if ctx.body.position.distance_squared(avoid.position) < near
        {
            AVOID_SPRINT
        } else {
            AVOID_WALK
        };
    }
}

fn stalkable(ctx: &MonsterGoalContext, target: TargetInfo) -> bool {
    ctx.kind_of(target.target)
        .is_some_and(|kind| LAND_PREY.contains(&kind))
}

fn crouch(ctx: &mut MonsterGoalContext, on: bool) {
    if let Some(goals) = fox_mut(ctx) {
        goals.set(INTERESTED, on);
        goals.set(CROUCHING, on);
    }
}

#[derive(Clone, Default)]
struct StalkPreyGoal {
    clear: bool,
}

impl Goal<MonsterGoalContext> for StalkPreyGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let Some(goals) = fox(ctx) else {
            return false;
        };
        if goals.flag(SLEEPING) {
            return false;
        }
        let Some(target) = live_target(ctx) else {
            return false;
        };
        let far =
            ctx.body.position.distance_squared(target.position) > STALK_DISTANCE * STALK_DISTANCE;
        stalkable(ctx, target)
            && far
            && !goals.flag(CROUCHING)
            && !goals.flag(INTERESTED)
            && !goals.jumping
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let from = ctx.body.position;
        self.clear =
            held_target(ctx).is_some_and(|target| is_path_clear(world, from, target.position));
        self.can_start(ctx, world)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = fox_mut(ctx) {
            goals.set(SITTING, false);
            goals.set(FACEPLANTED, false);
        }
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        let Some(target) = held_target(ctx).filter(|_| self.clear) else {
            crouch(ctx, false);
            return;
        };
        crouch(ctx, true);
        ctx.navigation.stop();
        ctx.look_control
            .set_look_at_with_limits(eye_of(target), MAX_HEAD_Y_ROT, MAX_HEAD_X_ROT);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(target) = held_target(ctx) else {
            return;
        };
        ctx.look_control
            .set_look_at_with_limits(eye_of(target), MAX_HEAD_Y_ROT, MAX_HEAD_X_ROT);
        let near = STALK_DISTANCE * STALK_DISTANCE;
        if ctx.body.position.distance_squared(target.position) <= near {
            crouch(ctx, true);
            ctx.navigation.stop();
            return;
        }
        let profile = ctx.profile();
        navigate_walk_to_entity(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            target.position,
            STALK_SPEED,
            1,
        );
    }
}

#[derive(Clone)]
struct FoxPounceGoal;

impl Goal<MonsterGoalContext> for FoxPounceGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Jump])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if fox(ctx).is_none_or(|goals| goals.crouch_amount != MAX_CROUCH_AMOUNT) {
            return false;
        }
        let Some(target) = live_target(ctx) else {
            return false;
        };
        let clear = is_path_clear(world, ctx.body.position, target.position);
        if !clear {
            crouch(ctx, false);
        }
        clear
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        if live_target(ctx).is_none() {
            return false;
        }
        let vy = ctx.body.velocity.y;
        let level = ctx.look_control.pitch.abs() < POUNCE_LEVEL;
        let landed = vy * vy < POUNCE_LANDED && level && ctx.body.on_ground;
        !(landed || flag(ctx, FACEPLANTED))
    }

    fn interruptible(&self) -> bool {
        false
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = fox_mut(ctx) {
            goals.jumping = true;
            goals.set(POUNCING, true);
            goals.set(INTERESTED, false);
        }
        if let Some(target) = held_target(ctx) {
            ctx.look_control
                .set_look_at_with_limits(eye_of(target), POUNCE_LOOK_Y, POUNCE_LOOK_X);
            let toward = normalize(target.position - ctx.body.position);
            let push = DVec3::new(toward.x * POUNCE_PUSH, POUNCE_LIFT, toward.z * POUNCE_PUSH);
            let velocity = ctx.body.velocity + push;
            ctx.body.velocity = velocity;
            ctx.leap = Some(velocity);
        }
        ctx.navigation.stop();
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = fox_mut(ctx) {
            goals.set(CROUCHING, false);
            goals.crouch_amount = 0.0;
            goals.uncrouched = true;
            goals.set(INTERESTED, false);
            goals.set(POUNCING, false);
        }
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let target = held_target(ctx);
        if let Some(target) = target {
            ctx.look_control
                .set_look_at_with_limits(eye_of(target), POUNCE_LOOK_Y, POUNCE_LOOK_X);
        }
        let velocity = ctx.body.velocity;
        if !flag(ctx, FACEPLANTED) {
            let pitch = ctx.look_control.pitch;
            if velocity.y * velocity.y < POUNCE_SETTLE && pitch != 0.0 {
                ctx.look_control.pitch = rot_lerp(POUNCE_EASE, pitch, 0.0);
            } else {
                let horizontal = (velocity.x * velocity.x + velocity.z * velocity.z).sqrt();
                let jumping = fox(ctx).is_some_and(|goals| goals.jumping);
                let rise = if jumping && velocity.y > 0.0 {
                    POUNCE_RISE
                } else {
                    1.0
                };
                let vy = velocity.y * f64::from(rise);
                let length = (horizontal * horizontal + vy * vy).sqrt();
                if length > f64::from(1.0e-5_f32) {
                    let angle = signum(-vy) * (horizontal / length).acos() * 57.2957763671875;
                    ctx.look_control.pitch = angle as f32;
                }
            }
        }
        let p = ctx.body.position;
        if let Some(target) = target.filter(|t| distance_to(p, t.position) <= POUNCE_REACH) {
            ctx.attack = Some(target.target);
            return;
        }
        let falling = velocity.y as f32 != 0.0;
        if ctx.look_control.pitch > 0.0 && ctx.body.on_ground && falling {
            let snow = world
                .block(block_of(p))
                .is_some_and(|block| block.id == SNOW_LAYER);
            if snow {
                ctx.look_control.pitch = FACEPLANT_PITCH;
                ctx.clear_target();
                if let Some(goals) = fox_mut(ctx) {
                    goals.set(DEFENDING, false);
                    goals.set(FACEPLANTED, true);
                }
            }
        }
    }
}

#[derive(Clone)]
struct SeekShelterGoal {
    interval: i32,
    wanted: Option<DVec3>,
}

impl Default for SeekShelterGoal {
    fn default() -> Self {
        Self {
            interval: reduced_tick_delay(SHELTER_INTERVAL) as i32,
            wanted: None,
        }
    }
}

impl SeekShelterGoal {
    fn set_wanted_pos(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let feet = block_of(ctx.body.position);
        self.wanted = None;
        for _ in 0..SHELTER_TRIES {
            let x = feet.0 + ctx.random.next_int(20) as i32 - 10;
            let y = feet.1 + ctx.random.next_int(6) as i32 - 3;
            let z = feet.2 + ctx.random.next_int(20) as i32 - 10;
            let pos = (x, y, z);
            if !world.can_see_sky(pos) && walk_value(world, pos) < 0.0 {
                self.wanted = Some(bottom_centre(pos));
                break;
            }
        }
        self.wanted.is_some()
    }
}

impl Goal<MonsterGoalContext> for SeekShelterGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if flag(ctx, SLEEPING) || ctx.target.is_some() {
            return false;
        }
        let feet = block_of(ctx.body.position);
        if world.thundering() && world.can_see_sky(feet) {
            return self.set_wanted_pos(ctx, world);
        }
        if self.interval > 0 {
            self.interval -= 1;
            return false;
        }
        self.interval = SHELTER_INTERVAL as i32;
        ctx.bright_outside
            && world.can_see_sky(feet)
            && !is_village(ctx, feet)
            && self.set_wanted_pos(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        clear_states(ctx);
        let Some(target) = self.wanted else {
            return;
        };
        let profile = ctx.profile();
        let _ = navigate_walk_to(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            target,
            SHELTER_SPEED,
            1,
        );
    }
}

fn bite(ctx: &mut MonsterGoalContext, world: &dyn World, target: TargetInfo) {
    let time = ctx.melee.is_time_to_attack();
    if time && ctx.within_melee_range(target) && ctx.sees(world, target) {
        ctx.melee.reset_attack_cooldown();
        ctx.attack = Some(target.target);
        if let Some(goals) = fox_mut(ctx) {
            goals.bit = true;
        }
    }
}

#[derive(Clone)]
struct FoxMeleeAttackGoal(CheckedMeleeAttackGoal);

impl Goal<MonsterGoalContext> for FoxMeleeAttackGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let ready = fox(ctx).is_some_and(|goals| {
            !(goals.flag(SITTING)
                || goals.flag(SLEEPING)
                || goals.flag(CROUCHING)
                || goals.flag(FACEPLANTED))
        });
        ready && self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world)
    }

    fn interruptible(&self) -> bool {
        self.0.interruptible()
    }

    fn every_tick(&self) -> bool {
        self.0.every_tick()
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = fox_mut(ctx) {
            goals.set(INTERESTED, false);
        }
        self.0.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.0.tick_world(ctx, world);
    }
}

#[derive(Clone, Default)]
struct SleepGoal {
    countdown: Option<i32>,
}

impl SleepGoal {
    fn can_sleep(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let wait = reduced_tick_delay(SLEEP_WAIT);
        let countdown = self
            .countdown
            .get_or_insert_with(|| ctx.random.next_int(wait) as i32);
        if *countdown > 0 {
            *countdown -= 1;
            return false;
        }
        let body = &ctx.body;
        let snowed = in_powder_snow(world, body.position, body.width, body.height);
        ctx.bright_outside
            && has_shelter(world, body)
            && !fox(ctx).is_some_and(|goals| goals.alertable)
            && !snowed
    }
}

impl Goal<MonsterGoalContext> for SleepGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look, Control::Jump])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if fox(ctx).is_none_or(|goals| goals.moving) {
            return false;
        }
        self.can_sleep(ctx, world) || flag(ctx, SLEEPING)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.can_sleep(ctx, world)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.jump = false;
        ctx.navigation.stop();
        if let Some(goals) = fox_mut(ctx) {
            for flag in [SITTING, CROUCHING, INTERESTED] {
                goals.set(flag, false);
            }
            goals.set(SLEEPING, true);
            goals.halt = true;
        }
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        let wait = reduced_tick_delay(SLEEP_WAIT);
        self.countdown = Some(ctx.random.next_int(wait) as i32);
        clear_states(ctx);
    }
}

#[derive(Clone)]
struct FoxFollowParentGoal(FollowParentGoal<FoxGoals>);

impl Goal<MonsterGoalContext> for FoxFollowParentGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        !flag(ctx, DEFENDING) && self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        !flag(ctx, DEFENDING) && self.0.can_continue(ctx, world)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        clear_states(ctx);
        self.0.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.0.tick_world(ctx, world);
    }
}

fn can_fox_move(ctx: &MonsterGoalContext) -> bool {
    let busy = fox(ctx)
        .is_none_or(|goals| goals.flag(SLEEPING) || goals.flag(SITTING) || goals.flag(DEFENDING));
    !busy && ctx.target.is_none()
}

#[derive(Clone, Default)]
struct FoxStrollThroughVillageGoal {
    wanted: Option<Pos>,
}

impl FoxStrollThroughVillageGoal {
    fn walk_to(ctx: &mut MonsterGoalContext, world: &dyn World, pos: Pos) -> bool {
        let y = world.motion_blocking_no_leaves_height(pos.0, pos.2);
        let target = DVec3::new(f64::from(pos.0), f64::from(y), f64::from(pos.2));
        let profile = ctx.profile();
        navigate_walk_to(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            target,
            1.0,
            1,
        )
        .unwrap_or(false)
    }
}

impl Goal<MonsterGoalContext> for FoxStrollThroughVillageGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.bright_outside {
            return false;
        }
        let interval = reduced_tick_delay(VILLAGE_INTERVAL);
        if ctx.random.next_int(interval) != 0 {
            return false;
        }
        let p = ctx.body.position;
        let Some(pois) = ctx.pois.as_mut() else {
            return false;
        };
        if pois.sections_to_village(section(block_of(p))) > VILLAGE_SECTIONS {
            return false;
        }
        let land = land_random_position_by(
            world,
            &ctx.walk,
            p,
            VILLAGE_HORIZONTAL,
            VILLAGE_VERTICAL,
            &mut ctx.random,
            |spot| -f64::from(pois.sections_to_village(section(spot))),
        );
        self.wanted = land.map(block_of);
        self.wanted.is_some() && can_fox_move(ctx)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        self.wanted.is_some()
            && !ctx.navigation.is_done()
            && ctx.navigation.target_pos == self.wanted
            && can_fox_move(ctx)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        clear_states(ctx);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(wanted) = self.wanted else {
            return;
        };
        let p = ctx.body.position;
        let centre = DVec3::new(
            f64::from(wanted.0) + 0.5,
            f64::from(wanted.1) + 0.5,
            f64::from(wanted.2) + 0.5,
        );
        if !ctx.navigation.is_done() || centre.distance_squared(p) < VILLAGE_STEP * VILLAGE_STEP {
            return;
        }
        let bottom = bottom_centre(wanted);
        let pulled = (p - bottom) * VILLAGE_PULL + bottom;
        let step = (pulled - p).normalize_or_zero() * VILLAGE_STEP + p;
        if Self::walk_to(ctx, world, block_of(step)) {
            return;
        }
        let spread = VILLAGE_WANDER;
        let x = block_of(p).0 - 8 + ctx.random.next_int(spread) as i32;
        let z = block_of(p).2 - 8 + ctx.random.next_int(spread) as i32;
        Self::walk_to(ctx, world, (x, 0, z));
    }
}

#[derive(Clone)]
struct FoxLookAtPlayerGoal(LookAtPlayerGoal);

fn looks_around(ctx: &MonsterGoalContext) -> bool {
    !flag(ctx, FACEPLANTED) && !flag(ctx, INTERESTED)
}

impl Goal<MonsterGoalContext> for FoxLookAtPlayerGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_start(ctx, world) && looks_around(ctx)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world) && looks_around(ctx)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.tick(ctx);
    }
}

#[derive(Clone, Default)]
struct PerchAndSearchGoal {
    rel_x: f64,
    rel_z: f64,
    look_time: i32,
    looks_remaining: i32,
}

impl PerchAndSearchGoal {
    fn reset_look(&mut self, random: &mut LegacyRandom) {
        let angle = std::f64::consts::TAU * random.next_double();
        self.rel_x = jmath::cos(angle);
        self.rel_z = jmath::sin(angle);
        let ticks = PERCH_LOOK_TICKS + random.next_int(PERCH_LOOK_SPREAD);
        self.look_time = reduced_tick_delay(ticks) as i32;
    }
}

impl Goal<MonsterGoalContext> for PerchAndSearchGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        if ctx.hurt_by.is_some() || ctx.random.next_float() >= PERCH_CHANCE {
            return false;
        }
        let Some(goals) = fox(ctx) else {
            return false;
        };
        !goals.flag(SLEEPING)
            && ctx.target.is_none()
            && ctx.navigation.is_done()
            && !goals.alertable
            && !goals.flag(POUNCING)
            && !goals.flag(CROUCHING)
    }

    fn can_continue(&mut self, _ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        self.looks_remaining > 0
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.reset_look(&mut ctx.random);
        self.looks_remaining = PERCH_LOOKS + ctx.random.next_int(PERCH_EXTRA_LOOKS) as i32;
        if let Some(goals) = fox_mut(ctx) {
            goals.set(SITTING, true);
        }
        ctx.navigation.stop();
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = fox_mut(ctx) {
            goals.set(SITTING, false);
        }
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        self.look_time -= 1;
        if self.look_time <= 0 {
            self.looks_remaining -= 1;
            self.reset_look(&mut ctx.random);
        }
        let eye = ctx.eye();
        let at = DVec3::new(eye.x + self.rel_x, eye.y, eye.z + self.rel_z);
        ctx.look_control
            .set_look_at_with_limits(at, MAX_HEAD_Y_ROT, MAX_HEAD_X_ROT);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fox_ai_step<W: World>(
    ai: &mut MonsterAi,
    body: &mut Body,
    health: f32,
    random: &mut LegacyRandom,
    voices: &mut Vec<(Voice, DVec3)>,
    no_action_time: &mut i32,
    previous_position: DVec3,
    tick_count: i32,
    id: u64,
    world: &W,
    step: &MonsterStep,
) -> Option<f64> {
    let position = body.position;
    let eye_height = ai.state.eye_height;
    let near = |p: &PlayerCandidate| {
        p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0
    };
    if step.players.iter().any(near) {
        *no_action_time = 0;
    }
    if ai.no_jump_delay > 0 {
        ai.no_jump_delay -= 1;
    }
    if fox(&ai.state).is_some_and(|goals| goals.flag(SLEEPING)) {
        ai.jumping = false;
        ai.forward = 0.0;
        ai.sideways = 0.0;
    }
    let moving = ai.forward != 0.0 || ai.sideways != 0.0;
    if let Some(goals) = ai.state.extra.get_mut::<FoxGoals>() {
        goals.moving = moving;
        goals.jumping = ai.jumping;
    }
    body.trim_small_velocity();
    *no_action_time += 1;
    let fluid = FluidFrame::sample(world, position, body.width, body.height);
    ai.state.body = body.clone();
    ai.state.health = health;
    ai.state.game_time = step.game_time;
    ai.state.difficulty = step.difficulty;
    ai.state.players = step.players.to_vec();
    ai.state.villagers = step.villagers.to_vec();
    ai.state.fluid = fluid;
    ai.state.no_action_time = *no_action_time;
    ai.state.random = std::mem::take(random);
    let full = tick_count <= 1 || (tick_count + id as i32) % 2 == 0;
    ai.tick_goals(world, full);
    *random = std::mem::take(&mut ai.state.random);
    if let Some(velocity) = ai.state.leap.take() {
        body.velocity = velocity;
    }
    let halt = ai
        .state
        .extra
        .get_mut::<FoxGoals>()
        .is_some_and(|goals| std::mem::take(&mut goals.halt));
    if halt {
        ai.move_control.set_wanted_position(position, 0.0);
    }
    let (can_update, surface) = ground_view(world, body, fluid, ai.state.walk.can_float);
    let navigation = &mut ai.state.navigation;
    if let Some((wanted, speed)) =
        navigation.tick_in(world, position, can_update, surface, body.width, ai.speed)
    {
        ai.move_control.set_wanted_position(wanted, speed);
    }
    let obstacle_top = obstacle_top(world, position);
    ai.sideways *= 0.98;
    ai.forward *= 0.98;
    let control = if ai.still.is_some_and(|still| still(&ai.state)) {
        MoveFrame {
            yaw: ai.yaw,
            speed: ai.speed,
            forward: ai.forward,
            sideways: ai.sideways,
            jump: false,
        }
    } else {
        ai.move_control.tick(
            position,
            body.on_ground,
            ai.yaw,
            ai.speed,
            ai.forward,
            ai.sideways,
            step.movement_speed,
            body.width,
            body.step_height,
            obstacle_top,
            |_, _| true,
        )
    };
    ai.yaw = control.yaw;
    ai.speed = control.speed;
    ai.forward = control.forward;
    ai.sideways = control.sideways;
    let goals = fox(&ai.state);
    if !goals.is_some_and(|goals| goals.flag(SLEEPING)) {
        let reset_pitch = !goals.is_some_and(FoxGoals::keeps_pitch);
        let navigating = !ai.state.navigation.is_done();
        let body_yaw = ai.body_rotation.body_yaw;
        ai.state.look_control.tick_from(
            position,
            eye_height,
            body_yaw,
            navigating,
            MAX_HEAD_Y_ROT,
            reset_pitch,
        );
    }
    ai.jumping = ai.state.jump || control.jump;
    body.living_jump(world, fluid, ai.jumping, &mut ai.no_jump_delay, 0.4);
    let input = DVec3::new(f64::from(ai.sideways), 0.0, f64::from(ai.forward));
    let landed = if fluid.in_water() {
        body.travel_water(world, input, ai.yaw);
        None
    } else if fluid.in_lava() {
        body.travel_lava(world, input, ai.yaw, fluid.lava_height);
        None
    } else {
        body.travel_air_jumping(world, input, ai.speed, ai.yaw, ai.jumping)
    };
    play_movement(body, tick_count, random, voices, step.sounds);
    ai.body_rotation.tick(
        ai.yaw,
        &mut ai.state.look_control,
        previous_position,
        body.position,
    );
    landed.map(|fallen| fallen - SAFE_FALL_BONUS / landed_block(body, world).0)
}
