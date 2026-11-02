use crate::client_mobs::{ClientMobs, MobPose};
use crate::families::FamilyDraw;
use crate::keyframes::{self, AnimationLibrary, AnimationState, Definition, Moved};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::deep_dark::wardens::{self, Animations, Warden};
use minecraftoss_player::mth;
use std::sync::Arc;

pub const TEXTURE: &str = "minecraft:entity/warden/warden";
pub const BIOLUMINESCENT_TEXTURE: &str = "minecraft:entity/warden/warden_bioluminescent_layer";
pub const SPOTS_TEXTURE: &str = "minecraft:entity/warden/warden_pulsating_spots_1";
pub const SPOTS_TEXTURE_2: &str = "minecraft:entity/warden/warden_pulsating_spots_2";
pub const HEART_TEXTURE: &str = "minecraft:entity/warden/warden_heart";
pub const TEXTURES: [&str; 5] = [
    TEXTURE,
    BIOLUMINESCENT_TEXTURE,
    SPOTS_TEXTURE,
    SPOTS_TEXTURE_2,
    HEART_TEXTURE,
];
pub const TEXTURE_SIZE: [f32; 2] = [128.0, 128.0];
pub const SHADOW_RADIUS: f32 = 0.9;
pub const FLIP_DEGREES: f32 = 90.0;
pub const FULL_BRIGHT: f32 = 15.0;
pub const SPOTS_SPEED: f32 = 0.045;
pub const SPOTS_ALPHA: f32 = 0.25;
pub const SPOTS_PHASE: f32 = std::f32::consts::PI;
pub const HIDDEN_ALPHA: f32 = 1.0e-5;
pub const ANIMATIONS: &str = "WardenAnimation";
pub const CLIPS: [&str; 6] = [
    "WARDEN_ATTACK",
    "WARDEN_SONIC_BOOM",
    "WARDEN_DIG",
    "WARDEN_EMERGE",
    "WARDEN_ROAR",
    "WARDEN_SNIFF",
];
const DEGREES: f32 = 0.017_453_292;

pub const BONE: usize = 0;
pub const BODY: usize = 1;
pub const RIGHT_RIBCAGE: usize = 2;
pub const LEFT_RIBCAGE: usize = 3;
pub const HEAD: usize = 4;
pub const RIGHT_TENDRIL: usize = 5;
pub const LEFT_TENDRIL: usize = 6;
pub const RIGHT_ARM: usize = 7;
pub const LEFT_ARM: usize = 8;
pub const RIGHT_LEG: usize = 9;
pub const LEFT_LEG: usize = 10;

pub const BODY_PARTS: [usize; 11] = [
    BONE,
    BODY,
    RIGHT_RIBCAGE,
    LEFT_RIBCAGE,
    HEAD,
    RIGHT_TENDRIL,
    LEFT_TENDRIL,
    RIGHT_ARM,
    LEFT_ARM,
    RIGHT_LEG,
    LEFT_LEG,
];
pub const BIOLUMINESCENT_PARTS: [usize; 5] = [HEAD, RIGHT_ARM, LEFT_ARM, RIGHT_LEG, LEFT_LEG];
pub const SPOTS_PARTS: [usize; 3] = [BODY, RIGHT_LEG, LEFT_LEG];
pub const TENDRIL_PARTS: [usize; 2] = [RIGHT_TENDRIL, LEFT_TENDRIL];
pub const HEART_PARTS: [usize; 1] = [BODY];

#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub name: &'static str,
    pub parent: Option<usize>,
    pub offset: [f32; 3],
    pub cubes: &'static [Cube],
}

const fn part(
    name: &'static str,
    parent: Option<usize>,
    offset: [f32; 3],
    cubes: &'static [Cube],
) -> Part {
    Part {
        name,
        parent,
        offset,
        cubes,
    }
}

