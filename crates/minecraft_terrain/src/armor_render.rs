//! `HumanoidArmorLayer` for humanoid mobs: each worn armour piece drawn on
//! the humanoid armour meshes (`HumanoidModel.createArmorMeshSet`: the
//! outer set inflated by 1.0 for the head, chest and feet, the inner by
//! 0.5 for the legs; armour legs a tenth thinner), posed as the wearer's
//! own parts, with its equipment asset's `humanoid` or `humanoid_leggings`
//! layers (`EquipmentLayerRenderer`; leather dyed its undyed colour, then
//! its overlay). Trims and enchantment glint are not drawn yet.
use crate::{
    body_gear::GearItem,
    cow_render::{cube_scaled, entity_shade},
    mesh::{Atlas, ChunkMesh, Vertex},
    pack::ResourceId,
    special_models::ENTITY_SHEET_ALPHA,
};
use glam::{DVec3, Mat4, Quat, Vec3};

/// The armour materials' equipment assets with humanoid textures.
pub const MATERIALS: [&str; 8] = ["leather", "chainmail", "iron", "gold", "diamond", "netherite", "copper", "turtle_scute"];

/// An armour item's equipment asset and the slot it is worn in (0 head,
/// 1 chest, 2 legs, 3 feet), as its `equippable` component names them.
pub fn armor_asset(item: &str) -> Option<(&'static str, usize)> {
    let name = item.strip_prefix("minecraft:")?;
    if name == "turtle_helmet" {
        return Some(("turtle_scute", 0));
    }
    let (material, piece) = name.rsplit_once('_')?;
    let slot = match piece {
        "helmet" => 0,
        "chestplate" => 1,
        "leggings" => 2,
        "boots" => 3,
        _ => return None,
    };
    let asset = match material {
        "leather" => "leather",
        "chainmail" => "chainmail",
        "iron" => "iron",
        "golden" => "gold",
        "diamond" => "diamond",
        "netherite" => "netherite",
        "copper" => "copper",
        _ => return None,
    };
    Some((asset, slot))
}

/// The wearer's humanoid parts: each part's pivot (model pixels) and
/// rotation — head, body, right arm, left arm, right leg, left leg.
#[derive(Clone, Copy, Debug)]
pub struct HumanoidParts {
    pub head: (Vec3, Quat),
    pub body: (Vec3, Quat),
    pub arms: [(Vec3, Quat); 2],
    pub legs: [(Vec3, Quat); 2],
}

/// `color_when_undyed` of leather armour (`-6265536`).
const UNDYED_LEATHER: [f32; 3] = [160.0 / 255.0, 101.0 / 255.0, 64.0 / 255.0];

/// Draws the armour in `armor` (head, chest, legs, feet item IDs) on a
/// humanoid posed by `parts`, in the wearer's frame (`feet`, `rotation`,
/// its model `scale`), lit like it.
#[allow(clippy::too_many_arguments)]
pub fn append_humanoid_armor(mesh: &mut ChunkMesh, atlas: &Atlas, feet: DVec3, rotation: Quat, scale: f32, parts: &HumanoidParts, armor: &[Option<String>; 4], sky: f32, block: f32) {
    for (slot, item) in armor.iter().enumerate() {
        let Some((asset, worn)) = item.as_deref().and_then(armor_asset) else { continue };
        // `shouldRender`: only in the slot it is made for.
        if worn != slot {
            continue;
        }
        let layer = if slot == 2 { "humanoid_leggings" } else { "humanoid" };
        let g = if slot == 2 { 0.5 } else { 1.0 };
        let textures: &[(&str, Option<[f32; 3]>)] = if asset == "leather" { &[("leather", Some(UNDYED_LEATHER)), ("leather_overlay", None)] } else { &[(asset, None)] };
        for &(texture, tint) in textures {
            let Ok(id) = ResourceId::parse(&format!("minecraft:entity/equipment/{layer}/{texture}")) else { continue };
            if !atlas.contains(&id) {
                continue;
            }
            let region = atlas.entity_region(&id);
            let mut cube = |from: [f32; 3], size: [f32; 3], grow: f32, uv: [f32; 2], (pivot, pose): (Vec3, Quat), mirror: bool| {
                let lo = [from[0] - grow, from[1] - grow, from[2] - grow];
                let hi = [from[0] + size[0] + grow, from[1] + size[1] + grow, from[2] + size[2] + grow];
                cube_scaled(mesh, feet, rotation, Vec3::splat(scale), region, sky, block, lo, hi, uv, pivot.to_array(), pose, tint.unwrap_or([1.0; 3]), [64.0, 32.0], Some(size), mirror);
            };
            match slot {
                0 => {
                    cube([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], g, [0.0, 0.0], parts.head, false);
                    cube([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], g + 0.5, [32.0, 0.0], parts.head, false);
                }
                1 => {
                    cube([-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], g, [16.0, 16.0], parts.body, false);
                    cube([-3.0, -2.0, -2.0], [4.0, 12.0, 4.0], g, [40.0, 16.0], parts.arms[0], false);
                    cube([-1.0, -2.0, -2.0], [4.0, 12.0, 4.0], g, [40.0, 16.0], parts.arms[1], true);
                }
                _ => {
                    if slot == 2 {
                        cube([-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], g, [16.0, 16.0], parts.body, false);
                    }
                    cube([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], g - 0.1, [0.0, 16.0], parts.legs[0], false);
                    cube([-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], g - 0.1, [0.0, 16.0], parts.legs[1], true);
                }
            }
        }
    }
}

