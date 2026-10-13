use std::collections::BTreeMap;

use crate::fork_items::{self, ForkItem, Kind};
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::ItemStack;
use serde_json::Value;

const MAX_COUNT: u8 = 99;

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

fn vanilla(id: &str) -> bool {
    id.split_once(':')
        .is_none_or(|(namespace, _)| namespace == "minecraft")
}

pub fn saved(id: &str) -> bool {
    vanilla(id) || fork_items::is_fork(id)
}

pub const GUN_CARRIER: &str = "minecraft:carrot_on_a_stick";
pub const ITEM_CARRIER: &str = "minecraft:poisonous_potato";
pub const CARRIER_REMOVALS: [&str; 2] = ["!minecraft:food", "!minecraft:consumable"];
const ITEM_NAME: &str = "minecraft:item_name";
const MAX_STACK_SIZE: &str = "minecraft:max_stack_size";
const DAMAGE_RESISTANT: &str = "minecraft:damage_resistant";
const CONTAINER: &str = "minecraft:container";
const BUNDLE: &str = "minecraft:bundle_contents";
const FORK_KEY: &str = "iw4l";

type Convert = fn(&str, Option<Tag>) -> (String, Option<Tag>);

pub fn carrier(row: &ForkItem) -> &'static str {
    match row.kind {
        Kind::Gun => GUN_CARRIER,
        _ => ITEM_CARRIER,
    }
}

fn text_tag(entries: &[(&str, &str)]) -> Tag {
    Tag::Compound(
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), Tag::String((*value).to_owned())))
            .collect(),
    )
}

fn carrier_components(row: &ForkItem) -> Vec<(&'static str, Tag)> {
    let name = row.language_key();
    let mut added = vec![(
        ITEM_NAME,
        text_tag(&[("translate", &name), ("fallback", row.name)]),
    )];
    if row.kind != Kind::Gun {
        for removal in CARRIER_REMOVALS {
            added.push((removal, Tag::Compound(BTreeMap::new())));
        }
        if row.max_stack != 64 {
            added.push((MAX_STACK_SIZE, Tag::Int(i32::from(row.max_stack))));
        }
    }
    if row.fire_proof {
        added.push((
            DAMAGE_RESISTANT,
            text_tag(&[("types", "#minecraft:is_fire")]),
        ));
    }
    added
}

fn compound_at<'a>(
    map: &'a mut BTreeMap<String, Tag>,
    key: &str,
) -> &'a mut BTreeMap<String, Tag> {
    let entry = map
        .entry(key.to_owned())
        .or_insert_with(|| Tag::Compound(BTreeMap::new()));
    if !matches!(entry, Tag::Compound(_)) {
        *entry = Tag::Compound(BTreeMap::new());
    }
    match entry {
        Tag::Compound(inner) => inner,
        _ => unreachable!(),
    }
}

fn nested_item(item: &mut Tag, convert: Convert, saving: bool) -> bool {
    let Tag::Compound(map) = item else {
        return true;
    };
    let Some(id) = map.get("id").and_then(Tag::as_str).map(str::to_owned) else {
        return true;
    };
    let (id, components) = convert(&id, map.remove("components"));
    if saving && !saved(&id) {
        return false;
    }
    map.insert("id".to_owned(), Tag::String(id));
    if let Some(components) = components {
        map.insert("components".to_owned(), components);
    }
    true
}

fn nested(map: &mut BTreeMap<String, Tag>, convert: Convert, saving: bool) {
    if let Some(Tag::List(entries)) = map.get_mut(CONTAINER) {
        entries.retain_mut(|entry| match entry {
            Tag::Compound(slot) => slot
                .get_mut("item")
                .is_none_or(|item| nested_item(item, convert, saving)),
            _ => true,
        });
    }
    if let Some(Tag::List(entries)) = map.get_mut(BUNDLE) {
        entries.retain_mut(|entry| nested_item(entry, convert, saving));
    }
}

fn non_empty(map: BTreeMap<String, Tag>) -> Option<Tag> {
    (!map.is_empty()).then_some(Tag::Compound(map))
}

