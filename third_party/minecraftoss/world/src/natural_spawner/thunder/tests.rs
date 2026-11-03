use super::*;
use crate::distance::PlayerChunkCounter;
use crate::level::Level;
use crate::level::entity::EntityKind;
use crate::level::update;
use crate::natural_spawner::StructureHere;
use crate::natural_spawner::tick::{CensusMob, SpawnPlayer};
use minecraftoss_core::chunk::ChunkStatus;
use minecraftoss_core::random::{AnyRandom, RandomSource};
use minecraftoss_core::registries::DataPaths;
use minecraftoss_core::{BiomeId, Chunk, Registries};
use minecraftoss_generator::feature::Library;
use minecraftoss_generator::providers::GenContext;
use std::sync::Arc;

const FLOOR: i32 = 64;
const GROUND: i32 = 65;
const COLUMN: (i32, i32) = (3, 3);

fn library() -> Option<Library> {
    let paths = DataPaths::discover().ok()?;
    let registries = Arc::new(Registries::load(&paths).ok()?);
    registries.entities.as_ref()?;
    let bounds = GenContext {
        min_y: -64,
        depth: 384,
        sea_level: 63,
    };
    Library::load(registries, bounds).ok()
}

fn storm<'a>(
    lib: &'a Library,
    biome: &str,
    xs: std::ops::RangeInclusive<i32>,
    zs: std::ops::RangeInclusive<i32>,
) -> Option<Level<'a>> {
    let mut level = Level::new(lib, -64, 384);
    level.set_dimension("minecraft:overworld").ok()?;
    let sky = level.sky.as_mut()?;
    sky.weather.rain_level = 1.0;
    sky.weather.thunder_level = 1.0;
    let biome = lib.registries.biomes.id(biome)?;
    let stone = lib.registries.blocks.parse_state("minecraft:stone").ok()?;
    for cx in xs {
        for cz in zs.clone() {
            let mut chunk = Chunk::new(ChunkPos::new(cx, cz), -64, 384, biome);
            chunk.status = ChunkStatus::Full;
            for x in 0..16 {
                for z in 0..16 {
                    chunk.set_block_raw(x, FLOOR, z, stone, &lib.registries);
                }
            }
            chunk.prime_heightmaps(&HeightmapKind::ALL, &lib.registries);
            level.insert_chunk(chunk);
        }
    }
    Some(level)
}

fn plains<'a>(lib: &'a Library, biome: &str) -> Option<Level<'a>> {
    storm(lib, biome, -1..=1, -2..=9)
}

fn spawns(lib: &Library) -> Option<CreatureSpawns> {
    CreatureSpawns::load(lib.registries.clone(), "minecraft:overworld", false).ok()
}

fn place(level: &mut Level<'_>, pos: BlockPos, state: &str) {
    let state = level.registries().blocks.parse_state(state).unwrap();
    level.set_block(pos, state, update::ALL, update::LIMIT);
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

fn context<'a>(difficulty: i32) -> SpawnContext<'a> {
    SpawnContext {
        players: &[],
        respawn: None,
        difficulty,
        overworld_time: 0,
        moon_brightness: 0.0,
        raining: true,
        thundering: true,
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

fn aim(level: &mut Level<'_>, x: i32, z: i32) {
    let next = (x | z << 8) << 2;
    level.rand_value = next
        .wrapping_sub(1_013_904_223)
        .wrapping_mul(0xAAAA_AAABu32 as i32);
}

struct Struck {
    bolts: Vec<([f64; 3], bool)>,
    horses: Vec<Tag>,
}

struct Around<'a> {
    census: &'a [CensusMob],
    players: &'a [SpawnPlayer],
    spawn_mobs: bool,
    difficulty: i32,
}

const NOBODY: Around<'static> = Around {
    census: &[],
    players: &[],
    spawn_mobs: false,
    difficulty: 2,
};

fn bolts_from(level: &Level<'_>, before: usize) -> Vec<([f64; 3], bool)> {
    level.entities[before..]
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::LightningBolt(data) => Some((e.pos, data.visual_only)),
            _ => None,
        })
        .collect()
}

