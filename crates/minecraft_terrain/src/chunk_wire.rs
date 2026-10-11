use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use minecraft_net::ChunkBatchBlob;
use minecraft_net::messages::ChunkBlob;
use minecraftoss_core::anvil::StoredChunk;
use minecraftoss_core::chunk_nbt::{read_chunk, write_chunk};
use minecraftoss_core::{Chunk, Registries};

use crate::terrain::BlockStates;

pub fn encode_chunk(chunk: &Chunk, registries: &Registries) -> ChunkBlob {
    let stored = StoredChunk::encode(&write_chunk(chunk, registries, 0));
    ChunkBlob(stored.zlib().expect("chunks are stored as zlib").to_vec())
}

pub fn decode_chunk(pos: [i32; 2], blob: &ChunkBlob, states: &BlockStates) -> Result<Chunk> {
    let tag = StoredChunk::from_zlib(&blob.0)
        .parse()
        .map_err(|error| anyhow!("chunk {pos:?}: {error}"))?;
    let range = states.vertical_range();
    let chunk = read_chunk(
        &tag,
        states.registries(),
        range.start,
        range.end - range.start,
    )
    .map_err(|error| anyhow!("chunk {pos:?}: {error}"))?;
    if [chunk.pos.x, chunk.pos.z] != pos {
        bail!("chunk {pos:?} holds chunk {:?}", [chunk.pos.x, chunk.pos.z]);
    }
    Ok(chunk)
}

pub fn encode_batch(seq: u32, chunks: &[Arc<Chunk>], registries: &Registries) -> Result<Vec<u8>> {
    ChunkBatchBlob {
        seq,
        chunks: chunks
            .iter()
            .map(|chunk| ([chunk.pos.x, chunk.pos.z], encode_chunk(chunk, registries)))
            .collect(),
    }
    .encode()
    .map_err(|error| anyhow!("chunk batch {seq}: {error}"))
}

