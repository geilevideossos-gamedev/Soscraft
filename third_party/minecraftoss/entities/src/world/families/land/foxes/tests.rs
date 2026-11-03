use super::ai::{self, FoxGoals};
use super::*;
use crate::chicken::Chicken;
use crate::controls::PlayerControls;
use crate::look::LookControl;
use crate::monster_ai::{MobCandidate, Target};
use crate::poi::PoiType;
use crate::walk_path::WalkTarget;
use crate::world::{DamageSourceKind, LeashHolder, PlayerAttack};
use minecraftoss_player::path_type::PathType;
use minecraftoss_player::{Block, Pos};

#[derive(Default)]
struct Den {
    sky: bool,
    roof: Option<[i32; 4]>,
    water: Vec<Pos>,
    bushes: Vec<Pos>,
    thunder: bool,
    snow: Vec<Pos>,
    walls: Vec<Pos>,
    effects: Vec<Pos>,
}

impl Den {
    fn roofed(&self, x: i32, z: i32) -> bool {
        self.roof
            .is_some_and(|[x0, z0, x1, z1]| (x0..=x1).contains(&x) && (z0..=z1).contains(&z))
    }
}

impl World for Den {
    fn block(&self, pos: Pos) -> Option<Block> {
        if self.snow.contains(&pos) {
            return Some(Block::new(ai::SNOW_LAYER).with("layers", "1"));
        }
        if self.walls.contains(&pos) {
            return Some(Block::new("minecraft:stone"));
        }
        if self.water.contains(&pos) {
            return Some(Block::new("minecraft:water"));
        }
        if self.bushes.contains(&pos) {
            return Some(Block::new("minecraft:sweet_berry_bush").with("age", "3"));
        }
        if pos.1 >= 64 {
            return None;
        }
        let floor = if self.roofed(pos.0, pos.2) {
            "minecraft:stone"
        } else {
            ai::GRASS_BLOCK
        };
        Some(Block::new(floor))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}

    fn can_see_sky(&self, pos: Pos) -> bool {
        self.sky && !self.roofed(pos.0, pos.2)
    }

    fn light_path_cost(&self, pos: Pos) -> f32 {
        if self.roofed(pos.0, pos.2) { -0.5 } else { 0.5 }
    }

    fn thundering(&self) -> bool {
        self.thunder
    }

    fn collision_boxes(&self, pos: Pos) -> Vec<[f64; 6]> {
        match self.block(pos) {
            Some(block) if block.id != ai::SNOW_LAYER => {
                minecraftoss_player::authored_collision_boxes(&block)
            }
            _ => Vec::new(),
        }
    }

    fn block_in_tag(&self, pos: Pos, tag: &str) -> bool {
        let replaceable =
            |block: Block| matches!(block.id.as_str(), ai::SNOW_LAYER | "minecraft:water");
        tag == ai::REPLACEABLE && self.block(pos).is_some_and(replaceable)
    }

    fn destroy_effect(&mut self, pos: Pos) {
        self.effects.push(pos);
    }
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

fn sneaking(id: u64) -> PlayerControls {
    PlayerControls {
        id,
        sneaking: true,
        ..PlayerControls::default()
    }
}

fn fox_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    world.spawn_family_kind(KIND, position).unwrap()
}

fn cub_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    let mut mob = Fox::spawn(KIND, position).unwrap();
    mob.state.age.set(Age::BABY_START);
    set_baby(&mut mob, true);
    world.spawn_family(mob)
}

