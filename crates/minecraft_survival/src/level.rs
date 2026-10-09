use std::sync::Arc;

use minecraft_net::Dimension;
use minecraft_terrain::bucket::Sound;
use minecraft_terrain::containers::{Containers, block_entity_kind};
use minecraft_terrain::environment::DimensionEnvironment;
use minecraft_terrain::scene::{Block, HandcraftedScene, Scene};
use minecraft_terrain::terrain::TerrainStream;
use minecraftoss_core::{Chunk, ChunkPos, Registries};
use minecraftoss_player::inventory::Inventory;

use crate::entities::{Entities, SentItemUse};
use crate::save::WorldSave;

type Writes = Vec<((i32, i32, i32), Option<Block>)>;

pub struct Level {
    pub dimension: Dimension,
    pub entities: Option<Entities>,
    pub containers: Containers,
    pub terrain: Terrain,
    pub empty_ticks: u32,
}

pub struct Terrain {
    pub stream: TerrainStream,
    pub scene: HandcraftedScene,
    pub environment: DimensionEnvironment,
}

impl Level {
    pub fn new(dimension: Dimension, terrain: Terrain) -> Self {
        Self {
            dimension,
            entities: None,
            containers: Containers::default(),
            terrain,
            empty_ticks: 0,
        }
    }
}

impl Terrain {
    pub fn open(
        registries: Arc<Registries>,
        seed: i64,
        view_distance: i32,
        dimension: Dimension,
        save: Option<&WorldSave>,
        known_spawn: Option<(i32, i32, i32)>,
    ) -> Result<Self, String> {
        if let Some(save) = save {
            save.create_dimension(dimension)?;
        }
        let stream = TerrainStream::for_dimension_at(
            registries.clone(),
            seed,
            view_distance,
            dimension,
            save.map(WorldSave::path),
            known_spawn,
        )
        .map_err(|error| error.to_string())?;
        let scene = HandcraftedScene::streamed(stream.states.clone());
        let environment = DimensionEnvironment::load(&registries, dimension.dimension_type())?;
        Ok(Self {
            stream,
            scene,
            environment,
        })
    }
}

pub fn open_for_respawn(
    terrain: &Terrain,
    registries: &Registries,
    pos: (i32, i32, i32),
) -> Option<bool> {
    terrain
        .scene
        .generated_chunk((pos.0.div_euclid(16), pos.2.div_euclid(16)))?;
    let Some(block) = Scene::block(&terrain.scene, pos) else {
        return Some(true);
    };
    Some(terrain.stream.states.state_of(block).is_none_or(|state| {
        registries.blocks.collision_boxes(state).is_empty()
            && registries.blocks.state(state).fluid.is_none()
    }))
}

pub fn spill(containers: &mut Containers, entities: &mut Entities, pos: (i32, i32, i32)) {
    let (items, experience) = containers.remove(pos, &entities.recipes);
    for stack in items {
        entities.world_items.drop_item_stack(stack, pos);
    }
    entities.experience_found += experience;
}

pub fn write_blocks(terrain: &mut Terrain, entities: &mut Entities, writes: Writes) {
    let positions: Vec<_> = writes.iter().map(|(p, _)| *p).collect();
    for (at, block) in writes {
        terrain.scene.set(at, block);
    }
    terrain.stream.record_edits(&terrain.scene, &positions);
    terrain.stream.mark_edited(&terrain.scene, &positions);
    for &at in &positions {
        if Scene::block(&terrain.scene, at).is_some() {
            entities.placed(&terrain.scene, at);
        } else {
            entities.broke(&[at]);
        }
    }
}

pub fn finish_item_use(
    entities: &mut Entities,
    inventory: &mut Inventory,
    used: SentItemUse,
    consumes: bool,
) -> Vec<Sound> {
    let mut heard = used.outcome.sounds;
    let spends = consumes || used.outcome.result.is_some();
    if inventory
        .slots
        .get(used.slot)
        .and_then(Option::as_ref)
        .is_some_and(|stack| stack.id == used.held)
    {
        if used.outcome.consumed
            && spends
            && let Some(slot) = inventory.slots.get_mut(used.slot)
            && let Some(stack) = slot.as_mut()
        {
            stack.count -= 1;
            if stack.count == 0 {
                *slot = None;
            }
        }
        if let Some(result) = used.outcome.result
            && let Some(rest) =
                minecraft_terrain::bottle::filled_result(inventory, used.slot, result)
        {
            entities
                .world_items
                .toss(rest, used.eye, used.yaw, used.pitch);
        }
        if spends && inventory.wear_tool(used.slot, used.outcome.wear) {
            heard.push(minecraft_terrain::flint::break_sound(
                entities.random_float(),
            ));
        }
    }
    heard
}

