use crate::client_mobs::{ClientMobs, MobPose};
use crate::families::FamilyDraw;
use crate::keyframes::{self, AnimationLibrary, AnimationState, Definition, Moved};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec2, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::trial::breezes::{Animations, Breeze};
use std::sync::Arc;

pub const TEXTURE: &str = "minecraft:entity/breeze/breeze";
pub const WIND_TEXTURE: &str = "minecraft:entity/breeze/breeze_wind";
pub const EYES_TEXTURE: &str = "minecraft:entity/breeze/breeze_eyes";
pub const TEXTURES: [&str; 3] = [TEXTURE, WIND_TEXTURE, EYES_TEXTURE];
pub const TEXTURE_SIZE: [f32; 2] = [32.0, 32.0];
pub const WIND_TEXTURE_SIZE: [f32; 2] = [128.0, 128.0];
pub const SHADOW_RADIUS: f32 = 0.5;
pub const FLIP_DEGREES: f32 = 90.0;
pub const WIND_SCROLL: f32 = 0.02;
pub const FULL_BRIGHT: f32 = 15.0;
pub const ANIMATIONS: &str = "BreezeAnimation";
pub const CLIPS: [&str; 6] = ["IDLE", "SHOOT", "SLIDE", "SLIDE_BACK", "INHALE", "JUMP"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Body,
    Wind,
    Eyes,
}

#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub name: &'static str,
    pub parent: Option<usize>,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub layer: Layer,
    pub cubes: &'static [Cube],
}

const ROD: [Cube; 1] = [Cube::new([-1.0, 0.0, -3.0], [2.0, 8.0, 2.0], [0.0, 17.0])];
const HEAD: [Cube; 2] = [
    Cube::new([-5.0, -5.0, -4.2], [10.0, 3.0, 4.0], [4.0, 24.0]),
    Cube::new([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], [0.0, 0.0]),
];
const WIND_BOTTOM: [Cube; 1] = [Cube::new([-2.5, -7.0, -2.5], [5.0, 7.0, 5.0], [1.0, 83.0])];
const WIND_MID: [Cube; 3] = [
    Cube::new([-6.0, -6.0, -6.0], [12.0, 6.0, 12.0], [74.0, 28.0]),
    Cube::new([-4.0, -6.0, -4.0], [8.0, 6.0, 8.0], [78.0, 32.0]),
    Cube::new([-2.5, -6.0, -2.5], [5.0, 6.0, 5.0], [49.0, 71.0]),
];
const WIND_TOP: [Cube; 3] = [
    Cube::new([-9.0, -8.0, -9.0], [18.0, 8.0, 18.0], [0.0, 0.0]),
    Cube::new([-6.0, -8.0, -6.0], [12.0, 8.0, 12.0], [6.0, 6.0]),
    Cube::new([-2.5, -8.0, -2.5], [5.0, 8.0, 5.0], [105.0, 57.0]),
];

const fn part(
    name: &'static str,
    parent: Option<usize>,
    offset: [f32; 3],
    rotation: [f32; 3],
    layer: Layer,
    cubes: &'static [Cube],
) -> Part {
    Part {
        name,
        parent,
        offset,
        rotation,
        layer,
        cubes,
    }
}

pub const PARTS: [Part; 11] = [
    part("body", None, [0.0; 3], [0.0; 3], Layer::Body, &[]),
    part("rods", Some(0), [0.0, 8.0, 0.0], [0.0; 3], Layer::Body, &[]),
    part(
        "rod_1",
        Some(1),
        [2.5981, -3.0, 1.5],
        [-2.7489, -1.0472, 3.1416],
        Layer::Body,
        &ROD,
    ),
    part(
        "rod_2",
        Some(1),
        [-2.5981, -3.0, 1.5],
        [-2.7489, 1.0472, 3.1416],
        Layer::Body,
        &ROD,
    ),
    part(
        "rod_3",
        Some(1),
        [0.0, -3.0, -3.0],
        [0.3927, 0.0, 0.0],
        Layer::Body,
        &ROD,
    ),
    part(
        "head",
        Some(0),
        [0.0, 4.0, 0.0],
        [0.0; 3],
        Layer::Body,
        &HEAD,
    ),
    part("eyes", Some(5), [0.0; 3], [0.0; 3], Layer::Eyes, &HEAD),
    part("wind_body", None, [0.0; 3], [0.0; 3], Layer::Wind, &[]),
    part(
        "wind_bottom",
        Some(7),
        [0.0, 24.0, 0.0],
        [0.0; 3],
        Layer::Wind,
        &WIND_BOTTOM,
    ),
    part(
        "wind_mid",
        Some(8),
        [0.0, -7.0, 0.0],
        [0.0; 3],
        Layer::Wind,
        &WIND_MID,
    ),
    part(
        "wind_top",
        Some(9),
        [0.0, -6.0, 0.0],
        [0.0; 3],
        Layer::Wind,
        &WIND_TOP,
    ),
];

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

    pub fn moved(&self, animations: &Animations, tick_count: i32, partial: f32) -> Moved {
        let mut moved = Moved::default();
        for (clip, start) in self.clips.iter().zip(animations.starts()) {
            if let (Some(clip), Some(start)) = (clip, start) {
                let elapsed = (tick_count - start) as f32 + partial;
                clip.apply_state(AnimationState::started(0), elapsed, 1.0, &mut moved);
            }
        }
        moved
    }
}

