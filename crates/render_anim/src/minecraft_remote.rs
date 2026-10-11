use std::collections::HashMap;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use minecraft_commands::{Clientbound, Serverbound, Text};
use minecraft_net::messages::GameMode;
use minecraft_net::{Lane, StateFrame, ToClient, ToServer};
use minecraft_survival::guns::{Follow, weapon_of};
use minecraft_survival::remote::{DecodeJob, RemoteWorld};
use minecraft_survival::views::{HudClock, Selection, view_stack};
use minecraft_terrain::containers::Containers;
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::AtlasAnimation;
use minecraft_terrain::pack::PackStack;
use minecraftoss_core::registries::{DataPaths, Registries};
use minecraftoss_core::{BlockStateId, Chunk};
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::Inventory;
use net::minecraft_wire::HOST;

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
}

fn recipes(root: &std::path::Path) -> RecipeBook {
    let Some(jar) = crate::minecraft_world::data_jar() else {
        return RecipeBook::default();
    };
    let catalog = minecraft_terrain::crafting::find_item_catalog(Some(root), &jar);
    match minecraft_terrain::crafting::load_recipes(&jar, catalog.as_deref()) {
        Ok((recipes, _)) => recipes,
        Err(error) => {
            diag::warn!(World, "Minecraft recipes unavailable: {error:#}");
            RecipeBook::default()
        }
    }
}

