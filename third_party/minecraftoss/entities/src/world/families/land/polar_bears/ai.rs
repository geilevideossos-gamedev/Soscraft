use crate::goals::{Controls, Goal, GoalSelector};
use crate::monster_ai::{
    CheckedMeleeAttackGoal, FloatGoal, HurtByTargetGoal, LookAtPlayerGoal, MeleeAttackGoal,
    MonsterGoalContext, NearestTargetGoal, NeverGoal, Prey, RandomLookGoal, TargetInfo,
};
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::families::MonsterSpec;
use crate::world::families::goals::RandomStrollGoal;
use crate::world::families::land::animal_goals::{
    AnimalExtra, AnimalGoals, FollowParentGoal, PanicGoal, panic_environmental_cause,
};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::path_type::PathType;

pub const ATTACK_SPEED: f64 = 1.25;
pub const PANIC_SPEED: f64 = 2.0;
pub const FOLLOW_PARENT_SPEED: f64 = 1.25;
pub const STROLL_SPEED: f64 = 1.0;
pub const STROLL_INTERVAL: u32 = 120;
pub const LOOK_RANGE: f64 = 6.0;
pub const WARNING_TICKS: i32 = 40;
pub const WARNING_REACH: f32 = 3.0;
pub const STAND_UP_TICKS: i32 = 10;
pub const CUB_RANGE: f64 = 8.0;
pub const CUB_RANGE_Y: f64 = 4.0;
pub const ATTACK_PLAYERS_INTERVAL: u32 = 10;
pub const TARGET_INTERVAL: u32 = 5;
pub const FOX: &str = "minecraft:fox";

pub const GOAL_NAMES: [&str; 7] = [
    "FloatGoal",
    "PolarBearMeleeAttackGoal",
    "PanicGoal",
    "FollowParentGoal",
    "RandomStrollGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
];
pub const TARGET_NAMES: [&str; 5] = [
    "PolarBearHurtByTargetGoal",
    "PolarBearAttackPlayersGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
    "ResetUniversalAngerTargetGoal",
];

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
pub struct PolarBearGoals {
    pub animal: AnimalGoals,
    pub baby: bool,
    pub standing: bool,
    pub warning_ticks: i32,
    pub warn: Option<f32>,
}

impl AnimalExtra for PolarBearGoals {
    fn animal(&self) -> &AnimalGoals {
        &self.animal
    }

    fn animal_mut(&mut self) -> &mut AnimalGoals {
        &mut self.animal
    }
}

fn goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(0, FloatGoal);
    let melee = MeleeAttackGoal {
        hits: true,
        raises_arms: false,
        follow_unseen: true,
        gives_up_in_light: false,
        speed: ATTACK_SPEED,
    };
    selector.add(
        1,
        PolarBearMeleeAttackGoal(melee.checked(check_and_perform_attack)),
    );
    selector.add(
        1,
        PolarBearPanicGoal(PanicGoal::new(PANIC_SPEED, panic_environmental_cause)),
    );
    selector.add(
        4,
        FollowParentGoal::<PolarBearGoals>::new(FOLLOW_PARENT_SPEED),
    );
    selector.add(5, RandomStrollGoal::new(STROLL_SPEED, STROLL_INTERVAL));
    selector.add(6, LookAtPlayerGoal { range: LOOK_RANGE });
    selector.add(7, RandomLookGoal);
    selector
}

fn targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        PolarBearHurtByTargetGoal(HurtByTargetGoal {
            timestamp: 0,
            unseen_ticks: 0,
            target_mob: None,
            alert_others: false,
        }),
    );
    selector.add(
        2,
        PolarBearAttackPlayersGoal(NearestTargetGoal::every(
            Prey::Player,
            true,
            ATTACK_PLAYERS_INTERVAL,
        )),
    );
    selector.add(
        3,
        NearestTargetGoal::every(Prey::AngryPlayer, true, TARGET_INTERVAL),
    );
    selector.add(
        4,
        Adult(NearestTargetGoal::every(
            Prey::Kinds(&[FOX]),
            true,
            TARGET_INTERVAL,
        )),
    );
    selector.add(5, NeverGoal { controls: &[] });
    selector
}

fn walk(walk: &mut WalkProfile) {
    walk.set_malus(PathType::FireInNeighbor, 16.0);
    walk.set_malus(PathType::Fire, -1.0);
    walk.walk_target = WalkTarget::Animal;
}

pub fn baby(ctx: &MonsterGoalContext) -> bool {
    ctx.extra
        .get::<PolarBearGoals>()
        .is_some_and(|goals| goals.baby)
}

