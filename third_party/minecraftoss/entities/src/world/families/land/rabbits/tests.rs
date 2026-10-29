use super::ai::{self, RabbitJump};
use super::*;
use crate::animal::AnimalEvent;
use crate::controls::PlayerControls;
use crate::cow::InteractionResult;
use crate::goals::Goal;
use crate::monster_ai::Target;
use crate::movement::Body;
use crate::walk_path::WalkTarget;
use crate::world::{DamageSourceKind, LeashHolder, PlayerAttack, PlayerHitKind};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::path_type::{PathType, path_type_of_block};
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::survival::Armor;
use minecraftoss_player::{
    Block, Difficulty, HitFrom, IncomingHit, Pos, authored_block_in_tag, authored_collision_boxes,
};
use std::collections::HashMap;

#[derive(Default)]
struct Meadow {
    water: Vec<Pos>,
}

impl World for Meadow {
    fn block(&self, pos: Pos) -> Option<Block> {
        if self.water.contains(&pos) {
            return Some(Block::new("minecraft:water"));
        }
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

fn rabbit_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    world.spawn_family_kind(KIND, position).unwrap()
}

fn baby_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    let mut mob = Rabbit::spawn(KIND, position).unwrap();
    mob.state.age.set(Age::BABY_START);
    set_baby(&mut mob, true);
    world.spawn_family(mob)
}

fn rabbit(world: &EntityWorld, id: u64) -> &FamilyMob<Rabbit> {
    world
        .family::<Rabbit>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn rabbit_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Rabbit> {
    world.family_mob_mut::<Rabbit>(id).unwrap()
}

fn jump_pitches(world: &mut EntityWorld) -> Vec<f32> {
    world
        .take_sounds()
        .into_iter()
        .filter(|sound| sound.event == ai::JUMP_SOUND)
        .map(|sound| sound.pitch)
        .collect()
}

#[test]
fn a_rabbit_is_small_with_three_health_and_its_kind() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let adult = rabbit_at(&mut world, at(0.5, 0.5));
    let baby = baby_at(&mut world, at(4.5, 0.5));
    assert_eq!(world.entity_type(adult), Some(KIND));
    assert_eq!(world.family_core(adult).map(|(key, _)| key), Some(48));
    let core = &rabbit(&world, adult).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (0.49, 0.6, 0.59, false)
    );
    assert_eq!((core.health, core.max_health), (3.0, 3.0));
    assert_eq!(core.ambient_interval, 120);
    let core = &rabbit(&world, baby).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (0.24, 0.4, 0.39, true)
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
            .any(|m| m.id == baby && m.kind == KIND && m.baby)
    );
    assert!(!crate::monster_ai::is_enemy(KIND));
    let traits = world.family_traits(adult).unwrap();
    assert_eq!(traits.category, "creature");
    assert!(traits.leashable && !traits.enemy && !traits.undead);
    assert!(!traits.fire_immune && !traits.breathes_water && !traits.panicking);
    assert_eq!(traits.voice, ("rabbit", 1.0, "neutral_volume"));
    let registered = MonsterKind::Registered(KIND);
    assert_eq!(registered.goal_names(), GOAL_NAMES);
    assert_eq!(registered.target_goal_names(), TARGET_NAMES);
    let ai = &rabbit(&world, adult).state.ai;
    assert_eq!(ai.state.follow_range, FOLLOW_RANGE);
    assert_eq!(ai.state.eye_height, EYE_HEIGHT);
    assert!(ai.state.walk.can_float, "its FloatGoal floats the path");
    assert_eq!(ai.state.walk.walk_target, WalkTarget::Animal);
    let player = player_at(1, at(2.5, 0.5));
    for id in [adult, baby] {
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
    let baby = deaths.iter().find(|d| d.id == baby).unwrap();
    assert_eq!(
        (baby.table, baby.experience),
        (None, 0),
        "babies drop nothing"
    );
}

#[test]
fn a_rabbit_moves_only_while_it_is_in_the_air() {
    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    let id = rabbit_at(&mut world, at(0.5, 0.5));
    rabbit_mut(&mut world, id).state.ai.retain_goals(&[]);
    let (mut pitches, mut delays, mut hops) = (Vec::new(), Vec::new(), Vec::new());
    for tick in 0..240 {
        if tick == 5 {
            let mob = rabbit_mut(&mut world, id);
            let state = &mut mob.state.ai.state;
            let walk = state.walk.clone();
            let target = at(14.5, 0.5);
            let navigation = &mut state.navigation;
            navigate_walk_to(
                &mob.core.body,
                navigation,
                &meadow,
                &walk,
                state.fluid,
                target,
                1.0,
                1,
            );
        }
        let grounded = rabbit(&world, id).core.body.on_ground;
        world.tick(&mut meadow);
        let hopped = jump_pitches(&mut world);
        let mob = rabbit(&world, id);
        let jump = mob.state.jump();
        if !hopped.is_empty() {
            hops.push(tick);
            assert_eq!((jump.duration, jump.ticks), (15, 1), "tick {tick}");
        }
        if grounded && mob.core.body.on_ground && hopped.is_empty() {
            assert_eq!(
                mob.state.ai.forward, 0.0,
                "tick {tick}: no push on the ground"
            );
        }
        pitches.extend(hopped);
        delays.push(jump.delay);
    }
    assert!(hops.len() >= 4, "{hops:?}");
    assert!(
        pitches.iter().all(|pitch| (0.64..=0.96).contains(pitch)),
        "{pitches:?}"
    );
    assert_eq!(delays.iter().max(), Some(&10), "a walk waits ten ticks");
    for pair in hops.windows(2) {
        assert!(pair[1] - pair[0] > 10, "{hops:?}");
    }
    let x = rabbit(&world, id).core.body.position.x;
    assert!(x > 6.0, "it hopped along its path: {x}");
}

