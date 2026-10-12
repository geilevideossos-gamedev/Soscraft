use playerstate_iw4::{ENTITYNUM_NONE, PlayerState};

use crate::{CollisionBackend, MoveBounds, Pml, WATER_BLOCK, step_slide_move};

const TICK_SECONDS: f64 = 0.05;
const PER_TICK: f64 = WATER_BLOCK as f64 / TICK_SECONDS;
const DRAG: f64 = 0.980_000_019_073_486_3;
const RISE_KEEP: f64 = 0.8 * DRAG;
const RISE_PUSH: f64 = 0.01 * DRAG;
const SLOW_GRAVITY: f64 = 0.01;
const CLIMB_BOOST: f64 = 0.2;
const LEVEL_BITS: i32 = 0xffff;
pub const SLOW_FALLING: i32 = 0x1_0000;

#[must_use]
pub fn encode(levitation: Option<u8>, slow_falling: bool) -> i32 {
    let level = levitation.map_or(0, |amplifier| i32::from(amplifier) + 1);
    if slow_falling {
        level | SLOW_FALLING
    } else {
        level
    }
}

#[must_use]
pub fn levitation(ps: &PlayerState) -> Option<f64> {
    let level = ps.mc_float & LEVEL_BITS;
    (level > 0).then(|| f64::from(level))
}

#[must_use]
pub fn slow_falling(ps: &PlayerState) -> bool {
    ps.mc_float & SLOW_FALLING != 0
}

#[must_use]
pub fn owns_vertical(ps: &PlayerState) -> bool {
    levitation(ps).is_some() || (slow_falling(ps) && ps.velocity[2] <= 0.0)
}

#[must_use]
pub fn glide_gravity(ps: &PlayerState, vertical: f64, gravity: f64) -> f64 {
    if slow_falling(ps) && vertical <= 0.0 {
        gravity.min(SLOW_GRAVITY)
    } else {
        gravity
    }
}

#[must_use]
pub fn stuck_fall(ps: &PlayerState) -> Option<f32> {
    match levitation(ps) {
        Some(level) => Some((RISE_PUSH * level) as f32),
        None if slow_falling(ps) => Some((-DRAG * SLOW_GRAVITY) as f32),
        None => None,
    }
}

fn settle(v: f64, keep: f64, push: f64, ticks: f64) -> (f64, f64) {
    let target = push / (1.0 - keep);
    let decay = libm::pow(keep, ticks);
    let end = target + (v - target) * decay;
    let moved = ticks * target + (v - target) * (1.0 - decay) / (1.0 - keep);
    (end, moved)
}

#[must_use]
pub fn rise(v: f64, level: f64, ticks: f64) -> f64 {
    settle(v, RISE_KEEP, RISE_PUSH * level, ticks).0
}

#[cfg(test)]
fn rise_path(v: f64, level: f64, ticks: f64) -> f64 {
    settle(v, RISE_KEEP, RISE_PUSH * level, ticks).1
}

#[must_use]
pub fn slow_fall(v: f64, ticks: f64) -> f64 {
    settle(v, DRAG, -DRAG * SLOW_GRAVITY, ticks).0
}

#[cfg(test)]
fn slow_fall_path(v: f64, ticks: f64) -> f64 {
    settle(v, DRAG, -DRAG * SLOW_GRAVITY, ticks).1
}

pub fn leave_ground(ps: &mut PlayerState, pml: &mut Pml) {
    ps.ground_entity_num = ENTITYNUM_NONE;
    pml.walking = 0;
    pml.ground_plane = 0;
    pml.almost_ground_plane = 0;
}

#[must_use]
pub fn climb_rise(ps: &PlayerState, vertical: f32, rising: bool, ticks: f32) -> Option<f32> {
    let level = levitation(ps)?;
    let end = if rising {
        rise(CLIMB_BOOST, level, 1.0)
    } else {
        rise(f64::from(vertical) / PER_TICK, level, f64::from(ticks))
    };
    Some((end * PER_TICK) as f32)
}

