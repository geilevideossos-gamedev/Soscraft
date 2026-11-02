use super::super::spawning::NaturalSpawning;
use super::super::update;
use super::*;
use crate::natural_spawner::CreatureSpawns;
use minecraftoss_core::registries::DataPaths;
use minecraftoss_core::{BiomeId, Chunk, ChunkPos, Registries};
use minecraftoss_generator::feature::Library;
use minecraftoss_generator::providers::GenContext;
use std::collections::BTreeMap;
use std::sync::Arc;

const FLOOR: i32 = 64;

fn library() -> Option<Library> {
    let paths = DataPaths::discover().ok()?;
    let registries = Arc::new(Registries::load(&paths).ok()?);
    registries.entities.as_ref()?;
    registries.block_entities.as_ref()?;
    let bounds = GenContext {
        min_y: -64,
        depth: 384,
        sea_level: 63,
    };
    Library::load(registries, bounds).ok()
}

fn level(lib: &Library) -> Option<Level<'_>> {
    let mut level = Level::new(lib, -64, 384);
    let stone = lib.registries.blocks.parse_state("minecraft:stone").ok()?;
    for cx in -1..=1 {
        for cz in -1..=1 {
            let mut chunk = Chunk::new(ChunkPos::new(cx, cz), -64, 384, BiomeId(0));
            chunk.status = minecraftoss_core::chunk::ChunkStatus::Full;
            for x in 0..16 {
                for z in 0..16 {
                    chunk.set_block_raw(x, FLOOR, z, stone, &lib.registries);
                }
            }
            level.insert_chunk(chunk);
        }
    }
    let spawns = CreatureSpawns::load(lib.registries.clone(), "minecraft:overworld", false).ok()?;
    level.natural_spawning = Some(NaturalSpawning::new(Arc::new(spawns)));
    Some(level)
}

fn place(level: &mut Level<'_>, pos: BlockPos, state: &str) {
    let state = level.registries().blocks.parse_state(state).unwrap();
    level.set_block(pos, state, update::ALL, update::LIMIT);
}

fn name(level: &Level<'_>, pos: BlockPos) -> String {
    level.name(level.block(pos)).to_owned()
}

fn bucket(data: Vec<(&str, Tag)>) -> Tag {
    let data: BTreeMap<String, Tag> = data
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    let mut components = BTreeMap::new();
    components.insert(BUCKET_ENTITY_DATA.to_owned(), Tag::Compound(data));
    Tag::Compound(components)
}

fn only_spawned(level: &mut Level<'_>) -> Tag {
    let mut spawned = std::mem::take(&mut level.spawned);
    assert_eq!(spawned.len(), 1, "{spawned:?}");
    spawned.remove(0)
}

fn position(tag: &Tag) -> Vec<f64> {
    let list = tag.get("Pos").and_then(Tag::as_list).unwrap();
    list.iter().filter_map(Tag::as_f64).collect()
}

fn events(level: &mut Level<'_>) -> Vec<(&'static str, [f64; 3])> {
    let events = level.take_game_events();
    events
        .into_iter()
        .map(|event| (event.event, event.pos))
        .collect()
}

