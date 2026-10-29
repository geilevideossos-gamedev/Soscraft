use crate::feline_ai::find_nearest_block;
use crate::goals::{Control, Controls, Goal};
use crate::monster_ai::{MonsterGoalContext, is_enemy};
use crate::navigation::{navigate_walk_to, navigate_walk_to_entity};
use crate::panic::PanicState;
use crate::tempt::PlayerCandidate;
use crate::world::families::GoalExtra;
use crate::world::families::goals::reduced_tick_delay;
use glam::DVec3;
use minecraftoss_player::{Pos, World};
use std::any::Any;
use std::marker::PhantomData;

pub const TEMPT_RANGE: f64 = 10.0;
pub const SNEAKING_VISIBILITY: f64 = 0.8;
pub const TEMPT_STOP_DISTANCE: f64 = 2.5;
pub const TEMPT_LOOK_Y: f32 = 95.0;
pub const TEMPT_CALM_DOWN: u32 = 100;
pub const PARTNER_RANGE: f64 = 8.0;
pub const PARTNER_LOOK_Y: f32 = 10.0;
pub const LOVE_TIME: i32 = 60;
pub const BREED_DELAY: u32 = 60;
pub const BREED_DISTANCE: f64 = 3.0;
pub const LAST_DAMAGE_TICKS: i32 = 40;
pub const MOVE_TO_BLOCK_INTERVAL: u32 = 200;
pub const MOVE_TO_BLOCK_GIVE_UP_TICKS: i32 = 1200;
pub const MOVE_TO_BLOCK_STAY_TICKS: i32 = 1200;
pub const MOVE_TO_BLOCK_RECALCULATE: i32 = 40;
pub const MOVE_TO_BLOCK_ACCEPTED_DISTANCE: f64 = 1.0;
pub const NOT_MONSTERS: [&str; 7] = [
    "minecraft:ghast",
    "minecraft:slime",
    "minecraft:magma_cube",
    "minecraft:hoglin",
    "minecraft:shulker",
    "minecraft:phantom",
    "minecraft:ender_dragon",
];

