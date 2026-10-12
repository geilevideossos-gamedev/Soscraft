use crate::{
    client_mobs::ClientMobs,
    cow_render::cube_scaled,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::ResourceId,
};
use glam::{DVec3, Quat, Vec3};
use minecraftoss_entities::world::{ShulkerBulletEntity, ShulkerEntity};
use minecraftoss_player::Face;
use std::f32::consts::{FRAC_PI_2, PI};

pub type Part = ([f32; 3], [f32; 3], [f32; 2]);

pub const LID: Part = ([-8.0, -16.0, -8.0], [8.0, -4.0, 8.0], [0.0, 0.0]);
pub const BASE: Part = ([-8.0, -8.0, -8.0], [8.0, 0.0, 8.0], [0.0, 28.0]);
pub const HEAD: Part = ([-3.0, 0.0, -3.0], [3.0, 6.0, 3.0], [0.0, 52.0]);
const TEXTURE_SIZE: [f32; 2] = [64.0, 64.0];
const PIVOT: Vec3 = Vec3::new(0.0, 0.5, 0.0);
pub const SPARK: &str = "minecraft:entity/shulker/spark";
pub const BULLET_PARTS: [Part; 3] = [
    ([-4.0, -4.0, -1.0], [4.0, 4.0, 1.0], [0.0, 0.0]),
    ([-1.0, -4.0, -4.0], [1.0, 4.0, 4.0], [0.0, 10.0]),
    ([-4.0, -1.0, -4.0], [4.0, 1.0, 4.0], [20.0, 0.0]),
];
const BULLET_TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];
const BULLET_LIFT: f64 = 0.15;
const BULLET_SCALE: f32 = 0.5;
const GLOW_SCALE: f32 = 1.5;
pub const GLOW_ALPHA: f32 = 38.0 / 255.0;
const MODEL_ORIGIN: [f32; 3] = [0.0, 24.016, 0.0];

fn sin(x: f32) -> f32 {
    minecraftoss_player::mth::sin(f64::from(x))
}

fn cos(x: f32) -> f32 {
    minecraftoss_player::mth::cos(f64::from(x))
}

pub fn texture(color: Option<u8>) -> ResourceId {
    let name = match color.and_then(|c| crate::special_models::DYES.get(usize::from(c))) {
        Some((dye, _)) => format!("minecraft:entity/shulker/shulker_{dye}"),
        None => "minecraft:entity/shulker/shulker".to_owned(),
    };
    ResourceId::parse(&name).unwrap()
}

pub fn lid_pose(peek: f32, age_in_ticks: f32) -> (f32, f32) {
    let g = (0.5 + peek) * PI;
    let h = -1.0 + sin(g);
    let wobble = if g > PI {
        sin(age_in_ticks * 0.1) * 0.7
    } else {
        0.0
    };
    let y = 16.0 + sin(g) * 8.0 + wobble;
    let turn = if peek > 0.3 {
        h * h * h * h * PI * 0.125
    } else {
        0.0
    };
    (y, turn)
}

pub fn head_turn(y_head_rot: f32, y_body_rot: f32) -> f32 {
    (y_head_rot - 180.0 - y_body_rot) * 0.017453292
}

pub fn face_rotation(face: Face) -> Quat {
    match face {
        Face::Down => Quat::from_rotation_x(PI),
        Face::Up => Quat::IDENTITY,
        Face::North => Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(PI),
        Face::South => Quat::from_rotation_x(FRAC_PI_2),
        Face::West => Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(FRAC_PI_2),
        Face::East => Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(-FRAC_PI_2),
    }
}

pub fn death_flip(death_time: f32) -> Quat {
    if death_time > 0.0 {
        let fall = ((death_time - 1.0) / 20.0 * 1.6).sqrt().min(1.0);
        Quat::from_rotation_z((fall * 90.0).to_radians())
    } else {
        Quat::IDENTITY
    }
}

pub struct ShulkerPose {
    pub feet: DVec3,
    pub opening: Face,
    pub peek: f32,
    pub age_in_ticks: f32,
    pub head_yaw: f32,
    pub death_time: f32,
    pub turn: Quat,
}

