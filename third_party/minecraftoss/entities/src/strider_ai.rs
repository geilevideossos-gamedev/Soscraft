use crate::{
    breed::BreedState,
    fluid::FluidFrame,
    follow_parent::{CowCandidate, FollowParentState},
    goals::{Control, Controls, Goal, GoalSelector},
    look::RandomLookState,
    panic::PanicState,
    tempt::{PlayerCandidate, TemptAction, TemptState},
};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

pub const GOAL_NAMES: [&str; 9] = [
    "PanicGoal",
    "BreedGoal",
    "TemptGoal",
    "StriderGoToLavaGoal",
    "FollowParentGoal",
    "RandomStrollGoal",
    "LookAtPlayerGoal",
    "RandomLookAroundGoal",
    "LookAtPlayerGoal",
];
pub const PANIC_GOAL: usize = 0;
pub const TEMPT_GOAL: usize = 2;
pub const PANIC_SPEED: f64 = 1.65;
pub const TEMPT_SPEED: f64 = 1.4;
pub const LOOK_RANGE: f64 = 8.0;

#[derive(Clone, Debug, Default)]
pub struct GoToLavaState {
    pub next_start_tick: i32,
    pub try_ticks: i32,
    pub max_stay_ticks: i32,
    pub target: Option<(i32, i32, i32)>,
}

#[derive(Clone, Debug, Default)]
pub struct LookAtState {
    pub target: Option<(u64, DVec3)>,
    pub look_time: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct LookTarget {
    pub id: u64,
    pub eye: DVec3,
}

pub struct StriderGoalContext {
    pub child: CowCandidate,
    pub candidates: Vec<CowCandidate>,
    pub follow_parent: FollowParentState,
    pub breed: BreedState,
    pub panic: PanicState,
    pub stroll: crate::stroll::StrollState,
    pub tempt: TemptState,
    pub go_to_lava: GoToLavaState,
    pub random_look: RandomLookState,
    pub look_at_player: LookAtState,
    pub look_at_strider: LookAtState,
    pub last_damage_panic: bool,
    pub no_action_time: i32,
    pub navigation_done: bool,
    pub in_lava: bool,
    pub eye_height: f32,
    pub players: Vec<PlayerCandidate>,
    pub tempting: Vec<u64>,
    pub look_players: Vec<LookTarget>,
    pub look_striders: Vec<LookTarget>,
    pub random: LegacyRandom,
    pub fluid: FluidFrame,
    pub walk: crate::walk_path::WalkProfile,
    pub effects: Vec<StriderGoalEffect>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StriderGoalEffect {
    Navigate {
        target: DVec3,
        speed: f64,
    },
    NavigateToEntity {
        target: DVec3,
        speed: f64,
    },
    LookAt {
        target: DVec3,
        y_max: f32,
        x_max: f32,
    },
    StopNavigation,
    Breed {
        partner_id: u64,
    },
}

pub fn registered_strider_goals() -> GoalSelector<StriderGoalContext> {
    let mut goals = GoalSelector::default();
    goals.add(1, PanicGoal);
    goals.add(2, BreedGoal);
    goals.add(3, TemptGoal);
    goals.add(4, GoToLavaGoal);
    goals.add(5, FollowParentGoal);
    goals.add(7, StrollGoal);
    goals.add(8, LookAtGoal { striders: false });
    goals.add(8, RandomLookGoal);
    goals.add(9, LookAtGoal { striders: true });
    goals
}

#[derive(Clone)]
struct PanicGoal;
impl Goal<StriderGoalContext> for PanicGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut StriderGoalContext, world: &dyn World) -> bool {
        ctx.panic.can_use(
            ctx.last_damage_panic,
            false,
            ctx.child.position,
            world,
            &ctx.walk,
            &mut ctx.random,
        )
    }
    fn can_continue(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation_done
    }
    fn start(&mut self, ctx: &mut StriderGoalContext) {
        ctx.panic.running = true;
        if let Some(target) = ctx.panic.wanted {
            ctx.effects.push(StriderGoalEffect::Navigate {
                target,
                speed: PANIC_SPEED,
            });
        }
    }
    fn stop(&mut self, ctx: &mut StriderGoalContext) {
        ctx.panic.running = false;
    }
}

