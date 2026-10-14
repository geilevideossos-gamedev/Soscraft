use minecraftoss_player::{Block, Face, Pos};
use std::collections::BTreeMap;

pub const BELL: &str = "minecraft:bell";
pub const USE_SOUND: &str = "minecraft:block.bell.use";
pub const RESONATE_SOUND: &str = "minecraft:block.bell.resonate";
pub const DURATION: i32 = 50;
pub const GLOW_DURATION: i32 = 60;
pub const MIN_TICKS_BETWEEN_SEARCHES: i64 = 60;
pub const MAX_RESONATION_TICKS: i32 = 40;
pub const TICKS_BEFORE_RESONATION: i32 = 5;
pub const SEARCH_RADIUS: f64 = 48.0;
pub const HEAR_BELL_RADIUS: f64 = 32.0;
pub const HIGHLIGHT_RAIDERS_RADIUS: f64 = 48.0;
pub const PROPER_HIT_TOP: f64 = 0.8123999834060669;

pub fn is_bell(block: &Block) -> bool {
    block.id == BELL
}

pub fn face_name(face: Face) -> &'static str {
    match face {
        Face::Down => "down",
        Face::Up => "up",
        Face::North => "north",
        Face::South => "south",
        Face::West => "west",
        Face::East => "east",
    }
}

fn axis(direction: &str) -> char {
    match direction {
        "east" | "west" => 'x',
        "up" | "down" => 'y',
        _ => 'z',
    }
}

pub fn proper_hit(block: &Block, face: Face, hit_y: f64) -> bool {
    let face = face_name(face);
    if axis(face) == 'y' || hit_y > PROPER_HIT_TOP {
        return false;
    }
    let facing = block.property("facing").unwrap_or("north");
    match block.property("attachment").unwrap_or("floor") {
        "floor" => axis(face) == axis(facing),
        "single_wall" | "double_wall" => axis(face) != axis(facing),
        "ceiling" => true,
        _ => false,
    }
}

pub fn center(pos: Pos) -> glam::DVec3 {
    glam::DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    )
}

pub trait BellListeners {
    fn living_near(&self, bell: Pos, radius: f64) -> Vec<u64>;
    fn hear_bell(&mut self, listeners: &[u64], bell: Pos, radius: f64, game_time: i64);
    fn raiders_near(&self, listeners: &[u64], bell: Pos, radius: f64) -> Vec<u64>;
    fn glow(&mut self, id: u64, ticks: i32);
}

impl BellListeners for minecraftoss_entities::world::EntityWorld {
    fn living_near(&self, bell: Pos, radius: f64) -> Vec<u64> {
        self.bell_listeners(bell, radius)
    }

    fn hear_bell(&mut self, listeners: &[u64], bell: Pos, radius: f64, game_time: i64) {
        minecraftoss_entities::world::EntityWorld::hear_bell(
            self, listeners, bell, radius, game_time,
        );
    }

    fn raiders_near(&self, listeners: &[u64], bell: Pos, radius: f64) -> Vec<u64> {
        self.raiders_near_bell(listeners, bell, radius)
    }

