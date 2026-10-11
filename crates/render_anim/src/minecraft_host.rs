use std::collections::BTreeMap;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Instant;

use bevy::prelude::*;
use minecraft_net::{Lane, ToServer};
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
}

impl HostSessions {
    fn idle(&self) -> bool {
        self.sessions.is_empty() && self.waiting.is_empty() && self.worker.is_none()
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
}

pub(crate) fn host(
    wire: Option<ResMut<net::MinecraftWire>>,
    mut authority: Option<ResMut<net::AuthorityWorld>>,
    mut runtime: NonSendMut<Runtime>,
    mut host: NonSendMut<HostSessions>,
    mut ui: ResMut<frame::MinecraftUi>,
) {
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
        for session in host.sessions.sessions.values_mut() {
            let Some(player) = server.players.get_mut(&session.uuid) else {
                continue;
            };
            let Some(&origin) = view.origins.get(&player.dimension) else {
                continue;
            };
            let id = sim::ClientId(session.client);
            let Some(ps) = authority.0.player(id).copied() else {
                continue;
            };
            if view.shown != Some(player.dimension) {
                continue;
            }
            let alive = ps.pm_type == 0;
            if session.observe_life(alive)
                && let Some(level) = server.levels.get(&player.dimension)
            {
                let spawn = minecraft_survival::respawn::level_spawn(&server.globals.spawn, level);
                player.arrival = Some(spawn);
            }
            if alive {
                crate::minecraft_world::place_friend(player, &mut authority.0, id, &ps, origin);
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