#[test]
fn a_cod_bucket_pours_water_and_summons_a_cod_from_the_bucket() {
    let Some(lib) = library() else {
        return;
    };
    let Some(mut level) = level(&lib) else {
        return;
    };
    let at = BlockPos::new(2, FLOOR + 1, 2);
    level.take_game_events();
    let data = bucket(vec![
        ("Health", Tag::Double(1.5)),
        ("NoAI", Tag::Int(1)),
        ("PersistenceRequired", Tag::Int(1)),
        ("Silent", Tag::Byte(0)),
        ("Pos", Tag::List(vec![Tag::Double(9.0); 3])),
        ("Age", Tag::Int(-500)),
        ("Variant", Tag::Int(7)),
    ]);
    assert!(level.empty_mob_bucket(at, at, "minecraft:cod_bucket", Some(&data)));
    assert_eq!(name(&level, at), "minecraft:water");
    let cod = only_spawned(&mut level);
    assert_eq!(cod.get("id").and_then(Tag::as_str), Some("minecraft:cod"));
    assert_eq!(
        position(&cod),
        [2.5, 65.0, 2.5],
        "on the poured block, not the saved Pos"
    );
    assert_eq!(cod.get("FromBucket"), Some(&Tag::Byte(1)));
    assert_eq!(cod.get("Health"), Some(&Tag::Float(1.5)));
    assert_eq!(cod.get("NoAI"), Some(&Tag::Byte(1)));
    assert_eq!(cod.get("PersistenceRequired"), Some(&Tag::Byte(1)));
    assert_eq!(cod.get("Silent"), Some(&Tag::Byte(0)));
    assert_eq!(
        cod.get("Variant"),
        None,
        "a key no cod bucket carries is ignored"
    );
    assert_ne!(
        cod.get("Age"),
        Some(&Tag::Int(-500)),
        "the cod's row has no Age"
    );
    let yaw = cod.get("Rotation").and_then(Tag::as_list).unwrap()[0]
        .as_f64()
        .unwrap();
    assert!((-180.0..180.0).contains(&yaw), "{yaw}");
    let centre = [2.5, 65.5, 2.5];
    let posted = events(&mut level);
    assert!(
        posted.contains(&("minecraft:fluid_place", centre)),
        "{posted:?}"
    );
    assert!(
        posted.contains(&("minecraft:entity_place", centre)),
        "{posted:?}"
    );
    let plain = BlockPos::new(5, FLOOR + 1, 5);
    assert!(level.empty_mob_bucket(plain, plain, "minecraft:cod_bucket", None));
    let fresh = only_spawned(&mut level);
    assert_eq!(fresh.get("FromBucket"), Some(&Tag::Byte(1)));
    assert_ne!(fresh.get("PersistenceRequired"), Some(&Tag::Byte(1)));
    assert_ne!(fresh.get("Health"), Some(&Tag::Float(1.5)));
    let kept = BlockPos::new(8, FLOOR + 1, 8);
    let data = bucket(vec![("PersistenceRequired", Tag::Byte(0))]);
    assert!(level.empty_mob_bucket(kept, kept, "minecraft:cod_bucket", Some(&data)));
    let free = only_spawned(&mut level);
    assert_eq!(
        free.get("PersistenceRequired"),
        fresh.get("PersistenceRequired"),
        "false leaves the default"
    );
}

#[test]
fn a_bucketed_mob_lands_on_what_the_water_went_into() {
    let Some(lib) = library() else {
        return;
    };
    let Some(mut level) = level(&lib) else {
        return;
    };
    let slab = BlockPos::new(3, FLOOR + 1, 3);
    place(
        &mut level,
        slab,
        "minecraft:stone_slab[type=bottom,waterlogged=false]",
    );
    let axolotl_keys = bucket(vec![("Age", Tag::Int(-1200)), ("AgeLocked", Tag::Byte(1))]);
    assert!(level.empty_mob_bucket(slab, slab, "minecraft:axolotl_bucket", Some(&axolotl_keys)));
    let state = level.block(slab);
    assert_eq!(
        level.registries().blocks.property(state, "waterlogged"),
        Some("true")
    );
    let axolotl = only_spawned(&mut level);
    assert_eq!(position(&axolotl), [3.5, 65.5, 3.5], "on top of the slab");
    assert_eq!(axolotl.get("Age"), Some(&Tag::Int(-1200)));
    assert_eq!(axolotl.get("AgeLocked"), Some(&Tag::Byte(1)));
    let stone = BlockPos::new(4, FLOOR, 4);
    assert!(
        !level.empty_mob_bucket(stone, stone, "minecraft:cod_bucket", None),
        "no water goes into stone"
    );
    assert!(level.spawned.is_empty(), "and no cod comes out");
    let air = BlockPos::new(6, FLOOR + 1, 6);
    assert!(!level.empty_mob_bucket(air, air, "minecraft:water_bucket", None));
    assert_eq!(name(&level, air), "minecraft:air");
}

