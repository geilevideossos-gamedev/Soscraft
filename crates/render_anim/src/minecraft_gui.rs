use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use frame::{McFont, McGlyph, McGui, McScaling, McSprite};
use image::RgbaImage;
use minecraft_terrain::pack::{PackStack, ResourceId};
use serde_json::Value;

const ATLAS_WIDTH: u32 = 2048;
const GUTTER: u32 = 1;
const SPRITES: &str = "assets/minecraft/textures/gui/sprites/";

pub(crate) const CONTAINER_SHEETS: [&str; 16] = [
    "inventory",
    "crafting_table",
    "generic_54",
    "shulker_box",
    "furnace",
    "smoker",
    "blast_furnace",
    "brewing_stand",
    "enchanting_table",
    "anvil",
    "grindstone",
    "stonecutter",
    "smithing",
    "creative_inventory/tab_items",
    "creative_inventory/tab_item_search",
    "creative_inventory/tab_inventory",
];

const SPRITE_FOLDERS: [&str; 14] = [
    "container/slot/",
    "container/anvil/",
    "container/brewing_stand/",
    "container/enchanting_table/",
    "container/grindstone/",
    "container/smithing/",
    "container/stonecutter/",
    "container/furnace/",
    "container/smoker/",
    "container/blast_furnace/",
    "container/creative_inventory/",
    "hud/heart/",
    "tooltip/",
    "recipe_book/",
];

const LOOSE_SPRITES: [&str; 14] = [
    "container/slot",
    "container/slot_highlight_back",
    "container/slot_highlight_front",
    "boss_bar/pink_background",
    "boss_bar/pink_progress",
    "widget/page_forward",
    "widget/page_forward_highlighted",
    "widget/page_backward",
    "widget/page_backward_highlighted",
    "widget/button",
    "widget/button_highlighted",
    "widget/button_disabled",
    "widget/text_field",
    "widget/text_field_highlighted",
];

pub(crate) const BOOK_SHEET: &str = "book";
pub(crate) const RECIPE_BOOK_SHEET: &str = "recipe_book";

const TEXT_PREFIXES: [&str; 13] = [
    "container.",
    "itemGroup.",
    "inventory.",
    "item.modifiers.",
    "gui.experience.",
    "item.minecraft.smithing_template.",
    "multiplayer.downloadingTerrain",
    "multiplayer.stopSleeping",
    "book.",
    "lectern.",
    "gui.done",
    "gui.cancel",
    "gui.recipebook.",
];

pub(crate) const PUPPET_HEIGHT: f32 = 60.0 / 70.0;
pub(crate) const PUPPET_FEET: f32 = 31.875 / 70.0;

pub(crate) fn puppet_axes(forward: Vec3, right: Vec3, up: Vec3, yaw: f32, tilt: f32) -> [Vec3; 3] {
    let turn = -yaw.to_radians();
    let facing = (-forward * turn.cos() + right * turn.sin()).normalize_or(-forward);
    let lean = Quat::from_axis_angle((-right).normalize_or(Vec3::NEG_X), tilt.to_radians());
    let facing = lean * facing;
    let up = lean * up;
    [facing, up.cross(facing), up]
}

pub(crate) struct GuiAssets {
    pub gui: Arc<McGui>,
    pub words: Vec<String>,
    pub glint: Option<Handle<Image>>,
    pub generation: u64,
}

enum Piece {
    Sheet(String),
    Sprite(String, McScaling),
    Page(usize),
    White,
}

struct PendingGlyph {
    alt: bool,
    c: char,
    page: usize,
    cell: [u32; 4],
    scale: f32,
    ascent: i32,
    advance: f32,
}

#[derive(Default)]
struct FontPages {
    pages: Vec<RgbaImage>,
    glyphs: Vec<PendingGlyph>,
    spaces: [HashMap<char, f32>; 2],
    seen: std::collections::HashSet<(bool, char)>,
}

