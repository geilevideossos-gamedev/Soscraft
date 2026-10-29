use crate::brain_behaviors::{Living, MoveSink, Track, WalkGoal, Walker, block_of, find, shuffled};
use crate::fluid::FluidFrame;
use crate::monster_ai::Target;
use crate::movement::Body;
use crate::navigation::GroundNavigation;
use crate::villager_brain::Slot;
use crate::walk_path::WalkProfile;
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::rng::LegacyRandom;

pub const SCAN_RATE: u32 = 20;
pub const TARGETING_RANGE: f64 = 16.0;
pub const SHOOT_RANGE_SQUARED: f64 = 256.0;
pub const SHOOT_CHARGING_TICKS: i64 = 15;
pub const SHOOT_RECOVERING_TICKS: i64 = 4;
pub const SHOOT_COOLDOWN_TICKS: i64 = 10;
pub const SHOOT_DURATION: i64 = SHOOT_CHARGING_TICKS + 1 + SHOOT_RECOVERING_TICKS;
pub const SHOOT_AFTER_SLIDE_TICKS: i64 = 60;
pub const SHOOT_WHEN_STUCK_TICKS: i64 = 60;
pub const CHARGE_POWER: f32 = 0.7;
pub const UNCERTAINTY_BASE: i32 = 5;
pub const UNCERTAINTY_MULTIPLIER: i32 = 4;
pub const SLIDE_SPEED: f32 = 0.6;
pub const STROLL_SPEED: f32 = 0.6;
pub const SLIDE_MIN_TICKS: i64 = 20;
pub const SLIDE_MAX_TICKS: i64 = 40;
pub const IDLE_MIN_TICKS: i64 = 20;
pub const IDLE_MAX_TICKS: i64 = 100;
pub const INNER_CIRCLE: f64 = 4.0;
pub const INNER_CIRCLE_HEIGHT: f64 = 10.0;
pub const MIDDLE_CIRCLE: f64 = 8.0;
pub const ATTACKABLE_MEMORY: i32 = 100;
pub const GIVE_UP_REACHING: i64 = 200;
pub const HURT_MEMORY_TICKS: i32 = 40;
pub const SWIM_CHANCE: f32 = 0.8;
pub const SIGHT_RANGE: f64 = 50.0;
pub const INHALE_SOUND: &str = "entity.breeze.inhale";
pub const SHOOT_SOUND: &str = "entity.breeze.shoot";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pose {
    #[default]
    Standing,
    Shooting,
    Inhaling,
    Sliding,
    LongJumping,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activity {
    Idle,
    #[default]
    Fight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub from: DVec3,
    pub direction: DVec3,
    pub power: f32,
    pub uncertainty: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Actions {
    pub face: Option<DVec3>,
    pub shot: Option<Shot>,
    pub jump: bool,
    pub sounds: Vec<(&'static str, f32, f32)>,
}

pub struct Ctx<'a> {
    pub body: &'a Body,
    pub eye_height: f32,
    pub others: &'a [Living],
    pub head_yaws: &'a [(Target, f32)],
    pub world: &'a dyn World,
    pub time: i64,
    pub tick_count: i32,
    pub difficulty: i32,
    pub random: &'a mut LegacyRandom,
    pub navigation: &'a mut GroundNavigation,
    pub profile: &'a WalkProfile,
    pub fluid: FluidFrame,
    pub passenger: bool,
    pub levitating: bool,
    pub last_hurt: Option<(i32, Option<Target>)>,
}

impl Ctx<'_> {
    pub fn position(&self) -> DVec3 {
        self.body.position
    }

    pub fn eye(&self) -> DVec3 {
        self.body.position + DVec3::Y * f64::from(self.eye_height)
    }

    fn walker(&mut self) -> Walker<'_> {
        Walker {
            body: self.body,
            others: self.others,
            world: self.world,
            time: self.time,
            random: self.random,
            navigation: self.navigation,
            profile: self.profile,
            fluid: self.fluid,
        }
    }

    fn deep_in_water(&self) -> bool {
        self.fluid.in_water() && self.fluid.water_height > f64::from(self.eye_height)
    }
}

