use super::ai::{self, PolarBearGoals};
use super::*;
use crate::monster_ai::{MobCandidate, Target};
use crate::walk_path::WalkTarget;
use crate::world::{DamageSourceKind, PlayerAttack};
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Block, Pos};

struct Tundra;

impl World for Tundra {
    fn block(&self, pos: Pos) -> Option<Block> {
        (pos.1 < 64).then(|| Block::new("minecraft:grass_block"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}
}

fn at(x: f64, z: f64) -> DVec3 {
    DVec3::new(x, 64.0, z)
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

fn bear_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    world.spawn_family_kind(KIND, position).unwrap()
}

fn cub_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    let mut mob = PolarBear::spawn(KIND, position).unwrap();
    mob.state.age.set(Age::BABY_START);
    set_baby(&mut mob, true);
    world.spawn_family(mob)
}

fn bear(world: &EntityWorld, id: u64) -> &FamilyMob<PolarBear> {
    world
        .family::<PolarBear>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn bear_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<PolarBear> {
    world.family_mob_mut::<PolarBear>(id).unwrap()
}

fn target(world: &EntityWorld, id: u64) -> Option<Target> {
    bear(world, id).state.ai.state.target
}

#[test]
fn a_polar_bear_is_large_with_thirty_health_and_its_kind() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let adult = bear_at(&mut world, at(0.5, 0.5));
    let cub = cub_at(&mut world, at(6.5, 0.5));
    assert_eq!(world.entity_type(adult), Some(KIND));
    assert_eq!(world.family_core(adult).map(|(key, _)| key), Some(55));
    let core = &bear(&world, adult).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (1.4, 1.4, 1.4 * 0.85, false)
    );
    assert!((core.eye_height - 1.19).abs() < 1.0e-6);
    assert_eq!((core.health, core.max_health), (30.0, 30.0));
    assert_eq!(core.ambient_interval, 120);
    let core = &bear(&world, cub).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (0.7, 0.7, 0.34375, true)
    );
    let census = world.census();
    assert_eq!(census.iter().filter(|c| c.kind == KIND).count(), 2);
    let candidates = world.mob_candidates();
    assert!(
        candidates
            .iter()
            .any(|m| m.id == adult && m.kind == KIND && !m.baby)
    );
    assert!(
        candidates
            .iter()
            .any(|m| m.id == cub && m.kind == KIND && m.baby)
    );
    assert!(!crate::monster_ai::is_enemy(KIND));
    let traits = world.family_traits(adult).unwrap();
    assert_eq!(traits.category, "creature");
    assert!(traits.leashable && !traits.enemy && !traits.undead);
    assert!(!traits.fire_immune && !traits.breathes_water && !traits.panicking);
    assert_eq!(traits.voice, ("polar_bear", 1.0, "neutral_volume"));
    assert_eq!(PolarBear::step_sound(KIND), Some(STEP_SOUND));
    let registered = MonsterKind::Registered(KIND);
    assert_eq!(registered.goal_names(), GOAL_NAMES);
    assert_eq!(registered.target_goal_names(), TARGET_NAMES);
    let ai = &bear(&world, adult).state.ai;
    assert_eq!(ai.state.follow_range, FOLLOW_RANGE);
    assert_eq!(ai.state.eye_height, EYE_HEIGHT);
    assert!(ai.state.walk.can_float, "its FloatGoal floats the path");
    assert_eq!(ai.state.walk.walk_target, WalkTarget::Animal);
    let player = player_at(1, at(3.5, 0.5));
    for id in [adult, cub] {
        assert!(
            world
                .player_attack(&attack(1, player.position, 100.0), id)
                .died
        );
    }
    let deaths = world.take_deaths();
    let adult = deaths.iter().find(|d| d.id == adult).unwrap();
    assert_eq!(adult.table, Some(KIND));
    assert!((1..=3).contains(&adult.experience), "{}", adult.experience);
    let cub = deaths.iter().find(|d| d.id == cub).unwrap();
    assert_eq!((cub.table, cub.experience), (None, 0), "cubs drop nothing");
}

