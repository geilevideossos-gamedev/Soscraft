use super::Gene;
use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{
    AvoidEntityGoal, FloatGoal, HurtByTargetGoal, MeleeAttackGoal, MonsterGoalContext,
    RandomLookGoal, StrollGoal, Target,
};
use crate::movement::Body;
use crate::navigation::navigate_walk_to_entity;
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::families::MonsterSpec;
use crate::world::families::goals::reduced_tick_delay;
use crate::world::families::land::animal_goals::{
    AnimalExtra, AnimalGoals, BreedGoal, FollowParentGoal, PanicGoal, TemptGoal, is_monster_class,
    panic_environmental_cause,
};
use glam::DVec3;
use minecraftoss_player::mth;
use minecraftoss_player::path_type::PathType;
use minecraftoss_player::{World, WorldItem};

pub const PANIC_SPEED: f64 = 2.0;
pub const BREED_SPEED: f64 = 1.0;
pub const ATTACK_SPEED: f64 = 1.2000000476837158;
pub const TEMPT_SPEED: f64 = 1.0;
pub const AVOID_SPEED: f64 = 2.0;
pub const AVOID_PLAYER: f64 = 8.0;
pub const AVOID_MONSTER: f64 = 4.0;
pub const LOOK_RANGE: f64 = 6.0;
pub const LOOK_PROBABILITY: f32 = 0.02;
pub const LOOK_SPEED: f32 = 10.0;
pub const FOLLOW_PARENT_SPEED: f64 = 1.25;
pub const STROLL_SPEED: f64 = 1.0;
pub const STROLL_PROBABILITY: f32 = 0.001;
pub const UNHAPPY_TICKS: i32 = 32;
pub const UNHAPPY_COOLDOWN: i32 = 600;
pub const BREED_LOOK_RANGE: f64 = 8.0;
pub const BAMBOO_REACH: i32 = 7;
pub const BAMBOO_HEIGHT: i32 = 2;
pub const BAMBOO: &str = "minecraft:bamboo";
pub const PLAYER: &str = "minecraft:player";
pub const SIT_REACH: f64 = 6.0;
pub const FETCH_REACH: f64 = 8.0;
pub const FETCH_SPEED: f64 = 1.2000000476837158;
pub const SIT_LASTS: u32 = 600;
pub const SIT_ENDS: u32 = 2000;
pub const SIT_COOLDOWN: i32 = 10;
pub const SIT_COOLDOWN_SPREAD: u32 = 150;
pub const LAZY_SIT_COOLDOWN_SPREAD: u32 = 50;
pub const ITEM_SIZE: f64 = 0.25;
pub const LIE_CHANCE: u32 = 400;
pub const LIE_COOLDOWN: i32 = 200;
pub const SNEEZE_CHANCE: u32 = 6000;
pub const WEAK_SNEEZE_CHANCE: u32 = 500;
pub const ROLL_CHANCE: u32 = 500;
pub const PLAYFUL_ROLL_CHANCE: u32 = 60;
const DEGREES: f32 = 0.017453292;

pub const GOAL_NAMES: [&str; 15] = [
    "FloatGoal",
    "PandaPanicGoal",
    "PandaBreedGoal",
    "PandaAttackGoal",
    "TemptGoal",
    "PandaAvoidGoal",
    "PandaAvoidGoal",
    "PandaSitGoal",
    "PandaLieOnBackGoal",
    "PandaSneezeGoal",
    "PandaLookAtPlayerGoal",
    "RandomLookAroundGoal",
    "PandaRollGoal",
    "FollowParentGoal",
    "WaterAvoidingRandomStrollGoal",
];
pub const TARGET_NAMES: [&str; 1] = ["PandaHurtByTargetGoal"];

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
pub struct PandaGoals {
    pub animal: AnimalGoals,
    pub variant: Gene,
    pub baby: bool,
    pub sitting: bool,
    pub on_back: bool,
    pub rolling: bool,
    pub sneezing: bool,
    pub scared: bool,
    pub unhappy: i32,
    pub sneeze_counter: i32,
    pub eat_counter: i32,
    pub roll_counter: i32,
    pub got_bamboo: bool,
    pub did_bite: bool,
    pub holding: bool,
    pub food: Vec<WorldItem>,
    pub sat_down: bool,
    pub dropped: Option<DVec3>,
}

impl PandaGoals {
    pub fn eating(&self) -> bool {
        self.eat_counter > 0
    }

    pub fn can_perform_action(&self) -> bool {
        !(self.on_back || self.scared || self.eating() || self.rolling || self.sitting)
    }
}

