use crate::client_mobs::{ClientMobs, MobPose};
use crate::keyframes::{self, AnimationLibrary, AnimationState, DEGREES, Definition, Moved};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::land::rabbits::{Rabbit, RabbitVariant};
use std::sync::Arc;

pub const FLIP_DEGREES: f32 = 90.0;
pub const HEAD: &str = "head";
pub const ANIMATIONS: [&str; 2] = ["RabbitAnimation", "BabyRabbitAnimation"];
pub const HOP: &str = "HOP";
pub const IDLE_HEAD_TILT: &str = "IDLE_HEAD_TILT";

pub const TEXTURES: [&str; 16] = [
    "minecraft:entity/rabbit/rabbit_brown",
    "minecraft:entity/rabbit/rabbit_white",
    "minecraft:entity/rabbit/rabbit_black",
    "minecraft:entity/rabbit/rabbit_white_splotched",
    "minecraft:entity/rabbit/rabbit_gold",
    "minecraft:entity/rabbit/rabbit_salt",
    "minecraft:entity/rabbit/rabbit_caerbannog",
    "minecraft:entity/rabbit/rabbit_toast",
    "minecraft:entity/rabbit/rabbit_brown_baby",
    "minecraft:entity/rabbit/rabbit_white_baby",
    "minecraft:entity/rabbit/rabbit_black_baby",
    "minecraft:entity/rabbit/rabbit_white_splotched_baby",
    "minecraft:entity/rabbit/rabbit_gold_baby",
    "minecraft:entity/rabbit/rabbit_salt_baby",
    "minecraft:entity/rabbit/rabbit_caerbannog_baby",
    "minecraft:entity/rabbit/rabbit_toast_baby",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub name: &'static str,
    pub parent: Option<usize>,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub cube: Option<Cube>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    pub parts: &'static [Part],
    pub texture_size: [f32; 2],
}

const fn part(
    name: &'static str,
    parent: Option<usize>,
    offset: [f32; 3],
    rotation: [f32; 3],
    cube: Option<Cube>,
) -> Part {
    Part {
        name,
        parent,
        offset,
        rotation,
        cube,
    }
}

const fn cube(uv: [f32; 2], from: [f32; 3], size: [f32; 3]) -> Option<Cube> {
    Some(Cube::new(from, size, uv))
}

const NONE: [f32; 3] = [0.0; 3];

const ADULT_PARTS: [Part; 13] = [
    part(
        "body",
        None,
        [0.0, 23.0, 4.0],
        [-0.3927, 0.0, 0.0],
        cube([0.0, 0.0], [-4.0, -6.0, -9.0], [8.0, 6.0, 10.0]),
    ),
    part(
        "tail",
        Some(0),
        [0.0, -4.9916, 0.0125],
        NONE,
        cube([20.0, 16.0], [-2.0, -3.0084, -1.0125], [4.0, 4.0, 4.0]),
    ),
    part(
        HEAD,
        Some(0),
        [0.0, -5.2929, -8.1213],
        [0.3927, 0.0, 0.0],
        cube([0.0, 16.0], [-2.5, -3.0, -4.0], [5.0, 5.0, 5.0]),
    ),
    part(
        "left_ear",
        Some(2),
        [1.5, -3.7071, -0.8787],
        NONE,
        cube([32.0, 0.0], [-1.0, -4.2929, -0.1213], [2.0, 5.0, 1.0]),
    ),
    part(
        "right_ear",
        Some(2),
        [-1.5, -3.7071, -0.8787],
        NONE,
        cube([26.0, 0.0], [-1.0, -4.2929, -0.1213], [2.0, 5.0, 1.0]),
    ),
    part("frontlegs", Some(0), [0.0, -1.5349, -6.3108], NONE, None),
    part(
        "right_front_leg",
        Some(5),
        [-2.0, 1.9239, 0.3827],
        [0.3927, 0.0, 0.0],
        cube([36.0, 18.0], [-0.9, -1.0, -0.9], [2.0, 4.0, 2.0]),
    ),
    part(
        "left_front_leg",
        Some(5),
        [2.0, 1.9239, 0.4827],
        [0.3927, 0.0, 0.0],
        cube([44.0, 18.0], [-1.0, -1.0, -1.0], [2.0, 4.0, 2.0]),
    ),
    part("backlegs", None, [0.0, 23.0, 4.0], NONE, None),
    part("right_hind_leg", Some(8), [-3.0, 0.5, 0.0], NONE, None),
    part(
        "right_haunch",
        Some(9),
        [0.0, -0.5, 0.0],
        [0.0, 0.3927, 0.0],
        cube([20.0, 24.0], [-1.0, 0.0, -5.0], [2.0, 1.0, 6.0]),
    ),
    part("left_hind_leg", Some(8), [3.0, 0.5, 0.0], NONE, None),
    part(
        "left_haunch",
        Some(11),
        [0.0, -0.5, 0.0],
        [0.0, -0.3927, 0.0],
        cube([36.0, 24.0], [-1.0, 0.0, -5.0], [2.0, 1.0, 6.0]),
    ),
];

