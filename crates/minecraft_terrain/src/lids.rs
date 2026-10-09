use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use crate::containers::{Opened, chest_partner, is_shulker_box};
use crate::model::ResolvedModel;
use crate::scene::{Block, BlockPos, Scene};
use crate::sections::SectionPos;

pub const SET_OPEN_COUNT: u8 = 1;
const STEP: f32 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LidKind {
    Shulker,
    Chest,
}

impl LidKind {
    pub fn of(block: &Block) -> Option<Self> {
        if block.id.namespace != "minecraft" {
            None
        } else if is_shulker_box(&block.id.path) {
            Some(Self::Shulker)
        } else if crate::special_models::is_chest(block) {
            Some(Self::Chest)
        } else {
            None
        }
    }
}

pub fn lid_model(block: &Block, progress: f32) -> Result<Option<ResolvedModel>> {
    match LidKind::of(block) {
        Some(LidKind::Shulker) => {
            crate::special_models::shulker_box_model(block, progress).map(Some)
        }
        Some(LidKind::Chest) => crate::model::opened_chest_model(block, progress).map(Some),
        None => Ok(None),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShulkerStatus {
    #[default]
    Closed,
    Opening,
    Opened,
    Closing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShulkerLid {
    pub status: ShulkerStatus,
    progress: f32,
    old: f32,
}

impl ShulkerLid {
    pub fn set_open_count(&mut self, count: u32) {
        if count == 0 {
            self.status = ShulkerStatus::Closing;
        }
        if count == 1 {
            self.status = ShulkerStatus::Opening;
        }
    }

    pub fn tick(&mut self) {
        self.old = self.progress;
        match self.status {
            ShulkerStatus::Closed => self.progress = 0.0,
            ShulkerStatus::Opening => {
                self.progress += STEP;
                if self.progress >= 1.0 {
                    self.status = ShulkerStatus::Opened;
                    self.progress = 1.0;
                }
            }
            ShulkerStatus::Opened => self.progress = 1.0,
            ShulkerStatus::Closing => {
                self.progress -= STEP;
                if self.progress <= 0.0 {
                    self.status = ShulkerStatus::Closed;
                    self.progress = 0.0;
                }
            }
        }
    }

    pub fn progress(&self, partial: f32) -> f32 {
        self.old + partial * (self.progress - self.old)
    }

    fn at_rest(&self) -> bool {
        self.status == ShulkerStatus::Closed && self.progress == 0.0 && self.old == 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChestLid {
    pub open: bool,
    openness: f32,
    old: f32,
}

impl ChestLid {
    pub fn set_open_count(&mut self, count: u32) {
        self.open = count > 0;
    }

    pub fn tick(&mut self) {
        self.old = self.openness;
        if !self.open && self.openness > 0.0 {
            self.openness = (self.openness - STEP).max(0.0);
        } else if self.open && self.openness < 1.0 {
            self.openness = (self.openness + STEP).min(1.0);
        }
    }

    pub fn openness(&self, partial: f32) -> f32 {
        self.old + partial * (self.openness - self.old)
    }

    fn at_rest(&self) -> bool {
        !self.open && self.openness == 0.0 && self.old == 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lid {
    Shulker(ShulkerLid),
    Chest(ChestLid),
}

impl Lid {
    pub fn new(kind: LidKind) -> Self {
        match kind {
            LidKind::Shulker => Self::Shulker(ShulkerLid::default()),
            LidKind::Chest => Self::Chest(ChestLid::default()),
        }
    }

    pub fn kind(&self) -> LidKind {
        match self {
            Self::Shulker(_) => LidKind::Shulker,
            Self::Chest(_) => LidKind::Chest,
        }
    }

    pub fn set_open_count(&mut self, count: u32) {
        match self {
            Self::Shulker(lid) => lid.set_open_count(count),
            Self::Chest(lid) => lid.set_open_count(count),
        }
    }

    pub fn tick(&mut self) {
        match self {
            Self::Shulker(lid) => lid.tick(),
            Self::Chest(lid) => lid.tick(),
        }
    }

    pub fn progress(&self, partial: f32) -> f32 {
        match self {
            Self::Shulker(lid) => lid.progress(partial),
            Self::Chest(lid) => lid.openness(partial),
        }
    }

    pub fn opening(&self) -> bool {
        match self {
            Self::Shulker(lid) => {
                matches!(lid.status, ShulkerStatus::Opening | ShulkerStatus::Opened)
            }
            Self::Chest(lid) => lid.open,
        }
    }

    pub fn at_rest(&self) -> bool {
        match self {
            Self::Shulker(lid) => lid.at_rest(),
            Self::Chest(lid) => lid.at_rest(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drawn {
    Sending(bool),
    Rebuilding(bool),
    Apart,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Entry {
    lid: Lid,
    drawn: Drawn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PosedLid {
    pub cell: BlockPos,
    pub block: Block,
    pub progress: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Lids {
    entries: BTreeMap<BlockPos, Entry>,
}

impl Lids {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn block_event<S: Scene>(&mut self, scene: &S, pos: BlockPos, id: u8, param: u32) {
        if id != SET_OPEN_COUNT {
            return;
        }
        let Some(kind) = scene.block(pos).and_then(LidKind::of) else {
            return;
        };
        if let Some(entry) = self
            .entries
            .get_mut(&pos)
            .filter(|entry| entry.lid.kind() == kind)
        {
            entry.lid.set_open_count(param);
            if entry.lid.opening() {
                entry.drawn = match entry.drawn {
                    Drawn::Sending(false) => Drawn::Apart,
                    Drawn::Rebuilding(false) => Drawn::Sending(true),
                    drawn => drawn,
                };
            }
            return;
        }
        let mut lid = Lid::new(kind);
        lid.set_open_count(param);
        if lid.opening() {
            let drawn = Drawn::Sending(true);
            self.entries.insert(pos, Entry { lid, drawn });
        }
    }

    pub fn tick(&mut self) {
        for entry in self.entries.values_mut() {
            if entry.drawn == Drawn::Apart {
                entry.lid.tick();
                if entry.lid.at_rest() {
                    entry.drawn = Drawn::Sending(false);
                }
            }
        }
    }

    pub fn take_apart_changes(&mut self) -> Vec<(BlockPos, bool)> {
        let mut changes = Vec::new();
        for (&pos, entry) in &mut self.entries {
            if let Drawn::Sending(apart) = entry.drawn {
                entry.drawn = Drawn::Rebuilding(apart);
                changes.push((pos, apart));
            }
        }
        changes
    }

    pub fn settle<S: Scene>(&mut self, scene: &S, cell_settled: impl Fn(BlockPos) -> bool) {
        self.entries.retain(|&pos, entry| {
            let kept = scene.block(pos).and_then(LidKind::of) == Some(entry.lid.kind());
            match entry.drawn {
                Drawn::Sending(true) if !kept => return false,
                Drawn::Rebuilding(true) | Drawn::Apart if !kept => {
                    entry.drawn = Drawn::Sending(false);
                }
                Drawn::Rebuilding(apart) if cell_settled(pos) => {
                    if !apart {
                        return false;
                    }
                    entry.drawn = Drawn::Apart;
                }
                _ => {}
            }
            true
        });
    }

    pub fn posed<S: Scene>(&self, scene: &S, partial: f32) -> Vec<PosedLid> {
        self.entries
            .iter()
            .filter_map(|(&cell, entry)| {
                let block = scene.block(cell)?;
                let kind = LidKind::of(block).filter(|&kind| kind == entry.lid.kind())?;
                let own = entry.lid.progress(partial);
                let partner = match kind {
                    LidKind::Chest => {
                        paired(scene, cell, block).and_then(|at| self.entries.get(&at))
                    }
                    LidKind::Shulker => None,
                };
                let progress = partner.map_or(own, |other| own.max(other.lid.progress(partial)));
                Some(PosedLid {
                    cell,
                    block: block.clone(),
                    progress,
                })
            })
            .collect()
    }

    pub fn lid_closed(&self, pos: BlockPos) -> bool {
        self.entries.get(&pos).is_none_or(|entry| match entry.lid {
            Lid::Shulker(lid) => lid.status == ShulkerStatus::Closed,
            Lid::Chest(lid) => lid.at_rest(),
        })
    }
}

fn paired<S: Scene>(scene: &S, cell: BlockPos, block: &Block) -> Option<BlockPos> {
    let partner = chest_partner(cell, block)?;
    let opened = Opened::new(cell, block, scene.block(partner))?;
    opened.cells.into_iter().find(|&other| other != cell)
}

#[derive(Clone, Debug, Default)]
pub struct ApartCells {
    cells: BTreeSet<BlockPos>,
}

impl ApartCells {
    pub fn set(&mut self, pos: BlockPos, apart: bool) -> Option<SectionPos> {
        let changed = if apart {
            self.cells.insert(pos)
        } else {
            self.cells.remove(&pos)
        };
        changed.then_some(section_of(pos))
    }

    pub fn contains(&self, pos: BlockPos) -> bool {
        self.cells.contains(&pos)
    }

    pub fn in_section(&self, section: SectionPos) -> Vec<BlockPos> {
        let x = section.0 * 16;
        self.cells
            .range((x, i32::MIN, i32::MIN)..(x + 16, i32::MIN, i32::MIN))
            .filter(|&&pos| section_of(pos) == section)
            .copied()
            .collect()
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }
}

fn section_of((x, y, z): BlockPos) -> SectionPos {
    (x >> 4, y >> 4, z >> 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::HandcraftedScene;

    fn ticks_until(lid: &mut Lid, done: impl Fn(&Lid) -> bool) -> u32 {
        let mut ticks = 0;
        while !done(lid) {
            lid.tick();
            ticks += 1;
            assert!(ticks < 100);
        }
        ticks
    }

    fn shulker_status(lid: &Lid) -> ShulkerStatus {
        match lid {
            Lid::Shulker(lid) => lid.status,
            Lid::Chest(_) => unreachable!(),
        }
    }

    fn box_scene(cell: BlockPos, id: &str) -> HandcraftedScene {
        let mut scene = HandcraftedScene::default();
        scene.set(cell, Some(Block::new(id).with("facing", "up")));
        scene
    }

    fn opened_lids(scene: &HandcraftedScene, cells: &[BlockPos]) -> Lids {
        let mut lids = Lids::default();
        for &cell in cells {
            lids.block_event(scene, cell, SET_OPEN_COUNT, 1);
        }
        lids.take_apart_changes();
        lids.settle(scene, |_| true);
        lids
    }

    #[test]
    fn a_shulker_lid_opens_in_ten_ticks_and_closes_in_ten() {
        let mut lid = Lid::new(LidKind::Shulker);
        lid.set_open_count(1);
        let opened = |lid: &Lid| shulker_status(lid) == ShulkerStatus::Opened;
        assert_eq!(ticks_until(&mut lid, opened), 10);
        assert_eq!(lid.progress(1.0), 1.0);
        lid.set_open_count(0);
        let closed = |lid: &Lid| shulker_status(lid) == ShulkerStatus::Closed;
        assert_eq!(ticks_until(&mut lid, closed), 10);
        assert_eq!(lid.progress(1.0), 0.0);
        assert!(!lid.at_rest(), "it still sinks the last tenth");
        lid.tick();
        assert!(lid.at_rest());
    }

    #[test]
    fn only_counts_of_zero_and_one_move_a_shulker_lid() {
        let mut lid = Lid::new(LidKind::Shulker);
        lid.set_open_count(2);
        assert_eq!(shulker_status(&lid), ShulkerStatus::Closed);
        lid.set_open_count(1);
        ticks_until(&mut lid, |lid| shulker_status(lid) == ShulkerStatus::Opened);
        lid.set_open_count(2);
        lid.tick();
        assert_eq!(shulker_status(&lid), ShulkerStatus::Opened);
        assert_eq!(lid.progress(0.5), 1.0);
    }

    #[test]
    fn a_lid_half_a_tick_on_is_halfway_between_its_ticks() {
        for kind in [LidKind::Shulker, LidKind::Chest] {
            let mut lid = Lid::new(kind);
            lid.set_open_count(1);
            for _ in 0..3 {
                lid.tick();
            }
            let (from, to) = (lid.progress(0.0), lid.progress(1.0));
            assert!((from - 0.2).abs() < 1e-6 && (to - 0.3).abs() < 1e-6);
            assert!((lid.progress(0.5) - 0.25).abs() < 1e-6);
        }
    }

    #[test]
    fn a_chest_lid_clamps_and_turns_back_from_where_it_is() {
        let mut lid = Lid::new(LidKind::Chest);
        lid.set_open_count(3);
        assert_eq!(ticks_until(&mut lid, |lid| lid.progress(1.0) == 1.0), 10);
        lid.tick();
        assert_eq!((lid.progress(0.0), lid.progress(1.0)), (1.0, 1.0));
        lid.set_open_count(0);
        for _ in 0..4 {
            lid.tick();
        }
        let before = lid.progress(1.0);
        lid.set_open_count(1);
        lid.tick();
        assert_eq!(lid.progress(0.0), before, "it turns back from where it was");
        assert!(lid.progress(1.0) > before);
        lid.set_open_count(0);
        assert_eq!(ticks_until(&mut lid, Lid::at_rest), 8);
        assert_eq!(lid.progress(1.0), 0.0);
    }

    #[test]
    fn a_new_lid_waits_for_its_section_before_it_moves() {
        let cell = (3, 70, -5);
        let scene = box_scene(cell, "minecraft:red_shulker_box");
        let mut lids = Lids::default();
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 1);
        assert_eq!(lids.posed(&scene, 1.0)[0].progress, 0.0);
        assert!(!lids.lid_closed(cell));
        lids.tick();
        assert_eq!(lids.take_apart_changes(), vec![(cell, true)]);
        assert_eq!(lids.take_apart_changes(), Vec::new());
        lids.settle(&scene, |_| false);
        lids.tick();
        assert_eq!(lids.posed(&scene, 1.0)[0].progress, 0.0);
        lids.settle(&scene, |_| true);
        lids.tick();
        let posed = lids.posed(&scene, 1.0);
        assert_eq!(posed.len(), 1);
        assert_eq!(
            (posed[0].cell, posed[0].block.id.path.as_str()),
            (cell, "red_shulker_box")
        );
        assert!((posed[0].progress - 0.1).abs() < 1e-6);
    }

    #[test]
    fn a_closed_lid_goes_back_to_its_section_once() {
        let cell = (0, 64, 0);
        let scene = box_scene(cell, "minecraft:shulker_box");
        let mut lids = opened_lids(&scene, &[cell]);
        for _ in 0..12 {
            lids.tick();
        }
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 0);
        for _ in 0..10 {
            lids.tick();
            assert_eq!(lids.take_apart_changes(), Vec::new());
        }
        assert!(lids.lid_closed(cell));
        lids.tick();
        assert_eq!(lids.take_apart_changes(), vec![(cell, false)]);
        lids.tick();
        assert_eq!(lids.take_apart_changes(), Vec::new());
        lids.settle(&scene, |_| false);
        assert_eq!(
            lids.posed(&scene, 0.5).len(),
            1,
            "drawn until the section has it"
        );
        lids.settle(&scene, |_| true);
        assert!(lids.is_empty());
    }

    #[test]
    fn a_lid_reopened_while_closing_turns_back_without_a_jump() {
        let cell = (0, 64, 0);
        let scene = box_scene(cell, "minecraft:shulker_box");
        let mut lids = opened_lids(&scene, &[cell]);
        for _ in 0..10 {
            lids.tick();
        }
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 0);
        for _ in 0..4 {
            lids.tick();
        }
        let before = lids.posed(&scene, 1.0)[0].progress;
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 1);
        lids.tick();
        assert_eq!(lids.posed(&scene, 0.0)[0].progress, before);
        assert!(lids.posed(&scene, 1.0)[0].progress > before);
        assert_eq!(lids.take_apart_changes(), Vec::new());
    }

    #[test]
    fn a_lid_shut_before_its_section_returned_it_is_taken_apart_again() {
        let cell = (0, 64, 0);
        let scene = box_scene(cell, "minecraft:shulker_box");
        let mut lids = opened_lids(&scene, &[cell]);
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 0);
        lids.tick();
        lids.tick();
        lids.tick();
        assert_eq!(lids.take_apart_changes(), vec![(cell, false)]);
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 1);
        lids.settle(&scene, |_| true);
        assert_eq!(lids.take_apart_changes(), vec![(cell, true)]);
        lids.settle(&scene, |_| true);
        lids.tick();
        assert!(lids.posed(&scene, 1.0)[0].progress > 0.0);
    }

    #[test]
    fn a_broken_box_leaves_no_lid_and_gives_its_cell_back() {
        let cell = (0, 64, 0);
        let mut scene = box_scene(cell, "minecraft:shulker_box");
        let mut lids = opened_lids(&scene, &[cell]);
        lids.tick();
        scene.set(cell, None);
        lids.settle(&scene, |_| false);
        assert_eq!(lids.posed(&scene, 1.0), Vec::new());
        assert_eq!(lids.take_apart_changes(), vec![(cell, false)]);
        lids.settle(&scene, |_| true);
        assert!(lids.is_empty());
        let mut lids = Lids::default();
        scene.set(cell, Some(Block::new("minecraft:shulker_box")));
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 1);
        scene.set(cell, Some(Block::new("minecraft:stone")));
        lids.settle(&scene, |_| true);
        assert!(lids.is_empty(), "never taken apart, nothing to give back");
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 1);
        assert!(lids.is_empty(), "stone has no lid");
    }

    #[test]
    fn a_count_that_opens_nothing_makes_no_lid() {
        let cell = (0, 64, 0);
        let scene = box_scene(cell, "minecraft:shulker_box");
        let mut lids = Lids::default();
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 0);
        lids.block_event(&scene, cell, SET_OPEN_COUNT, 2);
        lids.block_event(&scene, cell, 2, 1);
        assert!(lids.is_empty());
        assert!(lids.lid_closed(cell));
    }

    #[test]
    fn both_halves_of_a_double_chest_swing_as_the_wider_open_one() {
        let mut scene = HandcraftedScene::default();
        let (left, right) = ((0, 64, 0), (1, 64, 0));
        let chest = |side: &str| {
            Block::new("minecraft:trapped_chest")
                .with("facing", "north")
                .with("type", side)
        };
        scene.set(left, Some(chest("left")));
        scene.set(right, Some(chest("right")));
        assert_eq!(paired(&scene, left, &chest("left")), Some(right));
        let mut lids = opened_lids(&scene, &[right]);
        for _ in 0..5 {
            lids.tick();
        }
        lids.block_event(&scene, left, SET_OPEN_COUNT, 1);
        let posed = lids.posed(&scene, 1.0);
        assert_eq!(posed.len(), 2);
        assert!(posed.iter().all(|lid| (lid.progress - 0.5).abs() < 1e-6));
        let ender = Block::new("minecraft:ender_chest").with("facing", "north");
        scene.set(left, Some(ender));
        let apart: Vec<_> = lids
            .posed(&scene, 1.0)
            .iter()
            .map(|lid| (lid.cell, lid.progress))
            .collect();
        assert_eq!(apart, vec![(left, 0.0), (right, 0.5)]);
    }

    #[test]
    fn every_chest_and_shulker_box_has_a_lid() {
        for (id, kind) in [
            ("minecraft:chest", Some(LidKind::Chest)),
            ("minecraft:trapped_chest", Some(LidKind::Chest)),
            ("minecraft:ender_chest", Some(LidKind::Chest)),
            (
                "minecraft:waxed_oxidized_copper_chest",
                Some(LidKind::Chest),
            ),
            ("minecraft:shulker_box", Some(LidKind::Shulker)),
            ("minecraft:lime_shulker_box", Some(LidKind::Shulker)),
            ("minecraft:barrel", None),
            ("other:chest", None),
        ] {
            let block = Block::new(id);
            assert_eq!(LidKind::of(&block), kind, "{id}");
            assert_eq!(lid_model(&block, 0.5).unwrap().is_some(), kind.is_some());
        }
    }

    #[test]
    fn apart_cells_name_their_section_once_per_change() {
        let mut apart = ApartCells::default();
        assert_eq!(apart.set((17, -3, 40), true), Some((1, -1, 2)));
        assert_eq!(apart.set((17, -3, 40), true), None);
        assert_eq!(apart.set((18, 5, 33), true), Some((1, 0, 2)));
        assert_eq!(apart.set((31, -16, 47), true), Some((1, -1, 2)));
        assert_eq!(apart.set((32, -3, 40), true), Some((2, -1, 2)));
        assert_eq!(
            apart.in_section((1, -1, 2)),
            vec![(17, -3, 40), (31, -16, 47)]
        );
        assert_eq!(apart.in_section((1, 0, 2)), vec![(18, 5, 33)]);
        assert!(apart.in_section((0, -1, 2)).is_empty());
        assert!(apart.contains((32, -3, 40)));
        assert_eq!(apart.set((17, -3, 40), false), Some((1, -1, 2)));
        assert_eq!(apart.set((17, -3, 40), false), None);
        assert_eq!(apart.in_section((1, -1, 2)), vec![(31, -16, 47)]);
        apart.clear();
        assert!(!apart.contains((32, -3, 40)));
    }
}