fn melee_alone(x: f64) -> Vec<(bool, bool, bool)> {
    let mob = PolarBear::spawn(KIND, at(0.5, 0.5)).unwrap();
    let mut ai = (*mob.state.ai).clone();
    ai.retain_goals(&["PolarBearMeleeAttackGoal"]);
    ai.state.body.on_ground = true;
    ai.state.game_time = 100;
    ai.state.players = vec![player_at(1, at(x, 0.5))];
    ai.state.target = Some(Target::Player(1));
    let mut ticks = Vec::new();
    for _ in 0..60 {
        ai.tick_goals(&Tundra, true);
        let hit = ai.state.attack.take().is_some();
        let goals = ai.state.extra.get_mut::<PolarBearGoals>().unwrap();
        ticks.push((hit, goals.standing, goals.warn.take().is_some()));
    }
    ticks
}

#[test]
fn a_hit_bear_stands_warns_every_two_seconds_and_strikes_for_six() {
    let close = melee_alone(2.0);
    let hits: Vec<usize> = (0..close.len()).filter(|&t| close[t].0).collect();
    assert!(hits.len() >= 3, "{hits:?}");
    for pair in hits.windows(2) {
        assert_eq!(pair[1] - pair[0], 20, "a swipe a second: {hits:?}");
        for tick in pair[0]..pair[1] {
            let stands = tick >= pair[1] - 10;
            assert_eq!(close[tick].1, stands, "tick {tick}: {hits:?}");
        }
    }
    for &tick in &hits {
        assert!(!close[tick].1, "it drops onto its paws to swipe");
    }
    let warned: Vec<usize> = (0..close.len()).filter(|&t| close[t].2).collect();
    assert_eq!(warned, vec![hits[0] + 10], "it growls as it first rears up");

    let near = melee_alone(3.5);
    assert!(
        near.iter().all(|tick| !tick.0),
        "out of reach it never swipes"
    );
    let standing = near.iter().filter(|tick| tick.1).count();
    assert!(
        (25..=30).contains(&standing),
        "half the time up: {standing}"
    );
    assert_eq!(near.iter().filter(|tick| tick.2).count(), 1);
    let far = melee_alone(5.5);
    assert!(far.iter().all(|&tick| tick == (false, false, false)));

    let mut world = EntityWorld::default();
    let mut tundra = Tundra;
    let id = bear_at(&mut world, at(0.5, 0.5));
    let player = player_at(1, at(6.5, 0.5));
    world.tick_with_players(&mut tundra, &[player]);
    assert!(
        world
            .player_attack(&attack(1, player.position, 1.0), id)
            .hurt
    );
    let (mut hits, mut warnings, mut stood) = (Vec::new(), Vec::new(), Vec::new());
    let mut stands = Vec::new();
    for tick in 0..240 {
        world.tick_with_players(&mut tundra, &[player]);
        for hit in world.take_player_hits() {
            hits.push((tick, hit));
        }
        for sound in world.take_sounds() {
            if sound.event == WARNING_SOUND {
                assert_eq!(sound.category, "neutral_volume");
                assert!((0.8..=1.2).contains(&sound.pitch), "{}", sound.pitch);
                warnings.push(tick);
            }
        }
        let mob = bear(&world, id);
        if mob.state.standing() {
            stood.push(tick);
        }
        stands.push((mob.state.stand_old, mob.state.stand));
    }
    assert!(hits.len() >= 4, "{}", hits.len());
    for (_, hit) in &hits {
        assert_eq!((hit.player_id, hit.damage, hit.source), (1, 6.0, Some(id)));
        assert!(
            matches!(hit.kind, PlayerHitKind::Melee { .. }),
            "{:?}",
            hit.kind
        );
    }
    for pair in hits.windows(2) {
        assert!(pair[1].0 - pair[0].0 >= 20, "a swipe a second");
    }
    for (tick, _) in &hits {
        assert!(!stood.contains(tick), "it drops onto its paws to swipe");
        assert!(stood.contains(&(tick - 1)), "after standing up");
    }
    assert!(warnings.len() >= 3, "{warnings:?}");
    for pair in warnings.windows(2) {
        assert!(pair[1] - pair[0] >= WARNING_TICKS, "{warnings:?}");
    }
    assert!(
        warnings.windows(2).any(|pair| pair[1] - pair[0] == 40),
        "every two seconds while it keeps striking: {warnings:?}"
    );
    assert!(stands.contains(&(5, 6)), "the stand clock reaches six");
    for &(old, now) in &stands {
        assert!((0..=6).contains(&now) && (old - now).abs() <= 1);
    }
}

