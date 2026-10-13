use std::collections::BTreeMap;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Instant;

use bevy::prelude::*;
use minecraft_commands::{Outgoing, Recipient, Serverbound};
use minecraft_net::messages::HandState;
use minecraft_net::{Lane, PlayerUuid, ToClient, ToServer};
use minecraft_survival::menus::{Books, MenuInput};
use minecraft_survival::session::{BatchJob, EncodedBatch, Hello, JoinContext, Sessions};
use minecraft_survival::sleep::BedAction;
use minecraftoss_core::Registries;

use crate::minecraft_world::Runtime;

struct Encoded {
    client: u32,
    seq: u32,
    batch: EncodedBatch,
}

type Job = (BatchJob, Arc<Registries>);

struct ChunkWorker {
    jobs: Option<mpsc::Sender<Job>>,
    done: mpsc::Receiver<Encoded>,
    thread: Option<JoinHandle<()>>,
}

impl ChunkWorker {
    fn start() -> Option<Self> {
        let (jobs, receive) = mpsc::channel::<Job>();
        let (send, done) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("minecraft-chunk-send".into())
            .spawn(move || {
                for (job, registries) in receive {
                    let batch = encode(&job, &registries);
                    let encoded = Encoded {
                        client: job.client,
                        seq: job.seq,
                        batch,
                    };
                    if send.send(encoded).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| diag::warn!(World, "Minecraft chunk send thread not started: {error}"))
            .ok()?;
        Some(Self {
            jobs: Some(jobs),
            done,
            thread: Some(thread),
        })
    }
}

impl Drop for ChunkWorker {
    fn drop(&mut self) {
        drop(self.jobs.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

type LecternClicks = Vec<([i32; 3], Option<i32>)>;

fn encode(job: &BatchJob, registries: &Registries) -> EncodedBatch {
    minecraft_survival::session::encode_batch(job.seq, &job.chunks, registries)
}

#[derive(Default)]
pub(crate) struct HostSessions {
    sessions: Sessions,
    waiting: BTreeMap<u32, Hello>,
    worker: Option<ChunkWorker>,
    registry: Option<u64>,
    inline: Vec<Encoded>,
    time_at: Option<Instant>,
    chat: BTreeMap<u32, Vec<Serverbound>>,
    selects: BTreeMap<u32, u8>,
    credits_done: Vec<u32>,
    lecterns: BTreeMap<u32, LecternClicks>,
    leave_beds: Vec<u32>,
}

impl HostSessions {
    fn idle(&self) -> bool {
        self.sessions.is_empty()
            && self.waiting.is_empty()
            && self.worker.is_none()
            && self.chat.is_empty()
            && self.selects.is_empty()
            && self.credits_done.is_empty()
            && self.lecterns.is_empty()
            && self.leave_beds.is_empty()
    }
}

fn read_inbox(host: &mut HostSessions, wire: &mut net::MinecraftWire, now: Instant) {
    for (client, bytes) in wire.take_inbox() {
        match minecraft_net::decode::<ToServer>(Lane::Reliable, &bytes) {
            Ok(ToServer::Hello {
                protocol,
                view_distance,
                registry,
            }) => {
                if !host.sessions.sessions.contains_key(&client) {
                    let hello = Hello {
                        protocol,
                        view_distance,
                        registry,
                    };
                    host.waiting.insert(client, hello);
                }
            }
            Ok(ToServer::ChunkBatchReceived { seq }) => {
                host.sessions.batch_received(client, seq);
                wire.batch_received(client, seq);
            }
            Ok(ToServer::Chat(message)) => {
                if host.sessions.sessions.contains_key(&client)
                    && let Some(message) = minecraft_survival::commands::from_friend(message)
                {
                    host.chat.entry(client).or_default().push(message);
                }
            }
            Ok(ToServer::SelectSlot(slot)) => {
                if host.sessions.sessions.contains_key(&client) {
                    host.selects.insert(client, slot);
                }
            }
            Ok(ToServer::Hand(events)) => {
                if let Some(session) = host.sessions.sessions.get_mut(&client) {
                    session.hands.push(events);
                }
            }
            Ok(ToServer::Click(click)) => {
                if let Some(session) = host.sessions.sessions.get_mut(&client) {
                    session.push_menu(MenuInput::Click(click));
                }
            }
            Ok(ToServer::RenameItem(name)) => {
                if let Some(session) = host.sessions.sessions.get_mut(&client) {
                    session.push_menu(MenuInput::Rename(name));
                }
            }
            Ok(ToServer::CloseMenu { menu }) => {
                if let Some(session) = host.sessions.sessions.get_mut(&client) {
                    session.push_menu(MenuInput::Close(menu));
                }
            }
            Ok(ToServer::OpenMountInventory) => {
                if let Some(session) = host.sessions.sessions.get_mut(&client) {
                    session.push_menu(MenuInput::OpenMount);
                }
            }
            Ok(ToServer::CreditsDone) => {
                if host.sessions.sessions.contains_key(&client) {
                    host.credits_done.push(client);
                }
            }
            Ok(ToServer::Lectern { pos, button }) => {
                if host.sessions.sessions.contains_key(&client) {
                    host.lecterns.entry(client).or_default().push((pos, button));
                }
            }
            Ok(ToServer::LeaveBed) => {
                if host.sessions.sessions.contains_key(&client) {
                    host.leave_beds.push(client);
                }
            }
            Ok(_) => {}
            Err(error) => {
                diag::warn!(
                    World,
                    "Minecraft message from client {client} dropped: {error}"
                );
                wire.bad_message(client, now);
            }
        }
    }
    for (client, bytes) in wire.take_state() {
        if let Ok(state) = minecraft_net::decode::<HandState>(Lane::State, &bytes)
            && let Some(session) = host.sessions.sessions.get_mut(&client)
        {
            session.hands.state(state, now);
        }
    }
}

fn teleport(
    player: &mut minecraft_survival::player::Player,
    authority: &mut sim::SimWorld,
    client: sim::ClientId,
    origin: [f64; 3],
    to: [f64; 3],
) {
    let to = minecraft_survival::bands::kept_in_walls(player.dimension, to);
    authority.teleport(client, sim::voxel::to_map(origin, to));
    player.steps.air_peak = None;
}

fn run_friend_commands(
    host: &mut HostSessions,
    server: &mut minecraft_survival::server::Server,
    authority: &mut sim::SimWorld,
    origins: &std::collections::HashMap<minecraft_net::Dimension, [f64; 3]>,
    local: &PlayerUuid,
    local_client: sim::ClientId,
    relayed: minecraft_survival::commands::Handled,
) {
    let chat = std::mem::take(&mut host.chat);
    let mut teleports = relayed.teleports;
    host.sessions
        .deliver(&server.globals.chat, &relayed.delivered, false);
    let friends: Vec<(u32, PlayerUuid)> = host
        .sessions
        .sessions
        .values()
        .map(|session| (session.client, session.uuid.clone()))
        .collect();
    for (client, uuid) in friends {
        let lines = chat.get(&client).map_or_else(Vec::new, Vec::clone);
        let mut presence =
            crate::minecraft_world::friends_presence(&server.players, authority, origins, local);
        if let Some(player) = server.players.get(local)
            && let Some(&origin) = origins.get(&player.dimension)
        {
            let ps = authority.player(local_client);
            let own = crate::minecraft_world::presence_of(player, ps, origin);
            presence.push((local.clone(), own));
        }
        let minecraft_survival::server::Server {
            globals,
            levels,
            players,
            ..
        } = server;
        let handled =
            minecraft_survival::commands::run(globals, levels, players, &uuid, lines, presence);
        host.sessions
            .deliver(&globals.chat, &handled.delivered, true);
        teleports.extend(handled.teleports);
    }
    let teleported: Vec<PlayerUuid> = teleports.iter().map(|(uuid, _)| uuid.clone()).collect();
    let minecraft_survival::server::Server {
        globals,
        levels,
        players,
        published,
        ..
    } = server;
    let woke = minecraft_survival::sleep::wake_teleported(
        globals,
        levels,
        players,
        &teleported,
        *published,
    );
    let woke_lines: Vec<Outgoing> = woke
        .messages
        .into_iter()
        .filter_map(|(uuid, message)| {
            let to = Recipient::Player(players.get(&uuid)?.id);
            Some(Outgoing { to, message })
        })
        .collect();
    host.sessions.deliver(&globals.chat, &woke_lines, true);
    for (uuid, to) in teleports {
        let Some(player) = players.get_mut(&uuid) else {
            continue;
        };
        let Some(&origin) = origins.get(&player.dimension) else {
            continue;
        };
        let client = if uuid == *local {
            local_client
        } else {
            sim::ClientId(player.id.0)
        };
        let entities = levels
            .get_mut(&player.dimension)
            .and_then(|level| level.entities.as_mut());
        let rider = (client, u64::from(player.id.0));
        crate::minecraft_riding::dismount(&mut player.riding, entities, authority, rider);
        teleport(player, authority, client, origin, to);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn host(
    time: Res<Time>,
    wire: Option<ResMut<net::MinecraftWire>>,
    mut authority: Option<ResMut<net::AuthorityWorld>>,
    local: Res<net::LocalPresentClient>,
    mut runtime: NonSendMut<Runtime>,
    mut host: NonSendMut<HostSessions>,
    mut ui: ResMut<frame::MinecraftUi>,
    mut body_gear: ResMut<crate::anim::minecraft_gear::McBodyGear>,
) {
    let relayed = std::mem::take(&mut runtime.relayed);
    let Some(mut wire) = wire.filter(|wire| wire.role() == Some(net::WireRole::Host)) else {
        if !host.idle() {
            *host = HostSessions::default();
        }
        return;
    };
    let now = Instant::now();
    let left = std::mem::take(&mut wire.left);
    wire.joined.clear();
    for client in &left {
        host.waiting.remove(client);
        host.chat.remove(client);
        host.selects.remove(client);
        host.credits_done.retain(|done| done != client);
        host.lecterns.remove(client);
        host.leave_beds.retain(|leaving| leaving != client);
        body_gear.remove(*client);
        sim::voxel::set_death_protected(*client, false);
    }
    read_inbox(&mut host, &mut wire, now);
    let host = &mut *host;
    let Some(mut view) = crate::minecraft_world::host_view(&mut runtime) else {
        if !host.sessions.is_empty() {
            host.sessions = Sessions::default();
        }
        return;
    };
    let server = view.server;
    server.published = true;
    if let Some(sounds) = view.sounds.as_deref_mut() {
        let _ = sounds.take_recorded();
    }
    for client in left {
        let Some(left) = host.sessions.leave(server, client) else {
            continue;
        };
        if let Some(level) = server.levels.get(&left.player.dimension) {
            let band = minecraft_survival::bands::level(level.dimension);
            let tables = (&mut *view.shapes, &mut *view.shape_ids, band);
            let registries = &server.globals.registries;
            crate::minecraft_world::refresh_shapes(&level.terrain, registries, tables, &left.woke);
        }
    }
    for client in std::mem::take(&mut host.credits_done) {
        host.sessions.credits_done(&mut server.players, client);
    }
    let host_here = server.players.contains_key(view.local);
    if let Some(authority) = authority.as_ref().filter(|_| host_here)
        && !host.waiting.is_empty()
    {
        let registry = *host.registry.get_or_insert_with(|| {
            minecraft_survival::session::registry_fingerprint(&server.globals.registries)
        });
        let origins: Vec<_> = view
            .origins
            .iter()
            .map(|(dimension, origin)| (*dimension, *origin))
            .collect();
        let kit = std::env::var("IW4L_MINECRAFT_KIT").ok();
        let context = JoinContext {
            registry,
            origins: &origins,
            view_distance: crate::minecraft_world::VIEW_DISTANCE,
            kit: kit.as_deref(),
        };
        for (client, hello) in std::mem::take(&mut host.waiting) {
            let name = net::client_name_string(&authority.0, sim::ClientId(client));
            if name.is_empty() {
                host.waiting.insert(client, hello);
                continue;
            }
            host.sessions
                .hello(server, client, &name, hello, &context, now);
        }
    }
    for (client, reason) in host.sessions.take_retiring(now) {
        wire.drop_peer(client, &reason);
    }
    let mut beds = Vec::new();
    if let Some(authority) = authority.as_mut() {
        let minecraft_survival::server::Server {
            globals,
            levels,
            players,
            ..
        } = &mut *server;
        let mut world = crate::minecraft_players::FriendWorld {
            levels,
            globals,
            authority: &mut authority.0,
            queues: &mut *view.queues,
            shapes: &mut *view.shapes,
            shape_ids: &mut *view.shape_ids,
            language: view.language,
            packs: view.packs,
            atlas: view.atlas,
            mining: &mut *view.mining,
            minings: &mut *view.minings,
            shown: view.shown,
            sounds: view.sounds.as_deref_mut(),
            light: view.light,
            dt: time.delta_secs_f64(),
            now: time.elapsed_secs_f64(),
            instant: now,
        };
        let mut told = Vec::new();
        let leaving = std::mem::take(&mut host.leave_beds);
        for session in host.sessions.sessions.values_mut() {
            let inputs = std::mem::take(&mut session.menu_inputs);
            let Some(player) = players.get_mut(&session.uuid) else {
                session.hands.take(now, |_| false);
                continue;
            };
            let Some(&origin) = view.origins.get(&player.dimension) else {
                session.hands.take(now, |_| false);
                continue;
            };
            let selected = host.selects.remove(&session.client);
            let clicks = host
                .lecterns
                .remove(&session.client)
                .into_iter()
                .flatten()
                .collect();
            let id = sim::ClientId(session.client);
            let was = player.menus.block().cloned();
            let stepped = crate::minecraft_players::step_friend(
                player,
                &mut world,
                id,
                origin,
                selected,
                &mut session.hands,
                (&mut session.lectern, clicks),
            );
            session
                .outbox
                .extend(stepped.books.into_iter().map(ToClient::Lectern));
            if let Some(bed) = stepped.bed {
                beds.push((session.uuid.clone(), BedAction::Lie(bed)));
            }
            if leaving.contains(&session.client) {
                beds.push((session.uuid.clone(), BedAction::Leave));
            }
            session
                .outbox
                .extend(stepped.lines.into_iter().map(ToClient::Chat));
            for killer in stepped.killers {
                session.outbox.push(ToClient::KilledBy {
                    killer: Some(killer),
                    line: minecraft_commands::Text::literal(""),
                });
            }
            let lines = crate::minecraft_players::friend_menus(
                player,
                &mut world,
                (id, origin),
                inputs,
                was,
            );
            session.outbox.extend(lines.into_iter().map(ToClient::Chat));
            told.extend(stepped.told.map(|message| Outgoing {
                to: Recipient::All,
                message,
            }));
        }
        host.sessions.deliver(&server.globals.chat, &told, true);
        for player in server.players.values() {
            if player.uuid == *view.local {
                continue;
            }
            let pm_type = authority
                .0
                .player(sim::ClientId(player.id.0))
                .map(|ps| ps.pm_type)
                .filter(|_| view.shown == Some(player.dimension));
            let worn = minecraft_survival::gear::body_gear(
                &player.inventory,
                player.selected,
                &player.mode,
            );
            body_gear.follow(player.id.0, pm_type, worn);
        }
        run_friend_commands(
            host,
            server,
            &mut authority.0,
            view.origins,
            view.local,
            local.0,
            relayed,
        );
        let mut events = std::mem::take(view.events);
        events.extend(sim::voxel::take_events());
        let background = crate::minecraft_levels::Background {
            origins: view.origins,
            host: (view.local, view.shown),
            ticked: view.ticked,
            mining: &mut *view.mining,
            minings: &mut *view.minings,
            shapes: &mut *view.shapes,
            shape_ids: &mut *view.shape_ids,
            packs: view.packs,
            atlas: view.atlas,
            dt: time.delta_secs_f64(),
            now: time.elapsed_secs_f64(),
        };
        let ticked =
            crate::minecraft_levels::tick_levels(server, &mut authority.0, background, events);
        host.sessions.deliver(&server.globals.chat, &ticked, true);
        host.sessions.send_levels(&server.globals.chat);
    } else {
        host.chat.clear();
    }
    let origins: Vec<_> = view
        .origins
        .iter()
        .map(|(dimension, origin)| (*dimension, *origin))
        .collect();
    host.sessions.follow_levels(&server.players, &origins);
    host.sessions.send_credits(&server.players);
    host.sessions
        .send_sleepers(&server.players, (view.local, local.0.0));
    let fight = server
        .levels
        .get(&minecraft_net::Dimension::End)
        .and_then(|level| level.dragon_fight.as_ref());
    host.sessions.send_boss_bars(&server.players, fight);
    host.sessions.send_views(&server.players);
    let gears: Vec<_> = server
        .players
        .values()
        .map(|player| {
            let client = if player.uuid == *view.local {
                local.0.0
            } else {
                player.id.0
            };
            (client, minecraft_survival::gear::player_gear(player))
        })
        .collect();
    host.sessions.send_gear(&gears);
    let books = Books::of(&server.globals.data);
    host.sessions
        .send_menus(&mut server.players, &mut server.levels, books);
    let frames = host.sessions.send_mobs(
        &server.players,
        &mut server.levels,
        crate::minecraft_world::VIEW_DISTANCE,
    );
    host.sessions
        .send_heard(&server.players, &mut server.levels);
    for (client, frame) in frames {
        match minecraft_net::encode(Lane::State, &frame) {
            Ok(bytes) => {
                if !wire.send_state(client, &bytes) {
                    diag::warn!(
                        World,
                        "Minecraft state of {} bytes to client {client} not sent",
                        bytes.len()
                    );
                }
            }
            Err(error) => {
                diag::warn!(
                    World,
                    "Minecraft state to client {client} not sent: {error}"
                );
            }
        }
    }
    if host.worker.is_none() && !host.sessions.is_empty() {
        host.worker = ChunkWorker::start();
    }
    let mut encoded = std::mem::take(&mut host.inline);
    if let Some(worker) = host.worker.as_ref() {
        encoded.extend(worker.done.try_iter());
    }
    for Encoded { client, seq, batch } in encoded {
        if !host.sessions.sessions.contains_key(&client) {
            continue;
        }
        host.sessions.batch_encoded(client, &batch.rest);
        wire.send_batch(client, seq, Arc::new(batch.blob));
    }
    host.sessions.fan_out(&mut server.levels);
    host.sessions.send_cracks(&server.players, &server.levels);
    if host.sessions.is_empty() {
        host.time_at = None;
    } else if host
        .time_at
        .is_none_or(|at| now.duration_since(at) >= minecraft_survival::session::TIME_EVERY)
    {
        host.time_at = Some(now);
        host.sessions.send_time(&server.globals, &server.levels);
    }
    let registries = server.globals.registries.clone();
    for job in host.sessions.select_batches(server) {
        let sender = host.worker.as_ref().and_then(|worker| worker.jobs.as_ref());
        let refused = match sender {
            Some(sender) => sender
                .send((job, registries.clone()))
                .err()
                .map(|refused| refused.0.0),
            None => Some(job),
        };
        if let Some(job) = refused {
            let batch = encode(&job, &registries);
            host.inline.push(Encoded {
                client: job.client,
                seq: job.seq,
                batch,
            });
        }
    }
    for (client, message) in host.sessions.take_outgoing() {
        match minecraft_net::encode(Lane::Reliable, &message) {
            Ok(bytes) => {
                wire.send(client, &bytes);
            }
            Err(error) => {
                diag::warn!(
                    World,
                    "Minecraft message to client {client} not sent: {error}"
                );
            }
        }
    }
    runtime.friend_beds.extend(beds);
    let lines = std::mem::take(&mut host.sessions.host_lines);
    crate::minecraft_world::show_lines(&runtime, &mut ui.chat, &lines);
}

fn room_code(state: &net::MasterBridgeState) -> Option<String> {
    match state {
        net::MasterBridgeState::Hosting { identity, .. } => Some(identity.room_id.to_string()),
        _ => None,
    }
}

pub(crate) fn show_join_code(
    bridge: Option<Res<net::MasterBridge>>,
    runtime: NonSend<Runtime>,
    mut ui: ResMut<frame::MinecraftUi>,
    mut shown: Local<Option<String>>,
) {
    let Some(code) = bridge.and_then(|bridge| room_code(&bridge.state())) else {
        return;
    };
    if shown.as_ref() == Some(&code) {
        return;
    }
    let line = minecraft_survival::session::join_code_line(&code);
    if crate::minecraft_world::show_lines(&runtime, &mut ui.chat, &[line]) {
        *shown = Some(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hosting(room: &str) -> net::MasterBridgeState {
        net::MasterBridgeState::Hosting {
            identity: net::SessionIdentity {
                room_id: room.parse().unwrap(),
                ..net::SessionIdentity::unassigned(1)
            },
            name: "Minecraft world".to_owned(),
            map: "minecraft:overworld".to_owned(),
            mode: "dm".to_owned(),
            members: Vec::new(),
            member_names: Default::default(),
            max_players: 4,
            skip_votes: 0,
            in_match: true,
        }
    }

    #[test]
    fn only_a_hosting_bridge_gives_the_room_code_in_hex() {
        let room = "0123456789abcdef0123456789abcdef";
        assert_eq!(room_code(&hosting(room)).as_deref(), Some(room));
        let joined = net::MasterBridgeState::Joining {
            identity: net::SessionIdentity {
                room_id: room.parse().unwrap(),
                ..net::SessionIdentity::unassigned(1)
            },
        };
        assert_eq!(room_code(&joined), None);
        let connecting = net::MasterBridgeState::Connecting {
            identity: net::SessionIdentity::unassigned(1),
        };
        assert_eq!(room_code(&connecting), None);
    }
}
