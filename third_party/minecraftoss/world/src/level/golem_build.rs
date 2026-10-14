use super::{Level, update};
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::{BlockPos, BlockStateId};

pub const IRON_GOLEM: &str = "minecraft:iron_golem";
pub const SNOW_GOLEM: &str = "minecraft:snow_golem";
pub const CARVED_PUMPKIN: &str = "minecraft:carved_pumpkin";
pub const JACK_O_LANTERN: &str = "minecraft:jack_o_lantern";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GolemPattern {
    pub kind: &'static str,
    pub body: &'static str,
    pub rows: &'static [&'static str],
    pub spawn: (i32, i32),
}

pub const SNOW_GOLEM_FULL: GolemPattern = GolemPattern {
    kind: SNOW_GOLEM,
    body: "minecraft:snow_block",
    rows: &["^", "#", "#"],
    spawn: (0, 2),
};

pub const IRON_GOLEM_FULL: GolemPattern = GolemPattern {
    kind: IRON_GOLEM,
    body: "minecraft:iron_block",
    rows: &["~^~", "###", "~#~"],
    spawn: (1, 2),
};

pub const GOLEMS: [GolemPattern; 2] = [SNOW_GOLEM_FULL, IRON_GOLEM_FULL];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GolemMatch {
    pub front_top_left: BlockPos,
    pub forwards: Direction,
    pub up: Direction,
    pub width: i32,
    pub height: i32,
}

impl GolemMatch {
    pub fn block(&self, right: i32, down: i32) -> BlockPos {
        translate_and_rotate(self.front_top_left, self.forwards, self.up, right, down, 0)
    }

    pub fn cells(&self) -> Vec<BlockPos> {
        let mut cells = Vec::new();
        for right in 0..self.width {
            for down in 0..self.height {
                cells.push(self.block(right, down));
            }
        }
        cells
    }
}

fn cross(a: (i32, i32, i32), b: (i32, i32, i32)) -> (i32, i32, i32) {
    (
        a.1 * b.2 - a.2 * b.1,
        a.2 * b.0 - a.0 * b.2,
        a.0 * b.1 - a.1 * b.0,
    )
}

pub fn translate_and_rotate(
    origin: BlockPos,
    forwards: Direction,
    up: Direction,
    right: i32,
    down: i32,
    depth: i32,
) -> BlockPos {
    let f = forwards.offset();
    let u = up.offset();
    let r = cross(f, u);
    origin.offset(
        u.0 * -down + r.0 * right + f.0 * depth,
        u.1 * -down + r.1 * right + f.1 * depth,
        u.2 * -down + r.2 * right + f.2 * depth,
    )
}

pub fn is_pumpkin(name: &str) -> bool {
    name == CARVED_PUMPKIN || name == JACK_O_LANTERN
}

fn is_air(name: &str) -> bool {
    matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
}

fn cell_matches(pattern: &GolemPattern, cell: char, name: Option<&str>) -> bool {
    match cell {
        ' ' => true,
        '^' => name.is_some_and(is_pumpkin),
        '#' => name == Some(pattern.body),
        '~' => name.is_some_and(is_air),
        _ => false,
    }
}

fn matches_at(
    pattern: &GolemPattern,
    origin: BlockPos,
    forwards: Direction,
    up: Direction,
    name_at: &dyn Fn(BlockPos) -> Option<String>,
) -> Option<GolemMatch> {
    let width = pattern.rows[0].chars().count() as i32;
    let height = pattern.rows.len() as i32;
    for right in 0..width {
        for down in 0..height {
            let cell = pattern.rows[down as usize]
                .chars()
                .nth(right as usize)
                .unwrap_or(' ');
            let pos = translate_and_rotate(origin, forwards, up, right, down, 0);
            if !cell_matches(pattern, cell, name_at(pos).as_deref()) {
                return None;
            }
        }
    }
    Some(GolemMatch {
        front_top_left: origin,
        forwards,
        up,
        width,
        height,
    })
}

