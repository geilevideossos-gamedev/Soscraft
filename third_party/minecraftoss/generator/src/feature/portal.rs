use super::Library;
use minecraftoss_core::pos::{Axis, Direction};
use minecraftoss_core::{BlockPos, BlockStateId};

pub const NETHER_PORTAL: &str = "minecraft:nether_portal";

pub const MIN_WIDTH: i32 = 2;
pub const MAX_WIDTH: i32 = 21;
pub const MIN_HEIGHT: i32 = 3;
pub const MAX_HEIGHT: i32 = 21;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Portal,
    Frame,
    Other,
}

impl Cell {
    pub fn of(lib: &Library, state: BlockStateId) -> Self {
        let registries = &lib.registries;
        if registries.block_in_tag(state, lib.update_rules.nether_portal_frame) {
            Self::Frame
        } else if registries.blocks.is_air(state) || registries.block_in_tag(state, lib.tags.fire) {
            Self::Empty
        } else if registries
            .blocks
            .block(registries.blocks.block_of(state))
            .name
            .as_str()
            == NETHER_PORTAL
        {
            Self::Portal
        } else {
            Self::Other
        }
    }

    fn is_empty(self) -> bool {
        matches!(self, Self::Empty | Self::Portal)
    }

    fn is_frame(self) -> bool {
        self == Self::Frame
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalShape {
    pub axis: Axis,
    pub portal_blocks: i32,
    pub right: Direction,
    pub bottom_left: BlockPos,
    pub width: i32,
    pub height: i32,
}

impl PortalShape {
    pub fn find_any(
        cell: &impl Fn(BlockPos) -> Cell,
        pos: BlockPos,
        axis: Axis,
        min_y: i32,
    ) -> Self {
        let right = if axis == Axis::X {
            Direction::West
        } else {
            Direction::South
        };
        let empty = |bottom_left| Self {
            axis,
            portal_blocks: 0,
            right,
            bottom_left,
            width: 0,
            height: 0,
        };
        let Some(bottom_left) = bottom_left(cell, right, pos, min_y) else {
            return empty(pos);
        };
        let width = distance_until_edge_above_frame(cell, bottom_left, right);
        if !(MIN_WIDTH..=MAX_WIDTH).contains(&width) {
            return empty(bottom_left);
        }
        let (height, portal_blocks) = distance_until_top(cell, bottom_left, right, width);
        let closed = (MIN_HEIGHT..=MAX_HEIGHT).contains(&height)
            && (0..width).all(|i| {
                cell(
                    bottom_left
                        .relative(Direction::Up, height)
                        .relative(right, i),
                )
                .is_frame()
            });
        Self {
            axis,
            portal_blocks,
            right,
            bottom_left,
            width,
            height: if closed { height } else { 0 },
        }
    }

    pub fn find_empty(
        cell: &impl Fn(BlockPos) -> Cell,
        pos: BlockPos,
        preferred: Axis,
        min_y: i32,
    ) -> Option<Self> {
        let other = if preferred == Axis::X {
            Axis::Z
        } else {
            Axis::X
        };
        [preferred, other]
            .into_iter()
            .map(|axis| Self::find_any(cell, pos, axis, min_y))
            .find(|shape| shape.is_valid() && shape.portal_blocks == 0)
    }

    pub fn is_valid(&self) -> bool {
        (MIN_WIDTH..=MAX_WIDTH).contains(&self.width)
            && (MIN_HEIGHT..=MAX_HEIGHT).contains(&self.height)
    }

    pub fn is_complete(&self) -> bool {
        self.is_valid() && self.portal_blocks == self.width * self.height
    }

    pub fn portal_state(&self, lib: &Library) -> Option<BlockStateId> {
        let text = format!("{NETHER_PORTAL}[axis={}]", self.axis.name());
        lib.registries.blocks.parse_state(&text).ok()
    }

    pub fn interior(&self) -> Vec<BlockPos> {
        let corner = self
            .bottom_left
            .relative(Direction::Up, self.height - 1)
            .relative(self.right, self.width - 1);
        let (low, high) = (self.bottom_left, corner);
        let mut cells = Vec::with_capacity((self.width * self.height).max(0) as usize);
        for z in low.z.min(high.z)..=low.z.max(high.z) {
            for y in low.y.min(high.y)..=low.y.max(high.y) {
                for x in low.x.min(high.x)..=low.x.max(high.x) {
                    cells.push(BlockPos::new(x, y, z));
                }
            }
        }
        cells
    }
}

fn bottom_left(
    cell: &impl Fn(BlockPos) -> Cell,
    right: Direction,
    start: BlockPos,
    min_y: i32,
) -> Option<BlockPos> {
    let floor = min_y.max(start.y - MAX_HEIGHT);
    let mut pos = start;
    while pos.y > floor && cell(pos.below()).is_empty() {
        pos = pos.below();
    }
    let left = right.opposite();
    let edge = distance_until_edge_above_frame(cell, pos, left) - 1;
    if edge < 0 {
        None
    } else {
        Some(pos.relative(left, edge))
    }
}

fn distance_until_edge_above_frame(
    cell: &impl Fn(BlockPos) -> Cell,
    pos: BlockPos,
    direction: Direction,
) -> i32 {
    for width in 0..=MAX_WIDTH {
        let at = pos.relative(direction, width);
        let here = cell(at);
        if !here.is_empty() {
            return if here.is_frame() { width } else { 0 };
        }
        if !cell(at.below()).is_frame() {
            return 0;
        }
    }
    0
}

fn distance_until_top(
    cell: &impl Fn(BlockPos) -> Cell,
    bottom_left: BlockPos,
    right: Direction,
    width: i32,
) -> (i32, i32) {
    let mut portal_blocks = 0;
    for height in 0..MAX_HEIGHT {
        let row = bottom_left.relative(Direction::Up, height);
        if !cell(row.relative(right, -1)).is_frame() || !cell(row.relative(right, width)).is_frame()
        {
            return (height, portal_blocks);
        }
        for i in 0..width {
            let here = cell(row.relative(right, i));
            if !here.is_empty() {
                return (height, portal_blocks);
            }
            if here == Cell::Portal {
                portal_blocks += 1;
            }
        }
    }
    (MAX_HEIGHT, portal_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Grid(HashMap<BlockPos, Cell>);

    impl Grid {
        fn new() -> Self {
            Self(HashMap::new())
        }

        fn frame(
            mut self,
            axis: Axis,
            origin: BlockPos,
            width: i32,
            height: i32,
            corners: bool,
        ) -> Self {
            let step = |i: i32| match axis {
                Axis::X => BlockPos::new(origin.x + i, origin.y, origin.z),
                _ => BlockPos::new(origin.x, origin.y, origin.z + i),
            };
            for i in 0..=width + 1 {
                for y in 0..=height + 1 {
                    let edge_x = i == 0 || i == width + 1;
                    let edge_y = y == 0 || y == height + 1;
                    if (edge_x || edge_y) && (corners || !(edge_x && edge_y)) {
                        let at = step(i);
                        self.0
                            .insert(BlockPos::new(at.x, at.y + y, at.z), Cell::Frame);
                    }
                }
            }
            self
        }

        fn set(mut self, pos: BlockPos, cell: Cell) -> Self {
            self.0.insert(pos, cell);
            self
        }

        fn cell(&self) -> impl Fn(BlockPos) -> Cell + '_ {
            |pos| self.0.get(&pos).copied().unwrap_or(Cell::Empty)
        }
    }

    const ORIGIN: BlockPos = BlockPos::new(0, 64, 0);
    const MIN_Y: i32 = -64;

    #[test]
    fn the_smallest_frame_opens_two_by_three() {
        let grid = Grid::new().frame(Axis::X, ORIGIN, 2, 3, true);
        let shape =
            PortalShape::find_empty(&grid.cell(), BlockPos::new(1, 65, 0), Axis::X, MIN_Y).unwrap();
        assert_eq!((shape.axis, shape.width, shape.height), (Axis::X, 2, 3));
        assert_eq!(shape.bottom_left, BlockPos::new(2, 65, 0));
        let mut cells = shape.interior();
        cells.sort();
        let mut expected: Vec<BlockPos> = (65..68)
            .flat_map(|y| [BlockPos::new(1, y, 0), BlockPos::new(2, y, 0)])
            .collect();
        expected.sort();
        assert_eq!(cells, expected);
    }

    #[test]
    fn interior_runs_x_then_y_then_z_like_between_closed() {
        let grid = Grid::new().frame(Axis::Z, ORIGIN, 2, 3, true);
        let shape =
            PortalShape::find_empty(&grid.cell(), BlockPos::new(0, 66, 1), Axis::X, MIN_Y).unwrap();
        assert_eq!(shape.axis, Axis::Z);
        assert_eq!(shape.bottom_left, BlockPos::new(0, 65, 1));
        assert_eq!(
            shape.interior(),
            vec![
                BlockPos::new(0, 65, 1),
                BlockPos::new(0, 66, 1),
                BlockPos::new(0, 67, 1),
                BlockPos::new(0, 65, 2),
                BlockPos::new(0, 66, 2),
                BlockPos::new(0, 67, 2),
            ]
        );
    }

    #[test]
    fn the_largest_frame_opens_and_one_wider_or_shorter_does_not() {
        let grid = Grid::new().frame(Axis::X, ORIGIN, 21, 21, true);
        let shape = PortalShape::find_empty(&grid.cell(), BlockPos::new(11, 70, 0), Axis::X, MIN_Y)
            .unwrap();
        assert_eq!((shape.width, shape.height), (21, 21));
        let wide = Grid::new().frame(Axis::X, ORIGIN, 22, 3, true);
        assert_eq!(
            PortalShape::find_empty(&wide.cell(), BlockPos::new(5, 65, 0), Axis::X, MIN_Y),
            None
        );
        let tall = Grid::new().frame(Axis::X, ORIGIN, 2, 22, true);
        assert_eq!(
            PortalShape::find_empty(&tall.cell(), BlockPos::new(1, 65, 0), Axis::X, MIN_Y),
            None
        );
        let flat = Grid::new().frame(Axis::X, ORIGIN, 4, 2, true);
        assert_eq!(
            PortalShape::find_empty(&flat.cell(), BlockPos::new(1, 65, 0), Axis::X, MIN_Y),
            None
        );
        let narrow = Grid::new().frame(Axis::X, ORIGIN, 1, 3, true);
        assert_eq!(
            PortalShape::find_empty(&narrow.cell(), BlockPos::new(1, 65, 0), Axis::X, MIN_Y),
            None
        );
    }

    #[test]
    fn corners_are_optional() {
        let grid = Grid::new().frame(Axis::Z, ORIGIN, 4, 5, false);
        let shape =
            PortalShape::find_empty(&grid.cell(), BlockPos::new(0, 67, 3), Axis::X, MIN_Y).unwrap();
        assert_eq!((shape.axis, shape.width, shape.height), (Axis::Z, 4, 5));
        assert_eq!(shape.interior().len(), 20);
    }

    #[test]
    fn a_gap_or_a_foreign_block_keeps_it_closed() {
        let grid = Grid::new()
            .frame(Axis::X, ORIGIN, 2, 3, true)
            .set(BlockPos::new(0, 66, 0), Cell::Empty);
        assert_eq!(
            PortalShape::find_empty(&grid.cell(), BlockPos::new(1, 65, 0), Axis::X, MIN_Y),
            None
        );
        let grid = Grid::new()
            .frame(Axis::X, ORIGIN, 2, 3, true)
            .set(BlockPos::new(0, 66, 0), Cell::Other);
        assert_eq!(
            PortalShape::find_empty(&grid.cell(), BlockPos::new(1, 65, 0), Axis::X, MIN_Y),
            None
        );
        let grid = Grid::new()
            .frame(Axis::X, ORIGIN, 2, 3, true)
            .set(BlockPos::new(2, 67, 0), Cell::Other);
        assert_eq!(
            PortalShape::find_empty(&grid.cell(), BlockPos::new(1, 65, 0), Axis::X, MIN_Y),
            None
        );
    }

    #[test]
    fn fire_and_portal_count_as_empty_but_a_partial_sheet_is_not_empty() {
        let grid = Grid::new()
            .frame(Axis::X, ORIGIN, 2, 3, true)
            .set(BlockPos::new(1, 65, 0), Cell::Portal);
        let any = PortalShape::find_any(&grid.cell(), BlockPos::new(2, 66, 0), Axis::X, MIN_Y);
        assert!(any.is_valid());
        assert_eq!(any.portal_blocks, 1);
        assert!(!any.is_complete());
        assert_eq!(
            PortalShape::find_empty(&grid.cell(), BlockPos::new(2, 66, 0), Axis::X, MIN_Y),
            None
        );
    }

    #[test]
    fn a_full_sheet_is_complete_on_its_own_axis_only() {
        let mut grid = Grid::new().frame(Axis::X, ORIGIN, 3, 4, true);
        for x in 1..=3 {
            for y in 65..69 {
                grid = grid.set(BlockPos::new(x, y, 0), Cell::Portal);
            }
        }
        let shape = PortalShape::find_any(&grid.cell(), BlockPos::new(2, 67, 0), Axis::X, MIN_Y);
        assert!(shape.is_complete());
        assert_eq!(shape.portal_blocks, 12);
        assert!(
            !PortalShape::find_any(&grid.cell(), BlockPos::new(2, 67, 0), Axis::Z, MIN_Y)
                .is_complete()
        );
        let broken = grid.set(BlockPos::new(4, 66, 0), Cell::Empty);
        assert!(
            !PortalShape::find_any(&broken.cell(), BlockPos::new(2, 67, 0), Axis::X, MIN_Y)
                .is_complete()
        );
    }

    #[test]
    fn a_frame_floating_over_the_floor_needs_its_bottom_row() {
        let grid = Grid::new()
            .frame(Axis::X, ORIGIN, 2, 3, true)
            .set(BlockPos::new(2, 64, 0), Cell::Empty);
        assert_eq!(
            PortalShape::find_empty(&grid.cell(), BlockPos::new(1, 67, 0), Axis::X, MIN_Y),
            None
        );
    }
}
