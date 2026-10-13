use image::RgbaImage;
use image::imageops::FilterType;
use serde_json::json;

use crate::fork_items::{self, ForkItem, Icon};
use crate::mesh::{Atlas, AtlasPatch};
use crate::pack::{PackStack, ResourceId};

pub const SPRITE: u32 = 16;
const ATLAS_TILE: u32 = 64;
const ATLAS_LEVELS: u32 = 5;
const OPAQUE_FROM: u8 = 128;

fn drawn(row: &ForkItem) -> bool {
    !matches!(row.icon, Icon::Sprite { .. })
}

pub fn texture(id: &str) -> Option<ResourceId> {
    let row = fork_items::get(id).filter(|row| drawn(row))?;
    ResourceId::parse(&format!("iw4l:item/{}", row.path())).ok()
}

pub fn textures() -> Vec<ResourceId> {
    fork_items::rows()
        .iter()
        .filter_map(|row| texture(row.id))
        .collect()
}

fn png(image: &RgbaImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    let encoded = image.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    );
    if encoded.is_err() {
        bytes.clear();
    }
    bytes
}

fn definition(row: &ForkItem) -> Option<ResourceId> {
    ResourceId::parse(row.id).ok()
}

fn sprite_model(packs: &PackStack, item: &str, tint: Option<[u8; 3]>) -> Option<serde_json::Value> {
    let vanilla = ResourceId::parse(item).ok()?;
    let mut model = packs.item_definition(&vanilla).ok()??.get("model")?.clone();
    if let (Some(fields), Some([r, g, b])) = (model.as_object_mut(), tint) {
        let value = (i64::from(r) << 16) | (i64::from(g) << 8) | i64::from(b);
        fields.insert(
            "tints".to_owned(),
            json!([{"type": "minecraft:constant", "value": value}]),
        );
    }
    Some(model)
}

pub fn reserve(packs: &mut PackStack) {
    let blank = png(&RgbaImage::new(SPRITE, SPRITE));
    for row in fork_items::rows() {
        if let Icon::Sprite { item, tint } = row.icon {
            let (Some(model), Some(id)) = (sprite_model(packs, item, tint), definition(row)) else {
                continue;
            };
            let body = json!({ "model": model }).to_string().into_bytes();
            packs.put(&id.asset("items", ".json"), body);
        } else if let Some(texture) = texture(row.id) {
            packs.put(&texture.asset("textures", ".png"), blank.clone());
        }
    }
}

pub fn install(packs: &mut PackStack, id: &str, sprite: &RgbaImage) -> bool {
    let (Some(texture), Some(item)) = (texture(id), fork_items::get(id).and_then(definition))
    else {
        return false;
    };
    let model = json!({
        "parent": "minecraft:item/generated",
        "textures": {"layer0": texture.key()}
    });
    let item_definition = json!({
        "model": {"type": "minecraft:model", "model": texture.key()}
    });
    packs.put(&texture.asset("textures", ".png"), png(sprite));
    packs.put(
        &texture.asset("models", ".json"),
        model.to_string().into_bytes(),
    );
    packs.put(
        &item.asset("items", ".json"),
        item_definition.to_string().into_bytes(),
    );
    true
}

pub fn sprite(icon: &RgbaImage, flip: bool) -> RgbaImage {
    let mut out = RgbaImage::new(SPRITE, SPRITE);
    let (width, height) = icon.dimensions();
    if width == 0 || height == 0 {
        return out;
    }
    let scale = (SPRITE as f32 / width as f32).min(SPRITE as f32 / height as f32);
    let w = ((width as f32 * scale).round() as u32).clamp(1, SPRITE);
    let h = ((height as f32 * scale).round() as u32).clamp(1, SPRITE);
    let mut resized = image::imageops::resize(icon, w, h, FilterType::Triangle);
    if flip {
        image::imageops::flip_horizontal_in_place(&mut resized);
    }
    image::imageops::overlay(
        &mut out,
        &resized,
        i64::from((SPRITE - w) / 2),
        i64::from((SPRITE - h) / 2),
    );
    for pixel in out.pixels_mut() {
        if pixel[3] >= OPAQUE_FROM {
            pixel[3] = 255;
        } else {
            *pixel = image::Rgba([0, 0, 0, 0]);
        }
    }
    out
}

