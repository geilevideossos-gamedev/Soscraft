use crate::client_mobs::{ClientMobs, MobPose};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Quat, Vec3};
use minecraftoss_entities::world::StriderEntity;

pub const TEXTURE: &str = "minecraft:entity/strider/strider";
pub const COLD_TEXTURE: &str = "minecraft:entity/strider/strider_cold";
pub const BABY_TEXTURE: &str = "minecraft:entity/strider/strider_baby";
pub const COLD_BABY_TEXTURE: &str = "minecraft:entity/strider/strider_cold_baby";
pub const SADDLE_TEXTURE: &str = "minecraft:entity/equipment/strider_saddle/saddle";
pub const TEXTURES: [&str; 5] = [
    TEXTURE,
    COLD_TEXTURE,
    BABY_TEXTURE,
    COLD_BABY_TEXTURE,
    SADDLE_TEXTURE,
];
pub const SHADOW_RADIUS: f32 = 0.5;

const LEG: Cube = Cube::new([-2.0, 0.0, -2.0], [4.0, 16.0, 4.0], [0.0, 0.0]);
const ADULT_BODY: Cube = Cube::new([-8.0, -6.0, -8.0], [16.0, 14.0, 16.0], [0.0, 0.0]);
const ADULT_RIGHT_LEG_UV: [f32; 2] = [0.0, 32.0];
const ADULT_LEFT_LEG_UV: [f32; 2] = [0.0, 55.0];
const BRISTLE_SIZE: [f32; 3] = [12.0, 0.0, 16.0];
const RIGHT_BRISTLE_FROM: [f32; 3] = [-12.0, 0.0, 0.0];
const LEFT_BRISTLE_FROM: [f32; 3] = [0.0, 0.0, 0.0];
const BRISTLE_UVS: [[f32; 2]; 3] = [[16.0, 33.0], [16.0, 49.0], [16.0, 65.0]];
const RIGHT_BRISTLE_OFFSETS: [[f32; 3]; 3] =
    [[-8.0, -5.0, -8.0], [-8.0, -1.0, -8.0], [-8.0, 4.0, -8.0]];
const LEFT_BRISTLE_OFFSETS: [[f32; 3]; 3] =
    [[8.0, -6.0, -8.0], [8.0, -2.0, -8.0], [8.0, 3.0, -8.0]];
const BRISTLE_ROLLS: [f32; 3] = [0.872_664_63, 1.134_464, 1.221_730_5];
const BABY_BODY: Cube = Cube::new([-3.5, -3.75, -4.0], [7.0, 7.0, 8.0], [0.0, 0.0]);
const BABY_LEG: Cube = Cube::new([-1.0, 0.0, -1.0], [2.0, 4.0, 2.0], [0.0, 24.0]);
const BABY_BRISTLE: Cube = Cube::new([-3.5, -2.5, 0.0], [7.0, 3.0, 0.0], [0.0, 15.0]);
const BABY_BRISTLE_UVS: [[f32; 2]; 3] = [[0.0, 15.0], [0.0, 18.0], [0.0, 21.0]];
const BABY_BRISTLE_Z: [f32; 3] = [-2.0, 0.0, 2.0];

pub fn texture(baby: bool, cold: bool) -> &'static str {
    match (baby, cold) {
        (false, false) => TEXTURE,
        (false, true) => COLD_TEXTURE,
        (true, false) => BABY_TEXTURE,
        (true, true) => COLD_BABY_TEXTURE,
    }
}

pub fn shake_degrees(age_in_ticks: f32, cold: bool) -> f32 {
    if cold {
        ((f64::from(age_in_ticks * 3.25)).cos() * std::f64::consts::PI * f64::from(0.4_f32)) as f32
    } else {
        0.0
    }
}

pub fn shadow_radius(baby: bool) -> f32 {
    if baby {
        SHADOW_RADIUS * 0.5
    } else {
        SHADOW_RADIUS
    }
}

fn sin(value: f32) -> f32 {
    minecraftoss_player::mth::sin(f64::from(value))
}