pub const ARMOR_TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];
pub const DRAW_ORDER: [usize; 4] = [1, 2, 3, 0];
const WHITE: [f32; 3] = [1.0; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitPart {
    pub upper: Mat4,
    pub lower: Mat4,
    pub split: f32,
}

impl SplitPart {
    pub fn whole(part: Mat4) -> Self {
        Self { upper: part, lower: part, split: f32::INFINITY }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArmorRig {
    pub head: Option<Mat4>,
    pub body: Option<SplitPart>,
    pub arms: [Option<SplitPart>; 2],
    pub legs: [Option<SplitPart>; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct ArmorPanel {
    pub region: [f32; 4],
    pub tint: [f32; 3],
    pub sky: f32,
    pub block: f32,
}

pub fn dye_tint(rgb: u32) -> [f32; 3] {
    [rgb >> 16, rgb >> 8, rgb].map(|channel| (channel & 255) as f32 / 255.0)
}

#[allow(clippy::too_many_arguments)]
pub fn cube_posed_slice(mesh: &mut ChunkMesh, part: Mat4, panel: &ArmorPanel, from: [f32; 3], size: [f32; 3], grow: f32, uv: [f32; 2], range: (f32, f32), mirror: bool) {
    let lo = Vec3::from_array(from) - Vec3::splat(grow);
    let hi = Vec3::from_array(from) + Vec3::from_array(size) + Vec3::splat(grow);
    let (ya, yb) = (range.0.max(lo.y), range.1.min(hi.y));
    if yb <= ya {
        return;
    }
    let (x_lo, x_hi) = if mirror { (hi.x, lo.x) } else { (lo.x, hi.x) };
    let (z0, z1) = (lo.z, hi.z);
    let [w, h, d] = size;
    let (u, v) = (uv[0], uv[1]);
    let (u0, u1) = (u, u + d);
    let (u2, u22) = (u1 + w, u1 + w + w);
    let (u3, u4) = (u1 + w + d, u1 + w + d + w);
    let (v0, v1, v2) = (v, v + d, v + d + h);
    let side_v = |y: f32| v1 + (y - lo.y) / (hi.y - lo.y) * (v2 - v1);
    let (va, vb) = (side_v(ya), side_v(yb));
    let mut faces: Vec<([[f32; 3]; 4], [f32; 4])> = Vec::with_capacity(6);
    if ya <= lo.y {
        faces.push(([[x_hi, ya, z1], [x_lo, ya, z1], [x_lo, ya, z0], [x_hi, ya, z0]], [u1, v0, u2, v1]));
    }
    if yb >= hi.y {
        faces.push(([[x_hi, yb, z0], [x_lo, yb, z0], [x_lo, yb, z1], [x_hi, yb, z1]], [u2, v1, u22, v0]));
    }
    faces.push(([[x_lo, ya, z0], [x_lo, ya, z1], [x_lo, yb, z1], [x_lo, yb, z0]], [u0, va, u1, vb]));
    faces.push(([[x_hi, ya, z0], [x_lo, ya, z0], [x_lo, yb, z0], [x_hi, yb, z0]], [u1, va, u2, vb]));
    faces.push(([[x_hi, ya, z1], [x_hi, ya, z0], [x_hi, yb, z0], [x_hi, yb, z1]], [u2, va, u3, vb]));
    faces.push(([[x_lo, ya, z1], [x_hi, ya, z1], [x_hi, yb, z1], [x_lo, yb, z1]], [u3, va, u4, vb]));
    let [r0, s0, r1, s1] = panel.region;
    for (corners, tex) in faces {
        let mut vertices: [(Vec3, [f32; 2]); 4] = std::array::from_fn(|i| {
            let coord = [[tex[2], tex[1]], [tex[0], tex[1]], [tex[0], tex[3]], [tex[2], tex[3]]][i];
            (part.transform_point3(Vec3::from_array(corners[i])), coord)
        });
        if mirror {
            vertices.reverse();
        }
        let normal = (vertices[1].0 - vertices[0].0).cross(vertices[2].0 - vertices[0].0).try_normalize().unwrap_or(Vec3::Y);
        let shade = entity_shade(normal);
        let start = mesh.vertices.len() as u32;
        for (position, coord) in vertices {
            mesh.vertices.push(Vertex {
                position: position.to_array(),
                uv: [r0 + (r1 - r0) * coord[0] / ARMOR_TEXTURE_SIZE[0], s0 + (s1 - s0) * coord[1] / ARMOR_TEXTURE_SIZE[1]],
                color: [shade * panel.tint[0], shade * panel.tint[1], shade * panel.tint[2], ENTITY_SHEET_ALPHA],
                sky_light: panel.sky,
                block_light: panel.block,
            });
        }
        mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn split_cube(mesh: &mut ChunkMesh, part: &SplitPart, panel: &ArmorPanel, from: [f32; 3], size: [f32; 3], grow: f32, uv: [f32; 2], mirror: bool) {
    cube_posed_slice(mesh, part.upper, panel, from, size, grow, uv, (f32::NEG_INFINITY, part.split), mirror);
    cube_posed_slice(mesh, part.lower, panel, from, size, grow, uv, (part.split, f32::INFINITY), mirror);
}

pub fn armor_layers(item: &GearItem, slot: usize) -> Vec<(ResourceId, [f32; 3])> {
    let Some((asset, worn)) = armor_asset(&item.id) else { return Vec::new() };
    if worn != slot {
        return Vec::new();
    }
    let layer = if slot == 2 { "humanoid_leggings" } else { "humanoid" };
    let textures = if asset == "leather" { vec![("leather", item.dye.map_or(UNDYED_LEATHER, dye_tint)), ("leather_overlay", WHITE)] } else { vec![(asset, WHITE)] };
    textures.into_iter().filter_map(|(texture, tint)| Some((ResourceId::parse(&format!("minecraft:entity/equipment/{layer}/{texture}")).ok()?, tint))).collect()
}

pub fn append_armor_on_rig(mesh: &mut ChunkMesh, atlas: &Atlas, rig: &ArmorRig, armor: &[Option<GearItem>; 4], sky: f32, block: f32) {
    for slot in DRAW_ORDER {
        let Some(item) = armor[slot].as_ref() else { continue };
        let g = if slot == 2 { 0.5 } else { 1.0 };
        let mut foil = item.glint;
        for (id, tint) in armor_layers(item, slot) {
            if !atlas.contains(&id) {
                continue;
            }
            let start = mesh.vertices.len() as u32;
            let panel = ArmorPanel { region: atlas.entity_region(&id), tint, sky, block };
            match slot {
                0 => {
                    if let Some(head) = rig.head {
                        let whole = SplitPart::whole(head);
                        split_cube(mesh, &whole, &panel, [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], g, [0.0, 0.0], false);
                        split_cube(mesh, &whole, &panel, [-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], g + 0.5, [32.0, 0.0], false);
                    }
                }
                1 => {
                    if let Some(body) = rig.body.as_ref() {
                        split_cube(mesh, body, &panel, [-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], g, [16.0, 16.0], false);
                    }
                    if let Some(arm) = rig.arms[0].as_ref() {
                        split_cube(mesh, arm, &panel, [-3.0, -2.0, -2.0], [4.0, 12.0, 4.0], g, [40.0, 16.0], false);
                    }
                    if let Some(arm) = rig.arms[1].as_ref() {
                        split_cube(mesh, arm, &panel, [-1.0, -2.0, -2.0], [4.0, 12.0, 4.0], g, [40.0, 16.0], true);
                    }
                }
                _ => {
                    if let Some(body) = rig.body.as_ref().filter(|_| slot == 2) {
                        split_cube(mesh, body, &panel, [-4.0, 0.0, -2.0], [8.0, 12.0, 4.0], g, [16.0, 16.0], false);
                    }
                    if let Some(leg) = rig.legs[0].as_ref() {
                        split_cube(mesh, leg, &panel, [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], g - 0.1, [0.0, 16.0], false);
                    }
                    if let Some(leg) = rig.legs[1].as_ref() {
                        split_cube(mesh, leg, &panel, [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], g - 0.1, [0.0, 16.0], true);
                    }
                }
            }
            let end = mesh.vertices.len() as u32;
            if foil && end > start {
                mesh.armor_glint.push(start..end);
            }
            foil = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armour_items_name_their_asset_and_slot() {
        assert_eq!(armor_asset("minecraft:golden_chestplate"), Some(("gold", 1)));
        assert_eq!(armor_asset("minecraft:turtle_helmet"), Some(("turtle_scute", 0)));
        assert_eq!(armor_asset("minecraft:iron_boots"), Some(("iron", 3)));
        assert_eq!(armor_asset("minecraft:iron_sword"), None);
    }

    fn panel() -> ArmorPanel {
        ArmorPanel { region: [0.0, 0.0, 1.0, 1.0], tint: [1.0; 3], sky: 15.0, block: 0.0 }
    }

    fn v_range(mesh: &ChunkMesh, face: usize) -> (f32, f32) {
        let vs = mesh.vertices[face * 4..face * 4 + 4].iter().map(|vertex| vertex.uv[1]);
        vs.clone().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)))
    }

    #[test]
    fn a_cube_cut_in_two_keeps_its_sides_and_its_texture_panels() {
        let mut whole = ChunkMesh::default();
        cube_posed_slice(&mut whole, Mat4::IDENTITY, &panel(), [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], 1.0, [0.0, 16.0], (f32::NEG_INFINITY, f32::INFINITY), false);
        assert_eq!(whole.faces, 6);
        let mut open = ChunkMesh::default();
        cube_posed_slice(&mut open, Mat4::IDENTITY, &panel(), [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], 1.0, [0.0, 16.0], (0.0, 12.0), false);
        assert_eq!(open.faces, 4, "a slice inside the box has no caps");
        let mut upper = ChunkMesh::default();
        let mut lower = ChunkMesh::default();
        cube_posed_slice(&mut upper, Mat4::IDENTITY, &panel(), [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], 1.0, [0.0, 16.0], (f32::NEG_INFINITY, 4.0), false);
        cube_posed_slice(&mut lower, Mat4::IDENTITY, &panel(), [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], 1.0, [0.0, 16.0], (4.0, f32::INFINITY), false);
        assert_eq!((upper.faces, lower.faces), (5, 5));
        let (top, bottom) = (v_range(&upper, 1), v_range(&lower, 1));
        assert!((top.0 - 20.0 / 32.0).abs() < 1e-6 && (bottom.1 - 32.0 / 32.0).abs() < 1e-6, "{top:?} {bottom:?}");
        assert!((top.1 - bottom.0).abs() < 1e-6, "the halves meet: {top:?} {bottom:?}");
        let split_v = 20.0 + (4.0 + 1.0) / 14.0 * 12.0;
        assert!((top.1 - split_v / 32.0).abs() < 1e-6);
        let ys = |mesh: &ChunkMesh| mesh.vertices.iter().map(|vertex| vertex.position[1]).fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), y| (lo.min(y), hi.max(y)));
        assert_eq!(ys(&upper), (-1.0, 4.0));
        assert_eq!(ys(&lower), (4.0, 13.0));
        let mut outside = ChunkMesh::default();
        cube_posed_slice(&mut outside, Mat4::IDENTITY, &panel(), [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], 1.0, [0.0, 16.0], (20.0, 30.0), false);
        assert_eq!(outside.faces, 0);
    }

    #[test]
    fn a_posed_slice_follows_its_frame_and_faces_outward() {
        let part = Mat4::from_translation(Vec3::new(10.0, 64.0, 3.0)) * Mat4::from_rotation_z(0.7) * Mat4::from_scale(Vec3::splat(1.0 / 16.0));
        for mirror in [false, true] {
            let mut mesh = ChunkMesh::default();
            cube_posed_slice(&mut mesh, part, &panel(), [-2.0, 0.0, -2.0], [4.0, 12.0, 4.0], 0.0, [0.0, 16.0], (f32::NEG_INFINITY, f32::INFINITY), mirror);
            let centre = part.transform_point3(Vec3::new(0.0, 6.0, 0.0));
            for face in 0..6 {
                let corner = |i: usize| Vec3::from_array(mesh.vertices[mesh.indices[face * 6 + i] as usize].position);
                let middle = (0..4).map(|i| Vec3::from_array(mesh.vertices[face * 4 + i].position)).sum::<Vec3>() / 4.0;
                assert!((corner(1) - corner(0)).cross(corner(2) - corner(0)).dot(middle - centre) > 0.0, "face {face} mirror {mirror}");
            }
            assert!(mesh.vertices.iter().all(|vertex| vertex.color[3] == ENTITY_SHEET_ALPHA && vertex.sky_light == 15.0));
        }
    }

    fn armor_atlas() -> (crate::test_dir::TestDir, Atlas) {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("pack.mcmeta"), r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#).unwrap();
        let mut ids = Vec::new();
        for (layer, texture) in [("humanoid", "iron"), ("humanoid_leggings", "iron"), ("humanoid", "leather"), ("humanoid", "leather_overlay"), ("humanoid", "diamond")] {
            let path = root.join(format!("assets/minecraft/textures/entity/equipment/{layer}/{texture}.png"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            image::RgbaImage::from_pixel(64, 32, image::Rgba([200, 200, 200, 255])).save(path).unwrap();
            ids.push(ResourceId::parse(&format!("minecraft:entity/equipment/{layer}/{texture}")).unwrap());
        }
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let atlas = crate::mesh::make_atlas(&packs, ids).unwrap();
        (temp, atlas)
    }

    fn worn(id: &str, dye: Option<u32>) -> Option<GearItem> {
        Some(GearItem { id: id.to_owned(), glint: false, dye, potion: None, look: String::new() })
    }

    fn rig() -> ArmorRig {
        let part = |x: f32| SplitPart { upper: Mat4::from_translation(Vec3::new(x, 0.0, 0.0)), lower: Mat4::from_translation(Vec3::new(x, 0.0, 10.0)), split: 4.0 };
        ArmorRig { head: Some(Mat4::from_translation(Vec3::new(0.0, 100.0, 0.0))), body: Some(part(0.0)), arms: [Some(part(-20.0)), Some(part(20.0))], legs: [Some(part(-40.0)), Some(part(40.0))] }
    }

    fn in_region(mesh: &ChunkMesh, range: std::ops::Range<usize>, region: [f32; 4]) -> bool {
        mesh.vertices[range].iter().all(|vertex| vertex.uv[0] >= region[0] - 1e-6 && vertex.uv[0] <= region[2] + 1e-6 && vertex.uv[1] >= region[1] - 1e-6 && vertex.uv[1] <= region[3] + 1e-6)
    }

    #[test]
    fn each_piece_is_drawn_on_its_parts_in_the_vanilla_order() {
        let (_temp, atlas) = armor_atlas();
        let helmet = [worn("minecraft:diamond_helmet", None), None, None, None];
        let mut mesh = ChunkMesh::default();
        append_armor_on_rig(&mut mesh, &atlas, &rig(), &helmet, 15.0, 0.0);
        assert_eq!(mesh.faces, 12, "the head and the hat layer");
        assert!(mesh.vertices.iter().all(|vertex| vertex.position[1] > 90.0));
        let misplaced = [None, worn("minecraft:diamond_helmet", None), None, None];
        let mut mesh = ChunkMesh::default();
        append_armor_on_rig(&mut mesh, &atlas, &rig(), &misplaced, 15.0, 0.0);
        assert_eq!(mesh.faces, 0, "a helmet is only worn on the head");
        let iron = [worn("minecraft:iron_helmet", None), worn("minecraft:iron_chestplate", None), worn("minecraft:iron_leggings", None), worn("minecraft:iron_boots", None)];
        let mut mesh = ChunkMesh::default();
        append_armor_on_rig(&mut mesh, &atlas, &rig(), &iron, 15.0, 0.0);
        let humanoid = atlas.entity_region(&ResourceId::parse("minecraft:entity/equipment/humanoid/iron").unwrap());
        let leggings = atlas.entity_region(&ResourceId::parse("minecraft:entity/equipment/humanoid_leggings/iron").unwrap());
        let (chest, legs, feet, head) = (3 * 10, 3 * 10, 2 * 10, 12);
        assert_eq!(mesh.faces, chest + legs + feet + head);
        let mut at = 0;
        for (faces, region) in [(chest, humanoid), (legs, leggings), (feet, humanoid), (head, humanoid)] {
            assert!(in_region(&mesh, at * 4..(at + faces) * 4, region), "faces from {at}");
            at += faces;
        }
    }

    #[test]
    fn leather_is_dyed_under_its_undyed_overlay() {
        let (_temp, atlas) = armor_atlas();
        let base = atlas.entity_region(&ResourceId::parse("minecraft:entity/equipment/humanoid/leather").unwrap());
        let overlay = atlas.entity_region(&ResourceId::parse("minecraft:entity/equipment/humanoid/leather_overlay").unwrap());
        for (dye, colour) in [(Some(0xB02E26), dye_tint(0xB02E26)), (None, UNDYED_LEATHER)] {
            let mut mesh = ChunkMesh::default();
            append_armor_on_rig(&mut mesh, &atlas, &rig(), &[None, worn("minecraft:leather_chestplate", dye), None, None], 15.0, 0.0);
            assert_eq!(mesh.faces, 60);
            assert!(in_region(&mesh, 0..120, base) && in_region(&mesh, 120..240, overlay));
            let tint = |vertex: &Vertex| [vertex.color[0], vertex.color[1], vertex.color[2]];
            let top = &mesh.vertices[4];
            let shade = top.color[0] / colour[0];
            assert!(tint(top).iter().zip(colour).all(|(c, want)| (c - want * shade).abs() < 1e-5), "{:?}", tint(top));
            assert!(mesh.vertices[120..].iter().all(|vertex| vertex.color[0] == vertex.color[1] && vertex.color[1] == vertex.color[2]));
        }
    }

    #[test]
    fn an_enchanted_piece_shimmers_on_its_first_layer_only() {
        let (_temp, atlas) = armor_atlas();
        let enchanted = |id: &str| Some(GearItem { id: id.to_owned(), glint: true, dye: None, potion: None, look: String::new() });
        let mut mesh = ChunkMesh::default();
        append_armor_on_rig(&mut mesh, &atlas, &rig(), &[None, enchanted("minecraft:leather_chestplate"), None, None], 15.0, 0.0);
        assert_eq!((mesh.armor_glint.len(), mesh.armor_glint.first()), (1, Some(&(0..120))));
        assert!(mesh.glint.is_empty());
        let mut mesh = ChunkMesh::default();
        append_armor_on_rig(&mut mesh, &atlas, &rig(), &[enchanted("minecraft:diamond_helmet"), worn("minecraft:iron_chestplate", None), None, None], 15.0, 0.0);
        assert_eq!((mesh.armor_glint.len(), mesh.armor_glint.first()), (1, Some(&(120..168))), "the chestplate first, then the enchanted helmet");
    }
}
