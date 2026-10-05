use bevy::prelude::*;
use frame::MinecraftUi;

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::gpu_list::{HudTessGpuFrame, HudTessPass, TessBody, TessJob, apply_tess_job};
use crate::images::{HUD_CHROME_NAMESPACE, HudImages};

const MATERIAL: &str = "mc_fire_overlay";
const HUD_FOV_HALF_TAN: f32 = 0.700_207_5;
const FLUID_HUD_FOV_HALF_TAN: f32 = 0.577_350_3;
const QUAD_HALF: f32 = 0.5;
const QUAD_DEPTH: f32 = 0.5;
const SHIFT_X: f32 = 0.24;
const SHIFT_Y: f32 = -0.3;
const TURN: f32 = 0.174_532_92;
const STRIPS: usize = 16;
const TINT: [f32; 4] = [1.0, 1.0, 1.0, 229.0 / 255.0];

#[derive(Component)]
pub(crate) struct MinecraftFireRaster;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Strip {
    xy: [[f32; 2]; 4],
    s: [f32; 2],
}

fn hud_fov_half_tan(eye_in_lava: bool) -> f32 {
    if eye_in_lava {
        FLUID_HUD_FOV_HALF_TAN
    } else {
        HUD_FOV_HALF_TAN
    }
}

fn project(side: f32, x: f32, y: f32, size: [f32; 2], half_tan: f32) -> [f32; 2] {
    let [width, height] = size;
    let (sin, cos) = (-side * TURN).sin_cos();
    let view_x = cos * x - sin * QUAD_DEPTH + side * SHIFT_X;
    let depth = sin * x + cos * QUAD_DEPTH;
    let view_y = y + SHIFT_Y;
    [
        (1.0 + view_x / depth / (half_tan * width / height)) * width * 0.5,
        (1.0 - view_y / depth / half_tan) * height * 0.5,
    ]
}

fn flames(size: [f32; 2], half_tan: f32) -> Vec<Strip> {
    [1.0, -1.0]
        .into_iter()
        .flat_map(|side| flame(side, size, half_tan))
        .collect()
}

fn flame(side: f32, size: [f32; 2], half_tan: f32) -> Vec<Strip> {
    let [width, height] = size;
    if width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }
    let edge = |n: usize| (n as f32 / STRIPS as f32 * 2.0 - 1.0) * QUAD_HALF;
    let sprite_s = |x: f32| (QUAD_HALF - x) / (2.0 * QUAD_HALF);
    (0..STRIPS)
        .map(|n| {
            let (a, b) = (edge(n), edge(n + 1));
            let at = |x: f32, y: f32| project(side, x, y, size, half_tan);
            Strip {
                xy: [
                    at(a, QUAD_HALF),
                    at(b, QUAD_HALF),
                    at(b, -QUAD_HALF),
                    at(a, -QUAD_HALF),
                ],
                s: [sprite_s(a), sprite_s(b)],
            }
        })
        .filter(|strip| {
            let xs = strip.xy.map(|corner| corner[0]);
            let ys = strip.xy.map(|corner| corner[1]);
            xs.iter().any(|x| *x < width)
                && xs.iter().any(|x| *x > 0.0)
                && ys.iter().any(|y| *y < height)
                && ys.iter().any(|y| *y > 0.0)
        })
        .collect()
}

pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    ui: Option<Res<MinecraftUi>>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
) {
    pass.minecraft_fire = TessJob::Hide;
    let Some(ui) = ui else {
        return;
    };
    let burning = ui.vitals.as_ref().filter(|vitals| vitals.on_fire);
    let (Some(vitals), Some((strip, frames))) = (burning, ui.fire_frames.as_ref()) else {
        return;
    };
    if !ui.active || !surface.is_ready() {
        return;
    }
    hud_images.insert_runtime(MATERIAL, strip.clone());
    let count = (*frames).max(1);
    let frame = (vitals.tick % u64::from(count)) as f32;
    let count = count as f32;
    let (t0, t1) = (frame / count, (frame + 1.0) / count);
    let size = [surface.width(), surface.height()];
    let quads: Vec<Draw2dQuad> = flames(size, hud_fov_half_tan(vitals.eye_in_lava))
        .into_iter()
        .map(|strip| Draw2dQuad {
            xy: strip.xy,
            st: [
                [strip.s[0], t0],
                [strip.s[1], t0],
                [strip.s[1], t1],
                [strip.s[0], t1],
            ],
            color: TINT,
            material: MATERIAL.to_owned(),
            material_namespace: HUD_CHROME_NAMESPACE,
            provenance: Draw2dProvenance::CgDraw {
                site: "minecraft_fire",
            },
            layer: 0,
            clip: None,
        })
        .collect();
    if !quads.is_empty() {
        pass.minecraft_fire = TessJob::Quads(quads);
    }
}

