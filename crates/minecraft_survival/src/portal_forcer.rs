use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::game_mode::GameMode;
use minecraft_terrain::server::ServerSim;
use minecraft_terrain::terrain::TerrainStream;
use minecraftoss_core::block::flags;
use minecraftoss_core::palette::PalettedContainer;
use minecraftoss_core::{BlockId, BlockStateId, Chunk, ChunkPos, HeightmapKind, Registries};

use crate::level::Terrain;
use crate::player::Player;
use crate::player_nbt::Place;
use crate::portal::{
    NETHER_PORTAL, PLAYER_HEIGHT, PLAYER_WIDTH, PORTAL_SEARCH_LIMIT, PortalAxis, PortalKind,
    PortalTravel, Rect, clamp_to_border, destination, largest_rectangle_around, within_border,
};
use crate::server::{Arrival, Server};

const FREE_MARGIN: f64 = 1.0e-6;

type Arrived = (Place, Vec<((i32, i32, i32), BlockStateId)>);

pub const NETHER_SEARCH_RADIUS: i32 = 16;
pub const OVERWORLD_SEARCH_RADIUS: i32 = 128;
pub const BUILD_RADIUS: i32 = 16;
const FALLBACK_LOWEST: i32 = 70;
const FALLBACK_BELOW_TOP: i32 = 9;
const ROOM_ABOVE_FLOOR: i32 = 4;
const SHALLOW_GAP: i32 = 3;

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

