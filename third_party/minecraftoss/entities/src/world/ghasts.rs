use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::fireball::{Fireball, FireballKind, Shooter};
use crate::ghast::{self, Ghast};
use crate::monster_ai::MonsterKind;

const GHAST_SOUNDS: MovementSounds = MovementSounds::creature(None);

#[derive(Clone)]
pub struct GhastEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub ghast: Ghast,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
}

impl GhastEntity {
    pub fn position(&self) -> DVec3 {
        self.ghast.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.ghast.health > 0.0 && !self.ghast.damage.dead {
            self.no_action_time = 0;
        }
        let result =
            self.ghast
                .damage
                .hurt_generic(&mut self.ghast.health, ghast::MAX_HEALTH, amount);
        let position = self.position();
        self.ghast
            .damage
            .place_death(result, position, self.ghast.body.fire_ticks > 0);
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
    ) -> Option<(DVec3, DVec3)> {
        let position = self.ghast.body.position;
        if players
            .iter()
            .any(|p| p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0)
        {
            self.no_action_time = 0;
        }
        let health = self.ghast.health;
        let ai = &mut self.ai;
        let body = &mut self.ghast.body;
        body.trim_small_velocity();
        self.no_action_time += 1;
        let fluid = FluidFrame::sample(world, position, body.width, body.height);
        ai.state.body = body.clone();
        ai.state.health = health;
        ai.state.max_health = ghast::MAX_HEALTH;
        ai.state.game_time = game_time;
        ai.state.difficulty = difficulty;
        ai.state.players = players.to_vec();
        ai.state.villagers = Vec::new();
        ai.state.fluid = fluid;
        ai.state.no_action_time = self.no_action_time;
        ai.state.random = std::mem::take(&mut self.random);
        let full = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
        ai.tick_goals(world, full);
        self.random = std::mem::take(&mut ai.state.random);
        if std::mem::take(&mut ai.state.ghast.warn) {
            let pitch = (self.random.next_float() - self.random.next_float()) * 0.2 + 1.0;
            self.voices
                .push((Voice::Event("entity.ghast.warn", 10.0, pitch), position));
        }
        let shot = ai.state.ghast.shot.take();
        if shot.is_some() {
            let pitch = (self.random.next_float() - self.random.next_float()) * 0.2 + 1.0;
            self.voices
                .push((Voice::Event("entity.ghast.shoot", 10.0, pitch), position));
        }
        self.ghast.charging = ai.state.ghast.charging;
        if let Some(wanted) = ai.state.ghast.wanted {
            let floating = ai.state.ghast.float_duration;
            ai.state.ghast.float_duration -= 1;
            if floating <= 0 {
                ai.state.ghast.float_duration += self.random.next_int(5) as i32 + 2;
                let delta = wanted - position;
                if ghast::can_reach(world, position, body.width, body.height, delta) {
                    body.velocity += ghast::float_impulse(delta);
                } else {
                    ai.state.ghast.wanted = None;
                }
            }
        }
        ai.yaw = ai.state.yaw;
        ai.state.look_control.head_yaw = ai.yaw;
        ai.body_rotation.body_yaw = ai.yaw;
        self.ghast.yaw = ai.yaw;
        body.travel_flying(world, DVec3::ZERO, 0.02, ai.yaw, fluid);
        play_movement(
            body,
            self.tick_count,
            &mut self.random,
            &mut self.voices,
            GHAST_SOUNDS,
        );
        shot
    }
}

fn adult_pitch(random: &mut LegacyRandom) -> f32 {
    let (a, b) = (random.next_float(), random.next_float());
    (a - b) * 0.2 + 1.0
}

impl EntityWorld {
    pub fn spawn_ghast(&mut self, ghast: Ghast, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut ai = MonsterAi::of_kind(MonsterKind::Ghast, &ghast.body, ghast.yaw);
        ai.state.max_health = ghast::MAX_HEALTH;
        self.ghasts.push(GhastEntity {
            effects: Default::default(),
            id,
            previous_position: ghast.body.position,
            ghast,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
        });
        self.order.push(EntityKey::Ghast(id));
        self.file_in_section(EntityKey::Ghast(id));
        id
    }

    pub fn ghasts(&self) -> &[GhastEntity] {
        &self.ghasts
    }

    pub fn ghast_mut(&mut self, id: u64) -> Option<&mut GhastEntity> {
        self.ghasts.iter_mut().find(|entity| entity.id == id)
    }

