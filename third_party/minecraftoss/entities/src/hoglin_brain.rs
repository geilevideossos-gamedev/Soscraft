use crate::fluid::FluidFrame;
use crate::monster_ai::Target;
use crate::movement::Body;
use crate::navigation::{GroundNavigation, PlannedPath, plan_walk_path};
use crate::villager_brain::Slot;
use crate::walk_path::WalkProfile;
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

pub type Pos = (i32, i32, i32);

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Living {
    pub target: Target,
    pub kind: &'static str,
    pub position: DVec3,
    pub eye_height: f32,
    pub width: f32,
    pub height: f32,
    pub alive: bool,
    pub baby: bool,
    pub attackable: bool,
    pub spectator: bool,
}

impl Living {
    pub fn eye(&self) -> DVec3 {
        self.position + DVec3::Y * f64::from(self.eye_height)
    }

    pub fn is_player(&self) -> bool {
        matches!(self.target, Target::Player(_))
    }
}

pub fn block_of(position: DVec3) -> Pos {
    (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

pub fn find(others: &[Living], target: Target) -> Option<&Living> {
    others.iter().find(|other| other.target == target)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activity {
    #[default]
    Idle,
    Fight,
    Avoid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Track {
    Entity { target: Target, eyes: bool },
    Block(Pos),
}

impl Track {
    pub fn position(self, others: &[Living]) -> Option<DVec3> {
        match self {
            Self::Entity { target, eyes } => {
                find(others, target).map(|other| if eyes { other.eye() } else { other.position })
            }
            Self::Block((x, y, z)) => Some(DVec3::new(
                f64::from(x) + 0.5,
                f64::from(y) + 0.5,
                f64::from(z) + 0.5,
            )),
        }
    }

    pub fn block(self, others: &[Living]) -> Option<Pos> {
        match self {
            Self::Entity { target, .. } => {
                find(others, target).map(|other| block_of(other.position))
            }
            Self::Block(pos) => Some(pos),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WalkGoal {
    pub target: Track,
    pub speed: f32,
    pub close_enough: i32,
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

pub fn targetable(
    world: &dyn World,
    eye: DVec3,
    position: DVec3,
    other: &Living,
    combat: bool,
) -> bool {
    if !other.alive || other.spectator || (combat && !other.attackable) {
        return false;
    }
    if position.distance_squared(other.position) > FOLLOW_RANGE * FOLLOW_RANGE {
        return false;
    }
    crate::sight::line_of_sight(world, eye, other.eye())
}

pub fn within_melee_range(body: &Body, target: &Living) -> bool {
    let reach = f64::from(2.04_f32).sqrt() - f64::from(0.6_f32);
    let half = f64::from(body.width / 2.0);
    let p = body.position;
    let t = target.position;
    let target_half = f64::from(target.width / 2.0);
    p.x - half - reach < t.x + target_half
        && p.x + half + reach > t.x - target_half
        && p.y < t.y + f64::from(target.height)
        && p.y + f64::from(body.height) > t.y
        && p.z - half - reach < t.z + target_half
        && p.z + half + reach > t.z - target_half
}

pub fn nearer(
    position: DVec3,
    others: &[Living],
    current: Option<Target>,
    other: Target,
) -> Target {
    let Some(current) = current else {
        return other;
    };
    let distance =
        |target| find(others, target).map_or(f64::MAX, |l| l.position.distance_squared(position));
    if distance(current) < distance(other) {
        current
    } else {
        other
    }
}

pub fn shuffled(random: &mut LegacyRandom, weights: &[u32]) -> Vec<usize> {
    let mut scored: Vec<(f32, usize)> = weights
        .iter()
        .enumerate()
        .map(|(index, &weight)| (-random.next_float().powf(1.0 / weight as f32), index))
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().map(|(_, index)| index).collect()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Ticker {
    until_next: i32,
}

impl Ticker {
    pub fn tick_down_and_check(&mut self, random: &mut LegacyRandom, min: i32, max: i32) -> bool {
        if self.until_next == 0 {
            self.until_next = min + random.next_int((max - min + 1) as u32) as i32 - 1;
            false
        } else {
            self.until_next -= 1;
            self.until_next == 0
        }
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
}

#[derive(Clone, Debug, Default)]
pub struct MoveSink {
    until: Option<i64>,
    cooldown: i32,
    last_target: Option<Pos>,
    path: Option<PlannedPath>,
    speed: f32,
}

impl MoveSink {
    pub fn running(&self) -> bool {
        self.until.is_some()
    }

    fn reached(goal: &WalkGoal, ctx: &Ctx) -> bool {
        let at = block_of(ctx.position());
        goal.target.block(ctx.others).is_none_or(|(x, y, z)| {
            (x - at.0).abs() + (y - at.1).abs() + (z - at.2).abs() <= goal.close_enough
        })
    }

    fn compute(&mut self, mem: &mut Memories, ctx: &mut Ctx, goal: &WalkGoal) -> bool {
        let Some(pos) = goal.target.block(ctx.others) else {
            return false;
        };
        let at = DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
        self.path = plan_walk_path(
            ctx.body,
            ctx.navigation,
            ctx.world,
            ctx.profile,
            ctx.fluid,
            at,
            0,
        );
        self.speed = goal.speed;
        if Self::reached(goal, ctx) {
            mem.cant_reach_walk_target_since.erase();
            return false;
        }
        if self.path.as_ref().is_some_and(PlannedPath::reached) {
            mem.cant_reach_walk_target_since.erase();
        } else if !mem.cant_reach_walk_target_since.present() {
            mem.cant_reach_walk_target_since.set(ctx.time);
        }
        if self.path.is_some() {
            return true;
        }
        let towards = DVec3::new(
            f64::from(pos.0) + 0.5,
            f64::from(pos.1),
            f64::from(pos.2) + 0.5,
        );
        let Some(spot) = crate::stroll::default_random_position_towards(
            ctx.world,
            ctx.profile,
            ctx.position(),
            10,
            7,
            towards,
            std::f32::consts::FRAC_PI_2,
            ctx.random,
        ) else {
            return false;
        };
        self.path = plan_walk_path(
            ctx.body,
            ctx.navigation,
            ctx.world,
            ctx.profile,
            ctx.fluid,
            spot,
            0,
        );
        self.path.is_some()
    }

    fn try_start(&mut self, mem: &mut Memories, ctx: &mut Ctx) {
        let Some(goal) = mem.walk_target.get().copied() else {
            return;
        };
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return;
        }
        let reached = Self::reached(&goal, ctx);
        if !reached && self.compute(mem, ctx, &goal) {
            self.last_target = goal.target.block(ctx.others);
            self.until = Some(ctx.time + 150 + i64::from(ctx.random.next_int(101)));
            let path = self.path.clone();
            ctx.navigation
                .move_to_in(ctx.world, path, f64::from(self.speed), ctx.position());
            return;
        }
        mem.walk_target.erase();
        if reached {
            mem.cant_reach_walk_target_since.erase();
        }
    }

    fn tick(&mut self, mem: &mut Memories, ctx: &mut Ctx) {
        let Some(until) = self.until else {
            return;
        };
        let goal = mem.walk_target.get().copied();
        let usable = self.path.is_some()
            && self.last_target.is_some()
            && !ctx.navigation.is_done()
            && goal.is_some_and(|goal| !Self::reached(&goal, ctx));
        if ctx.time > until || !usable {
            if goal.is_some_and(|goal| !Self::reached(&goal, ctx)) && ctx.navigation.is_stuck() {
                self.cooldown = ctx.random.next_int(40) as i32;
            }
            ctx.navigation.stop();
            mem.walk_target.erase();
            self.path = None;
            self.until = None;
            return;
        }
        let Some(goal) = goal else {
            return;
        };
        let Some(now) = goal.target.block(ctx.others) else {
            return;
        };
        let moved = self.last_target.is_some_and(|(x, y, z)| {
            let (dx, dy, dz) = (now.0 - x, now.1 - y, now.2 - z);
            dx * dx + dy * dy + dz * dz > 4
        });
        if moved && self.compute(mem, ctx, &goal) {
            self.last_target = Some(now);
            let path = self.path.clone();
            ctx.navigation
                .move_to_in(ctx.world, path, f64::from(self.speed), ctx.position());
        }
    }

    pub fn stop(&mut self, navigation: &mut GroundNavigation) {
        if self.until.take().is_some() {
            navigation.stop();
        }
        self.path = None;
    }
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
        self.walk.try_start(&mut self.memories, ctx);
        match self.activity {
            Activity::Idle => self.start_idle(ctx),
            Activity::Fight => self.start_fight(ctx),
            Activity::Avoid => self.start_avoid(ctx),
        }
        self.tick_look(ctx);
        self.walk.tick(&mut self.memories, ctx);
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
        if fired[2] {
            let kind = ctx.kind;
            let adult = self
                .visible(ctx)
                .into_iter()
                .find(|other| other.kind == kind && !other.baby);
            self.memories
                .nearest_visible_adult
                .set_or_erase(adult.map(|other| other.target));
        }
        if fired[3] {
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
                    profile.walk_target = crate::walk_path::WalkTarget::Hoglin { repellent };
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
        if !rerun && self.memories.walk_target.present() {
            return;
        }
        let position = ctx.position();
        if position.distance_squared(from) >= distance * distance {
            return;
        }
        if let Some(goal) = self.memories.walk_target.get() {
            if goal.speed == speed {
                if let Some(to) = goal.target.position(ctx.others) {
                    if (to - position).dot(from - position) < 0.0 {
                        return;
                    }
                }
            }
        }
        for _ in 0..10 {
            if let Some(spot) = crate::stroll::land_random_position_away(
                ctx.world,
                ctx.profile,
                position,
                16,
                7,
                from,
                ctx.random,
            ) {
                self.memories.walk_target.set(WalkGoal {
                    target: Track::Block(block_of(spot)),
                    speed,
                    close_enough: 0,
                });
                return;
            }
        }
    }

    fn start_idle(&mut self, ctx: &mut Ctx) {
        self.start_attacking(ctx);
        self.look_sometimes(ctx, false);
        self.idle_movement(ctx, false);
    }

    fn start_fight(&mut self, ctx: &mut Ctx) {
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

pub fn closer_than(a: Pos, b: Pos, distance: f64) -> bool {
    let (dx, dy, dz) = (
        f64::from(a.0 - b.0),
        f64::from(a.1 - b.1),
        f64::from(a.2 - b.2),
    );
    dx * dx + dy * dy + dz * dz < distance * distance
}

fn set_list<T>(slot: &mut Slot<Vec<T>>, list: Vec<T>) {
    if list.is_empty() {
        slot.erase();
    } else {
        slot.set(list);
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
    fn a_heavier_choice_comes_first_more_often() {
        let mut random = LegacyRandom::new(8);
        let firsts = (0..3000)
            .filter(|_| shuffled(&mut random, &[2, 2, 1])[0] == 2)
            .count();
        assert!((300..700).contains(&firsts), "{firsts}");
    }
}
