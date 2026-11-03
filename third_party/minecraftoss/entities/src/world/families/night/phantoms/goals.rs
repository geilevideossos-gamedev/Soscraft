use super::{EYE_HEIGHT, FOLLOW_RANGE};
use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{MonsterGoalContext, Target, TargetInfo};
use crate::movement::Body;
use crate::shulker::is_air;
use crate::walk_path::WalkProfile;
use crate::world::families::goals::reduced_tick_delay;
use crate::world::families::{GoalExtra, MonsterSpec};
use glam::DVec3;
use minecraftoss_player::{World, mth};

pub const GOAL_NAMES: [&str; 3] = [
    "PhantomAttackStrategyGoal",
    "PhantomSweepAttackGoal",
    "PhantomCircleAroundAnchorGoal",
];
pub const TARGET_NAMES: [&str; 1] = ["PhantomAttackPlayerTargetGoal"];
pub const SCAN_REACH: DVec3 = DVec3::new(16.0, 64.0, 16.0);
pub const SCAN_RANGE: f64 = 64.0;
pub const BITE_REACH: f64 = 0.20000000298023224;
pub const TOUCHING: f64 = 4.0;
pub const CAT_SEARCH_TICKS: i32 = 20;
pub const CAT_REACH: f64 = 16.0;

pub(super) static SPEC: MonsterSpec = MonsterSpec {
    goal_names: &GOAL_NAMES,
    target_names: &TARGET_NAMES,
    goals,
    targets,
    eye_height: EYE_HEIGHT,
    follow_range: FOLLOW_RANGE,
    walk,
};

fn goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, AttackStrategyGoal::default());
    selector.add(2, SweepAttackGoal::default());
    selector.add(3, CircleAroundAnchorGoal::default());
    selector
}

fn targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, AttackPlayerTargetGoal::default());
    selector
}

