use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use minecraft_commands::{Clientbound, Serverbound, Text};
use minecraft_net::messages::{BookView, Click, MenuView};
use minecraft_net::messages::{GameMode, Target};
use minecraft_net::{Lane, StateFrame, ToClient, ToServer};
use minecraft_survival::data::GameData;
use minecraft_survival::guns::{Follow, weapon_of};
use minecraft_survival::heard::TOTEM_EVENT;
use minecraft_survival::menus::Open;
use minecraft_survival::remote::{BossBar, DecodeJob, RemoteWorld};
use minecraft_survival::session::BREAK_EVENT;
use minecraft_survival::switch::LoadingReason;
use minecraft_survival::views::{HudClock, Selection, view_stack};
use minecraft_terrain::containers::Containers;
use minecraft_terrain::lectern::LecternAction;
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{AtlasAnimation, ItemVisuals};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::stations::StationBook;
use minecraftoss_core::registries::{DataPaths, Registries};
use minecraftoss_core::{BlockStateId, Chunk};
use minecraftoss_entities::enchant::Enchantments;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::Inventory;
use net::minecraft_wire::HOST;

use crate::minecraft_sounds::Sounds;
use crate::minecraft_spawn_gate::{CLIENT_WAIT_TIMEOUT_MS, SPAWN_EYE_HEIGHT, SpawnGate};
use crate::minecraft_world::{Loaded, MinecraftWorldView, SkyInputs};

const HELLO_EVERY: Duration = Duration::from_secs(2);
pub(crate) const NO_ANSWER: &str =
    "The host's Minecraft world does not answer: the host may run a different build";
const FIRE_RESISTANCE: &str = "minecraft:fire_resistance";

struct Prepared {
    look: Loaded,
    registries: Arc<Registries>,
    registry: u64,
    language: HashMap<String, String>,
    recipes: Arc<RecipeBook>,
    stations: Arc<StationBook>,
    enchantments: Option<Arc<Enchantments>>,
}

fn prepare() -> Result<Prepared, String> {
    let root = assets::minecraft_map::root().ok_or_else(assets::minecraft_setup::status)?;
    let paths = DataPaths::under(&root);
    let mut registries = Registries::load(&paths)?;
    minecraft_terrain::fork_items::load(&mut registries.items);
    let registries = Arc::new(registries);
    let packs = PackStack::open(vec![root.join("resourcepacks/local/minecraft-26.3")])
        .map_err(|error| error.to_string())?;
    let language =
        minecraft_terrain::item_icons::language(&packs).unwrap_or_else(|_| HashMap::new());
    let look = crate::minecraft_world::load_look(packs)?;
    let registry = minecraft_survival::session::registry_fingerprint(&registries);
    let data = GameData::load(Some(&root), crate::minecraft_world::data_jar());
    Ok(Prepared {
        look,
        registries,
        registry,
        language,
        recipes: data.recipes,
        stations: data.stations,
        enchantments: data.enchantments,
    })
}

type Decoded = (u32, u32, Result<Vec<Arc<Chunk>>, String>);

struct ReadWorker {
    jobs: Option<mpsc::Sender<DecodeJob>>,
    done: mpsc::Receiver<Decoded>,
    thread: Option<JoinHandle<()>>,
}

