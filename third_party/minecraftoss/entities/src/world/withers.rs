use super::emissions::{MovementSounds, play_movement};
use super::*;
use crate::fireball::{Fireball, FireballKind, Shooter};
use crate::monster_ai::{MobCandidate, MonsterGoalContext, MonsterKind, Target, TargetInfo};
use crate::wither::{self as wt, SkullShot, Wither};

const SOUNDS: MovementSounds = MovementSounds::monster(None);

#[derive(Clone)]
pub struct WitherEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub wither: Wither,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
    pub main_target: Option<Target>,
    pub side_targets: [Option<Target>; 2],
    pub shots: Vec<SkullShot>,
}

fn adult_pitch(random: &mut LegacyRandom) -> f32 {
    let (a, b) = (random.next_float(), random.next_float());
    (a - b) * 0.2 + 1.0
}

fn next_double(random: &mut LegacyRandom, min: f64, max: f64) -> f64 {
    if min >= max {
        min
    } else {
        random.next_double() * (max - min) + min
    }
}

fn eye_of(info: TargetInfo) -> DVec3 {
    info.position + DVec3::new(0.0, f64::from(info.eye_height), 0.0)
}

fn in_search_box(wither: &Body, info: TargetInfo) -> bool {
    let p = wither.position;
    let half = f64::from(wither.width / 2.0);
    let reach = wt::SIDE_SEARCH;
    let other = f64::from(info.width / 2.0);
    let q = info.position;
    q.x + other > p.x - half - reach.x
        && q.x - other < p.x + half + reach.x
        && q.y + f64::from(info.height) > p.y - reach.y
        && q.y < p.y + f64::from(wither.height) + reach.y
        && q.z + other > p.z - half - reach.z
        && q.z - other < p.z + half + reach.z
}

pub fn view_vector(yaw: f32, pitch: f32) -> DVec3 {
    let x = pitch * 0.017_453_292;
    let y = -yaw * 0.017_453_292;
    let cos_y = minecraftoss_player::mth::cos(f64::from(y));
    let sin_y = minecraftoss_player::mth::sin(f64::from(y));
    let cos_x = minecraftoss_player::mth::cos(f64::from(x));
    let sin_x = minecraftoss_player::mth::sin(f64::from(x));
    DVec3::new(
        f64::from(sin_y * cos_x),
        f64::from(-sin_x),
        f64::from(cos_y * cos_x),
    )
}

pub(crate) fn side_heads(
    ctx: &mut MonsterGoalContext,
    world: &dyn World,
    wither: &mut Wither,
    sides: &mut [Option<Target>; 2],
    tick_count: i32,
) -> Vec<SkullShot> {
    let mut shots = Vec::new();
    for head in 1..3 {
        let i = head - 1;
        if tick_count < wither.next_head_update[i] {
            continue;
        }
        wither.next_head_update[i] =
            tick_count + wt::HEAD_UPDATE_BASE + ctx.random.next_int(wt::HEAD_UPDATE_SPREAD) as i32;
        if ctx.difficulty >= 2 {
            let idle = wither.idle_head_updates[i];
            wither.idle_head_updates[i] += 1;
            if idle > wt::IDLE_HEAD_LIMIT {
                let p = wither.body.position;
                let s = wt::IDLE_SHOT_SPREAD;
                let x = next_double(&mut ctx.random, p.x - s.x, p.x + s.x);
                let y = next_double(&mut ctx.random, p.y - s.y, p.y + s.y);
                let z = next_double(&mut ctx.random, p.z - s.z, p.z + s.z);
                shots.push(SkullShot {
                    head,
                    target: DVec3::new(x, y, z),
                    dangerous: true,
                });
                wither.idle_head_updates[i] = 0;
            }
        }
        match sides[i] {
            Some(target) => {
                let kept = ctx.info(target).filter(|&info| {
                    ctx.can_attack(info)
                        && wither.body.position.distance_squared(info.position)
                            <= wt::SIDE_TARGET_RANGE_SQR
                        && crate::sight::line_of_sight(world, wither.eye(), eye_of(info))
                });
                match kept {
                    None => sides[i] = None,
                    Some(info) => {
                        let aim =
                            info.position + DVec3::new(0.0, f64::from(info.eye_height) * 0.5, 0.0);
                        shots.push(SkullShot {
                            head,
                            target: aim,
                            dangerous: false,
                        });
                        wither.next_head_update[i] = tick_count
                            + wt::SHOT_UPDATE_BASE
                            + ctx.random.next_int(wt::SHOT_UPDATE_SPREAD) as i32;
                        wither.idle_head_updates[i] = 0;
                    }
                }
            }
            None => {
                let near: Vec<Target> = crate::wither_ai::prey(ctx)
                    .into_iter()
                    .filter(|&target| {
                        ctx.info(target).is_some_and(|info| {
                            ctx.can_attack(info)
                                && in_search_box(&wither.body, info)
                                && wither.body.position.distance_squared(info.position)
                                    <= wt::SIDE_SEARCH_RANGE * wt::SIDE_SEARCH_RANGE
                        })
                    })
                    .collect();
                let near: Vec<Target> = near
                    .into_iter()
                    .filter(|&target| {
                        let info = ctx.info(target).expect("listed");
                        ctx.sees(world, info)
                    })
                    .collect();
                if !near.is_empty() {
                    let pick = ctx.random.next_int(near.len() as u32) as usize;
                    sides[i] = Some(near[pick]);
                }
            }
        }
    }
    shots
}

