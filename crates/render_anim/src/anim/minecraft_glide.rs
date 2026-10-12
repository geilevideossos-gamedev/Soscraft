use std::collections::HashMap;

use anim_iw4::Local;
use bevy::prelude::*;
use minecraft_terrain::elytra_render::{ElytraAnim, WingAngles, flying_roll};
use net::{FrameClock, PresentedSnapshot};
use playerstate_iw4::pm_flags::MC_GLIDING;
use playerstate_iw4::{PM_TYPE_DEAD, PlayerState, eflags};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlideInput {
    pub gliding: bool,
    pub crouching: bool,
    pub motion: [f64; 3],
    pub look: [f64; 3],
    pub pitch_deg: f32,
}

pub fn glide_input(ps: &PlayerState) -> GlideInput {
    let per_tick = f64::from(sim::voxel::BLOCK) * 20.0;
    let [pitch, yaw] = [ps.viewangles[0], ps.viewangles[1]].map(|deg| f64::from(deg).to_radians());
    GlideInput {
        gliding: ps.pm_flags & MC_GLIDING != 0 && ps.pm_type < PM_TYPE_DEAD,
        crouching: ps.e_flags & eflags::DUCK != 0,
        motion: ps.velocity.map(|speed| f64::from(speed) / per_tick),
        look: [
            pitch.cos() * yaw.cos(),
            pitch.cos() * yaw.sin(),
            -pitch.sin(),
        ],
        pitch_deg: ps.viewangles[0],
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyGlide {
    pub anim: ElytraAnim,
    pub input: GlideInput,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlidePose {
    pub scale: f32,
    pub roll: Option<f32>,
    pub pitch_deg: f32,
    pub wings: WingAngles,
    pub crouching: bool,
}

impl BodyGlide {
    pub fn advance(&mut self, seconds: f32, input: GlideInput) {
        self.input = input;
        self.anim
            .advance(seconds, input.gliding, input.crouching, input.motion);
    }

    pub fn pose(&self) -> GlidePose {
        let gliding = self.anim.gliding;
        GlidePose {
            scale: self.anim.scale(),
            roll: gliding
                .then(|| flying_roll(self.input.look, self.input.motion))
                .flatten(),
            pitch_deg: self.input.pitch_deg,
            wings: self.anim.wings(),
            crouching: self.input.crouching,
        }
    }
}

#[derive(Resource, Default)]
pub struct McGlide {
    by_client: HashMap<u32, BodyGlide>,
}

impl McGlide {
    pub fn get(&self, client: u32) -> Option<&BodyGlide> {
        self.by_client.get(&client)
    }

    pub fn pose(&self, client: u32) -> Option<GlidePose> {
        self.get(client).map(BodyGlide::pose)
    }

    pub fn step(&mut self, seconds: f32, clients: impl IntoIterator<Item = (u32, GlideInput)>) {
        let mut seen = Vec::new();
        for (client, input) in clients {
            self.by_client
                .entry(client)
                .or_default()
                .advance(seconds, input);
            seen.push(client);
        }
        self.retain(&seen);
    }

    pub fn retain(&mut self, clients: &[u32]) {
        self.by_client.retain(|client, _| clients.contains(client));
    }

    pub fn clear(&mut self) {
        self.by_client.clear();
    }
}

pub fn step_body_glides(
    time: Res<Time>,
    clock: Option<Res<FrameClock>>,
    presented: Res<PresentedSnapshot>,
    mut glide: ResMut<McGlide>,
) {
    let seconds = clock
        .as_ref()
        .map(|clock| clock.frametime_secs())
        .unwrap_or_else(|| time.delta_secs());
    let Some(snapshot) = presented.snapshot() else {
        glide.clear();
        return;
    };
    let clients = snapshot.players.iter().filter_map(|(client, _)| {
        let ps = presented.player(*client)?;
        Some((client.0, glide_input(ps)))
    });
    glide.step(seconds, clients);
}

impl GlidePose {
    pub fn tilts(&self) -> bool {
        self.scale > 0.0 || self.roll.is_some()
    }
}

pub fn body_pose(
    glide: &McGlide,
    puppet: Option<&frame::InventoryPuppet>,
    client: u32,
    corpse: bool,
) -> Option<GlidePose> {
    if corpse {
        return None;
    }
    let pose = glide.pose(client)?;
    match puppet {
        Some(puppet) if puppet.active && puppet.client == client => Some(GlidePose {
            pitch_deg: puppet.pitch,
            roll: None,
            ..pose
        }),
        _ => Some(pose),
    }
}

pub fn glide_root(scale: f32, pitch_deg: f32, roll: Option<f32>) -> Quat {
    Quat::from_rotation_y((scale * (90.0 + pitch_deg)).to_radians())
        * Quat::from_rotation_z(roll.unwrap_or(0.0))
}

pub fn aim_pitch(pose: Option<GlidePose>, view_pitch_deg: f32) -> f32 {
    pose.map_or(view_pitch_deg, |pose| view_pitch_deg * (1.0 - pose.scale))
}

pub fn apply_glide_root(locals: &mut [Local], pose: GlidePose) {
    let Some(root) = locals.first_mut().filter(|_| pose.tilts()) else {
        return;
    };
    let [x, y, z, w] = root.rotation;
    let turned = glide_root(pose.scale, pose.pitch_deg, pose.roll) * Quat::from_xyzw(x, y, z, w);
    root.rotation = turned.normalize().to_array();
}

const HEAD_BACK_DEG: f32 = -45.0;
const NECK: &str = "j_neck";
const LEGS: [[&str; 3]; 2] = [
    ["j_hip_ri", "j_knee_ri", "j_ankle_ri"],
    ["j_hip_le", "j_knee_le", "j_ankle_le"],
];

fn turn_bone(locals: &mut [Local], world: &[Mat4], bone: usize, turn: Quat) {
    let (Some(local), Some(posed)) = (locals.get_mut(bone), world.get(bone)) else {
        return;
    };
    if local.control {
        return;
    }
    let [x, y, z, w] = local.rotation;
    let own = Quat::from_xyzw(x, y, z, w);
    let rotation = posed.to_scale_rotation_translation().1;
    local.rotation = (own * rotation.inverse() * turn * rotation)
        .normalize()
        .to_array();
}

fn straighten(
    dobj: &xmodel_runtime::DObj,
    locals: &mut [Local],
    joints: [usize; 2],
    down: Vec3,
    scale: f32,
) {
    let world = dobj.compose(locals, Mat4::IDENTITY);
    let at = |bone: usize| world.get(bone).map(|posed| posed.w_axis.truncate());
    let (Some(from), Some(to)) = (at(joints[0]), at(joints[1])) else {
        return;
    };
    let Some(along) = (to - from).try_normalize() else {
        return;
    };
    let arc = Quat::from_rotation_arc(along, down);
    turn_bone(locals, &world, joints[0], Quat::IDENTITY.slerp(arc, scale));
}

pub fn apply_glide_limbs(dobj: &xmodel_runtime::DObj, locals: &mut [Local], pose: GlidePose) {
    if pose.scale <= 0.0 || locals.is_empty() {
        return;
    }
    let world = dobj.compose(locals, Mat4::IDENTITY);
    let body = world[0].to_scale_rotation_translation().1;
    if let Some(neck) = dobj.find(NECK) {
        let back = (HEAD_BACK_DEG * pose.scale).to_radians();
        turn_bone(
            locals,
            &world,
            neck,
            Quat::from_axis_angle(body * Vec3::Y, back),
        );
    }
    let down = body * Vec3::NEG_Z;
    for leg in LEGS {
        let [Some(hip), Some(knee), Some(ankle)] = leg.map(|name| dobj.find(name)) else {
            continue;
        };
        straighten(dobj, locals, [hip, knee], down, pose.scale);
        straighten(dobj, locals, [knee, ankle], down, pose.scale);
    }
}

pub fn apply_glide(dobj: &xmodel_runtime::DObj, locals: &mut [Local], pose: GlidePose) {
    apply_glide_limbs(dobj, locals, pose);
    apply_glide_root(locals, pose);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(pm_flags: u32, velocity: [f32; 3]) -> PlayerState {
        PlayerState {
            pm_flags,
            velocity,
            ..PlayerState::ZERO
        }
    }

    #[test]
    fn a_glider_is_read_from_its_pm_flags_and_a_dead_one_is_not() {
        assert!(glide_input(&player(MC_GLIDING, [0.0; 3])).gliding);
        assert!(!glide_input(&player(0, [0.0; 3])).gliding);
        let dead = PlayerState {
            pm_type: PM_TYPE_DEAD,
            ..player(MC_GLIDING, [0.0; 3])
        };
        assert!(!glide_input(&dead).gliding);
        let ducked = PlayerState {
            e_flags: eflags::DUCK,
            ..player(0, [0.0; 3])
        };
        assert!(glide_input(&ducked).crouching);
        assert!(!glide_input(&player(0, [0.0; 3])).crouching);
    }

    #[test]
    fn velocity_becomes_blocks_per_tick() {
        let input = glide_input(&player(MC_GLIDING, [720.0, -360.0, 72.0]));
        assert_eq!(input.motion, [1.0, -0.5, 0.1]);
        let looking = PlayerState {
            viewangles: [30.0, 90.0, 0.0],
            ..PlayerState::ZERO
        };
        let input = glide_input(&looking);
        let want = [0.0, 30f64.to_radians().cos(), -0.5];
        assert!(
            input
                .look
                .iter()
                .zip(want)
                .all(|(got, want)| (got - want).abs() < 1e-6),
            "{:?}",
            input.look
        );
        assert_eq!(input.pitch_deg, 30.0);
    }

    fn gliding(client: u32) -> (u32, GlideInput) {
        (client, glide_input(&player(MC_GLIDING, [720.0, 0.0, 0.0])))
    }

    fn standing(client: u32) -> (u32, GlideInput) {
        (client, glide_input(&player(0, [0.0; 3])))
    }

    #[test]
    fn each_client_keeps_its_own_glide_and_gone_clients_are_dropped() {
        let mut glide = McGlide::default();
        for _ in 0..4 {
            glide.step(0.05, [gliding(0), standing(3)]);
        }
        let flyer = glide.get(0).unwrap();
        let walker = glide.get(3).unwrap();
        assert_eq!(flyer.anim.fall_fly_ticks, 4);
        assert_eq!(walker.anim.fall_fly_ticks, 0);
        assert!(flyer.anim.now.z < walker.anim.now.z);
        assert!(glide.pose(0).unwrap().scale > 0.0);
        assert_eq!(glide.pose(3).unwrap().scale, 0.0);
        glide.step(0.05, [standing(3), standing(5)]);
        assert!(glide.get(0).is_none());
        assert!(glide.get(3).is_some() && glide.get(5).is_some());
        assert_eq!(glide.get(5).unwrap().anim.now.x, WingAngles::REST.x * 0.3);
        glide.clear();
        assert!(glide.pose(3).is_none());
    }

    #[test]
    fn the_wings_spread_over_frames_while_gliding_and_fold_after_landing() {
        let mut glide = McGlide::default();
        for _ in 0..60 {
            glide.step(1.0 / 60.0, [gliding(1)]);
        }
        let pose = glide.pose(1).unwrap();
        assert!(
            (pose.wings.z - WingAngles::SPREAD.z).abs() < 0.01,
            "{pose:?}"
        );
        assert_eq!(pose.scale, 1.0);
        assert_eq!(pose.roll, Some(0.0));
        glide.step(1.0 / 60.0, [standing(1)]);
        let landed = glide.pose(1).unwrap();
        assert_eq!(landed.scale, 0.0);
        assert_eq!(landed.roll, None);
        for _ in 0..60 {
            glide.step(1.0 / 60.0, [standing(1)]);
        }
        let folded = glide.pose(1).unwrap().wings;
        assert!((folded.z - WingAngles::REST.z).abs() < 0.01, "{folded:?}");
        let crouched = (
            1,
            glide_input(&PlayerState {
                e_flags: eflags::DUCK,
                ..PlayerState::ZERO
            }),
        );
        for _ in 0..60 {
            glide.step(1.0 / 60.0, [crouched]);
        }
        let pose = glide.pose(1).unwrap();
        assert!(pose.crouching);
        assert!((pose.wings.x - WingAngles::CROUCH.x).abs() < 0.01);
    }

    fn flying(scale: f32, pitch_deg: f32, roll: Option<f32>) -> GlidePose {
        GlidePose {
            scale,
            roll,
            pitch_deg,
            wings: WingAngles::SPREAD,
            crouching: false,
        }
    }

    #[test]
    fn the_body_lies_along_the_look_at_full_scale() {
        let up = glide_root(1.0, 30.0, None) * Vec3::Z;
        let look = Vec3::new(30f32.to_radians().cos(), 0.0, -30f32.to_radians().sin());
        assert!(up.abs_diff_eq(look, 1e-5), "{up}");
        let level = glide_root(1.0, 0.0, None);
        assert!((level * Vec3::Z).abs_diff_eq(Vec3::X, 1e-5));
        assert!(
            (level * Vec3::X).abs_diff_eq(Vec3::NEG_Z, 1e-5),
            "the chest faces the ground"
        );
        let climb = glide_root(1.0, -40.0, None) * Vec3::Z;
        assert!(climb.z > 0.6 && climb.x > 0.7, "{climb}");
        assert!(glide_root(0.0, 30.0, None).abs_diff_eq(Quat::IDENTITY, 1e-6));
        let half = glide_root(0.5, 30.0, None) * Vec3::Z;
        assert!((half.angle_between(Vec3::Z).to_degrees() - 60.0).abs() < 1e-3);
    }

    #[test]
    fn the_roll_turns_about_the_long_axis() {
        let still = glide_root(1.0, 10.0, None);
        let rolled = glide_root(1.0, 10.0, Some(0.5));
        assert!((rolled * Vec3::Z).abs_diff_eq(still * Vec3::Z, 1e-5));
        let (left, turned) = (still * Vec3::Y, rolled * Vec3::Y);
        assert!((left.angle_between(turned) - 0.5).abs() < 1e-4);
        let upright = glide_root(0.0, 10.0, Some(0.5));
        assert!(
            (upright * Vec3::X).abs_diff_eq(Vec3::new(0.5f32.cos(), 0.5f32.sin(), 0.0), 1e-5),
            "a positive roll turns the chest to the left like vanilla's YP"
        );
    }

    #[test]
    fn the_aim_pitch_fades_as_the_body_tips() {
        assert_eq!(aim_pitch(None, 40.0), 40.0);
        assert_eq!(aim_pitch(Some(flying(0.0, 40.0, None)), 40.0), 40.0);
        assert_eq!(aim_pitch(Some(flying(0.25, 40.0, None)), 40.0), 30.0);
        assert_eq!(aim_pitch(Some(flying(1.0, 40.0, None)), 40.0), 0.0);
    }

    #[test]
    fn the_puppet_tilts_by_its_own_pitch_without_a_roll_and_corpses_never_tilt() {
        let mut glide = McGlide::default();
        let input = GlideInput {
            gliding: true,
            motion: [1.0, 0.5, 0.0],
            look: [1.0, 0.0, 0.0],
            pitch_deg: 20.0,
            ..Default::default()
        };
        glide.step(0.05, [(4, input)]);
        let world = body_pose(&glide, None, 4, false).unwrap();
        assert_eq!(world.pitch_deg, 20.0);
        assert!(world.roll.is_some_and(|roll| roll > 0.4));
        let puppet = frame::InventoryPuppet {
            active: true,
            client: 4,
            root: Mat4::IDENTITY,
            pitch: -5.0,
        };
        let boxed = body_pose(&glide, Some(&puppet), 4, false).unwrap();
        assert_eq!((boxed.pitch_deg, boxed.roll), (-5.0, None));
        assert_eq!(boxed.scale, world.scale);
        let closed = frame::InventoryPuppet {
            active: false,
            ..puppet
        };
        assert_eq!(body_pose(&glide, Some(&closed), 4, false), Some(world));
        assert_eq!(body_pose(&glide, None, 4, true), None);
        assert_eq!(body_pose(&glide, None, 7, false), None);
    }

    fn skeleton(joints: &[(&str, usize, Vec3)]) -> xmodel_runtime::DObj {
        let mut world: Vec<Vec3> = Vec::new();
        for (index, (_, parent, offset)) in joints.iter().enumerate() {
            let base = if index == 0 {
                Vec3::ZERO
            } else {
                world[*parent]
            };
            world.push(base + *offset);
        }
        let model = xmodel_runtime::ModelPoseSrc {
            name: "body".to_owned(),
            num_bones: joints.len(),
            num_root_bones: 1,
            scale: 1.0,
            no_scale_part_bits: [0; 6],
            bone_names: joints
                .iter()
                .map(|(name, _, _)| (*name).to_owned())
                .collect(),
            parent_list: joints[1..]
                .iter()
                .enumerate()
                .map(|(at, (_, parent, _))| (at + 1 - parent) as _)
                .collect(),
            quats: vec![[0, 0, 0, 32767]; joints.len() - 1],
            trans: joints[1..]
                .iter()
                .map(|(_, _, offset)| offset.to_array())
                .collect(),
            base_mat: world.iter().map(|at| (Quat::IDENTITY, *at)).collect(),
        };
        xmodel_runtime::DObj::build(&[(&model, None)]).unwrap()
    }

    fn body() -> xmodel_runtime::DObj {
        skeleton(&[
            ("tag_origin", 0, Vec3::ZERO),
            ("j_mainroot", 0, Vec3::new(0.0, 0.0, 40.0)),
            ("j_spinelower", 1, Vec3::new(0.0, 0.0, 4.0)),
            ("j_spine4", 2, Vec3::new(0.0, 0.0, 16.0)),
            ("j_neck", 3, Vec3::new(0.0, 0.0, 6.0)),
            ("j_head", 4, Vec3::new(1.0, 0.0, 5.0)),
            ("j_hip_ri", 1, Vec3::new(0.0, -4.0, -2.0)),
            ("j_knee_ri", 6, Vec3::new(4.0, 0.0, -17.0)),
            ("j_ankle_ri", 7, Vec3::new(-5.0, 0.0, -16.0)),
            ("j_hip_le", 1, Vec3::new(0.0, 4.0, -2.0)),
            ("j_knee_le", 9, Vec3::new(3.0, 1.0, -17.0)),
            ("j_ankle_le", 10, Vec3::new(-4.0, 0.0, -16.0)),
        ])
    }

    fn at(world: &[Mat4], dobj: &xmodel_runtime::DObj, name: &str) -> Vec3 {
        world[dobj.find(name).unwrap()].w_axis.truncate()
    }

    #[test]
    fn the_root_turns_every_bone_about_the_feet() {
        let dobj = body();
        let mut locals = dobj.bind_locals();
        apply_glide(&dobj, &mut locals, flying(1.0, 0.0, None));
        let world = dobj.compose(&locals, Mat4::IDENTITY);
        assert!(at(&world, &dobj, "tag_origin").abs_diff_eq(Vec3::ZERO, 1e-4));
        let spine = at(&world, &dobj, "j_spine4");
        assert!(
            spine.abs_diff_eq(Vec3::new(60.0, 0.0, 0.0), 1e-3),
            "{spine}"
        );
        let mut still = dobj.bind_locals();
        apply_glide(&dobj, &mut still, flying(0.0, 30.0, None));
        assert_eq!(still, dobj.bind_locals());
        let mut half = dobj.bind_locals();
        apply_glide(&dobj, &mut half, flying(0.5, 0.0, None));
        let world = dobj.compose(&half, Mat4::IDENTITY);
        let spine = at(&world, &dobj, "j_spine4");
        let want = Vec3::new(45f32.to_radians().sin(), 0.0, 45f32.to_radians().cos()) * 60.0;
        assert!(spine.abs_diff_eq(want, 1e-3), "{spine}");
    }

    fn bent() -> (xmodel_runtime::DObj, Vec<Local>) {
        let dobj = body();
        let mut locals = dobj.bind_locals();
        let world = dobj.compose(&locals, Mat4::IDENTITY);
        for (name, turn) in [
            ("j_hip_ri", Quat::from_rotation_y(0.6)),
            ("j_knee_ri", Quat::from_rotation_y(-1.6)),
            ("j_hip_le", Quat::from_rotation_x(0.3)),
            ("j_knee_le", Quat::from_rotation_y(-0.4)),
            ("j_neck", Quat::from_rotation_y(0.2)),
        ] {
            turn_bone(&mut locals, &world, dobj.find(name).unwrap(), turn);
        }
        (dobj, locals)
    }

    #[test]
    fn the_head_tilts_back_45_degrees_from_the_spine_at_full_scale() {
        let (dobj, before) = bent();
        let mut after = before.clone();
        apply_glide_limbs(&dobj, &mut after, flying(1.0, 0.0, None));
        let (was, now) = (
            dobj.compose(&before, Mat4::IDENTITY),
            dobj.compose(&after, Mat4::IDENTITY),
        );
        let neck = at(&was, &dobj, "j_neck");
        assert!(at(&now, &dobj, "j_neck").abs_diff_eq(neck, 1e-4));
        let face = |world: &[Mat4]| at(world, &dobj, "j_head") - neck;
        let back = Quat::from_rotation_y(-45f32.to_radians()) * face(&was);
        assert!(face(&now).abs_diff_eq(back, 1e-3), "{}", face(&now));
        assert!(
            face(&now).x < face(&was).x,
            "the face turns from the ground toward the flight"
        );
        let spine = at(&now, &dobj, "j_spine4");
        assert!(spine.abs_diff_eq(at(&was, &dobj, "j_spine4"), 1e-4));
        let mut half = before.clone();
        apply_glide_limbs(&dobj, &mut half, flying(0.5, 0.0, None));
        let halfway = dobj.compose(&half, Mat4::IDENTITY);
        let turned = face(&halfway).angle_between(face(&was)).to_degrees();
        assert!((turned - 22.5).abs() < 1e-2, "{turned}");
    }

    #[test]
    fn the_legs_trail_straight_down_the_body_at_full_scale() {
        let (dobj, mut locals) = bent();
        let world = dobj.compose(&locals, Mat4::IDENTITY);
        let along = |world: &[Mat4], a: &str, b: &str| {
            (at(world, &dobj, b) - at(world, &dobj, a)).normalize()
        };
        assert!(along(&world, "j_knee_ri", "j_ankle_ri").dot(Vec3::NEG_Z) < 0.9);
        apply_glide_limbs(&dobj, &mut locals, flying(1.0, 0.0, None));
        let world = dobj.compose(&locals, Mat4::IDENTITY);
        for side in ["ri", "le"] {
            let (hip, knee, ankle) = (
                format!("j_hip_{side}"),
                format!("j_knee_{side}"),
                format!("j_ankle_{side}"),
            );
            assert!(
                along(&world, hip.as_str(), knee.as_str()).abs_diff_eq(Vec3::NEG_Z, 1e-3),
                "{side}"
            );
            assert!(
                along(&world, knee.as_str(), ankle.as_str()).abs_diff_eq(Vec3::NEG_Z, 1e-3),
                "{side}"
            );
            assert!(at(&world, &dobj, &hip).abs_diff_eq(
                at(&dobj.compose(&bent().1, Mat4::IDENTITY), &dobj, &hip),
                1e-4
            ));
        }
        let (_, mut tilted) = bent();
        let pose = flying(1.0, 20.0, None);
        apply_glide(&dobj, &mut tilted, pose);
        let world = dobj.compose(&tilted, Mat4::IDENTITY);
        let tail = -(glide_root(pose.scale, pose.pitch_deg, None) * Vec3::Z);
        assert!(along(&world, "j_knee_le", "j_ankle_le").abs_diff_eq(tail, 1e-3));
    }

    #[test]
    fn nothing_moves_at_scale_zero() {
        let (dobj, before) = bent();
        let mut after = before.clone();
        apply_glide(&dobj, &mut after, flying(0.0, 30.0, None));
        assert_eq!(after, before);
        let rolled = flying(0.0, 30.0, Some(0.4));
        apply_glide_limbs(&dobj, &mut after, rolled);
        assert_eq!(after, before, "a roll alone leaves the limbs");
    }
}