fn fox(world: &EntityWorld, id: u64) -> &FamilyMob<Fox> {
    world
        .family::<Fox>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn fox_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Fox> {
    world.family_mob_mut::<Fox>(id).unwrap()
}

fn goals(world: &EntityWorld, id: u64) -> &FoxGoals {
    fox(world, id).state.goals().unwrap()
}

fn runs(world: &EntityWorld, id: u64, goal: &str) -> bool {
    fox(world, id).state.running_goals().contains(&goal)
}

fn trust(world: &mut EntityWorld, id: u64, player: u64) {
    let uuid = world.uuid_of(PLAYER_TARGET + player);
    fox_mut(world, id).state.trust(uuid);
}

fn village(world: &mut EntityWorld, pos: Pos) {
    assert!(world.pois.add(pos, PoiType::Home));
    let home = |kind: PoiType| kind == PoiType::Home;
    assert_eq!(world.pois.take(&home, |_, _| true, pos, 48), Some(pos));
}

#[test]
fn a_fox_is_small_with_ten_health_and_its_kind() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let adult = fox_at(&mut world, at(0.5, 0.5));
    let cub = cub_at(&mut world, at(6.5, 0.5));
    assert_eq!(world.entity_type(adult), Some(KIND));
    assert_eq!(world.family_core(adult).map(|(key, _)| key), Some(49));
    let core = &fox(&world, adult).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (0.6, 0.7, 0.4, false)
    );
    assert_eq!((core.health, core.max_health), (10.0, 10.0));
    assert_eq!(core.ambient_interval, 120);
    let core = &fox(&world, cub).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (0.6_f32 * 0.6, 0.7_f32 * 0.6, 0.34375, true)
    );
    let ai = &fox(&world, cub).state.ai.state;
    assert_eq!(
        (ai.walk.width, ai.walk.height, ai.eye_height),
        (BABY_WIDTH, BABY_HEIGHT, BABY_EYE_HEIGHT)
    );
    let state = &fox(&world, adult).state;
    assert_eq!(state.variant, FoxVariant::Red);
    assert!(
        state.can_pick_up_loot,
        "the constructor lets it pick up loot"
    );
    assert_eq!((state.held.as_ref(), state.drop_chance), (None, 0.085));
    assert!(state.trusted.is_empty());
    assert_eq!((SPEED, ATTACK_DAMAGE), (0.30000001192092896, 2.0));
    for (name, variant) in [("red", FoxVariant::Red), ("snow", FoxVariant::Snow)] {
        assert_eq!(FoxVariant::from_name(name), variant);
        assert_eq!(variant.name(), name);
    }
    assert_eq!(FoxVariant::from_name("arctic"), FoxVariant::Red);
    assert_eq!((FoxVariant::Red.id(), FoxVariant::Snow.id()), (0, 1));
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
    assert_eq!(traits.voice, ("fox", 1.0, "neutral_volume"));
    assert_eq!(Fox::spared_blocks(fox(&world, adult)), SPARED_BLOCKS);
    let registered = MonsterKind::Registered(KIND);
    assert_eq!(registered.goal_names(), GOAL_NAMES);
    assert_eq!(registered.target_goal_names(), TARGET_NAMES);
    let ai = &fox(&world, adult).state.ai.state;
    assert_eq!(ai.follow_range, FOLLOW_RANGE);
    assert_eq!(ai.eye_height, EYE_HEIGHT);
    assert!(ai.walk.can_float, "its FloatGoal floats the path");
    assert_eq!(ai.walk.walk_target, WalkTarget::Animal);
    assert_eq!(ai.walk.max_visited, Some(512), "setRequiredPathLength(32)");
    for path_type in [PathType::Damaging, PathType::DamagingInNeighbor] {
        assert_eq!(ai.walk.malus(path_type), 0.0, "{path_type:?}");
    }
    assert_eq!(ai.walk.malus(PathType::Fire), -1.0);
    fox_mut(&mut world, adult).state.held = Some(ItemStack::new("minecraft:emerald", 1));
    let player = player_at(1, at(3.5, 0.5));
    let attack = PlayerAttack {
        player_id: 1,
        position: player.position,
        yaw: 0.0,
        attack_damage: 100.0,
        strength: 1.0,
        sprinting: false,
        can_critical: false,
        can_sweep: false,
    };
    for id in [adult, cub] {
        assert!(world.player_attack(&attack, id).died);
    }
    let deaths = world.take_deaths();
    let adult = deaths.iter().find(|d| d.id == adult).unwrap();
    assert_eq!(adult.table, Some(KIND), "the empty entities/fox");
    assert!((1..=3).contains(&adult.experience), "{}", adult.experience);
    let held: Vec<_> = adult
        .equipment
        .iter()
        .map(|stack| stack.id.as_str())
        .collect();
    assert_eq!(held, ["minecraft:emerald"], "the mouth item drops");
    let cub = deaths.iter().find(|d| d.id == cub).unwrap();
    assert_eq!((cub.table, cub.experience), (None, 0));
}

fn flees(setup: impl FnOnce(&mut EntityWorld, u64) -> Vec<PlayerCandidate>) -> bool {
    let mut world = EntityWorld::default();
    let mut den = Den::default();
    let id = fox_at(&mut world, at(0.5, 0.5));
    fox_mut(&mut world, id)
        .state
        .ai
        .retain_goals(&["AvoidEntityGoal"]);
    let players = setup(&mut world, id);
    let mut fled = false;
    for _ in 0..40 {
        world.tick_with_players(&mut den, &players);
        fled |= runs(&world, id, "AvoidEntityGoal");
    }
    fled
}

#[test]
fn a_fox_flees_a_walking_player_but_not_a_sneaking_or_creative_one() {
    let walker = |x: f64| move |_: &mut EntityWorld, _: u64| vec![player_at(1, at(x, 0.5))];
    assert!(flees(walker(12.5)), "a player 12 blocks off");
    assert!(!flees(walker(20.5)), "a player 20 blocks off");
    let sneaker = |world: &mut EntityWorld, _: u64| {
        world.set_player_controls(&[sneaking(1)]);
        vec![player_at(1, at(6.5, 0.5))]
    };
    assert!(!flees(sneaker), "a sneaking player");
    let creative = |_: &mut EntityWorld, _: u64| {
        vec![PlayerCandidate {
            attackable: false,
            ..player_at(1, at(6.5, 0.5))
        }]
    };
    assert!(!flees(creative), "a creative player");
    let trusted = |world: &mut EntityWorld, id: u64| {
        trust(world, id, 1);
        vec![player_at(1, at(6.5, 0.5))]
    };
    assert!(!flees(trusted), "a player it trusts");
    let defending = |world: &mut EntityWorld, id: u64| {
        fox_mut(world, id).state.set_flag(DEFENDING, true);
        vec![player_at(1, at(6.5, 0.5))]
    };
    assert!(!flees(defending), "not while it defends");

    let mut world = EntityWorld::default();
    let mut den = Den::default();
    let id = fox_at(&mut world, at(0.5, 0.5));
    let players = [player_at(1, at(5.5, 0.5))];
    let mut speeds = Vec::new();
    for _ in 0..20 {
        world.tick_with_players(&mut den, &players);
        if runs(&world, id, "AvoidEntityGoal") {
            speeds.push(fox(&world, id).state.ai.state.navigation.speed_modifier);
        }
    }
    assert_eq!(
        speeds.first(),
        Some(&ai::AVOID_SPRINT),
        "1.4 within 7 blocks"
    );
}

