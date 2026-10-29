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
