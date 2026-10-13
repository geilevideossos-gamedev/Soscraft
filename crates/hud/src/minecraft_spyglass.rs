use bevy::prelude::*;
use frame::{McCamera, MinecraftUi};
use net::FrameClock;

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::gpu_list::{HudTessGpuFrame, HudTessPass, TessBody, TessJob, apply_tess_job};
use crate::images::{HUD_CHROME_NAMESPACE, HudImages};

const MATERIAL: &str = "mc_spyglass_scope";
const BLUR_MATERIAL: &str = "mc_pumpkin_blur";
const BLACK_MATERIAL: &str = "white";
const RESET_SCALE: f32 = 0.5;
const FULL_SCALE: f32 = 1.125;
const TICKS_PER_SECOND: f32 = 20.0;

#[derive(Component)]
pub(crate) struct MinecraftSpyglassRaster;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CameraOverlay {
    None,
    Scope,
    PumpkinBlur,
}

pub(crate) fn overlay_job(
    active: bool,
    scoping: bool,
    first_person: bool,
    wearing: bool,
    has_texture: bool,
) -> CameraOverlay {
    if !active || !first_person {
        CameraOverlay::None
    } else if scoping {
        CameraOverlay::Scope
    } else if wearing && has_texture {
        CameraOverlay::PumpkinBlur
    } else {
        CameraOverlay::None
    }
}

pub(crate) struct ScopeScale(f32);

impl Default for ScopeScale {
    fn default() -> Self {
        Self(RESET_SCALE)
    }
}

pub(crate) fn grow(scale: f32, seconds: f32) -> f32 {
    let delta = (0.5 * seconds * TICKS_PER_SECOND).clamp(0.0, 1.0);
    scale + delta * (FULL_SCALE - scale)
}

fn quad(rect: [f32; 4], material: &str, color: [f32; 4]) -> Draw2dQuad {
    let [x0, y0, x1, y1] = rect;
    Draw2dQuad {
        xy: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
        st: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        color,
        material: material.to_owned(),
        material_namespace: HUD_CHROME_NAMESPACE,
        provenance: Draw2dProvenance::CgDraw {
            site: "minecraft_spyglass",
        },
        layer: 0,
        clip: None,
        glint: None,
    }
}

pub(crate) fn overlay(size: [f32; 2], scale: f32) -> Vec<Draw2dQuad> {
    let [width, height] = size;
    let side = (width.min(height) * scale).floor();
    let left = ((width - side) / 2.0).trunc();
    let top = ((height - side) / 2.0).trunc();
    let (right, bottom) = (left + side, top + side);
    let black = [0.0, 0.0, 0.0, 1.0];
    let mut quads = vec![quad(
        [left, top, right, bottom],
        MATERIAL,
        [1.0, 1.0, 1.0, 1.0],
    )];
    for rect in [
        [0.0, bottom, width, height],
        [0.0, 0.0, width, top],
        [0.0, top, left, bottom],
        [right, top, width, bottom],
    ] {
        if rect[2] > rect[0] && rect[3] > rect[1] {
            quads.push(quad(rect, BLACK_MATERIAL, black));
        }
    }
    quads
}

pub(crate) fn camera_overlay_quad(size: [f32; 2]) -> Draw2dQuad {
    quad([0.0, 0.0, size[0], size[1]], BLUR_MATERIAL, [1.0; 4])
}

pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    clock: Res<FrameClock>,
    ui: Option<Res<MinecraftUi>>,
    camera: Option<Res<McCamera>>,
    mut scale: Local<ScopeScale>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
) {
    pass.minecraft_spyglass = TessJob::Hide;
    scale.0 = grow(scale.0, clock.frametime_secs());
    let Some(ui) = ui else {
        scale.0 = RESET_SCALE;
        return;
    };
    let first_person = !camera.is_some_and(|camera| camera.detached);
    let job = overlay_job(
        ui.active,
        ui.scoping,
        first_person,
        ui.camera_overlay,
        ui.pumpkin_blur.is_some(),
    );
    if let (CameraOverlay::PumpkinBlur, Some(blur)) = (job, ui.pumpkin_blur.as_ref()) {
        scale.0 = RESET_SCALE;
        if surface.is_ready() {
            hud_images.insert_runtime(BLUR_MATERIAL, blur.clone());
            let size = [surface.width(), surface.height()];
            pass.minecraft_spyglass = TessJob::Quads(vec![camera_overlay_quad(size)]);
        }
        return;
    }
    let Some(texture) = ui
        .spyglass_scope
        .as_ref()
        .filter(|_| job == CameraOverlay::Scope)
    else {
        scale.0 = RESET_SCALE;
        return;
    };
    if !surface.is_ready() {
        return;
    }
    hud_images.insert_runtime(MATERIAL, texture.clone());
    pass.minecraft_spyglass = TessJob::Quads(overlay([surface.width(), surface.height()], scale.0));
}

pub(crate) fn flush(
    surface: Res<crate::surface::Hud2dSurface>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut frame: ResMut<HudTessGpuFrame>,
    mut host: Query<(&mut Node, &mut crate::gpu_list::GpuListLatch), With<MinecraftSpyglassRaster>>,
) {
    let _body = TessBody::open();
    if !surface.is_ready() {
        return;
    }
    let job = std::mem::take(&mut pass.minecraft_spyglass);
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
    fn the_scope_grows_from_half_to_its_full_size_by_half_a_tick_per_frame() {
        let tick = 1.0 / TICKS_PER_SECOND;
        assert!((grow(RESET_SCALE, tick) - 0.8125).abs() < 1.0e-6);
        assert!((grow(RESET_SCALE, 2.0 * tick) - FULL_SCALE).abs() < 1.0e-6);
        assert!((grow(RESET_SCALE, 10.0) - FULL_SCALE).abs() < 1.0e-6);
        assert_eq!(grow(FULL_SCALE, tick), FULL_SCALE);
    }

    #[test]
    fn a_wide_screen_shows_the_scope_centred_with_black_beside_it() {
        let quads = overlay([1920.0, 1080.0], FULL_SCALE);
        assert_eq!(quads[0].material, MATERIAL);
        assert_eq!(quads[0].xy[0], [352.0, -67.0]);
        assert_eq!(quads[0].xy[2], [1567.0, 1148.0]);
        let sides: Vec<_> = quads[1..].iter().map(|q| (q.xy[0], q.xy[2])).collect();
        assert_eq!(
            sides,
            vec![
                ([0.0, -67.0], [352.0, 1148.0]),
                ([1567.0, -67.0], [1920.0, 1148.0])
            ]
        );
        assert!(quads[1..].iter().all(|q| q.color == [0.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn the_pumpkin_blur_covers_the_screen_only_in_first_person_and_not_while_scoping() {
        assert_eq!(
            overlay_job(true, false, true, true, true),
            CameraOverlay::PumpkinBlur
        );
        assert_eq!(
            overlay_job(true, true, true, true, true),
            CameraOverlay::Scope
        );
        assert_eq!(
            overlay_job(true, true, true, false, false),
            CameraOverlay::Scope
        );
        for (active, first_person, wearing, has_texture) in [
            (true, false, true, true),
            (false, true, true, true),
            (true, true, false, true),
            (true, true, true, false),
        ] {
            assert_eq!(
                overlay_job(active, false, first_person, wearing, has_texture),
                CameraOverlay::None
            );
        }
        assert_eq!(
            overlay_job(true, true, false, true, true),
            CameraOverlay::None
        );
        let quad = camera_overlay_quad([1920.0, 1080.0]);
        assert_eq!(quad.material, BLUR_MATERIAL);
        assert_eq!(
            quad.xy,
            [[0.0, 0.0], [1920.0, 0.0], [1920.0, 1080.0], [0.0, 1080.0]]
        );
        assert_eq!(quad.st, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        assert_eq!(quad.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn a_small_scope_is_framed_on_all_four_sides() {
        let quads = overlay([800.0, 600.0], RESET_SCALE);
        assert_eq!(quads[0].xy[0], [250.0, 150.0]);
        assert_eq!(quads[0].xy[2], [550.0, 450.0]);
        assert_eq!(quads.len(), 5);
    }
}
