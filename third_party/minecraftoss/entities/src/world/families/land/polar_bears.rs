mod ai;

use self::ai::SPEC;
pub use self::ai::{GOAL_NAMES, PolarBearGoals, TARGET_NAMES, WARNING_TICKS};
use crate::age::Age;
use crate::animal::{self, AnimalEvent};
use crate::cow::InteractionResult;
use crate::follow_parent::CowCandidate;
use crate::health::DamageResult;
use crate::monster_ai::{MonsterAi, MonsterKind, Target, update_enderman_anger};
use crate::navigation::navigate_walk_to;
use crate::tempt::PlayerCandidate;
use crate::world::families::land::animal_goals::LAST_DAMAGE_TICKS;
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, MonsterSpec, Registered};
use crate::world::{EntityWorld, PLAYER_TARGET, PlayerHitKind, Voice};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::inventory::Inventory;

pub const KIND: &str = "minecraft:polar_bear";
pub const WIDTH: f32 = 1.4;
pub const HEIGHT: f32 = 1.4;
pub const EYE_HEIGHT: f32 = HEIGHT * 0.85;
pub const BABY_WIDTH: f32 = 0.7;
pub const BABY_HEIGHT: f32 = 0.7;
pub const BABY_EYE_HEIGHT: f32 = 0.34375;
pub const MAX_HEALTH: f32 = 30.0;
pub const SPEED: f64 = 0.25;
pub const FOLLOW_RANGE: f64 = 20.0;
pub const ATTACK_DAMAGE: f32 = 6.0;
pub const AMBIENT_INTERVAL: i32 = 120;
pub const STAND_TICKS: i32 = 6;
pub const ALERT_RANGE_Y: f64 = 10.0;
pub const AMBIENT_BABY: &str = "entity.polar_bear.ambient_baby";
pub const WARNING_SOUND: &str = "entity.polar_bear.warning";
pub const STEP_SOUND: &str = "entity.polar_bear.step";

#[derive(Clone)]
pub struct PolarBear {
    pub age: Age,
    pub in_love: i32,
    pub last_damage: Option<(&'static str, i32)>,
    pub stand: i32,
    pub stand_old: i32,
    pub angry_at: Option<u128>,
    pub ai: Box<MonsterAi>,
}

impl PolarBear {
    pub fn goals(&self) -> Option<&PolarBearGoals> {
        self.ai.state.extra.get::<PolarBearGoals>()
    }

    pub fn goals_mut(&mut self) -> &mut PolarBearGoals {
        let extra = &mut self.ai.state.extra;
        if extra.get::<PolarBearGoals>().is_none() {
            extra.set(PolarBearGoals::default());
        }
        extra
            .get_mut::<PolarBearGoals>()
            .expect("the polar bear's goals")
    }

    pub fn standing(&self) -> bool {
        self.goals().is_some_and(|goals| goals.standing)
    }

    pub fn panicking(&self) -> bool {
        self.goals().is_some_and(|goals| goals.animal.panic.running)
    }

    pub fn running_goals(&self) -> Vec<&'static str> {
        self.ai.running_goals()
    }

    pub fn anger_end_time(&self) -> i64 {
        self.ai.state.enderman.anger_end_time
    }

    pub fn anger_target(&self) -> Option<Target> {
        self.ai.state.enderman.anger_target
    }
}

pub fn set_baby(mob: &mut FamilyMob<PolarBear>, baby: bool) {
    let (width, height, eye) = if baby {
        (BABY_WIDTH, BABY_HEIGHT, BABY_EYE_HEIGHT)
    } else {
        (WIDTH, HEIGHT, EYE_HEIGHT)
    };
    let core = &mut mob.core;
    core.baby = baby;
    core.body.width = width;
    core.body.height = height;
    core.eye_height = eye;
    let ai = &mut mob.state.ai;
    ai.state.walk.width = width;
    ai.state.walk.height = height;
    ai.state.eye_height = eye;
}

pub fn interact(
    world: &mut EntityWorld,
    id: u64,
    inventory: &mut Inventory,
    hand: usize,
    infinite: bool,
) -> Option<(InteractionResult, Vec<AnimalEvent>, DVec3)> {
    let mob = world.family_mob_mut::<PolarBear>(id)?;
    let position = mob.core.body.position;
    if !mob.core.alive() {
        return Some((InteractionResult::Pass, Vec::new(), position));
    }
    let state = &mut mob.state;
    let persistent = &mut mob.core.persistent;
    let (result, events) = animal::interact(
        &mut state.age,
        &mut state.in_love,
        persistent,
        inventory,
        hand,
        infinite,
        |_| false,
        false,
    );
    let baby = mob.state.age.baby();
    if baby != mob.core.baby {
        set_baby(mob, baby);
    }
    Some((result, events, position))
}

