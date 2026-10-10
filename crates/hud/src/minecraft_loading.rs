use bevy::prelude::*;
use frame::{McLevelLoading, MinecraftUi};

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::gpu_list::{HudTessGpuFrame, HudTessPass, TessBody, TessJob, apply_tess_job};
use crate::images::{HUD_CHROME_NAMESPACE, HudImages};
use crate::minecraft_gui::{GUI_MATERIAL, GuiSpace, Painter, WHITE};

const BACKGROUND_MATERIAL: &str = "mc_level_loading";
const MENU_MATERIAL: &str = "mc_level_loading_menu";
const MENU_TILE: i32 = 32;
const DOWNLOADING_TERRAIN: &str = "multiplayer.downloadingTerrain";
const BLACK: u32 = 0xFF00_0000;

#[derive(Component)]
pub(crate) struct MinecraftLoadingRaster;

pub(crate) fn menu_tiles(width: i32, height: i32) -> Vec<([i32; 4], [f32; 4])> {
    let mut tiles = Vec::new();
    let mut y = 0;
    while y < height {
        let h = MENU_TILE.min(height - y);
        let mut x = 0;
        while x < width {
            let w = MENU_TILE.min(width - x);
            tiles.push((
                [x, y, x + w, y + h],
                [
                    0.0,
                    0.0,
                    w as f32 / MENU_TILE as f32,
                    h as f32 / MENU_TILE as f32,
                ],
            ));
            x += MENU_TILE;
        }
        y += MENU_TILE;
    }
    tiles
}

pub(crate) fn text_origin(space: [i32; 2], text_width: i32) -> [i32; 2] {
    [space[0] / 2 - text_width / 2, space[1] / 2 - 50]
}

fn quad(
    space: &GuiSpace,
    material: &str,
    [x0, y0, x1, y1]: [f32; 4],
    [s0, t0, s1, t1]: [f32; 4],
) -> Draw2dQuad {
    Draw2dQuad {
        xy: [
            space.vertex(x0, y0),
            space.vertex(x1, y0),
            space.vertex(x1, y1),
            space.vertex(x0, y1),
        ],
        st: [[s0, t0], [s1, t0], [s1, t1], [s0, t1]],
        color: [1.0; 4],
        material: material.to_owned(),
        material_namespace: HUD_CHROME_NAMESPACE,
        provenance: Draw2dProvenance::CgDraw {
            site: "minecraft_loading",
        },
        layer: 1,
        clip: None,
        glint: None,
    }
}

fn background(
    space: &GuiSpace,
    loading: &McLevelLoading,
    hud_images: &mut HudImages,
) -> Vec<Draw2dQuad> {
    let mut quads = Vec::new();
    let full = [0.0, 0.0, space.width as f32, space.height as f32];
    if let Some(image) = loading.background.clone() {
        hud_images.insert_runtime(BACKGROUND_MATERIAL, image);
        quads.push(quad(
            space,
            BACKGROUND_MATERIAL,
            full,
            loading.background_st,
        ));
    }
    if let Some(menu) = loading.menu_background.clone() {
        hud_images.insert_runtime(MENU_MATERIAL, menu);
        for (rect, st) in menu_tiles(space.width, space.height) {
            quads.push(quad(space, MENU_MATERIAL, rect.map(|v| v as f32), st));
        }
    }
    quads
}

pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    ui: Option<Res<MinecraftUi>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
) {
    pass.minecraft_loading = TessJob::Hide;
    let Some(ui) = ui else {
        return;
    };
    let (Some(loading), Some(gui), Ok(window)) =
        (ui.level_loading.as_ref(), ui.gui.clone(), windows.single())
    else {
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
    hud_images.insert_runtime(GUI_MATERIAL, gui.image.clone());
    let mut painter = Painter::new(&gui, space);
    if loading.background.is_none() {
        painter.fill(0, 0, space.width, space.height, BLACK);
    }
    let mut quads = background(&space, loading, &mut hud_images);
    if ui.credits.is_none() {
        let text = gui.text(DOWNLOADING_TERRAIN);
        let [x, y] = text_origin([space.width, space.height], painter.width(&text));
        painter.text(&text, x, y, WHITE, true);
    }
    let mut drawn = std::mem::take(&mut painter.quads);
    if loading.background.is_none() {
        quads.splice(0..0, drawn.drain(..1));
    }
    quads.extend(drawn);
    pass.minecraft_loading = TessJob::Quads(quads);
}

pub(crate) fn flush(
    surface: Res<crate::surface::Hud2dSurface>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut frame: ResMut<HudTessGpuFrame>,
    mut host: Query<(&mut Node, &mut crate::gpu_list::GpuListLatch), With<MinecraftLoadingRaster>>,
) {
    let _body = TessBody::open();
    if !surface.is_ready() {
        return;
    }
    let job = std::mem::take(&mut pass.minecraft_loading);
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

    #[test]
    fn the_menu_background_tiles_the_screen_in_thirty_two_pixel_squares() {
        let tiles = menu_tiles(80, 40);
        assert_eq!(tiles.len(), 6);
        assert_eq!(tiles[0], ([0, 0, 32, 32], [0.0, 0.0, 1.0, 1.0]));
        assert_eq!(tiles[2], ([64, 0, 80, 32], [0.0, 0.0, 0.5, 1.0]));
        assert_eq!(tiles[5], ([64, 32, 80, 40], [0.0, 0.0, 0.5, 0.25]));
        assert!(menu_tiles(0, 10).is_empty());
    }

    #[test]
    fn loading_terrain_sits_centred_fifty_pixels_above_the_middle() {
        assert_eq!(text_origin([480, 270], 101), [190, 85]);
        assert_eq!(text_origin([427, 240], 100), [163, 70]);
    }
}