impl WitherEntity {
    pub fn position(&self) -> DVec3 {
        self.wither.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    fn voice_hurt(&mut self, result: DamageResult) {
        let position = self.position();
        self.wither
            .damage
            .place_death(result, position, self.wither.body.fire_ticks > 0);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -wt::AMBIENT_INTERVAL;
            }
            let voice = hurt_voice(&mut self.random, result.died, false);
            self.voices.push((voice, position));
        }
    }

    pub fn hurt_typed(
        &mut self,
        amount: f32,
        damage_type: &str,
        by_undead: bool,
        bypasses_armor: bool,
    ) -> DamageResult {
        let refused = DamageResult {
            applied: false,
            dealt: 0.0,
            died: false,
            full: false,
        };
        let bypasses_invulnerability = matches!(
            damage_type,
            "minecraft:out_of_world" | "minecraft:generic_kill"
        );
        if self
            .wither
            .refuses(damage_type, by_undead, bypasses_invulnerability)
        {
            return refused;
        }
        if self.wither.health > 0.0 && !self.wither.damage.dead {
            self.no_action_time = 0;
        }
        self.wither.note_hurt();
        let result = if bypasses_armor {
            self.wither.hurt_unarmored(amount)
        } else {
            self.wither.hurt_armored(amount)
        };
        self.voice_hurt(result);
        result
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        self.hurt_typed(amount, "minecraft:generic", false, false)
    }

    pub fn hurt_by(&mut self, amount: f32, attacker: Option<&str>) -> DamageResult {
        let undead = attacker.is_some_and(wt::is_undead);
        self.hurt_typed(amount, "minecraft:mob_attack", undead, false)
    }

    pub fn hurt_by_magic(&mut self, amount: f32) -> DamageResult {
        self.hurt_typed(amount, "minecraft:magic", false, true)
    }

    fn tick_ai(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        villagers: &[MobCandidate],
        game_time: i64,
        difficulty: i32,
    ) -> (bool, bool) {
        self.no_action_time = 0;
        let health = self.wither.health;
        let powered = self.wither.powered();
        let chase = self
            .main_target
            .and_then(|t| self.ai.state.info(t))
            .map(|info| info.position);
        let ai = &mut self.ai;
        let wither = &mut self.wither;
        wither.body.velocity =
            wt::chase(wither.body.velocity, wither.body.position, chase, powered);
        if let Some(yaw) = wt::facing(wither.body.velocity) {
            ai.yaw = yaw;
        }
        wither.body.trim_small_velocity();
        wither.forward *= 0.98;
        let position = wither.body.position;
        let fluid = FluidFrame::sample(world, position, wither.body.width, wither.body.height);
        ai.state.body = wither.body.clone();
        ai.state.health = health;
        ai.state.max_health = wt::MAX_HEALTH;
        ai.state.game_time = game_time;
        ai.state.difficulty = difficulty;
        ai.state.players = players.to_vec();
        ai.state.villagers = villagers.to_vec();
        ai.state.fluid = fluid;
        ai.state.no_action_time = self.no_action_time;
        ai.state.tick_count = self.tick_count;
        ai.state.wither.id = self.id;
        ai.state.wither.charging = wither.charging();
        ai.state.wither.view = view_vector(ai.yaw, ai.state.look_control.pitch);
        ai.state.random = std::mem::take(&mut self.random);
        let full = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
        ai.tick_goals(world, full);
        let mut shots = Vec::new();
        if let Some(target) = ai.state.wither.shot.take() {
            if let Some(info) = ai.state.info(target) {
                let dangerous = ai.state.random.next_float() < wt::DANGEROUS_CHANCE;
                shots.push(SkullShot {
                    head: 0,
                    target: info.position + DVec3::new(0.0, f64::from(info.eye_height) * 0.5, 0.0),
                    dangerous,
                });
            }
        }
        let wanted = ai
            .state
            .wither
            .flight
            .tick(position, wither.body.width, wt::SPEED as f32);
        let mut explode = false;
        let mut breaks = false;
        if wither.charging() {
            explode = wither.charge_step(self.tick_count).explode;
        } else {
            shots.extend(side_heads(
                &mut ai.state,
                world,
                wither,
                &mut self.side_targets,
                self.tick_count,
            ));
            self.main_target = ai.state.target().map(|t| t.target);
            breaks = wither.destroy_blocks_step();
            if self.tick_count % wt::HEAL_INTERVAL == 0 {
                wither.heal(wt::HEAL);
            }
        }
        self.random = std::mem::take(&mut ai.state.random);
        self.shots = shots;
        let flight = wt::fly(wanted, position, ai.yaw, wither.forward, wither.vertical);
        ai.yaw = flight.yaw;
        wither.forward = flight.forward;
        wither.vertical = flight.vertical;
        wither.body.gravity = if flight.no_gravity { 0.0 } else { 0.08 };
        ai.state.look_control.tick(
            position,
            wt::EYE_HEIGHT,
            ai.body_rotation.body_yaw,
            wanted.is_some(),
        );
        let input = DVec3::new(0.0, f64::from(wither.vertical), f64::from(wither.forward));
        let body = &mut wither.body;
        if fluid.in_water() {
            body.travel_water(world, input, ai.yaw);
        } else if fluid.in_lava() {
            body.travel_lava(world, input, ai.yaw, fluid.lava_height);
        } else {
            let _ = body.travel_air_flying(world, input, flight.speed, ai.yaw, false, 0.02);
        }
        body.fall_distance = 0.0;
        play_movement(
            body,
            self.tick_count,
            &mut self.random,
            &mut self.voices,
            SOUNDS,
        );
        let eyes = self
            .side_targets
            .map(|side| side.and_then(|t| ai.state.info(t)).map(eye_of));
        wither.turn_heads(eyes);
        ai.body_rotation.tick(
            ai.yaw,
            &mut ai.state.look_control,
            self.previous_position,
            wither.body.position,
        );
        wither.yaw = ai.yaw;
        wither.body_yaw = ai.body_rotation.body_yaw;
        (explode, breaks)
    }

    fn fire_skulls(&mut self) -> Vec<Fireball> {
        let mut skulls = Vec::new();
        for shot in self.shots.clone() {
            let p = self.position();
            let at = DVec3::new(p.x.floor() + 0.5, p.y.floor() + 0.5, p.z.floor() + 0.5);
            let pitch = (self.random.next_float() - self.random.next_float()) * 0.2 + 1.0;
            self.voices
                .push((Voice::Event(wt::SHOOT_SOUND, wt::EVENT_VOLUME, pitch), at));
            let head = self.wither.head(shot.head);
            let mut skull = Fireball::shoot(
                FireballKind::WitherSkull {
                    dangerous: shot.dangerous,
                },
                Shooter::Mob(self.id),
                head,
                shot.target - head,
            );
            skull.yaw = self.wither.yaw;
            skull.pitch = self.ai.state.look_control.pitch;
            skulls.push(skull);
        }
        skulls
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitherBreak {
    pub wither: u64,
    pub min: (i32, i32, i32),
    pub max: (i32, i32, i32),
    pub at: (i32, i32, i32),
}

impl WitherBreak {
    pub fn around(wither: u64, body: &Body) -> Self {
        let (reach, height) = wt::destroy_reach(body.width, body.height);
        let p = body.position;
        let at = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        Self {
            wither,
            min: (at.0 - reach, at.1, at.2 - reach),
            max: (at.0 + reach, at.1 + height, at.2 + reach),
            at,
        }
    }

    pub fn cells(&self) -> Vec<(i32, i32, i32)> {
        let mut cells = Vec::new();
        for z in self.min.2..=self.max.2 {
            for y in self.min.1..=self.max.1 {
                for x in self.min.0..=self.max.0 {
                    cells.push((x, y, z));
                }
            }
        }
        cells
    }
}

impl EntityWorld {
    pub fn spawn_wither(&mut self, wither: Wither, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut ai = MonsterAi::of_kind(MonsterKind::Wither, &wither.body, wither.yaw);
        ai.state.max_health = wt::MAX_HEALTH;
        ai.state.wither.id = id;
        ai.body_rotation.body_yaw = wither.body_yaw;
        self.withers.push(WitherEntity {
            effects: Default::default(),
            id,
            previous_position: wither.body.position,
            wither,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
            main_target: None,
            side_targets: [None; 2],
            shots: Vec::new(),
        });
        self.order.push(EntityKey::Wither(id));
        self.file_in_section(EntityKey::Wither(id));
        id
    }

    pub fn withers(&self) -> &[WitherEntity] {
        &self.withers
    }

    pub fn take_wither_breaks(&mut self) -> Vec<WitherBreak> {
        std::mem::take(&mut self.wither_breaks)
    }

    pub fn wither_mut(&mut self, id: u64) -> Option<&mut WitherEntity> {
        self.withers.iter_mut().find(|entity| entity.id == id)
    }

    pub(super) fn tick_wither(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
    ) {
        let (game_time, difficulty) = (self.game_time, self.difficulty);
        let mobs = self.mob_candidates();
        let villagers = self.illager_villagers();
        let Some(entity) = self.withers.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.ai.state.mobs = mobs;
        entity.previous_position = entity.wither.body.position;
        entity.tick_count += 1;
        entity.shots.clear();
        let removed = entity.wither.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        entity.effects = Default::default();
        if entity.wither.health > 0.0 {
            if (entity.random.next_int(1000) as i32) < entity.ambient_sound_time {
                entity.ambient_sound_time = -wt::AMBIENT_INTERVAL;
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
        let mut explode = false;
        let mut breaks = false;
        if entity.wither.health > 0.0 && !entity.no_ai {
            (explode, breaks) = entity.tick_ai(&*world, players, &villagers, game_time, difficulty);
        } else if entity.no_ai {
            monster_idle_without_ai(
                &*world,
                players,
                entity.wither.body.position,
                wt::EYE_HEIGHT,
                &mut entity.no_action_time,
            );
        }
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, &*world, old, game_time);
        let skulls = entity.fire_skulls();
        let broken =
            (breaks && self.mob_griefing).then(|| WitherBreak::around(id, &entity.wither.body));
        self.wither_breaks.extend(broken);
        for skull in skulls {
            self.spawn_fireball(skull);
        }
        if explode {
            let entity = self
                .withers
                .iter_mut()
                .find(|e| e.id == id)
                .expect("ticking");
            let blast = CreeperExplosion {
                position: entity.wither.eye(),
                radius: wt::BLAST_RADIUS,
                powered: false,
            };
            let at = entity.position();
            entity
                .voices
                .push((Voice::Event(wt::SPAWN_SOUND, 1.0, 1.0), at));
            self.apply_explosion(&*world, Some(id), blast, players);
            self.explosions.push(blast);
        }
        self.file_in_section(EntityKey::Wither(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn player_at(id: u64, x: f64, y: f64, z: f64) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position: DVec3::new(x, y, z),
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

    fn wither_at(world: &mut EntityWorld, wither: Wither) -> u64 {
        let id = world.spawn_wither(wither, false);
        world.wither_mut(id).unwrap().set_random_seed(5);
        id
    }

    #[test]
    fn a_wither_is_a_live_monster_of_its_size_and_health() {
        let mut world = EntityWorld::default();
        let id = wither_at(&mut world, Wither::new(DVec3::new(0.5, 70.0, 0.5)));
        assert_eq!(world.entity_type(id), Some(wt::TYPE_ID));
        let e = &world.withers()[0];
        assert_eq!(
            (e.wither.body.width, e.wither.body.height, e.wither.health),
            (0.9, 3.5, 300.0)
        );
        assert_eq!(e.ai.state.follow_range, 40.0);
        assert_eq!(
            MonsterKind::Wither.goal_names().len(),
            crate::wither_ai::GOAL_NAMES.len()
        );
        assert!(crate::monster_ai::is_enemy(wt::TYPE_ID));
        assert!(world.census().iter().any(|c| c.kind == wt::TYPE_ID));
        assert!(world.mob_candidates().iter().any(|m| m.id == id));
        let attack = PlayerAttack {
            player_id: 1,
            position: DVec3::new(0.5, 70.0, 3.5),
            yaw: 180.0,
            attack_damage: 1000.0,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        };
        assert!(world.player_attack(&attack, id).died);
        let deaths = world.take_deaths();
        assert_eq!(deaths[0].table, Some(wt::TYPE_ID));
        assert_eq!(deaths[0].experience, wt::EXPERIENCE);
    }

    #[test]
    fn a_built_wither_cannot_be_hurt_while_it_charges_then_blows_up_after_220_ticks() {
        let mut world = EntityWorld::default();
        world.set_players_pickable(true);
        let id = wither_at(&mut world, Wither::built(DVec3::new(0.5, 64.0, 0.5), 90.0));
        let players = [player_at(1, 0.5, 64.0, 4.5)];
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
        assert!(!world.player_attack(&attack, id).hurt);
        let mut blasts = Vec::new();
        let mut sounds = Vec::new();
        let mut healths = Vec::new();
        for tick in 1..=230 {
            world.tick_with_players(&mut Field, &players);
            for blast in world.take_creeper_explosions() {
                let eye = world.withers()[0].wither.eye();
                blasts.push((tick, blast, eye));
            }
            sounds.extend(world.take_sounds().into_iter().map(|s| (tick, s.event)));
            let e = world.withers().iter().find(|e| e.id == id).unwrap();
            if tick == 10 || tick == 100 || tick == 230 {
                healths.push(e.wither.health);
            }
            if tick < 220 {
                assert!(e.wither.charging(), "{tick}");
                assert!(e.shots.is_empty() && e.side_targets == [None, None]);
            }
        }
        assert_eq!(blasts.len(), 1, "one blast");
        let (tick, blast, eye) = blasts[0];
        assert_eq!(tick, 220);
        assert_eq!(blast.radius, 7.0);
        assert_eq!(blast.position, eye, "it goes off at the eyes");
        assert!(
            sounds
                .iter()
                .any(|(t, s)| *t == 220 && s == wt::SPAWN_SOUND),
            "{sounds:?}"
        );
        assert_eq!(healths[0], 110.0);
        assert_eq!(healths[1], 200.0);
        assert_eq!(healths[2], 300.0);
        let hits = world.take_player_hits();
        assert!(
            hits.iter()
                .any(|h| h.player_id == 1 && matches!(h.kind, PlayerHitKind::Explosion { .. })),
            "the blast hurts the builder standing beside it"
        );
        assert!(
            world.player_attack(&attack, id).hurt,
            "charged up it can be hurt"
        );
    }

    #[test]
    fn a_wither_flies_to_its_target_and_its_side_heads_pick_their_own() {
        let mut world = EntityWorld::default();
        let id = wither_at(&mut world, Wither::new(DVec3::new(0.5, 66.0, 0.5)));
        let players = [player_at(1, 0.5, 64.0, 30.5)];
        let cow = world.spawn_cow(crate::cow::Cow::new(DVec3::new(0.5, 64.0, 58.5)), true);
        let zombie = world.spawn_zombie(
            crate::zombie::Zombie::new(DVec3::new(-6.5, 64.0, 0.5)),
            true,
        );
        let mut shots = Vec::new();
        let mut highest = 0.0_f64;
        for _ in 0..200 {
            world.tick_with_players(&mut Field, &players);
            let e = world.withers().iter().find(|e| e.id == id).unwrap();
            shots.extend(e.shots.iter().copied());
            highest = highest.max(e.wither.body.position.y);
        }
        let e = world.withers().iter().find(|e| e.id == id).unwrap();
        assert_eq!(
            e.ai.state.target,
            Some(Target::Player(1)),
            "the nearest living is its main target"
        );
        assert!(
            e.wither.body.position.z > 10.0,
            "it flew towards the player: {}",
            e.wither.body.position
        );
        assert!(highest > 68.0, "it keeps above its target: {highest}");
        assert!(
            e.side_targets
                .iter()
                .all(|t| *t != Some(Target::Mob(zombie))),
            "the undead are its friends"
        );
        assert!(
            e.side_targets
                .iter()
                .any(|t| matches!(t, Some(Target::Player(1)) | Some(Target::Mob(_)))),
            "{:?}",
            e.side_targets
        );
        assert!(
            shots.iter().any(|s| s.head == 0 && !s.dangerous),
            "the centre head aims at its target"
        );
        assert!(
            shots.iter().any(|s| s.head > 0),
            "the side heads aim at theirs"
        );
        assert!(
            e.side_targets
                .iter()
                .flatten()
                .all(|t| *t == Target::Player(1) || *t == Target::Mob(cow))
        );
    }

    fn skull(world: &mut EntityWorld, owner: Option<u64>, from: DVec3, dangerous: bool) -> u64 {
        let mut skull = Fireball::shoot(
            FireballKind::WitherSkull { dangerous },
            Shooter::Mob(owner.unwrap_or(0)),
            from,
            DVec3::Z,
        );
        skull.owner = owner.map(Shooter::Mob);
        world.spawn_fireball(skull)
    }

    fn skull_hits(difficulty: i32, owned: bool) -> (Vec<PlayerHit>, u64) {
        let mut world = EntityWorld::default();
        world.set_players_pickable(true);
        world.set_difficulty(difficulty);
        let id = world.spawn_wither(Wither::new(DVec3::new(0.5, 70.0, -12.5)), true);
        skull(
            &mut world,
            owned.then_some(id),
            DVec3::new(0.5, 65.0, 0.5),
            false,
        );
        let players = [player_at(1, 0.5, 64.0, 4.5)];
        let mut hits = Vec::new();
        for _ in 0..20 {
            world.tick_with_players(&mut Field, &players);
            hits.extend(world.take_player_hits());
        }
        let skulls = hits
            .into_iter()
            .filter(|h| matches!(h.kind, PlayerHitKind::WitherSkull { .. }))
            .collect();
        (skulls, id)
    }

    #[test]
    fn its_heads_fire_wither_skulls_with_the_shoot_sound() {
        let mut world = EntityWorld::default();
        let id = wither_at(&mut world, Wither::new(DVec3::new(0.5, 66.0, 0.5)));
        let players = [player_at(1, 0.5, 64.0, 12.5)];
        let mut sounds = Vec::new();
        let mut skulls = Vec::new();
        for _ in 0..200 {
            world.tick_with_players(&mut Field, &players);
            sounds.extend(world.take_sounds());
            for f in world.fireballs() {
                if !skulls.contains(&f.id) {
                    assert_eq!(f.fireball.owner, Some(Shooter::Mob(id)));
                    assert!(f.fireball.kind.wither_skull());
                    skulls.push(f.id);
                }
            }
        }
        assert!(skulls.len() >= 3, "{} skulls", skulls.len());
        let shots = sounds
            .iter()
            .filter(|s| s.event == wt::SHOOT_SOUND && s.volume == 2.0)
            .count();
        assert!(
            shots >= skulls.len(),
            "{shots} shots for {} skulls",
            skulls.len()
        );
    }

    #[test]
    fn a_skull_hits_a_player_for_eight_and_withers_them_ten_or_forty_seconds() {
        let wither_ticks = |hits: &[PlayerHit]| match hits {
            [
                PlayerHit {
                    kind:
                        PlayerHitKind::WitherSkull {
                            wither_ticks,
                            velocity,
                        },
                    ..
                },
            ] => {
                assert!(velocity.z > 0.0);
                Some(*wither_ticks)
            }
            _ => None,
        };
        let (easy, _) = skull_hits(1, true);
        let (normal, id) = skull_hits(2, true);
        let (hard, _) = skull_hits(3, true);
        assert_eq!(
            [
                wither_ticks(&easy),
                wither_ticks(&normal),
                wither_ticks(&hard)
            ],
            [Some(0), Some(200), Some(800)]
        );
        assert_eq!((normal[0].damage, normal[0].source), (8.0, Some(id)));
        let (ownerless, _) = skull_hits(2, false);
        assert_eq!(
            (ownerless[0].damage, ownerless[0].source),
            (5.0, None),
            "a skull without its wither hurts as magic for five"
        );
    }

    #[test]
    fn a_skull_withers_a_mob_and_heals_its_wither_when_it_kills() {
        let mut world = EntityWorld::default();
        let id = world.spawn_wither(Wither::new(DVec3::new(0.5, 70.0, -12.5)), true);
        world.wither_mut(id).unwrap().wither.health = 200.0;
        let golem = world.spawn_iron_golem(
            crate::iron_golem::IronGolem::new(DVec3::new(0.5, 64.0, 4.5)),
            true,
        );
        skull(&mut world, Some(id), DVec3::new(0.5, 65.0, 0.5), false);
        for _ in 0..20 {
            world.tick_with_players(&mut Field, &[]);
        }
        let wither = world
            .mob_effects(golem)
            .and_then(|effects| effects.get(crate::effects::MobEffect::Wither))
            .map(|effect| (effect.amplifier, effect.duration));
        assert!(
            matches!(wither, Some((1, 180..=200))),
            "Wither II for ten seconds: {wither:?}"
        );
        assert_eq!(world.withers()[0].wither.health, 200.0);
        let chicken = world.spawn_chicken(
            crate::chicken::Chicken::new(DVec3::new(10.5, 64.0, 4.5)),
            true,
        );
        skull(&mut world, Some(id), DVec3::new(10.5, 64.2, 0.5), false);
        for _ in 0..20 {
            world.tick_with_players(&mut Field, &[]);
        }
        assert!(
            world
                .chickens()
                .iter()
                .all(|e| e.id != chicken || e.chicken.health <= 0.0),
            "the skull kills the chicken"
        );
        assert_eq!(
            world.withers()[0].wither.health,
            205.0,
            "a kill heals the wither five"
        );
    }

    #[test]
    fn a_dangerous_skull_blast_is_marked_for_the_level_and_a_plain_one_is_not() {
        for dangerous in [false, true] {
            let mut world = EntityWorld::default();
            let id = world.spawn_wither(Wither::new(DVec3::new(0.5, 70.0, -12.5)), true);
            skull(&mut world, Some(id), DVec3::new(0.5, 63.5, 0.5), dangerous);
            for _ in 0..5 {
                world.tick_with_players(&mut Field, &[]);
            }
            let blasts = world.take_fireball_blasts();
            assert_eq!(blasts.len(), 1);
            assert_eq!(
                (blasts[0].power, blasts[0].fire, blasts[0].dangerous),
                (1.0, false, dangerous)
            );
        }
    }

    #[test]
    fn arrows_tridents_and_wind_charges_bounce_off_its_armour_below_half_health() {
        let mut world = EntityWorld::default();
        let id = wither_at(&mut world, Wither::new(DVec3::new(0.5, 70.0, 0.5)));
        let e = world.wither_mut(id).unwrap();
        for kind in wt::ARMOR_DEFLECTS {
            assert!(
                e.hurt_typed(1.0, kind, false, false).applied,
                "{kind} at full health"
            );
            e.wither.damage = Default::default();
        }
        e.wither.health = 150.0;
        for kind in wt::ARMOR_DEFLECTS {
            assert!(
                !e.hurt_typed(1.0, kind, false, false).applied,
                "{kind} at half health"
            );
        }
        assert!(
            !e.hurt_typed(1.0, "minecraft:player_explosion", true, false)
                .applied,
            "the undead's blasts do not hurt it"
        );
        assert!(
            e.hurt_typed(1.0, "minecraft:player_attack", false, false)
                .applied
        );
    }

    #[test]
    fn its_own_skull_blast_does_not_hurt_it() {
        let mut world = EntityWorld::default();
        let id = world.spawn_wither(Wither::new(DVec3::new(0.5, 64.0, 2.5)), true);
        skull(&mut world, Some(id), DVec3::new(0.5, 63.5, 0.5), false);
        for _ in 0..5 {
            world.tick_with_players(&mut Field, &[]);
        }
        assert_eq!(world.take_fireball_blasts().len(), 1);
        assert_eq!(world.withers()[0].wither.health, wt::MAX_HEALTH);
    }

    #[test]
    fn a_hurt_wither_breaks_the_blocks_around_it_a_second_later_if_mobs_may_grief() {
        for griefing in [true, false] {
            let mut world = EntityWorld::default();
            world.set_mob_griefing(griefing);
            let id = wither_at(&mut world, Wither::new(DVec3::new(0.5, 64.0, 0.5)));
            world.tick_with_players(&mut Field, &[]);
            assert!(world.wither_mut(id).unwrap().hurt(1.0).applied);
            let mut when = Vec::new();
            for tick in 1..=40 {
                world.tick_with_players(&mut Field, &[]);
                for broken in world.take_wither_breaks() {
                    when.push((tick, broken));
                }
            }
            if !griefing {
                assert!(when.is_empty());
                continue;
            }
            assert_eq!(when.len(), 1, "{when:?}");
            let (tick, broken) = when[0];
            assert_eq!((tick, broken.wither), (20, id));
            let (x, y, z) = broken.at;
            assert_eq!(broken.min, (x - 1, y, z - 1));
            assert_eq!(broken.max, (x + 1, y + 3, z + 1));
            let cells = broken.cells();
            assert_eq!(cells.len(), 36);
            assert_eq!(&cells[..2], &[(x - 1, y, z - 1), (x, y, z - 1)]);
            assert_eq!(cells[3], (x - 1, y + 1, z - 1));
        }
    }

    #[test]
    fn a_dead_wither_drops_a_nether_star_when_mobs_drop_loot() {
        for drops in [true, false] {
            let mut world = EntityWorld::default();
            world.set_mob_drops(drops);
            let id = wither_at(&mut world, Wither::new(DVec3::new(0.5, 70.0, 0.5)));
            let attack = PlayerAttack {
                player_id: 1,
                position: DVec3::new(0.5, 70.0, 3.5),
                yaw: 180.0,
                attack_damage: 1000.0,
                strength: 1.0,
                sprinting: false,
                can_critical: false,
                can_sweep: false,
            };
            assert!(world.player_attack(&attack, id).died);
            let deaths = world.take_deaths();
            let stars: Vec<&str> = deaths
                .iter()
                .flat_map(|death| death.equipment.iter().map(|s| s.id.as_str()))
                .collect();
            let expected: Vec<&str> = if drops {
                vec![wt::NETHER_STAR]
            } else {
                Vec::new()
            };
            assert_eq!(stars, expected);
        }
    }
}
