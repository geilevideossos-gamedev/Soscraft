use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use image::RgbaImage;

use crate::class_file::{self, ClassFile, Constant};
use crate::maps::{ClientMap, MAP_SIZE};
use crate::pack::{PackStack, ResourceId};

pub const MAP_COLOR_CLASS: &str = "net/minecraft/world/level/material/MapColor";
pub const DECORATION_TYPES_CLASS: &str =
    "net/minecraft/world/level/saveddata/maps/MapDecorationTypes";
pub const BACKGROUND: &str = "minecraft:map/map_background";
pub const CHECKERBOARD: &str = "minecraft:map/map_background_checkerboard";
pub const DECORATION_FOLDER: &str = "minecraft:map/decorations/";
pub const BORDER: usize = 7;
pub const SCALE: usize = 2;
pub const SIDE: usize = (MAP_SIZE + 2 * BORDER) * SCALE;
pub const BRIGHTNESS: [u32; 4] = [180, 220, 255, 135];
pub const COLOR_IDS: usize = 64;
const SPRITE_SCALE: f32 = 4.0;
const SPRITE_SHIFT: f32 = 0.125;

#[derive(Clone, Debug, PartialEq)]
pub struct MapPalette {
    colors: [Option<u32>; COLOR_IDS],
    names: BTreeMap<String, u8>,
}

impl Default for MapPalette {
    fn default() -> Self {
        Self {
            colors: [None; COLOR_IDS],
            names: BTreeMap::new(),
        }
    }
}

impl MapPalette {
    pub fn with_colors<'a>(colors: impl IntoIterator<Item = (u8, &'a str, u32)>) -> Self {
        let mut palette = Self::default();
        for (id, name, rgb) in colors {
            palette.set(id, name, rgb);
        }
        palette
    }

    fn set(&mut self, id: u8, name: &str, rgb: u32) {
        if let Some(slot) = self.colors.get_mut(usize::from(id)) {
            *slot = Some(rgb & 0xFF_FFFF);
            self.names.insert(name.to_owned(), id);
        }
    }

    pub fn from_class(class: &ClassFile) -> Result<Self> {
        let init = class
            .method("<clinit>")
            .and_then(|method| method.code.as_deref())
            .ok_or_else(|| anyhow!("{} has no static initialiser", class.name))?;
        let mut palette = Self::default();
        let mut ints: Vec<i32> = Vec::new();
        let mut made: Option<(i32, i32)> = None;
        for instruction in class_file::instructions(init)? {
            match instruction.op {
                class_file::ICONST_M1..=class_file::ICONST_5
                | class_file::BIPUSH
                | class_file::SIPUSH => ints.push(instruction.value),
                class_file::LDC | class_file::LDC_W => {
                    if let Some(Constant::Integer(value)) = class.constant(instruction.index) {
                        ints.push(*value);
                    }
                }
                class_file::NEW => ints.clear(),
                class_file::INVOKESPECIAL => {
                    let Some(member) = class.member(instruction.index) else {
                        continue;
                    };
                    if member.class == MAP_COLOR_CLASS
                        && member.name == "<init>"
                        && member.descriptor == "(II)V"
                        && ints.len() >= 2
                    {
                        made = Some((ints[ints.len() - 2], ints[ints.len() - 1]));
                    }
                    ints.clear();
                }
                class_file::PUTSTATIC => {
                    let Some(member) = class.member(instruction.index) else {
                        continue;
                    };
                    if let Some((id, rgb)) = made.take() {
                        let id = u8::try_from(id)
                            .ok()
                            .filter(|&id| usize::from(id) < COLOR_IDS)
                            .ok_or_else(|| anyhow!("map colour {} has id {id}", member.name))?;
                        palette.set(id, &member.name, rgb as u32);
                    }
                }
                _ => {}
            }
        }
        if palette.names.is_empty() {
            bail!("{} makes no map colours", class.name);
        }
        Ok(palette)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn id(&self, name: &str) -> Option<u8> {
        self.names.get(name).copied()
    }

    pub fn rgb(&self, id: u8) -> Option<u32> {
        self.colors.get(usize::from(id)).copied().flatten()
    }

    pub fn rgba(&self, packed: u8) -> [u8; 4] {
        let id = packed >> 2;
        match self.rgb(id) {
            Some(rgb) if id != 0 => shade(rgb, packed & 3),
            _ => [0; 4],
        }
    }
}

pub fn shade(rgb: u32, brightness: u8) -> [u8; 4] {
    let modifier = BRIGHTNESS[usize::from(brightness & 3)];
    let channel = |shift: u32| ((rgb >> shift & 0xFF) * modifier / 255).min(255) as u8;
    [channel(16), channel(8), channel(0), 255]
}

