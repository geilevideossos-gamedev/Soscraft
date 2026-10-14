use super::golem_build::{GolemMatch, translate_and_rotate};
use super::{Level, update};
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::{BlockPos, BlockStateId};

pub const WITHER: &str = "minecraft:wither";
pub const SKULL: &str = "minecraft:wither_skeleton_skull";
pub const WALL_SKULL: &str = "minecraft:wither_skeleton_wall_skull";
pub const BASE_TAG: &str = "minecraft:wither_summon_base_blocks";
pub const ROWS: [&str; 3] = ["^^^", "###", "~#~"];
pub const INVULNERABLE_TICKS: i32 = 220;
pub const BUILT_HEALTH: f32 = 100.0;
pub const SPAWN_CELL: (i32, i32) = (1, 2);
pub const SPAWN_RISE: f64 = 0.55;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Skull,
    Base,
    Air,
    Other,
}

pub fn is_skull(name: &str) -> bool {
    name == SKULL || name == WALL_SKULL
}

fn cell_matches(cell: char, found: Option<Cell>) -> bool {
    match cell {
        ' ' => true,
        '^' => found == Some(Cell::Skull),
        '#' => found == Some(Cell::Base),
        '~' => found == Some(Cell::Air),
        _ => false,
    }
}

fn matches_at(
    origin: BlockPos,
    forwards: Direction,
    up: Direction,
    cell_at: &dyn Fn(BlockPos) -> Option<Cell>,
) -> Option<GolemMatch> {
    for (down, row) in ROWS.iter().enumerate() {
        for (right, cell) in row.chars().enumerate() {
            let pos = translate_and_rotate(origin, forwards, up, right as i32, down as i32, 0);
            if !cell_matches(cell, cell_at(pos)) {
                return None;
            }
        }
    }
    Some(GolemMatch {
        front_top_left: origin,
        forwards,
        up,
        width: 3,
        height: 3,
    })
}

