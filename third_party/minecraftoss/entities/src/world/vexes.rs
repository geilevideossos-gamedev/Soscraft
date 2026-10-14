use super::*;
use crate::monster_ai::{MobCandidate, MonsterKind, Target};
use crate::vex::{self as vx, Vex};

#[derive(Clone)]
pub struct VexEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub vex: Vex,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
}

fn adult_pitch(random: &mut LegacyRandom) -> f32 {
    let (a, b) = (random.next_float(), random.next_float());
    (a - b) * 0.2 + 1.0
}

impl VexEntity {
    pub fn position(&self) -> DVec3 {
        self.vex.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.vex.health > 0.0 && !self.vex.damage.dead {
            self.no_action_time = 0;
        }
        let result = self
            .vex
            .damage
            .hurt_generic(&mut self.vex.health, vx::MAX_HEALTH, amount);
        let position = self.position();
        self.vex.damage.place_death(result, position, false);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -vx::AMBIENT_INTERVAL;
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
        villagers: &[MobCandidate],
        game_time: i64,
        difficulty: i32,
    ) -> Option<Target> {
        let position = self.vex.body.position;
        if players
            .iter()
            .any(|p| p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0)
        {
            self.no_action_time = 0;
        }
        self.no_action_time += 1;
        let health = self.vex.health;
        let size = self.vex.size();
        let ai = &mut self.ai;
        let body = &mut self.vex.body;
        body.trim_small_velocity();
        ai.state.body = body.clone();
        ai.state.health = health;
        ai.state.max_health = vx::MAX_HEALTH;
        ai.state.game_time = game_time;
        ai.state.difficulty = difficulty;
        ai.state.players = players.to_vec();
        ai.state.villagers = villagers.to_vec();
        ai.state.no_action_time = self.no_action_time;
        ai.state.tick_count = self.tick_count;
        ai.state.vex.bound_origin = self.vex.bound_origin;
        ai.state.vex.size = size;
        ai.state.random = std::mem::take(&mut self.random);
        let full = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
        ai.tick_goals(world, full);
        let strike = ai.state.attack.take();
        if std::mem::take(&mut ai.state.vex.charged) {
            self.voices
                .push((Voice::Event(vx::CHARGE_SOUND, 1.0, 1.0), body.position));
        }
        self.random = std::mem::take(&mut ai.state.random);
        if let Some((wanted, speed)) = ai.state.vex.wanted {
            match vx::steer(body.velocity, body.position, wanted, speed, size) {
                Some(velocity) => {
                    body.velocity = velocity;
                    let facing = match ai.state.target().map(|t| t.position) {
                        Some(target) => target - body.position,
                        None => velocity,
                    };
                    ai.yaw = vx::yaw_towards(facing);
                    ai.body_rotation.body_yaw = ai.yaw;
                }
                None => {
                    ai.state.vex.wanted = None;
                    body.velocity *= vx::WAIT_BRAKE;
                }
            }
        }
        ai.state.look_control.tick(
            body.position,
            vx::EYE_HEIGHT,
            ai.body_rotation.body_yaw,
            ai.state.vex.wanted.is_some(),
        );
        body.position += body.velocity;
        body.velocity.x *= vx::HORIZONTAL_DRAG;
        body.velocity.y *= vx::VERTICAL_DRAG;
        body.velocity.z *= vx::HORIZONTAL_DRAG;
        body.on_ground = false;
        body.fall_distance = 0.0;
        ai.body_rotation.tick(
            ai.yaw,
            &mut ai.state.look_control,
            self.previous_position,
            body.position,
        );
        self.vex.yaw = ai.yaw;
        self.vex.charging = ai.state.vex.charging;
        strike
    }
}

impl EntityWorld {
    pub fn spawn_vex(&mut self, vex: Vex, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut ai = MonsterAi::of_kind(MonsterKind::Vex, &vex.body, vex.yaw);
        ai.state.max_health = vx::MAX_HEALTH;
        ai.state.illager.id = id;
        self.vexes.push(VexEntity {
            effects: Default::default(),
            id,
            previous_position: vex.body.position,
            vex,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
        });
        self.order.push(EntityKey::Vex(id));
        self.file_in_section(EntityKey::Vex(id));
        id
    }

    pub fn vexes(&self) -> &[VexEntity] {
        &self.vexes
    }

