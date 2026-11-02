use crate::client_mobs::{ClientMobs, MobPose, quadruped_legs};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::land::polar_bears::{PolarBear, STAND_TICKS};
use std::f32::consts::{FRAC_PI_2, PI};

pub const FLIP_DEGREES: f32 = 90.0;
pub const ADULT_SCALE: f32 = 1.2;
pub const BABY_AGE_SCALE: f32 = 0.5;

pub const TEXTURES: [&str; 2] = [
    "minecraft:entity/bear/polarbear",
    "minecraft:entity/bear/polarbear_baby",
];

pub const HEAD: usize = 0;
pub const BODY: usize = 1;
pub const RIGHT_HIND_LEG: usize = 2;
pub const LEFT_HIND_LEG: usize = 3;
pub const RIGHT_FRONT_LEG: usize = 4;
pub const LEFT_FRONT_LEG: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub name: &'static str,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub cubes: &'static [Cube],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    pub parts: [Part; 6],
    pub texture_size: [f32; 2],
    pub scale: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stance {
    pub look: [f32; 2],
    pub legs: [f32; 4],
    pub stand: f32,
    pub age: f32,
}

const fn part(
    name: &'static str,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &'static [Cube],
) -> Part {
    Part {
        name,
        offset,
        rotation,
        cubes,
    }
}

const NONE: [f32; 3] = [0.0; 3];

const ADULT_HEAD: [Cube; 4] = [
    Cube::new([-3.5, -3.0, -3.0], [7.0, 7.0, 7.0], [0.0, 0.0]),
    Cube::new([-2.5, 1.0, -6.0], [5.0, 3.0, 3.0], [0.0, 44.0]),
    Cube::new([-4.5, -4.0, -1.0], [2.0, 2.0, 1.0], [26.0, 0.0]),
    Cube::new([2.5, -4.0, -1.0], [2.0, 2.0, 1.0], [26.0, 0.0]).mirrored(),
];

const ADULT_BODY: [Cube; 2] = [
    Cube::new([-5.0, -13.0, -7.0], [14.0, 14.0, 11.0], [0.0, 19.0]),
    Cube::new([-4.0, -25.0, -7.0], [12.0, 12.0, 10.0], [39.0, 0.0]),
];

const ADULT_HIND_LEG: [Cube; 1] = [Cube::new([-2.0, 0.0, -2.0], [4.0, 10.0, 8.0], [50.0, 22.0])];

const ADULT_FRONT_LEG: [Cube; 1] = [Cube::new([-2.0, 0.0, -2.0], [4.0, 10.0, 6.0], [50.0, 40.0])];

const BABY_HEAD: [Cube; 4] = [
    Cube::new([-3.0, -2.625, -4.25], [6.0, 5.0, 4.0], [0.0, 0.0]),
    Cube::new([-2.0, 0.375, -6.25], [4.0, 2.0, 2.0], [20.0, 3.0]),
    Cube::new([-4.0, -3.625, -2.75], [2.0, 2.0, 1.0], [20.0, 0.0]),
    Cube::new([2.0, -3.625, -2.75], [2.0, 2.0, 1.0], [26.0, 0.0]),
];

const BABY_BODY: [Cube; 1] = [Cube::new([-4.0, -3.5, -6.0], [8.0, 7.0, 12.0], [0.0, 9.0])];

const fn baby_leg(uv: [f32; 2]) -> Cube {
    Cube::new([-1.5, -0.5, -1.5], [3.0, 3.0, 3.0], uv)
}

const BABY_RIGHT_HIND_LEG: [Cube; 1] = [baby_leg([0.0, 34.0])];
const BABY_LEFT_HIND_LEG: [Cube; 1] = [baby_leg([12.0, 34.0])];
const BABY_RIGHT_FRONT_LEG: [Cube; 1] = [baby_leg([0.0, 28.0])];
const BABY_LEFT_FRONT_LEG: [Cube; 1] = [baby_leg([12.0, 28.0])];

pub const ADULT: Model = Model {
    parts: [
        part("head", [0.0, 10.0, -16.0], NONE, &ADULT_HEAD),
        part(
            "body",
            [-2.0, 9.0, 12.0],
            [FRAC_PI_2, 0.0, 0.0],
            &ADULT_BODY,
        ),
        part("right_hind_leg", [-4.5, 14.0, 6.0], NONE, &ADULT_HIND_LEG),
        part("left_hind_leg", [4.5, 14.0, 6.0], NONE, &ADULT_HIND_LEG),
        part(
            "right_front_leg",
            [-3.5, 14.0, -8.0],
            NONE,
            &ADULT_FRONT_LEG,
        ),
        part("left_front_leg", [3.5, 14.0, -8.0], NONE, &ADULT_FRONT_LEG),
    ],
    texture_size: [128.0, 64.0],
    scale: ADULT_SCALE,
};

