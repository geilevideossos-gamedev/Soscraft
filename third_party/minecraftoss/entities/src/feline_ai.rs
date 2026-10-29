use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{
    FloatGoal, LeapGoal, LookAtPlayerGoal, MonsterGoalContext, NearestTargetGoal, Prey, StrollGoal,
    Target, TargetInfo,
};
use crate::movement::Body;
use crate::navigation::{PlannedPath, navigate_walk_to, navigate_walk_to_entity, plan_walk_path};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::path_type::PathType;

pub const CAT_GOAL_NAMES: [&str; 14] = [
    "FloatGoal",
    "TamableAnimalPanicGoal",
    "SitWhenOrderedToGoal",
    "CatRelaxOnOwnerGoal",
    "CatTemptGoal",
    "CatLieOnBlockGoal",
    "FollowOwnerGoal",
    "CatSitOnBlockGoal",
    "LeapAtTargetGoal",
    "OcelotAttackGoal",
    "BreedGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "CatAvoidEntityGoal",
];
pub const CAT_TARGET_NAMES: [&str; 2] = ["NonTameRandomTargetGoal", "NonTameRandomTargetGoal"];
pub const OCELOT_GOAL_NAMES: [&str; 8] = [
    "FloatGoal",
    "OcelotTemptGoal",
    "LeapAtTargetGoal",
    "OcelotAttackGoal",
    "BreedGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "OcelotAvoidEntityGoal",
];
pub const OCELOT_TARGET_NAMES: [&str; 2] =
    ["NearestAttackableTargetGoal", "NearestAttackableTargetGoal"];
pub const TEMPT_RANGE: f64 = 10.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FelinePlayer {
    pub id: u64,
    pub position: DVec3,
    pub eye_height: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub sneaking: bool,
    pub holds_food: bool,
}

#[derive(Clone, Debug, Default)]
pub struct TemptState {
    pub player: Option<u64>,
    pub px: f64,
    pub py: f64,
    pub pz: f64,
    pub rot_x: f64,
    pub rot_y: f64,
    pub calm_down: i32,
    pub running: bool,
    pub selected: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct FelineState {
    pub cat: bool,
    pub tame: bool,
    pub trusting: bool,
    pub owner: Option<u64>,
    pub ordered_to_sit: bool,
    pub sitting: bool,
    pub people: Vec<FelinePlayer>,
    pub tempt: TemptState,
    pub last_damage: Option<&'static str>,
    pub panic: crate::panic::PanicState,
    pub owner_hurt: bool,
    pub owner_sleeping: Option<(i32, i32, i32)>,
    pub owner_sleep_timer: i32,
    pub resting: Vec<(DVec3, f32, f32)>,
    pub gift_chance: f32,
    pub gift: bool,
    pub lying: bool,
    pub relaxed: bool,
    pub teleported: bool,
    pub follow_recalc: i32,
    pub follow_water_cost: f32,
    pub in_love: bool,
    pub mates: Vec<FelineMate>,
    pub breed_partner: Option<u64>,
    pub love_time: i32,
    pub breed_with: Option<u64>,
}

impl FelineState {
    fn person(&self, id: u64) -> Option<FelinePlayer> {
        self.people.iter().copied().find(|p| p.id == id)
    }
}

pub fn panic_cause(kind: &str) -> bool {
    matches!(
        kind,
        "minecraft:cactus"
            | "minecraft:freeze"
            | "minecraft:hot_floor"
            | "minecraft:sulfur_cube_hot"
            | "minecraft:in_fire"
            | "minecraft:lava"
            | "minecraft:lightning_bolt"
            | "minecraft:on_fire"
            | "minecraft:arrow"
            | "minecraft:dragon_breath"
            | "minecraft:explosion"
            | "minecraft:fireball"
            | "minecraft:fireworks"
            | "minecraft:indirect_magic"
            | "minecraft:magic"
            | "minecraft:mob_attack"
            | "minecraft:mob_projectile"
            | "minecraft:player_explosion"
            | "minecraft:sonic_boom"
            | "minecraft:sting"
            | "minecraft:thrown"
            | "minecraft:trident"
            | "minecraft:unattributed_fireball"
            | "minecraft:wind_charge"
            | "minecraft:wither"
            | "minecraft:wither_skull"
            | "minecraft:player_attack"
            | "minecraft:spear"
    )
}

pub fn cat_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, FloatGoal);
    selector.add(1, PanicGoal { speed: 1.5 });
    selector.add(2, SitGoal);
    selector.add(
        3,
        RelaxOnOwnerGoal {
            goal_pos: None,
            on_bed_ticks: 0,
        },
    );
    selector.add(
        4,
        TemptGoal {
            speed: crate::feline::TEMPT_SPEED,
        },
    );
    selector.add(5, BlockGoal::lie());
    selector.add(
        6,
        FollowOwnerGoal {
            speed: 1.0,
            start: 10.0,
            stop: 5.0,
        },
    );
    selector.add(7, BlockGoal::sit());
    selector.add(8, LeapGoal::new(0.3));
    selector.add(
        9,
        AttackGoal {
            target: None,
            attack_time: 0,
        },
    );
    selector.add(10, BreedGoal { speed: 0.8 });
    selector.add(
        11,
        StrollGoal::new(crate::feline::WALK_SPEED, 1.000_000_1e-5),
    );
    selector.add(12, LookAtPlayerGoal { range: 10.0 });
    selector.add(
        4,
        AvoidPlayersGoal {
            max_dist: 16.0,
            walk: crate::feline::WALK_SPEED,
            sprint: crate::feline::SPRINT_SPEED,
            to_avoid: None,
            path: None,
        },
    );
    selector
}

