use crate::goals::{Control, Controls, Goal};
use crate::monster_ai::MonsterGoalContext;
use crate::navigation::navigate_walk_to;
use crate::poi::{Occupancy, PoiType};
use glam::DVec3;
use minecraftoss_player::{Pos, World};

pub const VALID_RAID_RADIUS_SQR: f64 = 9216.0;
pub const RAID_REMOVAL_THRESHOLD_SQR: f64 = 12544.0;
pub const MAX_NO_ACTION_TIME: i32 = 2400;
pub const RECRUITMENT_DELAY: i32 = 20;
pub const RECRUIT_RANGE: f64 = 16.0;
pub const PATHFIND_SPEED: f64 = 1.0;
pub const PATHFIND_HORIZONTAL: u32 = 15;
pub const PATHFIND_VERTICAL: u32 = 4;
pub const VILLAGE_SPEED: f64 = 1.0499999523162842;
pub const VILLAGE_DISTANCE: i32 = 1;
pub const HOME_RANGE: i32 = 48;
pub const VISITED_KEPT: usize = 2;
pub const CELEBRATE_SOUND_ROLL: u32 = 50;
pub const CELEBRATE_JUMP_ROLL: u32 = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RaidView {
    pub id: i32,
    pub center: Pos,
    pub active: bool,
    pub over: bool,
    pub loss: bool,
    pub groups_spawned: i32,
}

impl RaidView {
    pub fn ongoing(&self) -> bool {
        self.active && !self.over
    }
}

pub fn dist_sqr(a: Pos, b: Pos) -> f64 {
    let (dx, dy, dz) = (
        f64::from(a.0 - b.0),
        f64::from(a.1 - b.1),
        f64::from(a.2 - b.2),
    );
    dx * dx + dy * dy + dz * dz
}

pub fn nearby_raid(raids: &[RaidView], pos: Pos) -> Option<&RaidView> {
    let mut best = None;
    let mut nearest = VALID_RAID_RADIUS_SQR;
    for raid in raids.iter().filter(|raid| raid.active) {
        let distance = dist_sqr(raid.center, pos);
        if distance < nearest {
            best = Some(raid);
            nearest = distance;
        }
    }
    best
}

pub fn block_of(position: DVec3) -> Pos {
    (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

pub fn shuffle<T>(list: &mut [T], random: &mut minecraftoss_player::rng::LegacyRandom) {
    for i in (2..=list.len()).rev() {
        let j = random.next_int(i as u32) as usize;
        list.swap(i - 1, j);
    }
}

fn ongoing(ctx: &MonsterGoalContext) -> Option<RaidView> {
    ctx.illager.raid.filter(RaidView::ongoing)
}

fn in_village(ctx: &mut MonsterGoalContext) -> bool {
    let here = block_of(ctx.body.position);
    ctx.pois.as_mut().is_some_and(|pois| pois.is_village(here))
}

fn walk_to(ctx: &mut MonsterGoalContext, world: &dyn World, at: DVec3, speed: f64) -> bool {
    let profile = ctx.profile();
    navigate_walk_to(
        &ctx.body,
        &mut ctx.navigation,
        world,
        &profile,
        ctx.fluid,
        at,
        speed,
        1,
    )
    .unwrap_or(false)
}

fn bottom_center(pos: Pos) -> DVec3 {
    DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1),
        f64::from(pos.2) + 0.5,
    )
}

fn towards(
    ctx: &mut MonsterGoalContext,
    world: &dyn World,
    (horizontal, vertical): (u32, u32),
    to: DVec3,
    max_radians: f32,
) -> Option<DVec3> {
    let profile = ctx.walk.clone();
    crate::stroll::default_random_position_towards(
        world,
        &profile,
        ctx.body.position,
        horizontal,
        vertical,
        to,
        max_radians,
        &mut ctx.random,
    )
}

#[derive(Clone, Default)]
pub struct PathfindToRaidGoal {
    recruitment_tick: i32,
}

impl Goal<MonsterGoalContext> for PathfindToRaidGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.target().is_none() && ongoing(ctx).is_some() && !in_village(ctx)
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ongoing(ctx).is_some() && !in_village(ctx)
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(raid) = ctx.illager.raid.filter(|raid| raid.active) else {
            return;
        };
        if ctx.tick_count > self.recruitment_tick {
            self.recruitment_tick = ctx.tick_count + RECRUITMENT_DELAY;
            ctx.illager.recruit = true;
        }
        if !ctx.navigation.is_done() {
            return;
        }
        let horizontal = (PATHFIND_HORIZONTAL, PATHFIND_VERTICAL);
        let center = bottom_center(raid.center);
        let spot = towards(ctx, world, horizontal, center, std::f32::consts::FRAC_PI_2);
        if let Some(spot) = spot {
            walk_to(ctx, world, spot, PATHFIND_SPEED);
        }
    }
}

