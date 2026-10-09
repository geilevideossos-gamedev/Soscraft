use std::collections::HashMap;

use bevy::prelude::*;
use minecraft_terrain::body_gear::BodyGear;
use minecraft_terrain::body_render::{BodyRig, append_body_gear};
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{Atlas, ChunkMesh, ItemVisuals};
use minecraft_terrain::pack::PackStack;

pub const HEAD: usize = 0;
pub const NECK: usize = 1;
pub const SPINE4: usize = 2;
pub const SPINE_UPPER: usize = 3;
pub const SPINE_LOWER: usize = 4;
pub const SHOULDERS: [usize; 2] = [5, 8];
pub const ELBOWS: [usize; 2] = [6, 9];
pub const WRISTS: [usize; 2] = [7, 10];
pub const HIPS: [usize; 2] = [11, 14];
pub const KNEES: [usize; 2] = [12, 15];
pub const ANKLES: [usize; 2] = [13, 16];
pub const HAND_TAGS: [usize; 2] = [17, 18];
pub const JOINTS: usize = 19;

pub const GEAR_BONES: [&str; JOINTS] = [
    "j_head",
    "j_neck",
    "j_spine4",
    "j_spineupper",
    "j_spinelower",
    "j_shoulder_ri",
    "j_elbow_ri",
    "j_wrist_ri",
    "j_shoulder_le",
    "j_elbow_le",
    "j_wrist_le",
    "j_hip_ri",
    "j_knee_ri",
    "j_ankle_ri",
    "j_hip_le",
    "j_knee_le",
    "j_ankle_le",
    "tag_weapon_right",
    "tag_weapon_left",
];

#[derive(Clone, Debug, PartialEq)]
pub struct GearBones {
    pub index: [Option<usize>; JOINTS],
    pub bind: [Mat4; JOINTS],
}

impl Default for GearBones {
    fn default() -> Self {
        Self {
            index: [None; JOINTS],
            bind: [Mat4::IDENTITY; JOINTS],
        }
    }
}

impl GearBones {
    pub fn resolve(dobj: &xmodel_runtime::DObj) -> Self {
        let bind = dobj.compose(&dobj.bind_locals(), Mat4::IDENTITY);
        let index = GEAR_BONES.map(|name| dobj.find(name));
        Self {
            index,
            bind: std::array::from_fn(|joint| {
                index[joint]
                    .and_then(|bone| bind.get(bone).copied())
                    .unwrap_or(Mat4::IDENTITY)
            }),
        }
    }

