use minecraftoss_core::block::FaceShape;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::tags::TagId;
use minecraftoss_core::{BlockStateId, Registries};
use std::collections::HashMap;

const EMPTY: u16 = 0;
const FULL: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkipRendering {
    Never,
    SameBlock,
    SameBlockVertically,
    Bars { tagged: bool, sides: u8 },
}

pub(crate) struct StateFaces {
    pub(crate) shapes: [u16; 6],
    pub(crate) block: u16,
    pub(crate) skip: SkipRendering,
}

pub(crate) struct FaceCulling {
    faces: Vec<[u16; 6]>,
    blocks: Vec<u16>,
    skip: Vec<SkipRendering>,
    sizes: [usize; 3],
    covered: [Vec<bool>; 3],
}

impl FaceCulling {
    pub(crate) fn new(registries: &Registries) -> Self {
        let blocks = &registries.blocks;
        let bars = registries.block_tags.id("minecraft:bars");
        let states = (0..blocks.state_count()).map(|index| {
            let state = BlockStateId(index as u16);
            StateFaces {
                shapes: Direction::ALL.map(|side| blocks.face_shape_id(state, side)),
                block: blocks.block_of(state).0,
                skip: skip_rendering(registries, state, bars),
            }
        });
        Self::build(|id| blocks.shape(id), states)
    }

    pub(crate) fn build<'s>(
        shape: impl Fn(u16) -> &'s FaceShape,
        states: impl Iterator<Item = StateFaces>,
    ) -> Self {
        let mut dense: [HashMap<u16, u16>; 3] = Default::default();
        let mut rects: [Vec<Vec<[f64; 4]>>; 3] =
            std::array::from_fn(|_| vec![Vec::new(), vec![[0.0, 0.0, 1.0, 1.0]]]);
        let (mut faces, mut blocks, mut skip) = (Vec::new(), Vec::new(), Vec::new());
        for state in states {
            let mut ids = [EMPTY; 6];
            for (side, (id, catalog)) in ids.iter_mut().zip(state.shapes).enumerate() {
                let axis = side / 2;
                *id = match shape(catalog) {
                    FaceShape::Empty => EMPTY,
                    FaceShape::Full => FULL,
                    boxes => *dense[axis].entry(catalog).or_insert_with(|| {
                        rects[axis].push(face_rects(boxes, axis));
                        (rects[axis].len() - 1) as u16
                    }),
                };
            }
            faces.push(ids);
            blocks.push(state.block);
            skip.push(state.skip);
        }
        let sizes = rects.each_ref().map(Vec::len);
        let covered = std::array::from_fn(|axis| {
            let shapes = &rects[axis];
            shapes
                .iter()
                .flat_map(|ours| shapes.iter().map(move |theirs| rects_cover(ours, theirs)))
                .collect()
        });
        Self {
            faces,
            blocks,
            skip,
            sizes,
            covered,
        }
    }

    pub(crate) fn hidden(
        &self,
        own: BlockStateId,
        neighbor: BlockStateId,
        side: Direction,
    ) -> bool {
        let theirs = self.faces[usize::from(neighbor.0)][side.opposite().index()];
        if theirs == FULL || self.skips(own, neighbor, side) {
            return true;
        }
        let ours = self.faces[usize::from(own.0)][side.index()];
        if theirs == EMPTY || ours == EMPTY {
            return false;
        }
        let axis = side.index() / 2;
        self.covered[axis][usize::from(ours) * self.sizes[axis] + usize::from(theirs)]
    }

    fn skips(&self, own: BlockStateId, neighbor: BlockStateId, side: Direction) -> bool {
        let same = || self.blocks[usize::from(own.0)] == self.blocks[usize::from(neighbor.0)];
        match self.skip[usize::from(own.0)] {
            SkipRendering::Never => false,
            SkipRendering::SameBlock => same(),
            SkipRendering::SameBlockVertically => !side.is_horizontal() && same(),
            SkipRendering::Bars { tagged, sides } => {
                let theirs = match self.skip[usize::from(neighbor.0)] {
                    SkipRendering::Bars {
                        tagged: their_tag,
                        sides: their_sides,
                    } if same() || (tagged && their_tag && side.is_horizontal()) => their_sides,
                    _ => return false,
                };
                !side.is_horizontal()
                    || (sides & 1 << side.index() != 0
                        && theirs & 1 << side.opposite().index() != 0)
            }
        }
    }
}

