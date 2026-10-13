use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::class_file::{self, ClassFile};
use crate::creative_tabs::Variants;

pub const POTIONS_CLASS: &str = "net/minecraft/world/item/alchemy/Potions";
pub const POTION_IDS_CLASS: &str = "net/minecraft/world/item/alchemy/PotionIds";
pub const TEST_BLOCK_MODE_CLASS: &str =
    "net/minecraft/world/level/block/state/properties/TestBlockMode";
const OMINOUS_AMPLIFIERS: std::ops::RangeInclusive<i64> = 0..=4;
const FLIGHT_DURATIONS: std::ops::RangeInclusive<i64> = 1..=3;
const LIGHT_LEVELS: std::ops::RangeInclusive<i64> = 0..=15;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VariantData {
    pub potions: Vec<String>,
    pub enchantments: Vec<(String, i64)>,
    pub instrument_tags: BTreeMap<String, Vec<String>>,
    pub paintings: Vec<(String, i64, i64)>,
    pub painting_tags: BTreeMap<String, Vec<String>>,
    pub test_block_modes: Vec<String>,
}

fn static_strings(class: &ClassFile) -> Result<Vec<(String, Option<String>)>> {
    let method = class
        .method("<clinit>")
        .and_then(|method| method.code.as_ref())
        .context("no static initializer")?;
    let mut out = Vec::new();
    let mut last = None;
    for instruction in class_file::instructions(method)? {
        match instruction.op {
            class_file::LDC | class_file::LDC_W => {
                if let Some(text) = class.string(instruction.index) {
                    last = Some(text.to_owned());
                }
            }
            class_file::PUTSTATIC => {
                if let Some(member) = class.member(instruction.index) {
                    out.push((member.name, last.take()));
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

pub fn potion_ids(potions: &ClassFile, ids: Option<&ClassFile>) -> Result<Vec<String>> {
    let names: HashMap<String, String> = match ids {
        Some(ids) => static_strings(ids)?
            .into_iter()
            .filter_map(|(field, text)| Some((field, text?)))
            .collect(),
        None => HashMap::new(),
    };
    let method = potions
        .method("<clinit>")
        .and_then(|method| method.code.as_ref())
        .context("Potions has no static initializer")?;
    let mut out: Vec<String> = Vec::new();
    for instruction in class_file::instructions(method)? {
        if instruction.op != class_file::GETSTATIC {
            continue;
        }
        let Some(member) = potions.member(instruction.index) else {
            continue;
        };
        if member.class != POTION_IDS_CLASS {
            continue;
        }
        let path = names
            .get(&member.name)
            .cloned()
            .unwrap_or_else(|| member.name.to_lowercase());
        let id = if path.contains(':') {
            path
        } else {
            format!("minecraft:{path}")
        };
        if !out.contains(&id) {
            out.push(id);
        }
    }
    Ok(out)
}

pub fn enum_names(class: &ClassFile) -> Result<Vec<String>> {
    let own = format!("L{};", class.name);
    let method = class
        .method("<clinit>")
        .and_then(|method| method.code.as_ref())
        .context("no static initializer")?;
    let mut out = Vec::new();
    for instruction in class_file::instructions(method)? {
        if instruction.op != class_file::PUTSTATIC {
            continue;
        }
        if let Some(member) = class.member(instruction.index) {
            if member.descriptor == own && member.class == class.name {
                out.push(member.name.to_lowercase());
            }
        }
    }
    Ok(out)
}

pub fn expand_tag(tags: &BTreeMap<String, Vec<String>>, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    expand_into(tags, tag, &mut out, 0);
    out
}

fn expand_into(
    tags: &BTreeMap<String, Vec<String>>,
    tag: &str,
    out: &mut Vec<String>,
    depth: usize,
) {
    if depth > 16 {
        return;
    }
    for value in tags.get(tag).into_iter().flatten() {
        match value.strip_prefix('#') {
            Some(inner) => expand_into(tags, inner, out, depth + 1),
            None => {
                if !out.contains(value) {
                    out.push(value.clone());
                }
            }
        }
    }
}

pub fn tag_values(json: &Value) -> Vec<String> {
    json.get("values")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |values| {
            values
                .iter()
                .filter_map(|value| match value {
                    Value::String(text) => Some(text.clone()),
                    Value::Object(fields) => {
                        fields.get("id").and_then(Value::as_str).map(str::to_owned)
                    }
                    _ => None,
                })
                .collect()
        })
}

impl VariantData {
    pub fn read(jar: &Path) -> Result<Self> {
        let file =
            std::fs::File::open(jar).with_context(|| format!("cannot open {}", jar.display()))?;
        let mut archive = zip::ZipArchive::new(file)?;
        let names: Vec<String> = archive.file_names().map(str::to_owned).collect();
        let mut read = |name: &str| -> Option<Vec<u8>> {
            let mut entry = archive.by_name(name).ok()?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).ok()?;
            Some(bytes)
        };
        let class = |bytes: Option<Vec<u8>>| bytes.and_then(|bytes| ClassFile::parse(&bytes).ok());
        let potions_class =
            class(read(&format!("{POTIONS_CLASS}.class"))).context("missing the Potions class")?;
        let ids_class = class(read(&format!("{POTION_IDS_CLASS}.class")));
        let potions = potion_ids(&potions_class, ids_class.as_ref())?;
        let test_block_modes = class(read(&format!("{TEST_BLOCK_MODE_CLASS}.class")))
            .map(|class| enum_names(&class))
            .transpose()?
            .unwrap_or_else(Vec::new);
        let mut json_files = |folder: &str| -> Vec<(String, Value)> {
            let mut found: Vec<(String, Value)> = names
                .iter()
                .filter_map(|name| {
                    let rest = name.strip_prefix(folder)?.strip_suffix(".json")?;
                    Some((name.clone(), format!("minecraft:{rest}")))
                })
                .filter_map(|(name, id)| {
                    let bytes = read(&name)?;
                    Some((id, serde_json::from_slice(&bytes).ok()?))
                })
                .collect();
            found.sort_by(|a, b| a.0.cmp(&b.0));
            found
        };
        let enchantments = json_files("data/minecraft/enchantment/")
            .into_iter()
            .map(|(id, json)| {
                let max = json.get("max_level").and_then(Value::as_i64).unwrap_or(1);
                (id, max)
            })
            .collect();
        let paintings = json_files("data/minecraft/painting_variant/")
            .into_iter()
            .map(|(id, json)| {
                let size = |key: &str| json.get(key).and_then(Value::as_i64).unwrap_or(1);
                (id, size("width"), size("height"))
            })
            .collect();
        let instrument_tags = json_files("data/minecraft/tags/instrument/")
            .into_iter()
            .map(|(id, json)| (id, tag_values(&json)))
            .collect();
        let painting_tags = json_files("data/minecraft/tags/painting_variant/")
            .into_iter()
            .map(|(id, json)| (id, tag_values(&json)))
            .collect();
        Ok(Self {
            potions,
            enchantments,
            instrument_tags,
            paintings,
            painting_tags,
            test_block_modes,
        })
    }

    pub fn expand(&self, variants: &Variants) -> Option<Vec<Value>> {
        Some(match variants {
            Variants::Potions => self
                .potions
                .iter()
                .map(|potion| json!({"minecraft:potion_contents": {"potion": potion}}))
                .collect(),
            Variants::EnchantedBooks { all_levels } => self
                .enchantments
                .iter()
                .flat_map(|(id, max)| {
                    let levels = if *all_levels { 1..=*max } else { *max..=*max };
                    levels.map(
                        move |level| json!({"minecraft:stored_enchantments": {id.as_str(): level}}),
                    )
                })
                .collect(),
            Variants::Instruments { tag } => expand_tag(&self.instrument_tags, tag)
                .into_iter()
                .map(|instrument| json!({"minecraft:instrument": instrument}))
                .collect(),
            Variants::OminousBottles => OMINOUS_AMPLIFIERS
                .map(|amplifier| json!({"minecraft:ominous_bottle_amplifier": amplifier}))
                .collect(),
            Variants::Fireworks => FLIGHT_DURATIONS
                .map(|flight| json!({"minecraft:fireworks": {"flight_duration": flight}}))
                .collect(),
            Variants::Paintings { tag, in_tag } => {
                let members = expand_tag(&self.painting_tags, tag);
                let mut chosen: Vec<&(String, i64, i64)> = self
                    .paintings
                    .iter()
                    .filter(|(id, _, _)| members.contains(id) == *in_tag)
                    .collect();
                chosen.sort_by_key(|(_, width, height)| (width * height, *width));
                chosen
                    .into_iter()
                    .map(|(id, _, _)| json!({"minecraft:painting/variant": id}))
                    .collect()
            }
            Variants::LightLevels => LIGHT_LEVELS
                .rev()
                .map(|level| json!({"minecraft:block_state": {"level": level.to_string()}}))
                .collect(),
            Variants::TestBlockModes => self
                .test_block_modes
                .iter()
                .map(|mode| json!({"minecraft:block_state": {"mode": mode}}))
                .collect(),
            Variants::FullClip => vec![crate::fork_items::loaded_gun()],
            Variants::SuspiciousStews | Variants::OminousBanner => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> VariantData {
        let mut instrument_tags = BTreeMap::new();
        instrument_tags.insert(
            "minecraft:goat_horns".to_owned(),
            vec![
                "#minecraft:regular_goat_horns".to_owned(),
                "#minecraft:screaming_goat_horns".to_owned(),
            ],
        );
        instrument_tags.insert(
            "minecraft:regular_goat_horns".to_owned(),
            vec!["minecraft:ponder_goat_horn".to_owned()],
        );
        instrument_tags.insert(
            "minecraft:screaming_goat_horns".to_owned(),
            vec!["minecraft:dream_goat_horn".to_owned()],
        );
        let mut painting_tags = BTreeMap::new();
        painting_tags.insert(
            "minecraft:placeable".to_owned(),
            vec!["minecraft:kebab".to_owned(), "minecraft:pool".to_owned()],
        );
        VariantData {
            potions: vec![
                "minecraft:water".to_owned(),
                "minecraft:swiftness".to_owned(),
            ],
            enchantments: vec![
                ("minecraft:mending".to_owned(), 1),
                ("minecraft:sharpness".to_owned(), 5),
            ],
            instrument_tags,
            paintings: vec![
                ("minecraft:earth".to_owned(), 2, 2),
                ("minecraft:kebab".to_owned(), 1, 1),
                ("minecraft:pool".to_owned(), 2, 1),
                ("minecraft:wanderer".to_owned(), 1, 2),
            ],
            painting_tags,
            test_block_modes: vec!["start".to_owned(), "log".to_owned()],
        }
    }

    #[test]
    fn potions_books_and_horns_expand_like_the_vanilla_generators() {
        let data = data();
        assert_eq!(
            data.expand(&Variants::Potions).unwrap(),
            [
                json!({"minecraft:potion_contents": {"potion": "minecraft:water"}}),
                json!({"minecraft:potion_contents": {"potion": "minecraft:swiftness"}})
            ]
        );
        let books = data
            .expand(&Variants::EnchantedBooks { all_levels: false })
            .unwrap();
        assert_eq!(
            books,
            [
                json!({"minecraft:stored_enchantments": {"minecraft:mending": 1}}),
                json!({"minecraft:stored_enchantments": {"minecraft:sharpness": 5}})
            ]
        );
        let all = data
            .expand(&Variants::EnchantedBooks { all_levels: true })
            .unwrap();
        assert_eq!(all.len(), 6);
        assert_eq!(
            all[1],
            json!({"minecraft:stored_enchantments": {"minecraft:sharpness": 1}})
        );
        assert_eq!(
            data.expand(&Variants::Instruments {
                tag: "minecraft:goat_horns".to_owned()
            })
            .unwrap(),
            [
                json!({"minecraft:instrument": "minecraft:ponder_goat_horn"}),
                json!({"minecraft:instrument": "minecraft:dream_goat_horn"})
            ]
        );
    }

    #[test]
    fn counted_variants_and_paintings_follow_vanilla_order() {
        let data = data();
        let bottles = data.expand(&Variants::OminousBottles).unwrap();
        assert_eq!(bottles.len(), 5);
        assert_eq!(bottles[4], json!({"minecraft:ominous_bottle_amplifier": 4}));
        assert_eq!(
            data.expand(&Variants::Fireworks).unwrap()[0],
            json!({"minecraft:fireworks": {"flight_duration": 1}})
        );
        let lights = data.expand(&Variants::LightLevels).unwrap();
        assert_eq!(lights.len(), 16);
        assert_eq!(lights[0], json!({"minecraft:block_state": {"level": "15"}}));
        assert_eq!(
            data.expand(&Variants::TestBlockModes).unwrap()[1],
            json!({"minecraft:block_state": {"mode": "log"}})
        );
        let painting = |in_tag| -> Vec<String> {
            data.expand(&Variants::Paintings {
                tag: "minecraft:placeable".to_owned(),
                in_tag,
            })
            .unwrap()
            .iter()
            .map(|value| {
                value["minecraft:painting/variant"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect()
        };
        assert_eq!(painting(true), ["minecraft:kebab", "minecraft:pool"]);
        assert_eq!(painting(false), ["minecraft:wanderer", "minecraft:earth"]);
        assert_eq!(data.expand(&Variants::SuspiciousStews), None);
        assert_eq!(data.expand(&Variants::OminousBanner), None);
    }

    #[test]
    fn nested_tags_expand_once_each() {
        let mut tags = BTreeMap::new();
        tags.insert("a".to_owned(), vec!["#b".to_owned(), "x".to_owned()]);
        tags.insert(
            "b".to_owned(),
            vec!["x".to_owned(), "y".to_owned(), "#a".to_owned()],
        );
        assert_eq!(expand_tag(&tags, "a"), ["x", "y"]);
        assert_eq!(
            tag_values(&json!({"values": ["a", {"id": "b", "required": false}, 3]})),
            ["a", "b"]
        );
    }

    #[test]
    fn the_real_variants_match_vanilla() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let data = VariantData::read(&jar).unwrap();
        assert_eq!(
            data.potions.first().map(String::as_str),
            Some("minecraft:water")
        );
        assert!(
            data.potions
                .iter()
                .any(|potion| potion == "minecraft:long_night_vision")
        );
        assert!(
            data.potions
                .iter()
                .any(|potion| potion == "minecraft:strong_swiftness")
        );
        assert_eq!(data.potions.len(), 46);
        assert!(
            data.enchantments
                .contains(&("minecraft:sharpness".to_owned(), 5))
        );
        assert!(
            data.enchantments
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0)
        );
        assert_eq!(
            data.expand(&Variants::Instruments {
                tag: "minecraft:goat_horns".to_owned()
            })
            .unwrap()
            .len(),
            8
        );
        assert_eq!(data.test_block_modes, ["start", "log", "fail", "accept"]);
        let placeable = data
            .expand(&Variants::Paintings {
                tag: "minecraft:placeable".to_owned(),
                in_tag: true,
            })
            .unwrap();
        assert_eq!(
            placeable[0],
            json!({"minecraft:painting/variant": "minecraft:alban"})
        );
    }
}
