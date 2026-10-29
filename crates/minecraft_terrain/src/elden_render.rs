use crate::client_mobs::{ClientMobs, MobPose, rot_lerp};
use crate::elden_skin;
use crate::keyframes::{self, Channel, Definition, Interpolation, Keyframe, Moved, Target};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use elden_boss::data::{BossDef, Pose, PoseKey};
use glam::{Mat4, Quat, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::bosses::elden_bosses::EldenBoss;
use std::sync::LazyLock;

pub const SCALE: f32 = 1.4;
pub const TEXTURE_SIZE: [f32; 2] = [64.0, 64.0];
pub const MAX_HEAD_TURN: f32 = 75.0;
pub const TICK: f32 = 0.05;
pub const WALK_SECONDS: f32 = 1.0;
pub const WALK_SPEED: f32 = 1.0;
pub const WALK_AMPLITUDE: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bone {
    Root,
    Body,
    Head,
    RightHorn,
    LeftHorn,
    RightArm,
    Cane,
    LeftArm,
    Hand,
    RightLeg,
    LeftLeg,
    Skirt,
}

pub const BONES: usize = 12;

impl Bone {
    pub const ALL: [Bone; BONES] = [
        Bone::Root,
        Bone::Body,
        Bone::Head,
        Bone::RightHorn,
        Bone::LeftHorn,
        Bone::RightArm,
        Bone::Cane,
        Bone::LeftArm,
        Bone::Hand,
        Bone::RightLeg,
        Bone::LeftLeg,
        Bone::Skirt,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Bone::Root => "root",
            Bone::Body => "body",
            Bone::Head => "head",
            Bone::RightHorn => "right_horn",
            Bone::LeftHorn => "left_horn",
            Bone::RightArm => "right_arm",
            Bone::Cane => "cane",
            Bone::LeftArm => "left_arm",
            Bone::Hand => "hand",
            Bone::RightLeg => "right_leg",
            Bone::LeftLeg => "left_leg",
            Bone::Skirt => "skirt",
        }
    }

    pub fn parent(self) -> Option<Bone> {
        match self {
            Bone::Root => None,
            Bone::Body | Bone::RightLeg | Bone::LeftLeg | Bone::Skirt => Some(Bone::Root),
            Bone::Head | Bone::RightArm | Bone::LeftArm => Some(Bone::Body),
            Bone::RightHorn | Bone::LeftHorn => Some(Bone::Head),
            Bone::Cane => Some(Bone::RightArm),
            Bone::Hand => Some(Bone::LeftArm),
        }
    }

    pub fn pivot(self) -> [f32; 3] {
        match self {
            Bone::Root => [0.0, 0.0, 0.0],
            Bone::Body => [0.0, 12.0, 1.5],
            Bone::Head => [0.0, -13.0, -0.5],
            Bone::RightHorn => [-4.0, -7.0, 1.0],
            Bone::LeftHorn => [4.0, -7.0, 1.0],
            Bone::RightArm => [-6.0, -11.5, 0.0],
            Bone::Cane => [0.0, 11.5, -0.5],
            Bone::LeftArm => [6.0, -11.5, 0.0],
            Bone::Hand => [0.0, 11.5, 0.0],
            Bone::RightLeg => [-2.2, 12.0, 0.5],
            Bone::LeftLeg => [2.2, 12.0, 0.5],
            Bone::Skirt => [0.0, 11.0, 0.0],
        }
    }

    pub fn rest(self) -> [f32; 3] {
        match self {
            Bone::Body => [20.0, 0.0, 0.0],
            Bone::Head => [-20.0, 0.0, 0.0],
            Bone::RightHorn => [-20.0, 0.0, -35.0],
            Bone::LeftHorn => [-20.0, 0.0, 35.0],
            Bone::RightArm => [-30.0, 0.0, 0.0],
            Bone::Cane => [10.0, 0.0, 0.0],
            Bone::LeftArm => [-23.0, 0.0, -6.0],
            Bone::Hand => [-90.0, 0.0, 0.0],
            _ => [0.0, 0.0, 0.0],
        }
    }
}

