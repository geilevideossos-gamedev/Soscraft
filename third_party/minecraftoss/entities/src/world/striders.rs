use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::riding::Rider;
use super::*;
use crate::goals::Control;
use crate::strider::{self, Strider};
use crate::strider_ai::{
    GoToLavaState, LookAtState, LookTarget, StriderGoalContext, StriderGoalEffect,
    registered_strider_goals,
};

#[derive(Clone)]
pub struct StriderEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub strider: Strider,
    pub previous_position: DVec3,
    pub previous_yaw: f32,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub last_damage_source: Option<DamageSourceKind>,
    pub last_damage_tick: i32,
    pub fluid: FluidFrame,
    pub jumping: bool,
    pub no_jump_delay: i32,
    pub random: LegacyRandom,
    pub look_control: LookControl,
    pub body_rotation: BodyRotation,
    pub follow_parent: FollowParentState,
    pub breed: BreedState,
    pub panic: PanicState,
    pub stroll: StrollState,
    pub tempt: TemptState,
    pub go_to_lava: GoToLavaState,
    pub random_look: RandomLookState,
    pub look_at_player: LookAtState,
    pub look_at_strider: LookAtState,
    goals: GoalSelector<StriderGoalContext>,
    steering_vehicle: Option<bool>,
    vehicle_orders: Vec<StriderGoalEffect>,
}

pub(super) struct StriderSurroundings {
    pub candidates: Vec<CowCandidate>,
    pub tempting: Vec<u64>,
    pub look_players: Vec<LookTarget>,
    pub look_striders: Vec<LookTarget>,
}

pub(super) fn strider_sounds(in_lava: bool) -> MovementSounds {
    let step = if in_lava {
        "entity.strider.step_lava"
    } else {
        "entity.strider.step"
    };
    MovementSounds::creature(Some(step)).with_step_volume(1.0)
}

impl StriderEntity {
    pub fn position(&self) -> DVec3 {
        self.strider.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn running_goals(&self) -> Vec<&'static str> {
        self.goals
            .running_ids()
            .map(|id| crate::strider_ai::GOAL_NAMES[id])
            .collect()
    }

    pub fn tempted(&self) -> bool {
        self.tempt.running
    }

    fn candidate(&self) -> CowCandidate {
        CowCandidate {
            id: self.id,
            position: self.strider.body.position,
            width: self.strider.body.width,
            height: self.strider.body.height,
            age: self.strider.age.ticks,
            alive: self.strider.health > 0.0,
            in_love: self.strider.in_love,
            panicking: self.panic.running,
        }
    }

    fn tick_goals(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        surroundings: StriderSurroundings,
    ) -> Option<u64> {
        let full_goal_tick = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
        let last_damage_panic = DamageSourceKind::panics(self.last_damage_source)
            && self.tick_count - self.last_damage_tick <= 40;
        let in_lava = self.fluid.in_lava();
        let mut context = StriderGoalContext {
            child: self.candidate(),
            candidates: surroundings.candidates,
            follow_parent: std::mem::take(&mut self.follow_parent),
            breed: std::mem::take(&mut self.breed),
            panic: std::mem::take(&mut self.panic),
            stroll: std::mem::take(&mut self.stroll),
            tempt: std::mem::take(&mut self.tempt),
            go_to_lava: std::mem::take(&mut self.go_to_lava),
            random_look: std::mem::take(&mut self.random_look),
            look_at_player: std::mem::take(&mut self.look_at_player),
            look_at_strider: std::mem::take(&mut self.look_at_strider),
            last_damage_panic,
            no_action_time: self.no_action_time,
            navigation_done: self
                .steering_vehicle
                .unwrap_or_else(|| self.strider.navigation.is_done()),
            in_lava,
            eye_height: self.strider.eye_height(),
            players: players.to_vec(),
            tempting: surroundings.tempting,
            look_players: surroundings.look_players,
            look_striders: surroundings.look_striders,
            random: std::mem::take(&mut self.random),
            fluid: self.fluid,
            walk: self.strider.walk_profile(in_lava),
            effects: Vec::new(),
        };
        if full_goal_tick {
            self.goals.tick(&mut context, world);
        } else {
            self.goals.tick_running(&mut context, false);
        }
        self.follow_parent = context.follow_parent;
        self.breed = context.breed;
        self.panic = context.panic;
        self.stroll = context.stroll;
        self.tempt = context.tempt;
        self.go_to_lava = context.go_to_lava;
        self.random_look = context.random_look;
        self.look_at_player = context.look_at_player;
        self.look_at_strider = context.look_at_strider;
        self.random = context.random;
        let mut partner = None;
        for effect in context.effects {
            let fluid = self.fluid;
            if self.steering_vehicle.is_some()
                && matches!(
                    effect,
                    StriderGoalEffect::Navigate { .. }
                        | StriderGoalEffect::NavigateToEntity { .. }
                        | StriderGoalEffect::StopNavigation
                )
            {
                self.vehicle_orders.push(effect);
                continue;
            }
            match effect {
                StriderGoalEffect::Navigate { target, speed } => {
                    let _ = self.strider.navigate_to(world, fluid, target, speed);
                }
                StriderGoalEffect::NavigateToEntity { target, speed } => {
                    let _ = self.strider.navigate_to_entity(world, fluid, target, speed);
                }
                StriderGoalEffect::LookAt {
                    target,
                    y_max,
                    x_max,
                } => self
                    .look_control
                    .set_look_at_with_limits(target, y_max, x_max),
                StriderGoalEffect::StopNavigation => self.strider.navigation.stop(),
                StriderGoalEffect::Breed { partner_id } => partner = Some(partner_id),
            }
        }
        partner
    }

