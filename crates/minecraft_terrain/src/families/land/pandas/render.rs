use crate::client_mobs::{ClientMobs, MobPose, quadruped_legs};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, HeldDisplay, HeldItem};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::land::pandas::{Gene, Panda, PandaGoals};
use minecraftoss_player::mth;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

pub const FLIP_DEGREES: f32 = 90.0;
pub const ROLL_TICKS: i32 = 32;
pub const ADULT_ROLL_LIFT: f32 = 0.8;
pub const BABY_ROLL_LIFT: f32 = 0.3;
pub const SIT_LIFT: f32 = 0.8;
pub const ADULT_BACK_LIFT: f32 = 1.3;
pub const BABY_BACK_LIFT: f32 = 0.5;
pub const BABY_SCARED_SHIFT: [f32; 3] = [0.0, 0.8, 0.55];
pub const ADULT_SIT_BODY: f32 = 1.7407963;
pub const BABY_SIT_BODY: f32 = 0.17453292;
pub const FRONT_LEG_SIT_Z: f32 = 0.27079642;
pub const HIND_LEG_SIT_Z: f32 = 0.5707964;
pub const SCARED_HEAD: f32 = 2.1707964;
pub const SCARED_FRONT_LEGS: f32 = -0.9;
pub const ROLL_HEAD: f32 = 2.0561945;
pub const HELD_AT: [f32; 3] = [0.1, 1.4, -0.6];

pub const TEXTURES: [&str; 14] = [
    "minecraft:entity/panda/panda",
    "minecraft:entity/panda/panda_lazy",
    "minecraft:entity/panda/panda_worried",
    "minecraft:entity/panda/panda_playful",
    "minecraft:entity/panda/panda_brown",
    "minecraft:entity/panda/panda_weak",
    "minecraft:entity/panda/panda_aggressive",
    "minecraft:entity/panda/panda_baby",
    "minecraft:entity/panda/lazy_panda_baby",
    "minecraft:entity/panda/worried_panda_baby",
    "minecraft:entity/panda/playful_panda_baby",
    "minecraft:entity/panda/brown_panda_baby",
    "minecraft:entity/panda/weak_panda_baby",
    "minecraft:entity/panda/aggressive_panda_baby",
];

pub const HEAD: usize = 0;
pub const BODY: usize = 1;
pub const RIGHT_HIND_LEG: usize = 2;
pub const LEFT_HIND_LEG: usize = 3;
pub const RIGHT_FRONT_LEG: usize = 4;
pub const LEFT_FRONT_LEG: usize = 5;
const LEGS: [usize; 4] = [
    RIGHT_HIND_LEG,
    LEFT_HIND_LEG,
    RIGHT_FRONT_LEG,
    LEFT_FRONT_LEG,
];
const HIND_LEGS: [usize; 2] = [RIGHT_HIND_LEG, LEFT_HIND_LEG];
const FRONT_LEGS: [usize; 2] = [RIGHT_FRONT_LEG, LEFT_FRONT_LEG];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub name: &'static str,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub cubes: &'static [Cube],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    pub parts: [Part; 6],
    pub texture_size: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stance {
    pub look: [f32; 2],
    pub legs: [f32; 4],
    pub age: f32,
    pub x_rot: f32,
    pub baby: bool,
    pub unhappy: bool,
    pub sneezing: bool,
    pub sneeze_time: i32,
    pub eating: bool,
    pub scared: bool,
    pub sitting: bool,
    pub sit: f32,
    pub back: f32,
    pub roll: f32,
    pub roll_time: f32,
}

pub type Parts = ([[f32; 3]; 6], [[f32; 3]; 6]);

const fn part(
    name: &'static str,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &'static [Cube],
) -> Part {
    Part {
        name,
        offset,
        rotation,
        cubes,
    }
}

const NONE: [f32; 3] = [0.0; 3];

const ADULT_HEAD: [Cube; 4] = [
    Cube::new([-6.5, -5.0, -4.0], [13.0, 10.0, 9.0], [0.0, 6.0]),
    Cube::new([-3.5, 0.0, -6.0], [7.0, 5.0, 2.0], [45.0, 16.0]),
    Cube::new([3.5, -8.0, -1.0], [5.0, 4.0, 1.0], [52.0, 25.0]),
    Cube::new([-8.5, -8.0, -1.0], [5.0, 4.0, 1.0], [52.0, 25.0]),
];

const ADULT_BODY: [Cube; 1] = [Cube::new(
    [-9.5, -13.0, -6.5],
    [19.0, 26.0, 13.0],
    [0.0, 25.0],
)];

const ADULT_LEG: [Cube; 1] = [Cube::new([-3.0, 0.0, -3.0], [6.0, 9.0, 6.0], [40.0, 0.0])];

const BABY_HEAD: [Cube; 4] = [
    Cube::new([-3.5, -3.0, -5.0], [7.0, 6.0, 5.0], [0.0, 0.0]),
    Cube::new([-2.0, 1.0, -6.0], [4.0, 2.0, 1.0], [24.0, 6.0]),
    Cube::new([-4.5, -4.0, -3.5], [3.0, 3.0, 1.0], [24.0, 0.0]),
    Cube::new([1.5, -4.0, -3.5], [3.0, 3.0, 1.0], [33.0, 0.0]),
];

const BABY_BODY: [Cube; 1] = [Cube::new([-4.5, -3.5, -5.5], [9.0, 7.0, 11.0], [0.0, 11.0])];

const fn baby_leg(uv: [f32; 2]) -> Cube {
    Cube::new([-1.5, 0.0, -1.5], [3.0, 2.0, 3.0], uv)
}

