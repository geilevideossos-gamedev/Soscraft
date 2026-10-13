use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::feline::{self, Feline, FelineKind};
use crate::feline_ai::FelinePlayer;
use crate::monster_ai::{MonsterKind, Target};

const SOUNDS: MovementSounds = MovementSounds::creature(None);

#[derive(Clone)]
pub struct FelineEntity {
    pub id: u64,
    pub feline: Feline,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
    pub effects: crate::effects::MobEffects,
    pub last_damage: Option<(&'static str, i64)>,
}

impl FelineEntity {
    pub fn position(&self) -> DVec3 {
        self.feline.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn tempted(&self) -> bool {
        self.ai.state.feline.tempt.running
    }

    pub fn running_goals(&self) -> Vec<&'static str> {
        self.ai.running_goals()
    }

    pub fn hurt_from(
        &mut self,
        amount: f32,
        kind: &'static str,
        attacker: Option<Target>,
        game_time: i64,
    ) -> DamageResult {
        if self.feline.health > 0.0 && !self.feline.damage.dead {
            self.no_action_time = 0;
        }
        let max = self.feline.max_health();
        let result = self
            .feline
            .damage
            .hurt_generic(&mut self.feline.health, max, amount);
        let position = self.position();
        self.feline
            .damage
            .place_death(result, position, self.feline.body.fire_ticks > 0);
        if result.applied {
            self.feline.in_love = 0;
            self.last_damage = Some((kind, game_time));
            if let Some(attacker) = attacker {
                self.ai.state.hurt_by = Some((attacker, self.tick_count));
            }
            if result.full {
                if !result.died {
                    self.ambient_sound_time = -self.feline.ambient_interval();
                }
                let sounds = self.feline.sounds();
                let pitch = voice_pitch(&mut self.random, self.feline.baby());
                let event = if result.died {
                    sounds.death
                } else {
                    sounds.hurt
                };
                self.voices
                    .push((Voice::Event(event, 1.0, pitch), position));
            }
        }
        result
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        self.hurt_from(amount, "minecraft:generic", None, 0)
    }

    fn ambient(&mut self) {
        if (self.random.next_int(1000) as i32) >= self.ambient_sound_time {
            self.ambient_sound_time += 1;
            return;
        }
        self.ambient_sound_time = -self.feline.ambient_interval();
        let sounds = self.feline.sounds();
        let event = match self.feline.kind {
            FelineKind::Ocelot => sounds.ambient,
            FelineKind::Cat if !self.feline.tame => sounds.stray_ambient,
            FelineKind::Cat if self.feline.in_love > 0 => sounds.purr,
            FelineKind::Cat if self.random.next_int(4) == 0 => sounds.purreow,
            FelineKind::Cat => sounds.ambient,
        };
        let pitch = voice_pitch(&mut self.random, self.feline.baby());
        self.voices
            .push((Voice::Event(event, 1.0, pitch), self.position()));
    }

