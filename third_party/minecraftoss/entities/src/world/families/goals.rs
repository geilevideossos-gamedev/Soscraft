use crate::goals::{Control, Controls, Goal};
use crate::monster_ai::MonsterGoalContext;
use crate::navigation::navigate_walk_to;
use crate::stroll::default_random_position;
use glam::DVec3;
use minecraftoss_player::World;

pub const POWDER_SNOW: &str = "minecraft:powder_snow";

pub fn reduced_tick_delay(ticks: u32) -> u32 {
    ticks.div_ceil(2)
}

pub fn in_powder_snow(world: &dyn World, position: DVec3, width: f32, height: f32) -> bool {
    let (half, inset) = (f64::from(width) / 2.0, 1.0e-5);
    let low = (
        (position.x - half + inset).floor() as i32,
        (position.y + inset).floor() as i32,
        (position.z - half + inset).floor() as i32,
    );
    let high = (
        (position.x + half - inset).floor() as i32,
        (position.y + f64::from(height) - inset).floor() as i32,
        (position.z + half - inset).floor() as i32,
    );
    (low.0..=high.0).any(|x| {
        (low.1..=high.1).any(|y| {
            (low.2..=high.2).any(|z| world.block((x, y, z)).is_some_and(|b| b.id == POWDER_SNOW))
        })
    })
}

#[derive(Clone)]
pub struct ClimbOnTopOfPowderSnowGoal;

impl Goal<MonsterGoalContext> for ClimbOnTopOfPowderSnowGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Jump])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let body = &ctx.body;
        if !in_powder_snow(world, body.position, body.width, body.height) {
            return false;
        }
        let p = body.position;
        let above = (
            p.x.floor() as i32,
            p.y.floor() as i32 + 1,
            p.z.floor() as i32,
        );
        world.block(above).is_some_and(|b| b.id == POWDER_SNOW)
            || world.collision_boxes(above).is_empty()
    }

    fn every_tick(&self) -> bool {
        true
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.jump = true;
    }
}

#[derive(Clone)]
pub struct RandomStrollGoal {
    pub speed: f64,
    pub interval: u32,
    wanted: Option<DVec3>,
}

impl RandomStrollGoal {
    pub fn new(speed: f64, interval: u32) -> Self {
        Self {
            speed,
            interval,
            wanted: None,
        }
    }
}

impl Goal<MonsterGoalContext> for RandomStrollGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.no_action_time >= 100 || ctx.random.next_int(reduced_tick_delay(self.interval)) != 0
        {
            return false;
        }
        let position = ctx.body.position;
        self.wanted = default_random_position(world, &ctx.walk, position, 10, 7, &mut ctx.random);
        self.wanted.is_some()
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if let Some(target) = self.wanted {
            let profile = ctx.profile();
            let _ = navigate_walk_to(
                &ctx.body,
                &mut ctx.navigation,
                world,
                &profile,
                ctx.fluid,
                target,
                self.speed,
                1,
            );
        }
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.navigation.stop();
    }
}
