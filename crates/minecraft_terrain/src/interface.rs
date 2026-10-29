//! The item model helpers the block mesher needs, from the viewer's interface.
use crate::pack::{PackStack, ResourceId};
use anyhow::Result;

pub(crate) fn item_model_reference(value: &serde_json::Value) -> Option<&str> {
    item_model_leaf(value)?.get("model")?.as_str()
}

pub(crate) fn item_gui_model_reference(value: &serde_json::Value) -> Option<&str> {
    let in_gui = |when: &serde_json::Value| match when {
        serde_json::Value::String(one) => one == "gui",
        serde_json::Value::Array(many) => many.iter().any(|one| one.as_str() == Some("gui")),
        _ => false,
    };
    value
        .get("cases")
        .and_then(serde_json::Value::as_array)
        .filter(|_| value.get("property").and_then(serde_json::Value::as_str) == Some("minecraft:display_context"))
        .and_then(|cases| cases.iter().filter(|case| case.get("when").is_some_and(in_gui)).find_map(|case| item_model_leaf(&case["model"])))
        .and_then(|leaf| leaf.get("model")?.as_str())
        .or_else(|| item_model_reference(value))
}

fn item_model_leaf(value: &serde_json::Value) -> Option<&serde_json::Value> {
    match value.get("type")?.as_str()? {
        "minecraft:model" => value.get("model")?.as_str().map(|_| value),
        "minecraft:select" | "minecraft:range_dispatch" => value
            .get("fallback")
            .and_then(item_model_leaf)
            .or_else(|| {
                value
                    .get("cases")
                    .or_else(|| value.get("entries"))?
                    .as_array()?
                    .iter()
                    .find_map(|entry| item_model_leaf(&entry["model"]))
            }),
        "minecraft:condition" => value
            .get("on_false")
            .or_else(|| value.get("on_true"))
            .and_then(item_model_leaf),
        "minecraft:composite" => value
            .get("models")?
            .as_array()?
            .iter()
            .find_map(item_model_leaf),
        _ => None,
    }
}

