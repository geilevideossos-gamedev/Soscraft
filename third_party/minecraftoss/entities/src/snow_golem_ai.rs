use crate::goals::GoalSelector;
use crate::monster_ai::{
    LookAtPlayerGoal, MonsterGoalContext, NearestTargetGoal, Prey, RandomLookGoal,
    RangedAttackGoal, StrollGoal,
};
use crate::snow_golem as golem;
use glam::DVec3;

pub const GOAL_NAMES: [&str; 4] = [
    "RangedAttackGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
];
pub const TARGET_NAMES: [&str; 1] = ["NearestAttackableTargetGoal"];

#[derive(Clone, Debug, Default)]
pub struct SnowGolemState {
    pub shot: Option<(DVec3, f32, f32)>,
}

pub fn snow_golem_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        RangedAttackGoal {
            speed: golem::ATTACK_SPEED,
            interval_min: golem::ATTACK_INTERVAL,
            interval_max: golem::ATTACK_INTERVAL,
            radius: golem::ATTACK_RADIUS,
        },
    );
    selector.add(
        2,
        StrollGoal::new(golem::STROLL_SPEED, golem::STROLL_PROBABILITY),
    );
    selector.add(
        3,
        LookAtPlayerGoal {
            range: golem::LOOK_RANGE,
        },
    );
    selector.add(4, RandomLookGoal);
    selector
}

pub fn snow_golem_targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        NearestTargetGoal::every(Prey::Monster, true, golem::TARGET_INTERVAL),
    );
    selector
}
