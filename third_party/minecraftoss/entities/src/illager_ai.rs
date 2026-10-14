use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::illager as il;
use crate::monster_ai::{
    AvoidEntityGoal, FloatGoal, HurtByTargetGoal, MeleeAttackGoal, MonsterGoalContext,
    NearestTargetGoal, NeverGoal, Prey, Target,
};
use crate::navigation::{navigate_walk_to, navigate_walk_to_entity};
use glam::DVec3;
use minecraftoss_player::{Pos, World};

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
pub const VINDICATOR_GOAL_NAMES: [&str; 14] = [
    "LongDistancePatrolGoal",
    "ObtainRaidLeaderBannerGoal",
    "PathfindToRaidGoal",
    "RaiderMoveThroughVillageGoal",
    "RaiderCelebration",
    "FloatGoal",
    "AvoidEntityGoal",
    "VindicatorBreakDoorGoal",
    "RaiderOpenDoorGoal",
    "HoldGroundAttackGoal",
    "MeleeAttackGoal",
    "RandomStrollGoal",
    "LookAtPlayerGoal",
    "LookAtPlayerGoal",
];
pub const VINDICATOR_TARGET_NAMES: [&str; 5] = [
    "HurtByTargetGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
    "VindicatorJohnnyAttackGoal",
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
    pub leader: bool,
    pub patrol_target: Option<Pos>,
    pub companions: Vec<u64>,
    pub patrol_orders: Option<Pos>,
    pub johnny: bool,
    pub reset_idle: bool,
    pub melee_aggressive: bool,
}

fn raider_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        4,
        LongDistancePatrolGoal {
            cooldown_until: -1,
        },
    );
    for priority in [1, 3, 4, 5] {
        selector.add(
            priority,
            NeverGoal {
                controls: &[Control::Move],
            },
        );
    }
    selector
}

pub fn vindicator_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = raider_goals();
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
        NeverGoal {
            controls: &[Control::Move],
        },
    );
    selector.add(3, NeverGoal { controls: &[] });
    selector.add(
        4,
        HoldGroundAttackGoal {
            radius: il::HOLD_GROUND_RADIUS,
        },
    );
    selector.add(
        5,
        MeleeAttackGoal {
            hits: true,
            raises_arms: false,
            follow_unseen: false,
            gives_up_in_light: false,
            speed: il::MELEE_SPEED,
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
            range: il::VINDICATOR_PLAYER_LOOK_RANGE,
            probability: 1.0,
            players: true,
        },
    );
    selector.add(
        10,
        LookAtGoal {
            range: il::VINDICATOR_MOB_LOOK_RANGE,
            probability: 0.02,
            players: false,
        },
    );
    selector
}

pub fn vindicator_targets() -> GoalSelector<MonsterGoalContext> {
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
    selector.add(3, NearestTargetGoal::every(Prey::Villager, true, 5));
    selector.add(3, NearestTargetGoal::every(Prey::Golem, true, 5));
    selector.add(
        4,
        JohnnyAttackGoal(NearestTargetGoal::every(Prey::Living, true, 0)),
    );
    selector
}

pub fn pillager_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = raider_goals();
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

pub fn find_patrol_target(from: Pos, random: &mut minecraftoss_player::rng::LegacyRandom) -> Pos {
    let x = from.0 - il::PATROL_TARGET_SPREAD + random.next_int(il::PATROL_TARGET_SPREAD as u32 * 2) as i32;
    let z = from.2 - il::PATROL_TARGET_SPREAD + random.next_int(il::PATROL_TARGET_SPREAD as u32 * 2) as i32;
    (x, from.1, z)
}

fn block_of(position: DVec3) -> Pos {
    (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

fn y_rot(v: DVec3, angle: f32) -> DVec3 {
    let cos = f64::from(minecraftoss_player::mth::cos(f64::from(angle)));
    let sin = f64::from(minecraftoss_player::mth::sin(f64::from(angle)));
    DVec3::new(v.x * cos + v.z * sin, v.y, v.z * cos - v.x * sin)
}

fn normalize(v: DVec3) -> DVec3 {
    let length = v.length();
    if length < 1.0e-5 {
        DVec3::ZERO
    } else {
        v / length
    }
}

#[derive(Clone)]
struct LongDistancePatrolGoal {
    cooldown_until: i64,
}

impl LongDistancePatrolGoal {
    fn move_to(ctx: &mut MonsterGoalContext, world: &dyn World, at: Pos, speed: f64) -> bool {
        let profile = ctx.profile();
        let target = DVec3::new(f64::from(at.0), f64::from(at.1), f64::from(at.2));
        navigate_walk_to(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            target,
            speed,
            1,
        )
        .unwrap_or(false)
    }

    fn move_randomly(ctx: &mut MonsterGoalContext, world: &dyn World) {
        let from = block_of(ctx.body.position);
        let spread = il::PATROL_WANDER as u32 * 2;
        let x = from.0 - il::PATROL_WANDER + ctx.random.next_int(spread) as i32;
        let z = from.2 - il::PATROL_WANDER + ctx.random.next_int(spread) as i32;
        let y = world.motion_blocking_no_leaves_height(x, z);
        Self::move_to(ctx, world, (x, y, z), il::PATROL_SPEED);
    }
}

impl Goal<MonsterGoalContext> for LongDistancePatrolGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let cooling = ctx.game_time < self.cooldown_until;
        ctx.illager.patrolling
            && ctx.target().is_none()
            && ctx.illager.patrol_target.is_some()
            && !cooling
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if !ctx.navigation.is_done() {
            return;
        }
        let leader = ctx.illager.leader;
        let Some(patrol_target) = ctx.illager.patrol_target else {
            return;
        };
        if ctx.illager.patrolling && ctx.illager.companions.is_empty() {
            ctx.illager.patrolling = false;
            return;
        }
        let position = ctx.body.position;
        let centre = DVec3::new(
            f64::from(patrol_target.0) + 0.5,
            f64::from(patrol_target.1) + 0.5,
            f64::from(patrol_target.2) + 0.5,
        );
        if leader && centre.distance_squared(position) < il::PATROL_ARRIVED * il::PATROL_ARRIVED {
            ctx.illager.patrol_target = Some(find_patrol_target(
                block_of(position),
                &mut ctx.random,
            ));
            ctx.illager.patrolling = true;
            return;
        }
        let bottom = DVec3::new(
            f64::from(patrol_target.0) + 0.5,
            f64::from(patrol_target.1),
            f64::from(patrol_target.2) + 0.5,
        );
        let swerved = y_rot(position - bottom, il::PATROL_SWERVE_ANGLE) * il::PATROL_SWERVE + bottom;
        let step = normalize(swerved - position) * il::PATROL_STEP + position;
        let (x, _, z) = block_of(step);
        let at = (x, world.motion_blocking_no_leaves_height(x, z), z);
        let speed = if leader {
            il::PATROL_LEADER_SPEED
        } else {
            il::PATROL_SPEED
        };
        if !Self::move_to(ctx, world, at, speed) {
            Self::move_randomly(ctx, world);
            self.cooldown_until = ctx.game_time + il::PATROL_COOLDOWN;
        } else if leader {
            ctx.illager.patrol_orders = Some(at);
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
struct JohnnyAttackGoal(NearestTargetGoal);

impl Goal<MonsterGoalContext> for JohnnyAttackGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        ctx.illager.johnny && self.0.can_start(ctx, world)
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world)
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.start(ctx);
        ctx.illager.reset_idle = true;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
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
