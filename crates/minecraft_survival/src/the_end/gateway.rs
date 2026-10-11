use std::collections::HashSet;

use minecraftoss_core::BlockPos;
use minecraftoss_core::random::LegacyRandom;

use crate::portal_forcer::ChunkWorld;

pub type Pos = (i32, i32, i32);

pub const END_GATEWAY: &str = "minecraft:end_gateway";
pub const EXIT_ABOVE: i32 = 10;
const END_STONE: &str = "minecraft:end_stone";
const BEDROCK: &str = "minecraft:bedrock";
const SEARCH_DISTANCE: f64 = 1024.0;
const SEARCH_STEP: f64 = 16.0;
const SEARCH_STEPS: i32 = 16;
const SPAWN_LOWEST: i32 = 30;
const ISLAND_Y: f64 = 75.0;
const TALLEST_RADIUS: i32 = 16;
const EXIT_RADIUS: i32 = 5;
const NORMALIZE_EPSILON: f64 = 1.0e-5_f32 as f64;

pub trait GatewayWorld {
    fn highest_filled_section(&mut self, chunk: (i32, i32)) -> Option<i32>;
    fn is_end_stone(&mut self, pos: Pos) -> bool;
    fn full_block(&mut self, pos: Pos) -> bool;
    fn is_bedrock(&mut self, pos: Pos) -> bool;
    fn min_y(&self) -> i32;
    fn max_y(&self) -> i32;

    fn chunk_is_empty(&mut self, chunk: (i32, i32)) -> bool {
        self.highest_filled_section(chunk).is_none()
    }

    fn highest_section_position(&mut self, chunk: (i32, i32)) -> i32 {
        self.highest_filled_section(chunk)
            .unwrap_or_else(|| self.min_y())
    }
}

impl ChunkWorld<'_> {
    fn is_block(&mut self, pos: Pos, name: &str) -> bool {
        let state = self.state(pos);
        let blocks = &self.registries().blocks;
        blocks.block_by_name(name) == Some(blocks.block_of(state))
    }
}

impl GatewayWorld for ChunkWorld<'_> {
    fn highest_filled_section(&mut self, chunk: (i32, i32)) -> Option<i32> {
        let chunk = self.chunk(chunk)?;
        let index = chunk
            .sections()
            .iter()
            .rposition(|section| !section.is_empty())?;
        Some(chunk.min_y() + index as i32 * 16)
    }

    fn is_end_stone(&mut self, pos: Pos) -> bool {
        self.is_block(pos, END_STONE)
    }

    fn full_block(&mut self, pos: Pos) -> bool {
        let state = self.state(pos);
        self.registries().blocks.collision_boxes(state) == [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
    }

    fn is_bedrock(&mut self, pos: Pos) -> bool {
        self.is_block(pos, BEDROCK)
    }

    fn min_y(&self) -> i32 {
        self.vertical_range().start
    }

    fn max_y(&self) -> i32 {
        self.vertical_range().end - 1
    }
}

struct WithIsland<'w, W: GatewayWorld> {
    world: &'w mut W,
    island: HashSet<Pos>,
}

