use anyhow::Result;
use glam::{Mat4, Vec3};

use crate::armor_render::{ArmorRig, append_armor_on_rig};
use crate::body_gear::BodyGear;
use crate::elytra_render::{WingAngles, append_elytra, wings_texture};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, HeldDisplay, HeldItem, ItemVisuals};
use crate::pack::PackStack;
use crate::worn_head::append_worn_head;

pub const HAND_DISPLAYS: [HeldDisplay; 2] = [HeldDisplay::RightHand, HeldDisplay::LeftHand];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyRig {
    pub hands: [Option<Mat4>; 2],
    pub armor: ArmorRig,
    pub light: Vec3,
    pub wings: Option<WingAngles>,
    pub head_swing: f32,
}

pub fn held_items(rig: &BodyRig, gear: &BodyGear) -> Vec<(HeldItem, bool)> {
    [&gear.main, &gear.off]
        .into_iter()
        .zip(rig.hands)
        .zip(HAND_DISPLAYS)
        .filter_map(|((item, hand), display)| {
            let (item, pose) = (item.as_ref()?, hand?);
            let held = HeldItem {
                pose,
                light: rig.light,
                id: crate::item_tint::key(&item.id, item.potion.or(item.dye)),
                display,
                first_tint: None,
            };
            Some((held, item.glint))
        })
        .collect()
}

