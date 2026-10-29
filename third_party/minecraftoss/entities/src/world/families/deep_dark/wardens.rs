mod anger;
mod brain;

pub use self::anger::{Anger, AngerLevel, Seen, Suspect};
pub use self::brain::{
    Activity, DIGGING_COOLDOWN, DIGGING_DURATION, EMERGE_DURATION, Memories, ROAR_DURATION,
    SNIFFING_DURATION, WardenBrain,
};

use crate::brain_behaviors::{Living, block_of, find};
use crate::control::MoveControl;
use crate::fluid::FluidFrame;
use crate::health::DamageResult;
use crate::look::{BodyRotation, LookControl};
use crate::monster_ai::Target;
use crate::navigation::GroundNavigation;
use crate::tempt::PlayerCandidate;
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::emissions::{MovementSounds, play_movement};
use crate::world::families::{Despawn, Family, FamilyMob, Hurt, MobCore};
use crate::world::living::fall_damage;
use crate::world::{DamageSourceKind, EntityWorld, PLAYER_TARGET, Rider, Voice, voice_pitch};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::path_type::PathType;

pub const KIND: &str = "minecraft:warden";
pub const KEY: u8 = 44;
pub const WIDTH: f32 = 0.9;
pub const HEIGHT: f32 = 2.9;
pub const EYE_HEIGHT: f32 = HEIGHT * 0.85;
pub const BURIED_HEIGHT: f32 = 1.0;
pub const BURIED_EYE_HEIGHT: f32 = BURIED_HEIGHT * 0.85;
pub const MAX_HEALTH: f32 = 500.0;
pub const MOVEMENT_SPEED: f64 = 0.300_000_011_920_928_96;
pub const KNOCKBACK_RESISTANCE: f64 = 1.0;
pub const ATTACK_KNOCKBACK: f64 = 1.5;
pub const ATTACK_DAMAGE: f32 = 30.0;
pub const FOLLOW_RANGE: f64 = 24.0;
pub const SOUND_VOLUME: f32 = 4.0;
pub const STEP_SOUND: &str = "entity.warden.step";
pub const STEP_VOLUME: f32 = 10.0;
pub const STEP_AFTER: f32 = 0.55;
pub const TENDRIL_EVENT: u8 = 61;
pub const TENDRIL_VOLUME: f32 = 5.0;
pub const PROJECTILE_ANGER: i32 = 10;
pub const PROJECTILE_ANGER_DISTANCE: f64 = 30.0;
pub const ANGER_TICK_INTERVAL: i32 = 20;
pub const BYPASSES_INVULNERABILITY: [&str; 2] =
    ["minecraft:out_of_world", "minecraft:generic_kill"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pose {
    #[default]
    Standing,
    Emerging,
    Digging,
    Roaring,
    Sniffing,
}

impl Pose {
    pub fn buried(self) -> bool {
        matches!(self, Self::Emerging | Self::Digging)
    }

    pub fn id(self) -> i32 {
        match self {
            Self::Standing => 0,
            Self::Emerging => 1,
            Self::Digging => 2,
            Self::Roaring => 3,
            Self::Sniffing => 4,
        }
    }

    pub fn of_id(id: i32) -> Self {
        match id {
            1 => Self::Emerging,
            2 => Self::Digging,
            3 => Self::Roaring,
            4 => Self::Sniffing,
            _ => Self::Standing,
        }
    }
}

#[derive(Clone)]
pub struct Warden {
    pub pose: Pose,
    pub pose_age: i32,
    pub anger: Anger,
    pub client_anger: i32,
    pub brain: WardenBrain,
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

impl Warden {
    pub fn activity(&self) -> Activity {
        self.brain.activity
    }

    pub fn memories(&self) -> &Memories {
        &self.brain.memories
    }

    pub fn memories_mut(&mut self) -> &mut Memories {
        &mut self.brain.memories
    }

    pub fn anger_level(&self) -> AngerLevel {
        brain::anger_level(self)
    }

    pub fn active_anger(&self) -> i32 {
        brain::active_anger(self)
    }
}

pub fn walk_profile() -> WalkProfile {
    let mut profile = WalkProfile::animal(WIDTH, HEIGHT);
    profile.follow_range = FOLLOW_RANGE as f32;
    profile.walk_target = WalkTarget::Zero;
    profile.distance_xz = true;
    profile.set_malus(PathType::UnpassableRail, 0.0);
    profile.set_malus(PathType::Damaging, 8.0);
    profile.set_malus(PathType::PowderSnow, 8.0);
    profile.set_malus(PathType::Lava, 8.0);
    profile.set_malus(PathType::Fire, 0.0);
    profile.set_malus(PathType::FireInNeighbor, 0.0);
    profile
}

pub fn warden(position: DVec3) -> FamilyMob<Warden> {
    let mut core = MobCore::new(KIND, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
    core.body.step_after = Some(STEP_AFTER);
    FamilyMob {
        core,
        state: Warden {
            pose: Pose::Standing,
            pose_age: 0,
            anger: Anger::default(),
            client_anger: 0,
            brain: WardenBrain::default(),
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

fn fit(core: &mut MobCore, pose: Pose) {
    let (height, eye) = if pose.buried() {
        (BURIED_HEIGHT, BURIED_EYE_HEIGHT)
    } else {
        (HEIGHT, EYE_HEIGHT)
    };
    core.body.height = height;
    core.eye_height = eye;
}

pub fn set_pose(mob: &mut FamilyMob<Warden>, pose: Pose) {
    brain::set_pose(mob, pose);
}

pub fn emerge(mob: &mut FamilyMob<Warden>) {
    set_pose(mob, Pose::Emerging);
    let memories = &mut mob.state.brain.memories;
    memories.dig_cooldown.set_for((), DIGGING_COOLDOWN);
    memories.is_emerging.set_for((), EMERGE_DURATION);
    mob.state.brain.activity = Activity::Emerge;
    let at = mob.core.body.position;
    let voice = Voice::Event("entity.warden.agitated", brain::BEHAVIOR_VOLUME, 1.0);
    mob.core.voices.push((voice, at));
}

fn movement_sounds() -> MovementSounds {
    MovementSounds::monster(Some(STEP_SOUND)).with_step_volume(STEP_VOLUME)
}

fn uuids(world: &EntityWorld, others: &[Living]) -> Vec<(Target, u128)> {
    others
        .iter()
        .map(|other| {
            let id = match other.target {
                Target::Player(player) => PLAYER_TARGET + player,
                Target::Mob(id) | Target::Villager(id) => id,
            };
            (other.target, world.uuid_of(id))
        })
        .collect()
}

fn speak(mob: &mut FamilyMob<Warden>) {
    let quiet = mob.state.pose == Pose::Roaring || mob.state.pose.buried();
    let ambient = mob.state.anger_level().ambient_sound();
    let said = std::mem::take(&mut mob.core.voices);
    mob.core.voices = said
        .into_iter()
        .filter_map(|(voice, at)| match voice {
            Voice::Ambient(_) if quiet => None,
            Voice::Ambient(pitch) => Some((Voice::Event(ambient, SOUND_VOLUME, pitch), at)),
            other => Some((other, at)),
        })
        .collect();
}

fn step<W: World>(
    mob: &mut FamilyMob<Warden>,
    level: &W,
    players: &[PlayerCandidate],
    ctx: &mut brain::Ctx,
) {
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
    let body = &mob.core.body;
    let fluid = FluidFrame::sample(level, position, body.width, body.height);
    mob.state.fluid = fluid;
    let (can_update, surface) =
        crate::navigation::ground_view(level, body, fluid, mob.state.profile.can_float);
    let width = body.width;
    let speed = mob.state.speed;
    if let Some((wanted, modifier)) = mob
        .state
        .navigation
        .tick_in(level, position, can_update, surface, width, speed)
    {
        mob.state.move_control.set_wanted_position(wanted, modifier);
    }
    brain::think(mob, ctx);
    if mob.core.tick_count % ANGER_TICK_INTERVAL == 0 {
        let others = ctx.others;
        let uuids = ctx.uuids;
        mob.state.anger.tick(
            |target| match find(others, target) {
                Some(other) => Seen::Here {
                    targetable: brain::can_target(other),
                },
                None => Seen::Gone,
            },
            |uuid| {
                uuids
                    .iter()
                    .find(|(_, known)| *known == uuid)
                    .map(|(target, _)| *target)
            },
        );
        mob.state.client_anger = mob.state.active_anger();
    }
    mob.state.brain.update_activity();
    if let Some(at) = ctx.look_at {
        mob.core.look.set_look_at(at);
    }
    let FamilyMob { core, state } = &mut *mob;
    let obstacle_top = crate::control::obstacle_top(level, position);
    state.sideways *= 0.98;
    state.forward *= 0.98;
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
    let navigating = !state.navigation.is_done();
    core.look.tick(
        position,
        core.eye_height,
        state.body_rotation.body_yaw,
        navigating,
    );
    state.jumping = ctx.jump || control.jump;
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
    state.body_rotation.tick(
        core.yaw,
        &mut core.look,
        core.previous_position,
        core.body.position,
    );
    let fallen =
        landed.and_then(|fallen| fall_damage(&core.body, level, fallen, true, &mut core.voices));
    if let Some(damage) = fallen {
        let fall = Hurt::new(damage, "minecraft:fall", DamageSourceKind::Generic).bypassing_armor();
        <Warden as Family>::hurt(mob, &fall);
    }
}

fn touch(world: &mut EntityWorld, id: u64, players: &[PlayerCandidate]) {
    let Some(mob) = world
        .family::<Warden>()
        .iter()
        .find(|mob| mob.core.id == id)
    else {
        return;
    };
    let core = &mob.core;
    if core.no_ai || !core.alive() || core.removed || mob.state.pose.buried() {
        return;
    }
    if mob.state.brain.memories.touch_cooldown.present() {
        return;
    }
    let body = &core.body;
    let (p, half) = (body.position, f64::from(body.width) / 2.0);
    let (min, max) = (
        p - DVec3::new(half, 0.0, half),
        p + DVec3::new(half, f64::from(body.height), half),
    );
    let others = world.hoglin_living(id, players);
    let touched = others.iter().find(|other| {
        let (q, h) = (other.position, f64::from(other.width) / 2.0);
        other.alive
            && !other.spectator
            && q.x - h < max.x
            && q.x + h > min.x
            && q.y < max.y
            && q.y + f64::from(other.height) > min.y
            && q.z - h < max.z
            && q.z + h > min.z
    });
    let Some(&touched) = touched else {
        return;
    };
    let uuids = uuids(world, &others);
    let uuid = uuids
        .iter()
        .find(|(target, _)| *target == touched.target)
        .map_or(0, |(_, uuid)| *uuid);
    let Some(mob) = world.family_mob_mut::<Warden>(id) else {
        return;
    };
    mob.state
        .brain
        .memories
        .touch_cooldown
        .set_for((), brain::TOUCH_COOLDOWN);
    brain::increase_anger_at(
        mob,
        &others,
        touched.target,
        uuid,
        brain::DEFAULT_ANGER,
        true,
    );
    brain::set_disturbance(mob, &others, block_of(touched.position));
}

impl Family for Warden {
    const KEY: u8 = KEY;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(_kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        Some(warden(position))
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            if mob.core.persistent {
                mob.state.brain.memories.set_dig_cooldown();
            }
        }
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            mob.state.pose_age += 1;
        }
        if base.alive && !base.no_ai {
            let others = world.hoglin_living(id, players);
            let uuids = uuids(world, &others);
            let passenger = world.vehicle_of(Rider::Mob(id)).is_some();
            let mut ctx = brain::Ctx {
                others: &others,
                uuids: &uuids,
                world: &*level,
                time: base.game_time,
                passenger,
                look_at: None,
                jump: false,
                unmount: false,
            };
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                step(mob, &*level, players, &mut ctx);
            }
            if ctx.unmount {
                let yaw = world.family_core(id).map_or(0.0, |(_, core)| core.yaw);
                world.stop_riding(Rider::Mob(id), yaw, &*level);
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
        touch(world, id, players);
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            speak(mob);
        }
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        if mob.state.pose.buried() && !BYPASSES_INVULNERABILITY.contains(&hurt.damage_type) {
            return DamageResult {
                applied: false,
                dealt: 0.0,
                died: false,
                full: false,
            };
        }
        mob.core.hurt_plain(hurt.amount)
    }

    fn despawn(_mob: &FamilyMob<Self>) -> Despawn {
        Despawn {
            removable: false,
            ..Despawn::of_category("monster")
        }
    }

    fn enemy(_kind: &str) -> bool {
        true
    }

    fn pushable(mob: &FamilyMob<Self>) -> bool {
        !mob.state.pose.buried()
    }

    fn knockback_resistance(_mob: &FamilyMob<Self>) -> f64 {
        KNOCKBACK_RESISTANCE
    }

    fn voice(_mob: &FamilyMob<Self>) -> (&'static str, f32, &'static str) {
        ("warden", SOUND_VOLUME, "hostile_volume")
    }

    fn step_sound(_kind: &str) -> Option<&'static str> {
        Some(STEP_SOUND)
    }
}

impl EntityWorld {
    pub fn wardens(&self) -> &[FamilyMob<Warden>] {
        self.family::<Warden>()
    }

    pub fn warden_mut(&mut self, id: u64) -> Option<&mut FamilyMob<Warden>> {
        self.family_mob_mut::<Warden>(id)
    }

    pub fn spawn_warden(&mut self, position: DVec3, yaw: f32) -> u64 {
        let mut mob = warden(position);
        mob.core.yaw = yaw;
        mob.core.look = LookControl::new(yaw);
        mob.state.body_rotation = BodyRotation::new(yaw);
        self.spawn_family(mob)
    }

    pub fn spawn_emerging_warden(&mut self, position: DVec3, yaw: f32) -> u64 {
        let id = self.spawn_warden(position, yaw);
        if let Some(mob) = self.warden_mut(id) {
            emerge(mob);
        }
        id
    }

    pub fn warden_can_hear(
        &self,
        id: u64,
        source: Option<Target>,
        players: &[PlayerCandidate],
    ) -> bool {
        let Some(mob) = self.wardens().iter().find(|mob| mob.core.id == id) else {
            return false;
        };
        let core = &mob.core;
        if core.no_ai || !core.alive() || core.removed || mob.state.pose.buried() {
            return false;
        }
        if mob.state.brain.memories.vibration_cooldown.present() {
            return false;
        }
        let Some(source) = source else {
            return true;
        };
        let others = self.hoglin_living(id, players);
        brain::targetable(&others, source)
    }

    pub fn warden_hears(
        &mut self,
        id: u64,
        at: (i32, i32, i32),
        source: Option<Target>,
        owner: Option<Target>,
        players: &[PlayerCandidate],
    ) {
        let others = self.hoglin_living(id, players);
        let uuids = uuids(self, &others);
        let uuid_of = |target: Target| {
            uuids
                .iter()
                .find(|(known, _)| *known == target)
                .map_or(0, |(_, uuid)| *uuid)
        };
        let Some(mob) = self.warden_mut(id) else {
            return;
        };
        if !mob.core.alive() {
            return;
        }
        mob.state
            .brain
            .memories
            .vibration_cooldown
            .set_for((), brain::VIBRATION_COOLDOWN);
        let pitch = voice_pitch(&mut mob.core.random, false);
        let position = mob.core.body.position;
        let voice = Voice::Event("entity.warden.tendril_clicks", TENDRIL_VOLUME, pitch);
        mob.core.voices.push((voice, position));
        let owner = owner.and_then(|owner| find(&others, owner)).copied();
        let source = source.filter(|&source| find(&others, source).is_some());
        let mut disturbance = at;
        match owner {
            Some(owner) => {
                let range = PROJECTILE_ANGER_DISTANCE * PROJECTILE_ANGER_DISTANCE;
                if owner.position.distance_squared(position) < range {
                    let uuid = uuid_of(owner.target);
                    if mob.state.brain.memories.recent_projectile.present() {
                        if brain::can_target(&owner) {
                            disturbance = block_of(owner.position);
                        }
                        let anger = brain::DEFAULT_ANGER;
                        brain::increase_anger_at(mob, &others, owner.target, uuid, anger, true);
                    } else {
                        brain::increase_anger_at(
                            mob,
                            &others,
                            owner.target,
                            uuid,
                            PROJECTILE_ANGER,
                            true,
                        );
                    }
                }
                mob.state
                    .brain
                    .memories
                    .recent_projectile
                    .set_for((), brain::RECENT_PROJECTILE);
            }
            None => {
                if let Some(source) = source {
                    let uuid = uuid_of(source);
                    brain::increase_anger_at(
                        mob,
                        &others,
                        source,
                        uuid,
                        brain::DEFAULT_ANGER,
                        true,
                    );
                }
            }
        }
        if !mob.state.anger_level().is_angry() {
            let active = mob
                .state
                .anger
                .top_suspect(|target| brain::targetable(&others, target));
            if owner.is_some() || active.is_none() || active == source {
                brain::set_disturbance(mob, &others, disturbance);
            }
        }
        self.entity_events.push((id, TENDRIL_EVENT));
    }
}

#[cfg(test)]
mod tests;
