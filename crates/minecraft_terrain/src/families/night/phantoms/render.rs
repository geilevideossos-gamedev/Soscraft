use crate::client_mobs::{ClientMobs, MobPose};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::night::phantoms::{self, Phantom};

pub const TEXTURE: &str = "minecraft:entity/phantom/phantom";
pub const EYES_TEXTURE: &str = "minecraft:entity/phantom/phantom_eyes";
pub const TEXTURES: [&str; 2] = [TEXTURE, EYES_TEXTURE];
pub const TEXTURE_SIZE: [f32; 2] = [64.0, 64.0];
pub const FLIP_DEGREES: f32 = 90.0;
pub const FLAP_DEGREES_PER_TICK: f32 = 7.448_451;
pub const FLAP_TICKS_PER_ID: i32 = 3;
pub const WING_DEGREES: f32 = 16.0;
pub const LIFT: [f32; 3] = [0.0, 1.3125, 0.1875];
pub const FULL_BRIGHT: f32 = 15.0;
pub const NO_OVERLAY: f32 = 1.0;
pub const CAGE_MARGIN: f32 = 0.3;
const DEGREES: f32 = 0.017_453_292;

pub const BODY: usize = 0;
pub const TAIL_BASE: usize = 1;
pub const TAIL_TIP: usize = 2;
pub const LEFT_WING_BASE: usize = 3;
pub const LEFT_WING_TIP: usize = 4;
pub const RIGHT_WING_BASE: usize = 5;
pub const RIGHT_WING_TIP: usize = 6;
pub const HEAD: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub name: &'static str,
    pub parent: Option<usize>,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub cube: Cube,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flap {
    pub wing: f32,
    pub tail: f32,
}

const fn part(
    name: &'static str,
    parent: Option<usize>,
    offset: [f32; 3],
    rotation: [f32; 3],
    cube: Cube,
) -> Part {
    Part {
        name,
        parent,
        offset,
        rotation,
        cube,
    }
}

pub const PARTS: [Part; 8] = [
    part(
        "body",
        None,
        [0.0; 3],
        [-0.1, 0.0, 0.0],
        Cube::new([-3.0, -2.0, -8.0], [5.0, 3.0, 9.0], [0.0, 8.0]),
    ),
    part(
        "tail_base",
        Some(BODY),
        [0.0, -2.0, 1.0],
        [0.0; 3],
        Cube::new([-2.0, 0.0, 0.0], [3.0, 2.0, 6.0], [3.0, 20.0]),
    ),
    part(
        "tail_tip",
        Some(TAIL_BASE),
        [0.0, 0.5, 6.0],
        [0.0; 3],
        Cube::new([-1.0, 0.0, 0.0], [1.0, 1.0, 6.0], [4.0, 29.0]),
    ),
    part(
        "left_wing_base",
        Some(BODY),
        [2.0, -2.0, -8.0],
        [0.0, 0.0, 0.1],
        Cube::new([0.0, 0.0, 0.0], [6.0, 2.0, 9.0], [23.0, 12.0]),
    ),
    part(
        "left_wing_tip",
        Some(LEFT_WING_BASE),
        [6.0, 0.0, 0.0],
        [0.0, 0.0, 0.1],
        Cube::new([0.0, 0.0, 0.0], [13.0, 1.0, 9.0], [16.0, 24.0]),
    ),
    part(
        "right_wing_base",
        Some(BODY),
        [-3.0, -2.0, -8.0],
        [0.0, 0.0, -0.1],
        Cube::new([-6.0, 0.0, 0.0], [6.0, 2.0, 9.0], [23.0, 12.0]).mirrored(),
    ),
    part(
        "right_wing_tip",
        Some(RIGHT_WING_BASE),
        [-6.0, 0.0, 0.0],
        [0.0, 0.0, -0.1],
        Cube::new([-13.0, 0.0, 0.0], [13.0, 1.0, 9.0], [16.0, 24.0]).mirrored(),
    ),
    part(
        "head",
        Some(BODY),
        [0.0, 1.0, -7.0],
        [0.2, 0.0, 0.0],
        Cube::new([-4.0, -2.0, -5.0], [7.0, 3.0, 5.0], [0.0, 0.0]),
    ),
];

pub fn flap_offset(id: u64) -> i32 {
    (id as i32).wrapping_mul(FLAP_TICKS_PER_ID)
}

pub fn flap_time(id: u64, age_in_ticks: f32) -> f32 {
    flap_offset(id) as f32 + age_in_ticks
}

