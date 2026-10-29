use std::collections::BTreeMap;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Instant;

use bevy::prelude::*;
use minecraft_commands::{Outgoing, Recipient, Serverbound};
use minecraft_net::messages::StateUp;
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
type BookEdits = Vec<(u8, Vec<String>, Option<String>)>;

const SEEN_RECIPES: usize = 256;

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
    edit_books: BTreeMap<u32, BookEdits>,
    skaters: BTreeMap<u32, crate::minecraft_skaters::Skater>,
    poses: crate::minecraft_skaters::SkaterTracks,
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
            && self.edit_books.is_empty()
            && self.skaters.is_empty()
            && self.poses.is_empty()
    }
}

fn read_inbox(
    host: &mut HostSessions,
    wire: &mut net::MinecraftWire,
    (now, seconds): (Instant, f64),
) {
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
            Ok(ToServer::EditBook { slot, pages, title }) => {
                if host.sessions.sessions.contains_key(&client) {
                    host.edit_books.entry(client).or_default().push((slot, pages, title));
                }
            }
            Ok(ToServer::CreativeSlot { slot, stack }) => {
                if let Some(session) = host.sessions.sessions.get_mut(&client) {
                    session.push_menu(MenuInput::Creative { slot, stack });
                }
            }
            Ok(ToServer::RecipeBookSettings { open, filtering }) => {
                if let Some(session) = host.sessions.sessions.get_mut(&client) {
                    session.book_settings = Some((open, filtering));
                }
            }
            Ok(ToServer::Skate(report)) => {
                if host.sessions.sessions.contains_key(&client) {
                    host.skaters.entry(client).or_default().heard(report, seconds);
                }
            }
            Ok(ToServer::RecipeSeen(ids)) => {
                if let Some(session) = host.sessions.sessions.get_mut(&client) {
                    let room = SEEN_RECIPES.saturating_sub(session.seen_recipes.len());
                    session.seen_recipes.extend(ids.into_iter().take(room));
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
        let Ok(up) = minecraft_net::decode::<StateUp>(Lane::State, &bytes) else {
            continue;
        };
        let Some(session) = host.sessions.sessions.get_mut(&client) else {
            continue;
        };
        match up {
            StateUp::Hand(state) => session.hands.state(state, now),
            StateUp::Skate(report) => {
                host.skaters.entry(client).or_default().heard(report, seconds);
            }
        }
    }
}

fn skating(skaters: &BTreeMap<u32, crate::minecraft_skaters::Skater>, client: u32) -> bool {
    skaters
        .get(&client)
        .is_some_and(|skater| skater.skating().active)
}

fn told_swing(client: u32, hand: &mut minecraft_survival::player::Hand) -> Option<ToClient> {
    if hand.swing != Some(0.0) || hand.swing_predicted {
        return None;
    }
    hand.swing_predicted = true;
    Some(ToClient::Animate {
        client,
        offhand: hand.swing_offhand,
    })
}

fn edit_books(player: &mut minecraft_survival::player::Player, edits: BookEdits) {
    let minecraft_survival::player::Player { inventory, name, .. } = player;
    for (slot, pages, title) in edits {
        let slot = usize::from(slot);
        minecraft_survival::books::edit_book(inventory, slot, &pages, title.as_deref(), name);
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
    (skate, mut poses): (Res<frame::SkateMode>, ResMut<frame::SkatePoses>),
) {
    let relayed = std::mem::take(&mut runtime.relayed);
    let Some(mut wire) = wire.filter(|wire| wire.role() == Some(net::WireRole::Host)) else {
        if !host.idle() {
            host.poses.forget(&mut poses);
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
        host.edit_books.remove(client);
        host.skaters.remove(client);
        host.poses.leave(*client);
        body_gear.remove(*client);
        sim::voxel::set_death_protected(*client, false);
        sim::voxel::forget_gun_guard(*client);
        sim::voxel::forget_arsenal(*client);
    }
    let seconds = time.elapsed_secs_f64();
    read_inbox(&mut host, &mut wire, (now, seconds));
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
        let kit = minecraft_commands::kit::kit_line(minecraft_survival::commands::env_file().as_deref());
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
            guns: &mut *view.guns,
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
            minecraft_survival::recipe_book::follow_friend(
                &mut player.recipe_book,
                &player.inventory,
                session.book_settings.take(),
                &std::mem::take(&mut session.seen_recipes),
            );
            let selected = host.selects.remove(&session.client);
            let clicks = host
                .lecterns
                .remove(&session.client)
                .into_iter()
                .flatten()
                .collect();
            let edits = host.edit_books.remove(&session.client).unwrap_or_else(Vec::new);
            edit_books(player, edits);
            let id = sim::ClientId(session.client);
            let was = player.menus.block().cloned();
            let skater = host.skaters.entry(session.client).or_default();
            let (skate, refused) =
                crate::minecraft_skaters::ride(skater, player, world.authority, id, seconds);
            session.outbox.extend(refused);
            if let Some(pose) = skater.take_fresh() {
                host.poses.heard(session.client, &pose, seconds);
            } else if !skater.skating().active {
                host.poses.leave(session.client);
            }
            let stepped = crate::minecraft_players::step_friend(
                player,
                &mut world,
                id,
                origin,
                (selected, skate),
                &mut session.hands,
                (&mut session.lectern, clicks),
            );
            skater.allowed = stepped.board_allowed;
            session
                .outbox
                .extend(told_swing(session.client, &mut player.hand));
            session
                .outbox
                .extend(stepped.books.into_iter().map(ToClient::Lectern));
            if let Some(open) = stepped.open_book
                && let Ok(slot) = u8::try_from(open.slot)
            {
                session.outbox.push(ToClient::OpenBook { slot });
            }
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
        host.poses.show(&mut poses, seconds);
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
            let skating = skating(&host.skaters, player.id.0);
            let worn = worn.map(|gear| crate::minecraft_world::board_gear(gear, skating));
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
    host.sessions.send_raid_bars(&server.players);
    host.sessions.send_wither_bars(&server.players);
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
            let gear = minecraft_survival::gear::player_gear(player);
            let skating = if player.uuid == *view.local {
                skate.active
            } else {
                skating(&host.skaters, client)
            };
            (client, crate::minecraft_skaters::board_view(gear, skating))
        })
        .collect();
    host.sessions.send_gear(&gears);
    let books = Books::of(&server.globals.data);
    host.sessions
        .send_menus(&mut server.players, &mut server.levels, books);
    let mut frames = host.sessions.send_mobs(
        &server.players,
        &mut server.levels,
        crate::minecraft_world::VIEW_DISTANCE,
    );
    let mut skaters = Vec::new();
    if let Some(me) = server.players.get(view.local)
        && let Some(pose) = frame::SkatePose::of(&skate)
            .as_ref()
            .and_then(crate::minecraft_skaters::net_pose)
    {
        skaters.push((local.0.0, me.dimension, pose));
    }
    for session in host.sessions.sessions.values() {
        let pose = host.skaters.get(&session.client).and_then(|skater| skater.shown());
        if let (Some(pose), Some(player)) = (pose, server.players.get(&session.uuid)) {
            skaters.push((session.client, player.dimension, pose));
        }
    }
    for (client, frame) in &mut frames {
        let level = host
            .sessions
            .sessions
            .get(client)
            .and_then(|session| server.players.get(&session.uuid))
            .map(|player| player.dimension);
        if let Some(level) = level {
            frame.skaters = crate::minecraft_skaters::relayed(&skaters, *client, level);
        }
    }
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

    #[test]
    fn a_friend_is_told_only_of_its_own_swings_it_did_not_predict_once() {
        let mut hand = minecraft_survival::player::Hand::default();
        assert_eq!(told_swing(2, &mut hand), None, "no swing");
        hand.swing = Some(0.0);
        hand.swing_offhand = true;
        assert_eq!(
            told_swing(2, &mut hand),
            Some(ToClient::Animate {
                client: 2,
                offhand: true
            })
        );
        assert_eq!(told_swing(2, &mut hand), None, "told once");
        hand.swing_predicted = false;
        hand.swing = Some(1.5);
        assert_eq!(told_swing(2, &mut hand), None, "a running swing was told when it began");
        hand.swing = Some(0.0);
        hand.swing_offhand = false;
        hand.swing_predicted = true;
        assert_eq!(told_swing(2, &mut hand), None, "the friend swung on its own press");
    }

    #[test]
    fn a_friends_book_edit_on_the_host_changes_their_slot_and_goes_back_in_the_player_view() {
        use minecraft_net::messages::PlayerView;
        use minecraft_survival::books::{WRITABLE_BOOK, WRITTEN_BOOK, WRITTEN_BOOK_CONTENT};
        use minecraft_survival::session::Session;
        use minecraftoss_player::inventory::{Inventory, ItemStack};
        let profile = minecraft_commands::server::Profile {
            id: minecraft_commands::PlayerId(2),
            name: "Alex".to_owned(),
            uuid: "00000000-0000-0000-0000-000000000002".to_owned(),
        };
        let mut player = minecraft_survival::player::Player::new(
            &profile,
            Default::default(),
            Default::default(),
            Inventory::default(),
        );
        player.inventory.slots[40] = Some(ItemStack::new(WRITABLE_BOOK, 1));
        player.inventory.slots[3] = Some(ItemStack::new(WRITABLE_BOOK, 1));
        let recipes = player.inventory.recipes.clone();
        let mut session = Session::new(2, player.uuid.clone(), "Alex", 2);
        let views = |session: &mut Session| -> Vec<PlayerView> {
            std::mem::take(&mut session.outbox)
                .into_iter()
                .filter_map(|message| match message {
                    ToClient::Player(view) => Some(*view),
                    _ => None,
                })
                .collect()
        };
        session.send_view(&player);
        let before = views(&mut session).pop().expect("the first view");
        let mut inventory_ui = crate::minecraft_inventory::InventoryUi::default();
        let mut ui = frame::MinecraftUi::default();
        let open = |inventory_ui: &mut crate::minecraft_inventory::InventoryUi,
                    ui: &mut frame::MinecraftUi,
                    view: &PlayerView| {
            crate::minecraft_remote::open_held_book(inventory_ui, ui, (view, &recipes), "Alex", 40)
        };
        assert!(open(&mut inventory_ui, &mut ui, &before));
        assert_eq!(ui.screen, frame::McScreen::BookEdit);
        assert_eq!(ui.book_edit.as_ref().map(|edit| edit.author.as_str()), Some("Alex"));
        ui.clicks = vec![
            frame::McClick::EditBook {
                pages: vec!["Dear diary".to_owned(), String::new()],
                title: Some("Diary".to_owned()),
            },
            frame::McClick::Close,
        ];
        assert!(inventory_ui.book_clicks(&mut ui));
        assert!(!ui.inventory_open && ui.book_edit.is_none());
        assert!(inventory_ui.take_lectern_actions().is_empty());
        let sent: Vec<ToServer> = inventory_ui
            .take_book_edits()
            .into_iter()
            .map(|(slot, pages, title)| ToServer::EditBook {
                slot: u8::try_from(slot).unwrap(),
                pages,
                title,
            })
            .collect();
        assert_eq!(sent.len(), 1);
        let bytes = minecraft_net::encode(Lane::Reliable, &sent[0]).unwrap();
        let Ok(ToServer::EditBook { slot, pages, title }) =
            minecraft_net::decode::<ToServer>(Lane::Reliable, &bytes)
        else {
            panic!("not a book edit");
        };
        assert_eq!(slot, 40);
        let refused = (9, vec!["x".to_owned()], None);
        let quill = (3, vec!["Draft".to_owned()], None);
        edit_books(&mut player, vec![(slot, pages, title), refused, quill]);
        session.send_view(&player);
        let after = views(&mut session).pop().expect("a view with the new book");
        let stack = after.slots[40]
            .as_ref()
            .and_then(|nbt| minecraft_survival::views::view_stack(nbt, &recipes))
            .expect("the signed book");
        assert_eq!((stack.id.as_str(), stack.count), (WRITTEN_BOOK, 1));
        let content = &stack.components.as_ref().unwrap()[WRITTEN_BOOK_CONTENT];
        assert_eq!(content["author"], "Alex");
        assert_eq!(content["title"]["raw"], "Diary");
        assert_eq!(
            minecraft_terrain::lectern::pages_text(stack.components.as_ref()),
            ["Dear diary", ""]
        );
        assert_eq!(after.slots[9], None);
        let draft = after.slots[3]
            .as_ref()
            .and_then(|nbt| minecraft_survival::views::view_stack(nbt, &recipes))
            .expect("the quill");
        assert_eq!(draft.id, WRITABLE_BOOK);
        assert_eq!(minecraft_terrain::lectern::pages_text(draft.components.as_ref()), ["Draft"]);
        assert!(open(&mut inventory_ui, &mut ui, &after));
        assert_eq!(ui.screen, frame::McScreen::Lectern);
        let book = ui.book.as_ref().expect("the friend reads the signed book");
        assert_eq!((book.pages.len(), book.page, book.take), (2, 0, false));
    }
}
