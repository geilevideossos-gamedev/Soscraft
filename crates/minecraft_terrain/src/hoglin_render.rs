use crate::client_mobs::{ClientMobs, MobPose};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::hoglin::HoglinKind;
use minecraftoss_entities::world::HoglinEntity;

pub const HOGLIN_TEXTURE: &str = "minecraft:entity/hoglin/hoglin";
pub const HOGLIN_BABY_TEXTURE: &str = "minecraft:entity/hoglin/hoglin_baby";
pub const ZOGLIN_TEXTURE: &str = "minecraft:entity/hoglin/zoglin";
pub const ZOGLIN_BABY_TEXTURE: &str = "minecraft:entity/hoglin/zoglin_baby";
pub const TEXTURES: [&str; 4] = [
    HOGLIN_TEXTURE,
    HOGLIN_BABY_TEXTURE,
    ZOGLIN_TEXTURE,
    ZOGLIN_BABY_TEXTURE,
];
pub const SHADOW_RADIUS: f32 = 0.7;

const HEAD_X_ROT: f32 = 0.872_664_63;
const ATTACK_HEAD_X_ROT_END: f32 = -0.349_065_84;
const EAR_Z_ROT: f32 = 0.698_131_7;

const BODY: Cube = Cube::new([-8.0, -7.0, -13.0], [16.0, 14.0, 26.0], [1.0, 1.0]);
const MANE: Cube = Cube::new([0.0, 0.0, -9.0], [0.0, 10.0, 19.0], [90.0, 33.0]);
const HEAD: Cube = Cube::new([-7.0, -3.0, -19.0], [14.0, 6.0, 19.0], [61.0, 1.0]);
const RIGHT_EAR: Cube = Cube::new([-6.0, -1.0, -2.0], [6.0, 1.0, 4.0], [1.0, 1.0]);
const LEFT_EAR: Cube = Cube::new([0.0, -1.0, -2.0], [6.0, 1.0, 4.0], [1.0, 6.0]);
const RIGHT_HORN: Cube = Cube::new([-1.0, -11.0, -1.0], [2.0, 11.0, 2.0], [10.0, 13.0]);
const LEFT_HORN: Cube = Cube::new([-1.0, -11.0, -1.0], [2.0, 11.0, 2.0], [1.0, 13.0]);
const FRONT_LEG_FROM: [f32; 3] = [-3.0, 0.0, -3.0];
const FRONT_LEG_SIZE: [f32; 3] = [6.0, 14.0, 6.0];
const HIND_LEG_FROM: [f32; 3] = [-2.5, 0.0, -2.5];
const HIND_LEG_SIZE: [f32; 3] = [5.0, 11.0, 5.0];
const ADULT_LEGS: [([f32; 3], [f32; 3], [f32; 2], [f32; 3]); 4] = [
    (
        FRONT_LEG_FROM,
        FRONT_LEG_SIZE,
        [66.0, 42.0],
        [-4.0, 10.0, -8.5],
    ),
    (
        FRONT_LEG_FROM,
        FRONT_LEG_SIZE,
        [41.0, 42.0],
        [4.0, 10.0, -8.5],
    ),
    (
        HIND_LEG_FROM,
        HIND_LEG_SIZE,
        [21.0, 45.0],
        [-5.0, 13.0, 10.0],
    ),
    (HIND_LEG_FROM, HIND_LEG_SIZE, [0.0, 45.0], [5.0, 13.0, 10.0]),
];