const BABY_RIGHT_HIND_LEG: [Cube; 1] = [baby_leg([0.0, 34.0])];
const BABY_LEFT_HIND_LEG: [Cube; 1] = [baby_leg([12.0, 34.0])];
const BABY_RIGHT_FRONT_LEG: [Cube; 1] = [baby_leg([0.0, 29.0])];
const BABY_LEFT_FRONT_LEG: [Cube; 1] = [baby_leg([12.0, 29.0])];

pub const ADULT: Model = Model {
    parts: [
        part("head", [0.0, 11.5, -17.0], NONE, &ADULT_HEAD),
        part("body", [0.0, 10.0, 0.0], [FRAC_PI_2, 0.0, 0.0], &ADULT_BODY),
        part("right_hind_leg", [-5.5, 15.0, 9.0], NONE, &ADULT_LEG),
        part("left_hind_leg", [5.5, 15.0, 9.0], NONE, &ADULT_LEG),
        part("right_front_leg", [-5.5, 15.0, -9.0], NONE, &ADULT_LEG),
        part("left_front_leg", [5.5, 15.0, -9.0], NONE, &ADULT_LEG),
    ],
    texture_size: [64.0, 64.0],
};

pub const BABY: Model = Model {
    parts: [
        part("head", [0.0, 19.0, -3.0], NONE, &BABY_HEAD),
        part("body", [0.0, 18.5, 2.5], NONE, &BABY_BODY),
        part(
            "right_hind_leg",
            [-3.0, 22.0, 6.5],
            NONE,
            &BABY_RIGHT_HIND_LEG,
        ),
        part("left_hind_leg", [3.0, 22.0, 6.5], NONE, &BABY_LEFT_HIND_LEG),
        part(
            "right_front_leg",
            [-3.0, 22.0, -1.5],
            NONE,
            &BABY_RIGHT_FRONT_LEG,
        ),
        part(
            "left_front_leg",
            [3.0, 22.0, -1.5],
            NONE,
            &BABY_LEFT_FRONT_LEG,
        ),
    ],
    texture_size: [64.0, 64.0],
};

pub fn model(baby: bool) -> &'static Model {
    if baby { &BABY } else { &ADULT }
}

pub fn texture(variant: Gene, baby: bool) -> &'static str {
    let ages = if baby { Gene::ALL.len() } else { 0 };
    TEXTURES[ages + variant.id() as usize]
}

fn sin(x: f32) -> f32 {
    mth::sin(f64::from(x))
}

fn lerp(t: f32, from: f32, to: f32) -> f32 {
    from + t * (to - from)
}

pub fn rot_lerp_rad(t: f32, from: f32, to: f32) -> f32 {
    let mut turn = to - from;
    while turn < -PI {
        turn += TAU;
    }
    while turn >= PI {
        turn -= TAU;
    }
    from + t * turn
}

fn sit(offsets: &mut [[f32; 3]; 6], rotations: &mut [[f32; 3]; 6], stance: &Stance) {
    let amount = stance.sit;
    if stance.baby {
        rotations[BODY][0] = rot_lerp_rad(amount, rotations[BODY][0], BABY_SIT_BODY);
        offsets[BODY][2] = lerp(amount, offsets[BODY][2], -1.5);
        offsets[HEAD][2] = lerp(amount, offsets[HEAD][2], -11.5);
        offsets[HEAD][1] = lerp(amount, offsets[HEAD][1], 17.5);
        for leg in FRONT_LEGS {
            offsets[leg][2] = lerp(amount, offsets[leg][2], -5.0);
        }
        for leg in HIND_LEGS {
            offsets[leg][2] = lerp(amount, offsets[leg][2], 3.0);
        }
    } else {
        rotations[BODY][0] = rot_lerp_rad(amount, rotations[BODY][0], ADULT_SIT_BODY);
        rotations[HEAD][0] = rot_lerp_rad(amount, rotations[HEAD][0], FRAC_PI_2);
    }
    rotations[RIGHT_FRONT_LEG][2] = -FRONT_LEG_SIT_Z;
    rotations[LEFT_FRONT_LEG][2] = FRONT_LEG_SIT_Z;
    rotations[RIGHT_HIND_LEG][2] = HIND_LEG_SIT_Z;
    rotations[LEFT_HIND_LEG][2] = -HIND_LEG_SIT_Z;
}

pub fn setup_anim(model: &Model, stance: &Stance) -> Parts {
    let mut offsets = model.parts.map(|part| part.offset);
    let mut rotations = model.parts.map(|part| part.rotation);
    let [pitch, yaw] = stance.look;
    rotations[HEAD][0] = pitch;
    rotations[HEAD][1] = yaw;
    for (leg, swing) in LEGS.into_iter().zip(stance.legs) {
        rotations[leg][0] = swing;
    }
    let age = stance.age;
    if stance.unhappy {
        let shake = 0.35 * sin(0.6 * age);
        rotations[HEAD][1] = shake;
        rotations[HEAD][2] = shake;
        rotations[RIGHT_FRONT_LEG][0] = -0.75 * sin(0.3 * age);
        rotations[LEFT_FRONT_LEG][0] = 0.75 * sin(0.3 * age);
    } else {
        rotations[HEAD][2] = 0.0;
    }
    if stance.sneezing {
        let time = stance.sneeze_time;
        if time < 15 {
            rotations[HEAD][0] = -FRAC_PI_4 * time as f32 / 14.0;
        } else if time < 20 {
            let back = ((time - 15) / 5) as f32;
            rotations[HEAD][0] = -FRAC_PI_4 + FRAC_PI_4 * back;
        }
    }
    if stance.sit > 0.0 {
        sit(&mut offsets, &mut rotations, stance);
        if stance.eating {
            rotations[HEAD][0] = FRAC_PI_2 + 0.2 * sin(age * 0.6);
            for leg in FRONT_LEGS {
                rotations[leg][0] = -0.4 - 0.2 * sin(age * 0.6);
            }
        }
        if stance.scared {
            rotations[HEAD][0] = SCARED_HEAD;
            for leg in FRONT_LEGS {
                rotations[leg][0] = SCARED_FRONT_LEGS;
            }
        }
    } else {
        for leg in LEGS {
            rotations[leg][2] = 0.0;
        }
    }
    if stance.back > 0.0 {
        rotations[RIGHT_HIND_LEG][0] = -0.6 * sin(age * 0.15);
        rotations[LEFT_HIND_LEG][0] = 0.6 * sin(age * 0.15);
        rotations[RIGHT_FRONT_LEG][0] = 0.3 * sin(age * 0.25);
        rotations[LEFT_FRONT_LEG][0] = -0.3 * sin(age * 0.25);
        rotations[HEAD][0] = rot_lerp_rad(stance.back, rotations[HEAD][0], FRAC_PI_2);
    }
    if stance.roll > 0.0 {
        rotations[HEAD][0] = rot_lerp_rad(stance.roll, rotations[HEAD][0], ROLL_HEAD);
        let kick = 0.5 * sin(age * 0.5);
        rotations[RIGHT_HIND_LEG][0] = -kick;
        rotations[LEFT_HIND_LEG][0] = kick;
        rotations[RIGHT_FRONT_LEG][0] = kick;
        rotations[LEFT_FRONT_LEG][0] = -kick;
    }
    (offsets, rotations)
}