pub fn append_shulker(
    mesh: &mut ChunkMesh,
    pose: &ShulkerPose,
    region: [f32; 4],
    sky: f32,
    block: f32,
) {
    let body = pose.turn * death_flip(pose.death_time);
    let face = face_rotation(pose.opening);
    let feet = pose.feet + (body * (PIVOT - face * PIVOT)).as_dvec3();
    let rotation = body * face;
    let (lid_y, lid_turn) = lid_pose(pose.peek, pose.age_in_ticks);
    let parts = [
        (LID, [0.0, lid_y, 0.0], Quat::from_rotation_y(lid_turn)),
        (BASE, [0.0, 24.0, 0.0], Quat::IDENTITY),
        (
            HEAD,
            [0.0, 12.0, 0.0],
            Quat::from_rotation_y(head_turn(pose.head_yaw, 0.0)),
        ),
    ];
    for ((from, to, uv), pivot, part) in parts {
        cube_scaled(
            mesh,
            feet,
            rotation,
            Vec3::ONE,
            region,
            sky,
            block,
            from,
            to,
            uv,
            pivot,
            part,
            [1.0; 3],
            TEXTURE_SIZE,
            None,
            false,
        );
    }
}

pub fn append_shulkers<'a>(
    mesh: &mut ChunkMesh,
    shulkers: impl IntoIterator<Item = &'a ShulkerEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let partial = partial.clamp(0.0, 1.0);
    let mut marks = Vec::new();
    for entity in shulkers {
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        let shulker = &entity.shulker;
        let region = atlas.entity_region(&texture(shulker.color));
        let feet = shulker.body.position + shulker.render_offset(partial);
        let eye = feet + DVec3::Y * f64::from(shulker.eye_height());
        let cell = (
            eye.x.floor() as i32,
            eye.y.floor() as i32,
            eye.z.floor() as i32,
        );
        let (sky, block) = (light.get(cell) as f32, light.get_block(cell) as f32);
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let pose = ShulkerPose {
            feet,
            opening: shulker.opening(),
            peek: shulker.client_peek(partial),
            age_in_ticks: mob.age_in_ticks,
            head_yaw: mob.body_rot + mob.head_yaw,
            death_time: mob.death_time,
            turn: mob.turn,
        };
        append_shulker(mesh, &pose, region, sky, block);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

pub fn bullet_spin(age_in_ticks: f32) -> Quat {
    let yaw = sin(age_in_ticks * 0.1) * 180.0;
    let pitch = cos(age_in_ticks * 0.1) * 180.0;
    let roll = sin(age_in_ticks * 0.15) * 360.0;
    Quat::from_rotation_y(yaw.to_radians())
        * Quat::from_rotation_x(pitch.to_radians())
        * Quat::from_rotation_z(roll.to_radians())
}

pub struct BulletPose {
    pub feet: DVec3,
    pub age_in_ticks: f32,
    pub yaw: f32,
    pub pitch: f32,
}

pub fn append_bullet(
    core: &mut ChunkMesh,
    glow: &mut ChunkMesh,
    pose: &BulletPose,
    region: [f32; 4],
) {
    let feet = pose.feet + DVec3::Y * BULLET_LIFT;
    let spin = bullet_spin(pose.age_in_ticks);
    let turn = Quat::from_rotation_y(pose.yaw * 0.017453292)
        * Quat::from_rotation_x(pose.pitch * 0.017453292);
    for (mesh, scale, alpha) in [
        (core, BULLET_SCALE, 1.0),
        (glow, BULLET_SCALE * GLOW_SCALE, GLOW_ALPHA),
    ] {
        let start = mesh.vertices.len();
        for (from, to, uv) in BULLET_PARTS {
            cube_scaled(
                mesh,
                feet,
                spin,
                Vec3::splat(scale),
                region,
                15.0,
                15.0,
                from,
                to,
                uv,
                MODEL_ORIGIN,
                turn,
                [1.0; 3],
                BULLET_TEXTURE_SIZE,
                None,
                false,
            );
        }
        for vertex in &mut mesh.vertices[start..] {
            vertex.color[3] = alpha;
        }
    }
}

pub fn append_bullets<'a>(
    core: &mut ChunkMesh,
    glow: &mut ChunkMesh,
    bullets: impl IntoIterator<Item = &'a ShulkerBulletEntity>,
    atlas: &Atlas,
    partial: f32,
) {
    let partial = partial.clamp(0.0, 1.0);
    let Ok(spark) = ResourceId::parse(SPARK) else {
        return;
    };
    let region = atlas.entity_region(&spark);
    for entity in bullets.into_iter().filter(|e| e.bullet.alive) {
        let bullet = &entity.bullet;
        let (yaw, pitch) = bullet.turn(partial);
        let pose = BulletPose {
            feet: entity
                .previous_position
                .lerp(bullet.position, f64::from(partial)),
            age_in_ticks: bullet.tick_count as f32 + partial,
            yaw,
            pitch,
        };
        append_bullet(core, glow, &pose, region);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(opening: Face, peek: f32, head_yaw: f32) -> ShulkerPose {
        ShulkerPose {
            feet: DVec3::new(0.5, 64.0, 0.5),
            opening,
            peek,
            age_in_ticks: 0.0,
            head_yaw,
            death_time: 0.0,
            turn: Quat::IDENTITY,
        }
    }

    fn mesh_of(pose: &ShulkerPose) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();
        append_shulker(&mut mesh, pose, [0.0, 0.0, 1.0, 1.0], 15.0, 0.0);
        mesh
    }

    fn positions(mesh: &ChunkMesh, range: std::ops::Range<usize>) -> Vec<Vec3> {
        mesh.vertices[range]
            .iter()
            .map(|v| Vec3::from_array(v.position))
            .collect()
    }

    #[test]
    fn a_shulker_is_a_lid_a_base_and_a_head() {
        let mesh = mesh_of(&pose(Face::Up, 0.0, 180.0));
        assert_eq!(mesh.vertices.len(), 3 * 24);
        let base = positions(&mesh, 24..48);
        let low = base.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        let high = base.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!((low - 64.0).abs() < 0.01 && (high - 64.5).abs() < 0.01);
        let lid = positions(&mesh, 0..24);
        let top = lid.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!(
            (top - 65.0).abs() < 0.01,
            "closed, the lid tops the block: {top}"
        );
    }

    #[test]
    fn the_lid_rises_and_turns_as_it_peeks() {
        assert!((lid_pose(0.0, 0.0).0 - 24.0).abs() < 1e-4);
        assert!((lid_pose(0.3, 0.0).0 - 20.702).abs() < 1e-2);
        assert!((lid_pose(1.0, 0.0).0 - 8.0).abs() < 1e-4);
        assert_eq!(lid_pose(0.3, 0.0).1, 0.0);
        let h = -1.0 + sin(0.9 * PI);
        assert!((lid_pose(0.4, 0.0).1 - h.powi(4) * PI / 8.0).abs() < 1e-6);
        assert_ne!(
            lid_pose(0.8, 0.0).0,
            lid_pose(0.8, 10.0).0,
            "it wobbles open"
        );
        assert_eq!(lid_pose(0.4, 0.0).0, lid_pose(0.4, 10.0).0);
    }

    #[test]
    fn the_head_follows_the_head_yaw() {
        assert!(head_turn(180.0, 0.0).abs() < 1e-6);
        assert!((head_turn(270.0, 0.0) - FRAC_PI_2).abs() < 1e-5);
        let ahead = mesh_of(&pose(Face::Up, 0.0, 180.0));
        let turned = mesh_of(&pose(Face::Up, 0.0, 270.0));
        assert_eq!(positions(&ahead, 0..48), positions(&turned, 0..48));
        assert_ne!(positions(&ahead, 48..72), positions(&turned, 48..72));
    }

    #[test]
    fn a_ceiling_shulker_is_the_floor_one_turned_over_about_the_block_centre() {
        let floor = mesh_of(&pose(Face::Up, 0.5, 200.0));
        let ceiling = mesh_of(&pose(Face::Down, 0.5, 200.0));
        let centre = Vec3::new(0.5, 64.5, 0.5);
        let over = Quat::from_rotation_x(PI);
        for (a, b) in floor.vertices.iter().zip(&ceiling.vertices) {
            let turned = centre + over * (Vec3::from_array(a.position) - centre);
            assert!(
                (turned - Vec3::from_array(b.position)).length() < 1e-3,
                "{turned} {:?}",
                b.position
            );
        }
        let wall = mesh_of(&pose(Face::East, 0.0, 180.0));
        let xs: Vec<f32> = wall.vertices.iter().map(|v| v.position[0]).collect();
        let low = xs.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            (low - 0.0).abs() < 0.01,
            "a wall shulker sits on its west face: {low}"
        );
    }

    fn bullet_meshes(age_in_ticks: f32) -> (ChunkMesh, ChunkMesh) {
        let (mut core, mut glow) = (ChunkMesh::default(), ChunkMesh::default());
        let pose = BulletPose {
            feet: DVec3::new(0.5, 64.0, 0.5),
            age_in_ticks,
            yaw: 0.0,
            pitch: 0.0,
        };
        append_bullet(&mut core, &mut glow, &pose, [0.0, 0.0, 1.0, 1.0]);
        (core, glow)
    }

    fn reach(mesh: &ChunkMesh) -> f32 {
        let centre = Vec3::new(0.5, 64.15, 0.5);
        mesh.vertices
            .iter()
            .map(|v| (Vec3::from_array(v.position) - centre).length())
            .fold(0.0, f32::max)
    }

    #[test]
    fn a_bullet_is_three_crossed_slabs_in_a_faint_bigger_glow() {
        let (core, glow) = bullet_meshes(0.0);
        assert_eq!((core.vertices.len(), glow.vertices.len()), (3 * 24, 3 * 24));
        assert!(core.vertices.iter().all(|v| v.color[3] == 1.0));
        assert!(
            glow.vertices
                .iter()
                .all(|v| (v.color[3] - 0.149).abs() < 1e-3)
        );
        assert!(
            core.vertices
                .iter()
                .all(|v| v.sky_light == 15.0 && v.block_light == 15.0)
        );
        let corner = (4.0_f32 * 4.0 + 4.0 * 4.0 + 1.0).sqrt() / 16.0 * 0.5;
        assert!((reach(&core) - corner).abs() < 1e-3, "{}", reach(&core));
        assert!(
            (reach(&glow) - corner * 1.5).abs() < 1e-3,
            "{}",
            reach(&glow)
        );
    }

    #[test]
    fn a_bullet_spins_with_its_age() {
        let still = bullet_spin(0.0);
        let want = Quat::from_rotation_x(PI);
        assert!(still.dot(want).abs() > 0.9999, "{still:?}");
        let ten = bullet_spin(10.0);
        let yaw = sin(1.0) * 180.0;
        let pitch = cos(1.0) * 180.0;
        let roll = sin(1.5) * 360.0;
        let want = Quat::from_rotation_y(yaw.to_radians())
            * Quat::from_rotation_x(pitch.to_radians())
            * Quat::from_rotation_z(roll.to_radians());
        assert!(ten.dot(want).abs() > 0.9999);
        assert_ne!(
            bullet_meshes(0.0).0.vertices[0].position,
            bullet_meshes(10.0).0.vertices[0].position
        );
    }

    #[test]
    fn its_colour_picks_its_sheet() {
        assert_eq!(
            texture(Some(14)),
            ResourceId::parse("minecraft:entity/shulker/shulker_red").unwrap()
        );
        assert_eq!(
            texture(Some(0)),
            ResourceId::parse("minecraft:entity/shulker/shulker_white").unwrap()
        );
        for none in [None, Some(16), Some(200)] {
            assert_eq!(
                texture(none),
                ResourceId::parse("minecraft:entity/shulker/shulker").unwrap()
            );
        }
    }
}
