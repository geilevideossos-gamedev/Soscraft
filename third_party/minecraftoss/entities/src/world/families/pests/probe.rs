use crate::loot::EntityLootContext;
use crate::monster_ai::{MonsterAi, MonsterKind};
use crate::tempt::PlayerCandidate;
use crate::world::families::{Family, FamilyMob, MobCore};
use crate::world::{EntityWorld, PlayerHitKind};
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
    pub ai: Box<MonsterAi>,
}

impl Family for Probe {
    const KEY: u8 = 39;
    const KINDS: &'static [&'static str] = &[KIND, ANIMAL];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let ai = MonsterAi::of_kind(MonsterKind::Spider, &core.body, core.yaw);
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
            let strike = world
                .family_goal_step::<Self, W>(id, level, players, SPEED, |probe| &mut *probe.ai);
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

    fn death_drops(mob: &mut FamilyMob<Self>) -> Vec<ItemStack> {
        std::mem::take(&mut mob.state.drops)
    }

    fn loot(_mob: &FamilyMob<Self>, context: &mut EntityLootContext) {
        context.cube_size = Some(CUBE_SIZE);
    }

    fn leash_walk<W: World>(mob: &mut FamilyMob<Self>, _level: &W, target: DVec3, speed: f64) {
        mob.state.walks.push((target, speed));
    }
}
