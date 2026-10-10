use bevy::prelude::*;
use frame::MinecraftUi;
use frame::minecraft_credits::{McCreditLine, McCredits};
use frame::minecraft_gui::McFont;

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::gpu_list::{HudTessGpuFrame, HudTessPass, TessBody, TessJob, apply_tess_job};
use crate::images::{HUD_CHROME_NAMESPACE, HudImages};
use crate::minecraft_gui::{GUI_MATERIAL, GuiSpace, Painter, TextLine};

const LOGO_MATERIAL: &str = "mc_credits_logo";
const EDITION_MATERIAL: &str = "mc_credits_edition";
const LOGO_SIZE: [f32; 2] = [256.0, 44.0];
const LOGO_TEXTURE_HEIGHT: f32 = 64.0;
const EDITION_SIZE: [f32; 2] = [128.0, 14.0];
const EDITION_TEXTURE_HEIGHT: f32 = 16.0;
const EDITION_OVERLAP: f32 = 7.0;
const OBFUSCATION_GLYPHS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

#[derive(Component)]
pub(crate) struct MinecraftCreditsRaster;

pub(crate) fn column_left(width: i32) -> i32 {
    width / 2 - 128
}

pub(crate) fn obfuscate(text: &str, font: &McFont, seed: &mut u64) -> String {
    text.chars()
        .map(|c| {
            let width = font.advance(c);
            let same: Vec<char> = OBFUSCATION_GLYPHS
                .chars()
                .filter(|&other| font.advance(other) == width)
                .collect();
            if same.is_empty() {
                return c;
            }
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            same[(*seed % same.len() as u64) as usize]
        })
        .collect()
}

pub(crate) fn text_line(line: &McCreditLine, font: &McFont, seed: &mut u64) -> TextLine {
    let mut text = String::new();
    let mut accents = Vec::new();
    for span in &line.spans {
        let start = text.len();
        if span.obfuscated {
            text.push_str(&obfuscate(&span.text, font, seed));
        } else {
            text.push_str(&span.text);
        }
        accents.push((start..text.len(), span.color));
    }
    TextLine {
        text,
        color: frame::minecraft_credits::WHITE,
        italic: false,
        accents,
    }
}

fn image_quad(space: &GuiSpace, material: &str, area: [f32; 4], t1: f32) -> Draw2dQuad {
    let [x0, y0, x1, y1] = area;
    Draw2dQuad {
        xy: [
            space.vertex(x0, y0),
            space.vertex(x1, y0),
            space.vertex(x1, y1),
            space.vertex(x0, y1),
        ],
        st: [[0.0, 0.0], [1.0, 0.0], [1.0, t1], [0.0, t1]],
        color: [1.0; 4],
        material: material.to_owned(),
        material_namespace: HUD_CHROME_NAMESPACE,
        provenance: Draw2dProvenance::CgDraw {
            site: "minecraft_credits",
        },
        layer: 1,
        clip: None,
        glint: None,
    }
}

pub(crate) fn logo_areas(width: i32, logo_y: f32) -> ([f32; 4], [f32; 4]) {
    let left = (width / 2 - 128) as f32;
    let logo = [left, logo_y, left + LOGO_SIZE[0], logo_y + LOGO_SIZE[1]];
    let edition_left = (width / 2 - 64) as f32;
    let edition_top = logo_y + LOGO_SIZE[1] - EDITION_OVERLAP;
    let edition = [
        edition_left,
        edition_top,
        edition_left + EDITION_SIZE[0],
        edition_top + EDITION_SIZE[1],
    ];
    (logo, edition)
}

pub(crate) fn steer(credits: &mut McCredits, keys: &ButtonInput<KeyCode>) {
    if keys.just_pressed(KeyCode::Escape) {
        credits.skip();
    }
    credits.speedup = keys.pressed(KeyCode::Space);
    credits.fast_keys = [KeyCode::ControlLeft, KeyCode::ControlRight]
        .into_iter()
        .filter(|&key| keys.pressed(key))
        .count() as u32;
    credits.reverse = keys.pressed(KeyCode::ArrowUp);
}

pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    ui: Option<ResMut<MinecraftUi>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
) {
    pass.minecraft_credits = TessJob::Hide;
    let Some(mut ui) = ui else {
        return;
    };
    let Some(gui) = ui.gui.clone() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    if !surface.is_ready() {
        return;
    }
    let space = GuiSpace::new(
        [window.physical_width(), window.physical_height()],
        [surface.width(), surface.height()],
        [window.width(), window.height()],
    );
    let Some(credits) = ui.credits.as_mut() else {
        return;
    };
    steer(credits, &keys);
    credits.advance(time.delta_secs() * 20.0, space.height);
    let mut quads = Vec::new();
    let (logo, edition) = logo_areas(space.width, credits.logo_y(space.height));
    if let Some(image) = credits.logo.clone() {
        hud_images.insert_runtime(LOGO_MATERIAL, image);
        quads.push(image_quad(
            &space,
            LOGO_MATERIAL,
            logo,
            LOGO_SIZE[1] / LOGO_TEXTURE_HEIGHT,
        ));
    }
    if let Some(image) = credits.edition.clone() {
        hud_images.insert_runtime(EDITION_MATERIAL, image);
        quads.push(image_quad(
            &space,
            EDITION_MATERIAL,
            edition,
            EDITION_SIZE[1] / EDITION_TEXTURE_HEIGHT,
        ));
    }
    hud_images.insert_runtime(GUI_MATERIAL, gui.image.clone());
    let mut painter = Painter::new(&gui, space);
    let mut seed = (time.elapsed().as_nanos() as u64) | 1;
    let left = column_left(space.width);
    for (index, line) in credits.lines.iter().enumerate() {
        if line.spans.is_empty()
            || !McCredits::visible(credits.natural_y(index, space.height), space.height)
        {
            continue;
        }
        let y = credits.line_y(index, space.height);
        let text = text_line(line, &gui.font, &mut seed);
        let x = if line.centered {
            left + 128 - painter.width(&text.text) / 2
        } else {
            left
        };
        let start = painter.quads.len();
        painter.line(false, &text, x, y.floor() as i32, true);
        let shift = space.vertex(0.0, y - y.floor())[1] - space.vertex(0.0, 0.0)[1];
        for quad in &mut painter.quads[start..] {
            for corner in &mut quad.xy {
                corner[1] += shift;
            }
        }
    }
    quads.append(&mut painter.quads);
    pass.minecraft_credits = TessJob::Quads(quads);
}

pub(crate) fn flush(
    surface: Res<crate::surface::Hud2dSurface>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut frame: ResMut<HudTessGpuFrame>,
    mut host: Query<(&mut Node, &mut crate::gpu_list::GpuListLatch), With<MinecraftCreditsRaster>>,
) {
    let _body = TessBody::open();
    if !surface.is_ready() {
        return;
    }
    let job = std::mem::take(&mut pass.minecraft_credits);
    if let Ok((mut node, mut latch)) = host.single_mut() {
        apply_tess_job(
            job,
            &mut node,
            &mut latch,
            &mut hud_images,
            &mut images,
            &mut frame,
            surface.width(),
            surface.height(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::minecraft_credits::McCreditSpan;

    #[test]
    fn the_logo_sits_centred_with_the_edition_overlapping_its_foot() {
        let (logo, edition) = logo_areas(480, 300.0);
        assert_eq!(logo, [112.0, 300.0, 368.0, 344.0]);
        assert_eq!(edition, [176.0, 337.0, 304.0, 351.0]);
        assert_eq!(column_left(480), 112);
    }

    #[test]
    fn a_line_keeps_its_span_colours_and_hides_obfuscated_words() {
        let font = McFont::default();
        let line = McCreditLine {
            spans: vec![
                McCreditSpan {
                    text: "piece of the ".to_owned(),
                    color: 0xFF00_AAAA,
                    obfuscated: false,
                },
                McCreditSpan {
                    text: "XXXX".to_owned(),
                    color: 0xFFFF_FFFF,
                    obfuscated: true,
                },
                McCreditSpan {
                    text: ", and".to_owned(),
                    color: 0xFF00_AAAA,
                    obfuscated: false,
                },
            ],
            centered: false,
        };
        let mut seed = 7;
        let text = text_line(&line, &font, &mut seed);
        assert_eq!(text.text.len(), "piece of the XXXX, and".len());
        assert!(text.text.starts_with("piece of the "));
        assert!(text.text.ends_with(", and"));
        assert_ne!(&text.text[13..17], "XXXX", "the hidden word changes");
        assert_eq!(
            text.accents,
            [
                (0..13, 0xFF00_AAAA),
                (13..17, 0xFFFF_FFFF),
                (17..22, 0xFF00_AAAA)
            ]
        );
        assert_eq!(font.width(&text.text), font.width("piece of the XXXX, and"));
    }

    #[test]
    fn the_credits_listen_to_escape_space_control_and_up() {
        let mut credits = McCredits::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Space);
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::ArrowUp);
        steer(&mut credits, &keys);
        assert!(credits.speedup && credits.reverse && !credits.finished);
        assert_eq!(credits.fast_keys, 1);
        keys.press(KeyCode::Escape);
        steer(&mut credits, &keys);
        assert!(credits.finished);
    }
}