pub fn to_disk(id: &str, components: Option<Tag>) -> (String, Option<Tag>) {
    let mut map = match components {
        Some(Tag::Compound(map)) => map,
        _ => BTreeMap::new(),
    };
    nested(&mut map, to_disk, true);
    let Some(row) = fork_items::get(id) else {
        return (id.to_owned(), non_empty(map));
    };
    let custom = compound_at(&mut map, fork_items::CUSTOM_DATA);
    compound_at(custom, FORK_KEY).insert("id".to_owned(), Tag::String(row.id.to_owned()));
    for (key, value) in carrier_components(row) {
        map.entry(key.to_owned()).or_insert(value);
    }
    (carrier(row).to_owned(), Some(Tag::Compound(map)))
}

pub fn from_disk(id: &str, components: Option<Tag>) -> (String, Option<Tag>) {
    let Some(Tag::Compound(mut map)) = components else {
        return (id.to_owned(), None);
    };
    nested(&mut map, from_disk, false);
    let row = map
        .get(fork_items::CUSTOM_DATA)
        .and_then(|custom| custom.get(FORK_KEY))
        .and_then(|state| state.get("id"))
        .and_then(Tag::as_str)
        .and_then(fork_items::get);
    let Some(row) = row else {
        return (id.to_owned(), non_empty(map));
    };
    let custom = compound_at(&mut map, fork_items::CUSTOM_DATA);
    let state = compound_at(custom, FORK_KEY);
    state.remove("id");
    if state.is_empty() {
        custom.remove(FORK_KEY);
    }
    if custom.is_empty() {
        map.remove(fork_items::CUSTOM_DATA);
    }
    for (key, value) in carrier_components(row) {
        if map.get(key) == Some(&value) {
            map.remove(key);
        }
    }
    (row.id.to_owned(), non_empty(map))
}

pub fn stack_tag(stack: &ItemStack) -> Option<Tag> {
    if stack.count == 0 || !saved(&stack.id) {
        return None;
    }
    let components = stack.components.as_ref().and_then(components_tag);
    let (id, components) = to_disk(&stack.id, components);
    let mut map = BTreeMap::new();
    map.insert("id".to_owned(), Tag::String(id));
    map.insert(
        "count".to_owned(),
        Tag::Int(i32::from(stack.count.min(MAX_COUNT))),
    );
    if let Some(components) = components {
        map.insert("components".to_owned(), components);
    }
    Some(Tag::Compound(map))
}

pub fn stack_of(tag: &Tag, recipes: &RecipeBook) -> Option<ItemStack> {
    let id = tag.get("id")?.as_str()?;
    let count = tag.get("count").map_or(Some(1), Tag::as_i64)?;
    if count <= 0 {
        return None;
    }
    let components = tag.get("components").and_then(saved_components);
    let (id, components) = from_disk(id, components);
    let mut stack = recipes.stack(&id, count.min(i64::from(u8::MAX)) as u8);
    stack.components = components.map(|components| tag_json(&components));
    Some(stack)
}

pub fn slot_list<'a>(cells: impl Iterator<Item = &'a Option<ItemStack>>) -> Tag {
    Tag::List(
        cells
            .enumerate()
            .filter_map(|(slot, cell)| {
                let Tag::Compound(mut map) = stack_tag(cell.as_ref()?)? else {
                    return None;
                };
                map.insert("Slot".to_owned(), Tag::Byte(slot as u8 as i8));
                Some(Tag::Compound(map))
            })
            .collect(),
    )
}