#[test]
fn a_hop_rises_and_pushes_forward_like_vanilla() {
    let meadow = Meadow::default();
    let mob = Rabbit::spawn(KIND, at(0.5, 0.5)).unwrap();
    let hop = |speed: f64, baby: bool, velocity: DVec3, nodes: Vec<Pos>| {
        let mut ai = (*mob.state.ai).clone();
        let mut body = mob.core.body.clone();
        body.on_ground = true;
        body.velocity = velocity;
        ai.move_control.speed_modifier = speed;
        ai.jumping = true;
        if !nodes.is_empty() {
            ai.state.navigation.follow(nodes, speed, body.position);
        }
        ai::jump_from_ground(&ai, &mut body, &meadow, baby);
        body.velocity
    };
    let push = |height: f64| {
        let length = (height * height + 1.0).sqrt();
        let scale = f64::from(0.1_f32);
        (height / length * scale, 1.0 / length * scale)
    };
    let close = |a: f64, b: f64| (a - b).abs() < 1.0e-6;
    let (up, forward) = push(1.5);
    let walk = hop(1.0, false, DVec3::ZERO, Vec::new());
    assert!(
        close(walk.y, 0.3 + up) && close(walk.z, forward) && close(walk.x, 0.0),
        "{walk}"
    );
    let slow = hop(0.6, false, DVec3::ZERO, Vec::new());
    assert!(close(slow.y, 0.2 + up), "{slow}");
    let climb = hop(1.0, false, DVec3::ZERO, vec![(0, 65, 1)]);
    assert!(close(climb.y, 0.5 + up), "{climb}");
    let (baby_up, baby_forward) = push(0.5);
    let baby = hop(1.0, true, DVec3::ZERO, Vec::new());
    assert!(
        close(baby.y, 0.3 + baby_up) && close(baby.z, baby_forward),
        "{baby}"
    );
    let still = hop(0.0, false, DVec3::ZERO, Vec::new());
    assert!(
        close(still.y, 0.2) && still.z == 0.0,
        "no push without a speed: {still}"
    );
    let moving = hop(1.0, false, DVec3::new(0.1, 0.0, 0.0), Vec::new());
    assert!(
        close(moving.y, 0.3) && moving.x == 0.1 && moving.z == 0.0,
        "{moving}"
    );
}

#[test]
fn a_rabbit_faces_its_next_path_point_before_each_hop() {
    let mob = Rabbit::spawn(KIND, at(0.5, 0.5)).unwrap();
    let mut body = mob.core.body.clone();
    body.on_ground = true;
    let hop = |ai: &mut MonsterAi, jump: &mut RabbitJump, body: &Body| {
        let (mut random, mut voices, mut carrots) = (LegacyRandom::new(3), Vec::new(), 0);
        ai::custom_step(ai, jump, &mut carrots, body, &mut random, &mut voices);
        voices.len()
    };
    let mut ai = (*mob.state.ai).clone();
    let mut jump = RabbitJump {
        was_on_ground: true,
        ..RabbitJump::default()
    };
    ai.state
        .navigation
        .follow(vec![(0, 64, -3), (0, 64, -4)], 1.0, body.position);
    ai.move_control.set_wanted_position(at(6.5, 0.5), 1.0);
    assert_eq!(hop(&mut ai, &mut jump, &body), 1, "a jump sound");
    assert!(
        (ai.yaw + 180.0).abs() < 1.0e-3,
        "north, to the path: {}",
        ai.yaw
    );
    assert!(ai.jumping);
    assert_eq!((jump.duration, jump.ticks), (15, 0));
    let mut ai = (*mob.state.ai).clone();
    ai.move_control.set_wanted_position(at(6.5, 0.5), 1.0);
    let mut jump = RabbitJump {
        was_on_ground: true,
        ..RabbitJump::default()
    };
    hop(&mut ai, &mut jump, &body);
    assert!(
        (ai.yaw + 90.0).abs() < 1.0e-3,
        "east, to the wanted spot: {}",
        ai.yaw
    );
    let mut ai = (*mob.state.ai).clone();
    ai.move_control.set_wanted_position(at(6.5, 0.5), 1.0);
    let mut jump = RabbitJump {
        was_on_ground: true,
        delay: 2,
        ..RabbitJump::default()
    };
    assert_eq!(hop(&mut ai, &mut jump, &body), 0, "it waits out its delay");
    assert_eq!(jump.delay, 1);
    let mut ai = (*mob.state.ai).clone();
    ai.move_control.set_wanted_position(at(6.5, 0.5), 1.0);
    let mut jump = RabbitJump::default();
    assert_eq!(hop(&mut ai, &mut jump, &body), 0, "landing waits");
    assert_eq!(jump.delay, 10);
}

fn flees(setup: impl FnOnce(&mut EntityWorld) -> Vec<PlayerCandidate>) -> bool {
    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    let id = rabbit_at(&mut world, at(0.5, 0.5));
    let players = setup(&mut world);
    let mut fled = false;
    for _ in 0..40 {
        world.tick_with_players(&mut meadow, &players);
        fled |= rabbit(&world, id)
            .state
            .running_goals()
            .contains(&"RabbitAvoidEntityGoal");
    }
    fled
}