pub fn part_poses(model: &Model, stance: &Stance) -> [Mat4; 6] {
    let (offsets, rotations) = setup_anim(model, stance);
    std::array::from_fn(|index| part_pose(offsets[index], rotations[index]))
}

pub fn roll_turn(roll_time: f32, baby: bool) -> Option<(f32, f32)> {
    if roll_time <= 0.0 {
        return None;
    }
    let tick = roll_time.floor() as i32;
    let frac = roll_time - tick as f32;
    let (now, next) = (tick as f32, (tick + 1) as f32);
    let lift = if baby {
        BABY_ROLL_LIFT
    } else {
        ADULT_ROLL_LIFT
    };
    let angle = |from: f32, to: f32, end: f32| {
        if next < end {
            lerp(frac, from, to)
        } else {
            from
        }
    };
    if now < 8.0 {
        let turn = angle(90.0 * now / 7.0, 90.0 * next / 7.0, 8.0);
        Some(((lift + 0.2) * (turn / 90.0), turn))
    } else if now < 16.0 {
        let from = 90.0 + 90.0 * ((now - 8.0) / 7.0);
        let turn = angle(from, 90.0 + 90.0 * (next - 8.0) / 7.0, 16.0);
        Some((lift + 0.2 + (lift - 0.2) * (turn - 90.0) / 90.0, turn))
    } else if now < 24.0 {
        let from = 180.0 + 90.0 * ((now - 16.0) / 7.0);
        let turn = angle(from, 180.0 + 90.0 * (next - 16.0) / 7.0, 24.0);
        Some((lift + lift * (270.0 - turn) / 90.0, turn))
    } else if tick < ROLL_TICKS {
        let from = 270.0 + 90.0 * ((now - 24.0) / 7.0);
        let turn = angle(from, 270.0 + 90.0 * (next - 24.0) / 7.0, 32.0);
        Some((lift * ((360.0 - turn) / 90.0), turn))
    } else {
        None
    }
}

fn lift(height: f32) -> Mat4 {
    Mat4::from_translation(Vec3::Y * height)
}

pub fn body_turns(stance: &Stance) -> Mat4 {
    let mut turns = Mat4::IDENTITY;
    if let Some((height, angle)) = roll_turn(stance.roll_time, stance.baby) {
        turns *= lift(height) * Mat4::from_rotation_x((-angle).to_radians());
    }
    let sit = stance.sit;
    if sit > 0.0 {
        let tilt = lerp(sit, stance.x_rot, stance.x_rot + 90.0);
        turns *= lift(SIT_LIFT * sit) * Mat4::from_rotation_x(tilt.to_radians()) * lift(-sit);
        if stance.scared {
            let swing = f64::from(stance.age * 1.25).cos() * std::f64::consts::PI;
            let tremble = (swing * 0.05000000074505806) as f32;
            turns *= Mat4::from_rotation_y(tremble.to_radians());
            if stance.baby {
                turns *= Mat4::from_translation(Vec3::from_array(BABY_SCARED_SHIFT));
            }
        }
    }
    let back = stance.back;
    if back > 0.0 {
        let height = if stance.baby {
            BABY_BACK_LIFT
        } else {
            ADULT_BACK_LIFT
        };
        let tilt = lerp(back, stance.x_rot, stance.x_rot + 180.0);
        turns *= lift(height * back) * Mat4::from_rotation_x(tilt.to_radians());
    }
    turns
}

