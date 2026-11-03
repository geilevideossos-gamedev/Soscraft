use crate::client_mobs::{ClientMobs, MobPose, quadruped_legs};
use crate::families::FamilyDraw;
use crate::keyframes::{AnimationLibrary, DEGREES, Definition, Moved};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::land::foxes::{
    CROUCHING, FACEPLANTED, Fox, FoxVariant, POUNCING, SITTING, SLEEPING,
};
use minecraftoss_player::mth::cos;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_6, PI};
use std::sync::Arc;

pub const FLIP_DEGREES: f32 = 90.0;
pub const ANIMATION: &str = "FoxBabyAnimation";
pub const BABY_WALK: &str = "FOX_BABY_WALK";
pub const WALK_SPEED: f32 = 1.0;
pub const WALK_SCALE: f32 = 2.5;
pub const HEAD_ROLL: f32 = 0.11;
pub const BABY_AGE_SCALE: f32 = 0.5;
pub const LEG_GROW: f32 = 0.001;
pub const SLEEPING_HEAD_YAW: f32 = -2.094_395_2;
pub const CROUCH_TILT: f32 = 0.104_719_76;
pub const LEG_MOTION_STEP: f32 = 0.67;
pub const FRAMES_PER_TICK: f32 = 3.0;

pub const TEXTURES: [&str; 8] = [
    "minecraft:entity/fox/fox",
    "minecraft:entity/fox/fox_sleep",
    "minecraft:entity/fox/fox_snow",
    "minecraft:entity/fox/fox_snow_sleep",
    "minecraft:entity/fox/fox_baby",
    "minecraft:entity/fox/fox_sleep_baby",
    "minecraft:entity/fox/fox_snow_baby",
    "minecraft:entity/fox/fox_snow_sleep_baby",
];

