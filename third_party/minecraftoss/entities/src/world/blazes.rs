use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::blaze::{self, Blaze};
use crate::fireball::{Fireball, FireballKind, Shooter};
use crate::monster_ai::{MonsterKind, Target};

const BLAZE_SOUNDS: MovementSounds = MovementSounds::monster(None);

#[derive(Clone)]
pub struct BlazeEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub blaze: Blaze,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
}

struct BlazeStep {
    hit: Option<(Target, DVec3)>,
    shots: Vec<DVec3>,
    alert: Option<Target>,
}

impl BlazeEntity {
    pub fn position(&self) -> DVec3 {
        self.blaze.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.blaze.health > 0.0 && !self.blaze.damage.dead {
            self.no_action_time = 0;
        }
        let result =
            self.blaze
                .damage
                .hurt_generic(&mut self.blaze.health, blaze::MAX_HEALTH, amount);
        let position = self.position();
        self.blaze
            .damage
            .place_death(result, position, self.blaze.body.fire_ticks > 0);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -80;
            }
            let voice = hurt_voice(&mut self.random, result.died, false);
            self.voices.push((voice, position));
        }
        result
    }

    fn tick_ai(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        game_time: i64,
        difficulty: i32,
    ) -> BlazeStep {
        let position = self.blaze.body.position;
        if players
            .iter()
            .any(|p| p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0)
        {
            self.no_action_time = 0;
        }
        self.no_action_time += 2;
        let health = self.blaze.health;
        let speed = self.effects.movement_speed(blaze::MOVEMENT_SPEED);
        let ai = &mut self.ai;
        let body = &mut self.blaze.body;
        if ai.no_jump_delay > 0 {
            ai.no_jump_delay -= 1;
        }
        body.trim_small_velocity();
        self.no_action_time += 1;
        let fluid = FluidFrame::sample(world, position, body.width, body.height);
        ai.state.body = body.clone();
        ai.state.health = health;
        ai.state.max_health = blaze::MAX_HEALTH;
        ai.state.game_time = game_time;
        ai.state.difficulty = difficulty;
        ai.state.players = players.to_vec();
        ai.state.villagers = Vec::new();
        ai.state.fluid = fluid;
        ai.state.no_action_time = self.no_action_time;
        ai.state.tick_count = self.tick_count;
        ai.state.random = std::mem::take(&mut self.random);
        let full = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
        ai.tick_goals(world, full);
        let hit = ai.state.attack.take().map(|target| (target, position));
        let shots = std::mem::take(&mut ai.state.blaze.shots);
        let alert = ai.state.alert.take();
        self.blaze.charged = ai.state.blaze.charged;
        if let Some(wanted) = ai.state.blaze.wanted.take() {
            ai.move_control.set_wanted_position(wanted, 1.0);
        }
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
        let target_eye = ai
            .state
            .target()
            .map(|t| t.position.y + f64::from(t.eye_height));
        self.blaze.rise_towards(target_eye, &mut self.random);
        let body = &mut self.blaze.body;
        let obstacle_top = crate::control::obstacle_top(world, position);
        ai.sideways *= 0.98;
        ai.forward *= 0.98;
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
        ai.state.look_control.tick(
            position,
            blaze::EYE_HEIGHT,
            ai.body_rotation.body_yaw,
            !ai.state.navigation.is_done(),
        );
        ai.jumping = ai.state.jump || control.jump;
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
            BLAZE_SOUNDS,
        );
        ai.body_rotation.tick(
            ai.yaw,
            &mut ai.state.look_control,
            self.previous_position,
            body.position,
        );
        self.blaze.yaw = ai.yaw;
        if let Some(damage) = landed.and_then(|fallen| {
            super::living::fall_damage(&self.blaze.body, world, fallen, true, &mut self.voices)
        }) {
            self.hurt(damage);
        }
        BlazeStep { hit, shots, alert }
    }

    fn hurt_by_water(&mut self, world: &impl World) {
        let p = self.blaze.body.position;
        let feet = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let top = (
            feet.0,
            (p.y + f64::from(self.blaze.body.height)).floor() as i32,
            feet.2,
        );
        if self.blaze.body.touching_water || world.rain_at(feet) || world.rain_at(top) {
            self.hurt(1.0);
        }
    }
}