pub fn is_monster_class(kind: &str) -> bool {
    is_enemy(kind) && !NOT_MONSTERS.contains(&kind)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tempter {
    pub id: u64,
    pub sneaking: bool,
    pub holds: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mate {
    pub id: u64,
    pub position: DVec3,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub alive: bool,
    pub in_love: bool,
    pub panicking: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AnimalGoals {
    pub last_damage: Option<&'static str>,
    pub panic: PanicState,
    pub tempters: Vec<Tempter>,
    pub tempting: Option<u64>,
    pub tempt_calm_down: i32,
    pub tempted: bool,
    pub in_love: bool,
    pub mates: Vec<Mate>,
    pub partner: Option<u64>,
    pub love_time: i32,
    pub breed_with: Option<u64>,
}

pub trait AnimalExtra: Any + Clone + Send {
    fn animal(&self) -> &AnimalGoals;
    fn animal_mut(&mut self) -> &mut AnimalGoals;
}

pub fn animal<E: AnimalExtra>(extra: &GoalExtra) -> Option<&AnimalGoals> {
    extra.get::<E>().map(E::animal)
}

pub fn animal_mut<E: AnimalExtra>(extra: &mut GoalExtra) -> Option<&mut AnimalGoals> {
    extra.get_mut::<E>().map(E::animal_mut)
}

pub struct PanicGoal<E> {
    pub speed: f64,
    pub causes: fn(&str) -> bool,
    extra: PhantomData<fn() -> E>,
}

impl<E> Clone for PanicGoal<E> {
    fn clone(&self) -> Self {
        Self {
            speed: self.speed,
            causes: self.causes,
            extra: PhantomData,
        }
    }
}

impl<E: AnimalExtra> PanicGoal<E> {
    pub fn new(speed: f64, causes: fn(&str) -> bool) -> Self {
        Self {
            speed,
            causes,
            extra: PhantomData,
        }
    }
}

impl<E: AnimalExtra> Goal<MonsterGoalContext> for PanicGoal<E> {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        let profile = ctx.profile();
        let (on_fire, position) = (ctx.on_fire, ctx.body.position);
        let Some(animal) = animal_mut::<E>(&mut ctx.extra) else {
            return false;
        };
        let panics = animal.last_damage.is_some_and(self.causes);
        animal
            .panic
            .can_use(panics, on_fire, position, world, &profile, &mut ctx.random)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation.is_done()
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let wanted = animal::<E>(&ctx.extra).and_then(|animal| animal.panic.wanted);
        if let Some(target) = wanted {
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
        if let Some(animal) = animal_mut::<E>(&mut ctx.extra) {
            animal.panic.running = true;
        }
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(animal) = animal_mut::<E>(&mut ctx.extra) {
            animal.panic.running = false;
        }
    }
}

pub struct TemptGoal<E> {
    pub speed: f64,
    extra: PhantomData<fn() -> E>,
}

impl<E> Clone for TemptGoal<E> {
    fn clone(&self) -> Self {
        Self {
            speed: self.speed,
            extra: PhantomData,
        }
    }
}

impl<E: AnimalExtra> TemptGoal<E> {
    pub fn new(speed: f64) -> Self {
        Self {
            speed,
            extra: PhantomData,
        }
    }

    fn player(ctx: &MonsterGoalContext) -> Option<PlayerCandidate> {
        let id = animal::<E>(&ctx.extra)?.tempting?;
        ctx.players.iter().copied().find(|p| p.id == id)
    }

    fn can_use(ctx: &mut MonsterGoalContext) -> bool {
        let position = ctx.body.position;
        let players = &ctx.players;
        let Some(animal) = animal_mut::<E>(&mut ctx.extra) else {
            return false;
        };
        if animal.tempt_calm_down > 0 {
            animal.tempt_calm_down -= 1;
            return false;
        }
        let mut best: Option<(f64, u64)> = None;
        for tempter in animal.tempters.iter().filter(|t| t.holds) {
            let Some(player) = players.iter().find(|p| p.id == tempter.id) else {
                continue;
            };
            if !player.alive || player.spectator {
                continue;
            }
            let visibility = if tempter.sneaking {
                SNEAKING_VISIBILITY
            } else {
                1.0
            };
            let range = (TEMPT_RANGE * visibility).max(2.0);
            let distance = position.distance_squared(player.position);
            if distance > range * range {
                continue;
            }
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, tempter.id));
            }
        }
        animal.tempting = best.map(|(_, id)| id);
        best.is_some()
    }
}

impl<E: AnimalExtra> Goal<MonsterGoalContext> for TemptGoal<E> {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        Self::can_use(ctx)
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(animal) = animal_mut::<E>(&mut ctx.extra) {
            animal.tempted = true;
        }
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.navigation.stop();
        if let Some(animal) = animal_mut::<E>(&mut ctx.extra) {
            animal.tempting = None;
            animal.tempt_calm_down = reduced_tick_delay(TEMPT_CALM_DOWN) as i32;
            animal.tempted = false;
        }
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(player) = Self::player(ctx) else {
            return;
        };
        let eyes = player.position + DVec3::Y * f64::from(player.eye_height);
        let x_max = ctx.max_head_x_rot();
        ctx.look_control
            .set_look_at_with_limits(eyes, TEMPT_LOOK_Y, x_max);
        let stop = TEMPT_STOP_DISTANCE * TEMPT_STOP_DISTANCE;
        if ctx.body.position.distance_squared(player.position) < stop {
            ctx.navigation.stop();
        } else {
            let profile = ctx.profile();
            let _ = navigate_walk_to_entity(
                &ctx.body,
                &mut ctx.navigation,
                world,
                &profile,
                ctx.fluid,
                player.position,
                self.speed,
                1,
            );
        }
    }
}

pub struct BreedGoal<E> {
    pub speed: f64,
    extra: PhantomData<fn() -> E>,
}