pub fn slots(list: &Tag, size: usize, recipes: &RecipeBook) -> Vec<Option<ItemStack>> {
    let mut slots = vec![None; size];
    for entry in list.as_list().unwrap_or(&[]) {
        let Some(slot) = entry
            .get("Slot")
            .and_then(Tag::as_i64)
            .map(|slot| usize::from(slot as u8))
        else {
            continue;
        };
        if slot < size {
            slots[slot] = stack_of(entry, recipes);
        }
    }
    slots
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

    fn fork_stack(id: &str, count: u8, components: Option<Value>) -> ItemStack {
        let mut stack = ItemStack::new(id, count);
        stack.components = components;
        stack
    }

    fn round_trip(stack: &ItemStack) -> (Tag, ItemStack) {
        let tag = stack_tag(stack).expect("saved");
        let back = stack_of(&tag, &RecipeBook::default()).expect("loaded");
        (tag, back)
    }

    fn disk_components(tag: &Tag) -> Value {
        tag_json(tag.get("components").expect("components"))
    }

    fn carrier_tag(id: &str, count: i32, components: &Value) -> Tag {
        compound(vec![
            ("id", Tag::String(id.to_owned())),
            ("count", Tag::Int(count)),
            ("components", json_tag(components)),
        ])
    }

    #[test]
    fn a_gun_is_saved_on_a_carrot_on_a_stick_and_comes_back_with_its_clip_and_attachments() {
        let state = json!({"minecraft:custom_data": {"iw4l": {
            "attachments": ["reflex", "fmj"], "clip": 27, "alt_clip": 1
        }}});
        let gun = fork_stack("iw4l:m4", 1, Some(state.clone()));
        let (tag, back) = round_trip(&gun);
        assert_eq!(tag.get("id"), Some(&Tag::String(GUN_CARRIER.to_owned())));
        assert_eq!(tag.get("count"), Some(&Tag::Int(1)));
        assert_eq!(
            disk_components(&tag),
            json!({
                "minecraft:custom_data": {"iw4l": {
                    "id": "iw4l:m4", "attachments": ["reflex", "fmj"], "clip": 27, "alt_clip": 1
                }},
                "minecraft:item_name": {"translate": "item.iw4l.m4", "fallback": "M4A1"}
            })
        );
        assert_eq!(back.id, "iw4l:m4");
        assert_eq!(back.count, 1);
        assert_eq!(back.components, Some(state));
        let (bare_tag, bare) = round_trip(&fork_stack("iw4l:m4", 1, None));
        assert_eq!(
            disk_components(&bare_tag)["minecraft:custom_data"],
            json!({"iw4l": {"id": "iw4l:m4"}})
        );
        assert_eq!(bare.id, "iw4l:m4");
        assert_eq!(bare.components, None);
    }

    #[test]
    fn a_grenade_stack_is_saved_on_a_poisonous_potato_that_cannot_be_eaten() {
        let (tag, back) = round_trip(&fork_stack("iw4l:frag_grenade", 32, None));
        assert_eq!(tag.get("id"), Some(&Tag::String(ITEM_CARRIER.to_owned())));
        assert_eq!(tag.get("count"), Some(&Tag::Int(32)));
        assert_eq!(
            disk_components(&tag),
            json!({
                "minecraft:custom_data": {"iw4l": {"id": "iw4l:frag_grenade"}},
                "minecraft:item_name": {
                    "translate": "item.iw4l.frag_grenade", "fallback": "Frag Grenade"
                },
                "!minecraft:food": {},
                "!minecraft:consumable": {}
            })
        );
        assert_eq!(back.id, "iw4l:frag_grenade");
        assert_eq!(back.count, 32);
        assert_eq!(back.components, None);
        let (rockets, back) = round_trip(&fork_stack("iw4l:rocket", 16, None));
        assert_eq!(disk_components(&rockets)["minecraft:max_stack_size"], json!(16));
        assert_eq!(back.components, None);
        let (ammo, _) = round_trip(&fork_stack("iw4l:rifle_ammo", 64, None));
        assert_eq!(disk_components(&ammo).get("minecraft:max_stack_size"), None);
    }

    #[test]
    fn netherite_guns_resist_fire_on_disk_and_other_guns_do_not() {
        for (id, resists) in [("iw4l:barrett", true), ("iw4l:m240", true), ("iw4l:m4", false)] {
            let (tag, back) = round_trip(&fork_stack(id, 1, None));
            let components = disk_components(&tag);
            let fire = json!({"types": "#minecraft:is_fire"});
            assert_eq!(
                components.get("minecraft:damage_resistant"),
                resists.then_some(&fire),
                "{id}"
            );
            assert_eq!(components.get("minecraft:max_stack_size"), None, "{id}");
            assert_eq!(back.components, None, "{id}");
        }
    }

    #[test]
    fn a_renamed_gun_keeps_its_name_and_other_packs_keep_their_custom_data() {
        let components = json!({
            "minecraft:custom_name": "Old Faithful",
            "minecraft:custom_data": {"other_pack": {"level": 3}, "iw4l": {"clip": 5}}
        });
        let gun = fork_stack("iw4l:ak47", 1, Some(components.clone()));
        let (tag, back) = round_trip(&gun);
        let disk = disk_components(&tag);
        assert_eq!(disk["minecraft:custom_name"], json!("Old Faithful"));
        assert_eq!(
            disk["minecraft:custom_data"]["other_pack"],
            json!({"level": 3})
        );
        assert_eq!(back.id, "iw4l:ak47");
        assert_eq!(back.components, Some(components));
        let mut renamed_by_a_pack = disk.clone();
        renamed_by_a_pack["minecraft:item_name"] = json!({"translate": "item.pack.gun"});
        let loaded = stack_of(
            &carrier_tag(GUN_CARRIER, 1, &renamed_by_a_pack),
            &RecipeBook::default(),
        )
        .unwrap();
        assert_eq!(loaded.id, "iw4l:ak47");
        assert_eq!(
            loaded.components.as_ref().unwrap()["minecraft:item_name"],
            json!({"translate": "item.pack.gun"})
        );
    }

    #[test]
    fn an_unknown_fork_id_or_a_plain_carrier_stays_the_base_item() {
        let unknown = json!({
            "minecraft:custom_data": {"iw4l": {"id": "iw4l:railgun", "clip": 3}},
            "minecraft:item_name": {"translate": "item.iw4l.railgun", "fallback": "Railgun"}
        });
        let loaded = stack_of(&carrier_tag(GUN_CARRIER, 1, &unknown), &RecipeBook::default())
            .unwrap();
        assert_eq!(loaded.id, GUN_CARRIER);
        assert_eq!(loaded.components, Some(unknown));
        let potato = compound(vec![
            ("id", Tag::String(ITEM_CARRIER.to_owned())),
            ("count", Tag::Int(3)),
        ]);
        let loaded = stack_of(&potato, &RecipeBook::default()).unwrap();
        assert_eq!(
            (loaded.id.as_str(), loaded.count, loaded.components),
            (ITEM_CARRIER, 3, None)
        );
        let other = json!({"minecraft:custom_data": {"other_pack": {"id": "iw4l:m4"}}});
        let (id, components) = from_disk(ITEM_CARRIER, Some(json_tag(&other)));
        assert_eq!(id, ITEM_CARRIER);
        assert_eq!(components.as_ref().map(tag_json), Some(other));
    }

    #[test]
    fn killstreak_mirrors_and_other_namespaces_are_still_never_saved() {
        assert_eq!(stack_tag(&fork_stack("iw4:weapon/12", 1, None)), None);
        assert_eq!(stack_tag(&fork_stack("othermod:widget", 1, None)), None);
        assert_eq!(stack_tag(&fork_stack("iw4l:m4", 0, None)), None);
        assert!(stack_tag(&fork_stack("iw4l:m4", 1, None)).is_some());
        assert!(stack_tag(&fork_stack("minecraft:stone", 1, None)).is_some());
    }

    #[test]
    fn fork_items_in_shulker_boxes_and_bundles_are_saved_and_loaded_both_ways() {
        let gun = json!({"minecraft:custom_data": {"iw4l": {"clip": 30, "attachments": ["acog"]}}});
        let memory = json!({
            "minecraft:container": [
                {"slot": 0, "item": {"id": "iw4l:m4", "count": 1, "components": gun}},
                {"slot": 1, "item": {"id": "iw4l:frag_grenade", "count": 5}},
                {"slot": 2, "item": {"id": "minecraft:diamond", "count": 3}},
                {"slot": 3, "item": {"id": "iw4:weapon/12", "count": 1}},
                {"slot": 4, "item": {"id": "minecraft:bundle", "count": 1, "components": {
                    "minecraft:bundle_contents": [
                        {"id": "iw4l:rocket", "count": 4},
                        {"id": "minecraft:stick", "count": 2}
                    ]
                }}}
            ],
            "minecraft:custom_name": "Armoury"
        });
        let shulker = fork_stack("minecraft:shulker_box", 1, Some(memory.clone()));
        let (tag, back) = round_trip(&shulker);
        let disk = disk_components(&tag);
        let entries = disk["minecraft:container"].as_array().unwrap();
        assert_eq!(entries.len(), 4, "the killstreak mirror is not saved");
        assert_eq!(entries[0]["item"]["id"], json!(GUN_CARRIER));
        assert_eq!(
            entries[0]["item"]["components"]["minecraft:custom_data"]["iw4l"],
            json!({"id": "iw4l:m4", "clip": 30, "attachments": ["acog"]})
        );
        assert_eq!(entries[1]["item"]["id"], json!(ITEM_CARRIER));
        assert_eq!(entries[1]["item"]["count"], json!(5));
        assert_eq!(
            entries[2]["item"],
            json!({"id": "minecraft:diamond", "count": 3})
        );
        let bundle = entries[3]["item"]["components"]["minecraft:bundle_contents"]
            .as_array()
            .unwrap();
        assert_eq!(bundle[0]["id"], json!(ITEM_CARRIER));
        assert_eq!(bundle[0]["components"]["minecraft:max_stack_size"], json!(16));
        assert_eq!(bundle[1], json!({"id": "minecraft:stick", "count": 2}));
        let mut expected = memory;
        expected["minecraft:container"]
            .as_array_mut()
            .unwrap()
            .remove(3);
        assert_eq!(back.id, "minecraft:shulker_box");
        assert_eq!(back.components, Some(expected));
        let foreign = json!({"minecraft:container": [
            {"slot": 0, "item": {"id": "othermod:widget", "count": 1}}
        ]});
        let (_, loaded) = from_disk("minecraft:shulker_box", Some(json_tag(&foreign)));
        assert_eq!(loaded.as_ref().map(tag_json), Some(foreign));
    }

    #[test]
    fn every_fork_item_comes_back_from_its_disk_form() {
        for row in fork_items::rows() {
            let (tag, back) = round_trip(&fork_stack(row.id, row.max_stack, None));
            assert_eq!(tag.get("id").and_then(Tag::as_str), Some(carrier(row)));
            assert_eq!((back.id.as_str(), back.count), (row.id, row.max_stack));
            assert_eq!(back.components, None, "{}", row.id);
        }
    }

    #[test]
    fn every_fork_item_s_disk_form_is_valid_in_vanilla() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let root = std::path::Path::new(&root);
        let Ok(file) = std::fs::File::open(root.join("client.jar")) else {
            return;
        };
        let mut jar = zip::ZipArchive::new(file).expect("the jar");
        let text = std::fs::read_to_string(root.join(crate::crafting::ITEM_CATALOG))
            .expect("the item catalog");
        let catalog: Value = serde_json::from_str(&text).unwrap();
        for row in fork_items::rows() {
            let (tag, _) = round_trip(&fork_stack(row.id, row.max_stack, None));
            let base = tag.get("id").and_then(Tag::as_str).unwrap();
            let path = base.strip_prefix("minecraft:").unwrap();
            assert!(
                jar.by_name(&format!("assets/minecraft/items/{path}.json"))
                    .is_ok(),
                "{base} is a 26.3 item"
            );
            let damageable = catalog["items"]
                .get(base)
                .and_then(|item| item.get("max_damage"))
                .and_then(Value::as_i64)
                .is_some_and(|damage| damage > 0);
            assert_eq!(damageable, row.kind == Kind::Gun, "{}", row.id);
            let size = disk_components(&tag)
                .get("minecraft:max_stack_size")
                .and_then(Value::as_i64);
            if damageable {
                assert_eq!(size, None, "{}", row.id);
            }
            if let Some(size) = size {
                assert!((1..=99).contains(&size), "{}", row.id);
            }
            let stacks_to = size.unwrap_or(if damageable { 1 } else { 64 });
            assert_eq!(stacks_to, i64::from(row.max_stack), "{}", row.id);
        }
    }
}