pub(crate) fn item_model_tints(
    packs: &PackStack,
    model: &serde_json::Value,
    potion: Option<u32>,
) -> Result<Vec<[u8; 3]>> {
    let Some(tints) = model.get("tints").or_else(|| item_model_leaf(model)?.get("tints")).and_then(serde_json::Value::as_array) else {
        return Ok(crate::special_models::special_tints(model));
    };
    tints
        .iter()
        .map(|tint| {
            let color = match tint.get("type").and_then(serde_json::Value::as_str) {
                Some("minecraft:constant") => tint
                    .get("value")
                    .and_then(serde_json::Value::as_i64)
                    .map(|color| color as u32),
                Some("minecraft:potion" | "minecraft:dye") => potion.or_else(|| {
                    tint.get("default")
                        .and_then(serde_json::Value::as_i64)
                        .map(|color| color as u32)
                }),
                Some("minecraft:firework" | "minecraft:map_color" | "minecraft:team" | "minecraft:custom_model_data") => tint
                    .get("default")
                    .and_then(serde_json::Value::as_i64)
                    .map(|color| color as u32),
                Some(kind @ ("minecraft:grass" | "minecraft:foliage")) => {
                    let name = if kind == "minecraft:grass" {
                        "grass"
                    } else {
                        "foliage"
                    };
                    let id = ResourceId::parse(&format!("minecraft:colormap/{name}"))?;
                    let temp = tint
                        .get("temperature")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.5)
                        .clamp(0.0, 1.0);
                    let rain = tint
                        .get("downfall")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.5)
                        .clamp(0.0, 1.0)
                        * temp;
                    if let Some(bytes) = packs.texture(&id)? {
                        let map = image::load_from_memory(&bytes)?.to_rgba8();
                        if map.width() >= 256 && map.height() >= 256 {
                            let pixel = map.get_pixel(
                                ((1.0 - temp) * 255.0) as u32,
                                ((1.0 - rain) * 255.0) as u32,
                            );
                            Some(u32::from_be_bytes([0, pixel[0], pixel[1], pixel[2]]))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
            .unwrap_or(0xffffff);
            Ok([
                ((color >> 16) & 255) as u8,
                ((color >> 8) & 255) as u8,
                (color & 255) as u8,
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dyed_and_firework_tints_use_their_default_colour_through_selects() {
        let packs = PackStack::open(vec![]).unwrap();
        let dye = json!({"type": "minecraft:dye", "default": -6265536});
        let leather = json!({
            "type": "minecraft:select",
            "property": "minecraft:trim_material",
            "cases": [{"when": "minecraft:iron", "model": {"type": "minecraft:model", "model": "minecraft:item/leather_helmet_iron_trim", "tints": [dye.clone()]}}],
            "fallback": {"type": "minecraft:model", "model": "minecraft:item/leather_helmet", "tints": [dye.clone()]}
        });
        assert_eq!(item_model_reference(&leather), Some("minecraft:item/leather_helmet"));
        assert_eq!(item_model_tints(&packs, &leather, None).unwrap(), vec![[0xa0, 0x65, 0x40]]);
        let horse = json!({"type": "minecraft:model", "model": "minecraft:item/leather_horse_armor", "tints": [dye]});
        assert_eq!(item_model_tints(&packs, &horse, None).unwrap(), vec![[0xa0, 0x65, 0x40]]);
        let star = json!({
            "type": "minecraft:model",
            "model": "minecraft:item/firework_star",
            "tints": [{"type": "minecraft:constant", "value": -1}, {"type": "minecraft:firework", "default": -7697782}]
        });
        assert_eq!(item_model_tints(&packs, &star, None).unwrap(), vec![[0xff; 3], [0x8a; 3]]);
        let wolf = json!({
            "type": "minecraft:condition",
            "property": "minecraft:has_component",
            "component": "minecraft:dyed_color",
            "on_false": {"type": "minecraft:model", "model": "minecraft:item/wolf_armor"},
            "on_true": {"type": "minecraft:model", "model": "minecraft:item/wolf_armor_dyed", "tints": [{"type": "minecraft:dye", "default": 0}]}
        });
        assert_eq!(item_model_reference(&wolf), Some("minecraft:item/wolf_armor"));
        assert!(item_model_tints(&packs, &wolf, None).unwrap().is_empty());
        let stonecutter = json!({"type": "minecraft:model", "model": "minecraft:block/stonecutter"});
        assert!(item_model_tints(&packs, &stonecutter, None).unwrap().is_empty());
        assert_eq!(item_model_tints(&packs, &horse, Some(0xF82423)).unwrap(), vec![[0xf8, 0x24, 0x23]]);
        assert_eq!(item_model_tints(&packs, &leather, Some(0xB02E26)).unwrap(), vec![[0xb0, 0x2e, 0x26]]);
        assert!(item_model_tints(&packs, &wolf, Some(0xB02E26)).unwrap().is_empty());
        assert_eq!(item_model_tints(&packs, &star, Some(0xF82423)).unwrap(), vec![[0xff; 3], [0x8a; 3]]);
    }

    #[test]
    fn a_potion_tint_takes_the_stack_colour_or_its_default() {
        let packs = PackStack::open(vec![]).unwrap();
        let potion = json!({
            "type": "minecraft:model",
            "model": "minecraft:item/potion",
            "tints": [{"type": "minecraft:potion", "default": -13083194}]
        });
        assert_eq!(item_model_tints(&packs, &potion, Some(0xF82423)).unwrap(), vec![[0xf8, 0x24, 0x23]]);
        assert_eq!(item_model_tints(&packs, &potion, None).unwrap(), vec![[0x38, 0x5d, 0xc6]]);
    }

    #[test]
    fn the_inventory_icon_takes_the_gui_case_of_a_display_context_select() {
        let spyglass = json!({
            "type": "minecraft:select",
            "property": "minecraft:display_context",
            "cases": [{"when": ["gui", "ground", "fixed", "on_shelf"], "model": {"type": "minecraft:model", "model": "minecraft:item/spyglass"}}],
            "fallback": {"type": "minecraft:model", "model": "minecraft:item/spyglass_in_hand"}
        });
        assert_eq!(item_gui_model_reference(&spyglass), Some("minecraft:item/spyglass"));
        assert_eq!(item_model_reference(&spyglass), Some("minecraft:item/spyglass_in_hand"));
        let bundle = json!({
            "type": "minecraft:select",
            "property": "minecraft:display_context",
            "cases": [{"when": "gui", "model": {"type": "minecraft:condition", "property": "minecraft:bundle/has_selected_item", "on_false": {"type": "minecraft:model", "model": "minecraft:item/bundle"}, "on_true": {"type": "minecraft:model", "model": "minecraft:item/bundle_open_front"}}}],
            "fallback": {"type": "minecraft:model", "model": "minecraft:item/bundle"}
        });
        assert_eq!(item_gui_model_reference(&bundle), Some("minecraft:item/bundle"));
        let held_only = json!({
            "type": "minecraft:select",
            "property": "minecraft:display_context",
            "cases": [{"when": "firstperson_righthand", "model": {"type": "minecraft:model", "model": "test:item/held"}}],
            "fallback": {"type": "minecraft:model", "model": "test:item/plain"}
        });
        assert_eq!(item_gui_model_reference(&held_only), Some("test:item/plain"));
        let other = json!({
            "type": "minecraft:select",
            "property": "minecraft:trim_material",
            "cases": [{"when": "gui", "model": {"type": "minecraft:model", "model": "test:item/trimmed"}}],
            "fallback": {"type": "minecraft:model", "model": "test:item/plain"}
        });
        assert_eq!(item_gui_model_reference(&other), Some("test:item/plain"));
    }
}
