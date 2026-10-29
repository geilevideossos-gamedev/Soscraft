use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use minecraftoss_player::inventory::Inventory;
use minecraftoss_player::{Block, Pos, World};
use serde_json::Value;

use crate::transform::{BlockFacts, Predicate};

const ENCHANTMENT_FOLDER: &str = "data/minecraft/enchantment/";
const DAMAGE_TAG_FOLDER: &str = "data/minecraft/tags/damage_type/";
const ENCHANTMENTS: &str = "minecraft:enchantments";
const DAMAGE_PROTECTION: &str = "minecraft:damage_protection";
const DAMAGE_IMMUNITY: &str = "minecraft:damage_immunity";
const LOCATION_CHANGED: &str = "minecraft:location_changed";
const REPLACE_DISK: &str = "minecraft:replace_disk";
const ATTRIBUTES: &str = "minecraft:attributes";
const BURNING_TIME: &str = "minecraft:burning_time";
const ADD_MULTIPLIED_BASE: &str = "add_multiplied_base";
const BYPASSES_EFFECTS: &str = "minecraft:bypasses_effects";
const BYPASSES_ENCHANTMENTS: &str = "minecraft:bypasses_enchantments";
const ARMOR_SLOTS: [(usize, &str); 4] = [(36, "feet"), (37, "legs"), (38, "chest"), (39, "head")];
const MAX_BURNING_TIME: f64 = 1024.0;
const TAG_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Linear {
    base: f32,
    per_level_above_first: f32,
}

impl Linear {
    fn parse(value: &Value) -> Option<Self> {
        if let Some(constant) = value.as_f64() {
            return Some(Self {
                base: constant as f32,
                per_level_above_first: 0.0,
            });
        }
        match value["type"].as_str()? {
            "minecraft:linear" => Some(Self {
                base: value["base"].as_f64()? as f32,
                per_level_above_first: value["per_level_above_first"].as_f64()? as f32,
            }),
            "minecraft:constant" => Some(Self {
                base: value["value"].as_f64()? as f32,
                per_level_above_first: 0.0,
            }),
            _ => None,
        }
    }

