use super::*;
use crate::effects::{EffectInstance, MobEffect};
use crate::world::families::{Despawn, category};
use crate::world::{LEAD, LeashHolder, LeashUse, MobSound, PlayerAttack};
use minecraftoss_player::Pos;

const FLOOR: i32 = 40;
const SURFACE: i32 = 63;
const SHORE: i32 = 20;

struct Sea;

impl World for Sea {
    fn block(&self, pos: Pos) -> Option<Block> {
        let (x, y, _) = pos;
        if y < FLOOR || (x >= SHORE && y <= SURFACE) {
            Some(Block::new("minecraft:stone"))
        } else if y <= SURFACE {
            Some(Block::new("minecraft:water"))
        } else {
            None
        }
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}

    fn step_sound(&self, pos: Pos) -> Option<(String, f32, f32)> {
        let stone = self
            .block(pos)
            .is_some_and(|block| block.id == "minecraft:stone");
        stone.then(|| ("block.stone.step".to_owned(), 1.0, 1.0))
    }
}

fn deep(x: f64, z: f64) -> DVec3 {
    DVec3::new(x, 50.0, z)
}

fn ashore(z: f64) -> DVec3 {
    DVec3::new(25.5, 64.0, z)
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

fn squid_at(world: &mut EntityWorld, kind: &str, at: DVec3) -> u64 {
    world.spawn_family_kind(kind, at).unwrap()
}

fn baby_at(world: &mut EntityWorld, at: DVec3) -> u64 {
    let mut mob = Squid::spawn(SQUID, at).unwrap();
    mob.state.age.set(Age::BABY_START);
    set_baby(&mut mob, true);
    world.spawn_family(mob)
}

fn squid(world: &EntityWorld, id: u64) -> &FamilyMob<Squid> {
    world
        .family::<Squid>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn squid_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Squid> {
    world.family_mob_mut::<Squid>(id).unwrap()
}

fn generic(amount: f32) -> Hurt {
    Hurt::new(amount, "minecraft:generic", DamageSourceKind::Generic)
}

fn heard(sounds: &[MobSound], event: &str) -> usize {
    sounds.iter().filter(|sound| sound.event == event).count()
}

fn full_tick(id: u64, tick_count: i32) -> bool {
    tick_count <= 1 || (tick_count + id as i32) % 2 == 0
}

#[test]
fn a_squid_is_0_8_with_ten_health_and_its_kind() {
    let mut world = EntityWorld::default();
    world.set_mob_drops(true);
    let plain = squid_at(&mut world, SQUID, deep(0.5, 0.5));
    let glow = squid_at(&mut world, GLOW_SQUID, deep(4.5, 0.5));
    let baby = baby_at(&mut world, deep(8.5, 0.5));
    for (id, kind) in [(plain, SQUID), (glow, GLOW_SQUID)] {
        assert_eq!(world.entity_type(id), Some(kind));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(64));
        let mob = squid(&world, id);
        let core = &mob.core;
        assert_eq!(
            (core.body.width, core.body.height, core.eye_height),
            (0.8, 0.8, 0.4)
        );
        assert_eq!((core.health, core.max_health), (10.0, 10.0));
        assert_eq!((core.baby, core.ambient_interval), (false, 120));
        assert_eq!(mob.state.glow, kind == GLOW_SQUID);
        assert_eq!(Squid::breath(mob), Breath::Gills);
        assert_eq!(
            Squid::despawn(mob),
            Despawn {
                distance: 128.0,
                removable: true,
                peaceful_ok: true,
                kept: false,
            }
        );
        assert!(!Squid::emits_sounds(kind), "{kind} moves by events only");
    }
    let core = &squid(&world, baby).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (0.5, 0.5, 0.37, true)
    );
    assert_eq!(category(64, SQUID), Some("water_creature"));
    assert_eq!(category(64, GLOW_SQUID), Some("underground_water_creature"));
    let traits = world.family_traits(plain).unwrap();
    assert_eq!(traits.category, "water_creature");
    assert!(traits.leashable && traits.breathes_water && traits.pushable);
    assert!(!traits.enemy && !traits.undead && !traits.fire_immune);
    assert_eq!(traits.voice, ("squid", 0.4, "neutral_volume"));
    let traits = world.family_traits(glow).unwrap();
    assert_eq!(traits.category, "underground_water_creature");
    assert_eq!(traits.voice, ("glow_squid", 0.4, "neutral_volume"));
    let census = world.census();
    assert_eq!(census.iter().filter(|c| c.kind == SQUID).count(), 2);
    assert_eq!(census.iter().filter(|c| c.kind == GLOW_SQUID).count(), 1);
    let candidates = world.mob_candidates();
    assert!(
        candidates
            .iter()
            .any(|m| m.id == baby && m.kind == SQUID && m.baby)
    );
    assert!(
        candidates
            .iter()
            .any(|m| m.id == glow && m.kind == GLOW_SQUID && !m.baby)
    );
    let player = deep(2.5, 2.5);
    for id in [plain, glow, baby] {
        assert!(world.player_attack(&attack(1, player, 100.0), id).died);
    }
    let deaths = world.take_deaths();
    for (id, table) in [(plain, SQUID), (glow, GLOW_SQUID)] {
        let death = deaths.iter().find(|d| d.id == id).unwrap();
        assert_eq!(death.table, Some(table));
        assert!((1..=3).contains(&death.experience), "{}", death.experience);
    }
    let death = deaths.iter().find(|d| d.id == baby).unwrap();
    assert_eq!(
        (death.table, death.experience),
        (None, 0),
        "babies drop nothing"
    );
}

