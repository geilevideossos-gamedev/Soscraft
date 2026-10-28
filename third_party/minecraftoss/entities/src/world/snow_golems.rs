use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::monster_ai::MonsterKind;
use crate::snow_golem::{self, SnowGolem};

const SOUNDS: MovementSounds = MovementSounds::creature(None);

#[derive(Clone)]
pub struct SnowGolemEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub golem: SnowGolem,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnowballShot {
    pub shooter: u64,
    pub from: DVec3,
    pub direction: DVec3,
}

fn is_air(block: Option<&minecraftoss_player::Block>) -> bool {
    block.is_none_or(|b| {
        matches!(
            b.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

impl SnowGolemEntity {
    pub fn position(&self) -> DVec3 {
        self.golem.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.golem.health > 0.0 && !self.golem.damage.dead {
            self.no_action_time = 0;
        }
        let result =
            self.golem
                .damage
                .hurt_generic(&mut self.golem.health, snow_golem::MAX_HEALTH, amount);
        let position = self.position();
        self.golem
            .damage
            .place_death(result, position, self.golem.body.fire_ticks > 0);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -snow_golem::AMBIENT_INTERVAL;
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
    ) -> Option<(DVec3, f32, f32)> {
        let health = self.golem.health;
        let speed = self.effects.movement_speed(snow_golem::MOVEMENT_SPEED);
        let ai = &mut self.ai;
        let body = &mut self.golem.body;
        if ai.no_jump_delay > 0 {
            ai.no_jump_delay -= 1;
        }
        body.trim_small_velocity();
        self.no_action_time += 1;
        let position = body.position;
        let fluid = FluidFrame::sample(world, position, body.width, body.height);
        ai.state.body = body.clone();
        ai.state.health = health;
        ai.state.max_health = snow_golem::MAX_HEALTH;
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
        let shot = ai.state.snow_golem.shot.take();
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
        ai.state.look_control.tick(
            position,
            snow_golem::EYE_HEIGHT,
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
        self.golem.yaw = ai.yaw;
        if let Some(damage) = landed.and_then(|fallen| {
            super::living::fall_damage(&self.golem.body, world, fallen, false, &mut self.voices)
        }) {
            self.hurt(damage);
        }
        shot
    }

    fn hurt_by_water(&mut self, world: &impl World) {
        let p = self.golem.body.position;
        let feet = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let top = (
            feet.0,
            (p.y + f64::from(self.golem.body.height)).floor() as i32,
            feet.2,
        );
        if self.golem.body.touching_water || world.rain_at(feet) || world.rain_at(top) {
            self.hurt(snow_golem::WATER_DAMAGE);
        }
    }

    fn melt_and_trail(&mut self, world: &mut impl World, mob_griefing: bool) {
        let p = self.golem.body.position;
        let feet = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        if world.snow_golem_melts(feet) {
            self.hurt(snow_golem::MELT_DAMAGE);
        }
        if !mob_griefing {
            return;
        }
        let snow = minecraftoss_player::Block::new(snow_golem::SNOW);
        for cell in self.golem.trail_cells() {
            if is_air(world.block(cell).as_ref()) && world.block_survives(cell, &snow) {
                world.set_block(cell, Some(snow.clone()));
            }
        }
    }
}

fn adult_pitch(random: &mut LegacyRandom) -> f32 {
    let (a, b) = (random.next_float(), random.next_float());
    (a - b) * 0.2 + 1.0
}

impl EntityWorld {
    pub fn spawn_snow_golem(&mut self, golem: SnowGolem, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut ai = MonsterAi::of_kind(MonsterKind::SnowGolem, &golem.body, golem.yaw);
        ai.state.max_health = snow_golem::MAX_HEALTH;
        self.snow_golems.push(SnowGolemEntity {
            effects: Default::default(),
            id,
            previous_position: golem.body.position,
            golem,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
        });
        self.order.push(EntityKey::SnowGolem(id));
        self.file_in_section(EntityKey::SnowGolem(id));
        id
    }

    pub fn snow_golems(&self) -> &[SnowGolemEntity] {
        &self.snow_golems
    }

    pub fn snow_golem_mut(&mut self, id: u64) -> Option<&mut SnowGolemEntity> {
        self.snow_golems.iter_mut().find(|entity| entity.id == id)
    }

    pub fn take_snowball_shots(&mut self) -> Vec<SnowballShot> {
        std::mem::take(&mut self.snowball_shots)
    }

    pub fn shear_snow_golem(&mut self, id: u64) -> Option<(DVec3, f32)> {
        let entity = self
            .snow_golems
            .iter_mut()
            .find(|e| e.id == id && e.golem.health > 0.0)?;
        if !entity.golem.ready_for_shearing() {
            return None;
        }
        entity.golem.pumpkin = false;
        Some((entity.position(), snow_golem::EYE_HEIGHT))
    }

    pub fn snowball_damage(&self, victim: u64) -> f32 {
        if self.blazes.iter().any(|e| e.id == victim) {
            snow_golem::SNOWBALL_BLAZE_DAMAGE
        } else {
            0.0
        }
    }

    pub fn snowball_hits_mob(&mut self, shooter: u64, victim: u64, velocity: DVec3) -> bool {
        let Some(feet) = self
            .positions()
            .find(|&(id, _)| id == victim)
            .map(|(_, p)| p)
        else {
            return false;
        };
        let damage = self.snowball_damage(victim);
        let from = feet - DVec3::new(velocity.x, 0.0, velocity.z);
        self.mob_hits_mob(shooter, victim, damage, from, 0.0)
            .is_some_and(|hit| hit.applied)
    }

    pub(super) fn tick_snow_golem(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty, mob_griefing) =
            (self.game_time, self.difficulty, self.mob_griefing);
        let mobs = self.mob_candidates();
        let Some(entity) = self.snow_golems.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.ai.state.mobs = mobs;
        entity.previous_position = entity.golem.body.position;
        entity.tick_count += 1;
        base_tick_fluid(
            &mut entity.golem.body,
            &*world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            SOUNDS,
        );
        super::hazards::burn(entity, &*world, game_time);
        super::hazards::suffocate(entity, &*world, snow_golem::EYE_HEIGHT, game_time);
        let water_breathing = entity
            .effects
            .has(crate::effects::MobEffect::WaterBreathing);
        if entity.golem.health > 0.0
            && super::living::breathe(
                &mut entity.golem.body,
                &*world,
                snow_golem::EYE_HEIGHT,
                water_breathing,
            )
        {
            entity.hurt(2.0);
        }
        let removed = entity.golem.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        for work in entity.effects.tick(false) {
            match work.resolve(entity.golem.health, snow_golem::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut entity.golem.health, snow_golem::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt(amount);
                }
                _ => {}
            }
        }
        entity.golem.body.float_from(&entity.effects);
        if entity.golem.health > 0.0 {
            if (entity.random.next_int(1000) as i32) < entity.ambient_sound_time {
                entity.ambient_sound_time = -snow_golem::AMBIENT_INTERVAL;
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
        if entity.golem.health <= 0.0 && !entity.no_ai {
            let yaw = entity.golem.yaw;
            let _ = super::living::dying_travel(
                &mut entity.golem.body,
                &*world,
                yaw,
                entity.tick_count,
                &mut entity.random,
                &mut entity.voices,
                SOUNDS,
                Some(false),
            );
        } else if !entity.no_ai {
            shot = entity.tick_ai(&*world, players, game_time, difficulty);
        } else {
            entity.golem.body.trim_small_velocity();
        }
        entity.hurt_by_water(&*world);
        entity.melt_and_trail(world, mob_griefing);
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, &*world, old, game_time);
        if let Some((target, eye_height, pitch)) = shot {
            let from = entity.golem.snowball_start();
            let direction = entity.golem.snowball_aim(target, eye_height);
            entity.voices.push((
                Voice::Event(snow_golem::SHOOT_SOUND, 1.0, pitch),
                entity.position(),
            ));
            self.snowball_shots.push(SnowballShot {
                shooter: id,
                from,
                direction,
            });
        }
        self.push_entities(EntityKey::SnowGolem(id), &*world, game_time, ticks);
        self.file_in_section(EntityKey::SnowGolem(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monster_ai::Target;
    use minecraftoss_player::{Block, Pos};
    use std::collections::HashMap;

    #[derive(Default)]
    struct Field {
        placed: HashMap<Pos, Block>,
        rain: bool,
        melts: bool,
        stone: bool,
    }

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            if pos.1 < 64 {
                let id = if self.stone {
                    "minecraft:stone"
                } else {
                    "minecraft:grass_block"
                };
                return Some(Block::new(id));
            }
            self.placed.get(&pos).cloned()
        }
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => {
                    self.placed.insert(pos, block);
                }
                None => {
                    self.placed.remove(&pos);
                }
            }
        }
        fn rain_at(&self, _: Pos) -> bool {
            self.rain
        }
        fn snow_golem_melts(&self, _: Pos) -> bool {
            self.melts
        }
        fn block_survives(&self, pos: Pos, _: &Block) -> bool {
            pos.1 == 64
        }
    }

    fn golem_at(world: &mut EntityWorld, at: DVec3, no_ai: bool) -> u64 {
        let id = world.spawn_snow_golem(SnowGolem::new(at), no_ai);
        world.snow_golem_mut(id).unwrap().set_random_seed(5);
        id
    }

    #[test]
    fn a_snow_golem_is_a_live_mob_of_its_size_and_health() {
        let mut world = EntityWorld::default();
        let id = golem_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        assert_eq!(world.entity_type(id), Some(snow_golem::TYPE_ID));
        let entity = &world.snow_golems()[0];
        assert_eq!(
            (
                entity.golem.body.width,
                entity.golem.body.height,
                entity.golem.health
            ),
            (0.7, 1.9, 4.0)
        );
        assert!(world.census().iter().any(|e| e.kind == snow_golem::TYPE_ID));
        assert!(world.can_be_leashed(id), "golems take a lead");
        assert!(world.mob_candidates().iter().any(|m| m.id == id));
        let hit = world.snow_golem_mut(id).unwrap().hurt(1.0);
        assert!(hit.applied);
        assert_eq!(world.snow_golems()[0].golem.health, 3.0);
    }

    #[test]
    fn a_snow_golem_leaves_snow_where_it_can_lie_unless_mobs_may_not_grief() {
        let mut world = EntityWorld::default();
        golem_at(&mut world, DVec3::new(10.1, 64.0, -3.9), true);
        let mut field = Field::default();
        world.tick(&mut field);
        let mut cells: Vec<Pos> = field.placed.keys().copied().collect();
        cells.sort();
        assert_eq!(
            cells,
            vec![(9, 64, -5), (9, 64, -4), (10, 64, -5), (10, 64, -4)]
        );
        assert!(field.placed.values().all(|b| b.id == snow_golem::SNOW));
        let mut world = EntityWorld::default();
        golem_at(&mut world, DVec3::new(10.1, 64.0, -3.9), true);
        world.set_mob_griefing(false);
        let mut field = Field::default();
        world.tick(&mut field);
        assert!(field.placed.is_empty(), "no trail without mob griefing");
    }

    #[test]
    fn a_snow_golem_melts_where_the_biome_is_too_warm_and_rain_hurts_it() {
        let mut world = EntityWorld::default();
        let id = golem_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let mut field = Field {
            melts: true,
            ..Field::default()
        };
        world.tick(&mut field);
        let health = world
            .snow_golems()
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .golem
            .health;
        assert_eq!(health, 3.0, "it melts a point at a time");
        let mut world = EntityWorld::default();
        let id = golem_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let mut field = Field {
            rain: true,
            ..Field::default()
        };
        world.tick(&mut field);
        let health = world
            .snow_golems()
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .golem
            .health;
        assert_eq!(health, 3.0, "rain hurts it");
        let mut world = EntityWorld::default();
        let id = golem_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let mut field = Field::default();
        for _ in 0..40 {
            world.tick(&mut field);
        }
        let health = world
            .snow_golems()
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .golem
            .health;
        assert_eq!(health, 4.0, "dry and cold it keeps its health");
    }

    #[test]
    fn a_snow_golem_throws_snowballs_at_a_monster_in_range() {
        let mut world = EntityWorld::default();
        let id = golem_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let zombie =
            world.spawn_zombie(crate::zombie::Zombie::new(DVec3::new(0.5, 64.0, 8.5)), true);
        let mut field = Field {
            stone: true,
            ..Field::default()
        };
        let mut shots = Vec::new();
        let mut sounds = Vec::new();
        for _ in 0..120 {
            world.tick(&mut field);
            shots.extend(world.take_snowball_shots());
            sounds.extend(world.take_sounds());
        }
        assert!(shots.len() >= 2, "a snowball a second: {}", shots.len());
        let shot = shots[0];
        assert_eq!(shot.shooter, id);
        assert!(
            shot.direction.z > 7.0 && shot.direction.x.abs() < 1.0,
            "{shot:?}"
        );
        assert!(
            sounds
                .iter()
                .any(|s| s.event == snow_golem::SHOOT_SOUND && s.volume == 1.0)
        );
        let target = world.snow_golems()[0].ai.state.target;
        assert_eq!(target, Some(Target::Mob(zombie)));
    }

    #[test]
    fn a_snow_golem_leaves_players_and_animals_alone() {
        let mut world = EntityWorld::default();
        golem_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        world.spawn_cow(crate::cow::Cow::new(DVec3::new(0.5, 64.0, 6.5)), true);
        let mut field = Field {
            stone: true,
            ..Field::default()
        };
        for _ in 0..120 {
            world.tick(&mut field);
            assert!(world.take_snowball_shots().is_empty());
        }
    }

    #[test]
    fn shears_take_its_pumpkin_once() {
        let mut world = EntityWorld::default();
        let id = golem_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let (at, eye) = world.shear_snow_golem(id).expect("it wears its pumpkin");
        assert_eq!(
            (at, eye),
            (DVec3::new(0.5, 64.0, 0.5), snow_golem::EYE_HEIGHT)
        );
        assert!(!world.snow_golems()[0].golem.pumpkin);
        assert!(
            world.shear_snow_golem(id).is_none(),
            "nothing left to shear"
        );
    }

    #[test]
    fn its_snowball_turns_a_zombie_on_it_and_burns_a_blaze() {
        let mut world = EntityWorld::default();
        let golem = golem_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let zombie =
            world.spawn_zombie_active(crate::zombie::Zombie::new(DVec3::new(0.5, 64.0, 8.5)), 0.0);
        assert!(world.snowball_hits_mob(golem, zombie, DVec3::new(0.0, 0.0, 1.5)));
        let hurt = world.zombies()[0].ai.as_ref().unwrap().state.hurt_by;
        assert_eq!(hurt.map(|(t, _)| t), Some(Target::Mob(golem)));
        assert_eq!(
            world.zombies()[0].zombie.health,
            20.0,
            "a snowball does no damage"
        );
        assert!(
            world.zombies()[0].zombie.body.velocity.z > 0.0,
            "it is knocked away"
        );
        let blaze = world.spawn_blaze(crate::blaze::Blaze::new(DVec3::new(4.5, 64.0, 8.5)), true);
        assert!(world.snowball_hits_mob(golem, blaze, DVec3::new(0.0, 0.0, 1.5)));
        assert_eq!(
            world.blazes()[0].blaze.health,
            crate::blaze::MAX_HEALTH - 3.0
        );
    }
}
