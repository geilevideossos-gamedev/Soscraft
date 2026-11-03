use super::*;
use crate::client_mobs::server_mobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::server_mobs::spawn_saved;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::Hurt;
use minecraftoss_entities::world::{DamageSourceKind, EntityWorld};

fn atlas() -> Atlas {
    let temp = crate::test_dir::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(
        root.join("pack.mcmeta"),
        r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
    )
    .unwrap();
    let skins = root.join("assets/minecraft/textures/entity/squid");
    std::fs::create_dir_all(&skins).unwrap();
    for (index, texture) in TEXTURES.into_iter().enumerate() {
        let name = texture.rsplit('/').next().unwrap();
        let width = if index % 2 == 0 { 64 } else { 32 };
        let shade = 40 + 50 * index as u8;
        image::RgbaImage::from_pixel(width, 32, image::Rgba([shade, 60, 120, 255]))
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

fn light(block: u8) -> SkyLight {
    SkyLight::from_levels(
        (-8, 56, -8),
        (17, 17, 17),
        vec![15; 4913],
        vec![block; 4913],
    )
}

fn parse(text: &str) -> Tag {
    minecraftoss_core::snbt::parse_compound(text).unwrap()
}

fn region(atlas: &Atlas, index: usize) -> [f32; 4] {
    atlas.entity_region(&ResourceId::parse(TEXTURES[index]).unwrap())
}

fn drawn_with(
    world: &EntityWorld,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) -> ChunkMesh {
    let mut models = ChunkMesh::default();
    let mut culled = ChunkMesh::default();
    let mut translucent = ChunkMesh::default();
    let mut held = Vec::new();
    Squid::draw(
        &mut FamilyDraw {
            models: &mut models,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut held,
        },
        world.family::<Squid>(),
        poses,
        atlas,
        light,
        partial,
    );
    assert!(models.vertices.is_empty(), "entityCutout culls back faces");
    assert!(translucent.vertices.is_empty() && held.is_empty());
    culled
}

fn drawn(world: &EntityWorld, atlas: &Atlas, light: &SkyLight, partial: f32) -> ChunkMesh {
    let mut poses = ClientMobs::default();
    poses.spawn_missing(&server_mobs(world));
    drawn_with(world, &poses, atlas, light, partial)
}

fn span(mesh: &ChunkMesh, axis: usize) -> (f32, f32) {
    mesh.vertices
        .iter()
        .map(|v| v.position[axis])
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
}

fn inside(region: [f32; 4], mesh: &ChunkMesh) -> bool {
    mesh.vertices.iter().all(|v| {
        (region[0]..=region[2]).contains(&v.uv[0]) && (region[1]..=region[3]).contains(&v.uv[1])
    })
}

fn positions(mesh: &ChunkMesh) -> Vec<[f32; 3]> {
    mesh.vertices.iter().map(|v| v.position).collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn an_adult_squid_has_a_body_and_eight_tentacles_on_64_by_32_and_a_baby_on_32_by_32() {
    assert_eq!(
        (ADULT.texture_size, BABY.texture_size),
        ([64.0, 32.0], [32.0, 32.0])
    );
    assert_eq!(
        (ADULT.body.from, ADULT.body.size, ADULT.body.uv),
        ([-6.0, -8.0, -6.0], [12.0, 16.0, 12.0], [0.0, 0.0])
    );
    assert_eq!(
        (ADULT.tentacle.from, ADULT.tentacle.size, ADULT.tentacle.uv),
        ([-1.0, 0.0, -1.0], [2.0, 18.0, 2.0], [48.0, 0.0])
    );
    assert_eq!(
        (BABY.body.from, BABY.body.size, BABY.body_offset),
        ([-4.0, -5.0, -4.0], [8.0, 10.0, 8.0], [0.0, 13.0, 0.0])
    );
    assert_eq!(
        (BABY.tentacle.from, BABY.tentacle.size, BABY.tentacle.uv),
        ([-1.0, -0.5, -1.0], [2.0, 6.0, 2.0], [0.0, 18.0])
    );
    assert_eq!((ADULT.body_growth, BABY.body_growth), (0.02, 0.0));
    assert_eq!(tentacle_offset(&ADULT, 0), [5.0, 15.0, 0.0]);
    let third = tentacle_offset(&ADULT, 2);
    assert!(third[0].abs() < 1e-6 && third[1] == 15.0 && third[2] == 5.0);
    let fifth = tentacle_offset(&BABY, 4);
    assert!(fifth[0] == -3.0 && fifth[1] == 18.5 && fifth[2].abs() < 1e-6);
    assert_eq!(
        (tentacle_turn(0), tentacle_turn(2), tentacle_turn(4)),
        (FRAC_PI_2 as f32, 0.0, -FRAC_PI_2 as f32)
    );
    let (_, arms) = part_poses(&ADULT, 0.6);
    for (index, arm) in arms.iter().enumerate() {
        let swung = arm.transform_vector3(Vec3::Y);
        assert!(
            (swung.y - 0.6_f32.cos()).abs() < 1e-5,
            "tentacle {index} swings out by the tentacle angle"
        );
    }

    let atlas = atlas();
    let mut world = EntityWorld::default();
    let adult = spawn_saved(
        &mut world,
        &parse("{id:\"minecraft:squid\",Pos:[0.5d,64.0d,0.5d]}"),
    )
    .unwrap();
    let mesh = drawn(&world, &atlas, &light(0), 0.0);
    assert_eq!(mesh.faces, 54, "a body and eight tentacles");
    let (low, high) = span(&mesh, 1);
    assert!(
        close(low, 64.0 + 0.801 - 33.0 / 16.0),
        "the tentacles hang below its feet: {low}"
    );
    assert!(
        close(high, 64.0 + 0.801 + 0.02 / 16.0),
        "the grown body tops its box: {high}"
    );
    let (west, east) = span(&mesh, 0);
    assert!(close(east - west, 12.04 / 16.0), "{west} {east}");
    assert!(inside(region(&atlas, 0), &mesh));

    world.family_core_mut(adult).unwrap().1.removed = true;
    spawn_saved(
        &mut world,
        &parse("{id:\"minecraft:squid\",Pos:[0.5d,64.0d,0.5d],Age:-24000}"),
    )
    .unwrap();
    let baby = drawn(&world, &atlas, &light(0), 0.0);
    assert_eq!(
        baby.faces, 54,
        "its own model, the adult that left not drawn"
    );
    let (low, high) = span(&baby, 1);
    assert!(close(low, 64.0 + 1.151 - 24.0 / 16.0), "{low}");
    assert!(close(high, 64.0 + 1.151 - 8.0 / 16.0), "{high}");
    let (west, east) = span(&baby, 0);
    assert!(close(east - west, 8.0 / 16.0), "{west} {east}");
    assert!(inside(region(&atlas, 1), &baby));
}

#[test]
fn every_squid_picks_its_texture_by_kind_and_age() {
    assert_eq!(texture(false, false), "minecraft:entity/squid/squid");
    assert_eq!(texture(false, true), "minecraft:entity/squid/squid_baby");
    assert_eq!(texture(true, false), "minecraft:entity/squid/glow_squid");
    assert_eq!(
        texture(true, true),
        "minecraft:entity/squid/glow_squid_baby"
    );
    assert_eq!(Squid::TEXTURES, &TEXTURES[..]);
    assert_eq!(Squid::SHADOW_RADIUS, 0.7);
    let atlas = atlas();
    for (kind, age, index) in [
        ("squid", 0, 0),
        ("squid", -24000, 1),
        ("glow_squid", 0, 2),
        ("glow_squid", -24000, 3),
    ] {
        let mut world = EntityWorld::default();
        let tag = format!("{{id:\"minecraft:{kind}\",Pos:[0.5d,64.0d,0.5d],Age:{age}}}");
        spawn_saved(&mut world, &parse(&tag)).unwrap();
        let mesh = drawn(&world, &atlas, &light(0), 0.0);
        assert_eq!(mesh.faces, 54, "{kind} {age}");
        for (other, texture) in TEXTURES.into_iter().enumerate() {
            assert_eq!(
                inside(region(&atlas, other), &mesh),
                other == index,
                "{kind} {age}: {texture}"
            );
        }
    }
}

#[test]
fn a_dying_squid_flashes_red_but_does_not_tip_over() {
    let atlas = atlas();
    let mut world = EntityWorld::default();
    let id = spawn_saved(
        &mut world,
        &parse("{id:\"minecraft:squid\",Pos:[0.5d,64.0d,0.5d],Rotation:[40.0f,0.0f]}"),
    )
    .unwrap();
    let mob = world.family_mob_mut::<Squid>(id).unwrap();
    mob.state.x_body_rot = -60.0;
    mob.state.x_body_rot_o = -60.0;
    let mut poses = ClientMobs::default();
    poses.spawn_missing(&server_mobs(&world));
    let living = drawn_with(&world, &poses, &atlas, &light(0), 0.5);
    assert!(living.vertices.iter().all(|v| v.color[3] > 0.0));
    let hurt = Hurt::new(20.0, "minecraft:generic", DamageSourceKind::Generic);
    world.hurt_family(id, &hurt);
    assert!(world.damage_state(id).is_some_and(|damage| damage.dead));
    poses.receive(server_mobs(&world));
    for _ in 0..12 {
        poses.tick();
    }
    let pose = poses.pose(id, 0.5).unwrap();
    assert!(pose.red_overlay && pose.death_time > 10.0, "{pose:?}");
    let dying = drawn_with(&world, &poses, &atlas, &light(0), 0.5);
    assert_eq!(
        positions(&dying),
        positions(&living),
        "a squid keeps its pose while it dies"
    );
    assert!(
        dying.vertices.iter().all(|v| v.color[3] < 0.0),
        "it flashes red"
    );
}

#[test]
fn a_glow_squid_is_full_bright_and_dims_in_its_last_ten_dark_ticks() {
    assert_eq!((glow_light(0), glow_light(-3)), (15, 15));
    assert_eq!((glow_light(1), glow_light(5)), (13, 7));
    assert_eq!((glow_light(10), glow_light(11), glow_light(100)), (0, 0, 0));
    let levels: Vec<i32> = (0..=10).map(glow_light).collect();
    assert!(
        levels.windows(2).all(|pair| pair[1] < pair[0]),
        "{levels:?}"
    );
    assert_eq!(block_light(true, 0, 3.0), 15.0);
    assert_eq!(
        block_light(true, 100, 3.0),
        3.0,
        "dark, it is lit like the water"
    );
    assert_eq!(block_light(true, 5, 3.0), 7.0);
    assert_eq!(block_light(true, 5, 9.0), 9.0, "never darker than its cell");
    assert_eq!(block_light(false, 0, 3.0), 3.0, "a squid does not glow");

    let atlas = atlas();
    let mut world = EntityWorld::default();
    let glow = spawn_saved(
        &mut world,
        &parse("{id:\"minecraft:glow_squid\",Pos:[0.5d,64.0d,0.5d]}"),
    )
    .unwrap();
    let lit = |world: &EntityWorld| {
        let mesh = drawn(world, &atlas, &light(2), 0.0);
        assert!(mesh.vertices.iter().all(|v| v.sky_light == 15.0));
        let blocks: Vec<f32> = mesh.vertices.iter().map(|v| v.block_light).collect();
        assert!(blocks.windows(2).all(|pair| pair[0] == pair[1]));
        blocks[0]
    };
    assert_eq!(lit(&world), 15.0);
    for (dark, expected) in [(100, 2.0), (10, 2.0), (5, 7.0), (1, 13.0), (0, 15.0)] {
        world
            .family_mob_mut::<Squid>(glow)
            .unwrap()
            .state
            .dark_ticks = dark;
        assert_eq!(lit(&world), expected, "{dark} dark ticks");
    }
    world.family_core_mut(glow).unwrap().1.removed = true;
    spawn_saved(
        &mut world,
        &parse("{id:\"minecraft:squid\",Pos:[0.5d,64.0d,0.5d]}"),
    )
    .unwrap();
    assert_eq!(lit(&world), 2.0);
}
