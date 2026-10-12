use crate::item_nbt::{saved_components, stack_tag, tag_json};
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::gossip::{uuid_from_ints, uuid_to_ints};
use minecraftoss_entities::piglin::{self, Piglin, PiglinKind, SLOT_NAMES};
use minecraftoss_entities::piglin_brain::{GlobalPos, Memories};
use minecraftoss_entities::villager_brain::Slot;
use minecraftoss_entities::world::{EntityWorld, PiglinConversion, PiglinEntity};
use minecraftoss_player::inventory::ItemStack;
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

const HUNTED_RECENTLY: &str = "minecraft:hunted_recently";
const ANGRY_AT: &str = "minecraft:angry_at";
const UNIVERSAL_ANGER: &str = "minecraft:universal_anger";
const ADMIRING_ITEM: &str = "minecraft:admiring_item";
const ADMIRING_DISABLED: &str = "minecraft:admiring_disabled";
const HOME: &str = "minecraft:home";

fn flag(tag: &Tag, key: &str, default: bool) -> bool {
    tag.get(key)
        .and_then(Tag::as_i64)
        .map_or(default, |value| value != 0)
}

fn byte(value: bool) -> Tag {
    Tag::Byte(i8::from(value))
}

pub fn stack_of(tag: &Tag) -> Option<ItemStack> {
    let id = tag.get("id")?.as_str()?;
    let count = tag.get("count").map_or(Some(1), Tag::as_i64)?;
    if count <= 0 {
        return None;
    }
    let mut stack = ItemStack::new(id, count.min(99) as u8);
    if piglin::max_damage(id).is_some()
        || piglin::armor_value(id).0 > 0.0
        || piglin::melee_weapon(id)
    {
        stack.max = 1;
    }
    stack.components = tag
        .get("components")
        .and_then(saved_components)
        .map(|components| tag_json(&components));
    Some(stack)
}

fn read_flag(memories: &Tag, key: &str, slot: &mut Slot<()>) {
    let Some(memory) = memories.get(key) else {
        return;
    };
    if memory.get("value").and_then(Tag::as_i64) != Some(1) {
        return;
    }
    match memory.get("ttl").and_then(Tag::as_i64) {
        Some(ttl) => slot.set_for((), ttl),
        None => slot.set(()),
    }
}

fn read_memories(tag: &Tag, memories: &mut Memories) {
    let Some(saved) = tag.get("Brain").and_then(|brain| brain.get("memories")) else {
        return;
    };
    read_flag(saved, HUNTED_RECENTLY, &mut memories.hunted_recently);
    read_flag(saved, UNIVERSAL_ANGER, &mut memories.universal_anger);
    read_flag(saved, ADMIRING_ITEM, &mut memories.admiring_item);
    read_flag(saved, ADMIRING_DISABLED, &mut memories.admiring_disabled);
    if let Some(memory) = saved.get(ANGRY_AT) {
        let uuid = memory
            .get("value")
            .and_then(Tag::as_ints)
            .and_then(|ints| <[i32; 4]>::try_from(ints).ok())
            .map(uuid_from_ints);
        if let Some(uuid) = uuid {
            match memory.get("ttl").and_then(Tag::as_i64) {
                Some(ttl) => memories.angry_at.set_for(uuid, ttl),
                None => memories.angry_at.set(uuid),
            }
        }
    }
    if let Some(value) = saved.get(HOME).and_then(|memory| memory.get("value")) {
        let dimension = value.get("dimension").and_then(Tag::as_str);
        let pos = value.get("pos").and_then(Tag::as_ints);
        if let (Some(dimension), Some(pos)) = (dimension, pos) {
            if let [x, y, z] = pos[..] {
                memories.home.set(GlobalPos {
                    dimension: dimension.to_owned(),
                    pos: (x, y, z),
                });
            }
        }
    }
}

fn memory_tag(value: Tag, ttl: Option<i64>) -> Tag {
    let mut map = BTreeMap::new();
    map.insert("value".to_owned(), value);
    if let Some(ttl) = ttl {
        map.insert("ttl".to_owned(), Tag::Long(ttl));
    }
    Tag::Compound(map)
}

fn brain_tag(memories: &Memories) -> Tag {
    let mut saved = BTreeMap::new();
    for (key, slot) in [
        (HUNTED_RECENTLY, &memories.hunted_recently),
        (UNIVERSAL_ANGER, &memories.universal_anger),
        (ADMIRING_ITEM, &memories.admiring_item),
        (ADMIRING_DISABLED, &memories.admiring_disabled),
    ] {
        if slot.present() {
            saved.insert(key.to_owned(), memory_tag(Tag::Byte(1), slot.ttl()));
        }
    }
    if let Some(&uuid) = memories.angry_at.get() {
        let value = Tag::IntArray(uuid_to_ints(uuid).to_vec());
        saved.insert(
            ANGRY_AT.to_owned(),
            memory_tag(value, memories.angry_at.ttl()),
        );
    }
    if let Some(home) = memories.home.get() {
        let (x, y, z) = home.pos;
        let mut value = BTreeMap::new();
        value.insert("dimension".to_owned(), Tag::String(home.dimension.clone()));
        value.insert("pos".to_owned(), Tag::IntArray(vec![x, y, z]));
        saved.insert(
            HOME.to_owned(),
            memory_tag(Tag::Compound(value), memories.home.ttl()),
        );
    }
    let mut brain = BTreeMap::new();
    brain.insert("memories".to_owned(), Tag::Compound(saved));
    Tag::Compound(brain)
}