pub const PARTS: [(Bone, Cube); 14] = [
    (
        Bone::Head,
        Cube::new([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], elden_skin::HEAD),
    ),
    (
        Bone::Head,
        Cube::new([-5.0, -9.0, -3.0], [10.0, 8.0, 8.0], elden_skin::HOOD),
    ),
    (
        Bone::Head,
        Cube::new([-2.5, -3.0, -4.75], [5.0, 6.0, 1.0], elden_skin::BEARD),
    ),
    (
        Bone::RightHorn,
        Cube::new([-1.0, -4.0, -1.0], [2.0, 4.0, 2.0], elden_skin::HORN),
    ),
    (
        Bone::LeftHorn,
        Cube::new([-1.0, -4.0, -1.0], [2.0, 4.0, 2.0], elden_skin::HORN).mirrored(),
    ),
    (
        Bone::Body,
        Cube::new([-4.5, -13.0, -2.5], [9.0, 13.0, 5.0], elden_skin::CLOTH),
    ),
    (
        Bone::RightArm,
        Cube::new([-1.5, -1.5, -1.5], [3.0, 14.0, 3.0], elden_skin::CLOTH),
    ),
    (
        Bone::Cane,
        Cube::new([-0.5, -2.0, -0.5], [1.0, 13.0, 1.0], elden_skin::WOOD),
    ),
    (
        Bone::Cane,
        Cube::new([-0.5, -3.0, -2.5], [1.0, 1.0, 3.0], elden_skin::WOOD),
    ),
    (
        Bone::LeftArm,
        Cube::new([-1.5, -1.5, -1.5], [3.0, 14.0, 3.0], elden_skin::ARM),
    ),
    (
        Bone::RightLeg,
        Cube::new([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], elden_skin::CLOTH),
    ),
    (
        Bone::LeftLeg,
        Cube::new([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], elden_skin::CLOTH).mirrored(),
    ),
    (
        Bone::Skirt,
        Cube::new([-5.0, 0.0, -3.5], [10.0, 5.0, 7.0], elden_skin::CLOTH),
    ),
    (
        Bone::Skirt,
        Cube::new([-8.0, 5.0, -4.5], [16.0, 5.0, 9.0], elden_skin::SKIRT),
    ),
];

