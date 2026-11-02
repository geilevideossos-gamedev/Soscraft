use crate::client_mobs::{ClientMobs, MobPose, mth_cos};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, HeldDisplay, HeldItem};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::illager::{self as il, IllagerArmPose, IllagerKind};
use minecraftoss_entities::world::IllagerEntity;
use std::f32::consts::{FRAC_PI_2, PI};

pub const PILLAGER_TEXTURE: &str = "minecraft:entity/illager/pillager";
pub const VINDICATOR_TEXTURE: &str = "minecraft:entity/illager/vindicator";
pub const EVOKER_TEXTURE: &str = "minecraft:entity/illager/evoker";
pub const TEXTURES: [&str; 3] = [PILLAGER_TEXTURE, VINDICATOR_TEXTURE, EVOKER_TEXTURE];
pub const SHADOW_RADIUS: f32 = 0.5;
pub const MODEL_SCALE: f32 = 0.9375;
pub const TEXTURE_SIZE: [f32; 2] = [64.0, 64.0];
pub const ROBE_DEFORMATION: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Head,
    Nose,
    Body,
    Robe,
    Arms,
    LeftShoulder,
    RightLeg,
    LeftLeg,
    RightArm,
    LeftArm,
}

const CUBES: [(Part, Cube); 11] = [
    (
        Part::Head,
        Cube::new([-4.0, -10.0, -4.0], [8.0, 10.0, 8.0], [0.0, 0.0]),
    ),
    (
        Part::Nose,
        Cube::new([-1.0, -1.0, -6.0], [2.0, 4.0, 2.0], [24.0, 0.0]),
    ),
    (
        Part::Body,
        Cube::new([-4.0, 0.0, -3.0], [8.0, 12.0, 6.0], [16.0, 20.0]),
    ),
    (
        Part::Robe,
        Cube::new([-4.0, 0.0, -3.0], [8.0, 20.0, 6.0], [0.0, 38.0]),
    ),
    (
        Part::Arms,
        Cube::new([-8.0, -2.0, -2.0], [4.0, 8.0, 4.0], [44.0, 22.0]),
    ),
    (
        Part::Arms,
        Cube::new([-4.0, 2.0, -2.0], [8.0, 4.0, 4.0], [40.0, 38.0]),
    ),
    (
        Part::LeftShoulder,
        Cube::new([4.0, -2.0, -2.0], [4.0, 8.0, 4.0], [44.0, 22.0]).mirrored(),
    ),
    (
        Part::RightLeg,
        Cube::new([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 22.0]),
    ),
    (
        Part::LeftLeg,
        Cube::new([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], [0.0, 22.0]).mirrored(),
    ),
    (
        Part::RightArm,
        Cube::new([-3.0, -2.0, -2.0], [4.0, 12.0, 4.0], [40.0, 46.0]),
    ),
    (
        Part::LeftArm,
        Cube::new([-1.0, -2.0, -2.0], [4.0, 12.0, 4.0], [40.0, 46.0]).mirrored(),
    ),
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Limb {
    offset: [f32; 3],
    rotation: [f32; 3],
}

impl Limb {
    const fn at(offset: [f32; 3]) -> Self {
        Self {
            offset,
            rotation: [0.0; 3],
        }
    }

    fn matrix(self) -> Mat4 {
        part_pose(self.offset, self.rotation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IllagerPose {
    pub head: Mat4,
    pub arms: Mat4,
    pub right_arm: Mat4,
    pub left_arm: Mat4,
    pub right_leg: Mat4,
    pub left_leg: Mat4,
    pub crossed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IllagerLook {
    pub arm_pose: IllagerArmPose,
    pub main_hand_empty: bool,
    pub left_handed: bool,
    pub charge_ticks: f32,
}

fn sin(x: f32) -> f32 {
    minecraftoss_player::mth::sin(f64::from(x))
}

fn bob_arms(right: &mut Limb, left: &mut Limb, age: f32) {
    right.rotation[2] += mth_cos(age * 0.09) * 0.05 + 0.05;
    left.rotation[2] -= mth_cos(age * 0.09) * 0.05 + 0.05;
    right.rotation[0] += sin(age * 0.067) * 0.05;
    left.rotation[0] -= sin(age * 0.067) * 0.05;
}

fn zombie_arms(right: &mut Limb, left: &mut Limb, attack: f32, age: f32) {
    let raised = -PI / 1.5;
    let reach = sin(attack * PI);
    let pull = sin((1.0 - (1.0 - attack) * (1.0 - attack)) * PI);
    right.rotation[2] = 0.0;
    left.rotation[2] = 0.0;
    right.rotation[1] = -(0.1 - reach * 0.6);
    left.rotation[1] = 0.1 - reach * 0.6;
    right.rotation[0] = raised + reach * 1.2 - pull * 0.4;
    left.rotation[0] = raised + reach * 1.2 - pull * 0.4;
    bob_arms(right, left, age);
}

fn swing_weapon_down(right: &mut Limb, left: &mut Limb, left_handed: bool, attack: f32, age: f32) {
    let reach = sin(attack * PI);
    let pull = sin((1.0 - (1.0 - attack) * (1.0 - attack)) * PI);
    right.rotation[2] = 0.0;
    left.rotation[2] = 0.0;
    right.rotation[1] = PI / 20.0;
    left.rotation[1] = -PI / 20.0;
    let (weapon, other) = if left_handed {
        (left, right)
    } else {
        (right, left)
    };
    weapon.rotation[0] = -1.884_955_8 + mth_cos(age * 0.09) * 0.15 + reach * 2.2 - pull * 0.4;
    other.rotation[0] = mth_cos(age * 0.19) * 0.5 + reach * 1.2 - pull * 0.4;
    if left_handed {
        bob_arms(other, weapon, age);
    } else {
        bob_arms(weapon, other, age);
    }
}

pub fn illager_pose(mob: &MobPose, look: IllagerLook) -> IllagerPose {
    let deg = 0.017_453_292_f32;
    let (head_yaw, head_pitch) = (mob.head_yaw * deg, mob.head_pitch * deg);
    let mut right_arm = Limb::at([-5.0, 2.0, 0.0]);
    let mut left_arm = Limb::at([5.0, 2.0, 0.0]);
    let mut right_leg = Limb::at([-2.0, 12.0, 0.0]);
    let mut left_leg = Limb::at([2.0, 12.0, 0.0]);
    if mob.riding {
        right_arm.rotation = [-0.628_318_55, 0.0, 0.0];
        left_arm.rotation = [-0.628_318_55, 0.0, 0.0];
        right_leg.rotation = [-1.413_716_7, 0.314_159_27, 0.078_539_82];
        left_leg.rotation = [-1.413_716_7, -0.314_159_27, -0.078_539_82];
    } else {
        let (walk, speed) = (mob.walk_position * 0.6662, mob.walk_speed);
        right_arm.rotation = [mth_cos(walk + PI) * 2.0 * speed * 0.5, 0.0, 0.0];
        left_arm.rotation = [mth_cos(walk) * 2.0 * speed * 0.5, 0.0, 0.0];
        right_leg.rotation = [mth_cos(walk) * 1.4 * speed * 0.5, 0.0, 0.0];
        left_leg.rotation = [mth_cos(walk + PI) * 1.4 * speed * 0.5, 0.0, 0.0];
    }
    let age = mob.age_in_ticks;
    let attack = mob.swing.unwrap_or(0.0);
    match look.arm_pose {
        IllagerArmPose::Attacking if look.main_hand_empty => {
            zombie_arms(&mut right_arm, &mut left_arm, attack, age);
            bob_arms(&mut right_arm, &mut left_arm, age);
        }
        IllagerArmPose::Attacking => {
            swing_weapon_down(&mut right_arm, &mut left_arm, look.left_handed, attack, age);
        }
        IllagerArmPose::Spellcasting => {
            right_arm.offset = [-5.0, 2.0, 0.0];
            left_arm.offset = [5.0, 2.0, 0.0];
            right_arm.rotation = [mth_cos(age * 0.6662) * 0.25, 0.0, 2.356_194_5];
            left_arm.rotation = [mth_cos(age * 0.6662) * 0.25, 0.0, -2.356_194_5];
        }
        IllagerArmPose::BowAndArrow => {
            right_arm.rotation[1] = -0.1 + head_yaw;
            right_arm.rotation[0] = -FRAC_PI_2 + head_pitch;
            left_arm.rotation[0] = -0.942_477_9 + head_pitch;
            left_arm.rotation[1] = head_yaw - 0.4;
            left_arm.rotation[2] = FRAC_PI_2;
        }
        IllagerArmPose::CrossbowHold => {
            right_arm.rotation[1] = -0.3 + head_yaw;
            left_arm.rotation[1] = 0.6 + head_yaw;
            right_arm.rotation[0] = -FRAC_PI_2 + head_pitch + 0.1;
            left_arm.rotation[0] = -1.5 + head_pitch;
        }
        IllagerArmPose::CrossbowCharge => {
            right_arm.rotation[1] = -0.8;
            right_arm.rotation[0] = -0.970_796_35;
            let max = il::CROSSBOW_CHARGE as f32;
            let charged = look.charge_ticks.clamp(0.0, max) / max;
            left_arm.rotation[1] = 0.4 + (0.85 - 0.4) * charged;
            left_arm.rotation[0] =
                right_arm.rotation[0] + (-FRAC_PI_2 - right_arm.rotation[0]) * charged;
        }
        IllagerArmPose::Celebrating => {
            right_arm.offset = [-5.0, 2.0, 0.0];
            left_arm.offset = [5.0, 2.0, 0.0];
            right_arm.rotation = [mth_cos(age * 0.6662) * 0.05, 0.0, 2.670_354];
            left_arm.rotation = [mth_cos(age * 0.6662) * 0.05, 0.0, -2.356_194_5];
        }
        IllagerArmPose::Crossed | IllagerArmPose::Neutral => {}
    }
    IllagerPose {
        head: part_pose([0.0; 3], [head_pitch, head_yaw, 0.0]),
        arms: part_pose([0.0, 3.0, -1.0], [-0.75, 0.0, 0.0]),
        right_arm: right_arm.matrix(),
        left_arm: left_arm.matrix(),
        right_leg: right_leg.matrix(),
        left_leg: left_leg.matrix(),
        crossed: look.arm_pose == IllagerArmPose::Crossed,
    }
}

fn deformed(pose: Mat4, cube: &Cube, grow: f32) -> Mat4 {
    let center = Vec3::from_array(cube.from) + Vec3::from_array(cube.size) / 2.0;
    let size = Vec3::from_array(cube.size);
    let scale = (size + Vec3::splat(2.0 * grow)) / size;
    pose * Mat4::from_translation(center / 16.0)
        * Mat4::from_scale(scale)
        * Mat4::from_translation(-center / 16.0)
}

pub fn model_root(feet: Mat4) -> Mat4 {
    feet * Mat4::from_translation(Vec3::new(0.0, 24.016 / 16.0, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, 24.016 * (1.0 - MODEL_SCALE) / 16.0, 0.0))
        * Mat4::from_scale(Vec3::splat(MODEL_SCALE))
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, pose: &IllagerPose) {
    for (part, cube) in &CUBES {
        let matrix = match part {
            Part::Head => pose.head,
            Part::Nose => pose.head * part_pose([0.0, -2.0, 0.0], [0.0; 3]),
            Part::Body => Mat4::IDENTITY,
            Part::Robe => deformed(Mat4::IDENTITY, cube, ROBE_DEFORMATION),
            Part::Arms | Part::LeftShoulder if !pose.crossed => continue,
            Part::Arms | Part::LeftShoulder => pose.arms,
            Part::RightArm | Part::LeftArm if pose.crossed => continue,
            Part::RightArm => pose.right_arm,
            Part::LeftArm => pose.left_arm,
            Part::RightLeg => pose.right_leg,
            Part::LeftLeg => pose.left_leg,
        };
        append(mesh, root * matrix, skin, cube);
    }
}

pub fn hand_pose(root: Mat4, arm: Mat4, left: bool) -> Mat4 {
    let side = if left { -1.0 } else { 1.0 };
    root * arm
        * Mat4::from_rotation_x(-FRAC_PI_2)
        * Mat4::from_rotation_y(PI)
        * Mat4::from_translation(Vec3::new(side / 16.0, 0.125, -0.625))
}

pub fn head_item_pose(root: Mat4, head: Mat4) -> Mat4 {
    crate::worn_head::item_pose(root * head * Mat4::from_scale(Vec3::splat(1.0 / 16.0)))
}

pub fn worn_on_head(id: &str) -> bool {
    crate::special_models::worn_skull(id).is_none()
        && !crate::armor_render::armor_asset(id).is_some_and(|(_, slot)| slot == 0)
}

pub fn texture(kind: IllagerKind) -> &'static str {
    match kind {
        IllagerKind::Pillager => PILLAGER_TEXTURE,
        IllagerKind::Vindicator => VINDICATOR_TEXTURE,
        IllagerKind::Evoker => EVOKER_TEXTURE,
    }
}

pub fn append_illagers<'a>(
    mesh: &mut ChunkMesh,
    illagers: impl IntoIterator<Item = &'a IllagerEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) -> Vec<HeldItem> {
    let mut held = Vec::new();
    let mut marks = Vec::new();
    for entity in illagers {
        let illager = &entity.illager;
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        let Ok(texture) = ResourceId::parse(texture(illager.kind)) else {
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
        let look = IllagerLook {
            arm_pose: entity.arm_pose,
            main_hand_empty: illager.equipment[il::MAINHAND].is_none(),
            left_handed: illager.left_handed,
            charge_ticks: entity
                .using_item
                .map_or(0.0, |ticks| ticks as f32 + partial),
        };
        let pose = illager_pose(&mob, look);
        let feet =
            Mat4::from_translation(mob.feet.as_vec3()) * Mat4::from_quat(mob.body_rotation(90.0));
        let root = model_root(feet);
        append_model(mesh, root, &skin, &pose);
        if let Some(stack) = illager.equipment[il::HEAD]
            .as_ref()
            .filter(|stack| worn_on_head(&stack.id))
        {
            held.push(HeldItem {
                pose: head_item_pose(root, pose.head),
                light: mob.light_probe.as_vec3(),
                id: crate::item_look::held_key(stack, None),
                display: HeldDisplay::Head,
                first_tint: None,
            });
        }
        if pose.crossed || !illager.shows_held_items() {
            continue;
        }
        let (right, left) = if illager.left_handed {
            (il::OFFHAND, il::MAINHAND)
        } else {
            (il::MAINHAND, il::OFFHAND)
        };
        for (slot, arm, is_left) in [(right, pose.right_arm, false), (left, pose.left_arm, true)] {
            let Some(id) = hand_key(entity, slot) else {
                continue;
            };
            held.push(HeldItem {
                pose: hand_pose(root, arm, is_left),
                light: mob.light_probe.as_vec3(),
                id,
                display: if is_left {
                    HeldDisplay::LeftHand
                } else {
                    HeldDisplay::RightHand
                },
                first_tint: None,
            });
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
    held
}

pub(crate) fn hand_key(entity: &IllagerEntity, slot: usize) -> Option<String> {
    let stack = entity.illager.equipment.get(slot)?.as_ref()?;
    let using = entity.using_item.filter(|_| slot == entity.illager.crossbow_hand());
    Some(crate::item_look::held_key(stack, using))
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

    fn look(arm_pose: IllagerArmPose, charge_ticks: f32) -> IllagerLook {
        IllagerLook {
            arm_pose,
            main_hand_empty: false,
            left_handed: false,
            charge_ticks,
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

    fn arm_angles(matrix: Mat4) -> (f32, f32, f32) {
        let (_, rotation, _) = matrix.to_scale_rotation_translation();
        rotation.to_euler(glam::EulerRot::ZYX)
    }

    #[test]
    fn a_pillager_stands_two_blocks_tall_at_fifteen_sixteenths_with_free_arms() {
        let pose = illager_pose(&standing(), look(IllagerArmPose::Neutral, 0.0));
        assert!(!pose.crossed);
        let mut mesh = ChunkMesh::default();
        append_model(&mut mesh, model_root(Mat4::IDENTITY), &skin(), &pose);
        assert_eq!(
            mesh.faces,
            8 * 6,
            "head, nose, body, robe, two legs, two arms"
        );
        let (low, high) = span(&mesh, 1);
        assert!(low.abs() < 0.04, "{low}");
        let expected = (24.016 - (24.016 + 10.0) * MODEL_SCALE) / 16.0;
        assert!((high - (24.016 / 16.0 - expected)).abs() < 0.05, "{high}");
    }

    #[test]
    fn crossed_arms_replace_the_two_free_arms() {
        let pose = illager_pose(&standing(), look(IllagerArmPose::Crossed, 0.0));
        assert!(pose.crossed);
        let mut mesh = ChunkMesh::default();
        append_model(&mut mesh, model_root(Mat4::IDENTITY), &skin(), &pose);
        assert_eq!(mesh.faces, 9 * 6, "the crossed arms are three cubes");
    }

    #[test]
    fn a_crossbow_is_held_up_to_the_eye_and_the_string_pulled_while_charging() {
        let hold = illager_pose(&standing(), look(IllagerArmPose::CrossbowHold, 0.0));
        let (_, y, x) = arm_angles(hold.right_arm);
        assert!((y + 0.3).abs() < 1e-4 && (x - (-FRAC_PI_2 + 0.1)).abs() < 1e-4);
        let (_, y, x) = arm_angles(hold.left_arm);
        assert!((y - 0.6).abs() < 1e-4 && (x + 1.5).abs() < 1e-4);
        let start = illager_pose(&standing(), look(IllagerArmPose::CrossbowCharge, 0.0));
        let full = illager_pose(&standing(), look(IllagerArmPose::CrossbowCharge, 30.0));
        let (_, y0, x0) = arm_angles(start.left_arm);
        let (_, y1, x1) = arm_angles(full.left_arm);
        assert!((y0 - 0.4).abs() < 1e-4 && (y1 - 0.85).abs() < 1e-4);
        assert!((x0 + 0.970_796_35).abs() < 1e-4 && (x1 + FRAC_PI_2).abs() < 1e-3);
        assert_eq!(start.right_arm, full.right_arm);
    }

    #[test]
    fn the_crossbow_sits_in_the_right_hand_ten_pixels_down_the_arm() {
        let pose = illager_pose(&standing(), look(IllagerArmPose::Neutral, 0.0));
        let root = model_root(Mat4::IDENTITY);
        let hand = hand_pose(root, pose.right_arm, false).transform_point3(Vec3::ZERO);
        let shoulder = (root * pose.right_arm).transform_point3(Vec3::ZERO);
        assert!(
            (shoulder.y - hand.y - 0.625 * MODEL_SCALE).abs() < 1e-4,
            "{shoulder} {hand}"
        );
        assert!(hand.x > 0.0, "the right hand is on the mob's right: {hand}");
    }

    #[test]
    fn a_captain_wears_its_banner_above_its_head_but_no_helmet_or_skull() {
        let pose = illager_pose(&standing(), look(IllagerArmPose::Crossed, 0.0));
        let root = model_root(Mat4::IDENTITY);
        let origin = head_item_pose(root, pose.head).transform_point3(Vec3::ZERO);
        let neck = (24.016 - 24.016 * (1.0 - MODEL_SCALE)) / 16.0;
        assert!(
            (origin.y - (neck + 0.25 * MODEL_SCALE)).abs() < 1e-4,
            "{origin}"
        );
        assert!(origin.x.abs() < 1e-5 && origin.z.abs() < 1e-5, "{origin}");
        assert!(worn_on_head("minecraft:white_banner"));
        assert!(!worn_on_head("minecraft:iron_helmet"));
        assert!(!worn_on_head("minecraft:zombie_head"));
    }

    #[test]
    fn a_riding_illager_bends_its_legs_forward() {
        let mut mob = standing();
        mob.riding = true;
        let pose = illager_pose(&mob, look(IllagerArmPose::Neutral, 0.0));
        let (z, y, x) = arm_angles(pose.right_leg);
        assert!((x + 1.413_716_7).abs() < 1e-4 && (y - 0.314_159_27).abs() < 1e-4);
        assert!((z - 0.078_539_82).abs() < 1e-4);
    }

    #[test]
    fn a_pillager_s_crossbow_pulls_in_its_using_hand_and_shows_its_arrow_when_loaded() {
        use minecraftoss_entities::illager::Illager;
        use minecraftoss_entities::world::EntityWorld;
        use minecraftoss_player::inventory::ItemStack;
        let atlas = crate::mesh::make_atlas(&crate::pack::PackStack::open(Vec::new()).unwrap(), Vec::new()).unwrap();
        let light = SkyLight::from_levels((-2, 62, -2), (5, 5, 5), vec![15; 125], vec![0; 125]);
        let drawn = |world: &EntityWorld| {
            let mut poses = ClientMobs::default();
            poses.spawn_missing(&crate::client_mobs::server_mobs(world));
            let mut mesh = ChunkMesh::default();
            let held = append_illagers(&mut mesh, world.illagers().iter(), &poses, &atlas, &light, 0.0);
            held.into_iter().map(|item| (item.display, item.id)).collect::<Vec<_>>()
        };
        let mut world = EntityWorld::default();
        let id = world.spawn_illager(Illager::pillager(glam::DVec3::new(0.5, 64.0, 0.5)), true);
        let crossbow = "minecraft:crossbow".to_owned();
        assert_eq!(drawn(&world), [(HeldDisplay::RightHand, crossbow.clone())]);
        let charge = |world: &mut EntityWorld, ticks: Option<i32>| {
            let e = world.illager_mut(id).unwrap();
            e.illager.charging_crossbow = ticks.is_some();
            e.arm_pose = if ticks.is_some() { IllagerArmPose::CrossbowCharge } else { IllagerArmPose::CrossbowHold };
            e.using_item = ticks;
        };
        for ticks in [0, 15, 25] {
            charge(&mut world, Some(ticks));
            assert_eq!(drawn(&world), [(HeldDisplay::RightHand, format!("{crossbow}|use={ticks}"))]);
        }
        charge(&mut world, None);
        world.illager_mut(id).unwrap().illager.set_crossbow_charged(true);
        assert_eq!(drawn(&world), [(HeldDisplay::RightHand, format!("{crossbow}|charge=arrow"))]);
        world.illager_mut(id).unwrap().illager.set_crossbow_charged(false);
        {
            let e = world.illager_mut(id).unwrap();
            e.illager.left_handed = true;
            e.illager.equipment[il::OFFHAND] = Some(ItemStack::new("minecraft:firework_rocket", 1));
            e.illager.equipment[il::HEAD] = Some(ItemStack {
                components: Some(serde_json::json!({"minecraft:banner_patterns": [{"pattern": "minecraft:rhombus", "color": "cyan"}]})),
                ..ItemStack::new("minecraft:white_banner", 1)
            });
        }
        charge(&mut world, Some(15));
        assert_eq!(
            drawn(&world),
            [
                (HeldDisplay::Head, "minecraft:white_banner|layer=cyan,minecraft:rhombus".to_owned()),
                (HeldDisplay::RightHand, "minecraft:firework_rocket".to_owned()),
                (HeldDisplay::LeftHand, format!("{crossbow}|use=15")),
            ],
            "a left-handed pillager pulls the crossbow in its left hand only"
        );
        {
            let e = world.illager_mut(id).unwrap();
            e.illager.equipment.swap(il::MAINHAND, il::OFFHAND);
        }
        assert_eq!(drawn(&world)[1..], [(HeldDisplay::RightHand, format!("{crossbow}|use=15")), (HeldDisplay::LeftHand, "minecraft:firework_rocket".to_owned())], "an offhand crossbow is the using hand");
    }
}
