use crate::end_crystal::CrystalView;
use crate::ender_dragon::{DragonView, Sample, sample_at};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_core::entity_data::wrap_degrees;
use minecraftoss_player::mth::{cos, sin};

pub const TEXTURE: &str = "minecraft:entity/enderdragon/dragon";
pub const EYES: &str = "minecraft:entity/enderdragon/dragon_eyes";
const TEXTURE_SIZE: [f32; 2] = [256.0, 256.0];
const DEG: f32 = 0.017_453_292;
const SEGMENT: [Cube; 2] = [
    Cube::new([-5.0, -5.0, -5.0], [10.0, 10.0, 10.0], [192.0, 104.0]),
    Cube::new([-1.0, -9.0, -3.0], [2.0, 4.0, 6.0], [48.0, 0.0]),
];
const HEAD: [Cube; 6] = [
    Cube::new([-6.0, -1.0, -24.0], [12.0, 5.0, 16.0], [176.0, 44.0]),
    Cube::new([-8.0, -8.0, -10.0], [16.0, 16.0, 16.0], [112.0, 30.0]),
    Cube::new([-5.0, -12.0, -4.0], [2.0, 4.0, 6.0], [0.0, 0.0]).mirrored(),
    Cube::new([-5.0, -3.0, -22.0], [2.0, 2.0, 4.0], [112.0, 0.0]).mirrored(),
    Cube::new([3.0, -12.0, -4.0], [2.0, 4.0, 6.0], [0.0, 0.0]).mirrored(),
    Cube::new([3.0, -3.0, -22.0], [2.0, 2.0, 4.0], [112.0, 0.0]).mirrored(),
];
const JAW: Cube = Cube::new([-6.0, 0.0, -16.0], [12.0, 4.0, 16.0], [176.0, 65.0]);
const BODY: [Cube; 4] = [
    Cube::new([-12.0, -16.0, -16.0], [24.0, 24.0, 64.0], [0.0, 0.0]),
    Cube::new([-1.0, -22.0, -10.0], [2.0, 6.0, 12.0], [220.0, 53.0]),
    Cube::new([-1.0, -22.0, 10.0], [2.0, 6.0, 12.0], [220.0, 53.0]),
    Cube::new([-1.0, -22.0, 30.0], [2.0, 6.0, 12.0], [220.0, 53.0]),
];
const FRONT_LEG: Cube = Cube::new([-4.0, -4.0, -4.0], [8.0, 24.0, 8.0], [112.0, 104.0]);
const FRONT_LEG_TIP: Cube = Cube::new([-3.0, -1.0, -3.0], [6.0, 24.0, 6.0], [226.0, 138.0]);
const FRONT_FOOT: Cube = Cube::new([-4.0, 0.0, -12.0], [8.0, 4.0, 16.0], [144.0, 104.0]);
const HIND_LEG: Cube = Cube::new([-8.0, -4.0, -8.0], [16.0, 32.0, 16.0], [0.0, 0.0]);
const HIND_LEG_TIP: Cube = Cube::new([-6.0, -2.0, 0.0], [12.0, 32.0, 12.0], [196.0, 0.0]);
const HIND_FOOT: Cube = Cube::new([-9.0, 0.0, -20.0], [18.0, 6.0, 24.0], [112.0, 0.0]);

fn wing(side: f32) -> [Cube; 2] {
    let from = if side > 0.0 { 0.0 } else { -56.0 };
    let bone = Cube::new([from, -4.0, -4.0], [56.0, 8.0, 8.0], [112.0, 88.0]);
    let skin = Cube::new([from, 0.0, 2.0], [56.0, 0.0, 56.0], [-56.0, 88.0]);
    if side > 0.0 {
        [bone.mirrored(), skin.mirrored()]
    } else {
        [bone, skin]
    }
}

