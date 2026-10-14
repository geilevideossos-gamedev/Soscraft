use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::monster_ai::{MobCandidate, MonsterKind, Target};
use crate::ravager::{self as rv, Blocked, Ravager};

const SOUNDS: MovementSounds = MovementSounds::monster(Some(rv::STEP_SOUND));

#[derive(Clone)]
pub struct RavagerEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub ravager: Ravager,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
    pub stun_particles: Vec<DVec3>,
}

fn adult_pitch(random: &mut LegacyRandom) -> f32 {
    let (a, b) = (random.next_float(), random.next_float());
    (a - b) * 0.2 + 1.0
}

fn boxes_meet(a: (DVec3, DVec3), b: (DVec3, DVec3)) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
}

fn body_box(body: &Body) -> (DVec3, DVec3) {
    let p = body.position;
    let half = f64::from(body.width / 2.0);
    (
        DVec3::new(p.x - half, p.y, p.z - half),
        DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
    )
}

impl RavagerEntity {
    pub fn position(&self) -> DVec3 {
        self.ravager.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.ravager.health > 0.0 && !self.ravager.damage.dead {
            self.no_action_time = 0;
        }
        let result =
            self.ravager
                .damage
                .hurt_generic(&mut self.ravager.health, rv::MAX_HEALTH, amount);
        let position = self.position();
        self.ravager
            .damage
            .place_death(result, position, self.ravager.body.fire_ticks > 0);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -rv::AMBIENT_INTERVAL;
            }
            let voice = hurt_voice(&mut self.random, result.died, false);
            self.voices.push((voice, position));
        }
        result
    }

    fn sync_ai(&mut self) {
        let state = &mut self.ai.state.illager;
        state.id = self.id;
        state.patrolling = self.ravager.raider.patrolling;
        state.leader = self.ravager.raider.patrol_leader;
        state.patrol_target = self.ravager.raider.patrol_target;
        state.has_raid = state.raid.is_some_and(|raid| raid.active);
        state.companions.clear();
    }

    fn tick_ai(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        villagers: &[MobCandidate],
        game_time: i64,
        difficulty: i32,
    ) -> Option<Target> {
        let position = self.ravager.body.position;
        if players
            .iter()
            .any(|p| p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0)
        {
            self.no_action_time = 0;
        }
        self.sync_ai();
        let health = self.ravager.health;
        let immobile = self.ravager.is_immobile();
        let speed = self.effects.movement_speed(self.ravager.speed);
        let ai = &mut self.ai;
        let body = &mut self.ravager.body;
        self.no_action_time += 2;
        if ai.no_jump_delay > 0 {
            ai.no_jump_delay -= 1;
        }
        body.trim_small_velocity();
        let fluid = FluidFrame::sample(world, position, body.width, body.height);
        let mut strike = None;
        if immobile {
            ai.jumping = false;
            ai.forward = 0.0;
            ai.sideways = 0.0;
        } else {
            self.no_action_time += 1;
            ai.state.body = body.clone();
            ai.state.health = health;
            ai.state.max_health = rv::MAX_HEALTH;
            ai.state.game_time = game_time;
            ai.state.difficulty = difficulty;
            ai.state.players = players.to_vec();
            ai.state.villagers = villagers.to_vec();
            ai.state.fluid = fluid;
            ai.state.no_action_time = self.no_action_time;
            ai.state.tick_count = self.tick_count;
            ai.state.random = std::mem::take(&mut self.random);
            let full = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
            ai.tick_goals(world, full);
            strike = ai.state.attack.take();
            body.position = ai.state.body.position;
            let position = body.position;
            let (can_update, surface) =
                crate::navigation::ground_view(world, body, fluid, ai.state.walk.can_float);
            if let Some((wanted, speed)) = ai
                .state
                .navigation
                .tick_in(world, position, can_update, surface, body.width, ai.speed)
            {
                ai.move_control.set_wanted_position(wanted, speed);
            }
            self.random = std::mem::take(&mut ai.state.random);
            let obstacle_top = crate::control::obstacle_top(world, position);
            let control = ai.move_control.tick(
                position,
                body.on_ground,
                ai.yaw,
                ai.speed,
                ai.forward,
                ai.sideways,
                speed,
                body.width,
                body.step_height,
                obstacle_top,
                |_, _| true,
            );
            ai.yaw = control.yaw;
            ai.speed = control.speed;
            ai.forward = control.forward;
            ai.sideways = control.sideways;
            ai.state.look_control.tick_limited(
                position,
                rv::EYE_HEIGHT,
                ai.body_rotation.body_yaw,
                !ai.state.navigation.is_done(),
                rv::MAX_HEAD_Y_ROT,
            );
            ai.jumping = ai.state.jump || control.jump;
        }
        if strike.is_some() {
            self.ravager.attack_tick = rv::ATTACK_DURATION;
            self.voices
                .push((Voice::Event(rv::ATTACK_SOUND, 1.0, 1.0), body.position));
        }
        body.living_jump(world, fluid, ai.jumping, &mut ai.no_jump_delay, 0.4);
        let input = DVec3::new(f64::from(ai.sideways), 0.0, f64::from(ai.forward));
        let mut landed = None;
        if fluid.in_water() {
            body.travel_water(world, input, ai.yaw);
        } else if fluid.in_lava() {
            body.travel_lava(world, input, ai.yaw, fluid.lava_height);
        } else {
            landed = body.travel_air_jumping(world, input, ai.speed, ai.yaw, ai.jumping);
        }
        play_movement(
            body,
            self.tick_count,
            &mut self.random,
            &mut self.voices,
            SOUNDS,
        );
        ai.body_rotation.tick_limited(
            ai.yaw,
            &mut ai.state.look_control,
            self.previous_position,
            body.position,
            rv::MAX_HEAD_Y_ROT,
        );
        self.ravager.yaw = ai.yaw;
        self.ravager.raider.patrolling = ai.state.illager.patrolling;
        self.ravager.raider.patrol_target = ai.state.illager.patrol_target;
        if let Some(damage) = landed.and_then(|fallen| {
            super::living::fall_damage(&self.ravager.body, world, fallen, true, &mut self.voices)
        }) {
            self.hurt(damage);
        }
        strike
    }

    fn break_leaves(&mut self, world: &mut impl World) {
        let mut broke = false;
        for cell in self.ravager.leaf_cells() {
            if world
                .block(cell)
                .is_some_and(|block| rv::is_leaves(&block.id))
            {
                broke |= world.destroy_block(cell);
            }
        }
        let body = &mut self.ravager.body;
        if !broke && body.on_ground {
            let power = body.jump_power(world, 0.42, 1.0);
            if power > 1.0e-5 {
                body.velocity.y = f64::from(power).max(body.velocity.y);
                body.needs_sync = true;
            }
        }
    }

    fn tick_timers(&mut self) -> bool {
        let timers = self.ravager.tick_timers();
        self.stun_particles.clear();
        if timers.stun_effect {
            let body_yaw = self.ai.body_rotation.body_yaw;
            if let Some(at) = self.ravager.stun_particle(&mut self.random, body_yaw) {
                self.stun_particles.push(at);
            }
        }
        if timers.roar_sound {
            self.voices
                .push((Voice::Event(rv::ROAR_SOUND, 1.0, 1.0), self.position()));
        }
        timers.roars
    }
}

