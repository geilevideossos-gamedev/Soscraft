//! Cow goal adapters into the shared source-informed GoalSelector.
//! Float, Panic, Breed, Tempt, FollowParent, WaterAvoidingRandomStroll, LookAtPlayer and RandomLookAround registration and flags follow
//! AbstractCow.registerGoals; the other cow goals remain to be implemented.
use minecraftoss_player::World;
use crate::breed::BreedState;
use crate::fluid::FluidFrame;
use crate::follow_parent::{CowCandidate, FollowParentState};
use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::look::{LookAtPlayerState, RandomLookState};
use crate::panic::PanicState;
use crate::stroll::StrollState;
use crate::tempt::{PlayerCandidate, TemptAction, TemptState};
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const FLOAT_GOAL: usize = 0;
pub const PANIC_GOAL: usize = 1;
pub const BREED_GOAL: usize = 2;
pub const TEMPT_GOAL: usize = 3;
pub const FOLLOW_PARENT_GOAL: usize = 4;
pub const STROLL_GOAL: usize = 5;
pub const LOOK_AT_PLAYER_GOAL: usize = 6;
pub const RANDOM_LOOK_GOAL: usize = 7;
pub const GOAL_NAMES: [&str; 8] = [
    "FloatGoal",
    "PanicGoal",
    "BreedGoal",
    "TemptGoal",
    "FollowParentGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
];

/// What sets one farm animal's goals apart: its goals' speeds, its eye
/// heights and the items that tempt it.
#[derive(Clone, Copy, Debug)]
pub struct Species {
    pub panic_speed: f64,
    pub stroll_speed: f64,
    pub tempt_speed: f64,
    pub follow_parent_speed: f64,
    pub breed_speed: f64,
    pub adult_eye: f32,
    pub baby_eye: f32,
    pub tempt: TemptItems,
}

/// Which items tempt it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemptItems {
    /// `#cow_food`.
    CowFood,
    /// `#horse_tempt_items`.
    HorseTempt,
    LlamaTempt,
}

impl Species {
    /// `AbstractCow.registerGoals`.
    pub const COW: Species = Species { panic_speed: 2.0, stroll_speed: 1.0, tempt_speed: 1.25, follow_parent_speed: 1.25, breed_speed: 1.0, adult_eye: 1.3, baby_eye: 0.665, tempt: TemptItems::CowFood };

    /// `AbstractHorse.registerGoals` and `addBehaviourGoals`.
    pub fn horse(kind: crate::horse::HorseKind) -> Species {
        let tempt = if kind.is_llama() { TemptItems::LlamaTempt } else { TemptItems::HorseTempt };
        Species { panic_speed: 1.2, stroll_speed: 0.7, tempt_speed: 1.25, follow_parent_speed: 1.0, breed_speed: 1.0, adult_eye: kind.eye_height(false), baby_eye: kind.eye_height(true), tempt }
    }

    fn eye(&self, age: i32) -> f32 {
        if age < 0 {
            self.baby_eye
        } else {
            self.adult_eye
        }
    }
}

/// `RandomStandGoal`'s interval and whether the horse is immobile.
#[derive(Clone, Copy, Debug, Default)]
pub struct StandState {
    pub next_stand: i32,
    pub immobile: bool,
}

