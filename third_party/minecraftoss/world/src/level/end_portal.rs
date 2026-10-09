use super::Level;
use super::burning::LevelSound;
use super::update;
use minecraftoss_core::BlockPos;
use minecraftoss_core::pos::Direction;
use minecraftoss_generator::feature::end_portal::{END_PORTAL, EndPortalRing, FRAME, RingCell};

pub const FILL_SOUND: &str = "minecraft:block.end_portal_frame.fill";
pub const OPEN_SOUND: &str = "minecraft:block.end_portal.spawn";
pub const OBSIDIAN: &str = "minecraft:obsidian";
pub const AIR: &str = "minecraft:air";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyePlaced {
    Filled,
    Opened(EndPortalRing),
}

pub fn end_platform_cells(origin: BlockPos) -> Vec<(BlockPos, &'static str)> {
    let mut cells = Vec::with_capacity(100);
    for z in -2..=2 {
        for x in -2..=2 {
            for y in -1..3 {
                cells.push((origin.offset(x, y, z), if y == -1 { OBSIDIAN } else { AIR }));
            }
        }
    }
    cells
}

fn centre(pos: BlockPos) -> [f64; 3] {
    [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    ]
}

impl Level<'_> {
    pub fn ring_cell(&self, pos: BlockPos) -> RingCell {
        if self.chunk(pos.chunk()).is_none() {
            return RingCell::Unloaded;
        }
        let state = self.block(pos);
        if self.name(state) != FRAME {
            return RingCell::Other;
        }
        let blocks = &self.registries().blocks;
        match blocks
            .property(state, "facing")
            .and_then(Direction::from_name)
        {
            Some(facing) => RingCell::Frame {
                facing,
                eye: blocks.property(state, "eye") == Some("true"),
            },
            None => RingCell::Other,
        }
    }

    pub fn place_ender_eye(&mut self, pos: BlockPos) -> Option<EyePlaced> {
        let state = self.block(pos);
        if self.name(state) != FRAME
            || self.registries().blocks.property(state, "eye") != Some("false")
        {
            return None;
        }
        let filled = self
            .registries()
            .blocks
            .with_property(state, "eye", "true")?;
        self.set_block(pos, filled, update::CLIENTS, update::LIMIT);
        let frame = self.block_id(filled);
        self.update_neighbour_for_output_signal(pos, frame);
        self.sounds.push(LevelSound {
            event: FILL_SOUND,
            pos: centre(pos),
            volume: 1.0,
            pitch: 1.0,
        });
        let Some(ring) = EndPortalRing::find(&|at| self.ring_cell(at), pos) else {
            return Some(EyePlaced::Filled);
        };
        let blocks = &self.registries().blocks;
        let Some(portal) = blocks
            .block_by_name(END_PORTAL)
            .map(|b| blocks.block(b).default_state())
        else {
            return Some(EyePlaced::Filled);
        };
        for at in ring.interior() {
            self.destroy_block_drops(at, true, update::LIMIT);
            self.set_block(at, portal, update::CLIENTS, update::LIMIT);
        }
        self.sounds.push(LevelSound {
            event: OPEN_SOUND,
            pos: centre(ring.centre()),
            volume: 1.0,
            pitch: 1.0,
        });
        Some(EyePlaced::Opened(ring))
    }

    pub fn create_end_platform(&mut self, origin: BlockPos, break_blocks: bool) {
        for (pos, name) in end_platform_cells(origin) {
            let blocks = &self.registries().blocks;
            let Some(block) = blocks.block_by_name(name) else {
                continue;
            };
            let state = blocks.block(block).default_state();
            if self.block_id(self.block(pos)) == block {
                continue;
            }
            if break_blocks {
                self.destroy_block_drops(pos, true, update::LIMIT);
            }
            self.set_block(pos, state, update::ALL, update::LIMIT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_end_platform_is_25_obsidian_under_75_air_in_vanilla_order() {
        let origin = BlockPos::new(100, 49, 0);
        let cells = end_platform_cells(origin);
        assert_eq!(cells.len(), 100);
        let obsidian: Vec<_> = cells.iter().filter(|(_, name)| *name == OBSIDIAN).collect();
        assert_eq!(obsidian.len(), 25);
        assert!(obsidian.iter().all(|(pos, _)| pos.y == 48));
        assert!(
            cells
                .iter()
                .filter(|(_, name)| *name == AIR)
                .all(|(pos, _)| (49..=51).contains(&pos.y))
        );
        assert_eq!(cells[0], (BlockPos::new(98, 48, -2), OBSIDIAN));
        assert_eq!(cells[1], (BlockPos::new(98, 49, -2), AIR));
        assert_eq!(cells[4], (BlockPos::new(99, 48, -2), OBSIDIAN));
        assert_eq!(cells[20], (BlockPos::new(98, 48, -1), OBSIDIAN));
        assert_eq!(cells[99], (BlockPos::new(102, 51, 2), AIR));
        for x in 98..=102 {
            for z in -2..=2 {
                for y in 48..=51 {
                    assert_eq!(
                        cells
                            .iter()
                            .filter(|(pos, _)| *pos == BlockPos::new(x, y, z))
                            .count(),
                        1
                    );
                }
            }
        }
    }
}