#[test]
fn a_baby_squid_is_half_a_block_and_grows_up_to_full_size() {
    let mut world = EntityWorld::default();
    let id = baby_at(&mut world, deep(0.5, 0.5));
    let locked = baby_at(&mut world, deep(4.5, 0.5));
    squid_mut(&mut world, id).state.age.ticks = -2;
    let mob = squid_mut(&mut world, locked);
    mob.state.age.ticks = -1;
    mob.state.age.locked = true;
    world.tick(&mut Sea);
    let core = &squid(&world, id).core;
    assert!(core.baby && core.body.width == 0.5, "one tick to go");
    world.tick(&mut Sea);
    let core = &squid(&world, id).core;
    assert_eq!(
        (
            core.body.width,
            core.body.height,
            core.eye_height,
            core.baby
        ),
        (0.8, 0.8, 0.4, false)
    );
    for _ in 0..5 {
        world.tick(&mut Sea);
    }
    let mob = squid(&world, locked);
    assert!(
        mob.core.baby && mob.core.body.height == 0.5,
        "a locked baby"
    );
    assert_eq!(mob.state.age.ticks, -1);
}

#[test]
fn a_squid_pulses_only_in_the_last_quarter_of_its_tentacle_stroke() {
    let mut world = EntityWorld::default();
    let id = squid_at(&mut world, SQUID, deep(0.5, 0.5));
    world.tick(&mut Sea);
    let speed = squid(&world, id).state.tentacle_speed;
    assert!((0.1..=0.2).contains(&speed), "{speed}");
    let push = DVec3::new(0.0, 0.0, 0.25);
    let mob = squid_mut(&mut world, id);
    mob.core.no_ai = true;
    mob.core.body.velocity = DVec3::ZERO;
    mob.state.tentacle_speed = 0.1;
    mob.state.tentacle_movement = 0.0;
    mob.state.movement_vector = push;
    let (mut pulses, mut glides, mut last) = (0, 0, push);
    for tick in 1..=62 {
        world.tick(&mut Sea);
        let mob = squid(&world, id);
        let (state, velocity) = (&mob.state, mob.core.body.velocity);
        let stroke = state.tentacle_movement / PI;
        assert!(state.tentacle_movement > state.tentacle_movement_old);
        if stroke < 1.0 {
            let angle = mth::sin(f64::from(stroke * stroke * PI)) * PI * 0.25;
            assert_eq!(state.tentacle_angle, angle, "tick {tick}");
        } else {
            assert_eq!(state.tentacle_angle, 0.0, "tick {tick}");
        }
        if stroke > 0.75 && stroke < 1.0 {
            pulses += 1;
            assert_eq!((velocity, state.rotate_speed), (push, 1.0), "tick {tick}");
        } else if stroke <= 0.75 {
            assert_eq!(velocity, DVec3::ZERO, "tick {tick}: it waits");
        } else {
            glides += 1;
            assert_eq!(velocity, last * 0.9, "tick {tick}: it glides");
        }
        last = velocity;
    }
    assert_eq!((pulses, glides), (8, 31));
    assert!(last.z < push.z * 0.05 && last.z > 0.0);
}