#[test]
fn a_fox_flees_wild_wolves_and_polar_bears_but_not_tame_wolves() {
    let wolf = |x: f64, owner: Option<u128>| {
        move |world: &mut EntityWorld, _: u64| {
            let mut wolf = crate::wolf::Wolf::new(at(x, 0.5));
            wolf.owner = owner;
            world.spawn_wolf(wolf, true);
            Vec::new()
        }
    };
    assert!(flees(wolf(6.5, None)), "a wild wolf 6 blocks off");
    assert!(!flees(wolf(10.5, None)), "a wild wolf 10 blocks off");
    assert!(!flees(wolf(4.5, Some(7))), "a tame wolf");
    let bear = |x: f64| {
        move |world: &mut EntityWorld, _: u64| {
            let bear = world
                .spawn_family_kind("minecraft:polar_bear", at(x, 0.5))
                .unwrap();
            world.family_core_mut(bear).unwrap().1.no_ai = true;
            Vec::new()
        }
    };
    assert!(flees(bear(7.5)), "a polar bear 7 blocks off");
    assert!(!flees(bear(11.5)), "a polar bear 11 blocks off");
    let cow = |world: &mut EntityWorld, _: u64| {
        world.spawn_cow(crate::cow::Cow::new(at(4.5, 0.5)), true);
        Vec::new()
    };
    assert!(!flees(cow), "a cow");
}

fn asleep_after(world: &mut EntityWorld, den: &mut Den, id: u64, ticks: usize) -> Option<usize> {
    (1..=ticks).find(|_| {
        world.tick(den);
        fox(world, id).state.sleeping()
    })
}

#[test]
fn a_fox_sleeps_by_day_under_cover_and_wakes_when_something_walks_near() {
    let mut world = EntityWorld::default();
    let mut den = Den::default();
    world.set_bright_outside(true);
    let id = fox_at(&mut world, at(0.5, 0.5));
    let slept = asleep_after(&mut world, &mut den, id, 200).expect("it falls asleep");
    assert!(slept <= 2 * 70 + 2, "{slept}");
    assert!(runs(&world, id, "SleepGoal"));
    let resting = fox(&world, id).core.body.position;
    for _ in 0..100 {
        world.tick(&mut den);
    }
    let mob = fox(&world, id);
    assert!(mob.state.sleeping());
    assert_eq!(mob.core.body.position, resting, "it lies still");
    assert!(mob.state.ai.state.navigation.is_done());

    world.set_player_controls(&[sneaking(1)]);
    let player = player_at(1, at(4.5, 0.5));
    for _ in 0..60 {
        world.tick_with_players(&mut den, &[player]);
    }
    assert!(fox(&world, id).state.sleeping(), "a sneaking player");
    world.set_player_controls(&[]);
    trust(&mut world, id, 1);
    for _ in 0..60 {
        world.tick_with_players(&mut den, &[player]);
    }
    assert!(fox(&world, id).state.sleeping(), "a player it trusts");
    let stranger = player_at(2, at(8.5, 0.5));
    world.tick_with_players(&mut den, &[stranger]);
    world.tick_with_players(&mut den, &[stranger]);
    assert!(!fox(&world, id).state.sleeping(), "a stranger walking near");

    let mut world = EntityWorld::default();
    world.set_bright_outside(true);
    let id = fox_at(&mut world, at(0.5, 0.5));
    asleep_after(&mut world, &mut den, id, 200).expect("it falls asleep");
    world.spawn_cow(crate::cow::Cow::new(at(10.5, 0.5)), false);
    world.tick(&mut den);
    world.tick(&mut den);
    assert!(!fox(&world, id).state.sleeping(), "a cow within 12 blocks");

    let mut world = EntityWorld::default();
    world.set_bright_outside(true);
    let id = fox_at(&mut world, at(0.5, 0.5));
    let mut tame = crate::wolf::Wolf::new(at(6.5, 0.5));
    tame.owner = Some(7);
    world.spawn_wolf(tame, true);
    assert!(
        asleep_after(&mut world, &mut den, id, 200).is_some(),
        "a tame wolf"
    );

    let mut world = EntityWorld::default();
    let id = fox_at(&mut world, at(0.5, 0.5));
    assert_eq!(
        asleep_after(&mut world, &mut den, id, 400),
        None,
        "at night"
    );
    let mut open = Den {
        sky: true,
        ..Den::default()
    };
    let mut world = EntityWorld::default();
    world.set_bright_outside(true);
    let id = fox_at(&mut world, at(0.5, 0.5));
    village(&mut world, (0, 64, 0));
    assert_eq!(
        asleep_after(&mut world, &mut open, id, 400),
        None,
        "under open sky"
    );
}

