use super::Level;
use super::burning::LevelSound;
use super::game_event::{EventSource, centre};
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::random::RandomSource;
use minecraftoss_core::{BlockPos, BlockStateId};

pub const SENSOR_ACTIVE_TICKS: i32 = 30;
pub const CALIBRATED_ACTIVE_TICKS: i32 = 10;
pub const COOLDOWN_TICKS: i32 = 10;
pub const LAST_FREQUENCY: &str = "last_vibration_frequency";
const PHASE: &str = "sculk_sensor_phase";
const TENDRILS_CLICKING: &str = "minecraft:sculk_sensor_tendrils_clicking";
const CLICKING: &str = "minecraft:block.sculk_sensor.clicking";
const CLICKING_STOP: &str = "minecraft:block.sculk_sensor.clicking_stop";

impl Level<'_> {
    pub fn is_sculk_sensor(&self, state: BlockStateId) -> bool {
        self.is_a(state, "SculkSensorBlock")
    }

    fn is_calibrated(&self, state: BlockStateId) -> bool {
        self.is_a(state, "CalibratedSculkSensorBlock")
    }

    fn sensor_property(&self, state: BlockStateId, name: &str) -> Option<&str> {
        self.registries().blocks.property(state, name)
    }

    fn sensor_with(&self, state: BlockStateId, name: &str, value: &str) -> BlockStateId {
        self.registries()
            .blocks
            .with_property(state, name, value)
            .unwrap_or(state)
    }

    pub fn sculk_can_activate(&self, pos: BlockPos) -> bool {
        let state = self.block(pos);
        self.is_sculk_sensor(state) && self.sensor_property(state, PHASE) == Some("inactive")
    }

    pub fn sculk_back_signal(&self, pos: BlockPos) -> i32 {
        let state = self.block(pos);
        if !self.is_calibrated(state) {
            return 0;
        }
        let Some(facing) = self
            .sensor_property(state, "facing")
            .and_then(Direction::from_name)
        else {
            return 0;
        };
        let back = facing.opposite();
        self.signal(pos.relative(back, 1), back)
    }

    pub fn sculk_receive(
        &mut self,
        pos: BlockPos,
        frequency: i32,
        power: i32,
        source: Option<EventSource>,
    ) {
        if !self.sculk_can_activate(pos) {
            return;
        }
        self.set_block_entity_value(pos, LAST_FREQUENCY, Tag::Int(frequency));
        let state = self.block(pos);
        if self.is_sculk_sensor(state) {
            self.sculk_activate(source, pos, state, power);
        }
    }

    fn sculk_activate(
        &mut self,
        source: Option<EventSource>,
        pos: BlockPos,
        state: BlockStateId,
        power: i32,
    ) {
        let active = self.sensor_with(state, PHASE, "active");
        let active = self.sensor_with(active, "power", &power.clamp(0, 15).to_string());
        self.set_block_and_update(pos, active);
        let ticks = if self.is_calibrated(state) {
            CALIBRATED_ACTIVE_TICKS
        } else {
            SENSOR_ACTIVE_TICKS
        };
        self.schedule_block_tick(pos, active, ticks);
        self.sculk_update_neighbours(pos, active);
        self.game_event_from(TENDRILS_CLICKING, centre(pos), source, None);
        if self.sensor_property(state, "waterlogged") != Some("true") {
            let pitch = self.random.next_f32() * 0.2 + 0.8;
            self.sounds.push(LevelSound {
                event: CLICKING,
                pos: centre(pos),
                volume: 1.0,
                pitch,
            });
        }
    }

    fn sculk_update_neighbours(&mut self, pos: BlockPos, state: BlockStateId) {
        let block = self.block_id(state);
        self.update_neighbors_at(pos, block);
        self.update_neighbors_at(pos.below(), block);
    }

    pub(super) fn sculk_tick(&mut self, state: BlockStateId, pos: BlockPos) {
        match self.sensor_property(state, PHASE) {
            Some("active") => {
                let cooling = self.sensor_with(state, PHASE, "cooldown");
                let cooling = self.sensor_with(cooling, "power", "0");
                self.set_block_and_update(pos, cooling);
                self.schedule_block_tick(pos, cooling, COOLDOWN_TICKS);
                self.sculk_update_neighbours(pos, cooling);
            }
            Some("cooldown") => {
                let resting = self.sensor_with(state, PHASE, "inactive");
                self.set_block_and_update(pos, resting);
                if self.sensor_property(state, "waterlogged") != Some("true") {
                    let pitch = self.random.next_f32() * 0.2 + 0.8;
                    self.sounds.push(LevelSound {
                        event: CLICKING_STOP,
                        pos: centre(pos),
                        volume: 1.0,
                        pitch,
                    });
                }
            }
            _ => {}
        }
    }

    pub(super) fn sculk_on_place(&mut self, state: BlockStateId, pos: BlockPos, old: BlockStateId) {
        let block = self.block_id(state);
        if self.block_id(old) == block {
            return;
        }
        let powered = self
            .sensor_property(state, "power")
            .is_some_and(|power| power != "0");
        if powered && !self.has_block_tick_at(pos, block) {
            let off = self.sensor_with(state, "power", "0");
            self.set_block(
                pos,
                off,
                super::update::CLIENTS | super::update::KNOWN_SHAPE,
                super::update::LIMIT,
            );
        }
    }

    pub(super) fn sculk_after_removal(&mut self, state: BlockStateId, pos: BlockPos) {
        if self.sensor_property(state, PHASE) == Some("active") {
            self.sculk_update_neighbours(pos, state);
        }
    }

    pub(super) fn sculk_signal(&self, state: BlockStateId, direction: Direction) -> i32 {
        let facing = self
            .sensor_property(state, "facing")
            .and_then(Direction::from_name);
        if self.is_calibrated(state) && facing == Some(direction) {
            return 0;
        }
        self.sensor_property(state, "power")
            .and_then(|power| power.parse().ok())
            .unwrap_or(0)
    }

    pub(super) fn sculk_comparator(&self, state: BlockStateId, pos: BlockPos) -> i32 {
        if self.sensor_property(state, PHASE) != Some("active") {
            return 0;
        }
        let frequency = self
            .block_entity(pos)
            .and_then(|tag| tag.get(LAST_FREQUENCY))
            .and_then(Tag::as_i64);
        frequency.unwrap_or(0) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::super::update;
    use super::*;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_core::{Chunk, ChunkPos, Registries};
    use minecraftoss_generator::feature::Library;

    const SENSOR: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

    fn library() -> Option<Library> {
        let paths = DataPaths::discover().ok()?;
        let registries = std::sync::Arc::new(Registries::load(&paths).ok()?);
        let bounds = minecraftoss_generator::providers::GenContext {
            min_y: -64,
            depth: 384,
            sea_level: 63,
        };
        Library::load(registries, bounds).ok()
    }

    fn level(lib: &Library) -> Level<'_> {
        let mut level = Level::new(lib, -64, 384);
        let stone = lib
            .registries
            .blocks
            .parse_state("minecraft:stone")
            .unwrap();
        for cx in -1..=1 {
            for cz in -1..=1 {
                let mut chunk = Chunk::new(
                    ChunkPos::new(cx, cz),
                    -64,
                    384,
                    minecraftoss_core::BiomeId(0),
                );
                chunk.status = minecraftoss_core::chunk::ChunkStatus::Full;
                for x in 0..16 {
                    for z in 0..16 {
                        chunk.set_block_raw(x, 64, z, stone, &lib.registries);
                    }
                }
                level.insert_chunk(chunk);
            }
        }
        level
    }

    fn place(level: &mut Level<'_>, pos: BlockPos, state: &str) -> BlockStateId {
        let state = level.registries().blocks.parse_state(state).unwrap();
        level.set_block(pos, state, update::ALL, update::LIMIT);
        level.block(pos)
    }

    fn phase(level: &Level<'_>) -> String {
        level
            .sensor_property(level.block(SENSOR), PHASE)
            .unwrap_or_default()
            .to_owned()
    }

    fn power(level: &Level<'_>) -> i32 {
        level.sculk_signal(level.block(SENSOR), Direction::Up)
    }

    fn ticks(level: &mut Level<'_>, count: i32) {
        for _ in 0..count {
            level.tick();
        }
    }

    #[test]
    fn phases_go_active_thirty_cooldown_ten_inactive() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        place(&mut level, SENSOR, "minecraft:sculk_sensor");
        level.sounds.clear();
        level.sculk_receive(SENSOR, 1, 6, Some(EventSource::Player(4)));
        assert_eq!((phase(&level), power(&level)), ("active".to_owned(), 6));
        assert_eq!(
            level
                .sounds
                .iter()
                .map(|sound| sound.event)
                .collect::<Vec<_>>(),
            vec![CLICKING]
        );
        let clicking = level.take_game_events();
        assert_eq!(clicking.len(), 1);
        assert_eq!(
            (clicking[0].event, clicking[0].source),
            (TENDRILS_CLICKING, Some(EventSource::Player(4)))
        );
        level.sculk_receive(SENSOR, 13, 15, None);
        assert_eq!(power(&level), 6, "an active sensor takes nothing new");
        ticks(&mut level, 29);
        assert_eq!(phase(&level), "active");
        ticks(&mut level, 1);
        assert_eq!((phase(&level), power(&level)), ("cooldown".to_owned(), 0));
        level.sounds.clear();
        ticks(&mut level, 9);
        assert_eq!(phase(&level), "cooldown");
        ticks(&mut level, 1);
        assert_eq!(phase(&level), "inactive");
        assert_eq!(
            level
                .sounds
                .iter()
                .map(|sound| sound.event)
                .collect::<Vec<_>>(),
            vec![CLICKING_STOP]
        );
        assert!(level.sculk_can_activate(SENSOR));
    }

    #[test]
    fn a_calibrated_sensor_is_active_ten_and_filters_by_its_back_input() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        place(
            &mut level,
            SENSOR,
            "minecraft:calibrated_sculk_sensor[facing=north]",
        );
        assert_eq!(level.sculk_back_signal(SENSOR), 0);
        let behind = SENSOR.relative(Direction::South, 1);
        place(&mut level, behind, "minecraft:redstone_block");
        assert_eq!(level.sculk_back_signal(SENSOR), 15);
        level.set_block(behind, BlockStateId::AIR, update::ALL, update::LIMIT);
        place(
            &mut level,
            SENSOR.relative(Direction::East, 1),
            "minecraft:redstone_block",
        );
        assert_eq!(
            level.sculk_back_signal(SENSOR),
            0,
            "only the back face counts"
        );
        level.sculk_receive(SENSOR, 4, 12, None);
        assert_eq!(power(&level), 12);
        assert_eq!(
            level.sculk_signal(level.block(SENSOR), Direction::North),
            0,
            "its back input gets nothing"
        );
        assert_eq!(
            level.sculk_signal(level.block(SENSOR), Direction::South),
            12
        );
        ticks(&mut level, 9);
        assert_eq!(phase(&level), "active");
        ticks(&mut level, 1);
        assert_eq!(phase(&level), "cooldown");
    }

    #[test]
    fn the_comparator_reads_the_last_frequency_while_active() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        place(&mut level, SENSOR, "minecraft:sculk_sensor");
        let state = level.block(SENSOR);
        assert_eq!(level.sculk_comparator(state, SENSOR), 0);
        level.sculk_receive(SENSOR, 13, 9, None);
        assert_eq!(level.sculk_comparator(level.block(SENSOR), SENSOR), 13);
        assert_eq!(
            level
                .block_entity(SENSOR)
                .and_then(|tag| tag.get(LAST_FREQUENCY))
                .and_then(Tag::as_i64),
            Some(13)
        );
        ticks(&mut level, 30);
        assert_eq!(
            level.sculk_comparator(level.block(SENSOR), SENSOR),
            0,
            "nothing while cooling down"
        );
        let lamp = SENSOR.relative(Direction::East, 1);
        place(&mut level, lamp, "minecraft:redstone_lamp");
        ticks(&mut level, 10);
        level.sculk_receive(SENSOR, 1, 6, None);
        assert_eq!(
            level.sensor_property(level.block(lamp), "lit"),
            Some("true"),
            "the power reaches a lamp"
        );
    }

    #[test]
    fn a_powered_sensor_placed_without_a_tick_loses_its_power() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        place(&mut level, SENSOR, "minecraft:sculk_sensor[power=7]");
        assert_eq!(power(&level), 0);
    }
}