impl AnimalExtra for PandaGoals {
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
    selector.add(
        2,
        PandaPanicGoal(PanicGoal::new(PANIC_SPEED, panic_environmental_cause)),
    );
    selector.add(
        2,
        PandaBreedGoal {
            breed: BreedGoal::new(BREED_SPEED),
            unhappy_cooldown: 0,
        },
    );
    let attack = MeleeAttackGoal {
        hits: true,
        raises_arms: false,
        follow_unseen: true,
        gives_up_in_light: false,
        speed: ATTACK_SPEED,
    };
    selector.add(
        3,
        Gated {
            goal: attack,
            gate: can_perform_action,
        },
    );
    selector.add(4, TemptGoal::<PandaGoals>::new(TEMPT_SPEED));
    let avoid = |kind: &'static str, distance: f64| {
        AvoidEntityGoal::new(kind, distance, AVOID_SPEED, AVOID_SPEED)
    };
    selector.add(
        6,
        Gated {
            goal: avoid(PLAYER, AVOID_PLAYER),
            gate: worried_and_able,
        },
    );
    selector.add(
        6,
        Gated {
            goal: avoid("", AVOID_MONSTER).matching(is_monster_class),
            gate: worried_and_able,
        },
    );
    selector.add(7, PandaSitGoal::default());
    selector.add(8, PandaLieOnBackGoal::default());
    selector.add(8, PandaSneezeGoal);
    selector.add(9, PandaLookAtPlayerGoal);
    selector.add(10, RandomLookGoal);
    selector.add(12, PandaRollGoal);
    selector.add(13, FollowParentGoal::<PandaGoals>::new(FOLLOW_PARENT_SPEED));
    selector.add(14, StrollGoal::new(STROLL_SPEED, STROLL_PROBABILITY));
    selector
}

fn targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        PandaHurtByTargetGoal(HurtByTargetGoal {
            timestamp: 0,
            unseen_ticks: 0,
            target_mob: None,
            alert_others: true,
        }),
    );
    selector
}

fn walk(walk: &mut WalkProfile) {
    walk.set_malus(PathType::FireInNeighbor, 16.0);
    walk.set_malus(PathType::Fire, -1.0);
    walk.walk_target = WalkTarget::Animal;
}

fn panda(ctx: &MonsterGoalContext) -> Option<&PandaGoals> {
    ctx.extra.get::<PandaGoals>()
}

pub fn can_perform_action(ctx: &MonsterGoalContext) -> bool {
    panda(ctx).is_some_and(PandaGoals::can_perform_action)
}

pub fn held_still(ctx: &MonsterGoalContext) -> bool {
    panda(ctx).is_some_and(|goals| !goals.can_perform_action())
}

pub fn within(body: &Body, item: &WorldItem, reach: f64) -> bool {
    let (p, q) = (body.position, item.position);
    let (half, size) = (f64::from(body.width) / 2.0 + reach, ITEM_SIZE / 2.0);
    q.x + size > p.x - half
        && q.x - size < p.x + half
        && q.y + ITEM_SIZE > p.y - reach
        && q.y < p.y + f64::from(body.height) + reach
        && q.z + size > p.z - half
        && q.z - size < p.z + half
}

fn try_to_sit(ctx: &mut MonsterGoalContext) {
    if ctx.fluid.in_water() {
        return;
    }
    ctx.navigation.stop();
    if let Some(goals) = ctx.extra.get_mut::<PandaGoals>() {
        goals.sat_down = true;
        goals.sitting = true;
    }
}

fn stays_down(ctx: &mut MonsterGoalContext) -> bool {
    if ctx.fluid.in_water() {
        return false;
    }
    let lazy = panda(ctx).is_some_and(|goals| goals.variant == Gene::Lazy);
    if !lazy && ctx.random.next_int(reduced_tick_delay(SIT_LASTS)) == 1 {
        return false;
    }
    ctx.random.next_int(reduced_tick_delay(SIT_ENDS)) != 1
}

fn worried_and_able(ctx: &MonsterGoalContext) -> bool {
    panda(ctx).is_some_and(|goals| goals.variant == Gene::Worried && goals.can_perform_action())
}