fn wing_tip(side: f32) -> [Cube; 2] {
    let from = if side > 0.0 { 0.0 } else { -56.0 };
    let bone = Cube::new([from, -2.0, -2.0], [56.0, 4.0, 4.0], [112.0, 136.0]);
    let skin = Cube::new([from, 0.0, 2.0], [56.0, 0.0, 56.0], [-56.0, 144.0]);
    if side > 0.0 {
        [bone.mirrored(), skin.mirrored()]
    } else {
        [bone, skin]
    }
}

pub struct Pose {
    pub flap: f32,
    pub samples: Vec<Sample>,
    pub sitting: bool,
}

impl Pose {
    pub fn of(view: &DragonView, partial: f32) -> Self {
        let partial = if view.health <= 0.0 { 0.0 } else { partial };
        let flap = view.o_flap_time + partial * (view.flap_time - view.o_flap_time);
        let samples = (0..24)
            .map(|back| sample_at(&view.history, back, partial))
            .collect();
        Self {
            flap,
            samples,
            sitting: view.sitting,
        }
    }

    fn at(&self, back: usize) -> Sample {
        self.samples[back.min(self.samples.len() - 1)]
    }

    fn head_part_offset(&self, index: usize, root: Sample, sample: Sample) -> f32 {
        if self.sitting {
            index as f32
        } else if index == 6 {
            0.0
        } else {
            (sample.y - root.y) as f32
        }
    }
}

pub fn entity_pose(feet: Vec3, pose: &Pose) -> Mat4 {
    let yaw = pose.at(7).y_rot;
    let tilt = (pose.at(5).y - pose.at(10).y) as f32;
    Mat4::from_translation(feet)
        * Mat4::from_rotation_y((-yaw).to_radians())
        * Mat4::from_rotation_x((tilt * 10.0).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0))
}