#[test]
fn a_fox_seeks_shelter_from_the_sun_but_not_in_a_village() {
    let seeks = |bright: bool, in_village: bool| {
        let mut den = Den {
            sky: true,
            roof: Some([3, -3, 8, 3]),
            ..Den::default()
        };
        let mut world = EntityWorld::default();
        world.set_bright_outside(bright);
        let id = fox_at(&mut world, at(0.5, 0.5));
        if in_village {
            village(&mut world, (0, 64, 0));
        }
        let mut sought = false;
        for _ in 0..800 {
            world.tick(&mut den);
            sought |= runs(&world, id, "SeekShelterGoal");
        }
        sought
    };
    assert!(seeks(true, false), "by day under open sky");
    assert!(!seeks(true, true), "not in a village");
    assert!(!seeks(false, false), "not at night");

    let den = Den {
        sky: true,
        roof: Some([3, -3, 8, 3]),
        ..Den::default()
    };
    let body = crate::movement::Body::new(at(5.5, 0.5), WIDTH, HEIGHT);
    assert_eq!(walk_value(&den, (5, 64, 0)), -0.5, "dark stone");
    assert_eq!(walk_value(&den, (0, 64, 0)), 10.0, "grass");
    assert!(!has_shelter(&den, &body), "too dark to sleep on stone");
    let grass = Den::default();
    assert!(has_shelter(&grass, &body), "covered grass");
}

#[test]
fn a_fox_wakes_in_a_thunderstorm_and_seeks_cover_under_open_sky() {
    let mut den = Den {
        thunder: true,
        ..Den::default()
    };
    let mut world = EntityWorld::default();
    world.set_bright_outside(true);
    let id = fox_at(&mut world, at(0.5, 0.5));
    fox_mut(&mut world, id).state.set_flag(SLEEPING, true);
    world.tick(&mut den);
    assert!(!fox(&world, id).state.sleeping(), "thunder wakes it");
    for _ in 0..200 {
        world.tick(&mut den);
        assert!(!fox(&world, id).state.sleeping(), "nor can it sleep");
    }

    let shelters = |sky: bool| {
        let mut den = Den {
            sky,
            roof: Some([3, -3, 8, 3]),
            thunder: true,
            ..Den::default()
        };
        let mut world = EntityWorld::default();
        let id = fox_at(&mut world, at(0.5, 0.5));
        village(&mut world, (0, 64, 0));
        (0..40).any(|_| {
            world.tick(&mut den);
            runs(&world, id, "SeekShelterGoal")
        })
    };
    assert!(shelters(true), "at night, in a village, at once");
    assert!(!shelters(false), "not under cover");
}

#[test]
fn a_fox_wakes_and_stands_up_in_water() {
    let mut den = Den {
        water: vec![(0, 64, 0)],
        ..Den::default()
    };
    let mut world = EntityWorld::default();
    let id = fox_at(&mut world, at(0.5, 0.5));
    let wet = fox_at(&mut world, at(10.5, 0.5));
    den.water.push((10, 64, 0));
    let dry = fox_at(&mut world, at(20.5, 0.5));
    for id in [id, wet, dry] {
        let state = &mut fox_mut(&mut world, id).state;
        state.set_flag(SLEEPING, true);
        state.set_flag(SITTING, true);
    }
    fox_mut(&mut world, wet).state.set_flag(SLEEPING, false);
    world.tick(&mut den);
    for id in [id, wet] {
        let state = &fox(&world, id).state;
        assert!(!state.sleeping() && !state.sitting(), "in water");
    }
    let state = &fox(&world, dry).state;
    assert!(
        state.sleeping() && !state.sitting(),
        "asleep it does not sit"
    );
}

#[test]
fn a_fox_perches_and_looks_around_two_to_four_times() {
    let mut den = Den::default();
    let mut world = EntityWorld::default();
    let id = fox_at(&mut world, at(0.5, 0.5));
    let mut sessions = Vec::new();
    let mut started: Option<(usize, DVec3, f32)> = None;
    let mut turned = false;
    for _ in 0..200 {
        world.tick(&mut den);
    }
    for tick in 0..6000 {
        world.tick(&mut den);
        let mob = fox(&world, id);
        let perching = runs(&world, id, "PerchAndSearchGoal");
        match (perching, started) {
            (true, None) => {
                assert!(mob.state.sitting(), "it sits down");
                assert!(mob.state.ai.state.navigation.is_done());
                started = Some((tick, mob.core.body.position, mob.core.look.head_yaw));
            }
            (true, Some((_, at, yaw))) => {
                assert_eq!(mob.core.body.position.x, at.x, "it stays put");
                turned |= (mob.core.look.head_yaw - yaw).abs() > 5.0;
            }
            (false, Some((start, _, _))) => {
                assert!(!mob.state.sitting(), "it gets up");
                sessions.push(tick - start);
                started = None;
            }
            (false, None) => {}
        }
    }
    assert!(sessions.len() >= 3, "{sessions:?}");
    for length in &sessions {
        assert!((150..=410).contains(length), "{sessions:?}");
    }
    assert!(turned, "it looks about");

    let mut world = EntityWorld::default();
    let id = fox_at(&mut world, at(0.5, 0.5));
    world.spawn_cow(crate::cow::Cow::new(at(6.5, 0.5)), true);
    for _ in 0..3000 {
        world.tick(&mut den);
        assert!(!runs(&world, id, "PerchAndSearchGoal"), "not when alert");
    }
}

