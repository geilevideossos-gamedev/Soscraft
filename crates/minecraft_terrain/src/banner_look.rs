use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use glam::{DVec3, Mat4, Vec3};
use minecraftoss_core::nbt::Tag;
use minecraftoss_world::level::components::{BANNER, BANNER_PATTERNS};
use serde_json::Value;

use crate::item_look::{ItemLook, MAX_LAYERS};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::model::ResolvedModel;
use crate::scene::{Block, BlockPos, Scene};
use crate::special_models;

pub const VIEW_DISTANCE: f64 = 64.0;

#[derive(Clone, Debug)]
pub struct PosedBanner {
    pub cell: BlockPos,
    pub block: Block,
    pub model: ResolvedModel,
    pub tints: Vec<[f32; 3]>,
}

#[derive(Clone, Debug)]
struct Banner {
    layers: Vec<(String, &'static str)>,
    posed: Option<PosedBanner>,
}

#[derive(Clone, Debug, Default)]
pub struct BannerLooks {
    banners: BTreeMap<BlockPos, Banner>,
    apart: BTreeSet<BlockPos>,
}

impl BannerLooks {
    pub fn len(&self) -> usize {
        self.banners.len()
    }

    pub fn is_empty(&self) -> bool {
        self.banners.is_empty()
    }