pub fn cat_targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        NearestTargetGoal::every(Prey::Kinds(&["minecraft:rabbit"]), false, 5).wild_only(),
    );
    selector.add(
        1,
        NearestTargetGoal::every(Prey::Absent, false, 5).wild_only(),
    );
    selector
}

pub fn ocelot_goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, FloatGoal);
    selector.add(
        3,
        TemptGoal {
            speed: crate::feline::TEMPT_SPEED,
        },
    );
    selector.add(7, LeapGoal::new(0.3));
    selector.add(
        8,
        AttackGoal {
            target: None,
            attack_time: 0,
        },
    );
    selector.add(9, BreedGoal { speed: 0.8 });
    selector.add(
        10,
        StrollGoal::new(crate::feline::WALK_SPEED, 1.000_000_1e-5),
    );
    selector.add(11, LookAtPlayerGoal { range: 10.0 });
    selector.add(
        4,
        AvoidPlayersGoal {
            max_dist: 16.0,
            walk: crate::feline::WALK_SPEED,
            sprint: crate::feline::SPRINT_SPEED,
            to_avoid: None,
            path: None,
        },
    );
    selector
}

pub fn ocelot_targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, NearestTargetGoal::every(Prey::Chicken, false, 5));
    selector.add(1, NearestTargetGoal::every(Prey::Absent, false, 5));
    selector
}

#[derive(Clone)]
struct PanicGoal {
    speed: f64,
}
impl Goal<MonsterGoalContext> for PanicGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let panics = ctx.feline.last_damage.is_some_and(panic_cause);
        let profile = ctx.profile();
        let (on_fire, position) = (ctx.on_fire, ctx.body.position);
        ctx.feline
            .panic
            .can_use(panics, on_fire, position, world, &profile, &mut ctx.random)
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }
    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if let Some(target) = ctx.feline.panic.wanted {
            let profile = ctx.profile();
            let _ = navigate_walk_to(
                &ctx.body,
                &mut ctx.navigation,
                world,
                &profile,
                ctx.fluid,
                target,
                self.speed,
                1,
            );
        }
        ctx.feline.panic.running = true;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.feline.panic.running = false;
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if let Some(owner) = owner_of(ctx) {
            if !unable_to_move_to_owner(ctx)
                && ctx.body.position.distance_squared(owner.position) >= 144.0
            {
                teleport_to_owner(ctx, world, owner.position);
            }
        }
    }
}

fn reduced_tick_delay(ticks: u32) -> u32 {
    ticks.div_ceil(2)
}

#[derive(Clone)]
struct TemptGoal {
    speed: f64,
}
impl TemptGoal {
    fn can_scare(ctx: &MonsterGoalContext) -> bool {
        let tempt = &ctx.feline.tempt;
        if ctx.feline.cat {
            !(tempt.selected.is_some() && tempt.selected == tempt.player)
        } else {
            !ctx.feline.trusting
        }
    }

