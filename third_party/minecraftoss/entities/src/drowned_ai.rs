use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

pub const TRIDENT_INTERVAL: i32 = 40;
pub const TRIDENT_RADIUS: f32 = 10.0;
pub const SHOOT_SOUND: &str = "entity.drowned.shoot";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DrownedMove {
    #[default]
    None,
    Water,
    Trident,
    Beach,
    Stroll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Beach {
    pub block: (i32, i32, i32),
    pub try_ticks: i32,
    pub max_stay_ticks: i32,
    pub reached: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TridentThrow {
    pub owner: u64,
    pub position: DVec3,
    pub velocity: DVec3,
    pub item: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrownedGoals {
    pub running: DrownedMove,
    pub searching_for_land: bool,
    pub swim_up: Option<bool>,
    pub beach_next_start: i32,
    pub beach: Option<Beach>,
    pub attack_time: i32,
    pub see_time: i32,
    pub sea_level: i32,
    pub difficulty: i32,
    pub throw: Option<(DVec3, DVec3)>,
}

impl Default for DrownedGoals {
    fn default() -> Self {
        Self {
            running: DrownedMove::None,
            searching_for_land: false,
            swim_up: None,
            beach_next_start: 0,
            beach: None,
            attack_time: -1,
            see_time: 0,
            sea_level: 63,
            difficulty: 2,
            throw: None,
        }
    }
}

impl DrownedGoals {
    pub fn wants_to_swim(&self, target_in_water: bool) -> bool {
        self.searching_for_land || target_in_water
    }

    pub fn swim_up_can_use(&self, bright_outside: bool, in_water: bool, y: f64) -> bool {
        !bright_outside && in_water && y < f64::from(self.sea_level - 2)
    }

    pub fn beach_can_use(
        &mut self,
        world: &dyn World,
        feet: DVec3,
        bright_outside: bool,
        in_water: bool,
        random: &mut LegacyRandom,
    ) -> bool {
        if self.beach_next_start > 0 {
            self.beach_next_start -= 1;
            return false;
        }
        self.beach_next_start = (200 + random.next_int(200) as i32 + 1) / 2;
        let Some(block) = beach_block(world, feet) else {
            return false;
        };
        if bright_outside || !in_water || feet.y < f64::from(self.sea_level - 3) {
            return false;
        }
        self.beach = Some(Beach {
            block,
            try_ticks: 0,
            max_stay_ticks: 0,
            reached: false,
        });
        true
    }

    pub fn start_beach(&mut self, random: &mut LegacyRandom) -> Option<DVec3> {
        self.searching_for_land = false;
        let beach = self.beach.as_mut()?;
        beach.try_ticks = 0;
        let bound = random.next_int(1200) + 1200;
        beach.max_stay_ticks = random.next_int(bound) as i32 + 1200;
        self.running = DrownedMove::Beach;
        Some(beach_goal(beach.block))
    }

    pub fn beach_continues(&self, world: &dyn World) -> bool {
        self.beach.is_some_and(|beach| {
            beach.try_ticks >= -beach.max_stay_ticks
                && beach.try_ticks <= 1200
                && beach_target(world, beach.block)
        })
    }

    pub fn tick_beach(&mut self, feet: DVec3) -> Option<DVec3> {
        let beach = self.beach.as_mut()?;
        let goal = beach_goal(beach.block);
        let center = goal + DVec3::Y * 0.5;
        if center.distance_squared(feet) < 1.0 {
            beach.reached = true;
            beach.try_ticks -= 1;
            return None;
        }
        beach.reached = false;
        beach.try_ticks += 1;
        (beach.try_ticks % 40 == 0).then_some(goal)
    }

    pub fn stop_trident(&mut self) {
        self.attack_time = -1;
        self.see_time = 0;
    }

    pub fn tick_trident(&mut self, distance_sqr: f64, sees: bool) -> (bool, Option<f32>) {
        if sees {
            self.see_time += 1;
        } else {
            self.see_time = 0;
        }
        let near = distance_sqr <= f64::from(TRIDENT_RADIUS * TRIDENT_RADIUS) && self.see_time >= 5;
        self.attack_time -= 1;
        let reach = distance_sqr.sqrt() as f32 / TRIDENT_RADIUS;
        if self.attack_time == 0 {
            if !sees {
                return (near, None);
            }
            self.attack_time = TRIDENT_INTERVAL;
            return (near, Some(reach.clamp(0.1, 1.0)));
        }
        if self.attack_time < 0 {
            self.attack_time = TRIDENT_INTERVAL;
        }
        (near, None)
    }
}

pub fn uncertainty(difficulty: i32) -> f32 {
    (14 - difficulty * 4) as f32
}

pub fn shoot_pitch(random: &mut LegacyRandom) -> f32 {
    1.0 / (random.next_float() * 0.4 + 0.8)
}

fn block_of(feet: DVec3) -> (i32, i32, i32) {
    (
        feet.x.floor() as i32,
        feet.y.floor() as i32,
        feet.z.floor() as i32,
    )
}

pub fn water_position(world: &dyn World, feet: DVec3, random: &mut LegacyRandom) -> Option<DVec3> {
    let (x, y, z) = block_of(feet);
    for _ in 0..10 {
        let dx = random.next_int(20) as i32 - 10;
        let dy = 2 - random.next_int(8) as i32;
        let dz = random.next_int(20) as i32 - 10;
        let pos = (x + dx, y + dy, z + dz);
        if world
            .block(pos)
            .is_some_and(|block| block.id == "minecraft:water")
        {
            return Some(DVec3::new(
                f64::from(pos.0) + 0.5,
                f64::from(pos.1),
                f64::from(pos.2) + 0.5,
            ));
        }
    }
    None
}

fn empty(world: &dyn World, pos: (i32, i32, i32)) -> bool {
    world.block(pos).is_none_or(|block| {
        matches!(
            block.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

fn full_top(world: &dyn World, pos: (i32, i32, i32)) -> bool {
    world
        .collision_boxes(pos)
        .iter()
        .any(|b| b[0] <= 0.0 && b[2] <= 0.0 && b[3] >= 1.0 && b[5] >= 1.0 && b[4] >= 1.0)
}

pub fn beach_target(world: &dyn World, (x, y, z): (i32, i32, i32)) -> bool {
    empty(world, (x, y + 1, z)) && empty(world, (x, y + 2, z)) && full_top(world, (x, y, z))
}

fn beach_goal((x, y, z): (i32, i32, i32)) -> DVec3 {
    DVec3::new(f64::from(x) + 0.5, f64::from(y + 1), f64::from(z) + 0.5)
}

pub fn beach_block(world: &dyn World, feet: DVec3) -> Option<(i32, i32, i32)> {
    let (mx, my, mz) = block_of(feet);
    let mut y = 0;
    while y <= 2 {
        for r in 0..8 {
            let mut x = 0;
            while x <= r {
                let mut z = if x < r && x > -r { r } else { 0 };
                while z <= r {
                    let pos = (mx + x, my + y - 1, mz + z);
                    if beach_target(world, pos) {
                        return Some(pos);
                    }
                    z = if z > 0 { -z } else { 1 - z };
                }
                x = if x > 0 { -x } else { 1 - x };
            }
        }
        y = if y > 0 { -y } else { 1 - y };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};

    struct Shore;

    impl World for Shore {
        fn block(&self, (x, y, _): Pos) -> Option<Block> {
            if y < 60 || (x >= 5 && y <= 62) {
                Some(Block::new("minecraft:sand"))
            } else if y <= 62 {
                Some(Block::new("minecraft:water").with("level", "0"))
            } else {
                None
            }
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    #[test]
    fn a_drowned_finds_water_by_day_and_the_shore_from_the_water() {
        let mut found = 0;
        for seed in 0..40 {
            let mut random = LegacyRandom::new(seed);
            if let Some(water) = water_position(&Shore, DVec3::new(8.5, 63.0, 0.5), &mut random) {
                assert!(water.x < 5.0 && (60.0..=62.0).contains(&water.y), "{water}");
                assert_eq!(water.x - water.x.floor(), 0.5);
                found += 1;
            }
        }
        assert!(found > 20, "{found}");
        assert_eq!(
            beach_block(&Shore, DVec3::new(2.5, 61.0, 0.5)),
            Some((5, 62, 0))
        );
        assert!(beach_target(&Shore, (5, 62, 0)) && !beach_target(&Shore, (4, 62, 0)));
        assert_eq!(beach_block(&Shore, DVec3::new(-20.5, 61.0, 0.5)), None);
    }

    #[test]
    fn the_trident_goal_throws_every_two_seconds_while_it_sees_its_target() {
        let mut goals = DrownedGoals::default();
        let mut throws = Vec::new();
        for tick in 0..120 {
            let (_, power) = goals.tick_trident(64.0, true);
            if let Some(power) = power {
                throws.push((tick, power));
            }
        }
        assert_eq!(throws, vec![(40, 0.8), (80, 0.8)]);
        let (near, _) = goals.tick_trident(64.0, true);
        assert!(near);
        let (near, _) = goals.tick_trident(400.0, true);
        assert!(!near, "it walks closer from beyond ten blocks");
        goals.stop_trident();
        assert_eq!((goals.attack_time, goals.see_time), (-1, 0));
        assert_eq!(uncertainty(2), 6.0);
    }
}