pub const EYES: [Cube; 2] = [
    Cube::new([-3.0, -5.0, -4.25], [2.0, 1.0, 0.25], elden_skin::GOLD),
    Cube::new([1.0, -5.0, -4.25], [2.0, 1.0, 0.25], elden_skin::GOLD),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turn {
    pub bone: Bone,
    pub rotation: [f32; 3],
    pub position: [f32; 3],
}

const fn turn(bone: Bone, rotation: [f32; 3]) -> Turn {
    Turn {
        bone,
        rotation,
        position: [0.0; 3],
    }
}

const BREATHE: &[Turn] = &[
    turn(Bone::Body, [3.0, 0.0, 0.0]),
    turn(Bone::Head, [-3.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-2.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [2.0, 0.0, 0.0]),
];

pub fn turns(pose: Pose) -> &'static [Turn] {
    match pose {
        Pose::Breathe => BREATHE,
        _ => &[],
    }
}

fn keys(at: &[(f32, [f32; 3])], vec: fn(f32, f32, f32) -> Vec3) -> Vec<Keyframe> {
    at.iter()
        .map(|&(seconds, [x, y, z])| {
            Keyframe::new(seconds, vec(x, y, z), Interpolation::CatmullRom)
        })
        .collect()
}

pub fn clip(length: u16, poses: &[PoseKey], looping: bool) -> Definition {
    let mut definition = Definition::new(f32::from(length) * TICK, looping);
    for bone in Bone::ALL {
        let of = |key: &PoseKey| {
            turns(key.pose)
                .iter()
                .find(|turn| turn.bone == bone)
                .copied()
        };
        if poses.iter().all(|key| of(key).is_none()) {
            continue;
        }
        let at = |pick: fn(&Turn) -> [f32; 3]| -> Vec<(f32, [f32; 3])> {
            poses
                .iter()
                .map(|key| {
                    let value = of(key).map_or([0.0; 3], |turn| pick(&turn));
                    (f32::from(key.at) * TICK, value)
                })
                .collect()
        };
        let rotation = at(|turn| turn.rotation);
        definition.add(
            bone.name(),
            Channel::new(Target::Rotation, keys(&rotation, keyframes::degree_vec)),
        );
        let position = at(|turn| turn.position);
        if position.iter().any(|(_, value)| *value != [0.0; 3]) {
            definition.add(
                bone.name(),
                Channel::new(Target::Position, keys(&position, keyframes::pos_vec)),
            );
        }
    }
    definition
}

fn walk() -> Definition {
    let mut walk = Definition::new(WALK_SECONDS, true);
    let rotations: [(Bone, [(f32, [f32; 3]); 3]); 5] = [
        (
            Bone::RightLeg,
            [
                (0.0, [-28.0, 0.0, 0.0]),
                (0.5, [24.0, 0.0, 0.0]),
                (1.0, [-28.0, 0.0, 0.0]),
            ],
        ),
        (
            Bone::LeftLeg,
            [
                (0.0, [14.0, 0.0, 0.0]),
                (0.5, [-12.0, 0.0, 0.0]),
                (1.0, [14.0, 0.0, 0.0]),
            ],
        ),
        (
            Bone::Body,
            [
                (0.0, [0.0, 0.0, 4.0]),
                (0.5, [3.0, 0.0, -4.0]),
                (1.0, [0.0, 0.0, 4.0]),
            ],
        ),
        (
            Bone::RightArm,
            [
                (0.0, [-16.0, 0.0, 0.0]),
                (0.5, [6.0, 0.0, 0.0]),
                (1.0, [-16.0, 0.0, 0.0]),
            ],
        ),
        (
            Bone::Skirt,
            [
                (0.0, [5.0, 0.0, 0.0]),
                (0.5, [-5.0, 0.0, 0.0]),
                (1.0, [5.0, 0.0, 0.0]),
            ],
        ),
    ];
    for (bone, at) in rotations {
        walk.add(
            bone.name(),
            Channel::new(Target::Rotation, keys(&at, keyframes::degree_vec)),
        );
    }
    let bob = [
        (0.0, [0.0, 0.0, 0.0]),
        (0.25, [0.0, -0.8, 0.0]),
        (0.5, [0.0, 0.0, 0.0]),
        (0.75, [0.0, -0.4, 0.0]),
        (1.0, [0.0, 0.0, 0.0]),
    ];
    walk.add(
        Bone::Root.name(),
        Channel::new(Target::Position, keys(&bob, keyframes::pos_vec)),
    );
    walk
}

pub struct Animations {
    pub id: &'static str,
    pub idle: Definition,
    pub walk: Definition,
}

impl Animations {
    pub fn of(def: &'static BossDef) -> Animations {
        let clips = &def.clips;
        Animations {
            id: def.id,
            idle: clip(clips.idle.length, clips.idle.poses, true),
            walk: walk(),
        }
    }
}

static ANIMATIONS: LazyLock<Vec<Animations>> = LazyLock::new(|| {
    elden_boss::IDS
        .iter()
        .filter_map(|id| elden_boss::def(id))
        .map(Animations::of)
        .collect()
});

pub fn animations(def: &BossDef) -> Option<&'static Animations> {
    ANIMATIONS.iter().find(|animations| animations.id == def.id)
}

