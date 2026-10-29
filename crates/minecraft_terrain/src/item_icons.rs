//! Inventory icons and names for items, as MinecraftOSS's viewer makes them
//! (`engine/viewer/src/interface.rs`: `load_catalog_icons`, `item_name`).
use crate::interface::{item_gui_model_reference, item_look_tints};
use crate::pack::{PackStack, ResourceId};
use anyhow::Result;
use image::{imageops::FilterType, RgbaImage};
use std::collections::HashMap;

/// An item's GUI icon: its block model rastered, its generated layers, or
/// its flat texture.
pub fn item_icon(packs: &PackStack, key: &str, icon_size: usize) -> Result<Option<RgbaImage>> {
    item_icon_at(packs, key, icon_size, &crate::item_icon::SpriteClock::at(0))
}

pub fn item_icon_at(
    packs: &PackStack,
    key: &str,
    icon_size: usize,
    clock: &crate::item_icon::SpriteClock,
) -> Result<Option<RgbaImage>> {
    let (key, potion) = crate::item_tint::split(key);
    let (key, look) = crate::item_look::split(key);
    let look = crate::item_look::ItemLook::parse(look);
    let (key, dial) = crate::dials::split(key);
    if dial.is_some() {
        clock.mark_animated();
    }
    let id = ResourceId::parse(key)?;
    let definition = packs.item_definition(&id)?;
    let chosen = definition.as_ref().and_then(|value| look.leaf(&value["model"], Some("gui")));
    let model = dial.or(chosen).or_else(|| definition.as_ref().and_then(|value| item_gui_model_reference(&value["model"])));
    let tints = definition
        .as_ref()
        .map(|value| item_look_tints(packs, &value["model"], potion, &look))
        .transpose()?
        .unwrap_or_default();
    if definition.as_ref().is_some_and(|value| crate::special_models::uses_item_geometry(&value["model"])) {
        if let Some(geometry) = crate::model::resolve_item_model_with(packs, &id, None, &look)? {
            let gui = crate::model::item_display_transform(packs, &id, "gui")?;
            let front_light = crate::model::item_gui_light_front(packs, &id)?;
            if let Some(icon) = crate::item_icon::model_icon(packs, &geometry, gui, front_light, &tints, icon_size, clock)? {
                return Ok(Some(icon));
            }
        }
    }
    if let Some(icon) = model
        .map(|model| crate::item_icon::block_icon(packs, model, &tints, icon_size, clock))
        .transpose()?
        .flatten()
    {
        return Ok(Some(icon));
    }
    let turned = model.and_then(|_| crate::model::resolve_item_model(packs, &id).ok().flatten()).filter(|geometry| geometry.elements.iter().any(|element| element.rotation.is_some()));
    if let Some(geometry) = turned {
        let gui = crate::model::item_display_transform(packs, &id, "gui")?;
        let front_light = crate::model::item_gui_light_front(packs, &id)?;
        if let Some(icon) = crate::item_icon::model_icon(packs, &geometry, gui, front_light, &tints, icon_size, clock)? {
            return Ok(Some(icon));
        }
    }
    if let Some(icon) = model
        .map(|model| generated_item_icon(packs, model, &tints, icon_size, clock))
        .transpose()?
        .flatten()
    {
        return Ok(Some(icon));
    }
    let texture = model.map(|model| model_texture(packs, model)).transpose()?.flatten();
    let texture = texture.or_else(|| ResourceId::parse(&format!("{}:item/{}", id.namespace, id.path)).ok());
    let Some(texture) = texture else { return Ok(None) };
    let Some(bytes) = packs.texture(&texture)? else { return Ok(None) };
    let decoded = clock.sprite(packs, &texture, &bytes)?;
    let side = decoded.width().min(decoded.height());
    let first = image::imageops::crop_imm(&decoded, 0, 0, side, side).to_image();
    Ok(Some(image::imageops::resize(&first, icon_size as u32, icon_size as u32, FilterType::Nearest)))
}

