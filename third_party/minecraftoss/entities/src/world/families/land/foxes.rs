mod ai;

pub use self::ai::{
    CROUCHING, DEFENDING, FACEPLANTED, FoxGoals, GOAL_NAMES, INTERESTED, POUNCING, SITTING,
    SLEEPING, TARGET_NAMES, has_shelter, walk_value,
};
use self::ai::{SPEC, fox_ai_step, held_still};
use crate::age::Age;
use crate::follow_parent::CowCandidate;
use crate::health::DamageResult;
use crate::monster_ai::{MonsterAi, MonsterKind};
use crate::movement::Body;
use crate::navigation::navigate_walk_to;
use crate::tempt::PlayerCandidate;
use crate::world::families::land::animal_goals::LAST_DAMAGE_TICKS;
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, MonsterSpec};
use crate::world::{EntityWorld, PLAYER_TARGET, Voice};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::rng::LegacyRandom;

pub const KIND: &str = "minecraft:fox";
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 0.7;
pub const EYE_HEIGHT: f32 = 0.4;
pub const BABY_SCALE: f32 = 0.6;
pub const BABY_WIDTH: f32 = WIDTH * BABY_SCALE;
pub const BABY_HEIGHT: f32 = HEIGHT * BABY_SCALE;
pub const BABY_EYE_HEIGHT: f32 = 0.34375;
pub const MAX_HEALTH: f32 = 10.0;
pub const SPEED: f64 = 0.30000001192092896;
pub const ATTACK_DAMAGE: f32 = 2.0;
pub const FOLLOW_RANGE: f64 = 32.0;
pub const SAFE_FALL_DISTANCE: f64 = 5.0;
pub const AMBIENT_INTERVAL: i32 = 120;
pub const DROP_CHANCE: f32 = 0.085;
pub const MAX_TRUSTED: usize = 2;
pub const ALERT_RANGE: f64 = 12.0;
pub const ALERT_RANGE_Y: f64 = 6.0;
pub const SCREECH_CHANCE: f32 = 0.1;
pub const SCREECH_VOLUME: f32 = 2.0;
pub const SCREECH_REACH: f64 = 16.0;
pub const AGGRO_CHANCE: f32 = 0.05;
pub const INTEREST_EASE: f32 = 0.4;
pub const CROUCH_STEP: f32 = 0.2;
pub const MAX_CROUCH_AMOUNT: f32 = 5.0;
pub const SPARED_BLOCKS: [&str; 1] = ["minecraft:sweet_berry_bush"];
pub const SLEEP_SOUND: &str = "entity.fox.sleep";
pub const SCREECH_SOUND: &str = "entity.fox.screech";
pub const AGGRO_SOUND: &str = "entity.fox.aggro";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FoxVariant {
    #[default]
    Red,
    Snow,
}

impl FoxVariant {
    pub fn id(self) -> i32 {
        match self {
            Self::Red => 0,
            Self::Snow => 1,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Snow => "snow",
        }
    }

    pub fn from_name(name: &str) -> Self {
        if name == Self::Snow.name() {
            Self::Snow
        } else {
            Self::Red
        }
    }
}

#[derive(Clone)]
pub struct Fox {
    pub variant: FoxVariant,
    pub trusted: Vec<u128>,
    pub held: Option<ItemStack>,
    pub drop_chance: f32,
    pub can_pick_up_loot: bool,
    pub ticks_since_eaten: i32,
    pub age: Age,
    pub in_love: i32,
    pub love_cause: Option<u128>,
    pub last_damage: Option<(&'static str, i32)>,
    pub interested_angle: f32,
    pub interested_angle_old: f32,
    pub crouch_amount: f32,
    pub crouch_amount_old: f32,
    pub ai: Box<MonsterAi>,
}

impl Fox {
    pub fn goals(&self) -> Option<&FoxGoals> {
        self.ai.state.extra.get::<FoxGoals>()
    }

    pub fn goals_mut(&mut self) -> &mut FoxGoals {
        let extra = &mut self.ai.state.extra;
        if extra.get::<FoxGoals>().is_none() {
            extra.set(FoxGoals::default());
        }
        extra.get_mut::<FoxGoals>().expect("the fox's goals")
    }

    pub fn flag(&self, flag: u8) -> bool {
        self.goals().is_some_and(|goals| goals.flag(flag))
    }

    pub fn set_flag(&mut self, flag: u8, on: bool) {
        self.goals_mut().set(flag, on);
    }

    pub fn flags(&self) -> u8 {
        self.goals().map_or(0, |goals| goals.flags)
    }

    pub fn sleeping(&self) -> bool {
        self.flag(SLEEPING)
    }

    pub fn sitting(&self) -> bool {
        self.flag(SITTING)
    }

    pub fn panicking(&self) -> bool {
        self.goals().is_some_and(|goals| goals.animal.panic.running)
    }

    pub fn running_goals(&self) -> Vec<&'static str> {
        self.ai.running_goals()
    }

