use crate::fluid::FluidFrame;
use crate::goals::{Control, Controls, Goal};
use crate::monster_ai::{MonsterAi, MonsterGoalContext};
use crate::movement::Body;
use crate::navigation::navigate_walk_to;
use crate::tempt::PlayerCandidate;
use crate::world::emissions::play_movement;
use crate::world::families::goals::reduced_tick_delay;
use crate::world::families::water::swim::{
    FishMoveControl, SmoothSwimmingMoveControl, SwimFrame, block_of, random_swimmable_position,
};
use crate::world::{MonsterStep, Voice};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;
use std::any::Any;
use std::marker::PhantomData;

#[cfg(test)]
pub mod swim_probe;

pub const NEAR_PLAYER: f64 = 32.0;
pub const INPUT_FADE: f32 = 0.98;
pub const JUMP_THRESHOLD: f64 = 0.4;
pub const SINK_WITHOUT_TARGET: f64 = 0.005;
pub const IDLE_LIMIT: i32 = 100;
pub const SWIM_RANGE: (u32, u32) = (10, 7);
pub const FIND_WATER_REACH: f64 = 2.0;
pub const FIND_WATER_SPEED: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwimMovement {
    Smooth(SmoothSwimmingMoveControl),
    Fish,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwimLook {
    Smooth(f32),
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwimAcceleration {
    Fixed(f64),
    Speed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwimControls {
    pub movement: SwimMovement,
    pub look: SwimLook,
    pub acceleration: SwimAcceleration,
    pub sink_without_target: bool,
    pub max_head_y_rot: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwimGoals {
    pub up: f32,
    pub wanted: Option<DVec3>,
}

pub trait SwimExtra: Any + Clone + Send {
    fn swim(&self) -> &SwimGoals;
    fn swim_mut(&mut self) -> &mut SwimGoals;
}

impl SwimExtra for SwimGoals {
    fn swim(&self) -> &SwimGoals {
        self
    }

    fn swim_mut(&mut self) -> &mut SwimGoals {
        self
    }
}

pub trait Swimmer {
    const CONTROLS: SwimControls;
    type Extra: SwimExtra;
}

#[cfg_attr(not(test), allow(dead_code))]
#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn swim_ai_step<S: Swimmer, W: World>(
    ai: &mut MonsterAi,
    body: &mut Body,
    health: f32,
    random: &mut LegacyRandom,
    voices: &mut Vec<(Voice, DVec3)>,
    no_action_time: &mut i32,
    previous_position: DVec3,
    tick_count: i32,
    id: u64,
    world: &W,
    step: &MonsterStep,
) -> Option<f64> {
    let controls = S::CONTROLS;
    let position = body.position;
    let eye_height = ai.state.eye_height;
    let near = |p: &PlayerCandidate| {
        p.alive && !p.spectator && p.position.distance_squared(position) < NEAR_PLAYER * NEAR_PLAYER
    };
    if step.players.iter().any(near) {
        *no_action_time = 0;
    }
    if ai.no_jump_delay > 0 {
        ai.no_jump_delay -= 1;
    }
    body.trim_small_velocity();
    *no_action_time += 1;
    let fluid = FluidFrame::sample(world, position, body.width, body.height);
    ai.state.body = body.clone();
    ai.state.health = health;
    ai.state.game_time = step.game_time;
    ai.state.difficulty = step.difficulty;
    ai.state.players = step.players.to_vec();
    ai.state.villagers = step.villagers.to_vec();
    ai.state.fluid = fluid;
    ai.state.no_action_time = *no_action_time;
    ai.state.random = std::mem::take(random);
    let full = tick_count <= 1 || (tick_count + id as i32) % 2 == 0;
    ai.tick_goals(world, full);
    *random = std::mem::take(&mut ai.state.random);
    if let Some(velocity) = ai.state.leap.take() {
        body.velocity = velocity;
    }
    let swim = ai
        .state
        .extra
        .get_mut::<S::Extra>()
        .map(SwimExtra::swim_mut);
    let (wanted, up) = swim.map_or((None, 0.0), |swim| (swim.wanted.take(), swim.up));
    if let Some(wanted) = wanted {
        ai.move_control
            .set_wanted_position(wanted, FIND_WATER_SPEED);
    }
    let profile = ai.state.profile();
    let navigation = &mut ai.state.navigation;
    navigation.delayed_recompute(world, body, &profile, fluid, step.game_time);
    if let Some((wanted, speed)) = navigation.tick_water(world, body, fluid, ai.speed) {
        ai.move_control.set_wanted_position(wanted, speed);
    }
    ai.sideways *= INPUT_FADE;
    ai.forward *= INPUT_FADE;
    let navigating = !ai.state.navigation.is_done();
    let in_water = fluid.in_water();
    let look = &mut ai.state.look_control;
    let mut frame = SwimFrame {
        yaw: ai.yaw,
        pitch: look.pitch,
        body_yaw: ai.body_rotation.body_yaw,
        head_yaw: look.head_yaw,
        speed: ai.speed,
        sideways: ai.sideways,
        up,
        forward: ai.forward,
    };
    let (control, speed) = (&ai.move_control, step.movement_speed);
    match controls.movement {
        SwimMovement::Smooth(smooth) => {
            let velocity = &mut body.velocity;
            smooth.tick(
                control,
                !navigating,
                position,
                in_water,
                speed,
                &mut frame,
                velocity,
            );
        }
        SwimMovement::Fish => {
            let eyes = FluidFrame::eye_in_water(world, position, eye_height);
            let velocity = &mut body.velocity;
            FishMoveControl.tick(
                control,
                !navigating,
                position,
                eyes,
                speed,
                &mut frame,
                velocity,
            );
        }
    }
    ai.yaw = frame.yaw;
    ai.speed = frame.speed;
    ai.sideways = frame.sideways;
    ai.forward = frame.forward;
    ai.body_rotation.body_yaw = frame.body_yaw;
    look.pitch = frame.pitch;
    look.head_yaw = frame.head_yaw;
    let body_yaw = &mut ai.body_rotation.body_yaw;
    match controls.look {
        SwimLook::Smooth(max) => {
            look.tick_smooth_swimming(position, eye_height, body_yaw, navigating, max);
        }
        SwimLook::Plain => {
            let max = controls.max_head_y_rot;
            look.tick_limited(position, eye_height, *body_yaw, navigating, max);
        }
    }
    ai.jumping = ai.state.jump;
    body.living_jump(
        world,
        fluid,
        ai.jumping,
        &mut ai.no_jump_delay,
        JUMP_THRESHOLD,
    );
    let input = DVec3::new(
        f64::from(ai.sideways),
        f64::from(frame.up),
        f64::from(ai.forward),
    );
    let landed = if in_water {
        let acceleration = match controls.acceleration {
            SwimAcceleration::Fixed(acceleration) => acceleration,
            SwimAcceleration::Speed => f64::from(ai.speed),
        };
        body.travel_swimming(world, input, ai.yaw, acceleration);
        if controls.sink_without_target && ai.state.target().is_none() {
            body.velocity.y -= SINK_WITHOUT_TARGET;
        }
        None
    } else if fluid.in_lava() {
        body.travel_lava(world, input, ai.yaw, fluid.lava_height);
        None
    } else {
        body.travel_air_jumping(world, input, ai.speed, ai.yaw, ai.jumping)
    };
    if let Some(extra) = ai.state.extra.get_mut::<S::Extra>() {
        extra.swim_mut().up = frame.up;
    }
    play_movement(body, tick_count, random, voices, step.sounds);
    let (yaw, after) = (ai.yaw, body.position);
    let look = &mut ai.state.look_control;
    let max = controls.max_head_y_rot;
    ai.body_rotation
        .tick_limited(yaw, look, previous_position, after, max);
    landed
}

#[derive(Clone)]
pub struct RandomSwimmingGoal {
    pub speed: f64,
    pub interval: u32,
    pub wanted: Option<DVec3>,
}

impl RandomSwimmingGoal {
    pub fn new(speed: f64, interval: u32) -> Self {
        Self {
            speed,
            interval,
            wanted: None,
        }
    }
}

impl Goal<MonsterGoalContext> for RandomSwimmingGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.no_action_time >= IDLE_LIMIT
            || ctx.random.next_int(reduced_tick_delay(self.interval)) != 0
        {
            return false;
        }
        let (water, position) = (ctx.navigation.water, ctx.body.position);
        let (horizontal, vertical) = SWIM_RANGE;
        let random = &mut ctx.random;
        self.wanted = random_swimmable_position(
            world, &ctx.walk, water, position, horizontal, vertical, None, random,
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

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.navigation.stop();
    }
}

pub struct TryFindLiquidGoal<E> {
    extra: PhantomData<fn() -> E>,
}

impl<E> Clone for TryFindLiquidGoal<E> {
    fn clone(&self) -> Self {
        Self { extra: PhantomData }
    }
}

impl<E> Default for TryFindLiquidGoal<E> {
    fn default() -> Self {
        Self { extra: PhantomData }
    }
}

impl<E: SwimExtra> Goal<MonsterGoalContext> for TryFindLiquidGoal<E> {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let body = &ctx.body;
        body.on_ground && !world.floatable_fluid(block_of(body.position))
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let p = ctx.body.position;
        let reach = FIND_WATER_REACH;
        let low = block_of(p - DVec3::splat(reach));
        let high = block_of(p + DVec3::new(reach, 0.0, reach));
        let mut blocks = (low.2..=high.2).flat_map(|z| {
            (low.1..=high.1).flat_map(move |y| (low.0..=high.0).map(move |x| (x, y, z)))
        });
        let Some((x, y, z)) = blocks.find(|&pos| world.floatable_fluid(pos)) else {
            return;
        };
        if let Some(extra) = ctx.extra.get_mut::<E>() {
            let corner = DVec3::new(f64::from(x), f64::from(y), f64::from(z));
            extra.swim_mut().wanted = Some(corner);
        }
    }
}
