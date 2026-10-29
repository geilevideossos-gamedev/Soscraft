use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MobGameEvent {
    pub event: &'static str,
    pub pos: DVec3,
    pub mob: Option<u64>,
    pub owner: Option<u64>,
    pub affected: Option<String>,
}

impl EntityWorld {
    pub fn game_event(
        &mut self,
        event: &'static str,
        pos: DVec3,
        mob: Option<u64>,
        affected: Option<String>,
    ) {
        self.game_events.push(MobGameEvent {
            event,
            pos,
            mob,
            owner: None,
            affected,
        });
    }

    pub fn take_game_events(&mut self) -> Vec<MobGameEvent> {
        std::mem::take(&mut self.game_events)
    }

    pub fn grounded(&mut self) -> Vec<(u64, DVec3)> {
        let ids: Vec<u64> = self.mob_ids().collect();
        ids.into_iter()
            .filter_map(|id| {
                let body = self.body_mut(id)?;
                body.on_ground.then_some((id, body.position))
            })
            .collect()
    }
}
