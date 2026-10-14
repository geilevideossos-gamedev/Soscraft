//! Bats: `BatModel.createBodyLayer` (26.3) posed by its keyframe
//! animations (`BatAnimation.BAT_FLYING` and `BAT_RESTING`, each from the
//! client tick its `AnimationState` started), turned with the body; a
//! resting bat also turns its head (`applyHeadRotation`).
use crate::{
    client_mobs::ClientMobs,
    cow_render::cube_tinted_pose,
    keyframes::{self, AnimationState, Definition, Interpolation, Keyframe, Moved, Target},
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::{EulerRot, Quat, Vec3};
use minecraftoss_entities::world::BatEntity;
use std::sync::LazyLock;

/// A part's placement: its rotation and origin (model pixels) in the
/// model's frame.
#[derive(Clone, Copy)]
struct Pose {
    rotation: Quat,
    origin: Vec3,
}

impl Pose {
    const ROOT: Pose = Pose { rotation: Quat::IDENTITY, origin: Vec3::ZERO };

    /// `ModelPart.translateAndRotate` of a child: its offset, then its
    /// rotation (Z, Y, X).
    fn child(self, offset: Vec3, rotation: Vec3) -> Pose {
        Pose {
            rotation: self.rotation * Quat::from_euler(EulerRot::ZYX, rotation.z, rotation.y, rotation.x),
            origin: self.origin + self.rotation * offset,
        }
    }
}

/// A keyframe channel: timestamps (seconds) and values, all linear.
type Channel = &'static [(f32, [f32; 3])];

/// The parts `BatAnimation` moves.
#[derive(Clone, Copy, PartialEq)]
enum Bone {
    Head,
    Body,
    Feet,
    RightWing,
    RightWingTip,
    LeftWing,
    LeftWingTip,
}

/// `(bone, position channel, rotation channel)`: positions in
/// `KeyframeAnimations.posVec` (y as authored; negated when applied),
/// rotations in degrees (`degreeVec`).
type Animation = &'static [(Bone, Channel, Channel)];

const RESTING: Animation = &[
    (Bone::Head, &[(0.0, [0.0, 0.5, 0.0])], &[(0.0, [180.0, 0.0, 0.0])]),
    (Bone::Body, &[(0.0, [0.0, 0.5, 0.0])], &[(0.0, [180.0, 0.0, 0.0])]),
    (Bone::Feet, &[], &[(0.0, [0.0, 0.0, 0.0])]),
    (Bone::RightWing, &[(0.0, [0.0, 0.0, 1.0])], &[(0.0, [0.0, -10.0, 0.0])]),
    (Bone::RightWingTip, &[], &[(0.0, [0.0, -120.0, 0.0])]),
    (Bone::LeftWing, &[(0.0, [0.0, 0.0, 1.0])], &[(0.0, [0.0, 10.0, 0.0])]),
    (Bone::LeftWingTip, &[], &[(0.0, [0.0, 120.0, 0.0])]),
];

const BOB: Channel = &[
    (0.0, [0.0, 0.0, 0.0]),
    (0.125, [0.0, 2.0, 0.0]),
    (0.25, [0.0, 1.0, 0.0]),
    (0.375, [0.0, 0.0, 0.0]),
    (0.4583, [0.0, -1.0, 0.0]),
    (0.5, [0.0, 0.0, 0.0]),
];

const FLYING: Animation = &[
    (Bone::Head, BOB, &[(0.0, [0.0, 0.0, 0.0]), (0.125, [20.0, 0.0, 0.0]), (0.5, [0.0, 0.0, 0.0])]),
    (Bone::Body, BOB, &[(0.0, [40.0, 0.0, 0.0]), (0.25, [52.5, 0.0, 0.0]), (0.5, [40.0, 0.0, 0.0])]),
    (Bone::Feet, &[], &[(0.0, [10.0, 0.0, 0.0]), (0.125, [-21.25, 0.0, 0.0]), (0.25, [-12.5, 0.0, 0.0]), (0.5, [10.0, 0.0, 0.0])]),
    (
        Bone::RightWing,
        &[],
        &[(0.0, [0.0, 85.0, 0.0]), (0.125, [0.0, -55.0, 0.0]), (0.25, [0.0, 50.0, 0.0]), (0.375, [0.0, 70.0, 0.0]), (0.5, [0.0, 85.0, 0.0])],
    ),
    (Bone::RightWingTip, &[], &[(0.0, [0.0, 10.5, 0.0]), (0.0417, [0.0, 65.5, 0.0]), (0.2083, [0.0, -135.0, 0.0]), (0.5, [0.0, 10.5, 0.0])]),
    (
        Bone::LeftWing,
        &[],
        &[(0.0, [0.0, -85.0, 0.0]), (0.125, [0.0, 55.0, 0.0]), (0.25, [0.0, -50.0, 0.0]), (0.375, [0.0, -70.0, 0.0]), (0.5, [0.0, -85.0, 0.0])],
    ),
    (Bone::LeftWingTip, &[], &[(0.0, [0.0, -10.5, 0.0]), (0.0417, [0.0, -65.5, 0.0]), (0.2083, [0.0, 135.0, 0.0]), (0.5, [0.0, -10.5, 0.0])]),
];

const LENGTH: f32 = 0.5;

impl Bone {
    const ALL: [Bone; 7] = [Bone::Head, Bone::Body, Bone::Feet, Bone::RightWing, Bone::RightWingTip, Bone::LeftWing, Bone::LeftWingTip];

    fn name(self) -> &'static str {
        match self {
            Bone::Head => "head",
            Bone::Body => "body",
            Bone::Feet => "feet",
            Bone::RightWing => "right_wing",
            Bone::RightWingTip => "right_wing_tip",
            Bone::LeftWing => "left_wing",
            Bone::LeftWingTip => "left_wing_tip",
        }
    }
}

