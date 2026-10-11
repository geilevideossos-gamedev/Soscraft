use std::any::Any;
use std::collections::HashSet;

use minecraft_net::Dimension;
use minecraft_terrain::end_gateway::Gateway;
use minecraft_terrain::scene::Scene;
use minecraft_terrain::server::ServerSim;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::LegacyRandom;
use minecraftoss_core::{BlockPos, BlockStateId};

use crate::entities::Entities;
use crate::level::{Level, Terrain};
use crate::player::Player;
use crate::player_nbt::Place;
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
const PEARL_GRAVITY: f64 = 0.03;
const PEARL_INERTIA: f64 = 0.99;
const VOID_DEPTH: i32 = 64;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GatewayTrip {
    pub exit: Pos,
    pub feet: [f64; 3],
    pub built: bool,
}

pub fn is_end_gateway(terrain: &Terrain, pos: Pos) -> bool {
    Scene::block(&terrain.scene, pos).is_some_and(|block| block.id.key() == END_GATEWAY)
}

pub fn enter(entities: Option<&mut Entities>, pos: Pos) -> bool {
    let Some(entities) = entities else {
        return true;
    };
    if entities.gateway_cooling(pos) {
        return false;
    }
    entities.run_job(Box::new(
        move |sim: &mut ServerSim| -> Box<dyn Any + Send> { Box::new(sim.trigger_gateway(pos)) },
    ));
    entities.mark_cooling(pos);
    true
}

pub fn known_exit(level: &mut Level, gateway: Pos) -> (Option<Pos>, bool) {
    if let Some(view) = level
        .entities
        .as_ref()
        .and_then(|entities| entities.gateway(gateway))
    {
        return (view.exit, view.exact);
    }
    let chunk = ChunkWorld::new(&mut level.terrain.stream).chunk((gateway.0 >> 4, gateway.2 >> 4));
    chunk
        .and_then(|chunk| {
            let store = &chunk.block_entities;
            store
                .entities
                .get(&gateway)
                .or_else(|| store.pending.get(&gateway))
                .and_then(Gateway::from_tag)
        })
        .map_or((None, false), |known| (known.exit, known.exact))
}