pub fn can_attack(other: &Living) -> bool {
    other.alive
        && !other.spectator
        && other.attackable
        && (other.is_player() || other.kind == "minecraft:iron_golem")
}

fn attackable(ctx: &Ctx, other: &Living, sight: bool) -> bool {
    can_attack(other)
        && ctx.position().distance_squared(other.position) <= TARGETING_RANGE * TARGETING_RANGE
        && (!sight || crate::sight::line_of_sight(ctx.world, ctx.eye(), other.eye()))
}

pub fn within_inner_circle(position: DVec3, at: DVec3) -> bool {
    let centre = block_of(position);
    let centre = DVec3::new(
        f64::from(centre.0) + 0.5,
        f64::from(centre.1) + 0.5,
        f64::from(centre.2) + 0.5,
    );
    let (dx, dz) = (at.x - centre.x, at.z - centre.z);
    dx * dx + dz * dz < INNER_CIRCLE * INNER_CIRCLE && (at.y - centre.y).abs() < INNER_CIRCLE_HEIGHT
}

pub fn has_line_of_sight(world: &dyn World, from: DVec3, to: DVec3) -> bool {
    from.distance(to) <= SIGHT_RANGE && !crate::sight::blocked(world, from, to)
}

pub fn point_behind(position: DVec3, head_yaw: f32, random: &mut LegacyRandom) -> DVec3 {
    let yaw = head_yaw + 180.0 + random.next_gaussian() as f32 * 90.0 / 2.0;
    let distance = 4.0 + random.next_float() * (8.0 - 4.0);
    let angle = f64::from(-yaw * 0.017453292 - std::f32::consts::PI);
    let (sin, cos) = (
        minecraftoss_player::mth::sin(angle),
        minecraftoss_player::mth::cos(angle),
    );
    let pitch = -minecraftoss_player::mth::cos(0.0);
    let direction = DVec3::new(f64::from(sin * pitch), 0.0, f64::from(cos * pitch));
    direction * f64::from(distance) + position
}

pub fn point_in_middle_circle(position: DVec3, target: DVec3, random: &mut LegacyRandom) -> DVec3 {
    let towards = target - position;
    let scale =
        towards.length() - (MIDDLE_CIRCLE + random.next_double() * (INNER_CIRCLE - MIDDLE_CIRCLE));
    crate::enderman::normalize(towards) * scale + position
}

#[derive(Clone, Debug, Default)]
pub struct Brain {
    pub activity: Activity,
    pub pose: Pose,
    started: bool,
    sensors: [i64; 2],
    pub attack_target: Option<Target>,
    pub nearest_attackable: Option<Target>,
    pub hurt_by: Option<Option<Target>>,
    pub walk_target: Slot<WalkGoal>,
    pub cant_reach_walk_target_since: Slot<i64>,
    pub shoot: Slot<()>,
    pub shoot_cooldown: Slot<()>,
    pub shoot_charging: Slot<()>,
    pub shoot_recovering: Slot<()>,
    attackable_for: i32,
    shooting_until: Option<i64>,
    pub slide: MoveSink,
    idle_until: Option<i64>,
}

impl Brain {
    pub fn tick(&mut self, ctx: &mut Ctx) -> Actions {
        let mut out = Actions::default();
        if !self.started {
            self.started = true;
            for clock in &mut self.sensors {
                *clock = i64::from(ctx.random.next_int(SCAN_RATE));
            }
        }
        self.forget();
        self.sense(ctx);
        self.start(ctx, &mut out);
        self.run(ctx, &mut out);
        self.activity = if self.attack_target.is_some() && !self.walk_target.present() {
            Activity::Fight
        } else {
            Activity::Idle
        };
        out
    }

    pub fn shooting(&self) -> bool {
        self.shooting_until.is_some()
    }

    fn forget(&mut self) {
        self.walk_target.tick();
        self.cant_reach_walk_target_since.tick();
        self.shoot.tick();
        self.shoot_cooldown.tick();
        self.shoot_charging.tick();
        self.shoot_recovering.tick();
    }