fn walk(_walk: &mut WalkProfile) {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttackPhase {
    #[default]
    Circle,
    Swoop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatSeen {
    pub id: u64,
    pub low: DVec3,
    pub high: DVec3,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PhantomGoals {
    pub phase: AttackPhase,
    pub anchor: Option<(i32, i32, i32)>,
    pub move_target: DVec3,
    pub sea_level: i32,
    pub hurt_recently: bool,
    pub swoop: Option<f32>,
    pub bit: bool,
    pub cats: Vec<CatSeen>,
    pub hiss: Vec<u64>,
}

pub fn shared(extra: &mut GoalExtra) -> &mut PhantomGoals {
    if extra.get::<PhantomGoals>().is_none() {
        extra.set(PhantomGoals::default());
    }
    extra
        .get_mut::<PhantomGoals>()
        .expect("the phantom's goals")
}

fn phase(ctx: &MonsterGoalContext) -> AttackPhase {
    ctx.extra
        .get::<PhantomGoals>()
        .map_or(AttackPhase::Circle, |goals| goals.phase)
}

pub fn block_of(position: DVec3) -> (i32, i32, i32) {
    (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

pub fn bounds(body: &Body) -> (DVec3, DVec3) {
    let half = f64::from(body.width) / 2.0;
    let p = body.position;
    (
        DVec3::new(p.x - half, p.y, p.z - half),
        DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
    )
}

pub fn target_bounds(target: &TargetInfo) -> (DVec3, DVec3) {
    let half = f64::from(target.width) / 2.0;
    let p = target.position;
    (
        DVec3::new(p.x - half, p.y, p.z - half),
        DVec3::new(p.x + half, p.y + f64::from(target.height), p.z + half),
    )
}

pub fn boxes_meet(a: (DVec3, DVec3), b: (DVec3, DVec3)) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
}

fn raw_target(ctx: &MonsterGoalContext) -> Option<TargetInfo> {
    ctx.target.and_then(|target| ctx.info(target))
}

fn attackable(ctx: &mut MonsterGoalContext, world: &dyn World, target: TargetInfo) -> bool {
    ctx.can_attack(target) && ctx.sees(world, target)
}

fn anchor_above_target(ctx: &mut MonsterGoalContext) {
    if shared(&mut ctx.extra).anchor.is_none() {
        return;
    }
    let Some(target) = raw_target(ctx) else {
        return;
    };
    let (x, y, z) = block_of(target.position);
    let up = 20 + ctx.random.next_int(20) as i32;
    let goals = shared(&mut ctx.extra);
    let y = if y + up < goals.sea_level {
        goals.sea_level + 1
    } else {
        y + up
    };
    goals.anchor = Some((x, y, z));
}

#[derive(Clone)]
pub(super) struct AttackPlayerTargetGoal {
    pub(super) next_scan_tick: u32,
}

impl Default for AttackPlayerTargetGoal {
    fn default() -> Self {
        Self {
            next_scan_tick: reduced_tick_delay(20),
        }
    }
}

impl Goal<MonsterGoalContext> for AttackPlayerTargetGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if self.next_scan_tick > 0 {
            self.next_scan_tick -= 1;
            return false;
        }
        self.next_scan_tick = reduced_tick_delay(60);
        let (low, high) = bounds(&ctx.body);
        let (low, high) = (low - SCAN_REACH, high + SCAN_REACH);
        let inside = |p: DVec3| {
            p.x >= low.x
                && p.x < high.x
                && p.y >= low.y
                && p.y < high.y
                && p.z >= low.z
                && p.z < high.z
        };
        let ids: Vec<u64> = ctx
            .players
            .iter()
            .filter(|player| inside(player.position))
            .map(|player| player.id)
            .collect();
        let mut near = Vec::new();
        for id in ids {
            let Some(info) = ctx.info(Target::Player(id)) else {
                continue;
            };
            let close =
                ctx.body.position.distance_squared(info.position) <= SCAN_RANGE * SCAN_RANGE;
            if close && attackable(ctx, world, info) {
                near.push(info);
            }
        }
        near.sort_by(|a, b| b.position.y.total_cmp(&a.position.y));
        for info in near {
            if attackable(ctx, world, info) {
                ctx.set_target(Some(info.target));
                return true;
            }
        }
        false
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        raw_target(ctx).is_some_and(|target| attackable(ctx, world, target))
    }
}

#[derive(Clone, Default)]
pub(super) struct AttackStrategyGoal {
    pub(super) next_sweep_tick: i32,
    stop_height: Option<i32>,
}

impl Goal<MonsterGoalContext> for AttackStrategyGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        raw_target(ctx).is_some_and(|target| attackable(ctx, world, target))
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let going = self.can_start(ctx, world);
        if !going {
            let anchor = shared(&mut ctx.extra).anchor;
            self.stop_height = anchor.map(|(x, _, z)| world.motion_blocking_height(x, z));
        }
        going
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.next_sweep_tick = reduced_tick_delay(10) as i32;
        shared(&mut ctx.extra).phase = AttackPhase::Circle;
        anchor_above_target(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        let Some((x, y, z)) = shared(&mut ctx.extra).anchor else {
            return;
        };
        let ground = self.stop_height.take().unwrap_or(y);
        let up = 10 + ctx.random.next_int(20) as i32;
        shared(&mut ctx.extra).anchor = Some((x, ground + up, z));
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        if phase(ctx) != AttackPhase::Circle {
            return;
        }
        self.next_sweep_tick -= 1;
        if self.next_sweep_tick > 0 {
            return;
        }
        shared(&mut ctx.extra).phase = AttackPhase::Swoop;
        anchor_above_target(ctx);
        let seconds = 8 + ctx.random.next_int(4);
        self.next_sweep_tick = reduced_tick_delay(seconds * 20) as i32;
        let pitch = 0.95 + ctx.random.next_float() * 0.1;
        shared(&mut ctx.extra).swoop = Some(pitch);
    }
}

#[derive(Clone, Default)]
pub(super) struct SweepAttackGoal {
    pub(super) scared_of_cat: bool,
    pub(super) cat_search_tick: i32,
}

impl Goal<MonsterGoalContext> for SweepAttackGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.target.is_some() && phase(ctx) == AttackPhase::Swoop
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let Some(target) = raw_target(ctx) else {
            return false;
        };
        if !target.alive {
            return false;
        }
        if let Target::Player(id) = target.target {
            let player = ctx.players.iter().find(|player| player.id == id);
            if player.is_some_and(|player| player.spectator || !player.attackable) {
                return false;
            }
        }
        if !self.can_start(ctx, world) {
            return false;
        }
        if ctx.tick_count > self.cat_search_tick {
            self.cat_search_tick = ctx.tick_count + CAT_SEARCH_TICKS;
            let (low, high) = bounds(&ctx.body);
            let reach = DVec3::splat(CAT_REACH);
            let around = (low - reach, high + reach);
            let goals = shared(&mut ctx.extra);
            let near: Vec<u64> = goals
                .cats
                .iter()
                .filter(|cat| boxes_meet(around, (cat.low, cat.high)))
                .map(|cat| cat.id)
                .collect();
            self.scared_of_cat = !near.is_empty();
            goals.hiss.extend(near);
        }
        !self.scared_of_cat
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.clear_target();
        shared(&mut ctx.extra).phase = AttackPhase::Circle;
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        let Some(target) = raw_target(ctx) else {
            return;
        };
        let p = target.position;
        let middle = DVec3::new(p.x, p.y + f64::from(target.height) * 0.5, p.z);
        shared(&mut ctx.extra).move_target = middle;
        let (low, high) = bounds(&ctx.body);
        let reach = DVec3::splat(BITE_REACH);
        if boxes_meet((low - reach, high + reach), target_bounds(&target)) {
            ctx.attack = Some(target.target);
            let goals = shared(&mut ctx.extra);
            goals.phase = AttackPhase::Circle;
            goals.bit = true;
        } else if ctx.body.horizontal_collision || shared(&mut ctx.extra).hurt_recently {
            shared(&mut ctx.extra).phase = AttackPhase::Circle;
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct CircleAroundAnchorGoal {
    pub(super) angle: f32,
    pub(super) distance: f32,
    pub(super) height: f32,
    pub(super) clockwise: f32,
}

impl CircleAroundAnchorGoal {
    fn select_next(&mut self, ctx: &mut MonsterGoalContext) {
        let own = block_of(ctx.body.position);
        let goals = shared(&mut ctx.extra);
        let (x, y, z) = *goals.anchor.get_or_insert(own);
        self.angle += self.clockwise * 15.0 * 0.017453292;
        let around = DVec3::new(
            f64::from(self.distance * mth::cos(f64::from(self.angle))),
            f64::from(-4.0 + self.height),
            f64::from(self.distance * mth::sin(f64::from(self.angle))),
        );
        goals.move_target = DVec3::new(f64::from(x), f64::from(y), f64::from(z)) + around;
    }
}

impl Goal<MonsterGoalContext> for CircleAroundAnchorGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.target.is_none() || phase(ctx) == AttackPhase::Circle
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.distance = 5.0 + ctx.random.next_float() * 10.0;
        self.height = -4.0 + ctx.random.next_float() * 9.0;
        self.clockwise = if ctx.random.next_boolean() { 1.0 } else { -1.0 };
        self.select_next(ctx);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if ctx.random.next_int(reduced_tick_delay(350)) == 0 {
            self.height = -4.0 + ctx.random.next_float() * 9.0;
        }
        if ctx.random.next_int(reduced_tick_delay(250)) == 0 {
            self.distance += 1.0;
            if self.distance > 15.0 {
                self.distance = 5.0;
                self.clockwise = -self.clockwise;
            }
        }
        if ctx.random.next_int(reduced_tick_delay(450)) == 0 {
            self.angle = ctx.random.next_float() * 2.0 * std::f32::consts::PI;
            self.select_next(ctx);
        }
        let position = ctx.body.position;
        if shared(&mut ctx.extra)
            .move_target
            .distance_squared(position)
            < TOUCHING
        {
            self.select_next(ctx);
        }
        let (x, y, z) = block_of(position);
        if shared(&mut ctx.extra).move_target.y < position.y && !is_air(world, (x, y - 1, z)) {
            self.height = self.height.max(1.0);
            self.select_next(ctx);
        }
        if shared(&mut ctx.extra).move_target.y > position.y && !is_air(world, (x, y + 1, z)) {
            self.height = self.height.min(-1.0);
            self.select_next(ctx);
        }
    }
}