impl<W: GatewayWorld> GatewayWorld for WithIsland<'_, W> {
    fn highest_filled_section(&mut self, chunk: (i32, i32)) -> Option<i32> {
        self.world.highest_filled_section(chunk)
    }

    fn is_end_stone(&mut self, pos: Pos) -> bool {
        self.island.contains(&pos) || self.world.is_end_stone(pos)
    }

    fn full_block(&mut self, pos: Pos) -> bool {
        self.island.contains(&pos) || self.world.full_block(pos)
    }

    fn is_bedrock(&mut self, pos: Pos) -> bool {
        !self.island.contains(&pos) && self.world.is_bedrock(pos)
    }

    fn min_y(&self) -> i32 {
        self.world.min_y()
    }

    fn max_y(&self) -> i32 {
        self.world.max_y()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GatewayExit {
    pub exit: Pos,
    pub exact: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    Gateway,
    Bedrock,
    Air,
}

pub fn chunk_at(point: [f64; 3]) -> (i32, i32) {
    (
        (point[0] / 16.0).floor() as i32,
        (point[2] / 16.0).floor() as i32,
    )
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length < NORMALIZE_EPSILON {
        [0.0; 3]
    } else {
        [v[0] / length, v[1] / length, v[2] / length]
    }
}

fn countdown(left: &mut i32) -> bool {
    let go = *left > 0;
    *left -= 1;
    go
}

pub fn exit_xz_tentative(world: &mut impl GatewayWorld, gateway: Pos) -> [f64; 3] {
    let dir = normalize([f64::from(gateway.0), 0.0, f64::from(gateway.2)]);
    let scaled = |by: f64| [dir[0] * by, dir[1] * by, dir[2] * by];
    let add = |p: [f64; 3], d: [f64; 3]| [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
    let mut point = scaled(SEARCH_DISTANCE);
    let mut left = SEARCH_STEPS;
    while !world.chunk_is_empty(chunk_at(point)) && countdown(&mut left) {
        point = add(point, scaled(-SEARCH_STEP));
    }
    left = SEARCH_STEPS;
    while world.chunk_is_empty(chunk_at(point)) && countdown(&mut left) {
        point = add(point, scaled(SEARCH_STEP));
    }
    point
}

fn dist_to_center_sqr((x, y, z): Pos) -> f64 {
    let (dx, dy, dz) = (f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5);
    dx * dx + dy * dy + dz * dz
}

pub fn valid_spawn_in_chunk(world: &mut impl GatewayWorld, (cx, cz): (i32, i32)) -> Option<Pos> {
    let top = world.highest_section_position((cx, cz)) + 15;
    let (low, high) = (SPAWN_LOWEST.min(top), SPAWN_LOWEST.max(top));
    let mut best: Option<(Pos, f64)> = None;
    for z in cz * 16..cz * 16 + 16 {
        for y in low..=high {
            for x in cx * 16..cx * 16 + 16 {
                let pos = (x, y, z);
                if world.is_end_stone(pos)
                    && !world.full_block((x, y + 1, z))
                    && !world.full_block((x, y + 2, z))
                {
                    let distance = dist_to_center_sqr(pos);
                    if best.is_none_or(|(_, nearest)| distance < nearest) {
                        best = Some((pos, distance));
                    }
                }
            }
        }
    }
    best.map(|(pos, _)| pos)
}

pub fn tallest_block(
    world: &mut impl GatewayWorld,
    center: Pos,
    radius: i32,
    allow_bedrock: bool,
) -> Pos {
    let mut best: Option<Pos> = None;
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            if dx == 0 && dz == 0 && !allow_bedrock {
                continue;
            }
            let mut y = world.max_y();
            while y > best.map_or_else(|| world.min_y(), |best| best.1) {
                let pos = (center.0 + dx, y, center.2 + dz);
                if world.full_block(pos) && (allow_bedrock || !world.is_bedrock(pos)) {
                    best = Some(pos);
                    break;
                }
                y -= 1;
            }
        }
    }
    best.unwrap_or(center)
}

pub fn find_exit_position(world: &mut impl GatewayWorld, exit: Pos) -> Pos {
    let (x, y, z) = tallest_block(world, (exit.0, exit.1 + 2, exit.2), EXIT_RADIUS, false);
    (x, y + 1, z)
}

pub fn find_or_create_teleport_pos(
    world: &mut impl GatewayWorld,
    gateway: Pos,
) -> (Pos, Option<Pos>) {
    let point = exit_xz_tentative(world, gateway);
    if let Some(spot) = valid_spawn_in_chunk(world, chunk_at(point)) {
        return (tallest_block(world, spot, TALLEST_RADIUS, true), None);
    }
    let origin = (
        (point[0] + 0.5).floor() as i32,
        ISLAND_Y.floor() as i32,
        (point[2] + 0.5).floor() as i32,
    );
    let mut with_island = WithIsland {
        world,
        island: end_island_blocks(origin).into_iter().collect(),
    };
    (
        tallest_block(&mut with_island, origin, TALLEST_RADIUS, true),
        Some(origin),
    )
}

pub fn new_exit(world: &mut impl GatewayWorld, gateway: Pos) -> (Pos, Option<Pos>) {
    let ((x, y, z), island) = find_or_create_teleport_pos(world, gateway);
    ((x, y + EXIT_ABOVE, z), island)
}

pub fn end_island_blocks(origin: Pos) -> Vec<Pos> {
    let mut random = LegacyRandom::new(BlockPos::new(origin.0, origin.1, origin.2).pack());
    let mut size = random.next_i32_bound(3) as f32 + 4.0;
    let mut y = 0;
    let mut blocks = Vec::new();
    while size > 0.5 {
        let (low, high) = ((-size).floor() as i32, size.ceil() as i32);
        for x in low..=high {
            for z in low..=high {
                if ((x * x + z * z) as f32) <= (size + 1.0) * (size + 1.0) {
                    blocks.push((origin.0 + x, origin.1 + y, origin.2 + z));
                }
            }
        }
        size -= random.next_i32_bound(2) as f32 + 0.5;
        y -= 1;
    }
    blocks
}

