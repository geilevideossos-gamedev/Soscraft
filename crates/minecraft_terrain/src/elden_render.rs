use crate::client_mobs::{ClientMobs, MobPose, rot_lerp};
use crate::elden_skin;
use crate::glow_render::FULL_BRIGHT;
use crate::keyframes::{self, Channel, Definition, Interpolation, Keyframe, Moved, Target};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use elden_boss::data::{BossDef, BranchTo, Conjure, Pose, PoseKey, Weapon};
use elden_boss::{Clip, State};
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
pub const GROW_TICKS: f32 = 4.0;
pub const TELL: u16 = 8;
pub const EYE_GROWTH: f32 = 1.0;
pub const HIDDEN_AROUND_SWAP: u16 = 2;
pub const RECOVERY: u16 = 10;

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

pub const DAGGER: [Cube; 3] = [
    Cube::new([-0.5, -1.0, -0.5], [1.0, 2.0, 1.0], elden_skin::GOLD),
    Cube::new([-1.5, 1.0, -0.5], [3.0, 1.0, 1.0], elden_skin::GOLD),
    Cube::new([-0.5, 2.0, -0.5], [1.0, 5.0, 1.0], elden_skin::GOLD),
];

pub const SWORD: [Cube; 3] = [
    Cube::new([-0.5, -2.0, -0.5], [1.0, 3.0, 1.0], elden_skin::GOLD),
    Cube::new([-2.0, 1.0, -1.0], [4.0, 1.0, 2.0], elden_skin::GOLD),
    Cube::new([-1.0, 2.0, -0.5], [2.0, 18.0, 1.0], elden_skin::GOLD),
];

pub const HAMMER: [Cube; 2] = [
    Cube::new([-0.5, -3.0, -0.5], [1.0, 20.0, 1.0], elden_skin::GOLD),
    Cube::new([-3.0, 16.0, -3.0], [6.0, 5.0, 6.0], elden_skin::GOLD),
];

pub fn weapon_parts(weapon: Weapon) -> &'static [Cube] {
    match weapon {
        Weapon::Dagger => &DAGGER,
        Weapon::Sword => &SWORD,
        Weapon::Hammer => &HAMMER,
    }
}

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

const fn sink(by: f32) -> Turn {
    Turn {
        bone: Bone::Root,
        rotation: [0.0; 3],
        position: [0.0, -by, 0.0],
    }
}

const fn rise(by: f32) -> Turn {
    sink(-by)
}

const fn grip(rotation: [f32; 3], up: f32) -> Turn {
    Turn {
        bone: Bone::Cane,
        rotation,
        position: [0.0, up, 0.0],
    }
}

