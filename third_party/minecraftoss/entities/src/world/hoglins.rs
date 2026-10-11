use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::hoglin::{self, Hoglin, HoglinKind};
use crate::hoglin_brain::{self, Ctx, HoglinBrain, Living};
use crate::monster_ai::Target;

#[derive(Clone)]
pub struct HoglinEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub hoglin: Hoglin,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub fluid: FluidFrame,
    pub jumping: bool,
    pub no_jump_delay: i32,
    pub look_control: LookControl,
    pub body_rotation: BodyRotation,
    pub brain: HoglinBrain,
    pub navigation: GroundNavigation,
    pub move_control: MoveControl,
    pub speed: f32,
    pub forward: f32,
    pub sideways: f32,
    pub hurt_by: Vec<Target>,
}

pub(super) fn hoglin_sounds(kind: HoglinKind) -> MovementSounds {
    match kind {
        HoglinKind::Hoglin => MovementSounds::creature(Some("entity.hoglin.step")),
        HoglinKind::Zoglin => MovementSounds::monster(Some("entity.zoglin.step")),
    }
    .with_step_volume(hoglin::STEP_VOLUME)
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Thought {
    attack: Option<Target>,
    courting: Option<Target>,
    birth: Option<Target>,
    convert: bool,
}

fn attacker_of(owner: u64) -> Target {
    if owner >= PLAYER_TARGET {
        Target::Player(owner - PLAYER_TARGET)
    } else {
        Target::Mob(owner)
    }
}

impl HoglinEntity {
    pub fn position(&self) -> DVec3 {
        self.hoglin.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.hoglin.health > 0.0 && !self.hoglin.damage.dead {
            self.no_action_time = 0;
        }
        let result = self.hoglin.hurt_generic(amount);
        let position = self.position();
        let on_fire = self.hoglin.body.fire_ticks > 0 && !self.hoglin.fire_immune();
        self.hoglin.damage.place_death(result, position, on_fire);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -hoglin::AMBIENT_INTERVAL;
            }
            let voice = hurt_voice(&mut self.random, result.died, self.hoglin.is_baby());
            self.voices.push((voice, position));
        }
        result
    }

    fn hostile(&self) -> bool {
        self.hoglin.kind == HoglinKind::Zoglin
    }

    fn is_hoglin(&self) -> bool {
        self.hoglin.kind == HoglinKind::Hoglin
    }

    pub fn walk_profile(&self) -> WalkProfile {
        let mut profile = WalkProfile::animal(self.hoglin.body.width, self.hoglin.body.height);
        profile.can_float = false;
        if self.hostile() {
            profile.clear_malus(PathType::FireInNeighbor);
            profile.clear_malus(PathType::Fire);
            profile.walk_target = crate::walk_path::WalkTarget::Monster;
        }
        profile
    }

    fn make_sound(&mut self, event: &'static str) {
        let pitch = voice_pitch(&mut self.random, self.hoglin.is_baby());
        self.voices.push((Voice::Event(event, 1.0, pitch), self.position()));
    }

    fn ambient_voice(&mut self) -> Voice {
        let pitch = voice_pitch(&mut self.random, self.hoglin.is_baby());
        if self.is_hoglin() {
            let event = self
                .brain
                .activity_sound(self.position(), self.hoglin.converting);
            Voice::Event(event, 1.0, pitch)
        } else if self.brain.memories.attack_target.present() {
            Voice::Event("entity.zoglin.angry", 1.0, pitch)
        } else {
            Voice::Ambient(pitch)
        }
    }

    fn think(
        &mut self,
        world: &impl World,
        others: &[Living],
        lovers: &[Target],
        game_time: i64,
    ) -> Thought {
        let profile = self.walk_profile();
        let mut ctx = Ctx {
            id: self.id,
            kind: self.hoglin.kind.type_id(),
            body: &self.hoglin.body,
            eye_height: self.hoglin.eye_height(),
            baby: self.hoglin.is_baby(),
            others,
            world,
            time: game_time,
            random: &mut self.random,
            navigation: &mut self.navigation,
            profile: &profile,
            fluid: self.fluid,
            look_at: None,
            attack: None,
            sounds: Vec::new(),
            in_love: self.hoglin.in_love > 0,
            lovers,
            courting: None,
            birth: None,
        };
        self.brain.tick(&mut ctx);
        let converting = self.hoglin.converting;
        if self.hoglin.kind == HoglinKind::Hoglin {
            self.brain.update_activity(&mut ctx, converting);
        } else {
            self.brain.update_zoglin_activity(&mut ctx);
        }
        let (look_at, attack, sounds) = (ctx.look_at, ctx.attack, ctx.sounds);
        let (courting, birth) = (ctx.courting, ctx.birth);
        if let Some(at) = look_at {
            self.look_control.set_look_at(at);
        }
        for sound in sounds {
            self.make_sound(sound);
        }
        if attack.is_some() {
            self.hoglin.attack_animation = hoglin::ATTACK_ANIMATION_DURATION;
            let event = match self.hoglin.kind {
                HoglinKind::Hoglin => "entity.hoglin.attack",
                HoglinKind::Zoglin => "entity.zoglin.attack",
            };
            self.make_sound(event);
        }
        Thought {
            attack,
            courting,
            birth,
            convert: false,
        }
    }

    fn steer(&mut self, world: &impl World) {
        let position = self.hoglin.body.position;
        let (can_update, surface) =
            crate::navigation::ground_view(world, &self.hoglin.body, self.fluid, false);
        if let Some((target, speed)) = self.navigation.tick_in(
            world,
            position,
            can_update,
            surface,
            self.hoglin.body.width,
            self.speed,
        ) {
            self.move_control.set_wanted_position(target, speed);
        }
        let obstacle_top = crate::control::obstacle_top(world, position);
        let movement_speed = self.effects.movement_speed(hoglin::MOVEMENT_SPEED);
        let control = self.move_control.tick(
            position,
            self.hoglin.body.on_ground,
            self.hoglin.yaw,
            self.speed,
            self.forward,
            self.sideways,
            movement_speed,
            self.hoglin.body.width,
            self.hoglin.body.step_height,
            obstacle_top,
            |_, _| true,
        );
        self.hoglin.yaw = control.yaw;
        self.speed = control.speed;
        self.forward = control.forward;
        self.sideways = control.sideways;
        self.jumping = control.jump;
    }

    fn travel(&mut self, world: &impl World) -> Option<f64> {
        let input = DVec3::new(f64::from(self.sideways), 0.0, f64::from(self.forward));
        let speed = self.speed;
        let body = &mut self.hoglin.body;
        let yaw = self.hoglin.yaw;
        let fluid = self.fluid;
        if fluid.in_water() {
            body.travel_water(world, input, yaw);
            None
        } else if fluid.in_lava() {
            body.travel_lava(world, input, yaw, fluid.lava_height);
            None
        } else {
            body.travel_air_jumping(world, input, speed, yaw, self.jumping)
        }
    }

    pub(super) fn tick_body(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        others: &[Living],
        lovers: &[Target],
        game_time: i64,
        zombify: bool,
    ) -> (bool, Thought) {
        self.previous_position = self.hoglin.body.position;
        self.hoglin.converting =
            self.is_hoglin() && !self.no_ai && !self.hoglin.immune_to_zombification && zombify;
        self.tick_count += 1;
        let sounds = hoglin_sounds(self.hoglin.kind);
        self.fluid = base_tick_fluid(
            &mut self.hoglin.body,
            world,
            self.tick_count == 1,
            &mut self.random,
            &mut self.voices,
            sounds,
        );
        hazards::burn(self, world, game_time);
        let eye = self.hoglin.eye_height();
        hazards::suffocate(self, world, eye, game_time);
        let water_breathing = self.effects.has(MobEffect::WaterBreathing) || self.hostile();
        if self.hoglin.health > 0.0 && breathe(&mut self.hoglin.body, world, eye, water_breathing) {
            self.hurt(2.0);
        }
        let removed = self.hoglin.damage.tick();
        let undead = self.hostile();
        for work in self.effects.tick(undead) {
            match work.resolve(self.hoglin.health, hoglin::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut self.hoglin.health, hoglin::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    self.hurt(amount);
                }
                _ => {}
            }
        }
        if self.hoglin.health > 0.0 {
            if (self.random.next_int(1000) as i32) < self.ambient_sound_time {
                self.ambient_sound_time = -hoglin::AMBIENT_INTERVAL;
                let voice = self.ambient_voice();
                self.voices.push((voice, self.position()));
            } else {
                self.ambient_sound_time += 1;
            }
        }
        if removed {
            return (false, Thought::default());
        }
        let old = self.previous_position;
        if self.hoglin.health <= 0.0 {
            if !self.no_ai {
                let yaw = self.hoglin.yaw;
                let hostile = self.hostile();
                dying_travel(
                    &mut self.hoglin.body,
                    world,
                    yaw,
                    self.tick_count,
                    &mut self.random,
                    &mut self.voices,
                    sounds,
                    Some(hostile),
                );
            }
            hazards::blocks_act(self, world, old, game_time);
            return (true, Thought::default());
        }
        self.hoglin.tick_attack_animation();
        let position = self.hoglin.body.position;
        let mut thought = Thought::default();
        if self.no_ai {
            self.hoglin.body.trim_small_velocity();
            if self.hostile() {
                monster_idle_without_ai(world, players, position, eye, &mut self.no_action_time);
            } else if players.iter().any(|p| {
                p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0
            }) {
                self.no_action_time = 0;
            }
        } else {
            self.hoglin.body.trim_small_velocity();
            if self.hostile() {
                monster_idle_without_ai(world, players, position, eye, &mut self.no_action_time);
            } else if players.iter().any(|p| {
                p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0
            }) {
                self.no_action_time = 0;
            }
            self.no_action_time += 1;
            thought = self.think(world, others, lovers, game_time);
            if self.hoglin.converting {
                self.hoglin.time_in_overworld += 1;
                if self.hoglin.time_in_overworld > hoglin::CONVERSION_TIME {
                    self.make_sound("entity.hoglin.converted_to_zombified");
                    thought.convert = true;
                }
            } else {
                self.hoglin.time_in_overworld = 0;
            }
            self.steer(world);
            if self.no_jump_delay > 0 {
                self.no_jump_delay -= 1;
            }
            let jumping = self.jumping;
            self.hoglin
                .body
                .living_jump(world, self.fluid, jumping, &mut self.no_jump_delay, 0.4);
            self.look_control.tick(
                self.hoglin.body.position,
                eye,
                self.body_rotation.body_yaw,
                !self.navigation.is_done(),
            );
            let landed = self.travel(world);
            play_movement(
                &mut self.hoglin.body,
                self.tick_count,
                &mut self.random,
                &mut self.voices,
                sounds,
            );
            self.body_rotation.tick(
                self.hoglin.yaw,
                &mut self.look_control,
                self.previous_position,
                self.hoglin.body.position,
            );
            let hostile = self.hostile();
            if let Some(damage) = landed.and_then(|fallen| {
                fall_damage(&self.hoglin.body, world, fallen, hostile, &mut self.voices)
            }) {
                self.hurt(damage);
            }
        }
        hazards::blocks_act(self, world, old, game_time);
        (true, thought)
    }

    pub(super) fn tick_age(&mut self) {
        if self.hoglin.kind != HoglinKind::Hoglin {
            return;
        }
        let was_baby = self.hoglin.age.baby();
        self.hoglin.age.tick(true);
        if was_baby != self.hoglin.age.baby() {
            self.hoglin.sync_dimensions();
        }
        if self.hoglin.age.ticks != 0 {
            self.hoglin.in_love = 0;
        } else if self.hoglin.in_love > 0 {
            self.hoglin.in_love -= 1;
            if self.hoglin.in_love % 10 == 0 {
                for _ in 0..3 {
                    let _ = self.random.next_gaussian();
                }
                for _ in 0..3 {
                    let _ = self.random.next_double();
                }
            }
        }
    }
}