pub fn save_block_entities(containers: &mut Containers, terrain: &mut Terrain) {
    let scene = &terrain.scene;
    let changes =
        containers.take_block_entities(|pos| Scene::block(scene, pos).and_then(block_entity_kind));
    terrain.stream.record_block_entities(&changes);
}

pub fn save_server_block_entities(entities: &mut Entities, terrain: &mut Terrain, closing: bool) {
    let changes = if closing {
        entities.finish_block_entity_changes()
    } else {
        entities.take_block_entity_changes()
    };
    terrain.stream.record_block_entities(&changes);
}

pub fn stream_tick(
    terrain: &mut Terrain,
    entities: &mut Option<Entities>,
    containers: &mut Containers,
    block: (i32, i32, i32),
) -> (Vec<Arc<Chunk>>, Vec<ChunkPos>) {
    save_block_entities(containers, terrain);
    if let Some(entities) = entities.as_mut() {
        save_server_block_entities(entities, terrain, false);
    }
    let (loaded, forgotten) = terrain.stream.server_tick(block, &mut terrain.scene);
    for pos in &forgotten {
        containers.forget_chunk((pos.x, pos.z));
    }
    if let Some(entities) = entities.as_mut() {
        for chunk in &loaded {
            entities.load_chunk(chunk);
            containers.load_chunk(chunk, &entities.recipes);
        }
        for &pos in &forgotten {
            entities.unload_chunk(pos);
        }
    }
    (loaded, forgotten)
}

pub fn tick_containers(
    containers: &mut Containers,
    entities: &mut Entities,
    terrain: &Terrain,
    ticks: u32,
) -> (Writes, Vec<(i32, i32, i32)>) {
    let mut props = Vec::new();
    let mut brewed = Vec::new();
    for _ in 0..ticks {
        let ticked = containers.tick(&entities.recipes);
        props.extend(ticked.props);
        brewed.extend(ticked.brewed);
        for (pos, stack) in ticked.drops {
            entities.world_items.spawn_block_drop(stack, pos);
        }
        let lit = |pos| Scene::block(&terrain.scene, pos).map(minecraft_terrain::campfire::is_lit);
        for (pos, stack) in containers.cook(lit, &entities.recipes) {
            entities.world_items.drop_item_stack(stack, pos);
        }
    }
    (prop_writes(&terrain.scene, props), brewed)
}

fn prop_writes(scene: &HandcraftedScene, props: Vec<((i32, i32, i32), &str, bool)>) -> Writes {
    let mut writes: Writes = Vec::new();
    for (pos, name, on) in props {
        let current = writes
            .iter()
            .rev()
            .find(|(at, _)| *at == pos)
            .and_then(|(_, block)| block.clone())
            .or_else(|| Scene::block(scene, pos).cloned());
        if let Some(block) = current.filter(|block| block.properties.contains_key(name)) {
            writes.push((
                pos,
                Some(block.with(name, if on { "true" } else { "false" })),
            ));
        }
    }
    writes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_furnace_lights_and_goes_out_through_its_lit_property() {
        let mut scene = HandcraftedScene::default();
        let furnace = Block::new("minecraft:furnace")
            .with("facing", "north")
            .with("lit", "false");
        scene.set((1, 64, 2), Some(furnace.clone()));
        let lit = prop_writes(&scene, vec![((1, 64, 2), "lit", true)]);
        assert_eq!(
            lit,
            vec![((1, 64, 2), Some(furnace.clone().with("lit", "true")))]
        );
        let flicker = prop_writes(
            &scene,
            vec![((1, 64, 2), "lit", true), ((1, 64, 2), "lit", false)],
        );
        assert_eq!(flicker.last(), Some(&((1, 64, 2), Some(furnace))));
    }

    #[test]
    fn brewing_stand_bottles_add_up_within_one_tick_batch() {
        let mut scene = HandcraftedScene::default();
        let stand = Block::new("minecraft:brewing_stand")
            .with("has_bottle_0", "false")
            .with("has_bottle_1", "false")
            .with("has_bottle_2", "false");
        scene.set((0, 70, 0), Some(stand.clone()));
        let writes = prop_writes(
            &scene,
            vec![
                ((0, 70, 0), "has_bottle_0", true),
                ((0, 70, 0), "has_bottle_2", true),
            ],
        );
        let last = writes.last().and_then(|(_, block)| block.clone());
        assert_eq!(
            last,
            Some(
                stand
                    .with("has_bottle_0", "true")
                    .with("has_bottle_2", "true")
            )
        );
    }

    #[test]
    fn blocks_without_the_property_and_empty_cells_are_left_alone() {
        let mut scene = HandcraftedScene::default();
        scene.set((5, 64, 5), Some(Block::new("minecraft:cobblestone")));
        let writes = prop_writes(
            &scene,
            vec![((5, 64, 5), "lit", true), ((6, 64, 5), "lit", true)],
        );
        assert!(writes.is_empty());
    }
}
