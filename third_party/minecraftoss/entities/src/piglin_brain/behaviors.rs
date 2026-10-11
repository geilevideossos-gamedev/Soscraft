use super::{PiglinBrain, PiglinCtx};
use crate::brain_behaviors::{Pos, Track, WalkGoal, block_of, find, shuffled};
use crate::monster_ai::Target;
use crate::piglin::PiglinKind;
use glam::DVec3;

pub const STROLL_SPEED: f32 = 0.6;
pub const LOOK_DISTANCE: f64 = 8.0;
pub const INTERACT_DISTANCE: f64 = 8.0;
pub const PLAYER_INTERACT_DISTANCE: f64 = 4.0;
pub const HOME_CLOSE_ENOUGH: i32 = 2;
pub const HOME_TOO_FAR: f64 = 100.0;
pub const HOME_STROLL_AROUND: f64 = 5.0;
pub const DOORS_TAG: &str = "minecraft:mob_interactable_doors";

#[derive(Clone, Debug, Default)]
pub struct Doors {
    last_node: Option<Pos>,
    cooldown: i32,
    to_close: Option<Vec<Pos>>,
}

#[derive(Clone, Copy)]
enum LookFor {
    Player,
    Kind(&'static str),
    Anything,
}

impl PiglinBrain {
    fn sees_track(&self, track: Track, ctx: &PiglinCtx) -> bool {
        match track {
            Track::Entity { target, .. } => {
                find(ctx.others, target).is_some_and(|other| other.alive)
                    && self.memories.sees(target)
            }
            Track::Block(_) => true,
        }
    }

    pub(super) fn start_look(&mut self, ctx: &mut PiglinCtx) {
        if self.look_until.is_some() {
            return;
        }
        if self
            .memories
            .look_target
            .get()
            .is_some_and(|&track| self.sees_track(track, ctx))
        {
            self.look_until = Some(ctx.time + 45 + i64::from(ctx.level_random.next_int(46)));
        }
    }

    pub(super) fn tick_look(&mut self, ctx: &mut PiglinCtx) {
        let Some(until) = self.look_until else {
            return;
        };
        let track = self
            .memories
            .look_target
            .get()
            .copied()
            .filter(|&track| self.sees_track(track, ctx));
        match track.and_then(|track| track.position(ctx.others)) {
            Some(at) if ctx.time <= until => ctx.look_at = Some(at),
            _ => {
                self.look_until = None;
                self.memories.look_target.erase();
            }
        }
    }

    fn door_open(ctx: &PiglinCtx, pos: Pos) -> Option<bool> {
        if let Some(&(_, open)) = ctx.doors.iter().rev().find(|(p, _)| *p == pos) {
            return Some(open);
        }
        let block = ctx.world.block(pos)?;
        if !block.id.ends_with("_door") || !ctx.world.block_in_tag(pos, DOORS_TAG) {
            return None;
        }
        Some(block.property("open") == Some("true"))
    }

    pub(super) fn interact_with_doors(&mut self, ctx: &mut PiglinCtx) {
        if !self.walk.running() {
            return;
        }
        let nav = &*ctx.navigation;
        if nav.next == 0 || nav.next >= nav.nodes.len() {
            return;
        }
        let (from, to) = (nav.nodes[nav.next - 1], nav.nodes[nav.next]);
        let doors = &mut self.doors;
        if doors.last_node == Some(to) {
            doors.cooldown = 20;
        } else {
            doors.cooldown -= 1;
            if doors.cooldown > 0 {
                return;
            }
        }
        doors.last_node = Some(to);
        let mut remembered = doors.to_close.take();
        if let Some(open) = Self::door_open(ctx, from) {
            if !open {
                ctx.doors.push((from, true));
            }
            let set = remembered.get_or_insert_with(Vec::new);
            if !set.contains(&from) {
                set.push(from);
            }
        }
        if Self::door_open(ctx, to) == Some(false) {
            ctx.doors.push((to, true));
            let set = remembered.get_or_insert_with(Vec::new);
            if !set.contains(&to) {
                set.push(to);
            }
        }
        let Some(set) = remembered else {
            return;
        };
        let me = ctx.position();
        let mut kept = Vec::new();
        for door in set {
            if door == from || door == to {
                kept.push(door);
                continue;
            }
            let center = DVec3::new(
                f64::from(door.0) + 0.5,
                f64::from(door.1) + 0.5,
                f64::from(door.2) + 0.5,
            );
            if center.distance_squared(me) >= 9.0 {
                continue;
            }
            if Self::door_open(ctx, door) == Some(true) {
                ctx.doors.push((door, false));
            }
        }
        self.doors.to_close = Some(kept);
    }

