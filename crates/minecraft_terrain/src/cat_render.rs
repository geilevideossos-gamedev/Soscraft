use crate::client_mobs::{ClientMobs, MobPose};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Quat, Vec3};
use minecraftoss_entities::feline::{CAT_VARIANTS, Feline, FelineKind};
use minecraftoss_entities::world::FelineEntity;
use std::f32::consts::PI;

pub const SHADOW_RADIUS: f32 = 0.4;
pub const COLLAR_TEXTURE: &str = "minecraft:entity/cat/cat_collar";
pub const COLLAR_BABY_TEXTURE: &str = "minecraft:entity/cat/cat_collar_baby";
pub const OCELOT_TEXTURE: &str = "minecraft:entity/cat/ocelot";
pub const OCELOT_BABY_TEXTURE: &str = "minecraft:entity/cat/ocelot_baby";
const CAT_SCALE: f32 = 0.8;
const COLLAR_GROW: f32 = 0.01;
const TAIL2_GROW: f32 = -0.02;
const BABY_COLLAR_SCALE: f32 = 1.01;
const DEGREES: f32 = 0.017_453_292;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Head,
    Body,
    Tail1,
    Tail2,
    LeftHindLeg,
    RightHindLeg,
    LeftFrontLeg,
    RightFrontLeg,
}

const PARTS: [Part; 8] = [
    Part::Head,
    Part::Body,
    Part::Tail1,
    Part::Tail2,
    Part::LeftHindLeg,
    Part::RightHindLeg,
    Part::LeftFrontLeg,
    Part::RightFrontLeg,
];

const ADULT: [(Part, Cube); 11] = [
    (
        Part::Head,
        Cube::new([-2.5, -2.0, -3.0], [5.0, 4.0, 5.0], [0.0, 0.0]),
    ),
    (
        Part::Head,
        Cube::new([-1.5, -0.001, -4.0], [3.0, 2.0, 2.0], [0.0, 24.0]),
    ),
    (
        Part::Head,
        Cube::new([-2.0, -3.0, 0.0], [1.0, 1.0, 2.0], [0.0, 10.0]),
    ),
    (
        Part::Head,
        Cube::new([1.0, -3.0, 0.0], [1.0, 1.0, 2.0], [6.0, 10.0]),
    ),
    (
        Part::Body,
        Cube::new([-2.0, 3.0, -8.0], [4.0, 16.0, 6.0], [20.0, 0.0]),
    ),
    (
        Part::Tail1,
        Cube::new([-0.5, 0.0, 0.0], [1.0, 8.0, 1.0], [0.0, 15.0]),
    ),
    (
        Part::Tail2,
        Cube::new([-0.5, 0.0, 0.0], [1.0, 8.0, 1.0], [4.0, 15.0]),
    ),
    (
        Part::LeftHindLeg,
        Cube::new([-1.0, 0.0, 1.0], [2.0, 6.0, 2.0], [8.0, 13.0]),
    ),
    (
        Part::RightHindLeg,
        Cube::new([-1.0, 0.0, 1.0], [2.0, 6.0, 2.0], [8.0, 13.0]),
    ),
    (
        Part::LeftFrontLeg,
        Cube::new([-1.0, 0.0, 0.0], [2.0, 10.0, 2.0], [40.0, 0.0]),
    ),
    (
        Part::RightFrontLeg,
        Cube::new([-1.0, 0.0, 0.0], [2.0, 10.0, 2.0], [40.0, 0.0]),
    ),
];