#[test]
fn a_rabbit_flees_players_wolves_and_monsters_but_not_slimes() {
    let player = |x: f64| player_at(1, at(x, 0.5));
    assert!(flees(|_| vec![player(6.5)]), "a player 6 blocks off");
    assert!(!flees(|_| vec![player(10.5)]), "a player 10 blocks off");
    let creative = PlayerCandidate {
        attackable: false,
        ..player(6.5)
    };
    assert!(!flees(|_| vec![creative]), "a creative player");
    let wolf = |x: f64| {
        move |world: &mut EntityWorld| {
            world.spawn_wolf(crate::wolf::Wolf::new(at(x, 0.5)), true);
            Vec::<PlayerCandidate>::new()
        }
    };
    assert!(flees(wolf(9.5)), "a wolf 9 blocks off");
    assert!(!flees(wolf(12.5)), "a wolf 12 blocks off");
    let zombie = |x: f64| {
        move |world: &mut EntityWorld| {
            world.spawn_zombie(crate::zombie::Zombie::new(at(x, 0.5)), true);
            Vec::<PlayerCandidate>::new()
        }
    };
    assert!(flees(zombie(3.5)), "a zombie 3 blocks off");
    assert!(!flees(zombie(6.5)), "a zombie 6 blocks off");
    let slime = |world: &mut EntityWorld| {
        world.spawn_slime(crate::slime::Slime::new(at(3.5, 0.5), 1), true);
        Vec::<PlayerCandidate>::new()
    };
    assert!(!flees(slime), "a slime is no monster");
}

#[test]
fn a_hurt_rabbit_panics_at_2_2() {
    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    let id = rabbit_at(&mut world, at(0.5, 0.5));
    for _ in 0..5 {
        world.tick(&mut meadow);
    }
    let fall = Hurt::new(0.5, "minecraft:fall", DamageSourceKind::Generic);
    assert!(world.hurt_family(id, &fall).unwrap().applied);
    for _ in 0..12 {
        world.tick(&mut meadow);
        assert!(!rabbit(&world, id).state.panicking(), "a fall is no panic");
    }
    let bite = Hurt::new(0.5, "minecraft:mob_attack", DamageSourceKind::MobAttack);
    assert!(world.hurt_family(id, &bite).unwrap().applied);
    let (mut panicked, mut delays) = (false, Vec::new());
    for _ in 0..60 {
        world.tick(&mut meadow);
        let mob = rabbit(&world, id);
        if mob.state.panicking() {
            panicked = true;
            assert_eq!(mob.state.jump().next_speed, ai::FLEE_SPEED);
            assert!(world.family_traits(id).unwrap().panicking);
            delays.push(mob.state.jump().delay);
        }
    }
    assert!(panicked, "a bite panics it");
    assert!(
        delays.contains(&3),
        "a panicked landing waits three ticks: {delays:?}"
    );
    assert!(!delays.contains(&10), "{delays:?}");
}

#[test]
fn a_rabbit_follows_a_player_holding_a_carrot_golden_carrot_or_dandelion() {
    for (item, follows) in [
        ("minecraft:carrot", true),
        ("minecraft:golden_carrot", true),
        ("minecraft:dandelion", true),
        ("minecraft:wheat", false),
    ] {
        let mut world = EntityWorld::default();
        let mut meadow = Meadow::default();
        let id = rabbit_at(&mut world, at(0.5, 0.5));
        let player = player_at(1, at(7.5, 0.5));
        world.set_player_controls(&[PlayerControls {
            id: 1,
            main_hand: Some(item.to_owned()),
            ..PlayerControls::default()
        }]);
        let mut tempted = false;
        for _ in 0..300 {
            world.tick_with_players(&mut meadow, &[player]);
            tempted |= rabbit(&world, id).state.tempted();
        }
        assert_eq!(tempted, follows, "{item}");
        if follows {
            let distance = rabbit(&world, id)
                .core
                .body
                .position
                .distance(player.position);
            assert!(distance < 4.0, "{item}: {distance}");
        }
    }
}

#[test]
fn a_rabbit_in_water_floats_up_and_hops_at_1_5() {
    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    for x in -3..=3 {
        for z in -3..=3 {
            for y in 58..=63 {
                meadow.water.push((x, y, z));
            }
        }
    }
    let id = rabbit_at(&mut world, DVec3::new(0.5, 59.0, 0.5));
    let mob = rabbit_mut(&mut world, id);
    mob.state.ai.retain_goals(&["FloatGoal"]);
    let position = mob.core.body.position;
    mob.state
        .ai
        .state
        .navigation
        .follow(vec![(2, 60, 0), (3, 60, 0), (4, 61, 0)], 1.0, position);
    let mut hopped = false;
    for _ in 0..20 {
        world.tick(&mut meadow);
        hopped |= !jump_pitches(&mut world).is_empty();
    }
    let mob = rabbit(&world, id);
    assert!(hopped, "its float goal makes it hop");
    assert!(
        mob.core.body.position.y > 59.5,
        "{}",
        mob.core.body.position
    );
    assert_eq!(mob.state.jump().next_speed, ai::WATER_SPEED);
    assert_eq!(mob.state.ai.move_control.speed_modifier, ai::WATER_SPEED);
}

#[derive(Default)]
struct Garden {
    blocks: HashMap<Pos, Block>,
}

fn carrots(age: i32) -> Block {
    Block::new(ai::CARROTS).with("age", &age.to_string())
}

impl Garden {
    fn with_carrot(crop: Pos, age: i32) -> Self {
        let mut garden = Self::default();
        let soil = (crop.0, crop.1 - 1, crop.2);
        garden.blocks.insert(soil, Block::new("minecraft:farmland"));
        garden.blocks.insert(crop, carrots(age));
        garden
    }

    fn age(&self, crop: Pos) -> Option<i32> {
        self.blocks
            .get(&crop)
            .filter(|block| block.id == ai::CARROTS)
            .map(crate::crops::age)
    }
}