    fn steer(&mut self, world: &impl World) {
        let strider = &mut self.strider;
        let position = strider.body.position;
        let (can_update, surface) =
            crate::navigation::ground_view(world, &strider.body, self.fluid, false);
        if let Some((target, speed)) = strider.navigation.tick_in(
            world,
            position,
            can_update,
            surface,
            strider.body.width,
            strider.speed,
        ) {
            strider.move_control.set_wanted_position(target, speed);
        }
        let obstacle_top = crate::control::obstacle_top(world, position);
        let movement_speed = self.effects.movement_speed(strider.movement_speed());
        let control = strider.move_control.tick(
            position,
            strider.body.on_ground,
            strider.yaw,
            strider.speed,
            strider.forward,
            strider.sideways,
            movement_speed,
            strider.body.width,
            strider.body.step_height,
            obstacle_top,
            |_, _| true,
        );
        strider.yaw = control.yaw;
        strider.speed = control.speed;
        strider.forward = control.forward;
        strider.sideways = control.sideways;
        self.jumping = control.jump;
    }

    pub fn hurt(&mut self, amount: f32, source: DamageSourceKind) -> DamageResult {
        let full_hit = self.strider.damage.cooldown_ticks <= 10;
        if self.strider.health > 0.0 {
            self.no_action_time = 0;
        }
        let result = self.strider.hurt_generic(amount);
        let position = self.position();
        self.strider.damage.place_death(result, position, false);
        if result.applied {
            self.last_damage_source = Some(source);
            self.last_damage_tick = self.tick_count + 1;
            if full_hit {
                if !result.died {
                    self.ambient_sound_time = -120;
                }
                let voice = hurt_voice(&mut self.random, result.died, self.strider.age.baby());
                self.voices.push((voice, position));
            }
        }
        result
    }

    pub fn ridden_speed(&self) -> f32 {
        let attribute = self.effects.movement_speed(self.strider.movement_speed());
        let factor = if self.strider.suffocating {
            0.35_f32
        } else {
            0.55_f32
        };
        (attribute * f64::from(factor) * f64::from(self.strider.steering.boost_factor())) as f32
    }

    fn hurt_by_water(&mut self, world: &impl World) {
        let p = self.strider.body.position;
        let feet = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let top = (
            feet.0,
            (p.y + f64::from(self.strider.body.height)).floor() as i32,
            feet.2,
        );
        if self.strider.body.touching_water || world.rain_at(feet) || world.rain_at(top) {
            self.hurt(1.0, DamageSourceKind::Generic);
        }
    }

