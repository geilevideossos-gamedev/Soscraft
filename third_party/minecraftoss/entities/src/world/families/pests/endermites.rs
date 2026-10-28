use crate::goals::GoalSelector;
use crate::monster_ai::{
    FloatGoal, HurtByTargetGoal, LookAtPlayerGoal, MeleeAttackGoal, MonsterAi, MonsterGoalContext,
    MonsterKind, NearestTargetGoal, Prey, RandomLookGoal, StrollGoal,
};
use crate::tempt::PlayerCandidate;
use crate::walk_path::WalkProfile;
use crate::world::families::goals::ClimbOnTopOfPowderSnowGoal;
use crate::world::families::{Family, FamilyMob, MobCore, MonsterSpec};
use crate::world::{EntityWorld, PlayerHitKind};
use glam::DVec3;
use minecraftoss_player::World;

pub const KIND: &str = "minecraft:endermite";
pub const WIDTH: f32 = 0.4;
pub const HEIGHT: f32 = 0.3;
pub const EYE_HEIGHT: f32 = 0.13;
pub const MAX_HEALTH: f32 = 8.0;
pub const SPEED: f64 = 0.25;
pub const ATTACK_DAMAGE: f32 = 2.0;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const EXPERIENCE: i32 = 3;
pub const MAX_LIFE: i32 = 2400;
pub const STEP_SOUND: &str = "entity.endermite.step";

pub const GOAL_NAMES: [&str; 6] = [
    "FloatGoal",
    "ClimbOnTopOfPowderSnowGoal",
    "MeleeAttackGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
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
    selector.add(1, ClimbOnTopOfPowderSnowGoal);
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
    selector.add(3, StrollGoal::new(1.0, 0.001));
    selector.add(7, LookAtPlayerGoal { range: 8.0 });
    selector.add(8, RandomLookGoal);
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
            alert_others: true,
        },
    );
    selector.add(2, NearestTargetGoal::every(Prey::Player, true, 5));
    selector
}

fn walk(_walk: &mut WalkProfile) {}

#[derive(Clone)]
pub struct Endermite {
    pub life: i32,
    pub ai: Box<MonsterAi>,
}

impl Family for Endermite {
    const KEY: u8 = 37;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        Some(FamilyMob {
            core,
            state: Endermite {
                life: 0,
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
        if base.alive && !base.no_ai {
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                mob.state.ai.body_rotation.body_yaw = mob.core.yaw;
            }
            let strike =
                world.family_goal_step::<Self, W>(id, level, players, SPEED, |mite| &mut *mite.ai);
            let alert = world
                .family_mob_mut::<Self>(id)
                .and_then(|mob| mob.state.ai.state.alert.take());
            if let Some(target) = strike {
                let attacker = world.body_of(id).map_or(DVec3::ZERO, |body| body.position);
                let kind = PlayerHitKind::Melee {
                    attacker,
                    hunger_ticks: 0,
                    lift: 0.0,
                    throw: DVec3::ZERO,
                    disable_shield_seconds: 0.0,
                };
                world.family_strike(id, target, ATTACK_DAMAGE, kind);
            }
            if let Some(attacker) = alert {
                world.family_alert::<Self>(id, attacker, |mite| &mut *mite.ai);
            }
        }
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            if !mob.core.persistent {
                mob.state.life += 1;
            }
            if mob.state.life >= MAX_LIFE {
                mob.core.removed = true;
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn experience(_mob: &mut FamilyMob<Self>) -> i32 {
        EXPERIENCE
    }

    fn enemy(_kind: &str) -> bool {
        true
    }

    fn monster(kind: &str) -> Option<&'static MonsterSpec> {
        (kind == KIND).then_some(&SPEC)
    }

    fn step_sound(_kind: &str) -> Option<&'static str> {
        Some(STEP_SOUND)
    }

    fn emits_sounds(_kind: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enderman::Enderman;
    use crate::monster_ai::Target;
    use crate::world::families::Hurt;
    use crate::world::families::pests::silverfish::{self, Silverfish};
    use crate::world::{DamageSourceKind, PlayerAttack};
    use minecraftoss_player::{Block, Pos};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn player_at(id: u64, position: DVec3) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position,
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true,
        }
    }