    pub fn vex_mut(&mut self, id: u64) -> Option<&mut VexEntity> {
        self.vexes.iter_mut().find(|entity| entity.id == id)
    }

    pub(super) fn vex_owner(&self, id: u64) -> Option<u64> {
        let entity = self.vexes.iter().find(|e| e.id == id)?;
        entity
            .vex
            .owner
            .or_else(|| entity.vex.owner_uuid.and_then(|uuid| self.id_of_uuid(uuid)))
            .filter(|&owner| self.entity_type(owner).is_some())
    }

    pub(super) fn vexes_near(&self, id: u64) -> usize {
        let Some(me) = self.illagers.iter().find(|e| e.id == id) else {
            return 0;
        };
        let body = &me.illager.body;
        let (p, half, height) = (
            body.position,
            f64::from(body.width / 2.0),
            f64::from(body.height),
        );
        let range = crate::illager::SPELL_RANGE;
        self.vexes
            .iter()
            .filter(|e| e.vex.health > 0.0)
            .filter(|e| {
                let (low, high) = e.vex.bounds();
                low.x < p.x + half + range
                    && high.x > p.x - half - range
                    && low.y < p.y + height + range
                    && high.y > p.y - range
                    && low.z < p.z + half + range
                    && high.z > p.z - half - range
                    && e.vex.body.position.distance_squared(p) <= range * range
            })
            .count()
    }

    pub(super) fn summon_vexes(&mut self, evoker: u64) -> Vec<u64> {
        let Some(entity) = self.illagers.iter_mut().find(|e| e.id == evoker) else {
            return Vec::new();
        };
        let p = entity.illager.body.position;
        let block = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let mut spots = Vec::new();
        for _ in 0..vx::SUMMONED {
            let dx = entity.random.next_int(vx::SUMMON_SPREAD) as i32 - 2;
            let dz = entity.random.next_int(vx::SUMMON_SPREAD) as i32 - 2;
            let life = entity.random.next_int(vx::LIFE_SPREAD_SECONDS) as i32;
            spots.push(((block.0 + dx, block.1 + 1, block.2 + dz), life));
        }
        let mut made = Vec::new();
        for (spot, life) in spots {
            let at = DVec3::new(
                f64::from(spot.0) + 0.5,
                f64::from(spot.1),
                f64::from(spot.2) + 0.5,
            );
            let mut vex = Vex::summoned(at, evoker, spot, vx::summoned_life(life));
            let _ = self.level_random.next_float();
            vex.left_handed = self.level_random.next_float() < 0.05;
            let id = self.spawn_vex(vex, false);
            let seed = self.level_random.next_long() as u64;
            if let Some(entity) = self.vex_mut(id) {
                entity.set_random_seed(seed);
            }
            made.push(id);
        }
        made
    }

    pub(super) fn vex_villagers(&self) -> Vec<MobCandidate> {
        self.illager_villagers()
    }

    fn vex_strikes(&mut self, id: u64, target: Target) {
        let Some(entity) = self.vexes.iter().find(|e| e.id == id) else {
            return;
        };
        let position = entity.position();
        let mut damage = vx::ATTACK_DAMAGE;
        let mut disable_shield_seconds = 0.0;
        if let Some(stack) = entity.vex.main_hand().cloned() {
            damage += self.weapon_damage(&stack);
            disable_shield_seconds = self.disable_shield_seconds(&stack);
        }
        match target {
            Target::Player(player_id) => self.player_hits.push(PlayerHit {
                player_id,
                damage,
                kind: PlayerHitKind::Melee {
                    attacker: position,
                    hunger_ticks: 0,
                    lift: 0.0,
                    throw: DVec3::ZERO,
                    disable_shield_seconds,
                },
                source: Some(id),
            }),
            Target::Mob(victim) | Target::Villager(victim) => {
                self.mob_hits_mob(id, victim, damage, position, 0.0);
            }
        }
        if let Some(entity) = self.vexes.iter_mut().find(|e| e.id == id) {
            entity.ai.state.melee.swings += 1;
        }
    }