    fn can_use(ctx: &mut MonsterGoalContext) -> bool {
        if ctx.feline.tempt.calm_down > 0 {
            ctx.feline.tempt.calm_down -= 1;
            return false;
        }
        let position = ctx.body.position;
        let mut best: Option<(f64, u64)> = None;
        for person in &ctx.feline.people {
            let Some(candidate) = ctx.players.iter().find(|p| p.id == person.id) else {
                continue;
            };
            if !candidate.alive || candidate.spectator || !person.holds_food {
                continue;
            }
            let range = (TEMPT_RANGE * if person.sneaking { 0.8 } else { 1.0 }).max(2.0);
            let distance = position.distance_squared(person.position);
            if distance > range * range {
                continue;
            }
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, person.id));
            }
        }
        ctx.feline.tempt.player = best.map(|(_, id)| id);
        best.is_some() && !(ctx.feline.cat && ctx.feline.tame)
    }
}
impl Goal<MonsterGoalContext> for TemptGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        Self::can_use(ctx)
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        if Self::can_scare(ctx) {
            if let Some(player) = ctx.feline.tempt.player.and_then(|id| ctx.feline.person(id)) {
                let tempt = &mut ctx.feline.tempt;
                if ctx.body.position.distance_squared(player.position) < 36.0 {
                    if player
                        .position
                        .distance_squared(DVec3::new(tempt.px, tempt.py, tempt.pz))
                        > 0.010_000_000_000_000_002
                    {
                        return false;
                    }
                    if (f64::from(player.pitch) - tempt.rot_x).abs() > 5.0
                        || (f64::from(player.yaw) - tempt.rot_y).abs() > 5.0
                    {
                        return false;
                    }
                } else {
                    tempt.px = player.position.x;
                    tempt.py = player.position.y;
                    tempt.pz = player.position.z;
                }
                tempt.rot_x = f64::from(player.pitch);
                tempt.rot_y = f64::from(player.yaw);
            }
        }
        Self::can_use(ctx)
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(player) = ctx.feline.tempt.player.and_then(|id| ctx.feline.person(id)) {
            let tempt = &mut ctx.feline.tempt;
            tempt.px = player.position.x;
            tempt.py = player.position.y;
            tempt.pz = player.position.z;
        }
        ctx.feline.tempt.running = true;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.feline.tempt.player = None;
        ctx.navigation.stop();
        ctx.feline.tempt.calm_down = reduced_tick_delay(100) as i32;
        ctx.feline.tempt.running = false;
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(player) = ctx.feline.tempt.player.and_then(|id| ctx.feline.person(id)) else {
            return;
        };
        let eyes = player.position + DVec3::Y * f64::from(player.eye_height);
        let x_max = ctx.max_head_x_rot();
        ctx.look_control.set_look_at_with_limits(eyes, 95.0, x_max);
        if ctx.body.position.distance_squared(player.position) < 6.25 {
            ctx.navigation.stop();
        } else {
            let profile = ctx.profile();
            let _ = navigate_walk_to_entity(
                &ctx.body,
                &mut ctx.navigation,
                world,
                &profile,
                ctx.fluid,
                player.position,
                self.speed,
                1,
            );
        }
        if ctx.feline.cat {
            let tempt = &mut ctx.feline.tempt;
            if tempt.selected.is_none() && ctx.random.next_int(reduced_tick_delay(600)) == 0 {
                tempt.selected = tempt.player;
            } else if ctx.random.next_int(reduced_tick_delay(500)) == 0 {
                tempt.selected = None;
            }
        }
    }
}

#[derive(Clone)]
struct AttackGoal {
    target: Option<Target>,
    attack_time: i32,
}
impl Goal<MonsterGoalContext> for AttackGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let Some(target) = ctx.target() else {
            return false;
        };
        self.target = Some(target.target);
        true
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let Some(target) = self.target.and_then(|t| ctx.info(t)) else {
            return false;
        };
        if !target.alive || ctx.body.position.distance_squared(target.position) > 225.0 {
            return false;
        }
        !ctx.navigation.is_done() || self.can_start(ctx, world)
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.target = None;
        ctx.navigation.stop();
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(target) = self.target.and_then(|t| ctx.info(t)) else {
            return;
        };
        ctx.look_control.set_look_at_with_limits(
            target.position + DVec3::Y * f64::from(target.eye_height),
            30.0,
            30.0,
        );
        let width = f64::from(ctx.body.width * 2.0 * ctx.body.width * 2.0);
        let distance = ctx.body.position.distance_squared(target.position);
        let speed = if distance > width && distance < 16.0 {
            crate::feline::SPRINT_SPEED
        } else if distance < 225.0 {
            crate::feline::TEMPT_SPEED
        } else {
            crate::feline::WALK_SPEED
        };
        let profile = ctx.profile();
        let _ = navigate_walk_to_entity(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            target.position,
            speed,
            1,
        );
        self.attack_time = (self.attack_time - 1).max(0);
        if distance <= width && self.attack_time <= 0 {
            self.attack_time = 20;
            ctx.attack = Some(target.target);
        }
    }
}

