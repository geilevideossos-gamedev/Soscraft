use anyhow::Result;
use glam::{Mat4, Vec3};

use crate::armor_render::{ArmorRig, append_armor_on_rig};
use crate::body_gear::BodyGear;
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, HeldDisplay, HeldItem, ItemVisuals};
use crate::pack::PackStack;

pub const HAND_DISPLAYS: [HeldDisplay; 2] = [HeldDisplay::RightHand, HeldDisplay::LeftHand];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyRig {
    pub hands: [Option<Mat4>; 2],
    pub armor: ArmorRig,
    pub light: Vec3,
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
                id: item.id.clone(),
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
}
