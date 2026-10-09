use std::collections::HashMap;
use std::sync::Arc;

use minecraft_net::Dimension;
use minecraft_terrain::terrain::TerrainStream;
use minecraftoss_core::palette::PalettedContainer;
use minecraftoss_core::{BlockId, BlockStateId, Chunk, ChunkPos, Registries};

use crate::portal::{NETHER_PORTAL, PortalAxis, within_border};

pub const NETHER_SEARCH_RADIUS: i32 = 16;
pub const OVERWORLD_SEARCH_RADIUS: i32 = 128;

pub trait PortalWorld {
    fn portal_axis(&mut self, pos: (i32, i32, i32)) -> Option<PortalAxis>;
    fn portals_in_chunk(&mut self, chunk: (i32, i32)) -> Vec<(i32, i32, i32)>;
}

pub fn search_radius(to: Dimension) -> i32 {
    if to == Dimension::Nether {
        NETHER_SEARCH_RADIUS
    } else {
        OVERWORLD_SEARCH_RADIUS
    }
}

fn dist_sqr(a: (i32, i32, i32), b: (i32, i32, i32)) -> f64 {
    let d = |a: i32, b: i32| f64::from(a) - f64::from(b);
    d(a.0, b.0).powi(2) + d(a.1, b.1).powi(2) + d(a.2, b.2).powi(2)
}

pub fn find_closest(
    world: &mut impl PortalWorld,
    target: (i32, i32, i32),
    radius: i32,
) -> Option<(i32, i32, i32)> {
    let chunks = |centre: i32| (centre - radius) >> 4..=(centre + radius) >> 4;
    let mut best: Option<((i32, i32, i32), f64)> = None;
    for cz in chunks(target.2) {
        for cx in chunks(target.0) {
            for pos in world.portals_in_chunk((cx, cz)) {
                if (pos.0 - target.0).abs() > radius
                    || (pos.2 - target.2).abs() > radius
                    || !within_border(f64::from(pos.0), f64::from(pos.2))
                    || world.portal_axis(pos).is_none()
                {
                    continue;
                }
                let distance = dist_sqr(pos, target);
                let closer = best
                    .is_none_or(|(at, best)| distance < best || (distance == best && pos.1 < at.1));
                if closer {
                    best = Some((pos, distance));
                }
            }
        }
    }
    best.map(|(pos, _)| pos)
}

pub struct ChunkWorld<'a> {
    stream: &'a mut TerrainStream,
    registries: Arc<Registries>,
    portal: Option<BlockId>,
    chunks: HashMap<(i32, i32), Option<Arc<Chunk>>>,
    pub generates: bool,
}

impl<'a> ChunkWorld<'a> {
    pub fn new(stream: &'a mut TerrainStream) -> Self {
        let registries = stream.states.registries().clone();
        let portal = registries.blocks.block_by_name(NETHER_PORTAL);
        Self {
            stream,
            registries,
            portal,
            chunks: HashMap::new(),
            generates: false,
        }
    }

    pub fn registries(&self) -> &Arc<Registries> {
        &self.registries
    }

    pub fn chunk(&mut self, (cx, cz): (i32, i32)) -> Option<Arc<Chunk>> {
        if let Some(Some(chunk)) = self.chunks.get(&(cx, cz)) {
            return Some(chunk.clone());
        }
        if !self.generates && self.chunks.contains_key(&(cx, cz)) {
            return None;
        }
        let pos = ChunkPos::new(cx, cz);
        let chunk = self
            .stream
            .loaded_chunk(pos)
            .or_else(|| {
                self.stream
                    .storage()
                    .and_then(|storage| storage.load(pos).ok().flatten())
                    .filter(|chunk| chunk.status == minecraftoss_core::chunk::ChunkStatus::Full)
                    .map(Arc::new)
            })
            .or_else(|| self.generates.then(|| self.stream.load_chunk_now(pos)));
        self.chunks.insert((cx, cz), chunk.clone());
        chunk
    }

