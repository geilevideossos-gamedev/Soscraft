use crate::mesh::{ChunkMesh, Vertex};
use glam::{Mat3, Mat4, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cube {
    pub from: [f32; 3],
    pub size: [f32; 3],
    pub uv: [f32; 2],
    pub mirror: bool,
}

impl Cube {
    pub const fn new(from: [f32; 3], size: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            from,
            size,
            uv,
            mirror: false,
        }
    }

    pub const fn mirrored(self) -> Self {
        Self {
            mirror: true,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skin {
    pub region: [f32; 4],
    pub texture_size: [f32; 2],
    pub sky: f32,
    pub block: f32,
    pub alpha: f32,
    pub shaded: bool,
}

pub fn part_pose(offset: [f32; 3], rotation: [f32; 3]) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(offset) / 16.0)
        * Mat4::from_rotation_z(rotation[2])
        * Mat4::from_rotation_y(rotation[1])
        * Mat4::from_rotation_x(rotation[0])
}

pub fn append(mesh: &mut ChunkMesh, pose: Mat4, skin: &Skin, cube: &Cube) {
    let [x0, y0, z0] = cube.from;
    let [x1, y1, z1] = [
        cube.from[0] + cube.size[0],
        cube.from[1] + cube.size[1],
        cube.from[2] + cube.size[2],
    ];
    let (x_lo, x_hi) = if cube.mirror { (x1, x0) } else { (x0, x1) };
    let [w, h, d] = cube.size;
    let [u, v] = cube.uv;
    let u1 = u + d;
    let u2 = u1 + w;
    let u22 = u2 + w;
    let u3 = u2 + d;
    let u4 = u3 + w;
    let v1 = v + d;
    let v2 = v1 + h;
    let side = if cube.mirror { -1.0 } else { 1.0 };
    let faces = [
        (
            [
                [x_hi, y0, z1],
                [x_lo, y0, z1],
                [x_lo, y0, z0],
                [x_hi, y0, z0],
            ],
            [u1, v, u2, v1],
            [0.0, -1.0, 0.0],
        ),
        (
            [
                [x_hi, y1, z0],
                [x_lo, y1, z0],
                [x_lo, y1, z1],
                [x_hi, y1, z1],
            ],
            [u2, v1, u22, v],
            [0.0, 1.0, 0.0],
        ),
        (
            [
                [x_lo, y0, z0],
                [x_lo, y0, z1],
                [x_lo, y1, z1],
                [x_lo, y1, z0],
            ],
            [u, v1, u1, v2],
            [-side, 0.0, 0.0],
        ),
        (
            [
                [x_hi, y0, z0],
                [x_lo, y0, z0],
                [x_lo, y1, z0],
                [x_hi, y1, z0],
            ],
            [u1, v1, u2, v2],
            [0.0, 0.0, -1.0],
        ),
        (
            [
                [x_hi, y0, z1],
                [x_hi, y0, z0],
                [x_hi, y1, z0],
                [x_hi, y1, z1],
            ],
            [u2, v1, u3, v2],
            [side, 0.0, 0.0],
        ),
        (
            [
                [x_lo, y0, z1],
                [x_hi, y0, z1],
                [x_hi, y1, z1],
                [x_lo, y1, z1],
            ],
            [u3, v1, u4, v2],
            [0.0, 0.0, 1.0],
        ),
    ];
    let normals = Mat3::from_mat4(pose).inverse().transpose();
    let region = skin.region;
    for (corners, tex, normal) in faces {
        let n = (normals * Vec3::from_array(normal)).normalize_or_zero();
        let shade = if skin.shaded {
            crate::cow_render::entity_shade(n)
        } else {
            1.0
        };
        let mut vertices = std::array::from_fn::<_, 4, _>(|i| {
            (
                corners[i],
                [
                    [tex[2], tex[1]],
                    [tex[0], tex[1]],
                    [tex[0], tex[3]],
                    [tex[2], tex[3]],
                ][i],
            )
        });
        if cube.mirror {
            vertices.reverse();
        }
        let start = mesh.vertices.len() as u32;
        for (corner, coord) in vertices {
            let world = pose.transform_point3(Vec3::from_array(corner) / 16.0);
            mesh.vertices.push(Vertex {
                position: world.to_array(),
                uv: [
                    region[0] + (region[2] - region[0]) * coord[0] / skin.texture_size[0],
                    region[1] + (region[3] - region[1]) * coord[1] / skin.texture_size[1],
                ],
                color: [shade, shade, shade, skin.alpha],
                sky_light: skin.sky,
                block_light: skin.block,
            });
        }
        mesh.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use glam::Quat;

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: [64.0, 32.0],
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    #[test]
    fn a_posed_cube_matches_the_living_entity_cuboid_under_its_flip() {
        let cube = Cube::new([-4.0, 0.0, -3.0], [8.0, 12.0, 6.0], [16.0, 20.0]);
        for mirror in [false, true] {
            let cube = Cube { mirror, ..cube };
            let rotation = Quat::from_rotation_y(0.7);
            let mut expected = ChunkMesh::default();
            crate::cow_render::cube_scaled(
                &mut expected,
                DVec3::new(1.0, 2.0, 3.0),
                rotation,
                Vec3::ONE,
                skin().region,
                15.0,
                0.0,
                cube.from,
                [8.0 - 4.0, 12.0, 3.0],
                cube.uv,
                [0.0; 3],
                Quat::IDENTITY,
                [1.0; 3],
                [64.0, 32.0],
                None,
                mirror,
            );
            let pose = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0))
                * Mat4::from_quat(rotation)
                * Mat4::from_translation(Vec3::new(0.0, 24.016 / 16.0, 0.0))
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
            let mut mesh = ChunkMesh::default();
            append(&mut mesh, pose, &skin(), &cube);
            assert_eq!(mesh.indices, expected.indices);
            for (a, b) in mesh.vertices.iter().zip(&expected.vertices) {
                for k in 0..3 {
                    assert!((a.position[k] - b.position[k]).abs() < 1e-4, "{a:?} {b:?}");
                }
                assert_eq!(a.uv, b.uv);
                assert!((a.color[0] - b.color[0]).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn a_part_pose_turns_z_then_y_then_x_about_its_offset() {
        let pose = part_pose([0.0, 16.0, 0.0], [std::f32::consts::FRAC_PI_2, 0.0, 0.0]);
        let moved = pose.transform_point3(Vec3::new(0.0, 0.0, 1.0));
        assert!((moved - Vec3::new(0.0, 0.0, 0.0)).length() < 1e-5);
        let turned = part_pose([0.0; 3], [0.0, 0.0, std::f32::consts::FRAC_PI_2])
            * part_pose([0.0; 3], [0.0, std::f32::consts::FRAC_PI_2, 0.0]);
        let combined = part_pose(
            [0.0; 3],
            [
                0.0,
                std::f32::consts::FRAC_PI_2,
                std::f32::consts::FRAC_PI_2,
            ],
        );
        let point = Vec3::new(1.0, 2.0, 3.0);
        assert!(
            (turned.transform_point3(point) - combined.transform_point3(point)).length() < 1e-5
        );
    }
}