#[test]
fn a_squid_picks_a_new_heading_on_full_ticks_and_stops_after_a_hundred_idle_ticks() {
    let mut world = EntityWorld::default();
    let id = squid_at(&mut world, SQUID, deep(0.5, 0.5));
    world.tick(&mut Sea);
    let check = |vector: DVec3| {
        let sideways = vector.x.hypot(vector.z);
        assert!((sideways - 0.2).abs() < 1.0e-4, "{vector}");
        assert!(vector.y >= -0.1 && vector.y < 0.1, "{vector}");
    };
    check(squid(&world, id).state.movement_vector);
    for _ in 0..10 {
        squid_mut(&mut world, id).state.movement_vector = DVec3::ZERO;
        world.tick(&mut Sea);
        let mob = squid(&world, id);
        let vector = mob.state.movement_vector;
        if full_tick(id, mob.core.tick_count) {
            check(vector);
        } else {
            assert_eq!(vector, DVec3::ZERO, "only on full ticks");
        }
    }

    let mob = squid_mut(&mut world, id);
    mob.core.body.position = ashore(0.5);
    let mut headings = Vec::new();
    for _ in 0..10 {
        world.tick(&mut Sea);
        let mob = squid(&world, id);
        if full_tick(id, mob.core.tick_count) {
            check(mob.state.movement_vector);
            headings.push(mob.state.movement_vector);
        }
    }
    assert!(headings.len() >= 4);
    assert!(
        headings.windows(2).all(|pair| pair[0] != pair[1]),
        "a new heading on every full tick out of water"
    );

    let mob = squid_mut(&mut world, id);
    mob.core.body.position = deep(0.5, 0.5);
    mob.core.no_action_time = 100;
    let mut idle = false;
    for _ in 0..4 {
        world.tick(&mut Sea);
        let mob = squid(&world, id);
        idle |= full_tick(id, mob.core.tick_count);
        if idle {
            assert_eq!(mob.state.movement_vector, DVec3::ZERO);
        }
    }
    assert!(idle && squid(&world, id).core.no_action_time > 100);
    world.hurt_family(id, &generic(1.0)).unwrap();
    assert_eq!(squid(&world, id).core.no_action_time, 0);
    world.tick(&mut Sea);
    world.tick(&mut Sea);
    check(squid(&world, id).state.movement_vector);
}

