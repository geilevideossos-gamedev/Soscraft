use super::FamilySpawn;
use crate::natural_spawner::spawning::{DifficultyInstance, MobCategory, SpawnContext};
use crate::natural_spawner::{
    CreatureSpawns, Placement, Rules, SpawnLevel, TypeInfo, Walk, set, set_attribute_base,
};
use minecraftoss_core::BlockPos;
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::RandomSource;

const POLAR_BEAR_BIOMES: &str = "minecraft:polar_bears_spawn_on_alternate_blocks";
const POLAR_BEAR_BLOCKS: &str = "minecraft:polar_bears_spawnable_on_alternate";
const PANDA_GENES: [&str; 16] = [
    "lazy",
    "worried",
    "playful",
    "weak",
    "aggressive",
    "weak",
    "weak",
    "weak",
    "weak",
    "brown",
    "brown",
    "normal",
    "normal",
    "normal",
    "normal",
    "normal",
];
const PANDA_WEAK_HEALTH: f64 = 10.0;
const PANDA_LAZY_SPEED: f64 = 0.07000000029802322;

pub(super) const ROWS: &[FamilySpawn] = &[PANDA, POLAR_BEAR];

const PANDA: FamilySpawn = FamilySpawn {
    kind: "minecraft:panda",
    info: TypeInfo {
        width: 1.3,
        height: 1.25,
        placement: Placement::NoRestrictions,
        heightmap: HeightmapKind::MotionBlockingNoLeaves,
        fire_immune: false,
        rules: Rules::Animal,
        walk: Walk::Animal,
        remove_far: false,
        cluster: 4,
    },
    category: MobCategory::Creature,
    follow_range: 16.0,
    baby_chance: Some(0.2),
    rule: None,
    finalize: Some(panda_finalize),
};

const POLAR_BEAR: FamilySpawn = FamilySpawn {
    kind: "minecraft:polar_bear",
    info: TypeInfo {
        width: 1.4,
        height: 1.4,
        placement: Placement::OnGround,
        heightmap: HeightmapKind::MotionBlockingNoLeaves,
        fire_immune: false,
        rules: Rules::Family { creature: true },
        walk: Walk::Animal,
        remove_far: false,
        cluster: 4,
    },
    category: MobCategory::Creature,
    follow_range: 20.0,
    baby_chance: Some(1.0),
    rule: Some(polar_bear_rule),
    finalize: None,
};

fn panda_gene(random: &mut dyn RandomSource) -> &'static str {
    PANDA_GENES[random.next_i32_bound(16) as usize]
}

fn finalize_panda(tag: &mut Tag, random: &mut dyn RandomSource) {
    let main = panda_gene(random);
    let hidden = panda_gene(random);
    set(tag, "MainGene", Tag::String(main.to_owned()));
    set(tag, "HiddenGene", Tag::String(hidden.to_owned()));
    let recessive = matches!(main, "brown" | "weak");
    let variant = if recessive && main != hidden {
        "normal"
    } else {
        main
    };
    match variant {
        "weak" => {
            set_attribute_base(tag, "minecraft:max_health", PANDA_WEAK_HEALTH);
            set(tag, "Health", Tag::Float(PANDA_WEAK_HEALTH as f32));
        }
        "lazy" => set_attribute_base(tag, "minecraft:movement_speed", PANDA_LAZY_SPEED),
        _ => {}
    }
}

fn panda_finalize(
    _spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    _context: Option<(&SpawnContext, DifficultyInstance)>,
    tag: &mut Tag,
    _pos: BlockPos,
    _data: &mut Option<i32>,
) {
    finalize_panda(tag, level.random());
}

fn polar_bear_rule(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    _context: &SpawnContext,
    _kind: &str,
    _info: &TypeInfo,
    pos: BlockPos,
    spawner: bool,
) -> bool {
    let registries = &spawns.registries;
    let biome = level.biome(pos);
    let alternate = registries
        .biome_tags
        .id(POLAR_BEAR_BIOMES)
        .is_some_and(|tag| registries.biome_in_tag(biome, tag));
    if alternate {
        let bright = level.raw_brightness(pos, 0) > 8;
        let below = level.block(pos.below());
        return bright
            && registries
                .block_tags
                .id(POLAR_BEAR_BLOCKS)
                .is_some_and(|tag| registries.block_in_tag(below, tag));
    }
    let bright = spawner || level.raw_brightness(pos, 0) > 8;
    let below = level.block(pos.below());
    registries.block_in_tag(below, spawns.animals_spawnable_on) && bright
}

