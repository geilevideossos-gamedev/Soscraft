//! The held item on the Minecraft map when no gun is selected, placed as
//! MinecraftOSS's viewer places it (26.3 `applyItemArmTransform` with the
//! item's `firstperson_righthand` display), swinging as vanilla swings it.
//! Vertices are in view space: x right, y up, z back.
use glam::{Mat4, Vec3};

/// Ticks of a swing (`LivingEntity.getCurrentSwingDuration`).
pub(crate) const SWING_TICKS: f32 = minecraft_survival::actions::SWING_TICKS;

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

pub(crate) fn eat_transform(remaining: f32, partial: f32, duration: u32, arm: f32) -> Mat4 {
    let duration = duration.max(1) as f32;
    let left = (remaining - partial + 1.0).min(duration);
    let progress = left / duration;
    let bob = if progress < 0.8 { ((left / 4.0 * std::f32::consts::PI).cos() * 0.1).abs() } else { 0.0 };
    let lift = 1.0 - f64::from(progress).powf(27.0) as f32;
    Mat4::from_translation(Vec3::new(lift * 0.6 * arm, bob + lift * -0.5, 0.0))
        * Mat4::from_rotation_y((arm * lift * 90.0).to_radians())
        * Mat4::from_rotation_x((lift * 10.0).to_radians())
        * Mat4::from_rotation_z((arm * lift * 30.0).to_radians())
}

pub(crate) fn trident_charge_transform(used: f32, arm: f32) -> Mat4 {
    let held = used - 1.0;
    let charge = (held / 10.0).min(1.0);
    let shake = if charge > 0.1 { ((held - 0.1) * 1.3).sin() * (charge - 0.1) } else { 0.0 };
    Mat4::from_translation(Vec3::new(arm * -0.5, 0.7, 0.1))
        * Mat4::from_rotation_x((-55.0f32).to_radians())
        * Mat4::from_rotation_y((arm * 35.3).to_radians())
        * Mat4::from_rotation_z((arm * -9.785).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, charge * 0.2))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + charge * 0.2))
        * Mat4::from_rotation_y((arm * -45.0).to_radians())
}

pub(crate) const ACTIVATION_TICKS: u32 = 40;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ItemActivation {
    pub(crate) ticks: u32,
    pub(crate) off: [f32; 2],
}

impl ItemActivation {
    pub(crate) fn activate(&mut self, mut random: impl FnMut() -> f32) {
        self.ticks = ACTIVATION_TICKS;
        let x = random() * 2.0 - 1.0;
        let y = random() * 2.0 - 1.0;
        self.off = [x, y];
    }

    pub(crate) fn tick(&mut self, ticks: u32) {
        self.ticks = self.ticks.saturating_sub(ticks);
    }

    pub(crate) fn pose(&self, partial: f32, aspect: f32) -> Option<Mat4> {
        activation_pose(self.ticks, partial, self.off, aspect)
    }
}

