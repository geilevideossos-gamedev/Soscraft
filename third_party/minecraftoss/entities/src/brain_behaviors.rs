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

pub const REPELLENT_HORIZONTAL: i32 = 8;
pub const REPELLENT_VERTICAL: i32 = 4;

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

pub fn targetable_within(
    world: &dyn World,
    eye: DVec3,
    position: DVec3,
    other: &Living,
    combat: bool,
    range: f64,
) -> bool {
    if !other.alive || other.spectator || (combat && !other.attackable) {
        return false;
    }
    if position.distance_squared(other.position) > range * range {
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

pub struct Walker<'a> {
    pub body: &'a Body,
    pub others: &'a [Living],
    pub world: &'a dyn World,
    pub time: i64,
    pub random: &'a mut LegacyRandom,
    pub navigation: &'a mut GroundNavigation,
    pub profile: &'a WalkProfile,
    pub fluid: FluidFrame,
}

impl Walker<'_> {
    pub fn position(&self) -> DVec3 {
        self.body.position
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

    fn reached(goal: &WalkGoal, ctx: &Walker) -> bool {
        let at = block_of(ctx.position());
        goal.target.block(ctx.others).is_none_or(|(x, y, z)| {
            (x - at.0).abs() + (y - at.1).abs() + (z - at.2).abs() <= goal.close_enough
        })
    }

    fn compute(&mut self, cant_reach: &mut Slot<i64>, ctx: &mut Walker, goal: &WalkGoal) -> bool {
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
            cant_reach.erase();
            return false;
        }
        if self.path.as_ref().is_some_and(PlannedPath::reached) {
            cant_reach.erase();
        } else if !cant_reach.present() {
            cant_reach.set(ctx.time);
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

    pub fn try_start(
        &mut self,
        walk_target: &mut Slot<WalkGoal>,
        cant_reach: &mut Slot<i64>,
        ctx: &mut Walker,
    ) {
        let Some(goal) = walk_target.get().copied() else {
            return;
        };
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return;
        }
        let reached = Self::reached(&goal, ctx);
        if !reached && self.compute(cant_reach, ctx, &goal) {
            self.last_target = goal.target.block(ctx.others);
            self.until = Some(ctx.time + 150 + i64::from(ctx.random.next_int(101)));
            let path = self.path.clone();
            let position = ctx.position();
            ctx.navigation
                .move_to_in(ctx.world, path, f64::from(self.speed), position);
            return;
        }
        walk_target.erase();
        if reached {
            cant_reach.erase();
        }
    }

    pub fn tick(
        &mut self,
        walk_target: &mut Slot<WalkGoal>,
        cant_reach: &mut Slot<i64>,
        ctx: &mut Walker,
    ) {
        let Some(until) = self.until else {
            return;
        };
        let goal = walk_target.get().copied();
        let usable = self.path.is_some()
            && self.last_target.is_some()
            && !ctx.navigation.is_done()
            && goal.is_some_and(|goal| !Self::reached(&goal, ctx));
        if ctx.time > until || !usable {
            if goal.is_some_and(|goal| !Self::reached(&goal, ctx)) && ctx.navigation.is_stuck() {
                self.cooldown = ctx.random.next_int(40) as i32;
            }
            ctx.navigation.stop();
            walk_target.erase();
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
        if moved && self.compute(cant_reach, ctx, &goal) {
            self.last_target = Some(now);
            let path = self.path.clone();
            let position = ctx.position();
            ctx.navigation
                .move_to_in(ctx.world, path, f64::from(self.speed), position);
        }
    }

    pub fn stop(&mut self, navigation: &mut GroundNavigation) {
        if self.until.take().is_some() {
            navigation.stop();
        }
        self.path = None;
    }
}

pub fn within_manhattan(horizontal: i32, vertical: i32) -> Vec<Pos> {
    let mut out = Vec::new();
    for depth in 0..=horizontal * 2 + vertical {
        let max_x = horizontal.min(depth);
        for x in -max_x..=max_x {
            let max_y = vertical.min(depth - x.abs());
            for y in -max_y..=max_y {
                let z = depth - x.abs() - y.abs();
                if z <= horizontal {
                    out.push((x, y, z));
                    if z != 0 {
                        out.push((x, y, -z));
                    }
                }
            }
        }
    }
    out
}

pub fn nearest_block_in_tag(world: &dyn World, at: Pos, tag: &str) -> Option<Pos> {
    nearest_block(world, at, |world, pos| world.block_in_tag(pos, tag))
}

pub fn nearest_block(
    world: &dyn World,
    at: Pos,
    matches: impl Fn(&dyn World, Pos) -> bool,
) -> Option<Pos> {
    static OFFSETS: std::sync::OnceLock<Vec<Pos>> = std::sync::OnceLock::new();
    OFFSETS
        .get_or_init(|| within_manhattan(REPELLENT_HORIZONTAL, REPELLENT_VERTICAL))
        .iter()
        .map(|&(x, y, z)| (at.0 + x, at.1 + y, at.2 + z))
        .find(|&pos| matches(world, pos))
}

pub fn closer_than(a: Pos, b: Pos, distance: f64) -> bool {
    let (dx, dy, dz) = (
        f64::from(a.0 - b.0),
        f64::from(a.1 - b.1),
        f64::from(a.2 - b.2),
    );
    dx * dx + dy * dy + dz * dz < distance * distance
}

pub fn walk_away(
    walk_target: &mut Slot<WalkGoal>,
    ctx: &mut Walker,
    from: DVec3,
    speed: f32,
    distance: f64,
    rerun: bool,
) {
    if !rerun && walk_target.present() {
        return;
    }
    let position = ctx.position();
    if position.distance_squared(from) >= distance * distance {
        return;
    }
    if let Some(goal) = walk_target.get() {
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
            walk_target.set(WalkGoal {
                target: Track::Block(block_of(spot)),
                speed,
                close_enough: 0,
            });
            return;
        }
    }
}

pub fn set_list<T>(slot: &mut Slot<Vec<T>>, list: Vec<T>) {
    if list.is_empty() {
        slot.erase();
    } else {
        slot.set(list);
    }
}
