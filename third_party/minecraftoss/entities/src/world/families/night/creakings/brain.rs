use crate::brain_behaviors::{
    Living, MoveSink, Ticker, Track, WalkGoal, Walker, block_of, find, set_list, shuffled,
    targetable_within, within_melee_range,
};
use crate::fluid::FluidFrame;
use crate::monster_ai::Target;
use crate::movement::Body;
use crate::navigation::GroundNavigation;
use crate::villager_brain::Slot;
use crate::walk_path::WalkProfile;
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

pub const SCAN_RATE: i32 = 20;
pub const SENSE_RANGE: f64 = 32.0;
pub const LOOK_DISTANCE: f64 = 8.0;
pub const LOOK_MIN: i64 = 45;
pub const LOOK_MAX: i64 = 90;
pub const STROLL_SPEED: f32 = 0.3;
pub const CHASE_SPEED: f32 = 1.0;
pub const ATTACK_COOLDOWN: i64 = 40;
pub const GIVE_UP_REACHING: i64 = 200;
pub const SWIM_CHANCE: f32 = 0.8;
pub const SWIM_TICKS: i64 = 60;
pub const FLUID_JUMP_THRESHOLD: f64 = 0.4;
pub const IDLE_MIN: i64 = 30;
pub const IDLE_MAX: i64 = 60;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Idle,
    Fight,
}

#[derive(Clone, Debug, Default)]
pub struct Memories {
    pub visible_living: Slot<Vec<Target>>,
    pub nearest_players: Slot<Vec<Target>>,
    pub visible_attackable_players: Slot<Vec<Target>>,
    pub nearest_visible_attackable_player: Slot<Target>,
    pub attack_target: Slot<Target>,
    pub attack_cooling_down: Slot<()>,
    pub walk_target: Slot<WalkGoal>,
    pub look_target: Slot<Track>,
    pub cant_reach_walk_target_since: Slot<i64>,
}

impl Memories {
    pub fn tick(&mut self) {
        self.visible_living.tick();
        self.nearest_players.tick();
        self.visible_attackable_players.tick();
        self.nearest_visible_attackable_player.tick();
        self.attack_target.tick();
        self.attack_cooling_down.tick();
        self.walk_target.tick();
        self.look_target.tick();
        self.cant_reach_walk_target_since.tick();
    }

    pub fn sees(&self, target: Target) -> bool {
        self.visible_living
            .get()
            .is_some_and(|visible| visible.contains(&target))
    }

    pub fn players(&self) -> Vec<Target> {
        self.nearest_players.get().cloned().unwrap_or_default()
    }

    pub fn reachable(&self, target: Target) -> bool {
        self.visible_attackable_players
            .get()
            .is_some_and(|players| players.contains(&target))
    }
}

pub struct Ctx<'a> {
    pub body: &'a Body,
    pub eye_height: f32,
    pub others: &'a [Living],
    pub world: &'a dyn World,
    pub time: i64,
    pub random: &'a mut LegacyRandom,
    pub navigation: &'a mut GroundNavigation,
    pub profile: &'a WalkProfile,
    pub fluid: FluidFrame,
    pub can_move: bool,
    pub active: bool,
    pub peaceful: bool,
    pub look_at: Option<DVec3>,
    pub attack: Option<Target>,
    pub jump: bool,
}