impl EntityWorld {
    pub fn spawn_ravager(&mut self, ravager: Ravager, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut ai = MonsterAi::of_kind(MonsterKind::Ravager, &ravager.body, ravager.yaw);
        ai.state.max_health = rv::MAX_HEALTH;
        ai.state.illager.id = id;
        self.ravagers.push(RavagerEntity {
            effects: Default::default(),
            id,
            previous_position: ravager.body.position,
            ravager,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
            stun_particles: Vec::new(),
        });
        self.order.push(EntityKey::Ravager(id));
        self.file_in_section(EntityKey::Ravager(id));
        id
    }

    pub fn ravagers(&self) -> &[RavagerEntity] {
        &self.ravagers
    }

    pub fn ravager_mut(&mut self, id: u64) -> Option<&mut RavagerEntity> {
        self.ravagers.iter_mut().find(|entity| entity.id == id)
    }

    pub fn take_player_pushes(&mut self) -> Vec<(u64, DVec3)> {
        std::mem::take(&mut self.player_pushes)
    }

    pub fn blocked_by_shield(
        &mut self,
        attacker: u64,
        player: u64,
        feet: DVec3,
    ) -> Vec<(u64, DVec3)> {
        let carries = !self.passengers_of(attacker).is_empty();
        let Some(entity) = self
            .ravagers
            .iter_mut()
            .find(|e| e.id == attacker && e.ravager.health > 0.0)
        else {
            return Vec::new();
        };
        let position = entity.position();
        match entity.ravager.blocked_by_item(&mut entity.random) {
            Blocked::Ignored => Vec::new(),
            Blocked::Stunned => {
                entity
                    .voices
                    .push((Voice::Event(rv::STUNNED_SOUND, 1.0, 1.0), position));
                self.entity_events.push((attacker, 39));
                let Some((xa, za)) = super::pushing::push_step(feet, position) else {
                    return Vec::new();
                };
                if !carries {
                    entity.ravager.body.velocity += DVec3::new(-xa, 0.0, -za);
                    entity.ravager.body.needs_sync = true;
                }
                vec![(player, DVec3::new(xa, 0.0, za))]
            }
            Blocked::Thrown => vec![(player, rv::strong_knockback(position, feet))],
        }
    }