#[derive(Clone)]
struct AvoidPlayersGoal {
    max_dist: f64,
    walk: f64,
    sprint: f64,
    to_avoid: Option<u64>,
    path: Option<PlannedPath>,
}
impl Goal<MonsterGoalContext> for AvoidPlayersGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if !ctx.feline.cat && ctx.feline.trusting || ctx.feline.cat && ctx.feline.tame {
            return false;
        }
        let body = ctx.body.clone();
        let half = f64::from(body.width / 2.0);
        let p = body.position;
        let (min, max) = (
            DVec3::new(
                p.x - half - self.max_dist,
                p.y - 3.0,
                p.z - half - self.max_dist,
            ),
            DVec3::new(
                p.x + half + self.max_dist,
                p.y + f64::from(body.height) + 3.0,
                p.z + half + self.max_dist,
            ),
        );
        let mut best: Option<(f64, TargetInfo)> = None;
        for person in ctx.feline.people.clone() {
            let Some(info) = ctx.info(Target::Player(person.id)) else {
                continue;
            };
            let q = info.position;
            let inside = q.x - 0.3 < max.x
                && q.x + 0.3 > min.x
                && q.y < max.y
                && q.y + 1.8 > min.y
                && q.z - 0.3 < max.z
                && q.z + 0.3 > min.z;
            if !inside {
                continue;
            }
            let range = (self.max_dist * if person.sneaking { 0.8 } else { 1.0 }).max(2.0);
            let distance = p.distance_squared(q);
            if !ctx.can_attack(info) || distance > range * range || !ctx.sees(world, info) {
                continue;
            }
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, info));
            }
        }
        let Some((_, avoid)) = best else { return false };
        let profile = ctx.profile();
        let Some(pos) = crate::stroll::default_random_position_away(
            world,
            &profile,
            p,
            16,
            7,
            avoid.position,
            &mut ctx.random,
        ) else {
            return false;
        };
        if avoid.position.distance_squared(pos) < avoid.position.distance_squared(p) {
            return false;
        }
        self.path = plan_walk_path(
            &body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            pos,
            0,
        );
        self.to_avoid = match avoid.target {
            Target::Player(id) => Some(id),
            _ => None,
        };
        self.path.is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let wild = if ctx.feline.cat {
            !ctx.feline.tame
        } else {
            !ctx.feline.trusting
        };
        wild && !ctx.navigation.is_done()
    }
    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let position = ctx.body.position;
        ctx.navigation
            .move_to_in(world, self.path.take(), self.walk, position);
    }
    fn stop(&mut self, _ctx: &mut MonsterGoalContext) {
        self.to_avoid = None;
    }
    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        let Some(avoid) = self.to_avoid.and_then(|id| ctx.info(Target::Player(id))) else {
            return;
        };
        ctx.navigation.speed_modifier = if ctx.body.position.distance_squared(avoid.position) < 49.0
        {
            self.sprint
        } else {
            self.walk
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FelineMate {
    pub id: u64,
    pub position: DVec3,
    pub width: f32,
    pub height: f32,
    pub alive: bool,
    pub cat: bool,
    pub tame: bool,
    pub in_love: bool,
    pub panicking: bool,
}

fn owner_of(ctx: &MonsterGoalContext) -> Option<crate::tempt::PlayerCandidate> {
    let owner = ctx.feline.owner?;
    ctx.players.iter().find(|p| p.id == owner).copied()
}

fn unable_to_move_to_owner(ctx: &MonsterGoalContext) -> bool {
    ctx.feline.ordered_to_sit || owner_of(ctx).is_some_and(|o| o.spectator)
}

fn teleport_to_owner(ctx: &mut MonsterGoalContext, world: &dyn World, owner: DVec3) {
    let target = (
        owner.x.floor() as i32,
        owner.y.floor() as i32,
        owner.z.floor() as i32,
    );
    for _ in 0..10 {
        let xd = ctx.random.next_int(7) as i32 - 3;
        let zd = ctx.random.next_int(7) as i32 - 3;
        if xd.abs() < 2 && zd.abs() < 2 {
            continue;
        }
        let yd = ctx.random.next_int(3) as i32 - 1;
        let pos = (target.0 + xd, target.1 + yd, target.2 + zd);
        if crate::walk_path::path_type_static(world, pos) != PathType::Walkable {
            continue;
        }
        if world
            .block((pos.0, pos.1 - 1, pos.2))
            .is_some_and(|b| b.id.ends_with("_leaves"))
        {
            continue;
        }
        let p = ctx.body.position;
        let here = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let delta = DVec3::new(
            f64::from(pos.0 - here.0),
            f64::from(pos.1 - here.1),
            f64::from(pos.2 - here.2),
        );
        let half = f64::from(ctx.body.width / 2.0);
        let min = DVec3::new(p.x - half, p.y, p.z - half) + delta;
        let max = DVec3::new(p.x + half, p.y + f64::from(ctx.body.height), p.z + half) + delta;
        if !minecraftoss_player::collision::no_block_collision(world, min, max) {
            continue;
        }
        ctx.body.position = DVec3::new(
            f64::from(pos.0) + 0.5,
            f64::from(pos.1),
            f64::from(pos.2) + 0.5,
        );
        ctx.navigation.stop();
        ctx.feline.teleported = true;
        return;
    }
}

pub fn random_teleport(body: &mut Body, world: &dyn World, x: f64, mut y: f64, z: f64) -> bool {
    let (bx, bz) = (x.floor() as i32, z.floor() as i32);
    let mut by = y.floor() as i32;
    while by > world.min_y() {
        by -= 1;
        if world.block_in_tag((bx, by, bz), "minecraft:entities_can_teleport_to") {
            if world.block_in_tag((bx, by, bz), crate::feline::NO_TELEPORT_TAG) {
                return false;
            }
            let half = f64::from(body.width / 2.0);
            let min = DVec3::new(x - half, y, z - half);
            let max = DVec3::new(x + half, y + f64::from(body.height), z + half);
            if !minecraftoss_player::collision::no_block_collision(world, min, max) {
                return false;
            }
            let (lo, hi) = (min.floor(), max.floor());
            for cx in lo.x as i32..=hi.x as i32 {
                for cy in lo.y as i32..=hi.y as i32 {
                    for cz in lo.z as i32..=hi.z as i32 {
                        let fluid = world
                            .block((cx, cy, cz))
                            .is_some_and(|b| minecraftoss_player::holds_fluid(&b));
                        if fluid || world.block_in_tag((cx, cy, cz), crate::feline::NO_TELEPORT_TAG)
                        {
                            return false;
                        }
                    }
                }
            }
            body.position = DVec3::new(x, y, z);
            return true;
        }
        y -= 1.0;
    }
    false
}

pub fn find_nearest_block(
    origin: (i32, i32, i32),
    range: i32,
    vertical: i32,
    start: i32,
    valid: impl Fn((i32, i32, i32)) -> bool,
) -> Option<(i32, i32, i32)> {
    let mut k = start;
    while k <= vertical {
        for l in 0..range {
            let mut m = 0;
            while m <= l {
                let mut n = if m < l && m > -l { l } else { 0 };
                while n <= l {
                    let pos = (origin.0 + m, origin.1 + k - 1, origin.2 + n);
                    if valid(pos) {
                        return Some(pos);
                    }
                    n = if n > 0 { -n } else { 1 - n };
                }
                m = if m > 0 { -m } else { 1 - m };
            }
        }
        k = if k > 0 { -k } else { 1 - k };
    }
    None
}

fn empty(world: &dyn World, pos: (i32, i32, i32)) -> bool {
    world.block(pos).is_none_or(|b| {
        matches!(
            b.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

pub fn can_lie_on(world: &dyn World, pos: (i32, i32, i32)) -> bool {
    empty(world, (pos.0, pos.1 + 1, pos.2)) && world.block_in_tag(pos, "minecraft:cats_can_lie_on")
}

pub fn can_sit_on(world: &dyn World, pos: (i32, i32, i32)) -> bool {
    if !empty(world, (pos.0, pos.1 + 1, pos.2))
        || !world.block_in_tag(pos, "minecraft:cats_can_sit_on")
    {
        return false;
    }
    let Some(block) = world.block(pos) else {
        return false;
    };
    if block.id == "minecraft:chest" {
        return true;
    }
    if block.id == "minecraft:furnace" {
        return block.property("lit") == Some("true");
    }
    if world.block_in_tag(pos, "minecraft:beds") {
        return block.property("part") != Some("head");
    }
    true
}

fn block_center_within(block: (i32, i32, i32), position: DVec3, distance: f64) -> bool {
    let center = DVec3::new(
        f64::from(block.0) + 0.5,
        f64::from(block.1) + 0.5,
        f64::from(block.2) + 0.5,
    );
    center.distance_squared(position) < distance * distance
}

#[derive(Clone)]
struct SitGoal;
impl Goal<MonsterGoalContext> for SitGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Jump, Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let ordered = ctx.feline.ordered_to_sit;
        if !ordered && !ctx.feline.tame || ctx.fluid.in_water() || !ctx.body.on_ground {
            return false;
        }
        let Some(owner) = owner_of(ctx) else {
            return true;
        };
        if ctx.body.position.distance_squared(owner.position) < 144.0 && ctx.feline.owner_hurt {
            false
        } else {
            ordered
        }
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.feline.ordered_to_sit
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.navigation.stop();
        ctx.feline.sitting = true;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.feline.sitting = false;
    }
}

#[derive(Clone)]
struct RelaxOnOwnerGoal {
    goal_pos: Option<(i32, i32, i32)>,
    on_bed_ticks: i32,
}
impl RelaxOnOwnerGoal {
    fn space_is_occupied(&self, ctx: &MonsterGoalContext) -> bool {
        let Some(goal) = self.goal_pos else {
            return false;
        };
        let min = DVec3::new(
            f64::from(goal.0) - 2.0,
            f64::from(goal.1) - 2.0,
            f64::from(goal.2) - 2.0,
        );
        let max = DVec3::new(
            f64::from(goal.0) + 3.0,
            f64::from(goal.1) + 3.0,
            f64::from(goal.2) + 3.0,
        );
        ctx.feline.resting.iter().any(|&(position, width, height)| {
            let half = f64::from(width / 2.0);
            position.x - half < max.x
                && position.x + half > min.x
                && position.y < max.y
                && position.y + f64::from(height) > min.y
                && position.z - half < max.z
                && position.z + half > min.z
        })
    }
}
impl Goal<MonsterGoalContext> for RelaxOnOwnerGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if !ctx.feline.tame || ctx.feline.ordered_to_sit {
            return false;
        }
        let Some(owner) = owner_of(ctx) else {
            return false;
        };
        let Some(bed) = ctx.feline.owner_sleeping else {
            return false;
        };
        if ctx.body.position.distance_squared(owner.position) > 100.0 {
            return false;
        }
        if !world.block_in_tag(bed, "minecraft:cats_can_lie_on") {
            return false;
        }
        let facing = world
            .block(bed)
            .and_then(|b| b.property("facing").map(str::to_owned));
        self.goal_pos = Some(match facing.as_deref() {
            Some("north") => (bed.0, bed.1, bed.2 + 1),
            Some("south") => (bed.0, bed.1, bed.2 - 1),
            Some("west") => (bed.0 + 1, bed.1, bed.2),
            Some("east") => (bed.0 - 1, bed.1, bed.2),
            _ => bed,
        });
        !self.space_is_occupied(ctx)
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.feline.tame
            && !ctx.feline.ordered_to_sit
            && owner_of(ctx).is_some()
            && ctx.feline.owner_sleeping.is_some()
            && self.goal_pos.is_some()
            && !self.space_is_occupied(ctx)
    }
    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if let Some(goal) = self.goal_pos {
            ctx.feline.sitting = false;
            let target = DVec3::new(f64::from(goal.0), f64::from(goal.1), f64::from(goal.2));
            let profile = ctx.profile();
            let _ = navigate_walk_to(
                &ctx.body,
                &mut ctx.navigation,
                world,
                &profile,
                ctx.fluid,
                target,
                1.100_000_023_841_858,
                1,
            );
        }
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.feline.lying = false;
        if ctx.feline.owner_sleep_timer >= 100 {
            let roll = ctx.level_random.as_mut().map(|random| random.next_float());
            if roll.is_some_and(|roll| roll < ctx.feline.gift_chance) {
                ctx.feline.gift = true;
            }
        }
        self.on_bed_ticks = 0;
        ctx.feline.relaxed = false;
        ctx.navigation.stop();
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let (Some(owner), Some(goal)) = (owner_of(ctx), self.goal_pos) else {
            return;
        };
        ctx.feline.sitting = false;
        let target = DVec3::new(f64::from(goal.0), f64::from(goal.1), f64::from(goal.2));
        let profile = ctx.profile();
        let _ = navigate_walk_to(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            target,
            1.100_000_023_841_858,
            1,
        );
        if ctx.body.position.distance_squared(owner.position) < 2.5 {
            self.on_bed_ticks += 1;
            if self.on_bed_ticks > reduced_tick_delay(16) as i32 {
                ctx.feline.lying = true;
                ctx.feline.relaxed = false;
            } else {
                if let Some(info) = ctx.info(Target::Player(owner.id)) {
                    crate::monster_ai::look_at(ctx, info, 45.0, 45.0);
                }
                ctx.feline.relaxed = true;
            }
        } else {
            ctx.feline.lying = false;
        }
    }
}

