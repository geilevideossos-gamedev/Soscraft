use super::goals::{
    AttackPlayerTargetGoal, AttackStrategyGoal, CircleAroundAnchorGoal, SweepAttackGoal,
};
use super::*;
use crate::feline::{Feline, FelineKind};
use crate::goals::Goal;
use crate::monster_ai::MonsterGoalContext;
use crate::world::families::Hurt;
use crate::world::{DamageSourceKind, PlayerAttack};
use minecraftoss_player::{Block, Pos};

struct Field {
    roof: Option<i32>,
}

impl World for Field {
    fn block(&self, pos: Pos) -> Option<Block> {
        let roofed = self
            .roof
            .is_some_and(|roof| pos.1 == roof && pos.0.abs() < 40 && pos.2.abs() < 40);
        (pos.1 < 64 || roofed).then(|| Block::new("minecraft:stone"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}
}

fn open() -> Field {
    Field { roof: None }
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

fn phantom_at(world: &mut EntityWorld, at: DVec3) -> u64 {
    world.spawn_family_kind(KIND, at).unwrap()
}

fn phantom(world: &EntityWorld, id: u64) -> &FamilyMob<Phantom> {
    world
        .family::<Phantom>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn goals_of(world: &EntityWorld, id: u64) -> PhantomGoals {
    phantom(world, id).state.goals().cloned().unwrap()
}

fn target(world: &EntityWorld, id: u64) -> Option<Target> {
    phantom(world, id).state.ai.state.target
}

fn context(at: DVec3, players: Vec<PlayerCandidate>) -> MonsterGoalContext {
    let body = Body::new(at, WIDTH, HEIGHT);
    let mut ai = MonsterAi::of_kind(MonsterKind::Registered(KIND), &body, 0.0);
    ai.state.body = body;
    ai.state.players = players;
    ai.state.extra.set(PhantomGoals {
        sea_level: 63,
        ..PhantomGoals::default()
    });
    ai.state
}

fn shared(ctx: &mut MonsterGoalContext) -> &mut PhantomGoals {
    ctx.extra.get_mut::<PhantomGoals>().unwrap()
}

#[test]
fn a_phantom_is_nine_tenths_by_half_with_twenty_health_and_its_kind() {
    let mut world = EntityWorld::default();
    let id = phantom_at(&mut world, DVec3::new(0.5, 70.0, 0.5));
    assert_eq!(world.entity_type(id), Some(KIND));
    assert_eq!(world.family_core(id).map(|(key, _)| key), Some(80));
    let mob = phantom(&world, id);
    let core = &mob.core;
    assert_eq!(
        (core.body.width, core.body.height, core.eye_height),
        (0.9, 0.5, 0.175)
    );
    assert_eq!((core.health, core.max_health), (20.0, 20.0));
    assert_eq!(
        (mob.state.size, mob.state.attack_damage, mob.state.speed),
        (0, 2.0, 0.1)
    );
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
    assert!(traits.enemy && traits.undead && !traits.fire_immune && !traits.leashable);
    assert_eq!(traits.voice, ("phantom", 1.0, "hostile_volume"));
    assert_eq!(traits.seats, 0);
    let registered = MonsterKind::Registered(KIND);
    assert_eq!(registered.goal_names(), GOAL_NAMES);
    assert_eq!(registered.target_goal_names(), TARGET_NAMES);
    let ai = &mob.state.ai;
    assert_eq!(ai.state.follow_range, FOLLOW_RANGE);
    assert_eq!(ai.state.eye_height, EYE_HEIGHT);
    let attack = PlayerAttack {
        player_id: 1,
        position: DVec3::new(0.5, 70.0, 3.5),
        yaw: 180.0,
        attack_damage: 100.0,
        strength: 1.0,
        sprinting: false,
        can_critical: false,
        can_sweep: false,
    };
    assert!(world.player_attack(&attack, id).died);
    let deaths = world.take_deaths();
    assert_eq!((deaths[0].id, deaths[0].table), (id, Some(KIND)));
    assert_eq!(deaths[0].experience, 5);
    assert!(deaths[0].equipment.is_empty());
}

#[test]
fn a_bigger_phantom_grows_fifteen_percent_a_size_and_bites_for_six_plus_its_size() {
    let mut mob = Phantom::spawn(KIND, DVec3::ZERO).unwrap();
    set_size(&mut mob, 0);
    assert_eq!(
        (mob.state.size, mob.state.attack_damage, mob.core.body.width),
        (0, 2.0, 0.9),
        "a size that does not change keeps the first damage"
    );
    set_size(&mut mob, 4);
    let core = &mob.core;
    assert_eq!(mob.state.size, 4);
    assert!((core.body.width - 1.44).abs() < 1e-6, "{}", core.body.width);
    assert!(
        (core.body.height - 0.8).abs() < 1e-6,
        "{}",
        core.body.height
    );
    assert!((core.eye_height - 0.28).abs() < 1e-6, "{}", core.eye_height);
    assert_eq!(mob.state.attack_damage, 10.0);
    set_size(&mut mob, 70);
    assert_eq!((mob.state.size, mob.state.attack_damage), (64, 70.0));
    assert!((mob.core.body.width - 0.9 * 10.6).abs() < 1e-4);
    set_size(&mut mob, -3);
    assert_eq!((mob.state.size, mob.state.attack_damage), (0, 6.0));
    assert_eq!((mob.core.body.width, mob.core.body.height), (0.9, 0.5));
}

#[test]
fn a_phantom_circles_five_to_fifteen_blocks_around_its_anchor_without_falling() {
    let mut world = EntityWorld::default();
    let mut field = open();
    let id = phantom_at(&mut world, DVec3::new(0.5, 80.0, 0.5));
    let (mut lowest, mut highest, mut farthest) = (f64::MAX, f64::MIN, 0.0_f64);
    let mut angles = Vec::new();
    for tick in 1..=600 {
        world.tick(&mut field);
        let mob = phantom(&world, id);
        let p = mob.core.body.position;
        assert_eq!(goals_of(&world, id).anchor, Some((0, 80, 0)), "tick {tick}");
        if tick > 100 {
            lowest = lowest.min(p.y);
            highest = highest.max(p.y);
            farthest = farthest.max(p.x.hypot(p.z));
            angles.push(p.z.atan2(p.x));
        }
    }
    assert!(lowest > 70.0 && highest < 86.0, "{lowest} to {highest}");
    assert!(farthest > 5.0 && farthest < 17.5, "{farthest}");
    let turned: f64 = angles
        .windows(2)
        .map(|pair| {
            let step = pair[1] - pair[0];
            (step + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
        })
        .sum();
    assert!(
        turned.abs() > std::f64::consts::TAU,
        "it goes around: {turned}"
    );
    let mob = phantom(&world, id);
    assert!(mob.core.health == MAX_HEALTH && mob.core.body.fall_distance == 0.0);
    assert_eq!(mob.core.look.head_yaw, mob.core.yaw);
}

#[test]
fn its_move_control_turns_four_degrees_a_tick_and_speeds_to_one_point_eight() {
    let mut body = Body::new(DVec3::ZERO, WIDTH, HEIGHT);
    let (mut speed, mut yaw, mut pitch) = (START_SPEED, 0.0_f32, 0.0_f32);
    let east = DVec3::new(100.0, 0.0, 0.0);
    move_control(&mut speed, &mut yaw, &mut pitch, &mut body, east);
    assert_eq!((yaw, speed, pitch), (-4.0, 0.125, 0.0));
    for _ in 0..21 {
        move_control(&mut speed, &mut yaw, &mut pitch, &mut body, east);
    }
    assert_eq!((yaw, speed), (-88.0, 0.2));
    move_control(&mut speed, &mut yaw, &mut pitch, &mut body, east);
    assert_eq!(yaw, -90.0);
    assert!(
        speed > 0.2,
        "aligned within three degrees it speeds up: {speed}"
    );
    let mut last = speed;
    for _ in 0..400 {
        move_control(&mut speed, &mut yaw, &mut pitch, &mut body, east);
        assert!(speed >= last && speed <= TOP_SPEED);
        last = speed;
    }
    assert_eq!((yaw, speed), (-90.0, TOP_SPEED));
    assert!((body.velocity.x - 1.8).abs() < 1e-3, "{}", body.velocity);
    assert!(body.velocity.y.abs() < 1e-9 && body.velocity.z.abs() < 1e-3);
    move_control(
        &mut speed,
        &mut yaw,
        &mut pitch,
        &mut body,
        DVec3::new(10.0, 10.0, 0.0),
    );
    assert!(pitch > 70.0 && pitch < 75.0, "it noses up: {pitch}");
    body.horizontal_collision = true;
    move_control(&mut speed, &mut yaw, &mut pitch, &mut body, east);
    assert_eq!(speed, 0.125, "a wall drops it back to 0.1 before it turns");
    let turned = crate::wither::wrap_degrees(yaw + 90.0).abs();
    assert!((turned - 176.0).abs() < 1e-3, "it turned round: {yaw}");
    body.horizontal_collision = false;
    let before = (speed, yaw, body.velocity);
    move_control(&mut speed, &mut yaw, &mut pitch, &mut body, DVec3::ZERO);
    assert_eq!(
        (speed, yaw, body.velocity),
        before,
        "nothing changes over its target"
    );
}

#[test]
fn a_phantom_picks_the_highest_player_in_sight_within_its_box() {
    let mut world = EntityWorld::default();
    let mut field = open();
    let id = phantom_at(&mut world, DVec3::new(0.5, 80.0, 0.5));
    let mut creative = player_at(3, DVec3::new(0.5, 75.0, 3.5));
    creative.attackable = false;
    let mut spectator = player_at(4, DVec3::new(0.5, 76.0, -3.5));
    spectator.spectator = true;
    let players = [
        player_at(1, DVec3::new(3.5, 64.0, 0.5)),
        player_at(2, DVec3::new(-3.5, 70.0, 0.5)),
        creative,
        spectator,
        player_at(5, DVec3::new(40.5, 79.0, 0.5)),
    ];
    for tick in 1..=20 {
        world.tick_with_players(&mut field, &players);
        assert_eq!(target(&world, id), None, "tick {tick}");
    }
    world.tick_with_players(&mut field, &players);
    assert_eq!(
        target(&world, id),
        Some(Target::Player(2)),
        "the scan at 20 ticks"
    );

    let mut roofed = Field { roof: Some(68) };
    let mut world = EntityWorld::default();
    let id = phantom_at(&mut world, DVec3::new(0.5, 80.0, 0.5));
    let under = [player_at(1, DVec3::new(0.5, 64.0, 0.5))];
    for _ in 0..300 {
        world.tick_with_players(&mut roofed, &under);
        assert_eq!(target(&world, id), None, "a roof hides it");
    }

    let at = DVec3::new(0.5, 80.0, 0.5);
    let below = player_at(1, DVec3::new(0.5, 64.0, 0.5));
    let mut ctx = context(at, Vec::new());
    let mut goal = AttackPlayerTargetGoal::default();
    for call in 1..=10 {
        assert!(!goal.can_start(&mut ctx, &field), "call {call}");
    }
    assert!(
        !goal.can_start(&mut ctx, &field),
        "nobody at the first scan"
    );
    ctx.players = vec![below];
    for call in 1..=30 {
        assert!(!goal.can_start(&mut ctx, &field), "call {call}");
    }
    assert!(
        goal.can_start(&mut ctx, &field),
        "the next scan 60 ticks on"
    );
    assert_eq!(ctx.target, Some(Target::Player(1)));
    assert!(goal.can_continue(&mut ctx, &field));
    ctx.players[0].attackable = false;
    assert!(!goal.can_continue(&mut ctx, &field), "creative ends it");
}

#[test]
fn a_phantom_swoops_ten_ticks_after_finding_a_target_bites_and_climbs_away() {
    let mut world = EntityWorld::default();
    let mut field = open();
    let id = phantom_at(&mut world, DVec3::new(0.5, 80.0, 0.5));
    let players = [player_at(1, DVec3::new(0.5, 64.0, 0.5))];
    let (mut found, mut swoops, mut bites, mut hits) = (None, Vec::new(), Vec::new(), Vec::new());
    for tick in 1..=600 {
        world.tick_with_players(&mut field, &players);
        if found.is_none() && target(&world, id).is_some() {
            found = Some(tick);
        }
        for sound in world.take_sounds() {
            match sound.event.as_str() {
                SWOOP_SOUND => {
                    assert_eq!(sound.volume, SWOOP_VOLUME);
                    assert!((0.95..1.05).contains(&sound.pitch), "{}", sound.pitch);
                    assert_eq!(sound.category, "hostile_volume");
                    swoops.push(tick);
                }
                BITE_SOUND => {
                    assert_eq!(sound.volume, BITE_VOLUME);
                    assert!((0.9..1.0).contains(&sound.pitch), "{}", sound.pitch);
                    let centre = sound.position - sound.position.floor();
                    assert_eq!(centre, DVec3::splat(0.5), "the bite sounds at its block");
                    bites.push(tick);
                }
                _ => {}
            }
        }
        for hit in world.take_player_hits() {
            hits.push((tick, hit));
        }
        if bites.len() == 1 && tick == bites[0] + 4 {
            let goals = goals_of(&world, id);
            assert_eq!(target(&world, id), None, "the bite drops the target");
            assert_eq!(goals.phase, AttackPhase::Circle);
            let (x, y, z) = goals.anchor.unwrap();
            assert!((74..=93).contains(&y), "{y}: 10-29 above the ground at 64");
            assert!((x, z) == (0, 0), "{x} {z}");
        }
        if bites.len() == 1 && tick == bites[0] + 40 {
            assert!(phantom(&world, id).core.body.position.y > 66.0);
        }
    }
    assert_eq!(found, Some(21));
    let swoop = swoops[0];
    assert!((28..=30).contains(&swoop), "{swoops:?}");
    let (bite, hit) = &hits[0];
    assert_eq!(bites[0], *bite);
    assert_eq!((hit.player_id, hit.damage, hit.source), (1, 2.0, Some(id)));
    assert!(matches!(hit.kind, PlayerHitKind::Melee { lift: 0.0, .. }));
    assert_eq!(phantom(&world, id).core.swings as usize, hits.len());
    let next = swoops.iter().find(|&&tick| tick > *bite).copied().unwrap();
    assert!(
        next >= bite + 60,
        "it dives again once it finds the player again: {bite} {next}"
    );
}

#[test]
fn a_wall_or_a_hit_breaks_off_a_swoop() {
    let below = player_at(1, DVec3::new(0.5, 64.0, 0.5));
    let mut ctx = context(DVec3::new(0.5, 80.0, 0.5), vec![below]);
    let mut goal = SweepAttackGoal::default();
    let swoop = |ctx: &mut MonsterGoalContext| {
        ctx.target = Some(Target::Player(1));
        ctx.body.horizontal_collision = false;
        let goals = shared(ctx);
        goals.phase = AttackPhase::Swoop;
        goals.hurt_recently = false;
    };
    swoop(&mut ctx);
    goal.tick(&mut ctx);
    assert_eq!(shared(&mut ctx).phase, AttackPhase::Swoop);
    let middle = shared(&mut ctx).move_target;
    assert!(
        (middle - DVec3::new(0.5, 64.9, 0.5)).length() < 1e-6,
        "{middle}"
    );
    ctx.body.horizontal_collision = true;
    goal.tick(&mut ctx);
    assert_eq!(shared(&mut ctx).phase, AttackPhase::Circle, "a wall");
    assert!(!goal.can_continue(&mut ctx, &open()));
    swoop(&mut ctx);
    shared(&mut ctx).hurt_recently = true;
    goal.tick(&mut ctx);
    assert_eq!(shared(&mut ctx).phase, AttackPhase::Circle, "a hit");
    goal.stop(&mut ctx);
    assert_eq!(ctx.target, None);
    assert_eq!(ctx.attack, None, "no bite");
    swoop(&mut ctx);
    ctx.players[0].spectator = true;
    assert!(!goal.can_continue(&mut ctx, &open()), "a spectator");

    let mut world = EntityWorld::default();
    let mut field = open();
    let id = phantom_at(&mut world, DVec3::new(0.5, 80.0, 0.5));
    let players = [below];
    let mut swooped = None;
    for tick in 1..=200 {
        world.tick_with_players(&mut field, &players);
        let sounds = world.take_sounds();
        if sounds.iter().any(|sound| sound.event == SWOOP_SOUND) {
            swooped = Some(tick);
            break;
        }
    }
    assert!(swooped.is_some());
    for _ in 0..4 {
        world.tick_with_players(&mut field, &players);
    }
    assert_eq!(goals_of(&world, id).phase, AttackPhase::Swoop);
    let poke = Hurt::new(1.0, "minecraft:generic", DamageSourceKind::Generic);
    assert!(world.hurt_family(id, &poke).unwrap().applied);
    for _ in 0..4 {
        world.tick_with_players(&mut field, &players);
    }
    assert_eq!(target(&world, id), None, "the hit broke the swoop off");
    assert_eq!(goals_of(&world, id).phase, AttackPhase::Circle);
    assert!(world.take_player_hits().is_empty());
}

#[test]
fn its_anchor_never_sinks_below_sea_level() {
    let deep = player_at(1, DVec3::new(0.5, 10.0, 0.5));
    let mut ctx = context(DVec3::new(0.5, 40.0, 0.5), vec![deep]);
    ctx.target = Some(Target::Player(1));
    let mut goal = AttackStrategyGoal::default();
    goal.start(&mut ctx);
    assert_eq!(shared(&mut ctx).anchor, None, "no anchor, nothing to raise");
    assert_eq!(goal.next_sweep_tick, 5);
    for _ in 0..20 {
        shared(&mut ctx).anchor = Some((7, 90, 7));
        goal.start(&mut ctx);
        assert_eq!(shared(&mut ctx).anchor, Some((0, 64, 0)), "sea level 63");
    }
    ctx.players[0].position.y = 50.0;
    let mut seen = Vec::new();
    for _ in 0..200 {
        goal.start(&mut ctx);
        let (_, y, _) = shared(&mut ctx).anchor.unwrap();
        assert!((70..=89).contains(&y), "{y}");
        seen.push(y);
    }
    assert!(seen.contains(&70) && seen.contains(&89));
    let mut circle = CircleAroundAnchorGoal::default();
    circle.start(&mut ctx);
    let (anchor, to) = (
        shared(&mut ctx).anchor.unwrap(),
        shared(&mut ctx).move_target,
    );
    let around = DVec3::new(to.x - f64::from(anchor.0), 0.0, to.z - f64::from(anchor.2));
    assert!((5.0..=15.0).contains(&around.length()), "{around}");
    assert!((to.y - f64::from(anchor.1) + 4.0 - circle.height as f64).abs() < 1e-5);
}

struct Sky {
    rain: bool,
}

impl World for Sky {
    fn block(&self, pos: Pos) -> Option<Block> {
        (pos.1 < 64).then(|| Block::new("minecraft:stone"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}

    fn light_path_cost(&self, pos: Pos) -> f32 {
        if pos.1 >= 64 { 0.5 } else { -0.5 }
    }

    fn can_see_sky(&self, pos: Pos) -> bool {
        pos.1 >= 64
    }

    fn rain_at(&self, pos: Pos) -> bool {
        self.rain && pos.1 >= 64
    }
}

fn burning(day: bool, rain: bool, no_ai: bool) -> (bool, f32) {
    let mut world = EntityWorld::default();
    world.set_monsters_burn(day);
    let mut sky = Sky { rain };
    let id = phantom_at(&mut world, DVec3::new(0.5, 80.0, 0.5));
    world.family_core_mut(id).unwrap().1.no_ai = no_ai;
    let mut lit = false;
    for _ in 0..300 {
        world.tick(&mut sky);
        let Some(mob) = world
            .family::<Phantom>()
            .iter()
            .find(|mob| mob.core.id == id)
        else {
            return (true, 0.0);
        };
        lit |= mob.core.body.fire_ticks > 0;
    }
    (lit, phantom(&world, id).core.health)
}

#[test]
fn a_phantom_catches_fire_under_the_day_sky_but_not_in_rain_or_at_night() {
    let (lit, health) = burning(true, false, false);
    assert!(lit && health < MAX_HEALTH, "it burns by day: {health}");
    let (lit, health) = burning(true, false, true);
    assert!(lit && health < MAX_HEALTH, "without AI too: {health}");
    assert_eq!(
        burning(true, true, false),
        (false, MAX_HEALTH),
        "not in rain"
    );
    assert_eq!(
        burning(false, false, false),
        (false, MAX_HEALTH),
        "not at night"
    );
}

fn hisses_and_bites(cat: Option<(FelineKind, DVec3)>) -> (Option<i32>, Option<i32>, usize) {
    let mut world = EntityWorld::default();
    let mut field = open();
    if let Some((kind, at)) = cat {
        world.spawn_feline(Feline::new(kind, at), true);
    }
    phantom_at(&mut world, DVec3::new(0.5, 70.0, 0.5));
    let players = [player_at(1, DVec3::new(0.5, 64.0, 0.5))];
    let (mut hissed, mut bitten, mut hisses) = (None, None, 0);
    for tick in 1..=600 {
        world.tick_with_players(&mut field, &players);
        let hiss = world
            .take_sounds()
            .iter()
            .filter(|sound| sound.event.ends_with(".hiss"))
            .count();
        if hiss > 0 {
            hissed.get_or_insert(tick);
        }
        hisses += hiss;
        if !world.take_player_hits().is_empty() {
            bitten.get_or_insert(tick);
        }
    }
    (hissed, bitten, hisses)
}

#[test]
fn a_cat_within_sixteen_blocks_breaks_off_a_swoop_and_hisses() {
    let at = DVec3::new(0.5, 80.0, 0.5);
    let below = player_at(1, DVec3::new(0.5, 64.0, 0.5));
    let mut ctx = context(at, vec![below]);
    let cat = |id: u64, x: f64| CatSeen {
        id,
        low: DVec3::new(x - 0.3, 64.0, 0.2),
        high: DVec3::new(x + 0.3, 64.7, 0.8),
    };
    let mut goal = SweepAttackGoal::default();
    let swoop = |ctx: &mut MonsterGoalContext, tick: i32, cats: Vec<CatSeen>| {
        ctx.target = Some(Target::Player(1));
        ctx.tick_count = tick;
        let goals = shared(ctx);
        goals.phase = AttackPhase::Swoop;
        goals.cats = cats;
        goals.hiss.clear();
    };
    swoop(&mut ctx, 100, vec![cat(7, 10.5), cat(8, 16.5)]);
    assert!(!goal.can_continue(&mut ctx, &open()), "a cat in reach");
    assert_eq!(shared(&mut ctx).hiss, [7, 8], "every cat in reach hisses");
    swoop(&mut ctx, 110, Vec::new());
    assert!(!goal.can_continue(&mut ctx, &open()), "still scared");
    assert!(shared(&mut ctx).hiss.is_empty(), "no search for 20 ticks");
    swoop(&mut ctx, 120, Vec::new());
    assert!(!goal.can_continue(&mut ctx, &open()));
    swoop(&mut ctx, 121, vec![cat(9, 20.5)]);
    assert!(goal.can_continue(&mut ctx, &open()), "a cat 20 blocks off");
    assert!(shared(&mut ctx).hiss.is_empty());

    let mut world = EntityWorld::default();
    let cat = world.spawn_feline(
        Feline::new(FelineKind::Cat, DVec3::new(2.5, 64.0, 0.5)),
        true,
    );
    let ocelot = world.spawn_feline(
        Feline::new(FelineKind::Ocelot, DVec3::new(4.5, 64.0, 0.5)),
        true,
    );
    let dead = world.spawn_feline(
        Feline::new(FelineKind::Cat, DVec3::new(6.5, 64.0, 0.5)),
        true,
    );
    world.feline_mut(dead).unwrap().feline.health = 0.0;
    let seen = cats_of(&world);
    assert_eq!(seen.iter().map(|cat| cat.id).collect::<Vec<_>>(), [cat]);
    assert!((seen[0].low - DVec3::new(2.2, 64.0, 0.2)).length() < 1e-6);
    assert!(world.felines().iter().any(|entity| entity.id == ocelot));

    let (hissed, bitten, hisses) = hisses_and_bites(None);
    assert!(hissed.is_none() && bitten.is_some(), "{bitten:?}");
    let near = Some((FelineKind::Cat, DVec3::new(1.5, 64.0, 0.5)));
    let (hissed, bitten, hisses_near) = hisses_and_bites(near);
    let hissed = hissed.expect("the cat hisses at the swooping phantom");
    assert!(
        bitten.is_none_or(|bite| bite > hissed),
        "{hissed} {bitten:?}"
    );
    assert!(hisses_near > hisses);
    for far in [
        (FelineKind::Cat, DVec3::new(40.5, 64.0, 0.5)),
        (FelineKind::Ocelot, DVec3::new(1.5, 64.0, 0.5)),
    ] {
        let (hissed, bitten, _) = hisses_and_bites(Some(far));
        assert!(hissed.is_none() && bitten.is_some(), "{far:?}: {bitten:?}");
    }
}