impl<E> Clone for BreedGoal<E> {
    fn clone(&self) -> Self {
        Self {
            speed: self.speed,
            extra: PhantomData,
        }
    }
}

impl<E: AnimalExtra> BreedGoal<E> {
    pub fn new(speed: f64) -> Self {
        Self {
            speed,
            extra: PhantomData,
        }
    }

    fn partner(ctx: &MonsterGoalContext) -> Option<Mate> {
        let animal = animal::<E>(&ctx.extra)?;
        let id = animal.partner?;
        animal.mates.iter().copied().find(|m| m.id == id)
    }
}

impl<E: AnimalExtra> Goal<MonsterGoalContext> for BreedGoal<E> {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let body = &ctx.body;
        let half = f64::from(body.width / 2.0);
        let p = body.position;
        let reach = PARTNER_RANGE;
        let min = DVec3::new(p.x - half - reach, p.y - reach, p.z - half - reach);
        let max = DVec3::new(
            p.x + half + reach,
            p.y + f64::from(body.height) + reach,
            p.z + half + reach,
        );
        let Some(animal) = animal_mut::<E>(&mut ctx.extra) else {
            return false;
        };
        if !animal.in_love {
            return false;
        }
        let mut best: Option<(f64, u64)> = None;
        for m in &animal.mates {
            let mh = f64::from(m.width / 2.0);
            let q = m.position;
            let inside = q.x - mh < max.x
                && q.x + mh > min.x
                && q.y < max.y
                && q.y + f64::from(m.height) > min.y
                && q.z - mh < max.z
                && q.z + mh > min.z;
            let distance = p.distance_squared(q);
            if !inside || !m.alive || distance > reach * reach {
                continue;
            }
            if m.in_love && !m.panicking && best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, m.id));
            }
        }
        animal.partner = best.map(|(_, id)| id);
        best.is_some()
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        let love_time = animal::<E>(&ctx.extra).map_or(LOVE_TIME, |a| a.love_time);
        Self::partner(ctx).is_some_and(|m| m.alive && m.in_love && !m.panicking)
            && love_time < LOVE_TIME
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(animal) = animal_mut::<E>(&mut ctx.extra) {
            animal.partner = None;
            animal.love_time = 0;
        }
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(partner) = Self::partner(ctx) else {
            return;
        };
        let x_max = ctx.max_head_x_rot();
        let eyes = partner.position + DVec3::Y * f64::from(partner.eye_height);
        ctx.look_control
            .set_look_at_with_limits(eyes, PARTNER_LOOK_Y, x_max);
        let profile = ctx.profile();
        let _ = navigate_walk_to_entity(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            partner.position,
            self.speed,
            1,
        );
        let close =
            ctx.body.position.distance_squared(partner.position) < BREED_DISTANCE * BREED_DISTANCE;
        if let Some(animal) = animal_mut::<E>(&mut ctx.extra) {
            animal.love_time += 1;
            if animal.love_time >= reduced_tick_delay(BREED_DELAY) as i32 && close {
                animal.breed_with = Some(partner.id);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveToBlockGoal {
    pub speed: f64,
    pub search_range: i32,
    pub vertical_search_range: i32,
    pub vertical_search_start: i32,
    pub next_start_tick: i32,
    pub try_ticks: i32,
    pub max_stay_ticks: i32,
    pub block: Pos,
    pub reached: bool,
}

impl MoveToBlockGoal {
    pub fn new(speed: f64, search_range: i32, vertical_search_range: i32) -> Self {
        Self {
            speed,
            search_range,
            vertical_search_range,
            vertical_search_start: 0,
            next_start_tick: 0,
            try_ticks: 0,
            max_stay_ticks: 0,
            block: (0, 0, 0),
            reached: false,
        }
    }

    pub fn can_use(&mut self, ctx: &mut MonsterGoalContext, valid: impl Fn(Pos) -> bool) -> bool {
        if self.next_start_tick > 0 {
            self.next_start_tick -= 1;
            return false;
        }
        let wait = MOVE_TO_BLOCK_INTERVAL + ctx.random.next_int(MOVE_TO_BLOCK_INTERVAL);
        self.next_start_tick = reduced_tick_delay(wait) as i32;
        self.find_nearest_block(ctx.body.position, valid)
    }

    pub fn find_nearest_block(&mut self, position: DVec3, valid: impl Fn(Pos) -> bool) -> bool {
        let origin = (
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        );
        let found = find_nearest_block(
            origin,
            self.search_range,
            self.vertical_search_range,
            self.vertical_search_start,
            valid,
        );
        if let Some(block) = found {
            self.block = block;
        }
        found.is_some()
    }

    pub fn can_continue(&self, valid: bool) -> bool {
        self.try_ticks >= -self.max_stay_ticks
            && self.try_ticks <= MOVE_TO_BLOCK_GIVE_UP_TICKS
            && valid
    }

    pub fn start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.walk_to(ctx, world, self.move_to_target());
        self.try_ticks = 0;
        let stay = MOVE_TO_BLOCK_STAY_TICKS as u32;
        let bound = ctx.random.next_int(stay) + stay;
        self.max_stay_ticks = ctx.random.next_int(bound) as i32 + MOVE_TO_BLOCK_STAY_TICKS;
    }

    pub fn move_to_target(&self) -> Pos {
        (self.block.0, self.block.1 + 1, self.block.2)
    }

    pub fn tick(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let target = self.move_to_target();
        let centre = DVec3::new(
            f64::from(target.0) + 0.5,
            f64::from(target.1) + 0.5,
            f64::from(target.2) + 0.5,
        );
        let reach = MOVE_TO_BLOCK_ACCEPTED_DISTANCE;
        if centre.distance_squared(ctx.body.position) < reach * reach {
            self.reached = true;
            self.try_ticks -= 1;
        } else {
            self.reached = false;
            self.try_ticks += 1;
            if self.try_ticks % MOVE_TO_BLOCK_RECALCULATE == 0 {
                self.walk_to(ctx, world, target);
            }
        }
    }

    fn walk_to(&self, ctx: &mut MonsterGoalContext, world: &dyn World, target: Pos) {
        let point = DVec3::new(
            f64::from(target.0) + 0.5,
            f64::from(target.1),
            f64::from(target.2) + 0.5,
        );
        let profile = ctx.profile();
        let _ = navigate_walk_to(
            &ctx.body,
            &mut ctx.navigation,
            world,
            &profile,
            ctx.fluid,
            point,
            self.speed,
            1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monster_ai::{MonsterAi, MonsterKind};
    use crate::movement::Body;
    use minecraftoss_player::Block;

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:grass_block"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn context(position: DVec3) -> MonsterGoalContext {
        let mut body = Body::new(position, 0.6, 1.95);
        body.on_ground = true;
        MonsterAi::of_kind(MonsterKind::Zombie, &body, 0.0).state
    }

    fn nearest(valid: &[Pos]) -> Option<Pos> {
        let mut goal = MoveToBlockGoal::new(1.0, 16, 1);
        let found = goal.find_nearest_block(DVec3::new(0.5, 64.0, 0.5), |pos| valid.contains(&pos));
        found.then_some(goal.block)
    }

    #[test]
    fn move_to_block_searches_in_vanilla_order_and_stops_within_reach() {
        assert_eq!(nearest(&[(0, 63, 0), (0, 64, 0)]), Some((0, 63, 0)));
        assert_eq!(nearest(&[(1, 63, 0), (0, 63, 1)]), Some((0, 63, 1)));
        assert_eq!(nearest(&[(0, 63, -1), (0, 63, 1)]), Some((0, 63, 1)));
        assert_eq!(nearest(&[(-1, 63, 0), (1, 63, 1)]), Some((1, 63, 1)));
        assert_eq!(nearest(&[(2, 63, 0), (1, 63, -1)]), Some((1, 63, -1)));
        assert_eq!(nearest(&[(0, 64, 0), (15, 63, 0)]), Some((15, 63, 0)));
        assert_eq!(nearest(&[(0, 62, 0), (0, 64, 3)]), Some((0, 64, 3)));
        assert_eq!(nearest(&[(0, 62, 0)]), Some((0, 62, 0)));
        assert_eq!(nearest(&[(16, 63, 0), (0, 65, 0), (0, 61, 0)]), None);

        let mut starts = Vec::new();
        for seed in 0..200 {
            let mut ctx = context(DVec3::new(0.5, 64.0, 0.5));
            ctx.random = minecraftoss_player::rng::LegacyRandom::new(seed);
            let mut goal = MoveToBlockGoal::new(1.0, 16, 1);
            assert!(goal.can_use(&mut ctx, |pos| pos == (4, 63, 0)));
            assert_eq!(goal.block, (4, 63, 0));
            assert!((100..=200).contains(&goal.next_start_tick));
            starts.push(goal.next_start_tick);
            let waiting = goal.next_start_tick;
            assert!(!goal.can_use(&mut ctx, |_| true), "it waits");
            assert_eq!(goal.next_start_tick, waiting - 1);
        }
        assert!(starts.iter().any(|&tick| tick < 110) && starts.iter().any(|&tick| tick > 190));

        let field = Field;
        let mut ctx = context(DVec3::new(0.5, 64.0, 0.5));
        let mut goal = MoveToBlockGoal::new(1.0, 16, 1);
        goal.block = (6, 63, 0);
        goal.try_ticks = 7;
        goal.start(&mut ctx, &field);
        assert_eq!(goal.try_ticks, 0);
        assert!(
            (1200..3600).contains(&goal.max_stay_ticks),
            "{}",
            goal.max_stay_ticks
        );
        assert!(!ctx.navigation.is_done(), "it walks to the block");
        for tick in 1..=39 {
            goal.tick(&mut ctx, &field);
            assert_eq!(goal.try_ticks, tick);
            assert!(!goal.reached);
        }
        ctx.navigation.stop();
        goal.tick(&mut ctx, &field);
        assert_eq!(goal.try_ticks, 40);
        assert!(!ctx.navigation.is_done(), "every 40 ticks it walks again");
        assert!(goal.can_continue(true) && !goal.can_continue(false));
        ctx.body.position = DVec3::new(5.8, 64.0, 0.5);
        goal.tick(&mut ctx, &field);
        assert!(goal.reached, "within a block of the centre above it");
        assert_eq!(goal.try_ticks, 39);
        ctx.body.position = DVec3::new(5.6, 64.0, 0.5);
        goal.tick(&mut ctx, &field);
        assert!(!goal.reached);
        assert_eq!(goal.try_ticks, 40);
        goal.try_ticks = 1200;
        assert!(goal.can_continue(true));
        goal.try_ticks = 1201;
        assert!(!goal.can_continue(true), "it gives up");
        goal.try_ticks = -goal.max_stay_ticks;
        assert!(goal.can_continue(true));
        goal.try_ticks -= 1;
        assert!(!goal.can_continue(true), "it stayed long enough");
    }

    #[test]
    fn a_monster_is_an_enemy_but_not_a_slime_ghast_hoglin_or_shulker() {
        for kind in [
            "minecraft:zombie",
            "minecraft:creeper",
            "minecraft:spider",
            "minecraft:blaze",
            "minecraft:zoglin",
            "minecraft:vex",
            "minecraft:ravager",
            "minecraft:silverfish",
            "minecraft:endermite",
        ] {
            assert!(is_monster_class(kind), "{kind}");
        }
        for kind in [
            "minecraft:slime",
            "minecraft:magma_cube",
            "minecraft:ghast",
            "minecraft:hoglin",
            "minecraft:shulker",
            "minecraft:phantom",
            "minecraft:ender_dragon",
            "minecraft:iron_golem",
            "minecraft:wolf",
            "minecraft:player",
        ] {
            assert!(!is_monster_class(kind), "{kind}");
        }
    }
}
