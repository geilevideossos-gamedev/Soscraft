use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::health::DamageResult;
use crate::monster_ai::{
    FloatGoal, HurtByTargetGoal, MeleeAttackGoal, MonsterAi, MonsterGoalContext, MonsterKind,
    NearestTargetGoal, Prey,
};
use crate::tempt::PlayerCandidate;
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::families::goals::{
    ClimbOnTopOfPowderSnowGoal, RandomStrollGoal, reduced_tick_delay,
};
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, MonsterSpec};
use crate::world::{EntityWorld, PlayerHitKind};
use glam::DVec3;
use minecraftoss_player::{Block, Pos, World};

pub const KIND: &str = "minecraft:silverfish";
pub const WIDTH: f32 = 0.4;
pub const HEIGHT: f32 = 0.3;
pub const EYE_HEIGHT: f32 = 0.13;
pub const MAX_HEALTH: f32 = 8.0;
pub const SPEED: f64 = 0.25;
pub const ATTACK_DAMAGE: f32 = 1.0;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const WAKE_DELAY: i32 = 10;
pub const STROLL_INTERVAL: u32 = 10;
pub const STEP_SOUND: &str = "entity.silverfish.step";
pub const ALWAYS_TRIGGERS: [&str; 1] = ["minecraft:magic"];
pub const HOSTS: [&str; 7] = [
    "minecraft:stone",
    "minecraft:cobblestone",
    "minecraft:stone_bricks",
    "minecraft:mossy_stone_bricks",
    "minecraft:cracked_stone_bricks",
    "minecraft:chiseled_stone_bricks",
    "minecraft:deepslate",
];
pub const INFESTED: [&str; 7] = [
    "minecraft:infested_stone",
    "minecraft:infested_cobblestone",
    "minecraft:infested_stone_bricks",
    "minecraft:infested_mossy_stone_bricks",
    "minecraft:infested_cracked_stone_bricks",
    "minecraft:infested_chiseled_stone_bricks",
    "minecraft:infested_deepslate",
];
pub const WAKE_HEIGHT: i32 = 5;
pub const WAKE_REACH: i32 = 10;
pub const MERGE_INTERVAL: u32 = 10;
pub const SPAWN_ANIM: u8 = 20;
const DIRECTIONS: [Pos; 6] = [
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
    (-1, 0, 0),
    (1, 0, 0),
];

pub fn infested_of(host: &Block) -> Option<Block> {
    let index = HOSTS.iter().position(|id| *id == host.id)?;
    Some(Block {
        id: INFESTED[index].to_owned(),
        properties: host.properties.clone(),
    })
}

pub fn host_of(infested: &Block) -> Option<Block> {
    let index = INFESTED.iter().position(|id| *id == infested.id)?;
    Some(Block {
        id: HOSTS[index].to_owned(),
        properties: infested.properties.clone(),
    })
}

fn offsets(reach: i32) -> impl Iterator<Item = i32> {
    (0..=2 * reach).map(|i| if i % 2 == 1 { (i + 1) / 2 } else { -(i / 2) })
}

