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

    pub fn take_fireball_blasts(&mut self) -> Vec<FireballBlast> {
        std::mem::take(&mut self.fireball_blasts)
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
                Some(FireballTarget {
                    id,
                    min: DVec3::new(p.x - half, p.y, p.z - half),
                    max: DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
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

    pub(super) fn tick_fireball(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
    ) {
        let Some(owner) = self
            .fireballs
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.fireball.owner)
        else {
            return;
        };
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
        let source = match fireball.owner {
            Some(Shooter::Mob(id)) => Some(id),
            _ => None,
        };
        let FireballKind::Large { explosion_power } = fireball.kind;
        if let Some(target) = hit.entity {
            if target >= PLAYER_TARGET {
                let player = target - PLAYER_TARGET;
                if players.iter().any(|p| p.id == player && p.attackable) {
                    self.player_hits.push(PlayerHit {
                        player_id: player,
                        damage: crate::fireball::LARGE_DAMAGE,
                        kind: PlayerHitKind::Fireball {
                            velocity: fireball.velocity,
                            ignite_ticks: 0,
                        },
                        source,
                    });
                }
            } else {
                self.fireball_hurts_mob(
                    target,
                    crate::fireball::LARGE_DAMAGE,
                    fireball.owner,
                    fireball.velocity,
                );
            }
        }
        self.fireball_explosion(
            world,
            fireball.position,
            explosion_power as f32,
            fireball.owner,
            players,
        );
    }

    fn fireball_hurts_mob(
        &mut self,
        victim: u64,
        damage: f32,
        owner: Option<Shooter>,
        velocity: DVec3,
    ) -> Option<DamageResult> {
        let kind = self.entity_type(victim)?;
        let reflected = kind == "minecraft:ghast" && matches!(owner, Some(Shooter::Player(_)));
        if fire_immune(kind) && !reflected {
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
                        },
                        source,
                    });
                }
            }
        }
        self.fireball_blasts.push(FireballBlast {
            position: center,
            power,
            fire: self.mob_griefing,
        });
    }
}