#[test]
fn a_hit_cub_runs_and_nearby_adults_attack() {
    let mut world = EntityWorld::default();
    let mut tundra = Tundra;
    let cub = cub_at(&mut world, at(0.5, 0.5));
    let near = bear_at(&mut world, at(6.5, 0.5));
    let far = bear_at(&mut world, at(0.5, 30.5));
    let sibling = cub_at(&mut world, at(-4.5, 0.5));
    let player = player_at(1, at(6.5, 16.5));
    for _ in 0..3 {
        world.tick_with_players(&mut tundra, &[player]);
    }
    let start = bear(&world, cub).core.body.position;
    let hit = Hurt::new(
        1.0,
        "minecraft:player_attack",
        DamageSourceKind::PlayerAttack,
    )
    .by(Some(Target::Player(1)), Some("minecraft:player"));
    assert!(world.hurt_family(cub, &hit).unwrap().applied);
    let mut panicked = false;
    for _ in 0..4 {
        world.tick_with_players(&mut tundra, &[player]);
        panicked |= bear(&world, cub).state.panicking();
    }
    assert_eq!(
        target(&world, near),
        Some(Target::Player(1)),
        "the cub calls it"
    );
    assert_eq!(target(&world, far), None, "too far to hear the cub");
    assert_eq!(target(&world, sibling), None, "cubs are not called");
    assert_eq!(target(&world, cub), None, "the cub drops its own target");
    for _ in 0..40 {
        world.tick_with_players(&mut tundra, &[player]);
        panicked |= bear(&world, cub).state.panicking();
    }
    assert!(panicked, "the hit cub runs");
    let ran = bear(&world, cub).core.body.position.distance(start);
    assert!(ran > 3.0, "{ran}");
    assert_eq!(target(&world, near), Some(Target::Player(1)));
    assert_eq!(
        bear(&world, near).state.anger_target(),
        Some(Target::Player(1))
    );
}

fn hunts_player(cub: Option<DVec3>, player: DVec3) -> (bool, bool) {
    let mut world = EntityWorld::default();
    let mut tundra = Tundra;
    let adult = bear_at(&mut world, at(0.5, 0.5));
    let cub = cub.map(|position| {
        let id = cub_at(&mut world, position);
        bear_mut(&mut world, id).core.no_ai = true;
        id
    });
    let player = player_at(1, player);
    let (mut hunted, mut by_goal) = (false, false);
    for _ in 0..200 {
        world.tick_with_players(&mut tundra, &[player]);
        hunted |= target(&world, adult) == Some(Target::Player(1));
        by_goal |= bear(&world, adult)
            .state
            .ai
            .running_targets()
            .contains(&"PolarBearAttackPlayersGoal");
    }
    if let Some(cub) = cub {
        assert_eq!(target(&world, cub), None);
    }
    (hunted, by_goal)
}