pub fn air_slide<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &Pml,
    collision: &C,
    bounds: MoveBounds,
    gravity: f32,
    vertical: f32,
) {
    let ticks = f64::from(pml.frametime) / TICK_SECONDS;
    let v = f64::from(vertical) / PER_TICK;
    let (keep, push, low, high) = match levitation(ps) {
        Some(level) if v >= 0.0 => (RISE_KEEP, RISE_PUSH * level, 0.0, f64::MAX),
        Some(level) => (RISE_KEEP, RISE_PUSH * level, f64::MIN, f64::MAX),
        None if slow_falling(ps) && vertical <= 0.0 => (DRAG, -DRAG * SLOW_GRAVITY, f64::MIN, 0.0),
        None => {
            step_slide_move(
                ps,
                pml,
                collision,
                bounds.mins,
                bounds.maxs,
                bounds.tracemask,
                Some(gravity),
            );
            return;
        }
    };
    if ticks <= 0.0 {
        return;
    }
    let (end, moved) = settle(v, keep, push, ticks);
    let average = (moved.clamp(low, high) * PER_TICK / ticks) as f32;
    ps.velocity[2] = average;
    step_slide_move(
        ps,
        pml,
        collision,
        bounds.mins,
        bounds.maxs,
        bounds.tracemask,
        None,
    );
    if ps.velocity[2] == average {
        ps.velocity[2] = (end * PER_TICK) as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vanilla_rise(level: f64, ticks: usize) -> (f64, f64) {
        let mut v = 0.0;
        let mut path = 0.0;
        for _ in 0..ticks {
            path += v;
            v = (v * 0.8 + 0.01 * level) * DRAG;
        }
        (v, path)
    }

    #[test]
    fn the_closed_form_matches_vanilla_tick_by_tick() {
        for level in [1.0, 5.0] {
            let (v, path) = vanilla_rise(level, 200);
            assert!((rise(0.0, level, 200.0) - v).abs() < 1e-9);
            assert!((rise_path(0.0, level, 200.0) - path).abs() < 1e-9);
        }
        assert!((vanilla_rise(1.0, 200).1 - 8.864).abs() < 0.01);
        assert!((vanilla_rise(5.0, 200).1 - 44.32).abs() < 0.05);
        let steady = rise(0.0, 1.0, 1000.0) * PER_TICK;
        assert!((steady - 32.67).abs() < 0.01, "{steady}");
        let mut v = 0.0;
        let mut path = 0.0;
        for _ in 0..20 {
            path += v;
            v = (v - SLOW_GRAVITY) * DRAG;
        }
        assert!((slow_fall(0.0, 20.0) - v).abs() < 1e-9);
        assert!((slow_fall_path(0.0, 20.0) - path).abs() < 1e-9);
        assert!((path + 1.656).abs() < 0.01, "{path}");
        assert!((slow_fall(0.0, 1000.0) * PER_TICK + 352.8).abs() < 1.0);
    }

    #[test]
    fn short_frames_add_up_to_whole_ticks() {
        let mut v = 0.1;
        let mut path = 0.0;
        for _ in 0..25 {
            path += rise_path(v, 2.0, 0.32);
            v = rise(v, 2.0, 0.32);
        }
        assert!((v - rise(0.1, 2.0, 8.0)).abs() < 1e-12);
        assert!((path - rise_path(0.1, 2.0, 8.0)).abs() < 1e-12);
    }

    #[test]
    fn the_float_field_keeps_the_level_and_slow_falling_apart() {
        let mut ps = PlayerState::ZERO;
        assert_eq!((levitation(&ps), slow_falling(&ps)), (None, false));
        ps.mc_float = encode(Some(0), false);
        assert_eq!((levitation(&ps), slow_falling(&ps)), (Some(1.0), false));
        ps.mc_float = encode(Some(255), true);
        assert_eq!((levitation(&ps), slow_falling(&ps)), (Some(256.0), true));
        ps.mc_float = encode(None, true);
        assert_eq!((levitation(&ps), slow_falling(&ps)), (None, true));
        assert_eq!(glide_gravity(&ps, -0.1, 0.08), 0.01);
        assert_eq!(glide_gravity(&ps, 0.1, 0.08), 0.08);
    }
}
