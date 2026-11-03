use crate::feline_ai::panic_cause;
use crate::fluid::FluidFrame;
use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{AvoidEntityGoal, MonsterGoalContext};
use crate::navigation::navigate_walk_to;
use crate::panic::manhattan_ordered;
use crate::world::emissions::play_movement;
use crate::world::families::land::animal_goals::LAST_DAMAGE_TICKS;
use crate::world::families::water::swim::{block_of, random_position};
use crate::world::families::water::swimming::{
    RandomSwimmingGoal, SINK_WITHOUT_TARGET, SwimAcceleration, SwimControls, SwimExtra, SwimGoals,
    SwimLook, SwimMovement,
};
use crate::world::families::{Despawn, Family, MobCore, movement_sounds};
use glam::DVec3;
use minecraftoss_player::{Pos, World};
use std::marker::PhantomData;

pub const CATEGORY: &str = "water_ambient";
pub const MAX_HEALTH: f32 = 3.0;
pub const SPEED: f64 = 0.7;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const AMBIENT_INTERVAL: i32 = 120;
pub const SWIM_SOUND: &str = "entity.fish.swim";
pub const SPLASH: &str = "entity.generic.splash";
pub const ACCELERATION: f64 = 0.01_f32 as f64;
pub const MAX_HEAD_Y_ROT: f32 = 75.0;
pub const PANIC_SPEED: f64 = 1.25;
pub const PANIC_RANGE: (u32, u32) = (5, 4);
pub const WATER_SEARCH: (i32, i32) = (5, 1);
pub const AVOIDED: &str = "minecraft:player";
pub const AVOID_DISTANCE: f64 = 8.0;
pub const AVOID_WALK_SPEED: f64 = 1.6;
pub const AVOID_SPRINT_SPEED: f64 = 1.4;
pub const SWIM_SPEED: f64 = 1.0;
pub const SWIM_INTERVAL: u32 = 40;
pub const GOAL_NAMES: [&str; 3] = ["PanicGoal", "AvoidEntityGoal", "FishSwimGoal"];

