mod brain;

pub use brain::{Activity, Brain, Pose, Shot};

use crate::brain_behaviors::Living;
use crate::control::MoveControl;
use crate::effects::MobEffect;
use crate::fluid::FluidFrame;
use crate::health::DamageResult;
use crate::look::BodyRotation;
use crate::monster_ai::Target;
use crate::navigation::GroundNavigation;
use crate::tempt::PlayerCandidate;
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::families::{Family, FamilyMob, Hurt, MobCore, Registered};
use crate::world::{EntityWorld, Rider, Voice};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::mth::{RAD_TO_DEG, atan2};
use minecraftoss_player::path_type::PathType;

pub const KIND: &str = "minecraft:breeze";
pub const CHARGE: &str = "minecraft:breeze_wind_charge";
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 1.77;
pub const EYE_HEIGHT: f32 = 1.3452;
pub const MAX_HEALTH: f32 = 30.0;
pub const MOVEMENT_SPEED: f64 = 0.63;
pub const FOLLOW_RANGE: f64 = 24.0;
pub const ATTACK_DAMAGE: f32 = 3.0;
pub const EXPERIENCE: i32 = 10;
pub const MAX_HEAD_Y_ROT: f32 = 30.0;
pub const FALL_SOUND_DISTANCE: f64 = 3.0;
pub const DEFLECT_SOUND: &str = "entity.breeze.deflect";
pub const IDLE_GROUND_SOUND: &str = "entity.breeze.idle_ground";
pub const IDLE_AIR_SOUND: &str = "entity.breeze.idle_air";

#[derive(Clone, Default)]
pub struct Breeze {
    pub brain: Brain,
    pub navigation: GroundNavigation,
    pub move_control: MoveControl,
    pub body_rotation: BodyRotation,
    pub speed: f32,
    pub forward: f32,
    pub sideways: f32,
    pub no_jump_delay: i32,
    pub last_hurt: Option<(i32, Option<Target>)>,
    pub shots: Vec<Shot>,
    pub discard_friction: bool,
    pub animations: Animations,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Animations {
    pub pose: Pose,
    pub idle: Option<i32>,
    pub shoot: Option<i32>,
    pub slide: Option<i32>,
    pub slide_back: Option<i32>,
    pub inhale: Option<i32>,
    pub long_jump: Option<i32>,
}

impl Animations {
    pub fn tick(&mut self, pose: Pose, tick_count: i32) {
        if pose != self.pose {
            let changed = tick_count - 1;
            self.shoot = None;
            self.idle = None;
            self.inhale = None;
            self.long_jump = None;
            match pose {
                Pose::Shooting => self.shoot = Some(changed),
                Pose::Inhaling => self.inhale = Some(changed),
                Pose::Sliding => {
                    self.slide.get_or_insert(changed);
                }
                Pose::Standing | Pose::LongJumping => {}
            }
            self.pose = pose;
        }
        if pose == Pose::LongJumping {
            self.long_jump.get_or_insert(tick_count);
        }
        self.idle.get_or_insert(tick_count);
        if pose != Pose::Sliding && self.slide.is_some() {
            self.slide_back = Some(tick_count);
            self.slide = None;
        }
    }

    pub fn starts(&self) -> [Option<i32>; 6] {
        [
            self.idle,
            self.shoot,
            self.slide,
            self.slide_back,
            self.inhale,
            self.long_jump,
        ]
    }