pub struct CowGoalContext {
    pub species: Species,
    /// A horse's rearing (`RandomStandGoal`).
    pub stand: Option<StandState>,
    pub child: CowCandidate,
    pub candidates: Vec<CowCandidate>,
    pub players: Vec<PlayerCandidate>,
    pub breed: BreedState,
    pub tempt: TemptState,
    pub follow_parent: FollowParentState,
    pub random_look: RandomLookState,
    pub look_at_player: LookAtPlayerState,
    pub stroll: StrollState,
    pub panic: PanicState,
    pub last_damage_panic: bool,
    /// `isOnFire`: panicking, it runs for water first.
    pub on_fire: bool,
    pub fluid: FluidFrame,
    /// The mob's walk search settings (stroll and panic positions).
    pub walk: crate::walk_path::WalkProfile,
    pub no_action_time: i32,
    pub navigation_done: bool,
    pub random: LegacyRandom,
    pub effects: Vec<CowGoalEffect>,
    pub mates: Option<Vec<CowCandidate>>,
    pub tempters: Vec<u64>,
    pub llama: Option<crate::llama_ai::LlamaContext>,
    pub crazy: Option<Crazy>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Crazy {
    pub rider: Option<u64>,
    pub mob_controlled: bool,
    pub temper: i32,
    pub max_temper: i32,
    pub target: Option<DVec3>,
}

pub enum CowGoalEffect {
    Navigate {
        target: DVec3,
        speed: f64,
    },
    /// `PathNavigation.moveTo(entity, speed)`: toward a mob or player,
    /// keeping the current path when none can be made (in the air).
    NavigateToEntity {
        target: DVec3,
        speed: f64,
    },
    StopNavigation,
    LookAt {
        target: DVec3,
        y_max: f32,
        x_max: f32,
    },
    Breed {
        partner_id: u64,
    },
    Jump,
    /// `RandomStandGoal.start`: rear, with the ambient stand sound.
    Stand,
    Spit {
        aim: DVec3,
    },
    JoinCaravan(u64),
    LeaveCaravan(u64),
    Tame(u64),
    Temper(i32),
    Buck,
}

pub fn registered_goals() -> GoalSelector<CowGoalContext> {
    let mut selector = GoalSelector::default();
    assert_eq!(selector.add(0, FloatGoal), FLOAT_GOAL);
    assert_eq!(selector.add(1, PanicGoal), PANIC_GOAL);
    assert_eq!(selector.add(2, BreedGoal), BREED_GOAL);
    assert_eq!(selector.add(3, TemptGoal), TEMPT_GOAL);
    assert_eq!(selector.add(4, FollowParentGoal), FOLLOW_PARENT_GOAL);
    assert_eq!(selector.add(5, StrollGoal), STROLL_GOAL);
    assert_eq!(selector.add(6, LookAtPlayerGoal), LOOK_AT_PLAYER_GOAL);
    assert_eq!(selector.add(7, RandomLookGoal), RANDOM_LOOK_GOAL);
    selector
}

#[derive(Clone)]
struct FloatGoal;
impl Goal<CowGoalContext> for FloatGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Jump])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        ctx.fluid.floatable_height() > 0.4 || ctx.fluid.in_lava()
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, ctx: &mut CowGoalContext) {
        if ctx.random.next_float() < 0.8 {
            ctx.effects.push(CowGoalEffect::Jump);
        }
    }
}

#[derive(Clone)]
struct PanicGoal;
impl Goal<CowGoalContext> for PanicGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        ctx.panic.can_use(
            ctx.last_damage_panic,
            ctx.on_fire,
            ctx.child.position,
            world,
            &ctx.walk,
            &mut ctx.random,
        )
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation_done
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        ctx.panic.running = true;
        if let Some(target) = ctx.panic.wanted {
            ctx.effects
                .push(CowGoalEffect::Navigate { target, speed: ctx.species.panic_speed });
        }
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        ctx.panic.running = false;
    }
}

#[derive(Clone)]
struct StrollGoal;
impl Goal<CowGoalContext> for StrollGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        ctx.stroll.can_use(
            ctx.child.position,
            ctx.no_action_time,
            ctx.fluid.in_water(),
            world,
            &ctx.walk,
            &mut ctx.random,
        )
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation_done
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        if let Some(target) = ctx.stroll.wanted {
            ctx.effects
                .push(CowGoalEffect::Navigate { target, speed: ctx.species.stroll_speed });
        }
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        ctx.effects.push(CowGoalEffect::StopNavigation);
    }
}

