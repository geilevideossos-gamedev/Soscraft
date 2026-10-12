use super::{PiglinBrain, PiglinCtx};
use crate::brain_behaviors::{Track, WalkGoal, block_of, find};
use glam::DVec3;

pub const ADMIRE_TICKS: i64 = 119;
pub const ADMIRE_SPEED: f32 = 1.0;
pub const ADMIRE_REACH: f64 = 9.0;
pub const TIRED_OF_REACHING: i32 = 200;
pub const DISABLE_WALK_TICKS: i64 = 200;
pub const ATE_RECENTLY_TICKS: i64 = 200;
pub const JEALOUS_LOOK: f64 = 14.0;
pub const ITEM_EYE_HEIGHT: f64 = 0.2125;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WantedItem {
    pub id: i32,
    pub position: DVec3,
    pub loved: bool,
}

impl PiglinBrain {
    pub fn sees_items_this_tick(&self) -> bool {
        !self.started || self.sensors[super::sensors::NEAREST_ITEMS] <= 1
    }

    pub(super) fn sense_items(&mut self, ctx: &PiglinCtx) {
        let nearest = ctx.wanted_items.and_then(|items| items.first().copied());
        self.memories
            .nearest_visible_wanted_item
            .set_or_erase(nearest);
    }

    pub(super) fn follow_wanted_item(&mut self, ctx: &PiglinCtx) {
        let Some(id) = self
            .memories
            .nearest_visible_wanted_item
            .get()
            .map(|item| item.id)
        else {
            return;
        };
        match ctx.world.item(id).filter(|item| item.count > 0) {
            Some(item) => {
                if let Some(wanted) = self.memories.nearest_visible_wanted_item.get_mut() {
                    wanted.position = item.position;
                }
            }
            None => self.memories.nearest_visible_wanted_item.erase(),
        }
    }

    fn wanted_within(&self, ctx: &PiglinCtx, reach: f64) -> Option<WantedItem> {
        self.memories
            .nearest_visible_wanted_item
            .get()
            .copied()
            .filter(|item| item.position.distance_squared(ctx.position()) < reach * reach)
    }

    pub(super) fn stop_holding_if_no_longer_admiring(&mut self, ctx: &mut PiglinCtx) {
        if self.memories.admiring_item.present() || !ctx.offhand || ctx.offhand_shield {
            return;
        }
        ctx.stop_holding = Some(true);
        ctx.offhand = false;
        ctx.offhand_loved = false;
    }

    pub(super) fn start_admiring_item_if_seen(&mut self, _ctx: &PiglinCtx) {
        let memories = &mut self.memories;
        if memories.admiring_item.present()
            || memories.admiring_disabled.present()
            || memories.disable_walk_to_admire_item.present()
        {
            return;
        }
        if memories
            .nearest_visible_wanted_item
            .get()
            .is_some_and(|item| item.loved)
        {
            memories.admiring_item.set_for((), ADMIRE_TICKS);
        }
    }

    pub(super) fn admire(&mut self, ctx: &mut PiglinCtx) {
        self.go_to_wanted_item(ctx);
        self.stop_admiring_if_too_far(ctx);
        self.stop_admiring_if_tired(ctx);
    }

    fn go_to_wanted_item(&mut self, ctx: &PiglinCtx) {
        if ctx.offhand_loved {
            return;
        }
        let Some(item) = self.wanted_within(ctx, ADMIRE_REACH) else {
            return;
        };
        let at = block_of(item.position);
        self.memories.walk_target.set(WalkGoal {
            target: Track::Block(at),
            speed: ADMIRE_SPEED,
            close_enough: 0,
        });
        self.memories.look_target.set(Track::Block(at));
    }

    fn stop_admiring_if_too_far(&mut self, ctx: &PiglinCtx) {
        if !self.memories.admiring_item.present() || ctx.offhand {
            return;
        }
        if self.wanted_within(ctx, ADMIRE_REACH).is_none() {
            self.memories.admiring_item.erase();
        }
    }

    fn stop_admiring_if_tired(&mut self, ctx: &PiglinCtx) {
        let memories = &mut self.memories;
        if !memories.admiring_item.present()
            || !memories.nearest_visible_wanted_item.present()
            || ctx.offhand
        {
            return;
        }
        match memories.time_trying_to_reach_admire_item.get().copied() {
            None => memories.time_trying_to_reach_admire_item.set(0),
            Some(time) if time > TIRED_OF_REACHING => {
                memories.admiring_item.erase();
                memories.time_trying_to_reach_admire_item.erase();
                memories
                    .disable_walk_to_admire_item
                    .set_for((), DISABLE_WALK_TICKS);
            }
            Some(time) => memories.time_trying_to_reach_admire_item.set(time + 1),
        }
    }

    pub(super) fn look_at_loved_holder(&mut self, ctx: &PiglinCtx) {
        if self.memories.look_target.present() {
            return;
        }
        let position = ctx.position();
        let holder = self
            .memories
            .visible_living
            .get()
            .into_iter()
            .flatten()
            .filter_map(|&target| find(ctx.others, target))
            .find(|other| {
                other.is_player()
                    && ctx.loved_holders.contains(&other.target)
                    && other.position.distance_squared(position) <= JEALOUS_LOOK * JEALOUS_LOOK
            });
        if let Some(other) = holder {
            self.memories.look_target.set(Track::Entity {
                target: other.target,
                eyes: true,
            });
        }
    }

    pub fn admire_for(&mut self, ticks: i64) {
        self.memories.admiring_item.set_for((), ticks);
    }
}