fn adult_pitch(random: &mut LegacyRandom) -> f32 {
    let (a, b) = (random.next_float(), random.next_float());
    (a - b) * 0.2 + 1.0
}

impl EntityWorld {
    pub fn spawn_blaze(&mut self, blaze: Blaze, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut ai = MonsterAi::of_kind(MonsterKind::Blaze, &blaze.body, blaze.yaw);
        ai.state.max_health = blaze::MAX_HEALTH;
        self.blazes.push(BlazeEntity {
            effects: Default::default(),
            id,
            previous_position: blaze.body.position,
            blaze,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
        });
        self.order.push(EntityKey::Blaze(id));
        self.file_in_section(EntityKey::Blaze(id));
        id
    }

    pub fn blazes(&self) -> &[BlazeEntity] {
        &self.blazes
    }

    pub fn blaze_mut(&mut self, id: u64) -> Option<&mut BlazeEntity> {
        self.blazes.iter_mut().find(|entity| entity.id == id)
    }

    fn alert_blazes(&mut self, blaze_id: u64, attacker: Target) {
        let Some(source) = self.blazes.iter().find(|e| e.id == blaze_id) else {
            return;
        };
        let range = source.ai.state.follow_range;
        let p = source.blaze.body.position;
        let (min, max) = (
            p - DVec3::new(range, 10.0, range),
            p + DVec3::new(1.0 + range, 11.0, 1.0 + range),
        );
        for other in &mut self.blazes {
            if other.id == blaze_id
                || other.blaze.health <= 0.0
                || other.ai.state.target().is_some()
            {
                continue;
            }
            let b = &other.blaze.body;
            let half = f64::from(b.width / 2.0);
            let q = b.position;
            let touches = q.x - half < max.x
                && q.x + half > min.x
                && q.y < max.y
                && q.y + f64::from(b.height) > min.y
                && q.z - half < max.z
                && q.z + half > min.z;
            if touches {
                other.ai.state.set_target(Some(attacker));
            }
        }
    }