fn definition(animation: Animation) -> Definition {
    let keys = |channel: Channel, vec: fn(f32, f32, f32) -> Vec3| -> Vec<Keyframe> {
        channel.iter().map(|&(t, [x, y, z])| Keyframe::new(t, vec(x, y, z), Interpolation::Linear)).collect()
    };
    let mut definition = Definition::new(LENGTH, true);
    for &(bone, position, rotation) in animation {
        if !rotation.is_empty() {
            definition.add(bone.name(), keyframes::Channel::new(Target::Rotation, keys(rotation, keyframes::degree_vec)));
        }
        if !position.is_empty() {
            definition.add(bone.name(), keyframes::Channel::new(Target::Position, keys(position, keyframes::pos_vec)));
        }
    }
    definition
}

static ANIMATIONS: LazyLock<[Definition; 2]> = LazyLock::new(|| [definition(RESTING), definition(FLYING)]);

/// A part's box and texture offset.
type Cube = ([f32; 3], [f32; 3], [f32; 2]);

pub fn append_bats<'a>(
    mesh: &mut ChunkMesh,
    bats: impl IntoIterator<Item = &'a BatEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let id = ResourceId::parse("minecraft:entity/bat/bat").unwrap();
    let region = atlas.entity_region(&id);
    // Each mob's first vertex and overlay (`getOverlayCoords`).
    let mut marks = Vec::new();
    for entity in bats {
        let Some(mob) = poses.pose(entity.id, partial) else { continue };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let sample_at = mob.light_block();
        let sky = light.get(sample_at) as f32;
        let block = light.get_block(sample_at) as f32;
        let rotation = mob.body_rotation(90.0);
        // `setupAnim`: the reset pose, a resting bat's head yaw, then the
        // running animation's offsets.
        let mut moved = [(Vec3::ZERO, Vec3::ZERO); 7];
        let index = |bone: Bone| bone as usize;
        if mob.resting {
            moved[index(Bone::Head)].1.y = mob.head_yaw * (std::f32::consts::PI / 180.0);
        }
        let running = if mob.resting { mob.rest_start.map(|s| (&ANIMATIONS[0], s)) } else { mob.fly_start.map(|s| (&ANIMATIONS[1], s)) };
        if let Some((animation, start)) = running {
            let mut offsets = Moved::default();
            animation.apply_state(AnimationState::started(start), mob.age_in_ticks, 1.0, &mut offsets);
            for bone in Bone::ALL {
                let offset = offsets.get(bone.name());
                moved[index(bone)].0 += offset.position;
                moved[index(bone)].1 += offset.rotation;
            }
        }
        let part = |parent: Pose, bone: Option<Bone>, pivot: [f32; 3]| {
            let (p, r) = bone.map_or((Vec3::ZERO, Vec3::ZERO), |b| moved[index(b)]);
            parent.child(Vec3::from_array(pivot) + p, r)
        };
        let body = part(Pose::ROOT, Some(Bone::Body), [0.0, 17.0, 0.0]);
        let head = part(Pose::ROOT, Some(Bone::Head), [0.0, 17.0, 0.0]);
        let right_wing = part(body, Some(Bone::RightWing), [-1.5, 0.0, 0.0]);
        let left_wing = part(body, Some(Bone::LeftWing), [1.5, 0.0, 0.0]);
        let parts: [(Pose, Cube); 9] = [
            (body, ([-1.5, 0.0, -1.0], [1.5, 5.0, 1.0], [0.0, 0.0])),
            (head, ([-2.0, -3.0, -1.0], [2.0, 0.0, 1.0], [0.0, 7.0])),
            (part(head, None, [-1.5, -2.0, 0.0]), ([-2.5, -4.0, 0.0], [0.5, 1.0, 0.0], [1.0, 15.0])),
            (part(head, None, [1.1, -3.0, 0.0]), ([-0.1, -3.0, 0.0], [2.9, 2.0, 0.0], [8.0, 15.0])),
            (right_wing, ([-2.0, -2.0, 0.0], [0.0, 5.0, 0.0], [12.0, 0.0])),
            (part(right_wing, Some(Bone::RightWingTip), [-2.0, 0.0, 0.0]), ([-6.0, -2.0, 0.0], [0.0, 6.0, 0.0], [16.0, 0.0])),
            (left_wing, ([0.0, -2.0, 0.0], [2.0, 5.0, 0.0], [12.0, 7.0])),
            (part(left_wing, Some(Bone::LeftWingTip), [2.0, 0.0, 0.0]), ([0.0, -2.0, 0.0], [6.0, 6.0, 0.0], [16.0, 8.0])),
            (part(body, Some(Bone::Feet), [0.0, 5.0, 0.0]), ([-1.5, 0.0, 0.0], [1.5, 2.0, 0.0], [16.0, 16.0])),
        ];
        for (pose, (from, to, uv)) in parts {
            cube_tinted_pose(mesh, mob.feet, rotation, 1.0, region, sky, block, from, to, uv, pose.origin.to_array(), pose.rotation, [1.0; 3], [32.0, 32.0], None);
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flying_at(start: i32, age_in_ticks: f32) -> Moved {
        let mut moved = Moved::default();
        ANIMATIONS[1].apply_state(AnimationState::started(start), age_in_ticks, 1.0, &mut moved);
        moved
    }

    #[test]
    fn keyframes_interpolate_linearly_between_their_pair() {
        let degrees = |moved: &Moved, bone: Bone| moved.get(bone.name()).rotation.x / keyframes::DEGREES;
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert_eq!(degrees(&flying_at(0, 0.0), Bone::Head), 0.0);
        assert!(close(degrees(&flying_at(0, 1.0), Bone::Head), 8.0));
        assert!(close(degrees(&flying_at(0, 2.5), Bone::Head), 20.0));
        assert!(close(degrees(&flying_at(10, 11.0), Bone::Head), 8.0));
        assert!(close(degrees(&flying_at(0, 5.0), Bone::Head), 40.0 / 3.0));
        assert_eq!(degrees(&flying_at(0, 10.0), Bone::Head), 0.0);
        assert_eq!(flying_at(0, 2.5).get(Bone::Head.name()).position, Vec3::new(0.0, -2.0, 0.0));
        let mut resting = Moved::default();
        ANIMATIONS[0].apply_state(AnimationState::started(0), 6.0, 1.0, &mut resting);
        assert_eq!(resting.get(Bone::Head.name()).rotation.x, 180.0 * keyframes::DEGREES);
        assert_eq!(resting.get(Bone::RightWingTip.name()).rotation.y, -120.0 * keyframes::DEGREES);
    }

    #[test]
    fn the_bat_tables_match_the_jar() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let bat = keyframes::read_from_jar(&jar, "BatAnimation").unwrap();
        for (field, ours) in [("BAT_RESTING", &ANIMATIONS[0]), ("BAT_FLYING", &ANIMATIONS[1])] {
            let theirs = bat.get(field).unwrap();
            assert_eq!((theirs.length, theirs.looping), (ours.length, ours.looping), "{field}");
            assert_eq!(theirs.channel_count(), ours.channel_count(), "{field}");
            for (bone, channels) in &theirs.bones {
                for channel in channels {
                    assert_eq!(ours.channel(bone, channel.target), Some(channel), "{field} {bone} {:?}", channel.target);
                }
            }
        }
    }
}
