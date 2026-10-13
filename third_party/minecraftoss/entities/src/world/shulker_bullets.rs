use super::*;
use crate::effects::{EffectInstance, MobEffect};
use crate::monster_ai::Target;
use crate::shulker_bullet::{self, Aim, Axis, BulletHit, BulletTarget, ShulkerBullet};

#[derive(Clone)]
pub struct ShulkerBulletEntity {
    pub id: u64,
    pub bullet: ShulkerBullet,
    pub previous_position: DVec3,
    pub random: LegacyRandom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BulletPuffKind {
    Hit,
    Hurt,
}

impl BulletPuffKind {
    pub fn count(self) -> usize {
        match self {
            Self::Hit => 2,
            Self::Hurt => 15,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BulletPuff {
    pub position: DVec3,
    pub kind: BulletPuffKind,
}

pub fn target_id(target: Target) -> u64 {
    match target {
        Target::Player(player) => PLAYER_TARGET + player,
        Target::Villager(id) | Target::Mob(id) => id,
    }
}

fn player_height(eye_height: f32) -> f32 {
    match eye_height {
        e if e >= 1.5 => 1.8,
        e if e >= 1.0 => 1.5,
        _ => 0.6,
    }
}

impl EntityWorld {
    pub fn spawn_shulker_bullet(&mut self, bullet: ShulkerBullet) -> u64 {
        let seed = self.fresh_seed();
        self.next_id += 1;
        let id = self.next_id;
        self.shulker_bullets.push(ShulkerBulletEntity {
            id,
            previous_position: bullet.position,
            bullet,
            random: LegacyRandom::new(seed),
        });
        self.order.push(EntityKey::ShulkerBullet(id));
        id
    }

    pub fn shulker_bullets(&self) -> &[ShulkerBulletEntity] {
        &self.shulker_bullets
    }

    pub fn shulker_bullet_mut(&mut self, id: u64) -> Option<&mut ShulkerBulletEntity> {
        self.shulker_bullets.iter_mut().find(|e| e.id == id)
    }

    pub fn take_bullet_puffs(&mut self) -> Vec<BulletPuff> {
        std::mem::take(&mut self.bullet_puffs)
    }

    pub(super) fn fresh_seed(&mut self) -> u64 {
        self.seed_uniquifier = self.seed_uniquifier.wrapping_mul(1_181_783_497_276_652_981);
        self.seed_uniquifier
    }

    pub fn id_of_uuid(&self, uuid: u128) -> Option<u64> {
        if let Some((&id, _)) = self.uuids.iter().find(|(_, known)| **known == uuid) {
            return Some(id);
        }
        self.order
            .iter()
            .map(|key| key.id())
            .find(|&id| self.uuid_of(id) == uuid)
    }

    pub fn hurt_shulker_bullet(&mut self, id: u64) -> bool {
        let Some(entity) = self
            .shulker_bullets
            .iter_mut()
            .find(|e| e.id == id && e.bullet.alive)
        else {
            return false;
        };
        entity.bullet.alive = false;
        let position = entity.bullet.position;
        self.bullet_sound("entity.shulker_bullet.hurt", position);
        self.bullet_puffs.push(BulletPuff {
            position,
            kind: BulletPuffKind::Hurt,
        });
        true
    }

    pub fn shulker_bullet_on_ray(&self, eye: DVec3, look: DVec3, reach: f64) -> Option<(u64, f64)> {
        let mut nearest: Option<(u64, f64)> = None;
        for entity in self.shulker_bullets.iter().filter(|e| e.bullet.alive) {
            let (min, max) = entity.bullet.bounds();
            if let Some(distance) = ray_box(eye, look, min, max, reach) {
                if nearest.is_none_or(|(_, best)| distance < best) {
                    nearest = Some((entity.id, distance));
                }
            }
        }
        nearest
    }

    fn bullet_sound(&mut self, event: &str, position: DVec3) {
        self.sounds.push(MobSound {
            event: event.to_owned(),
            position,
            volume: 1.0,
            pitch: 1.0,
            category: "hostile_volume",
        });
    }

    pub(super) fn shoot_shulker_bullet(
        &mut self,
        world: &impl World,
        owner: u64,
        from: DVec3,
        target: Target,
        axis: Axis,
        players: &[PlayerCandidate],
    ) -> u64 {
        let target = target_id(target);
        let aim = self
            .bullet_aim(target, players)
            .map(|(aim, _)| (target, aim));
        let seed = self.fresh_seed();
        let mut random = LegacyRandom::new(seed);
        let mut bullet = ShulkerBullet::shoot(world, owner, from, aim, axis, &mut random);
        bullet.target = Some(target);
        let id = self.spawn_shulker_bullet(bullet);
        if let Some(entity) = self.shulker_bullet_mut(id) {
            entity.random = random;
        }
        id
    }

    fn bullet_aim(&self, target: u64, players: &[PlayerCandidate]) -> Option<(Aim, bool)> {
        if target >= PLAYER_TARGET {
            let player = players.iter().find(|p| p.id == target - PLAYER_TARGET)?;
            let aim = Aim {
                position: player.position,
                height: player_height(player.eye_height),
            };
            return Some((aim, player.alive && !player.spectator));
        }
        let (body, health) = self.mob_body(target)?;
        let aim = Aim {
            position: body.position,
            height: body.height,
        };
        Some((aim, health > 0.0))
    }

    fn bullet_targets(&self, players: &[PlayerCandidate]) -> Vec<BulletTarget> {
        let mut targets: Vec<BulletTarget> = self
            .order
            .iter()
            .filter_map(|key| {
                if matches!(
                    key,
                    EntityKey::Arrow(_)
                        | EntityKey::Potion(_)
                        | EntityKey::Fireball(_)
                        | EntityKey::ShulkerBullet(_)
                        | EntityKey::LlamaSpit(_)
                ) {
                    return None;
                }
                let id = key.id();
                let (body, health) = self.mob_body(id)?;
                if health <= 0.0 {
                    return None;
                }
                let half = f64::from(body.width / 2.0);
                let p = body.position;
                let (min, max) = self.shulker_box(id).unwrap_or((
                    DVec3::new(p.x - half, p.y, p.z - half),
                    DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
                ));
                Some(BulletTarget { id, min, max })
            })
            .collect();
        targets.extend(
            players
                .iter()
                .filter(|p| self.players_pickable && p.alive && !p.spectator)
                .map(|p| BulletTarget {
                    id: PLAYER_TARGET + p.id,
                    min: p.position - DVec3::new(0.3, 0.0, 0.3),
                    max: p.position + DVec3::new(0.3, f64::from(player_height(p.eye_height)), 0.3),
                }),
        );
        targets
    }

    pub(super) fn tick_shulker_bullet(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
    ) {
        let Some(entity) = self
            .shulker_bullets
            .iter()
            .find(|e| e.id == id && e.bullet.alive)
        else {
            return;
        };
        let (mut owner, owner_uuid) = (entity.bullet.owner, entity.bullet.owner_uuid);
        let (mut target, target_uuid) = (entity.bullet.target, entity.bullet.target_uuid);
        if owner.is_none() {
            owner = owner_uuid.and_then(|uuid| self.id_of_uuid(uuid));
        }
        if let Some(uuid) = target_uuid {
            target = self.id_of_uuid(uuid);
        }
        let aim = target.and_then(|t| self.bullet_aim(t, players));
        if aim.is_none() {
            target = None;
        }
        let targets = self.bullet_targets(players);
        let Some(entity) = self.shulker_bullets.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if owner.is_some() {
            entity.bullet.owner = owner;
            entity.bullet.owner_uuid = None;
        }
        entity.bullet.target = target;
        entity.bullet.target_uuid = None;
        entity.previous_position = entity.bullet.position;
        let homing = aim.is_some_and(|(_, homes)| homes);
        let hit = entity.bullet.tick(
            world,
            &targets,
            aim.map(|(aim, _)| aim),
            homing,
            &mut entity.random,
        );
        let Some(hit) = hit.filter(|_| entity.bullet.alive) else {
            return;
        };
        entity.bullet.alive = false;
        let bullet = entity.bullet.clone();
        self.shulker_bullet_hit(world, &bullet, hit, players);
    }

    fn shulker_bullet_hit(
        &mut self,
        world: &impl World,
        bullet: &ShulkerBullet,
        hit: BulletHit,
        players: &[PlayerCandidate],
    ) {
        let owner = bullet.owner.filter(|&o| self.entity_type(o).is_some());
        match hit.entity {
            Some(target) if target >= PLAYER_TARGET => {
                let player = target - PLAYER_TARGET;
                if players.iter().any(|p| p.id == player && p.attackable) {
                    self.player_hits.push(PlayerHit {
                        player_id: player,
                        damage: shulker_bullet::DAMAGE,
                        kind: PlayerHitKind::ShulkerBullet {
                            velocity: bullet.velocity,
                        },
                        source: owner,
                    });
                }
            }
            Some(victim) => self.shulker_bullet_hurts_mob(world, victim, bullet.velocity, owner),
            None => {
                self.bullet_sound("entity.shulker_bullet.hit", bullet.position);
                self.bullet_puffs.push(BulletPuff {
                    position: bullet.position,
                    kind: BulletPuffKind::Hit,
                });
            }
        }
    }

    fn shulker_bullet_hurts_mob(
        &mut self,
        world: &impl World,
        victim: u64,
        velocity: DVec3,
        owner: Option<u64>,
    ) {
        let result = if let Some(entity) = self.shulkers.iter_mut().find(|e| e.id == victim) {
            let pending = entity.teleport_pending;
            let result = entity.hurt_armored(shulker_bullet::DAMAGE);
            if let Some(owner) = owner.filter(|_| result.applied) {
                entity.ai.state.hurt_by = Some((Target::Mob(owner), entity.tick_count));
            }
            let rolled = !pending && entity.teleport_pending;
            if result.applied && !rolled {
                self.shulker_hit_by_bullet(world, victim);
            }
            Some(result)
        } else {
            let impact = ArrowImpact {
                target_id: victim,
                damage: shulker_bullet::DAMAGE,
                velocity,
            };
            self.apply_arrow_hit(impact, owner.unwrap_or(victim))
        };
        let Some(result) = result else {
            return;
        };
        if let Some(kind) = owner.and_then(|o| self.entity_type(o)) {
            self.credit(victim, result, kind, false);
        }
        if result.applied {
            let levitation = EffectInstance::new(
                MobEffect::Levitation,
                shulker_bullet::LEVITATION_TICKS as i32,
                0,
            );
            self.add_mob_effect(victim, levitation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shulker::Shulker;
    use minecraftoss_player::Pos;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Blocks(RefCell<HashMap<Pos, &'static str>>);

    impl Blocks {
        fn floor(y: i32) -> Self {
            let blocks = Self::default();
            for x in -24..=24 {
                for z in -24..=24 {
                    blocks
                        .0
                        .borrow_mut()
                        .insert((x, y, z), "minecraft:end_stone");
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
                Some(_) => vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
                None => Vec::new(),
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

    fn shulker_world(seed: u64) -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        world.set_players_pickable(true);
        let id = world.spawn_shulker(Shulker::new(DVec3::new(0.5, 20.0, 0.5)), false);
        world.shulker_mut(id).unwrap().set_random_seed(seed);
        (world, id)
    }

    #[test]
    fn a_shulker_opens_and_fires_every_one_to_five_and_a_half_seconds() {
        let mut blocks = Blocks::floor(19);
        let (mut world, id) = shulker_world(3);
        let target = player(7, DVec3::new(0.5, 20.0, 12.5));
        let mut shots = Vec::new();
        let mut known = Vec::new();
        let mut opened = false;
        for tick in 0..1200 {
            world.tick_with_players(&mut blocks, &[target]);
            let _ = world.take_player_hits();
            if !world.take_shulker_teleports().is_empty() {
                break;
            }
            opened |= world.shulkers()[0].shulker.raw_peek == 100;
            for bullet in world.shulker_bullets() {
                if !known.contains(&bullet.id) {
                    known.push(bullet.id);
                    shots.push(tick);
                    assert_eq!(bullet.bullet.owner, Some(id));
                    assert_eq!(bullet.bullet.target, Some(PLAYER_TARGET + 7));
                }
            }
            for sound in world.take_sounds() {
                if sound.event == "entity.shulker.shoot" {
                    assert_eq!(sound.volume, 2.0);
                    assert!((0.8..=1.2).contains(&sound.pitch));
                }
            }
        }
        assert!(opened, "it opens fully to fire");
        assert!(shots.len() >= 3, "{shots:?}");
        for pair in shots.windows(2) {
            let gap = pair[1] - pair[0];
            assert!((20..=110).contains(&gap) && gap % 10 == 0, "{shots:?}");
        }
    }

    #[test]
    fn a_bullet_hurts_the_player_and_none_fly_in_peaceful() {
        let mut blocks = Blocks::floor(19);
        let (mut world, id) = shulker_world(3);
        let target = player(7, DVec3::new(0.5, 20.0, 8.5));
        let mut hits = Vec::new();
        for _ in 0..600 {
            world.tick_with_players(&mut blocks, &[target]);
            hits.extend(world.take_player_hits());
        }
        let hit = hits
            .iter()
            .find(|h| matches!(h.kind, PlayerHitKind::ShulkerBullet { .. }))
            .expect("a bullet strikes the player");
        assert_eq!((hit.damage, hit.source, hit.player_id), (4.0, Some(id), 7));
        world.check_despawn(&[target.position], true, &|_| true);
        assert!(world.shulker_bullets().is_empty());
        world.set_difficulty(0);
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &[target]);
            world.check_despawn(&[target.position], true, &|_| true);
        }
        assert!(world.shulker_bullets().is_empty());
        assert!(world.take_player_hits().is_empty());
        assert_eq!(world.shulkers().len(), 1, "the shulker stays");
        assert!(world.shulkers()[0].shulker.raw_peek < 100, "and closes");
    }

    #[test]
    fn a_creative_player_is_struck_but_unhurt() {
        let mut blocks = Blocks::floor(19);
        let (mut world, id) = shulker_world(3);
        let mut target = player(7, DVec3::new(0.5, 20.0, 3.5));
        target.attackable = false;
        let bullet = ShulkerBullet::shoot(
            &blocks,
            id,
            DVec3::new(0.5, 20.5, 0.5),
            Some((
                PLAYER_TARGET + 7,
                Aim {
                    position: target.position,
                    height: 1.8,
                },
            )),
            Axis::Y,
            &mut LegacyRandom::new(1),
        );
        world.shulker_mut(id).unwrap().no_ai = true;
        world.spawn_shulker_bullet(bullet);
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &[target]);
        }
        assert!(world.shulker_bullets().is_empty(), "it is gone");
        assert!(world.take_player_hits().is_empty());
    }

    #[test]
    fn a_struck_zombie_floats_up_for_ten_seconds() {
        let mut blocks = Blocks::floor(19);
        let (mut world, shulker) = shulker_world(1);
        world.shulker_mut(shulker).unwrap().no_ai = true;
        let zombie = crate::zombie::Zombie::new(DVec3::new(0.5, 20.0, 4.5));
        let zombie = world.spawn_zombie_active(zombie, 0.0);
        let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 20.5, 3.9));
        bullet.owner = Some(shulker);
        bullet.target = Some(zombie);
        bullet.velocity = DVec3::new(0.0, 0.0, 0.3);
        bullet.target_delta = DVec3::new(0.0, 0.0, 0.3);
        world.spawn_shulker_bullet(bullet);
        world.tick_with_players(&mut blocks, &[]);
        assert!(world.shulker_bullets().is_empty());
        let effects = world.mob_effects(zombie).unwrap();
        assert!(effects.has(MobEffect::Levitation));
        let health = world.zombies()[0].zombie.health;
        assert!(
            (health - (20.0 - damage_after_armor(4.0, 2.0, 0.0))).abs() < 1e-4,
            "{health}"
        );
        for _ in 0..40 {
            world.tick_with_players(&mut blocks, &[]);
        }
        assert!(world.zombies()[0].zombie.body.position.y > 21.0);
    }

    #[test]
    fn a_struck_open_shulker_jumps_away_and_a_lone_one_leaves_a_copy() {
        let mut copies = 0;
        for seed in 0..12 {
            let mut blocks = Blocks::floor(19);
            let (mut world, shulker) = shulker_world(seed);
            let entity = world.shulker_mut(shulker).unwrap();
            entity.shulker.color = Some(14);
            entity.shulker.raw_peek = 100;
            entity.shulker.peek = 1.0;
            let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 21.0, 4.5));
            bullet.target = Some(shulker);
            bullet.velocity = DVec3::new(0.0, 0.0, -0.3);
            bullet.target_delta = DVec3::new(0.0, 0.0, -0.3);
            world.spawn_shulker_bullet(bullet);
            for _ in 0..12 {
                world.tick_with_players(&mut blocks, &[]);
            }
            let moved = world.take_shulker_teleports();
            if moved.is_empty() {
                continue;
            }
            assert_eq!(moved[0].0, (0, 20, 0));
            assert_eq!(
                world.shulkers().len(),
                2,
                "seed {seed}: one alone always splits"
            );
            let copy = &world.shulkers()[1].shulker;
            assert_eq!(copy.block(), (0, 20, 0));
            assert_eq!(copy.color, Some(14));
            copies += 1;
        }
        assert!(copies > 0);
        let blocks = Blocks::floor(19);
        let (mut world, shulker) = shulker_world(2);
        world.shulker_hit_by_bullet(&blocks, shulker);
        assert!(
            world.take_shulker_teleports().is_empty(),
            "closed, it stays"
        );
        assert_eq!(world.shulkers().len(), 1);
    }

    #[test]
    fn any_hurt_pops_a_bullet_and_the_ground_stops_one() {
        let mut blocks = Blocks::floor(19);
        let mut world = EntityWorld::default();
        let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 20.5, 0.5));
        bullet.velocity = DVec3::new(0.0, 0.0, 0.15);
        bullet.target_delta = DVec3::new(0.0, 0.0, 0.15);
        let id = world.spawn_shulker_bullet(bullet.clone());
        let look = DVec3::new(0.0, 0.0, 1.0);
        let (found, distance) = world
            .shulker_bullet_on_ray(DVec3::new(0.5, 20.6, -3.0), look, 5.0)
            .unwrap();
        assert_eq!(found, id);
        assert!((distance - (3.0 + 0.5 - 0.15625)).abs() < 1e-9);
        assert!(world.hurt_shulker_bullet(id));
        assert!(!world.hurt_shulker_bullet(id), "only once");
        assert_eq!(
            world.take_bullet_puffs(),
            vec![BulletPuff {
                position: DVec3::new(0.5, 20.5, 0.5),
                kind: BulletPuffKind::Hurt
            }]
        );
        assert!(
            world
                .take_sounds()
                .iter()
                .any(|s| s.event == "entity.shulker_bullet.hurt")
        );
        world.tick_with_players(&mut blocks, &[]);
        assert!(world.shulker_bullets().is_empty());
        world.spawn_shulker_bullet(bullet);
        for _ in 0..30 {
            world.tick_with_players(&mut blocks, &[]);
        }
        assert!(world.shulker_bullets().is_empty());
        let puffs = world.take_bullet_puffs();
        assert_eq!(puffs.len(), 1);
        assert_eq!(puffs[0].kind, BulletPuffKind::Hit);
        assert!(
            world
                .take_sounds()
                .iter()
                .any(|s| s.event == "entity.shulker_bullet.hit")
        );
    }