fn strike(spawns: &CreatureSpawns, level: &mut Level<'_>, around: &Around<'_>) -> Struck {
    aim(level, COLUMN.0, COLUMN.1);
    let before = level.entities.len();
    let mut counter = PlayerChunkCounter::new(8);
    let input = TickInput {
        counter: &mut counter,
        players: around.players,
        census: around.census,
        spawn_mobs: around.spawn_mobs,
        spawn_enemies: around.spawn_mobs,
        spawn_persistent: false,
        loaded: &any_chunk,
        ticking: &any_chunk,
        entity_ticking_range: &any_chunk,
    };
    let context = context(around.difficulty);
    spawns.strike_thunder(level, &context, &input, ChunkPos::new(0, 0));
    Struck {
        bolts: bolts_from(level, before),
        horses: std::mem::take(&mut level.spawned),
    }
}

fn above(pos: BlockPos) -> [f64; 3] {
    [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y + 1),
        f64::from(pos.z) + 0.5,
    ]
}

fn on(x: i32, y: i32, z: i32) -> [f64; 3] {
    [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5]
}

fn cow(x: f64, y: f64, z: f64) -> CensusMob {
    CensusMob {
        kind: "minecraft:cow".to_owned(),
        pos: [x, y, z],
        bb: [x - 0.45, y, z - 0.45, x + 0.45, y + 1.4, z + 0.45],
        persistent: false,
        riding: false,
    }
}

fn player(x: f64, y: f64, z: f64, spectator: bool, alive: bool) -> SpawnPlayer {
    SpawnPlayer {
        pos: [x, y, z],
        spectator,
        alive,
    }
}

#[test]
fn a_strike_aims_at_the_nearest_surface_rod_within_128() {
    let Some(lib) = library() else { return };
    let (Some(spawns), Some(mut level)) = (spawns(&lib), plains(&lib, "minecraft:plains")) else {
        return;
    };
    let near = BlockPos::new(8, GROUND, 60);
    let far = BlockPos::new(8, GROUND, 120);
    place(&mut level, near, "minecraft:lightning_rod");
    place(&mut level, far, "minecraft:weathered_lightning_rod");
    assert_eq!(
        level.lightning_rod_near(BlockPos::new(3, GROUND, 3)),
        Some(near)
    );
    let struck = strike(&spawns, &mut level, &NOBODY);
    assert_eq!(struck.bolts, vec![(above(near), false)]);
    assert!(struck.horses.is_empty());

    place(&mut level, near, "minecraft:air");
    let struck = strike(&spawns, &mut level, &NOBODY);
    assert_eq!(
        struck.bolts,
        vec![(above(far), false)],
        "a waxed or weathered rod draws it too"
    );

    place(&mut level, far, "minecraft:air");
    place(
        &mut level,
        BlockPos::new(8, GROUND, 140),
        "minecraft:lightning_rod",
    );
    place(
        &mut level,
        BlockPos::new(8, 200, 8),
        "minecraft:lightning_rod",
    );
    let census = [cow(4.5, 65.0, 4.5)];
    let around = Around {
        census: &census,
        ..NOBODY
    };
    let struck = strike(&spawns, &mut level, &around);
    assert_eq!(
        struck.bolts,
        vec![(on(4, GROUND, 4), false)],
        "rods past 128 blocks, across or straight up, are left out"
    );
}

#[test]
fn a_buried_rod_is_ignored() {
    let Some(lib) = library() else { return };
    let (Some(spawns), Some(mut level)) = (spawns(&lib), plains(&lib, "minecraft:plains")) else {
        return;
    };
    let buried = BlockPos::new(6, GROUND, 20);
    place(&mut level, buried, "minecraft:lightning_rod");
    place(&mut level, buried.above(), "minecraft:stone");
    let roofed = BlockPos::new(5, GROUND, 5);
    place(&mut level, roofed, "minecraft:lightning_rod");
    place(&mut level, BlockPos::new(5, 70, 5), "minecraft:glass");
    let open = BlockPos::new(8, GROUND, 100);
    place(&mut level, open, "minecraft:lightning_rod");
    let struck = strike(&spawns, &mut level, &NOBODY);
    assert_eq!(struck.bolts, vec![(above(open), false)]);

    place(&mut level, open, "minecraft:air");
    let struck = strike(&spawns, &mut level, &NOBODY);
    assert_eq!(
        struck.bolts,
        vec![(on(COLUMN.0, GROUND, COLUMN.1), false)],
        "with no rod at the top of its column the bolt hits the ground"
    );
}