    pub(super) fn look_for_kind(&mut self, ctx: &PiglinCtx, kind: &'static str) -> bool {
        self.look_for(ctx, LookFor::Kind(kind))
    }

    fn look_for(&mut self, ctx: &PiglinCtx, look: LookFor) -> bool {
        if self.memories.look_target.present() || !self.memories.visible_living.present() {
            return false;
        }
        let position = ctx.position();
        let target = self
            .memories
            .visible_living
            .get()
            .into_iter()
            .flatten()
            .filter_map(|&target| find(ctx.others, target))
            .find(|other| {
                let matches = match look {
                    LookFor::Player => other.is_player(),
                    LookFor::Kind(kind) => other.kind == kind,
                    LookFor::Anything => true,
                };
                matches
                    && other.position.distance_squared(position) <= LOOK_DISTANCE * LOOK_DISTANCE
            })
            .map(|other| other.target);
        let Some(target) = target else {
            return false;
        };
        self.memories
            .look_target
            .set(Track::Entity { target, eyes: true });
        true
    }

    pub(super) fn do_nothing(ctx: &mut PiglinCtx) -> i64 {
        ctx.time + 30 + i64::from(ctx.level_random.next_int(31))
    }

    pub(super) fn idle_look(&mut self, ctx: &mut PiglinCtx) {
        if self.look_wait.is_some() {
            return;
        }
        let looks: &[LookFor] = match ctx.kind {
            PiglinKind::Piglin => &[
                LookFor::Player,
                LookFor::Kind("minecraft:piglin"),
                LookFor::Anything,
            ],
            PiglinKind::Brute => &[
                LookFor::Player,
                LookFor::Kind("minecraft:piglin"),
                LookFor::Kind("minecraft:piglin_brute"),
                LookFor::Anything,
            ],
        };
        let weights = vec![1; looks.len() + 1];
        for choice in shuffled(ctx.random, &weights) {
            if choice == looks.len() {
                self.look_wait = Some(Self::do_nothing(ctx));
                return;
            }
            if self.look_for(ctx, looks[choice]) {
                return;
            }
        }
    }

    pub(super) fn stroll(&mut self, ctx: &mut PiglinCtx, horizontal: u32, vertical: u32) -> bool {
        if self.memories.walk_target.present() {
            return false;
        }
        let spot = crate::stroll::land_random_position(
            ctx.world,
            ctx.profile,
            ctx.position(),
            horizontal,
            vertical,
            ctx.random,
        );
        self.memories
            .walk_target
            .set_or_erase(spot.map(|spot| WalkGoal {
                target: Track::Block(block_of(spot)),
                speed: STROLL_SPEED,
                close_enough: 0,
            }));
        true
    }

    fn interact_with(&mut self, ctx: &PiglinCtx, kind: &str) -> bool {
        if self.memories.walk_target.present() || !self.memories.visible_living.present() {
            return false;
        }
        let position = ctx.position();
        let visible: Vec<_> = self
            .memories
            .visible_living
            .get()
            .into_iter()
            .flatten()
            .filter_map(|&target| find(ctx.others, target))
            .filter(|other| other.kind == kind)
            .collect();
        if visible.is_empty() {
            return false;
        }
        let near = visible.iter().find(|other| {
            other.position.distance_squared(position) <= INTERACT_DISTANCE * INTERACT_DISTANCE
        });
        if let Some(other) = near {
            let target = other.target;
            self.memories.interaction_target.set(target);
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
            self.memories.walk_target.set(WalkGoal {
                target: Track::Entity {
                    target,
                    eyes: false,
                },
                speed: STROLL_SPEED,
                close_enough: 2,
            });
        }
        true
    }

    fn walk_to_look_target(&mut self) -> bool {
        if self.memories.nearest_player_holding_wanted_item.present()
            || self.memories.walk_target.present()
        {
            return false;
        }
        let Some(&track) = self.memories.look_target.get() else {
            return false;
        };
        self.memories.walk_target.set(WalkGoal {
            target: track,
            speed: STROLL_SPEED,
            close_enough: 3,
        });
        true
    }

