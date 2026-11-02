use super::FamilySpawn;
use crate::natural_spawner::spawning::{DifficultyInstance, MobCategory, SpawnContext};
use crate::natural_spawner::{CreatureSpawns, Placement, Rules, SpawnLevel, TypeInfo, Walk, set};
use minecraftoss_core::BlockPos;
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::nbt::Tag;

pub(super) const PHANTOM: &str = "minecraft:phantom";
pub(super) const ANCHOR_UP: i32 = 5;

pub(super) const ROWS: &[FamilySpawn] = &[FamilySpawn {
    kind: PHANTOM,
    info: TypeInfo {
        width: 0.9,
        height: 0.5,
        placement: Placement::NoRestrictions,
        heightmap: HeightmapKind::MotionBlockingNoLeaves,
        fire_immune: false,
        rules: Rules::Family { creature: false },
        walk: Walk::Neutral,
        remove_far: true,
        cluster: 4,
    },
    category: MobCategory::Monster,
    follow_range: 16.0,
    baby_chance: None,
    rule: Some(phantom_rule),
    finalize: Some(phantom_finalize),
}];

fn phantom_rule(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    _context: &SpawnContext,
    kind: &str,
    info: &TypeInfo,
    pos: BlockPos,
    spawner: bool,
) -> bool {
    spawner || spawns.mob_spawn_rules(level, kind, info, pos)
}

fn phantom_finalize(
    _spawns: &CreatureSpawns,
    _level: &mut dyn SpawnLevel,
    _context: Option<(&SpawnContext, DifficultyInstance)>,
    tag: &mut Tag,
    pos: BlockPos,
    _data: &mut Option<i32>,
) {
    let anchor = vec![pos.x, pos.y + ANCHOR_UP, pos.z];
    set(tag, "anchor_pos", Tag::IntArray(anchor));
    set(tag, "size", Tag::Int(0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::natural_spawner::{StructureHere, type_info};
    use minecraftoss_core::random::{LegacyRandom, RandomSource};
    use minecraftoss_core::{BiomeId, BlockStateId, ChunkPos};

    struct Field {
        floor: BlockStateId,
        biome: BiomeId,
        random: LegacyRandom,
        made: i32,
        notes: Vec<String>,
    }

    impl SpawnLevel for Field {
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
            if pos.y <= 63 { 0 } else { 15 }
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
            self.made += 1;
            [self.made, 0, 0, 1]
        }
        fn add_entity(&mut self, _entity: Tag) {}
        fn note_unsupported(&mut self, what: &str) {
            self.notes.push(what.to_owned());
        }
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

    fn context(players: &[[f64; 3]]) -> SpawnContext<'_> {
        SpawnContext {
            players,
            respawn: None,
            difficulty: 2,
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

    fn setup(floor: &str) -> Option<(CreatureSpawns, Field)> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        registries.entities.as_ref()?;
        let biome = registries.biomes.id("minecraft:plains")?;
        let floor = registries.blocks.parse_state(floor).ok()?;
        let registries = std::sync::Arc::new(registries);
        let spawns = CreatureSpawns::load(registries, "minecraft:overworld", false).ok()?;
        let field = Field {
            floor,
            biome,
            random: LegacyRandom::new(9),
            made: 0,
            notes: Vec::new(),
        };
        Some((spawns, field))
    }

    fn int(tag: &Tag, key: &str) -> Option<i64> {
        tag.get(key).and_then(Tag::as_i64)
    }

    #[test]
    fn a_phantom_row_spawns_from_spawners_and_finalizes_its_anchor_five_up() {
        let info = type_info(PHANTOM).unwrap();
        assert_eq!(
            (info.width, info.height, info.placement, info.fire_immune),
            (0.9, 0.5, Placement::NoRestrictions, false)
        );
        assert!(matches!(
            info.heightmap,
            HeightmapKind::MotionBlockingNoLeaves
        ));
        assert_eq!(
            (info.rules, info.walk, info.remove_far, info.cluster),
            (Rules::Family { creature: false }, Walk::Neutral, true, 4)
        );
        assert_eq!(MobCategory::of_type(PHANTOM), Some(MobCategory::Monster));
        let row = super::super::row(PHANTOM).unwrap();
        assert_eq!((row.follow_range, row.baby_chance), (16.0, None));
        let Some((spawns, mut level)) = setup("minecraft:grass_block") else {
            return;
        };
        let pos = BlockPos::new(0, 64, 0);
        let players: [[f64; 3]; 0] = [];
        let rule = |spawns: &CreatureSpawns, level: &mut Field, spawner: bool| {
            let context = context(&players);
            super::super::rule(spawns, level, &context, PHANTOM, &info, pos, spawner)
        };
        assert!(rule(&spawns, &mut level, false), "on grass");
        assert!(rule(&spawns, &mut level, true), "from a spawner");
        let (glass_spawns, mut glass) = setup("minecraft:glass").unwrap();
        assert!(!rule(&glass_spawns, &mut glass, false), "not on glass");
        assert!(rule(&glass_spawns, &mut glass, true), "a spawner anywhere");
        let at = [3.5, 70.0, -2.5];
        let tag = spawns
            .summon(&mut level, &mut context(&players), PHANTOM, at, None, 0.0)
            .unwrap();
        assert_eq!(
            tag.get("anchor_pos").and_then(Tag::as_ints),
            Some(vec![3, 75, -3])
        );
        assert_eq!(int(&tag, "size"), Some(0));
        assert!(level.notes.is_empty(), "{:?}", level.notes);
        let sized = minecraftoss_core::snbt::parse_compound("{size:6}").unwrap();
        let tag = spawns
            .summon(
                &mut level,
                &mut context(&players),
                PHANTOM,
                at,
                Some(&sized),
                0.0,
            )
            .unwrap();
        assert_eq!(int(&tag, "size"), Some(6));
        assert_eq!(
            tag.get("anchor_pos"),
            None,
            "a summon with a tag is not finalized"
        );
    }
}