const BABY_HEAD: [Cube; 3] = [
    Cube::new([-5.0, -2.2605, -10.547], [10.0, 4.0, 12.0], [0.0, 0.0]),
    Cube::new([-7.0, -4.0981, -8.4879], [2.0, 5.0, 2.0], [44.0, 29.0]),
    Cube::new([5.0, -4.0981, -8.4879], [2.0, 5.0, 2.0], [52.0, 29.0]),
];
const BABY_BODY: [Cube; 2] = [
    Cube::new([-4.0, -14.0, -7.0], [8.0, 8.0, 14.0], [0.0, 16.0]),
    Cube::new([0.0, -18.0, -8.0], [0.0, 6.0, 11.0], [24.0, 39.0]),
];
const BABY_RIGHT_EAR: Cube = Cube::new([-5.1, -0.5, -2.0], [6.0, 1.0, 4.0], [32.0, 5.0]);
const BABY_LEFT_EAR: Cube = Cube::new([-0.9, -0.5, -2.0], [6.0, 1.0, 4.0], [32.0, 0.0]).mirrored();
const BABY_LEG_FROM: [f32; 3] = [-1.5, 0.0, -1.5];
const BABY_LEG_SIZE: [f32; 3] = [3.0, 6.0, 3.0];
const BABY_LEGS: [([f32; 2], [f32; 3]); 4] = [
    ([0.0, 38.0], [-2.5, 18.0, -4.5]),
    ([12.0, 38.0], [2.5, 18.0, -4.5]),
    ([0.0, 47.0], [-2.5, 18.0, 4.5]),
    ([12.0, 47.0], [2.5, 18.0, 4.5]),
];

pub fn texture(kind: HoglinKind, baby: bool) -> &'static str {
    match (kind, baby) {
        (HoglinKind::Hoglin, false) => HOGLIN_TEXTURE,
        (HoglinKind::Hoglin, true) => HOGLIN_BABY_TEXTURE,
        (HoglinKind::Zoglin, false) => ZOGLIN_TEXTURE,
        (HoglinKind::Zoglin, true) => ZOGLIN_BABY_TEXTURE,
    }
}

pub fn shadow_radius(baby: bool) -> f32 {
    if baby {
        SHADOW_RADIUS * 0.5
    } else {
        SHADOW_RADIUS
    }
}

pub fn shake_degrees(age_in_ticks: f32, kind: HoglinKind, converting: bool) -> f32 {
    if kind == HoglinKind::Hoglin && converting {
        ((f64::from(age_in_ticks * 3.25)).cos() * std::f64::consts::PI * f64::from(0.4_f32)) as f32
    } else {
        0.0
    }
}

fn sin(value: f32) -> f32 {
    minecraftoss_player::mth::sin(f64::from(value))
}