    fn glow(&mut self, id: u64, ticks: i32) {
        use minecraftoss_entities::effects::{EffectInstance, MobEffect};
        self.add_mob_effect(id, EffectInstance::new(MobEffect::Glowing, ticks, 0));
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bell {
    pub last_ring: i64,
    pub ticks: i32,
    pub shaking: bool,
    pub nearby: Option<Vec<u64>>,
    pub resonating: bool,
    pub resonation_ticks: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Bells {
    pub bells: BTreeMap<Pos, Bell>,
}

impl Bells {
    pub fn ring(&mut self, pos: Pos, game_time: i64, world: &mut dyn BellListeners) {
        let bell = self.bells.entry(pos).or_default();
        if game_time > bell.last_ring + MIN_TICKS_BETWEEN_SEARCHES || bell.nearby.is_none() {
            bell.last_ring = game_time;
            bell.nearby = Some(world.living_near(pos, SEARCH_RADIUS));
        }
        if let Some(nearby) = &bell.nearby {
            world.hear_bell(nearby, pos, HEAR_BELL_RADIUS, game_time);
        }
        bell.resonation_ticks = 0;
        bell.ticks = 0;
        bell.shaking = true;
    }

    pub fn tick(
        &mut self,
        game_time: i64,
        is_bell: impl Fn(Pos) -> bool,
        world: &mut dyn BellListeners,
    ) -> Vec<Pos> {
        self.bells.retain(|&pos, bell| {
            let busy = bell.shaking || bell.resonating;
            is_bell(pos) && (busy || game_time <= bell.last_ring + MIN_TICKS_BETWEEN_SEARCHES)
        });
        let mut resonated = Vec::new();
        for (&pos, bell) in &mut self.bells {
            if bell.shaking {
                bell.ticks += 1;
            }
            if bell.ticks >= DURATION {
                bell.shaking = false;
                bell.ticks = 0;
            }
            let nearby = bell.nearby.clone().unwrap_or_default();
            if bell.ticks >= TICKS_BEFORE_RESONATION
                && bell.resonation_ticks == 0
                && !world
                    .raiders_near(&nearby, pos, HEAR_BELL_RADIUS)
                    .is_empty()
            {
                bell.resonating = true;
                resonated.push(pos);
            }
            if bell.resonating {
                if bell.resonation_ticks < MAX_RESONATION_TICKS {
                    bell.resonation_ticks += 1;
                } else {
                    for id in world.raiders_near(&nearby, pos, HIGHLIGHT_RAIDERS_RADIUS) {
                        world.glow(id, GLOW_DURATION);
                    }
                    bell.resonating = false;
                }
            }
        }
        resonated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Village {
        villagers: Vec<(u64, glam::DVec3)>,
        raiders: Vec<(u64, glam::DVec3)>,
        heard: Vec<(u64, i64)>,
        glowing: Vec<(u64, i32)>,
    }

    impl BellListeners for Village {
        fn living_near(&self, bell: Pos, radius: f64) -> Vec<u64> {
            let c = center(bell);
            self.villagers
                .iter()
                .chain(&self.raiders)
                .filter(|(_, at)| {
                    (at.x - c.x).abs() <= radius + 0.5 && (at.z - c.z).abs() <= radius + 0.5
                })
                .map(|(id, _)| *id)
                .collect()
        }

        fn hear_bell(&mut self, listeners: &[u64], bell: Pos, radius: f64, game_time: i64) {
            for (id, at) in &self.villagers {
                if listeners.contains(id) && at.distance_squared(center(bell)) < radius * radius {
                    self.heard.push((*id, game_time));
                }
            }
        }

        fn raiders_near(&self, listeners: &[u64], bell: Pos, radius: f64) -> Vec<u64> {
            self.raiders
                .iter()
                .filter(|(id, at)| {
                    listeners.contains(id) && at.distance_squared(center(bell)) < radius * radius
                })
                .map(|(id, _)| *id)
                .collect()
        }

        fn glow(&mut self, id: u64, ticks: i32) {
            self.glowing.push((id, ticks));
        }
    }

    const BELL_AT: Pos = (0, 64, 0);

    #[test]
    fn a_ring_wakes_the_villagers_within_32_blocks() {
        let mut village = Village {
            villagers: vec![
                (1, glam::DVec3::new(10.0, 64.0, 0.0)),
                (2, glam::DVec3::new(40.0, 64.0, 0.0)),
            ],
            ..Village::default()
        };
        let mut bells = Bells::default();
        bells.ring(BELL_AT, 100, &mut village);
        assert_eq!(village.heard, [(1, 100)]);
        let bell = &bells.bells[&BELL_AT];
        assert!(bell.shaking);
        assert_eq!(bell.nearby.as_deref(), Some(&[1, 2][..]));
    }

    #[test]
    fn the_listeners_are_searched_again_only_after_three_seconds() {
        let mut village = Village::default();
        let mut bells = Bells::default();
        bells.ring(BELL_AT, 100, &mut village);
        village
            .villagers
            .push((7, glam::DVec3::new(3.0, 64.0, 0.0)));
        bells.ring(BELL_AT, 160, &mut village);
        assert!(village.heard.is_empty(), "the list is kept for 60 ticks");
        bells.ring(BELL_AT, 161, &mut village);
        assert_eq!(village.heard, [(7, 161)]);
    }

    #[test]
    fn raiders_near_a_rung_bell_resonate_and_glow_after_two_seconds() {
        let mut village = Village {
            raiders: vec![
                (5, glam::DVec3::new(20.0, 64.0, 0.0)),
                (6, glam::DVec3::new(40.0, 64.0, 0.0)),
            ],
            ..Village::default()
        };
        let mut bells = Bells::default();
        bells.ring(BELL_AT, 0, &mut village);
        let mut resonated = Vec::new();
        for tick in 1..=4 {
            resonated.extend(
                bells
                    .tick(tick, |_| true, &mut village)
                    .into_iter()
                    .map(|_| tick),
            );
        }
        assert!(resonated.is_empty(), "nothing before five ticks of shaking");
        assert_eq!(bells.tick(5, |_| true, &mut village), [BELL_AT]);
        for tick in 6..=44 {
            assert!(bells.tick(tick, |_| true, &mut village).is_empty());
            assert!(village.glowing.is_empty(), "tick {tick}");
        }
        bells.tick(45, |_| true, &mut village);
        assert_eq!(village.glowing, [(5, GLOW_DURATION), (6, GLOW_DURATION)]);
        for tick in 46..=60 {
            assert!(
                bells.tick(tick, |_| true, &mut village).is_empty(),
                "one resonance a ring"
            );
        }
    }

    #[test]
    fn a_bell_without_raiders_only_shakes_and_a_broken_one_is_forgotten() {
        let mut village = Village::default();
        let mut bells = Bells::default();
        bells.ring(BELL_AT, 0, &mut village);
        for tick in 1..=50 {
            assert!(bells.tick(tick, |_| true, &mut village).is_empty());
        }
        assert!(!bells.bells[&BELL_AT].shaking);
        bells.ring(BELL_AT, 70, &mut village);
        bells.tick(71, |_| false, &mut village);
        assert!(bells.bells.is_empty());
    }

    #[test]
    fn a_bell_rings_from_its_sides_below_its_rim() {
        let floor = Block::new(BELL)
            .with("attachment", "floor")
            .with("facing", "north");
        assert!(proper_hit(&floor, Face::North, 0.5));
        assert!(proper_hit(&floor, Face::South, 0.5));
        assert!(!proper_hit(&floor, Face::East, 0.5));
        assert!(!proper_hit(&floor, Face::Up, 0.5));
        assert!(!proper_hit(&floor, Face::North, 0.9));
        let wall = Block::new(BELL)
            .with("attachment", "single_wall")
            .with("facing", "north");
        assert!(proper_hit(&wall, Face::East, 0.5));
        assert!(!proper_hit(&wall, Face::North, 0.5));
        let ceiling = Block::new(BELL)
            .with("attachment", "ceiling")
            .with("facing", "east");
        assert!(proper_hit(&ceiling, Face::North, 0.3));
        assert!(proper_hit(&ceiling, Face::West, 0.3));
    }
}