    fn ravager_steered(&self, id: u64) -> bool {
        let Some(entity) = self.ravagers.iter().find(|e| e.id == id) else {
            return false;
        };
        if entity.no_ai {
            return false;
        }
        match self.passengers_of(id).first() {
            Some(Rider::Mob(rider)) => self.entity_type(*rider).is_some_and(|kind| {
                !crate::illager::is_raider(kind)
                    && !super::riding::NON_CONTROLLING_RIDERS.contains(&kind)
            }),
            _ => false,
        }
    }

    fn ravager_strikes(&mut self, id: u64, target: Target) {
        let Some(entity) = self.ravagers.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let position = entity.position();
        let throw = entity.ravager.attack_throw();
        let yaw = entity.ravager.yaw;
        let velocity = entity.ravager.body.velocity;
        entity.ravager.body.velocity = DVec3::new(velocity.x * 0.6, velocity.y, velocity.z * 0.6);
        match target {
            Target::Player(player_id) => self.player_hits.push(PlayerHit {
                player_id,
                damage: rv::ATTACK_DAMAGE,
                kind: PlayerHitKind::Melee {
                    attacker: position,
                    hunger_ticks: 0,
                    lift: 0.0,
                    throw,
                    disable_shield_seconds: 0.0,
                },
                source: Some(id),
            }),
            Target::Mob(victim) | Target::Villager(victim) => {
                let hit = self.mob_hits_mob(id, victim, rv::ATTACK_DAMAGE, position, 0.0);
                if hit.is_some_and(|hit| hit.applied) {
                    let angle = f64::from(yaw * 0.017_453_292);
                    let (sin, cos) = (
                        f64::from(minecraftoss_player::mth::sin(angle)),
                        f64::from(minecraftoss_player::mth::cos(angle)),
                    );
                    self.knock_back(victim, rv::ATTACK_KNOCKBACK * 0.5, sin, -cos);
                }
            }
        }
    }

