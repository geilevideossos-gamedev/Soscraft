use minecraftoss_core::nbt::Tag;
use serde_json::Value;

pub fn tag_json(tag: &Tag) -> Value {
    match tag {
        Tag::Byte(v) => Value::from(*v),
        Tag::Short(v) => Value::from(*v),
        Tag::Int(v) => Value::from(*v),
        Tag::Long(v) => Value::from(*v),
        Tag::Float(v) => Value::from(f64::from(*v)),
        Tag::Double(v) => Value::from(*v),
        Tag::ByteArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
        Tag::String(s) => Value::from(s.clone()),
        Tag::List(list) => Value::Array(list.iter().map(tag_json).collect()),
        Tag::Compound(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), tag_json(v))).collect())
        }
        Tag::IntArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
        Tag::LongArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
    }
}

pub fn json_tag(value: &Value) -> Tag {
    match value {
        Value::Object(map) => {
            Tag::Compound(map.iter().map(|(k, v)| (k.clone(), json_tag(v))).collect())
        }
        Value::Array(list) => Tag::List(list.iter().map(json_tag).collect()),
        Value::String(s) => Tag::String(s.clone()),
        Value::Bool(b) => Tag::Byte(i8::from(*b)),
        Value::Number(n) => match n.as_i64() {
            Some(v) => i32::try_from(v).map_or(Tag::Long(v), Tag::Int),
            None => Tag::Double(n.as_f64().unwrap_or(0.0)),
        },
        Value::Null => Tag::Compound(Default::default()),
    }
}

pub fn components_tag(components: &Value) -> Option<Tag> {
    match json_tag(components) {
        Tag::Compound(map) if !map.is_empty() => Some(Tag::Compound(map)),
        _ => None,
    }
}

pub fn components_from_text(text: &str) -> Option<Tag> {
    serde_json::from_str::<Value>(text)
        .ok()
        .as_ref()
        .and_then(components_tag)
}

pub fn saved_components(tag: &Tag) -> Option<Tag> {
    match tag {
        Tag::String(text) => components_from_text(text),
        Tag::Compound(map) if !map.is_empty() => Some(tag.clone()),
        _ => None,
    }
}

pub fn components_text(tag: &Tag) -> String {
    match tag {
        Tag::String(text) => text.clone(),
        _ => tag_json(tag).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn compound(entries: Vec<(&str, Tag)>) -> Tag {
        Tag::Compound(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        )
    }

    #[test]
    fn enchantments_and_a_name_become_a_compound_of_ints_and_strings() {
        let components = json!({
            "minecraft:enchantments": {"minecraft:sharpness": 5, "minecraft:unbreaking": 3},
            "minecraft:custom_name": "Excalibur",
            "minecraft:damage": 12,
            "minecraft:repair_cost": 3
        });
        let tag = components_tag(&components).expect("a compound");
        assert_eq!(
            tag.get("minecraft:enchantments"),
            Some(&compound(vec![
                ("minecraft:sharpness", Tag::Int(5)),
                ("minecraft:unbreaking", Tag::Int(3)),
            ]))
        );
        assert_eq!(
            tag.get("minecraft:custom_name"),
            Some(&Tag::String("Excalibur".to_owned()))
        );
        assert_eq!(tag.get("minecraft:damage"), Some(&Tag::Int(12)));
        assert_eq!(tag_json(&tag), components);
    }

    #[test]
    fn a_shulker_box_keeps_its_nested_items_and_their_potions() {
        let components = json!({
            "minecraft:container": [
                {"slot": 0, "item": {"id": "minecraft:diamond", "count": 3}},
                {"slot": 26, "item": {
                    "id": "minecraft:potion",
                    "count": 1,
                    "components": {"minecraft:potion_contents": {"potion": "minecraft:swiftness"}}
                }}
            ],
            "minecraft:custom_name": "Loot"
        });
        let tag = components_tag(&components).expect("a compound");
        let entries = tag
            .get("minecraft:container")
            .and_then(Tag::as_list)
            .expect("a list");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].get("slot"), Some(&Tag::Int(0)));
        assert_eq!(
            entries[0].get("item").and_then(|item| item.get("count")),
            Some(&Tag::Int(3))
        );
        let potion = entries[1]
            .get("item")
            .and_then(|item| item.get("components"))
            .and_then(|c| c.get("minecraft:potion_contents"))
            .and_then(|c| c.get("potion"));
        assert_eq!(potion, Some(&Tag::String("minecraft:swiftness".to_owned())));
        assert_eq!(tag_json(&tag), components);
        assert_eq!(
            components_from_text(&components_text(&tag)).as_ref(),
            Some(&tag)
        );
    }

    #[test]
    fn a_plain_potion_round_trips_through_its_text() {
        let text = r#"{"minecraft:potion_contents":{"potion":"minecraft:healing"}}"#;
        let tag = components_from_text(text).expect("a compound");
        assert_eq!(components_text(&tag), text);
    }

    #[test]
    fn components_saved_as_text_by_older_builds_still_load() {
        let old = Tag::String(r#"{"minecraft:enchantments":{"minecraft:sharpness":5}}"#.to_owned());
        let loaded = saved_components(&old).expect("a compound");
        assert_eq!(
            loaded,
            compound(vec![(
                "minecraft:enchantments",
                compound(vec![("minecraft:sharpness", Tag::Int(5))])
            )])
        );
        assert_eq!(saved_components(&loaded), Some(loaded.clone()));
        assert_eq!(saved_components(&Tag::String("{}".to_owned())), None);
        assert_eq!(saved_components(&Tag::String("not json".to_owned())), None);
        assert_eq!(saved_components(&compound(Vec::new())), None);
        assert_eq!(
            components_text(&old),
            r#"{"minecraft:enchantments":{"minecraft:sharpness":5}}"#
        );
    }

    #[test]
    fn booleans_become_bytes_and_large_numbers_stay_whole() {
        let tag = json_tag(&json!({"glint": true, "big": 5_000_000_000_i64, "scale": 0.5}));
        assert_eq!(tag.get("glint"), Some(&Tag::Byte(1)));
        assert_eq!(tag.get("big"), Some(&Tag::Long(5_000_000_000)));
        assert_eq!(tag.get("scale"), Some(&Tag::Double(0.5)));
        assert_eq!(components_tag(&json!({})), None);
        assert_eq!(components_tag(&json!("minecraft:healing")), None);
    }
}