pub fn decode_batch(bytes: &[u8], states: &BlockStates) -> Result<(u32, Vec<Arc<Chunk>>)> {
    let batch = ChunkBatchBlob::decode(bytes).map_err(|error| anyhow!("chunk batch: {error}"))?;
    let chunks = batch
        .chunks
        .iter()
        .map(|(pos, blob)| decode_chunk(*pos, blob, states).map(Arc::new))
        .collect::<Result<Vec<_>>>()
        .map_err(|error| anyhow!("chunk batch {}: {error:#}", batch.seq))?;
    Ok((batch.seq, chunks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::chunk::ChunkStatus;
    use minecraftoss_core::nbt::Tag;
    use minecraftoss_core::{BiomeId, ChunkPos};

    fn states() -> Option<Arc<BlockStates>> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = Arc::new(Registries::load(&paths).ok()?);
        Some(Arc::new(BlockStates::new(registries, 7, -64, 384).ok()?))
    }

    fn chest(x: i32, y: i32, z: i32) -> Tag {
        let mut tag = std::collections::BTreeMap::new();
        tag.insert("id".to_owned(), Tag::String("minecraft:chest".to_owned()));
        tag.insert("x".to_owned(), Tag::Int(x));
        tag.insert("y".to_owned(), Tag::Int(y));
        tag.insert("z".to_owned(), Tag::Int(z));
        tag.insert("CustomName".to_owned(), Tag::String("Loot".to_owned()));
        Tag::Compound(tag)
    }

    fn sample_chunk(states: &BlockStates, pos: ChunkPos) -> Chunk {
        let registries = states.registries();
        let biome = |name: &str| registries.biomes.id(name).unwrap_or(BiomeId(0));
        let mut chunk = Chunk::new(pos, -64, 384, biome("minecraft:plains"));
        chunk.status = ChunkStatus::Full;
        let state = |text: &str| {
            registries
                .blocks
                .parse_state(text)
                .expect("a vanilla block state")
        };
        let palette = [
            state("minecraft:stone"),
            state("minecraft:grass_block[snowy=false]"),
            state("minecraft:water[level=0]"),
            state("minecraft:oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=false]"),
            state("minecraft:deepslate_diamond_ore"),
        ];
        for x in 0..16 {
            for z in 0..16 {
                for y in -64..64 {
                    let pick = (x * 7 + z * 3 + (y + 64) as usize) % (palette.len() + 3);
                    if let Some(&block) = palette.get(pick) {
                        chunk.set_block_raw(x, y, z, block, registries);
                    }
                }
            }
        }
        let chest_at = (pos.x * 16 + 3, 64, pos.z * 16 + 4);
        chunk.set_block_raw(
            3,
            64,
            4,
            state("minecraft:chest[facing=north,type=single,waterlogged=false]"),
            registries,
        );
        chunk
            .block_entities
            .entities
            .insert(chest_at, chest(chest_at.0, chest_at.1, chest_at.2));
        let biomes = [
            biome("minecraft:desert"),
            biome("minecraft:swamp"),
            biome("minecraft:deep_dark"),
        ];
        for (i, section) in chunk.sections_mut().iter_mut().enumerate() {
            for index in 0..64 {
                if (index + i) % 4 == 0 {
                    section
                        .biomes
                        .set(index, biomes[(index + i) % biomes.len()]);
                }
            }
        }
        chunk
    }

    fn assert_same(a: &Chunk, b: &Chunk) {
        assert_eq!(
            (a.pos, a.min_y(), a.height(), a.status),
            (b.pos, b.min_y(), b.height(), b.status)
        );
        for y in a.min_y()..a.min_y() + a.height() {
            for z in 0..16 {
                for x in 0..16 {
                    assert_eq!(a.block(x, y, z), b.block(x, y, z), "block {x} {y} {z}");
                }
            }
        }
        for qy in a.min_y() / 4..(a.min_y() + a.height()) / 4 {
            for qz in 0..4 {
                for qx in 0..4 {
                    assert_eq!(
                        a.biome(qx, qy, qz),
                        b.biome(qx, qy, qz),
                        "biome {qx} {qy} {qz}"
                    );
                }
            }
        }
        assert_eq!(
            a.block_entities.entities.keys().collect::<Vec<_>>(),
            b.block_entities.entities.keys().collect::<Vec<_>>()
        );
        for (pos, tag) in &a.block_entities.entities {
            let sent = &b.block_entities.entities[pos];
            for key in ["id", "x", "y", "z", "CustomName"] {
                assert_eq!(
                    tag.get(key),
                    sent.get(key),
                    "{key} of the block entity at {pos:?}"
                );
            }
        }
    }

    #[test]
    fn a_chunk_crosses_the_wire_with_its_blocks_biomes_and_block_entities() {
        let Some(states) = states() else { return };
        let chunk = sample_chunk(&states, ChunkPos::new(-3, 5));
        let blob = encode_chunk(&chunk, states.registries());
        assert!(blob.0.len() < 64 * 1024, "{} bytes", blob.0.len());
        let decoded = decode_chunk([-3, 5], &blob, &states).unwrap();
        assert_same(&chunk, &decoded);
        assert!(
            decode_chunk([-3, 6], &blob, &states).is_err(),
            "a chunk sent under the wrong position is refused"
        );
        assert!(
            decode_chunk(
                [-3, 5],
                &ChunkBlob(blob.0[..blob.0.len() / 2].to_vec()),
                &states
            )
            .is_err()
        );
    }

    #[test]
    fn a_batch_crosses_the_wire_in_order() {
        let Some(states) = states() else { return };
        let chunks: Vec<Arc<Chunk>> = [(0, 0), (1, 0), (0, -1)]
            .into_iter()
            .map(|(x, z)| Arc::new(sample_chunk(&states, ChunkPos::new(x, z))))
            .collect();
        let bytes = encode_batch(41, &chunks, states.registries()).unwrap();
        assert_eq!(ChunkBatchBlob::peek_seq(&bytes), Some(41));
        let (seq, decoded) = decode_batch(&bytes, &states).unwrap();
        assert_eq!(seq, 41);
        assert_eq!(decoded.len(), chunks.len());
        for (a, b) in chunks.iter().zip(&decoded) {
            assert_same(a, b);
        }
        let (_, empty) = decode_batch(
            &encode_batch(42, &[], states.registries()).unwrap(),
            &states,
        )
        .unwrap();
        assert!(empty.is_empty());
        assert!(decode_batch(&bytes[..bytes.len() - 1], &states).is_err());
    }
}