    fn travel(&mut self, world: &impl World, input: DVec3, speed: f32) -> Option<f64> {
        let body = &mut self.strider.body;
        let p = body.position;
        let feet = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let yaw = self.strider.yaw;
        if self.fluid.in_water() {
            body.travel_water(world, input, yaw);
            None
        } else if self.fluid.in_lava() && !strider::is_lava(world, feet) {
            body.travel_lava(world, input, yaw, self.fluid.lava_height);
            None
        } else {
            body.travel_air_jumping(world, input, speed, yaw, self.jumping)
        }
    }

    pub(super) fn tick_body(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        game_time: i64,
        carried_warm: bool,
        surroundings: StriderSurroundings,
        ridden: Option<(f32, f32)>,
    ) -> (bool, Option<u64>) {
        self.previous_position = self.strider.body.position;
        self.previous_yaw = self.strider.yaw;
        self.tick_count += 1;
        let at = self.position();
        if self.tempt.running && self.random.next_int(140) == 0 {
            let pitch = voice_pitch(&mut self.random, self.strider.age.baby());
            self.voices
                .push((Voice::Event("entity.strider.happy", 1.0, pitch), at));
        } else if self.panic.running && self.random.next_int(60) == 0 {
            let pitch = voice_pitch(&mut self.random, self.strider.age.baby());
            self.voices
                .push((Voice::Event("entity.strider.retreat", 1.0, pitch), at));
        }
        if !self.no_ai {
            let fluid = FluidFrame::sample(
                world,
                self.strider.body.position,
                self.strider.body.width,
                self.strider.body.height,
            );
            self.strider.suffocating = !self.strider.warm(world, fluid, carried_warm);
        }
        let in_lava = self.fluid.in_lava();
        self.fluid = base_tick_fluid(
            &mut self.strider.body,
            world,
            self.tick_count == 1,
            &mut self.random,
            &mut self.voices,
            strider_sounds(in_lava),
        );
        hazards::burn(self, world, game_time);
        let eye = self.strider.eye_height();
        hazards::suffocate(self, world, eye, game_time);
        let water_breathing = self.effects.has(MobEffect::WaterBreathing);
        if self.strider.health > 0.0 && breathe(&mut self.strider.body, world, eye, water_breathing)
        {
            self.hurt(2.0, DamageSourceKind::Generic);
        }
        let removed = self.strider.damage.tick();
        for work in self.effects.tick(false) {
            match work.resolve(self.strider.health, strider::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut self.strider.health, strider::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    self.hurt(amount, DamageSourceKind::Magic);
                }
                _ => {}
            }
        }
        if self.strider.health > 0.0 {
            if (self.random.next_int(1000) as i32) < self.ambient_sound_time {
                self.ambient_sound_time = -120;
                if !self.panic.running && !self.tempt.running {
                    let voice =
                        Voice::Ambient(voice_pitch(&mut self.random, self.strider.age.baby()));
                    self.voices.push((voice, self.position()));
                }
            } else {
                self.ambient_sound_time += 1;
            }
        }
        if removed {
            return (false, None);
        }
        if self.strider.health <= 0.0 {
            if !self.no_ai {
                let yaw = self.strider.yaw;
                dying_travel(
                    &mut self.strider.body,
                    world,
                    yaw,
                    self.tick_count,
                    &mut self.random,
                    &mut self.voices,
                    strider_sounds(self.fluid.in_lava()),
                    Some(false),
                );
            }
            let old = self.previous_position;
            hazards::blocks_act(self, world, old, game_time);
            return (true, None);
        }
        let mut partner = None;
        if self.no_ai {
            self.strider.body.trim_small_velocity();
        } else {
            self.strider.body.trim_small_velocity();
            if players.iter().any(|player| {
                player.alive
                    && !player.spectator
                    && player.position.distance_squared(self.strider.body.position) < 32.0 * 32.0
            }) {
                self.no_action_time = 0;
            }
            self.no_action_time += 1;
            partner = self.tick_goals(world, players, surroundings);
            self.steer(world);
            if self.no_jump_delay > 0 {
                self.no_jump_delay -= 1;
            }
            let jump_threshold = if self.strider.age.baby() { 0.0 } else { 0.4 };
            let jumping = self.jumping;
            self.strider.body.living_jump(
                world,
                self.fluid,
                jumping,
                &mut self.no_jump_delay,
                jump_threshold,
            );
            let eye_height = self.strider.eye_height();
            self.look_control.tick(
                self.strider.body.position,
                eye_height,
                self.body_rotation.body_yaw,
                !self.strider.navigation.is_done(),
            );
            let (input, speed) = match ridden {
                Some((yaw, pitch)) => {
                    self.strider.yaw = yaw;
                    self.look_control.head_yaw = yaw;
                    self.look_control.pitch = pitch * 0.5;
                    self.body_rotation.body_yaw = yaw;
                    self.previous_yaw = yaw;
                    self.strider.steering.tick_boost();
                    (DVec3::new(0.0, 0.0, 1.0), self.ridden_speed())
                }
                None => (
                    DVec3::new(
                        f64::from(self.strider.sideways),
                        0.0,
                        f64::from(self.strider.forward),
                    ),
                    self.strider.speed,
                ),
            };
            let landed = self.travel(world, input, speed);
            let in_lava = FluidFrame::sample(
                world,
                self.strider.body.position,
                self.strider.body.width,
                self.strider.body.height,
            )
            .in_lava();
            play_movement(
                &mut self.strider.body,
                self.tick_count,
                &mut self.random,
                &mut self.voices,
                strider_sounds(in_lava),
            );
            let fall = if in_lava {
                self.strider.body.fall_distance = 0.0;
                None
            } else {
                landed.and_then(|fallen| {
                    fall_damage(&self.strider.body, world, fallen, false, &mut self.voices)
                })
            };
            self.body_rotation.tick(
                self.strider.yaw,
                &mut self.look_control,
                self.previous_position,
                self.strider.body.position,
            );
            if let Some((yaw, _)) = ridden {
                self.body_rotation.body_yaw = yaw;
                self.look_control.head_yaw = yaw;
            }
            if let Some(damage) = fall {
                self.hurt(damage, DamageSourceKind::Generic);
            }
            self.hurt_by_water(world);
            self.fluid = FluidFrame::sample(
                world,
                self.strider.body.position,
                self.strider.body.width,
                self.strider.body.height,
            );
        }
        let old = self.previous_position;
        hazards::blocks_act(self, world, old, game_time);
        (true, partner)
    }

    pub(super) fn tick_age(&mut self) {
        let was_baby = self.strider.age.baby();
        self.strider.age.tick(true);
        if was_baby != self.strider.age.baby() {
            self.strider.sync_dimensions();
        }
        if self.strider.age.ticks != 0 {
            self.strider.in_love = 0;
        } else if self.strider.in_love > 0 {
            self.strider.in_love -= 1;
            if self.strider.in_love % 10 == 0 {
                for _ in 0..3 {
                    let _ = self.random.next_gaussian();
                }
                for _ in 0..3 {
                    let _ = self.random.next_double();
                }
            }
        }
    }

    pub(super) fn float(&mut self, world: &impl World) {
        let fluid = FluidFrame::sample(
            world,
            self.strider.body.position,
            self.strider.body.width,
            self.strider.body.height,
        );
        self.strider.float_in_lava(world, fluid);
    }
}