#[derive(Clone)]
struct BlockGoal {
    lie: bool,
    speed: f64,
    next_start_tick: i32,
    try_ticks: i32,
    max_stay_ticks: i32,
    block: (i32, i32, i32),
    reached: bool,
}
impl BlockGoal {
    fn lie() -> Self {
        Self {
            lie: true,
            speed: 1.1,
            next_start_tick: 0,
            try_ticks: 0,
            max_stay_ticks: 0,
            block: (0, 0, 0),
            reached: false,
        }
    }
    fn sit() -> Self {
        Self {
            lie: false,
            speed: 0.8,
            next_start_tick: 0,
            try_ticks: 0,
            max_stay_ticks: 0,
            block: (0, 0, 0),
            reached: false,
        }
    }
    fn valid(&self, world: &dyn World, pos: (i32, i32, i32)) -> bool {
        if self.lie {
            can_lie_on(world, pos)
        } else {
            can_sit_on(world, pos)
        }
    }
    fn move_to(&self, ctx: &mut MonsterGoalContext, world: &dyn World, y_offset: f64) {
        let target = DVec3::new(
            f64::from(self.block.0) + 0.5,
            f64::from(self.block.1) + y_offset,
            f64::from(self.block.2) + 0.5,
        );
        let profile = ctx.profile();
        let _ = navigate_walk_to(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            target,
            self.speed,
            1,
        );
    }
}
impl Goal<MonsterGoalContext> for BlockGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Jump])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if !ctx.feline.tame || ctx.feline.ordered_to_sit || self.lie && ctx.feline.lying {
            return false;
        }
        if self.next_start_tick > 0 {
            self.next_start_tick -= 1;
            return false;
        }
        self.next_start_tick = if self.lie {
            40
        } else {
            (200 + ctx.random.next_int(200) as i32 + 1) / 2
        };
        let p = ctx.body.position;
        let origin = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let (vertical, start) = if self.lie { (6, -2) } else { (1, 0) };
        match find_nearest_block(origin, 8, vertical, start, |pos| self.valid(world, pos)) {
            Some(block) => {
                self.block = block;
                true
            }
            None => false,
        }
    }
    fn can_continue(&mut self, _ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.try_ticks >= -self.max_stay_ticks
            && self.try_ticks <= 1200
            && self.valid(world, self.block)
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.move_to(ctx, world, 1.0);
        self.try_ticks = 0;
        let bound = ctx.random.next_int(1200) + 1200;
        self.max_stay_ticks = ctx.random.next_int(bound) as i32 + 1200;
        ctx.feline.sitting = false;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if self.lie {
            ctx.feline.lying = false;
        } else {
            ctx.feline.sitting = false;
        }
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let target = (self.block.0, self.block.1 + 1, self.block.2);
        if !block_center_within(target, ctx.body.position, 1.0) {
            self.reached = false;
            self.try_ticks += 1;
            if self.try_ticks % 40 == 0 {
                self.move_to(ctx, world, 1.0);
            }
        } else {
            self.reached = true;
            self.try_ticks -= 1;
        }
        if self.lie {
            ctx.feline.sitting = false;
            if !self.reached {
                ctx.feline.lying = false;
            } else if !ctx.feline.lying {
                ctx.feline.lying = true;
            }
        } else {
            ctx.feline.sitting = self.reached;
        }
    }
}

