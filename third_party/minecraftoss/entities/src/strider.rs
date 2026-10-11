use crate::age::Age;
use crate::fluid::FluidFrame;
use crate::health::{DamageResult, DamageState};
use crate::movement::Body;
use crate::steering::SteeringState;
use glam::DVec3;
use minecraftoss_player::World;

pub const MAX_HEALTH: f32 = 20.0;
pub const MOVEMENT_SPEED: f64 = 0.17499999701976776;
pub const SUFFOCATING_MODIFIER: f64 = -0.3400000035762787;
pub const WIDTH: f32 = 0.9;
pub const HEIGHT: f32 = 1.7;
pub const BABY_WIDTH: f32 = 0.45;
pub const BABY_HEIGHT: f32 = 0.85;
pub const EYE_HEIGHT: f32 = 1.445;
pub const BABY_EYE_HEIGHT: f32 = 0.4375;
pub const PASSENGER_ATTACHMENT: f64 = 1.7;
pub const BABY_PASSENGER_ATTACHMENT: f64 = 0.65625;
pub const FOOD_TAG: &str = "minecraft:strider_food";
pub const TEMPT_TAG: &str = "minecraft:strider_tempt_items";
pub const WARM_BLOCKS_TAG: &str = "minecraft:strider_warm_blocks";

#[derive(Clone, Debug)]
pub struct Strider {
    pub body: Body,
    pub yaw: f32,
    pub age: Age,
    pub in_love: i32,
    pub persistence_required: bool,
    pub health: f32,
    pub damage: DamageState,
    pub saddled: bool,
    pub suffocating: bool,
    pub steering: SteeringState,
}

impl Strider {
    pub fn new(position: DVec3) -> Self {
        let mut body = Body::new(position, WIDTH, HEIGHT);
        body.lava_floor = true;
        Self {
            body,
            yaw: 0.0,
            age: Age::default(),
            in_love: 0,
            persistence_required: false,
            health: MAX_HEALTH,
            damage: DamageState::default(),
            saddled: false,
            suffocating: false,
            steering: SteeringState::default(),
        }
    }

    pub fn dimensions(baby: bool) -> (f32, f32) {
        if baby {
            (BABY_WIDTH, BABY_HEIGHT)
        } else {
            (WIDTH, HEIGHT)
        }
    }

    pub fn sync_dimensions(&mut self) {
        let (width, height) = Self::dimensions(self.age.baby());
        self.body.width = width;
        self.body.height = height;
    }

    pub fn eye_height(&self) -> f32 {
        if self.age.baby() {
            BABY_EYE_HEIGHT
        } else {
            EYE_HEIGHT
        }
    }

    pub fn riding_attachment(baby: bool) -> f64 {
        if baby {
            BABY_PASSENGER_ATTACHMENT
        } else {
            PASSENGER_ATTACHMENT
        }
    }

    pub fn movement_speed(&self) -> f64 {
        if self.suffocating {
            MOVEMENT_SPEED + MOVEMENT_SPEED * SUFFOCATING_MODIFIER
        } else {
            MOVEMENT_SPEED
        }
    }

    pub fn hurt_generic(&mut self, amount: f32) -> DamageResult {
        self.damage
            .hurt_generic(&mut self.health, MAX_HEALTH, amount)
    }

    pub fn warm(&self, world: &impl World, fluid: FluidFrame, carried_warm: bool) -> bool {
        let p = self.body.position;
        let feet = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let below = (feet.0, (p.y - 1.0e-5).floor() as i32, feet.2);
        world.block_in_tag(feet, WARM_BLOCKS_TAG)
            || world.block_in_tag(below, WARM_BLOCKS_TAG)
            || fluid.in_lava()
            || carried_warm
    }

    pub fn float_in_lava(&mut self, world: &impl World, fluid: FluidFrame) {
        if !fluid.in_lava() {
            return;
        }
        let p = self.body.position;
        let pos = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let above = (pos.0, pos.1 + 1, pos.2);
        if stands_above_lava(p.y, pos.1) && !is_lava(world, above) {
            self.body.on_ground = true;
        } else {
            self.body.velocity = self.body.velocity * 0.5 + DVec3::new(0.0, 0.05, 0.0);
        }
    }
}