const BABY_PARTS: [Part; 17] = [
    part("body", None, [0.0, 23.0, 1.6], NONE, None),
    part(
        "body_r1",
        Some(0),
        [0.0, -2.0, -1.6],
        [-0.5236, 0.0, 0.0],
        cube([0.0, 8.0], [-2.0, -2.0, -3.0], [4.0, 3.0, 6.0]),
    ),
    part("tail", Some(0), [0.0, -2.2, 2.0], NONE, None),
    part(
        "tail_r1",
        Some(2),
        [-0.1, 0.0, 0.0],
        [-0.5236, 0.0, 0.0],
        cube([0.0, 21.0], [-1.4, -2.0268, -1.0177], [3.0, 3.0, 3.0]),
    ),
    part(
        HEAD,
        Some(0),
        [0.0, -5.0, -2.6],
        NONE,
        cube([0.0, 0.0], [-2.5, -3.0, -3.0], [5.0, 4.0, 4.0]),
    ),
    part(
        "right_ear",
        Some(4),
        [-1.5, -3.5, -0.5],
        NONE,
        cube([18.0, 0.0], [-1.0, -3.5, -0.5], [2.0, 4.0, 1.0]),
    ),
    part(
        "left_ear",
        Some(4),
        [1.5, -3.5, -0.5],
        NONE,
        cube([24.0, 0.0], [-1.0, -3.5, -0.5], [2.0, 4.0, 1.0]),
    ),
    part("frontlegs", Some(0), [0.0, -2.5, -2.6], NONE, None),
    part(
        "left_front_leg",
        Some(7),
        [1.0, 1.0, -0.5],
        [0.3927, 0.0, 0.0],
        None,
    ),
    part(
        "left_front_leg_r1",
        Some(8),
        [0.0, 1.0, 0.0],
        [-0.3927, 0.0, 0.0],
        cube([18.0, 8.0], [-0.5, -1.5, -0.5], [1.0, 3.0, 1.0]),
    ),
    part(
        "right_front_leg",
        Some(7),
        [-1.0, 1.0, -0.5],
        [0.3927, 0.0, 0.0],
        None,
    ),
    part(
        "right_front_leg_r1",
        Some(10),
        [0.0, 1.0, 0.0],
        [-0.3927, 0.0, 0.0],
        cube([14.0, 8.0], [-0.5, -1.5, -0.5], [1.0, 3.0, 1.0]),
    ),
    part("backlegs", None, [0.0, 23.0, 2.0], NONE, None),
    part(
        "left_hind_leg",
        Some(12),
        [1.5, 0.5, 0.5],
        [0.0, 3.1416, 0.0],
        None,
    ),
    part(
        "left_haunch",
        Some(13),
        [1.0, 0.0, 0.5],
        [0.0, -0.7854, 0.0],
        cube([10.0, 17.0], [-2.0, -0.5, 0.0], [2.0, 1.0, 3.0]),
    ),
    part(
        "right_hind_leg",
        Some(12),
        [-1.5, 0.5, 0.5],
        [0.0, 3.1416, 0.0],
        None,
    ),
    part(
        "right_haunch",
        Some(15),
        [0.5, 0.0, -0.9],
        [0.0, 0.7854, 0.0],
        cube([0.0, 17.0], [-2.0, -0.5, 0.0], [2.0, 1.0, 3.0]),
    ),
];

pub const ADULT: Model = Model {
    parts: &ADULT_PARTS,
    texture_size: [64.0, 64.0],
};

pub const BABY: Model = Model {
    parts: &BABY_PARTS,
    texture_size: [32.0, 32.0],
};

#[derive(Clone, Debug, Default)]
pub struct Clips {
    pub hop: Option<Arc<Definition>>,
    pub tilt: Option<Arc<Definition>>,
}