fn skip_rendering(
    registries: &Registries,
    state: BlockStateId,
    bars: Option<TagId>,
) -> SkipRendering {
    let blocks = &registries.blocks;
    let block = blocks.block(blocks.block_of(state));
    if block.is_a("HalfTransparentBlock") || block.is_a("PowderSnowBlock") {
        SkipRendering::SameBlock
    } else if block.is_a("MangroveRootsBlock") {
        SkipRendering::SameBlockVertically
    } else if block.is_a("IronBarsBlock") {
        let sides = Direction::HORIZONTAL
            .iter()
            .filter(|side| blocks.property(state, side.name()) == Some("true"))
            .fold(0, |sides, side| sides | 1 << side.index());
        SkipRendering::Bars {
            tagged: bars.is_some_and(|tag| registries.block_in_tag(state, tag)),
            sides,
        }
    } else {
        SkipRendering::Never
    }
}

fn face_rects(shape: &FaceShape, axis: usize) -> Vec<[f64; 4]> {
    let (u, v) = [(0, 2), (0, 1), (1, 2)][axis];
    match shape {
        FaceShape::Empty => Vec::new(),
        FaceShape::Full => vec![[0.0, 0.0, 1.0, 1.0]],
        FaceShape::Boxes(boxes) => boxes
            .iter()
            .map(|b| [b[u], b[v], b[u + 3], b[v + 3]])
            .collect(),
    }
}