pub fn model_parts(pose: &Pose) -> Vec<(Mat4, Vec<Cube>)> {
    let f2 = pose.flap * std::f32::consts::TAU;
    let swing = sin(f64::from(f2 - 1.0)) + 1.0;
    let breath = (swing * swing + swing * 2.0) * 0.05;
    let root = part_pose(
        [0.0, (breath - 2.0) * 16.0, -48.0],
        [breath * 2.0 * DEG, 0.0, 0.0],
    );
    let mut out = Vec::new();
    let (mut x, mut y, mut z) = (0.0_f32, 20.0_f32, -12.0_f32);
    let anchor = pose.at(6);
    let turn = wrap_degrees(pose.at(5).y_rot - pose.at(10).y_rot);
    let heading = wrap_degrees(pose.at(5).y_rot + turn / 2.0);
    for i in 0..5 {
        let sample = pose.at(5 - i);
        let wave = cos(f64::from(i as f32 * 0.45 + f2)) * 0.15;
        let y_rot = wrap_degrees(sample.y_rot - anchor.y_rot) * DEG * 1.5;
        let x_rot = wave + pose.head_part_offset(i, anchor, sample) * DEG * 1.5 * 5.0;
        let z_rot = -wrap_degrees(sample.y_rot - heading) * DEG * 1.5;
        out.push((
            root * part_pose([x, y, z], [x_rot, y_rot, z_rot]),
            SEGMENT.to_vec(),
        ));
        x -= sin(f64::from(y_rot)) * cos(f64::from(x_rot)) * 10.0;
        y += sin(f64::from(x_rot)) * 10.0;
        z -= cos(f64::from(y_rot)) * cos(f64::from(x_rot)) * 10.0;
    }
    let newest = pose.at(0);
    let head = root
        * part_pose(
            [x, y, z],
            [
                wrap_degrees(pose.head_part_offset(6, anchor, newest)) * DEG * 1.5 * 5.0,
                wrap_degrees(newest.y_rot - anchor.y_rot) * DEG,
                -wrap_degrees(newest.y_rot - heading) * DEG,
            ],
        );
    out.push((head, HEAD.to_vec()));
    let jaw_rot = (sin(f64::from(f2)) + 1.0) * 0.2;
    out.push((
        head * part_pose([0.0, 4.0, -8.0], [jaw_rot, 0.0, 0.0]),
        vec![JAW],
    ));
    let body = root * part_pose([0.0, 20.0, 8.0], [0.0, 0.0, -turn * 1.5 * DEG]);
    out.push((body, BODY.to_vec()));
    let wing_x = 0.125 - cos(f64::from(f2)) * 0.2;
    let wing_z = -(sin(f64::from(f2)) + 0.125) * 0.8;
    let tip_z = (sin(f64::from(f2 + 2.0)) + 0.5) * 0.75;
    for side in [1.0_f32, -1.0] {
        let wing_pose = body
            * part_pose(
                [12.0 * side, -15.0, -6.0],
                [wing_x, -0.25 * side, wing_z * side],
            );
        out.push((wing_pose, wing(side).to_vec()));
        out.push((
            wing_pose * part_pose([56.0 * side, 0.0, 0.0], [0.0, 0.0, tip_z * side]),
            wing_tip(side).to_vec(),
        ));
        let lift = breath * 0.1;
        let front = body * part_pose([12.0 * side, 0.0, -6.0], [1.3 + lift, 0.0, 0.0]);
        let front_tip = front * part_pose([0.0, 20.0, -1.0], [-0.5 - lift, 0.0, 0.0]);
        let front_foot = front_tip * part_pose([0.0, 23.0, 0.0], [0.75 + lift, 0.0, 0.0]);
        out.push((front, vec![FRONT_LEG]));
        out.push((front_tip, vec![FRONT_LEG_TIP]));
        out.push((front_foot, vec![FRONT_FOOT]));
        let hind = body * part_pose([16.0 * side, -4.0, 34.0], [1.0 + lift, 0.0, 0.0]);
        let hind_tip = hind * part_pose([0.0, 32.0, -4.0], [0.5 + lift, 0.0, 0.0]);
        let hind_foot = hind_tip * part_pose([0.0, 31.0, 4.0], [0.75 + lift, 0.0, 0.0]);
        out.push((hind, vec![HIND_LEG]));
        out.push((hind_tip, vec![HIND_LEG_TIP]));
        out.push((hind_foot, vec![HIND_FOOT]));
    }
    let (mut x, mut y, mut z) = (0.0_f32, 10.0_f32, 60.0_f32);
    let anchor = pose.at(11);
    let mut wave = 0.0_f32;
    for i in 0..12 {
        let sample = pose.at(12 + i);
        wave += sin(f64::from(i as f32 * 0.45 + f2)) * 0.05;
        let y_rot = (wrap_degrees(sample.y_rot - anchor.y_rot) * 1.5 + 180.0) * DEG;
        let x_rot = wave + (sample.y - anchor.y) as f32 * DEG * 1.5 * 5.0;
        let z_rot = wrap_degrees(sample.y_rot - heading) * DEG * 1.5;
        out.push((
            root * part_pose([x, y, z], [x_rot, y_rot, z_rot]),
            SEGMENT.to_vec(),
        ));
        y += sin(f64::from(x_rot)) * 10.0;
        z -= cos(f64::from(y_rot)) * cos(f64::from(x_rot)) * 10.0;
        x -= sin(f64::from(y_rot)) * cos(f64::from(x_rot)) * 10.0;
    }
    out
}

pub fn beam_offset(dragon: &DragonView, crystal: &CrystalView, partial: f32) -> glam::Vec3 {
    let feet = dragon
        .previous_position
        .lerp(dragon.position, f64::from(partial));
    let bob = crate::end_crystal_render::bob(crystal.time as f32 + partial);
    (crystal.position + glam::DVec3::new(0.0, f64::from(bob), 0.0) - feet).as_vec3()
}