/// The pack's English names (`lang/en_us.json`).
pub fn language(packs: &PackStack) -> Result<HashMap<String, String>> {
    let id = ResourceId::parse("minecraft:lang/en_us")?;
    let Some(value) = packs.json(&id, "lang/en_us.json")? else {
        let mut language = HashMap::new();
        crate::fork_items::add_names(&mut language);
        return Ok(language);
    };
    let mut language: HashMap<String, String> = value
        .as_object()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(key, text)| Some((key.clone(), text.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default();
    crate::fork_items::add_names(&mut language);
    Ok(language)
}

/// `item_name`: the item's or its block's name, else its id's path.
pub fn item_name(language: &HashMap<String, String>, id: &str) -> String {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    let path = path.split(':').next().unwrap_or(path);
    language
        .get(&format!("item.{namespace}.{path}"))
        .or_else(|| language.get(&format!("block.{namespace}.{path}")))
        .cloned()
        .or_else(|| crate::fork_items::get(id).map(|row| row.name.to_owned()))
        .unwrap_or_else(|| path.replace('_', " "))
}


/// Item/generated renders each layer in order; layer 0's tint is the liquid,
/// and the untinted bottle on layer 1 is drawn over it.
fn generated_item_icon(
    packs: &PackStack,
    model: &str,
    tints: &[[u8; 3]],
    icon_size: usize,
    clock: &crate::item_icon::SpriteClock,
) -> Result<Option<RgbaImage>> {
    let mut textures = HashMap::<String, String>::new();
    let mut current = Some(ResourceId::parse(model)?);
    for _ in 0..12 {
        let Some(id) = current.take() else { break };
        let Some(value) = packs.model(&id)? else {
            break;
        };
        if let Some(entries) = value.get("textures").and_then(serde_json::Value::as_object) {
            for (key, texture) in entries {
                if let Some(texture) = texture
                    .as_str()
                    .or_else(|| texture.get("sprite").and_then(serde_json::Value::as_str))
                {
                    textures
                        .entry(key.clone())
                        .or_insert_with(|| texture.into());
                }
            }
        }
        current = value
            .get("parent")
            .and_then(serde_json::Value::as_str)
            .map(ResourceId::parse)
            .transpose()?;
    }
    if !textures.contains_key("layer1") {
        return Ok(None);
    }
    let mut icon = RgbaImage::new(icon_size as u32, icon_size as u32);
    for layer in 0..16 {
        let key = format!("layer{layer}");
        let Some(mut texture) = textures.get(&key).map(String::as_str) else {
            break;
        };
        for _ in 0..8 {
            if let Some(reference) = texture.strip_prefix('#') {
                let Some(next) = textures.get(reference) else {
                    break;
                };
                texture = next;
            } else {
                break;
            }
        }
        let texture = ResourceId::parse(texture)?;
        let Some(bytes) = packs.texture(&texture)? else {
            continue;
        };
        let source = clock.sprite(packs, &texture, &bytes)?;
        let side = source.width().min(source.height());
        if side == 0 {
            continue;
        }
        let tint = tints.get(layer).copied().unwrap_or([255; 3]);
        let mut frame = image::imageops::crop_imm(&source, 0, 0, side, side).to_image();
        for pixel in frame.pixels_mut() {
            for channel in 0..3 {
                pixel[channel] = (pixel[channel] as u16 * tint[channel] as u16 / 255) as u8;
            }
        }
        let scaled = image::imageops::resize(
            &frame,
            icon_size as u32,
            icon_size as u32,
            FilterType::Nearest,
        );
        image::imageops::overlay(&mut icon, &scaled, 0, 0);
    }
    Ok((icon.pixels().any(|pixel| pixel[3] > 0)).then_some(icon))
}

fn model_texture(packs: &PackStack, model: &str) -> Result<Option<ResourceId>> {
    let mut textures = HashMap::<String, String>::new();
    let mut current = Some(ResourceId::parse(model)?);
    for _ in 0..12 {
        let Some(id) = current.take() else { break };
        let Some(value) = packs.model(&id)? else {
            break;
        };
        if let Some(entries) = value.get("textures").and_then(serde_json::Value::as_object) {
            for (key, texture) in entries {
                if let Some(texture) = texture
                    .as_str()
                    .or_else(|| texture.get("sprite").and_then(serde_json::Value::as_str))
                {
                    textures
                        .entry(key.clone())
                        .or_insert_with(|| texture.into());
                }
            }
        }
        current = value
            .get("parent")
            .and_then(serde_json::Value::as_str)
            .map(ResourceId::parse)
            .transpose()?;
    }
    for key in [
        "layer0", "all", "top", "up", "side", "front", "end", "particle",
    ] {
        let Some(mut texture) = textures.get(key).map(String::as_str) else {
            continue;
        };
        for _ in 0..8 {
            if let Some(reference) = texture.strip_prefix('#') {
                let Some(next) = textures.get(reference) else {
                    break;
                };
                texture = next;
            } else {
                return ResourceId::parse(texture).map(Some);
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn vanilla_spyglass_and_spear_icons_are_their_inventory_sprites() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        for (item, sprite) in [("minecraft:spyglass", "minecraft:item/spyglass"), ("minecraft:iron_spear", "minecraft:item/iron_spear")] {
            let icon = item_icon(&packs, item, 32).unwrap().unwrap();
            let bytes = packs.texture(&ResourceId::parse(sprite).unwrap()).unwrap().unwrap();
            let expected = image::imageops::resize(&image::load_from_memory(&bytes).unwrap().to_rgba8(), 32, 32, FilterType::Nearest);
            assert!(icon == expected, "{item} is drawn from {sprite}");
        }
    }

    #[test]
    fn loaded_crossbow_icons_show_their_arrow_or_rocket() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let mut crossbow = minecraftoss_player::inventory::ItemStack::new("minecraft:crossbow", 1);
        for (projectile, sprite) in [(None, "minecraft:item/crossbow_standby"), (Some("minecraft:arrow"), "minecraft:item/crossbow_arrow"), (Some("minecraft:firework_rocket"), "minecraft:item/crossbow_firework")] {
            crossbow.components = projectile.map(|id| serde_json::json!({"minecraft:charged_projectiles": [{"id": id}]}));
            let icon = item_icon(&packs, &crate::item_tint::of_stack(&crossbow), 32).unwrap().unwrap();
            let bytes = packs.texture(&ResourceId::parse(sprite).unwrap()).unwrap().unwrap();
            let expected = image::imageops::resize(&image::load_from_memory(&bytes).unwrap().to_rgba8(), 32, 32, FilterType::Nearest);
            assert!(icon == expected, "{projectile:?} is drawn from {sprite}");
        }
        let pulled = item_icon(&packs, &crate::item_look::keyed("minecraft:crossbow", &crate::item_look::ItemLook::of(None, Some(25))), 32).unwrap().unwrap();
        let bytes = packs.texture(&ResourceId::parse("minecraft:item/crossbow_pulling_2").unwrap()).unwrap().unwrap();
        assert!(pulled == image::imageops::resize(&image::load_from_memory(&bytes).unwrap().to_rgba8(), 32, 32, FilterType::Nearest));
    }

    #[test]
    fn potion_and_tipped_arrow_icons_take_the_stack_colour_on_layer_zero() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let healing = [0xF8u8, 0x24, 0x23];
        for (item, layer0, layer1) in [
            ("minecraft:potion", "minecraft:item/potion_overlay", "minecraft:item/potion"),
            ("minecraft:tipped_arrow", "minecraft:item/tipped_arrow_head", "minecraft:item/tipped_arrow_base"),
        ] {
            let sprite = |name: &str| {
                let bytes = packs.texture(&ResourceId::parse(name).unwrap()).unwrap().unwrap();
                image::imageops::resize(&image::load_from_memory(&bytes).unwrap().to_rgba8(), 16, 16, FilterType::Nearest)
            };
            let (under, over) = (sprite(layer0), sprite(layer1));
            let plain = item_icon(&packs, item, 16).unwrap().unwrap();
            let tinted = item_icon(&packs, &crate::item_tint::key(item, Some(0xF82423)), 16).unwrap().unwrap();
            assert!(plain != tinted, "{item} changes colour");
            let mut checked = 0;
            for (x, y, pixel) in under.enumerate_pixels() {
                if pixel[3] != 255 || over.get_pixel(x, y)[3] != 0 {
                    continue;
                }
                let expected: Vec<u8> = (0..3).map(|channel| (pixel[channel] as u16 * healing[channel] as u16 / 255) as u8).collect();
                assert_eq!(&tinted.get_pixel(x, y).0[..3], expected.as_slice(), "{item} at {x},{y}");
                checked += 1;
            }
            assert!(checked > 0, "{item} has a bare {layer0} pixel");
        }
    }

    #[test]
    fn a_red_leather_horse_armour_icon_is_red_on_layer_zero_and_plain_on_layer_one() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let sprite = |name: &str| {
            let bytes = packs.texture(&ResourceId::parse(name).unwrap()).unwrap().unwrap();
            image::imageops::resize(&image::load_from_memory(&bytes).unwrap().to_rgba8(), 16, 16, FilterType::Nearest)
        };
        let (under, over) = (sprite("minecraft:item/leather_horse_armor"), sprite("minecraft:item/leather_horse_armor_overlay"));
        let mut armour = minecraftoss_player::inventory::ItemStack::new("minecraft:leather_horse_armor", 1);
        let plain = item_icon(&packs, &crate::item_tint::of_stack(&armour), 16).unwrap().unwrap();
        armour.components = Some(serde_json::json!({"minecraft:dyed_color": 0xB02E26}));
        let red = item_icon(&packs, &crate::item_tint::of_stack(&armour), 16).unwrap().unwrap();
        assert!(plain != red);
        let (mut dyed, mut kept) = (0, 0);
        for (x, y, pixel) in under.enumerate_pixels() {
            let top = over.get_pixel(x, y);
            if top[3] == 255 {
                assert_eq!(red.get_pixel(x, y), top, "overlay at {x},{y}");
                assert_eq!(red.get_pixel(x, y), plain.get_pixel(x, y), "overlay at {x},{y}");
                kept += 1;
            } else if pixel[3] == 255 && top[3] == 0 {
                let expected: Vec<u8> = [0xB0u16, 0x2E, 0x26].iter().enumerate().map(|(channel, tint)| (pixel[channel] as u16 * tint / 255) as u8).collect();
                assert_eq!(&red.get_pixel(x, y).0[..3], expected.as_slice(), "armour at {x},{y}");
                dyed += 1;
            }
        }
        assert!(dyed > 0 && kept > 0, "{dyed} {kept}");
    }

    pub(crate) fn banner_pack() -> (crate::test_dir::TestDir, PackStack) {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("pack.mcmeta"), r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#).unwrap();
        for (name, contents) in [
            (
                "assets/minecraft/items/white_banner.json",
                r#"{"model":{"type":"minecraft:special","base":"minecraft:item/template_banner","model":{"type":"minecraft:banner","color":"white"},"transformation":{"scale":[0.6666667,-0.6666667,-0.6666667],"translation":[0.5,0.0,0.5]}}}"#,
            ),
            (
                "assets/minecraft/models/item/template_banner.json",
                r#"{"gui_light":"front","display":{"gui":{"rotation":[30,20,0],"translation":[0,-3.25,0],"scale":[0.5325,0.5325,0.5325]}}}"#,
            ),
        ] {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let sheets = root.join("assets/minecraft/textures/entity/banner");
        std::fs::create_dir_all(&sheets).unwrap();
        for (name, pixel) in [("banner_base", [120, 90, 60, 255]), ("base", [255, 255, 255, 255]), ("stripe", [255, 255, 255, 128])] {
            RgbaImage::from_pixel(64, 64, image::Rgba(pixel)).save(sheets.join(format!("{name}.png"))).unwrap();
        }
        let packs = PackStack::open(vec![root.into()]).unwrap();
        (temp, packs)
    }

    #[test]
    fn banner_icons_blend_their_pattern_layers_and_skip_unknown_patterns() {
        let (_temp, packs) = banner_pack();
        let look = |layers: &[(&str, &'static str)]| crate::item_look::ItemLook {
            layers: layers.iter().map(|&(asset, colour)| (asset.to_owned(), colour)).collect(),
            ..Default::default()
        };
        let icon = |look: crate::item_look::ItemLook| item_icon(&packs, &crate::item_look::keyed("minecraft:white_banner", &look), 32).unwrap().unwrap();
        let plain = icon(look(&[]));
        let striped = icon(look(&[("minecraft:stripe", "red")]));
        assert!(plain != striped);
        let alphas = |icon: &RgbaImage| icon.pixels().map(|pixel| pixel[3]).collect::<Vec<_>>();
        assert_eq!(alphas(&plain), alphas(&striped), "a half-clear layer blends over the flag and leaves no holes");
        let reddened = plain.pixels().zip(striped.pixels()).filter(|(before, after)| before != after).inspect(|(before, after)| {
            assert!(after[0] <= before[0] && after[1] < before[1] && after[1] > 0, "{before:?} {after:?}");
        }).count();
        assert!(reddened > 0);
        assert!(icon(look(&[("minecraft:stripe", "red"), ("minecraft:no_such_pattern", "blue")])) == striped, "a pattern with no sheet is skipped");
        assert!(icon(look(&[("minecraft:no_such_pattern", "blue"), ("minecraft:stripe", "red")])) == striped);
    }

    #[test]
    fn a_dropped_banner_draws_its_pattern_layers_in_their_colours() {
        let (_temp, packs) = banner_pack();
        let textures = ["banner_base", "base", "stripe"].map(|name| ResourceId::parse(&format!("minecraft:entity/banner/{name}")).unwrap());
        let atlas = crate::mesh::make_atlas(&packs, textures.to_vec()).unwrap();
        let scene = crate::scene::HandcraftedScene::new();
        let light = crate::lighting::SkyLight::build(&scene);
        let mesh = |stack: minecraftoss_player::inventory::ItemStack| {
            let mut items = minecraftoss_player::items::WorldItems::default();
            items.spawn(stack, glam::DVec3::new(0.5, 1.0, 0.5));
            crate::mesh::ItemVisuals::default().mesh(&items, &packs, &atlas, &light, 1.0).unwrap()
        };
        let mut banner = minecraftoss_player::inventory::ItemStack::new("minecraft:white_banner", 1);
        assert_eq!(mesh(banner.clone()).faces, 18, "pole, bar and flag");
        banner.components = Some(serde_json::json!({"minecraft:banner_patterns": [{"pattern": "minecraft:stripe", "color": "red"}]}));
        let striped = mesh(banner);
        assert_eq!(striped.faces, 24, "and one layer on the flag");
        for (vertices, [red, green, _]) in [(&striped.vertices[48..72], [249.0, 255.0, 254.0]), (&striped.vertices[72..], [176.0, 46.0, 38.0])] {
            for vertex in vertices {
                let ratio = vertex.color[1] / vertex.color[0];
                assert!((ratio - green / red).abs() < 1e-3, "{:?}", vertex.color);
            }
        }
    }
}
