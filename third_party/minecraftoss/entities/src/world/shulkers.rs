use super::emissions::{MovementSounds, base_tick_fluid};
use super::*;
use crate::monster_ai::MonsterKind;
use crate::shulker::{self, Aabb, Shulker};
use minecraftoss_player::Pos;
use minecraftoss_player::collision::{Bounds, move_body};

const SHULKER_SOUNDS: MovementSounds = MovementSounds::monster(None);

#[derive(Clone)]
pub struct ShulkerEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub shulker: Shulker,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub teleport_pending: bool,
    pub ai: Box<MonsterAi>,
}

impl ShulkerEntity {
    pub fn position(&self) -> DVec3 {
        self.shulker.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn head_yaw(&self) -> f32 {
        self.ai.state.look_control.head_yaw
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.shulker.health > 0.0 && !self.shulker.damage.dead {
            self.no_action_time = 0;
        }
        let closed = self.shulker.closed();
        let result =
            self.shulker
                .damage
                .hurt_generic(&mut self.shulker.health, shulker::MAX_HEALTH, amount);
        let position = self.position();
        self.shulker
            .damage
            .place_death(result, position, self.shulker.body.fire_ticks > 0);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -shulker::AMBIENT_SOUND_INTERVAL;
            }
            let voice = match hurt_voice(&mut self.random, result.died, false) {
                Voice::Hurt(pitch) if closed => {
                    Voice::Event("entity.shulker.hurt_closed", 1.0, pitch)
                }
                voice => voice,
            };
            self.voices.push((voice, position));
        }
        if result.applied
            && self.shulker.health < shulker::MAX_HEALTH * 0.5
            && self.random.next_int(4) == 0
        {
            self.teleport_pending = true;
        }
        result
    }

    pub fn hurt_armored(&mut self, amount: f32) -> DamageResult {
        let armor = if self.shulker.covered {
            shulker::COVERED_ARMOR
        } else {
            0.0
        };
        self.hurt(damage_after_armor(amount, armor, 0.0))
    }

    pub fn refuses_arrows(&self) -> bool {
        self.shulker.closed()
    }

    fn set_raw_peek(&mut self, amount: i32) {
        let sound = self.shulker.set_raw_peek(amount);
        let position = self.position();
        self.voices.push((Voice::Event(sound, 1.0, 1.0), position));
    }

    fn teleport(&mut self, world: &impl World, others: &[Aabb]) -> Option<(Pos, Pos)> {
        if self.no_ai || self.shulker.health <= 0.0 {
            return None;
        }
        let from = self.shulker.block();
        let (to, face) = shulker::teleport_somewhere(world, &mut self.random, from, others)?;
        self.voices.push((
            Voice::Event("entity.shulker.teleport", 1.0, 1.0),
            self.position(),
        ));
        let left = self.position();
        self.shulker.body.game_events.push(("minecraft:teleport", left, None));
        self.shulker.move_to(to, face);
        self.shulker.raw_peek = 0;
        self.ai.state.clear_target();
        Some((from, to))
    }

    fn tick_ai(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        game_time: i64,
        difficulty: i32,
        can_stay: bool,
    ) -> Vec<crate::monster_ai::Target> {
        let position = self.shulker.body.position;
        let eye_height = self.shulker.eye_height();
        self.no_action_time += 1;
        let ai = &mut self.ai;
        ai.state.body = self.shulker.body.clone();
        ai.state.eye_height = eye_height;
        ai.state.health = self.shulker.health;
        ai.state.max_health = shulker::MAX_HEALTH;
        ai.state.game_time = game_time;
        ai.state.difficulty = difficulty;
        ai.state.players = players.to_vec();
        ai.state.villagers = Vec::new();
        ai.state.no_action_time = self.no_action_time;
        ai.state.tick_count = self.tick_count;
        ai.state.shulker.can_stay = can_stay;
        ai.state.random = std::mem::take(&mut self.random);
        let full = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
        ai.tick_goals(world, full);
        self.random = std::mem::take(&mut ai.state.random);
        let peeks = std::mem::take(&mut ai.state.shulker.peeks);
        let shots = std::mem::take(&mut ai.state.shulker.shots);
        shulker::look_tick(
            &mut ai.state.look_control,
            self.shulker.opening(),
            position,
            eye_height,
        );
        for amount in peeks {
            self.set_raw_peek(amount);
        }
        let position = self.position();
        shots
            .into_iter()
            .map(|(target, pitch)| {
                self.voices
                    .push((Voice::Event("entity.shulker.shoot", 2.0, pitch), position));
                target
            })
            .collect()
    }
}