fn block_of(position: DVec3) -> Pos {
    (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

pub const GOAL_NAMES: [&str; 5] = [
    "FloatGoal",
    "ClimbOnTopOfPowderSnowGoal",
    "SilverfishWakeUpFriendsGoal",
    "MeleeAttackGoal",
    "SilverfishMergeWithStoneGoal",
];
pub const TARGET_NAMES: [&str; 2] = ["HurtByTargetGoal", "NearestAttackableTargetGoal"];

static SPEC: MonsterSpec = MonsterSpec {
    goal_names: &GOAL_NAMES,
    target_names: &TARGET_NAMES,
    goals,
    targets,
    eye_height: EYE_HEIGHT,
    follow_range: FOLLOW_RANGE,
    walk,
};

fn goals() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(1, FloatGoal);
    selector.add(1, ClimbOnTopOfPowderSnowGoal);
    selector.add(3, WakeUpFriendsGoal);
    selector.add(
        4,
        MeleeAttackGoal {
            hits: true,
            raises_arms: false,
            follow_unseen: false,
            gives_up_in_light: false,
            speed: 1.0,
        },
    );
    selector.add(
        5,
        MergeWithStoneGoal {
            stroll: RandomStrollGoal::new(1.0, STROLL_INTERVAL),
            selected: None,
        },
    );
    selector
}

fn targets() -> GoalSelector<MonsterGoalContext> {
    let mut selector = GoalSelector::default();
    selector.add(
        1,
        HurtByTargetGoal {
            timestamp: 0,
            unseen_ticks: 0,
            target_mob: None,
            alert_others: true,
        },
    );
    selector.add(2, NearestTargetGoal::every(Prey::Player, true, 5));
    selector
}

fn walk(walk: &mut WalkProfile) {
    walk.walk_target = WalkTarget::MonsterOver(&HOSTS);
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SilverfishExtra {
    pub look_for_friends: i32,
    pub woken: Vec<(Pos, Option<Block>)>,
    pub merge: Option<(Pos, Block)>,
}

impl SilverfishExtra {
    pub fn notify_hurt(&mut self) {
        if self.look_for_friends == 0 {
            self.look_for_friends = WAKE_DELAY;
        }
    }
}

#[derive(Clone)]
struct WakeUpFriendsGoal;

impl Goal<MonsterGoalContext> for WakeUpFriendsGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, _world: &dyn World) -> bool {
        ctx.extra
            .get::<SilverfishExtra>()
            .is_some_and(|extra| extra.look_for_friends > 0)
    }

    fn tick_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(extra) = ctx.extra.get_mut::<SilverfishExtra>() else {
            return;
        };
        extra.look_for_friends -= 1;
        if extra.look_for_friends > 0 {
            return;
        }
        let origin = block_of(ctx.body.position);
        let griefing = ctx.mob_griefing;
        'search: for y in offsets(WAKE_HEIGHT) {
            for x in offsets(WAKE_REACH) {
                for z in offsets(WAKE_REACH) {
                    let pos = (origin.0 + x, origin.1 + y, origin.2 + z);
                    let Some(host) = world.block(pos).and_then(|block| host_of(&block)) else {
                        continue;
                    };
                    extra.woken.push((pos, (!griefing).then_some(host)));
                    if ctx.random.next_boolean() {
                        break 'search;
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
struct MergeWithStoneGoal {
    stroll: RandomStrollGoal,
    selected: Option<Pos>,
}

impl Goal<MonsterGoalContext> for MergeWithStoneGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.target().is_some() || !ctx.navigation.is_done() {
            return false;
        }
        self.selected = None;
        if ctx.mob_griefing && ctx.random.next_int(reduced_tick_delay(MERGE_INTERVAL)) == 0 {
            let (dx, dy, dz) = DIRECTIONS[ctx.random.next_int(6) as usize];
            let cell = block_of(ctx.body.position + DVec3::new(0.0, 0.5, 0.0));
            let pos = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
            if world
                .block(pos)
                .is_some_and(|block| infested_of(&block).is_some())
            {
                self.selected = Some(pos);
                return true;
            }
        }
        self.stroll.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.selected.is_none() && self.stroll.can_continue(ctx, world)
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        let Some(pos) = self.selected else {
            self.stroll.start_world(ctx, world);
            return;
        };
        let Some(infested) = world.block(pos).and_then(|block| infested_of(&block)) else {
            return;
        };
        if let Some(extra) = ctx.extra.get_mut::<SilverfishExtra>() {
            extra.merge = Some((pos, infested));
        }
    }

    fn stop(&mut self, ctx: &mut MonsterGoalContext) {
        self.stroll.stop(ctx);
    }
}

#[derive(Clone)]
pub struct Silverfish {
    pub ai: Box<MonsterAi>,
}

impl Silverfish {
    pub fn extra(&self) -> SilverfishExtra {
        self.ai
            .state
            .extra
            .get::<SilverfishExtra>()
            .cloned()
            .unwrap_or_default()
    }
}

impl Family for Silverfish {
    const KEY: u8 = 36;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let mut ai = MonsterAi::of_kind(MonsterKind::Registered(kind), &core.body, core.yaw);
        ai.state.extra.set(SilverfishExtra::default());
        Some(FamilyMob {
            core,
            state: Silverfish { ai: Box::new(ai) },
        })
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        if base.alive && !base.no_ai {
            if let Some(mob) = world.family_mob_mut::<Self>(id) {
                mob.state.ai.body_rotation.body_yaw = mob.core.yaw;
            }
            let strike =
                world.family_goal_step::<Self, W>(id, level, players, SPEED, |fish| &mut *fish.ai);
            apply_blocks(world, id, level);
            let alert = world
                .family_mob_mut::<Self>(id)
                .and_then(|mob| mob.state.ai.state.alert.take());
            if let Some(target) = strike {
                let attacker = world.body_of(id).map_or(DVec3::ZERO, |body| body.position);
                let kind = PlayerHitKind::Melee {
                    attacker,
                    hunger_ticks: 0,
                    lift: 0.0,
                    throw: DVec3::ZERO,
                    disable_shield_seconds: 0.0,
                };
                world.family_strike(id, target, ATTACK_DAMAGE, kind);
            }
            if let Some(attacker) = alert {
                world.family_alert::<Self>(id, attacker, |fish| &mut *fish.ai);
            }
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        if hurt.attacker.is_some() || ALWAYS_TRIGGERS.contains(&hurt.damage_type) {
            if let Some(extra) = mob.state.ai.state.extra.get_mut::<SilverfishExtra>() {
                extra.notify_hurt();
            }
        }
        mob.core.hurt_plain(hurt.amount)
    }

    fn enemy(_kind: &str) -> bool {
        true
    }

    fn monster(kind: &str) -> Option<&'static MonsterSpec> {
        (kind == KIND).then_some(&SPEC)
    }

    fn step_sound(_kind: &str) -> Option<&'static str> {
        Some(STEP_SOUND)
    }

    fn emits_sounds(_kind: &str) -> bool {
        false
    }
}