    fn at(self, level: i32) -> f32 {
        self.base + self.per_level_above_first * (level - 1) as f32
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Clamped {
    value: Linear,
    min: f32,
    max: f32,
}

impl Clamped {
    fn parse(value: &Value) -> Option<Self> {
        if value["type"].as_str() == Some("minecraft:clamped") {
            return Some(Self {
                value: Linear::parse(&value["value"])?,
                min: value["min"].as_f64()? as f32,
                max: value["max"].as_f64()? as f32,
            });
        }
        Some(Self {
            value: Linear::parse(value)?,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        })
    }

    fn at(self, level: i32) -> f32 {
        self.value.at(level).clamp(self.min, self.max)
    }
}

fn damage_tags(requirements: &Value) -> Option<Vec<(String, bool)>> {
    if requirements.is_null() {
        return Some(Vec::new());
    }
    if requirements["type"].as_str()? != "minecraft:damage_source_properties" {
        return None;
    }
    let predicate = requirements["predicate"].as_object()?;
    if predicate.keys().any(|key| key != "tags") {
        return None;
    }
    predicate["tags"]
        .as_array()?
        .iter()
        .map(|tag| {
            let id = tag["id"].as_str()?.strip_prefix('#')?.to_owned();
            Some((id, tag["expected"].as_bool()?))
        })
        .collect()
}

fn needs_ground(requirements: &Value) -> Option<bool> {
    if requirements.is_null() {
        return Some(false);
    }
    let terms: Vec<&Value> = match requirements["type"].as_str()? {
        "minecraft:all_of" => requirements["terms"].as_array()?.iter().collect(),
        _ => vec![requirements],
    };
    let mut ground = false;
    for term in terms {
        let predicate = match term["type"].as_str()? {
            "minecraft:entity_properties" => &term["predicate"],
            "minecraft:inverted" => &term["term"]["predicate"],
            _ => return None,
        };
        let predicate = predicate.as_object()?;
        if term["type"].as_str() == Some("minecraft:inverted") {
            if predicate.keys().any(|key| key != "minecraft:vehicle") {
                return None;
            }
            continue;
        }
        let flags = predicate.get("minecraft:flags")?.as_object()?;
        if predicate.len() != 1 || flags.len() != 1 || !flags.get("is_on_ground")?.as_bool()? {
            return None;
        }
        ground = true;
    }
    Some(ground)
}

#[derive(Clone, Debug, PartialEq)]
struct Disk {
    block: Block,
    radius: Clamped,
    height: Clamped,
    offset: Pos,
    predicate: Option<Predicate>,
    on_ground: bool,
}

impl Disk {
    fn parse(entry: &Value) -> Option<Self> {
        let effect = &entry["effect"];
        if effect["type"].as_str()? != REPLACE_DISK {
            return None;
        }
        let state = &effect["block_state"];
        let mut block = Block::new(state["id"].as_str()?);
        if let Some(properties) = state.get("properties") {
            for (name, value) in properties.as_object()? {
                block = block.with(name, value.as_str()?);
            }
        }
        let offset = match effect.get("offset") {
            None => (0, 0, 0),
            Some(offset) => {
                let at = |i: usize| offset.get(i)?.as_i64().and_then(|v| i32::try_from(v).ok());
                (at(0)?, at(1)?, at(2)?)
            }
        };
        let predicate = match effect.get("predicate") {
            None => None,
            Some(predicate) => Some(Predicate::parse(predicate)?),
        };
        Some(Self {
            block,
            radius: Clamped::parse(&effect["radius"])?,
            height: Clamped::parse(&effect["height"])?,
            offset,
            predicate,
            on_ground: needs_ground(&entry["requirements"])?,
        })
    }

    fn apply(
        &self,
        level: i32,
        feet: [f64; 3],
        world: &impl World,
        facts: &impl BlockFacts,
        unobstructed: &dyn Fn(Pos) -> bool,
    ) -> Vec<(Pos, Block)> {
        let base = (
            feet[0].floor() as i32 + self.offset.0,
            feet[1].floor() as i32 + self.offset.1,
            feet[2].floor() as i32 + self.offset.2,
        );
        let radius = self.radius.at(level) as i32;
        let height = self.height.at(level) as i32;
        let reach = f64::from(radius * radius);
        let mut out = Vec::new();
        for y in base.1..=base.1 + (height - 1).min(0) {
            for z in base.2 - radius..=base.2 + radius {
                for x in base.0 - radius..=base.0 + radius {
                    let (dx, dz) = (f64::from(x) + 0.5 - feet[0], f64::from(z) + 0.5 - feet[2]);
                    let pos = (x, y, z);
                    let fits = self
                        .predicate
                        .as_ref()
                        .is_none_or(|predicate| predicate.test(world, facts, pos, unobstructed));
                    if dx * dx + dz * dz < reach && fits {
                        out.push((pos, self.block.clone()));
                    }
                }
            }
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DamageProtection {
    amount: Linear,
    tags: Vec<(String, bool)>,
}

impl DamageProtection {
    fn parse(entry: &Value) -> Option<Self> {
        let effect = &entry["effect"];
        if effect["type"].as_str()? != "minecraft:add" {
            return None;
        }
        let amount = Linear::parse(&effect["value"])?;
        let tags = damage_tags(&entry["requirements"])?;
        Some(Self { amount, tags })
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProtectionBook {
    protections: HashMap<String, Vec<DamageProtection>>,
    burning: HashMap<String, Vec<Linear>>,
    immunities: HashMap<String, Vec<Vec<(String, bool)>>>,
    disks: HashMap<String, Vec<Disk>>,
    slots: HashMap<String, Vec<String>>,
    damage_tags: HashMap<String, HashSet<String>>,
}

impl ProtectionBook {
    pub fn from_jar(jar: &Path) -> Result<Self> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(jar)?)?;
        let mut enchantments = Vec::new();
        let mut tags = HashMap::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name().to_owned();
            let Some(stem) = name.strip_suffix(".json") else {
                continue;
            };
            let (folder, path) = if let Some(path) = stem.strip_prefix(ENCHANTMENT_FOLDER) {
                (true, path)
            } else if let Some(path) = stem.strip_prefix(DAMAGE_TAG_FOLDER) {
                (false, path)
            } else {
                continue;
            };
            let id = format!("minecraft:{path}");
            let mut text = String::new();
            entry.read_to_string(&mut text)?;
            let json = serde_json::from_str::<Value>(&text)?;
            if folder {
                enchantments.push((id, json));
            } else {
                tags.insert(id, json);
            }
        }
        Ok(Self::from_json(enchantments, &tags))
    }

    pub fn from_json(enchantments: Vec<(String, Value)>, tags: &HashMap<String, Value>) -> Self {
        let mut book = Self::default();
        for (id, json) in enchantments {
            let effects = &json["effects"];
            let protections: Vec<_> = effects[DAMAGE_PROTECTION]
                .as_array()
                .map(|entries| entries.iter().filter_map(DamageProtection::parse).collect())
                .unwrap_or_else(Vec::new);
            if !protections.is_empty() {
                book.protections.insert(id.clone(), protections);
            }
            let burning: Vec<_> = effects[ATTRIBUTES]
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|entry| {
                            entry["attribute"].as_str() == Some(BURNING_TIME)
                                && entry["operation"].as_str() == Some(ADD_MULTIPLIED_BASE)
                        })
                        .filter_map(|entry| Linear::parse(&entry["amount"]))
                        .collect()
                })
                .unwrap_or_else(Vec::new);
            if !burning.is_empty() {
                book.burning.insert(id.clone(), burning);
            }
            let immunities: Vec<_> = effects[DAMAGE_IMMUNITY]
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|entry| entry["effect"].as_object().is_some_and(|e| e.is_empty()))
                        .filter_map(|entry| damage_tags(&entry["requirements"]))
                        .collect()
                })
                .unwrap_or_else(Vec::new);
            if !immunities.is_empty() {
                book.immunities.insert(id.clone(), immunities);
            }
            let disks: Vec<_> = effects[LOCATION_CHANGED]
                .as_array()
                .map(|entries| entries.iter().filter_map(Disk::parse).collect())
                .unwrap_or_else(Vec::new);
            if !disks.is_empty() {
                book.disks.insert(id.clone(), disks);
            }
            if let Some(slots) = json["slots"].as_array() {
                let slots = slots
                    .iter()
                    .filter_map(|slot| slot.as_str().map(str::to_owned))
                    .collect();
                book.slots.insert(id, slots);
            }
        }
        for tag in tags.keys() {
            let mut members = HashSet::new();
            resolve(tags, tag, &mut members, 0);
            book.damage_tags.insert(tag.clone(), members);
        }
        book
    }

    fn fits_slot(&self, id: &str, slot: &str) -> bool {
        self.slots.get(id).is_none_or(|groups| {
            groups
                .iter()
                .any(|group| matches!(group.as_str(), "any" | "armor") || group == slot)
        })
    }

    fn tags_match(&self, damage_type: &str, tags: &[(String, bool)]) -> bool {
        tags.iter()
            .all(|(tag, expected)| self.in_tag(damage_type, tag) == *expected)
    }

    pub fn immune(&self, worn: &[(String, i32)], damage_type: &str) -> bool {
        worn.iter()
            .filter(|(_, level)| *level > 0)
            .filter_map(|(id, _)| self.immunities.get(id))
            .flatten()
            .any(|tags| self.tags_match(damage_type, tags))
    }

    pub fn location_changed(
        &self,
        worn: &[(String, i32)],
        feet: [f64; 3],
        on_ground: bool,
        world: &impl World,
        facts: &impl BlockFacts,
        unobstructed: &dyn Fn(Pos) -> bool,
    ) -> Vec<(Pos, Block)> {
        worn.iter()
            .filter(|(_, level)| *level > 0)
            .flat_map(|(id, level)| {
                self.disks
                    .get(id)
                    .into_iter()
                    .flatten()
                    .map(move |disk| (disk, *level))
            })
            .filter(|(disk, _)| on_ground || !disk.on_ground)
            .flat_map(|(disk, level)| disk.apply(level, feet, world, facts, unobstructed))
            .collect()
    }

    fn in_tag(&self, damage_type: &str, tag: &str) -> bool {
        self.damage_tags
            .get(tag)
            .is_some_and(|members| members.contains(damage_type))
    }

    pub fn protection(&self, worn: &[(String, i32)], damage_type: &str) -> f32 {
        if self.in_tag(damage_type, BYPASSES_EFFECTS)
            || self.in_tag(damage_type, BYPASSES_ENCHANTMENTS)
        {
            return 0.0;
        }
        worn.iter()
            .filter(|(_, level)| *level > 0)
            .flat_map(|(id, level)| {
                self.protections
                    .get(id)
                    .into_iter()
                    .flatten()
                    .map(move |protection| (protection, *level))
            })
            .filter(|(protection, _)| self.tags_match(damage_type, &protection.tags))
            .map(|(protection, level)| protection.amount.at(level))
            .sum()
    }

    pub fn burning_time(&self, worn: &[(String, i32)]) -> f64 {
        let added: f64 = worn
            .iter()
            .filter(|(_, level)| *level > 0)
            .flat_map(|(id, level)| {
                self.burning
                    .get(id)
                    .into_iter()
                    .flatten()
                    .map(move |amount| f64::from(amount.at(*level)))
            })
            .sum();
        (1.0 + added).clamp(0.0, MAX_BURNING_TIME)
    }
}

