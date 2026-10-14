use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{
    AvoidEntityGoal, FloatGoal, LookAtPlayerGoal, MonsterGoalContext, StrollGoal,
};
use crate::navigation::navigate_walk_to;
use crate::wandering_trader::{self as trader, Drink};
use glam::DVec3;
use minecraftoss_player::{Pos, World};

pub const GOAL_NAMES: [&str; 18] = [
    "FloatGoal",
    "UseItemGoal",
    "UseItemGoal",
    "TradeWithPlayerGoal",
    "AvoidEntityGoal",
    "AvoidEntityGoal",
    "AvoidEntityGoal",
    "AvoidEntityGoal",
    "AvoidEntityGoal",
    "AvoidEntityGoal",
    "AvoidEntityGoal",
    "PanicGoal",
    "LookAtTradingPlayerGoal",
    "WanderToPositionGoal",
    "MoveTowardsRestrictionGoal",
    "WaterAvoidingRandomStrollGoal",
    "InteractGoal",
    "LookAtPlayerGoal",
];
pub const TARGET_NAMES: [&str; 0] = [];

#[derive(Clone, Debug, Default)]
pub struct TraderState {
    pub trading: Option<u64>,
    pub invisible: bool,
    pub drinking: bool,
    pub drink: Option<Drink>,
    pub finished: Option<Drink>,
    pub stop_trading: bool,
    pub hurt_recently: bool,
    pub wander_target: Option<Pos>,
    pub home: Option<(Pos, i32)>,
    pub within_home: bool,
    pub last_damage: Option<&'static str>,
    pub panic: crate::panic::PanicState,
    pub restriction_wanted: Option<DVec3>,
}

pub fn trader_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(0, FloatGoal);
    selector.add(0, UseItemGoal(Drink::Invisibility));
    selector.add(0, UseItemGoal(Drink::Milk));
    selector.add(1, TradeWithPlayerGoal);
    let (zombie, zombie_range) = trader::AVOIDED[0];
    selector.add(
        1,
        AvoidEntityGoal::new(zombie, zombie_range, trader::FLEE_SPEED, trader::FLEE_SPEED).also(&[
            "minecraft:husk",
            "minecraft:drowned",
            "minecraft:zombie_villager",
            "minecraft:zombified_piglin",
        ]),
    );
    for (illager, range) in trader::ILLAGERS_AVOIDED {
        selector.add(
            1,
            AvoidEntityGoal::new(illager, range, trader::FLEE_SPEED, trader::FLEE_SPEED),
        );
    }
    let (zoglin, zoglin_range) = trader::AVOIDED[5];
    selector.add(
        1,
        AvoidEntityGoal::new(zoglin, zoglin_range, trader::FLEE_SPEED, trader::FLEE_SPEED),
    );
    selector.add(1, PanicGoal);
    selector.add(1, LookAtTradingPlayerGoal);
    selector.add(2, WanderToPositionGoal);
    selector.add(4, MoveTowardsRestrictionGoal);
    selector.add(8, StrollGoal::new(trader::STROLL_SPEED, 0.001));
    selector.add(9, InteractGoal);
    selector.add(10, LookAtPlayerGoal { range: 8.0 });
    selector
}

pub fn trader_targets() -> GoalSelector<MonsterGoalContext> {
    GoalSelector::default()
}

#[derive(Clone)]
struct UseItemGoal(Drink);
impl Goal<MonsterGoalContext> for UseItemGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        match self.0 {
            Drink::Invisibility => !ctx.bright_outside && !ctx.trader.invisible,
            Drink::Milk => ctx.bright_outside && ctx.trader.invisible,
        }
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.trader.drinking
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.trader.drink = Some(self.0);
        ctx.trader.drinking = true;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.trader.finished = Some(self.0);
    }
}

#[derive(Clone)]
struct TradeWithPlayerGoal;
impl TradeWithPlayerGoal {
    fn usable(ctx: &MonsterGoalContext) -> bool {
        if ctx.fluid.in_water() || !ctx.body.on_ground || ctx.trader.hurt_recently {
            return false;
        }
        let Some(player) = ctx.trader.trading else {
            return false;
        };
        ctx.players.iter().any(|p| {
            p.id == player
                && p.position.distance_squared(ctx.body.position) <= trader::TRADE_RANGE_SQR
        })
    }
}
impl Goal<MonsterGoalContext> for TradeWithPlayerGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Jump, Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        Self::usable(ctx)
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        Self::usable(ctx)
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.navigation.stop();
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.trader.stop_trading = true;
    }
}

