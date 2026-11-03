use super::FamilySpawn;
use crate::natural_spawner::spawning::{MobCategory, SpawnContext};
use crate::natural_spawner::{CreatureSpawns, Placement, Rules, SpawnLevel, TypeInfo, Walk};
use minecraftoss_core::BlockPos;
use minecraftoss_core::block::FluidKind;
use minecraftoss_core::chunk::HeightmapKind;

const SQUID: TypeInfo = TypeInfo {
    width: 0.8,
    height: 0.8,
    placement: Placement::InWater,
    heightmap: HeightmapKind::MotionBlockingNoLeaves,
    fire_immune: false,
    rules: Rules::Family { creature: true },
    walk: Walk::Neutral,
    remove_far: true,
    cluster: 4,
};

pub(super) const ROWS: &[FamilySpawn] = &[
    FamilySpawn {
        kind: "minecraft:squid",
        info: SQUID,
        category: MobCategory::WaterCreature,
        follow_range: 16.0,
        baby_chance: Some(0.05),
        rule: Some(surface_water_animal),
        finalize: None,
    },
    FamilySpawn {
        kind: "minecraft:glow_squid",
        info: SQUID,
        category: MobCategory::UndergroundWaterCreature,
        follow_range: 16.0,
        baby_chance: Some(0.05),
        rule: Some(dark_deep_water),
        finalize: None,
    },
];

fn is_water_block(spawns: &CreatureSpawns, level: &dyn SpawnLevel, pos: BlockPos) -> bool {
    let blocks = &spawns.registries.blocks;
    let block = blocks.block(blocks.block_of(level.block(pos)));
    block.name.as_str() == "minecraft:water"
}

fn surface_water_animal(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    _context: &SpawnContext,
    _kind: &str,
    _info: &TypeInfo,
    pos: BlockPos,
    _spawner: bool,
) -> bool {
    let sea = level.sea_level();
    if pos.y < sea - 13 || pos.y > sea {
        return false;
    }
    let below = spawns.registries.blocks.state(level.block(pos.below()));
    let water_below = below
        .fluid
        .is_some_and(|fluid| fluid.kind == FluidKind::Water);
    water_below && is_water_block(spawns, level, pos.above())
}

fn dark_deep_water(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    _context: &SpawnContext,
    _kind: &str,
    _info: &TypeInfo,
    pos: BlockPos,
    _spawner: bool,
) -> bool {
    pos.y <= level.sea_level() - 33
        && level.raw_brightness(pos, 0) == 0
        && is_water_block(spawns, level, pos)
}

#[cfg(test)]
mod tests;