pub fn append_body_gear(
    mesh: &mut ChunkMesh,
    items: &mut ItemVisuals,
    rig: &BodyRig,
    gear: &BodyGear,
    packs: &PackStack,
    atlas: &Atlas,
    light: &SkyLight,
) -> Result<()> {
    let cell = rig.light.floor().as_ivec3();
    let (sky, block) = (
        f32::from(light.get((cell.x, cell.y, cell.z))),
        f32::from(light.get_block((cell.x, cell.y, cell.z))),
    );
    append_armor_on_rig(mesh, atlas, &rig.armor, &gear.armor, sky, block);
    append_worn_head(
        mesh,
        items,
        rig.armor.head,
        gear.armor[0].as_ref(),
        rig.head_swing,
        rig.light,
        packs,
        atlas,
        light,
        sky,
        block,
    )?;
    let elytra = gear.armor[1]
        .as_ref()
        .filter(|item| wings_texture(&item.id).is_some());
    if let (Some(item), Some(body)) = (elytra, rig.armor.body) {
        let wings = rig.wings.unwrap_or(WingAngles::REST);
        append_elytra(mesh, atlas, body.upper, wings, item.glint, sky, block);
    }
    for (held, glint) in held_items(rig, gear) {
        let start = mesh.vertices.len() as u32;
        items.append_held_items(mesh, std::slice::from_ref(&held), packs, atlas, light)?;
        let end = mesh.vertices.len() as u32;
        if glint && end > start {
            mesh.glint.push(start..end);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_gear::GearItem;
    use crate::pack::ResourceId;
    use crate::scene::HandcraftedScene;

    fn item(id: &str, glint: bool) -> Option<GearItem> {
        Some(GearItem {
            id: id.to_owned(),
            glint,
            dye: None,
            potion: None,
        })
    }

    fn rod_pack(root: &std::path::Path) -> PackStack {
        for (name, contents) in [
            (
                "pack.mcmeta",
                r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
            ),
            (
                "assets/test/items/rod.json",
                r#"{"model":{"type":"minecraft:model","model":"test:item/rod"}}"#,
            ),
            (
                "assets/test/models/item/rod.json",
                r#"{"parent":"test:item/generated","textures":{"layer0":"test:item/rod"}}"#,
            ),
            (
                "assets/test/models/item/generated.json",
                r#"{"parent":"builtin/generated","display":{"thirdperson_righthand":{"translation":[4,0,0]}}}"#,
            ),
        ] {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let sprite = root.join("assets/test/textures/item/rod.png");
        std::fs::create_dir_all(sprite.parent().unwrap()).unwrap();
        let mut image = image::RgbaImage::new(16, 16);
        image.put_pixel(8, 8, image::Rgba([200, 150, 100, 255]));
        image.save(sprite).unwrap();
        PackStack::open(vec![root.into()]).unwrap()
    }

    #[test]
    fn the_main_hand_holds_its_item_in_the_right_hand_and_the_offhand_in_the_left() {
        let right = Mat4::from_translation(Vec3::X);
        let left = Mat4::from_translation(Vec3::NEG_X);
        let rig = BodyRig {
            hands: [Some(right), Some(left)],
            light: Vec3::new(0.0, 64.0, 0.0),
            ..BodyRig::default()
        };
        let gear = BodyGear {
            main: item("minecraft:diamond_sword", true),
            off: item("minecraft:torch", false),
            ..BodyGear::default()
        };
        let held = held_items(&rig, &gear);
        assert_eq!(held.len(), 2);
        assert_eq!(
            (
                held[0].0.id.as_str(),
                held[0].0.display,
                held[0].0.pose,
                held[0].1
            ),
            (
                "minecraft:diamond_sword",
                HeldDisplay::RightHand,
                right,
                true
            )
        );
        assert_eq!(
            (
                held[1].0.id.as_str(),
                held[1].0.display,
                held[1].0.pose,
                held[1].1
            ),
            ("minecraft:torch", HeldDisplay::LeftHand, left, false)
        );
        assert!(held.iter().all(|(item, _)| item.light == rig.light));
        let one_handed = BodyRig {
            hands: [None, Some(left)],
            ..rig
        };
        let held = held_items(&one_handed, &gear);
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].0.display, HeldDisplay::LeftHand);
        assert!(held_items(&rig, &BodyGear::default()).is_empty());
    }

    #[test]
    fn held_items_take_their_hand_display_and_only_enchanted_ones_glint() {
        let temp = crate::test_dir::tempdir().unwrap();
        let packs = rod_pack(temp.path());
        let atlas =
            crate::mesh::make_atlas(&packs, vec![ResourceId::parse("test:item/rod").unwrap()])
                .unwrap();
        let light = SkyLight::build(&HandcraftedScene::new());
        let rig = BodyRig {
            hands: [Some(Mat4::IDENTITY), Some(Mat4::IDENTITY)],
            light: Vec3::ZERO,
            ..BodyRig::default()
        };
        let gear = BodyGear {
            main: item("test:rod", false),
            off: item("test:rod", true),
            ..BodyGear::default()
        };
        let mut mesh = ChunkMesh::default();
        let mut items = ItemVisuals::default();
        append_body_gear(&mut mesh, &mut items, &rig, &gear, &packs, &atlas, &light).unwrap();
        let half = mesh.vertices.len() / 2;
        assert!(half > 0);
        assert_eq!(mesh.glint.len(), 1);
        assert_eq!(mesh.glint[0], half as u32..mesh.vertices.len() as u32);
        let centre = |range: std::ops::Range<usize>| {
            mesh.vertices[range.clone()]
                .iter()
                .map(|vertex| Vec3::from_array(vertex.position))
                .sum::<Vec3>()
                / range.len() as f32
        };
        let (right, left) = (centre(0..half), centre(half..mesh.vertices.len()));
        assert!(
            (right - left).abs_diff_eq(Vec3::new(0.5, 0.0, 0.0), 1e-5),
            "{right} {left}"
        );
    }

    #[test]
    fn a_held_trident_is_the_3d_model_and_on_the_ground_it_stays_flat() {
        let temp = crate::test_dir::tempdir().unwrap();
        let definition = r#"{"model":{"type":"minecraft:select","property":"minecraft:display_context","cases":[{"when":["gui","ground","fixed"],"model":{"type":"minecraft:model","model":"test:item/rod"}}],"fallback":{"type":"minecraft:special","base":"test:item/rod","model":{"type":"minecraft:trident"},"transformation":{"scale":[1.0,-1.0,-1.0]}}}}"#;
        let items = temp.path().join("assets/test/items");
        std::fs::create_dir_all(&items).unwrap();
        std::fs::write(items.join("spear.json"), definition).unwrap();
        let sheet = temp
            .path()
            .join("assets/minecraft/textures/entity/trident/trident.png");
        std::fs::create_dir_all(sheet.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(32, 32, image::Rgba([90, 140, 130, 255]))
            .save(sheet)
            .unwrap();
        let packs = rod_pack(temp.path());
        let atlas = crate::mesh::make_atlas(
            &packs,
            vec![
                ResourceId::parse("test:item/rod").unwrap(),
                ResourceId::parse(crate::trident::TEXTURE).unwrap(),
            ],
        )
        .unwrap();
        let light = SkyLight::build(&HandcraftedScene::new());
        let held = |display| {
            let mut mesh = ChunkMesh::default();
            let mut items = ItemVisuals::default();
            let item = HeldItem {
                pose: Mat4::IDENTITY,
                light: Vec3::ZERO,
                id: "test:spear".to_owned(),
                display,
                first_tint: None,
            };
            items
                .append_held_items(&mut mesh, &[item], &packs, &atlas, &light)
                .unwrap();
            mesh.vertices.len()
        };
        assert_eq!(held(HeldDisplay::RightHand), 5 * 6 * 4);
        assert_eq!(held(HeldDisplay::LeftHand), 5 * 6 * 4);
        let flat = held(HeldDisplay::Ground);
        assert!(flat > 0 && flat != 5 * 6 * 4, "{flat}");
        assert_eq!(held(HeldDisplay::Fixed), flat);
    }

    #[test]
    fn worn_armour_is_drawn_with_the_body_and_lit_where_it_is() {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let texture = root.join("assets/minecraft/textures/entity/equipment/humanoid/diamond.png");
        std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(64, 32, image::Rgba([90, 220, 220, 255]))
            .save(texture)
            .unwrap();
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let id = ResourceId::parse("minecraft:entity/equipment/humanoid/diamond").unwrap();
        let atlas = crate::mesh::make_atlas(&packs, vec![id]).unwrap();
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![9], vec![3]);
        let rig = BodyRig {
            armor: crate::armor_render::ArmorRig {
                head: Some(Mat4::from_translation(Vec3::new(0.5, 65.5, 0.5))),
                ..Default::default()
            },
            light: Vec3::new(0.5, 64.5, 0.5),
            ..BodyRig::default()
        };
        let gear = BodyGear {
            armor: [item("minecraft:diamond_helmet", true), None, None, None],
            ..BodyGear::default()
        };
        let mut mesh = ChunkMesh::default();
        let mut items = ItemVisuals::default();
        append_body_gear(&mut mesh, &mut items, &rig, &gear, &packs, &atlas, &light).unwrap();
        assert_eq!(mesh.faces, 12);
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.sky_light, vertex.block_light) == (9.0, 3.0))
        );
        assert!(mesh.glint.is_empty());
        assert_eq!(mesh.armor_glint.len(), 1);
        assert_eq!(mesh.armor_glint[0], 0..mesh.vertices.len() as u32);
    }

    #[test]
    fn a_worn_elytra_is_drawn_on_the_back_and_shimmers_when_enchanted() {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let texture = root.join("assets/minecraft/textures/entity/equipment/wings/elytra.png");
        std::fs::create_dir_all(texture.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(64, 32, image::Rgba([150, 150, 160, 255]))
            .save(texture)
            .unwrap();
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let id = ResourceId::parse(crate::elytra_render::WINGS_TEXTURE).unwrap();
        let atlas = crate::mesh::make_atlas(&packs, vec![id]).unwrap();
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![12], vec![5]);
        let back = Mat4::from_translation(Vec3::new(0.5, 65.5, 0.5))
            * Mat4::from_scale(Vec3::splat(1.0 / 16.0));
        let rig = BodyRig {
            armor: crate::armor_render::ArmorRig {
                body: Some(crate::armor_render::SplitPart::whole(back)),
                ..Default::default()
            },
            light: Vec3::new(0.5, 64.5, 0.5),
            ..BodyRig::default()
        };
        let wearing = |slot: usize, glint: bool| {
            let mut gear = BodyGear::default();
            gear.armor[slot] = item("minecraft:elytra", glint);
            gear
        };
        let draw = |rig: &BodyRig, gear: &BodyGear| {
            let mut mesh = ChunkMesh::default();
            let mut items = ItemVisuals::default();
            append_body_gear(&mut mesh, &mut items, rig, gear, &packs, &atlas, &light).unwrap();
            mesh
        };
        let mesh = draw(&rig, &wearing(1, true));
        assert_eq!(mesh.faces, 12);
        assert_eq!(mesh.armor_glint, vec![0..mesh.vertices.len() as u32]);
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.sky_light, vertex.block_light) == (12.0, 5.0))
        );
        let mut alone = ChunkMesh::default();
        append_elytra(&mut alone, &atlas, back, WingAngles::REST, true, 12.0, 5.0);
        assert_eq!(
            alone.vertices, mesh.vertices,
            "folded at rest without wing angles"
        );
        let spread = BodyRig {
            wings: Some(WingAngles::SPREAD),
            ..rig
        };
        let open = draw(&spread, &wearing(1, false));
        assert_eq!(open.faces, 12);
        assert_ne!(open.vertices, mesh.vertices);
        assert!(open.armor_glint.is_empty());
        let no_back = BodyRig {
            armor: Default::default(),
            ..rig
        };
        assert_eq!(draw(&no_back, &wearing(1, true)).faces, 0);
        assert_eq!(draw(&rig, &wearing(0, true)).faces, 0);
    }

    #[test]
    fn a_held_potion_keys_its_colour() {
        let rig = BodyRig {
            hands: [Some(Mat4::IDENTITY), Some(Mat4::IDENTITY)],
            ..BodyRig::default()
        };
        let gear = BodyGear {
            main: Some(GearItem {
                id: "minecraft:potion".to_owned(),
                glint: false,
                dye: None,
                potion: Some(0x87A363),
            }),
            off: item("minecraft:potion", false),
            ..BodyGear::default()
        };
        let held = held_items(&rig, &gear);
        assert_eq!(held[0].0.id, "minecraft:potion~87a363");
        assert_eq!(held[0].0.first_tint, None);
        assert_eq!(held[1].0.id, "minecraft:potion");
    }

    #[test]
    fn a_held_dyed_leather_piece_keys_its_dye() {
        let rig = BodyRig {
            hands: [Some(Mat4::IDENTITY), Some(Mat4::IDENTITY)],
            ..BodyRig::default()
        };
        let gear = BodyGear {
            main: Some(GearItem {
                id: "minecraft:leather_horse_armor".to_owned(),
                glint: false,
                dye: Some(0xB02E26),
                potion: None,
            }),
            off: item("minecraft:leather_horse_armor", false),
            ..BodyGear::default()
        };
        let held = held_items(&rig, &gear);
        assert_eq!(held[0].0.id, "minecraft:leather_horse_armor~b02e26");
        assert_eq!(held[1].0.id, "minecraft:leather_horse_armor");
    }

    #[test]
    fn worn_heads_are_drawn_with_the_body_and_lit_where_it_is() {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        crate::worn_head::tests::write_pack(root, &[], &[("entity/zombie/zombie", 64, 64)]);
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let id = ResourceId::parse("minecraft:entity/zombie/zombie").unwrap();
        let atlas = crate::mesh::make_atlas(&packs, vec![id]).unwrap();
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![7], vec![11]);
        let head = Mat4::from_translation(Vec3::new(0.5, 65.5, 0.5))
            * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0) / 16.0);
        let rig = BodyRig {
            armor: crate::armor_render::ArmorRig {
                head: Some(head),
                ..Default::default()
            },
            light: Vec3::new(0.5, 64.5, 0.5),
            head_swing: 3.0,
            ..BodyRig::default()
        };
        let gear = BodyGear {
            armor: [item("minecraft:zombie_head", true), None, None, None],
            ..BodyGear::default()
        };
        let mut mesh = ChunkMesh::default();
        let mut items = ItemVisuals::default();
        append_body_gear(&mut mesh, &mut items, &rig, &gear, &packs, &atlas, &light).unwrap();
        assert_eq!(mesh.faces, 12, "the head and the hat layer");
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.sky_light, vertex.block_light) == (7.0, 11.0))
        );
        assert!(mesh.armor_glint.is_empty() && mesh.glint.is_empty());
        let top = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(top > 65.5 + 9.0 / 16.0 && top < 65.5 + 10.0 / 16.0, "{top}");
        let bare = BodyRig {
            armor: Default::default(),
            ..rig
        };
        let mut nothing = ChunkMesh::default();
        append_body_gear(&mut nothing, &mut items, &bare, &gear, &packs, &atlas, &light).unwrap();
        assert_eq!(nothing.faces, 0);
    }
}