pub fn find(
    pattern: &GolemPattern,
    origin: BlockPos,
    name_at: &dyn Fn(BlockPos) -> Option<String>,
) -> Option<GolemMatch> {
    let width = pattern.rows[0].chars().count() as i32;
    let height = pattern.rows.len() as i32;
    let reach = width.max(height).max(1);
    for dz in 0..reach {
        for dy in 0..reach {
            for dx in 0..reach {
                let pos = origin.offset(dx, dy, dz);
                for forwards in Direction::ALL {
                    for up in Direction::ALL {
                        if up == forwards || up == forwards.opposite() {
                            continue;
                        }
                        if let Some(found) = matches_at(pattern, pos, forwards, up, name_at) {
                            return Some(found);
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn spawn_position(pattern: &GolemPattern, found: &GolemMatch) -> [f64; 3] {
    let at = found.block(pattern.spawn.0, pattern.spawn.1);
    [
        f64::from(at.x) + 0.5,
        f64::from(at.y) + 0.05,
        f64::from(at.z) + 0.5,
    ]
}

pub fn golem_tag(pattern: &GolemPattern, default: Option<Tag>) -> Tag {
    let mut tag = default.unwrap_or_else(|| Tag::Compound(Default::default()));
    if let Tag::Compound(map) = &mut tag {
        map.insert("id".to_owned(), Tag::String(pattern.kind.to_owned()));
        if pattern.kind == IRON_GOLEM {
            map.insert("PlayerCreated".to_owned(), Tag::Byte(1));
        }
    }
    tag
}

impl Level<'_> {
    fn name_if_loaded(&self, pos: BlockPos) -> Option<String> {
        self.chunk(pos.chunk())?;
        Some(self.name(self.block(pos)).to_owned())
    }

    pub(super) fn carved_pumpkin_on_place(&mut self, pos: BlockPos) {
        for pattern in GOLEMS {
            let found = find(&pattern, pos, &|at| self.name_if_loaded(at));
            if let Some(found) = found {
                self.spawn_golem_in_world(&pattern, &found);
                return;
            }
        }
    }

    fn spawn_golem_in_world(&mut self, pattern: &GolemPattern, found: &GolemMatch) {
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
            .and_then(|entities| entities.default_tag(pattern.kind));
        let mut tag = golem_tag(pattern, default);
        let uuid = crate::natural_spawner::SpawnLevel::next_uuid(self);
        minecraftoss_core::entity_data::place(
            &mut tag,
            spawn_position(pattern, found),
            0.0,
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

    fn world(blocks: &[((i32, i32, i32), &str)]) -> HashMap<BlockPos, String> {
        blocks
            .iter()
            .map(|&((x, y, z), name)| (BlockPos::new(x, y, z), name.to_owned()))
            .collect()
    }

    fn lookup(map: &HashMap<BlockPos, String>) -> impl Fn(BlockPos) -> Option<String> + '_ {
        |pos| {
            Some(
                map.get(&pos)
                    .cloned()
                    .unwrap_or_else(|| "minecraft:air".to_owned()),
            )
        }
    }

    fn iron_t(
        head: (i32, i32, i32),
        sideways: (i32, i32, i32),
        pumpkin: &str,
    ) -> Vec<((i32, i32, i32), String)> {
        let (x, y, z) = head;
        let (sx, sy, sz) = sideways;
        vec![
            ((x, y, z), pumpkin.to_owned()),
            ((x, y - 1, z), "minecraft:iron_block".to_owned()),
            (
                (x - sx, y - 1 - sy, z - sz),
                "minecraft:iron_block".to_owned(),
            ),
            (
                (x + sx, y - 1 + sy, z + sz),
                "minecraft:iron_block".to_owned(),
            ),
            ((x, y - 2, z), "minecraft:iron_block".to_owned()),
        ]
    }

    fn owned(blocks: &[((i32, i32, i32), String)]) -> HashMap<BlockPos, String> {
        blocks
            .iter()
            .map(|((x, y, z), name)| (BlockPos::new(*x, *y, *z), name.clone()))
            .collect()
    }

    #[test]
    fn an_iron_t_with_a_carved_pumpkin_on_top_is_found_facing_either_way() {
        for sideways in [(1, 0, 0), (0, 0, 1)] {
            for pumpkin in [CARVED_PUMPKIN, JACK_O_LANTERN] {
                let map = owned(&iron_t((5, 70, 5), sideways, pumpkin));
                let found = find(&IRON_GOLEM_FULL, BlockPos::new(5, 70, 5), &lookup(&map))
                    .expect("the T is a golem");
                let mut cells = found.cells();
                cells.sort_by_key(|p| (p.x, p.y, p.z));
                assert_eq!(cells.len(), 9);
                for (pos, _) in &map {
                    assert!(cells.contains(pos), "{pos:?} is part of the golem");
                }
                assert_eq!(spawn_position(&IRON_GOLEM_FULL, &found), [5.5, 68.05, 5.5]);
            }
        }
    }

    #[test]
    fn the_four_rotations_of_the_iron_t_all_build_a_golem() {
        for (head, sideways) in [
            ((0, 66, 0), (1, 0, 0)),
            ((0, 66, 0), (-1, 0, 0)),
            ((0, 66, 0), (0, 0, 1)),
            ((0, 66, 0), (0, 0, -1)),
        ] {
            let map = owned(&iron_t(head, sideways, CARVED_PUMPKIN));
            let found = find(&IRON_GOLEM_FULL, BlockPos::new(0, 66, 0), &lookup(&map));
            assert!(found.is_some(), "{sideways:?}");
        }
    }

    #[test]
    fn the_arms_need_air_under_them_and_every_iron_block() {
        let mut map = owned(&iron_t((5, 70, 5), (1, 0, 0), CARVED_PUMPKIN));
        map.insert(BlockPos::new(4, 68, 5), "minecraft:dirt".to_owned());
        assert!(find(&IRON_GOLEM_FULL, BlockPos::new(5, 70, 5), &lookup(&map)).is_none());
        let mut map = owned(&iron_t((5, 70, 5), (1, 0, 0), CARVED_PUMPKIN));
        map.insert(BlockPos::new(6, 69, 5), "minecraft:gold_block".to_owned());
        assert!(find(&IRON_GOLEM_FULL, BlockPos::new(5, 70, 5), &lookup(&map)).is_none());
        let map = owned(&iron_t((5, 70, 5), (1, 0, 0), "minecraft:pumpkin"));
        assert!(find(&IRON_GOLEM_FULL, BlockPos::new(5, 70, 5), &lookup(&map)).is_none());
    }

    #[test]
    fn the_corners_beside_the_head_must_be_air() {
        let mut map = owned(&iron_t((5, 70, 5), (1, 0, 0), CARVED_PUMPKIN));
        map.insert(BlockPos::new(4, 70, 5), "minecraft:stone".to_owned());
        map.insert(BlockPos::new(6, 70, 5), "minecraft:stone".to_owned());
        assert!(
            find(&IRON_GOLEM_FULL, BlockPos::new(5, 70, 5), &lookup(&map)).is_none(),
            "the corners must be air"
        );
        let mut map = owned(&iron_t((5, 70, 5), (1, 0, 0), CARVED_PUMPKIN));
        map.insert(BlockPos::new(5, 70, 6), "minecraft:stone".to_owned());
        assert!(
            find(&IRON_GOLEM_FULL, BlockPos::new(5, 70, 5), &lookup(&map)).is_some(),
            "outside the pattern"
        );
    }

    #[test]
    fn a_golem_built_lying_down_is_found_too() {
        let map = world(&[
            ((10, 64, 5), CARVED_PUMPKIN),
            ((9, 64, 5), "minecraft:iron_block"),
            ((9, 64, 4), "minecraft:iron_block"),
            ((9, 64, 6), "minecraft:iron_block"),
            ((8, 64, 5), "minecraft:iron_block"),
        ]);
        let found =
            find(&IRON_GOLEM_FULL, BlockPos::new(10, 64, 5), &lookup(&map)).expect("lying down");
        assert_eq!(spawn_position(&IRON_GOLEM_FULL, &found), [8.5, 64.05, 5.5]);
    }

    #[test]
    fn an_unloaded_block_matches_only_the_open_cells() {
        let map = owned(&iron_t((5, 70, 5), (1, 0, 0), CARVED_PUMPKIN));
        let unloaded = |pos: BlockPos| {
            if pos == BlockPos::new(4, 68, 5) {
                None
            } else {
                Some(
                    map.get(&pos)
                        .cloned()
                        .unwrap_or_else(|| "minecraft:air".to_owned()),
                )
            }
        };
        assert!(find(&IRON_GOLEM_FULL, BlockPos::new(5, 70, 5), &unloaded).is_none());
    }

    #[test]
    fn a_built_iron_golem_is_player_made() {
        let tag = golem_tag(&IRON_GOLEM_FULL, None);
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some(IRON_GOLEM));
        assert_eq!(tag.get("PlayerCreated"), Some(&Tag::Byte(1)));
    }

    #[test]
    fn two_snow_blocks_under_a_pumpkin_build_a_snow_golem_facing_any_way() {
        for pumpkin in [CARVED_PUMPKIN, JACK_O_LANTERN] {
            let map = world(&[
                ((3, 66, 3), pumpkin),
                ((3, 65, 3), "minecraft:snow_block"),
                ((3, 64, 3), "minecraft:snow_block"),
            ]);
            let found = find(&SNOW_GOLEM_FULL, BlockPos::new(3, 66, 3), &lookup(&map))
                .expect("a snow golem");
            assert_eq!(found.cells().len(), 3);
            assert_eq!(spawn_position(&SNOW_GOLEM_FULL, &found), [3.5, 64.05, 3.5]);
            assert!(find(&IRON_GOLEM_FULL, BlockPos::new(3, 66, 3), &lookup(&map)).is_none());
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let map = world(&[
                ((3, 64, 3), CARVED_PUMPKIN),
                ((3 + dx, 64, 3 + dz), "minecraft:snow_block"),
                ((3 + 2 * dx, 64, 3 + 2 * dz), "minecraft:snow_block"),
            ]);
            let found = find(&SNOW_GOLEM_FULL, BlockPos::new(3, 64, 3), &lookup(&map))
                .expect("lying down along any side");
            let [x, y, z] = spawn_position(&SNOW_GOLEM_FULL, &found);
            assert_eq!(
                (x, y, z),
                (
                    f64::from(3 + 2 * dx) + 0.5,
                    64.05,
                    f64::from(3 + 2 * dz) + 0.5
                )
            );
        }
    }

    #[test]
    fn a_snow_golem_needs_both_snow_blocks_and_is_not_player_made() {
        let map = world(&[
            ((3, 66, 3), CARVED_PUMPKIN),
            ((3, 65, 3), "minecraft:snow_block"),
            ((3, 64, 3), "minecraft:snow"),
        ]);
        assert!(find(&SNOW_GOLEM_FULL, BlockPos::new(3, 66, 3), &lookup(&map)).is_none());
        let tag = golem_tag(&SNOW_GOLEM_FULL, None);
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some(SNOW_GOLEM));
        assert_eq!(tag.get("PlayerCreated"), None);
    }

    #[test]
    fn the_snow_golem_is_tried_before_the_iron_golem() {
        assert_eq!(GOLEMS.map(|g| g.kind), [SNOW_GOLEM, IRON_GOLEM]);
    }

    #[test]
    fn rotating_the_pattern_follows_vec3i_cross() {
        let origin = BlockPos::new(0, 0, 0);
        assert_eq!(
            translate_and_rotate(origin, Direction::North, Direction::Up, 1, 0, 0),
            BlockPos::new(1, 0, 0)
        );
        assert_eq!(
            translate_and_rotate(origin, Direction::North, Direction::Up, 0, 2, 0),
            BlockPos::new(0, -2, 0)
        );
        assert_eq!(
            translate_and_rotate(origin, Direction::East, Direction::Up, 1, 0, 0),
            BlockPos::new(0, 0, 1)
        );
        assert_eq!(
            translate_and_rotate(origin, Direction::Down, Direction::North, 0, 1, 1),
            BlockPos::new(0, -1, 1)
        );
    }
}