#[test]
fn a_squid_turns_tilts_and_rolls_with_its_motion() {
    let mut world = EntityWorld::default();
    let id = squid_at(&mut world, SQUID, deep(0.5, 0.5));
    world.tick(&mut Sea);
    let mob = squid_mut(&mut world, id);
    mob.core.no_ai = true;
    mob.state.tentacle_speed = 0.0;
    mob.state.tentacle_movement = 4.0;
    mob.state.rotate_speed = 1.0;
    let (mut roll, mut tilt) = (mob.state.z_body_rot, mob.state.x_body_rot);
    for _ in 0..60 {
        squid_mut(&mut world, id).core.body.velocity = DVec3::new(0.2, 0.0, 0.0);
        world.tick(&mut Sea);
        let mob = squid(&world, id);
        assert_eq!(
            (mob.state.z_body_rot_o, mob.state.x_body_rot_o),
            (roll, tilt)
        );
        assert!(mob.state.z_body_rot > roll, "it keeps rolling");
        roll = mob.state.z_body_rot;
        tilt = mob.state.x_body_rot;
    }
    let mob = squid(&world, id);
    assert!(
        (mob.core.yaw + 90.0).abs() < 0.5,
        "it faces east: {}",
        mob.core.yaw
    );
    assert_eq!(mob.core.look.head_yaw, mob.core.yaw);
    assert!(
        (mob.state.x_body_rot + 90.0).abs() < 0.5,
        "it lies along its motion"
    );
    assert!(mob.state.z_body_rot > 150.0 && mob.state.z_body_rot < 250.0);
    for _ in 0..60 {
        squid_mut(&mut world, id).core.body.velocity = DVec3::new(0.0, 0.2, 0.0);
        world.tick(&mut Sea);
    }
    let mob = squid(&world, id);
    assert!(mob.state.x_body_rot.abs() < 0.5, "it points up as it rises");
    for _ in 0..60 {
        squid_mut(&mut world, id).core.body.velocity = DVec3::new(-0.2, 0.0, 0.0);
        world.tick(&mut Sea);
    }
    assert!((squid(&world, id).core.yaw - 90.0).abs() < 0.5);

    let mob = squid_mut(&mut world, id);
    mob.core.body.position = ashore(0.5);
    mob.state.x_body_rot = 0.0;
    let yaw = mob.core.yaw;
    world.tick(&mut Sea);
    let mob = squid(&world, id);
    assert_eq!(mob.state.x_body_rot, -90.0 * 0.02, "it tips over on land");
    assert_eq!(mob.core.yaw, yaw);
}

fn flee_vector(world: &mut EntityWorld, id: u64, at: DVec3, from: DVec3) -> Option<DVec3> {
    let player = player_at(1, from);
    for _ in 0..2 {
        let mob = squid_mut(world, id);
        mob.core.body.position = at;
        mob.core.body.velocity = DVec3::ZERO;
        mob.core.no_action_time = 1000;
        world.tick_with_players(&mut Sea, &[player]);
    }
    let state = &squid(world, id).state;
    state.fleeing.map(|_| state.movement_vector)
}

#[test]
fn a_hit_squid_squirts_and_flees_up_to_a_block_a_tick() {
    let mut world = EntityWorld::default();
    let at = deep(10.5, 10.5);
    let id = squid_at(&mut world, SQUID, at);
    let player = player_at(1, at + DVec3::X);
    world.tick_with_players(&mut Sea, &[player]);
    world.take_sounds();
    assert!(
        world
            .player_attack(&attack(1, player.position, 1.0), id)
            .hurt
    );
    assert_eq!(squid(&world, id).state.ink_spurts, 1);
    let sounds: Vec<MobSound> = world
        .take_sounds()
        .into_iter()
        .filter(|sound| sound.event.starts_with("entity.squid."))
        .collect();
    let events: Vec<&str> = sounds.iter().map(|s| s.event.as_str()).collect();
    assert_eq!(events, ["entity.squid.hurt", SQUIRT]);
    assert_eq!(
        (sounds[1].volume, sounds[1].category),
        (0.4, "neutral_volume")
    );

    for distance in [1.0, 5.0, 9.5] {
        let scale = if distance > 5.0 {
            3.0 - (distance - 5.0) / 5.0
        } else {
            3.0
        };
        let from = at + DVec3::new(distance, 0.0, 0.0);
        let vector = flee_vector(&mut world, id, at, from).expect("it flees");
        let away = DVec3::new(-distance * scale / 20.0, 0.0, 0.0);
        assert!((vector - away).length() < 1.0e-9, "{distance}: {vector}");
    }
    let from = at + DVec3::new(9.99, 0.0, 0.0);
    let vector = flee_vector(&mut world, id, at, from).unwrap();
    assert!(vector.x < -0.99 && vector.x > -1.0, "almost a block a tick");
    let from = at + DVec3::new(10.0, 0.0, 0.0);
    assert_eq!(flee_vector(&mut world, id, at, from), None, "too far");

    let shallow = DVec3::new(10.5, 62.5, 10.5);
    let from = shallow + DVec3::new(1.0, -2.0, 0.0);
    let vector = flee_vector(&mut world, id, shallow, from).unwrap();
    assert_eq!(
        vector,
        DVec3::new(-0.15, 0.0, 0.0),
        "it never rises into the air"
    );
    let seabed = DVec3::new(10.5, 41.0, 10.5);
    let from = seabed + DVec3::new(0.0, 3.0, 0.0);
    let vector = flee_vector(&mut world, id, seabed, from).unwrap();
    assert_eq!(vector, DVec3::ZERO, "no way into the stone");
    let land = ashore(10.5);
    assert_eq!(
        flee_vector(&mut world, id, land, land + DVec3::X),
        None,
        "on land"
    );
}