pub trait BuildWorld {
    fn replaceable(&mut self, pos: (i32, i32, i32)) -> bool;
    fn solid(&mut self, pos: (i32, i32, i32)) -> bool;
    fn motion_blocking_height(&mut self, x: i32, z: i32) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placed {
    Obsidian,
    Air,
    Portal(PortalAxis),
}

pub fn spiral_around(
    (x, z): (i32, i32),
    radius: i32,
    first: (i32, i32),
    second: (i32, i32),
) -> Vec<(i32, i32)> {
    let directions = [first, second, (-first.0, -first.1), (-second.0, -second.1)];
    let legs = 4 * radius;
    let (mut leg, mut leg_size, mut leg_index) = (-1i32, 0, 0);
    let mut last = (x + second.0, z + second.1);
    let mut columns = Vec::new();
    loop {
        let step = directions[((leg + 4) % 4) as usize];
        last = (last.0 + step.0, last.1 + step.1);
        if leg_index >= leg_size {
            if leg >= legs {
                return columns;
            }
            leg += 1;
            leg_index = 0;
            leg_size = leg / 2 + 1;
        }
        leg_index += 1;
        columns.push(last);
    }
}

fn step_of(axis: PortalAxis) -> (i32, i32) {
    match axis {
        PortalAxis::X => (1, 0),
        PortalAxis::Z => (0, 1),
    }
}

fn clockwise((dx, dz): (i32, i32)) -> (i32, i32) {
    (-dz, dx)
}

fn offset(
    origin: (i32, i32, i32),
    (dx, dz): (i32, i32),
    along: i32,
    (cx, cz): (i32, i32),
    side: i32,
    up: i32,
) -> (i32, i32, i32) {
    (
        origin.0 + dx * along + cx * side,
        origin.1 + up,
        origin.2 + dz * along + cz * side,
    )
}

fn can_host_frame(
    world: &mut impl BuildWorld,
    origin: (i32, i32, i32),
    step: (i32, i32),
    side: i32,
) -> bool {
    let across = clockwise(step);
    for along in -1..3 {
        for up in -1..4 {
            let at = offset(origin, step, along, across, side, up);
            if up < 0 && !world.solid(at) {
                return false;
            }
            if up >= 0 && !world.replaceable(at) {
                return false;
            }
        }
    }
    true
}

fn block_within_border(x: i32, z: i32) -> bool {
    within_border(f64::from(x), f64::from(z))
}

pub fn build_top(min_y: i32, height: i32, logical_height: i32) -> i32 {
    (min_y + height - 1).min(min_y + logical_height - 1)
}

type Built = (Rect, Vec<((i32, i32, i32), Placed)>);

pub fn create_portal(
    world: &mut impl BuildWorld,
    pos: (i32, i32, i32),
    axis: PortalAxis,
    min_y: i32,
    top: i32,
) -> Option<Built> {
    let step = step_of(axis);
    let mut best: Option<((i32, i32, i32), f64)> = None;
    let mut second: Option<((i32, i32, i32), f64)> = None;
    for (x, z) in spiral_around((pos.0, pos.2), BUILD_RADIUS, (1, 0), (0, 1)) {
        let height = top.min(world.motion_blocking_height(x, z));
        if !block_within_border(x, z) || !block_within_border(x + step.0, z + step.1) {
            continue;
        }
        let mut y = height;
        while y >= min_y {
            if world.replaceable((x, y, z)) {
                let start = y;
                while y > min_y && world.replaceable((x, y - 1, z)) {
                    y -= 1;
                }
                let gap = start - y;
                let origin = (x, y, z);
                if y + ROOM_ABOVE_FLOOR <= top
                    && !(gap > 0 && gap < SHALLOW_GAP)
                    && can_host_frame(world, origin, step, 0)
                {
                    let distance = dist_sqr(pos, origin);
                    if can_host_frame(world, origin, step, -1)
                        && can_host_frame(world, origin, step, 1)
                        && best.is_none_or(|(_, best)| best > distance)
                    {
                        best = Some((origin, distance));
                    }
                    if best.is_none() && second.is_none_or(|(_, second)| second > distance) {
                        second = Some((origin, distance));
                    }
                }
            }
            y -= 1;
        }
    }
    let mut placed = Vec::new();
    let origin = match best.or(second) {
        Some((origin, _)) => origin,
        None => {
            let lowest = (min_y + 1).max(FALLBACK_LOWEST);
            let highest = top - FALLBACK_BELOW_TOP;
            if highest < lowest {
                return None;
            }
            let origin = clamp_to_border([
                f64::from(pos.0 - step.0),
                f64::from(pos.1.clamp(lowest, highest)),
                f64::from(pos.2 - step.1),
            ]);
            let across = clockwise(step);
            for side in -1..2 {
                for along in 0..2 {
                    for up in -1..3 {
                        let block = if up < 0 {
                            Placed::Obsidian
                        } else {
                            Placed::Air
                        };
                        placed.push((offset(origin, step, along, across, side, up), block));
                    }
                }
            }
            origin
        }
    };
    for along in -1..3 {
        for up in -1..4 {
            if along == -1 || along == 2 || up == -1 || up == 3 {
                placed.push((offset(origin, step, along, (0, 0), 0, up), Placed::Obsidian));
            }
        }
    }
    for along in 0..2 {
        for up in 0..3 {
            placed.push((
                offset(origin, step, along, (0, 0), 0, up),
                Placed::Portal(axis),
            ));
        }
    }
    Some((
        Rect {
            min: origin,
            along: 2,
            up: 3,
        },
        placed,
    ))
}

pub fn placed_state(registries: &Registries, block: Placed) -> Option<BlockStateId> {
    let text = match block {
        Placed::Obsidian => "minecraft:obsidian".to_owned(),
        Placed::Air => return Some(BlockStateId::AIR),
        Placed::Portal(axis) => format!("{NETHER_PORTAL}[axis={}]", axis.name()),
    };
    registries.blocks.parse_state(&text).ok()
}

pub fn build(
    terrain: &mut Terrain,
    placed: &[((i32, i32, i32), Placed)],
) -> Vec<((i32, i32, i32), BlockStateId)> {
    let registries = terrain.stream.states.registries().clone();
    let mut written = Vec::new();
    for &(pos, block) in placed {
        let Some(state) = placed_state(&registries, block) else {
            continue;
        };
        let block = terrain.stream.states.block(state).cloned();
        terrain.scene.set(pos, block);
        written.push((pos, state));
    }
    let positions: Vec<_> = written.iter().map(|(pos, _)| *pos).collect();
    terrain.stream.record_edits(&terrain.scene, &positions);
    terrain.stream.mark_edited(&terrain.scene, &positions);
    written
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

    pub fn vertical_range(&self) -> std::ops::Range<i32> {
        self.stream.states.vertical_range()
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

impl BuildWorld for ChunkWorld<'_> {
    fn replaceable(&mut self, pos: (i32, i32, i32)) -> bool {
        let state = self.state(pos);
        let blocks = &self.registries.blocks;
        blocks.is(state, flags::REPLACEABLE) && blocks.state(state).fluid.is_none()
    }

    fn solid(&mut self, pos: (i32, i32, i32)) -> bool {
        let state = self.state(pos);
        self.registries.blocks.is(state, flags::LEGACY_SOLID)
    }

    fn motion_blocking_height(&mut self, x: i32, z: i32) -> i32 {
        let Some(chunk) = self.chunk((x >> 4, z >> 4)) else {
            return i32::MIN;
        };
        let (local_x, local_z) = ((x & 15) as usize, (z & 15) as usize);
        let mut y = chunk.min_y() + chunk.height() - 1;
        while y >= chunk.min_y() {
            let mask = self
                .registries
                .heightmap_mask(chunk.block(local_x, y, local_z));
            if mask & HeightmapKind::MotionBlocking.bit() != 0 {
                return y + 1;
            }
            y -= 1;
        }
        chunk.min_y()
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

pub trait CollisionWorld {
    fn collision_boxes(&mut self, pos: (i32, i32, i32)) -> Vec<[f64; 6]>;
}

impl CollisionWorld for ChunkWorld<'_> {
    fn collision_boxes(&mut self, (x, y, z): (i32, i32, i32)) -> Vec<[f64; 6]> {
        let state = self.state((x, y, z));
        let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
        self.registries
            .blocks
            .collision_boxes(state)
            .into_iter()
            .map(|b| [b[0] + x, b[1] + y, b[2] + z, b[3] + x, b[4] + y, b[5] + z])
            .collect()
    }
}

fn overlaps(a: &[f64; 6], b: &[f64; 6]) -> bool {
    (0..3).all(|i| a[i] < b[i + 3] && a[i + 3] > b[i])
}

pub fn collision_free(world: &mut impl CollisionWorld, feet: [f64; 3]) -> [f64; 3] {
    let grow = [PLAYER_WIDTH / 2.0, PLAYER_HEIGHT / 2.0, PLAYER_WIDTH / 2.0];
    let centre = [feet[0], feet[1] + grow[1], feet[2]];
    let allowed = [
        centre[0] - grow[0] - FREE_MARGIN,
        centre[1] - FREE_MARGIN,
        centre[2] - grow[2] - FREE_MARGIN,
        centre[0] + grow[0] + FREE_MARGIN,
        centre[1] + 1.0 + FREE_MARGIN,
        centre[2] + grow[2] + FREE_MARGIN,
    ];
    let area: [f64; 6] = std::array::from_fn(|i| {
        if i < 3 {
            allowed[i] - grow[i]
        } else {
            allowed[i] + grow[i - 3]
        }
    });
    let mut forbidden = Vec::new();
    let span = |i: usize| area[i].floor() as i32..=area[i + 3].floor() as i32;
    for x in span(0) {
        for y in span(1) {
            for z in span(2) {
                for b in world.collision_boxes((x, y, z)) {
                    if overlaps(&b, &area) {
                        forbidden.push(std::array::from_fn::<f64, 6, _>(|i| {
                            if i < 3 {
                                b[i] - grow[i]
                            } else {
                                b[i] + grow[i - 3]
                            }
                        }));
                    }
                }
            }
        }
    }
    if forbidden.is_empty() {
        return feet;
    }
    let cuts: [Vec<f64>; 3] = std::array::from_fn(|i| {
        let mut cuts = vec![allowed[i], allowed[i + 3]];
        for f in &forbidden {
            for value in [f[i], f[i + 3]] {
                if value > allowed[i] && value < allowed[i + 3] {
                    cuts.push(value);
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        cuts
    });
    let mut best: Option<([f64; 3], f64)> = None;
    for xs in cuts[0].windows(2) {
        for ys in cuts[1].windows(2) {
            for zs in cuts[2].windows(2) {
                let cell = [xs[0], ys[0], zs[0], xs[1], ys[1], zs[1]];
                let middle: [f64; 3] = std::array::from_fn(|i| (cell[i] + cell[i + 3]) / 2.0);
                let blocked = forbidden
                    .iter()
                    .any(|f| (0..3).all(|i| middle[i] > f[i] && middle[i] < f[i + 3]));
                if blocked {
                    continue;
                }
                let point: [f64; 3] =
                    std::array::from_fn(|i| centre[i].clamp(cell[i], cell[i + 3]));
                let distance: f64 = (0..3).map(|i| (point[i] - centre[i]).powi(2)).sum();
                if best.is_none_or(|(_, best)| distance < best) {
                    best = Some((point, distance));
                }
            }
        }
    }
    best.map_or(feet, |(point, _)| [point[0], point[1] - grow[1], point[2]])
}

pub fn arrive(terrain: &mut Terrain, travel: &PortalTravel) -> Option<Arrived> {
    let (min_y, height, logical_height) = (
        terrain.rules.min_y,
        terrain.rules.height,
        terrain.rules.logical_height,
    );
    let mut world = ChunkWorld::new(&mut terrain.stream);
    let (rect, axis, placed) =
        match find_closest(&mut world, travel.target, search_radius(travel.to)) {
            Some(found) => {
                let axis = world.portal_axis(found)?;
                let rect = largest_rectangle_around(
                    found,
                    axis,
                    PORTAL_SEARCH_LIMIT,
                    PORTAL_SEARCH_LIMIT,
                    |pos| world.portal_axis(pos) == Some(axis),
                );
                (rect, axis, Vec::new())
            }
            None if travel.creates => {
                world.generates = true;
                let top = build_top(min_y, height, logical_height);
                let (rect, placed) =
                    create_portal(&mut world, travel.target, travel.entry_axis, min_y, top)?;
                (rect, travel.entry_axis, placed)
            }
            None => return None,
        };
    let written = build(terrain, &placed);
    let mut place = travel.exit(rect, axis);
    place.feet = collision_free(&mut ChunkWorld::new(&mut terrain.stream), place.feet);
    Some((place, written))
}

pub fn arrival(terrain: &mut Terrain, travel: &PortalTravel) -> Arrival {
    match arrive(terrain, travel) {
        Some((place, _)) => Arrival::of(&place),
        None => {
            diag::warn!(
                World,
                "Minecraft found no portal near {:?} in {} and could build none",
                travel.target,
                travel.to.dimension_type()
            );
            Arrival::of(&travel.target_place())
        }
    }
}

pub fn travel_into(
    players: &mut BTreeMap<PlayerUuid, Player>,
    dimension: Dimension,
    registries: &Registries,
) -> Option<PortalTravel> {
    let waiting = |player: &Player| {
        player.dimension == dimension
            && player
                .arrival
                .is_none_or(|arrival| arrival.dimension != dimension)
    };
    for player in players.values_mut() {
        if waiting(player)
            && let Some(travel) = player
                .portal
                .travel
                .take_if(|travel| travel.to == dimension)
        {
            return Some(travel);
        }
    }
    if dimension != Dimension::Nether {
        return None;
    }
    players
        .values()
        .filter(|player| waiting(player) && !player.dead)
        .find_map(|player| {
            let place = player.place.filter(|place| place.dimension != dimension)?;
            let scale =
                crate::switch::teleport_scale(registries, place.dimension, dimension).ok()?;
            Some(PortalTravel {
                kind: PortalKind::Nether,
                to: dimension,
                target: destination(place.feet, scale),
                entry_axis: PortalAxis::X,
                relative: [0.5, 0.0, 0.0],
                yaw: place.yaw,
                pitch: place.pitch,
                motion: [0.0; 3],
                creates: player.mode.mode != GameMode::Spectator,
            })
        })
}

pub fn waits_for_portal(player: &Player, dimension: Dimension) -> bool {
    player
        .portal
        .travel
        .is_some_and(|travel| travel.to == dimension)
}

pub fn arrive_through_portals(server: &mut Server) {
    let Server {
        levels, players, ..
    } = server;
    for player in players.values_mut() {
        let Some(travel) = player.portal.travel else {
            continue;
        };
        if travel.to != player.dimension {
            player.portal.travel = None;
            continue;
        }
        if player.switching.is_some_and(|switch| !switch.left) {
            continue;
        }
        let Some(level) = levels.get_mut(&travel.to) else {
            continue;
        };
        player.portal.travel = None;
        let place = match arrive(&mut level.terrain, &travel) {
            Some((place, written)) => {
                if let Some(entities) = level.entities.as_mut().filter(|_| !written.is_empty()) {
                    entities.run_job(Box::new(
                        move |sim: &mut ServerSim| -> Box<dyn Any + Send> {
                            for (pos, state) in written {
                                sim.command_place(pos, state, &[], true);
                            }
                            Box::new(())
                        },
                    ));
                }
                place
            }
            None => travel.target_place(),
        };
        player.arrive(place);
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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Fake {
        Air,
        Stone,
        Water,
    }

    struct FakeBuild<F: Fn((i32, i32, i32)) -> Fake> {
        ground: F,
        min_y: i32,
        max_y: i32,
    }

    impl<F: Fn((i32, i32, i32)) -> Fake> BuildWorld for FakeBuild<F> {
        fn replaceable(&mut self, pos: (i32, i32, i32)) -> bool {
            (self.ground)(pos) == Fake::Air
        }

        fn solid(&mut self, pos: (i32, i32, i32)) -> bool {
            (self.ground)(pos) == Fake::Stone
        }

        fn motion_blocking_height(&mut self, x: i32, z: i32) -> i32 {
            (self.min_y..=self.max_y)
                .rev()
                .find(|&y| (self.ground)((x, y, z)) != Fake::Air)
                .map_or(self.min_y, |y| y + 1)
        }
    }

    fn count(placed: &[((i32, i32, i32), Placed)], kind: Placed) -> usize {
        placed.iter().filter(|(_, block)| *block == kind).count()
    }

    #[test]
    fn the_build_spiral_walks_out_from_the_middle_like_spiral_around() {
        assert_eq!(
            spiral_around((0, 0), 1, (1, 0), (0, 1)),
            vec![
                (0, 0),
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1),
                (0, -1),
                (1, -1)
            ]
        );
        let columns = spiral_around((5, -7), BUILD_RADIUS, (1, 0), (0, 1));
        let unique: std::collections::BTreeSet<_> = columns.iter().copied().collect();
        assert_eq!((columns.len(), unique.len()), (33 * 33, 33 * 33));
        assert!(
            columns
                .iter()
                .all(|&(x, z)| (x - 5).abs() <= 16 && (z + 7).abs() <= 16)
        );
        assert_eq!(build_top(0, 256, 128), 127);
        assert_eq!(build_top(-64, 384, 384), 319);
    }

    #[test]
    fn a_portal_is_built_on_the_nearest_floor_with_room_around_it() {
        let mut flat = FakeBuild {
            ground: |(_, y, _): (i32, i32, i32)| {
                if (0..64).contains(&y) {
                    Fake::Stone
                } else {
                    Fake::Air
                }
            },
            min_y: 0,
            max_y: 255,
        };
        let (rect, placed) =
            create_portal(&mut flat, (3, 80, -5), PortalAxis::X, 0, 255).expect("built");
        assert_eq!(
            rect,
            Rect {
                min: (3, 64, -5),
                along: 2,
                up: 3
            }
        );
        assert_eq!(
            (
                count(&placed, Placed::Obsidian),
                count(&placed, Placed::Air)
            ),
            (14, 0)
        );
        assert_eq!(count(&placed, Placed::Portal(PortalAxis::X)), 6);
        for corner in [(2, 63, -5), (5, 63, -5), (2, 67, -5), (5, 67, -5)] {
            assert!(placed.contains(&(corner, Placed::Obsidian)), "{corner:?}");
        }
        assert!(placed.contains(&((4, 66, -5), Placed::Portal(PortalAxis::X))));
        let (rect, placed) =
            create_portal(&mut flat, (3, 80, -5), PortalAxis::Z, 0, 255).expect("built");
        assert_eq!(rect.min, (3, 64, -5));
        assert!(
            placed.contains(&((3, 63, -6), Placed::Obsidian))
                && placed.contains(&((3, 67, -3), Placed::Obsidian))
        );
        let mut ledge = FakeBuild {
            ground: |(_, y, z): (i32, i32, i32)| {
                if y == 63 && z == 0 {
                    Fake::Stone
                } else {
                    Fake::Air
                }
            },
            min_y: 0,
            max_y: 255,
        };
        let (rect, _) =
            create_portal(&mut ledge, (5, 70, 0), PortalAxis::X, 0, 255).expect("built");
        assert_eq!(
            rect.min,
            (5, 64, 0),
            "a ledge with no room beside it is still taken when nothing better is near"
        );
    }

    #[test]
    fn a_portal_over_water_or_under_the_nether_roof_stands_on_its_own_platform() {
        let mut lake = FakeBuild {
            ground: |(x, y, z): (i32, i32, i32)| match y {
                0..64 => Fake::Stone,
                64 if x.abs() <= 20 && z.abs() <= 20 => Fake::Water,
                _ => Fake::Air,
            },
            min_y: 0,
            max_y: 255,
        };
        let (rect, placed) =
            create_portal(&mut lake, (0, 30, 0), PortalAxis::X, 0, 255).expect("built");
        assert_eq!(rect.min, (-1, 70, 0));
        assert_eq!(
            (
                count(&placed, Placed::Obsidian),
                count(&placed, Placed::Air)
            ),
            (6 + 14, 18)
        );
        assert!(
            placed.contains(&((-1, 69, -1), Placed::Obsidian))
                && placed.contains(&((0, 72, 1), Placed::Air))
        );
        let roofed = |room: std::ops::RangeInclusive<i32>| {
            move |(x, y, z): (i32, i32, i32)| {
                if room.contains(&y) && x.abs() <= 5 && z.abs() <= 5 || !(0..=127).contains(&y) {
                    Fake::Air
                } else {
                    Fake::Stone
                }
            }
        };
        let top = build_top(0, 256, 128);
        let mut cave = FakeBuild {
            ground: roofed(100..=110),
            min_y: 0,
            max_y: 255,
        };
        let (rect, _) = create_portal(&mut cave, (0, 64, 0), PortalAxis::X, 0, top).expect("built");
        assert_eq!(rect.min, (0, 100, 0));
        let mut under_roof = FakeBuild {
            ground: roofed(124..=126),
            min_y: 0,
            max_y: 255,
        };
        let (rect, _) =
            create_portal(&mut under_roof, (100, 200, -200), PortalAxis::Z, 0, top).expect("built");
        assert_eq!(
            rect.min,
            (100, 118, -201),
            "no room under 127, so the platform at most 9 below it"
        );
        let mut low = FakeBuild {
            ground: roofed(124..=126),
            min_y: 0,
            max_y: 255,
        };
        assert_eq!(
            create_portal(&mut low, (0, 10, 0), PortalAxis::X, 0, top).map(|(rect, _)| rect.min),
            Some((-1, 70, 0))
        );
        let mut squeezed = FakeBuild {
            ground: roofed(124..=126),
            min_y: 0,
            max_y: 255,
        };
        assert!(create_portal(&mut squeezed, (0, 10, 0), PortalAxis::X, 0, 75).is_none());
    }

    struct Walls(Vec<(i32, i32, i32)>);

    impl CollisionWorld for Walls {
        fn collision_boxes(&mut self, (x, y, z): (i32, i32, i32)) -> Vec<[f64; 6]> {
            if !self.0.contains(&(x, y, z)) {
                return Vec::new();
            }
            let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
            vec![[x, y, z, x + 1.0, y + 1.0, z + 1.0]]
        }
    }

    #[test]
    fn an_exit_spot_inside_a_wall_moves_to_the_nearest_free_place() {
        let floor: Vec<_> = (-2..3)
            .flat_map(|x| (-2..4).map(move |z| (x, 63, z)))
            .collect();
        let mut open = Walls(floor.clone());
        assert_eq!(
            collision_free(&mut open, [0.5, 64.0, 0.5]),
            [0.5, 64.0, 0.5]
        );
        let mut wall = Walls(
            floor
                .iter()
                .copied()
                .chain([(0, 64, 1), (0, 65, 1)])
                .collect(),
        );
        let moved = collision_free(&mut wall, [0.5, 64.0, 0.9]);
        assert!((moved[2] - 0.7).abs() < 1e-9, "{moved:?}");
        assert_eq!((moved[0], moved[1]), (0.5, 64.0));
        let mut side = Walls(
            floor
                .iter()
                .copied()
                .chain([(1, 64, 0), (1, 65, 0)])
                .collect(),
        );
        let moved = collision_free(&mut side, [0.9, 64.0, 0.5]);
        assert!(
            (moved[0] - 0.7).abs() < 1e-9 && moved[2] == 0.5,
            "{moved:?}"
        );
        let mut buried = Walls(
            (-2..3)
                .flat_map(|x| (62..68).flat_map(move |y| (-2..3).map(move |z| (x, y, z))))
                .collect(),
        );
        assert_eq!(
            collision_free(&mut buried, [0.5, 64.0, 0.5]),
            [0.5, 64.0, 0.5]
        );
    }

    #[test]
    fn a_portal_is_built_in_real_nether_terrain_below_the_roof() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let mut terrain = Terrain::open(
            Arc::new(registries),
            77,
            2,
            Dimension::Nether,
            None,
            Some((0, 64, 0)),
        )
        .expect("a Nether level");
        let mut world = ChunkWorld::new(&mut terrain.stream);
        world.generates = true;
        let (rect, placed) =
            create_portal(&mut world, (0, 64, 0), PortalAxis::X, 0, 127).expect("built");
        assert!(rect.min.1 + 3 <= 127 && rect.min.1 >= 1, "{rect:?}");
        assert!(
            (rect.min.0).abs() <= 17 && (rect.min.2).abs() <= 17,
            "{rect:?}"
        );
        for (pos, block) in &placed {
            if let Placed::Portal(_) = block {
                assert!(world.replaceable(*pos), "{pos:?} was not free");
            }
        }
        let written = build(&mut terrain, &placed);
        assert_eq!(written.len(), placed.len());
        let (x, y, z) = rect.min;
        let top = (x, y + 2, z);
        assert_eq!(
            crate::portal::portal_axis_in(&terrain, top),
            Some(PortalAxis::X),
            "the scene shows the portal"
        );
        assert!(
            terrain
                .stream
                .loaded_chunk(ChunkPos::new(x >> 4, z >> 4))
                .is_some()
        );
        let mut again = ChunkWorld::new(&mut terrain.stream);
        assert_eq!(
            find_closest(&mut again, (x, y, z), 16),
            Some(rect.min),
            "the chunk map holds the built portal"
        );
        assert_eq!(again.portal_axis(top), Some(PortalAxis::X));
        assert!(again.solid((x - 1, y + 3, z)) && again.solid((x, y - 1, z)));
    }
}