fn grow(mob: &mut FamilyMob<PolarBear>) {
    let was_baby = mob.state.age.baby();
    let _ = mob.state.age.tick(mob.core.alive());
    if was_baby != mob.state.age.baby() {
        set_baby(mob, !was_baby);
    }
    if mob.state.age.ticks != 0 {
        mob.state.in_love = 0;
    } else if mob.state.in_love > 0 {
        mob.state.in_love -= 1;
        if mob.state.in_love % 10 == 0 {
            for _ in 0..3 {
                let _ = mob.core.random.next_gaussian();
            }
            for _ in 0..3 {
                let _ = mob.core.random.next_double();
            }
        }
    }
}

fn kin(world: &EntityWorld, id: u64) -> Vec<CowCandidate> {
    world
        .family::<PolarBear>()
        .iter()
        .filter(|mob| mob.core.id != id && !mob.core.removed)
        .map(|mob| CowCandidate {
            id: mob.core.id,
            position: mob.core.body.position,
            width: mob.core.body.width,
            height: mob.core.body.height,
            age: mob.state.age.ticks,
            alive: mob.core.alive(),
            in_love: mob.state.in_love,
            panicking: mob.state.panicking(),
        })
        .collect()
}

fn uuid_of_target(world: &EntityWorld, target: Target) -> u128 {
    match target {
        Target::Player(player) => world.uuid_of(PLAYER_TARGET + player),
        Target::Mob(id) | Target::Villager(id) => world.uuid_of(id),
    }
}

fn saved_anger_target(world: &EntityWorld, id: u64, players: &[PlayerCandidate]) -> Option<Target> {
    let mob = world
        .family::<PolarBear>()
        .iter()
        .find(|m| m.core.id == id)?;
    if mob.state.anger_target().is_some() {
        return None;
    }
    let uuid = mob.state.angry_at?;
    let player = players
        .iter()
        .find(|player| world.uuid_of(PLAYER_TARGET + player.id) == uuid);
    if let Some(player) = player {
        return Some(Target::Player(player.id));
    }
    let other = world.id_of_uuid(uuid)?;
    (other != id && other < PLAYER_TARGET).then_some(Target::Mob(other))
}

fn swipe(world: &mut EntityWorld, id: u64, target: Target) {
    let Some(attacker) = world.family_core(id).map(|(_, core)| core.body.position) else {
        return;
    };
    let kind = PlayerHitKind::Melee {
        attacker,
        hunger_ticks: 0,
        lift: 0.0,
        throw: DVec3::ZERO,
        disable_shield_seconds: 0.0,
    };
    let _ = world.family_strike(id, target, ATTACK_DAMAGE, kind);
}

fn alert_adults(world: &mut EntityWorld, id: u64, attacker: Target) {
    let store = PolarBear::store_mut(world);
    let Some(source) = store.get(id) else {
        return;
    };
    let (p, range) = (
        source.core.body.position,
        source.state.ai.state.follow_range,
    );
    let min = p - DVec3::new(range, ALERT_RANGE_Y, range);
    let max = p + DVec3::new(1.0 + range, 1.0 + ALERT_RANGE_Y, 1.0 + range);
    for other in &mut store.mobs {
        if other.core.id == id || other.core.baby {
            continue;
        }
        let body = &other.core.body;
        let (q, half) = (body.position, f64::from(body.width / 2.0));
        let touches = q.x - half < max.x
            && q.x + half > min.x
            && q.y < max.y
            && q.y + f64::from(body.height) > min.y
            && q.z - half < max.z
            && q.z + half > min.z;
        let state = &mut other.state.ai.state;
        if touches && state.target().is_none() {
            state.set_target(Some(attacker));
        }
    }
}

