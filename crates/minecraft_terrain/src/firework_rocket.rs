use crate::firework::{Explosion, Fireworks};
use crate::thrown::ThrownView;
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

pub const ITEM: &str = crate::firework::FIREWORK_ROCKET;
pub const BLAST_SOUND: &str = "minecraft:entity.firework_rocket.blast";
pub const BLAST_FAR_SOUND: &str = "minecraft:entity.firework_rocket.blast_far";
pub const BLAST_VOLUME: f32 = 20.0;
pub const FAR_FROM_CAMERA_SQR: f64 = 256.0;
pub const DAMAGE_TYPE: &str = "minecraft:fireworks";
pub const BLAST_RADIUS: f64 = 5.0;
pub const SPREAD: f64 = 0.002297;
pub const RISE: f64 = 0.05;
pub const LIFT: f64 = 0.04;
pub const HORIZONTAL_SPEED_UP: f64 = 1.15;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Burst {
    pub position: DVec3,
    pub velocity: DVec3,
    pub explosions: Vec<Explosion>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RocketEvent {
    Launched,
    Exploded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RocketOwner {
    Mob(u64),
    Player(u64),
    Nobody,
}

impl RocketOwner {
    pub fn mob(self) -> Option<u64> {
        match self {
            Self::Mob(id) => Some(id),
            Self::Player(_) | Self::Nobody => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rocket {
    pub owner: RocketOwner,
    pub previous: DVec3,
    pub position: DVec3,
    pub velocity: DVec3,
    pub life: i32,
    pub lifetime: i32,
    pub explosions: Vec<Explosion>,
    pub alive: bool,
}

fn triangle(random: &mut LegacyRandom, deviation: f64) -> f64 {
    deviation * (random.next_double() - random.next_double())
}

impl Rocket {
    pub fn launch(
        owner: RocketOwner,
        position: DVec3,
        fireworks: Fireworks,
        random: &mut LegacyRandom,
    ) -> Self {
        let x = triangle(random, SPREAD);
        let z = triangle(random, SPREAD);
        let flight = fireworks.flight;
        let lifetime = 10 * (1 + flight) + random.next_int(6) as i32 + random.next_int(7) as i32;
        Self {
            owner,
            previous: position,
            position,
            velocity: DVec3::new(x, RISE, z),
            life: 0,
            lifetime,
            explosions: fireworks.explosions,
            alive: true,
        }
    }

    pub fn tick(&mut self, world: &dyn World) -> Vec<RocketEvent> {
        let mut events = Vec::new();
        self.previous = self.position;
        self.velocity = DVec3::new(
            self.velocity.x * HORIZONTAL_SPEED_UP,
            self.velocity.y,
            self.velocity.z * HORIZONTAL_SPEED_UP,
        ) + DVec3::Y * LIFT;
        let to = self.position + self.velocity;
        let hit = minecraftoss_entities::sight::clip(world, self.position, to);
        self.position = hit.map_or(to, |(_, at)| at);
        if self.life == 0 {
            events.push(RocketEvent::Launched);
        }
        self.life += 1;
        if hit.is_some() || self.life > self.lifetime {
            self.alive = false;
            events.push(RocketEvent::Exploded);
        }
        events
    }

    pub fn burst(&self) -> Burst {
        Burst {
            position: self.position,
            velocity: self.velocity,
            explosions: self.explosions.clone(),
        }
    }

    pub fn view(&self) -> ThrownView {
        ThrownView {
            item: ITEM.to_owned(),
            owner: crate::thrown::MOB_THROWER,
            previous: self.previous,
            position: self.position,
            age: self.life,
            yaw: 0.0,
            pitch: 0.0,
            tint: None,
        }
    }

    pub fn damage_to(&self, world: &dyn World, feet: DVec3, height: f64) -> Option<f32> {
        let stars = self.explosions.len();
        if stars == 0 {
            return None;
        }
        let strength = 5.0_f32 + (stars * 2) as f32;
        if self.position.distance_squared(feet) > BLAST_RADIUS * BLAST_RADIUS {
            return None;
        }
        let clear = (0..2).any(|i| {
            let target = feet + DVec3::Y * (height * 0.5 * f64::from(i));
            minecraftoss_entities::sight::clip(world, self.position, target).is_none()
        });
        let distance = self.position.distance_squared(feet).sqrt() as f32;
        clear
            .then(|| strength * ((BLAST_RADIUS - f64::from(distance)) / BLAST_RADIUS).sqrt() as f32)
    }
}

pub fn is_rocket(view: &ThrownView) -> bool {
    view.item == ITEM
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};

    struct Sky {
        roof: Option<i32>,
    }

    impl World for Sky {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64 || Some(pos.1) == self.roof).then(|| Block::new("minecraft:stone"))
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn rocket(flight: i32, seed: u64) -> Rocket {
        starred(flight, seed, vec![Explosion::burst(15790320)])
    }

    fn starred(flight: i32, seed: u64, explosions: Vec<Explosion>) -> Rocket {
        let mut random = LegacyRandom::new(seed);
        Rocket::launch(
            RocketOwner::Mob(3),
            DVec3::new(0.5, 65.62, 0.5),
            Fireworks { flight, explosions },
            &mut random,
        )
    }

    #[test]
    fn a_rocket_lives_ten_ticks_a_flight_level_and_up_to_eleven_more() {
        for seed in 0..200 {
            for flight in 0..3 {
                let rocket = rocket(flight, seed);
                let least = 10 * (1 + flight);
                assert!((least..=least + 11).contains(&rocket.lifetime));
                assert_eq!(rocket.velocity.y, RISE);
                assert!(rocket.velocity.x.abs() <= SPREAD && rocket.velocity.z.abs() <= SPREAD);
            }
        }
    }

    #[test]
    fn a_rocket_speeds_up_its_climb_and_explodes_once_its_life_is_over() {
        let world = Sky { roof: None };
        let mut rocket = rocket(0, 1);
        let lifetime = rocket.lifetime;
        let mut events = Vec::new();
        let mut ticks = 0;
        while rocket.alive {
            events.extend(rocket.tick(&world));
            ticks += 1;
        }
        assert_eq!(ticks, lifetime + 1);
        assert_eq!(events.first(), Some(&RocketEvent::Launched));
        assert_eq!(events.last(), Some(&RocketEvent::Exploded));
        assert_eq!(events.len(), 2);
        let n = f64::from(ticks);
        let risen = RISE * n + LIFT * n * (n + 1.0) / 2.0;
        assert!((rocket.position.y - (65.62 + risen)).abs() < 1e-9);
        let burst = rocket.burst();
        assert_eq!(burst.explosions, [Explosion::burst(15790320)]);
        assert!(burst.velocity.y > RISE);
    }

    #[test]
    fn a_rocket_that_meets_a_roof_explodes_there() {
        let world = Sky { roof: Some(68) };
        let mut rocket = rocket(2, 4);
        let mut ticks = 0;
        while rocket.alive {
            rocket.tick(&world);
            ticks += 1;
        }
        assert!(ticks < rocket.lifetime, "{ticks}");
        assert!(
            (rocket.position.y - 68.0).abs() < 1e-6,
            "{}",
            rocket.position
        );
    }

    #[test]
    fn its_blast_hurts_what_is_in_reach_and_in_the_open() {
        let world = Sky { roof: None };
        let mut rocket = rocket(0, 1);
        rocket.position = DVec3::new(0.5, 66.0, 0.5);
        let close = rocket
            .damage_to(&world, DVec3::new(0.5, 64.0, 0.5), 1.8)
            .unwrap();
        assert!((close - 7.0 * (0.6_f32).sqrt()).abs() < 1e-4, "{close}");
        assert!(
            rocket
                .damage_to(&world, DVec3::new(6.5, 64.0, 0.5), 1.8)
                .is_none()
        );
        let walled = Sky { roof: Some(65) };
        assert!(
            rocket
                .damage_to(&walled, DVec3::new(0.5, 64.0, 0.5), 1.8)
                .is_none()
        );
        assert!(is_rocket(&rocket.view()));
    }

    #[test]
    fn a_starless_rocket_hurts_no_one_and_each_star_adds_two() {
        let world = Sky { roof: None };
        let feet = DVec3::new(0.5, 64.0, 0.5);
        let mut plain = starred(1, 1, Vec::new());
        plain.position = feet;
        assert_eq!(plain.damage_to(&world, feet, 1.8), None);
        assert!(plain.burst().explosions.is_empty());
        let stars = vec![
            Explosion::burst(15790320),
            Explosion {
                shape: crate::firework::Shape::SmallBall,
                ..Explosion::burst(2437522)
            },
            Explosion {
                shape: crate::firework::Shape::LargeBall,
                ..Explosion::burst(14602026)
            },
        ];
        let mut three = starred(0, 1, stars);
        three.position = feet;
        assert_eq!(three.damage_to(&world, feet, 1.8), Some(11.0));
        let mut one = rocket(0, 1);
        one.position = feet;
        assert_eq!(one.damage_to(&world, feet, 1.8), Some(7.0));
        assert_eq!(three.burst().explosions.len(), 3);
    }
}
