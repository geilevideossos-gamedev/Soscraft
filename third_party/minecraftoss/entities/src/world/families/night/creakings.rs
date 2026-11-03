mod brain;
#[cfg(test)]
mod tests;

pub use self::brain::{Activity, CreakingBrain, Ctx, Memories};
use crate::brain_behaviors::{Living, Pos, block_of};
use crate::control::MoveControl;
use crate::enderman::{PlayerView, normalize, view_vector};
use crate::fluid::FluidFrame;
use crate::health::DamageResult;
use crate::look::BodyRotation;
use crate::monster_ai::Target;
use crate::movement::Body;
use crate::navigation::GroundNavigation;
use crate::tempt::PlayerCandidate;
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::emissions::{MovementSounds, play_movement};
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, Registered};
use crate::world::living::fall_damage;
use crate::world::{DamageSourceKind, EntityWorld, MobSound, PlayerHitKind, Voice, voice_pitch};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::path_type::PathType;

pub const KIND: &str = "minecraft:creaking";
pub const KEY: u8 = 83;
pub const WIDTH: f32 = 0.9;
pub const HEIGHT: f32 = 2.7;
pub const EYE_HEIGHT: f32 = 2.3;
pub const MAX_HEALTH: f32 = 1.0;
pub const MOVEMENT_SPEED: f64 = 0.400_000_005_960_464_5;
pub const ATTACK_DAMAGE: f32 = 3.0;
pub const FOLLOW_RANGE: f64 = 32.0;
pub const STEP_HEIGHT: f32 = 1.0625;
pub const ACTIVATION_RANGE_SQ: f64 = 144.0;
pub const GAZE_TOLERANCE: f64 = 0.5;
pub const SIGHT_RANGE: f64 = 128.0;
pub const ATTACK_ANIMATION: i32 = 15;
pub const INVULNERABILITY: i32 = 8;
pub const TWITCH_DEATH: i32 = 45;
pub const MAX_PLAYER_STUCK: i32 = 4;
pub const ATTACK_EVENT: u8 = 4;
pub const STEP_SOUND: &str = "entity.creaking.step";
pub const SWAY_SOUND: &str = "entity.creaking.sway";
pub const HOSTILE_HURT_SOUND: &str = "entity.hostile.hurt";
pub const ACTIVATE_SOUND: &str = "entity.creaking.activate";
pub const DEACTIVATE_SOUND: &str = "entity.creaking.deactivate";
pub const FREEZE_SOUND: &str = "entity.creaking.freeze";
pub const UNFREEZE_SOUND: &str = "entity.creaking.unfreeze";
pub const TWITCH_SOUND: &str = "entity.creaking.twitch";
pub const DEATH_SOUND: &str = "entity.creaking.death";
pub const ENTITY_ACTION: &str = "minecraft:entity_action";
pub const BYPASSES_INVULNERABILITY: [&str; 2] =
    ["minecraft:out_of_world", "minecraft:generic_kill"];
pub const IS_FIRE: [&str; 8] = [
    "minecraft:in_fire",
    "minecraft:campfire",
    "minecraft:on_fire",
    "minecraft:lava",
    "minecraft:hot_floor",
    "minecraft:sulfur_cube_hot",
    "minecraft:unattributed_fireball",
    "minecraft:fireball",
];

#[derive(Clone)]
pub struct Creaking {
    pub home: Option<Pos>,
    pub can_move: bool,
    pub active: bool,
    pub tearing_down: bool,
    pub death_time: i32,
    pub attack_animation: i32,
    pub attack_tick: Option<i32>,
    pub invulnerable: i32,
    pub shudder_tick: Option<i32>,
    pub stuck: i32,
    pub eyes: Vec<DVec3>,
    pub linked: Option<bool>,
    pub heart_calls: u32,
    pub emitter: i32,
    pub brain: CreakingBrain,
    pub navigation: GroundNavigation,
    pub move_control: MoveControl,
    pub body_rotation: BodyRotation,
    pub profile: WalkProfile,
    pub fluid: FluidFrame,
    pub jumping: bool,
    pub no_jump_delay: i32,
    pub speed: f32,
    pub forward: f32,
    pub sideways: f32,
}