pub const BABY: Model = Model {
    parts: [
        part("head", [0.0, 18.625, -5.75], NONE, &BABY_HEAD),
        part("body", [0.0, 17.5, 0.0], NONE, &BABY_BODY),
        part(
            "right_hind_leg",
            [-2.5, 21.5, 4.5],
            NONE,
            &BABY_RIGHT_HIND_LEG,
        ),
        part("left_hind_leg", [2.5, 21.5, 4.5], NONE, &BABY_LEFT_HIND_LEG),
        part(
            "right_front_leg",
            [-2.5, 21.5, -4.5],
            NONE,
            &BABY_RIGHT_FRONT_LEG,
        ),
        part(
            "left_front_leg",
            [2.5, 21.5, -4.5],
            NONE,
            &BABY_LEFT_FRONT_LEG,
        ),
    ],
    texture_size: [64.0, 64.0],
    scale: 1.0,
};

pub fn model(baby: bool) -> &'static Model {
    if baby { &BABY } else { &ADULT }
}

pub fn texture(baby: bool) -> &'static str {
    TEXTURES[usize::from(baby)]
}

pub fn age_scale(baby: bool) -> f32 {
    if baby { BABY_AGE_SCALE } else { 1.0 }
}

pub fn stand_scale(stand_old: i32, stand: i32, partial: f32) -> f32 {
    let (old, now) = (stand_old as f32, stand as f32);
    (old + partial * (now - old)) / STAND_TICKS as f32
}

pub fn part_poses(model: &Model, stance: &Stance) -> [Mat4; 6] {
    let mut offsets = model.parts.map(|part| part.offset);
    let mut rotations = model.parts.map(|part| part.rotation);
    let [pitch, yaw] = stance.look;
    rotations[HEAD][0] = pitch;
    rotations[HEAD][1] = yaw;
    let legs = [
        RIGHT_HIND_LEG,
        LEFT_HIND_LEG,
        RIGHT_FRONT_LEG,
        LEFT_FRONT_LEG,
    ];
    for (leg, swing) in legs.into_iter().zip(stance.legs) {
        rotations[leg][0] = swing;
    }
    let (f, a) = (stance.stand * stance.stand, stance.age);
    rotations[BODY][0] -= f * PI * 0.35;
    offsets[BODY][1] += f * a * 2.0;
    for leg in [RIGHT_FRONT_LEG, LEFT_FRONT_LEG] {
        offsets[leg][1] -= f * a * 20.0;
        offsets[leg][2] += f * a * 4.0;
        rotations[leg][0] -= f * PI * 0.45;
    }
    offsets[HEAD][1] -= f * 24.0;
    offsets[HEAD][2] += f * 13.0;
    rotations[HEAD][0] += f * PI * 0.15;
    std::array::from_fn(|index| part_pose(offsets[index], rotations[index]))
}

pub fn scaled_root(scale: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.0, 24.016 * (1.0 - scale) / 16.0, 0.0))
        * Mat4::from_scale(Vec3::splat(scale))
}

pub fn model_root(pose: &MobPose, model: &Model) -> Mat4 {
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.body_rotation(FLIP_DEGREES))
        * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * scaled_root(model.scale)
}

