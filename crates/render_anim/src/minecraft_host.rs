use std::collections::BTreeMap;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Instant;

use bevy::prelude::*;
use minecraft_commands::{Outgoing, Recipient, Serverbound};
use minecraft_net::messages::HandState;
use minecraft_net::{Lane, PlayerUuid, ToClient, ToServer};
use minecraft_survival::session::{BatchJob, EncodedBatch, Hello, JoinContext, Sessions};
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
}

impl HostSessions {
    fn idle(&self) -> bool {
        self.sessions.is_empty()
            && self.waiting.is_empty()
            && self.worker.is_none()
            && self.chat.is_empty()
            && self.selects.is_empty()
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
    for (uuid, to) in teleports {
        let Some(player) = server.players.get_mut(&uuid) else {
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
        sim::voxel::set_death_protected(*client, false);
    }
    read_inbox(&mut host, &mut wire, now);
    let host = &mut *host;
    let Some(view) = crate::minecraft_world::host_view(&mut runtime) else {
        if !host.sessions.is_empty() {
            host.sessions = Sessions::default();
        }
        return;
    };
    let server = view.server;
    for client in left {
        host.sessions.leave(server, client);
    }
    let host_dimension = server
        .players
        .get(view.local)
        .map(|player| player.dimension);
    if let (Some(dimension), Some(authority)) = (host_dimension, authority.as_ref())
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
            dimension,
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
    host.sessions.lock_travel(&mut server.players);
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
            queues: view.queues,
            shapes: view.shapes,
            shape_ids: view.shape_ids,
            language: view.language,
            packs: view.packs,
            atlas: view.atlas,
            mining: view.mining,
            sounds: view.sounds,
            light: view.light,
            dt: time.delta_secs_f64(),
            now: time.elapsed_secs_f64(),
            instant: now,
        };
        let mut told = Vec::new();
        for session in host.sessions.sessions.values_mut() {
            let Some(player) = players.get_mut(&session.uuid) else {
                session.hands.take(now, |_| false);
                continue;
            };
            let Some(&origin) = view.origins.get(&player.dimension) else {
                session.hands.take(now, |_| false);
                continue;
            };
            if view.shown != Some(player.dimension) {
                session.hands.take(now, |_| false);
                continue;
            }
            let selected = host.selects.remove(&session.client);
            let id = sim::ClientId(session.client);
            let stepped = crate::minecraft_players::step_friend(
                player,
                &mut world,
                id,
                origin,
                selected,
                &mut session.hands,
            );
            session
                .outbox
                .extend(stepped.lines.into_iter().map(ToClient::Chat));
            told.extend(stepped.told.map(|message| Outgoing {
                to: Recipient::All,
                message,
            }));
        }
        host.sessions.deliver(&server.globals.chat, &told, true);
        run_friend_commands(
            host,
            server,
            &mut authority.0,
            view.origins,
            view.local,
            local.0,
            relayed,
        );
        host.sessions.send_levels(&server.globals.chat);
    } else {
        host.chat.clear();
    }
    host.sessions.send_views(&server.players);
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
    let lines = std::mem::take(&mut host.sessions.host_lines);
    crate::minecraft_world::show_lines(&runtime, &mut ui.chat, &lines);
}