pub fn part_poses(moved: &Moved) -> [Mat4; 11] {
    let mut out = [Mat4::IDENTITY; 11];
    for (index, part) in PARTS.iter().enumerate() {
        let offsets = moved.get(part.name);
        let at = Vec3::from_array(part.offset) + offsets.position;
        let rotation = Vec3::from_array(part.rotation) + offsets.rotation;
        let local = part_pose(at.to_array(), rotation.to_array())
            * Mat4::from_scale(Vec3::ONE + offsets.scale);
        out[index] = part.parent.map_or(local, |parent| out[parent] * local);
    }
    out
}

pub fn model_root(pose: &MobPose) -> Mat4 {
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.body_rotation(FLIP_DEGREES))
        * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn wind_shift(age_in_ticks: f32) -> Vec2 {
    Vec2::new((age_in_ticks * WIND_SCROLL) % 1.0, 0.0)
}

fn skin(atlas: &Atlas, texture: &str, size: [f32; 2], light: [f32; 2], shaded: bool) -> Skin {
    Skin {
        region: ResourceId::parse(texture).map_or([0.0; 4], |id| atlas.entity_region(&id)),
        texture_size: size,
        sky: light[0],
        block: light[1],
        alpha: 1.0,
        shaded,
    }
}

pub fn append_layer(
    mesh: &mut ChunkMesh,
    root: Mat4,
    parts: &[Mat4; 11],
    layer: Layer,
    skin: &Skin,
) {
    for (part, pose) in PARTS.iter().zip(parts) {
        if part.layer == layer {
            for cube in part.cubes {
                append(mesh, root * *pose, skin, cube);
            }
        }
    }
}

pub fn append_wind(mesh: &mut ChunkMesh, root: Mat4, parts: &[Mat4; 11], skin: &Skin, shift: Vec2) {
    let mut plain = ChunkMesh::default();
    append_layer(&mut plain, root, parts, Layer::Wind, skin);
    for quad in plain.vertices.chunks(4) {
        crate::wither_render::wrap_face(mesh, quad, skin.region, shift);
    }
}

