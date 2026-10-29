use crate::control::{MoveControl, MoveOperation, minecraft_atan2, obstacle_top};
use crate::crops;
use crate::fluid::FluidFrame;
use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{
    AvoidEntityGoal, FloatGoal, HurtByTargetGoal, LookAtPlayerGoal, MeleeAttackGoal, MonsterAi,
    MonsterGoalContext, NearestTargetGoal, Prey, StrollGoal,
};
use crate::movement::Body;
use crate::navigation::{GroundNavigation, ground_view};
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::emissions::play_movement;
use crate::world::families::MonsterSpec;
use crate::world::families::goals::ClimbOnTopOfPowderSnowGoal;
use crate::world::families::land::animal_goals::{
    AnimalExtra, AnimalGoals, BreedGoal, MoveToBlockGoal, PanicGoal, TemptGoal, is_monster_class,
};
use crate::world::{MonsterStep, Voice};
use glam::DVec3;
use minecraftoss_player::minecraft_sin_cos;
use minecraftoss_player::path_type::PathType;
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Block, Pos, World};

pub const STROLL_SPEED: f64 = 0.6;
pub const STROLL_PROBABILITY: f32 = 0.001;
pub const BREED_SPEED: f64 = 0.8;
pub const TEMPT_SPEED: f64 = 1.0;
pub const FLEE_SPEED: f64 = 2.2;
pub const WATER_SPEED: f64 = 1.5;
pub const SLOW_JUMP_SPEED: f64 = 0.6;
pub const ADULT_JUMP_HEIGHT: f64 = 1.5;
pub const BABY_JUMP_HEIGHT: f64 = 0.5;
pub const JUMP_DELAY: i32 = 10;
pub const PANIC_JUMP_DELAY: i32 = 3;
pub const JUMP_DURATION: i32 = 15;
pub const JUMP_STRENGTH: f32 = 0.42;
pub const JUMP_PUSH: f32 = 0.1;
pub const LOOK_RANGE: f64 = 10.0;
pub const AVOID_PLAYER: f64 = 8.0;
pub const AVOID_WOLF: f64 = 10.0;
pub const AVOID_MONSTER: f64 = 4.0;
pub const JUMP_SOUND: &str = "entity.rabbit.jump";
pub const RAID_SPEED: f64 = 0.699999988079071;
pub const RAID_RANGE: i32 = 16;
pub const RAID_VERTICAL_RANGE: i32 = 1;
pub const RAID_LOOK_Y: f32 = 10.0;
pub const RAID_NEXT_START: i32 = 10;
pub const MORE_CARROT_TICKS: i32 = 40;
pub const SUPPORTS_CROPS: &str = "minecraft:supports_crops";
pub const CARROTS: &str = "minecraft:carrots";
pub const EVIL_ATTACK_SPEED: f64 = 1.4;
pub const LEAP_DISTANCE: f64 = 4.0;
pub const WOLF: &str = "minecraft:wolf";

pub const GOAL_NAMES: [&str; 12] = [
    "FloatGoal",
    "ClimbOnTopOfPowderSnowGoal",
    "RabbitPanicGoal",
    "BreedGoal",
    "TemptGoal",
    "RabbitAvoidEntityGoal",
    "RabbitAvoidEntityGoal",
    "RabbitAvoidEntityGoal",
    "RaidGardenGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "MeleeAttackGoal",
];
pub const TARGET_NAMES: [&str; 3] = [
    "HurtByTargetGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
];

