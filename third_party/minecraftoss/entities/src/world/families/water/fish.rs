mod ai;

pub use self::ai::{FishAi, FishSwimmer, spec_of};
use crate::health::DamageResult;
use crate::monster_ai::{MonsterAi, MonsterKind};
use crate::navigation::WaterNav;
use crate::tempt::PlayerCandidate;
use crate::world::families::water::abstract_fish::{
    self, AMBIENT_INTERVAL, CATEGORY, MAX_HEALTH, SPEED, SPLASH, SWIM_SOUND, dying_swim,
    last_damage,
};
use crate::world::families::water::swim::flop;
use crate::world::families::water::swimming::swim_ai_step;
use crate::world::families::{Breath, Despawn, Family, FamilyMob, Hurt, MobCore, MonsterSpec};
use crate::world::{EntityWorld, Voice};
use glam::DVec3;
use minecraftoss_player::World;

pub const COD: &str = "minecraft:cod";
pub const SALMON: &str = "minecraft:salmon";
pub const TROPICAL_FISH: &str = "minecraft:tropical_fish";
pub const PUFFERFISH: &str = "minecraft:pufferfish";
pub const MAX_PUFF: i32 = 2;
pub const VOLUME: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SalmonSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl SalmonSize {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];

    pub fn id(self) -> i32 {
        match self {
            Self::Small => 0,
            Self::Medium => 1,
            Self::Large => 2,
        }
    }

    pub fn by_id(id: i32) -> Self {
        Self::ALL[id.clamp(0, 2) as usize]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|size| size.name() == name)
    }

    pub fn scale(self) -> f32 {
        match self {
            Self::Small => 0.5,
            Self::Medium => 1.0,
            Self::Large => 1.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FishVariant {
    Cod,
    Salmon(SalmonSize),
    Tropical(i32),
    Pufferfish,
}

impl FishVariant {
    pub fn of_kind(kind: &str) -> Option<Self> {
        match kind {
            COD => Some(Self::Cod),
            SALMON => Some(Self::Salmon(SalmonSize::default())),
            TROPICAL_FISH => Some(Self::Tropical(0)),
            PUFFERFISH => Some(Self::Pufferfish),
            _ => None,
        }
    }

    pub fn kind(self) -> &'static str {
        match self {
            Self::Cod => COD,
            Self::Salmon(_) => SALMON,
            Self::Tropical(_) => TROPICAL_FISH,
            Self::Pufferfish => PUFFERFISH,
        }
    }

    pub fn dimensions(self) -> (f32, f32, f32) {
        match self {
            Self::Cod => (0.5, 0.3, 0.195),
            Self::Salmon(_) => (0.7, 0.4, 0.26),
            Self::Tropical(_) => (0.5, 0.4, 0.26),
            Self::Pufferfish => (0.7, 0.7, 0.455),
        }
    }

    pub fn sound_family(self) -> &'static str {
        match self {
            Self::Cod => "cod",
            Self::Salmon(_) => "salmon",
            Self::Tropical(_) => "tropical_fish",
            Self::Pufferfish => "puffer_fish",
        }
    }

    pub fn flop_sound(self) -> &'static str {
        match self {
            Self::Cod => "entity.cod.flop",
            Self::Salmon(_) => "entity.salmon.flop",
            Self::Tropical(_) => "entity.tropical_fish.flop",
            Self::Pufferfish => "entity.puffer_fish.flop",
        }
    }

    pub fn has_ambient_sound(self) -> bool {
        self != Self::Pufferfish
    }
}

pub fn puff_scale(puff: i32) -> f32 {
    match puff {
        0 => 0.5,
        1 => 0.7,
        _ => 1.0,
    }
}

#[derive(Clone)]
pub struct Fish {
    pub variant: FishVariant,
    pub from_bucket: bool,
    pub puff: i32,
    pub last_damage: Option<(&'static str, i32)>,
    pub ai: Box<MonsterAi>,
}

impl Fish {
    pub fn scale(&self) -> f32 {
        match self.variant {
            FishVariant::Salmon(size) => size.scale(),
            FishVariant::Pufferfish => puff_scale(self.puff),
            FishVariant::Cod | FishVariant::Tropical(_) => 1.0,
        }
    }

    pub fn goals(&self) -> Option<&FishAi> {
        self.ai.state.extra.get::<FishAi>()
    }