pub fn gateway_tag(pos: Pos, exit: Pos) -> Tag {
    Tag::Compound(
        [
            ("id", Tag::String(END_GATEWAY.to_owned())),
            ("x", Tag::Int(pos.0)),
            ("y", Tag::Int(pos.1)),
            ("z", Tag::Int(pos.2)),
            ("Age", Tag::Long(0)),
            ("exit_portal", Tag::IntArray(vec![exit.0, exit.1, exit.2])),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect(),
    )
}

pub fn exit_states(level: &mut Level, gateway: Pos) -> Option<(Pos, Vec<(Pos, BlockStateId)>)> {
    let (exit, island) = {
        let mut world = ChunkWorld::new(&mut level.terrain.stream);
        world.generates = true;
        new_exit(&mut world, gateway)
    };
    let registries = level.terrain.stream.states.registries().clone();
    let state = |name: &str| registries.blocks.parse_state(name).ok();
    let (end_stone, bedrock, portal) = (state(END_STONE)?, state(BEDROCK)?, state(END_GATEWAY)?);
    let mut states: Vec<(Pos, BlockStateId)> = island
        .map(end_island_blocks)
        .unwrap_or_else(Vec::new)
        .into_iter()
        .map(|pos| (pos, end_stone))
        .collect();
    states.extend(gateway_frame(exit).into_iter().map(|(pos, frame)| {
        let state = match frame {
            Frame::Gateway => portal,
            Frame::Bedrock => bedrock,
            Frame::Air => BlockStateId::AIR,
        };
        (pos, state)
    }));
    Some((exit, states))
}

pub fn create_exit(level: &mut Level, gateway: Pos, exact: bool) -> Option<Pos> {
    let (exit, states) = exit_states(level, gateway)?;
    let terrain = &mut level.terrain;
    terrain.stream.write_states(
        &mut terrain.scene,
        &states,
        &[(exit, Some(gateway_tag(exit, gateway)))],
    );
    if let Some(entities) = level.entities.as_mut() {
        entities.run_job(Box::new(
            move |sim: &mut ServerSim| -> Box<dyn Any + Send> {
                for &(pos, state) in &states {
                    sim.command_place(pos, state, &[], true);
                }
                sim.set_gateway_exit(exit, gateway, false);
                sim.set_gateway_exit(gateway, exit, exact);
                Box::new(())
            },
        ));
    }
    Some(exit)
}

pub fn arrival(from: Option<Place>, dimension: Dimension, feet: [f64; 3]) -> Place {
    match from {
        Some(from) => Place {
            dimension,
            feet,
            on_ground: false,
            ..from
        },
        None => Place {
            on_ground: false,
            ..Place::standing(dimension, feet, 0.0, 0.0)
        },
    }
}

pub fn resolve_exit(level: &mut Level, gateway: Pos) -> Option<(GatewayExit, bool)> {
    let (known, exact) = known_exit(level, gateway);
    let (exit, built) = match known {
        Some(exit) => (exit, false),
        None if level.dimension == Dimension::End => (create_exit(level, gateway, exact)?, true),
        None => return None,
    };
    Some((GatewayExit { exit, exact }, built))
}

pub fn travel(level: &mut Level, player: &mut Player, gateway: Pos) -> Option<GatewayTrip> {
    let (found, built) = resolve_exit(level, gateway)?;
    let exit = found.exit;
    let feet = {
        let mut world = ChunkWorld::new(&mut level.terrain.stream);
        world.generates = true;
        destination(&mut world, found)
    };
    let from = player
        .place
        .filter(|place| place.dimension == player.dimension);
    player.arrive(arrival(from, level.dimension, feet));
    Some(GatewayTrip { exit, feet, built })
}

pub fn collision_spans(world: &mut ChunkWorld<'_>, pos: Pos, fx: f64, fz: f64) -> Vec<(f64, f64)> {
    let state = world.state(pos);
    world
        .registries()
        .blocks
        .collision_boxes(state)
        .into_iter()
        .filter(|b| (b[0]..=b[3]).contains(&fx) && (b[2]..=b[5]).contains(&fz))
        .map(|b| (f64::from(pos.1) + b[1], f64::from(pos.1) + b[4]))
        .collect()
}

pub fn pearl_drop(
    start: [f64; 3],
    min_y: i32,
    mut spans_at: impl FnMut(Pos, f64, f64) -> Vec<(f64, f64)>,
) -> Option<[f64; 3]> {
    let (cell_x, cell_z) = (start[0].floor() as i32, start[2].floor() as i32);
    let (fx, fz) = (start[0] - f64::from(cell_x), start[2] - f64::from(cell_z));
    let lowest = f64::from(min_y - VOID_DEPTH);
    let mut y = start[1];
    let mut velocity = 0.0;
    loop {
        velocity = (velocity - PEARL_GRAVITY) * PEARL_INERTIA;
        let next = y + velocity;
        let hit = (next.floor() as i32 - 1..=y.floor() as i32)
            .rev()
            .any(|cell_y| {
                spans_at((cell_x, cell_y, cell_z), fx, fz)
                    .into_iter()
                    .any(|(bottom, top)| top >= next && bottom <= y)
            });
        if hit {
            return Some([start[0], y, start[2]]);
        }
        if next < lowest {
            return None;
        }
        y = next;
    }
}

pub fn pearl_landing(level: &mut Level, gateway: Pos) -> Option<[f64; 3]> {
    let (exit, _) = resolve_exit(level, gateway)?;
    let mut world = ChunkWorld::new(&mut level.terrain.stream);
    world.generates = true;
    let start = destination(&mut world, exit);
    let min_y = world.min_y();
    pearl_drop(start, min_y, |pos, fx, fz| {
        collision_spans(&mut world, pos, fx, fz)
    })
}

pub fn pearls_through(level: &mut Level) -> Vec<(Pos, Option<[f64; 3]>)> {
    let gateways = match level.entities.as_mut() {
        Some(entities) => std::mem::take(&mut entities.pearl_gateways),
        None => Vec::new(),
    };
    let mut trips = Vec::new();
    for gateway in gateways {
        let landing = pearl_landing(level, gateway);
        if let (Some(landing), Some(entities)) = (landing, level.entities.as_mut()) {
            entities
                .pearl_landings
                .push(glam::DVec3::from_array(landing));
        }
        trips.push((gateway, landing));
    }
    trips
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

    struct TestDir(std::path::PathBuf);

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn end_level(save: Option<&crate::save::WorldSave>) -> Option<Level> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = Arc::new(Registries::load(&paths).ok()?);
        let terrain = Terrain::open(registries, 77, 2, Dimension::End, save, None).ok()?;
        Some(Level::new(Dimension::End, terrain))
    }

    fn state(level: &Level, name: &str) -> BlockStateId {
        level
            .terrain
            .stream
            .states
            .registries()
            .blocks
            .parse_state(name)
            .expect(name)
    }

    fn write(level: &mut Level, states: &[(Pos, BlockStateId)], tags: &[(Pos, Option<Tag>)]) {
        let terrain = &mut level.terrain;
        terrain
            .stream
            .write_states(&mut terrain.scene, states, tags);
    }

    fn traveller(feet: [f64; 3]) -> Player {
        let mut steve = crate::the_end::tests::player(Dimension::End);
        steve.place = Some(Place {
            motion: [0.4, 0.1, -0.2],
            on_ground: false,
            fall_distance: 3.0,
            ..Place::standing(Dimension::End, feet, 30.0, -12.0)
        });
        steve
    }

    fn below(feet: [f64; 3]) -> Pos {
        (
            feet[0].floor() as i32,
            feet[1].floor() as i32 - 1,
            feet[2].floor() as i32,
        )
    }

    #[test]
    fn a_first_trip_builds_one_return_gateway_pointing_back() {
        let Some(mut level) = end_level(None) else {
            return;
        };
        let data = crate::data::GameData::default();
        level.entities = Some(crate::entities::Entities::new(
            &level.terrain.stream,
            Dimension::End,
            77,
            &data,
            None,
            None,
        ));
        let source = minecraft_terrain::server::end_gateway_position(0);
        let portal = state(&level, END_GATEWAY);
        write(&mut level, &[(source, portal)], &[]);
        assert_eq!(known_exit(&mut level, source), (None, false));
        let mut steve = traveller([96.5, 75.0, 0.5]);
        let started = std::time::Instant::now();
        let trip = travel(&mut level, &mut steve, source).expect("a trip");
        eprintln!("first trip {trip:?} in {:?}", started.elapsed());
        assert!(trip.built);
        assert!(
            level
                .entities
                .as_ref()
                .is_some_and(|entities| entities.runs_jobs()),
            "the source's exit and the new blocks go to the server"
        );
        let exit = trip.exit;
        assert!(
            f64::from(exit.0).hypot(f64::from(exit.2)) >= 768.0,
            "{exit:?}"
        );
        let arrival = steve.arrival.expect("an arrival");
        assert_eq!(arrival.feet, trip.feet);
        assert_eq!(
            (
                arrival.dimension,
                arrival.yaw,
                arrival.pitch,
                arrival.motion
            ),
            (Dimension::End, 30.0, -12.0, [0.4, 0.1, -0.2])
        );
        assert!(!arrival.on_ground);
        let mut world = ChunkWorld::new(&mut level.terrain.stream);
        assert!(world.is_block(exit, END_GATEWAY));
        assert!(world.is_bedrock((exit.0, exit.1 + 2, exit.2)));
        assert!(world.is_bedrock((exit.0, exit.1 - 2, exit.2)));
        assert!(world.is_bedrock((exit.0 + 1, exit.1 + 1, exit.2)));
        assert!(
            !world.full_block((exit.0 + 1, exit.1, exit.2)),
            "the side ring is open"
        );
        let landing = below(trip.feet);
        assert!(
            world.full_block(landing) && !world.is_bedrock(landing),
            "{landing:?}"
        );
        assert!((landing.0 - exit.0).abs() <= 5 && (landing.2 - exit.2).abs() <= 5);
        let chunk = world
            .chunk((exit.0 >> 4, exit.2 >> 4))
            .expect("the exit chunk");
        let tag = chunk
            .block_entities
            .entities
            .get(&exit)
            .expect("the return gateway's block entity");
        assert_eq!(
            tag.get("exit_portal").and_then(Tag::as_ints),
            Some(vec![source.0, source.1, source.2])
        );
        assert_eq!(tag.get("ExactTeleport"), None);
        assert_eq!(tag.get("Age").and_then(Tag::as_i64), Some(0));
        drop(world);
        write(
            &mut level,
            &[],
            &[(source, Some(gateway_tag(source, exit)))],
        );
        let mut again = traveller([96.5, 75.0, 0.5]);
        let second = travel(&mut level, &mut again, source).expect("a second trip");
        assert_eq!(
            (second.exit, second.feet, second.built),
            (exit, trip.feet, false)
        );
        let back = travel(&mut level, &mut again, exit).expect("the way back");
        assert_eq!((back.exit, back.built), (source, false));
        let home = below(back.feet);
        assert!(
            (home.0 - source.0).abs() <= 5 && (home.2 - source.2).abs() <= 5,
            "{back:?}"
        );
    }

    #[test]
    fn an_exact_exit_lands_on_it_and_a_loose_one_on_the_tallest_block() {
        let Some(mut level) = end_level(None) else {
            return;
        };
        let gateway = (40, 70, 40);
        let portal = state(&level, END_GATEWAY);
        let mut exact = gateway_tag(gateway, (100, 50, 0));
        if let Tag::Compound(map) = &mut exact {
            map.insert("ExactTeleport".to_owned(), Tag::Byte(1));
        }
        write(&mut level, &[(gateway, portal)], &[(gateway, Some(exact))]);
        let mut steve = traveller([40.5, 70.0, 40.5]);
        let trip = travel(&mut level, &mut steve, gateway).expect("an exact trip");
        assert_eq!(
            (trip.exit, trip.feet, trip.built),
            ((100, 50, 0), [100.5, 50.0, 0.5], false)
        );
        write(
            &mut level,
            &[],
            &[(gateway, Some(gateway_tag(gateway, (20, 50, 0))))],
        );
        let loose = travel(&mut level, &mut steve, gateway).expect("a loose trip");
        let landing = below(loose.feet);
        let mut world = ChunkWorld::new(&mut level.terrain.stream);
        world.generates = true;
        assert!(
            world.full_block(landing) && !world.is_bedrock(landing),
            "{loose:?}"
        );
        assert!((landing.0 - 20).abs() <= 5 && landing.2.abs() <= 5);
        assert_ne!(
            (landing.0, landing.2),
            (20, 0),
            "never the column under the gateway"
        );
        assert!(landing.1 > 52, "the tallest block, not one under the exit");
        drop(world);
        let mut no_exit = gateway_tag(gateway, (0, 0, 0));
        if let Tag::Compound(map) = &mut no_exit {
            map.remove("exit_portal");
        }
        write(&mut level, &[], &[(gateway, Some(no_exit))]);
        level.dimension = Dimension::Overworld;
        let mut alex = traveller([40.5, 70.0, 40.5]);
        assert_eq!(travel(&mut level, &mut alex, gateway), None);
        assert_eq!(
            alex.arrival, None,
            "outside the End a gateway with no exit goes nowhere"
        );
    }

    #[test]
    fn a_pearl_lands_on_the_first_block_below_the_exit_or_is_lost_in_the_void() {
        let floor = |pos: Pos, _: f64, _: f64| {
            if pos.1 == 60 {
                vec![(60.0, 61.0)]
            } else {
                Vec::new()
            }
        };
        assert_eq!(
            pearl_drop([0.5, 61.0, 0.5], 0, floor),
            Some([0.5, 61.0, 0.5]),
            "an exit on the ground lands where it is"
        );
        let from_high = pearl_drop([3.25, 75.0, -7.75], 0, floor).expect("lands");
        assert_eq!((from_high[0], from_high[2]), (3.25, -7.75));
        assert!(
            from_high[1] > 61.0 && from_high[1] < 62.0,
            "the start of the tick that reaches the floor: {from_high:?}"
        );
        let mut y = 75.0;
        let mut velocity = 0.0;
        loop {
            velocity = (velocity - 0.03) * 0.99;
            if y + velocity <= 61.0 {
                break;
            }
            y += velocity;
        }
        assert_eq!(from_high[1], y);
        let slab = |pos: Pos, _: f64, _: f64| {
            if pos.1 == 60 {
                vec![(60.0, 60.5)]
            } else {
                Vec::new()
            }
        };
        let on_slab = pearl_drop([0.5, 61.0, 0.5], 0, slab).expect("lands");
        assert!(on_slab[1] > 60.5 && on_slab[1] <= 61.0, "{on_slab:?}");
        assert_eq!(
            pearl_drop([0.5, 60.5, 0.5], 0, floor),
            Some([0.5, 60.5, 0.5]),
            "inside a block it lands at once"
        );
        let mut lowest = f64::MAX;
        let lost = pearl_drop([0.5, 61.0, 0.5], 0, |pos, _, _| {
            lowest = lowest.min(f64::from(pos.1));
            Vec::new()
        });
        assert_eq!(lost, None);
        assert!(lowest < -60.0 && lowest > -70.0, "{lowest}");
        let edge = |pos: Pos, fx: f64, _: f64| {
            if pos.1 == 60 && fx <= 0.5 {
                vec![(60.0, 61.0)]
            } else {
                Vec::new()
            }
        };
        assert_eq!(pearl_drop([0.75, 61.0, 0.5], -64, edge), None);
    }

    #[test]
    fn a_pearl_through_a_gateway_lands_its_thrower_below_the_exit() {
        let Some(mut level) = end_level(None) else {
            return;
        };
        let data = crate::data::GameData::default();
        level.entities = Some(crate::entities::Entities::new(
            &level.terrain.stream,
            Dimension::End,
            77,
            &data,
            None,
            None,
        ));
        let gateway = (40, 70, 40);
        let portal = state(&level, END_GATEWAY);
        let mut exact = gateway_tag(gateway, (20, 120, 0));
        if let Tag::Compound(map) = &mut exact {
            map.insert("ExactTeleport".to_owned(), Tag::Byte(1));
        }
        write(&mut level, &[(gateway, portal)], &[(gateway, Some(exact))]);
        if let Some(entities) = level.entities.as_mut() {
            entities.pearl_gateways.push(gateway);
        }
        let trips = pearls_through(&mut level);
        let landing = trips
            .first()
            .and_then(|&(at, landing)| (at == gateway).then_some(landing).flatten())
            .expect("the pearl lands");
        assert_eq!((landing[0], landing[2]), (20.5, 0.5));
        let entities = level.entities.as_ref().expect("entities");
        assert_eq!(
            entities.pearl_landings,
            vec![glam::DVec3::from_array(landing)]
        );
        assert!(entities.pearl_gateways.is_empty());
        let mut world = ChunkWorld::new(&mut level.terrain.stream);
        let mut ground = landing[1].floor() as i32;
        while !world.full_block((20, ground, 0)) {
            ground -= 1;
        }
        assert!(
            landing[1] >= f64::from(ground + 1) && landing[1] < f64::from(ground + 3),
            "{landing:?} over the ground at {ground}"
        );
    }

    #[test]
    fn far_edits_survive_the_chunk_leaving_memory() {
        let dir = std::env::temp_dir().join(format!(
            "minecraft_survival-gateway-far-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let _cleanup = TestDir(dir.clone());
        let Some(save) = crate::save::open_world(&dir, None, Some("77"), 2).save else {
            return;
        };
        let Some(mut level) = end_level(Some(&save)) else {
            return;
        };
        let far = (1000, 70, 8);
        let (portal, glass) = (state(&level, END_GATEWAY), state(&level, "minecraft:glass"));
        write(
            &mut level,
            &[(far, portal), ((1000, 71, 8), glass)],
            &[(far, Some(gateway_tag(far, (1, 2, 3))))],
        );
        let pos = minecraftoss_core::ChunkPos::new(62, 0);
        let started = std::time::Instant::now();
        while level.terrain.stream.loaded_chunk(pos).is_some() {
            assert!(
                started.elapsed().as_secs() < 120,
                "the far chunk never left"
            );
            let terrain = &mut level.terrain;
            terrain
                .stream
                .server_tick(&[(0, 64, 0)], Some((0, 64, 0)), &mut terrain.scene);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let chunk = level.terrain.stream.load_chunk_now(pos);
        assert_eq!(chunk.block(8, 70, 8), portal);
        assert_eq!(chunk.block(8, 71, 8), glass);
        assert_eq!(
            chunk
                .block_entities
                .entities
                .get(&far)
                .and_then(|tag| tag.get("exit_portal"))
                .and_then(Tag::as_ints),
            Some(vec![1, 2, 3])
        );
    }
}
