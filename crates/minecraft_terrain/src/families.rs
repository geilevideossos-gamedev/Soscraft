use crate::ambient_particles::AmbientParticles;
use crate::block_particles::BlockDust;
use crate::client_mobs::{ClientMobs, ServerMob};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, HeldItem};
use crate::mob_actions::{Actor, MobOutcome};
use crate::mob_tracking::MobShow;
use crate::poof_particles::PoofParticles;
use crate::portal_particles::PortalParticles;
use crate::spell_particles::SpellParticles;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::look::LookControl;
use minecraftoss_entities::movement::Body;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::{FamilyMob, Registered};
use minecraftoss_player::rng::LegacyRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

        #[allow(unused_mut)]
        pub fn textures() -> Vec<&'static str> {
            let mut out = Vec::new();
            $($(#[$attr])* out.extend_from_slice(<$ty as $crate::families::TerrainFamily>::TEXTURES);)*
            out
        }

        #[allow(unused_variables)]
        pub fn draw(
            out: &mut $crate::families::FamilyDraw<'_>,
            world: &minecraftoss_entities::world::EntityWorld,
            poses: &$crate::client_mobs::ClientMobs,
            atlas: &$crate::mesh::Atlas,
            light: &$crate::lighting::SkyLight,
            partial: f32,
        ) {
            $($(#[$attr])* <$ty as $crate::families::TerrainFamily>::draw(out, world.family::<$ty>(), poses, atlas, light, partial);)*
        }

        #[allow(unused_variables)]
        pub fn draw_glowing(
            world: &minecraftoss_entities::world::EntityWorld,
            poses: &$crate::client_mobs::ClientMobs,
            atlas: &$crate::mesh::Atlas,
            light: &$crate::lighting::SkyLight,
            partial: f32,
            glows: &dyn Fn(u64) -> bool,
            each: &mut dyn FnMut(u64, &$crate::mesh::ChunkMesh),
        ) {
            $($(#[$attr])* $crate::families::glowing_of::<$ty>(world, poses, atlas, light, partial, glows, each);)*
        }

        #[allow(unused_variables, clippy::ptr_arg)]
        pub fn shadows(world: &minecraftoss_entities::world::EntityWorld, out: &mut Vec<(u64, f32)>) {
            $($(#[$attr])* $crate::families::shadows_of::<$ty>(world, out);)*
        }

        #[allow(unused_variables)]
        pub fn max_head_y_rot(key: u8) -> Option<f32> {
            $($(#[$attr])* if key == <$ty as minecraftoss_entities::world::families::Family>::KEY {
                return Some(<$ty as $crate::families::TerrainFamily>::MAX_HEAD_Y_ROT);
            })*
            None
        }

        #[allow(unused_variables)]
        pub fn cage_margin(kind: &str) -> Option<f32> {
            $($(#[$attr])* if $crate::families::names(<$ty as minecraftoss_entities::world::families::Family>::KINDS, kind) {
                return Some(<$ty as $crate::families::TerrainFamily>::CAGE_MARGIN);
            })*
            None
        }

        #[allow(unused_variables)]
        pub fn interact(
            world: &mut minecraftoss_entities::world::EntityWorld,
            key: u8,
            id: u64,
            actor: &mut $crate::mob_actions::Actor,
            outcome: &mut $crate::mob_actions::MobOutcome,
        ) -> Option<bool> {
            $($(#[$attr])* if key == <$ty as minecraftoss_entities::world::families::Family>::KEY {
                return Some(<$ty as $crate::families::TerrainFamily>::interact(world, id, actor, outcome));
            })*
            None
        }

        #[allow(unused_variables)]
        pub fn client_effects(
            world: &minecraftoss_entities::world::EntityWorld,
            poses: &$crate::client_mobs::ClientMobs,
            fx: &mut $crate::families::ClientFx<'_>,
        ) {
            $($(#[$attr])* <$ty as $crate::families::TerrainFamily>::client_effects(world.family::<$ty>(), poses, fx);)*
        }

        #[allow(unused_variables, clippy::ptr_arg)]
        pub fn block_dust(
            world: &minecraftoss_entities::world::EntityWorld,
            poses: &$crate::client_mobs::ClientMobs,
            out: &mut Vec<$crate::block_particles::BlockDust>,
        ) {
            $($(#[$attr])* <$ty as $crate::families::TerrainFamily>::block_dust(world.family::<$ty>(), poses, out);)*
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

#[cfg(test)]
mod tests;

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

pub struct FamilyDraw<'a> {
    pub models: &'a mut ChunkMesh,
    pub culled: &'a mut ChunkMesh,
    pub translucent: &'a mut ChunkMesh,
    pub held: &'a mut Vec<HeldItem>,
}

pub struct ClientFx<'a> {
    pub ambient: &'a mut AmbientParticles,
    pub spell: &'a mut SpellParticles,
    pub portal: &'a mut PortalParticles,
    pub poof: &'a mut PoofParticles,
    pub sounds: &'a mut Vec<(String, DVec3, f32, f32)>,
    pub random: &'a mut LegacyRandom,
    pub seen: &'a mut HashMap<u64, i32>,
}

pub trait TerrainFamily: Registered {
    const TRACKING_RANGE: i32;
    const VOLATILE: &'static [&'static str] = &[];
    const TEXTURES: &'static [&'static str];
    const SHADOW_RADIUS: f32;
    const MAX_HEAD_Y_ROT: f32 = 75.0;
    const CAGE_MARGIN: f32 = 0.05;

    fn read(_mob: &mut FamilyMob<Self>, _tag: &Tag) {}

    fn fields(_mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        Vec::new()
    }

    fn show(_mob: &FamilyMob<Self>) -> FamilyShow {
        FamilyShow::default()
    }

    fn show_on(_mob: &mut FamilyMob<Self>, _show: &FamilyShow) {}

    fn draw(
        out: &mut FamilyDraw<'_>,
        mobs: &[FamilyMob<Self>],
        poses: &ClientMobs,
        atlas: &Atlas,
        light: &SkyLight,
        partial: f32,
    );

    fn interact(
        _world: &mut EntityWorld,
        _id: u64,
        _actor: &mut Actor,
        _outcome: &mut MobOutcome,
    ) -> bool {
        false
    }

    fn client_effects(_mobs: &[FamilyMob<Self>], _poses: &ClientMobs, _fx: &mut ClientFx<'_>) {}

    fn block_dust(_mobs: &[FamilyMob<Self>], _poses: &ClientMobs, _out: &mut Vec<BlockDust>) {}
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

pub fn knows(kind: &str) -> bool {
    kinds().contains(&kind)
}

pub fn textures() -> Vec<&'static str> {
    let mut out = pests::textures();
    out.extend(trial::textures());
    out.extend(deep_dark::textures());
    out.extend(land::textures());
    out.extend(water::textures());
    out.extend(night::textures());
    out.extend(bosses::textures());
    out
}

pub fn draw(
    out: &mut FamilyDraw<'_>,
    world: &EntityWorld,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    pests::draw(out, world, poses, atlas, light, partial);
    trial::draw(out, world, poses, atlas, light, partial);
    deep_dark::draw(out, world, poses, atlas, light, partial);
    land::draw(out, world, poses, atlas, light, partial);
    water::draw(out, world, poses, atlas, light, partial);
    night::draw(out, world, poses, atlas, light, partial);
    bosses::draw(out, world, poses, atlas, light, partial);
}

pub fn draw_glowing(
    world: &EntityWorld,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
    glows: &dyn Fn(u64) -> bool,
    each: &mut dyn FnMut(u64, &ChunkMesh),
) {
    pests::draw_glowing(world, poses, atlas, light, partial, glows, each);
    trial::draw_glowing(world, poses, atlas, light, partial, glows, each);
    deep_dark::draw_glowing(world, poses, atlas, light, partial, glows, each);
    land::draw_glowing(world, poses, atlas, light, partial, glows, each);
    water::draw_glowing(world, poses, atlas, light, partial, glows, each);
    night::draw_glowing(world, poses, atlas, light, partial, glows, each);
    bosses::draw_glowing(world, poses, atlas, light, partial, glows, each);
}

pub fn shadows(world: &EntityWorld, out: &mut Vec<(u64, f32)>) {
    pests::shadows(world, out);
    trial::shadows(world, out);
    deep_dark::shadows(world, out);
    land::shadows(world, out);
    water::shadows(world, out);
    night::shadows(world, out);
    bosses::shadows(world, out);
}

pub fn max_head_y_rot(world: &EntityWorld, id: u64) -> Option<f32> {
    let (key, _) = world.family_core(id)?;
    pests::max_head_y_rot(key)
        .or_else(|| trial::max_head_y_rot(key))
        .or_else(|| deep_dark::max_head_y_rot(key))
        .or_else(|| land::max_head_y_rot(key))
        .or_else(|| water::max_head_y_rot(key))
        .or_else(|| night::max_head_y_rot(key))
        .or_else(|| bosses::max_head_y_rot(key))
}

pub fn cage_margin(kind: &str) -> Option<f32> {
    pests::cage_margin(kind)
        .or_else(|| trial::cage_margin(kind))
        .or_else(|| deep_dark::cage_margin(kind))
        .or_else(|| land::cage_margin(kind))
        .or_else(|| water::cage_margin(kind))
        .or_else(|| night::cage_margin(kind))
        .or_else(|| bosses::cage_margin(kind))
}

pub fn interact(
    world: &mut EntityWorld,
    key: u8,
    id: u64,
    actor: &mut Actor,
    outcome: &mut MobOutcome,
) -> bool {
    pests::interact(world, key, id, actor, outcome)
        .or_else(|| trial::interact(world, key, id, actor, outcome))
        .or_else(|| deep_dark::interact(world, key, id, actor, outcome))
        .or_else(|| land::interact(world, key, id, actor, outcome))
        .or_else(|| water::interact(world, key, id, actor, outcome))
        .or_else(|| night::interact(world, key, id, actor, outcome))
        .or_else(|| bosses::interact(world, key, id, actor, outcome))
        .unwrap_or(false)
}

pub fn client_effects(world: &EntityWorld, poses: &ClientMobs, fx: &mut ClientFx<'_>) {
    fx.seen.retain(|id, _| world.family_core(*id).is_some());
    pests::client_effects(world, poses, fx);
    trial::client_effects(world, poses, fx);
    deep_dark::client_effects(world, poses, fx);
    land::client_effects(world, poses, fx);
    water::client_effects(world, poses, fx);
    night::client_effects(world, poses, fx);
    bosses::client_effects(world, poses, fx);
}

pub fn block_dust(world: &EntityWorld, poses: &ClientMobs, out: &mut Vec<BlockDust>) {
    pests::block_dust(world, poses, out);
    trial::block_dust(world, poses, out);
    deep_dark::block_dust(world, poses, out);
    land::block_dust(world, poses, out);
    water::block_dust(world, poses, out);
    night::block_dust(world, poses, out);
    bosses::block_dust(world, poses, out);
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

fn glowing_of<S: TerrainFamily>(
    world: &EntityWorld,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
    glows: &dyn Fn(u64) -> bool,
    each: &mut dyn FnMut(u64, &ChunkMesh),
) {
    let shown = world.family::<S>().iter();
    for mob in shown.filter(|mob| !mob.core.removed && glows(mob.core.id)) {
        let mut models = ChunkMesh::default();
        let mut culled = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let mut held = Vec::new();
        let mut out = FamilyDraw {
            models: &mut models,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut held,
        };
        S::draw(
            &mut out,
            std::slice::from_ref(mob),
            poses,
            atlas,
            light,
            partial,
        );
        merge(&mut models, culled);
        merge(&mut models, translucent);
        each(mob.core.id, &models);
    }
}

fn merge(into: &mut ChunkMesh, from: ChunkMesh) {
    let base = into.vertices.len() as u32;
    into.vertices.extend(from.vertices);
    into.indices
        .extend(from.indices.iter().map(|index| index + base));
    into.faces += from.faces;
}

fn shadows_of<S: TerrainFamily>(world: &EntityWorld, out: &mut Vec<(u64, f32)>) {
    for mob in world.family::<S>().iter().filter(|mob| !mob.core.removed) {
        let age = if mob.core.baby { 0.5 } else { 1.0 };
        out.push((mob.core.id, S::SHADOW_RADIUS * age));
    }
}