pub fn flap(flap_time: f32) -> Flap {
    let angle = flap_time * FLAP_DEGREES_PER_TICK * DEGREES;
    Flap {
        wing: cos(angle) * WING_DEGREES * DEGREES,
        tail: -(5.0 + cos(angle * 2.0) * 5.0) * DEGREES,
    }
}

pub fn part_poses(flap: Flap) -> [Mat4; 8] {
    let mut rotations = PARTS.map(|part| part.rotation);
    rotations[LEFT_WING_BASE][2] = flap.wing;
    rotations[LEFT_WING_TIP][2] = flap.wing;
    rotations[RIGHT_WING_BASE][2] = -flap.wing;
    rotations[RIGHT_WING_TIP][2] = -flap.wing;
    rotations[TAIL_BASE][0] = flap.tail;
    rotations[TAIL_TIP][0] = flap.tail;
    let mut poses = [Mat4::IDENTITY; 8];
    for (index, part) in PARTS.iter().enumerate() {
        let own = part_pose(part.offset, rotations[index]);
        poses[index] = part.parent.map_or(own, |parent| poses[parent] * own);
    }
    poses
}

pub fn model_root(pose: &MobPose, size: i32) -> Mat4 {
    let scale = phantoms::scale(size);
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.body_rotation(FLIP_DEGREES))
        * Mat4::from_rotation_x(pose.head_pitch.to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_scale(Vec3::splat(scale))
        * Mat4::from_translation(Vec3::from_array(LIFT))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0))
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, poses: &[Mat4; 8], skin: &Skin) {
    for (part, pose) in PARTS.iter().zip(poses) {
        append(mesh, root * *pose, skin, &part.cube);
    }
}

fn skin(atlas: &Atlas, light: &SkyLight, pose: &MobPose) -> Skin {
    let position = pose.light_block();
    Skin {
        region: ResourceId::parse(TEXTURE).map_or([0.0; 4], |id| atlas.entity_region(&id)),
        texture_size: TEXTURE_SIZE,
        sky: light.get(position) as f32,
        block: light.get_block(position) as f32,
        alpha: 1.0,
        shaded: true,
    }
}