fn rects_cover(inner: &[[f64; 4]], outer: &[[f64; 4]]) -> bool {
    let cuts = |low: usize| {
        let mut cuts: Vec<f64> = inner
            .iter()
            .chain(outer)
            .flat_map(|r| [r[low], r[low + 2]])
            .collect();
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        cuts
    };
    let (us, vs) = (cuts(0), cuts(1));
    let inside = |rects: &[[f64; 4]], u: f64, v: f64| {
        rects
            .iter()
            .any(|r| r[0] < u && u < r[2] && r[1] < v && v < r[3])
    };
    us.windows(2).all(|du| {
        vs.windows(2).all(|dv| {
            let (u, v) = ((du[0] + du[1]) / 2.0, (dv[0] + dv[1]) / 2.0);
            !inside(inner, u, v) || inside(outer, u, v)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Direction::{Down, East, North, South, Up, West};

    const AIR: BlockStateId = BlockStateId(0);
    const STONE: BlockStateId = BlockStateId(1);
    const CACTUS: BlockStateId = BlockStateId(2);
    const GLASS: BlockStateId = BlockStateId(3);
    const BOTTOM_SLAB: BlockStateId = BlockStateId(4);
    const TOP_SLAB: BlockStateId = BlockStateId(5);
    const STACKED_HALVES: BlockStateId = BlockStateId(6);
    const ROOTS: BlockStateId = BlockStateId(7);
    const IRON_BARS_EAST_WEST: BlockStateId = BlockStateId(8);
    const IRON_BARS_NORTH: BlockStateId = BlockStateId(9);
    const COPPER_BARS_WEST: BlockStateId = BlockStateId(10);
    const PANE_EAST_WEST: BlockStateId = BlockStateId(11);
    const OTHER_PANE_EAST_WEST: BlockStateId = BlockStateId(12);

    fn synthetic() -> FaceCulling {
        let px = 1.0 / 16.0;
        let shapes = [
            FaceShape::Empty,
            FaceShape::Full,
            FaceShape::Boxes(Box::new([[px, 0.0, px, 1.0 - px, 1.0, 1.0 - px]])),
            FaceShape::Boxes(Box::new([[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]])),
            FaceShape::Boxes(Box::new([[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]])),
            FaceShape::Boxes(Box::new([[0.0, 0.5, 0.0, 1.0, 1.0, 1.0]])),
            FaceShape::Boxes(Box::new([
                [0.0, 0.0, 0.0, 1.0, 0.5, 1.0],
                [0.0, 0.5, 0.0, 1.0, 1.0, 1.0],
            ])),
        ];
        let (empty, full, cactus, slab_face, low, high, halves) = (0, 1, 2, 3, 4, 5, 6);
        let bars = |block: u16, tagged: bool, connected: &[Direction]| StateFaces {
            shapes: [empty; 6],
            block,
            skip: SkipRendering::Bars {
                tagged,
                sides: connected
                    .iter()
                    .fold(0, |sides, side| sides | 1 << side.index()),
            },
        };
        let plain = |shapes: [u16; 6], block: u16, skip: SkipRendering| StateFaces {
            shapes,
            block,
            skip,
        };
        let states = vec![
            plain([empty; 6], 0, SkipRendering::Never),
            plain([full; 6], 1, SkipRendering::Never),
            plain(
                [cactus, cactus, empty, empty, empty, empty],
                2,
                SkipRendering::Never,
            ),
            plain([empty; 6], 3, SkipRendering::SameBlock),
            plain(
                [slab_face, empty, low, low, low, low],
                4,
                SkipRendering::Never,
            ),
            plain(
                [empty, slab_face, high, high, high, high],
                4,
                SkipRendering::Never,
            ),
            plain([halves; 6], 5, SkipRendering::Never),
            plain([empty; 6], 6, SkipRendering::SameBlockVertically),
            bars(7, true, &[East, West]),
            bars(7, true, &[North]),
            bars(8, true, &[West]),
            bars(9, false, &[East, West]),
            bars(10, false, &[East, West]),
        ];
        FaceCulling::build(|id| &shapes[usize::from(id)], states.into_iter())
    }

    #[test]
    fn a_cactus_does_not_hide_the_sand_under_it() {
        let culling = synthetic();
        assert!(!culling.hidden(STONE, CACTUS, Up));
        assert!(!culling.hidden(STONE, CACTUS, Down));
        assert!(!culling.hidden(STONE, CACTUS, East));
        assert!(culling.hidden(CACTUS, CACTUS, Up));
        assert!(culling.hidden(CACTUS, STONE, Down));
    }

    #[test]
    fn empty_faces_never_hide_and_full_faces_always_do() {
        let culling = synthetic();
        for side in Direction::ALL {
            assert!(!culling.hidden(STONE, AIR, side));
            assert!(!culling.hidden(STONE, GLASS, side));
            assert!(culling.hidden(GLASS, STONE, side));
            assert!(culling.hidden(CACTUS, STONE, side));
            assert!(!culling.hidden(GLASS, AIR, side));
        }
    }

    #[test]
    fn faces_hide_only_behind_a_covering_neighbour_face() {
        let culling = synthetic();
        assert!(culling.hidden(STONE, BOTTOM_SLAB, Up));
        assert!(!culling.hidden(STONE, BOTTOM_SLAB, Down));
        assert!(!culling.hidden(STONE, BOTTOM_SLAB, East));
        assert!(culling.hidden(BOTTOM_SLAB, BOTTOM_SLAB, East));
        assert!(!culling.hidden(BOTTOM_SLAB, TOP_SLAB, East));
        assert!(culling.hidden(BOTTOM_SLAB, STONE, East));
        assert!(culling.hidden(BOTTOM_SLAB, STACKED_HALVES, North));
        assert!(culling.hidden(STONE, STACKED_HALVES, South));
        assert!(culling.hidden(STACKED_HALVES, BOTTOM_SLAB, Up));
        assert!(!culling.hidden(STACKED_HALVES, BOTTOM_SLAB, West));
    }

    #[test]
    fn transparent_blocks_hide_only_their_own_kind() {
        let culling = synthetic();
        assert!(culling.hidden(GLASS, GLASS, East));
        assert!(!culling.hidden(GLASS, ROOTS, East));
        assert!(culling.hidden(ROOTS, ROOTS, Up));
        assert!(culling.hidden(ROOTS, ROOTS, Down));
        assert!(!culling.hidden(ROOTS, ROOTS, East));
        assert!(!culling.hidden(ROOTS, GLASS, Up));
    }

    #[test]
    fn bars_and_panes_hide_faces_they_join() {
        let culling = synthetic();
        assert!(culling.hidden(IRON_BARS_EAST_WEST, IRON_BARS_EAST_WEST, East));
        assert!(culling.hidden(IRON_BARS_EAST_WEST, IRON_BARS_NORTH, Up));
        assert!(!culling.hidden(IRON_BARS_EAST_WEST, IRON_BARS_NORTH, East));
        assert!(!culling.hidden(IRON_BARS_NORTH, IRON_BARS_EAST_WEST, North));
        assert!(culling.hidden(IRON_BARS_EAST_WEST, COPPER_BARS_WEST, East));
        assert!(!culling.hidden(IRON_BARS_EAST_WEST, COPPER_BARS_WEST, West));
        assert!(!culling.hidden(IRON_BARS_EAST_WEST, COPPER_BARS_WEST, Up));
        assert!(culling.hidden(PANE_EAST_WEST, PANE_EAST_WEST, West));
        assert!(culling.hidden(PANE_EAST_WEST, PANE_EAST_WEST, Down));
        assert!(!culling.hidden(PANE_EAST_WEST, OTHER_PANE_EAST_WEST, East));
        assert!(!culling.hidden(PANE_EAST_WEST, IRON_BARS_EAST_WEST, East));
        assert!(!culling.hidden(IRON_BARS_EAST_WEST, PANE_EAST_WEST, East));
    }

    #[test]
    fn catalog_faces_follow_vanilla_occlusion() {
        use minecraftoss_core::registries::DataPaths;
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let culling = FaceCulling::new(&registries);
        let state = |text: &str| registries.blocks.parse_state(text).unwrap();
        let hidden =
            |own: &str, neighbor: &str, side| culling.hidden(state(own), state(neighbor), side);
        assert!(!hidden("minecraft:sand", "minecraft:cactus", Up));
        assert!(!hidden("minecraft:stone", "minecraft:cactus", Down));
        assert!(hidden("minecraft:cactus", "minecraft:cactus", Up));
        assert!(hidden("minecraft:cactus", "minecraft:sand", Down));
        for (open, hides_its_own_kind) in [
            ("minecraft:ice", true),
            ("minecraft:frosted_ice", true),
            ("minecraft:slime_block", true),
            ("minecraft:honey_block", true),
            ("minecraft:copper_grate", true),
            ("minecraft:waxed_oxidized_copper_grate", true),
            ("minecraft:glass", true),
            ("minecraft:spawner", false),
            ("minecraft:trial_spawner", false),
            ("minecraft:vault", false),
            ("minecraft:beacon", false),
            ("minecraft:oak_leaves", false),
        ] {
            for side in Direction::ALL {
                assert!(!hidden("minecraft:stone", open, side), "{open} {side:?}");
            }
            assert_eq!(hidden(open, open, East), hides_its_own_kind, "{open}");
            assert!(hidden(open, "minecraft:stone", West), "{open}");
        }
        assert!(hidden("minecraft:stone", "minecraft:blue_ice", East));
        assert!(hidden(
            "minecraft:stone",
            "minecraft:stone_slab[type=bottom]",
            Up
        ));
        assert!(!hidden(
            "minecraft:stone",
            "minecraft:stone_slab[type=bottom]",
            East
        ));
        assert!(hidden(
            "minecraft:stone_slab[type=bottom]",
            "minecraft:stone_slab[type=bottom]",
            East
        ));
        assert!(!hidden(
            "minecraft:stone_slab[type=bottom]",
            "minecraft:stone_slab[type=top]",
            East
        ));
        assert!(hidden("minecraft:stone", "minecraft:stonecutter", Up));
        assert!(!hidden("minecraft:stone", "minecraft:stonecutter", Down));
        assert!(hidden("minecraft:powder_snow", "minecraft:powder_snow", Up));
        assert!(hidden(
            "minecraft:mangrove_roots",
            "minecraft:mangrove_roots",
            Up
        ));
        assert!(!hidden(
            "minecraft:mangrove_roots",
            "minecraft:mangrove_roots",
            East
        ));
        let east_west = "[east=true,north=false,south=false,waterlogged=false,west=true]";
        assert!(hidden(
            &format!("minecraft:iron_bars{east_west}"),
            &format!("minecraft:copper_bars{east_west}"),
            East
        ));
        assert!(!hidden(
            &format!("minecraft:iron_bars{east_west}"),
            &format!("minecraft:copper_bars{east_west}"),
            Up
        ));
        assert!(hidden(
            &format!("minecraft:glass_pane{east_west}"),
            &format!("minecraft:glass_pane{east_west}"),
            East
        ));
        assert!(!hidden(
            &format!("minecraft:glass_pane{east_west}"),
            &format!("minecraft:red_stained_glass_pane{east_west}"),
            East
        ));
    }
}