#[cfg(test)]
mod tests {
    use crate::natural_spawner::spawning::{MobCategory, SpawnContext, Unlimited};
    use crate::natural_spawner::{
        CreatureSpawns, Creatures, Placement, Rules, SpawnLevel, SpawnerData, StructureHere, Walk,
        type_info,
    };
    use minecraftoss_core::chunk::HeightmapKind;
    use minecraftoss_core::nbt::Tag;
    use minecraftoss_core::random::{LegacyRandom, RandomSource};
    use minecraftoss_core::{BiomeId, BlockPos, BlockStateId, ChunkPos};

    const RABBIT: &str = "minecraft:rabbit";
    const POLAR_BEAR: &str = "minecraft:polar_bear";
    const PANDA: &str = "minecraft:panda";

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

    fn pandas() -> Option<(Vec<Vec<Tag>>, Vec<String>)> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        registries.entities.as_ref()?;
        let biome = registries.biomes.id("minecraft:bamboo_jungle")?;
        let floor = registries
            .blocks
            .parse_state("minecraft:grass_block")
            .ok()?;
        let registries = std::sync::Arc::new(registries);
        let mut spawns = CreatureSpawns::load(registries, "minecraft:overworld", false).ok()?;
        spawns.by_category[usize::from(biome.0)][MobCategory::Creature as usize] = Creatures {
            entries: vec![SpawnerData {
                kind: PANDA.to_owned(),
                weight: 1,
                min: 4,
                max: 4,
                constant: true,
            }],
            total: 1,
        };
        let mut level = Field {
            floor,
            sky: 15,
            biome,
            random: LegacyRandom::new(9),
            made: 0,
            added: Vec::new(),
            notes: Vec::new(),
        };
        let players = [[8.5, 64.0, 40.5]];
        let mut calls = Vec::new();
        for _ in 0..80 {
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
        let summoned = spawns
            .summon(
                &mut level,
                &mut context(&players),
                PANDA,
                [0.5, 64.0, 0.5],
                None,
                0.0,
            )
            .ok()?;
        calls.push(vec![summoned]);
        Some((calls, level.notes))
    }

    fn gene(tag: &Tag, key: &str) -> String {
        tag.get(key).and_then(Tag::as_str).unwrap_or("").to_owned()
    }

    fn attribute_base(tag: &Tag, id: &str) -> Option<f64> {
        let list = tag.get("attributes")?.as_list()?;
        let entry = list
            .iter()
            .find(|a| a.get("id").and_then(Tag::as_str) == Some(id))?;
        entry.get("base")?.as_f64()
    }

    #[test]
    fn a_panda_spawns_with_two_genes_and_later_group_members_are_cubs_one_time_in_five() {
        let info = type_info(PANDA).unwrap();
        assert_eq!(
            (info.width, info.height, info.placement, info.fire_immune),
            (1.3, 1.25, Placement::NoRestrictions, false)
        );
        assert!(matches!(
            info.heightmap,
            HeightmapKind::MotionBlockingNoLeaves
        ));
        assert_eq!(
            (info.rules, info.walk, info.remove_far, info.cluster),
            (Rules::Animal, Walk::Animal, false, 4)
        );
        assert_eq!(MobCategory::of_type(PANDA), Some(MobCategory::Creature));
        let row = super::super::row(PANDA).unwrap();
        assert_eq!((row.follow_range, row.baby_chance), (16.0, Some(0.2)));
        assert!(row.rule.is_none() && row.finalize.is_some());
        let mut counts = std::collections::HashMap::new();
        let mut random = LegacyRandom::new(3200);
        for _ in 0..3200 {
            let mut tag = Tag::Compound(Default::default());
            super::finalize_panda(&mut tag, &mut random);
            let (main, hidden) = (gene(&tag, "MainGene"), gene(&tag, "HiddenGene"));
            *counts.entry(main.clone()).or_insert(0) += 1;
            let recessive = main == "brown" || main == "weak";
            let shown = if recessive && main != hidden {
                "normal"
            } else {
                main.as_str()
            };
            let health = tag.get("Health").and_then(Tag::as_f64);
            let max_health = attribute_base(&tag, "minecraft:max_health");
            let speed = attribute_base(&tag, "minecraft:movement_speed");
            match shown {
                "weak" => assert_eq!((max_health, health, speed), (Some(10.0), Some(10.0), None)),
                "lazy" => assert_eq!(
                    (max_health, health, speed),
                    (None, None, Some(0.07000000029802322))
                ),
                _ => assert_eq!(tag.get("attributes"), None, "{main} {hidden}"),
            }
        }
        for (gene, slots) in [
            ("lazy", 1),
            ("worried", 1),
            ("playful", 1),
            ("aggressive", 1),
            ("weak", 5),
            ("brown", 2),
            ("normal", 5),
        ] {
            let expected = 3200 * slots / 16;
            let found = counts[gene];
            assert!(
                (expected * 7 / 10..=expected * 13 / 10).contains(&found),
                "{gene}: {found} of about {expected}"
            );
        }
        let Some((calls, notes)) = pandas() else {
            return;
        };
        assert!(notes.is_empty(), "{notes:?}");
        let (mut groups, mut later, mut cubs) = (0, 0, 0);
        for call in calls.iter().filter(|call| !call.is_empty()) {
            groups += 1;
            assert_ne!(int(&call[0], "Age"), Some(-24000), "an adult first");
            for tag in call {
                assert_eq!(tag.get("id").and_then(Tag::as_str), Some(PANDA));
                for key in ["MainGene", "HiddenGene"] {
                    assert!(super::PANDA_GENES.contains(&gene(tag, key).as_str()));
                }
            }
            for tag in &call[1..] {
                later += 1;
                if int(tag, "Age") == Some(-24000) {
                    cubs += 1;
                }
            }
        }
        assert!(groups > 10 && later > 60, "{groups} groups, {later} later");
        assert!(
            (later / 10..=later * 3 / 10).contains(&cubs),
            "about one in five later pandas is a cub: {cubs} of {later}"
        );
    }

