use super::{COD, PUFFERFISH, SALMON, TROPICAL_FISH};
use crate::goals::GoalSelector;
use crate::monster_ai::MonsterGoalContext;
use crate::world::families::MonsterSpec;
use crate::world::families::water::abstract_fish::{
    CONTROLS, FOLLOW_RANGE, FishExtra, FishGoals, GOAL_NAMES, add_goals,
};
use crate::world::families::water::swim::water_walk;
use crate::world::families::water::swimming::{SwimControls, SwimExtra, SwimGoals, Swimmer};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FishAi {
    pub fish: FishGoals,
}

impl SwimExtra for FishAi {
    fn swim(&self) -> &SwimGoals {
        &self.fish.swim
    }

    fn swim_mut(&mut self) -> &mut SwimGoals {
        &mut self.fish.swim
    }
}

impl FishExtra for FishAi {
    fn fish(&self) -> &FishGoals {
        &self.fish
    }

    fn fish_mut(&mut self) -> &mut FishGoals {
        &mut self.fish
    }
}

pub struct FishSwimmer;

impl Swimmer for FishSwimmer {
    const CONTROLS: SwimControls = CONTROLS;
    type Extra = FishAi;
}

static COD_SPEC: MonsterSpec = spec(0.195);
static SALMON_SPEC: MonsterSpec = spec(0.26);
static TROPICAL_SPEC: MonsterSpec = spec(0.26);
static PUFFERFISH_SPEC: MonsterSpec = spec(0.455);

const fn spec(eye_height: f32) -> MonsterSpec {
    MonsterSpec {
        goal_names: &GOAL_NAMES,
        target_names: &[],
        goals,
        targets: no_goals,
        eye_height,
        follow_range: FOLLOW_RANGE,
        walk: water_walk,
    }
}

pub fn spec_of(kind: &str) -> Option<&'static MonsterSpec> {
    match kind {
        COD => Some(&COD_SPEC),
        SALMON => Some(&SALMON_SPEC),
        TROPICAL_FISH => Some(&TROPICAL_SPEC),
        PUFFERFISH => Some(&PUFFERFISH_SPEC),
        _ => None,
    }
}

fn goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    add_goals::<FishAi>(&mut selector);
    selector
}

fn no_goals() -> GoalSelector<MonsterGoalContext> {
    GoalSelector::default()
}