#[test]
fn without_a_rod_it_picks_a_living_thing_under_open_sky() {
    let Some(lib) = library() else { return };
    let (Some(spawns), Some(mut level)) = (spawns(&lib), plains(&lib, "minecraft:plains")) else {
        return;
    };
    place(&mut level, BlockPos::new(5, 67, 1), "minecraft:stone");
    let census = [
        cow(5.5, 65.0, 5.5),
        cow(1.5, 65.0, 6.5),
        cow(5.5, 65.0, 1.5),
        cow(20.5, 65.0, 3.5),
    ];
    let players = [
        player(2.5, 65.0, 2.5, true, true),
        player(4.5, 65.0, 0.5, false, false),
        player(6.5, 200.0, 2.5, false, true),
        player(6.5, 330.0, 2.5, false, true),
    ];
    let around = Around {
        census: &census,
        players: &players,
        ..NOBODY
    };
    let mut picked = Vec::new();
    for _ in 0..40 {
        let struck = strike(&spawns, &mut level, &around);
        assert_eq!(struck.bolts.len(), 1);
        assert!(!struck.bolts[0].1);
        if !picked.contains(&struck.bolts[0].0) {
            picked.push(struck.bolts[0].0);
        }
    }
    picked.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut expected = vec![
        on(5, GROUND, 5),
        on(1, GROUND, 6),
        on(2, GROUND, 2),
        on(6, 200, 2),
    ];
    expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        picked, expected,
        "the cows and the spectator under open sky inside the box, and the player high above, never the roofed cow, the far cow, the dead player or one above the box"
    );

    let struck = strike(&spawns, &mut level, &NOBODY);
    assert_eq!(struck.bolts, vec![(on(COLUMN.0, GROUND, COLUMN.1), false)]);
}

#[test]
fn no_bolt_in_a_desert_a_snowy_biome_or_under_a_roof() {
    let Some(lib) = library() else { return };
    let Some(spawns) = spawns(&lib) else { return };
    for biome in ["minecraft:desert", "minecraft:snowy_plains"] {
        let Some(mut level) = plains(&lib, biome) else {
            return;
        };
        let struck = strike(&spawns, &mut level, &NOBODY);
        assert!(struck.bolts.is_empty(), "{biome}");
        assert!(struck.horses.is_empty(), "{biome}");
    }
    let Some(mut level) = plains(&lib, "minecraft:plains") else {
        return;
    };
    place(&mut level, BlockPos::new(5, 67, 5), "minecraft:glass");
    let census = [cow(5.5, 65.0, 5.5)];
    let around = Around {
        census: &census,
        ..NOBODY
    };
    let struck = strike(&spawns, &mut level, &around);
    assert!(
        struck.bolts.is_empty(),
        "a cow under glass sees the sky, but it does not rain on it"
    );
}

fn seed_where(first: impl Fn(f64) -> bool) -> i64 {
    (0..1_000_000)
        .find(|&seed| first(AnyRandom::new(true, seed).next_f64()))
        .expect("a seed")
}

#[test]
fn a_trap_roll_leaves_a_visual_bolt_and_a_trap_horse_tag() {
    let Some(lib) = library() else { return };
    let (Some(spawns), Some(mut level)) = (spawns(&lib), plains(&lib, "minecraft:plains")) else {
        return;
    };
    let around = Around {
        spawn_mobs: true,
        ..NOBODY
    };
    level.random = AnyRandom::new(true, seed_where(|odds| odds < 0.0149));
    let struck = strike(&spawns, &mut level, &around);
    let spot = on(COLUMN.0, GROUND, COLUMN.1);
    assert_eq!(struck.bolts, vec![(spot, true)], "a trap bolt is only seen");
    let [horse] = struck.horses.as_slice() else {
        panic!("one horse: {:?}", struck.horses);
    };
    assert_eq!(horse.get("id").and_then(Tag::as_str), Some(SKELETON_HORSE));
    assert_eq!(horse.get("SkeletonTrap").and_then(Tag::as_i64), Some(1));
    assert_eq!(horse.get("Age").and_then(Tag::as_i64), Some(0));
    let pos: Vec<f64> = horse
        .get("Pos")
        .and_then(Tag::as_list)
        .unwrap()
        .iter()
        .filter_map(Tag::as_f64)
        .collect();
    assert_eq!(
        pos,
        vec![f64::from(COLUMN.0), f64::from(GROUND), f64::from(COLUMN.1)]
    );
    assert!(horse.get("UUID").is_some());
    assert_ne!(
        horse.get("PersistenceRequired").and_then(Tag::as_i64),
        Some(1)
    );

    let normal_miss = seed_where(|odds| (0.0151..0.0224).contains(&odds));
    level.random = AnyRandom::new(true, normal_miss);
    let struck = strike(&spawns, &mut level, &around);
    assert_eq!(struck.bolts, vec![(spot, false)]);
    assert!(struck.horses.is_empty());
    level.random = AnyRandom::new(true, normal_miss);
    let hard = Around {
        difficulty: 3,
        ..around
    };
    let struck = strike(&spawns, &mut level, &hard);
    assert_eq!(struck.bolts, vec![(spot, true)], "Hard's odds are higher");
    assert_eq!(struck.horses.len(), 1);
}

