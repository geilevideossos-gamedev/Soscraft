use crate::goals::{Control, Controls, Goal};
use crate::monster_ai::{MonsterGoalContext, is_enemy};
use crate::navigation::{navigate_walk_to, navigate_walk_to_entity};
use crate::panic::PanicState;
use crate::tempt::PlayerCandidate;
use crate::world::families::GoalExtra;
use crate::world::families::goals::reduced_tick_delay;
use glam::DVec3;
use minecraftoss_player::World;
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

#[cfg(test)]
mod tests {
    use super::*;

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
