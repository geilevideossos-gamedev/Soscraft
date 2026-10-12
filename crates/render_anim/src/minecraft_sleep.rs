use bevy::math::{Mat4, Vec3};
use frame::McSleepCamera;

pub(crate) const SLEEPING_EYE: f64 = 0.2;
pub(crate) const PILLOW_LIFT: f64 = 0.15;
pub(crate) const BODY_SHIFT: f64 = 1.52;

pub(crate) fn foot_yaw((x, z): (i32, i32)) -> f32 {
    (z as f32).atan2(-(x as f32)).to_degrees()
}

pub(crate) fn sleep_camera(origin: [f64; 3], feet: [f64; 3], facing: (i32, i32)) -> McSleepCamera {
    let eye = [feet[0], feet[1] + SLEEPING_EYE, feet[2]];
    let pillow = [eye[0], eye[1] + PILLOW_LIFT, eye[2]];
    McSleepCamera {
        eye: sim::voxel::to_map(origin, eye),
        origin: sim::voxel::to_map(origin, pillow),
        angles: [0.0, foot_yaw(facing), 0.0],
    }
}

pub(crate) fn lying_root(origin: [f64; 3], feet: [f64; 3], facing: (i32, i32)) -> Mat4 {
    let (x, z) = (f64::from(facing.0), f64::from(facing.1));
    let root = [feet[0] - x * BODY_SHIFT, feet[1], feet[2] - z * BODY_SHIFT];
    let up = Vec3::new(x as f32, -z as f32, 0.0);
    let forward = Vec3::Z;
    let left = up.cross(forward);
    Mat4::from_cols(
        forward.extend(0.0),
        left.extend(0.0),
        up.extend(0.0),
        Vec3::from_array(sim::voxel::to_map(origin, root)).extend(1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: [f64; 3] = [100.0, 60.0, -40.0];
    const FACINGS: [(i32, i32); 4] = [(0, 1), (-1, 0), (0, -1), (1, 0)];

    fn block(map: Vec3) -> [f64; 3] {
        sim::voxel::to_block(ORIGIN, map.to_array())
    }

    fn near(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1.0e-3)
    }

    fn bed_feet(head: (i32, i32, i32), height: f64) -> [f64; 3] {
        [
            f64::from(head.0) + 0.5,
            f64::from(head.1) + height + 0.125,
            f64::from(head.2) + 0.5,
        ]
    }

    #[test]
    fn the_sleep_camera_looks_down_the_bed_from_the_pillow() {
        let head = (103, 64, -38);
        for facing in FACINGS {
            let feet = bed_feet(head, 0.5625);
            let camera = sleep_camera(ORIGIN, feet, facing);
            assert!(near(
                block(Vec3::from_array(camera.origin)),
                [103.5, 64.0 + 0.6875 + 0.35, -37.5]
            ));
            assert!(near(
                block(Vec3::from_array(camera.eye)),
                [103.5, 64.0 + 0.6875 + 0.2, -37.5]
            ));
            assert_eq!((camera.angles[0], camera.angles[2]), (0.0, 0.0));
            let yaw = camera.angles[1].to_radians();
            let ahead = Vec3::from_array(camera.origin) + Vec3::new(yaw.cos(), yaw.sin(), 0.0);
            let step = block(ahead);
            let look = [step[0] - 103.5, step[2] + 37.5];
            let foot = [-f64::from(facing.0), -f64::from(facing.1)];
            assert!(
                (look[0] - foot[0] / 36.0).abs() < 1.0e-4
                    && (look[1] - foot[1] / 36.0).abs() < 1.0e-4,
                "{facing:?} looks along {look:?}"
            );
        }
        let minecraft = |mw2: f32| {
            let yaw = mw2.to_radians();
            (-yaw.cos()).atan2(-yaw.sin()).to_degrees()
        };
        assert!((minecraft(foot_yaw((0, 1))).rem_euclid(360.0) - 180.0).abs() < 1.0e-3);
        assert!((minecraft(foot_yaw((-1, 0))).rem_euclid(360.0) - 270.0).abs() < 1.0e-3);
    }

    #[test]
    fn the_lying_body_runs_from_the_pillow_towards_the_foot() {
        let head = (103, 64, -38);
        let centre = [103.5, 64.6875, -37.5];
        for facing in FACINGS {
            let root = lying_root(ORIGIN, bed_feet(head, 0.5625), facing);
            let (x, z) = (f64::from(facing.0), f64::from(facing.1));
            let top = block(root.transform_point3(Vec3::new(0.0, 0.0, 72.0)));
            let feet = block(root.transform_point3(Vec3::ZERO));
            assert!(near(
                feet,
                [centre[0] - x * 1.52, centre[1], centre[2] - z * 1.52]
            ));
            assert!(near(
                top,
                [centre[0] + x * 0.48, centre[1], centre[2] + z * 0.48]
            ));
            let along = (top[0] - feet[0]) * x + (top[2] - feet[2]) * z;
            assert!(
                (along - 2.0).abs() < 1.0e-3,
                "the 72-unit body spans two blocks: {along}"
            );
            let face = block(root.transform_point3(Vec3::new(10.0, 0.0, 0.0)));
            assert!(
                near(face, [feet[0], feet[1] + 10.0 / 36.0, feet[2]]),
                "it faces the sky"
            );
            assert!((root.determinant() - 1.0).abs() < 1.0e-5);
        }
    }

    #[test]
    fn a_straw_bed_lies_lower() {
        let head = (0, 70, 0);
        let bed = lying_root(ORIGIN, bed_feet(head, 0.5625), (1, 0));
        let straw = lying_root(ORIGIN, bed_feet(head, 0.25), (1, 0));
        let drop = block(bed.transform_point3(Vec3::ZERO))[1]
            - block(straw.transform_point3(Vec3::ZERO))[1];
        assert!((drop - 0.3125).abs() < 1.0e-4, "{drop}");
        let bed_eye = sleep_camera(ORIGIN, bed_feet(head, 0.5625), (1, 0));
        let straw_eye = sleep_camera(ORIGIN, bed_feet(head, 0.25), (1, 0));
        assert!((bed_eye.origin[2] - straw_eye.origin[2] - 0.3125 * 36.0).abs() < 1.0e-3);
        assert!(near(
            block(Vec3::from_array(straw_eye.origin)),
            [0.5, 70.0 + 0.375 + 0.35, 0.5]
        ));
    }
}
