use crate::client_mobs::{ClientMobs, MobPose, mth_cos};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec2, Vec3};
use minecraftoss_entities::fireball::FireballKind;
use minecraftoss_entities::wither::{INVULNERABLE_TICKS, Wither};
use minecraftoss_entities::world::{FireballEntity, WitherEntity};
use std::f32::consts::PI;

pub const TEXTURE: &str = "minecraft:entity/wither/wither";
pub const INVULNERABLE_TEXTURE: &str = "minecraft:entity/wither/wither_invulnerable";
pub const ARMOR_TEXTURE: &str = "minecraft:entity/wither/wither_armor";
pub const TEXTURES: [&str; 3] = [TEXTURE, INVULNERABLE_TEXTURE, ARMOR_TEXTURE];
pub const TEXTURE_SIZE: [f32; 2] = [64.0, 64.0];
pub const SHADOW_RADIUS: f32 = 1.0;
pub const MODEL_SCALE: f32 = 2.0;
pub const CHARGE_SHRINK: f32 = 0.5;
pub const ARMOR_GROW: f32 = 0.5;
pub const ARMOR_ALPHA: f32 = 0.5;
pub const FULL_BRIGHT: f32 = 15.0;
const RIBCAGE_TILT: f32 = 0.204_203_52;
const TAIL_TILT: f32 = 0.832_522;
const DEG: f32 = 0.017_453_292;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Shoulders,
    Ribcage,
    Tail,
    CenterHead,
    RightHead,
    LeftHead,
}

const SIDE_HEAD: Cube = Cube::new([-4.0, -4.0, -4.0], [6.0, 6.0, 6.0], [32.0, 0.0]);
pub const SKULL: Cube = Cube::new([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], [0.0, 35.0]);

const CUBES: [(Part, Cube); 9] = [
    (
        Part::Shoulders,
        Cube::new([-10.0, 3.9, -0.5], [20.0, 3.0, 3.0], [0.0, 16.0]),
    ),
    (
        Part::Ribcage,
        Cube::new([0.0, 0.0, 0.0], [3.0, 10.0, 3.0], [0.0, 22.0]),
    ),
    (
        Part::Ribcage,
        Cube::new([-4.0, 1.5, 0.5], [11.0, 2.0, 2.0], [24.0, 22.0]),
    ),
    (
        Part::Ribcage,
        Cube::new([-4.0, 4.0, 0.5], [11.0, 2.0, 2.0], [24.0, 22.0]),
    ),
    (
        Part::Ribcage,
        Cube::new([-4.0, 6.5, 0.5], [11.0, 2.0, 2.0], [24.0, 22.0]),
    ),
    (
        Part::Tail,
        Cube::new([0.0, 0.0, 0.0], [3.0, 6.0, 3.0], [12.0, 22.0]),
    ),
    (
        Part::CenterHead,
        Cube::new([-4.0, -4.0, -4.0], [8.0, 8.0, 8.0], [0.0, 0.0]),
    ),
    (Part::RightHead, SIDE_HEAD),
    (Part::LeftHead, SIDE_HEAD),
];

