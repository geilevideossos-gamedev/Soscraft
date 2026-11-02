use super::endermite_render;
use crate::client_mobs::ClientMobs;
use crate::families::{ClientFx, FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use crate::thrown::PlayerBody;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::pests::endermites::Endermite;
use minecraftoss_entities::world::families::pests::endermites::{
    KIND, PEARL_CHANCE, player_rotation,
};
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_world::level::Level;
use minecraftoss_world::natural_spawner::SpawnLevel;
use std::collections::{BTreeMap, HashMap};

pub(crate) fn pearl_leaves_one(random: &mut LegacyRandom) -> bool {
    let _ = crate::thrown::pearl_portal(DVec3::ZERO, random);
    random.next_float() < PEARL_CHANCE
}

pub(crate) fn mite_tag(feet: DVec3, (yaw, pitch): (f32, f32), uuid: [i32; 4]) -> Tag {
    let mut map = BTreeMap::new();
    map.insert("id".to_owned(), Tag::String(KIND.to_owned()));
    map.insert(
        "Pos".to_owned(),
        Tag::List(feet.to_array().into_iter().map(Tag::Double).collect()),
    );
    map.insert(
        "Rotation".to_owned(),
        Tag::List(vec![Tag::Float(yaw), Tag::Float(pitch)]),
    );
    map.insert("UUID".to_owned(), Tag::IntArray(uuid.to_vec()));
    Tag::Compound(map)
}

pub(crate) fn from_pearls(
    level: &mut Level<'_>,
    mobs: &mut EntityWorld,
    tags: &mut HashMap<u64, Tag>,
    throwers: &[u64],
    players: &[PlayerBody],
) {
    let spawning = level
        .natural_spawning
        .as_ref()
        .is_some_and(|natural| natural.spawn_mobs && natural.spawn_monsters);
    if !spawning || level.difficulty == 0 {
        return;
    }
    for owner in throwers {
        let Some(thrower) = players.iter().find(|p| p.id == *owner && p.alive) else {
            continue;
        };
        let rotation = player_rotation(mobs, *owner);
        let tag = mite_tag(thrower.feet, rotation, SpawnLevel::next_uuid(level));
        tags.extend(crate::server_mobs::spawn_saved_all(mobs, &tag));
    }
}

impl TerrainFamily for Endermite {
    const TRACKING_RANGE: i32 = 8;
    const VOLATILE: &'static [&'static str] = &["Lifetime"];
    const TEXTURES: &'static [&'static str] = &[endermite_render::TEXTURE];
    const SHADOW_RADIUS: f32 = endermite_render::SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        mob.state.ai.state.look_control.head_yaw = mob.core.yaw;
        mob.state.life = tag
            .get("Lifetime")
            .and_then(Tag::as_i64)
            .map_or(0, |life| life as i32);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        vec![("Lifetime", Tag::Int(mob.state.life))]
    }

    fn draw(
        out: &mut FamilyDraw<'_>,
        mobs: &[FamilyMob<Self>],
        poses: &ClientMobs,
        atlas: &Atlas,
        light: &SkyLight,
        partial: f32,
    ) {
        let shown = mobs.iter().filter(|mob| !mob.core.removed);
        let ids = shown.map(|mob| mob.core.id);
        endermite_render::append_endermites(out.models, ids, poses, atlas, light, partial);
    }

    fn client_effects(mobs: &[FamilyMob<Self>], poses: &ClientMobs, fx: &mut ClientFx<'_>) {
        for mob in mobs.iter().filter(|mob| !mob.core.removed) {
            let Some(shown) = poses.get(mob.core.id) else {
                continue;
            };
            for (start, motion) in endermite_render::portal_sparks(shown.position, fx.random) {
                fx.portal.emit(start, motion);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_landing_pearl_draws_its_portal_specks_then_rolls_one_in_twenty() {
        let mut hits = 0;
        for seed in 0..20_000 {
            let mut random = LegacyRandom::new(seed);
            let mut replay = LegacyRandom::new(seed);
            for _ in 0..32 {
                let _ = (replay.next_double(), replay.next_gaussian());
                let _ = replay.next_gaussian();
            }
            let roll = replay.next_float();
            let left = pearl_leaves_one(&mut random);
            assert_eq!(left, roll < 0.05, "seed {seed}");
            assert_eq!(random.next_int(1000), replay.next_int(1000), "seed {seed}");
            hits += usize::from(left);
        }
        assert!((900..=1100).contains(&hits), "{hits} of 20000");
    }

    #[test]
    fn a_pearl_endermite_takes_its_thrower_s_place_and_turn() {
        let tag = mite_tag(DVec3::new(4.25, 70.0, -9.75), (135.0, 20.0), [5, 6, 7, 8]);
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some(KIND));
        let mut world = EntityWorld::default();
        let spawned = crate::server_mobs::spawn_saved_all(&mut world, &tag);
        assert_eq!(spawned.len(), 1);
        let mite = &world.family::<Endermite>()[0];
        assert_eq!(mite.core.body.position, DVec3::new(4.25, 70.0, -9.75));
        assert_eq!(mite.core.yaw, 135.0);
        assert_eq!(mite.state.life, 0);
        assert!(!mite.core.persistent);
    }
}
