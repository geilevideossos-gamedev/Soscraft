use crate::mesh::{ChunkMesh, Vertex};

pub const GROW: f32 = 0.0625;
pub const GLOW_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub const FULL_BRIGHT: f32 = 15.0;

fn bounds(mesh: &ChunkMesh) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for vertex in &mesh.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    (min, max)
}

pub fn append_glowing(glow: &mut ChunkMesh, cover: &mut ChunkMesh, mob: &ChunkMesh) {
    if mob.vertices.is_empty() {
        return;
    }
    let (min, max) = bounds(mob);
    let center: [f32; 3] = std::array::from_fn(|axis| (min[axis] + max[axis]) / 2.0);
    let scale: [f32; 3] = std::array::from_fn(|axis| {
        let extent = (max[axis] - min[axis]).max(0.001);
        (extent + 2.0 * GROW) / extent
    });
    let base = glow.vertices.len() as u32;
    glow.vertices
        .extend(mob.vertices.iter().map(|vertex| Vertex {
            position: std::array::from_fn(|axis| {
                center[axis] + (vertex.position[axis] - center[axis]) * scale[axis]
            }),
            uv: vertex.uv,
            color: GLOW_COLOR,
            sky_light: FULL_BRIGHT,
            block_light: FULL_BRIGHT,
        }));
    glow.indices
        .extend(mob.indices.iter().map(|index| index + base));
    glow.faces += mob.faces;
    let base = cover.vertices.len() as u32;
    cover.vertices.extend_from_slice(&mob.vertices);
    cover
        .indices
        .extend(mob.indices.iter().map(|index| index + base));
    cover.faces += mob.faces;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(at: [f32; 3], size: f32) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();
        for corner in 0..8 {
            let offset = |bit: usize| if corner & (1 << bit) != 0 { size } else { 0.0 };
            mesh.vertices.push(Vertex {
                position: [at[0] + offset(0), at[1] + offset(1), at[2] + offset(2)],
                uv: [0.25, 0.5],
                color: [0.5, 0.5, 0.5, -0.4],
                sky_light: 7.0,
                block_light: 2.0,
            });
        }
        mesh.indices = vec![0, 1, 2, 2, 1, 3];
        mesh.faces = 1;
        mesh
    }

    #[test]
    fn a_glowing_mob_gets_a_white_hull_a_sixteenth_bigger_and_a_copy_that_masks_its_shape() {
        let mut glow = ChunkMesh::default();
        let mut cover = ChunkMesh::default();
        append_glowing(&mut glow, &mut cover, &cube([10.0, 64.0, 3.0], 1.0));
        append_glowing(&mut glow, &mut cover, &cube([20.0, 64.0, 3.0], 2.0));
        let (min, max) = bounds(&glow);
        assert_eq!(min, [10.0 - GROW, 64.0 - GROW, 3.0 - GROW]);
        assert_eq!(max, [22.0 + GROW, 66.0 + GROW, 5.0 + GROW]);
        assert!(
            glow.vertices
                .iter()
                .all(|v| v.color == GLOW_COLOR && v.sky_light == FULL_BRIGHT)
        );
        assert!(glow.vertices.iter().all(|v| v.uv == [0.25, 0.5]));
        assert_eq!(glow.indices[6..], [8, 9, 10, 10, 9, 11]);
        assert_eq!(
            cover.vertices[..8],
            cube([10.0, 64.0, 3.0], 1.0).vertices[..]
        );
        assert_eq!((glow.faces, cover.faces), (2, 2));
        let mut empty = ChunkMesh::default();
        append_glowing(&mut empty, &mut ChunkMesh::default(), &ChunkMesh::default());
        assert!(empty.vertices.is_empty());
    }
}
