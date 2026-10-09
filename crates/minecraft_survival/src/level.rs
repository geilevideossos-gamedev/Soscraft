use minecraft_net::Dimension;
use minecraft_terrain::bucket::Sound;
use minecraft_terrain::containers::Containers;
use minecraft_terrain::environment::DimensionEnvironment;
use minecraft_terrain::scene::{Block, HandcraftedScene, Scene};
use minecraft_terrain::terrain::TerrainStream;
use minecraftoss_player::inventory::Inventory;

use crate::entities::{Entities, SentItemUse};

type Writes = Vec<((i32, i32, i32), Option<Block>)>;

pub struct Level {
    pub dimension: Dimension,
    pub entities: Option<Entities>,
    pub containers: Containers,
    pub terrain: Terrain,
}

pub struct Terrain {
    pub stream: TerrainStream,
    pub scene: HandcraftedScene,
    pub environment: DimensionEnvironment,
}

pub fn spill(containers: &mut Containers, entities: &mut Entities, pos: (i32, i32, i32)) {
    let (items, experience) = containers.remove(pos);
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