#[derive(Clone)]
struct BreedGoal;
impl Goal<StriderGoalContext> for BreedGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        ctx.breed.can_use(ctx.child, &ctx.candidates)
    }
    fn can_continue(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        ctx.breed.can_continue(&ctx.candidates)
    }
    fn start(&mut self, ctx: &mut StriderGoalContext) {
        ctx.breed.start();
    }
    fn stop(&mut self, ctx: &mut StriderGoalContext) {
        ctx.breed.stop();
    }
    fn tick(&mut self, ctx: &mut StriderGoalContext) {
        if let Some((target, partner)) = ctx.breed.tick(ctx.child, &ctx.candidates) {
            if let Some(candidate) = ctx
                .candidates
                .iter()
                .find(|candidate| Some(candidate.id) == ctx.breed.partner_id)
            {
                let eye = if candidate.age < 0 {
                    crate::strider::BABY_EYE_HEIGHT
                } else {
                    crate::strider::EYE_HEIGHT
                };
                ctx.effects.push(StriderGoalEffect::LookAt {
                    target: candidate.position + DVec3::new(0.0, f64::from(eye), 0.0),
                    y_max: 10.0,
                    x_max: 40.0,
                });
            }
            ctx.effects
                .push(StriderGoalEffect::NavigateToEntity { target, speed: 1.0 });
            if let Some(partner_id) = partner {
                ctx.effects.push(StriderGoalEffect::Breed { partner_id });
            }
        }
    }
}

#[derive(Clone)]
struct TemptGoal;
impl TemptGoal {
    fn can_use(ctx: &mut StriderGoalContext) -> bool {
        let tempting = &ctx.tempting;
        ctx.tempt
            .can_use_matching(ctx.child, &ctx.players, |player| {
                tempting.contains(&player.id)
            })
    }
}
impl Goal<StriderGoalContext> for TemptGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        Self::can_use(ctx)
    }
    fn can_continue(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        Self::can_use(ctx)
    }
    fn start(&mut self, ctx: &mut StriderGoalContext) {
        ctx.tempt.start();
    }
    fn stop(&mut self, ctx: &mut StriderGoalContext) {
        ctx.tempt.stop();
        ctx.effects.push(StriderGoalEffect::StopNavigation);
    }
    fn tick(&mut self, ctx: &mut StriderGoalContext) {
        if let Some(player) = ctx
            .players
            .iter()
            .find(|player| Some(player.id) == ctx.tempt.player_id)
        {
            ctx.effects.push(StriderGoalEffect::LookAt {
                target: player.position + DVec3::new(0.0, f64::from(player.eye_height), 0.0),
                y_max: 95.0,
                x_max: 40.0,
            });
        }
        match ctx.tempt.tick(ctx.child, &ctx.players) {
            Some(TemptAction::Navigate(target)) => {
                ctx.effects.push(StriderGoalEffect::NavigateToEntity {
                    target,
                    speed: TEMPT_SPEED,
                })
            }
            Some(TemptAction::StopNavigation) => {
                ctx.effects.push(StriderGoalEffect::StopNavigation)
            }
            None => {}
        }
    }
}

pub fn lava_target(world: &dyn World, pos: (i32, i32, i32)) -> bool {
    crate::strider::is_lava(world, pos) && world.pathfindable((pos.0, pos.1 + 1, pos.2))
}