    pub fn set_starts(&mut self, starts: [Option<i32>; 6]) {
        [
            self.idle,
            self.shoot,
            self.slide,
            self.slide_back,
            self.inhale,
            self.long_jump,
        ] = starts;
    }
}

pub fn walk_profile() -> WalkProfile {
    let mut profile = WalkProfile::animal(WIDTH, HEIGHT);
    profile.clear_malus(PathType::FireInNeighbor);
    profile.set_malus(PathType::OnTopOfTrapdoor, -1.0);
    profile.set_malus(PathType::Fire, -1.0);
    profile.can_float = false;
    profile.follow_range = FOLLOW_RANGE as f32;
    profile.walk_target = WalkTarget::Monster;
    profile
}

pub fn take_shots(world: &mut EntityWorld) -> Vec<(u64, Shot)> {
    let mut out = Vec::new();
    for mob in &mut Breeze::store_mut(world).mobs {
        let id = mob.core.id;
        out.extend(mob.state.shots.drain(..).map(|shot| (id, shot)));
    }
    out
}

fn living(world: &EntityWorld, players: &[PlayerCandidate]) -> Vec<Living> {
    let mut out: Vec<Living> = players
        .iter()
        .map(|p| Living {
            target: Target::Player(p.id),
            kind: "minecraft:player",
            position: p.position,
            eye_height: p.eye_height,
            width: 0.6,
            height: 1.8,
            alive: p.alive,
            baby: false,
            attackable: p.attackable,
            spectator: p.spectator,
        })
        .collect();
    out.extend(
        world
            .mob_candidates()
            .into_iter()
            .filter(|m| m.kind == "minecraft:iron_golem")
            .map(|m| Living {
                target: Target::Mob(m.id),
                kind: m.kind,
                position: m.position,
                eye_height: m.eye_height,
                width: m.width,
                height: m.height,
                alive: m.alive,
                baby: m.baby,
                attackable: true,
                spectator: false,
            }),
    );
    out
}

fn head_yaws(world: &EntityWorld) -> Vec<(Target, f32)> {
    let mut out: Vec<(Target, f32)> = world
        .player_views
        .iter()
        .map(|(id, view)| (Target::Player(*id), view.head_yaw))
        .collect();
    out.extend(
        world
            .iron_golems
            .iter()
            .map(|e| (Target::Mob(e.id), e.ai.state.look_control.head_yaw)),
    );
    out
}

fn look_at(core: &mut MobCore, at: DVec3) {
    let eye = core.body.position + DVec3::Y * f64::from(core.eye_height);
    let delta = at - eye;
    let horizontal = (delta.x * delta.x + delta.z * delta.z).sqrt();
    let pitch = crate::wither::wrap_degrees(-(atan2(delta.y, horizontal) * RAD_TO_DEG) as f32);
    let yaw = crate::wither::wrap_degrees((atan2(delta.z, delta.x) * RAD_TO_DEG) as f32 - 90.0);
    core.yaw = yaw;
    core.look.head_yaw = yaw;
    core.look.pitch = pitch;
}

fn step<W: World>(
    world: &mut EntityWorld,
    id: u64,
    level: &W,
    players: &[PlayerCandidate],
    now: i64,
) {
    let others = living(world, players);
    let yaws = head_yaws(world);
    let difficulty = world.difficulty;
    let passenger = world.vehicle_of(Rider::Mob(id)).is_some();
    let sounds = crate::world::families::movement_sounds::<Breeze>(KIND);
    let Some(mob) = world.family_mob_mut::<Breeze>(id) else {
        return;
    };
    let FamilyMob { core, state } = mob;
    core.body.trim_small_velocity();
    let position = core.body.position;
    crate::world::monster_idle_without_ai(
        level,
        players,
        position,
        core.eye_height,
        &mut core.no_action_time,
    );
    core.no_action_time += 1;
    let fluid = FluidFrame::sample(level, position, core.body.width, core.body.height);
    let profile = walk_profile();
    let actions = {
        let mut ctx = brain::Ctx {
            body: &core.body,
            eye_height: core.eye_height,
            others: &others,
            head_yaws: &yaws,
            world: level,
            time: now,
            tick_count: core.tick_count,
            difficulty,
            random: &mut core.random,
            navigation: &mut state.navigation,
            profile: &profile,
            fluid,
            passenger,
            levitating: core.effects.has(MobEffect::Levitation),
            last_hurt: state.last_hurt,
        };
        state.brain.tick(&mut ctx)
    };
    if let Some(at) = actions.face {
        look_at(core, at);
    }
    state.shots.extend(actions.shot);
    if let Some(discard) = actions.discard_friction {
        state.discard_friction = discard;
    }
    if let Some(leap) = actions.leap {
        core.yaw = state.body_rotation.body_yaw;
        core.body.velocity = leap;
    }
    for (event, volume, pitch) in actions.sounds {
        core.voices
            .push((Voice::Event(event, volume, pitch), core.body.position));
    }
    let (can_update, surface) =
        crate::navigation::ground_view(level, &core.body, fluid, profile.can_float);
    if let Some((target, speed)) = state.navigation.tick_in(
        level,
        position,
        can_update,
        surface,
        core.body.width,
        state.speed,
    ) {
        state.move_control.set_wanted_position(target, speed);
    }
    let obstacle_top = crate::control::obstacle_top(level, position);
    let control = state.move_control.tick(
        position,
        core.body.on_ground,
        core.yaw,
        state.speed,
        state.forward,
        state.sideways,
        core.effects.movement_speed(MOVEMENT_SPEED),
        core.body.width,
        core.body.step_height,
        obstacle_top,
        |_, _| true,
    );
    core.yaw = control.yaw;
    state.speed = control.speed;
    state.forward = control.forward;
    state.sideways = control.sideways;
    let jumping = control.jump || actions.jump;
    if state.no_jump_delay > 0 {
        state.no_jump_delay -= 1;
    }
    core.body.living_jump(
        level,
        fluid,
        jumping,
        &mut state.no_jump_delay,
        f64::from(EYE_HEIGHT),
    );
    core.look.tick_limited(
        core.body.position,
        core.eye_height,
        state.body_rotation.body_yaw,
        !state.navigation.is_done(),
        MAX_HEAD_Y_ROT,
    );
    let input = DVec3::new(f64::from(state.sideways), 0.0, f64::from(state.forward));
    if fluid.in_water() {
        core.body.travel_water(level, input, core.yaw);
    } else if fluid.in_lava() {
        core.body
            .travel_lava(level, input, core.yaw, fluid.lava_height);
    } else {
        let friction = if core.body.on_ground {
            crate::movement::ground_friction(level, core.body.on_pos(level, 0.500001))
        } else {
            1.0
        };
        let landed = core
            .body
            .travel_air_jumping(level, input, state.speed, core.yaw, jumping);
        if state.discard_friction {
            let drag = f64::from(friction * 0.91_f32);
            core.body.velocity.x /= drag;
            core.body.velocity.z /= drag;
            core.body.velocity.y /= f64::from(0.98_f32);
        }
        if landed.is_some_and(|fallen| fallen > FALL_SOUND_DISTANCE) {
            core.voices.push((
                Voice::Event(brain::LAND_SOUND, 1.0, 1.0),
                core.body.position,
            ));
        }
    }
    crate::world::emissions::play_movement(
        &mut core.body,
        core.tick_count,
        &mut core.random,
        &mut core.voices,
        sounds,
    );
    state.body_rotation.tick_limited(
        core.yaw,
        &mut core.look,
        core.previous_position,
        core.body.position,
        MAX_HEAD_Y_ROT,
    );
}

fn idle_voices(core: &mut MobCore) {
    let on_ground = core.body.on_ground;
    for (voice, _) in &mut core.voices {
        if matches!(voice, Voice::Ambient(_)) {
            let event = if on_ground {
                IDLE_GROUND_SOUND
            } else {
                IDLE_AIR_SOUND
            };
            *voice = Voice::Event(event, 1.0, 1.0);
        }
    }
}

impl Family for Breeze {
    const KEY: u8 = 40;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let state = Breeze {
            body_rotation: BodyRotation::new(core.yaw),
            ..Breeze::default()
        };
        Some(FamilyMob { core, state })
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
        if let Some(mob) = world.family_mob_mut::<Self>(id) {
            idle_voices(&mut mob.core);
            let pose = mob.state.brain.pose;
            mob.state.animations.tick(pose, mob.core.tick_count);
        }
        if base.alive && !base.no_ai {
            step(world, id, level, players, base.game_time);
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn hurt(mob: &mut FamilyMob<Self>, hurt: &Hurt) -> DamageResult {
        if hurt.attacker_kind == Some(KIND) {
            return DamageResult {
                applied: false,
                dealt: 0.0,
                died: false,
                full: false,
            };
        }
        let result = mob.core.hurt_plain(hurt.amount);
        if result.applied {
            mob.state.last_hurt = Some((mob.core.tick_count, hurt.attacker));
        }
        result
    }

    fn experience(_mob: &mut FamilyMob<Self>) -> i32 {
        EXPERIENCE
    }

    fn enemy(_kind: &str) -> bool {
        true
    }

    fn emits_sounds(_kind: &str) -> bool {
        false
    }

    fn deflect(mob: &mut FamilyMob<Self>, projectile: &str) -> bool {
        if matches!(projectile, "minecraft:wind_charge" | CHARGE) {
            return false;
        }
        let position = mob.core.body.position;
        mob.core
            .voices
            .push((Voice::Event(DEFLECT_SOUND, 1.0, 1.0), position));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{DamageSourceKind, PlayerAttack};
    use minecraftoss_player::{Block, Pos};
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Field {
        blocks: BTreeMap<Pos, Block>,
    }

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            if let Some(block) = self.blocks.get(&pos) {
                return Some(block.clone());
            }
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.blocks.insert(pos, block),
                None => self.blocks.remove(&pos),
            };
        }
    }