impl GuiAssets {
    pub(crate) fn load(
        packs: &PackStack,
        language: &HashMap<String, String>,
        jar: Option<&Path>,
        images: &mut Assets<Image>,
    ) -> Option<Self> {
        let (atlas, mut gui) = match build_atlas(packs, language) {
            Ok(built) => built,
            Err(error) => {
                diag::warn!(World, "Minecraft GUI textures unavailable: {error:#}");
                return None;
            }
        };
        let words = jar.map_or_else(Vec::new, |jar| {
            minecraft_terrain::enchantment_names::enchantment_names(jar).unwrap_or_else(|error| {
                diag::warn!(
                    World,
                    "Minecraft enchanting table words unavailable: {error:#}"
                );
                Vec::new()
            })
        });
        let image = Image {
            sampler: ImageSampler::nearest(),
            ..Image::new(
                Extent3d {
                    width: atlas.width(),
                    height: atlas.height(),
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                atlas.into_raw(),
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::default(),
            )
        };
        gui.image = images.add(image);
        let glint = glint_image(packs, images);
        Some(Self {
            gui: Arc::new(gui),
            words,
            glint,
            generation: packs.generation,
        })
    }
}

fn glint_image(packs: &PackStack, images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let glint = match minecraft_terrain::glint::glint_texture(packs) {
        Ok(Some(glint)) => glint,
        Ok(None) => {
            diag::warn!(World, "Minecraft enchantment glint texture missing");
            return None;
        }
        Err(error) => {
            diag::warn!(World, "Minecraft enchantment glint unavailable: {error:#}");
            return None;
        }
    };
    let image = Image {
        sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        }),
        ..Image::new(
            Extent3d {
                width: glint.width(),
                height: glint.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            glint.into_raw(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        )
    };
    Some(images.add(image))
}

fn decode(bytes: &[u8]) -> anyhow::Result<RgbaImage> {
    Ok(image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.to_rgba8())
}

fn build_atlas(
    packs: &PackStack,
    language: &HashMap<String, String>,
) -> anyhow::Result<(RgbaImage, McGui)> {
    let mut pieces: Vec<(Piece, RgbaImage)> = Vec::new();
    for sheet in CONTAINER_SHEETS {
        let id = ResourceId::parse(&format!("minecraft:gui/container/{sheet}"))?;
        let Some(bytes) = packs.texture(&id)? else {
            anyhow::bail!("missing textures/gui/container/{sheet}.png");
        };
        pieces.push((Piece::Sheet(sheet.to_owned()), decode(&bytes)?));
    }
    let id = ResourceId::parse(&format!("minecraft:gui/{BOOK_SHEET}"))?;
    let Some(bytes) = packs.texture(&id)? else {
        anyhow::bail!("missing textures/gui/{BOOK_SHEET}.png");
    };
    pieces.push((Piece::Sheet(BOOK_SHEET.to_owned()), decode(&bytes)?));
    let id = ResourceId::parse(&format!("minecraft:gui/{RECIPE_BOOK_SHEET}"))?;
    if let Some(bytes) = packs.texture(&id)? {
        pieces.push((Piece::Sheet(RECIPE_BOOK_SHEET.to_owned()), decode(&bytes)?));
    }
    let mut names: Vec<String> = LOOSE_SPRITES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    for folder in SPRITE_FOLDERS {
        names.extend(sprite_names(
            &packs.list("minecraft", &format!("textures/gui/sprites/{folder}"))?,
            folder,
        ));
    }
    names.extend(sprite_names(
        &packs.list("minecraft", "textures/gui/sprites/hud/")?,
        "hud/",
    ));
    names.sort();
    names.dedup();
    for name in names {
        let id = ResourceId::parse(&format!("minecraft:gui/sprites/{name}"))?;
        let Some(bytes) = packs.texture(&id)? else {
            continue;
        };
        let meta = packs.animation(&id).ok().flatten();
        let mut sprite = decode(&bytes)?;
        if let Some(frame) = meta.as_ref().and_then(|meta| meta.get("animation")) {
            let height = frame
                .get("height")
                .and_then(Value::as_u64)
                .map_or(sprite.width(), |h| h as u32)
                .min(sprite.height());
            sprite = image::imageops::crop_imm(&sprite, 0, 0, sprite.width(), height).to_image();
        }
        let scaling = sprite_scaling(meta.as_ref());
        pieces.push((Piece::Sprite(name, scaling), sprite));
    }
    let mut fonts = FontPages::default();
    load_font(packs, "minecraft:default", false, &mut fonts, 0)?;
    load_font(packs, "minecraft:alt", true, &mut fonts, 0)?;
    for (index, page) in std::mem::take(&mut fonts.pages).into_iter().enumerate() {
        pieces.push((Piece::Page(index), page));
    }
    pieces.push((
        Piece::White,
        RgbaImage::from_pixel(2, 2, image::Rgba([255; 4])),
    ));
    let sizes: Vec<[u32; 2]> = pieces
        .iter()
        .map(|(_, image)| [image.width(), image.height()])
        .collect();
    let (spots, height) = pack_rects(&sizes, ATLAS_WIDTH);
    let mut atlas = RgbaImage::new(ATLAS_WIDTH, height);
    let mut gui = McGui {
        size: [ATLAS_WIDTH, height],
        texts: language
            .iter()
            .filter(|(key, _)| TEXT_PREFIXES.iter().any(|prefix| key.starts_with(prefix)))
            .map(|(key, text)| (key.clone(), text.clone()))
            .collect(),
        ..McGui::default()
    };
    let mut page_origins = HashMap::new();
    for ((piece, image), [x, y]) in pieces.into_iter().zip(spots) {
        image::imageops::replace(&mut atlas, &image, i64::from(x), i64::from(y));
        match piece {
            Piece::Sheet(name) => {
                gui.sheets.insert(name, [x, y]);
            }
            Piece::Sprite(name, scaling) => {
                gui.sprites.insert(
                    name,
                    McSprite {
                        rect: [x, y, image.width(), image.height()],
                        scaling,
                    },
                );
            }
            Piece::Page(index) => {
                page_origins.insert(index, [x, y]);
            }
            Piece::White => gui.white = [x + 1, y + 1],
        }
    }
    let [mut default, mut alt] = [McFont::default(), McFont::default()];
    [default.spaces, alt.spaces] = std::mem::take(&mut fonts.spaces);
    for glyph in fonts.glyphs {
        let Some(&[px, py]) = page_origins.get(&glyph.page) else {
            continue;
        };
        let font = if glyph.alt { &mut alt } else { &mut default };
        font.glyphs.insert(
            glyph.c,
            McGlyph {
                rect: [
                    px + glyph.cell[0],
                    py + glyph.cell[1],
                    glyph.cell[2],
                    glyph.cell[3],
                ],
                scale: glyph.scale,
                ascent: glyph.ascent,
                advance: glyph.advance,
            },
        );
    }
    gui.font = default;
    gui.alt_font = alt;
    Ok((atlas, gui))
}

fn sprite_names(paths: &[String], folder: &str) -> Vec<String> {
    let prefix = format!("{SPRITES}{folder}");
    paths
        .iter()
        .filter_map(|path| path.strip_prefix(&prefix))
        .filter(|rest| !rest.contains('/'))
        .filter_map(|rest| rest.strip_suffix(".png"))
        .map(|rest| format!("{folder}{rest}"))
        .collect()
}

pub(crate) fn sprite_scaling(meta: Option<&Value>) -> McScaling {
    let Some(scaling) = meta
        .and_then(|meta| meta.get("gui"))
        .and_then(|gui| gui.get("scaling"))
    else {
        return McScaling::Stretch;
    };
    let number = |key: &str| {
        scaling
            .get(key)
            .and_then(Value::as_u64)
            .map_or(0, |n| n as u32)
    };
    match scaling.get("type").and_then(Value::as_str) {
        Some("tile") => McScaling::Tile {
            width: number("width"),
            height: number("height"),
        },
        Some("nine_slice") => {
            let border = match scaling.get("border") {
                Some(Value::Object(sides)) => {
                    let side = |key: &str| {
                        sides
                            .get(key)
                            .and_then(Value::as_u64)
                            .map_or(0, |n| n as u32)
                    };
                    [side("left"), side("top"), side("right"), side("bottom")]
                }
                Some(all) => [all.as_u64().map_or(0, |n| n as u32); 4],
                None => [0; 4],
            };
            McScaling::NineSlice {
                width: number("width"),
                height: number("height"),
                border,
                stretch_inner: scaling
                    .get("stretch_inner")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            }
        }
        _ => McScaling::Stretch,
    }
}

fn load_font(
    packs: &PackStack,
    id: &str,
    alt: bool,
    fonts: &mut FontPages,
    depth: u32,
) -> anyhow::Result<()> {
    if depth > 8 {
        return Ok(());
    }
    let resource = ResourceId::parse(id)?;
    let Some(definition) = packs.font(&resource)? else {
        anyhow::bail!("missing font {id}");
    };
    let providers = definition
        .get("providers")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, Clone::clone);
    for provider in providers {
        let filtered = provider
            .get("filter")
            .and_then(Value::as_object)
            .is_some_and(|filter| filter.values().any(|wanted| wanted.as_bool() == Some(true)));
        if filtered {
            continue;
        }
        match provider.get("type").and_then(Value::as_str) {
            Some("reference") => {
                if let Some(next) = provider.get("id").and_then(Value::as_str) {
                    load_font(packs, next, alt, fonts, depth + 1)?;
                }
            }
            Some("space") => {
                let Some(advances) = provider.get("advances").and_then(Value::as_object) else {
                    continue;
                };
                for (key, advance) in advances {
                    let (Some(c), Some(advance)) = (key.chars().next(), advance.as_f64()) else {
                        continue;
                    };
                    if fonts.seen.insert((alt, c)) {
                        fonts.spaces[usize::from(alt)].insert(c, advance as f32);
                    }
                }
            }
            Some("bitmap") => bitmap_provider(packs, &provider, alt, fonts)?,
            _ => {}
        }
    }
    Ok(())
}