pub fn find_lava(world: &dyn World, position: DVec3) -> Option<(i32, i32, i32)> {
    let (mx, my, mz) = (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    let mut y = 0;
    while y <= 2 {
        for r in 0..8 {
            let mut x = 0;
            while x <= r {
                let mut z = if x < r && x > -r { r } else { 0 };
                while z <= r {
                    let pos = (mx + x, my + y - 1, mz + z);
                    if lava_target(world, pos) {
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

fn block_center(pos: (i32, i32, i32)) -> DVec3 {
    DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    )
}

#[derive(Clone)]
struct GoToLavaGoal;
impl Goal<StriderGoalContext> for GoToLavaGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Jump])
    }
    fn can_start(&mut self, ctx: &mut StriderGoalContext, world: &dyn World) -> bool {
        if ctx.in_lava {
            return false;
        }
        let state = &mut ctx.go_to_lava;
        if state.next_start_tick > 0 {
            state.next_start_tick -= 1;
            return false;
        }
        state.next_start_tick = (200 + ctx.random.next_int(200) as i32 + 1) / 2;
        state.target = find_lava(world, ctx.child.position);
        state.target.is_some()
    }
    fn can_continue(&mut self, ctx: &mut StriderGoalContext, world: &dyn World) -> bool {
        !ctx.in_lava
            && ctx
                .go_to_lava
                .target
                .is_some_and(|pos| lava_target(world, pos))
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, ctx: &mut StriderGoalContext) {
        let Some(pos) = ctx.go_to_lava.target else {
            return;
        };
        ctx.effects.push(StriderGoalEffect::Navigate {
            target: DVec3::new(
                f64::from(pos.0) + 0.5,
                f64::from(pos.1 + 1),
                f64::from(pos.2) + 0.5,
            ),
            speed: 1.0,
        });
        ctx.go_to_lava.try_ticks = 0;
        let bound = ctx.random.next_int(1200) + 1200;
        ctx.go_to_lava.max_stay_ticks = ctx.random.next_int(bound) as i32 + 1200;
    }
    fn tick(&mut self, ctx: &mut StriderGoalContext) {
        let Some(pos) = ctx.go_to_lava.target else {
            return;
        };
        let state = &mut ctx.go_to_lava;
        if block_center(pos).distance_squared(ctx.child.position) >= 1.0 {
            state.try_ticks += 1;
            if state.try_ticks % 20 == 0 {
                ctx.effects.push(StriderGoalEffect::Navigate {
                    target: DVec3::new(
                        f64::from(pos.0) + 0.5,
                        f64::from(pos.1),
                        f64::from(pos.2) + 0.5,
                    ),
                    speed: 1.0,
                });
            }
        } else {
            state.try_ticks -= 1;
        }
    }
}

#[derive(Clone)]
struct FollowParentGoal;
impl Goal<StriderGoalContext> for FollowParentGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }
    fn can_start(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        ctx.follow_parent.can_use(ctx.child, &ctx.candidates)
    }
    fn can_continue(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        ctx.follow_parent.can_continue(ctx.child, &ctx.candidates)
    }
    fn start(&mut self, ctx: &mut StriderGoalContext) {
        ctx.follow_parent.start();
    }
    fn stop(&mut self, ctx: &mut StriderGoalContext) {
        ctx.follow_parent.stop();
    }
    fn tick(&mut self, ctx: &mut StriderGoalContext) {
        if let Some(target) = ctx.follow_parent.tick(&ctx.candidates) {
            ctx.effects
                .push(StriderGoalEffect::NavigateToEntity { target, speed: 1.0 });
        }
    }
}

#[derive(Clone)]
struct StrollGoal;
impl Goal<StriderGoalContext> for StrollGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut StriderGoalContext, world: &dyn World) -> bool {
        if ctx.no_action_time >= 100 || ctx.random.next_int(60) != 0 {
            return false;
        }
        ctx.stroll.wanted = crate::stroll::default_random_position(
            world,
            &ctx.walk,
            ctx.child.position,
            10,
            7,
            &mut ctx.random,
        );
        ctx.stroll.wanted.is_some()
    }
    fn start(&mut self, ctx: &mut StriderGoalContext) {
        if let Some(target) = ctx.stroll.wanted {
            ctx.effects
                .push(StriderGoalEffect::Navigate { target, speed: 1.0 });
        }
    }
    fn can_continue(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        !ctx.navigation_done
    }
    fn stop(&mut self, ctx: &mut StriderGoalContext) {
        ctx.effects.push(StriderGoalEffect::StopNavigation);
    }
}

