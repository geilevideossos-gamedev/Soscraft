use std::sync::Arc;

use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::bucket::Sound;
use minecraft_terrain::containers::{Containers, block_entity_kind};
use minecraft_terrain::environment::DimensionEnvironment;
use minecraft_terrain::openers::{OpenerEvent, OpenerSound, Openers};
use minecraft_terrain::scene::{Block, HandcraftedScene, Scene};
use minecraft_terrain::terrain::TerrainStream;
use minecraftoss_core::{Chunk, ChunkPos, Registries};
use minecraftoss_player::inventory::Inventory;

use crate::dimension::DimensionRules;
use crate::entities::{Entities, SentItemUse};
use crate::save::WorldSave;

type Writes = Vec<((i32, i32, i32), Option<Block>)>;

pub struct Level {
    pub dimension: Dimension,
    pub entities: Option<Entities>,
    pub containers: Containers,
    pub terrain: Terrain,
    pub openers: Openers,
    pub empty_ticks: u32,
    pub dragon_fight: Option<crate::the_end::dragon_fight::DragonFight>,
    pub sleep_status: crate::sleep::SleepStatus,
}

pub struct Terrain {
    pub stream: TerrainStream,
    pub scene: HandcraftedScene,
    pub environment: DimensionEnvironment,
    pub rules: DimensionRules,
}

impl Level {
    pub fn new(dimension: Dimension, terrain: Terrain) -> Self {
        Self {
            dimension,
            entities: None,
            containers: Containers::default(),
            terrain,
            openers: Openers::default(),
            empty_ticks: 0,
            dragon_fight: None,
            sleep_status: crate::sleep::SleepStatus::default(),
        }
    }

