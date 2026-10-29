use crate::firework::{Explosion, Fireworks};
use crate::thrown::ThrownView;
use glam::DVec3;
use minecraftoss_entities::potion::PotionTarget;
pub use minecraftoss_entities::world::FireworkOwner as RocketOwner;
use minecraftoss_player::World;
use minecraftoss_player::collision::{Bounds, move_body};
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
pub const SIZE: f64 = 0.25;

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
    pub left_owner: bool,
    pub horizontal_collision: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RocketHit {
    Entity,
    Block,
}

fn triangle(random: &mut LegacyRandom, deviation: f64) -> f64 {
    deviation * (random.next_double() - random.next_double())
}

pub fn bounds_at(position: DVec3) -> (DVec3, DVec3) {
    let half = DVec3::new(SIZE / 2.0, 0.0, SIZE / 2.0);
    (position - half, position + half + DVec3::Y * SIZE)
}

fn swept((min, max): (DVec3, DVec3), by: DVec3) -> (DVec3, DVec3) {
    (min.min(min + by), max.max(max + by))
}

fn grown((min, max): (DVec3, DVec3), by: f64) -> (DVec3, DVec3) {
    (min - DVec3::splat(by), max + DVec3::splat(by))
}

fn overlaps(a: (DVec3, DVec3), b: (DVec3, DVec3)) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
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
            left_owner: false,
            horizontal_collision: false,
        }
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        bounds_at(self.position)
    }

    pub fn margin(&self) -> f64 {
        let tick_count = self.life + 1;
        f64::from(0.0_f32.max(0.3_f32.min((tick_count - 2) as f32 / 20.0)))
    }

    pub fn tick(
        &mut self,
        world: &impl World,
        targets: &[PotionTarget],
        owner: Option<(DVec3, DVec3)>,
    ) -> Vec<RocketEvent> {
        let mut events = Vec::new();
        self.previous = self.position;
        if !self.left_owner {
            let near = grown(swept(self.bounds(), self.velocity), 1.0);
            self.left_owner = owner.is_none_or(|owner| !overlaps(owner, near));
        }
        let speed_up = if self.horizontal_collision {
            1.0
        } else {
            HORIZONTAL_SPEED_UP
        };
        self.velocity = DVec3::new(
            self.velocity.x * speed_up,
            self.velocity.y,
            self.velocity.z * speed_up,
        ) + DVec3::Y * LIFT;
        let delta = self.velocity;
        let hit = self.hit_on_move(world, targets, delta);
        let (min, max) = self.bounds();
        let moved = move_body(world, Bounds { min, max }, delta, false, 0.0);
        self.position += moved;
        let same = |a: f64, b: f64| (b - a).abs() < f64::from(1.0e-5_f32);
        self.horizontal_collision = !same(delta.x, moved.x) || !same(delta.z, moved.z);
        let bursts = match hit {
            Some(RocketHit::Entity) => true,
            Some(RocketHit::Block) => !self.explosions.is_empty(),
            None => false,
        };
        if bursts {
            self.alive = false;
            events.push(RocketEvent::Exploded);
        }
        if self.life == 0 {
            events.push(RocketEvent::Launched);
        }
        self.life += 1;
        if self.alive && self.life > self.lifetime {
            self.alive = false;
            events.push(RocketEvent::Exploded);
        }
        events
    }

    fn hit_on_move(
        &self,
        world: &impl World,
        targets: &[PotionTarget],
        delta: DVec3,
    ) -> Option<RocketHit> {
        let from = self.position;
        let block = minecraftoss_entities::sight::clip(world, from, from + delta);
        let to = block.map_or(from + delta, |(_, at)| at);
        let search = grown(swept(self.bounds(), delta), 1.0);
        let margin = self.margin();
        let struck = targets
            .iter()
            .filter(|target| self.left_owner || !target.owner)
            .filter(|target| overlaps((target.min, target.max), search))
            .any(|target| {
                let (min, max) = grown((target.min, target.max), margin);
                minecraftoss_entities::sight::clip_box(min, max, from, to).is_some()
            });
        if struck {
            Some(RocketHit::Entity)
        } else {
            block.map(|_| RocketHit::Block)
        }
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
            events.extend(rocket.tick(&world, &[], None));
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
            rocket.tick(&world, &[], None);
            ticks += 1;
        }
        assert!(ticks < rocket.lifetime, "{ticks}");
        assert!(
            (rocket.position.y - (68.0 - SIZE)).abs() < 1e-6,
            "{}",
            rocket.position
        );
    }

    struct Wall;

    impl World for Wall {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64 || pos.0 >= 1).then(|| Block::new("minecraft:stone"))
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn zombie_at(feet: DVec3) -> PotionTarget {
        PotionTarget {
            id: 9,
            min: feet - DVec3::new(0.3, 0.0, 0.3),
            max: feet + DVec3::new(0.3, 1.95, 0.3),
            owner: false,
        }
    }

    fn fly(
        rocket: &mut Rocket,
        world: &impl World,
        targets: &[PotionTarget],
        owner: Option<(DVec3, DVec3)>,
    ) -> (i32, Vec<RocketEvent>) {
        let mut events = Vec::new();
        let mut ticks = 0;
        while rocket.alive {
            events.extend(rocket.tick(world, targets, owner));
            ticks += 1;
        }
        (ticks, events)
    }

    #[test]
    fn a_rocket_bursts_on_the_first_tick_at_a_zombie_just_above_it() {
        let world = Sky { roof: None };
        let mut rising = rocket(1, 2);
        let events = rising.tick(&world, &[zombie_at(DVec3::new(0.5, 65.7, 0.5))], None);
        assert!(!rising.alive);
        assert_eq!(events, [RocketEvent::Exploded, RocketEvent::Launched]);
        assert!(
            (rising.position.y - (65.62 + RISE + LIFT)).abs() < 1e-9,
            "it bursts where its move ends: {}",
            rising.position
        );
    }

    #[test]
    fn a_rocket_launched_inside_a_zombie_rises_through_it() {
        let world = Sky { roof: None };
        let mut inside = rocket(1, 2);
        let lifetime = inside.lifetime;
        let zombie = zombie_at(DVec3::new(0.5, 65.0, 0.5));
        let (ticks, events) = fly(&mut inside, &world, &[zombie], None);
        assert_eq!(ticks, lifetime + 1);
        assert_eq!(events, [RocketEvent::Launched, RocketEvent::Exploded]);
    }

    #[test]
    fn a_rocket_passes_its_owner_until_it_has_left_it() {
        let world = Sky { roof: None };
        let owner = PotionTarget {
            owner: true,
            ..zombie_at(DVec3::new(0.5, 65.7, 0.5))
        };
        let mut own = rocket(1, 2);
        let lifetime = own.lifetime;
        let (ticks, _) = fly(&mut own, &world, &[owner], Some((owner.min, owner.max)));
        assert_eq!(ticks, lifetime + 1, "it never bursts on its owner");
        assert!(own.left_owner);
        let mut stranger = rocket(1, 2);
        let events = stranger.tick(&world, &[PotionTarget { owner: false, ..owner }], None);
        assert!(events.contains(&RocketEvent::Exploded));
        let mut far = rocket(1, 2);
        far.tick(&world, &[], Some((DVec3::splat(20.0), DVec3::splat(21.0))));
        assert!(far.left_owner && far.alive);
    }

    #[test]
    fn a_starless_rocket_under_a_roof_waits_out_its_lifetime() {
        let world = Sky { roof: Some(68) };
        let mut plain = starred(1, 4, Vec::new());
        let lifetime = plain.lifetime;
        let (ticks, events) = fly(&mut plain, &world, &[], None);
        assert_eq!(ticks, lifetime + 1);
        assert_eq!(events, [RocketEvent::Launched, RocketEvent::Exploded]);
        assert!(
            (plain.position.y - (68.0 - SIZE)).abs() < 1e-6,
            "{}",
            plain.position
        );
        assert!(plain.burst().explosions.is_empty());
    }

    #[test]
    fn a_rocket_against_a_wall_stops_speeding_up_sideways() {
        let mut plain = starred(1, 1, Vec::new());
        plain.position = DVec3::new(0.8, 65.0, 0.5);
        plain.velocity = DVec3::new(0.1, RISE, 0.0);
        plain.tick(&Wall, &[], None);
        assert!(plain.horizontal_collision);
        assert!((plain.position.x - (1.0 - SIZE / 2.0)).abs() < 1e-9, "{}", plain.position);
        let pressed = plain.velocity.x;
        assert!((pressed - 0.1 * HORIZONTAL_SPEED_UP).abs() < 1e-12);
        plain.tick(&Wall, &[], None);
        assert_eq!(plain.velocity.x, pressed, "no speed-up against the wall");
        assert!(plain.alive, "a starless rocket does not burst on the wall");
        let mut starry = rocket(1, 1);
        starry.position = DVec3::new(0.85, 65.0, 0.5);
        starry.velocity = DVec3::new(0.2, RISE, 0.0);
        assert!(starry.tick(&Wall, &[], None).contains(&RocketEvent::Exploded));
        assert!((starry.position.x - (1.0 - SIZE / 2.0)).abs() < 1e-9);
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
