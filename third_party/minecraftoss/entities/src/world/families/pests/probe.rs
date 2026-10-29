use crate::goals::{Controls, Goal, GoalSelector};
use crate::loot::EntityLootContext;
use crate::monster_ai::{
    FloatGoal, HurtByTargetGoal, LookAtPlayerGoal, MeleeAttackGoal, MonsterAi, MonsterGoalContext,
    MonsterKind, NearestTargetGoal, Prey, RandomLookGoal,
};
use crate::tempt::PlayerCandidate;
use crate::walk_path::WalkProfile;
use crate::world::families::{Family, FamilyMob, MobCore, MonsterSpec};
use crate::world::{EntityWorld, PlayerHitKind, Rider};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::inventory::ItemStack;

pub const KIND: &str = "minecraft:registry_probe";
pub const ANIMAL: &str = "minecraft:registry_probe_animal";
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 0.9;
pub const EYE_HEIGHT: f32 = 0.7;
pub const MAX_HEALTH: f32 = 10.0;
pub const CUBE_SIZE: i32 = 3;
pub const SPEED: f64 = 0.3;
pub const DAMAGE: f32 = 2.0;
pub const FOLLOW_RANGE: f64 = 24.0;
pub const STEP: f32 = 1.0;
pub const SEAT_SPACING: f64 = 0.5;

pub const GOAL_NAMES: [&str; 5] = [
    "FloatGoal",
    "MeleeAttackGoal",
    "ProbeGoal",
    "LookAtPlayerGoal",
    "RandomLookGoal",
];
pub const TARGET_NAMES: [&str; 2] = ["HurtByTargetGoal", "NearestAttackableTargetGoal"];

static SPEC: MonsterSpec = MonsterSpec {
    goal_names: &GOAL_NAMES,
    target_names: &TARGET_NAMES,
    goals,
    targets,
    eye_height: EYE_HEIGHT,
    follow_range: FOLLOW_RANGE,
    walk,
};

fn goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, FloatGoal);
    selector.add(
        2,
        MeleeAttackGoal {
            hits: true,
            raises_arms: false,
            follow_unseen: false,
            gives_up_in_light: false,
            speed: 1.0,
        },
    );
    selector.add(3, ProbeGoal);
    selector.add(4, LookAtPlayerGoal { range: 8.0 });
    selector.add(5, RandomLookGoal);
    selector
}

fn targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        HurtByTargetGoal {
            timestamp: 0,
            unseen_ticks: 0,
            target_mob: None,
            alert_others: false,
        },
    );
    selector.add(2, NearestTargetGoal::every(Prey::Kinds(&[ANIMAL]), true, 5));
    selector
}

fn walk(walk: &mut WalkProfile) {
    walk.max_up_step = STEP;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProbeExtra {
    pub allowed: bool,
    pub ticks: u32,
}

#[derive(Clone)]
struct ProbeGoal;

impl Goal<MonsterGoalContext> for ProbeGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.extra
            .get::<ProbeExtra>()
            .is_some_and(|extra| extra.allowed)
    }

    fn every_tick(&self) -> bool {
        true
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(extra) = ctx.extra.get_mut::<ProbeExtra>() {
            extra.ticks += 1;
        }
    }
}

#[derive(Clone)]
pub struct Probe {
    pub undead: bool,
    pub fire_immune: bool,
    pub immune_to_poison: bool,
    pub leashable: bool,
    pub unpushable: bool,
    pub panicking: bool,
    pub knockback_resistance: f64,
    pub drops: Vec<ItemStack>,
    pub walks: Vec<(DVec3, f64)>,
    pub goals: bool,
    pub goal_allowed: bool,
    pub goal_ticks: u32,
    pub seats: usize,
    pub saddled: bool,
    pub mob_steers: bool,
    pub jumps: bool,
    pub jumped: Vec<i32>,
    pub orders: Vec<(DVec3, f64)>,
    pub attachment: f64,
    pub deflects: bool,
    pub deflected: Vec<String>,
    pub ai: Box<MonsterAi>,
}

