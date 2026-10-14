use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::ravager::{self as rv, Ravager};
use minecraftoss_entities::world::{EntityWorld, RavagerEntity};
use minecraftoss_player::rng::LegacyRandom;

pub struct Saved {
    pub pos: DVec3,
    pub yaw: f32,
    pub health: Option<f32>,
    pub persistent: bool,
    pub no_ai: bool,
    pub random: LegacyRandom,
}

fn flag(tag: &Tag, key: &str) -> bool {
    tag.get(key)
        .and_then(Tag::as_i64)
        .is_some_and(|value| value != 0)
}

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|value| value as i32)
}

fn byte(value: bool) -> Tag {
    Tag::Byte(i8::from(value))
}

pub fn spawn(world: &mut EntityWorld, tag: &Tag, saved: Saved) -> Option<u64> {
    let mut ravager = Ravager::new(saved.pos);
    ravager.yaw = saved.yaw;
    ravager.persistence_required = saved.persistent;
    ravager.left_handed = flag(tag, "LeftHanded");
    if let Some(health) = saved.health {
        ravager.health = health.min(rv::MAX_HEALTH);
    }
    ravager.attack_tick = int(tag, "AttackTick").unwrap_or(0);
    ravager.stunned_tick = int(tag, "StunTick").unwrap_or(0);
    ravager.roar_tick = int(tag, "RoarTick").unwrap_or(0);
    let raider = &mut ravager.raider;
    raider.wave = int(tag, "Wave").unwrap_or(0);
    raider.can_join_raid = flag(tag, "CanJoinRaid");
    raider.raid_id = int(tag, "RaidId");
    raider.patrol_target = tag
        .get("patrol_target")
        .and_then(Tag::as_ints)
        .and_then(|ints| <[i32; 3]>::try_from(ints).ok())
        .map(|[x, y, z]| (x, y, z));
    raider.patrol_leader = flag(tag, "PatrolLeader");
    raider.patrolling = flag(tag, "Patrolling");
    let id = world.spawn_ravager(ravager, saved.no_ai);
    let entity = world.ravager_mut(id)?;
    entity.random = saved.random;
    entity.ai.yaw = saved.yaw;
    entity.ai.state.look_control.head_yaw = saved.yaw;
    entity.ai.body_rotation.body_yaw = saved.yaw;
    Some(id)
}

pub fn fields(entity: &RavagerEntity) -> Vec<(&'static str, Tag)> {
    let ravager = &entity.ravager;
    let raider = &ravager.raider;
    let mut fields = vec![
        ("LeftHanded", byte(ravager.left_handed)),
        ("AttackTick", Tag::Int(ravager.attack_tick)),
        ("StunTick", Tag::Int(ravager.stunned_tick)),
        ("RoarTick", Tag::Int(ravager.roar_tick)),
        ("Wave", Tag::Int(raider.wave)),
        ("CanJoinRaid", byte(raider.can_join_raid)),
        ("PatrolLeader", byte(raider.patrol_leader)),
        ("Patrolling", byte(raider.patrolling)),
    ];
    if let Some(raid) = raider.raid_id {
        fields.push(("RaidId", Tag::Int(raid)));
    }
    if let Some((x, y, z)) = raider.patrol_target {
        fields.push(("patrol_target", Tag::IntArray(vec![x, y, z])));
    }
    fields
}