#[derive(Clone)]
struct FollowOwnerGoal {
    speed: f64,
    start: f64,
    stop: f64,
}
impl Goal<MonsterGoalContext> for FollowOwnerGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let Some(owner) = owner_of(ctx) else {
            return false;
        };
        !unable_to_move_to_owner(ctx)
            && ctx.body.position.distance_squared(owner.position) >= self.start * self.start
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        if ctx.navigation.is_done() || unable_to_move_to_owner(ctx) {
            return false;
        }
        owner_of(ctx).is_some_and(|owner| {
            ctx.body.position.distance_squared(owner.position) > self.stop * self.stop
        })
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.feline.follow_recalc = 0;
        ctx.feline.follow_water_cost = ctx.walk.malus(PathType::Water);
        ctx.walk.set_malus(PathType::Water, 0.0);
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.navigation.stop();
        let cost = ctx.feline.follow_water_cost;
        ctx.walk.set_malus(PathType::Water, cost);
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(owner) = owner_of(ctx) else {
            return;
        };
        let far = ctx.body.position.distance_squared(owner.position) >= 144.0;
        if !far {
            let eyes = owner.position + DVec3::Y * f64::from(owner.eye_height);
            let x_max = ctx.max_head_x_rot();
            ctx.look_control.set_look_at_with_limits(eyes, 10.0, x_max);
        }
        ctx.feline.follow_recalc -= 1;
        if ctx.feline.follow_recalc <= 0 {
            ctx.feline.follow_recalc = reduced_tick_delay(10) as i32;
            if far {
                teleport_to_owner(ctx, world, owner.position);
            } else {
                let profile = ctx.profile();
                let _ = navigate_walk_to_entity(
                    &ctx.body,
                    &mut ctx.navigation,
                    world,
                    &profile,
                    ctx.fluid,
                    owner.position,
                    self.speed,
                    1,
                );
            }
        }
    }
}