pub const HEAD: usize = 0;
pub const BODY: usize = 1;
pub const TAIL: usize = 2;
pub const RIGHT_HIND_LEG: usize = 3;
pub const LEFT_HIND_LEG: usize = 4;
pub const RIGHT_FRONT_LEG: usize = 5;
pub const LEFT_FRONT_LEG: usize = 6;
pub const LEGS: [usize; 4] = [
    RIGHT_HIND_LEG,
    LEFT_HIND_LEG,
    RIGHT_FRONT_LEG,
    LEFT_FRONT_LEG,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub name: &'static str,
    pub parent: Option<usize>,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub cubes: &'static [Cube],
    pub grow: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    pub parts: &'static [Part],
    pub texture_size: [f32; 2],
    pub baby: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartState {
    pub offset: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub visible: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stance {
    pub x_rot: f32,
    pub y_rot: f32,
    pub head_roll: f32,
    pub crouch: f32,
    pub age: f32,
    pub walk_position: f32,
    pub walk_speed: f32,
    pub flags: u8,
}

impl Stance {
    pub fn flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }
}

impl PartState {
    fn of(part: &Part) -> PartState {
        PartState {
            offset: Vec3::from_array(part.offset),
            rotation: Vec3::from_array(part.rotation),
            scale: Vec3::ONE,
            visible: true,
        }
    }
}

const fn part(
    name: &'static str,
    parent: Option<usize>,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &'static [Cube],
) -> Part {
    Part {
        name,
        parent,
        offset,
        rotation,
        cubes,
        grow: 0.0,
    }
}

const fn leg(name: &'static str, offset: [f32; 3], cubes: &'static [Cube], grow: f32) -> Part {
    Part {
        name,
        parent: None,
        offset,
        rotation: NONE,
        cubes,
        grow,
    }
}

const NONE: [f32; 3] = [0.0; 3];

const ADULT_HEAD: [Cube; 1] = [Cube::new([-3.0, -2.0, -5.0], [8.0, 6.0, 6.0], [1.0, 5.0])];
const ADULT_BODY: [Cube; 1] = [Cube::new(
    [-3.0, 3.999, -3.5],
    [6.0, 11.0, 6.0],
    [24.0, 15.0],
)];
const ADULT_TAIL: [Cube; 1] = [Cube::new([2.0, 0.0, -1.0], [4.0, 9.0, 5.0], [30.0, 0.0])];
const ADULT_RIGHT_LEG: [Cube; 1] = [Cube::new([2.0, 0.5, -1.0], [2.0, 6.0, 2.0], [13.0, 24.0])];
const ADULT_LEFT_LEG: [Cube; 1] = [Cube::new([2.0, 0.5, -1.0], [2.0, 6.0, 2.0], [4.0, 24.0])];
const ADULT_RIGHT_EAR: [Cube; 1] = [Cube::new([-3.0, -4.0, -4.0], [2.0, 2.0, 1.0], [8.0, 1.0])];
const ADULT_LEFT_EAR: [Cube; 1] = [Cube::new([3.0, -4.0, -4.0], [2.0, 2.0, 1.0], [15.0, 1.0])];
const ADULT_NOSE: [Cube; 1] = [Cube::new([-1.0, 2.01, -8.0], [4.0, 2.0, 3.0], [6.0, 18.0])];

const ADULT_PARTS: [Part; 10] = [
    part("head", None, [-1.0, 16.5, -3.0], NONE, &ADULT_HEAD),
    part(
        "body",
        None,
        [0.0, 16.0, -6.0],
        [FRAC_PI_2, 0.0, 0.0],
        &ADULT_BODY,
    ),
    part(
        "tail",
        Some(BODY),
        [-4.0, 15.0, -1.0],
        [-0.052_359_88, 0.0, 0.0],
        &ADULT_TAIL,
    ),
    leg(
        "right_hind_leg",
        [-5.0, 17.5, 7.0],
        &ADULT_RIGHT_LEG,
        LEG_GROW,
    ),
    leg(
        "left_hind_leg",
        [-1.0, 17.5, 7.0],
        &ADULT_LEFT_LEG,
        LEG_GROW,
    ),
    leg(
        "right_front_leg",
        [-5.0, 17.5, 0.0],
        &ADULT_RIGHT_LEG,
        LEG_GROW,
    ),
    leg(
        "left_front_leg",
        [-1.0, 17.5, 0.0],
        &ADULT_LEFT_LEG,
        LEG_GROW,
    ),
    part("right_ear", Some(HEAD), NONE, NONE, &ADULT_RIGHT_EAR),
    part("left_ear", Some(HEAD), NONE, NONE, &ADULT_LEFT_EAR),
    part("nose", Some(HEAD), NONE, NONE, &ADULT_NOSE),
];

const BABY_HEAD: [Cube; 4] = [
    Cube::new([-3.0, -2.125, -5.125], [6.0, 5.0, 5.0], [0.0, 0.0]),
    Cube::new([-1.0, 0.875, -7.125], [2.0, 2.0, 2.0], [18.0, 20.0]),
    Cube::new([-3.0, -4.125, -4.125], [2.0, 2.0, 1.0], [22.0, 8.0]),
    Cube::new([1.0, -4.125, -4.125], [2.0, 2.0, 1.0], [22.0, 11.0]),
];
const BABY_BODY: [Cube; 1] = [Cube::new([-2.5, -2.0, -3.0], [5.0, 4.0, 6.0], [0.0, 10.0])];
const BABY_TAIL: [Cube; 1] = [Cube::new([-1.5, -1.48, -1.0], [3.0, 3.0, 6.0], [0.0, 20.0])];
const BABY_RIGHT_LEG: [Cube; 1] = [Cube::new([-1.0, 0.0, -1.0], [2.0, 2.0, 2.0], [22.0, 4.0])];
const BABY_LEFT_LEG: [Cube; 1] = [Cube::new([-1.0, 0.0, -1.0], [2.0, 2.0, 2.0], [22.0, 0.0])];

const BABY_PARTS: [Part; 7] = [
    part("head", None, [0.0, 18.125, 0.125], NONE, &BABY_HEAD),
    part("body", None, [0.0, 20.0, 2.0], NONE, &BABY_BODY),
    part("tail", Some(BODY), [0.0, -0.5, 3.0], NONE, &BABY_TAIL),
    leg("right_hind_leg", [-1.5, 22.0, 4.0], &BABY_RIGHT_LEG, 0.0),
    leg("left_hind_leg", [1.5, 22.0, 4.0], &BABY_LEFT_LEG, 0.0),
    leg("right_front_leg", [-1.5, 22.0, 0.0], &BABY_RIGHT_LEG, 0.0),
    leg("left_front_leg", [1.5, 22.0, 0.0], &BABY_LEFT_LEG, 0.0),
];

pub const ADULT: Model = Model {
    parts: &ADULT_PARTS,
    texture_size: [48.0, 32.0],
    baby: false,
};

pub const BABY: Model = Model {
    parts: &BABY_PARTS,
    texture_size: [32.0, 32.0],
    baby: true,
};

pub fn model(baby: bool) -> &'static Model {
    if baby { &BABY } else { &ADULT }
}

pub fn texture(variant: FoxVariant, sleeping: bool, baby: bool) -> &'static str {
    let snow = usize::from(variant == FoxVariant::Snow);
    TEXTURES[snow * 2 + usize::from(sleeping) + usize::from(baby) * 4]
}

pub fn baby_walk(library: &AnimationLibrary) -> Option<Arc<Definition>> {
    library.definition(ANIMATION, BABY_WALK)
}

impl Model {
    pub fn age_scale(&self) -> f32 {
        if self.baby { BABY_AGE_SCALE } else { 1.0 }
    }
}

fn walking_pose(
    model: &Model,
    stance: &Stance,
    walk: Option<&Definition>,
    parts: &mut [PartState],
) {
    parts[HEAD].rotation.z = stance.head_roll;
    if !model.baby {
        let swings = quadruped_legs(stance.walk_position, stance.walk_speed);
        for (leg, swing) in LEGS.into_iter().zip(swings) {
            parts[leg].rotation.x = swing;
        }
        return;
    }
    let Some(walk) = walk else {
        return;
    };
    let mut moved = Moved::default();
    walk.apply_walk(
        stance.walk_position,
        stance.walk_speed,
        WALK_SPEED,
        WALK_SCALE,
        &mut moved,
    );
    for (part, state) in model.parts.iter().zip(parts.iter_mut()) {
        let offsets = moved.get(part.name);
        state.offset += offsets.position;
        state.rotation += offsets.rotation;
        state.scale += offsets.scale;
    }
}

