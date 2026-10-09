//! The Minecraft map at run time: MinecraftOSS generates and streams a seeded
//! world around the local player, whose chunks become block collision and
//! whose section meshes go to the renderer. The world sits with its player
//! spawn at map origin, a block to `sim::voxel::BLOCK` map units.
use std::collections::HashMap;
use std::sync::{Arc, mpsc};

use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use minecraft_terrain::clouds::CloudMask;
use minecraft_terrain::day_cycle::{DayCycle, Skybox};
use minecraft_terrain::environment::{DimensionEnvironment, View};
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{Atlas, AtlasAnimation, AtlasPatch, SectionMesh, Vertex};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::HandcraftedScene;
use minecraft_terrain::sections::{CullCamera, SectionPos};
use minecraft_terrain::terrain::{Dimension, TerrainStream};
use minecraftoss_core::BlockStateId;
use minecraftoss_core::registries::{DataPaths, Registries};

const VIEW_DISTANCE: i32 = 8;
const TICK_SECONDS: f64 = 1.0 / 20.0;
const CRAWLING_BELOW_VIEW_HEIGHT: f32 = 30.0;
const RECENT_PLACE_SECONDS: f64 = 1.0;
/// Blocks on a side of the light volume MW2 models are lit from.
pub const LIGHT_VOLUME: i32 = 64;
/// Chunk sections fade in over this long, as the viewer's default option.
const FADE_MILLIS: u64 = 750;

/// What the renderer takes from the world each frame.
#[derive(Resource, Default)]
pub struct MinecraftWorldView {
    /// A Minecraft map is loaded: the stand-in map's world is not drawn.
    pub active: bool,
    /// Block point at map origin.
    pub origin: [f64; 3],
    pub atlas: Option<Arc<Atlas>>,
    pub atlas_patches: Vec<AtlasPatch>,
    pub uploads: Vec<(SectionPos, SectionMesh)>,
    pub removed: Vec<SectionPos>,
    pub visible: Vec<(SectionPos, f32)>,
    /// Bumped when the world is replaced, so stale sections are dropped.
    pub generation: u64,
    /// The environment uniform of MinecraftOSS for this frame, in block space.
    pub environment: [[f32; 4]; 16],
    /// Sun and the eight moon phases, 32 pixels each, side by side.
    pub celestial: Option<Arc<image::RgbaImage>>,
    pub clouds: Option<Arc<(Vec<Vertex>, Vec<u32>)>>,
    /// Sky and block light around the player: origin block, then
    /// `LIGHT_VOLUME` cubed pairs, x fastest then z then y.
    pub light_volume: Option<Arc<([i32; 3], Vec<u8>)>>,
    /// Sky and block light at the eye, for the view model.
    pub eye_light: [f32; 2],
    /// Break particles as section vertices and indices, rebuilt each frame.
    pub particles: (Vec<u8>, Vec<u32>),
    pub cracks: (Vec<[f32; 6]>, Vec<u32>),
    /// The ten destroy stages side by side.
    pub crack_texture: Option<Arc<image::RgbaImage>>,
    pub glint_texture: Option<Arc<image::RgbaImage>>,
    /// Mob models (cut out, back-face culled, translucent) and entity
    /// shadows, as `mesh::Vertex` bytes and indices.
    pub entity_meshes: [(Vec<u8>, Vec<u32>); 4],
    /// The black card behind the inventory's character, in blocks.
    pub backdrop: Option<[[f32; 3]; 4]>,
    /// The first-person hand or held item: section vertex bytes in view
    /// space (x right, y up, z back) and indices, with the projection that
    /// draws them (vanilla's fixed 70 degree hand field of view).
    pub hand: (Vec<u8>, Vec<u32>),
    pub hand_clip: [f32; 16],
}

struct Loaded {
    packs: PackStack,
    atlas: Arc<Atlas>,
    celestial: Arc<image::RgbaImage>,
    cloud_mask: Option<CloudMask>,
    crack_texture: Arc<image::RgbaImage>,
    glint_texture: Option<Arc<image::RgbaImage>>,
}

/// The hand's swing and the timers of mining and placing by hand.
#[derive(Default)]
struct HandState {
    /// Ticks into a swing, while one runs.
    swing: Option<f32>,
    /// Seconds towards the next hand-mining and placing tick.
    clock: f64,
    /// Ticks until another placement while the button is held
    /// (`rightClickDelay`).
    place_delay: u32,
    pearl_cooldown: u32,
    recent: Vec<((i32, i32, i32), minecraftoss_player::Block, f64)>,
    attack_ticks: u32,
    attack_item: Option<String>,
    miss_time: u32,
    blocking: Option<minecraft_survival::offhand::Blocking>,
    swing_offhand: bool,
    use_by_key: bool,
    offhand_key_was: bool,
    creative_delay: u32,
}

/// The player's walk, for vanilla's step and fall sounds.
#[derive(Default)]
struct StepState {
    last: Option<[f64; 3]>,
    /// `Entity.moveDist` and `nextStep`.
    move_dist: f32,
    next_step: f32,
    /// The highest point since leaving the ground.
    air_peak: Option<f64>,
}

#[derive(Default)]
struct Runtime {
    loading: Option<mpsc::Receiver<Result<(Loaded, minecraft_survival::server::Server), String>>>,
    world: Option<Loaded>,
    server: Option<minecraft_survival::server::Server>,
    local: Option<minecraft_net::PlayerUuid>,
    carried_survival: Option<minecraft_survival::survival::Survival>,
    environment_accumulator: f64,
    environment_primed: bool,
    light: Option<SkyLight>,
    light_volume_at: Option<[i32; 3]>,
    light_volume_age: u32,
    cloud_center: Option<(i32, i32)>,
    mining: crate::minecraft_mining::Mining,
    minimap: crate::minecraft_minimap::Minimap,
    hand: HandState,
    steps: StepState,
    sounds: Option<crate::minecraft_sounds::Sounds>,
    inventory_ui: crate::minecraft_inventory::InventoryUi,
    /// Shape id of each block state already seen.
    shapes: HashMap<BlockStateId, u16>,
    /// Boxes of each shape id, to reuse an id for a repeated shape.
    shape_ids: HashMap<(Vec<[u32; 6]>, u16), u16>,
    fire_frames: Option<(Handle<Image>, u32)>,
    atlas_animation: AtlasAnimation,
    animation_clock: f64,
    spawn_gate: crate::minecraft_spawn_gate::SpawnGate,
    loader: Option<std::thread::JoinHandle<()>>,
    closing: Vec<std::thread::JoinHandle<()>>,
}

impl Drop for Runtime {
    fn drop(&mut self) {
        close_world(self, true);
    }
}

/// The player's MW2 body stands in the inventory's character window, on a
/// black card: placed from this frame's camera and projection where the
/// window shows, facing the camera, turned and aiming towards the mouse,
/// and drawn with the card in the view model's depth band so no wall comes
/// between.
pub(crate) fn place_inventory_puppet(
    ui: Res<frame::MinecraftUi>,
    mut puppet: ResMut<frame::InventoryPuppet>,
    mut view: ResMut<MinecraftWorldView>,
    local: Res<net::LocalPresentClient>,
    presented: Res<net::PresentedSnapshot>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<&Transform, With<render_scene::FlyCamera>>,
    lenses: Query<&Projection, With<render_scene::FpvLens>>,
) {
    use std::sync::atomic::Ordering::Relaxed;
    puppet.active = false;
    view.backdrop = None;
    render_frame::DEPTH_HACK_SCENE_ENTNUM.store(u32::MAX, Relaxed);
    let alive = presented.player(local.0).is_some_and(|ps| ps.pm_type == 0);
    if !(ui.active && ui.inventory_open && alive) {
        return;
    }
    let (Some([cx, cy, box_w, box_h]), Ok(window), Ok(camera), Ok(Projection::Perspective(lens))) =
        (ui.character_box, windows.single(), cameras.single(), lenses.single())
    else {
        return;
    };
    let (w, h) = (window.width().max(1.0), window.height().max(1.0));
    let tan_v = (lens.fov * 0.5).tan();
    let tan_h = tan_v * if lens.aspect_ratio > 1e-3 { lens.aspect_ratio } else { w / h };
    let (eye, fwd, right, up) = (camera.translation, *camera.forward(), *camera.right(), *camera.up());
    // A point `distance` along the view at a window pixel.
    let at = |px: f32, py: f32, distance: f32| {
        let (nx, ny) = (px / w * 2.0 - 1.0, 1.0 - py / h * 2.0);
        eye + (fwd + right * (nx * tan_h) + up * (ny * tan_v)) * distance
    };
    let distance = 10.0;
    let world_h = box_h / h * 2.0 * tan_v * distance;
    let scale = world_h * crate::minecraft_gui::PUPPET_HEIGHT / 72.0;
    let feet = at(cx, cy + box_h * crate::minecraft_gui::PUPPET_FEET, distance);
    let [yaw, pitch, tilt] = ui.puppet_turn;
    let [x_axis, y_axis, up_axis] = crate::minecraft_gui::puppet_axes(fwd, right, up, yaw, tilt);
    puppet.root = Mat4::from_cols(
        (x_axis * scale).extend(0.0),
        (y_axis * scale).extend(0.0),
        (up_axis * scale).extend(0.0),
        feet.extend(1.0),
    );
    puppet.pitch = pitch;
    puppet.client = local.0.0;
    puppet.active = true;
    render_frame::DEPTH_HACK_SCENE_ENTNUM.store(local.0.0, Relaxed);
    // The card: the window and a margin the panel covers, behind it.
    let corner = |dx: f32, dy: f32| {
        let p = at(cx + dx * box_w * 0.6, cy + dy * box_h * 0.6, distance * 1.4);
        let b = sim::voxel::to_block(view.origin, p.to_array());
        [b[0] as f32, b[1] as f32, b[2] as f32]
    };
    view.backdrop = Some([corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)]);
}

pub(crate) fn register(app: &mut App) {
    crate::minecraft_camera::register(app);
    app.add_systems(
        Update,
        place_inventory_puppet
            .after(crate::sync_camera_from_presented)
            .after(frame::PresentedPublished)
            .in_set(frame::ClientSet::Present),
    );
    app.init_resource::<MinecraftWorldView>()
        .init_resource::<frame::MinecraftUi>()
        .init_resource::<frame::InventoryPuppet>()
        .insert_non_send(Runtime::default())
        .add_systems(
            Update,
            update
                .after(frame::PresentedPublished)
                .in_set(frame::ClientSet::Present),
        )
        .add_systems(
            Last,
            save_before_exit
                .in_set(frame::SaveBeforeExit)
                .after(bevy::window::ExitSystems),
        );
}

fn load() -> Result<(Loaded, minecraft_survival::server::Server), String> {
    let root = assets::minecraft_map::root().ok_or_else(assets::minecraft_setup::status)?;
    let opened = minecraft_survival::save::open_world(
        &minecraft_survival::save::saves_dir(),
        std::env::var("IW4L_MINECRAFT_WORLD").ok().as_deref(),
        std::env::var("IW4L_MINECRAFT_SEED").ok().as_deref(),
        minecraft_survival::save::random_seed(),
    );
    for note in &opened.notes {
        diag::info!(World, "{note}");
    }
    for warning in &opened.warnings {
        diag::warn!(World, "{warning}");
    }
    let seed = opened.seed;
    let paths = DataPaths::under(&root);
    let registries = Arc::new(Registries::load(&paths)?);
    let packs = PackStack::open(vec![root.join("resourcepacks/local/minecraft-26.3")])
        .map_err(|e| e.to_string())?;
    let stream = TerrainStream::for_dimension_at(
        registries.clone(),
        seed,
        VIEW_DISTANCE,
        Dimension::Overworld,
        opened.save.as_ref().map(minecraft_survival::save::WorldSave::path),
        opened.spawn,
    )
    .map_err(|e| e.to_string())?;
    if let Some(save) = &opened.save
        && let Err(error) = save.write(stream.world_spawn, None, &opened.game_rules, opened.difficulty)
    {
        diag::warn!(World, "Minecraft level.dat not written: {error}");
    }
    let mut chat = minecraft_commands::CommandServer::new(minecraft_survival::commands::HOST);
    *chat.registries_mut() = minecraft_survival::commands::registries(&packs);
    if let Some(save) = &opened.save
        && let Err(error) = chat.open_ops(&save.ops_path())
    {
        diag::warn!(World, "Minecraft ops.json unreadable, no operators loaded: {error}");
    }
    let build = minecraft_terrain::mesh::build(&HandcraftedScene::default(), &packs)
        .map_err(|e| e.to_string())?;
    let scene = HandcraftedScene::streamed(stream.states.clone());
    let environment =
        DimensionEnvironment::load(&registries, Dimension::Overworld.dimension_type())?;
    let celestial = Arc::new(celestial_image(&packs).map_err(|e| e.to_string())?);
    let cloud_mask = CloudMask::from_pack(&packs).ok();
    let crack_texture = Arc::new(crate::minecraft_mining::crack_strip(&packs).map_err(|e| e.to_string())?);
    let glint_texture = minecraft_terrain::glint::glint_texture(&packs).ok().flatten().map(Arc::new);
    let loaded = Loaded {
        packs,
        atlas: build.atlas,
        celestial,
        cloud_mask,
        crack_texture,
        glint_texture,
    };
    let mut day = DayCycle::default();
    if let Some(ticks) = opened.day_ticks {
        day.set(ticks);
    }
    let globals = minecraft_survival::server::Globals {
        seed,
        registries,
        save: opened.save,
        game_rules: opened.game_rules,
        difficulty: opened.difficulty,
        day,
        chat,
        data: Default::default(),
        level_writer: None,
        autosave_ticks: 0,
    };
    let terrain = minecraft_survival::level::Terrain { stream, scene, environment };
    let level = minecraft_survival::level::Level { dimension: Dimension::Overworld, entities: None, containers: Default::default(), terrain };
    Ok((loaded, minecraft_survival::server::Server { globals, levels: std::collections::BTreeMap::from([(Dimension::Overworld, level)]), players: std::collections::BTreeMap::new() }))
}

pub(crate) fn data_jar() -> Option<std::path::PathBuf> {
    let root = assets::minecraft_map::root()?;
    let fetched = root.join(assets::minecraft_setup::CLIENT_JAR);
    if fetched.is_file() {
        return Some(fetched);
    }
    let root = root.join("harness/.gradle/loom-cache/minecraftMaven/net/minecraft");
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("minecraft-common-") {
            continue;
        }
        for jar in std::fs::read_dir(entry.path().join("26.3")).ok()?.flatten() {
            if jar.path().extension().is_some_and(|ext| ext == "jar") {
                return Some(jar.path());
            }
        }
    }
    None
}

/// The sky's sun and moon phases, laid out as MinecraftOSS lays them out.
fn celestial_image(packs: &PackStack) -> anyhow::Result<image::RgbaImage> {
    let mut celestial = image::RgbaImage::new(32 * 9, 32);
    let names = [
        "environment/celestial/sun",
        "environment/celestial/moon/full_moon",
        "environment/celestial/moon/waning_gibbous",
        "environment/celestial/moon/third_quarter",
        "environment/celestial/moon/waning_crescent",
        "environment/celestial/moon/new_moon",
        "environment/celestial/moon/waxing_crescent",
        "environment/celestial/moon/first_quarter",
        "environment/celestial/moon/waxing_gibbous",
    ];
    for (index, path) in names.iter().enumerate() {
        let id = minecraft_terrain::pack::ResourceId::parse(&format!("minecraft:{path}"))?;
        if let Some(bytes) = packs.texture(&id)? {
            let img =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
            let tile =
                image::imageops::resize(&img, 32, 32, image::imageops::FilterType::Nearest);
            image::imageops::replace(&mut celestial, &tile, (index as i64) * 32, 0);
        }
    }
    Ok(celestial)
}