#[derive(Clone)]
struct LookAtPlayerGoal;
impl Goal<CowGoalContext> for LookAtPlayerGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Look])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        let eye_height: f32 = ctx.species.eye(ctx.child.age);
        ctx.look_at_player
            .can_use(world, ctx.child.position, eye_height, &ctx.players, &mut ctx.random)
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        ctx.look_at_player
            .can_continue(ctx.child.position, &ctx.players)
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        ctx.look_at_player.start(&mut ctx.random);
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        ctx.look_at_player.stop();
    }
    fn tick(&mut self, ctx: &mut CowGoalContext) {
        if let Some(target) = ctx.look_at_player.tick(&ctx.players) {
            ctx.effects.push(CowGoalEffect::LookAt {
                target,
                y_max: 10.0,
                x_max: 40.0,
            });
        }
    }
}

#[derive(Clone)]
struct RandomLookGoal;
impl Goal<CowGoalContext> for RandomLookGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        RandomLookState::can_use(&mut ctx.random)
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        ctx.random_look.can_continue()
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        ctx.random_look.start(&mut ctx.random);
    }
    fn tick(&mut self, ctx: &mut CowGoalContext) {
        let eye_height = ctx.species.eye(ctx.child.age);
        ctx.effects.push(CowGoalEffect::LookAt {
            target: ctx.random_look.tick(ctx.child.position, eye_height),
            y_max: 10.0,
            x_max: 40.0,
        });
    }
}

#[derive(Clone)]
struct TemptGoal;
impl Goal<CowGoalContext> for TemptGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        tempt_can_use(ctx)
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        // Cows and horses pass canScare=false, so TemptGoal calls canUse again.
        tempt_can_use(ctx)
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        ctx.tempt.start();
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        ctx.tempt.stop();
        ctx.effects.push(CowGoalEffect::StopNavigation);
    }
    fn tick(&mut self, ctx: &mut CowGoalContext) {
        if let Some(player) = ctx
            .players
            .iter()
            .find(|p| Some(p.id) == ctx.tempt.player_id)
        {
            ctx.effects.push(CowGoalEffect::LookAt {
                target: player.position + DVec3::new(0.0, f64::from(player.eye_height), 0.0),
                y_max: 95.0,
                x_max: 40.0,
            });
        }
        match ctx.tempt.tick(ctx.child, &ctx.players) {
            Some(TemptAction::Navigate(target)) => ctx.effects.push(CowGoalEffect::NavigateToEntity {
                target,
                speed: ctx.species.tempt_speed,
            }),
            Some(TemptAction::StopNavigation) => ctx.effects.push(CowGoalEffect::StopNavigation),
            None => {}
        }
    }
}

#[derive(Clone)]
struct BreedGoal;
impl Goal<CowGoalContext> for BreedGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        let mates = ctx.mates.as_deref().unwrap_or(&ctx.candidates);
        ctx.breed.can_use(ctx.child, mates)
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        let mates = ctx.mates.as_deref().unwrap_or(&ctx.candidates);
        ctx.breed.can_continue(mates)
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        ctx.breed.start();
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        ctx.breed.stop();
    }
    fn tick(&mut self, ctx: &mut CowGoalContext) {
        let mates = ctx.mates.as_deref().unwrap_or(&ctx.candidates);
        if let Some((target, partner)) = ctx.breed.tick(ctx.child, mates) {
            if let Some(candidate) = mates
                .iter()
                .find(|p| Some(p.id) == ctx.breed.partner_id)
            {
                let eye_height: f32 = ctx.species.eye(candidate.age);
                ctx.effects.push(CowGoalEffect::LookAt {
                    target: candidate.position + DVec3::new(0.0, f64::from(eye_height), 0.0),
                    y_max: 10.0,
                    x_max: 40.0,
                });
            }
            ctx.effects
                .push(CowGoalEffect::NavigateToEntity { target, speed: ctx.species.breed_speed });
            if let Some(partner_id) = partner {
                ctx.effects.push(CowGoalEffect::Breed { partner_id });
            }
        }
    }
}

