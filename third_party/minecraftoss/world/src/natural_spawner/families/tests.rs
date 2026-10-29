use super::super::spawning::{
    DifficultyInstance, MobCategory, SpawnContext, SpawnerReason, Unlimited,
};
use super::super::{
    CreatureSpawns, Creatures, Group, Placement, Rules, SpawnLevel, SpawnerData, StructureHere,
    TypeInfo, Walk, set, type_info, type_width,
};
use super::{FamilySpawn, row, rows};
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::entity_data::finalize_mob;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::{LegacyRandom, RandomSource};
use minecraftoss_core::{BiomeId, BlockPos, BlockStateId, ChunkPos};

const PROBE: TypeInfo = TypeInfo {
    width: 0.5,
    height: 0.5,
    placement: Placement::OnGround,
    heightmap: HeightmapKind::MotionBlockingNoLeaves,
    fire_immune: false,
    rules: Rules::Family { creature: true },
    walk: Walk::Neutral,
    remove_far: false,
    cluster: 8,
};

pub(super) const ROWS: &[FamilySpawn] = &[
    FamilySpawn {
        kind: "minecraft:mooshroom",
        info: PROBE,
        category: MobCategory::Creature,
        follow_range: 16.0,
        baby_chance: Some(1.0),
        rule: None,
        finalize: Some(numbered),
    },
    FamilySpawn {
        kind: "minecraft:giant",
        info: TypeInfo {
            rules: Rules::Family { creature: false },
            ..PROBE
        },
        category: MobCategory::Monster,
        follow_range: 30.0,
        baby_chance: Some(1.0),
        rule: None,
        finalize: Some(numbered),
    },
    FamilySpawn {
        kind: "minecraft:trader_llama",
        info: PROBE,
        category: MobCategory::Creature,
        follow_range: 16.0,
        baby_chance: None,
        rule: None,
        finalize: None,
    },
];

fn numbered(
    _: &CreatureSpawns,
    _: &mut dyn SpawnLevel,
    context: Option<(&SpawnContext, DifficultyInstance)>,
    tag: &mut Tag,
    _: BlockPos,
    group: &mut Option<i32>,
) {
    let member = group.map_or(1, |n| n + 1);
    *group = Some(member);
    set(tag, "ProbeMember", Tag::Int(member));
    let difficulty = context.map_or(-1, |(context, difficulty)| {
        assert_eq!(context.difficulty, difficulty.base);
        difficulty.base
    });
    set(tag, "ProbeDifficulty", Tag::Int(difficulty));
}

struct Flat {
    floor_top: i32,
    floor: BlockStateId,
    sky: i32,
    biome: BiomeId,
    random: LegacyRandom,
    made: i32,
    added: Vec<Tag>,
    notes: Vec<String>,
}