    pub(super) fn tick_blaze(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty) = (self.game_time, self.difficulty);
        let mobs = self.mob_candidates();
        let Some(entity) = self.blazes.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.ai.state.mobs = mobs;
        entity.previous_position = entity.blaze.body.position;
        entity.tick_count += 1;
        base_tick_fluid(
            &mut entity.blaze.body,
            world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            BLAZE_SOUNDS,
        );
        super::hazards::burn(entity, world, game_time);
        super::hazards::suffocate(entity, world, blaze::EYE_HEIGHT, game_time);
        let water_breathing = entity
            .effects
            .has(crate::effects::MobEffect::WaterBreathing);
        if entity.blaze.health > 0.0
            && super::living::breathe(
                &mut entity.blaze.body,
                world,
                blaze::EYE_HEIGHT,
                water_breathing,
            )
        {
            entity.hurt(2.0);
        }
        let removed = entity.blaze.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        for work in entity.effects.tick(false) {
            match work.resolve(entity.blaze.health, blaze::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut entity.blaze.health, blaze::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt(amount);
                }
                _ => {}
            }
        }
        entity.blaze.body.float_from(&entity.effects);
        if entity.blaze.health > 0.0 {
            if (entity.random.next_int(1000) as i32) < entity.ambient_sound_time {
                entity.ambient_sound_time = -80;
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
        entity.blaze.slow_fall();
        let origin = entity.blaze.body.position;
        let mut step = BlazeStep {
            hit: None,
            shots: Vec::new(),
            alert: None,
        };
        let mut active = false;
        if entity.blaze.health <= 0.0 && !entity.no_ai {
            let yaw = entity.blaze.yaw;
            let _ = super::living::dying_travel(
                &mut entity.blaze.body,
                world,
                yaw,
                entity.tick_count,
                &mut entity.random,
                &mut entity.voices,
                BLAZE_SOUNDS,
                Some(true),
            );
        } else if !entity.no_ai {
            step = entity.tick_ai(world, players, game_time, difficulty);
            active = true;
        } else {
            entity.blaze.body.trim_small_velocity();
            monster_idle_without_ai(
                world,
                players,
                entity.blaze.body.position,
                blaze::EYE_HEIGHT,
                &mut entity.no_action_time,
            );
        }
        if active {
            entity.hurt_by_water(world);
        }
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, world, old, game_time);
        if !step.shots.is_empty() {
            let pitch = (entity.random.next_float() - entity.random.next_float()) * 0.2 + 1.0;
            entity
                .voices
                .push((Voice::Event("entity.blaze.shoot", 2.0, pitch), origin));
        }
        self.push_entities(EntityKey::Blaze(id), world, game_time, ticks);
        let start = origin + DVec3::new(0.0, f64::from(blaze::HEIGHT) * 0.5 + 0.5, 0.0);
        for aim in step.shots {
            self.spawn_fireball(Fireball::shoot(
                FireballKind::Small,
                Shooter::Mob(id),
                start,
                aim,
            ));
        }
        if let Some(attacker) = step.alert {
            self.alert_blazes(id, attacker);
        }
        if let Some((Target::Player(player_id), attacker)) = step.hit {
            self.player_hits.push(PlayerHit {
                player_id,
                damage: blaze::ATTACK_DAMAGE,
                kind: PlayerHitKind::Melee {
                    attacker,
                    hunger_ticks: 0,
                    lift: 0.0,
                },
                source: Some(id),
            });
        }
        if let Some((Target::Mob(victim), attacker)) = step.hit {
            self.mob_hits_mob(id, victim, blaze::ATTACK_DAMAGE, attacker, 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Floor;
    impl World for Floor {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            (pos.1 < 1 || pos.2 >= 30)
                .then(|| minecraftoss_player::Block::new("minecraft:nether_bricks"))
        }
        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
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

    fn blaze_world(at: DVec3) -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        world.set_players_pickable(true);
        let id = world.spawn_blaze(Blaze::new(at), false);
        world.blaze_mut(id).unwrap().set_random_seed(11);
        (world, id)
    }

    #[test]
    fn a_blaze_burns_bright_and_fires_three_small_fireballs_that_set_the_player_alight() {
        let (mut world, id) = blaze_world(DVec3::new(0.5, 1.0, 0.5));
        let target = player(7, DVec3::new(0.5, 1.0, 12.5));
        let (mut hits, mut sounds, mut charged, mut most) = (Vec::new(), Vec::new(), false, 0);
        for _ in 0..400 {
            world.tick_with_players(&mut Floor, &[target]);
            hits.extend(world.take_player_hits());
            sounds.extend(world.take_sounds());
            charged |= world.blazes()[0].blaze.charged;
            most = most.max(world.fireballs().len());
        }
        assert!(charged, "it flares up before a volley");
        assert!(
            world.burning().iter().any(|b| b.2 == 0.6) || !world.blazes()[0].blaze.charged,
            "a flaring blaze is drawn burning"
        );
        assert!(
            sounds
                .iter()
                .filter(|s| s.event == "entity.blaze.shoot" && s.volume == 2.0)
                .count()
                >= 3,
            "three shots a volley"
        );
        assert!(
            most >= 1
                && world
                    .fireballs()
                    .iter()
                    .all(|f| f.fireball.kind == FireballKind::Small)
        );
        let burning = hits
            .iter()
            .find(|h| {
                matches!(
                    h.kind,
                    PlayerHitKind::Fireball {
                        ignite_ticks: 100,
                        ..
                    }
                )
            })
            .expect("a small fireball strikes and sets the player alight");
        assert_eq!((burning.damage, burning.source), (5.0, Some(id)));
        let (mut world, _) = blaze_world(DVec3::new(0.5, 1.0, 0.5));
        let close = player(7, DVec3::new(0.5, 1.0, 1.7));
        let mut hits = Vec::new();
        for _ in 0..60 {
            world.tick_with_players(&mut Floor, &[close]);
            hits.extend(world.take_player_hits());
        }
        assert!(
            hits.iter()
                .any(|h| matches!(h.kind, PlayerHitKind::Melee { .. }) && h.damage == 6.0),
            "up close it strikes"
        );
    }

    #[test]
    fn a_blaze_rises_towards_a_player_above_it() {
        let (mut world, _) = blaze_world(DVec3::new(0.5, 1.0, 0.5));
        let above = player(7, DVec3::new(0.5, 12.0, 6.5));
        let mut highest: f64 = 1.0;
        for _ in 0..200 {
            world.tick_with_players(&mut Floor, &[above]);
            let _ = world.take_player_hits();
            highest = highest.max(world.blazes()[0].blaze.body.position.y);
        }
        assert!(highest > 3.0, "it climbs into the air: {highest}");
    }

    #[test]
    fn a_small_fireball_lights_the_block_beside_where_it_lands_and_spares_fire_immune_mobs() {
        let mut world = EntityWorld::default();
        let blaze = world.spawn_blaze(Blaze::new(DVec3::new(0.5, 1.0, -40.5)), true);
        let other = world.spawn_blaze(Blaze::new(DVec3::new(0.5, 1.0, 10.5)), true);
        let zombie =
            world.spawn_zombie(crate::zombie::Zombie::new(DVec3::new(0.5, 1.0, 20.5)), true);
        world.spawn_fireball(Fireball::shoot(
            FireballKind::Small,
            Shooter::Mob(blaze),
            DVec3::new(0.5, 1.5, 2.0),
            DVec3::Z,
        ));
        for _ in 0..30 {
            world.tick(&mut Floor);
        }
        let other = world.blazes().iter().find(|e| e.id == other).unwrap();
        assert_eq!(
            (other.blaze.health, other.blaze.body.fire_ticks),
            (crate::blaze::MAX_HEALTH, 0),
            "blazes shrug off small fireballs"
        );
        world.spawn_fireball(Fireball::shoot(
            FireballKind::Small,
            Shooter::Mob(blaze),
            DVec3::new(0.5, 1.5, 12.0),
            DVec3::Z,
        ));
        for _ in 0..20 {
            world.tick(&mut Floor);
        }
        let zombie = world.zombies().iter().find(|e| e.id == zombie).unwrap();
        assert!(
            zombie.zombie.health < 20.0 && zombie.zombie.body.fire_ticks > 0,
            "a zombie burns: {} {}",
            zombie.zombie.health,
            zombie.zombie.body.fire_ticks
        );
        world.take_fireball_fires();
        world.spawn_fireball(Fireball::shoot(
            FireballKind::Small,
            Shooter::Mob(blaze),
            DVec3::new(5.5, 1.5, 24.0),
            DVec3::Z,
        ));
        for _ in 0..20 {
            world.tick(&mut Floor);
        }
        assert_eq!(
            world.take_fireball_fires(),
            vec![(5, 1, 29)],
            "fire goes on the air before the wall"
        );
        world.set_mob_griefing(false);
        world.spawn_fireball(Fireball::shoot(
            FireballKind::Small,
            Shooter::Mob(blaze),
            DVec3::new(5.5, 1.5, 24.0),
            DVec3::Z,
        ));
        for _ in 0..20 {
            world.tick(&mut Floor);
        }
        assert!(
            world.take_fireball_fires().is_empty(),
            "no fire where mobs may not grief"
        );
        let small = world.spawn_fireball(Fireball::shoot(
            FireballKind::Small,
            Shooter::Mob(blaze),
            DVec3::new(5.5, 1.5, 24.0),
            DVec3::Z,
        ));
        assert!(
            !world.deflect_fireball(small, DVec3::X, 7, true),
            "small fireballs cannot be sent back"
        );
    }
}
