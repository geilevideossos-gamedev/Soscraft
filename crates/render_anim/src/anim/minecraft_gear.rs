use std::collections::HashMap;

use bevy::prelude::*;
use minecraft_terrain::armor_render::{ArmorRig, SplitPart};
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
        Self::NONE
    }
}

impl GearBones {
    pub const NONE: Self = Self {
        index: [None; JOINTS],
        bind: [Mat4::IDENTITY; JOINTS],
    };

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

    pub fn measures(&self) -> String {
        let at = |joint: usize| self.index[joint].map(|_| self.bind[joint].w_axis.truncate());
        let span = |a: usize, b: usize| match (at(a), at(b)) {
            (Some(a), Some(b)) => format!("{:.1}", a.distance(b)),
            _ => "-".to_owned(),
        };
        let mut parts = vec![
            format!("shoulders {}", span(SHOULDERS[0], SHOULDERS[1])),
            format!("hips {}", span(HIPS[0], HIPS[1])),
            format!("neck to spine {}", span(NECK, SPINE_LOWER)),
            format!("neck to head {}", span(NECK, HEAD)),
        ];
        for (side, name) in ["right", "left"].into_iter().enumerate() {
            parts.push(format!(
                "{name} arm {} + {}",
                span(SHOULDERS[side], ELBOWS[side]),
                span(ELBOWS[side], WRISTS[side])
            ));
            parts.push(format!(
                "{name} leg {} + {}",
                span(HIPS[side], KNEES[side]),
                span(KNEES[side], ANKLES[side])
            ));
            let Some(tag) = self.index[HAND_TAGS[side]].map(|_| self.bind[HAND_TAGS[side]]) else {
                parts.push(format!("{name} hand tag -"));
                continue;
            };
            let place = match at(WRISTS[side]) {
                Some(wrist) => format!("{} from the wrist", vector(tag.w_axis.truncate() - wrist)),
                None => format!("at {}", vector(tag.w_axis.truncate())),
            };
            parts.push(format!(
                "{name} hand tag {place}, X {} Z {}",
                vector(tag.x_axis.truncate().normalize_or_zero()),
                vector(tag.z_axis.truncate().normalize_or_zero())
            ));
        }
        parts.join(", ")
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

fn vector(v: Vec3) -> String {
    format!("({:.1} {:.1} {:.1})", v.x, v.y, v.z)
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
    corpses: HashMap<u32, bool>,
}

impl McBodyGear {
    pub fn set(&mut self, client: u32, gear: BodyGear) {
        self.by_client.insert(client, gear);
    }

    pub fn remove(&mut self, client: u32) {
        self.by_client.remove(&client);
    }

    pub fn died(&mut self, client: u32) {
        if let Some(gear) = self.by_client.remove(&client) {
            self.corpses.insert(client, gear.hides_gun);
        }
    }

    pub fn clear(&mut self) {
        self.by_client.clear();
        self.corpses.clear();
    }

    pub fn follow(&mut self, client: u32, pm_type: Option<i32>, gear: Option<BodyGear>) {
        match pm_type {
            Some(0) => match gear {
                Some(gear) => self.set(client, gear),
                None => self.remove(client),
            },
            Some(
                playerstate_iw4::PM_TYPE_SPECTATOR
                | playerstate_iw4::PM_TYPE_INTERMISSION
                | playerstate_iw4::PM_TYPE_DEAD
                | playerstate_iw4::PM_TYPE_DEAD_LINKED,
            ) => self.died(client),
            _ => self.remove(client),
        }
    }

    pub fn get(&self, client: u32) -> Option<&BodyGear> {
        self.by_client.get(&client)
    }

    pub fn shown_weapon(&self, client: Option<u32>, corpse: bool, weapon: u32) -> u32 {
        let hidden = client.is_some_and(|client| {
            if corpse {
                self.corpses.get(&client) == Some(&true)
            } else {
                self.get(client).is_some_and(|gear| gear.hides_gun)
            }
        });
        if hidden { 0 } else { weapon }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GearFit {
    pub item_scale: f32,
    pub hand_offset: [f32; 3],
    pub head: f32,
    pub head_offset: [f32; 3],
    pub torso_width: f32,
    pub torso_depth: f32,
    pub torso_top: f32,
    pub arm_girth: f32,
    pub leg_girth: f32,
    pub sole: f32,
}

pub const GEAR_FIT: GearFit = GearFit {
    item_scale: 0.9375,
    hand_offset: [0.0, 2.0, -2.0],
    head: 9.0,
    head_offset: [0.5, 0.0, 3.5],
    torso_width: 13.0,
    torso_depth: 8.0,
    torso_top: 3.5,
    arm_girth: 4.0,
    leg_girth: 5.5,
    sole: 4.0,
};

const ARM_LENGTH: f32 = 10.0;
const LEG_LENGTH: f32 = 12.0;
const TORSO_SPLIT: f32 = 6.0;

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

pub fn part_frame(origin: Vec3, down: Vec3, scale: Vec3, anchor: Vec3) -> Option<Mat4> {
    let y = down.try_normalize()?;
    let back = (Vec3::NEG_X - y * y.dot(Vec3::NEG_X)).try_normalize()?;
    let x = y.cross(back);
    Some(
        Mat4::from_cols(
            (x * scale.x).extend(0.0),
            (y * scale.y).extend(0.0),
            (back * scale.z).extend(0.0),
            origin.extend(1.0),
        ) * Mat4::from_translation(-anchor),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundPart {
    pub riders: [PosedJoint; 2],
    pub upper: Mat4,
    pub lower: Mat4,
    pub split: f32,
}

impl BoundPart {
    pub fn posed(&self, to_block: Mat4) -> SplitPart {
        SplitPart {
            upper: to_terrain(to_block * self.riders[0].skin() * self.upper),
            lower: to_terrain(to_block * self.riders[1].skin() * self.lower),
            split: self.split,
        }
    }
}

fn bound_at(joint: Option<PosedJoint>) -> Option<(PosedJoint, Vec3)> {
    let joint = joint?;
    Some((joint, joint.bind.w_axis.truncate()))
}

pub fn limb(
    top: Option<PosedJoint>,
    middle: Option<PosedJoint>,
    end: Vec3,
    girth: f32,
    length: f32,
    centre: f32,
) -> Option<BoundPart> {
    let ((upper_rider, top), (lower_rider, middle)) = (bound_at(top)?, bound_at(middle)?);
    let (upper_length, lower_length) = (top.distance(middle), middle.distance(end));
    let along = (upper_length + lower_length) / length;
    if along <= f32::EPSILON {
        return None;
    }
    let split = upper_length / along;
    let scale = Vec3::new(girth / 4.0, along, girth / 4.0);
    Some(BoundPart {
        riders: [upper_rider, lower_rider],
        upper: part_frame(top, middle - top, scale, Vec3::new(centre, 0.0, 0.0))?,
        lower: part_frame(middle, end - middle, scale, Vec3::new(centre, split, 0.0))?,
        split,
    })
}

pub fn bound_armor(joints: &[Option<PosedJoint>; JOINTS], fit: &GearFit) -> BoundArmor {
    let position = |joint: usize| joints[joint].map(|joint| joint.bind.w_axis.truncate());
    let head = joints[HEAD].and_then(|head| {
        let centre = head.bind.w_axis.truncate() + Vec3::from_array(fit.head_offset);
        let frame = part_frame(
            centre,
            Vec3::NEG_Z,
            Vec3::splat(fit.head / 8.0),
            Vec3::new(0.0, -4.0, 0.0),
        )?;
        Some((head, frame))
    });
    let body = (|| {
        let shoulders = (position(SHOULDERS[0])? + position(SHOULDERS[1])?) * 0.5;
        let hips = (position(HIPS[0])? + position(HIPS[1])?) * 0.5;
        let top = shoulders + Vec3::Z * fit.torso_top;
        let scale = Vec3::new(
            fit.torso_width / 8.0,
            top.distance(hips) / 12.0,
            fit.torso_depth / 4.0,
        );
        let frame = part_frame(top, hips - top, scale, Vec3::ZERO)?;
        Some(BoundPart {
            riders: [
                joints[SPINE4].or(joints[SPINE_UPPER])?,
                joints[SPINE_LOWER].or(joints[SPINE_UPPER])?,
            ],
            upper: frame,
            lower: frame,
            split: TORSO_SPLIT,
        })
    })();
    let arms = [0, 1].map(|side| {
        limb(
            joints[SHOULDERS[side]],
            joints[ELBOWS[side]],
            position(WRISTS[side])?,
            fit.arm_girth,
            ARM_LENGTH,
            if side == 0 { -1.0 } else { 1.0 },
        )
    });
    let legs = [0, 1].map(|side| {
        let (knee, ankle) = (position(KNEES[side])?, position(ANKLES[side])?);
        let sole = ankle + (ankle - knee).normalize_or_zero() * fit.sole;
        limb(
            joints[HIPS[side]],
            joints[KNEES[side]],
            sole,
            fit.leg_girth,
            LEG_LENGTH,
            0.0,
        )
    });
    BoundArmor {
        head,
        body,
        arms,
        legs,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundArmor {
    pub head: Option<(PosedJoint, Mat4)>,
    pub body: Option<BoundPart>,
    pub arms: [Option<BoundPart>; 2],
    pub legs: [Option<BoundPart>; 2],
}

impl BoundArmor {
    pub fn posed(&self, to_block: Mat4) -> ArmorRig {
        ArmorRig {
            head: self
                .head
                .map(|(rider, frame)| to_terrain(to_block * rider.skin() * frame)),
            body: self.body.map(|part| part.posed(to_block)),
            arms: self.arms.map(|part| part.map(|part| part.posed(to_block))),
            legs: self.legs.map(|part| part.map(|part| part.posed(to_block))),
        }
    }
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
        armor: bound_armor(joints, fit).posed(to_block),
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
    fn the_bind_pose_measures_name_the_sizes_the_fit_is_tuned_against() {
        let measures = GearBones::resolve(&skeleton()).measures();
        assert!(measures.starts_with("shoulders -, hips -,"), "{measures}");
        assert!(measures.contains("right arm 11.0 + -"), "{measures}");
        assert!(
            measures
                .contains("right hand tag at (10.0 -18.0 58.0), X (1.0 0.0 0.0) Z (0.0 0.0 1.0)"),
            "{measures}"
        );
        assert!(measures.ends_with("left hand tag -"), "{measures}");
        let mut bones = GearBones::resolve(&skeleton());
        bones.index[WRISTS[0]] = Some(4);
        bones.bind[WRISTS[0]] = Mat4::from_translation(Vec3::new(0.0, -18.0, 58.0));
        assert!(
            bones
                .measures()
                .contains("right hand tag (10.0 0.0 0.0) from the wrist"),
            "{}",
            bones.measures()
        );
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
            ..GEAR_FIT
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
            ..GEAR_FIT
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
            ..GEAR_FIT
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
    fn a_corpse_puts_its_gun_away_when_its_player_died_holding_a_minecraft_item() {
        let mut gear = McBodyGear::default();
        let holding = |hides_gun| BodyGear {
            hides_gun,
            ..BodyGear::default()
        };
        gear.set(2, holding(true));
        gear.set(5, holding(false));
        assert_eq!(gear.shown_weapon(Some(2), true, 7), 7);
        gear.died(2);
        gear.died(5);
        assert!(gear.get(2).is_none());
        assert_eq!(gear.shown_weapon(Some(2), true, 7), 0);
        assert_eq!(gear.shown_weapon(Some(5), true, 7), 7);
        assert_eq!(gear.shown_weapon(Some(2), false, 7), 7);
        gear.died(2);
        assert_eq!(gear.shown_weapon(Some(2), true, 7), 0);
        gear.set(2, holding(false));
        assert_eq!(gear.shown_weapon(Some(2), true, 7), 0);
        gear.died(2);
        assert_eq!(gear.shown_weapon(Some(2), true, 7), 7);
        gear.set(2, holding(true));
        gear.died(2);
        gear.clear();
        assert_eq!(gear.shown_weapon(Some(2), true, 7), 7);
    }

    #[test]
    fn the_corpse_rule_follows_each_client_on_its_own() {
        let mut gear = McBodyGear::default();
        let holding = |hides_gun| {
            Some(BodyGear {
                hides_gun,
                ..BodyGear::default()
            })
        };
        let dead = Some(playerstate_iw4::PM_TYPE_DEAD);
        gear.follow(1, Some(0), holding(true));
        gear.follow(2, Some(0), holding(false));
        gear.follow(1, dead, holding(true));
        gear.follow(2, Some(0), holding(false));
        assert_eq!(gear.shown_weapon(Some(1), true, 7), 0);
        assert_eq!(gear.shown_weapon(Some(2), false, 7), 7);
        assert!(gear.get(1).is_none() && gear.get(2).is_some());
        gear.follow(1, dead, holding(false));
        assert_eq!(
            gear.shown_weapon(Some(1), true, 7),
            0,
            "the gear at the death decides"
        );
        gear.follow(2, dead, holding(false));
        assert_eq!(gear.shown_weapon(Some(2), true, 7), 7);
        assert_eq!(gear.shown_weapon(Some(1), true, 7), 0);
        gear.follow(1, Some(0), holding(false));
        assert_eq!(gear.shown_weapon(Some(1), false, 7), 7);
        gear.follow(3, Some(0), None);
        gear.follow(3, None, holding(true));
        assert!(gear.get(3).is_none(), "no body, no gear");
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

    fn standing() -> [Option<PosedJoint>; JOINTS] {
        let mut joints = [None; JOINTS];
        let at = |x: f32, y: f32, z: f32| {
            Some(PosedJoint {
                posed: Mat4::from_translation(Vec3::new(x, y, z)),
                bind: Mat4::from_translation(Vec3::new(x, y, z)),
            })
        };
        joints[HEAD] = at(0.0, 0.0, 62.0);
        joints[SPINE4] = at(0.0, 0.0, 52.0);
        joints[SPINE_LOWER] = at(0.0, 0.0, 40.0);
        joints[SHOULDERS[0]] = at(0.0, -8.0, 56.0);
        joints[SHOULDERS[1]] = at(0.0, 8.0, 56.0);
        joints[ELBOWS[0]] = at(0.0, -9.0, 45.0);
        joints[ELBOWS[1]] = at(0.0, 9.0, 45.0);
        joints[WRISTS[0]] = at(0.0, -10.0, 35.0);
        joints[WRISTS[1]] = at(0.0, 10.0, 35.0);
        joints[HIPS[0]] = at(0.0, -4.0, 36.0);
        joints[HIPS[1]] = at(0.0, 4.0, 36.0);
        joints[KNEES[0]] = at(1.0, -4.0, 20.0);
        joints[KNEES[1]] = at(1.0, 4.0, 20.0);
        joints[ANKLES[0]] = at(0.0, -4.0, 4.0);
        joints[ANKLES[1]] = at(0.0, 4.0, 4.0);
        joints
    }

    #[test]
    fn a_part_frame_turns_vanilla_left_down_back_into_the_model_axes() {
        let frame = part_frame(
            Vec3::new(0.0, 0.0, 50.0),
            Vec3::NEG_Z,
            Vec3::new(2.0, 3.0, 4.0),
            Vec3::new(1.0, 0.0, 0.0),
        )
        .unwrap();
        assert!(
            frame
                .transform_point3(Vec3::new(1.0, 0.0, 0.0))
                .abs_diff_eq(Vec3::new(0.0, 0.0, 50.0), 1e-5)
        );
        assert!(frame.x_axis.truncate().abs_diff_eq(Vec3::Y * 2.0, 1e-5));
        assert!(frame.y_axis.truncate().abs_diff_eq(Vec3::NEG_Z * 3.0, 1e-5));
        assert!(frame.z_axis.truncate().abs_diff_eq(Vec3::NEG_X * 4.0, 1e-5));
        assert!(part_frame(Vec3::ZERO, Vec3::ZERO, Vec3::ONE, Vec3::ZERO).is_none());
    }

    #[test]
    fn limbs_run_from_joint_to_joint_and_split_at_the_elbow_and_knee() {
        let joints = standing();
        let armor = bound_armor(&joints, &GEAR_FIT);
        let arm = armor.arms[0].unwrap();
        let (shoulder, elbow, wrist) = (
            Vec3::new(0.0, -8.0, 56.0),
            Vec3::new(0.0, -9.0, 45.0),
            Vec3::new(0.0, -10.0, 35.0),
        );
        let along = (shoulder.distance(elbow) + elbow.distance(wrist)) / ARM_LENGTH;
        assert!((arm.split - shoulder.distance(elbow) / along).abs() < 1e-4);
        assert!(
            arm.upper
                .transform_point3(Vec3::new(-1.0, 0.0, 0.0))
                .abs_diff_eq(shoulder, 1e-4)
        );
        assert!(
            arm.upper
                .transform_point3(Vec3::new(-1.0, arm.split, 0.0))
                .abs_diff_eq(elbow, 1e-4),
            "the upper slice ends at the elbow"
        );
        assert!(
            arm.lower
                .transform_point3(Vec3::new(-1.0, arm.split, 0.0))
                .abs_diff_eq(elbow, 1e-4),
            "the lower slice starts there"
        );
        assert!(
            arm.lower
                .transform_point3(Vec3::new(-1.0, ARM_LENGTH, 0.0))
                .abs_diff_eq(wrist, 1e-4)
        );
        assert!((arm.upper.y_axis.truncate().length() - along).abs() < 1e-4);
        assert!((arm.upper.x_axis.truncate().length() - GEAR_FIT.arm_girth / 4.0).abs() < 1e-4);
        let leg = armor.legs[1].unwrap();
        let (knee, ankle) = (Vec3::new(1.0, 4.0, 20.0), Vec3::new(0.0, 4.0, 4.0));
        let sole = ankle + (ankle - knee).normalize() * GEAR_FIT.sole;
        assert!(
            leg.lower
                .transform_point3(Vec3::new(0.0, LEG_LENGTH, 0.0))
                .abs_diff_eq(sole, 1e-3),
            "boots reach the sole"
        );
        let body = armor.body.unwrap();
        assert!(
            body.upper
                .transform_point3(Vec3::new(0.0, 12.0, 0.0))
                .abs_diff_eq(Vec3::new(0.0, 0.0, 36.0), 1e-4)
        );
        assert_eq!(body.riders[0].bind, joints[SPINE4].unwrap().bind);
        assert_eq!(body.riders[1].bind, joints[SPINE_LOWER].unwrap().bind);
    }

    #[test]
    fn a_bent_elbow_turns_only_the_lower_slice() {
        let mut joints = standing();
        let elbow = joints[ELBOWS[0]].unwrap();
        let pivot = elbow.bind.w_axis.truncate();
        let bend = Mat4::from_translation(pivot)
            * Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2)
            * Mat4::from_translation(-pivot);
        joints[ELBOWS[0]] = Some(PosedJoint {
            posed: bend * elbow.bind,
            bind: elbow.bind,
        });
        let rig = body_rig(&joints, [0.0; 3], &GEAR_FIT);
        let straight = body_rig(&standing(), [0.0; 3], &GEAR_FIT);
        let (bent, before) = (rig.armor.arms[0].unwrap(), straight.armor.arms[0].unwrap());
        assert_eq!(bent.upper, before.upper);
        assert_ne!(bent.lower, before.lower);
        let to_block = |p: Vec3| map_to_block([0.0; 3]).transform_point3(p);
        let wrist = bent
            .lower
            .transform_point3(glam::Vec3::new(-1.0, ARM_LENGTH, 0.0));
        let elbow_at = to_block(pivot);
        let reach = Vec3::new(wrist.x, wrist.y, wrist.z) - elbow_at;
        assert!(
            reach.y.abs() < 0.05,
            "the forearm now points forward: {reach}"
        );
        assert_eq!(bent.split, before.split);
        assert_eq!(rig.armor.head, straight.armor.head);
    }

    #[test]
    fn missing_joints_leave_their_part_out() {
        let mut joints = standing();
        joints[HEAD] = None;
        joints[KNEES[1]] = None;
        joints[SPINE4] = None;
        let armor = bound_armor(&joints, &GEAR_FIT);
        assert!(armor.head.is_none());
        assert!(armor.legs[0].is_some() && armor.legs[1].is_none());
        assert!(armor.body.is_none(), "no spine to ride");
        joints[SPINE_UPPER] = joints[SPINE_LOWER];
        assert!(bound_armor(&joints, &GEAR_FIT).body.is_some());
    }
}
