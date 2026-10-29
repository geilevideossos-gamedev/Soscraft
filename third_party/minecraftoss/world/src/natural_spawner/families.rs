mod bosses;
mod deep_dark;
mod land;
mod night;
mod pests;
#[cfg(test)]
mod tests;
mod trial;
mod water;

use super::spawning::{DifficultyInstance, MobCategory, MonsterGroup, SpawnContext};
use super::{CreatureSpawns, Group, RandomRef, SpawnLevel, TypeInfo, ageable, set};
use minecraftoss_core::entity_data::finalize_mob;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{BiomeId, BlockPos, ChunkPos};

pub(super) type FamilyRule = fn(
    &CreatureSpawns,
    &mut dyn SpawnLevel,
    &SpawnContext,
    &str,
    &TypeInfo,
    BlockPos,
    bool,
) -> bool;

pub(super) type FamilyFinalize = fn(
    &CreatureSpawns,
    &mut dyn SpawnLevel,
    Option<(&SpawnContext, DifficultyInstance)>,
    &mut Tag,
    BlockPos,
    &mut Option<i32>,
);

pub(super) struct FamilySpawn {
    pub kind: &'static str,
    pub info: TypeInfo,
    pub category: MobCategory,
    pub follow_range: f64,
    pub baby_chance: Option<f32>,
    pub rule: Option<FamilyRule>,
    pub finalize: Option<FamilyFinalize>,
}

fn rows() -> impl Iterator<Item = &'static FamilySpawn> {
    let groups = [
        pests::ROWS,
        trial::ROWS,
        deep_dark::ROWS,
        land::ROWS,
        water::ROWS,
        night::ROWS,
        bosses::ROWS,
    ];
    #[cfg(test)]
    let groups = groups.into_iter().chain([tests::ROWS]);
    groups.into_iter().flatten()
}

pub(super) fn row(kind: &str) -> Option<&'static FamilySpawn> {
    let short = kind.trim_start_matches("minecraft:");
    rows().find(|row| row.kind.trim_start_matches("minecraft:") == short)
}

pub(super) fn rule(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    context: &SpawnContext,
    kind: &str,
    info: &TypeInfo,
    pos: BlockPos,
    spawner: bool,
) -> bool {
    match row(kind).and_then(|row| row.rule) {
        Some(rule) => rule(spawns, level, context, kind, info, pos, spawner),
        None => true,
    }
}

pub(super) fn generation_rule(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    kind: &str,
    info: &TypeInfo,
    pos: BlockPos,
) -> bool {
    let Some(rule) = row(kind).and_then(|row| row.rule) else {
        return spawns.check_spawn_rules(level, info, pos);
    };
    let no_slimes = |_: BiomeId| 0.0;
    let any_chunk = |_: ChunkPos| true;
    let no_structures = |_: BlockPos| Vec::new();
    let context = SpawnContext {
        players: &[],
        respawn: None,
        difficulty: 0,
        overworld_time: 0,
        moon_brightness: 0.0,
        raining: false,
        thundering: false,
        seed: 0,
        surface_slime_chance: &no_slimes,
        halloween: false,
        can_spawn_in_chunk: &any_chunk,
        obstacles: Vec::new(),
        chickens: Vec::new(),
        structures: &no_structures,
        spawning_monsters: false,
    };
    rule(spawns, level, &context, kind, info, pos, false)
}

pub(super) fn walk_over(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    hosts: &[&str],
    pos: BlockPos,
) -> bool {
    let blocks = &spawns.registries.blocks;
    let below = blocks.block_of(level.block(pos.below()));
    hosts
        .iter()
        .any(|host| blocks.block_by_name(host) == Some(below))
        || -spawns.pathfinding_cost_from_light(level, pos) >= 0.0
}

pub(super) fn finalize_creature(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    row: &FamilySpawn,
    tag: &mut Tag,
    pos: BlockPos,
    group: &mut Group,
) {
    let mut data = match group {
        Group::Family { data, .. } => Some(*data),
        _ => None,
    };
    if let Some(finalize) = row.finalize {
        finalize(spawns, level, None, tag, pos, &mut data);
    }
    if let Some(data) = data {
        let size = match group {
            Group::Ageable { size, .. } | Group::Family { size, .. } => *size,
            _ => 0,
        };
        *group = Group::Family { data, size };
    }
    if let Some(chance) = row.baby_chance {
        ageable(level, tag, group, chance);
    }
    finalize_mob(tag, &mut RandomRef(level.random()));
}

pub(super) fn finalize_monster(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    context: (&SpawnContext, DifficultyInstance),
    row: &FamilySpawn,
    tag: &mut Tag,
    pos: BlockPos,
    group: &mut MonsterGroup,
) {
    let mut data = match group {
        MonsterGroup::Family { data, .. } => Some(*data),
        _ => None,
    };
    if let Some(finalize) = row.finalize {
        finalize(spawns, level, Some(context), tag, pos, &mut data);
    }
    if let Some(data) = data {
        let size = match group {
            MonsterGroup::Ageable { size, .. } | MonsterGroup::Family { size, .. } => *size,
            _ => 0,
        };
        *group = MonsterGroup::Family { data, size };
    }
    if let Some(chance) = row.baby_chance {
        if matches!(group, MonsterGroup::None) {
            *group = MonsterGroup::Ageable {
                should_spawn_baby: true,
                chance,
                size: 0,
            };
        }
        if let MonsterGroup::Ageable { size, .. } | MonsterGroup::Family { size, .. } = group {
            if *size > 0 && level.random().next_f32() <= chance {
                set(tag, "Age", Tag::Int(-24000));
            }
            *size += 1;
        }
    }
    finalize_mob(tag, &mut RandomRef(level.random()));
}