impl ReadWorker {
    fn start() -> Option<Self> {
        let (jobs, receive) = mpsc::channel::<DecodeJob>();
        let (send, done) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("minecraft-chunk-read".into())
            .spawn(move || {
                for job in receive {
                    if send.send((job.seq, job.epoch, job.decode())).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| diag::warn!(World, "Minecraft chunk read thread not started: {error}"))
            .ok()?;
        Some(Self {
            jobs: Some(jobs),
            done,
            thread: Some(thread),
        })
    }
}

impl Drop for ReadWorker {
    fn drop(&mut self) {
        drop(self.jobs.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Default)]
struct Told {
    rejected: bool,
    failed: bool,
    silent: bool,
}

#[derive(Default)]
struct Flakes {
    tick: u64,
    feet: BTreeMap<u32, [f64; 3]>,
}

#[derive(Default)]
pub(crate) struct Friend {
    loading: Option<mpsc::Receiver<Result<Prepared, String>>>,
    loader: Option<JoinHandle<()>>,
    prepared: Option<Prepared>,
    remote: Option<RemoteWorld>,
    worker: Option<ReadWorker>,
    ready_at: Option<u64>,
    host_heard: bool,
    hello_at: Option<Instant>,
    told: Told,
    light: Option<SkyLight>,
    light_volume_at: Option<[i32; 3]>,
    light_volume_age: u32,
    cloud_center: Option<(i32, i32)>,
    environment_accumulator: f64,
    environment_primed: bool,
    weather: [f32; 2],
    shapes: HashMap<BlockStateId, u16>,
    shape_ids: HashMap<(Vec<[u32; 6]>, u16), u16>,
    atlas_animation: AtlasAnimation,
    animation_clock: f64,
    spawn_gate: SpawnGate,
    gui: Option<crate::minecraft_gui::GuiAssets>,
    inventory_ui: crate::minecraft_inventory::InventoryUi,
    hud: HudClock,
    selection: Selection,
    follow: Follow,
    fire_frames: Option<(Handle<Image>, u32)>,
    mining: crate::minecraft_mining::Mining,
    cracks: BTreeMap<(u32, [i32; 3]), u8>,
    hand: crate::minecraft_input::HandSampler,
    shown_menu: Option<u8>,
    screen_open: bool,
    gear_items: ItemVisuals,
    sounds: Option<Sounds>,
    dimension: Option<minecraft_net::Dimension>,
    switch_screen: Option<LoadingReason>,
    loading_backgrounds: Option<crate::minecraft_loading::LoadingBackgrounds>,
    boss_bars: BTreeMap<u64, frame::minecraft_ui::McBossBar>,
    credits_due: bool,
    flakes: Flakes,
    activation: crate::minecraft_hand::ItemActivation,
    activation_tick: u64,
    recipe_book: crate::minecraft_recipe_book::FriendBook,
}

fn host_silent(ready_at: Option<u64>, now_ms: u64, heard: bool) -> bool {
    !heard && ready_at.is_some_and(|ready| now_ms.saturating_sub(ready) >= CLIENT_WAIT_TIMEOUT_MS)
}

impl Friend {
    pub(crate) fn gear_look(&mut self) -> Option<(&Loaded, &SkyLight, &mut ItemVisuals)> {
        let Friend {
            prepared: Some(prepared),
            remote: Some(remote),
            light: Some(light),
            gear_items,
            ..
        } = self
        else {
            return None;
        };
        remote.level.as_ref()?;
        Some((&prepared.look, light, gear_items))
    }

    fn holding(&self, now_ms: u64) -> bool {
        let silent = host_silent(self.ready_at, now_ms, self.host_heard);
        match self.remote.as_ref() {
            Some(remote) if remote.level.is_some() => self.spawn_gate.holding(now_ms),
            Some(remote) => remote.failed.is_none() && !silent,
            None => self.loading.is_some(),
        }
    }
}

fn stop(friend: &mut Friend, view: &mut MinecraftWorldView, ui: &mut frame::MinecraftUi) {
    if view.active {
        sim::voxel::deactivate();
        crate::minecraft_world::clear_view(view);
    }
    *friend = Friend::default();
    ui.loading_world = false;
    ui.level_loading = None;
    ui.chat_only = false;
    ui.credits = None;
    ui.boss_bars.clear();
    ui.sleep = None;
    ui.chat.wake_up();
}

fn send(wire: &mut net::MinecraftWire, message: &ToServer) {
    match minecraft_net::encode(Lane::Reliable, message) {
        Ok(bytes) => {
            wire.send(HOST, &bytes);
        }
        Err(error) => diag::warn!(World, "Minecraft message to the host not sent: {error}"),
    }
}

fn line(text: &str) -> Clientbound {
    Clientbound::System {
        text: Text::literal(text),
        overlay: false,
    }
}

fn exchange(
    friend: &mut Friend,
    wire: &mut net::MinecraftWire,
    now: Instant,
    now_ms: u64,
) -> Vec<Clientbound> {
    let Friend {
        prepared: Some(prepared),
        remote: Some(remote),
        worker,
        hello_at,
        ready_at,
        host_heard,
        told,
        ..
    } = friend
    else {
        return Vec::new();
    };
    for (_, bytes) in wire.take_inbox() {
        match minecraft_net::decode::<ToClient>(Lane::Reliable, &bytes) {
            Ok(message) => remote.apply(message),
            Err(error) => {
                diag::warn!(World, "Minecraft message from the host dropped: {error}");
                wire.bad_message(HOST, now);
            }
        }
    }
    for (_, bytes) in wire.take_state() {
        if let Ok(frame) = minecraft_net::decode::<StateFrame>(Lane::State, &bytes) {
            remote.apply_state(frame);
        }
    }
    for blob in wire.take_batches() {
        remote.receive_batch(blob);
    }
    let sender = worker.as_ref().and_then(|worker| worker.jobs.as_ref());
    for job in remote.take_decode_jobs() {
        let refused = match sender {
            Some(sender) => sender.send(job).err().map(|refused| refused.0),
            None => Some(job),
        };
        if let Some(job) = refused {
            let result = job.decode();
            remote.decoded(job.seq, job.epoch, result);
        }
    }
    if let Some(worker) = worker.as_ref() {
        for (seq, epoch, result) in worker.done.try_iter() {
            remote.decoded(seq, epoch, result);
        }
    }
    let asking = remote.login.is_none() && remote.rejected.is_none() && remote.failed.is_none();
    if wire.connected && asking && hello_at.is_none_or(|at| now.duration_since(at) >= HELLO_EVERY) {
        let view_distance = u8::try_from(crate::minecraft_world::VIEW_DISTANCE).unwrap_or(u8::MAX);
        send(wire, &ToServer::hello(view_distance, prepared.registry));
        *hello_at = Some(now);
    }
    for seq in remote.take_acks() {
        send(wire, &ToServer::ChunkBatchReceived { seq });
    }
    let mut lines = remote.take_chat();
    if let Some(reason) = wire.failed.clone()
        && remote.failed.is_none()
    {
        remote.failed = Some(reason);
    }
    if let Some(reason) = remote.rejected.clone().filter(|_| !told.rejected) {
        told.rejected = true;
        diag::warn!(World, "Minecraft host refused this game: {reason:?}");
        lines.push(Clientbound::System {
            text: reason,
            overlay: false,
        });
    }
    if let Some(reason) = remote.failed.clone().filter(|_| !told.failed) {
        told.failed = true;
        diag::warn!(World, "Minecraft stream from the host failed: {reason}");
        lines.push(line(&reason));
    }
    *host_heard |= wire.heard_from(HOST);
    let waited = host_silent(*ready_at, now_ms, *host_heard);
    if asking && waited && !told.silent {
        told.silent = true;
        diag::warn!(World, "Minecraft host never answered the hello");
        lines.push(line(NO_ANSWER));
    }
    lines
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn friend(
    time: Res<Time>,
    real_time: Res<Time<Real>>,
    role: Res<frame::RuntimeRole>,
    mut installed: MessageReader<frame::MatchInstalled>,
    mut torn_down: MessageReader<frame::MatchTornDown>,
    wire: Option<ResMut<net::MinecraftWire>>,
    mut prediction: Option<ResMut<net::ClientPredictionState>>,
    (local, presented): (Res<net::LocalPresentClient>, Res<net::PresentedSnapshot>),
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    (skate, cameras, mc_camera, ui_draw): (
        Res<frame::SkateMode>,
        Query<&Transform, With<render_scene::FlyCamera>>,
        Option<Res<frame::McCamera>>,
        Option<Res<frame::UiDraw>>,
    ),
    mut ui: ResMut<frame::MinecraftUi>,
    mut view: ResMut<MinecraftWorldView>,
    mut friend: NonSendMut<Friend>,
    mut images: ResMut<Assets<Image>>,
    (buttons, gamepads, active_pad, hud_input): (
        Res<ButtonInput<MouseButton>>,
        Query<&bevy::input::gamepad::Gamepad>,
        Option<Res<frame::ActivePad>>,
        Option<Res<frame::HudInputView>>,
    ),
    (mut body_gear, mut sound_queue, mut sleepers, weapons, strings): (
        ResMut<crate::anim::minecraft_gear::McBodyGear>,
        ResMut<audio::McSoundQueue>,
        ResMut<frame::McSleepers>,
        Option<Res<assets::PreparedWeapons>>,
        Option<Res<assets::PreparedLocalizedStrings>>,
    ),
) {
    if *role != frame::RuntimeRole::Client {
        if ui.chat_only {
            ui.chat_only = false;
        }
        return;
    }
    let friend = &mut *friend;
    if *sleepers != frame::McSleepers::default() {
        *sleepers = frame::McSleepers::default();
    }
    for _ in torn_down.read() {
        stop(friend, &mut view, &mut ui);
    }
    for match_ in installed.read() {
        stop(friend, &mut view, &mut ui);
        if assets::minecraft_map::is_minecraft(&match_.zone) {
            let (send, receive) = mpsc::channel();
            friend.loader = std::thread::Builder::new()
                .name("minecraft-world-load".into())
                .spawn(move || {
                    let _ = send.send(prepare());
                })
                .ok();
            friend.loading = Some(receive);
            view.active = true;
        }
    }
    if !view.active {
        return;
    }
    let now = Instant::now();
    let now_ms = real_time.elapsed().as_millis() as u64;
    if let Some(receive) = &friend.loading
        && let Ok(result) = receive.try_recv()
    {
        friend.loading = None;
        friend.loader = None;
        match result {
            Ok(prepared) => {
                crate::minecraft_world::show_look(&mut view, &prepared.look);
                friend.atlas_animation = AtlasAnimation::new(&prepared.look.atlas);
                let mut remote = RemoteWorld::new(prepared.registries.clone());
                remote.recipes = prepared.recipes.clone();
                friend.remote = Some(remote);
                friend.worker = ReadWorker::start();
                friend.ready_at = Some(now_ms);
                friend.gui = crate::minecraft_gui::GuiAssets::load(
                    &prepared.look.packs,
                    &prepared.language,
                    crate::minecraft_world::data_jar().as_deref(),
                    &mut images,
                );
                friend.fire_frames = crate::minecraft_world::fire_frames(&prepared.look.packs)
                    .map(|(image, frames)| (images.add(image), frames));
                ui.spyglass_scope = crate::minecraft_portal::strip(
                    &prepared.look.packs,
                    &mut images,
                    crate::minecraft_portal::SPYGLASS_SCOPE,
                )
                .map(|(image, _)| image);
                friend
                    .inventory_ui
                    .set_station_data(prepared.stations.clone(), prepared.enchantments.clone());
                friend.sounds = Some(Sounds::load(&prepared.look.packs));
                friend.loading_backgrounds = Some(
                    crate::minecraft_loading::LoadingBackgrounds::load(
                        &prepared.look.packs,
                        &mut images,
                    ),
                );
                friend.prepared = Some(prepared);
                diag::info!(World, "Minecraft look loaded: waiting for the host's world");
            }
            Err(error) => {
                diag::warn!(World, "Minecraft world failed to load: {error}");
                stop(friend, &mut view, &mut ui);
                return;
            }
        }
    }
    ui.active = false;
    let sent = std::mem::take(&mut ui.chat.sent);
    let mut wire = wire.filter(|wire| wire.role() == Some(net::WireRole::Friend));
    if let Some(wire) = wire.as_mut() {
        let lines = exchange(friend, wire, now, now_ms);
        if let Some(prepared) = friend.prepared.as_ref().filter(|_| !lines.is_empty()) {
            crate::minecraft_chat::show(&mut ui.chat, &lines, &prepared.language);
            show_overlays(&mut ui, &lines, &prepared.language);
        }
        show_effects(friend, &presented, &mut ui);
        track_bodies(friend, &presented);
        if show_credits(friend, &mut ui, &mut images) {
            send(wire, &ToServer::CreditsDone);
        }
        if friend
            .remote
            .as_ref()
            .is_some_and(|remote| remote.login.is_some())
        {
            for message in sent.iter().filter_map(|line| Serverbound::from_input(line)) {
                send(wire, &ToServer::Chat(message));
            }
        }
    }
    match friend.remote.as_ref() {
        Some(remote) => {
            for (&client, gear) in &remote.gear {
                let pm_type = presented.player(sim::ClientId(client)).map(|ps| ps.pm_type);
                let worn = minecraft_survival::gear::gear_of(gear);
                body_gear.follow(client, pm_type, Some(worn));
            }
        }
        None => body_gear.clear(),
    }
    ui.chat_only = friend
        .remote
        .as_ref()
        .is_some_and(|remote| remote.login.is_some() && remote.rejected.is_none());
    ui.chat.command_level = friend
        .remote
        .as_ref()
        .map_or(0, |remote| remote.command_level);
    if let Some(gui) = friend.gui.as_ref().filter(|_| ui.gui.is_none()) {
        ui.gui = Some(gui.gui.clone());
    }
    let aspect = windows
        .single()
        .map(|w| w.width() / w.height().max(1.0))
        .unwrap_or(16.0 / 9.0);
    let dt = time.delta_secs_f64();
    let mc_detached = mc_camera.as_ref().is_some_and(|c| c.detached);
    let player = presented.player(local.0).copied();
    let blocked = ui.inventory_open
        || hud_input
            .as_deref()
            .is_some_and(|input| input.console_open || input.script_menu_open);
    let leave_bed = show_sleep(friend, &mut ui, &mut sleepers, blocked);
    let drawn_from = cameras
        .iter()
        .next()
        .filter(|_| skate.active || mc_detached || sleepers.camera.is_some());
    if let (Some(prepared), Some(remote)) = (friend.prepared.as_ref(), friend.remote.as_ref()) {
        let lang = |key: &str| prepared.language.get(key).cloned();
        let name = |text: &Text| text.resolve(&lang).text;
        let bars = (&mut friend.boss_bars, &remote.boss_bars);
        ui.boss_bars = follow_boss_bars(bars, name, dt as f32);
    }
    let shown_ui = (
        ui.boss_bars.iter().any(|bar| bar.world_fog),
        ui.credits.is_some(),
        ui_draw.as_deref().is_none_or(|draw| draw.0),
    );
    present(
        friend,
        &mut view,
        prediction.as_deref(),
        player.as_ref(),
        drawn_from,
        aspect,
        (dt, shown_ui),
        now_ms,
    );
    snowflakes(friend, &presented);
    if let Some(prediction) = prediction.as_deref_mut() {
        prediction
            .0
            .set_minecraft_move_scale(frost(friend, player.as_ref()));
    }
    let seconds = real_time.elapsed_secs_f64();
    let shown = (dt, seconds, now_ms);
    let swap = std::mem::take(&mut ui.swap_hands);
    let drop = ui.drop_selected.take();
    friend.inventory_ui.set_weapons(&mut ui, weapons.as_deref(), strings.as_deref());
    if let Some(prepared) = friend.prepared.as_mut() {
        let state = (view.generation, view.active);
        let look = &mut prepared.look;
        friend.inventory_ui.fork.dress_world(&mut look.packs, &look.atlas, state, &mut view.atlas_patches);
    }
    let mut messages = show_player(friend, &mut ui, player.as_ref(), &mut images, shown);
    if leave_bed {
        messages.push(ToServer::LeaveBed);
    }
    if let (Some(sounds), Some(prepared)) = (friend.sounds.as_mut(), friend.prepared.as_ref()) {
        for event in std::mem::take(&mut ui.ui_sounds) {
            sounds.play(&prepared.look.packs, event, None, 1.0, 1.0);
        }
        sound_queue.0.append(&mut sounds.queued);
    }
    crate::minecraft_respawn::fade_overlay(&mut ui, dt);
    if let Some(wire) = wire.as_mut() {
        for message in &messages {
            send(wire, message);
        }
    }
    let logged_in = friend
        .remote
        .as_ref()
        .is_some_and(|remote| remote.login.is_some());
    if let Some(wire) = wire.as_mut().filter(|_| logged_in) {
        let pad = active_pad
            .and_then(|active| active.0)
            .and_then(|entity| gamepads.get(entity).ok());
        let mut hands =
            crate::minecraft_input::host_hands(&ui, hud_input.as_deref(), &buttons, pad);
        hands.captured |= !ui.active;
        let target = aim(friend, player.as_ref());
        let sampled = friend.hand.sample(hands, swap, drop, target, seconds);
        if !sampled.events.is_empty() {
            send(wire, &ToServer::Hand(sampled.events));
        }
        if let Some(state) = sampled.state {
            match minecraft_net::encode(Lane::State, &state) {
                Ok(bytes) => {
                    wire.send_state(HOST, &bytes);
                }
                Err(error) => diag::warn!(World, "Minecraft hand state not sent: {error}"),
            }
        }
    }
    ui.loading_world = friend.holding(now_ms);
    if friend.switch_screen.is_some() && !friend.spawn_gate.holding(now_ms) {
        friend.switch_screen = None;
    }
    let game_time = friend
        .remote
        .as_ref()
        .map_or(0.0, |remote| remote.clocks.overworld.ticks);
    let reason = friend.switch_screen;
    ui.level_loading = friend
        .loading_backgrounds
        .as_mut()
        .and_then(|backgrounds| backgrounds.frame(reason, game_time, dt, aspect, &mut images));
}

fn show_overlays(
    ui: &mut frame::MinecraftUi,
    lines: &[Clientbound],
    language: &HashMap<String, String>,
) {
    let lang = |key: &str| language.get(key).cloned();
    for line in lines {
        if let Clientbound::System {
            text,
            overlay: true,
        } = line
        {
            ui.overlay_message = Some(frame::minecraft_ui::McOverlayMessage {
                text: text.resolve(&lang).text,
                ticks: frame::minecraft_ui::McOverlayMessage::TICKS,
            });
        }
    }
}

fn show_effects(
    friend: &mut Friend,
    presented: &net::PresentedSnapshot,
    ui: &mut frame::MinecraftUi,
) {
    let Friend {
        prepared: Some(prepared),
        remote: Some(remote),
        mining,
        cracks,
        credits_due,
        activation,
        recipe_book,
        inventory_ui,
        ..
    } = friend
    else {
        return;
    };
    let own = remote.login.as_ref().map(|login| login.client);
    for effect in remote.take_effects() {
        match effect {
            ToClient::Crack {
                breaker,
                pos,
                stage,
            } => match u8::try_from(stage).ok().filter(|stage| *stage <= 9) {
                Some(stage) => {
                    cracks.insert((breaker, pos), stage);
                }
                None => {
                    cracks.remove(&(breaker, pos));
                }
            },
            ToClient::LevelEvent {
                event: BREAK_EVENT,
                pos,
                data,
            } => {
                if let Some(level) = remote.level.as_ref()
                    && let Ok(state) = u16::try_from(data)
                    && let Some(block) = level.terrain.stream.states.block(BlockStateId(state))
                {
                    mining.burst(
                        &prepared.look.packs,
                        &level.terrain.scene,
                        &prepared.look.atlas,
                        (pos[0], pos[1], pos[2]),
                        &block.clone(),
                    );
                }
            }
            ToClient::KilledBy {
                killer: Some(killer),
                ..
            } => killed_by(ui, killer),
            ToClient::Credits => *credits_due = true,
            ToClient::RecipeBook(book) => {
                let unlocked = recipe_book.receive(book, &mut ui.recipe_book);
                inventory_ui.set_unlocked(unlocked);
            }
            effect => {
                if own_totem(&effect, own) {
                    let level = remote.level.as_mut();
                    let mut random = level.map(|level| &mut level.view);
                    activation.activate(|| random.as_mut().map_or(0.5, |view| view.random_float()));
                }
                if let Some(level) = remote.level.as_mut() {
                    show_heard(level, effect, presented);
                }
            }
        }
    }
}

fn show_sleep(
    friend: &Friend,
    ui: &mut frame::MinecraftUi,
    sleepers: &mut frame::McSleepers,
    blocked: bool,
) -> bool {
    let Some(remote) = friend.remote.as_ref() else {
        return false;
    };
    let Some(level) = remote.level.as_ref() else {
        return false;
    };
    for (&client, lying) in &remote.sleepers {
        let facing = (lying.facing[0], lying.facing[1]);
        let root = crate::minecraft_sleep::lying_root(level.origin, lying.feet, facing);
        sleepers.bodies.push((client, root));
    }
    let own = remote.lying();
    if let Some(lying) = own {
        let facing = (lying.facing[0], lying.facing[1]);
        let camera = crate::minecraft_sleep::sleep_camera(level.origin, lying.feet, facing);
        sleepers.camera = Some(camera);
    }
    let timer = remote.sleep_timer;
    ui.sleep = (timer > 0).then_some(frame::minecraft_ui::McSleep { timer });
    if own.is_none() {
        ui.chat.wake_up();
    } else if !blocked {
        ui.chat.lie_in_bed();
    }
    std::mem::take(&mut ui.chat.leave_bed)
}

fn own_totem(effect: &ToClient, own: Option<u32>) -> bool {
    matches!(effect, ToClient::PlayerEvent { client, event } if *event == TOTEM_EVENT && Some(*client) == own)
}

fn follow_boss_bars(
    (shown, bars): (
        &mut BTreeMap<u64, frame::minecraft_ui::McBossBar>,
        &BTreeMap<u64, BossBar>,
    ),
    name: impl Fn(&Text) -> String,
    seconds: f32,
) -> Vec<frame::minecraft_ui::McBossBar> {
    shown.retain(|id, _| bars.contains_key(id));
    for (id, bar) in bars {
        let (fog, music) = (bar.flags.create_world_fog, bar.flags.play_music);
        let shown = shown.entry(*id).or_insert_with(|| {
            frame::minecraft_ui::McBossBar::new(name(&bar.name), bar.progress, fog, music)
        });
        shown.name = name(&bar.name);
        (shown.world_fog, shown.music) = (fog, music);
        shown.set_progress(bar.progress);
        shown.advance(seconds);
    }
    shown.values().cloned().collect()
}

fn show_credits(
    friend: &mut Friend,
    ui: &mut frame::MinecraftUi,
    images: &mut Assets<Image>,
) -> bool {
    if ui.credits.as_ref().is_some_and(|credits| credits.finished) {
        ui.credits = None;
        diag::info!(World, "Minecraft credits closed");
        return true;
    }
    if !std::mem::take(&mut friend.credits_due) || ui.credits.is_some() {
        return false;
    }
    let (Some(prepared), Some(remote)) = (friend.prepared.as_ref(), friend.remote.as_ref()) else {
        return true;
    };
    let name = remote
        .login
        .as_ref()
        .and_then(|login| remote.players.get(&login.client))
        .map_or("Player", |(name, _)| name.as_str());
    !crate::minecraft_end::roll(name, ui, &prepared.look.packs, images)
}

fn killed_by(ui: &mut frame::MinecraftUi, killer: String) {
    let seq = ui
        .killed_by
        .as_ref()
        .map_or(0, |(seq, _)| seq.wrapping_add(1));
    ui.killed_by = Some((seq, killer));
}

fn show_heard(
    level: &mut minecraft_survival::remote::RemoteLevel,
    effect: ToClient,
    presented: &net::PresentedSnapshot,
) {
    match effect {
        ToClient::LevelEvent { event, pos, data } => {
            let pos = (pos[0], pos[1], pos[2]);
            level
                .view
                .level_event(&level.terrain.scene, event, pos, data);
        }
        ToClient::Sound {
            event,
            pos,
            volume,
            pitch,
            ..
        } => {
            let at = glam::DVec3::from_array(pos);
            level.view.sounds.push((event, at, volume, pitch));
        }
        ToClient::Explosion {
            center,
            radius,
            blocks,
            interacts,
        } => {
            let center = glam::DVec3::from_array(center);
            let blocks = usize::try_from(blocks).unwrap_or(usize::MAX);
            level.view.explosion(center, radius, blocks, interacts);
        }
        ToClient::BlockEvent { pos, kind, param } => {
            let pos = (pos[0], pos[1], pos[2]);
            let count = u32::try_from(param).unwrap_or(0);
            level
                .view
                .lids
                .block_event(&level.terrain.scene, pos, kind, count);
        }
        ToClient::PlayerEvent { client, event } if event == TOTEM_EVENT => {
            if let Some((feet, height)) = body_of(presented, client, level.origin) {
                let width = crate::minecraft_players::PLAYER_WIDTH;
                level
                    .view
                    .totem_pop(u128::from(client), feet, width, height);
            }
        }
        _ => {}
    }
}

fn body_of(
    presented: &net::PresentedSnapshot,
    client: u32,
    origin: [f64; 3],
) -> Option<([f64; 3], f64)> {
    let ps = presented.player(sim::ClientId(client))?;
    let (_, maxs) = sim::voxel::player_hull(ps);
    let feet = sim::voxel::to_block(origin, ps.origin);
    Some((feet, f64::from(maxs[2] / sim::voxel::BLOCK)))
}

fn track_bodies(friend: &mut Friend, presented: &net::PresentedSnapshot) {
    let Some(remote) = friend.remote.as_mut() else {
        return;
    };
    let Some(level) = remote.level.as_mut() else {
        return;
    };
    let width = crate::minecraft_players::PLAYER_WIDTH;
    for &client in remote.players.keys() {
        if let Some((feet, height)) = body_of(presented, client, level.origin) {
            level
                .view
                .track_body(u128::from(client), feet, width, height);
        }
    }
}

fn snowflakes(friend: &mut Friend, presented: &net::PresentedSnapshot) {
    let Friend {
        remote: Some(remote),
        flakes,
        ..
    } = friend
    else {
        return;
    };
    let Some(level) = remote.level.as_mut() else {
        flakes.feet.clear();
        return;
    };
    let now = level.view.ticks();
    if now < flakes.tick {
        flakes.feet.clear();
    }
    let ticks = now.saturating_sub(flakes.tick).min(4);
    flakes.tick = now;
    if ticks == 0 {
        return;
    }
    let mut seen = BTreeMap::new();
    for &client in remote.players.keys() {
        let alive = presented
            .player(sim::ClientId(client))
            .is_some_and(|ps| ps.pm_type == 0);
        let Some((feet, height)) = body_of(presented, client, level.origin).filter(|_| alive)
        else {
            continue;
        };
        let moved = flakes
            .feet
            .get(&client)
            .is_some_and(|last| last[0] != feet[0] || last[2] != feet[2]);
        if moved {
            for _ in 0..ticks {
                level
                    .view
                    .snowflakes(&level.terrain.scene, feet, height as f32);
            }
        }
        seen.insert(client, feet);
    }
    flakes.feet = seen;
}

fn aim(friend: &Friend, ps: Option<&playerstate_iw4::PlayerState>) -> Target {
    let Some(remote) = friend.remote.as_ref() else {
        return Target::Miss;
    };
    let (Some(level), Some(view), Some(ps)) = (remote.level.as_ref(), remote.player.as_ref(), ps)
    else {
        return Target::Miss;
    };
    let feet = sim::voxel::to_block(level.origin, ps.origin);
    let mut aim = minecraftoss_player::Player::new(glam::DVec3::from_array(feet));
    aim.yaw = f64::from(crate::minecraft_world::minecraft_yaw(ps.viewangles[1]));
    aim.pitch = f64::from(ps.viewangles[0]);
    aim.crouching =
        ps.pm_flags & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE) != 0;
    aim.swimming = ps.view_height_current < crate::minecraft_world::CRAWLING_BELOW_VIEW_HEIGHT;
    let reach = if view.game_mode == GameMode::Creative {
        minecraft_terrain::game_mode::CREATIVE_BLOCK_REACH
    } else {
        minecraft_terrain::game_mode::BLOCK_REACH
    };
    let outlined = minecraft_terrain::doors::Outlined(&level.terrain.scene);
    let block = aim.target(&outlined, reach);
    let entity_reach = if view.game_mode == GameMode::Creative {
        minecraft_terrain::game_mode::CREATIVE_ENTITY_REACH
    } else {
        minecraft_terrain::game_mode::ENTITY_REACH
    };
    let block_distance = block.as_ref().map(|hit| hit.point.distance(aim.eye()));
    if let Some((id, distance)) = level.view.mob_on_ray(aim.eye(), aim.look(), entity_reach)
        && block_distance.is_none_or(|block| distance < block)
    {
        return Target::Mob { id };
    }
    block.map_or(Target::Miss, |hit| Target::Block {
        pos: [hit.pos.0, hit.pos.1, hit.pos.2],
        face: minecraft_survival::actions::face_index(hit.face),
        hit: hit.point.as_vec3().to_array(),
    })
}

fn frost(friend: &Friend, ps: Option<&playerstate_iw4::PlayerState>) -> f32 {
    let Some(remote) = friend.remote.as_ref() else {
        return 1.0;
    };
    let (Some(level), Some(view), Some(ps)) = (remote.level.as_ref(), remote.player.as_ref(), ps)
    else {
        return 1.0;
    };
    let feet = sim::voxel::to_block(level.origin, ps.origin);
    let alive = ps.pm_type == 0;
    crate::minecraft_players::frost_scale(&level.terrain.scene, feet, view.frozen, alive)
}

fn mc_game_mode(mode: GameMode) -> frame::McGameMode {
    match mode {
        GameMode::Survival => frame::McGameMode::Survival,
        GameMode::Creative => frame::McGameMode::Creative,
        GameMode::Adventure => frame::McGameMode::Adventure,
        GameMode::Spectator => frame::McGameMode::Spectator,
    }
}

fn show_player(
    friend: &mut Friend,
    ui: &mut frame::MinecraftUi,
    ps: Option<&playerstate_iw4::PlayerState>,
    images: &mut Assets<Image>,
    (dt, seconds, now_ms): (f64, f64, u64),
) -> Vec<ToServer> {
    let mut out = Vec::new();
    let Friend {
        prepared: Some(prepared),
        remote: Some(remote),
        spawn_gate,
        inventory_ui,
        hud,
        selection,
        follow,
        fire_frames,
        atlas_animation,
        shown_menu,
        screen_open,
        recipe_book,
        ..
    } = friend
    else {
        ui.inventory_open = false;
        return out;
    };
    let Some(view) = remote
        .player
        .as_ref()
        .filter(|_| remote.level.is_some() && remote.rejected.is_none())
    else {
        ui.inventory_open = false;
        return out;
    };
    selection.follow(view.selected, seconds);
    let picked = ui
        .select
        .take()
        .and_then(|slot| selection.pick(slot, seconds));
    out.extend(picked.map(ToServer::SelectSlot));
    ui.drop_selected = None;
    ui.swap_hands = false;
    let interacts = view.game_mode != GameMode::Spectator;
    ui.game_mode = mc_game_mode(view.game_mode);
    let alive = ps.filter(|ps| ps.pm_type == 0 && !spawn_gate.holding(now_ms));
    ui.active = alive.is_some() && ui.credits.is_none();
    if alive.is_none() || !interacts {
        ui.inventory_open = false;
    }
    let recipes = &prepared.recipes;
    for book in std::mem::take(&mut remote.lecterns) {
        show_book(inventory_ui, ui, book, &mut out);
    }
    if !inventory_ui.lectern_clicks(ui) {
        show_menus(
            ui,
            (&mut remote.menu, remote.inventory_menu.as_ref()),
            (shown_menu, screen_open),
            recipes,
            &prepared.language,
            &mut out,
        );
    }
    for (pos, action) in inventory_ui.take_lectern_actions() {
        let button = match action {
            LecternAction::Button(button) => Some(button),
            LecternAction::Close => None,
            LecternAction::PlaceBook(_) | LecternAction::Open => continue,
        };
        let pos = [pos.0, pos.1, pos.2];
        out.push(ToServer::Lectern { pos, button });
    }
    let Some(ps) = alive else {
        ui.vitals = None;
        ui.weapon_request = None;
        ui.holding_item = false;
        ui.empty_hand = false;
        return out;
    };
    let mut inventory = Inventory::default();
    inventory.recipes = recipes.clone();
    for (slot, stack) in inventory.slots.iter_mut().zip(&view.slots) {
        *slot = stack.as_ref().and_then(|nbt| view_stack(nbt, recipes));
    }
    let screen = minecraft_survival::menus::mirror(remote.menu.as_ref(), recipes);
    let current = remote.menu.as_ref().or(remote.inventory_menu.as_ref());
    inventory.cursor = current
        .and_then(|menu| menu.carried.as_ref())
        .and_then(|nbt| view_stack(nbt, recipes));
    if let Some(grid) = remote.inventory_menu.as_ref() {
        let cells = minecraft_survival::menus::stacks(&grid.slots, recipes);
        for (cell, stack) in inventory.crafting.iter_mut().zip(cells) {
            *cell = stack;
        }
    }
    if matches!(screen.open, Some(Open::Table(_))) {
        for (cell, stack) in inventory.workbench.iter_mut().zip(&screen.cells) {
            *cell = stack.clone();
        }
    }
    let (mins, maxs) = sim::voxel::player_hull(ps);
    let eye_in_water = sim::voxel::water(ps.origin, mins, maxs, ps.view_height_current).eye;
    let eye_in_lava = sim::voxel::eye_in_lava(ps.origin, ps.view_height_current);
    let vitals = hud.vitals(view, dt, eye_in_water, eye_in_lava);
    ui.vitals = Some(crate::minecraft_world::mc_vitals(vitals));
    ui.fire_frames = fire_frames.clone();
    let mut guns = [None; minecraft_survival::guns::HOTBAR];
    for (gun, stack) in guns.iter_mut().zip(&inventory.slots) {
        *gun = stack
            .as_ref()
            .and_then(|stack| inventory_ui.fork.weapon(stack))
            .filter(|&weapon| ps.weapons.contains(&(weapon as i32)));
    }
    let held = inventory.slots[selection.slot].as_ref();
    ui.holding_item = interacts
        && held.is_none_or(|stack| weapon_of(stack).is_none())
        && guns.get(selection.slot).copied().flatten().is_none();
    ui.empty_hand = ui.holding_item && held.is_none();
    ui.using_item = view.using.is_some();
    ui.scoping = minecraft_survival::spyglass::seen_scoping(
        view.using,
        &inventory,
        usize::from(view.selected),
    );
    let mut selected = selection.slot;
    ui.weapon_request = follow
        .request_with(&inventory, &guns, &mut selected, ps.weapon)
        .filter(|_| interacts);
    selection.slot = selected;
    inventory_ui.animation_tick = atlas_animation.ticks();
    let packs = &prepared.look.packs;
    let mut containers = Containers::default();
    containers.set_book(prepared.stations.clone());
    let level = u32::try_from(view.xp_level).unwrap_or(0);
    inventory_ui.publish(
        ui,
        &inventory,
        selection.slot,
        packs,
        images,
        (&screen, level, &containers),
    );
    out.extend(recipe_book.outgoing(&ui.recipe_book, inventory_ui.take_seen()));
    out
}

fn show_book(
    inventory_ui: &mut crate::minecraft_inventory::InventoryUi,
    ui: &mut frame::MinecraftUi,
    book: Option<BookView>,
    out: &mut Vec<ToServer>,
) {
    let Some(book) = book else {
        if let Some(pos) = inventory_ui.lectern() {
            inventory_ui.lectern_closed(ui, pos);
        }
        return;
    };
    let [x, y, z] = book.pos;
    let shown = frame::McBook {
        pages: book.pages,
        page: usize::try_from(book.page).unwrap_or(0),
        take: book.take,
    };
    if !inventory_ui.show_lectern(ui, (x, y, z), shown) {
        out.push(ToServer::Lectern {
            pos: book.pos,
            button: None,
        });
    }
}

fn show_menus(
    ui: &mut frame::MinecraftUi,
    (menu, inventory_menu): (&mut Option<MenuView>, Option<&MenuView>),
    (shown_menu, screen_open): (&mut Option<u8>, &mut bool),
    recipes: &RecipeBook,
    language: &HashMap<String, String>,
    out: &mut Vec<ToServer>,
) {
    if shown_menu.is_some() && menu.as_ref().map(|view| view.menu) != *shown_menu {
        *shown_menu = None;
        *screen_open = false;
        ui.inventory_open = false;
    }
    if let Some(view) = menu
        .as_ref()
        .filter(|view| Some(view.menu) != *shown_menu && ui.active)
    {
        *shown_menu = Some(view.menu);
        let screen = minecraft_survival::menus::mirror(Some(view), recipes);
        let lang = |key: &str| language.get(key).cloned();
        ui.screen = crate::minecraft_inventory::screen_for(screen.open.as_ref());
        ui.screen_title = view.title.resolve(&lang).text;
        ui.inventory_open = true;
        ui.screen_opened = true;
        *screen_open = true;
    }
    if menu.is_none() && matches!(ui.screen, frame::McScreen::Creative(_)) {
        ui.screen = frame::McScreen::Inventory;
    }
    let (id, state) = menu
        .as_ref()
        .or(inventory_menu)
        .map_or((minecraft_survival::menus::INVENTORY_MENU, 0), |view| {
            (view.menu, view.state)
        });
    let mut closed = false;
    for click in std::mem::take(&mut ui.clicks) {
        match click {
            frame::McClick::Close => {
                if !closed {
                    closed = true;
                    out.push(ToServer::CloseMenu { menu: id });
                }
                *menu = None;
                *shown_menu = None;
            }
            frame::McClick::Rename(name) => out.push(ToServer::RenameItem(name)),
            other => {
                if let Some(action) = crate::minecraft_inventory::action_of(&other) {
                    out.push(ToServer::Click(Click {
                        menu: id,
                        state,
                        action,
                    }));
                }
            }
        }
    }
    if *screen_open && !ui.inventory_open && !closed {
        out.push(ToServer::CloseMenu { menu: id });
        *menu = None;
        *shown_menu = None;
    }
    *screen_open = ui.inventory_open;
}

#[allow(clippy::too_many_arguments)]
fn present(
    friend: &mut Friend,
    view: &mut MinecraftWorldView,
    prediction: Option<&net::ClientPredictionState>,
    player: Option<&playerstate_iw4::PlayerState>,
    drawn_from: Option<&Transform>,
    aspect: f32,
    (dt, (world_fog, credits, ui_shown)): (f64, (bool, bool, bool)),
    now_ms: u64,
) {
    let Friend {
        prepared: Some(prepared),
        remote: Some(remote),
        light,
        light_volume_at,
        light_volume_age,
        cloud_center,
        environment_accumulator,
        environment_primed,
        weather,
        shapes,
        shape_ids,
        atlas_animation,
        animation_clock,
        spawn_gate,
        mining,
        cracks,
        sounds,
        dimension,
        switch_screen,
        activation,
        activation_tick,
        ..
    } = friend
    else {
        return;
    };
    let shown = remote.take_shown();
    remote.advance_clocks(dt);
    let Some(level) = remote.level.as_mut() else {
        return;
    };
    let band = minecraft_survival::bands::level(level.dimension);
    if shown.level {
        match prediction {
            Some(prediction) => {
                let (min_y, height) = minecraft_survival::bands::heights(level.dimension);
                let dead = player.is_some_and(|ps| ps.pm_type != 0);
                let previous = dimension.replace(level.dimension);
                match level_switch(previous, level.dimension, dead) {
                    Some((old, reason)) => {
                        *switch_screen = Some(reason);
                        sim::voxel::drop_level(old);
                        sim::voxel::set_level(band, level.origin, min_y, height);
                        sim::voxel::mark_level_switch();
                    }
                    None => {
                        sim::voxel::activate(prediction.0.world().clip_brushes(), vec![Vec::new()]);
                        shapes.clear();
                        shape_ids.clear();
                        sim::voxel::set_level(band, level.origin, min_y, height);
                    }
                }
            }
            None => diag::warn!(
                World,
                "Minecraft level from the host has no MW2 world to collide with"
            ),
        }
        view.origin = level.origin;
        view.generation += 1;
        view.uploads.clear();
        view.removed.clear();
        view.visible.clear();
        view.particles = Default::default();
        view.entity_meshes = Default::default();
        view.hand = Default::default();
        view.cracks = Default::default();
        view.clouds = None;
        view.light_volume = None;
        *light = Some(SkyLight::streamed());
        *light_volume_at = None;
        *cloud_center = None;
        *environment_accumulator = 0.0;
        *environment_primed = false;
        *weather = remote.weather;
        *mining = crate::minecraft_mining::Mining::default();
        cracks.clear();
        spawn_gate.start(now_ms);
    }
    crate::minecraft_world::apply_streamed(
        &prepared.registries,
        (shapes, shape_ids, band),
        (shown.loaded, shown.forgotten),
    );
    for ((x, y, z), state) in shown.blocks {
        let shape =
            crate::minecraft_world::shape_id(shapes, shape_ids, &prepared.registries, state);
        sim::voxel::set_block_shape(band, x, y, z, shape);
    }
    crate::minecraft_world::animate_atlas(
        view,
        atlas_animation,
        animation_clock,
        dt,
        &prepared.look.atlas,
    );
    let origin = level.origin;
    let standing = player.filter(|_| !spawn_gate.waiting());
    let (feet, eye, forward) = match standing {
        Some(ps) => (
            sim::voxel::to_block(origin, ps.origin),
            sim::voxel::to_block(
                origin,
                [
                    ps.origin[0],
                    ps.origin[1],
                    ps.origin[2] + ps.view_height_current,
                ],
            ),
            crate::minecraft_world::view_forward(ps.viewangles),
        ),
        None => (
            level.feet,
            [
                level.feet[0],
                level.feet[1] + SPAWN_EYE_HEIGHT,
                level.feet[2],
            ],
            glam::Vec3::Z,
        ),
    };
    let camera = crate::minecraft_world::cull_camera(
        drawn_from.filter(|_| standing.is_some()),
        origin,
        eye,
        forward,
        aspect,
    );
    let lights =
        crate::minecraft_world::terrain_frame(&mut level.terrain, &prepared.look, view, &camera);
    crate::minecraft_world::observe_spawn(
        &level.terrain,
        spawn_gate,
        (band, level.feet),
        camera.position,
        now_ms,
    );
    let Some(light) = light.as_mut() else {
        return;
    };
    for (chunk, column) in lights {
        light.set_chunk_column(chunk, column);
    }
    let partial = mining.tick(&level.terrain.scene, dt);
    view.particles = mining.particle_mesh(&prepared.look.atlas, forward, partial, light);
    let shown_cracks: Vec<_> = cracks
        .iter()
        .map(|(&(_, pos), &stage)| ((pos[0], pos[1], pos[2]), stage))
        .collect();
    view.cracks = mining.crack_mesh(
        &level.terrain.scene,
        &level.terrain.stream.states,
        &prepared.look.packs,
        &shown_cracks,
    );
    let step = (dt * 0.2) as f32;
    for (shown, target) in weather.iter_mut().zip(remote.weather) {
        *shown += (target - *shown).clamp(-step, step);
    }
    let lava = standing
        .filter(|ps| ps.pm_type == 0 && sim::voxel::eye_in_lava(ps.origin, ps.view_height_current))
        .map(|_| {
            let view = remote.player.as_ref();
            let spectator = view.is_some_and(|view| view.game_mode == GameMode::Spectator);
            let fire_resistant = view.is_some_and(|view| {
                view.effects
                    .iter()
                    .any(|effect| effect.kind == FIRE_RESISTANCE)
            });
            (spectator, fire_resistant)
        });
    let inputs = SkyInputs {
        eye,
        forward,
        aspect,
        dt,
        clock_ticks: remote.clocks.ticks(level.dimension),
        game_time: remote.clocks.overworld.ticks,
        rain_level: weather[0],
        thunder_level: weather[1],
        lava,
        powder_snow: standing
            .filter(|ps| ps.pm_type == 0)
            .filter(|_| {
                let at = (
                    eye[0].floor() as i32,
                    eye[1].floor() as i32,
                    eye[2].floor() as i32,
                );
                minecraft_terrain::scene::Scene::block(&level.terrain.scene, at)
                    .is_some_and(|b| b.id.key() == "minecraft:powder_snow")
            })
            .map(|_| {
                remote
                    .player
                    .as_ref()
                    .is_some_and(|view| view.game_mode == GameMode::Spectator)
            }),
        world_fog,
        credits,
    };
    view.weather = if view.weather_texture.is_some() {
        let rain = (weather[0], level.view.partial());
        level
            .view
            .weather_mesh(&level.terrain.scene, eye, light, rain)
    } else {
        Default::default()
    };
    let sky = crate::minecraft_world::present_sky(
        view,
        &mut level.terrain,
        &prepared.registries,
        light,
        (environment_accumulator, environment_primed),
        &inputs,
    );
    if let (Some((x_angle, y_angle)), Some(sounds)) = (sky.flash, sounds.as_mut()) {
        let toward = minecraft_terrain::end_flash::direction_from_rotation(x_angle, y_angle) * 10.0;
        let source = [eye[0] + toward.x, eye[1] + toward.y, eye[2] + toward.z];
        let at = Vec3::from_array(sim::voxel::to_map(origin, source));
        let flash = "minecraft:weather.end_flash";
        sounds.play_without_falloff(&prepared.look.packs, flash, Some(at), 1.0, 1.0);
    }
    crate::minecraft_world::present_clouds(
        view,
        &prepared.look,
        cloud_center,
        eye,
        inputs.game_time,
    );
    let block = (
        feet[0].floor() as i32,
        feet[1].floor() as i32,
        feet[2].floor() as i32,
    );
    crate::minecraft_world::present_light_volume(
        view,
        light,
        block,
        light_volume_at,
        light_volume_age,
    );
    present_entities(
        level,
        prepared,
        view,
        light,
        (feet, eye, forward),
        (
            dt,
            remote.game_time,
            standing.is_some_and(|ps| ps.pm_type == 0),
        ),
        (sounds.as_mut(), mining, weather[0]),
    );
    let now = level.view.ticks();
    let ticks = now.saturating_sub(*activation_tick);
    *activation_tick = now;
    activation.tick(u32::try_from(ticks).unwrap_or(u32::MAX));
    let pose = activation
        .pose(level.view.partial(), aspect)
        .filter(|_| ui_shown);
    view.hand = Default::default();
    if let Some(pose) = pose {
        let mut mesh = minecraft_terrain::mesh::ChunkMesh::default();
        let light_at = glam::Vec3::new(eye[0] as f32, eye[1] as f32, eye[2] as f32);
        let look = (&prepared.look.packs, &*prepared.look.atlas, &*light);
        crate::minecraft_hand::append_totem(&mut mesh, &mut level.view, pose, light_at, look);
        let vertices = minecraft_terrain::mesh::section_vertices(&mesh);
        view.hand = (bytemuck::cast_slice(&vertices).to_vec(), mesh.indices);
        view.hand_clip = crate::minecraft_hand::hand_clip(aspect);
    }
}

fn level_switch(
    previous: Option<minecraft_net::Dimension>,
    next: minecraft_net::Dimension,
    dead: bool,
) -> Option<(u8, LoadingReason)> {
    let previous = previous?;
    let reason = minecraft_survival::switch::loading_reason(dead, previous, next);
    Some((minecraft_survival::bands::level(previous), reason))
}

fn present_entities(
    level: &mut minecraft_survival::remote::RemoteLevel,
    prepared: &Prepared,
    view: &mut MinecraftWorldView,
    light: &SkyLight,
    (feet, eye, forward): ([f64; 3], [f64; 3], glam::Vec3),
    (dt, game_time, alive): (f64, i64, bool),
    (sounds, mining, rain_level): (
        Option<&mut Sounds>,
        &mut crate::minecraft_mining::Mining,
        f32,
    ),
) {
    let feet = glam::DVec3::from_array(feet);
    let near: Vec<glam::DVec3> = alive.then_some(feet).into_iter().collect();
    let entities = &mut level.view;
    entities.world.set_game_time(game_time);
    let before = entities.ticks();
    let timing = minecraft_survival::tick_rate::ClientTiming::default();
    entities.advance(dt, timing, &near, feet + glam::DVec3::Y * 0.81);
    let ticks = u32::try_from(entities.ticks() - before).unwrap_or(u32::MAX);
    entities.tick_scene(&level.terrain.scene, ticks);
    for _ in 0..ticks {
        let rain = entities.rain_tick(&level.terrain.scene, eye, rain_level);
        let ambient = entities.animate_tick(&level.terrain.scene, feet.to_array());
        for sound in ambient.into_iter().chain(rain) {
            let at = glam::DVec3::from_array(sound.at);
            let event = sound.event.to_owned();
            entities.sounds.push((event, at, sound.volume, sound.pitch));
        }
    }
    for (pos, apart) in entities.lids.take_apart_changes() {
        level.terrain.stream.draw_apart(pos, apart);
    }
    let (packs, atlas) = (&prepared.look.packs, &prepared.look.atlas);
    crate::minecraft_world::present_crumbs(entities, mining, packs, atlas);
    let heard = std::mem::take(&mut entities.sounds);
    if let Some(sounds) = sounds {
        let camera = glam::DVec3::from_array(eye);
        for (event, at, volume, pitch) in heard {
            let at = minecraft_terrain::ender_dragon::heard_at(&event, at, camera);
            let at = Vec3::from_array(sim::voxel::to_map(level.origin, at.to_array()));
            sounds.play(packs, &event, Some(at), volume, pitch);
        }
    }
    let sky_light = level.terrain.environment.sky_light_level();
    let sky_darken = (15.0 - sky_light).clamp(0.0, 15.0) as u8;
    let stream = &level.terrain.stream;
    let meshes = entities.meshes(
        &level.terrain.scene,
        &prepared.look.packs,
        &prepared.look.atlas,
        light,
        forward,
        glam::DVec3::from_array(eye),
        sky_darken,
        &|pos| stream.cell_settled(pos),
        &[],
    );
    let raw = |mesh: &minecraft_terrain::mesh::ChunkMesh| {
        (
            bytemuck::cast_slice::<_, u8>(&mesh.vertices).to_vec(),
            mesh.indices.clone(),
        )
    };
    view.entity_meshes = [
        raw(&meshes.models),
        raw(&meshes.culled),
        raw(&meshes.translucent),
        raw(&meshes.shadows),
    ];
    let (bytes, indices) = &mut view.particles;
    let vertex = std::mem::size_of::<minecraft_terrain::mesh::SectionVertex>();
    let base = (bytes.len() / vertex) as u32;
    let vertices = minecraft_terrain::mesh::section_vertices(&meshes.items);
    bytes.extend_from_slice(bytemuck::cast_slice(&vertices));
    indices.extend(meshes.items.indices.iter().map(|i| i + base));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dimension_change_drops_only_the_old_level_behind_its_loading_screen() {
        use minecraft_net::Dimension::{End, Nether, Overworld};
        assert_eq!(level_switch(None, Nether, false), None, "the first level activates the world");
        assert_eq!(
            level_switch(Some(Overworld), Nether, false),
            Some((0, LoadingReason::NetherPortal))
        );
        assert_eq!(
            level_switch(Some(Nether), Overworld, true),
            Some((1, LoadingReason::Other)),
            "a respawn shows the plain screen"
        );
        assert_eq!(
            level_switch(Some(Overworld), End, false),
            Some((0, LoadingReason::EndPortal))
        );
        assert_eq!(
            level_switch(Some(End), Overworld, false),
            Some((2, LoadingReason::EndPortal))
        );
    }

    #[test]
    fn only_a_host_that_never_answers_lets_the_friend_in_without_a_world() {
        let ready = Some(1_000);
        let late = 1_000 + CLIENT_WAIT_TIMEOUT_MS;
        assert!(!host_silent(None, late, false), "nothing to wait for yet");
        assert!(!host_silent(ready, late - 1, false));
        assert!(host_silent(ready, late, false));
        assert!(
            !host_silent(ready, late * 10, true),
            "a host that acknowledges the hello is loading its world"
        );
    }

    #[test]
    fn only_the_friends_own_totem_rises_in_their_view() {
        let pop = |client| ToClient::PlayerEvent {
            client,
            event: TOTEM_EVENT,
        };
        assert!(own_totem(&pop(2), Some(2)));
        assert!(!own_totem(&pop(3), Some(2)), "another player's totem");
        assert!(!own_totem(&pop(2), None), "before the login");
        let other = ToClient::PlayerEvent {
            client: 2,
            event: TOTEM_EVENT + 1,
        };
        assert!(!own_totem(&other, Some(2)));
        let mut activation = crate::minecraft_hand::ItemActivation::default();
        activation.activate(|| 0.5);
        assert!(activation.pose(0.0, 1.5).is_some());
        activation.tick(crate::minecraft_hand::ACTIVATION_TICKS);
        assert!(activation.pose(0.0, 1.5).is_none(), "it ends after two seconds");
    }

    #[test]
    fn the_friends_boss_bars_follow_the_hosts_and_slide_to_a_new_health() {
        use minecraft_net::messages::{BossBarColor, BossBarFlags, BossBarOverlay};
        let dragon = minecraft_survival::session::DRAGON_NAME;
        let bar = |progress| BossBar {
            name: Text::translate(dragon, Vec::new()),
            progress,
            color: BossBarColor::Pink,
            overlay: BossBarOverlay::Progress,
            flags: BossBarFlags {
                darken_screen: false,
                play_music: true,
                create_world_fog: true,
            },
        };
        let lang = |key: &str| (key == dragon).then(|| "Ender Dragon".to_owned());
        let name = |text: &Text| text.resolve(&lang).text;
        let mut shown = BTreeMap::new();
        let mut bars = BTreeMap::from([(1, bar(1.0))]);
        let ui = follow_boss_bars((&mut shown, &bars), name, 0.05);
        assert_eq!(ui.len(), 1);
        assert_eq!(ui[0].name, "Ender Dragon");
        assert!(ui[0].world_fog && ui[0].music);
        assert_eq!(ui[0].progress(), 1.0);
        bars.insert(1, bar(0.5));
        let ui = follow_boss_bars((&mut shown, &bars), name, 0.05);
        let halfway = (ui[0].progress() - 0.75).abs() < 1.0e-4;
        assert!(halfway, "it slides over a tenth of a second");
        let ui = follow_boss_bars((&mut shown, &bars), name, 0.1);
        assert_eq!(ui[0].progress(), 0.5);
        bars.clear();
        assert!(follow_boss_bars((&mut shown, &bars), name, 0.05).is_empty());
        assert!(shown.is_empty());
    }
}
