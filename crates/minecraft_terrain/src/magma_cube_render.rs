use crate::{
    client_mobs::ClientMobs,
    cow_render::cube_scaled,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::{Quat, Vec3};
use minecraftoss_entities::world::SlimeEntity;

type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3]);

pub fn parts(squish: f32) -> Vec<Part> {
    let spread = squish.max(0.0);
    let mut parts: Vec<Part> = (0..8)
        .map(|i| {
            let uv = match i {
                0 => [0.0, 0.0],
                1..=3 => [0.0, (9 * i) as f32],
                _ => [32.0, (9 * i - 36) as f32],
            };
            let y = (16 + i) as f32;
            let offset = (i - 4) as f32 * spread * 1.7;
            ([-4.0, y, -4.0], [4.0, y + 1.0, 4.0], uv, [0.0, offset, 0.0])
        })
        .collect();
    parts.push(([-2.0, 18.0, -2.0], [2.0, 22.0, 2.0], [24.0, 40.0], [0.0; 3]));
    parts
}

pub fn size_and_squish(size: i32, squish: f32) -> Vec3 {
    let size = size as f32;
    let ss = squish / (size * 0.5 + 1.0);
    let w = 1.0 / (ss + 1.0);
    Vec3::new(w * size, 1.0 / w * size, w * size)
}

pub fn append_magma_cubes<'a>(
    mesh: &mut ChunkMesh,
    cubes: impl IntoIterator<Item = &'a SlimeEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let region =
        atlas.entity_region(&ResourceId::parse("minecraft:entity/slime/magmacube").unwrap());
    let partial = partial.clamp(0.0, 1.0);
    let mut marks = Vec::new();
    for entity in cubes.into_iter().filter(|e| e.slime.magma) {
        let cube = &entity.slime;
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let sky = light.get(mob.light_block()) as f32;
        let rotation = mob.body_rotation(90.0);
        let squish = cube.previous_squish + (cube.squish - cube.previous_squish) * partial;
        let scale = size_and_squish(cube.size, squish);
        for (from, to, uv, pivot) in parts(squish) {
            cube_scaled(
                mesh,
                mob.feet,
                rotation,
                scale,
                region,
                sky,
                15.0,
                from,
                to,
                uv,
                pivot,
                Quat::IDENTITY,
                [1.0; 3],
                [64., 64.],
                None,
                false,
            );
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_magma_cube_is_eight_slices_round_a_core_that_spread_as_it_leaps() {
        let rest = parts(0.0);
        assert_eq!(rest.len(), 9);
        assert!(rest.iter().all(|part| part.3 == [0.0; 3]));
        let uvs: Vec<[f32; 2]> = rest.iter().map(|part| part.2).collect();
        assert_eq!(
            uvs[..8],
            [
                [0., 0.],
                [0., 9.],
                [0., 18.],
                [0., 27.],
                [32., 0.],
                [32., 9.],
                [32., 18.],
                [32., 27.]
            ]
        );
        let leaping = parts(1.0);
        assert_eq!(leaping[0].3[1], -4.0 * 1.7);
        assert_eq!(leaping[7].3[1], 3.0 * 1.7);
        assert_eq!(
            parts(-0.5)[0].3,
            [0.0; 3],
            "landing squashes without spreading"
        );
        let mut mesh = ChunkMesh::default();
        for (from, to, uv, pivot) in leaping {
            cube_scaled(
                &mut mesh,
                glam::DVec3::ZERO,
                Quat::IDENTITY,
                Vec3::ONE,
                [0.0, 0.0, 1.0, 1.0],
                15.0,
                15.0,
                from,
                to,
                uv,
                pivot,
                Quat::IDENTITY,
                [1.0; 3],
                [64., 64.],
                None,
                false,
            );
        }
        assert_eq!(mesh.vertices.len(), 9 * 24);
        assert_eq!(size_and_squish(4, 0.0), Vec3::splat(4.0));
    }
}
