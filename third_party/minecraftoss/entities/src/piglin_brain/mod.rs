mod admire;
mod behaviors;
mod fight;
mod memory;
mod sensors;

pub use admire::{ADMIRE_TICKS, ATE_RECENTLY_TICKS, ITEM_EYE_HEIGHT, WantedItem};
pub use behaviors::Doors;
pub use fight::{
    ADMIRING_DISABLED_TICKS, ANGER_TICKS, BABY_FLEE_TICKS, CROSSBOW_CHARGE, CrossbowAttack,
    CrossbowState, ItemUse, attackable_ignoring_sight,
};
pub use memory::{GlobalPos, Memories};
pub use sensors::{NEMESES, ZOMBIFIED, adult_piglin};

use crate::brain_behaviors::{Living, MoveSink, Pos, Walker, find};
use crate::fluid::FluidFrame;
use crate::monster_ai::Target;
use crate::movement::Body;
use crate::navigation::GroundNavigation;
use crate::piglin::PiglinKind;
use crate::walk_path::WalkProfile;
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Idle,
    AdmireItem,
    Fight,
    Celebrate,
    Avoid,
    Ride,
}

pub struct PiglinCtx<'a> {
    pub id: u64,
    pub kind: PiglinKind,
    pub body: &'a Body,
    pub eye_height: f32,
    pub baby: bool,
    pub follow_range: f64,
    pub others: &'a [Living],
    pub world: &'a dyn World,
    pub time: i64,
    pub dimension: &'a str,
    pub random: &'a mut LegacyRandom,
    pub level_random: &'a mut LegacyRandom,
    pub navigation: &'a mut GroundNavigation,
    pub profile: &'a WalkProfile,
    pub fluid: FluidFrame,
    pub converting: bool,
    pub uuids: &'a [(Target, u128)],
    pub gilded: &'a [Target],
    pub loved_holders: &'a [Target],
    pub universal_anger: bool,
    pub forgive_dead_players: bool,
    pub holds_crossbow: bool,
    pub using_item: Option<i32>,
    pub wanted_items: Option<&'a [WantedItem]>,
    pub offhand: bool,
    pub offhand_shield: bool,
    pub offhand_loved: bool,
    pub look_at: Option<DVec3>,
    pub sounds: Vec<&'static str>,
    pub doors: Vec<(Pos, bool)>,
    pub attack: Option<Target>,
    pub shot: Option<Target>,
    pub strafe: Option<f32>,
    pub item_use: Option<ItemUse>,
    pub charging: Option<bool>,
    pub stop_holding: Option<bool>,
}

