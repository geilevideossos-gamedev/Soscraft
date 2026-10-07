//! Chunk persistence for one dimension: vanilla 26.3 region files under
//! `<world>/dimensions/<namespace>/<path>/region`, chunks in the
//! `SerializableChunkData` layout (`minecraftoss_core::chunk_nbt`).
//!
//! Chunks of every status are saved, as vanilla saves proto chunks: a
//! chunk that was decorated keeps its decoration and is never decorated
//! again after a reload. FULL chunks' entities live in the entity storage,
//! `<dimension>/entities`, which the level writes as its entities leave
//! (`EntityStorage`); a FULL chunk saved before the level ever wrote its
//! entities seeds the storage with the ones generation made.

use minecraftoss_core::anvil::{region_of, region_path, RegionFile, StoredChunk};
use minecraftoss_core::chunk::ChunkStatus;
use minecraftoss_core::chunk_nbt::{read_chunk, write_chunk, write_entities};
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{Chunk, ChunkPos, Registries};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct ChunkStorage {
    dir: PathBuf,
    entities: EntityRegions,
    registries: Arc<Registries>,
    min_y: i32,
    height: i32,
    regions: Mutex<Regions>,
    /// Held while region files are written, so two flushes never write the
    /// same file at once; loads and saves only take the region caches.
    writing: Mutex<()>,
}

/// Region files of the entity storage.
struct EntityRegions {
    dir: PathBuf,
    regions: Mutex<Regions>,
}

#[derive(Default)]
struct Regions {
    loaded: HashMap<(i32, i32), RegionFile>,
    dirty: HashSet<(i32, i32)>,
}

impl ChunkStorage {
    /// Storage for a dimension (`minecraft:overworld`) of the world at `world`.
    pub fn new(world: &Path, dimension: &str, registries: Arc<Registries>, min_y: i32, height: i32) -> Self {
        let (namespace, path) = dimension.split_once(':').unwrap_or(("minecraft", dimension));
        let base = world.join("dimensions").join(namespace).join(path);
        Self {
            dir: base.join("region"),
            entities: EntityRegions { dir: base.join("entities"), regions: Mutex::default() },
            registries,
            min_y,
            height,
            regions: Mutex::default(),
            writing: Mutex::default(),
        }
    }

    fn with_region<T>(&self, region: (i32, i32), f: impl FnOnce(&mut RegionFile) -> T) -> Result<T, String> {
        with_region(&self.dir, &self.regions, region, f)
    }

    fn with_entity_region<T>(&self, region: (i32, i32), f: impl FnOnce(&mut RegionFile) -> T) -> Result<T, String> {
        with_region(&self.entities.dir, &self.entities.regions, region, f)
    }

    /// A saved chunk, if there is one. Only the lookup holds the region
    /// cache; the chunk is decompressed and read outside it.
    pub fn load(&self, pos: ChunkPos) -> Result<Option<Chunk>, String> {
        let (region, index) = region_of(pos);
        let Some(stored) = self.with_region(region, |file| file.stored(index))? else { return Ok(None) };
        let mut chunk = read_chunk(&stored.parse()?, &self.registries, self.min_y, self.height)?;
        if chunk.status == ChunkStatus::Full {
            let entities = self.with_entity_region(region, |file| file.stored(index))?.map(|stored| stored.parse()).transpose()?;
            chunk.generation.entities = entities.and_then(|t| t.get("Entities").and_then(Tag::as_list).map(<[Tag]>::to_vec)).unwrap_or_default();
        }
        Ok(Some(chunk))
    }

    /// Queues a chunk for the next `flush`, compressed before the region
    /// cache is taken.
    pub fn save(&self, chunk: &Chunk) -> Result<(), String> {
        let started = std::time::Instant::now();
        let tag = write_chunk(chunk, &self.registries, 0);
        crate::chunk_map::stage_times::add(crate::chunk_map::stage_times::SAVE_NBT, started);
        let started = std::time::Instant::now();
        let stored = StoredChunk::encode(&tag);
        crate::chunk_map::stage_times::add(crate::chunk_map::stage_times::SAVE_COMPRESS, started);
        let (region, index) = region_of(chunk.pos);
        store(&self.dir, &self.regions, region, index, stored)?;
        // The level owns a FULL chunk's entities once it has saved them.
        if chunk.status == ChunkStatus::Full && !self.with_entity_region(region, |file| file.contains(index))? {
            self.seed_entities(chunk.pos, &chunk.generation.entities)?;
        }
        Ok(())
    }

