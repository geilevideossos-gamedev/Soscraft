use crate::mob_particles::{MobPuff, SPAWN_ANIM, mob_puffs};
use crate::server::HowBroken;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{BlockPos, BlockStateId};
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::pests::silverfish;
use minecraftoss_world::level::Level;
use minecraftoss_world::natural_spawner::SpawnLevel;
use std::collections::{BTreeMap, HashMap};

pub const INFESTED_BLOCK: &str = "InfestedBlock";

pub fn is_infested(level: &Level<'_>, state: BlockStateId) -> bool {
    let blocks = &level.registries().blocks;
    blocks.block(blocks.block_of(state)).is_a(INFESTED_BLOCK)
}

pub fn player_broke(
    level: &mut Level<'_>,
    at: &[BlockPos],
    old: &[BlockStateId],
    broken: &[(crate::scene::BlockPos, HowBroken)],
) {
    for ((pos, state), (_, how)) in at.iter().zip(old).zip(broken) {
        if *how != HowBroken::SilkTouched && level.block_drops && is_infested(level, *state) {
            level.infestations.push(*pos);
        }
    }
}

pub fn silverfish_tag(pos: BlockPos, uuid: [i32; 4]) -> Tag {
    let feet = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y),
        f64::from(pos.z) + 0.5,
    ];
    let mut map = BTreeMap::new();
    map.insert("id".to_owned(), Tag::String(silverfish::KIND.to_owned()));
    map.insert(
        "Pos".to_owned(),
        Tag::List(feet.into_iter().map(Tag::Double).collect()),
    );
    map.insert(
        "Rotation".to_owned(),
        Tag::List(vec![Tag::Float(0.0), Tag::Float(0.0)]),
    );
    map.insert("UUID".to_owned(), Tag::IntArray(uuid.to_vec()));
    Tag::Compound(map)
}

pub fn release(
    level: &mut Level<'_>,
    mobs: &mut EntityWorld,
    tags: &mut HashMap<u64, Tag>,
    puffs: &mut Vec<MobPuff>,
) {
    for pos in std::mem::take(&mut level.infestations) {
        let tag = silverfish_tag(pos, SpawnLevel::next_uuid(level));
        for (id, tag) in crate::server_mobs::spawn_saved_all(mobs, &tag) {
            puffs.extend(mob_puffs(mobs, &[(id, SPAWN_ANIM)]));
            tags.insert(id, tag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_released_silverfish_stands_in_the_middle_of_its_block_facing_south() {
        let tag = silverfish_tag(BlockPos::new(-3, 40, 7), [1, 2, 3, 4]);
        assert_eq!(
            tag.get("id").and_then(Tag::as_str),
            Some("minecraft:silverfish")
        );
        let pos: Vec<f64> = tag
            .get("Pos")
            .and_then(Tag::as_list)
            .unwrap()
            .iter()
            .filter_map(Tag::as_f64)
            .collect();
        assert_eq!(pos, [-2.5, 40.0, 7.5]);
        let rotation = tag.get("Rotation").and_then(Tag::as_list).unwrap();
        assert_eq!(rotation, [Tag::Float(0.0), Tag::Float(0.0)]);
        assert_eq!(
            tag.get("UUID").and_then(Tag::as_ints),
            Some(vec![1, 2, 3, 4])
        );
        let mut world = EntityWorld::default();
        let spawned = crate::server_mobs::spawn_saved_all(&mut world, &tag);
        assert_eq!(spawned.len(), 1);
        let (_, core) = world.family_core(spawned[0].0).unwrap();
        assert_eq!(core.kind, silverfish::KIND);
        assert_eq!(core.body.position, glam::DVec3::new(-2.5, 40.0, 7.5));
        assert_eq!(core.yaw, 0.0);
        assert!(!core.persistent);
    }
}
