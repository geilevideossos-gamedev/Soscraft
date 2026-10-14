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