    pub fn layers(&self, pos: BlockPos) -> Option<&[(String, &'static str)]> {
        self.banners
            .get(&pos)
            .map(|banner| banner.layers.as_slice())
    }

    pub fn load_chunk(&mut self, chunk: &minecraftoss_core::Chunk) {
        let store = &chunk.block_entities;
        for (pos, tag) in store.entities.iter().chain(
            store
                .pending
                .iter()
                .filter(|(pos, _)| !store.entities.contains_key(pos)),
        ) {
            self.set(*pos, Some(tag));
        }
    }

    pub fn unload_chunk(&mut self, pos: minecraftoss_core::ChunkPos) {
        self.banners
            .retain(|&(x, _, z), _| (x >> 4, z >> 4) != (pos.x, pos.z));
    }

    pub fn block_entities(&mut self, changes: &[(BlockPos, Option<Tag>)]) {
        for (pos, tag) in changes {
            self.set(*pos, tag.as_ref());
        }
    }

    fn set(&mut self, pos: BlockPos, tag: Option<&Tag>) {
        let layers = tag.map(layers).unwrap_or_default();
        if layers.is_empty() {
            self.banners.remove(&pos);
        } else if self
            .banners
            .get(&pos)
            .is_none_or(|banner| banner.layers != layers)
        {
            let posed = None;
            self.banners.insert(pos, Banner { layers, posed });
        }
    }

    pub fn take_apart_changes<S: Scene + ?Sized>(&mut self, scene: &S) -> Vec<(BlockPos, bool)> {
        let shown: BTreeSet<BlockPos> = self
            .banners
            .keys()
            .copied()
            .filter(|&pos| scene.block(pos).is_some_and(is_banner_block))
            .collect();
        let changes = self
            .apart
            .difference(&shown)
            .map(|&pos| (pos, false))
            .chain(shown.difference(&self.apart).map(|&pos| (pos, true)))
            .collect();
        self.apart = shown;
        changes
    }

    pub fn posed<S: Scene + ?Sized>(&mut self, scene: &S, camera: DVec3) -> Vec<&PosedBanner> {
        let mut shown = Vec::new();
        for (&cell, banner) in &mut self.banners {
            if centre(cell).distance_squared(camera) >= VIEW_DISTANCE * VIEW_DISTANCE {
                continue;
            }
            let Some(block) = scene.block(cell).filter(|block| is_banner_block(block)) else {
                continue;
            };
            if banner
                .posed
                .as_ref()
                .is_none_or(|posed| posed.block != *block)
            {
                banner.posed = pose(cell, block, &banner.layers);
            }
            let banner: &Banner = banner;
            shown.extend(banner.posed.as_ref());
        }
        shown
    }

    pub fn append<S: Scene + ?Sized>(
        &mut self,
        mesh: &mut ChunkMesh,
        scene: &S,
        atlas: &Atlas,
        light: &SkyLight,
        camera: DVec3,
    ) -> Result<()> {
        for banner in self.posed(scene, camera) {
            let (x, y, z) = banner.cell;
            let pose = Mat4::from_translation(Vec3::new(x as f32, y as f32, z as f32));
            let sky = f32::from(light.get(banner.cell));
            let block = f32::from(light.get_block(banner.cell));
            let elements = &banner.model.elements;
            let tints = Some(banner.tints.as_slice());
            crate::mesh::append_elements(mesh, elements, pose, false, tints, atlas, sky, block)?;
        }
        Ok(())
    }
}

pub fn is_banner_block(block: &Block) -> bool {
    block.id.namespace == "minecraft" && special_models::banner_dye(block).is_some()
}

pub fn layers(tag: &Tag) -> Vec<(String, &'static str)> {
    if tag.get("id").and_then(Tag::as_str) != Some(BANNER) {
        return Vec::new();
    }
    let Some(patterns) = tag.get("patterns") else {
        return Vec::new();
    };
    let patterns = crate::item_nbt::tag_json(patterns);
    let components = Value::Object(
        [(BANNER_PATTERNS.to_owned(), patterns)]
            .into_iter()
            .collect(),
    );
    ItemLook::of(Some(&components), None).layers
}

pub fn pose(cell: BlockPos, block: &Block, layers: &[(String, &str)]) -> Option<PosedBanner> {
    let base = special_models::banner_dye(block)?;
    let model = special_models::placed_banner_model(block, layers).ok()??;
    let colours = layers
        .iter()
        .take(MAX_LAYERS)
        .map(|&(_, colour)| special_models::dye_color(colour).unwrap_or([1.0; 3]));
    Some(PosedBanner {
        cell,
        block: block.clone(),
        model,
        tints: std::iter::once(base).chain(colours).collect(),
    })
}

fn centre((x, y, z): BlockPos) -> DVec3 {
    DVec3::new(f64::from(x), f64::from(y), f64::from(z)) + 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::HandcraftedScene;
    use minecraftoss_core::snbt::parse;

    const CREEPER_AND_BORDER: &str = r#"[{pattern:"minecraft:creeper",color:"black"},{pattern:{asset_id:"minecraft:border",translation_key:"block.minecraft.banner.border"},color:"red"}]"#;

    fn banner_tag((x, y, z): BlockPos, patterns: &str) -> Tag {
        parse(&format!(
            r#"{{id:"minecraft:banner",x:{x},y:{y},z:{z},patterns:{patterns},components:{{}}}}"#
        ))
        .unwrap()
    }

    fn plain_tag((x, y, z): BlockPos) -> Tag {
        parse(&format!(
            r#"{{id:"minecraft:banner",x:{x},y:{y},z:{z},components:{{}}}}"#
        ))
        .unwrap()
    }

    fn owned(layers: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
        layers
            .iter()
            .map(|&(asset, colour)| (asset.to_owned(), colour))
            .collect()
    }

    fn shape(element: &crate::model::Element) -> String {
        let faces: Vec<_> = element
            .faces
            .iter()
            .map(|face| (&face.direction, face.uv, face.uv_rotation))
            .collect();
        format!(
            "{:?} {:?} {:?} {:?} {:?} {faces:?}",
            element.from, element.to, element.rotation, element.rotation_x, element.rotation_y
        )
    }

    fn sheets(model: &ResolvedModel) -> Vec<(String, Option<usize>)> {
        model
            .elements
            .iter()
            .map(|element| (element.faces[0].texture.key(), element.faces[0].tint_index))
            .collect()
    }

    #[test]
    fn a_banner_s_layers_come_from_its_patterns_in_either_form() {
        let at = (1, 2, 3);
        let tag = banner_tag(at, CREEPER_AND_BORDER);
        assert_eq!(
            layers(&tag),
            owned(&[("minecraft:creeper", "black"), ("minecraft:border", "red")])
        );
        let odd = banner_tag(
            at,
            r#"[{pattern:"minecraft:creeper"},{pattern:"stripe_top",color:"lime"},{pattern:"minecraft:cross",color:"no_such_dye"}]"#,
        );
        assert_eq!(layers(&odd), owned(&[("minecraft:stripe_top", "lime")]));
        let crowded: Vec<String> = (0..20)
            .map(|i| format!(r#"{{pattern:"minecraft:p{i}",color:"blue"}}"#))
            .collect();
        let crowded = layers(&banner_tag(at, &format!("[{}]", crowded.join(","))));
        assert_eq!(crowded.len(), MAX_LAYERS);
        assert_eq!(crowded.last().unwrap().0, "minecraft:p15");
        assert!(layers(&plain_tag(at)).is_empty());
        assert!(layers(&banner_tag(at, "[]")).is_empty());
        let spawner = parse(&format!(
            r#"{{id:"minecraft:mob_spawner",x:1,y:2,z:3,patterns:{CREEPER_AND_BORDER}}}"#
        ))
        .unwrap();
        assert!(layers(&spawner).is_empty(), "only a banner has patterns");
    }

    #[test]
    fn a_placed_banner_poses_like_its_block_model_with_its_layers_on_the_flag() {
        let layers = owned(&[("minecraft:creeper", "black"), ("minecraft:border", "red")]);
        for (block, flag) in [
            (
                Block::new("minecraft:white_banner").with("rotation", "4"),
                2,
            ),
            (
                Block::new("minecraft:blue_wall_banner").with("facing", "east"),
                1,
            ),
        ] {
            let baked = special_models::block_model(&block).unwrap().unwrap();
            let posed = pose((7, 64, -3), &block, &layers).unwrap();
            let elements = &posed.model.elements;
            assert_eq!(elements.len(), baked.elements.len() + 2);
            for (posed, baked) in elements.iter().zip(&baked.elements) {
                assert_eq!(shape(posed), shape(baked), "{}", block.id.path);
            }
            assert_eq!(
                sheets(&posed.model)[..baked.elements.len()],
                sheets(&baked)[..]
            );
            assert_eq!(
                sheets(&posed.model)[baked.elements.len()..],
                [
                    ("minecraft:entity/banner/creeper".to_owned(), Some(1)),
                    ("minecraft:entity/banner/border".to_owned(), Some(2))
                ]
            );
            for layer in &elements[flag + 1..] {
                assert_eq!(
                    shape(layer),
                    shape(&elements[flag]),
                    "a layer lies on the flag"
                );
            }
            let base = special_models::banner_dye(&block).unwrap();
            let [black, red] = ["black", "red"].map(|dye| special_models::dye_color(dye).unwrap());
            assert_eq!(posed.tints, vec![base, black, red]);
        }
        assert!(pose((0, 0, 0), &Block::new("minecraft:stone"), &layers).is_none());
    }

    #[test]
    fn only_patterned_banners_on_banner_blocks_are_drawn_apart() {
        let mut scene = HandcraftedScene::default();
        let (standing, wall, plain) = ((1, 70, 1), (3, 70, 1), (5, 70, 1));
        scene.set(
            standing,
            Some(Block::new("minecraft:white_banner").with("rotation", "0")),
        );
        scene.set(
            wall,
            Some(Block::new("minecraft:red_wall_banner").with("facing", "north")),
        );
        scene.set(
            plain,
            Some(Block::new("minecraft:white_banner").with("rotation", "8")),
        );
        let mut banners = BannerLooks::default();
        banners.block_entities(&[
            (standing, Some(banner_tag(standing, CREEPER_AND_BORDER))),
            (wall, Some(banner_tag(wall, CREEPER_AND_BORDER))),
            (plain, Some(plain_tag(plain))),
        ]);
        assert_eq!(banners.len(), 2, "a plain banner stays baked");
        assert_eq!(
            banners.take_apart_changes(&scene),
            [(standing, true), (wall, true)]
        );
        assert!(banners.take_apart_changes(&scene).is_empty(), "told once");
        let camera = DVec3::new(3.0, 71.0, 3.0);
        let cells = |banners: &mut BannerLooks, scene: &HandcraftedScene| -> Vec<BlockPos> {
            banners
                .posed(scene, camera)
                .iter()
                .map(|banner| banner.cell)
                .collect()
        };
        assert_eq!(cells(&mut banners, &scene), [standing, wall]);
        scene.set(wall, None);
        assert_eq!(
            banners.take_apart_changes(&scene),
            [(wall, false)],
            "mined away"
        );
        assert_eq!(
            cells(&mut banners, &scene),
            [standing],
            "only a banner block is drawn"
        );
        scene.set(
            wall,
            Some(Block::new("minecraft:red_wall_banner").with("facing", "south")),
        );
        assert_eq!(banners.take_apart_changes(&scene), [(wall, true)]);
        let turned = banners.posed(&scene, camera);
        assert_eq!(turned.len(), 2);
        assert_eq!(
            turned[1].block.properties.get("facing").map(String::as_str),
            Some("south")
        );
        let north = pose(
            wall,
            &Block::new("minecraft:red_wall_banner").with("facing", "north"),
            &owned(&[("minecraft:creeper", "black"), ("minecraft:border", "red")]),
        )
        .unwrap();
        assert_ne!(
            shape(&turned[1].model.elements[0]),
            shape(&north.model.elements[0]),
            "a new block poses again"
        );
        banners.block_entities(&[(standing, Some(plain_tag(standing))), (wall, None)]);
        assert!(banners.is_empty());
        assert_eq!(
            banners.take_apart_changes(&scene),
            [(standing, false), (wall, false)],
            "patterns gone: baked again"
        );
        scene.set(standing, Some(Block::new("minecraft:stone")));
        banners.block_entities(&[(standing, Some(banner_tag(standing, CREEPER_AND_BORDER)))]);
        assert!(
            banners.take_apart_changes(&scene).is_empty(),
            "stone is never drawn apart"
        );
        assert!(banners.posed(&scene, camera).is_empty());
    }

    #[test]
    fn banners_follow_their_chunks_and_are_drawn_within_sixty_four_blocks() {
        let plains = minecraftoss_core::BiomeId(0);
        let mut chunk = minecraftoss_core::Chunk::new(
            minecraftoss_core::ChunkPos::new(1, -1),
            -64,
            384,
            plains,
        );
        let (loaded, pending, plain) = ((17, 64, -15), (18, 64, -15), (19, 64, -15));
        let store = &mut chunk.block_entities;
        store
            .entities
            .insert(loaded, banner_tag(loaded, CREEPER_AND_BORDER));
        store.pending.insert(
            pending,
            banner_tag(pending, r#"[{pattern:"minecraft:globe",color:"lime"}]"#),
        );
        store.pending.insert(loaded, plain_tag(loaded));
        store.entities.insert(plain, plain_tag(plain));
        let mut banners = BannerLooks::default();
        let other = (0, 64, 0);
        banners.block_entities(&[(other, Some(banner_tag(other, CREEPER_AND_BORDER)))]);
        banners.load_chunk(&chunk);
        assert_eq!(banners.len(), 3);
        assert_eq!(
            banners.layers(loaded).map(<[_]>::len),
            Some(2),
            "a loaded tag wins over a pending one"
        );
        assert_eq!(
            banners.layers(pending),
            Some(&owned(&[("minecraft:globe", "lime")])[..])
        );
        assert_eq!(banners.layers(plain), None);
        let mut scene = HandcraftedScene::default();
        for at in [loaded, pending, other] {
            scene.set(at, Some(Block::new("minecraft:white_banner")));
        }
        assert_eq!(banners.take_apart_changes(&scene).len(), 3);
        let near = DVec3::new(17.5, 64.5, -15.5);
        let cells = |banners: &mut BannerLooks, camera: DVec3| -> Vec<BlockPos> {
            banners
                .posed(&scene, camera)
                .iter()
                .map(|banner| banner.cell)
                .collect()
        };
        assert_eq!(cells(&mut banners, near), [other, loaded, pending]);
        let far = near + DVec3::X * (VIEW_DISTANCE - 0.01);
        assert_eq!(
            cells(&mut banners, far),
            [loaded, pending],
            "the other banner is 64 blocks off"
        );
        banners.unload_chunk(minecraftoss_core::ChunkPos::new(1, -1));
        assert_eq!(banners.len(), 1);
        assert_eq!(
            banners.take_apart_changes(&scene),
            [(loaded, false), (pending, false)],
            "an unloaded chunk leaves no cell apart"
        );
    }

    #[test]
    fn a_placed_banner_draws_the_baked_faces_then_its_layers_in_their_colours() {
        let (_temp, packs) = crate::item_icons::tests::banner_pack();
        let textures = ["banner_base", "base", "stripe"].map(|name| {
            crate::pack::ResourceId::parse(&format!("minecraft:entity/banner/{name}")).unwrap()
        });
        let atlas = crate::mesh::make_atlas(&packs, textures.to_vec()).unwrap();
        let tint = crate::mesh::BiomeTint::from_pack(&packs).unwrap();
        let cell = (4, 10, 4);
        let block = Block::new("minecraft:white_banner").with("rotation", "6");
        let mut scene = HandcraftedScene::default();
        scene.set(cell, Some(block.clone()));
        let light = SkyLight::build(&scene);
        let variants = [(special_models::block_model(&block).unwrap().unwrap(), 1)];
        let mut baked = ChunkMesh::default();
        let mut transparent = Vec::new();
        crate::mesh::append_block(
            &scene,
            cell,
            &block,
            || Ok(variants.as_slice()),
            |_, _| false,
            &atlas,
            &tint,
            &light,
            &mut baked,
            &mut transparent,
        )
        .unwrap();
        assert!(transparent.is_empty());
        assert_eq!(baked.faces, 18, "pole, bar and flag");
        let mut banners = BannerLooks::default();
        banners.block_entities(&[(
            cell,
            Some(banner_tag(
                cell,
                r#"[{pattern:"minecraft:stripe",color:"red"}]"#,
            )),
        )]);
        let mut drawn = ChunkMesh::default();
        let camera = DVec3::new(0.0, 12.0, 0.0);
        banners
            .append(&mut drawn, &scene, &atlas, &light, camera)
            .unwrap();
        assert_eq!(drawn.faces, 24, "and one layer on the flag");
        assert_eq!(drawn.vertices.len(), 96);
        let close = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5);
        for (apart, baked) in drawn.vertices.iter().zip(&baked.vertices) {
            assert_eq!(
                (apart.position, apart.uv, apart.sky_light, apart.block_light),
                (baked.position, baked.uv, baked.sky_light, baked.block_light),
                "drawn apart where the section would bake it"
            );
            assert!(
                close(apart.color, baked.color),
                "{:?} {:?}",
                apart.color,
                baked.color
            );
        }
        assert_eq!(drawn.indices[..108], baked.indices[..]);
        let (flag, layer) = (&drawn.vertices[48..72], &drawn.vertices[72..]);
        let white = special_models::banner_dye(&block).unwrap();
        let red = special_models::dye_color("red").unwrap();
        for (on_flag, on_layer) in flag.iter().zip(layer) {
            assert_eq!(
                on_layer.position, on_flag.position,
                "the layer lies on the flag"
            );
            let shade = on_flag.color[1] / white[1];
            let expected = [
                red[0] * shade,
                red[1] * shade,
                red[2] * shade,
                on_layer.color[3],
            ];
            assert!(close(on_layer.color, expected), "{:?}", on_layer.color);
            assert_eq!(
                (on_layer.sky_light, on_layer.block_light),
                (on_flag.sky_light, on_flag.block_light)
            );
        }
    }
}
