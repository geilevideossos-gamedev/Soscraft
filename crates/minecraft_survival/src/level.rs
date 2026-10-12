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
    pub container_ticks: u32,
    pub shots: Shots,
    pub sleep_status: crate::sleep::SleepStatus,
}

pub type Shots = std::collections::HashMap<(i32, i32, i32), (f32, f64)>;

pub fn shot_cracks(shots: &Shots) -> Vec<((i32, i32, i32), u8)> {
    let mut cracks: Vec<_> = shots
        .iter()
        .map(|(&pos, &(progress, _))| (pos, ((progress * 10.0) as i32).clamp(0, 9) as u8))
        .collect();
    cracks.sort_unstable();
    cracks
}

pub struct Terrain {
    pub stream: TerrainStream,
    pub scene: HandcraftedScene,
    pub environment: DimensionEnvironment,
    pub rules: DimensionRules,
    pub friend_changes: Option<Vec<(i32, i32, i32)>>,
    pub friend_breaks: Option<Vec<([i32; 3], u32)>>,
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
            container_ticks: 0,
            shots: Shots::new(),
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
            friend_changes: None,
            friend_breaks: None,
        })
    }

    pub fn remote(
        registries: Arc<Registries>,
        seed: i64,
        view_distance: i32,
        dimension: Dimension,
    ) -> Result<Self, String> {
        let stream = TerrainStream::remote(registries.clone(), seed, dimension, view_distance)
            .map_err(|error| error.to_string())?;
        let scene = HandcraftedScene::streamed(stream.states.clone());
        let environment = DimensionEnvironment::load(&registries, dimension.dimension_type())?;
        let rules = DimensionRules::load(&registries, dimension)?;
        Ok(Self {
            stream,
            scene,
            environment,
            rules,
            friend_changes: None,
            friend_breaks: None,
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
        entities.view.world_items.drop_item_stack(stack, pos);
    }
    let mut points = experience.floor() as i32;
    if experience.fract() > 0.0 && entities.random_float() < experience.fract() {
        points += 1;
    }
    if points > 0 {
        let centre = [
            f64::from(pos.0) + 0.5,
            f64::from(pos.1) + 0.5,
            f64::from(pos.2) + 0.5,
        ];
        entities.award_experience(centre, points);
    }
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
                .view.world_items
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

pub fn player_blocks<'a>(
    players: impl IntoIterator<Item = &'a crate::player::Player>,
) -> Vec<(Dimension, (i32, i32, i32))> {
    players
        .into_iter()
        .filter_map(|player| {
            let place = player
                .arrival
                .or(player.place)
                .filter(|place| place.dimension == player.dimension)?;
            let [x, y, z] = place.feet.map(|v| v.floor() as i32);
            Some((player.dimension, (x, y, z)))
        })
        .collect()
}

pub fn blocks_in(
    players: &[(Dimension, (i32, i32, i32))],
    dimension: Dimension,
) -> Vec<(i32, i32, i32)> {
    players
        .iter()
        .filter(|(at, _)| *at == dimension)
        .map(|&(_, block)| block)
        .collect()
}

pub fn stream_tick(
    terrain: &mut Terrain,
    entities: &mut Option<Entities>,
    containers: &mut Containers,
    players: &[(i32, i32, i32)],
    local: Option<(i32, i32, i32)>,
) -> (Vec<Arc<Chunk>>, Vec<ChunkPos>) {
    let changed = terrain.scene.take_changed();
    terrain.stream.record_edits(&terrain.scene, &changed);
    if let Some(friend_changes) = terrain.friend_changes.as_mut() {
        friend_changes.extend_from_slice(&changed);
    }
    save_block_entities(containers, terrain);
    if let Some(entities) = entities.as_mut() {
        save_server_block_entities(entities, terrain, false);
    }
    let (loaded, forgotten) = terrain
        .stream
        .server_tick(players, local, &mut terrain.scene);
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
            entities.view.world_items.spawn_block_drop(stack, pos);
        }
        let lit = |pos| Scene::block(&terrain.scene, pos).map(minecraft_terrain::campfire::is_lit);
        for (pos, stack) in containers.cook(lit, &entities.recipes) {
            entities.view.world_items.drop_item_stack(stack, pos);
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
    pub trapped: Option<((i32, i32, i32), u32)>,
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
        trapped: Scene::block(scene, event.pos)
            .filter(|block| block.id.key() == "minecraft:trapped_chest")
            .map(|_| (event.pos, event.count)),
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
    let events = level.openers.leave(opener_key(player));
    let mut writes: Writes = Vec::new();
    for event in &events {
        let outcome = opener_outcome(&level.terrain.scene, event);
        if let Some(entities) = level.entities.as_mut() {
            let (cell, id, count) = outcome.block_event;
            entities.lid_event(&level.terrain.scene, cell, id, count);
            if let Some((pos, viewers)) = outcome.trapped {
                entities.chest_openers(pos, viewers);
            }
            if let Some(sound) = outcome.sound {
                let pitch = minecraft_terrain::openers::pitch(entities.random_float());
                let at = glam::DVec3::from_array(sound.position);
                entities.sound(sound.event, at, sound.volume, pitch);
            }
        }
        writes.extend(outcome.state.map(|(pos, block)| (pos, Some(block))));
    }
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
    fn the_chunk_map_follows_every_player_standing_in_the_level() {
        use crate::player_nbt::Place;
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let player = |chat: &mut minecraft_commands::CommandServer, id: u32, name: &str| {
            let id = minecraft_commands::PlayerId(id);
            chat.join(id, name);
            crate::player::Player::new(
                chat.profile(id).unwrap(),
                minecraft_terrain::game_mode::PlayerMode::new(
                    minecraft_terrain::game_mode::GameMode::Survival,
                ),
                crate::survival::Survival::default(),
                Inventory::default(),
            )
        };
        let at = |dimension, feet| Some(Place::standing(dimension, feet, 0.0, 0.0));
        let mut host = player(&mut chat, 0, "Steve");
        host.dimension = Dimension::End;
        host.place = at(Dimension::End, [100.5, 49.0, -0.5]);
        let mut arriving = player(&mut chat, 1, "Alex");
        arriving.dimension = Dimension::End;
        arriving.place = at(Dimension::Overworld, [8.0, 70.0, 8.0]);
        arriving.arrival = at(Dimension::End, [1700.2, 50.9, 3.7]);
        let mut left_behind = player(&mut chat, 2, "Noor");
        left_behind.dimension = Dimension::End;
        left_behind.place = at(Dimension::Overworld, [8.0, 70.0, 8.0]);
        let mut elsewhere = player(&mut chat, 3, "Kai");
        elsewhere.place = at(Dimension::Overworld, [-30.0, 64.0, 2.0]);
        let everyone = player_blocks([&host, &arriving, &left_behind, &elsewhere]);
        assert_eq!(
            everyone,
            vec![
                (Dimension::End, (100, 49, -1)),
                (Dimension::End, (1700, 50, 3)),
                (Dimension::Overworld, (-30, 64, 2)),
            ]
        );
        assert_eq!(
            blocks_in(&everyone, Dimension::End),
            vec![(100, 49, -1), (1700, 50, 3)]
        );
        assert_eq!(
            blocks_in(&everyone, Dimension::Overworld),
            vec![(-30, 64, 2)]
        );
        assert!(blocks_in(&everyone, Dimension::Nether).is_empty());
    }

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
    fn a_double_trapped_chest_counts_on_both_halves() {
        let mut scene = HandcraftedScene::default();
        let half = |kind: &str| {
            Block::new("minecraft:trapped_chest")
                .with("facing", "north")
                .with("type", kind)
                .with("waterlogged", "false")
        };
        scene.set((0, 64, 0), Some(half("left")));
        scene.set((1, 64, 0), Some(half("right")));
        scene.set((5, 64, 0), Some(Block::new("minecraft:chest").with("facing", "north")));
        let mut openers = Openers::default();
        let cells = [(0, 64, 0), (1, 64, 0)];
        let trapped = |events: &[OpenerEvent]| -> Vec<_> {
            events
                .iter()
                .map(|event| opener_outcome(&scene, event).trapped)
                .collect()
        };
        let opened = toggle_openers(&mut openers, &scene, &cells, &host(), true, false);
        assert_eq!(trapped(&opened), [Some(((0, 64, 0), 1)), Some(((1, 64, 0), 1))]);
        let joined = toggle_openers(&mut openers, &scene, &cells, &friend(), true, false);
        assert_eq!(trapped(&joined), [Some(((0, 64, 0), 2)), Some(((1, 64, 0), 2))]);
        let left = openers.leave(opener_key(&host()));
        assert_eq!(trapped(&left), [Some(((0, 64, 0), 1)), Some(((1, 64, 0), 1))]);
        let closed = toggle_openers(&mut openers, &scene, &cells, &friend(), false, false);
        assert_eq!(trapped(&closed), [Some(((0, 64, 0), 0)), Some(((1, 64, 0), 0))]);
        let plain = toggle_openers(&mut openers, &scene, &[(5, 64, 0)], &host(), true, false);
        assert_eq!(trapped(&plain), [None]);
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
    fn a_chest_lid_closes_when_its_last_player_leaves_the_world() {
        let Some(mut level) =
            crate::commands::tests::test_stream().and_then(crate::commands::tests::test_level)
        else {
            return;
        };
        level.entities = Some(Entities::new(
            &level.terrain.stream,
            Dimension::Overworld,
            0,
            &crate::data::GameData::default(),
            None,
            None,
        ));
        level.terrain.scene = HandcraftedScene::default();
        let chest = (3, 64, 3);
        let block = Block::new("minecraft:chest")
            .with("facing", "north")
            .with("type", "single");
        level.terrain.scene.set(chest, Some(block));
        for player in [host(), friend()] {
            let opened = toggle_openers(
                &mut level.openers,
                &level.terrain.scene,
                &[chest],
                &player,
                true,
                false,
            );
            let entities = level.entities.as_mut().unwrap();
            for event in opened {
                let (cell, id, count) = event.block_event();
                entities
                    .view.lids
                    .block_event(&level.terrain.scene, cell, id, count);
            }
        }
        let settle = |level: &mut Level| {
            let entities = level.entities.as_mut().unwrap();
            for _ in 0..20 {
                let _ = entities.view.lids.take_apart_changes();
                entities.view.lids.settle(&level.terrain.scene, |_| true);
                entities.view.lids.tick();
            }
            entities.view.lids.lid_closed(chest)
        };
        assert!(!settle(&mut level), "both players opened it");
        leave(&mut level, &friend());
        assert!(!settle(&mut level), "the host still has it open");
        leave(&mut level, &host());
        assert!(settle(&mut level), "the last player left");
        let heard: Vec<&str> = level
            .entities
            .as_ref()
            .unwrap()
            .view.sounds
            .iter()
            .map(|sound| sound.0.as_str())
            .collect();
        assert_eq!(heard, ["minecraft:block.chest.close"]);
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

    #[test]
    fn a_chunk_sent_after_edits_from_every_source_matches_the_scene() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let mut terrain =
            Terrain::open(Arc::new(registries), 0, 2, Dimension::End, None, None).unwrap();
        let (mut entities, mut containers) = (None, Containers::default());
        let started = std::time::Instant::now();
        while [(6, 0), (6, 1), (7, 0), (7, 1)]
            .iter()
            .any(|&chunk| terrain.scene.generated_chunk(chunk).is_none())
        {
            assert!(
                started.elapsed().as_secs() < 600,
                "no chunks near the platform"
            );
            stream_tick(
                &mut terrain,
                &mut entities,
                &mut containers,
                &[(100, 49, 0)],
                Some((100, 49, 0)),
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(terrain.scene.take_changed().is_empty());
        let solid = (96..112)
            .flat_map(|x| (0..256).rev().map(move |y| (x, y, 0)))
            .find(|&pos| {
                terrain
                    .scene
                    .state_at(pos)
                    .is_some_and(|state| state != minecraftoss_core::BlockStateId::AIR)
            })
            .expect("End stone near the platform");
        let placed = (101, 50, 1);
        minecraftoss_player::World::set_block(
            &mut terrain.scene,
            placed,
            Some(minecraftoss_player::Block::new("minecraft:oak_planks")),
        );
        minecraftoss_player::World::set_block(&mut terrain.scene, solid, None);
        terrain.scene.set(
            (102, 51, 2),
            Some(
                Block::new("minecraft:chest")
                    .with("facing", "north")
                    .with("type", "single")
                    .with("waterlogged", "false"),
            ),
        );
        let crater = (solid.0, solid.1 - 3, solid.2 + 4);
        for dx in -2..=2 {
            for dy in -2..=2 {
                for dz in -2..=2 {
                    if dx * dx + dy * dy + dz * dz <= 4 {
                        terrain
                            .scene
                            .set((crater.0 + dx, crater.1 + dy, crater.2 + dz), None);
                    }
                }
            }
        }
        for (x, level) in [(104, "0"), (105, "1"), (106, "2")] {
            terrain.scene.set(
                (x, 52, 4),
                Some(Block::new("minecraft:water").with("level", level)),
            );
        }
        for x in 110..114 {
            for y in 55..58 {
                for z in 14..18 {
                    terrain
                        .scene
                        .set((x, y, z), Some(Block::new("minecraft:stone")));
                }
            }
        }
        let sent = |terrain: &mut Terrain, (x, z): (i32, i32)| {
            let chunk = terrain
                .stream
                .loaded_chunk(ChunkPos::new(x, z))
                .expect("a chunk the friend was sent");
            let blob = minecraft_terrain::chunk_wire::encode_chunk(
                &chunk,
                terrain.stream.states.registries(),
            );
            minecraft_terrain::chunk_wire::decode_chunk([x, z], &blob, &terrain.stream.states)
                .unwrap()
        };
        let planks = terrain.scene.state_at(placed);
        let before = sent(&mut terrain, (6, 0));
        assert_ne!(
            Some(before.block(5, 50, 1)),
            planks,
            "nothing told the chunk map yet"
        );
        stream_tick(
            &mut terrain,
            &mut entities,
            &mut containers,
            &[(100, 49, 0)],
            Some((100, 49, 0)),
        );
        assert!(
            terrain.scene.take_changed().is_empty(),
            "the tick drained the log"
        );
        for (cx, cz) in [(6, 0), (6, 1), (7, 0), (7, 1)] {
            let chunk = sent(&mut terrain, (cx, cz));
            for y in 0..256 {
                for z in 0..16 {
                    for x in 0..16 {
                        let pos = (cx * 16 + x, y, cz * 16 + z);
                        assert_eq!(
                            Some(chunk.block(x as usize, y, z as usize)),
                            terrain.scene.state_at(pos),
                            "{pos:?}"
                        );
                    }
                }
            }
        }
    }

    fn tick_until(
        entities: &mut Entities,
        inventory: &mut Inventory,
        feet: glam::DVec3,
        mut done: impl FnMut(&Entities) -> bool,
    ) {
        let started = std::time::Instant::now();
        while !done(entities) {
            assert!(started.elapsed().as_secs() < 300, "no answer from the level");
            let batch = crate::tick_rate::TickBatch {
                normal: if entities.ready_for_ticks() {
                    vec![true]
                } else {
                    Vec::new()
                },
                overload: None,
            };
            let _ = entities.tick(
                &mut [crate::entities::TickActor {
                    id: 0,
                    view: crate::entities::PlayerView {
                        feet: feet.to_array(),
                        alive: true,
                        attackable: true,
                        spectator: false,
                        picks_up: false,
                        health: 20.0,
                        yaw: 0.0,
                        pitch: -90.0,
                        main_hand: None,
                        offhand: None,
                        sneaking: false,
                    },
                    inventory: &mut *inventory,
                    selected: 0,
                }],
                0.05,
                &batch,
                crate::tick_rate::ClientTiming::default(),
                minecraft_terrain::server::ClockTicks::default(),
                true,
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn an_eye_thrown_from_the_hand_leaves_the_stack_and_flies() {
        use minecraft_terrain::bucket::Change;
        use minecraft_terrain::ender_eye::{ENDER_EYE, throw_use};
        use minecraftoss_player::inventory::ItemStack;
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let Ok(stream) = TerrainStream::for_dimension(
            Arc::new(registries),
            4986579828511054607,
            2,
            Dimension::Overworld,
            None,
        ) else {
            return;
        };
        minecraft_terrain::locate::resolve_rings(&stream.world_gen());
        let mut entities = Entities::new(
            &stream,
            Dimension::Overworld,
            4986579828511054607,
            &crate::data::GameData::default(),
            None,
            None,
        );
        let scene = HandcraftedScene::streamed(stream.states.clone());
        let feet = glam::DVec3::new(-300.5, 80.0, -1700.5);
        let mut player = minecraftoss_player::Player::new(feet);
        player.pitch = -90.0;
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new(ENDER_EYE, 16));
        let held = inventory.slots[0].clone().unwrap();
        let used = throw_use(&player, &minecraft_terrain::doors::Outlined(&scene), &held)
            .expect("an eye in the hand has a use");
        let Change::ThrowEye(throw) = used.change.clone() else {
            panic!("aiming at the sky throws: {:?}", used.change);
        };
        assert!(used.consumed);
        let sent = SentItemUse {
            slot: 0,
            held: held.id.clone(),
            eye: feet + glam::DVec3::Y * 1.62,
            yaw: 0.0,
            pitch: -90.0,
            outcome: used,
        };
        entities.throw_eye(0, throw, sent);
        assert!(entities.awaits_item_use(0));
        tick_until(&mut entities, &mut inventory, feet, |entities| {
            entities
                .player_outputs(0)
                .is_some_and(|outputs| !outputs.item_uses_done.is_empty())
        });
        assert!(!entities.awaits_item_use(0));
        let done = entities.take_player_outputs(0).item_uses_done;
        assert_eq!(done.len(), 1);
        for used in done {
            let heard = finish_item_use(&mut entities, &mut inventory, used, true);
            assert!(heard.is_empty());
        }
        assert_eq!(
            inventory.slots[0].as_ref().map(|stack| stack.count),
            Some(15)
        );
        tick_until(&mut entities, &mut inventory, feet, |entities| {
            entities.eye_views().iter().any(|view| view.age >= 5)
        });
        let views = entities.eye_views();
        assert_eq!(views.len(), 1);
        assert!(views[0].position.y > feet.y + 0.9, "{:?}", views[0]);
        assert!(
            views[0].position.x < feet.x && views[0].position.z < feet.z,
            "{:?}",
            views[0]
        );
    }
}
