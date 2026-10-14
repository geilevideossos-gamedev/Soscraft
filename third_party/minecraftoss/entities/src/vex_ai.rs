use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{
    FloatGoal, MonsterGoalContext, NearestTargetGoal, Prey, Target, continue_target,
};
use crate::vex as vx;
use glam::DVec3;
use minecraftoss_player::{Pos, World};

pub const GOAL_NAMES: [&str; 5] = [
    "FloatGoal",
    "VexChargeAttackGoal",
    "VexRandomMoveGoal",
    "LookAtPlayerGoal",
    "LookAtPlayerGoal",
];
pub const TARGET_NAMES: [&str; 3] = [
    "HurtByTargetGoal",
    "VexCopyOwnerTargetGoal",
    "NearestAttackableTargetGoal",
];
pub const MOVE_CHANCE: u32 = 4;
pub const WANDER_TRIES: i32 = 3;

#[derive(Clone, Debug, Default)]
pub struct VexState {
    pub wanted: Option<(DVec3, f64)>,
    pub charging: bool,
    pub charged: bool,
    pub owner_target: Option<Target>,
    pub bound_origin: Option<Pos>,
    pub size: f64,
    pub look_at: Option<DVec3>,
}

pub fn vex_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(0, FloatGoal);
    selector.add(4, ChargeAttackGoal);
    selector.add(8, RandomMoveGoal);
    selector.add(9, crate::illager_ai::look_at(vx::LOOK_RANGE, 1.0, true));
    selector.add(
        10,
        crate::illager_ai::look_at(vx::MOB_LOOK_RANGE, 0.02, false),
    );
    selector
}

pub fn vex_targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, crate::illager_ai::raider_hurt_by());
    selector.add(2, CopyOwnerTargetGoal { unseen_ticks: 0 });
    selector.add(3, NearestTargetGoal::every(Prey::Player, true, 5));
    selector
}

fn eye_of(ctx: &MonsterGoalContext, target: Target) -> Option<DVec3> {
    let info = ctx.info(target)?;
    Some(info.position + DVec3::new(0.0, f64::from(info.eye_height), 0.0))
}

fn meets(ctx: &MonsterGoalContext, target: Target) -> bool {
    let Some(info) = ctx.info(target) else {
        return false;
    };
    let p = ctx.body.position;
    let half = f64::from(ctx.body.width / 2.0);
    let height = f64::from(ctx.body.height);
    let q = info.position;
    let other = f64::from(info.width / 2.0);
    p.x - half < q.x + other
        && p.x + half > q.x - other
        && p.y < q.y + f64::from(info.height)
        && p.y + height > q.y
        && p.z - half < q.z + other
        && p.z + half > q.z - other
}

#[derive(Clone)]
struct ChargeAttackGoal;

impl Goal<MonsterGoalContext> for ChargeAttackGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let Some(target) = ctx.target().filter(|t| t.alive) else {
            return false;
        };
        ctx.vex.wanted.is_none()
            && ctx.random.next_int(MOVE_CHANCE) == 0
            && ctx.body.position.distance_squared(target.position) > vx::CHARGE_START_SQR
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.vex.wanted.is_some() && ctx.vex.charging && ctx.target().is_some_and(|t| t.alive)
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(eye) = ctx.target().and_then(|t| eye_of(ctx, t.target)) {
            ctx.vex.wanted = Some((eye, vx::CHARGE_SPEED));
        }
        ctx.vex.charging = true;
        ctx.vex.charged = true;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.vex.charging = false;
    }
    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        let Some(target) = ctx.target() else {
            return;
        };
        if meets(ctx, target.target) {
            ctx.attack = Some(target.target);
            ctx.vex.charging = false;
        } else if ctx.body.position.distance_squared(target.position) < vx::CHARGE_FOLLOW_SQR {
            if let Some(eye) = eye_of(ctx, target.target) {
                ctx.vex.wanted = Some((eye, vx::CHARGE_SPEED));
            }
        }
    }
}

fn is_air(world: &dyn World, pos: Pos) -> bool {
    world.block(pos).is_none_or(|block| {
        matches!(
            block.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

#[derive(Clone)]
struct RandomMoveGoal;

impl Goal<MonsterGoalContext> for RandomMoveGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.vex.wanted.is_none() && ctx.random.next_int(MOVE_CHANCE) == 0
    }
    fn can_continue(&mut self, _ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        false
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let p = ctx.body.position;
        let origin = ctx.vex.bound_origin.unwrap_or((
            p.x.floor() as i32,
            p.y.floor() as i32,
            p.z.floor() as i32,
        ));
        for _ in 0..WANDER_TRIES {
            let x = origin.0 + ctx.random.next_int(15) as i32 - 7;
            let y = origin.1 + ctx.random.next_int(11) as i32 - 5;
            let z = origin.2 + ctx.random.next_int(15) as i32 - 7;
            if !is_air(world, (x, y, z)) {
                continue;
            }
            let center = DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5);
            ctx.vex.wanted = Some((center, vx::WANDER_SPEED));
            if ctx.target().is_none() {
                ctx.look_control
                    .set_look_at_with_limits(center, 180.0, 20.0);
            }
            break;
        }
    }
}

#[derive(Clone)]
struct CopyOwnerTargetGoal {
    unseen_ticks: i32,
}

impl Goal<MonsterGoalContext> for CopyOwnerTargetGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Target])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.vex
            .owner_target
            .and_then(|t| ctx.info(t))
            .is_some_and(|info| ctx.can_attack(info))
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        continue_target(ctx, world, None, &mut self.unseen_ticks, None)
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        let owner_target = ctx.vex.owner_target;
        ctx.set_target(owner_target);
        self.unseen_ticks = 0;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.clear_target();
    }
}