const BABY: [(Part, Cube); 10] = [
    (
        Part::Head,
        Cube::new([-2.5, -3.0, -2.875], [5.0, 4.0, 4.0], [0.0, 0.0]),
    ),
    (
        Part::Head,
        Cube::new([-2.0, -4.0, -0.875], [1.0, 1.0, 2.0], [18.0, 0.0]),
    ),
    (
        Part::Head,
        Cube::new([1.0, -4.0, -0.875], [1.0, 1.0, 2.0], [24.0, 0.0]),
    ),
    (
        Part::Head,
        Cube::new([-1.5, -1.0, -3.875], [3.0, 2.0, 1.0], [18.0, 3.0]),
    ),
    (
        Part::LeftFrontLeg,
        Cube::new([-0.5, 0.0, -1.0], [1.0, 2.0, 2.0], [18.0, 18.0]),
    ),
    (
        Part::RightFrontLeg,
        Cube::new([-0.5, 0.0, -1.0], [1.0, 2.0, 2.0], [12.0, 18.0]),
    ),
    (
        Part::LeftHindLeg,
        Cube::new([-0.5, 0.0, -1.0], [1.0, 2.0, 2.0], [18.0, 22.0]),
    ),
    (
        Part::Body,
        Cube::new([-2.0, -1.5, -3.5], [4.0, 3.0, 7.0], [0.0, 8.0]),
    ),
    (
        Part::RightHindLeg,
        Cube::new([-0.5, 0.0, -1.0], [1.0, 2.0, 2.0], [12.0, 22.0]),
    ),
    (
        Part::Tail1,
        Cube::new([-0.5, -0.107, 0.0849], [1.0, 1.0, 5.0], [0.0, 18.0]),
    ),
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PartPose {
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
}

impl PartPose {
    const fn at(offset: [f32; 3], rotation: [f32; 3]) -> Self {
        Self { offset, rotation }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FelineLook {
    pub baby: bool,
    pub crouching: bool,
    pub sprinting: bool,
    pub sitting: bool,
    pub lie_down: f32,
    pub lie_down_tail: f32,
    pub relax: f32,
    pub walk_position: f32,
    pub walk_speed: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
}

pub fn textures() -> Vec<String> {
    let mut out = Vec::new();
    for variant in CAT_VARIANTS {
        out.push(format!("minecraft:entity/cat/cat_{variant}"));
        out.push(format!("minecraft:entity/cat/cat_{variant}_baby"));
    }
    for texture in [
        COLLAR_TEXTURE,
        COLLAR_BABY_TEXTURE,
        OCELOT_TEXTURE,
        OCELOT_BABY_TEXTURE,
    ] {
        out.push(texture.to_owned());
    }
    out
}

fn cos(value: f32) -> f32 {
    minecraftoss_player::mth::cos(f64::from(value))
}

fn rot_lerp(delta: f32, start: f32, end: f32) -> f32 {
    let mut difference = (end - start) % 360.0;
    if difference >= 180.0 {
        difference -= 360.0;
    }
    if difference < -180.0 {
        difference += 360.0;
    }
    start + delta * difference
}

fn index(part: Part) -> usize {
    PARTS.iter().position(|p| *p == part).unwrap_or(0)
}

pub fn feline_pose(look: &FelineLook) -> [PartPose; 8] {
    if look.baby {
        baby_pose(look)
    } else {
        adult_pose(look)
    }
}

fn adult_pose(look: &FelineLook) -> [PartPose; 8] {
    let mut p = [
        PartPose::at([0.0, 15.0, -9.0], [0.0; 3]),
        PartPose::at([0.0, 12.0, -10.0], [PI / 2.0, 0.0, 0.0]),
        PartPose::at([0.0, 15.0, 8.0], [0.9, 0.0, 0.0]),
        PartPose::at([0.0, 20.0, 14.0], [0.0; 3]),
        PartPose::at([1.1, 18.0, 5.0], [0.0; 3]),
        PartPose::at([-1.1, 18.0, 5.0], [0.0; 3]),
        PartPose::at([1.2, 14.1, -5.0], [0.0; 3]),
        PartPose::at([-1.2, 14.1, -5.0], [0.0; 3]),
    ];
    let [
        head,
        body,
        tail1,
        tail2,
        left_hind,
        right_hind,
        left_front,
        right_front,
    ] = &mut p;
    let f = 1.0;
    if look.crouching {
        body.offset[1] += f;
        head.offset[1] += 2.0 * f;
        tail1.offset[1] += f;
        tail2.offset[1] += -4.0 * f;
        tail2.offset[2] += 2.0 * f;
        tail1.rotation[0] = PI / 2.0;
        tail2.rotation[0] = PI / 2.0;
    } else if look.sprinting {
        tail2.offset[1] = tail1.offset[1];
        tail2.offset[2] += 2.0 * f;
        tail1.rotation[0] = PI / 2.0;
        tail2.rotation[0] = PI / 2.0;
    }
    head.rotation[0] = look.head_pitch * DEGREES;
    head.rotation[1] = look.head_yaw * DEGREES;
    let (g, h) = (look.walk_speed, look.walk_position);
    if !look.sitting {
        body.rotation[0] = PI / 2.0;
        if look.sprinting {
            left_hind.rotation[0] = cos(h * 0.6662) * g;
            right_hind.rotation[0] = cos(h * 0.6662 + 0.3) * g;
            left_front.rotation[0] = cos(h * 0.6662 + PI + 0.3) * g;
            right_front.rotation[0] = cos(h * 0.6662 + PI) * g;
            tail2.rotation[0] = 1.727_876_1 + 0.314_159_27 * cos(h) * g;
        } else {
            left_hind.rotation[0] = cos(h * 0.6662) * g;
            right_hind.rotation[0] = cos(h * 0.6662 + PI) * g;
            left_front.rotation[0] = cos(h * 0.6662 + PI) * g;
            right_front.rotation[0] = cos(h * 0.6662) * g;
            let swing = if look.crouching {
                0.471_238_94
            } else {
                0.785_398_2
            };
            tail2.rotation[0] = 1.727_876_1 + swing * cos(h) * g;
        }
    } else {
        body.rotation[0] = 0.785_398_2;
        body.offset[1] += -4.0 * f;
        body.offset[2] += 5.0 * f;
        head.offset[1] += -3.3 * f;
        head.offset[2] += f;
        tail1.offset[1] += 8.0 * f;
        tail1.offset[2] += -2.0 * f;
        tail2.offset[1] += 2.0 * f;
        tail2.offset[2] += -0.8 * f;
        tail1.rotation[0] = 1.727_876_1;
        tail2.rotation[0] = 2.670_354;
        for leg in [&mut *left_front, &mut *right_front] {
            leg.rotation[0] = -0.157_079_64;
            leg.offset[1] += 2.0 * f;
            leg.offset[2] -= 2.0 * f;
        }
        for leg in [&mut *left_hind, &mut *right_hind] {
            leg.rotation[0] = -PI / 2.0;
            leg.offset[1] += 3.0 * f;
            leg.offset[2] -= 4.0 * f;
        }
    }
    if look.lie_down > 0.0 {
        head.rotation[2] = rot_lerp(look.lie_down, head.rotation[2], -1.270_796_3);
        head.rotation[1] = rot_lerp(look.lie_down, head.rotation[1], 1.270_796_3);
        left_front.rotation[0] = -1.270_796_3;
        right_front.rotation[0] = -0.470_796_35;
        right_front.rotation[2] = -0.2;
        right_front.offset[0] += f;
        left_hind.rotation[0] = -0.4;
        right_hind.rotation[0] = 0.5;
        right_hind.rotation[2] = -0.5;
        right_hind.offset[0] += 0.8 * f;
        right_hind.offset[1] += 2.0 * f;
        tail1.rotation[0] = rot_lerp(look.lie_down_tail, tail1.rotation[0], 0.8);
        tail2.rotation[0] = rot_lerp(look.lie_down_tail, tail2.rotation[0], -0.4);
    }
    if look.relax > 0.0 {
        head.rotation[0] = rot_lerp(look.relax, head.rotation[0], -0.581_776_44);
    }
    p
}

fn baby_pose(look: &FelineLook) -> [PartPose; 8] {
    let mut p = [
        PartPose::at([0.0, 20.0, -3.125], [0.0; 3]),
        PartPose::at([0.0, 20.5, 0.5], [0.0; 3]),
        PartPose::at([0.0, 19.107, 3.9151], [-0.567_232, 0.0, 0.0]),
        PartPose::at([0.0; 3], [0.0; 3]),
        PartPose::at([1.0, 22.0, 2.5], [0.0; 3]),
        PartPose::at([-1.0, 22.0, 2.5], [0.0; 3]),
        PartPose::at([1.0, 22.0, -1.5], [0.0; 3]),
        PartPose::at([-1.0, 22.0, -1.5], [0.0; 3]),
    ];
    let [
        head,
        body,
        tail1,
        tail2,
        left_hind,
        right_hind,
        left_front,
        right_front,
    ] = &mut p;
    let f = 0.5;
    if look.crouching {
        body.offset[1] += f;
        head.offset[1] += 2.0 * f;
        tail1.offset[1] += f;
        tail2.offset[1] += -4.0 * f;
        tail2.offset[2] += 2.0 * f;
        tail1.rotation[0] = PI / 2.0;
        tail2.rotation[0] = PI / 2.0;
    } else if look.sprinting {
        tail2.offset[1] = tail1.offset[1];
        tail2.offset[2] += 2.0 * f;
        tail1.rotation[0] = PI / 2.0;
        tail2.rotation[0] = PI / 2.0;
    }
    head.rotation[0] = look.head_pitch * DEGREES;
    head.rotation[1] = look.head_yaw * DEGREES;
    let (g, h) = (look.walk_speed, look.walk_position);
    if !look.sitting {
        if look.sprinting {
            left_hind.rotation[0] = cos(h * 0.6662) * g;
            right_hind.rotation[0] = cos(h * 0.6662 + 0.3) * g;
            left_front.rotation[0] = cos(h * 0.6662 + PI + 0.3) * g;
            right_front.rotation[0] = cos(h * 0.6662 + PI) * g;
            tail2.rotation[0] = 1.727_876_1 + 0.314_159_27 * cos(h) * g;
        } else {
            left_hind.rotation[0] = cos(h * 0.6662) * g;
            right_hind.rotation[0] = cos(h * 0.6662 + PI) * g;
            left_front.rotation[0] = cos(h * 0.6662 + PI) * g;
            right_front.rotation[0] = cos(h * 0.6662) * g;
            let swing = if look.crouching {
                0.471_238_94
            } else {
                0.785_398_2
            };
            tail2.rotation[0] = 1.727_876_1 + swing * cos(h) * g;
        }
    } else {
        body.rotation[0] += -0.436_332_32;
        body.offset[1] += 1.25;
        head.offset[2] += 0.75;
        tail1.rotation[0] += 0.545_415_4;
        tail1.offset[1] += 4.0;
        tail1.offset[2] -= 0.9;
        left_hind.offset[2] -= 0.9;
        right_hind.offset[2] -= 0.9;
    }
    if look.lie_down > 0.0 {
        body.offset[0] += 1.0;
        head.rotation[0] = rot_lerp(look.lie_down, head.rotation[0], 0.174_532_92);
        head.rotation[2] = rot_lerp(look.lie_down, head.rotation[2], -1.308_996_9);
        head.offset[0] += 1.5;
        head.offset[1] += 0.75;
        head.offset[2] -= 0.5;
        right_front.rotation[0] = -0.785_398_2;
        right_front.offset[0] += 3.5;
        right_front.offset[1] -= 0.5;
        left_front.rotation[0] = -PI / 2.0;
        left_front.offset[0] += 1.5;
        left_front.offset[1] -= 1.0;
        left_front.offset[2] -= 2.0;
        right_hind.rotation = [0.698_131_7, 0.349_065_84, -0.349_065_84];
        right_hind.offset[0] += 2.5;
        right_hind.offset[1] -= 0.25;
        right_hind.offset[2] += 0.5;
        left_hind.offset[0] += 1.5;
        left_hind.offset[2] -= 1.0;
        let tail = look.lie_down_tail;
        tail1.rotation[0] += rot_lerp(tail, tail1.rotation[0], -0.523_598_8);
        tail1.rotation[1] += rot_lerp(tail, tail1.rotation[1], 0.0);
        tail1.rotation[2] += rot_lerp(tail, tail1.rotation[2], -0.174_532_92);
        tail1.offset[0] += 1.0;
        tail1.offset[1] += 0.5;
        tail1.offset[2] -= 0.25;
    }
    if look.relax > 0.0 {
        head.rotation[0] = rot_lerp(look.relax, head.rotation[0], -0.581_776_44);
    }
    p
}

fn grown(pose: Mat4, cube: &Cube, grow: f32) -> Mat4 {
    if grow == 0.0 {
        return pose;
    }
    let center = Vec3::from_array(cube.from) + Vec3::from_array(cube.size) / 2.0;
    let size = Vec3::from_array(cube.size);
    let scale = (size + Vec3::splat(2.0 * grow)) / size;
    pose * Mat4::from_translation(center / 16.0)
        * Mat4::from_scale(scale)
        * Mat4::from_translation(-center / 16.0)
}

pub fn append_model(
    mesh: &mut ChunkMesh,
    root: Mat4,
    skin: &Skin,
    baby: bool,
    pose: &[PartPose; 8],
    grow: f32,
) {
    let cubes: &[(Part, Cube)] = if baby { &BABY } else { &ADULT };
    for (part, cube) in cubes {
        let part_pose_of = pose[index(*part)];
        let matrix = root * part_pose(part_pose_of.offset, part_pose_of.rotation);
        let grow = match part {
            Part::Tail2 if !baby => TAIL2_GROW,
            _ => grow,
        };
        append(mesh, grown(matrix, cube, grow), skin, cube);
    }
}

pub fn model_root(feet: Mat4, kind: FelineKind, baby: bool) -> Mat4 {
    let base = feet
        * Mat4::from_translation(Vec3::new(0.0, 24.016 / 16.0, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
    if kind == FelineKind::Cat && !baby {
        base * scaled_root(CAT_SCALE)
    } else {
        base
    }
}

fn scaled_root(scale: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.0, 24.016 * (1.0 - scale) / 16.0, 0.0))
        * Mat4::from_scale(Vec3::splat(scale))
}

pub fn lying_turn(lie_down: f32, on_sleeper: bool) -> Mat4 {
    if lie_down <= 0.0 {
        return Mat4::IDENTITY;
    }
    let mut turn =
        Mat4::from_translation(Vec3::new(0.4 * lie_down, 0.15 * lie_down, 0.1 * lie_down))
            * Mat4::from_rotation_z(rot_lerp(lie_down, 0.0, 90.0).to_radians());
    if on_sleeper {
        turn *= Mat4::from_translation(Vec3::new(0.15 * lie_down, 0.0, 0.0));
    }
    turn
}

pub fn texture(feline: &Feline) -> String {
    feline.texture()
}

pub fn look_of(feline: &Feline, mob: &MobPose, partial: f32) -> FelineLook {
    let (lie_down, lie_down_tail, relax) = if feline.is_cat() {
        feline.lie_down(partial)
    } else {
        (0.0, 0.0, 0.0)
    };
    FelineLook {
        baby: feline.baby(),
        crouching: feline.crouching,
        sprinting: feline.sprinting,
        sitting: feline.is_cat() && feline.sitting,
        lie_down,
        lie_down_tail,
        relax,
        walk_position: mob.walk_position,
        walk_speed: mob.walk_speed,
        head_yaw: mob.head_yaw,
        head_pitch: mob.head_pitch,
    }
}

fn tint(mesh: &mut ChunkMesh, from: usize, rgb: u32) {
    let channel = |shift: u32| ((rgb >> shift) & 255) as f32 / 255.0;
    let (r, g, b) = (channel(16), channel(8), channel(0));
    for vertex in &mut mesh.vertices[from..] {
        vertex.color[0] *= r;
        vertex.color[1] *= g;
        vertex.color[2] *= b;
    }
}

pub fn append_felines<'a>(
    mesh: &mut ChunkMesh,
    felines: impl IntoIterator<Item = &'a FelineEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let partial = partial.clamp(0.0, 1.0);
    let mut marks = Vec::new();
    for entity in felines {
        let feline = &entity.feline;
        let Ok(skin_id) = ResourceId::parse(&texture(feline)) else {
            continue;
        };
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let baby = feline.baby();
        let position = mob.light_block();
        let (sky, block) = (light.get(position) as f32, light.get_block(position) as f32);
        let look = look_of(feline, &mob, partial);
        let turn: Quat = mob.body_rotation(90.0);
        let feet = Mat4::from_translation(mob.feet.as_vec3())
            * Mat4::from_quat(turn)
            * lying_turn(look.lie_down, feline.lying_on_sleeping_player);
        let pose = feline_pose(&look);
        let texture_size = if baby { [32.0, 32.0] } else { [64.0, 32.0] };
        let skin = Skin {
            region: atlas.entity_region(&skin_id),
            texture_size,
            sky,
            block,
            alpha: 1.0,
            shaded: true,
        };
        append_model(
            mesh,
            model_root(feet, feline.kind, baby),
            &skin,
            baby,
            &pose,
            0.0,
        );
        if feline.shows_collar() {
            let collar = if baby {
                COLLAR_BABY_TEXTURE
            } else {
                COLLAR_TEXTURE
            };
            let Ok(collar_id) = ResourceId::parse(collar) else {
                continue;
            };
            let collar_skin = Skin {
                region: atlas.entity_region(&collar_id),
                ..skin
            };
            let start = mesh.vertices.len();
            let (root, grow) = if baby {
                (
                    model_root(feet, feline.kind, true) * scaled_root(BABY_COLLAR_SCALE),
                    0.0,
                )
            } else {
                (model_root(feet, feline.kind, false), COLLAR_GROW)
            };
            append_model(mesh, root, &collar_skin, baby, &pose, grow);
            tint(
                mesh,
                start,
                crate::wolf_render::DYE_DIFFUSE[usize::from(feline.collar & 15)],
            );
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_entities::age::Age;

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: [64.0, 32.0],
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    fn lowest(mesh: &ChunkMesh) -> f32 {
        mesh.vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MAX, f32::min)
    }

    #[test]
    fn an_adult_cat_has_eleven_cubes_a_baby_ten_and_both_stand_on_the_ground() {
        let look = FelineLook::default();
        let mut adult = ChunkMesh::default();
        let root = model_root(Mat4::IDENTITY, FelineKind::Ocelot, false);
        append_model(&mut adult, root, &skin(), false, &feline_pose(&look), 0.0);
        assert_eq!(adult.faces, 11 * 6);
        assert!(lowest(&adult).abs() < 0.01, "{}", lowest(&adult));
        let mut cat = ChunkMesh::default();
        let root = model_root(Mat4::IDENTITY, FelineKind::Cat, false);
        append_model(&mut cat, root, &skin(), false, &feline_pose(&look), 0.0);
        assert!(lowest(&cat).abs() < 0.01, "{}", lowest(&cat));
        let top = |mesh: &ChunkMesh| {
            mesh.vertices
                .iter()
                .map(|v| v.position[1])
                .fold(f32::MIN, f32::max)
        };
        assert!(
            (top(&cat) / top(&adult) - 0.8).abs() < 0.01,
            "the cat is 0.8 of the ocelot"
        );
        let baby_look = FelineLook {
            baby: true,
            ..FelineLook::default()
        };
        let mut baby = ChunkMesh::default();
        let root = model_root(Mat4::IDENTITY, FelineKind::Cat, true);
        append_model(
            &mut baby,
            root,
            &skin(),
            true,
            &feline_pose(&baby_look),
            0.0,
        );
        assert_eq!(baby.faces, 10 * 6);
        assert!(lowest(&baby).abs() < 0.01, "{}", lowest(&baby));
    }

    #[test]
    fn sitting_folds_the_hind_legs_and_lying_turns_the_head() {
        let sitting = adult_pose(&FelineLook {
            sitting: true,
            ..FelineLook::default()
        });
        assert_eq!(sitting[index(Part::LeftHindLeg)].rotation[0], -PI / 2.0);
        assert_eq!(sitting[index(Part::Body)].rotation[0], 0.785_398_2);
        assert_eq!(sitting[index(Part::Tail2)].rotation[0], 2.670_354);
        let head = sitting[index(Part::Head)].offset;
        assert!((Vec3::from_array(head) - Vec3::new(0.0, 11.7, -8.0)).length() < 1.0e-5);
        let lying = adult_pose(&FelineLook {
            lie_down: 1.0,
            lie_down_tail: 1.0,
            ..FelineLook::default()
        });
        assert!((lying[index(Part::Head)].rotation[2] + 1.270_796_3).abs() < 1.0e-6);
        assert!((lying[index(Part::Tail1)].rotation[0] - 0.8).abs() < 1.0e-6);
        let baby = baby_pose(&FelineLook {
            baby: true,
            sitting: true,
            ..FelineLook::default()
        });
        assert!((baby[index(Part::Body)].rotation[0] + 0.436_332_32).abs() < 1.0e-6);
        let body = baby[index(Part::Body)].offset;
        assert!((Vec3::from_array(body) - Vec3::new(0.0, 21.75, 0.5)).length() < 1.0e-5);
        let crouch = adult_pose(&FelineLook {
            crouching: true,
            ..FelineLook::default()
        });
        assert_eq!(crouch[index(Part::Head)].offset, [0.0, 17.0, -9.0]);
        assert_eq!(crouch[index(Part::Tail1)].rotation[0], PI / 2.0);
        assert!(lying_turn(0.0, false) == Mat4::IDENTITY);
        let turned = lying_turn(1.0, true).transform_point3(Vec3::ZERO);
        assert!(
            (turned - Vec3::new(0.4, 0.15 + 0.15, 0.1)).length() < 1.0e-5,
            "{turned}"
        );
    }

    #[test]
    fn the_cat_textures_follow_their_variants_and_the_collar_shows_on_tame_cats() {
        let list = textures();
        assert_eq!(list.len(), CAT_VARIANTS.len() * 2 + 4);
        assert!(list.contains(&"minecraft:entity/cat/cat_all_black_baby".to_owned()));
        let mut cat = Feline::new(FelineKind::Cat, glam::DVec3::ZERO);
        cat.variant = "jellie".to_owned();
        assert_eq!(texture(&cat), "minecraft:entity/cat/cat_jellie");
        assert!(!cat.shows_collar());
        cat.tame = true;
        assert!(cat.shows_collar());
        cat.set_age(Age::BABY_START);
        assert_eq!(texture(&cat), "minecraft:entity/cat/cat_jellie_baby");
        let ocelot = Feline::new(FelineKind::Ocelot, glam::DVec3::ZERO);
        assert_eq!(texture(&ocelot), OCELOT_TEXTURE);
        assert_eq!(SHADOW_RADIUS, 0.4);
    }
}
