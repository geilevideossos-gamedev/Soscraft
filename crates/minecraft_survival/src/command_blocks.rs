use minecraft_commands::blocks::BlockState;
use minecraft_commands::{BlockAccess, BlockInput, Loaded};
use minecraft_terrain::scene::{HandcraftedScene, Scene};
use minecraft_terrain::server::ServerSim;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{BlockStateId, Registries};

pub const WORLD_BORDER: i32 = 30_000_000;

pub struct SimBlocks<'a>(pub &'a mut ServerSim);

fn at(pos: [i32; 3]) -> (i32, i32, i32) {
    (pos[0], pos[1], pos[2])
}

pub fn describe(registries: &Registries, state: BlockStateId) -> BlockState {
    let blocks = &registries.blocks;
    let block = blocks.block(blocks.block_of(state));
    BlockState {
        id: block.name.to_string(),
        properties: block
            .properties()
            .iter()
            .filter_map(|property| {
                let value = blocks.property(state, &property.name)?;
                Some((property.name.to_string(), value.to_owned()))
            })
            .collect(),
    }
}

pub fn state_of(
    registries: &Registries,
    id: &str,
    properties: &[(String, String)],
) -> Option<BlockStateId> {
    let blocks = &registries.blocks;
    let mut state = blocks.block(blocks.block_by_name(id)?).default_state();
    for (name, value) in properties {
        if let Some(next) = blocks.with_property(state, name, value) {
            state = next;
        }
    }
    Some(state)
}

pub fn nbt_matches(expected: &Tag, actual: Option<&Tag>, partial: bool) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    match (expected, actual) {
        (Tag::Compound(wanted), Tag::Compound(_)) => wanted
            .iter()
            .all(|(key, value)| nbt_matches(value, actual.get(key), partial)),
        (Tag::List(wanted), Tag::List(found)) if partial => {
            if wanted.is_empty() {
                return found.is_empty();
            }
            wanted.iter().all(|value| {
                found
                    .iter()
                    .any(|candidate| nbt_matches(value, Some(candidate), partial))
            })
        }
        _ => expected == actual,
    }
}

pub fn loaded_at(scene: &HandcraftedScene, pos: [i32; 3]) -> Loaded {
    if scene.generated_chunk((pos[0] >> 4, pos[2] >> 4)).is_none() {
        return Loaded::Unloaded;
    }
    let horizontal = |value: i32| (-WORLD_BORDER..WORLD_BORDER).contains(&value);
    if Scene::vertical_range(scene).contains(&pos[1]) && horizontal(pos[0]) && horizontal(pos[2]) {
        Loaded::Yes
    } else {
        Loaded::OutOfWorld
    }
}