pub fn can_find_bamboo(world: &dyn World, position: DVec3) -> bool {
    let (x, y, z) = (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    let reach = -BAMBOO_REACH..=BAMBOO_REACH;
    (0..=BAMBOO_HEIGHT).any(|dy| {
        reach.clone().any(|dx| {
            reach.clone().any(|dz| {
                world
                    .block((x + dx, y + dy, z + dz))
                    .is_some_and(|block| block.id == BAMBOO)
            })
        })
    })
}

fn nearest_player(
    ctx: &mut MonsterGoalContext,
    world: &dyn World,
    range: f64,
    from: DVec3,
) -> Option<u64> {
    let position = ctx.body.position;
    let mut best: Option<(f64, u64)> = None;
    for player in ctx.players.clone() {
        if !player.alive || player.spectator {
            continue;
        }
        if position.distance_squared(player.position) > range * range {
            continue;
        }
        let Some(info) = ctx.info(Target::Player(player.id)) else {
            continue;
        };
        if !ctx.sees(world, info) {
            continue;
        }
        let distance = from.distance_squared(player.position);
        if best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, player.id));
        }
    }
    best.map(|(_, id)| id)
}

#[derive(Clone)]
struct Gated<G> {
    goal: G,
    gate: fn(&MonsterGoalContext) -> bool,
}

impl<G: Goal<MonsterGoalContext> + Clone + 'static> Goal<MonsterGoalContext> for Gated<G> {
    fn controls(&self) -> Controls {
        self.goal.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        (self.gate)(ctx) && self.goal.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.goal.can_continue(ctx, world)
    }

    fn interruptible(&self) -> bool {
        self.goal.interruptible()
    }

    fn every_tick(&self) -> bool {
        self.goal.every_tick()
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.goal.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.goal.stop(ctx);
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        self.goal.tick(ctx);
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.goal.start_world(ctx, world);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.goal.tick_world(ctx, world);
    }
}

#[derive(Clone)]
struct PandaPanicGoal(PanicGoal<PandaGoals>);

impl Goal<MonsterGoalContext> for PandaPanicGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if panda(ctx).is_some_and(|goals| goals.sitting) {
            ctx.navigation.stop();
            return false;
        }
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
struct PandaBreedGoal {
    breed: BreedGoal<PandaGoals>,
    unhappy_cooldown: i32,
}

impl Goal<MonsterGoalContext> for PandaBreedGoal {
    fn controls(&self) -> Controls {
        self.breed.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if !self.breed.can_start(ctx, world) || panda(ctx).is_none_or(|goals| goals.unhappy != 0) {
            return false;
        }
        if can_find_bamboo(world, ctx.body.position) {
            return true;
        }
        if self.unhappy_cooldown <= ctx.tick_count {
            if let Some(goals) = ctx.extra.get_mut::<PandaGoals>() {
                goals.unhappy = UNHAPPY_TICKS;
            }
            self.unhappy_cooldown = ctx.tick_count + UNHAPPY_COOLDOWN;
            let position = ctx.body.position;
            ctx.look_at_player.target_id = nearest_player(ctx, world, BREED_LOOK_RANGE, position);
        }
        false
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.breed.can_continue(ctx, world)
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.breed.stop(ctx);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.breed.tick_world(ctx, world);
    }
}

#[derive(Clone, Default)]
struct PandaSitGoal {
    cooldown: i32,
}

impl Goal<MonsterGoalContext> for PandaSitGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let Some(goals) = panda(ctx) else {
            return false;
        };
        if self.cooldown > ctx.tick_count
            || goals.baby
            || ctx.fluid.in_water()
            || !goals.can_perform_action()
            || goals.unhappy > 0
        {
            return false;
        }
        goals.holding
            || goals
                .food
                .iter()
                .any(|item| within(&ctx.body, item, SIT_REACH))
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        stays_down(ctx)
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if panda(ctx).is_some_and(|goals| goals.holding) {
            try_to_sit(ctx);
        } else {
            let first = panda(ctx).and_then(|goals| {
                let mut near = goals.food.iter();
                near.find(|item| within(&ctx.body, item, FETCH_REACH))
                    .map(|item| item.position)
            });
            if let Some(target) = first {
                let profile = ctx.profile();
                navigate_walk_to_entity(
                    &ctx.body,
                    &mut ctx.navigation,
                    world,
                    &profile,
                    ctx.fluid,
                    target,
                    FETCH_SPEED,
                    1,
                );
            }
        }
        self.cooldown = 0;
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        let Some((holding, lazy)) =
            panda(ctx).map(|goals| (goals.holding, goals.variant == Gene::Lazy))
        else {
            return;
        };
        if holding {
            let spread = if lazy {
                LAZY_SIT_COOLDOWN_SPREAD
            } else {
                SIT_COOLDOWN_SPREAD
            };
            let wait = SIT_COOLDOWN + ctx.random.next_int(spread) as i32;
            self.cooldown = ctx.tick_count + wait * 20;
        }
        let at = ctx.body.position;
        if let Some(goals) = ctx.extra.get_mut::<PandaGoals>() {
            if holding {
                goals.holding = false;
                goals.dropped = Some(at);
            }
            goals.sitting = false;
        }
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        if panda(ctx).is_some_and(|goals| goals.holding && !goals.sitting) {
            try_to_sit(ctx);
        }
    }
}