    fn sense(&mut self, ctx: &mut Ctx) {
        let mut fired = [false; 2];
        for (clock, fired) in self.sensors.iter_mut().zip(fired.iter_mut()) {
            *clock -= 1;
            if *clock <= 0 {
                *clock = i64::from(SCAN_RATE);
                *fired = true;
            }
        }
        if fired[0] {
            let position = ctx.position();
            let mut found: Vec<&Living> = ctx.others.iter().collect();
            found.sort_by(|a, b| {
                let (a, b) = (
                    a.position.distance_squared(position),
                    b.position.distance_squared(position),
                );
                a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
            });
            self.nearest_attackable = found
                .into_iter()
                .find(|other| attackable(ctx, other, self.attack_target != Some(other.target)))
                .map(|other| other.target);
        }
        if fired[1] {
            self.hurt_by = ctx
                .last_hurt
                .filter(|(at, _)| ctx.tick_count - at <= HURT_MEMORY_TICKS)
                .map(|(_, attacker)| attacker);
        }
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Actions) {
        match self.activity {
            Activity::Fight => self.stop_attacking_if_invalid(ctx),
            Activity::Idle => self.start_attacking(ctx, self.nearest_attackable),
        }
        match self.activity {
            Activity::Fight => self.start_shoot(ctx, out),
            Activity::Idle => self.start_attacking(ctx, self.hurt_by.flatten()),
        }
        if self.activity == Activity::Idle {
            self.start_slide_to_target(ctx);
        }
        match self.activity {
            Activity::Fight => self.shoot_when_stuck(ctx),
            Activity::Idle => self.idle(ctx),
        }
        if self.activity == Activity::Fight {
            self.start_slide(ctx);
        }
    }

    fn run(&mut self, ctx: &mut Ctx, out: &mut Actions) {
        if (ctx.deep_in_water() || ctx.fluid.in_lava()) && ctx.random.next_float() < SWIM_CHANCE {
            out.jump = true;
        }
        if self.slide.running() {
            self.slide.tick(
                &mut self.walk_target,
                &mut self.cant_reach_walk_target_since,
                &mut ctx.walker(),
            );
            if !self.slide.running() {
                self.pose = Pose::Standing;
                if self.attack_target.is_some() {
                    self.shoot.set_for((), SHOOT_AFTER_SLIDE_TICKS);
                }
            }
        }
        if let Some(end) = self.shooting_until {
            if ctx.time > end || self.attack_target.is_none() || !self.shoot.present() {
                self.stop_shoot();
            } else {
                self.tick_shoot(ctx, out);
            }
        }
        if self.idle_until.is_some_and(|end| ctx.time > end) {
            self.idle_until = None;
        }
    }

    fn start_attacking(&mut self, ctx: &mut Ctx, found: Option<Target>) {
        if self.attack_target.is_some() {
            return;
        }
        let Some(target) = found else {
            return;
        };
        if !find(ctx.others, target).is_some_and(can_attack) {
            return;
        }
        self.attack_target = Some(target);
        self.attackable_for = 0;
        self.cant_reach_walk_target_since.erase();
    }

    fn stop_attacking_if_invalid(&mut self, ctx: &mut Ctx) {
        let Some(target) = self.attack_target else {
            return;
        };
        let invalid = match find(ctx.others, target) {
            None => true,
            Some(other) if !can_attack(other) => true,
            Some(_)
                if self
                    .cant_reach_walk_target_since
                    .get()
                    .is_some_and(|since| ctx.time - since > GIVE_UP_REACHING) =>
            {
                true
            }
            Some(other) => {
                if attackable(ctx, other, false) {
                    self.attackable_for = ATTACKABLE_MEMORY;
                    false
                } else {
                    self.attackable_for -= 1;
                    self.attackable_for < 0
                }
            }
        };
        if invalid {
            self.attack_target = None;
        }
    }