    fn player_at(id: u64, position: DVec3, attackable: bool) -> PlayerCandidate {
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
            attackable,
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

    fn at(x: f64, z: f64) -> DVec3 {
        DVec3::new(x, 64.0, z)
    }

    fn breeze_at(world: &mut EntityWorld, x: f64, z: f64) -> u64 {
        world.spawn_family_kind(KIND, at(x, z)).unwrap()
    }

    fn breeze(world: &EntityWorld, id: u64) -> &FamilyMob<Breeze> {
        world
            .family::<Breeze>()
            .iter()
            .find(|mob| mob.core.id == id)
            .unwrap()
    }

    fn breeze_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Breeze> {
        world.family_mob_mut::<Breeze>(id).unwrap()
    }

    fn first_shot(
        world: &mut EntityWorld,
        field: &mut Field,
        players: &[PlayerCandidate],
        ticks: usize,
    ) -> Option<(u64, Shot)> {
        for _ in 0..ticks {
            world.tick_with_players(field, players);
            if let Some(shot) = take_shots(world).into_iter().next() {
                return Some(shot);
            }
        }
        None
    }

    #[test]
    fn a_breeze_is_a_tall_thirty_health_monster_with_its_kind() {
        let mut world = EntityWorld::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        assert_eq!(world.entity_type(id), Some(KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(40));
        let core = &breeze(&world, id).core;
        assert_eq!(
            (core.body.width, core.body.height, core.eye_height),
            (0.6, 1.77, 1.3452)
        );
        assert_eq!((core.health, core.max_health), (30.0, 30.0));
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
        assert!(traits.enemy && !traits.undead && !traits.fire_immune);
        assert!(!traits.breathes_water && !traits.leashable);
        assert_eq!(traits.voice, ("breeze", 1.0, "hostile_volume"));
        assert!(
            world
                .player_attack(&attack(1, at(0.5, 1.5), 100.0), id)
                .died
        );
        let deaths = world.take_deaths();
        assert_eq!((deaths[0].id, deaths[0].table), (id, Some(KIND)));
        assert_eq!(deaths[0].experience, 10);
    }

    #[test]
    fn a_breeze_shrugs_off_what_another_breeze_does_and_takes_the_rest() {
        let mut world = EntityWorld::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        let other = breeze_at(&mut world, 4.5, 0.5);
        let from_breeze = Hurt::new(5.0, "minecraft:mob_attack", DamageSourceKind::MobAttack)
            .by(Some(Target::Mob(other)), Some(KIND));
        assert!(!world.hurt_family(id, &from_breeze).unwrap().applied);
        let charge = world.mob_shot_mob(other, id, 1.0, DVec3::new(-0.7, 0.0, 0.0));
        assert!(!charge, "another breeze's charge does not hurt it");
        let core = &breeze(&world, id).core;
        assert_eq!(core.health, MAX_HEALTH);
        assert!(core.voices.is_empty(), "no hurt sound");
        assert_eq!(breeze(&world, id).state.last_hurt, None);
        let from_player = Hurt::new(
            5.0,
            "minecraft:player_attack",
            DamageSourceKind::PlayerAttack,
        )
        .by(Some(Target::Player(1)), Some("minecraft:player"));
        assert!(world.hurt_family(id, &from_player).unwrap().applied);
        let mob = breeze(&world, id);
        assert_eq!(mob.core.health, MAX_HEALTH - 5.0);
        assert_eq!(
            mob.state.last_hurt,
            Some((mob.core.tick_count, Some(Target::Player(1))))
        );
    }

    #[test]
    fn a_breeze_lands_from_a_long_fall_without_hurting() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        breeze_mut(&mut world, id).core.body.position = DVec3::new(0.5, 84.0, 0.5);
        for _ in 0..60 {
            world.tick_with_players(&mut field, &[]);
        }
        let core = &breeze(&world, id).core;
        assert!(core.body.on_ground && core.body.position.y < 64.5);
        assert_eq!(core.health, MAX_HEALTH);
    }

    #[test]
    fn a_breeze_targets_survival_players_and_iron_golems_only() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        world.spawn_zombie(crate::zombie::Zombie::new(at(2.5, 0.5)), true);
        let creative = [player_at(1, at(0.5, 3.5), false)];
        for _ in 0..60 {
            world.tick_with_players(&mut field, &creative);
        }
        assert_eq!(breeze(&world, id).state.brain.attack_target, None);
        let golem = world.spawn_iron_golem(crate::iron_golem::IronGolem::new(at(0.5, -6.5)), true);
        for _ in 0..30 {
            world.tick_with_players(&mut field, &creative);
        }
        assert_eq!(
            breeze(&world, id).state.brain.attack_target,
            Some(Target::Mob(golem))
        );
        let mut fresh = EntityWorld::default();
        let other = breeze_at(&mut fresh, 0.5, 0.5);
        let survival = [player_at(2, at(0.5, 10.5), true)];
        for _ in 0..30 {
            fresh.tick_with_players(&mut field, &survival);
        }
        assert_eq!(
            breeze(&fresh, other).state.brain.attack_target,
            Some(Target::Player(2))
        );
    }

