use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use minecraftoss_player::inventory::Inventory;
use serde_json::Value;

const ENCHANTMENT_FOLDER: &str = "data/minecraft/enchantment/";
const DAMAGE_TAG_FOLDER: &str = "data/minecraft/tags/damage_type/";
const ENCHANTMENTS: &str = "minecraft:enchantments";
const DAMAGE_PROTECTION: &str = "minecraft:damage_protection";
const ATTRIBUTES: &str = "minecraft:attributes";
const BURNING_TIME: &str = "minecraft:burning_time";
const ADD_MULTIPLIED_BASE: &str = "add_multiplied_base";
const BYPASSES_EFFECTS: &str = "minecraft:bypasses_effects";
const BYPASSES_ENCHANTMENTS: &str = "minecraft:bypasses_enchantments";
const ARMOR_SLOTS: [usize; 4] = [36, 37, 38, 39];
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
        let requirements = &entry["requirements"];
        let tags = if requirements.is_null() {
            Vec::new()
        } else {
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
                .collect::<Option<Vec<_>>>()?
        };
        Some(Self { amount, tags })
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProtectionBook {
    protections: HashMap<String, Vec<DamageProtection>>,
    burning: HashMap<String, Vec<Linear>>,
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
                .unwrap_or_default();
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
                .unwrap_or_default();
            if !burning.is_empty() {
                book.burning.insert(id, burning);
            }
        }
        for tag in tags.keys() {
            let mut members = HashSet::new();
            resolve(tags, tag, &mut members, 0);
            book.damage_tags.insert(tag.clone(), members);
        }
        book
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
            .filter(|(protection, _)| {
                protection
                    .tags
                    .iter()
                    .all(|(tag, expected)| self.in_tag(damage_type, tag) == *expected)
            })
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
            .unwrap_or_default();
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
            .filter_map(|&slot| inventory.slots.get(slot)?.as_ref())
            .filter_map(|stack| stack.components.as_ref()?.get(ENCHANTMENTS)?.as_object())
            .flat_map(|levels| {
                levels.iter().filter_map(|(id, level)| {
                    let id = if id.contains(':') {
                        id.clone()
                    } else {
                        format!("minecraft:{id}")
                    };
                    Some((id, i32::try_from(level.as_i64()?).ok()?))
                })
            })
            .collect();
        Self { book, worn }
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
    }
}
