use std::ops::Range;

use asset_core::AssetNamespace;
use assets::PreparedWeapons;
use frame::{McGlyph, McGui, McScaling, McSprite, McStack, MinecraftUi};

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::images::HUD_CHROME_NAMESPACE;

pub(crate) const GUI_MATERIAL: &str = "mc_gui";
pub(crate) const ICON_MATERIAL: &str = "mc_item_icons";
pub(crate) const ANIMATED_ICON_MATERIAL: &str = "mc_animated_item_icons";
pub(crate) const GLINT_MATERIAL: &str = "mc_item_glint";
pub(crate) const WHITE: u32 = 0xFFFF_FFFF;
pub(crate) const GRAY: u32 = 0xFFAA_AAAA;
pub(crate) const DARK_GREEN: u32 = 0xFF00_AA00;
pub(crate) const RED: u32 = 0xFFFF_5555;
pub(crate) const YELLOW: u32 = 0xFFFF_FF55;
pub(crate) const AQUA: u32 = 0xFF55_FFFF;
pub(crate) const LABEL: u32 = 0xFF40_4040;
const GRADIENT_BANDS: i32 = 16;

pub(crate) fn gui_scale(width: u32, height: u32) -> u32 {
    let mut scale = 1;
    while scale < width
        && scale < height
        && width / (scale + 1) >= 320
        && height / (scale + 1) >= 240
    {
        scale += 1;
    }
    scale
}

pub(crate) fn argb(color: u32) -> [f32; 4] {
    let channel = |shift: u32| ((color >> shift) & 0xff) as f32 / 255.0;
    [channel(16), channel(8), channel(0), channel(24)]
}

pub(crate) fn shadow_color(color: u32) -> u32 {
    let dim = |shift: u32| ((((color >> shift) & 0xff) as f32 * 0.25) as u32) << shift;
    (color & 0xff00_0000) | dim(16) | dim(8) | dim(0)
}

pub(crate) fn opaque(color: u32) -> u32 {
    color | 0xff00_0000
}

