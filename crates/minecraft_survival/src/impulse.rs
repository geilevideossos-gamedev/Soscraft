use glam::DVec3;

const GRAVITY: f64 = 0.08;
const VERTICAL_DRAG: f64 = 0.98;
const AIR_FRICTION: f64 = 0.91;
const TICKS_PER_SECOND: f64 = 20.0;
const LONGEST_FLIGHT: u32 = 1200;
pub const FALL_GRACE_TICKS: u32 = 40;
pub const RIPTIDE_LIFT: f64 = 1.2;

fn flight(rise: f64) -> (f64, u32) {
    let (mut height, mut speed, mut apex, mut ticks) = (0.0, rise, 0.0_f64, 0);
    while ticks < LONGEST_FLIGHT {
        height += speed;
        ticks += 1;
        apex = apex.max(height);
        speed = (speed - GRAVITY) * VERTICAL_DRAG;
        if height <= 0.0 {
            break;
        }
    }
    (apex, ticks)
}

pub fn launch_velocity(push: DVec3, gravity: f64) -> DVec3 {
    if push.y <= 0.0 || gravity <= 0.0 {
        return push * TICKS_PER_SECOND;
    }
    let (apex, ticks) = flight(push.y);
    let up = (2.0 * gravity * apex).sqrt();
    let airtime = 2.0 * up / gravity;
    let carried = (1.0 - AIR_FRICTION.powi(ticks as i32)) / (1.0 - AIR_FRICTION);
    let across = DVec3::new(push.x, 0.0, push.z) * carried / airtime;
    DVec3::new(across.x, up, across.z)
}

pub fn riptide_velocity(push: DVec3, on_ground: bool, gravity: f64) -> DVec3 {
    let launch = launch_velocity(push, gravity);
    if !on_ground || gravity <= 0.0 || launch.y < 0.0 {
        return launch;
    }
    let up = (launch.y * launch.y + 2.0 * gravity * RIPTIDE_LIFT).sqrt();
    DVec3::new(launch.x, up, launch.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MW2_GRAVITY: f64 = 800.0 / 36.0;

    #[test]
    fn a_push_up_reaches_the_vanilla_height_under_mw2_gravity() {
        let (apex, ticks) = flight(1.0);
        assert!((apex - 5.92).abs() < 0.01, "{apex}");
        assert_eq!(ticks, 25);
        let launch = launch_velocity(DVec3::new(0.0, 1.0, 0.0), MW2_GRAVITY);
        let reached = launch.y * launch.y / (2.0 * MW2_GRAVITY);
        assert!((reached - apex).abs() < 1e-9);
        assert_eq!((launch.x, launch.z), (0.0, 0.0));
    }

    #[test]
    fn a_push_up_and_away_lands_as_far_off_as_in_vanilla() {
        let push = DVec3::new(0.6, 0.8, -0.3);
        let (_, ticks) = flight(push.y);
        let launch = launch_velocity(push, MW2_GRAVITY);
        let airtime = 2.0 * launch.y / MW2_GRAVITY;
        let mut vanilla = DVec3::ZERO;
        let mut speed = DVec3::new(push.x, 0.0, push.z);
        for _ in 0..ticks {
            vanilla += speed;
            speed *= AIR_FRICTION;
        }
        let mw2 = DVec3::new(launch.x, 0.0, launch.z) * airtime;
        assert!((mw2 - vanilla).length() < 1e-9, "{mw2} {vanilla}");
    }

    #[test]
    fn a_riptide_from_the_ground_climbs_its_lift_higher() {
        let push = DVec3::new(0.0, 1.5, 0.0);
        let flying = riptide_velocity(push, false, MW2_GRAVITY);
        assert_eq!(flying, launch_velocity(push, MW2_GRAVITY));
        let grounded = riptide_velocity(push, true, MW2_GRAVITY);
        let height = |v: DVec3| v.y * v.y / (2.0 * MW2_GRAVITY);
        assert!((height(grounded) - height(flying) - RIPTIDE_LIFT).abs() < 1e-9);
        let level = riptide_velocity(DVec3::new(2.0, 0.0, 0.0), true, MW2_GRAVITY);
        assert_eq!(level.x, 40.0);
        assert!((height(level) - RIPTIDE_LIFT).abs() < 1e-9);
        let down = DVec3::new(0.0, -1.0, 0.0);
        assert_eq!(
            riptide_velocity(down, true, MW2_GRAVITY),
            DVec3::new(0.0, -20.0, 0.0)
        );
    }

    #[test]
    fn a_push_along_the_ground_or_down_is_carried_as_it_is() {
        assert_eq!(
            launch_velocity(DVec3::new(0.5, 0.0, -1.0), MW2_GRAVITY),
            DVec3::new(10.0, 0.0, -20.0)
        );
        assert_eq!(
            launch_velocity(DVec3::new(0.0, -0.5, 0.0), MW2_GRAVITY),
            DVec3::new(0.0, -10.0, 0.0)
        );
        assert_eq!(
            launch_velocity(DVec3::new(0.0, 1.0, 0.0), 0.0),
            DVec3::new(0.0, 20.0, 0.0)
        );
    }
}
