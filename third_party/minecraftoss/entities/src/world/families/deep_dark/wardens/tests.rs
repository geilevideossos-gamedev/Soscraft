use super::*;
use crate::cow::Cow;
use crate::world::{PlayerAttack, ProjectileHit};
use minecraftoss_player::{Block, Pos};

struct Cave;

impl World for Cave {
    fn block(&self, pos: Pos) -> Option<Block> {
        (pos.1 < 64).then(|| Block::new("minecraft:deepslate"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}
}

struct Pool;

impl World for Pool {
    fn block(&self, pos: Pos) -> Option<Block> {
        if pos.1 < 64 {
            return Some(Block::new("minecraft:stone"));
        }
        (pos.0 >= 6 && pos.1 < 68).then(|| Block::new("minecraft:water"))
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

fn sword(player_id: u64, position: DVec3) -> PlayerAttack {
    PlayerAttack {
        player_id,
        position,
        yaw: 0.0,
        attack_damage: 8.0,
        strength: 1.0,
        sprinting: false,
        can_critical: false,
        can_sweep: false,
    }
}

fn calm_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    let id = world.spawn_family_kind(KIND, position).unwrap();
    let memories = world.warden_mut(id).unwrap().state.memories_mut();
    memories.dig_cooldown.set_for((), DIGGING_COOLDOWN);
    id
}

fn warden_of(world: &EntityWorld, id: u64) -> &FamilyMob<Warden> {
    world
        .wardens()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn sounds_of(world: &mut EntityWorld) -> Vec<String> {
    world.take_sounds().into_iter().map(|s| s.event).collect()
}

fn fighting(world: &mut EntityWorld, id: u64, target: Target) {
    let mob = world.warden_mut(id).unwrap();
    mob.state.anger.increase(target, 0, 100);
    set_attack_target(mob, target);
    mob.state.brain.update_activity();
    mob.state.brain.started = true;
    mob.state.brain.scan = 1;
}

#[test]
fn a_warden_is_tall_strong_and_slow() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let id = calm_at(&mut world, at(0.5, 0.5));
    assert_eq!(world.entity_type(id), Some(KIND));
    assert_eq!(world.family_core(id).map(|(key, _)| key), Some(44));
    let core = &warden_of(&world, id).core;
    assert_eq!((core.body.width, core.body.height), (0.9, 2.9));
    assert!((core.eye_height - 2.465).abs() < 1.0e-6);
    assert_eq!((core.health, core.max_health), (500.0, 500.0));
    assert_eq!(core.body.step_after, Some(0.55));
    assert_eq!(MOVEMENT_SPEED, 0.300_000_011_920_928_96);
    assert_eq!(
        (ATTACK_DAMAGE, ATTACK_KNOCKBACK, FOLLOW_RANGE),
        (30.0, 1.5, 24.0)
    );
    let traits = world.family_traits(id).unwrap();
    assert_eq!(traits.category, "monster");
    assert_eq!(traits.knockback_resistance, 1.0);
    assert!(traits.fire_immune && traits.enemy && !traits.undead && !traits.leashable);
    assert_eq!(traits.voice, ("warden", 4.0, "hostile_volume"));
    assert!(crate::monster_ai::is_enemy(KIND));
    let profile = &warden_of(&world, id).state.profile;
    assert!(profile.can_float && profile.distance_xz);
    assert_eq!(profile.walk_target, WalkTarget::Zero);
    for (path, malus) in [
        (PathType::UnpassableRail, 0.0),
        (PathType::Damaging, 8.0),
        (PathType::PowderSnow, 8.0),
        (PathType::Lava, 8.0),
        (PathType::Fire, 0.0),
        (PathType::FireInNeighbor, 0.0),
    ] {
        assert_eq!(profile.malus(path), malus, "{path:?}");
    }
    let mob = world.warden_mut(id).unwrap();
    set_pose(mob, Pose::Digging);
    assert_eq!((mob.core.body.height, mob.core.eye_height), (1.0, 0.85));
    set_pose(mob, Pose::Standing);
    assert_eq!(mob.core.body.height, 2.9);
    let player = DVec3::new(0.5, 64.0, 2.5);
    world.warden_mut(id).unwrap().core.health = 1.0;
    assert!(world.player_attack(&sword(3, player), id).died);
    let deaths = world.take_deaths();
    assert_eq!(deaths.len(), 1);
    assert_eq!((deaths[0].table, deaths[0].experience), (Some(KIND), 5));
}

#[test]
fn a_vibration_angers_it_by_thirty_five_and_it_listens_for_forty_ticks() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let players = [player_at(1, at(-20.5, 0.5))];
    assert!(world.warden_can_hear(id, Some(Target::Player(1)), &players));
    world.warden_hears(id, (6, 64, 0), Some(Target::Player(1)), None, &players);
    let state = &warden_of(&world, id).state;
    assert_eq!(state.anger.anger_of(Target::Player(1)), 35);
    assert_eq!(state.anger_level(), AngerLevel::Calm);
    assert_eq!(
        state.memories().disturbance_location.get(),
        Some(&(6, 64, 0))
    );
    assert!(state.memories().vibration_cooldown.present());
    assert_eq!(world.take_entity_events(), vec![(id, TENDRIL_EVENT)]);
    let sounds = world.take_sounds();
    let events: Vec<(&str, f32)> = sounds
        .iter()
        .map(|s| (s.event.as_str(), s.volume))
        .collect();
    assert_eq!(
        events,
        vec![
            ("entity.warden.tendril_clicks", 5.0),
            ("entity.warden.listening", 10.0)
        ]
    );
    assert!(!world.warden_can_hear(id, Some(Target::Player(1)), &players));
    for _ in 0..41 {
        world.tick_with_players(&mut Cave, &players);
    }
    assert!(world.warden_can_hear(id, Some(Target::Player(1)), &players));
    world.warden_hears(id, (6, 64, 0), Some(Target::Player(1)), None, &players);
    world.warden_hears(id, (6, 64, 0), Some(Target::Player(1)), None, &players);
    let state = &warden_of(&world, id).state;
    let anger = state.anger.anger_of(Target::Player(1));
    assert_eq!(anger, 33 + 35 + 35, "two seconds of calming, then two more");
    assert_eq!(state.anger_level(), AngerLevel::Angry);
    assert!(sounds_of(&mut world).contains(&"entity.warden.listening_angry".to_owned()));
    let mut creative = players;
    creative[0].attackable = false;
    let calm = calm_at(&mut world, at(-6.5, 0.5));
    assert!(
        !world.warden_can_hear(calm, Some(Target::Player(1)), &creative),
        "a creative player is no suspect"
    );
    assert!(
        world.warden_can_hear(calm, None, &creative),
        "a sourceless event is heard"
    );
}

#[test]
fn a_far_projectile_angers_only_its_near_owner() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let players = [player_at(1, at(20.5, 0.5)), player_at(2, at(40.5, 0.5))];
    let landed = (3, 64, 0);
    world.warden_hears(id, landed, None, Some(Target::Player(1)), &players);
    let state = &warden_of(&world, id).state;
    assert_eq!(state.anger.anger_of(Target::Player(1)), 10);
    assert!(state.memories().recent_projectile.present());
    assert_eq!(state.memories().disturbance_location.get(), Some(&landed));
    world.warden_hears(id, landed, None, Some(Target::Player(1)), &players);
    let state = &warden_of(&world, id).state;
    assert_eq!(state.anger.anger_of(Target::Player(1)), 45);
    assert_eq!(
        state.memories().disturbance_location.get(),
        Some(&(20, 64, 0)),
        "a second shot within the memory leads it to the shooter"
    );
    let other = calm_at(&mut world, at(0.5, 8.5));
    world.warden_hears(other, landed, None, Some(Target::Player(2)), &players);
    let state = &warden_of(&world, other).state;
    assert_eq!(
        state.anger.anger_of(Target::Player(2)),
        0,
        "a shooter forty blocks off is not blamed"
    );
    assert!(state.memories().recent_projectile.present());
    assert_eq!(state.memories().disturbance_location.get(), Some(&landed));
}

#[test]
fn it_walks_to_a_vibration_and_sniffs() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let players = [player_at(1, at(-12.5, 0.5))];
    world.warden_hears(id, (8, 64, 8), None, None, &players);
    let start = warden_of(&world, id).core.body.position;
    let goal = DVec3::new(8.5, 64.0, 8.5);
    let mut closest = start.distance(goal);
    let mut sniffed = None;
    let mut heard = Vec::new();
    for tick in 0..400 {
        world.tick_with_players(&mut Cave, &players);
        heard.extend(sounds_of(&mut world));
        let mob = warden_of(&world, id);
        closest = closest.min(mob.core.body.position.distance(goal));
        if tick < 60 {
            assert_eq!(mob.state.activity(), Activity::Investigate, "tick {tick}");
        }
        if mob.state.pose == Pose::Sniffing {
            sniffed = Some(tick);
            break;
        }
    }
    assert!(closest < 3.0, "it reached the disturbance: {closest}");
    assert!(sniffed.is_some(), "and sniffed after it");
    world.tick_with_players(&mut Cave, &players);
    heard.extend(sounds_of(&mut world));
    assert!(
        heard.contains(&"entity.warden.sniff".to_owned()),
        "{heard:?}"
    );
    assert_eq!(warden_of(&world, id).state.activity(), Activity::Sniff);
    for _ in 0..90 {
        world.tick_with_players(&mut Cave, &players);
    }
    let state = &warden_of(&world, id).state;
    assert_ne!(state.pose, Pose::Sniffing);
    assert_eq!(
        state.anger.anger_of(Target::Player(1)),
        0,
        "a player farther than six is not blamed"
    );
}