impl EntityWorld {
    pub fn spawn_shulker(&mut self, shulker: Shulker, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let mut ai = MonsterAi::of_kind(MonsterKind::Shulker, &shulker.body, 0.0);
        ai.state.max_health = shulker::MAX_HEALTH;
        self.shulkers.push(ShulkerEntity {
            effects: Default::default(),
            id,
            previous_position: shulker.body.position,
            teleport_pending: false,
            shulker,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
        });
        self.order.push(EntityKey::Shulker(id));
        self.file_in_section(EntityKey::Shulker(id));
        id
    }

    pub fn shulkers(&self) -> &[ShulkerEntity] {
        &self.shulkers
    }

    pub fn shulker_mut(&mut self, id: u64) -> Option<&mut ShulkerEntity> {
        self.shulkers.iter_mut().find(|entity| entity.id == id)
    }

    pub fn take_shulker_teleports(&mut self) -> Vec<(Pos, Pos)> {
        std::mem::take(&mut self.shulker_teleports)
    }

    pub(super) fn shulker_box(&self, id: u64) -> Option<Aabb> {
        self.shulkers
            .iter()
            .find(|entity| entity.id == id)
            .map(|entity| entity.shulker.bounding_box())
    }

    fn other_shulker_boxes(&self, id: u64) -> Vec<Aabb> {
        self.shulkers
            .iter()
            .filter(|entity| entity.id != id && entity.shulker.health > 0.0)
            .map(|entity| entity.shulker.bounding_box())
            .collect()
    }

    fn shove_from_shulker(&mut self, world: &impl World, area: Aabb, push: DVec3) {
        let keys: Vec<EntityKey> = self
            .order
            .iter()
            .copied()
            .filter(|key| {
                !matches!(
                    key,
                    EntityKey::Shulker(_)
                        | EntityKey::Arrow(_)
                        | EntityKey::Potion(_)
                        | EntityKey::Fireball(_)
                )
            })
            .collect();
        for key in keys {
            let Some(body) = self.body_mut(key.id()) else {
                continue;
            };
            let half = f64::from(body.width / 2.0);
            let p = body.position;
            let own = (
                p - DVec3::new(half, 0.0, half),
                p + DVec3::new(half, f64::from(body.height), half),
            );
            if !shulker::overlaps(own, area) {
                continue;
            }
            let bounds = Bounds::standing(p, body.width, body.height);
            let moved = move_body(world, bounds, push, body.on_ground, body.step_height);
            body.position += moved;
            body.needs_sync = true;
        }
    }

    pub(super) fn shulker_hit_by_bullet(&mut self, world: &impl World, id: u64) {
        let others = self.other_shulker_boxes(id);
        let Some(entity) = self.shulkers.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entity.shulker.closed() {
            return;
        }
        let (position, (low, high)) = (entity.position(), entity.shulker.bounding_box());
        let color = entity.shulker.color;
        let Some(moved) = entity.teleport(world, &others) else {
            return;
        };
        self.shulker_teleports.push(moved);
        self.file_in_section(EntityKey::Shulker(id));
        let area = (low - DVec3::splat(8.0), high + DVec3::splat(8.0));
        let nearby = self
            .shulkers
            .iter()
            .filter(|e| e.shulker.health > 0.0 && shulker::overlaps(e.shulker.bounding_box(), area))
            .count();
        if self.level_random.next_float() < (nearby as f32 - 1.0) / 5.0 {
            return;
        }
        let mut copy = Shulker::new(position);
        copy.color = color;
        let copy = self.spawn_shulker(copy, false);
        let seed = self.fresh_seed();
        if let Some(entity) = self.shulker_mut(copy) {
            entity.set_random_seed(seed);
        }
    }

