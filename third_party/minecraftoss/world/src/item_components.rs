use crate::book::{describe, list, number, string, tag_json};
use minecraftoss_core::nbt::Tag;
use serde_json::{Map, Value};

pub const BANNER_PATTERNS: &str = "minecraft:banner_patterns";
pub const BASE_COLOR: &str = "minecraft:base_color";
pub const CHARGED_PROJECTILES: &str = "minecraft:charged_projectiles";
pub const FIREWORKS: &str = "minecraft:fireworks";
pub const FIREWORK_EXPLOSION: &str = "minecraft:firework_explosion";
pub const COMPONENTS: [&str; 5] = [
    BANNER_PATTERNS,
    BASE_COLOR,
    CHARGED_PROJECTILES,
    FIREWORKS,
    FIREWORK_EXPLOSION,
];
pub const DYES: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];
pub const SHAPES: [&str; 5] = ["small_ball", "large_ball", "star", "creeper", "burst"];
pub const MAX_PROJECTILES: usize = 1024;
pub const MAX_EXPLOSIONS: usize = 256;
pub const MAX_STACK: i64 = 99;
pub const MAX_FLIGHT: i64 = 255;

pub fn content(component: &str, snbt: &str) -> Result<Value, String> {
    let tag = minecraftoss_core::snbt::parse(snbt)?;
    match component {
        BANNER_PATTERNS => banner_patterns(&tag),
        BASE_COLOR => named(&tag, &DYES),
        CHARGED_PROJECTILES => charged_projectiles(&tag),
        FIREWORKS => fireworks(&tag),
        FIREWORK_EXPLOSION => explosion(&tag),
        _ => Err(format!("Not a listed item component: {component}")),
    }
}

pub fn banner_patterns(tag: &Tag) -> Result<Value, String> {
    let layers = list(tag, None)?.iter().map(layer);
    Ok(Value::Array(layers.collect::<Result<_, _>>()?))
}

pub fn charged_projectiles(tag: &Tag) -> Result<Value, String> {
    let projectiles = list(tag, Some(MAX_PROJECTILES))?.iter().map(projectile);
    Ok(Value::Array(projectiles.collect::<Result<_, _>>()?))
}

pub fn fireworks(tag: &Tag) -> Result<Value, String> {
    compound(tag)?;
    let mut out = Map::new();
    if let Some(flight) = tag.get("flight_duration") {
        let flight = in_range(number(flight)?, 0, MAX_FLIGHT)?;
        if flight != 0 {
            out.insert("flight_duration".to_owned(), Value::from(flight));
        }
    }
    if let Some(explosions) = tag.get("explosions") {
        let explosions = list(explosions, Some(MAX_EXPLOSIONS))?
            .iter()
            .map(explosion)
            .collect::<Result<Vec<_>, _>>()?;
        if !explosions.is_empty() {
            out.insert("explosions".to_owned(), Value::Array(explosions));
        }
    }
    Ok(Value::Object(out))
}

pub fn explosion(tag: &Tag) -> Result<Value, String> {
    compound(tag)?;
    let mut out = Map::new();
    out.insert("shape".to_owned(), named(field(tag, "shape")?, &SHAPES)?);
    for key in ["colors", "fade_colors"] {
        let Some(colors) = tag.get(key) else {
            continue;
        };
        let colors = colors
            .as_ints()
            .ok_or_else(|| format!("Not a list of ints: {}", describe(colors)))?;
        if !colors.is_empty() {
            out.insert(key.to_owned(), Value::from(colors));
        }
    }
    for key in ["has_trail", "has_twinkle"] {
        if tag
            .get(key)
            .map(number)
            .transpose()?
            .is_some_and(|flag| flag != 0)
        {
            out.insert(key.to_owned(), Value::from(1));
        }
    }
    Ok(Value::Object(out))
}

pub fn identifier(tag: &Tag) -> Result<Value, String> {
    let Tag::String(text) = tag else {
        return Err(format!("Not a string: {}", describe(tag)));
    };
    let (namespace, path) = match text.split_once(':') {
        Some(("", path)) => ("minecraft", path),
        Some(split) => split,
        None => ("minecraft", text.as_str()),
    };
    let location = format!("{namespace}:{path}");
    if !namespace
        .chars()
        .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '.' | '-'))
    {
        return Err(format!(
            "Non [a-z0-9_.-] character in namespace of location: {location}"
        ));
    }
    if path.is_empty()
        || !path
            .chars()
            .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '.' | '-' | '/'))
    {
        return Err(format!(
            "Non [a-z0-9/._-] character in path of location: {location}"
        ));
    }
    Ok(Value::from(location))
}

