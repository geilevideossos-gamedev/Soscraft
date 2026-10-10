use crate::end_crystal::CrystalView;
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{DVec3, Mat4, Quat, Vec3};

pub const TEXTURE: &str = "minecraft:entity/end_crystal/end_crystal";
pub const BEAM_TEXTURE: &str = "minecraft:entity/end_crystal/end_crystal_beam";
const TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];
const GLASS: Cube = Cube::new([-4.0, -4.0, -4.0], [8.0, 8.0, 8.0], [0.0, 0.0]);
const CORE: Cube = Cube::new([-4.0, -4.0, -4.0], [8.0, 8.0, 8.0], [32.0, 0.0]);
const BASE: Cube = Cube::new([-6.0, 0.0, -6.0], [12.0, 4.0, 12.0], [0.0, 16.0]);
const INNER_SCALE: f32 = 0.875;
const CORE_SCALE: f32 = 0.765625;
const BEAM_SIDES: usize = 8;
const BEAM_RADIUS: f32 = 0.75;
const BEAM_NEAR: f32 = 0.2;

pub fn bob(age: f32) -> f32 {
    let wave = minecraftoss_player::mth::sin(f64::from(age * 0.2)) / 2.0 + 0.5;
    (wave * wave + wave) * 0.4 - 1.4
}

fn tilt() -> Quat {
    let half = std::f32::consts::FRAC_PI_4.sin();
    Quat::from_axis_angle(Vec3::new(half, 0.0, half).normalize(), 60f32.to_radians())
}

pub fn glass_poses(age: f32) -> [Mat4; 3] {
    let spin = (age * 3.0).to_radians();
    let outer = part_pose([0.0, 24.0 + bob(age) * 16.0 / 2.0, 0.0], [0.0; 3])
        * Mat4::from_quat(Quat::from_rotation_y(spin) * tilt());
    let turn = Mat4::from_quat(tilt() * Quat::from_rotation_y(spin));
    let inner = outer * turn * Mat4::from_scale(Vec3::splat(INNER_SCALE));
    let core = inner * turn * Mat4::from_scale(Vec3::splat(CORE_SCALE));
    [outer, inner, core]
}

pub fn crystal_pose(position: DVec3) -> Mat4 {
    Mat4::from_translation(position.as_vec3())
        * Mat4::from_scale(Vec3::splat(2.0))
        * Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0))
}

fn light_at(light: &SkyLight, position: DVec3) -> (f32, f32) {
    let at = (
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    (light.get(at) as f32, light.get_block(at) as f32)
}

pub fn append_crystals<'a>(
    mesh: &mut ChunkMesh,
    crystals: impl IntoIterator<Item = &'a CrystalView>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let Ok(id) = ResourceId::parse(TEXTURE) else {
        return;
    };
    if !atlas.contains(&id) {
        return;
    }
    let region = atlas.entity_region(&id);
    let beam = ResourceId::parse(BEAM_TEXTURE)
        .ok()
        .filter(|beam| atlas.contains(beam))
        .map(|beam| atlas.entity_region(&beam));
    for crystal in crystals {
        let age = crystal.time as f32 + partial;
        let (sky, block) = light_at(light, crystal.position);
        let skin = Skin {
            region,
            texture_size: TEXTURE_SIZE,
            sky,
            block,
            alpha: 1.0,
            shaded: true,
        };
        let root = crystal_pose(crystal.position);
        if crystal.show_bottom {
            append(mesh, root, &skin, &BASE);
        }
        let [outer, inner, core] = glass_poses(age);
        append(mesh, root * outer, &skin, &GLASS);
        append(mesh, root * inner, &skin, &GLASS);
        append(mesh, root * core, &skin, &CORE);
        if let (Some((x, y, z)), Some(beam)) = (crystal.beam_target, beam) {
            let target = DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5);
            let offset = (target - crystal.position).as_vec3();
            let start = target.as_vec3();
            let toward = Vec3::new(-offset.x, -offset.y + bob(age), -offset.z);
            append_beam(mesh, start, toward, age, beam, (sky, block));
        }
    }
}

pub fn beam_pose(start: Vec3, offset: Vec3) -> Mat4 {
    let horizontal = (offset.x * offset.x + offset.z * offset.z).sqrt();
    let yaw =
        -(f64::from(offset.z).atan2(f64::from(offset.x))) as f32 - std::f32::consts::FRAC_PI_2;
    let pitch =
        -(f64::from(horizontal).atan2(f64::from(offset.y))) as f32 - std::f32::consts::FRAC_PI_2;
    Mat4::from_translation(start + Vec3::new(0.0, 2.0, 0.0))
        * Mat4::from_rotation_y(yaw)
        * Mat4::from_rotation_x(pitch)
}

pub fn beam_v(length: f32, time: f32) -> (f32, f32) {
    let near = -time * 0.01;
    (near, length / 32.0 - time * 0.01)
}

pub fn beam_spans(v: (f32, f32)) -> Vec<(f32, f32)> {
    let (near, far) = v;
    if far <= near {
        return vec![(0.0, 1.0)];
    }
    let mut cuts = vec![0.0];
    let mut edge = near.floor() + 1.0;
    while edge < far {
        cuts.push((edge - near) / (far - near));
        edge += 1.0;
    }
    cuts.push(1.0);
    cuts.windows(2).map(|pair| (pair[0], pair[1])).collect()
}

