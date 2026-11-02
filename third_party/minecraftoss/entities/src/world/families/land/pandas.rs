mod ai;

use self::ai::SPEC;
pub use self::ai::{GOAL_NAMES, PandaGoals, TARGET_NAMES, can_find_bamboo};
use crate::age::Age;
use crate::animal::{self, AnimalEvent};
use crate::cow::InteractionResult;
use crate::follow_parent::CowCandidate;
use crate::health::DamageResult;
use crate::monster_ai::{MonsterAi, MonsterKind, Target, look_at};
use crate::tempt::PlayerCandidate;
use crate::world::families::land::animal_goals::{LAST_DAMAGE_TICKS, Mate, Tempter};
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, MonsterSpec, Registered};
use crate::world::{EntityWorld, PlayerHitKind, Voice};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::rng::LegacyRandom;

pub const KIND: &str = "minecraft:panda";
pub const WIDTH: f32 = 1.3;
pub const HEIGHT: f32 = 1.25;
pub const EYE_HEIGHT: f32 = 1.0625;
pub const BABY_WIDTH: f32 = 0.65;
pub const BABY_HEIGHT: f32 = 0.625;
pub const BABY_EYE_HEIGHT: f32 = 0.28125;
pub const MAX_HEALTH: f32 = 20.0;
pub const WEAK_MAX_HEALTH: f32 = 10.0;
pub const SPEED: f64 = 0.15000000596046448;
pub const LAZY_SPEED: f64 = 0.07000000029802322;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const ATTACK_DAMAGE: f32 = 6.0;
pub const AMBIENT_INTERVAL: i32 = 120;
pub const PARENT_AGE: i32 = 6000;
pub const DROP_CHANCE: f32 = 0.085;
pub const GENE_ROLLS: u32 = 16;
pub const MUTATION_CHANCE: u32 = 32;
pub const ALERT_RANGE_Y: f64 = 10.0;
pub const UNHAPPY_LOOK: f32 = 90.0;
pub const CANT_BREED_TICKS: [i32; 2] = [29, 14];
pub const FOOD_TAG: &str = "minecraft:panda_food";
pub const FOOD: [&str; 1] = ["minecraft:bamboo"];
pub const BITE_SOUND: &str = "entity.panda.bite";
pub const CANT_BREED_SOUND: &str = "entity.panda.cant_breed";
pub const STEP_SOUND: &str = "entity.panda.step";
pub const AGGRESSIVE_AMBIENT: &str = "entity.panda.aggressive_ambient";
pub const WORRIED_AMBIENT: &str = "entity.panda.worried_ambient";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Gene {
    #[default]
    Normal,
    Lazy,
    Worried,
    Playful,
    Brown,
    Weak,
    Aggressive,
}

impl Gene {
    pub const ALL: [Self; 7] = [
        Self::Normal,
        Self::Lazy,
        Self::Worried,
        Self::Playful,
        Self::Brown,
        Self::Weak,
        Self::Aggressive,
    ];

    pub fn id(self) -> i32 {
        match self {
            Self::Normal => 0,
            Self::Lazy => 1,
            Self::Worried => 2,
            Self::Playful => 3,
            Self::Brown => 4,
            Self::Weak => 5,
            Self::Aggressive => 6,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Lazy => "lazy",
            Self::Worried => "worried",
            Self::Playful => "playful",
            Self::Brown => "brown",
            Self::Weak => "weak",
            Self::Aggressive => "aggressive",
        }
    }

    pub fn from_name(name: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|gene| gene.name() == name)
            .unwrap_or_default()
    }

    pub fn recessive(self) -> bool {
        matches!(self, Self::Brown | Self::Weak)
    }

    pub fn from_roll(roll: u32) -> Self {
        match roll {
            0 => Self::Lazy,
            1 => Self::Worried,
            2 => Self::Playful,
            4 => Self::Aggressive,
            3 | 5..=8 => Self::Weak,
            9 | 10 => Self::Brown,
            _ => Self::Normal,
        }
    }

    pub fn random(random: &mut LegacyRandom) -> Self {
        Self::from_roll(random.next_int(GENE_ROLLS))
    }

    pub fn variant(main: Self, hidden: Self) -> Self {
        if main.recessive() && main != hidden {
            Self::Normal
        } else {
            main
        }
    }
}