impl EntityWorld {
    pub fn spawn_strider(&mut self, mut strider: Strider, no_ai: bool) -> u64 {
        self.next_id += 1;
        strider.sync_dimensions();
        strider.body.lava_floor = true;
        strider.body.step_after = Some(0.6);
        let id = self.next_id;
        let previous_position = strider.body.position;
        let previous_yaw = strider.yaw;
        self.striders.push(StriderEntity {
            id,
            effects: Default::default(),
            strider,
            previous_position,
            previous_yaw,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            last_damage_source: None,
            last_damage_tick: 0,
            fluid: FluidFrame::default(),
            jumping: false,
            no_jump_delay: 0,
            random: LegacyRandom::new(0),
            look_control: LookControl::new(previous_yaw),
            body_rotation: BodyRotation::new(previous_yaw),
            follow_parent: FollowParentState::default(),
            breed: BreedState::default(),
            panic: PanicState::default(),
            stroll: StrollState::default(),
            tempt: TemptState::default(),
            go_to_lava: GoToLavaState::default(),
            random_look: RandomLookState::default(),
            look_at_player: LookAtState::default(),
            look_at_strider: LookAtState::default(),
            goals: registered_strider_goals(),
            steering_vehicle: None,
            vehicle_orders: Vec::new(),
        });
        self.order.push(EntityKey::Strider(id));
        self.file_in_section(EntityKey::Strider(id));
        id
    }

