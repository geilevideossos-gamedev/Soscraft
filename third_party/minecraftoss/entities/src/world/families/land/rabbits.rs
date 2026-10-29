mod ai;

pub use self::ai::{GOAL_NAMES, JUMP_DURATION, RabbitGoals, RabbitJump, TARGET_NAMES};
use self::ai::{SPEC, rabbit_ai_step};
use crate::age::Age;
use crate::animal::{self, AnimalEvent};
use crate::cow::InteractionResult;
use crate::health::DamageResult;
use crate::monster_ai::{MonsterAi, MonsterKind};
use crate::navigation::navigate_walk_to;
use crate::tempt::PlayerCandidate;
use crate::world::EntityWorld;
use crate::world::families::land::animal_goals::{LAST_DAMAGE_TICKS, Mate, Tempter};
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, MonsterSpec};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::inventory::Inventory;
use minecraftoss_player::rng::LegacyRandom;

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
pub const WHITE_RABBITS_TAG: &str = "minecraft:spawns_white_rabbits";
pub const GOLD_RABBITS_TAG: &str = "minecraft:spawns_gold_rabbits";
pub const PARENT_AGE: i32 = 6000;
pub const TOAST: &str = "Toast";
pub const KILLER_BUNNY: &str = "entity.minecraft.killer_bunny";
pub const BIOME_COLOUR_CHANCE: u32 = 20;
pub const FOOD: [&str; 3] = [
    "minecraft:carrot",
    "minecraft:golden_carrot",
    "minecraft:dandelion",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
    pub named: bool,
    pub toast: bool,
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

pub fn biome_variant(roll: i32, white: bool, gold: bool) -> RabbitVariant {
    if white {
        if roll < 80 {
            RabbitVariant::White
        } else {
            RabbitVariant::WhiteSplotched
        }
    } else if gold {
        RabbitVariant::Gold
    } else if roll < 50 {
        RabbitVariant::Brown
    } else if roll < 90 {
        RabbitVariant::Salt
    } else {
        RabbitVariant::Black
    }
}

pub fn feed(
    world: &mut EntityWorld,
    id: u64,
    inventory: &mut Inventory,
    hand: usize,
    infinite: bool,
) -> Option<(InteractionResult, Vec<AnimalEvent>, DVec3)> {
    let recipes = world.recipes.clone();
    let food = |item: &str| match &recipes {
        Some(recipes) => recipes.item_in_tag(FOOD_TAG, item),
        None => FOOD.contains(&item),
    };
    let mob = world.family_mob_mut::<Rabbit>(id)?;
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
        food,
        false,
    );
    let baby = mob.state.age.baby();
    if baby != mob.core.baby {
        set_baby(mob, baby);
    }
    if events.contains(&AnimalEvent::Hearts) {
        world.entity_events.push((id, 18));
    }
    Some((result, events, position))
}

fn breed<W: World>(world: &mut EntityWorld, id: u64, partner: u64, level: &W) {
    let mate = world
        .family::<Rabbit>()
        .iter()
        .find(|mob| mob.core.id == partner && mob.core.alive());
    let Some(partner_variant) = mate.map(|mob| mob.state.variant) else {
        return;
    };
    let Some(position) = world.family_core(id).map(|(_, core)| core.body.position) else {
        return;
    };
    let block = (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    let roll = world.level_random.next_int(100) as i32;
    let white = level.biome_in_tag(block, WHITE_RABBITS_TAG);
    let gold = !white && level.biome_in_tag(block, GOLD_RABBITS_TAG);
    let biome = biome_variant(roll, white, gold);
    let mob_drops = world.mob_drops;
    let Some(mob) = world.family_mob_mut::<Rabbit>(id) else {
        return;
    };
    let random = &mut mob.core.random;
    let variant = if random.next_int(BIOME_COLOUR_CHANCE) == 0 {
        biome
    } else if random.next_boolean() {
        partner_variant
    } else {
        mob.state.variant
    };
    mob.state.age.set(PARENT_AGE);
    mob.state.in_love = 0;
    let orb = mob_drops.then(|| 1 + mob.core.random.next_int(7) as i32);
    if let Some(other) = world.family_mob_mut::<Rabbit>(partner) {
        other.state.age.set(PARENT_AGE);
        other.state.in_love = 0;
    }
    world.entity_events.push((id, 18));
    if let Some(orb) = orb {
        world.trade_experience.push((position, orb));
    }
    let seed = world.born_seeds.pop_front().unwrap_or_else(|| {
        world.seed_uniquifier = world
            .seed_uniquifier
            .wrapping_mul(1_181_783_497_276_652_981);
        world.seed_uniquifier
    });
    let Some(mut child) = Rabbit::spawn(KIND, position) else {
        return;
    };
    child.state.age.set(Age::BABY_START);
    set_baby(&mut child, true);
    child.state.set_variant(variant);
    child.core.random = LegacyRandom::new(seed);
    world.spawn_family(child);
}

fn grow(mob: &mut FamilyMob<Rabbit>) {
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
                named: false,
                toast: false,
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
            let partner = world
                .family_mob_mut::<Self>(id)
                .and_then(|mob| mob.state.goals_mut().animal.breed_with.take());
            if let Some(partner) = partner {
                breed(world, id, partner, &*level);
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            grow(mob);
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