fn crouching_pose(model: &Model, stance: &Stance, parts: &mut [PartState]) {
    parts[BODY].rotation.x += CROUCH_TILT;
    parts[HEAD].offset.y += stance.crouch * model.age_scale();
    let wiggle = cos(f64::from(stance.age)) * 0.05;
    parts[BODY].rotation.y = wiggle;
    parts[RIGHT_HIND_LEG].rotation.z = wiggle;
    parts[LEFT_HIND_LEG].rotation.z = wiggle;
    parts[RIGHT_FRONT_LEG].rotation.z = wiggle / 2.0;
    parts[LEFT_FRONT_LEG].rotation.z = wiggle / 2.0;
    parts[BODY].offset.y += if model.baby {
        stance.crouch / 6.0
    } else {
        stance.crouch
    };
}

fn sleeping_pose(model: &Model, parts: &mut [PartState]) {
    for leg in LEGS {
        parts[leg].visible = false;
    }
    parts[BODY].rotation.z = -FRAC_PI_2;
    if model.baby {
        parts[BODY].rotation.x = -0.174_532_92;
        parts[BODY].offset += Vec3::new(-1.5, 1.5, -1.5);
        parts[TAIL].rotation.x = -2.181_661_6;
        parts[TAIL].offset += Vec3::new(-0.7, 0.9, 0.6);
        parts[HEAD].offset += Vec3::new(-2.0, 2.8, -4.0);
    } else {
        parts[BODY].offset.y += 5.0;
        parts[TAIL].rotation.x = -2.617_993_8;
        parts[HEAD].offset += Vec3::new(2.0, 2.99, 0.0);
    }
    parts[HEAD].rotation.y = SLEEPING_HEAD_YAW;
    parts[HEAD].rotation.z = 0.0;
}

fn sitting_pose(model: &Model, parts: &mut [PartState]) {
    parts[HEAD].rotation.x = 0.0;
    parts[HEAD].rotation.y = 0.0;
    let front = [RIGHT_FRONT_LEG, LEFT_FRONT_LEG];
    let hind = [RIGHT_HIND_LEG, LEFT_HIND_LEG];
    for leg in front {
        parts[leg].rotation.x = -0.261_799_4;
    }
    if model.baby {
        let scale = model.age_scale();
        parts[BODY].rotation.x = -0.959_931;
        parts[BODY].offset.z -= 4.5 * scale;
        parts[BODY].offset.y += 3.0 * scale;
        parts[TAIL].offset.y -= 0.6;
        parts[TAIL].offset.z -= 2.0 * scale;
        parts[TAIL].rotation.x = 0.959_931_14;
        parts[HEAD].offset.y -= 0.75;
        for leg in front {
            parts[leg].offset.z -= 1.5;
        }
        for leg in hind {
            parts[leg].offset.z -= 3.75;
        }
        for leg in [RIGHT_FRONT_LEG, RIGHT_HIND_LEG] {
            parts[leg].offset.x += 0.01;
        }
        for leg in [LEFT_FRONT_LEG, LEFT_HIND_LEG] {
            parts[leg].offset.x -= 0.01;
        }
    } else {
        parts[BODY].rotation.x = FRAC_PI_6;
        parts[BODY].offset.y -= 7.0;
        parts[BODY].offset.z += 3.0;
        parts[TAIL].rotation.x = FRAC_PI_4;
        parts[HEAD].offset.y -= 6.5;
        parts[HEAD].offset.z += 2.75;
        for leg in hind {
            parts[leg].rotation.x = -1.308_996_9;
            parts[leg].offset.y += 4.0;
            parts[leg].offset.z -= 0.25;
        }
        parts[TAIL].offset.z -= 1.0;
    }
}

fn pouncing_pose(model: &Model, stance: &Stance, parts: &mut [PartState]) {
    if model.baby {
        return;
    }
    let drop = stance.crouch / 2.0;
    parts[BODY].offset.y -= drop;
    parts[HEAD].offset.y -= drop;
}

pub fn leg_motion(age: f32) -> f32 {
    age * LEG_MOTION_STEP * FRAMES_PER_TICK
}

pub fn setup_anim(model: &Model, stance: &Stance, walk: Option<&Definition>) -> Vec<PartState> {
    let mut parts: Vec<PartState> = model.parts.iter().map(PartState::of).collect();
    walking_pose(model, stance, walk, &mut parts);
    if stance.flag(CROUCHING) {
        crouching_pose(model, stance, &mut parts);
    } else if stance.flag(SLEEPING) {
        sleeping_pose(model, &mut parts);
    } else if stance.flag(SITTING) {
        sitting_pose(model, &mut parts);
    }
    if stance.flag(POUNCING) {
        pouncing_pose(model, stance, &mut parts);
    }
    let still = stance.flag(SLEEPING) || stance.flag(FACEPLANTED) || stance.flag(CROUCHING);
    if !still {
        parts[HEAD].rotation.x = stance.x_rot * DEGREES;
        parts[HEAD].rotation.y = stance.y_rot * DEGREES;
    }
    if stance.flag(SLEEPING) {
        parts[HEAD].rotation.x = 0.0;
        parts[HEAD].rotation.y = SLEEPING_HEAD_YAW;
        parts[HEAD].rotation.z = cos(f64::from(stance.age * 0.027)) / 22.0;
    }
    if stance.flag(FACEPLANTED) {
        let motion = leg_motion(stance.age) * 0.4662;
        let a = cos(f64::from(motion)) * 0.1;
        let b = cos(f64::from(motion + PI)) * 0.1;
        for (leg, swing) in LEGS.into_iter().zip([a, b, b, a]) {
            parts[leg].rotation.x = swing;
        }
    }
    parts
}

