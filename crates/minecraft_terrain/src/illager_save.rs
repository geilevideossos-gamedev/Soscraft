use crate::item_nbt::stack_tag;
use crate::piglin_save::stack_of;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::illager::{self as il, Illager, IllagerKind, SLOT_NAMES};
use minecraftoss_entities::world::{EntityWorld, IllagerEntity};
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

fn flag(tag: &Tag, key: &str, default: bool) -> bool {
    tag.get(key)
        .and_then(Tag::as_i64)
        .map_or(default, |value| value != 0)
}

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|value| value as i32)
}

fn byte(value: bool) -> Tag {
    Tag::Byte(i8::from(value))
}

fn custom_name(tag: &Tag) -> Option<String> {
    match tag.get("CustomName")? {
        Tag::String(name) => Some(name.clone()),
        name => name.get("text").and_then(Tag::as_str).map(str::to_owned),
    }
}

pub fn read_raider(tag: &Tag) -> il::Raider {
    il::Raider {
        wave: int(tag, "Wave").unwrap_or(0),
        can_join_raid: flag(tag, "CanJoinRaid", false),
        celebrating: false,
        ticks_outside_raid: 0,
        raid_id: int(tag, "RaidId"),
        patrol_target: tag
            .get("patrol_target")
            .and_then(Tag::as_ints)
            .and_then(|ints| <[i32; 3]>::try_from(ints).ok())
            .map(|[x, y, z]| (x, y, z)),
        patrol_leader: flag(tag, "PatrolLeader", false),
        patrolling: flag(tag, "Patrolling", false),
    }
}

pub fn raider_fields(raider: &il::Raider) -> Vec<(&'static str, Tag)> {
    let mut fields = vec![
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

pub fn spawn(world: &mut EntityWorld, tag: &Tag, kind: IllagerKind, saved: Saved) -> Option<u64> {
    let mut illager = Illager::new(kind, saved.pos);
    illager.yaw = saved.yaw;
    illager.persistence_required = saved.persistent;
    illager.left_handed = flag(tag, "LeftHanded", false);
    illager.can_pick_up_loot = kind == IllagerKind::Pillager || flag(tag, "CanPickUpLoot", false);
    if let Some(health) = saved.health {
        illager.health = health.min(il::MAX_HEALTH);
    }
    if let Some(equipment) = tag.get("equipment") {
        for (slot, name) in SLOT_NAMES.iter().enumerate() {
            illager.equipment[slot] = equipment.get(name).and_then(stack_of);
        }
    }
    if let Some(chances) = tag.get("drop_chances") {
        for (slot, name) in SLOT_NAMES.iter().enumerate() {
            if let Some(chance) = chances.get(name).and_then(Tag::as_f64) {
                illager.drop_chances[slot] = chance as f32;
            }
        }
    }
    if kind == IllagerKind::Pillager {
        let items = tag
            .get("Inventory")
            .and_then(Tag::as_list)
            .unwrap_or(&[])
            .iter()
            .filter_map(stack_of);
        for (cell, stack) in illager.inventory.iter_mut().zip(items) {
            *cell = Some(stack);
        }
    }
    illager.raider = read_raider(tag);
    illager.johnny = flag(tag, "Johnny", false);
    if kind == IllagerKind::Evoker {
        illager.spell_casting_ticks = int(tag, "SpellTicks").unwrap_or(0);
    }
    if let Some(name) = custom_name(tag) {
        illager.set_custom_name(&name);
    }
    let id = world.spawn_illager(illager, saved.no_ai);
    let entity = world.illager_mut(id)?;
    entity.random = saved.random;
    entity.ai.yaw = saved.yaw;
    entity.ai.state.look_control.head_yaw = saved.yaw;
    entity.ai.body_rotation.body_yaw = saved.yaw;
    Some(id)
}

pub fn fields(entity: &IllagerEntity) -> Vec<(&'static str, Tag)> {
    let illager = &entity.illager;
    let mut fields = vec![
        ("CanPickUpLoot", byte(illager.can_pick_up_loot)),
        ("LeftHanded", byte(illager.left_handed)),
    ];
    fields.extend(raider_fields(&illager.raider));
    if illager.johnny {
        fields.push(("Johnny", byte(true)));
    }
    if illager.kind == IllagerKind::Evoker {
        fields.push(("SpellTicks", Tag::Int(illager.spell_casting_ticks)));
    }
    if illager.kind == IllagerKind::Pillager {
        let items = illager
            .inventory
            .iter()
            .flatten()
            .filter_map(stack_tag)
            .collect();
        fields.push(("Inventory", Tag::List(items)));
    }
    let mut equipment = BTreeMap::new();
    let mut chances = BTreeMap::new();
    for (slot, name) in SLOT_NAMES.iter().enumerate() {
        if let Some(stack) = illager.equipment[slot].as_ref().and_then(stack_tag) {
            equipment.insert((*name).to_owned(), stack);
        }
        let chance = illager.drop_chances[slot];
        if chance != il::DEFAULT_DROP_CHANCE {
            chances.insert((*name).to_owned(), Tag::Float(chance));
        }
    }
    fields.push(("equipment", Tag::Compound(equipment)));
    fields.push(("drop_chances", Tag::Compound(chances)));
    fields
}