pub(crate) fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> u32 {
    let h = (hue * 6.0) as i32 % 6;
    let f = hue * 6.0 - h as f32;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (r, g, b) = match h {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    let byte = |c: f32| ((c * 255.0) as i32).clamp(0, 255) as u32;
    (byte(r) << 16) | (byte(g) << 8) | byte(b)
}

pub(crate) fn bar_width(damage: u32, max: u32) -> i32 {
    let max = max.max(1) as f32;
    ((13.0 - damage as f32 * 13.0 / max + 0.5).floor() as i32).clamp(0, 13)
}

pub(crate) fn bar_color(damage: u32, max: u32) -> u32 {
    let max = max.max(1) as f32;
    let left = ((max - damage as f32) / max).max(0.0);
    hsv_to_rgb(left / 3.0, 1.0, 1.0)
}

pub(crate) fn clip_bar(loaded: u32, capacity: u32) -> (i32, u32) {
    let capacity = capacity.max(1);
    let loaded = loaded.min(capacity);
    let width = (1 + loaded as i32 * 12 / capacity as i32).min(13);
    (width, bar_color(capacity - loaded, capacity))
}

pub(crate) fn cooldown_of(ui: &MinecraftUi, stack: &McStack) -> f32 {
    ui.cooldowns
        .iter()
        .find(|(id, _)| *id == stack.id)
        .map_or(0.0, |&(_, left)| left.clamp(0.0, 1.0))
}

pub(crate) fn cooldown_rows(left: f32) -> Option<[i32; 2]> {
    if left <= 0.0 {
        return None;
    }
    let top = (16.0 * (1.0 - left)).floor() as i32;
    Some([top, top + (16.0 * left).ceil() as i32])
}

pub(crate) fn tooltip_position(screen: [i32; 2], mouse: [i32; 2], size: [i32; 2]) -> [i32; 2] {
    let [mut x, mut y] = [mouse[0] + 12, mouse[1] - 12];
    if x + size[0] > screen[0] {
        x = (x - 24 - size[0]).max(4);
    }
    if y + size[1] + 3 > screen[1] {
        y = screen[1] - size[1] - 3;
    }
    [x, y]
}

pub(crate) fn tooltip_height(lines: usize, title_gap: bool) -> i32 {
    let start = if !title_gap || lines == 1 { -2 } else { 0 };
    start + 10 * lines as i32
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GuiSpace {
    pub scale: u32,
    pub width: i32,
    pub height: i32,
    physical: [f32; 2],
    vertex_per_physical: [f32; 2],
    window_per_physical: [f32; 2],
}

impl GuiSpace {
    pub(crate) fn new(physical: [u32; 2], surface: [f32; 2], window: [f32; 2]) -> Self {
        let [pw, ph] = [physical[0].max(1), physical[1].max(1)];
        let scale = gui_scale(pw, ph);
        Self {
            scale,
            width: pw.div_ceil(scale) as i32,
            height: ph.div_ceil(scale) as i32,
            physical: [pw as f32, ph as f32],
            vertex_per_physical: [
                (surface[0] as i32) as f32 / pw as f32,
                (surface[1] as i32) as f32 / ph as f32,
            ],
            window_per_physical: [window[0] / pw as f32, window[1] / ph as f32],
        }
    }

    pub(crate) fn vertex(&self, x: f32, y: f32) -> [f32; 2] {
        let s = self.scale as f32;
        [
            x * s * self.vertex_per_physical[0] + 0.5,
            y * s * self.vertex_per_physical[1] + 0.5,
        ]
    }

    pub(crate) fn window(&self, x: f32, y: f32) -> [f32; 2] {
        let s = self.scale as f32;
        [
            x * s * self.window_per_physical[0],
            y * s * self.window_per_physical[1],
        ]
    }

    pub(crate) fn mouse(&self, physical: [f32; 2]) -> [f64; 2] {
        [
            f64::from(physical[0]) * f64::from(self.width) / f64::from(self.physical[0]),
            f64::from(physical[1]) * f64::from(self.height) / f64::from(self.physical[1]),
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextLine {
    pub text: String,
    pub color: u32,
    pub italic: bool,
    pub accents: Vec<(Range<usize>, u32)>,
}

impl TextLine {
    pub(crate) fn new(text: impl Into<String>, color: u32) -> Self {
        Self {
            text: text.into(),
            color,
            italic: false,
            accents: Vec::new(),
        }
    }
}

pub(crate) struct Painter<'a> {
    pub gui: &'a McGui,
    pub space: GuiSpace,
    pub quads: Vec<Draw2dQuad>,
    pub glint_shift: [f32; 2],
}

impl<'a> Painter<'a> {
    pub(crate) fn new(gui: &'a McGui, space: GuiSpace) -> Self {
        Self {
            gui,
            space,
            quads: Vec::new(),
            glint_shift: [0.0, 0.0],
        }
    }

    fn push(
        &mut self,
        material: &str,
        namespace: AssetNamespace,
        corners: [[f32; 2]; 4],
        st: [[f32; 2]; 4],
        color: [f32; 4],
    ) {
        let xy = corners.map(|[x, y]| self.space.vertex(x, y));
        self.quads.push(Draw2dQuad {
            xy,
            st,
            color,
            material: material.to_owned(),
            material_namespace: namespace,
            provenance: Draw2dProvenance::CgDraw {
                site: "minecraft_gui",
            },
            layer: 1,
            clip: None,
            glint: None,
        });
    }

    fn rect(
        &mut self,
        material: &str,
        namespace: AssetNamespace,
        [x0, y0, x1, y1]: [f32; 4],
        [s0, t0, s1, t1]: [f32; 4],
        color: [f32; 4],
    ) {
        self.push(
            material,
            namespace,
            [[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
            [[s0, t0], [s1, t0], [s1, t1], [s0, t1]],
            color,
        );
    }

    fn atlas(&mut self, area: [f32; 4], [u0, v0, u1, v1]: [f32; 4], color: u32) {
        let [w, h] = [
            self.gui.size[0].max(1) as f32,
            self.gui.size[1].max(1) as f32,
        ];
        self.rect(
            GUI_MATERIAL,
            HUD_CHROME_NAMESPACE,
            area,
            [u0 / w, v0 / h, u1 / w, v1 / h],
            argb(color),
        );
    }

    pub(crate) fn fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (y0, y1) = (y0.min(y1), y0.max(y1));
        if x0 == x1 || y0 == y1 {
            return;
        }
        let [wx, wy] = [self.gui.white[0] as f32, self.gui.white[1] as f32];
        self.atlas(
            [x0 as f32, y0 as f32, x1 as f32, y1 as f32],
            [wx, wy, wx, wy],
            color,
        );
    }

    pub(crate) fn gradient(
        &mut self,
        area: [i32; 4],
        hole: Option<[i32; 4]>,
        top: u32,
        bottom: u32,
    ) {
        let [x0, y0, x1, y1] = area;
        let height = (y1 - y0).max(1);
        let mut cuts: Vec<i32> = (0..=GRADIENT_BANDS)
            .map(|i| y0 + height * i / GRADIENT_BANDS)
            .collect();
        if let Some([_, hy0, _, hy1]) = hole {
            cuts.extend([hy0.clamp(y0, y1), hy1.clamp(y0, y1)]);
        }
        cuts.sort_unstable();
        cuts.dedup();
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let t = ((a + b) as f32 * 0.5 - y0 as f32) / height as f32;
            let color = lerp_argb(top, bottom, t);
            match hole {
                Some([hx0, hy0, hx1, hy1]) if a >= hy0 && b <= hy1 => {
                    self.fill(x0, a, hx0.clamp(x0, x1), b, color);
                    self.fill(hx1.clamp(x0, x1), a, x1, b, color);
                }
                _ => self.fill(x0, a, x1, b, color),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn blit(&mut self, sheet: &str, x: i32, y: i32, u: i32, v: i32, w: i32, h: i32) {
        let Some(&[ax, ay]) = self.gui.sheets.get(sheet) else {
            return;
        };
        let (u0, v0) = ((ax as i32 + u) as f32, (ay as i32 + v) as f32);
        self.atlas(
            [x as f32, y as f32, (x + w) as f32, (y + h) as f32],
            [u0, v0, u0 + w as f32, v0 + h as f32],
            WHITE,
        );
    }

    pub(crate) fn sprite(&mut self, name: &str, x: i32, y: i32, w: i32, h: i32) {
        self.sprite_tinted(name, x, y, w, h, WHITE);
    }

    pub(crate) fn sprite_tinted(&mut self, name: &str, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let Some(&sprite) = self.gui.sprites.get(name) else {
            return;
        };
        if w <= 0 || h <= 0 {
            return;
        }
        match sprite.scaling {
            McScaling::Stretch => self.region(sprite, [1, 1], [0, 0, 1, 1], [x, y, w, h], color),
            McScaling::Tile { width, height } => {
                self.tiled(
                    sprite,
                    [width as i32, height as i32],
                    [0, 0, width as i32, height as i32],
                    [x, y, w, h],
                    color,
                );
            }
            McScaling::NineSlice {
                width,
                height,
                border,
                stretch_inner,
            } => {
                self.nine_slice(
                    sprite,
                    [width as i32, height as i32],
                    border.map(|b| b as i32),
                    stretch_inner,
                    [x, y, w, h],
                    color,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn sprite_part(
        &mut self,
        name: &str,
        tex_w: i32,
        tex_h: i32,
        u: i32,
        v: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) {
        let Some(&sprite) = self.gui.sprites.get(name) else {
            return;
        };
        if w <= 0 || h <= 0 {
            return;
        }
        self.region(sprite, [tex_w, tex_h], [u, v, w, h], [x, y, w, h], WHITE);
    }

    fn region(
        &mut self,
        sprite: McSprite,
        [tex_w, tex_h]: [i32; 2],
        [u, v, uw, vh]: [i32; 4],
        [x, y, w, h]: [i32; 4],
        color: u32,
    ) {
        let [ax, ay, sw, sh] = sprite.rect.map(|n| n as f32);
        let (tw, th) = (tex_w.max(1) as f32, tex_h.max(1) as f32);
        let source = [
            ax + u as f32 * sw / tw,
            ay + v as f32 * sh / th,
            ax + (u + uw) as f32 * sw / tw,
            ay + (v + vh) as f32 * sh / th,
        ];
        self.atlas(
            [x as f32, y as f32, (x + w) as f32, (y + h) as f32],
            source,
            color,
        );
    }

    fn tiled(
        &mut self,
        sprite: McSprite,
        texture: [i32; 2],
        [u, v, tile_w, tile_h]: [i32; 4],
        [x, y, w, h]: [i32; 4],
        color: u32,
    ) {
        if w <= 0 || h <= 0 || tile_w <= 0 || tile_h <= 0 {
            return;
        }
        let mut ty = 0;
        while ty < h {
            let th = tile_h.min(h - ty);
            let mut tx = 0;
            while tx < w {
                let tw = tile_w.min(w - tx);
                self.region(
                    sprite,
                    texture,
                    [u, v, tw, th],
                    [x + tx, y + ty, tw, th],
                    color,
                );
                tx += tile_w;
            }
            ty += tile_h;
        }
    }

    fn inner(
        &mut self,
        sprite: McSprite,
        texture: [i32; 2],
        stretch: bool,
        source: [i32; 4],
        area: [i32; 4],
        color: u32,
    ) {
        if area[2] <= 0 || area[3] <= 0 {
            return;
        }
        if stretch {
            self.region(sprite, texture, source, area, color);
        } else {
            self.tiled(sprite, texture, source, area, color);
        }
    }

    fn nine_slice(
        &mut self,
        sprite: McSprite,
        [nw, nh]: [i32; 2],
        border: [i32; 4],
        stretch: bool,
        [x, y, w, h]: [i32; 4],
        color: u32,
    ) {
        let left = border[0].min(w / 2);
        let top = border[1].min(h / 2);
        let right = border[2].min(w / 2);
        let bottom = border[3].min(h / 2);
        let t = [nw, nh];
        if w == nw && h == nh {
            self.region(sprite, t, [0, 0, w, h], [x, y, w, h], color);
            return;
        }
        if h == nh {
            self.region(sprite, t, [0, 0, left, h], [x, y, left, h], color);
            self.inner(
                sprite,
                t,
                stretch,
                [left, 0, nw - right - left, nh],
                [x + left, y, w - right - left, h],
                color,
            );
            self.region(
                sprite,
                t,
                [nw - right, 0, right, h],
                [x + w - right, y, right, h],
                color,
            );
            return;
        }
        if w == nw {
            self.region(sprite, t, [0, 0, w, top], [x, y, w, top], color);
            self.inner(
                sprite,
                t,
                stretch,
                [0, top, nw, nh - bottom - top],
                [x, y + top, w, h - bottom - top],
                color,
            );
            self.region(
                sprite,
                t,
                [0, nh - bottom, w, bottom],
                [x, y + h - bottom, w, bottom],
                color,
            );
            return;
        }
        let mid_w = w - right - left;
        let mid_h = h - bottom - top;
        let src_w = nw - right - left;
        let src_h = nh - bottom - top;
        self.region(sprite, t, [0, 0, left, top], [x, y, left, top], color);
        self.inner(
            sprite,
            t,
            stretch,
            [left, 0, src_w, top],
            [x + left, y, mid_w, top],
            color,
        );
        self.region(
            sprite,
            t,
            [nw - right, 0, right, top],
            [x + w - right, y, right, top],
            color,
        );
        self.region(
            sprite,
            t,
            [0, nh - bottom, left, bottom],
            [x, y + h - bottom, left, bottom],
            color,
        );
        self.inner(
            sprite,
            t,
            stretch,
            [left, nh - bottom, src_w, bottom],
            [x + left, y + h - bottom, mid_w, bottom],
            color,
        );
        self.region(
            sprite,
            t,
            [nw - right, nh - bottom, right, bottom],
            [x + w - right, y + h - bottom, right, bottom],
            color,
        );
        self.inner(
            sprite,
            t,
            stretch,
            [0, top, left, src_h],
            [x, y + top, left, mid_h],
            color,
        );
        self.inner(
            sprite,
            t,
            stretch,
            [left, top, src_w, src_h],
            [x + left, y + top, mid_w, mid_h],
            color,
        );
        self.inner(
            sprite,
            t,
            stretch,
            [nw - right, top, right, src_h],
            [x + w - right, y + top, right, mid_h],
            color,
        );
    }

    pub(crate) fn width(&self, text: &str) -> i32 {
        self.gui.font.width(text)
    }

    pub(crate) fn text(&mut self, text: &str, x: i32, y: i32, color: u32, shadow: bool) {
        self.styled_text(false, text, x, y, color, shadow, false);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn styled_text(
        &mut self,
        alt: bool,
        text: &str,
        x: i32,
        y: i32,
        color: u32,
        shadow: bool,
        italic: bool,
    ) {
        let line = TextLine {
            text: text.to_owned(),
            color,
            italic,
            accents: Vec::new(),
        };
        self.line(alt, &line, x, y, shadow);
    }

    pub(crate) fn line(&mut self, alt: bool, line: &TextLine, x: i32, y: i32, shadow: bool) {
        let font = if alt {
            &self.gui.alt_font
        } else {
            &self.gui.font
        };
        let mut pen = x as f32;
        let mut glyphs: Vec<(f32, McGlyph, u32)> = Vec::with_capacity(line.text.len());
        for (at, c) in line.text.char_indices() {
            if let Some(space) = font.spaces.get(&c) {
                pen += space;
                continue;
            }
            match font.glyphs.get(&c) {
                Some(glyph) => {
                    let color = line
                        .accents
                        .iter()
                        .find(|(range, _)| range.contains(&at))
                        .map_or(line.color, |&(_, color)| color);
                    glyphs.push((pen, *glyph, color));
                    pen += glyph.advance;
                }
                None => pen += frame::minecraft_gui::MC_MISSING_ADVANCE,
            }
        }
        if shadow {
            for &(gx, glyph, color) in &glyphs {
                let dark = shadow_color(color);
                self.glyph(gx + 1.0, (y + 1) as f32, glyph, dark, line.italic);
            }
        }
        for &(gx, glyph, color) in &glyphs {
            self.glyph(gx, y as f32, glyph, color, line.italic);
        }
    }

    fn glyph(&mut self, x: f32, y: f32, glyph: McGlyph, color: u32, italic: bool) {
        let up = 7.0 - glyph.ascent as f32;
        let down = up + glyph.rect[3] as f32 * glyph.scale;
        let right = glyph.rect[2] as f32 * glyph.scale;
        let (shear_top, shear_bottom) = if italic {
            (1.0 - 0.25 * up, 1.0 - 0.25 * down)
        } else {
            (0.0, 0.0)
        };
        let [w, h] = [
            self.gui.size[0].max(1) as f32,
            self.gui.size[1].max(1) as f32,
        ];
        let [u0, v0, u1, v1] = [
            glyph.rect[0] as f32 / w,
            glyph.rect[1] as f32 / h,
            (glyph.rect[0] + glyph.rect[2]) as f32 / w,
            (glyph.rect[1] + glyph.rect[3]) as f32 / h,
        ];
        self.push(
            GUI_MATERIAL,
            HUD_CHROME_NAMESPACE,
            [
                [x + shear_top, y + up],
                [x + right + shear_top, y + up],
                [x + right + shear_bottom, y + down],
                [x + shear_bottom, y + down],
            ],
            [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
            argb(color),
        );
    }

    pub(crate) fn item(
        &mut self,
        ui: &MinecraftUi,
        weapons: Option<&PreparedWeapons>,
        stack: &McStack,
        x: i32,
        y: i32,
    ) {
        self.item_tinted(ui, weapons, stack, x, y, WHITE);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn item_tinted(
        &mut self,
        ui: &MinecraftUi,
        weapons: Option<&PreparedWeapons>,
        stack: &McStack,
        x: i32,
        y: i32,
        color: u32,
    ) {
        self.item_sized(ui, weapons, stack, [x as f32, y as f32], 16.0, color);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn item_sized(
        &mut self,
        ui: &MinecraftUi,
        weapons: Option<&PreparedWeapons>,
        stack: &McStack,
        [x, y]: [f32; 2],
        size: f32,
        color: u32,
    ) {
        let half = size * 0.5;
        if let Some(weapon) = stack.weapon {
            let Some(registry) = weapons else { return };
            let namespace = registry
                .0
                .namespace_of(weapon)
                .unwrap_or(HUD_CHROME_NAMESPACE);
            let Some(icon) = registry
                .0
                .kill_icon_image_of(weapon)
                .or_else(|| registry.0.kill_icon_of(weapon))
            else {
                return;
            };
            let facts = registry.0.facts_of(weapon);
            let ratio = match facts.as_ref().map(|f| f.kill_icon_ratio) {
                Some(1) => 2.0,
                Some(2) => 4.0,
                _ => 1.0,
            };
            let flip = facts.as_ref().is_some_and(|f| f.flip_kill_icon);
            let w: f32 = 18.0 * size / 16.0;
            let h = (w / ratio).min(size);
            let st = if flip {
                [1.0, 0.0, 0.0, 1.0]
            } else {
                [0.0, 0.0, 1.0, 1.0]
            };
            let icon = icon.to_owned();
            self.rect(
                &icon,
                namespace,
                [
                    x + half - w * 0.5,
                    y + half - h * 0.5,
                    x + half + w * 0.5,
                    y + half + h * 0.5,
                ],
                st,
                argb(color),
            );
            return;
        }
        let icon = ui
            .animated_icon_rects
            .get(&stack.icon)
            .map(|&st| (ANIMATED_ICON_MATERIAL, st))
            .or_else(|| ui.icon_rects.get(&stack.icon).map(|&st| (ICON_MATERIAL, st)));
        if let Some((material, st)) = icon {
            self.rect(
                material,
                HUD_CHROME_NAMESPACE,
                [x, y, x + size, y + size],
                st,
                argb(color),
            );
            if stack.glint {
                self.glint(material, x, y, st, size);
            }
        }
    }

    fn glint(&mut self, material: &str, x: f32, y: f32, st: [f32; 4], size: f32) {
        let cell = |lo: f32, hi: f32| (lo / (hi - lo).max(f32::EPSILON)).round();
        let cell = [cell(st[0], st[2]), cell(st[1], st[3])];
        let shift = self.glint_shift;
        let corner = |[u, v]: [f32; 2]| {
            frame::minecraft_ui::glint_uv(
                shift,
                [
                    (cell[0] + u) / frame::minecraft_ui::GLINT_ATLAS_CELLS,
                    (cell[1] + v) / frame::minecraft_ui::GLINT_ATLAS_CELLS,
                ],
                frame::minecraft_ui::ITEM_GLINT_SCALE,
            )
        };
        self.rect(
            material,
            HUD_CHROME_NAMESPACE,
            [x, y, x + size, y + size],
            st,
            [1.0, 1.0, 1.0, frame::minecraft_ui::GLINT_STRENGTH],
        );
        if let Some(quad) = self.quads.last_mut() {
            quad.glint = Some([
                corner([0.0, 0.0]),
                corner([1.0, 0.0]),
                corner([1.0, 1.0]),
                corner([0.0, 1.0]),
            ]);
        }
    }

    pub(crate) fn decorations(
        &mut self,
        ui: &MinecraftUi,
        stack: &McStack,
        [x, y]: [i32; 2],
        count: Option<(String, u32)>,
    ) {
        if let Some((damage, max)) = stack.damage.filter(|&(damage, _)| damage > 0) {
            self.fill(x + 2, y + 13, x + 15, y + 15, 0xff00_0000);
            self.fill(
                x + 2,
                y + 13,
                x + 2 + bar_width(damage, max),
                y + 14,
                opaque(bar_color(damage, max)),
            );
        }
        if let Some((loaded, capacity)) = stack.clip {
            let (width, color) = clip_bar(loaded, capacity);
            self.fill(x + 2, y + 13, x + 15, y + 15, 0xff00_0000);
            self.fill(x + 2, y + 13, x + 2 + width, y + 14, opaque(color));
        }
        if let Some([top, bottom]) = cooldown_rows(cooldown_of(ui, stack)) {
            self.fill(x, y + top, x + 16, y + bottom, 0x7FFF_FFFF);
        }
        let label = count.or_else(|| (stack.count != 1).then(|| (stack.count.to_string(), WHITE)));
        if let Some((text, color)) = label {
            let width = self.width(&text);
            self.text(&text, x + 19 - 2 - width, y + 6 + 3, color, true);
        }
    }

    pub(crate) fn tooltip(&mut self, lines: &[TextLine], mouse: [i32; 2], title_gap: bool) {
        if lines.is_empty() {
            return;
        }
        let width = lines
            .iter()
            .map(|line| self.width(&line.text))
            .max()
            .unwrap_or(0);
        let height = tooltip_height(lines.len(), title_gap);
        let [x, y] = tooltip_position(
            [self.space.width, self.space.height],
            mouse,
            [width, height],
        );
        self.sprite(
            "tooltip/background",
            x - 12,
            y - 12,
            width + 24,
            height + 24,
        );
        self.sprite("tooltip/frame", x - 12, y - 12, width + 24, height + 24);
        let mut line_y = y;
        for (i, line) in lines.iter().enumerate() {
            self.line(false, line, x, line_y, true);
            line_y += 10 + if title_gap && i == 0 { 2 } else { 0 };
        }
    }
}

fn lerp_argb(from: u32, to: u32, t: f32) -> u32 {
    let mix = |shift: u32| {
        let (a, b) = (
            ((from >> shift) & 0xff) as f32,
            ((to >> shift) & 0xff) as f32,
        );
        ((a + (b - a) * t.clamp(0.0, 1.0)).round() as u32) << shift
    };
    mix(24) | mix(16) | mix(8) | mix(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gui() -> McGui {
        let mut gui = McGui {
            size: [512, 512],
            white: [101, 101],
            ..McGui::default()
        };
        gui.sheets.insert("inventory".into(), [0, 256]);
        gui.sprites.insert(
            "tooltip/frame".into(),
            McSprite {
                rect: [0, 0, 100, 100],
                scaling: McScaling::NineSlice {
                    width: 100,
                    height: 100,
                    border: [10; 4],
                    stretch_inner: true,
                },
            },
        );
        gui.sprites.insert(
            "tooltip/background".into(),
            McSprite {
                rect: [100, 0, 100, 100],
                scaling: McScaling::NineSlice {
                    width: 100,
                    height: 100,
                    border: [9; 4],
                    stretch_inner: false,
                },
            },
        );
        gui.sprites.insert(
            "hud/hotbar".into(),
            McSprite {
                rect: [200, 0, 182, 22],
                scaling: McScaling::Stretch,
            },
        );
        let glyph = McGlyph {
            rect: [300, 300, 8, 8],
            scale: 1.0,
            ascent: 7,
            advance: 6.0,
        };
        for c in 'a'..='z' {
            gui.font.glyphs.insert(c, glyph);
        }
        gui.font.glyphs.insert(
            'é',
            McGlyph {
                rect: [320, 300, 9, 12],
                scale: 1.0,
                ascent: 10,
                advance: 6.0,
            },
        );
        gui.font.spaces.insert(' ', 4.0);
        gui
    }

    fn space(scale: u32) -> GuiSpace {
        let physical = [320 * scale + 7, 240 * scale + 5];
        GuiSpace::new(
            physical,
            [physical[0] as f32, physical[1] as f32],
            [physical[0] as f32, physical[1] as f32],
        )
    }

    #[test]
    fn auto_scale_is_the_largest_that_keeps_320_by_240() {
        assert_eq!(gui_scale(1920, 1080), 4);
        assert_eq!(gui_scale(2560, 1440), 6);
        assert_eq!(gui_scale(3840, 2160), 9);
        assert_eq!(gui_scale(1280, 720), 3);
        assert_eq!(gui_scale(640, 480), 2);
        assert_eq!(gui_scale(639, 480), 1);
        assert_eq!(gui_scale(200, 100), 1);
    }

    #[test]
    fn gui_size_rounds_up_and_pixels_land_on_physical_edges() {
        let space = GuiSpace::new([1366, 768], [1366.0, 768.0], [1366.0, 768.0]);
        assert_eq!((space.scale, space.width, space.height), (3, 456, 256));
        assert_eq!(space.vertex(10.0, 2.0), [30.5, 6.5]);
        let hidpi = GuiSpace::new([2560, 1440], [1706.6666, 960.0], [1706.6666, 960.0]);
        assert_eq!(hidpi.scale, 6);
        let [x, _] = hidpi.vertex(1.0, 0.0);
        assert!((x - (6.0 * 1706.0 / 2560.0 + 0.5)).abs() < 1e-4);
        let [wx, wy] = hidpi.window(10.0, 10.0);
        assert!((wx - 40.0).abs() < 1e-3 && (wy - 40.0).abs() < 1e-3);
        let [mx, my] = space.mouse([683.0, 384.0]);
        assert!((mx - 228.0).abs() < 1e-9 && (my - 128.0).abs() < 1e-9);
    }

    #[test]
    fn text_shadow_is_a_quarter_of_each_channel() {
        assert_eq!(shadow_color(0xFFFF_FFFF), 0xFF3F_3F3F);
        assert_eq!(shadow_color(0x80FF_FF55), 0x803F_3F15);
        assert_eq!(
            argb(0xFF40_4040),
            [64.0 / 255.0, 64.0 / 255.0, 64.0 / 255.0, 1.0]
        );
    }

    #[test]
    fn durability_bar_matches_vanilla() {
        assert_eq!(bar_width(0, 250), 13);
        assert_eq!(bar_width(125, 250), 7);
        assert_eq!(bar_width(249, 250), 0);
        assert_eq!(bar_width(250, 250), 0);
        assert_eq!(bar_color(0, 100), 0x00FF00);
        assert_eq!(bar_color(100, 100), 0xFF0000);
        assert_eq!(bar_color(50, 100), 0xFFFF00);
    }

    #[test]
    fn a_gun_s_clip_bar_is_green_when_full_and_red_when_empty() {
        assert_eq!(clip_bar(30, 30), (13, 0x00FF00));
        assert_eq!(clip_bar(0, 30), (1, 0xFF0000));
        assert_eq!(clip_bar(15, 30).0, 7);
        assert_eq!(clip_bar(29, 30).0, 12);
        assert_eq!(clip_bar(17, 24).0, 9);
        assert_eq!(clip_bar(40, 30), (13, 0x00FF00));
        assert_eq!(clip_bar(0, 0), (1, 0xFF0000));
    }

    #[test]
    fn tooltips_flip_left_and_clamp_up() {
        assert_eq!(
            tooltip_position([400, 300], [100, 100], [50, 20]),
            [112, 88]
        );
        assert_eq!(
            tooltip_position([400, 300], [380, 100], [50, 20]),
            [318, 88]
        );
        assert_eq!(tooltip_position([400, 300], [20, 100], [390, 20]), [4, 88]);
        assert_eq!(
            tooltip_position([400, 300], [100, 295], [50, 30]),
            [112, 267]
        );
        assert_eq!(tooltip_height(1, true), 8);
        assert_eq!(tooltip_height(3, true), 30);
        assert_eq!(tooltip_height(3, false), 28);
    }

    #[test]
    fn text_draws_shadow_then_glyphs_on_the_ascent_line() {
        let gui = gui();
        let mut painter = Painter::new(&gui, space(1));
        painter.text("ab é", 10, 20, WHITE, true);
        assert_eq!(painter.quads.len(), 6);
        let first_shadow = &painter.quads[0];
        assert_eq!(first_shadow.xy[0], [11.5, 21.5]);
        let a = &painter.quads[3];
        assert_eq!(a.xy[0], [10.5, 20.5]);
        assert_eq!(a.xy[2], [18.5, 28.5]);
        let e = &painter.quads[5];
        assert_eq!(e.xy[0], [26.5, 17.5]);
        assert_eq!(e.xy[2], [35.5, 29.5]);
        assert_eq!(painter.width("ab é"), 22);
    }

    #[test]
    fn italic_glyphs_lean_by_a_quarter_pixel_per_pixel() {
        let gui = gui();
        let mut painter = Painter::new(&gui, space(1));
        painter.styled_text(false, "a", 0, 0, WHITE, false, true);
        let quad = &painter.quads[0];
        assert_eq!(quad.xy[0], [1.5, 0.5]);
        assert_eq!(quad.xy[3], [-0.5, 8.5]);
    }

    #[test]
    fn nine_slices_keep_corners_and_stretch_or_tile_the_middle() {
        let gui = gui();
        let mut painter = Painter::new(&gui, space(1));
        painter.sprite("tooltip/frame", 0, 0, 60, 40);
        assert_eq!(painter.quads.len(), 9);
        assert_eq!(painter.quads[0].xy[2], [10.5, 10.5]);
        assert_eq!(painter.quads[2].xy[0], [50.5, 0.5]);
        let mut painter = Painter::new(&gui, space(1));
        painter.sprite("tooltip/background", 0, 0, 300, 40);
        let middles = painter
            .quads
            .iter()
            .filter(|q| q.xy[0][1] == 9.5 && q.xy[2][1] == 31.5)
            .count();
        assert!(middles > 3);
        let mut painter = Painter::new(&gui, space(1));
        painter.sprite("tooltip/frame", 0, 0, 100, 100);
        assert_eq!(painter.quads.len(), 1);
    }

    #[test]
    fn sprite_parts_crop_by_texture_fraction() {
        let gui = gui();
        let mut painter = Painter::new(&gui, space(2));
        painter.sprite_part("hud/hotbar", 182, 22, 0, 0, 10, 10, 91, 22);
        let quad = &painter.quads[0];
        assert_eq!(quad.st[1][0], (200.0 + 91.0) / 512.0);
        assert_eq!(quad.xy[0], [20.5, 20.5]);
    }

    #[test]
    fn gradients_skip_the_hole_and_blend_top_to_bottom() {
        let gui = gui();
        let mut painter = Painter::new(&gui, space(1));
        painter.gradient(
            [0, 0, 100, 160],
            Some([20, 40, 60, 80]),
            0xC010_1010,
            0xD010_1010,
        );
        let alpha = |q: &Draw2dQuad| (q.color[3] * 255.0).round() as u32;
        assert_eq!(alpha(&painter.quads[0]), 0xC1);
        assert_eq!(alpha(painter.quads.last().unwrap()), 0xD0);
        for quad in &painter.quads {
            let [x0, y0] = quad.xy[0];
            let [x1, y1] = quad.xy[2];
            let inside = x0 >= 20.5 && x1 <= 60.5 && y0 >= 40.5 && y1 <= 80.5;
            assert!(!inside);
        }
    }

    #[test]
    fn counts_sit_at_the_bottom_right_with_a_shadow() {
        let gui = gui();
        let mut painter = Painter::new(&gui, space(1));
        let stack = McStack {
            id: "minecraft:dirt".into(),
            count: 1,
            weapon: None,
            damage: Some((10, 20)),
            max: 64,
            named: false,
            name: None,
            attack: None,
            enchantments: Vec::new(),
            contents: Vec::new(),
            more: None,
            glint: false,
            icon: "minecraft:dirt".into(),
            lore: Vec::new(),
            clip: None,
        };
        let mut ui = MinecraftUi::default();
        painter.decorations(&ui, &stack, [0, 0], Some(("ab".into(), YELLOW)));
        assert_eq!(painter.quads.len(), 6);
        assert_eq!(painter.quads[0].xy[0], [2.5, 13.5]);
        assert_eq!(painter.quads[1].xy[2], [9.5, 14.5]);
        assert_eq!(painter.quads[4].xy[0], [5.5, 9.5]);
        ui.cooldowns.push(("minecraft:dirt".into(), 0.3));
        let mut painter = Painter::new(&gui, space(1));
        painter.decorations(&ui, &stack, [0, 0], None);
        assert_eq!(painter.quads.len(), 3);
        assert_eq!(painter.quads[2].xy[0], [0.5, 11.5]);
        assert_eq!(painter.quads[2].xy[2], [16.5, 16.5]);
        assert_eq!(painter.quads[2].color, argb(0x7FFF_FFFF));
    }

    #[test]
    fn cooldowns_shrink_from_the_top_like_vanilla() {
        assert_eq!(cooldown_rows(0.0), None);
        assert_eq!(cooldown_rows(1.0), Some([0, 16]));
        assert_eq!(cooldown_rows(0.5), Some([8, 16]));
        assert_eq!(cooldown_rows(0.01), Some([15, 16]));
    }

    #[test]
    fn accents_recolour_their_glyphs_and_shadows() {
        let gui = gui();
        let mut painter = Painter::new(&gui, space(1));
        let line = TextLine {
            accents: vec![(1..2, GRAY)],
            ..TextLine::new("ab", WHITE)
        };
        painter.line(false, &line, 0, 0, true);
        let colors: Vec<[f32; 4]> = painter.quads.iter().map(|q| q.color).collect();
        assert_eq!(
            colors,
            vec![
                argb(shadow_color(WHITE)),
                argb(shadow_color(GRAY)),
                argb(WHITE),
                argb(GRAY)
            ]
        );
    }

    fn shiny(id: &str, glint: bool) -> McStack {
        McStack {
            id: id.into(),
            count: 1,
            weapon: None,
            damage: None,
            max: 1,
            named: false,
            name: None,
            attack: None,
            enchantments: Vec::new(),
            contents: Vec::new(),
            more: None,
            glint,
            icon: id.into(),
            lore: Vec::new(),
            clip: None,
        }
    }

    #[test]
    fn glinting_icons_get_a_second_quad_with_the_vanilla_glint_corners() {
        let gui = gui();
        let mut ui = MinecraftUi::default();
        let st = [3.0 / 32.0, 5.0 / 32.0, 4.0 / 32.0, 6.0 / 32.0];
        ui.icon_rects.insert("minecraft:diamond_pickaxe".into(), st);
        let mut painter = Painter::new(&gui, space(1));
        painter.glint_shift = [-0.25, 0.5];
        painter.item(&ui, None, &shiny("minecraft:diamond_pickaxe", true), 10, 20);
        assert_eq!(painter.quads.len(), 2);
        let [icon, glint] = [&painter.quads[0], &painter.quads[1]];
        assert_eq!(icon.glint, None);
        assert_eq!(glint.xy, icon.xy);
        assert_eq!(glint.st, icon.st);
        assert_eq!(glint.material, ICON_MATERIAL);
        assert_eq!(glint.color, [1.0, 1.0, 1.0, 0.75]);
        let corner = |u: f32, v: f32| {
            frame::minecraft_ui::glint_uv([-0.25, 0.5], [(3.0 + u) / 32.0, (5.0 + v) / 32.0], 8.0)
        };
        assert_eq!(
            glint.glint,
            Some([
                corner(0.0, 0.0),
                corner(1.0, 0.0),
                corner(1.0, 1.0),
                corner(0.0, 1.0)
            ])
        );
        let mut painter = Painter::new(&gui, space(1));
        painter.item(
            &ui,
            None,
            &shiny("minecraft:diamond_pickaxe", false),
            10,
            20,
        );
        assert_eq!(painter.quads.len(), 1);
        assert_eq!(painter.quads[0].glint, None);
    }

    #[test]
    fn animated_icons_glint_through_their_own_atlas_and_guns_never_glint() {
        let gui = gui();
        let mut ui = MinecraftUi::default();
        ui.animated_icon_rects
            .insert("minecraft:compass".into(), [0.0, 0.0, 0.5, 0.5]);
        let mut painter = Painter::new(&gui, space(1));
        painter.item(&ui, None, &shiny("minecraft:compass", true), 0, 0);
        let materials: Vec<&str> = painter.quads.iter().map(|q| q.material.as_str()).collect();
        assert_eq!(materials, [ANIMATED_ICON_MATERIAL, ANIMATED_ICON_MATERIAL]);
        assert!(painter.quads[1].glint.is_some());
        let gun = McStack {
            weapon: Some(3),
            ..shiny("iw4:weapon/3", true)
        };
        let mut painter = Painter::new(&gui, space(1));
        painter.item(&ui, None, &gun, 0, 0);
        assert!(painter.quads.is_empty());
    }
}