#[test]
fn a_hurt_fox_panics_and_falls_out_of_love() {
    let mut den = Den::default();
    let punch = Hurt::new(
        1.0,
        "minecraft:player_attack",
        DamageSourceKind::PlayerAttack,
    )
    .by(Some(Target::Player(1)), Some("minecraft:player"));
    let mut world = EntityWorld::default();
    let id = fox_at(&mut world, at(0.5, 0.5));
    let calm = fox_at(&mut world, at(10.5, 0.5));
    for id in [id, calm] {
        fox_mut(&mut world, id).state.in_love = 600;
    }
    fox_mut(&mut world, calm).state.set_flag(DEFENDING, true);
    for id in [id, calm] {
        assert!(world.hurt_family(id, &punch).unwrap().applied);
        assert_eq!(fox(&world, id).state.in_love, 0, "out of love");
    }
    let mut panicked = false;
    for _ in 0..20 {
        world.tick(&mut den);
        let running = runs(&world, id, "FoxPanicGoal");
        panicked |= running;
        assert_eq!(world.family_traits(id).unwrap().panicking, running);
        assert!(!runs(&world, calm, "FoxPanicGoal"), "not while it defends");
    }
    assert!(panicked);
}

#[test]
fn a_fox_screeches_at_night_only_with_no_player_within_sixteen() {
    let count = |sleeping: bool, bright: bool, players: &[PlayerCandidate]| {
        let mut world = EntityWorld::default();
        let id = fox_at(&mut world, at(0.5, 0.5));
        let mob = fox_mut(&mut world, id);
        mob.state.set_flag(SLEEPING, sleeping);
        let mut voices = Vec::new();
        for _ in 0..2000 {
            mob.core
                .voices
                .push((Voice::Ambient(1.1), mob.core.body.position));
            ambient(mob, bright, players);
            voices.extend(mob.core.voices.drain(..).map(|(voice, _)| voice));
        }
        voices
    };
    let screeches = |voices: &[Voice]| {
        voices
            .iter()
            .filter(|voice| **voice == Voice::Event(SCREECH_SOUND, 2.0, 1.1))
            .count()
    };
    let night = count(false, false, &[]);
    let screamed = screeches(&night);
    assert!((140..=260).contains(&screamed), "{screamed} of 2000");
    let plain = night
        .iter()
        .filter(|voice| **voice == Voice::Ambient(1.1))
        .count();
    assert_eq!(plain + screamed, 2000);
    assert_eq!(screeches(&count(false, true, &[])), 0, "not by day");
    let near = [player_at(1, at(15.0, 0.5))];
    assert_eq!(
        screeches(&count(false, false, &near)),
        0,
        "a player within 16"
    );
    let far = [player_at(1, at(18.0, 0.5))];
    assert!(
        screeches(&count(false, false, &far)) > 100,
        "a player 18 off"
    );
    let spectator = [PlayerCandidate {
        spectator: true,
        ..player_at(1, at(2.0, 0.5))
    }];
    assert!(screeches(&count(false, false, &spectator)) > 100);
    let asleep = count(true, false, &[]);
    assert!(
        asleep
            .iter()
            .all(|voice| *voice == Voice::Event(SLEEP_SOUND, 1.0, 1.1))
    );
}

#[test]
fn a_fox_walks_through_sweet_berry_bushes_unhurt_and_unslowed() {
    let mut den = Den {
        bushes: (0..=3).map(|z| (0, 64, z)).collect(),
        ..Den::default()
    };
    let mut world = EntityWorld::default();
    let fox_in = fox_at(&mut world, at(0.5, 0.5));
    let fox_out = fox_at(&mut world, at(20.5, 0.5));
    for _ in 0..5 {
        for id in [fox_in, fox_out] {
            let body = &mut fox_mut(&mut world, id).core.body;
            body.velocity.z = 0.2;
            body.velocity.x = 0.0;
        }
        world.tick(&mut den);
        let (inside, outside) = (fox(&world, fox_in), fox(&world, fox_out));
        assert_eq!(inside.core.body.stuck, None, "not held");
        assert_eq!(inside.core.health, MAX_HEALTH, "not pricked");
        assert_eq!(
            inside.core.body.position.z - 0.5,
            outside.core.body.position.z - 0.5,
            "as fast as on grass"
        );
    }
    assert!(fox(&world, fox_in).core.body.position.z > 1.2);
}