    pub fn forget(&mut self, chunk: (i32, i32)) {
        self.chunks.remove(&chunk);
    }

    pub fn state(&mut self, (x, y, z): (i32, i32, i32)) -> BlockStateId {
        self.chunk((x >> 4, z >> 4))
            .map_or(BlockStateId::AIR, |chunk| {
                chunk.block((x & 15) as usize, y, (z & 15) as usize)
            })
    }

    fn is_portal(&self, state: BlockStateId) -> bool {
        self.portal == Some(self.registries.blocks.block_of(state))
    }
}

impl PortalWorld for ChunkWorld<'_> {
    fn portal_axis(&mut self, pos: (i32, i32, i32)) -> Option<PortalAxis> {
        let state = self.state(pos);
        if !self.is_portal(state) {
            return None;
        }
        self.registries
            .blocks
            .property(state, "axis")
            .and_then(PortalAxis::named)
    }

    fn portals_in_chunk(&mut self, (cx, cz): (i32, i32)) -> Vec<(i32, i32, i32)> {
        let Some(chunk) = self.chunk((cx, cz)) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for (index, section) in chunk.sections().iter().enumerate() {
            if section.is_empty() {
                continue;
            }
            let base_y = (chunk.min_section_y() + index as i32) * 16;
            let mut take = |at: usize| {
                found.push((
                    cx * 16 + (at & 15) as i32,
                    base_y + (at >> 8) as i32,
                    cz * 16 + ((at >> 4) & 15) as i32,
                ));
            };
            match &section.blocks {
                PalettedContainer::Single(state) => {
                    if self.is_portal(*state) {
                        (0..16 * 16 * 16).for_each(&mut take);
                    }
                }
                PalettedContainer::Direct(states) => {
                    for (at, state) in states.iter().enumerate() {
                        if self.is_portal(*state) {
                            take(at);
                        }
                    }
                }
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeWorld {
        portals: HashMap<(i32, i32, i32), Option<PortalAxis>>,
        read: std::collections::BTreeSet<(i32, i32)>,
    }

    impl PortalWorld for FakeWorld {
        fn portal_axis(&mut self, pos: (i32, i32, i32)) -> Option<PortalAxis> {
            self.portals.get(&pos).copied().flatten()
        }

        fn portals_in_chunk(&mut self, chunk: (i32, i32)) -> Vec<(i32, i32, i32)> {
            self.read.insert(chunk);
            let mut found: Vec<_> = self
                .portals
                .keys()
                .copied()
                .filter(|pos| (pos.0 >> 4, pos.2 >> 4) == chunk)
                .collect();
            found.sort_unstable();
            found
        }
    }

    fn portal(world: &mut FakeWorld, min: (i32, i32, i32), axis: PortalAxis) {
        for along in 0..2 {
            for up in 0..3 {
                let pos = match axis {
                    PortalAxis::X => (min.0 + along, min.1 + up, min.2),
                    PortalAxis::Z => (min.0, min.1 + up, min.2 + along),
                };
                world.portals.insert(pos, Some(axis));
            }
        }
    }

    #[test]
    fn the_search_looks_sixteen_blocks_into_the_nether_and_a_hundred_and_twenty_eight_out() {
        assert_eq!(search_radius(Dimension::Nether), 16);
        assert_eq!(search_radius(Dimension::Overworld), 128);
        assert_eq!(search_radius(Dimension::End), 128);
        let mut world = FakeWorld::default();
        portal(&mut world, (116, 40, -200), PortalAxis::X);
        assert_eq!(
            find_closest(&mut world, (100, 70, -200), 16),
            Some((116, 42, -200))
        );
        assert_eq!(
            find_closest(&mut world, (99, 70, -200), 16),
            None,
            "17 blocks away"
        );
        assert_eq!(
            find_closest(&mut world, (99, 70, -200), 128),
            Some((116, 42, -200))
        );
        let expected: std::collections::BTreeSet<_> = (-14..=-12)
            .flat_map(|cz| (5..=7).map(move |cx| (cx, cz)))
            .collect();
        let mut nether = FakeWorld::default();
        find_closest(&mut nether, (100, 70, -200), 16);
        assert_eq!(
            nether.read, expected,
            "only the chunks under the square are read"
        );
        let mut overworld = FakeWorld::default();
        find_closest(&mut overworld, (800, 70, -1600), 128);
        assert_eq!(overworld.read.len(), 17 * 17);
    }

    #[test]
    fn the_closest_portal_block_wins_and_a_tie_goes_to_the_lowest() {
        let mut world = FakeWorld::default();
        portal(&mut world, (10, 64, 0), PortalAxis::X);
        portal(&mut world, (-12, 64, 0), PortalAxis::Z);
        assert_eq!(find_closest(&mut world, (0, 64, 0), 128), Some((10, 64, 0)));
        assert_eq!(find_closest(&mut world, (0, 66, 0), 128), Some((10, 66, 0)));
        let mut tie = FakeWorld::default();
        tie.portals.insert((0, 70, 3), Some(PortalAxis::X));
        tie.portals.insert((0, 70, -3), Some(PortalAxis::X));
        tie.portals.insert((0, 64, 0), Some(PortalAxis::X));
        tie.portals.insert((0, 76, 0), Some(PortalAxis::X));
        assert_eq!(find_closest(&mut tie, (0, 70, 0), 16), Some((0, 70, -3)));
        tie.portals.insert((0, 67, 0), Some(PortalAxis::X));
        tie.portals.insert((0, 73, 0), Some(PortalAxis::X));
        assert_eq!(find_closest(&mut tie, (0, 70, 0), 16), Some((0, 67, 0)));
    }

    #[test]
    fn broken_portals_and_portals_past_the_border_are_skipped() {
        let mut world = FakeWorld::default();
        world.portals.insert((2, 64, 0), None);
        portal(&mut world, (20, 64, 0), PortalAxis::X);
        assert_eq!(find_closest(&mut world, (0, 64, 0), 128), Some((20, 64, 0)));
        let mut edge = FakeWorld::default();
        portal(&mut edge, (29_999_983, 64, 0), PortalAxis::X);
        assert_eq!(
            find_closest(&mut edge, (29_999_983, 64, 0), 16),
            Some((29_999_983, 64, 0)),
            "the block at 29,999,984 is past the border"
        );
        let mut past = FakeWorld::default();
        portal(&mut past, (29_999_984, 64, 0), PortalAxis::X);
        assert_eq!(find_closest(&mut past, (29_999_983, 64, 0), 16), None);
    }

    #[test]
    fn a_level_s_chunks_are_searched_as_loaded_or_saved_and_never_generated() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let mut stream = TerrainStream::for_dimension_at(
            Arc::new(registries),
            77,
            2,
            Dimension::Nether,
            None,
            Some((0, 64, 0)),
        )
        .expect("a Nether stream");
        let far = ChunkPos::new(6, 6);
        assert_eq!(
            find_closest(&mut ChunkWorld::new(&mut stream), (100, 64, 100), 16),
            None
        );
        assert!(
            stream.loaded_chunk(far).is_none(),
            "the search generated a chunk"
        );
        stream.load_chunk_now(ChunkPos::new(0, 0));
        let mut scene = minecraft_terrain::scene::HandcraftedScene::streamed(stream.states.clone());
        let block = minecraft_terrain::scene::Block::new(NETHER_PORTAL).with("axis", "z");
        let lit = [(1, 64, 1), (1, 65, 1)];
        for pos in lit {
            scene.set(pos, Some(block.clone()));
        }
        stream.record_edits(&scene, &lit);
        let mut world = ChunkWorld::new(&mut stream);
        assert_eq!(world.portals_in_chunk((0, 0)), lit.to_vec());
        assert_eq!(world.portal_axis((1, 65, 1)), Some(PortalAxis::Z));
        assert_eq!(world.portal_axis((1, 66, 1)), None);
        assert_eq!(find_closest(&mut world, (0, 70, 0), 16), Some((1, 65, 1)));
    }
}
