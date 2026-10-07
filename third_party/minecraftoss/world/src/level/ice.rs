use super::{Level, Update};
use minecraftoss_core::block::flags;
use minecraftoss_core::{BlockPos, BlockStateId};

const ICE_MELTS_WHEN_DESTROYED_ABOVE: &str = "minecraft:ice_melts_when_destroyed_above";
const WATER_EVAPORATES: &str = "gameplay/water_evaporates";

pub fn melts_when_player_destroyed(
    is_ice_block: bool,
    prevents_melting: bool,
    water_evaporates: bool,
    below_melts_ice: bool,
    below_liquid: bool,
) -> bool {
    is_ice_block && !prevents_melting && !water_evaporates && (below_melts_ice || below_liquid)
}

impl Level<'_> {
    fn ice_melts_into(&self) -> BlockStateId {
        self.registries()
            .blocks
            .parse_state("minecraft:water")
            .expect("vanilla block")
    }

    pub fn ice_player_destroyed(
        &mut self,
        pos: BlockPos,
        old: BlockStateId,
        prevents_melting: bool,
    ) {
        let registries = self.registries();
        let below = self.block(pos.below());
        let below_melts_ice = registries
            .block_tags
            .id(ICE_MELTS_WHEN_DESTROYED_ABOVE)
            .is_some_and(|tag| registries.block_in_tag(below, tag));
        let melts = melts_when_player_destroyed(
            self.is_a(old, "IceBlock"),
            prevents_melting,
            self.gameplay_attribute(pos, WATER_EVAPORATES),
            below_melts_ice,
            registries.blocks.is(below, flags::LIQUID),
        );
        if melts {
            let water = self.ice_melts_into();
            self.set_block_and_update(pos, water);
        }
    }

    pub(super) fn melt_ice(&mut self, pos: BlockPos) {
        if self.gameplay_attribute(pos, WATER_EVAPORATES) {
            self.remove_block(pos, false);
            return;
        }
        let water = self.ice_melts_into();
        self.set_block_and_update(pos, water);
        let block = self.block_id(water);
        self.add_and_run(Update::Simple { pos, block });
    }
}

#[cfg(test)]
mod tests {
    use super::melts_when_player_destroyed;

    #[test]
    fn ice_over_a_block_that_blocks_motion_melts() {
        assert!(melts_when_player_destroyed(true, false, false, true, false));
    }

    #[test]
    fn ice_over_a_liquid_melts() {
        assert!(melts_when_player_destroyed(true, false, false, false, true));
    }

    #[test]
    fn ice_over_air_or_a_carpet_leaves_air() {
        assert!(!melts_when_player_destroyed(
            true, false, false, false, false
        ));
    }

    #[test]
    fn silk_touch_keeps_ice_from_melting() {
        assert!(!melts_when_player_destroyed(true, true, false, true, true));
    }

    #[test]
    fn ice_never_melts_where_water_evaporates() {
        assert!(!melts_when_player_destroyed(true, false, true, true, true));
    }

    #[test]
    fn packed_and_blue_ice_never_melt() {
        assert!(!melts_when_player_destroyed(
            false, false, false, true, true
        ));
    }
}
