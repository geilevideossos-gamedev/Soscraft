use super::*;
use crate::world::families::water::abstract_fish::{
    AVOID_SPRINT_SPEED, AVOID_WALK_SPEED, PANIC_SPEED, look_for_water,
};
use crate::world::families::water::swim::FISH_SPEED_LERP;
use crate::world::families::{Despawn, category};
use crate::world::{MobSound, PlayerAttack};
use minecraftoss_player::{Block, Pos};
use std::collections::HashSet;

struct Tank {
    size: i32,
    top: i32,
}

impl World for Tank {
    fn block(&self, (x, y, z): Pos) -> Option<Block> {
        if y > self.top {
            return None;
        }
        let inside = |c: i32| (0..self.size).contains(&c);
        let water = y >= 64 && inside(x) && inside(z);
        Some(Block::new(if water {
            "minecraft:water"
        } else {
            "minecraft:stone"
        }))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}

    fn step_sound(&self, pos: Pos) -> Option<(String, f32, f32)> {
        let stone = self
            .block(pos)
            .is_some_and(|block| block.id == "minecraft:stone");
        stone.then(|| ("block.stone.step".to_owned(), 1.0, 1.0))
    }
}

const LAND: Tank = Tank { size: 0, top: 63 };

#[derive(Default)]
struct Shore {
    water: HashSet<Pos>,
    stone: HashSet<Pos>,
}

