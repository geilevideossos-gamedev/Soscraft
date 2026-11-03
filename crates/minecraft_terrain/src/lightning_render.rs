use crate::lightning::BoltView;
use crate::mesh::{Atlas, ChunkMesh, Vertex, WHITE_TEXTURE};
use crate::pack::ResourceId;
use glam::{DVec3, Vec3};
use minecraftoss_core::random::LegacyRandom;

pub const COLOR: [f32; 4] = [0.45, 0.45, 0.5, 0.3];
pub const RENDER_RANGE: f64 = 64.0;
pub const SEGMENT_HEIGHT: f32 = 16.0;
const SEGMENTS: usize = 8;
const LAYERS: i32 = 4;
const BRANCHES: usize = 3;
const BRANCH_SEGMENTS: usize = 3;
const SIDES: [(bool, bool, bool, bool); 4] = [
    (false, false, true, false),
    (true, false, true, true),
    (true, true, false, true),
    (false, true, false, false),
];

pub fn bolt_quads(seed: i64) -> Vec<[Vec3; 4]> {
    let mut xs = [0.0_f32; SEGMENTS];
    let mut zs = [0.0_f32; SEGMENTS];
    let (mut x, mut z) = (0.0_f32, 0.0_f32);
    let mut random = LegacyRandom::new(seed);
    for segment in (0..SEGMENTS).rev() {
        xs[segment] = x;
        zs[segment] = z;
        x += (random.next_i32_bound(11) - 5) as f32;
        z += (random.next_i32_bound(11) - 5) as f32;
    }
    let (tip_x, tip_z) = (x, z);
    let mut quads = Vec::with_capacity(224);
    for layer in 0..LAYERS {
        let mut random = LegacyRandom::new(seed);
        for branch in 0..BRANCHES {
            let trunk = branch == 0;
            let top = SEGMENTS - 1 - branch;
            let bottom = if trunk { 0 } else { top + 1 - BRANCH_SEGMENTS };
            let mut x = xs[top] - tip_x;
            let mut z = zs[top] - tip_z;
            for segment in (bottom..=top).rev() {
                let (above_x, above_z) = (x, z);
                let (bound, half) = if trunk { (11, 5) } else { (31, 15) };
                x += (random.next_i32_bound(bound) - half) as f32;
                z += (random.next_i32_bound(bound) - half) as f32;
                let width = 0.1 + layer as f32 * 0.2;
                let (mut above_width, mut below_width) = (width, width);
                if trunk {
                    above_width *= segment as f32 * 0.1 + 1.0;
                    below_width *= (segment as f32 - 1.0) * 0.1 + 1.0;
                }
                let below = segment as f32 * SEGMENT_HEIGHT;
                let above = below + SEGMENT_HEIGHT;
                let side = |positive: bool, width: f32| if positive { width } else { -width };
                for (a, b, c, d) in SIDES {
                    quads.push([
                        Vec3::new(x + side(a, below_width), below, z + side(b, below_width)),
                        Vec3::new(
                            above_x + side(a, above_width),
                            above,
                            above_z + side(b, above_width),
                        ),
                        Vec3::new(
                            above_x + side(c, above_width),
                            above,
                            above_z + side(d, above_width),
                        ),
                        Vec3::new(x + side(c, below_width), below, z + side(d, below_width)),
                    ]);
                }
            }
        }
    }
    quads
}

pub fn drawn(bolt: &BoltView, camera: DVec3) -> bool {
    bolt.position.distance_squared(camera) < RENDER_RANGE * RENDER_RANGE
}

