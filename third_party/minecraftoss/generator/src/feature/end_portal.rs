use minecraftoss_core::BlockPos;
use minecraftoss_core::pos::Direction;

pub const FRAME: &str = "minecraft:end_portal_frame";
pub const END_PORTAL: &str = "minecraft:end_portal";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RingCell {
    Unloaded,
    Other,
    Frame { facing: Direction, eye: bool },
}

const PATTERN: [&[u8; 5]; 5] = [b"?vvv?", b">???<", b">???<", b">???<", b"?^^^?"];

fn accepts(symbol: u8, cell: RingCell) -> bool {
    let facing = match symbol {
        b'^' => Direction::South,
        b'>' => Direction::West,
        b'v' => Direction::North,
        b'<' => Direction::East,
        _ => return cell != RingCell::Unloaded,
    };
    cell == RingCell::Frame { facing, eye: true }
}

fn translate(
    origin: BlockPos,
    forward: Direction,
    up: Direction,
    x: i32,
    y: i32,
    z: i32,
) -> BlockPos {
    let f = forward.offset();
    let u = up.offset();
    let r = (
        f.1 * u.2 - f.2 * u.1,
        f.2 * u.0 - f.0 * u.2,
        f.0 * u.1 - f.1 * u.0,
    );
    origin.offset(
        u.0 * -y + r.0 * x + f.0 * z,
        u.1 * -y + r.1 * x + f.1 * z,
        u.2 * -y + r.2 * x + f.2 * z,
    )
}

