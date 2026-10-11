use super::{PiglinBrain, PiglinCtx};
use crate::brain_behaviors::{Living, set_list, targetable_within};
use crate::monster_ai::Target;
use glam::DVec3;

pub const SCAN_RATE: i32 = 20;
pub const SENSORS: usize = 5;
pub const NEAREST_LIVING: usize = 0;
pub const NEAREST_PLAYERS: usize = 1;

impl PiglinBrain {
    pub(super) fn sense(&mut self, ctx: &mut PiglinCtx) {
        if !self.started {
            self.started = true;
            for clock in &mut self.sensors {
                *clock = i64::from(ctx.random.next_int(SCAN_RATE as u32));
            }
        }
        let mut fired = [false; SENSORS];
        for (clock, fired) in self.sensors.iter_mut().zip(fired.iter_mut()) {
            *clock -= 1;
            if *clock <= 0 {
                *clock = i64::from(SCAN_RATE);
                *fired = true;
            }
        }
        if fired[NEAREST_LIVING] {
            self.sense_living(ctx);
        }
        if fired[NEAREST_PLAYERS] {
            self.sense_players(ctx);
        }
    }

    fn sense_living(&mut self, ctx: &PiglinCtx) {
        let position = ctx.position();
        let eye = ctx.eye();
        let range = ctx.follow_range;
        let half = f64::from(ctx.body.width) / 2.0;
        let min = position - DVec3::new(half + range, range, half + range);
        let max = position
            + DVec3::new(
                half + range,
                f64::from(ctx.body.height) + range,
                half + range,
            );
        let mut found: Vec<(&Living, f64)> = ctx
            .others
            .iter()
            .filter(|other| other.alive && !other.spectator)
            .filter(|other| {
                let h = f64::from(other.width) / 2.0;
                let p = other.position;
                p.x - h < max.x
                    && p.x + h > min.x
                    && p.y < max.y
                    && p.y + f64::from(other.height) > min.y
                    && p.z - h < max.z
                    && p.z + h > min.z
            })
            .map(|other| (other, other.position.distance_squared(position)))
            .collect();
        found.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let visible: Vec<Target> = found
            .iter()
            .filter(|(other, _)| targetable_within(ctx.world, eye, position, other, false, range))
            .map(|(other, _)| other.target)
            .collect();
        set_list(
            &mut self.memories.nearest_living,
            found.iter().map(|(other, _)| other.target).collect(),
        );
        self.memories.visible_living.set(visible);
    }

    fn sense_players(&mut self, ctx: &PiglinCtx) {
        let position = ctx.position();
        let eye = ctx.eye();
        let range = ctx.follow_range;
        let mut players: Vec<(&Living, f64)> = ctx
            .others
            .iter()
            .filter(|other| other.is_player() && other.alive && !other.spectator)
            .map(|other| (other, other.position.distance_squared(position)))
            .filter(|(_, distance)| *distance < range * range)
            .collect();
        players.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        set_list(
            &mut self.memories.nearest_players,
            players.iter().map(|(other, _)| other.target).collect(),
        );
        let visible: Vec<&Living> = players
            .iter()
            .filter(|(other, _)| targetable_within(ctx.world, eye, position, other, false, range))
            .map(|(other, _)| *other)
            .collect();
        self.memories
            .nearest_visible_player
            .set_or_erase(visible.first().map(|other| other.target));
        let attackable = visible
            .iter()
            .find(|other| targetable_within(ctx.world, eye, position, other, true, range))
            .map(|other| other.target);
        self.memories
            .nearest_visible_attackable_player
            .set_or_erase(attackable);
    }
}