fn mth_sin(x: f32) -> f32 {
    minecraftoss_player::mth::sin(f64::from(x))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WitherLook {
    pub invulnerable: f32,
    pub head_x_rots: [f32; 2],
    pub head_y_rots: [f32; 2],
    pub powered: bool,
}

impl WitherLook {
    pub fn of(wither: &Wither, partial: f32) -> Self {
        let ticks = wither.invulnerable_ticks;
        Self {
            invulnerable: if ticks > 0 {
                ticks as f32 - partial
            } else {
                0.0
            },
            head_x_rots: wither.head_x_rots,
            head_y_rots: wither.head_y_rots,
            powered: wither.powered(),
        }
    }

    pub fn scale(&self) -> f32 {
        if self.invulnerable > 0.0 {
            MODEL_SCALE - self.invulnerable / INVULNERABLE_TICKS as f32 * CHARGE_SHRINK
        } else {
            MODEL_SCALE
        }
    }

    pub fn texture(&self) -> &'static str {
        let ticks = self.invulnerable.floor() as i32;
        if ticks > 0 && (ticks > 80 || ticks / 5 % 2 != 1) {
            INVULNERABLE_TEXTURE
        } else {
            TEXTURE
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WitherPose {
    pub shoulders: Mat4,
    pub ribcage: Mat4,
    pub tail: Mat4,
    pub center_head: Mat4,
    pub right_head: Mat4,
    pub left_head: Mat4,
    pub ribcage_pitch: f32,
    pub tail_offset: [f32; 3],
    pub tail_pitch: f32,
}

impl WitherPose {
    fn of(&self, part: Part) -> Mat4 {
        match part {
            Part::Shoulders => self.shoulders,
            Part::Ribcage => self.ribcage,
            Part::Tail => self.tail,
            Part::CenterHead => self.center_head,
            Part::RightHead => self.right_head,
            Part::LeftHead => self.left_head,
        }
    }
}

pub fn wither_pose(mob: &MobPose, look: &WitherLook) -> WitherPose {
    let wave = mth_cos(mob.age_in_ticks * 0.1);
    let ribcage_pitch = (0.065 + 0.05 * wave) * PI;
    let tail_offset = [
        -2.0,
        6.9 + mth_cos(ribcage_pitch) * 10.0,
        -0.5 + mth_sin(ribcage_pitch) * 10.0,
    ];
    let tail_pitch = (0.265 + 0.1 * wave) * PI;
    let side = |i: usize| {
        [
            look.head_x_rots[i] * DEG,
            (look.head_y_rots[i] - mob.body_rot) * DEG,
            0.0,
        ]
    };
    WitherPose {
        shoulders: Mat4::IDENTITY,
        ribcage: part_pose([-2.0, 6.9, -0.5], [ribcage_pitch, 0.0, 0.0]),
        tail: part_pose(tail_offset, [tail_pitch, 0.0, 0.0]),
        center_head: part_pose([0.0; 3], [mob.head_pitch * DEG, mob.head_yaw * DEG, 0.0]),
        right_head: part_pose([-8.0, 4.0, 0.0], side(0)),
        left_head: part_pose([10.0, 4.0, 0.0], side(1)),
        ribcage_pitch,
        tail_offset,
        tail_pitch,
    }
}

pub fn rest_pose() -> WitherPose {
    WitherPose {
        shoulders: Mat4::IDENTITY,
        ribcage: part_pose([-2.0, 6.9, -0.5], [RIBCAGE_TILT, 0.0, 0.0]),
        tail: part_pose(
            [
                -2.0,
                6.9 + mth_cos(RIBCAGE_TILT) * 10.0,
                -0.5 + mth_sin(RIBCAGE_TILT) * 10.0,
            ],
            [TAIL_TILT, 0.0, 0.0],
        ),
        center_head: Mat4::IDENTITY,
        right_head: part_pose([-8.0, 4.0, 0.0], [0.0; 3]),
        left_head: part_pose([10.0, 4.0, 0.0], [0.0; 3]),
        ribcage_pitch: RIBCAGE_TILT,
        tail_offset: [
            -2.0,
            6.9 + mth_cos(RIBCAGE_TILT) * 10.0,
            -0.5 + mth_sin(RIBCAGE_TILT) * 10.0,
        ],
        tail_pitch: TAIL_TILT,
    }
}

pub fn model_root(feet: Mat4, scale: f32) -> Mat4 {
    feet * Mat4::from_translation(Vec3::new(0.0, 1.501 * scale, 0.0))
        * Mat4::from_scale(Vec3::new(-scale, -scale, scale))
}

fn deformed(pose: Mat4, cube: &Cube, grow: f32) -> Mat4 {
    let center = Vec3::from_array(cube.from) + Vec3::from_array(cube.size) / 2.0;
    let size = Vec3::from_array(cube.size);
    let scale = (size + Vec3::splat(2.0 * grow)) / size;
    pose * Mat4::from_translation(center / 16.0)
        * Mat4::from_scale(scale)
        * Mat4::from_translation(-center / 16.0)
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, pose: &WitherPose) {
    for (part, cube) in &CUBES {
        append(mesh, root * pose.of(*part), skin, cube);
    }
}

pub fn swirl_shift(age_in_ticks: f32) -> Vec2 {
    Vec2::new(
        (mth_cos(age_in_ticks * 0.02) * 3.0) % 1.0,
        (age_in_ticks * 0.01) % 1.0,
    )
}

fn crossings(from: f32, to: f32) -> Vec<f32> {
    let (low, high) = if from <= to { (from, to) } else { (to, from) };
    let mut cuts = vec![0.0];
    if (to - from).abs() > f32::EPSILON {
        let mut k = low.floor() + 1.0;
        while k < high {
            cuts.push((k - from) / (to - from));
            k += 1.0;
        }
    }
    cuts.push(1.0);
    cuts.sort_by(f32::total_cmp);
    cuts
}

pub(crate) fn wrap_face(out: &mut ChunkMesh, quad: &[Vertex], region: [f32; 4], shift: Vec2) {
    let size = Vec2::new(region[2] - region[0], region[3] - region[1]);
    let local = |v: &Vertex| (Vec2::new(v.uv[0] - region[0], v.uv[1] - region[1]) / size) + shift;
    let (p0, p1, p3) = (
        Vec3::from_array(quad[0].position),
        Vec3::from_array(quad[1].position),
        Vec3::from_array(quad[3].position),
    );
    let (n0, n1, n3) = (local(&quad[0]), local(&quad[1]), local(&quad[3]));
    let (ds, dt) = (n1 - n0, n3 - n0);
    let along = |d: Vec2, start: Vec2| {
        if d.x.abs() > d.y.abs() {
            crossings(start.x, start.x + d.x)
        } else {
            crossings(start.y, start.y + d.y)
        }
    };
    let s_cuts = along(ds, n0);
    let t_cuts = along(dt, n0);
    for s in s_cuts.windows(2) {
        for t in t_cuts.windows(2) {
            let mid = n0 + ds * ((s[0] + s[1]) / 2.0) + dt * ((t[0] + t[1]) / 2.0);
            let floor = mid.floor();
            let start = out.vertices.len() as u32;
            for (a, b) in [(s[0], t[0]), (s[1], t[0]), (s[1], t[1]), (s[0], t[1])] {
                let position = p0 + (p1 - p0) * a + (p3 - p0) * b;
                let uv = (n0 + ds * a + dt * b - floor) * size;
                out.vertices.push(Vertex {
                    position: position.to_array(),
                    uv: [region[0] + uv.x, region[1] + uv.y],
                    ..quad[0]
                });
            }
            out.indices.extend_from_slice(&[
                start,
                start + 1,
                start + 2,
                start,
                start + 2,
                start + 3,
            ]);
            out.faces += 1;
        }
    }
}

pub fn append_swirl(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, pose: &WitherPose, shift: Vec2) {
    for (part, cube) in &CUBES {
        let mut cube_mesh = ChunkMesh::default();
        append(
            &mut cube_mesh,
            deformed(root * pose.of(*part), cube, ARMOR_GROW),
            skin,
            cube,
        );
        for quad in cube_mesh.vertices.chunks(4) {
            wrap_face(mesh, quad, skin.region, shift);
        }
    }
}

pub fn append_withers<'a>(
    mesh: &mut ChunkMesh,
    translucent: &mut ChunkMesh,
    withers: impl IntoIterator<Item = &'a WitherEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let (Ok(plain), Ok(charging), Ok(armor)) = (
        ResourceId::parse(TEXTURE),
        ResourceId::parse(INVULNERABLE_TEXTURE),
        ResourceId::parse(ARMOR_TEXTURE),
    ) else {
        return;
    };
    let mut marks = Vec::new();
    for entity in withers {
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        let look = WitherLook::of(&entity.wither, partial);
        let texture = if look.texture() == TEXTURE {
            &plain
        } else {
            &charging
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let position = mob.light_block();
        let skin = Skin {
            region: atlas.entity_region(texture),
            texture_size: TEXTURE_SIZE,
            sky: light.get(position) as f32,
            block: FULL_BRIGHT,
            alpha: 1.0,
            shaded: true,
        };
        let pose = wither_pose(&mob, &look);
        let feet =
            Mat4::from_translation(mob.feet.as_vec3()) * Mat4::from_quat(mob.body_rotation(90.0));
        let root = model_root(feet, look.scale());
        append_model(mesh, root, &skin, &pose);
        if look.powered {
            let swirl = Skin {
                region: atlas.entity_region(&armor),
                texture_size: TEXTURE_SIZE,
                sky: FULL_BRIGHT,
                block: FULL_BRIGHT,
                alpha: ARMOR_ALPHA,
                shaded: false,
            };
            append_swirl(
                translucent,
                root,
                &swirl,
                &pose,
                swirl_shift(mob.age_in_ticks),
            );
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

pub fn skull_texture(kind: FireballKind) -> Option<&'static str> {
    match kind {
        FireballKind::WitherSkull { dangerous: true } => Some(INVULNERABLE_TEXTURE),
        FireballKind::WitherSkull { dangerous: false } => Some(TEXTURE),
        _ => None,
    }
}

pub fn skull_root(at: glam::DVec3, yaw: f32, pitch: f32) -> Mat4 {
    Mat4::from_translation(at.as_vec3())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * part_pose([0.0; 3], [pitch * DEG, yaw * DEG, 0.0])
}

pub fn append_skulls<'a>(
    mesh: &mut ChunkMesh,
    skulls: impl IntoIterator<Item = (&'a FireballEntity, glam::DVec3)>,
    atlas: &Atlas,
    light: &SkyLight,
) {
    for (entity, at) in skulls {
        let Some(texture) = skull_texture(entity.fireball.kind) else {
            continue;
        };
        let Ok(texture) = ResourceId::parse(texture) else {
            continue;
        };
        let block = (
            at.x.floor() as i32,
            at.y.floor() as i32,
            at.z.floor() as i32,
        );
        let skin = Skin {
            region: atlas.entity_region(&texture),
            texture_size: TEXTURE_SIZE,
            sky: light.get(block) as f32,
            block: FULL_BRIGHT,
            alpha: 1.0,
            shaded: true,
        };
        let root = skull_root(at, entity.fireball.yaw, entity.fireball.pitch);
        append(mesh, root, &skin, &SKULL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standing() -> MobPose {
        MobPose {
            feet: glam::DVec3::ZERO,
            body_rot: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
            walk_position: 0.0,
            walk_speed: 0.0,
            age_in_ticks: 0.0,
            light_probe: glam::DVec3::ZERO,
            flap: 0.0,
            flap_speed: 0.0,
            resting: false,
            rest_start: None,
            fly_start: None,
            swing: None,
            red_overlay: false,
            death_time: 0.0,
            turn: glam::Quat::IDENTITY,
            riding: false,
        }
    }

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: TEXTURE_SIZE,
            sky: 15.0,
            block: 15.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
    }

    #[test]
    fn a_wither_is_drawn_at_twice_its_model_size_about_three_and_a_half_blocks_tall() {
        let mut mesh = ChunkMesh::default();
        let look = WitherLook::default();
        let pose = wither_pose(&standing(), &look);
        append_model(
            &mut mesh,
            model_root(Mat4::IDENTITY, look.scale()),
            &skin(),
            &pose,
        );
        assert_eq!(mesh.faces, 54, "nine cubes");
        let (low, high) = span(&mesh, 1);
        assert!(high > 3.4 && high < 3.6, "the centre head's top: {high}");
        assert!(
            low > 0.0 && low < 1.5,
            "the tail hangs above its feet: {low}"
        );
        let (left, right) = span(&mesh, 0);
        assert!(
            (right - left - 3.0).abs() < 0.01,
            "the side heads span 24 pixels at twice the size: {left} {right}"
        );
    }

    #[test]
    fn a_charging_wither_grows_and_flashes_its_blue_skin() {
        let charging = |ticks: i32, partial: f32| {
            let mut wither = Wither::new(glam::DVec3::ZERO);
            wither.invulnerable_ticks = ticks;
            WitherLook::of(&wither, partial)
        };
        assert!((charging(220, 0.0).scale() - 1.5).abs() < 1e-6);
        assert!((charging(110, 0.0).scale() - 1.75).abs() < 1e-6);
        assert_eq!(charging(0, 0.5).scale(), 2.0);
        assert!((charging(1, 0.5).scale() - (2.0 - 0.5 / 220.0 * 0.5)).abs() < 1e-6);
        assert_eq!(charging(200, 0.0).texture(), INVULNERABLE_TEXTURE);
        assert_eq!(charging(81, 0.0).texture(), INVULNERABLE_TEXTURE);
        assert_eq!(charging(80, 0.0).texture(), INVULNERABLE_TEXTURE);
        assert_eq!(charging(75, 0.0).texture(), TEXTURE);
        assert_eq!(charging(79, 0.0).texture(), TEXTURE);
        assert_eq!(charging(70, 0.0).texture(), INVULNERABLE_TEXTURE);
        assert_eq!(charging(0, 0.0).texture(), TEXTURE);
        assert!(
            WitherLook::of(&Wither::built(glam::DVec3::ZERO, 0.0), 0.0).powered,
            "a charging wither has a third of its health"
        );
    }

    #[test]
    fn its_ribcage_and_tail_breathe_and_its_side_heads_turn_from_the_body() {
        let rest = rest_pose();
        let mut mob = standing();
        let pose = wither_pose(&mob, &WitherLook::default());
        assert!((pose.ribcage_pitch - 0.115 * PI).abs() < 1e-6);
        assert!((pose.tail_pitch - 0.365 * PI).abs() < 1e-6);
        assert!((rest.tail_offset[1] - (6.9 + RIBCAGE_TILT.cos() * 10.0)).abs() < 2e-3);
        mob.age_in_ticks = 10.0 * PI;
        let breathed = wither_pose(&mob, &WitherLook::default());
        assert!((breathed.ribcage_pitch - 0.015 * PI).abs() < 1e-5);
        mob.body_rot = 30.0;
        let look = WitherLook {
            head_x_rots: [20.0, 0.0],
            head_y_rots: [30.0, 75.0],
            ..WitherLook::default()
        };
        let turned = wither_pose(&mob, &look);
        let (_, right, _) = turned.right_head.to_scale_rotation_translation();
        let (_, left, _) = turned.left_head.to_scale_rotation_translation();
        let (_, right_yaw, right_pitch) = right.to_euler(glam::EulerRot::ZYX);
        let (_, left_yaw, _) = left.to_euler(glam::EulerRot::ZYX);
        assert!(right_yaw.abs() < 1e-5, "it looks the body's way");
        assert!((right_pitch - 20.0 * DEG).abs() < 1e-5);
        assert!((left_yaw - 45.0 * DEG).abs() < 1e-5);
    }

    #[test]
    fn its_armour_scrolls_and_wraps_inside_the_texture() {
        let shift = swirl_shift(0.0);
        assert!(
            shift.x.abs() < 1e-6 && shift.y == 0.0,
            "cos 0 * 3 wraps to 0"
        );
        let later = swirl_shift(250.0);
        assert!((later.y - 0.5).abs() < 1e-5);
        assert!((later.x - (mth_cos(5.0) * 3.0) % 1.0).abs() < 1e-6);
        let mut plain = ChunkMesh::default();
        append_swirl(
            &mut plain,
            Mat4::IDENTITY,
            &skin(),
            &rest_pose(),
            Vec2::ZERO,
        );
        assert_eq!(plain.faces, 54, "no shift, no cuts");
        let mut moved = ChunkMesh::default();
        append_swirl(
            &mut moved,
            Mat4::IDENTITY,
            &skin(),
            &rest_pose(),
            Vec2::new(0.3, 0.7),
        );
        assert!(moved.faces > 54, "faces across the edge are cut");
        assert!(
            moved
                .vertices
                .iter()
                .all(|v| (-1e-5..=1.0 + 1e-5).contains(&v.uv[0])
                    && (-1e-5..=1.0 + 1e-5).contains(&v.uv[1]))
        );
        let area = |mesh: &ChunkMesh| {
            mesh.vertices
                .chunks(4)
                .map(|q| {
                    let a = Vec3::from_array(q[1].position) - Vec3::from_array(q[0].position);
                    let b = Vec3::from_array(q[3].position) - Vec3::from_array(q[0].position);
                    a.cross(b).length()
                })
                .sum::<f32>()
        };
        assert!(
            (area(&plain) - area(&moved)).abs() < 1e-3,
            "the cuts cover the same shell"
        );
        let (lo, hi) = span(&plain, 1);
        let mut model = ChunkMesh::default();
        append_model(&mut model, Mat4::IDENTITY, &skin(), &rest_pose());
        let (mlo, mhi) = span(&model, 1);
        assert!(
            lo < mlo && hi > mhi,
            "the armour sits half a pixel outside the body"
        );
    }

    #[test]
    fn a_wither_skull_is_the_head_cube_half_a_block_high_and_blue_when_dangerous() {
        let mut mesh = ChunkMesh::default();
        append(
            &mut mesh,
            skull_root(glam::DVec3::new(0.0, 64.0, 0.0), 0.0, 0.0),
            &skin(),
            &SKULL,
        );
        assert_eq!(mesh.faces, 6);
        let (low, high) = span(&mesh, 1);
        assert!(
            (low - 64.0).abs() < 1e-4 && (high - 64.5).abs() < 1e-4,
            "{low} {high}"
        );
        let (left, right) = span(&mesh, 0);
        assert!((left + 0.25).abs() < 1e-4 && (right - 0.25).abs() < 1e-4);
        let mut turned = ChunkMesh::default();
        append(
            &mut turned,
            skull_root(glam::DVec3::new(0.0, 64.0, 0.0), 90.0, 0.0),
            &skin(),
            &SKULL,
        );
        assert_ne!(mesh.vertices[0].position, turned.vertices[0].position);
        assert_eq!(
            skull_texture(FireballKind::WitherSkull { dangerous: true }),
            Some(INVULNERABLE_TEXTURE)
        );
        assert_eq!(
            skull_texture(FireballKind::WitherSkull { dangerous: false }),
            Some(TEXTURE)
        );
        assert_eq!(skull_texture(FireballKind::Small), None);
    }
}