pub(super) static SPEC: MonsterSpec = MonsterSpec {
    goal_names: &GOAL_NAMES,
    target_names: &TARGET_NAMES,
    goals,
    targets,
    eye_height: super::EYE_HEIGHT,
    follow_range: super::FOLLOW_RANGE,
    walk,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RabbitJump {
    pub ticks: i32,
    pub duration: i32,
    pub was_on_ground: bool,
    pub delay: i32,
    pub next_speed: f64,
}

#[derive(Clone, Debug, Default)]
pub struct RabbitGoals {
    pub animal: AnimalGoals,
    pub evil: bool,
    pub baby: bool,
    pub more_carrot_ticks: i32,
    pub speed_modifier: Option<f64>,
    pub jump: RabbitJump,
    pub nibble: Option<(Pos, Option<Block>)>,
}

impl AnimalExtra for RabbitGoals {
    fn animal(&self) -> &AnimalGoals {
        &self.animal
    }

    fn animal_mut(&mut self) -> &mut AnimalGoals {
        &mut self.animal
    }
}

fn goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, FloatGoal);
    selector.add(1, ClimbOnTopOfPowderSnowGoal);
    selector.add(
        1,
        RabbitPanicGoal(PanicGoal::new(FLEE_SPEED, crate::feline_ai::panic_cause)),
    );
    selector.add(2, BreedGoal::<RabbitGoals>::new(BREED_SPEED));
    selector.add(3, TemptGoal::<RabbitGoals>::new(TEMPT_SPEED));
    let flee = |kind: &'static str, distance: f64| {
        AvoidEntityGoal::new(kind, distance, FLEE_SPEED, FLEE_SPEED)
    };
    selector.add(
        4,
        RabbitAvoidEntityGoal(flee("minecraft:player", AVOID_PLAYER)),
    );
    selector.add(4, RabbitAvoidEntityGoal(flee("minecraft:wolf", AVOID_WOLF)));
    selector.add(
        4,
        RabbitAvoidEntityGoal(flee("", AVOID_MONSTER).matching(is_monster_class)),
    );
    selector.add(5, RaidGardenGoal::new());
    selector.add(6, StrollGoal::new(STROLL_SPEED, STROLL_PROBABILITY));
    selector.add(11, LookAtPlayerGoal { range: LOOK_RANGE });
    selector.add(
        4,
        Evil(MeleeAttackGoal {
            hits: true,
            raises_arms: false,
            follow_unseen: true,
            gives_up_in_light: false,
            speed: EVIL_ATTACK_SPEED,
        }),
    );
    selector
}

fn targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        Evil(HurtByTargetGoal {
            timestamp: 0,
            unseen_ticks: 0,
            target_mob: None,
            alert_others: true,
        }),
    );
    selector.add(2, Evil(NearestTargetGoal::every(Prey::Player, true, 5)));
    selector.add(
        2,
        Evil(NearestTargetGoal::every(Prey::Kinds(&[WOLF]), true, 5)),
    );
    selector
}

pub fn evil(ctx: &MonsterGoalContext) -> bool {
    ctx.extra
        .get::<RabbitGoals>()
        .is_some_and(|goals| goals.evil)
}

#[derive(Clone)]
struct Evil<G>(G);

impl<G: Goal<MonsterGoalContext> + Clone + 'static> Goal<MonsterGoalContext> for Evil<G> {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        evil(ctx) && self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world)
    }

    fn interruptible(&self) -> bool {
        self.0.interruptible()
    }

    fn every_tick(&self) -> bool {
        self.0.every_tick()
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.start(ctx);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.tick(ctx);
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.0.start_world(ctx, world);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.0.tick_world(ctx, world);
    }
}

fn walk(walk: &mut WalkProfile) {
    walk.set_malus(PathType::FireInNeighbor, 16.0);
    walk.set_malus(PathType::Fire, -1.0);
    walk.walk_target = WalkTarget::Animal;
}

#[derive(Clone)]
struct RabbitPanicGoal(PanicGoal<RabbitGoals>);

impl Goal<MonsterGoalContext> for RabbitPanicGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world)
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.0.start_world(ctx, world);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(goals) = ctx.extra.get_mut::<RabbitGoals>() {
            goals.speed_modifier = Some(self.0.speed);
        }
    }
}

#[derive(Clone)]
struct RabbitAvoidEntityGoal(AvoidEntityGoal);

impl Goal<MonsterGoalContext> for RabbitAvoidEntityGoal {
    fn controls(&self) -> Controls {
        self.0.controls()
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        !evil(ctx) && self.0.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.0.can_continue(ctx, world)
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.0.start_world(ctx, world);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.stop(ctx);
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        self.0.tick(ctx);
    }
}

#[derive(Clone)]
pub(super) struct RaidGardenGoal {
    walk: MoveToBlockGoal,
    wants_to_raid: bool,
    can_raid: bool,
}