pub fn decoration_assets(class: &ClassFile) -> Result<Vec<(String, String)>> {
    let init = class
        .method("<clinit>")
        .and_then(|method| method.code.as_deref())
        .ok_or_else(|| anyhow!("{} has no static initialiser", class.name))?;
    let mut strings: Vec<String> = Vec::new();
    let mut assets = Vec::new();
    for instruction in class_file::instructions(init)? {
        match instruction.op {
            class_file::LDC | class_file::LDC_W => {
                if let Some(text) = class.string(instruction.index) {
                    strings.push(text.to_owned());
                }
            }
            class_file::INVOKESTATIC => {
                let registers = class.member(instruction.index).is_some_and(|member| {
                    member.class == DECORATION_TYPES_CLASS
                        && member.name == "register"
                        && member
                            .descriptor
                            .starts_with("(Ljava/lang/String;Ljava/lang/String;")
                });
                if registers && strings.len() >= 2 {
                    let asset = strings.pop().unwrap_or_default();
                    let id = strings.pop().unwrap_or_default();
                    assets.push((format!("minecraft:{id}"), asset));
                }
                strings.clear();
            }
            _ => {}
        }
    }
    if assets.is_empty() {
        bail!("{} registers no map decorations", class.name);
    }
    Ok(assets)
}

pub fn read_jar(jar: &Path) -> Result<(MapPalette, Vec<(String, String)>)> {
    let file =
        std::fs::File::open(jar).with_context(|| format!("cannot open {}", jar.display()))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut class = |name: &str| -> Result<ClassFile> {
        let mut entry = archive
            .by_name(&format!("{name}.class"))
            .with_context(|| format!("no {name} in the jar"))?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        ClassFile::parse(&bytes)
    };
    let palette = MapPalette::from_class(&class(MAP_COLOR_CLASS)?)?;
    let assets = decoration_assets(&class(DECORATION_TYPES_CLASS)?)?;
    Ok((palette, assets))
}

#[derive(Clone, Debug, Default)]
pub struct MapLook {
    pub palette: MapPalette,
    pub background: Option<RgbaImage>,
    pub checkerboard: Option<RgbaImage>,
    pub sprites: HashMap<String, RgbaImage>,
}

fn texture(packs: &PackStack, id: &str) -> Result<RgbaImage> {
    let resource = ResourceId::parse(id)?;
    let bytes = packs
        .texture(&resource)?
        .ok_or_else(|| anyhow!("no texture {id}"))?;
    Ok(
        image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .with_context(|| format!("texture {id}"))?
            .to_rgba8(),
    )
}

impl MapLook {
    pub fn load(packs: &PackStack, jar: Option<&Path>) -> (Self, Vec<String>) {
        let mut look = Self::default();
        let mut warnings = Vec::new();
        let read = jar
            .ok_or_else(|| anyhow!("no client jar"))
            .and_then(read_jar);
        let assets = match read {
            Ok((palette, assets)) => {
                look.palette = palette;
                assets
            }
            Err(error) => {
                warnings.push(format!("map colours and markers unavailable: {error:#}"));
                Vec::new()
            }
        };
        let mut keep = |result: Result<RgbaImage>| match result {
            Ok(image) => Some(image),
            Err(error) => {
                warnings.push(format!("{error:#}"));
                None
            }
        };
        look.background = keep(texture(packs, BACKGROUND));
        look.checkerboard = keep(texture(packs, CHECKERBOARD));
        for (kind, asset) in assets {
            if let Some(sprite) = keep(texture(packs, &format!("{DECORATION_FOLDER}{asset}"))) {
                look.sprites.insert(kind, sprite);
            }
        }
        (look, warnings)
    }
}

fn blend(out: &mut [u8], at: usize, src: [u8; 4]) {
    let alpha = u32::from(src[3]);
    if alpha == 0 {
        return;
    }
    let dst = &mut out[at..at + 4];
    let keep = 255 - alpha;
    for channel in 0..3 {
        dst[channel] =
            ((u32::from(src[channel]) * alpha + u32::from(dst[channel]) * keep) / 255) as u8;
    }
    dst[3] = (alpha + u32::from(dst[3]) * keep / 255) as u8;
}

fn map_unit(pixel: usize) -> f32 {
    (pixel as f32 + 0.5) / SCALE as f32 - BORDER as f32
}