    fn ravager_roars(&mut self, id: u64, players: &[PlayerCandidate]) {
        let Some(entity) = self.ravagers.iter().find(|e| e.id == id) else {
            return;
        };
        if entity.ravager.health <= 0.0 {
            return;
        }
        let position = entity.position();
        let (low, high) = entity.ravager.bounds();
        let reach = DVec3::splat(rv::ROAR_RANGE);
        let area = (low - reach, high + reach);
        for key in self.order.clone() {
            let victim = key.id();
            if victim == id {
                continue;
            }
            let Some(kind) = self.entity_type(victim) else {
                continue;
            };
            if kind == rv::TYPE_ID {
                continue;
            }
            let Some((body, health)) = self.mob_body(victim) else {
                continue;
            };
            if health <= 0.0 || !boxes_meet(body_box(&body), area) {
                continue;
            }
            if rv::roar_hurts(kind) {
                self.mob_hits_mob(id, victim, rv::ROAR_DAMAGE, position, 0.0);
            }
            let push = rv::strong_knockback(position, body.position);
            if let Some(body) = self.body_mut(victim) {
                body.velocity += push;
                body.needs_sync = true;
            }
        }
        for player in players.iter().filter(|p| p.alive) {
            let (width, height) = super::pushing::player_size(player.eye_height);
            let half = f64::from(width / 2.0);
            let p = player.position;
            let player_box = (
                DVec3::new(p.x - half, p.y, p.z - half),
                DVec3::new(p.x + half, p.y + f64::from(height), p.z + half),
            );
            if !boxes_meet(player_box, area) {
                continue;
            }
            self.player_hits.push(PlayerHit {
                player_id: player.id,
                damage: rv::ROAR_DAMAGE,
                kind: PlayerHitKind::Melee {
                    attacker: position,
                    hunger_ticks: 0,
                    lift: 0.0,
                    throw: DVec3::ZERO,
                    disable_shield_seconds: 0.0,
                },
                source: Some(id),
            });
            self.player_pushes
                .push((player.id, rv::strong_knockback(position, p)));
        }
        self.entity_events.push((id, 69));
    }