    pub fn dark_outside(&self, globals: &crate::server::Globals) -> bool {
        let (rain, thunder) = self.entities.as_ref().map_or((0.0, 0.0), |entities| {
            (entities.weather.rain_level, entities.weather.thunder_level)
        });
        let ticks = globals.default_clock_ticks(self.dimension);
        let light = self.terrain.environment.level_sky_light(ticks, rain, thunder);
        crate::sleep::dark_outside(light, self.terrain.rules.has_fixed_time)
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
        let rules = DimensionRules::load(&registries, dimension)?;
        Ok(Self {
            stream,
            scene,
            environment,
            rules,
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
    if let Some(pos) = minecraft_terrain::bucket::evaporated(&used.outcome) {
        entities.evaporation_smoke(pos);
    }
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
            entities.broken_items.push(used.held.clone());
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

pub fn opener_key(player: &PlayerUuid) -> u128 {
    minecraftoss_entities::gossip::parse_uuid(&player.0).unwrap_or_else(|| {
        player
            .0
            .bytes()
            .fold(0x6c62_272e_07bb_0142_62b8_2175_6295_c58d, |hash, byte| {
                (hash ^ u128::from(byte)).wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b)
            })
    })
}

pub fn toggle_openers(
    openers: &mut Openers,
    scene: &HandcraftedScene,
    cells: &[(i32, i32, i32)],
    player: &PlayerUuid,
    open: bool,
    spectator: bool,
) -> Vec<OpenerEvent> {
    let key = opener_key(player);
    cells
        .iter()
        .filter(|&&pos| Scene::block(scene, pos).is_some_and(Openers::counted))
        .filter_map(|&pos| {
            if open {
                openers.open(pos, key, spectator)
            } else {
                openers.close(pos, key)
            }
        })
        .collect()
}

pub struct OpenerOutcome {
    pub block_event: ((i32, i32, i32), u8, u32),
    pub sound: Option<OpenerSound>,
    pub state: Option<((i32, i32, i32), Block)>,
}

pub fn opener_outcome(scene: &HandcraftedScene, event: &OpenerEvent) -> OpenerOutcome {
    let changed = Scene::block(scene, event.pos).zip(event.change);
    OpenerOutcome {
        block_event: event.block_event(),
        sound: changed.and_then(|(block, change)| {
            minecraft_terrain::openers::sound(block, event.pos, change)
        }),
        state: changed
            .and_then(|(block, change)| minecraft_terrain::openers::opened_state(block, change))
            .map(|state| (event.pos, state)),
    }
}

pub fn leave_containers(
    openers: &mut Openers,
    scene: &HandcraftedScene,
    player: &PlayerUuid,
) -> Writes {
    openers
        .leave(opener_key(player))
        .iter()
        .filter_map(|event| opener_outcome(scene, event).state)
        .map(|(pos, block)| (pos, Some(block)))
        .collect()
}

pub fn leave(level: &mut Level, player: &PlayerUuid) {
    let writes = leave_containers(&mut level.openers, &level.terrain.scene, player);
    let positions: Vec<_> = writes.iter().map(|(pos, _)| *pos).collect();
    for (pos, block) in writes {
        level.terrain.scene.set(pos, block);
    }
    if !positions.is_empty() {
        level
            .terrain
            .stream
            .record_edits(&level.terrain.scene, &positions);
    }
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

    fn host() -> PlayerUuid {
        PlayerUuid("b50ad385-829d-3141-a216-7e7d7539ba7f".to_owned())
    }

    fn friend() -> PlayerUuid {
        PlayerUuid("0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0".to_owned())
    }

    #[test]
    fn a_player_uuid_is_its_vanilla_number() {
        assert_eq!(
            opener_key(&host()),
            0xb50a_d385_829d_3141_a216_7e7d_7539_ba7f
        );
        let odd = PlayerUuid("not a uuid".to_owned());
        assert_eq!(opener_key(&odd), opener_key(&odd));
        assert_ne!(
            opener_key(&odd),
            opener_key(&PlayerUuid("not a uuid either".to_owned()))
        );
    }

    #[test]
    fn a_double_chest_opens_both_halves_and_sounds_once_from_its_middle() {
        let mut scene = HandcraftedScene::default();
        let half = |kind| {
            Block::new("minecraft:chest")
                .with("facing", "south")
                .with("type", kind)
        };
        scene.set((14, 65, 14), Some(half("left")));
        scene.set((13, 65, 14), Some(half("right")));
        let mut openers = Openers::default();
        let cells = [(14, 65, 14), (13, 65, 14)];
        let events = toggle_openers(&mut openers, &scene, &cells, &host(), true, false);
        assert_eq!(events.len(), 2);
        let outcomes: Vec<_> = events.iter().map(|e| opener_outcome(&scene, e)).collect();
        assert_eq!(outcomes[0].block_event, ((14, 65, 14), 1, 1));
        assert_eq!(outcomes[1].block_event, ((13, 65, 14), 1, 1));
        let sounds: Vec<_> = outcomes.iter().filter_map(|o| o.sound).collect();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].event, "minecraft:block.chest.open");
        assert_eq!(sounds[0].position, [14.0, 65.5, 14.5]);
        assert!(outcomes.iter().all(|o| o.state.is_none()));
        let again = toggle_openers(&mut openers, &scene, &cells, &host(), true, false);
        assert!(again.is_empty());
        let shut = toggle_openers(&mut openers, &scene, &cells, &host(), false, false);
        assert_eq!(
            shut.iter().map(|e| e.block_event()).collect::<Vec<_>>(),
            vec![((14, 65, 14), 1, 0), ((13, 65, 14), 1, 0)]
        );
    }

    #[test]
    fn a_second_player_keeps_a_shulker_box_open_and_a_spectator_is_not_counted() {
        let mut scene = HandcraftedScene::default();
        scene.set(
            (0, 64, 0),
            Some(Block::new("minecraft:red_shulker_box").with("facing", "up")),
        );
        let mut openers = Openers::default();
        let cells = [(0, 64, 0)];
        let first = toggle_openers(&mut openers, &scene, &cells, &host(), true, false);
        let sound = opener_outcome(&scene, &first[0]).sound.map(|s| s.event);
        assert_eq!(sound, Some("minecraft:block.shulker_box.open"));
        let second = toggle_openers(&mut openers, &scene, &cells, &friend(), true, false);
        assert_eq!(second[0].block_event(), ((0, 64, 0), 1, 2));
        assert!(opener_outcome(&scene, &second[0]).sound.is_none());
        let ghost = PlayerUuid("11111111-2222-3333-4444-555555555555".to_owned());
        assert!(toggle_openers(&mut openers, &scene, &cells, &ghost, true, true).is_empty());
        let left = toggle_openers(&mut openers, &scene, &cells, &host(), false, false);
        assert_eq!(left[0].block_event(), ((0, 64, 0), 1, 1));
        assert!(opener_outcome(&scene, &left[0]).sound.is_none());
        let last = toggle_openers(&mut openers, &scene, &cells, &friend(), false, false);
        let sound = opener_outcome(&scene, &last[0]).sound.map(|s| s.event);
        assert_eq!(sound, Some("minecraft:block.shulker_box.close"));
    }

    #[test]
    fn furnaces_and_empty_cells_count_no_openers() {
        let mut scene = HandcraftedScene::default();
        scene.set(
            (1, 64, 2),
            Some(Block::new("minecraft:furnace").with("facing", "north")),
        );
        let mut openers = Openers::default();
        let cells = [(1, 64, 2), (2, 64, 2)];
        assert!(toggle_openers(&mut openers, &scene, &cells, &host(), true, false).is_empty());
    }

    #[test]
    fn a_barrel_shows_open_until_its_player_leaves_the_world() {
        let mut scene = HandcraftedScene::default();
        let barrel = Block::new("minecraft:barrel")
            .with("facing", "east")
            .with("open", "false");
        scene.set((3, 64, 3), Some(barrel.clone()));
        let mut openers = Openers::default();
        let opened = toggle_openers(&mut openers, &scene, &[(3, 64, 3)], &host(), true, false);
        let outcome = opener_outcome(&scene, &opened[0]);
        assert_eq!(
            outcome.state,
            Some(((3, 64, 3), barrel.clone().with("open", "true")))
        );
        assert_eq!(outcome.sound.map(|s| s.position), Some([4.0, 64.5, 3.5]));
        scene.set((3, 64, 3), Some(barrel.clone().with("open", "true")));
        assert!(leave_containers(&mut openers, &scene, &friend()).is_empty());
        assert_eq!(
            leave_containers(&mut openers, &scene, &host()),
            vec![((3, 64, 3), Some(barrel))]
        );
        assert_eq!(openers.count((3, 64, 3)), 0);
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
