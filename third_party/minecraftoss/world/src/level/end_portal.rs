use super::Level;
use super::burning::LevelSound;
use super::update;
use minecraftoss_core::BlockPos;
use minecraftoss_core::pos::Direction;
use minecraftoss_generator::feature::end_portal::{END_PORTAL, EndPortalRing, FRAME, RingCell};

pub const FILL_SOUND: &str = "minecraft:block.end_portal_frame.fill";
pub const OPEN_SOUND: &str = "minecraft:block.end_portal.spawn";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyePlaced {
    Filled,
    Opened(EndPortalRing),
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
}