fn bitmap_provider(
    packs: &PackStack,
    provider: &Value,
    alt: bool,
    fonts: &mut FontPages,
) -> anyhow::Result<()> {
    let Some(file) = provider.get("file").and_then(Value::as_str) else {
        return Ok(());
    };
    let id = ResourceId::parse(file.strip_suffix(".png").unwrap_or(file))?;
    let Some(bytes) = packs.texture(&id)? else {
        anyhow::bail!("missing font page {file}");
    };
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?;
    let luminance = !decoded.color().has_alpha();
    let page = decoded.to_rgba8();
    let rows: Vec<Vec<char>> =
        provider
            .get("chars")
            .and_then(Value::as_array)
            .map_or_else(Vec::new, |rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(|row| row.chars().collect())
                    .collect()
            });
    let columns = rows.first().map_or(0, Vec::len) as u32;
    if columns == 0 {
        return Ok(());
    }
    let cell_w = page.width() / columns;
    let cell_h = page.height() / rows.len() as u32;
    let height = provider.get("height").and_then(Value::as_i64).unwrap_or(8) as f32;
    let ascent = provider.get("ascent").and_then(Value::as_i64).unwrap_or(0) as i32;
    let scale = height / cell_h as f32;
    let index = fonts.pages.len();
    let mut any = false;
    for (row, chars) in rows.iter().enumerate() {
        for (column, &c) in chars.iter().enumerate() {
            if c == '\0' || !fonts.seen.insert((alt, c)) {
                continue;
            }
            let (x, y) = (column as u32 * cell_w, row as u32 * cell_h);
            let width = glyph_width(&page, [x, y, cell_w, cell_h], luminance);
            fonts.glyphs.push(PendingGlyph {
                alt,
                c,
                page: index,
                cell: [x, y, cell_w, cell_h],
                scale,
                ascent,
                advance: glyph_advance(width, scale),
            });
            any = true;
        }
    }
    if any {
        fonts.pages.push(page);
    }
    Ok(())
}

