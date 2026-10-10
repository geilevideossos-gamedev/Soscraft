use super::{Level, Update};
use minecraftoss_core::block::{flags, BlockId};
use minecraftoss_core::pos::Direction;
use minecraftoss_core::random::RandomSource;
use minecraftoss_core::{BlockPos, BlockStateId};

const ICE_MELTS_WHEN_DESTROYED_ABOVE: &str = "minecraft:ice_melts_when_destroyed_above";
const WATER_EVAPORATES: &str = "gameplay/water_evaporates";
const FROSTED_ICE_MAX_AGE: i32 = 3;
const FROSTED_ICE_LIGHT: i32 = 11;
const FROSTED_ICE_PLACED_TICKS: (i32, i32) = (60, 120);
const FROSTED_ICE_RETRY_TICKS: (i32, i32) = (20, 40);
const FROSTED_ICE_CROWD: usize = 4;
const FROSTED_ICE_ALONE: usize = 2;
const DIRECTION_VALUES: [Direction; 6] = [
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

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

    fn next_int_between(&mut self, (min, max): (i32, i32)) -> i32 {
        if min >= max {
            min
        } else {
            self.random.next_i32_bound(max - min + 1) + min
        }
    }

    pub(super) fn frosted_ice_on_place(&mut self, state: BlockStateId, pos: BlockPos) {
        let delay = self.next_int_between(FROSTED_ICE_PLACED_TICKS);
        self.schedule_block_tick(pos, state, delay);
    }

    fn fewer_frosted_neighbours_than(&self, state: BlockStateId, pos: BlockPos, limit: usize) -> bool {
        let block = self.block_id(state);
        DIRECTION_VALUES
            .iter()
            .filter(|direction| self.block_id(self.block(pos.relative(**direction, 1))) == block)
            .count()
            < limit
    }

    fn frosted_ice_slightly_melt(&mut self, state: BlockStateId, pos: BlockPos) -> bool {
        let age: i32 = self
            .registries()
            .blocks
            .property(state, "age")
            .and_then(|age| age.parse().ok())
            .unwrap_or(0);
        if age < FROSTED_ICE_MAX_AGE {
            let older = self.with(state, "age", &(age + 1).to_string());
            self.set_block(pos, older, super::update::CLIENTS, super::update::LIMIT);
            false
        } else {
            self.melt_ice(pos);
            true
        }
    }

    pub(super) fn frosted_ice_tick(&mut self, state: BlockStateId, pos: BlockPos) {
        let crowded = self.random.next_i32_bound(3) != 0
            && !self.fewer_frosted_neighbours_than(state, pos, FROSTED_ICE_CROWD);
        let light = if self.dimension.as_deref() == Some("minecraft:the_end") {
            self.block_light(pos)
        } else {
            self.max_local_raw_brightness(pos)
        };
        let age: i32 = self
            .registries()
            .blocks
            .property(state, "age")
            .and_then(|age| age.parse().ok())
            .unwrap_or(0);
        let dampening = i32::from(self.registries().blocks.state(state).light_dampening);
        if !crowded
            && light > FROSTED_ICE_LIGHT - age - dampening
            && self.frosted_ice_slightly_melt(state, pos)
        {
            let block = self.block_id(state);
            for direction in DIRECTION_VALUES {
                let neighbour = pos.relative(direction, 1);
                let other = self.block(neighbour);
                if self.block_id(other) == block && !self.frosted_ice_slightly_melt(other, neighbour) {
                    let delay = self.next_int_between(FROSTED_ICE_RETRY_TICKS);
                    self.schedule_block_tick(neighbour, other, delay);
                }
            }
        } else {
            let delay = self.next_int_between(FROSTED_ICE_RETRY_TICKS);
            self.schedule_block_tick(pos, state, delay);
        }
    }

    pub(super) fn frosted_ice_neighbor_changed(&mut self, state: BlockStateId, pos: BlockPos, source: BlockId) {
        if self.block_id(state) == source
            && self.fewer_frosted_neighbours_than(state, pos, FROSTED_ICE_ALONE)
        {
            self.melt_ice(pos);
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