    fn polar_bears(
        biome: &str,
        floor: &str,
        sky: i32,
        generation: bool,
    ) -> Option<(Vec<Vec<Tag>>, Vec<String>)> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        registries.entities.as_ref()?;
        let biome = registries.biomes.id(biome)?;
        let floor = registries.blocks.parse_state(floor).ok()?;
        let registries = std::sync::Arc::new(registries);
        let mut spawns = CreatureSpawns::load(registries, "minecraft:overworld", false).ok()?;
        let bears = || Creatures {
            entries: vec![SpawnerData {
                kind: POLAR_BEAR.to_owned(),
                weight: 1,
                min: 3,
                max: 3,
                constant: true,
            }],
            total: 1,
        };
        let index = usize::from(biome.0);
        spawns.by_category[index][MobCategory::Creature as usize] = bears();
        spawns.creatures[index] = bears();
        spawns.probability[index] = 0.5;
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
        for round in 0..40_i64 {
            let before = level.added.len();
            if generation {
                let seed = round.wrapping_mul(0x2545_F491_4F6C_DD1D);
                spawns.spawn_original_mobs(&mut level, ChunkPos::new(0, 0), seed, 80);
            } else {
                spawns.spawn_category_for_position(
                    &mut level,
                    &mut context(&players),
                    &mut Unlimited,
                    MobCategory::Creature,
                    ChunkPos::new(0, 0),
                    BlockPos::new(8, 64, 8),
                );
            }
            calls.push(level.added[before..].to_vec());
        }
        Some((calls, level.notes))
    }

    fn rule_allows(biome: &str, floor: &str, sky: i32, spawner: bool) -> Option<bool> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        let biome = registries.biomes.id(biome)?;
        let floor = registries.blocks.parse_state(floor).ok()?;
        let registries = std::sync::Arc::new(registries);
        let spawns = CreatureSpawns::load(registries, "minecraft:overworld", false).ok()?;
        let mut level = Field {
            floor,
            sky,
            biome,
            random: LegacyRandom::new(9),
            made: 0,
            added: Vec::new(),
            notes: Vec::new(),
        };
        let info = type_info(POLAR_BEAR)?;
        let pos = BlockPos::new(0, 64, 0);
        let context = context(&[]);
        Some(super::super::rule(
            &spawns, &mut level, &context, POLAR_BEAR, &info, pos, spawner,
        ))
    }

    #[test]
    fn polar_bears_spawn_on_grass_in_the_light_and_on_ice_in_frozen_oceans() {
        let info = type_info(POLAR_BEAR).unwrap();
        assert_eq!(
            (info.width, info.height, info.placement, info.fire_immune),
            (1.4, 1.4, Placement::OnGround, false)
        );
        assert!(matches!(
            info.heightmap,
            HeightmapKind::MotionBlockingNoLeaves
        ));
        assert_eq!(
            (info.rules, info.walk, info.remove_far, info.cluster),
            (Rules::Family { creature: true }, Walk::Animal, false, 4)
        );
        assert_eq!(
            MobCategory::of_type(POLAR_BEAR),
            Some(MobCategory::Creature)
        );
        let row = super::super::row(POLAR_BEAR).unwrap();
        assert_eq!((row.follow_range, row.baby_chance), (20.0, Some(1.0)));
        assert!(row.rule.is_some() && row.finalize.is_none());
        let snowy = "minecraft:snowy_plains";
        let frozen = "minecraft:frozen_ocean";
        let Some(on_grass) = rule_allows(snowy, "minecraft:grass_block", 15, false) else {
            return;
        };
        assert!(on_grass, "grass in daylight");
        for (biome, floor, sky, spawner, allowed, why) in [
            (
                snowy,
                "minecraft:grass_block",
                8,
                false,
                false,
                "grass in the dark",
            ),
            (
                snowy,
                "minecraft:grass_block",
                8,
                true,
                true,
                "a spawner in the dark",
            ),
            (snowy, "minecraft:snow_block", 15, false, false, "snow"),
            (snowy, "minecraft:ice", 15, false, false, "ice on land"),
            (frozen, "minecraft:ice", 15, false, true, "sea ice"),
            (
                "minecraft:deep_frozen_ocean",
                "minecraft:ice",
                15,
                false,
                true,
                "deep sea ice",
            ),
            (
                frozen,
                "minecraft:ice",
                8,
                true,
                false,
                "sea ice in the dark",
            ),
            (
                frozen,
                "minecraft:packed_ice",
                15,
                false,
                false,
                "packed ice",
            ),
            (
                frozen,
                "minecraft:grass_block",
                15,
                false,
                false,
                "grass by the sea",
            ),
        ] {
            let found = rule_allows(biome, floor, sky, spawner).unwrap();
            assert_eq!(found, allowed, "{why}");
        }
        for generation in [false, true] {
            let (calls, notes) = polar_bears(frozen, "minecraft:ice", 15, generation).unwrap();
            assert!(notes.is_empty(), "{notes:?}");
            let spawned = calls.concat();
            assert!(
                spawned.len() > 5,
                "generation {generation}: {}",
                spawned.len()
            );
            assert!(
                spawned
                    .iter()
                    .all(|tag| tag.get("id").and_then(Tag::as_str) == Some(POLAR_BEAR))
            );
            let (calls, _) = polar_bears(frozen, "minecraft:packed_ice", 15, generation).unwrap();
            assert!(calls.concat().is_empty(), "generation {generation}");
        }
    }

    #[test]
    fn a_polar_bear_group_is_an_adult_with_cubs() {
        for generation in [false, true] {
            let found = polar_bears(
                "minecraft:snowy_plains",
                "minecraft:grass_block",
                15,
                generation,
            );
            let Some((calls, notes)) = found else {
                return;
            };
            assert!(notes.is_empty(), "{notes:?}");
            let (mut adults, mut cubs) = (0, 0);
            for call in calls.iter().filter(|call| !call.is_empty()) {
                assert_ne!(int(&call[0], "Age"), Some(-24000), "an adult first");
                for tag in call {
                    if int(tag, "Age") == Some(-24000) {
                        cubs += 1;
                    } else {
                        adults += 1;
                    }
                }
            }
            assert!(adults > 5, "generation {generation}: {adults}");
            assert!(cubs > 5, "generation {generation}: {adults} and {cubs}");
        }
        let paths = minecraftoss_core::registries::DataPaths::discover().unwrap();
        let registries = minecraftoss_core::Registries::load(&paths).unwrap();
        let biome = registries.biomes.id("minecraft:snowy_plains").unwrap();
        let floor = registries
            .blocks
            .parse_state("minecraft:grass_block")
            .unwrap();
        let spawns = CreatureSpawns::load(
            std::sync::Arc::new(registries),
            "minecraft:overworld",
            false,
        )
        .unwrap();
        let mut level = Field {
            floor,
            sky: 15,
            biome,
            random: LegacyRandom::new(9),
            made: 0,
            added: Vec::new(),
            notes: Vec::new(),
        };
        let players: [[f64; 3]; 0] = [];
        let tag = spawns
            .summon(
                &mut level,
                &mut context(&players),
                POLAR_BEAR,
                [0.5, 64.0, 0.5],
                None,
                0.0,
            )
            .unwrap();
        assert_ne!(
            int(&tag, "Age"),
            Some(-24000),
            "a summoned bear is an adult"
        );
        assert!(level.notes.is_empty(), "{:?}", level.notes);
    }
}