#[derive(Clone)]
struct LookAtGoal {
    striders: bool,
}
impl LookAtGoal {
    fn targets(&self, ctx: &StriderGoalContext) -> Vec<LookTarget> {
        if self.striders {
            ctx.look_striders.clone()
        } else {
            ctx.look_players.clone()
        }
    }
    fn state<'a>(&self, ctx: &'a mut StriderGoalContext) -> &'a mut LookAtState {
        if self.striders {
            &mut ctx.look_at_strider
        } else {
            &mut ctx.look_at_player
        }
    }
}
impl Goal<StriderGoalContext> for LookAtGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Look])
    }
    fn can_start(&mut self, ctx: &mut StriderGoalContext, world: &dyn World) -> bool {
        if ctx.random.next_float() >= 0.02 {
            return false;
        }
        let eye = ctx.child.position + DVec3::new(0.0, f64::from(ctx.eye_height), 0.0);
        let position = ctx.child.position;
        let id = ctx.child.id;
        let best = self
            .targets(ctx)
            .into_iter()
            .filter(|target| target.id != id)
            .filter(|target| position.distance_squared(target.eye) <= LOOK_RANGE * LOOK_RANGE)
            .filter(|target| crate::sight::line_of_sight(world, eye, target.eye))
            .min_by(|a, b| {
                a.eye
                    .distance_squared(eye)
                    .total_cmp(&b.eye.distance_squared(eye))
            });
        self.state(ctx).target = best.map(|target| (target.id, target.eye));
        best.is_some()
    }
    fn can_continue(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        let position = ctx.child.position;
        let targets = self.targets(ctx);
        let state = self.state(ctx);
        state.look_time > 0
            && state.target.is_some_and(|(id, _)| {
                targets.iter().any(|target| {
                    target.id == id
                        && position.distance_squared(target.eye) <= LOOK_RANGE * LOOK_RANGE
                })
            })
    }
    fn start(&mut self, ctx: &mut StriderGoalContext) {
        let duration = 40 + ctx.random.next_int(40) as i32;
        self.state(ctx).look_time = (duration + 1) / 2;
    }
    fn stop(&mut self, ctx: &mut StriderGoalContext) {
        self.state(ctx).target = None;
    }
    fn tick(&mut self, ctx: &mut StriderGoalContext) {
        let targets = self.targets(ctx);
        let state = self.state(ctx);
        state.look_time -= 1;
        let Some((id, _)) = state.target else {
            return;
        };
        if let Some(target) = targets.iter().find(|target| target.id == id) {
            ctx.effects.push(StriderGoalEffect::LookAt {
                target: target.eye,
                y_max: 10.0,
                x_max: 40.0,
            });
        }
    }
}