pub fn gateway_frame(origin: Pos) -> Vec<(Pos, Frame)> {
    let mut cells = Vec::with_capacity(45);
    for z in -1..=1 {
        for y in -2..=2 {
            for x in -1..=1 {
                let (same_x, same_y, same_z) = (x == 0, y == 0, z == 0);
                let end = i32::abs(y) == 2;
                let frame = if same_x && same_y && same_z {
                    Frame::Gateway
                } else if same_y {
                    Frame::Air
                } else if end && same_x && same_z || (same_x || same_z) && !end {
                    Frame::Bedrock
                } else {
                    Frame::Air
                };
                cells.push(((origin.0 + x, origin.1 + y, origin.2 + z), frame));
            }
        }
    }
    cells
}

pub fn destination(world: &mut impl GatewayWorld, exit: GatewayExit) -> [f64; 3] {
    let (x, y, z) = if exit.exact {
        exit.exit
    } else {
        find_exit_position(world, exit.exit)
    };
    [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5]
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::sync::Arc;

    use minecraft_net::Dimension;
    use minecraft_terrain::terrain::TerrainStream;
    use minecraftoss_core::Registries;

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Fake {
        EndStone,
        Stone,
        Bedrock,
        Slab,
    }

    #[derive(Default)]
    struct FakeWorld {
        blocks: HashMap<Pos, Fake>,
        filled: HashMap<(i32, i32), i32>,
        checked: Vec<(i32, i32)>,
    }

    impl FakeWorld {
        fn with(blocks: &[(Pos, Fake)]) -> Self {
            Self {
                blocks: blocks.iter().copied().collect(),
                ..Self::default()
            }
        }

        fn filled(chunks: impl IntoIterator<Item = (i32, i32)>) -> Self {
            Self {
                filled: chunks.into_iter().map(|chunk| (chunk, 0)).collect(),
                ..Self::default()
            }
        }
    }

    impl GatewayWorld for FakeWorld {
        fn highest_filled_section(&mut self, chunk: (i32, i32)) -> Option<i32> {
            self.checked.push(chunk);
            let from_blocks = self
                .blocks
                .keys()
                .filter(|pos| (pos.0 >> 4, pos.2 >> 4) == chunk)
                .map(|pos| pos.1 >> 4 << 4)
                .max();
            from_blocks.max(self.filled.get(&chunk).copied())
        }

        fn is_end_stone(&mut self, pos: Pos) -> bool {
            self.blocks.get(&pos) == Some(&Fake::EndStone)
        }

        fn full_block(&mut self, pos: Pos) -> bool {
            matches!(
                self.blocks.get(&pos),
                Some(Fake::EndStone | Fake::Stone | Fake::Bedrock)
            )
        }

        fn is_bedrock(&mut self, pos: Pos) -> bool {
            self.blocks.get(&pos) == Some(&Fake::Bedrock)
        }

        fn min_y(&self) -> i32 {
            0
        }

        fn max_y(&self) -> i32 {
            255
        }
    }

    fn close(got: [f64; 3], want: [f64; 3]) -> bool {
        got.iter().zip(want).all(|(a, b)| (a - b).abs() < 1.0e-9)
    }

    #[test]
    fn the_exit_search_starts_1024_out_along_the_gateway_s_direction() {
        let out = [-320.6093751268596, 0.0, 972.5151045514741];
        let mut world = FakeWorld::filled([chunk_at(out)]);
        assert_eq!(chunk_at(out), (-21, 60));
        let found = exit_xz_tentative(&mut world, (-30, 75, 91));
        assert!(close(found, out), "{found:?}");
        assert_eq!(world.checked, [(-21, 60), (-20, 59), (-20, 59), (-21, 60)]);
        let mut centre = FakeWorld::default();
        assert_eq!(exit_xz_tentative(&mut centre, (0, 75, 0)), [0.0; 3]);
    }

    #[test]
    fn a_full_chunk_steps_back_and_an_empty_one_steps_forward_sixteen_times_at_most() {
        let east = (100, 75, 0);
        let mut full = FakeWorld::filled((0..100).map(|x| (x, 0)));
        assert_eq!(exit_xz_tentative(&mut full, east), [768.0, 0.0, 0.0]);
        assert_eq!(full.checked.len(), 18);
        let mut empty = FakeWorld::default();
        assert_eq!(exit_xz_tentative(&mut empty, east), [1280.0, 0.0, 0.0]);
        assert_eq!(empty.checked.len(), 18);
        let mut island = FakeWorld::filled((60..=66).map(|x| (x, 0)));
        assert_eq!(exit_xz_tentative(&mut island, east), [960.0, 0.0, 0.0]);
        let mut beyond = FakeWorld::filled([(70, 0)]);
        assert_eq!(exit_xz_tentative(&mut beyond, east), [1120.0, 0.0, 0.0]);
    }

    #[test]
    fn the_spawn_spot_is_the_end_stone_nearest_the_origin_with_two_free_blocks_above() {
        let mut covered = FakeWorld::with(&[
            ((1030, 40, 5), Fake::EndStone),
            ((1030, 42, 5), Fake::Stone),
            ((1035, 40, 5), Fake::EndStone),
            ((1036, 40, 5), Fake::EndStone),
            ((1036, 41, 5), Fake::Slab),
        ]);
        assert_eq!(
            valid_spawn_in_chunk(&mut covered, (64, 0)),
            Some((1035, 40, 5))
        );
        let mut low = FakeWorld::with(&[
            ((1024, 60, 0), Fake::EndStone),
            ((1025, 31, 0), Fake::EndStone),
            ((1024, 29, 0), Fake::EndStone),
        ]);
        assert_eq!(valid_spawn_in_chunk(&mut low, (64, 0)), Some((1025, 31, 0)));
        let mut tie =
            FakeWorld::with(&[((3, 40, 5), Fake::EndStone), ((5, 40, 3), Fake::EndStone)]);
        assert_eq!(valid_spawn_in_chunk(&mut tie, (0, 0)), Some((5, 40, 3)));
        let mut shallow =
            FakeWorld::with(&[((2, 10, 2), Fake::EndStone), ((3, 15, 3), Fake::EndStone)]);
        assert_eq!(valid_spawn_in_chunk(&mut shallow, (0, 0)), Some((3, 15, 3)));
        let mut high = FakeWorld::with(&[((4, 200, 4), Fake::EndStone)]);
        assert_eq!(valid_spawn_in_chunk(&mut high, (0, 0)), Some((4, 200, 4)));
        let mut stone = FakeWorld::with(&[((4, 50, 4), Fake::Stone)]);
        assert_eq!(valid_spawn_in_chunk(&mut stone, (0, 0)), None);
    }

    #[test]
    fn no_spot_places_a_seeded_end_island_at_75() {
        let layers = |blocks: &[Pos]| {
            let mut layers = BTreeMap::new();
            for pos in blocks {
                *layers.entry(pos.1).or_insert(0) += 1;
            }
            layers.into_iter().collect::<Vec<_>>()
        };
        let island = end_island_blocks((-320, 75, 972));
        assert_eq!(island.len(), 365);
        assert_eq!(
            layers(&island),
            [(71, 9), (72, 37), (73, 77), (74, 97), (75, 145)]
        );
        assert_eq!(island.first(), Some(&(-326, 75, 969)));
        assert_eq!(island.last(), Some(&(-319, 71, 973)));
        let east = end_island_blocks((1000, 75, 0));
        assert_eq!(east.len(), 398);
        assert_eq!(
            layers(&east),
            [(70, 21), (71, 25), (72, 69), (73, 77), (74, 97), (75, 109)]
        );
        assert_eq!(east.first(), Some(&(995, 75, -3)));
        assert_eq!(east.last(), Some(&(1002, 70, 1)));
        let north = end_island_blocks((5, 75, -1030));
        assert_eq!(layers(&north), [(73, 25), (74, 37), (75, 77)]);
        let mut world = FakeWorld::with(&[((-336, 20, 961), Fake::Stone)]);
        let (spot, origin) = find_or_create_teleport_pos(&mut world, (-30, 75, 91));
        assert_eq!(origin, Some((-321, 75, 973)));
        let built = end_island_blocks((-321, 75, 973));
        assert_eq!(built.len(), 483);
        assert_eq!(built.first(), Some(&(-327, 75, 970)));
        assert_eq!(built.last(), Some(&(-320, 69, 974)));
        assert_eq!(spot, (-327, 75, 970));
        assert_eq!(new_exit(&mut world, (-30, 75, 91)).0, (-327, 85, 970));
    }

    #[test]
    fn the_tallest_block_skips_bedrock_and_the_middle_only_when_asked() {
        let mut world = FakeWorld::with(&[
            ((0, 90, 0), Fake::Stone),
            ((1, 85, 0), Fake::Bedrock),
            ((1, 70, 0), Fake::EndStone),
            ((-2, 70, 2), Fake::EndStone),
            ((2, 80, -2), Fake::Slab),
        ]);
        assert_eq!(tallest_block(&mut world, (0, 50, 0), 2, true), (0, 90, 0));
        assert_eq!(tallest_block(&mut world, (0, 50, 0), 2, false), (-2, 70, 2));
        assert_eq!(tallest_block(&mut world, (0, 50, 0), 0, false), (0, 50, 0));
        assert_eq!(tallest_block(&mut world, (3, 50, 0), 2, true), (1, 85, 0));
        assert_eq!(tallest_block(&mut world, (3, 50, 0), 2, false), (1, 70, 0));
        let mut empty = FakeWorld::default();
        assert_eq!(tallest_block(&mut empty, (7, 60, 7), 16, true), (7, 60, 7));
    }

    #[test]
    fn a_loose_exit_lands_on_the_tallest_block_within_five_or_three_above_the_gateway() {
        let exit = (0, 80, 0);
        let mut framed = FakeWorld::default();
        for (pos, frame) in gateway_frame(exit) {
            if frame == Frame::Bedrock {
                framed.blocks.insert(pos, Fake::Bedrock);
            }
        }
        assert_eq!(find_exit_position(&mut framed, exit), (0, 83, 0));
        framed.blocks.insert((2, 78, 1), Fake::EndStone);
        framed.blocks.insert((6, 79, 0), Fake::EndStone);
        framed.blocks.insert((0, 60, 0), Fake::EndStone);
        assert_eq!(find_exit_position(&mut framed, exit), (2, 79, 1));
        assert_eq!(
            destination(&mut framed, GatewayExit { exit, exact: false }),
            [2.5, 79.0, 1.5]
        );
        assert_eq!(
            destination(&mut framed, GatewayExit { exit, exact: true }),
            [0.5, 80.0, 0.5]
        );
    }

    #[test]
    fn the_gateway_frame_is_a_bedrock_plus_with_an_open_side_ring() {
        let frame = gateway_frame((10, 75, -4));
        assert_eq!(frame.len(), 45);
        assert_eq!(frame[0], ((9, 73, -5), Frame::Air));
        assert_eq!(frame[4], ((10, 74, -5), Frame::Bedrock));
        assert_eq!(frame[22], ((10, 75, -4), Frame::Gateway));
        let of = |kind: Frame| {
            frame
                .iter()
                .filter(|(_, frame)| *frame == kind)
                .map(|((x, y, z), _)| (x - 10, y - 75, z + 4))
                .collect::<Vec<_>>()
        };
        assert_eq!(of(Frame::Gateway), [(0, 0, 0)]);
        assert_eq!(
            of(Frame::Bedrock),
            [
                (0, -1, -1),
                (0, 1, -1),
                (0, -2, 0),
                (-1, -1, 0),
                (0, -1, 0),
                (1, -1, 0),
                (-1, 1, 0),
                (0, 1, 0),
                (1, 1, 0),
                (0, 2, 0),
                (0, -1, 1),
                (0, 1, 1),
            ]
        );
        assert!(
            of(Frame::Air)
                .iter()
                .all(|&(x, y, z)| y == 0 || (x != 0 && z != 0) || y.abs() == 2)
        );
    }

    #[test]
    fn a_ring_gateway_finds_an_outer_island() {
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
            Dimension::End,
            None,
            None,
        )
        .expect("an End stream");
        let gateway = minecraft_terrain::server::end_gateway_position(0);
        let started = std::time::Instant::now();
        let mut world = ChunkWorld::new(&mut stream);
        world.generates = true;
        let tentative = exit_xz_tentative(&mut world, gateway);
        let ((x, y, z), island) = find_or_create_teleport_pos(&mut world, gateway);
        let took = started.elapsed();
        eprintln!(
            "gateway {gateway:?}: tentative {tentative:?}, spot {:?}, island {island:?}, in {took:?}",
            (x, y, z)
        );
        assert!(
            f64::from(x).hypot(f64::from(z)) >= 768.0,
            "the spot is on the outer islands"
        );
        assert!(world.full_block((x, y, z)), "the spot is a full block");
        assert!(
            island.is_some() || world.is_end_stone((x, y, z)),
            "an outer island spot is end stone"
        );
        let exit = (x, y + EXIT_ABOVE, z);
        let loose = destination(&mut world, GatewayExit { exit, exact: false });
        assert!(loose[1] <= f64::from(exit.1 + 3), "{loose:?}");
    }
}
