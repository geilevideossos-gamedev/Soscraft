use crate::goals::{Control, Controls, Goal};
use crate::monster_ai::MonsterGoalContext;
use glam::DVec3;
use minecraftoss_player::{Block, Pos, World};

pub const ATTACK_WOODEN_DOOR: i32 = 1019;
pub const BREAK_WOODEN_DOOR: i32 = 1021;
pub const DOOR_BREAK_TIME: i32 = 240;
const SWING_TICKS: i64 = 6;
const RAIDER_BREAK_ROLL: u32 = 5;
const OPEN_DOOR_FORGET_TIME: i32 = 20;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DoorWork {
    pub toggles: Vec<(Pos, bool)>,
    pub events: Vec<(i32, Pos)>,
    pub broken: Option<Pos>,
    pub crack: Option<(Pos, u8)>,
    pub swing_until: i64,
}

impl DoorWork {
    pub fn take_requests(&mut self) -> DoorWork {
        DoorWork {
            toggles: std::mem::take(&mut self.toggles),
            events: std::mem::take(&mut self.events),
            broken: self.broken.take(),
            crack: None,
            swing_until: 0,
        }
    }

    pub fn door_open(&self, world: &dyn World, pos: Pos) -> Option<bool> {
        if let Some(&(_, open)) = self.toggles.iter().rev().find(|(p, _)| *p == pos) {
            return Some(open);
        }
        let door = world.block(pos).filter(|b| b.id.ends_with("_door"))?;
        Some(door.property("open") == Some("true"))
    }
}

pub fn is_wooden_door(block: &Block) -> bool {
    block.id.ends_with("_door") && block.id != "minecraft:iron_door"
}

fn wooden_door_at(world: &dyn World, pos: Pos) -> bool {
    world.block(pos).is_some_and(|b| is_wooden_door(&b))
}

fn has_active_raid(ctx: &MonsterGoalContext) -> bool {
    ctx.illager.raid.is_some_and(|raid| raid.active)
}

fn closer_to_center(pos: Pos, position: DVec3, distance: f64) -> bool {
    let center = DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    );
    center.distance_squared(position) < distance * distance
}

#[derive(Clone, Debug, Default)]
pub struct DoorInteract {
    pos: Pos,
    has_door: bool,
    passed: bool,
    open_dir: (f32, f32),
}

impl DoorInteract {
    pub fn pos(&self) -> Pos {
        self.pos
    }

    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn can_use(&mut self, ctx: &MonsterGoalContext, world: &dyn World) -> bool {
        if !ctx.body.horizontal_collision || ctx.navigation.is_done() {
            return false;
        }
        let position = ctx.body.position;
        let nodes = &ctx.navigation.nodes;
        for &(x, y, z) in &nodes[..(ctx.navigation.next + 2).min(nodes.len())] {
            self.pos = (x, y + 1, z);
            let (dx, dz) = (position.x - f64::from(x), position.z - f64::from(z));
            if dx * dx + dz * dz <= 2.25 {
                self.has_door = wooden_door_at(world, self.pos);
                if self.has_door {
                    return true;
                }
            }
        }
        self.pos = (
            position.x.floor() as i32,
            position.y.floor() as i32 + 1,
            position.z.floor() as i32,
        );
        self.has_door = wooden_door_at(world, self.pos);
        self.has_door
    }

    pub fn is_open(&mut self, work: &DoorWork, world: &dyn World) -> bool {
        if !self.has_door {
            return false;
        }
        match work.door_open(world, self.pos) {
            Some(open) => open,
            None => {
                self.has_door = false;
                false
            }
        }
    }

    fn direction(&self, position: DVec3) -> (f32, f32) {
        (
            (f64::from(self.pos.0) + 0.5 - position.x) as f32,
            (f64::from(self.pos.2) + 0.5 - position.z) as f32,
        )
    }

    pub fn set_open(&self, work: &mut DoorWork, open: bool) {
        if self.has_door {
            work.toggles.push((self.pos, open));
        }
    }

    pub fn start(&mut self, ctx: &MonsterGoalContext) {
        self.passed = false;
        self.open_dir = self.direction(ctx.body.position);
    }

