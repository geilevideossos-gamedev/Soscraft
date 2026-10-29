mod ai;

pub use self::ai::{GOAL_NAMES, JUMP_DURATION, RabbitGoals, RabbitJump, TARGET_NAMES};
use self::ai::{SPEC, rabbit_ai_step};
use crate::age::Age;
use crate::health::DamageResult;
use crate::monster_ai::{MonsterAi, MonsterKind};
use crate::navigation::navigate_walk_to;
use crate::tempt::PlayerCandidate;
use crate::world::EntityWorld;
use crate::world::families::land::animal_goals::{LAST_DAMAGE_TICKS, Mate, Tempter};
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, MonsterSpec};
use glam::DVec3;
use minecraftoss_player::World;

pub const KIND: &str = "minecraft:rabbit";
pub const WIDTH: f32 = 0.49;
pub const HEIGHT: f32 = 0.6;
pub const EYE_HEIGHT: f32 = 0.59;
pub const BABY_WIDTH: f32 = 0.24;
pub const BABY_HEIGHT: f32 = 0.4;
pub const BABY_EYE_HEIGHT: f32 = 0.39;
pub const MAX_HEALTH: f32 = 3.0;
pub const SPEED: f64 = 0.30000001192092896;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const AMBIENT_INTERVAL: i32 = 120;
pub const FOOD_TAG: &str = "minecraft:rabbit_food";
pub const FOOD: [&str; 3] = [
    "minecraft:carrot",
    "minecraft:golden_carrot",
    "minecraft:dandelion",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RabbitVariant {
    #[default]
    Brown,
    White,
    Black,
    WhiteSplotched,
    Gold,
    Salt,
    Evil,
}

impl RabbitVariant {
    pub const ALL: [Self; 7] = [
        Self::Brown,
        Self::White,
        Self::Black,
        Self::WhiteSplotched,
        Self::Gold,
        Self::Salt,
        Self::Evil,
    ];

    pub fn id(self) -> i32 {
        match self {
            Self::Brown => 0,
            Self::White => 1,
            Self::Black => 2,
            Self::WhiteSplotched => 3,
            Self::Gold => 4,
            Self::Salt => 5,
            Self::Evil => 99,
        }
    }

    pub fn from_id(id: i32) -> Self {
        Self::ALL
            .into_iter()
            .find(|variant| variant.id() == id)
            .unwrap_or_default()
    }
}

#[derive(Clone)]
pub struct Rabbit {
    pub variant: RabbitVariant,
    pub age: Age,
    pub in_love: i32,
    pub last_damage: Option<(&'static str, i32)>,
    pub ai: Box<MonsterAi>,
}

impl Rabbit {
    pub fn goals(&self) -> Option<&RabbitGoals> {
        self.ai.state.extra.get::<RabbitGoals>()
    }

    pub fn goals_mut(&mut self) -> &mut RabbitGoals {
        let extra = &mut self.ai.state.extra;
        if extra.get::<RabbitGoals>().is_none() {
            extra.set(RabbitGoals::default());
        }
        extra.get_mut::<RabbitGoals>().expect("the rabbit's goals")
    }

    pub fn jump(&self) -> RabbitJump {
        self.goals()
            .map_or_else(RabbitJump::default, |goals| goals.jump)
    }

    pub fn panicking(&self) -> bool {
        self.goals().is_some_and(|goals| goals.animal.panic.running)
    }

    pub fn tempted(&self) -> bool {
        self.goals().is_some_and(|goals| goals.animal.tempted)
    }

    pub fn running_goals(&self) -> Vec<&'static str> {
        self.ai.running_goals()
    }

    pub fn set_variant(&mut self, variant: RabbitVariant) {
        self.variant = variant;
        self.goals_mut().evil = variant == RabbitVariant::Evil;
    }
}

pub fn set_baby(mob: &mut FamilyMob<Rabbit>, baby: bool) {
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

pub fn is_food(world: &EntityWorld, item: &str) -> bool {
    match &world.recipes {
        Some(recipes) => recipes.item_in_tag(FOOD_TAG, item),
        None => FOOD.contains(&item),
    }
}

fn tempters(world: &EntityWorld, players: &[PlayerCandidate]) -> Vec<Tempter> {
    players
        .iter()
        .map(|player| {
            let controls = world.player_controls(player.id);
            Tempter {
                id: player.id,
                sneaking: controls.is_some_and(|c| c.sneaking),
                holds: controls.is_some_and(|c| c.holds(|item| is_food(world, item))),
            }
        })
        .collect()
}

fn mates(world: &EntityWorld, id: u64) -> Vec<Mate> {
    world
        .family::<Rabbit>()
        .iter()
        .filter(|mob| mob.core.id != id && !mob.core.removed)
        .map(|mob| Mate {
            id: mob.core.id,
            position: mob.core.body.position,
            width: mob.core.body.width,
            height: mob.core.body.height,
            eye_height: mob.core.eye_height,
            alive: mob.core.alive(),
            in_love: mob.state.in_love > 0,
            panicking: mob.state.panicking(),
        })
        .collect()
}

impl Family for Rabbit {
    const KEY: u8 = 48;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let mut core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        core.ambient_interval = AMBIENT_INTERVAL;
        let mut ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        ai.state.extra.set(RabbitGoals::default());
        ai.move_control.set_wanted_position(DVec3::ZERO, 0.0);
        Some(FamilyMob {
            core,
            state: Rabbit {
                variant: RabbitVariant::Brown,
                age: Age::default(),
                in_love: 0,
                last_damage: None,
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
        if base.alive && !base.no_ai {
            let tempters = tempters(world, players);
            let mates = mates(world, id);
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                let tick_count = mob.core.tick_count;
                let last_damage = mob
                    .state
                    .last_damage
                    .filter(|&(_, at)| tick_count - at <= LAST_DAMAGE_TICKS)
                    .map(|(damage_type, _)| damage_type);
                let (in_love, baby) = (mob.state.in_love > 0, mob.core.baby);
                let goals = mob.state.goals_mut();
                goals.baby = baby;
                goals.animal.last_damage = last_damage;
                goals.animal.tempters = tempters;
                goals.animal.mates = mates;
                goals.animal.in_love = in_love;
            }
            world.family_goal_step_with::<Self, W>(
                id,
                level,
                players,
                SPEED,
                |rabbit| &mut *rabbit.ai,
                rabbit_ai_step::<W>,
            );
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
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