pub fn millis(ticks: f32) -> i64 {
    (ticks * 50.0) as i64
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Look {
    pub moved: Moved,
}

pub fn head_turn(pose: &MobPose, facing: f32) -> f32 {
    wrap(pose.body_rot + pose.head_yaw - facing).clamp(-MAX_HEAD_TURN, MAX_HEAD_TURN)
}

fn wrap(degrees: f32) -> f32 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

pub fn look_of(def: &BossDef, pose: &MobPose, facing: f32) -> Look {
    let mut moved = Moved::default();
    if let Some(animations) = animations(def) {
        animations
            .idle
            .apply(millis(pose.age_in_ticks), 1.0, &mut moved);
        animations.walk.apply_walk(
            pose.walk_position,
            pose.walk_speed,
            WALK_SPEED,
            WALK_AMPLITUDE,
            &mut moved,
        );
    }
    let yaw = head_turn(pose, facing);
    moved.add(
        Bone::Head.name(),
        Target::Rotation,
        keyframes::degree_vec(pose.head_pitch, yaw, 0.0),
    );
    Look { moved }
}

pub fn posed(moved: &Moved) -> [Mat4; BONES] {
    let mut out = [Mat4::IDENTITY; BONES];
    for bone in Bone::ALL {
        let offsets = moved.get(bone.name());
        let pivot = Vec3::from_array(bone.pivot()) + offsets.position;
        let turn = Vec3::from_array(bone.rest()) * keyframes::DEGREES + offsets.rotation;
        let local = part_pose(pivot.to_array(), turn.to_array());
        out[bone as usize] = match bone.parent() {
            Some(parent) => out[parent as usize] * local,
            None => local,
        };
    }
    out
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, look: &Look) {
    let bones = posed(&look.moved);
    for (bone, cube) in &PARTS {
        append(mesh, root * bones[*bone as usize], skin, cube);
    }
    let head = root * bones[Bone::Head as usize];
    for cube in &EYES {
        append(mesh, head, skin, cube);
    }
}

pub fn root_at(feet: Vec3, turn: Quat, facing: f32) -> Mat4 {
    let turn = turn * Quat::from_rotation_y((180.0 - facing).to_radians());
    crate::wither_render::model_root(Mat4::from_translation(feet) * Mat4::from_quat(turn), SCALE)
}

pub fn append_bosses(
    mesh: &mut ChunkMesh,
    mobs: &[FamilyMob<EldenBoss>],
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let Ok(texture) = ResourceId::parse(elden_skin::TEXTURE) else {
        return;
    };
    let region = atlas.entity_region(&texture);
    let mut marks = Vec::new();
    for mob in mobs.iter().filter(|mob| !mob.core.removed) {
        let id = mob.core.id;
        let (Some(client), Some(pose)) = (poses.get(id), poses.pose(id, partial)) else {
            continue;
        };
        let shown = poses.partial_of(id, partial).clamp(0.0, 1.0);
        let facing = rot_lerp(shown, client.y_rot_o, client.y_rot);
        let look = look_of(mob.state.def, &pose, facing);
        marks.push((mesh.vertices.len(), pose.overlay(0.0)));
        let block = pose.light_block();
        let skin = Skin {
            region,
            texture_size: TEXTURE_SIZE,
            sky: f32::from(light.get(block)),
            block: f32::from(light.get_block(block)),
            alpha: 1.0,
            shaded: true,
        };
        let root = root_at(pose.feet.as_vec3(), pose.turn, facing);
        append_model(mesh, root, &skin, &look);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use elden_boss::margit::MARGIT;

    pub(super) fn standing() -> MobPose {
        MobPose {
            feet: glam::DVec3::ZERO,
            body_rot: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
            walk_position: 0.0,
            walk_speed: 0.0,
            age_in_ticks: 0.0,
            light_probe: glam::DVec3::ZERO,
            flap: 0.0,
            flap_speed: 0.0,
            resting: false,
            rest_start: None,
            fly_start: None,
            swing: None,
            red_overlay: false,
            death_time: 0.0,
            turn: Quat::IDENTITY,
            riding: false,
        }
    }

    pub(super) fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: TEXTURE_SIZE,
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    pub(super) fn span(
        mesh: &ChunkMesh,
        axis: usize,
        keep: impl Fn(&[f32; 3]) -> bool,
    ) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position)
            .filter(|p| keep(p))
            .map(|p| p[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
    }

    fn drawn(look: &Look) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();
        append_model(
            &mut mesh,
            root_at(Vec3::ZERO, Quat::IDENTITY, 0.0),
            &skin(),
            look,
        );
        mesh
    }

    #[test]
    fn a_standing_margit_is_about_3_blocks_tall_and_1_4_wide_at_his_feet() {
        let mesh = drawn(&look_of(&MARGIT, &standing(), 0.0));
        assert_eq!(mesh.faces, 6 * 16, "fourteen cubes and two eyes");
        let (low, high) = span(&mesh, 1, |_| true);
        assert!(low.abs() < 0.01, "his feet are on the ground: {low}");
        assert!((2.85..=3.1).contains(&high), "about 3 blocks tall: {high}");
        let feet = |p: &[f32; 3]| p[1] < 0.5;
        let (left, right) = span(&mesh, 0, feet);
        assert!(
            (right - left - 1.4).abs() < 0.1,
            "1.4 wide at his feet: {}",
            right - left
        );
        let (front, back) = span(&mesh, 2, |_| true);
        assert!(
            front > -1.0 && back < 1.0,
            "inside his box's reach: {front} {back}"
        );
        let (_, face) = span(&mesh, 2, |p| p[1] > 1.8);
        assert!(
            face > 0.5,
            "hunched, his face is in front of his feet: {face}"
        );
        let (_, cane) = span(&mesh, 2, |p| p[1] < 0.2 && p[0] < -0.3);
        assert!(cane > 0.3, "his cane stands in front of him: {cane}");
    }

    #[test]
    fn every_part_samples_its_own_paint() {
        let areas = [
            (elden_skin::HEAD, elden_skin::HEAD_AREA),
            (elden_skin::HOOD, elden_skin::HOOD_AREA),
            (elden_skin::BEARD, elden_skin::BEARD_AREA),
            (elden_skin::HORN, elden_skin::HORN_AREA),
            (elden_skin::CLOTH, elden_skin::CLOTH_AREA),
            (elden_skin::WOOD, elden_skin::WOOD_AREA),
            (elden_skin::ARM, elden_skin::ARM_AREA),
            (elden_skin::SKIRT, elden_skin::SKIRT_AREA),
            (elden_skin::GOLD, elden_skin::GOLD_AREA),
        ];
        for cube in PARTS.iter().map(|(_, cube)| cube).chain(&EYES) {
            let [w, h, d] = cube.size;
            let (_, area) = areas.iter().find(|(uv, _)| *uv == cube.uv).unwrap();
            let reach = [cube.uv[0] + 2.0 * (d + w), cube.uv[1] + d + h];
            assert!(
                reach[0] <= area[2] as f32 && reach[1] <= area[3] as f32,
                "{cube:?} reaches {reach:?} past {area:?}"
            );
        }
    }

    #[test]
    fn his_head_follows_his_target_and_his_legs_swing_as_he_walks() {
        let mut pose = standing();
        pose.body_rot = 20.0;
        pose.head_yaw = 30.0;
        pose.head_pitch = 15.0;
        assert_eq!(head_turn(&pose, 10.0), 40.0);
        assert_eq!(head_turn(&pose, -100.0), MAX_HEAD_TURN);
        let look = look_of(&MARGIT, &pose, 10.0);
        let head = look.moved.get(Bone::Head.name()).rotation;
        assert!((head.y - 40.0 * keyframes::DEGREES).abs() < 1e-5);
        assert!((head.x - 15.0 * keyframes::DEGREES).abs() < 1e-5);
        let standing_still = look_of(&MARGIT, &standing(), 0.0);
        assert_eq!(
            standing_still.moved.get(Bone::RightLeg.name()).rotation,
            Vec3::ZERO
        );
        let mut walking = standing();
        walking.walk_speed = 1.0;
        let mut stride = |position: f32| {
            walking.walk_position = position;
            look_of(&MARGIT, &walking, 0.0)
                .moved
                .get(Bone::RightLeg.name())
                .rotation
                .x
        };
        assert!(
            stride(0.0) < -0.4 && stride(10.0) > 0.35,
            "{} {}",
            stride(0.0),
            stride(10.0)
        );
        let mut breathing = standing();
        breathing.age_in_ticks = 20.0;
        let body = look_of(&MARGIT, &breathing, 0.0)
            .moved
            .get(Bone::Body.name())
            .rotation
            .x;
        assert!(
            (body - 3.0 * keyframes::DEGREES).abs() < 1e-4,
            "he breathes: {body}"
        );
    }
}
