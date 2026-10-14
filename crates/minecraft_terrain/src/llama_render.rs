use crate::client_mobs::{ClientMobs, mth_cos};
use crate::horse_render::llama_texture;
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{DVec3, Mat4, Vec3};
use minecraftoss_entities::horse::{HorseKind, HorseState};
use minecraftoss_entities::world::{CowEntity, LlamaSpitEntity};
use std::f32::consts::{FRAC_PI_2, PI};

pub const SHADOW_RADIUS: f32 = 0.7;
pub const SPIT_TEXTURE: &str = "minecraft:entity/llama/llama_spit";
pub const TRADER_DECOR: &str = "minecraft:entity/equipment/llama_body/trader_llama";
pub const TRADER_BABY_DECOR: &str = "minecraft:entity/equipment/llama_body/trader_llama_baby";
pub const ADULT_DECOR_GROW: f32 = 0.5;
pub const BABY_DECOR_GROW: f32 = 0.2;
pub const SPIT_LIFT: f32 = 0.15;
pub const CARPET_COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Head,
    Body,
    RightChest,
    LeftChest,
    RightHindLeg,
    LeftHindLeg,
    RightFrontLeg,
    LeftFrontLeg,
}

type Placed = (Part, [f32; 3], [f32; 3], Cube);

const ADULT_LEG: Cube = Cube::new([-2.0, 0.0, -2.0], [4.0, 14.0, 4.0], [29.0, 29.0]);
const ADULT_HEAD: [f32; 3] = [0.0, 7.0, -6.0];
const ADULT: [Placed; 11] = [
    (
        Part::Head,
        ADULT_HEAD,
        [0.0; 3],
        Cube::new([-2.0, -14.0, -10.0], [4.0, 4.0, 9.0], [0.0, 0.0]),
    ),
    (
        Part::Head,
        ADULT_HEAD,
        [0.0; 3],
        Cube::new([-4.0, -16.0, -6.0], [8.0, 18.0, 6.0], [0.0, 14.0]),
    ),
    (
        Part::Head,
        ADULT_HEAD,
        [0.0; 3],
        Cube::new([-4.0, -19.0, -4.0], [3.0, 3.0, 2.0], [17.0, 0.0]),
    ),
    (
        Part::Head,
        ADULT_HEAD,
        [0.0; 3],
        Cube::new([1.0, -19.0, -4.0], [3.0, 3.0, 2.0], [17.0, 0.0]),
    ),
    (
        Part::Body,
        [0.0, 5.0, 2.0],
        [FRAC_PI_2, 0.0, 0.0],
        Cube::new([-6.0, -10.0, -7.0], [12.0, 18.0, 10.0], [29.0, 0.0]),
    ),
    (
        Part::RightChest,
        [-8.5, 3.0, 3.0],
        [0.0, FRAC_PI_2, 0.0],
        Cube::new([-3.0, 0.0, 0.0], [8.0, 8.0, 3.0], [45.0, 28.0]),
    ),
    (
        Part::LeftChest,
        [5.5, 3.0, 3.0],
        [0.0, FRAC_PI_2, 0.0],
        Cube::new([-3.0, 0.0, 0.0], [8.0, 8.0, 3.0], [45.0, 41.0]),
    ),
    (Part::RightHindLeg, [-3.5, 10.0, 6.0], [0.0; 3], ADULT_LEG),
    (Part::LeftHindLeg, [3.5, 10.0, 6.0], [0.0; 3], ADULT_LEG),
    (Part::RightFrontLeg, [-3.5, 10.0, -5.0], [0.0; 3], ADULT_LEG),
    (Part::LeftFrontLeg, [3.5, 10.0, -5.0], [0.0; 3], ADULT_LEG),
];

