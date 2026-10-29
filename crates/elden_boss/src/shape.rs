use glam::DVec3;

use crate::data::Shape;

pub const PLAYER_WIDTH: f64 = 0.6;
pub const EYE_SHARE: f64 = 0.9;
pub const CHEST_SHARE: f64 = 0.6;
pub const RING_CLEAR: f64 = 1.0;

pub fn forward(yaw: f64) -> DVec3 {
    let radians = yaw.to_radians();
    DVec3::new(-radians.sin(), 0.0, radians.cos())
}

pub fn left(yaw: f64) -> DVec3 {
    let radians = yaw.to_radians();
    DVec3::new(radians.cos(), 0.0, radians.sin())
}

pub fn wrap(degrees: f64) -> f64 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

pub fn yaw_to(from: DVec3, to: DVec3) -> Option<f64> {
    let dx = to.x - from.x;
    let dz = to.z - from.z;
    if dx * dx + dz * dz < 1e-12 {
        return None;
    }
    Some((-dx).atan2(dz).to_degrees())
}

pub fn flat_distance(a: DVec3, b: DVec3) -> f64 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    (dx * dx + dz * dz).sqrt()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: DVec3,
    pub yaw: f64,
}

impl Frame {
    pub fn forward(&self) -> DVec3 {
        forward(self.yaw)
    }

    pub fn ahead(&self, distance: f64) -> DVec3 {
        self.origin + self.forward() * distance
    }

    pub fn at(&self, local: DVec3) -> DVec3 {
        self.origin + left(self.yaw) * local.x + DVec3::Y * local.y + self.forward() * local.z
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub min: DVec3,
    pub max: DVec3,
}

impl Body {
    pub fn player(feet: DVec3, eye: f64) -> Body {
        let half = PLAYER_WIDTH / 2.0;
        Body {
            min: feet - DVec3::new(half, 0.0, half),
            max: feet + DVec3::new(half, eye / EYE_SHARE, half),
        }
    }

    pub fn feet(&self) -> DVec3 {
        DVec3::new(
            (self.min.x + self.max.x) / 2.0,
            self.min.y,
            (self.min.z + self.max.z) / 2.0,
        )
    }

    pub fn chest(&self) -> DVec3 {
        self.feet() + DVec3::Y * ((self.max.y - self.min.y) * CHEST_SHARE)
    }

    pub fn flat_near(&self, point: DVec3) -> f64 {
        let dx = (self.min.x - point.x).max(point.x - self.max.x).max(0.0);
        let dz = (self.min.z - point.z).max(point.z - self.max.z).max(0.0);
        (dx * dx + dz * dz).sqrt()
    }

    pub fn flat_far(&self, point: DVec3) -> f64 {
        let dx = (point.x - self.min.x)
            .abs()
            .max((self.max.x - point.x).abs());
        let dz = (point.z - self.min.z)
            .abs()
            .max((self.max.z - point.z).abs());
        (dx * dx + dz * dz).sqrt()
    }

    pub fn spans(&self, low: f64, high: f64) -> bool {
        self.min.y <= high && low <= self.max.y
    }

