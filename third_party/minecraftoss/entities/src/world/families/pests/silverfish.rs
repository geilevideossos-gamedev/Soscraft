use crate::goals::{Control, Controls, Goal, GoalSelector};
use crate::health::DamageResult;
use crate::monster_ai::{
    FloatGoal, HurtByTargetGoal, MeleeAttackGoal, MonsterAi, MonsterGoalContext, MonsterKind,
    NearestTargetGoal, Prey,
};
use crate::tempt::PlayerCandidate;
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::families::goals::{ClimbOnTopOfPowderSnowGoal, RandomStrollGoal};
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, MonsterSpec};
use crate::world::{EntityWorld, PlayerHitKind};
use glam::DVec3;
use minecraftoss_player::World;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SilverfishExtra {
    pub look_for_friends: i32,
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

    fn tick(&mut self, ctx: &mut MonsterGoalContext) {
        if let Some(extra) = ctx.extra.get_mut::<SilverfishExtra>() {
            extra.look_for_friends -= 1;
        }
    }
}

#[derive(Clone)]
struct MergeWithStoneGoal {
    stroll: RandomStrollGoal,
}

impl Goal<MonsterGoalContext> for MergeWithStoneGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }

    fn can_start(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        if ctx.target().is_some() || !ctx.navigation.is_done() {
            return false;
        }
        self.stroll.can_start(ctx, world)
    }

    fn can_continue(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) -> bool {
        self.stroll.can_continue(ctx, world)
    }

    fn start_world(&mut self, ctx: &mut MonsterGoalContext, world: &dyn World) {
        self.stroll.start_world(ctx, world);
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
            .copied()
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
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
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
}