#[derive(Clone)]
pub struct Panda {
    pub main_gene: Gene,
    pub hidden_gene: Gene,
    pub age: Age,
    pub in_love: i32,
    pub last_damage: Option<(&'static str, i32)>,
    pub speed: f64,
    pub held: Option<ItemStack>,
    pub drop_chance: f32,
    pub can_pick_up_loot: bool,
    pub bred: bool,
    pub ai: Box<MonsterAi>,
}

impl Panda {
    pub fn goals(&self) -> Option<&PandaGoals> {
        self.ai.state.extra.get::<PandaGoals>()
    }

    pub fn goals_mut(&mut self) -> &mut PandaGoals {
        let extra = &mut self.ai.state.extra;
        if extra.get::<PandaGoals>().is_none() {
            extra.set(PandaGoals::default());
        }
        extra.get_mut::<PandaGoals>().expect("the panda's goals")
    }

    pub fn variant(&self) -> Gene {
        Gene::variant(self.main_gene, self.hidden_gene)
    }

    pub fn set_genes(&mut self, main: Gene, hidden: Gene) {
        self.main_gene = main;
        self.hidden_gene = hidden;
        let variant = self.variant();
        self.goals_mut().variant = variant;
    }

    pub fn panicking(&self) -> bool {
        self.goals().is_some_and(|goals| goals.animal.panic.running)
    }

    pub fn tempted(&self) -> bool {
        self.goals().is_some_and(|goals| goals.animal.tempted)
    }

    pub fn unhappy(&self) -> i32 {
        self.goals().map_or(0, |goals| goals.unhappy)
    }

    pub fn running_goals(&self) -> Vec<&'static str> {
        self.ai.running_goals()
    }

    pub fn target(&self) -> Option<Target> {
        self.ai.state.target
    }
}