impl RaidGardenGoal {
    pub(super) fn new() -> Self {
        Self {
            walk: MoveToBlockGoal::new(RAID_SPEED, RAID_RANGE, RAID_VERTICAL_RANGE),
            wants_to_raid: false,
            can_raid: false,
        }
    }
}

pub fn ripe_carrot(block: Option<&Block>) -> bool {
    block.is_some_and(|block| block.id == CARROTS && crops::ripe(block))
}

pub fn is_garden_target(world: &dyn World, pos: Pos, wants_to_raid: bool, can_raid: bool) -> bool {
    let above = (pos.0, pos.1 + 1, pos.2);
    world.block_in_tag(pos, SUPPORTS_CROPS)
        && wants_to_raid
        && !can_raid
        && ripe_carrot(world.block(above).as_ref())
}

pub fn nibbled(carrot: &Block) -> Option<Block> {
    let age = crops::age(carrot);
    (age > 0).then(|| carrot.clone().with("age", &(age - 1).to_string()))
}

impl Goal<MonsterGoalContext> for RaidGardenGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Jump])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if self.walk.next_start_tick <= 0 {
            if !ctx.mob_griefing {
                return false;
            }
            self.can_raid = false;
            self.wants_to_raid = ctx
                .extra
                .get::<RabbitGoals>()
                .is_some_and(|goals| goals.more_carrot_ticks <= 0);
        }
        let (wants, can) = (self.wants_to_raid, self.can_raid);
        let found = self
            .walk
            .can_use(ctx, |pos| is_garden_target(world, pos, wants, can));
        if found {
            self.can_raid = true;
        }
        found
    }

    fn can_continue(&mut self, _ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let valid = is_garden_target(world, self.walk.block, self.wants_to_raid, self.can_raid);
        self.can_raid && self.walk.can_continue(valid)
    }

    fn every_tick(&self) -> bool {
        true
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.walk.start(ctx, world);
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.walk.tick(ctx, world);
        let above = self.walk.move_to_target();
        let look = DVec3::new(
            f64::from(above.0) + 0.5,
            f64::from(above.1),
            f64::from(above.2) + 0.5,
        );
        let x_max = ctx.max_head_x_rot();
        ctx.look_control
            .set_look_at_with_limits(look, RAID_LOOK_Y, x_max);
        if !self.walk.reached {
            return;
        }
        let crop = world.block(above).filter(|block| block.id == CARROTS);
        if let Some(crop) = crop.filter(|_| self.can_raid) {
            if let Some(goals) = ctx.extra.get_mut::<RabbitGoals>() {
                goals.nibble = Some((above, nibbled(&crop)));
                goals.more_carrot_ticks = MORE_CARROT_TICKS;
            }
        }
        self.can_raid = false;
        self.walk.next_start_tick = RAID_NEXT_START;
    }
}

pub(super) fn face_point(position: DVec3, point: DVec3) -> f32 {
    let (dx, dz) = (point.x - position.x, point.z - position.z);
    (minecraft_atan2(dz, dx) * 57.2957763671875) as f32 - 90.0
}

pub(super) fn next_entity_pos(navigation: &GroundNavigation, width: f32) -> Option<DVec3> {
    if navigation.is_done() {
        return None;
    }
    let node = navigation.nodes[navigation.next];
    let offset = f64::from((width + 1.0) as i32) * 0.5;
    Some(DVec3::new(
        f64::from(node.0) + offset,
        f64::from(node.1),
        f64::from(node.2) + offset,
    ))
}

fn set_wanted(
    control: &mut MoveControl,
    jump: &mut RabbitJump,
    wanted: DVec3,
    speed: f64,
    water: bool,
) {
    let speed = if water { WATER_SPEED } else { speed };
    control.set_wanted_position(wanted, speed);
    if speed > 0.0 {
        jump.next_speed = speed;
    }
}

pub(super) fn set_speed_modifier(
    ai: &mut MonsterAi,
    jump: &mut RabbitJump,
    speed: f64,
    water: bool,
) {
    ai.state.navigation.speed_modifier = speed;
    let wanted = ai.move_control.wanted;
    set_wanted(&mut ai.move_control, jump, wanted, speed, water);
}

