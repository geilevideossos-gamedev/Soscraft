use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{
    HurtByTargetGoal, LookAtPlayerGoal, MonsterGoalContext, RandomLookGoal, Target, continue_target,
};
use crate::wither as wt;
use glam::DVec3;
use minecraftoss_player::World;

pub const GOAL_NAMES: [&str; 5] = [
    "WitherDoNothingGoal",
    "RangedAttackGoal",
    "WaterAvoidingRandomFlyingGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
];
pub const TARGET_NAMES: [&str; 2] = ["HurtByTargetGoal", "NearestAttackableTargetGoal"];
pub const STUCK_CHECK_TICKS: i32 = 100;

type NodePos = (i32, i32, i32);

fn block_of(position: DVec3) -> NodePos {
    (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

#[derive(Clone, Debug, Default)]
pub struct FlightPath {
    pub node: Option<NodePos>,
    pub speed: f64,
    pub tick: i32,
    pub last_stuck_check: i32,
    pub stuck_check_position: DVec3,
}

impl FlightPath {
    pub fn move_to(&mut self, node: NodePos, speed: f64, position: DVec3) {
        self.node = Some(node);
        self.speed = speed;
        self.last_stuck_check = self.tick;
        self.stuck_check_position = position;
    }

    pub fn stop(&mut self) {
        self.node = None;
    }

    pub fn is_done(&self) -> bool {
        self.node.is_none()
    }

    pub fn tick(&mut self, position: DVec3, width: f32, mob_speed: f32) -> Option<(DVec3, f64)> {
        self.tick += 1;
        let node = self.node?;
        let reach = if width > 0.75 {
            f64::from(width / 2.0)
        } else {
            f64::from(0.75 - width / 2.0)
        };
        let close = (position.x - (f64::from(node.0) + 0.5)).abs() < reach
            && (position.z - (f64::from(node.2) + 0.5)).abs() < reach
            && (position.y - f64::from(node.1)).abs() < 1.0;
        if close {
            self.node = None;
        }
        if self.tick - self.last_stuck_check > STUCK_CHECK_TICKS {
            let speed = if mob_speed >= 1.0 {
                mob_speed
            } else {
                mob_speed * mob_speed
            };
            let least = speed * 100.0 * 0.25;
            if position.distance_squared(self.stuck_check_position) < f64::from(least * least) {
                self.node = None;
            }
            self.last_stuck_check = self.tick;
            self.stuck_check_position = position;
        }
        let node = self.node?;
        let center = ((width + 1.0) as i32) as f64 * 0.5;
        Some((
            DVec3::new(
                f64::from(node.0) + center,
                f64::from(node.1),
                f64::from(node.2) + center,
            ),
            self.speed,
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct WitherState {
    pub id: u64,
    pub charging: bool,
    pub flight: FlightPath,
    pub shot: Option<Target>,
    pub view: DVec3,
}

pub fn wither_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(0, DoNothingGoal);
    selector.add(
        2,
        RangedAttackGoal {
            target: None,
            attack_time: -1,
            see_time: 0,
        },
    );
    selector.add(5, FlyingStrollGoal { wanted: None });
    selector.add(
        6,
        LookAtPlayerGoal {
            range: wt::LOOK_RANGE,
        },
    );
    selector.add(7, RandomLookGoal);
    selector
}

pub fn wither_targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        HurtByTargetGoal {
            timestamp: 0,
            unseen_ticks: 0,
            target_mob: None,
            alert_others: false,
        },
    );
    selector.add(
        2,
        NearestLivingGoal {
            candidate: None,
            unseen_ticks: 0,
        },
    );
    selector
}

pub fn prey(ctx: &MonsterGoalContext) -> Vec<Target> {
    let me = ctx.wither.id;
    ctx.players
        .iter()
        .map(|p| Target::Player(p.id))
        .chain(
            ctx.villagers
                .iter()
                .filter(|v| v.id != me && ctx.mobs.iter().all(|m| m.id != v.id))
                .map(|v| Target::Villager(v.id)),
        )
        .chain(
            ctx.mobs
                .iter()
                .filter(|m| m.id != me && !wt::is_undead(m.kind))
                .map(|m| Target::Mob(m.id)),
        )
        .collect()
}

#[derive(Clone)]
struct DoNothingGoal;

impl Goal<MonsterGoalContext> for DoNothingGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Jump, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.wither.charging
    }
}

#[derive(Clone)]
struct RangedAttackGoal {
    target: Option<Target>,
    attack_time: i32,
    see_time: i32,
}

impl Goal<MonsterGoalContext> for RangedAttackGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let Some(target) = ctx.target().filter(|t| t.alive) else {
            return false;
        };
        self.target = Some(target.target);
        true
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if self.can_start(ctx, world) {
            return true;
        }
        let alive = self
            .target
            .and_then(|t| ctx.info(t))
            .is_some_and(|info| info.alive);
        alive && !ctx.wither.flight.is_done()
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn stop(&mut self, _ctx: &mut MonsterGoalContext) {
        self.target = None;
        self.see_time = 0;
        self.attack_time = -1;
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(info) = self.target.and_then(|t| ctx.info(t)) else {
            return;
        };
        let distance = ctx.body.position.distance_squared(info.position);
        let seen = ctx.sees(world, info);
        if seen {
            self.see_time += 1;
        } else {
            self.see_time = 0;
        }
        let radius = f64::from(wt::RANGED_RADIUS);
        if distance <= radius * radius && self.see_time >= 5 {
            ctx.wither.flight.stop();
        } else {
            let position = ctx.body.position;
            ctx.wither
                .flight
                .move_to(block_of(info.position), wt::RANGED_SPEED, position);
        }
        let eye = info.position + DVec3::new(0.0, f64::from(info.eye_height), 0.0);
        ctx.look_control.set_look_at_with_limits(eye, 30.0, 30.0);
        self.attack_time -= 1;
        if self.attack_time == 0 {
            if !seen {
                return;
            }
            ctx.wither.shot = Some(info.target);
            self.attack_time = wt::RANGED_INTERVAL;
        } else if self.attack_time < 0 {
            self.attack_time = wt::RANGED_INTERVAL;
        }
    }
}