impl SpawnLevel for Flat {
    fn block(&self, pos: BlockPos) -> BlockStateId {
        if pos.y <= self.floor_top {
            self.floor
        } else {
            BlockStateId::AIR
        }
    }
    fn height(&self, _kind: HeightmapKind, _x: i32, _z: i32) -> i32 {
        self.floor_top + 1
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
        if pos.y <= self.floor_top { 0 } else { self.sky }
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

fn flat() -> Option<(CreatureSpawns, Flat)> {
    let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
    let registries = minecraftoss_core::Registries::load(&paths).ok()?;
    registries.entities.as_ref()?;
    let biome = registries.biomes.id("minecraft:plains")?;
    let floor = registries.blocks.parse_state("minecraft:stone").ok()?;
    let spawns = CreatureSpawns::load(
        std::sync::Arc::new(registries),
        "minecraft:overworld",
        false,
    )
    .ok()?;
    let level = Flat {
        floor_top: 63,
        floor,
        sky: 0,
        biome,
        random: LegacyRandom::new(5),
        made: 0,
        added: Vec::new(),
        notes: Vec::new(),
    };
    Some((spawns, level))
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

fn context(players: &[[f64; 3]], difficulty: i32) -> SpawnContext<'_> {
    SpawnContext {
        players,
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

fn only(spawns: &mut CreatureSpawns, biome: BiomeId, category: MobCategory, kind: &str) {
    spawns.by_category[usize::from(biome.0)][category as usize] = Creatures {
        entries: vec![SpawnerData {
            kind: kind.to_owned(),
            weight: 1,
            min: 4,
            max: 4,
            constant: true,
        }],
        total: 1,
    };
}

fn int(tag: &Tag, key: &str) -> Option<i64> {
    tag.get(key).and_then(Tag::as_i64)
}

fn follow_range(tag: &Tag) -> Option<f64> {
    let list = tag.get("attributes").and_then(Tag::as_list)?;
    let entry = list
        .iter()
        .find(|a| a.get("id").and_then(Tag::as_str) == Some("minecraft:follow_range"))?;
    entry.get("base").and_then(Tag::as_f64)
}

fn natural(
    spawns: &CreatureSpawns,
    level: &mut Flat,
    category: MobCategory,
    difficulty: i32,
) -> Vec<Vec<Tag>> {
    let players = [[8.5, 64.0, 40.5]];
    let mut calls = Vec::new();
    for _ in 0..40 {
        let before = level.added.len();
        let mut spawn_context = context(&players, difficulty);
        spawns.spawn_category_for_position(
            level,
            &mut spawn_context,
            &mut Unlimited,
            category,
            ChunkPos::new(0, 0),
            BlockPos::new(8, 64, 8),
        );
        calls.push(level.added[before..].to_vec());
    }
    calls
}

#[test]
fn a_family_row_comes_before_the_built_in_tables() {
    for kind in ["minecraft:silverfish", "minecraft:endermite"] {
        let info = type_info(kind).unwrap();
        assert_eq!(
            (info.width, info.height, info.placement, info.fire_immune),
            (0.4, 0.3, Placement::OnGround, false),
            "{kind}"
        );
        assert!(matches!(
            info.heightmap,
            HeightmapKind::MotionBlockingNoLeaves
        ));
        let walk = if kind == "minecraft:silverfish" {
            Walk::MonsterOver(&super::pests::HOSTS)
        } else {
            Walk::Monster
        };
        assert_eq!(
            (info.rules, info.walk, info.remove_far, info.cluster),
            (Rules::Family { creature: false }, walk, true, 4)
        );
        assert!(!info.rules.creature());
        assert_eq!(
            MobCategory::of_type(kind),
            Some(MobCategory::Monster),
            "{kind}"
        );
        assert_eq!(type_width(kind), Some(0.4));
        let found = row(kind).unwrap();
        assert_eq!(found.kind, kind);
        assert_eq!((found.follow_range, found.baby_chance), (16.0, None));
        assert!(found.rule.is_some() && found.finalize.is_none());
    }
    assert_eq!(
        row("silverfish").map(|r| r.kind),
        Some("minecraft:silverfish")
    );
    assert!(Rules::Family { creature: true }.creature());
    let kinds: Vec<&str> = rows().map(|r| r.kind).collect();
    for (i, kind) in kinds.iter().enumerate() {
        assert!(kind.starts_with("minecraft:"), "{kind}");
        assert!(!kinds[i + 1..].contains(kind), "{kind} has two rows");
    }
    let Some((spawns, mut level)) = flat() else {
        return;
    };
    let players: [[f64; 3]; 0] = [];
    for kind in ["minecraft:silverfish", "minecraft:endermite"] {
        let tag = spawns
            .summon(
                &mut level,
                &mut context(&players, 2),
                kind,
                [0.5, 64.0, 0.5],
                None,
                0.0,
            )
            .unwrap();
        assert_eq!(follow_range(&tag), Some(16.0), "{kind}");
    }
    let giant = spawns
        .summon(
            &mut level,
            &mut context(&players, 2),
            "minecraft:giant",
            [0.5, 64.0, 0.5],
            None,
            0.0,
        )
        .unwrap();
    assert_eq!(follow_range(&giant), Some(30.0));
}

#[test]
fn a_row_overrides_a_built_in_kind() {
    let info = type_info("minecraft:mooshroom").unwrap();
    assert_eq!((info.width, info.height, info.cluster), (0.5, 0.5, 8));
    assert_eq!(
        (info.rules, info.walk, info.remove_far),
        (Rules::Family { creature: true }, Walk::Neutral, false)
    );
    assert!(info.rules.creature());
    assert_eq!(type_width("minecraft:mooshroom"), Some(0.5));
    assert_eq!(
        MobCategory::of_type("minecraft:mooshroom"),
        Some(MobCategory::Creature)
    );
    assert_eq!(
        MobCategory::of_type("minecraft:giant"),
        Some(MobCategory::Monster)
    );
    assert_eq!(
        type_info("minecraft:giant").map(|i| i.rules),
        Some(Rules::Family { creature: false })
    );
    let Some((spawns, mut level)) = flat() else {
        return;
    };
    let players: [[f64; 3]; 0] = [];
    let tag = spawns
        .summon(
            &mut level,
            &mut context(&players, 2),
            "minecraft:mooshroom",
            [0.5, 64.0, 0.5],
            None,
            0.0,
        )
        .unwrap();
    assert_eq!(int(&tag, "ProbeMember"), Some(1));
    assert_eq!(int(&tag, "ProbeDifficulty"), Some(-1));
    assert!(level.notes.is_empty(), "{:?}", level.notes);
}

#[test]
fn silverfish_spawn_by_any_light_from_spawners_and_never_near_a_player() {
    let Some((mut spawns, mut level)) = flat() else {
        return;
    };
    let kind = "minecraft:silverfish";
    let info = type_info(kind).unwrap();
    let pos = BlockPos::new(0, 64, 0);
    level.sky = 15;
    let rule = |level: &mut Flat, players: &[[f64; 3]], spawner: bool| {
        super::rule(
            &spawns,
            level,
            &context(players, 2),
            kind,
            &info,
            pos,
            spawner,
        )
    };
    assert!(rule(&mut level, &[], false), "full daylight");
    assert!(
        rule(&mut level, &[[0.5, 64.5, 5.5]], false),
        "five blocks away"
    );
    assert!(
        !rule(&mut level, &[[0.5, 64.5, 5.4]], false),
        "within five blocks"
    );
    assert!(!rule(
        &mut level,
        &[[30.0, 64.0, 0.5], [0.5, 64.0, 1.5]],
        false
    ));
    assert!(
        rule(&mut level, &[[0.5, 64.0, 1.5]], true),
        "a spawner ignores players"
    );
    let floor = level.floor;
    level.floor = spawns
        .registries
        .blocks
        .parse_state("minecraft:bedrock")
        .unwrap();
    assert!(!rule(&mut level, &[], false), "bedrock is no spawn block");
    assert!(
        rule(&mut level, &[], true),
        "a spawner ignores the block below"
    );
    level.floor = floor;
    let near = [[0.5, 64.0, 1.5]];
    level.sky = 0;
    assert!(spawns.spawner_may_spawn(
        &mut level,
        &context(&near, 2),
        kind,
        [0.5, 64.0, 0.5],
        SpawnerReason::Spawner
    ));
    assert!(
        !spawns.spawner_may_spawn(
            &mut level,
            &context(&near, 0),
            kind,
            [0.5, 64.0, 0.5],
            SpawnerReason::Spawner
        ),
        "peaceful"
    );
    level.sky = 15;
    let at = [0.5, 64.0, 0.5];
    assert!(
        spawns.spawner_may_spawn(
            &mut level,
            &context(&near, 2),
            kind,
            at,
            SpawnerReason::Spawner
        ),
        "stone below is worth 10 to a silverfish in any light"
    );
    assert!(
        !spawns.spawner_may_spawn(
            &mut level,
            &context(&near, 2),
            "minecraft:endermite",
            at,
            SpawnerReason::Spawner
        ),
        "an endermite has a monster's walk target, which refuses full light"
    );
    for host in super::pests::HOSTS {
        level.floor = spawns.registries.blocks.parse_state(host).unwrap();
        assert!(
            spawns.spawner_may_spawn(
                &mut level,
                &context(&near, 2),
                kind,
                at,
                SpawnerReason::Spawner
            ),
            "{host}"
        );
    }
    level.floor = spawns
        .registries
        .blocks
        .parse_state("minecraft:andesite")
        .unwrap();
    assert!(
        !spawns.spawner_may_spawn(
            &mut level,
            &context(&near, 2),
            kind,
            at,
            SpawnerReason::Spawner
        ),
        "away from its host blocks a silverfish needs the dark"
    );
    level.sky = 0;
    assert!(spawns.spawner_may_spawn(
        &mut level,
        &context(&near, 2),
        kind,
        at,
        SpawnerReason::Spawner
    ));
    level.floor = floor;
    let biome = level.biome;
    only(&mut spawns, biome, MobCategory::Monster, kind);
    let spawned: Vec<Tag> = natural(&spawns, &mut level, MobCategory::Monster, 2).concat();
    assert!(spawned.len() > 10, "{}", spawned.len());
    assert!(
        spawned
            .iter()
            .all(|t| t.get("id").and_then(Tag::as_str) == Some(kind))
    );
    assert!(level.notes.is_empty(), "{:?}", level.notes);
    assert!(
        natural(&spawns, &mut level, MobCategory::Monster, 0)
            .concat()
            .is_empty(),
        "peaceful"
    );
}

#[test]
fn a_family_finalize_runs_for_summons_and_natural_spawns_and_shares_group_data() {
    let Some((mut spawns, mut level)) = flat() else {
        return;
    };
    let players: [[f64; 3]; 0] = [];
    let giant = spawns
        .summon(
            &mut level,
            &mut context(&players, 3),
            "minecraft:giant",
            [0.5, 64.0, 0.5],
            None,
            0.0,
        )
        .unwrap();
    assert_eq!(
        (int(&giant, "ProbeMember"), int(&giant, "ProbeDifficulty")),
        (Some(1), Some(3))
    );
    assert_ne!(int(&giant, "Age"), Some(-24000));
    let mooshroom = spawns
        .summon(
            &mut level,
            &mut context(&players, 3),
            "minecraft:mooshroom",
            [0.5, 64.0, 0.5],
            None,
            0.0,
        )
        .unwrap();
    assert_eq!(
        (
            int(&mooshroom, "ProbeMember"),
            int(&mooshroom, "ProbeDifficulty")
        ),
        (Some(1), Some(-1))
    );
    let biome = level.biome;
    for (category, kind, difficulty) in [
        (MobCategory::Monster, "minecraft:giant", 2),
        (MobCategory::Creature, "minecraft:mooshroom", -1),
    ] {
        only(&mut spawns, biome, category, kind);
        let mut grouped = 0;
        for call in natural(&spawns, &mut level, category, 2) {
            let mut previous = 0;
            for tag in &call {
                assert_eq!(tag.get("id").and_then(Tag::as_str), Some(kind));
                let member = int(tag, "ProbeMember").unwrap();
                assert!(
                    member == 1 || member == previous + 1,
                    "{kind}: {member} after {previous}"
                );
                assert_eq!(int(tag, "ProbeDifficulty"), Some(difficulty), "{kind}");
                assert_eq!(
                    int(tag, "Age") == Some(-24000),
                    member > 1,
                    "{kind} member {member}"
                );
                if member > 1 {
                    grouped += 1;
                }
                previous = member;
            }
        }
        assert!(grouped > 5, "{kind}: {grouped}");
    }
    assert!(level.notes.is_empty(), "{:?}", level.notes);
}

#[test]
fn a_row_without_finalize_or_babies_is_supported_and_draws_nothing() {
    let Some((spawns, mut level)) = flat() else {
        return;
    };
    let players: [[f64; 3]; 0] = [];
    for kind in ["minecraft:silverfish", "minecraft:endermite"] {
        spawns
            .summon(
                &mut level,
                &mut context(&players, 2),
                kind,
                [0.5, 64.0, 0.5],
                None,
                0.0,
            )
            .unwrap();
    }
    assert!(level.notes.is_empty(), "{:?}", level.notes);
    let fresh = |kind: &str| {
        spawns
            .registries
            .entities
            .as_ref()
            .and_then(|c| c.default_tag(kind))
            .unwrap()
    };
    let pos = BlockPos::new(0, 64, 0);
    for (kind, babies) in [
        ("minecraft:trader_llama", false),
        ("minecraft:mooshroom", true),
    ] {
        level.random = LegacyRandom::new(7);
        let mut group = Group::None;
        let mut tags = [fresh(kind), fresh(kind)];
        for tag in &mut tags {
            spawns.finalize_spawn(&mut level, kind, tag, pos, &mut group);
        }
        let mut alone = LegacyRandom::new(7);
        for _ in 0..2 {
            finalize_mob(&mut fresh(kind), &mut alone);
        }
        assert_eq!(
            level.random.next_i64() != alone.next_i64(),
            babies,
            "{kind}"
        );
        assert_eq!(int(&tags[1], "Age") == Some(-24000), babies, "{kind}");
        assert_eq!(matches!(group, Group::None), !babies, "{kind}");
        assert!(
            tags.iter()
                .all(|t| t.get("Strength") == fresh(kind).get("Strength")),
            "{kind}"
        );
    }
    assert!(level.notes.is_empty(), "{:?}", level.notes);
}