#[derive(Clone)]
struct BreedGoal {
    speed: f64,
}
impl BreedGoal {
    fn partner(ctx: &MonsterGoalContext) -> Option<FelineMate> {
        let id = ctx.feline.breed_partner?;
        ctx.feline.mates.iter().find(|m| m.id == id).copied()
    }
    fn can_mate(ctx: &MonsterGoalContext, mate: &FelineMate) -> bool {
        let both_in_love = ctx.feline.in_love && mate.in_love && mate.cat == ctx.feline.cat;
        if ctx.feline.cat {
            ctx.feline.tame && mate.tame && both_in_love
        } else {
            both_in_love
        }
    }
}
impl Goal<MonsterGoalContext> for BreedGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        if !ctx.feline.in_love {
            return false;
        }
        let body = &ctx.body;
        let half = f64::from(body.width / 2.0);
        let p = body.position;
        let (min, max) = (
            DVec3::new(p.x - half - 8.0, p.y - 8.0, p.z - half - 8.0),
            DVec3::new(
                p.x + half + 8.0,
                p.y + f64::from(body.height) + 8.0,
                p.z + half + 8.0,
            ),
        );
        let mut best: Option<(f64, u64)> = None;
        for m in &ctx.feline.mates {
            let mh = f64::from(m.width / 2.0);
            let q = m.position;
            let inside = q.x - mh < max.x
                && q.x + mh > min.x
                && q.y < max.y
                && q.y + f64::from(m.height) > min.y
                && q.z - mh < max.z
                && q.z + mh > min.z;
            let distance = p.distance_squared(q);
            if !inside || !m.alive || distance > 64.0 {
                continue;
            }
            if Self::can_mate(ctx, m) && !m.panicking && best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, m.id));
            }
        }
        ctx.feline.breed_partner = best.map(|(_, id)| id);
        best.is_some()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        Self::partner(ctx).is_some_and(|m| m.alive && m.in_love && !m.panicking)
            && ctx.feline.love_time < 60
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.feline.breed_partner = None;
        ctx.feline.love_time = 0;
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(partner) = Self::partner(ctx) else {
            return;
        };
        let eye = if partner.height < crate::feline::HEIGHT {
            crate::feline::BABY_EYE_HEIGHT
        } else if partner.cat {
            crate::feline::CAT_EYE_HEIGHT
        } else {
            crate::feline::OCELOT_EYE_HEIGHT
        };
        let x_max = ctx.max_head_x_rot();
        ctx.look_control.set_look_at_with_limits(
            partner.position + DVec3::Y * f64::from(eye),
            10.0,
            x_max,
        );
        let profile = ctx.profile();
        let _ = navigate_walk_to_entity(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            partner.position,
            self.speed,
            1,
        );
        ctx.feline.love_time += 1;
        if ctx.feline.love_time >= 30 && ctx.body.position.distance_squared(partner.position) < 9.0
        {
            ctx.feline.breed_with = Some(partner.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panic_causes_follow_the_damage_type_tag() {
        assert!(
            panic_cause("minecraft:player_attack")
                && panic_cause("minecraft:on_fire")
                && panic_cause("minecraft:arrow")
        );
        assert!(
            !panic_cause("minecraft:fall")
                && !panic_cause("minecraft:drown")
                && !panic_cause("minecraft:generic")
        );
        assert_eq!(
            (
                reduced_tick_delay(100),
                reduced_tick_delay(600),
                reduced_tick_delay(500)
            ),
            (50, 300, 250)
        );
    }
}