pub fn spawn(world: &mut EntityWorld, tag: &Tag, kind: PiglinKind, saved: Saved) -> Option<u64> {
    let mut piglin = Piglin::new(kind, saved.pos);
    piglin.yaw = saved.yaw;
    piglin.persistence_required = saved.persistent;
    piglin.set_baby(flag(tag, "IsBaby", false));
    piglin.cannot_hunt = flag(tag, "CannotHunt", false);
    piglin.immune_to_zombification = flag(tag, "IsImmuneToZombification", false);
    piglin.time_in_overworld = tag
        .get("TimeInOverworld")
        .and_then(Tag::as_i64)
        .unwrap_or(0) as i32;
    piglin.can_pick_up_loot = flag(tag, "CanPickUpLoot", true);
    piglin.left_handed = flag(tag, "LeftHanded", false);
    if let Some(health) = saved.health {
        piglin.health = health;
    }
    if let Some(equipment) = tag.get("equipment") {
        for (slot, name) in SLOT_NAMES.iter().enumerate() {
            if let Some(cell) = piglin.equipment_mut(slot) {
                *cell = equipment.get(name).and_then(stack_of);
            }
        }
    }
    if let Some(chances) = tag.get("drop_chances") {
        for (slot, name) in SLOT_NAMES.iter().enumerate() {
            if let Some(chance) = chances.get(name).and_then(Tag::as_f64) {
                piglin.drop_chances[slot] = chance as f32;
            }
        }
    }
    if kind == PiglinKind::Piglin {
        let items = tag
            .get("Inventory")
            .and_then(Tag::as_list)
            .unwrap_or(&[])
            .iter()
            .filter_map(stack_of);
        for (cell, stack) in piglin.inventory.iter_mut().zip(items) {
            *cell = Some(stack);
        }
    }
    let id = world.spawn_piglin(piglin, saved.no_ai);
    let entity = world.piglin_mut(id)?;
    entity.random = saved.random;
    entity.look_control.head_yaw = saved.yaw;
    entity.body_rotation.body_yaw = saved.yaw;
    read_memories(tag, &mut entity.brain.memories);
    Some(id)
}

pub fn fields(entity: &PiglinEntity) -> Vec<(&'static str, Tag)> {
    let piglin = &entity.piglin;
    let mut fields = vec![
        (
            "IsImmuneToZombification",
            byte(piglin.immune_to_zombification),
        ),
        ("TimeInOverworld", Tag::Int(piglin.time_in_overworld)),
        ("CanPickUpLoot", byte(piglin.can_pick_up_loot)),
        ("LeftHanded", byte(piglin.left_handed)),
    ];
    if piglin.kind == PiglinKind::Piglin {
        fields.push(("IsBaby", byte(piglin.baby)));
        fields.push(("CannotHunt", byte(piglin.cannot_hunt)));
        let items = piglin
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
        if let Some(stack) = piglin.equipment(slot).and_then(stack_tag) {
            equipment.insert((*name).to_owned(), stack);
        }
        let chance = piglin.drop_chances[slot];
        if chance != piglin::DEFAULT_DROP_CHANCE {
            chances.insert((*name).to_owned(), Tag::Float(chance));
        }
    }
    fields.push(("equipment", Tag::Compound(equipment)));
    fields.push(("drop_chances", Tag::Compound(chances)));
    fields.push(("Brain", brain_tag(&entity.brain.memories)));
    fields
}

pub fn converted_tag(conversion: &PiglinConversion) -> Tag {
    let mut equipment = BTreeMap::new();
    let mut chances = BTreeMap::new();
    for (slot, name) in SLOT_NAMES.iter().enumerate() {
        if let Some(stack) = conversion.equipment[slot].as_ref().and_then(stack_tag) {
            equipment.insert((*name).to_owned(), stack);
        }
        let chance = conversion.drop_chances[slot];
        if chance != piglin::DEFAULT_DROP_CHANCE {
            chances.insert((*name).to_owned(), Tag::Float(chance));
        }
    }
    let mut map = BTreeMap::new();
    map.insert("equipment".to_owned(), Tag::Compound(equipment));
    map.insert("drop_chances".to_owned(), Tag::Compound(chances));
    map.insert(
        "CanPickUpLoot".to_owned(),
        byte(conversion.can_pick_up_loot),
    );
    map.insert("LeftHanded".to_owned(), byte(conversion.left_handed));
    Tag::Compound(map)
}
