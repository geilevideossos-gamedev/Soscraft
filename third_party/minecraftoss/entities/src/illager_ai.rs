use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::illager as il;
use crate::monster_ai::{
    AvoidEntityGoal, FloatGoal, HurtByTargetGoal, MonsterGoalContext, NearestTargetGoal, NeverGoal,
    Prey, Target,
};
use crate::navigation::{navigate_walk_to, navigate_walk_to_entity};
use glam::DVec3;
use minecraftoss_player::World;

pub const PILLAGER_GOAL_NAMES: [&str; 12] = [
    "LongDistancePatrolGoal",
    "ObtainRaidLeaderBannerGoal",
    "PathfindToRaidGoal",
    "RaiderMoveThroughVillageGoal",
    "RaiderCelebration",
    "FloatGoal",
    "AvoidEntityGoal",
    "HoldGroundAttackGoal",
    "RangedCrossbowAttackGoal",
    "RandomStrollGoal",
    "LookAtPlayerGoal",
    "LookAtPlayerGoal",
];
pub const PILLAGER_TARGET_NAMES: [&str; 4] = [
    "HurtByTargetGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
];
pub const PATH_DELAY_MIN: i32 = 20;
pub const PATH_DELAY_MAX: i32 = 40;
pub const ATTACK_DELAY_MIN: i32 = 20;
pub const ATTACK_DELAY_SPREAD: u32 = 20;
pub const AMBIENT_ROLL: u32 = 50;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossbowState {
    #[default]
    Uncharged,
    Charging,
    Charged,
    ReadyToAttack,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossbowShot {
    pub target: Target,
    pub position: DVec3,
    pub height: f32,
}

#[derive(Clone, Debug, Default)]
pub struct IllagerState {
    pub id: u64,
    pub holds_crossbow: bool,
    pub patrolling: bool,
    pub has_raid: bool,
    pub aggressive: bool,
    pub crossbow: CrossbowState,
    pub see_time: i32,
    pub attack_delay: i32,
    pub update_path_delay: i32,
    pub charging: bool,
    pub released: bool,
    pub uncharge: bool,
    pub shot: Option<CrossbowShot>,
    pub shout: Option<(Target, bool)>,
    pub ambient: bool,
    pub look_mob: Option<Target>,
    pub look_mob_time: i32,
}

pub fn pillager_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        4,
        NeverGoal {
            controls: &[Control::Move],
        },
    );
    selector.add(
        1,
        NeverGoal {
            controls: &[Control::Move],
        },
    );
    selector.add(
        3,
        NeverGoal {
            controls: &[Control::Move],
        },
    );
    selector.add(
        4,
        NeverGoal {
            controls: &[Control::Move],
        },
    );
    selector.add(
        5,
        NeverGoal {
            controls: &[Control::Move],
        },
    );
    selector.add(0, FloatGoal);
    selector.add(
        1,
        AvoidEntityGoal::new(
            "minecraft:creaking",
            il::CREAKING_AVOID_RANGE,
            il::CREAKING_WALK_SPEED,
            il::CREAKING_SPRINT_SPEED,
        ),
    );
    selector.add(
        2,
        HoldGroundAttackGoal {
            radius: il::HOLD_GROUND_RADIUS,
        },
    );
    selector.add(
        3,
        RangedCrossbowAttackGoal {
            speed: il::CROSSBOW_SPEED,
            radius: il::CROSSBOW_RANGE,
        },
    );
    selector.add(
        8,
        RandomStrollGoal {
            speed: il::STROLL_SPEED,
        },
    );
    selector.add(
        9,
        LookAtGoal {
            range: il::LOOK_RANGE,
            probability: 1.0,
            players: true,
        },
    );
    selector.add(
        10,
        LookAtGoal {
            range: il::LOOK_RANGE,
            probability: 0.02,
            players: false,
        },
    );
    selector
}

pub fn pillager_targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        RaiderHurtByGoal(HurtByTargetGoal {
            timestamp: 0,
            unseen_ticks: 0,
            target_mob: None,
            alert_others: true,
        }),
    );
    selector.add(2, NearestTargetGoal::every(Prey::Player, true, 5));
    selector.add(3, NearestTargetGoal::every(Prey::Villager, false, 5));
    selector.add(3, NearestTargetGoal::every(Prey::Golem, true, 5));
    selector
}

fn start_using(ctx: &mut MonsterGoalContext) {
    if !ctx.bow.using_item {
        ctx.bow.using_item = true;
        ctx.bow.ticks_using_item = 0;
    }
}

fn stop_using(ctx: &mut MonsterGoalContext) {
    ctx.bow.using_item = false;
    ctx.bow.ticks_using_item = 0;
}

#[derive(Clone)]
struct RangedCrossbowAttackGoal {
    speed: f64,
    radius: f32,
}

impl RangedCrossbowAttackGoal {
    fn valid_target(ctx: &MonsterGoalContext) -> bool {
        ctx.target().is_some_and(|target| target.alive)
    }
}