    pub fn goals_mut(&mut self) -> &mut FishAi {
        let extra = &mut self.ai.state.extra;
        if extra.get::<FishAi>().is_none() {
            extra.set(FishAi::default());
        }
        extra.get_mut::<FishAi>().expect("the fish's goals")
    }

    pub fn running_goals(&self) -> Vec<&'static str> {
        self.ai.running_goals()
    }
}

pub fn refresh_size(mob: &mut FamilyMob<Fish>) {
    let (width, height, eye) = mob.state.variant.dimensions();
    let scale = mob.state.scale();
    let (width, height, eye) = (width * scale, height * scale, eye * scale);
    let core = &mut mob.core;
    core.body.width = width;
    core.body.height = height;
    core.eye_height = eye;
    let ai = &mut mob.state.ai;
    ai.state.walk.width = width;
    ai.state.walk.height = height;
    ai.state.eye_height = eye;
}

pub fn set_salmon_size(mob: &mut FamilyMob<Fish>, size: SalmonSize) {
    if let FishVariant::Salmon(_) = mob.state.variant {
        mob.state.variant = FishVariant::Salmon(size);
        refresh_size(mob);
    }
}

pub fn set_puff(mob: &mut FamilyMob<Fish>, puff: i32) {
    mob.state.puff = puff.min(MAX_PUFF);
    refresh_size(mob);
}

fn thinking(fish: &mut Fish) -> &mut MonsterAi {
    &mut fish.ai
}

impl Family for Fish {
    const KEY: u8 = 65;
    const KINDS: &'static [&'static str] = &[COD, SALMON, TROPICAL_FISH, PUFFERFISH];
    const OWN_DYING_TRAVEL: bool = true;

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let variant = FishVariant::of_kind(kind)?;
        let (width, height, eye) = variant.dimensions();
        let mut core = MobCore::new(kind, position, width, height, eye, MAX_HEALTH);
        core.ambient_interval = AMBIENT_INTERVAL;
        let mut ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        ai.state.navigation.water = WaterNav::Bound { breaching: false };
        ai.state.extra.set(FishAi::default());
        let state = Fish {
            variant,
            from_bucket: false,
            puff: 0,
            last_damage: None,
            ai: Box::new(ai),
        };
        let mut mob = FamilyMob { core, state };
        refresh_size(&mut mob);
        Some(mob)
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
        let thinks = base.alive && !base.no_ai;
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            let variant = mob.state.variant;
            if !variant.has_ambient_sound() {
                let voices = &mut mob.core.voices;
                voices.retain(|(voice, _)| !matches!(voice, Voice::Ambient(_)));
            }
            flop(&mut mob.core, variant.flop_sound());
            if !base.alive && !base.no_ai {
                dying_swim::<Self, W>(&mut mob.core, &*level);
            }
            if thinks {
                let damage = last_damage(mob.state.last_damage, mob.core.tick_count);
                mob.state.goals_mut().fish.last_damage = damage;
            }
        }
        if thinks {
            let run = swim_ai_step::<FishSwimmer, W>;
            let _ =
                world.family_goal_step_with::<Self, W>(id, level, players, SPEED, thinking, run);
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn category(_kind: &str) -> &'static str {
        CATEGORY
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        let result = mob.core.hurt_plain(hurt.amount);
        if result.applied {
            mob.state.last_damage = Some((hurt.damage_type, mob.core.tick_count));
        }
        result
    }

    fn experience(mob: &mut FamilyMob<Self>) -> i32 {
        1 + mob.core.random.next_int(3) as i32
    }

    fn despawn(mob: &FamilyMob<Self>) -> Despawn {
        abstract_fish::despawn(mob.state.from_bucket)
    }

    fn breathes_water(_mob: &FamilyMob<Self>) -> bool {
        true
    }

    fn breath(_mob: &FamilyMob<Self>) -> Breath {
        Breath::Gills
    }

    fn voice(mob: &FamilyMob<Self>) -> (&'static str, f32, &'static str) {
        (mob.state.variant.sound_family(), VOLUME, "neutral_volume")
    }

    fn monster(kind: &str) -> Option<&'static MonsterSpec> {
        spec_of(kind)
    }

    fn swim_sounds(_kind: &str) -> Option<(&'static str, &'static str)> {
        Some((SWIM_SOUND, SPLASH))
    }

    fn quiet_steps(_kind: &str) -> bool {
        true
    }
}

#[cfg(test)]
mod tests;