impl World for Garden {
    fn block(&self, pos: Pos) -> Option<Block> {
        match self.blocks.get(&pos) {
            Some(block) => Some(block.clone()),
            None => (pos.1 < 64).then(|| Block::new("minecraft:grass_block")),
        }
    }

    fn set_block(&mut self, pos: Pos, block: Option<Block>) {
        match block {
            Some(block) => {
                self.blocks.insert(pos, block);
            }
            None => {
                self.blocks.remove(&pos);
            }
        }
    }

    fn collision_boxes(&self, pos: Pos) -> Vec<[f64; 6]> {
        match self.block(pos) {
            Some(block) if block.id != ai::CARROTS => authored_collision_boxes(&block),
            _ => Vec::new(),
        }
    }

    fn path_type_from_state(&self, pos: Pos) -> PathType {
        match self.block(pos) {
            Some(block) if block.id == "minecraft:farmland" => PathType::Blocked,
            Some(block) if block.id == ai::CARROTS => PathType::Open,
            block => path_type_of_block(block.as_ref()),
        }
    }

    fn pathfindable(&self, pos: Pos) -> bool {
        self.block(pos).is_none_or(|block| block.id == ai::CARROTS)
    }

    fn solid(&self, pos: Pos) -> bool {
        self.collision_boxes(pos) == [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]
    }

    fn block_in_tag(&self, pos: Pos, tag: &str) -> bool {
        let Some(block) = self.block(pos) else {
            return false;
        };
        if tag == ai::SUPPORTS_CROPS {
            return block.id == "minecraft:farmland";
        }
        authored_block_in_tag(&block, tag, &self.collision_boxes(pos))
    }
}

fn raiding(world: &EntityWorld, id: u64) -> bool {
    rabbit(world, id)
        .state
        .running_goals()
        .contains(&"RaidGardenGoal")
}

fn raider_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    let id = rabbit_at(world, position);
    rabbit_mut(world, id)
        .state
        .ai
        .retain_goals(&["RaidGardenGoal"]);
    id
}

#[test]
fn a_rabbit_walks_to_a_ripe_carrot_and_bites_it_down_a_stage() {
    let crop = (4, 64, 0);
    let centre = DVec3::new(4.5, 64.0, 0.5);
    let mut world = EntityWorld::default();
    let mut garden = Garden::with_carrot(crop, 7);
    let id = rabbit_at(&mut world, at(0.5, 0.5));
    rabbit_mut(&mut world, id).state.ai.retain_goals(&[]);
    for _ in 0..5 {
        world.tick(&mut garden);
    }
    rabbit_mut(&mut world, id)
        .state
        .ai
        .retain_goals(&["RaidGardenGoal"]);
    let (mut runs, mut walking_on) = (Vec::new(), None);
    for _ in 0..190 {
        world.tick(&mut garden);
        let running = raiding(&world, id);
        if !running && runs.contains(&true) && walking_on.is_none() {
            walking_on = Some(!rabbit(&world, id).state.ai.state.navigation.is_done());
        }
        runs.push(running);
    }
    let first = runs.iter().position(|&running| running);
    assert!(
        first.is_some_and(|tick| tick <= 1),
        "once it stands on the grass it looks for a ripe carrot at once: {first:?}"
    );
    let first = first.unwrap_or_default();
    let length = runs[first..].iter().take_while(|&&running| running).count();
    assert!(
        (1..=2).contains(&length),
        "it stops at its next full update: {length}"
    );
    assert_eq!(walking_on, Some(true), "but its walk goes on");
    let left = rabbit(&world, id).core.body.position.distance(centre);
    assert!(left < 1.5, "it hopped up to the carrot: {left}");
    assert_eq!(garden.age(crop), Some(7), "its first look only walks");

    let crop = (5, 64, 0);
    let mut world = EntityWorld::default();
    let mut garden = Garden::with_carrot(crop, 7);
    let id = raider_at(&mut world, DVec3::new(5.5, 64.0, 0.5));
    let mut bites = Vec::new();
    for tick in 0..3000 {
        if tick == 1000 {
            garden.set_block(crop, Some(carrots(7)));
        }
        let before = garden.age(crop);
        world.tick(&mut garden);
        if garden.age(crop) != before {
            assert!(raiding(&world, id), "tick {tick}");
            let goals = rabbit(&world, id).state.goals().unwrap();
            let more = goals.more_carrot_ticks;
            assert!((38..=40).contains(&more), "tick {tick}: {more}");
            bites.push((tick, garden.age(crop)));
        }
    }
    assert_eq!(bites.len(), 2, "{bites:?}");
    assert_eq!(
        bites[0],
        (0, Some(6)),
        "beside a ripe carrot it bites at once"
    );
    assert!(
        bites[1].0 > 1000,
        "an unripe carrot is left alone: {bites:?}"
    );
    assert_eq!(bites[1].1, Some(6), "until it is ripe again");

    for age in 1..=7 {
        assert_eq!(ai::nibbled(&carrots(age)), Some(carrots(age - 1)));
    }
    assert_eq!(ai::nibbled(&carrots(0)), None, "the last bite leaves air");
    let mut garden = Garden::with_carrot(crop, 7);
    let mob = Rabbit::spawn(KIND, DVec3::new(5.5, 64.0, 0.5)).unwrap();
    let mut ctx = mob.state.ai.state.clone();
    ctx.mob_griefing = true;
    let mut goal = ai::RaidGardenGoal::new();
    assert!(goal.can_start(&mut ctx, &garden));
    garden.set_block(crop, Some(carrots(0)));
    goal.start_world(&mut ctx, &garden);
    goal.tick_world(&mut ctx, &garden);
    let goals = ctx.extra.get::<RabbitGoals>().unwrap();
    assert_eq!(goals.nibble, Some((crop, None)), "a seedling is eaten up");
    assert_eq!(goals.more_carrot_ticks, ai::MORE_CARROT_TICKS);
    assert!(!goal.can_continue(&mut ctx, &garden));
}

