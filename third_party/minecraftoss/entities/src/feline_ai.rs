use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::monster_ai::{
    FloatGoal, LeapGoal, LookAtPlayerGoal, MonsterGoalContext, NearestTargetGoal, NeverGoal, Prey,
    StrollGoal, Target, TargetInfo,
};
use crate::navigation::{PlannedPath, navigate_walk_to, navigate_walk_to_entity, plan_walk_path};
use glam::DVec3;
use minecraftoss_player::World;

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
    selector.add(
        2,
        NeverGoal {
            controls: &[Control::Jump, Control::Move],
        },
    );
    selector.add(
        3,
        NeverGoal {
            controls: &[Control::Move],
        },
    );
    selector.add(
        4,
        TemptGoal {
            speed: crate::feline::TEMPT_SPEED,
        },
    );
    selector.add(
        5,
        NeverGoal {
            controls: &[Control::Move, Control::Jump],
        },
    );
    selector.add(
        6,
        NeverGoal {
            controls: &[Control::Move, Control::Look],
        },
    );
    selector.add(
        7,
        NeverGoal {
            controls: &[Control::Move, Control::Jump],
        },
    );
    selector.add(8, LeapGoal::new(0.3));
    selector.add(
        9,
        AttackGoal {
            target: None,
            attack_time: 0,
        },
    );
    selector.add(
        10,
        NeverGoal {
            controls: &[Control::Move, Control::Look],
        },
    );
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
        NearestTargetGoal::every(Prey::Absent, false, 5).wild_only(),
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
    selector.add(
        9,
        NeverGoal {
            controls: &[Control::Move, Control::Look],
        },
    );
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