    pub fn seed_entities(&self, pos: ChunkPos, entities: &[Tag]) -> Result<bool, String> {
        let (region, index) = region_of(pos);
        let stored = StoredChunk::encode(&write_entities(pos, entities));
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32);
        let seeded = self.with_entity_region(region, |file| {
            let absent = !file.contains(index);
            if absent {
                file.set_stored(index, stored, timestamp);
            }
            absent
        })?;
        if seeded {
            self.entities.regions.lock().expect("region cache").dirty.insert(region);
        }
        Ok(seeded)
    }

    /// A chunk's saved entities (`EntityStorage.loadEntities`): `None` when
    /// the storage has no entry for it yet.
    pub fn load_entities(&self, pos: ChunkPos) -> Result<Option<Vec<Tag>>, String> {
        let (region, index) = region_of(pos);
        let tag = self.with_entity_region(region, |file| file.stored(index))?.map(|stored| stored.parse()).transpose()?;
        Ok(tag.map(|t| t.get("Entities").and_then(Tag::as_list).map(<[Tag]>::to_vec).unwrap_or_default()))
    }

    /// Queues a chunk's entities for the next `flush` (`EntityStorage.storeEntities`).
    pub fn save_entities(&self, pos: ChunkPos, entities: &[Tag]) -> Result<(), String> {
        let (region, index) = region_of(pos);
        let stored = StoredChunk::encode(&write_entities(pos, entities));
        store(&self.entities.dir, &self.entities.regions, region, index, stored)
    }

    /// Writes every region with unsaved chunks: each is copied (its chunks
    /// are shared) under the region cache and written outside it.
    pub fn flush(&self) -> Result<(), String> {
        let _writing = self.writing.lock().expect("region writer");
        for (dir, regions) in [(&self.dir, &self.regions), (&self.entities.dir, &self.entities.regions)] {
            let snapshots: Vec<((i32, i32), RegionFile)> = {
                let mut regions = regions.lock().expect("region cache");
                let dirty: Vec<(i32, i32)> = regions.dirty.drain().collect();
                dirty.into_iter().filter_map(|region| regions.loaded.get(&region).map(|file| (region, file.clone()))).collect()
            };
            for (region, file) in snapshots {
                file.write(&region_path(dir, region))?;
            }
        }
        Ok(())
    }
}

/// Sets a stored chunk and marks its region for the next flush.
fn store(dir: &Path, regions: &Mutex<Regions>, region: (i32, i32), index: usize, stored: StoredChunk) -> Result<(), String> {
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as u32);
    with_region(dir, regions, region, |file| file.set_stored(index, stored, timestamp))?;
    regions.lock().expect("region cache").dirty.insert(region);
    Ok(())
}

