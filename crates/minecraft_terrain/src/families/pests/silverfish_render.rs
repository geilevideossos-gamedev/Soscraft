use crate::client_mobs::{ClientMobs, MobPose};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use std::f32::consts::PI;

pub const TEXTURE: &str = "minecraft:entity/silverfish/silverfish";
pub const TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];
pub const SHADOW_RADIUS: f32 = 0.3;
pub const FLIP_DEGREES: f32 = 180.0;

const SIZES: [[f32; 3]; 7] = [
    [3.0, 2.0, 2.0],
    [4.0, 3.0, 2.0],
    [6.0, 4.0, 3.0],
    [3.0, 3.0, 3.0],
    [2.0, 2.0, 3.0],
    [2.0, 1.0, 2.0],
    [1.0, 1.0, 2.0],
];
const UVS: [[f32; 2]; 7] = [
    [0.0, 0.0],
    [0.0, 4.0],
    [0.0, 9.0],
    [0.0, 16.0],
    [0.0, 22.0],
    [11.0, 0.0],
    [13.0, 4.0],
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub cube: Cube,
    pub y: f32,
    pub segment: usize,
    pub sways: bool,
}

pub const LAYERS: [Layer; 3] = [
    Layer {
        cube: Cube::new([-5.0, 0.0, -1.5], [10.0, 8.0, 3.0], [20.0, 0.0]),
        y: 16.0,
        segment: 2,
        sways: false,
    },
    Layer {
        cube: Cube::new([-3.0, 0.0, -1.5], [6.0, 4.0, 3.0], [20.0, 11.0]),
        y: 20.0,
        segment: 4,
        sways: true,
    },
    Layer {
        cube: Cube::new([-3.0, 0.0, -1.5], [6.0, 5.0, 2.0], [20.0, 18.0]),
        y: 19.0,
        segment: 1,
        sways: true,
    },
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Segment {
    pub x: f32,
    pub y_rot: f32,
}

pub fn placements<const N: usize>(sizes: &[[f32; 3]; N]) -> [f32; N] {
    let mut z = -3.5;
    std::array::from_fn(|i| {
        let at = z;
        if let Some(next) = sizes.get(i + 1) {
            z += (sizes[i][2] + next[2]) * 0.5;
        }
        at
    })
}

pub fn sway<const N: usize>(age_in_ticks: f32, turn: f32, shift: f32) -> [Segment; N] {
    std::array::from_fn(|i| {
        let angle = age_in_ticks * 0.9 + i as f32 * 0.15 * PI;
        let from_middle = (i as i32 - 2).abs();
        Segment {
            x: sin(angle) * PI * shift * from_middle as f32,
            y_rot: cos(angle) * PI * turn * (1 + from_middle) as f32,
        }
    })
}

pub fn wiggle(age_in_ticks: f32) -> [Segment; 7] {
    sway(age_in_ticks, 0.05, 0.2)
}

pub fn segment_cubes<const N: usize>(
    sizes: &[[f32; 3]; N],
    uvs: &[[f32; 2]; N],
    segments: &[Segment; N],
) -> [(Mat4, Cube); N] {
    let z = placements(sizes);
    std::array::from_fn(|i| {
        let [width, height, depth] = sizes[i];
        let cube = Cube::new([width * -0.5, 0.0, depth * -0.5], sizes[i], uvs[i]);
        let pose = part_pose(
            [segments[i].x, 24.0 - height, z[i]],
            [0.0, segments[i].y_rot, 0.0],
        );
        (pose, cube)
    })
}

pub fn cubes(segments: &[Segment; 7]) -> Vec<(Mat4, Cube)> {
    let z = placements(&SIZES);
    let mut out = segment_cubes(&SIZES, &UVS, segments).to_vec();
    for layer in LAYERS {
        let follows = segments[layer.segment];
        let x = if layer.sways { follows.x } else { 0.0 };
        let pose = part_pose([x, layer.y, z[layer.segment]], [0.0, follows.y_rot, 0.0]);
        out.push((pose, layer.cube));
    }
    out
}

pub fn model_root(pose: &MobPose, flip_degrees: f32) -> Mat4 {
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.body_rotation(flip_degrees))
        * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn append_each(
    mesh: &mut ChunkMesh,
    ids: impl IntoIterator<Item = u64>,
    poses: &ClientMobs,
    partial: f32,
    mut each: impl FnMut(&mut ChunkMesh, &MobPose),
) {
    let mut marks = Vec::new();
    for id in ids {
        let Some(mob) = poses.pose(id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        each(mesh, &mob);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

pub fn skin(atlas: &Atlas, texture: &str, light: &SkyLight, mob: &MobPose) -> Skin {
    let position = mob.light_block();
    Skin {
        region: ResourceId::parse(texture).map_or([0.0; 4], |id| atlas.entity_region(&id)),
        texture_size: TEXTURE_SIZE,
        sky: light.get(position) as f32,
        block: light.get_block(position) as f32,
        alpha: 1.0,
        shaded: true,
    }
}

pub fn append_silverfish(
    mesh: &mut ChunkMesh,
    ids: impl IntoIterator<Item = u64>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    append_each(mesh, ids, poses, partial, |mesh, mob| {
        let skin = skin(atlas, TEXTURE, light, mob);
        let root = model_root(mob, FLIP_DEGREES);
        for (pose, cube) in cubes(&wiggle(mob.age_in_ticks)) {
            append(mesh, root * pose, &skin, &cube);
        }
    });
}

fn sin(value: f32) -> f32 {
    minecraftoss_player::mth::sin(f64::from(value))
}

fn cos(value: f32) -> f32 {
    minecraftoss_player::mth::cos(f64::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use glam::DVec3;
    use minecraftoss_entities::world::EntityWorld;

    const HAND: [[(f32, f32); 7]; 2] = [
        [
            (0.471_239, 0.0),
            (0.279_918, 0.285_251),
            (0.092_329, 0.0),
            (0.049_145, 0.620_583),
            (-0.145_621, 1.195_133),
            (-0.444_288, 1.332_865),
            (-0.746_958, 0.776_644),
        ],
        [
            (-0.296_020, 0.977_756),
            (-0.286_810, 0.256_406),
            (-0.156_876, 0.0),
            (-0.272_301, -0.313_356),
            (-0.257_238, -1.052_894),
            (-0.066_599, -1.874_337),
            (0.280_380, -2.347_669),
        ],
    ];

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let skins = root.join("assets/minecraft/textures/entity/silverfish");
        std::fs::create_dir_all(&skins).unwrap();
        image::RgbaImage::from_pixel(64, 32, image::Rgba([90, 90, 90, 255]))
            .save(skins.join("silverfish.png"))
            .unwrap();
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let ids = vec![ResourceId::parse(TEXTURE).unwrap()];
        crate::mesh::make_atlas(&packs, ids).unwrap()
    }

    fn light() -> SkyLight {
        SkyLight::from_levels((-8, 56, -8), (17, 17, 17), vec![15; 4913], vec![3; 4913])
    }

    fn drawn(world: &EntityWorld, poses: &ClientMobs, atlas: &Atlas) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();
        let ids = world.family_cores().into_iter().map(|(_, core)| core.id);
        append_silverfish(&mut mesh, ids, poses, atlas, &light(), 0.0);
        mesh
    }

    fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
    }

    #[test]
    fn the_seven_segments_line_up_nose_to_tail() {
        assert_eq!(placements(&SIZES), [-3.5, -1.5, 1.0, 4.0, 7.0, 9.5, 11.5]);
        let still = cubes(&[Segment::default(); 7]);
        assert_eq!(still.len(), 10, "seven segments and three layers");
        let middle = still[2].0.transform_point3(Vec3::ZERO) * 16.0;
        assert!(
            middle.abs_diff_eq(Vec3::new(0.0, 20.0, 1.0), 1e-4),
            "{middle}"
        );
        assert_eq!(
            still[2].1,
            Cube::new([-3.0, 0.0, -1.5], [6.0, 4.0, 3.0], [0.0, 9.0])
        );
        assert_eq!(
            still[6].1,
            Cube::new([-0.5, 0.0, -1.0], [1.0, 1.0, 2.0], [13.0, 4.0])
        );
        let tail = still[6].0.transform_point3(Vec3::ZERO) * 16.0;
        assert!(tail.abs_diff_eq(Vec3::new(0.0, 23.0, 11.5), 1e-4), "{tail}");
    }

    #[test]
    fn the_three_fur_layers_ride_on_their_segments() {
        let segments = wiggle(7.3);
        let parts = cubes(&segments);
        let layer = |index: usize| parts[7 + index].0;
        let pivot = |pose: Mat4| pose.transform_point3(Vec3::ZERO) * 16.0;
        assert!(pivot(layer(0)).abs_diff_eq(Vec3::new(0.0, 16.0, 1.0), 1e-4));
        let tail = segments[4];
        assert!(pivot(layer(1)).abs_diff_eq(Vec3::new(tail.x, 20.0, 7.0), 1e-4));
        let neck = segments[1];
        assert!(pivot(layer(2)).abs_diff_eq(Vec3::new(neck.x, 19.0, -1.5), 1e-4));
        for (index, segment) in [(0, 2), (1, 4), (2, 1)] {
            let along = layer(index).transform_vector3(Vec3::Z);
            let turned = parts[segment].0.transform_vector3(Vec3::Z);
            assert!(
                along.abs_diff_eq(turned, 1e-5),
                "layer {index} turns with segment {segment}"
            );
        }
        assert_eq!(parts[7].1.size, [10.0, 8.0, 3.0]);
        assert_eq!((parts[8].1.uv, parts[9].1.uv), ([20.0, 11.0], [20.0, 18.0]));
        assert_eq!(
            parts[9].1.size,
            [6.0, 5.0, 2.0],
            "vanilla's third layer is two deep"
        );
    }

    #[test]
    fn the_wiggle_matches_hand_computed_angles() {
        for (age, expected) in [(0.0, HAND[0]), (2.5, HAND[1])] {
            for (i, (segment, (y_rot, x))) in wiggle(age).iter().zip(expected).enumerate() {
                assert!(
                    (segment.y_rot - y_rot).abs() < 1e-3,
                    "{age} {i}: {segment:?}"
                );
                assert!((segment.x - x).abs() < 1e-3, "{age} {i}: {segment:?}");
            }
            assert_eq!(wiggle(age)[2].x, 0.0, "the middle segment never shifts");
        }
    }

    #[test]
    fn a_silverfish_is_drawn_low_and_long_with_its_skin() {
        let atlas = atlas();
        let mut world = EntityWorld::default();
        let id = world
            .spawn_family_kind("minecraft:silverfish", DVec3::new(0.5, 64.0, 0.5))
            .unwrap();
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(&world));
        let mesh = drawn(&world, &poses, &atlas);
        assert_eq!(mesh.faces, 60);
        let (low, high) = span(&mesh, 1);
        assert!(low >= 64.0 && high <= 64.0 + 0.502, "{low} {high}");
        assert!(
            high > 64.49,
            "the front fur layer is eight pixels tall: {high}"
        );
        let (x0, x1) = span(&mesh, 0);
        let (z0, z1) = span(&mesh, 2);
        assert!(
            (x1 - x0).max(z1 - z0) > 1.0,
            "seventeen pixels from nose to tail"
        );
        let region = atlas.entity_region(&ResourceId::parse(TEXTURE).unwrap());
        assert!(mesh.vertices.iter().all(|v| {
            (region[0]..=region[2]).contains(&v.uv[0]) && (region[1]..=region[3]).contains(&v.uv[1])
        }));
        assert!(
            mesh.vertices
                .iter()
                .all(|v| v.sky_light == 15.0 && v.block_light == 3.0)
        );
        assert!(
            mesh.vertices.iter().all(|v| v.color[3] > 0.0),
            "no red when well"
        );
        world.family_core_mut(id).unwrap().1.damage.dead = true;
        poses.receive(server_mobs(&world));
        for _ in 0..20 {
            poses.tick();
        }
        let dying = drawn(&world, &poses, &atlas);
        assert!(
            dying.vertices.iter().all(|v| v.color[3] < 0.0),
            "red while dying"
        );
        let (_, top) = span(&dying, 1);
        assert!(top <= 64.0 + 1e-3, "flipped over by 180 degrees: {top}");
    }
}