#[test]
fn no_rabbit_touches_crops_without_mob_griefing() {
    let crop = (5, 64, 0);
    let mut world = EntityWorld::default();
    world.set_mob_griefing(false);
    let mut garden = Garden::with_carrot(crop, 7);
    let id = raider_at(&mut world, DVec3::new(5.5, 64.0, 0.5));
    for tick in 0..1200 {
        world.tick(&mut garden);
        assert!(!raiding(&world, id), "tick {tick}");
    }
    assert_eq!(garden.age(crop), Some(7));
    world.set_mob_griefing(true);
    for _ in 0..3 {
        world.tick(&mut garden);
    }
    assert_eq!(garden.age(crop), Some(6), "with the rule back it bites");
}

fn killer_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    let id = rabbit_at(world, position);
    rabbit_mut(world, id).state.set_variant(RabbitVariant::Evil);
    id
}

#[test]
fn a_killer_bunny_has_eight_armour_and_bites_for_eight() {
    let mut world = EntityWorld::default();
    let plain = rabbit_at(&mut world, at(-6.5, 0.5));
    let bunny = killer_at(&mut world, at(0.5, 0.5));
    assert_eq!(rabbit(&world, plain).state.attack_damage(), 3.0);
    assert_eq!(rabbit(&world, bunny).state.attack_damage(), 8.0);
    let traits = world.family_traits(bunny).unwrap();
    assert_eq!(traits.voice, ("rabbit", 1.0, "hostile_volume"));
    assert_eq!(traits.category, "creature");
    assert!(!traits.enemy);
    let traits = world.family_traits(plain).unwrap();
    assert_eq!(traits.voice, ("rabbit", 1.0, "neutral_volume"));
    let bite = Hurt::new(2.0, "minecraft:mob_attack", DamageSourceKind::MobAttack);
    for id in [plain, bunny] {
        assert!(world.hurt_family(id, &bite).unwrap().applied);
    }
    let health = |world: &EntityWorld, id: u64| rabbit(world, id).core.health;
    assert_eq!(health(&world, plain), 1.0);
    let armoured = 3.0 - crate::health::damage_after_armor(2.0, 8.0, 0.0);
    assert_eq!(health(&world, bunny), armoured);
    assert!((armoured - 1.56).abs() < 1.0e-5, "{armoured}");

    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    let bunny = killer_at(&mut world, at(0.5, 0.5));
    let player = player_at(1, at(5.5, 0.5));
    let (mut hits, mut sounds, mut fled, mut target) = (Vec::new(), Vec::new(), false, None);
    for _ in 0..300 {
        world.tick_with_players(&mut meadow, &[player]);
        hits.extend(world.take_player_hits());
        sounds.extend(world.take_sounds());
        let mob = rabbit(&world, bunny);
        fled |= mob.state.running_goals().contains(&"RabbitAvoidEntityGoal");
        target = target.or(mob.state.ai.state.target);
        if !hits.is_empty() {
            break;
        }
    }
    assert!(!fled, "it never runs from a player");
    assert_eq!(target, Some(Target::Player(1)));
    let hit = hits.first().expect("it bites the player");
    assert_eq!(
        (hit.player_id, hit.damage, hit.source),
        (1, 8.0, Some(bunny))
    );
    let PlayerHitKind::Melee { attacker, .. } = hit.kind else {
        panic!("a bite: {:?}", hit.kind);
    };
    let attack = sounds
        .iter()
        .find(|sound| sound.event == ATTACK_SOUND)
        .expect("its attack sound");
    assert_eq!(attack.category, "hostile_volume");
    assert!((0.8..=1.2).contains(&attack.pitch), "{}", attack.pitch);
    assert!(
        sounds
            .iter()
            .filter(|sound| sound.event == ai::JUMP_SOUND)
            .all(|sound| sound.category == "hostile_volume")
    );
    for (difficulty, taken) in [
        (Difficulty::Easy, 5.0),
        (Difficulty::Normal, 8.0),
        (Difficulty::Hard, 12.0),
    ] {
        let mut body = minecraftoss_player::Player::new(player.position);
        let incoming = IncomingHit {
            damage: hit.damage,
            from: HitFrom::Position(attacker),
            scales_with_difficulty: true,
            exhaustion: 0.1,
        };
        assert!(body.hurt_by(&incoming, difficulty, Armor::default(), 0.0));
        let health = body.survival.health;
        assert_eq!(health, 20.0 - taken, "{difficulty:?}");
    }
}