fn prepare() -> Result<Prepared, String> {
    let root = assets::minecraft_map::root().ok_or_else(assets::minecraft_setup::status)?;
    let paths = DataPaths::under(&root);
    let registries = Arc::new(Registries::load(&paths)?);
    let packs = PackStack::open(vec![root.join("resourcepacks/local/minecraft-26.3")])
        .map_err(|error| error.to_string())?;
    let language =
        minecraft_terrain::item_icons::language(&packs).unwrap_or_else(|_| HashMap::new());
    let look = crate::minecraft_world::load_look(packs)?;
    let registry = minecraft_survival::session::registry_fingerprint(&registries);
    Ok(Prepared {
        look,
        registries,
        registry,
        language,
        recipes: Arc::new(recipes(&root)),
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
}

fn host_silent(ready_at: Option<u64>, now_ms: u64, heard: bool) -> bool {
    !heard && ready_at.is_some_and(|ready| now_ms.saturating_sub(ready) >= CLIENT_WAIT_TIMEOUT_MS)
}

impl Friend {
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
    ui.chat_only = false;
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
    remote.take_effects();
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
    prediction: Option<Res<net::ClientPredictionState>>,
    (local, presented): (Res<net::LocalPresentClient>, Res<net::PresentedSnapshot>),
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    (skate, cameras, mc_camera): (
        Res<frame::SkateMode>,
        Query<&Transform, With<render_scene::FlyCamera>>,
        Option<Res<frame::McCamera>>,
    ),
    mut ui: ResMut<frame::MinecraftUi>,
    mut view: ResMut<MinecraftWorldView>,
    mut friend: NonSendMut<Friend>,
    mut images: ResMut<Assets<Image>>,
) {
    if *role != frame::RuntimeRole::Client {
        if ui.chat_only {
            ui.chat_only = false;
        }
        return;
    }
    let friend = &mut *friend;
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
                friend.remote = Some(RemoteWorld::new(prepared.registries.clone()));
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
    ui.inventory_open = false;
    let sent = std::mem::take(&mut ui.chat.sent);
    let mut wire = wire.filter(|wire| wire.role() == Some(net::WireRole::Friend));
    if let Some(wire) = wire.as_mut() {
        let lines = exchange(friend, wire, now, now_ms);
        if let Some(prepared) = friend.prepared.as_ref().filter(|_| !lines.is_empty()) {
            crate::minecraft_chat::show(&mut ui.chat, &lines, &prepared.language);
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
    let drawn_from = cameras
        .iter()
        .next()
        .filter(|_| skate.active || mc_detached);
    present(
        friend,
        &mut view,
        prediction.as_deref(),
        player.as_ref(),
        drawn_from,
        aspect,
        dt,
        now_ms,
    );
    let seconds = real_time.elapsed_secs_f64();
    let shown = (dt, seconds, now_ms);
    let picked = show_player(friend, &mut ui, player.as_ref(), &mut images, shown);
    if let (Some(wire), Some(slot)) = (wire.as_mut(), picked) {
        send(wire, &ToServer::SelectSlot(slot));
    }
    ui.loading_world = friend.holding(now_ms);
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
) -> Option<u8> {
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
        ..
    } = friend
    else {
        return None;
    };
    let view = remote
        .player
        .as_ref()
        .filter(|_| remote.level.is_some() && remote.rejected.is_none())?;
    selection.follow(view.selected, seconds);
    let picked = ui
        .select
        .take()
        .and_then(|slot| selection.pick(slot, seconds));
    ui.drop_selected = None;
    ui.swap_hands = false;
    ui.clicks.clear();
    let interacts = view.game_mode != GameMode::Spectator;
    ui.game_mode = mc_game_mode(view.game_mode);
    let alive = ps.filter(|ps| ps.pm_type == 0 && !spawn_gate.holding(now_ms));
    ui.active = alive.is_some();
    let Some(ps) = alive else {
        ui.vitals = None;
        ui.weapon_request = None;
        ui.holding_item = false;
        ui.empty_hand = false;
        return picked;
    };
    let mut inventory = Inventory::default();
    inventory.recipes = prepared.recipes.clone();
    for (slot, stack) in inventory.slots.iter_mut().zip(&view.slots) {
        *slot = stack
            .as_ref()
            .and_then(|nbt| view_stack(nbt, &prepared.recipes));
    }
    let (mins, maxs) = sim::voxel::player_hull(ps);
    let eye_in_water = sim::voxel::water(ps.origin, mins, maxs, ps.view_height_current).eye;
    let eye_in_lava = sim::voxel::eye_in_lava(ps.origin, ps.view_height_current);
    let vitals = hud.vitals(view, dt, eye_in_water, eye_in_lava);
    ui.vitals = Some(crate::minecraft_world::mc_vitals(vitals));
    ui.fire_frames = fire_frames.clone();
    let held = inventory.slots[selection.slot].as_ref();
    ui.holding_item = interacts && held.is_none_or(|stack| weapon_of(stack).is_none());
    ui.empty_hand = ui.holding_item && held.is_none();
    let mut selected = selection.slot;
    ui.weapon_request = follow
        .request(&inventory, &mut selected, ps.weapon)
        .filter(|_| interacts);
    selection.slot = selected;
    inventory_ui.animation_tick = atlas_animation.ticks();
    let packs = &prepared.look.packs;
    let mut containers = Containers::default();
    inventory_ui.publish(
        ui,
        &inventory,
        selection.slot,
        packs,
        images,
        &mut containers,
    );
    picked
}

#[allow(clippy::too_many_arguments)]
fn present(
    friend: &mut Friend,
    view: &mut MinecraftWorldView,
    prediction: Option<&net::ClientPredictionState>,
    player: Option<&playerstate_iw4::PlayerState>,
    drawn_from: Option<&Transform>,
    aspect: f32,
    dt: f64,
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
    if shown.level {
        match prediction {
            Some(prediction) => {
                sim::voxel::activate(
                    prediction.0.world().clip_brushes(),
                    level.origin,
                    vec![Vec::new()],
                );
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
        shapes.clear();
        shape_ids.clear();
        *light = Some(SkyLight::streamed());
        *light_volume_at = None;
        *cloud_center = None;
        *environment_accumulator = 0.0;
        *environment_primed = false;
        *weather = remote.weather;
        spawn_gate.start(now_ms);
    }
    crate::minecraft_world::apply_streamed(
        &prepared.registries,
        shapes,
        shape_ids,
        (shown.loaded, shown.forgotten),
    );
    for ((x, y, z), state) in shown.blocks {
        let shape =
            crate::minecraft_world::shape_id(shapes, shape_ids, &prepared.registries, state);
        sim::voxel::set_block_shape(x, y, z, shape);
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
        level.feet,
        camera.position,
        now_ms,
    );
    let Some(light) = light.as_mut() else {
        return;
    };
    for (chunk, column) in lights {
        light.set_chunk_column(chunk, column);
    }
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
        world_fog: false,
    };
    crate::minecraft_world::present_sky(
        view,
        &mut level.terrain,
        &prepared.registries,
        light,
        (environment_accumulator, environment_primed),
        &inputs,
    );
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