    pub(super) fn tick_shulker(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty) = (self.game_time, self.difficulty);
        let mobs = self.mob_candidates();
        let others = self.other_shulker_boxes(id);
        let Some(entity) = self.shulkers.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.ai.state.mobs = mobs;
        entity.tick_count += 1;
        entity.shulker.tick_teleport_lerp();
        let mut teleported = None;
        if std::mem::take(&mut entity.teleport_pending) {
            teleported = entity.teleport(world, &others);
        }
        entity.previous_position = entity.shulker.body.position;
        entity.shulker.body.velocity = DVec3::ZERO;
        base_tick_fluid(
            &mut entity.shulker.body,
            world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            SHULKER_SOUNDS,
        );
        super::hazards::burn(entity, world, game_time);
        let eye = entity.shulker.eye_height();
        super::hazards::suffocate(entity, world, eye, game_time);
        let water_breathing = entity
            .effects
            .has(crate::effects::MobEffect::WaterBreathing);
        if entity.shulker.health > 0.0
            && super::living::breathe(&mut entity.shulker.body, world, eye, water_breathing)
        {
            entity.hurt(2.0);
        }
        let removed = entity.shulker.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        for work in entity.effects.tick(false) {
            match work.resolve(entity.shulker.health, shulker::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut entity.shulker.health, shulker::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt(amount);
                }
                _ => {}
            }
        }
        if entity.shulker.health > 0.0 {
            if (entity.random.next_int(1000) as i32) < entity.ambient_sound_time {
                entity.ambient_sound_time = -shulker::AMBIENT_SOUND_INTERVAL;
                if !entity.shulker.closed() {
                    let pitch = voice_pitch(&mut entity.random, false);
                    let position = entity.position();
                    entity.voices.push((Voice::Ambient(pitch), position));
                }
            } else {
                entity.ambient_sound_time += 1;
            }
        }
        if removed {
            if let Some(moved) = teleported {
                self.shulker_teleports.push(moved);
                self.file_in_section(EntityKey::Shulker(id));
            }
            return;
        }
        let own = entity.shulker.block();
        let can_stay = shulker::can_stay_at(world, own, entity.shulker.attach_face, own, &others);
        let mut shots = Vec::new();
        if entity.shulker.health > 0.0 && !entity.no_ai {
            shots = entity.tick_ai(world, players, game_time, difficulty, can_stay);
        }
        let (low, high) = entity.shulker.bounding_box();
        let axis = crate::shulker_bullet::axis_of(entity.shulker.attach_face);
        entity.shulker.body.velocity = DVec3::ZERO;
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, world, old, game_time);
        for target in shots {
            self.shoot_shulker_bullet(world, id, (low + high) * 0.5, target, axis, players);
        }
        self.push_entities(EntityKey::Shulker(id), world, game_time, ticks);
        let Some(entity) = self.shulkers.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.shulker.body.velocity = DVec3::ZERO;
        if !can_stay {
            match shulker::find_attachable_surface(world, own, own, &others) {
                Some(face) => {
                    entity.shulker.attach_face = face;
                    entity.shulker.refresh_dimensions();
                }
                None => teleported = teleported.or_else(|| entity.teleport(world, &others)),
            }
        }
        let mut shove = None;
        if entity.shulker.update_peek() {
            entity.shulker.refresh_dimensions();
            shove = entity.shulker.peek_growth();
        }
        if let Some(moved) = teleported {
            self.shulker_teleports.push(moved);
            self.file_in_section(EntityKey::Shulker(id));
        }
        if let Some((area, push)) = shove {
            self.shove_from_shulker(world, area, push);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::Face;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Blocks(RefCell<HashMap<Pos, &'static str>>);

    impl Blocks {
        fn put(&self, pos: Pos, id: &'static str) {
            self.0.borrow_mut().insert(pos, id);
        }
        fn clear(&self, pos: Pos) {
            self.0.borrow_mut().remove(&pos);
        }
        fn floor(y: i32) -> Self {
            let blocks = Self::default();
            for x in -12..=12 {
                for z in -12..=12 {
                    blocks.put((x, y, z), "minecraft:end_stone");
                }
            }
            blocks
        }
    }

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<minecraftoss_player::Block> {
            self.0
                .borrow()
                .get(&pos)
                .map(|id| minecraftoss_player::Block::new(*id))
        }
        fn set_block(&mut self, _: Pos, _: Option<minecraftoss_player::Block>) {}
        fn collision_boxes(&self, pos: Pos) -> Vec<[f64; 6]> {
            match self.0.borrow().get(&pos) {
                Some(&"minecraft:water") | None => Vec::new(),
                Some(_) => vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
            }
        }
        fn min_y(&self) -> i32 {
            0
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

    fn shulker_world(at: DVec3, seed: u64) -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        let id = world.spawn_shulker(Shulker::new(at), false);
        world.shulker_mut(id).unwrap().set_random_seed(seed);
        (world, id)
    }

    #[test]
    fn a_lone_shulker_peeks_now_and_then_with_its_sounds() {
        let mut blocks = Blocks::floor(19);
        let (mut world, _) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 3);
        let (mut opened, mut closed, mut highest) = (0, 0, 0.0_f32);
        let mut sounds = Vec::new();
        for _ in 0..1200 {
            world.tick_with_players(&mut blocks, &[]);
            highest = highest.max(world.shulkers()[0].shulker.peek);
            sounds.extend(world.take_sounds());
        }
        for sound in &sounds {
            match sound.event.as_str() {
                "entity.shulker.open" => opened += 1,
                "entity.shulker.close" => closed += 1,
                _ => {}
            }
        }
        assert!(opened >= 3, "{opened}");
        assert!(
            closed >= opened - 1 && closed <= opened,
            "{opened} {closed}"
        );
        assert!((highest - 0.3).abs() < 1e-6, "it peeks to 30: {highest}");
        let shulker = &world.shulkers()[0].shulker;
        assert_eq!(shulker.body.position, DVec3::new(0.5, 20.0, 0.5));
        assert_eq!(shulker.attach_face, Face::Down);
        assert!(sounds.iter().all(|s| s.category == "hostile_volume"));
    }

    #[test]
    fn a_shulker_whose_floor_goes_clings_to_a_wall_or_teleports() {
        let mut blocks = Blocks::floor(19);
        blocks.put((1, 20, 0), "minecraft:purpur_block");
        let (mut world, _) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 5);
        world.tick_with_players(&mut blocks, &[]);
        blocks.clear((0, 19, 0));
        world.tick_with_players(&mut blocks, &[]);
        let shulker = &world.shulkers()[0].shulker;
        assert_eq!(shulker.attach_face, Face::East);
        assert_eq!(shulker.body.position, DVec3::new(0.5, 20.0, 0.5));
        assert!(world.take_shulker_teleports().is_empty());
        blocks.clear((1, 20, 0));
        let mut teleports = Vec::new();
        for _ in 0..40 {
            world.tick_with_players(&mut blocks, &[]);
            teleports.extend(world.take_shulker_teleports());
            if !teleports.is_empty() {
                break;
            }
        }
        assert_eq!(teleports.len(), 1, "with nothing to cling to it teleports");
        let (from, to) = teleports[0];
        assert_eq!(from, (0, 20, 0));
        let shulker = &world.shulkers()[0].shulker;
        assert_eq!(shulker.block(), to);
        assert!(
            (to.0 - from.0).abs() <= 8 && (to.1 - from.1).abs() <= 8 && (to.2 - from.2).abs() <= 8
        );
        assert!(shulker::can_stay_at(
            &blocks,
            to,
            shulker.attach_face,
            to,
            &[]
        ));
        assert_eq!(shulker.teleport_from, Some(from));
        assert!(
            world
                .take_sounds()
                .iter()
                .any(|s| s.event == "entity.shulker.teleport")
        );
    }

    #[test]
    fn water_or_a_block_over_the_lid_sends_it_away() {
        for (pos, id) in [
            ((0, 20, 0), "minecraft:water"),
            ((0, 21, 0), "minecraft:end_stone"),
        ] {
            let mut blocks = Blocks::floor(19);
            let (mut world, _) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 9);
            world.tick_with_players(&mut blocks, &[]);
            blocks.put(pos, id);
            for _ in 0..5 {
                world.tick_with_players(&mut blocks, &[]);
            }
            assert_ne!(world.shulkers()[0].shulker.block(), (0, 20, 0), "{id}");
            assert_eq!(world.take_shulker_teleports().len(), 1, "{id}");
        }
    }

    #[test]
    fn a_teleport_resets_the_peek_and_drops_the_target() {
        let mut blocks = Blocks::floor(19);
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 11);
        let near = player(7, DVec3::new(0.5, 20.0, 6.5));
        for _ in 0..40 {
            world.tick_with_players(&mut blocks, &[near]);
        }
        let entity = world.shulker_mut(id).unwrap();
        assert!(
            entity.ai.state.target().is_some(),
            "it took the player as its target"
        );
        entity.shulker.raw_peek = 30;
        blocks.clear((0, 19, 0));
        for _ in 0..40 {
            world.tick_with_players(&mut blocks, &[near]);
            if !world.take_shulker_teleports().is_empty() {
                break;
            }
        }
        let entity = &world.shulkers()[0];
        assert_ne!(entity.shulker.block(), (0, 20, 0));
        assert_eq!(entity.shulker.raw_peek, 0);
        assert!(entity.ai.state.target.is_none());
    }

    #[test]
    fn a_shulker_turns_its_head_to_a_nearby_player_and_never_moves() {
        let mut blocks = Blocks::floor(19);
        let (mut world, _) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 2);
        let east = player(7, DVec3::new(6.5, 20.0, 0.5));
        let mut turned = false;
        for _ in 0..400 {
            world.tick_with_players(&mut blocks, &[east]);
            turned |= (world.shulkers()[0].head_yaw() + 90.0).abs() < 1.0;
            let body = &world.shulkers()[0].shulker.body;
            assert_eq!(body.position, DVec3::new(0.5, 20.0, 0.5));
            assert_eq!(body.velocity, DVec3::ZERO);
        }
        assert!(turned, "{}", world.shulkers()[0].head_yaw());
    }

    #[test]
    fn a_far_climber_is_acquired_like_any_player_in_sight() {
        let mut blocks = Blocks::floor(19);
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 4);
        let high = player(7, DVec3::new(0.5, 30.0, 6.5));
        for _ in 0..60 {
            world.tick_with_players(&mut blocks, &[high]);
        }
        let entity = world.shulker_mut(id).unwrap();
        assert_eq!(
            entity.ai.state.target().map(|t| t.target),
            Some(crate::monster_ai::Target::Player(7)),
            "the axis box only bounds non-player targets"
        );
        let far = player(7, DVec3::new(0.5, 20.0, 17.5));
        for _ in 0..20 {
            world.tick_with_players(&mut blocks, &[far]);
        }
        assert!(
            world.shulkers()[0].ai.state.target().is_none(),
            "beyond its follow range it lets go"
        );
        let (mut world, _) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 4);
        world.set_difficulty(0);
        for _ in 0..60 {
            world.tick_with_players(&mut blocks, &[high]);
        }
        assert!(
            world.shulkers()[0].ai.state.target().is_none(),
            "not in peaceful"
        );
    }

    #[test]
    fn an_opening_shulker_pushes_a_mob_in_its_way() {
        let rise = |peek: i32| {
            let mut blocks = Blocks::floor(19);
            let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
            let zombie = crate::zombie::Zombie::new(DVec3::new(0.5, 20.0, 0.5));
            let zombie = world.spawn_zombie(zombie, true);
            world.shulker_mut(id).unwrap().no_ai = true;
            world.shulker_mut(id).unwrap().shulker.raw_peek = peek;
            let mut highest = 20.0_f64;
            for _ in 0..10 {
                world.tick_with_players(&mut blocks, &[]);
                let body = &world
                    .zombies()
                    .iter()
                    .find(|e| e.id == zombie)
                    .unwrap()
                    .zombie
                    .body;
                highest = highest.max(body.position.y);
            }
            highest - 20.0
        };
        assert!(rise(100) > 0.005, "{}", rise(100));
        assert_eq!(rise(0), 0.0);
    }

    fn attack(player_id: u64, damage: f64) -> crate::world::PlayerAttack {
        crate::world::PlayerAttack {
            player_id,
            position: DVec3::new(0.5, 20.0, 3.5),
            yaw: 180.0,
            attack_damage: damage,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        }
    }

    #[test]
    fn the_shell_armour_comes_only_with_the_first_close() {
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        world.player_attack(&attack(7, 10.0), id);
        assert_eq!(
            world.shulkers()[0].shulker.health,
            20.0,
            "a fresh shulker has no shell armour"
        );
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        let entity = world.shulker_mut(id).unwrap();
        entity.set_raw_peek(30);
        entity.set_raw_peek(0);
        world.player_attack(&attack(7, 10.0), id);
        assert!(
            (world.shulkers()[0].shulker.health - 26.0).abs() < 1e-4,
            "armour 20 takes 10 down to 4"
        );
        let (mut open, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        let entity = open.shulker_mut(id).unwrap();
        entity.set_raw_peek(30);
        entity.set_raw_peek(0);
        entity.set_raw_peek(100);
        open.player_attack(&attack(7, 10.0), id);
        assert_eq!(
            open.shulkers()[0].shulker.health,
            20.0,
            "open, it takes the full hit"
        );
    }

    #[test]
    fn a_closed_shulker_turns_arrows_away_and_an_open_one_takes_them() {
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        let arrow = crate::projectile::ArrowImpact {
            target_id: id,
            damage: 6.0,
            velocity: DVec3::new(0.0, 0.0, -2.0),
        };
        let refused = world.apply_arrow_hit(arrow, 99).unwrap();
        assert!(!refused.applied);
        assert_eq!(world.shulkers()[0].shulker.health, shulker::MAX_HEALTH);
        world.shulker_mut(id).unwrap().shulker.raw_peek = 30;
        let taken = world.apply_arrow_hit(arrow, 99).unwrap();
        assert!(taken.applied);
        assert_eq!(world.shulkers()[0].shulker.health, 24.0);
        assert_eq!(
            world.shulkers()[0].shulker.body.velocity,
            DVec3::ZERO,
            "no knockback"
        );
    }

    #[test]
    fn below_half_health_one_hit_in_four_teleports_it() {
        let mut blocks = Blocks::floor(19);
        let (mut rolls, mut teleports) = (0, 0);
        for seed in 0..80 {
            let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), seed);
            world.shulker_mut(id).unwrap().shulker.health = 16.0;
            world.player_attack(&attack(7, 2.0), id);
            let pending = world.shulkers()[0].teleport_pending;
            rolls += usize::from(pending);
            world.tick_with_players(&mut blocks, &[]);
            let moved = world.take_shulker_teleports();
            assert!(
                moved.len() <= usize::from(pending),
                "only a rolled hit teleports it"
            );
            if let Some(&(from, to)) = moved.first() {
                assert_eq!(
                    (from, world.shulkers()[0].shulker.block()),
                    ((0, 20, 0), to)
                );
                teleports += 1;
            }
        }
        assert!((10..=32).contains(&rolls), "{rolls} of 80");
        assert!(teleports > 0);
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 3);
        for _ in 0..10 {
            world.player_attack(&attack(7, 1.0), id);
            for _ in 0..11 {
                world.tick_with_players(&mut blocks, &[]);
            }
        }
        assert_eq!(
            world.shulkers()[0].shulker.block(),
            (0, 20, 0),
            "above half health it stays"
        );
    }

    #[test]
    fn hits_and_blasts_never_move_it() {
        let mut blocks = Blocks::floor(19);
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        world.player_attack(&attack(7, 2.0), id);
        world.player_blast(Some(7), id, 2.0, DVec3::new(2.5, 20.0, 0.5), 1.0);
        world.tick_with_players(&mut blocks, &[]);
        let body = &world.shulkers()[0].shulker.body;
        assert_eq!(
            (body.position, body.velocity),
            (DVec3::new(0.5, 20.0, 0.5), DVec3::ZERO)
        );
    }

    #[test]
    fn its_voices_follow_its_shell() {
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        world.player_attack(&attack(7, 2.0), id);
        let events: Vec<String> = world.take_sounds().into_iter().map(|s| s.event).collect();
        assert!(
            events.contains(&"entity.shulker.hurt_closed".to_owned()),
            "{events:?}"
        );
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        world.shulker_mut(id).unwrap().shulker.raw_peek = 30;
        world.player_attack(&attack(7, 2.0), id);
        let events: Vec<String> = world.take_sounds().into_iter().map(|s| s.event).collect();
        assert!(
            events.contains(&"entity.shulker.hurt".to_owned()),
            "{events:?}"
        );
        let mut blocks = Blocks::floor(19);
        let (mut quiet, qid) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        quiet.shulker_mut(qid).unwrap().no_ai = true;
        let mut ambient = 0;
        for _ in 0..4000 {
            quiet.tick_with_players(&mut blocks, &[]);
            ambient += quiet
                .take_sounds()
                .iter()
                .filter(|s| s.event == "entity.shulker.ambient")
                .count();
        }
        assert_eq!(ambient, 0, "closed, it makes no ambient sound");
        let (mut open, oid) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        open.shulker_mut(oid).unwrap().no_ai = true;
        open.shulker_mut(oid).unwrap().shulker.raw_peek = 30;
        for _ in 0..4000 {
            open.tick_with_players(&mut blocks, &[]);
            ambient += open
                .take_sounds()
                .iter()
                .filter(|s| s.event == "entity.shulker.ambient")
                .count();
        }
        assert!(ambient > 5, "{ambient}");
    }

    #[test]
    fn a_dead_shulker_leaves_five_experience_and_its_shell_about_half_the_time() {
        let (mut world, id) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        world.player_attack(&attack(7, 100.0), id);
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        assert_eq!(
            (deaths[0].table, deaths[0].experience),
            (Some("minecraft:shulker"), 5)
        );
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        let Ok(mut book) = crate::loot::EntityLootBook::from_jar(&jar, 1234) else {
            return;
        };
        let mut shells = 0;
        for _ in 0..400 {
            let drops = book.death_drops(vec![deaths[0].clone()]);
            assert!(
                drops
                    .iter()
                    .all(|(s, _)| s.id == "minecraft:shulker_shell" && s.count == 1)
            );
            shells += drops.len();
        }
        assert!((160..=240).contains(&shells), "{shells} of 400");
    }

    #[test]
    fn shulkers_stay_far_away_and_in_peaceful() {
        let (mut world, _) = shulker_world(DVec3::new(0.5, 20.0, 0.5), 1);
        let far = [DVec3::new(500.0, 20.0, 0.5)];
        for _ in 0..50 {
            world.check_despawn(&far, false, &|_| true);
        }
        world.check_despawn(&far, true, &|_| true);
        assert_eq!(world.shulkers().len(), 1);
    }
}