fn layer(tag: &Tag) -> Result<Value, String> {
    compound(tag)?;
    let pattern = field(tag, "pattern")?;
    let pattern = match pattern {
        Tag::Compound(_) => {
            let mut inline = Map::new();
            inline.insert(
                "asset_id".to_owned(),
                identifier(field(pattern, "asset_id")?)?,
            );
            inline.insert(
                "translation_key".to_owned(),
                string(field(pattern, "translation_key")?, usize::MAX)?,
            );
            Value::Object(inline)
        }
        _ => identifier(pattern)?,
    };
    let mut out = Map::new();
    out.insert("pattern".to_owned(), pattern);
    out.insert("color".to_owned(), named(field(tag, "color")?, &DYES)?);
    Ok(Value::Object(out))
}

fn projectile(tag: &Tag) -> Result<Value, String> {
    let id = match tag {
        Tag::String(_) => identifier(tag)?,
        _ => {
            compound(tag)?;
            identifier(field(tag, "id")?)?
        }
    };
    if id == "minecraft:air" {
        return Err("Item must be non-empty".to_owned());
    }
    let mut out = Map::new();
    out.insert("id".to_owned(), id);
    if let Some(count) = tag.get("count") {
        let count = in_range(number(count)?, 1, MAX_STACK)?;
        if count != 1 {
            out.insert("count".to_owned(), Value::from(count));
        }
    }
    if let Some(components) = tag.get("components") {
        compound(components)?;
        if components.as_compound().is_some_and(|map| !map.is_empty()) {
            out.insert("components".to_owned(), tag_json(components));
        }
    }
    Ok(Value::Object(out))
}

fn named(tag: &Tag, names: &[&str]) -> Result<Value, String> {
    match tag {
        Tag::String(name) if names.contains(&name.as_str()) => Ok(Value::from(name.clone())),
        Tag::String(name) => Err(format!("Unknown element name:{name}")),
        _ => Err(format!("Not a string: {}", describe(tag))),
    }
}

fn compound(tag: &Tag) -> Result<(), String> {
    match tag {
        Tag::Compound(_) => Ok(()),
        _ => Err(format!("Not a map: {}", describe(tag))),
    }
}

fn field<'a>(tag: &'a Tag, key: &str) -> Result<&'a Tag, String> {
    tag.get(key).ok_or_else(|| format!("No key {key}"))
}