pub fn append_bolts<'a>(
    mesh: &mut ChunkMesh,
    bolts: impl IntoIterator<Item = &'a BoltView>,
    atlas: &Atlas,
    camera: DVec3,
) {
    let Some(white) = ResourceId::parse(WHITE_TEXTURE)
        .ok()
        .filter(|white| atlas.contains(white))
        .map(|white| atlas.entity_region(&white))
    else {
        return;
    };
    let uv = [(white[0] + white[2]) / 2.0, (white[1] + white[3]) / 2.0];
    for bolt in bolts.into_iter().filter(|bolt| drawn(bolt, camera)) {
        for corners in bolt_quads(bolt.seed) {
            let start = mesh.vertices.len() as u32;
            for corner in corners {
                let at = bolt.position + corner.as_dvec3();
                mesh.vertices.push(Vertex {
                    position: at.as_vec3().to_array(),
                    uv,
                    color: COLOR,
                    sky_light: 15.0,
                    block_light: 15.0,
                });
            }
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

    fn atlas(white: bool) -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let ids = white
            .then(|| ResourceId::parse(WHITE_TEXTURE).unwrap())
            .into_iter()
            .collect();
        crate::mesh::make_atlas(&packs, ids).unwrap()
    }

    fn bolt(id: i32, position: DVec3, seed: i64) -> BoltView {
        BoltView {
            id,
            position,
            seed,
            life: 1,
        }
    }

    #[test]
    fn a_bolt_is_224_quads_128_blocks_tall_from_its_seed() {
        let quads = bolt_quads(42);
        assert_eq!(quads.len(), 224, "4 layers of 14 segments, 4 sides each");
        let corners: Vec<Vec3> = quads.iter().flatten().copied().collect();
        let low = corners.iter().map(|c| c.y).fold(f32::INFINITY, f32::min);
        let high = corners
            .iter()
            .map(|c| c.y)
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!((low, high), (0.0, 128.0));
        for c in corners.iter().filter(|c| c.y == 0.0) {
            assert!(
                c.x.abs() <= 0.63 + 1e-5 && c.z.abs() <= 0.63 + 1e-5,
                "the trunk lands on the bolt, tapered to 0.9 of its widest layer: {c:?}"
            );
        }
        let first = quads[0];
        let foot = (first[3] - first[0]).length();
        let head = (first[2] - first[1]).length();
        assert!(
            (foot - 0.32).abs() < 1e-5 && (head - 0.34).abs() < 1e-5,
            "the thinnest layer's top trunk segment tapers from 0.17 to 0.16 each side: {foot} {head}"
        );
        let centres: Vec<Vec3> = quads
            .chunks(4)
            .map(|sides| sides.iter().flatten().sum::<Vec3>() / 16.0)
            .collect();
        let layer = centres.len() / 4;
        for (thin, thick) in centres[..layer].iter().zip(&centres[3 * layer..]) {
            assert!(
                (*thin - *thick).length() < 1e-4,
                "every layer follows the same path: {thin} {thick}"
            );
        }
    }

    #[test]
    fn the_same_seed_draws_the_same_bolt() {
        assert_eq!(bolt_quads(7), bolt_quads(7));
        assert_ne!(bolt_quads(7), bolt_quads(8));
        let tips: Vec<Vec3> = [7, 8, 9]
            .map(|seed| bolt_quads(seed)[0][1])
            .into_iter()
            .collect();
        assert!(
            tips.windows(2).any(|pair| pair[0] != pair[1]),
            "the top of the trunk moves with the seed"
        );
    }

    #[test]
    fn bolts_are_drawn_white_and_unlit_within_sixty_four_blocks() {
        let white = atlas(true);
        let camera = DVec3::new(0.5, 64.0, 0.5);
        let near = bolt(1, DVec3::new(8.5, 64.0, 0.5), 3);
        let far = bolt(2, DVec3::new(70.5, 64.0, 0.5), 3);
        let mut mesh = ChunkMesh::default();
        append_bolts(&mut mesh, [&near, &far], &white, camera);
        assert_eq!(mesh.vertices.len(), 224 * 4, "only the near bolt");
        assert_eq!(mesh.indices.len(), 224 * 6);
        assert!(mesh.vertices.iter().all(|v| v.color == COLOR));
        let lowest = mesh
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::INFINITY, f32::min);
        assert_eq!(lowest, 64.0, "from the bolt's feet up");
        let offsets: Vec<Vec3> = bolt_quads(3).into_iter().flatten().collect();
        for (vertex, offset) in mesh.vertices.iter().zip(offsets) {
            let expected = (near.position + offset.as_dvec3()).as_vec3();
            assert_eq!(Vec3::from_array(vertex.position), expected);
        }
        let mut blank = ChunkMesh::default();
        append_bolts(&mut blank, [&near], &atlas(false), camera);
        assert!(blank.vertices.is_empty(), "no white texel, nothing drawn");
    }
}
