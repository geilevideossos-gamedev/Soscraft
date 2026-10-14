use crate::client_mobs::{ClientMobs, MobPose, mth_cos};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, HeldDisplay, HeldItem};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::vex::{MAINHAND, OFFHAND};
use minecraftoss_entities::world::VexEntity;
use std::f32::consts::{FRAC_PI_2, PI};

pub const TEXTURE: &str = "minecraft:entity/illager/vex";
pub const CHARGING_TEXTURE: &str = "minecraft:entity/illager/vex_charging";
pub const TEXTURES: [&str; 2] = [TEXTURE, CHARGING_TEXTURE];
pub const SHADOW_RADIUS: f32 = 0.3;
pub const TEXTURE_SIZE: [f32; 2] = [32.0, 32.0];
pub const BLOCK_LIGHT: f32 = 15.0;
const ROBE_DEFORMATION: f32 = -0.2;
const ARM_DEFORMATION: f32 = -0.1;
const ARM_ROLL: f32 = 0.628_318_55;
const BODY_TILT: f32 = 0.157_079_64;
const WING_TURN: f32 = 1.099_557_4;
const WING_TILT: f32 = 0.471_238_88;
const HAND_SCALE: f32 = 0.55;

const HEAD: Cube = Cube::new([-2.5, -5.0, -2.5], [5.0, 5.0, 5.0], [0.0, 0.0]);
const BODY: Cube = Cube::new([-1.5, 0.0, -1.0], [3.0, 4.0, 2.0], [0.0, 10.0]);
const ROBE: Cube = Cube::new([-1.5, 1.0, -1.0], [3.0, 5.0, 2.0], [0.0, 16.0]);
const RIGHT_ARM: Cube = Cube::new([-1.25, -0.5, -1.0], [2.0, 4.0, 2.0], [23.0, 0.0]);
const LEFT_ARM: Cube = Cube::new([-0.75, -0.5, -1.0], [2.0, 4.0, 2.0], [23.0, 6.0]);
const RIGHT_WING: Cube = Cube::new([0.0, 0.0, 0.0], [0.0, 5.0, 8.0], [16.0, 14.0]);
const LEFT_WING: Cube = Cube::new([0.0, 0.0, 0.0], [0.0, 5.0, 8.0], [16.0, 14.0]).mirrored();

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VexPose {
    pub head: Mat4,
    pub body: Mat4,
    pub right_arm: Mat4,
    pub left_arm: Mat4,
    pub right_wing: Mat4,
    pub left_wing: Mat4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VexLook {
    pub charging: bool,
    pub right_item: bool,
    pub left_item: bool,
}

pub fn vex_pose(mob: &MobPose, look: VexLook) -> VexPose {
    let deg = 0.017_453_292_f32;
    let age = mob.age_in_ticks;
    let sway = mth_cos(age * 5.5 * deg) * 0.1;
    let mut right = [0.0, 0.0, ARM_ROLL + sway];
    let mut left = [0.0, 0.0, -(ARM_ROLL + sway)];
    let body_tilt = if look.charging { 0.0 } else { BODY_TILT };
    if look.charging {
        let (raised, lowered) = (3.665_191_4, -1.221_730_5);
        let turn = 0.261_799_4;
        if !look.right_item && !look.left_item {
            right = [lowered, turn, -WING_TILT - sway];
            left = [lowered, -turn, WING_TILT + sway];
        }
        if look.right_item {
            right = [raised, turn, -WING_TILT - sway];
        }
        if look.left_item {
            left = [raised, -turn, WING_TILT + sway];
        }
    }
    let wing = WING_TURN + mth_cos(age * 45.836_624 * deg) * deg * 16.2;
    let root = part_pose([0.0, -2.5, 0.0], [0.0; 3]);
    let body = root * part_pose([0.0, 20.0, 0.0], [body_tilt, 0.0, 0.0]);
    VexPose {
        head: root
            * part_pose(
                [0.0, 20.0, 0.0],
                [mob.head_pitch * deg, mob.head_yaw * deg, 0.0],
            ),
        body,
        right_arm: body * part_pose([-1.75, 0.25, 0.0], right),
        left_arm: body * part_pose([1.75, 0.25, 0.0], left),
        right_wing: body * part_pose([-0.5, 1.0, 1.0], [WING_TILT, -wing, WING_TILT]),
        left_wing: body * part_pose([0.5, 1.0, 1.0], [WING_TILT, wing, -WING_TILT]),
    }
}

fn deformed(pose: Mat4, cube: &Cube, grow: f32) -> Mat4 {
    let center = Vec3::from_array(cube.from) + Vec3::from_array(cube.size) / 2.0;
    let size = Vec3::from_array(cube.size);
    let scale = (size + Vec3::splat(2.0 * grow)) / size;
    pose * Mat4::from_translation(center / 16.0)
        * Mat4::from_scale(scale)
        * Mat4::from_translation(-center / 16.0)
}

pub fn model_root(feet: Mat4) -> Mat4 {
    feet * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, pose: &VexPose) {
    append(mesh, root * pose.head, skin, &HEAD);
    append(mesh, root * pose.body, skin, &BODY);
    append(
        mesh,
        root * deformed(pose.body, &ROBE, ROBE_DEFORMATION),
        skin,
        &ROBE,
    );
    append(
        mesh,
        root * deformed(pose.right_arm, &RIGHT_ARM, ARM_DEFORMATION),
        skin,
        &RIGHT_ARM,
    );
    append(
        mesh,
        root * deformed(pose.left_arm, &LEFT_ARM, ARM_DEFORMATION),
        skin,
        &LEFT_ARM,
    );
    append(mesh, root * pose.right_wing, skin, &RIGHT_WING);
    append(mesh, root * pose.left_wing, skin, &LEFT_WING);
}

pub fn hand_pose(root: Mat4, arm: Mat4, left: bool) -> Mat4 {
    let side = if left { -1.0 } else { 1.0 };
    root * arm
        * Mat4::from_scale(Vec3::splat(HAND_SCALE))
        * Mat4::from_translation(Vec3::new(side * 0.046_875, -0.156_25, 0.078_125))
        * Mat4::from_rotation_x(-FRAC_PI_2)
        * Mat4::from_rotation_y(PI)
        * Mat4::from_translation(Vec3::new(side / 16.0, 0.125, -0.625))
}

pub fn append_vexes<'a>(
    mesh: &mut ChunkMesh,
    vexes: impl IntoIterator<Item = &'a VexEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) -> Vec<HeldItem> {
    let mut held = Vec::new();
    let mut marks = Vec::new();
    for entity in vexes {
        let vex = &entity.vex;
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        let name = if vex.charging {
            CHARGING_TEXTURE
        } else {
            TEXTURE
        };
        let Ok(texture) = ResourceId::parse(name) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let position = mob.light_block();
        let skin = Skin {
            region: atlas.entity_region(&texture),
            texture_size: TEXTURE_SIZE,
            sky: light.get(position) as f32,
            block: BLOCK_LIGHT,
            alpha: 1.0,
            shaded: true,
        };
        let (right, left) = if vex.left_handed {
            (&vex.equipment[OFFHAND], &vex.equipment[MAINHAND])
        } else {
            (&vex.equipment[MAINHAND], &vex.equipment[OFFHAND])
        };
        let look = VexLook {
            charging: vex.charging,
            right_item: right.is_some(),
            left_item: left.is_some(),
        };
        let pose = vex_pose(&mob, look);
        let feet =
            Mat4::from_translation(mob.feet.as_vec3()) * Mat4::from_quat(mob.body_rotation(90.0));
        let root = model_root(feet);
        append_model(mesh, root, &skin, &pose);
        for (stack, arm, is_left) in [(right, pose.right_arm, false), (left, pose.left_arm, true)] {
            let Some(stack) = stack else {
                continue;
            };
            held.push(HeldItem {
                pose: hand_pose(root, arm, is_left),
                light: mob.light_probe.as_vec3(),
                id: stack.id.clone(),
                display: if is_left {
                    HeldDisplay::LeftHand
                } else {
                    HeldDisplay::RightHand
                },
                first_tint: None,
            });
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
    held
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hovering() -> MobPose {
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
            turn: glam::Quat::IDENTITY,
            riding: false,
        }
    }

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: TEXTURE_SIZE,
            sky: 15.0,
            block: BLOCK_LIGHT,
            alpha: 1.0,
            shaded: true,
        }
    }

    fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
    }

    #[test]
    fn a_vex_floats_above_its_feet_within_its_box() {
        let look = VexLook {
            charging: false,
            right_item: true,
            left_item: false,
        };
        let mut mesh = ChunkMesh::default();
        append_model(
            &mut mesh,
            model_root(Mat4::IDENTITY),
            &skin(),
            &vex_pose(&hovering(), look),
        );
        assert_eq!(mesh.faces, 42, "seven cubes");
        let (low, high) = span(&mesh, 1);
        assert!(low > 0.0 && high < 0.8, "{low} {high}");
    }

    #[test]
    fn charging_raises_the_sword_arm_and_straightens_the_body() {
        let calm = vex_pose(
            &hovering(),
            VexLook {
                charging: false,
                right_item: true,
                left_item: false,
            },
        );
        let charging = vex_pose(
            &hovering(),
            VexLook {
                charging: true,
                right_item: true,
                left_item: false,
            },
        );
        assert_ne!(calm.body, charging.body);
        let hand = |pose: &VexPose| {
            pose.right_arm
                .transform_point3(Vec3::new(0.0, 4.0 / 16.0, 0.0))
        };
        assert!(
            hand(&charging).y < hand(&calm).y,
            "the arm swings up (y runs down in the model)"
        );
        let empty = vex_pose(
            &hovering(),
            VexLook {
                charging: true,
                right_item: false,
                left_item: false,
            },
        );
        assert_ne!(empty.right_arm, charging.right_arm);
    }

    #[test]
    fn the_sword_sits_in_the_right_hand() {
        let pose = vex_pose(
            &hovering(),
            VexLook {
                charging: false,
                right_item: true,
                left_item: false,
            },
        );
        let root = model_root(Mat4::from_quat(hovering().body_rotation(90.0)));
        let hand = hand_pose(root, pose.right_arm, false).transform_point3(Vec3::ZERO);
        assert!(hand.y > 0.0 && hand.y < 0.8, "{hand}");
        assert!(hand.x < 0.0, "the right hand is on its right: {hand}");
    }
}
