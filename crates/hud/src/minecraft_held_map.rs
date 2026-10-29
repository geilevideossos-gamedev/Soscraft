use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use frame::MinecraftUi;
use frame::minecraft_ui::McMapHands;

use crate::draw2d::{Draw2dProvenance, Draw2dQuad};
use crate::gpu_list::{HudTessGpuFrame, HudTessPass, TessBody, TessJob, apply_tess_job};
use crate::images::{HUD_CHROME_NAMESPACE, HudImages};

const MATERIALS: [&str; 2] = ["mc_held_map_main", "mc_held_map_offhand"];
const HAND_FOV_DEGREES: f32 = 70.0;
const NEAR: f32 = 0.05;
const MAP_UNITS: f32 = 128.0;
const BORDER: f32 = 7.0;
const FRAME_UNITS: f32 = MAP_UNITS + 2.0 * BORDER;
const CELLS: usize = 16;

#[derive(Component)]
pub(crate) struct MinecraftHeldMapRaster;

pub(crate) fn tilt(pitch: f32) -> f32 {
    let tilt = (1.0 - pitch / 45.0 + 0.1).clamp(0.0, 1.0);
    -(tilt * PI).cos() * 0.5 + 0.5
}

pub(crate) fn pose(hands: McMapHands, pitch: f32, swing: f32) -> Mat4 {
    let root = swing.sqrt();
    let lift = (root * PI).sin();
    let held = match hands {
        McMapHands::Both => {
            let tilt = tilt(pitch);
            Mat4::from_translation(Vec3::new(0.0, 0.1 * (swing * PI).sin(), -0.4 * lift))
                * Mat4::from_translation(Vec3::new(0.0, 0.04 - 0.5 * tilt, -0.72))
                * Mat4::from_rotation_x((-85.0 * tilt).to_radians())
                * Mat4::from_rotation_x((20.0 * lift).to_radians())
                * Mat4::from_scale(Vec3::splat(2.0))
        }
        McMapHands::Right | McMapHands::Left => {
            let side = if hands == McMapHands::Right {
                1.0
            } else {
                -1.0
            };
            let sway = Vec3::new(
                side * -0.5 * lift,
                0.4 * (root * TAU).sin() - 0.3 * lift,
                -0.3 * (swing * PI).sin(),
            );
            Mat4::from_translation(Vec3::new(side * 0.125, -0.125, 0.0))
                * Mat4::from_translation(Vec3::new(side * 0.51, -0.08, -0.75))
                * Mat4::from_translation(sway)
                * Mat4::from_rotation_x((-45.0 * lift).to_radians())
                * Mat4::from_rotation_y((side * lift * -30.0).to_radians())
        }
    };
    held * Mat4::from_rotation_y(PI)
        * Mat4::from_rotation_z(PI)
        * Mat4::from_scale(Vec3::splat(0.38))
        * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0))
        * Mat4::from_scale(Vec3::splat(1.0 / MAP_UNITS))
}

pub(crate) fn to_screen(view: Vec3, size: [f32; 2]) -> Option<[f32; 2]> {
    let depth = -view.z;
    if depth < NEAR {
        return None;
    }
    let focal = 1.0 / (HAND_FOV_DEGREES / 2.0).to_radians().tan();
    let aspect = size[0] / size[1].max(1.0);
    let ndc = [focal / aspect * view.x / depth, focal * view.y / depth];
    Some([
        (ndc[0] + 1.0) / 2.0 * size[0],
        (1.0 - ndc[1]) / 2.0 * size[1],
    ])
}

pub(crate) fn frame_point(pose: Mat4, [s, t]: [f32; 2], size: [f32; 2]) -> Option<[f32; 2]> {
    let unit = |along: f32| along * FRAME_UNITS - BORDER;
    to_screen(
        pose.transform_point3(Vec3::new(unit(s), unit(t), 0.0)),
        size,
    )
}

pub(crate) fn map_quads(pose: Mat4, size: [f32; 2], material: &str) -> Vec<Draw2dQuad> {
    let step = 1.0 / CELLS as f32;
    let mut quads = Vec::with_capacity(CELLS * CELLS);
    for row in 0..CELLS {
        for col in 0..CELLS {
            let (s0, t0) = (col as f32 * step, row as f32 * step);
            let (s1, t1) = (s0 + step, t0 + step);
            let st = [[s0, t0], [s1, t0], [s1, t1], [s0, t1]];
            let [Some(a), Some(b), Some(c), Some(d)] = st.map(|at| frame_point(pose, at, size))
            else {
                continue;
            };
            quads.push(Draw2dQuad {
                xy: [a, b, c, d],
                st,
                color: [1.0; 4],
                material: material.to_owned(),
                material_namespace: HUD_CHROME_NAMESPACE,
                provenance: Draw2dProvenance::CgDraw {
                    site: "minecraft_held_map",
                },
                layer: 0,
                clip: None,
                glint: None,
            });
        }
    }
    quads
}

