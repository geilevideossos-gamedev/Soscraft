use super::FamilySpawn;
use crate::natural_spawner::spawning::{MobCategory, SpawnContext};
use crate::natural_spawner::{CreatureSpawns, Placement, Rules, SpawnLevel, TypeInfo, Walk};
use minecraftoss_core::BlockPos;
use minecraftoss_core::chunk::HeightmapKind;

pub(super) const ROWS: &[FamilySpawn] = &[FamilySpawn {
    kind: "minecraft:breeze",
    info: TypeInfo {
        width: 0.6,
        height: 1.77,
        placement: Placement::OnGround,
        heightmap: HeightmapKind::MotionBlockingNoLeaves,
        fire_immune: false,
        rules: Rules::Family { creature: false },
        walk: Walk::Monster,
        remove_far: true,
        cluster: 4,
    },
    category: MobCategory::Monster,
    follow_range: 24.0,
    baby_chance: None,
    rule: Some(any_light),
    finalize: None,
}];

fn any_light(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    context: &SpawnContext,
    kind: &str,
    info: &TypeInfo,
    pos: BlockPos,
    spawner: bool,
) -> bool {
    context.difficulty != 0 && (spawner || spawns.mob_spawn_rules(level, kind, info, pos))
}