pub fn append_breezes<'a>(
    out: &mut FamilyDraw<'_>,
    breezes: impl IntoIterator<Item = &'a FamilyMob<Breeze>>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
    clips: &Clips,
) {
    let mut marks = Vec::new();
    for breeze in breezes {
        let Some(pose) = poses.pose(breeze.core.id, partial) else {
            continue;
        };
        let root = model_root(&pose);
        let moved = clips.moved(&breeze.state.animations, breeze.core.tick_count, partial);
        let parts = part_poses(&moved);
        let at = pose.light_block();
        let lit = [light.get(at) as f32, light.get_block(at) as f32];
        let body = skin(atlas, TEXTURE, TEXTURE_SIZE, lit, true);
        marks.push((out.models.vertices.len(), pose.overlay(0.0)));
        append_layer(out.models, root, &parts, Layer::Body, &body);
        let eyes = skin(atlas, EYES_TEXTURE, TEXTURE_SIZE, [FULL_BRIGHT; 2], true);
        marks.push((out.models.vertices.len(), 1.0));
        append_layer(out.models, root, &parts, Layer::Eyes, &eyes);
        let wind = skin(atlas, WIND_TEXTURE, WIND_TEXTURE_SIZE, lit, false);
        let shift = wind_shift(pose.age_in_ticks);
        append_wind(out.translucent, root, &parts, &wind, shift);
    }
    crate::cow_render::apply_overlays(out.models, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use glam::DVec3;
    use minecraftoss_entities::world::EntityWorld;
    use minecraftoss_entities::world::families::trial::breezes::Pose;

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let skins = root.join("assets/minecraft/textures/entity/breeze");
        std::fs::create_dir_all(&skins).unwrap();
        for (name, size, shade) in [
            ("breeze", 32, 200),
            ("breeze_wind", 128, 120),
            ("breeze_eyes", 32, 250),
        ] {
            image::RgbaImage::from_pixel(size, size, image::Rgba([shade, shade, 255, 255]))
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

    struct Drawn {
        models: ChunkMesh,
        translucent: ChunkMesh,
    }

    fn drawn(world: &EntityWorld, atlas: &Atlas, light: &SkyLight, clips: &Clips) -> Drawn {
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(world));
        let (mut models, mut culled, mut translucent) = Default::default();
        let mut held = Vec::new();
        let mut out = FamilyDraw {
            models: &mut models,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut held,
        };
        append_breezes(
            &mut out,
            world.family::<Breeze>(),
            &poses,
            atlas,
            light,
            0.0,
            clips,
        );
        assert!(culled.vertices.is_empty() && held.is_empty());
        Drawn {
            models,
            translucent,
        }
    }

    fn breeze_world() -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        let id = world
            .spawn_family_kind("minecraft:breeze", DVec3::new(0.5, 64.0, 0.5))
            .unwrap();
        (world, id)
    }

    fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)))
    }

    #[test]
    fn a_breeze_has_eleven_parts_on_its_three_textures() {
        assert_eq!(
            TEXTURES,
            [
                "minecraft:entity/breeze/breeze",
                "minecraft:entity/breeze/breeze_wind",
                "minecraft:entity/breeze/breeze_eyes"
            ]
        );
        let cubes = |layer| {
            PARTS
                .iter()
                .filter(|part| part.layer == layer)
                .map(|part| part.cubes.len())
                .sum::<usize>()
        };
        assert_eq!(
            (
                PARTS.len(),
                cubes(Layer::Body),
                cubes(Layer::Eyes),
                cubes(Layer::Wind)
            ),
            (11, 5, 2, 7)
        );
        assert_eq!((FLIP_DEGREES, SHADOW_RADIUS), (90.0, 0.5));
        let still = part_poses(&Moved::default());
        let pivot = |index: usize| still[index].transform_point3(Vec3::ZERO) * 16.0;
        assert!(pivot(5).abs_diff_eq(Vec3::new(0.0, 4.0, 0.0), 1e-4));
        assert!(
            pivot(6).abs_diff_eq(pivot(5), 1e-4),
            "the eyes sit on the head"
        );
        assert!(pivot(4).abs_diff_eq(Vec3::new(0.0, 5.0, -3.0), 1e-4));
        assert!(pivot(10).abs_diff_eq(Vec3::new(0.0, 11.0, 0.0), 1e-4));
        assert_eq!(
            PARTS.iter().map(|part| part.name).collect::<Vec<_>>(),
            [
                "body",
                "rods",
                "rod_1",
                "rod_2",
                "rod_3",
                "head",
                "eyes",
                "wind_body",
                "wind_bottom",
                "wind_mid",
                "wind_top"
            ]
        );
    }

    #[test]
    fn a_breeze_is_drawn_from_its_wind_up_to_its_head_with_glowing_eyes() {
        let atlas = atlas();
        let (world, id) = breeze_world();
        let drawn = drawn(&world, &atlas, &dark(), &Clips::default());
        assert_eq!(drawn.models.faces, 42, "five body cubes and two eye cubes");
        assert_eq!(drawn.translucent.faces, 42, "seven wind cubes, unshifted");
        let (low, high) = span(&drawn.models, 1);
        assert!((high - 65.751).abs() < 1e-4 && low > 64.4, "{low} {high}");
        let (low, high) = span(&drawn.translucent, 1);
        assert!(
            (low - 64.001).abs() < 1e-4 && (high - 65.3135).abs() < 1e-4,
            "{low} {high}"
        );
        let (west, east) = span(&drawn.translucent, 0);
        assert!((east - west - 18.0 / 16.0).abs() < 1e-4, "{west} {east}");
        let body = region(&atlas, TEXTURE);
        let eyes = region(&atlas, EYES_TEXTURE);
        let (body_vertices, eye_vertices) = drawn.models.vertices.split_at(5 * 24);
        assert!(body_vertices.iter().all(|v| inside(body, v.uv)
            && (v.sky_light, v.block_light) == (0.0, 0.0)
            && v.color[3] == 1.0));
        assert!(
            eye_vertices.iter().all(|v| inside(eyes, v.uv)
                && (v.sky_light, v.block_light) == (FULL_BRIGHT, FULL_BRIGHT))
        );
        assert!(
            body_vertices.iter().any(|v| v.color[0] < 1.0),
            "the body is shaded"
        );
        let wind = region(&atlas, WIND_TEXTURE);
        assert!(drawn.translucent.vertices.iter().all(|v| inside(wind, v.uv)
            && v.color == [1.0; 4]
            && (v.sky_light, v.block_light) == (0.0, 0.0)));
        let mut none = ChunkMesh::default();
        let mut culled = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let mut out = FamilyDraw {
            models: &mut none,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut Vec::new(),
        };
        let poses = ClientMobs::default();
        let breezes = world.family::<Breeze>();
        append_breezes(
            &mut out,
            breezes,
            &poses,
            &atlas,
            &dark(),
            0.0,
            &Clips::default(),
        );
        assert!(none.vertices.is_empty() && translucent.vertices.is_empty());
        assert!(world.family_core(id).is_some());
    }

    #[test]
    fn its_wind_scrolls_a_fiftieth_of_its_texture_a_tick() {
        assert_eq!(wind_shift(0.0), Vec2::ZERO);
        assert!((wind_shift(25.0).x - 0.5).abs() < 1e-6);
        assert!((wind_shift(75.0).x - 0.5).abs() < 1e-5);
        assert!((wind_shift(60.0).x - 0.2).abs() < 1e-5);
        assert!((0..400).all(|tick| wind_shift(tick as f32 * 0.7).y == 0.0));
        let atlas = atlas();
        let wind = region(&atlas, WIND_TEXTURE);
        let parts = part_poses(&Moved::default());
        let skin = skin(&atlas, WIND_TEXTURE, WIND_TEXTURE_SIZE, [15.0, 0.0], false);
        let mut plain = ChunkMesh::default();
        append_wind(&mut plain, Mat4::IDENTITY, &parts, &skin, Vec2::ZERO);
        let mut moved = ChunkMesh::default();
        append_wind(&mut moved, Mat4::IDENTITY, &parts, &skin, wind_shift(30.0));
        assert_eq!(plain.faces, 42);
        assert!(moved.faces > 42, "faces across the edge are cut");
        assert!(moved.vertices.iter().all(|v| inside(wind, v.uv)));
        assert_ne!(
            plain.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
            moved.vertices[..plain.vertices.len()]
                .iter()
                .map(|v| v.uv)
                .collect::<Vec<_>>()
        );
    }

    fn library() -> Option<AnimationLibrary> {
        let root = std::env::var_os("MINECRAFTOSS_ROOT")?;
        let jar = std::path::Path::new(&root).join("client.jar");
        jar.is_file().then(|| AnimationLibrary::new(Some(jar)))
    }

    #[test]
    fn the_breeze_keyframes_from_the_jar_turn_its_wind_and_move_its_head() {
        let Some(library) = library() else {
            return;
        };
        let clips = Clips::read(&library);
        let names: Vec<&str> = PARTS.iter().map(|part| part.name).collect();
        for (field, clip) in CLIPS.iter().zip(&clips.clips) {
            let clip = clip.as_ref().unwrap_or_else(|| panic!("{field}"));
            for (bone, _) in &clip.bones {
                assert!(names.contains(&bone.as_str()), "{field} {bone}");
            }
        }
        let idle = clips.clips[0].as_ref().unwrap();
        assert!(idle.looping);
        let animations = |pose: Pose| {
            let mut animations = Animations::default();
            animations.tick(pose, 1);
            animations.tick(pose, 2);
            animations
        };
        let still = part_poses(&Moved::default());
        let idling = (1..60)
            .map(|tick| part_poses(&clips.moved(&animations(Pose::Standing), tick, 0.0)))
            .collect::<Vec<_>>();
        assert!(
            idling
                .iter()
                .any(|parts| !parts[10].abs_diff_eq(still[10], 1e-4)),
            "the top of the wind turns"
        );
        assert!(
            idling
                .iter()
                .any(|parts| !parts[5].abs_diff_eq(still[5], 1e-4)),
            "the head bobs"
        );
        assert!(
            idling.iter().all(|parts| parts[6] == parts[5]),
            "the eyes stay on the head"
        );
        let shooting = clips.moved(&animations(Pose::Shooting), 6, 0.0);
        let idle_only = clips.moved(&animations(Pose::Standing), 6, 0.0);
        assert_ne!(shooting, idle_only, "the shot moves it more than idling");
        assert_ne!(
            clips.moved(&animations(Pose::Standing), 5, 0.0),
            clips.moved(&animations(Pose::Standing), 5, 0.5),
            "the partial tick moves it on"
        );
        let atlas = atlas();
        let (mut world, id) = breeze_world();
        world.family_mob_mut::<Breeze>(id).unwrap().state.animations = animations(Pose::Sliding);
        world.family_core_mut(id).unwrap().1.tick_count = 8;
        let sliding = drawn(&world, &atlas, &dark(), &clips);
        let resting = drawn(&world, &atlas, &dark(), &Clips::default());
        assert_ne!(
            sliding
                .models
                .vertices
                .iter()
                .map(|v| v.position)
                .collect::<Vec<_>>(),
            resting
                .models
                .vertices
                .iter()
                .map(|v| v.position)
                .collect::<Vec<_>>(),
            "the drawn breeze follows its keyframes"
        );
    }
}