pub fn stance(pose: &MobPose, bear: &PolarBear, baby: bool, partial: f32) -> Stance {
    Stance {
        look: [pose.head_pitch.to_radians(), pose.head_yaw.to_radians()],
        legs: quadruped_legs(pose.walk_position, pose.walk_speed),
        stand: stand_scale(bear.stand_old, bear.stand, partial),
        age: age_scale(baby),
    }
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

pub fn append_model(
    mesh: &mut ChunkMesh,
    root: Mat4,
    model: &Model,
    poses: &[Mat4; 6],
    skin: &Skin,
) {
    for (part, pose) in model.parts.iter().zip(poses) {
        for cube in part.cubes {
            append(mesh, root * *pose, skin, cube);
        }
    }
}

pub fn append_polar_bears<'a>(
    mesh: &mut ChunkMesh,
    bears: impl IntoIterator<Item = &'a FamilyMob<PolarBear>>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let mut marks = Vec::new();
    for bear in bears {
        let Some(pose) = poses.pose(bear.core.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), pose.overlay(0.0)));
        let baby = bear.core.baby;
        let model = model(baby);
        let skin = skin(atlas, texture(baby), model.texture_size, light, &pose);
        let parts = part_poses(model, &stance(&pose, &bear.state, baby, partial));
        append_model(mesh, model_root(&pose, model), model, &parts, &skin);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use crate::families::{FamilyDraw, TerrainFamily};
    use crate::mob_tracking::{shows, spawn_tracked};
    use crate::server_mobs::{mob_tags, spawn_saved};
    use minecraftoss_core::nbt::Tag;
    use minecraftoss_entities::world::EntityWorld;
    use std::collections::HashMap;

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let skins = root.join("assets/minecraft/textures/entity/bear");
        std::fs::create_dir_all(&skins).unwrap();
        for (texture, width) in TEXTURES.into_iter().zip([128, 64]) {
            let name = texture.rsplit('/').next().unwrap();
            image::RgbaImage::from_pixel(width, 64, image::Rgba([230, 230, 220, 255]))
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

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    fn pivot(pose: Mat4) -> Vec3 {
        pose.transform_point3(Vec3::ZERO) * 16.0
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        a.abs_diff_eq(b, 2e-3)
    }

    fn standing(stand: f32, baby: bool) -> Stance {
        Stance {
            stand,
            age: age_scale(baby),
            ..Stance::default()
        }
    }

    fn drawn(world: &EntityWorld, atlas: &Atlas, partial: f32) -> (ChunkMesh, ChunkMesh) {
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(world));
        let mut models = ChunkMesh::default();
        let mut culled = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let mut held = Vec::new();
        PolarBear::draw(
            &mut FamilyDraw {
                models: &mut models,
                culled: &mut culled,
                translucent: &mut translucent,
                held: &mut held,
            },
            world.family::<PolarBear>(),
            &poses,
            atlas,
            &light(),
            partial,
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

    fn inside(region: [f32; 4], mesh: &ChunkMesh) -> bool {
        mesh.vertices.iter().all(|v| {
            (region[0]..=region[2]).contains(&v.uv[0]) && (region[1]..=region[3]).contains(&v.uv[1])
        })
    }

    fn set_stand(world: &mut EntityWorld, id: u64, old: i32, now: i32) {
        let mob = world.family_mob_mut::<PolarBear>(id).unwrap();
        mob.state.goals_mut().standing = now >= old && now > 0;
        mob.state.stand_old = old;
        mob.state.stand = now;
    }

    #[test]
    fn the_adult_is_scaled_1_2_and_the_cub_has_its_own_model() {
        let cubes = |model: &Model| model.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
        assert_eq!(cubes(&ADULT), 10);
        assert_eq!(cubes(&BABY), 9);
        assert_eq!(
            (ADULT.texture_size, BABY.texture_size),
            ([128.0, 64.0], [64.0, 64.0])
        );
        assert_eq!((ADULT.scale, BABY.scale), (1.2, 1.0));
        for model in [&ADULT, &BABY] {
            let names = model.parts.map(|part| part.name);
            assert_eq!(
                names,
                [
                    "head",
                    "body",
                    "right_hind_leg",
                    "left_hind_leg",
                    "right_front_leg",
                    "left_front_leg"
                ]
            );
        }
        assert!(
            ADULT.parts[HEAD].cubes[3].mirror,
            "the left ear is mirrored"
        );
        assert!(!BABY.parts[HEAD].cubes.iter().any(|cube| cube.mirror));
        assert_eq!(model(false), &ADULT);
        assert_eq!(model(true), &BABY);
        assert_eq!(texture(false), "minecraft:entity/bear/polarbear");
        assert_eq!(texture(true), "minecraft:entity/bear/polarbear_baby");
        assert_eq!(PolarBear::TEXTURES, TEXTURES);
        assert_eq!(PolarBear::SHADOW_RADIUS, 0.9);
        let root = scaled_root(1.2);
        let floor = root.transform_point3(Vec3::new(3.0, 24.016, -5.0) / 16.0) * 16.0;
        assert!(
            close(floor, Vec3::new(3.6, 24.016, -6.0)),
            "MeshTransformer.scaling grows the model from the floor: {floor}"
        );
        assert_eq!(scaled_root(1.0), Mat4::IDENTITY);
        let still = part_poses(&ADULT, &standing(0.0, false));
        assert!(close(pivot(still[HEAD]), Vec3::new(0.0, 10.0, -16.0)));
        let along = still[BODY].transform_vector3(Vec3::NEG_Y);
        assert!(
            close(along, Vec3::NEG_Z),
            "the body lies along the bear: {along}"
        );
        assert!(close(
            pivot(still[LEFT_HIND_LEG]),
            Vec3::new(4.5, 14.0, 6.0)
        ));

        let atlas = atlas();
        let mut world = EntityWorld::default();
        let adult = spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:polar_bear\",Pos:[0.5d,64.0d,0.5d]}"),
        )
        .unwrap();
        let (models, culled) = drawn(&world, &atlas, 0.0);
        assert!(models.vertices.is_empty(), "entityCutout culls back faces");
        assert_eq!(culled.faces, 60, "ten cubes");
        let (low, high) = span(&culled, 1);
        assert!(
            (low - 64.0).abs() < 0.01,
            "the paws reach the ground: {low}"
        );
        assert!(
            (high - 65.426).abs() < 0.01,
            "1.2 times the model, the ears on top: {high}"
        );
        let (west, east) = span(&culled, 0);
        let (north, south) = span(&culled, 2);
        let length = (east - west).max(south - north);
        assert!(
            (length - 35.0 * 1.2 / 16.0).abs() < 0.01,
            "snout to tail: {length}"
        );
        let skin = atlas.entity_region(&ResourceId::parse(TEXTURES[0]).unwrap());
        assert!(inside(skin, &culled));
        assert!(
            culled
                .vertices
                .iter()
                .all(|v| v.sky_light == 15.0 && v.block_light == 2.0 && v.color[3] > 0.0)
        );

        world.family_core_mut(adult).unwrap().1.removed = true;
        spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:polar_bear\",Pos:[0.5d,64.0d,0.5d],Age:-24000}"),
        )
        .unwrap();
        let (_, cub) = drawn(&world, &atlas, 0.0);
        assert_eq!(cub.faces, 54, "nine cubes, the adult that left not drawn");
        let (low, high) = span(&cub, 1);
        assert!((low - 64.0).abs() < 0.01, "{low}");
        assert!(
            (high - (64.0 + 1.501 - 14.0 / 16.0)).abs() < 0.01,
            "the cub's own model, unscaled, its back on top: {high}"
        );
        let skin = atlas.entity_region(&ResourceId::parse(TEXTURES[1]).unwrap());
        assert!(inside(skin, &cub));
    }

    #[test]
    fn a_standing_bear_lifts_its_body_front_legs_and_head() {
        let still = part_poses(&ADULT, &standing(0.0, false));
        let up = part_poses(&ADULT, &standing(1.0, false));
        assert!(close(pivot(up[BODY]), Vec3::new(-2.0, 11.0, 12.0)));
        let along = up[BODY].transform_vector3(Vec3::NEG_Y);
        let tilt = 0.15 * PI;
        assert!(
            close(along, Vec3::new(0.0, -tilt.cos(), -tilt.sin())),
            "the body rears up to 63 degrees: {along}"
        );
        for leg in [RIGHT_FRONT_LEG, LEFT_FRONT_LEG] {
            let x = ADULT.parts[leg].offset[0];
            assert!(close(pivot(up[leg]), Vec3::new(x, -6.0, -4.0)));
            let reach = up[leg].transform_vector3(Vec3::Y);
            let angle = -0.45 * PI;
            assert!(
                close(reach, Vec3::new(0.0, angle.cos(), angle.sin())),
                "the front paws reach forward: {reach}"
            );
        }
        assert!(close(pivot(up[HEAD]), Vec3::new(0.0, -14.0, -3.0)));
        let face = up[HEAD].transform_vector3(Vec3::NEG_Z);
        let nod = 0.15 * PI;
        assert!(close(face, Vec3::new(0.0, nod.sin(), -nod.cos())));
        for leg in [RIGHT_HIND_LEG, LEFT_HIND_LEG] {
            assert_eq!(up[leg], still[leg], "the hind legs stay put");
        }

        let half = part_poses(&ADULT, &standing(0.5, false));
        assert!(
            close(pivot(half[HEAD]), Vec3::new(0.0, 10.0 - 6.0, -16.0 + 3.25)),
            "the pose follows the stand scale squared"
        );
        let cub = part_poses(&BABY, &standing(1.0, true));
        assert!(close(pivot(cub[BODY]), Vec3::new(0.0, 18.5, 0.0)));
        assert!(close(
            pivot(cub[RIGHT_FRONT_LEG]),
            Vec3::new(-2.5, 11.5, -2.5)
        ));
        assert!(
            close(pivot(cub[HEAD]), Vec3::new(0.0, -5.375, 7.25)),
            "the head moves by the full amount for a cub too"
        );

        let looking = part_poses(
            &ADULT,
            &Stance {
                look: [0.3, -0.4],
                stand: 1.0,
                age: 1.0,
                ..Stance::default()
            },
        );
        let turned = (Mat4::from_translation(Vec3::new(0.0, -14.0, -3.0) / 16.0)
            * Mat4::from_rotation_y(-0.4)
            * Mat4::from_rotation_x(0.3 + 0.15 * PI))
        .transform_vector3(Vec3::NEG_Z);
        assert!(close(looking[HEAD].transform_vector3(Vec3::NEG_Z), turned));
        let walking = part_poses(
            &ADULT,
            &Stance {
                legs: [0.5, -0.5, -0.5, 0.5],
                stand: 1.0,
                age: 1.0,
                ..Stance::default()
            },
        );
        let swung = walking[RIGHT_FRONT_LEG].transform_vector3(Vec3::Y);
        let angle = -0.5 - 0.45 * PI;
        assert!(close(swung, Vec3::new(0.0, angle.cos(), angle.sin())));

        let atlas = atlas();
        let mut world = EntityWorld::default();
        let id = spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:polar_bear\",Pos:[0.5d,64.0d,0.5d]}"),
        )
        .unwrap();
        let (_, before) = drawn(&world, &atlas, 0.0);
        set_stand(&mut world, id, 6, 6);
        let (_, reared) = drawn(&world, &atlas, 0.0);
        let (low, high) = span(&reared, 1);
        assert!((low - 64.0).abs() < 0.01, "on its hind paws: {low}");
        assert!(
            high > span(&before, 1).1 + 1.4,
            "a standing bear is far taller: {high}"
        );
    }

    #[test]
    fn the_stand_clock_lerps_with_the_partial_tick() {
        let sixths = |value: f32| value / 6.0;
        assert_eq!(stand_scale(0, 0, 0.7), 0.0);
        assert_eq!(stand_scale(6, 6, 0.3), 1.0);
        assert_eq!(stand_scale(2, 3, 0.0), sixths(2.0));
        assert_eq!(stand_scale(2, 3, 0.5), sixths(2.5));
        assert_eq!(stand_scale(2, 3, 1.0), sixths(3.0));
        assert_eq!(stand_scale(4, 3, 0.25), sixths(3.75));
        assert_eq!(stand_scale(1, 0, 1.0), 0.0);

        let atlas = atlas();
        let tag = parse("{id:\"minecraft:polar_bear\",Pos:[0.5d,64.0d,0.5d]}");
        let mut host = EntityWorld::default();
        let id = spawn_saved(&mut host, &tag).unwrap();
        let positions =
            |mesh: ChunkMesh| mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>();
        let at = |world: &mut EntityWorld, old: i32, now: i32, partial: f32| {
            set_stand(world, id, old, now);
            positions(drawn(world, &atlas, partial).1)
        };
        let two = at(&mut host, 2, 2, 0.0);
        let three = at(&mut host, 3, 3, 0.0);
        assert_ne!(two, three);
        assert_eq!(at(&mut host, 2, 3, 0.0), two, "partial 0 is last tick's");
        assert_eq!(at(&mut host, 2, 3, 1.0), three, "partial 1 is this tick's");
        let between = at(&mut host, 2, 3, 0.5);
        assert!(between != two && between != three);
        let head = part_poses(&ADULT, &standing(stand_scale(2, 3, 0.5), false))[HEAD];
        let f = (2.5f32 / 6.0).powi(2);
        assert!(
            close(
                pivot(head),
                Vec3::new(0.0, 10.0 - 24.0 * f, -16.0 + 13.0 * f)
            ),
            "the draw lerps the clock, then squares it"
        );

        set_stand(&mut host, id, 2, 3);
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let mut saved = mob_tags(&host, |_| true, &originals);
        saved.retain(|(saved, _)| *saved == id);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved.remove(0).1));
        let show = shows(&host)[&id].family.unwrap();
        PolarBear::show_on(friend.family_mob_mut::<PolarBear>(id).unwrap(), &show);
        for partial in [0.0, 0.25, 0.5, 1.0] {
            assert_eq!(
                positions(drawn(&friend, &atlas, partial).1),
                positions(drawn(&host, &atlas, partial).1),
                "the friend draws the same pose at partial {partial}"
            );
        }
    }
}
