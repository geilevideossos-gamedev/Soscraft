//! The held item on the Minecraft map when no gun is selected, placed as
//! MinecraftOSS's viewer places it (26.3 `applyItemArmTransform` with the
//! item's `firstperson_righthand` display), swinging as vanilla swings it.
//! Vertices are in view space: x right, y up, z back.
use glam::{Mat4, Vec3};

/// Ticks of a swing (`LivingEntity.getCurrentSwingDuration`).
pub(crate) const SWING_TICKS: f32 = 6.0;

/// The item's view-space pose (`applyItemArmTransform` then its display).
pub(crate) fn item_pose(display: Mat4, swing: f32, inverse_height: f32, arm: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(arm * 0.56, -0.52 - 0.6 * inverse_height, -0.72)) * item_swing_transform(swing, arm) * display
}

fn item_swing_transform(swing: f32, arm: f32) -> Mat4 {
    let root = swing.sqrt();
    let x = -0.4 * (root * std::f32::consts::PI).sin();
    let y = 0.2 * (root * std::f32::consts::TAU).sin();
    let z = -0.2 * (swing * std::f32::consts::PI).sin();
    let y_rotation = (swing * swing * std::f32::consts::PI).sin();
    let xz_rotation = (root * std::f32::consts::PI).sin();
    Mat4::from_translation(Vec3::new(arm * x, y, z))
        * Mat4::from_rotation_y((arm * (45.0 - 20.0 * y_rotation)).to_radians())
        * Mat4::from_rotation_z((arm * -20.0 * xz_rotation).to_radians())
        * Mat4::from_rotation_x((-80.0 * xz_rotation).to_radians())
        * Mat4::from_rotation_y((arm * -45.0f32).to_radians())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_left_arm_mirrors_the_right_through_a_swing() {
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        for swing in [0.0, 0.25, 0.5, 1.0] {
            let right = item_pose(Mat4::IDENTITY, swing, 0.0, 1.0);
            let left = item_pose(Mat4::IDENTITY, swing, 0.0, -1.0);
            assert!(left.abs_diff_eq(flip * right * flip, 1e-5));
        }
        let rest = item_pose(Mat4::IDENTITY, 0.0, 0.0, -1.0).transform_point3(Vec3::ZERO);
        assert!((rest - Vec3::new(-0.56, -0.52, -0.72)).length() < 1e-6);
    }
}
