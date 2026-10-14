use super::*;
use crate::evoker_fangs::{self as fangs, EvokerFangs};

#[derive(Clone)]
pub struct EvokerFangsEntity {
    pub id: u64,
    pub fangs: EvokerFangs,
}

fn boxes_meet(min: DVec3, max: DVec3, low: DVec3, high: DVec3) -> bool {
    min.x < high.x
        && max.x > low.x
        && min.y < high.y
        && max.y > low.y
        && min.z < high.z
        && max.z > low.z
}

impl EntityWorld {
    pub fn spawn_evoker_fangs(&mut self, fangs: EvokerFangs) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.evoker_fangs.push(EvokerFangsEntity { id, fangs });
        self.order.push(EntityKey::EvokerFangs(id));
        id
    }

    pub fn evoker_fangs(&self) -> &[EvokerFangsEntity] {
        &self.evoker_fangs
    }

    pub fn evoker_fangs_mut(&mut self, id: u64) -> Option<&mut EvokerFangsEntity> {
        self.evoker_fangs.iter_mut().find(|e| e.id == id)
    }

    pub(super) fn evoker_fangs_owner(&self, id: u64) -> Option<u64> {
        let entity = self.evoker_fangs.iter().find(|e| e.id == id)?;
        entity
            .fangs
            .owner
            .or_else(|| {
                entity
                    .fangs
                    .owner_uuid
                    .and_then(|uuid| self.id_of_uuid(uuid))
            })
            .filter(|&owner| self.entity_type(owner).is_some())
    }

    pub(super) fn fangs_spare(&self, owner: Option<u64>, victim: u64, kind: &str) -> bool {
        let Some(owner) = owner else {
            return false;
        };
        if victim == owner {
            return true;
        }
        let owner_kind = self.entity_type(owner);
        owner_kind.is_some_and(crate::illager::is_illager) && crate::illager::is_illager(kind)
    }

    pub(super) fn tick_evoker_fangs(&mut self, id: u64, players: &[PlayerCandidate]) {
        let owner = self.evoker_fangs_owner(id);
        let Some(entity) = self
            .evoker_fangs
            .iter_mut()
            .find(|e| e.id == id && e.fangs.alive)
        else {
            return;
        };
        if owner.is_some() {
            entity.fangs.owner = owner;
        }
        let step = entity.fangs.tick();
        let position = entity.fangs.position;
        let (min, max) = entity.fangs.bite_box();
        if step.bite {
            for player in players.iter().filter(|p| p.alive) {
                let p = player.position;
                let (low, high) = (
                    DVec3::new(p.x - 0.3, p.y, p.z - 0.3),
                    DVec3::new(p.x + 0.3, p.y + 1.8, p.z + 0.3),
                );
                if boxes_meet(min, max, low, high) {
                    self.player_hits.push(PlayerHit {
                        player_id: player.id,
                        damage: fangs::DAMAGE,
                        kind: PlayerHitKind::Magic { at: position },
                        source: owner,
                    });
                }
            }
            let victims: Vec<u64> = self
                .mob_candidates()
                .into_iter()
                .filter(|m| m.alive && !self.fangs_spare(owner, m.id, m.kind))
                .filter(|m| {
                    let half = f64::from(m.width / 2.0);
                    let q = m.position;
                    let (low, high) = (
                        DVec3::new(q.x - half, q.y, q.z - half),
                        DVec3::new(q.x + half, q.y + f64::from(m.height), q.z + half),
                    );
                    boxes_meet(min, max, low, high)
                })
                .map(|m| m.id)
                .collect();
            for victim in victims {
                let _ = self.hurt_mob_by_magic(victim, fangs::DAMAGE, owner);
            }
        }
        if step.spike {
            let pitch = self.level_random.next_float() * 0.2 + 0.85;
            self.sounds.push(MobSound {
                event: fangs::ATTACK_SOUND.to_owned(),
                position,
                volume: 1.0,
                pitch,
                category: "neutral_volume",
            });
        }
    }

    pub(super) fn cast_fangs(&mut self, caster: u64, world: &dyn World, target: DVec3) -> usize {
        let Some(position) = self
            .illagers
            .iter()
            .find(|e| e.id == caster)
            .map(|e| e.illager.body.position)
        else {
            return 0;
        };
        let min_y = target.y.min(position.y);
        let max_y = target.y.max(position.y) + 1.0;
        let mut made = 0;
        for spot in fangs::spots(position, target) {
            let Some(y) = fangs::floor_at(world, spot.x, spot.z, min_y, max_y) else {
                continue;
            };
            let at = DVec3::new(spot.x, y, spot.z);
            self.spawn_evoker_fangs(EvokerFangs::new(at, spot.yaw, spot.warmup, Some(caster)));
            made += 1;
        }
        made
    }
}