pub fn find(origin: BlockPos, cell_at: &dyn Fn(BlockPos) -> Option<Cell>) -> Option<GolemMatch> {
    for dz in 0..3 {
        for dy in 0..3 {
            for dx in 0..3 {
                let pos = origin.offset(dx, dy, dz);
                for forwards in Direction::ALL {
                    for up in Direction::ALL {
                        if up == forwards || up == forwards.opposite() {
                            continue;
                        }
                        if let Some(found) = matches_at(pos, forwards, up, cell_at) {
                            return Some(found);
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn spawn_position(found: &GolemMatch) -> [f64; 3] {
    let at = found.block(SPAWN_CELL.0, SPAWN_CELL.1);
    [
        f64::from(at.x) + 0.5,
        f64::from(at.y) + SPAWN_RISE,
        f64::from(at.z) + 0.5,
    ]
}

pub fn spawn_yaw(found: &GolemMatch) -> f32 {
    if matches!(found.forwards, Direction::West | Direction::East) {
        0.0
    } else {
        90.0
    }
}

pub fn wither_tag(default: Option<Tag>) -> Tag {
    let mut tag = default.unwrap_or_else(|| Tag::Compound(Default::default()));
    if let Tag::Compound(map) = &mut tag {
        map.insert("id".to_owned(), Tag::String(WITHER.to_owned()));
        map.insert("Invul".to_owned(), Tag::Int(INVULNERABLE_TICKS));
        map.insert("Health".to_owned(), Tag::Float(BUILT_HEALTH));
    }
    tag
}

impl Level<'_> {
    fn wither_cell(
        &self,
        pos: BlockPos,
        base: Option<minecraftoss_core::tags::TagId>,
    ) -> Option<Cell> {
        self.chunk(pos.chunk())?;
        let state = self.block(pos);
        let name = self.name(state);
        Some(if is_skull(name) {
            Cell::Skull
        } else if base.is_some_and(|tag| self.lib.registries.block_in_tag(state, tag)) {
            Cell::Base
        } else if self.registries().blocks.is_air(state) {
            Cell::Air
        } else {
            Cell::Other
        })
    }

    pub fn wither_skull_placed(&mut self, pos: BlockPos) {
        if !is_skull(self.name(self.block(pos))) || pos.y < self.min_y() || self.difficulty == 0 {
            return;
        }
        let base = self.lib.registries.block_tags.id(BASE_TAG);
        let Some(found) = find(pos, &|at| self.wither_cell(at, base)) else {
            return;
        };
        let cells: Vec<(BlockPos, BlockStateId)> = found
            .cells()
            .into_iter()
            .map(|cell| (cell, self.block(cell)))
            .collect();
        for &(cell, old) in &cells {
            self.set_block(cell, BlockStateId::AIR, update::CLIENTS, update::LIMIT);
            if !self.registries().blocks.is_air(old) {
                if let Some(destroyed) = self.destroyed.as_mut() {
                    destroyed.push((cell, old));
                }
            }
        }
        let default = self
            .registries()
            .entities
            .as_ref()
            .and_then(|entities| entities.default_tag(WITHER));
        let mut tag = wither_tag(default);
        let uuid = crate::natural_spawner::SpawnLevel::next_uuid(self);
        minecraftoss_core::entity_data::place(
            &mut tag,
            spawn_position(&found),
            spawn_yaw(&found),
            0.0,
            uuid,
        );
        self.spawned.push(tag);
        let air = self.block_id(BlockStateId::AIR);
        for (cell, _) in cells {
            self.update_neighbors_at(cell, air);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn cell_of(name: &str) -> Cell {
        match name {
            SKULL | WALL_SKULL => Cell::Skull,
            "minecraft:soul_sand" | "minecraft:soul_soil" => Cell::Base,
            "minecraft:air" => Cell::Air,
            _ => Cell::Other,
        }
    }

    fn lookup(map: &HashMap<BlockPos, String>) -> impl Fn(BlockPos) -> Option<Cell> + '_ {
        |pos| {
            Some(cell_of(
                map.get(&pos).map_or("minecraft:air", String::as_str),
            ))
        }
    }

    fn wither_t(
        middle_skull: (i32, i32, i32),
        sideways: (i32, i32, i32),
        skull: &str,
        base: &str,
    ) -> HashMap<BlockPos, String> {
        let (x, y, z) = middle_skull;
        let (sx, sy, sz) = sideways;
        let mut map = HashMap::new();
        for side in [-1, 0, 1] {
            map.insert(
                BlockPos::new(x + side * sx, y + side * sy, z + side * sz),
                skull.to_owned(),
            );
            map.insert(
                BlockPos::new(x + side * sx, y - 1 + side * sy, z + side * sz),
                base.to_owned(),
            );
        }
        map.insert(BlockPos::new(x, y - 2, z), base.to_owned());
        map
    }

    #[test]
    fn three_skulls_on_a_soul_sand_t_are_found_in_all_four_rotations() {
        for sideways in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)] {
            for (skull, base) in [
                (SKULL, "minecraft:soul_sand"),
                (WALL_SKULL, "minecraft:soul_soil"),
            ] {
                let map = wither_t((5, 70, 5), sideways, skull, base);
                for placed in map.keys().filter(|p| map[p] == skull) {
                    let found = find(*placed, &lookup(&map)).expect("the T is a wither");
                    let mut cells = found.cells();
                    cells.sort_by_key(|p| (p.x, p.y, p.z));
                    assert_eq!(cells.len(), 9);
                    for pos in map.keys() {
                        assert!(cells.contains(pos), "{pos:?} is part of the wither");
                    }
                    assert_eq!(spawn_position(&found), [5.5, 68.55, 5.5]);
                    let expected = if sideways.0 != 0 { 90.0 } else { 0.0 };
                    assert_eq!(spawn_yaw(&found), expected, "{sideways:?}");
                }
            }
        }
    }

    #[test]
    fn a_mixed_soul_sand_and_soil_t_counts() {
        let mut map = wither_t((5, 70, 5), (1, 0, 0), SKULL, "minecraft:soul_sand");
        map.insert(BlockPos::new(5, 68, 5), "minecraft:soul_soil".to_owned());
        assert!(find(BlockPos::new(4, 70, 5), &lookup(&map)).is_some());
    }

    #[test]
    fn two_skulls_another_base_or_blocked_corners_build_nothing() {
        let mut map = wither_t((5, 70, 5), (1, 0, 0), SKULL, "minecraft:soul_sand");
        map.remove(&BlockPos::new(6, 70, 5));
        assert!(
            find(BlockPos::new(5, 70, 5), &lookup(&map)).is_none(),
            "two skulls"
        );
        let map = wither_t(
            (5, 70, 5),
            (1, 0, 0),
            "minecraft:skeleton_skull",
            "minecraft:soul_sand",
        );
        assert!(
            find(BlockPos::new(5, 70, 5), &lookup(&map)).is_none(),
            "skeleton skulls"
        );
        let map = wither_t((5, 70, 5), (1, 0, 0), SKULL, "minecraft:sand");
        assert!(
            find(BlockPos::new(5, 70, 5), &lookup(&map)).is_none(),
            "sand"
        );
        let mut map = wither_t((5, 70, 5), (1, 0, 0), SKULL, "minecraft:soul_sand");
        map.insert(BlockPos::new(4, 68, 5), "minecraft:stone".to_owned());
        assert!(
            find(BlockPos::new(5, 70, 5), &lookup(&map)).is_none(),
            "the corner under an arm must be air"
        );
    }

    #[test]
    fn a_wither_built_lying_down_is_found_too() {
        let mut map = HashMap::new();
        for z in 4..=6 {
            map.insert(BlockPos::new(10, 64, z), SKULL.to_owned());
            map.insert(BlockPos::new(9, 64, z), "minecraft:soul_sand".to_owned());
        }
        map.insert(BlockPos::new(8, 64, 5), "minecraft:soul_sand".to_owned());
        let found = find(BlockPos::new(10, 64, 6), &lookup(&map)).expect("lying down");
        assert_eq!(spawn_position(&found), [8.5, 64.55, 5.5]);
    }

    #[test]
    fn a_built_wither_starts_charging_at_a_third_of_its_health() {
        let tag = wither_tag(None);
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some(WITHER));
        assert_eq!(tag.get("Invul"), Some(&Tag::Int(220)));
        assert_eq!(tag.get("Health"), Some(&Tag::Float(100.0)));
    }
}