#[test]
fn a_sulfur_cube_bucket_pours_nothing() {
    let Some(lib) = library() else {
        return;
    };
    let Some(mut level) = level(&lib) else {
        return;
    };
    let at = BlockPos::new(2, FLOOR + 1, 2);
    level.take_game_events();
    assert!(level.empty_mob_bucket(at, at, "minecraft:sulfur_cube_bucket", None));
    assert_eq!(name(&level, at), "minecraft:air");
    let cube = only_spawned(&mut level);
    assert_eq!(
        cube.get("id").and_then(Tag::as_str),
        Some("minecraft:sulfur_cube")
    );
    assert_eq!(cube.get("FromBucket"), Some(&Tag::Byte(1)));
    assert_eq!(position(&cube), [2.5, 65.0, 2.5]);
    let posted = events(&mut level);
    assert_eq!(posted, [("minecraft:entity_place", [2.5, 65.5, 2.5])]);
}

#[test]
fn a_fish_bucket_in_the_nether_hisses_and_still_leaves_its_fish() {
    let Some(lib) = library() else {
        return;
    };
    let Some(mut level) = level(&lib) else {
        return;
    };
    if level.set_dimension("minecraft:the_nether").is_err() {
        return;
    }
    let at = BlockPos::new(2, FLOOR + 1, 2);
    assert!(level.empty_mob_bucket(at, at, "minecraft:salmon_bucket", None));
    assert_eq!(name(&level, at), "minecraft:air", "the water boils away");
    let salmon = only_spawned(&mut level);
    assert_eq!(
        salmon.get("id").and_then(Tag::as_str),
        Some("minecraft:salmon")
    );
    assert_eq!(salmon.get("FromBucket"), Some(&Tag::Byte(1)));
}

