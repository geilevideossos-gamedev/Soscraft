use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::strider::{self, Strider};

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
    ) -> bool {
        self.previous_position = self.strider.body.position;
        self.previous_yaw = self.strider.yaw;
        self.tick_count += 1;
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
                let voice = Voice::Ambient(voice_pitch(&mut self.random, self.strider.age.baby()));
                self.voices.push((voice, self.position()));
            } else {
                self.ambient_sound_time += 1;
            }
        }
        if removed {
            return false;
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
            return true;
        }
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
            let speed = self.effects.movement_speed(self.strider.movement_speed()) as f32;
            let landed = self.travel(world, DVec3::ZERO, speed);
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
        true
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
        let Some(entity) = self.striders.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if !entity.tick_body(world, players, game_time, false) {
            return;
        }
        self.push_entities(EntityKey::Strider(id), world, game_time, ticks);
        let Some(entity) = self.striders.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entity.strider.health > 0.0 {
            entity.tick_age();
        }
        entity.float(world);
    }
}