#[derive(Clone)]
struct FollowParentGoal;
impl Goal<CowGoalContext> for FollowParentGoal {
    fn controls(&self) -> Controls {
        // The pinned FollowParentGoal constructor does not set any flags.
        Controls::new(&[])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        ctx.follow_parent.can_use(ctx.child, &ctx.candidates)
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        ctx.follow_parent.can_continue(ctx.child, &ctx.candidates)
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        ctx.follow_parent.start();
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        ctx.follow_parent.stop();
    }
    fn tick(&mut self, ctx: &mut CowGoalContext) {
        if let Some(target) = ctx.follow_parent.tick(&ctx.candidates) {
            ctx.effects.push(CowGoalEffect::NavigateToEntity {
                target,
                speed: ctx.species.follow_parent_speed,
            });
        }
    }
}

/// `TemptGoal.canUse` with its species' items.
fn tempt_can_use(ctx: &mut CowGoalContext) -> bool {
    match ctx.species.tempt {
        TemptItems::CowFood => ctx.tempt.can_use(ctx.child, &ctx.players),
        TemptItems::HorseTempt => {
            let tempters = std::mem::take(&mut ctx.tempters);
            let found = ctx.tempt.can_use_matching(ctx.child, &ctx.players, |p| p.main_hand_horse_tempt || p.offhand_horse_tempt || tempters.contains(&p.id));
            ctx.tempters = tempters;
            found
        }
        TemptItems::LlamaTempt => {
            let tempters = std::mem::take(&mut ctx.tempters);
            let found = ctx.tempt.can_use_matching(ctx.child, &ctx.players, |p| tempters.contains(&p.id));
            ctx.tempters = tempters;
            found
        }
    }
}

/// `AbstractHorse`'s goals, in the order its constructor adds them
/// (`registerGoals`, then `addBehaviourGoals`).
pub const HORSE_GOAL_NAMES: [&str; 10] = [
    "RunAroundLikeCrazyGoal",
    "BreedGoal",
    "FollowParentGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
    "RandomStandGoal",
    "FloatGoal",
    "MountPanicGoal",
    "TemptGoal",
];

pub const LLAMA_GOAL_NAMES: [&str; 13] = [
    "LlamaHurtByTargetGoal",
    "LlamaAttackWolfGoal",
    "FloatGoal",
    "RunAroundLikeCrazyGoal",
    "LlamaFollowCaravanGoal",
    "RangedAttackGoal",
    "PanicGoal",
    "BreedGoal",
    "TemptGoal",
    "FollowParentGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
];

pub const TRADER_LLAMA_GOAL_NAMES: [&str; 17] = [
    "LlamaHurtByTargetGoal",
    "LlamaAttackWolfGoal",
    "TraderLlamaDefendWanderingTraderGoal",
    "NearestAttackableTargetGoal",
    "NearestAttackableTargetGoal",
    "FloatGoal",
    "RunAroundLikeCrazyGoal",
    "LlamaFollowCaravanGoal",
    "RangedAttackGoal",
    "PanicGoal",
    "BreedGoal",
    "TemptGoal",
    "FollowParentGoal",
    "WaterAvoidingRandomStrollGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
    "PanicGoal",
];

