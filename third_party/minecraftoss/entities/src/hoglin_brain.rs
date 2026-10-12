pub use crate::brain_behaviors::Living;
use crate::brain_behaviors::set_list;
pub use crate::brain_behaviors::{
    MoveSink, Pos, REPELLENT_HORIZONTAL, REPELLENT_VERTICAL, Ticker, Track, WalkGoal, Walker,
    block_of, closer_than, find, nearer, nearest_block_in_tag, shuffled, within_manhattan,
    within_melee_range,
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
pub const FOLLOW_RANGE: f64 = 16.0;
pub const RETALIATION_TICKS: i64 = 200;
pub const PACIFIED_TICKS: i64 = 200;
pub const ADULT_ATTACK_COOLDOWN: i64 = 40;
pub const BABY_ATTACK_COOLDOWN: i64 = 15;
pub const GIVE_UP_REACHING: i64 = 200;
pub const RETREAT_MIN: i64 = 100;
pub const RETREAT_MAX: i64 = 400;
pub const LOOK_DISTANCE: f64 = 8.0;
pub const STROLL_SPEED: f32 = 0.4;
pub const CHASE_SPEED: f32 = 1.0;
pub const FLEE_SPEED: f32 = 1.3;
pub const FLEE_DISTANCE: f64 = 15.0;
pub const MUCH_FURTHER: f64 = 4.0;
pub const REPELLENTS: &str = "minecraft:hoglin_repellents";
pub const REPELLENT_SPEED: f32 = 1.0;
pub const REPELLENT_DISTANCE: f64 = 8.0;
pub const PIGLIN_SPEED: f32 = 0.4;
pub const PIGLIN_DISTANCE: f64 = 8.0;
pub const LOVE_SPEED: f32 = 0.6;
pub const LOVE_CLOSE_ENOUGH: i32 = 2;
pub const LOVE_TICKS: i64 = 110;
pub const FOLLOW_SPEED: f32 = 0.6;
pub const FOLLOW_MIN: f64 = 5.0;
pub const FOLLOW_MAX: f64 = 16.0;
pub const ZOGLIN: &str = "minecraft:zoglin";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Idle,
    Fight,
    Avoid,
}

#[derive(Clone, Debug, Default)]
pub struct Memories {
    pub nearest_living: Slot<Vec<Target>>,
    pub visible_living: Slot<Vec<Target>>,
    pub nearest_visible_attackable_player: Slot<Target>,
    pub nearest_visible_adult: Slot<Target>,
    pub nearest_repellent: Slot<Pos>,
    pub nearest_visible_adult_piglin: Slot<Target>,
    pub nearest_visible_adult_hoglins: Slot<Vec<Target>>,
    pub visible_adult_piglin_count: Slot<usize>,
    pub visible_adult_hoglin_count: Slot<usize>,
    pub attack_target: Slot<Target>,
    pub avoid_target: Slot<Target>,
    pub pacified: Slot<()>,
    pub breed_target: Slot<Target>,
    pub walk_target: Slot<WalkGoal>,
    pub look_target: Slot<Track>,
    pub cant_reach_walk_target_since: Slot<i64>,
    pub attack_cooling_down: Slot<()>,
}

impl Memories {
    pub fn tick(&mut self) {
        self.nearest_living.tick();
        self.visible_living.tick();
        self.nearest_visible_attackable_player.tick();
        self.nearest_visible_adult.tick();
        self.nearest_repellent.tick();
        self.nearest_visible_adult_piglin.tick();
        self.nearest_visible_adult_hoglins.tick();
        self.visible_adult_piglin_count.tick();
        self.visible_adult_hoglin_count.tick();
        self.attack_target.tick();
        self.avoid_target.tick();
        self.pacified.tick();
        self.breed_target.tick();
        self.walk_target.tick();
        self.look_target.tick();
        self.cant_reach_walk_target_since.tick();
        self.attack_cooling_down.tick();
    }

    pub fn sees(&self, target: Target) -> bool {
        self.visible_living
            .get()
            .is_some_and(|visible| visible.contains(&target))
    }

    pub fn visible_adult_hoglins(&self) -> Vec<Target> {
        self.nearest_visible_adult_hoglins
            .get()
            .cloned()
            .unwrap_or_default()
    }