pub(crate) fn activation_pose(ticks: u32, partial: f32, off: [f32; 2], aspect: f32) -> Option<Mat4> {
    if ticks == 0 {
        return None;
    }
    let t = ((ACTIVATION_TICKS - ticks.min(ACTIVATION_TICKS)) as f32 + partial) / ACTIVATION_TICKS as f32;
    let t2 = t * t;
    let t3 = t * t2;
    let curve = 10.25 * t3 * t2 - 24.95 * t2 * t2 + 25.5 * t3 - 13.8 * t2 + 4.0 * t;
    let angle = curve * std::f32::consts::PI;
    let sway = (angle * 2.0).sin().abs();
    let tilt = (6.0 * (t * 8.0).cos()).to_radians();
    Some(
        Mat4::from_translation(Vec3::new(off[0] * 0.3 * aspect * sway, off[1] * 0.3 * sway, -10.0 + 9.0 * angle.sin()))
            * Mat4::from_scale(Vec3::splat(0.8))
            * Mat4::from_rotation_y((900.0 * angle.sin().abs()).to_radians())
            * Mat4::from_rotation_x(tilt)
            * Mat4::from_rotation_z(tilt),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_activation_rises_spins_and_returns_like_vanilla() {
        let off = [0.7, -0.4];
        assert!(activation_pose(0, 0.5, off, 1.5).is_none());
        let at = |ticks: u32, partial: f32| activation_pose(ticks, partial, off, 1.5).unwrap();
        let centre = |pose: Mat4| pose.transform_point3(Vec3::ZERO);
        let start = at(40, 0.0);
        assert!((centre(start) - Vec3::new(0.0, 0.0, -10.0)).length() < 1e-4, "{}", centre(start));
        let tilt = 6.0f32.to_radians();
        let expected = Mat4::from_translation(Vec3::new(0.0, 0.0, -10.0)) * Mat4::from_scale(Vec3::splat(0.8)) * Mat4::from_rotation_x(tilt) * Mat4::from_rotation_z(tilt);
        assert!(start.abs_diff_eq(expected, 1e-4));
        let end = at(1, 1.0);
        assert!((centre(end) - Vec3::new(0.0, 0.0, -10.0)).length() < 1e-3, "{}", centre(end));
        for (ticks, partial, t) in [(30, 0.0, 0.25f32), (20, 0.0, 0.5), (10, 0.0, 0.75), (21, 0.5, 0.4875)] {
            let curve = 10.25 * t.powi(5) - 24.95 * t.powi(4) + 25.5 * t.powi(3) - 13.8 * t * t + 4.0 * t;
            let a = curve * std::f32::consts::PI;
            let sway = (2.0 * a).sin().abs();
            let point = centre(at(ticks, partial));
            let wanted = Vec3::new(0.7 * 0.3 * 1.5 * sway, -0.4 * 0.3 * sway, -10.0 + 9.0 * a.sin());
            assert!((point - wanted).length() < 1e-4, "t {t}: {point} vs {wanted}");
            let spun = at(ticks, partial) * Mat4::from_rotation_z(-(6.0 * (8.0 * t).cos()).to_radians()) * Mat4::from_rotation_x(-(6.0 * (8.0 * t).cos()).to_radians());
            let facing = spun.transform_vector3(Vec3::X) / 0.8;
            let spin = (900.0 * a.sin().abs()).to_radians();
            assert!((facing - Vec3::new(spin.cos(), 0.0, -spin.sin())).length() < 1e-4, "t {t}");
        }
        let half = 0.5f32;
        let curve = 10.25 * half.powi(5) - 24.95 * half.powi(4) + 25.5 * half.powi(3) - 13.8 * half * half + 4.0 * half;
        assert!((curve - 0.4984375).abs() < 1e-5);
        assert!(centre(at(20, 0.0)).z > -1.9);
        let mut activation = ItemActivation::default();
        let mut draws = [0.25f32, 0.75].into_iter();
        activation.activate(|| draws.next().unwrap());
        assert_eq!(activation, ItemActivation { ticks: 40, off: [-0.5, 0.5] });
        activation.tick(39);
        assert!(activation.pose(0.0, 1.0).is_some());
        activation.tick(2);
        assert!(activation.pose(0.0, 1.0).is_none());
    }

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

    #[test]
    fn eating_starts_from_the_idle_pose_before_the_first_tick() {
        assert!(eat_transform(32.0, 0.0, 32, 1.0).abs_diff_eq(Mat4::IDENTITY, 1e-6));
        assert!(eat_transform(32.0, 0.7, 32, -1.0).abs_diff_eq(Mat4::IDENTITY, 1e-6));
    }

    #[test]
    fn eating_holds_the_item_at_the_mouth_and_bobs_every_four_ticks() {
        let middle = eat_transform(16.0, 0.0, 32, 1.0).transform_point3(Vec3::ZERO);
        let bob = (17.0f32 / 4.0 * std::f32::consts::PI).cos().abs() * 0.1;
        assert!((middle - Vec3::new(0.6, -0.5 + bob, 0.0)).length() < 1e-4);
        assert!((bob - 0.0707).abs() < 1e-3);
        let crest = eat_transform(15.0, 0.0, 32, 1.0).transform_point3(Vec3::ZERO);
        assert!((crest.y - (-0.4)).abs() < 1e-4);
        let early = eat_transform(30.0, 0.0, 32, 1.0);
        let lift = 1.0 - (31.0f64 / 32.0).powf(27.0) as f32;
        assert!((early.transform_point3(Vec3::ZERO) - Vec3::new(0.6 * lift, -0.5 * lift, 0.0)).length() < 1e-5);
    }

    #[test]
    fn the_left_arm_mirrors_the_right_while_eating() {
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        for remaining in [31.0, 24.0, 9.0, 1.0] {
            let right = eat_transform(remaining, 0.4, 32, 1.0) * item_pose(Mat4::IDENTITY, 0.0, 0.0, 1.0);
            let left = eat_transform(remaining, 0.4, 32, -1.0) * item_pose(Mat4::IDENTITY, 0.0, 0.0, -1.0);
            assert!(left.abs_diff_eq(flip * right * flip, 1e-5));
        }
    }

    #[test]
    fn a_charging_trident_is_raised_and_pulled_back_over_half_a_second() {
        let start = trident_charge_transform(1.0, 1.0).transform_point3(Vec3::ZERO);
        assert!((start - Vec3::new(-0.5, 0.7, 0.1)).length() < 1e-5, "{start}");
        let unturn = Mat4::from_rotation_y(45.0f32.to_radians());
        let back = |used: f32| (trident_charge_transform(used, 1.0) * unturn).transform_vector3(Vec3::Z).length();
        assert!((back(1.0) - 1.0).abs() < 1e-5);
        assert!((back(11.0) - 1.2).abs() < 1e-5);
        assert!((back(40.0) - 1.2).abs() < 1e-5);
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        for used in [1.0, 4.5, 12.0] {
            let right = trident_charge_transform(used, 1.0);
            let left = trident_charge_transform(used, -1.0);
            assert!(left.abs_diff_eq(flip * right * flip, 1e-5));
        }
    }

    #[test]
    fn dried_kelp_reaches_the_same_pose_in_half_the_ticks() {
        for (kelp, bread) in [(15.0, 30.0), (8.0, 16.0)] {
            let kelp_lift = eat_transform(kelp, 1.0, 16, 1.0).transform_point3(Vec3::ZERO).x;
            let bread_lift = eat_transform(bread, 1.0, 32, 1.0).transform_point3(Vec3::ZERO).x;
            assert!((kelp_lift - bread_lift).abs() < 1e-5);
        }
    }
}