    pub fn striders(&self) -> &[StriderEntity] {
        &self.striders
    }

    pub fn strider_mut(&mut self, id: u64) -> Option<&mut StriderEntity> {
        self.striders.iter_mut().find(|entity| entity.id == id)
    }

    pub(super) fn tick_strider(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let game_time = self.game_time;
        let surroundings = self.strider_surroundings(players);
        let vehicle = self
            .vehicle_of(Rider::Mob(id))
            .and_then(|vehicle| self.striders.iter().find(|e| e.id == vehicle));
        let carried_warm = vehicle.is_some_and(|e| !e.strider.suffocating);
        let steering = vehicle
            .filter(|e| self.controlling_mob(e.id) == Some(id))
            .map(|e| (e.id, e.strider.navigation.is_done()));
        let controlled = self.controlling_mob(id).is_some();
        let ridden = self
            .controlling_player(id)
            .and_then(|player| self.player_controls(player))
            .map(|controls| (controls.yaw, controls.pitch));
        let Some(entity) = self.striders.iter_mut().find(|e| e.id == id) else {
            return;
        };
        for control in [Control::Move, Control::Jump, Control::Look] {
            entity.goals.set_enabled(control, !controlled);
        }
        entity.steering_vehicle = steering.map(|(_, done)| done);
        let (stepped, partner) = entity.tick_body(
            world,
            players,
            game_time,
            carried_warm,
            surroundings,
            ridden,
        );
        entity.steering_vehicle = None;
        let orders = std::mem::take(&mut entity.vehicle_orders);
        if let Some((vehicle, _)) = steering {
            self.order_vehicle(vehicle, &orders, world);
        }
        if !stepped {
            return;
        }
        self.push_entities(EntityKey::Strider(id), world, game_time, ticks);
        if let Some(partner) = partner {
            self.breed_striders(id, partner);
        }
        let Some(entity) = self.striders.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entity.strider.health > 0.0 {
            entity.tick_age();
        }
        entity.float(world);
    }

    fn strider_surroundings(&self, players: &[PlayerCandidate]) -> StriderSurroundings {
        let tempting = match &self.recipes {
            Some(recipes) => self
                .player_controls
                .iter()
                .filter(|controls| {
                    controls.holds(|item| recipes.item_in_tag(strider::TEMPT_TAG, item))
                })
                .map(|controls| controls.id)
                .collect(),
            None => Vec::new(),
        };
        let look_players = players
            .iter()
            .filter(|p| p.alive && !p.spectator)
            .map(|p| LookTarget {
                id: PLAYER_TARGET + p.id,
                eye: p.position + DVec3::new(0.0, f64::from(p.eye_height), 0.0),
            })
            .collect();
        let look_striders = self
            .striders
            .iter()
            .filter(|e| e.strider.health > 0.0)
            .map(|e| LookTarget {
                id: e.id,
                eye: e.position() + DVec3::new(0.0, f64::from(e.strider.eye_height()), 0.0),
            })
            .collect();
        StriderSurroundings {
            candidates: self.striders.iter().map(StriderEntity::candidate).collect(),
            tempting,
            look_players,
            look_striders,
        }
    }

    fn breed_striders(&mut self, id: u64, partner: u64) {
        let Some(index) = self.striders.iter().position(|e| e.id == id) else {
            return;
        };
        let Some(partner_index) = self.striders.iter().position(|e| e.id == partner) else {
            return;
        };
        let position = self.striders[index].position();
        for i in [index, partner_index] {
            self.striders[i].strider.age.set(6000);
            self.striders[i].strider.in_love = 0;
        }
        let mut offspring = Strider::new(position);
        offspring.age.set(Age::BABY_START);
        self.spawn_strider(offspring, false);
    }
}
