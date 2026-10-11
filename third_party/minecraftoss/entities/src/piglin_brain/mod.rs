mod behaviors;
mod memory;
mod sensors;

pub use behaviors::Doors;
pub use memory::{GlobalPos, Memories};

use crate::brain_behaviors::{Living, MoveSink, Pos, Walker, find};
use crate::fluid::FluidFrame;
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
    pub look_at: Option<DVec3>,
    pub sounds: Vec<&'static str>,
    pub doors: Vec<(Pos, bool)>,
}

impl PiglinCtx<'_> {
    pub fn position(&self) -> DVec3 {
        self.body.position
    }

    pub fn eye(&self) -> DVec3 {
        self.body.position + DVec3::Y * f64::from(self.eye_height)
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
    doors: Doors,
    home_stroll_next: i64,
    home_around_next: i64,
}

impl PiglinBrain {
    pub fn tick(&mut self, ctx: &mut PiglinCtx) {
        self.memories.tick();
        self.sense(ctx);
        self.start_look(ctx);
        self.walk.try_start(
            &mut self.memories.walk_target,
            &mut self.memories.cant_reach_walk_target_since,
            &mut ctx.walker(),
        );
        self.interact_with_doors(ctx);
        match self.activity {
            Activity::Idle => {
                self.idle_look(ctx);
                self.idle_movement(ctx);
                self.set_look_and_interact(ctx);
            }
            Activity::Avoid if ctx.kind == PiglinKind::Piglin => {
                self.idle_look(ctx);
                self.idle_movement(ctx);
            }
            _ => {}
        }
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
            match before {
                Activity::AdmireItem => self.memories.admiring_item.erase(),
                Activity::Fight => self.memories.attack_target.erase(),
                Activity::Avoid => self.memories.avoid_target.erase(),
                Activity::Celebrate => self.memories.celebrate_location.erase(),
                Activity::Idle | Activity::Ride => {}
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

    pub fn near_repellent(&self, position: DVec3) -> bool {
        self.memories
            .nearest_repellent
            .get()
            .is_some_and(|&(x, y, z)| {
                let center = DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5);
                center.distance_squared(position) < 8.0 * 8.0
            })
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
            _ if self.near_repellent(position) => "entity.piglin.retreat",
            _ => "entity.piglin.ambient",
        }
    }
}