#[test]
fn a_squid_hurt_again_within_five_seconds_squirts_even_without_an_attacker() {
    let mut world = EntityWorld::default();
    let id = squid_at(&mut world, SQUID, deep(0.5, 0.5));
    let other = squid_at(&mut world, SQUID, deep(8.5, 0.5));
    assert!(world.hurt_family(other, &generic(1.0)).unwrap().applied);
    assert_eq!(squid(&world, other).state.ink_spurts, 0, "nobody to blame");
    world.player_attack(&attack(1, deep(1.5, 0.5), 1.0), id);
    assert_eq!(squid(&world, id).state.ink_spurts, 1);
    for _ in 0..20 {
        world.tick(&mut Sea);
    }
    assert!(world.hurt_family(id, &generic(1.0)).unwrap().applied);
    assert_eq!(
        squid(&world, id).state.ink_spurts,
        2,
        "the player is still to blame"
    );
    for _ in 0..101 {
        world.tick(&mut Sea);
    }
    assert!(world.hurt_family(id, &generic(1.0)).unwrap().applied);
    assert_eq!(squid(&world, id).state.ink_spurts, 2, "five seconds later");
    for _ in 0..20 {
        world.tick(&mut Sea);
    }
    let bite = generic(1.0).by(Some(Target::Mob(other)), Some(SQUID));
    assert!(world.hurt_family(id, &bite).unwrap().applied);
    assert_eq!(
        squid(&world, id).state.ink_spurts,
        3,
        "a mob's hit inks too"
    );
}