pub fn append_beams<'a>(
    mesh: &mut ChunkMesh,
    dragons: impl IntoIterator<Item = &'a DragonView>,
    crystals: &[CrystalView],
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let Some(region) = ResourceId::parse(crate::end_crystal_render::BEAM_TEXTURE)
        .ok()
        .filter(|beam| atlas.contains(beam))
        .map(|beam| atlas.entity_region(&beam))
    else {
        return;
    };
    for dragon in dragons {
        let Some(crystal) = dragon
            .nearest_crystal
            .and_then(|id| crystals.iter().find(|crystal| crystal.id == id))
        else {
            continue;
        };
        let feet = dragon
            .previous_position
            .lerp(dragon.position, f64::from(partial));
        let probe = feet + glam::DVec3::new(0.0, 6.8, 0.0);
        let at = (
            probe.x.floor() as i32,
            probe.y.floor() as i32,
            probe.z.floor() as i32,
        );
        crate::end_crystal_render::append_beam(
            mesh,
            feet.as_vec3(),
            beam_offset(dragon, crystal, partial),
            dragon.tick_count as f32 + partial,
            region,
            (light.get(at) as f32, light.get_block(at) as f32),
        );
    }
}

pub fn append_dragons<'a>(
    mesh: &mut ChunkMesh,
    dragons: impl IntoIterator<Item = &'a DragonView>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let Ok(skin_id) = ResourceId::parse(TEXTURE) else {
        return;
    };
    if !atlas.contains(&skin_id) {
        return;
    }
    let region = atlas.entity_region(&skin_id);
    let eyes = ResourceId::parse(EYES)
        .ok()
        .filter(|eyes| atlas.contains(eyes))
        .map(|eyes| atlas.entity_region(&eyes));
    let mut marks = Vec::new();
    for dragon in dragons {
        let feet = dragon
            .previous_position
            .lerp(dragon.position, f64::from(partial));
        let pose = Pose::of(dragon, partial);
        let probe = feet + glam::DVec3::new(0.0, 6.8, 0.0);
        let at = (
            probe.x.floor() as i32,
            probe.y.floor() as i32,
            probe.z.floor() as i32,
        );
        let overlay = if dragon.hurt_time > 0 {
            -crate::client_mobs::RED_OVERLAY_ALPHA
        } else {
            1.0
        };
        let skin = Skin {
            region,
            texture_size: TEXTURE_SIZE,
            sky: light.get(at) as f32,
            block: light.get_block(at) as f32,
            alpha: 1.0,
            shaded: true,
        };
        let base = entity_pose(feet.as_vec3(), &pose);
        let parts = model_parts(&pose);
        marks.push((mesh.vertices.len(), overlay));
        for (part, cubes) in &parts {
            for cube in cubes {
                append(mesh, base * *part, &skin, cube);
            }
        }
        if let Some(eyes) = eyes {
            marks.push((mesh.vertices.len(), 1.0));
            let glow = Skin {
                region: eyes,
                sky: 15.0,
                block: 15.0,
                shaded: false,
                ..skin
            };
            for (part, cubes) in &parts {
                for cube in cubes {
                    append(mesh, base * *part, &glow, cube);
                }
            }
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ender_dragon::FlightHistory;

    fn level_view(y: f64, y_rot: f32) -> DragonView {
        let mut history = FlightHistory::default();
        for _ in 0..30 {
            history.record(y, y_rot);
        }
        DragonView {
            id: 1,
            uuid: [0; 4],
            position: glam::DVec3::new(0.0, y, 0.0),
            previous_position: glam::DVec3::new(0.0, y, 0.0),
            y_rot,
            flap_time: 0.25,
            o_flap_time: 0.25,
            history: history.recent(crate::ender_dragon::HISTORY_SENT),
            health: 200.0,
            hurt_time: 0,
            death_time: 0,
            sitting: false,
            parts: [glam::DVec3::ZERO; 8],
            tick_count: 0,
            nearest_crystal: None,
        }
    }

    #[test]
    fn a_level_dragon_lays_its_neck_and_tail_out_in_a_straight_line() {
        let pose = Pose::of(&level_view(100.0, 0.0), 0.0);
        let parts = model_parts(&pose);
        assert_eq!(parts.len(), 5 + 2 + 1 + 2 * 8 + 12);
        let cubes: usize = parts.iter().map(|(_, cubes)| cubes.len()).sum();
        assert_eq!(cubes, 5 * 2 + 6 + 1 + 4 + 2 * (2 + 2 + 6) + 12 * 2);
        let origin = |index: usize| parts[index].0.transform_point3(Vec3::ZERO);
        for i in 1..5 {
            let step = origin(i) - origin(i - 1);
            assert!(step.x.abs() < 1e-4, "{step:?}");
            assert!(
                (step.length() * 16.0 - 10.0).abs() < 1e-3,
                "neck segments are 10 pixels apart"
            );
        }
        let tail_start = parts.len() - 12;
        for i in tail_start + 1..parts.len() {
            let step = origin(i) - origin(i - 1);
            assert!((step.length() * 16.0 - 10.0).abs() < 1e-3);
            assert!(step.z > 0.0, "the tail trails behind");
        }
    }

    #[test]
    fn the_wings_beat_with_the_flap_time_and_mirror_each_other() {
        let mut view = level_view(100.0, 0.0);
        let tip = |view: &DragonView| {
            let pose = Pose::of(view, 0.0);
            let parts = model_parts(&pose);
            let base = entity_pose(Vec3::new(0.0, 100.0, 0.0), &pose);
            (
                (base * parts[8].0).transform_point3(Vec3::new(56.0 / 16.0, 0.0, 0.0)),
                (base * parts[16].0).transform_point3(Vec3::new(-56.0 / 16.0, 0.0, 0.0)),
            )
        };
        view.flap_time = 0.25;
        view.o_flap_time = 0.25;
        let (left_up, right_up) = tip(&view);
        view.flap_time = 0.75;
        view.o_flap_time = 0.75;
        let (left_down, _) = tip(&view);
        assert!(
            (left_up.y - left_down.y).abs() > 0.5,
            "{left_up:?} {left_down:?}"
        );
        assert!(
            (left_up.y - right_up.y).abs() < 1e-3,
            "both wings rise together"
        );
        assert!(
            (left_up.x + right_up.x).abs() < 1e-3,
            "mirrored across the body"
        );
    }

    #[test]
    fn the_heal_beam_reaches_from_two_above_the_dragon_to_the_bobbing_glass() {
        let dragon = level_view(100.0, 0.0);
        let crystal = CrystalView {
            id: 4,
            position: glam::DVec3::new(30.0, 80.0, 5.0),
            time: 0,
            beam_target: None,
            show_bottom: true,
        };
        let offset = beam_offset(&dragon, &crystal, 0.0);
        let glass = glam::Vec3::new(30.0, 80.0 + crate::end_crystal_render::bob(0.0), 5.0);
        assert!((glam::Vec3::new(0.0, 100.0, 0.0) + offset - glass).length() < 1e-4);
        let pose = crate::end_crystal_render::beam_pose(glam::Vec3::new(0.0, 100.0, 0.0), offset);
        let far = pose.transform_point3(glam::Vec3::new(0.0, 0.0, offset.length()));
        assert!((far - (glass + glam::Vec3::new(0.0, 2.0, 0.0))).length() < 1e-3);
    }

    #[test]
    fn the_dragon_faces_its_flight_history_heading() {
        let pose = Pose::of(&level_view(100.0, 90.0), 0.0);
        let base = entity_pose(Vec3::new(0.0, 100.0, 0.0), &pose);
        let parts = model_parts(&pose);
        let head = (base * parts[5].0).transform_point3(Vec3::ZERO);
        let tail = (base * parts[parts.len() - 1].0).transform_point3(Vec3::ZERO);
        assert!(
            head.x > 3.0 && tail.x < -3.0,
            "facing +x at yaw 90: {head:?} {tail:?}"
        );
    }
}