const BABY_HEAD: [f32; 3] = [0.0, 12.0, -4.0];
const BABY: [Placed; 11] = [
    (
        Part::Head,
        BABY_HEAD,
        [0.0; 3],
        Cube::new([-3.0, -9.0, -4.0], [6.0, 11.0, 4.0], [0.0, 0.0]),
    ),
    (
        Part::Head,
        BABY_HEAD,
        [0.0; 3],
        Cube::new([-1.5, -7.0, -7.0], [3.0, 3.0, 3.0], [0.0, 15.0]),
    ),
    (
        Part::Head,
        BABY_HEAD,
        [0.0; 3],
        Cube::new([0.5, -11.0, -3.0], [2.0, 2.0, 2.0], [20.0, 4.0]),
    ),
    (
        Part::Head,
        BABY_HEAD,
        [0.0; 3],
        Cube::new([-2.5, -11.0, -3.0], [2.0, 2.0, 2.0], [20.0, 0.0]),
    ),
    (
        Part::RightHindLeg,
        [-2.5, 16.5, 4.5],
        [0.0; 3],
        Cube::new([-1.4, -0.5, -1.5], [3.0, 8.0, 3.0], [0.0, 45.0]),
    ),
    (
        Part::LeftHindLeg,
        [2.5, 16.5, 4.5],
        [0.0; 3],
        Cube::new([-1.6, -0.5, -1.5], [3.0, 8.0, 3.0], [12.0, 45.0]),
    ),
    (
        Part::RightFrontLeg,
        [-2.5, 16.5, -3.5],
        [0.0; 3],
        Cube::new([-1.4, -0.5, -1.5], [3.0, 8.0, 3.0], [0.0, 34.0]),
    ),
    (
        Part::LeftFrontLeg,
        [2.5, 16.5, -3.5],
        [0.0; 3],
        Cube::new([-1.6, -0.5, -1.5], [3.0, 8.0, 3.0], [12.0, 34.0]),
    ),
    (
        Part::Body,
        [0.0, 14.0, 2.5],
        [0.0; 3],
        Cube::new([-4.0, -3.0, -8.5], [8.0, 6.0, 13.0], [0.0, 15.0]),
    ),
    (
        Part::RightChest,
        [-8.5, 4.0, 3.0],
        [0.0, FRAC_PI_2, 0.0],
        Cube::new([-3.0, 0.0, 0.0], [8.0, 8.0, 3.0], [45.0, 28.0]),
    ),
    (
        Part::LeftChest,
        [5.5, 4.0, 3.0],
        [0.0, FRAC_PI_2, 0.0],
        Cube::new([-3.0, 0.0, 0.0], [8.0, 8.0, 3.0], [45.0, 41.0]),
    ),
];

const SPIT_CUBES: [[f32; 3]; 7] = [
    [-4.0, 0.0, 0.0],
    [0.0, -4.0, 0.0],
    [0.0, 0.0, -4.0],
    [0.0, 0.0, 0.0],
    [2.0, 0.0, 0.0],
    [0.0, 2.0, 0.0],
    [0.0, 0.0, 2.0],
];

pub fn carpet_texture(item: &str) -> Option<String> {
    let color = item
        .strip_prefix("minecraft:")
        .unwrap_or(item)
        .strip_suffix("_carpet")?;
    CARPET_COLORS
        .contains(&color)
        .then(|| format!("minecraft:entity/equipment/llama_body/{color}"))
}

pub fn decor(horse: &HorseState, baby: bool) -> Option<String> {
    let carpet = horse.body_item.as_deref().and_then(carpet_texture);
    match carpet {
        Some(carpet) if !baby => Some(carpet),
        _ if horse.kind == HorseKind::TraderLlama => Some(
            if baby {
                TRADER_BABY_DECOR
            } else {
                TRADER_DECOR
            }
            .to_owned(),
        ),
        _ => None,
    }
}

