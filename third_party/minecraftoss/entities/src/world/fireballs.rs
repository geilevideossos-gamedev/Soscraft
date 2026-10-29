use super::*;
use crate::fireball::{Fireball, FireballHit, FireballKind, FireballTarget, Shooter};

#[derive(Clone)]
pub struct FireballEntity {
    pub id: u64,
    pub fireball: Fireball,
    pub previous_position: DVec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireballBlast {
    pub position: DVec3,
    pub power: f32,
    pub fire: bool,
    pub dangerous: bool,
}

impl EntityWorld {
    pub fn spawn_fireball(&mut self, fireball: Fireball) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let previous_position = fireball.position;
        self.fireballs.push(FireballEntity {
            id,
            fireball,
            previous_position,
        });
        self.order.push(EntityKey::Fireball(id));
        id
    }

    pub fn fireballs(&self) -> &[FireballEntity] {
        &self.fireballs
    }

    pub fn fireball_mut(&mut self, id: u64) -> Option<&mut FireballEntity> {
        self.fireballs.iter_mut().find(|e| e.id == id)
    }

    pub fn take_fireball_blasts(&mut self) -> Vec<FireballBlast> {
        std::mem::take(&mut self.fireball_blasts)
    }

    pub fn take_fireball_fires(&mut self) -> Vec<(i32, i32, i32)> {
        std::mem::take(&mut self.fireball_fires)
    }

    pub fn deflect_fireball(&mut self, id: u64, look: DVec3, player: u64, by_attack: bool) -> bool {
        let Some(entity) = self
            .fireballs
            .iter_mut()
            .find(|e| e.id == id && e.fireball.alive)
        else {
            return false;
        };
        if !entity.fireball.kind.redirectable() {
            return false;
        }
        entity
            .fireball
            .deflect(look, Shooter::Player(player), by_attack);
        true
    }

    pub fn push_fireball(&mut self, id: u64, push: DVec3) -> bool {
        let Some(entity) = self
            .fireballs
            .iter_mut()
            .find(|e| e.id == id && e.fireball.alive)
        else {
            return false;
        };
        entity.fireball.velocity += push;
        true
    }

    pub fn fireball_on_ray(&self, eye: DVec3, look: DVec3, reach: f64) -> Option<(u64, f64)> {
        let mut nearest: Option<(u64, f64)> = None;
        for entity in self
            .fireballs
            .iter()
            .filter(|e| e.fireball.alive && e.fireball.kind.redirectable())
        {
            let (min, max) =
                crate::fireball::pick_box(entity.fireball.kind, entity.fireball.position);
            if let Some(distance) = ray_box(eye, look, min, max, reach) {
                if nearest.is_none_or(|(_, best)| distance < best) {
                    nearest = Some((entity.id, distance));
                }
            }
        }
        nearest
    }