    pub(super) fn tick_ghast(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty) = (self.game_time, self.difficulty);
        let Some(entity) = self.ghasts.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.previous_position = entity.ghast.body.position;
        entity.tick_count += 1;
        base_tick_fluid(
            &mut entity.ghast.body,
            world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            GHAST_SOUNDS,
        );
        super::hazards::burn(entity, world, game_time);
        super::hazards::suffocate(entity, world, ghast::EYE_HEIGHT, game_time);
        let water_breathing = entity
            .effects
            .has(crate::effects::MobEffect::WaterBreathing);
        if entity.ghast.health > 0.0
            && super::living::breathe(
                &mut entity.ghast.body,
                world,
                ghast::EYE_HEIGHT,
                water_breathing,
            )
        {
            entity.hurt(2.0);
        }
        let removed = entity.ghast.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        for work in entity.effects.tick(false) {
            match work.resolve(entity.ghast.health, ghast::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut entity.ghast.health, ghast::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt(amount);
                }
                _ => {}
            }
        }
        entity.ghast.body.float_from(&entity.effects);
        if entity.ghast.health > 0.0 {
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
        let mut shot = None;
        if entity.ghast.health <= 0.0 && !entity.no_ai {
            let fluid = FluidFrame::sample(
                world,
                entity.ghast.body.position,
                entity.ghast.body.width,
                entity.ghast.body.height,
            );
            let yaw = entity.ghast.yaw;
            entity.ghast.body.trim_small_velocity();
            entity
                .ghast
                .body
                .travel_flying(world, DVec3::ZERO, 0.02, yaw, fluid);
        } else if !entity.no_ai {
            shot = entity.tick_ai(world, players, game_time, difficulty);
        } else {
            entity.ghast.body.trim_small_velocity();
        }
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, world, old, game_time);
        let power = entity.ghast.explosion_power;
        self.push_entities(EntityKey::Ghast(id), world, game_time, ticks);
        if let Some((aim, start)) = shot {
            self.spawn_fireball(Fireball::shoot(
                FireballKind::Large {
                    explosion_power: power,
                },
                Shooter::Mob(id),
                start,
                aim,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Floor;
    impl World for Floor {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            (pos.1 < 1).then(|| minecraftoss_player::Block::new("minecraft:netherrack"))
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

    fn ghast_world(at: DVec3) -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        world.set_players_pickable(true);
        let id = world.spawn_ghast(Ghast::new(at), false);
        world.ghast_mut(id).unwrap().set_random_seed(5);
        (world, id)
    }

    #[test]
    fn a_ghast_floats_about_warns_and_fires_at_a_player_level_with_it() {
        let (mut world, id) = ghast_world(DVec3::new(0.5, 20.0, 0.5));
        let target = player(7, DVec3::new(0.5, 20.0, 20.5));
        let (mut hits, mut sounds, mut blasts, mut charged) =
            (Vec::new(), Vec::new(), Vec::new(), false);
        for _ in 0..200 {
            world.tick_with_players(&mut Floor, &[target]);
            hits.extend(world.take_player_hits());
            sounds.extend(world.take_sounds());
            blasts.extend(world.take_fireball_blasts());
            charged |= world.ghasts()[0].ghast.charging;
        }
        assert!(charged, "it opens its mouth before it fires");
        assert!(
            sounds
                .iter()
                .any(|s| s.event == "entity.ghast.warn" && s.volume == 10.0)
        );
        assert!(sounds.iter().any(|s| s.event == "entity.ghast.shoot"));
        let direct = hits
            .iter()
            .find(|h| matches!(h.kind, PlayerHitKind::Fireball { .. }))
            .expect("a fireball strikes the player");
        assert_eq!(
            (direct.damage, direct.source, direct.player_id),
            (6.0, Some(id), 7)
        );
        assert!(
            hits.iter()
                .any(|h| matches!(h.kind, PlayerHitKind::Explosion { .. }) && h.source == Some(id)),
            "and blows up"
        );
        assert!(!blasts.is_empty() && blasts.iter().all(|b| b.power == 1.0 && b.fire));
        assert!(
            world.ghasts()[0]
                .ghast
                .body
                .position
                .distance(DVec3::new(0.5, 20.0, 0.5))
                > 0.5,
            "it floats about"
        );
        assert!(
            world.ghasts()[0].ghast.health == crate::ghast::MAX_HEALTH,
            "its own fireballs leave it alone"
        );
        let (mut world, _) = ghast_world(DVec3::new(0.5, 20.0, 0.5));
        let above = player(7, DVec3::new(0.5, 50.0, 20.5));
        for _ in 0..60 {
            world.tick_with_players(&mut Floor, &[above]);
        }
        assert!(
            world.fireballs().is_empty() && world.take_player_hits().is_empty(),
            "a player more than four blocks above or below goes unseen"
        );
    }

    #[test]
    fn a_fireball_struck_back_along_the_aim_kills_its_ghast_and_credits_the_player() {
        let (mut world, id) = ghast_world(DVec3::new(0.5, 20.0, 0.5));
        let target = player(7, DVec3::new(0.5, 20.0, 24.5));
        let mut deflected = false;
        for _ in 0..300 {
            world.tick_with_players(&mut Floor, &[target]);
            let _ = world.take_player_hits();
            if !deflected {
                if let Some(fireball) = world
                    .fireballs()
                    .first()
                    .filter(|f| f.fireball.position.z > 8.0)
                {
                    let (fireball_id, at) = (fireball.id, fireball.fireball.position);
                    let ghast = world.ghasts()[0].ghast.body.position + DVec3::new(0.0, 2.0, 0.0);
                    assert!(
                        world
                            .fireball_on_ray(
                                target.position + DVec3::new(0.0, 1.62, 0.0),
                                crate::fireball::normalize(at - target.position),
                                30.0
                            )
                            .is_some()
                    );
                    deflected = world.deflect_fireball(
                        fireball_id,
                        crate::fireball::normalize(ghast - at),
                        7,
                        true,
                    );
                }
            }
            if world.ghasts().iter().all(|e| e.ghast.health <= 0.0) {
                break;
            }
        }
        assert!(deflected);
        let ghast = world
            .ghasts()
            .iter()
            .find(|e| e.id == id)
            .expect("it is dying");
        assert!(
            ghast.ghast.health <= 0.0,
            "a ghast's own fireball sent back kills it outright"
        );
        let deaths = world.take_deaths();
        let death = deaths.iter().find(|d| d.id == id).unwrap();
        assert!(death.context.killed_by_player);
        assert_eq!(
            (death.table, death.experience),
            (Some("minecraft:ghast"), 5)
        );
    }
}