    pub fn missing(&self) -> Vec<&'static str> {
        GEAR_BONES
            .iter()
            .zip(self.index)
            .filter(|(_, index)| index.is_none())
            .map(|(name, _)| *name)
            .collect()
    }

    pub fn posed(&self, world: &[Mat4], placement: Mat4) -> [Option<PosedJoint>; JOINTS] {
        std::array::from_fn(|joint| {
            let bone = world.get(self.index[joint]?)?;
            Some(PosedJoint {
                posed: placement * *bone,
                bind: self.bind[joint],
            })
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PosedJoint {
    pub posed: Mat4,
    pub bind: Mat4,
}

impl PosedJoint {
    pub fn skin(&self) -> Mat4 {
        self.posed * self.bind.inverse()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PosedGear {
    pub client: u32,
    pub joints: [Option<PosedJoint>; JOINTS],
}

#[derive(Resource, Default)]
pub struct PosedGearFrame {
    bodies: Vec<PosedGear>,
}

impl PosedGearFrame {
    pub fn clear(&mut self) {
        self.bodies.clear();
    }

    pub fn publish(&mut self, body: PosedGear) {
        self.bodies.retain(|old| old.client != body.client);
        self.bodies.push(body);
    }

    pub fn iter(&self) -> impl Iterator<Item = &PosedGear> {
        self.bodies.iter()
    }

    pub fn get(&self, client: u32) -> Option<&PosedGear> {
        self.bodies.iter().find(|body| body.client == client)
    }
}

pub fn publish_posed_gear(
    frame: &mut PosedGearFrame,
    bones: &GearBones,
    world: &[Mat4],
    client: Option<u32>,
    placement: Mat4,
) {
    let Some(client) = client else {
        return;
    };
    frame.publish(PosedGear {
        client,
        joints: bones.posed(world, placement),
    });
}

#[derive(Resource, Default)]
pub struct McBodyGear {
    by_client: HashMap<u32, BodyGear>,
}

impl McBodyGear {
    pub fn set(&mut self, client: u32, gear: BodyGear) {
        self.by_client.insert(client, gear);
    }

    pub fn remove(&mut self, client: u32) {
        self.by_client.remove(&client);
    }

    pub fn clear(&mut self) {
        self.by_client.clear();
    }

    pub fn get(&self, client: u32) -> Option<&BodyGear> {
        self.by_client.get(&client)
    }

    pub fn shown_weapon(&self, client: Option<u32>, corpse: bool, weapon: u32) -> u32 {
        let hidden = !corpse
            && client
                .and_then(|client| self.get(client))
                .is_some_and(|gear| gear.hides_gun);
        if hidden { 0 } else { weapon }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GearFit {
    pub item_scale: f32,
    pub hand_offset: [f32; 3],
}

pub const GEAR_FIT: GearFit = GearFit {
    item_scale: 0.9375,
    hand_offset: [0.0, 1.0, -1.0],
};

pub fn map_to_block(origin: [f64; 3]) -> Mat4 {
    let scale = 1.0 / sim::voxel::BLOCK;
    Mat4::from_cols(
        Vec4::new(scale, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, -scale, 0.0),
        Vec4::new(0.0, scale, 0.0, 0.0),
        Vec4::new(origin[0] as f32, origin[1] as f32, origin[2] as f32, 1.0),
    )
}

fn model_to_hand() -> Mat4 {
    Mat4::from_rotation_z(-std::f32::consts::FRAC_PI_2)
}

pub fn hand_frame(
    joints: &[Option<PosedJoint>; JOINTS],
    side: usize,
    fit: &GearFit,
) -> Option<Mat4> {
    let base = match joints[HAND_TAGS[side]] {
        Some(tag) => tag.posed,
        None => {
            let wrist = joints[WRISTS[side]]?;
            wrist.skin() * Mat4::from_translation(wrist.bind.w_axis.truncate())
        }
    };
    let mirror = if side == 0 { 1.0 } else { -1.0 };
    let [x, y, z] = fit.hand_offset;
    Some(
        base * model_to_hand()
            * Mat4::from_scale(Vec3::splat(sim::voxel::BLOCK * fit.item_scale))
            * Mat4::from_translation(Vec3::new(x * mirror, y, z) / 16.0),
    )
}

fn to_terrain(matrix: Mat4) -> glam::Mat4 {
    glam::Mat4::from_cols_array(&matrix.to_cols_array())
}

pub fn body_rig(joints: &[Option<PosedJoint>; JOINTS], origin: [f64; 3], fit: &GearFit) -> BodyRig {
    let to_block = map_to_block(origin);
    let lit_at = [SPINE4, SPINE_UPPER, SPINE_LOWER, NECK, HEAD]
        .into_iter()
        .chain(0..JOINTS)
        .find_map(|joint| joints[joint])
        .map(|joint| to_block.transform_point3(joint.posed.w_axis.truncate()))
        .unwrap_or(Vec3::new(
            origin[0] as f32,
            origin[1] as f32,
            origin[2] as f32,
        ));
    BodyRig {
        hands: [0, 1]
            .map(|side| hand_frame(joints, side, fit).map(|hand| to_terrain(to_block * hand))),
        light: glam::Vec3::from_array(lit_at.to_array()),
    }
}

pub struct GearRig<'a> {
    pub client: u32,
    pub puppet: bool,
    pub rig: BodyRig,
    pub gear: &'a BodyGear,
}

pub fn gear_rigs<'a>(
    posed: &PosedGearFrame,
    gear: &'a McBodyGear,
    origin: [f64; 3],
    puppet: Option<u32>,
) -> Vec<GearRig<'a>> {
    posed
        .iter()
        .filter_map(|body| {
            let worn = gear.get(body.client).filter(|worn| !worn.is_bare())?;
            Some(GearRig {
                client: body.client,
                puppet: puppet == Some(body.client),
                rig: body_rig(&body.joints, origin, &GEAR_FIT),
                gear: worn,
            })
        })
        .collect()
}

#[derive(Default)]
pub struct GearMeshes {
    pub world: ChunkMesh,
    pub puppet: ChunkMesh,
}

#[allow(clippy::too_many_arguments)]
pub fn gear_meshes(
    posed: &PosedGearFrame,
    gear: &McBodyGear,
    origin: [f64; 3],
    puppet: Option<u32>,
    items: &mut ItemVisuals,
    packs: &PackStack,
    atlas: &Atlas,
    light: &SkyLight,
) -> GearMeshes {
    let mut meshes = GearMeshes::default();
    for body in gear_rigs(posed, gear, origin, puppet) {
        let mesh = if body.puppet {
            &mut meshes.puppet
        } else {
            &mut meshes.world
        };
        let _ = append_body_gear(mesh, items, &body.rig, body.gear, packs, atlas, light);
    }
    meshes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skeleton() -> xmodel_runtime::DObj {
        let joints: [(&str, Vec3); 6] = [
            ("j_mainroot", Vec3::ZERO),
            ("j_spinelower", Vec3::new(0.0, 0.0, 44.0)),
            ("j_spine4", Vec3::new(0.0, 0.0, 14.0)),
            ("j_shoulder_ri", Vec3::new(0.0, -7.0, 0.0)),
            ("j_elbow_ri", Vec3::new(0.0, -11.0, 0.0)),
            ("tag_weapon_right", Vec3::new(10.0, 0.0, 0.0)),
        ];
        let mut at = Vec3::ZERO;
        let base_mat = joints
            .iter()
            .map(|(_, offset)| {
                at += *offset;
                (Quat::IDENTITY, at)
            })
            .collect();
        let model = xmodel_runtime::ModelPoseSrc {
            name: "body".to_owned(),
            num_bones: joints.len(),
            num_root_bones: 1,
            scale: 1.0,
            no_scale_part_bits: [0; 6],
            bone_names: joints.iter().map(|(name, _)| (*name).to_owned()).collect(),
            parent_list: vec![1; joints.len() - 1],
            quats: vec![[0, 0, 0, 32767]; joints.len() - 1],
            trans: joints[1..]
                .iter()
                .map(|(_, offset)| offset.to_array())
                .collect(),
            base_mat,
        };
        xmodel_runtime::DObj::build(&[(&model, None)]).unwrap()
    }

    #[test]
    fn gear_bones_are_found_by_name_with_their_bind_pose() {
        let bones = GearBones::resolve(&skeleton());
        assert_eq!(bones.index[SPINE_LOWER], Some(1));
        assert_eq!(bones.index[ELBOWS[0]], Some(4));
        assert_eq!(bones.index[HAND_TAGS[0]], Some(5));
        assert_eq!(bones.index[HEAD], None);
        assert_eq!(
            bones.bind[ELBOWS[0]].transform_point3(Vec3::ZERO),
            Vec3::new(0.0, -18.0, 58.0)
        );
        assert_eq!(
            bones.bind[HAND_TAGS[0]].transform_point3(Vec3::ZERO),
            Vec3::new(10.0, -18.0, 58.0)
        );
        let missing = bones.missing();
        assert!(missing.contains(&"j_head") && missing.contains(&"tag_weapon_left"));
        assert!(!missing.contains(&"j_elbow_ri"));
    }

    #[test]
    fn posed_joints_are_placed_and_skin_from_the_bind_pose() {
        let dobj = skeleton();
        let bones = GearBones::resolve(&dobj);
        let mut locals = dobj.bind_locals();
        let bend = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        locals[4].rotation = bend.to_array();
        let world = dobj.compose(&locals, Mat4::IDENTITY);
        let placement = Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0));
        let joints = bones.posed(&world, placement);
        assert!(joints[HEAD].is_none());
        let elbow = joints[ELBOWS[0]].unwrap();
        assert!(
            elbow
                .posed
                .transform_point3(Vec3::ZERO)
                .abs_diff_eq(Vec3::new(100.0, -18.0, 58.0), 1e-4)
        );
        let bound_below = Vec3::new(0.0, -18.0, 50.0);
        assert!(
            elbow
                .skin()
                .transform_point3(bound_below)
                .abs_diff_eq(Vec3::new(100.0, -10.0, 58.0), 1e-4)
        );
        let shoulder = joints[SHOULDERS[0]].unwrap();
        assert!(
            shoulder
                .skin()
                .transform_point3(Vec3::new(0.0, -7.0, 58.0))
                .abs_diff_eq(Vec3::new(100.0, -7.0, 58.0), 1e-4)
        );
    }

    #[test]
    fn a_body_publishes_once_per_client_and_without_a_client_not_at_all() {
        let dobj = skeleton();
        let bones = GearBones::resolve(&dobj);
        let world = dobj.compose(&dobj.bind_locals(), Mat4::IDENTITY);
        let mut frame = PosedGearFrame::default();
        publish_posed_gear(&mut frame, &bones, &world, Some(3), Mat4::IDENTITY);
        publish_posed_gear(&mut frame, &bones, &world, Some(3), Mat4::IDENTITY);
        publish_posed_gear(&mut frame, &bones, &world, None, Mat4::IDENTITY);
        assert_eq!(frame.iter().count(), 1);
        assert!(
            frame
                .get(3)
                .is_some_and(|body| body.joints[SPINE4].is_some())
        );
        frame.clear();
        assert!(frame.get(3).is_none());
    }

    #[test]
    fn a_minecraft_item_in_hand_puts_the_mw2_gun_away_but_not_on_a_corpse() {
        let mut gear = McBodyGear::default();
        assert_eq!(gear.shown_weapon(Some(2), false, 7), 7);
        gear.set(
            2,
            BodyGear {
                hides_gun: true,
                ..BodyGear::default()
            },
        );
        gear.set(5, BodyGear::default());
        assert_eq!(gear.shown_weapon(Some(2), false, 7), 0);
        assert_eq!(gear.shown_weapon(Some(2), true, 7), 7);
        assert_eq!(gear.shown_weapon(Some(5), false, 7), 7);
        assert_eq!(gear.shown_weapon(Some(3), false, 7), 7);
        assert_eq!(gear.shown_weapon(None, false, 7), 7);
        gear.remove(2);
        assert_eq!(gear.shown_weapon(Some(2), false, 7), 7);
        gear.set(2, BodyGear::default());
        gear.clear();
        assert!(gear.get(2).is_none() && gear.get(5).is_none());
    }

    fn joint_at(posed: Mat4) -> Option<PosedJoint> {
        Some(PosedJoint {
            posed,
            bind: Mat4::IDENTITY,
        })
    }

    #[test]
    fn a_hand_tag_turns_into_the_vanilla_hand_frame_in_blocks() {
        let mut joints = [None; JOINTS];
        joints[HAND_TAGS[0]] = joint_at(Mat4::from_translation(Vec3::new(36.0, 0.0, 72.0)));
        let fit = GearFit {
            item_scale: 1.0,
            hand_offset: [0.0; 3],
        };
        let rig = body_rig(&joints, [10.0, 64.0, -5.0], &fit);
        let hand = rig.hands[0].unwrap();
        assert!(rig.hands[1].is_none());
        let at = |p: [f32; 3]| hand.transform_point3(glam::Vec3::from_array(p));
        assert!(at([0.0; 3]).abs_diff_eq(glam::Vec3::new(11.0, 66.0, -5.0), 1e-4));
        assert!(
            (at([0.0, 1.0, 0.0]) - at([0.0; 3])).abs_diff_eq(glam::Vec3::X, 1e-5),
            "forward is map +X, block +x"
        );
        assert!(
            (at([0.0, 0.0, 1.0]) - at([0.0; 3])).abs_diff_eq(glam::Vec3::Y, 1e-5),
            "up is map +Z, block +y"
        );
        assert!(
            (at([1.0, 0.0, 0.0]) - at([0.0; 3])).abs_diff_eq(glam::Vec3::Z, 1e-5),
            "right is map -Y, block +z"
        );
        assert!(
            rig.light
                .abs_diff_eq(glam::Vec3::new(11.0, 66.0, -5.0), 1e-4)
        );
    }

    #[test]
    fn the_hand_offset_is_in_pixels_and_mirrored_for_the_left_hand() {
        let mut joints = [None; JOINTS];
        joints[HAND_TAGS[0]] = joint_at(Mat4::IDENTITY);
        joints[HAND_TAGS[1]] = joint_at(Mat4::IDENTITY);
        let fit = GearFit {
            item_scale: 0.5,
            hand_offset: [2.0, 4.0, -8.0],
        };
        let rig = body_rig(&joints, [0.0; 3], &fit);
        let origin = |hand: Option<glam::Mat4>| hand.unwrap().transform_point3(glam::Vec3::ZERO);
        let right = origin(rig.hands[0]);
        let left = origin(rig.hands[1]);
        let pixel = 0.5 / 16.0;
        assert!(
            right.abs_diff_eq(glam::Vec3::new(4.0, -8.0, 2.0) * pixel, 1e-5),
            "{right}"
        );
        assert!(
            left.abs_diff_eq(glam::Vec3::new(4.0, -8.0, -2.0) * pixel, 1e-5),
            "{left}"
        );
    }

    #[test]
    fn without_a_hand_tag_the_hand_rides_the_wrist_upright_from_the_bind_pose() {
        let mut joints = [None; JOINTS];
        let bind = Mat4::from_translation(Vec3::new(0.0, -20.0, 40.0));
        let turn = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        joints[WRISTS[0]] = Some(PosedJoint {
            posed: Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0)) * turn * bind,
            bind,
        });
        let fit = GearFit {
            item_scale: 1.0,
            hand_offset: [0.0; 3],
        };
        let hand = hand_frame(&joints, 0, &fit).unwrap();
        assert!(
            hand.transform_point3(Vec3::ZERO)
                .abs_diff_eq(Vec3::new(25.0, 0.0, 40.0), 1e-4)
        );
        let forward = hand.transform_vector3(Vec3::Y).normalize();
        assert!(forward.abs_diff_eq(Vec3::Y, 1e-5), "{forward}");
    }

    #[test]
    fn only_bodies_with_gear_get_a_rig_and_the_puppet_is_kept_apart() {
        let mut posed = PosedGearFrame::default();
        for client in [1, 2, 3] {
            posed.publish(PosedGear {
                client,
                joints: [None; JOINTS],
            });
        }
        let mut gear = McBodyGear::default();
        let torch = BodyGear {
            off: Some(minecraft_terrain::body_gear::GearItem {
                id: "minecraft:torch".to_owned(),
                glint: false,
                dye: None,
            }),
            ..BodyGear::default()
        };
        gear.set(1, torch.clone());
        gear.set(2, torch);
        gear.set(3, BodyGear::default());
        let rigs = gear_rigs(&posed, &gear, [0.0; 3], Some(2));
        assert_eq!(
            rigs.iter()
                .map(|rig| (rig.client, rig.puppet))
                .collect::<Vec<_>>(),
            [(1, false), (2, true)]
        );
    }
}