impl Goal<MonsterGoalContext> for RangedCrossbowAttackGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        Self::valid_target(ctx) && ctx.illager.holds_crossbow
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        Self::valid_target(ctx)
            && (self.can_start(ctx, world) || !ctx.navigation.is_done())
            && ctx.illager.holds_crossbow
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.illager.aggressive = false;
        ctx.clear_target();
        ctx.illager.see_time = 0;
        if ctx.bow.using_item {
            stop_using(ctx);
            ctx.illager.charging = false;
            ctx.illager.uncharge = true;
        }
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(target) = ctx.target() else {
            return;
        };
        let sees = ctx.sees(world, target);
        if sees != (ctx.illager.see_time > 0) {
            ctx.illager.see_time = 0;
        }
        if sees {
            ctx.illager.see_time += 1;
        } else {
            ctx.illager.see_time -= 1;
        }
        let distance = ctx.body.position.distance_squared(target.position);
        let radius = f64::from(self.radius * self.radius);
        let should_move =
            (distance > radius || ctx.illager.see_time < 5) && ctx.illager.attack_delay == 0;
        if should_move {
            ctx.illager.update_path_delay -= 1;
            if ctx.illager.update_path_delay <= 0 {
                let speed = if ctx.illager.crossbow == CrossbowState::Uncharged {
                    self.speed
                } else {
                    self.speed * 0.5
                };
                let profile = ctx.profile();
                let _ = navigate_walk_to_entity(
                    &ctx.body,
                    &mut ctx.navigation,
                    world,
                    &profile,
                    ctx.fluid,
                    target.position,
                    speed,
                    1,
                );
                let spread = (PATH_DELAY_MAX - PATH_DELAY_MIN + 1) as u32;
                ctx.illager.update_path_delay = PATH_DELAY_MIN + ctx.random.next_int(spread) as i32;
            }
        } else {
            ctx.illager.update_path_delay = 0;
            ctx.navigation.stop();
        }
        let eye = target.position + DVec3::new(0.0, f64::from(target.eye_height), 0.0);
        ctx.look_control.set_look_at_with_limits(eye, 30.0, 30.0);
        match ctx.illager.crossbow {
            CrossbowState::Uncharged => {
                if !should_move {
                    start_using(ctx);
                    ctx.illager.crossbow = CrossbowState::Charging;
                    ctx.illager.charging = true;
                }
            }
            CrossbowState::Charging => {
                if !ctx.bow.using_item {
                    ctx.illager.crossbow = CrossbowState::Uncharged;
                }
                let ticks = if ctx.bow.using_item {
                    ctx.bow.ticks_using_item
                } else {
                    0
                };
                if ticks >= il::CROSSBOW_CHARGE {
                    ctx.illager.released = true;
                    ctx.illager.crossbow = CrossbowState::Charged;
                    ctx.illager.attack_delay =
                        ATTACK_DELAY_MIN + ctx.random.next_int(ATTACK_DELAY_SPREAD) as i32;
                    ctx.illager.charging = false;
                }
            }
            CrossbowState::Charged => {
                ctx.illager.attack_delay -= 1;
                if ctx.illager.attack_delay == 0 {
                    ctx.illager.crossbow = CrossbowState::ReadyToAttack;
                }
            }
            CrossbowState::ReadyToAttack => {
                if sees {
                    ctx.illager.shot = Some(CrossbowShot {
                        target: target.target,
                        position: target.position,
                        height: target.height,
                    });
                    ctx.illager.crossbow = CrossbowState::Uncharged;
                }
            }
        }
    }
}

#[derive(Clone)]
struct HoldGroundAttackGoal {
    radius: f32,
}

impl Goal<MonsterGoalContext> for HoldGroundAttackGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let hurt_by_player = matches!(ctx.hurt_by, Some((Target::Player(_), _)));
        !ctx.illager.has_raid
            && ctx.illager.patrolling
            && ctx.target().is_some()
            && !ctx.illager.aggressive
            && !hurt_by_player
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.navigation.stop();
        if let Some(target) = ctx.target {
            ctx.illager.shout = Some((target, false));
        }
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(target) = ctx.target() {
            ctx.illager.shout = Some((target.target, true));
            ctx.illager.aggressive = true;
        }
    }
    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        let Some(target) = ctx.target() else {
            return;
        };
        let distance = ctx.body.position.distance_squared(target.position);
        if distance > f64::from(self.radius * self.radius) {
            let eye = target.position + DVec3::new(0.0, f64::from(target.eye_height), 0.0);
            ctx.look_control.set_look_at_with_limits(eye, 30.0, 30.0);
            if ctx.random.next_int(AMBIENT_ROLL) == 0 {
                ctx.illager.ambient = true;
            }
        } else {
            ctx.illager.aggressive = true;
        }
    }
}

#[derive(Clone)]
struct RandomStrollGoal {
    speed: f64,
}

