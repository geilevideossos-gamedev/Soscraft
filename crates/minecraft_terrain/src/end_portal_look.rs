use crate::model::{Element, Face, ResolvedModel};
use crate::pack::ResourceId;
use crate::scene::Block;

pub const END_PORTAL_ALPHA: f32 = 0.25;
pub const END_GATEWAY_ALPHA: f32 = 0.125;

const PORTAL_TEXTURE: &str = "minecraft:entity/end_portal/end_portal";
const GATEWAY_TEXTURE: &str = "minecraft:entity/end_portal/end_gateway_beam";
const PORTAL_BOTTOM: f32 = 0.375;
const PORTAL_TOP: f32 = 0.75;
const ALL_FACES: [&str; 6] = ["down", "up", "north", "south", "west", "east"];

pub(crate) fn block_model(block: &Block) -> Option<ResolvedModel> {
    let (texture, from, to, faces, cull): (_, _, _, &[&str], _) = match block.id.path.as_str() {
        "end_portal" if block.id.namespace == "minecraft" => (
            PORTAL_TEXTURE,
            [0.0, PORTAL_BOTTOM, 0.0],
            [1.0, PORTAL_TOP, 1.0],
            &ALL_FACES[..2],
            false,
        ),
        "end_gateway" if block.id.namespace == "minecraft" => {
            (GATEWAY_TEXTURE, [0.0; 3], [1.0; 3], &ALL_FACES[..], true)
        }
        _ => return None,
    };
    let texture = ResourceId::parse(texture).ok()?;
    let faces = faces
        .iter()
        .map(|direction| Face {
            direction: (*direction).to_owned(),
            texture: texture.clone(),
            uv: [0.0, 0.0, 1.0, 1.0],
            cull,
            cullface: None,
            tint: false,
            tint_index: None,
            force_translucent: false,
        })
        .collect();
    Some(ResolvedModel {
        elements: vec![Element {
            from,
            to,
            faces,
            rotation_y: 0,
            rotation_x: 0,
            rotation: None,
            shade_direction_override: None,
        }],
    })
}

pub(crate) fn face_alpha(texture: &ResourceId) -> Option<f32> {
    if texture.namespace != "minecraft" {
        return None;
    }
    match texture.path.as_str() {
        "entity/end_portal/end_portal" => Some(END_PORTAL_ALPHA),
        "entity/end_portal/end_gateway_beam" => Some(END_GATEWAY_ALPHA),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lighting::SkyLight;
    use crate::mesh::{
        BiomeTint, ChunkMesh, SectionVertex, TintKind, append_baked, append_block, bake_quads,
        make_atlas,
    };
    use crate::pack::PackStack;
    use crate::scene::HandcraftedScene;

    fn mesh_of(
        scene: &HandcraftedScene,
        pos: (i32, i32, i32),
        baked: bool,
        hidden: impl Fn((i32, i32, i32)) -> bool,
    ) -> ChunkMesh {
        let packs = PackStack::open(vec![]).unwrap();
        let atlas = make_atlas(&packs, vec![]).unwrap();
        let tint = BiomeTint::from_pack(&packs).unwrap();
        let light = SkyLight::build(scene);
        let block = crate::scene::Scene::block(scene, pos).unwrap();
        let variants = crate::model::resolve_block_variants(&packs, block).unwrap();
        let (mut mesh, mut transparent) = (ChunkMesh::default(), Vec::new());
        if baked {
            let quads = bake_quads(block, &variants[0].0, &atlas).unwrap();
            append_baked(
                scene,
                pos,
                &quads,
                TintKind::of(block),
                false,
                |at, _| hidden(at),
                |_| false,
                &atlas,
                &tint,
                &light,
                &mut mesh,
                &mut transparent,
            )
            .unwrap();
        } else {
            append_block(
                scene,
                pos,
                block,
                || Ok(&variants[..]),
                |at, _| hidden(at),
                &atlas,
                &tint,
                &light,
                &mut mesh,
                &mut transparent,
            )
            .unwrap();
        }
        assert!(
            transparent.is_empty(),
            "drawn opaque, as vanilla's end portal pipelines are"
        );
        mesh
    }

    fn alphas(mesh: &ChunkMesh) -> Vec<u8> {
        mesh.vertices
            .iter()
            .map(|vertex| SectionVertex::from_vertex(vertex).color[3])
            .collect()
    }

    #[test]
    fn an_end_portal_shows_its_top_and_bottom_at_vanilla_heights() {
        let mut scene = HandcraftedScene::default();
        scene.set((0, 64, 0), Some(Block::new("minecraft:end_portal")));
        scene.set((1, 64, 0), Some(Block::new("minecraft:end_portal")));
        for baked in [false, true] {
            let mesh = mesh_of(&scene, (0, 64, 0), baked, |_| false);
            assert_eq!(mesh.faces, 2);
            let mut heights: Vec<f32> = mesh
                .vertices
                .iter()
                .map(|vertex| vertex.position[1])
                .collect();
            heights.dedup();
            assert_eq!(heights, [64.375, 64.75]);
            assert!(
                mesh.vertices
                    .iter()
                    .all(|vertex| (0.0..=1.0).contains(&vertex.position[0])
                        && (0.0..=1.0).contains(&vertex.position[2]))
            );
            assert_eq!(alphas(&mesh), [63; 8]);
        }
    }

    #[test]
    fn an_end_gateway_is_a_cube_whose_covered_faces_are_hidden() {
        let mut scene = HandcraftedScene::default();
        scene.set((4, 70, 4), Some(Block::new("minecraft:end_gateway")));
        scene.set((4, 71, 4), Some(Block::new("minecraft:bedrock")));
        scene.set((4, 69, 4), Some(Block::new("minecraft:bedrock")));
        for baked in [false, true] {
            assert_eq!(mesh_of(&scene, (4, 70, 4), baked, |_| false).faces, 6);
            let mesh = mesh_of(&scene, (4, 70, 4), baked, |at| at.1 != 70);
            assert_eq!(mesh.faces, 4);
            assert!(
                mesh.vertices
                    .iter()
                    .all(|vertex| (70.0..=71.0).contains(&vertex.position[1]))
            );
            assert_eq!(alphas(&mesh), [31; 16]);
        }
    }

    #[test]
    fn the_marker_alphas_stay_apart_from_every_other_section_alpha() {
        assert_eq!(
            face_alpha(&ResourceId::parse(PORTAL_TEXTURE).unwrap()),
            Some(END_PORTAL_ALPHA)
        );
        assert_eq!(
            face_alpha(&ResourceId::parse(GATEWAY_TEXTURE).unwrap()),
            Some(END_GATEWAY_ALPHA)
        );
        assert_eq!(
            face_alpha(&ResourceId::parse("minecraft:entity/chest/normal").unwrap()),
            None
        );
        assert_eq!(
            face_alpha(&ResourceId::parse("minecraft:block/obsidian").unwrap()),
            None
        );
        let byte = |alpha: f32| (alpha * 255.0).floor() / 255.0;
        assert!(byte(END_GATEWAY_ALPHA) < 0.1875 && 0.1875 < byte(END_PORTAL_ALPHA));
        assert!(
            byte(END_PORTAL_ALPHA) < 0.375
                && 0.375 < byte(crate::special_models::ENTITY_SHEET_ALPHA)
        );
    }

    #[test]
    fn other_blocks_keep_their_own_models() {
        for id in [
            "minecraft:obsidian",
            "minecraft:end_portal_frame",
            "minecraft:nether_portal",
            "other:end_portal",
        ] {
            assert!(block_model(&Block::new(id)).is_none(), "{id}");
        }
    }
}
