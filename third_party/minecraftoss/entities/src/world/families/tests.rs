use super::pests::probe::{self, ANIMAL, KIND, Probe};
use super::*;
use crate::creeper::CreeperExplosion;
use crate::effects::{EffectInstance, MobEffect};
use crate::fireball::{Fireball, FireballKind, Shooter};
use crate::monster_ai::{MonsterKind, Target};
use crate::projectile::ArrowImpact;
use minecraftoss_player::{Block, Pos};
use std::collections::HashSet;

#[derive(Default)]
struct Field {
    water: Vec<Pos>,
    stone: Vec<Pos>,
}

impl World for Field {
    fn block(&self, pos: Pos) -> Option<Block> {
        if self.water.contains(&pos) {
            return Some(Block::new("minecraft:water"));
        }
        (pos.1 < 64 || self.stone.contains(&pos)).then(|| Block::new("minecraft:stone"))
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

fn probe_at(world: &mut EntityWorld, kind: &str, at: DVec3) -> u64 {
    world.spawn_family_kind(kind, at).unwrap()
}

fn probe(world: &EntityWorld, id: u64) -> &FamilyMob<Probe> {
    world
        .family::<Probe>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn probe_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Probe> {
    world.family_mob_mut::<Probe>(id).unwrap()
}

fn at(x: f64) -> DVec3 {
    DVec3::new(x, 64.0, 0.5)
}

#[test]
fn every_family_key_is_unique_and_inside_its_group_range() {
    let ranges = key_ranges();
    let names: Vec<_> = ranges.iter().map(|(name, _, _)| *name).collect();
    assert_eq!(
        names,
        [
            "pests",
            "trial",
            "deep_dark",
            "land",
            "water",
            "night",
            "bosses"
        ]
    );
    let mut seen = HashSet::new();
    for (index, (name, range, keys)) in ranges.iter().enumerate() {
        assert!(*range.start() >= 36, "{name} keeps clear of 0-35");
        for (other, next, _) in &ranges[index + 1..] {
            assert!(range.end() < next.start(), "{name} and {other} overlap");
        }
        for &key in keys {
            assert!(range.contains(&key), "{name}: {key}");
            assert!(seen.insert(key), "{key} is taken twice");
            assert!(has_key(key));
        }
    }
    assert!(seen.contains(&Probe::KEY));
    assert!(!has_key(0) && !has_key(35) && !has_key(100));
    let all = kinds();
    let unique: HashSet<_> = all.iter().collect();
    assert_eq!(unique.len(), all.len(), "no kind in two families");
    assert_eq!(key_of_kind(KIND), Some(Probe::KEY));
    assert_eq!(key_of_kind("minecraft:zombie"), None);
}

#[test]
fn a_registered_mob_is_seen_by_every_world_walk() {
    let mut world = EntityWorld::default();
    let id = probe_at(&mut world, KIND, at(0.5));
    let animal = probe_at(&mut world, ANIMAL, at(4.5));
    assert!(world.mob_ids().any(|m| m == id));
    assert_eq!(world.entity_type(id), Some(KIND));
    assert_eq!(world.entity_type(animal), Some(ANIMAL));
    let census = world.census();
    let entry = census.iter().find(|c| c.kind == KIND).unwrap();
    assert_eq!(
        (entry.width, entry.height, entry.persistent),
        (probe::WIDTH, probe::HEIGHT, false)
    );
    assert!(world.positions().any(|(m, p)| m == id && p == at(0.5)));
    let candidate = world
        .mob_candidates()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap();
    assert_eq!(
        (
            candidate.kind,
            candidate.eye_height,
            candidate.alive,
            candidate.baby
        ),
        (KIND, probe::EYE_HEIGHT, true, false)
    );
    assert!(world.body_mut(id).is_some());
    assert_eq!(world.body_of(id).map(|b| b.position), Some(at(0.5)));
    assert!(world.damage_state(id).is_some());
    assert!(world.mob_effects(id).is_some());
    let seen = world.seen_all(u64::MAX, &[]);
    assert!(
        seen.iter()
            .any(|s| s.id == id && s.kind == KIND && s.category == "monster")
    );
    assert!(
        seen.iter()
            .any(|s| s.id == animal && s.category == "creature")
    );
    let targets = world.arrow_targets(u64::MAX, &[]);
    let target = targets.iter().find(|t| t.id == id).unwrap();
    assert!((target.min.x - 0.2).abs() < 1e-6 && (target.max.y - 64.9).abs() < 1e-6);
    assert!(!world.can_be_leashed(id));
    let (hit, _) = world
        .mob_on_ray(DVec3::new(0.5, 64.5, -3.0), DVec3::Z, 10.0)
        .unwrap();
    assert_eq!(hit, MobHit::Family(Probe::KEY, id));
    assert_eq!(hit.id(), id);
    assert!(
        world
            .blast_target(EntityKey::Family(Probe::KEY, id))
            .is_some()
    );
    world.body_mut(id).unwrap().fire_ticks = 20;
    assert_eq!(world.burning().len(), 1);
    assert_eq!(world.family_cores().len(), 2);
    assert_eq!(world.family_core(id).map(|(key, _)| key), Some(Probe::KEY));
    assert!(crate::monster_ai::is_enemy(KIND));
    assert!(!crate::monster_ai::is_enemy(ANIMAL));
}

#[test]
fn a_registered_mob_takes_melee_arrows_blasts_magic_and_fire() {
    let mut world = EntityWorld::default();
    let id = probe_at(&mut world, KIND, at(0.5));
    world.body_mut(id).unwrap().on_ground = true;
    let hit = world.player_attack(&attack(1, DVec3::new(0.5, 64.0, -1.5), 3.0), id);
    assert!(hit.hurt && hit.full_hit && !hit.died);
    let mob = probe(&world, id);
    assert_eq!(mob.core.health, 7.0);
    assert_eq!(mob.core.hurt_by, Some((Target::Player(1), 0)));
    assert!(
        mob.core.body.velocity.z > 0.0,
        "knocked away from the player"
    );
    assert_eq!(
        mob.core.damage.player_memory,
        crate::health::PLAYER_MEMORY_TICKS
    );
    assert!(
        mob.core
            .voices
            .iter()
            .any(|(voice, _)| matches!(voice, Voice::Hurt(_)))
    );

    let shooter = 9_999;
    let plain = probe_at(&mut world, KIND, at(10.5));
    let half = probe_at(&mut world, KIND, at(20.5));
    let rock = probe_at(&mut world, KIND, at(30.5));
    probe_mut(&mut world, half).state.knockback_resistance = 0.5;
    probe_mut(&mut world, rock).state.knockback_resistance = 1.0;
    for target in [plain, half, rock] {
        let impact = ArrowImpact {
            target_id: target,
            damage: 4.0,
            velocity: DVec3::new(0.0, 0.0, 2.0),
        };
        assert!(world.apply_arrow_hit(impact, shooter).unwrap().applied);
    }
    let push = |world: &EntityWorld, id| probe(world, id).core.body.velocity.z;
    assert!((push(&world, plain) - 0.4).abs() < 1e-6);
    assert!((push(&world, half) - 0.2).abs() < 1e-6);
    assert_eq!(push(&world, rock), 0.0);
    assert_eq!(probe(&world, plain).core.health, 6.0);
    assert_eq!(
        probe(&world, plain).core.hurt_by,
        Some((Target::Mob(shooter), 0))
    );

    let blasted = probe_at(&mut world, KIND, at(40.5));
    let blast = CreeperExplosion {
        position: DVec3::new(40.5, 64.5, -1.0),
        radius: 3.0,
        powered: false,
    };
    world.blast(&Field::default(), blast, &[]);
    let mob = probe(&world, blasted);
    assert!(mob.core.health < probe::MAX_HEALTH);
    assert!(mob.core.body.velocity.z > 0.0, "pushed away from the blast");

    let living = probe_at(&mut world, KIND, at(50.5));
    let undead = probe_at(&mut world, KIND, at(60.5));
    probe_mut(&mut world, undead).state.undead = true;
    probe_mut(&mut world, undead).core.health = 5.0;
    for (effect, living_health, undead_health) in [
        (MobEffect::InstantDamage, 4.0, 9.0),
        (MobEffect::InstantHealth, 8.0, 3.0),
    ] {
        for id in [living, undead] {
            let spot = world.body_of(id).unwrap().position;
            world.splash_instant(id, effect, 0, 1.0, spot, None);
        }
        assert_eq!(probe(&world, living).core.health, living_health);
        assert_eq!(probe(&world, undead).core.health, undead_health);
    }
    let poison = || EffectInstance::new(MobEffect::Poison, 100, 0);
    assert!(world.add_mob_effect(living, poison()));
    let immune = probe_at(&mut world, KIND, at(70.5));
    probe_mut(&mut world, immune).state.immune_to_poison = true;
    assert!(!world.add_mob_effect(immune, poison()));
    assert!(!world.add_mob_effect(undead, poison()), "the undead too");

    let mut world = EntityWorld::default();
    let mut field = Field::default();
    let burning = probe_at(&mut world, KIND, at(0.5));
    let fireproof = probe_at(&mut world, KIND, at(10.5));
    probe_mut(&mut world, fireproof).state.fire_immune = true;
    let diver = probe_at(&mut world, KIND, at(20.5));
    for y in 64..=66 {
        field.water.push((20, y, 0));
    }
    world.body_mut(diver).unwrap().air = -19;
    let buried = probe_at(&mut world, KIND, DVec3::new(30.5, 70.0, 0.5));
    probe_mut(&mut world, buried).core.no_ai = true;
    field.stone.push((30, 70, 0));
    for id in [burning, fireproof] {
        world.body_mut(id).unwrap().fire_ticks = 40;
    }
    world.tick(&mut field);
    let health = |world: &EntityWorld, id| probe(world, id).core.health;
    assert_eq!(health(&world, burning), 9.0, "a second of fire");
    assert_eq!(probe(&world, burning).core.body.fire_ticks, 39);
    assert_eq!(health(&world, fireproof), probe::MAX_HEALTH);
    assert_eq!(probe(&world, fireproof).core.body.fire_ticks, 0);
    assert_eq!(health(&world, diver), 8.0, "it drowns");
    assert_eq!(probe(&world, diver).core.body.air, 0);
    assert_eq!(health(&world, buried), 9.0, "it suffocates");
}

#[test]
fn a_registered_mob_dies_with_its_table_experience_and_voices() {
    let mut world = EntityWorld::default();
    let monster = probe_at(&mut world, KIND, at(0.5));
    let animal = probe_at(&mut world, ANIMAL, at(10.5));
    let baby = probe_at(&mut world, ANIMAL, at(20.5));
    let hurt = probe_at(&mut world, KIND, at(30.5));
    probe_mut(&mut world, baby).core.baby = true;
    probe_mut(&mut world, monster).state.drops = vec![ItemStack::new("minecraft:stick", 2)];
    let hit = world.player_attack(&attack(1, at(30.5), 3.0), hurt);
    assert!(hit.hurt && !hit.died);
    for id in [animal, monster, baby] {
        let near = world.body_of(id).unwrap().position - DVec3::Z;
        assert!(world.player_attack(&attack(1, near, 100.0), id).died);
    }
    let deaths = world.take_deaths();
    assert_eq!(
        deaths.iter().map(|d| d.id).collect::<Vec<_>>(),
        [animal, monster, baby]
    );
    let killed = &deaths[1];
    assert_eq!(killed.table, Some(KIND));
    assert_eq!(killed.experience, 5);
    assert_eq!(killed.context.this_type, Some(KIND));
    assert_eq!(killed.context.cube_size, Some(probe::CUBE_SIZE));
    assert!(killed.context.killed_by_player);
    assert_eq!(killed.equipment, vec![ItemStack::new("minecraft:stick", 2)]);
    assert_eq!(deaths[0].table, Some(ANIMAL));
    assert!(
        (1..=3).contains(&deaths[0].experience),
        "{}",
        deaths[0].experience
    );
    assert_eq!((deaths[2].table, deaths[2].experience), (None, 0));
    assert!(deaths[2].context.baby);
    assert!(world.take_deaths().is_empty());
    let sounds = world.take_sounds();
    let heard = |event: &str, category: &str| {
        sounds
            .iter()
            .any(|s| s.event == event && s.category == category)
    };
    assert!(heard("entity.registry_probe.hurt", "hostile_volume"));
    assert!(heard("entity.registry_probe.death", "hostile_volume"));
    assert!(heard(
        "entity.registry_probe_animal.death",
        "neutral_volume"
    ));
}

#[test]
fn a_registered_mob_despawns_by_its_rule_leaves_with_its_chunk_and_is_copied_where_kept() {
    let mut world = EntityWorld::default();
    let monster = probe_at(&mut world, KIND, at(0.5));
    let kept = probe_at(&mut world, KIND, at(2.5));
    let animal = probe_at(&mut world, ANIMAL, at(4.5));
    probe_mut(&mut world, kept).core.persistent = true;
    let far = [DVec3::new(200.0, 64.0, 0.5)];
    assert_eq!(world.check_despawn(&far, false, &|_| true), 1);
    assert_eq!(world.entity_type(monster), None);
    assert!(!world.mob_ids().any(|id| id == monster));
    assert!(world.entity_type(kept).is_some() && world.entity_type(animal).is_some());
    let near = [DVec3::new(0.5, 64.0, 3.5)];
    assert_eq!(world.check_despawn(&near, true, &|_| true), 1);
    assert_eq!(world.entity_type(kept), None, "peaceful takes monsters");
    assert!(world.entity_type(animal).is_some());

    let unloaded = probe_at(&mut world, KIND, at(40.5));
    assert_eq!(world.remove_where(|p| p.x > 30.0), 1);
    assert_eq!(world.entity_type(unloaded), None);

    let behind = probe_at(&mut world, KIND, at(-20.5));
    let copy = world.clone_where(|p| p.x > 0.0);
    assert!(copy.entity_type(animal).is_some());
    assert_eq!(copy.entity_type(behind), None);
    assert_eq!(copy.mob_ids().collect::<Vec<_>>(), [animal]);

    let mut field = Field::default();
    assert!(
        world
            .player_attack(&attack(1, at(4.5) - DVec3::Z, 100.0), animal)
            .died
    );
    for tick in 1..=20 {
        world.tick(&mut field);
        assert_eq!(world.entity_type(animal).is_some(), tick < 20, "{tick}");
    }
    assert!(world.entity_type(behind).is_some());
}

#[test]
fn a_removed_registered_mob_leaves_without_a_death() {
    let mut world = EntityWorld::default();
    let idle = probe_at(&mut world, KIND, at(0.5));
    let killed = probe_at(&mut world, KIND, at(10.5));
    assert!(
        world
            .player_attack(&attack(1, at(10.5) - DVec3::Z, 100.0), killed)
            .died
    );
    for id in [idle, killed] {
        probe_mut(&mut world, id).core.removed = true;
    }
    assert!(world.take_deaths().is_empty(), "no loot and no experience");
    world.tick(&mut Field::default());
    assert_eq!(world.entity_type(idle), None);
    assert_eq!(world.entity_type(killed), None);
    assert_eq!(world.mob_ids().count(), 0);
    assert!(world.take_deaths().is_empty());
}

#[test]
fn a_lead_pulls_a_leashable_registered_mob() {
    let mut world = EntityWorld::default();
    let mut field = Field::default();
    let dog = probe_at(&mut world, KIND, at(3.5));
    let wild = probe_at(&mut world, KIND, DVec3::new(3.5, 64.0, 4.5));
    probe_mut(&mut world, dog).state.leashable = true;
    probe_mut(&mut world, dog).core.yaw = 90.0;
    assert!(world.can_be_leashed(dog));
    assert!(!world.can_be_leashed(wild));
    let player = player_at(0, at(0.5));
    world.set_leash_players(&[player]);
    assert_eq!(
        world.leash_interact(wild, 0, Some(LEAD), false, false),
        LeashUse::Pass
    );
    assert_eq!(
        world.leash_interact(dog, 0, Some(LEAD), false, false),
        LeashUse::Tied
    );
    assert_eq!(world.leash_yaw(dog), 90.0);
    world.tick_with_players(&mut field, &[player]);
    let walks = probe(&world, dog).state.walks.clone();
    assert_eq!(walks.len(), 1, "walked towards its holder");
    assert_eq!(walks[0].1, 1.0);
    assert!(walks[0].0.x < 3.5);
    probe_mut(&mut world, dog).state.panicking = true;
    world.tick_with_players(&mut field, &[player]);
    assert_eq!(
        probe(&world, dog).state.walks.len(),
        1,
        "not while panicking"
    );
    probe_mut(&mut world, dog).state.panicking = false;
    probe_mut(&mut world, dog).core.no_ai = true;
    world.tick_with_players(&mut field, &[player]);
    assert_eq!(probe(&world, dog).state.walks.len(), 1, "not without AI");

    let pup = probe_at(&mut world, KIND, DVec3::new(3.5, 64.0, 2.5));
    probe_mut(&mut world, pup).state.leashable = true;
    probe_mut(&mut world, pup).core.baby = true;
    assert!(world.mob_baby(pup));
    assert_eq!(
        world.leash_interact(pup, 0, None, true, false),
        LeashUse::Pass,
        "a baby takes no lead from its player"
    );
    let adult = probe_at(&mut world, KIND, DVec3::new(2.5, 64.0, 2.5));
    probe_mut(&mut world, adult).state.leashable = true;
    assert_eq!(
        world.leash_interact(adult, 0, None, true, false),
        LeashUse::Moved
    );
    assert_eq!(world.leash_holder(dog), Some(LeashHolder::Mob(adult)));
}

#[test]
fn a_fireball_spares_a_fire_immune_registered_mob() {
    let mut world = EntityWorld::default();
    let mut field = Field::default();
    let blaze = world.spawn_blaze(crate::blaze::Blaze::new(DVec3::new(0.5, 64.0, -40.5)), true);
    let plain = probe_at(&mut world, KIND, at(0.5));
    let fireproof = probe_at(&mut world, KIND, at(10.5));
    probe_mut(&mut world, fireproof).state.fire_immune = true;
    for x in [0.5, 10.5] {
        world.spawn_fireball(Fireball::shoot(
            FireballKind::Small,
            Shooter::Mob(blaze),
            DVec3::new(x, 64.5, -1.5),
            DVec3::Z,
        ));
    }
    for _ in 0..20 {
        world.tick(&mut field);
    }
    let plain = probe(&world, plain);
    assert!(
        plain.core.health < probe::MAX_HEALTH && plain.core.body.fire_ticks > 0,
        "{} {}",
        plain.core.health,
        plain.core.body.fire_ticks
    );
    let fireproof = probe(&world, fireproof);
    assert_eq!(
        (fireproof.core.health, fireproof.core.body.fire_ticks),
        (probe::MAX_HEALTH, 0)
    );
}

#[test]
fn two_registered_mobs_push_apart_and_cram() {
    let mut world = EntityWorld::default();
    let mut field = Field::default();
    let left = probe_at(&mut world, KIND, at(0.4));
    let right = probe_at(&mut world, KIND, at(0.6));
    let ghost = probe_at(&mut world, KIND, at(0.5));
    probe_mut(&mut world, ghost).state.unpushable = true;
    let around = world
        .sections
        .living_around(DVec3::new(0.0, 64.0, 0.0), DVec3::new(1.0, 65.0, 1.0));
    assert!(around.contains(&left) && around.contains(&right));
    for _ in 0..3 {
        world.tick(&mut field);
    }
    let x = |world: &EntityWorld, id| probe(world, id).core.body.position.x;
    assert!(
        x(&world, left) < 0.4 && x(&world, right) > 0.6,
        "{} {}",
        x(&world, left),
        x(&world, right)
    );
    assert_eq!(x(&world, ghost), 0.5, "an unpushable mob stays put");

    let mut world = EntityWorld::default();
    world.max_entity_cramming = 2;
    let crowd: Vec<u64> = (0..3)
        .map(|i| probe_at(&mut world, KIND, at(0.5 + f64::from(i) * 0.01)))
        .collect();
    for &id in &crowd {
        probe_mut(&mut world, id).core.no_ai = true;
    }
    for _ in 0..40 {
        world.tick(&mut field);
    }
    assert!(
        crowd.iter().any(|&id| world.entity_type(id).is_none()
            || probe(&world, id).core.health < probe::MAX_HEALTH),
        "a crowd crams"
    );
}

#[test]
fn a_hurt_registered_monster_turns_on_its_attacker() {
    let mut world = EntityWorld::default();
    let mut field = Field::default();
    let id = probe_at(&mut world, KIND, at(0.5));
    probe_mut(&mut world, id).state.goals = true;
    let player = player_at(1, at(8.5));
    world.tick_with_players(&mut field, &[player]);
    assert_eq!(probe(&world, id).state.ai.state.target, None);
    let hit = world.player_attack(&attack(1, player.position, 1.0), id);
    assert!(hit.hurt);
    let stamp = probe(&world, id).core.tick_count;
    assert_eq!(
        probe(&world, id).core.hurt_by,
        Some((Target::Player(1), stamp))
    );
    for _ in 0..20 {
        world.tick_with_players(&mut field, &[player]);
    }
    let mob = probe(&world, id);
    assert_eq!(mob.state.ai.state.target, Some(Target::Player(1)));
    assert_eq!(mob.core.hurt_by, Some((Target::Player(1), stamp)));
    let east = |yaw: f32| ((yaw + 180.0).rem_euclid(360.0) - 180.0 + 90.0).abs() < 45.0;
    assert!(east(mob.core.yaw), "it faces the player: {}", mob.core.yaw);
    assert_eq!(mob.core.yaw, mob.state.ai.yaw);
    assert_ne!(mob.core.look.head_yaw, 0.0, "its head turned");
    assert_eq!(
        mob.core.look.head_yaw,
        mob.state.ai.state.look_control.head_yaw
    );
}

#[test]
fn an_iron_golem_hunts_an_enemy_registered_mob() {
    for (kind, hunted) in [(KIND, true), (ANIMAL, false)] {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let prey = probe_at(&mut world, kind, at(0.5));
        probe_mut(&mut world, prey).core.no_ai = true;
        let golem = world.spawn_iron_golem(
            crate::iron_golem::IronGolem::new(DVec3::new(0.5, 64.0, 8.5)),
            false,
        );
        let mut target = None;
        for _ in 0..40 {
            world.tick(&mut field);
            target = world
                .iron_golems()
                .iter()
                .find(|e| e.id == golem)
                .unwrap()
                .ai
                .state
                .target;
            if target.is_some() {
                break;
            }
        }
        assert_eq!(target, hunted.then_some(Target::Mob(prey)), "{kind}");
    }
}

#[test]
fn a_registered_monster_runs_its_goals_and_strikes_a_player() {
    let registered = MonsterKind::Registered(KIND);
    assert_eq!(registered.goal_names(), probe::GOAL_NAMES);
    assert_eq!(registered.target_goal_names(), probe::TARGET_NAMES);
    assert!(
        MonsterKind::Registered("minecraft:unknown")
            .goal_names()
            .is_empty()
    );
    let mut world = EntityWorld::default();
    let mut field = Field::default();
    let hunter = probe_at(&mut world, KIND, at(0.5));
    let idle = probe_at(&mut world, KIND, DVec3::new(0.5, 64.0, 20.5));
    let kept = probe_at(&mut world, KIND, DVec3::new(0.5, 64.0, -20.5));
    let ai = &probe(&world, hunter).state.ai;
    assert_eq!(ai.state.kind, registered);
    assert!(ai.state.walk.can_float, "its FloatGoal floats the path");
    assert_eq!(ai.state.walk.max_up_step, probe::STEP);
    assert_eq!(ai.state.follow_range, probe::FOLLOW_RANGE);
    assert_eq!(ai.state.walk.follow_range, probe::FOLLOW_RANGE as f32);
    assert_eq!(ai.state.eye_height, probe::EYE_HEIGHT);
    for id in [hunter, idle, kept] {
        probe_mut(&mut world, id).state.goals = true;
    }
    for id in [hunter, kept] {
        probe_mut(&mut world, id).state.goal_allowed = true;
    }
    probe_mut(&mut world, kept)
        .state
        .ai
        .retain_goals(&["ProbeGoal"]);
    let players: Vec<PlayerCandidate> = [hunter, idle, kept]
        .iter()
        .enumerate()
        .map(|(index, &id)| {
            let spot = probe(&world, id).core.body.position + DVec3::new(2.0, 0.0, 0.0);
            player_at(index as u64 + 1, spot)
        })
        .collect();
    world.tick_with_players(&mut field, &players);
    for (player, id) in players.iter().zip([hunter, idle, kept]) {
        assert!(
            world
                .player_attack(&attack(player.id, player.position, 1.0), id)
                .hurt
        );
    }
    let mut hits = Vec::new();
    let mut running = HashSet::new();
    for _ in 0..100 {
        world.tick_with_players(&mut field, &players);
        hits.extend(world.take_player_hits());
        running.extend(probe(&world, hunter).state.ai.running_goals());
        let kept_goals = probe(&world, kept).state.ai.running_goals();
        assert!(kept_goals.iter().all(|name| *name == "ProbeGoal"));
    }
    assert!(
        running.contains("MeleeAttackGoal") && running.contains("ProbeGoal"),
        "{running:?}"
    );
    assert_eq!(
        probe(&world, hunter).state.ai.running_targets(),
        ["HurtByTargetGoal"]
    );
    assert!(hits.iter().all(|hit| hit.source != Some(kept)));
    let hits: Vec<_> = hits
        .into_iter()
        .filter(|hit| hit.source == Some(hunter))
        .collect();
    assert!(!hits.is_empty(), "it bites its attacker");
    for hit in &hits {
        assert_eq!(hit.player_id, 1);
        assert_eq!(hit.damage, probe::DAMAGE);
        let PlayerHitKind::Melee { attacker, .. } = hit.kind else {
            panic!("{:?}", hit.kind);
        };
        assert!(attacker.distance(players[0].position) < 2.0, "{attacker}");
    }
    assert_eq!(probe(&world, hunter).core.swings as usize, hits.len());
    assert!(probe(&world, hunter).state.goal_ticks > 0, "the goal ran");
    assert_eq!(probe(&world, idle).state.goal_ticks, 0, "never let in");
    assert!(
        probe(&world, kept).state.goal_ticks > 0,
        "the kept goal ran"
    );
    assert_eq!(
        probe(&world, kept).state.ai.state.target,
        Some(Target::Player(3)),
        "its targets stay"
    );
}

#[test]
fn prey_kinds_targets_the_named_kinds_only() {
    let mut world = EntityWorld::default();
    let mut field = Field::default();
    let hunter = probe_at(&mut world, KIND, at(0.5));
    probe_mut(&mut world, hunter).state.goals = true;
    let other = probe_at(&mut world, KIND, at(3.5));
    probe_mut(&mut world, other).core.no_ai = true;
    world.spawn_zombie(crate::zombie::Zombie::new(at(-3.5)), true);
    for _ in 0..40 {
        world.tick(&mut field);
    }
    assert_eq!(probe(&world, hunter).state.ai.state.target, None);
    let prey = probe_at(&mut world, ANIMAL, DVec3::new(0.5, 64.0, 3.5));
    probe_mut(&mut world, prey).core.no_ai = true;
    let mut target = None;
    for _ in 0..40 {
        world.tick(&mut field);
        target = probe(&world, hunter).state.ai.state.target;
        if target.is_some() {
            break;
        }
    }
    assert_eq!(target, Some(Target::Mob(prey)));
    for _ in 0..30 {
        world.tick(&mut field);
    }
    assert!(
        world
            .family_core(prey)
            .is_none_or(|(_, core)| core.health < probe::MAX_HEALTH),
        "and bites it"
    );
}

#[test]
fn a_registered_mount_seats_a_rider_and_lets_it_off() {
    let mut world = EntityWorld::default();
    let field = Field::default();
    let mount = probe_at(&mut world, KIND, at(0.5));
    let plain = probe_at(&mut world, KIND, at(10.5));
    let rider = probe_at(&mut world, KIND, at(3.5));
    let third = probe_at(&mut world, KIND, at(6.5));
    assert!(is_vehicle(KIND), "saved jockeys see it before it lives");
    assert!(!world.carries_riders(mount), "no seats yet");
    assert!(!world.start_riding(Rider::Mob(rider), plain, &field));
    probe_mut(&mut world, mount).state.seats = 2;
    probe_mut(&mut world, rider).state.attachment = 0.25;
    assert!(world.carries_riders(mount));
    assert!(world.start_riding(Rider::Mob(rider), mount, &field));
    assert!(world.start_riding(Rider::Player(0), mount, &field));
    assert!(
        !world.start_riding(Rider::Mob(third), mount, &field),
        "two seats"
    );
    assert_eq!(
        world.passengers_of(mount),
        [Rider::Mob(rider), Rider::Player(0)]
    );
    let base = at(0.5);
    let height = f64::from(probe::HEIGHT);
    assert_eq!(
        world.passenger_point(mount),
        Some(DVec3::new(0.0, height, 0.0))
    );
    let seat = world.seat(Rider::Mob(rider)).unwrap();
    let wanted = base + DVec3::new(0.0, height - 0.25, 0.0);
    assert!((seat - wanted).length() < 1e-9, "{seat}");
    assert_eq!(probe(&world, rider).core.body.position, seat, "seated");
    let player_seat = world.seat(Rider::Player(0)).unwrap();
    let lift = crate::world::riding::PLAYER_VEHICLE_ATTACHMENT;
    let wanted = base + DVec3::new(0.0, height - lift, -probe::SEAT_SPACING);
    assert!((player_seat - wanted).length() < 1e-9, "{player_seat}");
    let spot = world.stop_riding(Rider::Mob(rider), 0.0, &field).unwrap();
    assert_eq!(spot, base + DVec3::new(0.0, height, 0.0), "on top of it");
    assert_eq!(probe(&world, rider).core.body.position, spot);
    assert_eq!(world.vehicle_of(Rider::Mob(rider)), None);
    assert!(
        world.start_riding(Rider::Mob(third), mount, &field),
        "a seat is free again"
    );
}

#[test]
fn a_player_steers_and_jumps_a_registered_mount() {
    let mut world = EntityWorld::default();
    let mut field = Field::default();
    let mount = probe_at(&mut world, KIND, at(0.5));
    probe_mut(&mut world, mount).state.seats = 1;
    assert!(world.start_riding(Rider::Player(0), mount, &field));
    assert_eq!(world.controlling_player(mount), None, "not saddled");
    probe_mut(&mut world, mount).state.saddled = true;
    assert_eq!(world.controlling_player(mount), Some(0));
    assert_eq!(world.controlling_mob(mount), None);
    assert_eq!(world.ride_jump_scale(0), None, "no jump bar");
    probe_mut(&mut world, mount).state.jumps = true;
    assert_eq!(world.ride_jump_scale(0), Some(0.0));
    let players = [player_at(0, world.seat(Rider::Player(0)).unwrap())];
    let steer = |world: &mut EntityWorld, jumping| {
        world.set_player_controls(&[crate::controls::PlayerControls {
            id: 0,
            jumping,
            ..Default::default()
        }]);
    };
    steer(&mut world, true);
    for _ in 0..5 {
        world.tick_with_players(&mut field, &players);
    }
    assert!((world.ride_jump_scale(0).unwrap() - 0.4).abs() < 1e-6);
    assert!(probe(&world, mount).state.jumped.is_empty());
    steer(&mut world, false);
    world.tick_with_players(&mut field, &players);
    assert_eq!(probe(&world, mount).state.jumped, [40], "on release");

    assert!(world.dismount_player(0, 0.0, &field).is_some());
    let zombie = world.spawn_zombie(crate::zombie::Zombie::new(at(3.5)), true);
    assert!(world.start_riding(Rider::Mob(zombie), mount, &field));
    assert_eq!(world.controlling_mob(mount), None, "it does not steer yet");
    probe_mut(&mut world, mount).state.mob_steers = true;
    assert_eq!(world.controlling_mob(mount), Some(zombie));
    assert_eq!(world.controlling_player(mount), None);
    let tick = world.begin_ride_tick(zombie).unwrap();
    let navigation = &mut world.zombie_mut(zombie).unwrap().navigation;
    navigation.target_pos = Some((8, 64, 0));
    navigation.speed_modifier = 1.25;
    world.end_ride_tick(tick, &field);
    assert_eq!(
        probe(&world, mount).state.orders,
        [(DVec3::new(8.0, 64.0, 0.0), 1.25)]
    );
}
