use super::hazards::{self, Exposed};
use super::*;
use crate::effects::MobEffects;
use crate::inside_blocks::Hazard;
use crate::monster_ai::{MonsterGoalContext, Target};
use std::any::Any;
use std::ops::RangeInclusive;

macro_rules! family_group {
    ($group:ident $low:literal ..= $high:literal { $($(#[$attr:meta])* $field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Default)]
        pub struct Group {
            $($(#[$attr])* pub $field: $crate::world::families::FamilyStore<$ty>,)*
        }

        $($(#[$attr])*
        impl $crate::world::families::Registered for $ty {
            fn store(world: &$crate::world::EntityWorld) -> &$crate::world::families::FamilyStore<Self> {
                &world.families.$group.$field
            }
            fn store_mut(world: &mut $crate::world::EntityWorld) -> &mut $crate::world::families::FamilyStore<Self> {
                &mut world.families.$group.$field
            }
        })*

        #[allow(unused_variables, unused_mut)]
        impl Group {
            pub const NAME: &'static str = stringify!($group);
            pub const RANGE: std::ops::RangeInclusive<u8> = $low..=$high;

            pub fn keys() -> Vec<u8> {
                let mut out = Vec::new();
                $($(#[$attr])* out.push(<$ty as $crate::world::families::Family>::KEY);)*
                out
            }

            pub fn has_key(key: u8) -> bool {
                Self::keys().contains(&key)
            }

            pub fn kinds() -> Vec<&'static str> {
                let mut out = Vec::new();
                $($(#[$attr])* out.extend_from_slice(<$ty as $crate::world::families::Family>::KINDS);)*
                out
            }

            pub fn key_of_kind(kind: &str) -> Option<u8> {
                $($(#[$attr])* if <$ty as $crate::world::families::Family>::KINDS.contains(&kind) {
                    return Some(<$ty as $crate::world::families::Family>::KEY);
                })*
                None
            }

            pub fn category(key: u8, kind: &str) -> Option<&'static str> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return Some(<$ty as $crate::world::families::Family>::category(kind));
                })*
                None
            }

            pub fn enemy(kind: &str) -> bool {
                $($(#[$attr])* if <$ty as $crate::world::families::Family>::KINDS.contains(&kind) {
                    return <$ty as $crate::world::families::Family>::enemy(kind);
                })*
                false
            }

            pub fn untargetable(kind: &str) -> bool {
                $($(#[$attr])* if <$ty as $crate::world::families::Family>::KINDS.contains(&kind) {
                    return !<$ty as $crate::world::families::Family>::targetable(kind);
                })*
                false
            }

            pub fn display_name(kind: &str) -> Option<&'static str> {
                $($(#[$attr])* if <$ty as $crate::world::families::Family>::KINDS.contains(&kind) {
                    return <$ty as $crate::world::families::Family>::display_name(kind);
                })*
                None
            }

            pub fn is_vehicle(kind: &str) -> bool {
                $($(#[$attr])* if <$ty as $crate::world::families::Family>::KINDS.contains(&kind) {
                    return <$ty as $crate::world::families::Family>::MOUNT;
                })*
                false
            }

            pub fn monster_spec(kind: &str) -> Option<&'static $crate::world::families::MonsterSpec> {
                $($(#[$attr])* if <$ty as $crate::world::families::Family>::KINDS.contains(&kind) {
                    return <$ty as $crate::world::families::Family>::monster(kind);
                })*
                None
            }

            pub fn spawn(world: &mut $crate::world::EntityWorld, kind: &str, position: glam::DVec3) -> Option<u64> {
                $($(#[$attr])* if let Some(&kind) = <$ty as $crate::world::families::Family>::KINDS.iter().find(|k| **k == kind) {
                    let mob = <$ty as $crate::world::families::Family>::spawn(kind, position)?;
                    return Some(world.spawn_family::<$ty>(mob));
                })*
                None
            }

            pub fn tick<W: minecraftoss_player::World>(
                world: &mut $crate::world::EntityWorld,
                key: u8,
                id: u64,
                level: &mut W,
                players: &[$crate::tempt::PlayerCandidate],
                ticks: &dyn Fn(glam::DVec3) -> bool,
            ) -> bool {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    <$ty as $crate::world::families::Family>::tick(world, id, level, players, ticks);
                    return true;
                })*
                false
            }

            pub fn key_of(&self, id: u64) -> Option<u8> {
                $($(#[$attr])* if self.$field.get(id).is_some() {
                    return Some(<$ty as $crate::world::families::Family>::KEY);
                })*
                None
            }

            pub fn core_in(&self, key: u8, id: u64) -> Option<&$crate::world::families::MobCore> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.get(id).map(|mob| &mob.core);
                })*
                None
            }

            pub fn core_in_mut(&mut self, key: u8, id: u64) -> Option<&mut $crate::world::families::MobCore> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.get_mut(id).map(|mob| &mut mob.core);
                })*
                None
            }

            pub fn hurt(&mut self, key: u8, id: u64, hurt: &$crate::world::families::Hurt) -> Option<$crate::health::DamageResult> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.hurt(id, hurt);
                })*
                None
            }

            pub fn traits(&self, key: u8, id: u64) -> Option<$crate::world::families::MobTraits> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.traits(id);
                })*
                None
            }

            pub fn effects_mut(&mut self, key: u8, id: u64) -> Option<(&mut $crate::effects::MobEffects, bool, bool)> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.effects_mut(id);
                })*
                None
            }

            pub(crate) fn exposed_mut(&mut self, key: u8, id: u64) -> Option<&mut dyn $crate::world::hazards::Exposed> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.get_mut(id).map(|mob| mob as &mut dyn $crate::world::hazards::Exposed);
                })*
                None
            }

            pub fn push_body(&mut self, key: u8, id: u64) -> Option<(&mut $crate::movement::Body, bool)> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.push_body(id);
                })*
                None
            }

            pub fn push_body_ref(&self, key: u8, id: u64) -> Option<&$crate::movement::Body> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.push_body_ref(id);
                })*
                None
            }

            pub fn passenger_point(&self, key: u8, id: u64, seat: usize) -> Option<glam::DVec3> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.passenger_point(id, seat);
                })*
                None
            }

            pub fn dismount(&self, key: u8, id: u64, level: &dyn minecraftoss_player::World, width: f32, heights: &[f32]) -> Option<glam::DVec3> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.dismount(id, level, width, heights);
                })*
                None
            }

            pub fn controller(&self, key: u8, id: u64, passengers: &[$crate::world::Rider]) -> Option<$crate::world::Rider> {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.controller(id, passengers);
                })*
                None
            }

            pub fn rider_jump(&mut self, key: u8, id: u64, power: i32) -> bool {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.rider_jump(id, power);
                })*
                false
            }

            pub fn rider_order(&mut self, key: u8, id: u64, target: glam::DVec3, speed: f64) -> bool {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.rider_order(id, target, speed);
                })*
                false
            }

            pub fn leash_walk<W: minecraftoss_player::World>(&mut self, key: u8, id: u64, level: &W, target: glam::DVec3, speed: f64) -> bool {
                $($(#[$attr])* if key == <$ty as $crate::world::families::Family>::KEY {
                    return self.$field.leash_walk(id, level, target, speed);
                })*
                false
            }

            pub fn for_each(&self, f: &mut dyn FnMut(u8, &$crate::world::families::MobCore)) {
                $($(#[$attr])* self.$field.for_each(f);)*
            }

            pub fn for_each_mut(&mut self, f: &mut dyn FnMut(u8, &mut $crate::world::families::MobCore)) {
                $($(#[$attr])* self.$field.for_each_mut(f);)*
            }

            pub fn census(&self, f: &mut dyn FnMut(&$crate::world::families::MobCore, &$crate::world::families::Despawn)) {
                $($(#[$attr])* self.$field.census(f);)*
            }

            pub fn retain_mut(&mut self, f: &mut dyn FnMut(u8, &mut $crate::world::families::MobCore, &$crate::world::families::Despawn) -> bool) {
                $($(#[$attr])* self.$field.retain_mut(f);)*
            }

            pub fn clone_where(&self, keep: &dyn Fn(glam::DVec3) -> bool) -> Self {
                Self {
                    $($(#[$attr])* $field: self.$field.clone_where(keep),)*
                }
            }

            pub fn take_deaths(&mut self, out: &mut Vec<(u64, $crate::world::MobDeath)>, mob_drops: bool) {
                $($(#[$attr])* self.$field.take_deaths(out, mob_drops);)*
            }

            pub fn take_sounds(&mut self, out: &mut Vec<$crate::world::MobSound>) {
                $($(#[$attr])* self.$field.take_sounds(out);)*
            }

            pub fn bodies_mut<'a>(&'a mut self, out: &mut Vec<&'a mut $crate::movement::Body>) {
                $($(#[$attr])* self.$field.bodies_mut(out);)*
            }

            pub fn positions(&self, out: &mut Vec<(u64, glam::DVec3)>) {
                $($(#[$attr])* self.$field.positions(out);)*
            }

            pub fn cores<'a>(&'a self, out: &mut Vec<(u8, &'a $crate::world::families::MobCore)>) {
                $($(#[$attr])* self.$field.cores(out);)*
            }
        }
    };
}

pub mod bosses;
pub mod deep_dark;
pub mod goals;
pub mod land;
pub mod night;
pub mod pests;
pub mod trial;
pub mod water;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct MobCore {
    pub id: u64,
    pub kind: &'static str,
    pub body: Body,
    pub eye_height: f32,
    pub health: f32,
    pub max_health: f32,
    pub damage: DamageState,
    pub effects: MobEffects,
    pub yaw: f32,
    pub look: LookControl,
    pub baby: bool,
    pub persistent: bool,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub ambient_interval: i32,
    pub no_action_time: i32,
    pub hurt_by: Option<(Target, i32)>,
    pub swings: u32,
    pub voices: Vec<(Voice, DVec3)>,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub removed: bool,
}

impl MobCore {
    pub fn new(
        kind: &'static str,
        position: DVec3,
        width: f32,
        height: f32,
        eye_height: f32,
        max_health: f32,
    ) -> Self {
        Self {
            id: 0,
            kind,
            body: Body::new(position, width, height),
            eye_height,
            health: max_health,
            max_health,
            damage: DamageState::default(),
            effects: MobEffects::default(),
            yaw: 0.0,
            look: LookControl::new(0.0),
            baby: false,
            persistent: false,
            no_ai: false,
            tick_count: 0,
            ambient_sound_time: 0,
            ambient_interval: 80,
            no_action_time: 0,
            hurt_by: None,
            swings: 0,
            voices: Vec::new(),
            random: LegacyRandom::new(0),
            previous_position: position,
            removed: false,
        }
    }

    pub fn position(&self) -> DVec3 {
        self.body.position
    }

    pub fn alive(&self) -> bool {
        self.health > 0.0
    }

    pub fn hurt_plain(&mut self, amount: f32) -> DamageResult {
        if self.health > 0.0 && !self.damage.dead {
            self.no_action_time = 0;
        }
        let result = self
            .damage
            .hurt_generic(&mut self.health, self.max_health, amount);
        let position = self.body.position;
        self.damage
            .place_death(result, position, self.body.fire_ticks > 0);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -self.ambient_interval;
            }
            let voice = hurt_voice(&mut self.random, result.died, self.baby);
            self.voices.push((voice, position));
        }
        result
    }
}

#[derive(Clone)]
pub struct FamilyMob<S> {
    pub core: MobCore,
    pub state: S,
}

#[derive(Clone)]
pub struct FamilyStore<S> {
    pub mobs: Vec<FamilyMob<S>>,
}

impl<S> Default for FamilyStore<S> {
    fn default() -> Self {
        Self { mobs: Vec::new() }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Hurt {
    pub amount: f32,
    pub damage_type: &'static str,
    pub source: DamageSourceKind,
    pub attacker: Option<Target>,
    pub attacker_kind: Option<&'static str>,
    pub bypasses_armor: bool,
}

impl Hurt {
    pub fn new(amount: f32, damage_type: &'static str, source: DamageSourceKind) -> Self {
        Self {
            amount,
            damage_type,
            source,
            attacker: None,
            attacker_kind: None,
            bypasses_armor: false,
        }
    }

    pub fn by(self, attacker: Option<Target>, attacker_kind: Option<&'static str>) -> Self {
        Self {
            attacker,
            attacker_kind,
            ..self
        }
    }

    pub fn bypassing_armor(self) -> Self {
        Self {
            bypasses_armor: true,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Despawn {
    pub distance: f64,
    pub removable: bool,
    pub peaceful_ok: bool,
    pub kept: bool,
}

impl Despawn {
    pub fn of_category(category: &str) -> Self {
        let (distance, removable, peaceful_ok) = match category {
            "monster" => (128.0, true, false),
            "creature" | "misc" => (128.0, false, true),
            "water_ambient" => (64.0, true, true),
            _ => (128.0, true, true),
        };
        Self {
            distance,
            removable,
            peaceful_ok,
            kept: false,
        }
    }
}

pub struct MonsterSpec {
    pub goal_names: &'static [&'static str],
    pub target_names: &'static [&'static str],
    pub goals: fn() -> GoalSelector<MonsterGoalContext>,
    pub targets: fn() -> GoalSelector<MonsterGoalContext>,
    pub eye_height: f32,
    pub follow_range: f64,
    pub walk: fn(&mut WalkProfile),
}

pub trait ExtraState: Any + Send {
    fn clone_box(&self) -> Box<dyn ExtraState>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Any + Clone + Send> ExtraState for T {
    fn clone_box(&self) -> Box<dyn ExtraState> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
pub struct GoalExtra(Option<Box<dyn ExtraState>>);

impl Clone for GoalExtra {
    fn clone(&self) -> Self {
        Self(self.0.as_ref().map(|state| state.clone_box()))
    }
}

impl GoalExtra {
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.0.as_deref()?.as_any().downcast_ref()
    }

    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.0.as_deref_mut()?.as_any_mut().downcast_mut()
    }

    pub fn set<T: Any + Clone + Send>(&mut self, state: T) {
        self.0 = Some(Box::new(state));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MobTraits {
    pub undead: bool,
    pub immune_to_poison: bool,
    pub fire_immune: bool,
    pub breathes_water: bool,
    pub enemy: bool,
    pub leashable: bool,
    pub pushable: bool,
    pub panicking: bool,
    pub knockback_resistance: f64,
    pub category: &'static str,
    pub voice: (&'static str, f32, &'static str),
    pub seats: usize,
    pub rider_jumps: bool,
    pub vehicle_attachment: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BaseTick {
    pub alive: bool,
    pub no_ai: bool,
    pub game_time: i64,
}

pub trait Family: Clone + Send + Sized + 'static {
    const KEY: u8;
    const KINDS: &'static [&'static str];
    const MOUNT: bool = false;

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>>;

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    );

    fn category(_kind: &str) -> &'static str {
        "monster"
    }

    fn armor(_mob: &FamilyMob<Self>) -> f32 {
        0.0
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        let armor = Self::armor(mob);
        let amount = if hurt.bypasses_armor || armor <= 0.0 {
            hurt.amount
        } else {
            damage_after_armor(hurt.amount, armor, 0.0)
        };
        mob.core.hurt_plain(amount)
    }

    fn experience(mob: &mut FamilyMob<Self>) -> i32 {
        match Self::category(mob.core.kind) {
            "monster" => 5,
            "creature" if !mob.core.baby => 1 + mob.core.random.next_int(3) as i32,
            _ => 0,
        }
    }

    fn death_drops(_mob: &mut FamilyMob<Self>) -> Vec<ItemStack> {
        Vec::new()
    }

    fn despawn(mob: &FamilyMob<Self>) -> Despawn {
        Despawn::of_category(Self::category(mob.core.kind))
    }

    fn undead(mob: &FamilyMob<Self>) -> bool {
        crate::wither::is_undead(mob.core.kind)
    }

    fn immune_to_poison(_mob: &FamilyMob<Self>) -> bool {
        false
    }

    fn fire_immune(mob: &FamilyMob<Self>) -> bool {
        hazards::fire_immune(mob.core.kind)
    }

    fn breathes_water(mob: &FamilyMob<Self>) -> bool {
        crate::wither::is_undead(mob.core.kind)
    }

    fn enemy(_kind: &str) -> bool {
        false
    }

    fn targetable(_kind: &str) -> bool {
        true
    }

    fn display_name(_kind: &str) -> Option<&'static str> {
        None
    }

    fn leashable(_mob: &FamilyMob<Self>) -> bool {
        false
    }

    fn pushable(_mob: &FamilyMob<Self>) -> bool {
        true
    }

    fn knockback_resistance(_mob: &FamilyMob<Self>) -> f64 {
        0.0
    }

    fn voice(mob: &FamilyMob<Self>) -> (&'static str, f32, &'static str) {
        let kind = mob.core.kind;
        let family = kind.strip_prefix("minecraft:").unwrap_or(kind);
        let category = if Self::category(kind) == "monster" {
            "hostile_volume"
        } else {
            "neutral_volume"
        };
        (family, 1.0, category)
    }

    fn seats(_mob: &FamilyMob<Self>) -> usize {
        0
    }

    fn passenger_point(mob: &FamilyMob<Self>, _seat: usize) -> DVec3 {
        DVec3::new(0.0, f64::from(mob.core.body.height), 0.0)
    }

    fn dismount(mob: &FamilyMob<Self>, _level: &dyn World, _width: f32, _heights: &[f32]) -> DVec3 {
        let body = &mob.core.body;
        body.position + DVec3::new(0.0, f64::from(body.height), 0.0)
    }

    fn monster(_kind: &str) -> Option<&'static MonsterSpec> {
        None
    }

    fn step_sound(_kind: &str) -> Option<&'static str> {
        None
    }

    fn emits_sounds(_kind: &str) -> bool {
        true
    }

    fn loot(_mob: &FamilyMob<Self>, _context: &mut EntityLootContext) {}

    fn panicking(_mob: &FamilyMob<Self>) -> bool {
        false
    }

    fn leash_walk<W: World>(_mob: &mut FamilyMob<Self>, _level: &W, _target: DVec3, _speed: f64) {}

    fn controller(_mob: &FamilyMob<Self>, _passengers: &[Rider]) -> Option<Rider> {
        None
    }

    fn rider_jumps(_mob: &FamilyMob<Self>) -> bool {
        false
    }

    fn rider_jump(_mob: &mut FamilyMob<Self>, _power: i32) {}

    fn rider_order(_mob: &mut FamilyMob<Self>, _target: DVec3, _speed: f64) {}

    fn vehicle_attachment(_mob: &FamilyMob<Self>) -> f64 {
        0.0
    }
}

pub trait Registered: Family {
    fn store(world: &EntityWorld) -> &FamilyStore<Self>;
    fn store_mut(world: &mut EntityWorld) -> &mut FamilyStore<Self>;
}

fn traits_of<S: Family>(mob: &FamilyMob<S>) -> MobTraits {
    let kind = mob.core.kind;
    MobTraits {
        undead: S::undead(mob),
        immune_to_poison: S::immune_to_poison(mob),
        fire_immune: S::fire_immune(mob),
        breathes_water: S::breathes_water(mob),
        enemy: S::enemy(kind),
        leashable: S::leashable(mob),
        pushable: S::pushable(mob),
        panicking: S::panicking(mob),
        knockback_resistance: S::knockback_resistance(mob),
        category: S::category(kind),
        voice: S::voice(mob),
        seats: S::seats(mob),
        rider_jumps: S::rider_jumps(mob),
        vehicle_attachment: S::vehicle_attachment(mob),
    }
}

fn movement_sounds<S: Family>(kind: &str) -> MovementSounds {
    let step = S::step_sound(kind);
    let sounds = if S::category(kind) == "monster" {
        MovementSounds::monster(step)
    } else {
        MovementSounds::creature(step)
    };
    if S::emits_sounds(kind) {
        sounds
    } else {
        sounds.without_sounds()
    }
}

impl<S: Family> FamilyStore<S> {
    pub fn get(&self, id: u64) -> Option<&FamilyMob<S>> {
        self.mobs.iter().find(|mob| mob.core.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut FamilyMob<S>> {
        self.mobs.iter_mut().find(|mob| mob.core.id == id)
    }

    pub fn hurt(&mut self, id: u64, hurt: &Hurt) -> Option<DamageResult> {
        let mob = self.get_mut(id)?;
        Some(S::hurt(mob, hurt))
    }

    pub fn traits(&self, id: u64) -> Option<MobTraits> {
        self.get(id).map(traits_of)
    }

    pub fn effects_mut(&mut self, id: u64) -> Option<(&mut MobEffects, bool, bool)> {
        let mob = self.get_mut(id)?;
        let (undead, immune) = (S::undead(mob), S::immune_to_poison(mob));
        Some((&mut mob.core.effects, undead, immune))
    }

    pub fn push_body(&mut self, id: u64) -> Option<(&mut Body, bool)> {
        let mob = self.get_mut(id)?;
        if !S::pushable(mob) {
            return None;
        }
        let alive = mob.core.health > 0.0;
        Some((&mut mob.core.body, alive))
    }

    pub fn push_body_ref(&self, id: u64) -> Option<&Body> {
        self.get(id)
            .filter(|mob| S::pushable(mob))
            .map(|mob| &mob.core.body)
    }

    pub fn passenger_point(&self, id: u64, seat: usize) -> Option<DVec3> {
        self.get(id).map(|mob| S::passenger_point(mob, seat))
    }

    pub fn dismount(
        &self,
        id: u64,
        level: &dyn World,
        width: f32,
        heights: &[f32],
    ) -> Option<DVec3> {
        self.get(id)
            .map(|mob| S::dismount(mob, level, width, heights))
    }

    pub fn controller(&self, id: u64, passengers: &[Rider]) -> Option<Rider> {
        self.get(id).and_then(|mob| S::controller(mob, passengers))
    }

    pub fn rider_jump(&mut self, id: u64, power: i32) -> bool {
        let Some(mob) = self.get_mut(id) else {
            return false;
        };
        S::rider_jump(mob, power);
        true
    }

    pub fn rider_order(&mut self, id: u64, target: DVec3, speed: f64) -> bool {
        let Some(mob) = self.get_mut(id) else {
            return false;
        };
        S::rider_order(mob, target, speed);
        true
    }

    pub fn leash_walk<W: World>(&mut self, id: u64, level: &W, target: DVec3, speed: f64) -> bool {
        let Some(mob) = self.get_mut(id) else {
            return false;
        };
        S::leash_walk(mob, level, target, speed);
        true
    }

    pub fn for_each(&self, f: &mut dyn FnMut(u8, &MobCore)) {
        for mob in &self.mobs {
            f(S::KEY, &mob.core);
        }
    }

    pub fn for_each_mut(&mut self, f: &mut dyn FnMut(u8, &mut MobCore)) {
        for mob in &mut self.mobs {
            f(S::KEY, &mut mob.core);
        }
    }

    pub fn census(&self, f: &mut dyn FnMut(&MobCore, &Despawn)) {
        for mob in &self.mobs {
            f(&mob.core, &S::despawn(mob));
        }
    }

    pub fn retain_mut(&mut self, f: &mut dyn FnMut(u8, &mut MobCore, &Despawn) -> bool) {
        self.mobs.retain_mut(|mob| {
            let rule = S::despawn(mob);
            f(S::KEY, &mut mob.core, &rule)
        });
    }

    pub fn clone_where(&self, keep: &dyn Fn(DVec3) -> bool) -> Self {
        Self {
            mobs: self
                .mobs
                .iter()
                .filter(|mob| keep(mob.core.body.position))
                .cloned()
                .collect(),
        }
    }

    pub fn take_deaths(&mut self, out: &mut Vec<(u64, MobDeath)>, mob_drops: bool) {
        for mob in &mut self.mobs {
            if mob.core.removed || mob.core.damage.death.is_none() {
                continue;
            }
            let kind = mob.core.kind;
            let monster = S::category(kind) == "monster";
            let table = (monster || !mob.core.baby).then_some(kind);
            let mut context = EntityLootContext {
                baby: mob.core.baby,
                ..EntityLootContext::default()
            };
            S::loot(mob, &mut context);
            let Some((order, mut death)) =
                death_of(mob.core.id, &mut mob.core.damage, kind, table, context)
            else {
                continue;
            };
            if mob_drops && death.context.killed_by_player && (monster || !death.context.baby) {
                death.experience = S::experience(mob);
            }
            death.equipment = S::death_drops(mob);
            out.push((order, death));
        }
    }

    pub fn take_sounds(&mut self, out: &mut Vec<MobSound>) {
        for mob in &mut self.mobs {
            let (family, volume, category) = S::voice(mob);
            resolve_voices(out, &mut mob.core.voices, family, volume, category);
        }
    }

    pub fn bodies_mut<'a>(&'a mut self, out: &mut Vec<&'a mut Body>) {
        out.extend(self.mobs.iter_mut().map(|mob| &mut mob.core.body));
    }

    pub fn positions(&self, out: &mut Vec<(u64, DVec3)>) {
        out.extend(
            self.mobs
                .iter()
                .map(|mob| (mob.core.id, mob.core.body.position)),
        );
    }

    pub fn cores<'a>(&'a self, out: &mut Vec<(u8, &'a MobCore)>) {
        out.extend(self.mobs.iter().map(|mob| (S::KEY, &mob.core)));
    }
}

impl<S: Family> Exposed for FamilyMob<S> {
    fn body(&mut self) -> &mut Body {
        &mut self.core.body
    }

    fn alive(&self) -> bool {
        self.core.health > 0.0
    }

    fn hurt_hazard(
        &mut self,
        hazard: Hazard,
        amount: f32,
        _world: &dyn World,
        _game_time: i64,
    ) -> bool {
        if hazard.is_fire() && self.core.effects.has(MobEffect::FireResistance) {
            return false;
        }
        let source = if hazard.panics() {
            DamageSourceKind::Hazard
        } else {
            DamageSourceKind::Generic
        };
        let hurt = Hurt {
            bypasses_armor: hazard.bypasses_armor(),
            ..Hurt::new(amount, hazard.damage_type(), source)
        };
        S::hurt(self, &hurt).applied
    }

    fn random(&mut self) -> &mut LegacyRandom {
        &mut self.core.random
    }

    fn voices(&mut self) -> &mut Vec<(Voice, DVec3)> {
        &mut self.core.voices
    }

    fn fire_immune(&self) -> bool {
        S::fire_immune(self)
    }
}

macro_rules! any_group {
    ($self:ident . $method:ident ( $($arg:expr),* )) => {
        $self
            .pests
            .$method($($arg),*)
            .or_else(|| $self.trial.$method($($arg),*))
            .or_else(|| $self.deep_dark.$method($($arg),*))
            .or_else(|| $self.land.$method($($arg),*))
            .or_else(|| $self.water.$method($($arg),*))
            .or_else(|| $self.night.$method($($arg),*))
            .or_else(|| $self.bosses.$method($($arg),*))
    };
}

macro_rules! every_group {
    ($self:ident . $method:ident ( $($arg:expr),* )) => {
        $self.pests.$method($($arg),*);
        $self.trial.$method($($arg),*);
        $self.deep_dark.$method($($arg),*);
        $self.land.$method($($arg),*);
        $self.water.$method($($arg),*);
        $self.night.$method($($arg),*);
        $self.bosses.$method($($arg),*);
    };
}

macro_rules! any_static {
    ($method:ident ( $($arg:expr),* )) => {
        pests::Group::$method($($arg),*)
            || trial::Group::$method($($arg),*)
            || deep_dark::Group::$method($($arg),*)
            || land::Group::$method($($arg),*)
            || water::Group::$method($($arg),*)
            || night::Group::$method($($arg),*)
            || bosses::Group::$method($($arg),*)
    };
}

macro_rules! first_static {
    ($method:ident ( $($arg:expr),* )) => {
        pests::Group::$method($($arg),*)
            .or_else(|| trial::Group::$method($($arg),*))
            .or_else(|| deep_dark::Group::$method($($arg),*))
            .or_else(|| land::Group::$method($($arg),*))
            .or_else(|| water::Group::$method($($arg),*))
            .or_else(|| night::Group::$method($($arg),*))
            .or_else(|| bosses::Group::$method($($arg),*))
    };
}

#[derive(Clone, Default)]
pub struct Families {
    pub pests: pests::Group,
    pub trial: trial::Group,
    pub deep_dark: deep_dark::Group,
    pub land: land::Group,
    pub water: water::Group,
    pub night: night::Group,
    pub bosses: bosses::Group,
}

pub fn kinds() -> Vec<&'static str> {
    let mut out = pests::Group::kinds();
    out.extend(trial::Group::kinds());
    out.extend(deep_dark::Group::kinds());
    out.extend(land::Group::kinds());
    out.extend(water::Group::kinds());
    out.extend(night::Group::kinds());
    out.extend(bosses::Group::kinds());
    out
}

pub fn knows(kind: &str) -> bool {
    key_of_kind(kind).is_some()
}

pub fn key_of_kind(kind: &str) -> Option<u8> {
    first_static!(key_of_kind(kind))
}

pub fn has_key(key: u8) -> bool {
    any_static!(has_key(key))
}

pub fn enemy(kind: &str) -> bool {
    any_static!(enemy(kind))
}

pub fn targetable(kind: &str) -> bool {
    !any_static!(untargetable(kind))
}

pub fn display_name(kind: &str) -> Option<&'static str> {
    first_static!(display_name(kind))
}

pub fn is_vehicle(kind: &str) -> bool {
    any_static!(is_vehicle(kind))
}

pub fn monster_spec(kind: &str) -> Option<&'static MonsterSpec> {
    first_static!(monster_spec(kind))
}

pub fn category(key: u8, kind: &str) -> Option<&'static str> {
    first_static!(category(key, kind))
}

pub fn key_ranges() -> Vec<(&'static str, RangeInclusive<u8>, Vec<u8>)> {
    vec![
        (
            pests::Group::NAME,
            pests::Group::RANGE,
            pests::Group::keys(),
        ),
        (
            trial::Group::NAME,
            trial::Group::RANGE,
            trial::Group::keys(),
        ),
        (
            deep_dark::Group::NAME,
            deep_dark::Group::RANGE,
            deep_dark::Group::keys(),
        ),
        (land::Group::NAME, land::Group::RANGE, land::Group::keys()),
        (
            water::Group::NAME,
            water::Group::RANGE,
            water::Group::keys(),
        ),
        (
            night::Group::NAME,
            night::Group::RANGE,
            night::Group::keys(),
        ),
        (
            bosses::Group::NAME,
            bosses::Group::RANGE,
            bosses::Group::keys(),
        ),
    ]
}

impl Families {
    pub fn spawn(world: &mut EntityWorld, kind: &str, position: DVec3) -> Option<u64> {
        pests::Group::spawn(world, kind, position)
            .or_else(|| trial::Group::spawn(world, kind, position))
            .or_else(|| deep_dark::Group::spawn(world, kind, position))
            .or_else(|| land::Group::spawn(world, kind, position))
            .or_else(|| water::Group::spawn(world, kind, position))
            .or_else(|| night::Group::spawn(world, kind, position))
            .or_else(|| bosses::Group::spawn(world, kind, position))
    }

    pub fn tick<W: World>(
        world: &mut EntityWorld,
        key: u8,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) -> bool {
        pests::Group::tick(world, key, id, &mut *level, players, ticks)
            || trial::Group::tick(world, key, id, &mut *level, players, ticks)
            || deep_dark::Group::tick(world, key, id, &mut *level, players, ticks)
            || land::Group::tick(world, key, id, &mut *level, players, ticks)
            || water::Group::tick(world, key, id, &mut *level, players, ticks)
            || night::Group::tick(world, key, id, &mut *level, players, ticks)
            || bosses::Group::tick(world, key, id, &mut *level, players, ticks)
    }

    pub fn key_of(&self, id: u64) -> Option<u8> {
        any_group!(self.key_of(id))
    }

    pub fn core(&self, id: u64) -> Option<(u8, &MobCore)> {
        let key = self.key_of(id)?;
        self.core_in(key, id).map(|core| (key, core))
    }

    pub fn core_mut(&mut self, id: u64) -> Option<(u8, &mut MobCore)> {
        let key = self.key_of(id)?;
        self.core_in_mut(key, id).map(|core| (key, core))
    }

    pub fn core_in(&self, key: u8, id: u64) -> Option<&MobCore> {
        any_group!(self.core_in(key, id))
    }

    pub fn core_in_mut(&mut self, key: u8, id: u64) -> Option<&mut MobCore> {
        any_group!(self.core_in_mut(key, id))
    }

    pub fn hurt(&mut self, key: u8, id: u64, hurt: &Hurt) -> Option<DamageResult> {
        let result = any_group!(self.hurt(key, id, hurt))?;
        if let (true, Some(attacker)) = (result.applied, hurt.attacker) {
            if let Some(core) = self.core_in_mut(key, id) {
                core.hurt_by = Some((attacker, core.tick_count));
                core.no_action_time = 0;
            }
        }
        Some(result)
    }

    pub fn traits(&self, id: u64) -> Option<MobTraits> {
        let key = self.key_of(id)?;
        any_group!(self.traits(key, id))
    }

    pub fn effects_mut(&mut self, id: u64) -> Option<(&mut MobEffects, bool, bool)> {
        let key = self.key_of(id)?;
        any_group!(self.effects_mut(key, id))
    }

    pub(crate) fn exposed_mut(&mut self, key: u8, id: u64) -> Option<&mut dyn Exposed> {
        any_group!(self.exposed_mut(key, id))
    }

    pub fn push_body(&mut self, key: u8, id: u64) -> Option<(&mut Body, bool)> {
        any_group!(self.push_body(key, id))
    }

    pub fn push_body_ref(&self, key: u8, id: u64) -> Option<&Body> {
        any_group!(self.push_body_ref(key, id))
    }

    pub fn passenger_point(&self, id: u64, seat: usize) -> Option<DVec3> {
        let key = self.key_of(id)?;
        any_group!(self.passenger_point(key, id, seat))
    }

    pub fn dismount(
        &self,
        id: u64,
        level: &dyn World,
        width: f32,
        heights: &[f32],
    ) -> Option<DVec3> {
        let key = self.key_of(id)?;
        any_group!(self.dismount(key, id, level, width, heights))
    }

    pub fn controller(&self, id: u64, passengers: &[Rider]) -> Option<Rider> {
        let key = self.key_of(id)?;
        any_group!(self.controller(key, id, passengers))
    }

    pub fn rider_jump(&mut self, id: u64, power: i32) -> bool {
        let Some(key) = self.key_of(id) else {
            return false;
        };
        self.pests.rider_jump(key, id, power)
            || self.trial.rider_jump(key, id, power)
            || self.deep_dark.rider_jump(key, id, power)
            || self.land.rider_jump(key, id, power)
            || self.water.rider_jump(key, id, power)
            || self.night.rider_jump(key, id, power)
            || self.bosses.rider_jump(key, id, power)
    }

    pub fn rider_order(&mut self, id: u64, target: DVec3, speed: f64) -> bool {
        let Some(key) = self.key_of(id) else {
            return false;
        };
        self.pests.rider_order(key, id, target, speed)
            || self.trial.rider_order(key, id, target, speed)
            || self.deep_dark.rider_order(key, id, target, speed)
            || self.land.rider_order(key, id, target, speed)
            || self.water.rider_order(key, id, target, speed)
            || self.night.rider_order(key, id, target, speed)
            || self.bosses.rider_order(key, id, target, speed)
    }

    pub fn leash_walk<W: World>(&mut self, id: u64, level: &W, target: DVec3, speed: f64) -> bool {
        let Some(key) = self.key_of(id) else {
            return false;
        };
        self.pests.leash_walk(key, id, level, target, speed)
            || self.trial.leash_walk(key, id, level, target, speed)
            || self.deep_dark.leash_walk(key, id, level, target, speed)
            || self.land.leash_walk(key, id, level, target, speed)
            || self.water.leash_walk(key, id, level, target, speed)
            || self.night.leash_walk(key, id, level, target, speed)
            || self.bosses.leash_walk(key, id, level, target, speed)
    }

    pub fn for_each(&self, f: &mut dyn FnMut(u8, &MobCore)) {
        every_group!(self.for_each(f));
    }

    pub fn for_each_mut(&mut self, f: &mut dyn FnMut(u8, &mut MobCore)) {
        every_group!(self.for_each_mut(f));
    }

    pub fn census(&self, f: &mut dyn FnMut(&MobCore, &Despawn)) {
        every_group!(self.census(f));
    }

    pub fn retain_mut(&mut self, f: &mut dyn FnMut(u8, &mut MobCore, &Despawn) -> bool) {
        every_group!(self.retain_mut(f));
    }

    pub fn clone_where(&self, keep: &dyn Fn(DVec3) -> bool) -> Self {
        Self {
            pests: self.pests.clone_where(keep),
            trial: self.trial.clone_where(keep),
            deep_dark: self.deep_dark.clone_where(keep),
            land: self.land.clone_where(keep),
            water: self.water.clone_where(keep),
            night: self.night.clone_where(keep),
            bosses: self.bosses.clone_where(keep),
        }
    }

    pub fn take_deaths(&mut self, out: &mut Vec<(u64, MobDeath)>, mob_drops: bool) {
        every_group!(self.take_deaths(out, mob_drops));
    }

    pub fn take_sounds(&mut self, out: &mut Vec<MobSound>) {
        every_group!(self.take_sounds(out));
    }

    pub fn bodies_mut(&mut self) -> Vec<&mut Body> {
        let mut out = Vec::new();
        every_group!(self.bodies_mut(&mut out));
        out
    }

    pub fn positions(&self) -> Vec<(u64, DVec3)> {
        let mut out = Vec::new();
        every_group!(self.positions(&mut out));
        out
    }

    pub fn cores(&self) -> Vec<(u8, &MobCore)> {
        let mut out = Vec::new();
        every_group!(self.cores(&mut out));
        out
    }

    pub fn ids(&self) -> Vec<u64> {
        self.positions().into_iter().map(|(id, _)| id).collect()
    }
}

impl EntityWorld {
    pub fn spawn_family<S: Registered>(&mut self, mob: FamilyMob<S>) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut mob = mob;
        mob.core.id = id;
        mob.core.previous_position = mob.core.body.position;
        S::store_mut(self).mobs.push(mob);
        self.order.push(EntityKey::Family(S::KEY, id));
        self.file_in_section(EntityKey::Family(S::KEY, id));
        id
    }

    pub fn spawn_family_kind(&mut self, kind: &str, position: DVec3) -> Option<u64> {
        Families::spawn(self, kind, position)
    }

    pub fn family<S: Registered>(&self) -> &[FamilyMob<S>] {
        &S::store(self).mobs
    }

    pub fn family_mob_mut<S: Registered>(&mut self, id: u64) -> Option<&mut FamilyMob<S>> {
        S::store_mut(self).get_mut(id)
    }

    pub fn family_core(&self, id: u64) -> Option<(u8, &MobCore)> {
        self.families.core(id)
    }

    pub fn family_core_mut(&mut self, id: u64) -> Option<(u8, &mut MobCore)> {
        self.families.core_mut(id)
    }

    pub fn family_cores(&self) -> Vec<(u8, &MobCore)> {
        self.families.cores()
    }

    pub fn family_traits(&self, id: u64) -> Option<MobTraits> {
        self.families.traits(id)
    }

    pub fn hurt_family(&mut self, id: u64, hurt: &Hurt) -> Option<DamageResult> {
        let key = self.families.key_of(id)?;
        self.families.hurt(key, id, hurt)
    }

    pub fn family_base_tick<S: Registered, W: World>(
        &mut self,
        id: u64,
        level: &W,
        players: &[PlayerCandidate],
    ) -> Option<BaseTick> {
        let game_time = self.game_time;
        let mob = S::store_mut(self).get_mut(id)?;
        let kind = mob.core.kind;
        let monster = S::category(kind) == "monster";
        let sounds = movement_sounds::<S>(kind);
        let core = &mut mob.core;
        core.previous_position = core.body.position;
        core.tick_count += 1;
        base_tick_fluid(
            &mut core.body,
            level,
            core.tick_count == 1,
            &mut core.random,
            &mut core.voices,
            sounds,
        );
        hazards::burn(mob, level, game_time);
        let eye = mob.core.eye_height;
        hazards::suffocate(mob, level, eye, game_time);
        let breathes = S::breathes_water(mob) || mob.core.effects.has(MobEffect::WaterBreathing);
        if mob.core.health > 0.0 && breathe(&mut mob.core.body, level, eye, breathes) {
            let drown =
                Hurt::new(2.0, "minecraft:drown", DamageSourceKind::Generic).bypassing_armor();
            S::hurt(mob, &drown);
        }
        let removed = mob.core.damage.tick();
        let tick_count = mob.core.tick_count;
        if mob
            .core
            .hurt_by
            .is_some_and(|(_, when)| tick_count - when > 100)
        {
            mob.core.hurt_by = None;
        }
        let undead = S::undead(mob);
        let max_health = mob.core.max_health;
        for work in mob.core.effects.tick(undead) {
            match work.resolve(mob.core.health, max_health) {
                Some(EffectWork::Heal(amount)) => heal(&mut mob.core.health, max_health, amount),
                Some(EffectWork::HurtMagic(amount)) => {
                    let magic = Hurt::new(amount, "minecraft:magic", DamageSourceKind::Magic)
                        .bypassing_armor();
                    S::hurt(mob, &magic);
                }
                _ => {}
            }
        }
        let core = &mut mob.core;
        core.body.float_from(&core.effects);
        if core.health > 0.0 {
            if (core.random.next_int(1000) as i32) < core.ambient_sound_time {
                core.ambient_sound_time = -core.ambient_interval;
                let voice = Voice::Ambient(voice_pitch(&mut core.random, core.baby));
                core.voices.push((voice, core.body.position));
            } else {
                core.ambient_sound_time += 1;
            }
        }
        if removed {
            return None;
        }
        let alive = core.health > 0.0;
        if !alive && !core.no_ai {
            let _ = dying_travel(
                &mut core.body,
                level,
                core.yaw,
                core.tick_count,
                &mut core.random,
                &mut core.voices,
                sounds,
                Some(monster),
            );
        } else if core.no_ai {
            core.body.trim_small_velocity();
            if monster {
                monster_idle_without_ai(
                    level,
                    players,
                    core.body.position,
                    core.eye_height,
                    &mut core.no_action_time,
                );
            } else if players.iter().any(|p| {
                p.alive
                    && !p.spectator
                    && p.position.distance_squared(core.body.position) < 32.0 * 32.0
            }) {
                core.no_action_time = 0;
            }
        }
        Some(BaseTick {
            alive,
            no_ai: core.no_ai,
            game_time,
        })
    }

    pub fn family_goal_step<S: Registered, W: World>(
        &mut self,
        id: u64,
        level: &W,
        players: &[PlayerCandidate],
        movement_speed: f64,
        ai: fn(&mut S) -> &mut MonsterAi,
    ) -> Option<Target> {
        let (game_time, difficulty) = (self.game_time, self.difficulty);
        let (griefing, bright) = (self.mob_griefing, self.bright_outside);
        let mobs = self.mob_candidates();
        let mob = S::store_mut(self).get_mut(id)?;
        let kind = mob.core.kind;
        let monster = S::category(kind) == "monster";
        let sounds = movement_sounds::<S>(kind);
        let FamilyMob { core, state } = &mut *mob;
        let ai = ai(state);
        ai.yaw = core.yaw;
        ai.state.mobs = mobs;
        ai.state.hurt_by = core.hurt_by;
        ai.state.max_health = core.max_health;
        ai.state.eye_height = core.eye_height;
        ai.state.tick_count = core.tick_count;
        ai.state.mob_griefing = griefing;
        ai.state.bright_outside = bright;
        ai.state.on_fire = core.body.fire_ticks > 0;
        let step = MonsterStep {
            players,
            villagers: &[],
            game_time,
            difficulty,
            movement_speed: core.effects.movement_speed(movement_speed),
            sounds,
        };
        let landed = monster_ai_step(
            ai,
            &mut core.body,
            core.health,
            &mut core.random,
            &mut core.voices,
            &mut core.no_action_time,
            core.previous_position,
            core.tick_count,
            core.id,
            level,
            &step,
        );
        core.yaw = ai.yaw;
        core.look = ai.state.look_control.clone();
        core.hurt_by = ai.state.hurt_by;
        let strike = ai.state.attack.take();
        if let Some(damage) = landed
            .and_then(|fallen| fall_damage(&core.body, level, fallen, monster, &mut core.voices))
        {
            let fall =
                Hurt::new(damage, "minecraft:fall", DamageSourceKind::Generic).bypassing_armor();
            S::hurt(mob, &fall);
        }
        strike
    }

    pub fn family_alert<S: Registered>(
        &mut self,
        id: u64,
        attacker: Target,
        ai: fn(&mut S) -> &mut MonsterAi,
    ) {
        let store = S::store_mut(self);
        let Some(source) = store.get_mut(id) else {
            return;
        };
        let (kind, p) = (source.core.kind, source.core.body.position);
        let range = ai(&mut source.state).state.follow_range;
        let min = p - DVec3::new(range, 10.0, range);
        let max = p + DVec3::new(1.0 + range, 11.0, 1.0 + range);
        for other in &mut store.mobs {
            if other.core.id == id || other.core.kind != kind {
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
            let ai = ai(&mut other.state);
            if touches && ai.state.target().is_none() {
                ai.state.set_target(Some(attacker));
            }
        }
    }

    pub fn family_after_move<S: Registered, W: World>(
        &mut self,
        id: u64,
        level: &W,
        game_time: i64,
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        if let Some(mob) = S::store_mut(self).get_mut(id) {
            let old = mob.core.previous_position;
            hazards::blocks_act(mob, level, old, game_time);
        }
        self.push_entities(EntityKey::Family(S::KEY, id), level, game_time, ticks);
    }

    pub fn family_strike(
        &mut self,
        id: u64,
        target: Target,
        damage: f32,
        kind: PlayerHitKind,
    ) -> Option<DamageResult> {
        let (_, core) = self.families.core(id)?;
        let position = core.body.position;
        let result = match target {
            Target::Player(player_id) => {
                self.player_hits.push(PlayerHit {
                    player_id,
                    damage,
                    kind,
                    source: Some(id),
                });
                None
            }
            Target::Mob(victim) | Target::Villager(victim) => {
                self.mob_hits_mob(id, victim, damage, position, 0.0)
            }
        };
        if let Some((_, core)) = self.families.core_mut(id) {
            core.swings += 1;
        }
        result
    }
}