    #[test]
    fn a_saved_target_is_found_again_by_its_uuid() {
        let mut blocks = Blocks::floor(19);
        let mut world = EntityWorld::default();
        world.set_players_pickable(true);
        world.set_player_uuid(7, 0xabc);
        let target = player(7, DVec3::new(0.5, 20.0, 6.5));
        let mut bullet = ShulkerBullet::new(DVec3::new(0.5, 21.0, 0.5));
        bullet.target_uuid = Some(0xabc);
        let id = world.spawn_shulker_bullet(bullet);
        world.tick_with_players(&mut blocks, &[target]);
        assert_eq!(
            world.shulker_bullets()[0].bullet.target,
            Some(PLAYER_TARGET + 7)
        );
        let mut lost = ShulkerBullet::new(DVec3::new(0.5, 21.0, 0.5));
        lost.target_uuid = Some(0xdef);
        let lost = world.spawn_shulker_bullet(lost);
        world.tick_with_players(&mut blocks, &[target]);
        let lost = world
            .shulker_bullets()
            .iter()
            .find(|e| e.id == lost)
            .unwrap();
        assert_eq!((lost.bullet.target, lost.bullet.target_uuid), (None, None));
        assert!(world.shulker_bullets().iter().any(|e| e.id == id));
    }
}