#[test]
fn an_adult_with_a_cub_near_attacks_players_within_ten() {
    assert_eq!(
        hunts_player(Some(at(4.5, 0.5)), at(0.5, 8.5)),
        (true, true),
        "a cub 4 blocks off and a player 8 blocks off"
    );
    assert_eq!(
        hunts_player(Some(at(4.5, 0.5)), at(0.5, 11.5)),
        (false, false),
        "a player 11 blocks off"
    );
    assert_eq!(
        hunts_player(Some(at(10.5, 0.5)), at(0.5, 8.5)),
        (false, false),
        "a cub 10 blocks off"
    );
    assert_eq!(hunts_player(None, at(0.5, 8.5)), (false, false), "no cub");
    let mut world = EntityWorld::default();
    let mut tundra = Tundra;
    let cub = cub_at(&mut world, at(0.5, 0.5));
    let other = cub_at(&mut world, at(3.5, 0.5));
    let player = player_at(1, at(0.5, 5.5));
    for _ in 0..200 {
        world.tick_with_players(&mut tundra, &[player]);
        for id in [cub, other] {
            assert_eq!(target(&world, id), None, "a cub never goes for players");
        }
    }
}

#[test]
fn an_adult_hunts_foxes_and_a_cub_does_not() {
    for baby in [false, true] {
        let mut mob = PolarBear::spawn(KIND, at(0.5, 0.5)).unwrap();
        if baby {
            set_baby(&mut mob, true);
        }
        let mut ai = (*mob.state.ai).clone();
        ai.state.extra.get_mut::<PolarBearGoals>().unwrap().baby = baby;
        ai.state.body.on_ground = true;
        ai.state.game_time = 100;
        let candidate = |id: u64, kind: &'static str, x: f64| MobCandidate {
            id,
            position: at(x, 0.5),
            eye_height: 0.4,
            width: 0.6,
            height: 0.7,
            alive: true,
            kind,
            baby: false,
        };
        ai.state.mobs = vec![
            candidate(7, "minecraft:rabbit", 3.5),
            candidate(8, ai::FOX, 6.5),
        ];
        let mut hunted = None;
        for _ in 0..100 {
            ai.state.game_time += 1;
            ai.tick_goals(&Tundra, true);
            hunted = hunted.or(ai.state.target);
        }
        assert_eq!(hunted, (!baby).then_some(Target::Mob(8)), "baby {baby}");
        if !baby {
            assert!(ai.running_goals().contains(&"PolarBearMeleeAttackGoal"));
        }
    }
}

fn angered(seed: u64) -> (EntityWorld, u64, i64) {
    let mut world = EntityWorld::default();
    let mut tundra = Tundra;
    let id = bear_at(&mut world, at(0.5, 0.5));
    bear_mut(&mut world, id).core.random = LegacyRandom::new(seed);
    world.set_player_uuid(1, 0x1234);
    let player = player_at(1, at(3.5, 0.5));
    world.tick_with_players(&mut tundra, &[player]);
    assert!(
        world
            .player_attack(&attack(1, player.position, 1.0), id)
            .hurt
    );
    let mut now = 1;
    for _ in 0..4 {
        world.tick_with_players(&mut tundra, &[player]);
        now += 1;
        if bear(&world, id).state.anger_target().is_some() {
            break;
        }
    }
    (world, id, now)
}

#[test]
fn its_anger_lasts_twenty_to_thirty_nine_seconds_and_ends_for_creative_players() {
    let mut lengths = Vec::new();
    for seed in 0..60 {
        let (world, id, now) = angered(seed);
        let mob = bear(&world, id);
        assert_eq!(mob.state.anger_target(), Some(Target::Player(1)));
        assert_eq!(mob.state.angry_at, Some(0x1234), "saved as its UUID");
        lengths.push(mob.state.anger_end_time() - now);
    }
    assert!(
        lengths.iter().all(|length| (400..=780).contains(length)),
        "{lengths:?}"
    );
    assert!(lengths.iter().any(|&length| length < 450), "{lengths:?}");
    assert!(lengths.iter().any(|&length| length > 730), "{lengths:?}");

    let (mut world, id, _) = angered(1);
    let creative = PlayerCandidate {
        attackable: false,
        ..player_at(1, at(3.5, 0.5))
    };
    world.tick_with_players(&mut Tundra, &[creative]);
    let mob = bear(&world, id);
    assert_eq!(mob.state.anger_target(), None, "creative ends it");
    assert_eq!(mob.state.anger_end_time(), -1);
    assert_eq!((target(&world, id), mob.state.angry_at), (None, None));

    let (mut world, id, _) = angered(2);
    let gone = player_at(1, at(40.5, 0.5));
    let end = bear(&world, id).state.anger_end_time();
    let mut calm = None;
    for tick in 0..1000 {
        world.tick_with_players(&mut Tundra, &[gone]);
        let mob = bear(&world, id);
        if mob.state.anger_target().is_none() {
            calm = Some(tick);
            break;
        }
        assert_eq!(mob.state.angry_at, Some(0x1234));
    }
    let calm = calm.expect("it calms down");
    assert!(
        calm > 300 && calm <= 781,
        "{calm} ticks, the anger ran to {end}"
    );
    assert_eq!(target(&world, id), None);
    assert_eq!(bear(&world, id).state.angry_at, None);
}