    fn attack(player_id: u64, position: DVec3, attack_damage: f64) -> PlayerAttack {
        PlayerAttack {
            player_id,
            position,
            yaw: 0.0,
            attack_damage,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        }
    }

    fn at(x: f64, z: f64) -> DVec3 {
        DVec3::new(x, 64.0, z)
    }

    fn mite_at(world: &mut EntityWorld, x: f64, z: f64) -> u64 {
        world.spawn_family_kind(KIND, at(x, z)).unwrap()
    }

    fn mite(world: &EntityWorld, id: u64) -> &FamilyMob<Endermite> {
        world
            .family::<Endermite>()
            .iter()
            .find(|mob| mob.core.id == id)
            .unwrap()
    }

    fn mite_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Endermite> {
        world.family_mob_mut::<Endermite>(id).unwrap()
    }

    #[test]
    fn an_endermite_is_a_small_eight_health_monster_that_bites_for_two() {
        let mut world = EntityWorld::default();
        let mut field = Field;
        let id = mite_at(&mut world, 0.5, 0.5);
        assert_eq!(world.entity_type(id), Some(KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(37));
        let core = &mite(&world, id).core;
        assert_eq!(
            (core.body.width, core.body.height, core.eye_height),
            (0.4, 0.3, 0.13)
        );
        assert_eq!((core.health, core.max_health), (8.0, 8.0));
        assert_eq!(mite(&world, id).state.life, 0);
        let census = world.census();
        let entry = census.iter().find(|c| c.kind == KIND).unwrap();
        assert_eq!((entry.width, entry.height), (WIDTH, HEIGHT));
        assert!(
            world
                .mob_candidates()
                .iter()
                .any(|m| m.id == id && m.kind == KIND)
        );
        assert!(crate::monster_ai::is_enemy(KIND));
        let traits = world.family_traits(id).unwrap();
        assert_eq!(traits.category, "monster");
        assert!(traits.enemy && !traits.undead && !traits.immune_to_poison);
        assert!(!traits.fire_immune && !traits.breathes_water && !traits.leashable);
        assert_eq!(traits.voice, ("endermite", 1.0, "hostile_volume"));
        let registered = MonsterKind::Registered(KIND);
        assert_eq!(registered.goal_names(), GOAL_NAMES);
        assert_eq!(registered.target_goal_names(), TARGET_NAMES);
        let ai = &mite(&world, id).state.ai;
        assert_eq!(ai.state.follow_range, FOLLOW_RANGE);
        assert_eq!(ai.state.eye_height, EYE_HEIGHT);
        assert!(ai.state.walk.can_float, "its FloatGoal floats the path");
        let players = [player_at(1, at(0.5, 4.5))];
        let (mut hits, mut sounds) = (Vec::new(), Vec::new());
        for _ in 0..200 {
            world.tick_with_players(&mut field, &players);
            hits.extend(world.take_player_hits());
            sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
            if !hits.is_empty() {
                break;
            }
        }
        let hit = hits.first().expect("it bites the player");
        assert_eq!(
            (hit.player_id, hit.damage, hit.source),
            (1, ATTACK_DAMAGE, Some(id))
        );
        let PlayerHitKind::Melee { attacker, .. } = hit.kind else {
            panic!("{:?}", hit.kind);
        };
        assert!(attacker.z > 2.0, "it walked up to bite: {attacker}");
        assert_eq!(mite(&world, id).core.swings, 1);
        assert!(
            sounds
                .iter()
                .all(|s| !s.ends_with(".step") && !s.ends_with(".swim")),
            "it walks without step sounds: {sounds:?}"
        );
        assert!(mite(&world, id).state.life > 0, "its life counts up");
        assert!(
            world
                .player_attack(&attack(1, players[0].position, 100.0), id)
                .died
        );
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        assert_eq!((deaths[0].id, deaths[0].table), (id, Some(KIND)));
    }

    #[test]
    fn an_endermite_leaves_after_2400_ticks_unless_persistent() {
        let mut world = EntityWorld::default();
        let mut field = Field;
        let fresh = mite_at(&mut world, 0.5, 0.5);
        let kept = mite_at(&mut world, 4.5, 0.5);
        let stale = mite_at(&mut world, 8.5, 0.5);
        let walker = mite_at(&mut world, 12.5, 0.5);
        for id in [fresh, kept, stale] {
            mite_mut(&mut world, id).core.no_ai = true;
        }
        mite_mut(&mut world, kept).core.persistent = true;
        mite_mut(&mut world, stale).core.persistent = true;
        mite_mut(&mut world, stale).state.life = MAX_LIFE;
        mite_mut(&mut world, walker).state.life = MAX_LIFE - 5;
        world.tick(&mut field);
        assert_eq!(
            world.entity_type(stale),
            None,
            "a saved life past 2400 ends"
        );
        for _ in 1..4 {
            world.tick(&mut field);
        }
        assert_eq!(mite(&world, walker).state.life, MAX_LIFE - 1);
        world.tick(&mut field);
        assert_eq!(world.entity_type(walker), None, "with its AI too");
        for _ in 5..MAX_LIFE - 1 {
            world.tick(&mut field);
        }
        assert_eq!(mite(&world, fresh).state.life, MAX_LIFE - 1);
        world.tick(&mut field);
        assert_eq!(world.entity_type(fresh), None);
        assert!(!world.mob_ids().any(|id| id == fresh || id == walker));
        assert!(world.family_core(fresh).is_none());
        assert_eq!(mite(&world, kept).state.life, 0);
        assert_eq!(world.entity_type(kept), Some(KIND));
        assert!(world.take_deaths().is_empty(), "it leaves without a death");
    }

    #[test]
    fn an_enderman_hunts_an_endermite() {
        let mut world = EntityWorld::default();
        let mut field = Field;
        let enderman = world.spawn_enderman(Enderman::new(at(0.5, 0.5)), false);
        let fish = world
            .spawn_family_kind(silverfish::KIND, at(0.5, 2.5))
            .unwrap();
        let prey = mite_at(&mut world, 0.5, 5.5);
        world.family_mob_mut::<Silverfish>(fish).unwrap().core.no_ai = true;
        mite_mut(&mut world, prey).core.no_ai = true;
        let target = |world: &EntityWorld| {
            world
                .endermen()
                .iter()
                .find(|e| e.id == enderman)
                .unwrap()
                .ai
                .state
                .target
        };
        for _ in 0..200 {
            world.tick(&mut field);
            if target(&world).is_some() {
                break;
            }
        }
        assert_eq!(target(&world), Some(Target::Mob(prey)));
        let mut hurt = false;
        for _ in 0..200 {
            world.tick(&mut field);
            hurt = world
                .family_core(prey)
                .is_none_or(|(_, core)| core.health < MAX_HEALTH);
            if hurt {
                break;
            }
        }
        assert!(hurt, "the enderman hits the endermite");
        let fish_health = world.family_core(fish).map(|(_, core)| core.health);
        assert_eq!(
            fish_health,
            Some(silverfish::MAX_HEALTH),
            "not the silverfish"
        );
    }

    #[test]
    fn silverfish_and_endermites_give_five_and_three_experience() {
        let mut world = EntityWorld::default();
        let mut field = Field;
        let fish = world
            .spawn_family_kind(silverfish::KIND, at(0.5, 0.5))
            .unwrap();
        let prey = mite_at(&mut world, 4.5, 0.5);
        let burnt = mite_at(&mut world, 8.5, 0.5);
        world.tick(&mut field);
        for id in [fish, prey] {
            let near = world.body_of(id).unwrap().position - DVec3::Z;
            assert!(world.player_attack(&attack(1, near, 100.0), id).died);
        }
        let lava = Hurt::new(100.0, "minecraft:lava", DamageSourceKind::Hazard);
        assert!(world.hurt_family(burnt, &lava).unwrap().died);
        let deaths = world.take_deaths();
        let death = |id: u64| deaths.iter().find(|d| d.id == id).unwrap();
        assert_eq!(death(fish).table, Some(silverfish::KIND));
        assert_eq!(death(fish).experience, 5);
        assert_eq!(death(prey).table, Some(KIND));
        assert_eq!(death(prey).experience, EXPERIENCE);
        assert_eq!(death(burnt).table, Some(KIND));
        assert_eq!(death(burnt).experience, 0, "no player killed it");
    }
}