    fn fireball_targets(
        &self,
        owner: Option<Shooter>,
        players: &[PlayerCandidate],
    ) -> Vec<FireballTarget> {
        let mut targets: Vec<FireballTarget> = self
            .order
            .iter()
            .filter_map(|key| {
                if matches!(
                    key,
                    EntityKey::Arrow(_) | EntityKey::Potion(_) | EntityKey::Fireball(_)
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
                Some(FireballTarget {
                    id,
                    min,
                    max,
                    owner: owner == Some(Shooter::Mob(id)),
                })
            })
            .collect();
        targets.extend(
            players
                .iter()
                .filter(|p| self.players_pickable && p.alive && !p.spectator)
                .map(|p| FireballTarget {
                    id: PLAYER_TARGET + p.id,
                    min: p.position - DVec3::new(0.3, 0.0, 0.3),
                    max: p.position + DVec3::new(0.3, 1.8, 0.3),
                    owner: owner == Some(Shooter::Player(p.id)),
                }),
        );
        targets
    }

    fn shooter_box(
        &self,
        owner: Option<Shooter>,
        players: &[PlayerCandidate],
    ) -> Option<(DVec3, DVec3)> {
        match owner? {
            Shooter::Mob(id) => self.mob_body(id).map(|(body, _)| {
                let half = f64::from(body.width / 2.0);
                let p = body.position;
                (
                    DVec3::new(p.x - half, p.y, p.z - half),
                    DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
                )
            }),
            Shooter::Player(id) => players.iter().find(|p| p.id == id).map(|p| {
                (
                    p.position - DVec3::new(0.3, 0.0, 0.3),
                    p.position + DVec3::new(0.3, 1.8, 0.3),
                )
            }),
        }
    }

    fn shooter_of_uuid(&self, uuid: u128) -> Option<Shooter> {
        let id = self
            .uuids
            .iter()
            .find(|&(&id, &known)| {
                known == uuid && (id >= PLAYER_TARGET || self.entity_type(id).is_some())
            })
            .map(|(&id, _)| id)
            .or_else(|| {
                self.order
                    .iter()
                    .map(|key| key.id())
                    .find(|&id| self.uuid_of(id) == uuid)
            })?;
        Some(if id >= PLAYER_TARGET {
            Shooter::Player(id - PLAYER_TARGET)
        } else {
            Shooter::Mob(id)
        })
    }

    pub(super) fn tick_fireball(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
    ) {
        let Some((owner, owner_uuid)) = self
            .fireballs
            .iter()
            .find(|e| e.id == id)
            .map(|e| (e.fireball.owner, e.fireball.owner_uuid))
        else {
            return;
        };
        let owner = owner.or_else(|| owner_uuid.and_then(|uuid| self.shooter_of_uuid(uuid)));
        if owner.is_some() && owner_uuid.is_some() {
            if let Some(entity) = self.fireballs.iter_mut().find(|e| e.id == id) {
                entity.fireball.owner = owner;
                entity.fireball.owner_uuid = None;
            }
        }
        if let Some(Shooter::Mob(shooter)) = owner {
            if self.entity_type(shooter).is_none() {
                if let Some(entity) = self.fireballs.iter_mut().find(|e| e.id == id) {
                    entity.fireball.alive = false;
                }
                return;
            }
        }
        let targets = self.fireball_targets(owner, players);
        let owner_box = self.shooter_box(owner, players);
        let Some(entity) = self.fireballs.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.previous_position = entity.fireball.position;
        let Some(hit) = entity.fireball.tick(world, &targets, owner_box) else {
            return;
        };
        entity.fireball.alive = false;
        let fireball = entity.fireball.clone();
        self.fireball_hit(world, &fireball, hit, players);
    }

    fn fireball_hit(
        &mut self,
        world: &impl World,
        fireball: &Fireball,
        hit: FireballHit,
        players: &[PlayerCandidate],
    ) {
        self.projectile_block_hits
            .extend(hit.block.map(|pos| (pos, hit.location, fireball.velocity)));
        let source = match fireball.owner {
            Some(Shooter::Mob(id)) => Some(id),
            _ => None,
        };
        if let FireballKind::WitherSkull { dangerous } = fireball.kind {
            if let Some(target) = hit.entity {
                self.wither_skull_strikes(target, fireball, players);
            }
            self.fireball_explosion(
                world,
                fireball.position,
                crate::fireball::SKULL_BLAST_POWER,
                fireball.owner,
                players,
                (false, dangerous),
            );
            return;
        }
        let (damage, ignite_ticks) = match fireball.kind {
            FireballKind::Large { .. } => (crate::fireball::LARGE_DAMAGE, 0),
            FireballKind::Small | FireballKind::WitherSkull { .. } => (
                crate::fireball::SMALL_DAMAGE,
                crate::fireball::SMALL_IGNITE_TICKS,
            ),
        };
        if let Some(target) = hit.entity {
            if target >= PLAYER_TARGET {
                let player = target - PLAYER_TARGET;
                if players.iter().any(|p| p.id == player && p.attackable) {
                    self.player_hits.push(PlayerHit {
                        player_id: player,
                        damage,
                        kind: PlayerHitKind::Fireball {
                            velocity: fireball.velocity,
                            ignite_ticks,
                        },
                        source,
                    });
                }
            } else {
                self.fireball_hurts_mob(
                    target,
                    damage,
                    fireball.owner,
                    fireball.velocity,
                    ignite_ticks,
                );
            }
        }
        match fireball.kind {
            FireballKind::Large { explosion_power } => self.fireball_explosion(
                world,
                fireball.position,
                explosion_power as f32,
                fireball.owner,
                players,
                (self.mob_griefing, false),
            ),
            FireballKind::Small | FireballKind::WitherSkull { .. } => {
                let griefs = !matches!(fireball.owner, Some(Shooter::Mob(_))) || self.mob_griefing;
                if let Some(pos) = hit.block.filter(|_| griefs) {
                    let beside = struck_side(world, pos, hit.location, fireball.velocity);
                    self.fireball_fires.push(beside);
                }
            }
        }
    }

    fn wither_skull_strikes(
        &mut self,
        target: u64,
        fireball: &Fireball,
        players: &[PlayerCandidate],
    ) {
        let owner = match fireball.owner {
            Some(Shooter::Mob(id)) if self.entity_type(id).is_some() => Some(id),
            _ => None,
        };
        let wither_ticks = crate::fireball::skull_wither_ticks(self.difficulty);
        let damage = if owner.is_some() {
            crate::fireball::WITHER_SKULL_DAMAGE
        } else {
            crate::fireball::OWNERLESS_SKULL_DAMAGE
        };
        if target >= PLAYER_TARGET {
            let player = target - PLAYER_TARGET;
            if players.iter().any(|p| p.id == player && p.attackable) {
                self.player_hits.push(PlayerHit {
                    player_id: player,
                    damage,
                    kind: PlayerHitKind::WitherSkull {
                        velocity: fireball.velocity,
                        wither_ticks,
                    },
                    source: owner,
                });
            }
            return;
        }
        let result = match owner {
            Some(owner) => {
                let result = self.apply_arrow_hit(
                    ArrowImpact {
                        target_id: target,
                        damage,
                        velocity: fireball.velocity,
                    },
                    owner,
                );
                if let Some(result) = result {
                    let kind = self.entity_type(owner).unwrap_or(crate::wither::TYPE_ID);
                    self.credit(target, result, kind, false);
                }
                result
            }
            None => self.hurt_mob_by_magic(target, damage, None),
        };
        let Some(result) = result.filter(|r| r.applied) else {
            return;
        };
        if result.died {
            if let Some(e) = owner.and_then(|owner| self.withers.iter_mut().find(|e| e.id == owner))
            {
                e.wither.heal(crate::fireball::SKULL_KILL_HEAL);
            }
        } else if wither_ticks > 0 {
            self.add_mob_effect(
                target,
                crate::effects::EffectInstance::new(MobEffect::Wither, wither_ticks as i32, 1),
            );
        }
    }

    fn fireball_hurts_mob(
        &mut self,
        victim: u64,
        damage: f32,
        owner: Option<Shooter>,
        velocity: DVec3,
        ignite_ticks: i32,
    ) -> Option<DamageResult> {
        let before = self.body_mut(victim).map(|body| body.fire_ticks);
        if let Some(body) = self.body_mut(victim).filter(|_| ignite_ticks > 0) {
            body.fire_ticks = body.fire_ticks.max(ignite_ticks);
        }
        let result = self.fireball_damages_mob(victim, damage, owner, velocity);
        if !result.is_some_and(|r| r.applied) {
            if let (Some(before), Some(body)) = (before, self.body_mut(victim)) {
                body.fire_ticks = before;
            }
        }
        result
    }

    fn fireball_damages_mob(
        &mut self,
        victim: u64,
        damage: f32,
        owner: Option<Shooter>,
        velocity: DVec3,
    ) -> Option<DamageResult> {
        let kind = self.entity_type(victim)?;
        let reflected = kind == "minecraft:ghast" && matches!(owner, Some(Shooter::Player(_)));
        if (fire_immune(kind) || self.families.traits(victim).is_some_and(|t| t.fire_immune)) && !reflected {
            return None;
        }
        let damage = if reflected {
            crate::ghast::REFLECTED_FIREBALL_DAMAGE
        } else {
            damage
        };
        let attacker = match owner {
            Some(Shooter::Mob(id)) => id,
            Some(Shooter::Player(player)) => PLAYER_TARGET + player,
            None => victim,
        };
        let result = self.apply_arrow_hit(
            ArrowImpact {
                target_id: victim,
                damage,
                velocity,
            },
            attacker,
        )?;
        match owner {
            Some(Shooter::Mob(id)) => {
                let kind = self.entity_type(id).unwrap_or("minecraft:fireball");
                self.credit(victim, result, kind, false);
            }
            Some(Shooter::Player(player)) => {
                if let Some(key) = self.order.iter().copied().find(|key| key.id() == victim) {
                    if result.applied {
                        self.blasted_by_player(key, player);
                    }
                }
                self.credit(victim, result, "minecraft:player", false);
            }
            None => {}
        }
        Some(result)
    }

    fn fireball_explosion(
        &mut self,
        world: &impl World,
        center: DVec3,
        power: f32,
        owner: Option<Shooter>,
        players: &[PlayerCandidate],
        (fire, dangerous): (bool, bool),
    ) {
        let reach = power * 2.0_f32;
        if reach >= 1.0e-5_f32 {
            let reach = f64::from(reach);
            let blaster = match owner {
                Some(Shooter::Mob(id)) => Blaster::Mob(id),
                Some(Shooter::Player(player)) => Blaster::Player(player),
                None => Blaster::Nobody,
            };
            for key in self.order.clone() {
                let Some((body, eye_height)) = self.blast_target(key) else {
                    continue;
                };
                let distance = body.position.distance(center) / reach;
                if distance > 1.0 {
                    continue;
                }
                let (direction, exposure) = explosion_exposure(world, &body, eye_height, center);
                let impact = (1.0 - distance) * f64::from(exposure);
                let mut damage = ((impact * impact + impact) / 2.0 * 7.0 * reach + 1.0) as f32;
                if matches!(key, EntityKey::Ghast(_)) && matches!(owner, Some(Shooter::Player(_))) {
                    damage = crate::ghast::REFLECTED_FIREBALL_DAMAGE;
                }
                let Some(result) = self.hurt_by_blast(key, damage, direction * impact, blaster)
                else {
                    continue;
                };
                if let (true, Some(attacker)) = (result.applied, blaster.wolf_attacker()) {
                    self.note_hoglin_hurt(key.id(), attacker);
                }
                match owner {
                    Some(Shooter::Mob(id)) => {
                        let kind = self.entity_type(id).unwrap_or("minecraft:fireball");
                        self.credit(key.id(), result, kind, false);
                    }
                    Some(Shooter::Player(player)) => {
                        if result.applied {
                            self.blasted_by_player(key, player);
                        }
                        self.credit(key.id(), result, "minecraft:player", false);
                    }
                    None => {}
                }
            }
            if self.players_pickable {
                let source = match owner {
                    Some(Shooter::Mob(id)) => Some(id),
                    _ => None,
                };
                for player in players.iter().filter(|p| p.alive && !p.spectator) {
                    let height = match player.eye_height {
                        e if e >= 1.5 => 1.8,
                        e if e >= 1.0 => 1.5,
                        _ => 0.6,
                    };
                    let mut body = Body::new(player.position, 0.6, height);
                    body.on_ground = true;
                    let distance = body.position.distance(center) / reach;
                    if distance > 1.0 {
                        continue;
                    }
                    let (direction, exposure) =
                        explosion_exposure(world, &body, player.eye_height, center);
                    let impact = (1.0 - distance) * f64::from(exposure);
                    let damage = ((impact * impact + impact) / 2.0 * 7.0 * reach + 1.0) as f32;
                    self.player_hits.push(PlayerHit {
                        player_id: player.id,
                        damage,
                        kind: PlayerHitKind::Explosion {
                            knockback: direction * impact,
                            by: match owner {
                                Some(Shooter::Player(player)) => Some(player),
                                _ => None,
                            },
                        },
                        source,
                    });
                }
            }
        }
        self.fireball_blasts.push(FireballBlast {
            position: center,
            power,
            fire,
            dangerous,
        });
    }
}

fn struck_side(
    world: &impl World,
    pos: (i32, i32, i32),
    at: DVec3,
    velocity: DVec3,
) -> (i32, i32, i32) {
    let local = at - DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    let step = |axis: usize, by: i32| match axis {
        0 => (pos.0 + by, pos.1, pos.2),
        1 => (pos.0, pos.1 + by, pos.2),
        _ => (pos.0, pos.1, pos.2 + by),
    };
    for b in world.collision_boxes(pos) {
        for axis in 0..3 {
            if velocity[axis] > 0.0 && (local[axis] - b[axis]).abs() < 1.0e-4 {
                return step(axis, -1);
            }
            if velocity[axis] < 0.0 && (local[axis] - b[axis + 3]).abs() < 1.0e-4 {
                return step(axis, 1);
            }
        }
    }
    let back = at - crate::fireball::normalize(velocity) * 1.0e-3;
    (
        back.x.floor() as i32,
        back.y.floor() as i32,
        back.z.floor() as i32,
    )
}