#[test]
fn sniffing_angers_it_at_the_nearest_within_six() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let brain = &mut world.warden_mut(id).unwrap().state.brain;
    brain.started = true;
    brain.scan = 1;
    let players = [player_at(1, at(4.5, 0.5))];
    let mut sniffs = 0;
    for _ in 0..200 {
        world.tick_with_players(&mut Cave, &players);
        if warden_of(&world, id).state.pose == Pose::Sniffing {
            sniffs += 1;
        }
        if warden_of(&world, id)
            .state
            .anger
            .anger_of(Target::Player(1))
            > 0
        {
            break;
        }
    }
    assert_eq!(sniffs, 86, "it sniffed for its full length first");
    let state = &warden_of(&world, id).state;
    assert_eq!(state.anger.anger_of(Target::Player(1)), 35);
    assert_eq!(
        state.memories().disturbance_location.get(),
        Some(&(4, 64, 0))
    );
    assert_eq!(state.pose, Pose::Standing);
}

#[test]
fn a_summoned_warden_emerges_for_134_ticks_and_cannot_be_hurt() {
    let mut world = EntityWorld::default();
    let id = world.spawn_emerging_warden(at(0.5, 0.5), 30.0);
    let players = [player_at(1, at(3.5, 0.5))];
    let mob = warden_of(&world, id);
    assert_eq!(
        (mob.state.pose, mob.core.body.height),
        (Pose::Emerging, 1.0)
    );
    assert!(mob.state.memories().dig_cooldown.present());
    assert!(
        !world.warden_can_hear(id, None, &players),
        "it hears nothing while it climbs out"
    );
    let mut heard = sounds_of(&mut world);
    world.tick_with_players(&mut Cave, &players);
    heard.extend(sounds_of(&mut world));
    assert_eq!(
        heard[..2],
        [
            "entity.warden.agitated".to_owned(),
            "entity.warden.emerge".to_owned()
        ]
    );
    assert!(!world.player_attack(&sword(1, at(2.5, 0.5)), id).hurt);
    let lava = Hurt::new(4.0, "minecraft:lava", DamageSourceKind::Hazard);
    assert!(!world.hurt_family(id, &lava).unwrap().applied);
    let void = Hurt::new(4.0, "minecraft:out_of_world", DamageSourceKind::Generic);
    assert!(world.hurt_family(id, &void).unwrap().applied);
    let start = warden_of(&world, id).core.body.position;
    for tick in 1..135 {
        world.tick_with_players(&mut Cave, &players);
        assert_eq!(
            warden_of(&world, id).state.pose,
            Pose::Emerging,
            "tick {tick}"
        );
    }
    world.tick_with_players(&mut Cave, &players);
    let mob = warden_of(&world, id);
    assert!(!mob.state.pose.buried() && mob.core.body.height == 2.9);
    assert!(!mob.state.memories().is_emerging.present());
    let moved = mob.core.body.position - start;
    assert!(
        moved.x.abs() < 1.0e-9 && moved.z.abs() < 1.0e-9,
        "it stays put while it climbs out"
    );
    assert!(world.player_attack(&sword(1, at(2.5, 0.5)), id).hurt);
}

