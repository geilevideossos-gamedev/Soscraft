use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::illager::{self as il, Illager, IllagerArmPose};
use crate::illager_ai::CrossbowShot;
use crate::monster_ai::{MobCandidate, MonsterKind, Target};

const SOUNDS: MovementSounds = MovementSounds::monster(None);

#[derive(Clone)]
pub struct IllagerEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub illager: Illager,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
    pub using_item: Option<i32>,
    pub loading_sounds: [bool; 2],
    pub arm_pose: IllagerArmPose,
}

fn adult_pitch(random: &mut LegacyRandom) -> f32 {
    let (a, b) = (random.next_float(), random.next_float());
    (a - b) * 0.2 + 1.0
}

pub fn is_ominous_banner(stack: &ItemStack) -> bool {
    stack.id == "minecraft:white_banner"
        && stack
            .components
            .as_ref()
            .and_then(|c| c.get("minecraft:item_name"))
            .is_some_and(|name| name.to_string().contains("block.minecraft.ominous_banner"))
}

impl IllagerEntity {
    pub fn position(&self) -> DVec3 {
        self.illager.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn is_captain(&self) -> bool {
        self.illager.raider.patrol_leader
            && self.illager.equipment[il::HEAD]
                .as_ref()
                .is_some_and(is_ominous_banner)
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.illager.health > 0.0 && !self.illager.damage.dead {
            self.no_action_time = 0;
        }
        let result =
            self.illager
                .damage
                .hurt_generic(&mut self.illager.health, il::MAX_HEALTH, amount);
        let position = self.position();
        self.illager
            .damage
            .place_death(result, position, self.illager.body.fire_ticks > 0);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -il::AMBIENT_INTERVAL;
            }
            let voice = hurt_voice(&mut self.random, result.died, false);
            self.voices.push((voice, position));
        }
        result
    }

    fn crossbow_use_tick(&mut self, level_random: &mut LegacyRandom) {
        if !self.ai.state.bow.using_item {
            return;
        }
        let ticks = self.ai.state.bow.ticks_using_item;
        let progress = ticks as f32 / il::CROSSBOW_CHARGE as f32;
        let position = self.position();
        if progress < 0.2 {
            self.loading_sounds = [false; 2];
        }
        for (index, (at, event)) in [(0.2, il::LOADING_START), (0.5, il::LOADING_MIDDLE)]
            .into_iter()
            .enumerate()
        {
            if progress >= at && !self.loading_sounds[index] {
                self.loading_sounds[index] = true;
                self.voices.push((Voice::Event(event, 0.5, 1.0), position));
            }
        }
        if progress >= 1.0 && !self.illager.crossbow_charged() {
            self.illager.set_crossbow_charged(true);
            let pitch = 1.0 / (level_random.next_float() * 0.5 + 1.0) + 0.2;
            self.voices
                .push((Voice::Event(il::LOADING_END, 1.0, pitch), position));
        }
    }

    fn sync_ai(&mut self) {
        let state = &mut self.ai.state.illager;
        state.id = self.id;
        state.holds_crossbow = self.illager.holds_crossbow();
        state.patrolling = self.illager.raider.patrolling;
        state.has_raid = false;
        state.aggressive = self.illager.aggressive;
        state.charging = self.illager.charging_crossbow;
        if self.ai.state.bow.using_item && !state.holds_crossbow {
            self.ai.state.bow.using_item = false;
            self.ai.state.bow.ticks_using_item = 0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn tick_ai(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        villagers: &[MobCandidate],
        game_time: i64,
        difficulty: i32,
        level_random: &mut LegacyRandom,
    ) -> Option<CrossbowShot> {
        let position = self.illager.body.position;
        if players
            .iter()
            .any(|p| p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0)
        {
            self.no_action_time = 0;
        }
        self.sync_ai();
        self.crossbow_use_tick(level_random);
        let health = self.illager.health;
        let speed = self
            .effects
            .movement_speed(self.illager.kind.movement_speed());
        let ai = &mut self.ai;
        let body = &mut self.illager.body;
        self.no_action_time += 2;
        if ai.no_jump_delay > 0 {
            ai.no_jump_delay -= 1;
        }
        body.trim_small_velocity();
        self.no_action_time += 1;
        let fluid = FluidFrame::sample(world, position, body.width, body.height);
        ai.state.body = body.clone();
        ai.state.health = health;
        ai.state.max_health = il::MAX_HEALTH;
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
        let shot = ai.state.illager.shot.take();
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
            il::EYE_HEIGHT,
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
        self.illager.yaw = ai.yaw;
        if std::mem::take(&mut self.ai.state.illager.released) {
            self.crossbow_use_tick(level_random);
            self.ai.state.bow.using_item = false;
            self.ai.state.bow.ticks_using_item = 0;
        }
        if std::mem::take(&mut self.ai.state.illager.ambient) {
            let pitch = adult_pitch(&mut self.random);
            self.voices
                .push((Voice::Ambient(pitch), self.illager.body.position));
        }
        self.illager.aggressive = self.ai.state.illager.aggressive;
        self.illager.charging_crossbow = self.ai.state.illager.charging;
        if let Some(damage) = landed.and_then(|fallen| {
            super::living::fall_damage(&self.illager.body, world, fallen, true, &mut self.voices)
        }) {
            self.hurt(damage);
        }
        shot
    }
}

impl EntityWorld {
    pub fn spawn_illager(&mut self, illager: Illager, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let kind = match illager.kind {
            il::IllagerKind::Pillager => MonsterKind::Pillager,
            il::IllagerKind::Vindicator | il::IllagerKind::Evoker => MonsterKind::Pillager,
        };
        let mut ai = MonsterAi::of_kind(kind, &illager.body, illager.yaw);
        ai.state.max_health = il::MAX_HEALTH;
        ai.state.illager.id = id;
        let arm_pose = illager.arm_pose();
        self.illagers.push(IllagerEntity {
            effects: Default::default(),
            id,
            previous_position: illager.body.position,
            illager,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
            using_item: None,
            loading_sounds: [false; 2],
            arm_pose,
        });
        self.order.push(EntityKey::Illager(id));
        self.file_in_section(EntityKey::Illager(id));
        id
    }

    pub fn illagers(&self) -> &[IllagerEntity] {
        &self.illagers
    }

    pub fn illager_mut(&mut self, id: u64) -> Option<&mut IllagerEntity> {
        self.illagers.iter_mut().find(|entity| entity.id == id)
    }

    pub(super) fn illager_villagers(&self) -> Vec<MobCandidate> {
        let villagers = self.villagers.iter().map(|e| MobCandidate {
            id: e.id,
            position: e.villager.body.position,
            eye_height: e.villager.eye_height(),
            width: e.villager.body.width,
            height: e.villager.body.height,
            alive: e.villager.health > 0.0,
            kind: "minecraft:villager",
            baby: e.villager.age.baby(),
        });
        let traders = self.wandering_traders.iter().map(|e| MobCandidate {
            id: e.id,
            position: e.trader.body.position,
            eye_height: e.trader.eye_height(),
            width: e.trader.body.width,
            height: e.trader.body.height,
            alive: e.trader.health > 0.0,
            kind: crate::wandering_trader::TYPE_ID,
            baby: false,
        });
        villagers.chain(traders).collect()
    }

    fn illager_shoots(&mut self, id: u64, shot: CrossbowShot) {
        let uncertainty = (14 - self.difficulty * 4) as f32;
        let seed = self
            .arrow_shoot_seed
            .unwrap_or_else(|| self.projectile_seed_random.next_long());
        let Some(entity) = self.illagers.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.no_action_time = 0;
        if !entity.illager.crossbow_charged() {
            return;
        }
        let position = entity.position();
        let arrow = crate::projectile::Arrow::skeleton_shot(
            position,
            il::EYE_HEIGHT,
            shot.position,
            shot.height,
            uncertainty,
            &mut LegacyRandom::new(seed),
        );
        entity.illager.set_crossbow_charged(false);
        entity
            .voices
            .push((Voice::Event(il::SHOOT_SOUND, 1.0, 1.0), position));
        let arrow_id = self.spawn_arrow(id, arrow);
        if let Some(arrow) = self.arrows.iter_mut().find(|e| e.id == arrow_id) {
            arrow.hit_sound = il::HIT_SOUND;
        }
    }

    fn illager_shouts(&mut self, id: u64, target: Target, aggressive: bool) {
        let Some(from) = self.illagers.iter().find(|e| e.id == id).map(|e| {
            let p = e.illager.body.position;
            let half = f64::from(e.illager.body.width / 2.0);
            let height = f64::from(e.illager.body.height);
            (p, half, height)
        }) else {
            return;
        };
        let (p, half, height) = from;
        let range = il::SHOUT_RANGE;
        for other in self.illagers.iter_mut().filter(|e| e.id != id) {
            let q = other.illager.body.position;
            let other_half = f64::from(other.illager.body.width / 2.0);
            let inside = q.x - other_half < p.x + half + range
                && q.x + other_half > p.x - half - range
                && q.y < p.y + height + range
                && q.y + f64::from(other.illager.body.height) > p.y - range
                && q.z - other_half < p.z + half + range
                && q.z + other_half > p.z - half - range;
            if !inside
                || other.illager.health <= 0.0
                || p.distance_squared(q) > range.max(2.0) * range.max(2.0)
            {
                continue;
            }
            other.ai.state.set_target(Some(target));
            if aggressive {
                other.illager.aggressive = true;
                other.ai.state.illager.aggressive = true;
            }
        }
    }

    fn illager_alerts(&mut self, id: u64, attacker: Target) {
        let Some((p, kind, range)) = self.illagers.iter().find(|e| e.id == id).map(|e| {
            (
                e.illager.body.position,
                e.illager.kind,
                e.ai.state.follow_range,
            )
        }) else {
            return;
        };
        for other in self.illagers.iter_mut().filter(|e| e.id != id) {
            let q = other.illager.body.position;
            if other.illager.kind != kind
                || other.ai.state.target.is_some()
                || (q.x - p.x).abs() > range + 0.5
                || (q.z - p.z).abs() > range + 0.5
                || (q.y - p.y).abs() > 10.5
            {
                continue;
            }
            other.ai.state.set_target(Some(attacker));
        }
    }

    pub(super) fn tick_illager(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty) = (self.game_time, self.difficulty);
        let mobs = self.mob_candidates();
        let villagers = self.illager_villagers();
        let mut level_random = std::mem::take(&mut self.level_random);
        let Some(entity) = self.illagers.iter_mut().find(|e| e.id == id) else {
            self.level_random = level_random;
            return;
        };
        entity.ai.state.mobs = mobs;
        entity.previous_position = entity.illager.body.position;
        entity.tick_count += 1;
        base_tick_fluid(
            &mut entity.illager.body,
            &*world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            SOUNDS,
        );
        super::hazards::burn(entity, &*world, game_time);
        super::hazards::suffocate(entity, &*world, il::EYE_HEIGHT, game_time);
        let water_breathing = entity
            .effects
            .has(crate::effects::MobEffect::WaterBreathing);
        if entity.illager.health > 0.0
            && super::living::breathe(
                &mut entity.illager.body,
                &*world,
                il::EYE_HEIGHT,
                water_breathing,
            )
        {
            entity.hurt(2.0);
        }
        let removed = entity.illager.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        for work in entity.effects.tick(false) {
            match work.resolve(entity.illager.health, il::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut entity.illager.health, il::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt(amount);
                }
                _ => {}
            }
        }
        entity.illager.body.float_from(&entity.effects);
        if entity.illager.health > 0.0 {
            if (entity.random.next_int(1000) as i32) < entity.ambient_sound_time {
                entity.ambient_sound_time = -il::AMBIENT_INTERVAL;
                let pitch = adult_pitch(&mut entity.random);
                entity
                    .voices
                    .push((Voice::Ambient(pitch), entity.position()));
            } else {
                entity.ambient_sound_time += 1;
            }
        }
        if removed {
            self.level_random = level_random;
            return;
        }
        let mut shot = None;
        if entity.illager.health <= 0.0 && !entity.no_ai {
            let yaw = entity.illager.yaw;
            let _ = super::living::dying_travel(
                &mut entity.illager.body,
                &*world,
                yaw,
                entity.tick_count,
                &mut entity.random,
                &mut entity.voices,
                SOUNDS,
                Some(true),
            );
        } else if !entity.no_ai {
            shot = entity.tick_ai(
                &*world,
                players,
                &villagers,
                game_time,
                difficulty,
                &mut level_random,
            );
        } else {
            entity.illager.body.trim_small_velocity();
            monster_idle_without_ai(
                &*world,
                players,
                entity.illager.body.position,
                il::EYE_HEIGHT,
                &mut entity.no_action_time,
            );
        }
        entity.using_item = entity
            .ai
            .state
            .bow
            .using_item
            .then_some(entity.ai.state.bow.ticks_using_item);
        entity.arm_pose = entity.illager.arm_pose();
        let shout = entity.ai.state.illager.shout.take();
        let alert = entity.ai.state.alert.take();
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, &*world, old, game_time);
        self.level_random = level_random;
        if let Some(shot) = shot {
            self.illager_shoots(id, shot);
        }
        if let Some((target, aggressive)) = shout {
            self.illager_shouts(id, target, aggressive);
        }
        if let Some(attacker) = alert {
            self.illager_alerts(id, attacker);
        }
        self.push_entities(EntityKey::Illager(id), &*world, game_time, ticks);
        self.file_in_section(EntityKey::Illager(id));
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

    fn kill(world: &mut EntityWorld, id: u64) -> bool {
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
        world.player_attack(&attack, id).died
    }

    fn pillager_at(world: &mut EntityWorld, at: DVec3, no_ai: bool) -> u64 {
        let id = world.spawn_illager(Illager::pillager(at), no_ai);
        world.illager_mut(id).unwrap().set_random_seed(7);
        id
    }

    #[test]
    fn a_pillager_is_a_live_monster_of_its_size_and_health() {
        let mut world = EntityWorld::default();
        let id = pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        assert_eq!(world.entity_type(id), Some(il::PILLAGER));
        let entity = &world.illagers()[0];
        assert_eq!(
            (
                entity.illager.body.width,
                entity.illager.body.height,
                entity.illager.health
            ),
            (0.6, 1.95, 24.0)
        );
        assert!(world.census().iter().any(|e| e.kind == il::PILLAGER));
        assert!(world.mob_candidates().iter().any(|m| m.id == id));
        assert!(!world.can_be_leashed(id), "monsters take no lead");
        let hit = world.illager_mut(id).unwrap().hurt(4.0);
        assert!(hit.applied);
        assert_eq!(world.illagers()[0].illager.health, 20.0);
        assert!(crate::monster_ai::is_enemy(il::PILLAGER));
    }

    #[test]
    fn a_pillager_charges_its_crossbow_for_25_ticks_then_shoots_20_to_39_ticks_later() {
        let mut gaps = Vec::new();
        for seed in 0..6 {
            let mut world = EntityWorld::default();
            let id = pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
            world.illager_mut(id).unwrap().set_random_seed(seed);
            let players = [player_at(1, 0.5, 6.5)];
            let (mut started, mut charged, mut fired) = (None, None, None);
            let mut sounds = Vec::new();
            for tick in 0..200 {
                world.tick_with_players(&mut Field, &players);
                sounds.extend(world.take_sounds().into_iter().map(|s| s.event));
                let entity = &world.illagers()[0];
                if started.is_none() && entity.using_item == Some(0) {
                    started = Some(tick);
                    assert_eq!(entity.arm_pose, IllagerArmPose::CrossbowCharge);
                }
                if charged.is_none() && entity.illager.crossbow_charged() {
                    charged = Some(tick);
                    assert_eq!(entity.arm_pose, IllagerArmPose::CrossbowHold);
                    assert_eq!(entity.using_item, None);
                }
                if fired.is_none() && !world.arrows().is_empty() {
                    fired = Some(tick);
                    assert!(!world.illagers()[0].illager.crossbow_charged());
                    break;
                }
            }
            let (started, charged, fired) = (started.unwrap(), charged.unwrap(), fired.unwrap());
            assert_eq!(charged - started, 25, "seed {seed}");
            assert!(
                (21..=40).contains(&(fired - charged)),
                "{}",
                fired - charged
            );
            gaps.push(fired - charged);
            let arrow = &world.arrows()[0];
            assert_eq!(arrow.owner_id, id);
            assert_eq!(arrow.hit_sound, il::HIT_SOUND);
            assert!(arrow.arrow.velocity.z > 1.0, "{:?}", arrow.arrow.velocity);
            for event in [
                il::LOADING_START,
                il::LOADING_MIDDLE,
                il::LOADING_END,
                il::SHOOT_SOUND,
            ] {
                assert!(sounds.iter().any(|s| s == event), "{event}: {sounds:?}");
            }
        }
        gaps.sort();
        assert!(gaps.first() != gaps.last(), "{gaps:?}");
    }

    #[test]
    fn a_pillager_walks_closer_when_its_target_is_beyond_eight_blocks() {
        let mut world = EntityWorld::default();
        pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let players = [player_at(1, 0.5, 20.5)];
        for _ in 0..60 {
            world.tick_with_players(&mut Field, &players);
        }
        let entity = &world.illagers()[0];
        assert_eq!(entity.ai.state.target, Some(Target::Player(1)));
        assert!(
            entity.illager.body.position.z > 3.0,
            "{:?}",
            entity.illager.body.position
        );
        assert!(world.arrows().is_empty(), "too far to shoot yet");
    }

    #[test]
    fn a_pillager_targets_villagers_and_iron_golems_but_not_baby_villagers() {
        let mut world = EntityWorld::default();
        pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let mut baby = crate::villager::Villager::new(DVec3::new(0.5, 64.0, 5.5));
        baby.age.set(-24000);
        world.spawn_villager(baby, true);
        for _ in 0..40 {
            world.tick(&mut Field);
        }
        assert_eq!(
            world.illagers()[0].ai.state.target,
            None,
            "babies are spared"
        );
        let adult = world.spawn_villager(
            crate::villager::Villager::new(DVec3::new(4.5, 64.0, 5.5)),
            true,
        );
        for _ in 0..40 {
            world.tick(&mut Field);
        }
        assert_eq!(
            world.illagers()[0].ai.state.target,
            Some(Target::Villager(adult))
        );
        let mut world = EntityWorld::default();
        pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let golem = world.spawn_iron_golem(
            crate::iron_golem::IronGolem::new(DVec3::new(0.5, 64.0, 6.5)),
            true,
        );
        for _ in 0..40 {
            world.tick(&mut Field);
        }
        assert_eq!(
            world.illagers()[0].ai.state.target,
            Some(Target::Mob(golem))
        );
    }

    #[test]
    fn a_patrolling_pillager_holds_its_ground_and_calls_the_others() {
        let mut world = EntityWorld::default();
        let first = pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let second = pillager_at(&mut world, DVec3::new(3.5, 64.0, 0.5), false);
        for id in [first, second] {
            world.illager_mut(id).unwrap().illager.raider.patrolling = true;
        }
        let players = [player_at(1, 0.5, 14.5)];
        let mut called = false;
        for _ in 0..80 {
            world.tick_with_players(&mut Field, &players);
            let target = world
                .illagers()
                .iter()
                .find(|e| e.id == second)
                .unwrap()
                .ai
                .state
                .target;
            called |= target == Some(Target::Player(1));
        }
        assert!(called, "the second pillager hears the call");
        let first_z = world
            .illagers()
            .iter()
            .find(|e| e.id == first)
            .unwrap()
            .illager
            .body
            .position
            .z;
        assert!(
            first_z < 2.0,
            "it holds its ground beyond ten blocks: {first_z}"
        );
    }

    #[test]
    fn iron_golems_hunt_pillagers_and_wandering_traders_flee_them() {
        let mut world = EntityWorld::default();
        let pillager = pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let golem = world.spawn_iron_golem(
            crate::iron_golem::IronGolem::new(DVec3::new(0.5, 64.0, 8.5)),
            false,
        );
        for _ in 0..40 {
            world.tick(&mut Field);
        }
        let hunted = world.iron_golems().iter().find(|e| e.id == golem).unwrap();
        assert_eq!(hunted.ai.state.target, Some(Target::Mob(pillager)));
        let mut world = EntityWorld::default();
        pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let trader = world.spawn_wandering_trader(
            crate::wandering_trader::WanderingTrader::new(DVec3::new(0.5, 64.0, 6.5)),
            false,
        );
        world.wandering_trader_mut(trader).unwrap().random = LegacyRandom::new(3);
        for _ in 0..80 {
            world.tick(&mut Field);
        }
        let fled = world
            .wandering_traders()
            .iter()
            .find(|e| e.id == trader)
            .unwrap();
        assert!(
            fled.trader
                .body
                .position
                .distance(DVec3::new(0.5, 64.0, 0.5))
                > 8.0,
            "{:?}",
            fled.trader.body.position
        );
    }

    #[test]
    fn a_villager_senses_a_pillager_within_fifteen_blocks_as_a_hostile() {
        let mut world = EntityWorld::default();
        let pillager = pillager_at(&mut world, DVec3::new(0.5, 64.0, 12.5), true);
        let villager = world.spawn_villager_active(
            crate::villager::Villager::new(DVec3::new(0.5, 64.0, 0.5)),
            0.0,
        );
        let mut seen = None;
        for _ in 0..40 {
            world.tick(&mut Field);
            let entity = world.villagers().iter().find(|e| e.id == villager).unwrap();
            if let Some(ai) = entity.ai.as_deref() {
                seen = seen.or(ai.brain.memories.nearest_hostile.get().copied());
            }
        }
        assert_eq!(seen, Some(pillager));
    }

    #[test]
    fn a_pillager_shot_by_another_raider_does_not_turn_on_it() {
        let mut world = EntityWorld::default();
        let first = pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), false);
        let second = pillager_at(&mut world, DVec3::new(3.5, 64.0, 0.5), false);
        let hit = world.mob_hits_mob(second, first, 1.0, DVec3::new(3.5, 64.0, 0.5), 0.0);
        assert!(hit.is_some_and(|h| h.applied));
        for _ in 0..20 {
            world.tick(&mut Field);
        }
        let target = world
            .illagers()
            .iter()
            .find(|e| e.id == first)
            .unwrap()
            .ai
            .state
            .target;
        assert_eq!(target, None);
    }

    #[test]
    fn a_dead_pillager_leaves_five_experience_and_sometimes_its_crossbow() {
        let mut dropped = 0;
        for seed in 0..200 {
            let mut world = EntityWorld::default();
            let id = pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
            world.illager_mut(id).unwrap().set_random_seed(seed);
            assert!(kill(&mut world, id));
            let deaths = world.take_deaths();
            let death = &deaths[0];
            assert_eq!(death.table, Some(il::PILLAGER));
            assert!(!death.context.captain);
            if death.equipment.is_empty() {
                assert!((6..=8).contains(&death.experience), "{}", death.experience);
            } else {
                dropped += 1;
                assert_eq!(death.equipment[0].id, il::CROSSBOW);
                assert_eq!(death.experience, 5);
            }
        }
        assert!((5..=40).contains(&dropped), "{dropped}");
    }

    #[test]
    fn a_patrol_leader_wearing_the_ominous_banner_dies_as_a_captain() {
        let mut world = EntityWorld::default();
        let id = pillager_at(&mut world, DVec3::new(0.5, 64.0, 0.5), true);
        let mut banner = ItemStack::new("minecraft:white_banner", 1);
        banner.components = Some(serde_json::json!({
            "minecraft:item_name": {"translate": "block.minecraft.ominous_banner"}
        }));
        let entity = world.illager_mut(id).unwrap();
        entity.illager.equipment[il::HEAD] = Some(banner);
        assert!(!entity.is_captain(), "a banner alone makes no captain");
        entity.illager.raider.patrol_leader = true;
        entity.illager.drop_chances[il::HEAD] = 2.0;
        assert!(entity.is_captain());
        assert!(kill(&mut world, id));
        let death = &world.take_deaths()[0];
        assert!(death.context.captain);
        assert!(
            death
                .equipment
                .iter()
                .any(|stack| stack.id == "minecraft:white_banner"),
            "the banner's chance of 2 always drops it"
        );
    }
}