#[test]
fn a_killer_bunny_leaps_at_a_target_within_four_blocks() {
    let mut mob = Rabbit::spawn(KIND, at(0.5, 0.5)).unwrap();
    mob.state.set_variant(RabbitVariant::Evil);
    let mut body = mob.core.body.clone();
    body.on_ground = true;
    let leap = |evil: bool, x: f64, delay: i32| {
        let mut ai = (*mob.state.ai).clone();
        ai.state.extra.get_mut::<RabbitGoals>().unwrap().evil = evil;
        ai.state.difficulty = 2;
        ai.state.players = vec![player_at(1, at(x, 0.5))];
        ai.state.target = Some(Target::Player(1));
        ai.move_control.operation = crate::control::MoveOperation::Wait;
        ai.move_control.speed_modifier = ai::EVIL_ATTACK_SPEED;
        let mut jump = RabbitJump {
            was_on_ground: true,
            delay,
            ..RabbitJump::default()
        };
        let (mut random, mut voices, mut carrots) = (LegacyRandom::new(3), Vec::new(), 0);
        ai::custom_step(
            &mut ai,
            &mut jump,
            &mut carrots,
            &body,
            &mut random,
            &mut voices,
        );
        (ai, jump, voices.len())
    };
    let (ai, jump, sounds) = leap(true, 3.5, 0);
    assert_eq!(
        ai.move_control.wanted,
        at(3.5, 0.5),
        "it aims at its target"
    );
    assert!(ai.move_control.has_wanted() && ai.jumping);
    assert!((ai.yaw + 90.0).abs() < 1.0e-3, "it faces east: {}", ai.yaw);
    assert_eq!(jump.next_speed, ai::EVIL_ATTACK_SPEED);
    assert_eq!((jump.duration, jump.ticks), (ai::JUMP_DURATION, 0));
    assert_eq!(
        sounds, 2,
        "the leap and the hop that follows it each play the jump sound"
    );
    let (ai, _, sounds) = leap(true, 4.6, 0);
    assert!(!ai.move_control.has_wanted() && !ai.jumping, "too far");
    assert_eq!(sounds, 0);
    let (ai, jump, sounds) = leap(true, 3.5, 2);
    assert!(!ai.jumping && sounds == 0, "it waits out its delay");
    assert_eq!(jump.delay, 1);
    let (ai, _, sounds) = leap(false, 3.5, 0);
    assert!(!ai.jumping && sounds == 0, "a plain rabbit never leaps");
}

#[test]
fn a_killer_bunny_attacks_a_wolf_and_its_kin_join_in() {
    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    let wolf = world.spawn_wolf(crate::wolf::Wolf::new(at(3.5, 0.5)), true);
    let bunny = killer_at(&mut world, at(0.5, 0.5));
    let kin = killer_at(&mut world, at(-12.5, 12.5));
    rabbit_mut(&mut world, kin)
        .state
        .ai
        .retain_goals(&["MeleeAttackGoal"]);
    let wolf_health = |world: &EntityWorld| {
        world
            .wolves()
            .iter()
            .find(|e| e.id == wolf)
            .map_or(0.0, |e| e.wolf.health)
    };
    let full = wolf_health(&world);
    let target = |world: &EntityWorld, id: u64| rabbit(world, id).state.ai.state.target;
    for _ in 0..3 {
        world.tick(&mut meadow);
    }
    assert_eq!(target(&world, kin), None, "the wolf is too far for it");
    let bitten = Hurt::new(1.0, "minecraft:mob_attack", DamageSourceKind::MobAttack)
        .by(Some(Target::Mob(wolf)), Some("minecraft:wolf"));
    assert!(world.hurt_family(bunny, &bitten).unwrap().applied);
    let mut joined = false;
    for _ in 0..4 {
        world.tick(&mut meadow);
        joined |= target(&world, kin) == Some(Target::Mob(wolf));
    }
    assert_eq!(target(&world, bunny), Some(Target::Mob(wolf)));
    assert!(joined, "the bitten bunny calls its kin");
    let mut sounds = Vec::new();
    for _ in 0..300 {
        world.tick(&mut meadow);
        sounds.extend(world.take_sounds());
        if wolf_health(&world) < full {
            break;
        }
    }
    assert!(wolf_health(&world) <= full - 8.0, "a bite of eight");
    assert!(
        sounds
            .iter()
            .any(|sound| sound.event == ATTACK_SOUND && sound.category == "hostile_volume")
    );
    let plain = rabbit_at(&mut world, at(-4.5, 0.5));
    for _ in 0..40 {
        world.tick(&mut meadow);
    }
    assert!(
        !rabbit(&world, plain)
            .state
            .running_goals()
            .contains(&"MeleeAttackGoal"),
        "a plain rabbit never attacks"
    );
}

#[test]
fn a_wild_wolf_hunts_a_rabbit() {
    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    let id = rabbit_at(&mut world, at(0.5, 0.5));
    rabbit_mut(&mut world, id).core.no_ai = true;
    let wolf = world.spawn_wolf(crate::wolf::Wolf::new(at(6.5, 0.5)), false);
    let mut hunted = false;
    for _ in 0..400 {
        world.tick(&mut meadow);
        let target = world
            .wolves()
            .iter()
            .find(|e| e.id == wolf)
            .and_then(|e| e.ai.state.target);
        hunted |= target == Some(Target::Mob(id));
        if world
            .family_core(id)
            .is_none_or(|(_, core)| core.health < MAX_HEALTH)
        {
            break;
        }
    }
    assert!(hunted, "the wolf hunts it");
    assert!(
        world
            .family_core(id)
            .is_none_or(|(_, core)| core.health < MAX_HEALTH),
        "and bites it"
    );
}

struct Snowfield;