#[test]
fn an_unbothered_warden_digs_away_after_1200_ticks() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let mut dug = None;
    let mut heard = Vec::new();
    for tick in 0..1600 {
        world.tick_with_players(&mut Cave, &[]);
        heard.extend(sounds_of(&mut world));
        if !world.mob_ids().any(|mob| mob == id) {
            dug = Some(tick);
            break;
        }
        let mob = warden_of(&world, id);
        if tick < 1200 {
            assert_ne!(mob.state.pose, Pose::Digging, "tick {tick}");
        }
    }
    let dug = dug.expect("it digs away");
    assert!((1300..1500).contains(&dug), "{dug}");
    assert!(heard.contains(&"entity.warden.dig".to_owned()));
    assert!(world.take_deaths().is_empty(), "no loot and no experience");
}

#[test]
fn a_persistent_warden_never_digs() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    world.warden_mut(id).unwrap().core.persistent = true;
    for _ in 0..1500 {
        world.tick_with_players(&mut Cave, &[]);
    }
    let mob = warden_of(&world, id);
    assert!(mob.state.memories().dig_cooldown.present());
    assert_ne!(mob.state.pose, Pose::Digging);
    assert_ne!(mob.state.activity(), Activity::Dig);
}

#[test]
fn a_warden_on_peaceful_is_removed_but_never_far_away() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let far = [DVec3::new(500.5, 64.0, 0.5)];
    world.warden_mut(id).unwrap().core.no_action_time = 10_000;
    for _ in 0..2_000 {
        assert_eq!(world.check_despawn(&far, false, &|_| true), 0);
    }
    assert_eq!(world.check_despawn(&far, true, &|_| true), 1);
    assert!(world.mob_ids().all(|mob| mob != id));
}

