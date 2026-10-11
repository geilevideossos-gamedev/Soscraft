use crate::{
    client_mobs::MobPose,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh, HeldItem},
    pack::ResourceId,
};
use glam::{EulerRot, Quat, Vec3};
use minecraftoss_entities::world::ZombieEntity;

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
    pub head: Quat,
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

pub fn pose(baby: bool, aggressive: bool, pose: &MobPose) -> Posed {
    let model = if baby { &BABY } else { &ADULT };
    let head = Quat::from_euler(
        EulerRot::ZYX,
        0.0,
        pose.head_yaw.to_radians(),
        pose.head_pitch.to_radians(),
    );
    let rest = [
        (model.arm_pivots[0][0], model.arm_pivots[0][2]),
        (model.arm_pivots[1][0], model.arm_pivots[1][2]),
    ];
    let limbs = crate::zombie_render::limb_rotations(
        pose.walk_position,
        pose.walk_speed,
        pose.age_in_ticks,
        aggressive,
        pose.swing,
        if baby { 0.5 } else { 1.0 },
        rest,
    );
    let body = Quat::from_rotation_y(limbs.body_yaw);
    let arms = [0, 1].map(|arm| {
        let (x, z) = limbs.arm_pivots[arm];
        ([x, model.arm_pivots[arm][1], z], limbs.limbs[arm])
    });
    let legs = [0, 1].map(|leg| (model.leg_pivots[leg], limbs.limbs[2 + leg]));
    let mut cubes = Vec::new();
    for &part in model.head {
        cubes.push(PosedCube {
            part,
            pivot: model.head_pivot,
            rotation: head,
        });
    }
    let rolls = ear_rolls(baby, pose.age_in_ticks, pose.walk_position, pose.walk_speed);
    for (side, &(part, offset, child)) in model.ears.iter().enumerate() {
        let ear = head * Quat::from_rotation_z(rolls[side]);
        let (inner, rotation) = match child {
            Some((child_offset, roll)) => (
                ear * Vec3::from_array(child_offset),
                ear * Quat::from_rotation_z(roll),
            ),
            None => (Vec3::ZERO, ear),
        };
        let pivot = Vec3::from_array(model.head_pivot) + head * Vec3::from_array(offset) + inner;
        cubes.push(PosedCube {
            part,
            pivot: pivot.to_array(),
            rotation,
        });
    }
    for &part in model.body {
        cubes.push(PosedCube {
            part,
            pivot: model.body_pivot,
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
        head,
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
}