#[derive(Clone)]
struct PanicGoal;
impl Goal<MonsterGoalContext> for PanicGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let panics = ctx
            .trader
            .last_damage
            .is_some_and(crate::feline_ai::panic_cause);
        let profile = ctx.profile();
        let (on_fire, position) = (ctx.on_fire, ctx.body.position);
        ctx.trader
            .panic
            .can_use(panics, on_fire, position, world, &profile, &mut ctx.random)
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }
    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if let Some(target) = ctx.trader.panic.wanted {
            let profile = ctx.profile();
            let _ = navigate_walk_to(
                &ctx.body,
                &mut ctx.navigation,
                world,
                &profile,
                ctx.fluid,
                target,
                trader::FLEE_SPEED,
                1,
            );
        }
        ctx.trader.panic.running = true;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.trader.panic.running = false;
    }
}

#[derive(Clone)]
struct LookAtTradingPlayerGoal;
impl Goal<MonsterGoalContext> for LookAtTradingPlayerGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.look_at_player.target_id = ctx.trader.trading;
        ctx.trader.trading.is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.look_at_player
            .can_continue_within(ctx.body.position, &ctx.players, 8.0)
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
                .set_look_at_with_limits(target, 10.0, x_max);
        }
    }
}

#[derive(Clone)]
struct InteractGoal;
impl Goal<MonsterGoalContext> for InteractGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Look, Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let (position, eye) = (ctx.body.position, ctx.eye());
        let mut best: Option<(f64, u64)> = None;
        for player in ctx.players.clone() {
            if !player.alive || player.spectator || position.distance_squared(player.position) > 9.0
            {
                continue;
            }
            let player_eye = player.position + DVec3::new(0.0, f64::from(player.eye_height), 0.0);
            if !crate::sight::line_of_sight(world, eye, player_eye) {
                continue;
            }
            let distance = player.position.distance_squared(eye);
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, player.id));
            }
        }
        ctx.look_at_player.target_id = best.map(|(_, id)| id);
        best.is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.look_at_player
            .can_continue_within(ctx.body.position, &ctx.players, 3.0)
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
                .set_look_at_with_limits(target, 10.0, x_max);
        }
    }
}

#[derive(Clone)]
struct WanderToPositionGoal;
impl WanderToPositionGoal {
    fn too_far(ctx: &MonsterGoalContext, distance: f64) -> Option<Pos> {
        let target = ctx.trader.wander_target?;
        (!trader::closer_to_center_than(target, ctx.body.position, distance)).then_some(target)
    }
}
impl Goal<MonsterGoalContext> for WanderToPositionGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        Self::too_far(ctx, trader::WANDER_STOP).is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        Self::too_far(ctx, trader::WANDER_STOP).is_some()
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.trader.wander_target = None;
        ctx.navigation.stop();
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(target) = ctx.trader.wander_target else {
            return;
        };
        if !ctx.navigation.is_done() {
            return;
        }
        let p = ctx.body.position;
        let goal = DVec3::new(
            f64::from(target.0),
            f64::from(target.1),
            f64::from(target.2),
        );
        let to = if Self::too_far(ctx, trader::WANDER_STEP).is_some() {
            p + (goal - p).normalize_or_zero() * trader::WANDER_STEP
        } else {
            goal
        };
        let profile = ctx.profile();
        let _ = navigate_walk_to(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            to,
            trader::STROLL_SPEED,
            1,
        );
    }
}

#[derive(Clone)]
struct MoveTowardsRestrictionGoal;
impl Goal<MonsterGoalContext> for MoveTowardsRestrictionGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.trader.within_home {
            return false;
        }
        let Some((home, _)) = ctx.trader.home else {
            return false;
        };
        let towards = DVec3::new(
            f64::from(home.0) + 0.5,
            f64::from(home.1),
            f64::from(home.2) + 0.5,
        );
        let profile = ctx.profile();
        let position = ctx.body.position;
        ctx.trader.restriction_wanted = crate::stroll::default_random_position_towards(
            world,
            &profile,
            position,
            16,
            7,
            towards,
            std::f32::consts::FRAC_PI_2,
            &mut ctx.random,
        );
        ctx.trader.restriction_wanted.is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }
    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(target) = ctx.trader.restriction_wanted.take() else {
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
            trader::STROLL_SPEED,
            1,
        );
    }
}
