use super::FamilySpawn;
use crate::natural_spawner::spawning::{DifficultyInstance, MobCategory, SpawnContext};
use crate::natural_spawner::{CreatureSpawns, Placement, Rules, SpawnLevel, TypeInfo, Walk, set};
use minecraftoss_core::BlockPos;
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::nbt::Tag;
use std::collections::BTreeMap;

const DIG_COOLDOWN: &str = "minecraft:dig_cooldown";
const DIG_COOLDOWN_TICKS: i64 = 1200;

pub(super) const ROWS: &[FamilySpawn] = &[FamilySpawn {
    kind: "minecraft:warden",
    info: TypeInfo {
        width: 0.9,
        height: 2.9,
        placement: Placement::NoRestrictions,
        heightmap: HeightmapKind::MotionBlockingNoLeaves,
        fire_immune: true,
        rules: Rules::Family { creature: false },
        walk: Walk::Neutral,
        remove_far: false,
        cluster: 4,
    },
    category: MobCategory::Monster,
    follow_range: 24.0,
    baby_chance: None,
    rule: Some(monster_rule),
    finalize: Some(warden_finalize),
}];

fn monster_rule(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    context: &SpawnContext,
    kind: &str,
    info: &TypeInfo,
    pos: BlockPos,
    spawner: bool,
) -> bool {
    context.difficulty != 0
        && spawns.dark_enough(level, context, pos)
        && (spawner || spawns.mob_spawn_rules(level, kind, info, pos))
}

fn warden_finalize(
    _spawns: &CreatureSpawns,
    _level: &mut dyn SpawnLevel,
    _context: Option<(&SpawnContext, DifficultyInstance)>,
    tag: &mut Tag,
    _pos: BlockPos,
    _group: &mut Option<i32>,
) {
    let memory = [
        ("value".to_owned(), Tag::Compound(BTreeMap::new())),
        ("ttl".to_owned(), Tag::Long(DIG_COOLDOWN_TICKS)),
    ];
    let memories = [(DIG_COOLDOWN.to_owned(), Tag::Compound(memory.into()))];
    let brain = [("memories".to_owned(), Tag::Compound(memories.into()))];
    set(tag, "Brain", Tag::Compound(brain.into()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::natural_spawner::StructureHere;
    use crate::natural_spawner::type_info;
    use minecraftoss_core::random::{LegacyRandom, RandomSource};
    use minecraftoss_core::{BiomeId, BlockStateId, ChunkPos};

    struct Cave {
        floor: BlockStateId,
        sky: i32,
        biome: BiomeId,
        random: LegacyRandom,
    }

    impl SpawnLevel for Cave {
        fn block(&self, pos: BlockPos) -> BlockStateId {
            if pos.y <= 63 {
                self.floor
            } else {
                BlockStateId::AIR
            }
        }
        fn height(&self, _kind: HeightmapKind, _x: i32, _z: i32) -> i32 {
            64
        }
        fn biome(&self, _pos: BlockPos) -> BiomeId {
            self.biome
        }
        fn noise_biome(&self, _qx: i32, _qy: i32, _qz: i32) -> BiomeId {
            self.biome
        }
        fn raw_brightness(&mut self, pos: BlockPos, darkening: i32) -> i32 {
            (self.sky_brightness(pos) - darkening).max(0)
        }
        fn sky_brightness(&mut self, pos: BlockPos) -> i32 {
            if pos.y <= 63 { 0 } else { self.sky }
        }
        fn block_brightness(&mut self, _pos: BlockPos) -> i32 {
            0
        }
        fn min_y(&self) -> i32 {
            -64
        }
        fn random(&mut self) -> &mut dyn RandomSource {
            &mut self.random
        }
        fn next_uuid(&mut self) -> [i32; 4] {
            [1, 2, 3, 4]
        }
        fn add_entity(&mut self, _entity: Tag) {}
        fn note_unsupported(&mut self, _what: &str) {}
    }

    fn no_slimes(_: BiomeId) -> f32 {
        0.0
    }

    fn any_chunk(_: ChunkPos) -> bool {
        true
    }

    fn no_structures(_: BlockPos) -> Vec<StructureHere> {
        Vec::new()
    }

    fn context(difficulty: i32) -> SpawnContext<'static> {
        SpawnContext {
            players: &[],
            respawn: None,
            difficulty,
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
            spawning_monsters: true,
        }
    }

    #[test]
    fn a_summoned_warden_waits_to_dig_and_a_triggered_one_needs_the_dark() {
        let info = type_info("minecraft:warden").unwrap();
        assert_eq!(
            (info.width, info.height, info.placement, info.fire_immune),
            (0.9, 2.9, Placement::NoRestrictions, true)
        );
        assert_eq!(
            MobCategory::of_type("minecraft:warden"),
            Some(MobCategory::Monster)
        );
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let biome = registries.biomes.id("minecraft:deep_dark").unwrap();
        let floor = registries
            .blocks
            .parse_state("minecraft:deepslate")
            .unwrap();
        let registries = std::sync::Arc::new(registries);
        let spawns = CreatureSpawns::load(registries, "minecraft:overworld", false).unwrap();
        let mut cave = Cave {
            floor,
            sky: 0,
            biome,
            random: LegacyRandom::new(3),
        };
        let mut summon = context(2);
        let tag = spawns
            .summon(
                &mut cave,
                &mut summon,
                "minecraft:warden",
                [0.5, 64.0, 0.5],
                None,
                0.0,
            )
            .unwrap();
        let memory = tag
            .get("Brain")
            .and_then(|brain| brain.get("memories"))
            .and_then(|memories| memories.get(DIG_COOLDOWN));
        assert_eq!(
            memory.and_then(|m| m.get("ttl")).and_then(Tag::as_i64),
            Some(1200)
        );
        let at = [4.5, 64.0, 4.5];
        assert!(spawns.triggered_may_spawn(&mut cave, &context(2), "minecraft:warden", at));
        assert!(
            !spawns.triggered_may_spawn(&mut cave, &context(0), "minecraft:warden", at),
            "never on peaceful"
        );
        assert!(
            !spawns.triggered_may_spawn(
                &mut cave,
                &context(2),
                "minecraft:warden",
                [4.5, 63.0, 4.5]
            ),
            "no room inside the floor"
        );
        cave.sky = 15;
        assert!(
            (0..20).all(|_| !spawns.triggered_may_spawn(
                &mut cave,
                &context(2),
                "minecraft:warden",
                at
            )),
            "never in daylight"
        );
    }
}