impl Goal<MonsterGoalContext> for RandomStrollGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.no_action_time >= 100 || ctx.random.next_int(60) != 0 {
            return false;
        }
        let profile = ctx.walk.clone();
        ctx.stroll.wanted = crate::stroll::default_random_position(
            world,
            &profile,
            ctx.body.position,
            10,
            7,
            &mut ctx.random,
        );
        ctx.stroll.wanted.is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }
    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if let Some(target) = ctx.stroll.wanted {
            let profile = ctx.profile();
            let _ = navigate_walk_to(
                &ctx.body,
                &mut ctx.navigation,
                world,
                &profile,
                ctx.fluid,
                target,
                self.speed,
                1,
            );
        }
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.navigation.stop();
    }
}

#[derive(Clone)]
struct LookAtGoal {
    range: f64,
    probability: f32,
    players: bool,
}

impl LookAtGoal {
    fn nearest_player(&self, ctx: &mut MonsterGoalContext, world: &dyn World) -> Option<u64> {
        let (position, eye) = (ctx.body.position, ctx.eye());
        let mut best: Option<(f64, u64)> = None;
        for player in ctx.players.clone() {
            if !player.alive
                || player.spectator
                || position.distance_squared(player.position) > self.range * self.range
            {
                continue;
            }
            let Some(info) = ctx.info(Target::Player(player.id)) else {
                continue;
            };
            if !ctx.sees(world, info) {
                continue;
            }
            let distance = player.position.distance_squared(eye);
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, player.id));
            }
        }
        best.map(|(_, id)| id)
    }

    fn nearest_mob(&self, ctx: &mut MonsterGoalContext, world: &dyn World) -> Option<Target> {
        let (position, eye) = (ctx.body.position, ctx.eye());
        let half = f64::from(ctx.body.width / 2.0);
        let height = f64::from(ctx.body.height);
        let mut candidates: Vec<Target> = ctx
            .mobs
            .iter()
            .filter(|m| m.id != ctx.illager.id)
            .map(|m| Target::Mob(m.id))
            .collect();
        candidates.extend(
            ctx.villagers
                .iter()
                .filter(|v| ctx.mobs.iter().all(|m| m.id != v.id))
                .map(|v| Target::Villager(v.id)),
        );
        let mut best: Option<(f64, Target)> = None;
        for target in candidates {
            let Some(info) = ctx.info(target) else {
                continue;
            };
            let other_half = f64::from(info.width / 2.0);
            let q = info.position;
            let inside = q.x - other_half < position.x + half + self.range
                && q.x + other_half > position.x - half - self.range
                && q.y < position.y + height + 3.0
                && q.y + f64::from(info.height) > position.y - 3.0
                && q.z - other_half < position.z + half + self.range
                && q.z + other_half > position.z - half - self.range;
            if !inside || !info.alive {
                continue;
            }
            if position.distance_squared(q) > self.range.max(2.0).powi(2) || !ctx.sees(world, info)
            {
                continue;
            }
            let distance = q.distance_squared(eye);
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, target));
            }
        }
        best.map(|(_, target)| target)
    }
}

impl Goal<MonsterGoalContext> for LookAtGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.random.next_float() >= self.probability {
            return false;
        }
        if self.players {
            ctx.look_at_player.target_id = self.nearest_player(ctx, world);
            ctx.look_at_player.target_id.is_some()
        } else {
            ctx.illager.look_mob = self.nearest_mob(ctx, world);
            ctx.illager.look_mob.is_some()
        }
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        if self.players {
            return ctx.look_at_player.can_continue_within(
                ctx.body.position,
                &ctx.players,
                self.range,
            );
        }
        let Some(info) = ctx.illager.look_mob.and_then(|t| ctx.info(t)) else {
            return false;
        };
        info.alive
            && ctx.body.position.distance_squared(info.position) <= self.range * self.range
            && ctx.illager.look_mob_time > 0
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if self.players {
            ctx.look_at_player.start(&mut ctx.random);
        } else {
            let duration = 40 + ctx.random.next_int(40) as i32;
            ctx.illager.look_mob_time = (duration + 1) / 2;
        }
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if self.players {
            ctx.look_at_player.stop();
        } else {
            ctx.illager.look_mob = None;
        }
    }
    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        let x_max = ctx.max_head_x_rot();
        if self.players {
            if let Some(target) = ctx.look_at_player.tick(&ctx.players) {
                ctx.look_control
                    .set_look_at_with_limits(target, 10.0, x_max);
            }
            return;
        }
        let Some(info) = ctx.illager.look_mob.and_then(|t| ctx.info(t)) else {
            return;
        };
        if !info.alive {
            return;
        }
        let eye = info.position + DVec3::new(0.0, f64::from(info.eye_height), 0.0);
        ctx.look_control.set_look_at_with_limits(eye, 10.0, x_max);
        ctx.illager.look_mob_time -= 1;
    }
}

#[derive(Clone)]
struct RaiderHurtByGoal(HurtByTargetGoal);

impl Goal<MonsterGoalContext> for RaiderHurtByGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let raider = ctx
            .hurt_by
            .and_then(|(attacker, _)| ctx.kind_of(attacker))
            .is_some_and(il::is_raider);
        !raider && self.0.can_start(ctx, world)
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