impl EntityWorld {
    pub fn spawn_hoglin(&mut self, mut hoglin: Hoglin, no_ai: bool) -> u64 {
        self.next_id += 1;
        hoglin.sync_dimensions();
        let id = self.next_id;
        let previous_position = hoglin.body.position;
        let yaw = hoglin.yaw;
        self.hoglins.push(HoglinEntity {
            id,
            effects: Default::default(),
            hoglin,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            previous_position,
            fluid: FluidFrame::default(),
            jumping: false,
            no_jump_delay: 0,
            look_control: LookControl::new(yaw),
            body_rotation: BodyRotation::new(yaw),
            brain: HoglinBrain::default(),
            navigation: GroundNavigation::default(),
            move_control: MoveControl::default(),
            speed: 0.0,
            forward: 0.0,
            sideways: 0.0,
            hurt_by: Vec::new(),
        });
        self.order.push(EntityKey::Hoglin(id));
        self.file_in_section(EntityKey::Hoglin(id));
        id
    }

    pub fn hoglins(&self) -> &[HoglinEntity] {
        &self.hoglins
    }

    pub fn hoglin_mut(&mut self, id: u64) -> Option<&mut HoglinEntity> {
        self.hoglins.iter_mut().find(|entity| entity.id == id)
    }

    pub fn note_hoglin_hurt(&mut self, id: u64, attacker: Target) {
        if let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) {
            entity.hurt_by.push(attacker);
        }
    }

    pub(super) fn note_hoglin_hurt_by_owner(&mut self, id: u64, owner: u64) {
        self.note_hoglin_hurt(id, attacker_of(owner));
    }

    pub(super) fn hoglin_living(&self, id: u64, players: &[PlayerCandidate]) -> Vec<Living> {
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
            self.mob_candidates()
                .into_iter()
                .filter(|m| m.id != id)
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
        out.extend(self.villagers.iter().map(|e| Living {
            target: Target::Mob(e.id),
            kind: "minecraft:villager",
            position: e.villager.body.position,
            eye_height: e.villager.eye_height(),
            width: e.villager.body.width,
            height: e.villager.body.height,
            alive: e.villager.health > 0.0,
            baby: e.villager.age.baby(),
            attackable: true,
            spectator: false,
        }));
        out
    }

    fn hoglin_was_hurt_by(
        &mut self,
        id: u64,
        attacker: Target,
        others: &[Living],
        world: &dyn World,
    ) {
        let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entity.hoglin.health <= 0.0 {
            return;
        }
        if entity.hoglin.kind == HoglinKind::Zoglin {
            let position = entity.position();
            let Some(living) = hoglin_brain::find(others, attacker) else {
                return;
            };
            let valid = living.alive && living.attackable && !living.spectator;
            if valid && !entity.brain.much_further(position, others, attacker) {
                entity.brain.set_attack_target(attacker);
            }
            return;
        }
        let position = entity.position();
        entity.brain.memories.pacified.erase();
        entity.brain.memories.breed_target.erase();
        if entity.hoglin.is_baby() {
            entity
                .brain
                .retreat_from_nearest(position, others, attacker, &mut entity.random);
            return;
        }
        let Some(living) = hoglin_brain::find(others, attacker).copied() else {
            return;
        };
        if entity.brain.activity == hoglin_brain::Activity::Avoid && living.kind == "minecraft:piglin"
        {
            return;
        }
        let eye = position + DVec3::Y * f64::from(entity.hoglin.eye_height());
        let current = entity.brain.memories.attack_target.get() == Some(&attacker);
        let attackable = if current {
            living.alive
                && living.attackable
                && !living.spectator
                && position.distance_squared(living.position)
                    <= hoglin_brain::FOLLOW_RANGE * hoglin_brain::FOLLOW_RANGE
        } else {
            hoglin_brain::targetable(world, eye, position, &living, true)
        };
        if !attackable || living.kind == "minecraft:hoglin" {
            return;
        }
        if entity.brain.much_further(position, others, attacker) {
            return;
        }
        entity.brain.set_attack_target(attacker);
        self.broadcast_hoglin_attack(id, attacker, others);
    }

    fn broadcast_hoglin_attack(&mut self, id: u64, target: Target, others: &[Living]) {
        let Some(entity) = self.hoglins.iter().find(|e| e.id == id) else {
            return;
        };
        for other in entity.brain.memories.visible_adult_hoglins() {
            let Target::Mob(other) = other else { continue };
            if let Some(e) = self.hoglins.iter_mut().find(|e| e.id == other) {
                let position = e.position();
                e.brain.set_attack_target_if_closer(position, others, target);
            }
        }
    }

    fn broadcast_hoglin_retreat(&mut self, id: u64, target: Target, others: &[Living]) {
        let Some(entity) = self.hoglins.iter().find(|e| e.id == id) else {
            return;
        };
        for other in entity.brain.memories.visible_adult_hoglins() {
            let Target::Mob(other) = other else { continue };
            if let Some(e) = self.hoglins.iter_mut().find(|e| e.id == other) {
                let position = e.position();
                e.brain
                    .retreat_from_nearest(position, others, target, &mut e.random);
            }
        }
    }

    pub fn convert_hoglin_to_zoglin(&mut self, old_id: u64) -> Option<u64> {
        let index = self.hoglins.iter().position(|e| e.id == old_id)?;
        let old = self.hoglins.remove(index);
        self.order
            .retain(|key| !matches!(key, EntityKey::Hoglin(id) if *id == old_id));
        let mut zoglin = Hoglin::new(HoglinKind::Zoglin, old.hoglin.body.position);
        zoglin.yaw = old.hoglin.yaw;
        zoglin.set_baby(old.hoglin.is_baby());
        zoglin.body.velocity = old.hoglin.body.velocity;
        zoglin.body.on_ground = old.hoglin.body.on_ground;
        zoglin.body.fall_distance = old.hoglin.body.fall_distance;
        zoglin.body.fire_ticks = old.hoglin.body.fire_ticks;
        zoglin.damage.cooldown_ticks = old.hoglin.damage.cooldown_ticks;
        zoglin.persistence_required = old.hoglin.persistence_required;
        let id = self.spawn_hoglin(zoglin, old.no_ai);
        let entity = self.hoglins.iter_mut().find(|e| e.id == id)?;
        entity.effects = old.effects;
        entity.voices = old.voices;
        entity.effects.add(crate::effects::EffectInstance::new(
            MobEffect::Nausea,
            200,
            0,
        ));
        entity.look_control = old.look_control;
        entity.body_rotation = old.body_rotation;
        Some(id)
    }

    fn breed_hoglins(&mut self, id: u64, partner: u64) {
        let Some(index) = self.hoglins.iter().position(|e| e.id == id) else {
            return;
        };
        let Some(partner_index) = self.hoglins.iter().position(|e| e.id == partner) else {
            return;
        };
        let position = self.hoglins[index].position();
        for i in [index, partner_index] {
            self.hoglins[i].hoglin.age.set(6000);
            self.hoglins[i].hoglin.in_love = 0;
        }
        self.hoglins[partner_index].brain.memories.breed_target.erase();
        let experience = 1 + self.hoglins[index].random.next_int(7) as i32;
        if self.mob_drops {
            self.trade_experience.push((position, experience));
        }
        let mut child = Hoglin::new(HoglinKind::Hoglin, position);
        child.set_baby(true);
        child.persistence_required = true;
        self.spawn_hoglin(child, false);
    }

    fn knockback_resistance(&self, id: u64) -> f64 {
        if self.iron_golems.iter().any(|e| e.id == id) {
            1.0
        } else if self.hoglins.iter().any(|e| e.id == id) {
            hoglin::KNOCKBACK_RESISTANCE
        } else {
            0.0
        }
    }

    fn hoglin_hits(&mut self, id: u64, target: Target, others: &[Living]) {
        let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let (position, baby) = (entity.position(), entity.hoglin.is_baby());
        if entity.is_hoglin() && !baby {
            let piglin =
                hoglin_brain::find(others, target).is_some_and(|l| l.kind == "minecraft:piglin");
            if piglin && entity.brain.memories.piglins_outnumber_hoglins(false) {
                entity.brain.set_avoid_target(target, &mut entity.random);
                self.broadcast_hoglin_retreat(id, target, others);
            } else {
                self.broadcast_hoglin_attack(id, target, others);
            }
        }
        let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let damage = hoglin::damage_roll(entity.hoglin.attack_damage(), baby, &mut entity.random);
        match target {
            Target::Player(player_id) => {
                let Some(at) = hoglin_brain::find(others, target).map(|l| l.position) else {
                    return;
                };
                let throw = if baby {
                    DVec3::ZERO
                } else {
                    hoglin::throw_push(position, at, 0.0, &mut entity.random)
                };
                self.player_hits.push(PlayerHit {
                    player_id,
                    damage,
                    kind: PlayerHitKind::Melee {
                        attacker: position,
                        hunger_ticks: 0,
                        lift: 0.0,
                        throw,
                    },
                    source: Some(id),
                });
            }
            Target::Mob(victim) | Target::Villager(victim) => {
                let hit = self.mob_hits_mob(id, victim, damage, position, 0.0);
                if baby || !hit.is_some_and(|hit| hit.applied) {
                    return;
                }
                let resistance = self.knockback_resistance(victim);
                let Some(at) = self.body_mut(victim).map(|body| body.position) else {
                    return;
                };
                let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) else {
                    return;
                };
                let push = hoglin::throw_push(position, at, resistance, &mut entity.random);
                if let Some(body) = self.body_mut(victim) {
                    body.velocity += push;
                    body.needs_sync = true;
                }
            }
        }
    }

    pub(super) fn tick_hoglin(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let game_time = self.game_time;
        let others = self.hoglin_living(id, players);
        let lovers: Vec<Target> = self
            .hoglins
            .iter()
            .filter(|e| {
                e.id != id
                    && e.hoglin.kind == HoglinKind::Hoglin
                    && e.hoglin.health > 0.0
                    && e.hoglin.in_love > 0
            })
            .map(|e| Target::Mob(e.id))
            .collect();
        let hurts = match self.hoglins.iter_mut().find(|e| e.id == id) {
            Some(entity) => std::mem::take(&mut entity.hurt_by),
            None => return,
        };
        for attacker in hurts {
            self.hoglin_was_hurt_by(id, attacker, &others, world);
        }
        let zombify = self.piglins_zombify;
        let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let (stepped, thought) =
            entity.tick_body(world, players, &others, &lovers, game_time, zombify);
        if !stepped {
            return;
        }
        if let Some(Target::Mob(partner)) = thought.courting {
            if let Some(e) = self.hoglins.iter_mut().find(|e| e.id == partner) {
                e.brain.court(Target::Mob(id));
            }
        }
        if let Some(Target::Mob(partner)) = thought.birth {
            self.breed_hoglins(id, partner);
        }
        if let Some(target) = thought.attack {
            self.hoglin_hits(id, target, &others);
        }
        if thought.convert {
            self.convert_hoglin_to_zoglin(id);
            return;
        }
        self.push_entities(EntityKey::Hoglin(id), world, game_time, ticks);
        let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entity.hoglin.health > 0.0 {
            entity.tick_age();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoglin::HoglinKind;
    use minecraftoss_player::{Block, Pos};
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Blocks {
        blocks: BTreeMap<Pos, Block>,
    }

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.blocks.get(&pos).cloned()
        }
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.blocks.insert(pos, block),
                None => self.blocks.remove(&pos),
            };
        }
    }

    fn floor() -> Blocks {
        wide_floor(8)
    }

    fn wide_floor(radius: i32) -> Blocks {
        let mut world = Blocks::default();
        for x in -radius..=radius {
            for z in -radius..=radius {
                world
                    .blocks
                    .insert((x, 63, z), Block::new("minecraft:netherrack"));
            }
        }
        world
    }

    fn spawn(world: &mut EntityWorld, kind: HoglinKind, baby: bool, x: f64) -> u64 {
        let mut hoglin = Hoglin::new(kind, DVec3::new(x, 64.0, 0.5));
        hoglin.set_baby(baby);
        world.spawn_hoglin(hoglin, false)
    }

    fn attack(damage: f64) -> PlayerAttack {
        PlayerAttack {
            player_id: 7,
            position: DVec3::new(0.5, 64.0, 3.5),
            yaw: 180.0,
            attack_damage: damage,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        }
    }

    #[test]
    fn a_hoglin_and_a_zoglin_have_forty_health_and_vanilla_sizes() {
        let mut world = EntityWorld::default();
        let hoglin = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
        let zoglin = spawn(&mut world, HoglinKind::Zoglin, false, 4.5);
        let piglet = spawn(&mut world, HoglinKind::Hoglin, true, -4.5);
        for id in [hoglin, zoglin] {
            let e = &world.hoglin_mut(id).unwrap().hoglin;
            assert_eq!(e.health, 40.0);
            assert_eq!((e.body.width, e.body.height), (1.3964844, 1.4));
            assert!((e.eye_height() - 1.19).abs() < 1.0e-6);
        }
        let e = &world.hoglin_mut(piglet).unwrap().hoglin;
        assert_eq!((e.body.width, e.body.height), (0.75, 0.85));
        assert_eq!(e.eye_height(), 0.625);
        assert_eq!(world.entity_type(hoglin), Some("minecraft:hoglin"));
        assert_eq!(world.entity_type(zoglin), Some("minecraft:zoglin"));
        let candidates = world.mob_candidates();
        let baby = candidates.iter().find(|c| c.id == piglet).unwrap();
        assert!(baby.baby && baby.kind == "minecraft:hoglin");
        assert!(!candidates.iter().find(|c| c.id == hoglin).unwrap().baby);
    }

    #[test]
    fn a_baby_hoglin_drops_experience_but_no_loot_and_a_baby_zoglin_drops_both() {
        let mut world = EntityWorld::default();
        let piglet = spawn(&mut world, HoglinKind::Hoglin, true, 0.5);
        assert!(world.player_attack(&attack(100.0), piglet).died);
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        assert_eq!((deaths[0].table, deaths[0].experience), (None, 3));
        let zoglin = spawn(&mut world, HoglinKind::Zoglin, true, 0.5);
        assert!(world.player_attack(&attack(100.0), zoglin).died);
        let deaths = world.take_deaths();
        assert_eq!(
            (deaths[0].table, deaths[0].experience),
            (Some("minecraft:zoglin"), 5)
        );
        let adult = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
        assert!(world.player_attack(&attack(100.0), adult).died);
        let deaths = world.take_deaths();
        assert_eq!(
            (deaths[0].table, deaths[0].experience),
            (Some("minecraft:hoglin"), 5)
        );
    }

    #[test]
    fn a_burning_hoglin_drops_cooked_porkchops() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let mut book = crate::loot::EntityLootBook::from_jar(&jar, 0).expect("the loot tables");
        let mut world = EntityWorld::default();
        let (mut raw, mut cooked, mut leather) = (0, 0, 0);
        for burning in [false, true] {
            for _ in 0..40 {
                let id = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
                if burning {
                    world.hoglin_mut(id).unwrap().hoglin.body.fire_ticks = 100;
                }
                assert!(world.player_attack(&attack(100.0), id).died);
                let deaths = world.take_deaths();
                let drops = book.death_drops(deaths);
                let count = |item: &str| {
                    drops
                        .iter()
                        .filter(|(s, _)| s.id == item)
                        .map(|(s, _)| u32::from(s.count))
                        .sum::<u32>()
                };
                let meat = count("minecraft:porkchop") + count("minecraft:cooked_porkchop");
                assert!((2..=4).contains(&meat), "{drops:?}");
                assert!(count("minecraft:leather") <= 1);
                if burning {
                    assert_eq!(count("minecraft:porkchop"), 0);
                    cooked += count("minecraft:cooked_porkchop");
                } else {
                    assert_eq!(count("minecraft:cooked_porkchop"), 0);
                    raw += count("minecraft:porkchop");
                }
                leather += count("minecraft:leather");
                world.remove_mobs(&[id]);
            }
        }
        assert!(raw > 0 && cooked > 0 && leather > 0);
    }

    #[test]
    fn hoglins_and_zoglins_leave_on_peaceful() {
        let mut world = EntityWorld::default();
        let hoglin = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
        let zoglin = spawn(&mut world, HoglinKind::Zoglin, false, 4.5);
        world
            .hoglin_mut(zoglin)
            .unwrap()
            .hoglin
            .persistence_required = true;
        let player = [DVec3::new(0.5, 64.0, 2.5)];
        assert_eq!(world.check_despawn(&player, false, &|_| true), 0);
        let far = [DVec3::new(0.5, 64.0, 200.5)];
        assert_eq!(world.check_despawn(&far, false, &|_| true), 1);
        assert!(world.hoglin_mut(hoglin).is_none());
        assert!(world.hoglin_mut(zoglin).is_some(), "persistent");
        assert_eq!(world.check_despawn(&player, true, &|_| true), 1);
        assert!(world.hoglins().is_empty());
    }

    #[test]
    fn a_zoglin_shrugs_off_lava_and_a_hoglin_burns() {
        let mut world = floor();
        for x in -4..=4 {
            world
                .blocks
                .insert((x, 64, 0), Block::new("minecraft:lava").with("level", "0"));
        }
        let mut entities = EntityWorld::default();
        let hoglin = spawn(&mut entities, HoglinKind::Hoglin, false, -2.5);
        let zoglin = spawn(&mut entities, HoglinKind::Zoglin, false, 2.5);
        for _ in 0..40 {
            entities.tick_with_players(&mut world, &[]);
        }
        let burnt = &entities.hoglin_mut(hoglin).unwrap().hoglin;
        assert!(burnt.health < 40.0 && burnt.body.fire_ticks > 0);
        let zoglin = &entities.hoglin_mut(zoglin).unwrap().hoglin;
        assert_eq!((zoglin.health, zoglin.body.fire_ticks), (40.0, 0));
        assert!(crate::world::fire_immune("minecraft:zoglin"));
        assert!(!crate::world::fire_immune("minecraft:hoglin"));
    }

    #[test]
    fn a_hoglin_takes_four_tenths_of_the_knockback() {
        let mut world = EntityWorld::default();
        let hoglin = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
        world.hoglin_mut(hoglin).unwrap().hoglin.body.on_ground = true;
        assert!(world.player_attack(&attack(1.0), hoglin).hurt);
        let v = world.hoglin_mut(hoglin).unwrap().hoglin.body.velocity;
        let expected = f64::from(0.4_f32) * (1.0 - crate::hoglin::KNOCKBACK_RESISTANCE);
        assert!((v.z + expected).abs() < 1.0e-9, "{v}");
        assert!((v.y - expected).abs() < 1.0e-9, "{v}");
    }

    #[test]
    fn an_iron_golem_goes_for_a_hoglin() {
        let mut world = floor();
        let mut entities = EntityWorld::default();
        let hoglin = spawn(&mut entities, HoglinKind::Hoglin, false, 0.5);
        let zoglin = spawn(&mut entities, HoglinKind::Zoglin, false, -5.5);
        let golem = crate::iron_golem::IronGolem::new(DVec3::new(5.5, 64.0, 0.5));
        let golem = entities.spawn_iron_golem(golem, false);
        entities.iron_golem_mut(golem).unwrap().random = LegacyRandom::new(5);
        for _ in 0..400 {
            entities.tick_with_players(&mut world, &[]);
        }
        let hurt = |entities: &mut EntityWorld, id| {
            entities
                .hoglin_mut(id)
                .is_none_or(|e| e.hoglin.health < crate::hoglin::MAX_HEALTH)
        };
        assert!(hurt(&mut entities, hoglin) || hurt(&mut entities, zoglin));
        assert!(crate::monster_ai::is_enemy("minecraft:hoglin"));
        assert!(crate::monster_ai::is_enemy("minecraft:zoglin"));
    }

    fn player(id: u64, position: DVec3) -> PlayerCandidate {
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

    fn seeded(world: &mut EntityWorld, kind: HoglinKind, baby: bool, at: DVec3, seed: u64) -> u64 {
        let mut hoglin = Hoglin::new(kind, at);
        hoglin.set_baby(baby);
        let id = world.spawn_hoglin(hoglin, false);
        world.hoglin_mut(id).unwrap().set_random_seed(seed);
        id
    }

    fn slowed(world: &mut EntityWorld, id: u64) {
        let slowness = crate::effects::EffectInstance::new(MobEffect::Slowness, 10_000, 9);
        world.hoglin_mut(id).unwrap().effects.add(slowness);
    }

    fn hit_ticks(baby: bool) -> Vec<(i32, PlayerHit)> {
        let mut blocks = wide_floor(12);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let hoglin = seeded(&mut world, HoglinKind::Hoglin, baby, at, 3);
        let players = [player(1, DVec3::new(0.5, 64.0, 2.3))];
        let mut hits = Vec::new();
        for tick in 0..400 {
            world.tick_with_players(&mut blocks, &players);
            for hit in world.take_player_hits() {
                assert_eq!(hit.source, Some(hoglin));
                hits.push((tick, hit));
            }
        }
        hits
    }

    fn gaps(hits: &[(i32, PlayerHit)]) -> Vec<i32> {
        hits.windows(2).map(|w| w[1].0 - w[0].0).collect()
    }

    #[test]
    fn an_adult_hoglin_hits_for_three_to_eight_every_two_seconds_and_a_baby_for_half_a_heart_every_fifteen_ticks(
    ) {
        let adult = hit_ticks(false);
        assert!(adult.len() >= 5, "{}", adult.len());
        for (_, hit) in &adult {
            assert!((3.0..=8.0).contains(&hit.damage), "{}", hit.damage);
            let PlayerHitKind::Melee { throw, .. } = hit.kind else {
                panic!("a headbutt")
            };
            assert!((0.0..=0.5).contains(&throw.y), "{throw}");
        }
        assert!(gaps(&adult).iter().all(|&gap| gap == 41), "{:?}", gaps(&adult));
        let baby = hit_ticks(true);
        assert!(baby.len() >= 10, "{}", baby.len());
        for (_, hit) in &baby {
            assert_eq!(hit.damage, 0.5);
            let PlayerHitKind::Melee { throw, .. } = hit.kind else {
                panic!("a nibble")
            };
            assert_eq!(throw, DVec3::ZERO);
        }
        assert!(gaps(&baby).iter().all(|&gap| gap == 16), "{:?}", gaps(&baby));
    }

    #[test]
    fn a_hoglin_throw_lifts_up_to_half_a_block_per_tick_and_never_lifts_an_iron_golem() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let hoglin = seeded(&mut world, HoglinKind::Hoglin, false, at, 7);
        let golem = crate::iron_golem::IronGolem::new(DVec3::new(0.5, 64.0, 2.0));
        let golem = world.spawn_iron_golem(golem, true);
        let others = world.hoglin_living(hoglin, &[]);
        world.hoglin_hits(hoglin, Target::Mob(golem), &others);
        let golem = world.iron_golem_mut(golem).unwrap();
        assert!(golem.golem.health < 100.0);
        assert_eq!(golem.golem.body.velocity, DVec3::ZERO);
        let mut lifts = Vec::new();
        for _ in 0..40 {
            let cow = crate::cow::Cow::new(DVec3::new(2.5, 64.0, 0.5));
            let cow = world.spawn_cow(cow, true);
            world.cow_mut(cow).unwrap().cow.body.on_ground = false;
            let others = world.hoglin_living(hoglin, &[]);
            world.hoglin_hits(hoglin, Target::Mob(cow), &others);
            let v = world.cow_mut(cow).unwrap().cow.body.velocity;
            assert!((0.0..=0.5).contains(&v.y), "{v}");
            lifts.push(v.y);
            world.remove_mobs(&[cow]);
        }
        assert!(lifts.iter().any(|&y| y > 0.25), "{lifts:?}");
    }

    #[test]
    fn a_hoglin_ignores_a_creative_player_and_one_seventeen_blocks_away() {
        let mut blocks = wide_floor(20);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let hoglin = seeded(&mut world, HoglinKind::Hoglin, false, at, 5);
        slowed(&mut world, hoglin);
        let mut creative = player(1, DVec3::new(0.5, 64.0, 3.5));
        creative.attackable = false;
        let far = player(2, DVec3::new(17.5, 64.0, 0.5));
        for _ in 0..60 {
            world.tick_with_players(&mut blocks, &[creative, far]);
        }
        assert!(world.take_player_hits().is_empty());
        let memories = &world.hoglin_mut(hoglin).unwrap().brain.memories;
        assert!(!memories.attack_target.present());
        let near = player(3, DVec3::new(10.5, 64.0, 0.5));
        for _ in 0..30 {
            world.tick_with_players(&mut blocks, &[creative, far, near]);
        }
        let brain = &world.hoglin_mut(hoglin).unwrap().brain;
        assert_eq!(brain.memories.attack_target.get(), Some(&Target::Player(3)));
        assert_eq!(brain.activity, crate::hoglin_brain::Activity::Fight);
    }

    #[test]
    fn hitting_one_hoglin_sets_every_visible_adult_on_the_attacker() {
        let mut blocks = wide_floor(12);
        let mut world = EntityWorld::default();
        let spots = [
            (0.5, 0.5, false),
            (3.5, 0.5, false),
            (0.5, 3.5, false),
            (-3.5, 0.5, true),
        ];
        let mut ids = Vec::new();
        for (i, &(x, z, baby)) in spots.iter().enumerate() {
            let at = DVec3::new(x, 64.0, z);
            let id = seeded(&mut world, HoglinKind::Hoglin, baby, at, i as u64);
            slowed(&mut world, id);
            ids.push(id);
        }
        let cow = crate::cow::Cow::new(DVec3::new(0.5, 64.0, -5.5));
        let cow = world.spawn_cow(cow, true);
        for _ in 0..25 {
            world.tick_with_players(&mut blocks, &[]);
        }
        world.note_hoglin_hurt(ids[0], Target::Mob(cow));
        world.tick_with_players(&mut blocks, &[]);
        for &id in &ids[..3] {
            let memories = &world.hoglin_mut(id).unwrap().brain.memories;
            assert_eq!(memories.attack_target.get(), Some(&Target::Mob(cow)), "{id}");
        }
        let piglet = &world.hoglin_mut(ids[3]).unwrap().brain.memories;
        assert!(!piglet.attack_target.present());
    }

    #[test]
    fn a_hurt_piglet_runs_away_for_five_to_twenty_seconds() {
        let mut blocks = wide_floor(20);
        let mut ticks = Vec::new();
        for seed in 0..12 {
            let mut world = EntityWorld::default();
            let at = DVec3::new(0.5, 64.0, 0.5);
            let piglet = seeded(&mut world, HoglinKind::Hoglin, true, at, seed);
            let players = [player(7, DVec3::new(0.5, 64.0, 3.5))];
            for _ in 0..25 {
                world.tick_with_players(&mut blocks, &players);
            }
            assert!(world.player_attack(&attack(0.5), piglet).hurt);
            world.tick_with_players(&mut blocks, &players);
            let brain = &world.hoglin_mut(piglet).unwrap().brain;
            assert_eq!(brain.memories.avoid_target.get(), Some(&Target::Player(7)));
            assert!(!brain.memories.attack_target.present());
            assert_eq!(brain.activity, crate::hoglin_brain::Activity::Avoid);
            ticks.push(brain.memories.avoid_target.ttl().unwrap());
            let from = players[0].position;
            let start = world.hoglin_mut(piglet).unwrap().position().distance(from);
            for _ in 0..60 {
                world.tick_with_players(&mut blocks, &players);
            }
            let after = world.hoglin_mut(piglet).unwrap().position().distance(from);
            assert!(after > start + 2.0, "{start} {after}");
        }
        assert!(ticks.iter().all(|t| (98..=400).contains(t)), "{ticks:?}");
        assert!(ticks.iter().any(|&t| t < 250), "{ticks:?}");
        assert!(ticks.iter().any(|&t| t > 250), "{ticks:?}");
    }

    #[test]
    fn a_retaliation_target_is_forgotten_after_two_hundred_ticks() {
        let mut blocks = wide_floor(12);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let hoglin = seeded(&mut world, HoglinKind::Hoglin, false, at, 9);
        slowed(&mut world, hoglin);
        let cow = crate::cow::Cow::new(DVec3::new(0.5, 64.0, 8.5));
        let cow = world.spawn_cow(cow, true);
        world.note_hoglin_hurt(hoglin, Target::Mob(cow));
        for tick in 0..200 {
            world.tick_with_players(&mut blocks, &[]);
            let memories = &world.hoglin_mut(hoglin).unwrap().brain.memories;
            assert!(memories.attack_target.present(), "{tick}");
        }
        world.tick_with_players(&mut blocks, &[]);
        let brain = &world.hoglin_mut(hoglin).unwrap().brain;
        assert!(!brain.memories.attack_target.present());
        assert_eq!(brain.activity, crate::hoglin_brain::Activity::Idle);
    }

    fn calmed_by(repellent: Block) {
        let mut blocks = wide_floor(16);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let hoglin = seeded(&mut world, HoglinKind::Hoglin, false, at, 2);
        let players = [player(1, DVec3::new(6.5, 64.0, 0.5))];
        for _ in 0..30 {
            world.tick_with_players(&mut blocks, &players);
        }
        let memories = &world.hoglin_mut(hoglin).unwrap().brain.memories;
        assert!(memories.attack_target.present());
        let near = world.hoglin_mut(hoglin).unwrap().position() + DVec3::new(-3.0, 0.0, 0.0);
        let spot = (near.x.floor() as i32, 64, near.z.floor() as i32);
        blocks.blocks.insert(spot, repellent);
        for _ in 0..25 {
            world.tick_with_players(&mut blocks, &players);
        }
        world.take_player_hits();
        for _ in 0..75 {
            world.tick_with_players(&mut blocks, &players);
        }
        let memories = &world.hoglin_mut(hoglin).unwrap().brain.memories;
        assert!(memories.pacified.present());
        assert!(!memories.attack_target.present());
        assert!(world.take_player_hits().is_empty());
        blocks.blocks.remove(&spot);
        for _ in 0..40 {
            world.tick_with_players(&mut blocks, &players);
        }
        let memories = &world.hoglin_mut(hoglin).unwrap().brain.memories;
        assert!(!memories.nearest_repellent.present());
        assert!(memories.pacified.present() && !memories.attack_target.present());
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &players);
        }
        let memories = &world.hoglin_mut(hoglin).unwrap().brain.memories;
        assert!(!memories.pacified.present());
        assert_eq!(memories.attack_target.get(), Some(&Target::Player(1)));
    }

    #[test]
    fn a_respawn_anchor_or_warped_fungus_within_eight_blocks_calms_a_hoglin_for_ten_seconds() {
        calmed_by(Block::new("minecraft:respawn_anchor").with("charges", "0"));
        calmed_by(Block::new("minecraft:warped_fungus"));
    }

    #[test]
    fn a_calm_hoglin_walks_away_from_the_repellent_and_strolls_back_to_crimson_nylium() {
        let mut blocks = wide_floor(16);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let hoglin = seeded(&mut world, HoglinKind::Hoglin, false, at, 6);
        let anchor = (2, 64, 0);
        blocks.blocks.insert(anchor, Block::new("minecraft:respawn_anchor"));
        for _ in 0..120 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let away = world.hoglin_mut(hoglin).unwrap().position();
        assert!(away.distance(DVec3::new(2.5, 64.0, 0.5)) > 6.0, "{away}");
        let mut nylium = wide_floor(16);
        for x in -16..=16 {
            for z in 0..=16 {
                nylium.blocks.insert((x, 63, z), Block::new("minecraft:crimson_nylium"));
            }
        }
        let entity = world.hoglin_mut(hoglin).unwrap();
        let mut profile = entity.walk_profile();
        let mut random = LegacyRandom::new(1);
        let mut on_nylium = 0;
        for _ in 0..200 {
            profile.walk_target = crate::walk_path::WalkTarget::Hoglin { repellent: None };
            let from = DVec3::new(0.5, 64.0, 0.5);
            let spot = crate::stroll::land_random_position(&nylium, &profile, from, 10, 0, &mut random).unwrap();
            if spot.z > 0.0 {
                on_nylium += 1;
            }
        }
        assert!(on_nylium > 190, "{on_nylium}");
        let mut near_anchor = 0;
        for _ in 0..200 {
            profile.walk_target = crate::walk_path::WalkTarget::Hoglin { repellent: Some((0, 64, 6)) };
            let from = DVec3::new(0.5, 64.0, 0.5);
            let spot = crate::stroll::land_random_position(&nylium, &profile, from, 10, 0, &mut random).unwrap();
            if spot.distance(DVec3::new(0.5, 64.0, 6.5)) < 7.5 {
                near_anchor += 1;
            }
        }
        assert!(near_anchor < 20, "{near_anchor}");
    }

    #[test]
    fn outnumbered_hoglins_retreat_from_a_piglin_they_hit() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let hoglin = seeded(&mut world, HoglinKind::Hoglin, false, at, 1);
        let friend = seeded(&mut world, HoglinKind::Hoglin, false, DVec3::new(3.5, 64.0, 0.5), 2);
        let piglin = Target::Mob(9_999);
        let mut others = world.hoglin_living(hoglin, &[]);
        others.push(Living {
            target: piglin,
            kind: "minecraft:piglin",
            position: DVec3::new(0.5, 64.0, 2.0),
            eye_height: 1.79,
            width: 0.6,
            height: 1.95,
            alive: true,
            baby: false,
            attackable: true,
            spectator: false,
        });
        for (piglins, retreats) in [(2, false), (3, true)] {
            for id in [hoglin, friend] {
                let memories = &mut world.hoglin_mut(id).unwrap().brain.memories;
                memories.visible_adult_piglin_count.set(piglins);
                memories.visible_adult_hoglin_count.set(1);
                memories.avoid_target.erase();
                memories.attack_target.erase();
            }
            let memories = &mut world.hoglin_mut(hoglin).unwrap().brain.memories;
            memories.nearest_visible_adult_hoglins.set(vec![Target::Mob(friend)]);
            world.hoglin_hits(hoglin, piglin, &others);
            for id in [hoglin, friend] {
                let memories = &world.hoglin_mut(id).unwrap().brain.memories;
                assert_eq!(memories.avoid_target.present(), retreats, "{piglins} {id}");
            }
            let friend = &world.hoglin_mut(friend).unwrap().brain.memories;
            assert_eq!(friend.attack_target.present(), !retreats);
        }
    }

    #[test]
    fn a_piglet_follows_an_adult_from_six_to_sixteen_blocks() {
        let mut blocks = wide_floor(24);
        let mut world = EntityWorld::default();
        let adult = seeded(&mut world, HoglinKind::Hoglin, false, DVec3::new(0.5, 64.0, 0.5), 1);
        slowed(&mut world, adult);
        let piglet = seeded(&mut world, HoglinKind::Hoglin, true, DVec3::new(10.5, 64.0, 0.5), 2);
        for _ in 0..30 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let memories = &world.hoglin_mut(piglet).unwrap().brain.memories;
        assert_eq!(memories.nearest_visible_adult.get(), Some(&Target::Mob(adult)));
        let mut nearest = f64::MAX;
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &[]);
            let at = world.hoglin_mut(adult).unwrap().position();
            let distance = world.hoglin_mut(piglet).unwrap().position().distance(at);
            nearest = nearest.min(distance);
        }
        assert!(nearest < 6.5, "{nearest}");
        let mut world = EntityWorld::default();
        let adult = seeded(&mut world, HoglinKind::Hoglin, false, DVec3::new(0.5, 64.0, 0.5), 1);
        slowed(&mut world, adult);
        let far = seeded(&mut world, HoglinKind::Hoglin, true, DVec3::new(20.5, 64.0, 0.5), 2);
        for _ in 0..30 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let memories = &world.hoglin_mut(far).unwrap().brain.memories;
        assert!(!memories.nearest_visible_adult.present());
        let towards = memories.walk_target.get().map(|goal| goal.target);
        let adult_track = crate::hoglin_brain::Track::Entity { target: Target::Mob(adult), eyes: false };
        assert_ne!(towards, Some(adult_track));
    }

    #[test]
    fn a_grown_piglet_hits_for_three_to_eight() {
        let mut blocks = wide_floor(12);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let piglet = seeded(&mut world, HoglinKind::Hoglin, true, at, 4);
        world.hoglin_mut(piglet).unwrap().hoglin.age.ticks = -3;
        let players = [player(1, DVec3::new(0.5, 64.0, 2.3))];
        for _ in 0..5 {
            world.tick_with_players(&mut blocks, &players);
        }
        let grown = &world.hoglin_mut(piglet).unwrap().hoglin;
        assert!(!grown.is_baby());
        assert_eq!((grown.body.width, grown.body.height), (1.3964844, 1.4));
        assert_eq!(grown.experience(), 5);
        let mut hits = world.take_player_hits();
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &players);
            hits.extend(world.take_player_hits());
        }
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|hit| (3.0..=8.0).contains(&hit.damage)), "{hits:?}");
    }

    fn lone(baby: bool) -> (Blocks, EntityWorld, u64) {
        let blocks = wide_floor(10);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = seeded(&mut world, HoglinKind::Hoglin, baby, at, 3);
        (blocks, world, id)
    }

    fn zoglins(world: &EntityWorld) -> Vec<&HoglinEntity> {
        world
            .hoglins()
            .iter()
            .filter(|e| e.hoglin.kind == HoglinKind::Zoglin)
            .collect()
    }

    #[test]
    fn a_hoglin_in_the_overworld_becomes_a_zoglin_on_tick_301_and_one_in_the_nether_never_does() {
        let (mut blocks, mut world, id) = lone(false);
        world.set_piglins_zombify(true);
        world.hoglin_mut(id).unwrap().hoglin.health = 25.0;
        for _ in 0..300 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let hoglin = &world.hoglin_mut(id).unwrap().hoglin;
        assert!(hoglin.converting && hoglin.time_in_overworld == 300);
        world.take_sounds();
        world.tick_with_players(&mut blocks, &[]);
        assert!(world.hoglin_mut(id).is_none());
        let zoglins = zoglins(&world);
        assert_eq!(zoglins.len(), 1);
        let zoglin = zoglins[0];
        assert_ne!(zoglin.id, id);
        assert_eq!(zoglin.hoglin.health, 40.0);
        assert!(!zoglin.hoglin.converting && !zoglin.hoglin.is_baby());
        assert!(zoglin.effects.has(MobEffect::Nausea));
        assert_eq!(world.entity_type(zoglin.id), Some("minecraft:zoglin"));
        let sounds = world.take_sounds();
        assert!(sounds.iter().any(|s| s.event == "entity.hoglin.converted_to_zombified"));
        let (mut blocks, mut world, id) = lone(false);
        world.set_piglins_zombify(false);
        for _ in 0..400 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let hoglin = &world.hoglin_mut(id).unwrap().hoglin;
        assert!(!hoglin.converting && hoglin.time_in_overworld == 0);
    }

    #[test]
    fn going_back_to_the_nether_resets_the_clock() {
        let (mut blocks, mut world, id) = lone(false);
        world.set_piglins_zombify(true);
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &[]);
        }
        world.set_piglins_zombify(false);
        world.tick_with_players(&mut blocks, &[]);
        assert_eq!(world.hoglin_mut(id).unwrap().hoglin.time_in_overworld, 0);
        world.set_piglins_zombify(true);
        for _ in 0..300 {
            world.tick_with_players(&mut blocks, &[]);
        }
        assert!(world.hoglin_mut(id).is_some());
        world.tick_with_players(&mut blocks, &[]);
        assert!(world.hoglin_mut(id).is_none());
    }

    #[test]
    fn an_immune_or_no_ai_hoglin_stays() {
        let (mut blocks, mut world, immune) = lone(false);
        world.hoglin_mut(immune).unwrap().hoglin.immune_to_zombification = true;
        let still = world.spawn_hoglin(Hoglin::new(HoglinKind::Hoglin, DVec3::new(4.5, 64.0, 0.5)), true);
        world.set_piglins_zombify(true);
        for _ in 0..400 {
            world.tick_with_players(&mut blocks, &[]);
        }
        for id in [immune, still] {
            let hoglin = &world.hoglin_mut(id).unwrap().hoglin;
            assert!(!hoglin.converting && hoglin.time_in_overworld == 0, "{id}");
        }
        assert!(zoglins(&world).is_empty());
    }

    #[test]
    fn a_piglet_becomes_a_baby_zoglin_and_keeps_its_persistence() {
        let (mut blocks, mut world, id) = lone(true);
        world.hoglin_mut(id).unwrap().hoglin.persistence_required = true;
        world.set_piglins_zombify(true);
        for _ in 0..301 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let zoglins = zoglins(&world);
        assert_eq!(zoglins.len(), 1);
        let zoglin = &zoglins[0].hoglin;
        assert!(zoglin.is_baby() && zoglin.baby && zoglin.persistence_required);
        assert_eq!((zoglin.body.width, zoglin.body.height), (0.75, 0.85));
        assert_eq!(zoglin.attack_damage(), 0.5);
    }

    #[test]
    fn a_zoglin_goes_for_the_nearest_cow_villager_or_player_but_never_a_creeper_or_zoglin() {
        let mut blocks = wide_floor(16);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let zoglin = seeded(&mut world, HoglinKind::Zoglin, false, at, 3);
        slowed(&mut world, zoglin);
        let creeper = crate::creeper::Creeper::new(DVec3::new(2.5, 64.0, 0.5));
        let creeper = world.spawn_creeper(creeper, true);
        let other = world.spawn_hoglin(Hoglin::new(HoglinKind::Zoglin, DVec3::new(-2.5, 64.0, 0.5)), true);
        let cow = world.spawn_cow(crate::cow::Cow::new(DVec3::new(0.5, 64.0, 5.5)), true);
        let villager = crate::villager::Villager::new(DVec3::new(0.5, 64.0, -7.5));
        let villager = world.spawn_villager(villager, true);
        let players = [player(4, DVec3::new(9.5, 64.0, 0.5))];
        let mut chosen = Vec::new();
        for gone in [Some(cow), Some(villager), None] {
            for _ in 0..30 {
                world.tick_with_players(&mut blocks, &players);
            }
            let target = world.hoglin_mut(zoglin).unwrap().brain.memories.attack_target.get().copied();
            chosen.push(target);
            if let Some(gone) = gone {
                world.remove_mobs(&[gone]);
            }
        }
        assert_eq!(chosen, [Some(Target::Mob(cow)), Some(Target::Mob(villager)), Some(Target::Player(4))]);
        assert!(world.hoglin_mut(other).is_some());
        let creeper = world.mob_candidates().into_iter().find(|m| m.id == creeper).unwrap();
        assert!(creeper.alive);
        assert_eq!(world.hoglin_mut(zoglin).unwrap().brain.activity, crate::hoglin_brain::Activity::Fight);
    }

    #[test]
    fn a_zoglin_switches_to_a_closer_attacker_but_not_a_far_one() {
        let mut blocks = wide_floor(16);
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let zoglin = seeded(&mut world, HoglinKind::Zoglin, false, at, 5);
        slowed(&mut world, zoglin);
        let first = world.spawn_cow(crate::cow::Cow::new(DVec3::new(0.5, 64.0, 3.5)), true);
        let far = world.spawn_cow(crate::cow::Cow::new(DVec3::new(0.5, 64.0, 10.5)), true);
        let near = world.spawn_cow(crate::cow::Cow::new(DVec3::new(2.5, 64.0, 0.5)), true);
        world.note_hoglin_hurt(zoglin, Target::Mob(first));
        world.tick_with_players(&mut blocks, &[]);
        let target = |world: &mut EntityWorld| world.hoglin_mut(zoglin).unwrap().brain.memories.attack_target.get().copied();
        assert_eq!(target(&mut world), Some(Target::Mob(first)));
        world.note_hoglin_hurt(zoglin, Target::Mob(far));
        world.tick_with_players(&mut blocks, &[]);
        assert_eq!(target(&mut world), Some(Target::Mob(first)), "ten blocks is more than four further");
        world.note_hoglin_hurt(zoglin, Target::Mob(near));
        world.tick_with_players(&mut blocks, &[]);
        assert_eq!(target(&mut world), Some(Target::Mob(near)));
        let ttl = world.hoglin_mut(zoglin).unwrap().brain.memories.attack_target.ttl();
        assert!(ttl.is_some_and(|ttl| ttl > 190));
    }

    #[test]
    fn a_zoglin_kills_a_hoglin_which_fights_back() {
        let mut blocks = wide_floor(16);
        let mut world = EntityWorld::default();
        let zoglin = seeded(&mut world, HoglinKind::Zoglin, false, DVec3::new(0.5, 64.0, 0.5), 8);
        let hoglin = seeded(&mut world, HoglinKind::Hoglin, false, DVec3::new(3.5, 64.0, 0.5), 9);
        world.hoglin_mut(hoglin).unwrap().hoglin.health = 12.0;
        let mut fought_back = false;
        for _ in 0..600 {
            world.tick_with_players(&mut blocks, &[]);
            if let Some(e) = world.hoglin_mut(hoglin) {
                fought_back |= e.brain.memories.attack_target.get() == Some(&Target::Mob(zoglin));
            }
        }
        assert!(fought_back);
        assert!(world.hoglin_mut(hoglin).is_none_or(|e| e.hoglin.health <= 0.0));
        let zoglin = &world.hoglin_mut(zoglin).unwrap().hoglin;
        assert!(zoglin.health > 0.0 && zoglin.health < 40.0);
    }

    #[test]
    fn a_zoglin_throws_its_victims_but_not_an_iron_golem() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let zoglin = seeded(&mut world, HoglinKind::Zoglin, false, at, 7);
        let golem = crate::iron_golem::IronGolem::new(DVec3::new(0.5, 64.0, 2.0));
        let golem = world.spawn_iron_golem(golem, true);
        let others = world.hoglin_living(zoglin, &[]);
        world.hoglin_hits(zoglin, Target::Mob(golem), &others);
        assert_eq!(world.iron_golem_mut(golem).unwrap().golem.body.velocity, DVec3::ZERO);
        let cow = world.spawn_cow(crate::cow::Cow::new(DVec3::new(2.5, 64.0, 0.5)), true);
        world.cow_mut(cow).unwrap().cow.body.on_ground = false;
        let others = world.hoglin_living(zoglin, &[]);
        world.hoglin_hits(zoglin, Target::Mob(cow), &others);
        let v = world.cow_mut(cow).unwrap().cow.body.velocity;
        assert!(v.x.abs() + v.z.abs() > 0.1 && (0.0..=0.5).contains(&v.y), "{v}");
        let players = [player(2, DVec3::new(0.5, 64.0, 2.0))];
        let others = world.hoglin_living(zoglin, &players);
        world.hoglin_hits(zoglin, Target::Player(2), &others);
        let hits = world.take_player_hits();
        let PlayerHitKind::Melee { throw, .. } = hits[0].kind else {
            panic!("a headbutt")
        };
        assert!((3.0..=8.0).contains(&hits[0].damage) && throw != DVec3::ZERO);
        let sounds: Vec<String> = world.take_sounds().into_iter().map(|s| s.event).collect();
        assert!(!sounds.iter().any(|s| s.starts_with("entity.hoglin")), "{sounds:?}");
    }
}
