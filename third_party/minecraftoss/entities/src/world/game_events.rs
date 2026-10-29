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
        let ids: Vec<u64> = self.mob_ids().collect();
        for id in ids {
            let damaged = self
                .damage_state_mut(id)
                .is_some_and(|damage| std::mem::take(&mut damage.damaged));
            let Some(body) = self.body_mut(id) else {
                continue;
            };
            let moved: Vec<_> = body.game_events.drain(..).collect();
            let at = body.position;
            if damaged {
                self.game_event("minecraft:entity_damage", at, Some(id), None);
            }
            for (event, pos, affected) in moved {
                self.game_event(event, pos, Some(id), affected);
            }
        }
        std::mem::take(&mut self.game_events)
    }

    pub(super) fn rider_id(rider: Rider) -> u64 {
        match rider {
            Rider::Mob(id) => id,
            Rider::Player(player) => PLAYER_TARGET + player,
        }
    }

    pub(super) fn arrow_landed(&mut self, id: u64, owner: u64, at: DVec3) {
        let Some(arrow) = self
            .arrows
            .iter()
            .find(|entity| entity.id == id)
            .map(|entity| &entity.arrow)
        else {
            return;
        };
        let (pos, affected) = match arrow.impact_block.filter(|_| arrow.in_ground) {
            Some((x, y, z)) => (
                DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5),
                arrow.impact_state().map(|block| block.id.clone()),
            ),
            None => (at, None),
        };
        self.game_events.push(MobGameEvent {
            event: "minecraft:projectile_land",
            pos,
            mob: None,
            owner: Some(owner),
            affected,
        });
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