#[test]
fn a_squid_on_land_falls_curls_and_dries_out() {
    let mut world = EntityWorld::default();
    let falling = squid_at(&mut world, SQUID, DVec3::new(25.5, 70.5, 0.5));
    let stranded = squid_at(&mut world, SQUID, ashore(4.5));
    let floating = squid_at(&mut world, SQUID, ashore(8.5));
    let levitation = EffectInstance::new(MobEffect::Levitation, 400, 0);
    squid_mut(&mut world, floating).core.effects.add(levitation);
    world.tick(&mut Sea);
    let drop = -0.08 * f64::from(0.98_f32);
    assert_eq!(
        squid(&world, falling).core.body.velocity,
        DVec3::new(0.0, drop, 0.0)
    );
    let rise = 0.05 * f64::from(0.98_f32);
    assert_eq!(squid(&world, floating).core.body.velocity.y, rise);
    world.tick(&mut Sea);
    let core = &squid(&world, falling).core;
    assert!((core.body.position.y - (70.5 + drop)).abs() < 1.0e-9);
    assert_eq!(core.body.velocity.y, (drop - 0.08) * f64::from(0.98_f32));
    assert!(
        squid(&world, floating).core.body.position.y > 64.0,
        "it floats up"
    );
    let mut sounds = Vec::new();
    for _ in 0..30 {
        world.tick(&mut Sea);
        sounds.extend(world.take_sounds());
    }
    let core = &squid(&world, falling).core;
    assert!(core.body.on_ground && (core.body.position.y - 64.0).abs() < 1.0e-9);
    assert_eq!(core.health, 7.0, "six and a half blocks hurt three");
    assert_eq!(heard(&sounds, "entity.generic.small_fall"), 1);
    assert_eq!(squid(&world, falling).state.ink_spurts, 0);

    let mut hits = Vec::new();
    for tick in 33..=400 {
        world.tick(&mut Sea);
        let core = &squid(&world, stranded).core;
        if core.health < 10.0 - 2.0 * hits.len() as f32 {
            hits.push(tick);
            assert_eq!(core.body.air, 0, "tick {tick}");
        }
    }
    assert_eq!(hits, [320, 340, 360, 380, 400], "dry for twenty seconds");
    let mob = squid(&world, stranded);
    assert!(mob.core.damage.death.is_some());
    assert!(mob.state.x_body_rot < -89.0, "it lies flat");
}

#[test]
fn a_glow_squid_goes_dark_for_a_hundred_ticks_when_it_squirts() {
    let mut world = EntityWorld::default();
    let glow = squid_at(&mut world, GLOW_SQUID, deep(0.5, 0.5));
    let plain = squid_at(&mut world, SQUID, deep(4.5, 0.5));
    assert!(world.hurt_family(glow, &generic(1.0)).unwrap().applied);
    assert_eq!(
        squid(&world, glow).state.dark_ticks,
        0,
        "no squirt, no dark"
    );
    for _ in 0..20 {
        world.tick(&mut Sea);
    }
    world.take_sounds();
    for id in [glow, plain] {
        world.player_attack(&attack(1, deep(2.5, 0.5), 1.0), id);
    }
    assert_eq!(squid(&world, glow).state.dark_ticks, 100);
    assert_eq!(squid(&world, plain).state.dark_ticks, 0);
    let sounds = world.take_sounds();
    assert_eq!(heard(&sounds, GLOW_SQUIRT), 1);
    assert_eq!(heard(&sounds, "entity.glow_squid.hurt"), 1);
    for tick in 1..=100 {
        world.tick(&mut Sea);
        assert_eq!(squid(&world, glow).state.dark_ticks, 100 - tick);
    }
    world.tick(&mut Sea);
    assert_eq!(squid(&world, glow).state.dark_ticks, 0);
    world.tick(&mut Sea);
    assert!(world.hurt_family(glow, &generic(1.0)).unwrap().applied);
    assert_eq!(squid(&world, glow).state.dark_ticks, 0, "five seconds on");

    let mut world = EntityWorld::default();
    let mut randoms = Vec::new();
    for (kind, x) in [(SQUID, 0.5), (GLOW_SQUID, 4.5)] {
        let id = squid_at(&mut world, kind, deep(x, 0.5));
        let mob = squid_mut(&mut world, id);
        mob.core.no_ai = true;
        mob.core.random = LegacyRandom::new(7);
        randoms.push(id);
    }
    world.tick(&mut Sea);
    let mut expected = squid(&world, randoms[0]).core.random.clone();
    let _ = (
        expected.next_double(),
        expected.next_double(),
        expected.next_double(),
    );
    let mut glows = squid(&world, randoms[1]).core.random.clone();
    assert_eq!(
        expected.next_int(1 << 30),
        glows.next_int(1 << 30),
        "its sparkle draws three doubles a tick"
    );
}