#[test]
fn a_fox_takes_no_damage_from_a_five_block_fall() {
    let mut den = Den::default();
    let mut world = EntityWorld::default();
    let five = fox_at(&mut world, DVec3::new(0.5, 69.0, 0.5));
    let eight = fox_at(&mut world, DVec3::new(10.5, 72.0, 0.5));
    let six = fox_at(&mut world, DVec3::new(20.5, 70.0, 0.5));
    for _ in 0..60 {
        world.tick(&mut den);
    }
    assert_eq!(fox(&world, five).core.health, MAX_HEALTH, "five blocks");
    assert_eq!(fox(&world, six).core.health, MAX_HEALTH - 1.0, "six blocks");
    assert_eq!(
        fox(&world, eight).core.health,
        MAX_HEALTH - 3.0,
        "eight blocks"
    );
    assert_eq!(SAFE_FALL_DISTANCE - 3.0, ai::SAFE_FALL_BONUS);
}

#[test]
fn a_fox_cub_follows_its_parent() {
    let mut den = Den::default();
    let mut world = EntityWorld::default();
    let cub = cub_at(&mut world, at(0.5, 0.5));
    let parent = fox_at(&mut world, at(7.5, 0.5));
    world.family_core_mut(parent).unwrap().1.no_ai = true;
    let mut followed = false;
    for _ in 0..600 {
        world.tick(&mut den);
        followed |= runs(&world, cub, "FoxFollowParentGoal");
    }
    assert!(followed);
    let gap = fox(&world, cub)
        .core
        .body
        .position
        .distance(fox(&world, parent).core.body.position);
    assert!(gap < 5.0, "{gap}");
}

#[test]
fn a_leashed_fox_is_pulled() {
    let mut den = Den::default();
    let mut world = EntityWorld::default();
    let id = fox_at(&mut world, at(0.5, 0.5));
    let holder = fox_at(&mut world, at(9.5, 0.5));
    world.family_core_mut(holder).unwrap().1.no_ai = true;
    assert!(world.can_be_leashed(id));
    world.set_leash(id, LeashHolder::Mob(holder));
    for _ in 0..60 {
        world.tick(&mut den);
    }
    assert_eq!(world.leash_holder(id), Some(LeashHolder::Mob(holder)));
    let x = fox(&world, id).core.body.position.x;
    assert!(x > 2.5, "pulled towards its holder: {x}");
}

#[test]
fn a_sleeping_fox_holds_its_head_and_a_faceplanted_one_its_pitch() {
    let mut look = LookControl::new(0.0);
    look.pitch = 30.0;
    look.tick_from(at(0.5, 0.5), 0.4, 0.0, false, 75.0, false);
    assert_eq!(look.pitch, 30.0, "kept");
    look.tick_limited(at(0.5, 0.5), 0.4, 0.0, false, 75.0);
    assert_eq!(look.pitch, 0.0, "reset");

    let mut den = Den::default();
    let mut world = EntityWorld::default();
    world.set_bright_outside(true);
    let asleep = fox_at(&mut world, at(0.5, 0.5));
    asleep_after(&mut world, &mut den, asleep, 200).expect("it falls asleep");
    fox_mut(&mut world, asleep)
        .state
        .ai
        .state
        .look_control
        .head_yaw = 50.0;
    for _ in 0..10 {
        world.tick(&mut den);
    }
    let mob = fox(&world, asleep);
    assert!(mob.state.sleeping());
    assert_eq!(mob.core.look.head_yaw, 50.0, "no look tick asleep");

    let mut world = EntityWorld::default();
    let planted = fox_at(&mut world, at(10.5, 0.5));
    let plain = fox_at(&mut world, at(20.5, 0.5));
    fox_mut(&mut world, planted)
        .state
        .set_flag(FACEPLANTED, true);
    for id in [planted, plain] {
        let ai = &mut fox_mut(&mut world, id).state.ai;
        ai.retain_goals(&[]);
        ai.state.look_control.pitch = 25.0;
    }
    for _ in 0..10 {
        world.tick(&mut den);
    }
    let pitch = |id: u64| fox(&world, id).core.look.pitch;
    assert_eq!(pitch(planted), 25.0, "a faceplant keeps its pitch");
    assert_eq!(pitch(plain), 0.0);
    assert!(!goals(&world, planted).can_move(), "held still");
}

#[test]
fn a_fox_strolls_through_a_village_only_at_night() {
    let strolls = |bright: bool| {
        let mut den = Den::default();
        let mut world = EntityWorld::default();
        world.set_bright_outside(bright);
        let id = fox_at(&mut world, at(0.5, 0.5));
        village(&mut world, (40, 64, 0));
        (0..4000).any(|_| {
            world.tick(&mut den);
            runs(&world, id, "FoxStrollThroughVillageGoal")
        })
    };
    assert!(strolls(false), "at night");
    assert!(!strolls(true), "by day");
}

fn hunter_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    let id = fox_at(world, position);
    fox_mut(world, id).state.set_target_goals();
    id
}