fn resolve(tags: &HashMap<String, Value>, tag: &str, out: &mut HashSet<String>, depth: usize) {
    if depth > TAG_DEPTH {
        return;
    }
    let Some(values) = tags.get(tag).and_then(|json| json["values"].as_array()) else {
        return;
    };
    for value in values {
        let id = value
            .as_str()
            .or_else(|| value["id"].as_str())
            .unwrap_or("");
        match id.strip_prefix('#') {
            Some(nested) => resolve(tags, nested, out, depth + 1),
            None if !id.is_empty() => {
                out.insert(id.to_owned());
            }
            None => {}
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Protection {
    book: Option<Arc<ProtectionBook>>,
    worn: Vec<(String, i32)>,
}

impl Protection {
    pub fn worn(book: Option<Arc<ProtectionBook>>, inventory: &Inventory) -> Self {
        let worn = ARMOR_SLOTS
            .iter()
            .filter_map(|&(slot, name)| {
                let stack = inventory.slots.get(slot)?.as_ref()?;
                Some((
                    stack.components.as_ref()?.get(ENCHANTMENTS)?.as_object()?,
                    name,
                ))
            })
            .flat_map(|(levels, slot)| {
                let book = book.as_deref();
                levels.iter().filter_map(move |(id, level)| {
                    let id = if id.contains(':') {
                        id.clone()
                    } else {
                        format!("minecraft:{id}")
                    };
                    book.is_none_or(|book| book.fits_slot(&id, slot))
                        .then_some(())?;
                    Some((id, i32::try_from(level.as_i64()?).ok()?))
                })
            })
            .collect();
        Self { book, worn }
    }

    pub fn immune(&self, damage_type: &str) -> bool {
        self.book
            .as_ref()
            .is_some_and(|book| book.immune(&self.worn, damage_type))
    }

    pub fn location_changed(
        &self,
        feet: [f64; 3],
        on_ground: bool,
        world: &impl World,
        facts: &impl BlockFacts,
        unobstructed: &dyn Fn(Pos) -> bool,
    ) -> Vec<(Pos, Block)> {
        self.book.as_ref().map_or_else(Vec::new, |book| {
            book.location_changed(&self.worn, feet, on_ground, world, facts, unobstructed)
        })
    }

    pub fn against(&self, damage_type: &str) -> f32 {
        self.book
            .as_ref()
            .map_or(0.0, |book| book.protection(&self.worn, damage_type))
    }

    pub fn burning_ticks(&self, ticks: i32) -> i32 {
        let scale = self
            .book
            .as_ref()
            .map_or(1.0, |book| book.burning_time(&self.worn));
        (f64::from(ticks) * scale).ceil() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::inventory::ItemStack;
    use serde_json::json;

    fn protection(base: f64, tags: Value) -> Value {
        json!({
            "effect": {
                "type": "minecraft:add",
                "value": {"type": "minecraft:linear", "base": base, "per_level_above_first": base}
            },
            "requirements": {
                "type": "minecraft:damage_source_properties",
                "predicate": {"tags": tags}
            }
        })
    }

    fn book() -> ProtectionBook {
        let not_bypassing = json!({"expected": false, "id": "#minecraft:bypasses_invulnerability"});
        let tagged = |tag: &str| json!({"expected": true, "id": tag});
        let enchantments = vec![
            (
                "minecraft:protection".to_owned(),
                json!({"effects": {"minecraft:damage_protection": [protection(1.0, json!([not_bypassing]))]}}),
            ),
            (
                "minecraft:fire_protection".to_owned(),
                json!({"effects": {
                    "minecraft:attributes": [{
                        "amount": {"type": "minecraft:linear", "base": -0.15, "per_level_above_first": -0.15},
                        "attribute": "minecraft:burning_time",
                        "id": "minecraft:enchantment.fire_protection",
                        "operation": "add_multiplied_base"
                    }],
                    "minecraft:damage_protection": [protection(2.0, json!([tagged("#minecraft:is_fire"), not_bypassing]))]
                }}),
            ),
            (
                "minecraft:feather_falling".to_owned(),
                json!({"effects": {"minecraft:damage_protection": [protection(3.0, json!([tagged("#minecraft:is_fall"), not_bypassing]))]}}),
            ),
            (
                "minecraft:sharpness".to_owned(),
                json!({"effects": {"minecraft:damage": []}}),
            ),
        ];
        let tags = HashMap::from([
            (
                "minecraft:is_fire".to_owned(),
                json!({"values": ["minecraft:lava", "#minecraft:burns"]}),
            ),
            (
                "minecraft:burns".to_owned(),
                json!({"values": ["minecraft:on_fire", {"id": "minecraft:in_fire", "required": false}]}),
            ),
            (
                "minecraft:is_fall".to_owned(),
                json!({"values": ["minecraft:fall"]}),
            ),
            (
                "minecraft:bypasses_invulnerability".to_owned(),
                json!({"values": ["minecraft:out_of_world"]}),
            ),
            (
                "minecraft:bypasses_effects".to_owned(),
                json!({"values": ["minecraft:starve"]}),
            ),
        ]);
        ProtectionBook::from_json(enchantments, &tags)
    }

    fn worn(levels: &[(&str, i32)]) -> Vec<(String, i32)> {
        levels
            .iter()
            .map(|(id, level)| ((*id).to_owned(), *level))
            .collect()
    }

    #[test]
    fn protections_add_their_levels_for_the_damage_types_they_match() {
        let book = book();
        let full = worn(&[
            ("minecraft:protection", 4),
            ("minecraft:fire_protection", 4),
            ("minecraft:feather_falling", 4),
        ]);
        assert_eq!(book.protection(&full, "minecraft:mob_attack"), 4.0);
        assert_eq!(book.protection(&full, "minecraft:lava"), 12.0);
        assert_eq!(book.protection(&full, "minecraft:in_fire"), 12.0);
        assert_eq!(book.protection(&full, "minecraft:fall"), 16.0);
        assert_eq!(book.protection(&full, "minecraft:out_of_world"), 0.0);
        assert_eq!(book.protection(&full, "minecraft:starve"), 0.0);
        let two_pieces = worn(&[("minecraft:protection", 2), ("minecraft:protection", 3)]);
        assert_eq!(book.protection(&two_pieces, "minecraft:mob_attack"), 5.0);
        assert_eq!(book.protection(&[], "minecraft:lava"), 0.0);
    }

    #[test]
    fn fire_protection_shortens_burning_by_fifteen_percent_a_level() {
        let book = book();
        assert_eq!(book.burning_time(&[]), 1.0);
        let one = worn(&[("minecraft:fire_protection", 1)]);
        assert!((book.burning_time(&one) - 0.85).abs() < 1e-6);
        let many = worn(&[
            ("minecraft:fire_protection", 4),
            ("minecraft:fire_protection", 4),
        ]);
        assert_eq!(book.burning_time(&many), 0.0);
        let shield = Protection {
            book: Some(Arc::new(book)),
            worn: worn(&[("minecraft:fire_protection", 2)]),
        };
        assert_eq!(shield.burning_ticks(300), 210);
        assert_eq!(shield.burning_ticks(160), 112);
        assert_eq!(Protection::default().burning_ticks(300), 300);
    }

    #[test]
    fn only_armour_slots_count() {
        let mut inventory = Inventory::default();
        let enchanted = |id: &str, levels: Value| {
            let mut stack = ItemStack::new(id, 1);
            stack.components = Some(json!({ "minecraft:enchantments": levels }));
            stack
        };
        inventory.slots[39] = Some(enchanted(
            "minecraft:iron_helmet",
            json!({"minecraft:protection": 3}),
        ));
        inventory.slots[36] = Some(enchanted(
            "minecraft:iron_boots",
            json!({"feather_falling": 2}),
        ));
        inventory.slots[0] = Some(enchanted(
            "minecraft:iron_chestplate",
            json!({"minecraft:protection": 4}),
        ));
        let worn = Protection::worn(Some(Arc::new(book())), &inventory);
        assert_eq!(worn.against("minecraft:mob_attack"), 3.0);
        assert_eq!(worn.against("minecraft:fall"), 9.0);
        assert_eq!(Protection::default().against("minecraft:fall"), 0.0);
    }

    #[test]
    fn the_vanilla_protections_read_from_the_jar() {
        let Ok(root) = std::env::var("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = ProtectionBook::from_jar(&jar).unwrap();
        let level = |id: &str| worn(&[(id, 4)]);
        assert_eq!(
            book.protection(&level("minecraft:protection"), "minecraft:mob_attack"),
            4.0
        );
        assert_eq!(
            book.protection(&level("minecraft:protection"), "minecraft:generic_kill"),
            0.0
        );
        assert_eq!(
            book.protection(&level("minecraft:protection"), "minecraft:starve"),
            0.0
        );
        assert_eq!(
            book.protection(&level("minecraft:fire_protection"), "minecraft:lava"),
            8.0
        );
        assert_eq!(
            book.protection(&level("minecraft:fire_protection"), "minecraft:fireball"),
            8.0
        );
        assert_eq!(
            book.protection(&level("minecraft:blast_protection"), "minecraft:explosion"),
            8.0
        );
        assert_eq!(
            book.protection(&level("minecraft:projectile_protection"), "minecraft:arrow"),
            8.0
        );
        assert_eq!(
            book.protection(&level("minecraft:feather_falling"), "minecraft:fall"),
            12.0
        );
        assert_eq!(
            book.protection(&level("minecraft:feather_falling"), "minecraft:ender_pearl"),
            12.0
        );
        assert_eq!(
            book.protection(&level("minecraft:feather_falling"), "minecraft:lava"),
            0.0
        );
        assert!((book.burning_time(&level("minecraft:fire_protection")) - 0.4).abs() < 1e-6);
        let walker = worn(&[("minecraft:frost_walker", 2)]);
        assert!(book.immune(&walker, "minecraft:hot_floor"));
        assert!(book.immune(&walker, "minecraft:campfire"));
        assert!(!book.immune(&walker, "minecraft:lava"));
        let mut lake = Lake::default();
        for x in -6..=6 {
            for z in -6..=6 {
                lake.0
                    .insert((x, 0, z), Block::new("minecraft:water").with("level", "0"));
            }
        }
        let frozen = book.location_changed(&walker, [0.5, 1.0, 0.5], true, &lake, &Air, &|_| true);
        assert_eq!(
            frozen.len(),
            45,
            "Frost Walker II freezes a disk of radius 4"
        );
        assert!(
            frozen
                .iter()
                .all(|(_, block)| block == &Block::new("minecraft:frosted_ice").with("age", "0"))
        );
    }

    #[test]
    fn no_protection_softens_a_warden_s_sonic_boom() {
        let Ok(root) = std::env::var("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = ProtectionBook::from_jar(&jar).unwrap();
        let all = worn(&[
            ("minecraft:protection", 4),
            ("minecraft:blast_protection", 4),
            ("minecraft:projectile_protection", 4),
        ]);
        assert_eq!(book.protection(&all, "minecraft:sonic_boom"), 0.0);
        assert_eq!(book.protection(&all, "minecraft:mob_attack"), 4.0);
    }

    #[derive(Default)]
    struct Lake(HashMap<Pos, Block>);

    impl World for Lake {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.0.get(&pos).cloned()
        }

        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.0.insert(pos, block),
                None => self.0.remove(&pos),
            };
        }
    }

    struct Air;

    impl BlockFacts for Air {
        fn in_tag(&self, block: Option<&Block>, tag: &str) -> bool {
            tag == "minecraft:air" && block.is_none()
        }

        fn has_property(&self, _block: &Block, _name: &str) -> bool {
            false
        }
    }

    fn frost_book() -> ProtectionBook {
        let enchantments = vec![(
            "minecraft:frost_walker".to_owned(),
            json!({
                "effects": {
                    "minecraft:damage_immunity": [{
                        "effect": {},
                        "requirements": {
                            "type": "minecraft:damage_source_properties",
                            "predicate": {"tags": [{"expected": true, "id": "#minecraft:burn_from_stepping"}]}
                        }
                    }],
                    "minecraft:location_changed": [{
                        "effect": {
                            "type": "minecraft:replace_disk",
                            "block_state": {"id": "minecraft:frosted_ice", "properties": {"age": "0"}},
                            "height": 1.0,
                            "offset": [0, -1, 0],
                            "predicate": {"type": "minecraft:all_of", "predicates": [
                                {"type": "minecraft:matching_block_tag", "offset": [0, 1, 0], "tag": "minecraft:air"},
                                {"type": "minecraft:matching_blocks", "blocks": "minecraft:water"},
                                {"type": "minecraft:matching_fluids", "fluids": "minecraft:water"},
                                {"type": "minecraft:unobstructed"}
                            ]},
                            "radius": {"type": "minecraft:clamped", "max": 16.0, "min": 0.0,
                                "value": {"type": "minecraft:linear", "base": 3.0, "per_level_above_first": 1.0}}
                        },
                        "requirements": {"type": "minecraft:all_of", "terms": [
                            {"type": "minecraft:entity_properties", "entity": "this",
                                "predicate": {"minecraft:flags": {"is_on_ground": true}}},
                            {"type": "minecraft:inverted", "term": {"type": "minecraft:entity_properties",
                                "entity": "this", "predicate": {"minecraft:vehicle": {}}}}
                        ]}
                    }]
                },
                "slots": ["feet"]
            }),
        )];
        let tags = HashMap::from([(
            "minecraft:burn_from_stepping".to_owned(),
            json!({"values": ["minecraft:hot_floor", "minecraft:campfire"]}),
        )]);
        ProtectionBook::from_json(enchantments, &tags)
    }

    #[test]
    fn frost_walker_freezes_still_open_water_around_the_feet_on_the_ground() {
        let book = frost_book();
        let mut lake = Lake::default();
        for x in -5..=5 {
            for z in -5..=5 {
                lake.0
                    .insert((x, 0, z), Block::new("minecraft:water").with("level", "0"));
            }
        }
        let walker = |level| worn(&[("minecraft:frost_walker", level)]);
        let feet = [0.5, 1.0, 0.5];
        let frozen = |lake: &Lake, level, ground, free: &dyn Fn(Pos) -> bool| {
            let mut cells: Vec<Pos> = book
                .location_changed(&walker(level), feet, ground, lake, &Air, free)
                .into_iter()
                .map(|(pos, _)| pos)
                .collect();
            cells.sort_unstable();
            cells
        };
        assert_eq!(frozen(&lake, 1, true, &|_| true).len(), 25);
        assert_eq!(frozen(&lake, 2, true, &|_| true).len(), 45);
        assert!(
            frozen(&lake, 1, false, &|_| true).is_empty(),
            "only on the ground"
        );
        lake.0.insert((1, 1, 0), Block::new("minecraft:lily_pad"));
        lake.0
            .insert((0, 0, 1), Block::new("minecraft:water").with("level", "3"));
        let cells = frozen(&lake, 1, true, &|pos| pos != (-1, 0, -1));
        assert_eq!(cells.len(), 22);
        assert!(!cells.contains(&(1, 0, 0)), "covered");
        assert!(!cells.contains(&(0, 0, 1)), "flowing");
        assert!(!cells.contains(&(-1, 0, -1)), "a body in the way");
        assert!(cells.iter().all(|pos| pos.1 == 0));
    }

    #[test]
    fn frost_walker_shields_from_hot_floors_and_only_counts_on_the_feet() {
        let mut inventory = Inventory::default();
        let enchanted = |id: &str| {
            let mut stack = ItemStack::new(id, 1);
            stack.components =
                Some(json!({ "minecraft:enchantments": {"minecraft:frost_walker": 2} }));
            stack
        };
        inventory.slots[39] = Some(enchanted("minecraft:diamond_helmet"));
        let book = Some(Arc::new(frost_book()));
        let on_head = Protection::worn(book.clone(), &inventory);
        assert!(!on_head.immune("minecraft:hot_floor"));
        inventory.slots[36] = Some(enchanted("minecraft:diamond_boots"));
        let on_feet = Protection::worn(book, &inventory);
        assert!(on_feet.immune("minecraft:hot_floor"));
        assert!(on_feet.immune("minecraft:campfire"));
        assert!(!on_feet.immune("minecraft:lava"));
        assert!(!Protection::default().immune("minecraft:hot_floor"));
    }
}