pub(super) fn start_jumping(
    ai: &mut MonsterAi,
    jump: &mut RabbitJump,
    random: &mut LegacyRandom,
    voices: &mut Vec<(Voice, DVec3)>,
    position: DVec3,
) {
    ai.jumping = true;
    let pitch = ((random.next_float() - random.next_float()) * 0.2 + 1.0) * 0.8;
    voices.push((Voice::Event(JUMP_SOUND, 1.0, pitch), position));
    jump.duration = JUMP_DURATION;
    jump.ticks = 0;
}

pub(super) fn custom_step(
    ai: &mut MonsterAi,
    jump: &mut RabbitJump,
    carrots: &mut i32,
    body: &Body,
    random: &mut LegacyRandom,
    voices: &mut Vec<(Voice, DVec3)>,
) {
    if jump.delay > 0 {
        jump.delay -= 1;
    }
    if *carrots > 0 {
        *carrots = (*carrots - random.next_int(3) as i32).max(0);
    }
    if body.on_ground {
        if !jump.was_on_ground {
            ai.jumping = false;
            jump.delay = if ai.move_control.speed_modifier < FLEE_SPEED {
                JUMP_DELAY
            } else {
                PANIC_JUMP_DELAY
            };
        }
        let target = ai.state.target().map(|target| target.position);
        let leap = target.filter(|&target| {
            evil(&ai.state)
                && jump.delay == 0
                && body.position.distance_squared(target) < LEAP_DISTANCE * LEAP_DISTANCE
        });
        if let Some(target) = leap {
            ai.yaw = face_point(body.position, target);
            let speed = ai.move_control.speed_modifier;
            set_wanted(
                &mut ai.move_control,
                jump,
                target,
                speed,
                body.touching_water,
            );
            start_jumping(ai, jump, random, voices, body.position);
        }
        if !ai.state.jump && ai.move_control.has_wanted() && jump.delay == 0 {
            let wanted = ai.move_control.wanted;
            let point = next_entity_pos(&ai.state.navigation, body.width).unwrap_or(wanted);
            ai.yaw = face_point(body.position, point);
            start_jumping(ai, jump, random, voices, body.position);
        }
    }
    jump.was_on_ground = body.on_ground;
}

pub(super) fn input_vector(input: DVec3, speed: f32, yaw: f32) -> DVec3 {
    let length = input.length_squared();
    if length < 1.0e-7 {
        return DVec3::ZERO;
    }
    let unit = if length > 1.0 {
        input.normalize()
    } else {
        input
    };
    let movement = unit * f64::from(speed);
    let (sin, cos) = minecraft_sin_cos(f64::from(yaw));
    DVec3::new(
        movement.x * cos - movement.z * sin,
        movement.y,
        movement.z * cos + movement.x * sin,
    )
}

pub(super) fn jump_from_ground<W: World>(ai: &MonsterAi, body: &mut Body, world: &W, baby: bool) {
    let position = body.position;
    let mut strength = 0.3_f32;
    if ai.move_control.speed_modifier <= SLOW_JUMP_SPEED {
        strength = 0.2;
    }
    let next = next_entity_pos(&ai.state.navigation, body.width);
    if next.is_some_and(|next| next.y > position.y + 0.5) {
        strength = 0.5;
    }
    let rising = ai.jumping && ai.move_control.wanted.y > position.y + 0.5;
    if body.horizontal_collision || rising {
        strength = 0.5;
    }
    let power = body.jump_power(world, f64::from(JUMP_STRENGTH), strength / JUMP_STRENGTH);
    if power > 1.0e-5 {
        body.velocity.y = f64::from(power).max(body.velocity.y);
        body.needs_sync = true;
    }
    let v = body.velocity;
    if ai.move_control.speed_modifier > 0.0 && v.x * v.x + v.z * v.z < 0.01 {
        let height = if baby {
            BABY_JUMP_HEIGHT
        } else {
            ADULT_JUMP_HEIGHT
        };
        body.velocity += input_vector(DVec3::new(0.0, height, 1.0), JUMP_PUSH, ai.yaw);
    }
}