    fn flat_radius(&self) -> f64 {
        let dx = (self.max.x - self.min.x) / 2.0;
        let dz = (self.max.z - self.min.z) / 2.0;
        (dx * dx + dz * dz).sqrt()
    }
}

pub fn in_arc(frame: &Frame, reach: f64, spread: f64, low: f64, high: f64, body: &Body) -> bool {
    if !body.spans(frame.origin.y + low, frame.origin.y + high) {
        return false;
    }
    let near = body.flat_near(frame.origin);
    if near > reach {
        return false;
    }
    let centre = body.feet();
    let Some(bearing) = yaw_to(frame.origin, centre) else {
        return true;
    };
    if near <= 0.0 {
        return true;
    }
    let distance = flat_distance(frame.origin, centre);
    let width = (body.flat_radius() / distance).min(1.0).asin().to_degrees();
    wrap(bearing - frame.yaw).abs() <= spread / 2.0 + width
}

pub fn in_circle(frame: &Frame, ahead: f64, radius: f64, low: f64, high: f64, body: &Body) -> bool {
    body.spans(frame.origin.y + low, frame.origin.y + high)
        && body.flat_near(frame.ahead(ahead)) <= radius
}

pub fn ring_band(near: f64, far: f64, tick: u16, ticks: u16) -> (f64, f64) {
    let ticks = f64::from(ticks.max(1));
    let width = (far - near) / ticks;
    let tick = f64::from(tick);
    (near + width * tick, near + width * (tick + 1.0))
}

#[allow(clippy::too_many_arguments)]
pub fn under_ring(
    frame: &Frame,
    ahead: f64,
    inner: f64,
    outer: f64,
    low: f64,
    high: f64,
    body: &Body,
    ground: f64,
) -> bool {
    let feet = body.min.y;
    if feet - ground >= RING_CLEAR {
        return false;
    }
    if feet < frame.origin.y + low || feet > frame.origin.y + high {
        return false;
    }
    let centre = frame.ahead(ahead);
    body.flat_near(centre) <= outer && body.flat_far(centre) >= inner
}

pub fn struck(
    shape: &Shape,
    frame: &Frame,
    tick: u16,
    ticks: u16,
    body: &Body,
    ground: f64,
) -> bool {
    match *shape {
        Shape::Arc {
            reach,
            spread,
            low,
            high,
        } => in_arc(frame, reach, spread, low, high, body),
        Shape::Circle {
            ahead,
            radius,
            low,
            high,
        } => in_circle(frame, ahead, radius, low, high, body),
        Shape::Ring {
            ahead,
            near,
            far,
            low,
            high,
        } => {
            let (inner, outer) = ring_band(near, far, tick, ticks);
            under_ring(frame, ahead, inner, outer, low, high, body, ground)
        }
    }
}

pub fn centre(shape: &Shape, frame: &Frame) -> DVec3 {
    match *shape {
        Shape::Arc { .. } => frame.origin,
        Shape::Circle { ahead, .. } | Shape::Ring { ahead, .. } => frame.ahead(ahead),
    }
}

pub fn segment(from: DVec3, to: DVec3, body: &Body) -> Option<f64> {
    let delta = to - from;
    let mut enter = 0.0_f64;
    let mut leave = 1.0_f64;
    for axis in 0..3 {
        let start = from[axis];
        let step = delta[axis];
        let (min, max) = (body.min[axis], body.max[axis]);
        if step.abs() < 1e-12 {
            if start < min || start > max {
                return None;
            }
            continue;
        }
        let a = (min - start) / step;
        let b = (max - start) / step;
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b));
        if enter > leave {
            return None;
        }
    }
    Some(enter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facing_south() -> Frame {
        Frame {
            origin: DVec3::ZERO,
            yaw: 0.0,
        }
    }

    fn standing(x: f64, y: f64, z: f64) -> Body {
        Body::player(DVec3::new(x, y, z), 1.62)
    }

    #[test]
    fn left_is_plus_x_and_forward_plus_z_at_yaw_zero() {
        assert!((forward(0.0) - DVec3::Z).length() < 1e-9);
        assert!((left(0.0) - DVec3::X).length() < 1e-9);
        assert!((forward(90.0) + DVec3::X).length() < 1e-9);
        assert!((yaw_to(DVec3::ZERO, DVec3::new(-3.0, 0.0, 0.0)).unwrap() - 90.0).abs() < 1e-9);
        assert!((wrap(270.0) + 90.0).abs() < 1e-9);
        let hand = facing_south().at(DVec3::new(0.5, 2.2, 0.4));
        assert!((hand - DVec3::new(0.5, 2.2, 0.4)).length() < 1e-9);
    }

    #[test]
    fn a_player_box_is_as_tall_as_the_eye_over_nine_tenths() {
        let body = standing(1.0, 0.0, 2.0);
        assert!((body.max.y - 1.8).abs() < 1e-9);
        assert!((body.max.x - body.min.x - PLAYER_WIDTH).abs() < 1e-9);
        assert!((body.chest().y - 1.08).abs() < 1e-9);
    }

    fn cane(spread: f64, x: f64, y: f64, z: f64) -> bool {
        in_arc(&facing_south(), 3.5, spread, -1.0, 2.5, &standing(x, y, z))
    }

    fn landing(x: f64, z: f64) -> bool {
        in_circle(&facing_south(), 1.0, 1.5, -1.0, 2.5, &standing(x, 0.0, z))
    }

    #[test]
    fn an_arc_reaches_a_player_in_a_two_deep_hole_beside_him_and_not_a_three_deep_one() {
        assert!(cane(100.0, 0.0, -2.0, 1.5));
        assert!(!cane(100.0, 0.0, -3.0, 1.5));
    }

    #[test]
    fn an_arc_misses_behind_him_beyond_its_reach_and_outside_its_width() {
        assert!(cane(100.0, 0.0, 0.0, 3.7));
        assert!(!cane(100.0, 0.0, 0.0, 3.9));
        assert!(!cane(100.0, 0.0, 0.0, -2.0));
        assert!(!cane(60.0, 2.5, 0.0, 1.0));
        assert!(cane(60.0, 0.1, 0.0, 0.1));
        assert!(!cane(100.0, 0.0, 3.0, 2.0));
        let sweep = standing(3.0, 0.0, 0.1);
        assert!(in_arc(&facing_south(), 4.5, 180.0, -1.0, 2.0, &sweep));
    }

    #[test]
    fn a_circle_hits_what_it_touches_in_front_of_him() {
        assert!(landing(0.0, 2.6));
        assert!(landing(1.6, 1.0));
        assert!(!landing(0.0, 3.0));
        assert!(!landing(0.0, -1.0));
    }

    #[test]
    fn a_ring_hits_standing_feet_as_its_edge_passes_and_never_a_jumper() {
        let frame = facing_south();
        let ring = Shape::Ring {
            ahead: 2.0,
            near: 0.5,
            far: 3.5,
            low: -1.0,
            high: 1.0,
        };
        let feet = standing(0.0, 0.0, 5.0);
        let ticks: Vec<u16> = (0..7)
            .filter(|&tick| struck(&ring, &frame, tick, 7, &feet, 0.0))
            .collect();
        assert!(!ticks.is_empty() && ticks.len() <= 3, "{ticks:?}");
        assert!(ticks.iter().all(|&tick| tick >= 4), "{ticks:?}");
        let jumper = standing(0.0, 1.1, 5.0);
        assert!((0..7).all(|tick| !struck(&ring, &frame, tick, 7, &jumper, 0.0)));
        let on_a_step = standing(0.0, 1.0, 5.0);
        assert!((0..7).any(|tick| struck(&ring, &frame, tick, 7, &on_a_step, 1.0)));
        let far = standing(0.0, 0.0, 6.0);
        assert!((0..7).all(|tick| !struck(&ring, &frame, tick, 7, &far, 0.0)));
        assert!((ring_band(0.5, 3.5, 6, 7).1 - 3.5).abs() < 1e-9);
    }

    #[test]
    fn a_segment_hits_a_box_it_crosses_and_not_one_it_passes() {
        let body = standing(0.0, 0.0, 5.0);
        let from = DVec3::new(0.0, 1.0, 0.0);
        let at = segment(from, DVec3::new(0.0, 1.0, 6.0), &body).unwrap();
        assert!((at - 4.7 / 6.0).abs() < 1e-9);
        assert!(segment(from, DVec3::new(0.0, 1.0, 4.0), &body).is_none());
        assert!(segment(from, DVec3::new(1.0, 1.0, 6.0), &body).is_none());
        assert!(segment(from, DVec3::new(0.0, 3.0, 6.0), &body).is_none());
        assert_eq!(
            segment(DVec3::new(0.0, 1.0, 5.0), DVec3::new(0.0, 1.0, 6.0), &body),
            Some(0.0)
        );
    }
}