    fn start_shoot(&mut self, ctx: &mut Ctx, out: &mut Actions) {
        if self.shooting_until.is_some()
            || self.shoot_cooldown.present()
            || self.shoot_charging.present()
            || self.shoot_recovering.present()
            || !self.shoot.present()
            || self.walk_target.present()
            || self.pose != Pose::Standing
        {
            return;
        }
        let Some(target) = self
            .attack_target
            .and_then(|target| find(ctx.others, target))
        else {
            return;
        };
        if ctx.position().distance_squared(target.position) >= SHOOT_RANGE_SQUARED {
            self.shoot.erase();
            return;
        }
        self.shooting_until = Some(ctx.time + SHOOT_DURATION);
        self.pose = Pose::Shooting;
        self.shoot_charging.set_for((), SHOOT_CHARGING_TICKS);
        out.sounds.push((INHALE_SOUND, 1.0, 1.0));
    }

    fn tick_shoot(&mut self, ctx: &mut Ctx, out: &mut Actions) {
        let Some(target) = self
            .attack_target
            .and_then(|target| find(ctx.others, target))
        else {
            return;
        };
        out.face = Some(target.position);
        if self.shoot_charging.present() || self.shoot_recovering.present() {
            return;
        }
        self.shoot_recovering.set_for((), SHOOT_RECOVERING_TICKS);
        let position = ctx.position();
        let firing = position.y + f64::from(ctx.body.height / 2.0) + f64::from(0.3_f32);
        let aim = target.position.y + f64::from(target.height) * 0.3;
        out.shot = Some(Shot {
            from: DVec3::new(position.x, firing, position.z),
            direction: DVec3::new(
                target.position.x - position.x,
                aim - firing,
                target.position.z - position.z,
            ),
            power: CHARGE_POWER,
            uncertainty: (UNCERTAINTY_BASE - ctx.difficulty * UNCERTAINTY_MULTIPLIER) as f32,
        });
        out.sounds.push((SHOOT_SOUND, 1.5, 1.0));
    }

    fn stop_shoot(&mut self) {
        self.shooting_until = None;
        if self.pose == Pose::Shooting {
            self.pose = Pose::Standing;
        }
        self.shoot_cooldown.set_for((), SHOOT_COOLDOWN_TICKS);
        self.shoot.erase();
    }

    fn shoot_when_stuck(&mut self, ctx: &mut Ctx) {
        if self.attack_target.is_none() || self.walk_target.present() || self.shoot.present() {
            return;
        }
        if ctx.passenger || ctx.fluid.in_water() || ctx.levitating {
            self.shoot.set_for((), SHOOT_WHEN_STUCK_TICKS);
        }
    }

    fn start_slide(&mut self, ctx: &mut Ctx) {
        if self.walk_target.present()
            || self.shoot.present()
            || !ctx.body.on_ground
            || ctx.fluid.in_water()
            || self.pose != Pose::Standing
        {
            return;
        }
        let Some(target) = self
            .attack_target
            .and_then(|target| find(ctx.others, target))
        else {
            return;
        };
        let target = *target;
        let position = ctx.position();
        let mut spot = None;
        if within_inner_circle(position, target.position) {
            let away = crate::stroll::default_random_position_away(
                ctx.world,
                ctx.profile,
                position,
                5,
                5,
                target.position,
                ctx.random,
            );
            spot = away.filter(|away| {
                has_line_of_sight(ctx.world, position, *away)
                    && target.position.distance_squared(*away)
                        > target.position.distance_squared(position)
            });
        }
        let spot = spot.unwrap_or_else(|| {
            if ctx.random.next_boolean() {
                let yaw = ctx
                    .head_yaws
                    .iter()
                    .find(|(id, _)| *id == target.target)
                    .map_or(0.0, |(_, yaw)| *yaw);
                point_behind(target.position, yaw, ctx.random)
            } else {
                point_in_middle_circle(position, target.position, ctx.random)
            }
        });
        self.walk_target.set(WalkGoal {
            target: Track::Block(block_of(spot)),
            speed: SLIDE_SPEED,
            close_enough: 1,
        });
    }

    fn start_slide_to_target(&mut self, ctx: &mut Ctx) {
        if self.slide.running() {
            return;
        }
        self.slide.try_start_for(
            &mut self.walk_target,
            &mut self.cant_reach_walk_target_since,
            &mut ctx.walker(),
            SLIDE_MIN_TICKS,
            SLIDE_MAX_TICKS,
        );
        if self.slide.running() {
            self.pose = Pose::Sliding;
        }
    }

