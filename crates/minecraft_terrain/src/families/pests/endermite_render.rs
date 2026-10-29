use super::silverfish_render::{Segment, append_each, model_root, segment_cubes, skin, sway};
use crate::client_mobs::ClientMobs;
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::posed_cube::{Cube, append};
use glam::{DVec3, Mat4};
use minecraftoss_entities::world::families::pests::endermites::{HEIGHT, WIDTH};
use minecraftoss_player::rng::LegacyRandom;

pub const TEXTURE: &str = "minecraft:entity/endermite/endermite";
pub const SHADOW_RADIUS: f32 = 0.3;
pub const FLIP_DEGREES: f32 = 180.0;

const SIZES: [[f32; 3]; 4] = [
    [4.0, 3.0, 2.0],
    [6.0, 4.0, 5.0],
    [3.0, 3.0, 1.0],
    [1.0, 2.0, 1.0],
];
const UVS: [[f32; 2]; 4] = [[0.0, 0.0], [0.0, 5.0], [0.0, 14.0], [0.0, 18.0]];

pub fn wiggle(age_in_ticks: f32) -> [Segment; 4] {
    sway(age_in_ticks, 0.01, 0.1)
}

pub fn cubes(segments: &[Segment; 4]) -> [(Mat4, Cube); 4] {
    segment_cubes(&SIZES, &UVS, segments)
}

pub fn append_endermites(
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

pub fn portal_sparks(feet: DVec3, random: &mut LegacyRandom) -> [(DVec3, DVec3); 2] {
    let (width, height) = (f64::from(WIDTH), f64::from(HEIGHT));
    std::array::from_fn(|_| {
        let x = feet.x + width * ((2.0 * random.next_double() - 1.0) * 0.5);
        let y = feet.y + height * random.next_double();
        let z = feet.z + width * ((2.0 * random.next_double() - 1.0) * 0.5);
        let motion = DVec3::new(
            (random.next_double() - 0.5) * 2.0,
            -random.next_double(),
            (random.next_double() - 0.5) * 2.0,
        );
        (DVec3::new(x, y, z), motion)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use crate::pack::ResourceId;
    use glam::Vec3;
    use minecraftoss_entities::world::EntityWorld;

    const HAND: [[(f32, f32); 4]; 2] = [
        [
            (0.094_248, 0.0),
            (0.055_984, 0.142_625),
            (0.018_466, 0.0),
            (0.009_829, 0.310_291),
        ],
        [
            (-0.059_204, 0.488_878),
            (-0.057_362, 0.128_203),
            (-0.031_375, 0.0),
            (-0.054_460, -0.156_678),
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
        let skins = root.join("assets/minecraft/textures/entity/endermite");
        std::fs::create_dir_all(&skins).unwrap();
        image::RgbaImage::from_pixel(64, 32, image::Rgba([60, 20, 80, 255]))
            .save(skins.join("endermite.png"))
            .unwrap();
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let ids = vec![ResourceId::parse(TEXTURE).unwrap()];
        crate::mesh::make_atlas(&packs, ids).unwrap()
    }

    #[test]
    fn the_four_segments_line_up_nose_to_tail() {
        let still = cubes(&[Segment::default(); 4]);
        let pivots: Vec<Vec3> = still
            .iter()
            .map(|(pose, _)| pose.transform_point3(Vec3::ZERO) * 16.0)
            .collect();
        let expected = [
            Vec3::new(0.0, 21.0, -3.5),
            Vec3::new(0.0, 20.0, 0.0),
            Vec3::new(0.0, 21.0, 3.0),
            Vec3::new(0.0, 22.0, 4.0),
        ];
        for (pivot, expected) in pivots.iter().zip(expected) {
            assert!(pivot.abs_diff_eq(expected, 1e-4), "{pivot} {expected}");
        }
        assert_eq!(
            still[1].1,
            Cube::new([-3.0, 0.0, -2.5], [6.0, 4.0, 5.0], [0.0, 5.0])
        );
        assert_eq!(
            still[3].1,
            Cube::new([-0.5, 0.0, -0.5], [1.0, 2.0, 1.0], [0.0, 18.0])
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
        }
        let fish = super::super::silverfish_render::wiggle(4.0);
        let mite = wiggle(4.0);
        assert!(
            mite[0].y_rot.abs() < fish[0].y_rot.abs(),
            "a mite sways a fifth as far"
        );
    }

    #[test]
    fn an_endermite_is_drawn_as_four_purple_segments() {
        let atlas = atlas();
        let mut world = EntityWorld::default();
        let id = world
            .spawn_family_kind("minecraft:endermite", DVec3::new(0.5, 64.0, 0.5))
            .unwrap();
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(&world));
        let light =
            SkyLight::from_levels((-8, 56, -8), (17, 17, 17), vec![15; 4913], vec![0; 4913]);
        let mut mesh = ChunkMesh::default();
        append_endermites(&mut mesh, [id], &poses, &atlas, &light, 0.0);
        assert_eq!(mesh.faces, 24);
        let (low, high) = mesh
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold((f32::MAX, f32::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)));
        assert!(low >= 64.0 && (high - 64.251).abs() < 1e-4, "{low} {high}");
        let region = atlas.entity_region(&ResourceId::parse(TEXTURE).unwrap());
        assert!(mesh.vertices.iter().all(|v| {
            (region[0]..=region[2]).contains(&v.uv[0]) && (region[1]..=region[3]).contains(&v.uv[1])
        }));
        let mut none = ChunkMesh::default();
        append_endermites(&mut none, [id + 1], &poses, &atlas, &light, 0.0);
        assert!(none.vertices.is_empty(), "a mob the client has not seen");
    }

    #[test]
    fn portal_sparks_start_inside_the_box_and_drift_out_and_down() {
        let feet = DVec3::new(10.5, 64.0, -3.5);
        let mut random = LegacyRandom::new(42);
        let mut copy = LegacyRandom::new(42);
        for _ in 0..200 {
            for (start, motion) in portal_sparks(feet, &mut random) {
                let local = start - feet;
                assert!(
                    local.x.abs() <= 0.2 + 1e-6 && local.z.abs() <= 0.2 + 1e-6,
                    "{local}"
                );
                assert!((0.0..=f64::from(HEIGHT)).contains(&local.y), "{local}");
                assert!(motion.x.abs() <= 1.0 && motion.z.abs() <= 1.0, "{motion}");
                assert!((-1.0..=0.0).contains(&motion.y), "{motion}");
            }
        }
        let first = portal_sparks(feet, &mut copy)[0];
        let mut by_hand = LegacyRandom::new(42);
        let draws: Vec<f64> = (0..6).map(|_| by_hand.next_double()).collect();
        let width = f64::from(WIDTH);
        assert_eq!(
            first.0,
            DVec3::new(
                feet.x + width * ((2.0 * draws[0] - 1.0) * 0.5),
                feet.y + f64::from(HEIGHT) * draws[1],
                feet.z + width * ((2.0 * draws[2] - 1.0) * 0.5),
            )
        );
        assert_eq!(
            first.1,
            DVec3::new((draws[3] - 0.5) * 2.0, -draws[4], (draws[5] - 0.5) * 2.0)
        );
    }
}