fn matches(
    cell: &impl Fn(BlockPos) -> RingCell,
    origin: BlockPos,
    forward: Direction,
    up: Direction,
) -> bool {
    (0..5).all(|x| {
        (0..5).all(|y| {
            accepts(
                PATTERN[y as usize][x as usize],
                cell(translate(origin, forward, up, x, y, 0)),
            )
        })
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndPortalRing {
    pub front_top_left: BlockPos,
}

impl EndPortalRing {
    pub fn find(cell: &impl Fn(BlockPos) -> RingCell, pos: BlockPos) -> Option<Self> {
        for dz in 0..5 {
            for dy in 0..5 {
                for dx in 0..5 {
                    let origin = pos.offset(dx, dy, dz);
                    for forward in Direction::ALL {
                        for up in Direction::ALL {
                            if up != forward
                                && up != forward.opposite()
                                && matches(cell, origin, forward, up)
                            {
                                return Some(Self {
                                    front_top_left: origin,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn interior(self) -> [BlockPos; 9] {
        let start = self.front_top_left.offset(-3, 0, -3);
        std::array::from_fn(|i| start.offset(i as i32 / 3, 0, i as i32 % 3))
    }

    pub fn centre(self) -> BlockPos {
        self.front_top_left.offset(-2, 0, -2)
    }
}

pub const BEDROCK: &str = "minecraft:bedrock";

const EXIT_PATTERN: [[&[u8; 7]; 7]; 5] = [
    [
        b"       ", b"       ", b"       ", b"   #   ", b"       ", b"       ", b"       ",
    ],
    [
        b"       ", b"       ", b"       ", b"   #   ", b"       ", b"       ", b"       ",
    ],
    [
        b"       ", b"       ", b"       ", b"   #   ", b"       ", b"       ", b"       ",
    ],
    [
        b"  ###  ", b" #   # ", b"#     #", b"#  #  #", b"#     #", b" #   # ", b"  ###  ",
    ],
    [
        b"       ", b"  ###  ", b" ##### ", b" ##### ", b" ##### ", b"  ###  ", b"       ",
    ],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitPortal {
    pub front_top_left: BlockPos,
    pub forward: Direction,
    pub up: Direction,
}

impl ExitPortal {
    fn matches(
        is_bedrock: &impl Fn(BlockPos) -> bool,
        origin: BlockPos,
        forward: Direction,
        up: Direction,
    ) -> bool {
        (0..7).all(|x| {
            (0..7).all(|y| {
                (0..5).all(|z| {
                    EXIT_PATTERN[z as usize][y as usize][x as usize] != b'#'
                        || is_bedrock(translate(origin, forward, up, x, y, z))
                })
            })
        })
    }

    pub fn find(is_bedrock: &impl Fn(BlockPos) -> bool, pos: BlockPos) -> Option<Self> {
        for dz in 0..7 {
            for dy in 0..7 {
                for dx in 0..7 {
                    let origin = pos.offset(dx, dy, dz);
                    for forward in Direction::ALL {
                        for up in Direction::ALL {
                            if up != forward
                                && up != forward.opposite()
                                && Self::matches(is_bedrock, origin, forward, up)
                            {
                                return Some(Self {
                                    front_top_left: origin,
                                    forward,
                                    up,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn centre(self) -> BlockPos {
        translate(self.front_top_left, self.forward, self.up, 3, 3, 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn ring(centre: BlockPos) -> HashMap<BlockPos, RingCell> {
        let mut cells = HashMap::new();
        for i in -1..=1 {
            let frame = |facing| RingCell::Frame { facing, eye: true };
            cells.insert(centre.offset(i, 0, -2), frame(Direction::South));
            cells.insert(centre.offset(i, 0, 2), frame(Direction::North));
            cells.insert(centre.offset(-2, 0, i), frame(Direction::East));
            cells.insert(centre.offset(2, 0, i), frame(Direction::West));
        }
        cells
    }

    fn lookup(cells: &HashMap<BlockPos, RingCell>) -> impl Fn(BlockPos) -> RingCell + '_ {
        move |pos| cells.get(&pos).copied().unwrap_or(RingCell::Other)
    }

    fn frames(centre: BlockPos) -> Vec<BlockPos> {
        ring(centre).into_keys().collect()
    }

    #[test]
    fn a_full_ring_is_found_from_each_of_its_12_frames() {
        let centre = BlockPos::new(100, 30, -40);
        let cells = ring(centre);
        assert_eq!(cells.len(), 12);
        for frame in frames(centre) {
            let found = EndPortalRing::find(&lookup(&cells), frame)
                .unwrap_or_else(|| panic!("from {frame:?}"));
            assert_eq!(found.front_top_left, centre.offset(2, 0, 2));
            assert_eq!(found.centre(), centre);
        }
    }

    #[test]
    fn the_portal_fills_the_3x3_inside_the_ring() {
        let centre = BlockPos::new(-7, 64, 3);
        let found = EndPortalRing::find(&lookup(&ring(centre)), centre.offset(0, 0, 2)).unwrap();
        let interior = found.interior();
        assert_eq!(interior[0], centre.offset(-1, 0, -1));
        assert_eq!(interior[1], centre.offset(-1, 0, 0));
        assert_eq!(interior[3], centre.offset(0, 0, -1));
        assert_eq!(interior[8], centre.offset(1, 0, 1));
        for x in -1..=1 {
            for z in -1..=1 {
                assert!(interior.contains(&centre.offset(x, 0, z)));
            }
        }
    }

    #[test]
    fn eleven_eyes_find_nothing() {
        let centre = BlockPos::new(0, 40, 0);
        for missing in frames(centre) {
            let mut cells = ring(centre);
            if let Some(RingCell::Frame { facing, .. }) = cells.get(&missing).copied() {
                cells.insert(missing, RingCell::Frame { facing, eye: false });
            }
            for frame in frames(centre) {
                assert_eq!(EndPortalRing::find(&lookup(&cells), frame), None);
            }
        }
    }

    #[test]
    fn one_frame_facing_out_or_sideways_finds_nothing() {
        let centre = BlockPos::new(5, 40, 5);
        for turned in frames(centre) {
            let Some(RingCell::Frame { facing, .. }) = ring(centre).get(&turned).copied() else {
                unreachable!()
            };
            for wrong in [facing.opposite(), facing.clockwise()] {
                let mut cells = ring(centre);
                cells.insert(
                    turned,
                    RingCell::Frame {
                        facing: wrong,
                        eye: true,
                    },
                );
                assert_eq!(
                    EndPortalRing::find(&lookup(&cells), turned),
                    None,
                    "{turned:?} facing {wrong:?}"
                );
            }
        }
    }

    #[test]
    fn a_ring_facing_outward_finds_nothing() {
        let centre = BlockPos::new(5, 40, 5);
        let cells: HashMap<_, _> = ring(centre)
            .into_iter()
            .map(|(pos, cell)| match cell {
                RingCell::Frame { facing, eye } => (
                    pos,
                    RingCell::Frame {
                        facing: facing.opposite(),
                        eye,
                    },
                ),
                other => (pos, other),
            })
            .collect();
        for frame in frames(centre) {
            assert_eq!(EndPortalRing::find(&lookup(&cells), frame), None);
        }
    }

    #[test]
    fn an_unloaded_corner_finds_nothing() {
        let centre = BlockPos::new(0, 40, 0);
        let cells = ring(centre);
        let corner = centre.offset(2, 0, 2);
        let cell = |pos: BlockPos| {
            if pos == corner {
                RingCell::Unloaded
            } else {
                lookup(&cells)(pos)
            }
        };
        assert_eq!(EndPortalRing::find(&cell, centre.offset(0, 0, 2)), None);
    }

    fn podium(centre: BlockPos) -> std::collections::HashSet<BlockPos> {
        let mut bedrock = std::collections::HashSet::new();
        for y in 0..4 {
            bedrock.insert(centre.offset(0, y, 0));
        }
        for x in -4i32..=4 {
            for z in -4i32..=4 {
                let flat = x * x + z * z;
                if flat + 1 < 7 {
                    bedrock.insert(centre.offset(x, -1, z));
                }
                if (7..13).contains(&flat) {
                    bedrock.insert(centre.offset(x, 0, z));
                }
            }
        }
        bedrock
    }

    #[test]
    fn the_exit_portal_is_found_from_its_pillar_top_and_names_its_centre() {
        let centre = BlockPos::new(0, 62, 0);
        let bedrock = podium(centre);
        assert_eq!(bedrock.len(), 4 + 21 + 16);
        let is = |pos: BlockPos| bedrock.contains(&pos);
        let found = ExitPortal::find(&is, centre.offset(0, 3, 0)).expect("found");
        assert_eq!(found.forward, Direction::Down);
        assert_eq!(found.centre(), centre);
        assert_eq!(
            ExitPortal::find(&is, centre.offset(0, 4, 0)),
            None,
            "the pillar top is below the search"
        );
        for y in -3..=3 {
            let below = ExitPortal::find(&is, centre.offset(0, y, 0)).expect("found lower");
            assert_eq!(below.centre(), centre, "from {y}");
        }
    }

    #[test]
    fn a_podium_missing_one_bedrock_is_no_exit_portal() {
        let centre = BlockPos::new(0, 62, 0);
        for missing in podium(centre) {
            let mut bedrock = podium(centre);
            bedrock.remove(&missing);
            let is = |pos: BlockPos| bedrock.contains(&pos);
            assert_eq!(
                ExitPortal::find(&is, centre.offset(0, 3, 0)),
                None,
                "{missing:?}"
            );
        }
    }
}