const TORSO: [Cube; 1] = [Cube::new(
    [-9.0, -13.0, -4.0],
    [18.0, 21.0, 11.0],
    [0.0, 0.0],
)];
const RIGHT_RIBS: [Cube; 1] = [Cube::new(
    [-2.0, -11.0, -0.1],
    [9.0, 21.0, 0.0],
    [90.0, 11.0],
)];
const LEFT_RIBS: [Cube; 1] =
    [Cube::new([-7.0, -11.0, -0.1], [9.0, 21.0, 0.0], [90.0, 11.0]).mirrored()];
const SKULL: [Cube; 1] = [Cube::new(
    [-8.0, -16.0, -5.0],
    [16.0, 16.0, 10.0],
    [0.0, 32.0],
)];
const RIGHT_HORN: [Cube; 1] = [Cube::new(
    [-16.0, -13.0, 0.0],
    [16.0, 16.0, 0.0],
    [52.0, 32.0],
)];
const LEFT_HORN: [Cube; 1] = [Cube::new([0.0, -13.0, 0.0], [16.0, 16.0, 0.0], [58.0, 0.0])];
const RIGHT_FOREARM: [Cube; 1] = [Cube::new([-4.0, 0.0, -4.0], [8.0, 28.0, 8.0], [44.0, 50.0])];
const LEFT_FOREARM: [Cube; 1] = [Cube::new([-4.0, 0.0, -4.0], [8.0, 28.0, 8.0], [0.0, 58.0])];
const RIGHT_SHIN: [Cube; 1] = [Cube::new([-3.1, 0.0, -3.0], [6.0, 13.0, 6.0], [76.0, 48.0])];
const LEFT_SHIN: [Cube; 1] = [Cube::new([-2.9, 0.0, -3.0], [6.0, 13.0, 6.0], [76.0, 76.0])];