#[test]
fn a_dying_squid_drifts_on_its_last_pulse_and_does_not_sink() {
    let mut world = EntityWorld::default();
    let id = squid_at(&mut world, SQUID, deep(0.5, 0.5));
    world.tick(&mut Sea);
    let push = DVec3::new(0.0, 0.0, 0.2);
    let mob = squid_mut(&mut world, id);
    mob.core.body.velocity = DVec3::ZERO;
    mob.state.movement_vector = push;
    mob.state.tentacle_speed = 0.05;
    mob.state.tentacle_movement = 2.5;
    let start = mob.core.body.position;
    assert!(world.hurt_family(id, &generic(100.0)).unwrap().died);
    for _ in 0..12 {
        world.tick(&mut Sea);
        let mob = squid(&world, id);
        assert_eq!(mob.state.movement_vector, push, "no goals while dying");
    }
    let mob = squid(&world, id);
    assert!(mob.core.health <= 0.0 && !mob.core.removed);
    assert_eq!(mob.core.body.position.y, start.y, "it does not sink");
    assert!(mob.core.body.position.z > start.z + 2.0, "it drifts on");
}

#[test]
fn squids_are_silent_when_they_swim_and_call_every_120_ticks_or_so() {
    let mut world = EntityWorld::default();
    let diver = squid_at(&mut world, SQUID, DVec3::new(0.5, 66.0, 0.5));
    squid_mut(&mut world, diver).core.no_action_time = 1000;
    let mut sounds = Vec::new();
    for _ in 0..60 {
        world.tick(&mut Sea);
        sounds.extend(world.take_sounds());
    }
    assert_eq!(heard(&sounds, "entity.generic.splash"), 1, "it fell in");
    assert!(squid(&world, diver).core.body.touching_water);

    let mut world = EntityWorld::default();
    let id = squid_at(&mut world, SQUID, deep(10.5, 0.5));
    let (mut swims, mut ambient) = (0, Vec::new());
    for tick in 0..3000 {
        world.tick(&mut Sea);
        for sound in world.take_sounds() {
            if sound.event == "entity.squid.ambient" {
                assert_eq!(sound.volume, 0.4);
                ambient.push(tick);
            }
            sounds.push(sound);
        }
        let events = world.take_game_events();
        swims += events
            .iter()
            .filter(|event| event.event == "minecraft:swim" && event.mob == Some(id))
            .count();
    }
    assert!(swims >= 2, "its moves are still heard as game events");
    for sound in &sounds {
        let moving = sound.event.ends_with(".swim") || sound.event.ends_with(".step");
        assert!(!moving, "{sound:?}");
    }
    assert!(ambient.len() >= 8, "{ambient:?}");
    assert!(
        ambient.windows(2).all(|pair| pair[1] - pair[0] > 120),
        "{ambient:?}"
    );
}

#[test]
fn a_leashed_squid_is_pulled_after_its_player() {
    let mut world = EntityWorld::default();
    let at = deep(10.5, 0.5);
    let id = squid_at(&mut world, SQUID, at);
    let near = player_at(0, at + DVec3::X);
    world.set_leash_players(&[near]);
    assert_eq!(
        world.leash_interact(id, 0, Some(LEAD), false, false),
        LeashUse::Tied
    );
    let away = player_at(0, at - DVec3::new(8.0, 0.0, 0.0));
    for _ in 0..40 {
        world.tick_with_players(&mut Sea, &[away]);
    }
    assert_eq!(world.leash_holder(id), Some(LeashHolder::Player(0)));
    let position = squid(&world, id).core.body.position;
    assert!(position.x < at.x - 1.0, "pulled along: {position}");
    assert!(position.distance(away.position) < 7.0, "{position}");
    let gone = player_at(0, position - DVec3::new(13.0, 0.0, 0.0));
    world.tick_with_players(&mut Sea, &[gone]);
    assert_eq!(
        world.leash_holder(id),
        None,
        "the lead snaps past 12 blocks"
    );
}
