use std::collections::HashMap;

use bevy::prelude::*;
use minecraft_terrain::body_gear::BodyGear;

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
}