    fn idle(&mut self, ctx: &mut Ctx) {
        if self.idle_until.is_some() {
            return;
        }
        for choice in shuffled(ctx.random, &[1, 2]) {
            if choice == 0 {
                let spread = (IDLE_MAX_TICKS - IDLE_MIN_TICKS + 1) as u32;
                self.idle_until =
                    Some(ctx.time + IDLE_MIN_TICKS + i64::from(ctx.random.next_int(spread)));
                return;
            }
            if self.walk_target.present() {
                continue;
            }
            let spot = crate::stroll::land_random_position(
                ctx.world,
                ctx.profile,
                ctx.position(),
                10,
                7,
                ctx.random,
            );
            self.walk_target.set_or_erase(spot.map(|spot| WalkGoal {
                target: Track::Block(block_of(spot)),
                speed: STROLL_SPEED,
                close_enough: 0,
            }));
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};

    struct Floor;

    impl World for Floor {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn player(position: DVec3) -> Living {
        Living {
            target: Target::Player(1),
            kind: "minecraft:player",
            position,
            eye_height: 1.62,
            width: 0.6,
            height: 1.8,
            alive: true,
            baby: false,
            attackable: true,
            spectator: false,
        }
    }

    struct Rig {
        body: Body,
        others: Vec<Living>,
        random: LegacyRandom,
        navigation: GroundNavigation,
        profile: WalkProfile,
    }

    impl Rig {
        fn new(target: DVec3) -> Self {
            let mut body = Body::new(DVec3::new(0.5, 64.0, 0.5), 0.6, 1.77);
            body.on_ground = true;
            Self {
                body,
                others: vec![player(target)],
                random: LegacyRandom::new(5),
                navigation: GroundNavigation::default(),
                profile: super::super::walk_profile(),
            }
        }

        fn tick(&mut self, brain: &mut Brain, time: i64) -> Actions {
            let mut ctx = Ctx {
                body: &self.body,
                eye_height: 1.3452,
                others: &self.others,
                head_yaws: &[],
                world: &Floor,
                time,
                tick_count: time as i32,
                difficulty: 2,
                random: &mut self.random,
                navigation: &mut self.navigation,
                profile: &self.profile,
                fluid: FluidFrame::default(),
                passenger: false,
                levitating: false,
                last_hurt: None,
            };
            brain.tick(&mut ctx)
        }
    }

    #[test]
    fn a_shot_comes_sixteen_ticks_after_the_inhale_and_once_per_shoot() {
        let mut rig = Rig::new(DVec3::new(0.5, 64.0, 10.5));
        let mut brain = Brain {
            activity: Activity::Fight,
            attack_target: Some(Target::Player(1)),
            attackable_for: ATTACKABLE_MEMORY,
            ..Brain::default()
        };
        brain.shoot.set_for((), SHOOT_AFTER_SLIDE_TICKS);
        let first = rig.tick(&mut brain, 100);
        assert_eq!(first.sounds, vec![(INHALE_SOUND, 1.0, 1.0)]);
        assert_eq!(brain.pose, Pose::Shooting);
        assert!(first.shot.is_none() && first.face.is_some());
        let mut shots = Vec::new();
        for time in 101..=125 {
            let out = rig.tick(&mut brain, time);
            if let Some(shot) = out.shot {
                shots.push((time, shot));
                assert_eq!(out.sounds, vec![(SHOOT_SOUND, 1.5, 1.0)]);
            }
            if time == 120 {
                assert!(brain.shooting(), "it still runs at its end tick");
            }
            if time == 121 {
                assert!(!brain.shooting() && brain.pose == Pose::Standing);
                assert!(brain.shoot_cooldown.present() && !brain.shoot.present());
            }
        }
        assert_eq!(shots.len(), 1);
        assert_eq!(shots[0].0, 116);
        let shot = shots[0].1;
        assert_eq!(
            shot.from,
            DVec3::new(
                0.5,
                64.0 + f64::from(1.77_f32 / 2.0) + f64::from(0.3_f32),
                0.5
            )
        );
        assert_eq!((shot.power, shot.uncertainty), (CHARGE_POWER, -3.0));
    }

    #[test]
    fn a_target_beyond_sixteen_blocks_cancels_the_shot() {
        let mut rig = Rig::new(DVec3::new(0.5, 64.0, 17.5));
        let mut brain = Brain {
            activity: Activity::Fight,
            attack_target: Some(Target::Player(1)),
            attackable_for: ATTACKABLE_MEMORY,
            ..Brain::default()
        };
        brain.shoot.set_for((), SHOOT_AFTER_SLIDE_TICKS);
        let out = rig.tick(&mut brain, 100);
        assert!(out.sounds.is_empty() && !brain.shooting());
        assert!(!brain.shoot.present(), "the shot is forgotten");
        assert!(brain.walk_target.present(), "so it slides instead");
    }

    #[test]
    fn a_breeze_near_its_target_slides_away_and_far_from_it_slides_in() {
        let target = DVec3::new(0.5, 64.0, 2.5);
        let mut rig = Rig::new(target);
        let mut brain = Brain {
            activity: Activity::Fight,
            attack_target: Some(Target::Player(1)),
            attackable_for: ATTACKABLE_MEMORY,
            ..Brain::default()
        };
        rig.tick(&mut brain, 100);
        let Some(goal) = brain.walk_target.get().copied() else {
            panic!("it slides");
        };
        assert_eq!((goal.speed, goal.close_enough), (SLIDE_SPEED, 1));
        let Track::Block((x, y, z)) = goal.target else {
            panic!("{goal:?}");
        };
        let spot = DVec3::new(f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5);
        assert!(spot.distance(target) > 2.0, "{spot}");
        assert_eq!(brain.activity, Activity::Idle, "a walk target means idle");
        let far = DVec3::new(0.5, 64.0, 15.5);
        for seed in 0..20 {
            let mut rig = Rig::new(far);
            rig.random = LegacyRandom::new(seed);
            rig.others[0].position = far;
            let mut brain = Brain {
                activity: Activity::Fight,
                attack_target: Some(Target::Player(1)),
                attackable_for: ATTACKABLE_MEMORY,
                ..Brain::default()
            };
            rig.tick(&mut brain, 100);
            let Some(Track::Block((x, _, z))) = brain.walk_target.get().map(|goal| goal.target)
            else {
                panic!("it slides");
            };
            let distance = DVec3::new(f64::from(x) + 0.5, 64.0, f64::from(z) + 0.5).distance(far);
            assert!((3.0..=9.0).contains(&distance), "{seed}: {distance}");
        }
    }

    #[test]
    fn the_circles_and_the_spot_behind_a_target_follow_vanilla() {
        let at = DVec3::new(0.3, 64.0, 0.7);
        assert!(within_inner_circle(at, DVec3::new(3.9, 64.0, 0.5)));
        assert!(!within_inner_circle(at, DVec3::new(4.6, 64.0, 0.5)));
        assert!(!within_inner_circle(at, DVec3::new(0.5, 74.6, 0.5)));
        let mut random = LegacyRandom::new(1);
        for _ in 0..50 {
            let spot = point_in_middle_circle(at, DVec3::new(0.5, 64.0, 20.5), &mut random);
            let distance = spot.distance(DVec3::new(0.5, 64.0, 20.5));
            assert!((4.0..=8.0).contains(&distance), "{distance}");
        }
        let mut random = LegacyRandom::new(2);
        let mut behind = 0;
        for _ in 0..100 {
            let spot = point_behind(DVec3::new(0.5, 64.0, 0.5), 0.0, &mut random);
            let distance = spot.distance(DVec3::new(0.5, 64.0, 0.5));
            assert!((4.0..=8.0).contains(&distance), "{distance}");
            behind += usize::from(spot.z < 0.5);
        }
        assert!(
            behind > 80,
            "a target facing south has its back to the north: {behind}"
        );
    }
}