    pub fn trusts(&self, uuid: u128) -> bool {
        self.trusted.contains(&uuid)
    }

    pub fn trust(&mut self, uuid: u128) {
        if self.trusted.len() < MAX_TRUSTED {
            self.trusted.push(uuid);
        } else {
            self.trusted[MAX_TRUSTED - 1] = uuid;
        }
    }

    pub fn step_angles(&mut self) {
        let (interested, crouching) = (self.flag(INTERESTED), self.flag(CROUCHING));
        self.interested_angle_old = self.interested_angle;
        let wanted = if interested { 1.0 } else { 0.0 };
        self.interested_angle += (wanted - self.interested_angle) * INTEREST_EASE;
        self.crouch_amount_old = self.crouch_amount;
        self.crouch_amount = if crouching {
            (self.crouch_amount + CROUCH_STEP).min(MAX_CROUCH_AMOUNT)
        } else {
            0.0
        };
    }
}

pub fn set_baby(mob: &mut FamilyMob<Fox>, baby: bool) {
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
    mob.state.goals_mut().baby = baby;
}

fn touches(min: DVec3, max: DVec3, q: DVec3, width: f32, height: f32) -> bool {
    let half = f64::from(width / 2.0);
    q.x - half < max.x
        && q.x + half > min.x
        && q.y < max.y
        && q.y + f64::from(height) > min.y
        && q.z - half < max.z
        && q.z + half > min.z
}

fn grown_box(body: &Body, across: f64, up: f64) -> (DVec3, DVec3) {
    let p = body.position;
    let half = f64::from(body.width / 2.0) + across;
    let min = DVec3::new(p.x - half, p.y - up, p.z - half);
    let max = DVec3::new(p.x + half, p.y + f64::from(body.height) + up, p.z + half);
    (min, max)
}

fn trusted_player(world: &EntityWorld, fox: &Fox, player: u64) -> bool {
    fox.trusts(world.uuid_of(PLAYER_TARGET + player))
}

fn alertable(world: &EntityWorld, id: u64, players: &[PlayerCandidate]) -> bool {
    let Some(mob) = world.family::<Fox>().iter().find(|mob| mob.core.id == id) else {
        return false;
    };
    let body = &mob.core.body;
    let (min, max) = grown_box(body, ALERT_RANGE, ALERT_RANGE_Y);
    let p = body.position;
    let near = |q: DVec3, width: f32, height: f32| {
        touches(min, max, q, width, height) && p.distance_squared(q) <= ALERT_RANGE * ALERT_RANGE
    };
    let peaceful = world.difficulty == 0;
    let alert_player = |player: &&PlayerCandidate| {
        let controls = world.player_controls(player.id);
        let discrete = controls.is_some_and(|c| c.sneaking || c.sleeping.is_some());
        player.alive
            && !player.spectator
            && player.attackable
            && !peaceful
            && !discrete
            && !trusted_player(world, &mob.state, player.id)
    };
    if players
        .iter()
        .filter(alert_player)
        .any(|player| near(player.position, 0.6, 1.8))
    {
        return true;
    }
    let mut awake = world
        .villagers
        .iter()
        .filter(|e| e.villager.health > 0.0 && e.sleeping.is_none());
    if awake.any(|e| {
        let body = &e.villager.body;
        near(body.position, body.width, body.height)
    }) {
        return true;
    }
    let tame = tame_pets(world);
    world.mob_candidates().iter().any(|other| {
        other.id != id
            && other.kind != KIND
            && other.alive
            && !tame.contains(&other.id)
            && near(other.position, other.width, other.height)
    })
}

fn tame_pets(world: &EntityWorld) -> Vec<u64> {
    let wolves = world.wolves.iter().filter(|e| e.wolf.owner.is_some());
    let cats = world.felines.iter().filter(|e| e.feline.owner.is_some());
    wolves.map(|e| e.id).chain(cats.map(|e| e.id)).collect()
}

fn unwary_players(world: &EntityWorld, id: u64, players: &[PlayerCandidate]) -> Vec<u64> {
    let Some(mob) = world.family::<Fox>().iter().find(|mob| mob.core.id == id) else {
        return Vec::new();
    };
    players
        .iter()
        .filter(|player| {
            let sneaking = world
                .player_controls(player.id)
                .is_some_and(|controls| controls.sneaking);
            sneaking || trusted_player(world, &mob.state, player.id)
        })
        .map(|player| player.id)
        .collect()
}

