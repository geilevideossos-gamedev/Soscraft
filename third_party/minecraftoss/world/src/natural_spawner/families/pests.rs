use super::FamilySpawn;
use crate::natural_spawner::spawning::{MobCategory, SpawnContext};
use crate::natural_spawner::{CreatureSpawns, Placement, Rules, SpawnLevel, TypeInfo, Walk};
use minecraftoss_core::BlockPos;
use minecraftoss_core::chunk::HeightmapKind;

const PEST: TypeInfo = TypeInfo {
    width: 0.4,
    height: 0.3,
    placement: Placement::OnGround,
    heightmap: HeightmapKind::MotionBlockingNoLeaves,
    fire_immune: false,
    rules: Rules::Family { creature: false },
    walk: Walk::Monster,
    remove_far: true,
    cluster: 4,
};

pub(super) const HOSTS: [&str; 7] = [
    "minecraft:stone",
    "minecraft:cobblestone",
    "minecraft:stone_bricks",
    "minecraft:mossy_stone_bricks",
    "minecraft:cracked_stone_bricks",
    "minecraft:chiseled_stone_bricks",
    "minecraft:deepslate",
];

pub(super) const ROWS: &[FamilySpawn] = &[
    FamilySpawn {
        kind: "minecraft:silverfish",
        info: TypeInfo {
            walk: Walk::MonsterOver(&HOSTS),
            ..PEST
        },
        category: MobCategory::Monster,
        follow_range: 16.0,
        baby_chance: None,
        rule: Some(any_light_away_from_players),
        finalize: None,
    },
    FamilySpawn {
        kind: "minecraft:endermite",
        info: PEST,
        category: MobCategory::Monster,
        follow_range: 16.0,
        baby_chance: None,
        rule: Some(any_light_away_from_players),
        finalize: None,
    },
];

fn any_light_away_from_players(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    context: &SpawnContext,
    kind: &str,
    info: &TypeInfo,
    pos: BlockPos,
    spawner: bool,
) -> bool {
    if spawner {
        return true;
    }
    if !spawns.mob_spawn_rules(level, kind, info, pos) {
        return false;
    }
    let centre = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    ];
    !context
        .players
        .iter()
        .any(|player| (0..3).map(|k| (player[k] - centre[k]).powi(2)).sum::<f64>() < 25.0)
}