fn in_range(value: i64, min: i64, max: i64) -> Result<i64, String> {
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(format!("Value {value} outside of range [{min}:{max}]"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parsed(component: &str, snbt: &str) -> Value {
        content(component, snbt).unwrap()
    }

    fn refused(component: &str, snbt: &str) -> String {
        content(component, snbt).unwrap_err()
    }

    #[test]
    fn banner_layers_take_ids_with_or_without_a_namespace_and_inline_patterns() {
        assert_eq!(
            parsed(
                BANNER_PATTERNS,
                r#"[{pattern:"minecraft:creeper",color:"black"},{pattern:border,color:red}]"#
            ),
            json!([
                {"pattern": "minecraft:creeper", "color": "black"},
                {"pattern": "minecraft:border", "color": "red"}
            ])
        );
        assert_eq!(
            parsed(
                BANNER_PATTERNS,
                r#"[{pattern:{asset_id:"globe",translation_key:"block.minecraft.banner.globe"},color:"light_blue"}]"#
            ),
            json!([{
                "pattern": {"asset_id": "minecraft:globe", "translation_key": "block.minecraft.banner.globe"},
                "color": "light_blue"
            }])
        );
        assert_eq!(parsed(BANNER_PATTERNS, "[]"), json!([]));
    }

    #[test]
    fn a_base_colour_is_a_dye_name_quoted_or_not() {
        assert_eq!(parsed(BASE_COLOR, "red"), json!("red"));
        assert_eq!(parsed(BASE_COLOR, "'light_gray'"), json!("light_gray"));
        assert_eq!(refused(BASE_COLOR, "purpl"), "Unknown element name:purpl");
        assert!(refused(BASE_COLOR, "3").starts_with("Not a string"));
    }

    #[test]
    fn charged_projectiles_are_item_templates_with_a_count_of_one_left_out() {
        assert_eq!(
            parsed(CHARGED_PROJECTILES, r#"[{id:"minecraft:arrow"}]"#),
            json!([{"id": "minecraft:arrow"}])
        );
        assert_eq!(
            parsed(
                CHARGED_PROJECTILES,
                r#"[{id:"spectral_arrow",count:1},{id:"arrow",count:3b},"tipped_arrow"]"#
            ),
            json!([
                {"id": "minecraft:spectral_arrow"},
                {"id": "minecraft:arrow", "count": 3},
                {"id": "minecraft:tipped_arrow"}
            ])
        );
        assert_eq!(
            parsed(
                CHARGED_PROJECTILES,
                r#"[{id:"firework_rocket",components:{"minecraft:fireworks":{flight_duration:2b}}}]"#
            ),
            json!([{
                "id": "minecraft:firework_rocket",
                "components": {"minecraft:fireworks": {"flight_duration": 2}}
            }])
        );
        assert_eq!(
            refused(CHARGED_PROJECTILES, r#"[{id:"arrow",count:100}]"#),
            "Value 100 outside of range [1:99]"
        );
        assert_eq!(
            refused(CHARGED_PROJECTILES, r#"[{id:"air"}]"#),
            "Item must be non-empty"
        );
        assert_eq!(refused(CHARGED_PROJECTILES, r#"[{count:2}]"#), "No key id");
        assert!(refused(CHARGED_PROJECTILES, r#"[{id:"Arrow"}]"#).starts_with("Non [a-z0-9/._-]"));
    }

    #[test]
    fn fireworks_keep_their_flight_and_stars_in_the_saved_shape() {
        assert_eq!(
            parsed(
                FIREWORKS,
                "{flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}"
            ),
            json!({
                "flight_duration": 1,
                "explosions": [{"shape": "large_ball", "colors": [11743532], "has_trail": 1}]
            })
        );
        assert_eq!(
            parsed(FIREWORKS, "{flight_duration:0b,explosions:[]}"),
            json!({})
        );
        assert_eq!(
            refused(FIREWORKS, "{flight_duration:256}"),
            "Value 256 outside of range [0:255]"
        );
        assert!(refused(FIREWORKS, "{explosions:{shape:star}}").starts_with("Not a list"));
        let many = vec!["{shape:star}"; MAX_EXPLOSIONS + 1].join(",");
        assert!(
            refused(FIREWORKS, &format!("{{explosions:[{many}]}}")).starts_with("List is too long")
        );
    }

    #[test]
    fn a_firework_star_takes_its_shape_colours_and_flags() {
        assert_eq!(
            parsed(
                FIREWORK_EXPLOSION,
                "{shape:\"star\",colors:[1,2],fade_colors:[I;3],has_trail:false,has_twinkle:true}"
            ),
            json!({"shape": "star", "colors": [1, 2], "fade_colors": [3], "has_twinkle": 1})
        );
        assert_eq!(
            parsed(FIREWORK_EXPLOSION, "{shape:burst,colors:[]}"),
            json!({"shape": "burst"})
        );
        assert_eq!(
            refused(FIREWORK_EXPLOSION, "{shape:square}"),
            "Unknown element name:square"
        );
        assert_eq!(refused(FIREWORK_EXPLOSION, "{colors:[1]}"), "No key shape");
        assert!(
            refused(FIREWORK_EXPLOSION, "{shape:star,colors:[\"red\"]}")
                .starts_with("Not a list of ints")
        );
        assert!(refused(FIREWORK_EXPLOSION, "[{shape:star}]").starts_with("Not a map"));
    }

    #[test]
    fn malformed_lists_and_layers_are_refused() {
        assert!(
            refused(BANNER_PATTERNS, r#"{pattern:"creeper",color:"black"}"#)
                .starts_with("Not a list")
        );
        assert_eq!(
            refused(BANNER_PATTERNS, r#"[{pattern:"creeper"}]"#),
            "No key color"
        );
        assert_eq!(
            refused(BANNER_PATTERNS, r#"[{pattern:"creeper",color:"grey"}]"#),
            "Unknown element name:grey"
        );
        assert!(refused(BANNER_PATTERNS, r#"["creeper"]"#).starts_with("Not a map"));
        assert!(
            refused(BANNER_PATTERNS, r#"[{pattern:"creeper",color:"black"}"#)
                .contains("at position")
        );
        assert!(refused("minecraft:custom_data", "{}").starts_with("Not a listed"));
    }
}