    fn tick_ai(
        &mut self,
        world: &impl World,
        game_time: i64,
        owner: Option<u64>,
        people: Vec<FelinePlayer>,
    ) -> (Option<(Target, DVec3)>, Option<f64>) {
        let (max_health, eye) = (self.feline.max_health(), self.feline.eye_height());
        let (cat, tame, trusting, ordered_to_sit, sitting) = (
            self.feline.is_cat(),
            self.feline.tame,
            self.feline.trusting,
            self.feline.ordered_to_sit,
            self.feline.sitting,
        );
        let ai = &mut self.ai;
        let body = &mut self.feline.body;
        if ai.no_jump_delay > 0 {
            ai.no_jump_delay -= 1;
        }
        body.trim_small_velocity();
        ai.state.body = body.clone();
        ai.state.health = self.feline.health;
        ai.state.max_health = max_health;
        ai.state.game_time = game_time;
        ai.state.tick_count = self.tick_count;
        ai.state.on_fire = body.fire_ticks > 0;
        ai.state.eye_height = eye;
        let state = &mut ai.state.feline;
        state.cat = cat;
        state.tame = tame;
        state.trusting = trusting;
        state.owner = owner;
        state.ordered_to_sit = ordered_to_sit;
        state.sitting = sitting;
        state.people = people;
        state.last_damage = self
            .last_damage
            .filter(|&(_, at)| game_time - at <= 40)
            .map(|(kind, _)| kind);
        ai.state.random = std::mem::take(&mut self.random);
        self.no_action_time += 1;
        let position = body.position;
        let fluid = FluidFrame::sample(world, position, body.width, body.height);
        ai.state.fluid = fluid;
        ai.state.no_action_time = self.no_action_time;
        let full = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
        ai.tick_goals(world, full);
        let struck = ai.state.attack.take().map(|target| (target, body.position));
        body.position = ai.state.body.position;
        self.feline.sitting = ai.state.feline.sitting;
        if let Some(leap) = ai.state.leap.take() {
            body.velocity = leap;
        }
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
        let speed_modifier = ai
            .move_control
            .has_wanted()
            .then_some(ai.move_control.speed_modifier);
        self.feline.crouching = speed_modifier == Some(feline::TEMPT_SPEED);
        self.feline.sprinting = speed_modifier == Some(feline::SPRINT_SPEED);
        self.random = std::mem::take(&mut ai.state.random);
        let obstacle_top = crate::control::obstacle_top(world, position);
        let sprint = if self.feline.sprinting {
            1.0 + feline::SPRINT_SPEED_BONUS
        } else {
            1.0
        };
        let speed = self.effects.movement_speed(feline::MOVEMENT_SPEED) * sprint;
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
            eye,
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
            SOUNDS,
        );
        ai.body_rotation.tick(
            ai.yaw,
            &mut ai.state.look_control,
            self.previous_position,
            body.position,
        );
        self.feline.yaw = ai.yaw;
        (struck, landed)
    }

    fn tail(&mut self) {
        if self.feline.health <= 0.0 || !self.feline.is_cat() {
            return;
        }
        let position = self.position();
        if self.tempted() && !self.feline.tame && self.tick_count % 100 == 0 {
            self.voices.push((
                Voice::Event(self.feline.sounds().beg_for_food, 1.0, 1.0),
                position,
            ));
        }
        if (self.feline.lying || self.feline.relax_state_one) && self.tick_count % 5 == 0 {
            let (a, b) = (self.random.next_float(), self.random.next_float());
            self.voices.push((
                Voice::Event(self.feline.sounds().purr, 0.6 + 0.4 * (a - b), 1.0),
                position,
            ));
        }
        self.feline.update_lie_down_amount();
        self.feline.update_relax_state_one_amount();
    }
}

impl EntityWorld {
    pub fn spawn_feline(&mut self, feline: Feline, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let kind = match feline.kind {
            FelineKind::Cat => MonsterKind::Cat,
            FelineKind::Ocelot => MonsterKind::Ocelot,
        };
        let ai = MonsterAi::of_kind(kind, &feline.body, feline.yaw);
        self.felines.push(FelineEntity {
            id,
            previous_position: feline.body.position,
            feline,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
            effects: Default::default(),
            last_damage: None,
        });
        self.order.push(EntityKey::Feline(id));
        self.file_in_section(EntityKey::Feline(id));
        id
    }

    pub fn felines(&self) -> &[FelineEntity] {
        &self.felines
    }

    pub fn feline_mut(&mut self, id: u64) -> Option<&mut FelineEntity> {
        self.felines.iter_mut().find(|entity| entity.id == id)
    }

    fn feline_people(&self, players: &[PlayerCandidate], kind: FelineKind) -> Vec<FelinePlayer> {
        let tag = match kind {
            FelineKind::Cat => feline::FOOD_TAG,
            FelineKind::Ocelot => feline::OCELOT_FOOD_TAG,
        };
        let food = |item: &str| match &self.recipes {
            Some(recipes) => recipes.item_in_tag(tag, item),
            None => feline::is_food(item),
        };
        players
            .iter()
            .map(|p| {
                let controls = self.player_controls(p.id);
                FelinePlayer {
                    id: p.id,
                    position: p.position,
                    eye_height: p.eye_height,
                    yaw: controls.map_or(0.0, |c| c.yaw),
                    pitch: controls.map_or(0.0, |c| c.pitch),
                    sneaking: controls.is_some_and(|c| c.sneaking),
                    holds_food: controls.is_some_and(|c| c.holds(food)),
                }
            })
            .collect()
    }

