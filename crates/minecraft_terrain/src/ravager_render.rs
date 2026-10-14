use crate::client_mobs::{ClientMobs, MobPose, mth_cos};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::ravager::{ROAR_DURATION, STUN_DURATION};
use minecraftoss_entities::world::RavagerEntity;
use std::f32::consts::{FRAC_PI_2, PI};

pub const TEXTURE: &str = "minecraft:entity/illager/ravager";
pub const SHADOW_RADIUS: f32 = 1.1;
pub const TEXTURE_SIZE: [f32; 2] = [128.0, 128.0];
const HORN_TILT: f32 = 1.099_557_4;
const STUNNED_NECK: f32 = 0.219_911_49;
const ATTACK_TICKS: f32 = 10.0;

const NECK: Cube = Cube::new([-5.0, -1.0, -18.0], [10.0, 10.0, 18.0], [68.0, 73.0]);
const HEAD: Cube = Cube::new([-8.0, -20.0, -14.0], [16.0, 20.0, 16.0], [0.0, 0.0]);
const NOSE: Cube = Cube::new([-2.0, -6.0, -18.0], [4.0, 8.0, 4.0], [0.0, 0.0]);
const RIGHT_HORN: Cube = Cube::new([0.0, -14.0, -2.0], [2.0, 14.0, 4.0], [74.0, 55.0]);
const LEFT_HORN: Cube = Cube::new([0.0, -14.0, -2.0], [2.0, 14.0, 4.0], [74.0, 55.0]).mirrored();
const MOUTH: Cube = Cube::new([-8.0, 0.0, -16.0], [16.0, 3.0, 16.0], [0.0, 36.0]);
const BODY: Cube = Cube::new([-7.0, -10.0, -7.0], [14.0, 16.0, 20.0], [0.0, 55.0]);
const BELLY: Cube = Cube::new([-6.0, 6.0, -7.0], [12.0, 13.0, 18.0], [0.0, 91.0]);
const HIND_LEG: Cube = Cube::new([-4.0, 0.0, -4.0], [8.0, 37.0, 8.0], [96.0, 0.0]);
const FRONT_LEG: Cube = Cube::new([-4.0, 0.0, -4.0], [8.0, 37.0, 8.0], [64.0, 0.0]);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RavagerLook {
    pub attack_ticks: f32,
    pub stunned_ticks: f32,
    pub roar: f32,
}