fn kin(world: &EntityWorld, id: u64) -> Vec<CowCandidate> {
    world
        .family::<Fox>()
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

fn player_within(body: &Body, players: &[PlayerCandidate], reach: f64) -> bool {
    let (min, max) = grown_box(body, reach, reach);
    players
        .iter()
        .any(|player| !player.spectator && touches(min, max, player.position, 0.6, 1.8))
}

fn ambient(mob: &mut FamilyMob<Fox>, bright: bool, players: &[PlayerCandidate]) {
    let sleeping = mob.state.sleeping();
    let MobCore {
        voices,
        random,
        body,
        ..
    } = &mut mob.core;
    for (voice, _) in voices.iter_mut() {
        let Voice::Ambient(pitch) = *voice else {
            continue;
        };
        if sleeping {
            *voice = Voice::Event(SLEEP_SOUND, 1.0, pitch);
        } else if !bright && screeches(random, body, players) {
            *voice = Voice::Event(SCREECH_SOUND, SCREECH_VOLUME, pitch);
        }
    }
}

fn screeches(random: &mut LegacyRandom, body: &Body, players: &[PlayerCandidate]) -> bool {
    random.next_float() < SCREECH_CHANCE && !player_within(body, players, SCREECH_REACH)
}

fn before_step(mob: &mut FamilyMob<Fox>) {
    mob.state.ticks_since_eaten += 1;
    let ai = &mob.state.ai.state;
    let target = ai.target.and_then(|target| ai.info(target));
    if target.is_none_or(|target| !target.alive) {
        let goals = mob.state.goals_mut();
        goals.set(CROUCHING, false);
        goals.set(INTERESTED, false);
    }
}

fn after_step(mob: &mut FamilyMob<Fox>) {
    if mob.state.flag(DEFENDING) && mob.core.random.next_float() < AGGRO_CHANCE {
        let voice = Voice::Event(AGGRO_SOUND, 1.0, 1.0);
        mob.core.voices.push((voice, mob.core.body.position));
    }
}

fn grow(mob: &mut FamilyMob<Fox>) {
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

fn wake(mob: &mut FamilyMob<Fox>, thundering: bool) {
    let wet = mob.core.body.touching_water;
    let target = mob.state.ai.state.target.is_some();
    let goals = mob.state.goals_mut();
    if wet || target || thundering {
        goals.set(SLEEPING, false);
    }
    if wet || goals.flag(SLEEPING) {
        goals.set(SITTING, false);
    }
}

fn lend_pois(world: &mut EntityWorld, id: u64) {
    let pois = std::mem::take(&mut world.pois);
    match world.family_mob_mut::<Fox>(id) {
        Some(mob) => mob.state.ai.state.pois = Some(pois),
        None => world.pois = pois,
    }
}

fn return_pois(world: &mut EntityWorld, id: u64) {
    let lent = world.family_mob_mut::<Fox>(id);
    if let Some(pois) = lent.and_then(|mob| mob.state.ai.state.pois.take()) {
        world.pois = pois;
    }
}

impl Family for Fox {
    const KEY: u8 = 49;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let mut core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        core.ambient_interval = AMBIENT_INTERVAL;
        let mut ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        ai.state.extra.set(FoxGoals::default());
        ai.still = Some(held_still);
        Some(FamilyMob {
            core,
            state: Fox {
                variant: FoxVariant::Red,
                trusted: Vec::new(),
                held: None,
                drop_chance: DROP_CHANCE,
                can_pick_up_loot: true,
                ticks_since_eaten: 0,
                age: Age::default(),
                in_love: 0,
                love_cause: None,
                last_damage: None,
                interested_angle: 0.0,
                interested_angle_old: 0.0,
                crouch_amount: 0.0,
                crouch_amount_old: 0.0,
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
        let bright = world.bright_outside;
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            ambient(mob, bright, players);
            if base.alive {
                before_step(mob);
            }
        }
        if base.alive && !base.no_ai {
            let alertable = alertable(world, id, players);
            let unwary = unwary_players(world, id, players);
            let tame = tame_pets(world);
            let kin = kin(world, id);
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                let tick_count = mob.core.tick_count;
                let last_damage = mob
                    .state
                    .last_damage
                    .filter(|&(_, at)| tick_count - at <= LAST_DAMAGE_TICKS)
                    .map(|(damage_type, _)| damage_type);
                let (baby, age) = (mob.core.baby, mob.state.age.ticks);
                let goals = mob.state.goals_mut();
                goals.baby = baby;
                goals.alertable = alertable;
                goals.unwary = unwary;
                goals.tame = tame;
                goals.animal.id = id;
                goals.animal.age = age;
                goals.animal.last_damage = last_damage;
                goals.animal.kin = kin;
            }
            lend_pois(world, id);
            let _ = world.family_goal_step_with::<Self, W>(
                id,
                level,
                players,
                SPEED,
                |fox| &mut *fox.ai,
                fox_ai_step,
            );
            return_pois(world, id);
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                after_step(mob);
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
        let thundering = level.thundering();
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            grow(mob);
            wake(mob, thundering);
            mob.state.step_angles();
        }
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

    fn death_drops(mob: &mut FamilyMob<Self>) -> Vec<ItemStack> {
        mob.state.held.take().into_iter().collect()
    }

    fn spared_blocks(_mob: &FamilyMob<Self>) -> &'static [&'static str] {
        &SPARED_BLOCKS
    }

    fn leashable(_mob: &FamilyMob<Self>) -> bool {
        true
    }

    fn panicking(mob: &FamilyMob<Self>) -> bool {
        mob.state.panicking()
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