fn chicken_at(world: &mut EntityWorld, position: DVec3) -> u64 {
    world.spawn_chicken(Chicken::new(position), true)
}

fn chicken_health(world: &EntityWorld, id: u64) -> f32 {
    let chicken = world.chickens().iter().find(|e| e.id == id);
    chicken.map_or(0.0, |e| e.chicken.health)
}

fn chicken_position(world: &EntityWorld, id: u64) -> DVec3 {
    let chicken = world.chickens().iter().find(|e| e.id == id).unwrap();
    chicken.chicken.body.position
}

fn crouched(world: &mut EntityWorld, id: u64, prey: u64) {
    let state = &mut fox_mut(world, id).state;
    state.set_flag(CROUCHING, true);
    state.set_flag(INTERESTED, true);
    state.crouch_amount = MAX_CROUCH_AMOUNT;
    state.crouch_amount_old = MAX_CROUCH_AMOUNT;
    state.ai.state.target = Some(Target::Mob(prey));
}

fn bites(world: &mut EntityWorld, id: u64) -> usize {
    let voices = &mut fox_mut(world, id).core.voices;
    let bite = Voice::Event(BITE_SOUND, 1.0, 1.0);
    let heard = voices.iter().filter(|(voice, _)| *voice == bite).count();
    voices.clear();
    heard
}

#[test]
fn a_red_fox_stalks_a_chicken_crouches_within_six_and_pounces_when_fully_crouched() {
    let mut den = Den::default();
    let mut world = EntityWorld::default();
    let id = hunter_at(&mut world, at(0.5, 0.5));
    let idle = fox_at(&mut world, at(0.5, 16.5));
    let chicken = chicken_at(&mut world, at(14.5, 0.5));
    let mut stalked = false;
    let mut crouched_at = None;
    let mut pounced_at = None;
    for tick in 0..400 {
        world.tick(&mut den);
        stalked |= runs(&world, id, "StalkPreyGoal");
        let mob = fox(&world, id);
        let gap = mob
            .core
            .body
            .position
            .distance(chicken_position(&world, chicken));
        if crouched_at.is_none() && mob.state.flag(CROUCHING) {
            assert!(stalked, "it stalks before it crouches");
            assert!(mob.state.flag(INTERESTED), "its head cocked");
            assert!(gap <= ai::STALK_DISTANCE + 0.5, "{gap}");
            crouched_at = Some(tick);
        }
        if mob.state.flag(POUNCING) {
            assert_eq!(mob.state.crouch_amount, MAX_CROUCH_AMOUNT);
            let velocity = mob.core.body.velocity;
            assert!(velocity.y > 0.6 && velocity.x > 0.4, "{velocity}");
            pounced_at = Some(tick);
            break;
        }
        let idle = fox(&world, idle);
        assert_eq!(idle.state.ai.state.target, None, "no prey goals, no prey");
        assert!(!idle.state.flag(CROUCHING));
    }
    let crouched_at = crouched_at.expect("it crouches");
    let pounced_at = pounced_at.expect("it pounces");
    let waited = pounced_at - crouched_at;
    assert!((25..=30).contains(&waited), "{crouched_at} {pounced_at}");
}

#[test]
fn a_pounce_that_lands_within_two_bites() {
    let mut den = Den::default();
    let mut world = EntityWorld::default();
    let id = hunter_at(&mut world, at(0.5, 0.5));
    let chicken = chicken_at(&mut world, at(6.5, 0.5));
    world.tick(&mut den);
    crouched(&mut world, id, chicken);
    let mut bitten = None;
    for tick in 0..60 {
        world.tick(&mut den);
        assert_eq!(bites(&mut world, id), 0, "a pounce bites in silence");
        if chicken_health(&world, chicken) < 4.0 {
            let mob = fox(&world, id);
            let gap = mob
                .core
                .body
                .position
                .distance(chicken_position(&world, chicken));
            assert!(mob.state.flag(POUNCING), "the pounce bites");
            assert!(gap <= 2.5, "{gap}");
            bitten = Some(tick);
            break;
        }
    }
    let tick = bitten.expect("the chicken is bitten");
    assert!(tick > 5, "after a flight: {tick}");
    assert_eq!(chicken_health(&world, chicken), 2.0);
}