fn panics(baby: bool, damage_type: &'static str, source: DamageSourceKind) -> bool {
    let mut world = EntityWorld::default();
    let mut tundra = Tundra;
    let id = if baby {
        cub_at(&mut world, at(0.5, 0.5))
    } else {
        bear_at(&mut world, at(0.5, 0.5))
    };
    for _ in 0..3 {
        world.tick(&mut tundra);
    }
    let hurt = Hurt::new(1.0, damage_type, source);
    assert!(world.hurt_family(id, &hurt).unwrap().applied);
    let mut panicked = false;
    for _ in 0..40 {
        world.tick(&mut tundra);
        let mob = bear(&world, id);
        if mob.state.panicking() {
            panicked = true;
            assert!(world.family_traits(id).unwrap().panicking);
            let speed = mob.state.ai.state.navigation.speed_modifier;
            assert_eq!(speed, ai::PANIC_SPEED);
        }
    }
    panicked
}

#[test]
fn an_adult_panics_only_from_fire_and_a_cub_from_any_hit() {
    use DamageSourceKind::{Generic, Hazard, MobAttack, PlayerAttack, Projectile};
    assert!(!panics(false, "minecraft:player_attack", PlayerAttack));
    assert!(!panics(false, "minecraft:mob_attack", MobAttack));
    assert!(!panics(false, "minecraft:arrow", Projectile));
    assert!(panics(false, "minecraft:in_fire", Hazard));
    assert!(panics(false, "minecraft:on_fire", Hazard));
    assert!(panics(false, "minecraft:lava", Hazard));
    assert!(panics(true, "minecraft:player_attack", PlayerAttack));
    assert!(panics(true, "minecraft:mob_attack", MobAttack));
    assert!(panics(true, "minecraft:arrow", Projectile));
    assert!(panics(true, "minecraft:in_fire", Hazard));
    assert!(!panics(true, "minecraft:fall", Generic));
    assert!(!panics(true, "minecraft:drown", Generic));
}

#[test]
fn a_cub_follows_an_adult_and_sounds_like_a_cub() {
    let mut world = EntityWorld::default();
    let mut tundra = Tundra;
    let adult = bear_at(&mut world, at(0.5, 0.5));
    bear_mut(&mut world, adult).core.no_ai = true;
    let cub = cub_at(&mut world, at(7.5, 0.5));
    let mut followed = false;
    let mut cub_sounds = 0;
    for _ in 0..400 {
        world.tick(&mut tundra);
        followed |= bear(&world, cub)
            .state
            .running_goals()
            .contains(&"FollowParentGoal");
        for sound in world.take_sounds() {
            if sound.event == AMBIENT_BABY {
                assert!((1.3..=1.7).contains(&sound.pitch), "{}", sound.pitch);
                cub_sounds += 1;
            } else if sound.event == "entity.polar_bear.ambient" {
                assert_eq!(sound.position, at(0.5, 0.5), "only the adult");
            }
        }
        bear_mut(&mut world, cub).core.ambient_sound_time = 1000;
    }
    assert!(followed);
    let distance = bear(&world, cub).core.body.position.distance(at(0.5, 0.5));
    assert!(distance < 5.0, "{distance}");
    assert!(cub_sounds > 300, "{cub_sounds}");
}