    #[test]
    fn a_breeze_slides_or_jumps_then_shoots_its_charge_at_a_player_within_sixteen_blocks() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        let players = [player_at(1, at(0.5, 12.5), true)];
        let (mut sounds, mut slid, mut shot) = (Vec::new(), false, None);
        for _ in 0..400 {
            world.tick_with_players(&mut field, &players);
            slid |= matches!(
                breeze(&world, id).state.brain.pose,
                Pose::Sliding | Pose::LongJumping
            );
            sounds.extend(
                world
                    .take_sounds()
                    .into_iter()
                    .map(|s| (s.event, s.volume, s.category)),
            );
            shot = take_shots(&mut world).into_iter().next();
            if shot.is_some() {
                break;
            }
        }
        assert!(slid, "it slides or jumps before it shoots");
        let (shooter, shot) = shot.expect("it shoots");
        assert_eq!(shooter, id);
        let core = &breeze(&world, id).core;
        let feet = core.body.position;
        let firing = feet.y + f64::from(HEIGHT / 2.0) + f64::from(0.3_f32);
        assert_eq!(shot.from, DVec3::new(feet.x, firing, feet.z));
        assert!(feet.distance_squared(players[0].position) < 256.0);
        let aim = players[0].position + DVec3::Y * (f64::from(1.8_f32) * 0.3) - shot.from;
        assert!((shot.direction - aim).length() < 1.0e-9, "{shot:?}");
        assert_eq!((shot.power, shot.uncertainty), (0.7, -3.0));
        assert_eq!(breeze(&world, id).state.brain.pose, Pose::Shooting);
        let inhale = sounds.iter().position(|s| s.0 == "entity.breeze.inhale");
        let shoot = sounds.iter().position(|s| s.0 == "entity.breeze.shoot");
        assert!(
            inhale.is_some() && shoot.is_some() && inhale < shoot,
            "{sounds:?}"
        );
        let shoot = &sounds[shoot.unwrap()];
        assert_eq!((shoot.1, shoot.2), (1.5, "hostile_volume"));
        let yaw = (atan2(aim.z, aim.x) * RAD_TO_DEG) as f32 - 90.0;
        assert!(
            crate::wither::wrap_degrees(core.yaw - yaw).abs() < 1.0,
            "it faces the player"
        );
        let mut more = 0;
        for _ in 0..12 {
            world.tick_with_players(&mut field, &players);
            more += take_shots(&mut world).len();
        }
        assert_eq!(more, 0, "one charge per shot, then a cooldown");
    }

    #[test]
    fn a_breeze_charge_spreads_with_the_difficulty() {
        let mut field = Field::default();
        for (difficulty, uncertainty) in [(1, 1.0), (2, -3.0), (3, -7.0)] {
            let mut world = EntityWorld::default();
            world.set_difficulty(difficulty);
            breeze_at(&mut world, 0.5, 0.5);
            let players = [player_at(1, at(0.5, 10.5), true)];
            let shot = first_shot(&mut world, &mut field, &players, 400);
            assert_eq!(
                shot.map(|(_, s)| (s.uncertainty, s.power)),
                Some((uncertainty, 0.7)),
                "difficulty {difficulty}"
            );
        }
    }

    #[test]
    fn a_breeze_in_water_shoots_without_sliding_first() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        for x in -3..=3 {
            for z in -3..=3 {
                for y in 64..67 {
                    field.set_block((x, y, z), Some(Block::new("minecraft:water")));
                }
            }
        }
        let id = breeze_at(&mut world, 0.5, 0.5);
        let players = [player_at(1, at(0.5, 9.5), true)];
        let mut shot = None;
        for _ in 0..200 {
            world.tick_with_players(&mut field, &players);
            let brain = &breeze(&world, id).state.brain;
            if brain.attack_target.is_some() && !brain.walk_target.present() {
                assert!(!brain.slide.running(), "no slide from the water");
            }
            shot = take_shots(&mut world).into_iter().next();
            if shot.is_some() {
                break;
            }
        }
        assert!(shot.is_some(), "stuck in water it shoots");
    }

    #[test]
    fn a_breeze_forgets_a_player_who_turns_creative() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        let near = [player_at(1, at(0.5, 8.5), true)];
        for _ in 0..30 {
            world.tick_with_players(&mut field, &near);
        }
        assert_eq!(
            breeze(&world, id).state.brain.attack_target,
            Some(Target::Player(1))
        );
        let creative = [player_at(1, at(0.5, 8.5), false)];
        let mut forgot = false;
        for _ in 0..60 {
            world.tick_with_players(&mut field, &creative);
            if breeze(&world, id).state.brain.attack_target.is_none() {
                forgot = true;
                break;
            }
        }
        assert!(forgot);
    }

    #[test]
    fn a_breeze_long_jumps_behind_its_target_and_lands_without_hurting() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        let players = [player_at(1, at(0.5, 10.5), true)];
        breeze_mut(&mut world, id).state.brain.attack_target = Some(Target::Player(1));
        let (mut poses, mut sounds, mut leap) = (Vec::new(), Vec::new(), None);
        for _ in 0..120 {
            world.tick_with_players(&mut field, &players);
            let mob = breeze(&world, id);
            if poses.last() != Some(&mob.state.brain.pose) {
                poses.push(mob.state.brain.pose);
            }
            if leap.is_none() && mob.state.brain.pose == Pose::LongJumping {
                leap = Some((mob.core.body.velocity, mob.state.discard_friction));
            }
            sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
            if poses.ends_with(&[Pose::LongJumping, Pose::Standing]) {
                break;
            }
        }
        assert!(
            poses.ends_with(&[Pose::Inhaling, Pose::LongJumping, Pose::Standing]),
            "{poses:?}"
        );
        let (velocity, discard) = leap.unwrap();
        assert!(discard, "no friction in the air");
        assert!(velocity.length() <= f64::from(brain::MAX_JUMP_VELOCITY) && velocity.z > 0.0);
        let order: Vec<&str> = sounds
            .iter()
            .map(String::as_str)
            .filter(|s| s.contains("charge") || s.contains("jump") || s.contains("land"))
            .collect();
        assert_eq!(
            order[..3],
            [
                "entity.breeze.charge",
                "entity.breeze.jump",
                "entity.breeze.land"
            ]
        );
        let mob = breeze(&world, id);
        assert!(!mob.state.discard_friction);
        assert_eq!(mob.core.health, MAX_HEALTH);
        assert!(mob.state.brain.jump_cooldown.present() && mob.state.brain.shoot.present());
        let landed = mob.core.body.position;
        assert!(
            landed.z > 2.0 && landed.distance(players[0].position) <= 9.0,
            "{landed}"
        );
    }

    #[test]
    fn a_breeze_under_a_low_ceiling_slides_instead_of_jumping() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        field.set_block((0, 67, 0), Some(Block::new("minecraft:stone")));
        let id = breeze_at(&mut world, 0.5, 0.5);
        let players = [player_at(1, at(0.5, 10.5), true)];
        breeze_mut(&mut world, id).state.brain.attack_target = Some(Target::Player(1));
        for _ in 0..3 {
            world.tick_with_players(&mut field, &players);
            assert_ne!(breeze(&world, id).state.brain.pose, Pose::Inhaling);
        }
        assert!(breeze(&world, id).state.brain.walk_target.present());
    }

    #[test]
    fn a_breeze_forgets_a_target_beyond_its_follow_range() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        let near = [player_at(1, at(0.5, 8.5), true)];
        for _ in 0..30 {
            world.tick_with_players(&mut field, &near);
        }
        assert_eq!(
            breeze(&world, id).state.brain.attack_target,
            Some(Target::Player(1))
        );
        let far = [player_at(1, at(0.5, 40.5), true)];
        let mut forgot = false;
        for _ in 0..60 {
            world.tick_with_players(&mut field, &far);
            if breeze(&world, id).state.brain.attack_target.is_none() {
                forgot = true;
                break;
            }
        }
        assert!(
            forgot,
            "sooner than the 100 ticks a target out of sight is kept"
        );
    }

    #[test]
    fn a_breeze_turns_arrows_back_with_its_sound_but_not_wind_charges() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        breeze_mut(&mut world, id).core.no_ai = true;
        assert!(!world.deflects_projectile(id, "minecraft:wind_charge"));
        assert!(!world.deflects_projectile(id, CHARGE));
        assert!(breeze(&world, id).core.voices.is_empty());
        let arrow = crate::projectile::Arrow::in_flight(
            DVec3::new(0.5, 65.0, -2.5),
            DVec3::Z,
            minecraftoss_player::rng::LegacyRandom::new(0),
        );
        let arrow = world.spawn_owned_arrow(0, arrow);
        let mut sounds = Vec::new();
        for _ in 0..6 {
            world.tick_with_players(&mut field, &[]);
            sounds.extend(world.take_sounds());
        }
        assert_eq!(breeze(&world, id).core.health, MAX_HEALTH);
        let deflect = sounds
            .iter()
            .find(|s| s.event == DEFLECT_SOUND)
            .expect("the deflect sound");
        assert_eq!(
            (deflect.volume, deflect.pitch, deflect.category),
            (1.0, 1.0, "hostile_volume")
        );
        let turned = &world.arrows.iter().find(|e| e.id == arrow).unwrap().arrow;
        assert!(turned.velocity.z < 0.0);
    }

    #[test]
    fn the_animations_follow_the_pose_like_vanillas_client() {
        let mut animations = Animations::default();
        animations.tick(Pose::Standing, 1);
        animations.tick(Pose::Standing, 2);
        assert_eq!(animations.starts(), [Some(1), None, None, None, None, None]);
        animations.tick(Pose::Sliding, 10);
        assert_eq!(
            animations.starts(),
            [Some(10), None, Some(9), None, None, None]
        );
        animations.tick(Pose::Standing, 30);
        assert_eq!(
            animations.starts(),
            [Some(30), None, None, Some(30), None, None]
        );
        animations.tick(Pose::Shooting, 40);
        assert_eq!(
            animations.starts(),
            [Some(40), Some(39), None, Some(30), None, None]
        );
        animations.tick(Pose::Inhaling, 60);
        assert_eq!(
            animations.starts(),
            [Some(60), None, None, Some(30), Some(59), None]
        );
        animations.tick(Pose::LongJumping, 70);
        animations.tick(Pose::LongJumping, 71);
        assert_eq!(
            animations.starts(),
            [Some(70), None, None, Some(30), None, Some(70)]
        );
        animations.tick(Pose::Standing, 90);
        assert_eq!(
            animations.starts(),
            [Some(90), None, None, Some(30), None, None]
        );
        assert_eq!(animations.pose, Pose::Standing);
        let mut copy = Animations::default();
        copy.set_starts(animations.starts());
        assert_eq!(copy.starts(), animations.starts());
    }

    #[test]
    fn a_breeze_animates_each_pose_a_tick_after_it_takes_it() {
        let mut world = EntityWorld::default();
        let mut field = Field::default();
        let id = breeze_at(&mut world, 0.5, 0.5);
        let players = [player_at(1, at(0.5, 10.5), true)];
        let (mut poses, mut seen) = (vec![Pose::Standing], Vec::new());
        for _ in 0..400 {
            world.tick_with_players(&mut field, &players);
            take_shots(&mut world);
            let mob = breeze(&world, id);
            let (now, animations) = (mob.core.tick_count, mob.state.animations);
            let shown = poses[poses.len() - 1];
            assert_eq!(animations.pose, shown);
            if poses.len() >= 2 && poses[poses.len() - 2] != shown {
                let started = match shown {
                    Pose::Shooting => animations.shoot,
                    Pose::Inhaling => animations.inhale,
                    Pose::Sliding => animations.slide,
                    Pose::LongJumping => animations.long_jump.map(|start| start - 1),
                    Pose::Standing => Some(now - 1),
                };
                assert_eq!((started, animations.idle), (Some(now - 1), Some(now)));
                if poses[poses.len() - 2] == Pose::Sliding {
                    assert_eq!((animations.slide, animations.slide_back), (None, Some(now)));
                }
                seen.push(shown);
            }
            poses.push(mob.state.brain.pose);
        }
        assert!(seen.contains(&Pose::Shooting), "{seen:?}");
        assert!(
            seen.contains(&Pose::Sliding) || seen.contains(&Pose::LongJumping),
            "{seen:?}"
        );
    }
}