pub fn append_phantoms<'a>(
    mesh: &mut ChunkMesh,
    phantoms: impl IntoIterator<Item = &'a FamilyMob<Phantom>>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let eyes = ResourceId::parse(EYES_TEXTURE)
        .ok()
        .filter(|id| atlas.contains(id));
    let mut marks = Vec::new();
    for phantom in phantoms {
        let Some(pose) = poses.pose(phantom.core.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), pose.overlay(0.0)));
        let root = model_root(&pose, phantom.state.size);
        let parts = part_poses(flap(flap_time(phantom.core.id, pose.age_in_ticks)));
        let skin = skin(atlas, light, &pose);
        append_model(mesh, root, &parts, &skin);
        if let Some(eyes) = &eyes {
            marks.push((mesh.vertices.len(), NO_OVERLAY));
            let glow = Skin {
                region: atlas.entity_region(eyes),
                sky: FULL_BRIGHT,
                block: FULL_BRIGHT,
                shaded: false,
                ..skin
            };
            append(mesh, root * parts[HEAD], &glow, &PARTS[HEAD].cube);
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
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

    const FEET: DVec3 = DVec3::new(0.5, 70.0, 0.5);

    fn atlas(with_eyes: bool) -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let skins = root.join("assets/minecraft/textures/entity/phantom");
        std::fs::create_dir_all(&skins).unwrap();
        image::RgbaImage::from_pixel(64, 64, image::Rgba([40, 60, 120, 255]))
            .save(skins.join("phantom.png"))
            .unwrap();
        let mut eyes = image::RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 0]));
        for x in [5, 6, 10, 11] {
            eyes.put_pixel(x, 6, image::Rgba([136, 255, 0, 255]));
        }
        eyes.save(skins.join("phantom_eyes.png")).unwrap();
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let wanted = if with_eyes {
            &TEXTURES[..]
        } else {
            &TEXTURES[..1]
        };
        let ids = wanted
            .iter()
            .map(|id| ResourceId::parse(id).unwrap())
            .collect();
        crate::mesh::make_atlas(&packs, ids).unwrap()
    }

    fn light() -> SkyLight {
        SkyLight::from_levels((-8, 62, -8), (17, 17, 17), vec![4; 4913], vec![2; 4913])
    }

    fn world(size: i32, pitch: f32) -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        let id = world.spawn_family_kind(phantoms::KIND, FEET).unwrap();
        let mob = world.family_mob_mut::<Phantom>(id).unwrap();
        phantoms::set_size(mob, size);
        mob.core.look.pitch = pitch;
        (world, id)
    }

    fn drawn(world: &EntityWorld, atlas: &Atlas) -> (ChunkMesh, ClientMobs) {
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(world));
        let mut mesh = ChunkMesh::default();
        let shown = world.family::<Phantom>();
        append_phantoms(&mut mesh, shown, &poses, atlas, &light(), 0.0);
        (mesh, poses)
    }

    fn part_vertices(mesh: &ChunkMesh, part: usize) -> Vec<Vec3> {
        mesh.vertices[part * 24..(part + 1) * 24]
            .iter()
            .map(|v| Vec3::from_array(v.position))
            .collect()
    }

    fn middle(points: &[Vec3]) -> Vec3 {
        points.iter().copied().sum::<Vec3>() / points.len() as f32
    }

    #[test]
    fn a_phantom_is_eight_cubes_on_a_64_texture() {
        let names = PARTS.map(|part| part.name);
        assert_eq!(
            names,
            [
                "body",
                "tail_base",
                "tail_tip",
                "left_wing_base",
                "left_wing_tip",
                "right_wing_base",
                "right_wing_tip",
                "head"
            ]
        );
        for part in PARTS {
            let [w, h, d] = part.cube.size;
            let [u, v] = part.cube.uv;
            assert!(u + 2.0 * (d + w) <= TEXTURE_SIZE[0], "{}", part.name);
            assert!(v + d + h <= TEXTURE_SIZE[1], "{}", part.name);
            if let Some(parent) = part.parent {
                assert!(parent < PARTS.iter().position(|p| p == &part).unwrap());
            }
        }
        assert!(PARTS[RIGHT_WING_BASE].cube.mirror && PARTS[RIGHT_WING_TIP].cube.mirror);
        assert!(!PARTS[LEFT_WING_BASE].cube.mirror);

        let bare = atlas(false);
        let atlas = atlas(true);
        let (world, id) = world(0, 0.0);
        let (mesh, _) = drawn(&world, &atlas);
        assert_eq!(
            mesh.faces,
            9 * 6,
            "eight cubes and the head again for the eyes"
        );
        let body = atlas.entity_region(&ResourceId::parse(TEXTURE).unwrap());
        let eyes = atlas.entity_region(&ResourceId::parse(EYES_TEXTURE).unwrap());
        let inside = |uv: [f32; 2], region: [f32; 4]| {
            (region[0]..=region[2]).contains(&uv[0]) && (region[1]..=region[3]).contains(&uv[1])
        };
        let (model, glow) = mesh.vertices.split_at(8 * 24);
        assert!(model.iter().all(|v| inside(v.uv, body)));
        assert!(
            model
                .iter()
                .all(|v| (v.sky_light, v.block_light) == (4.0, 2.0))
        );
        assert!(model.iter().any(|v| v.color[0] < 1.0), "the body is shaded");
        assert!(glow.iter().all(|v| inside(v.uv, eyes)));
        assert!(
            glow.iter()
                .all(|v| (v.sky_light, v.block_light) == (15.0, 15.0))
        );
        assert!(
            glow.iter().all(|v| v.color == [1.0; 4]),
            "unshaded, no overlay"
        );
        let head = part_vertices(&mesh, HEAD);
        let again = part_vertices(&mesh, 8);
        assert_eq!(head, again, "the eyes sit on the head");
        assert!(
            model.iter().all(|v| v.color[3] == 1.0),
            "no overlay while unhurt"
        );

        let (mesh, _) = drawn(&world, &bare);
        assert_eq!(mesh.faces, 8 * 6, "no eyes without their texture");
        let mut none = ChunkMesh::default();
        let poses = ClientMobs::default();
        let shown = world.family::<Phantom>();
        append_phantoms(&mut none, shown, &poses, &atlas, &light(), 0.0);
        assert!(
            none.vertices.is_empty(),
            "a phantom the client has not seen"
        );
        assert_eq!(flap_offset(id), id as i32 * 3);
    }

    #[test]
    fn its_wings_flap_sixteen_degrees_and_its_tail_curls_with_the_flap_time() {
        let up = flap(0.0);
        assert_eq!(up.wing, 16.0 * DEGREES);
        assert_eq!(up.tail, -10.0 * DEGREES);
        let level = flap(90.0 / FLAP_DEGREES_PER_TICK);
        assert!(level.wing.abs() < 1e-3, "{level:?}");
        assert!(level.tail.abs() < 1e-3, "{level:?}");
        let down = flap(180.0 / FLAP_DEGREES_PER_TICK);
        assert!((down.wing + 16.0 * DEGREES).abs() < 1e-3, "{down:?}");
        assert!((down.tail - up.tail).abs() < 1e-3, "{down:?}");
        let period = 360.0 / FLAP_DEGREES_PER_TICK;
        for t in [0.0, 3.5, 17.25, 40.0] {
            let (a, b) = (flap(t), flap(t + period));
            assert!((a.wing - b.wing).abs() < 1e-3 && (a.tail - b.tail).abs() < 1e-3);
        }
        assert_eq!(flap_time(7, 2.5), 23.5);
        assert_eq!(flap_time(100, 0.0), 300.0);

        let rest = part_poses(level);
        let first = part_poses(up);
        let second = part_poses(down);
        let body = Mat4::from_rotation_x(-0.1);
        let pivot = |pose: Mat4| pose.transform_point3(Vec3::ZERO) * 16.0;
        let at =
            |pose: Mat4, x: f32, z: f32| pose.transform_point3(Vec3::new(x, 0.0, z) / 16.0) * 16.0;
        let wing_tip = body.transform_point3(Vec3::new(8.0, -2.0, -8.0));
        let tail_tip = body.transform_point3(Vec3::new(0.0, -1.5, 7.0));
        assert!(pivot(rest[LEFT_WING_TIP]).abs_diff_eq(wing_tip, 1e-2));
        assert!(pivot(rest[TAIL_TIP]).abs_diff_eq(tail_tip, 1e-2));
        let left = at(first[LEFT_WING_TIP], 13.0, 4.5);
        let flat = at(rest[LEFT_WING_TIP], 13.0, 4.5);
        let other = at(second[LEFT_WING_TIP], 13.0, 4.5);
        assert!(
            left.y > flat.y + 3.0 && other.y < flat.y - 3.0,
            "{left} {flat} {other}"
        );
        let right = at(first[RIGHT_WING_TIP], -13.0, 4.5);
        assert!((right.y - left.y).abs() < 1e-3, "both wings beat together");
        assert!(right.x < -18.0 && left.x > 18.0, "{right} {left}");
        let curled = at(first[TAIL_TIP], 0.0, 6.0);
        let straight = at(rest[TAIL_TIP], 0.0, 6.0);
        assert!(curled.y > straight.y + 2.0, "{curled} {straight}");
    }

    #[test]
    fn a_size_four_phantom_draws_one_point_six_times_larger_and_pitches_with_its_dive() {
        let atlas = atlas(true);
        let (small, id) = world(0, 0.0);
        let (big, big_id) = world(4, 0.0);
        let (diving, dive_id) = world(0, -35.0);
        assert_eq!((big_id, dive_id), (id, id), "the same flap time");
        let (small, poses) = drawn(&small, &atlas);
        let (big, _) = drawn(&big, &atlas);
        assert_eq!(small.vertices.len(), big.vertices.len());
        let feet = FEET.as_vec3();
        for (a, b) in small.vertices.iter().zip(&big.vertices) {
            let (a, b) = (Vec3::from_array(a.position), Vec3::from_array(b.position));
            assert!(((b - feet) - (a - feet) * 1.6).length() < 1e-3, "{a} {b}");
        }
        let body = middle(&part_vertices(&small, BODY));
        let lifted = 1.501 - LIFT[1] + 0.847 / 16.0;
        assert!(
            (body.y - feet.y - lifted).abs() < 0.01,
            "the body flies just above its feet: {body}"
        );
        let span = small
            .vertices
            .iter()
            .map(|v| v.position[0])
            .fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
        assert!(span.1 - span.0 > 2.3, "{span:?}");

        let (dive, dive_poses) = drawn(&diving, &atlas);
        let pitch = dive_poses.pose(id, 0.0).unwrap().head_pitch;
        assert!(pitch < -30.0, "{pitch}");
        let level = poses.pose(id, 0.0).unwrap();
        assert_eq!(level.head_pitch, 0.0);
        let turn = level.body_rotation(FLIP_DEGREES);
        let tilt = Mat4::from_quat(turn)
            * Mat4::from_rotation_x(pitch.to_radians())
            * Mat4::from_quat(turn.inverse());
        for (a, b) in small.vertices.iter().zip(&dive.vertices) {
            let (a, b) = (Vec3::from_array(a.position), Vec3::from_array(b.position));
            let expected = feet + tilt.transform_vector3(a - feet);
            assert!(b.abs_diff_eq(expected, 1e-3), "{b} {expected}");
        }
        let nose = |mesh: &ChunkMesh| middle(&part_vertices(mesh, HEAD)).y;
        let tail = |mesh: &ChunkMesh| middle(&part_vertices(mesh, TAIL_TIP)).y;
        assert!(nose(&dive) < tail(&dive) - 0.3, "it dives nose first");
        assert!(nose(&small) > tail(&small) - 0.3);
    }
}
