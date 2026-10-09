use minecraft_net::Dimension;
use minecraft_terrain::bucket::Sound;
use minecraft_terrain::containers::Containers;
use minecraft_terrain::environment::DimensionEnvironment;
use minecraft_terrain::scene::{Block, HandcraftedScene, Scene};
use minecraft_terrain::terrain::TerrainStream;
use minecraftoss_player::inventory::Inventory;

use crate::entities::{Entities, SentItemUse};

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

pub fn write_blocks(
    terrain: &mut Terrain,
    entities: &mut Entities,
    writes: Vec<((i32, i32, i32), Option<Block>)>,
) {
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
