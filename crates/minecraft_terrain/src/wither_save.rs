use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::wither::{self as wt, Wither};
use minecraftoss_entities::world::{EntityWorld, WitherEntity};
use minecraftoss_player::rng::LegacyRandom;

pub struct Saved {
    pub pos: DVec3,
    pub yaw: f32,
    pub health: Option<f32>,
    pub persistent: bool,
    pub no_ai: bool,
    pub random: LegacyRandom,
}

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|value| value as i32)
}

pub fn spawn(world: &mut EntityWorld, tag: &Tag, saved: Saved) -> Option<u64> {
    let mut wither = Wither::new(saved.pos);
    wither.yaw = saved.yaw;
    wither.body_yaw = saved.yaw;
    wither.persistence_required = saved.persistent;
    if let Some(health) = saved.health {
        wither.health = health.min(wt::MAX_HEALTH);
    }
    wither.invulnerable_ticks = int(tag, "Invul").unwrap_or(0).max(0);
    let id = world.spawn_wither(wither, saved.no_ai);
    let entity = world.wither_mut(id)?;
    entity.random = saved.random;
    entity.ai.yaw = saved.yaw;
    entity.ai.state.look_control.head_yaw = saved.yaw;
    entity.ai.body_rotation.body_yaw = saved.yaw;
    Some(id)
}

pub fn fields(entity: &WitherEntity) -> Vec<(&'static str, Tag)> {
    vec![("Invul", Tag::Int(entity.wither.invulnerable_ticks))]
}

pub const EXTENDED_LIFETIME_AGE: i32 = -6000;

pub fn skull_kind(tag: &Tag) -> minecraftoss_entities::fireball::FireballKind {
    minecraftoss_entities::fireball::FireballKind::WitherSkull {
        dangerous: int(tag, "dangerous").unwrap_or(0) != 0,
    }
}

pub fn rotation(tag: &Tag) -> (f32, f32) {
    let list = tag.get("Rotation").and_then(Tag::as_list);
    let angle = |i: usize| {
        list.and_then(|l| l.get(i))
            .and_then(Tag::as_f64)
            .unwrap_or(0.0) as f32
    };
    (angle(0), angle(1))
}

pub fn extend_lifetime(level: &mut minecraftoss_world::level::Level, id: i32) {
    let item = level.entities.iter_mut().find(|e| e.id == id);
    if let Some(minecraftoss_world::level::entity::EntityKind::Item(data)) =
        item.map(|e| &mut e.kind)
    {
        data.age = EXTENDED_LIFETIME_AGE;
    }
}