pub fn append_beam(
    mesh: &mut ChunkMesh,
    start: Vec3,
    offset: Vec3,
    time: f32,
    region: [f32; 4],
    (sky, block): (f32, f32),
) {
    let length = offset.length();
    let pose = beam_pose(start, offset);
    let (near, far) = beam_v(length, time);
    let ring = |i: usize| {
        let angle = i as f32 * std::f32::consts::TAU / BEAM_SIDES as f32;
        (
            minecraftoss_player::mth::sin(f64::from(angle)) * BEAM_RADIUS,
            minecraftoss_player::mth::cos(f64::from(angle)) * BEAM_RADIUS,
        )
    };
    for (s0, s1) in beam_spans((near, far)) {
        let v_at = |s: f32| near + (far - near) * s;
        let floor = v_at((s0 + s1) / 2.0).floor();
        let at = |s: f32, (x, y): (f32, f32), u: f32| {
            let scale = BEAM_NEAR + (1.0 - BEAM_NEAR) * s;
            let point = pose.transform_point3(Vec3::new(x * scale, y * scale, length * s));
            let v = v_at(s) - floor;
            Vertex {
                position: point.to_array(),
                uv: [
                    region[0] + (region[2] - region[0]) * u,
                    region[1] + (region[3] - region[1]) * v.clamp(0.0, 1.0),
                ],
                color: [s, s, s, 1.0],
                sky_light: sky,
                block_light: block,
            }
        };
        for i in 1..=BEAM_SIDES {
            let (u0, u1) = ((i - 1) as f32 / 8.0, i as f32 / 8.0);
            let start = mesh.vertices.len() as u32;
            mesh.vertices.extend([
                at(s0, ring(i - 1), u0),
                at(s1, ring(i - 1), u0),
                at(s1, ring(i), u1),
                at(s0, ring(i), u1),
            ]);
            mesh.indices.extend_from_slice(&[
                start,
                start + 1,
                start + 2,
                start,
                start + 2,
                start + 3,
            ]);
            mesh.faces += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crystal_bobs_between_vanilla_heights() {
        assert!((bob(0.0) - (0.75 * 0.4 - 1.4)).abs() < 1e-6);
        let heights: Vec<f32> = (0..200).map(|t| bob(t as f32)).collect();
        assert!(
            heights
                .iter()
                .all(|&y| (-1.4 - 1e-6..=-0.6 + 1e-6).contains(&y))
        );
        assert!(heights.iter().any(|&y| y > -0.65) && heights.iter().any(|&y| y < -1.35));
    }

    #[test]
    fn the_crystal_is_a_base_and_three_turning_cubes_about_a_block_above_its_feet() {
        let feet = DVec3::new(10.5, 70.0, -3.5);
        let [outer, inner, core] = glass_poses(0.0);
        let root = crystal_pose(feet);
        let centre = (root * outer).transform_point3(Vec3::ZERO);
        let lift = (bob(0.0) * 8.0 + 24.0) / 16.0 * 2.0 - 1.0;
        assert!((centre - Vec3::new(10.5, 70.0 + lift, -3.5)).length() < 1e-4);
        assert!(((root * inner).transform_point3(Vec3::ZERO) - centre).length() < 1e-4);
        assert!(((root * core).transform_point3(Vec3::ZERO) - centre).length() < 1e-4);
        let size = |pose: Mat4| (pose.transform_vector3(Vec3::X)).length();
        assert!((size(root * inner) / size(root * outer) - INNER_SCALE).abs() < 1e-5);
        assert!((size(root * core) / size(root * outer) - INNER_SCALE * CORE_SCALE).abs() < 1e-5);
        let mut mesh = ChunkMesh::default();
        let skin = Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: TEXTURE_SIZE,
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        };
        append(&mut mesh, root, &skin, &BASE);
        let low = mesh
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MAX, f32::min);
        let high = mesh
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max);
        assert!(
            (low - 69.0).abs() < 1e-4 && (high - 69.5).abs() < 1e-4,
            "{low} {high}"
        );
    }

    #[test]
    fn a_beam_runs_from_two_above_its_start_to_two_above_the_far_end() {
        let start = Vec3::new(0.0, 100.0, 0.0);
        for offset in [
            Vec3::new(10.0, -5.0, 3.0),
            Vec3::new(0.0, 0.0, -7.0),
            Vec3::new(-4.0, 20.0, 0.5),
        ] {
            let pose = beam_pose(start, offset);
            let far = pose.transform_point3(Vec3::new(0.0, 0.0, offset.length()));
            let expected = start + Vec3::new(0.0, 2.0, 0.0) + offset;
            assert!((far - expected).length() < 1e-3, "{offset:?}: {far:?}");
        }
    }

    #[test]
    fn a_long_beam_is_cut_where_its_texture_repeats() {
        assert_eq!(beam_v(64.0, 0.0), (0.0, 2.0));
        assert_eq!(beam_spans((0.0, 2.0)), vec![(0.0, 0.5), (0.5, 1.0)]);
        assert_eq!(beam_spans((0.25, 0.75)), vec![(0.0, 1.0)]);
        let spans = beam_spans((-0.5, 1.0));
        assert_eq!(spans.len(), 2);
        assert!((spans[0].1 - 1.0 / 3.0).abs() < 1e-6);
        let mut mesh = ChunkMesh::default();
        append_beam(
            &mut mesh,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 64.0),
            0.0,
            [0.0, 0.0, 1.0, 1.0],
            (15.0, 15.0),
        );
        assert_eq!(mesh.faces, 16);
        assert!(mesh.vertices.iter().all(|v| (0.0..=1.0).contains(&v.uv[1])));
        let first = &mesh.vertices[0];
        assert_eq!(first.color, [0.0, 0.0, 0.0, 1.0], "black at the start");
        assert!(
            mesh.vertices
                .iter()
                .any(|v| v.color == [1.0, 1.0, 1.0, 1.0])
        );
    }
}
