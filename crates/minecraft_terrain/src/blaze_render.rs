use crate::{
    client_mobs::ClientMobs,
    cow_render::cube_scaled,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::{EulerRot, Quat, Vec3};
use minecraftoss_entities::world::BlazeEntity;
use std::f32::consts::{FRAC_PI_2, PI};

type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3]);

const HEAD: Part = ([-4.0, -4.0, -4.0], [4.0, 4.0, 4.0], [0.0, 0.0], [0.0; 3]);
const TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];

fn cos(x: f32) -> f32 {
    minecraftoss_player::mth::cos(f64::from(x))
}

fn sin(x: f32) -> f32 {
    minecraftoss_player::mth::sin(f64::from(x))
}

pub fn rods(age_in_ticks: f32) -> Vec<Part> {
    let mut out = Vec::with_capacity(12);
    let mut angle = age_in_ticks * PI * -0.1;
    for i in 0..4 {
        let y = -2.0 + cos(((i * 2) as f32 + age_in_ticks) * 0.25);
        out.push(rod([cos(angle) * 9.0, y, sin(angle) * 9.0]));
        angle += FRAC_PI_2;
    }
    let mut angle = 0.7853982 + age_in_ticks * PI * 0.03;
    for i in 4..8 {
        let y = 2.0 + cos(((i * 2) as f32 + age_in_ticks) * 0.25);
        out.push(rod([cos(angle) * 7.0, y, sin(angle) * 7.0]));
        angle += FRAC_PI_2;
    }
    let mut angle = 0.47123894 + age_in_ticks * PI * -0.05;
    for i in 8..12 {
        let y = 11.0 + cos((i as f32 * 1.5 + age_in_ticks) * 0.5);
        out.push(rod([cos(angle) * 5.0, y, sin(angle) * 5.0]));
        angle += FRAC_PI_2;
    }
    out
}

fn rod(pivot: [f32; 3]) -> Part {
    ([0.0, 0.0, 0.0], [2.0, 8.0, 2.0], [0.0, 16.0], pivot)
}

pub fn append_blazes<'a>(
    mesh: &mut ChunkMesh,
    blazes: impl IntoIterator<Item = &'a BlazeEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let region = atlas.entity_region(&ResourceId::parse("minecraft:entity/blaze/blaze").unwrap());
    let partial = partial.clamp(0.0, 1.0);
    let mut marks = Vec::new();
    for entity in blazes {
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let sky = light.get(mob.light_block()) as f32;
        let rotation = mob.body_rotation(90.0);
        let head = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            mob.head_yaw.to_radians(),
            mob.head_pitch.to_radians(),
        );
        let mut parts = vec![(HEAD, head)];
        parts.extend(
            rods(mob.age_in_ticks)
                .into_iter()
                .map(|part| (part, Quat::IDENTITY)),
        );
        for ((from, to, uv, pivot), pose) in parts {
            cube_scaled(
                mesh,
                mob.feet,
                rotation,
                Vec3::ONE,
                region,
                sky,
                15.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blaze_is_a_head_over_three_rings_of_rods_that_turn_and_bob() {
        let still = rods(0.0);
        assert_eq!(still.len(), 12);
        assert!((still[0].3[0] - 9.0).abs() < 1e-4 && still[0].3[2].abs() < 1e-4);
        assert!((still[0].3[1] + 1.0).abs() < 1e-4, "{}", still[0].3[1]);
        assert!((still[4].3[0] - 7.0 * 0.7853982_f32.cos()).abs() < 1e-3);
        assert!((still[8].3[1] - (11.0 + (12.0_f32 * 0.5).cos())).abs() < 1e-3);
        let later = rods(10.0);
        assert!(
            still.iter().zip(&later).all(|(a, b)| a.3 != b.3),
            "every rod moves"
        );
        assert!(
            later
                .iter()
                .take(4)
                .all(|r| (r.3[0].hypot(r.3[2]) - 9.0).abs() < 1e-3)
        );
        let mut mesh = ChunkMesh::default();
        for (from, to, uv, pivot) in std::iter::once(HEAD).chain(still) {
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
                TEXTURE_SIZE,
                None,
                false,
            );
        }
        assert_eq!(mesh.vertices.len(), 13 * 24);
        assert!(mesh.vertices.iter().all(|v| v.block_light == 15.0));
    }
}