#[test]
fn the_bucket_data_keeps_vanillas_keys_with_their_types() {
    let cod = mob_bucket("minecraft:cod_bucket").unwrap();
    let mut components = bucket(vec![
        ("Health", Tag::Int(2)),
        ("Glowing", Tag::Double(1.0)),
        ("Invulnerable", Tag::Byte(1)),
        ("NoGravity", Tag::Long(0)),
        ("PersistenceRequired", Tag::Byte(0)),
        ("UUID", Tag::IntArray(vec![1, 2, 3, 4])),
    ]);
    if let Tag::Compound(map) = &mut components {
        map.insert(CUSTOM_NAME.to_owned(), Tag::String("Gill".to_owned()));
    }
    let data: BTreeMap<String, Tag> = entity_data(cod, Some(&components)).into_iter().collect();
    let expected: BTreeMap<String, Tag> = [
        ("Health", Tag::Float(2.0)),
        ("Glowing", Tag::Byte(1)),
        ("Invulnerable", Tag::Byte(1)),
        ("NoGravity", Tag::Byte(0)),
        ("CustomName", Tag::String("Gill".to_owned())),
        ("FromBucket", Tag::Byte(1)),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect();
    assert_eq!(data, expected);
    let rows: Vec<(&str, &str, bool)> = MOB_BUCKETS
        .iter()
        .map(|row| (row.item, row.kind, row.water))
        .collect();
    assert_eq!(
        rows,
        [
            ("minecraft:cod_bucket", "minecraft:cod", true),
            ("minecraft:salmon_bucket", "minecraft:salmon", true),
            (
                "minecraft:tropical_fish_bucket",
                "minecraft:tropical_fish",
                true
            ),
            ("minecraft:pufferfish_bucket", "minecraft:pufferfish", true),
            ("minecraft:axolotl_bucket", "minecraft:axolotl", true),
            ("minecraft:tadpole_bucket", "minecraft:tadpole", true),
            (
                "minecraft:sulfur_cube_bucket",
                "minecraft:sulfur_cube",
                false
            ),
        ]
    );
    assert_eq!(
        mob_bucket("minecraft:water_bucket").map(|row| row.kind),
        None
    );
}

fn dispenser(level: &mut Level<'_>, at: BlockPos, facing: &str, stack: Stack) {
    place(
        level,
        at,
        &format!("minecraft:dispenser[facing={facing},triggered=false]"),
    );
    assert!(level.replace_block_item(at, 0, stack));
    level.sounds.clear();
    level.take_game_events();
}

fn held(level: &Level<'_>, at: BlockPos) -> Vec<(String, i32)> {
    let items = level.block_container_items(at).unwrap();
    items
        .into_iter()
        .filter(|stack| !stack.is_empty())
        .map(|stack| (stack.id, stack.count))
        .collect()
}

#[test]
fn a_dispenser_empties_a_cod_bucket_and_keeps_the_empty_bucket() {
    let Some(lib) = library() else {
        return;
    };
    let Some(mut level) = level(&lib) else {
        return;
    };
    let at = BlockPos::new(2, FLOOR + 1, 2);
    let front = BlockPos::new(3, FLOOR + 1, 2);
    let mut cod = Stack::new("minecraft:cod_bucket", 1);
    cod.components = Some(bucket(vec![("Health", Tag::Float(2.0))]));
    dispenser(&mut level, at, "east", cod);
    level.dispense_from(at, false);
    assert_eq!(name(&level, front), "minecraft:water");
    assert_eq!(held(&level, at), [("minecraft:bucket".to_owned(), 1)]);
    let fish = only_spawned(&mut level);
    assert_eq!(fish.get("id").and_then(Tag::as_str), Some("minecraft:cod"));
    assert_eq!(fish.get("FromBucket"), Some(&Tag::Byte(1)));
    assert_eq!(fish.get("Health"), Some(&Tag::Float(2.0)));
    assert_eq!(position(&fish), [3.5, 65.0, 2.5]);
    let splashes: Vec<(&str, [f64; 3])> = level
        .sounds
        .iter()
        .map(|sound| (sound.event, sound.pos))
        .collect();
    assert_eq!(splashes, [(EMPTY_FISH, [3.5, 65.5, 2.5])]);
    let placed = level.take_game_events();
    let entity_place = placed
        .iter()
        .find(|event| event.event == "minecraft:entity_place")
        .expect("entity_place");
    assert_eq!(
        (entity_place.pos, entity_place.source),
        ([3.5, 65.5, 2.5], None)
    );
    level.dispense_from(at, false);
    assert_eq!(
        held(&level, at),
        [("minecraft:water_bucket".to_owned(), 1)],
        "the empty bucket takes the water back"
    );
    let below = BlockPos::new(6, FLOOR + 1, 6);
    dispenser(
        &mut level,
        below,
        "down",
        Stack::new("minecraft:tadpole_bucket", 1),
    );
    level.dispense_from(below, false);
    assert!(
        held(&level, below).is_empty(),
        "into stone it is thrown out whole"
    );
    assert!(level.spawned.is_empty());
    assert!(level.sounds.is_empty());
}

#[test]
fn a_sulfur_cube_bucket_is_still_not_dispensed() {
    let Some(lib) = library() else {
        return;
    };
    let Some(mut level) = level(&lib) else {
        return;
    };
    let at = BlockPos::new(2, FLOOR + 1, 2);
    let cube = Stack::new("minecraft:sulfur_cube_bucket", 1);
    dispenser(&mut level, at, "east", cube);
    level.unsupported.clear();
    level.dispense_from(at, false);
    assert_eq!(
        held(&level, at),
        [("minecraft:sulfur_cube_bucket".to_owned(), 1)]
    );
    assert!(level.spawned.is_empty());
    assert_eq!(
        level.unsupported,
        ["dispensing minecraft:sulfur_cube_bucket"]
    );
}