fn update_anger(world: &mut EntityWorld, id: u64, game_time: i64) {
    let mobs = world.mob_candidates();
    let Some(mob) = world.family_mob_mut::<PolarBear>(id) else {
        return;
    };
    let state = &mut mob.state.ai.state;
    state.mobs = mobs;
    state.game_time = game_time;
    state.hurt_by = mob.core.hurt_by;
    state.random = std::mem::take(&mut mob.core.random);
    update_enderman_anger(state);
    mob.core.random = std::mem::take(&mut state.random);
    mob.core.hurt_by = state.hurt_by;
    let (held, end) = (state.enderman.anger_target, state.enderman.anger_end_time);
    let uuid = held.map(|target| uuid_of_target(world, target));
    let Some(mob) = world.family_mob_mut::<PolarBear>(id) else {
        return;
    };
    let angry = end > 0 && end - game_time > 0;
    mob.state.angry_at = uuid.or(mob.state.angry_at.filter(|_| angry));
}

impl Family for PolarBear {
    const KEY: u8 = 55;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let mut core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        core.ambient_interval = AMBIENT_INTERVAL;
        let mut ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        ai.state.extra.set(PolarBearGoals::default());
        Some(FamilyMob {
            core,
            state: PolarBear {
                age: Age::default(),
                in_love: 0,
                last_damage: None,
                stand: 0,
                stand_old: 0,
                angry_at: None,
                ai: Box::new(ai),
            },
        })
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
        if let Some(mob) = world.family_mob_mut::<Self>(id).filter(|mob| mob.core.baby) {
            for (voice, _) in &mut mob.core.voices {
                if let Voice::Ambient(pitch) = *voice {
                    *voice = Voice::Event(AMBIENT_BABY, 1.0, pitch);
                }
            }
        }
        if base.alive && !base.no_ai {
            let anger = saved_anger_target(world, id, players);
            let kin = kin(world, id);
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                let tick_count = mob.core.tick_count;
                let last_damage = mob
                    .state
                    .last_damage
                    .filter(|&(_, at)| tick_count - at <= LAST_DAMAGE_TICKS)
                    .map(|(damage_type, _)| damage_type);
                if anger.is_some() {
                    mob.state.ai.state.enderman.anger_target = anger;
                }
                let (baby, age) = (mob.core.baby, mob.state.age.ticks);
                let goals = mob.state.goals_mut();
                goals.baby = baby;
                goals.animal.id = id;
                goals.animal.age = age;
                goals.animal.last_damage = last_damage;
                goals.animal.kin = kin;
            }
            let strike =
                world.family_goal_step::<Self, W>(id, level, players, SPEED, |bear| &mut *bear.ai);
            let (alert, warn) = world
                .family_mob_mut::<Self>(id)
                .map_or((None, None), |mob| {
                    let alert = mob.state.ai.state.alert.take();
                    (alert, mob.state.goals_mut().warn.take())
                });
            if let Some(target) = strike {
                swipe(world, id, target);
            }
            if let Some(attacker) = alert {
                alert_adults(world, id, attacker);
            }
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                let position = mob.core.body.position;
                if let Some(pitch) = warn {
                    let voice = Voice::Event(WARNING_SOUND, 1.0, pitch);
                    mob.core.voices.push((voice, position));
                }
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            grow(mob);
            let standing = mob.state.standing();
            let state = &mut mob.state;
            state.stand_old = state.stand;
            let step = if standing { 1 } else { -1 };
            state.stand = (state.stand + step).clamp(0, STAND_TICKS);
            let goals = state.goals_mut();
            if goals.warning_ticks > 0 {
                goals.warning_ticks -= 1;
            }
        }
        update_anger(world, id, base.game_time);
    }

    fn category(_kind: &str) -> &'static str {
        "creature"
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        let result = mob.core.hurt_plain(hurt.amount);
        if result.applied {
            mob.state.in_love = 0;
            mob.state.last_damage = Some((hurt.damage_type, mob.core.tick_count));
        }
        result
    }

    fn leashable(_mob: &FamilyMob<Self>) -> bool {
        true
    }

    fn panicking(mob: &FamilyMob<Self>) -> bool {
        mob.state.panicking()
    }

    fn step_sound(_kind: &str) -> Option<&'static str> {
        Some(STEP_SOUND)
    }

    fn leash_walk<W: World>(mob: &mut FamilyMob<Self>, level: &W, target: DVec3, speed: f64) {
        let state = &mut mob.state.ai.state;
        let _ = navigate_walk_to(
            &mob.core.body,
            &mut state.navigation,
            level,
            &state.walk,
            state.fluid,
            target,
            speed,
            1,
        );
    }

    fn monster(kind: &str) -> Option<&'static MonsterSpec> {
        (kind == KIND).then_some(&SPEC)
    }
}

#[cfg(test)]
mod tests;