    pub(super) fn tick_ravager(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty) = (self.game_time, self.difficulty);
        let mobs = self.mob_candidates();
        let villagers = self.illager_villagers();
        let mob_griefing = self.mob_griefing;
        let steered = self.ravager_steered(id);
        let raid = self
            .ravagers
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| self.raid_view(e.ravager.raider.raid_id));
        let Some(entity) = self.ravagers.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.ai.state.mobs = mobs;
        entity.ai.state.illager.raid = raid;
        entity.ai.state.mob_griefing = mob_griefing;
        entity.ai.set_steered(steered);
        entity.previous_position = entity.ravager.body.position;
        entity.tick_count += 1;
        base_tick_fluid(
            &mut entity.ravager.body,
            &*world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            SOUNDS,
        );
        super::hazards::burn(entity, &*world, game_time);
        super::hazards::suffocate(entity, &*world, rv::EYE_HEIGHT, game_time);
        let water_breathing = entity
            .effects
            .has(crate::effects::MobEffect::WaterBreathing);
        if entity.ravager.health > 0.0
            && super::living::breathe(
                &mut entity.ravager.body,
                &*world,
                rv::EYE_HEIGHT,
                water_breathing,
            )
        {
            entity.hurt(2.0);
        }
        let removed = entity.ravager.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        for work in entity.effects.tick(false) {
            match work.resolve(entity.ravager.health, rv::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut entity.ravager.health, rv::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt(amount);
                }
                _ => {}
            }
        }
        entity.ravager.body.float_from(&entity.effects);
        if entity.ravager.health > 0.0 {
            if (entity.random.next_int(1000) as i32) < entity.ambient_sound_time {
                entity.ambient_sound_time = -rv::AMBIENT_INTERVAL;
                let pitch = adult_pitch(&mut entity.random);
                entity
                    .voices
                    .push((Voice::Ambient(pitch), entity.position()));
            } else {
                entity.ambient_sound_time += 1;
            }
        }
        if removed {
            return;
        }
        let mut strike = None;
        let mut roars = false;
        if entity.ravager.health <= 0.0 && !entity.no_ai {
            let yaw = entity.ravager.yaw;
            let _ = super::living::dying_travel(
                &mut entity.ravager.body,
                &*world,
                yaw,
                entity.tick_count,
                &mut entity.random,
                &mut entity.voices,
                SOUNDS,
                Some(true),
            );
        } else {
            if entity.no_ai {
                entity.ravager.body.trim_small_velocity();
                monster_idle_without_ai(
                    &*world,
                    players,
                    entity.ravager.body.position,
                    rv::EYE_HEIGHT,
                    &mut entity.no_action_time,
                );
            } else {
                entity.ai.state.pois = raid.map(|_| std::mem::take(&mut self.pois));
                strike = entity.tick_ai(&*world, players, &villagers, game_time, difficulty);
                if let Some(pois) = entity.ai.state.pois.take() {
                    self.pois = pois;
                }
                if std::mem::take(&mut entity.ai.state.illager.reset_idle) {
                    entity.no_action_time = 0;
                }
                entity.ravager.raider.celebrating = entity.ai.state.illager.celebrating;
                if std::mem::take(&mut entity.ai.state.illager.celebrate_sound) {
                    let pitch = adult_pitch(&mut entity.random);
                    let position = entity.position();
                    entity.voices.push((
                        Voice::Event("entity.ravager.celebrate", 1.0, pitch),
                        position,
                    ));
                }
            }
            if entity.ravager.health > 0.0 {
                let has_target = entity.ai.state.target.is_some();
                entity.ravager.update_speed(has_target);
                if entity.ravager.body.horizontal_collision && mob_griefing {
                    entity.break_leaves(&mut *world);
                }
                roars = entity.tick_timers();
            }
        }
        let alert = entity.ai.state.alert.take();
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, &*world, old, game_time);
        if let Some(target) = strike {
            self.ravager_strikes(id, target);
        }
        if roars {
            self.ravager_roars(id, players);
        }
        if let Some(attacker) = alert {
            self.ravager_alerts(id, attacker);
        }
        self.push_entities(EntityKey::Ravager(id), &*world, game_time, ticks);
        self.file_in_section(EntityKey::Ravager(id));
    }

    fn ravager_alerts(&mut self, id: u64, attacker: Target) {
        let Some((p, range)) = self
            .ravagers
            .iter()
            .find(|e| e.id == id)
            .map(|e| (e.ravager.body.position, e.ai.state.follow_range))
        else {
            return;
        };
        for other in self.ravagers.iter_mut().filter(|e| e.id != id) {
            let q = other.ravager.body.position;
            if other.ai.state.target.is_some()
                || (q.x - p.x).abs() > range + 0.5
                || (q.z - p.z).abs() > range + 0.5
                || (q.y - p.y).abs() > 10.5
            {
                continue;
            }
            other.ai.state.set_target(Some(attacker));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};
    use std::collections::HashMap;

    #[derive(Default)]
    struct Field {
        placed: HashMap<Pos, Block>,
        broken: Vec<Pos>,
    }

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            if pos.1 < 64 {
                return Some(Block::new("minecraft:stone"));
            }
            self.placed.get(&pos).cloned()
        }
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => {
                    self.placed.insert(pos, block);
                }
                None => {
                    if self.placed.remove(&pos).is_some() {
                        self.broken.push(pos);
                    }
                }
            }
        }
    }

    fn player_at(id: u64, x: f64, z: f64) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position: DVec3::new(x, 64.0, z),
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

    fn ravager_at(world: &mut EntityWorld, at: DVec3, no_ai: bool) -> u64 {
        let id = world.spawn_ravager(Ravager::new(at), no_ai);
        world.ravager_mut(id).unwrap().set_random_seed(7);
        id
    }

    fn seed_where(stunned: bool) -> u64 {
        (0..)
            .find(|&seed| (LegacyRandom::new(seed).next_double() < rv::STUN_CHANCE) == stunned)
            .unwrap()
    }

    #[test]
    fn a_ravager_is_a_live_monster_of_its_size_and_health() {
        let mut world = EntityWorld::default();
        let id = ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        assert_eq!(world.entity_type(id), Some(rv::TYPE_ID));
        let e = &world.ravagers()[0];
        assert_eq!(
            (
                e.ravager.body.width,
                e.ravager.body.height,
                e.ravager.health
            ),
            (1.95, 2.2, 100.0)
        );
        assert_eq!(e.ai.state.follow_range, 32.0);
        assert_eq!(
            MonsterKind::Ravager.goal_names().len(),
            crate::ravager_ai::GOAL_NAMES.len()
        );
        assert!(crate::monster_ai::is_enemy(rv::TYPE_ID));
        assert!(world.census().iter().any(|c| c.kind == rv::TYPE_ID));
        assert!(world.mob_candidates().iter().any(|m| m.id == id));
        assert!(!world.can_be_leashed(id), "monsters take no lead");
        let attack = PlayerAttack {
            player_id: 1,
            position: DVec3::new(0.5, 64.0, 3.5),
            yaw: 180.0,
            attack_damage: 1000.0,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        };
        assert!(world.player_attack(&attack, id).died);
        let deaths = world.take_deaths();
        assert_eq!(deaths[0].table, Some(rv::TYPE_ID));
        assert_eq!(deaths[0].experience, rv::EXPERIENCE);
    }

    #[test]
    fn a_ravager_runs_down_a_player_bites_for_twelve_and_stands_still_after() {
        let mut world = EntityWorld::default();
        let id = ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let players = [player_at(1, 0.5, 8.5)];
        let mut struck = None;
        let mut sounds = Vec::new();
        for _ in 0..200 {
            world.tick_with_players(&mut Field::default(), &players);
            sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
            if let Some(hit) = world.take_player_hits().into_iter().next() {
                struck = Some(hit);
                break;
            }
        }
        let hit = struck.expect("it charges in and bites");
        assert_eq!((hit.player_id, hit.source, hit.damage), (1, Some(id), 12.0));
        let PlayerHitKind::Melee { throw, .. } = hit.kind else {
            panic!("{:?}", hit.kind);
        };
        assert!(throw.z > 0.5 && (throw.y - 0.4).abs() < 1e-6, "{throw}");
        assert!(sounds.iter().any(|s| s == rv::ATTACK_SOUND), "{sounds:?}");
        let e = &world.ravagers()[0];
        assert_eq!(e.ravager.attack_tick, rv::ATTACK_DURATION - 1);
        assert_eq!(e.ai.state.target, Some(Target::Player(1)));
        let at = e.ravager.body.position;
        for _ in 0..9 {
            world.tick_with_players(&mut Field::default(), &players);
        }
        let e = &world.ravagers()[0];
        assert_eq!(e.ravager.attack_tick, 0);
        assert!(
            (e.ravager.body.position.x - at.x).abs() < 0.2
                && (e.ravager.body.position.z - at.z).abs() < 0.2,
            "it holds still while it bites"
        );
        assert!(world.take_player_hits().is_empty());
    }

    #[test]
    fn a_blocked_ravager_is_stunned_for_forty_ticks_then_roars_and_flings_what_is_near() {
        let mut world = EntityWorld::default();
        let id = ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        world
            .ravager_mut(id)
            .unwrap()
            .set_random_seed(seed_where(true));
        let cow = world.spawn_cow(crate::cow::Cow::new(DVec3::new(-2.5, 64.0, 0.5)), true);
        let pillager = world.spawn_illager(
            crate::illager::Illager::pillager(DVec3::new(3.5, 64.0, 0.5)),
            true,
        );
        let feet = DVec3::new(0.5, 64.0, 2.5);
        let pushes = world.blocked_by_shield(id, 1, feet);
        assert_eq!(pushes.len(), 1);
        assert_eq!(pushes[0].0, 1);
        assert!(
            pushes[0].1.z > 0.0 && pushes[0].1.length() <= 0.05 + 1e-9,
            "a nudge away: {:?}",
            pushes[0].1
        );
        assert_eq!(world.ravagers()[0].ravager.stunned_tick, rv::STUN_DURATION);
        assert_eq!(world.take_entity_events(), vec![(id, 39)]);
        assert!(
            world
                .take_sounds()
                .iter()
                .any(|s| s.event == rv::STUNNED_SOUND)
        );
        let players = [player_at(1, 0.5, 2.5)];
        let mut roar_sound = None;
        let mut roared = None;
        let mut particles = 0;
        for tick in 1..=60 {
            world.tick_with_players(&mut Field::default(), &players);
            particles += world.ravagers()[0].stun_particles.len();
            if world
                .take_sounds()
                .iter()
                .any(|s| s.event == rv::ROAR_SOUND)
            {
                roar_sound.get_or_insert(tick);
            }
            if world.take_entity_events().contains(&(id, 69)) {
                roared.get_or_insert(tick);
                let hits = world.take_player_hits();
                assert_eq!(hits.len(), 1);
                assert_eq!(
                    (hits[0].damage, hits[0].source),
                    (rv::ROAR_DAMAGE, Some(id))
                );
                let flung = world.take_player_pushes();
                assert_eq!(flung.len(), 1);
                let from = world.ravagers()[0].ravager.body.position;
                let expected = rv::strong_knockback(from, players[0].position);
                assert!((flung[0].1 - expected).length() < 1e-9, "{:?}", flung[0].1);
                assert!(
                    flung[0].1.z > 1.5 && flung[0].1.y == 0.2,
                    "{:?}",
                    flung[0].1
                );
                let cow_entity = world.cows().iter().find(|e| e.id == cow).unwrap();
                assert!(cow_entity.cow.health < 10.0, "the cow is hurt");
                assert!(cow_entity.cow.body.velocity.x < -0.5, "and flung");
                let pillager_entity = world.illagers().iter().find(|e| e.id == pillager).unwrap();
                assert_eq!(
                    pillager_entity.illager.health,
                    crate::illager::MAX_HEALTH,
                    "illagers are spared"
                );
                assert!(pillager_entity.illager.body.velocity.x > 0.5, "but flung");
            } else {
                assert!(world.take_player_hits().is_empty(), "tick {tick}");
            }
        }
        assert_eq!(roar_sound, Some(40));
        assert_eq!(roared, Some(50));
        assert!(
            (1..=20).contains(&particles),
            "about one tick in six: {particles}"
        );
        assert_eq!(world.ravagers()[0].ravager.roar_tick, 0);
    }

    #[test]
    fn a_blocked_ravager_that_is_not_stunned_throws_the_shield_bearer_back() {
        let mut world = EntityWorld::default();
        let id = ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        world
            .ravager_mut(id)
            .unwrap()
            .set_random_seed(seed_where(false));
        let pushes = world.blocked_by_shield(id, 4, DVec3::new(2.5, 64.0, 0.5));
        assert_eq!(pushes, vec![(4, DVec3::new(2.0, 0.2, 0.0))]);
        assert_eq!(world.ravagers()[0].ravager.stunned_tick, 0);
        assert!(
            world.blocked_by_shield(9999, 4, DVec3::ZERO).is_empty(),
            "only ravagers react"
        );
        world.ravager_mut(id).unwrap().ravager.roar_tick = 3;
        assert!(
            world
                .blocked_by_shield(id, 4, DVec3::new(2.5, 64.0, 0.5))
                .is_empty(),
            "a roaring ravager ignores it"
        );
    }

    fn leaf_hedge() -> Field {
        let mut field = Field::default();
        for x in -3..=3 {
            for y in 64..=66 {
                field
                    .placed
                    .insert((x, y, 2), Block::new("minecraft:oak_leaves"));
            }
        }
        field
    }

    fn push_into_hedge(griefing: bool) -> Field {
        let mut world = EntityWorld::default();
        world.set_mob_griefing(griefing);
        let id = ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let mut field = leaf_hedge();
        for _ in 0..10 {
            world.ravager_mut(id).unwrap().ravager.body.velocity.z = 0.3;
            world.tick(&mut field);
        }
        field
    }

    #[test]
    fn a_ravager_breaks_leaves_it_walks_into_only_with_mob_griefing() {
        let field = push_into_hedge(true);
        assert!(!field.broken.is_empty(), "it breaks through");
        assert!(
            field
                .broken
                .iter()
                .all(|&(x, _, z)| z == 2 && (-2..=1).contains(&x)),
            "{:?}",
            field.broken
        );
        let field = push_into_hedge(false);
        assert!(field.broken.is_empty(), "{:?}", field.broken);
    }

    #[test]
    fn a_pillager_rides_a_ravager_two_blocks_up_and_the_ravager_still_hunts() {
        let mut world = EntityWorld::default();
        let ravager = ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let pillager = world.spawn_illager(
            crate::illager::Illager::pillager(DVec3::new(5.5, 64.0, 0.5)),
            false,
        );
        assert!(world.start_riding(Rider::Mob(pillager), ravager, &Field::default()));
        let second = world.spawn_illager(
            crate::illager::Illager::pillager(DVec3::new(5.5, 64.0, 5.5)),
            true,
        );
        assert!(
            !world.start_riding(Rider::Mob(second), ravager, &Field::default()),
            "one rider"
        );
        let seat = world.seat(Rider::Mob(pillager)).unwrap();
        assert!((seat.y - (64.0 + 2.2625 - 0.6)).abs() < 1e-6, "{seat}");
        let players = [player_at(1, 0.5, 12.5)];
        for _ in 0..40 {
            world.tick_with_players(&mut Field::default(), &players);
        }
        let carrier = &world.ravagers()[0];
        assert_eq!(carrier.ai.state.target, Some(Target::Player(1)));
        assert!(
            carrier.ravager.body.position.z > 2.0,
            "{:?}",
            carrier.ravager.body.position
        );
        let rider = world.illagers().iter().find(|e| e.id == pillager).unwrap();
        let expected = carrier.ravager.body.position
            + crate::ravager::passenger_point(carrier.ravager.yaw)
            - DVec3::new(0.0, 0.6, 0.0);
        assert!(
            (rider.illager.body.position - expected).length() < 0.5,
            "{:?} {:?}",
            rider.illager.body.position,
            expected
        );
        assert_eq!(world.vehicle_of(Rider::Mob(pillager)), Some(ravager));
        let top = world.ravagers()[0].ravager.body.position.y + 2.2;
        let spot = world
            .stop_riding(Rider::Mob(pillager), 0.0, &Field::default())
            .unwrap();
        assert!((spot.y - top).abs() < 1e-6);
    }

    #[test]
    fn a_zombie_on_a_ravager_holds_it_still() {
        let mut world = EntityWorld::default();
        let ravager = ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let zombie =
            world.spawn_zombie(crate::zombie::Zombie::new(DVec3::new(0.5, 64.0, 3.5)), true);
        assert!(world.force_riding(Rider::Mob(zombie), ravager));
        let players = [player_at(1, 0.5, 8.5)];
        for _ in 0..60 {
            world.tick_with_players(&mut Field::default(), &players);
        }
        let carrier = &world.ravagers()[0];
        assert!(
            carrier
                .ravager
                .body
                .position
                .distance(DVec3::new(0.5, 64.0, 0.5))
                < 0.1
        );
        assert!(world.take_player_hits().is_empty());
    }

    #[test]
    fn iron_golems_hunt_ravagers_villagers_flee_them_and_babies_are_spared() {
        let mut world = EntityWorld::default();
        let ravager = ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let golem = world.spawn_iron_golem(
            crate::iron_golem::IronGolem::new(DVec3::new(0.5, 64.0, 8.5)),
            false,
        );
        for _ in 0..40 {
            world.tick(&mut Field::default());
        }
        let hunter = world.iron_golems().iter().find(|e| e.id == golem).unwrap();
        assert_eq!(hunter.ai.state.target, Some(Target::Mob(ravager)));
        let mut world = EntityWorld::default();
        let ravager = ravager_at(&mut world, DVec3::new(0.5, 64.0, 10.5), true);
        let villager = world.spawn_villager_active(
            crate::villager::Villager::new(DVec3::new(0.5, 64.0, 0.5)),
            0.0,
        );
        let mut seen = None;
        for _ in 0..40 {
            world.tick(&mut Field::default());
            let entity = world.villagers().iter().find(|e| e.id == villager).unwrap();
            if let Some(ai) = entity.ai.as_deref() {
                seen = seen.or(ai.brain.memories.nearest_hostile.get().copied());
            }
        }
        assert_eq!(seen, Some(ravager));
        let mut world = EntityWorld::default();
        ravager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let mut baby = crate::villager::Villager::new(DVec3::new(0.5, 64.0, 6.5));
        baby.age.set(-24000);
        world.spawn_villager(baby, true);
        for _ in 0..40 {
            world.tick(&mut Field::default());
        }
        assert_eq!(
            world.ravagers()[0].ai.state.target,
            None,
            "babies are spared"
        );
        let adult = world.spawn_villager(
            crate::villager::Villager::new(DVec3::new(4.5, 64.0, 6.5)),
            true,
        );
        for _ in 0..40 {
            world.tick(&mut Field::default());
        }
        assert_eq!(
            world.ravagers()[0].ai.state.target,
            Some(Target::Villager(adult))
        );
    }
}