#[derive(Clone, Default)]
struct PandaLieOnBackGoal {
    cooldown: i32,
}

impl Goal<MonsterGoalContext> for PandaLieOnBackGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let lazy = panda(ctx).is_some_and(|goals| goals.variant == Gene::Lazy);
        self.cooldown < ctx.tick_count
            && lazy
            && can_perform_action(ctx)
            && ctx.random.next_int(reduced_tick_delay(LIE_CHANCE)) == 1
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        stays_down(ctx)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = ctx.extra.get_mut::<PandaGoals>() {
            goals.on_back = true;
        }
        self.cooldown = 0;
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = ctx.extra.get_mut::<PandaGoals>() {
            goals.on_back = false;
        }
        self.cooldown = ctx.tick_count + LIE_COOLDOWN;
    }
}

#[derive(Clone)]
struct PandaSneezeGoal;

impl Goal<MonsterGoalContext> for PandaSneezeGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let Some((baby, weak)) = panda(ctx).map(|goals| (goals.baby, goals.variant == Gene::Weak))
        else {
            return false;
        };
        if !baby || !can_perform_action(ctx) {
            return false;
        }
        if weak && ctx.random.next_int(reduced_tick_delay(WEAK_SNEEZE_CHANCE)) == 1 {
            return true;
        }
        ctx.random.next_int(reduced_tick_delay(SNEEZE_CHANCE)) == 1
    }

    fn can_continue(&mut self, _ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        false
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = ctx.extra.get_mut::<PandaGoals>() {
            goals.sneezing = true;
        }
    }
}

pub fn roll_ahead(yaw: f32) -> (i32, i32) {
    let yaw = f64::from(yaw * DEGREES);
    let step = |d: f32| {
        if d.abs() <= 0.5 {
            0
        } else if d > 0.0 {
            1
        } else {
            -1
        }
    };
    (step(-mth::sin(yaw)), step(mth::cos(yaw)))
}

#[derive(Clone)]
struct PandaRollGoal;

impl Goal<MonsterGoalContext> for PandaRollGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look, Control::Jump])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let Some((baby, playful)) =
            panda(ctx).map(|goals| (goals.baby, goals.variant == Gene::Playful))
        else {
            return false;
        };
        if !(baby || playful) || !ctx.body.on_ground || !can_perform_action(ctx) {
            return false;
        }
        let (dx, dz) = roll_ahead(ctx.yaw);
        let p = ctx.body.position;
        let below = (
            p.x.floor() as i32 + dx,
            p.y.floor() as i32 - 1,
            p.z.floor() as i32 + dz,
        );
        if crate::shulker::is_air(world, below) {
            return true;
        }
        if playful && ctx.random.next_int(reduced_tick_delay(PLAYFUL_ROLL_CHANCE)) == 1 {
            return true;
        }
        ctx.random.next_int(reduced_tick_delay(ROLL_CHANCE)) == 1
    }

    fn can_continue(&mut self, _ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        false
    }

    fn interruptible(&self) -> bool {
        false
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = ctx.extra.get_mut::<PandaGoals>() {
            goals.rolling = true;
        }
    }
}

#[derive(Clone)]
struct PandaLookAtPlayerGoal;

impl Goal<MonsterGoalContext> for PandaLookAtPlayerGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Look])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.random.next_float() >= LOOK_PROBABILITY {
            return false;
        }
        if ctx.look_at_player.target_id.is_none() {
            let eye = ctx.eye();
            ctx.look_at_player.target_id = nearest_player(ctx, world, LOOK_RANGE, eye);
        }
        can_perform_action(ctx) && ctx.look_at_player.target_id.is_some()
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.look_at_player.target_id.is_some()
            && ctx
                .look_at_player
                .can_continue_within(ctx.body.position, &ctx.players, LOOK_RANGE)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.look_at_player.start(&mut ctx.random);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.look_at_player.stop();
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(target) = ctx.look_at_player.tick(&ctx.players) {
            let x_max = ctx.max_head_x_rot();
            ctx.look_control
                .set_look_at_with_limits(target, LOOK_SPEED, x_max);
        }
    }
}

#[derive(Clone)]
struct PandaHurtByTargetGoal(HurtByTargetGoal);

impl Goal<MonsterGoalContext> for PandaHurtByTargetGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if panda(ctx).is_some_and(|goals| goals.got_bamboo || goals.did_bite) {
            ctx.clear_target();
            return false;
        }
        self.0.can_continue(ctx, world)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }
}
