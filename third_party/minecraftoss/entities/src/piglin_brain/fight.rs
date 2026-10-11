use super::sensors::ZOMBIFIED;
use super::{Activity, PiglinBrain, PiglinCtx};
use crate::brain_behaviors::{Living, Track, WalkGoal, block_of, within_melee_range};
use crate::monster_ai::Target;
use crate::piglin::PiglinKind;
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const ANGER_TICKS: i64 = 600;
pub const CELEBRATE_TICKS: i64 = 300;
pub const ATTACK_COOLDOWN: i64 = 20;
pub const GIVE_UP_REACHING: i64 = 200;
pub const BACK_UP_DISTANCE: f64 = 5.0;
pub const BACK_UP_SPEED: f32 = 0.75;
pub const CHASE_SPEED: f32 = 1.0;
pub const CROSSBOW_RANGE: f64 = 8.0;
pub const CROSSBOW_CHARGE: i32 = 25;
pub const CROSSBOW_TIMEOUT: i64 = 1200;
pub const BRUTE_TARGET_RANGE: f64 = 12.0;
pub const NEAR_ZOMBIFIED: f64 = 6.0;
pub const AVOID_SPEED: f32 = 1.0;
pub const AVOID_DISTANCE: f64 = 12.0;
pub const CELEBRATE_SPEED: f32 = 1.0;
pub const DANCE_SPEED: f32 = 0.6;
pub const AVOID_MIN_TICKS: i64 = 100;
pub const AVOID_MAX_TICKS: i64 = 140;
pub const REPELLENT_SPEED: f32 = 1.0;
pub const REPELLENT_DISTANCE: f64 = 8.0;
pub const BABY_FLEE_TICKS: i64 = 100;
pub const ADMIRING_DISABLED_TICKS: i64 = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemUse {
    Start,
    Release,
    Stop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossbowState {
    #[default]
    Uncharged,
    Charging,
    Charged,
    ReadyToAttack,
}

#[derive(Clone, Debug, Default)]
pub struct CrossbowAttack {
    pub state: CrossbowState,
    attack_delay: i32,
    until: Option<i64>,
}

impl CrossbowAttack {
    pub fn running(&self) -> bool {
        self.until.is_some()
    }
}

pub fn attackable_ignoring_sight(position: DVec3, range: f64, other: &Living) -> bool {
    let range = range.max(2.0);
    other.alive
        && other.attackable
        && !other.spectator
        && other.position.distance_squared(position) <= range * range
}

fn can_attack(other: &Living) -> bool {
    other.alive && other.attackable && !other.spectator
}

impl PiglinBrain {
    pub fn angry_target(&self, ctx: &PiglinCtx) -> Option<Target> {
        self.memories
            .angry_at
            .get()
            .and_then(|&uuid| ctx.target_of(uuid))
    }

    pub fn near_zombified(&self, ctx: &PiglinCtx) -> bool {
        self.memories
            .nearest_visible_zombified
            .get()
            .and_then(|&target| ctx.living(target))
            .is_some_and(|other| {
                other.position.distance_squared(ctx.position()) < NEAR_ZOMBIFIED * NEAR_ZOMBIFIED
            })
    }

    pub fn nearest_valid_attack_target(&self, ctx: &PiglinCtx) -> Option<Target> {
        let memories = &self.memories;
        if ctx.kind == PiglinKind::Brute {
            if let Some(angry) = self.angry_target(ctx) {
                if ctx.attackable_ignoring_sight(angry) {
                    return Some(angry);
                }
            }
            let within = |&target: &Target| {
                ctx.living(target).is_some_and(|other| {
                    other.position.distance_squared(ctx.position())
                        < BRUTE_TARGET_RANGE * BRUTE_TARGET_RANGE
                })
            };
            return memories
                .nearest_visible_attackable_player
                .get()
                .copied()
                .filter(within)
                .or_else(|| {
                    memories
                        .nearest_visible_nemesis
                        .get()
                        .copied()
                        .filter(within)
                });
        }
        if self.near_zombified(ctx) {
            return None;
        }
        if let Some(angry) = self.angry_target(ctx) {
            if ctx.attackable_ignoring_sight(angry) {
                return Some(angry);
            }
        }
        if memories.universal_anger.present() {
            if let Some(&player) = memories.nearest_visible_attackable_player.get() {
                return Some(player);
            }
        }
        if let Some(&nemesis) = memories.nearest_visible_nemesis.get() {
            return Some(nemesis);
        }
        memories
            .nearest_targetable_player_not_wearing_gold
            .get()
            .copied()
            .filter(|&player| ctx.attackable(player))
    }

    pub(super) fn start_attacking(&mut self, ctx: &PiglinCtx) {
        if self.memories.attack_target.present() || ctx.baby {
            return;
        }
        let Some(target) = self.nearest_valid_attack_target(ctx) else {
            return;
        };
        if !ctx.living(target).is_some_and(can_attack) {
            return;
        }
        self.memories.attack_target.set(target);
        self.memories.cant_reach_walk_target_since.erase();
    }

    pub(super) fn start_celebrating_if_target_dead(&mut self, ctx: &PiglinCtx) {
        if self.memories.celebrate_location.present() {
            return;
        }
        let Some(&target) = self.memories.attack_target.get() else {
            return;
        };
        let Some(other) = ctx.living(target).copied() else {
            return;
        };
        if other.alive {
            return;
        }
        if wants_to_dance(ctx.time, &other) {
            self.memories.dancing.set_for((), CELEBRATE_TICKS);
        }
        self.memories
            .celebrate_location
            .set_for(block_of(other.position), CELEBRATE_TICKS);
        if !other.is_player() || ctx.forgive_dead_players {
            self.memories.attack_target.erase();
            self.memories.angry_at.erase();
        }
    }

    pub(super) fn stop_being_angry_if_target_dead(&mut self, ctx: &PiglinCtx) {
        let Some(target) = self.angry_target(ctx) else {
            return;
        };
        let Some(other) = ctx.living(target) else {
            return;
        };
        if !other.alive && (!other.is_player() || ctx.forgive_dead_players) {
            self.memories.angry_at.erase();
        }
    }

    fn within_attack_range(&self, ctx: &PiglinCtx, other: &Living, margin: f64) -> bool {
        if ctx.holds_crossbow {
            let range = CROSSBOW_RANGE - margin;
            other.position.distance_squared(ctx.position()) < range * range
        } else {
            within_melee_range(ctx.body, other)
        }
    }

    pub(super) fn fight(&mut self, ctx: &mut PiglinCtx) {
        self.stop_attacking_if_invalid(ctx);
        self.back_up_if_too_close(ctx);
        self.walk_to_attack_target(ctx);
        self.melee_attack(ctx);
        self.start_crossbow(ctx);
        if self.near_zombified(ctx) {
            self.memories.attack_target.erase();
        }
    }

    fn stop_attacking_if_invalid(&mut self, ctx: &PiglinCtx) {
        let Some(&target) = self.memories.attack_target.get() else {
            return;
        };
        let living = ctx.living(target).is_some_and(can_attack);
        let tired = self
            .memories
            .cant_reach_walk_target_since
            .get()
            .is_some_and(|&since| ctx.time - since > GIVE_UP_REACHING);
        if !living || tired || self.nearest_valid_attack_target(ctx) != Some(target) {
            self.memories.attack_target.erase();
        }
    }

    fn back_up_if_too_close(&mut self, ctx: &mut PiglinCtx) {
        if !ctx.holds_crossbow
            || self.memories.walk_target.present()
            || !self.memories.visible_living.present()
        {
            return;
        }
        let Some(&target) = self.memories.attack_target.get() else {
            return;
        };
        let Some(other) = ctx.living(target) else {
            return;
        };
        let close =
            other.position.distance_squared(ctx.position()) < BACK_UP_DISTANCE * BACK_UP_DISTANCE;
        if close && self.memories.sees(target) {
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
            ctx.strafe = Some(-BACK_UP_SPEED);
        }
    }

    fn walk_to_attack_target(&mut self, ctx: &PiglinCtx) {
        let Some(&target) = self.memories.attack_target.get() else {
            return;
        };
        let Some(other) = ctx.living(target).copied() else {
            return;
        };
        if self.memories.sees(target) && self.within_attack_range(ctx, &other, 1.0) {
            self.memories.walk_target.erase();
        } else {
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
            self.memories.walk_target.set(WalkGoal {
                target: Track::Entity {
                    target,
                    eyes: false,
                },
                speed: CHASE_SPEED,
                close_enough: 0,
            });
        }
    }

    fn melee_attack(&mut self, ctx: &mut PiglinCtx) {
        if ctx.holds_crossbow
            || self.memories.attack_cooling_down.present()
            || !self.memories.visible_living.present()
        {
            return;
        }
        let Some(&target) = self.memories.attack_target.get() else {
            return;
        };
        let Some(other) = ctx.living(target).copied() else {
            return;
        };
        if within_melee_range(ctx.body, &other) && self.memories.sees(target) {
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
            ctx.attack = Some(target);
            self.memories
                .attack_cooling_down
                .set_for((), ATTACK_COOLDOWN);
        }
    }

    fn crossbow_usable(&self, ctx: &PiglinCtx) -> Option<Target> {
        let &target = self.memories.attack_target.get()?;
        let other = ctx.living(target)?;
        (ctx.holds_crossbow
            && self.memories.sees(target)
            && self.within_attack_range(ctx, other, 0.0))
        .then_some(target)
    }

    fn start_crossbow(&mut self, ctx: &PiglinCtx) {
        if self.crossbow.running() || self.crossbow_usable(ctx).is_none() {
            return;
        }
        self.crossbow.until = Some(ctx.time + CROSSBOW_TIMEOUT);
    }

    pub(super) fn tick_crossbow(&mut self, ctx: &mut PiglinCtx) {
        let Some(until) = self.crossbow.until else {
            return;
        };
        let target = self.crossbow_usable(ctx);
        let Some(target) = target.filter(|_| ctx.time <= until) else {
            self.crossbow.until = None;
            if ctx.using_item.is_some() {
                ctx.item_use = Some(ItemUse::Stop);
            }
            ctx.charging = Some(false);
            return;
        };
        self.memories
            .look_target
            .set(Track::Entity { target, eyes: true });
        let crossbow = &mut self.crossbow;
        match crossbow.state {
            CrossbowState::Uncharged => {
                ctx.item_use = Some(ItemUse::Start);
                crossbow.state = CrossbowState::Charging;
                ctx.charging = Some(true);
            }
            CrossbowState::Charging => {
                if ctx.using_item.is_none() {
                    crossbow.state = CrossbowState::Uncharged;
                }
                if ctx.using_item.unwrap_or(0) >= CROSSBOW_CHARGE {
                    ctx.item_use = Some(ItemUse::Release);
                    crossbow.state = CrossbowState::Charged;
                    crossbow.attack_delay = 20 + ctx.random.next_int(20) as i32;
                    ctx.charging = Some(false);
                }
            }
            CrossbowState::Charged => {
                crossbow.attack_delay -= 1;
                if crossbow.attack_delay == 0 {
                    crossbow.state = CrossbowState::ReadyToAttack;
                }
            }
            CrossbowState::ReadyToAttack => {
                ctx.shot = Some(target);
                crossbow.state = CrossbowState::Uncharged;
            }
        }
    }

    pub(super) fn go_to_celebrate_location(&mut self, ctx: &mut PiglinCtx) {
        if self.memories.walk_target.present() {
            return;
        }
        let Some(&(x, y, z)) = self.memories.celebrate_location.get() else {
            return;
        };
        let (close_enough, speed) = if self.memories.dancing.present() {
            (4, DANCE_SPEED)
        } else {
            (2, CELEBRATE_SPEED)
        };
        let at = block_of(ctx.position());
        let (dx, dy, dz) = (x - at.0, y - at.1, z - at.2);
        if dx * dx + dy * dy + dz * dz < close_enough * close_enough {
            return;
        }
        let near = (
            x + ctx.level_random.next_int(3) as i32 - 1,
            y,
            z + ctx.level_random.next_int(3) as i32 - 1,
        );
        self.memories.walk_target.set(WalkGoal {
            target: Track::Block(near),
            speed,
            close_enough,
        });
        self.memories.look_target.set(Track::Block(near));
    }

    pub(super) fn celebrate_movement(&mut self, ctx: &mut PiglinCtx) {
        if self.celebrate_wait.is_some() {
            return;
        }
        for choice in crate::brain_behaviors::shuffled(ctx.random, &[1, 1, 1]) {
            let started = match choice {
                0 => self.look_for_kind(ctx, "minecraft:piglin"),
                1 => self.stroll(ctx, 2, 1),
                _ => {
                    self.celebrate_wait =
                        Some(ctx.time + 10 + i64::from(ctx.level_random.next_int(11)));
                    true
                }
            };
            if started {
                return;
            }
        }
    }

    fn avoid_for(&mut self, target: Target, ctx: &mut PiglinCtx) {
        let spread = (AVOID_MAX_TICKS - AVOID_MIN_TICKS + 1) as u32;
        let ticks = AVOID_MIN_TICKS + i64::from(ctx.level_random.next_int(spread));
        self.memories.avoid_target.set_for(target, ticks);
    }

    pub(super) fn baby_avoid_nemesis(&mut self, ctx: &mut PiglinCtx) {
        if !ctx.baby || self.memories.avoid_target.present() {
            return;
        }
        if let Some(&nemesis) = self.memories.nearest_visible_nemesis.get() {
            self.avoid_for(nemesis, ctx);
        }
    }

    pub(super) fn avoid_zombified(&mut self, ctx: &mut PiglinCtx) {
        if self.memories.avoid_target.present() || !self.near_zombified(ctx) {
            return;
        }
        if let Some(&zombified) = self.memories.nearest_visible_zombified.get() {
            self.avoid_for(zombified, ctx);
        }
    }

    pub(super) fn avoid_repellent(&mut self, ctx: &mut PiglinCtx) {
        let Some(&(x, y, z)) = self.memories.nearest_repellent.get() else {
            return;
        };
        let from = DVec3::new(f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5);
        crate::brain_behaviors::walk_away(
            &mut self.memories.walk_target,
            &mut ctx.walker(),
            from,
            REPELLENT_SPEED,
            REPELLENT_DISTANCE,
            false,
        );
    }

    pub fn hoglins_outnumber_piglins(&self) -> bool {
        let piglins = self.memories.visible_adult_piglin_count.get().copied();
        let hoglins = self.memories.visible_adult_hoglin_count.get().copied();
        hoglins.unwrap_or(0) > piglins.unwrap_or(0) + 1
    }

    fn wants_to_stop_fleeing(&self, ctx: &PiglinCtx) -> bool {
        let Some(&avoided) = self.memories.avoid_target.get() else {
            return true;
        };
        match ctx.living(avoided).map(|other| other.kind) {
            None => true,
            Some("minecraft:hoglin") => !self.hoglins_outnumber_piglins(),
            Some(kind) if ZOMBIFIED.contains(&kind) => {
                self.memories.nearest_visible_zombified.get() != Some(&avoided)
            }
            Some(_) => false,
        }
    }

    pub(super) fn stop_fleeing_if_safe(&mut self, ctx: &PiglinCtx) {
        if self.wants_to_stop_fleeing(ctx) {
            self.memories.avoid_target.erase();
        }
    }

    pub(super) fn walk_away_from_avoid_target(&mut self, ctx: &mut PiglinCtx) {
        let Some(from) = self
            .memories
            .avoid_target
            .get()
            .and_then(|&target| ctx.living(target))
            .map(|other| other.position)
        else {
            return;
        };
        crate::brain_behaviors::walk_away(
            &mut self.memories.walk_target,
            &mut ctx.walker(),
            from,
            AVOID_SPEED,
            AVOID_DISTANCE,
            true,
        );
    }

    pub fn set_anger(&mut self, uuid: u128, player: bool, universal_anger: bool) {
        self.memories.cant_reach_walk_target_since.erase();
        self.memories.angry_at.set_for(uuid, ANGER_TICKS);
        if player && universal_anger {
            self.memories.universal_anger.set_for((), ANGER_TICKS);
        }
    }

    pub fn active(&self, activity: Activity) -> bool {
        self.activity == activity
    }
}

pub fn wants_to_dance(time: i64, victim: &Living) -> bool {
    victim.kind == "minecraft:hoglin" && LegacyRandom::new(time as u64).next_float() < 0.1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::Body;
    use crate::navigation::GroundNavigation;
    use crate::walk_path::WalkProfile;
    use minecraftoss_player::{Block, Pos, World};

    struct Open;

    impl World for Open {
        fn block(&self, _: Pos) -> Option<Block> {
            None
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn living(target: Target, kind: &'static str, x: f64) -> Living {
        Living {
            target,
            kind,
            position: DVec3::new(x, 64.0, 0.5),
            eye_height: 1.6,
            width: 0.6,
            height: 1.8,
            alive: true,
            baby: false,
            attackable: true,
            spectator: false,
        }
    }

    #[test]
    fn the_nearest_valid_target_order_is_zombified_angry_universal_nemesis_then_ungilded() {
        let mut others = vec![
            living(Target::Mob(1), "minecraft:zombified_piglin", 4.5),
            living(Target::Mob(2), "minecraft:cow", 5.5),
            living(Target::Player(3), "minecraft:player", 6.5),
            living(Target::Mob(4), "minecraft:wither_skeleton", 7.5),
            living(Target::Player(5), "minecraft:player", 8.5),
        ];
        let uuids: Vec<(Target, u128)> = others
            .iter()
            .enumerate()
            .map(|(i, other)| (other.target, 100 + i as u128))
            .collect();
        let mut brain = PiglinBrain::default();
        let memories = &mut brain.memories;
        memories.nearest_visible_zombified.set(Target::Mob(1));
        memories.angry_at.set(101);
        memories.universal_anger.set(());
        memories
            .nearest_visible_attackable_player
            .set(Target::Player(3));
        memories.nearest_visible_nemesis.set(Target::Mob(4));
        memories
            .nearest_targetable_player_not_wearing_gold
            .set(Target::Player(5));
        let body = Body::new(DVec3::new(0.5, 64.0, 0.5), 0.6, 1.95);
        let (mut random, mut level_random) = (LegacyRandom::new(1), LegacyRandom::new(2));
        let mut navigation = GroundNavigation::default();
        let profile = WalkProfile::animal(0.6, 1.95);
        let mut order = Vec::new();
        for step in 0..6 {
            if step == 1 {
                others[0].position.x = 20.5;
            }
            let ctx = PiglinCtx {
                id: 9,
                kind: PiglinKind::Piglin,
                body: &body,
                eye_height: 1.79,
                baby: false,
                follow_range: 16.0,
                others: &others,
                world: &Open,
                time: 0,
                dimension: "minecraft:the_nether",
                random: &mut random,
                level_random: &mut level_random,
                navigation: &mut navigation,
                profile: &profile,
                fluid: Default::default(),
                converting: false,
                uuids: &uuids,
                gilded: &[],
                loved_holders: &[],
                universal_anger: false,
                forgive_dead_players: true,
                holds_crossbow: false,
                using_item: None,
                wanted_items: None,
                offhand: false,
                offhand_shield: false,
                offhand_loved: false,
                look_at: None,
                sounds: Vec::new(),
                doors: Vec::new(),
                attack: None,
                shot: None,
                strafe: None,
                item_use: None,
                charging: None,
                stop_holding: None,
            };
            order.push(brain.nearest_valid_attack_target(&ctx));
            let memories = &mut brain.memories;
            match step {
                1 => memories.angry_at.erase(),
                2 => memories.universal_anger.erase(),
                3 => memories.nearest_visible_nemesis.erase(),
                4 => memories.nearest_targetable_player_not_wearing_gold.erase(),
                _ => {}
            }
        }
        assert_eq!(
            order,
            [
                None,
                Some(Target::Mob(2)),
                Some(Target::Player(3)),
                Some(Target::Mob(4)),
                Some(Target::Player(5)),
                None
            ]
        );
    }

    #[test]
    fn only_a_hoglin_kill_can_end_in_a_dance() {
        let hoglin = living(Target::Mob(1), "minecraft:hoglin", 0.0);
        let cow = living(Target::Mob(2), "minecraft:cow", 0.0);
        let days = |n: i64| (0..n).map(|day| day * 24_000 + 6_000);
        let dances = days(2000)
            .filter(|&time| wants_to_dance(time, &hoglin))
            .count();
        assert!((100..300).contains(&dances), "{dances}");
        assert!(days(2000).all(|time| !wants_to_dance(time, &cow)));
    }
}