pub fn turned_root(stance: &Stance) -> Mat4 {
    body_turns(stance) * lift(1.501) * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn model_root(pose: &MobPose, stance: &Stance) -> Mat4 {
    Mat4::from_translation(pose.feet.as_vec3())
        * Mat4::from_quat(pose.body_rotation(FLIP_DEGREES))
        * turned_root(stance)
}

pub fn held_pose(root: Mat4, stance: &Stance) -> Option<Mat4> {
    if !stance.sitting || stance.scared {
        return None;
    }
    let [x, mut y, mut z] = HELD_AT;
    if stance.eating {
        let chew = sin(stance.age * 0.6);
        z -= 0.2 * chew + 0.2;
        y -= 0.09 * chew;
    }
    Some(root * Mat4::from_translation(Vec3::new(x, y, z)))
}

pub fn stance(pose: &MobPose, panda: &Panda, baby: bool, partial: f32) -> Stance {
    let fallback;
    let goals = match panda.goals() {
        Some(goals) => goals,
        None => {
            fallback = PandaGoals::default();
            &fallback
        }
    };
    let roll_time = if goals.roll_counter > 0 {
        goals.roll_counter as f32 + partial
    } else {
        0.0
    };
    Stance {
        look: [pose.head_pitch.to_radians(), pose.head_yaw.to_radians()],
        legs: quadruped_legs(pose.walk_position, pose.walk_speed),
        age: pose.age_in_ticks,
        x_rot: pose.head_pitch,
        baby,
        unhappy: goals.unhappy > 0,
        sneezing: goals.sneezing,
        sneeze_time: goals.sneeze_counter,
        eating: goals.eating(),
        scared: goals.scared,
        sitting: goals.sitting,
        sit: panda.sit_amount.at(partial),
        back: panda.back_amount.at(partial),
        roll: if baby {
            0.0
        } else {
            panda.roll_amount.at(partial)
        },
        roll_time,
    }
}

fn skin(atlas: &Atlas, texture: &str, size: [f32; 2], light: &SkyLight, pose: &MobPose) -> Skin {
    let position = pose.light_block();
    Skin {
        region: ResourceId::parse(texture).map_or([0.0; 4], |id| atlas.entity_region(&id)),
        texture_size: size,
        sky: light.get(position) as f32,
        block: light.get_block(position) as f32,
        alpha: 1.0,
        shaded: true,
    }
}

pub fn append_model(
    mesh: &mut ChunkMesh,
    root: Mat4,
    model: &Model,
    poses: &[Mat4; 6],
    skin: &Skin,
) {
    for (part, pose) in model.parts.iter().zip(poses) {
        for cube in part.cubes {
            append(mesh, root * *pose, skin, cube);
        }
    }
}

pub fn append_pandas<'a>(
    mesh: &mut ChunkMesh,
    held: &mut Vec<HeldItem>,
    pandas: impl IntoIterator<Item = &'a FamilyMob<Panda>>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let mut marks = Vec::new();
    for panda in pandas {
        let Some(pose) = poses.pose(panda.core.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), pose.overlay(0.0)));
        let baby = panda.core.baby;
        let model = model(baby);
        let texture = texture(panda.state.variant(), baby);
        let skin = skin(atlas, texture, model.texture_size, light, &pose);
        let stance = stance(&pose, &panda.state, baby, partial);
        let root = model_root(&pose, &stance);
        append_model(mesh, root, model, &part_poses(model, &stance), &skin);
        let item = panda.state.held.as_ref();
        if let (Some(stack), Some(at)) = (item, held_pose(root, &stance)) {
            held.push(HeldItem {
                pose: at,
                light: pose.light_probe.as_vec3(),
                id: stack.id.clone(),
                display: HeldDisplay::Ground,
                first_tint: None,
            });
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use crate::families::{FamilyDraw, TerrainFamily};
    use crate::mob_tracking::{shows, spawn_tracked};
    use crate::server_mobs::{mob_tags, spawn_saved};
    use minecraftoss_core::nbt::Tag;
    use minecraftoss_entities::world::EntityWorld;
    use minecraftoss_player::inventory::ItemStack;
    use std::collections::{HashMap, HashSet};

    const BAMBOO_PAW: &str = "{id:\"minecraft:panda\",Pos:[0.5d,64.0d,0.5d],equipment:{mainhand:{id:\"minecraft:bamboo\",count:1}}}";
    const UPRIGHT_TOP: f32 = 1.501 - 3.5 / 16.0;
    const CUB_TOP: f32 = 1.501 - 15.0 / 16.0;

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let skins = root.join("assets/minecraft/textures/entity/panda");
        std::fs::create_dir_all(&skins).unwrap();
        for texture in TEXTURES {
            let name = texture.rsplit('/').next().unwrap();
            image::RgbaImage::from_pixel(64, 64, image::Rgba([230, 230, 230, 255]))
                .save(skins.join(format!("{name}.png")))
                .unwrap();
        }
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let ids = TEXTURES
            .iter()
            .map(|texture| ResourceId::parse(texture).unwrap())
            .collect();
        crate::mesh::make_atlas(&packs, ids).unwrap()
    }

    fn light() -> SkyLight {
        SkyLight::from_levels((-8, 56, -8), (17, 17, 17), vec![15; 4913], vec![2; 4913])
    }

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    fn spawn(world: &mut EntityWorld, extra: &str) -> u64 {
        let text = format!("{{id:\"minecraft:panda\",Pos:[0.5d,64.0d,0.5d]{extra}}}");
        spawn_saved(world, &parse(&text)).unwrap()
    }

    fn goals(world: &mut EntityWorld, id: u64) -> &mut PandaGoals {
        world.family_mob_mut::<Panda>(id).unwrap().state.goals_mut()
    }

    fn settle(world: &mut EntityWorld, id: u64, ticks: usize) {
        for _ in 0..ticks {
            world
                .family_mob_mut::<Panda>(id)
                .unwrap()
                .state
                .step_amounts();
        }
    }

    fn friend_of(host: &EntityWorld, id: u64, tag: Tag) -> EntityWorld {
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let mut saved = mob_tags(host, |_| true, &originals);
        saved.retain(|(saved, _)| *saved == id);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved.remove(0).1));
        friend
    }

    fn mirror(host: &EntityWorld, friend: &mut EntityWorld, id: u64) {
        let show = shows(host)[&id].family.unwrap();
        Panda::show_on(friend.family_mob_mut::<Panda>(id).unwrap(), &show);
    }

    fn pivot(pose: Mat4) -> Vec3 {
        pose.transform_point3(Vec3::ZERO) * 16.0
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        a.abs_diff_eq(b, 2e-3)
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.005
    }

    fn sine(x: f32) -> f32 {
        mth::sin(f64::from(x))
    }

    fn drawn(world: &EntityWorld, atlas: &Atlas, partial: f32) -> (ChunkMesh, Vec<HeldItem>) {
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&server_mobs(world));
        let mut models = ChunkMesh::default();
        let mut culled = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let mut held = Vec::new();
        Panda::draw(
            &mut FamilyDraw {
                models: &mut models,
                culled: &mut culled,
                translucent: &mut translucent,
                held: &mut held,
            },
            world.family::<Panda>(),
            &poses,
            atlas,
            &light(),
            partial,
        );
        assert!(models.vertices.is_empty() && translucent.vertices.is_empty());
        (culled, held)
    }

    fn positions(mesh: &ChunkMesh) -> Vec<[f32; 3]> {
        mesh.vertices.iter().map(|v| v.position).collect()
    }

    fn held_items(held: &[HeldItem]) -> Vec<(String, Mat4, HeldDisplay)> {
        held.iter()
            .map(|item| (item.id.clone(), item.pose, item.display))
            .collect()
    }

    fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
        mesh.vertices
            .iter()
            .map(|v| v.position[axis])
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
    }

    fn region(atlas: &Atlas, texture: &str) -> [f32; 4] {
        atlas.entity_region(&ResourceId::parse(texture).unwrap())
    }

    fn inside(region: [f32; 4], mesh: &ChunkMesh) -> bool {
        mesh.vertices.iter().all(|v| {
            (region[0]..=region[2]).contains(&v.uv[0]) && (region[1]..=region[3]).contains(&v.uv[1])
        })
    }

    #[test]
    fn an_adult_and_a_cub_have_nine_cubes_each_on_their_own_models() {
        let cubes = |model: &Model| model.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
        assert_eq!((cubes(&ADULT), cubes(&BABY)), (9, 9));
        for model in [&ADULT, &BABY] {
            assert_eq!(model.texture_size, [64.0, 64.0]);
            assert_eq!(
                model.parts.map(|part| part.name),
                [
                    "head",
                    "body",
                    "right_hind_leg",
                    "left_hind_leg",
                    "right_front_leg",
                    "left_front_leg"
                ]
            );
            let mut cubes = model.parts.iter().flat_map(|part| part.cubes);
            assert!(cubes.all(|cube| !cube.mirror));
        }
        assert_eq!(model(false), &ADULT);
        assert_eq!(model(true), &BABY);
        assert_eq!(Panda::TEXTURES, TEXTURES);
        assert_eq!(Panda::SHADOW_RADIUS, 0.9);
        let still = part_poses(&ADULT, &Stance::default());
        assert!(close(pivot(still[HEAD]), Vec3::new(0.0, 11.5, -17.0)));
        let along = still[BODY].transform_vector3(Vec3::NEG_Y);
        assert!(
            close(along, Vec3::NEG_Z),
            "the body lies along the panda: {along}"
        );
        assert!(close(
            pivot(still[LEFT_FRONT_LEG]),
            Vec3::new(5.5, 15.0, -9.0)
        ));
        let baby = Stance {
            baby: true,
            ..Stance::default()
        };
        let cub = part_poses(&BABY, &baby);
        assert!(close(pivot(cub[BODY]), Vec3::new(0.0, 18.5, 2.5)));
        assert!(close(cub[BODY].transform_vector3(Vec3::NEG_Y), Vec3::NEG_Y));
        assert!(close(pivot(cub[HEAD]), Vec3::new(0.0, 19.0, -3.0)));
        assert!(close(
            pivot(cub[RIGHT_HIND_LEG]),
            Vec3::new(-3.0, 22.0, 6.5)
        ));

        let atlas = atlas();
        let mut world = EntityWorld::default();
        let adult = spawn(&mut world, "");
        let (culled, held) = drawn(&world, &atlas, 0.0);
        assert!(held.is_empty());
        assert_eq!(culled.faces, 54, "nine cubes");
        let (low, high) = span(&culled, 1);
        assert!(near(low, 64.001), "the paws reach the ground: {low}");
        assert!(
            near(high, 64.0 + UPRIGHT_TOP),
            "the ears and back on top: {high}"
        );
        let (west, east) = span(&culled, 0);
        let (north, south) = span(&culled, 2);
        assert!(near(south - north, 36.0 / 16.0), "nose to rump");
        assert!(near(east - west, 19.0 / 16.0), "the body is widest");
        assert!(inside(region(&atlas, TEXTURES[0]), &culled));
        assert!(
            culled
                .vertices
                .iter()
                .all(|v| v.sky_light == 15.0 && v.block_light == 2.0 && v.color[3] > 0.0)
        );

        world.family_core_mut(adult).unwrap().1.removed = true;
        spawn(&mut world, ",Age:-24000");
        let (cub, _) = drawn(&world, &atlas, 0.0);
        assert_eq!(cub.faces, 54, "nine cubes, the adult that left not drawn");
        let (low, high) = span(&cub, 1);
        assert!(
            near(low, 64.001) && near(high, 64.0 + CUB_TOP),
            "{low} {high}"
        );
        let (west, east) = span(&cub, 0);
        let (north, south) = span(&cub, 2);
        assert!(near(south - north, 17.0 / 16.0) && near(east - west, 9.0 / 16.0));
        assert!(inside(region(&atlas, TEXTURES[7]), &cub));
    }

    #[test]
    fn each_gene_and_age_draws_its_texture() {
        for gene in Gene::ALL {
            let name = gene.name();
            let (adult, cub) = if gene == Gene::Normal {
                ("panda".to_owned(), "panda_baby".to_owned())
            } else {
                (format!("panda_{name}"), format!("{name}_panda_baby"))
            };
            let path = |file: &str| format!("minecraft:entity/panda/{file}");
            assert_eq!(texture(gene, false), path(&adult));
            assert_eq!(texture(gene, true), path(&cub));
        }
        assert_eq!(TEXTURES.iter().collect::<HashSet<_>>().len(), 14);
        let atlas = atlas();
        for (genes, age, expected) in [
            ("weak\",HiddenGene:\"weak", 0, "panda_weak"),
            ("weak\",HiddenGene:\"normal", 0, "panda"),
            ("lazy\",HiddenGene:\"brown", 0, "panda_lazy"),
            ("worried\",HiddenGene:\"worried", 0, "panda_worried"),
            ("brown\",HiddenGene:\"brown", -24000, "brown_panda_baby"),
            (
                "aggressive\",HiddenGene:\"normal",
                -24000,
                "aggressive_panda_baby",
            ),
            (
                "playful\",HiddenGene:\"normal",
                -24000,
                "playful_panda_baby",
            ),
        ] {
            let mut world = EntityWorld::default();
            spawn(&mut world, &format!(",MainGene:\"{genes}\",Age:{age}"));
            let (culled, _) = drawn(&world, &atlas, 0.0);
            let skin = region(&atlas, &format!("minecraft:entity/panda/{expected}"));
            assert!(inside(skin, &culled), "{genes} at age {age}");
        }
    }

    #[test]
    fn a_sitting_panda_sits_upright_and_an_eating_one_works_its_paws() {
        let sat = Stance {
            sit: 1.0,
            ..Stance::default()
        };
        let (_, turned) = setup_anim(&ADULT, &sat);
        assert_eq!(
            (turned[BODY][0], turned[HEAD][0]),
            (ADULT_SIT_BODY, FRAC_PI_2)
        );
        assert_eq!(
            LEGS.map(|leg| turned[leg][2]),
            [0.5707964, -0.5707964, -0.27079642, 0.27079642]
        );
        let half = Stance {
            sit: 0.5,
            ..Stance::default()
        };
        let (_, halfway) = setup_anim(&ADULT, &half);
        let body = FRAC_PI_2 + 0.5 * (ADULT_SIT_BODY - FRAC_PI_2);
        assert!((halfway[BODY][0] - body).abs() < 1e-6);
        let (_, standing) = setup_anim(&ADULT, &Stance::default());
        assert!(LEGS.iter().all(|&leg| standing[leg][2] == 0.0));

        let cub = Stance {
            sit: 1.0,
            baby: true,
            ..Stance::default()
        };
        let (moved, turned) = setup_anim(&BABY, &cub);
        assert_eq!(turned[BODY][0], BABY_SIT_BODY);
        assert_eq!(turned[HEAD][0], 0.0, "a cub's head keeps its look");
        assert_eq!(
            (moved[BODY][2], moved[HEAD][1], moved[HEAD][2]),
            (-1.5, 17.5, -11.5)
        );
        assert_eq!(LEGS.map(|leg| moved[leg][2]), [3.0, 3.0, -5.0, -5.0]);
        let (moved, _) = setup_anim(&BABY, &Stance { sit: 0.5, ..cub });
        assert_eq!(moved[HEAD][2], -7.25);

        let root = turned_root(&sat);
        let poses = part_poses(&ADULT, &sat);
        let back = (root * poses[BODY]).transform_vector3(Vec3::NEG_Y);
        assert!(back.y > 0.98, "the back stands upright: {back}");
        let face = (root * poses[HEAD]).transform_vector3(Vec3::NEG_Z);
        let still = part_poses(&ADULT, &Stance::default())[HEAD];
        let ahead = (turned_root(&Stance::default()) * still).transform_vector3(Vec3::NEG_Z);
        assert!(close(face, ahead), "the head still looks ahead: {face}");

        let chewing = |age: f32| {
            let eating = Stance {
                eating: true,
                age,
                ..sat
            };
            setup_anim(&ADULT, &eating).1
        };
        for age in [1.0f32, 4.0] {
            let turned = chewing(age);
            let chew = sine(age * 0.6);
            assert_eq!(turned[HEAD][0], FRAC_PI_2 + 0.2 * chew);
            let paws = (turned[RIGHT_FRONT_LEG][0], turned[LEFT_FRONT_LEG][0]);
            assert_eq!(paws, (-0.4 - 0.2 * chew, -0.4 - 0.2 * chew));
        }
        assert_ne!(chewing(1.0)[RIGHT_FRONT_LEG], chewing(4.0)[RIGHT_FRONT_LEG]);
        let idle = Stance {
            eating: true,
            age: 4.0,
            ..Stance::default()
        };
        let (_, idle) = setup_anim(&ADULT, &idle);
        assert_eq!(
            (idle[HEAD][0], idle[RIGHT_FRONT_LEG][0]),
            (0.0, 0.0),
            "eating shows only while sitting"
        );

        let atlas = atlas();
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, "");
        let (standing, _) = drawn(&world, &atlas, 0.0);
        goals(&mut world, id).sitting = true;
        settle(&mut world, id, 1);
        let (first, _) = drawn(&world, &atlas, 0.0);
        assert_eq!(
            positions(&first),
            positions(&standing),
            "partial 0 is last tick's"
        );
        settle(&mut world, id, 6);
        let (sitting, _) = drawn(&world, &atlas, 1.0);
        let (low, high) = span(&sitting, 1);
        assert!(
            near(high, 64.0 + 0.8 + 25.0 / 16.0),
            "its ears on top: {high}"
        );
        assert!(
            near(low, 63.9305),
            "its rump sinks a little, as in vanilla: {low}"
        );

        let mut host = EntityWorld::default();
        let tag = parse(BAMBOO_PAW);
        let id = spawn_saved(&mut host, &tag).unwrap();
        let mut friend = friend_of(&host, id, tag);
        goals(&mut host, id).sitting = true;
        goals(&mut host, id).eat_counter = 1;
        for _ in 0..3 {
            settle(&mut host, id, 1);
            mirror(&host, &mut friend, id);
        }
        for partial in [0.0, 0.25, 0.5, 1.0] {
            let (mesh, held) = drawn(&host, &atlas, partial);
            let (seen, seen_held) = drawn(&friend, &atlas, partial);
            assert_eq!(positions(&seen), positions(&mesh), "at partial {partial}");
            assert_eq!(held_items(&seen_held), held_items(&held));
            assert_eq!(held.len(), 1);
        }
    }

    #[test]
    fn a_rolling_panda_turns_a_full_circle_in_thirty_two_ticks() {
        assert_eq!(roll_turn(0.0, false), None);
        let turn = |time: f32| roll_turn(time, false).unwrap();
        let about = |(lift, angle): (f32, f32), (high, turned): (f32, f32)| {
            (lift - high).abs() < 1e-4 && (angle - turned).abs() < 1e-3
        };
        for (time, expected) in [
            (3.5, (0.5, 45.0)),
            (7.5, (1.0, 90.0)),
            (8.0, (1.0, 90.0)),
            (11.5, (1.3, 135.0)),
            (15.5, (1.6, 180.0)),
            (19.5, (1.2, 225.0)),
            (23.5, (0.8, 270.0)),
            (27.5, (0.4, 315.0)),
            (31.5, (0.0, 360.0)),
        ] {
            assert!(about(turn(time), expected), "{time}: {:?}", turn(time));
        }
        assert_eq!(roll_turn(32.0, false), None);
        assert_eq!(roll_turn(40.5, false), None);
        let mut last = 0.0;
        for step in 1..128 {
            let (_, angle) = turn(step as f32 / 4.0);
            assert!(angle >= last, "{step}: {angle} after {last}");
            last = angle;
        }
        assert_eq!(last, 360.0, "a full circle");
        assert!(about(roll_turn(15.5, true).unwrap(), (0.6, 180.0)));
        assert!(about(roll_turn(3.5, true).unwrap(), (0.25, 45.0)));

        let rolling = Stance {
            roll: 1.0,
            age: 2.0,
            look: [0.3, 0.0],
            ..Stance::default()
        };
        let (_, turned) = setup_anim(&ADULT, &rolling);
        assert!((turned[HEAD][0] - ROLL_HEAD).abs() < 1e-6);
        let kick = 0.5 * sine(1.0);
        assert_eq!(LEGS.map(|leg| turned[leg][0]), [-kick, kick, kick, -kick]);

        let atlas = atlas();
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, "");
        goals(&mut world, id).roll_counter = 15;
        let (rolled, _) = drawn(&world, &atlas, 0.5);
        let (low, high) = span(&rolled, 1);
        assert!(
            near(high, 64.0 + 1.6 - 0.001),
            "upside down, paws up: {high}"
        );
        assert!(near(low, 64.0 + 1.6 - UPRIGHT_TOP), "{low}");

        world.family_core_mut(id).unwrap().1.removed = true;
        let cub = spawn(&mut world, ",Age:-24000");
        goals(&mut world, cub).roll_counter = 15;
        goals(&mut world, cub).rolling = true;
        settle(&mut world, cub, 7);
        let (rolled, _) = drawn(&world, &atlas, 0.5);
        let (low, high) = span(&rolled, 1);
        assert!(near(high, 64.0 + 0.6 - 0.001), "a cub rolls lower: {high}");
        assert!(near(low, 64.0 + 0.6 - CUB_TOP), "{low}");
    }

    #[test]
    fn a_panda_on_its_back_and_a_scared_panda_pose_like_vanilla() {
        let age = 3.0f32;
        let lying = Stance {
            back: 1.0,
            age,
            look: [0.3, 0.0],
            ..Stance::default()
        };
        let (_, turned) = setup_anim(&ADULT, &lying);
        let (hind, front) = (0.6 * sine(age * 0.15), 0.3 * sine(age * 0.25));
        assert_eq!(LEGS.map(|leg| turned[leg][0]), [-hind, hind, front, -front]);
        assert!((turned[HEAD][0] - FRAC_PI_2).abs() < 1e-6);
        let (_, half) = setup_anim(&ADULT, &Stance { back: 0.5, ..lying });
        assert!((half[HEAD][0] - (0.3 + 0.5 * (FRAC_PI_2 - 0.3))).abs() < 1e-6);
        let flipped = turned_root(&Stance {
            back: 1.0,
            ..Stance::default()
        });
        let paw = flipped.transform_point3(Vec3::new(5.5, 24.0, 9.0) / 16.0);
        let spine = flipped.transform_point3(Vec3::new(0.0, 3.5, 0.0) / 16.0);
        assert!((paw.y - (1.3 - 0.001)).abs() < 1e-4, "{paw}");
        assert!((spine.y - (1.3 - UPRIGHT_TOP)).abs() < 1e-4, "{spine}");
        let cub = turned_root(&Stance {
            back: 1.0,
            baby: true,
            ..Stance::default()
        });
        let paw = cub.transform_point3(Vec3::new(3.0, 24.0, 6.5) / 16.0);
        assert!((paw.y - 0.499).abs() < 1e-4, "{paw}");

        let atlas = atlas();
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, "");
        goals(&mut world, id).on_back = true;
        settle(&mut world, id, 7);
        let (lying, _) = drawn(&world, &atlas, 1.0);
        let (low, high) = span(&lying, 1);
        assert!(near(low, 64.0 + 1.3 - UPRIGHT_TOP), "on its back: {low}");
        assert!(high > 65.29 && high < 65.32, "its paws in the air: {high}");

        let scared = Stance {
            sit: 1.0,
            scared: true,
            eating: true,
            age: 2.0,
            ..Stance::default()
        };
        let (_, turned) = setup_anim(&ADULT, &scared);
        assert_eq!(
            (
                turned[HEAD][0],
                turned[RIGHT_FRONT_LEG][0],
                turned[LEFT_FRONT_LEG][0]
            ),
            (SCARED_HEAD, -0.9, -0.9),
            "it covers its face"
        );
        let calm = Stance {
            scared: false,
            ..scared
        };
        let tremble = body_turns(&calm).inverse() * body_turns(&scared);
        let x = tremble.transform_vector3(Vec3::X);
        let degrees = (2.5f64.cos() * std::f64::consts::PI * 0.05) as f32;
        assert!(((-x.z).atan2(x.x).to_degrees() - degrees).abs() < 1e-3);
        let cub = Stance {
            baby: true,
            ..scared
        };
        let calm_cub = Stance {
            scared: false,
            ..cub
        };
        let shift = body_turns(&calm_cub).inverse() * body_turns(&cub);
        let moved = shift.transform_point3(Vec3::ZERO);
        assert!(close(moved, Vec3::new(0.0, 0.8, 0.55)), "{moved}");
    }

    #[test]
    fn a_sneezing_panda_holds_its_head_back_from_tick_fifteen() {
        let head = |sneezing: bool, sneeze_time: i32| {
            let stance = Stance {
                sneezing,
                sneeze_time,
                look: [0.3, 0.0],
                ..Stance::default()
            };
            setup_anim(&ADULT, &stance).1[HEAD][0]
        };
        assert_eq!(head(true, 0), 0.0);
        assert!((head(true, 7) + PI / 8.0).abs() < 1e-6);
        assert!((head(true, 14) + FRAC_PI_4).abs() < 1e-6);
        for time in 15..20 {
            assert_eq!(head(true, time), -FRAC_PI_4, "{time}");
        }
        assert_eq!(head(true, 20), 0.3, "it looks about again");
        assert_eq!(head(false, 10), 0.3);
    }

    #[test]
    fn an_unhappy_panda_shakes_its_head_and_paws() {
        let age = 5.0f32;
        let sulking = Stance {
            unhappy: true,
            age,
            look: [0.2, 0.4],
            ..Stance::default()
        };
        let (_, turned) = setup_anim(&ADULT, &sulking);
        let shake = 0.35 * sine(0.6 * age);
        assert_eq!(turned[HEAD], [0.2, shake, shake]);
        let paw = 0.75 * sine(0.3 * age);
        assert_eq!(
            (turned[RIGHT_FRONT_LEG][0], turned[LEFT_FRONT_LEG][0]),
            (-paw, paw)
        );
        let calm = Stance {
            unhappy: false,
            ..sulking
        };
        assert_eq!(setup_anim(&ADULT, &calm).1[HEAD], [0.2, 0.4, 0.0]);
    }

    #[test]
    fn a_sitting_panda_holds_its_item_on_the_ground_display_unless_scared() {
        let holding = Stance {
            sitting: true,
            ..Stance::default()
        };
        let at = |stance: &Stance| {
            held_pose(Mat4::IDENTITY, stance).map(|pose| pose.transform_point3(Vec3::ZERO))
        };
        assert_eq!(at(&holding), Some(Vec3::from_array(HELD_AT)));
        let age = 2.0f32;
        let chew = sine(age * 0.6);
        let eating = Stance {
            eating: true,
            age,
            ..holding
        };
        let chewed = Vec3::new(0.1, 1.4 - 0.09 * chew, -0.6 - (0.2 * chew + 0.2));
        assert_eq!(at(&eating), Some(chewed));
        let scared = Stance {
            scared: true,
            ..holding
        };
        assert_eq!(at(&scared), None, "a scared panda covers its face");
        let settling = Stance {
            sit: 1.0,
            ..Stance::default()
        };
        assert_eq!(at(&settling), None, "only while it sits");

        let atlas = atlas();
        let tag = parse(BAMBOO_PAW);
        let mut host = EntityWorld::default();
        let id = spawn_saved(&mut host, &tag).unwrap();
        let mut friend = friend_of(&host, id, tag);
        assert!(drawn(&host, &atlas, 1.0).1.is_empty(), "standing");
        goals(&mut host, id).sitting = true;
        for _ in 0..7 {
            settle(&mut host, id, 1);
            mirror(&host, &mut friend, id);
        }
        let (_, held) = drawn(&host, &atlas, 1.0);
        assert_eq!(held.len(), 1);
        let item = &held[0];
        assert_eq!(
            (item.id.as_str(), item.display, item.first_tint),
            ("minecraft:bamboo", HeldDisplay::Ground, None)
        );
        let paws = item.pose.transform_point3(Vec3::ZERO);
        assert!(
            close(paws, Vec3::new(0.6, 65.4, 1.399)),
            "in its paws before its chest: {paws}"
        );
        assert!(close(item.light, Vec3::new(0.5, 65.0625, 0.5)));
        let seen = drawn(&friend, &atlas, 1.0).1;
        assert_eq!(held_items(&seen), held_items(&held));

        goals(&mut host, id).scared = true;
        mirror(&host, &mut friend, id);
        assert!(drawn(&host, &atlas, 1.0).1.is_empty());
        assert!(drawn(&friend, &atlas, 1.0).1.is_empty());
        goals(&mut host, id).scared = false;
        let mob = host.family_mob_mut::<Panda>(id).unwrap();
        mob.state.held = Some(ItemStack::new("minecraft:cake", 1));
        mirror(&host, &mut friend, id);
        assert_eq!(drawn(&friend, &atlas, 1.0).1[0].id, "minecraft:cake");
        let mob = host.family_mob_mut::<Panda>(id).unwrap();
        mob.state.held = Some(ItemStack::new("minecraft:stick", 1));
        mirror(&host, &mut friend, id);
        assert_eq!(drawn(&host, &atlas, 1.0).1[0].id, "minecraft:stick");
        assert!(
            drawn(&friend, &atlas, 1.0).1.is_empty(),
            "friends draw no other item"
        );
        host.family_mob_mut::<Panda>(id).unwrap().state.held = None;
        assert!(drawn(&host, &atlas, 1.0).1.is_empty(), "an empty paw");
    }
}
