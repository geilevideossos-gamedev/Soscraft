use std::collections::{BTreeMap, BTreeSet, HashMap};

use minecraft_commands::{Clientbound, Outgoing, Recipient};
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_survival::heard::{Heard, Said};
use minecraft_survival::level::Level;
use minecraft_survival::player::Player;
use minecraft_survival::server::{Globals, Server};
use minecraft_survival::tick_rate::TickBatch;
use minecraft_terrain::containers::Station;
use minecraft_terrain::mesh::Atlas;
use minecraft_terrain::mob_tracking::Watcher;
use minecraft_terrain::openers::Openers;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::Scene;
use minecraftoss_core::BlockStateId;
use sim::voxel::VoxelEvent;

use crate::minecraft_mining::{Mined, Mining};
use crate::minecraft_world::{VIEW_DISTANCE, event_level, minecraft_yaw};

type Shapes = HashMap<BlockStateId, u16>;
type ShapeIds = HashMap<(Vec<[u32; 6]>, u16), u16>;
type BlockPos = (i32, i32, i32);
type Players = BTreeMap<PlayerUuid, Player>;

const BREW_SOUND: &str = "minecraft:block.brewing_stand.brew";
const EXPLODE_SOUND: &str = "minecraft:entity.generic.explode";

pub(crate) struct Background<'a> {
    pub(crate) origins: &'a HashMap<Dimension, [f64; 3]>,
    pub(crate) host: (&'a PlayerUuid, Option<Dimension>),
    pub(crate) ticked: Option<Dimension>,
    pub(crate) mining: &'a mut Mining,
    pub(crate) minings: &'a mut HashMap<Dimension, Mining>,
    pub(crate) shapes: &'a mut Shapes,
    pub(crate) shape_ids: &'a mut ShapeIds,
    pub(crate) packs: &'a PackStack,
    pub(crate) atlas: &'a Atlas,
    pub(crate) dt: f64,
    pub(crate) now: f64,
}

#[derive(Clone, Copy)]
struct Context<'c> {
    origin: [f64; 3],
    local: &'c PlayerUuid,
    shown: Option<Dimension>,
    packs: &'c PackStack,
    atlas: &'c Atlas,
    dt: f64,
    now: f64,
}

