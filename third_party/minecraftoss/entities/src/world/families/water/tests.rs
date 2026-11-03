use super::swimming::swim_probe::{self, KIND, SILENT, SILENT_SPLASH, SWIM, SWIMMING, SwimProbe};
use crate::fluid::FluidFrame;
use crate::navigation::{GroundNavigation, navigate_walk_to};
use crate::world::families::pests::probe::{self, Probe};
use crate::world::families::{Breath, Family, FamilyMob, Hurt};
use crate::world::{DamageSourceKind, EntityWorld, MobSound};
use glam::DVec3;
use minecraftoss_player::{Block, Pos, World};
use std::collections::HashSet;

const STEP: &str = "block.stone.step";

#[derive(Default)]
struct Pool {
    water: HashSet<Pos>,
}

impl Pool {
    fn column(x: i32, z: i32, top: i32) -> Self {
        Self {
            water: (64..=top).map(|y| (x, y, z)).collect(),
        }
    }

    fn lanes(xs: &[i32], top: i32) -> Self {
        let mut water = HashSet::new();
        for &x in xs {
            for z in 0..40 {
                water.extend((64..=top).map(|y| (x, y, z)));
            }
        }
        Self { water }
    }
}

impl World for Pool {
    fn block(&self, pos: Pos) -> Option<Block> {
        if self.water.contains(&pos) {
            return Some(Block::new("minecraft:water"));
        }
        (pos.1 < 64).then(|| Block::new("minecraft:stone"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}

    fn step_sound(&self, pos: Pos) -> Option<(String, f32, f32)> {
        let stone = self
            .block(pos)
            .is_some_and(|block| block.id == "minecraft:stone");
        stone.then(|| (STEP.to_owned(), 1.0, 1.0))
    }
}

fn swimmer(world: &mut EntityWorld, kind: &str, at: DVec3) -> u64 {
    let id = world.spawn_family_kind(kind, at).unwrap();
    let mob = probe_mut(world, id);
    mob.state.input = DVec3::Z;
    mob.state.speed = 0.1;
    id
}

fn heard_near(heard: &[MobSound], event: &str, x: f64) -> usize {
    heard
        .iter()
        .filter(|sound| sound.event == event && (sound.position.x - x).abs() < 1.0)
        .count()
}

fn probe_at(world: &mut EntityWorld, at: DVec3, breath: Breath) -> u64 {
    let id = world.spawn_family_kind(KIND, at).unwrap();
    probe_mut(world, id).state.breath = breath;
    id
}

fn probe(world: &EntityWorld, id: u64) -> &FamilyMob<SwimProbe> {
    world
        .family::<SwimProbe>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn probe_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<SwimProbe> {
    world.family_mob_mut::<SwimProbe>(id).unwrap()
}

fn air_and_health(world: &EntityWorld, id: u64) -> (i32, f32) {
    let core = &probe(world, id).core;
    (core.body.air, core.health)
}

const IN_POOL: DVec3 = DVec3::new(0.5, 64.2, 0.5);
const ON_LAND: DVec3 = DVec3::new(10.5, 64.0, 0.5);
const LUNGS: Breath = Breath::Lungs {
    max: 300,
    refill: 4,
};

#[test]
fn a_gill_breather_keeps_full_air_in_water_and_dies_on_land_after_340_ticks() {
    let mut world = EntityWorld::default();
    let mut pool = Pool::column(0, 0, 66);
    let id = probe_at(&mut world, IN_POOL, Breath::Gills);
    probe_mut(&mut world, id).core.body.air = 100;
    world.tick(&mut pool);
    assert_eq!(air_and_health(&world, id), (300, swim_probe::MAX_HEALTH));
    for _ in 0..400 {
        world.tick(&mut pool);
    }
    assert_eq!(air_and_health(&world, id), (300, swim_probe::MAX_HEALTH));

    probe_mut(&mut world, id).core.body.position = ON_LAND;
    for tick in 1..=340 {
        world.tick(&mut pool);
        let (air, health) = air_and_health(&world, id);
        match tick {
            ..=319 => assert_eq!((air, health), (300 - tick, 3.0), "tick {tick}"),
            320 => assert_eq!((air, health), (0, 1.0), "the first gasp hurts"),
            321..=339 => assert_eq!((air, health), (320 - tick, 1.0), "tick {tick}"),
            _ => assert!(health <= 0.0, "dead at tick 340"),
        }
    }
    assert!(probe(&world, id).core.damage.death.is_some());
    world.tick(&mut pool);
    assert_eq!(
        probe(&world, id).core.body.air,
        300,
        "a dead fish stops gasping"
    );
}

#[test]
fn a_lung_breather_still_drowns_under_water() {
    let pest = Probe::spawn(probe::KIND, IN_POOL).unwrap();
    assert_eq!(
        Probe::breath(&pest),
        LUNGS,
        "the default is vanilla's lungs"
    );
    let mut world = EntityWorld::default();
    let mut pool = Pool::column(0, 0, 66);
    let id = probe_at(&mut world, IN_POOL, LUNGS);
    probe_mut(&mut world, id).core.body.air = 5;
    for _ in 0..24 {
        world.tick(&mut pool);
    }
    assert_eq!(air_and_health(&world, id), (-19, 3.0));
    world.tick(&mut pool);
    assert_eq!(air_and_health(&world, id), (0, 1.0), "it drowns");

    probe_mut(&mut world, id).core.body.position = ON_LAND;
    world.tick(&mut pool);
    assert_eq!(air_and_health(&world, id), (4, 1.0), "four back a tick");
    for _ in 0..100 {
        world.tick(&mut pool);
    }
    assert_eq!(air_and_health(&world, id), (300, 1.0));
}

#[test]
fn an_own_breather_keeps_its_air() {
    let mut world = EntityWorld::default();
    let mut pool = Pool::column(0, 0, 66);
    let id = probe_at(&mut world, IN_POOL, Breath::Own);
    probe_mut(&mut world, id).core.body.air = 123;
    for _ in 0..400 {
        world.tick(&mut pool);
    }
    assert_eq!(air_and_health(&world, id), (123, 3.0), "under water");

    probe_mut(&mut world, id).core.body.position = ON_LAND;
    for _ in 0..400 {
        world.tick(&mut pool);
    }
    assert_eq!(air_and_health(&world, id), (123, 3.0), "on land");
}

#[test]
fn a_lung_breather_with_4800_air_refills_at_once_and_still_drowns() {
    let mut world = EntityWorld::default();
    let mut pool = Pool::column(0, 0, 66);
    let breath = Breath::Lungs {
        max: 4800,
        refill: 4800,
    };
    let id = probe_at(&mut world, ON_LAND, breath);
    probe_mut(&mut world, id).core.body.air = -5;
    world.tick(&mut pool);
    assert_eq!(
        air_and_health(&world, id),
        (4800, 3.0),
        "one breath fills it"
    );
    world.tick(&mut pool);
    assert_eq!(air_and_health(&world, id), (4800, 3.0));

    probe_mut(&mut world, id).core.body.position = IN_POOL;
    world.tick(&mut pool);
    assert_eq!(air_and_health(&world, id), (4799, 3.0));
    probe_mut(&mut world, id).core.body.air = -19;
    world.tick(&mut pool);
    assert_eq!(air_and_health(&world, id), (0, 1.0), "it drowns");
}

#[test]
fn a_quiet_stepper_flops_without_a_step_sound_but_swims_with_its_own() {
    let mut world = EntityWorld::default();
    let mut pool = Pool::lanes(&[0], 79);
    let quiet = swimmer(&mut world, KIND, DVec3::new(10.5, 64.0, 0.5));
    swimmer(&mut world, SILENT, DVec3::new(14.5, 64.0, 0.5));
    let mut heard = Vec::new();
    for _ in 0..60 {
        world.tick(&mut pool);
        heard.extend(world.take_sounds());
    }
    assert!(probe(&world, quiet).core.body.position.z > 3.0, "it walked");
    assert!(heard_near(&heard, STEP, 14.5) >= 2, "a plain stepper steps");
    assert_eq!(heard_near(&heard, STEP, 10.5), 0, "the quiet one does not");

    probe_mut(&mut world, quiet).core.body.position = DVec3::new(0.5, 75.0, 0.5);
    heard.clear();
    for _ in 0..60 {
        world.tick(&mut pool);
        heard.extend(world.take_sounds());
    }
    assert!(
        heard_near(&heard, SWIM, 0.5) >= 2,
        "it swims with its own sound"
    );
    assert_eq!(heard_near(&heard, "entity.generic.swim", 0.5), 0);
    assert!(heard.iter().all(|sound| !sound.event.is_empty()));
}

#[test]
fn an_empty_swim_sound_is_silent_but_the_splash_plays() {
    let mut world = EntityWorld::default();
    let mut pool = Pool::lanes(&[0, 4], 79);
    swimmer(&mut world, KIND, DVec3::new(0.5, 81.0, 0.5));
    let silent = swimmer(&mut world, SILENT, DVec3::new(4.5, 81.0, 0.5));
    let (mut heard, mut swims) = (Vec::new(), 0);
    for _ in 0..80 {
        world.tick(&mut pool);
        heard.extend(world.take_sounds());
        let events = world.take_game_events();
        swims += events
            .iter()
            .filter(|event| event.event == "minecraft:swim" && event.mob == Some(silent))
            .count();
    }
    assert_eq!(heard_near(&heard, SILENT_SPLASH, 4.5), 1, "its own splash");
    assert_eq!(heard_near(&heard, "entity.generic.splash", 0.5), 1);
    assert!(heard_near(&heard, SWIM, 0.5) >= 2, "the other swims aloud");
    assert!(swims >= 2, "it swam too");
    for sound in heard.iter().filter(|sound| sound.position.x > 4.0) {
        let swim = sound.event.is_empty() || sound.event.ends_with(".swim");
        assert!(!swim, "{sound:?}");
    }
}

#[test]
fn a_family_that_moves_its_own_way_while_dying_skips_the_shared_dying_travel() {
    assert!(SwimProbe::OWN_DYING_TRAVEL);
    assert!(
        !Probe::OWN_DYING_TRAVEL,
        "the shared dying travel by default"
    );
    let mut world = EntityWorld::default();
    let mut pool = Pool::lanes(&[0, 4], 70);
    let push = DVec3::new(0.0, 0.0, 0.2);
    let kill = Hurt::new(100.0, "minecraft:generic", DamageSourceKind::Generic);
    let mut dying = Vec::new();
    for (kind, x) in [(KIND, 0.5), (probe::KIND, 4.5)] {
        for y in [67.0, 90.0] {
            let start = DVec3::new(x, y, 0.5);
            let id = world.spawn_family_kind(kind, start).unwrap();
            assert!(world.hurt_family(id, &kill).unwrap().died);
            world.body_mut(id).unwrap().velocity = push;
            dying.push((kind, id, start));
        }
    }
    world.tick(&mut pool);
    for (kind, id, start) in dying {
        let body = world.body_of(id).unwrap();
        if kind == KIND {
            assert_eq!((body.position, body.velocity), (start, push), "{start}");
        } else {
            assert!(body.position.z > start.z + 0.1, "{start}: it slides on");
            let slowed = body.velocity.z < push.z;
            assert!(slowed && body.velocity.y < 0.0, "{start}: drag and sinking");
        }
    }
}

struct Basin {
    size: i32,
    top: i32,
}

impl World for Basin {
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
}

fn swimming(world: &mut EntityWorld, at: DVec3, smooth: bool) -> u64 {
    let id = world.spawn_family_kind(SWIMMING, at).unwrap();
    probe_mut(world, id).state.smooth = smooth;
    id
}

fn navigation(world: &EntityWorld, id: u64) -> &GroundNavigation {
    &probe(world, id).state.ai.as_ref().unwrap().state.navigation
}

fn send(world: &mut EntityWorld, id: u64, level: &impl World, to: DVec3) {
    let FamilyMob { core, state } = probe_mut(world, id);
    let ai = state.ai.as_deref_mut().unwrap();
    let body = &core.body;
    let fluid = FluidFrame::sample(level, body.position, body.width, body.height);
    let profile = ai.state.profile();
    let navigation = &mut ai.state.navigation;
    navigate_walk_to(body, navigation, level, &profile, fluid, to, 1.0, 1);
}

#[test]
fn a_swim_probe_swims_to_a_spot_in_a_pool_and_never_leaves_it() {
    let spot = DVec3::new(7.5, 67.0, 7.5);
    for smooth in [false, true] {
        let mut world = EntityWorld::default();
        let mut basin = Basin { size: 10, top: 67 };
        let id = swimming(&mut world, DVec3::new(2.5, 64.0, 2.5), smooth);
        send(&mut world, id, &basin, spot);
        assert!(!navigation(&world, id).is_done(), "{smooth}: a path");
        let mut nose_up = false;
        for tick in 0..300 {
            world.tick(&mut basin);
            let core = &probe(&world, id).core;
            let p = core.body.position;
            let pool = (0.0..10.0).contains(&p.x) && (0.0..10.0).contains(&p.z);
            let wet = pool && (64.0..68.0).contains(&p.y) && core.body.touching_water;
            assert!(wet, "{smooth} at tick {tick}: {p}");
            nose_up |= core.look.pitch < 0.0;
        }
        assert!(navigation(&world, id).is_done(), "{smooth}: there");
        let p = probe(&world, id).core.body.position;
        assert!(p.distance(spot) < 2.0, "{smooth}: {p}");
        assert_eq!(nose_up, smooth, "only the smooth swimmer tilts");
    }
}
