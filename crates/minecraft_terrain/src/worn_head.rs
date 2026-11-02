use anyhow::Result;
use glam::{Mat4, Vec3};
use std::f32::consts::PI;

use crate::armor_render::armor_asset;
use crate::body_gear::GearItem;
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, HeldDisplay, HeldItem, ItemVisuals, append_elements};
use crate::pack::PackStack;
use crate::special_models::{worn_head_model, worn_skull};

pub const SKULL_SCALE: f32 = 1.1875;
const ITEM_SCALE: f32 = 0.625;

pub fn skull_pose(head: Mat4) -> Mat4 {
    head * Mat4::from_scale(Vec3::splat(16.0))
}

pub fn item_pose(head: Mat4) -> Mat4 {
    head * Mat4::from_scale(Vec3::splat(16.0))
        * Mat4::from_translation(Vec3::new(0.0, -0.25, 0.0))
        * Mat4::from_rotation_y(PI)
        * Mat4::from_scale(Vec3::new(ITEM_SCALE, -ITEM_SCALE, -ITEM_SCALE))
}

pub fn is_helmet(item: &GearItem) -> bool {
    armor_asset(&item.id).is_some_and(|(_, slot)| slot == 0)
}

#[allow(clippy::too_many_arguments)]
pub fn append_worn_head(
    mesh: &mut ChunkMesh,
    items: &mut ItemVisuals,
    head: Option<Mat4>,
    item: Option<&GearItem>,
    animation: f32,
    light: Vec3,
    packs: &PackStack,
    atlas: &Atlas,
    sky_light: &SkyLight,
    sky: f32,
    block: f32,
) -> Result<()> {
    let (Some(head), Some(item)) = (head, item) else {
        return Ok(());
    };
    if is_helmet(item) {
        return Ok(());
    }
    if let Some(skull) = worn_skull(&item.id) {
        let model = worn_head_model(skull, animation)?;
        let pose = skull_pose(head);
        return append_elements(mesh, &model.elements, pose, false, None, atlas, sky, block);
    }
    let held = HeldItem {
        pose: item_pose(head),
        light,
        id: item.key(),
        display: HeldDisplay::Head,
        first_tint: None,
    };
    let start = mesh.vertices.len() as u32;
    items.append_held_items(mesh, std::slice::from_ref(&held), packs, atlas, sky_light)?;
    let end = mesh.vertices.len() as u32;
    if item.glint && end > start {
        mesh.glint.push(start..end);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::pack::ResourceId;
    use crate::scene::HandcraftedScene;
    use std::path::Path;

    pub(crate) fn write_pack(root: &Path, files: &[(&str, &str)], images: &[(&str, u32, u32)]) {
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        for (name, contents) in files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        for (index, (name, width, height)) in images.iter().enumerate() {
            let path = root.join(format!("assets/minecraft/textures/{name}.png"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let shade = 60 + 40 * index as u8;
            image::RgbaImage::from_pixel(*width, *height, image::Rgba([shade, 120, 40, 255]))
                .save(path)
                .unwrap();
        }
    }

    pub(crate) fn pumpkin_pack(root: &Path) -> PackStack {
        write_pack(
            root,
            &[
                (
                    "assets/minecraft/items/carved_pumpkin.json",
                    r#"{"model":{"type":"minecraft:model","model":"minecraft:block/carved_pumpkin"}}"#,
                ),
                (
                    "assets/minecraft/items/hat_test.json",
                    r#"{"model":{"type":"minecraft:model","model":"minecraft:block/hat_test"}}"#,
                ),
                (
                    "assets/minecraft/models/block/block.json",
                    r#"{"display":{"thirdperson_righthand":{"rotation":[75,45,0],"translation":[0,2.5,0],"scale":[0.375,0.375,0.375]}}}"#,
                ),
                (
                    "assets/minecraft/models/block/carved_pumpkin.json",
                    r##"{"parent":"minecraft:block/block","textures":{"front":"minecraft:block/carved_pumpkin","side":"minecraft:block/pumpkin_side"},"elements":[{"from":[0,0,0],"to":[16,16,16],"faces":{"north":{"texture":"#front"},"south":{"texture":"#side"},"east":{"texture":"#side"},"west":{"texture":"#side"},"up":{"texture":"#side"},"down":{"texture":"#side"}}}]}"##,
                ),
                (
                    "assets/minecraft/models/block/hat_test.json",
                    r#"{"parent":"minecraft:block/carved_pumpkin","display":{"head":{"scale":[2,2,2]}}}"#,
                ),
            ],
            &[
                ("block/carved_pumpkin", 16, 16),
                ("block/pumpkin_side", 16, 16),
            ],
        );
        PackStack::open(vec![root.into()]).unwrap()
    }

    fn gear(id: &str, glint: bool) -> GearItem {
        GearItem {
            id: id.to_owned(),
            glint,
            dye: None,
            potion: None,
            look: String::new(),
        }
    }

    fn draw(
        packs: &PackStack,
        atlas: &Atlas,
        head: Option<Mat4>,
        item: Option<&GearItem>,
        animation: f32,
    ) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();
        let mut items = ItemVisuals::default();
        let light = SkyLight::build(&HandcraftedScene::new());
        append_worn_head(
            &mut mesh,
            &mut items,
            head,
            item,
            animation,
            Vec3::ZERO,
            packs,
            atlas,
            &light,
            15.0,
            0.0,
        )
        .unwrap();
        mesh
    }

    fn bounds(mesh: &ChunkMesh) -> (Vec3, Vec3) {
        mesh.vertices.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(low, high), vertex| {
                let point = Vec3::from_array(vertex.position);
                (low.min(point), high.max(point))
            },
        )
    }

    fn sheet_pack(root: &Path, texture: &str, size: (u32, u32)) -> (PackStack, Atlas) {
        write_pack(root, &[], &[(texture, size.0, size.1)]);
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let id = ResourceId::parse(&format!("minecraft:{texture}")).unwrap();
        let atlas = crate::mesh::make_atlas(&packs, vec![id]).unwrap();
        (packs, atlas)
    }

    #[test]
    fn a_worn_skull_is_the_head_cube_scaled_like_vanilla() {
        let temp = crate::test_dir::tempdir().unwrap();
        let (packs, atlas) = sheet_pack(temp.path(), "entity/skeleton/skeleton", (64, 32));
        let pixels = Mat4::from_scale(Vec3::splat(1.0 / 16.0));
        let skull = gear("minecraft:skeleton_skull", true);
        let mesh = draw(&packs, &atlas, Some(pixels), Some(&skull), 0.0);
        assert_eq!(mesh.faces, 6);
        let (low, high) = bounds(&mesh);
        assert!((high - low - Vec3::splat(9.5 / 16.0)).abs().max_element() < 1e-4);
        assert!(
            (low - Vec3::new(-4.75, -9.5, -4.75) / 16.0)
                .abs()
                .max_element()
                < 1e-4
        );
        let id = ResourceId::parse("minecraft:entity/skeleton/skeleton").unwrap();
        let [u0, v0, u1, v1] = atlas.entity_region(&id);
        assert!(mesh.vertices.iter().all(|vertex| {
            let [u, v] = vertex.uv;
            (u0..=u1).contains(&u) && (v0..=v1).contains(&v)
        }));
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.sky_light, vertex.block_light) == (15.0, 0.0))
        );
        assert!(mesh.glint.is_empty() && mesh.armor_glint.is_empty());
    }

    #[test]
    fn a_worn_dragon_head_points_its_snout_forward() {
        let temp = crate::test_dir::tempdir().unwrap();
        let (packs, atlas) = sheet_pack(temp.path(), "entity/enderdragon/dragon", (256, 256));
        let dragon = gear("minecraft:dragon_head", false);
        let mesh = draw(&packs, &atlas, Some(Mat4::IDENTITY), Some(&dragon), 0.0);
        assert_eq!(mesh.faces, 7 * 6, "six head cubes and the jaw");
        let (low, high) = bounds(&mesh);
        assert!(low.z < -15.0, "{low}");
        assert!(high.z < 10.0, "{high}");
        let open = draw(&packs, &atlas, Some(Mat4::IDENTITY), Some(&dragon), 2.5);
        assert_eq!(open.faces, mesh.faces);
        let head_faces = 6 * 6 * 4;
        assert_eq!(
            open.vertices[..head_faces],
            mesh.vertices[..head_faces],
            "only the jaw moves"
        );
        assert_ne!(open.vertices[head_faces..], mesh.vertices[head_faces..]);
    }

    #[test]
    fn a_worn_carved_pumpkin_is_its_block_model_centred_on_the_head() {
        let temp = crate::test_dir::tempdir().unwrap();
        let packs = pumpkin_pack(temp.path());
        let front = ResourceId::parse("minecraft:block/carved_pumpkin").unwrap();
        let side = ResourceId::parse("minecraft:block/pumpkin_side").unwrap();
        let atlas = crate::mesh::make_atlas(&packs, vec![front.clone(), side]).unwrap();
        let pumpkin = gear("minecraft:carved_pumpkin", false);
        let mesh = draw(&packs, &atlas, Some(Mat4::IDENTITY), Some(&pumpkin), 0.0);
        assert_eq!(mesh.faces, 6);
        let (low, high) = bounds(&mesh);
        assert!((high - low - Vec3::splat(10.0)).abs().max_element() < 1e-3);
        let centre = (low + high) / 2.0;
        assert!(
            (centre - Vec3::new(0.0, -4.0, 0.0)).length() < 1e-3,
            "{centre}"
        );
        let [u0, v0, u1, v1] = atlas.region(&front);
        let carved = mesh
            .vertices
            .iter()
            .filter(|vertex| {
                let [u, v] = vertex.uv;
                (u0..=u1).contains(&u) && (v0..=v1).contains(&v)
            })
            .collect::<Vec<_>>();
        assert_eq!(carved.len(), 4);
        assert!(
            carved
                .iter()
                .all(|vertex| (vertex.position[2] - low.z).abs() < 1e-3)
        );
        assert!(mesh.glint.is_empty());
        let enchanted = gear("minecraft:carved_pumpkin", true);
        let shiny = draw(&packs, &atlas, Some(Mat4::IDENTITY), Some(&enchanted), 0.0);
        assert_eq!(shiny.glint, vec![0..shiny.vertices.len() as u32]);
        assert!(shiny.armor_glint.is_empty());
    }

    #[test]
    fn a_helmet_is_left_to_the_armour_layer() {
        let temp = crate::test_dir::tempdir().unwrap();
        let packs = pumpkin_pack(temp.path());
        let side = ResourceId::parse("minecraft:block/pumpkin_side").unwrap();
        let atlas = crate::mesh::make_atlas(&packs, vec![side]).unwrap();
        for id in ["minecraft:diamond_helmet", "minecraft:turtle_helmet"] {
            let helmet = gear(id, true);
            assert!(is_helmet(&helmet));
            let mesh = draw(&packs, &atlas, Some(Mat4::IDENTITY), Some(&helmet), 0.0);
            assert_eq!((mesh.faces, mesh.glint.len()), (0, 0), "{id}");
        }
        assert!(!is_helmet(&gear("minecraft:carved_pumpkin", false)));
        assert!(!is_helmet(&gear("minecraft:diamond_chestplate", false)));
        let pumpkin = gear("minecraft:carved_pumpkin", false);
        assert_eq!(draw(&packs, &atlas, None, Some(&pumpkin), 0.0).faces, 0);
        assert_eq!(
            draw(&packs, &atlas, Some(Mat4::IDENTITY), None, 0.0).faces,
            0
        );
    }
}