#[derive(Clone)]
struct FlyingStrollGoal {
    wanted: Option<DVec3>,
}

impl Goal<MonsterGoalContext> for FlyingStrollGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.no_action_time >= 100 || ctx.random.next_int(wt::STROLL_INTERVAL) != 0 {
            return false;
        }
        let (position, view) = (ctx.body.position, ctx.wither.view);
        let profile = ctx.walk.clone();
        self.wanted =
            crate::stroll::hover_random_position(world, &profile, position, view, &mut ctx.random)
                .or_else(|| {
                    crate::stroll::air_and_water_random_position(
                        world,
                        &profile,
                        position,
                        view,
                        &mut ctx.random,
                    )
                });
        self.wanted.is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.wither.flight.is_done()
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(wanted) = self.wanted {
            let position = ctx.body.position;
            ctx.wither
                .flight
                .move_to(block_of(wanted), wt::STROLL_SPEED, position);
        }
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.wither.flight.stop();
    }
}

#[derive(Clone)]
struct NearestLivingGoal {
    candidate: Option<Target>,
    unseen_ticks: i32,
}

impl Goal<MonsterGoalContext> for NearestLivingGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Target])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let (range, eye) = (ctx.follow_range.max(2.0), ctx.eye());
        let mut best: Option<(f64, Target)> = None;
        for target in prey(ctx) {
            let Some(info) = ctx.info(target) else {
                continue;
            };
            if !ctx.can_attack(info)
                || ctx.body.position.distance_squared(info.position) > range * range
                || !ctx.sees(world, info)
            {
                continue;
            }
            let distance = info.position.distance_squared(eye);
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, target));
            }
        }
        self.candidate = best.map(|(_, target)| target);
        self.candidate.is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        continue_target(ctx, world, None, &mut self.unseen_ticks, None)
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.set_target(self.candidate);
        self.unseen_ticks = 0;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.clear_target();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flight_path_ends_within_half_its_width_and_a_block_of_height() {
        let mut path = FlightPath::default();
        assert!(path.is_done());
        path.move_to((10, 64, 0), 1.0, DVec3::ZERO);
        let (wanted, speed) = path.tick(DVec3::new(0.5, 64.0, 0.5), 0.9, 0.6).unwrap();
        assert_eq!((wanted, speed), (DVec3::new(10.5, 64.0, 0.5), 1.0));
        assert!(path.tick(DVec3::new(10.1, 64.5, 0.5), 0.9, 0.6).is_none());
        assert!(path.is_done());
    }

    #[test]
    fn a_flight_path_gives_up_when_it_moves_under_nine_blocks_in_a_hundred_ticks() {
        let mut path = FlightPath::default();
        path.move_to((50, 64, 0), 1.0, DVec3::new(0.0, 64.0, 0.0));
        for tick in 0..100 {
            let at = DVec3::new(tick as f64 * 0.05, 64.0, 0.0);
            assert!(path.tick(at, 0.9, 0.6).is_some(), "{tick}");
        }
        assert!(path.tick(DVec3::new(5.0, 64.0, 0.0), 0.9, 0.6).is_none());
    }
}
