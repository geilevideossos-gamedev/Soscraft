use crate::{
    client_mobs::ClientMobs,
    cow_render::cube_scaled,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh, Vertex},
    pack::ResourceId,
};
use glam::{DVec3, Quat, Vec2, Vec3};
use minecraftoss_entities::world::{FireballEntity, GhastEntity};
use minecraftoss_player::rng::LegacyRandom;

type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3]);

pub const SCALE: f32 = 4.5;
const BODY: Part = (
    [-8.0, -8.0, -8.0],
    [8.0, 8.0, 8.0],
    [0.0, 0.0],
    [0.0, 17.6, 0.0],
);
const TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];
pub const FIRE_CHARGE: &str = "minecraft:item/fire_charge";

pub fn tentacles() -> Vec<Part> {
    let mut random = LegacyRandom::new(1660);
    (0..9)
        .map(|i: i32| {
            let x = (((i % 3) as f32 - (i / 3 % 2) as f32 * 0.5 + 0.25) / 2.0 * 2.0 - 1.0) * 5.0;
            let z = ((i / 3) as f32 / 2.0 * 2.0 - 1.0) * 5.0;
            let length = (random.next_int(7) + 8) as f32;
            (
                [-1.0, 0.0, -1.0],
                [1.0, length, 1.0],
                [0.0, 0.0],
                [x, 24.6, z],
            )
        })
        .collect()
}

pub fn tentacle_swing(age_in_ticks: f32, index: usize) -> f32 {
    0.2 * minecraftoss_player::mth::sin(f64::from(age_in_ticks * 0.3 + index as f32)) + 0.4
}

pub fn append_ghasts<'a>(
    mesh: &mut ChunkMesh,
    ghasts: impl IntoIterator<Item = &'a GhastEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let calm = atlas.entity_region(&ResourceId::parse("minecraft:entity/ghast/ghast").unwrap());
    let shooting =
        atlas.entity_region(&ResourceId::parse("minecraft:entity/ghast/ghast_shooting").unwrap());
    let partial = partial.clamp(0.0, 1.0);
    let tentacles = tentacles();
    let mut marks = Vec::new();
    for entity in ghasts {
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let sample = mob.light_block();
        let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
        let rotation = mob.body_rotation(90.0);
        let region = if entity.ghast.charging {
            shooting
        } else {
            calm
        };
        let scale = Vec3::splat(SCALE);
        let mut parts = vec![(BODY, Quat::IDENTITY)];
        for (index, &tentacle) in tentacles.iter().enumerate() {
            parts.push((
                tentacle,
                Quat::from_rotation_x(tentacle_swing(mob.age_in_ticks, index)),
            ));
        }
        for ((from, to, uv, pivot), pose) in parts {
            cube_scaled(
                mesh,
                mob.feet,
                rotation,
                scale,
                region,
                sky,
                block,
                from,
                to,
                uv,
                pivot,
                pose,
                [1.0; 3],
                TEXTURE_SIZE,
                None,
                false,
            );
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

pub fn fireball_scale(entity: &FireballEntity) -> f32 {
    match entity.fireball.kind {
        minecraftoss_entities::fireball::FireballKind::Large { .. } => 3.0,
    }
}

pub fn append_fireballs<'a>(
    mesh: &mut ChunkMesh,
    fireballs: impl IntoIterator<Item = (&'a FireballEntity, DVec3)>,
    atlas: &Atlas,
    forward: Vec3,
) {
    let Ok(sprite) = ResourceId::parse(FIRE_CHARGE) else {
        return;
    };
    if !atlas.contains(&sprite) {
        return;
    }
    let [u0, v0, u1, v1] = atlas.region(&sprite);
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    for (entity, at) in fireballs {
        let scale = fireball_scale(entity);
        let center = at.as_vec3() + up * 0.125 * scale;
        let half = 0.25 * scale;
        let start = mesh.vertices.len() as u32;
        for (corner, uv) in [
            (Vec2::new(-half, -half), [u0, v1]),
            (Vec2::new(half, -half), [u1, v1]),
            (Vec2::new(half, half), [u1, v0]),
            (Vec2::new(-half, half), [u0, v0]),
        ] {
            let point = center + right * corner.x + up * corner.y;
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                uv,
                color: [1.0; 4],
                sky_light: 15.0,
                block_light: 15.0,
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

    #[test]
    fn a_ghast_is_a_big_cube_over_nine_tentacles_of_fixed_lengths_that_sway() {
        let parts = tentacles();
        assert_eq!(parts.len(), 9);
        let pivots: Vec<[f32; 3]> = parts.iter().map(|p| p.3).collect();
        assert_eq!(pivots[0], [-3.75, 24.6, -5.0]);
        assert_eq!(pivots[4], [-1.25, 24.6, 0.0]);
        assert_eq!(pivots[8], [6.25, 24.6, 5.0]);
        assert!(parts.iter().all(|p| (8.0..=14.0).contains(&p.1[1])));
        assert_eq!(parts, tentacles(), "the lengths come from a fixed seed");
        assert!((tentacle_swing(0.0, 0) - 0.4).abs() < 1e-6);
        let swings: Vec<f32> = (0..40).map(|t| tentacle_swing(t as f32, 3)).collect();
        assert!(
            swings
                .iter()
                .all(|&x| (0.2 - 1e-6..=0.6 + 1e-6).contains(&x))
        );
        assert!(swings.iter().any(|&x| x > 0.55) && swings.iter().any(|&x| x < 0.25));
        let mut mesh = ChunkMesh::default();
        for (from, to, uv, pivot) in std::iter::once(BODY).chain(parts) {
            cube_scaled(
                &mut mesh,
                DVec3::ZERO,
                Quat::IDENTITY,
                Vec3::splat(SCALE),
                [0.0, 0.0, 1.0, 1.0],
                15.0,
                15.0,
                from,
                to,
                uv,
                pivot,
                Quat::IDENTITY,
                [1.0; 3],
                TEXTURE_SIZE,
                None,
                false,
            );
        }
        assert_eq!(mesh.vertices.len(), 10 * 24);
        let top = mesh
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max);
        assert!((top - (24.016 - 9.6) * SCALE / 16.0).abs() < 1e-3, "{top}");
    }
}