pub fn textures() -> Vec<String> {
    let mut out = Vec::new();
    for variant in 0..4 {
        out.push(llama_texture(variant, false));
        out.push(llama_texture(variant, true));
    }
    for color in CARPET_COLORS {
        out.push(format!("minecraft:entity/equipment/llama_body/{color}"));
    }
    out.push(TRADER_DECOR.to_owned());
    out.push(TRADER_BABY_DECOR.to_owned());
    out.push(SPIT_TEXTURE.to_owned());
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LlamaPose {
    pub head: [f32; 3],
    pub legs: [f32; 4],
    pub chest: bool,
}

pub fn llama_pose(
    head_pitch: f32,
    head_yaw: f32,
    walk_position: f32,
    walk_speed: f32,
    chest: bool,
) -> LlamaPose {
    let swing = |phase: f32| mth_cos(walk_position * 0.6662 + phase) * 1.4 * walk_speed;
    LlamaPose {
        head: [head_pitch * 0.017_453_292, head_yaw * 0.017_453_292, 0.0],
        legs: [swing(0.0), swing(PI), swing(PI), swing(0.0)],
        chest,
    }
}

fn leg_index(part: Part) -> Option<usize> {
    match part {
        Part::RightHindLeg => Some(0),
        Part::LeftHindLeg => Some(1),
        Part::RightFrontLeg => Some(2),
        Part::LeftFrontLeg => Some(3),
        _ => None,
    }
}

fn grown(pose: Mat4, cube: &Cube, grow: f32) -> Mat4 {
    if grow == 0.0 {
        return pose;
    }
    let center = Vec3::from_array(cube.from) + Vec3::from_array(cube.size) / 2.0;
    let size = Vec3::from_array(cube.size);
    let scale = (size + Vec3::splat(2.0 * grow)) / size;
    pose * Mat4::from_translation(center / 16.0)
        * Mat4::from_scale(scale)
        * Mat4::from_translation(-center / 16.0)
}

pub fn append_model(
    mesh: &mut ChunkMesh,
    root: Mat4,
    skin: &Skin,
    baby: bool,
    pose: &LlamaPose,
    grow: f32,
) {
    let cubes: &[Placed] = if baby { &BABY } else { &ADULT };
    for (part, offset, rotation, cube) in cubes {
        let rotation = match part {
            Part::Head => pose.head,
            Part::RightChest | Part::LeftChest if !pose.chest => continue,
            _ => match leg_index(*part) {
                Some(i) => [pose.legs[i], 0.0, 0.0],
                None => *rotation,
            },
        };
        let matrix = root * part_pose(*offset, rotation);
        append(mesh, grown(matrix, cube, grow), skin, cube);
    }
}

pub fn model_root(feet: Mat4) -> Mat4 {
    feet * Mat4::from_translation(Vec3::new(0.0, 24.016 / 16.0, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

pub fn append_llamas<'a>(
    mesh: &mut ChunkMesh,
    llamas: impl IntoIterator<Item = &'a CowEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let mut marks = Vec::new();
    for entity in llamas {
        let Some(horse) = entity.horse.as_ref().filter(|h| h.kind.is_llama()) else {
            continue;
        };
        let baby = entity.cow.age.baby();
        let Ok(coat) = ResourceId::parse(&llama_texture(horse.variant, baby)) else {
            continue;
        };
        let Some(mob) = poses.pose(entity.id, partial) else {
            continue;
        };
        marks.push((mesh.vertices.len(), mob.overlay(0.0)));
        let position = mob.light_block();
        let skin = Skin {
            region: atlas.entity_region(&coat),
            texture_size: if baby { [64.0, 64.0] } else { [128.0, 64.0] },
            sky: light.get(position) as f32,
            block: light.get_block(position) as f32,
            alpha: 1.0,
            shaded: true,
        };
        let pose = llama_pose(
            mob.head_pitch,
            mob.head_yaw,
            mob.walk_position,
            mob.walk_speed,
            horse.chest && !baby,
        );
        let feet =
            Mat4::from_translation(mob.feet.as_vec3()) * Mat4::from_quat(mob.body_rotation(90.0));
        let root = model_root(feet);
        append_model(mesh, root, &skin, baby, &pose, 0.0);
        let Some(Ok(decor)) = decor(horse, baby).map(|d| ResourceId::parse(&d)) else {
            continue;
        };
        let decor_skin = Skin {
            region: atlas.entity_region(&decor),
            ..skin
        };
        let grow = if baby {
            BABY_DECOR_GROW
        } else {
            ADULT_DECOR_GROW
        };
        append_model(mesh, root, &decor_skin, baby, &pose, grow);
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

pub fn spit_pose(feet: DVec3, yaw: f32, pitch: f32) -> Mat4 {
    Mat4::from_translation(feet.as_vec3() + Vec3::new(0.0, SPIT_LIFT, 0.0))
        * Mat4::from_rotation_y((yaw - 90.0).to_radians())
        * Mat4::from_rotation_z(pitch.to_radians())
}

pub fn append_spit_model(mesh: &mut ChunkMesh, pose: Mat4, skin: &Skin) {
    for from in SPIT_CUBES {
        append(
            mesh,
            pose,
            skin,
            &Cube::new(from, [2.0, 2.0, 2.0], [0.0, 0.0]),
        );
    }
}

pub fn append_spits<'a>(
    mesh: &mut ChunkMesh,
    spits: impl IntoIterator<Item = &'a LlamaSpitEntity>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let partial = partial.clamp(0.0, 1.0);
    let Ok(texture) = ResourceId::parse(SPIT_TEXTURE) else {
        return;
    };
    let region = atlas.entity_region(&texture);
    for entity in spits.into_iter().filter(|e| e.spit.alive) {
        let spit = &entity.spit;
        let feet = entity
            .previous_position
            .lerp(spit.position, f64::from(partial));
        let yaw = spit.yaw_o + (spit.yaw - spit.yaw_o) * partial;
        let pitch = spit.pitch_o + (spit.pitch - spit.pitch_o) * partial;
        let block = (
            feet.x.floor() as i32,
            feet.y.floor() as i32,
            feet.z.floor() as i32,
        );
        let skin = Skin {
            region,
            texture_size: [64.0, 32.0],
            sky: light.get(block) as f32,
            block: light.get_block(block) as f32,
            alpha: 1.0,
            shaded: true,
        };
        append_spit_model(mesh, spit_pose(feet, yaw, pitch), &skin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_entities::world::EntityWorld;

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: [128.0, 64.0],
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    fn lowest(mesh: &ChunkMesh) -> f32 {
        mesh.vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MAX, f32::min)
    }

    fn highest(mesh: &ChunkMesh) -> f32 {
        mesh.vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max)
    }

    fn model(baby: bool, pose: &LlamaPose, grow: f32) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();
        append_model(
            &mut mesh,
            model_root(Mat4::IDENTITY),
            &skin(),
            baby,
            pose,
            grow,
        );
        mesh
    }

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let ids: Vec<ResourceId> = textures()
            .iter()
            .map(|t| ResourceId::parse(t).unwrap())
            .collect();
        for (i, id) in ids.iter().enumerate() {
            let path = root.join(format!("assets/minecraft/textures/{}.png", id.path));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let shade = 10 + i as u8 * 8;
            image::RgbaImage::from_pixel(64, 64, image::Rgba([shade, shade, shade, 255]))
                .save(path)
                .unwrap();
        }
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        crate::mesh::make_atlas(&packs, ids).unwrap()
    }

    fn drawn(text: &str, atlas: &Atlas) -> Vec<crate::mesh::Vertex> {
        let mut world = EntityWorld::default();
        let tag = minecraftoss_core::snbt::parse_compound(text).unwrap();
        crate::server_mobs::spawn_saved(&mut world, &tag).expect("the llama loads");
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&crate::client_mobs::server_mobs(&world));
        let light = SkyLight::from_levels((-2, 62, -2), (5, 5, 5), vec![15; 125], vec![0; 125]);
        let mut mesh = ChunkMesh::default();
        append_llamas(&mut mesh, world.cows().iter(), &poses, atlas, &light, 0.0);
        mesh.vertices
    }

    fn inside(uv: [f32; 2], region: [f32; 4]) -> bool {
        uv[0] >= region[0] && uv[0] <= region[2] && uv[1] >= region[1] && uv[1] <= region[3]
    }

    #[test]
    fn a_llama_has_nine_cubes_and_two_more_with_a_chest_and_stands_on_the_ground() {
        for baby in [false, true] {
            let plain = model(baby, &llama_pose(0.0, 0.0, 0.0, 0.0, false), 0.0);
            assert_eq!(plain.faces, 9 * 6, "baby {baby}");
            assert!(
                lowest(&plain).abs() < 0.01,
                "baby {baby}: {}",
                lowest(&plain)
            );
            let chested = model(baby, &llama_pose(0.0, 0.0, 0.0, 0.0, true), 0.0);
            assert_eq!(chested.faces, 11 * 6);
        }
        let adult = model(false, &llama_pose(0.0, 0.0, 0.0, 0.0, false), 0.0);
        assert!(
            (highest(&adult) - (1.501 + 12.0 / 16.0)).abs() < 0.01,
            "the ears top the neck: {}",
            highest(&adult)
        );
    }

    #[test]
    fn the_legs_swing_in_pairs_and_the_head_follows_the_look() {
        let pose = llama_pose(-20.0, 30.0, 1.0, 0.8, false);
        assert!((pose.legs[0] - mth_cos(0.6662) * 1.4 * 0.8).abs() < 1.0e-6);
        assert_eq!(pose.legs[0], pose.legs[3]);
        assert_eq!(pose.legs[1], pose.legs[2]);
        assert!((pose.legs[0] + pose.legs[1]).abs() < 1.0e-5);
        assert!((pose.head[0] + 20.0_f32.to_radians()).abs() < 1.0e-5);
        assert!((pose.head[1] - 30.0_f32.to_radians()).abs() < 1.0e-5);
    }

    #[test]
    fn the_decor_is_an_adult_s_carpet_or_the_trader_s_blanket_and_grows_half_a_pixel() {
        let mut llama = HorseState::new(HorseKind::Llama);
        assert_eq!(decor(&llama, false), None);
        llama.body_item = Some("minecraft:light_blue_carpet".to_owned());
        assert_eq!(
            decor(&llama, false).as_deref(),
            Some("minecraft:entity/equipment/llama_body/light_blue")
        );
        assert_eq!(decor(&llama, true), None);
        llama.body_item = Some("minecraft:moss_carpet".to_owned());
        assert_eq!(decor(&llama, false), None);
        let mut trader = HorseState::new(HorseKind::TraderLlama);
        assert_eq!(decor(&trader, false).as_deref(), Some(TRADER_DECOR));
        assert_eq!(decor(&trader, true).as_deref(), Some(TRADER_BABY_DECOR));
        trader.body_item = Some("minecraft:red_carpet".to_owned());
        assert_eq!(
            decor(&trader, false).as_deref(),
            Some("minecraft:entity/equipment/llama_body/red")
        );
        assert_eq!(decor(&trader, true).as_deref(), Some(TRADER_BABY_DECOR));
        let pose = llama_pose(0.0, 0.0, 0.0, 0.0, false);
        let plain = model(false, &pose, 0.0);
        let decorated = model(false, &pose, ADULT_DECOR_GROW);
        assert!((highest(&decorated) - highest(&plain) - 0.5 / 16.0).abs() < 1.0e-3);
        assert_eq!(
            plain.vertices.iter().map(|v| v.uv).collect::<Vec<_>>(),
            decorated.vertices.iter().map(|v| v.uv).collect::<Vec<_>>()
        );
        assert_eq!(textures().len(), 4 * 2 + 16 + 3);
    }

    #[test]
    fn a_saved_llama_draws_its_coat_chests_and_carpet_and_a_baby_trader_its_blanket() {
        let atlas = atlas();
        let region = |id: &str| atlas.entity_region(&ResourceId::parse(id).unwrap());
        let tame = drawn(
            "{id:\"minecraft:llama\",Pos:[0.5d,64.0d,0.5d],Variant:1,Strength:3,Tame:1b,ChestedHorse:1b,equipment:{body:{id:\"minecraft:white_carpet\",count:1}}}",
            &atlas,
        );
        assert_eq!(tame.len(), 2 * 11 * 6 * 4);
        let (coat, carpet) = tame.split_at(11 * 6 * 4);
        let white = region("minecraft:entity/llama/llama_white");
        assert!(coat.iter().all(|v| inside(v.uv, white)));
        let white_carpet = region("minecraft:entity/equipment/llama_body/white");
        assert!(carpet.iter().all(|v| inside(v.uv, white_carpet)));
        let foal = drawn(
            "{id:\"minecraft:trader_llama\",Pos:[0.5d,64.0d,0.5d],Variant:3,Age:-24000,ChestedHorse:1b}",
            &atlas,
        );
        assert_eq!(foal.len(), 2 * 9 * 6 * 4);
        let (coat, blanket) = foal.split_at(9 * 6 * 4);
        let gray = region("minecraft:entity/llama/llama_gray_baby");
        assert!(coat.iter().all(|v| inside(v.uv, gray)));
        let trader = region(TRADER_BABY_DECOR);
        assert!(blanket.iter().all(|v| inside(v.uv, trader)));
        let horse = drawn("{id:\"minecraft:horse\",Pos:[0.5d,64.0d,0.5d]}", &atlas);
        assert!(horse.is_empty());
    }

    #[test]
    fn spit_is_seven_little_cubes_turned_along_its_flight() {
        let mut mesh = ChunkMesh::default();
        let skin = Skin {
            texture_size: [64.0, 32.0],
            ..skin()
        };
        append_spit_model(
            &mut mesh,
            spit_pose(DVec3::new(0.5, 64.0, 0.5), 90.0, 0.0),
            &skin,
        );
        assert_eq!(mesh.faces, 7 * 6);
        let xs: Vec<f32> = mesh.vertices.iter().map(|v| v.position[0]).collect();
        let min = xs.iter().copied().fold(f32::MAX, f32::min);
        let max = xs.iter().copied().fold(f32::MIN, f32::max);
        assert!(
            (min - 0.25).abs() < 1.0e-4 && (max - 0.75).abs() < 1.0e-4,
            "{min} {max}"
        );
        assert!((lowest(&mesh) - (64.15 - 0.25)).abs() < 1.0e-4);
        let mut turned = ChunkMesh::default();
        append_spit_model(
            &mut turned,
            spit_pose(DVec3::new(0.5, 64.0, 0.5), 0.0, 0.0),
            &skin,
        );
        let zs: Vec<f32> = turned.vertices.iter().map(|v| v.position[2]).collect();
        let span = zs.iter().copied().fold(f32::MIN, f32::max)
            - zs.iter().copied().fold(f32::MAX, f32::min);
        assert!((span - 0.5).abs() < 1.0e-4, "{span}");
    }

    #[test]
    fn flying_spit_is_drawn_between_its_last_two_places() {
        let mut world = EntityWorld::default();
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:llama_spit\",Pos:[0.5d,64.0d,0.5d],Motion:[0.0d,0.0d,1.0d],Rotation:[0.0f,0.0f]}").unwrap();
        let id = crate::server_mobs::spawn_saved(&mut world, &tag).expect("the spit loads");
        {
            let e = world.llama_spit_mut(id).unwrap();
            e.spit.position = DVec3::new(0.5, 64.0, 1.5);
        }
        let atlas = atlas();
        let light = SkyLight::from_levels((-2, 62, -2), (5, 5, 5), vec![15; 125], vec![0; 125]);
        let mut mesh = ChunkMesh::default();
        append_spits(&mut mesh, world.llama_spits(), &atlas, &light, 0.5);
        assert_eq!(mesh.faces, 7 * 6);
        let zs: Vec<f32> = mesh.vertices.iter().map(|v| v.position[2]).collect();
        let middle = (zs.iter().copied().fold(f32::MIN, f32::max)
            + zs.iter().copied().fold(f32::MAX, f32::min))
            / 2.0;
        assert!((middle - 1.0).abs() < 1.0e-4, "{middle}");
        let spit = region_of(&atlas);
        assert!(mesh.vertices.iter().all(|v| inside(v.uv, spit)));
    }

    fn region_of(atlas: &Atlas) -> [f32; 4] {
        atlas.entity_region(&ResourceId::parse(SPIT_TEXTURE).unwrap())
    }
}