    pub(super) fn tick_feline(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty, griefing) =
            (self.game_time, self.difficulty, self.mob_griefing);
        let Some((owner_uuid, kind)) = self
            .felines
            .iter()
            .find(|e| e.id == id)
            .map(|e| (e.feline.owner, e.feline.kind))
        else {
            return;
        };
        let owner = owner_uuid
            .and_then(|uuid| {
                players
                    .iter()
                    .find(|p| self.uuid_of(PLAYER_TARGET + p.id) == uuid)
            })
            .map(|p| p.id);
        let people = self.feline_people(players, kind);
        let mobs = self.mob_candidates();
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.previous_position = entity.feline.body.position;
        entity.tick_count += 1;
        base_tick_fluid(
            &mut entity.feline.body,
            &*world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            SOUNDS,
        );
        super::hazards::burn(entity, &*world, game_time);
        let eye = entity.feline.eye_height();
        super::hazards::suffocate(entity, &*world, eye, game_time);
        let water_breathing = entity.effects.has(MobEffect::WaterBreathing);
        if entity.feline.health > 0.0
            && super::living::breathe(&mut entity.feline.body, &*world, eye, water_breathing)
        {
            entity.hurt_from(2.0, "minecraft:drown", None, game_time);
        }
        let removed = entity.feline.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        let max = entity.feline.max_health();
        for work in entity.effects.tick(false) {
            match work.resolve(entity.feline.health, max) {
                Some(EffectWork::Heal(amount)) => heal(&mut entity.feline.health, max, amount),
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt_from(amount, "minecraft:magic", None, game_time);
                }
                _ => {}
            }
        }
        entity.feline.body.float_from(&entity.effects);
        if entity.feline.health > 0.0 {
            entity.ambient();
        }
        if removed {
            return;
        }
        let mut struck = None;
        let mut level_random = std::mem::take(&mut self.level_random);
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            self.level_random = level_random;
            return;
        };
        if entity.feline.health <= 0.0 && !entity.no_ai {
            let yaw = entity.feline.yaw;
            let landed = super::living::dying_travel(
                &mut entity.feline.body,
                &*world,
                yaw,
                entity.tick_count,
                &mut entity.random,
                &mut entity.voices,
                SOUNDS,
                Some(false),
            );
            if landed.is_some_and(|fallen| {
                super::living::fall_on_farmland(
                    &mut entity.feline.body,
                    world,
                    fallen,
                    &mut level_random,
                    griefing,
                )
            }) {
                entity.previous_position = entity.feline.body.position;
            }
        } else if !entity.no_ai {
            let state = &mut entity.ai.state;
            state.difficulty = difficulty;
            state.players = players.to_vec();
            state.mobs = mobs;
            let (hit, landed) = entity.tick_ai(&*world, game_time, owner, people);
            struck = hit;
            if let Some(fallen) = landed {
                if super::living::fall_on_farmland(
                    &mut entity.feline.body,
                    world,
                    fallen,
                    &mut level_random,
                    griefing,
                ) {
                    entity.previous_position = entity.feline.body.position;
                }
                let sounds = &mut entity.voices;
                if let Some(damage) =
                    fall_damage(&entity.feline.body, &*world, fallen, false, sounds)
                {
                    entity.hurt_from(damage, "minecraft:fall", None, game_time);
                }
            }
        } else {
            entity.feline.body.trim_small_velocity();
        }
        self.level_random = level_random;
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, &*world, old, game_time);
        if let Some((target, from)) = struck {
            match target {
                Target::Mob(victim) | Target::Villager(victim) => {
                    self.mob_hits_mob(id, victim, feline::ATTACK_DAMAGE, from, 0.0);
                }
                Target::Player(player_id) => {
                    self.player_hits.push(PlayerHit {
                        player_id,
                        damage: feline::ATTACK_DAMAGE,
                        kind: PlayerHitKind::Melee {
                            attacker: from,
                            hunger_ticks: 0,
                            lift: 0.0,
                            throw: glam::DVec3::ZERO,
                            disable_shield_seconds: 0.0,
                        },
                        source: Some(id),
                    });
                }
            }
        }
        self.push_entities(EntityKey::Feline(id), &*world, game_time, ticks);
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let was_baby = entity.feline.baby();
        let _ = entity.feline.age.tick(entity.feline.health > 0.0);
        if was_baby != entity.feline.baby() {
            let ticks = entity.feline.age.ticks;
            entity.feline.set_age(ticks);
        }
        if entity.feline.age.ticks != 0 {
            entity.feline.in_love = 0;
        } else if entity.feline.in_love > 0 {
            entity.feline.in_love -= 1;
            if entity.feline.in_love % 10 == 0 {
                for _ in 0..3 {
                    let _ = entity.random.next_gaussian();
                }
                for _ in 0..3 {
                    let _ = entity.random.next_double();
                }
            }
        }
        entity.tail();
        self.file_in_section(EntityKey::Feline(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};

    struct Floor;
    impl World for Floor {
        fn block(&self, (_, y, _): Pos) -> Option<Block> {
            (y < 64).then(|| Block::new("minecraft:grass_block"))
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
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

    #[test]
    fn a_cat_and_an_ocelot_are_live_mobs_of_their_kind() {
        let mut world = EntityWorld::default();
        let cat = world.spawn_feline(
            Feline::new(FelineKind::Cat, DVec3::new(0.5, 64.0, 0.5)),
            false,
        );
        let ocelot = world.spawn_feline(
            Feline::new(FelineKind::Ocelot, DVec3::new(4.5, 64.0, 0.5)),
            false,
        );
        assert_eq!(world.entity_type(cat), Some("minecraft:cat"));
        assert_eq!(world.entity_type(ocelot), Some("minecraft:ocelot"));
        assert_eq!(world.felines().len(), 2);
        let mut floor = Floor;
        for _ in 0..40 {
            world.tick(&mut floor);
        }
        let census = world.census();
        assert!(
            census.iter().any(|entry| entry.kind == "minecraft:cat")
                && census.iter().any(|entry| entry.kind == "minecraft:ocelot")
        );
        assert!(
            world
                .felines()
                .iter()
                .all(|e| e.feline.health == feline::MAX_HEALTH && e.feline.body.position.y >= 63.9)
        );
        let hit = world
            .felines
            .iter_mut()
            .find(|e| e.id == cat)
            .unwrap()
            .hurt(4.0);
        assert!(hit.applied);
        assert_eq!(world.felines()[0].feline.health, 6.0);
        assert!(
            world
                .mob_candidates()
                .iter()
                .any(|m| m.id == ocelot && m.kind == "minecraft:ocelot")
        );
    }

    #[test]
    fn a_wild_ocelot_runs_from_a_player_and_a_tempted_one_crouches_after_fish() {
        let mut world = EntityWorld::default();
        let id = world.spawn_feline(
            Feline::new(FelineKind::Ocelot, DVec3::new(0.5, 64.0, 0.5)),
            false,
        );
        world
            .felines
            .iter_mut()
            .find(|e| e.id == id)
            .unwrap()
            .set_random_seed(7);
        let players = [player(1, DVec3::new(5.5, 64.0, 0.5))];
        let mut floor = Floor;
        let mut tempting = world.clone();
        let mut ran = false;
        for _ in 0..80 {
            world.tick_with_players(&mut floor, &players);
            ran |= world.felines()[0]
                .running_goals()
                .contains(&"OcelotAvoidEntityGoal");
        }
        assert!(ran, "an ocelot that does not trust flees");
        assert!(
            world.felines()[0]
                .feline
                .body
                .position
                .distance(players[0].position)
                > 5.0
        );
        tempting.set_player_controls(&[crate::controls::PlayerControls {
            id: 1,
            main_hand: Some("minecraft:cod".to_owned()),
            ..Default::default()
        }]);
        let mut tempted = false;
        for _ in 0..200 {
            tempting.tick_with_players(&mut floor, &players);
            tempted |= tempting.felines()[0].tempted();
        }
        assert!(tempted, "raw cod tempts it");
    }
}
