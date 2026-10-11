use glam::DVec3;

pub const PLAYER_VEHICLE_ATTACHMENT: f64 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Riding {
    pub vehicle: u64,
    pub uuid: u128,
}

pub fn bob(walk_position: f32, walk_speed: f32) -> f64 {
    let speed = walk_speed.min(0.25);
    let swing = minecraftoss_player::mth::cos(f64::from(walk_position * 1.5));
    f64::from(0.12 * swing * 2.0 * speed)
}

pub fn seat(previous: DVec3, now: DVec3, partial: f64, attachment: f64, bob: f64) -> DVec3 {
    previous.lerp(now, partial) + DVec3::Y * (attachment - PLAYER_VEHICLE_ATTACHMENT + bob)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seat_is_1_1_above_the_strider_and_bobs_with_its_stride() {
        let feet = DVec3::new(3.5, 100.5, -2.5);
        let attachment = minecraftoss_entities::strider::PASSENGER_ATTACHMENT;
        let still = seat(feet, feet, 0.5, attachment, bob(0.0, 0.0));
        assert!((still - feet - DVec3::Y * 1.1).length() < 1e-9);
        assert!((bob(0.0, 0.25) - 0.06).abs() < 1e-6);
        assert!(
            (bob(0.0, 0.9) - 0.06).abs() < 1e-6,
            "the stride is capped at 0.25"
        );
        let down = std::f32::consts::PI / 1.5;
        assert!((bob(down, 0.25) + 0.06).abs() < 1e-4);
        let up = seat(feet, feet, 0.0, attachment, bob(0.0, 0.25));
        assert!((up.y - feet.y - 1.16).abs() < 1e-6);
        let eyes = still.y + 1.62;
        assert!((eyes - feet.y - 2.72).abs() < 1e-6);
    }

    #[test]
    fn the_seat_lerps_between_the_last_two_server_ticks() {
        let before = DVec3::new(0.0, 100.5, 0.0);
        let after = DVec3::new(0.4, 100.5, -0.2);
        let attachment = minecraftoss_entities::strider::PASSENGER_ATTACHMENT;
        assert_eq!(
            seat(before, after, 0.0, attachment, 0.0),
            seat(before, before, 0.7, attachment, 0.0)
        );
        let quarter = seat(before, after, 0.25, attachment, 0.0);
        assert!((quarter.x - 0.1).abs() < 1e-9 && (quarter.z + 0.05).abs() < 1e-9);
        let end = seat(before, after, 1.0, attachment, 0.0);
        assert!((end - after - DVec3::Y * 1.1).length() < 1e-9);
    }
}
