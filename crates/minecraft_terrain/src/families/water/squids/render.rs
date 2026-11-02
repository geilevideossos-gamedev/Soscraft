use crate::client_mobs::{ClientMobs, MobPose, rot_lerp};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::water::squids::Squid;
use std::f64::consts::{FRAC_PI_2, PI};

pub const SHADOW_RADIUS: f32 = 0.7;
pub const CAGE_MARGIN: f32 = 0.5;
pub const FULL_BRIGHT: f32 = 15.0;
pub const TENTACLES: usize = 8;
pub const TEXTURES: [&str; 4] = [
    "minecraft:entity/squid/squid",
    "minecraft:entity/squid/squid_baby",
    "minecraft:entity/squid/glow_squid",
    "minecraft:entity/squid/glow_squid_baby",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    pub body: Cube,
    pub body_offset: [f32; 3],
    pub body_growth: f32,
    pub tentacle: Cube,
    pub tentacle_reach: f32,
    pub tentacle_y: f32,
    pub texture_size: [f32; 2],
    pub lift: f32,
    pub drop: f32,
}

pub const ADULT: Model = Model {
    body: Cube::new([-6.0, -8.0, -6.0], [12.0, 16.0, 12.0], [0.0, 0.0]),
    body_offset: [0.0, 8.0, 0.0],
    body_growth: 0.02,
    tentacle: Cube::new([-1.0, 0.0, -1.0], [2.0, 18.0, 2.0], [48.0, 0.0]),
    tentacle_reach: 5.0,
    tentacle_y: 15.0,
    texture_size: [64.0, 32.0],
    lift: 0.5,
    drop: 1.2,
};

pub const BABY: Model = Model {
    body: Cube::new([-4.0, -5.0, -4.0], [8.0, 10.0, 8.0], [0.0, 0.0]),
    body_offset: [0.0, 13.0, 0.0],
    body_growth: 0.0,
    tentacle: Cube::new([-1.0, -0.5, -1.0], [2.0, 6.0, 2.0], [0.0, 18.0]),
    tentacle_reach: 3.0,
    tentacle_y: 18.5,
    texture_size: [32.0, 32.0],
    lift: 0.25,
    drop: 0.6,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stance {
    pub yaw: f32,
    pub tilt: f32,
    pub roll: f32,
    pub tentacles: f32,
}

pub fn model(baby: bool) -> &'static Model {
    if baby { &BABY } else { &ADULT }
}

pub fn texture(glow: bool, baby: bool) -> &'static str {
    TEXTURES[usize::from(glow) * 2 + usize::from(baby)]
}

pub fn tentacle_offset(model: &Model, index: usize) -> [f32; 3] {
    let angle = index as f64 * PI * 2.0 / TENTACLES as f64;
    [
        angle.cos() as f32 * model.tentacle_reach,
        model.tentacle_y,
        angle.sin() as f32 * model.tentacle_reach,
    ]
}

pub fn tentacle_turn(index: usize) -> f32 {
    (index as f64 * PI * -2.0 / TENTACLES as f64 + FRAC_PI_2) as f32
}

fn lerp(partial: f32, from: f32, to: f32) -> f32 {
    from + partial * (to - from)
}

pub fn stance(squid: &Squid, yaw: f32, partial: f32) -> Stance {
    Stance {
        yaw: rot_lerp(partial, squid.y_body_rot_o, yaw),
        tilt: lerp(partial, squid.x_body_rot_o, squid.x_body_rot),
        roll: lerp(partial, squid.z_body_rot_o, squid.z_body_rot),
        tentacles: lerp(partial, squid.tentacle_angle_old, squid.tentacle_angle),
    }
}

pub fn glow_light(dark_ticks: i32) -> i32 {
    let fade = 1.0 - dark_ticks as f32 / 10.0;
    let level = if fade < 0.0 {
        0.0
    } else if fade > 1.0 {
        15.0
    } else {
        fade * 15.0
    };
    level as i32
}

pub fn block_light(glow: bool, dark_ticks: i32, block: f32) -> f32 {
    if !glow {
        return block;
    }
    let level = glow_light(dark_ticks);
    if level == 15 {
        FULL_BRIGHT
    } else {
        (level as f32).max(block)
    }
}

pub fn part_poses(model: &Model, tentacles: f32) -> (Mat4, [Mat4; TENTACLES]) {
    let body = part_pose(model.body_offset, [0.0; 3]);
    let arms = std::array::from_fn(|index| {
        let turn = [tentacles, tentacle_turn(index), 0.0];
        part_pose(tentacle_offset(model, index), turn)
    });
    (body, arms)
}

pub fn model_root(pose: &MobPose, model: &Model, stance: &Stance) -> Mat4 {
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.turn)
        * Mat4::from_translation(Vec3::new(0.0, model.lift, 0.0))
        * Mat4::from_rotation_y((180.0 - stance.yaw).to_radians())
        * Mat4::from_rotation_x(stance.tilt.to_radians())
        * Mat4::from_rotation_y(stance.roll.to_radians())
        * Mat4::from_translation(Vec3::new(0.0, -model.drop, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

fn grown(pose: Mat4, cube: &Cube, grow: f32) -> Mat4 {
    let center = Vec3::from_array(cube.from) + Vec3::from_array(cube.size) / 2.0;
    let size = Vec3::from_array(cube.size);
    let scale = (size + Vec3::splat(2.0 * grow)) / size;
    pose * Mat4::from_translation(center / 16.0)
        * Mat4::from_scale(scale)
        * Mat4::from_translation(-center / 16.0)
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, model: &Model, tentacles: f32, skin: &Skin) {
    let (body, arms) = part_poses(model, tentacles);
    let body = grown(body, &model.body, model.body_growth);
    append(mesh, root * body, skin, &model.body);
    for arm in arms {
        append(mesh, root * arm, skin, &model.tentacle);
    }
}

pub fn append_squids<'a>(
    mesh: &mut ChunkMesh,
    squids: impl IntoIterator<Item = &'a FamilyMob<Squid>>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let mut marks = Vec::new();
    for squid in squids {
        let id = squid.core.id;
        let Some(pose) = poses.pose(id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), pose.overlay(0.0)));
        let (glow, baby) = (squid.state.glow, squid.core.baby);
        let model = model(baby);
        let position = pose.light_block();
        let block = f32::from(light.get_block(position));
        let skin = Skin {
            region: ResourceId::parse(texture(glow, baby))
                .map_or([0.0; 4], |texture| atlas.entity_region(&texture)),
            texture_size: model.texture_size,
            sky: f32::from(light.get(position)),
            block: block_light(glow, squid.state.dark_ticks, block),
            alpha: 1.0,
            shaded: true,
        };
        let partial = poses.partial_of(id, partial).clamp(0.0, 1.0);
        let stance = stance(&squid.state, squid.core.yaw, partial);
        let root = model_root(&pose, model, &stance);
        append_model(mesh, root, model, stance.tentacles, &skin);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests;
