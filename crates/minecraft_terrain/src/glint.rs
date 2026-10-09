use anyhow::Result;
use image::RgbaImage;
use minecraftoss_player::inventory::ItemStack;
use serde_json::Value;

use crate::pack::{PackStack, ResourceId};

pub const ITEM_GLINT_TEXTURE: &str = "minecraft:misc/enchanted_glint_item";
pub const ARMOR_GLINT_TEXTURE: &str = "minecraft:misc/enchanted_glint_armor";

pub const DEFAULT_GLINT: [&str; 7] = [
    "minecraft:enchanted_golden_apple",
    "minecraft:experience_bottle",
    "minecraft:written_book",
    "minecraft:nether_star",
    "minecraft:enchanted_book",
    "minecraft:end_crystal",
    "minecraft:debug_stick",
];

const OVERRIDE: &str = "minecraft:enchantment_glint_override";
const REMOVED_OVERRIDE: &str = "!minecraft:enchantment_glint_override";

pub fn has_glint(stack: &ItemStack) -> bool {
    let components = stack.components.as_ref().and_then(Value::as_object);
    let component = |key: &str| components.and_then(|map| map.get(key));
    if let Some(glint) = component(OVERRIDE).and_then(Value::as_bool) {
        return glint;
    }
    if DEFAULT_GLINT.contains(&stack.id.as_str()) && component(REMOVED_OVERRIDE).is_none() {
        return true;
    }
    let enchanted = component("minecraft:enchantments")
        .and_then(Value::as_object)
        .is_some_and(|map| !map.is_empty());
    enchanted
        || (stack.id == "minecraft:compass" && component("minecraft:lodestone_tracker").is_some())
}

pub fn glint_texture(packs: &PackStack) -> Result<Option<RgbaImage>> {
    texture_image(packs, ITEM_GLINT_TEXTURE)
}

pub fn armor_glint_texture(packs: &PackStack) -> Result<Option<RgbaImage>> {
    texture_image(packs, ARMOR_GLINT_TEXTURE)
}

fn texture_image(packs: &PackStack, texture: &str) -> Result<Option<RgbaImage>> {
    let id = ResourceId::parse(texture)?;
    packs
        .texture(&id)?
        .map(|bytes| {
            Ok(image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8())
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn stack(id: &str, components: Option<Value>) -> ItemStack {
        ItemStack {
            components,
            ..ItemStack::new(id, 1)
        }
    }

    #[test]
    fn enchantments_make_an_item_glint() {
        assert!(!has_glint(&stack("minecraft:diamond_pickaxe", None)));
        assert!(has_glint(&stack(
            "minecraft:diamond_pickaxe",
            Some(json!({"minecraft:enchantments": {"minecraft:silk_touch": 1}}))
        )));
        assert!(!has_glint(&stack(
            "minecraft:diamond_pickaxe",
            Some(json!({"minecraft:enchantments": {}}))
        )));
        assert!(!has_glint(&stack(
            "minecraft:book",
            Some(json!({"minecraft:stored_enchantments": {"minecraft:mending": 1}}))
        )));
    }

    #[test]
    fn vanilla_foil_items_glint_unless_overridden() {
        assert!(has_glint(&stack(
            "minecraft:enchanted_book",
            Some(json!({"minecraft:stored_enchantments": {"minecraft:mending": 1}}))
        )));
        assert!(!has_glint(&stack(
            "minecraft:enchanted_book",
            Some(json!({"minecraft:enchantment_glint_override": false}))
        )));
        for id in DEFAULT_GLINT {
            assert!(has_glint(&stack(id, None)), "{id}");
        }
        assert!(!has_glint(&stack("minecraft:golden_apple", None)));
        assert!(!has_glint(&stack("minecraft:potion", None)));
        assert!(has_glint(&stack(
            "minecraft:diamond_sword",
            Some(json!({"minecraft:enchantment_glint_override": true}))
        )));
        assert!(!has_glint(&stack(
            "minecraft:nether_star",
            Some(json!({"!minecraft:enchantment_glint_override": {}}))
        )));
    }

    #[test]
    fn a_lodestone_compass_glints() {
        assert!(!has_glint(&stack("minecraft:compass", None)));
        assert!(has_glint(&stack(
            "minecraft:compass",
            Some(json!({"minecraft:lodestone_tracker": {"tracked": false}}))
        )));
        assert!(!has_glint(&stack(
            "minecraft:clock",
            Some(json!({"minecraft:lodestone_tracker": {"tracked": false}}))
        )));
    }

    #[test]
    fn the_vanilla_item_glint_is_128_pixels_square() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let glint = glint_texture(&packs).unwrap().unwrap();
        assert_eq!(glint.dimensions(), (128, 128));
        assert!(glint.pixels().all(|p| p[3] == 255));
        assert!(glint.pixels().any(|p| p[0] > 0));
        let armor = armor_glint_texture(&packs).unwrap().unwrap();
        assert_eq!(armor.dimensions(), (128, 128));
        assert_ne!(armor, glint);
    }

    #[test]
    fn the_armour_glint_comes_from_its_own_texture() {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let path = root.join("assets/minecraft/textures/misc/enchanted_glint_armor.png");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        RgbaImage::from_pixel(4, 4, image::Rgba([10, 0, 30, 255]))
            .save(path)
            .unwrap();
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let armor = armor_glint_texture(&packs).unwrap().unwrap();
        assert_eq!(armor.get_pixel(1, 1).0, [10, 0, 30, 255]);
        assert!(glint_texture(&packs).unwrap().is_none());
    }
}