fn cos(value: f32) -> f32 {
    minecraftoss_player::mth::cos(f64::from(value))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoglinPose {
    pub head: [f32; 3],
    pub head_y: f32,
    pub ears: [f32; 2],
    pub legs: [f32; 4],
}

pub fn hoglin_pose(
    baby: bool,
    walk_position: f32,
    walk_speed: f32,
    head_yaw: f32,
    attack_ticks: i32,
) -> HoglinPose {
    let swing = walk_speed * sin(walk_position);
    let butt = 1.0 - (10 - 2 * attack_ticks).abs() as f32 / 10.0;
    let head_x = HEAD_X_ROT + butt * (ATTACK_HEAD_X_ROT_END - HEAD_X_ROT);
    let right_front = cos(walk_position) * 1.2 * walk_speed;
    let left_front = cos(walk_position + std::f32::consts::PI) * 1.2 * walk_speed;
    let rest_y = if baby { 13.0 } else { 2.0 };
    HoglinPose {
        head: [head_x, head_yaw * 0.017_453_292, 0.0],
        head_y: if baby { rest_y + butt * 2.5 } else { rest_y },
        ears: [-EAR_Z_ROT - swing, EAR_Z_ROT + swing],
        legs: [right_front, left_front, left_front, right_front],
    }
}

fn root(pose: &MobPose, shake: f32) -> Mat4 {
    let turned = MobPose {
        body_rot: pose.body_rot + shake,
        ..*pose
    };
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(turned.body_rotation(90.0))
        * Mat4::from_translation(Vec3::new(0.0, 24.016 / 16.0, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, baby: bool, pose: &HoglinPose) {
    if baby {
        let head = root * part_pose([0.0, pose.head_y, -7.0], pose.head);
        for cube in &BABY_HEAD {
            append(mesh, head, skin, cube);
        }
        let body = root * part_pose([0.0, 24.0, 0.0], [0.0; 3]);
        for cube in &BABY_BODY {
            append(mesh, body, skin, cube);
        }
        append(
            mesh,
            head * part_pose([-5.0, -1.0, -1.5], [0.0, 0.0, pose.ears[0]]),
            skin,
            &BABY_RIGHT_EAR,
        );
        append(
            mesh,
            head * part_pose([5.0, -1.0, -1.5], [0.0, 0.0, pose.ears[1]]),
            skin,
            &BABY_LEFT_EAR,
        );
        for (i, (uv, offset)) in BABY_LEGS.iter().enumerate() {
            let leg = Cube::new(BABY_LEG_FROM, BABY_LEG_SIZE, *uv);
            append(
                mesh,
                root * part_pose(*offset, [pose.legs[i], 0.0, 0.0]),
                skin,
                &leg,
            );
        }
        return;
    }
    let body = root * part_pose([0.0, 7.0, 0.0], [0.0; 3]);
    append(mesh, body, skin, &BODY);
    append(
        mesh,
        body * part_pose([0.0, -14.0, -7.0], [0.0; 3]),
        skin,
        &MANE,
    );
    let head = root * part_pose([0.0, pose.head_y, -12.0], pose.head);
    append(mesh, head, skin, &HEAD);
    append(
        mesh,
        head * part_pose([-6.0, -2.0, -3.0], [0.0, 0.0, pose.ears[0]]),
        skin,
        &RIGHT_EAR,
    );
    append(
        mesh,
        head * part_pose([6.0, -2.0, -3.0], [0.0, 0.0, pose.ears[1]]),
        skin,
        &LEFT_EAR,
    );
    append(
        mesh,
        head * part_pose([-7.0, 2.0, -12.0], [0.0; 3]),
        skin,
        &RIGHT_HORN,
    );
    append(
        mesh,
        head * part_pose([7.0, 2.0, -12.0], [0.0; 3]),
        skin,
        &LEFT_HORN,
    );
    for (i, (from, size, uv, offset)) in ADULT_LEGS.iter().enumerate() {
        let leg = Cube::new(*from, *size, *uv);
        append(
            mesh,
            root * part_pose(*offset, [pose.legs[i], 0.0, 0.0]),
            skin,
            &leg,
        );
    }
}

pub fn append_hoglins<'a>(
    mesh: &mut ChunkMesh,
    hoglins: impl IntoIterator<Item = &'a HoglinEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let mut marks = Vec::new();
    for entity in hoglins {
        let hoglin = &entity.hoglin;
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let baby = hoglin.is_baby();
        let position = mob.light_block();
        let skin = Skin {
            region: atlas.entity_region(&ResourceId::parse(texture(hoglin.kind, baby)).unwrap()),
            texture_size: if baby { [64.0, 64.0] } else { [128.0, 64.0] },
            sky: light.get(position) as f32,
            block: light.get_block(position) as f32,
            alpha: 1.0,
            shaded: true,
        };
        let pose = hoglin_pose(
            baby,
            mob.walk_position,
            mob.walk_speed,
            mob.head_yaw,
            hoglin.attack_animation,
        );
        let shake = shake_degrees(mob.age_in_ticks, hoglin.kind, hoglin.converting);
        append_model(mesh, root(&mob, shake), &skin, baby, &pose);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: [128.0, 64.0],
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    #[test]
    fn an_adult_and_a_baby_hoglin_have_eleven_cubes_each() {
        let pose = hoglin_pose(false, 0.0, 0.0, 0.0, 0);
        let mut adult = ChunkMesh::default();
        append_model(&mut adult, Mat4::IDENTITY, &skin(), false, &pose);
        assert_eq!(adult.faces, 11 * 6);
        assert_eq!(adult.vertices.len(), 264);
        let pose = hoglin_pose(true, 0.0, 0.0, 0.0, 0);
        let mut baby = ChunkMesh::default();
        append_model(&mut baby, Mat4::IDENTITY, &skin(), true, &pose);
        assert_eq!(baby.vertices.len(), 264);
        let lowest = |mesh: &ChunkMesh| {
            mesh.vertices
                .iter()
                .map(|v| v.position[1])
                .fold(f32::MIN, f32::max)
        };
        assert!((lowest(&adult) - 1.5).abs() < 1.0e-4, "{}", lowest(&adult));
        assert!((lowest(&baby) - 1.5).abs() < 1.0e-4, "{}", lowest(&baby));
    }

    #[test]
    fn the_head_butts_up_and_back_over_ten_ticks() {
        let rest = 50.0_f32.to_radians();
        for ticks in [0, 10] {
            let pose = hoglin_pose(false, 0.0, 0.0, 0.0, ticks);
            assert!(
                (pose.head[0] - rest).abs() < 1.0e-5,
                "{ticks}: {}",
                pose.head[0]
            );
        }
        let butt = hoglin_pose(false, 0.0, 0.0, 0.0, 5);
        assert!((butt.head[0] + 20.0_f32.to_radians()).abs() < 1.0e-5);
        assert_eq!(butt.head_y, 2.0);
        let piglet = hoglin_pose(true, 0.0, 0.0, 0.0, 5);
        assert_eq!(piglet.head_y, 13.0 + 2.5);
        assert_eq!(hoglin_pose(true, 0.0, 0.0, 0.0, 0).head_y, 13.0);
        let turned = hoglin_pose(false, 0.0, 0.0, 30.0, 0);
        assert!((turned.head[1] - 30.0_f32.to_radians()).abs() < 1.0e-5);
    }

    #[test]
    fn the_ears_hang_at_forty_degrees_and_flap_with_the_walk() {
        let still = hoglin_pose(false, 0.0, 0.0, 0.0, 0);
        assert!((still.ears[0] + 40.0_f32.to_radians()).abs() < 1.0e-5);
        assert!((still.ears[1] - 40.0_f32.to_radians()).abs() < 1.0e-5);
        let walking = hoglin_pose(false, 1.0, 0.5, 0.0, 0);
        let flap = 0.5 * sin(1.0);
        assert!((walking.ears[0] - (-EAR_Z_ROT - flap)).abs() < 1.0e-6);
        assert!((walking.ears[1] - (EAR_Z_ROT + flap)).abs() < 1.0e-6);
        assert!((walking.legs[0] + walking.legs[1]).abs() < 1.0e-5);
        assert_eq!(walking.legs[2], walking.legs[1]);
        assert_eq!(walking.legs[3], walking.legs[0]);
        assert!(walking.legs[0] != 0.0);
    }

    #[test]
    fn a_converting_hoglin_shakes_and_a_zoglin_does_not() {
        assert_eq!(shake_degrees(7.0, HoglinKind::Hoglin, false), 0.0);
        assert_eq!(shake_degrees(7.0, HoglinKind::Zoglin, true), 0.0);
        let expected = ((7.0_f64 * 3.25).cos() * std::f64::consts::PI * 0.4) as f32;
        assert!((shake_degrees(7.0, HoglinKind::Hoglin, true) - expected).abs() < 1.0e-5);
        assert_eq!(texture(HoglinKind::Hoglin, false), HOGLIN_TEXTURE);
        assert_eq!(texture(HoglinKind::Hoglin, true), HOGLIN_BABY_TEXTURE);
        assert_eq!(texture(HoglinKind::Zoglin, false), ZOGLIN_TEXTURE);
        assert_eq!(texture(HoglinKind::Zoglin, true), ZOGLIN_BABY_TEXTURE);
        assert_eq!((shadow_radius(false), shadow_radius(true)), (0.7, 0.35));
    }
}
