use super::swimming::swim_probe::{self, KIND, SwimProbe};
use crate::world::EntityWorld;
use crate::world::families::pests::probe::{self, Probe};
use crate::world::families::{Breath, Family, FamilyMob};
use glam::DVec3;
use minecraftoss_player::{Block, Pos, World};

#[derive(Default)]
struct Pool {
    water: Vec<Pos>,
}

impl Pool {
    fn column(x: i32, z: i32, top: i32) -> Self {
        Self {
            water: (64..=top).map(|y| (x, y, z)).collect(),
        }
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