pub fn registered_llama_goals(trader: bool) -> GoalSelector<CowGoalContext> {
    use crate::llama_ai::{CaravanGoal, HurtByGoal, NearestFoeGoal, NoTargetGoal, SpitGoal};
    let mut selector = GoalSelector::default();
    selector.add(1, HurtByGoal);
    selector.add(2, NearestFoeGoal::wolves());
    if trader {
        selector.add(1, NoTargetGoal);
        selector.add(2, NearestFoeGoal::zombies());
        selector.add(2, NoTargetGoal);
    }
    selector.add(0, FloatGoal);
    selector.add(1, RunAroundLikeCrazyGoal);
    selector.add(2, CaravanGoal);
    selector.add(3, SpitGoal { target: None });
    selector.add(3, PanicGoal);
    selector.add(4, BreedGoal);
    selector.add(5, TemptGoal);
    selector.add(6, FollowParentGoal);
    selector.add(7, StrollGoal);
    selector.add(8, LookAtPlayerGoal);
    selector.add(9, RandomLookGoal);
    if trader {
        selector.add(1, SpeedPanicGoal(2.0));
    }
    selector
}

#[derive(Clone)]
struct SpeedPanicGoal(f64);
impl Goal<CowGoalContext> for SpeedPanicGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        PanicGoal.can_start(ctx, world)
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation_done
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        ctx.panic.running = true;
        if let Some(target) = ctx.panic.wanted {
            ctx.effects.push(CowGoalEffect::Navigate { target, speed: self.0 });
        }
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        ctx.panic.running = false;
    }
}

pub fn registered_horse_goals() -> GoalSelector<CowGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, RunAroundLikeCrazyGoal);
    selector.add(2, BreedGoal);
    selector.add(4, FollowParentGoal);
    selector.add(6, StrollGoal);
    selector.add(7, LookAtPlayerGoal);
    selector.add(8, RandomLookGoal);
    selector.add(9, RandomStandGoal);
    selector.add(0, FloatGoal);
    selector.add(1, PanicGoal);
    selector.add(3, TemptGoal);
    selector
}

#[derive(Clone)]
struct RunAroundLikeCrazyGoal;
impl Goal<CowGoalContext> for RunAroundLikeCrazyGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        let Some(crazy) = ctx.crazy.as_mut() else { return false };
        if crazy.mob_controlled || crazy.rider.is_none() {
            return false;
        }
        crazy.target = crate::stroll::default_random_position(world, &ctx.walk, ctx.child.position, 5, 4, &mut ctx.random);
        crazy.target.is_some()
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        ctx.crazy.is_some_and(|c| c.rider.is_some()) && !ctx.navigation_done
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        if let Some(target) = ctx.crazy.and_then(|c| c.target) {
            ctx.effects.push(CowGoalEffect::Navigate { target, speed: 1.2 });
        }
    }
    fn tick(&mut self, ctx: &mut CowGoalContext) {
        let Some(crazy) = ctx.crazy else { return };
        if ctx.random.next_int(25) != 0 {
            return;
        }
        let Some(rider) = crazy.rider else { return };
        if crazy.max_temper > 0 && (ctx.random.next_int(crazy.max_temper as u32) as i32) < crazy.temper {
            ctx.effects.push(CowGoalEffect::Tame(rider));
            ctx.crazy = None;
            return;
        }
        ctx.effects.push(CowGoalEffect::Temper(5));
        ctx.effects.push(CowGoalEffect::Buck);
        ctx.crazy = None;
    }
}

/// `RandomStandGoal`: every tried tick its interval grows, and past zero
/// a roll against it resets it and, one time in ten, rears the horse
/// (unless it is grazing or already rearing).
#[derive(Clone)]
struct RandomStandGoal;
impl Goal<CowGoalContext> for RandomStandGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        let Some(stand) = ctx.stand.as_mut() else { return false };
        stand.next_stand += 1;
        if stand.next_stand > 0 && (ctx.random.next_int(1000) as i32) < stand.next_stand {
            stand.next_stand = -crate::horse::AMBIENT_SOUND_INTERVAL;
            !stand.immobile && ctx.random.next_int(10) == 0
        } else {
            false
        }
    }
    fn can_continue(&mut self, _ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        false
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        ctx.effects.push(CowGoalEffect::Stand);
    }
}