    pub fn piglins_outnumber_hoglins(&self, baby: bool) -> bool {
        if baby {
            return false;
        }
        let piglins = self.visible_adult_piglin_count.get().copied().unwrap_or(0);
        let hoglins = self.visible_adult_hoglin_count.get().copied().unwrap_or(0);
        piglins > hoglins + 1
    }
}

pub struct Ctx<'a> {
    pub id: u64,
    pub kind: &'static str,
    pub body: &'a Body,
    pub eye_height: f32,
    pub baby: bool,
    pub others: &'a [Living],
    pub world: &'a dyn World,
    pub time: i64,
    pub random: &'a mut LegacyRandom,
    pub navigation: &'a mut GroundNavigation,
    pub profile: &'a WalkProfile,
    pub fluid: FluidFrame,
    pub look_at: Option<DVec3>,
    pub attack: Option<Target>,
    pub sounds: Vec<&'static str>,
    pub in_love: bool,
    pub lovers: &'a [Target],
    pub courting: Option<Target>,
    pub birth: Option<Target>,
}

impl Ctx<'_> {
    pub fn position(&self) -> DVec3 {
        self.body.position
    }

    pub fn eye(&self) -> DVec3 {
        self.body.position + DVec3::Y * f64::from(self.eye_height)
    }

    pub fn attackable(&self, other: &Living) -> bool {
        targetable(self.world, self.eye(), self.position(), other, true)
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

pub fn targetable(
    world: &dyn World,
    eye: DVec3,
    position: DVec3,
    other: &Living,
    combat: bool,
) -> bool {
    crate::brain_behaviors::targetable_within(world, eye, position, other, combat, FOLLOW_RANGE)
}

#[derive(Clone, Copy, Debug)]
struct Love {
    partner: Target,
    child_at: i64,
    until: i64,
}

#[derive(Clone, Debug, Default)]
pub struct HoglinBrain {
    pub memories: Memories,
    pub activity: Activity,
    started: bool,
    sensors: [i64; 4],
    look_until: Option<i64>,
    pub walk: MoveSink,
    idle_wait: Option<i64>,
    avoid_wait: Option<i64>,
    idle_look: Ticker,
    avoid_look: Ticker,
    love: Option<Love>,
}

impl HoglinBrain {
    pub fn tick(&mut self, ctx: &mut Ctx) {
        if !self.started {
            self.started = true;
            for clock in &mut self.sensors {
                *clock = i64::from(ctx.random.next_int(SCAN_RATE as u32));
            }
        }
        self.memories.tick();
        self.sense(ctx);
        self.start_look(ctx);
        self.walk.try_start(
            &mut self.memories.walk_target,
            &mut self.memories.cant_reach_walk_target_since,
            &mut ctx.walker(),
        );
        match (self.activity, ctx.kind == ZOGLIN) {
            (Activity::Fight, true) => self.fight(ctx),
            (_, true) => self.start_zoglin_idle(ctx),
            (Activity::Idle, false) => self.start_idle(ctx),
            (Activity::Fight, false) => self.start_fight(ctx),
            (Activity::Avoid, false) => self.start_avoid(ctx),
        }
        self.tick_look(ctx);
        self.walk.tick(
            &mut self.memories.walk_target,
            &mut self.memories.cant_reach_walk_target_since,
            &mut ctx.walker(),
        );
        self.tick_love(ctx);
        for wait in [&mut self.idle_wait, &mut self.avoid_wait] {
            if wait.is_some_and(|end| ctx.time > end) {
                *wait = None;
            }
        }
    }

    fn sense(&mut self, ctx: &mut Ctx) {
        let fired: Vec<bool> = self
            .sensors
            .iter_mut()
            .map(|clock| {
                *clock -= 1;
                if *clock > 0 {
                    return false;
                }
                *clock = i64::from(SCAN_RATE);
                true
            })
            .collect();
        let position = ctx.position();
        let eye = ctx.eye();
        if fired[0] {
            let half = f64::from(ctx.body.width) / 2.0;
            let (min, max) = (
                position - DVec3::new(half + FOLLOW_RANGE, FOLLOW_RANGE, half + FOLLOW_RANGE),
                position
                    + DVec3::new(
                        half + FOLLOW_RANGE,
                        f64::from(ctx.body.height) + FOLLOW_RANGE,
                        half + FOLLOW_RANGE,
                    ),
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
                .filter(|(other, _)| targetable(ctx.world, eye, position, other, false))
                .map(|(other, _)| other.target)
                .collect();
            set_list(
                &mut self.memories.nearest_living,
                found.iter().map(|(other, _)| other.target).collect(),
            );
            self.memories.visible_living.set(visible);
        }
        if fired[1] {
            let mut players: Vec<(&Living, f64)> = ctx
                .others
                .iter()
                .filter(|other| other.is_player() && !other.spectator)
                .map(|other| (other, other.position.distance_squared(position)))
                .filter(|(_, distance)| *distance < FOLLOW_RANGE * FOLLOW_RANGE)
                .collect();
            players.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let attackable = players
                .iter()
                .find(|(other, _)| targetable(ctx.world, eye, position, other, true))
                .map(|(other, _)| other.target);
            self.memories
                .nearest_visible_attackable_player
                .set_or_erase(attackable);
        }
        if fired[2] && ctx.kind != ZOGLIN {
            let kind = ctx.kind;
            let adult = self
                .visible(ctx)
                .into_iter()
                .find(|other| other.kind == kind && !other.baby);
            self.memories
                .nearest_visible_adult
                .set_or_erase(adult.map(|other| other.target));
        }
        if fired[3] && ctx.kind != ZOGLIN {
            let repellent = nearest_block_in_tag(ctx.world, block_of(position), REPELLENTS);
            self.memories.nearest_repellent.set_or_erase(repellent);
            let mut piglin = None;
            let mut piglins = 0;
            let mut hoglins = Vec::new();
            for other in self.visible(ctx).into_iter().filter(|other| !other.baby) {
                if other.kind == "minecraft:piglin" {
                    piglins += 1;
                    piglin = piglin.or(Some(other.target));
                } else if other.kind == "minecraft:hoglin" {
                    hoglins.push(other.target);
                }
            }
            self.memories
                .nearest_visible_adult_piglin
                .set_or_erase(piglin);
            self.memories.visible_adult_piglin_count.set(piglins);
            self.memories.visible_adult_hoglin_count.set(hoglins.len());
            set_list(&mut self.memories.nearest_visible_adult_hoglins, hoglins);
        }
    }

    fn visible<'a>(&self, ctx: &Ctx<'a>) -> Vec<&'a Living> {
        let others = ctx.others;
        self.memories
            .visible_living
            .get()
            .map(|visible| {
                visible
                    .iter()
                    .filter_map(|&target| find(others, target))
                    .collect()
            })
            .unwrap_or_default()
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
            self.look_until = Some(ctx.time + 45 + i64::from(ctx.random.next_int(46)));
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

    fn start_zoglin_idle(&mut self, ctx: &mut Ctx) {
        if !self.memories.attack_target.present() {
            let target = self
                .visible(ctx)
                .into_iter()
                .find(|other| {
                    other.kind != ZOGLIN
                        && other.kind != "minecraft:creeper"
                        && other.alive
                        && other.attackable
                        && !other.spectator
                })
                .map(|other| other.target);
            if let Some(target) = target {
                self.memories.attack_target.set(target);
                self.memories.cant_reach_walk_target_since.erase();
            }
        }
        self.look_sometimes(ctx, false);
        self.idle_movement(ctx, false);
    }

    pub fn update_zoglin_activity(&mut self, ctx: &mut Ctx) {
        let after = if self.memories.attack_target.present() {
            Activity::Fight
        } else {
            Activity::Idle
        };
        if self.activity != after {
            if after == Activity::Fight {
                ctx.sounds.push("entity.zoglin.angry");
            } else {
                self.memories.attack_target.erase();
            }
            self.activity = after;
        }
    }

    fn start_attacking(&mut self, ctx: &mut Ctx) {
        if self.memories.attack_target.present() {
            return;
        }
        if self.memories.pacified.present() || self.memories.breed_target.present() {
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
        if find(ctx.others, target)
            .is_some_and(|other| other.alive && other.attackable && !other.spectator)
        {
            self.memories.attack_target.set(target);
            self.memories.cant_reach_walk_target_since.erase();
        }
    }

    fn look_sometimes(&mut self, ctx: &mut Ctx, avoid: bool) {
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
        let ticker = if avoid {
            &mut self.avoid_look
        } else {
            &mut self.idle_look
        };
        if ticker.tick_down_and_check(ctx.random, 30, 60) {
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
        }
    }

    fn idle_movement(&mut self, ctx: &mut Ctx, avoid: bool) {
        let wait = if avoid {
            self.avoid_wait
        } else {
            self.idle_wait
        };
        if wait.is_some() {
            return;
        }
        for choice in shuffled(ctx.random, &[2, 2, 1]) {
            match choice {
                0 => {
                    if self.memories.walk_target.present() {
                        continue;
                    }
                    let repellent = self.memories.nearest_repellent.get().copied();
                    let mut profile = ctx.profile.clone();
                    if ctx.kind != ZOGLIN {
                        profile.walk_target = crate::walk_path::WalkTarget::Hoglin { repellent };
                    }
                    let Some(spot) = crate::stroll::land_random_position(
                        ctx.world,
                        &profile,
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
                    let end = ctx.time + 30 + i64::from(ctx.random.next_int(31));
                    if avoid {
                        self.avoid_wait = Some(end);
                    } else {
                        self.idle_wait = Some(end);
                    }
                    return;
                }
            }
        }
    }

    fn walk_away(&mut self, ctx: &mut Ctx, from: DVec3, speed: f32, distance: f64, rerun: bool) {
        crate::brain_behaviors::walk_away(
            &mut self.memories.walk_target,
            &mut ctx.walker(),
            from,
            speed,
            distance,
            rerun,
        );
    }

    fn become_passive(&mut self) {
        if self.memories.pacified.present() || !self.memories.nearest_repellent.present() {
            return;
        }
        self.memories.pacified.set_for((), PACIFIED_TICKS);
        self.memories.attack_target.erase();
    }

    fn start_idle(&mut self, ctx: &mut Ctx) {
        self.become_passive();
        self.start_love(ctx);
        if let Some(&(x, y, z)) = self.memories.nearest_repellent.get() {
            let from = DVec3::new(f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5);
            self.walk_away(ctx, from, REPELLENT_SPEED, REPELLENT_DISTANCE, true);
        }
        self.start_attacking(ctx);
        if !ctx.baby {
            if let Some(from) = self
                .memories
                .nearest_visible_adult_piglin
                .get()
                .and_then(|&target| find(ctx.others, target))
                .map(|other| other.position)
            {
                self.walk_away(ctx, from, PIGLIN_SPEED, PIGLIN_DISTANCE, false);
            }
        }
        self.look_sometimes(ctx, false);
        self.follow_adult(ctx);
        self.idle_movement(ctx, false);
    }

    fn start_fight(&mut self, ctx: &mut Ctx) {
        self.become_passive();
        self.start_love(ctx);
        self.fight(ctx);
        if self.memories.breed_target.present() {
            self.memories.attack_target.erase();
        }
    }

    fn lock_gaze(&mut self, partner: Target) {
        self.memories.look_target.set(Track::Entity {
            target: partner,
            eyes: true,
        });
        self.memories.walk_target.set(WalkGoal {
            target: Track::Entity {
                target: partner,
                eyes: false,
            },
            speed: LOVE_SPEED,
            close_enough: LOVE_CLOSE_ENOUGH,
        });
    }

    fn start_love(&mut self, ctx: &mut Ctx) {
        if self.love.is_some()
            || !ctx.in_love
            || self.memories.breed_target.present()
            || !self.memories.visible_living.present()
        {
            return;
        }
        let (kind, lovers) = (ctx.kind, ctx.lovers);
        let Some(partner) = self
            .visible(ctx)
            .into_iter()
            .find(|other| other.kind == kind && lovers.contains(&other.target))
            .map(|other| other.target)
        else {
            return;
        };
        let until = ctx.time + LOVE_TICKS + i64::from(ctx.random.next_int(1));
        self.memories.breed_target.set(partner);
        self.lock_gaze(partner);
        ctx.courting = Some(partner);
        let child_at = ctx.time + 60 + i64::from(ctx.random.next_int(50));
        self.love = Some(Love {
            partner,
            child_at,
            until,
        });
    }

    fn tick_love(&mut self, ctx: &mut Ctx) {
        let Some(love) = self.love else {
            return;
        };
        let partner = find(ctx.others, love.partner).copied();
        let usable = self.memories.breed_target.get() == Some(&love.partner)
            && partner.is_some_and(|other| other.alive)
            && ctx.in_love
            && ctx.lovers.contains(&love.partner)
            && self.memories.sees(love.partner)
            && ctx.time <= love.child_at;
        let Some(partner) = partner.filter(|_| usable && ctx.time <= love.until) else {
            self.memories.breed_target.erase();
            self.memories.walk_target.erase();
            self.memories.look_target.erase();
            self.love = None;
            return;
        };
        self.lock_gaze(love.partner);
        ctx.courting = Some(love.partner);
        let close = partner.position.distance_squared(ctx.position()) < 9.0;
        if close && ctx.time >= love.child_at {
            ctx.birth = Some(love.partner);
            self.memories.breed_target.erase();
        }
    }

    fn follow_adult(&mut self, ctx: &mut Ctx) {
        if !ctx.baby || self.memories.walk_target.present() {
            return;
        }
        let Some(adult) = self.memories.nearest_visible_adult.get().copied() else {
            return;
        };
        let Some(living) = find(ctx.others, adult) else {
            return;
        };
        let distance = living.position.distance_squared(ctx.position());
        if distance < (FOLLOW_MAX + 1.0).powi(2) && distance >= FOLLOW_MIN * FOLLOW_MIN {
            self.memories.look_target.set(Track::Entity {
                target: adult,
                eyes: true,
            });
            self.memories.walk_target.set(WalkGoal {
                target: Track::Entity {
                    target: adult,
                    eyes: false,
                },
                speed: FOLLOW_SPEED,
                close_enough: FOLLOW_MIN as i32 - 1,
            });
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
        if close
            && !self.memories.attack_cooling_down.present()
            && self.memories.visible_living.present()
        {
            self.memories
                .look_target
                .set(Track::Entity { target, eyes: true });
            ctx.attack = Some(target);
            let cooldown = if ctx.baby {
                BABY_ATTACK_COOLDOWN
            } else {
                ADULT_ATTACK_COOLDOWN
            };
            self.memories.attack_cooling_down.set_for((), cooldown);
        }
        let valid = living.is_some_and(|other| other.alive && other.attackable && !other.spectator);
        let tired = self
            .memories
            .cant_reach_walk_target_since
            .get()
            .is_some_and(|&since| ctx.time - since > GIVE_UP_REACHING);
        if !valid || tired {
            self.memories.attack_target.erase();
        }
    }

    fn start_avoid(&mut self, ctx: &mut Ctx) {
        if let Some(from) = self
            .memories
            .avoid_target
            .get()
            .and_then(|&target| find(ctx.others, target))
            .map(|other| other.position)
        {
            self.walk_away(ctx, from, FLEE_SPEED, FLEE_DISTANCE, false);
        }
        self.idle_movement(ctx, true);
        self.look_sometimes(ctx, true);
        if !ctx.baby && !self.memories.piglins_outnumber_hoglins(ctx.baby) {
            self.memories.avoid_target.erase();
        }
    }

    pub fn update_activity(&mut self, ctx: &mut Ctx, converting: bool) {
        let before = self.activity;
        let after = if self.memories.attack_target.present() {
            Activity::Fight
        } else if self.memories.avoid_target.present() {
            Activity::Avoid
        } else {
            Activity::Idle
        };
        if before != after {
            match after {
                Activity::Fight => self.memories.avoid_target.erase(),
                Activity::Avoid => self.memories.attack_target.erase(),
                Activity::Idle => {
                    self.memories.attack_target.erase();
                    self.memories.avoid_target.erase();
                }
            }
            self.activity = after;
            ctx.sounds
                .push(self.activity_sound(ctx.position(), converting));
        }
    }

    pub fn activity_sound(&self, position: DVec3, converting: bool) -> &'static str {
        match self.activity {
            Activity::Avoid => "entity.hoglin.retreat",
            _ if converting => "entity.hoglin.retreat",
            Activity::Fight => "entity.hoglin.angry",
            Activity::Idle if self.near_repellent(position) => "entity.hoglin.retreat",
            Activity::Idle => "entity.hoglin.ambient",
        }
    }

    pub fn near_repellent(&self, position: DVec3) -> bool {
        let at = block_of(position);
        self.memories
            .nearest_repellent
            .get()
            .is_some_and(|&repellent| closer_than(repellent, at, 8.0))
    }

    pub fn court(&mut self, partner: Target) {
        self.memories.breed_target.set(partner);
        self.lock_gaze(partner);
    }

    pub fn set_attack_target(&mut self, target: Target) {
        self.memories.cant_reach_walk_target_since.erase();
        self.memories.breed_target.erase();
        self.memories
            .attack_target
            .set_for(target, RETALIATION_TICKS);
    }

    pub fn set_avoid_target(&mut self, target: Target, random: &mut LegacyRandom) {
        self.memories.attack_target.erase();
        self.memories.walk_target.erase();
        let ticks =
            RETREAT_MIN + i64::from(random.next_int((RETREAT_MAX - RETREAT_MIN + 1) as u32));
        self.memories.avoid_target.set_for(target, ticks);
    }

    pub fn retreat_from_nearest(
        &mut self,
        position: DVec3,
        others: &[Living],
        target: Target,
        random: &mut LegacyRandom,
    ) {
        let nearest = nearer(
            position,
            others,
            self.memories.avoid_target.get().copied(),
            target,
        );
        let nearest = nearer(
            position,
            others,
            self.memories.attack_target.get().copied(),
            nearest,
        );
        self.set_avoid_target(nearest, random);
    }

    pub fn much_further(&self, position: DVec3, others: &[Living], other: Target) -> bool {
        let Some(current) = self.memories.attack_target.get().copied() else {
            return false;
        };
        let distance = |target| find(others, target).map(|l| l.position.distance_squared(position));
        match (distance(current), distance(other)) {
            (Some(current), Some(other)) => other > current + MUCH_FURTHER * MUCH_FURTHER,
            _ => false,
        }
    }

    pub fn set_attack_target_if_closer(
        &mut self,
        position: DVec3,
        others: &[Living],
        target: Target,
    ) {
        if self.memories.pacified.present() {
            return;
        }
        let nearest = nearer(
            position,
            others,
            self.memories.attack_target.get().copied(),
            target,
        );
        self.set_attack_target(nearest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_look_ticker_fires_after_its_interval() {
        let mut random = LegacyRandom::new(4);
        let mut ticker = Ticker::default();
        assert!(!ticker.tick_down_and_check(&mut random, 30, 60));
        let mut ticks = 1;
        while !ticker.tick_down_and_check(&mut random, 30, 60) {
            ticks += 1;
        }
        assert!((30..=60).contains(&ticks), "{ticks}");
    }

    #[test]
    fn the_repellent_scan_runs_by_manhattan_distance_within_eight_by_four() {
        let offsets = within_manhattan(8, 4);
        assert_eq!(offsets.len(), 17 * 9 * 17);
        assert_eq!(
            &offsets[..7],
            &[
                (0, 0, 0),
                (-1, 0, 0),
                (0, -1, 0),
                (0, 0, 1),
                (0, 0, -1),
                (0, 1, 0),
                (1, 0, 0)
            ]
        );
        let depth = |&(x, y, z): &Pos| x.abs() + y.abs() + z.abs();
        assert!(offsets.windows(2).all(|w| depth(&w[0]) <= depth(&w[1])));
        assert!(
            offsets
                .iter()
                .all(|&(x, y, z)| x.abs() <= 8 && y.abs() <= 4 && z.abs() <= 8)
        );
        let mut unique = offsets.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), offsets.len());
    }

    #[test]
    fn a_heavier_choice_comes_first_more_often() {
        let mut random = LegacyRandom::new(8);
        let firsts = (0..3000)
            .filter(|_| shuffled(&mut random, &[2, 2, 1])[0] == 2)
            .count();
        assert!((300..700).contains(&firsts), "{firsts}");
    }
}