impl RavagerLook {
    pub fn of(attack: i32, stunned: i32, roar: i32, partial: f32) -> Self {
        let left = |ticks: i32| {
            if ticks > 0 {
                ticks as f32 - partial
            } else {
                0.0
            }
        };
        let roar_ticks = if roar > 0 {
            (ROAR_DURATION - roar) as f32 + partial
        } else {
            0.0
        };
        Self {
            attack_ticks: left(attack),
            stunned_ticks: left(stunned),
            roar: roar_ticks / ROAR_DURATION as f32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RavagerPose {
    pub neck: Mat4,
    pub head: Mat4,
    pub right_horn: Mat4,
    pub left_horn: Mat4,
    pub mouth: Mat4,
    pub body: Mat4,
    pub right_hind_leg: Mat4,
    pub left_hind_leg: Mat4,
    pub right_front_leg: Mat4,
    pub left_front_leg: Mat4,
    pub neck_offset: [f32; 3],
    pub neck_pitch: f32,
    pub mouth_pitch: f32,
}

fn mth_sin(x: f32) -> f32 {
    minecraftoss_player::mth::sin(f64::from(x))
}

fn triangle_wave(index: f32, period: f32) -> f32 {
    ((index % period - period * 0.5).abs() - period * 0.25) / (period * 0.25)
}

pub fn ravager_pose(mob: &MobPose, look: RavagerLook) -> RavagerPose {
    let deg = 0.017_453_292_f32;
    let mut neck_offset = [0.0, -7.0, 5.5];
    let mut neck_pitch = 0.0;
    let mouth_pitch;
    if look.attack_ticks > 0.0 {
        let wave = (1.0 + triangle_wave(look.attack_ticks, ATTACK_TICKS)) * 0.5;
        let lunge = wave * wave * wave * 12.0;
        neck_offset[2] = -6.5 + lunge;
        neck_offset[1] = -7.0 - lunge * mth_sin(0.0);
        mouth_pitch = if look.attack_ticks > 5.0 {
            mth_sin((-4.0 + look.attack_ticks) / 4.0) * PI * 0.4
        } else {
            0.157_079_64 * mth_sin(PI * look.attack_ticks / 10.0)
        };
    } else {
        let stunned = look.stunned_ticks > 0.0;
        neck_pitch = if stunned { STUNNED_NECK } else { 0.0 };
        let mut mouth = PI * if stunned { 0.05 } else { 0.01 };
        if stunned {
            let spin = f64::from(look.stunned_ticks) / f64::from(STUN_DURATION);
            neck_offset[0] = (spin * 10.0).sin() as f32 * 3.0;
        } else if look.roar > 0.0 {
            mouth = FRAC_PI_2 * mth_sin(look.roar * PI * 0.25);
        }
        mouth_pitch = mouth;
    }
    let swing = 0.4 * mob.walk_speed;
    let walk = mob.walk_position * 0.6662;
    let neck = part_pose(neck_offset, [neck_pitch, 0.0, 0.0]);
    let head = neck
        * part_pose(
            [0.0, 16.0, -17.0],
            [mob.head_pitch * deg, mob.head_yaw * deg, 0.0],
        );
    RavagerPose {
        neck,
        head,
        right_horn: head * part_pose([-10.0, -14.0, -8.0], [HORN_TILT, 0.0, 0.0]),
        left_horn: head * part_pose([8.0, -14.0, -8.0], [HORN_TILT, 0.0, 0.0]),
        mouth: head * part_pose([0.0, -2.0, 2.0], [mouth_pitch, 0.0, 0.0]),
        body: part_pose([0.0, 1.0, 2.0], [FRAC_PI_2, 0.0, 0.0]),
        right_hind_leg: part_pose([-8.0, -13.0, 18.0], [mth_cos(walk) * swing, 0.0, 0.0]),
        left_hind_leg: part_pose([8.0, -13.0, 18.0], [mth_cos(walk + PI) * swing, 0.0, 0.0]),
        right_front_leg: part_pose([-8.0, -13.0, -5.0], [mth_cos(walk + PI) * swing, 0.0, 0.0]),
        left_front_leg: part_pose([8.0, -13.0, -5.0], [mth_cos(walk) * swing, 0.0, 0.0]),
        neck_offset,
        neck_pitch,
        mouth_pitch,
    }
}

pub fn model_root(feet: Mat4) -> Mat4 {
    feet * Mat4::from_translation(Vec3::new(0.0, 1.501, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, pose: &RavagerPose) {
    append(mesh, root * pose.neck, skin, &NECK);
    append(mesh, root * pose.head, skin, &HEAD);
    append(mesh, root * pose.head, skin, &NOSE);
    append(mesh, root * pose.right_horn, skin, &RIGHT_HORN);
    append(mesh, root * pose.left_horn, skin, &LEFT_HORN);
    append(mesh, root * pose.mouth, skin, &MOUTH);
    append(mesh, root * pose.body, skin, &BODY);
    append(mesh, root * pose.body, skin, &BELLY);
    append(mesh, root * pose.right_hind_leg, skin, &HIND_LEG);
    append(mesh, root * pose.left_hind_leg, skin, &HIND_LEG.mirrored());
    append(mesh, root * pose.right_front_leg, skin, &FRONT_LEG);
    append(
        mesh,
        root * pose.left_front_leg,
        skin,
        &FRONT_LEG.mirrored(),
    );
}

pub fn append_ravagers<'a>(
    mesh: &mut ChunkMesh,
    ravagers: impl IntoIterator<Item = &'a RavagerEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let Ok(texture) = ResourceId::parse(TEXTURE) else {
        return;
    };
    let mut marks = Vec::new();
    for entity in ravagers {
        let ravager = &entity.ravager;
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let position = mob.light_block();
        let skin = Skin {
            region: atlas.entity_region(&texture),
            texture_size: TEXTURE_SIZE,
            sky: light.get(position) as f32,
            block: light.get_block(position) as f32,
            alpha: 1.0,
            shaded: true,
        };
        let look = RavagerLook::of(
            ravager.attack_tick,
            ravager.stunned_tick,
            ravager.roar_tick,
            partial,
        );
        let pose = ravager_pose(&mob, look);
        let feet =
            Mat4::from_translation(mob.feet.as_vec3()) * Mat4::from_quat(mob.body_rotation(90.0));
        append_model(mesh, model_root(feet), &skin, &pose);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
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
            block: 0.0,
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
    fn a_ravager_stands_on_its_legs_about_two_blocks_tall() {
        let mut mesh = ChunkMesh::default();
        let pose = ravager_pose(&standing(), RavagerLook::default());
        append_model(&mut mesh, model_root(Mat4::IDENTITY), &skin(), &pose);
        assert_eq!(mesh.faces, 72, "twelve cubes");
        let (low, high) = span(&mesh, 1);
        assert!(low.abs() < 0.01, "its feet are on the ground: {low}");
        assert!(high > 2.0 && high < 2.7, "{high}");
        assert!((pose.mouth_pitch - PI * 0.01).abs() < 1e-6);
    }

    #[test]
    fn its_neck_lunges_and_its_jaw_snaps_while_it_bites() {
        let rest = ravager_pose(&standing(), RavagerLook::default());
        let mid = ravager_pose(&standing(), RavagerLook::of(5, 0, 0, 0.0));
        assert!(
            (mid.neck_offset[2] - -6.5).abs() < 1e-5,
            "{:?}",
            mid.neck_offset
        );
        assert!(mid.neck_offset[2] < rest.neck_offset[2]);
        let start = ravager_pose(&standing(), RavagerLook::of(10, 0, 0, 0.0));
        assert!((start.neck_offset[2] - 5.5).abs() < 1e-5);
        assert!(
            start.mouth_pitch > 1.0,
            "the jaw is wide open: {}",
            start.mouth_pitch
        );
        let late = ravager_pose(&standing(), RavagerLook::of(3, 0, 0, 0.0));
        assert!(late.mouth_pitch > 0.0 && late.mouth_pitch < 0.16);
    }

    #[test]
    fn a_stunned_ravager_hangs_its_head_and_sways_then_roars_wide() {
        let stunned = ravager_pose(&standing(), RavagerLook::of(0, 30, 0, 0.0));
        assert_eq!(stunned.neck_pitch, STUNNED_NECK);
        assert!((stunned.neck_offset[0] - (7.5_f64.sin() as f32 * 3.0)).abs() < 1e-5);
        assert!((stunned.mouth_pitch - PI * 0.05).abs() < 1e-6);
        let look = RavagerLook::of(0, 0, 10, 0.0);
        assert!((look.roar - 0.5).abs() < 1e-6);
        let roaring = ravager_pose(&standing(), look);
        assert_eq!(roaring.neck_pitch, 0.0);
        assert!((roaring.mouth_pitch - FRAC_PI_2 * mth_sin(0.5 * PI * 0.25)).abs() < 1e-6);
        assert_eq!(RavagerLook::of(0, 0, 0, 0.5), RavagerLook::default());
    }

    #[test]
    fn its_legs_swing_in_diagonal_pairs() {
        let mut walking = standing();
        walking.walk_position = 1.0;
        walking.walk_speed = 1.0;
        let pose = ravager_pose(&walking, RavagerLook::default());
        let pitch = |leg: Mat4| {
            leg.to_scale_rotation_translation()
                .1
                .to_euler(glam::EulerRot::ZYX)
                .2
        };
        assert!((pitch(pose.right_hind_leg) - pitch(pose.left_front_leg)).abs() < 1e-5);
        assert!((pitch(pose.left_hind_leg) - pitch(pose.right_front_leg)).abs() < 1e-5);
        assert!(pitch(pose.right_hind_leg) > 0.2);
        assert!((pitch(pose.right_hind_leg) + pitch(pose.left_hind_leg)).abs() < 1e-3);
    }
}
