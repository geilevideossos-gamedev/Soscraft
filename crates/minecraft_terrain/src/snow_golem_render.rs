use crate::client_mobs::ClientMobs;
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::SnowGolemEntity;
use std::f32::consts::PI;

pub const TEXTURE: &str = "minecraft:entity/snow_golem/snow_golem";
pub const PUMPKIN: &str = "minecraft:carved_pumpkin";
pub const SHADOW_RADIUS: f32 = 0.5;
pub const DEFORMATION: f32 = -0.5;
pub const TEXTURE_SIZE: [f32; 2] = [64.0, 64.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Head,
    LeftArm,
    RightArm,
    UpperBody,
    LowerBody,
}

const ARM: Cube = Cube::new([-1.0, 0.0, -1.0], [12.0, 2.0, 2.0], [32.0, 0.0]);
const CUBES: [(Part, Cube); 5] = [
    (
        Part::Head,
        Cube::new([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], [0.0, 0.0]),
    ),
    (Part::LeftArm, ARM),
    (Part::RightArm, ARM),
    (
        Part::UpperBody,
        Cube::new([-5.0, -10.0, -5.0], [10.0, 10.0, 10.0], [0.0, 16.0]),
    ),
    (
        Part::LowerBody,
        Cube::new([-6.0, -12.0, -6.0], [12.0, 12.0, 12.0], [0.0, 36.0]),
    ),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnowGolemPose {
    pub head: Mat4,
    pub left_arm: Mat4,
    pub right_arm: Mat4,
    pub upper_body: Mat4,
    pub lower_body: Mat4,
}

pub fn snow_golem_pose(head_pitch: f32, head_yaw: f32) -> SnowGolemPose {
    let deg = 0.017_453_292_f32;
    let head = part_pose([0.0, 4.0, 0.0], [head_pitch * deg, head_yaw * deg, 0.0]);
    let upper = head_yaw * deg * 0.25;
    let (sin, cos) = (upper.sin(), upper.cos());
    SnowGolemPose {
        head,
        left_arm: part_pose([cos * 5.0, 6.0, -sin * 5.0], [0.0, upper, 1.0]),
        right_arm: part_pose([-cos * 5.0, 6.0, sin * 5.0], [0.0, upper + PI, -1.0]),
        upper_body: part_pose([0.0, 13.0, 0.0], [0.0, upper, 0.0]),
        lower_body: part_pose([0.0, 24.0, 0.0], [0.0; 3]),
    }
}

fn deformed(pose: Mat4, cube: &Cube) -> Mat4 {
    let center = Vec3::from_array(cube.from) + Vec3::from_array(cube.size) / 2.0;
    let size = Vec3::from_array(cube.size);
    let scale = (size + Vec3::splat(2.0 * DEFORMATION)) / size;
    pose * Mat4::from_translation(center / 16.0)
        * Mat4::from_scale(scale)
        * Mat4::from_translation(-center / 16.0)
}

pub fn model_root(feet: Mat4) -> Mat4 {
    feet * Mat4::from_translation(Vec3::new(0.0, 24.016 / 16.0, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, pose: &SnowGolemPose) {
    for (part, cube) in &CUBES {
        let matrix = match part {
            Part::Head => pose.head,
            Part::LeftArm => pose.left_arm,
            Part::RightArm => pose.right_arm,
            Part::UpperBody => pose.upper_body,
            Part::LowerBody => pose.lower_body,
        };
        append(mesh, deformed(root * matrix, cube), skin, cube);
    }
}

pub fn pumpkin_pose(root: Mat4, pose: &SnowGolemPose) -> Mat4 {
    root * pose.head
        * Mat4::from_translation(Vec3::new(0.0, -0.34375, 0.0))
        * Mat4::from_rotation_y(PI)
        * Mat4::from_scale(Vec3::new(0.625, -0.625, -0.625))
        * Mat4::from_translation(Vec3::splat(-0.5))
}

pub fn append_snow_golems<'a>(
    mesh: &mut ChunkMesh,
    golems: impl IntoIterator<Item = &'a SnowGolemEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) -> Vec<(Mat4, Vec3, String)> {
    let mut pumpkins = Vec::new();
    let Ok(texture) = ResourceId::parse(TEXTURE) else {
        return pumpkins;
    };
    let region = atlas.entity_region(&texture);
    let mut marks = Vec::new();
    for entity in golems {
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let position = mob.light_block();
        let skin = Skin {
            region,
            texture_size: TEXTURE_SIZE,
            sky: light.get(position) as f32,
            block: light.get_block(position) as f32,
            alpha: 1.0,
            shaded: true,
        };
        let pose = snow_golem_pose(mob.head_pitch, mob.head_yaw);
        let feet =
            Mat4::from_translation(mob.feet.as_vec3()) * Mat4::from_quat(mob.body_rotation(90.0));
        let root = model_root(feet);
        append_model(mesh, root, &skin, &pose);
        if entity.golem.pumpkin {
            pumpkins.push((
                pumpkin_pose(root, &pose),
                mob.light_probe.as_vec3(),
                PUMPKIN.to_owned(),
            ));
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
    pumpkins
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn model(pose: &SnowGolemPose) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();
        append_model(&mut mesh, model_root(Mat4::IDENTITY), &skin(), pose);
        mesh
    }

    fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
    }

    #[test]
    fn a_snow_golem_is_five_shrunk_cubes_standing_on_its_feet() {
        let mesh = model(&snow_golem_pose(0.0, 0.0));
        assert_eq!(mesh.faces, 5 * 6);
        let (low, high) = span(&mesh, 1);
        assert!((low - (24.016 - 24.0 + 0.5) / 16.0).abs() < 1e-4, "{low}");
        assert!((high - (24.016 + 4.0 - 0.5) / 16.0).abs() < 1e-4, "{high}");
    }

    #[test]
    fn the_arms_follow_a_quarter_of_the_head_turn() {
        let still = snow_golem_pose(0.0, 0.0);
        let turned = snow_golem_pose(0.0, 40.0);
        assert_ne!(still.left_arm, turned.left_arm);
        assert_eq!(still.lower_body, turned.lower_body);
        let arm = turned.left_arm.transform_point3(Vec3::ZERO) * 16.0;
        let angle = 40.0_f32.to_radians() * 0.25;
        assert!((arm.x - angle.cos() * 5.0).abs() < 1e-4);
        assert!((arm.z + angle.sin() * 5.0).abs() < 1e-4);
        let (left, right) = (
            still.left_arm.transform_point3(Vec3::ZERO),
            still.right_arm.transform_point3(Vec3::ZERO),
        );
        assert!((left.x + right.x).abs() < 1e-6 && left.x > 0.0);
    }

    #[test]
    fn the_pumpkin_sits_on_its_head_at_ten_sixteenths_of_a_block() {
        let pose = snow_golem_pose(0.0, 0.0);
        let block = pumpkin_pose(model_root(Mat4::IDENTITY), &pose);
        let low = block.transform_point3(Vec3::new(0.5, 0.0, 0.5));
        let high = block.transform_point3(Vec3::new(0.5, 1.0, 0.5));
        assert!((high.y - low.y - 0.625).abs() < 1e-5, "{low} {high}");
        let head_top = (24.016 + 4.0) / 16.0;
        assert!(low.y < head_top && high.y > head_top, "{low} {high}");
    }
}