impl Clips {
    pub fn read(library: &AnimationLibrary, baby: bool) -> Clips {
        let class = ANIMATIONS[usize::from(baby)];
        Clips {
            hop: library.definition(class, HOP),
            tilt: library.definition(class, IDLE_HEAD_TILT),
        }
    }

    pub fn shared(baby: bool) -> Clips {
        Clips::read(keyframes::shared(), baby)
    }

    pub fn moved(&self, hop: Option<f32>, tilt: Option<f32>) -> Moved {
        let mut moved = Moved::default();
        for (clip, elapsed) in [(&self.hop, hop), (&self.tilt, tilt)] {
            if let (Some(clip), Some(elapsed)) = (clip, elapsed) {
                clip.apply_state(AnimationState::started(0), elapsed, 1.0, &mut moved);
            }
        }
        moved
    }
}

pub fn elapsed(start: Option<i32>, tick_count: i32, partial: f32) -> Option<f32> {
    start.map(|start| (tick_count - start) as f32 + partial)
}

pub fn model(baby: bool) -> &'static Model {
    if baby { &BABY } else { &ADULT }
}

pub fn texture(variant: RabbitVariant, toast: bool, baby: bool) -> &'static str {
    let index = if toast {
        7
    } else {
        match variant {
            RabbitVariant::Brown => 0,
            RabbitVariant::White => 1,
            RabbitVariant::Black => 2,
            RabbitVariant::WhiteSplotched => 3,
            RabbitVariant::Gold => 4,
            RabbitVariant::Salt => 5,
            RabbitVariant::Evil => 6,
        }
    };
    TEXTURES[index + if baby { 8 } else { 0 }]
}

pub fn part_poses(model: &Model, look: Option<[f32; 2]>, moved: &Moved) -> Vec<Mat4> {
    let mut out: Vec<Mat4> = Vec::with_capacity(model.parts.len());
    for part in model.parts {
        let mut rotation = Vec3::from_array(part.rotation);
        if let (HEAD, Some([pitch, yaw])) = (part.name, look) {
            rotation.x = pitch;
            rotation.y = yaw;
        }
        let offsets = moved.get(part.name);
        let at = Vec3::from_array(part.offset) + offsets.position;
        let local = part_pose(at.to_array(), (rotation + offsets.rotation).to_array())
            * Mat4::from_scale(Vec3::ONE + offsets.scale);
        let pose = part.parent.map_or(local, |parent| out[parent] * local);
        out.push(pose);
    }
    out
}

pub fn model_root(pose: &MobPose) -> Mat4 {
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.body_rotation(FLIP_DEGREES))
        * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn look(pose: &MobPose) -> [f32; 2] {
    [pose.head_pitch * DEGREES, pose.head_yaw * DEGREES]
}

fn skin(atlas: &Atlas, texture: &str, size: [f32; 2], light: &SkyLight, pose: &MobPose) -> Skin {
    let position = pose.light_block();
    Skin {
        region: ResourceId::parse(texture).map_or([0.0; 4], |id| atlas.entity_region(&id)),
        texture_size: size,
        sky: light.get(position) as f32,
        block: light.get_block(position) as f32,
        alpha: 1.0,
        shaded: true,
    }
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, model: &Model, poses: &[Mat4], skin: &Skin) {
    for (part, pose) in model.parts.iter().zip(poses) {
        if let Some(cube) = &part.cube {
            append(mesh, root * *pose, skin, cube);
        }
    }
}