fn set_standing(ctx: &mut MonsterGoalContext, standing: bool) {
    if let Some(goals) = ctx.extra.get_mut::<PolarBearGoals>() {
        goals.standing = standing;
    }
}

fn play_warning_sound(ctx: &mut MonsterGoalContext) {
    let baby = baby(ctx);
    let Some(goals) = ctx.extra.get_mut::<PolarBearGoals>() else {
        return;
    };
    if goals.warning_ticks <= 0 {
        goals.warn = Some(crate::world::voice_pitch(&mut ctx.random, baby));
        goals.warning_ticks = WARNING_TICKS;
    }
}

pub(super) fn check_and_perform_attack(
    ctx: &mut MonsterGoalContext,
    world: &dyn World,
    target: TargetInfo,
) {
    let time = ctx.melee.is_time_to_attack();
    if time && ctx.within_melee_range(target) && ctx.sees(world, target) {
        ctx.melee.reset_attack_cooldown();
        ctx.attack = Some(target.target);
        set_standing(ctx, false);
        return;
    }
    let reach = (target.width + WARNING_REACH) * (target.width + WARNING_REACH);
    if ctx.body.position.distance_squared(target.position) < f64::from(reach) {
        if ctx.melee.is_time_to_attack() {
            set_standing(ctx, false);
            ctx.melee.reset_attack_cooldown();
        }
        if ctx.melee.ticks_until_attack() <= STAND_UP_TICKS {
            set_standing(ctx, true);
            play_warning_sound(ctx);
        }
    } else {
        ctx.melee.reset_attack_cooldown();
        set_standing(ctx, false);
    }
}

pub fn cub_near(ctx: &MonsterGoalContext) -> bool {
    let body = &ctx.body;
    let (p, half) = (body.position, f64::from(body.width) / 2.0);
    let min = DVec3::new(
        p.x - half - CUB_RANGE,
        p.y - CUB_RANGE_Y,
        p.z - half - CUB_RANGE,
    );
    let max = DVec3::new(
        p.x + half + CUB_RANGE,
        p.y + f64::from(body.height) + CUB_RANGE_Y,
        p.z + half + CUB_RANGE,
    );
    ctx.mobs.iter().any(|mob| {
        let (q, half) = (mob.position, f64::from(mob.width) / 2.0);
        mob.kind == super::KIND
            && mob.baby
            && q.x - half < max.x
            && q.x + half > min.x
            && q.y < max.y
            && q.y + f64::from(mob.height) > min.y
            && q.z - half < max.z
            && q.z + half > min.z
    })
}

fn halved(ctx: &mut MonsterGoalContext, run: impl FnOnce(&mut MonsterGoalContext) -> bool) -> bool {
    let range = ctx.follow_range;
    ctx.follow_range = range * 0.5;
    let result = run(ctx);
    ctx.follow_range = range;
    result
}

#[derive(Clone)]
struct PolarBearMeleeAttackGoal(CheckedMeleeAttackGoal);

impl Goal<MonsterGoalContext> for PolarBearMeleeAttackGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_start(ctx, world)
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
        self.0.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        set_standing(ctx, false);
        self.0.stop(ctx);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.0.tick_world(ctx, world);
    }
}

#[derive(Clone)]
struct PolarBearPanicGoal(PanicGoal<PolarBearGoals>);

impl Goal<MonsterGoalContext> for PolarBearPanicGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.causes = if baby(ctx) {
            crate::feline_ai::panic_cause
        } else {
            panic_environmental_cause
        };
        self.0.can_start(ctx, world)
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
struct PolarBearHurtByTargetGoal(HurtByTargetGoal);

impl Goal<MonsterGoalContext> for PolarBearHurtByTargetGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.start(ctx);
        if baby(ctx) {
            ctx.alert = ctx.hurt_by.map(|(attacker, _)| attacker);
            self.0.stop(ctx);
        }
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }
}

#[derive(Clone)]
struct PolarBearAttackPlayersGoal(NearestTargetGoal);

impl Goal<MonsterGoalContext> for PolarBearAttackPlayersGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if baby(ctx) {
            return false;
        }
        let goal = &mut self.0;
        halved(ctx, |ctx| goal.can_start(ctx, world)) && cub_near(ctx)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let goal = &mut self.0;
        halved(ctx, |ctx| goal.can_continue(ctx, world))
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }
}

#[derive(Clone)]
struct Adult<G>(G);

impl<G: Goal<MonsterGoalContext> + Clone + 'static> Goal<MonsterGoalContext> for Adult<G> {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_start(ctx, world) && !baby(ctx)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }
}