fn stamp(out: &mut [u8], sprite: &RgbaImage, (x, y, rot): (i8, i8, i8)) {
    let (width, height) = sprite.dimensions();
    if width == 0 || height == 0 {
        return;
    }
    let centre = (f32::from(x) / 2.0 + 64.0, f32::from(y) / 2.0 + 64.0);
    let angle = (f32::from(rot) * 360.0 / 16.0).to_radians();
    let (sin, cos) = angle.sin_cos();
    let reach = SPRITE_SCALE * (1.0 + SPRITE_SHIFT) * std::f32::consts::SQRT_2;
    let to_pixel = |unit: f32| ((unit + BORDER as f32) * SCALE as f32).floor();
    let span = |middle: f32| {
        let low = to_pixel(middle - reach).max(0.0) as usize;
        let high = (to_pixel(middle + reach) as usize + 1).min(SIDE);
        low..high
    };
    for py in span(centre.1) {
        for px in span(centre.0) {
            let (dx, dy) = (map_unit(px) - centre.0, map_unit(py) - centre.1);
            let lx = (dx * cos + dy * sin) / SPRITE_SCALE + SPRITE_SHIFT;
            let ly = (-dx * sin + dy * cos) / SPRITE_SCALE - SPRITE_SHIFT;
            if !(-1.0..=1.0).contains(&lx) || !(-1.0..=1.0).contains(&ly) {
                continue;
            }
            let u = ((lx + 1.0) / 2.0 * width as f32) as u32;
            let v = ((1.0 - ly) / 2.0 * height as f32) as u32;
            let texel = sprite.get_pixel(u.min(width - 1), v.min(height - 1)).0;
            blend(out, (py * SIDE + px) * 4, texel);
        }
    }
}