impl BlockAccess for SimBlocks<'_> {
    fn state(&self, pos: [i32; 3]) -> BlockState {
        describe(self.0.command_registries(), self.0.command_block(at(pos)))
    }

    fn destroy(&mut self, pos: [i32; 3]) -> bool {
        self.0.command_destroy(at(pos))
    }

    fn place(&mut self, pos: [i32; 3], block: &BlockInput, strict: bool) -> bool {
        let Some(state) = state_of(self.0.command_registries(), &block.id, &block.properties)
        else {
            return false;
        };
        self.0
            .command_place(at(pos), state, &block.properties, strict)
    }

    fn update_neighbours(&mut self, pos: [i32; 3], old: &BlockState) {
        if let Some(old) = state_of(self.0.command_registries(), &old.id, &old.properties) {
            self.0.command_updates(at(pos), old);
        }
    }

    fn block_entity_matches(&self, pos: [i32; 3], nbt: &str) -> bool {
        let Ok(expected) = minecraftoss_core::snbt::parse_compound(nbt) else {
            return false;
        };
        nbt_matches(&expected, self.0.command_block_entity(at(pos)), true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use minecraft_commands::blocks::{BlockEdit, BlockFilter, EditKind, FillMode};
    use minecraftoss_core::{BiomeId, Chunk, ChunkPos};

    use super::*;

    fn compound(entries: &[(&str, Tag)]) -> Tag {
        Tag::Compound(
            entries
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn nbt_predicates_compare_like_vanilla_compare_nbt() {
        let item = |id: &str, slot: i8| {
            compound(&[
                ("id", Tag::String(id.to_owned())),
                ("Slot", Tag::Byte(slot)),
            ])
        };
        let chest = compound(&[
            ("id", Tag::String("minecraft:chest".to_owned())),
            (
                "Items",
                Tag::List(vec![item("minecraft:stone", 0), item("minecraft:dirt", 4)]),
            ),
            ("Lock", Tag::String(String::new())),
        ]);
        let wanted = |text: &str| minecraftoss_core::snbt::parse_compound(text).unwrap();
        assert!(nbt_matches(&wanted("{}"), Some(&chest), true));
        assert!(nbt_matches(
            &wanted("{Items:[{id:\"minecraft:dirt\"}]}"),
            Some(&chest),
            true
        ));
        assert!(!nbt_matches(
            &wanted("{Items:[{id:\"minecraft:dirt\"}]}"),
            Some(&chest),
            false
        ));
        assert!(!nbt_matches(
            &wanted("{Items:[{id:\"minecraft:gold_block\"}]}"),
            Some(&chest),
            true
        ));
        assert!(!nbt_matches(&wanted("{Items:[]}"), Some(&chest), true));
        assert!(!nbt_matches(&wanted("{Lock:\"key\"}"), Some(&chest), true));
        assert!(!nbt_matches(&wanted("{Missing:1b}"), Some(&chest), true));
        assert!(!nbt_matches(&wanted("{}"), None, true));
        assert!(!nbt_matches(
            &wanted("{Lock:1b}"),
            Some(&compound(&[("Lock", Tag::Int(1))])),
            true
        ));
    }

    #[test]
    fn positions_are_loaded_only_in_streamed_chunks_inside_the_world() {
        let mut scene = HandcraftedScene::default();
        assert_eq!(loaded_at(&scene, [3, 0, 3]), Loaded::Unloaded);
        scene.insert_chunk(Arc::new(Chunk::new(
            ChunkPos::new(-1, 0),
            -64,
            384,
            BiomeId(0),
        )));
        assert_eq!(loaded_at(&scene, [-3, 0, 15]), Loaded::Yes);
        assert_eq!(loaded_at(&scene, [-3, 0, 16]), Loaded::Unloaded);
        assert_eq!(loaded_at(&scene, [0, 0, 3]), Loaded::Unloaded);
        let range = Scene::vertical_range(&scene);
        assert_eq!(loaded_at(&scene, [-3, range.end, 3]), Loaded::OutOfWorld);
        assert_eq!(
            loaded_at(&scene, [-3, range.start - 1, 3]),
            Loaded::OutOfWorld
        );
        let far = WORLD_BORDER >> 4;
        scene.insert_chunk(Arc::new(Chunk::new(
            ChunkPos::new(far, 0),
            -64,
            384,
            BiomeId(0),
        )));
        assert_eq!(loaded_at(&scene, [WORLD_BORDER, 0, 3]), Loaded::OutOfWorld);
        assert_eq!(
            loaded_at(&scene, [WORLD_BORDER + 15, 0, 3]),
            Loaded::OutOfWorld
        );
    }

    fn edit(kind: EditKind, from: [i32; 3], to: [i32; 3], block: &str) -> BlockEdit {
        BlockEdit {
            kind,
            dimension: minecraft_commands::OVERWORLD.to_owned(),
            from,
            to,
            block: BlockInput {
                id: block.to_owned(),
                ..BlockInput::default()
            },
            mode: FillMode::Replace,
            filter: None,
            strict: false,
        }
    }

    #[test]
    fn setblock_and_fill_run_on_the_level_like_vanilla() {
        let Some(stream) = crate::commands::tests::test_stream() else {
            return;
        };
        let registries = stream.states.registries().clone();
        let mut sim = ServerSim::new(
            stream.world_gen(),
            stream.states.clone(),
            "minecraft:overworld",
        );
        let stone = state_of(&registries, "minecraft:stone", &[]).unwrap();
        let mut chunk = Chunk::new(ChunkPos::new(0, 0), -64, 384, BiomeId(0));
        for x in 0..16 {
            for z in 0..16 {
                chunk.set_block_raw(x, 63, z, stone, &registries);
            }
        }
        sim.load_chunk(&chunk);
        let mut blocks = SimBlocks(&mut sim);
        let id = |blocks: &SimBlocks, pos: [i32; 3]| blocks.state(pos).id;
        let gold = edit(
            EditKind::SetBlock,
            [2, 64, 2],
            [2, 64, 2],
            "minecraft:gold_block",
        );
        assert_eq!(minecraft_commands::apply_edit(&gold, &mut blocks), 1);
        assert_eq!(id(&blocks, [2, 64, 2]), "minecraft:gold_block");
        assert_eq!(minecraft_commands::apply_edit(&gold, &mut blocks), 0);
        let keep = BlockEdit {
            filter: Some(BlockFilter::Air),
            ..edit(
                EditKind::SetBlock,
                [2, 64, 2],
                [2, 64, 2],
                "minecraft:stone",
            )
        };
        assert_eq!(minecraft_commands::apply_edit(&keep, &mut blocks), 0);
        let destroy = BlockEdit {
            mode: FillMode::Destroy,
            ..edit(
                EditKind::SetBlock,
                [2, 64, 2],
                [2, 64, 2],
                "minecraft:stone",
            )
        };
        assert_eq!(minecraft_commands::apply_edit(&destroy, &mut blocks), 1);
        assert_eq!(id(&blocks, [2, 64, 2]), "minecraft:stone");
        let fence = edit(
            EditKind::Fill,
            [5, 64, 5],
            [7, 64, 5],
            "minecraft:oak_fence",
        );
        assert_eq!(minecraft_commands::apply_edit(&fence, &mut blocks), 3);
        let middle = blocks.state([6, 64, 5]);
        let side = |name: &str| {
            middle
                .properties
                .iter()
                .find(|(known, _)| known == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(side("east").as_deref(), Some("true"));
        assert_eq!(side("west").as_deref(), Some("true"));
        let strict = BlockEdit {
            strict: true,
            ..edit(
                EditKind::Fill,
                [5, 64, 8],
                [7, 64, 8],
                "minecraft:oak_fence",
            )
        };
        assert_eq!(minecraft_commands::apply_edit(&strict, &mut blocks), 3);
        assert!(
            blocks
                .state([6, 64, 8])
                .properties
                .contains(&("east".to_owned(), "false".to_owned()))
        );
        let hollow = BlockEdit {
            mode: FillMode::Hollow,
            ..edit(EditKind::Fill, [0, 64, 10], [4, 68, 14], "minecraft:glass")
        };
        assert_eq!(minecraft_commands::apply_edit(&hollow, &mut blocks), 98);
        assert_eq!(id(&blocks, [2, 66, 12]), "minecraft:air");
        assert_eq!(minecraft_commands::apply_edit(&hollow, &mut blocks), 0);
        assert!(!blocks.block_entity_matches([2, 64, 2], "{}"));
        assert_eq!(id(&blocks, [2, 400, 2]), "minecraft:void_air");
        sim.take_changes();
        let destroyed = sim.take_destroyed();
        assert_eq!(destroyed.len(), 1);
        assert_eq!(destroyed[0].0, (2, 64, 2));
        assert!(
            sim.items()
                .iter()
                .any(|item| item.item == "minecraft:gold_block")
        );
    }
}