impl World for Shore {
    fn block(&self, pos: Pos) -> Option<Block> {
        if self.water.contains(&pos) {
            Some(Block::new("minecraft:water"))
        } else if pos.1 < 64 || self.stone.contains(&pos) {
            Some(Block::new("minecraft:stone"))
        } else {
            None
        }
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

fn fish_at(world: &mut EntityWorld, kind: &str, at: DVec3) -> u64 {
    world.spawn_family_kind(kind, at).unwrap()
}

fn fish(world: &EntityWorld, id: u64) -> &FamilyMob<Fish> {
    world
        .family::<Fish>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn fish_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Fish> {
    world.family_mob_mut::<Fish>(id).unwrap()
}

fn size_of(mob: &FamilyMob<Fish>) -> (f32, f32, f32) {
    let core = &mob.core;
    (core.body.width, core.body.height, core.eye_height)
}

fn heard(sounds: &[MobSound], event: &str) -> usize {
    sounds.iter().filter(|sound| sound.event == event).count()
}

#[test]
fn a_cod_is_half_a_block_long_with_three_health_and_its_kind() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let kinds = [
        (COD, (0.5, 0.3, 0.195), "cod"),
        (SALMON, (0.7, 0.4, 0.26), "salmon"),
        (TROPICAL_FISH, (0.5, 0.4, 0.26), "tropical_fish"),
        (PUFFERFISH, (0.35, 0.35, 0.455 * 0.5), "puffer_fish"),
    ];
    let mut ids = Vec::new();
    for (i, (kind, size, family)) in kinds.into_iter().enumerate() {
        let id = fish_at(
            &mut world,
            kind,
            DVec3::new(2.5 + 4.0 * i as f64, 66.0, 2.5),
        );
        ids.push((id, kind));
        assert_eq!(world.entity_type(id), Some(kind));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(65));
        let mob = fish(&world, id);
        assert_eq!(size_of(mob), size, "{kind}");
        assert_eq!(mob.state.variant.kind(), kind);
        let core = &mob.core;
        assert_eq!((core.health, core.max_health), (3.0, 3.0));
        assert_eq!((core.baby, core.ambient_interval), (false, 120));
        assert!(!mob.state.from_bucket);
        assert_eq!(Fish::breath(mob), Breath::Gills);
        assert_eq!(
            Fish::despawn(mob),
            Despawn {
                distance: 64.0,
                removable: true,
                peaceful_ok: true,
                kept: false,
            }
        );
        let ai = &mob.state.ai;
        assert_eq!(
            ai.state.navigation.water,
            WaterNav::Bound { breaching: false }
        );
        assert_eq!(
            (ai.state.walk.width, ai.state.walk.height),
            (size.0, size.1)
        );
        assert_eq!(ai.state.walk.follow_range, 16.0);
        assert!(!ai.state.walk.can_float);
        assert_eq!(category(65, kind), Some("water_ambient"));
        let traits = world.family_traits(id).unwrap();
        assert!(traits.breathes_water && traits.pushable, "{kind}");
        assert!(!traits.leashable && !traits.enemy && !traits.undead && !traits.fire_immune);
        assert_eq!(traits.voice, (family, 1.0, "neutral_volume"));
    }
    assert_eq!(spec_of(COD).map(|spec| spec.eye_height), Some(0.195));
    assert_eq!(spec_of(PUFFERFISH).map(|spec| spec.eye_height), Some(0.455));
    assert!(spec_of("minecraft:squid").is_none());
    let census = world.census();
    for (id, kind) in &ids {
        assert_eq!(census.iter().filter(|c| c.kind == *kind).count(), 1);
        let candidates = world.mob_candidates();
        assert!(candidates.iter().any(|m| m.id == *id && m.kind == *kind));
    }
    for (id, _) in &ids {
        let player = DVec3::new(8.5, 66.0, 6.5);
        assert!(world.player_attack(&attack(1, player, 100.0), *id).died);
    }
    let deaths = world.take_deaths();
    for (id, kind) in ids {
        let death = deaths.iter().find(|d| d.id == id).unwrap();
        assert_eq!(death.table, Some(kind));
        assert!((1..=3).contains(&death.experience), "{}", death.experience);
    }
}

#[test]
fn salmon_sizes_and_pufferfish_puffs_scale_their_bodies() {
    let mut world = EntityWorld::default();
    let salmon = fish_at(&mut world, SALMON, DVec3::new(0.5, 66.0, 0.5));
    let mob = fish(&world, salmon);
    assert_eq!(mob.state.variant, FishVariant::Salmon(SalmonSize::Medium));
    for (size, scale) in [
        (SalmonSize::Small, 0.5_f32),
        (SalmonSize::Large, 1.5),
        (SalmonSize::Medium, 1.0),
    ] {
        set_salmon_size(fish_mut(&mut world, salmon), size);
        let mob = fish(&world, salmon);
        assert_eq!(mob.state.variant, FishVariant::Salmon(size));
        let wanted = (0.7_f32 * scale, 0.4_f32 * scale, 0.26_f32 * scale);
        assert_eq!(size_of(mob), wanted, "{size:?}");
        let state = &mob.state.ai.state;
        assert_eq!(
            (state.walk.width, state.walk.height, state.eye_height),
            wanted
        );
    }
    assert_eq!(SalmonSize::by_id(-3), SalmonSize::Small);
    assert_eq!(SalmonSize::by_id(7), SalmonSize::Large);
    for size in SalmonSize::ALL {
        assert_eq!(SalmonSize::by_id(size.id()), size);
        assert_eq!(SalmonSize::named(size.name()), Some(size));
    }
    assert_eq!(SalmonSize::named("huge"), None);

    let puffer = fish_at(&mut world, PUFFERFISH, DVec3::new(4.5, 66.0, 0.5));
    for (puff, kept, scale) in [(1, 1, 0.7_f32), (2, 2, 1.0), (7, 2, 1.0), (0, 0, 0.5)] {
        set_puff(fish_mut(&mut world, puffer), puff);
        let mob = fish(&world, puffer);
        assert_eq!(mob.state.puff, kept);
        let wanted = (0.7_f32 * scale, 0.7_f32 * scale, 0.455_f32 * scale);
        assert_eq!(size_of(mob), wanted, "puff {puff}");
    }
    let cod = fish_at(&mut world, COD, DVec3::new(8.5, 66.0, 0.5));
    set_salmon_size(fish_mut(&mut world, cod), SalmonSize::Large);
    set_puff(fish_mut(&mut world, cod), 2);
    assert_eq!(
        size_of(fish(&world, cod)),
        (0.5, 0.3, 0.195),
        "a cod keeps its size"
    );
}

#[test]
fn a_fish_swims_with_the_fish_move_control_and_settles_when_idle() {
    let mut world = EntityWorld::default();
    let mut tank = Tank { size: 24, top: 79 };
    let start = DVec3::new(12.5, 70.0, 12.5);
    let id = fish_at(&mut world, COD, start);
    let watcher = player_at(1, DVec3::new(12.5, 70.0, 36.5));
    let wanted = (1.0 * SPEED) as f32;
    let (mut before, mut moving, mut wandered, mut farthest) = (0.0_f32, 0, false, 0.0_f64);
    for tick in 0..300 {
        world.tick_with_players(&mut tank, &[watcher]);
        let mob = fish(&world, id);
        let body = &mob.core.body;
        let p = body.position;
        let inside = (0.0..24.0).contains(&p.x) && (0.0..24.0).contains(&p.z);
        assert!(body.touching_water && inside, "tick {tick}: {p}");
        let speed = mob.state.ai.speed;
        let lerped = before + FISH_SPEED_LERP * (wanted - before);
        assert!(
            speed == 0.0 || speed == lerped,
            "tick {tick}: {before} then {speed}"
        );
        moving += usize::from(speed > 0.0);
        wandered |= mob.state.running_goals().contains(&"FishSwimGoal");
        farthest = farthest.max(p.distance(start));
        before = speed;
    }
    assert!(wandered && moving > 20, "{moving} ticks under way");
    assert!(farthest > 2.0, "{farthest}");

    for _ in 0..500 {
        world.tick(&mut tank);
    }
    let mob = fish(&world, id);
    let body = &mob.core.body;
    assert!(body.touching_water);
    assert!(mob.state.ai.state.navigation.is_done());
    assert!(!mob.state.running_goals().contains(&"FishSwimGoal"), "idle");
    assert_eq!(mob.state.ai.speed, 0.0);
    assert!(body.velocity.length() < 0.01, "{}", body.velocity);
}

#[test]
fn a_fish_out_of_water_flops_and_drowns_in_seventeen_seconds() {
    for kind in Fish::KINDS {
        let mut world = EntityWorld::default();
        let mut land = LAND;
        let id = fish_at(&mut world, kind, DVec3::new(0.5, 64.0, 0.5));
        let variant = fish(&world, id).state.variant;
        let death = format!("entity.{}.death", variant.sound_family());
        let (mut flops, mut died) = (0, 0);
        for tick in 1..=340 {
            world.tick(&mut land);
            let sounds = world.take_sounds();
            flops += heard(&sounds, variant.flop_sound());
            died += heard(&sounds, &death);
            let core = &fish(&world, id).core;
            let health = match tick {
                ..320 => 3.0,
                320..340 => 1.0,
                _ => 0.0,
            };
            assert_eq!(core.health.max(0.0), health, "{kind} at tick {tick}");
        }
        assert!(flops >= 10, "{kind} flops: {flops}");
        assert_eq!(died, 1, "{kind}");
    }
}

#[test]
fn a_hurt_fish_panics_and_fish_flee_survival_players_within_eight_blocks() {
    let mut tank = Tank { size: 24, top: 79 };
    let at = DVec3::new(12.5, 70.0, 12.5);
    let mut world = EntityWorld::default();
    let id = fish_at(&mut world, COD, at);
    world.tick(&mut tank);
    let running = fish(&world, id).state.running_goals();
    assert!(!running.contains(&"PanicGoal"));
    let from = at + DVec3::new(12.0, 0.0, 0.0);
    world.player_attack(&attack(1, from, 1.0), id);
    for _ in 0..2 {
        world.tick(&mut tank);
    }
    let mob = fish(&world, id);
    assert_eq!(mob.core.health, 2.0);
    assert!(mob.state.running_goals().contains(&"PanicGoal"));
    let navigation = &mob.state.ai.state.navigation;
    assert_eq!(navigation.speed_modifier, PANIC_SPEED);
    let (x, y, z) = navigation.path_target.expect("a spot to flee to");
    let block = (
        at.x.floor() as i32,
        at.y.floor() as i32,
        at.z.floor() as i32,
    );
    assert!((x - block.0).abs() <= 6 && (y - block.1).abs() <= 5 && (z - block.2).abs() <= 6);
    for _ in 0..300 {
        world.tick(&mut tank);
    }
    let mob = fish(&world, id);
    assert!(
        !mob.state.running_goals().contains(&"PanicGoal"),
        "the hit is forgotten after two seconds"
    );

    let sprint = Some(AVOID_SPRINT_SPEED);
    let walk = Some(AVOID_WALK_SPEED);
    for (offset, creative, peaceful, speed) in [
        (5.0, false, false, sprint),
        (7.5, false, false, walk),
        (9.0, false, false, None),
        (5.0, true, false, None),
        (5.0, false, true, None),
    ] {
        let mut world = EntityWorld::default();
        if peaceful {
            world.set_difficulty(0);
        }
        let id = fish_at(&mut world, COD, at);
        let mut player = player_at(1, at + DVec3::new(offset, 0.0, 0.0));
        player.attackable = !creative;
        world.tick_with_players(&mut tank, &[player]);
        let mob = fish(&world, id);
        let fleeing = mob.state.running_goals().contains(&"AvoidEntityGoal");
        assert_eq!(fleeing, speed.is_some(), "{offset} {creative} {peaceful}");
        let Some(speed) = speed else { continue };
        let navigation = &mob.state.ai.state.navigation;
        assert_eq!(navigation.speed_modifier, speed, "{offset}");
        let (x, y, z) = navigation.path_target.expect("a spot away");
        let spot = DVec3::new(f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5);
        let threat = player.position;
        assert!(spot.distance_squared(threat) >= at.distance_squared(threat));
    }

    let mut shore = Shore::default();
    shore.water.insert((3, 64, 0));
    let on_land = DVec3::new(0.5, 64.0, 0.5);
    assert_eq!(look_for_water(&shore, on_land), Some((3, 64, 0)));
    shore.water.insert((0, 65, -1));
    assert_eq!(look_for_water(&shore, on_land), Some((0, 65, -1)));
    shore.stone.insert((0, 64, 0));
    assert_eq!(look_for_water(&shore, on_land), None, "stuck in stone");
    let dry = Shore::default();
    assert_eq!(look_for_water(&dry, on_land), None);
}

#[test]
fn fish_despawn_beyond_64_blocks_unless_from_a_bucket() {
    let mut world = EntityWorld::default();
    let player = DVec3::new(0.5, 64.0, 0.5);
    let near = fish_at(&mut world, COD, DVec3::new(60.5, 64.0, 0.5));
    let far = fish_at(&mut world, COD, DVec3::new(70.5, 64.0, 0.5));
    let kept = fish_at(&mut world, SALMON, DVec3::new(70.5, 64.0, 4.5));
    fish_mut(&mut world, kept).state.from_bucket = true;
    assert_eq!(
        Fish::despawn(fish(&world, kept)),
        Despawn {
            distance: 64.0,
            removable: false,
            peaceful_ok: true,
            kept: true,
        }
    );
    let census = world.census();
    let persistent = |kind: &str| census.iter().find(|c| c.kind == kind).unwrap().persistent;
    assert!(persistent(SALMON) && !persistent(COD));
    let beside = DVec3::new(66.5, 64.0, 0.5);
    assert_eq!(
        world.check_despawn(&[beside], true, &|_| true),
        0,
        "peaceful"
    );
    assert_eq!(world.check_despawn(&[player], false, &|_| true), 1);
    let left: Vec<u64> = world.family::<Fish>().iter().map(|m| m.core.id).collect();
    assert_eq!(left, vec![near, kept]);
    assert!(!left.contains(&far));
}

#[test]
fn fish_sound_like_vanilla() {
    assert_eq!(
        Fish::swim_sounds(COD),
        Some(("entity.fish.swim", "entity.generic.splash"))
    );
    assert!(Fish::KINDS.iter().all(|kind| Fish::quiet_steps(kind)));
    let mut world = EntityWorld::default();
    let mut tank = Tank { size: 30, top: 79 };
    let cod = fish_at(&mut world, COD, DVec3::new(5.5, 70.0, 5.5));
    let puffer = fish_at(&mut world, PUFFERFISH, DVec3::new(24.5, 70.0, 24.5));
    let watcher = player_at(1, DVec3::new(15.5, 90.0, 15.5));
    let (mut calls, mut swims, mut stepped) = (Vec::new(), 0, 0);
    for tick in 0..3000 {
        world.tick_with_players(&mut tank, &[watcher]);
        let sounds = world.take_sounds();
        if heard(&sounds, "entity.cod.ambient") > 0 {
            calls.push(tick);
        }
        assert_eq!(
            heard(&sounds, "entity.puffer_fish.ambient"),
            0,
            "tick {tick}"
        );
        assert_eq!(heard(&sounds, "entity.generic.swim"), 0);
        swims += heard(&sounds, "entity.fish.swim");
        stepped += sounds.iter().filter(|s| s.event.contains("step")).count();
    }
    assert!(calls.len() >= 3, "{calls:?}");
    assert!(
        calls.windows(2).all(|pair| pair[1] - pair[0] > 120),
        "{calls:?}"
    );
    assert!(swims > 0 && stepped == 0, "{swims} {stepped}");
    assert_eq!(fish(&world, cod).core.ambient_interval, 120);
    assert_eq!(fish(&world, puffer).core.ambient_interval, 120);

    let mut world = EntityWorld::default();
    let mut land = LAND;
    fish_at(&mut world, COD, DVec3::new(0.5, 64.0, 0.5));
    let mut flops = 0;
    for _ in 0..100 {
        world.tick(&mut land);
        let sounds = world.take_sounds();
        flops += heard(&sounds, "entity.cod.flop");
        assert!(sounds.iter().all(|s| !s.event.contains("step")), "no steps");
    }
    assert!(flops > 0);
}

#[test]
fn a_cod_drops_cod_and_sometimes_bone_meal_and_a_burning_one_cooked_cod() {
    let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
        return;
    };
    let jar = std::path::Path::new(&root).join("client.jar");
    if !jar.is_file() {
        return;
    }
    let mut book = crate::loot::EntityLootBook::from_jar(&jar, 0).expect("the loot tables");
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let (mut raw, mut cooked, mut bone_meal) = (0, 0, 0);
    let at = DVec3::new(0.5, 64.0, 0.5);
    for burning in [false, true] {
        for _ in 0..200 {
            let id = fish_at(&mut world, COD, at);
            if burning {
                fish_mut(&mut world, id).core.body.fire_ticks = 100;
            }
            assert!(
                world
                    .player_attack(&attack(1, at + DVec3::X, 100.0), id)
                    .died
            );
            let deaths = world.take_deaths();
            assert!((1..=3).contains(&deaths[0].experience));
            let drops = book.death_drops(deaths);
            let count = |item: &str| {
                drops
                    .iter()
                    .filter(|(stack, _)| stack.id == item)
                    .map(|(stack, _)| u32::from(stack.count))
                    .sum::<u32>()
            };
            let (plain, fried) = (count("minecraft:cod"), count("minecraft:cooked_cod"));
            assert_eq!(plain + fried, 1, "{drops:?}");
            assert_eq!(fried, u32::from(burning));
            raw += plain;
            cooked += fried;
            bone_meal += count("minecraft:bone_meal");
            world.remove_mobs(&[id]);
        }
    }
    assert_eq!((raw, cooked), (200, 200));
    assert!(
        (1..=60).contains(&bone_meal),
        "about one in twenty: {bone_meal}"
    );
}