pub fn part_poses(model: &Model, parts: &[PartState]) -> Vec<Mat4> {
    let mut out: Vec<Mat4> = Vec::with_capacity(parts.len());
    for (part, state) in model.parts.iter().zip(parts) {
        let local = part_pose(state.offset.to_array(), state.rotation.to_array())
            * Mat4::from_scale(state.scale);
        let pose = part.parent.map_or(local, |parent| out[parent] * local);
        out.push(pose);
    }
    out
}

pub fn shown(model: &Model, parts: &[PartState], index: usize) -> bool {
    let mut at = Some(index);
    while let Some(part) = at {
        if !parts[part].visible {
            return false;
        }
        at = model.parts[part].parent;
    }
    true
}

pub fn grown(cube: &Cube, grow: f32) -> Mat4 {
    if grow == 0.0 {
        return Mat4::IDENTITY;
    }
    let size = Vec3::from_array(cube.size);
    let centre = (Vec3::from_array(cube.from) + size / 2.0) / 16.0;
    Mat4::from_translation(centre)
        * Mat4::from_scale((size + 2.0 * grow) / size)
        * Mat4::from_translation(-centre)
}

pub fn model_root(pose: &MobPose, flags: u8) -> Mat4 {
    let mut root = Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.body_rotation(FLIP_DEGREES));
    if flags & (POUNCING | FACEPLANTED) != 0 {
        root *= Mat4::from_rotation_x((-pose.head_pitch).to_radians());
    }
    root * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn stance(pose: &MobPose, fox: &Fox, partial: f32) -> Stance {
    let lerp = |old: f32, now: f32| old + partial * (now - old);
    Stance {
        x_rot: pose.head_pitch,
        y_rot: pose.head_yaw,
        head_roll: lerp(fox.interested_angle_old, fox.interested_angle) * HEAD_ROLL * PI,
        crouch: lerp(fox.crouch_amount_old, fox.crouch_amount),
        age: pose.age_in_ticks,
        walk_position: pose.walk_position,
        walk_speed: pose.walk_speed,
        flags: fox.flags(),
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
    parts: &[PartState],
    skin: &Skin,
) {
    let poses = part_poses(model, parts);
    for (index, (part, pose)) in model.parts.iter().zip(&poses).enumerate() {
        if !shown(model, parts, index) {
            continue;
        }
        for cube in part.cubes {
            append(mesh, root * *pose * grown(cube, part.grow), skin, cube);
        }
    }
}

pub fn append_foxes<'a>(
    out: &mut FamilyDraw<'_>,
    foxes: impl IntoIterator<Item = &'a FamilyMob<Fox>>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
    walk: Option<&Definition>,
) {
    let mut marks = Vec::new();
    for fox in foxes {
        let Some(pose) = poses.pose(fox.core.id, partial) else {
            continue;
        };
        marks.push((out.culled.vertices.len(), pose.overlay(0.0)));
        let baby = fox.core.baby;
        let model = model(baby);
        let stance = stance(&pose, &fox.state, partial);
        let texture = texture(fox.state.variant, stance.flag(SLEEPING), baby);
        let skin = skin(atlas, texture, model.texture_size, light, &pose);
        let parts = setup_anim(model, &stance, walk);
        let root = model_root(&pose, stance.flags);
        append_model(out.culled, root, model, &parts, &skin);
    }
    crate::cow_render::apply_overlays(out.culled, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use crate::families::TerrainFamily;
    use crate::mesh::HeldItem;
    use crate::mob_tracking::{shows, spawn_tracked};
    use crate::server_mobs::{mob_tags, spawn_saved};
    use glam::DVec3;
    use minecraftoss_core::nbt::Tag;
    use minecraftoss_entities::world::EntityWorld;
    use minecraftoss_entities::world::families::land::foxes::INTERESTED;
    use std::collections::HashMap;

    const NAMES: [&str; 7] = [
        "head",
        "body",
        "tail",
        "right_hind_leg",
        "left_hind_leg",
        "right_front_leg",
        "left_front_leg",
    ];

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let skins = root.join("assets/minecraft/textures/entity/fox");
        std::fs::create_dir_all(&skins).unwrap();
        for texture in TEXTURES {
            let name = texture.rsplit('/').next().unwrap();
            let width = if name.ends_with("_baby") { 32 } else { 48 };
            image::RgbaImage::from_pixel(width, 32, image::Rgba([220, 120, 40, 255]))
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

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    fn drawn(world: &EntityWorld, atlas: &Atlas, partial: f32) -> (ChunkMesh, Vec<HeldItem>) {
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(world));
        let mut models = ChunkMesh::default();
        let mut culled = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let mut held = Vec::new();
        Fox::draw(
            &mut FamilyDraw {
                models: &mut models,
                culled: &mut culled,
                translucent: &mut translucent,
                held: &mut held,
            },
            world.family::<Fox>(),
            &poses,
            atlas,
            &light(),
            partial,
        );
        assert!(models.vertices.is_empty(), "entityCutout culls back faces");
        assert!(translucent.vertices.is_empty());
        (culled, held)
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

    fn region(atlas: &Atlas, texture: &str) -> [f32; 4] {
        atlas.entity_region(&ResourceId::parse(texture).unwrap())
    }

    fn positions(mesh: &ChunkMesh) -> Vec<[f32; 3]> {
        mesh.vertices.iter().map(|v| v.position).collect()
    }

    fn with(flags: u8) -> Stance {
        Stance {
            flags,
            ..Stance::default()
        }
    }

    #[test]
    fn an_adult_and_a_cub_have_their_parts_and_textures() {
        let cubes = |model: &Model| model.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
        assert_eq!((ADULT.parts.len(), cubes(&ADULT)), (10, 10));
        assert_eq!((BABY.parts.len(), cubes(&BABY)), (7, 10));
        assert_eq!(
            (ADULT.texture_size, BABY.texture_size),
            ([48.0, 32.0], [32.0, 32.0])
        );
        for model in [&ADULT, &BABY] {
            let names: Vec<&str> = model.parts.iter().map(|part| part.name).collect();
            assert_eq!(names[..7], NAMES);
            for (index, part) in model.parts.iter().enumerate() {
                assert!(part.parent.is_none_or(|parent| parent < index));
                assert!(part.cubes.iter().all(|cube| !cube.mirror));
            }
            assert_eq!(
                model.parts[TAIL].parent,
                Some(BODY),
                "the tail is the body's"
            );
        }
        let ears: Vec<&str> = ADULT.parts[7..].iter().map(|part| part.name).collect();
        assert_eq!(ears, ["right_ear", "left_ear", "nose"]);
        assert!(
            ADULT.parts[7..]
                .iter()
                .all(|part| part.parent == Some(HEAD))
        );
        assert_eq!(BABY.parts[HEAD].cubes.len(), 4, "the cub's ears and nose");
        for leg in LEGS {
            assert_eq!(
                (ADULT.parts[leg].grow, BABY.parts[leg].grow),
                (LEG_GROW, 0.0)
            );
        }
        let leg = &ADULT.parts[RIGHT_HIND_LEG].cubes[0];
        let corner = grown(leg, LEG_GROW).transform_point3(Vec3::new(2.0, 0.5, -1.0) / 16.0);
        let wanted = Vec3::new(1.999, 0.499, -1.001);
        assert!((corner * 16.0).abs_diff_eq(wanted, 1e-4), "{corner}");
        assert_eq!(grown(leg, 0.0), Mat4::IDENTITY);
        assert_eq!(model(false), &ADULT);
        assert_eq!(model(true), &BABY);
        assert_eq!((ADULT.age_scale(), BABY.age_scale()), (1.0, 0.5));

        assert_eq!(
            texture(FoxVariant::Red, false, false),
            "minecraft:entity/fox/fox"
        );
        assert_eq!(
            texture(FoxVariant::Red, true, false),
            "minecraft:entity/fox/fox_sleep"
        );
        assert_eq!(
            texture(FoxVariant::Snow, false, false),
            "minecraft:entity/fox/fox_snow"
        );
        assert_eq!(
            texture(FoxVariant::Snow, true, true),
            "minecraft:entity/fox/fox_snow_sleep_baby"
        );
        assert_eq!(
            texture(FoxVariant::Red, true, true),
            "minecraft:entity/fox/fox_sleep_baby"
        );
        assert_eq!(Fox::TEXTURES, TEXTURES);
        let mut unique = TEXTURES.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 8);

        let still = part_poses(&ADULT, &setup_anim(&ADULT, &Stance::default(), None));
        assert!(close(pivot(still[HEAD]), Vec3::new(-1.0, 16.5, -3.0)));
        assert!(close(pivot(still[BODY]), Vec3::new(0.0, 16.0, -6.0)));
        assert!(
            close(still[BODY].transform_vector3(Vec3::Y), Vec3::Z),
            "the body lies along the fox"
        );
        assert!(close(pivot(still[TAIL]), Vec3::new(-4.0, 17.0, 9.0)));
        assert!(close(pivot(still[7]), pivot(still[HEAD])));

        let atlas = atlas();
        let mut world = EntityWorld::default();
        let adult = spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d]}"),
        )
        .unwrap();
        let (culled, held) = drawn(&world, &atlas, 0.0);
        assert!(held.is_empty(), "an empty mouth");
        assert_eq!(culled.faces, 60, "ten cubes");
        let (low, high) = span(&culled, 1);
        assert!((low - 64.0).abs() < 0.01, "the paws on the ground: {low}");
        assert!((high - 64.719_75).abs() < 0.01, "the ears on top: {high}");
        let (_, south) = span(&culled, 2);
        assert!(
            (south - (0.5 + 11.0 / 16.0)).abs() < 0.01,
            "the nose leads: {south}"
        );
        assert!(inside(region(&atlas, TEXTURES[0]), &culled));
        assert!(
            culled
                .vertices
                .iter()
                .all(|v| v.sky_light == 15.0 && v.block_light == 2.0 && v.color[3] > 0.0)
        );

        world.family_core_mut(adult).unwrap().1.removed = true;
        spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d],Age:-24000,Type:\"snow\"}"),
        )
        .unwrap();
        let (cub, _) = drawn(&world, &atlas, 0.0);
        assert_eq!(
            cub.faces, 60,
            "the cub's ten cubes, the adult that left not drawn"
        );
        let (low, high) = span(&cub, 1);
        assert!((low - 64.0).abs() < 0.01, "{low}");
        assert!((high - 64.626).abs() < 0.01, "the cub's own model: {high}");
        assert!(inside(region(&atlas, TEXTURES[6]), &cub));
    }

    #[test]
    fn a_sleeping_fox_hides_its_legs_and_closes_its_eyes() {
        let asleep = Stance {
            x_rot: 20.0,
            y_rot: 30.0,
            age: 40.0,
            head_roll: 0.3,
            ..with(SLEEPING)
        };
        let parts = setup_anim(&ADULT, &asleep, None);
        for leg in LEGS {
            assert!(!parts[leg].visible && !shown(&ADULT, &parts, leg));
        }
        assert!(shown(&ADULT, &parts, 7), "its ears stay");
        let head = parts[HEAD];
        assert_eq!(head.rotation.x, 0.0, "the look does not lift it");
        assert_eq!(head.rotation.y, SLEEPING_HEAD_YAW);
        assert!(near(
            head.rotation.z,
            cos(f64::from(40.0_f32 * 0.027)) / 22.0
        ));
        assert!(close(head.offset, Vec3::new(1.0, 19.49, -3.0)));
        assert_eq!(parts[BODY].rotation, Vec3::new(FRAC_PI_2, 0.0, -FRAC_PI_2));
        assert!(close(parts[BODY].offset, Vec3::new(0.0, 21.0, -6.0)));
        assert_eq!(parts[TAIL].rotation.x, -2.617_993_8);

        let cub = setup_anim(&BABY, &asleep, None);
        assert!(LEGS.iter().all(|&leg| !cub[leg].visible));
        assert_eq!(
            cub[BODY].rotation,
            Vec3::new(-0.174_532_92, 0.0, -FRAC_PI_2)
        );
        assert!(close(cub[BODY].offset, Vec3::new(-1.5, 21.5, 0.5)));
        assert_eq!(cub[TAIL].rotation.x, -2.181_661_6);
        assert!(close(cub[TAIL].offset, Vec3::new(-0.7, 0.4, 3.6)));
        assert!(close(cub[HEAD].offset, Vec3::new(-2.0, 20.925, -3.875)));
        assert_eq!(cub[HEAD].rotation.y, SLEEPING_HEAD_YAW);

        let atlas = atlas();
        let mut world = EntityWorld::default();
        spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d],Sleeping:1b}"),
        )
        .unwrap();
        let (culled, _) = drawn(&world, &atlas, 0.0);
        assert_eq!(culled.faces, 36, "no legs");
        let (low, high) = span(&culled, 1);
        assert!((low - 64.0).abs() < 0.01, "curled up on the ground: {low}");
        assert!(high < 64.6, "lower than a standing fox: {high}");
        assert!(inside(region(&atlas, TEXTURES[1]), &culled), "eyes shut");
        let mut world = EntityWorld::default();
        spawn_saved(
            &mut world,
            &parse(
                "{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d],Sleeping:1b,Age:-24000,Type:\"snow\"}",
            ),
        )
        .unwrap();
        let (cub, _) = drawn(&world, &atlas, 0.0);
        assert_eq!(cub.faces, 36);
        assert!(inside(region(&atlas, TEXTURES[7]), &cub));
    }

    #[test]
    fn a_crouching_fox_lowers_its_head_by_its_crouch_clock() {
        let crouched = Stance {
            x_rot: 20.0,
            y_rot: 30.0,
            age: 7.0,
            crouch: 2.5,
            head_roll: 0.2,
            ..with(CROUCHING)
        };
        let parts = setup_anim(&ADULT, &crouched, None);
        let wiggle = cos(7.0) * 0.05;
        assert!(close(parts[HEAD].offset, Vec3::new(-1.0, 19.0, -3.0)));
        assert_eq!(
            parts[HEAD].rotation,
            Vec3::new(0.0, 0.0, 0.2),
            "it keeps its eyes on its prey, its head cocked"
        );
        assert!(near(parts[BODY].rotation.x, FRAC_PI_2 + CROUCH_TILT));
        assert_eq!(parts[BODY].rotation.y, wiggle);
        assert!(close(parts[BODY].offset, Vec3::new(0.0, 18.5, -6.0)));
        for (leg, sway) in LEGS.into_iter().zip([1.0, 1.0, 0.5, 0.5]) {
            assert!(near(parts[leg].rotation.z, wiggle * sway));
        }
        let cub = setup_anim(&BABY, &crouched, None);
        assert!(close(cub[HEAD].offset, Vec3::new(0.0, 19.375, 0.125)));
        assert!(close(
            cub[BODY].offset,
            Vec3::new(0.0, 20.0 + 2.5 / 6.0, 2.0)
        ));
        let standing = setup_anim(
            &ADULT,
            &Stance {
                flags: 0,
                ..crouched
            },
            None,
        );
        assert_eq!(
            standing[HEAD].rotation,
            Vec3::new(20.0 * DEGREES, 30.0 * DEGREES, 0.2)
        );

        let atlas = atlas();
        let mut world = EntityWorld::default();
        let id = spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d]}"),
        )
        .unwrap();
        let (up, _) = drawn(&world, &atlas, 0.5);
        let mob = world.family_mob_mut::<Fox>(id).unwrap();
        mob.state.goals_mut().flags = CROUCHING | INTERESTED;
        mob.state.crouch_amount_old = 4.8;
        mob.state.crouch_amount = 5.0;
        mob.state.interested_angle_old = 0.4;
        mob.state.interested_angle = 0.64;
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(&world));
        let pose = poses.pose(id, 0.5).unwrap();
        let seen = stance(&pose, &world.family::<Fox>()[0].state, 0.5);
        assert!(near(seen.crouch, 4.9), "the crouch clock lerps");
        assert!(
            near(seen.head_roll, 0.52 * HEAD_ROLL * PI),
            "the tilt lerps"
        );
        let parts = setup_anim(&ADULT, &seen, None);
        assert!(close(parts[HEAD].offset, Vec3::new(-1.0, 16.5 + 4.9, -3.0)));
        let (down, _) = drawn(&world, &atlas, 0.5);
        assert_ne!(positions(&down), positions(&up));
        let mut expected = ChunkMesh::default();
        let skin = skin(&atlas, TEXTURES[0], ADULT.texture_size, &light(), &pose);
        let root = model_root(&pose, seen.flags);
        append_model(&mut expected, root, &ADULT, &parts, &skin);
        assert_eq!(positions(&down), positions(&expected), "drawn as set up");
    }

    #[test]
    fn a_sitting_pouncing_or_faceplanted_fox_takes_vanilla_s_pose() {
        let look = Stance {
            x_rot: 10.0,
            y_rot: -40.0,
            age: 13.0,
            crouch: 5.0,
            ..Stance::default()
        };
        let sitting = setup_anim(
            &ADULT,
            &Stance {
                flags: SITTING,
                ..look
            },
            None,
        );
        assert_eq!(sitting[BODY].rotation.x, FRAC_PI_6);
        assert!(close(sitting[BODY].offset, Vec3::new(0.0, 9.0, -3.0)));
        assert_eq!(sitting[TAIL].rotation.x, FRAC_PI_4);
        assert!(close(sitting[TAIL].offset, Vec3::new(-4.0, 15.0, -2.0)));
        assert!(close(sitting[HEAD].offset, Vec3::new(-1.0, 10.0, -0.25)));
        assert_eq!(
            sitting[HEAD].rotation,
            Vec3::new(10.0 * DEGREES, -40.0 * DEGREES, 0.0),
            "a sitting fox looks about"
        );
        for leg in [RIGHT_FRONT_LEG, LEFT_FRONT_LEG] {
            assert_eq!(sitting[leg].rotation.x, -0.261_799_4);
        }
        for leg in [RIGHT_HIND_LEG, LEFT_HIND_LEG] {
            assert_eq!(sitting[leg].rotation.x, -1.308_996_9);
            assert!(close(
                sitting[leg].offset,
                Vec3::from_array(ADULT.parts[leg].offset) + Vec3::new(0.0, 4.0, -0.25)
            ));
        }
        let cub = setup_anim(
            &BABY,
            &Stance {
                flags: SITTING,
                ..look
            },
            None,
        );
        assert_eq!(cub[BODY].rotation.x, -0.959_931);
        assert!(close(cub[BODY].offset, Vec3::new(0.0, 21.5, -0.25)));
        assert!(close(cub[TAIL].offset, Vec3::new(0.0, -1.1, 2.0)));
        assert_eq!(cub[TAIL].rotation.x, 0.959_931_14);
        assert!(close(cub[HEAD].offset, Vec3::new(0.0, 17.375, 0.125)));
        assert!(close(
            cub[RIGHT_FRONT_LEG].offset,
            Vec3::new(-1.49, 22.0, -1.5)
        ));
        assert!(close(
            cub[LEFT_HIND_LEG].offset,
            Vec3::new(1.49, 22.0, 0.25)
        ));

        let pouncing = setup_anim(
            &ADULT,
            &Stance {
                flags: POUNCING,
                ..look
            },
            None,
        );
        assert!(close(pouncing[BODY].offset, Vec3::new(0.0, 13.5, -6.0)));
        assert!(close(pouncing[HEAD].offset, Vec3::new(-1.0, 14.0, -3.0)));
        let cub = setup_anim(
            &BABY,
            &Stance {
                flags: POUNCING,
                ..look
            },
            None,
        );
        assert_eq!(
            cub[BODY].offset,
            Vec3::new(0.0, 20.0, 2.0),
            "a cub does not drop"
        );

        let planted = setup_anim(
            &ADULT,
            &Stance {
                flags: FACEPLANTED,
                ..look
            },
            None,
        );
        assert_eq!(planted[HEAD].rotation, Vec3::ZERO, "head down in the snow");
        let motion = leg_motion(13.0) * 0.4662;
        let a = cos(f64::from(motion)) * 0.1;
        let b = cos(f64::from(motion + PI)) * 0.1;
        let swings: Vec<f32> = LEGS.iter().map(|&leg| planted[leg].rotation.x).collect();
        assert_eq!(swings, [a, b, b, a], "the legs paddle");
        let later = setup_anim(
            &ADULT,
            &Stance {
                age: 14.0,
                flags: FACEPLANTED,
                ..look
            },
            None,
        );
        assert_ne!(later[RIGHT_HIND_LEG], planted[RIGHT_HIND_LEG]);
        assert!(near(leg_motion(1.0), 2.01));

        let mut world = EntityWorld::default();
        let id = spawn_saved(
            &mut world,
            &parse("{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d],Rotation:[30.0f,0.0f]}"),
        )
        .unwrap();
        world.family_mob_mut::<Fox>(id).unwrap().core.look.pitch = 25.0;
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(&world));
        let pose = poses.pose(id, 0.0).unwrap();
        assert!(pose.head_pitch > 20.0, "packed: {}", pose.head_pitch);
        let level = model_root(&pose, 0);
        assert_eq!(model_root(&pose, SITTING | CROUCHING), level);
        for flags in [POUNCING, FACEPLANTED] {
            let tipped = Mat4::from_translation(pose.feet.as_vec3())
                * Mat4::from_quat(pose.body_rotation(FLIP_DEGREES))
                * Mat4::from_rotation_x((-pose.head_pitch).to_radians())
                * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
            assert!(
                model_root(&pose, flags).abs_diff_eq(tipped, 1e-6),
                "{flags}"
            );
        }
    }

    #[test]
    fn a_fox_s_lead_ties_at_its_neck() {
        let offset = crate::leash_render::leash_offset("minecraft:fox", 0.4, 0.6);
        assert!(
            offset.abs_diff_eq(DVec3::new(0.0, 0.55 * 0.4, 0.4 * 0.6), 1e-6),
            "{offset}"
        );
        let cub = crate::leash_render::leash_offset("minecraft:fox", 0.34375, 0.36);
        assert!(
            cub.abs_diff_eq(DVec3::new(0.0, 0.55 * 0.34375, 0.4 * 0.36), 1e-6),
            "{cub}"
        );
    }

    #[test]
    fn a_friend_draws_a_fox_as_the_host_does() {
        let atlas = atlas();
        let tag = parse("{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d],Type:\"snow\"}");
        let mut host = EntityWorld::default();
        let id = spawn_saved(&mut host, &tag).unwrap();
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let mut saved = mob_tags(&host, |_| true, &originals);
        saved.retain(|(saved, _)| *saved == id);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved.remove(0).1));
        for flags in [CROUCHING | INTERESTED, SLEEPING, SITTING] {
            let mob = host.family_mob_mut::<Fox>(id).unwrap();
            mob.state.goals_mut().flags = flags;
            for _ in 0..3 {
                host.family_mob_mut::<Fox>(id).unwrap().state.step_angles();
                let show = shows(&host)[&id].family.unwrap();
                Fox::show_on(friend.family_mob_mut::<Fox>(id).unwrap(), &show);
            }
            for partial in [0.0, 0.25, 0.5, 1.0] {
                let (seen, _) = drawn(&friend, &atlas, partial);
                let (mesh, _) = drawn(&host, &atlas, partial);
                assert_eq!(positions(&seen), positions(&mesh), "{flags} at {partial}");
                assert_eq!(seen.vertices[0].uv, mesh.vertices[0].uv, "{flags}");
            }
        }
    }

    fn library() -> Option<AnimationLibrary> {
        let root = std::env::var_os("MINECRAFTOSS_ROOT")?;
        let jar = std::path::Path::new(&root).join("client.jar");
        jar.is_file().then(|| AnimationLibrary::new(Some(jar)))
    }

    #[test]
    fn the_cub_walk_matches_the_jar() {
        let Some(library) = library() else {
            return;
        };
        let walk = baby_walk(&library).expect("FOX_BABY_WALK");
        assert_eq!((walk.length, walk.looping), (0.5, true));
        for (bone, _) in &walk.bones {
            assert!(NAMES.contains(&bone.as_str()), "{bone}");
        }
        let walking = |position: f32, speed: f32| Stance {
            walk_position: position,
            walk_speed: speed,
            ..Stance::default()
        };
        let still = setup_anim(&BABY, &Stance::default(), None);
        assert_eq!(
            setup_anim(&BABY, &walking(3.0, 0.0), Some(&walk)),
            still,
            "no speed, no steps"
        );
        let step = setup_anim(&BABY, &walking(3.0, 0.3), Some(&walk));
        assert_ne!(step, still, "the cub steps with the keyframes");
        let mut moved = Moved::default();
        walk.apply_walk(3.0, 0.3, 1.0, 2.5, &mut moved);
        for (index, part) in BABY.parts.iter().enumerate() {
            let offsets = moved.get(part.name);
            assert!(close(
                step[index].offset,
                still[index].offset + offsets.position
            ));
            assert!(close(
                step[index].rotation,
                still[index].rotation + offsets.rotation
            ));
            assert!(close(step[index].scale, Vec3::ONE + offsets.scale));
        }
        let looped = setup_anim(&BABY, &walking(13.0, 0.3), Some(&walk));
        for (a, b) in looped.iter().zip(&step) {
            assert!(close(a.rotation, b.rotation), "half a second loops");
        }
        let full = setup_anim(&BABY, &walking(3.0, 0.4), Some(&walk));
        let fuller = setup_anim(&BABY, &walking(3.0, 0.9), Some(&walk));
        assert_eq!(full, fuller, "the steps top out at speed 0.4");
        let adult = setup_anim(&ADULT, &walking(3.0, 0.3), Some(&walk));
        let legs: Vec<f32> = LEGS.iter().map(|&leg| adult[leg].rotation.x).collect();
        assert_eq!(legs, quadruped_legs(3.0, 0.3), "adults swing their legs");
        assert_eq!(
            adult[BODY],
            setup_anim(&ADULT, &Stance::default(), None)[BODY]
        );
    }
}