    fn home_center(&self, ctx: &PiglinCtx) -> Option<(Pos, DVec3)> {
        let home = self.memories.home.get()?;
        if home.dimension != ctx.dimension {
            return None;
        }
        let (x, y, z) = home.pos;
        Some((
            home.pos,
            DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5),
        ))
    }

    fn stroll_to_home(&mut self, ctx: &PiglinCtx) -> bool {
        let Some((pos, center)) = self.home_center(ctx) else {
            return false;
        };
        if center.distance_squared(ctx.position()) >= HOME_TOO_FAR * HOME_TOO_FAR {
            return false;
        }
        if ctx.time <= self.home_stroll_next {
            return true;
        }
        self.memories.walk_target.set(WalkGoal {
            target: Track::Block(pos),
            speed: STROLL_SPEED,
            close_enough: HOME_CLOSE_ENOUGH,
        });
        self.home_stroll_next = ctx.time + 80;
        true
    }

    fn stroll_around_home(&mut self, ctx: &mut PiglinCtx) -> bool {
        let Some((_, center)) = self.home_center(ctx) else {
            return false;
        };
        if center.distance_squared(ctx.position()) >= HOME_STROLL_AROUND * HOME_STROLL_AROUND {
            return false;
        }
        if ctx.time <= self.home_around_next {
            return true;
        }
        let spot = crate::stroll::land_random_position(
            ctx.world,
            ctx.profile,
            ctx.position(),
            8,
            6,
            ctx.random,
        );
        self.memories
            .walk_target
            .set_or_erase(spot.map(|spot| WalkGoal {
                target: Track::Block(block_of(spot)),
                speed: STROLL_SPEED,
                close_enough: 1,
            }));
        self.home_around_next = ctx.time + 180;
        true
    }

    pub(super) fn idle_movement(&mut self, ctx: &mut PiglinCtx) {
        if self.move_wait.is_some() {
            return;
        }
        match ctx.kind {
            PiglinKind::Piglin => {
                for choice in shuffled(ctx.random, &[2, 2, 2, 1]) {
                    let started = match choice {
                        0 => self.stroll(ctx, 10, 7),
                        1 => self.interact_with(ctx, "minecraft:piglin"),
                        2 => self.walk_to_look_target(),
                        _ => {
                            self.move_wait = Some(Self::do_nothing(ctx));
                            true
                        }
                    };
                    if started {
                        return;
                    }
                }
            }
            PiglinKind::Brute => {
                for choice in shuffled(ctx.random, &[2, 2, 2, 2, 2, 1]) {
                    let started = match choice {
                        0 => self.stroll(ctx, 10, 7),
                        1 => self.interact_with(ctx, "minecraft:piglin"),
                        2 => self.interact_with(ctx, "minecraft:piglin_brute"),
                        3 => self.stroll_to_home(ctx),
                        4 => self.stroll_around_home(ctx),
                        _ => {
                            self.move_wait = Some(Self::do_nothing(ctx));
                            true
                        }
                    };
                    if started {
                        return;
                    }
                }
            }
        }
    }

    pub(super) fn set_look_and_interact(&mut self, ctx: &PiglinCtx) {
        if self.memories.interaction_target.present() || !self.memories.visible_living.present() {
            return;
        }
        let position = ctx.position();
        let player = self
            .memories
            .visible_living
            .get()
            .into_iter()
            .flatten()
            .filter_map(|&target| find(ctx.others, target))
            .find(|other| {
                other.is_player()
                    && other.position.distance_squared(position)
                        <= PLAYER_INTERACT_DISTANCE * PLAYER_INTERACT_DISTANCE
            })
            .map(|other| other.target);
        if let Some(target) = player {
            self.memories.interaction_target.set(target);
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
        }
    }

    pub(super) fn end_waits(&mut self, time: i64) {
        for wait in [
            &mut self.look_wait,
            &mut self.move_wait,
            &mut self.celebrate_wait,
        ] {
            if wait.is_some_and(|end| time > end) {
                *wait = None;
            }
        }
    }

    pub fn interaction_target(&self) -> Option<Target> {
        self.memories.interaction_target.get().copied()
    }
}