pub const PARTS: [Part; 11] = [
    part("bone", None, [0.0, 24.0, 0.0], &[]),
    part("body", Some(BONE), [0.0, -21.0, 0.0], &TORSO),
    part("right_ribcage", Some(BODY), [-7.0, -2.0, -4.0], &RIGHT_RIBS),
    part("left_ribcage", Some(BODY), [7.0, -2.0, -4.0], &LEFT_RIBS),
    part("head", Some(BODY), [0.0, -13.0, 0.0], &SKULL),
    part("right_tendril", Some(HEAD), [-8.0, -12.0, 0.0], &RIGHT_HORN),
    part("left_tendril", Some(HEAD), [8.0, -12.0, 0.0], &LEFT_HORN),
    part("right_arm", Some(BODY), [-13.0, -13.0, 1.0], &RIGHT_FOREARM),
    part("left_arm", Some(BODY), [13.0, -13.0, 1.0], &LEFT_FOREARM),
    part("right_leg", Some(BONE), [-5.9, -13.0, 0.0], &RIGHT_SHIN),
    part("left_leg", Some(BONE), [5.9, -13.0, 0.0], &LEFT_SHIN),
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Look {
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub walk_position: f32,
    pub walk_speed: f32,
    pub age_in_ticks: f32,
    pub tendril: f32,
}

impl Look {
    pub fn of(pose: &MobPose, tendril: f32) -> Look {
        Look {
            head_yaw: pose.head_yaw,
            head_pitch: pose.head_pitch,
            walk_position: pose.walk_position,
            walk_speed: pose.walk_speed,
            age_in_ticks: pose.age_in_ticks,
            tendril,
        }
    }
}

pub fn rotations(look: &Look) -> [[f32; 3]; 11] {
    let mut out = [[0.0; 3]; 11];
    out[HEAD] = [look.head_pitch * DEGREES, look.head_yaw * DEGREES, 0.0];
    let speed = (3.0 * look.walk_speed).min(0.5);
    let step = look.walk_position * 0.8662;
    let (cos, sin) = (mth::cos(f64::from(step)), mth::sin(f64::from(step)));
    let sway = speed.min(0.35);
    out[HEAD][2] += 0.3 * sin * speed;
    out[HEAD][0] += 1.2 * mth::cos(f64::from(step + 1.570_796_4)) * sway;
    out[BODY] = [cos * sway, 0.0, 0.1 * sin * speed];
    out[LEFT_LEG][0] = cos * speed;
    out[RIGHT_LEG][0] = mth::cos(f64::from(step + std::f32::consts::PI)) * speed;
    out[LEFT_ARM][0] = -(0.8 * cos * speed);
    out[RIGHT_ARM][0] = -(0.8 * sin * speed);
    let idle = look.age_in_ticks * 0.1;
    let (cos, sin) = (mth::cos(f64::from(idle)), mth::sin(f64::from(idle)));
    out[HEAD][2] += 0.06 * cos;
    out[HEAD][0] += 0.06 * sin;
    out[BODY][2] += 0.025 * sin;
    out[BODY][0] += 0.025 * cos;
    let wave =
        (f64::from(look.age_in_ticks) * 2.25).cos() * std::f64::consts::PI * f64::from(0.1_f32);
    let wave = look.tendril * wave as f32;
    out[LEFT_TENDRIL][0] = wave;
    out[RIGHT_TENDRIL][0] = -wave;
    out
}

pub fn part_poses(turned: &[[f32; 3]; 11], moved: &Moved) -> [Mat4; 11] {
    let mut out = [Mat4::IDENTITY; 11];
    for (index, part) in PARTS.iter().enumerate() {
        let offsets = moved.get(part.name);
        let at = Vec3::from_array(part.offset) + offsets.position;
        let rotation = Vec3::from_array(turned[index]) + offsets.rotation;
        let local = part_pose(at.to_array(), rotation.to_array())
            * Mat4::from_scale(Vec3::ONE + offsets.scale);
        out[index] = part.parent.map_or(local, |parent| out[parent] * local);
    }
    out
}

pub fn spots_alpha(age_in_ticks: f32, phase: f32) -> f32 {
    let pulse = mth::cos(f64::from(age_in_ticks * SPOTS_SPEED + phase)) * SPOTS_ALPHA;
    pulse.max(0.0)
}

pub fn layers(
    age_in_ticks: f32,
    tendril: f32,
    heart: f32,
) -> [(&'static str, &'static [usize], f32); 5] {
    [
        (BIOLUMINESCENT_TEXTURE, &BIOLUMINESCENT_PARTS, 1.0),
        (SPOTS_TEXTURE, &SPOTS_PARTS, spots_alpha(age_in_ticks, 0.0)),
        (
            SPOTS_TEXTURE_2,
            &SPOTS_PARTS,
            spots_alpha(age_in_ticks, SPOTS_PHASE),
        ),
        (TEXTURE, &TENDRIL_PARTS, tendril),
        (HEART_TEXTURE, &HEART_PARTS, heart),
    ]
}

#[derive(Clone, Debug, Default)]
pub struct Clips {
    pub clips: [Option<Arc<Definition>>; 6],
}

impl Clips {
    pub fn read(library: &AnimationLibrary) -> Clips {
        Clips {
            clips: CLIPS.map(|field| library.definition(ANIMATIONS, field)),
        }
    }

    pub fn shared() -> Clips {
        Clips::read(keyframes::shared())
    }

    pub fn starts(animations: &Animations) -> [Option<i32>; 6] {
        [
            animations.attack,
            animations.sonic_boom,
            animations.dig,
            animations.emerge,
            animations.roar,
            animations.sniff,
        ]
    }

    pub fn moved(&self, animations: &Animations, tick_count: i32, partial: f32) -> Moved {
        let mut moved = Moved::default();
        for (clip, start) in self.clips.iter().zip(Clips::starts(animations)) {
            if let (Some(clip), Some(start)) = (clip, start) {
                let elapsed = (tick_count - start) as f32 + partial;
                clip.apply_state(AnimationState::started(0), elapsed, 1.0, &mut moved);
            }
        }
        moved
    }
}

pub fn model_root(pose: &MobPose) -> Mat4 {
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.body_rotation(FLIP_DEGREES))
        * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

fn skin(atlas: &Atlas, texture: &str, light: [f32; 2], alpha: f32) -> Skin {
    Skin {
        region: ResourceId::parse(texture).map_or([0.0; 4], |id| atlas.entity_region(&id)),
        texture_size: TEXTURE_SIZE,
        sky: light[0],
        block: light[1],
        alpha,
        shaded: true,
    }
}

pub fn append_parts(
    mesh: &mut ChunkMesh,
    root: Mat4,
    parts: &[Mat4; 11],
    keep: &[usize],
    skin: &Skin,
) {
    for &index in keep {
        for cube in PARTS[index].cubes {
            append(mesh, root * parts[index], skin, cube);
        }
    }
}

pub fn append_wardens<'a>(
    out: &mut FamilyDraw<'_>,
    wardens: impl IntoIterator<Item = &'a FamilyMob<Warden>>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
    clips: &Clips,
) {
    let mut marks = Vec::new();
    for warden in wardens {
        let id = warden.core.id;
        let (Some(pose), Some(shown)) = (poses.pose(id, partial), poses.get(id)) else {
            continue;
        };
        let state = &warden.state;
        let now = warden.core.tick_count;
        let clicked = state.animations.tendril.map(|start| now - start);
        let tendril = wardens::tendril_animation(clicked, partial);
        let heart = wardens::heart_animation(shown.tick_count, state.client_anger, partial);
        let moved = clips.moved(&state.animations, now, partial);
        let parts = part_poses(&rotations(&Look::of(&pose, tendril)), &moved);
        let root = model_root(&pose);
        let at = pose.light_block();
        let lit = [light.get(at) as f32, light.get_block(at) as f32];
        marks.push((out.models.vertices.len(), pose.overlay(0.0)));
        let body = skin(atlas, TEXTURE, lit, 1.0);
        append_parts(out.models, root, &parts, &BODY_PARTS, &body);
        for (texture, keep, alpha) in layers(pose.age_in_ticks, tendril, heart) {
            if alpha > HIDDEN_ALPHA {
                let glow = skin(atlas, texture, [FULL_BRIGHT; 2], alpha);
                append_parts(out.translucent, root, &parts, keep, &glow);
            }
        }
    }
    crate::cow_render::apply_overlays(out.models, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use crate::keyframes::{Channel, Interpolation, Keyframe, Target, degree_vec, pos_vec};
    use glam::DVec3;
    use minecraftoss_entities::world::EntityWorld;

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let skins = root.join("assets/minecraft/textures/entity/warden");
        std::fs::create_dir_all(&skins).unwrap();
        for (index, texture) in TEXTURES.iter().enumerate() {
            let name = texture.rsplit('/').next().unwrap();
            let shade = 40 + 40 * index as u8;
            image::RgbaImage::from_pixel(128, 128, image::Rgba([shade, 90, 120, 255]))
                .save(skins.join(format!("{name}.png")))
                .unwrap();
        }
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let ids = TEXTURES.map(|id| ResourceId::parse(id).unwrap()).to_vec();
        crate::mesh::make_atlas(&packs, ids).unwrap()
    }

    fn dark() -> SkyLight {
        SkyLight::from_levels((-8, 56, -8), (17, 17, 17), vec![0; 4913], vec![0; 4913])
    }

    fn region(atlas: &Atlas, texture: &str) -> [f32; 4] {
        atlas.entity_region(&ResourceId::parse(texture).unwrap())
    }

    fn inside(region: [f32; 4], uv: [f32; 2]) -> bool {
        (region[0] - 1e-5..=region[2] + 1e-5).contains(&uv[0])
            && (region[1] - 1e-5..=region[3] + 1e-5).contains(&uv[1])
    }

    fn warden_world() -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        let id = world
            .spawn_family_kind(wardens::KIND, DVec3::new(0.5, 64.0, 0.5))
            .unwrap();
        (world, id)
    }

    fn seen(world: &EntityWorld, ticks: i32) -> ClientMobs {
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(world));
        for _ in 0..ticks {
            poses.tick();
        }
        poses
    }

    struct Drawn {
        models: ChunkMesh,
        translucent: ChunkMesh,
    }

    fn drawn(
        atlas: &Atlas,
        world: &EntityWorld,
        poses: &ClientMobs,
        partial: f32,
        clips: &Clips,
    ) -> Drawn {
        let (mut models, mut culled, mut translucent) = Default::default();
        let mut held = Vec::new();
        let mut out = FamilyDraw {
            models: &mut models,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut held,
        };
        append_wardens(
            &mut out,
            world.family::<Warden>(),
            poses,
            atlas,
            &dark(),
            partial,
            clips,
        );
        assert!(culled.vertices.is_empty() && held.is_empty());
        Drawn {
            models,
            translucent,
        }
    }

    fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)))
    }

    fn still() -> [Mat4; 11] {
        part_poses(&[[0.0; 3]; 11], &Moved::default())
    }

    fn cubes(keep: &[usize]) -> usize {
        keep.iter().map(|&index| PARTS[index].cubes.len()).sum()
    }

    #[test]
    fn a_warden_has_eleven_parts_and_stands_over_three_blocks_tall() {
        assert_eq!(
            PARTS.map(|part| part.name),
            [
                "bone",
                "body",
                "right_ribcage",
                "left_ribcage",
                "head",
                "right_tendril",
                "left_tendril",
                "right_arm",
                "left_arm",
                "right_leg",
                "left_leg"
            ]
        );
        assert_eq!(
            (
                cubes(&BODY_PARTS),
                cubes(&BIOLUMINESCENT_PARTS),
                cubes(&SPOTS_PARTS),
                cubes(&TENDRIL_PARTS),
                cubes(&HEART_PARTS)
            ),
            (10, 5, 3, 2, 1)
        );
        assert_eq!(
            TEXTURES,
            [
                "minecraft:entity/warden/warden",
                "minecraft:entity/warden/warden_bioluminescent_layer",
                "minecraft:entity/warden/warden_pulsating_spots_1",
                "minecraft:entity/warden/warden_pulsating_spots_2",
                "minecraft:entity/warden/warden_heart"
            ]
        );
        assert_eq!((SHADOW_RADIUS, FLIP_DEGREES), (0.9, 90.0));
        let parts = still();
        let pivot = |index: usize| parts[index].transform_point3(Vec3::ZERO) * 16.0;
        assert!(pivot(HEAD).abs_diff_eq(Vec3::new(0.0, -10.0, 0.0), 1e-4));
        assert!(pivot(LEFT_TENDRIL).abs_diff_eq(Vec3::new(8.0, -22.0, 0.0), 1e-4));
        assert!(pivot(RIGHT_ARM).abs_diff_eq(Vec3::new(-13.0, -10.0, 1.0), 1e-4));
        assert!(pivot(LEFT_LEG).abs_diff_eq(Vec3::new(5.9, 11.0, 0.0), 1e-4));
        let feet = MobPose {
            feet: DVec3::ZERO,
            ..seen(&warden_world().0, 0)
                .iter()
                .next()
                .unwrap()
                .1
                .pose(0.0)
        };
        let root = model_root(&MobPose {
            body_rot: 180.0,
            ..feet
        });
        let mut whole = ChunkMesh::default();
        let skin = Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: TEXTURE_SIZE,
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        };
        append_parts(&mut whole, root, &parts, &BODY_PARTS, &skin);
        assert_eq!(whole.faces, 60);
        let (low, high) = span(&whole, 1);
        assert!((low - 0.001).abs() < 1e-4, "on its feet: {low}");
        assert!((high - 3.6885).abs() < 1e-4, "its tendrils' tips: {high}");
        let mut head = ChunkMesh::default();
        append_parts(&mut head, root, &parts, &[HEAD], &skin);
        let (_, top) = span(&head, 1);
        assert!(
            (top - 3.126).abs() < 1e-4 && top > wardens::HEIGHT,
            "its head tops its box: {top}"
        );
        let (west, east) = span(&whole, 0);
        assert!(
            (west + 1.5).abs() < 1e-4 && (east - 1.5).abs() < 1e-4,
            "{west} {east}"
        );
    }

    #[test]
    fn a_warden_is_drawn_whole_with_its_glowing_layers_on_their_own_parts() {
        let atlas = atlas();
        let (mut world, id) = warden_world();
        let poses = seen(&world, 0);
        let first = drawn(&atlas, &world, &poses, 0.0, &Clips::default());
        assert_eq!(first.models.faces, 60);
        let body = region(&atlas, TEXTURE);
        assert!(
            first
                .models
                .vertices
                .iter()
                .all(|v| inside(body, v.uv) && (v.sky_light, v.block_light) == (0.0, 0.0))
        );
        assert_eq!(
            first.translucent.faces,
            6 * (5 + 3),
            "the glowing spots and the first pulse, no tendrils or heart yet"
        );
        assert!(
            first
                .translucent
                .vertices
                .iter()
                .all(|v| (v.sky_light, v.block_light) == (FULL_BRIGHT, FULL_BRIGHT))
        );
        let (glowing, pulsing) = first.translucent.vertices.split_at(5 * 24);
        let lit = region(&atlas, BIOLUMINESCENT_TEXTURE);
        assert!(
            glowing
                .iter()
                .all(|v| inside(lit, v.uv) && v.color[3] == 1.0)
        );
        let spots = region(&atlas, SPOTS_TEXTURE);
        assert!(
            pulsing
                .iter()
                .all(|v| inside(spots, v.uv) && v.color[3] == 0.25)
        );
        let (low, high) = span(&first.translucent, 1);
        let (model_low, model_high) = span(&first.models, 1);
        assert!(low >= model_low - 1e-4 && high <= model_high + 1e-4);
        world.warden_mut(id).unwrap().core.tick_count = 30;
        let calm = drawn(&atlas, &world, &poses, 0.5, &Clips::default());
        world.warden_mut(id).unwrap().state.animations.tendril = Some(29);
        let clicked = drawn(&atlas, &world, &poses, 0.5, &Clips::default());
        let tendrils = &clicked.translucent.vertices[(5 + 3) * 24..];
        assert_eq!(tendrils.len(), 2 * 24);
        let skin = region(&atlas, TEXTURE);
        assert!(
            tendrils
                .iter()
                .all(|v| inside(skin, v.uv) && (v.color[3] - 0.95).abs() < 1e-6)
        );
        let positions = |mesh: &ChunkMesh, cubes: std::ops::Range<usize>| {
            mesh.vertices[cubes.start * 24..cubes.end * 24]
                .iter()
                .map(|v| v.position)
                .collect::<Vec<_>>()
        };
        assert_ne!(
            positions(&clicked.models, 4..6),
            positions(&calm.models, 4..6),
            "its tendrils wave"
        );
        for still in [0..4, 6..10] {
            assert_eq!(
                positions(&clicked.models, still.clone()),
                positions(&calm.models, still)
            );
        }
        let poses = seen(&world, 40);
        let beating = drawn(&atlas, &world, &poses, 0.0, &Clips::default());
        let heart = &beating.translucent.vertices[beating.translucent.vertices.len() - 24..];
        let red = region(&atlas, HEART_TEXTURE);
        assert!(heart.iter().all(|v| inside(red, v.uv) && v.color[3] == 1.0));
        let mut none = ChunkMesh::default();
        let mut culled = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let mut out = FamilyDraw {
            models: &mut none,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut Vec::new(),
        };
        append_wardens(
            &mut out,
            world.family::<Warden>(),
            &ClientMobs::default(),
            &atlas,
            &dark(),
            0.0,
            &Clips::default(),
        );
        assert!(none.vertices.is_empty() && translucent.vertices.is_empty());
    }

    #[test]
    fn its_spots_pulse_in_opposite_phase() {
        assert_eq!(spots_alpha(0.0, 0.0), 0.25);
        assert_eq!(spots_alpha(0.0, SPOTS_PHASE), 0.0);
        let mut brightest: [f32; 2] = [0.0; 2];
        for tenth in 0..2000 {
            let age = tenth as f32 * 0.1;
            let (one, two) = (spots_alpha(age, 0.0), spots_alpha(age, SPOTS_PHASE));
            assert!(one.min(two) <= HIDDEN_ALPHA, "{age}: {one} {two}");
            assert!(one <= SPOTS_ALPHA && two <= SPOTS_ALPHA);
            brightest = [brightest[0].max(one), brightest[1].max(two)];
        }
        assert!(brightest.iter().all(|b| (b - SPOTS_ALPHA).abs() < 1e-3));
        let half = std::f32::consts::PI / SPOTS_SPEED;
        assert!(spots_alpha(half, 0.0) <= HIDDEN_ALPHA);
        assert!((spots_alpha(half, SPOTS_PHASE) - SPOTS_ALPHA).abs() < 1e-3);
        let order = layers(0.0, 0.0, 0.0).map(|(texture, _, alpha)| (texture, alpha));
        assert_eq!(
            order,
            [
                (BIOLUMINESCENT_TEXTURE, 1.0),
                (SPOTS_TEXTURE, 0.25),
                (SPOTS_TEXTURE_2, 0.0),
                (TEXTURE, 0.0),
                (HEART_TEXTURE, 0.0)
            ]
        );
    }

    #[test]
    fn the_heart_glows_on_each_beat() {
        let atlas = atlas();
        let red = region(&atlas, HEART_TEXTURE);
        let (mut world, id) = warden_world();
        let hearts = |world: &EntityWorld, ticks: i32, partial: f32| {
            let poses = seen(world, ticks);
            let drawn = drawn(&atlas, world, &poses, partial, &Clips::default());
            let glowing: Vec<f32> = drawn
                .translucent
                .vertices
                .iter()
                .filter(|v| inside(red, v.uv))
                .map(|v| v.color[3])
                .collect();
            assert!(glowing.is_empty() || glowing.len() == 24);
            glowing.first().copied()
        };
        assert_eq!(hearts(&world, 40, 0.0), Some(1.0));
        assert_eq!(hearts(&world, 45, 0.0), Some(0.5));
        assert_eq!(hearts(&world, 50, 0.0), None, "dark until the next beat");
        assert_eq!(
            hearts(&world, 30, 0.0),
            None,
            "no beat before the fortieth tick"
        );
        world.warden_mut(id).unwrap().state.client_anger = 80;
        assert_eq!(
            hearts(&world, 50, 0.0),
            Some(1.0),
            "an angry heart beats every half second"
        );
        assert_eq!(hearts(&world, 30, 0.0), Some(1.0));
    }

    #[test]
    fn a_clip_moves_the_part_it_names_from_the_tick_it_started() {
        let mut roar = Definition::new(4.2, false);
        roar.add(
            "head",
            Channel::new(
                Target::Rotation,
                vec![
                    Keyframe::new(0.0, degree_vec(0.0, 0.0, 0.0), Interpolation::Linear),
                    Keyframe::new(1.0, degree_vec(-40.0, 0.0, 0.0), Interpolation::Linear),
                ],
            ),
        );
        let mut clips = Clips::default();
        clips.clips[4] = Some(Arc::new(roar));
        let animations = Animations {
            roar: Some(10),
            ..Animations::default()
        };
        let half = clips.moved(&animations, 20, 0.0).get("head").rotation.x;
        assert!((half + 20.0 * DEGREES).abs() < 1e-4, "{half}");
        let later = clips.moved(&animations, 50, 0.5).get("head").rotation.x;
        assert!(
            (later + 40.0 * DEGREES).abs() < 1e-4,
            "it holds its last frame"
        );
        assert_eq!(
            clips.moved(&Animations::default(), 20, 0.0),
            Moved::default()
        );
        let mut dig = Definition::new(5.0, false);
        dig.add(
            "body",
            Channel::new(
                Target::Position,
                vec![Keyframe::new(
                    0.0,
                    pos_vec(0.0, -40.0, 0.0),
                    Interpolation::Linear,
                )],
            ),
        );
        clips.clips[2] = Some(Arc::new(dig));
        let buried = Animations {
            dig: Some(0),
            ..animations
        };
        let moved = clips.moved(&buried, 20, 0.0);
        let parts = part_poses(&[[0.0; 3]; 11], &moved);
        let head = parts[HEAD].transform_point3(Vec3::ZERO) * 16.0;
        assert!((head.y - 30.0).abs() < 1e-3, "40 pixels down: {head}");
        assert_eq!(
            Clips::starts(&Animations {
                emerge: Some(1),
                dig: Some(2),
                roar: Some(3),
                sniff: Some(4),
                attack: Some(5),
                sonic_boom: Some(6),
                tendril: Some(7),
            }),
            [Some(5), Some(6), Some(2), Some(1), Some(3), Some(4)],
            "vanilla's order: attack, boom, dig, emerge, roar, sniff"
        );
    }

    #[test]
    fn its_head_follows_its_look_and_its_legs_its_walk() {
        let resting = rotations(&Look::default());
        assert!((resting[HEAD][2] - 0.06).abs() < 1e-6 && (resting[BODY][0] - 0.025).abs() < 1e-6);
        let looking = rotations(&Look {
            head_yaw: 30.0,
            head_pitch: -10.0,
            ..Look::default()
        });
        assert!((looking[HEAD][1] - 30.0 * DEGREES).abs() < 1e-6);
        assert!((looking[HEAD][0] - (-10.0 * DEGREES + 0.0)).abs() < 1e-6);
        let walking = rotations(&Look {
            walk_position: 1.0,
            walk_speed: 0.5,
            ..Look::default()
        });
        assert!(
            walking[LEFT_LEG][0] * walking[RIGHT_LEG][0] < 0.0,
            "its legs swing apart"
        );
        assert!(walking[LEFT_ARM][0] != 0.0 && walking[RIGHT_ARM][0] != 0.0);
        let clicked = rotations(&Look {
            tendril: 1.0,
            ..Look::default()
        });
        assert!(
            clicked[LEFT_TENDRIL][0] > 0.3
                && clicked[RIGHT_TENDRIL][0] == -clicked[LEFT_TENDRIL][0]
        );
        assert_eq!(resting[LEFT_TENDRIL], [0.0; 3]);
    }

    fn library() -> Option<AnimationLibrary> {
        let root = std::env::var_os("MINECRAFTOSS_ROOT")?;
        let jar = std::path::Path::new(&root).join("client.jar");
        jar.is_file().then(|| AnimationLibrary::new(Some(jar)))
    }

    #[test]
    fn emerging_starts_the_emerge_animation_and_poses_its_body_underground_at_the_start() {
        let Some(library) = library() else {
            return;
        };
        let clips = Clips::read(&library);
        for (field, clip) in CLIPS.iter().zip(&clips.clips) {
            let clip = clip.as_ref().unwrap_or_else(|| panic!("{field}"));
            for (bone, _) in &clip.bones {
                assert!(PARTS.iter().any(|part| part.name == bone), "{field} {bone}");
            }
        }
        let (mut world, id) = warden_world();
        let mob = world.warden_mut(id).unwrap();
        wardens::emerge(mob);
        assert_eq!(mob.state.animations.emerge, Some(0));
        let poses = seen(&world, 0);
        let pose = poses.pose(id, 0.0).unwrap();
        let root = model_root(&pose);
        let head_at = |tick: i32| {
            let mob = &world.family::<Warden>()[0];
            let moved = clips.moved(&mob.state.animations, tick, 0.0);
            let parts = part_poses(&[[0.0; 3]; 11], &moved);
            (root * parts[HEAD]).transform_point3(Vec3::ZERO).y
        };
        let ground = pose.feet.y as f32;
        assert!(
            head_at(0) < ground - 1.0,
            "its head starts deep in the ground: {}",
            head_at(0)
        );
        let rest = (root * still()[HEAD]).transform_point3(Vec3::ZERO).y;
        assert!(
            (head_at(140) - rest).abs() < 1e-3,
            "it stands up at the end: {}",
            head_at(140)
        );
        let atlas = atlas();
        let start = drawn(&atlas, &world, &poses, 0.0, &clips);
        let (_, top) = span(&start.models, 1);
        let mut standing = world.clone();
        standing.warden_mut(id).unwrap().state.animations = Animations::default();
        let stood = drawn(&atlas, &standing, &poses, 0.0, &clips);
        let (_, tall) = span(&stood.models, 1);
        assert!(top < tall - 2.0, "{top} {tall}");
    }
}
