use bevy::prelude::*;
use frame::MinecraftUi;

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::gpu_list::{HudTessGpuFrame, HudTessPass, TessBody, TessJob, apply_tess_job};
use crate::images::{HUD_CHROME_NAMESPACE, HudImages};

const MATERIAL: &str = "mc_portal_overlay";

#[derive(Component)]
pub(crate) struct MinecraftPortalRaster;

pub(crate) fn frame_st(frame: u64, frames: u32) -> [f32; 4] {
    let count = frames.max(1);
    let at = (frame % u64::from(count)) as f32;
    let count = count as f32;
    [0.0, at / count, 1.0, (at + 1.0) / count]
}

pub(crate) fn overlay(size: [f32; 2], st: [f32; 4], alpha: f32) -> Draw2dQuad {
    let [width, height] = size;
    let [s0, t0, s1, t1] = st;
    Draw2dQuad {
        xy: [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]],
        st: [[s0, t0], [s1, t0], [s1, t1], [s0, t1]],
        color: [1.0, 1.0, 1.0, alpha],
        material: MATERIAL.to_owned(),
        material_namespace: HUD_CHROME_NAMESPACE,
        provenance: Draw2dProvenance::CgDraw {
            site: "minecraft_portal",
        },
        layer: 0,
        clip: None,
        glint: None,
    }
}

pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    ui: Option<Res<MinecraftUi>>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
) {
    pass.minecraft_portal = TessJob::Hide;
    let Some(ui) = ui else {
        return;
    };
    let (Some(shown), Some((strip, frames))) = (ui.portal_overlay, ui.portal_frames.as_ref())
    else {
        return;
    };
    if !ui.active || !surface.is_ready() {
        return;
    }
    hud_images.insert_runtime(MATERIAL, strip.clone());
    let size = [surface.width(), surface.height()];
    pass.minecraft_portal = TessJob::Quads(vec![overlay(
        size,
        frame_st(shown.frame, *frames),
        shown.alpha,
    )]);
}

pub(crate) fn flush(
    surface: Res<crate::surface::Hud2dSurface>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut frame: ResMut<HudTessGpuFrame>,
    mut host: Query<(&mut Node, &mut crate::gpu_list::GpuListLatch), With<MinecraftPortalRaster>>,
) {
    let _body = TessBody::open();
    if !surface.is_ready() {
        return;
    }
    let job = std::mem::take(&mut pass.minecraft_portal);
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
    fn the_portal_overlay_covers_the_screen_with_one_animation_frame() {
        assert_eq!(frame_st(0, 32), [0.0, 0.0, 1.0, 1.0 / 32.0]);
        assert_eq!(frame_st(33, 32), [0.0, 1.0 / 32.0, 1.0, 2.0 / 32.0]);
        assert_eq!(frame_st(7, 0), [0.0, 0.0, 1.0, 1.0]);
        let quad = overlay([1920.0, 1080.0], frame_st(0, 32), 0.25);
        assert_eq!(
            quad.xy,
            [[0.0, 0.0], [1920.0, 0.0], [1920.0, 1080.0], [0.0, 1080.0]]
        );
        assert_eq!(quad.color, [1.0, 1.0, 1.0, 0.25]);
        assert_eq!(quad.st[2], [1.0, 1.0 / 32.0]);
    }
}
