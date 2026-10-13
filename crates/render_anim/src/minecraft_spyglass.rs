use bevy::prelude::*;
use hud_iw4::fov::CG_TANHALF_FOV_Y_SCALE;
use hud_iw4::horizontal_to_vertical_fov_deg;
use minecraft_survival::spyglass::ZOOM_FOV_MODIFIER;
use net::ClientActionInput;
use render_scene::FpvLens;

pub(crate) const SCOPED_LOOK_SCALE: f32 = 1.0 / 8.0;

const TICKS_PER_SECOND: f32 = 20.0;
const MIN_FOV_MODIFIER: f32 = 0.1;
const MAX_FOV_MODIFIER: f32 = 1.5;

pub struct FovModifier(pub(crate) f32);

impl Default for FovModifier {
    fn default() -> Self {
        Self(1.0)
    }
}

pub(crate) fn step_fov_modifier(current: f32, scoping: bool, seconds: f32) -> f32 {
    let target = if scoping { ZOOM_FOV_MODIFIER } else { 1.0 };
    let ticks = (seconds * TICKS_PER_SECOND).max(0.0);
    (target + (current - target) * 0.5f32.powf(ticks)).clamp(MIN_FOV_MODIFIER, MAX_FOV_MODIFIER)
}

pub(crate) fn zoomed_horizontal_fov(horizontal_deg: f32, modifier: f32) -> f32 {
    let vertical = horizontal_to_vertical_fov_deg(horizontal_deg) * modifier;
    let tan_half_x = (vertical.to_radians() * 0.5).tan() / CG_TANHALF_FOV_Y_SCALE;
    (2.0 * tan_half_x.atan()).to_degrees()
}

pub(crate) fn zoom_lens(
    lenses: &mut Query<&mut Projection, With<FpvLens>>,
    horizontal_deg: f32,
    modifier: f32,
    scoping: bool,
    actions: Option<&mut ClientActionInput>,
) -> f32 {
    if scoping && let Some(actions) = actions {
        actions.fov_scale *= SCOPED_LOOK_SCALE;
    }
    if (modifier - 1.0).abs() < 1.0e-4 {
        return horizontal_deg;
    }
    let zoomed = zoomed_horizontal_fov(horizontal_deg, modifier);
    let vertical = horizontal_to_vertical_fov_deg(zoomed).to_radians();
    for mut projection in lenses.iter_mut() {
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.fov = vertical;
        }
    }
    zoomed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fov_modifier_halves_its_distance_to_the_target_each_tick() {
        let tick = 1.0 / TICKS_PER_SECOND;
        let one = step_fov_modifier(1.0, true, tick);
        assert!((one - 0.55).abs() < 1.0e-5);
        let two = step_fov_modifier(one, true, tick);
        assert!((two - 0.325).abs() < 1.0e-5);
        let settled = (0..40).fold(1.0, |m, _| step_fov_modifier(m, true, tick));
        assert!((settled - ZOOM_FOV_MODIFIER).abs() < 1.0e-5);
        let back = step_fov_modifier(settled, false, tick);
        assert!((back - 0.55).abs() < 1.0e-5);
        assert_eq!(step_fov_modifier(1.0, false, 0.016), 1.0);
        assert_eq!(step_fov_modifier(1.0, true, 0.0), 1.0);
    }

    #[test]
    fn a_frame_split_in_two_moves_the_modifier_as_far_as_one_frame() {
        let whole = step_fov_modifier(1.0, true, 0.03);
        let halves = step_fov_modifier(step_fov_modifier(1.0, true, 0.015), true, 0.015);
        assert!((whole - halves).abs() < 1.0e-5);
    }

    #[test]
    fn the_spyglass_scales_the_vertical_fov_by_a_tenth() {
        let base = 65.0;
        let zoomed = zoomed_horizontal_fov(base, ZOOM_FOV_MODIFIER);
        let ratio = horizontal_to_vertical_fov_deg(zoomed) / horizontal_to_vertical_fov_deg(base);
        assert!((ratio - ZOOM_FOV_MODIFIER).abs() < 1.0e-4);
        assert!((zoomed_horizontal_fov(base, 1.0) - base).abs() < 1.0e-3);
    }
}
