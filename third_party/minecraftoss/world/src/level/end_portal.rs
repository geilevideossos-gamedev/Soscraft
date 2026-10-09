use super::Level;
use super::burning::LevelSound;
use super::update;
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::{BlockPos, ChunkPos};
use minecraftoss_generator::feature::World;
use minecraftoss_generator::feature::end_portal::{
    BEDROCK, END_PORTAL, EndPortalRing, ExitPortal, FRAME, RingCell,
};

pub const FILL_SOUND: &str = "minecraft:block.end_portal_frame.fill";
pub const OPEN_SOUND: &str = "minecraft:block.end_portal.spawn";
pub const OBSIDIAN: &str = "minecraft:obsidian";
pub const AIR: &str = "minecraft:air";
pub const ARENA_SIZE_CHUNKS: i32 = 8;
pub const PODIUM_ACTIVE: &str = "minecraft:end_podium_active";
pub const PODIUM_INACTIVE: &str = "minecraft:end_podium_inactive";

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

    fn end_portals_in(&self, chunk: ChunkPos) -> Vec<BlockPos> {
        let Some(chunk) = self.chunk(chunk) else {
            return Vec::new();
        };
        let mut portals: Vec<BlockPos> = chunk
            .block_entities
            .entities
            .iter()
            .filter(|(_, nbt)| nbt.get("id").and_then(Tag::as_str) == Some(END_PORTAL))
            .map(|(&(x, y, z), _)| BlockPos::new(x, y, z))
            .collect();
        portals.sort_by_key(|pos| (pos.y, pos.z, pos.x));
        portals
    }

    pub fn has_active_exit_portal(&self) -> bool {
        (-ARENA_SIZE_CHUNKS..=ARENA_SIZE_CHUNKS).any(|x| {
            (-ARENA_SIZE_CHUNKS..=ARENA_SIZE_CHUNKS)
                .any(|z| !self.end_portals_in(ChunkPos::new(x, z)).is_empty())
        })
    }

    pub fn is_bedrock(&self, pos: BlockPos) -> bool {
        self.chunk(pos.chunk()).is_some() && self.name(self.block(pos)) == BEDROCK
    }

    pub fn find_exit_portal(&self, origin: BlockPos) -> Option<BlockPos> {
        let is_bedrock = |pos: BlockPos| self.is_bedrock(pos);
        let centre = origin.chunk();
        for x in centre.x - ARENA_SIZE_CHUNKS..=centre.x + ARENA_SIZE_CHUNKS {
            for z in centre.z - ARENA_SIZE_CHUNKS..=centre.z + ARENA_SIZE_CHUNKS {
                for portal in self.end_portals_in(ChunkPos::new(x, z)) {
                    if let Some(found) = ExitPortal::find(&is_bedrock, portal) {
                        return Some(found.centre());
                    }
                }
            }
        }
        let top = World::height_at(self, HeightmapKind::MotionBlocking, origin.x, origin.z);
        (self.min_y()..=top).rev().find_map(|y| {
            ExitPortal::find(&is_bedrock, BlockPos::new(origin.x, y, origin.z))
                .map(ExitPortal::centre)
        })
    }

    pub fn exit_portal_location(&self, origin: BlockPos) -> BlockPos {
        let top = World::height_at(
            self,
            HeightmapKind::MotionBlockingNoLeaves,
            origin.x,
            origin.z,
        );
        let mut at = BlockPos::new(origin.x, top - 1, origin.z);
        while self.is_bedrock(at) && at.y > 63 {
            at = at.below();
        }
        BlockPos::new(at.x, (self.min_y() + 1).max(at.y), at.z)
    }

    pub fn spawn_exit_portal(
        &mut self,
        origin: BlockPos,
        location: Option<BlockPos>,
        active: bool,
    ) -> BlockPos {
        let location = location.unwrap_or_else(|| self.exit_portal_location(origin));
        let podium = if active {
            PODIUM_ACTIVE
        } else {
            PODIUM_INACTIVE
        };
        self.place_configured_feature(podium, location);
        location
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