    pub fn tick(&mut self, ctx: &MonsterGoalContext) {
        let (x, z) = self.direction(ctx.body.position);
        if self.open_dir.0 * x + self.open_dir.1 * z < 0.0 {
            self.passed = true;
        }
    }
}

#[derive(Clone)]
pub struct BreakDoorGoal {
    door: DoorInteract,
    valid_difficulty: fn(i32) -> bool,
    needs_can_break_doors: bool,
    raider: bool,
    break_time: i32,
    last_progress: i32,
}

impl BreakDoorGoal {
    pub fn zombie() -> Self {
        Self {
            door: DoorInteract::default(),
            valid_difficulty: |difficulty| difficulty == 3,
            needs_can_break_doors: true,
            raider: false,
            break_time: 0,
            last_progress: -1,
        }
    }

    pub fn vindicator() -> Self {
        Self {
            door: DoorInteract::default(),
            valid_difficulty: |difficulty| difficulty == 2 || difficulty == 3,
            needs_can_break_doors: false,
            raider: true,
            break_time: 0,
            last_progress: -1,
        }
    }

    fn unable(&self, ctx: &MonsterGoalContext) -> bool {
        (self.needs_can_break_doors && !ctx.can_break_doors)
            || (self.raider && !has_active_raid(ctx))
    }
}

impl Goal<MonsterGoalContext> for BreakDoorGoal {
    fn controls(&self) -> Controls {
        if self.raider {
            Controls::new(&[Control::Move])
        } else {
            Controls::new(&[])
        }
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if self.unable(ctx) {
            return false;
        }
        if self.raider && ctx.random.next_int(RAIDER_BREAK_ROLL) != 0 {
            return false;
        }
        self.door.can_use(ctx, world)
            && ctx.mob_griefing
            && (self.valid_difficulty)(ctx.difficulty)
            && !self.door.is_open(&ctx.door, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if self.unable(ctx) {
            return false;
        }
        self.break_time <= DOOR_BREAK_TIME
            && !self.door.is_open(&ctx.door, world)
            && closer_to_center(self.door.pos(), ctx.body.position, 2.0)
            && (self.valid_difficulty)(ctx.difficulty)
    }

    fn every_tick(&self) -> bool {
        true
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.door.start(ctx);
        self.break_time = 0;
        if self.raider {
            ctx.illager.reset_idle = true;
        }
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        ctx.door.crack = None;
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        self.door.tick(ctx);
        let pos = self.door.pos();
        if ctx.random.next_int(20) == 0 {
            ctx.door.events.push((ATTACK_WOODEN_DOOR, pos));
            if ctx.game_time >= ctx.door.swing_until {
                ctx.door.swing_until = ctx.game_time + SWING_TICKS + 1;
                ctx.melee.swings = ctx.melee.swings.wrapping_add(1);
            }
        }
        self.break_time += 1;
        let progress = (self.break_time as f32 / DOOR_BREAK_TIME as f32 * 10.0) as i32;
        if progress != self.last_progress {
            ctx.door.crack = u8::try_from(progress)
                .ok()
                .filter(|&stage| stage < 10)
                .map(|stage| (pos, stage));
            self.last_progress = progress;
        }
        if self.break_time == DOOR_BREAK_TIME && (self.valid_difficulty)(ctx.difficulty) {
            ctx.door.broken = Some(pos);
            ctx.door.events.push((BREAK_WOODEN_DOOR, pos));
        }
    }
}

#[derive(Clone)]
pub struct OpenDoorGoal {
    door: DoorInteract,
    close_door: bool,
    raider: bool,
    forget_time: i32,
}

impl OpenDoorGoal {
    pub fn raider() -> Self {
        Self {
            door: DoorInteract::default(),
            close_door: false,
            raider: true,
            forget_time: 0,
        }
    }
}

impl Goal<MonsterGoalContext> for OpenDoorGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.door.can_use(ctx, world) && (!self.raider || has_active_raid(ctx))
    }

    fn can_continue(&mut self, _: &mut MonsterGoalContext, _: &dyn World) -> bool {
        self.close_door && self.forget_time > 0 && !self.door.passed()
    }

    fn every_tick(&self) -> bool {
        true
    }

    fn start(&mut self, ctx: &mut MonsterGoalContext) {
        self.forget_time = OPEN_DOOR_FORGET_TIME;
        self.door.set_open(&mut ctx.door, true);
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.door.set_open(&mut ctx.door, false);
    }

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        self.forget_time -= 1;
        self.door.tick(ctx);
    }
}

