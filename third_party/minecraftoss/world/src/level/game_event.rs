use super::Level;
use minecraftoss_core::{BlockPos, BlockStateId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventSource {
    Player(u64),
    Mob(u64),
    Entity(i32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameEvent {
    pub event: &'static str,
    pub pos: [f64; 3],
    pub source: Option<EventSource>,
    pub owner: Option<EventSource>,
    pub affected: Option<BlockStateId>,
    pub tick: i64,
    pub forced_to: Option<BlockPos>,
}

pub fn centre(pos: BlockPos) -> [f64; 3] {
    [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    ]
}

pub fn opened(open: bool) -> &'static str {
    if open {
        "minecraft:block_open"
    } else {
        "minecraft:block_close"
    }
}

pub fn activated(on: bool) -> &'static str {
    if on {
        "minecraft:block_activate"
    } else {
        "minecraft:block_deactivate"
    }
}

impl Level<'_> {
    pub fn game_event(
        &mut self,
        event: &'static str,
        pos: [f64; 3],
        affected: Option<BlockStateId>,
    ) {
        let source = self.acting_player.map(EventSource::Player);
        self.game_event_from(event, pos, source, affected);
    }

    pub fn block_game_event(
        &mut self,
        event: &'static str,
        pos: BlockPos,
        affected: Option<BlockStateId>,
    ) {
        self.game_event(event, centre(pos), affected);
    }

    pub fn block_game_event_from(
        &mut self,
        event: &'static str,
        pos: BlockPos,
        source: Option<EventSource>,
        affected: Option<BlockStateId>,
    ) {
        self.game_event_from(event, centre(pos), source, affected);
    }

    pub fn acting_source(&self) -> Option<EventSource> {
        self.acting_player.map(EventSource::Player)
    }

    pub fn game_event_from(
        &mut self,
        event: &'static str,
        pos: [f64; 3],
        source: Option<EventSource>,
        affected: Option<BlockStateId>,
    ) {
        self.game_events.push(GameEvent {
            event,
            pos,
            source,
            owner: None,
            affected,
            tick: self.game_time,
            forced_to: None,
        });
    }

    pub fn projectile_game_event(
        &mut self,
        event: &'static str,
        pos: [f64; 3],
        owner: Option<EventSource>,
        affected: Option<BlockStateId>,
    ) {
        self.game_events.push(GameEvent {
            event,
            pos,
            source: None,
            owner,
            affected,
            tick: self.game_time,
            forced_to: None,
        });
    }

    pub fn force_game_event(
        &mut self,
        listener: BlockPos,
        event: &'static str,
        pos: [f64; 3],
        source: Option<EventSource>,
    ) {
        self.game_events.push(GameEvent {
            event,
            pos,
            source,
            owner: None,
            affected: None,
            tick: self.game_time,
            forced_to: Some(listener),
        });
    }

    pub fn take_game_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.game_events)
    }
}

#[cfg(test)]
mod tests {
    use super::super::update;
    use super::*;
    use minecraftoss_core::pos::Direction;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_core::{Chunk, ChunkPos, Registries};
    use minecraftoss_generator::feature::Library;

    const PLAYER: Option<EventSource> = Some(EventSource::Player(7));

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

    fn place(level: &mut Level<'_>, pos: BlockPos, state: &str) {
        let state = level.registries().blocks.parse_state(state).unwrap();
        level.set_block(pos, state, update::ALL, update::LIMIT);
    }

    fn posted(level: &mut Level<'_>) -> Vec<(&'static str, Option<EventSource>)> {
        level
            .take_game_events()
            .into_iter()
            .map(|event| (event.event, event.source))
            .collect()
    }

    #[test]
    fn doors_gates_trapdoors_levers_buttons_and_note_blocks_post_their_events() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        let door = BlockPos::new(-12, 65, 2);
        place(&mut level, door, "minecraft:oak_door[half=lower]");
        place(&mut level, door.above(), "minecraft:oak_door[half=upper]");
        let trapdoor = BlockPos::new(-8, 65, 2);
        place(&mut level, trapdoor, "minecraft:oak_trapdoor");
        let gate = BlockPos::new(-4, 65, 2);
        place(&mut level, gate, "minecraft:oak_fence_gate");
        let lever = BlockPos::new(0, 65, 2);
        place(&mut level, lever, "minecraft:lever[face=floor]");
        let button = BlockPos::new(4, 65, 2);
        place(&mut level, button, "minecraft:oak_button[face=floor]");
        let note = BlockPos::new(8, 65, 2);
        place(&mut level, note, "minecraft:note_block");
        level.take_game_events();
        level.acting_player = Some(7);
        for pos in [door, trapdoor, gate] {
            assert!(level.use_block_facing(pos, Some(Direction::North)));
            assert_eq!(posted(&mut level), vec![("minecraft:block_open", PLAYER)]);
            assert!(level.use_block_facing(pos, Some(Direction::North)));
            assert_eq!(
                posted(&mut level),
                vec![("minecraft:block_close", PLAYER)],
                "closing {pos:?}"
            );
        }
        assert!(level.use_block_facing(lever, None));
        assert_eq!(
            posted(&mut level),
            vec![("minecraft:block_activate", PLAYER)]
        );
        assert!(level.use_block_facing(lever, None));
        assert_eq!(
            posted(&mut level),
            vec![("minecraft:block_deactivate", PLAYER)]
        );
        assert!(level.use_block_facing(button, None));
        assert_eq!(
            posted(&mut level),
            vec![("minecraft:block_activate", PLAYER)]
        );
        level.acting_player = None;
        for _ in 0..30 {
            level.tick();
        }
        assert_eq!(
            posted(&mut level),
            vec![("minecraft:block_deactivate", None)],
            "the button springs back on its own"
        );
        level.acting_player = Some(7);
        assert!(level.use_block_facing(note, None));
        assert_eq!(
            posted(&mut level),
            vec![("minecraft:note_block_play", PLAYER)]
        );
        assert!(level.attack_block(note));
        assert_eq!(
            posted(&mut level),
            vec![("minecraft:note_block_play", PLAYER)]
        );
        let powered = BlockPos::new(12, 65, 8);
        place(&mut level, powered, "minecraft:oak_door[half=lower]");
        place(
            &mut level,
            powered.above(),
            "minecraft:oak_door[half=upper]",
        );
        level.take_game_events();
        place(
            &mut level,
            powered.relative(Direction::East, 1),
            "minecraft:redstone_block",
        );
        let opened = posted(&mut level);
        assert!(!opened.is_empty());
        assert!(
            opened
                .iter()
                .all(|event| *event == ("minecraft:block_open", None)),
            "redstone opens it without a source, even inside a player's action: {opened:?}"
        );
    }
}