fn living_jump<W: World>(
    ai: &mut MonsterAi,
    body: &mut Body,
    world: &W,
    fluid: FluidFrame,
    baby: bool,
) {
    if !ai.jumping {
        ai.no_jump_delay = 0;
        return;
    }
    let threshold = if ai.state.eye_height < 0.4 { 0.0 } else { 0.4 };
    let height = if fluid.in_lava() {
        fluid.lava_height
    } else {
        fluid.water_height
    };
    let in_water = fluid.in_water() && height > 0.0;
    if in_water && !(body.on_ground && height <= threshold) {
        body.velocity.y += f64::from(0.04_f32);
    } else if fluid.in_lava() && !(body.on_ground && fluid.lava_height <= threshold) {
        body.velocity.y += f64::from(0.04_f32);
    } else if (body.on_ground || in_water && height <= threshold) && ai.no_jump_delay == 0 {
        jump_from_ground(ai, body, world, baby);
        ai.no_jump_delay = 10;
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn rabbit_ai_step<W: World>(
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
    let position = body.position;
    let eye_height = ai.state.eye_height;
    let near = |p: &crate::tempt::PlayerCandidate| {
        p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0
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
    let goals = ai.state.extra.get_mut::<RabbitGoals>();
    let (mut jump, mut carrots, request, baby) = match goals {
        Some(goals) => (
            goals.jump,
            goals.more_carrot_ticks,
            goals.speed_modifier.take(),
            goals.baby,
        ),
        None => (RabbitJump::default(), 0, None, false),
    };
    let water = body.touching_water;
    if let Some(speed) = request {
        set_speed_modifier(ai, &mut jump, speed, water);
    }
    let (can_update, surface) = ground_view(world, body, fluid, ai.state.walk.can_float);
    let navigation = &mut ai.state.navigation;
    if let Some((wanted, speed)) =
        navigation.tick_in(world, position, can_update, surface, body.width, ai.speed)
    {
        set_wanted(&mut ai.move_control, &mut jump, wanted, speed, water);
    }
    custom_step(ai, &mut jump, &mut carrots, body, random, voices);
    let obstacle_top = obstacle_top(world, position);
    ai.sideways *= 0.98;
    ai.forward *= 0.98;
    if body.on_ground && !ai.jumping && !ai.state.jump {
        set_speed_modifier(ai, &mut jump, 0.0, water);
    } else if ai.move_control.has_wanted() || ai.move_control.operation == MoveOperation::Jumping {
        let next = jump.next_speed;
        set_speed_modifier(ai, &mut jump, next, water);
    }
    let control = ai.move_control.tick(
        position,
        body.on_ground,
        ai.yaw,
        ai.speed,
        ai.forward,
        ai.sideways,
        step.movement_speed,
        body.width,
        body.step_height,
        obstacle_top,
        |_, _| true,
    );
    ai.yaw = control.yaw;
    ai.speed = control.speed;
    ai.forward = control.forward;
    ai.sideways = control.sideways;
    let navigating = !ai.state.navigation.is_done();
    let body_yaw = ai.body_rotation.body_yaw;
    ai.state
        .look_control
        .tick(position, eye_height, body_yaw, navigating);
    if ai.state.jump || control.jump {
        start_jumping(ai, &mut jump, random, voices, position);
    }
    living_jump(ai, body, world, fluid, baby);
    let input = DVec3::new(f64::from(ai.sideways), 0.0, f64::from(ai.forward));
    let landed = if fluid.in_water() {
        body.travel_water(world, input, ai.yaw);
        None
    } else if fluid.in_lava() {
        body.travel_lava(world, input, ai.yaw, fluid.lava_height);
        None
    } else {
        body.travel_air_jumping(world, input, ai.speed, ai.yaw, ai.jumping)
    };
    play_movement(body, tick_count, random, voices, step.sounds);
    ai.body_rotation.tick(
        ai.yaw,
        &mut ai.state.look_control,
        previous_position,
        body.position,
    );
    if jump.ticks != jump.duration {
        jump.ticks += 1;
    } else if jump.duration != 0 {
        jump.ticks = 0;
        jump.duration = 0;
        ai.jumping = false;
    }
    if let Some(goals) = ai.state.extra.get_mut::<RabbitGoals>() {
        goals.jump = jump;
        goals.more_carrot_ticks = carrots;
    }
    landed
}
