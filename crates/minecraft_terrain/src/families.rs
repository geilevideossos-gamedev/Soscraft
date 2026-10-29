use crate::client_mobs::ServerMob;
use crate::mob_tracking::MobShow;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::look::LookControl;
use minecraftoss_entities::movement::Body;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::{FamilyMob, Registered};
use minecraftoss_player::rng::LegacyRandom;
use serde::{Deserialize, Serialize};

macro_rules! terrain_group {
    ($group:ident { $($(#[$attr:meta])* $ty:ty),* $(,)? }) => {
        #[allow(unused_mut)]
        pub fn kinds() -> Vec<&'static str> {
            let mut out = Vec::new();
            $($(#[$attr])* out.extend_from_slice(<$ty as minecraftoss_entities::world::families::Family>::KINDS);)*
            out
        }

        #[allow(unused_variables)]
        pub fn spawn_saved(
            world: &mut minecraftoss_entities::world::EntityWorld,
            kind: &str,
            tag: &minecraftoss_core::nbt::Tag,
            common: &$crate::families::Common,
        ) -> Option<u64> {
            $($(#[$attr])* if let Some(&kind) = <$ty as minecraftoss_entities::world::families::Family>::KINDS.iter().find(|k| **k == kind) {
                return $crate::families::spawn_one::<$ty>(world, kind, tag, common);
            })*
            None
        }

        #[allow(unused_variables)]
        pub fn save(world: &minecraftoss_entities::world::EntityWorld, add: &mut $crate::families::Add<'_>) {
            $($(#[$attr])* $crate::families::save_one::<$ty>(world, add);)*
        }

        #[allow(unused_variables)]
        pub fn tracking_range(kind: &str) -> Option<i32> {
            $($(#[$attr])* if $crate::families::names(<$ty as minecraftoss_entities::world::families::Family>::KINDS, kind) {
                return Some(<$ty as $crate::families::TerrainFamily>::TRACKING_RANGE);
            })*
            None
        }

        #[allow(unused_variables)]
        pub fn volatile(kind: &str) -> Option<&'static [&'static str]> {
            $($(#[$attr])* if $crate::families::names(<$ty as minecraftoss_entities::world::families::Family>::KINDS, kind) {
                return Some(<$ty as $crate::families::TerrainFamily>::VOLATILE);
            })*
            None
        }

        #[allow(unused_variables)]
        pub fn shows(world: &minecraftoss_entities::world::EntityWorld, put: &mut dyn FnMut(u64, $crate::mob_tracking::MobShow)) {
            $($(#[$attr])* $crate::families::shows_of::<$ty>(world, put);)*
        }

        #[allow(unused_variables)]
        pub fn show_on(world: &mut minecraftoss_entities::world::EntityWorld, mob: &$crate::client_mobs::ServerMob) -> bool {
            $($(#[$attr])* if $crate::families::show_on_one::<$ty>(world, mob) {
                return true;
            })*
            false
        }
    };
}

mod bosses;
mod deep_dark;
mod land;
mod night;
mod pests;
mod trial;
mod water;

pub struct Common {
    pub pos: DVec3,
    pub yaw: f32,
    pub health: Option<f32>,
    pub persistent: bool,
    pub no_ai: bool,
    pub random: LegacyRandom,
    pub fire: i32,
    pub age: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FamilyShow {
    pub flags: u32,
    pub values: [i32; 8],
}

pub type Add<'a> = dyn FnMut(u64, &str, &Body, f32, f32, bool, Vec<(&'static str, Tag)>) + 'a;

pub trait TerrainFamily: Registered {
    const TRACKING_RANGE: i32;
    const VOLATILE: &'static [&'static str] = &[];

    fn read(_mob: &mut FamilyMob<Self>, _tag: &Tag) {}

    fn fields(_mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        Vec::new()
    }

    fn show(_mob: &FamilyMob<Self>) -> FamilyShow {
        FamilyShow::default()
    }

    fn show_on(_mob: &mut FamilyMob<Self>, _show: &FamilyShow) {}
}

pub fn kinds() -> Vec<&'static str> {
    let mut out = pests::kinds();
    out.extend(trial::kinds());
    out.extend(deep_dark::kinds());
    out.extend(land::kinds());
    out.extend(water::kinds());
    out.extend(night::kinds());
    out.extend(bosses::kinds());
    out
}

pub fn spawn_saved(world: &mut EntityWorld, kind: &str, tag: &Tag, common: &Common) -> Option<u64> {
    if tag.get("Passengers").is_some() {
        return None;
    }
    pests::spawn_saved(world, kind, tag, common)
        .or_else(|| trial::spawn_saved(world, kind, tag, common))
        .or_else(|| deep_dark::spawn_saved(world, kind, tag, common))
        .or_else(|| land::spawn_saved(world, kind, tag, common))
        .or_else(|| water::spawn_saved(world, kind, tag, common))
        .or_else(|| night::spawn_saved(world, kind, tag, common))
        .or_else(|| bosses::spawn_saved(world, kind, tag, common))
}

pub fn save(world: &EntityWorld, add: &mut Add<'_>) {
    pests::save(world, add);
    trial::save(world, add);
    deep_dark::save(world, add);
    land::save(world, add);
    water::save(world, add);
    night::save(world, add);
    bosses::save(world, add);
}

pub fn tracking_range(kind: &str) -> Option<i32> {
    pests::tracking_range(kind)
        .or_else(|| trial::tracking_range(kind))
        .or_else(|| deep_dark::tracking_range(kind))
        .or_else(|| land::tracking_range(kind))
        .or_else(|| water::tracking_range(kind))
        .or_else(|| night::tracking_range(kind))
        .or_else(|| bosses::tracking_range(kind))
}

pub fn volatile(kind: &str) -> Option<&'static [&'static str]> {
    pests::volatile(kind)
        .or_else(|| trial::volatile(kind))
        .or_else(|| deep_dark::volatile(kind))
        .or_else(|| land::volatile(kind))
        .or_else(|| water::volatile(kind))
        .or_else(|| night::volatile(kind))
        .or_else(|| bosses::volatile(kind))
}

pub fn shows(world: &EntityWorld, put: &mut dyn FnMut(u64, MobShow)) {
    pests::shows(world, put);
    trial::shows(world, put);
    deep_dark::shows(world, put);
    land::shows(world, put);
    water::shows(world, put);
    night::shows(world, put);
    bosses::shows(world, put);
}

pub fn show_on(world: &mut EntityWorld, mob: &ServerMob) -> bool {
    pests::show_on(world, mob)
        || trial::show_on(world, mob)
        || deep_dark::show_on(world, mob)
        || land::show_on(world, mob)
        || water::show_on(world, mob)
        || night::show_on(world, mob)
        || bosses::show_on(world, mob)
}

fn names(kinds: &[&str], kind: &str) -> bool {
    kinds
        .iter()
        .any(|known| *known == kind || known.strip_prefix("minecraft:") == Some(kind))
}

fn spawn_one<S: TerrainFamily>(
    world: &mut EntityWorld,
    kind: &'static str,
    tag: &Tag,
    common: &Common,
) -> Option<u64> {
    let mut mob = S::spawn(kind, common.pos)?;
    let core = &mut mob.core;
    core.yaw = common.yaw;
    core.look = LookControl::new(common.yaw);
    if let Some(health) = common.health {
        core.health = health.min(core.max_health);
    }
    core.persistent = common.persistent;
    core.no_ai = common.no_ai;
    core.random = common.random.clone();
    core.body.fire_ticks = common.fire;
    core.baby = common.age < 0;
    S::read(&mut mob, tag);
    Some(world.spawn_family(mob))
}

fn save_one<S: TerrainFamily>(world: &EntityWorld, add: &mut Add<'_>) {
    for mob in world.family::<S>().iter().filter(|mob| !mob.core.removed) {
        let core = &mob.core;
        add(
            core.id,
            core.kind,
            &core.body,
            core.yaw,
            core.health,
            core.persistent,
            S::fields(mob),
        );
    }
}

fn shows_of<S: TerrainFamily>(world: &EntityWorld, put: &mut dyn FnMut(u64, MobShow)) {
    for mob in world.family::<S>() {
        let show = MobShow {
            family: Some(S::show(mob)),
            ..MobShow::default()
        };
        put(mob.core.id, show);
    }
}

fn show_on_one<S: TerrainFamily>(world: &mut EntityWorld, mob: &ServerMob) -> bool {
    let Some(shown) = world.family_mob_mut::<S>(mob.id) else {
        return false;
    };
    let core = &mut shown.core;
    crate::mob_tracking::place(&mut core.previous_position, &mut core.body, mob);
    if let Some(show) = &mob.show.family {
        S::show_on(shown, show);
    }
    true
}