fn fire_frames(packs: &PackStack) -> Option<(Image, u32)> {
    let id = minecraft_terrain::pack::ResourceId::parse("minecraft:block/fire_1").ok()?;
    let bytes = packs.texture(&id).ok()??;
    let strip = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?.to_rgba8();
    let (width, height) = strip.dimensions();
    let frames = (height / width.max(1)).max(1);
    let image = Image {
        sampler: bevy::image::ImageSampler::nearest(),
        ..Image::new(
            bevy::render::render_resource::Extent3d { width, height, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            strip.into_raw(),
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        )
    };
    Some((image, frames))
}

fn minecraft_yaw(mw2_yaw: f32) -> f32 {
    let yaw_rad = mw2_yaw.to_radians();
    (-yaw_rad.cos()).atan2(-yaw_rad.sin()).to_degrees()
}

fn show_chat(chat: &mut frame::McChat, inventory_ui: &crate::minecraft_inventory::InventoryUi, packs: &PackStack, delivered: &[minecraft_commands::Clientbound]) {
    match inventory_ui.language() {
        Some(language) => crate::minecraft_chat::show(chat, delivered, language),
        None => crate::minecraft_chat::show(chat, delivered, &minecraft_terrain::item_icons::language(packs).unwrap_or_else(|_| HashMap::new())),
    }
}

/// A controller trigger, held or just pressed: with a block or an empty hand
/// the right trigger mines and the left places, as the mouse buttons do.
fn pad_trigger(pad: Option<&bevy::input::gamepad::Gamepad>, button: GamepadButton, just: bool) -> bool {
    pad.is_some_and(|pad| if just { pad.just_pressed(button) } else { pad.pressed(button) })
}

#[allow(clippy::too_many_arguments)]
fn update(
    time: Res<Time>,
    mut installed: MessageReader<frame::MatchInstalled>,
    mut torn_down: MessageReader<frame::MatchTornDown>,
    local: Res<net::LocalPresentClient>,
    presented: Res<net::PresentedSnapshot>,
    authority: Option<ResMut<net::AuthorityWorld>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    (mut ui, mut puppet, mut images, mut sound_queue, buttons): (
        ResMut<frame::MinecraftUi>,
        ResMut<frame::InventoryPuppet>,
        ResMut<Assets<Image>>,
        ResMut<audio::McSoundQueue>,
        Res<ButtonInput<MouseButton>>,
    ),
    mut view: ResMut<MinecraftWorldView>,
    mut runtime: NonSendMut<Runtime>,
    (skate, cameras, gamepads, active_pad): (
        Res<frame::SkateMode>,
        Query<&Transform, With<render_scene::FlyCamera>>,
        Query<&bevy::input::gamepad::Gamepad>,
        Option<Res<frame::ActivePad>>,
    ),
    (hud_input, settings): (Option<Res<frame::HudInputView>>, Option<Res<frame::GameSettings>>),
    real_time: Res<Time<Real>>,
    mc_camera: Option<Res<frame::McCamera>>,
) {
    let pad = active_pad.and_then(|active| active.0).and_then(|entity| gamepads.get(entity).ok());
    for _ in torn_down.read() {
        stop(&mut runtime, &mut view);
        ui.chat.clear_messages();
    }
    let now_ms = real_time.elapsed().as_millis() as u64;
    for match_ in installed.read() {
        stop(&mut runtime, &mut view);
        ui.chat.clear_messages();
        if assets::minecraft_map::is_minecraft(&match_.zone) {
            let closing = std::mem::take(&mut runtime.closing);
            let (send, receive) = mpsc::channel();
            runtime.loader = std::thread::Builder::new()
                .name("minecraft-world-load".into())
                .spawn(move || {
                    for previous in closing {
                        let _ = previous.join();
                    }
                    let _ = send.send(load());
                })
                .ok();
            runtime.loading = Some(receive);
            view.active = true;
        }
    }
    ui.loading_world = runtime.loading.is_some() || (runtime.world.is_some() && runtime.spawn_gate.holding(now_ms));
    let Some(mut authority) = authority else {
        return;
    };

    if let Some(receive) = &runtime.loading
        && let Ok(result) = receive.try_recv()
    {
        runtime.loading = None;
        runtime.loader = None;
        match result {
            Ok((world, mut server)) => {
                let minecraft_survival::server::Server { globals, levels, players } = &mut server;
                let Some(level) = levels.get_mut(&Dimension::Overworld) else {
                    diag::warn!(World, "Minecraft world failed to load: no overworld");
                    view.active = false;
                    return;
                };
                let host = settings.as_deref().map_or_else(|| "Player".to_owned(), |settings| settings.player_name.clone());
                globals.chat.join(minecraft_survival::commands::HOST, host);
                let Some(profile) = globals.chat.profile(minecraft_survival::commands::HOST).cloned() else {
                    diag::warn!(World, "Minecraft world failed to load: no host profile");
                    view.active = false;
                    return;
                };
                let (x, y, z) = level.terrain.stream.player_spawn;
                view.origin = [x, y, z];
                view.atlas = Some(world.atlas.clone());
                view.atlas_patches.clear();
                runtime.atlas_animation = AtlasAnimation::new(&world.atlas);
                runtime.animation_clock = 0.0;
                view.celestial = Some(world.celestial.clone());
                view.crack_texture = Some(world.crack_texture.clone());
                view.glint_texture = world.glint_texture.clone();
                runtime.mining = Default::default();
                runtime.minimap = Default::default();
                runtime.sounds = Some(crate::minecraft_sounds::Sounds::load(&world.packs));
                globals.data = minecraft_survival::data::GameData::load(assets::minecraft_map::root().as_deref(), data_jar());
                let mut entities = minecraft_survival::entities::Entities::new(&level.terrain.stream, globals.seed, &globals.data);
                let inventory = minecraft_survival::player::starting_inventory(globals.data.recipes.clone(), std::env::var("IW4L_MINECRAFT_KIT").ok().as_deref());
                entities.set_game_rules(&globals.game_rules);
                level.containers.set_book(globals.data.stations.clone());
                runtime.inventory_ui.set_station_data(globals.data.stations.clone(), globals.data.enchantments.clone(), globals.seed as i32);
                globals.chat.registries_mut().enchantments = globals.data.enchantments.as_ref().map_or_else(Vec::new, |book| book.list.iter().map(|enchantment| enchantment.id.clone()).collect());
                // Game ticks since sunrise to start at: 6000 noon, 13000
                // dusk, 18000 midnight.
                if let Some(ticks) = std::env::var("IW4L_MINECRAFT_TIME")
                    .ok()
                    .and_then(|t| t.trim().parse::<f64>().ok())
                {
                    globals.day.set(ticks);
                }
                if let Some(save) = globals.save.as_mut() {
                    save.started(globals.day.ticks);
                    if let Some(host) = globals.chat.profile(minecraft_survival::commands::HOST) {
                        save.set_host(&host.uuid);
                    }
                }
                let saved_mode = globals.save.as_ref().map(minecraft_survival::save::WorldSave::host_mode);
                if let Some((_, Some(warning))) = &saved_mode {
                    diag::warn!(World, "{warning}");
                }
                let player = minecraft_survival::player::Player::new(&profile, starting_game_mode(saved_mode.map(|(mode, _)| mode)), runtime.carried_survival.take().unwrap_or_else(minecraft_survival::survival::Survival::default), inventory);
                let uuid = player.uuid.clone();
                players.insert(uuid.clone(), player);
                runtime.environment_accumulator = 0.0;
                runtime.environment_primed = false;
                runtime.light = Some(SkyLight::streamed());
                runtime.light_volume_at = None;
                runtime.cloud_center = None;
                view.generation += 1;
                sim::voxel::activate(
                    authority.0.content().clip_brushes(),
                    view.origin,
                    vec![Vec::new()],
                );
                diag::info!(
                    World,
                    "Minecraft world ready: seed {} spawn {:?}",
                    globals.seed,
                    level.terrain.stream.player_spawn
                );
                runtime.fire_frames = fire_frames(&world.packs).map(|(image, frames)| (images.add(image), frames));
                level.entities = Some(entities);
                runtime.server = Some(server);
                runtime.local = Some(uuid);
                runtime.world = Some(world);
                runtime.shapes.clear();
                runtime.shape_ids.clear();
                runtime.spawn_gate.start(now_ms);
            }
            Err(error) => {
                diag::warn!(World, "Minecraft world failed to load: {error}");
                view.active = false;
            }
        }
    }

    let origin = view.origin;
    let Runtime {
        world,
        server,
        local: local_uuid,
        shapes,
        shape_ids,
        environment_accumulator,
        environment_primed,
        light,
        light_volume_at,
        light_volume_age,
        cloud_center,
        mining,
        inventory_ui,
        sounds,
        hand,
        minimap,
        steps,
        fire_frames,
        atlas_animation,
        animation_clock,
        spawn_gate,
        ..
    } = &mut *runtime;
    let (Some(world), Some(server), Some(local_uuid)) = (world.as_mut(), server.as_mut(), local_uuid.as_ref()) else {
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    };
    let minecraft_survival::server::Server { globals, levels, players } = server;
    let Some(player) = players.get_mut(local_uuid) else {
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    };
    let Some(level) = levels.get_mut(&player.dimension) else {
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    };
    let minecraft_survival::player::Player { survival, mode: player_mode, death_watch, inventory, selected, .. } = player;
    let minecraft_survival::level::Level { entities, containers, terrain, .. } = level;
    let host_alive = presented.player(local.0).is_some_and(|ps| ps.pm_type == 0);
    let host_pose = presented
        .player(local.0)
        .filter(|_| host_alive)
        .map(|ps| minecraft_commands::Pose { feet: sim::voxel::to_block(origin, ps.origin), yaw: minecraft_yaw(ps.viewangles[1]), pitch: ps.viewangles[0] })
        .or_else(|| survival.last_feet().map(|feet| minecraft_commands::Pose { feet, yaw: 0.0, pitch: 0.0 }));
    let mut host = minecraft_survival::commands::Host {
        seed: globals.seed,
        rules: &mut globals.game_rules,
        difficulty: &mut globals.difficulty,
        day: &mut globals.day,
        save: globals.save.as_mut(),
        survival,
        entities: entities.as_mut(),
        inventory,
        selected: *selected,
        player_mode,
        rules_changed: false,
        mode_changed: false,
        pose: host_pose,
        alive: host_alive,
        teleports: Vec::new(),
    };
    let delivered = minecraft_survival::commands::pump(&mut ui.chat.sent, &mut globals.chat, &mut host);
    let mode_changed = host.mode_changed;
    let teleports = std::mem::take(&mut host.teleports);
    if host.rules_changed
        && let Some(entities) = entities.as_mut()
    {
        entities.set_game_rules(&globals.game_rules);
    }
    for warning in globals.chat.take_warnings() {
        diag::warn!(World, "{warning}");
    }
    if mode_changed {
        save_player_mode(globals, *player_mode);
    }
    if !delivered.is_empty() {
        show_chat(&mut ui.chat, inventory_ui, &world.packs, &delivered);
    }
    *animation_clock += time.delta_secs_f64();
    if *animation_clock >= TICK_SECONDS {
        *animation_clock %= TICK_SECONDS;
        let patches = atlas_animation.tick(&world.atlas);
        view.atlas_patches.retain(|queued| patches.iter().all(|patch| patch.origin != queued.origin));
        view.atlas_patches.extend(patches);
    }
    let aspect = windows
        .single()
        .map(|w| w.width() / w.height().max(1.0))
        .unwrap_or(16.0 / 9.0);
    let Some(ps) = presented.player(local.0) else {
        ui.active = false;
        puppet.active = false;
        stream_around_spawn(world, terrain, &globals.registries, shapes, shape_ids, entities, light, spawn_gate, &mut view, aspect, now_ms);
        return;
    };

    let alive = ps.pm_type == 0;
    let dead = matches!(
        ps.pm_type,
        playerstate_iw4::PM_TYPE_SPECTATOR | playerstate_iw4::PM_TYPE_INTERMISSION | playerstate_iw4::PM_TYPE_DEAD | playerstate_iw4::PM_TYPE_DEAD_LINKED
    );
    let (in_world, died) = minecraft_survival::death::life_state(ps.pm_type);
    if sim::voxel::take_team_switches().contains(&local.0.0) {
        death_watch.switch_teams();
    }
    let keep_inventory = minecraft_survival::death::keeps_belongings(globals.game_rules.keep_inventory(), player_mode.mode);
    survival.rules = minecraft_survival::survival::SurvivalRules::of(&globals.game_rules, globals.difficulty);
    survival.set_game_mode(player_mode);
    authority.0.set_minecraft_abilities(local.0, ability_bits(player_mode));
    if alive && player_mode.abilities.may_fly {
        player_mode.abilities.flying = authority.0.player(local.0).is_some_and(|p| p.pm_flags & playerstate_iw4::pm_flags::MC_FLYING != 0);
    }
    ui.game_mode = ui_game_mode(player_mode.mode);
    death_watch.observe(in_world, died, survival.status.experience_level, keep_inventory);
    if dead {
        if keep_inventory || death_watch.switched_teams() {
            survival.respawn_keeping_experience();
        } else {
            survival.respawn();
        }
    } else if !sim::voxel::player_damage_pending()
        && let Some(health) = authority.0.player(local.0).map(|p| (p.health, p.max_health))
    {
        survival.observe(health.0, health.1);
    }
    let totem_popped = sim::voxel::take_totem_pops().contains(&local.0.0);
    for (_, killer) in sim::voxel::take_minecraft_deaths().into_iter().filter(|(client, _)| *client == local.0.0) {
        let seq = ui.killed_by.as_ref().map_or(0, |(seq, _)| seq.wrapping_add(1));
        ui.killed_by = Some((seq, killer));
    }

    let feet = sim::voxel::to_block(origin, ps.origin);
    let block = (
        feet[0].floor() as i32,
        feet[1].floor() as i32,
        feet[2].floor() as i32,
    );
    let streamed = terrain.stream.server_tick(block, &mut terrain.scene);
    apply_streamed(&globals.registries, shapes, shape_ids, entities, streamed);

    // Minecraft's yaw: 0 facing +Z (map -Y), turning towards -X.
    let mc_yaw = minecraft_yaw(ps.viewangles[1]);
    let mut all_events = sim::voxel::take_events();
    let mut hand_mined = Vec::new();
    let mut creative_broken = Vec::new();

    // The hand, when no gun is selected: vanilla's left click mines by hand
    // (with the hand's break speed) or punches, its right click places the
    // held block (`Player.place_selected`, vanilla's placement states).
    // Footsteps (`Entity.applyMovementEmissionAndPlaySound`): the walked
    // distance grows by 0.6 of each horizontal move on the ground, and past
    // the next step the block under the feet sounds its step at 0.15 of its
    // volume. A fall of more than three blocks lands with the block's fall
    // sound and the player's (`LivingEntity.causeFallDamage`).
    let on_ground = alive && ps.ground_entity_num != playerstate_iw4::ENTITYNUM_NONE;
    let (hull_mins, hull_maxs) = sim::voxel::player_hull(ps);
    let water = sim::voxel::water(ps.origin, hull_mins, hull_maxs, ps.view_height_current);
    if water.wet() {
        steps.air_peak = None;
    }
    if let Some(last) = steps.last.filter(|_| alive && !player_mode.noclip()) {
        let climbing = sim::voxel::climb(ps.origin).on;
        let horizontal = ((feet[0] - last[0]).hypot(feet[2] - last[2]).hypot(if climbing { feet[1] - last[1] } else { 0.0 }) * 0.6) as f32;
        let block_at = |dy: f64| {
            let pos = (feet[0].floor() as i32, (feet[1] - dy).floor() as i32, feet[2].floor() as i32);
            minecraft_terrain::scene::Scene::block(&terrain.scene, pos).cloned().map(|b| (pos, b))
        };
        let under = block_at(0.2);
        if climbing {
            survival.set_climbable(Some(block_at(0.0).map_or(minecraft_survival::death::FallLocation::OtherClimbable, |(_, b)| minecraft_survival::death::FallLocation::of_climbable(&b.id.key()))));
        }
        let centre = |pos: (i32, i32, i32)| {
            Vec3::from_array(sim::voxel::to_map(origin, [pos.0 as f64 + 0.5, pos.1 as f64 + 1.0, pos.2 as f64 + 0.5]))
        };
        if (on_ground || climbing) && horizontal < 2.0 {
            steps.move_dist += horizontal;
            if steps.move_dist > steps.next_step
                && let Some((pos, block)) = under.as_ref()
            {
                steps.next_step = steps.move_dist as i32 as f32 + 1.0;
                let sounding = on_ground
                    || block.id.path == "powder_snow"
                    || terrain.stream.states.state_of(block).is_some_and(|state| {
                        globals.registries.block_tags.id("minecraft:climbable").is_some_and(|tag| globals.registries.block_in_tag(state, tag))
                    });
                // Snow layers and carpets sound instead of what they lie on.
                let inside = block_at(-0.01).filter(|(_, b)| {
                    let p = b.id.path.as_str();
                    p == "snow" || p.ends_with("_carpet") || p == "moss_carpet"
                });
                let (pos, block) = inside.as_ref().map_or((*pos, block), |(p, b)| (*p, b));
                if sounding && let (Some(sounds), Some(kind)) = (sounds.as_mut(), terrain.scene.sound_type(block)) {
                    sounds.play(&world.packs, &kind.step, Some(centre(pos)), kind.volume * 0.15, kind.pitch);
                }
            }
        }
        let caught = [-0.01, -1.0]
            .into_iter()
            .any(|dy| block_at(dy).is_some_and(|(_, b)| minecraft_survival::survival::breaks_fall(&b.id.key())));
        if on_ground {
            if let Some(peak) = steps.air_peak.take().filter(|_| !caught) {
                let fall = peak - feet[1];
                let landed_on = under.as_ref().map(|(_, b)| b.id.key());
                let crouching = ps.pm_flags & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE) != 0;
                let damage = survival.fall(fall, landed_on.as_deref(), crouching);
                if damage > 0
                    && let Some(sounds) = sounds.as_mut()
                {
                    sounds.play(&world.packs, minecraft_survival::survival::fall_sound(damage), None, 1.0, 1.0);
                    let voice = (sounds.random() - sounds.random()) * 0.2 + 1.0;
                    sounds.play(&world.packs, "minecraft:entity.player.hurt", None, 1.0, voice);
                    if let Some((pos, block)) = under.as_ref()
                        && let Some(kind) = terrain.scene.sound_type(block)
                    {
                        sounds.play(&world.packs, &kind.fall, Some(centre(*pos)), kind.volume * 0.5, kind.pitch * 0.75);
                    }
                }
            }
            survival.set_climbable(None);
        } else if caught || ps.pm_flags & playerstate_iw4::pm_flags::MC_FLYING != 0 || (climbing && sim::voxel::lava(ps.origin, hull_mins, hull_maxs) <= 0.0) {
            steps.air_peak = Some(feet[1]);
        } else {
            steps.air_peak = Some(steps.air_peak.map_or(feet[1], |p| p.max(feet[1])));
        }
    } else {
        steps.air_peak = None;
    }
    steps.last = alive.then_some(feet);

    let dt_hand = time.delta_secs_f64();
    if let Some(ticks) = hand.swing.as_mut() {
        *ticks += (dt_hand * 20.0) as f32;
        if *ticks >= crate::minecraft_hand::SWING_TICKS {
            hand.swing = None;
        }
    }
    let holding = entities.as_ref().is_some_and(|_| {
        inventory.slots[*selected].as_ref().is_none_or(|s| minecraft_survival::guns::weapon_of(s).is_none())
    });
    ui.holding_item = alive && holding && player_mode.interacts();
    ui.empty_hand = ui.holding_item
        && entities.as_ref().is_some_and(|_| inventory.slots[*selected].is_none());
    hand.clock += dt_hand;
    let hand_ticks = (hand.clock / TICK_SECONDS) as u32;
    hand.clock -= f64::from(hand_ticks) * TICK_SECONDS;
    hand.pearl_cooldown = hand.pearl_cooldown.saturating_sub(hand_ticks);
    let pearl_left = (hand.pearl_cooldown as f32 - (hand.clock / TICK_SECONDS) as f32) / minecraft_terrain::thrown::PEARL_COOLDOWN as f32;
    ui.cooldowns = if hand.pearl_cooldown > 0 { vec![("minecraft:ender_pearl".to_owned(), pearl_left)] } else { Vec::new() };
    hand.miss_time = hand.miss_time.saturating_sub(hand_ticks);
    hand.attack_ticks = hand.attack_ticks.saturating_add(hand_ticks);
    let main_hand = entities.as_ref().and_then(|_| inventory.slots[*selected].as_ref().map(|s| s.id.clone()));
    if hand.attack_item != main_hand {
        hand.attack_item = main_hand;
        hand.attack_ticks = 0;
    }
    if alive {
        let motion = minecraft_survival::survival::Motion {
            feet,
            on_ground,
            sprinting: ps.pm_flags & playerstate_iw4::pm_flags::SPRINTING != 0,
            jumping: ps.pm_flags & playerstate_iw4::pm_flags::JUMPING != 0,
            in_water: water.wet(),
            eye_in_water: water.eye,
            in_lava: sim::voxel::lava(ps.origin, hull_mins, hull_maxs) > 0.0,
            eye_in_lava: sim::voxel::eye_in_lava(ps.origin, ps.view_height_current),
            in_rain: entities.as_ref().is_some_and(|e| e.weather.player_in_rain),
            crouching: ps.pm_flags & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE) != 0,
            height: hull_maxs[2] / sim::voxel::BLOCK,
        };
        let armor = entities.as_ref().map_or_else(minecraftoss_player::survival::Armor::default, |_| minecraft_survival::survival::armor(inventory));
        for _ in 0..hand_ticks {
            let from = survival.last_feet().unwrap_or(feet);
            let breath = survival.tick(&motion);
            if let Some(sounds) = sounds.as_mut() {
                if breath.drowned {
                    sounds.play(&world.packs, "minecraft:entity.player.hurt_drown", None, 1.0, 1.0);
                }
                if let Some(popped) = breath.popped {
                    let (volume, pitch) = minecraft_survival::survival::pop_sound(popped);
                    sounds.play(&world.packs, "minecraft:ui.hud.bubble_pop", None, volume, pitch);
                }
            }
            if motion.in_lava {
                steps.air_peak = steps.air_peak.map(|peak| feet[1] + (peak - feet[1]) * 0.5);
            }
            let heard = survival.hazards(&terrain.scene, &motion, from, armor, || entities.as_mut().map_or(0.5, |e| e.random_float()));
            if entities.is_some() {
                hold_off_death(survival, inventory, *selected, sounds.as_mut(), &world.packs, false);
            }
            if survival.stuck() {
                steps.air_peak = steps.air_peak.map(|_| feet[1]);
            }
            if let Some(sounds) = sounds.as_mut() {
                for sound in heard {
                    sounds.play(&world.packs, sound.event, None, sound.volume, sound.pitch);
                }
            }
        }
    }
    if let Some(entities) = entities.as_mut() {
        let mut props = Vec::new();
        let mut brewed = Vec::new();
        for _ in 0..hand_ticks {
            let ticked = containers.tick(&entities.recipes);
            props.extend(ticked.props);
            brewed.extend(ticked.brewed);
            for (pos, stack) in ticked.drops {
                entities.world_items.spawn_block_drop(stack, pos);
            }
            let lit = |pos| minecraft_terrain::scene::Scene::block(&terrain.scene, pos).map(minecraft_terrain::campfire::is_lit);
            for (pos, stack) in containers.cook(lit, &entities.recipes) {
                entities.world_items.drop_item_stack(stack, pos);
            }
        }
        let mut writes: Vec<((i32, i32, i32), Option<minecraft_terrain::scene::Block>)> = Vec::new();
        for (pos, name, on) in props {
            let current = writes
                .iter()
                .rev()
                .find(|(at, _)| *at == pos)
                .and_then(|(_, block)| block.clone())
                .or_else(|| minecraft_terrain::scene::Scene::block(&terrain.scene, pos).cloned());
            if let Some(block) = current.filter(|block| block.properties.contains_key(name)) {
                writes.push((pos, Some(block.with(name, if on { "true" } else { "false" }))));
            }
        }
        if let Some(sounds) = sounds.as_mut() {
            for pos in brewed {
                let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                sounds.play(&world.packs, "minecraft:block.brewing_stand.brew", Some(Vec3::from_array(sim::voxel::to_map(origin, centre))), 1.0, 1.0);
            }
        }
        if !writes.is_empty() {
            write_blocks(terrain, &globals.registries, shapes, shape_ids, entities, writes);
        }
    }
    let game_mode = *player_mode;
    let menu_captured = ui.chat.open || hud_input.as_deref().is_some_and(|input| input.console_open || input.script_menu_open);
    let use_held = buttons.pressed(MouseButton::Right) || pad_trigger(pad, GamepadButton::LeftTrigger2, false);
    let offhand_key = alive && !holding && ui.offhand_key == Some(true);
    let use_live = !ui.inventory_open && !menu_captured && if hand.use_by_key { offhand_key } else { use_held && ui.holding_item };
    if survival.eating().is_some() && !use_live {
        survival.stop_eating();
    }
    let shield_held = entities.as_ref().is_some_and(|_| hand.blocking.is_some_and(|blocking| blocking.holds_shield(inventory, *selected)));
    if !use_live || !shield_held {
        hand.blocking = None;
    }
    let mut open_table = None;
    let mut open_block = None;
    let mut open_bench = None;
    if let Some(entities) = entities.as_mut()
        && (ui.holding_item || offhand_key || survival.eating().is_some() || hand.blocking.is_some())
        && !ui.inventory_open
        && !menu_captured
        && game_mode.interacts()
    {
        let mut player = minecraftoss_player::Player::new(glam::DVec3::from_array(feet));
        player.yaw = f64::from(mc_yaw);
        player.pitch = f64::from(ps.viewangles[0]);
        player.selected = *selected;
        player.crouching = ps.pm_flags & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE) != 0;
        player.swimming = ps.view_height_current < CRAWLING_BELOW_VIEW_HEIGHT;
        let eye_block = player.eye();
        let look = {
            let (yaw, pitch) = (f64::from(mc_yaw).to_radians(), f64::from(ps.viewangles[0]).to_radians());
            glam::DVec3::new(-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos())
        };
        let attack_held = buttons.pressed(MouseButton::Left) || pad_trigger(pad, GamepadButton::RightTrigger2, false);
        if !attack_held {
            hand.miss_time = 0;
        }
        let busy = survival.eating().is_some() || hand.blocking.is_some() || hand.miss_time > 0;
        let mut creative_press = false;
        let mut punched = false;
        if ui.holding_item && (buttons.just_pressed(MouseButton::Left) || pad_trigger(pad, GamepadButton::RightTrigger2, true)) && !busy {
            hand.swing = Some(0.0);
            hand.swing_offhand = false;
            let held = inventory.slots[*selected].clone();
            let (damage, speed) = entities.recipes.attack_attributes(held.as_ref());
            let damage = (damage + survival.status.attack_damage_modifier()).max(0.0);
            let strength = minecraft_survival::survival::attack_strength(hand.attack_ticks, speed);
            let sprinting = ps.pm_flags & playerstate_iw4::pm_flags::SPRINTING != 0;
            let falling = steps.air_peak.is_some_and(|peak| peak > feet[1]);
            let blow = minecraft_survival::entities::Blow {
                sprinting,
                can_critical: falling && !on_ground && !water.wet() && !sprinting,
                can_sweep: on_ground && !sprinting && held.as_ref().is_some_and(|s| s.id.ends_with("_sword")),
                reach: game_mode.entity_reach(),
            };
            if entities.punch(inventory, *selected, eye_block, look, mc_yaw, damage, strength, blow) {
                punched = true;
                hand.attack_ticks = 0;
                survival.exhaust(minecraft_survival::survival::ATTACK_EXHAUSTION);
            } else if player.target(&minecraft_terrain::doors::Outlined(&terrain.scene), game_mode.block_reach()).is_none() {
                hand.attack_ticks = 0;
                hand.miss_time = if game_mode.instant_break() { 0 } else { 10 };
            } else {
                creative_press = true;
            }
        }
        for _ in 0..hand_ticks {
            let Some((drinking, crumbs, item)) = survival.eating().map(|eating| (eating.info.animation.eq_ignore_ascii_case("drink"), eating.info.particles, eating.stack.id.clone())) else {
                break;
            };
            let roll = entities.random_float();
            let bite = survival.chew(inventory, *selected, || roll);
            let (sound, finished) = match bite {
                minecraft_survival::survival::Bite::Nothing => continue,
                minecraft_survival::survival::Bite::Chew(sound) => (sound, None),
                minecraft_survival::survival::Bite::Done { sound, overflow, fed } => (sound, Some((overflow, fed))),
            };
            if crumbs
                && let Some(texture) = minecraft_terrain::pack::ResourceId::parse(&item)
                    .ok()
                    .and_then(|id| minecraft_terrain::model::item_particle_texture(&world.packs, &id).ok().flatten())
                    .filter(|texture| world.atlas.contains(texture))
            {
                let count = if finished.is_some() { 16 } else { 5 };
                mining.eat_burst(&world.packs, eye_block.as_vec3(), mc_yaw, ps.viewangles[0], count, &texture);
            }
            let (r1, r2) = (entities.random_float(), entities.random_float());
            if let Some(sounds) = sounds.as_mut() {
                let (volume, pitch) = if drinking { (0.5, r1 * 0.1 + 0.9) } else { (if r1 < 0.5 { 0.5 } else { 1.0 }, (r1 - r2) * 0.2 + 1.0) };
                sounds.play(&world.packs, &sound, None, volume, pitch);
            }
            if let Some((overflow, fed)) = finished {
                if let Some(sounds) = sounds.as_mut().filter(|_| fed) {
                    sounds.play(&world.packs, "minecraft:entity.player.burp", None, 0.5, r2 * 0.1 + 0.9);
                }
                if let Some(stack) = overflow {
                    entities.world_items.toss(stack, eye_block, mc_yaw, ps.viewangles[0]);
                }
            }
        }
        if ui.holding_item && attack_held && !busy && game_mode.may_build() && game_mode.instant_break() {
            let held = inventory.slots[*selected].as_ref().map(|stack| stack.id.as_str());
            if let Some(hit) = player.target(&minecraft_terrain::doors::Outlined(&terrain.scene), game_mode.block_reach())
                && !punched
                && entities.mob_distance(eye_block, look, game_mode.entity_reach()).is_none_or(|distance| distance > hit.distance)
                && minecraft_terrain::game_mode::can_destroy_in_creative(held)
                && minecraft_terrain::game_mode::creative_breaks(&mut hand.creative_delay, creative_press, attack_held, hand_ticks) > 0
            {
                creative_broken.push(hit.pos);
                hand_mined.push(hit.pos);
                if hand.swing.is_none_or(|t| t >= crate::minecraft_hand::SWING_TICKS * 0.5) {
                    hand.swing = Some(0.0);
                    hand.swing_offhand = false;
                }
            }
        } else if ui.holding_item && attack_held && !busy && game_mode.may_build() {
            for _ in 0..hand_ticks {
                if let Some(hit) = player.target(&minecraft_terrain::doors::Outlined(&terrain.scene), 4.5) {
                    // A hand mines as vanilla's `getDestroyProgress`: a
                    // block's hardness times thirty ticks.
                    all_events.push(sim::voxel::VoxelEvent::Shot {
                        block: [hit.pos.0, hit.pos.1, hit.pos.2],
                        damage: 160.0 / 30.0,
                    });
                    hand_mined.push(hit.pos);
                    if hand.swing.is_none_or(|t| t >= crate::minecraft_hand::SWING_TICKS * 0.5) {
                        hand.swing = Some(0.0);
                        hand.swing_offhand = false;
                    }
                }
            }
        }
        if let Some(blocking) = hand.blocking.as_mut() {
            blocking.ticks = blocking.ticks.saturating_add(hand_ticks);
        }
        hand.place_delay = hand.place_delay.saturating_sub(hand_ticks);
        let idle = survival.eating().is_none() && hand.blocking.is_none();
        let (use_pressed, use_down) = if ui.holding_item {
            (buttons.just_pressed(MouseButton::Right) || pad_trigger(pad, GamepadButton::LeftTrigger2, true), use_held)
        } else {
            (offhand_key && !hand.offhand_key_was, offhand_key)
        };
        let use_pressed = idle && use_pressed;
        let place = use_pressed || (idle && use_down && hand.place_delay == 0);
        let hands = if place {
            hand.place_delay = 4;
            hand.use_by_key = !ui.holding_item;
            minecraft_survival::offhand::hand_slots(*selected, !ui.holding_item)
        } else {
            Vec::new()
        };
        let has_item = inventory.slots[*selected].is_some() || inventory.slots[minecraft_survival::offhand::OFFHAND].is_some();
        let used = minecraft_survival::offhand::use_hands(&hands, |slot| {
            use minecraft_survival::offhand::HandUse;
            let main = slot != minecraft_survival::offhand::OFFHAND;
            player.selected = slot;
            let held = inventory.slots[slot].clone();
            let table = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|&pos| minecraft_terrain::scene::Scene::block(&terrain.scene, pos).is_some_and(minecraft_terrain::crafting::is_crafting_table))
                .filter(|_| main && minecraft_terrain::crafting::opens_table(player.crouching, has_item));
            let station = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|_| main && table.is_none() && minecraft_terrain::crafting::opens_table(player.crouching, has_item))
                .and_then(|pos| {
                    let block = minecraft_terrain::scene::Scene::block(&terrain.scene, pos)?;
                    let partner = minecraft_terrain::containers::chest_partner(pos, block).and_then(|at| minecraft_terrain::scene::Scene::block(&terrain.scene, at));
                    minecraft_terrain::containers::Opened::new(pos, block, partner)
                });
            let workshop = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|_| main && table.is_none() && station.is_none() && minecraft_terrain::crafting::opens_table(player.crouching, has_item))
                .and_then(|pos| minecraft_terrain::scene::Scene::block(&terrain.scene, pos).and_then(minecraft_terrain::stations::Workshop::of).map(|kind| (pos, kind)));
            let campfire = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|&pos| minecraft_terrain::scene::Scene::block(&terrain.scene, pos).is_some_and(minecraft_terrain::campfire::is_campfire))
                .filter(|_| !(player.crouching && has_item));
            let placeable = held.as_ref().is_some_and(|stack| {
                let id = if stack.id == "minecraft:redstone" { "minecraft:redstone_wire" } else { stack.id.as_str() };
                minecraft_terrain::pack::ResourceId::parse(id).is_ok_and(|id| {
                    terrain.stream.states.state_of(&minecraft_terrain::scene::Block { id, properties: Default::default() }).is_some()
                })
            });
            let used = if main && table.is_none() && station.is_none() && workshop.is_none() && !(player.crouching && has_item) {
                let facing = minecraft_terrain::doors::facing(f64::from(mc_yaw));
                player
                    .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                    .map(|hit| hit.pos)
                    .filter(|&pos| !minecraftoss_player::World::block(&terrain.scene, pos).is_some_and(|block| minecraft_terrain::berries::bone_meal_passes(&block, held.as_ref())))
                    .filter(|&pos| entities.use_block(&terrain.scene, pos, facing))
            } else {
                None
            };
            if let Some(pos) = table {
                if use_pressed {
                    open_table = Some(pos);
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(bench) = workshop {
                if use_pressed {
                    open_bench = Some(bench);
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(opened) = station {
                if use_pressed {
                    let conductor = |(x, y, z): (i32, i32, i32)| {
                        minecraft_terrain::scene::Scene::block(&terrain.scene, (x, y + 1, z))
                            .and_then(|b| terrain.stream.states.state_of(b))
                            .is_some_and(|state| globals.registries.blocks.is(state, minecraftoss_core::block::flags::REDSTONE_CONDUCTOR))
                    };
                    let blocked = match opened.station {
                        minecraft_terrain::containers::Station::Chest | minecraft_terrain::containers::Station::Ender => opened.cells.iter().any(|&pos| conductor(pos)),
                        minecraft_terrain::containers::Station::Shulker => {
                            let facing = minecraft_terrain::scene::Scene::block(&terrain.scene, opened.cells[0]).and_then(|b| b.properties.get("facing").cloned()).unwrap_or_else(|| "up".to_owned());
                            minecraft_terrain::containers::lid_obstructed(opened.cells[0], &facing, |at| minecraftoss_player::World::collision_boxes(&terrain.scene, at))
                        }
                        _ => false,
                    };
                    if !blocked {
                        open_block = Some(opened);
                    }
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(pos) = campfire
                && let Some(stack) = held.as_ref()
                && let Some(placed) = containers.place_food(pos, stack, &entities.recipes)
            {
                if placed {
                    let slot = &mut inventory.slots[slot];
                    if let Some(left) = slot.as_mut().filter(|_| game_mode.consumes_items()) {
                        left.count -= 1;
                        if left.count == 0 {
                            *slot = None;
                        }
                    }
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(pos) = used {
                hand.swing = Some(0.0);
                if let Some(block) = minecraftoss_player::World::block(&terrain.scene, pos)
                    && minecraft_terrain::doors::note_plays(&block, minecraftoss_player::World::block(&terrain.scene, (pos.0, pos.1 + 1, pos.2)).as_ref())
                    && let Some(sound) = minecraft_terrain::doors::toggle_sound(&block)
                    && let Some(sounds) = sounds.as_mut()
                {
                    let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                    let pitch = sound.pitch + sounds.random() * sound.pitch_spread;
                    sounds.play(&world.packs, &sound.event, Some(Vec3::from_array(sim::voxel::to_map(origin, centre))), sound.volume, pitch);
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && let Some(outcome) = minecraft_terrain::bottle::bottle_use(&player, &terrain.scene, stack)
            {
                use minecraft_terrain::bottle::{self, BottleUse};
                hand.swing = Some(0.0);
                let (result, heard) = match outcome {
                    BottleUse::Block { pos, block, result, sounds: events } => {
                        write_blocks(terrain, &globals.registries, shapes, shape_ids, entities, vec![(pos, block.and_then(|b| scene_block(&b)))]);
                        let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                        (Some(result), events.into_iter().map(|event| (event, Some(centre))).collect())
                    }
                    BottleUse::Fill { result } => (Some(result), vec![(bottle::FILL_SOUND, None)]),
                    BottleUse::Drink => {
                        hand.swing = None;
                        survival.start_using(bottle::drink(slot, stack));
                        (None, Vec::new())
                    }
                };
                if let Some(result) = result
                    && let Some(rest) = bottle::filled_result(inventory, slot, result)
                {
                    entities.world_items.toss(rest, eye_block, mc_yaw, ps.viewangles[0]);
                }
                if let Some(sounds) = sounds.as_mut() {
                    for (event, at) in heard {
                        sounds.play(&world.packs, event, at.map(|centre| Vec3::from_array(sim::voxel::to_map(origin, centre))), 1.0, 1.0);
                    }
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && !entities.awaits_item_use()
                && game_mode.may_build()
                && let Some(outcome) = {
                    let catalog = minecraft_terrain::bucket::Catalog { states: &terrain.stream.states, water_evaporates: terrain.environment.water_evaporates() };
                    let outlined = minecraft_terrain::doors::Outlined(&terrain.scene);
                    minecraft_terrain::bucket::bucket_use(&player, &outlined, &catalog, stack, || entities.random_float())
                        .or_else(|| minecraft_terrain::flint::flint_use(&player, &outlined, &catalog, stack, || entities.random_float()))
                        .or_else(|| minecraft_terrain::flint::fire_charge_use(&player, &outlined, &catalog, stack, || entities.random_float()))
                        .or_else(|| minecraft_terrain::berries::bone_meal_use(&player, &outlined, stack))
                }
            {
                if !outcome.consumed || matches!(outcome.change, minecraft_terrain::bucket::Change::Set(..)) {
                    hand.swing = Some(0.0);
                }
                let change = outcome.change.clone();
                let sent = minecraft_survival::entities::SentItemUse { slot, held: stack.id.clone(), eye: eye_block, yaw: mc_yaw, pitch: ps.viewangles[0], outcome };
                match change {
                    minecraft_terrain::bucket::Change::Set(pos, block) => {
                        write_blocks(terrain, &globals.registries, shapes, shape_ids, entities, vec![(pos, scene_block(&block))]);
                        finish_item_use(entities, inventory, sounds.as_mut(), &world.packs, origin, sent, game_mode.consumes_items());
                    }
                    minecraft_terrain::bucket::Change::Server(pos, action) => entities.item_on_block(pos, action, sent),
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && game_mode.may_build()
                && let Some(egg) = minecraft_terrain::spawn_egg::egg_use(&player, &minecraft_terrain::doors::Outlined(&terrain.scene), &stack.id)
            {
                if let minecraft_terrain::spawn_egg::EggUse::Spawn { kind, position } = egg {
                    entities.hatch_egg(inventory, slot, kind, position, game_mode.consumes_items());
                }
                hand.swing = Some(0.0);
                HandUse::Consumed
            } else if let Some((stack, kind)) = held.as_ref().and_then(|s| minecraft_terrain::thrown::Throwable::of(&s.id).map(|k| (s, k))) {
                let pearl = kind == minecraft_terrain::thrown::Throwable::EnderPearl;
                let eye = sim::voxel::to_block(origin, [ps.origin[0], ps.origin[1], ps.origin[2] + ps.view_height_current]);
                let motion = glam::DVec3::new(f64::from(ps.velocity[0]), f64::from(ps.velocity[2]), -f64::from(ps.velocity[1]))
                    / (f64::from(sim::voxel::BLOCK) * 20.0);
                if !(pearl && hand.pearl_cooldown > 0)
                    && entities.throw(&stack.id, glam::DVec3::from_array(eye), mc_yaw, ps.viewangles[0], motion, on_ground)
                {
                    let slot = &mut inventory.slots[slot];
                    if let Some(left) = slot.as_mut().filter(|_| game_mode.consumes_items()) {
                        left.count -= 1;
                        if left.count == 0 {
                            *slot = None;
                        }
                    }
                    if pearl {
                        hand.pearl_cooldown = minecraft_terrain::thrown::PEARL_COOLDOWN;
                    }
                    hand.swing = Some(0.0);
                    let pitch = minecraft_terrain::thrown::throw_pitch(entities.random_float());
                    if let Some(sounds) = sounds.as_mut() {
                        sounds.play(&world.packs, kind.sound(), None, 0.5, pitch);
                    }
                    HandUse::Consumed
                } else {
                    HandUse::Passed
                }
            } else if let Some((pos, writes)) = held.as_ref().filter(|_| game_mode.may_build()).and_then(|stack| {
                let outlined = minecraft_terrain::doors::Outlined(&terrain.scene);
                let mut staged = minecraft_terrain::placing::Staged::new(&outlined);
                let soil = minecraft_terrain::berries::Ground { states: &terrain.stream.states };
                let pos = minecraft_terrain::berries::plant(&player, &mut staged, &stack.id, &soil, || entities.random_float())?;
                Some((pos, staged.into_writes()))
            }) {
                let slot = &mut inventory.slots[slot];
                if let Some(left) = slot.as_mut().filter(|_| game_mode.consumes_items()) {
                    left.count -= 1;
                    if left.count == 0 {
                        *slot = None;
                    }
                }
                let writes = writes.into_iter().map(|(at, block)| (at, block.and_then(|b| scene_block(&b)))).collect();
                write_blocks(terrain, &globals.registries, shapes, shape_ids, entities, writes);
                hand.swing = Some(0.0);
                if let Some(block) = minecraft_terrain::scene::Scene::block(&terrain.scene, pos).cloned()
                    && let (Some(sounds), Some(kind)) = (sounds.as_mut(), terrain.scene.sound_type(&block))
                {
                    let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                    sounds.play(&world.packs, &kind.place, Some(Vec3::from_array(sim::voxel::to_map(origin, centre))), (kind.volume + 1.0) / 2.0, kind.pitch * 0.8);
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref().filter(|s| !placeable && minecraftoss_player::food::catalog().contains_key(&s.id)) {
                if survival.start_eating(slot, stack) { HandUse::Consumed } else { HandUse::Passed }
            } else if let Some(stack) = held.as_ref().filter(|s| s.id == minecraft_terrain::bottle::MILK_BUCKET) {
                survival.start_using(minecraft_terrain::bottle::drink_milk(slot, stack));
                HandUse::Consumed
            } else if held.as_ref().is_some_and(|stack| stack.id == minecraft_survival::offhand::SHIELD) {
                hand.blocking = Some(minecraft_survival::offhand::Blocking::start(slot));
                HandUse::Consumed
            } else if placeable {
                let outlined = minecraft_terrain::doors::Outlined(&terrain.scene);
                let mut staged = minecraft_terrain::placing::Staged::new(&outlined);
                let opening = held.as_ref().filter(|stack| minecraft_terrain::doors::Opening::of(&stack.id).is_some());
                let sturdy = |below: (i32, i32, i32)| {
                    minecraft_terrain::scene::Scene::block(&terrain.scene, below)
                        .and_then(|b| terrain.stream.states.state_of(b))
                        .is_some_and(|state| {
                            globals.registries.blocks.is_face_sturdy(state, minecraftoss_core::pos::Direction::Up, minecraftoss_core::SupportType::Full)
                        })
                };
                let placed = match opening {
                    _ if !game_mode.may_build() && (opening.is_some() || held.as_ref().is_some_and(|s| s.id == minecraft_terrain::scaffolding::SCAFFOLDING)) => None,
                    Some(stack) => minecraft_terrain::doors::place(&player, &mut staged, &stack.id, &sturdy).inspect(|_| {
                        let slot = &mut inventory.slots[slot];
                        if let Some(left) = slot.as_mut() {
                            left.count -= 1;
                            if left.count == 0 {
                                *slot = None;
                            }
                        }
                    }),
                    None if held.as_ref().is_some_and(|s| s.id == minecraft_terrain::scaffolding::SCAFFOLDING) => minecraft_terrain::scaffolding::place(
                        &player,
                        &mut staged,
                        inventory,
                        ps.pm_flags & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE) != 0,
                        &sturdy,
                        &|block: Option<&minecraftoss_player::Block>| {
                            block.is_none_or(|b| {
                                scene_block(b)
                                    .and_then(|b| terrain.stream.states.state_of(&b))
                                    .is_some_and(|state| globals.registries.blocks.is(state, minecraftoss_core::block::flags::REPLACEABLE))
                            })
                        },
                    ),
                    None => player.place_selected(&mut staged, inventory, game_mode.mode),
                };
                let writes: Vec<_> = staged
                    .into_writes()
                    .into_iter()
                    .map(|(pos, block)| (pos, block.and_then(|b| scene_block(&b))))
                    .collect();
                if std::env::var_os("MINECRAFTOSS_DEBUG_SERVER").is_some() && writes.iter().any(|(_, b)| b.as_ref().is_some_and(|b| b.id.path.ends_with("chest"))) {
                    eprintln!("chest placement: crouching {} pm_flags {:#x} view height {} yaw {} placed {:?} writes {:?}", player.crouching, ps.pm_flags, ps.view_height_current, player.yaw, placed, writes);
                }
                let bodies = obstacles(ps, &authority.0, local.0, entities);
                let blocked = writes.iter().any(|(pos, block)| {
                    let boxes = block
                        .as_ref()
                        .filter(|b| b.id.key() != minecraft_terrain::scaffolding::SCAFFOLDING)
                        .and_then(|b| terrain.stream.states.state_of(b))
                        .map_or_else(Vec::new, |state| globals.registries.blocks.collision_boxes(state));
                    minecraft_terrain::placing::obstructed(*pos, &boxes, &bodies)
                });
                let survives = placed.is_none_or(|pos| {
                    writes.iter().find(|(at, _)| *at == pos).and_then(|(_, block)| block.as_ref()).is_none_or(|block| {
                        let view = minecraft_terrain::placing::SurviveView::new(&terrain.scene, light.as_ref());
                        minecraft_terrain::placing::may_place(&terrain.stream.world_gen().library, &view, pos, block)
                    })
                });
                if let Some(pos) = placed.filter(|_| !blocked && survives) {
                    let now = time.elapsed_secs_f64();
                    hand.recent.retain(|(_, _, at)| now - at < RECENT_PLACE_SECONDS);
                    let written: Vec<_> = writes.iter().map(|(p, _)| *p).collect();
                    write_blocks(terrain, &globals.registries, shapes, shape_ids, entities, writes);
                    if let Some(stack) = held.as_ref().filter(|stack| minecraft_terrain::containers::is_shulker_box(&stack.id))
                        && minecraft_terrain::scene::Scene::block(&terrain.scene, pos).and_then(minecraft_terrain::containers::Station::of) == Some(minecraft_terrain::containers::Station::Shulker)
                    {
                        containers.place_shulker(pos, stack, &entities.recipes);
                    }
                    if !game_mode.consumes_items() {
                        inventory.slots[slot].clone_from(&held);
                    }
                    hand.recent.extend(written.into_iter().filter_map(|p| minecraftoss_player::World::block(&terrain.scene, p).map(|b| (p, b, now))));
                    hand.swing = Some(0.0);
                    if let Some(block) = minecraft_terrain::scene::Scene::block(&terrain.scene, pos).cloned()
                        && let (Some(sounds), Some(kind)) = (sounds.as_mut(), terrain.scene.sound_type(&block))
                    {
                        let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                        let at = Vec3::from_array(sim::voxel::to_map(origin, centre));
                        sounds.play(&world.packs, &kind.place, Some(at), (kind.volume + 1.0) / 2.0, kind.pitch * 0.8);
                    }
                    HandUse::Consumed
                } else {
                    inventory.slots[slot] = held;
                    hand.place_delay = 0;
                    if player.target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0).is_some() { HandUse::Failed } else { HandUse::Passed }
                }
            } else {
                HandUse::Passed
            }
        });
        hand.place_delay = minecraft_survival::offhand::repeat_delay(used, hand.place_delay);
        if let Some((slot, minecraft_survival::offhand::HandUse::Consumed)) = used
            && hand.swing == Some(0.0)
        {
            hand.swing_offhand = slot == minecraft_survival::offhand::OFFHAND;
        }
    }
    hand.offhand_key_was = offhand_key;

    // Shots and explosions from the authoritative game: bullets that met a
    // mob hurt it, the rest mine.
    let (mob_shots, events): (Vec<_>, Vec<_>) = all_events
        .into_iter()
        .partition(|event| matches!(event, sim::voxel::VoxelEvent::MobShot { .. } | sim::voxel::VoxelEvent::MobBlast { .. }));
    let events: Vec<_> = events
        .into_iter()
        .filter(|event| player_mode.may_build() || !matches!(event, sim::voxel::VoxelEvent::Shot { .. } | sim::voxel::VoxelEvent::Ray { .. }))
        .collect();
    if let Some(entities) = entities.as_mut() {
        for shot in mob_shots {
            match shot {
                sim::voxel::VoxelEvent::MobShot { key, damage, from } => entities.shoot(key, damage, from, mc_yaw),
                sim::voxel::VoxelEvent::MobBlast { key, damage, center, impact, attacker, slow_ticks } => {
                    entities.blast(key, damage, center, impact, attacker == Some(local.0.0), slow_ticks);
                }
                _ => {}
            }
        }
        for event in &events {
            if let sim::voxel::VoxelEvent::Explosion { center, power } = *event {
                entities.weapon_blast(center, power);
            }
        }
    }
    // Vanilla's block sounds: a hit for each bullet into a block, then the
    // break of each block broken (`SoundType` volume and pitch as
    // `MultiPlayerGameMode` and `LevelRenderer` scale them), and blasts.
    let at = |b: [f64; 3]| Vec3::from_array(sim::voxel::to_map(origin, b));
    if let Some(sounds) = sounds.as_mut() {
        for event in &events {
            match *event {
                sim::voxel::VoxelEvent::Shot { block, .. } => {
                    let pos = (block[0], block[1], block[2]);
                    if let Some(kind) = minecraft_terrain::scene::Scene::block(&terrain.scene, pos)
                        .and_then(|b| terrain.scene.sound_type(b))
                    {
                        let centre = [block[0] as f64 + 0.5, block[1] as f64 + 0.5, block[2] as f64 + 0.5];
                        sounds.play(&world.packs, &kind.hit, Some(at(centre)), (kind.volume + 1.0) / 4.0, kind.pitch * 0.5);
                    }
                }
                sim::voxel::VoxelEvent::Explosion { center, .. } => {
                    let pitch = (1.0 + (sounds.random() - sounds.random()) * 0.2) * 0.7;
                    sounds.play(&world.packs, "minecraft:entity.generic.explode", Some(at(center)), 4.0, pitch);
                }
                sim::voxel::VoxelEvent::MobShot { .. } | sim::voxel::VoxelEvent::MobBlast { .. } | sim::voxel::VoxelEvent::Ray { .. } => {}
            }
        }
    }
    let mut refs = crate::minecraft_mining::WorldRefs {
        stream: &mut terrain.stream,
        scene: &mut terrain.scene,
        packs: &world.packs,
        atlas: &world.atlas,
        registries: &globals.registries,
    };
    let mut broken = mining.apply(events, &mut refs, time.elapsed_secs_f64());
    broken.extend(mining.break_now(&creative_broken, &mut refs));
    if let Some(sounds) = sounds.as_mut() {
        for (pos, block, blast) in &broken {
            if blast.is_some() {
                continue;
            }
            if let Some(kind) = terrain.scene.sound_type(block) {
                let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                sounds.play(&world.packs, &kind.break_sound, Some(at(centre)), (kind.volume + 1.0) / 2.0, kind.pitch * 0.8);
            }
        }
    }
    survival.exhaust(minecraft_survival::survival::MINE_EXHAUSTION * broken.iter().filter(|(.., blast)| blast.is_none()).count() as f32);
    if let Some(entities) = entities.as_mut() {
        let positions: Vec<_> = broken.iter().map(|(pos, ..)| *pos).collect();
        entities.mined(inventory.slots[*selected].as_ref(), &broken, &hand_mined, !player_mode.drops_from_breaking());
        let kept: HashMap<_, _> = broken
            .iter()
            .filter(|(_, block, _)| minecraft_terrain::containers::Station::of(block) == Some(minecraft_terrain::containers::Station::Shulker))
            .filter_map(|(pos, ..)| containers.take_shulker(*pos).map(|components| (*pos, components)))
            .collect();
        let dropping: Vec<_> = broken
            .iter()
            .filter(|(pos, _, blast)| player_mode.drops_from_breaking() || blast.is_some() || kept.get(pos).is_some_and(|components| components.get(minecraft_terrain::containers::CONTAINER).is_some()))
            .cloned()
            .collect();
        entities.award_block_experience(inventory.slots[*selected].as_ref(), &dropping, &hand_mined);
        entities.drop_blocks(inventory.slots[*selected].as_ref(), &dropping, kept, &hand_mined);
        for &pos in &positions {
            spill(containers, entities, pos);
        }
    }

    let eye = sim::voxel::to_block(origin, [ps.origin[0], ps.origin[1], ps.origin[2] + ps.view_height_current]);
    death_watch.stood(minecraft_survival::death::DeathSpot { feet, eye });
    let (pitch, yaw) = (ps.viewangles[0].to_radians(), ps.viewangles[1].to_radians());
    let map_forward = [pitch.cos() * yaw.cos(), pitch.cos() * yaw.sin(), -pitch.sin()];
    let forward = glam::Vec3::new(map_forward[0], map_forward[2], -map_forward[1]);
    // Culled from the camera that draws: the player's eye, or while
    // skating the Skate camera (a frame behind, so with room to spare).
    let mc_detached = mc_camera.as_ref().is_some_and(|c| c.detached);
    let skate_camera = cameras.iter().next().filter(|_| skate.active || mc_detached).map(|t| {
        let at = sim::voxel::to_block(origin, t.translation.to_array());
        let ahead = t.rotation * Vec3::NEG_Z;
        (
            glam::DVec3::new(at[0], at[1], at[2]),
            glam::Vec3::new(ahead.x, ahead.z, -ahead.y).normalize_or(forward),
        )
    });
    let (cull_at, cull_forward) = skate_camera.unwrap_or((glam::DVec3::new(eye[0], eye[1], eye[2]), forward));
    let camera = CullCamera {
        position: cull_at,
        forward: cull_forward,
        fov_degrees: if skate_camera.is_some() { 120.0 } else { 90.0 },
        aspect,
        yaw_degrees: (-cull_forward.x).atan2(cull_forward.z).to_degrees(),
        pitch_degrees: (-cull_forward.y).asin().to_degrees(),
    };
    let update = terrain
        .stream
        .frame(&terrain.scene, &camera, FADE_MILLIS, &world.atlas, &world.packs);
    view.uploads.extend(update.uploads);
    view.removed.extend(update.removed);
    view.visible = update.visible;
    observe_spawn(terrain, spawn_gate, origin, cull_at, now_ms);
    let Some(light) = light.as_mut() else {
        return;
    };
    for (chunk, column) in update.lights {
        light.set_chunk_column(chunk, column);
    }

    if let Some(sounds) = sounds.as_mut() {
        sound_queue.0.append(&mut sounds.queued);
    }
    // The minimap's picture, and its corners on the map.
    let minimap_shown = settings.as_deref().is_none_or(|s| s.minimap_shown(true));
    ui.minimap = minimap_shown
        .then(|| minimap.update(time.delta_secs_f64(), feet, &terrain.scene, &world.packs, &world.atlas, &mut images))
        .flatten()
        .map(|(image, [bx, bz])| {
            let corner = |x: i32, z: i32| {
                let p = sim::voxel::to_map(origin, [f64::from(x), feet[1], f64::from(z)]);
                [p[0], p[1]]
            };
            (image, corner(bx, bz), corner(bx + 256, bz + 256))
        });
    // The Overworld clock and the environment attributes of MinecraftOSS.
    let dt = time.delta_secs_f64();
    let partial = mining.tick(&terrain.scene, dt);
    view.particles = mining.particle_mesh(&world.atlas, forward, partial, light);

    // The mobs: a server tick when due, the blocks it changed, its hits on
    // the player, the mobs' boxes for bullets and their meshes.
    if let Some(entities) = entities.as_mut() {
        let player = minecraft_survival::entities::PlayerView {
            feet,
            alive,
            attackable: alive && player_mode.attackable_by_mobs(),
            spectator: !alive || player_mode.mode == minecraftoss_player::GameMode::Spectator,
            picks_up: alive && player_mode.picks_up(),
            health: survival.status.health,
            yaw: mc_yaw,
            pitch: ps.viewangles[0],
        };
        let bright_outside = terrain.environment.sky_light_level() > 11.0;
        entities.difficulty = minecraft_commands::difficulty_id(globals.difficulty);
        let ticks_before = entities.client_ticks();
        let (changes, hits) = entities.tick(inventory, *selected, dt, globals.day.ticks as i64, bright_outside, &player);
        let summoned = std::mem::take(&mut entities.command_summons);
        if !summoned.is_empty() {
            let answers = minecraft_survival::commands::summon_answers(&globals.chat, &globals.game_rules, summoned);
            show_chat(&mut ui.chat, inventory_ui, &world.packs, &answers);
        }
        let mob_ticks = (entities.client_ticks() - ticks_before) as u32;
        globals.autosave_ticks += mob_ticks;
        if globals.autosave_ticks >= minecraft_survival::save::AUTOSAVE_TICKS {
            globals.autosave_ticks = 0;
            autosave(globals, terrain, *player_mode);
        }
        for _ in 0..mob_ticks {
            for sound in entities.animate_tick(&terrain.scene, feet) {
                if let Some(sounds) = sounds.as_mut() {
                    sounds.play(&world.packs, sound.event, Some(at(sound.at)), sound.volume, sound.pitch);
                }
            }
        }
        if !changes.is_empty() {
            let registries = &globals.registries;
            let mut positions = Vec::with_capacity(changes.len());
            for (pos, block) in changes {
                let state = block.as_ref().and_then(|b| terrain.stream.states.state_of(b));
                let shape = state.map_or(0, |state| shape_id(shapes, shape_ids, registries, state));
                sim::voxel::set_block_shape(pos.0, pos.1, pos.2, shape);
                let before = minecraftoss_player::World::block(&terrain.scene, pos);
                let shulker_before = minecraft_terrain::scene::Scene::block(&terrain.scene, pos)
                    .filter(|b| minecraft_terrain::containers::Station::of(b) == Some(minecraft_terrain::containers::Station::Shulker))
                    .map(|b| b.id.key());
                terrain.scene.set(pos, block);
                if let Some(id) = shulker_before
                    && minecraft_terrain::scene::Scene::block(&terrain.scene, pos).and_then(minecraft_terrain::containers::Station::of) != Some(minecraft_terrain::containers::Station::Shulker)
                    && let Some(components) = containers.take_shulker(pos)
                {
                    entities.keep_dropped_contents(pos, &id, &components);
                }
                if let (Some(before), Some(after)) = (before, minecraftoss_player::World::block(&terrain.scene, pos))
                    && let Some(sound) = minecraft_terrain::doors::release_sound(&before, &after).or_else(|| {
                        let toggled = minecraft_terrain::doors::powered_toggle(&before, &after);
                        let now = time.elapsed_secs_f64();
                        let placed = hand.recent.iter().position(|(p, b, at)| *p == pos && *b == before && now - at < RECENT_PLACE_SECONDS).filter(|_| toggled);
                        if let Some(index) = placed {
                            hand.recent.swap_remove(index);
                        }
                        toggled.then(|| minecraft_terrain::doors::toggle_sound(&before)).flatten().filter(|_| placed.is_none())
                    })
                    && let Some(sounds) = sounds.as_mut()
                {
                    let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                    let pitch = sound.pitch + sounds.random() * sound.pitch_spread;
                    sounds.play(&world.packs, &sound.event, Some(at(centre)), sound.volume, pitch);
                }
                positions.push(pos);
            }
            for &pos in &positions {
                if !minecraft_terrain::scene::Scene::block(&terrain.scene, pos).is_some_and(|b| minecraft_terrain::containers::Station::of(b).is_some() || minecraft_terrain::campfire::is_campfire(b)) {
                    spill(containers, entities, pos);
                }
            }
            terrain.stream.record_edits(&terrain.scene, &positions);
            terrain.stream.mark_edited(&terrain.scene, &positions);
        }
        for (pos, block) in std::mem::take(&mut entities.destroyed) {
            mining.burst(&world.packs, &terrain.scene, &world.atlas, pos, &block);
            if let (Some(sounds), Some(kind)) = (sounds.as_mut(), terrain.scene.sound_type(&block)) {
                let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                sounds.play(&world.packs, &kind.break_sound, Some(at(centre)), (kind.volume + 1.0) / 2.0, kind.pitch * 0.8);
            }
        }
        let armor = minecraft_survival::survival::armor(inventory);
        for (amount, from, toward, cause) in hits.into_iter().filter(|_| !player_mode.invulnerable()) {
            let blocked = hand.blocking.filter(|blocking| blocking.raised()).map_or(0.0, |_| minecraft_survival::offhand::blocked_damage(mc_yaw, toward, amount));
            if let Some(blocking) = hand.blocking.filter(|_| blocked > 0.0) {
                if inventory.wear_tool(blocking.slot, minecraft_survival::offhand::shield_wear(blocked)) {
                    hand.blocking = None;
                    let pitch = 0.8 + entities.random_float() * 0.4;
                    if let Some(sounds) = sounds.as_mut() {
                        sounds.play(&world.packs, minecraft_survival::offhand::BREAK_SOUND, None, 0.8, pitch);
                    }
                }
                let pitch = 0.8 + entities.random_float() * 0.4;
                if let Some(sounds) = sounds.as_mut() {
                    sounds.play(&world.packs, minecraft_survival::offhand::BLOCK_SOUND, None, 1.0, pitch);
                }
            }
            if amount > blocked {
                survival.hurt(amount - blocked, armor, from, cause);
            } else {
                survival.hurt_blocked(cause);
            }
        }
        for used in std::mem::take(&mut entities.item_uses_done) {
            if used.outcome.consumed {
                hand.swing = Some(0.0);
                hand.swing_offhand = used.slot == minecraft_survival::offhand::OFFHAND;
            }
            finish_item_use(entities, inventory, sounds.as_mut(), &world.packs, origin, used, player_mode.consumes_items());
        }
        for to in teleports.iter().filter(|_| alive) {
            authority.0.teleport(local.0, sim::voxel::to_map(origin, *to));
            steps.air_peak = None;
        }
        for landing in std::mem::take(&mut entities.pearl_landings) {
            if !alive {
                continue;
            }
            authority.0.teleport(local.0, sim::voxel::to_map(origin, landing.to_array()));
            survival.hurt(minecraft_terrain::thrown::PEARL_DAMAGE, minecraftoss_player::survival::Armor::default(), None, minecraft_survival::death::DamageCause::EnderPearl);
            steps.air_peak = None;
            if let Some(sounds) = sounds.as_mut() {
                sounds.play(&world.packs, "minecraft:entity.player.teleport", Some(at(landing.to_array())), 1.0, 1.0);
            }
        }
        hold_off_death(survival, inventory, *selected, sounds.as_mut(), &world.packs, totem_popped);
        if alive
            && !sim::voxel::player_damage_pending()
            && let Some(player) = authority.0.player(local.0).copied()
        {
            let settle = survival.settle(player.health, player.max_health);
            if let Some(health) = settle.set {
                authority.0.set_health(local.0, health);
            }
            if let Some((amount, from)) = settle.damage {
                let death = (survival.status.health <= 0.0).then(|| {
                    let line = match inventory_ui.language() {
                        Some(language) => survival.death_line(language, |id| entities.mob_present(id)),
                        None => survival.death_line(&minecraft_terrain::item_icons::language(&world.packs).unwrap_or_else(|_| HashMap::new()), |id| entities.mob_present(id)),
                    };
                    sim::voxel::MinecraftDeath { message: line.message, killer: line.killer }
                });
                if let Some(death) = &death
                    && globals.game_rules.bool("show_death_messages")
                {
                    let name = globals.chat.name(minecraft_survival::commands::HOST).unwrap_or("Player");
                    crate::minecraft_chat::show(&mut ui.chat, &[minecraft_survival::commands::death_message(death.line(name))], &HashMap::new());
                }
                sim::voxel::push_player_damage(local.0.0, amount, from.map(|b| sim::voxel::to_map(origin, b)), death);
            }
            if let Some(blocked) = survival.sprint_change(player.pm_flags & playerstate_iw4::pm_flags::SPRINT_BLOCKED != 0) {
                authority.0.set_sprint_blocked(local.0, blocked);
            }
        }
        ui.vitals = alive.then(|| mc_vitals(survival.vitals()));
        ui.fire_frames = fire_frames.clone();
        // The inventory: MW2 guns as items, the HUD's clicks, the hotbar's
        // gun, and what the HUD shows.
        let owned: Vec<u32> = ps
            .weapons
            .iter()
            .filter(|&&w| w > 0)
            .map(|&w| w as u32)
            .filter(|&w| authority.0.weapon_combat_row(w).is_some_and(|facts| facts.inventory_type == 0))
            .collect();
        ui.active = alive;
        if !alive {
            ui.inventory_open = false;
            ui.chat.close();
        }
        if !player_mode.interacts() {
            ui.inventory_open = false;
            ui.select = None;
            ui.drop_selected = None;
            ui.swap_hands = false;
        }
        if inventory_ui.table().is_some_and(|pos| {
            !minecraft_terrain::crafting::within_table_reach(eye, pos)
                || !minecraft_terrain::scene::Scene::block(&terrain.scene, pos).is_some_and(minecraft_terrain::crafting::is_crafting_table)
        }) {
            ui.inventory_open = false;
        }
        if inventory_ui.block().is_some_and(|opened| {
            opened.cells.iter().any(|&pos| {
                !minecraft_terrain::crafting::within_table_reach(eye, pos)
                    || minecraft_terrain::scene::Scene::block(&terrain.scene, pos).and_then(minecraft_terrain::containers::Station::of) != Some(opened.station)
            })
        }) {
            ui.inventory_open = false;
        }
        if inventory_ui.bench().is_some_and(|(pos, kind)| {
            !minecraft_terrain::crafting::within_table_reach(eye, pos)
                || minecraft_terrain::scene::Scene::block(&terrain.scene, pos).and_then(minecraft_terrain::stations::Workshop::of) != Some(kind)
        }) {
            ui.inventory_open = false;
        }
        inventory_ui.level = survival.status.experience_level;
        inventory_ui.sync_weapons(inventory, &owned);
        if std::mem::take(&mut ui.swap_hands) && alive && minecraft_survival::offhand::swap_hands(inventory, *selected) {
            survival.stop_eating();
            hand.blocking = None;
        }
        let was_open = inventory_ui.block().cloned();
        let roll = entities.random_float();
        let thrown = inventory_ui.apply_input(&mut ui, inventory, selected, containers, &mut || roll);
        if let Some(pos) = open_table {
            inventory_ui.open_table(&mut ui, pos);
        }
        if let Some(opened) = open_block.take() {
            let storage = matches!(opened.station, minecraft_terrain::containers::Station::Chest | minecraft_terrain::containers::Station::Barrel);
            for &pos in opened.cells.iter().filter(|_| storage) {
                if containers.is_new(pos)
                    && let Some(chunk) = terrain.scene.generated_chunk((pos.0.div_euclid(16), pos.2.div_euclid(16)))
                    && let Some(entity) = chunk.block_entities.entities.get(&pos).or_else(|| chunk.block_entities.pending.get(&pos))
                    && let Some((items, seed)) = minecraft_terrain::containers::structure_loot(&globals.registries, entity, pos, globals.seed, &entities.recipes)
                {
                    containers.fill(pos, items, seed);
                }
            }
            let named = containers.custom_name(opened.cells[0]);
            if inventory_ui.open_block(&mut ui, opened)
                && let Some(name) = named
            {
                ui.screen_title = name;
            }
        }
        if let Some((pos, kind)) = open_bench.take() {
            if kind == minecraft_terrain::stations::Workshop::Enchanting {
                let block_at = |at: (i32, i32, i32)| minecraft_terrain::scene::Scene::block(&terrain.scene, at);
                inventory_ui.shelves = minecraft_terrain::stations::bookshelves(
                    pos,
                    |at| block_at(at).is_some_and(|b| b.id.key() == "minecraft:bookshelf"),
                    |at| {
                        block_at(at).is_none_or(|b| {
                            terrain.stream.states.state_of(b).is_some_and(|state| globals.registries.blocks.is(state, minecraftoss_core::block::flags::REPLACEABLE))
                        })
                    },
                );
            }
            inventory_ui.open_bench(&mut ui, pos, kind);
        }
        survival.spend_levels(inventory_ui.take_spent_levels());
        for (event, pos) in std::mem::take(&mut inventory_ui.used) {
            let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
            let pitch = entities.random_float() * 0.1 + 0.9;
            if let Some(sounds) = sounds.as_mut() {
                sounds.play(&world.packs, event, Some(at(centre)), 1.0, pitch);
            }
        }
        if let Some(pos) = inventory_ui.anvil_used.take() {
            let block = minecraft_terrain::scene::Scene::block(&terrain.scene, pos).cloned();
            let worn = entities.random_float() < 0.12;
            let next = block.as_ref().filter(|_| worn).map(|b| match b.id.key().as_str() {
                "minecraft:anvil" => Some("minecraft:chipped_anvil"),
                "minecraft:chipped_anvil" => Some("minecraft:damaged_anvil"),
                _ => None,
            });
            let event = if next == Some(None) { "minecraft:block.anvil.destroy" } else { "minecraft:block.anvil.use" };
            if let Some(next) = next {
                let state = next.map(|id| {
                    let mut worn = minecraft_terrain::scene::Block::new(id);
                    worn.properties = block.as_ref().map_or_else(Default::default, |b| b.properties.clone());
                    worn
                });
                write_blocks(terrain, &globals.registries, shapes, shape_ids, entities, vec![(pos, state)]);
            }
            let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
            let pitch = entities.random_float() * 0.1 + 0.9;
            if let Some(sounds) = sounds.as_mut() {
                sounds.play(&world.packs, event, Some(at(centre)), 1.0, pitch);
            }
        }
        let now_open = inventory_ui.block().cloned();
        for (opened, open) in [(was_open.clone().filter(|o| now_open.as_ref() != Some(o)), false), (now_open.filter(|o| was_open.as_ref() != Some(o)), true)] {
            let Some(opened) = opened else { continue };
            let sound = match opened.station {
                minecraft_terrain::containers::Station::Chest => {
                    let path = minecraft_terrain::scene::Scene::block(&terrain.scene, opened.cells[0]).map_or_else(|| "chest".to_owned(), |b| b.id.path.clone());
                    Some(minecraft_terrain::containers::chest_sound(&path, open))
                }
                minecraft_terrain::containers::Station::Ender => Some(if open { "minecraft:block.ender_chest.open" } else { "minecraft:block.ender_chest.close" }),
                minecraft_terrain::containers::Station::Shulker => Some(if open { "minecraft:block.shulker_box.open" } else { "minecraft:block.shulker_box.close" }),
                minecraft_terrain::containers::Station::Barrel => Some(if open { "minecraft:block.barrel.open" } else { "minecraft:block.barrel.close" }),
                minecraft_terrain::containers::Station::Oven(_) | minecraft_terrain::containers::Station::Brewer => None,
            };
            let count = opened.cells.len() as f64;
            let centre = opened.cells.iter().fold([0.0; 3], |sum, &(x, y, z)| [sum[0] + (x as f64 + 0.5) / count, sum[1] + (y as f64 + 0.5) / count, sum[2] + (z as f64 + 0.5) / count]);
            let pitch = entities.random_float() * 0.1 + 0.9;
            if let (Some(event), Some(sounds)) = (sound, sounds.as_mut()) {
                sounds.play(&world.packs, event, Some(at(centre)), 0.5, pitch);
            }
            if opened.station == minecraft_terrain::containers::Station::Barrel
                && let Some(block) = minecraft_terrain::scene::Scene::block(&terrain.scene, opened.cells[0]).cloned()
                && block.properties.contains_key("open")
            {
                let state = block.with("open", if open { "true" } else { "false" });
                write_blocks(terrain, &globals.registries, shapes, shape_ids, entities, vec![(opened.cells[0], Some(state))]);
            }
        }
        entities.experience_found += inventory_ui.take_experience() as f32;
        let found = std::mem::take(&mut entities.experience_found);
        let points = found.floor() as u32 + u32::from(found.fract() > 0.0 && entities.random_float() < found.fract());
        if points > 0 {
            let (r1, r2) = (entities.random_float(), entities.random_float());
            let chime = survival.gain_experience(points);
            if let Some(sounds) = sounds.as_mut() {
                sounds.play(&world.packs, "minecraft:entity.experience_orb.pickup", None, 0.1, (r1 - r2) * 0.35 + 0.9);
                if let Some(volume) = chime {
                    sounds.play(&world.packs, "minecraft:entity.player.levelup", None, volume, 1.0);
                }
            }
        }
        let thrower = crate::minecraft_inventory::Thrower {
            eye: glam::DVec3::from_array(eye),
            yaw: mc_yaw,
            pitch: ps.viewangles[0],
        };
        if let Some(death) = death_watch.take()
            && !death.keep_inventory
        {
            for stack in minecraft_survival::death::death_drops(inventory, |stack| minecraft_survival::guns::weapon_of(stack).is_some()) {
                entities.world_items.drop_on_death(stack, glam::DVec3::from_array(death.spot.eye));
            }
            entities.award_experience(death.spot.feet, death.experience);
            hand.blocking = None;
        }
        crate::minecraft_inventory::throw(&mut entities.world_items, thrown, &thrower);
        ui.weapon_request = inventory_ui.weapon_request(inventory, selected, ps.weapon).filter(|_| player_mode.interacts());
        inventory_ui.animation_tick = atlas_animation.ticks();
        inventory_ui.publish(&mut ui, inventory, *selected, &world.packs, &mut images, containers);

        if let Some(sounds) = sounds.as_mut() {
            for (event, position, volume, pitch) in std::mem::take(&mut entities.sounds) {
                sounds.play(&world.packs, &event, Some(at(position.to_array())), volume, pitch);
            }
        }
        let placeable = |stack: &minecraftoss_player::inventory::ItemStack| {
            let id = if stack.id == "minecraft:redstone" { "minecraft:redstone_wire" } else { stack.id.as_str() };
            minecraft_terrain::pack::ResourceId::parse(id).is_ok_and(|id| terrain.stream.states.state_of(&minecraft_terrain::scene::Block { id, properties: Default::default() }).is_some())
        };
        let offhand_stack = inventory.slots[minecraft_survival::offhand::OFFHAND].clone();
        let aims_at_campfire = || {
            let mut aim = minecraftoss_player::Player::new(glam::DVec3::from_array(feet));
            aim.yaw = f64::from(mc_yaw);
            aim.pitch = f64::from(ps.viewangles[0]);
            aim.swimming = ps.view_height_current < CRAWLING_BELOW_VIEW_HEIGHT;
            ps.pm_flags & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE) == 0
                && aim.target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0).is_some_and(|hit| minecraft_terrain::scene::Scene::block(&terrain.scene, hit.pos).is_some_and(minecraft_terrain::campfire::is_campfire))
        };
        ui.offhand_usable = alive
            && !holding
            && offhand_stack.as_ref().is_some_and(|stack| {
                minecraft_survival::offhand::has_use(stack, placeable(stack), &survival.status.food, player_mode.mode) || (containers.cooks_on_campfire(stack, &entities.recipes) && aims_at_campfire())
            });
        ui.offhand_busy = alive && !holding && (survival.eating().is_some() || hand.blocking.is_some());
        ui.using_item = alive && (survival.eating().is_some() || hand.blocking.is_some());
        sim::voxel::set_death_protected(local.0.0, alive && !survival.killed() && minecraft_survival::offhand::totem_slot(inventory, *selected).is_some());
        // The held item in view; an empty hand is MW2's own hands.
        view.hand = Default::default();
        let swing = hand.swing.map_or(0.0, |t| (t / crate::minecraft_hand::SWING_TICKS).clamp(0.0, 1.0));
        ui.hand_swing = if hand.swing_offhand { 0.0 } else { swing };
        let main_stack = inventory.slots[*selected].clone().filter(|_| ui.holding_item);
        let offhand_stack = offhand_stack.filter(|_| ui.holding_item || ui.offhand_busy);
        if !puppet.active && !mc_detached && (main_stack.is_some() || offhand_stack.is_some()) {
            let eye_light_at = glam::Vec3::new(eye[0] as f32, eye[1] as f32, eye[2] as f32);
            let mut posed = Vec::new();
            for (slot, stack, arm) in [(*selected, main_stack, 1.0), (minecraft_survival::offhand::OFFHAND, offhand_stack, -1.0)] {
                let Some(stack) = stack else {
                    continue;
                };
                let using = hand.blocking.is_some_and(|blocking| blocking.slot == slot);
                let display = minecraft_terrain::pack::ResourceId::parse(&stack.id)
                    .ok()
                    .and_then(|id| minecraft_terrain::model::item_hand_transform(&world.packs, &id, arm < 0.0, using).ok())
                    .unwrap_or(glam::Mat4::IDENTITY);
                let eat = survival
                    .eating()
                    .filter(|eating| eating.slot == slot)
                    .map(|eating| crate::minecraft_hand::eat_transform(eating.remaining_ticks() as f32, (hand.clock / TICK_SECONDS) as f32, eating.info.consume_ticks, arm));
                let swung = if using || eat.is_some() || hand.swing_offhand != (slot == minecraft_survival::offhand::OFFHAND) { 0.0 } else { swing };
                let glint = minecraft_terrain::glint::has_glint(&stack);
                posed.push((eat.unwrap_or(glam::Mat4::IDENTITY) * crate::minecraft_hand::item_pose(display, swung, 0.0, arm), stack.id, glint));
            }
            let mesh = entities.held_item_mesh(&posed, eye_light_at, &world.packs, &world.atlas, light);
            let vertices = minecraft_terrain::mesh::section_vertices(&mesh);
            view.hand = (bytemuck::cast_slice(&vertices).to_vec(), mesh.indices);
            // Reverse-Z with no far plane, as the scene's; 70 degrees up.
            let f = 1.0 / (35.0f32.to_radians()).tan();
            let near = 0.05;
            view.hand_clip = Mat4::from_cols(
                Vec4::new(f / aspect, 0.0, 0.0, 0.0),
                Vec4::new(0.0, f, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 0.0, -1.0),
                Vec4::new(0.0, 0.0, near, 0.0),
            )
            .to_cols_array();
        }

        sim::voxel::set_mob_boxes(entities.boxes());
        entities.tick_scene(&terrain.scene, mob_ticks);
        let sky_darken = (15.0 - terrain.environment.sky_light_level()).clamp(0.0, 15.0) as u8;
        let campfire_items: Vec<_> = containers
            .campfires()
            .filter(|(pos, _)| minecraft_terrain::campfire::shown(*pos, glam::DVec3::from_array(eye)))
            .filter_map(|(pos, campfire)| {
                let block = minecraft_terrain::scene::Scene::block(&terrain.scene, pos).filter(|b| minecraft_terrain::campfire::is_campfire(b))?;
                Some(campfire.held_items(pos, block.properties.get("facing").map_or("south", String::as_str)))
            })
            .flatten()
            .collect();
        let meshes = entities.meshes(
            &terrain.scene,
            &world.packs,
            &world.atlas,
            light,
            forward,
            glam::DVec3::from_array(eye),
            sky_darken,
            &|pos| terrain.stream.cell_settled(pos),
            &campfire_items,
        );
        let raw = |mesh: &minecraft_terrain::mesh::ChunkMesh| {
            (bytemuck::cast_slice::<_, u8>(&mesh.vertices).to_vec(), mesh.indices.clone())
        };
        view.entity_meshes = [
            raw(&meshes.models),
            raw(&meshes.culled),
            raw(&meshes.translucent),
            raw(&meshes.shadows),
        ];
        let mesh = meshes.items;
        let (bytes, indices) = &mut view.particles;
        let base = (bytes.len() / std::mem::size_of::<minecraft_terrain::mesh::SectionVertex>()) as u32;
        let vertices = minecraft_terrain::mesh::section_vertices(&mesh);
        bytes.extend_from_slice(bytemuck::cast_slice(&vertices));
        indices.extend(mesh.indices.iter().map(|i| i + base));
    }
    view.cracks = mining.crack_mesh(&terrain.scene, &terrain.stream.states, &world.packs);
    if globals.game_rules.bool("advance_time") && !globals.day.paused {
        globals.day.advance(dt);
    } else if let Some(save) = globals.save.as_mut() {
        save.shift_day(-dt * 20.0);
    }
    let eye_block = (eye[0].floor() as i32, eye[1].floor() as i32, eye[2].floor() as i32);
    view.eye_light = [
        f32::from(light.get(eye_block)),
        f32::from(light.get_block(eye_block)),
    ];
    let weather = entities.as_ref().map_or_else(minecraft_terrain::server::WeatherView::default, |e| e.weather);
    let rains_here = terrain
        .scene
        .noise_biome((eye_block.0 >> 2, eye_block.1 >> 2, eye_block.2 >> 2))
        .is_some_and(|biome| globals.registries.biomes.get(biome).has_precipitation);
    terrain
        .environment
        .update_rain_fog(weather.rain_level, light.get(eye_block), rains_here, (dt * 20.0) as f32);
    *environment_accumulator += dt;
    if !*environment_primed || *environment_accumulator >= TICK_SECONDS {
        *environment_accumulator = (*environment_accumulator % TICK_SECONDS).min(TICK_SECONDS);
        let scene = &terrain.scene;
        terrain.environment.tick(
            globals.day.ticks.floor() as i64,
            weather.rain_level,
            weather.thunder_level,
            eye,
            |x, y, z| scene.noise_biome((x, y, z)).map_or(0, |id| id.0),
            !*environment_primed,
        );
        *environment_primed = true;
    }
    let partial_tick = (*environment_accumulator / TICK_SECONDS).clamp(0.0, 1.0) as f32;
    let sky = terrain.environment.sky_state(&View {
        partial_tick,
        forward,
        camera_y: eye[1] as f32,
        render_distance: VIEW_DISTANCE as u32,
        rain_level: weather.rain_level,
        thunder_level: weather.thunder_level,
    });
    let render_distance = VIEW_DISTANCE as f32 * 16.0;
    let right = forward.cross(glam::Vec3::Y).normalize_or(glam::Vec3::X);
    let up = right.cross(forward).normalize_or(glam::Vec3::Y);
    let put = |v: glam::Vec3| [v.x, v.y, v.z, 0.0];
    let game_time = globals.day.ticks;
    view.environment = [
        put(forward),
        put(right),
        put(up),
        [eye[0] as f32, eye[1] as f32, eye[2] as f32, 0.0],
        put(sky.sky),
        [sky.fog.x, sky.fog.y, sky.fog.z, render_distance.min(sky.sky_fog_end)],
        [
            sky.sky_light_color.x,
            sky.sky_light_color.y,
            sky.sky_light_color.z,
            sky.sky_light_factor,
        ],
        sky.sunset,
        [sky.sun_direction.x, sky.sun_direction.y, sky.sun_direction.z, sky.rain_brightness],
        [sky.moon_direction.x, sky.moon_direction.y, sky.moon_direction.z, sky.rain_brightness],
        // The brightness option at its default.
        [sky.cloud.x, sky.cloud.y, sky.cloud.z, 0.5],
        [aspect, 0.0, sky.star_brightness, sky.star_angle],
        [sky.moon_phase as f32, (game_time as f32) * 0.03, 96.0, 160.0],
        [
            sky.fog_start,
            sky.fog_end,
            render_distance - (render_distance / 10.0).clamp(4.0, 64.0),
            render_distance,
        ],
        [
            sky.ambient.x,
            sky.ambient.y,
            sky.ambient.z,
            match sky.skybox {
                Skybox::Overworld => 0.0,
                Skybox::End => 1.0,
                _ => 2.0,
            },
        ],
        [
            sky.block_light_tint.x,
            sky.block_light_tint.y,
            sky.block_light_tint.z,
            sky.block_factor,
        ],
    ];

    // Clouds, rebuilt when the camera crosses a cloud cell.
    if let Some(mask) = &world.cloud_mask {
        let center = mask.center(eye[0] as f32, eye[2] as f32, game_time);
        if *cloud_center != Some(center) {
            *cloud_center = Some(center);
            let mesh = mask.build(center, eye[1] as f32);
            view.clouds = Some(Arc::new((mesh.vertices, mesh.indices)));
        }
    }

    // The light MW2 models stand in, around the player.
    *light_volume_age += 1;
    let half = LIGHT_VOLUME / 2;
    let corner = [block.0 - half, block.1 - half, block.2 - half];
    let moved =
        light_volume_at.is_none_or(|at| (0..3).any(|k| (at[k] - corner[k]).abs() >= 4));
    if moved || *light_volume_age >= 20 {
        *light_volume_age = 0;
        *light_volume_at = Some(corner);
        let n = LIGHT_VOLUME;
        let index = |x: i32, y: i32, z: i32| (((y * n + z) * n + x) * 2) as usize;
        let mut raw = vec![0u8; (n * n * n * 2) as usize];
        for z in 0..n {
            for x in 0..n {
                let (bx, bz) = (corner[0] + x, corner[2] + z);
                // One column lookup for the whole stack, not two per cell.
                let column = light.chunk_column((bx >> 4, bz >> 4));
                for y in 0..n {
                    let pos = (bx, corner[1] + y, bz);
                    let at = index(x, y, z);
                    let (sky, block) = match column {
                        Some(column) => (column.get(pos), column.get_block(pos)),
                        None => (light.get(pos), light.get_block(pos)),
                    };
                    raw[at] = sky;
                    raw[at + 1] = block;
                }
            }
        }
        // Unlit cells (inside blocks) take their brightest neighbour, so a
        // model beside a block is not darkened by filtering into it. Levels
        // become unorm bytes.
        let mut data = vec![0u8; raw.len()];
        for y in 0..n {
            for z in 0..n {
                for x in 0..n {
                    let at = index(x, y, z);
                    let (mut sky, mut block) = (raw[at], raw[at + 1]);
                    if sky == 0 && block == 0 {
                        for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                            let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                            if (0..n).contains(&nx) && (0..n).contains(&ny) && (0..n).contains(&nz) {
                                let near = index(nx, ny, nz);
                                sky = sky.max(raw[near]);
                                block = block.max(raw[near + 1]);
                            }
                        }
                    }
                    data[at] = sky.min(15) * 17;
                    data[at + 1] = block.min(15) * 17;
                }
            }
        }
        view.light_volume = Some(Arc::new((corner, data)));
    }
}

fn apply_streamed(
    registries: &Registries,
    shapes: &mut HashMap<BlockStateId, u16>,
    shape_ids: &mut HashMap<(Vec<[u32; 6]>, u16), u16>,
    entities: &mut Option<minecraft_survival::entities::Entities>,
    (loaded, forgotten): (Vec<Arc<minecraftoss_core::Chunk>>, Vec<minecraftoss_core::ChunkPos>),
) {
    for chunk in loaded {
        let (min_y, height) = (chunk.min_y(), chunk.height());
        let mut ids = vec![0u16; (height * 256) as usize];
        let mut last = None;
        for y in 0..height {
            for z in 0..16usize {
                for x in 0..16usize {
                    let state = chunk.block(x, min_y + y, z);
                    if let Some((last_state, id)) = last
                        && last_state == state
                    {
                        ids[((y as usize * 16) + z) * 16 + x] = id;
                        continue;
                    }
                    let id = shape_id(shapes, shape_ids, registries, state);
                    last = Some((state, id));
                    ids[((y as usize * 16) + z) * 16 + x] = id;
                }
            }
        }
        if let Some(entities) = entities.as_mut() {
            entities.load_chunk(&chunk);
        }
        sim::voxel::set_chunk(
            chunk.pos.x,
            chunk.pos.z,
            sim::voxel::VoxelChunk {
                min_y,
                height,
                shapes: ids,
            },
        );
    }
    for pos in forgotten {
        sim::voxel::remove_chunk(pos.x, pos.z);
        if let Some(entities) = entities.as_mut() {
            entities.unload_chunk(pos);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stream_around_spawn(
    world: &Loaded,
    terrain: &mut minecraft_survival::level::Terrain,
    registries: &Registries,
    shapes: &mut HashMap<BlockStateId, u16>,
    shape_ids: &mut HashMap<(Vec<[u32; 6]>, u16), u16>,
    entities: &mut Option<minecraft_survival::entities::Entities>,
    light: &mut Option<SkyLight>,
    gate: &mut crate::minecraft_spawn_gate::SpawnGate,
    view: &mut MinecraftWorldView,
    aspect: f32,
    now_ms: u64,
) {
    let [x, y, z] = view.origin;
    let spawn_block = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let streamed = terrain.stream.server_tick(spawn_block, &mut terrain.scene);
    apply_streamed(registries, shapes, shape_ids, entities, streamed);
    let camera = CullCamera {
        position: glam::DVec3::new(x, y + crate::minecraft_spawn_gate::SPAWN_EYE_HEIGHT, z),
        forward: glam::Vec3::Z,
        fov_degrees: 90.0,
        aspect,
        yaw_degrees: 0.0,
        pitch_degrees: 0.0,
    };
    let update = terrain.stream.frame(&terrain.scene, &camera, FADE_MILLIS, &world.atlas, &world.packs);
    view.uploads.extend(update.uploads);
    view.removed.extend(update.removed);
    view.visible = update.visible;
    if let Some(light) = light.as_mut() {
        for (chunk, column) in update.lights {
            light.set_chunk_column(chunk, column);
        }
    }
    observe_spawn(terrain, gate, view.origin, camera.position, now_ms);
}

fn observe_spawn(terrain: &minecraft_survival::level::Terrain, gate: &mut crate::minecraft_spawn_gate::SpawnGate, origin: [f64; 3], eye: glam::DVec3, now_ms: u64) {
    if !gate.waiting() {
        return;
    }
    let spawn_chunk = ((origin[0].floor() as i32) >> 4, (origin[2].floor() as i32) >> 4);
    let chunks_ready = sim::voxel::chunks_ready_around(spawn_chunk.0, spawn_chunk.1, crate::minecraft_spawn_gate::SPAWN_CHUNK_RADIUS);
    let eye = eye.floor();
    let section_visible = terrain.stream.section_compiled_and_visible((eye.x as i32, eye.y as i32, eye.z as i32), FADE_MILLIS);
    gate.observe(now_ms, chunks_ready, section_visible);
}

fn shape_id(
    shapes: &mut HashMap<BlockStateId, u16>,
    shape_ids: &mut HashMap<(Vec<[u32; 6]>, u16), u16>,
    registries: &Registries,
    state: BlockStateId,
) -> u16 {
    *shapes.entry(state).or_insert_with(|| {
        let blocks = &registries.blocks;
        let boxes = blocks.collision_boxes(state);
        let fluid = blocks.state(state).fluid.map_or(0, |f| {
            let kind = match f.kind {
                minecraftoss_core::block::FluidKind::Water => sim::voxel::WATER,
                minecraftoss_core::block::FluidKind::Lava => sim::voxel::LAVA,
            };
            kind | u16::from(f.amount).min(sim::voxel::FLUID_AMOUNT)
        }) | match blocks.block(blocks.block_of(state)).name.as_str() {
            "minecraft:sweet_berry_bush" => sim::voxel::STUCK_BERRY_BUSH,
            "minecraft:cobweb" => sim::voxel::STUCK_COBWEB,
            _ => 0,
        };
        let scaffolding = blocks.block(blocks.block_of(state)).name.as_str() == minecraft_terrain::scaffolding::SCAFFOLDING;
        let climbable = registries.block_tags.id("minecraft:climbable").is_some_and(|tag| registries.block_in_tag(state, tag));
        let hanging = scaffolding && blocks.property(state, "distance") != Some("0") && blocks.property(state, "bottom") == Some("true");
        let fluid = fluid
            | (u16::from(climbable) * sim::voxel::CLIMBABLE)
            | (u16::from(scaffolding) * sim::voxel::SCAFFOLDING)
            | (u16::from(hanging) * sim::voxel::SCAFFOLDING_BOTTOM);
        if boxes.is_empty() && fluid == 0 {
            return 0;
        }
        let key = (boxes.iter().map(|b| b.map(|v| (v as f32).to_bits())).collect::<Vec<[u32; 6]>>(), fluid);
        if let Some(&id) = shape_ids.get(&key) {
            return id;
        }
        let boxes32 = boxes.iter().map(|b| b.map(|v| v as f32)).collect();
        let id = sim::voxel::add_fluid_shape(boxes32, fluid).unwrap_or(0);
        shape_ids.insert(key, id);
        id
    })
}

fn spill(containers: &mut minecraft_terrain::containers::Containers, entities: &mut minecraft_survival::entities::Entities, pos: (i32, i32, i32)) {
    let (items, experience) = containers.remove(pos);
    for stack in items {
        entities.world_items.drop_item_stack(stack, pos);
    }
    entities.experience_found += experience;
}

fn write_blocks(
    terrain: &mut minecraft_survival::level::Terrain,
    registries: &Registries,
    shapes: &mut HashMap<BlockStateId, u16>,
    shape_ids: &mut HashMap<(Vec<[u32; 6]>, u16), u16>,
    entities: &mut minecraft_survival::entities::Entities,
    writes: Vec<((i32, i32, i32), Option<minecraft_terrain::scene::Block>)>,
) {
    let positions: Vec<_> = writes.iter().map(|(p, _)| *p).collect();
    for (at, block) in writes {
        let state = block.as_ref().and_then(|b| terrain.stream.states.state_of(b));
        let shape = state.map_or(0, |state| shape_id(shapes, shape_ids, registries, state));
        sim::voxel::set_block_shape(at.0, at.1, at.2, shape);
        terrain.scene.set(at, block);
    }
    terrain.stream.record_edits(&terrain.scene, &positions);
    terrain.stream.mark_edited(&terrain.scene, &positions);
    for &at in &positions {
        if minecraft_terrain::scene::Scene::block(&terrain.scene, at).is_some() {
            entities.placed(&terrain.scene, at);
        } else {
            entities.broke(&[at]);
        }
    }
}

fn finish_item_use(
    entities: &mut minecraft_survival::entities::Entities,
    inventory: &mut minecraftoss_player::inventory::Inventory,
    sounds: Option<&mut crate::minecraft_sounds::Sounds>,
    packs: &PackStack,
    origin: [f64; 3],
    used: minecraft_survival::entities::SentItemUse,
    consumes: bool,
) {
    let mut heard = used.outcome.sounds;
    let spends = consumes || used.outcome.result.is_some();
    if inventory.slots.get(used.slot).and_then(Option::as_ref).is_some_and(|stack| stack.id == used.held) {
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
            && let Some(rest) = minecraft_terrain::bottle::filled_result(inventory, used.slot, result)
        {
            entities.world_items.toss(rest, used.eye, used.yaw, used.pitch);
        }
        if spends && inventory.wear_tool(used.slot, used.outcome.wear) {
            heard.push(minecraft_terrain::flint::break_sound(entities.random_float()));
        }
    }
    if let Some(sounds) = sounds {
        for sound in heard {
            sounds.play(packs, sound.event, sound.at.map(|at| Vec3::from_array(sim::voxel::to_map(origin, at))), sound.volume, sound.pitch);
        }
    }
}

fn hold_off_death(
    survival: &mut minecraft_survival::survival::Survival,
    inventory: &mut minecraftoss_player::inventory::Inventory,
    selected: usize,
    sounds: Option<&mut crate::minecraft_sounds::Sounds>,
    packs: &PackStack,
    struck: bool,
) {
    if (struck || survival.status.health <= 0.0)
        && survival.protect(inventory, selected)
        && let Some(sounds) = sounds
    {
        sounds.play(packs, minecraft_survival::offhand::TOTEM_SOUND, None, 1.0, 1.0);
    }
}

fn scene_block(block: &minecraftoss_player::Block) -> Option<minecraft_terrain::scene::Block> {
    Some(minecraft_terrain::scene::Block {
        id: minecraft_terrain::pack::ResourceId::parse(&block.id).ok()?,
        properties: block.properties.clone(),
    })
}

fn obstacles(
    predicted: &playerstate_iw4::PlayerState,
    authority: &sim::SimWorld,
    local: sim::ClientId,
    entities: &minecraft_survival::entities::Entities,
) -> Vec<[f64; 6]> {
    let hull = |ps: &playerstate_iw4::PlayerState| {
        let (mins, maxs) = sim::voxel::player_hull(ps);
        sim::voxel::body_box(ps.origin, mins, maxs)
    };
    let mut out: Vec<[f64; 6]> = authority.player(local).map_or_else(|| hull(predicted), hull).into_iter().collect();
    authority.visit_players(|id, ps| {
        let standing = matches!(
            ps.pm_type,
            0 | playerstate_iw4::PM_TYPE_NORMAL_LINKED | playerstate_iw4::PM_TYPE_LAST_STAND
        );
        if id != local && standing {
            out.extend(hull(ps));
        }
    });
    out.extend(entities.boxes().into_iter().map(|(_, b)| b));
    out
}

fn starting_game_mode(saved: Option<minecraft_terrain::game_mode::PlayerMode>) -> minecraft_terrain::game_mode::PlayerMode {
    use minecraft_terrain::game_mode::{GameMode, PlayerMode, from_id, parse};
    let chosen = std::env::var("IW4L_MINECRAFT_GAMEMODE").ok().and_then(|name| {
        let name = name.trim().to_ascii_lowercase();
        parse(&name).or_else(|| name.parse().ok().and_then(from_id))
    });
    let mut mode = saved.unwrap_or_else(|| PlayerMode::new(GameMode::Survival));
    if let Some(chosen) = chosen {
        mode.set(chosen);
    }
    mode
}

fn save_player_mode(globals: &mut minecraft_survival::server::Globals, mode: minecraft_terrain::game_mode::PlayerMode) {
    let Some(save) = globals.save.clone() else {
        return;
    };
    if let Some(previous) = globals.level_writer.take() {
        let _ = previous.join();
    }
    globals.level_writer = std::thread::Builder::new()
        .name("minecraft-player-save".into())
        .spawn(move || {
            if let Err(error) = save.write_host(&mode) {
                diag::warn!(World, "Minecraft player file not written: {error}");
            }
        })
        .map_err(|error| diag::warn!(World, "Minecraft player file thread not started: {error}"))
        .ok();
}

fn ability_bits(mode: &minecraft_terrain::game_mode::PlayerMode) -> u32 {
    use playerstate_iw4::pm_flags::{MC_INVULNERABLE, MC_MAY_FLY, MC_NOCLIP};
    [(mode.abilities.invulnerable, MC_INVULNERABLE), (mode.abilities.may_fly, MC_MAY_FLY), (mode.noclip(), MC_NOCLIP)]
        .into_iter()
        .filter(|&(on, _)| on)
        .fold(0, |bits, (_, bit)| bits | bit)
}

fn ui_game_mode(mode: minecraftoss_player::GameMode) -> frame::McGameMode {
    match mode {
        minecraftoss_player::GameMode::Survival => frame::McGameMode::Survival,
        minecraftoss_player::GameMode::Creative => frame::McGameMode::Creative,
        minecraftoss_player::GameMode::Adventure => frame::McGameMode::Adventure,
        minecraftoss_player::GameMode::Spectator => frame::McGameMode::Spectator,
    }
}

fn mc_vitals(vitals: minecraft_survival::survival::Vitals) -> frame::McVitals {
    frame::McVitals {
        air: vitals.air,
        max_air: vitals.max_air,
        eye_in_water: vitals.eye_in_water,
        health: vitals.health,
        max_health: vitals.max_health,
        absorption: vitals.absorption,
        food: vitals.food,
        saturation: vitals.saturation,
        poisoned: vitals.poisoned,
        hunger: vitals.hunger,
        regen_index: vitals.regen_index,
        blink: vitals.blink,
        display_health: vitals.display_health,
        tick: vitals.tick,
        experience_level: vitals.experience_level,
        experience_progress: vitals.experience_progress,
        on_fire: vitals.on_fire,
        eye_in_lava: vitals.eye_in_lava,
    }
}

fn autosave(globals: &mut minecraft_survival::server::Globals, terrain: &mut minecraft_survival::level::Terrain, mode: minecraft_terrain::game_mode::PlayerMode) {
    let Some(save) = globals.save.clone() else {
        return;
    };
    let started = std::time::Instant::now();
    let chunks = terrain.stream.queue_save();
    if let Some(previous) = globals.level_writer.take() {
        let _ = previous.join();
    }
    let path = save.path().display().to_string();
    let spawn = terrain.stream.world_spawn;
    let rules = globals.game_rules.clone();
    let difficulty = globals.difficulty;
    let day_ticks = globals.day.ticks;
    globals.level_writer = std::thread::Builder::new()
        .name("minecraft-level-save".into())
        .spawn(move || {
            if let Err(error) = save.write(spawn, Some(day_ticks), &rules, difficulty) {
                diag::warn!(World, "Minecraft autosave: level.dat not written: {error}");
            }
            if let Err(error) = save.write_host(&mode) {
                diag::warn!(World, "Minecraft autosave: player file not written: {error}");
            }
        })
        .map_err(|error| diag::warn!(World, "Minecraft autosave: level.dat thread not started: {error}"))
        .ok();
    diag::info!(
        World,
        "Minecraft autosave: {chunks} chunks queued for {path} in {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

fn save_before_exit(mut exit: MessageReader<AppExit>, mut runtime: NonSendMut<Runtime>) {
    if exit.read().last().is_some() {
        close_world(&mut runtime, true);
    }
}

fn close_world(runtime: &mut Runtime, blocking: bool) -> bool {
    let mut server = runtime.server.take();
    let local = runtime.local.take();
    if let (Some(server), Some(local)) = (server.as_mut(), local.as_ref())
        && let Some(player) = server.players.get_mut(local)
    {
        runtime.carried_survival = Some(std::mem::take(&mut player.survival));
    }
    let world = runtime.world.take();
    let loading = runtime.loading.take();
    let loader = runtime.loader.take();
    let open = world.is_some() || loading.is_some();
    if server.is_some() || open || loader.is_some() {
        let close = move || save_and_close(server, world, loading, loader, local);
        if blocking {
            close();
        } else {
            match std::thread::Builder::new().name("minecraft-world-save".into()).spawn(close) {
                Ok(handle) => runtime.closing.push(handle),
                Err(error) => diag::warn!(World, "Minecraft world save thread not started: {error}"),
            }
        }
    }
    if blocking {
        for handle in runtime.closing.drain(..) {
            let _ = handle.join();
        }
    }
    open
}

fn save_and_close(
    mut server: Option<minecraft_survival::server::Server>,
    world: Option<Loaded>,
    loading: Option<mpsc::Receiver<Result<(Loaded, minecraft_survival::server::Server), String>>>,
    loader: Option<std::thread::JoinHandle<()>>,
    local: Option<minecraft_net::PlayerUuid>,
) {
    let started = std::time::Instant::now();
    for level in server.iter_mut().flat_map(|server| server.levels.values_mut()) {
        drop(level.entities.take());
    }
    if let Some(loader) = loader {
        let _ = loader.join();
    }
    drop(loading);
    let (Some(world), Some(mut server)) = (world, server) else {
        return;
    };
    let save = server.globals.save.take();
    if let Some(writer) = server.globals.level_writer.take() {
        let _ = writer.join();
    }
    let Some(spawn) = server.levels.get(&Dimension::Overworld).map(|level| level.terrain.stream.world_spawn) else {
        return;
    };
    let rules = std::mem::take(&mut server.globals.game_rules);
    let difficulty = server.globals.difficulty;
    let day_ticks = server.globals.day.ticks;
    let player_mode = local.and_then(|uuid| server.players.get(&uuid)).map(|player| player.mode);
    drop(server);
    drop(world);
    let Some(save) = save else {
        return;
    };
    if let Err(error) = save.write(spawn, Some(day_ticks), &rules, difficulty) {
        diag::warn!(World, "Minecraft level.dat not written: {error}");
    }
    if let Some(player_mode) = player_mode
        && let Err(error) = save.write_host(&player_mode)
    {
        diag::warn!(World, "Minecraft player file not written: {error}");
    }
    diag::info!(
        World,
        "Minecraft world saved in {} ms: {}",
        started.elapsed().as_millis(),
        save.path().display()
    );
}

fn stop(runtime: &mut Runtime, view: &mut MinecraftWorldView) {
    if close_world(runtime, false) || view.active {
        sim::voxel::deactivate();
        runtime.spawn_gate = Default::default();
        view.active = false;
        view.atlas = None;
        view.atlas_patches.clear();
        view.uploads.clear();
        view.removed.clear();
        view.visible.clear();
        view.celestial = None;
        view.crack_texture = None;
        view.glint_texture = None;
        view.particles = Default::default();
        view.entity_meshes = Default::default();
        view.hand = Default::default();
        view.cracks = Default::default();
        view.clouds = None;
        view.light_volume = None;
        view.generation += 1;
    }
}
