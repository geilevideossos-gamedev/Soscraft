use bevy::prelude::*;
use frame::MinecraftUi;

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::gpu_list::{HudTessGpuFrame, HudTessPass, TessBody, TessJob, apply_tess_job};
use crate::images::{HUD_CHROME_NAMESPACE, HudImages};

const MATERIAL: &str = "mc_powder_snow_outline";

#[derive(Component)]
pub(crate) struct MinecraftFreezeRaster;

pub(crate) fn outline(size: [f32; 2], frozen: f32) -> Option<Draw2dQuad> {
    if frozen <= 0.0 {
        return None;
    }
    let [width, height] = size;
    Some(Draw2dQuad {
        xy: [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]],
        st: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        color: [1.0, 1.0, 1.0, frozen.min(1.0)],
        material: MATERIAL.to_owned(),
        material_namespace: HUD_CHROME_NAMESPACE,
        provenance: Draw2dProvenance::CgDraw {
            site: "minecraft_freeze",
        },
        layer: 0,
        clip: None,
        glint: None,
    })
}

pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    ui: Option<Res<MinecraftUi>>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
) {
    pass.minecraft_freeze = TessJob::Hide;
    let Some(ui) = ui else {
        return;
    };
    let (Some(vitals), Some(texture)) = (ui.vitals.as_ref(), ui.freeze_outline.as_ref()) else {
        return;
    };
    if !ui.active || !surface.is_ready() {
        return;
    }
    let Some(quad) = outline([surface.width(), surface.height()], vitals.frozen) else {
        return;
    };
    hud_images.insert_runtime(MATERIAL, texture.clone());
    pass.minecraft_freeze = TessJob::Quads(vec![quad]);
}

pub(crate) fn flush(
    surface: Res<crate::surface::Hud2dSurface>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut frame: ResMut<HudTessGpuFrame>,
    mut host: Query<(&mut Node, &mut crate::gpu_list::GpuListLatch), With<MinecraftFreezeRaster>>,
) {
    let _body = TessBody::open();
    if !surface.is_ready() {
        return;
    }
    let job = std::mem::take(&mut pass.minecraft_freeze);
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
    fn the_outline_fades_with_the_frost() {
        assert!(outline([1920.0, 1080.0], 0.0).is_none());
        let half = outline([1920.0, 1080.0], 0.5).expect("half frozen shows the outline");
        assert_eq!(
            half.xy,
            [[0.0, 0.0], [1920.0, 0.0], [1920.0, 1080.0], [0.0, 1080.0]]
        );
        assert_eq!(half.st, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        assert_eq!(half.color, [1.0, 1.0, 1.0, 0.5]);
        let full = outline([800.0, 600.0], 1.0).expect("full frost shows the outline");
        assert_eq!(full.color[3], 1.0);
        assert_eq!(full.material, MATERIAL);
    }
}