pub(crate) fn glyph_width(page: &RgbaImage, [x, y, w, h]: [u32; 4], luminance: bool) -> u32 {
    for column in (0..w).rev() {
        let lit = (0..h).any(|row| {
            let pixel = page.get_pixel(x + column, y + row);
            if luminance {
                pixel[0] != 0
            } else {
                pixel[3] != 0
            }
        });
        if lit {
            return column + 1;
        }
    }
    0
}

pub(crate) fn glyph_advance(width: u32, scale: f32) -> f32 {
    ((0.5 + f64::from(width as f32 * scale)) as i32 + 1) as f32
}

pub(crate) fn pack_rects(sizes: &[[u32; 2]], width: u32) -> (Vec<[u32; 2]>, u32) {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| {
        (
            std::cmp::Reverse(sizes[i][1]),
            std::cmp::Reverse(sizes[i][0]),
        )
    });
    let mut spots = vec![[0, 0]; sizes.len()];
    let (mut x, mut y, mut shelf) = (0, 0, 0);
    for i in order {
        let [w, h] = sizes[i];
        if x > 0 && x + w + GUTTER > width {
            x = 0;
            y += shelf + GUTTER;
            shelf = 0;
        }
        spots[i] = [x, y];
        x += w + GUTTER;
        shelf = shelf.max(h);
    }
    (spots, (y + shelf).max(1))
}