fn with_region<T>(dir: &Path, regions: &Mutex<Regions>, region: (i32, i32), f: impl FnOnce(&mut RegionFile) -> T) -> Result<T, String> {
    let mut regions = regions.lock().expect("region cache");
    if !regions.loaded.contains_key(&region) {
        let path = region_path(dir, region);
        let file = if path.exists() { RegionFile::read(&path)? } else { RegionFile::new() };
        regions.loaded.insert(region, file);
    }
    Ok(f(regions.loaded.get_mut(&region).expect("inserted")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_core::{BiomeId, BlockStateId};

    pub(crate) struct TestDir(pub(crate) PathBuf);

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn test_dir(name: &str) -> TestDir {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!("minecraftoss-storage-{name}-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TestDir(dir)
    }

    pub(crate) fn synthetic_registries(dir: &Path) -> Arc<Registries> {
        let state = |id: u32, block: &str, air: bool| {
            serde_json::json!({"id": id, "block": block, "block_id": id, "default": true, "properties": [],
                "air": air, "liquid": false, "can_occlude": !air, "solid_render": !air, "legacy_solid": !air,
                "replaceable": air, "light_emission": 0, "light_dampening": if air { 0 } else { 15 },
                "propagates_skylight_down": air, "use_shape_for_light_occlusion": false, "has_block_entity": false,
                "randomly_ticking": false, "render_shape": if air { "INVISIBLE" } else { "MODEL" }, "has_offset": false,
                "face_occlusion": if air { [0, 0, 0, 0, 0, 0] } else { [1, 1, 1, 1, 1, 1] }})
        };
        let catalog = serde_json::json!({"schema_version": 2, "minecraft_version": "26.3",
            "face_order": ["down", "up", "north", "south", "west", "east"], "shapes": ["empty", "full"],
            "states": [state(0, "minecraft:air", true), state(1, "minecraft:stone", false)]});
        let pack = dir.join("datapack");
        let write = |path: PathBuf, value: serde_json::Value| {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, value.to_string()).unwrap();
        };
        write(dir.join("catalog.json"), catalog);
        write(pack.join("data/minecraft/worldgen/biome/plains.json"), serde_json::json!({"temperature": 0.8, "downfall": 0.4, "has_precipitation": true}));
        for tag in ["blocks_motion_in_heightmap", "blocks_motion_in_heightmap_no_leaves"] {
            write(pack.join(format!("data/minecraft/tags/block/{tag}.json")), serde_json::json!({"values": ["minecraft:stone"]}));
        }
        let missing = dir.join("missing.json");
        let paths = DataPaths { block_catalog: dir.join("catalog.json"), datapack: pack, block_entity_catalog: missing.clone(), entity_catalog: missing.clone(), item_catalog: missing };
        Arc::new(Registries::load(&paths).unwrap())
    }

    pub(crate) const STONE: BlockStateId = BlockStateId(1);

    pub(crate) fn full_chunk(pos: ChunkPos, registries: &Registries, column: (usize, usize), height: i32, entities: Vec<Tag>) -> Chunk {
        let mut chunk = Chunk::new(pos, 0, 32, BiomeId(0));
        for y in 0..height {
            chunk.set_block_raw(column.0, y, column.1, STONE, registries);
        }
        chunk.status = ChunkStatus::Full;
        chunk.generation.entities = entities;
        chunk
    }

    pub(crate) fn entity(id: &str) -> Tag {
        Tag::Compound([("id".to_owned(), Tag::String(id.to_owned()))].into_iter().collect())
    }

    pub(crate) fn ids(entities: &[Tag]) -> Vec<&str> {
        entities.iter().filter_map(|t| t.get("id").and_then(Tag::as_str)).collect()
    }

    #[test]
    fn a_full_chunk_and_its_generated_entities_come_back_from_a_new_storage() {
        let dir = test_dir("reload");
        let registries = synthetic_registries(&dir.0);
        let world = dir.0.join("world");
        let storage = ChunkStorage::new(&world, "minecraft:overworld", registries.clone(), 0, 32);
        let pos = ChunkPos::new(-33, 70);
        storage.save(&full_chunk(pos, &registries, (3, 9), 5, vec![entity("minecraft:cow")])).unwrap();
        storage.flush().unwrap();
        assert!(world.join("dimensions/minecraft/overworld/region/r.-2.2.mca").is_file());
        assert!(world.join("dimensions/minecraft/overworld/entities/r.-2.2.mca").is_file());
        let reopened = ChunkStorage::new(&world, "minecraft:overworld", registries.clone(), 0, 32);
        let chunk = reopened.load(pos).unwrap().expect("saved");
        assert_eq!(chunk.status, ChunkStatus::Full);
        assert_eq!(chunk.block(3, 4, 9), STONE);
        assert!(registries.blocks.is_air(chunk.block(3, 5, 9)));
        assert_eq!(ids(&chunk.generation.entities), ["minecraft:cow"]);
        assert_eq!(reopened.load(ChunkPos::new(0, 0)).unwrap().map(|c| c.pos), None);
    }

    #[test]
    fn saving_a_full_chunk_never_overwrites_entities_the_level_saved_first() {
        let dir = test_dir("level-first");
        let registries = synthetic_registries(&dir.0);
        let storage = ChunkStorage::new(&dir.0, "minecraft:overworld", registries.clone(), 0, 32);
        let pos = ChunkPos::new(4, -2);
        storage.save_entities(pos, &[entity("minecraft:item")]).unwrap();
        storage.save(&full_chunk(pos, &registries, (0, 0), 1, vec![entity("minecraft:cow"), entity("minecraft:pig")])).unwrap();
        storage.flush().unwrap();
        let reopened = ChunkStorage::new(&dir.0, "minecraft:overworld", registries, 0, 32);
        assert_eq!(ids(&reopened.load_entities(pos).unwrap().unwrap()), ["minecraft:item"]);
    }

    #[test]
    fn the_level_saving_after_the_chunk_replaces_the_generated_entities() {
        let dir = test_dir("chunk-first");
        let registries = synthetic_registries(&dir.0);
        let storage = ChunkStorage::new(&dir.0, "minecraft:overworld", registries.clone(), 0, 32);
        let pos = ChunkPos::new(4, -2);
        storage.save(&full_chunk(pos, &registries, (0, 0), 1, vec![entity("minecraft:cow")])).unwrap();
        assert_eq!(ids(&storage.load_entities(pos).unwrap().unwrap()), ["minecraft:cow"]);
        storage.save_entities(pos, &[]).unwrap();
        storage.save(&full_chunk(pos, &registries, (0, 0), 2, vec![entity("minecraft:cow")])).unwrap();
        storage.flush().unwrap();
        let reopened = ChunkStorage::new(&dir.0, "minecraft:overworld", registries, 0, 32);
        assert_eq!(reopened.load_entities(pos).unwrap(), Some(Vec::new()));
        assert_eq!(reopened.load(pos).unwrap().unwrap().block(0, 1, 0), STONE);
    }

    #[test]
    fn seeding_only_fills_an_empty_entry_however_the_threads_interleave() {
        let dir = test_dir("race");
        let registries = synthetic_registries(&dir.0);
        let storage = Arc::new(ChunkStorage::new(&dir.0, "minecraft:overworld", registries, 0, 32));
        for round in 0..50 {
            let pos = ChunkPos::new(round, 0);
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let seeding = {
                let (storage, barrier) = (storage.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    storage.seed_entities(pos, &[entity("minecraft:cow")]).unwrap()
                })
            };
            barrier.wait();
            storage.save_entities(pos, &[entity("minecraft:item")]).unwrap();
            let seeded = seeding.join().unwrap();
            let saved = storage.load_entities(pos).unwrap().unwrap();
            assert_eq!(ids(&saved), ["minecraft:item"], "round {round}, seeded first: {seeded}");
            assert!(!storage.seed_entities(pos, &[entity("minecraft:pig")]).unwrap());
        }
    }
}