#[test]
fn no_trap_without_spawn_mobs_or_above_a_rod() {
    let Some(lib) = library() else { return };
    let (Some(spawns), Some(mut level)) = (spawns(&lib), plains(&lib, "minecraft:plains")) else {
        return;
    };
    let trap_seed = seed_where(|odds| odds < 0.0149);
    level.random = AnyRandom::new(true, trap_seed);
    let struck = strike(&spawns, &mut level, &NOBODY);
    assert_eq!(struck.bolts, vec![(on(COLUMN.0, GROUND, COLUMN.1), false)]);
    assert!(struck.horses.is_empty());
    assert!(
        level.random.next_f64() < 0.015,
        "without spawn_mobs the roll is not drawn"
    );

    let rod = BlockPos::new(8, GROUND, 8);
    place(&mut level, rod, "minecraft:lightning_rod");
    level.random = AnyRandom::new(true, trap_seed);
    let around = Around {
        spawn_mobs: true,
        ..NOBODY
    };
    let struck = strike(&spawns, &mut level, &around);
    assert_eq!(struck.bolts, vec![(above(rod), false)]);
    assert!(struck.horses.is_empty(), "never a trap above a rod");
    let mut drawn = AnyRandom::new(true, trap_seed);
    drawn.next_f64();
    assert_eq!(
        level.random.next_f64(),
        drawn.next_f64(),
        "the roll was drawn"
    );
}

#[test]
fn a_long_storm_strikes_inside_the_spawning_chunks() {
    let Some(lib) = library() else { return };
    let (Some(spawns), Some(mut level)) = (
        spawns(&lib),
        storm(&lib, "minecraft:plains", -2..=2, -2..=2),
    ) else {
        return;
    };
    level.random = AnyRandom::new(true, 7);
    let feet = [[8.5, 65.0, 8.5]];
    let players = [player(8.5, 65.0, 8.5, false, true)];
    let mut counter = PlayerChunkCounter::new(2);
    counter.add_player(ChunkPos::new(0, 0));
    let inner = |c: ChunkPos| c.x.abs() <= 1 && c.z.abs() <= 1;
    let mut spawn_context = context(2);
    spawn_context.players = &feet;
    let mut input = TickInput {
        counter: &mut counter,
        players: &players,
        census: &[],
        spawn_mobs: false,
        spawn_enemies: false,
        spawn_persistent: false,
        loaded: &any_chunk,
        ticking: &any_chunk,
        entity_ticking_range: &inner,
    };
    let mut bolts = Vec::new();
    for _ in 0..100_000 {
        let before = level.entities.len();
        spawns.tick_spawning(&mut level, &mut spawn_context, &mut input);
        bolts.extend(bolts_from(&level, before));
    }
    assert!((2..=30).contains(&bolts.len()), "{}", bolts.len());
    for (at, visual) in &bolts {
        let chunk = ChunkPos::new((at[0].floor() as i32) >> 4, (at[2].floor() as i32) >> 4);
        assert!(inner(chunk), "{at:?}");
        assert_eq!(at[1], f64::from(GROUND));
        assert!(!visual);
    }
    let chunks: std::collections::HashSet<(i32, i32)> = bolts
        .iter()
        .map(|(at, _)| ((at[0].floor() as i32) >> 4, (at[2].floor() as i32) >> 4))
        .collect();
    assert!(chunks.len() > 1, "{chunks:?}");
}