pub(crate) fn enchanting_runes(
    words: &[String],
    gui: &McGui,
    seed: i32,
    costs: &[u32],
) -> Vec<String> {
    let mut random = minecraftoss_player::rng::LegacyRandom::new(i64::from(seed) as u64);
    costs
        .iter()
        .map(|&cost| {
            if cost == 0 || words.is_empty() {
                return String::new();
            }
            let count = random.next_int(2) + 3;
            let mut text = String::new();
            for i in 0..count {
                if i != 0 {
                    text.push(' ');
                }
                text.push_str(&words[random.next_int(words.len() as u32) as usize]);
            }
            gui.alt_font
                .head_by_width(&text, 86 - gui.font.width(&cost.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_puppet_faces_the_mouse_and_leans_back_when_it_is_above() {
        let [forward, right, up] = [Vec3::NEG_Z, Vec3::X, Vec3::Y];
        let [facing, side, top] = puppet_axes(forward, right, up, 0.0, 0.0);
        assert!((facing - Vec3::Z).length() < 1e-6 && (top - Vec3::Y).length() < 1e-6);
        assert!((side - Vec3::Y.cross(Vec3::Z)).length() < 1e-6);
        let [facing, _, _] = puppet_axes(forward, right, up, -15.0, 0.0);
        assert!(facing.dot(right) > 0.2);
        let [facing, _, top] = puppet_axes(forward, right, up, 0.0, 15.0);
        assert!(top.dot(forward) > 0.2 && facing.dot(up) > 0.2);
        assert!((PUPPET_HEIGHT * 70.0 - 60.0).abs() < 1e-4);
    }

    #[test]
    fn the_real_gui_textures_and_fonts_load_into_one_atlas() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let mut language = HashMap::new();
        language.insert("container.crafting".to_owned(), "Crafting".to_owned());
        language.insert("block.minecraft.stone".to_owned(), "Stone".to_owned());
        language.insert("multiplayer.stopSleeping".to_owned(), "Leave Bed".to_owned());
        let (atlas, gui) = build_atlas(&packs, &language).unwrap();
        assert_eq!(atlas.width(), ATLAS_WIDTH);
        assert!(atlas.height() <= 2048);
        assert_eq!(gui.sheets.len(), CONTAINER_SHEETS.len() + 2);
        assert!(gui.sheets.contains_key("creative_inventory/tab_item_search"));
        assert_eq!(gui.sprites["container/creative_inventory/tab_top_selected_7"].rect[2..], [26, 32]);
        assert_eq!(gui.sprites["container/creative_inventory/tab_bottom_unselected_1"].rect[2..], [26, 32]);
        assert_eq!(gui.sprites["container/creative_inventory/scroller"].rect[2..], [12, 15]);
        assert_eq!(gui.sprites["hud/hotbar"].rect[2..], [182, 22]);
        assert_eq!(
            gui.sprites["tooltip/frame"].scaling,
            McScaling::NineSlice {
                width: 100,
                height: 100,
                border: [10; 4],
                stretch_inner: true
            }
        );
        for button in ["widget/button", "widget/button_highlighted"] {
            assert_eq!(
                gui.sprites[button].scaling,
                McScaling::NineSlice {
                    width: 200,
                    height: 20,
                    border: [3; 4],
                    stretch_inner: false
                }
            );
        }
        assert_eq!(gui.text("multiplayer.stopSleeping"), "Leave Bed");
        assert!(gui.sprites.contains_key("container/slot/shield"));
        assert!(gui.sprites.contains_key("hud/heart/full"));
        assert_eq!(gui.font.advance('A'), 6.0);
        assert_eq!(gui.font.advance('i'), 2.0);
        assert_eq!(gui.font.advance(' '), 4.0);
        assert_eq!(gui.font.glyphs[&'A'].ascent, 7);
        assert!(gui.alt_font.glyphs.contains_key(&'a'));
        assert_eq!(gui.text("container.crafting"), "Crafting");
        assert!(!gui.texts.contains_key("block.minecraft.stone"));
        let [wx, wy] = gui.white;
        assert_eq!(atlas.get_pixel(wx, wy).0, [255; 4]);
    }

    #[test]
    fn the_recipe_book_sheet_sprites_and_texts_load() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let language = HashMap::from([
            ("gui.recipebook.page".to_owned(), "%s/%s".to_owned()),
            ("gui.recipebook.search_hint".to_owned(), "Search...".to_owned()),
        ]);
        let (atlas, gui) = build_atlas(&packs, &language).unwrap();
        let [x, y] = gui.sheets[RECIPE_BOOK_SHEET];
        assert!(atlas.width() >= x + 256 && atlas.height() >= y + 256);
        for (name, size) in [
            ("recipe_book/button", [20, 18]),
            ("recipe_book/button_highlighted", [20, 18]),
            ("recipe_book/tab", [35, 27]),
            ("recipe_book/tab_selected", [35, 27]),
            ("recipe_book/slot_craftable", [25, 25]),
            ("recipe_book/slot_many_uncraftable", [25, 25]),
            ("recipe_book/filter_enabled", [26, 16]),
            ("recipe_book/page_forward", [12, 17]),
            ("recipe_book/crafting_overlay_disabled", [24, 24]),
        ] {
            assert_eq!(gui.sprites[name].rect[2..], size, "{name}");
        }
        assert!(matches!(gui.sprites["recipe_book/overlay_recipe"].scaling, McScaling::NineSlice { border: [4, 4, 4, 4], .. }));
        assert!(matches!(gui.sprites["widget/text_field"].scaling, McScaling::NineSlice { .. }));
        assert!(gui.sprites.contains_key("widget/text_field_highlighted"));
        assert_eq!(gui.format("gui.recipebook.page", &["2", "3"]), "2/3");
        assert_eq!(gui.text("gui.recipebook.search_hint"), "Search...");
    }

    #[test]
    fn the_book_sheet_and_page_sprites_load() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let language = HashMap::from([
            ("book.pageIndicator".to_owned(), "Page %1$s of %2$s".to_owned()),
            ("lectern.take_book".to_owned(), "Take Book".to_owned()),
            ("gui.done".to_owned(), "Done".to_owned()),
            ("gui.cancel".to_owned(), "Cancel".to_owned()),
            ("gui.yes".to_owned(), "Yes".to_owned()),
        ]);
        let (atlas, gui) = build_atlas(&packs, &language).unwrap();
        let [bx, by] = gui.sheets[BOOK_SHEET];
        assert!(atlas.width() >= bx + 256 && atlas.height() >= by + 256);
        let book = decode(&packs.texture(&ResourceId::parse("minecraft:gui/book").unwrap()).unwrap().unwrap()).unwrap();
        assert_eq!(atlas.get_pixel(bx + 96, by + 96), book.get_pixel(96, 96));
        for page in ["page_forward", "page_backward"] {
            assert_eq!(gui.sprites[&format!("widget/{page}")].rect[2..], [23, 13]);
            assert_eq!(gui.sprites[&format!("widget/{page}_highlighted")].rect[2..], [23, 13]);
        }
        assert_eq!(
            gui.sprites["widget/button"].scaling,
            McScaling::NineSlice {
                width: 200,
                height: 20,
                border: [3; 4],
                stretch_inner: false
            }
        );
        assert!(gui.sprites.contains_key("widget/button_highlighted"));
        assert_eq!(gui.format("book.pageIndicator", &["2", "3"]), "Page 2 of 3");
        assert_eq!(gui.text("lectern.take_book"), "Take Book");
        assert_eq!(gui.text("gui.done"), "Done");
        assert_eq!(gui.text("gui.cancel"), "Cancel");
        assert!(!gui.texts.contains_key("gui.yes"));
        assert!(gui.sprites.contains_key("widget/button_disabled"));
    }

    #[test]
    fn nine_slice_borders_read_as_one_number_or_four_sides() {
        let meta: Value = serde_json::from_str(r#"{"gui":{"scaling":{"type":"nine_slice","width":100,"height":100,"border":10,"stretch_inner":true}}}"#).unwrap();
        assert_eq!(
            sprite_scaling(Some(&meta)),
            McScaling::NineSlice {
                width: 100,
                height: 100,
                border: [10; 4],
                stretch_inner: true
            }
        );
        let meta: Value = serde_json::from_str(r#"{"gui":{"scaling":{"type":"nine_slice","width":24,"height":24,"border":{"left":1,"top":2,"right":3,"bottom":4}}}}"#).unwrap();
        assert_eq!(
            sprite_scaling(Some(&meta)),
            McScaling::NineSlice {
                width: 24,
                height: 24,
                border: [1, 2, 3, 4],
                stretch_inner: false
            }
        );
        let meta: Value =
            serde_json::from_str(r#"{"gui":{"scaling":{"type":"tile","width":8,"height":4}}}"#)
                .unwrap();
        assert_eq!(
            sprite_scaling(Some(&meta)),
            McScaling::Tile {
                width: 8,
                height: 4
            }
        );
        assert_eq!(sprite_scaling(None), McScaling::Stretch);
    }

    #[test]
    fn glyph_width_is_the_last_lit_column() {
        let mut page = RgbaImage::new(16, 8);
        page.put_pixel(2, 3, image::Rgba([255, 255, 255, 255]));
        page.put_pixel(9, 0, image::Rgba([255, 255, 255, 255]));
        page.put_pixel(13, 7, image::Rgba([255, 255, 255, 255]));
        assert_eq!(glyph_width(&page, [0, 0, 8, 8], false), 3);
        assert_eq!(glyph_width(&page, [8, 0, 8, 8], false), 6);
        assert_eq!(glyph_width(&RgbaImage::new(8, 8), [0, 0, 8, 8], false), 0);
    }

    #[test]
    fn advance_rounds_the_scaled_width_and_adds_one() {
        assert_eq!(glyph_advance(5, 1.0), 6.0);
        assert_eq!(glyph_advance(5, 0.5), 4.0);
        assert_eq!(glyph_advance(0, 1.0), 1.0);
    }

    #[test]
    fn packed_rects_never_overlap() {
        let sizes = [
            [256, 256],
            [9, 9],
            [182, 22],
            [2000, 10],
            [100, 100],
            [256, 256],
        ];
        let (spots, height) = pack_rects(&sizes, 2048);
        for (i, (a, sa)) in spots.iter().zip(sizes).enumerate() {
            assert!(a[0] + sa[0] <= 2048 && a[1] + sa[1] <= height);
            for (b, sb) in spots.iter().zip(sizes).skip(i + 1) {
                let apart = a[0] + sa[0] <= b[0]
                    || b[0] + sb[0] <= a[0]
                    || a[1] + sa[1] <= b[1]
                    || b[1] + sb[1] <= a[1];
                assert!(apart);
            }
        }
    }

    fn rune_gui() -> McGui {
        let glyph = McGlyph {
            rect: [0, 0, 8, 8],
            scale: 1.0,
            ascent: 7,
            advance: 6.0,
        };
        let mut gui = McGui::default();
        for c in ('a'..='z').chain('0'..='9') {
            gui.font.glyphs.insert(c, glyph);
            gui.alt_font.glyphs.insert(c, glyph);
        }
        gui.alt_font.spaces.insert(' ', 4.0);
        gui
    }

    #[test]
    fn runes_follow_the_java_random_and_fit_the_row() {
        let words: Vec<String> = ["the", "elder", "scrolls", "klaatu"]
            .iter()
            .map(|w| (*w).to_owned())
            .collect();
        let gui = rune_gui();
        let runes = enchanting_runes(&words, &gui, 1234, &[3, 0, 30]);
        assert_eq!(runes.len(), 3);
        assert!(runes[1].is_empty());
        assert!(gui.alt_font.width(&runes[0]) <= 86 - 6);
        assert!(gui.alt_font.width(&runes[2]) <= 86 - 12);
        assert_eq!(runes, enchanting_runes(&words, &gui, 1234, &[3, 0, 30]));
        let mut random = minecraftoss_player::rng::LegacyRandom::new(1234);
        let count = random.next_int(2) + 3;
        let first = &words[random.next_int(4) as usize];
        assert!(runes[0].starts_with(first.as_str()) || first.starts_with(runes[0].as_str()));
        assert!(count >= 3);
    }
}