const BREATHE: &[Turn] = &[
    turn(Bone::Body, [3.0, 0.0, 0.0]),
    turn(Bone::Head, [-3.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-2.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [2.0, 0.0, 0.0]),
];

const CANE_BACK: &[Turn] = &[
    turn(Bone::Body, [-5.0, 25.0, 0.0]),
    turn(Bone::Head, [0.0, -25.0, 0.0]),
    turn(Bone::RightArm, [-55.0, 60.0, 0.0]),
    turn(Bone::Cane, [-10.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-10.0, 0.0, -10.0]),
];

const SWING_THROUGH: &[Turn] = &[
    turn(Bone::Body, [5.0, -30.0, 0.0]),
    turn(Bone::Head, [0.0, 30.0, 0.0]),
    turn(Bone::RightArm, [-60.0, -55.0, 0.0]),
    turn(Bone::Cane, [-10.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [10.0, 0.0, 0.0]),
];

const SWING_BACK: &[Turn] = &[
    turn(Bone::Body, [5.0, 30.0, 0.0]),
    turn(Bone::Head, [0.0, -30.0, 0.0]),
    turn(Bone::RightArm, [-60.0, 65.0, 0.0]),
    turn(Bone::Cane, [-10.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-10.0, 0.0, 0.0]),
];

const CANE_HIGH: &[Turn] = &[
    turn(Bone::Body, [-15.0, 0.0, 0.0]),
    turn(Bone::Head, [10.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-150.0, 0.0, 10.0]),
    turn(Bone::Cane, [-10.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-20.0, 0.0, -10.0]),
];

const SLAM: &[Turn] = &[
    turn(Bone::Body, [20.0, 0.0, 0.0]),
    turn(Bone::Head, [-15.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-55.0, 0.0, 0.0]),
    turn(Bone::Cane, [-10.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [10.0, 0.0, 0.0]),
];

const DRAW: &[Turn] = &[
    turn(Bone::Body, [0.0, -10.0, 0.0]),
    turn(Bone::Head, [0.0, 10.0, 0.0]),
    turn(Bone::LeftArm, [-50.0, -20.0, 0.0]),
];

const THROW_WIND_UP: &[Turn] = &[
    turn(Bone::Body, [-10.0, -25.0, 0.0]),
    turn(Bone::Head, [5.0, 25.0, 0.0]),
    turn(Bone::LeftArm, [-150.0, -20.0, -10.0]),
];

const RELEASE: &[Turn] = &[
    turn(Bone::Body, [15.0, 20.0, 0.0]),
    turn(Bone::Head, [-10.0, -20.0, 0.0]),
    turn(Bone::RightArm, [-10.0, 0.0, 0.0]),
    turn(Bone::Cane, [25.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-80.0, 15.0, 0.0]),
];

const HOP: &[Turn] = &[
    rise(2.0),
    turn(Bone::Body, [-10.0, 0.0, 0.0]),
    turn(Bone::Head, [5.0, 0.0, 0.0]),
    turn(Bone::RightLeg, [-20.0, 0.0, 0.0]),
    turn(Bone::LeftLeg, [25.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-20.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-30.0, 0.0, -20.0]),
    turn(Bone::Skirt, [-10.0, 0.0, 0.0]),
];

const CROUCH: &[Turn] = &[
    sink(4.0),
    turn(Bone::Body, [20.0, 0.0, 0.0]),
    turn(Bone::Head, [-20.0, 0.0, 0.0]),
    turn(Bone::RightLeg, [-60.0, 0.0, 0.0]),
    turn(Bone::LeftLeg, [60.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-25.0, 0.0, 0.0]),
    grip([10.0, 0.0, 0.0], 6.0),
    turn(Bone::LeftArm, [20.0, 0.0, -15.0]),
    turn(Bone::Skirt, [-60.0, 0.0, 0.0]),
];

const AIRBORNE: &[Turn] = &[
    turn(Bone::Body, [-5.0, 0.0, 0.0]),
    turn(Bone::Head, [5.0, 0.0, 0.0]),
    turn(Bone::RightLeg, [-35.0, 0.0, 0.0]),
    turn(Bone::LeftLeg, [20.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-90.0, 0.0, 20.0]),
    turn(Bone::LeftArm, [-120.0, 0.0, -20.0]),
    turn(Bone::Skirt, [20.0, 0.0, 0.0]),
];

const LANDING: &[Turn] = &[
    sink(3.0),
    turn(Bone::Body, [30.0, 0.0, 0.0]),
    turn(Bone::Head, [-25.0, 0.0, 0.0]),
    turn(Bone::RightLeg, [-55.0, 0.0, 0.0]),
    turn(Bone::LeftLeg, [55.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-60.0, 0.0, 0.0]),
    turn(Bone::Cane, [-20.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-40.0, 0.0, -10.0]),
    turn(Bone::Skirt, [-50.0, 0.0, 0.0]),
];

const SWORD_OUT: &[Turn] = &[
    turn(Bone::Body, [-5.0, -15.0, 0.0]),
    turn(Bone::Head, [0.0, 15.0, 0.0]),
    turn(Bone::LeftArm, [-70.0, -60.0, 0.0]),
    turn(Bone::Hand, [80.0, 0.0, 0.0]),
];

const SWORD_BACK: &[Turn] = &[
    turn(Bone::Body, [0.0, -40.0, 0.0]),
    turn(Bone::Head, [0.0, 40.0, 0.0]),
    turn(Bone::LeftArm, [-80.0, -110.0, 0.0]),
    turn(Bone::Hand, [80.0, 0.0, 0.0]),
];

const SWEEP_THROUGH: &[Turn] = &[
    turn(Bone::Body, [10.0, 40.0, 0.0]),
    turn(Bone::Head, [-5.0, -40.0, 0.0]),
    turn(Bone::RightArm, [-5.0, 0.0, 0.0]),
    turn(Bone::Cane, [15.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-80.0, 70.0, 0.0]),
    turn(Bone::Hand, [80.0, 0.0, 0.0]),
];

const HAMMER_RAISED: &[Turn] = &[
    turn(Bone::Body, [-20.0, 0.0, 0.0]),
    turn(Bone::Head, [10.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-165.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-10.0, 0.0, 0.0]),
];

const HAMMER_DOWN: &[Turn] = &[
    turn(Bone::Body, [30.0, 0.0, 0.0]),
    turn(Bone::Head, [-25.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-30.0, 0.0, 0.0]),
    grip([10.0, 0.0, 0.0], 3.0),
    turn(Bone::LeftArm, [-70.0, 0.0, 0.0]),
    turn(Bone::Hand, [100.0, 0.0, 0.0]),
];

const HAND_RAISED: &[Turn] = &[
    turn(Bone::Body, [-15.0, 0.0, 0.0]),
    turn(Bone::Head, [-20.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-160.0, 0.0, -15.0]),
    turn(Bone::Hand, [90.0, 0.0, 0.0]),
];

const KNEEL: &[Turn] = &[
    sink(4.0),
    turn(Bone::Body, [15.0, 0.0, 0.0]),
    turn(Bone::Head, [-15.0, 0.0, 0.0]),
    turn(Bone::RightLeg, [-85.0, 0.0, 0.0]),
    turn(Bone::LeftLeg, [65.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-25.0, 0.0, 0.0]),
    grip([10.0, 0.0, 0.0], 5.0),
    turn(Bone::LeftArm, [15.0, 0.0, 0.0]),
    turn(Bone::Skirt, [-60.0, 0.0, 0.0]),
];

const RISE: &[Turn] = &[
    sink(2.0),
    turn(Bone::Body, [10.0, 0.0, 0.0]),
    turn(Bone::Head, [-10.0, 0.0, 0.0]),
    turn(Bone::RightLeg, [-45.0, 0.0, 0.0]),
    turn(Bone::LeftLeg, [45.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-20.0, 0.0, 0.0]),
    grip([5.0, 0.0, 0.0], 2.0),
    turn(Bone::Skirt, [-40.0, 0.0, 0.0]),
];

const VANISH: &[Turn] = &[
    turn(Bone::Body, [25.0, 0.0, 0.0]),
    turn(Bone::Head, [-15.0, 0.0, 0.0]),
    turn(Bone::RightLeg, [-15.0, 0.0, 0.0]),
    turn(Bone::LeftLeg, [-15.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-30.0, 0.0, 15.0]),
    turn(Bone::Cane, [40.0, 0.0, 0.0]),
    turn(Bone::LeftArm, [-20.0, 0.0, -15.0]),
    turn(Bone::Skirt, [-15.0, 0.0, 0.0]),
];

const COLLAPSE: &[Turn] = &[
    sink(6.0),
    turn(Bone::Body, [35.0, 0.0, 0.0]),
    turn(Bone::Head, [10.0, 0.0, 0.0]),
    turn(Bone::RightLeg, [-90.0, 0.0, 0.0]),
    turn(Bone::LeftLeg, [-90.0, 0.0, 0.0]),
    turn(Bone::RightArm, [-70.0, 0.0, 0.0]),
    grip([-10.0, 0.0, 0.0], 8.0),
    turn(Bone::LeftArm, [-60.0, 0.0, 0.0]),
    turn(Bone::Skirt, [-75.0, 0.0, 0.0]),
];

pub fn turns(pose: Pose) -> &'static [Turn] {
    match pose {
        Pose::Ready => &[],
        Pose::Breathe => BREATHE,
        Pose::CaneBack => CANE_BACK,
        Pose::SwingThrough => SWING_THROUGH,
        Pose::SwingBack => SWING_BACK,
        Pose::CaneHigh => CANE_HIGH,
        Pose::Slam => SLAM,
        Pose::Draw => DRAW,
        Pose::ThrowWindUp => THROW_WIND_UP,
        Pose::Release => RELEASE,
        Pose::Hop => HOP,
        Pose::Crouch => CROUCH,
        Pose::Airborne => AIRBORNE,
        Pose::Landing => LANDING,
        Pose::SwordOut => SWORD_OUT,
        Pose::SwordBack => SWORD_BACK,
        Pose::SweepThrough => SWEEP_THROUGH,
        Pose::HammerRaised => HAMMER_RAISED,
        Pose::HammerDown => HAMMER_DOWN,
        Pose::HandRaised => HAND_RAISED,
        Pose::Kneel => KNEEL,
        Pose::Rise => RISE,
        Pose::Vanish => VANISH,
        Pose::Collapse => COLLAPSE,
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
    pub wake: Definition,
    pub kneel: Definition,
    pub fade: Definition,
    pub death: Definition,
    pub moves: Vec<Definition>,
    pub shifts: Vec<Option<Definition>>,
}

impl Animations {
    pub fn of(def: &'static BossDef) -> Animations {
        let clips = &def.clips;
        let once = |length: u16, poses: &[PoseKey]| clip(length, poses, false);
        Animations {
            id: def.id,
            idle: clip(clips.idle.length, clips.idle.poses, true),
            walk: walk(),
            wake: once(clips.wake.length, clips.wake.poses),
            kneel: once(clips.kneel.length, clips.kneel.poses),
            fade: once(clips.fade.length, clips.fade.poses),
            death: once(clips.death.length, clips.death.poses),
            moves: def
                .moves
                .iter()
                .map(|mv| once(mv.length, mv.poses))
                .collect(),
            shifts: def
                .phases
                .iter()
                .map(|phase| phase.shift.map(|shift| once(shift.length, shift.poses)))
                .collect(),
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
    pub weapon: Option<(Weapon, f32)>,
    pub eyes: Option<f32>,
    pub hidden: bool,
}

pub fn clip_time(def: &'static BossDef, clip: &Clip, partial: f32) -> f32 {
    let hold = clip.played(def).and_then(|mv| mv.hold);
    let frozen = clip.held || hold.is_some_and(|hold| hold.at == clip.clock);
    f32::from(clip.clock) + if frozen { 0.0 } else { partial }
}

pub fn weapon(def: &'static BossDef, clip: &Clip, partial: f32) -> Option<(Weapon, f32)> {
    let conjure: &[Conjure] = match clip.state {
        State::Move => clip.played(def)?.conjure,
        State::Shift => def.shift(clip.phase)?.conjure,
        _ => return None,
    };
    let shown = conjure
        .iter()
        .find(|conjure| conjure.from <= clip.clock && clip.clock <= conjure.to)?;
    let grown = (clip_time(def, clip, partial) - f32::from(shown.from)) / GROW_TICKS;
    Some((shown.weapon, grown.clamp(0.0, 1.0)))
}

pub fn flare(def: &'static BossDef, clip: &Clip, partial: f32) -> Option<f32> {
    if clip.state != State::Move || clip.held {
        return None;
    }
    let mv = clip.played(def)?;
    let hold = mv.hold?;
    mv.strikes
        .iter()
        .filter(|strike| strike.from > hold.at)
        .find_map(|strike| {
            let start = strike.from.saturating_sub(TELL).max(hold.at + 1);
            (start..strike.from).contains(&clip.clock).then(|| {
                let span = f32::from(strike.from - start);
                ((f32::from(clip.clock - start) + partial) / span).clamp(0.0, 1.0)
            })
        })
}

pub fn hidden(def: &BossDef, clip: &Clip) -> bool {
    clip.state == State::Fade && clip.clock.abs_diff(def.arena.fade_swap) <= HIDDEN_AROUND_SWAP
}

fn sample(definition: &Definition, ticks: f32, moved: &mut Moved) {
    let ticks = ticks.clamp(0.0, definition.length / TICK);
    definition.apply(millis(ticks), 1.0, moved);
}

fn recover(
    def: &'static BossDef,
    animations: &Animations,
    clip: &Clip,
    time: f32,
    moved: &mut Moved,
) {
    let (Some(mv), Some(definition)) = (
        clip.played(def),
        animations.moves.get(usize::from(clip.index)),
    ) else {
        return;
    };
    let recovery = mv
        .branches
        .iter()
        .find_map(|branch| match branch.to {
            BranchTo::End { recovery } if branch.at == clip.from => Some(recovery),
            _ => None,
        })
        .unwrap_or(RECOVERY);
    let mut cut = Moved::default();
    sample(definition, f32::from(clip.from), &mut cut);
    let done = (time / f32::from(recovery.max(1))).clamp(0.0, 1.0);
    let keep = 1.0 - done * done * (3.0 - 2.0 * done);
    for (bone, offsets) in cut.iter() {
        moved.add(bone, Target::Position, offsets.position * keep);
        moved.add(bone, Target::Rotation, offsets.rotation * keep);
    }
}

pub fn look(def: &'static BossDef, clip: &Clip, pose: &MobPose, facing: f32, partial: f32) -> Look {
    let Some(animations) = animations(def) else {
        return look_of(def, pose, facing);
    };
    let mut moved = Moved::default();
    let time = clip_time(def, clip, partial);
    match clip.state {
        State::Idle | State::Return => return look_of(def, pose, facing),
        State::Asleep => sample(&animations.wake, 0.0, &mut moved),
        State::Wake => sample(&animations.wake, time, &mut moved),
        State::Kneel => sample(&animations.kneel, time, &mut moved),
        State::Fade => sample(&animations.fade, time, &mut moved),
        State::Dead => sample(&animations.death, time, &mut moved),
        State::Shift => {
            let phase = usize::from(clip.phase.max(1)) - 1;
            if let Some(Some(shift)) = animations.shifts.get(phase) {
                sample(shift, time, &mut moved);
            }
        }
        State::Move => {
            if let Some(definition) = animations.moves.get(usize::from(clip.index)) {
                sample(definition, time, &mut moved);
            }
        }
        State::Recover => recover(def, animations, clip, time, &mut moved),
    }
    turn_head(pose, facing, &mut moved);
    Look {
        moved,
        weapon: weapon(def, clip, partial),
        eyes: flare(def, clip, partial),
        hidden: hidden(def, clip),
    }
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
    turn_head(pose, facing, &mut moved);
    Look {
        moved,
        ..Look::default()
    }
}

fn turn_head(pose: &MobPose, facing: f32, moved: &mut Moved) {
    let yaw = head_turn(pose, facing);
    moved.add(
        Bone::Head.name(),
        Target::Rotation,
        keyframes::degree_vec(pose.head_pitch, yaw, 0.0),
    );
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

fn about(cube: &Cube, scale: f32) -> Mat4 {
    let centre = (Vec3::from_array(cube.from) + Vec3::from_array(cube.size) / 2.0) / 16.0;
    Mat4::from_translation(centre)
        * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_translation(-centre)
}

pub fn bright(skin: &Skin) -> Skin {
    Skin {
        sky: FULL_BRIGHT,
        block: FULL_BRIGHT,
        shaded: false,
        ..*skin
    }
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, look: &Look) {
    if look.hidden {
        return;
    }
    let bones = posed(&look.moved);
    for (bone, cube) in &PARTS {
        append(mesh, root * bones[*bone as usize], skin, cube);
    }
    let glowing = bright(skin);
    let head = root * bones[Bone::Head as usize];
    let (eyes, eye_skin) = match look.eyes {
        Some(flare) => (1.0 + flare * EYE_GROWTH, &glowing),
        None => (1.0, skin),
    };
    for cube in &EYES {
        append(mesh, head * about(cube, eyes), eye_skin, cube);
    }
    if let Some((weapon, grown)) = look.weapon {
        if grown > 0.0 {
            let hand = root * bones[Bone::Hand as usize] * Mat4::from_scale(Vec3::splat(grown));
            for cube in weapon_parts(weapon) {
                append(mesh, hand, &glowing, cube);
            }
        }
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
        let look = look(mob.state.def, &mob.state.shown, &pose, facing, shown);
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

    fn standing() -> MobPose {
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

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: TEXTURE_SIZE,
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    fn span(mesh: &ChunkMesh, axis: usize, keep: impl Fn(&[f32; 3]) -> bool) -> (f32, f32) {
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

    fn shown_at(state: State, key: &str, clock: u16, held: bool) -> Clip {
        let index = MARGIT.move_index(key).map_or(0, |index| index as u8);
        Clip {
            state,
            index,
            clock,
            held,
            phase: 2,
            awake: true,
            from: 0,
            criticals: 0,
        }
    }

    fn moved_by(look: &Look, bone: Bone) -> Vec3 {
        look.moved.get(bone.name()).rotation
    }

    #[test]
    fn every_pose_sets_every_bone_it_names() {
        for pose in Pose::ALL {
            let named = turns(pose);
            for (at, turn) in named.iter().enumerate() {
                assert!(
                    named[..at].iter().all(|other| other.bone != turn.bone),
                    "{pose:?} names {:?} twice",
                    turn.bone
                );
            }
            let keys = [PoseKey { at: 0, pose }, PoseKey { at: 20, pose }];
            let mut moved = Moved::default();
            clip(20, &keys, false).apply(500, 1.0, &mut moved);
            for bone in Bone::ALL {
                let offsets = moved.get(bone.name());
                match named.iter().find(|turn| turn.bone == bone) {
                    Some(turn) => {
                        let [x, y, z] = turn.rotation;
                        let rotation = keyframes::degree_vec(x, y, z);
                        assert!(
                            (offsets.rotation - rotation).length() < 1e-5,
                            "{pose:?} {bone:?}"
                        );
                        let [x, y, z] = turn.position;
                        let position = keyframes::pos_vec(x, y, z);
                        assert!(
                            (offsets.position - position).length() < 1e-5,
                            "{pose:?} {bone:?}"
                        );
                    }
                    None => assert_eq!(offsets, keyframes::Offsets::default(), "{pose:?} {bone:?}"),
                }
            }
        }
        assert!(turns(Pose::Ready).is_empty());
    }

    #[test]
    fn each_moves_clip_lasts_as_long_as_the_move() {
        let animations = animations(&MARGIT).unwrap();
        assert_eq!(animations.moves.len(), MARGIT.moves.len());
        let lasts = |definition: &Definition, ticks: u16| {
            (definition.length - f32::from(ticks) * TICK).abs() < 1e-6
        };
        for (mv, definition) in MARGIT.moves.iter().zip(&animations.moves) {
            assert!(
                lasts(definition, mv.length) && !definition.looping,
                "{}",
                mv.key
            );
        }
        let clips = &MARGIT.clips;
        assert!(lasts(&animations.wake, clips.wake.length));
        assert!(lasts(&animations.kneel, clips.kneel.length));
        assert!(lasts(&animations.fade, clips.fade.length));
        assert!(lasts(&animations.death, clips.death.length));
        assert!(lasts(&animations.idle, clips.idle.length) && animations.idle.looping);
        assert!(animations.shifts[0].is_none());
        let shift = animations.shifts[1].as_ref().unwrap();
        assert!(lasts(shift, MARGIT.phases[1].shift.unwrap().length));
        let swing = look(
            &MARGIT,
            &shown_at(State::Move, "cane_combo", 13, false),
            &standing(),
            0.0,
            0.0,
        );
        let [x, y, z] = SWING_THROUGH[2].rotation;
        assert_eq!(SWING_THROUGH[2].bone, Bone::RightArm);
        let through = keyframes::degree_vec(x, y, z);
        assert!(
            (moved_by(&swing, Bone::RightArm) - through).length() < 1e-3,
            "the first swing is drawn on its strike"
        );
        let ended = look(
            &MARGIT,
            &shown_at(State::Move, "cane_combo", 70, false),
            &standing(),
            0.0,
            0.0,
        );
        assert!(
            moved_by(&ended, Bone::RightArm).length() < 1e-3,
            "back to ready at its end"
        );
    }

    #[test]
    fn a_held_swing_keeps_its_pose_while_the_clock_stops() {
        let pose = standing();
        let at = |clock: u16, held: bool, partial: f32| {
            look(
                &MARGIT,
                &shown_at(State::Move, "cane_combo", clock, held),
                &pose,
                0.0,
                partial,
            )
        };
        let held = at(34, true, 0.0);
        for partial in [0.25, 0.5, 0.99] {
            assert_eq!(at(34, true, partial), held);
            assert_eq!(at(34, false, partial), held, "the tick the hold starts on");
        }
        assert!(
            moved_by(&held, Bone::RightArm).x < -140.0 * keyframes::DEGREES,
            "the cane is up"
        );
        assert_ne!(at(35, false, 0.0), held, "the clock runs on after the hold");
        let time = |clock: u16, held: bool| {
            clip_time(
                &MARGIT,
                &shown_at(State::Move, "cane_combo", clock, held),
                0.5,
            )
        };
        assert_eq!(
            (time(34, true), time(34, false), time(35, false)),
            (34.0, 34.0, 35.5)
        );
    }

    #[test]
    fn the_dagger_part_shows_only_while_conjured() {
        let throw = |clock: u16, partial: f32| {
            weapon(
                &MARGIT,
                &shown_at(State::Move, "dagger_throw", clock, false),
                partial,
            )
        };
        for clock in 0..36 {
            let shown = throw(clock, 0.0).map(|(weapon, _)| weapon);
            let conjured = (2..=22).contains(&clock).then_some(Weapon::Dagger);
            assert_eq!(shown, conjured, "{clock}");
        }
        let grown: Vec<f32> = (2..7).map(|clock| throw(clock, 0.0).unwrap().1).collect();
        assert_eq!(grown, [0.0, 0.25, 0.5, 0.75, 1.0], "it grows over 4 ticks");
        assert_eq!(throw(3, 0.5).unwrap().1, 0.375);
        let faces = |clock: u16| {
            let shown = shown_at(State::Move, "dagger_throw", clock, false);
            drawn(&look(&MARGIT, &shown, &standing(), 0.0, 0.0)).faces
        };
        assert_eq!(faces(12), 6 * (16 + DAGGER.len()));
        assert_eq!((faces(2), faces(30)), (6 * 16, 6 * 16));
        let sword = weapon(
            &MARGIT,
            &shown_at(State::Move, "sword_sweep", 10, false),
            0.0,
        );
        assert_eq!(sword, Some((Weapon::Sword, 1.0)));
        let shift = Clip {
            state: State::Shift,
            clock: 12,
            ..shown_at(State::Shift, "", 0, false)
        };
        assert_eq!(weapon(&MARGIT, &shift, 0.0), Some((Weapon::Hammer, 1.0)));
        let kneel = Clip {
            state: State::Kneel,
            ..shift
        };
        assert_eq!(weapon(&MARGIT, &kneel, 0.0), None);
    }

    #[test]
    fn the_golden_weapons_are_full_bright_and_gold_in_the_skin() {
        let painted = elden_skin::paint();
        let plain = look_of(&MARGIT, &standing(), 0.0);
        let body = drawn(&plain);
        assert!(body.vertices.iter().all(|vertex| vertex.block_light == 0.0));
        for weapon in [Weapon::Dagger, Weapon::Sword, Weapon::Hammer] {
            let armed = drawn(&Look {
                weapon: Some((weapon, 1.0)),
                ..plain.clone()
            });
            let added = &armed.vertices[body.vertices.len()..];
            assert_eq!(added.len(), 24 * weapon_parts(weapon).len(), "{weapon:?}");
            let [u0, v0, u1, v1] = elden_skin::GOLD_AREA.map(|edge| edge as f32 / 64.0);
            for vertex in added {
                assert_eq!(
                    (vertex.sky_light, vertex.block_light),
                    (FULL_BRIGHT, FULL_BRIGHT)
                );
                let [u, v] = vertex.uv;
                assert!(
                    (u0..=u1).contains(&u) && (v0..=v1).contains(&v),
                    "{weapon:?} {u} {v}"
                );
            }
            for cube in weapon_parts(weapon) {
                let [w, h, d] = cube.size;
                let (width, height) = ((2.0 * (d + w)).ceil() as u32, (d + h).ceil() as u32);
                let [u, v] = cube.uv.map(|at| at as u32);
                for y in v..v + height {
                    for x in u..u + width {
                        let pixel = painted.get_pixel(x, y);
                        assert!(
                            pixel[0] > 200 && pixel[2] < 140,
                            "{weapon:?} {x} {y} {pixel:?}"
                        );
                    }
                }
            }
        }
        let half = drawn(&Look {
            weapon: Some((Weapon::Sword, 0.5)),
            ..plain.clone()
        });
        let reach = |mesh: &ChunkMesh| {
            let (_, front) = span(mesh, 2, |_| true);
            front
        };
        assert!(reach(&body) < reach(&half), "half grown");
    }

    #[test]
    fn his_eyes_flare_before_a_delayed_swing() {
        let flares = |key: &str| -> Vec<u16> {
            (0..80)
                .filter(|&clock| {
                    flare(&MARGIT, &shown_at(State::Move, key, clock, false), 0.0).is_some()
                })
                .collect()
        };
        assert_eq!(flares("cane_combo"), (36..44).collect::<Vec<_>>());
        assert_eq!(flares("cane_slam"), (21..28).collect::<Vec<_>>());
        assert_eq!(flares("hammer_slam"), (32..40).collect::<Vec<_>>());
        for key in [
            "dagger_throw",
            "backstep_daggers",
            "leap_slam",
            "sword_sweep",
            "hammer_leap",
        ] {
            assert!(flares(key).is_empty(), "{key} has no delayed swing");
        }
        let held = shown_at(State::Move, "cane_combo", 34, true);
        assert_eq!(flare(&MARGIT, &held, 0.0), None, "dark while he holds");
        let rising: Vec<f32> = (36..44)
            .map(|clock| {
                flare(
                    &MARGIT,
                    &shown_at(State::Move, "cane_combo", clock, false),
                    0.5,
                )
                .unwrap()
            })
            .collect();
        assert!(
            rising.windows(2).all(|pair| pair[0] < pair[1]),
            "{rising:?}"
        );
        assert!(rising[0] > 0.0 && rising[7] < 1.0);
        let calm = look_of(&MARGIT, &standing(), 0.0);
        let flaring = Look {
            eyes: Some(1.0),
            ..calm.clone()
        };
        let eyes = |look: &Look| {
            let mesh = drawn(look);
            mesh.vertices[24 * PARTS.len()..24 * (PARTS.len() + EYES.len())].to_vec()
        };
        let (dim, bright) = (eyes(&calm), eyes(&flaring));
        assert!(dim.iter().all(|vertex| vertex.block_light == 0.0));
        assert!(
            bright
                .iter()
                .all(|vertex| vertex.sky_light == FULL_BRIGHT && vertex.block_light == FULL_BRIGHT)
        );
        let width = |vertices: &[crate::mesh::Vertex]| {
            let xs = vertices[..24].iter().map(|vertex| vertex.position[0]);
            let (lo, hi) = xs.fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
            hi - lo
        };
        assert!(
            (width(&bright) / width(&dim) - 2.0).abs() < 1e-3,
            "they grow to twice their size"
        );
    }

    #[test]
    fn asleep_he_kneels_and_a_fade_hides_him_around_its_swap() {
        let asleep = Clip {
            state: State::Asleep,
            awake: false,
            ..shown_at(State::Asleep, "", 0, false)
        };
        let kneeling = drawn(&look(&MARGIT, &asleep, &standing(), 0.0, 0.0));
        let (_, top) = span(&kneeling, 1, |_| true);
        assert!(top > 2.3 && top < 2.7, "kneeling: {top}");
        let fade = |clock: u16| Clip {
            state: State::Fade,
            clock,
            ..asleep
        };
        let shown: Vec<bool> = (0..20)
            .map(|clock| drawn(&look(&MARGIT, &fade(clock), &standing(), 0.0, 0.0)).faces > 0)
            .collect();
        let swap = MARGIT.arena.fade_swap;
        for (clock, seen) in shown.iter().enumerate() {
            assert_eq!(
                *seen,
                (clock as u16).abs_diff(swap) > HIDDEN_AROUND_SWAP,
                "{clock}"
            );
        }
    }

    #[test]
    fn a_recovery_eases_from_where_the_combo_was_cut_back_to_ready() {
        let cut = Clip {
            state: State::Recover,
            from: 16,
            ..shown_at(State::Recover, "cane_combo", 0, false)
        };
        let at = |clock: u16| {
            moved_by(
                &look(&MARGIT, &Clip { clock, ..cut }, &standing(), 0.0, 0.0),
                Bone::RightArm,
            )
        };
        let start = look(
            &MARGIT,
            &shown_at(State::Move, "cane_combo", 16, false),
            &standing(),
            0.0,
            0.0,
        );
        assert!((at(0) - moved_by(&start, Bone::RightArm)).length() < 1e-4);
        assert!(at(7).length() < at(0).length() && at(7).length() > 0.0);
        assert!(at(14).length() < 1e-5, "ready after its 14 ticks");
    }
}