pub fn append_rabbits<'a>(
    mesh: &mut ChunkMesh,
    rabbits: impl IntoIterator<Item = &'a FamilyMob<Rabbit>>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
    clips: &[Clips; 2],
) {
    let mut marks = Vec::new();
    for rabbit in rabbits {
        let Some(pose) = poses.pose(rabbit.core.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), pose.overlay(0.0)));
        let baby = rabbit.core.baby;
        let model = model(baby);
        let texture = texture(rabbit.state.variant, rabbit.state.toast, baby);
        let skin = skin(atlas, texture, model.texture_size, light, &pose);
        let (animations, now) = (&rabbit.state.animations, rabbit.core.tick_count);
        let tilt = elapsed(animations.tilt, now, partial);
        let hop = elapsed(animations.hop, now, partial);
        let moved = clips[usize::from(baby)].moved(hop, tilt);
        let look = tilt.is_none().then(|| look(&pose));
        let parts = part_poses(model, look, &moved);
        append_model(mesh, model_root(&pose), model, &parts, &skin);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use crate::families::{FamilyDraw, TerrainFamily};
    use crate::server_mobs::spawn_saved;
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
        let skins = root.join("assets/minecraft/textures/entity/rabbit");
        std::fs::create_dir_all(&skins).unwrap();
        for texture in TEXTURES {
            let name = texture.rsplit('/').next().unwrap();
            let size = if name.ends_with("_baby") { 32 } else { 64 };
            image::RgbaImage::from_pixel(size, size, image::Rgba([120, 90, 60, 255]))
                .save(skins.join(format!("{name}.png")))
                .unwrap();
        }
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let ids = TEXTURES
            .iter()
            .map(|texture| ResourceId::parse(texture).unwrap())
            .collect();
        crate::mesh::make_atlas(&packs, ids).unwrap()
    }

    fn light() -> SkyLight {
        SkyLight::from_levels((-8, 56, -8), (17, 17, 17), vec![15; 4913], vec![2; 4913])
    }

    fn pivot(pose: Mat4) -> Vec3 {
        pose.transform_point3(Vec3::ZERO) * 16.0
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        a.abs_diff_eq(b, 2e-3)
    }

    fn drawn(world: &EntityWorld, poses: &ClientMobs, atlas: &Atlas) -> (ChunkMesh, ChunkMesh) {
        let mut models = ChunkMesh::default();
        let mut culled = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let mut held = Vec::new();
        Rabbit::draw(
            &mut FamilyDraw {
                models: &mut models,
                culled: &mut culled,
                translucent: &mut translucent,
                held: &mut held,
            },
            world.family::<Rabbit>(),
            poses,
            atlas,
            &light(),
            0.0,
        );
        assert!(translucent.vertices.is_empty() && held.is_empty());
        (models, culled)
    }

    fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
    }

    #[test]
    fn an_adult_rabbit_has_thirteen_parts_on_64_and_a_baby_seventeen_on_32() {
        let cubes = |model: &Model| model.parts.iter().filter(|p| p.cube.is_some()).count();
        assert_eq!((ADULT.parts.len(), cubes(&ADULT)), (13, 9));
        assert_eq!((BABY.parts.len(), cubes(&BABY)), (17, 9));
        assert_eq!(
            (ADULT.texture_size, BABY.texture_size),
            ([64.0; 2], [32.0; 2])
        );
        for model in [&ADULT, &BABY] {
            for (index, part) in model.parts.iter().enumerate() {
                assert!(
                    part.parent.is_none_or(|parent| parent < index),
                    "{}",
                    part.name
                );
            }
            let names: Vec<&str> = model.parts.iter().map(|part| part.name).collect();
            for bone in [
                "body",
                "tail",
                HEAD,
                "left_ear",
                "right_ear",
                "frontlegs",
                "right_front_leg",
                "left_front_leg",
                "backlegs",
                "right_hind_leg",
                "left_hind_leg",
                "right_haunch",
                "left_haunch",
            ] {
                assert!(names.contains(&bone), "{bone}");
            }
        }
        let still = part_poses(&ADULT, None, &Moved::default());
        assert!(close(pivot(still[2]), Vec3::new(0.0, 15.002, -1.478)));
        let level = still[2].transform_vector3(Vec3::Z);
        assert!(
            close(level, Vec3::Z),
            "the head undoes the body's tilt: {level}"
        );
        assert!(close(pivot(still[3]), Vec3::new(1.5, 11.295, -2.357)));
        assert!(close(pivot(still[10]), Vec3::new(-3.0, 23.0, 4.0)));
        let baby = part_poses(&BABY, None, &Moved::default());
        assert!(close(pivot(baby[4]), Vec3::new(0.0, 18.0, -1.0)));
        assert!(close(
            pivot(baby[9]),
            Vec3::new(1.0, 21.5 + 0.92388, -1.5 + 0.38268)
        ));
        let turned = baby[14].transform_vector3(Vec3::X);
        assert!(
            close(turned, Vec3::new(-0.70711, 0.0, -0.70711)),
            "a baby's hind legs face back: {turned}"
        );
        let looking = part_poses(&ADULT, Some([0.2, -0.5]), &Moved::default());
        let face = (still[0] * Mat4::from_rotation_y(-0.5) * Mat4::from_rotation_x(0.2))
            .transform_vector3(Vec3::Z);
        assert!(
            close(looking[2].transform_vector3(Vec3::Z), face),
            "the look sets the head's turn in place of its pose"
        );
        assert!(close(
            pivot(looking[3]),
            pivot(looking[2])
                + looking[2].transform_vector3(Vec3::new(1.5, -3.7071, -0.8787) / 16.0) * 16.0
        ));
    }

    #[test]
    fn every_rabbit_type_picks_its_texture_and_toast_wins() {
        for (variant, name) in [
            (RabbitVariant::Brown, "brown"),
            (RabbitVariant::White, "white"),
            (RabbitVariant::Black, "black"),
            (RabbitVariant::WhiteSplotched, "white_splotched"),
            (RabbitVariant::Gold, "gold"),
            (RabbitVariant::Salt, "salt"),
            (RabbitVariant::Evil, "caerbannog"),
        ] {
            assert_eq!(
                texture(variant, false, false),
                format!("minecraft:entity/rabbit/rabbit_{name}")
            );
            assert_eq!(
                texture(variant, false, true),
                format!("minecraft:entity/rabbit/rabbit_{name}_baby")
            );
            assert_eq!(
                texture(variant, true, false),
                "minecraft:entity/rabbit/rabbit_toast"
            );
            assert_eq!(
                texture(variant, true, true),
                "minecraft:entity/rabbit/rabbit_toast_baby"
            );
        }
        assert_eq!(Rabbit::TEXTURES, TEXTURES);
        let mut unique = TEXTURES.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 16);
    }

    #[test]
    fn a_rabbit_s_lead_ties_at_its_neck() {
        let offset = crate::leash_render::leash_offset("minecraft:rabbit", 0.59, 0.49);
        assert!(
            offset.abs_diff_eq(DVec3::new(0.0, 0.6 * 0.59, 0.4 * 0.49), 1e-6),
            "{offset}"
        );
        let baby = crate::leash_render::leash_offset("minecraft:rabbit", 0.39, 0.24);
        assert!(
            baby.abs_diff_eq(DVec3::new(0.0, 0.6 * 0.39, 0.4 * 0.24), 1e-6),
            "{baby}"
        );
    }

    #[test]
    fn a_rabbit_is_drawn_with_its_skin_back_faces_culled() {
        let atlas = atlas();
        let mut world = EntityWorld::default();
        let parse = |text: &str| minecraftoss_core::snbt::parse_compound(text).unwrap();
        let adult = spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:rabbit\",Pos:[0.5d,64.0d,0.5d],RabbitType:4}"),
        )
        .unwrap();
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(&world));
        let (models, culled) = drawn(&world, &poses, &atlas);
        assert!(models.vertices.is_empty(), "entityCutout culls back faces");
        assert_eq!(culled.faces, 54, "nine cubes");
        let (low, high) = span(&culled, 1);
        assert!(
            low > 63.99 && low < 64.1,
            "the front paws reach the ground: {low}"
        );
        assert!(
            high > 64.7 && high < 65.1,
            "the ears stand above the head: {high}"
        );
        let gold = atlas.entity_region(&ResourceId::parse(TEXTURES[4]).unwrap());
        assert!(culled.vertices.iter().all(|v| {
            (gold[0]..=gold[2]).contains(&v.uv[0]) && (gold[1]..=gold[3]).contains(&v.uv[1])
        }));
        assert!(
            culled
                .vertices
                .iter()
                .all(|v| v.sky_light == 15.0 && v.block_light == 2.0)
        );
        assert!(
            culled.vertices.iter().all(|v| v.color[3] > 0.0),
            "no red when well"
        );
        let baby = spawn_saved(
            &mut world,
            &parse(
                "{id:\"minecraft:rabbit\",Pos:[4.5d,64.0d,0.5d],Age:-24000,CustomName:\"Toast\"}",
            ),
        )
        .unwrap();
        poses.spawn_missing(&server_mobs(&world));
        let (_, both) = drawn(&world, &poses, &atlas);
        assert_eq!(both.faces, 108);
        let toast = atlas.entity_region(&ResourceId::parse(TEXTURES[15]).unwrap());
        let small: Vec<_> = both
            .vertices
            .iter()
            .filter(|v| v.position[0] > 2.5)
            .collect();
        assert_eq!(small.len(), 9 * 24);
        assert!(small.iter().all(|v| {
            (toast[0]..=toast[2]).contains(&v.uv[0]) && (toast[1]..=toast[3]).contains(&v.uv[1])
        }));
        let top = small.iter().map(|v| v.position[1]).fold(f32::MIN, f32::max);
        assert!(top > 64.5 && top < high, "the baby is smaller: {top}");
        world.family_core_mut(baby).unwrap().1.damage.dead = true;
        world.family_core_mut(adult).unwrap().1.removed = true;
        poses.receive(server_mobs(&world));
        for _ in 0..20 {
            poses.tick();
        }
        let (_, dying) = drawn(&world, &poses, &atlas);
        assert_eq!(dying.faces, 54, "a rabbit that left is not drawn");
        assert!(
            dying.vertices.iter().all(|v| v.color[3] < 0.0),
            "red while dying"
        );
    }

    fn library() -> Option<AnimationLibrary> {
        let root = std::env::var_os("MINECRAFTOSS_ROOT")?;
        let jar = std::path::Path::new(&root).join("client.jar");
        jar.is_file().then(|| AnimationLibrary::new(Some(jar)))
    }

    #[test]
    fn a_rabbit_hops_with_the_hop_keyframes_and_tilts_its_head_when_idle() {
        let Some(library) = library() else {
            return;
        };
        for baby in [false, true] {
            let clips = Clips::read(&library, baby);
            let hop = clips.hop.as_ref().unwrap();
            let tilt = clips.tilt.as_ref().unwrap();
            assert_eq!((hop.length, hop.looping), (0.75, true), "{baby}");
            assert_eq!((tilt.length, tilt.looping), (4.0, false), "{baby}");
            let names: Vec<&str> = model(baby).parts.iter().map(|part| part.name).collect();
            for (bone, _) in hop.bones.iter().chain(&tilt.bones) {
                assert!(names.contains(&bone.as_str()), "{baby} {bone}");
            }
        }
        let clips = Clips::read(&library, false);
        let hop = |ticks: f32| clips.moved(Some(ticks), None);
        let legs = |moved: &Moved| moved.get("backlegs").rotation.x / DEGREES;
        assert_eq!(legs(&hop(0.0)), 0.0);
        let tucked = legs(&hop(5.0));
        assert!((tucked - 125.0).abs() < 1e-3, "{tucked}");
        assert!(
            (legs(&hop(20.0)) - 125.0).abs() < 1e-3,
            "the hop loops every 15 ticks"
        );
        let hopping = part_poses(&ADULT, None, &hop(5.0));
        let turned = hopping[8].transform_vector3(Vec3::Z);
        let tucked = Vec3::new(0.0, -(125.0 * DEGREES).sin(), (125.0 * DEGREES).cos());
        assert!(close(turned, tucked), "the back legs tuck in: {turned}");
        let still = part_poses(&ADULT, None, &Moved::default());
        let tilt = |ticks: f32| part_poses(&ADULT, None, &clips.moved(None, Some(ticks)));
        assert!(
            (1..80).any(|ticks| tilt(ticks as f32)[2] != still[2]),
            "the head tilts"
        );
        let end = tilt(80.0);
        for (held, last) in tilt(130.0).iter().zip(&end) {
            assert!(
                held.abs_diff_eq(*last, 1e-5),
                "a finished tilt keeps its last pose"
            );
        }
        let baby = Clips::read(&library, true);
        let small = part_poses(&BABY, None, &baby.moved(Some(5.0), None));
        let resting = part_poses(&BABY, None, &Moved::default());
        assert_ne!(small, resting, "the baby hops with its own keyframes");
        let atlas = atlas();
        let clips = [Clips::read(&library, false), Clips::read(&library, true)];
        let drawn = |pitch: f32, hop: Option<i32>, tilt: Option<i32>| {
            let mut world = EntityWorld::default();
            let id = world
                .spawn_family_kind("minecraft:rabbit", DVec3::new(0.5, 64.0, 0.5))
                .unwrap();
            let mob = world.family_mob_mut::<Rabbit>(id).unwrap();
            mob.core.look.pitch = pitch;
            mob.core.tick_count = 40;
            mob.state.animations.hop = hop;
            mob.state.animations.tilt = tilt;
            let mut poses = ClientMobs::default();
            poses.spawn_missing(&server_mobs(&world));
            let mut mesh = ChunkMesh::default();
            let rabbits = world.family::<Rabbit>();
            append_rabbits(&mut mesh, rabbits, &poses, &atlas, &light(), 0.0, &clips);
            mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>()
        };
        let sitting = drawn(0.0, None, None);
        assert_ne!(drawn(0.0, Some(35), None), sitting, "the drawn rabbit hops");
        assert_ne!(drawn(30.0, None, None), sitting, "it looks up at a player");
        let tilting = drawn(0.0, None, Some(10));
        assert_ne!(tilting, sitting, "and tilts its head");
        assert_eq!(
            drawn(30.0, None, Some(10)),
            tilting,
            "while the tilt runs the head does not follow the look"
        );
    }
}
