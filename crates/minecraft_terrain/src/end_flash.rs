use glam::DVec3;
use minecraftoss_core::random::LegacyRandom;
use minecraftoss_player::mth::{cos, sin};

const INTERVAL: i64 = 600;
const MAX_OFFSET: i32 = 200;
const MIN_DURATION: i32 = 100;
const MAX_DURATION: i32 = 380;
const DEG: f32 = std::f32::consts::PI / 180.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EndFlash {
    window: i64,
    offset: i32,
    duration: i32,
    x_angle: f32,
    y_angle: f32,
    intensity: f32,
    old_intensity: f32,
}

impl EndFlash {
    pub fn tick(&mut self, clock_ticks: i64) {
        self.draw(clock_ticks);
        self.old_intensity = self.intensity;
        self.intensity = self.intensity_at(clock_ticks);
    }

    fn draw(&mut self, clock_ticks: i64) {
        let window = clock_ticks / INTERVAL;
        if window == self.window {
            return;
        }
        let mut random = LegacyRandom::new(window);
        random.next_f32();
        self.offset = random.next_i32_bound(MAX_OFFSET + 1);
        let longest = MAX_DURATION.min(INTERVAL as i32 - self.offset);
        self.duration = MIN_DURATION + random.next_i32_bound(longest - MIN_DURATION + 1);
        self.x_angle = random.next_f32() * 70.0 + -60.0;
        self.y_angle = random.next_f32() * 360.0 + -180.0;
        self.window = window;
    }

    fn intensity_at(&self, clock_ticks: i64) -> f32 {
        let t = clock_ticks % INTERVAL;
        if t < i64::from(self.offset) || t > i64::from(self.offset + self.duration) {
            return 0.0;
        }
        if self.duration == 0 {
            return 0.0;
        }
        sin(f64::from(
            (t - i64::from(self.offset)) as f32 * std::f32::consts::PI / self.duration as f32,
        ))
    }

    pub fn intensity(&self, partial: f32) -> f32 {
        self.old_intensity + partial * (self.intensity - self.old_intensity)
    }

    pub fn started_this_tick(&self) -> bool {
        self.intensity > 0.0 && self.old_intensity <= 0.0
    }

    pub fn angles(&self) -> (f32, f32) {
        (self.x_angle, self.y_angle)
    }

    pub fn direction(&self) -> DVec3 {
        direction_from_rotation(self.x_angle, self.y_angle)
    }
}

pub fn direction_from_rotation(x_angle: f32, y_angle: f32) -> DVec3 {
    let yaw = f64::from(-y_angle * DEG - std::f32::consts::PI);
    let pitch = f64::from(-x_angle * DEG);
    let (yaw_cos, yaw_sin) = (cos(yaw), sin(yaw));
    let (pitch_cos, pitch_sin) = (-cos(pitch), sin(pitch));
    DVec3::new(
        f64::from(yaw_sin * pitch_cos),
        f64::from(pitch_sin),
        f64::from(yaw_cos * pitch_cos),
    )
}

pub fn light_boost(intensity: f32, world_fog: bool) -> f32 {
    if world_fog {
        intensity / 3.0
    } else {
        intensity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(flash: &mut EndFlash, ticks: std::ops::Range<i64>) -> Vec<(i64, f32, bool)> {
        ticks
            .map(|tick| {
                flash.tick(tick);
                (tick, flash.intensity(1.0), flash.started_this_tick())
            })
            .collect()
    }

    #[test]
    fn window_zero_never_flashes() {
        let mut flash = EndFlash::default();
        for (tick, intensity, started) in run(&mut flash, 0..600) {
            assert_eq!(intensity, 0.0, "tick {tick}");
            assert!(!started, "tick {tick}");
        }
    }

    #[test]
    fn a_window_draws_its_flash_like_java_random() {
        for (window, offset, duration, x_angle, y_angle) in [
            (1, 133, 311, -31.479216, -105.22267),
            (2, 138, 296, -59.709076, -1.1438904),
            (7, 29, 334, -59.325974, -54.608513),
        ] {
            let mut flash = EndFlash::default();
            flash.tick(window * 600 + 5);
            assert_eq!(
                (flash.offset, flash.duration),
                (offset, duration),
                "window {window}"
            );
            assert_eq!(flash.angles(), (x_angle, y_angle), "window {window}");
        }
    }

    #[test]
    fn one_flash_every_600_ticks_rising_and_falling_as_a_sine() {
        let mut flash = EndFlash::default();
        let ticks = run(&mut flash, 0..3000);
        for window in 1..5 {
            let starts: Vec<i64> = ticks
                .iter()
                .filter(|(tick, _, started)| *started && tick / 600 == window)
                .map(|(tick, _, _)| *tick)
                .collect();
            assert_eq!(starts.len(), 1, "window {window}: {starts:?}");
        }
        let mut flash = EndFlash::default();
        flash.tick(600);
        let (offset, duration) = (i64::from(flash.offset), i64::from(flash.duration));
        let at = |tick: i64| {
            let mut probe = flash;
            probe.tick(600 + tick);
            probe.intensity(1.0)
        };
        assert_eq!(at(offset - 1), 0.0);
        assert_eq!(at(offset), 0.0);
        assert!(at(offset + 1) > 0.0);
        let middle = offset + duration / 2;
        let expected = sin(f64::from(
            (duration / 2) as f32 * std::f32::consts::PI / duration as f32,
        ));
        assert_eq!(at(middle), expected);
        assert!(expected > 0.99);
        assert!(at(offset + duration) < 0.02);
        assert_eq!(at(offset + duration + 1), 0.0);
        let mut previous = flash;
        previous.tick(600 + offset + 10);
        let old = previous.intensity(1.0);
        previous.tick(600 + offset + 11);
        let new = previous.intensity(1.0);
        assert_eq!(previous.intensity(0.25), old + 0.25 * (new - old));
    }

    #[test]
    fn a_paused_clock_holds_the_flash() {
        let mut flash = EndFlash::default();
        flash.tick(600 + 200);
        flash.tick(600 + 201);
        let lit = flash.intensity(1.0);
        assert!(lit > 0.0);
        for _ in 0..40 {
            flash.tick(600 + 201);
            assert_eq!(flash.intensity(0.5), lit);
            assert!(!flash.started_this_tick());
        }
    }

    #[test]
    fn the_flash_lights_the_world_a_third_as_much_in_boss_fog() {
        assert_eq!(light_boost(0.75, false), 0.75);
        assert_eq!(light_boost(0.75, true), 0.25);
        assert_eq!(light_boost(0.0, true), 0.0);
    }

    #[test]
    fn the_flash_points_where_direction_from_rotation_points() {
        let up = direction_from_rotation(-90.0, 0.0);
        assert!((up - DVec3::Y).length() < 1e-3, "{up}");
        let south = direction_from_rotation(0.0, 0.0);
        assert!((south - DVec3::Z).length() < 1e-3, "{south}");
        let west = direction_from_rotation(0.0, 90.0);
        assert!((west - DVec3::NEG_X).length() < 1e-3, "{west}");
        let mut flash = EndFlash::default();
        flash.tick(600);
        let (x, y) = (
            f64::from(flash.x_angle).to_radians(),
            f64::from(flash.y_angle).to_radians(),
        );
        let expected = DVec3::new(-y.sin() * x.cos(), -x.sin(), y.cos() * x.cos());
        assert!((flash.direction() - expected).length() < 1e-3);
    }
}