#[derive(Clone, Default)]
pub struct MoveThroughVillageGoal {
    poi: Option<Pos>,
    visited: Vec<Pos>,
    stuck: bool,
}

impl MoveThroughVillageGoal {
    fn suitable_poi(&mut self, ctx: &mut MonsterGoalContext) -> bool {
        let here = block_of(ctx.body.position);
        let Some(pois) = ctx.pois.as_ref() else {
            return false;
        };
        let home = |kind: PoiType| kind == PoiType::Home;
        let mut homes: Vec<Pos> = pois
            .in_range(&home, here, HOME_RANGE, Occupancy::Any)
            .into_iter()
            .map(|record| record.pos)
            .collect();
        shuffle(&mut homes, &mut ctx.random);
        let found = homes.into_iter().find(|pos| !self.visited.contains(pos));
        self.poi = found;
        found.is_some()
    }

    fn close(&self, ctx: &MonsterGoalContext, distance: f64) -> bool {
        self.poi.is_some_and(|poi| {
            let centre = bottom_center(poi) + DVec3::Y * 0.5;
            centre.distance_squared(ctx.body.position) < distance * distance
        })
    }
}

impl Goal<MonsterGoalContext> for MoveThroughVillageGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        if self.visited.len() > VISITED_KEPT {
            self.visited.remove(0);
        }
        ongoing(ctx).is_some() && self.suitable_poi(ctx) && ctx.target().is_none()
    }
    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        if ctx.navigation.is_done() {
            return false;
        }
        let reach = f64::from(ctx.body.width) + f64::from(VILLAGE_DISTANCE);
        ctx.target().is_none() && !self.close(ctx, reach) && !self.stuck
    }
    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        ctx.no_action_time = 0;
        ctx.illager.reset_idle = true;
        if let Some(poi) = self.poi {
            let at = DVec3::new(f64::from(poi.0), f64::from(poi.1), f64::from(poi.2));
            walk_to(ctx, world, at, VILLAGE_SPEED);
        }
        self.stuck = false;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if self.close(ctx, f64::from(VILLAGE_DISTANCE)) {
            if let Some(poi) = self.poi {
                self.visited.push(poi);
            }
        }
    }
    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        if !ctx.navigation.is_done() {
            return;
        }
        let Some(poi) = self.poi else {
            return;
        };
        let target = bottom_center(poi);
        let spot = towards(ctx, world, (16, 7), target, 0.31415927)
            .or_else(|| towards(ctx, world, (8, 7), target, std::f32::consts::FRAC_PI_2));
        match spot {
            Some(spot) => {
                walk_to(ctx, world, spot, VILLAGE_SPEED);
            }
            None => self.stuck = true,
        }
    }
}

#[derive(Clone, Default)]
pub struct CelebrationGoal;

impl Goal<MonsterGoalContext> for CelebrationGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.health > 0.0 && ctx.target().is_none() && ctx.illager.raid.is_some_and(|raid| raid.loss)
    }
    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.illager.celebrating = true;
    }
    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.illager.celebrating = false;
    }
    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        if ctx.random.next_int(CELEBRATE_SOUND_ROLL) == 0 {
            ctx.illager.celebrate_sound = true;
        }
        if ctx.random.next_int(CELEBRATE_JUMP_ROLL) == 0 {
            ctx.jump = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raid(id: i32, center: Pos, active: bool) -> RaidView {
        RaidView {
            id,
            center,
            active,
            over: false,
            loss: false,
            groups_spawned: 0,
        }
    }

    #[test]
    fn the_nearest_active_raid_within_96_blocks_is_the_one_here() {
        let raids = [
            raid(1, (0, 64, 0), true),
            raid(2, (50, 64, 0), true),
            raid(3, (10, 64, 0), false),
        ];
        assert_eq!(nearby_raid(&raids, (12, 64, 0)).map(|r| r.id), Some(1));
        assert_eq!(nearby_raid(&raids, (40, 64, 0)).map(|r| r.id), Some(2));
        assert_eq!(nearby_raid(&raids, (145, 64, 0)).map(|r| r.id), Some(2));
        assert_eq!(nearby_raid(&raids, (146, 64, 0)), None);
        assert_eq!(nearby_raid(&raids, (0, 64, 96)), None, "96 is not within");
    }

    #[test]
    fn shuffling_swaps_from_the_back_as_util_shuffle_does() {
        let mut random = minecraftoss_player::rng::LegacyRandom::new(5);
        let mut list = [1, 2, 3, 4, 5];
        shuffle(&mut list, &mut random);
        let mut again = minecraftoss_player::rng::LegacyRandom::new(5);
        let mut expected = [1, 2, 3, 4, 5];
        for i in (2..=5).rev() {
            let j = again.next_int(i as u32) as usize;
            expected.swap(i - 1, j);
        }
        assert_eq!(list, expected);
        let mut sorted = list;
        sorted.sort();
        assert_eq!(sorted, [1, 2, 3, 4, 5]);
    }
}