pub(crate) fn unticked<L>(
    players: &Players,
    levels: &BTreeMap<Dimension, L>,
    ticked: Option<Dimension>,
) -> Vec<Dimension> {
    players
        .values()
        .filter(|player| player.switching.is_none())
        .map(|player| player.dimension)
        .filter(|dimension| Some(*dimension) != ticked && levels.contains_key(dimension))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn tick_levels(
    server: &mut Server,
    authority: &mut sim::SimWorld,
    background: Background<'_>,
    mut events: Vec<VoxelEvent>,
) -> Vec<Outgoing> {
    let Background {
        origins,
        host: (local, shown),
        ticked,
        mining,
        minings,
        shapes,
        shape_ids,
        packs,
        atlas,
        dt,
        now,
    } = background;
    let served = server
        .players
        .get(local)
        .filter(|player| player.switching.is_none())
        .map(|player| player.dimension);
    let mut out = Vec::new();
    for dimension in unticked(&server.players, &server.levels, ticked) {
        let band = minecraft_survival::bands::level(dimension);
        let (here, rest): (Vec<_>, Vec<_>) = events
            .into_iter()
            .partition(|event| event_level(event) == band);
        events = rest;
        let Some(&origin) = origins.get(&dimension) else {
            continue;
        };
        let Server {
            globals,
            levels,
            players,
            published,
            ..
        } = &mut *server;
        let Some(level) = levels.get_mut(&dimension) else {
            continue;
        };
        if served != Some(dimension) {
            let ticks = level.pending_batch.as_ref().map_or(0, TickBatch::len);
            for _ in 0..ticks {
                let slept = minecraft_survival::sleep::tick(globals, level, players, *published);
                let shaping = (&mut *shapes, &mut *shape_ids, band);
                let registries = &globals.registries;
                crate::minecraft_world::refresh_shapes(
                    &level.terrain,
                    registries,
                    shaping,
                    &slept.written,
                );
                out.extend(told(players, slept.messages));
            }
        }
        let mining =
            crate::minecraft_players::mining_in((&mut *mining, shown), &mut *minings, dimension);
        let context = Context {
            origin,
            local,
            shown,
            packs,
            atlas,
            dt,
            now,
        };
        let world = (&mut *globals, &mut *players, &mut *authority);
        let shaping = (&mut *shapes, &mut *shape_ids);
        out.extend(tick_level(level, world, shaping, mining, context, here));
    }
    out
}

fn told(players: &Players, messages: Vec<(PlayerUuid, Clientbound)>) -> Vec<Outgoing> {
    messages
        .into_iter()
        .filter_map(|(uuid, message)| {
            let to = Recipient::Player(players.get(&uuid)?.id);
            Some(Outgoing { to, message })
        })
        .collect()
}

fn centre(pos: BlockPos) -> [f64; 3] {
    [
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    ]
}

fn block_of(feet: [f64; 3]) -> BlockPos {
    let [x, y, z] = feet.map(|v| v.floor() as i32);
    (x, y, z)
}

struct Friend {
    id: u64,
    builds: bool,
    infinite: bool,
}

fn friends_in(players: &Players, dimension: Dimension, local: &PlayerUuid) -> Vec<Friend> {
    players
        .values()
        .filter(|player| player.dimension == dimension && player.uuid != *local)
        .map(|player| Friend {
            id: u64::from(player.id.0),
            builds: player.mode.may_build(),
            infinite: !player.mode.consumes_items(),
        })
        .collect()
}

fn tick_level(
    level: &mut Level,
    (globals, players, authority): (&mut Globals, &mut Players, &mut sim::SimWorld),
    (shapes, shape_ids): (&mut Shapes, &mut ShapeIds),
    mining: &mut Mining,
    context: Context<'_>,
    events: Vec<VoxelEvent>,
) -> Vec<Outgoing> {
    let mut out = Vec::new();
    let dimension = level.dimension;
    let band = minecraft_survival::bands::level(dimension);
    let registries = globals.registries.clone();
    let everyone = minecraft_survival::level::player_blocks(players.values());
    let here = minecraft_survival::level::blocks_in(&everyone, dimension);
    let shown_at = players
        .get(context.local)
        .filter(|_| context.shown == Some(dimension))
        .and_then(|player| player.arrival.or(player.place))
        .filter(|place| place.dimension == dimension)
        .map(|place| block_of(place.feet));
    let streamed = minecraft_survival::level::stream_tick(
        &mut level.terrain,
        &mut level.entities,
        &mut level.containers,
        &here,
        shown_at,
    );
    let shaping = (&mut *shapes, &mut *shape_ids, band);
    crate::minecraft_world::apply_streamed(&registries, shaping, streamed);
    if let Some(entities) = level.entities.as_mut() {
        let ticks = std::mem::take(&mut level.container_ticks);
        let (writes, brewed) = minecraft_survival::level::tick_containers(
            &mut level.containers,
            entities,
            &level.terrain,
            ticks,
        );
        for pos in brewed {
            entities.hear(Said::all(Heard::sound(BREW_SOUND, centre(pos), 1.0, 1.0)));
        }
        if !writes.is_empty() {
            let shaping = (&mut *shapes, &mut *shape_ids, band);
            crate::minecraft_world::write_blocks(
                &mut level.terrain,
                &registries,
                shaping,
                entities,
                writes,
            );
        }
    }
    let friends = friends_in(players, dimension, context.local);
    let credit = |attacker: Option<u32>| {
        attacker
            .map(u64::from)
            .filter(|id| friends.iter().any(|friend| friend.id == *id))
    };
    let builds = |id: u64| {
        friends
            .iter()
            .any(|friend| friend.id == id && friend.builds)
    };
    let infinite = |id: Option<u64>| {
        id.is_some_and(|id| {
            friends
                .iter()
                .any(|friend| friend.id == id && friend.infinite)
        })
    };
    let (mob_shots, events): (Vec<_>, Vec<_>) = events.into_iter().partition(|event| {
        matches!(
            event,
            VoxelEvent::MobShot { .. } | VoxelEvent::MobBlast { .. }
        )
    });
    let mut events: Vec<VoxelEvent> = events
        .into_iter()
        .filter(|event| match *event {
            VoxelEvent::Shot { attacker, .. } | VoxelEvent::Ray { attacker, .. } => {
                credit(attacker).is_some_and(builds)
            }
            _ => true,
        })
        .collect();
    if let Some(entities) = level.entities.as_mut() {
        let scene = &level.terrain.scene;
        events.retain(|event| match *event {
            VoxelEvent::Shot { block, .. } => {
                let pos = (block[0], block[1], block[2]);
                let egg =
                    Scene::block(scene, pos).is_some_and(minecraft_terrain::dragon_egg::is_egg);
                if egg {
                    entities.attack_block(scene, pos);
                }
                !egg
            }
            _ => true,
        });
        for shot in mob_shots {
            match shot {
                VoxelEvent::MobShot {
                    key,
                    damage,
                    from,
                    attacker,
                    ..
                } => {
                    if let (Some(id), Some(client)) = (credit(attacker), attacker) {
                        let yaw = authority
                            .player(sim::ClientId(client))
                            .map_or(0.0, |ps| minecraft_yaw(ps.viewangles[1]));
                        entities.shoot(id, key, damage, from, yaw, infinite(Some(id)));
                    }
                }
                VoxelEvent::MobBlast {
                    key,
                    damage,
                    center,
                    impact,
                    attacker,
                    slow_ticks,
                    ..
                } => {
                    let by = credit(attacker);
                    entities.blast(key, damage, center, impact, by, slow_ticks, infinite(by));
                }
                _ => {}
            }
        }
        for event in &events {
            match *event {
                VoxelEvent::Explosion { center, power, .. } => {
                    entities.weapon_blast(center, power);
                    let spread = entities.random_float() - entities.random_float();
                    let pitch = (1.0 + spread * 0.2) * 0.7;
                    let heard = Heard::sound(EXPLODE_SOUND, center, 4.0, pitch);
                    entities.hear(Said::all(heard));
                }
                VoxelEvent::Shot { block, .. } => {
                    let pos = (block[0], block[1], block[2]);
                    if let Some(kind) =
                        Scene::block(scene, pos).and_then(|block| scene.sound_type(block))
                    {
                        let volume = (kind.volume + 1.0) / 4.0;
                        let heard = Heard::sound(&kind.hit, centre(pos), volume, kind.pitch * 0.5);
                        entities.hear(Said::all(heard));
                    }
                }
                _ => {}
            }
        }
    }
    let mut refs = crate::minecraft_mining::WorldRefs {
        stream: &mut level.terrain.stream,
        scene: &mut level.terrain.scene,
        packs: context.packs,
        atlas: context.atlas,
        registries: &registries,
        level: band,
    };
    let mined = mining.apply(events, &mut level.shots, &mut refs, context.now);
    let mut by_friend: Vec<(u64, Vec<Mined>)> = Vec::new();
    for (broken, attacker) in mined {
        if let Some(entities) = level.entities.as_mut().filter(|_| broken.2.is_none())
            && let Some(kind) = level.terrain.scene.sound_type(&broken.1)
        {
            let volume = (kind.volume + 1.0) / 2.0;
            let heard = Heard::sound(
                &kind.break_sound,
                centre(broken.0),
                volume,
                kind.pitch * 0.8,
            );
            entities.hear(Said::all(heard));
        }
        let Some(id) = credit(attacker) else {
            continue;
        };
        match by_friend.iter_mut().find(|(friend, _)| *friend == id) {
            Some((_, blocks)) => blocks.push(broken),
            None => by_friend.push((id, vec![broken])),
        }
    }
    for (id, broken) in by_friend {
        let Some(friend) = players
            .values_mut()
            .find(|player| u64::from(player.id.0) == id)
        else {
            continue;
        };
        let at = authority
            .player(sim::ClientId(friend.id.0))
            .map(|ps| sim::voxel::to_block(context.origin, ps.origin))
            .or_else(|| friend.place.map(|place| place.feet))
            .unwrap_or(context.origin);
        let _ = minecraft_survival::actions::harvest(
            friend,
            level,
            globals,
            (broken, Vec::new()),
            &[],
            at,
        );
    }
    let batch = level.pending_batch.take().unwrap_or(TickBatch {
        normal: Vec::new(),
        overload: None,
    });
    let Level {
        entities,
        containers,
        terrain,
        openers,
        ..
    } = level;
    let Some(entities) = entities.as_mut() else {
        return out;
    };
    let watchers = players
        .values()
        .filter(|player| player.dimension == dimension && player.uuid != *context.local)
        .filter_map(|player| {
            let place = player
                .arrival
                .or(player.place)
                .filter(|place| place.dimension == dimension)?;
            Some(Watcher {
                id: u64::from(player.id.0),
                feet: place.feet,
                view_distance: VIEW_DISTANCE,
            })
        })
        .collect();
    entities.set_watchers(watchers, VIEW_DISTANCE);
    let mut present = Vec::new();
    let mut actors = Vec::new();
    for friend in players
        .values_mut()
        .filter(|player| player.dimension == dimension && player.uuid != *context.local)
    {
        if let Some((actor, uuid)) =
            crate::minecraft_players::tick_actor(friend, authority, context.origin)
        {
            present.push((actor.id, uuid));
            actors.push(actor);
        }
    }
    entities.set_players(&present);
    entities.difficulty = minecraft_commands::difficulty_id(globals.difficulty);
    let weather = entities.weather;
    let clock = globals.default_clock_ticks(dimension);
    let light =
        terrain
            .environment
            .level_sky_light(clock, weather.rain_level, weather.thunder_level);
    let ticks_before = entities.client_ticks();
    let timing = globals.tick_rate.client_timing();
    let clocks = globals.clock_ticks();
    let changes = entities.tick(
        &mut actors,
        context.dt,
        &batch,
        timing,
        clocks,
        light > 11.0,
    );
    drop(actors);
    entities.present_level_events(&terrain.scene);
    globals.weather = entities.weather_data.or(globals.weather);
    let summoned = std::mem::take(&mut entities.command_summons);
    if !summoned.is_empty() {
        let rules = &globals.game_rules;
        out.extend(minecraft_survival::commands::summon_answers(
            &globals.chat,
            rules,
            summoned,
        ));
    }
    let mob_ticks = u32::try_from(entities.client_ticks() - ticks_before).unwrap_or(u32::MAX);
    if !changes.is_empty() {
        let mut positions = Vec::with_capacity(changes.len());
        for (pos, block) in changes {
            let state = block
                .as_ref()
                .and_then(|b| terrain.stream.states.state_of(b));
            let shape = state.map_or(0, |state| {
                crate::minecraft_world::shape_id(shapes, shape_ids, &registries, state)
            });
            sim::voxel::set_block_shape(band, pos.0, pos.1, pos.2, shape);
            let before = minecraftoss_player::World::block(&terrain.scene, pos);
            let shulker = Some(Station::Shulker);
            let shulker_before = Scene::block(&terrain.scene, pos)
                .filter(|b| Station::of(b) == shulker)
                .map(|b| b.id.key());
            terrain.scene.set(pos, block);
            if let Some(id) = shulker_before
                && Scene::block(&terrain.scene, pos).and_then(Station::of) != shulker
                && let Some(components) = containers.take_shulker(pos)
            {
                entities.keep_dropped_contents(pos, &id, &components);
            }
            let after = minecraftoss_player::World::block(&terrain.scene, pos);
            if let (Some(before), Some(after)) = (before, after)
                && let Some(sound) = minecraft_terrain::doors::release_sound(&before, &after)
                    .or_else(|| {
                        minecraft_terrain::doors::powered_toggle(&before, &after)
                            .then(|| minecraft_terrain::doors::toggle_sound(&before))
                            .flatten()
                    })
            {
                let pitch = sound.pitch + entities.random_float() * sound.pitch_spread;
                let heard = Heard::sound(&sound.event, centre(pos), sound.volume, pitch);
                entities.hear(Said::all(heard));
            }
            positions.push(pos);
        }
        for &pos in &positions {
            let block = Scene::block(&terrain.scene, pos);
            if !block.is_some_and(Openers::counted) {
                openers.removed(pos);
            }
            let keeps = block.is_some_and(|b| {
                Station::of(b).is_some() || minecraft_terrain::campfire::is_campfire(b)
            });
            if !keeps {
                minecraft_survival::level::spill(containers, entities, pos);
            }
        }
        terrain.stream.record_edits(&terrain.scene, &positions);
        terrain.stream.mark_edited(&terrain.scene, &positions);
    }
    for (pos, block) in std::mem::take(&mut entities.destroyed) {
        if let Some(state) = terrain.stream.states.state_of(&block) {
            let heard = Heard::Event {
                event: minecraft_survival::session::BREAK_EVENT,
                pos: [pos.0, pos.1, pos.2],
                data: i32::from(state.0),
            };
            entities.hear(Said::all(heard));
        }
        if let Some(kind) = terrain.scene.sound_type(&block) {
            let volume = (kind.volume + 1.0) / 2.0;
            let heard = Heard::sound(&kind.break_sound, centre(pos), volume, kind.pitch * 0.8);
            entities.hear(Said::all(heard));
        }
    }
    sim::voxel::set_mob_boxes(band, entities.shot_boxes(None));
    entities.view.tick_scene(&terrain.scene, mob_ticks);
    for (pos, apart) in entities.view.lids.take_apart_changes() {
        terrain.stream.draw_apart(pos, apart);
    }
    entities.view.sounds.clear();
    entities.view.item_crumbs.clear();
    entities.view.potion_crumbs.clear();
    entities.broken_items.clear();
    mining.tick(&terrain.scene, context.dt);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_commands::{CommandServer, PlayerId};

    fn player(chat: &mut CommandServer, id: u32, name: &str, dimension: Dimension) -> Player {
        chat.join(PlayerId(id), name);
        let mut player = Player::new(
            chat.profile(PlayerId(id)).unwrap(),
            minecraft_terrain::game_mode::PlayerMode::default(),
            minecraft_survival::survival::Survival::default(),
            minecraftoss_player::inventory::Inventory::default(),
        );
        player.dimension = dimension;
        player
    }

    #[test]
    fn every_open_level_with_a_player_in_it_ticks_once_and_the_hosts_own_level_is_left_alone() {
        let mut chat = CommandServer::new(minecraft_survival::commands::HOST);
        let mut players = Players::new();
        for (id, name, dimension) in [
            (0, "Steve", Dimension::Overworld),
            (1, "Alex", Dimension::Nether),
            (2, "Bo", Dimension::Nether),
            (3, "Cy", Dimension::End),
        ] {
            let player = player(&mut chat, id, name, dimension);
            players.insert(player.uuid.clone(), player);
        }
        let levels: BTreeMap<Dimension, ()> = [Dimension::Overworld, Dimension::Nether]
            .into_iter()
            .map(|dimension| (dimension, ()))
            .collect();
        let ticking = |players: &Players, ticked| unticked(players, &levels, ticked);
        assert_eq!(
            ticking(&players, Some(Dimension::Overworld)),
            vec![Dimension::Nether],
            "the Nether once for both friends, the End is not open yet"
        );
        assert_eq!(
            ticking(&players, None),
            vec![Dimension::Overworld, Dimension::Nether],
            "a host frame that returned early leaves its level to the others"
        );
        for player in players
            .values_mut()
            .filter(|player| player.dimension == Dimension::Nether)
        {
            player.switch_to(
                Dimension::Overworld,
                None,
                minecraft_survival::switch::LoadingReason::NetherPortal,
            );
        }
        assert!(
            ticking(&players, Some(Dimension::Overworld)).is_empty(),
            "players on the way are in no level"
        );
    }
}