fn apply_blocks<W: World>(world: &mut EntityWorld, id: u64, level: &mut W) {
    let Some(mob) = world.family_mob_mut::<Silverfish>(id) else {
        return;
    };
    let Some(extra) = mob.state.ai.state.extra.get_mut::<SilverfishExtra>() else {
        return;
    };
    let (woken, merge) = (std::mem::take(&mut extra.woken), extra.merge.take());
    for (pos, host) in woken {
        match host {
            Some(host) => level.set_block(pos, Some(host)),
            None => {
                level.destroy_block(pos);
            }
        }
    }
    if let Some((pos, infested)) = merge {
        level.set_block(pos, Some(infested));
        mob.core.removed = true;
        world.entity_events.push((id, SPAWN_ANIM));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monster_ai::Target;
    use crate::world::families::pests::endermites;
    use crate::world::{DamageSourceKind, PlayerAttack};
    use minecraftoss_player::{Block, Pos};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:dirt"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn player_at(id: u64, position: DVec3) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position,
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

    fn attack(player_id: u64, position: DVec3, attack_damage: f64) -> PlayerAttack {
        PlayerAttack {
            player_id,
            position,
            yaw: 0.0,
            attack_damage,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        }
    }

    fn fish_at(world: &mut EntityWorld, x: f64, z: f64) -> u64 {
        world
            .spawn_family_kind(KIND, DVec3::new(x, 64.0, z))
            .unwrap()
    }

    fn fish(world: &EntityWorld, id: u64) -> &FamilyMob<Silverfish> {
        world
            .family::<Silverfish>()
            .iter()
            .find(|mob| mob.core.id == id)
            .unwrap()
    }

    fn target(world: &EntityWorld, id: u64) -> Option<Target> {
        fish(world, id).state.ai.state.target
    }

    #[test]
    fn a_silverfish_is_a_small_eight_health_monster_that_bites_for_one() {
        let mut world = EntityWorld::default();
        let mut field = Field;
        let id = fish_at(&mut world, 0.5, 0.5);
        assert_eq!(world.entity_type(id), Some(KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(36));
        let core = &fish(&world, id).core;
        assert_eq!(
            (core.body.width, core.body.height, core.eye_height),
            (0.4, 0.3, 0.13)
        );
        assert_eq!((core.health, core.max_health), (8.0, 8.0));
        let census = world.census();
        let entry = census.iter().find(|c| c.kind == KIND).unwrap();
        assert_eq!((entry.width, entry.height), (WIDTH, HEIGHT));
        assert!(
            world
                .mob_candidates()
                .iter()
                .any(|m| m.id == id && m.kind == KIND)
        );
        assert!(crate::monster_ai::is_enemy(KIND));
        let traits = world.family_traits(id).unwrap();
        assert_eq!(traits.category, "monster");
        assert!(traits.enemy && !traits.undead && !traits.immune_to_poison);
        assert!(!traits.fire_immune && !traits.breathes_water && !traits.leashable);
        assert_eq!(traits.voice, ("silverfish", 1.0, "hostile_volume"));
        let registered = MonsterKind::Registered(KIND);
        assert_eq!(registered.goal_names(), GOAL_NAMES);
        assert_eq!(registered.target_goal_names(), TARGET_NAMES);
        let ai = &fish(&world, id).state.ai;
        assert_eq!(ai.state.follow_range, FOLLOW_RANGE);
        assert_eq!(ai.state.eye_height, EYE_HEIGHT);
        assert!(ai.state.walk.can_float, "its FloatGoal floats the path");
        assert_eq!(ai.state.walk.walk_target, WalkTarget::MonsterOver(&HOSTS));
        let players = [player_at(1, DVec3::new(0.5, 64.0, 4.5))];
        let (mut hits, mut sounds) = (Vec::new(), Vec::new());
        for _ in 0..200 {
            world.tick_with_players(&mut field, &players);
            hits.extend(world.take_player_hits());
            sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
            if !hits.is_empty() {
                break;
            }
        }
        let hit = hits.first().expect("it bites the player");
        assert_eq!(
            (hit.player_id, hit.damage, hit.source),
            (1, ATTACK_DAMAGE, Some(id))
        );
        let PlayerHitKind::Melee { attacker, .. } = hit.kind else {
            panic!("{:?}", hit.kind);
        };
        assert!(attacker.z > 2.0, "it walked up to bite: {attacker}");
        assert_eq!(fish(&world, id).core.swings, 1);
        assert!(
            sounds
                .iter()
                .all(|s| !s.ends_with(".step") && !s.ends_with(".swim")),
            "it walks without step sounds: {sounds:?}"
        );
        assert_eq!(target(&world, id), Some(Target::Player(1)));
        assert!(
            world
                .player_attack(&attack(1, players[0].position, 100.0), id)
                .died
        );
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        assert_eq!((deaths[0].id, deaths[0].table), (id, Some(KIND)));
        assert!(deaths[0].equipment.is_empty());
    }

    #[test]
    fn a_hurt_silverfish_alerts_the_others() {
        let mut world = EntityWorld::default();
        let mut field = Field;
        let hurt = fish_at(&mut world, 0.5, 0.5);
        let near = fish_at(&mut world, 4.5, 0.5);
        let behind = fish_at(&mut world, -6.5, -3.5);
        let far = fish_at(&mut world, 40.5, 0.5);
        let fallen = fish_at(&mut world, 100.5, 0.5);
        let poisoned = fish_at(&mut world, 100.5, 40.5);
        let mite = world
            .spawn_family_kind(endermites::KIND, DVec3::new(2.5, 64.0, 2.5))
            .unwrap();
        let players = [player_at(1, DVec3::new(0.5, 64.0, 20.5))];
        world.tick_with_players(&mut field, &players);
        let clock = |world: &EntityWorld, id| fish(world, id).state.extra().look_for_friends;
        assert_eq!(clock(&world, hurt), 0);
        let fall = Hurt::new(1.0, "minecraft:fall", DamageSourceKind::Generic);
        assert!(world.hurt_family(fallen, &fall).unwrap().applied);
        assert_eq!(clock(&world, fallen), 0, "a fall wakes no friends");
        let magic = Hurt::new(1.0, "minecraft:magic", DamageSourceKind::Magic);
        assert!(world.hurt_family(poisoned, &magic).unwrap().applied);
        assert_eq!(clock(&world, poisoned), WAKE_DELAY, "magic always does");
        assert!(
            world
                .player_attack(&attack(1, players[0].position, 1.0), hurt)
                .hurt
        );
        assert_eq!(clock(&world, hurt), WAKE_DELAY);
        for _ in 0..10 {
            world.tick_with_players(&mut field, &players);
            if target(&world, near).is_some() {
                break;
            }
        }
        assert_eq!(target(&world, near), Some(Target::Player(1)));
        assert_eq!(target(&world, behind), Some(Target::Player(1)));
        assert_eq!(target(&world, far), None, "outside its follow range");
        assert_eq!(target(&world, fallen), None);
        let mite_target = world
            .family::<endermites::Endermite>()
            .iter()
            .find(|m| m.core.id == mite)
            .unwrap()
            .state
            .ai
            .state
            .target;
        assert_eq!(mite_target, None, "only its own kind");
        let start = clock(&world, hurt);
        assert!(start < WAKE_DELAY, "the goal counts down: {start}");
        for _ in 0..40 {
            world.tick_with_players(&mut field, &players);
        }
        assert_eq!(clock(&world, hurt), 0);
        assert_eq!(clock(&world, poisoned), 0);
    }

    #[derive(Default)]
    struct Yard {
        blocks: std::collections::HashMap<Pos, Option<Block>>,
        destroyed: Vec<Pos>,
    }

    impl World for Yard {
        fn block(&self, pos: Pos) -> Option<Block> {
            match self.blocks.get(&pos) {
                Some(block) => block.clone(),
                None => (pos.1 < 64).then(|| Block::new("minecraft:dirt")),
            }
        }

        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            self.blocks.insert(pos, block);
        }

        fn destroy_block(&mut self, pos: Pos) -> bool {
            self.destroyed.push(pos);
            self.blocks.insert(pos, None);
            true
        }
    }

    impl Yard {
        fn put(&mut self, pos: Pos, block: Block) {
            self.blocks.insert(pos, Some(block));
        }

        fn ring(&mut self, id: &str) {
            for (x, z) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                for y in [64, 65] {
                    self.put((x, y, z), Block::new(id));
                }
            }
        }

        fn found(&self, id: &str) -> Vec<Pos> {
            let mut found: Vec<Pos> = self
                .blocks
                .iter()
                .filter(|(_, block)| block.as_ref().is_some_and(|block| block.id == id))
                .map(|(pos, _)| *pos)
                .collect();
            found.sort_unstable();
            found
        }
    }

    #[test]
    fn infested_blocks_and_their_hosts_turn_into_each_other() {
        for (host, infested) in HOSTS.iter().zip(INFESTED) {
            assert_eq!(infested_of(&Block::new(*host)), Some(Block::new(infested)));
            assert_eq!(host_of(&Block::new(infested)), Some(Block::new(*host)));
        }
        let pillar = Block::new("minecraft:deepslate").with("axis", "x");
        let infested = infested_of(&pillar).unwrap();
        let expected = Block::new("minecraft:infested_deepslate").with("axis", "x");
        assert_eq!(infested, expected, "the axis is kept");
        assert_eq!(host_of(&infested), Some(pillar));
        assert_eq!(infested_of(&Block::new("minecraft:andesite")), None);
        assert_eq!(host_of(&Block::new("minecraft:stone")), None);
        assert_eq!(offsets(2).collect::<Vec<_>>(), [0, 1, -1, 2, -2]);
        assert_eq!(offsets(WAKE_HEIGHT).count(), 11);
    }

    const WAKE_ORDER: [Pos; 16] = [
        (0, 64, 3),
        (1, 64, 3),
        (-1, 64, 3),
        (2, 64, 3),
        (-2, 64, 3),
        (0, 65, 3),
        (1, 65, 3),
        (-1, 65, 3),
        (2, 65, 3),
        (-2, 65, 3),
        (0, 63, 0),
        (0, 66, 3),
        (1, 66, 3),
        (-1, 66, 3),
        (2, 66, 3),
        (-2, 66, 3),
    ];

    fn infested_yard() -> Yard {
        let mut yard = Yard::default();
        yard.ring("minecraft:andesite");
        for &pos in WAKE_ORDER.iter().filter(|pos| pos.2 == 3) {
            yard.put(pos, Block::new("minecraft:infested_stone_bricks"));
        }
        let pillar = Block::new("minecraft:infested_deepslate").with("axis", "x");
        yard.put((0, 63, 0), pillar);
        yard
    }

    #[test]
    fn a_hurt_silverfish_wakes_infested_blocks_nearby() {
        for griefing in [true, false] {
            let mut world = EntityWorld::default();
            world.set_mob_griefing(griefing);
            let mut yard = infested_yard();
            let id = fish_at(&mut world, 0.5, 0.5);
            let players = [player_at(1, DVec3::new(0.5, 64.0, -6.5))];
            world.tick_with_players(&mut yard, &players);
            let hit = attack(1, players[0].position, 1.0);
            assert!(world.player_attack(&hit, id).hurt);
            let untouched = yard.blocks.clone();
            let mut waited = 0;
            while yard.blocks == untouched && waited < 60 {
                world.tick_with_players(&mut yard, &players);
                waited += 1;
            }
            assert!((16..=24).contains(&waited), "about a second: {waited}");
            let woken: Vec<Pos> = WAKE_ORDER
                .iter()
                .copied()
                .filter(|pos| yard.blocks[pos] != untouched[pos])
                .collect();
            assert!(!woken.is_empty() && woken.len() < WAKE_ORDER.len());
            assert_eq!(woken, WAKE_ORDER[..woken.len()], "vanilla's search order");
            for pos in &woken {
                let now = yard.block(*pos);
                if griefing {
                    assert_eq!(now, None, "broken: {pos:?}");
                } else if *pos == (0, 63, 0) {
                    let host = Block::new("minecraft:deepslate").with("axis", "x");
                    assert_eq!(now, Some(host));
                } else {
                    assert_eq!(now, Some(Block::new("minecraft:stone_bricks")), "{pos:?}");
                }
            }
            let expected = if griefing { woken.clone() } else { Vec::new() };
            assert_eq!(yard.destroyed, expected);
            assert_eq!(fish(&world, id).state.extra().look_for_friends, 0);
            let left = yard.found("minecraft:infested_stone_bricks").len();
            assert_eq!(left, 15 - woken.iter().filter(|pos| pos.2 == 3).count());
        }
    }

    #[test]
    fn an_idle_silverfish_merges_into_stone() {
        for griefing in [true, false] {
            let mut world = EntityWorld::default();
            world.set_mob_griefing(griefing);
            let mut yard = Yard::default();
            yard.ring("minecraft:stone_bricks");
            let id = fish_at(&mut world, 0.5, 0.5);
            let mut events = Vec::new();
            for _ in 0..2400 {
                world.tick_with_players(&mut yard, &[]);
                events.extend(world.take_entity_events());
                if world.entity_type(id).is_none() {
                    break;
                }
            }
            let merged = yard.found("minecraft:infested_stone_bricks");
            if !griefing {
                assert_eq!(
                    world.entity_type(id),
                    Some(KIND),
                    "no merging without griefing"
                );
                assert!(merged.is_empty());
                continue;
            }
            assert_eq!(world.entity_type(id), None, "it went into the wall");
            assert!(!world.mob_ids().any(|other| other == id));
            assert_eq!(merged.len(), 1, "{merged:?}");
            let sides = [(1, 64, 0), (-1, 64, 0), (0, 64, 1), (0, 64, -1)];
            assert!(
                sides.contains(&merged[0]),
                "a wall block beside it: {merged:?}"
            );
            assert_eq!(yard.found("minecraft:stone_bricks").len(), 7);
            assert!(yard.destroyed.is_empty());
            assert!(
                world.take_deaths().is_empty(),
                "no death, loot or experience"
            );
            assert_eq!(events, vec![(id, SPAWN_ANIM)], "its poof");
        }
    }

    #[test]
    fn a_merged_silverfish_leaves_its_body_for_its_poof() {
        let mut world = EntityWorld::default();
        world.set_mob_griefing(true);
        let mut yard = Yard::default();
        yard.ring("minecraft:stone_bricks");
        let id = fish_at(&mut world, 0.5, 0.5);
        let mut last = None;
        for _ in 0..2400 {
            last = world.body_of(id).cloned();
            world.tick_with_players(&mut yard, &[]);
            if world.entity_type(id).is_none() {
                break;
            }
        }
        assert_eq!(world.entity_type(id), None, "it went into the wall");
        let (kind, body) = world.departed(id).expect("its body for this tick");
        assert_eq!(kind, KIND);
        assert_eq!((body.width, body.height), (WIDTH, HEIGHT));
        let before = last.expect("it stood in the ring").position;
        assert!(body.position.distance(before) < 0.5, "{body:?}");
        assert!(world.departed(id + 1).is_none());
        world.tick_with_players(&mut yard, &[]);
        assert!(world.departed(id).is_none(), "only for the tick it left");
    }
}
