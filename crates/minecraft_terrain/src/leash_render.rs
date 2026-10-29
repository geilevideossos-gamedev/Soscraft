use crate::client_mobs::ClientMobs;
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append};
use crate::scene::Block;
use glam::{DVec3, Mat4, Vec3};
use minecraftoss_entities::world::{
    EntityWorld, KNOT_HEIGHT, KNOT_WIDTH, LeashHolder, SavedLeash, knot_position,
};
use std::collections::HashMap;

pub const KNOT_TEXTURE: &str = "minecraft:entity/lead_knot/lead_knot";
pub const STEPS: i32 = 24;
pub const WIDTH: f32 = 0.05;
const KNOT_CUBE: Cube = Cube::new([-3.0, -8.0, -3.0], [6.0, 8.0, 6.0], [0.0, 0.0]);
const KNOT_HOLD: f64 = 0.2;
const KNOT_EYE: f64 = 0.0625;

pub type BlockPos = (i32, i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RopeEnd {
    Mob(u64),
    Player(u128),
    Knot(BlockPos),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotAction {
    Punch,
    Shear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RopeHolder {
    pub feet: DVec3,
    pub yaw: f32,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RopeVertex {
    pub position: Vec3,
    pub shade: f32,
    pub progress: f32,
}

pub fn is_fence_block(block: &Block) -> bool {
    block.id.namespace == "minecraft" && minecraftoss_entities::world::is_fence(&block.id.path)
}

pub fn rope_ends(world: &EntityWorld) -> Vec<(u64, RopeEnd)> {
    world
        .leashes()
        .filter_map(|(mob, leash)| {
            let end = match (leash.holder, leash.saved) {
                (Some(LeashHolder::Player(id)), _) => {
                    RopeEnd::Player(world.uuid_of(crate::thrown::PLAYER_TARGET + id))
                }
                (Some(LeashHolder::Mob(id)), _) => RopeEnd::Mob(id),
                (Some(LeashHolder::Knot(pos)), _) => RopeEnd::Knot(pos),
                (None, Some(SavedLeash::Uuid(uuid))) => world
                    .mob_with_uuid(uuid)
                    .filter(|&id| id != mob)
                    .map_or(RopeEnd::Player(uuid), RopeEnd::Mob),
                (None, Some(SavedLeash::Knot(pos))) => RopeEnd::Knot(pos),
                (None, None) => return None,
            };
            Some((mob, end))
        })
        .collect()
}

pub fn y_rot(v: DVec3, angle: f64) -> DVec3 {
    let (sin, cos) = (angle.sin(), angle.cos());
    DVec3::new(v.x * cos + v.z * sin, v.y, v.z * cos - v.x * sin)
}

pub fn leash_offset(kind: &str, eye_height: f32, width: f32) -> DVec3 {
    let (eye, side) = match kind.strip_prefix("minecraft:").unwrap_or(kind) {
        "llama" | "trader_llama" => (0.75, 0.5),
        "snow_golem" => (0.75, 0.4),
        "ocelot" => (0.5, 0.4),
        "iron_golem" => (0.875, 0.4),
        "rabbit" => (0.6, 0.4),
        "pig" | "wolf" | "strider" => (0.6, 0.4),
        _ => (1.0, 0.4),
    };
    DVec3::new(0.0, f64::from(eye * eye_height), f64::from(side * width))
}

pub fn player_hold(holder: &RopeHolder) -> DVec3 {
    let crouching = holder.height < 1.65;
    let local = DVec3::new(
        -0.22,
        holder.height - 1.0,
        if crouching { -0.2 } else { 0.07 },
    );
    let yaw = f64::from(holder.yaw * 0.017_453_292);
    holder.feet + y_rot(local, -yaw)
}

pub fn knot_hold(pos: BlockPos) -> DVec3 {
    knot_position(pos) + DVec3::new(0.0, KNOT_HOLD, 0.0)
}

pub fn knot_box(pos: BlockPos) -> (DVec3, DVec3) {
    let half = f64::from(KNOT_WIDTH) / 2.0;
    let at = knot_position(pos);
    (
        at - DVec3::new(half, 0.0, half),
        at + DVec3::new(half, f64::from(KNOT_HEIGHT), half),
    )
}

pub fn knot_on_ray(
    knots: &[BlockPos],
    eye: DVec3,
    look: DVec3,
    reach: f64,
) -> Option<(BlockPos, f64)> {
    knots
        .iter()
        .filter_map(|&pos| {
            crate::end_crystal::ray_box(eye, look, reach, knot_box(pos))
                .map(|distance| (pos, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

pub fn rope_strips(start: DVec3, end: DVec3) -> Vec<Vec<RopeVertex>> {
    let d = (end - start).as_vec3();
    let horizontal = d.x * d.x + d.z * d.z;
    let inverse = if horizontal > 0.0 {
        horizontal.sqrt().recip() * WIDTH / 2.0
    } else {
        0.0
    };
    let (aa, bb) = (d.z * inverse, d.x * inverse);
    let pair = |k: i32, fudge: f32, backwards: bool| {
        let progress = k as f32 / STEPS as f32;
        let shade = if k % 2 == i32::from(backwards) {
            0.7
        } else {
            1.0
        };
        let x = d.x * progress;
        let y = if d.y > 0.0 {
            d.y * progress * progress
        } else {
            d.y - d.y * (1.0 - progress) * (1.0 - progress)
        };
        let z = d.z * progress;
        [
            RopeVertex {
                position: Vec3::new(x - aa, y + fudge, z + bb),
                shade,
                progress,
            },
            RopeVertex {
                position: Vec3::new(x + aa, y + WIDTH - fudge, z - bb),
                shade,
                progress,
            },
        ]
    };
    let first = (0..=STEPS).flat_map(|k| pair(k, WIDTH, false)).collect();
    let second = (0..=STEPS).rev().flat_map(|k| pair(k, 0.0, true)).collect();
    vec![first, second]
}

#[allow(clippy::too_many_arguments)]
pub fn append_rope(
    mesh: &mut ChunkMesh,
    start: DVec3,
    end: DVec3,
    uv: [f32; 2],
    start_light: (u8, u8),
    end_light: (u8, u8),
) {
    for strip in rope_strips(start, end) {
        let base = mesh.vertices.len() as u32;
        for vertex in &strip {
            let lerp = |a: u8, b: u8| {
                (f32::from(a) + (f32::from(b) - f32::from(a)) * vertex.progress) as i32 as f32
            };
            mesh.vertices.push(Vertex {
                position: (start.as_vec3() + vertex.position).to_array(),
                uv,
                color: [
                    0.5 * vertex.shade,
                    0.4 * vertex.shade,
                    0.3 * vertex.shade,
                    1.0,
                ],
                sky_light: lerp(start_light.0, end_light.0),
                block_light: lerp(start_light.1, end_light.1),
            });
        }
        for i in 0..strip.len().saturating_sub(2) as u32 {
            let (a, b, c) = (base + i, base + i + 1, base + i + 2);
            if i % 2 == 0 {
                mesh.indices.extend_from_slice(&[a, b, c]);
            } else {
                mesh.indices.extend_from_slice(&[b, a, c]);
            }
            mesh.faces += 1;
        }
    }
}

fn block_of(at: DVec3) -> BlockPos {
    (
        at.x.floor() as i32,
        at.y.floor() as i32,
        at.z.floor() as i32,
    )
}

fn light_at(light: &SkyLight, at: DVec3) -> (u8, u8) {
    let block = block_of(at);
    (light.get(block), light.get_block(block))
}

pub fn knot_pose(pos: BlockPos) -> Mat4 {
    Mat4::from_translation(knot_position(pos).as_vec3())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn append_knots(mesh: &mut ChunkMesh, knots: &[BlockPos], atlas: &Atlas, light: &SkyLight) {
    let Ok(texture) = ResourceId::parse(KNOT_TEXTURE) else {
        return;
    };
    if !atlas.contains(&texture) {
        return;
    }
    let region = atlas.entity_region(&texture);
    for &pos in knots {
        let (sky, block) = light_at(light, knot_position(pos));
        let skin = Skin {
            region,
            texture_size: [32.0, 32.0],
            sky: f32::from(sky),
            block: f32::from(block),
            alpha: 1.0,
            shaded: true,
        };
        append(mesh, knot_pose(pos), &skin, &KNOT_CUBE);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn append_ropes(
    mesh: &mut ChunkMesh,
    world: &EntityWorld,
    poses: &ClientMobs,
    holders: &HashMap<u128, RopeHolder>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let Some(white) = ResourceId::parse(crate::mesh::WHITE_TEXTURE)
        .ok()
        .filter(|white| atlas.contains(white))
        .map(|white| atlas.entity_region(&white))
    else {
        return;
    };
    let uv = [(white[0] + white[2]) / 2.0, (white[1] + white[3]) / 2.0];
    for (mob, end) in rope_ends(world) {
        let Some(pose) = poses.pose(mob, partial) else {
            continue;
        };
        let width = world.body_of(mob).map_or(0.0, |body| body.width);
        let kind = world.entity_type(mob).unwrap_or("");
        let eye = (pose.light_probe.y - pose.feet.y) as f32;
        let yaw = f64::from(pose.body_rot * 0.017_453_292);
        let start = pose.feet + y_rot(leash_offset(kind, eye, width), -yaw);
        let held = match end {
            RopeEnd::Mob(id) => poses.pose(id, partial).map(|holder| {
                let eye = holder.light_probe.y - holder.feet.y;
                (
                    holder.feet + DVec3::new(0.0, eye * 0.7, 0.0),
                    holder.light_probe,
                )
            }),
            RopeEnd::Player(uuid) => holders.get(&uuid).map(|holder| {
                let eye = if holder.height < 1.65 { 1.27 } else { 1.62 };
                (player_hold(holder), holder.feet + DVec3::new(0.0, eye, 0.0))
            }),
            RopeEnd::Knot(pos) => Some((
                knot_hold(pos),
                knot_position(pos) + DVec3::new(0.0, KNOT_EYE, 0.0),
            )),
        };
        let Some((end, end_eye)) = held else {
            continue;
        };
        append_rope(
            mesh,
            start,
            end,
            uv,
            light_at(light, pose.light_probe),
            light_at(light, end_eye),
        );
    }
}

pub fn textures() -> [&'static str; 1] {
    [KNOT_TEXTURE]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rope_is_two_strips_of_twenty_five_pairs_hanging_slack_between_its_ends() {
        let strips = rope_strips(DVec3::ZERO, DVec3::new(4.0, -2.0, 0.0));
        assert_eq!(strips.len(), 2);
        assert!(strips.iter().all(|strip| strip.len() == 50));
        let first = &strips[0];
        assert_eq!(first[0].position, Vec3::new(0.0, 0.05, 0.025));
        assert_eq!(first[1].position, Vec3::new(0.0, 0.0, -0.025));
        let middle = &first[24];
        assert!((middle.position.x - 2.0).abs() < 1e-6);
        assert!((middle.position.y - (-2.0 + 2.0 * 0.25 + 0.05)).abs() < 1e-6);
        assert_eq!((first[0].shade, first[2].shade), (0.7, 1.0));
        assert_eq!((strips[1][0].shade, strips[1][2].shade), (1.0, 0.7));
        assert_eq!(strips[1][0].progress, 1.0);
        let rising = rope_strips(DVec3::ZERO, DVec3::new(0.0, 2.0, 4.0));
        assert!((rising[0][24].position.y - (0.5 + 0.05)).abs() < 1e-6);
        let mut mesh = ChunkMesh::default();
        append_rope(
            &mut mesh,
            DVec3::ZERO,
            DVec3::new(4.0, -2.0, 0.0),
            [0.5, 0.5],
            (15, 0),
            (5, 10),
        );
        assert_eq!(mesh.vertices.len(), 100);
        assert_eq!(mesh.indices.len(), 2 * 48 * 3);
        assert_eq!(mesh.vertices[0].sky_light, 15.0);
        assert_eq!(mesh.vertices[48].block_light, 10.0);
        assert_eq!(
            mesh.vertices[0].color,
            [0.5 * 0.7, 0.4 * 0.7, 0.3 * 0.7, 1.0]
        );
    }

    #[test]
    fn the_rope_leaves_the_mob_at_its_leash_offset_and_the_players_right_hand() {
        let cow = leash_offset("minecraft:cow", 1.3, 0.9);
        assert!((cow - DVec3::new(0.0, 1.3, 0.36)).length() < 1e-6);
        let llama = leash_offset("minecraft:trader_llama", 1.7765, 0.9);
        assert!((llama - DVec3::new(0.0, 0.75 * 1.7765, 0.45)).length() < 1e-5);
        let golem = leash_offset("minecraft:iron_golem", 2.295, 1.4);
        assert!((golem.y - 0.875 * 2.295).abs() < 1e-5);
        let ahead = y_rot(DVec3::new(0.0, 0.0, 1.0), -f64::from(90.0_f32.to_radians()));
        assert!(
            (ahead - DVec3::new(-1.0, 0.0, 0.0)).length() < 1e-6,
            "{ahead}"
        );
        let standing = RopeHolder {
            feet: DVec3::new(10.0, 64.0, 10.0),
            yaw: 0.0,
            height: 1.8,
        };
        assert!((player_hold(&standing) - DVec3::new(9.78, 64.8, 10.07)).length() < 1e-6);
        let crouching = RopeHolder {
            height: 1.5,
            ..standing
        };
        assert!((player_hold(&crouching) - DVec3::new(9.78, 64.5, 9.8)).length() < 1e-6);
        assert!((knot_hold((3, 64, -2)) - DVec3::new(3.5, 64.575, -1.5)).length() < 1e-9);
    }

    #[test]
    fn a_knot_is_a_six_by_eight_cube_on_top_of_its_fence_post() {
        let mut mesh = ChunkMesh::default();
        let skin = Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: [32.0, 32.0],
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        };
        append(&mut mesh, knot_pose((3, 64, 0)), &skin, &KNOT_CUBE);
        assert_eq!(mesh.faces, 6);
        let low = mesh.vertices.iter().fold(Vec3::splat(f32::MAX), |a, v| {
            a.min(Vec3::from_array(v.position))
        });
        let high = mesh.vertices.iter().fold(Vec3::splat(f32::MIN), |a, v| {
            a.max(Vec3::from_array(v.position))
        });
        assert!(
            (low - Vec3::new(3.3125, 64.375, 0.3125)).length() < 1e-5,
            "{low}"
        );
        assert!(
            (high - Vec3::new(3.6875, 64.875, 0.6875)).length() < 1e-5,
            "{high}"
        );
    }

    #[test]
    fn a_knot_is_hit_from_the_side_before_its_fence_post_but_not_from_above() {
        let fence = (3, 64, 0);
        let (low, high) = knot_box(fence);
        assert!((low - DVec3::new(3.3125, 64.375, 0.3125)).length() < 1e-9, "{low}");
        assert!((high - DVec3::new(3.6875, 64.875, 0.6875)).length() < 1e-9, "{high}");
        let post = minecraftoss_player::outline_bounds(&minecraftoss_player::Block::new(
            "minecraft:oak_fence",
        ))
        .expect("a fence post");
        let post = (
            DVec3::new(3.0 + post[0], 64.0 + post[1], post[2]),
            DVec3::new(3.0 + post[3], 64.0 + post[4], post[5]),
        );
        let ray = crate::end_crystal::ray_box;
        let side = DVec3::new(1.0, 64.6, 0.5);
        let (pos, knot) = knot_on_ray(&[(9, 64, 9), fence], side, DVec3::X, 3.0).expect("a knot");
        assert_eq!(pos, fence);
        assert!((knot - 2.3125).abs() < 1e-9, "{knot}");
        assert!(knot < ray(side, DVec3::X, 3.0, post).expect("the post"));
        let above = DVec3::new(3.5, 66.0, 0.5);
        let (_, knot) = knot_on_ray(&[fence], above, DVec3::NEG_Y, 3.0).expect("a knot");
        let top = ray(above, DVec3::NEG_Y, 3.0, post).expect("the post");
        assert!((top - 1.0).abs() < 1e-9 && top < knot, "{top} {knot}");
        let below = DVec3::new(1.0, 64.2, 0.5);
        assert_eq!(knot_on_ray(&[fence], below, DVec3::X, 3.0), None);
        assert!(ray(below, DVec3::X, 3.0, post).is_some());
        let far = DVec3::new(0.0, 64.6, 0.5);
        assert_eq!(knot_on_ray(&[fence], far, DVec3::X, 3.0), None, "beyond reach");
    }

    #[test]
    fn a_friend_draws_a_saved_leash_to_its_player_mob_or_knot() {
        use minecraftoss_entities::cow::Cow;
        let mut world = EntityWorld::default();
        let cow = world.spawn_cow(Cow::new(DVec3::new(0.5, 64.0, 0.5)), true);
        let trader = world.spawn_cow(Cow::new(DVec3::new(2.5, 64.0, 0.5)), true);
        let knotted = world.spawn_cow(Cow::new(DVec3::new(4.5, 64.0, 0.5)), true);
        world.set_uuid(trader, 0x77);
        world.load_leash(cow, SavedLeash::Uuid(0x99));
        world.load_leash(trader, SavedLeash::Knot((1, 64, 1)));
        world.load_leash(knotted, SavedLeash::Uuid(0x77));
        let ends = rope_ends(&world);
        assert_eq!(
            ends,
            [
                (cow, RopeEnd::Player(0x99)),
                (trader, RopeEnd::Knot((1, 64, 1))),
                (knotted, RopeEnd::Mob(trader)),
            ]
        );
        assert_eq!(world.knots(), [(1, 64, 1)]);
    }
}
