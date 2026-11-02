use crate::firework::{Explosion, Fireworks};
use crate::thrown::ThrownView;
use glam::DVec3;
use minecraftoss_entities::controls::PlayerControls;
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
pub const SHOT_SPREAD: f64 = 0.0172275;
pub const DISPENSE_POWER: f32 = 0.5;
pub const DISPENSE_UNCERTAINTY: f32 = 1.0;
pub const HAND_REACH: f64 = 0.5;
pub const HAND_TURN: f32 = 80.0;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Burst {
    pub position: DVec3,
    pub velocity: DVec3,
    pub explosions: Vec<Explosion>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RocketEvent {
    Launched,
    Exploded,
    Block {
        pos: minecraftoss_player::Pos,
        at: DVec3,
        velocity: DVec3,
    },
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
    pub shot_at_angle: bool,
    pub rider: Option<u64>,
    pub hand: DVec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum RocketHit {
    Entity,
    Block(minecraftoss_player::Pos, DVec3),
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

pub fn hand_offset(controls: Option<&PlayerControls>) -> DVec3 {
    let Some(controls) = controls else {
        return DVec3::ZERO;
    };
    let holds = |item: &Option<String>| item.as_deref() == Some(ITEM);
    let turn = if holds(&controls.offhand) && !holds(&controls.main_hand) {
        -HAND_TURN
    } else {
        HAND_TURN
    };
    minecraftoss_entities::enderman::view_vector(0.0, controls.yaw + turn) * HAND_REACH
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
            shot_at_angle: false,
            rider: None,
            hand: DVec3::ZERO,
        }
    }

    pub fn attached(
        rider: u64,
        feet: DVec3,
        hand: DVec3,
        boost_ticks: i32,
        fireworks: Fireworks,
        random: &mut LegacyRandom,
    ) -> Self {
        let mut rocket = Self::launch(RocketOwner::Player(rider), feet + hand, fireworks, random);
        rocket.lifetime = boost_ticks - 1;
        rocket.rider = Some(rider);
        rocket.hand = hand;
        rocket
    }

    pub fn shot(
        owner: RocketOwner,
        position: DVec3,
        direction: DVec3,
        power: f32,
        uncertainty: f32,
        fireworks: Fireworks,
        random: &mut LegacyRandom,
    ) -> Self {
        let mut rocket = Self::launch(owner, position, fireworks, random);
        let spread = SHOT_SPREAD * f64::from(uncertainty);
        let x = triangle(random, spread);
        let y = triangle(random, spread);
        let z = triangle(random, spread);
        rocket.velocity = (direction.normalize() + DVec3::new(x, y, z)) * f64::from(power);
        rocket.shot_at_angle = true;
        rocket
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
        self.previous = self.position;
        if !self.left_owner {
            let near = grown(swept(self.bounds(), self.velocity), 1.0);
            self.left_owner = owner.is_none_or(|owner| !overlaps(owner, near));
        }
        if !self.shot_at_angle {
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
        }
        let delta = self.velocity;
        let hit = self.hit_on_move(world, targets, delta);
        let (min, max) = self.bounds();
        let moved = move_body(world, Bounds { min, max }, delta, false, 0.0);
        self.position += moved;
        let same = |a: f64, b: f64| (b - a).abs() < f64::from(1.0e-5_f32);
        self.horizontal_collision = !same(delta.x, moved.x) || !same(delta.z, moved.z);
        self.after_hit(hit)
    }

    pub fn follow(
        &mut self,
        world: &impl World,
        targets: &[PotionTarget],
        rider: Option<(DVec3, DVec3)>,
    ) -> Vec<RocketEvent> {
        self.previous = self.position;
        if let Some((feet, hand)) = rider {
            self.velocity = feet - (self.position - self.hand);
            self.hand = hand;
            self.position = feet + hand;
        }
        let hit = self.hit_on_move(world, targets, self.velocity);
        self.after_hit(hit)
    }

    fn after_hit(&mut self, hit: Option<RocketHit>) -> Vec<RocketEvent> {
        let mut events = Vec::new();
        let bursts = match hit {
            Some(RocketHit::Entity) => true,
            Some(RocketHit::Block(..)) => !self.explosions.is_empty(),
            None => false,
        };
        if bursts {
            self.alive = false;
            events.push(RocketEvent::Exploded);
        }
        if let Some(RocketHit::Block(pos, at)) = hit {
            events.push(RocketEvent::Block {
                pos,
                at,
                velocity: self.velocity,
            });
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
            block.map(|(pos, at)| RocketHit::Block(pos, at))
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
            owner: self.rider.unwrap_or(crate::thrown::MOB_THROWER),
            previous: self.previous,
            position: self.position,
            age: self.life,
            yaw: 0.0,
            pitch: 0.0,
            tint: None,
        }
    }

    pub fn strength(&self) -> Option<f32> {
        let stars = self.explosions.len();
        (stars > 0).then(|| 5.0_f32 + (stars * 2) as f32)
    }

    pub fn damage_to(&self, world: &dyn World, feet: DVec3, height: f64) -> Option<f32> {
        let strength = self.strength()?;
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

pub fn is_attached(view: &ThrownView) -> bool {
    is_rocket(view) && view.owner != crate::thrown::MOB_THROWER
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
        let (roof, events): (Vec<_>, Vec<_>) = events
            .into_iter()
            .partition(|event| matches!(event, RocketEvent::Block { .. }));
        assert_eq!(events, [RocketEvent::Launched, RocketEvent::Exploded]);
        assert!(!roof.is_empty());
        assert!(
            roof.iter().all(|event| matches!(event, RocketEvent::Block { pos: (0, 68, 0), at, .. } if (at.y - 68.0).abs() < 1e-6)),
            "it strikes the roof each tick it presses on it: {roof:?}"
        );
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
    fn a_rocket_reports_the_block_it_strikes_and_a_starless_one_strikes_it_every_tick() {
        let mut starry = rocket(1, 1);
        starry.position = DVec3::new(0.85, 65.0, 0.5);
        starry.velocity = DVec3::new(0.2, RISE, 0.0);
        let events = starry.tick(&Wall, &[], None);
        assert_eq!(events.len(), 3, "{events:?}");
        assert_eq!(
            (events[0], events[2]),
            (RocketEvent::Exploded, RocketEvent::Launched)
        );
        let RocketEvent::Block { pos, at, velocity } = events[1] else {
            panic!("{events:?}");
        };
        assert_eq!((pos, velocity), ((1, 65, 0), starry.velocity));
        let rise = (RISE + LIFT) * 0.15 / (0.2 * HORIZONTAL_SPEED_UP);
        assert!(
            (at - DVec3::new(1.0, 65.0 + rise, 0.5)).length() < 1e-6,
            "{at}"
        );
        let mut plain = starred(1, 1, Vec::new());
        plain.position = DVec3::new(0.8, 65.0, 0.5);
        plain.velocity = DVec3::new(0.2, RISE, 0.0);
        for _ in 0..3 {
            let events = plain.tick(&Wall, &[], None);
            let struck = events
                .iter()
                .filter(|event| matches!(event, RocketEvent::Block { pos: (1, 65, 0), .. }))
                .count();
            assert_eq!(struck, 1, "{events:?}");
            assert!(plain.alive);
        }
    }

    #[test]
    fn a_rocket_shot_at_an_angle_flies_straight_without_lift_or_speed_up() {
        let world = Sky { roof: None };
        let start = DVec3::new(0.5, 70.5, 0.5);
        let mut random = LegacyRandom::new(3);
        let plain = Fireworks {
            flight: 1,
            explosions: Vec::new(),
        };
        let mut shot = Rocket::shot(
            RocketOwner::Nobody,
            start,
            DVec3::X,
            DISPENSE_POWER,
            DISPENSE_UNCERTAINTY,
            plain,
            &mut random,
        );
        assert!(shot.shot_at_angle);
        let velocity = shot.velocity;
        let wobble = f64::from(DISPENSE_POWER) * SHOT_SPREAD;
        assert!(velocity.x > 0.49 && velocity.x < 0.51, "{velocity}");
        assert!(
            velocity.y.abs() < wobble && velocity.z.abs() < wobble,
            "{velocity}"
        );
        let lifetime = shot.lifetime;
        assert!((20..=31).contains(&lifetime));
        let (ticks, events) = fly(&mut shot, &world, &[], None);
        assert_eq!(ticks, lifetime + 1);
        assert_eq!(events, [RocketEvent::Launched, RocketEvent::Exploded]);
        assert_eq!(shot.velocity, velocity, "no lift and no speed-up");
        assert!(
            (shot.position - (start + velocity * f64::from(ticks))).length() < 1e-9,
            "{}",
            shot.position
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

    fn rider_at(id: u64, feet: DVec3) -> PotionTarget {
        PotionTarget {
            id: crate::thrown::PLAYER_TARGET + id,
            min: feet - DVec3::new(0.3, 0.0, 0.3),
            max: feet + DVec3::new(0.3, 1.8, 0.3),
            owner: true,
        }
    }

    fn holding(main_hand: Option<&str>, offhand: Option<&str>, yaw: f32) -> PlayerControls {
        PlayerControls {
            id: 4,
            yaw,
            main_hand: main_hand.map(str::to_owned),
            offhand: offhand.map(str::to_owned),
            ..PlayerControls::default()
        }
    }

    #[test]
    fn the_rocket_sits_half_a_block_out_in_the_hand_that_holds_it() {
        assert_eq!(hand_offset(None), DVec3::ZERO);
        let right = hand_offset(Some(&holding(Some(ITEM), None, 0.0)));
        let turned = (80.0_f64).to_radians();
        assert!(
            (right - DVec3::new(-turned.sin(), 0.0, turned.cos()) * 0.5).length() < 1e-3,
            "facing south the right hand is west: {right}"
        );
        let left = hand_offset(Some(&holding(None, Some(ITEM), 0.0)));
        assert!(
            (left - DVec3::new(turned.sin(), 0.0, turned.cos()) * 0.5).length() < 1e-3,
            "{left}"
        );
        assert_eq!(
            hand_offset(Some(&holding(Some(ITEM), Some(ITEM), 0.0))),
            right
        );
        assert_eq!(hand_offset(Some(&holding(None, None, 0.0))), right);
        let east = hand_offset(Some(&holding(Some(ITEM), None, -90.0)));
        let ten = (10.0_f64).to_radians();
        assert!(
            (east - DVec3::new(ten.sin(), 0.0, ten.cos()) * 0.5).length() < 1e-3,
            "facing east the right hand is south: {east}"
        );
    }

    #[test]
    fn an_elytra_rocket_rides_at_its_riders_hand_and_bursts_when_the_boost_ends() {
        let world = Sky { roof: None };
        let hand = DVec3::new(-0.49, 0.0, 0.09);
        let feet = |tick: i32| {
            DVec3::new(
                0.5,
                100.0 - 0.2 * f64::from(tick),
                0.5 + 1.5 * f64::from(tick),
            )
        };
        let mut random = LegacyRandom::new(7);
        let one_star = Fireworks {
            flight: 1,
            explosions: vec![Explosion::burst(11743532)],
        };
        let mut rocket = Rocket::attached(4, feet(0), hand, 21, one_star, &mut random);
        assert_eq!(
            (rocket.lifetime, rocket.rider, rocket.owner),
            (20, Some(4), RocketOwner::Player(4))
        );
        assert!((rocket.position - (feet(0) + hand)).length() < 1e-12);
        assert!(is_attached(&rocket.view()) && rocket.view().owner == 4);
        let mut events = Vec::new();
        let mut ticks = 0;
        while rocket.alive {
            ticks += 1;
            let rider = rider_at(4, feet(ticks));
            events.extend(rocket.follow(&world, &[rider], Some((feet(ticks), hand))));
            assert!(
                (rocket.position - (feet(ticks) + hand)).length() < 1e-9,
                "{}",
                rocket.position
            );
            assert!(
                (rocket.velocity - DVec3::new(0.0, -0.2, 1.5)).length() < 1e-9,
                "{}",
                rocket.velocity
            );
        }
        assert_eq!(ticks, 21, "it bursts on the last tick of the boost");
        assert_eq!(
            events,
            [RocketEvent::Launched, RocketEvent::Exploded],
            "it never hits its rider"
        );
        assert_eq!(rocket.strength(), Some(7.0));
        assert!((rocket.burst().velocity - DVec3::new(0.0, -0.2, 1.5)).length() < 1e-9);
        let mut plain = Rocket::attached(5, feet(0), hand, 21, Fireworks::default(), &mut random);
        while plain.alive {
            plain.follow(&world, &[], Some((feet(0), hand)));
        }
        assert_eq!(
            (plain.life, plain.strength()),
            (21, None),
            "a starless one only puffs"
        );
        assert!(plain.burst().explosions.is_empty());
    }

    #[test]
    fn an_elytra_rocket_bursts_early_on_a_mob_in_its_riders_path_and_stays_put_without_its_rider() {
        let world = Sky { roof: None };
        let feet = |tick: i32| DVec3::new(0.5, 100.0, 0.5 + f64::from(tick));
        let mut random = LegacyRandom::new(7);
        let mut rocket = Rocket::attached(
            4,
            feet(0),
            DVec3::ZERO,
            21,
            Fireworks {
                flight: 1,
                explosions: vec![Explosion::burst(1)],
            },
            &mut random,
        );
        let zombie = zombie_at(DVec3::new(0.5, 99.5, 6.0));
        let mut ticks = 0;
        while rocket.alive {
            ticks += 1;
            rocket.follow(
                &world,
                &[rider_at(4, feet(ticks)), zombie],
                Some((feet(ticks), DVec3::ZERO)),
            );
        }
        assert!(ticks < 6, "it meets the zombie ahead: {ticks}");
        let mut alone = Rocket::attached(
            4,
            feet(0),
            DVec3::ZERO,
            21,
            Fireworks::default(),
            &mut random,
        );
        alone.follow(&world, &[], Some((feet(1), DVec3::ZERO)));
        let left = alone.position;
        alone.follow(&world, &[], None);
        assert_eq!(
            (alone.position, alone.previous),
            (left, left),
            "its rider is gone"
        );
    }
}