pub fn compose(look: &MapLook, map: Option<&ClientMap>) -> Vec<u8> {
    let mut out = vec![0u8; SIDE * SIDE * 4];
    let background = match map {
        Some(_) => look.checkerboard.as_ref(),
        None => look.background.as_ref(),
    };
    if let Some(background) = background {
        let (width, height) = background.dimensions();
        for py in 0..SIDE {
            for px in 0..SIDE {
                let u = (px as u64 * u64::from(width) / SIDE as u64) as u32;
                let v = (py as u64 * u64::from(height) / SIDE as u64) as u32;
                let at = (py * SIDE + px) * 4;
                out[at..at + 4].copy_from_slice(&background.get_pixel(u, v).0);
            }
        }
    }
    let Some(map) = map else {
        return out;
    };
    for row in 0..MAP_SIZE {
        for col in 0..MAP_SIZE {
            let color = look.palette.rgba(map.colors[col + row * MAP_SIZE]);
            if color[3] == 0 {
                continue;
            }
            for dy in 0..SCALE {
                for dx in 0..SCALE {
                    let (px, py) = ((col + BORDER) * SCALE + dx, (row + BORDER) * SCALE + dy);
                    blend(&mut out, (py * SIDE + px) * 4, color);
                }
            }
        }
    }
    for mark in &map.decorations {
        if let Some(sprite) = look.sprites.get(&mark.kind) {
            stamp(&mut out, sprite, (mark.x, mark.y, mark.rot));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{COLORS, HIGH, LOW, LOWEST, NORMAL};
    use minecraft_net::messages::MapMark;

    const GRASS: u32 = 0x7F_B238;
    const PARCHMENT: [u8; 4] = [200, 180, 140, 255];
    const CHECKS: [u8; 4] = [90, 90, 90, 255];
    const RED: [u8; 4] = [255, 0, 0, 255];

    fn pixel(picture: &[u8], px: usize, py: usize) -> [u8; 4] {
        let at = (py * SIDE + px) * 4;
        picture[at..at + 4].try_into().unwrap()
    }

    fn look() -> MapLook {
        let mut corner = RgbaImage::new(8, 8);
        corner.put_pixel(0, 0, image::Rgba(RED));
        MapLook {
            palette: MapPalette::with_colors([(0, "NONE", 0), (1, "GRASS", GRASS)]),
            background: Some(RgbaImage::from_pixel(64, 64, image::Rgba(PARCHMENT))),
            checkerboard: Some(RgbaImage::from_pixel(128, 128, image::Rgba(CHECKS))),
            sprites: HashMap::from([("minecraft:player".to_owned(), corner)]),
        }
    }

    fn map(decorations: Vec<MapMark>) -> ClientMap {
        let mut colors = Box::new([0; COLORS]);
        colors[0] = crate::maps::packed(1, NORMAL);
        ClientMap {
            scale: 0,
            locked: false,
            colors,
            decorations,
            version: 1,
        }
    }

    fn player(rot: i8) -> MapMark {
        MapMark {
            kind: "minecraft:player".to_owned(),
            x: 0,
            y: 0,
            rot,
        }
    }

    #[test]
    fn map_colours_are_shaded_like_vanilla_brightness() {
        assert_eq!(shade(GRASS, NORMAL), [109, 153, 48, 255]);
        assert_eq!(shade(GRASS, HIGH), [127, 178, 56, 255]);
        assert_eq!(shade(GRASS, LOW), [89, 125, 39, 255]);
        assert_eq!(shade(GRASS, LOWEST), [67, 94, 29, 255]);
        let palette = MapPalette::with_colors([(0, "NONE", 0), (1, "GRASS", GRASS)]);
        assert_eq!(
            palette.rgba(crate::maps::packed(1, NORMAL)),
            [109, 153, 48, 255]
        );
        assert_eq!(palette.rgba(crate::maps::packed(0, NORMAL)), [0; 4]);
        assert_eq!(palette.rgba(crate::maps::packed(9, HIGH)), [0; 4]);
        assert_eq!(palette.id("GRASS"), Some(1));
    }

    #[test]
    fn a_map_picture_is_its_colours_at_twice_the_size_inside_the_frame() {
        let look = look();
        let blank = compose(&look, None);
        assert_eq!(blank.len(), 284 * 284 * 4);
        assert_eq!(pixel(&blank, 0, 0), PARCHMENT);
        assert_eq!(pixel(&blank, 14, 14), PARCHMENT);
        let picture = compose(&look, Some(&map(Vec::new())));
        for (px, py) in [(14, 14), (15, 14), (14, 15), (15, 15)] {
            assert_eq!(pixel(&picture, px, py), [109, 153, 48, 255], "{px} {py}");
        }
        assert_eq!(pixel(&picture, 16, 14), CHECKS);
        assert_eq!(pixel(&picture, 13, 13), CHECKS);
        assert_eq!(pixel(&picture, 283, 283), CHECKS);
    }

    #[test]
    fn markers_are_eight_map_pixels_turned_by_a_sixteenth_per_step() {
        let look = look();
        let red_at = |rot: i8| -> Vec<(usize, usize)> {
            let picture = compose(&look, Some(&map(vec![player(rot)])));
            (0..SIDE)
                .flat_map(|py| (0..SIDE).map(move |px| (px, py)))
                .filter(|&(px, py)| pixel(&picture, px, py) == RED)
                .collect()
        };
        assert_eq!(
            red_at(0),
            vec![(133, 149), (134, 149), (133, 150), (134, 150)]
        );
        assert_eq!(
            red_at(8),
            vec![(149, 133), (150, 133), (149, 134), (150, 134)]
        );
        assert_eq!(
            red_at(4),
            vec![(133, 133), (134, 133), (133, 134), (134, 134)]
        );
        assert_eq!(red_at(-12), red_at(4));
        let unknown = MapMark {
            kind: "minecraft:red_x".to_owned(),
            ..player(0)
        };
        assert_eq!(
            compose(&look, Some(&map(vec![unknown]))),
            compose(&look, Some(&map(Vec::new())))
        );
    }

    #[test]
    fn the_jar_gives_the_map_colours_and_marker_textures() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let root = Path::new(&root);
        let jar = root.join("client.jar");
        if !jar.is_file() {
            return;
        }
        let (palette, assets) = read_jar(&jar).unwrap();
        assert_eq!(palette.len(), 62);
        assert_eq!(palette.id("NONE"), Some(0));
        assert_eq!(palette.id("GRASS"), Some(1));
        assert_eq!(palette.rgb(1), Some(GRASS));
        assert_eq!(palette.id("COLOR_ORANGE"), Some(crate::maps::COLOR_ORANGE));
        assert_eq!(palette.id("COLOR_BROWN"), Some(crate::maps::COLOR_BROWN));
        assert_eq!(palette.id("GLOW_LICHEN"), Some(61));
        assert_eq!(
            palette.rgba(crate::maps::packed(1, NORMAL)),
            [109, 153, 48, 255]
        );
        assert_eq!(assets.len(), 40);
        for (kind, asset) in [
            ("minecraft:player", "player"),
            ("minecraft:mansion", "woodland_mansion"),
            ("minecraft:banner_light_blue", "light_blue_banner"),
            ("minecraft:red_x", "red_x"),
            ("minecraft:ocean_ruin_warm", "warm_ocean_ruins"),
        ] {
            assert!(
                assets.contains(&(kind.to_owned(), asset.to_owned())),
                "{kind}"
            );
        }
        let pack = root.join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let (look, warnings) = MapLook::load(&packs, Some(&jar));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(look.sprites.len(), 40);
        assert_eq!(look.sprites["minecraft:mansion"].dimensions(), (8, 8));
        assert_eq!(
            look.background.as_ref().map(RgbaImage::dimensions),
            Some((64, 64))
        );
        assert_eq!(
            look.checkerboard.as_ref().map(RgbaImage::dimensions),
            Some((128, 128))
        );
    }
}