impl Creaking {
    pub fn heart_bound(&self) -> bool {
        self.home.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TearDown {
    Now,
    Killed { by: Option<Target> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreakingLink {
    pub id: u64,
    pub home: Pos,
    pub position: DVec3,
    pub centre: DVec3,
    pub persistent: bool,
    pub tearing_down: bool,
}

pub fn walk_profile() -> WalkProfile {
    let mut profile = WalkProfile::animal(WIDTH, HEIGHT);
    profile.max_up_step = STEP_HEIGHT;
    profile.follow_range = FOLLOW_RANGE as f32;
    profile.walk_target = WalkTarget::Zero;
    profile.clear_malus(PathType::FireInNeighbor);
    profile.clear_malus(PathType::Fire);
    profile
}

pub fn creaking(position: DVec3) -> FamilyMob<Creaking> {
    let mut core = MobCore::new(KIND, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
    core.body.step_height = STEP_HEIGHT;
    FamilyMob {
        core,
        state: Creaking {
            home: None,
            can_move: true,
            active: false,
            tearing_down: false,
            death_time: 0,
            attack_animation: 0,
            attack_tick: None,
            invulnerable: 0,
            shudder_tick: None,
            stuck: 0,
            eyes: Vec::new(),
            linked: None,
            heart_calls: 0,
            emitter: 0,
            brain: CreakingBrain::default(),
            navigation: GroundNavigation::default(),
            move_control: MoveControl::default(),
            body_rotation: BodyRotation::new(0.0),
            profile: walk_profile(),
            fluid: FluidFrame::default(),
            jumping: false,
            no_jump_delay: 0,
            speed: 0.0,
            forward: 0.0,
            sideways: 0.0,
        },
    }
}

pub fn bind(mob: &mut FamilyMob<Creaking>, home: Pos) {
    let profile = &mut mob.state.profile;
    profile.home = Some(home);
    profile.set_malus(PathType::Damaging, 8.0);
    profile.set_malus(PathType::PowderSnow, 8.0);
    profile.set_malus(PathType::Lava, 8.0);
    profile.set_malus(PathType::Fire, 0.0);
    profile.set_malus(PathType::FireInNeighbor, 0.0);
    mob.state.home = Some(home);
    mob.state.linked = None;
}

fn see_through(id: &str) -> bool {
    let id = id.strip_prefix("minecraft:").unwrap_or(id);
    matches!(id, "glass" | "tinted_glass" | "iron_bars" | "powder_snow")
        || id.ends_with("_stained_glass")
        || id.ends_with("glass_pane")
        || id.ends_with("copper_grate")
        || id.ends_with("copper_bars")
}

pub fn visual_boxes(world: &dyn World, pos: Pos) -> Vec<[f64; 6]> {
    let Some(block) = world.block(pos) else {
        return Vec::new();
    };
    if see_through(&block.id) {
        return Vec::new();
    }
    match block.id.as_str() {
        "minecraft:mud" | "minecraft:soul_sand" => vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
        "minecraft:snow" => {
            let layers = block
                .property("layers")
                .and_then(|layers| layers.parse::<f64>().ok());
            vec![[0.0, 0.0, 0.0, 1.0, layers.unwrap_or(1.0) * 2.0 / 16.0, 1.0]]
        }
        id if id.ends_with("_fence") => world
            .collision_boxes(pos)
            .into_iter()
            .map(|b| [b[0], b[1], b[2], b[3], b[4].min(1.0), b[5]])
            .collect(),
        _ => world.collision_boxes(pos),
    }
}

fn stops_sight(world: &dyn World, pos: Pos, from: DVec3, to: DVec3) -> bool {
    let local = visual_boxes(world, pos);
    if local.is_empty() {
        return false;
    }
    let d = to - from;
    if d.length_squared() < 1.0e-7 {
        return false;
    }
    let corner = DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    let test = DVec3::new(
        from.x + d.x * 0.001,
        from.y + d.y * 0.001,
        from.z + d.z * 0.001,
    ) - corner;
    let inside = |b: &[f64; 6]| {
        b[0] <= test.x
            && test.x < b[3]
            && b[1] <= test.y
            && test.y < b[4]
            && b[2] <= test.z
            && test.z < b[5]
    };
    if local.iter().any(inside) {
        return true;
    }
    local.iter().any(|b| {
        let min = corner + DVec3::new(b[0], b[1], b[2]);
        let max = corner + DVec3::new(b[3], b[4], b[5]);
        crate::sight::clip_box(min, max, from, to).is_some()
    })
}

pub fn visual_clip(world: &dyn World, from: DVec3, to: DVec3) -> bool {
    if from == to {
        return false;
    }
    let lerp = |delta: f64, a: f64, b: f64| a + delta * (b - a);
    let end = DVec3::new(
        lerp(-1.0e-7, to.x, from.x),
        lerp(-1.0e-7, to.y, from.y),
        lerp(-1.0e-7, to.z, from.z),
    );
    let start = DVec3::new(
        lerp(-1.0e-7, from.x, to.x),
        lerp(-1.0e-7, from.y, to.y),
        lerp(-1.0e-7, from.z, to.z),
    );
    let mut block = block_of(start);
    if stops_sight(world, block, from, to) {
        return true;
    }
    let d = end - start;
    let sign = |v: f64| {
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let step = [sign(d.x), sign(d.y), sign(d.z)];
    let delta = |s: i32, d: f64| if s == 0 { f64::MAX } else { f64::from(s) / d };
    let t_delta = [
        delta(step[0], d.x),
        delta(step[1], d.y),
        delta(step[2], d.z),
    ];
    let frac = |v: f64| v - v.floor();
    let first = |s: i32, v: f64, t: f64| t * if s > 0 { 1.0 - frac(v) } else { frac(v) };
    let mut t = [
        first(step[0], start.x, t_delta[0]),
        first(step[1], start.y, t_delta[1]),
        first(step[2], start.z, t_delta[2]),
    ];
    while t[0] <= 1.0 || t[1] <= 1.0 || t[2] <= 1.0 {
        let axis = if t[0] < t[1] {
            if t[0] < t[2] { 0 } else { 2 }
        } else if t[1] < t[2] {
            1
        } else {
            2
        };
        match axis {
            0 => block.0 += step[0],
            1 => block.1 += step[1],
            _ => block.2 += step[2],
        }
        t[axis] += t_delta[axis];
        if stops_sight(world, block, from, to) {
            return true;
        }
    }
    false
}

pub fn sees(world: &dyn World, from: DVec3, to: DVec3) -> bool {
    to.distance(from) <= SIGHT_RANGE && !visual_clip(world, from, to)
}

pub fn gaze_heights(body: &Body, eye_height: f32) -> [f64; 3] {
    let y = body.position.y;
    let eye = y + f64::from(eye_height);
    [eye, y + 0.5, (eye + y) / 2.0]
}

pub fn looked_at(
    world: &dyn World,
    player: &PlayerCandidate,
    view: PlayerView,
    body: &Body,
    eye_height: f32,
) -> bool {
    let eye = player.position + DVec3::Y * f64::from(player.eye_height);
    let look = normalize(view_vector(view.pitch, view.head_yaw));
    let me = body.position;
    gaze_heights(body, eye_height).into_iter().any(|y| {
        let dir = normalize(DVec3::new(me.x - eye.x, y - eye.y, me.z - eye.z));
        look.dot(dir) > 1.0 - GAZE_TOLERANCE && sees(world, eye, DVec3::new(me.x, y, me.z))
    })
}

fn play(core: &mut MobCore, event: &'static str) {
    let pitch = voice_pitch(&mut core.random, false);
    let at = core.body.position;
    core.voices.push((Voice::Event(event, 1.0, pitch), at));
}

fn act(core: &mut MobCore) {
    let at = core.body.position;
    core.body.game_events.push((ENTITY_ACTION, at, None));
}

fn activate(mob: &mut FamilyMob<Creaking>, player: Target) {
    mob.state.brain.memories.attack_target.set(player);
    act(&mut mob.core);
    play(&mut mob.core, ACTIVATE_SOUND);
    mob.state.active = true;
}

fn deactivate(mob: &mut FamilyMob<Creaking>) {
    mob.state.brain.memories.attack_target.erase();
    act(&mut mob.core);
    play(&mut mob.core, DEACTIVATE_SOUND);
    mob.state.active = false;
}

fn check_can_move(
    world: &dyn World,
    mob: &mut FamilyMob<Creaking>,
    players: &[PlayerCandidate],
    views: &[(u64, PlayerView)],
    peaceful: bool,
) -> bool {
    let nearest = mob.state.brain.memories.players();
    let active = mob.state.active;
    if nearest.is_empty() {
        if active {
            deactivate(mob);
        }
        return true;
    }
    let mut attackable = false;
    for target in nearest {
        let Target::Player(id) = target else {
            continue;
        };
        let Some(player) = players.iter().find(|player| player.id == id) else {
            continue;
        };
        if peaceful || !player.alive || player.spectator || !player.attackable {
            continue;
        }
        attackable = true;
        let Some(view) = views
            .iter()
            .find(|(seen, _)| *seen == id)
            .map(|(_, view)| *view)
        else {
            continue;
        };
        if active && view.disguised {
            continue;
        }
        if !looked_at(world, player, view, &mob.core.body, mob.core.eye_height) {
            continue;
        }
        if active {
            return false;
        }
        if player.position.distance_squared(mob.core.body.position) < ACTIVATION_RANGE_SQ {
            activate(mob, target);
            return false;
        }
    }
    if !attackable && active {
        deactivate(mob);
    }
    true
}

fn stop_in_place(mob: &mut FamilyMob<Creaking>) {
    mob.state.navigation.stop();
    mob.state.forward = 0.0;
    mob.state.sideways = 0.0;
    mob.state.speed = 0.0;
    mob.core.body.velocity = DVec3::ZERO;
}

fn ai_step(
    world: &dyn World,
    mob: &mut FamilyMob<Creaking>,
    players: &[PlayerCandidate],
    views: &[(u64, PlayerView)],
    peaceful: bool,
) {
    if mob.state.invulnerable > 0 {
        mob.state.invulnerable -= 1;
    }
    if mob.state.attack_animation > 0 {
        mob.state.attack_animation -= 1;
    }
    let before = mob.state.can_move;
    let now = check_can_move(world, mob, players, views, peaceful);
    if now != before {
        act(&mut mob.core);
        if now {
            play(&mut mob.core, UNFREEZE_SOUND);
        } else {
            stop_in_place(mob);
            play(&mut mob.core, FREEZE_SOUND);
        }
    }
    mob.state.can_move = now;
}

fn hold(mob: &mut FamilyMob<Creaking>) {
    if mob.state.home.is_none() {
        return;
    }
    if mob.state.tearing_down {
        mob.core.damage.death_ticks = 0;
    }
    if mob.state.linked == Some(false) && mob.core.health > 0.0 {
        mob.core.health = 0.0;
        mob.core.damage.dead = true;
    }
}

fn tear_down(world: &mut EntityWorld, id: u64) {
    let Some(mob) = world.creaking_mut(id) else {
        return;
    };
    let pitch = voice_pitch(&mut mob.core.random, false);
    let position = mob.core.body.position;
    mob.core.removed = true;
    world.sounds.push(MobSound {
        event: DEATH_SOUND.to_owned(),
        position,
        volume: 1.0,
        pitch,
        category: "hostile_volume",
    });
}

fn twitch(mob: &mut FamilyMob<Creaking>) -> bool {
    if mob.state.home.is_none() || !mob.state.tearing_down {
        return false;
    }
    mob.state.death_time += 1;
    mob.state.death_time > TWITCH_DEATH && !mob.core.removed
}

fn quiet(mob: &mut FamilyMob<Creaking>) {
    if mob.state.active {
        mob.core
            .voices
            .retain(|(voice, _)| !matches!(voice, Voice::Ambient(_)));
    }
}

fn watch(mob: &mut FamilyMob<Creaking>, players: &[PlayerCandidate]) {
    mob.state.eyes = mob
        .state
        .brain
        .memories
        .players()
        .into_iter()
        .filter_map(|target| match target {
            Target::Player(id) => players.iter().find(|player| player.id == id),
            Target::Mob(_) | Target::Villager(_) => None,
        })
        .map(|player| player.position + DVec3::Y * f64::from(player.eye_height))
        .collect();
}

fn movement_sounds() -> MovementSounds {
    MovementSounds::monster(Some(STEP_SOUND))
}

fn step<W: World>(
    mob: &mut FamilyMob<Creaking>,
    level: &W,
    players: &[PlayerCandidate],
    others: &[Living],
    time: i64,
    peaceful: bool,
) -> Option<Target> {
    let position = mob.core.body.position;
    if players
        .iter()
        .any(|p| p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0)
    {
        mob.core.no_action_time = 0;
    }
    let eye = (
        position.x.floor() as i32,
        (position.y + f64::from(mob.core.eye_height)).floor() as i32,
        position.z.floor() as i32,
    );
    if level.light_path_cost(eye) > 0.0 {
        mob.core.no_action_time += 2;
    }
    if mob.state.no_jump_delay > 0 {
        mob.state.no_jump_delay -= 1;
    }
    mob.core.body.trim_small_velocity();
    mob.core.no_action_time += 1;
    let FamilyMob { core, state } = &mut *mob;
    let fluid = FluidFrame::sample(level, position, core.body.width, core.body.height);
    state.fluid = fluid;
    let can_move = state.can_move;
    if can_move {
        let can_float = state.profile.can_float;
        let (can_update, surface) =
            crate::navigation::ground_view(level, &core.body, fluid, can_float);
        let (width, speed) = (core.body.width, state.speed);
        if let Some((wanted, modifier)) = state
            .navigation
            .tick_in(level, position, can_update, surface, width, speed)
        {
            state.move_control.set_wanted_position(wanted, modifier);
        }
    }
    let mut ctx = Ctx {
        body: &core.body,
        eye_height: core.eye_height,
        others,
        world: level,
        time,
        random: &mut core.random,
        navigation: &mut state.navigation,
        profile: &state.profile,
        fluid,
        can_move,
        active: state.active,
        peaceful,
        look_at: None,
        attack: None,
        jump: false,
    };
    state.brain.tick(&mut ctx);
    let (look_at, attack, swim) = (ctx.look_at, ctx.attack, ctx.jump);
    state.brain.update_activity(can_move);
    if let Some(at) = look_at {
        core.look.set_look_at(at);
    }
    state.sideways *= 0.98;
    state.forward *= 0.98;
    let mut leap = false;
    if can_move {
        let obstacle_top = crate::control::obstacle_top(level, position);
        let control = state.move_control.tick(
            position,
            core.body.on_ground,
            core.yaw,
            state.speed,
            state.forward,
            state.sideways,
            core.effects.movement_speed(MOVEMENT_SPEED),
            core.body.width,
            core.body.step_height,
            obstacle_top,
            |_, _| true,
        );
        core.yaw = control.yaw;
        state.speed = control.speed;
        state.forward = control.forward;
        state.sideways = control.sideways;
        leap = control.jump;
        let navigating = !state.navigation.is_done();
        core.look.tick(
            position,
            core.eye_height,
            state.body_rotation.body_yaw,
            navigating,
        );
    }
    state.jumping = can_move && (swim || leap);
    core.body.living_jump(
        level,
        fluid,
        state.jumping,
        &mut state.no_jump_delay,
        brain::FLUID_JUMP_THRESHOLD,
    );
    let input = DVec3::new(f64::from(state.sideways), 0.0, f64::from(state.forward));
    let landed = if fluid.in_water() {
        core.body.travel_water(level, input, core.yaw);
        None
    } else if fluid.in_lava() {
        core.body
            .travel_lava(level, input, core.yaw, fluid.lava_height);
        None
    } else {
        core.body
            .travel_air_jumping(level, input, state.speed, core.yaw, state.jumping)
    };
    play_movement(
        &mut core.body,
        core.tick_count,
        &mut core.random,
        &mut core.voices,
        movement_sounds(),
    );
    if can_move {
        state.body_rotation.tick(
            core.yaw,
            &mut core.look,
            core.previous_position,
            core.body.position,
        );
    }
    let fallen =
        landed.and_then(|fallen| fall_damage(&core.body, level, fallen, true, &mut core.voices));
    if let Some(damage) = fallen {
        let fall = Hurt::new(damage, "minecraft:fall", DamageSourceKind::Generic).bypassing_armor();
        <Creaking as Family>::hurt(mob, &fall);
    }
    attack
}

fn strike(world: &mut EntityWorld, id: u64, target: Target) {
    let Some(mob) = world.creaking_mut(id) else {
        return;
    };
    mob.state.attack_animation = ATTACK_ANIMATION;
    mob.state.attack_tick = Some(mob.core.tick_count);
    let position = mob.core.body.position;
    world.entity_events.push((id, ATTACK_EVENT));
    let kind = PlayerHitKind::Melee {
        attacker: position,
        hunger_ticks: 0,
        lift: 0.0,
        throw: DVec3::ZERO,
        disable_shield_seconds: 0.0,
    };
    world.family_strike(id, target, ATTACK_DAMAGE, kind);
}

impl Family for Creaking {
    const KEY: u8 = KEY;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(_kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        Some(creaking(position))
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let peaceful = world.difficulty == 0;
        if let Some(mob) = world.creaking_mut(id) {
            hold(mob);
        }
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        let views = world.player_views.clone();
        let Some(mob) = world.creaking_mut(id) else {
            return;
        };
        if twitch(mob) {
            tear_down(world, id);
        }
        let Some(mob) = world.creaking_mut(id).filter(|mob| !mob.core.removed) else {
            return;
        };
        quiet(mob);
        ai_step(&*level, mob, players, &views, peaceful);
        let mut attack = None;
        if base.alive && !base.no_ai {
            let others = world.hoglin_living(id, players);
            if let Some(mob) = world.creaking_mut(id) {
                attack = step(mob, &*level, players, &others, base.game_time, peaceful);
            }
        }
        if let Some(target) = attack {
            strike(world, id, target);
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
        if let Some(mob) = world.creaking_mut(id) {
            watch(mob, players);
        }
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        if mob.state.home.is_none() || BYPASSES_INVULNERABILITY.contains(&hurt.damage_type) {
            let result = mob.core.hurt_plain(hurt.amount);
            let sound = if mob.state.home.is_some() {
                SWAY_SOUND
            } else {
                HOSTILE_HURT_SOUND
            };
            for (voice, _) in &mut mob.core.voices {
                if let Voice::Hurt(pitch) = *voice {
                    *voice = Voice::Event(sound, 1.0, pitch);
                }
            }
            return result;
        }
        let refused = DamageResult {
            applied: false,
            dealt: 0.0,
            died: false,
            full: false,
        };
        let dying = mob.core.health <= 0.0 || mob.core.damage.dead;
        if IS_FIRE.contains(&hurt.damage_type) || mob.state.invulnerable > 0 || dying {
            return refused;
        }
        if hurt.attacker.is_none() {
            return refused;
        }
        mob.state.invulnerable = INVULNERABILITY;
        mob.state.shudder_tick = Some(mob.core.tick_count);
        act(&mut mob.core);
        if mob.state.linked != Some(false) {
            if matches!(hurt.attacker, Some(Target::Player(_))) {
                mob.state.heart_calls += 1;
            }
            play(&mut mob.core, SWAY_SOUND);
        }
        DamageResult {
            applied: true,
            dealt: 0.0,
            died: false,
            full: false,
        }
    }

    fn experience(_mob: &mut FamilyMob<Self>) -> i32 {
        0
    }

    fn fire_immune(mob: &FamilyMob<Self>) -> bool {
        mob.state.heart_bound()
    }

    fn enemy(_kind: &str) -> bool {
        true
    }

    fn pushable(mob: &FamilyMob<Self>) -> bool {
        mob.state.can_move
    }

    fn knockback_resistance(mob: &FamilyMob<Self>) -> f64 {
        if mob.state.can_move { 0.0 } else { 1.0 }
    }

    fn step_sound(_kind: &str) -> Option<&'static str> {
        Some(STEP_SOUND)
    }
}

impl EntityWorld {
    pub fn creakings(&self) -> &[FamilyMob<Creaking>] {
        self.family::<Creaking>()
    }

    pub fn creaking_mut(&mut self, id: u64) -> Option<&mut FamilyMob<Creaking>> {
        self.family_mob_mut::<Creaking>(id)
    }

    pub fn bind_creaking(&mut self, id: u64, home: Pos) -> bool {
        let Some(mob) = self.creaking_mut(id) else {
            return false;
        };
        bind(mob, home);
        true
    }

    pub fn link_creakings(&mut self, links: &[(u64, Pos, i32)]) {
        for mob in &mut <Creaking as Registered>::store_mut(self).mobs {
            let Some(home) = mob.state.home else {
                continue;
            };
            let id = mob.core.id;
            let link = links
                .iter()
                .find(|(linked, at, _)| *linked == id && *at == home);
            mob.state.linked = Some(link.is_some());
            mob.state.emitter = link.map_or(0, |(_, _, emitter)| *emitter);
        }
    }

    pub fn creaking_links(&self) -> Vec<CreakingLink> {
        self.creakings()
            .iter()
            .filter(|mob| !mob.core.removed)
            .filter_map(|mob| {
                let home = mob.state.home?;
                let body = &mob.core.body;
                let position = body.position;
                Some(CreakingLink {
                    id: mob.core.id,
                    home,
                    position,
                    centre: position + DVec3::Y * (f64::from(body.height) / 2.0),
                    persistent: mob.core.persistent,
                    tearing_down: mob.state.tearing_down,
                })
            })
            .collect()
    }

    pub fn creaking_player_stuck(&mut self, id: u64) -> bool {
        let Some(mob) = self.creaking_mut(id) else {
            return false;
        };
        let body = &mob.core.body;
        let half = f64::from(body.width) / 2.0;
        let min = body.position - DVec3::new(half, 0.0, half);
        let max = body.position + DVec3::new(half, f64::from(body.height), half);
        let inside = mob.state.eyes.iter().any(|eye| {
            eye.x >= min.x
                && eye.x < max.x
                && eye.y >= min.y
                && eye.y < max.y
                && eye.z >= min.z
                && eye.z < max.z
        });
        if !inside {
            mob.state.stuck = 0;
            return false;
        }
        mob.state.stuck += 1;
        mob.state.stuck > MAX_PLAYER_STUCK
    }

    pub fn take_creaking_heart_calls(&mut self) -> Vec<(u64, Pos)> {
        let mut calls = Vec::new();
        for mob in &mut <Creaking as Registered>::store_mut(self).mobs {
            if std::mem::take(&mut mob.state.heart_calls) == 0 {
                continue;
            }
            if let Some(home) = mob.state.home {
                calls.push((mob.core.id, home));
            }
        }
        calls
    }

    pub fn tear_down_creaking(&mut self, id: u64, how: TearDown) -> bool {
        let killer = match how {
            TearDown::Killed {
                by: Some(Target::Player(_)),
            } => Some("minecraft:player"),
            TearDown::Killed {
                by: Some(Target::Mob(by) | Target::Villager(by)),
            } => self.entity_type(by),
            _ => None,
        };
        let Some(mob) = self.creaking_mut(id) else {
            return false;
        };
        if mob.core.removed {
            return false;
        }
        match how {
            TearDown::Now => tear_down(self, id),
            TearDown::Killed { .. } => {
                let damage = &mut mob.core.damage;
                if damage.die() {
                    let burning = mob.core.body.fire_ticks > 0 && mob.state.home.is_none();
                    if let Some(death) = damage.death.as_mut() {
                        death.position = mob.core.body.position;
                        death.on_fire = burning;
                    }
                    if let Some(attacker) = killer {
                        let died = DamageResult {
                            applied: true,
                            dealt: 0.0,
                            died: true,
                            full: false,
                        };
                        damage.credit(died, attacker, true);
                    }
                }
                play(&mut mob.core, TWITCH_SOUND);
                mob.state.tearing_down = true;
                mob.core.health = 0.0;
            }
        }
        true
    }
}
