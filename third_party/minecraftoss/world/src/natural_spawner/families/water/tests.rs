use crate::natural_spawner::spawning::{MobCategory, SpawnContext, Unlimited};
use crate::natural_spawner::{
    CreatureSpawns, Creatures, Placement, Rules, SpawnLevel, SpawnerData, StructureHere, Walk,
    type_info,
};
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::{LegacyRandom, RandomSource};
use minecraftoss_core::{BiomeId, BlockPos, BlockStateId, ChunkPos};

const SQUID: &str = "minecraft:squid";
const GLOW_SQUID: &str = "minecraft:glow_squid";

struct Ocean {
    water: BlockStateId,
    stone: BlockStateId,
    floor: i32,
    top: i32,
    light: i32,
    sea: i32,
    biome: BiomeId,
    random: LegacyRandom,
    made: i32,
    added: Vec<Tag>,
    notes: Vec<String>,
}

impl SpawnLevel for Ocean {
    fn block(&self, pos: BlockPos) -> BlockStateId {
        if pos.y < self.floor {
            self.stone
        } else if pos.y <= self.top {
            self.water
        } else {
            BlockStateId::AIR
        }
    }
    fn height(&self, _kind: HeightmapKind, _x: i32, _z: i32) -> i32 {
        self.top + 1
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
        if pos.y > self.top { 15 } else { self.light }
    }
    fn block_brightness(&mut self, _pos: BlockPos) -> i32 {
        0
    }
    fn sea_level(&self) -> i32 {
        self.sea
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

fn ocean(biome: &str) -> Option<(CreatureSpawns, Ocean)> {
    let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
    let registries = minecraftoss_core::Registries::load(&paths).ok()?;
    registries.entities.as_ref()?;
    let biome = registries.biomes.id(biome)?;
    let water = registries.blocks.parse_state("minecraft:water").ok()?;
    let stone = registries.blocks.parse_state("minecraft:stone").ok()?;
    let spawns = CreatureSpawns::load(
        std::sync::Arc::new(registries),
        "minecraft:overworld",
        false,
    )
    .ok()?;
    let ocean = Ocean {
        water,
        stone,
        floor: 40,
        top: 63,
        light: 8,
        sea: 63,
        biome,
        random: LegacyRandom::new(11),
        made: 0,
        added: Vec::new(),
        notes: Vec::new(),
    };
    Some((spawns, ocean))
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

fn only(spawns: &mut CreatureSpawns, ocean: &Ocean, category: MobCategory, kind: &str) {
    spawns.by_category[usize::from(ocean.biome.0)][category as usize] = Creatures {
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

fn allowed(spawns: &CreatureSpawns, ocean: &mut Ocean, kind: &str, y: i32, spawner: bool) -> bool {
    let info = type_info(kind).unwrap();
    let pos = BlockPos::new(0, y, 0);
    super::super::rule(spawns, ocean, &context(&[]), kind, &info, pos, spawner)
}

fn natural(
    spawns: &CreatureSpawns,
    ocean: &mut Ocean,
    category: MobCategory,
    y: i32,
    calls: usize,
) -> Vec<Vec<Tag>> {
    let players = [[8.5, 64.0, 40.5]];
    let mut out = Vec::new();
    for _ in 0..calls {
        let before = ocean.added.len();
        spawns.spawn_category_for_position(
            ocean,
            &mut context(&players),
            &mut Unlimited,
            category,
            ChunkPos::new(0, 0),
            BlockPos::new(8, y, 8),
        );
        out.push(ocean.added[before..].to_vec());
    }
    out
}

fn kind_of(tag: &Tag) -> Option<&str> {
    tag.get("id").and_then(Tag::as_str)
}

fn y_of(tag: &Tag) -> f64 {
    let pos = tag.get("Pos").and_then(Tag::as_list).unwrap();
    pos[1].as_f64().unwrap()
}

fn baby(tag: &Tag) -> bool {
    tag.get("Age").and_then(Tag::as_i64) == Some(-24000)
}

#[test]
fn surface_water_animals_spawn_from_thirteen_below_sea_level_up_to_it() {
    let info = type_info(SQUID).unwrap();
    assert_eq!(
        (info.width, info.height, info.placement, info.fire_immune),
        (0.8, 0.8, Placement::InWater, false)
    );
    assert!(matches!(
        info.heightmap,
        HeightmapKind::MotionBlockingNoLeaves
    ));
    assert_eq!(
        (info.rules, info.walk, info.remove_far, info.cluster),
        (Rules::Family { creature: true }, Walk::Neutral, true, 4)
    );
    assert_eq!(
        MobCategory::of_type(SQUID),
        Some(MobCategory::WaterCreature)
    );
    let row = super::super::row(SQUID).unwrap();
    assert_eq!((row.follow_range, row.baby_chance), (16.0, Some(0.05)));
    assert!(row.rule.is_some() && row.finalize.is_none());
    let Some((spawns, mut ocean)) = ocean("minecraft:lukewarm_ocean") else {
        return;
    };
    ocean.top = 70;
    for spawner in [false, true] {
        for (y, expected) in [(49, false), (50, true), (63, true), (64, false)] {
            let found = allowed(&spawns, &mut ocean, SQUID, y, spawner);
            assert_eq!(found, expected, "y {y}, spawner {spawner}");
        }
    }
    ocean.top = 63;
    assert!(!allowed(&spawns, &mut ocean, SQUID, 63, false), "air above");
    assert!(allowed(&spawns, &mut ocean, SQUID, 62, false));
    ocean.floor = 55;
    assert!(
        !allowed(&spawns, &mut ocean, SQUID, 55, false),
        "stone below"
    );
    assert!(allowed(&spawns, &mut ocean, SQUID, 56, false));
    ocean.sea = 40;
    ocean.floor = 0;
    for (y, expected) in [(26, false), (27, true), (40, true), (41, false)] {
        let found = allowed(&spawns, &mut ocean, SQUID, y, false);
        assert_eq!(found, expected, "y {y} by the level's sea level");
    }
}

#[test]
fn squids_spawn_in_the_top_thirteen_blocks_of_open_water() {
    let Some((mut spawns, mut ocean)) = ocean("minecraft:lukewarm_ocean") else {
        return;
    };
    ocean.floor = 30;
    only(&mut spawns, &ocean, MobCategory::WaterCreature, SQUID);
    for y in [35, 49, 63] {
        let calls = natural(&spawns, &mut ocean, MobCategory::WaterCreature, y, 20);
        assert!(calls.concat().is_empty(), "none at y {y}");
    }
    let mut spawned = 0;
    for y in [50, 56, 62] {
        let calls = natural(&spawns, &mut ocean, MobCategory::WaterCreature, y, 20);
        for tag in calls.concat() {
            assert_eq!(kind_of(&tag), Some(SQUID));
            assert_eq!(y_of(&tag), f64::from(y));
            spawned += 1;
        }
    }
    assert!(spawned > 20, "{spawned}");
    assert!(ocean.notes.is_empty(), "{:?}", ocean.notes);
}

#[test]
fn glow_squids_spawn_only_in_unlit_water_thirty_three_below_sea_level() {
    assert_eq!(
        MobCategory::of_type(GLOW_SQUID),
        Some(MobCategory::UndergroundWaterCreature)
    );
    let info = type_info(GLOW_SQUID).unwrap();
    assert_eq!(
        (info.width, info.placement, info.rules),
        (0.8, Placement::InWater, Rules::Family { creature: true })
    );
    let Some((mut spawns, mut ocean)) = ocean("minecraft:deep_ocean") else {
        return;
    };
    ocean.floor = 0;
    ocean.light = 0;
    for spawner in [false, true] {
        assert!(allowed(&spawns, &mut ocean, GLOW_SQUID, 30, spawner));
        assert!(!allowed(&spawns, &mut ocean, GLOW_SQUID, 31, spawner));
    }
    assert!(
        !allowed(&spawns, &mut ocean, GLOW_SQUID, -1, false),
        "in stone"
    );
    ocean.light = 1;
    assert!(
        !allowed(&spawns, &mut ocean, GLOW_SQUID, 20, false),
        "a lit cave"
    );
    ocean.light = 0;
    only(
        &mut spawns,
        &ocean,
        MobCategory::UndergroundWaterCreature,
        GLOW_SQUID,
    );
    let category = MobCategory::UndergroundWaterCreature;
    assert!(
        natural(&spawns, &mut ocean, category, 40, 20)
            .concat()
            .is_empty()
    );
    let spawned = natural(&spawns, &mut ocean, category, 20, 20).concat();
    assert!(spawned.len() > 10, "{}", spawned.len());
    assert!(spawned.iter().all(|tag| kind_of(tag) == Some(GLOW_SQUID)));
    assert!(ocean.notes.is_empty(), "{:?}", ocean.notes);
}

#[test]
fn a_squid_group_starts_with_an_adult_and_has_babies_one_time_in_twenty() {
    let Some((mut spawns, mut ocean)) = ocean("minecraft:warm_ocean") else {
        return;
    };
    only(&mut spawns, &ocean, MobCategory::WaterCreature, SQUID);
    let calls = natural(&spawns, &mut ocean, MobCategory::WaterCreature, 56, 300);
    let (mut total, mut babies) = (0, 0);
    for call in calls.iter().filter(|call| !call.is_empty()) {
        assert!(!baby(&call[0]), "an adult first");
        assert!(call.len() <= 4, "a cluster of at most four");
        total += call.len();
        babies += call.iter().filter(|tag| baby(tag)).count();
    }
    assert!(total > 300, "{total}");
    assert!(babies > 0 && babies * 10 < total, "{babies} of {total}");
    assert!(ocean.notes.is_empty(), "{:?}", ocean.notes);
}

#[test]
fn water_spawns_ignore_the_water_in_their_box() {
    let Some((spawns, mut ocean)) = ocean("minecraft:ocean") else {
        return;
    };
    for kind in [SQUID, GLOW_SQUID, "minecraft:axolotl", "minecraft:drowned"] {
        let info = type_info(kind).unwrap();
        assert_eq!(info.placement, Placement::InWater, "{kind}");
        let free = spawns.check_spawn_obstruction(&mut ocean, &info, 0.5, 50.0, 0.5);
        assert!(free, "{kind}");
    }
    let cow = type_info("minecraft:cow").unwrap();
    let in_water = spawns.check_spawn_obstruction(&mut ocean, &cow, 0.5, 50.0, 0.5);
    assert!(!in_water, "a cow is still kept out of water");
    assert!(spawns.check_spawn_obstruction(&mut ocean, &cow, 0.5, 70.0, 0.5));
}

#[test]
fn summoning_a_squid_notes_nothing_unsupported() {
    let Some((spawns, mut ocean)) = ocean("minecraft:ocean") else {
        return;
    };
    for kind in [SQUID, GLOW_SQUID] {
        let tag = spawns
            .summon(
                &mut ocean,
                &mut context(&[]),
                kind,
                [0.5, 50.0, 0.5],
                None,
                0.0,
            )
            .unwrap();
        assert_eq!(kind_of(&tag), Some(kind));
        assert!(!baby(&tag), "{kind}: a summoned squid is an adult");
    }
    assert!(ocean.notes.is_empty(), "{:?}", ocean.notes);
}