#[test]
fn a_pounce_into_snow_faceplants_for_forty_ticks_with_break_effects() {
    let field = (-2..=16).flat_map(|x| (-3..=3).map(move |z| (x, 64, z)));
    let mut den = Den {
        snow: field.collect(),
        ..Den::default()
    };
    let mut world = EntityWorld::default();
    let id = hunter_at(&mut world, at(0.5, 0.5));
    let chicken = chicken_at(&mut world, at(14.5, 0.5));
    world.tick(&mut den);
    crouched(&mut world, id, chicken);
    let mut planted: Option<(usize, usize)> = None;
    let mut resting = None;
    let mut lasted = None;
    for tick in 0..160 {
        world.tick(&mut den);
        let mob = fox(&world, id);
        let (start, effects) = match (mob.state.flag(FACEPLANTED), planted) {
            (true, None) => *planted.insert((tick, den.effects.len())),
            (true, Some(planted)) => planted,
            (false, Some((start, _))) => {
                lasted = Some(tick - start);
                break;
            }
            (false, None) => continue,
        };
        if tick == start {
            assert_eq!(mob.state.ai.state.target, None, "it drops its prey");
        }
        if tick > start + 1 {
            assert!(runs(&world, id, "FaceplantGoal"));
            assert!(!mob.state.flag(POUNCING));
        }
        if tick == start + 10 {
            resting = Some(mob.core.body.position);
        }
        if let Some(resting) = resting {
            assert_eq!(mob.core.body.position, resting, "head down, it stays");
        }
        let block = ai::block_of(mob.core.body.position);
        assert_eq!(block.1, 64, "in the snow layer");
        assert!(den.effects[effects..].iter().all(|pos| *pos == block));
    }
    let lasted = lasted.expect("it gets up again");
    assert!((40..=42).contains(&lasted), "{lasted}");
    let effects = den.effects.len() - planted.unwrap().1;
    assert!(
        (2..=20).contains(&effects),
        "about one tick in five: {effects}"
    );
    assert_eq!(chicken_health(&world, chicken), 4.0, "the chicken got away");
    let mob = fox(&world, id);
    assert!(!mob.state.flag(CROUCHING) && !mob.state.flag(POUNCING));
    assert_eq!(mob.state.crouch_amount, 0.0);
}

#[test]
fn a_snow_fox_puts_fish_first() {
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
    let hunt = |variant: Option<FoxVariant>| {
        let mut mob = Fox::spawn(KIND, at(0.5, 0.5)).unwrap();
        if let Some(variant) = variant {
            mob.state.variant = variant;
            mob.state.set_target_goals();
        }
        let mut ai = (*mob.state.ai).clone();
        ai.state.body.on_ground = true;
        ai.state.mobs = vec![
            candidate(7, "minecraft:chicken", 8.5),
            candidate(8, "minecraft:cod", 10.5),
            candidate(9, "minecraft:pufferfish", 3.5),
        ];
        let den = Den::default();
        let mut target = None;
        for tick in 0..300 {
            ai.state.game_time = tick;
            ai.tick_goals(&den, true);
            target = ai.state.target;
        }
        target
    };
    let red = hunt(Some(FoxVariant::Red));
    assert_eq!(red, Some(Target::Mob(7)), "red");
    let snow = hunt(Some(FoxVariant::Snow));
    assert_eq!(snow, Some(Target::Mob(8)), "snow");
    assert_eq!(hunt(None), None, "a fox never loaded hunts nothing");

    let mut mob = Fox::spawn(KIND, at(0.5, 0.5)).unwrap();
    mob.state.set_target_goals();
    mob.state.variant = FoxVariant::Snow;
    mob.state.set_target_goals();
    let hunts = mob.state.goals().unwrap().hunts;
    assert_eq!(hunts, Some(FoxVariant::Red), "the first call sets them");
}

#[test]
fn a_blocked_pounce_paths_instead() {
    let open = Den::default();
    let (from, to) = (at(0.5, 0.5), at(6.5, 0.5));
    assert!(ai::is_path_clear(&open, from, to));
    let walled = |pos: Pos| Den {
        walls: vec![pos],
        ..Den::default()
    };
    for y in [65, 66, 67] {
        assert!(!ai::is_path_clear(&walled((3, y, 0)), from, to), "{y}");
    }
    for y in [64, 68] {
        assert!(ai::is_path_clear(&walled((3, y, 0)), from, to), "{y}");
    }
    let snowed = Den {
        snow: vec![(3, 65, 0)],
        ..Den::default()
    };
    assert!(ai::is_path_clear(&snowed, from, to), "snow is replaceable");
    let diagonal = at(6.5, 6.5);
    assert!(!ai::is_path_clear(&walled((3, 65, 3)), from, diagonal));
    let ahead = at(0.5, 6.5);
    assert!(!ai::is_path_clear(&walled((0, 66, 4)), from, ahead));
    assert!(ai::is_path_clear(&walled((1, 66, 4)), from, ahead));

    let mut den = walled((3, 65, 0));
    let mut world = EntityWorld::default();
    let id = hunter_at(&mut world, from);
    let chicken = chicken_at(&mut world, to);
    world.tick(&mut den);
    crouched(&mut world, id, chicken);
    world.tick(&mut den);
    world.tick(&mut den);
    let mob = fox(&world, id);
    assert!(!mob.state.flag(CROUCHING) && !mob.state.flag(INTERESTED));
    let mut melee = false;
    let mut heard = 0;
    for _ in 0..300 {
        world.tick(&mut den);
        assert!(!fox(&world, id).state.flag(POUNCING), "no pounce");
        melee |= runs(&world, id, "FoxMeleeAttackGoal");
        heard += bites(&mut world, id);
        if chicken_health(&world, chicken) <= 0.0 {
            break;
        }
    }
    assert!(melee, "it runs at its prey");
    assert!(chicken_health(&world, chicken) <= 0.0, "and bites it dead");
    assert_eq!(heard, 2, "two bites of two for a chicken of four");
}
