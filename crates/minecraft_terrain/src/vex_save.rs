use crate::item_nbt::stack_tag;
use crate::piglin_save::stack_of;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::vex::{self as vx, SLOT_NAMES, Vex};
use minecraftoss_entities::world::{EntityWorld, VexEntity};
use minecraftoss_player::rng::LegacyRandom;
use std::collections::BTreeMap;

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

fn uuid(tag: &Tag, key: &str) -> Option<u128> {
    tag.get(key)
        .and_then(Tag::as_ints)
        .and_then(|ints| <[i32; 4]>::try_from(ints).ok())
        .map(minecraftoss_entities::gossip::uuid_from_ints)
}

pub fn spawn(world: &mut EntityWorld, tag: &Tag, saved: Saved) -> Option<u64> {
    let mut vex = Vex::new(saved.pos);
    vex.yaw = saved.yaw;
    vex.persistence_required = saved.persistent;
    vex.left_handed = flag(tag, "LeftHanded");
    if let Some(health) = saved.health {
        vex.health = health.min(vx::MAX_HEALTH);
    }
    if let Some(equipment) = tag.get("equipment") {
        for (slot, name) in SLOT_NAMES.iter().enumerate() {
            vex.equipment[slot] = equipment.get(name).and_then(stack_of);
        }
    }
    if let Some(chances) = tag.get("drop_chances") {
        for (slot, name) in SLOT_NAMES.iter().enumerate() {
            if let Some(chance) = chances.get(name).and_then(Tag::as_f64) {
                vex.drop_chances[slot] = chance as f32;
            }
        }
    }
    vex.bound_origin = tag
        .get("bound_pos")
        .and_then(Tag::as_ints)
        .and_then(|ints| <[i32; 3]>::try_from(ints).ok())
        .map(|[x, y, z]| (x, y, z));
    vex.life_ticks = tag
        .get("life_ticks")
        .and_then(Tag::as_i64)
        .map(|ticks| ticks as i32);
    vex.owner_uuid = uuid(tag, "owner");
    let id = world.spawn_vex(vex, saved.no_ai);
    let entity = world.vex_mut(id)?;
    entity.random = saved.random;
    entity.ai.yaw = saved.yaw;
    entity.ai.state.look_control.head_yaw = saved.yaw;
    entity.ai.body_rotation.body_yaw = saved.yaw;
    Some(id)
}

pub fn fields(world: &EntityWorld, entity: &VexEntity) -> Vec<(&'static str, Tag)> {
    let vex = &entity.vex;
    let mut fields = vec![("LeftHanded", Tag::Byte(i8::from(vex.left_handed)))];
    if let Some((x, y, z)) = vex.bound_origin {
        fields.push(("bound_pos", Tag::IntArray(vec![x, y, z])));
    }
    if let Some(ticks) = vex.life_ticks {
        fields.push(("life_ticks", Tag::Int(ticks)));
    }
    if let Some(owner) = vex.owner.map(|o| world.uuid_of(o)).or(vex.owner_uuid) {
        let ints = minecraftoss_entities::gossip::uuid_to_ints(owner).to_vec();
        fields.push(("owner", Tag::IntArray(ints)));
    }
    let mut equipment = BTreeMap::new();
    let mut chances = BTreeMap::new();
    for (slot, name) in SLOT_NAMES.iter().enumerate() {
        if let Some(stack) = vex.equipment[slot].as_ref().and_then(stack_tag) {
            equipment.insert((*name).to_owned(), stack);
        }
        let chance = vex.drop_chances[slot];
        if chance != vx::DEFAULT_DROP_CHANCE {
            chances.insert((*name).to_owned(), Tag::Float(chance));
        }
    }
    fields.push(("equipment", Tag::Compound(equipment)));
    fields.push(("drop_chances", Tag::Compound(chances)));
    fields
}