impl crate::world::EntityWorld {
    pub fn door_cracks(&self) -> Vec<(u64, Pos, u8)> {
        let zombies = self
            .zombies()
            .iter()
            .filter(|e| e.zombie.health > 0.0)
            .filter_map(|e| Some((e.id, e.ai.as_deref()?.state.door.crack?)));
        let illagers = self
            .illagers()
            .iter()
            .filter(|e| e.illager.health > 0.0)
            .filter_map(|e| Some((e.id, e.ai.state.door.crack?)));
        zombies
            .chain(illagers)
            .map(|(id, (pos, stage))| (id, pos, stage))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goals::GoalSelector;
    use crate::monster_ai::{MonsterAi, MonsterKind};
    use crate::movement::Body;
    use minecraftoss_player::rng::LegacyRandom;
    use std::collections::HashMap;

    struct Doors(HashMap<Pos, Block>);

    impl World for Doors {
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

    const UPPER: Pos = (0, 2, 1);

    fn house(id: &str) -> Doors {
        let half = |half: &str| {
            Block::new(id)
                .with("facing", "north")
                .with("half", half)
                .with("hinge", "left")
                .with("open", "false")
                .with("powered", "false")
        };
        Doors(HashMap::from([
            ((0, 1, 1), half("lower")),
            (UPPER, half("upper")),
        ]))
    }

    fn zombie_at_the_door() -> MonsterGoalContext {
        let mut body = Body::new(DVec3::new(0.5, 1.0, 0.5), 0.6, 1.95);
        body.horizontal_collision = true;
        let mut ctx = MonsterAi::of_kind(MonsterKind::Zombie, &body, 0.0).state;
        ctx.difficulty = 3;
        ctx.can_break_doors = true;
        ctx.random = LegacyRandom::new(5);
        ctx.navigation
            .follow(vec![(0, 1, 0), (0, 1, 1), (0, 1, 2)], 1.0, body.position);
        ctx
    }

    fn breaker() -> GoalSelector<MonsterGoalContext> {
        let mut selector = GoalSelector::default();
        selector.add(1, BreakDoorGoal::zombie());
        selector
    }

    fn tick(
        selector: &mut GoalSelector<MonsterGoalContext>,
        ctx: &mut MonsterGoalContext,
        world: &Doors,
    ) {
        ctx.game_time += 1;
        if ctx.game_time % 2 == 1 {
            selector.tick(ctx, world);
        } else {
            selector.tick_running_world(ctx, world, false);
        }
    }

    fn breaking(selector: &GoalSelector<MonsterGoalContext>) -> bool {
        selector.running_ids().next().is_some()
    }

    #[test]
    fn a_door_breaking_zombie_knocks_cracks_and_breaks_a_wooden_door_after_240_ticks_on_hard() {
        let world = house("minecraft:oak_door");
        let mut ctx = zombie_at_the_door();
        let mut selector = breaker();
        let (mut stages, mut knocks) = (Vec::new(), 0);
        for t in 1..=DOOR_BREAK_TIME {
            tick(&mut selector, &mut ctx, &world);
            if let Some((pos, stage)) = ctx.door.crack {
                assert_eq!(pos, UPPER);
                if stages.last() != Some(&stage) {
                    stages.push(stage);
                }
            }
            for (event, pos) in ctx.door.events.drain(..) {
                assert_eq!(pos, UPPER);
                if event == ATTACK_WOODEN_DOOR {
                    knocks += 1;
                } else {
                    assert_eq!((event, t), (BREAK_WOODEN_DOOR, DOOR_BREAK_TIME));
                }
            }
            assert_eq!(ctx.door.broken.is_some(), t == DOOR_BREAK_TIME, "tick {t}");
        }
        assert_eq!(stages, (0..10).collect::<Vec<u8>>());
        assert_eq!(ctx.door.crack, None, "the tenth stage clears the crack");
        assert_eq!(ctx.door.broken, Some(UPPER));
        assert!(knocks > 0, "it knocks now and then");
        assert!(
            ctx.melee.swings > 0 && ctx.melee.swings <= knocks,
            "{} swings",
            ctx.melee.swings
        );
    }

    #[test]
    fn no_door_breaking_on_normal_without_mob_griefing_at_an_iron_door_or_without_can_break_doors()
    {
        let refused: [(&str, fn(&mut MonsterGoalContext)); 5] = [
            ("minecraft:oak_door", |ctx| ctx.difficulty = 2),
            ("minecraft:oak_door", |ctx| ctx.mob_griefing = false),
            ("minecraft:iron_door", |_| {}),
            ("minecraft:oak_door", |ctx| ctx.can_break_doors = false),
            ("minecraft:oak_door", |ctx| {
                ctx.body.horizontal_collision = false
            }),
        ];
        for (door, change) in refused {
            let world = house(door);
            let mut ctx = zombie_at_the_door();
            change(&mut ctx);
            let mut selector = breaker();
            for _ in 0..40 {
                tick(&mut selector, &mut ctx, &world);
            }
            assert!(!breaking(&selector), "{door}");
            assert_eq!(ctx.door, DoorWork::default(), "{door}");
            assert_eq!(ctx.melee.swings, 0);
        }
        for door in [
            "minecraft:copper_door",
            "minecraft:spruce_door",
            "minecraft:waxed_oxidized_copper_door",
        ] {
            let world = house(door);
            let mut ctx = zombie_at_the_door();
            let mut selector = breaker();
            tick(&mut selector, &mut ctx, &world);
            assert!(breaking(&selector), "{door}");
            assert_eq!(ctx.door.crack, Some((UPPER, 0)), "{door}");
        }
    }

    #[test]
    fn switching_to_normal_stops_the_break_and_clears_the_crack() {
        let world = house("minecraft:oak_door");
        let mut ctx = zombie_at_the_door();
        let mut selector = breaker();
        for _ in 0..60 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert_eq!(ctx.door.crack, Some((UPPER, 2)));
        ctx.difficulty = 2;
        for _ in 0..2 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert!(!breaking(&selector));
        assert_eq!(ctx.door.crack, None);
        ctx.door.events.clear();
        for _ in 0..300 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert!(!breaking(&selector));
        assert_eq!(
            (ctx.door.events.len(), ctx.door.broken, ctx.door.crack),
            (0, None, None),
            "no knocks and no break on normal"
        );
    }

    #[test]
    fn opening_the_door_stops_a_break() {
        let mut world = house("minecraft:oak_door");
        let mut ctx = zombie_at_the_door();
        let mut selector = breaker();
        for _ in 0..30 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert_eq!(ctx.door.crack, Some((UPPER, 1)));
        ctx.door.toggles.push((UPPER, true));
        for _ in 0..2 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert!(
            !breaking(&selector),
            "a door opened this tick ends the break"
        );
        assert_eq!(ctx.door.crack, None);
        let opened = world.block(UPPER).unwrap().with("open", "true");
        world.set_block(UPPER, Some(opened));
        let mut ctx = zombie_at_the_door();
        let mut selector = breaker();
        for _ in 0..40 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert!(!breaking(&selector), "an open door is not broken");
        assert_eq!(ctx.door, DoorWork::default());
    }

    #[test]
    fn a_mob_walking_through_the_doorway_has_passed_it() {
        let world = house("minecraft:oak_door");
        let mut ctx = zombie_at_the_door();
        let mut door = DoorInteract::default();
        assert!(door.can_use(&ctx, &world));
        assert_eq!(door.pos(), UPPER);
        door.start(&ctx);
        door.tick(&ctx);
        assert!(!door.passed());
        ctx.body.position = DVec3::new(0.5, 1.0, 2.5);
        door.tick(&ctx);
        assert!(door.passed());
        ctx.navigation.stop();
        assert!(!door.can_use(&ctx, &world), "no path, no door");
    }

    fn raid(active: bool) -> crate::raid::RaidView {
        crate::raid::RaidView {
            id: 1,
            center: (0, 1, 20),
            active,
            over: false,
            loss: false,
            groups_spawned: 1,
            victory: false,
            stopped: false,
            between_waves: false,
        }
    }

    fn vindicator_at_the_door(
        difficulty: i32,
        raid: Option<crate::raid::RaidView>,
        seed: u64,
    ) -> MonsterGoalContext {
        let mut body = Body::new(DVec3::new(0.5, 1.0, 0.5), 0.6, 1.95);
        body.horizontal_collision = true;
        let mut ctx = MonsterAi::of_kind(MonsterKind::Vindicator, &body, 0.0).state;
        ctx.difficulty = difficulty;
        ctx.illager.raid = raid;
        ctx.random = LegacyRandom::new(seed);
        ctx.navigation
            .follow(vec![(0, 1, 0), (0, 1, 1), (0, 1, 2)], 1.0, body.position);
        ctx
    }

    fn raider_doors() -> GoalSelector<MonsterGoalContext> {
        let mut selector = GoalSelector::default();
        selector.add(2, BreakDoorGoal::vindicator());
        selector.add(3, OpenDoorGoal::raider());
        selector
    }

    fn swing_door(world: &mut Doors, ctx: &mut MonsterGoalContext) -> Vec<bool> {
        let toggles = ctx.door.take_requests().toggles;
        for &(pos, open) in &toggles {
            assert_eq!(pos, UPPER);
            for half in [UPPER, (0, 1, 1)] {
                let value = if open { "true" } else { "false" };
                let door = world.block(half).unwrap().with("open", value);
                world.set_block(half, Some(door));
            }
        }
        toggles.into_iter().map(|(_, open)| open).collect()
    }

    #[test]
    fn a_raiding_vindicator_opens_a_door_and_shuts_it_once_it_stops_bumping_into_it() {
        let mut world = house("minecraft:oak_door");
        let mut ctx = vindicator_at_the_door(3, Some(raid(true)), 5);
        let mut selector = raider_doors();
        tick(&mut selector, &mut ctx, &world);
        assert_eq!(swing_door(&mut world, &mut ctx), vec![true]);
        tick(&mut selector, &mut ctx, &world);
        assert_eq!(swing_door(&mut world, &mut ctx), Vec::<bool>::new());
        tick(&mut selector, &mut ctx, &world);
        assert_eq!(
            swing_door(&mut world, &mut ctx),
            vec![false, true],
            "OpenDoorGoal(mob, false) stops and starts again in one goal pass"
        );
        ctx.body.horizontal_collision = false;
        for _ in 0..2 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert_eq!(swing_door(&mut world, &mut ctx), vec![false]);
        assert_eq!(world.block(UPPER).unwrap().property("open"), Some("false"));
        for _ in 0..40 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert_eq!(swing_door(&mut world, &mut ctx), Vec::<bool>::new());
        assert!(!breaking(&selector));
        assert_eq!(ctx.door.crack, None);
    }

    #[test]
    fn a_vindicator_outside_an_active_raid_leaves_doors_alone_and_draws_nothing() {
        for raid in [None, Some(raid(false))] {
            let mut world = house("minecraft:oak_door");
            let mut ctx = vindicator_at_the_door(3, raid, 5);
            let mut selector = raider_doors();
            for _ in 0..40 {
                tick(&mut selector, &mut ctx, &world);
            }
            assert!(!breaking(&selector), "{raid:?}");
            assert_eq!(swing_door(&mut world, &mut ctx), Vec::<bool>::new());
            assert_eq!(ctx.door, DoorWork::default());
            assert_eq!(
                ctx.random.next_int(1 << 30),
                LegacyRandom::new(5).next_int(1 << 30),
                "the roll waits for an active raid"
            );
        }
    }

    #[test]
    fn a_raiding_vindicator_chops_at_a_door_on_one_full_tick_in_five_on_normal_and_hard() {
        assert_eq!(
            BreakDoorGoal::vindicator().controls(),
            Controls::new(&[Control::Move])
        );
        assert_eq!(BreakDoorGoal::zombie().controls(), Controls::new(&[]));
        let mut at_once = 0;
        for seed in 0..40 {
            let difficulty = 2 + (seed % 2) as i32;
            let world = house("minecraft:oak_door");
            let mut ctx = vindicator_at_the_door(difficulty, Some(raid(true)), seed);
            let mut selector = GoalSelector::default();
            selector.add(2, BreakDoorGoal::vindicator());
            let mut rolls = LegacyRandom::new(seed);
            let full_ticks = (0..).position(|_| rolls.next_int(5) == 0).unwrap() as i64;
            at_once += i32::from(full_ticks == 0);
            let started = 2 * full_ticks + 1;
            while ctx.game_time < started {
                assert!(!breaking(&selector), "seed {seed}");
                tick(&mut selector, &mut ctx, &world);
            }
            assert!(breaking(&selector), "seed {seed}");
            assert!(ctx.illager.reset_idle, "setNoActionTime(0)");
            assert_eq!(ctx.door.crack, Some((UPPER, 0)));
            while ctx.door.broken.is_none() {
                tick(&mut selector, &mut ctx, &world);
            }
            assert_eq!(ctx.game_time, started + i64::from(DOOR_BREAK_TIME) - 1);
            assert_eq!(ctx.door.events.last(), Some(&(BREAK_WOODEN_DOOR, UPPER)));
        }
        assert!(
            (2..=16).contains(&at_once),
            "about one in five starts on its first full tick: {at_once}"
        );
        let world = house("minecraft:oak_door");
        let mut ctx = vindicator_at_the_door(1, Some(raid(true)), 5);
        let mut selector = GoalSelector::default();
        selector.add(2, BreakDoorGoal::vindicator());
        for _ in 0..100 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert!(!breaking(&selector), "not on Easy");
    }

    #[test]
    fn a_vindicator_stops_chopping_when_the_raid_ends() {
        let world = house("minecraft:oak_door");
        let mut ctx = vindicator_at_the_door(2, Some(raid(true)), 5);
        let mut selector = GoalSelector::default();
        selector.add(2, BreakDoorGoal::vindicator());
        while !matches!(ctx.door.crack, Some((_, stage)) if stage >= 2) {
            tick(&mut selector, &mut ctx, &world);
        }
        ctx.illager.raid = Some(raid(false));
        for _ in 0..2 {
            tick(&mut selector, &mut ctx, &world);
        }
        assert!(!breaking(&selector));
        assert_eq!(ctx.door.crack, None);
    }

    #[test]
    fn door_cracks_list_the_living_zombies_and_illagers_at_a_door() {
        use crate::illager::{Illager, IllagerKind};
        use crate::world::EntityWorld;
        use crate::zombie::Zombie;
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 1.0, 0.5);
        let zombie = world.spawn_zombie_active(Zombie::new(at), 0.0);
        let idle = world.spawn_zombie_active(Zombie::new(at), 0.0);
        let dead = world.spawn_zombie_active(Zombie::new(at), 0.0);
        let vindicator = world.spawn_illager(Illager::new(IllagerKind::Vindicator, at), false);
        let crack = |world: &mut EntityWorld, id: u64, stage: u8| {
            if let Some(entity) = world.zombie_mut(id) {
                entity.ai.as_deref_mut().unwrap().state.door.crack = Some((UPPER, stage));
            } else {
                world.illager_mut(id).unwrap().ai.state.door.crack = Some(((5, 2, 1), stage));
            }
        };
        crack(&mut world, zombie, 4);
        crack(&mut world, dead, 7);
        crack(&mut world, vindicator, 1);
        world.zombie_mut(dead).unwrap().zombie.health = 0.0;
        assert_eq!(
            world.door_cracks(),
            vec![(zombie, UPPER, 4), (vindicator, (5, 2, 1), 1)]
        );
        assert!(world.door_cracks().iter().all(|&(id, ..)| id != idle));
    }
}