impl Ctx<'_> {
    pub fn position(&self) -> DVec3 {
        self.body.position
    }

    pub fn eye(&self) -> DVec3 {
        self.body.position + DVec3::Y * f64::from(self.eye_height)
    }

    pub fn can_attack(&self, other: &Living) -> bool {
        !self.peaceful && other.alive && !other.spectator && other.attackable
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
pub struct CreakingBrain {
    pub memories: Memories,
    pub activity: Activity,
    pub started: bool,
    pub sensors: [i64; 2],
    look_until: Option<i64>,
    swim_until: Option<i64>,
    pub walk: MoveSink,
    idle_wait: Option<i64>,
    idle_look: Ticker,
}

impl CreakingBrain {
    pub fn tick(&mut self, ctx: &mut Ctx) {
        if !self.started {
            self.started = true;
            for clock in &mut self.sensors {
                *clock = i64::from(ctx.random.next_int(SCAN_RATE as u32));
            }
        }
        self.memories.tick();
        self.sense(ctx);
        self.swim(ctx);
        self.start_look(ctx);
        self.walk.try_start(
            &mut self.memories.walk_target,
            &mut self.memories.cant_reach_walk_target_since,
            &mut ctx.walker(),
        );
        match self.activity {
            Activity::Idle => self.idle(ctx),
            Activity::Fight => self.fight(ctx),
        }
        self.tick_look(ctx);
        self.walk.tick(
            &mut self.memories.walk_target,
            &mut self.memories.cant_reach_walk_target_since,
            &mut ctx.walker(),
        );
        if self.idle_wait.is_some_and(|end| ctx.time > end) {
            self.idle_wait = None;
        }
    }

    pub fn update_activity(&mut self, can_move: bool) {
        let after = if can_move && self.memories.attack_target.present() {
            Activity::Fight
        } else {
            Activity::Idle
        };
        if after != self.activity {
            self.idle_wait = None;
            self.activity = after;
        }
    }

    pub fn sense(&mut self, ctx: &mut Ctx) {
        let mut fired = [false; 2];
        for (clock, fired) in self.sensors.iter_mut().zip(&mut fired) {
            *clock -= 1;
            if *clock <= 0 {
                *clock = i64::from(SCAN_RATE);
                *fired = true;
            }
        }
        let position = ctx.position();
        let eye = ctx.eye();
        if fired[0] {
            let half = f64::from(ctx.body.width) / 2.0;
            let min = position - DVec3::new(half + SENSE_RANGE, SENSE_RANGE, half + SENSE_RANGE);
            let max = position
                + DVec3::new(
                    half + SENSE_RANGE,
                    f64::from(ctx.body.height) + SENSE_RANGE,
                    half + SENSE_RANGE,
                );
            let mut found: Vec<(&Living, f64)> = ctx
                .others
                .iter()
                .filter(|other| other.alive)
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
            found.sort_by(|a, b| a.1.total_cmp(&b.1));
            let visible = found
                .iter()
                .filter(|(other, _)| {
                    targetable_within(ctx.world, eye, position, other, false, SENSE_RANGE)
                })
                .map(|(other, _)| other.target)
                .collect();
            self.memories.visible_living.set(visible);
        }
        if fired[1] {
            let mut players: Vec<(&Living, f64)> = ctx
                .others
                .iter()
                .filter(|other| other.is_player() && !other.spectator)
                .map(|other| (other, other.position.distance_squared(position)))
                .filter(|(_, distance)| *distance < SENSE_RANGE * SENSE_RANGE)
                .collect();
            players.sort_by(|a, b| a.1.total_cmp(&b.1));
            set_list(
                &mut self.memories.nearest_players,
                players.iter().map(|(other, _)| other.target).collect(),
            );
            let attackable: Vec<Target> = players
                .iter()
                .filter(|(other, _)| {
                    ctx.can_attack(other)
                        && targetable_within(ctx.world, eye, position, other, true, SENSE_RANGE)
                })
                .map(|(other, _)| other.target)
                .collect();
            self.memories
                .nearest_visible_attackable_player
                .set_or_erase(attackable.first().copied());
            set_list(&mut self.memories.visible_attackable_players, attackable);
        }
    }

    fn swim(&mut self, ctx: &mut Ctx) {
        let fluid = ctx.fluid;
        let swimmable = ctx.can_move
            && (fluid.in_water() && fluid.water_height > FLUID_JUMP_THRESHOLD || fluid.in_lava());
        match self.swim_until {
            None if swimmable => self.swim_until = Some(ctx.time + SWIM_TICKS),
            None => return,
            Some(until) if ctx.time > until || !swimmable => {
                self.swim_until = None;
                return;
            }
            Some(_) => {}
        }
        if ctx.random.next_float() < SWIM_CHANCE {
            ctx.jump = true;
        }
    }

    fn sees(&self, track: Track, ctx: &Ctx) -> bool {
        match track {
            Track::Entity { target, .. } => {
                find(ctx.others, target).is_some_and(|other| other.alive)
                    && self.memories.sees(target)
            }
            Track::Block(_) => true,
        }
    }

    fn start_look(&mut self, ctx: &mut Ctx) {
        if self.look_until.is_some() {
            return;
        }
        if self
            .memories
            .look_target
            .get()
            .is_some_and(|&track| self.sees(track, ctx))
        {
            let span = (LOOK_MAX - LOOK_MIN + 1) as u32;
            self.look_until = Some(ctx.time + LOOK_MIN + i64::from(ctx.random.next_int(span)));
        }
    }

    fn tick_look(&mut self, ctx: &mut Ctx) {
        let Some(until) = self.look_until else {
            return;
        };
        let track = self
            .memories
            .look_target
            .get()
            .copied()
            .filter(|&track| self.sees(track, ctx));
        match track.and_then(|track| track.position(ctx.others)) {
            Some(at) if ctx.time <= until => ctx.look_at = Some(at),
            _ => {
                self.look_until = None;
                self.memories.look_target.erase();
            }
        }
    }

    fn visible<'a>(&self, ctx: &Ctx<'a>) -> Vec<&'a Living> {
        let others = ctx.others;
        self.memories
            .visible_living
            .get()
            .into_iter()
            .flatten()
            .filter_map(|&target| find(others, target))
            .collect()
    }

    fn idle(&mut self, ctx: &mut Ctx) {
        self.start_attacking(ctx);
        self.look_sometimes(ctx);
        self.idle_movement(ctx);
    }

    fn start_attacking(&mut self, ctx: &mut Ctx) {
        if !ctx.active || self.memories.attack_target.present() {
            return;
        }
        let Some(target) = self
            .memories
            .nearest_visible_attackable_player
            .get()
            .copied()
        else {
            return;
        };
        if find(ctx.others, target).is_some_and(|other| ctx.can_attack(other)) {
            self.memories.attack_target.set(target);
            self.memories.cant_reach_walk_target_since.erase();
        }
    }

    fn look_sometimes(&mut self, ctx: &mut Ctx) {
        if !self.memories.visible_living.present() {
            return;
        }
        let position = ctx.position();
        let Some(target) = self
            .visible(ctx)
            .into_iter()
            .find(|other| {
                other.position.distance_squared(position) <= LOOK_DISTANCE * LOOK_DISTANCE
            })
            .map(|other| other.target)
        else {
            return;
        };
        if self.idle_look.tick_down_and_check(ctx.random, 30, 60) {
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
        }
    }

    fn idle_movement(&mut self, ctx: &mut Ctx) {
        if self.idle_wait.is_some() {
            return;
        }
        for choice in shuffled(ctx.random, &[2, 2, 1]) {
            match choice {
                0 => {
                    if self.memories.walk_target.present() {
                        continue;
                    }
                    let Some(spot) = crate::stroll::land_random_position(
                        ctx.world,
                        ctx.profile,
                        ctx.position(),
                        10,
                        7,
                        ctx.random,
                    ) else {
                        continue;
                    };
                    self.memories.walk_target.set(WalkGoal {
                        target: Track::Block(block_of(spot)),
                        speed: STROLL_SPEED,
                        close_enough: 0,
                    });
                    return;
                }
                1 => {
                    if self.memories.walk_target.present() {
                        continue;
                    }
                    let Some(track) = self.memories.look_target.get().copied() else {
                        continue;
                    };
                    self.memories.walk_target.set(WalkGoal {
                        target: track,
                        speed: STROLL_SPEED,
                        close_enough: 3,
                    });
                    return;
                }
                _ => {
                    let span = (IDLE_MAX - IDLE_MIN + 1) as u32;
                    self.idle_wait =
                        Some(ctx.time + IDLE_MIN + i64::from(ctx.random.next_int(span)));
                    return;
                }
            }
        }
    }

    fn fight(&mut self, ctx: &mut Ctx) {
        let Some(target) = self.memories.attack_target.get().copied() else {
            return;
        };
        let living = find(ctx.others, target).copied();
        let close = living.is_some_and(|other| {
            self.memories.sees(target) && within_melee_range(ctx.body, &other)
        });
        if close {
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
        if ctx.can_move
            && close
            && !self.memories.attack_cooling_down.present()
            && self.memories.visible_living.present()
        {
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
            ctx.attack = Some(target);
            self.memories
                .attack_cooling_down
                .set_for((), ATTACK_COOLDOWN);
        }
        let valid = living.is_some_and(|other| ctx.can_attack(&other));
        let tired = self
            .memories
            .cant_reach_walk_target_since
            .get()
            .is_some_and(|&since| ctx.time - since > GIVE_UP_REACHING);
        if !valid || tired || !self.memories.reachable(target) {
            self.memories.attack_target.erase();
        }
    }
}