#[derive(Clone)]
struct RandomLookGoal;
impl Goal<StriderGoalContext> for RandomLookGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        RandomLookState::can_use(&mut ctx.random)
    }
    fn can_continue(&mut self, ctx: &mut StriderGoalContext, _world: &dyn World) -> bool {
        ctx.random_look.can_continue()
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, ctx: &mut StriderGoalContext) {
        ctx.random_look.start(&mut ctx.random);
    }
    fn tick(&mut self, ctx: &mut StriderGoalContext) {
        let target = ctx.random_look.tick(ctx.child.position, ctx.eye_height);
        ctx.effects.push(StriderGoalEffect::LookAt {
            target,
            y_max: 10.0,
            x_max: 40.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::controls::PlayerControls;
    use crate::strider::Strider;
    use crate::tempt::PlayerCandidate;
    use crate::world::EntityWorld;
    use glam::DVec3;
    use minecraftoss_player::crafting::RecipeBook;
    use minecraftoss_player::{Block, Pos, World};
    use std::collections::BTreeMap;
    use std::io::Write;

    #[derive(Default)]
    struct Blocks(BTreeMap<Pos, Block>);

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.0.get(&pos).cloned()
        }
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.0.insert(pos, block),
                None => self.0.remove(&pos),
            };
        }
    }

    fn lava() -> Block {
        Block::new("minecraft:lava").with("level", "0")
    }

    fn ground(lava_from_x: Option<i32>) -> Blocks {
        let mut world = Blocks::default();
        for x in -16..=16 {
            for z in -16..=16 {
                world
                    .0
                    .insert((x, 62, z), Block::new("minecraft:netherrack"));
                let pool = lava_from_x.is_some_and(|from| x >= from);
                let block = if pool {
                    lava()
                } else {
                    Block::new("minecraft:netherrack")
                };
                world.0.insert((x, 63, z), block);
            }
        }
        world
    }

    fn recipes() -> RecipeBook {
        let jar = std::env::temp_dir().join(format!("strider-ai-{}.jar", std::process::id()));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        let tags = [
            (
                "strider_food",
                serde_json::json!({ "values": ["minecraft:warped_fungus"] }),
            ),
            (
                "strider_tempt_items",
                serde_json::json!({ "values": ["#minecraft:strider_food", "minecraft:warped_fungus_on_a_stick"] }),
            ),
        ];
        for (tag, value) in tags {
            zip.start_file(
                format!("data/minecraft/tags/item/{tag}.json"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(value.to_string().as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        let book = RecipeBook::from_jar(&jar).unwrap();
        let _ = std::fs::remove_file(&jar);
        book
    }

    fn player(at: DVec3) -> PlayerCandidate {
        PlayerCandidate {
            id: 0,
            position: at,
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true,
        }
    }

    fn in_lava(world: &Blocks, position: DVec3) -> bool {
        crate::fluid::FluidFrame::sample(world, position, 0.9, 1.7).in_lava()
    }

    #[test]
    fn a_strider_on_netherrack_walks_to_lava_within_eight_blocks() {
        let mut world = ground(Some(6));
        assert_eq!(
            super::find_lava(&world, DVec3::new(0.5, 64.0, 0.5)),
            Some((6, 63, 0))
        );
        assert_eq!(
            super::find_lava(&world, DVec3::new(-3.5, 64.0, 0.5)),
            None,
            "nine blocks is too far"
        );
        let mut entities = EntityWorld::default();
        let id = entities.spawn_strider(Strider::new(DVec3::new(0.5, 64.0, 0.5)), false);
        let mut reached = None;
        for tick in 0..800 {
            entities.tick_with_players(&mut world, &[]);
            let position = entities.strider_mut(id).unwrap().position();
            if in_lava(&world, position) {
                reached = Some(tick);
                break;
            }
        }
        assert!(
            reached.is_some(),
            "it went into the lava: {:?}",
            entities.strider_mut(id).unwrap().position()
        );
    }

    #[test]
    fn a_strider_in_lava_never_strolls_onto_land() {
        let mut world = ground(None);
        for x in -7..=7 {
            for z in -7..=7 {
                world.0.insert((x, 63, z), lava());
            }
        }
        let mut entities = EntityWorld::default();
        let id = entities.spawn_strider(Strider::new(DVec3::new(0.5, 63.5, 0.5)), false);
        let watcher = [player(DVec3::new(0.5, 64.0, 12.5))];
        let mut moved = 0.0_f64;
        for _ in 0..3000 {
            entities.tick_with_players(&mut world, &watcher);
            let position = entities.strider_mut(id).unwrap().position();
            moved = moved.max(position.distance(DVec3::new(0.5, 63.5, 0.5)));
            assert!(
                in_lava(&world, position),
                "it left the lava at {position:?}"
            );
        }
        assert!(moved > 1.0, "it strolled about the pool ({moved})");
    }

    #[test]
    fn warped_fungus_and_the_stick_tempt_it_at_1_4() {
        for held in [
            "minecraft:warped_fungus",
            "minecraft:warped_fungus_on_a_stick",
        ] {
            let mut world = ground(None);
            let mut entities = EntityWorld::default();
            entities.set_recipe_book(std::sync::Arc::new(recipes()));
            let id = entities.spawn_strider(Strider::new(DVec3::new(0.5, 64.0, 0.5)), false);
            let at = DVec3::new(7.5, 64.0, 0.5);
            entities.set_player_controls(&[PlayerControls {
                id: 0,
                main_hand: Some(held.to_owned()),
                ..PlayerControls::default()
            }]);
            for _ in 0..40 {
                entities.tick_with_players(&mut world, &[player(at)]);
            }
            let entity = entities.strider_mut(id).unwrap();
            assert!(
                entity.running_goals().contains(&"TemptGoal"),
                "{held}: {:?}",
                entity.running_goals()
            );
            assert!(entity.tempted());
            assert!(
                (entity.strider.navigation.speed_modifier - super::TEMPT_SPEED).abs() < 1e-9
                    || entity.position().distance(at) < 2.5
            );
            assert!(
                entity.position().x > 1.5,
                "{held}: it follows ({:?})",
                entity.position()
            );
        }
        let mut world = ground(None);
        let mut entities = EntityWorld::default();
        entities.set_recipe_book(std::sync::Arc::new(recipes()));
        let id = entities.spawn_strider(Strider::new(DVec3::new(0.5, 64.0, 0.5)), false);
        entities.set_player_controls(&[PlayerControls {
            id: 0,
            main_hand: Some("minecraft:carrot_on_a_stick".to_owned()),
            ..PlayerControls::default()
        }]);
        for _ in 0..40 {
            entities.tick_with_players(&mut world, &[player(DVec3::new(7.5, 64.0, 0.5))]);
        }
        assert!(
            !entities.strider_mut(id).unwrap().tempted(),
            "a carrot on a stick leaves it cold"
        );
    }
}