fn cos(value: f32) -> f32 {
    minecraftoss_player::mth::cos(f64::from(value))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StriderPose {
    pub body: [f32; 3],
    pub body_y: f32,
    pub legs: [([f32; 3], f32); 2],
    pub bristles: [[f32; 3]; 2],
}

pub fn strider_pose(
    baby: bool,
    walk_position: f32,
    walk_speed: f32,
    age_in_ticks: f32,
    head_pitch: f32,
    head_yaw: f32,
    ridden: bool,
) -> StriderPose {
    let speed = walk_speed.min(0.25);
    let body = if ridden {
        [0.0, 0.0, 0.1 * sin(walk_position * 1.5) * 4.0 * speed]
    } else {
        [
            head_pitch * 0.017_453_292,
            head_yaw * 0.017_453_292,
            0.1 * sin(walk_position * 1.5) * 4.0 * speed,
        ]
    };
    let half = walk_position * 1.5 * 0.5;
    let pi = std::f32::consts::PI;
    let left = [
        sin(half) * 2.0 * speed,
        0.0,
        0.174_532_92 * cos(half) * speed,
    ];
    let right = [
        sin(half + pi) * 2.0 * speed,
        0.0,
        0.174_532_92 * cos(half + pi) * speed,
    ];
    let sway = cos(walk_position * 1.5 + pi) * speed;
    let bristles = [
        sway * 0.6 + 0.1 * sin(age_in_ticks * 0.4),
        sway * 1.2 + 0.1 * sin(age_in_ticks * 0.2),
        sway * 1.3 + 0.05 * sin(age_in_ticks * -0.4),
    ];
    let (rest_y, leg_y, body_bob) = if baby {
        (17.25, 20.0, 1.0)
    } else {
        (2.0, 8.0, 2.0)
    };
    let body_y = rest_y - body_bob * cos(walk_position * 1.5) * 2.0 * speed;
    let left_y = leg_y + 2.0 * sin(half + pi) * 2.0 * speed;
    let right_y = leg_y + 2.0 * sin(half) * 2.0 * speed;
    StriderPose {
        body,
        body_y,
        legs: [(right, right_y), (left, left_y)],
        bristles: [bristles, bristles],
    }
}

fn root(pose: &MobPose, cold: bool) -> Mat4 {
    let body_rot = pose.body_rot + shake_degrees(pose.age_in_ticks, cold);
    let turn = pose.turn * Quat::from_rotation_y((180.0 - body_rot).to_radians());
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(turn)
        * Mat4::from_translation(Vec3::new(0.0, 24.016 / 16.0, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, baby: bool, pose: &StriderPose) {
    let [right_leg, left_leg] = pose.legs;
    let body = part_pose([0.0, pose.body_y, 0.0], pose.body);
    if baby {
        append(
            mesh,
            root * part_pose([-1.5, right_leg.1, 0.0], right_leg.0),
            skin,
            &BABY_LEG,
        );
        let left = Cube {
            uv: [8.0, 24.0],
            ..BABY_LEG
        };
        append(
            mesh,
            root * part_pose([1.5, left_leg.1, 0.0], left_leg.0),
            skin,
            &left,
        );
        append(mesh, root * body, skin, &BABY_BODY);
        for i in 0..3 {
            let bristle = Cube {
                uv: BABY_BRISTLE_UVS[i],
                ..BABY_BRISTLE
            };
            let rotation = [pose.bristles[0][i], 0.0, 0.0];
            append(
                mesh,
                root * body * part_pose([0.0, -4.25, BABY_BRISTLE_Z[i]], rotation),
                skin,
                &bristle,
            );
        }
        return;
    }
    let right = Cube {
        uv: ADULT_RIGHT_LEG_UV,
        ..LEG
    };
    let left = Cube {
        uv: ADULT_LEFT_LEG_UV,
        ..LEG
    };
    append(
        mesh,
        root * part_pose([-4.0, right_leg.1, 0.0], right_leg.0),
        skin,
        &right,
    );
    append(
        mesh,
        root * part_pose([4.0, left_leg.1, 0.0], left_leg.0),
        skin,
        &left,
    );
    append(mesh, root * body, skin, &ADULT_BODY);
    for i in 0..3 {
        let roll = BRISTLE_ROLLS[i];
        let right = Cube::new(RIGHT_BRISTLE_FROM, BRISTLE_SIZE, BRISTLE_UVS[i]).mirrored();
        let right_pose = part_pose(
            RIGHT_BRISTLE_OFFSETS[i],
            [0.0, 0.0, -roll + pose.bristles[0][i]],
        );
        append(mesh, root * body * right_pose, skin, &right);
        let left = Cube::new(LEFT_BRISTLE_FROM, BRISTLE_SIZE, BRISTLE_UVS[i]);
        let left_pose = part_pose(
            LEFT_BRISTLE_OFFSETS[i],
            [0.0, 0.0, roll + pose.bristles[1][i]],
        );
        append(mesh, root * body * left_pose, skin, &left);
    }
}

pub fn append_striders<'a>(
    mesh: &mut ChunkMesh,
    striders: impl IntoIterator<Item = &'a StriderEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let mut marks = Vec::new();
    for entity in striders {
        let strider = &entity.strider;
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let baby = strider.age.baby();
        let cold = strider.suffocating;
        let position = mob.light_block();
        let mut skin = Skin {
            region: atlas.entity_region(&ResourceId::parse(texture(baby, cold)).unwrap()),
            texture_size: if baby { [32.0, 32.0] } else { [64.0, 128.0] },
            sky: light.get(position) as f32,
            block: light.get_block(position) as f32,
            alpha: 1.0,
            shaded: true,
        };
        let pose = strider_pose(
            baby,
            mob.walk_position,
            mob.walk_speed,
            mob.age_in_ticks,
            mob.head_pitch,
            mob.head_yaw,
            false,
        );
        let root = root(&mob, cold);
        append_model(mesh, root, &skin, baby, &pose);
        if strider.saddled && !baby {
            skin.region = atlas.entity_region(&ResourceId::parse(SADDLE_TEXTURE).unwrap());
            append_model(mesh, root, &skin, false, &pose);
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: [64.0, 128.0],
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    #[test]
    fn the_adult_has_three_cubes_and_six_bristle_planes() {
        let pose = strider_pose(false, 0.0, 0.0, 0.0, 0.0, 0.0, false);
        let mut mesh = ChunkMesh::default();
        append_model(&mut mesh, Mat4::IDENTITY, &skin(), false, &pose);
        assert_eq!(mesh.faces, 9 * 6);
        assert_eq!(mesh.vertices.len(), 9 * 6 * 4);
        let mut baby = ChunkMesh::default();
        append_model(&mut baby, Mat4::IDENTITY, &skin(), true, &pose);
        assert_eq!(
            baby.faces,
            6 * 6,
            "a baby has two legs, a body and three bristles"
        );
        let top = mesh
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max);
        assert!(
            top >= 24.0 / 16.0 - 1.0e-4,
            "the legs reach 24 pixels down in model space: {top}"
        );
    }

    #[test]
    fn the_cold_texture_shows_while_suffocating() {
        assert_eq!(texture(false, false), TEXTURE);
        assert_eq!(texture(false, true), COLD_TEXTURE);
        assert_eq!(texture(true, false), BABY_TEXTURE);
        assert_eq!(texture(true, true), COLD_BABY_TEXTURE);
        assert_eq!((shadow_radius(false), shadow_radius(true)), (0.5, 0.25));
    }

    #[test]
    fn the_shake_is_cos_3_25_times_0_4_pi_degrees() {
        assert_eq!(shake_degrees(7.0, false), 0.0);
        let expected = ((7.0_f64 * 3.25).cos() * std::f64::consts::PI * 0.4) as f32;
        assert!((shake_degrees(7.0, true) - expected).abs() < 1.0e-5);
        let peak = (0..400)
            .map(|i| shake_degrees(i as f32 * 0.05, true).abs())
            .fold(0.0, f32::max);
        assert!((peak - 1.2566).abs() < 0.01, "{peak}");
    }

    #[test]
    fn a_saddled_adult_adds_the_saddle_layer() {
        let pose = strider_pose(false, 1.0, 0.2, 10.0, 0.0, 0.0, false);
        let mut mesh = ChunkMesh::default();
        append_model(&mut mesh, Mat4::IDENTITY, &skin(), false, &pose);
        let plain = mesh.vertices.len();
        append_model(&mut mesh, Mat4::IDENTITY, &skin(), false, &pose);
        assert_eq!(mesh.vertices.len(), plain * 2);
        for (a, b) in mesh.vertices[..plain].iter().zip(&mesh.vertices[plain..]) {
            assert_eq!(
                a.position, b.position,
                "the saddle layer is the adult model again"
            );
        }
    }

    #[test]
    fn walking_swings_the_legs_against_each_other() {
        let still = strider_pose(false, 0.0, 0.0, 0.0, 0.0, 0.0, false);
        assert_eq!(still.legs[0], ([0.0, 0.0, 0.0], 8.0));
        assert_eq!(still.body_y, 2.0);
        let walking = strider_pose(false, 1.0, 0.5, 0.0, 0.0, 0.0, false);
        assert!((walking.legs[0].0[0] + walking.legs[1].0[0]).abs() < 1.0e-5);
        assert!(walking.legs[0].0[0] != 0.0);
        let ridden = strider_pose(false, 0.0, 0.0, 0.0, 30.0, 40.0, true);
        assert_eq!((ridden.body[0], ridden.body[1]), (0.0, 0.0));
        let free = strider_pose(false, 0.0, 0.0, 0.0, 30.0, 40.0, false);
        assert!((free.body[1] - 40.0_f32.to_radians()).abs() < 1.0e-5);
    }
}