pub(crate) fn update(
    surface: Res<crate::surface::Hud2dSurface>,
    ui: Option<Res<MinecraftUi>>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
) {
    pass.minecraft_held_map = TessJob::Hide;
    let Some(ui) = ui.filter(|ui| ui.active) else {
        return;
    };
    if !surface.is_ready() || ui.held_maps.is_empty() {
        return;
    }
    let size = [surface.width(), surface.height()];
    let mut quads = Vec::new();
    for (held, material) in ui.held_maps.iter().zip(MATERIALS) {
        hud_images.insert_runtime(material, held.picture.clone());
        let pose = pose(held.hands, held.pitch, held.swing);
        quads.extend(map_quads(pose, size, material));
    }
    pass.minecraft_held_map = TessJob::Quads(quads);
}

pub(crate) fn flush(
    surface: Res<crate::surface::Hud2dSurface>,
    mut pass: ResMut<HudTessPass>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut frame: ResMut<HudTessGpuFrame>,
    mut host: Query<(&mut Node, &mut crate::gpu_list::GpuListLatch), With<MinecraftHeldMapRaster>>,
) {
    let _body = TessBody::open();
    if !surface.is_ready() {
        return;
    }
    let job = std::mem::take(&mut pass.minecraft_held_map);
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

    const SCREEN: [f32; 2] = [1920.0, 1080.0];

    fn corner(hands: McMapHands, pitch: f32, swing: f32, at: [f32; 2]) -> [f32; 2] {
        frame_point(pose(hands, pitch, swing), at, SCREEN).expect("in front of the eye")
    }

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 0.5 && (a[1] - b[1]).abs() < 0.5
    }

    #[test]
    fn the_map_tilts_up_as_the_holder_looks_down() {
        assert_eq!(tilt(0.0), 1.0);
        assert_eq!(tilt(-90.0), 1.0);
        assert!((tilt(45.0) - 0.024_472).abs() < 1.0e-5, "{}", tilt(45.0));
        assert_eq!(tilt(49.5), 0.0);
        assert_eq!(tilt(90.0), 0.0);
    }

    #[test]
    fn looking_down_the_two_handed_map_faces_the_eye_upright_and_centred() {
        let top_left = corner(McMapHands::Both, 90.0, 0.0, [0.0, 0.0]);
        let bottom_right = corner(McMapHands::Both, 90.0, 0.0, [1.0, 1.0]);
        assert!(near(top_left, [508.46, 45.62]), "{top_left:?}");
        assert!(near(bottom_right, [1411.54, 948.70]), "{bottom_right:?}");
        let top_right = corner(McMapHands::Both, 90.0, 0.0, [1.0, 0.0]);
        assert!(
            near(top_right, [bottom_right[0], top_left[1]]),
            "{top_right:?}"
        );
    }

    #[test]
    fn looking_ahead_the_two_handed_map_lies_low_and_leans_away() {
        let [far_left, far_right, near_right] =
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]].map(|at| corner(McMapHands::Both, 0.0, 0.0, at));
        assert!(far_left[1] > SCREEN[1] / 2.0, "{far_left:?}");
        assert!((far_left[0] + far_right[0] - SCREEN[0]).abs() < 0.01);
        assert!(near_right[1] > far_right[1]);
        assert!(near_right[0] - SCREEN[0] / 2.0 > far_right[0] - SCREEN[0] / 2.0);
        let quads = map_quads(pose(McMapHands::Both, 0.0, 0.0), SCREEN, MATERIALS[0]);
        assert_eq!(quads.len(), CELLS * CELLS);
        assert_eq!(
            quads[0].st,
            [[0.0, 0.0], [0.0625, 0.0], [0.0625, 0.0625], [0.0, 0.0625]]
        );
    }

    #[test]
    fn a_one_handed_map_sits_beside_its_hand_and_the_left_mirrors_the_right() {
        for t in [0.0, 1.0] {
            for s in [0.0, 1.0] {
                let right = corner(McMapHands::Right, 30.0, 0.0, [s, t]);
                let left = corner(McMapHands::Left, -30.0, 0.0, [1.0 - s, t]);
                assert!(right[0] > SCREEN[0] / 2.0, "{right:?}");
                assert!(
                    near(left, [SCREEN[0] - right[0], right[1]]),
                    "{left:?} {right:?}"
                );
            }
        }
    }

    #[test]
    fn a_swing_moves_the_map_and_a_finished_swing_puts_it_back() {
        for hands in [McMapHands::Both, McMapHands::Right] {
            let rest = corner(hands, 10.0, 0.0, [0.5, 0.5]);
            assert!(!near(corner(hands, 10.0, 0.5, [0.5, 0.5]), rest));
            assert!(near(corner(hands, 10.0, 1.0, [0.5, 0.5]), rest));
        }
    }

    #[test]
    fn nothing_is_drawn_behind_the_eye() {
        assert_eq!(to_screen(Vec3::new(0.0, 0.0, 0.1), SCREEN), None);
        assert_eq!(
            to_screen(Vec3::new(0.0, 0.0, -1.0), SCREEN),
            Some([960.0, 540.0])
        );
    }
}