pub fn is_lava(world: &(impl World + ?Sized), pos: minecraftoss_player::Pos) -> bool {
    world.block(pos).is_some_and(|b| b.id == "minecraft:lava")
}

pub fn stands_above_lava(feet_y: f64, block_y: i32) -> bool {
    feet_y > f64::from(block_y) + 0.5 - f64::from(1.0e-5_f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::EntityWorld;
    use minecraftoss_player::{Block, Pos};
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Blocks {
        blocks: BTreeMap<Pos, Block>,
        rain: bool,
    }

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.blocks.get(&pos).cloned()
        }
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.blocks.insert(pos, block),
                None => self.blocks.remove(&pos),
            };
        }
        fn rain_at(&self, _pos: Pos) -> bool {
            self.rain
        }
    }

    fn lava(level: u32) -> Block {
        Block::new("minecraft:lava").with("level", &level.to_string())
    }

    fn floor(id: &str) -> Blocks {
        let mut world = Blocks::default();
        for x in -3..=3 {
            for z in -3..=3 {
                world.blocks.insert((x, 63, z), Block::new(id));
            }
        }
        world
    }

    fn spawn(world: &mut EntityWorld, y: f64) -> u64 {
        world.spawn_strider(Strider::new(DVec3::new(0.5, y, 0.5)), false)
    }

    fn tick(entities: &mut EntityWorld, world: &mut Blocks, ticks: usize) {
        for _ in 0..ticks {
            entities.tick_with_players(world, &[]);
        }
    }

    #[test]
    fn a_strider_on_a_lava_source_stands_half_a_block_up() {
        let mut world = floor("minecraft:netherrack");
        for x in -3..=3 {
            for z in -3..=3 {
                world.blocks.insert((x, 64, z), lava(0));
            }
        }
        let mut entities = EntityWorld::default();
        let id = spawn(&mut entities, 66.0);
        tick(&mut entities, &mut world, 60);
        let strider = &entities.strider_mut(id).unwrap().strider;
        assert!(
            (strider.body.position.y - 64.5).abs() < 1.0e-6,
            "{}",
            strider.body.position.y
        );
        assert!(strider.body.on_ground);
        assert!(!strider.suffocating);
        let mut flowing = floor("minecraft:netherrack");
        flowing.blocks.insert((0, 64, 0), lava(3));
        let mut entities = EntityWorld::default();
        let id = spawn(&mut entities, 64.6);
        tick(&mut entities, &mut flowing, 3);
        let sunk = entities.strider_mut(id).unwrap().strider.body.position.y;
        assert!(sunk < 64.6, "flowing lava holds nothing up: {sunk}");
        let lowest = (0..40)
            .map(|_| {
                tick(&mut entities, &mut flowing, 1);
                entities.strider_mut(id).unwrap().strider.body.position.y
            })
            .fold(f64::MAX, f64::min);
        let strider = &entities.strider_mut(id).unwrap().strider;
        assert!(
            strider.body.position.y > lowest,
            "it rises out of the lava again"
        );
        assert!(!strider.suffocating, "warm in lava");
    }

    #[test]
    fn only_striders_stand_on_lava() {
        let mut world = floor("minecraft:netherrack");
        world.blocks.insert((0, 64, 0), lava(0));
        let bounds =
            minecraftoss_player::collision::Bounds::standing(DVec3::new(0.5, 64.5, 0.5), 0.9, 1.7);
        let fall = DVec3::new(0.0, -0.3, 0.0);
        let strider = minecraftoss_player::collision::move_body_on_fluid(
            &world, bounds, fall, false, 0.6, true,
        );
        assert_eq!(strider.y, 0.0);
        let other = minecraftoss_player::collision::move_body(&world, bounds, fall, false, 0.6);
        assert_eq!(other.y, -0.3);
        world.blocks.insert((0, 65, 0), lava(0));
        let covered = minecraftoss_player::collision::move_body_on_fluid(
            &world, bounds, fall, false, 0.6, true,
        );
        assert_eq!(
            covered.y, -0.3,
            "lava above the source takes the floor away"
        );
    }

    #[test]
    fn out_of_lava_it_suffocates_and_slows_to_0_1155() {
        let mut world = floor("minecraft:netherrack");
        let mut entities = EntityWorld::default();
        let id = spawn(&mut entities, 64.0);
        assert_eq!(
            entities.strider_mut(id).unwrap().strider.movement_speed(),
            MOVEMENT_SPEED
        );
        tick(&mut entities, &mut world, 2);
        let strider = &entities.strider_mut(id).unwrap().strider;
        assert!(strider.suffocating);
        assert!(
            (strider.movement_speed() - 0.1155).abs() < 1.0e-6,
            "{}",
            strider.movement_speed()
        );
        world.blocks.insert((0, 64, 0), lava(0));
        tick(&mut entities, &mut world, 2);
        assert!(!entities.strider_mut(id).unwrap().strider.suffocating);
    }

    #[test]
    fn water_and_rain_hurt_it() {
        let mut world = floor("minecraft:netherrack");
        world
            .blocks
            .insert((0, 64, 0), Block::new("minecraft:water").with("level", "0"));
        let mut entities = EntityWorld::default();
        let id = spawn(&mut entities, 64.0);
        tick(&mut entities, &mut world, 1);
        assert_eq!(
            entities.strider_mut(id).unwrap().strider.health,
            MAX_HEALTH - 1.0
        );
        let mut world = floor("minecraft:netherrack");
        world.rain = true;
        let mut entities = EntityWorld::default();
        let id = spawn(&mut entities, 64.0);
        tick(&mut entities, &mut world, 1);
        assert_eq!(
            entities.strider_mut(id).unwrap().strider.health,
            MAX_HEALTH - 1.0
        );
        world.rain = false;
        tick(&mut entities, &mut world, 40);
        assert_eq!(
            entities.strider_mut(id).unwrap().strider.health,
            MAX_HEALTH - 1.0,
            "dry, it takes nothing more"
        );
    }

    #[test]
    fn a_baby_is_0_45_by_0_85() {
        let mut strider = Strider::new(DVec3::ZERO);
        assert_eq!((strider.body.width, strider.body.height), (0.9, 1.7));
        strider.age.set(Age::BABY_START);
        strider.sync_dimensions();
        assert_eq!((strider.body.width, strider.body.height), (0.45, 0.85));
        assert_eq!(strider.eye_height(), 0.4375);
        assert_eq!(Strider::riding_attachment(true), 0.65625);
        assert_eq!(Strider::riding_attachment(false), 1.7);
    }

    #[test]
    fn the_boost_factor_peaks_at_2_15_halfway() {
        let mut steering = SteeringState::default();
        assert_eq!(steering.boost_factor(), 1.0);
        let mut random = minecraftoss_player::rng::LegacyRandom::new(7);
        assert!(steering.boost(&mut random));
        assert!((140..=980).contains(&steering.boost_time_total));
        assert!(
            !steering.boost(&mut random),
            "a second boost fails meanwhile"
        );
        steering.boost_time_total = 200;
        assert_eq!(steering.boost_factor(), 1.0);
        steering.boost_time = 100;
        assert!((steering.boost_factor() - 2.15).abs() < 1.0e-5);
        steering.boost_time = 0;
        let mut ticks = 0;
        while steering.boosting {
            steering.tick_boost();
            ticks += 1;
        }
        assert_eq!(
            ticks, 202,
            "it ends on the tick after boost_time passes the total"
        );
    }

    #[test]
    fn striders_ignore_lava_and_fire() {
        let mut world = floor("minecraft:netherrack");
        world.blocks.insert((0, 64, 0), lava(2));
        world
            .blocks
            .insert((0, 65, 0), Block::new("minecraft:fire"));
        let mut entities = EntityWorld::default();
        let id = spawn(&mut entities, 64.0);
        tick(&mut entities, &mut world, 60);
        let strider = &entities.strider_mut(id).unwrap().strider;
        assert_eq!(strider.health, MAX_HEALTH);
        assert_eq!(strider.body.fire_ticks, 0);
        assert!(crate::world::fire_immune("minecraft:strider"));
    }
}