impl Family for Probe {
    const KEY: u8 = 39;
    const KINDS: &'static [&'static str] = &[KIND, ANIMAL];
    const MOUNT: bool = true;

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        Some(FamilyMob {
            core,
            state: Probe {
                undead: false,
                fire_immune: false,
                immune_to_poison: false,
                leashable: false,
                unpushable: false,
                panicking: false,
                knockback_resistance: 0.0,
                drops: Vec::new(),
                walks: Vec::new(),
                goals: false,
                goal_allowed: false,
                goal_ticks: 0,
                seats: 0,
                saddled: false,
                mob_steers: false,
                jumps: false,
                jumped: Vec::new(),
                orders: Vec::new(),
                attachment: 0.0,
                deflects: false,
                deflected: Vec::new(),
                ai: Box::new(ai),
            },
        })
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        let goals = world
            .family_mob_mut::<Self>(id)
            .is_some_and(|mob| mob.state.goals);
        if base.alive && !base.no_ai && goals {
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                let allowed = mob.state.goal_allowed;
                let extra = &mut mob.state.ai.state.extra;
                match extra.get_mut::<ProbeExtra>() {
                    Some(state) => state.allowed = allowed,
                    None => extra.set(ProbeExtra { allowed, ticks: 0 }),
                }
            }
            let strike = world
                .family_goal_step::<Self, W>(id, level, players, SPEED, |probe| &mut *probe.ai);
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                let extra = mob.state.ai.state.extra.get::<ProbeExtra>();
                mob.state.goal_ticks = extra.map_or(0, |extra| extra.ticks);
            }
            if let Some(target) = strike {
                let attacker = world.body_of(id).map_or(DVec3::ZERO, |body| body.position);
                let kind = PlayerHitKind::Melee {
                    attacker,
                    hunger_ticks: 0,
                    lift: 0.0,
                    throw: DVec3::ZERO,
                    disable_shield_seconds: 0.0,
                };
                world.family_strike(id, target, DAMAGE, kind);
            }
        } else if base.alive && !base.no_ai {
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                let yaw = mob.core.yaw;
                mob.core.body.trim_small_velocity();
                let _ = mob.core.body.travel_air(&*level, DVec3::ZERO, 0.0, yaw);
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn category(kind: &str) -> &'static str {
        if kind == ANIMAL {
            "creature"
        } else {
            "monster"
        }
    }

    fn enemy(kind: &str) -> bool {
        kind == KIND
    }

    fn undead(mob: &FamilyMob<Self>) -> bool {
        mob.state.undead
    }

    fn immune_to_poison(mob: &FamilyMob<Self>) -> bool {
        mob.state.immune_to_poison
    }

    fn fire_immune(mob: &FamilyMob<Self>) -> bool {
        mob.state.fire_immune
    }

    fn leashable(mob: &FamilyMob<Self>) -> bool {
        mob.state.leashable
    }

    fn pushable(mob: &FamilyMob<Self>) -> bool {
        !mob.state.unpushable
    }

    fn panicking(mob: &FamilyMob<Self>) -> bool {
        mob.state.panicking
    }

    fn knockback_resistance(mob: &FamilyMob<Self>) -> f64 {
        mob.state.knockback_resistance
    }

    fn deflect(mob: &mut FamilyMob<Self>, projectile: &str) -> bool {
        if mob.state.deflects {
            mob.state.deflected.push(projectile.to_owned());
        }
        mob.state.deflects
    }

    fn death_drops(mob: &mut FamilyMob<Self>) -> Vec<ItemStack> {
        std::mem::take(&mut mob.state.drops)
    }

    fn monster(kind: &str) -> Option<&'static MonsterSpec> {
        (kind == KIND).then_some(&SPEC)
    }

    fn loot(_mob: &FamilyMob<Self>, context: &mut EntityLootContext) {
        context.cube_size = Some(CUBE_SIZE);
    }

    fn leash_walk<W: World>(mob: &mut FamilyMob<Self>, _level: &W, target: DVec3, speed: f64) {
        mob.state.walks.push((target, speed));
    }

    fn seats(mob: &FamilyMob<Self>) -> usize {
        mob.state.seats
    }

    fn passenger_point(mob: &FamilyMob<Self>, seat: usize) -> DVec3 {
        let height = f64::from(mob.core.body.height);
        DVec3::new(0.0, height, -SEAT_SPACING * seat as f64)
    }

    fn controller(mob: &FamilyMob<Self>, passengers: &[Rider]) -> Option<Rider> {
        match *passengers.first()? {
            Rider::Player(player) if mob.state.saddled => Some(Rider::Player(player)),
            Rider::Mob(id) if mob.state.mob_steers => Some(Rider::Mob(id)),
            _ => None,
        }
    }

    fn rider_jumps(mob: &FamilyMob<Self>) -> bool {
        mob.state.jumps
    }

    fn rider_jump(mob: &mut FamilyMob<Self>, power: i32) {
        mob.state.jumped.push(power);
    }

    fn rider_order(mob: &mut FamilyMob<Self>, target: DVec3, speed: f64) {
        mob.state.orders.push((target, speed));
    }

    fn vehicle_attachment(mob: &FamilyMob<Self>) -> f64 {
        mob.state.attachment
    }
}