pub fn patch(atlas: &Atlas, id: &str, sprite: &RgbaImage) -> Option<AtlasPatch> {
    let texture = texture(id)?;
    if !atlas.contains(&texture) {
        return None;
    }
    let width = atlas.pixels.width() as f32;
    let [u0, v0, _, _] = atlas.region(&texture);
    let corner = |at: f32| (at * width - 0.5).round().max(0.0) as u32;
    let tile = image::imageops::resize(sprite, ATLAS_TILE, ATLAS_TILE, FilterType::Nearest);
    let levels = (0..ATLAS_LEVELS)
        .map(|level| {
            let size = ATLAS_TILE >> level;
            image::imageops::resize(&tile, size, size, FilterType::Triangle)
        })
        .collect();
    Some(AtlasPatch {
        origin: (corner(u0), corner(v0)),
        levels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packs() -> PackStack {
        let mut packs = PackStack::open(Vec::new()).unwrap();
        packs.put(
            "assets/minecraft/models/item/generated.json",
            br#"{"parent": "builtin/generated"}"#.to_vec(),
        );
        packs
    }

    fn block(x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        let mut image = RgbaImage::new(SPRITE, SPRITE);
        for dy in 0..h {
            for dx in 0..w {
                image.put_pixel(x + dx, y + dy, image::Rgba([200, 100, 50, 255]));
            }
        }
        image
    }

    #[test]
    fn guns_grenades_and_attachments_have_icon_textures_and_sprites_do_not() {
        assert_eq!(texture("iw4l:m4").unwrap().key(), "iw4l:item/m4");
        assert_eq!(
            texture("iw4l:frag_grenade").unwrap().key(),
            "iw4l:item/frag_grenade"
        );
        assert!(texture("iw4l:red_dot_sight").is_some());
        assert!(texture("iw4l:rifle_ammo").is_none());
        assert!(texture("iw4l:gun_barrel").is_none());
        assert!(texture("iw4l:rapid_fire").is_none());
        assert!(texture("minecraft:stick").is_none());
        let all = textures();
        assert!(all.len() > 70);
        assert!(all.iter().all(|id| id.namespace == "iw4l"));
    }

    #[test]
    fn a_fork_id_resolves_to_a_generated_model_with_its_icon_sprite() {
        let mut packs = packs();
        let id = ResourceId::parse("iw4l:m4").unwrap();
        reserve(&mut packs);
        assert!(
            crate::model::resolve_item_model(&packs, &id)
                .unwrap()
                .is_none()
        );
        let texture = texture("iw4l:m4").unwrap();
        let blank = packs.texture(&texture).unwrap().unwrap();
        let blank = image::load_from_memory(&blank).unwrap().to_rgba8();
        assert_eq!(blank.dimensions(), (SPRITE, SPRITE));
        assert!(install(&mut packs, "iw4l:m4", &block(6, 7, 4, 2)));
        let model = crate::model::resolve_item_model(&packs, &id)
            .unwrap()
            .unwrap();
        assert_eq!(model.elements.len(), 1 + 12);
        assert!(
            model
                .elements
                .iter()
                .flat_map(|element| &element.faces)
                .all(|face| face.texture == texture)
        );
        assert!(!install(&mut packs, "iw4l:rifle_ammo", &block(0, 0, 1, 1)));
        assert!(!install(&mut packs, "minecraft:stick", &block(0, 0, 1, 1)));
    }

    #[test]
    fn ammo_and_parts_use_their_vanilla_sprite_with_the_tint() {
        let mut packs = packs();
        for item in ["iron_nugget", "iron_bars"] {
            packs.put(
                &format!("assets/minecraft/items/{item}.json"),
                format!(r#"{{"model": {{"type": "minecraft:model", "model": "minecraft:item/{item}"}}}}"#)
                    .into_bytes(),
            );
            packs.put(
                &format!("assets/minecraft/models/item/{item}.json"),
                format!(r#"{{"parent": "minecraft:item/generated", "textures": {{"layer0": "minecraft:item/{item}"}}}}"#)
                    .into_bytes(),
            );
            packs.put(
                &format!("assets/minecraft/textures/item/{item}.png"),
                png(&block(7, 7, 2, 2)),
            );
        }
        reserve(&mut packs);
        let rifle = ResourceId::parse("iw4l:rifle_ammo").unwrap();
        let definition = packs.item_definition(&rifle).unwrap().unwrap();
        let Some(Icon::Sprite {
            tint: Some(copper), ..
        }) = fork_items::get("iw4l:rifle_ammo").map(|row| row.icon)
        else {
            panic!("rifle ammo is a tinted nugget");
        };
        let tints = crate::interface::item_model_tints(&packs, &definition["model"], None).unwrap();
        assert_eq!(tints, vec![copper]);
        let model = crate::model::resolve_item_model(&packs, &rifle)
            .unwrap()
            .unwrap();
        assert_eq!(
            model.elements[0].faces[0].texture.key(),
            "minecraft:item/iron_nugget"
        );
        assert_eq!(model.elements[0].faces[0].tint_index, Some(0));
        let barrel = ResourceId::parse("iw4l:gun_barrel").unwrap();
        let barrel = packs.item_definition(&barrel).unwrap().unwrap();
        assert!(barrel["model"].get("tints").is_none());
    }

    #[test]
    fn a_sprite_is_fitted_flipped_and_cut_out() {
        let mut wide = RgbaImage::new(64, 16);
        for x in 0..32 {
            for y in 0..16 {
                wide.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        wide.put_pixel(63, 0, image::Rgba([255, 255, 255, 40]));
        let fitted = sprite(&wide, false);
        assert_eq!(fitted.dimensions(), (SPRITE, SPRITE));
        assert!(
            fitted
                .pixels()
                .all(|pixel| pixel[3] == 0 || pixel[3] == 255)
        );
        assert_eq!(fitted.get_pixel(0, 8)[3], 255);
        assert_eq!(fitted.get_pixel(15, 8)[3], 0);
        assert_eq!(fitted.get_pixel(0, 0)[3], 0);
        let flipped = sprite(&wide, true);
        assert_eq!(flipped.get_pixel(15, 8)[3], 255);
        assert_eq!(flipped.get_pixel(0, 8)[3], 0);
    }

    #[test]
    fn a_patch_covers_the_reserved_atlas_tile_with_every_mip_level() {
        let mut packs = packs();
        reserve(&mut packs);
        let atlas = crate::mesh::make_atlas(&packs, textures()).unwrap();
        assert!(atlas.missing.is_empty(), "{:?}", atlas.missing);
        let icon = block(0, 0, 16, 16);
        let patch = patch(&atlas, "iw4l:frag_grenade", &icon).unwrap();
        let width = atlas.pixels.width() as f32;
        let [u0, v0, u1, _] = atlas.region(&texture("iw4l:frag_grenade").unwrap());
        assert_eq!(patch.origin.0 % ATLAS_TILE, 0);
        assert_eq!(patch.origin.1 % ATLAS_TILE, 0);
        assert!((patch.origin.0 as f32 + 0.5 - u0 * width).abs() < 0.01);
        assert!((patch.origin.1 as f32 + 0.5 - v0 * width).abs() < 0.01);
        assert!(((u1 - u0) * width - (ATLAS_TILE as f32 - 1.0)).abs() < 0.01);
        let sizes: Vec<u32> = patch.levels.iter().map(|level| level.width()).collect();
        assert_eq!(sizes, vec![64, 32, 16, 8, 4]);
        assert_eq!(patch.levels[0].get_pixel(0, 0)[3], 255);
        assert!(super::patch(&atlas, "iw4l:rifle_ammo", &icon).is_none());
    }
}