#[test]
fn a_cow_bumping_into_it_angers_it_and_draws_it_there() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let cow = world.spawn_cow(Cow::new(at(0.9, 0.5)), true);
    world.tick_with_players(&mut Cave, &[]);
    let state = &warden_of(&world, id).state;
    assert_eq!(state.anger.anger_of(Target::Mob(cow)), 35);
    assert!(state.memories().touch_cooldown.present());
    assert_eq!(
        state.memories().disturbance_location.get(),
        Some(&(0, 64, 0))
    );
    for _ in 0..10 {
        world.tick_with_players(&mut Cave, &[]);
    }
    assert_eq!(
        warden_of(&world, id).state.anger.anger_of(Target::Mob(cow)),
        35,
        "the touch cools down for a second"
    );
}

#[test]
fn a_silverfish_and_a_warden_post_their_steps_and_swims() {
    use crate::world::families::pests::silverfish;
    let mut world = EntityWorld::default();
    let fish = world
        .spawn_family_kind(silverfish::KIND, at(0.5, 0.5))
        .unwrap();
    let id = calm_at(&mut world, at(0.5, 3.5));
    let mut posted: Vec<(u64, &'static str)> = Vec::new();
    for _ in 0..80 {
        for mob in [fish, id] {
            let (_, core) = world.family_core_mut(mob).unwrap();
            core.body.velocity.x = 0.25;
        }
        world.tick_with_players(&mut Pool, &[]);
        for event in world.take_game_events() {
            if let Some(mob) = event.mob {
                posted.push((mob, event.event));
            }
        }
    }
    for mob in [fish, id] {
        let events: Vec<&str> = posted
            .iter()
            .filter(|(m, _)| *m == mob)
            .map(|(_, e)| *e)
            .collect();
        assert!(events.contains(&"minecraft:step"), "{mob}: {events:?}");
        assert!(events.contains(&"minecraft:swim"), "{mob}: {events:?}");
    }
}

#[test]
fn an_angry_warden_roars_eighty_four_ticks_then_attacks() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let players = [player_at(1, at(8.5, 0.5))];
    let mob = world.warden_mut(id).unwrap();
    mob.state.anger.increase(Target::Player(1), 0, 90);
    let mut roaring = Vec::new();
    let mut roared = Vec::new();
    let mut angered = None;
    for tick in 0..120 {
        world.tick_with_players(&mut Cave, &players);
        let heard = world.take_sounds();
        if let Some(roar) = heard.iter().find(|s| s.event == "entity.warden.roar") {
            assert_eq!((roar.volume, roar.pitch), (3.0, 1.0));
            roared.push(tick);
        }
        let state = &warden_of(&world, id).state;
        if state.pose == Pose::Roaring {
            if roaring.is_empty() {
                angered = Some(state.anger.anger_of(Target::Player(1)));
                assert_eq!(state.activity(), Activity::Roar);
            }
            roaring.push(tick);
        }
        if state.memories().attack_target.present() {
            break;
        }
    }
    assert_eq!(
        roaring.len(),
        85,
        "the roar runs from its start through its 84th tick"
    );
    assert_eq!(angered, Some(110), "roaring adds 20 anger");
    assert_eq!(
        roared,
        vec![roaring[0] + 26],
        "one roar once its 25-tick delay is over"
    );
    let state = &warden_of(&world, id).state;
    assert_eq!(state.pose, Pose::Standing);
    assert_eq!(
        state.memories().attack_target.get(),
        Some(&Target::Player(1))
    );
    assert!(!state.memories().roar_target.present());
    assert_eq!(
        state.memories().sonic_boom_cooldown.ttl(),
        Some(MELEE_UNTIL_SONIC_BOOM),
        "it fights in melee for ten seconds before its first boom"
    );
    world.tick_with_players(&mut Cave, &players);
    assert_eq!(warden_of(&world, id).state.activity(), Activity::Fight);
}