pub(crate) fn flush(
    surface: Res<crate::surface::Hud2dSurface>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut frame: ResMut<HudTessGpuFrame>,
    mut host: Query<(&mut Node, &mut crate::gpu_list::GpuListLatch), With<MinecraftFireRaster>>,
) {
    let _body = TessBody::open();
    if !surface.is_ready() {
        return;
    }
    let job = std::mem::take(&mut pass.minecraft_fire);
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

    const WIDE: [f32; 2] = [1920.0, 1080.0];

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1.0 && (a[1] - b[1]).abs() < 1.0
    }

    #[test]
    fn the_right_flame_is_vanillas_turned_quad_through_the_hud_fov() {
        let inner_top = project(1.0, -QUAD_HALF, QUAD_HALF, WIDE, HUD_FOV_HALF_TAN);
        assert!(
            near(inner_top, [0.3851 * 1920.0, 0.2534 * 1080.0]),
            "{inner_top:?}"
        );
        let at_the_edge = project(1.0, 0.23826, QUAD_HALF, WIDE, HUD_FOV_HALF_TAN);
        assert!(
            near(at_the_edge, [1920.0, 0.1834 * 1080.0]),
            "{at_the_edge:?}"
        );
        for x in [-QUAD_HALF, 0.0, QUAD_HALF] {
            let top = project(1.0, x, QUAD_HALF, WIDE, HUD_FOV_HALF_TAN);
            let bottom = project(1.0, x, -QUAD_HALF, WIDE, HUD_FOV_HALF_TAN);
            assert!((top[0] - bottom[0]).abs() < 1e-3 && bottom[1] > 1080.0);
        }
    }

    #[test]
    fn the_flames_mirror_each_other_and_both_show_the_sprite_flipped() {
        let right = flame(1.0, WIDE, HUD_FOV_HALF_TAN);
        let left = flame(-1.0, WIDE, HUD_FOV_HALF_TAN);
        assert_eq!(
            flames(WIDE, HUD_FOV_HALF_TAN),
            [right.clone(), left.clone()].concat()
        );
        assert_eq!(right.len(), left.len());
        assert_eq!(right[0].s[0], 1.0);
        assert_eq!(left.last().map(|strip| strip.s[1]), Some(0.0));
        for (r, l) in right.iter().zip(left.iter().rev()) {
            assert!(near([1920.0 - r.xy[0][0], r.xy[0][1]], l.xy[1]));
            assert!(near([1920.0 - r.xy[2][0], r.xy[2][1]], l.xy[3]));
            assert!((r.s[0] - (1.0 - l.s[1])).abs() < 1e-6);
            assert!(r.s[0] > r.s[1] && l.s[0] > l.s[1]);
        }
        assert!(
            right.len() < STRIPS,
            "strips past the screen edge are dropped"
        );
    }

    #[test]
    fn eyes_in_lava_narrow_the_hud_fov_and_raise_the_flames() {
        let clear = project(1.0, -QUAD_HALF, QUAD_HALF, WIDE, hud_fov_half_tan(false));
        let in_lava = project(1.0, -QUAD_HALF, QUAD_HALF, WIDE, hud_fov_half_tan(true));
        assert!(in_lava[1] < clear[1] && in_lava[0] < clear[0]);
        assert!(flames([0.0, 1080.0], HUD_FOV_HALF_TAN).is_empty());
    }
}
