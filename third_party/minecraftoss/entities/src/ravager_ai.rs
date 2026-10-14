use crate::goals::GoalSelector;
use crate::illager_ai::{look_at, raider_goals, raider_hurt_by};
use crate::monster_ai::{
    FloatGoal, MeleeAttackGoal, MonsterGoalContext, NearestTargetGoal, Prey, StrollGoal,
};
use crate::ravager as rv;

pub const GOAL_NAMES: [&str; 10] = [
    "LongDistancePatrolGoal",
    "ObtainRaidLeaderBannerGoal",
    "PathfindToRaidGoal",
    "RaiderMoveThroughVillageGoal",
    "RaiderCelebration",
    "FloatGoal",
    "MeleeAttackGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "LookAtPlayerGoal",
];
pub const TARGET_NAMES: [&str; 4] = [
    "HurtByTargetGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
];
pub const STROLL_PROBABILITY: f32 = 0.001;

pub fn ravager_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = raider_goals();
    selector.add(0, FloatGoal);
    selector.add(
        4,
        MeleeAttackGoal {
            hits: true,
            raises_arms: false,
            follow_unseen: true,
            gives_up_in_light: false,
            speed: rv::MELEE_SPEED,
        },
    );
    selector.add(5, StrollGoal::new(rv::STROLL_SPEED, STROLL_PROBABILITY));
    selector.add(
        6,
        look_at(rv::PLAYER_LOOK_RANGE, rv::LOOK_PROBABILITY, true),
    );
    selector.add(10, look_at(rv::MOB_LOOK_RANGE, rv::LOOK_PROBABILITY, false));
    selector
}

pub fn ravager_targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(2, raider_hurt_by());
    selector.add(3, NearestTargetGoal::every(Prey::Player, true, 5));
    selector.add(4, NearestTargetGoal::every(Prey::Villager, true, 5));
    selector.add(4, NearestTargetGoal::every(Prey::Golem, true, 5));
    selector
}
