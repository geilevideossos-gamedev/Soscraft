use super::{
    SwimAcceleration, SwimControls, SwimGoals, SwimLook, SwimMovement, Swimmer, swim_ai_step,
};
use crate::goals::GoalSelector;
use crate::monster_ai::{MonsterAi, MonsterGoalContext, MonsterKind};
use crate::navigation::WaterNav;
use crate::tempt::PlayerCandidate;
use crate::world::EntityWorld;
use crate::world::emissions::play_movement;
use crate::world::families::water::swim::{SmoothSwimmingMoveControl, water_walk};
use crate::world::families::{
    AiStep, Breath, Family, FamilyMob, MobCore, MonsterSpec, movement_sounds,
};
use glam::DVec3;
use minecraftoss_player::World;

pub const KIND: &str = "minecraft:swim_probe";
pub const SILENT: &str = "minecraft:silent_swim_probe";
pub const SWIMMING: &str = "minecraft:swimming_probe";
pub const SWIM: &str = "entity.fish.swim";
pub const SILENT_SPLASH: &str = "entity.hostile.splash";
pub const WIDTH: f32 = 0.5;
pub const HEIGHT: f32 = 0.3;
pub const EYE_HEIGHT: f32 = 0.195;
pub const MAX_HEALTH: f32 = 3.0;
pub const SPEED: f64 = 0.7;
pub const FOLLOW_RANGE: f64 = 16.0;

pub const FISH: SwimControls = SwimControls {
    movement: SwimMovement::Fish,
    look: SwimLook::Plain,
    acceleration: SwimAcceleration::Fixed(0.01_f32 as f64),
    sink_without_target: true,
    max_head_y_rot: 75.0,
};

pub const SMOOTH: SwimControls = SwimControls {
    movement: SwimMovement::Smooth(SmoothSwimmingMoveControl::new(85, 10, 0.02, 0.1, true)),
    look: SwimLook::Smooth(10.0),
    acceleration: SwimAcceleration::Speed,
    sink_without_target: true,
    max_head_y_rot: 1.0,
};

pub struct Fish;

impl Swimmer for Fish {
    const CONTROLS: SwimControls = FISH;
    type Extra = SwimGoals;
}

pub struct Smooth;

impl Swimmer for Smooth {
    const CONTROLS: SwimControls = SMOOTH;
    type Extra = SwimGoals;
}

pub const GOAL_NAMES: [&str; 0] = [];

pub static SPEC: MonsterSpec = MonsterSpec {
    goal_names: &GOAL_NAMES,
    target_names: &[],
    goals: no_goals,
    targets: no_goals,
    eye_height: EYE_HEIGHT,
    follow_range: FOLLOW_RANGE,
    walk: water_walk,
};

fn no_goals() -> GoalSelector<MonsterGoalContext> {
    GoalSelector::default()
}

fn thinking(probe: &mut SwimProbe) -> &mut MonsterAi {
    probe.ai.as_deref_mut().expect("a swimming probe")
}

#[derive(Clone)]
pub struct SwimProbe {
    pub breath: Breath,
    pub input: DVec3,
    pub speed: f32,
    pub ai: Option<Box<MonsterAi>>,
    pub smooth: bool,
}

impl Family for SwimProbe {
    const KEY: u8 = 79;
    const KINDS: &'static [&'static str] = &[KIND, SILENT, SWIMMING];
    const OWN_DYING_TRAVEL: bool = true;

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let ai = (kind == SWIMMING).then(|| {
            let mut ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
            ai.state.navigation.water = WaterNav::Bound { breaching: false };
            ai.state.extra.set(SwimGoals::default());
            Box::new(ai)
        });
        let state = SwimProbe {
            breath: Breath::Gills,
            input: DVec3::ZERO,
            speed: 0.0,
            ai,
            smooth: false,
        };
        Some(FamilyMob { core, state })
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        let smooth = world
            .family_mob_mut::<Self>(id)
            .and_then(|mob| mob.state.ai.as_ref().map(|_| mob.state.smooth));
        if let Some(smooth) = smooth {
            if base.alive && !base.no_ai {
                let run: AiStep<W> = if smooth {
                    swim_ai_step::<Smooth, W>
                } else {
                    swim_ai_step::<Fish, W>
                };
                let _ = world
                    .family_goal_step_with::<Self, W>(id, level, players, SPEED, thinking, run);
            }
        } else if let Some(mob) = world
            .family_mob_mut::<Self>(id)
            .filter(|_| base.alive && !base.no_ai)
        {
            let sounds = movement_sounds::<Self>(mob.core.kind);
            let FamilyMob { core, state } = mob;
            let body = &mut core.body;
            body.trim_small_velocity();
            if body.touching_water {
                body.travel_water(&*level, state.input, core.yaw);
            } else {
                let _ = body.travel_air(&*level, state.input, state.speed, core.yaw);
            }
            play_movement(
                body,
                core.tick_count,
                &mut core.random,
                &mut core.voices,
                sounds,
            );
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn category(_kind: &str) -> &'static str {
        "water_ambient"
    }

    fn breath(mob: &FamilyMob<Self>) -> Breath {
        mob.state.breath
    }

    fn monster(kind: &str) -> Option<&'static MonsterSpec> {
        (kind == SWIMMING).then_some(&SPEC)
    }

    fn swim_sounds(kind: &str) -> Option<(&'static str, &'static str)> {
        if kind == SILENT {
            Some(("", SILENT_SPLASH))
        } else {
            Some((SWIM, "entity.generic.splash"))
        }
    }

    fn quiet_steps(kind: &str) -> bool {
        kind != SILENT
    }
}