impl PiglinCtx<'_> {
    pub fn position(&self) -> DVec3 {
        self.body.position
    }

    pub fn eye(&self) -> DVec3 {
        self.body.position + DVec3::Y * f64::from(self.eye_height)
    }

    pub fn living(&self, target: Target) -> Option<&Living> {
        find(self.others, target)
    }

    pub fn target_of(&self, uuid: u128) -> Option<Target> {
        self.uuids
            .iter()
            .find(|(_, other)| *other == uuid)
            .map(|(target, _)| *target)
    }

    pub fn attackable_ignoring_sight(&self, target: Target) -> bool {
        self.living(target).is_some_and(|other| {
            attackable_ignoring_sight(self.position(), self.follow_range, other)
        })
    }

    pub fn attackable(&self, target: Target) -> bool {
        self.living(target).is_some_and(|other| {
            crate::brain_behaviors::targetable_within(
                self.world,
                self.eye(),
                self.position(),
                other,
                true,
                self.follow_range,
            )
        })
    }

    pub fn walker(&mut self) -> Walker<'_> {
        Walker {
            body: self.body,
            others: self.others,
            world: self.world,
            time: self.time,
            random: self.random,
            navigation: self.navigation,
            profile: self.profile,
            fluid: self.fluid,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PiglinBrain {
    pub memories: Memories,
    pub activity: Activity,
    started: bool,
    sensors: [i64; sensors::SENSORS],
    look_until: Option<i64>,
    pub walk: MoveSink,
    look_wait: Option<i64>,
    move_wait: Option<i64>,
    celebrate_wait: Option<i64>,
    doors: Doors,
    home_stroll_next: i64,
    home_around_next: i64,
    pub crossbow: CrossbowAttack,
}

impl PiglinBrain {
    pub fn tick(&mut self, ctx: &mut PiglinCtx) {
        self.memories.tick();
        self.sense(ctx);
        self.follow_wanted_item(ctx);
        self.start_look(ctx);
        self.walk.try_start(
            &mut self.memories.walk_target,
            &mut self.memories.cant_reach_walk_target_since,
            &mut ctx.walker(),
        );
        self.interact_with_doors(ctx);
        if ctx.kind == PiglinKind::Piglin {
            self.baby_avoid_nemesis(ctx);
            self.avoid_zombified(ctx);
            self.stop_holding_if_no_longer_admiring(ctx);
            self.start_admiring_item_if_seen(ctx);
            self.start_celebrating_if_target_dead(ctx);
        }
        self.stop_being_angry_if_target_dead(ctx);
        match self.activity {
            Activity::Idle => {
                if ctx.kind == PiglinKind::Piglin {
                    self.look_at_loved_holder(ctx);
                }
                self.start_attacking(ctx);
                if ctx.kind == PiglinKind::Piglin {
                    self.avoid_repellent(ctx);
                }
                self.idle_look(ctx);
                self.idle_movement(ctx);
                self.set_look_and_interact(ctx);
            }
            Activity::Fight => self.fight(ctx),
            Activity::AdmireItem => self.admire(ctx),
            Activity::Celebrate => {
                self.avoid_repellent(ctx);
                self.look_at_loved_holder(ctx);
                self.start_attacking(ctx);
                self.go_to_celebrate_location(ctx);
                self.celebrate_movement(ctx);
            }
            Activity::Avoid if ctx.kind == PiglinKind::Piglin => {
                self.walk_away_from_avoid_target(ctx);
                self.idle_look(ctx);
                self.idle_movement(ctx);
                self.stop_fleeing_if_safe(ctx);
            }
            _ => {}
        }
        self.tick_crossbow(ctx);
        self.tick_look(ctx);
        self.walk.tick(
            &mut self.memories.walk_target,
            &mut self.memories.cant_reach_walk_target_since,
            &mut ctx.walker(),
        );
        self.end_waits(ctx.time);
    }

    fn first_valid(&self, kind: PiglinKind) -> Activity {
        let memories = &self.memories;
        match kind {
            PiglinKind::Piglin if memories.admiring_item.present() => Activity::AdmireItem,
            _ if memories.attack_target.present() => Activity::Fight,
            PiglinKind::Piglin if memories.avoid_target.present() => Activity::Avoid,
            PiglinKind::Piglin if memories.celebrate_location.present() => Activity::Celebrate,
            _ => Activity::Idle,
        }
    }

    pub fn update_activity(&mut self, ctx: &mut PiglinCtx) {
        let before = self.activity;
        let after = self.first_valid(ctx.kind);
        if before != after {
            let memories = &mut self.memories;
            if after != Activity::Fight {
                memories.attack_target.erase();
            }
            if ctx.kind == PiglinKind::Piglin {
                if after != Activity::AdmireItem {
                    memories.admiring_item.erase();
                }
                if after != Activity::Avoid {
                    memories.avoid_target.erase();
                }
                if after != Activity::Celebrate {
                    memories.celebrate_location.erase();
                }
            }
            self.activity = after;
            match ctx.kind {
                PiglinKind::Piglin => {
                    let sound = self.activity_sound(ctx.position(), ctx.others, ctx.converting);
                    ctx.sounds.push(sound);
                }
                PiglinKind::Brute if after == Activity::Fight => {
                    ctx.sounds.push("entity.piglin_brute.angry");
                }
                PiglinKind::Brute => {}
            }
        }
        if !self.memories.celebrate_location.present() {
            self.memories.dancing.erase();
        }
    }

    pub fn aggressive(&self) -> bool {
        self.memories.attack_target.present()
    }

    pub fn dancing(&self) -> bool {
        self.memories.dancing.present()
    }

    fn near_avoid_target(&self, position: DVec3, others: &[Living]) -> bool {
        self.memories
            .avoid_target
            .get()
            .and_then(|&target| find(others, target))
            .is_some_and(|other| other.position.distance_squared(position) < 12.0 * 12.0)
    }

    pub fn near_repellent(&self) -> bool {
        self.memories.nearest_repellent.present()
    }

    pub fn activity_sound(
        &self,
        position: DVec3,
        others: &[Living],
        converting: bool,
    ) -> &'static str {
        match self.activity {
            Activity::Fight => "entity.piglin.angry",
            _ if converting => "entity.piglin.retreat",
            Activity::Avoid if self.near_avoid_target(position, others) => "entity.piglin.retreat",
            Activity::AdmireItem => "entity.piglin.admiring_item",
            Activity::Celebrate => "entity.piglin.celebrate",
            _ if self.memories.nearest_player_holding_wanted_item.present() => {
                "entity.piglin.jealous"
            }
            _ if self.near_repellent() => "entity.piglin.retreat",
            _ => "entity.piglin.ambient",
        }
    }
}
