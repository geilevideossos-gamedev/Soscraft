use super::FamilySpawn;

pub(super) const ROWS: &[FamilySpawn] = &[];

#[cfg(test)]
mod tests {
    use crate::natural_spawner::spawning::{MobCategory, SpawnContext, Unlimited};
    use crate::natural_spawner::{
        CreatureSpawns, Creatures, Placement, SpawnLevel, SpawnerData, StructureHere, type_info,
    };
    use minecraftoss_core::chunk::HeightmapKind;
    use minecraftoss_core::nbt::Tag;
    use minecraftoss_core::random::{LegacyRandom, RandomSource};
    use minecraftoss_core::{BiomeId, BlockPos, BlockStateId, ChunkPos};

    const RABBIT: &str = "minecraft:rabbit";

    struct Field {
        floor: BlockStateId,
        sky: i32,
        biome: BiomeId,
        random: LegacyRandom,
        made: i32,
        added: Vec<Tag>,
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
            self.made += 1;
            [self.made, 0, 0, 1]
        }
        fn add_entity(&mut self, entity: Tag) {
            self.added.push(entity);
        }
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

    fn rabbits(biome: &str, floor: &str, sky: i32) -> Option<(Vec<Vec<Tag>>, Vec<String>)> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        registries.entities.as_ref()?;
        let biome = registries.biomes.id(biome)?;
        let floor = registries.blocks.parse_state(floor).ok()?;
        let registries = std::sync::Arc::new(registries);
        let mut spawns = CreatureSpawns::load(registries, "minecraft:overworld", false).ok()?;
        spawns.by_category[usize::from(biome.0)][MobCategory::Creature as usize] = Creatures {
            entries: vec![SpawnerData {
                kind: RABBIT.to_owned(),
                weight: 1,
                min: 4,
                max: 4,
                constant: true,
            }],
            total: 1,
        };
        let mut level = Field {
            floor,
            sky,
            biome,
            random: LegacyRandom::new(9),
            made: 0,
            added: Vec::new(),
            notes: Vec::new(),
        };
        let players = [[8.5, 64.0, 40.5]];
        let mut calls = Vec::new();
        for _ in 0..40 {
            let before = level.added.len();
            spawns.spawn_category_for_position(
                &mut level,
                &mut context(&players),
                &mut Unlimited,
                MobCategory::Creature,
                ChunkPos::new(0, 0),
                BlockPos::new(8, 64, 8),
            );
            calls.push(level.added[before..].to_vec());
        }
        Some((calls, level.notes))
    }

    fn int(tag: &Tag, key: &str) -> Option<i64> {
        tag.get(key).and_then(Tag::as_i64)
    }

    #[test]
    fn rabbits_spawn_on_grass_sand_and_snow_in_daylight_as_one_adult_and_babies_of_its_biome_colour()
     {
        let info = type_info(RABBIT).unwrap();
        assert_eq!(
            (info.width, info.height, info.placement, info.fire_immune),
            (0.49, 0.6, Placement::OnGround, false)
        );
        assert!(matches!(
            info.heightmap,
            HeightmapKind::MotionBlockingNoLeaves
        ));
        assert!(info.rules.creature());
        assert_eq!(MobCategory::of_type(RABBIT), Some(MobCategory::Creature));
        assert!(
            super::super::row(RABBIT).is_none(),
            "the built-in row spawns it"
        );
        for (biome, floor, colours) in [
            (
                "minecraft:snowy_plains",
                "minecraft:snow_block",
                &[1, 3][..],
            ),
            ("minecraft:desert", "minecraft:sand", &[4][..]),
            ("minecraft:plains", "minecraft:grass_block", &[0, 2, 5][..]),
        ] {
            let Some((calls, notes)) = rabbits(biome, floor, 15) else {
                return;
            };
            assert!(notes.is_empty(), "{biome}: {notes:?}");
            let mut groups = 0;
            for call in calls.iter().filter(|call| !call.is_empty()) {
                assert_ne!(
                    int(&call[0], "Age"),
                    Some(-24000),
                    "{biome}: an adult first"
                );
                let mut colour = None;
                for tag in call {
                    assert_eq!(tag.get("id").and_then(Tag::as_str), Some(RABBIT));
                    let kind = int(tag, "RabbitType").unwrap();
                    assert!(colours.contains(&kind), "{biome}: {kind}");
                    if int(tag, "Age") == Some(-24000) {
                        assert_eq!(Some(kind), colour, "{biome}: a baby of its adult's colour");
                    } else {
                        groups += 1;
                        colour = Some(kind);
                    }
                }
            }
            assert!(groups > 5, "{biome}: {groups}");
            for (floor, sky, why) in [("minecraft:stone", 15, "stone"), (floor, 8, "the dark")] {
                let (calls, _) = rabbits(biome, floor, sky).unwrap();
                assert!(calls.concat().is_empty(), "{biome}: none on {why}");
            }
        }
    }
}