pub const CONTROLS: SwimControls = SwimControls {
    movement: SwimMovement::Fish,
    look: SwimLook::Plain,
    acceleration: SwimAcceleration::Fixed(ACCELERATION),
    sink_without_target: true,
    max_head_y_rot: MAX_HEAD_Y_ROT,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FishGoals {
    pub swim: SwimGoals,
    pub last_damage: Option<&'static str>,
}

pub trait FishExtra: SwimExtra {
    fn fish(&self) -> &FishGoals;
    fn fish_mut(&mut self) -> &mut FishGoals;

    fn can_random_swim(&self) -> bool {
        true
    }
}

impl SwimExtra for FishGoals {
    fn swim(&self) -> &SwimGoals {
        &self.swim
    }

    fn swim_mut(&mut self) -> &mut SwimGoals {
        &mut self.swim
    }
}

impl FishExtra for FishGoals {
    fn fish(&self) -> &FishGoals {
        self
    }

    fn fish_mut(&mut self) -> &mut FishGoals {
        self
    }
}

pub fn last_damage(stamp: Option<(&'static str, i32)>, tick_count: i32) -> Option<&'static str> {
    stamp
        .filter(|&(_, at)| tick_count - at <= LAST_DAMAGE_TICKS)
        .map(|(damage_type, _)| damage_type)
}

pub fn despawn(from_bucket: bool) -> Despawn {
    Despawn {
        removable: !from_bucket,
        kept: from_bucket,
        ..Despawn::of_category(CATEGORY)
    }
}

pub fn look_for_water(world: &dyn World, position: DVec3) -> Option<Pos> {
    let feet = block_of(position);
    if !world.collision_boxes(feet).is_empty() {
        return None;
    }
    let (horizontal, vertical) = WATER_SEARCH;
    let depth = 2 * horizontal + vertical;
    manhattan_ordered(feet, horizontal, vertical, horizontal, depth)
        .into_iter()
        .find(|&pos| world.floatable_fluid(pos))
}

pub fn dying_swim<S: Family, W: World>(core: &mut MobCore, level: &W) {
    let sounds = movement_sounds::<S>(core.kind);
    let body = &mut core.body;
    body.trim_small_velocity();
    let fluid = FluidFrame::sample(level, body.position, body.width, body.height);
    if fluid.in_water() {
        body.travel_swimming(level, DVec3::ZERO, core.yaw, ACCELERATION);
        body.velocity.y -= SINK_WITHOUT_TARGET;
    } else if fluid.in_lava() {
        body.travel_lava(level, DVec3::ZERO, core.yaw, fluid.lava_height);
    } else {
        let _ = body.travel_air(level, DVec3::ZERO, 0.0, core.yaw);
    }
    let (tick_count, random, voices) = (core.tick_count, &mut core.random, &mut core.voices);
    play_movement(body, tick_count, random, voices, sounds);
}

pub fn add_goals<E: FishExtra>(selector: &mut GoalSelector<MonsterGoalContext>) {
    selector.add(0, FishPanicGoal::<E>::new(PANIC_SPEED));
    let avoid = AvoidEntityGoal::new(
        AVOIDED,
        AVOID_DISTANCE,
        AVOID_WALK_SPEED,
        AVOID_SPRINT_SPEED,
    );
    selector.add(2, avoid);
    selector.add(4, FishSwimGoal::<E>::new(SWIM_SPEED, SWIM_INTERVAL));
}

pub struct FishPanicGoal<E> {
    pub speed: f64,
    pub wanted: Option<DVec3>,
    extra: PhantomData<fn() -> E>,
}

impl<E> Clone for FishPanicGoal<E> {
    fn clone(&self) -> Self {
        Self {
            speed: self.speed,
            wanted: self.wanted,
            extra: PhantomData,
        }
    }
}

impl<E: FishExtra> FishPanicGoal<E> {
    pub fn new(speed: f64) -> Self {
        Self {
            speed,
            wanted: None,
            extra: PhantomData,
        }
    }
}

impl<E: FishExtra> Goal<MonsterGoalContext> for FishPanicGoal<E> {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let damage = ctx.extra.get::<E>().and_then(|e| e.fish().last_damage);
        if !damage.is_some_and(panic_cause) {
            return false;
        }
        let position = ctx.body.position;
        if ctx.on_fire {
            if let Some((x, y, z)) = look_for_water(world, position) {
                self.wanted = Some(DVec3::new(f64::from(x), f64::from(y), f64::from(z)));
                return true;
            }
        }
        let (horizontal, vertical) = PANIC_RANGE;
        let (profile, water) = (ctx.profile(), ctx.navigation.water);
        let random = &mut ctx.random;
        self.wanted = random_position(
            world, &profile, water, position, horizontal, vertical, None, random,
        );
        self.wanted.is_some()
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(target) = self.wanted else {
            return;
        };
        let profile = ctx.profile();
        let navigation = &mut ctx.navigation;
        let _ = navigate_walk_to(
            &ctx.body, navigation, world, &profile, ctx.fluid, target, self.speed, 1,
        );
    }
}

pub struct FishSwimGoal<E> {
    pub swim: RandomSwimmingGoal,
    extra: PhantomData<fn() -> E>,
}

impl<E> Clone for FishSwimGoal<E> {
    fn clone(&self) -> Self {
        Self {
            swim: self.swim.clone(),
            extra: PhantomData,
        }
    }
}

impl<E: FishExtra> FishSwimGoal<E> {
    pub fn new(speed: f64, interval: u32) -> Self {
        Self {
            swim: RandomSwimmingGoal::new(speed, interval),
            extra: PhantomData,
        }
    }
}

impl<E: FishExtra> Goal<MonsterGoalContext> for FishSwimGoal<E> {
    fn controls(&self) -> Controls {
        self.swim.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let free = ctx.extra.get::<E>().is_none_or(E::can_random_swim);
        free && self.swim.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.swim.can_continue(ctx, world)
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.swim.start_world(ctx, world);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.swim.stop(ctx);
    }
}