pub fn set_baby(mob: &mut FamilyMob<Panda>, baby: bool) {
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

pub fn set_attributes(mob: &mut FamilyMob<Panda>) {
    match mob.state.variant() {
        Gene::Weak => {
            mob.core.max_health = WEAK_MAX_HEALTH;
            mob.core.health = mob.core.health.min(WEAK_MAX_HEALTH);
        }
        Gene::Lazy => mob.state.speed = LAZY_SPEED,
        _ => {}
    }
}

pub fn is_food(world: &EntityWorld, item: &str) -> bool {
    match &world.recipes {
        Some(recipes) => recipes.item_in_tag(FOOD_TAG, item),
        None => FOOD.contains(&item),
    }
}

pub fn genes_from_parents(
    random: &mut LegacyRandom,
    first: (Gene, Gene),
    first_random: &mut LegacyRandom,
    second: (Gene, Gene),
    second_random: &mut LegacyRandom,
) -> (Gene, Gene) {
    let one_of = |(main, hidden): (Gene, Gene), random: &mut LegacyRandom| {
        if random.next_boolean() { main } else { hidden }
    };
    let (mut main, mut hidden) = if random.next_boolean() {
        let main = one_of(first, first_random);
        (main, one_of(second, second_random))
    } else {
        let main = one_of(second, second_random);
        (main, one_of(first, first_random))
    };
    if random.next_int(MUTATION_CHANCE) == 0 {
        main = Gene::random(random);
    }
    if random.next_int(MUTATION_CHANCE) == 0 {
        hidden = Gene::random(random);
    }
    (main, hidden)
}

pub fn interact(
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
    let mob = world.family_mob_mut::<Panda>(id)?;
    let position = mob.core.body.position;
    let pass = Some((InteractionResult::Pass, Vec::new(), position));
    if !mob.core.alive() {
        return pass;
    }
    let has_target = mob.state.ai.state.target.is_some();
    let goals = mob.state.goals_mut();
    if goals.scared {
        return pass;
    }
    if goals.on_back {
        goals.on_back = false;
        return Some((InteractionResult::SuccessServer, Vec::new(), position));
    }
    let held = inventory.slots.get(hand).and_then(Option::as_ref);
    if held.is_some_and(|stack| food(&stack.id)) && has_target {
        goals.got_bamboo = true;
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
        world.broadcast_entity_event(id, 18);
    }
    Some((result, events, position))
}

fn take_random(world: &mut EntityWorld, id: u64) -> Option<LegacyRandom> {
    let mob = world.family_mob_mut::<Panda>(id)?;
    Some(std::mem::take(&mut mob.core.random))
}

fn put_random(world: &mut EntityWorld, id: u64, random: LegacyRandom) {
    if let Some(mob) = world.family_mob_mut::<Panda>(id) {
        mob.core.random = random;
    }
}

fn breed(world: &mut EntityWorld, id: u64, partner: u64) {
    let genes_of = |mob: &FamilyMob<Panda>| (mob.state.main_gene, mob.state.hidden_gene);
    let pandas = world.family::<Panda>();
    let second = pandas
        .iter()
        .find(|mob| mob.core.id == partner && mob.core.alive())
        .map(genes_of);
    let first = pandas.iter().find(|mob| mob.core.id == id).map(genes_of);
    let (Some(first), Some(second)) = (first, second) else {
        return;
    };
    let seed = world.born_seeds.pop_front().unwrap_or_else(|| {
        world.seed_uniquifier = world
            .seed_uniquifier
            .wrapping_mul(1_181_783_497_276_652_981);
        world.seed_uniquifier
    });
    let mut random = LegacyRandom::new(seed);
    let (Some(mut first_random), Some(mut second_random)) =
        (take_random(world, id), take_random(world, partner))
    else {
        return;
    };
    let (main, hidden) = genes_from_parents(
        &mut random,
        first,
        &mut first_random,
        second,
        &mut second_random,
    );
    put_random(world, id, first_random);
    put_random(world, partner, second_random);
    let mob_drops = world.mob_drops;
    let Some(mob) = world.family_mob_mut::<Panda>(id) else {
        return;
    };
    let position = mob.core.body.position;
    mob.state.age.set(PARENT_AGE);
    mob.state.in_love = 0;
    let orb = mob_drops.then(|| 1 + mob.core.random.next_int(7) as i32);
    if let Some(other) = world.family_mob_mut::<Panda>(partner) {
        other.state.age.set(PARENT_AGE);
        other.state.in_love = 0;
    }
    world.broadcast_entity_event(id, 18);
    if let Some(orb) = orb {
        world.trade_experience.push((position, orb));
    }
    let Some(mut child) = Panda::spawn(KIND, position) else {
        return;
    };
    child.state.age.set(Age::BABY_START);
    set_baby(&mut child, true);
    child.state.set_genes(main, hidden);
    set_attributes(&mut child);
    child.state.bred = true;
    child.core.random = random;
    world.spawn_family(child);
}

fn grow(mob: &mut FamilyMob<Panda>) {
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

fn tail(mob: &mut FamilyMob<Panda>) {
    let FamilyMob { core, state } = mob;
    let ai = &mut *state.ai;
    let target = ai.state.target;
    let unhappy = ai.state.extra.get_mut::<PandaGoals>().map_or(0, |goals| {
        if target.is_none() {
            goals.got_bamboo = false;
            goals.did_bite = false;
        }
        goals.unhappy
    });
    if unhappy <= 0 {
        return;
    }
    if let Some(info) = target.and_then(|target| ai.state.info(target)) {
        ai.state.body.position = core.body.position;
        ai.state.yaw = ai.yaw;
        look_at(&mut ai.state, info, UNHAPPY_LOOK, UNHAPPY_LOOK);
        ai.yaw = ai.state.yaw;
        core.yaw = ai.yaw;
        core.look.pitch = ai.state.look_control.pitch;
    }
    if CANT_BREED_TICKS.contains(&unhappy) {
        let voice = Voice::Event(CANT_BREED_SOUND, 1.0, 1.0);
        core.voices.push((voice, core.body.position));
    }
    if let Some(goals) = ai.state.extra.get_mut::<PandaGoals>() {
        goals.unhappy -= 1;
    }
}

fn bite(world: &mut EntityWorld, id: u64, target: Target) {
    let Some(mob) = world.family_mob_mut::<Panda>(id) else {
        return;
    };
    let attacker = mob.core.body.position;
    if mob.state.variant() != Gene::Aggressive {
        mob.state.goals_mut().did_bite = true;
    }
    let kind = PlayerHitKind::Melee {
        attacker,
        hunger_ticks: 0,
        lift: 0.0,
        throw: DVec3::ZERO,
        disable_shield_seconds: 0.0,
    };
    let hit = world.family_strike(id, target, ATTACK_DAMAGE, kind);
    if hit.is_some_and(|hit| !hit.applied) {
        return;
    }
    if let Some(mob) = world.family_mob_mut::<Panda>(id) {
        let voice = Voice::Event(BITE_SOUND, 1.0, 1.0);
        mob.core.voices.push((voice, mob.core.body.position));
    }
}

fn alert_aggressive(world: &mut EntityWorld, id: u64, attacker: Target) {
    let store = Panda::store_mut(world);
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
        if other.core.id == id || other.state.variant() != Gene::Aggressive {
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
        if touches && state.target.is_none() {
            state.set_target(Some(attacker));
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
        .family::<Panda>()
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

fn kin(world: &EntityWorld, id: u64) -> Vec<CowCandidate> {
    world
        .family::<Panda>()
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

fn ambient_event(variant: Gene) -> Option<&'static str> {
    match variant {
        Gene::Aggressive => Some(AGGRESSIVE_AMBIENT),
        Gene::Worried => Some(WORRIED_AMBIENT),
        _ => None,
    }
}

impl Family for Panda {
    const KEY: u8 = 52;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let mut core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        core.ambient_interval = AMBIENT_INTERVAL;
        let mut ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        ai.state.extra.set(PandaGoals::default());
        Some(FamilyMob {
            core,
            state: Panda {
                main_gene: Gene::Normal,
                hidden_gene: Gene::Normal,
                age: Age::default(),
                in_love: 0,
                last_damage: None,
                speed: SPEED,
                held: None,
                drop_chance: DROP_CHANCE,
                can_pick_up_loot: true,
                bred: false,
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
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            if let Some(event) = ambient_event(mob.state.variant()) {
                for (voice, _) in &mut mob.core.voices {
                    if let Voice::Ambient(pitch) = *voice {
                        *voice = Voice::Event(event, 1.0, pitch);
                    }
                }
            }
        }
        if base.alive && !base.no_ai {
            let tempters = tempters(world, players);
            let mates = mates(world, id);
            let kin = kin(world, id);
            let mut speed = SPEED;
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                let tick_count = mob.core.tick_count;
                let last_damage = mob
                    .state
                    .last_damage
                    .filter(|&(_, at)| tick_count - at <= LAST_DAMAGE_TICKS)
                    .map(|(damage_type, _)| damage_type);
                speed = mob.state.speed;
                let (variant, baby) = (mob.state.variant(), mob.core.baby);
                let (age, in_love) = (mob.state.age.ticks, mob.state.in_love > 0);
                let goals = mob.state.goals_mut();
                goals.variant = variant;
                goals.baby = baby;
                goals.animal.id = id;
                goals.animal.age = age;
                goals.animal.last_damage = last_damage;
                goals.animal.tempters = tempters;
                goals.animal.mates = mates;
                goals.animal.in_love = in_love;
                goals.animal.kin = kin;
            }
            let strike = world
                .family_goal_step::<Self, W>(id, level, players, speed, |panda| &mut *panda.ai);
            let (alert, partner) = world
                .family_mob_mut::<Self>(id)
                .map_or((None, None), |mob| {
                    let alert = mob.state.ai.state.alert.take();
                    (alert, mob.state.goals_mut().animal.breed_with.take())
                });
            if let Some(target) = strike {
                bite(world, id, target);
            }
            if let Some(attacker) = alert {
                alert_aggressive(world, id, attacker);
            }
            if let Some(partner) = partner {
                breed(world, id, partner);
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            grow(mob);
            tail(mob);
        }
    }

    fn category(_kind: &str) -> &'static str {
        "creature"
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        mob.state.goals_mut().sitting = false;
        let result = mob.core.hurt_plain(hurt.amount);
        if result.applied {
            mob.state.in_love = 0;
            mob.state.last_damage = Some((hurt.damage_type, mob.core.tick_count));
        }
        result
    }

    fn panicking(mob: &FamilyMob<Self>) -> bool {
        mob.state.panicking()
    }

    fn step_sound(_kind: &str) -> Option<&'static str> {
        Some(STEP_SOUND)
    }

    fn monster(kind: &str) -> Option<&'static MonsterSpec> {
        (kind == KIND).then_some(&SPEC)
    }
}

#[cfg(test)]
mod tests;
