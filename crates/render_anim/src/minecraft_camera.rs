use bevy::prelude::*;
use frame::{McAction, McActionInput, McCamera};
use render_scene::WorldCameraPose;

pub(crate) const CAMERA_DISTANCE: f32 = 4.0 * sim::voxel::BLOCK;

pub(crate) fn third_person_pose(
    eye: [f32; 3],
    viewangles: [f32; 3],
    mirrored: bool,
    reach: impl FnOnce([f32; 3], [f32; 3], f32) -> f32,
) -> WorldCameraPose {
    let angles = if mirrored {
        [-viewangles[0], viewangles[1] + 180.0, 0.0]
    } else {
        [viewangles[0], viewangles[1], 0.0]
    };
    let (forward, _, _) = math_iw4::angle_vectors(angles);
    let back = forward.map(|c| -c);
    let distance = reach(eye, back, CAMERA_DISTANCE);
    WorldCameraPose {
        origin: std::array::from_fn(|k| eye[k] + back[k] * distance),
        angles,
    }
}

fn next_camera(camera: McCamera, cycle: bool, detachable: bool) -> McCamera {
    let camera_type = if cycle {
        camera.camera_type.cycle()
    } else {
        camera.camera_type
    };
    McCamera {
        camera_type,
        detached: detachable && !camera_type.is_first_person(),
    }
}

pub(crate) fn update_minecraft_camera(
    actions: Option<Res<McActionInput>>,
    ui: Res<frame::MinecraftUi>,
    skate: Option<Res<frame::SkateMode>>,
    presented: Res<net::PresentedSnapshot>,
    local: Res<net::LocalPresentClient>,
    view: Res<frame::ViewSubject>,
    mut camera: ResMut<McCamera>,
) {
    let pressed = actions
        .as_deref()
        .is_some_and(|actions| actions.just_pressed(McAction::Perspective));
    let detachable = ui.active
        && !skate.as_deref().is_some_and(|skate| skate.active)
        && !view.in_killcam()
        && !crate::occupancy::third_person::presented_is_third_person(
            &presented,
            local.0,
            view.in_killcam(),
        );
    let next = next_camera(
        *camera,
        ui.active && !ui.inventory_open && pressed,
        detachable,
    );
    camera.set_if_neq(next);
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<McCamera>().add_systems(
        Update,
        update_minecraft_camera
            .after(frame::PresentedPublished)
            .before(crate::sync_camera_from_presented)
            .in_set(frame::ClientSet::Present),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::McCameraType;

    fn open(_: [f32; 3], _: [f32; 3], distance: f32) -> f32 {
        distance
    }

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-3)
    }

    fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f32>().sqrt()
    }

    #[test]
    fn the_back_view_sits_four_blocks_behind_the_eye() {
        let pose = third_person_pose([0.0; 3], [0.0; 3], false, open);
        assert!(near(pose.origin, [-144.0, 0.0, 0.0]), "{pose:?}");
        assert!(near(pose.angles, [0.0; 3]), "{pose:?}");
    }

    #[test]
    fn the_front_view_turns_round_and_looks_back_at_the_face() {
        let pose = third_person_pose([0.0; 3], [20.0, 0.0, 0.0], true, open);
        assert!(near(pose.angles, [-20.0, 180.0, 0.0]), "{pose:?}");
        let level = third_person_pose([0.0; 3], [0.0; 3], true, open);
        assert!(near(level.origin, [144.0, 0.0, 0.0]), "{level:?}");
    }

    #[test]
    fn looking_down_lifts_the_back_view_above_the_eye() {
        let pose = third_person_pose([0.0; 3], [45.0, 0.0, 0.0], false, open);
        let (forward, _, _) = math_iw4::angle_vectors([45.0, 0.0, 0.0]);
        assert!(near(pose.origin, forward.map(|c| -c * 144.0)), "{pose:?}");
        assert!(pose.origin[2] > 0.0, "{pose:?}");
    }

    #[test]
    fn a_short_reach_pulls_the_camera_in() {
        let eye = [10.0, 20.0, 30.0];
        let mut asked = None;
        let pose = third_person_pose(eye, [10.0, 70.0, 0.0], false, |from, back, full| {
            asked = Some((from, back, full));
            50.0
        });
        assert!((distance(pose.origin, eye) - 50.0).abs() < 1e-3, "{pose:?}");
        let (from, back, full) = asked.expect("reach asked");
        assert_eq!((from, full), (eye, CAMERA_DISTANCE));
        assert!((distance(back, [0.0; 3]) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn the_perspective_key_cycles_and_detaches_only_when_allowed() {
        let first = McCamera::default();
        let back = next_camera(first, true, true);
        assert_eq!(back.camera_type, McCameraType::ThirdPersonBack);
        assert!(back.detached);
        let front = next_camera(back, true, true);
        assert_eq!(front.camera_type, McCameraType::ThirdPersonFront);
        assert!(front.detached);
        assert_eq!(next_camera(front, true, true), first);
        assert_eq!(next_camera(back, false, true), back);
        let held = next_camera(back, false, false);
        assert_eq!(held.camera_type, McCameraType::ThirdPersonBack);
        assert!(!held.detached);
        assert!(next_camera(held, false, true).detached);
    }

    fn camera_app(active: bool, inventory_open: bool) -> App {
        let mut app = App::new();
        app.insert_resource(frame::MinecraftUi {
            active,
            inventory_open,
            ..Default::default()
        })
        .init_resource::<net::PresentedSnapshot>()
        .insert_resource(net::LocalPresentClient(sim::ClientId(0)))
        .init_resource::<frame::ViewSubject>()
        .init_resource::<McCamera>()
        .add_systems(Update, update_minecraft_camera);
        app
    }

    fn press_perspective(app: &mut App) -> McCamera {
        let mut input = McActionInput::default();
        input.press(McAction::Perspective);
        app.insert_resource(input);
        app.update();
        app.insert_resource(McActionInput::default());
        app.update();
        *app.world().resource::<McCamera>()
    }

    #[test]
    fn the_perspective_key_works_in_the_world_but_not_in_a_screen() {
        let mut app = camera_app(true, false);
        let camera = press_perspective(&mut app);
        assert_eq!(camera.camera_type, McCameraType::ThirdPersonBack);
        assert!(camera.detached);
        app.world_mut()
            .resource_mut::<frame::MinecraftUi>()
            .inventory_open = true;
        let camera = press_perspective(&mut app);
        assert_eq!(camera.camera_type, McCameraType::ThirdPersonBack);
        assert!(camera.detached);
    }

    #[test]
    fn the_camera_stays_attached_off_the_minecraft_world() {
        let mut app = camera_app(false, false);
        let camera = press_perspective(&mut app);
        assert_eq!(camera, McCamera::default());
        app.world_mut().resource_mut::<McCamera>().camera_type = McCameraType::ThirdPersonFront;
        app.update();
        assert!(!app.world().resource::<McCamera>().detached);
        app.world_mut().resource_mut::<frame::MinecraftUi>().active = true;
        app.update();
        assert!(app.world().resource::<McCamera>().detached);
    }
}
