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
pub const WARNING_LEVEL: &str = "warning_level";
pub const SHRIEK_TICKS: i32 = 90;
pub const SHRIEK_EVENT: i32 = 3007;
const SHRIEKING: &str = "shrieking";
const CAN_SUMMON: &str = "can_summon";
const SHRIEK: &str = "minecraft:shriek";
pub const BLOOM_TICKS: i32 = 8;
const BLOOM: &str = "bloom";
const BLOOM_SOUND: &str = "minecraft:block.sculk_catalyst.bloom";
const PHASE: &str = "sculk_sensor_phase";
const TENDRILS_CLICKING: &str = "minecraft:sculk_sensor_tendrils_clicking";
const CLICKING: &str = "minecraft:block.sculk_sensor.clicking";
const CLICKING_STOP: &str = "minecraft:block.sculk_sensor.clicking_stop";
const RESONATORS: &str = "minecraft:vibration_resonators";
const RESONATE: &str = "minecraft:block.amethyst_block.resonate";
const RESONANCE_NOTES: [i32; 16] = [0, 0, 2, 4, 6, 7, 9, 10, 12, 14, 15, 18, 19, 21, 22, 24];
const RESONATE_EVENTS: [&str; 15] = [
    "minecraft:resonate_1",
    "minecraft:resonate_2",
    "minecraft:resonate_3",
    "minecraft:resonate_4",
    "minecraft:resonate_5",
    "minecraft:resonate_6",
    "minecraft:resonate_7",
    "minecraft:resonate_8",
    "minecraft:resonate_9",
    "minecraft:resonate_10",
    "minecraft:resonate_11",
    "minecraft:resonate_12",
    "minecraft:resonate_13",
    "minecraft:resonate_14",
    "minecraft:resonate_15",
];

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
            self.sculk_activate(source, pos, state, power, frequency);
        }
    }

    fn sculk_activate(
        &mut self,
        source: Option<EventSource>,
        pos: BlockPos,
        state: BlockStateId,
        power: i32,
        frequency: i32,
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
        self.sculk_resonate(source, pos, frequency);
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

    fn sculk_resonate(&mut self, source: Option<EventSource>, pos: BlockPos, frequency: i32) {
        let Some(tag) = self.registries().block_tags.id(RESONATORS) else {
            return;
        };
        let Some(index) = usize::try_from(frequency - 1)
            .ok()
            .filter(|&index| index < RESONATE_EVENTS.len())
        else {
            return;
        };
        let event = RESONATE_EVENTS[index];
        let pitch = 2f64.powf(f64::from(RESONANCE_NOTES[index + 1] - 12) / 12.0) as f32;
        for direction in Direction::ALL {
            let at = pos.relative(direction, 1);
            let state = self.block(at);
            if self.registries().block_in_tag(state, tag) {
                self.game_event_from(event, centre(at), source, Some(state));
                self.sounds.push(LevelSound {
                    event: RESONATE,
                    pos: centre(at),
                    volume: 1.0,
                    pitch,
                });
            }
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

    pub fn is_sculk_shrieker(&self, state: BlockStateId) -> bool {
        self.is_a(state, "SculkShriekerBlock")
    }

    pub fn shrieker_shrieking(&self, pos: BlockPos) -> bool {
        let state = self.block(pos);
        self.is_sculk_shrieker(state) && self.sensor_property(state, SHRIEKING) == Some("true")
    }

    fn shrieker_can_respond_as(&self, state: BlockStateId) -> bool {
        self.sensor_property(state, CAN_SUMMON) == Some("true") && self.difficulty != 0 && self.spawn_wardens
    }

    pub fn shrieker_can_respond(&self, pos: BlockPos) -> bool {
        let state = self.block(pos);
        self.is_sculk_shrieker(state) && self.shrieker_can_respond_as(state)
    }

    pub fn shrieker_warning_level(&self, pos: BlockPos) -> i32 {
        let level = self
            .block_entity(pos)
            .and_then(|tag| tag.get(WARNING_LEVEL))
            .and_then(Tag::as_i64);
        level.unwrap_or(0) as i32
    }

    pub fn set_shrieker_warning_level(&mut self, pos: BlockPos, level: i32) {
        self.set_block_entity_value(pos, WARNING_LEVEL, Tag::Int(level));
    }

    pub fn sculk_shriek(&mut self, pos: BlockPos, source: Option<EventSource>) {
        let state = self.block(pos);
        if !self.is_sculk_shrieker(state) {
            return;
        }
        let shrieking = self.sensor_with(state, SHRIEKING, "true");
        self.set_block(pos, shrieking, super::update::CLIENTS, super::update::LIMIT);
        self.schedule_block_tick(pos, shrieking, SHRIEK_TICKS);
        self.level_events.push((SHRIEK_EVENT, pos, 0));
        self.game_event_from(SHRIEK, centre(pos), source, None);
    }

    fn shrieker_respond(&mut self, pos: BlockPos, state: BlockStateId) {
        let level = self.shrieker_warning_level(pos);
        if self.shrieker_can_respond_as(state) && level > 0 {
            self.shrieker_responses.push((pos, level));
        }
    }

    pub(super) fn sculk_block_tick(&mut self, state: BlockStateId, pos: BlockPos) {
        if self.is_sculk_shrieker(state) && self.sensor_property(state, SHRIEKING) == Some("true") {
            let quiet = self.sensor_with(state, SHRIEKING, "false");
            self.set_block_and_update(pos, quiet);
            self.shrieker_respond(pos, quiet);
        }
        if self.is_sculk_catalyst(state) && self.sensor_property(state, BLOOM) == Some("true") {
            let rest = self.sensor_with(state, BLOOM, "false");
            self.set_block_and_update(pos, rest);
        }
    }

    pub fn is_sculk_catalyst(&self, state: BlockStateId) -> bool {
        self.is_a(state, "SculkCatalystBlock")
    }

    pub fn sculk_bloom(&mut self, pos: BlockPos) {
        let state = self.block(pos);
        if !self.is_sculk_catalyst(state) {
            return;
        }
        let bloom = self.sensor_with(state, BLOOM, "true");
        self.set_block_and_update(pos, bloom);
        self.schedule_block_tick(pos, bloom, BLOOM_TICKS);
        self.catalyst_blooms.push(pos);
        let pitch = 0.6 + self.random.next_f32() * 0.4;
        self.sounds.push(LevelSound {
            event: BLOOM_SOUND,
            pos: centre(pos),
            volume: 2.0,
            pitch,
        });
    }

    pub(super) fn sculk_pre_remove(&mut self, pos: BlockPos, old: BlockStateId) {
        if self.is_sculk_shrieker(old) && self.sensor_property(old, SHRIEKING) == Some("true") {
            self.shrieker_respond(pos, old);
        }
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

    #[test]
    fn an_amethyst_block_beside_a_sensor_resonates_its_frequency_to_a_calibrated_sensor() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        place(&mut level, SENSOR, "minecraft:sculk_sensor");
        let east = SENSOR.relative(Direction::East, 1);
        let amethyst = place(&mut level, east, "minecraft:amethyst_block");
        place(&mut level, SENSOR.above(), "minecraft:amethyst_block");
        place(
            &mut level,
            SENSOR.relative(Direction::West, 1),
            "minecraft:budding_amethyst",
        );
        level.take_game_events();
        level.sounds.clear();
        level.sculk_receive(SENSOR, 8, 12, Some(EventSource::Player(4)));
        let posted: Vec<_> = level
            .take_game_events()
            .into_iter()
            .map(|event| (event.event, event.pos, event.source, event.affected))
            .collect();
        let player = Some(EventSource::Player(4));
        assert_eq!(
            posted,
            vec![
                (
                    "minecraft:resonate_8",
                    centre(SENSOR.above()),
                    player,
                    Some(amethyst)
                ),
                ("minecraft:resonate_8", centre(east), player, Some(amethyst)),
                (TENDRILS_CLICKING, centre(SENSOR), player, None),
            ],
            "each amethyst block beside it, in vanilla's direction order, before the tendrils"
        );
        let resonating: Vec<_> = level
            .sounds
            .iter()
            .filter(|sound| sound.event == RESONATE)
            .map(|sound| sound.pitch)
            .collect();
        assert_eq!(resonating, vec![1.0, 1.0], "frequency 8 is note 12");
        assert_eq!(RESONATE_EVENTS[0], "minecraft:resonate_1");
        assert_eq!(RESONATE_EVENTS[14], "minecraft:resonate_15");
        let highest = 2f64.powf(f64::from(RESONANCE_NOTES[15] - 12) / 12.0) as f32;
        assert_eq!(highest, 2.0);
    }

    const SHRIEKER: BlockPos = BlockPos { x: 4, y: 65, z: 4 };

    fn shrieker(level: &mut Level<'_>, state: &str, warning: i32) {
        place(level, SHRIEKER, state);
        level.set_shrieker_warning_level(SHRIEKER, warning);
        level.take_game_events();
        level.level_events.clear();
    }

    #[test]
    fn a_shriek_lasts_ninety_ticks_and_responds_at_its_end() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        shrieker(&mut level, "minecraft:sculk_shrieker[can_summon=true]", 2);
        assert!(level.shrieker_can_respond(SHRIEKER));
        assert!(!level.shrieker_shrieking(SHRIEKER));
        level.sculk_shriek(SHRIEKER, Some(EventSource::Player(4)));
        assert!(level.shrieker_shrieking(SHRIEKER));
        assert_eq!(level.level_events, vec![(SHRIEK_EVENT, SHRIEKER, 0)]);
        let posted = level.take_game_events();
        assert_eq!(
            posted
                .iter()
                .map(|event| (event.event, event.pos, event.source))
                .collect::<Vec<_>>(),
            vec![(SHRIEK, centre(SHRIEKER), Some(EventSource::Player(4)))]
        );
        ticks(&mut level, SHRIEK_TICKS - 1);
        assert!(level.shrieker_shrieking(SHRIEKER));
        assert!(level.shrieker_responses.is_empty());
        ticks(&mut level, 1);
        assert!(!level.shrieker_shrieking(SHRIEKER));
        assert_eq!(level.shrieker_responses, vec![(SHRIEKER, 2)]);
        level.shrieker_responses.clear();
        level.set_shrieker_warning_level(SHRIEKER, 0);
        level.sculk_shriek(SHRIEKER, None);
        ticks(&mut level, SHRIEK_TICKS);
        assert!(
            level.shrieker_responses.is_empty(),
            "no warning, no response"
        );
    }

    #[test]
    fn a_shrieker_without_can_summon_shrieks_but_never_warns() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        shrieker(&mut level, "minecraft:sculk_shrieker", 3);
        assert!(!level.shrieker_can_respond(SHRIEKER));
        level.sculk_shriek(SHRIEKER, Some(EventSource::Player(4)));
        assert!(level.shrieker_shrieking(SHRIEKER));
        assert_eq!(level.level_events, vec![(SHRIEK_EVENT, SHRIEKER, 0)]);
        ticks(&mut level, SHRIEK_TICKS);
        assert!(!level.shrieker_shrieking(SHRIEKER));
        assert!(level.shrieker_responses.is_empty());
    }

    #[test]
    fn peaceful_or_spawn_wardens_off_means_no_response() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        shrieker(&mut level, "minecraft:sculk_shrieker[can_summon=true]", 1);
        level.difficulty = 0;
        assert!(!level.shrieker_can_respond(SHRIEKER));
        level.sculk_shriek(SHRIEKER, None);
        ticks(&mut level, SHRIEK_TICKS);
        assert!(level.shrieker_responses.is_empty(), "peaceful");
        level.difficulty = 2;
        level.spawn_wardens = false;
        assert!(!level.shrieker_can_respond(SHRIEKER));
        level.sculk_shriek(SHRIEKER, None);
        ticks(&mut level, SHRIEK_TICKS);
        assert!(level.shrieker_responses.is_empty(), "spawn_wardens off");
        level.spawn_wardens = true;
        assert!(level.shrieker_can_respond(SHRIEKER));
    }

    #[test]
    fn a_shrieking_shrieker_broken_still_responds() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        shrieker(&mut level, "minecraft:sculk_shrieker[can_summon=true]", 1);
        level.set_block(SHRIEKER, BlockStateId::AIR, update::ALL, update::LIMIT);
        assert!(level.shrieker_responses.is_empty(), "a quiet one says nothing");
        shrieker(&mut level, "minecraft:sculk_shrieker[can_summon=true]", 1);
        level.sculk_shriek(SHRIEKER, None);
        ticks(&mut level, 10);
        level.set_block(SHRIEKER, BlockStateId::AIR, update::ALL, update::LIMIT);
        assert_eq!(level.shrieker_responses, vec![(SHRIEKER, 1)]);
        ticks(&mut level, SHRIEK_TICKS);
        assert_eq!(level.shrieker_responses.len(), 1, "its tick finds no shrieker");
    }

    #[test]
    fn a_catalyst_blooms_for_eight_ticks_with_its_sound() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        place(&mut level, SHRIEKER, "minecraft:sculk_catalyst");
        level.sounds.clear();
        let bloom = |level: &Level<'_>| level.sensor_property(level.block(SHRIEKER), BLOOM) == Some("true");
        level.sculk_bloom(SHRIEKER);
        assert!(bloom(&level));
        assert_eq!(level.catalyst_blooms, vec![SHRIEKER]);
        assert_eq!(level.sounds.len(), 1);
        let sound = &level.sounds[0];
        assert_eq!((sound.event, sound.pos, sound.volume), (BLOOM_SOUND, centre(SHRIEKER), 2.0));
        assert!((0.6..1.0).contains(&sound.pitch));
        ticks(&mut level, BLOOM_TICKS - 1);
        assert!(bloom(&level));
        ticks(&mut level, 1);
        assert!(!bloom(&level), "eight ticks later it rests");
        level.catalyst_blooms.clear();
        level.sculk_bloom(SHRIEKER.above());
        assert!(level.catalyst_blooms.is_empty(), "only a catalyst blooms");
    }
}