#[test]
fn its_melee_hits_thirty_and_disables_a_shield_for_five_seconds() {
    let mut world = EntityWorld::default();
    let id = calm_at(&mut world, at(0.5, 0.5));
    let players = [player_at(1, at(1.5, 0.5))];
    fighting(&mut world, id, Target::Player(1));
    world.tick_with_players(&mut Cave, &players);
    let hits = world.take_player_hits();
    assert_eq!(hits.len(), 1, "it swings at once");
    let mob = warden_of(&world, id);
    assert_eq!(
        (hits[0].player_id, hits[0].damage, hits[0].source),
        (1, 30.0, Some(id))
    );
    let PlayerHitKind::Melee {
        attacker,
        hunger_ticks,
        lift,
        throw,
        disable_shield_seconds,
    } = hits[0].kind
    else {
        panic!("{:?}", hits[0].kind);
    };
    assert!(attacker.distance(mob.core.body.position) < 0.1);
    assert_eq!((hunger_ticks, lift, disable_shield_seconds), (0, 0.0, 5.0));
    assert_eq!(throw, attack_throw(mob.core.yaw));
    assert!((throw.length() - (0.75_f64 * 0.75 + 0.4 * 0.4).sqrt()).abs() < 1.0e-6);
    assert_eq!(
        mob.state.memories().sonic_boom_cooldown.ttl(),
        Some(brain::SONIC_BOOM_COOLDOWN),
        "a swing sets the boom's cooldown to 40 even with 199 ticks left"
    );
    assert_eq!(world.take_entity_events(), vec![(id, ATTACK_EVENT)]);
    let sounds = world.take_sounds();
    let impact = sounds
        .iter()
        .find(|s| s.event == "entity.warden.attack_impact")
        .expect("the impact is heard");
    assert_eq!(impact.volume, 10.0);
    let mut next = None;
    for tick in 1..40 {
        world.tick_with_players(&mut Cave, &players);
        if !world.take_player_hits().is_empty() {
            next = Some(tick);
            break;
        }
    }
    assert_eq!(next, Some(19), "18 ticks of cooling down between swings");
}