impl World for Snowfield {
    fn block(&self, pos: Pos) -> Option<Block> {
        (pos.1 < 64).then(|| Block::new("minecraft:snow_block"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}

    fn biome_in_tag(&self, _pos: Pos, tag: &str) -> bool {
        tag == WHITE_RABBITS_TAG
    }
}

fn holding(item: &str, count: u8) -> Inventory {
    let mut inventory = Inventory::default();
    inventory.slots[0] = Some(ItemStack::new(item, count));
    inventory
}

#[test]
fn two_fed_rabbits_have_a_baby_that_grows_up() {
    for mob_drops in [true, false] {
        let mut world = EntityWorld::default();
        world.set_mob_drops(mob_drops);
        let mut meadow = Meadow::default();
        let first = rabbit_at(&mut world, at(0.5, 0.5));
        let second = rabbit_at(&mut world, at(2.5, 0.5));
        for id in [first, second] {
            rabbit_mut(&mut world, id)
                .state
                .set_variant(RabbitVariant::Black);
        }
        let mut carrots = holding("minecraft:carrot", 4);
        for id in [first, second] {
            let (result, events, _) = feed(&mut world, id, &mut carrots, 0, false).unwrap();
            assert_eq!(result, InteractionResult::SuccessServer);
            assert_eq!(events, vec![AnimalEvent::Hearts]);
            assert_eq!(rabbit(&world, id).state.in_love, 600);
        }
        assert_eq!(carrots.slots[0].as_ref().map(|s| s.count), Some(2));
        assert_eq!(world.take_entity_events(), vec![(first, 18), (second, 18)]);
        let mut baby = None;
        for _ in 0..400 {
            world.tick(&mut meadow);
            baby = world
                .family::<Rabbit>()
                .iter()
                .map(|mob| mob.core.id)
                .find(|id| ![first, second].contains(id));
            if baby.is_some() {
                break;
            }
        }
        let baby = baby.expect("the two make a baby");
        let events = world.take_entity_events();
        assert!(
            events
                .iter()
                .any(|&(id, event)| [first, second].contains(&id) && event == 18),
            "{events:?}"
        );
        for id in [first, second] {
            let parent = &rabbit(&world, id).state;
            assert!((5999..=6000).contains(&parent.age.ticks), "{id}");
            assert_eq!(parent.in_love, 0, "{id}");
        }
        let orbs = world.take_trade_experience();
        assert_eq!(orbs.len(), usize::from(mob_drops), "{mob_drops}");
        assert!(orbs.iter().all(|(_, xp)| (1..=7).contains(xp)));
        let child = rabbit(&world, baby);
        assert!(child.core.baby && child.state.age.ticks < -23990);
        assert_eq!(
            (child.core.body.width, child.core.body.height),
            (BABY_WIDTH, BABY_HEIGHT)
        );
        let plains = [
            RabbitVariant::Brown,
            RabbitVariant::Salt,
            RabbitVariant::Black,
        ];
        assert!(
            plains.contains(&child.state.variant),
            "black like its parents, or the plains' colour: {:?}",
            child.state.variant
        );
        let before = child.state.age.ticks;
        let mut dandelions = holding("minecraft:dandelion", 1);
        let (result, _, _) = feed(&mut world, baby, &mut dandelions, 0, false).unwrap();
        assert_eq!(result, InteractionResult::SuccessPredicted);
        assert!(dandelions.slots[0].is_none(), "it ate the dandelion");
        let grown = rabbit(&world, baby).state.age.ticks - before;
        assert_eq!(grown, (-before / 20) / 10 * 20, "a tenth of the time left");
        rabbit_mut(&mut world, baby).state.age.ticks = -1;
        world.tick(&mut meadow);
        let adult = rabbit(&world, baby);
        assert!(!adult.core.baby && adult.state.age.ticks == 0, "it grew up");
        assert_eq!(
            (
                adult.core.body.width,
                adult.core.body.height,
                adult.core.eye_height
            ),
            (WIDTH, HEIGHT, EYE_HEIGHT)
        );
    }
}

#[test]
fn a_baby_takes_a_parent_s_colour_or_one_time_in_twenty_its_biome_s() {
    assert_eq!(biome_variant(79, true, false), RabbitVariant::White);
    assert_eq!(
        biome_variant(80, true, false),
        RabbitVariant::WhiteSplotched
    );
    assert_eq!(biome_variant(10, false, true), RabbitVariant::Gold);
    assert_eq!(biome_variant(49, false, false), RabbitVariant::Brown);
    assert_eq!(biome_variant(50, false, false), RabbitVariant::Salt);
    assert_eq!(biome_variant(90, false, false), RabbitVariant::Black);
    let snow = Snowfield;
    let mut counts = std::collections::HashMap::new();
    for seed in 0..600 {
        let mut world = EntityWorld::default();
        let mother = rabbit_at(&mut world, at(0.5, 0.5));
        let father = rabbit_at(&mut world, at(1.5, 0.5));
        rabbit_mut(&mut world, mother)
            .state
            .set_variant(RabbitVariant::Gold);
        rabbit_mut(&mut world, father)
            .state
            .set_variant(RabbitVariant::Black);
        rabbit_mut(&mut world, mother).core.random = LegacyRandom::new(seed);
        breed(&mut world, mother, father, &snow);
        let child = world
            .family::<Rabbit>()
            .iter()
            .find(|m| ![mother, father].contains(&m.core.id))
            .unwrap();
        assert!(child.core.baby);
        *counts.entry(child.state.variant).or_insert(0) += 1;
    }
    let count = |variant| counts.get(&variant).copied().unwrap_or(0);
    let biome = count(RabbitVariant::White) + count(RabbitVariant::WhiteSplotched);
    assert!(
        (10..=55).contains(&biome),
        "about one in twenty: {counts:?}"
    );
    assert!(
        count(RabbitVariant::Gold) > 200 && count(RabbitVariant::Black) > 200,
        "{counts:?}"
    );
    assert_eq!(
        biome + count(RabbitVariant::Gold) + count(RabbitVariant::Black),
        600
    );
}

#[test]
fn a_golden_dandelion_locks_a_baby_rabbit_s_age() {
    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    let baby = baby_at(&mut world, at(0.5, 0.5));
    rabbit_mut(&mut world, baby).state.age.ticks = -100;
    let mut dandelion = holding("minecraft:golden_dandelion", 2);
    let (result, events, _) = feed(&mut world, baby, &mut dandelion, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessPredicted);
    assert_eq!(events, vec![AnimalEvent::AgeLock]);
    let mob = rabbit(&world, baby);
    assert!(mob.state.age.locked && mob.core.persistent);
    assert_eq!(mob.state.age.ticks, Age::BABY_START);
    for _ in 0..200 {
        world.tick(&mut meadow);
    }
    assert_eq!(
        rabbit(&world, baby).state.age.ticks,
        Age::BABY_START,
        "it stays a baby"
    );
    let mut carrot = holding("minecraft:carrot", 1);
    let (result, _, _) = feed(&mut world, baby, &mut carrot, 0, false).unwrap();
    assert_eq!(
        result,
        InteractionResult::Pass,
        "a locked baby eats nothing"
    );
    let (result, events, _) = feed(&mut world, baby, &mut dandelion, 0, false).unwrap();
    assert_eq!(result, InteractionResult::SuccessPredicted);
    assert_eq!(events, vec![AnimalEvent::AgeUnlock]);
    world.tick(&mut meadow);
    assert_eq!(
        rabbit(&world, baby).state.age.ticks,
        Age::BABY_START + 1,
        "and grows again"
    );
}

#[test]
fn the_tilt_waits_180_to_219_ticks_stops_on_a_hop_and_never_runs_on_a_lead() {
    let waits: Vec<i32> = (0..400)
        .map(|x| {
            let mob = Rabbit::spawn(KIND, at(f64::from(x) + 0.5, 0.5)).unwrap();
            mob.state.animations.idle_timeout
        })
        .collect();
    assert!(waits.iter().all(|wait| (180..=219).contains(wait)));
    assert!(waits.contains(&180) && waits.contains(&219), "{waits:?}");
    let mut world = EntityWorld::default();
    let mut meadow = Meadow::default();
    let id = rabbit_at(&mut world, at(0.5, 0.5));
    let holder = rabbit_at(&mut world, at(0.5, 6.5));
    let leashed = rabbit_at(&mut world, at(2.5, 6.5));
    let still = rabbit_at(&mut world, at(0.5, -6.5));
    for rabbit in [id, holder, leashed, still] {
        rabbit_mut(&mut world, rabbit).state.ai.retain_goals(&[]);
    }
    rabbit_mut(&mut world, still).core.no_ai = true;
    world.set_leash(leashed, LeashHolder::Mob(holder));
    rabbit_mut(&mut world, leashed).state.animations.tilt = Some(0);
    let animations = |world: &EntityWorld, id: u64| rabbit(world, id).state.animations.clone();
    world.tick(&mut meadow);
    assert_eq!(
        animations(&world, leashed).tilt,
        None,
        "a lead stops a running tilt"
    );
    for _ in 1..30 {
        world.tick(&mut meadow);
    }
    assert_eq!(rabbit(&world, id).state.jump().duration, 0);
    let (wait, settled) = (
        animations(&world, id).idle_timeout,
        rabbit(&world, id).core.tick_count,
    );
    assert!(wait > 0);
    for _ in 0..wait {
        world.tick(&mut meadow);
        assert_eq!(animations(&world, id).tilt, None);
    }
    world.tick(&mut meadow);
    let started = rabbit(&world, id).core.tick_count;
    assert_eq!(
        started,
        settled + wait + 1,
        "the tilt starts once the wait runs out"
    );
    let tilt = animations(&world, id);
    assert_eq!((tilt.tilt, tilt.hop), (Some(started), None));
    assert!(
        (180..=219).contains(&tilt.idle_timeout),
        "and a new wait is drawn"
    );
    for _ in 0..120 {
        world.tick(&mut meadow);
    }
    assert_eq!(
        animations(&world, id).tilt,
        Some(started),
        "a finished tilt stays started until a hop"
    );
    let mob = rabbit_mut(&mut world, id);
    let state = &mut mob.state.ai.state;
    let walk = state.walk.clone();
    navigate_walk_to(
        &mob.core.body,
        &mut state.navigation,
        &meadow,
        &walk,
        state.fluid,
        at(14.5, 0.5),
        1.0,
        1,
    );
    let mut hopped = false;
    for _ in 0..40 {
        world.tick(&mut meadow);
        hopped = !jump_pitches(&mut world).is_empty();
        if hopped {
            break;
        }
    }
    assert!(hopped, "it hops along its path");
    assert_eq!(animations(&world, id).tilt, Some(started));
    world.tick(&mut meadow);
    let hop_start = rabbit(&world, id).core.tick_count;
    let hop = animations(&world, id);
    assert_eq!(
        (hop.hop, hop.tilt),
        (Some(hop_start), None),
        "a hop stops the tilt"
    );
    let wait = hop.idle_timeout;
    for _ in 1..15 {
        world.tick(&mut meadow);
        let hop = animations(&world, id);
        assert_eq!((hop.hop, hop.idle_timeout), (Some(hop_start), wait));
    }
    world.tick(&mut meadow);
    let landed = animations(&world, id);
    assert_eq!(landed.hop, None, "the hop stops with the jump");
    assert_eq!(landed.idle_timeout, wait - 1, "and the wait runs again");
    assert!(world.leash_holder(leashed).is_some());
    for rabbit in [leashed, still] {
        let animations = animations(&world, rabbit);
        assert_eq!(animations.tilt, None, "never on a lead or without AI");
        assert!(animations.idle_timeout < 0, "{}", animations.idle_timeout);
    }
    world.drop_leash(leashed, false);
    world.tick(&mut meadow);
    let free = animations(&world, leashed);
    assert_eq!(
        free.tilt,
        Some(rabbit(&world, leashed).core.tick_count),
        "off the lead it tilts at once"
    );
}
