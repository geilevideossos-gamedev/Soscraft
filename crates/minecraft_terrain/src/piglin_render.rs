use crate::{
    client_mobs::MobPose,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh, HeldItem},
    pack::ResourceId,
};
use glam::{EulerRot, Quat, Vec3};
use minecraftoss_entities::piglin::{ArmPose, PiglinKind};
use minecraftoss_entities::world::{PiglinEntity, ZombieEntity};

type Part = ([f32; 3], [f32; 3], [f32; 2], Option<[f32; 3]>);

type Ear = (Part, [f32; 3], Option<([f32; 3], f32)>);

const fn part(from: [f32; 3], size: [f32; 3], uv: [f32; 2]) -> Part {
    (
        from,
        [from[0] + size[0], from[1] + size[1], from[2] + size[2]],
        uv,
        None,
    )
}

const fn inflated(from: [f32; 3], size: [f32; 3], uv: [f32; 2], grow: f32) -> Part {
    (
        [from[0] - grow, from[1] - grow, from[2] - grow],
        [
            from[0] + size[0] + grow,
            from[1] + size[1] + grow,
            from[2] + size[2] + grow,
        ],
        uv,
        Some(size),
    )
}

struct Model {
    head_pivot: [f32; 3],
    head: &'static [Part],
    ears: [Ear; 2],
    ear_angle: f32,
    body_pivot: [f32; 3],
    body: &'static [Part],
    arm_pivots: [[f32; 3]; 2],
    arms: [&'static [Part]; 2],
    leg_pivots: [[f32; 3]; 2],
    legs: [&'static [Part]; 2],
}

const ADULT_HEAD: [Part; 4] = [
    part([-5., -8., -4.], [10., 8., 8.], [0., 0.]),
    part([-2., -4., -5.], [4., 4., 1.], [31., 1.]),
    part([2., -2., -5.], [1., 2., 1.], [2., 4.]),
    part([-3., -2., -5.], [1., 2., 1.], [2., 0.]),
];
const ADULT_BODY: [Part; 2] = [
    part([-4., 0., -2.], [8., 12., 4.], [16., 16.]),
    inflated([-4., 0., -2.], [8., 12., 4.], [16., 32.], 0.25),
];
const ADULT_RIGHT_ARM: [Part; 2] = [
    part([-3., -2., -2.], [4., 12., 4.], [40., 16.]),
    inflated([-3., -2., -2.], [4., 12., 4.], [40., 32.], 0.25),
];
const ADULT_LEFT_ARM: [Part; 2] = [
    part([-1., -2., -2.], [4., 12., 4.], [32., 48.]),
    inflated([-1., -2., -2.], [4., 12., 4.], [48., 48.], 0.25),
];
const ADULT_RIGHT_LEG: [Part; 2] = [
    part([-2., 0., -2.], [4., 12., 4.], [0., 16.]),
    inflated([-2., 0., -2.], [4., 12., 4.], [0., 32.], 0.25),
];
const ADULT_LEFT_LEG: [Part; 2] = [
    part([-2., 0., -2.], [4., 12., 4.], [16., 48.]),
    inflated([-2., 0., -2.], [4., 12., 4.], [0., 48.], 0.25),
];

const ADULT: Model = Model {
    head_pivot: [0., 0., 0.],
    head: &ADULT_HEAD,
    ears: [
        (
            part([-1., 0., -2.], [1., 5., 4.], [39., 6.]),
            [-4.5, -6., 0.],
            None,
        ),
        (
            part([0., 0., -2.], [1., 5., 4.], [51., 6.]),
            [4.5, -6., 0.],
            None,
        ),
    ],
    ear_angle: 30.0,
    body_pivot: [0., 0., 0.],
    body: &ADULT_BODY,
    arm_pivots: [[-5., 2., 0.], [5., 2., 0.]],
    arms: [&ADULT_RIGHT_ARM, &ADULT_LEFT_ARM],
    leg_pivots: [[-1.9, 12., 0.], [1.9, 12., 0.]],
    legs: [&ADULT_RIGHT_LEG, &ADULT_LEFT_LEG],
};

const BABY_HEAD: [Part; 2] = [
    part([-1.5, -3., -4.5], [3., 3., 1.], [21., 30.]),
    part([-4.5, -6., -3.5], [9., 6., 7.], [0., 0.]),
];
const BABY_BODY: [Part; 1] = [part([-3., -3., -1.], [6., 5., 3.], [0., 13.])];
const BABY_RIGHT_ARM: [Part; 1] = [part([-1., 0., -1.5], [2., 5., 3.], [10., 30.])];
const BABY_LEFT_ARM: [Part; 1] = [part([-1., 0., -1.5], [2., 5., 3.], [28., 13.])];
const BABY_RIGHT_LEG: [Part; 1] = [part([-1.5, 0., -1.5], [3., 4., 3.], [22., 23.])];
const BABY_LEFT_LEG: [Part; 1] = [part([-1.5, 0., -1.5], [3., 4., 3.], [10., 23.])];

const BABY: Model = Model {
    head_pivot: [0., 15., 0.],
    head: &BABY_HEAD,
    ears: [
        (
            part([-0.5, -3., -2.], [1., 6., 4.], [18., 13.]),
            [-4.2, -4., 0.],
            Some(([-1., 1.75, 0.], 0.6109)),
        ),
        (
            part([-0.5, -3., -2.], [1., 6., 4.], [0., 21.]),
            [4.2, -4., 0.],
            Some(([1., 1.75, 0.], -0.6109)),
        ),
    ],
    ear_angle: 5.0,
    body_pivot: [0., 18., -0.5],
    body: &BABY_BODY,
    arm_pivots: [[-4., 15., 0.], [4., 15., 0.]],
    arms: [&BABY_RIGHT_ARM, &BABY_LEFT_ARM],
    leg_pivots: [[-1.5, 20., 0.], [1.5, 20., 0.]],
    legs: [&BABY_RIGHT_LEG, &BABY_LEFT_LEG],
};

pub struct PosedCube {
    pub part: Part,
    pub pivot: [f32; 3],
    pub rotation: Quat,
}

pub struct Posed {
    pub cubes: Vec<PosedCube>,
    pub head_pivot: [f32; 3],
    pub head: Quat,
    pub body_pivot: [f32; 3],
    pub body: Quat,
    pub arms: [([f32; 3], Quat); 2],
    pub legs: [([f32; 3], Quat); 2],
}

pub fn ear_rolls(baby: bool, age: f32, walk_position: f32, walk_speed: f32) -> [f32; 2] {
    let angle = if baby {
        BABY.ear_angle
    } else {
        ADULT.ear_angle
    } * 0.017_453_292;
    let f = age * 0.1 + walk_position * 0.5;
    let g = 0.08 + walk_speed * 0.4;
    let right = angle + minecraftoss_player::mth::cos(f64::from(f)) * g;
    let left = -angle - minecraftoss_player::mth::cos(f64::from(f * 1.2)) * g;
    [right, left]
}

fn head_of(pose: &MobPose) -> Quat {
    Quat::from_euler(
        EulerRot::ZYX,
        0.0,
        pose.head_yaw.to_radians(),
        pose.head_pitch.to_radians(),
    )
}

fn rest_of(baby: bool) -> [(f32, f32); 2] {
    let model = if baby { &BABY } else { &ADULT };
    [
        (model.arm_pivots[0][0], model.arm_pivots[0][2]),
        (model.arm_pivots[1][0], model.arm_pivots[1][2]),
    ]
}

pub fn pose(baby: bool, aggressive: bool, pose: &MobPose) -> Posed {
    let limbs = crate::zombie_render::limb_rotations(
        pose.walk_position,
        pose.walk_speed,
        pose.age_in_ticks,
        aggressive,
        pose.swing,
        if baby { 0.5 } else { 1.0 },
        rest_of(baby),
    )
    .seated(pose.riding);
    assemble(baby, head_of(pose), limbs, pose, None)
}

fn assemble(
    baby: bool,
    head: Quat,
    limbs: crate::zombie_render::ZombieLimbs,
    pose: &MobPose,
    dance: Option<Dance>,
) -> Posed {
    let model = if baby { &BABY } else { &ADULT };
    let body = Quat::from_rotation_y(limbs.body_yaw);
    let lift = dance.unwrap_or(STILL);
    let arms = [0, 1].map(|arm| {
        let (x, z) = limbs.arm_pivots[arm];
        let y = model.arm_pivots[arm][1] + lift.arms_y[arm];
        ([x, y, z], limbs.limbs[arm])
    });
    let [head_x, head_y, head_z] = model.head_pivot;
    let head_pivot = [head_x + lift.head[0], head_y + lift.head[1], head_z];
    let [body_x, body_y, body_z] = model.body_pivot;
    let body_pivot = [body_x, body_y + lift.body_y, body_z];
    let legs = [0, 1].map(|leg| (model.leg_pivots[leg], limbs.limbs[2 + leg]));
    let mut cubes = Vec::new();
    for &part in model.head {
        cubes.push(PosedCube {
            part,
            pivot: head_pivot,
            rotation: head,
        });
    }
    let rolls = match dance {
        Some(dance) => dance.ears,
        None => ear_rolls(baby, pose.age_in_ticks, pose.walk_position, pose.walk_speed),
    };
    for (side, &(part, offset, child)) in model.ears.iter().enumerate() {
        let ear = head * Quat::from_rotation_z(rolls[side]);
        let (inner, rotation) = match child {
            Some((child_offset, roll)) => (
                ear * Vec3::from_array(child_offset),
                ear * Quat::from_rotation_z(roll),
            ),
            None => (Vec3::ZERO, ear),
        };
        let pivot = Vec3::from_array(head_pivot) + head * Vec3::from_array(offset) + inner;
        cubes.push(PosedCube {
            part,
            pivot: pivot.to_array(),
            rotation,
        });
    }
    for &part in model.body {
        cubes.push(PosedCube {
            part,
            pivot: body_pivot,
            rotation: body,
        });
    }
    for (arm, parts) in model.arms.iter().enumerate() {
        for &part in parts.iter() {
            cubes.push(PosedCube {
                part,
                pivot: arms[arm].0,
                rotation: arms[arm].1,
            });
        }
    }
    for (leg, parts) in model.legs.iter().enumerate() {
        for &part in parts.iter() {
            cubes.push(PosedCube {
                part,
                pivot: legs[leg].0,
                rotation: legs[leg].1,
            });
        }
    }
    Posed {
        cubes,
        head_pivot,
        head,
        body_pivot,
        body,
        arms,
        legs,
    }
}

pub fn append_zombified_piglin(
    mesh: &mut ChunkMesh,
    marks: &mut Vec<(usize, f32)>,
    entity: &ZombieEntity,
    mob: &MobPose,
    atlas: &Atlas,
    light: &SkyLight,
) -> Option<HeldItem> {
    let zombie = &entity.zombie;
    let id = ResourceId::parse(if zombie.baby {
        "minecraft:entity/piglin/zombified_piglin_baby"
    } else {
        "minecraft:entity/piglin/zombified_piglin"
    })
    .ok()?;
    let region = atlas.entity_region(&id);
    let sample = mob.light_block();
    let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
    let rotation = mob.body_rotation(90.0);
    let posed = pose(zombie.baby, entity.aggressive, mob);
    for cube in &posed.cubes {
        let (from, to, uv, dims) = cube.part;
        crate::cow_render::cube_scaled(
            mesh,
            mob.feet,
            rotation,
            Vec3::ONE,
            region,
            sky,
            block,
            from,
            to,
            uv,
            cube.pivot,
            cube.rotation,
            [1.0; 3],
            [64., 64.],
            dims,
            false,
        );
    }
    if !zombie.baby && zombie.armor.iter().any(Option::is_some) {
        marks.push((mesh.vertices.len(), 1.0));
        let armor_parts = crate::armor_render::HumanoidParts {
            head: (Vec3::ZERO, posed.head),
            body: (Vec3::ZERO, posed.body),
            arms: posed
                .arms
                .map(|(pivot, rotation)| (Vec3::from_array(pivot), rotation)),
            legs: posed
                .legs
                .map(|(pivot, rotation)| (Vec3::from_array(pivot), rotation)),
        };
        crate::armor_render::append_humanoid_armor(
            mesh,
            atlas,
            mob.feet,
            rotation,
            1.0,
            &armor_parts,
            &zombie.armor,
            sky,
            block,
        );
    }
    let item = zombie.main_hand.clone()?;
    let (pivot, arm) = posed.arms[0];
    let pose = crate::cow_render::right_hand_pose(
        mob.feet,
        rotation,
        Vec3::from_array(pivot),
        arm,
        zombie.baby,
    );
    Some(HeldItem {
        pose,
        light: mob.light_probe.as_vec3(),
        id: item,
        display: crate::mesh::HeldDisplay::RightHand,
        first_tint: None,
    })
}

pub const PIGLIN_TEXTURE: &str = "minecraft:entity/piglin/piglin";
pub const PIGLIN_BABY_TEXTURE: &str = "minecraft:entity/piglin/piglin_baby";
pub const BRUTE_TEXTURE: &str = "minecraft:entity/piglin/piglin_brute";
pub const TEXTURES: [&str; 3] = [PIGLIN_TEXTURE, PIGLIN_BABY_TEXTURE, BRUTE_TEXTURE];

pub fn texture(kind: PiglinKind, baby: bool) -> &'static str {
    match (kind, baby) {
        (PiglinKind::Brute, _) => BRUTE_TEXTURE,
        (PiglinKind::Piglin, true) => PIGLIN_BABY_TEXTURE,
        (PiglinKind::Piglin, false) => PIGLIN_TEXTURE,
    }
}

pub struct Hands {
    pub main: bool,
    pub off: bool,
    pub left_handed: bool,
    pub charge_ticks: f32,
}

fn swing_weapon_down(arms: [&mut Vec3; 2], left_handed: bool, attack: f32, age: f32) {
    use crate::client_mobs::mth_cos;
    let pi = std::f32::consts::PI;
    let sin = |x: f32| minecraftoss_player::mth::sin(f64::from(x));
    let reach = sin(attack * pi);
    let pull = sin((1.0 - (1.0 - attack) * (1.0 - attack)) * pi);
    let [right, left] = arms;
    right.z = 0.0;
    left.z = 0.0;
    right.y = 0.157_079_64;
    left.y = -0.157_079_64;
    let (weapon, other) = if left_handed {
        (left, right)
    } else {
        (right, left)
    };
    weapon.x = -1.884_955_8 + mth_cos(age * 0.09) * 0.15 + reach * 2.2 - pull * 0.4;
    other.x = mth_cos(age * 0.19) * 0.5 + reach * 1.2 - pull * 0.4;
}

fn crossbow_arms(arms: [&mut Vec3; 2], left_handed: bool) -> (&mut Vec3, &mut Vec3) {
    let [right, left] = arms;
    if left_handed {
        (left, right)
    } else {
        (right, left)
    }
}

pub fn humanoid_limbs(
    pose: &MobPose,
    hands: &Hands,
    arm_pose: ArmPose,
    baby: bool,
) -> crate::zombie_render::ZombieLimbs {
    use crate::client_mobs::mth_cos;
    let pi = std::f32::consts::PI;
    let walk = pose.walk_position * 0.6662;
    let speed = pose.walk_speed;
    let mut right_arm = Vec3::new(mth_cos(walk + pi) * 2.0 * speed * 0.5, 0.0, 0.0);
    let mut left_arm = Vec3::new(mth_cos(walk) * 2.0 * speed * 0.5, 0.0, 0.0);
    let right_leg = Vec3::new(mth_cos(walk) * 1.4 * speed, 0.005, 0.005);
    let left_leg = Vec3::new(mth_cos(walk + pi) * 1.4 * speed, -0.005, -0.005);
    if pose.riding {
        right_arm.x += -pi / 5.0;
        left_arm.x += -pi / 5.0;
    }
    let (right_item, left_item) = if hands.left_handed {
        (hands.off, hands.main)
    } else {
        (hands.main, hands.off)
    };
    for (arm, held) in [(&mut right_arm, right_item), (&mut left_arm, left_item)] {
        if held {
            arm.x = arm.x * 0.5 - pi / 10.0;
            arm.y = 0.0;
        }
    }
    let swing = pose.swing.filter(|&swing| swing > 0.0);
    let melee = arm_pose == ArmPose::AttackingWithMeleeWeapon;
    if let Some(attack) = swing.filter(|_| melee) {
        swing_weapon_down(
            [&mut right_arm, &mut left_arm],
            hands.left_handed,
            attack,
            pose.age_in_ticks,
        );
    }
    let bob_z = mth_cos(pose.age_in_ticks * 0.09) * 0.05 + 0.05;
    let bob_x = minecraftoss_player::mth::sin(f64::from(pose.age_in_ticks * 0.067)) * 0.05;
    let bobs = if swing.is_some() && melee { 2.0 } else { 1.0 };
    right_arm.z += bob_z * bobs;
    right_arm.x += bob_x * bobs;
    left_arm.z -= bob_z * bobs;
    left_arm.x -= bob_x * bobs;
    let sign = if hands.left_handed { -1.0 } else { 1.0 };
    let (head_yaw, head_pitch) = (pose.head_yaw.to_radians(), pose.head_pitch.to_radians());
    match arm_pose {
        ArmPose::AttackingWithMeleeWeapon if swing.is_none() => {
            let (weapon, _) = crossbow_arms([&mut right_arm, &mut left_arm], hands.left_handed);
            weapon.x = -1.8;
        }
        ArmPose::CrossbowHold => {
            let (holding, other) =
                crossbow_arms([&mut right_arm, &mut left_arm], hands.left_handed);
            holding.y = -0.3 * sign + head_yaw;
            other.y = 0.6 * sign + head_yaw;
            holding.x = -std::f32::consts::FRAC_PI_2 + head_pitch + 0.1;
            other.x = -1.5 + head_pitch;
        }
        ArmPose::CrossbowCharge => {
            let (holding, other) =
                crossbow_arms([&mut right_arm, &mut left_arm], hands.left_handed);
            holding.y = -0.8 * sign;
            holding.x = -0.970_796_35;
            let charged = hands.charge_ticks.clamp(0.0, 25.0) / 25.0;
            other.y = (0.4 + (0.85 - 0.4) * charged) * sign;
            other.x = holding.x + (-std::f32::consts::FRAC_PI_2 - holding.x) * charged;
        }
        ArmPose::Dancing => {
            let arms = dance(pose.age_in_ticks).arms_z;
            right_arm.z = arms;
            left_arm.z = -arms;
        }
        _ => {}
    }
    if arm_pose == ArmPose::AdmiringItem {
        if hands.left_handed {
            right_arm.y = -0.5;
            right_arm.x = -0.9;
        } else {
            left_arm.y = 0.5;
            left_arm.x = -0.9;
        }
    }
    let part = |r: Vec3| Quat::from_euler(EulerRot::ZYX, r.z, r.y, r.x);
    crate::zombie_render::ZombieLimbs {
        body_yaw: 0.0,
        arm_pivots: rest_of(baby),
        limbs: [
            part(right_arm),
            part(left_arm),
            part(right_leg),
            part(left_leg),
        ],
    }
    .seated(pose.riding)
}

pub fn living_pose(baby: bool, hands: &Hands, arm_pose: ArmPose, pose: &MobPose) -> Posed {
    let head = if arm_pose == ArmPose::AdmiringItem {
        Quat::from_rotation_x(0.5)
    } else {
        head_of(pose)
    };
    let limbs = humanoid_limbs(pose, hands, arm_pose, baby);
    let dancing = (arm_pose == ArmPose::Dancing).then(|| dance(pose.age_in_ticks));
    assemble(baby, head, limbs, pose, dancing)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dance {
    pub ears: [f32; 2],
    pub head: [f32; 2],
    pub arms_y: [f32; 2],
    pub arms_z: f32,
    pub body_y: f32,
}

const STILL: Dance = Dance {
    ears: [0.0; 2],
    head: [0.0; 2],
    arms_y: [0.0; 2],
    arms_z: 0.0,
    body_y: 0.0,
};

pub fn dance(age_in_ticks: f32) -> Dance {
    let f = age_in_ticks / 60.0;
    let sin = |x: f32| minecraftoss_player::mth::sin(f64::from(x));
    let cos = |x: f32| minecraftoss_player::mth::cos(f64::from(x));
    let degree = 0.017_453_292;
    Dance {
        ears: [
            0.523_598_8 + degree * sin(f * 30.0) * 10.0,
            -0.523_598_8 - degree * cos(f * 30.0) * 10.0,
        ],
        head: [sin(f * 10.0), sin(f * 40.0) + 0.4],
        arms_y: [sin(f * 40.0) * 0.5 - 0.5, sin(f * 40.0) * 0.5 + 0.5],
        arms_z: degree * (70.0 + cos(f * 40.0) * 10.0),
        body_y: sin(f * 40.0) * 0.35,
    }
}

pub fn shake_degrees(age_in_ticks: f32, converting: bool) -> f32 {
    if converting {
        ((f64::from(age_in_ticks * 3.25)).cos() * std::f64::consts::PI * f64::from(0.4_f32)) as f32
    } else {
        0.0
    }
}

pub fn append_piglins<'a>(
    mesh: &mut ChunkMesh,
    piglins: impl IntoIterator<Item = &'a PiglinEntity>,
    poses: &crate::client_mobs::ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) -> Vec<HeldItem> {
    let mut held = Vec::new();
    let mut marks = Vec::new();
    for entity in piglins {
        let piglin = &entity.piglin;
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        let mob = MobPose {
            body_rot: mob.body_rot + shake_degrees(mob.age_in_ticks, piglin.converting),
            ..mob
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let baby = piglin.is_baby();
        let Ok(id) = ResourceId::parse(texture(piglin.kind, baby)) else {
            continue;
        };
        let region = atlas.entity_region(&id);
        let sample = mob.light_block();
        let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
        let rotation = mob.body_rotation(90.0);
        let hands = Hands {
            main: piglin.main_hand.is_some(),
            off: piglin.offhand.is_some(),
            left_handed: piglin.left_handed,
            charge_ticks: entity.using_item.unwrap_or(0) as f32,
        };
        let posed = living_pose(baby, &hands, entity.arm_pose, &mob);
        for cube in &posed.cubes {
            let (from, to, uv, dims) = cube.part;
            crate::cow_render::cube_scaled(
                mesh,
                mob.feet,
                rotation,
                Vec3::ONE,
                region,
                sky,
                block,
                from,
                to,
                uv,
                cube.pivot,
                cube.rotation,
                [1.0; 3],
                [64., 64.],
                dims,
                false,
            );
        }
        if !baby && piglin.armor.iter().any(Option::is_some) {
            marks.push((mesh.vertices.len(), 1.0));
            let armor_parts = crate::armor_render::HumanoidParts {
                head: (Vec3::from_array(posed.head_pivot), posed.head),
                body: (Vec3::from_array(posed.body_pivot), posed.body),
                arms: posed
                    .arms
                    .map(|(pivot, rotation)| (Vec3::from_array(pivot), rotation)),
                legs: posed
                    .legs
                    .map(|(pivot, rotation)| (Vec3::from_array(pivot), rotation)),
            };
            crate::armor_render::append_humanoid_armor(
                mesh,
                atlas,
                mob.feet,
                rotation,
                1.0,
                &armor_parts,
                &piglin.armor_ids(),
                sky,
                block,
            );
        }
        for (arm, main) in [(0, !piglin.left_handed), (1, piglin.left_handed)] {
            let Some(id) = hand_key(entity, main) else {
                continue;
            };
            let (pivot, quat) = posed.arms[arm];
            let pivot = Vec3::from_array(pivot);
            let (pose, display) = if arm == 0 {
                (
                    crate::cow_render::right_hand_pose(mob.feet, rotation, pivot, quat, baby),
                    crate::mesh::HeldDisplay::RightHand,
                )
            } else {
                (
                    crate::cow_render::left_hand_pose(mob.feet, rotation, pivot, quat, baby),
                    crate::mesh::HeldDisplay::LeftHand,
                )
            };
            held.push(HeldItem {
                pose,
                light: mob.light_probe.as_vec3(),
                id,
                display,
                first_tint: None,
            });
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
    held
}

pub(crate) fn hand_key(entity: &PiglinEntity, main: bool) -> Option<String> {
    let piglin = &entity.piglin;
    let stack = if main { &piglin.main_hand } else { &piglin.offhand };
    let using = entity.using_item.filter(|_| main == piglin.holds_crossbow());
    Some(crate::item_look::held_key(stack.as_ref()?, using))
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

    #[test]
    fn a_zombified_piglin_has_the_piglin_head_ears_and_the_player_body_with_its_outer_layer() {
        let adult = pose(false, false, &standing());
        assert_eq!(adult.cubes.len(), 4 + 2 + 2 + 4 + 4);
        assert_eq!(
            adult
                .cubes
                .iter()
                .filter(|cube| cube.part.3.is_some())
                .count(),
            5,
            "jacket, sleeves and trousers"
        );
        let baby = pose(true, false, &standing());
        assert_eq!(baby.cubes.len(), 2 + 2 + 1 + 2 + 2);
        let mut mesh = ChunkMesh::default();
        for cube in &adult.cubes {
            let (from, to, uv, dims) = cube.part;
            crate::cow_render::cube_scaled(
                &mut mesh,
                glam::DVec3::ZERO,
                Quat::IDENTITY,
                Vec3::ONE,
                [0.0, 0.0, 1.0, 1.0],
                15.0,
                0.0,
                from,
                to,
                uv,
                cube.pivot,
                cube.rotation,
                [1.0; 3],
                [64., 64.],
                dims,
                false,
            );
        }
        assert_eq!(mesh.vertices.len(), 16 * 24);
    }

    #[test]
    fn its_ears_hang_out_at_thirty_degrees_and_flap_as_it_walks() {
        let [right, left] = ear_rolls(false, 0.0, 0.0, 0.0);
        assert!(
            (right - (30f32.to_radians() + 0.08)).abs() < 1e-5,
            "{right}"
        );
        assert!((left + 30f32.to_radians() + 0.08).abs() < 1e-5, "{left}");
        let [baby_right, _] = ear_rolls(true, 0.0, 0.0, 0.0);
        assert!((baby_right - (5f32.to_radians() + 0.08)).abs() < 1e-5);
        let walking = ear_rolls(false, 0.0, 0.0, 1.0);
        assert!(walking[0] > right, "walking flaps the ears wider");
        let ears = pose(false, false, &standing());
        assert_eq!(ears.cubes[4].pivot, [-4.5, -6.0, 0.0]);
        assert_eq!(ears.cubes[5].pivot, [4.5, -6.0, 0.0]);
    }

    #[test]
    fn a_riding_zombified_piglin_bends_its_legs_forward() {
        let upright = pose(false, false, &standing());
        let mut riding = standing();
        riding.riding = true;
        let seated = pose(false, false, &riding);
        let (_, _, x) = seated.legs[0].1.to_euler(EulerRot::ZYX);
        assert!((x + 1.413_716_7).abs() < 1.0e-5, "{x}");
        assert_eq!(seated.arms[0].1, upright.arms[0].1);
        assert_ne!(seated.legs[1].1, upright.legs[1].1);
    }

    #[test]
    fn a_piglin_and_a_brute_use_their_textures_and_the_piglin_mesh() {
        assert_eq!(texture(PiglinKind::Piglin, false), "minecraft:entity/piglin/piglin");
        assert_eq!(
            texture(PiglinKind::Piglin, true),
            "minecraft:entity/piglin/piglin_baby"
        );
        assert_eq!(
            texture(PiglinKind::Brute, false),
            "minecraft:entity/piglin/piglin_brute"
        );
        let hands = Hands {
            main: true,
            off: false,
            left_handed: false,
            charge_ticks: 0.0,
        };
        let adult = living_pose(false, &hands, ArmPose::Default, &standing());
        assert_eq!(adult.cubes.len(), 4 + 2 + 2 + 4 + 4);
        let baby = living_pose(true, &hands, ArmPose::Default, &standing());
        assert_eq!(baby.cubes.len(), 2 + 2 + 1 + 2 + 2);
        let mut mesh = ChunkMesh::default();
        for cube in &adult.cubes {
            let (from, to, uv, dims) = cube.part;
            crate::cow_render::cube_scaled(
                &mut mesh,
                glam::DVec3::ZERO,
                Quat::IDENTITY,
                Vec3::ONE,
                [0.0, 0.0, 1.0, 1.0],
                15.0,
                0.0,
                from,
                to,
                uv,
                cube.pivot,
                cube.rotation,
                [1.0; 3],
                [64., 64.],
                dims,
                false,
            );
        }
        assert_eq!(mesh.vertices.len(), 16 * 24);
        let (_, _, held_x) = adult.arms[0].1.to_euler(EulerRot::ZYX);
        let empty = Hands {
            main: false,
            off: false,
            left_handed: false,
            charge_ticks: 0.0,
        };
        let bare = living_pose(false, &empty, ArmPose::Default, &standing());
        let (_, _, bare_x) = bare.arms[0].1.to_euler(EulerRot::ZYX);
        assert!(
            (held_x - (bare_x - std::f32::consts::PI / 10.0)).abs() < 1.0e-4,
            "{held_x} {bare_x}"
        );
    }

    #[test]
    fn a_fighting_piglin_raises_its_sword_and_a_crossbow_piglin_pulls_the_string() {
        let mut hands = Hands {
            main: true,
            off: false,
            left_handed: false,
            charge_ticks: 0.0,
        };
        let arm = |hands: &Hands, arm_pose, arm: usize| {
            let posed = living_pose(false, hands, arm_pose, &standing());
            let (_, y, x) = posed.arms[arm].1.to_euler(EulerRot::ZYX);
            (x, y)
        };
        let (x, _) = arm(&hands, ArmPose::AttackingWithMeleeWeapon, 0);
        assert!((x + 1.8).abs() < 1.0e-5, "{x}");
        let (x, y) = arm(&hands, ArmPose::CrossbowHold, 0);
        assert!((x + std::f32::consts::FRAC_PI_2 - 0.1).abs() < 1.0e-5, "{x}");
        assert!((y + 0.3).abs() < 1.0e-5, "{y}");
        let (x, y) = arm(&hands, ArmPose::CrossbowCharge, 1);
        assert!((x + 0.970_796_35).abs() < 1.0e-5 && (y - 0.4).abs() < 1.0e-5);
        hands.charge_ticks = 30.0;
        let (x, y) = arm(&hands, ArmPose::CrossbowCharge, 1);
        assert!((x + std::f32::consts::FRAC_PI_2).abs() < 1.0e-5, "{x}");
        assert!((y - 0.85).abs() < 1.0e-5, "{y}");
        hands.left_handed = true;
        let (x, _) = arm(&hands, ArmPose::AttackingWithMeleeWeapon, 1);
        assert!((x + 1.8).abs() < 1.0e-5, "{x}");
        let mut swinging = standing();
        swinging.swing = Some(0.5);
        let posed = living_pose(false, &hands, ArmPose::AttackingWithMeleeWeapon, &swinging);
        let (_, y, _) = posed.arms[1].1.to_euler(EulerRot::ZYX);
        assert!((y + 0.157_079_64).abs() < 1.0e-4, "{y}");
    }

    #[test]
    fn an_admiring_piglin_holds_its_item_in_the_left_hand() {
        let hands = Hands {
            main: false,
            off: true,
            left_handed: false,
            charge_ticks: 0.0,
        };
        let admiring = living_pose(false, &hands, ArmPose::AdmiringItem, &standing());
        let (_, y, x) = admiring.arms[1].1.to_euler(EulerRot::ZYX);
        assert!((x + 0.9).abs() < 1.0e-5 && (y - 0.5).abs() < 1.0e-5, "{x} {y}");
        let (_, _, head_x) = admiring.head.to_euler(EulerRot::ZYX);
        assert!((head_x - 0.5).abs() < 1.0e-5);
        let right = crate::cow_render::right_hand_pose(
            glam::DVec3::ZERO,
            Quat::IDENTITY,
            Vec3::new(-5.0, 2.0, 0.0),
            Quat::IDENTITY,
            false,
        );
        let left = crate::cow_render::left_hand_pose(
            glam::DVec3::ZERO,
            Quat::IDENTITY,
            Vec3::new(5.0, 2.0, 0.0),
            Quat::IDENTITY,
            false,
        );
        let r = right.transform_point3(Vec3::ZERO);
        let l = left.transform_point3(Vec3::ZERO);
        assert!((r.x + l.x).abs() < 1.0e-5, "{r} {l}");
        assert!((r.y - l.y).abs() < 1.0e-5 && (r.z - l.z).abs() < 1.0e-5);
    }

    #[test]
    fn a_converting_piglin_shakes_its_body() {
        assert_eq!(shake_degrees(7.0, false), 0.0);
        let expected = ((7.0_f64 * 3.25).cos() * std::f64::consts::PI * f64::from(0.4_f32)) as f32;
        assert!((shake_degrees(7.0, true) - expected).abs() < 1.0e-5);
        assert!(shake_degrees(0.0, true) > 1.25);
    }

    #[test]
    fn a_dancing_piglin_bobs_its_head_and_body_and_spreads_its_arms() {
        let hands = Hands {
            main: false,
            off: false,
            left_handed: false,
            charge_ticks: 0.0,
        };
        let mut pose = standing();
        pose.age_in_ticks = 20.0;
        let still = living_pose(false, &hands, ArmPose::Default, &pose);
        let dancing = living_pose(false, &hands, ArmPose::Dancing, &pose);
        let moves = dance(20.0);
        let f = 20.0_f32 / 60.0;
        assert!((moves.head[1] - ((f * 40.0).sin() + 0.4)).abs() < 1.0e-4);
        assert!((moves.arms_z - (70.0 + (f * 40.0).cos() * 10.0).to_radians()).abs() < 1.0e-4);
        assert_eq!(still.head_pivot, [0.0, 0.0, 0.0]);
        assert_eq!(dancing.head_pivot, [moves.head[0], moves.head[1], 0.0]);
        assert!((dancing.body_pivot[1] - moves.body_y).abs() < 1.0e-6);
        assert!((dancing.arms[0].0[1] - (2.0 + moves.arms_y[0])).abs() < 1.0e-6);
        let (z, _, _) = dancing.arms[0].1.to_euler(EulerRot::ZYX);
        assert!((z - moves.arms_z).abs() < 1.0e-4, "{z}");
        let (z, _, _) = dancing.arms[1].1.to_euler(EulerRot::ZYX);
        assert!((z + moves.arms_z).abs() < 1.0e-4, "{z}");
    }

    #[test]
    fn a_piglin_s_crossbow_pulls_in_its_using_hand_and_shows_its_arrow_when_loaded() {
        use crate::mesh::HeldDisplay::{LeftHand, RightHand};
        use minecraftoss_entities::piglin::Piglin;
        use minecraftoss_entities::world::EntityWorld;
        use minecraftoss_player::inventory::ItemStack;
        let atlas = crate::mesh::make_atlas(&crate::pack::PackStack::open(Vec::new()).unwrap(), Vec::new()).unwrap();
        let light = SkyLight::from_levels((-2, 62, -2), (5, 5, 5), vec![15; 125], vec![0; 125]);
        let drawn = |world: &EntityWorld| {
            let mut poses = crate::client_mobs::ClientMobs::default();
            poses.spawn_missing(&crate::client_mobs::server_mobs(world));
            let mut mesh = ChunkMesh::default();
            let held = append_piglins(&mut mesh, world.piglins().iter(), &poses, &atlas, &light, 0.0);
            held.into_iter().map(|item| (item.display, item.id)).collect::<Vec<_>>()
        };
        let mut world = EntityWorld::default();
        let mut piglin = Piglin::new(PiglinKind::Piglin, glam::DVec3::new(0.5, 64.0, 0.5));
        piglin.main_hand = Some(ItemStack::new("minecraft:crossbow", 1));
        piglin.offhand = Some(ItemStack::new("minecraft:gold_ingot", 1));
        let id = world.spawn_piglin(piglin, true);
        let gold = (LeftHand, "minecraft:gold_ingot".to_owned());
        assert_eq!(drawn(&world), [(RightHand, "minecraft:crossbow".to_owned()), gold.clone()]);
        for ticks in [0, 15, 25] {
            {
                let e = world.piglin_mut(id).unwrap();
                e.piglin.charging_crossbow = true;
                e.arm_pose = ArmPose::CrossbowCharge;
                e.using_item = Some(ticks);
            }
            assert_eq!(drawn(&world), [(RightHand, format!("minecraft:crossbow|use={ticks}")), gold.clone()]);
        }
        {
            let e = world.piglin_mut(id).unwrap();
            e.piglin.charging_crossbow = false;
            e.using_item = None;
            e.piglin.set_crossbow_charged(true);
            e.arm_pose = ArmPose::CrossbowHold;
            e.piglin.left_handed = true;
        }
        assert_eq!(drawn(&world), [(RightHand, "minecraft:gold_ingot".to_owned()), (LeftHand, "minecraft:crossbow|charge=arrow".to_owned())]);
        {
            let e = world.piglin_mut(id).unwrap();
            e.piglin.main_hand = Some(ItemStack::new("minecraft:golden_sword", 1));
            e.piglin.offhand = Some(ItemStack::new("minecraft:crossbow", 1));
            e.using_item = Some(10);
        }
        assert_eq!(
            drawn(&world),
            [(RightHand, "minecraft:crossbow|use=10".to_owned()), (LeftHand, "minecraft:golden_sword".to_owned())],
            "with no crossbow in the main hand the off hand is the using hand"
        );
    }
}