#[test]
fn a_hit_angers_it_by_a_hundred_and_only_a_close_or_direct_one_starts_the_fight() {
    let mut world = EntityWorld::default();
    let players = [
        player_at(1, at(2.5, 0.5)),
        player_at(2, at(12.5, 20.5)),
        player_at(3, at(20.5, 40.5)),
    ];
    let sword_hit = calm_at(&mut world, at(0.5, 0.5));
    assert!(world.player_attack(&sword(1, at(2.5, 0.5)), sword_hit).hurt);
    let arrow_hit = calm_at(&mut world, at(0.5, 20.5));
    let arrow = ProjectileHit {
        amount: 6.0,
        kind: "minecraft:arrow",
        player: 2,
        velocity: DVec3::new(-2.0, 0.0, 0.0),
    };
    assert!(
        world
            .player_projectile_hurt(arrow_hit, arrow)
            .unwrap()
            .applied
    );
    let bullet_hit = calm_at(&mut world, at(0.5, 40.5));
    assert!(
        world
            .player_attack(&sword(3, at(20.5, 40.5)), bullet_hit)
            .hurt
    );
    let bitten = calm_at(&mut world, at(0.5, 60.5));
    let cow = world.spawn_cow(Cow::new(at(8.5, 60.5)), true);
    let bite = Hurt::new(4.0, "minecraft:mob_attack", DamageSourceKind::MobAttack)
        .by(Some(Target::Mob(cow)), Some("minecraft:cow"));
    assert!(world.hurt_family(bitten, &bite).unwrap().applied);
    world.tick_with_players(&mut Cave, &players);
    for (id, by, fights) in [
        (sword_hit, Target::Player(1), true),
        (arrow_hit, Target::Player(2), false),
        (bullet_hit, Target::Player(3), false),
        (bitten, Target::Mob(cow), true),
    ] {
        let state = &warden_of(&world, id).state;
        assert_eq!(state.anger.anger_of(by), 100, "{by:?}");
        assert_eq!(state.anger_level(), AngerLevel::Angry, "{by:?}");
        let target = state.memories().attack_target.get().copied();
        assert_eq!(target, fights.then_some(by), "{by:?}");
        assert_eq!(
            state.memories().roar_target.get().copied(),
            (!fights).then_some(by),
            "{by:?}: a far shot makes it roar first"
        );
        if fights {
            assert_eq!(
                state.memories().sonic_boom_cooldown.ttl(),
                Some(MELEE_UNTIL_SONIC_BOOM - 1),
                "{by:?}"
            );
        }
    }
    let buried = world.spawn_emerging_warden(at(0.5, 80.5), 0.0);
    world.tick_with_players(&mut Cave, &players);
    let void = Hurt::new(1.0, "minecraft:out_of_world", DamageSourceKind::Generic)
        .by(Some(Target::Player(1)), Some("minecraft:player"));
    assert!(world.hurt_family(buried, &void).unwrap().applied);
    world.tick_with_players(&mut Cave, &players);
    assert_eq!(
        warden_of(&world, buried)
            .state
            .anger
            .anger_of(Target::Player(1)),
        0,
        "nothing angers it while it climbs out"
    );
}

#[test]
fn an_angry_warden_roars_at_a_cow_and_beats_it_to_death() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let id = calm_at(&mut world, at(0.5, 0.5));
    let cow = world.spawn_cow(Cow::new(at(4.5, 0.5)), true);
    world
        .warden_mut(id)
        .unwrap()
        .state
        .anger
        .increase(Target::Mob(cow), 0, 90);
    let mut roared = false;
    let mut died = None;
    for tick in 0..400 {
        world.tick_with_players(&mut Cave, &[]);
        roared |= warden_of(&world, id).state.pose == Pose::Roaring;
        if world.take_deaths().iter().any(|death| death.id == cow) {
            died = Some(tick);
            break;
        }
    }
    assert!(roared, "it roars at the cow first");
    let died = died.expect("one swing of 30 kills a cow");
    assert!((86..150).contains(&died), "{died}");
}