    fn vex_alerts(&mut self, id: u64, attacker: Target) {
        let Some(p) = self.vexes.iter().find(|e| e.id == id).map(|e| e.position()) else {
            return;
        };
        let range = vx::FOLLOW_RANGE;
        for other in self.vexes.iter_mut().filter(|e| e.id != id) {
            let q = other.position();
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

    pub(super) fn tick_vex(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
    ) {
        let (game_time, difficulty) = (self.game_time, self.difficulty);
        let mobs = self.mob_candidates();
        let villagers = self.vex_villagers();
        let owner = self.vex_owner(id);
        let owner_target = owner.and_then(|o| {
            self.illagers
                .iter()
                .find(|e| e.id == o)
                .and_then(|e| e.ai.state.target)
        });
        let Some(entity) = self.vexes.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if owner.is_some() {
            entity.vex.owner = owner;
        }
        entity.ai.state.mobs = mobs;
        entity.ai.state.vex.owner_target = owner_target;
        entity.previous_position = entity.vex.body.position;
        entity.tick_count += 1;
        let removed = entity.vex.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        for work in entity.effects.tick(false) {
            match work.resolve(entity.vex.health, vx::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut entity.vex.health, vx::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt(amount);
                }
                _ => {}
            }
        }
        if entity.vex.health > 0.0 {
            if (entity.random.next_int(1000) as i32) < entity.ambient_sound_time {
                entity.ambient_sound_time = -vx::AMBIENT_INTERVAL;
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
        if entity.vex.health > 0.0 && !entity.no_ai {
            strike = entity.tick_ai(&*world, players, &villagers, game_time, difficulty);
        } else if entity.no_ai {
            monster_idle_without_ai(
                &*world,
                players,
                entity.vex.body.position,
                vx::EYE_HEIGHT,
                &mut entity.no_action_time,
            );
        }
        if entity.vex.health > 0.0 && entity.vex.tick_life() {
            entity.hurt(vx::LIFE_HURT);
        }
        let alert = entity.ai.state.alert.take();
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, &*world, old, game_time);
        if let Some(target) = strike {
            self.vex_strikes(id, target);
        }
        if let Some(attacker) = alert {
            self.vex_alerts(id, attacker);
        }
        self.file_in_section(EntityKey::Vex(id));
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

    fn vex_at(world: &mut EntityWorld, at: DVec3) -> u64 {
        let id = world.spawn_vex(Vex::armed(at), false);
        world.vex_mut(id).unwrap().set_random_seed(3);
        id
    }

    #[test]
    fn a_vex_is_a_live_monster_of_its_size_and_health() {
        let mut world = EntityWorld::default();
        let id = vex_at(&mut world, DVec3::new(0.5, 66.0, 0.5));
        assert_eq!(world.entity_type(id), Some(vx::TYPE_ID));
        let e = &world.vexes()[0];
        assert_eq!(
            (e.vex.body.width, e.vex.body.height, e.vex.health),
            (0.4, 0.8, 14.0)
        );
        assert_eq!(e.ai.state.follow_range, 16.0);
        assert_eq!(
            MonsterKind::Vex.goal_names().len(),
            crate::vex_ai::GOAL_NAMES.len()
        );
        assert!(crate::monster_ai::is_enemy(vx::TYPE_ID));
        assert!(world.census().iter().any(|c| c.kind == vx::TYPE_ID));
        assert!(world.mob_candidates().iter().any(|m| m.id == id));
        let attack = PlayerAttack {
            player_id: 1,
            position: DVec3::new(0.5, 64.0, 3.5),
            yaw: 180.0,
            attack_damage: 100.0,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        };
        assert!(world.player_attack(&attack, id).died);
        let deaths = world.take_deaths();
        assert_eq!(deaths[0].table, Some(vx::TYPE_ID));
        assert!(deaths[0].equipment.is_empty(), "its sword never drops");
        assert!(
            (4..=6).contains(&deaths[0].experience),
            "{}",
            deaths[0].experience
        );
    }

    #[test]
    fn a_vex_flies_through_stone_hovers_and_charges_a_player() {
        let mut world = EntityWorld::default();
        let id = vex_at(&mut world, DVec3::new(0.5, 63.5, 0.5));
        let players = [player_at(1, 0.5, 6.5)];
        let mut charged = false;
        let mut hit = None;
        let mut sounds = Vec::new();
        let mut lowest = f64::MAX;
        for _ in 0..400 {
            world.tick_with_players(&mut Field, &players);
            sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
            let e = world.vexes().iter().find(|e| e.id == id).unwrap();
            charged |= e.vex.charging;
            lowest = lowest.min(e.vex.body.position.y);
            if let Some(h) = world.take_player_hits().into_iter().next() {
                hit = Some(h);
                break;
            }
        }
        let hit = hit.expect("it charges the player");
        assert!(charged, "it charges");
        assert!(sounds.iter().any(|s| s == vx::CHARGE_SOUND), "{sounds:?}");
        assert_eq!((hit.player_id, hit.source), (1, Some(id)));
        assert!(hit.damage >= vx::ATTACK_DAMAGE);
        assert!(
            lowest < 64.0,
            "it started in the stone and was not pushed out"
        );
    }

    #[test]
    fn an_evoker_summons_three_vexes_that_take_its_target() {
        let mut world = EntityWorld::default();
        let evoker = world.spawn_illager(
            crate::illager::Illager::evoker(DVec3::new(0.5, 64.0, 0.5)),
            false,
        );
        world.illager_mut(evoker).unwrap().set_random_seed(11);
        let players = [player_at(1, 0.5, 6.5)];
        let mut sounds = Vec::new();
        for _ in 0..200 {
            world.tick_with_players(&mut Field, &players);
            sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
            if !world.vexes().is_empty() {
                break;
            }
        }
        assert_eq!(world.vexes().len(), vx::SUMMONED);
        assert!(
            sounds
                .iter()
                .any(|s| s == crate::illager::PREPARE_SUMMON_SOUND)
        );
        for e in world.vexes() {
            assert_eq!(e.vex.owner, Some(evoker));
            assert!(e.vex.bound_origin.is_some());
            let life = e.vex.life_ticks.unwrap();
            assert!((600..=2380).contains(&life), "{life}");
            assert_eq!(
                e.vex.main_hand().map(|s| s.id.as_str()),
                Some(vx::IRON_SWORD)
            );
            let at = e.vex.body.position - DVec3::new(0.5, 64.0, 0.5);
            assert!(
                at.x.abs() <= 2.2 && at.z.abs() <= 2.2 && (at.y - 1.0).abs() < 0.2,
                "{at}"
            );
        }
        for _ in 0..10 {
            world.tick_with_players(&mut Field, &players);
        }
        assert!(
            world
                .vexes()
                .iter()
                .all(|e| e.ai.state.target == Some(Target::Player(1)))
        );
        assert_eq!(world.vexes_near(evoker), vx::SUMMONED);
    }

    #[test]
    fn a_vex_wears_out_a_heart_a_second_once_its_life_is_over() {
        let mut world = EntityWorld::default();
        let mut vex = Vex::summoned(DVec3::new(0.5, 70.0, 0.5), 999, (0, 70, 0), 5);
        vex.owner = None;
        let id = world.spawn_vex(vex, false);
        let mut healths = Vec::new();
        for tick in 1..=320 {
            world.tick(&mut Field);
            if let Some(e) = world.vexes().iter().find(|e| e.id == id) {
                if tick == 5 || tick == 25 {
                    healths.push(e.vex.health);
                }
            }
        }
        assert_eq!(healths, vec![13.0, 12.0]);
        assert!(world.vexes().is_empty(), "it wore out");
    }

    #[test]
    fn evoker_fangs_spare_the_vexes_of_illagers() {
        let mut world = EntityWorld::default();
        let evoker = world.spawn_illager(
            crate::illager::Illager::evoker(DVec3::new(9.5, 64.0, 9.5)),
            true,
        );
        let owned = world.spawn_vex(
            Vex::summoned(DVec3::new(0.5, 64.0, 0.5), evoker, (0, 64, 0), 2000),
            true,
        );
        let wild = world.spawn_vex(Vex::armed(DVec3::new(0.6, 64.0, 0.5)), true);
        let at = DVec3::new(0.5, 64.0, 0.5);
        world.spawn_evoker_fangs(crate::evoker_fangs::EvokerFangs::new(
            at,
            0.0,
            0,
            Some(evoker),
        ));
        for _ in 0..12 {
            world.tick(&mut Field);
        }
        let health = |id: u64| {
            world
                .vexes()
                .iter()
                .find(|e| e.id == id)
                .unwrap()
                .vex
                .health
        };
        assert_eq!(health(owned), vx::MAX_HEALTH);
        assert!(health(wild) < vx::MAX_HEALTH);
    }
}
