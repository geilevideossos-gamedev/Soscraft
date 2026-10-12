//! The Minecraft map at run time: MinecraftOSS generates and streams a seeded
//! world around the local player, whose chunks become block collision and
//! whose section meshes go to the renderer. The world sits with its player
//! spawn at map origin, a block to `sim::voxel::BLOCK` map units.
use std::collections::HashMap;
use std::sync::{Arc, mpsc};

use bevy::prelude::*;
use minecraft_terrain::clouds::CloudMask;
use minecraft_terrain::day_cycle::{DayCycle, Skybox};
use minecraft_terrain::environment::View;
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{Atlas, AtlasAnimation, AtlasPatch, SectionMesh, Vertex};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::HandcraftedScene;
use minecraft_terrain::sections::{CullCamera, SectionPos};
use minecraft_terrain::terrain::Dimension;
use minecraftoss_core::BlockStateId;
use minecraftoss_core::registries::{DataPaths, Registries};

pub(crate) const VIEW_DISTANCE: i32 = 8;
pub(crate) const TICK_SECONDS: f64 = 1.0 / 20.0;
pub(crate) const CRAWLING_BELOW_VIEW_HEIGHT: f32 = 30.0;
pub(crate) const RECENT_PLACE_SECONDS: f64 = minecraft_survival::actions::RECENT_PLACE_SECONDS;
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
    pub armor_glint_texture: Option<Arc<image::RgbaImage>>,
    /// Mob models (cut out, back-face culled, translucent) and entity
    /// shadows, as `mesh::Vertex` bytes and indices.
    pub entity_meshes: [(Vec<u8>, Vec<u32>); 4],
    /// The black card behind the inventory's character, in blocks.
    pub backdrop: Option<[[f32; 3]; 4]>,
    pub puppet_gear: (Vec<u8>, Vec<u32>),
    /// The first-person hand or held item: section vertex bytes in view
    /// space (x right, y up, z back) and indices, with the projection that
    /// draws them (vanilla's fixed 70 degree hand field of view).
    pub hand: (Vec<u8>, Vec<u32>),
    pub hand_clip: [f32; 16],
    pub end_look: crate::minecraft_end_look::EndFrame,
}

pub(crate) struct Loaded {
    pub(crate) packs: PackStack,
    pub(crate) atlas: Arc<Atlas>,
    celestial: Arc<image::RgbaImage>,
    cloud_mask: Option<CloudMask>,
    crack_texture: Arc<image::RgbaImage>,
    glint_texture: Option<Arc<image::RgbaImage>>,
    armor_glint_texture: Option<Arc<image::RgbaImage>>,
    end_look: Arc<crate::minecraft_end_look::EndLook>,
}

#[derive(Default)]
pub(crate) struct Runtime {
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
    gear_items: minecraft_terrain::mesh::ItemVisuals,
    loader: Option<std::thread::JoinHandle<()>>,
    closing: Vec<std::thread::JoinHandle<()>>,
    shown: Option<Dimension>,
    origins: HashMap<Dimension, [f64; 3]>,
    switch_screen: Option<minecraft_survival::switch::LoadingReason>,
    loading_backgrounds: Option<crate::minecraft_loading::LoadingBackgrounds>,
    refused_air: Option<u32>,
    queues: crate::minecraft_players::VoxelQueues,
    pub(crate) relayed: minecraft_survival::commands::Handled,
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
        .insert_non_send(crate::minecraft_host::HostSessions::default())
        .insert_non_send(crate::minecraft_remote::Friend::default())
        .add_systems(
            Update,
            update
                .after(frame::PresentedPublished)
                .in_set(frame::ClientSet::Present),
        )
        .add_systems(
            Update,
            crate::minecraft_host::host
                .after(update)
                .in_set(frame::ClientSet::Present),
        )
        .add_systems(
            Update,
            crate::minecraft_remote::friend
                .after(update)
                .in_set(frame::ClientSet::Present),
        )
        .add_systems(
            Update,
            draw_body_gear
                .after(update)
                .after(frame::WorkerCmdSet::SkinModel)
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
    let terrain = minecraft_survival::level::Terrain::open(
        registries.clone(),
        seed,
        VIEW_DISTANCE,
        Dimension::Overworld,
        opened.save.as_ref(),
        opened.spawn.as_ref().filter(|spawn| minecraft_survival::save::spawn_dimension(spawn) == Dimension::Overworld).map(|spawn| spawn.pos),
    )?;
    let spawn = opened.spawn.clone().unwrap_or_else(|| minecraft_survival::save::world_spawn(Dimension::Overworld, terrain.stream.world_spawn));
    if let Some(save) = &opened.save
        && let Err(error) = save.write(&spawn, None, &opened.game_rules, opened.difficulty)
    {
        diag::warn!(World, "Minecraft level.dat not written: {error}");
    }
    let mut chat = minecraft_commands::CommandServer::new(minecraft_survival::commands::HOST);
    *chat.registries_mut() = minecraft_survival::commands::registries(&packs);
    *chat.data_mut() = minecraft_survival::commands::command_data(&terrain.stream);
    if let Some(save) = &opened.save
        && let Err(error) = chat.open_ops(&save.ops_path())
    {
        diag::warn!(World, "Minecraft ops.json unreadable, no operators loaded: {error}");
    }
    let loaded = load_look(packs)?;
    let mut day = DayCycle::default();
    if let Some(ticks) = opened.day_ticks {
        day.set(ticks);
    }
    day.paused = opened.time_paused;
    let globals = minecraft_survival::server::Globals {
        seed,
        registries,
        save: opened.save,
        game_rules: opened.game_rules,
        difficulty: opened.difficulty,
        spawn,
        day,
        end_clock: opened.end_clock,
        weather: opened.weather,
        weather_random: minecraft_survival::server::weather_random(seed),
        forced_weather: std::env::var("IW4L_MINECRAFT_WEATHER").ok(),
        chat,
        data: Default::default(),
        writer: Default::default(),
        autosave_due: false,
        level_ticks: 0,
        closing: Vec::new(),
        waiting_locates: Vec::new(),
        tick_rate: Default::default(),
    };
    let level = minecraft_survival::level::Level::new(Dimension::Overworld, terrain);
    Ok((loaded, minecraft_survival::server::Server { globals, levels: std::collections::BTreeMap::from([(Dimension::Overworld, level)]), opening: Default::default(), players: std::collections::BTreeMap::new(), pending_batch: None }))
}

pub(crate) fn load_look(packs: PackStack) -> Result<Loaded, String> {
    let build = minecraft_terrain::mesh::build(&HandcraftedScene::default(), &packs)
        .map_err(|e| e.to_string())?;
    let celestial = Arc::new(celestial_image(&packs).map_err(|e| e.to_string())?);
    let cloud_mask = CloudMask::from_pack(&packs).ok();
    let crack_texture = Arc::new(crate::minecraft_mining::crack_strip(&packs).map_err(|e| e.to_string())?);
    let glint_texture = minecraft_terrain::glint::glint_texture(&packs).ok().flatten().map(Arc::new);
    let armor_glint_texture = minecraft_terrain::glint::armor_glint_texture(&packs).ok().flatten().map(Arc::new);
    let end_look = Arc::new(crate::minecraft_end_look::EndLook::load(&packs));
    Ok(Loaded {
        packs,
        atlas: build.atlas,
        celestial,
        cloud_mask,
        crack_texture,
        glint_texture,
        armor_glint_texture,
        end_look,
    })
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
pub(crate) fn show_look(view: &mut MinecraftWorldView, world: &Loaded) {
    view.atlas = Some(world.atlas.clone());
    view.atlas_patches.clear();
    view.celestial = Some(world.celestial.clone());
    view.crack_texture = Some(world.crack_texture.clone());
    view.glint_texture = world.glint_texture.clone();
    view.armor_glint_texture = world.armor_glint_texture.clone();
    view.end_look.look = Some(world.end_look.clone());
}

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

pub(crate) fn fire_frames(packs: &PackStack) -> Option<(Image, u32)> {
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

pub(crate) fn minecraft_yaw(mw2_yaw: f32) -> f32 {
    let yaw_rad = mw2_yaw.to_radians();
    (-yaw_rad.cos()).atan2(-yaw_rad.sin()).to_degrees()
}

fn show_chat(chat: &mut frame::McChat, inventory_ui: &crate::minecraft_inventory::InventoryUi, packs: &PackStack, delivered: &[minecraft_commands::Clientbound]) {
    match inventory_ui.language() {
        Some(language) => crate::minecraft_chat::show(chat, delivered, language),
        None => crate::minecraft_chat::show(chat, delivered, &minecraft_terrain::item_icons::language(packs).unwrap_or_else(|_| HashMap::new())),
    }
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
    (hud_input, settings, role): (Option<Res<frame::HudInputView>>, Option<Res<frame::GameSettings>>, Res<frame::RuntimeRole>),
    real_time: Res<Time<Real>>,
    mc_camera: Option<Res<frame::McCamera>>,
    mut body_gear: ResMut<crate::anim::minecraft_gear::McBodyGear>,
) {
    if *role == frame::RuntimeRole::Client {
        for _ in installed.read() {}
        for _ in torn_down.read() {}
        return;
    }
    let pad = active_pad.and_then(|active| active.0).and_then(|entity| gamepads.get(entity).ok());
    for _ in torn_down.read() {
        stop(&mut runtime, &mut view);
        ui.chat.clear_messages();
        ui.credits = None;
    }
    let now_ms = real_time.elapsed().as_millis() as u64;
    for match_ in installed.read() {
        stop(&mut runtime, &mut view);
        ui.chat.clear_messages();
        ui.credits = None;
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
    ui.loading_world = runtime.loading.is_some() || (runtime.world.is_some() && (runtime.spawn_gate.holding(now_ms) || awaiting_level(&runtime)));
    if runtime.server.is_none() {
        body_gear.clear();
    }
    if runtime.switch_screen.is_some() && !local_switching(&runtime) && !runtime.spawn_gate.holding(now_ms) {
        runtime.switch_screen = None;
    }
    let aspect = windows.single().map(|w| w.width() / w.height().max(1.0)).unwrap_or(16.0 / 9.0);
    let game_time = runtime.server.as_ref().map_or(0.0, |server| server.globals.day.ticks);
    let reason = runtime.switch_screen;
    ui.level_loading = runtime.loading_backgrounds.as_mut().and_then(|backgrounds| backgrounds.frame(reason, game_time, time.delta_secs_f64(), aspect, &mut images));
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
                let minecraft_survival::server::Server { globals, levels, players, .. } = &mut server;
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
                show_look(&mut view, &world);
                runtime.atlas_animation = AtlasAnimation::new(&world.atlas);
                runtime.animation_clock = 0.0;
                runtime.mining = Default::default();
                runtime.minimap = Default::default();
                runtime.sounds = Some(crate::minecraft_sounds::Sounds::load(&world.packs));
                globals.data = minecraft_survival::data::GameData::load(assets::minecraft_map::root().as_deref(), data_jar());
                minecraft_survival::server::start_level(globals, level);
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
                let chosen_mode = std::env::var("IW4L_MINECRAFT_GAMEMODE").ok();
                let kit = std::env::var("IW4L_MINECRAFT_KIT").ok();
                let joining = minecraft_survival::server::Joining { profile: &profile, spawn: view.origin, chosen_mode: chosen_mode.as_deref(), kit: kit.as_deref(), carried: runtime.carried_survival.take() };
                let mut player = minecraft_survival::server::join(globals.save.as_ref(), &globals.data.recipes, &globals.game_rules, joining);
                match minecraft_survival::server::debug_dimension(std::env::var("IW4L_MINECRAFT_DIMENSION").as_deref().unwrap_or("")) {
                    Ok(Some(dimension)) => {
                        minecraft_survival::server::start_in(&mut player, dimension, view.origin);
                        diag::info!(World, "Minecraft player starts in {} from IW4L_MINECRAFT_DIMENSION", dimension.dimension_type());
                    }
                    Ok(None) => {}
                    Err(error) => diag::warn!(World, "{error}"),
                }
                let uuid = player.uuid.clone();
                if let Some(entities) = level.entities.as_mut() {
                    entities.set_players(&[(u64::from(player.id.0), uuid.clone())]);
                }
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
                ui.portal_frames = crate::minecraft_portal::frames(&world.packs, &mut images);
                runtime.loading_backgrounds = Some(crate::minecraft_loading::LoadingBackgrounds::load(&world.packs, &mut images));
                runtime.server = Some(server);
                runtime.local = Some(uuid);
                runtime.world = Some(world);
                runtime.shapes.clear();
                runtime.shape_ids.clear();
                runtime.spawn_gate.start(now_ms);
                runtime.shown = Some(Dimension::Overworld);
                runtime.origins = HashMap::from([(Dimension::Overworld, view.origin)]);
            }
            Err(error) => {
                diag::warn!(World, "Minecraft world failed to load: {error}");
                view.active = false;
            }
        }
    }

    let failed = follow_levels(&mut runtime, &mut view, now_ms);
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
        minimap,
        fire_frames,
        atlas_animation,
        animation_clock,
        spawn_gate,
        shown,
        switch_screen,
        refused_air,
        origins,
        relayed,
        queues,
        ..
    } = &mut *runtime;
    let (Some(world), Some(server), Some(local_uuid)) = (world.as_mut(), server.as_mut(), local_uuid.as_ref()) else {
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    };
    let ticking: Vec<Dimension> = server.players.get(local_uuid).filter(|player| player.switching.is_none()).map(|player| player.dimension).into_iter().collect();
    let planned = server.plan_ticks(std::time::Instant::now(), &ticking);
    let minecraft_survival::server::Server { globals, levels, players, pending_batch, .. } = server;
    if std::mem::take(&mut globals.autosave_due) {
        minecraft_survival::server::autosave(globals, levels, players);
    }
    let others = minecraft_survival::level::player_blocks(players.iter().filter(|(uuid, _)| *uuid != local_uuid).map(|(_, player)| player));
    let clients: Vec<u32> = players.values().filter(|player| &player.uuid != local_uuid).map(|player| player.id.0).chain([local.0.0]).collect();
    queues.drain(|client| clients.contains(&client));
    let Some(player) = players.get_mut(local_uuid) else {
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    };
    if player.dead {
        player.held = None;
    }
    crate::minecraft_end::credits(player, &mut ui, &world.packs, &mut images);
    if player.switching.is_some() {
        for (dimension, error) in &failed {
            show_chat(&mut ui.chat, inventory_ui, &world.packs, &[minecraft_survival::switch::failed_message(*dimension, error)]);
        }
    }
    let mut begun_lines = Vec::new();
    let begun = crate::minecraft_players::begin(player, levels, globals, &mut authority.0, local.0, origin, presented.player(local.0), in_remote_killstreak(&presented, local.0), Some(spawn_gate), now_ms, time.delta_secs_f64(), sounds, &world.packs, &mut begun_lines);
    if !begun_lines.is_empty() {
        show_chat(&mut ui.chat, inventory_ui, &world.packs, &begun_lines);
    }
    match begun {
        crate::minecraft_players::Begun::Live => {}
        crate::minecraft_players::Begun::Left(switch) => {
            *switch_screen = Some(switch.reason);
            if let Some(level) = levels.get_mut(&switch.from) {
                leave_level(level, player, &mut ui, inventory_ui);
            }
            sim::voxel::switch_level(origin);
            *shown = None;
            ui.active = false;
            ui.inventory_open = false;
            puppet.active = false;
            return;
        }
        crate::minecraft_players::Begun::Arriving { at, standing } => {
            if let Some(level) = levels.get_mut(&player.dimension) {
                let minecraft_survival::level::Level { entities, containers, terrain, dimension, .. } = level;
                stream_around_spawn(world, terrain, &globals.registries, shapes, shape_ids, entities, containers, light, spawn_gate, &mut view, at, &minecraft_survival::level::blocks_in(&others, *dimension), aspect, now_ms);
            }
            if standing && player.arrival.is_none() && !spawn_gate.waiting() && !minecraft_survival::the_end::holding_for_credits(player) {
                player.switching = None;
                diag::info!(World, "Minecraft player arrived in {} at {:?}", player.dimension.dimension_type(), at);
            }
            ui.active = false;
            ui.inventory_open = false;
            puppet.active = false;
            return;
        }
        crate::minecraft_players::Begun::Hidden => {
            ui.active = false;
            ui.inventory_open = false;
            puppet.active = false;
            return;
        }
    }
    let arriving_at = player.arrival.map(|place| place.feet);
    let host_alive = presented.player(local.0).is_some_and(|ps| ps.pm_type == 0);
    let host_pose = presented
        .player(local.0)
        .filter(|_| host_alive)
        .map(|ps| minecraft_commands::Pose { feet: sim::voxel::to_block(origin, ps.origin), yaw: minecraft_yaw(ps.viewangles[1]), pitch: ps.viewangles[0] })
        .or_else(|| player.survival.last_feet().map(|feet| minecraft_commands::Pose { feet, yaw: 0.0, pitch: 0.0 }));
    let mut presence = vec![(local_uuid.clone(), minecraft_survival::commands::Presence { pose: host_pose, alive: host_alive })];
    presence.extend(friends_presence(players, &authority.0, origins, local_uuid));
    let lines = minecraft_survival::commands::take_lines(&mut ui.chat.sent);
    let minecraft_survival::commands::Handled { delivered, teleports } = minecraft_survival::commands::run(globals, levels, players, local_uuid, lines, presence);
    let delivered_here = minecraft_survival::commands::reaching(&globals.chat, &delivered, minecraft_survival::commands::HOST);
    relayed.delivered.extend(delivered);
    relayed.teleports.extend(teleports.iter().filter(|(uuid, _)| uuid != local_uuid).cloned());
    let delivered = delivered_here;
    ui.chat.command_level = players.get(local_uuid).map_or(0, |player| globals.chat.level(player.id));
    crate::minecraft_end::tick_fight(levels, players, globals, f64::from(planned.normal_count()) * minecraft_survival::the_end::dragon_fight::TICK_SECONDS, local_uuid, &mut ui);
    let mut friends = Vec::new();
    let mut host = None;
    for (uuid, each) in players.iter_mut() {
        if uuid == local_uuid {
            host = Some(each);
        } else {
            friends.push(each);
        }
    }
    let Some(player) = host else {
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    };
    let host_id = u64::from(player.id.0);
    let friend_ids: Vec<u64> = friends.iter().filter(|friend| friend.dimension == player.dimension).map(|friend| u64::from(friend.id.0)).collect();
    let friend_builds: Vec<u64> = friends.iter().filter(|friend| friend.dimension == player.dimension && friend.mode.may_build()).map(|friend| u64::from(friend.id.0)).collect();
    crate::minecraft_portal::step(player, levels, globals, presented.player(local.0).filter(|ps| ps.pm_type == 0).map(|ps| sim::voxel::to_block(origin, ps.origin)), time.delta_secs_f64(), sounds.as_mut(), &world.packs, &mut ui);
    crate::minecraft_end::pearls_through(player, levels);
    let credit = |attacker: Option<u32>| attacker.and_then(|client| if client == local.0.0 { Some(host_id) } else { friend_ids.contains(&u64::from(client)).then_some(u64::from(client)) });
    if player.switching.is_some_and(|switch| !switch.left) {
        show_chat(&mut ui.chat, inventory_ui, &world.packs, &delivered);
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    }
    let Some(level) = levels.get_mut(&player.dimension) else {
        ui.active = false;
        ui.inventory_open = false;
        puppet.active = false;
        return;
    };
    let minecraft_survival::level::Level { entities, containers, terrain, dimension, container_ticks, .. } = &mut *level;
    minecraft_terrain::cow_render::set_nether_entity_lighting(terrain.rules.nether_cardinal_light);
    if !delivered.is_empty() {
        show_chat(&mut ui.chat, inventory_ui, &world.packs, &delivered);
    }
    crate::minecraft_respawn::fade_overlay(&mut ui, time.delta_secs_f64());
    animate_atlas(&mut view, atlas_animation, animation_clock, time.delta_secs_f64(), &world.atlas);
    let aspect = windows
        .single()
        .map(|w| w.width() / w.height().max(1.0))
        .unwrap_or(16.0 / 9.0);
    let Some(ps) = presented.player(local.0) else {
        ui.active = false;
        puppet.active = false;
        stream_around_spawn(world, terrain, &globals.registries, shapes, shape_ids, entities, containers, light, spawn_gate, &mut view, arriving_at.unwrap_or(origin), &minecraft_survival::level::blocks_in(&others, *dimension), aspect, now_ms);
        return;
    };

    let crate::minecraft_players::Life { alive, dead, totem_popped, killers } = crate::minecraft_players::life(player, globals, terrain, &mut authority.0, local.0, ps, queues);
    ui.game_mode = ui_game_mode(player.mode.mode);
    for killer in killers {
        let seq = ui.killed_by.as_ref().map_or(0, |(seq, _)| seq.wrapping_add(1));
        ui.killed_by = Some((seq, killer));
    }

    let feet = sim::voxel::to_block(origin, ps.origin);
    let block = (
        feet[0].floor() as i32,
        feet[1].floor() as i32,
        feet[2].floor() as i32,
    );
    let here: Vec<(i32, i32, i32)> = std::iter::once(block).chain(minecraft_survival::level::blocks_in(&others, *dimension)).collect();
    let streamed = minecraft_survival::level::stream_tick(terrain, entities, containers, &here, Some(block));
    apply_streamed(&globals.registries, shapes, shape_ids, streamed);

    // Minecraft's yaw: 0 facing +Z (map -Y), turning towards -X.
    let mc_yaw = minecraft_yaw(ps.viewangles[1]);
    inventory_ui.dials.step(time.delta_secs_f64(), &world.packs, &minecraft_terrain::dials::DialContext { dimension: *dimension, feet, yaw: mc_yaw, spawn: Some((minecraft_survival::save::spawn_dimension(&globals.spawn), globals.spawn.pos)), last_death: player.last_death.map(|death| (death.dimension, death.pos)), sun_angle: terrain.environment.sun_angle_degrees(1.0) });
    let aim = {
        let mut aim = minecraftoss_player::Player::new(glam::DVec3::from_array(feet));
        aim.yaw = f64::from(mc_yaw);
        aim.pitch = f64::from(ps.viewangles[0]);
        aim.crouching = ps.pm_flags & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE) != 0;
        aim.target(&minecraft_terrain::doors::Outlined(&terrain.scene), player.mode.block_reach())
    };
    minecraft_survival::commands::refresh_hints(&mut ui.chat.hints, &globals.chat, minecraft_survival::switch::DIMENSIONS.into_iter(), aim.map(|hit| (hit.point.to_array(), [hit.pos.0, hit.pos.1, hit.pos.2])));
    if ui.chat.language.is_empty() && let Some(language) = inventory_ui.language() {
        ui.chat.language = std::sync::Arc::new(language.clone());
    }
    let all_events = sim::voxel::take_events();
    let mut moved = crate::minecraft_players::movement(player, globals, terrain, entities, (shapes, shape_ids), &authority.0, local.0, origin, ps, alive, time.delta_secs_f64(), sounds, &world.packs);
    ui.holding_item = moved.holding_item;
    ui.empty_hand = moved.empty_hand;
    ui.cooldowns = std::mem::take(&mut moved.cooldowns);
    if let Some(entities) = entities.as_mut() {
        let (writes, brewed) = minecraft_survival::level::tick_containers(containers, entities, terrain, std::mem::take(container_ticks));
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
    let hands = crate::minecraft_input::host_hands(&ui, hud_input.as_deref(), &buttons, pad);
    let actor = crate::minecraft_players::actor(ps, origin, alive);
    let mut body = crate::minecraft_players::MwBody { authority: &mut authority.0, client: local.0, ps, registries: &globals.registries, shapes: &mut *shapes, shape_ids: &mut *shape_ids };
    let minecraft_survival::actions::Acted { open_table, mut open_block, mut open_bench, hand_mined, creative_broken, hand_broken, effects } = minecraft_survival::actions::act(player, level, globals, &mut body, &actor, hands, &moved, light.as_ref(), time.elapsed_secs_f64());
    let player_mode = player.mode;
    let minecraft_survival::level::Level { entities, terrain, shots, .. } = &mut *level;
    crate::minecraft_input::play(effects, &mut crate::minecraft_input::Presenter { sounds: sounds.as_mut(), packs: &world.packs, atlas: &world.atlas, mining: &mut *mining, scene: &terrain.scene, origin, ui: &mut ui, inventory_ui: &*inventory_ui });

    // Shots and explosions from the authoritative game: bullets that met a
    // mob hurt it, the rest mine.
    let (mob_shots, events): (Vec<_>, Vec<_>) = all_events
        .into_iter()
        .partition(|event| matches!(event, sim::voxel::VoxelEvent::MobShot { .. } | sim::voxel::VoxelEvent::MobBlast { .. }));
    let events: Vec<_> = events
        .into_iter()
        .filter(|event| match *event {
            sim::voxel::VoxelEvent::Shot { attacker, .. } | sim::voxel::VoxelEvent::Ray { attacker, .. } => match credit(attacker) {
                Some(id) if id == host_id => player_mode.may_build(),
                Some(id) => friend_builds.contains(&id),
                None => false,
            },
            _ => true,
        })
        .filter(|event| match *event {
            sim::voxel::VoxelEvent::Shot { block, .. } => {
                let pos = (block[0], block[1], block[2]);
                let egg = minecraft_terrain::scene::Scene::block(&terrain.scene, pos).is_some_and(minecraft_terrain::dragon_egg::is_egg);
                if let Some(entities) = entities.as_mut().filter(|_| egg) {
                    entities.attack_block(&terrain.scene, pos);
                }
                !egg
            }
            _ => true,
        })
        .collect();
    if let Some(entities) = entities.as_mut() {
        for shot in mob_shots {
            match shot {
                sim::voxel::VoxelEvent::MobShot { key, damage, from, attacker, .. } => {
                    if let (Some(id), Some(client)) = (credit(attacker), attacker) {
                        let yaw = if id == host_id { mc_yaw } else { authority.0.player(sim::ClientId(client)).map_or(mc_yaw, |ps| minecraft_yaw(ps.viewangles[1])) };
                        entities.shoot(id, key, damage, from, yaw);
                    }
                }
                sim::voxel::VoxelEvent::MobBlast { key, damage, center, impact, attacker, slow_ticks, .. } => {
                    entities.blast(key, damage, center, impact, credit(attacker), slow_ticks);
                }
                _ => {}
            }
        }
        for event in &events {
            if let sim::voxel::VoxelEvent::Explosion { center, power, .. } = *event {
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
    let mut broken = Vec::new();
    let mut friends_broken: Vec<(u64, Vec<crate::minecraft_mining::Mined>)> = Vec::new();
    for (mined, attacker) in mining.apply(events, shots, &mut refs, time.elapsed_secs_f64()) {
        match credit(attacker).filter(|&id| id != host_id) {
            Some(id) => match friends_broken.iter_mut().find(|(friend, _)| *friend == id) {
                Some((_, by_friend)) => by_friend.push(mined),
                None => friends_broken.push((id, vec![mined])),
            },
            None => broken.push(mined),
        }
    }
    broken.extend(mining.break_now(&creative_broken, shots, &mut refs));
    let dug = mining.break_now(&hand_broken, shots, &mut refs);
    if let Some(sounds) = sounds.as_mut() {
        for (pos, block, blast) in broken.iter().chain(&dug).chain(friends_broken.iter().flat_map(|(_, mined)| mined)) {
            if blast.is_some() {
                continue;
            }
            if let Some(kind) = terrain.scene.sound_type(block) {
                let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                sounds.play(&world.packs, &kind.break_sound, Some(at(centre)), (kind.volume + 1.0) / 2.0, kind.pitch * 0.8);
            }
        }
    }
    let harvest = minecraft_survival::actions::harvest(player, level, globals, broken, dug, &hand_mined);
    for (id, mined) in friends_broken {
        if let Some(friend) = friends.iter_mut().find(|friend| u64::from(friend.id.0) == id) {
            let _ = minecraft_survival::actions::harvest(friend, level, globals, mined, Vec::new(), &[]);
        }
    }
    let minecraft_survival::player::Player { survival, mode: player_mode, death_watch, inventory, selected, hand, .. } = &mut *player;
    let minecraft_survival::level::Level { entities, containers, terrain, openers, dimension, shots, .. } = &mut *level;
    if let Some(entities) = entities.as_mut() {
        entities.broken_items.extend(harvest.worn_out);
    }
    crate::minecraft_input::play(harvest.effects, &mut crate::minecraft_input::Presenter { sounds: sounds.as_mut(), packs: &world.packs, atlas: &world.atlas, mining: &mut *mining, scene: &terrain.scene, origin, ui: &mut ui, inventory_ui: &*inventory_ui });

    let eye = sim::voxel::to_block(origin, [ps.origin[0], ps.origin[1], ps.origin[2] + ps.view_height_current]);
    death_watch.stood(minecraft_survival::death::DeathSpot { feet, eye });
    let forward = view_forward(ps.viewangles);
    let mc_detached = mc_camera.as_ref().is_some_and(|c| c.detached);
    let drawn_from = cameras.iter().next().filter(|_| skate.active || mc_detached);
    let camera = cull_camera(drawn_from, origin, eye, forward, aspect);
    let lights = terrain_frame(terrain, world, &mut view, &camera);
    observe_spawn(terrain, spawn_gate, feet, camera.position, now_ms);
    let Some(light) = light.as_mut() else {
        return;
    };
    for (chunk, column) in lights {
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
        let viewer = minecraft_survival::entities::PlayerView {
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
        let batch = pending_batch.take().unwrap_or(minecraft_survival::tick_rate::TickBatch { normal: Vec::new(), overload: None });
        let mut present = vec![(host_id, local_uuid.clone())];
        let mut actors = vec![minecraft_survival::entities::TickActor { id: host_id, view: viewer, inventory: &mut *inventory, selected: *selected }];
        for friend in friends.iter_mut().filter(|friend| friend.dimension == *dimension) {
            if let Some((actor, uuid)) = crate::minecraft_players::tick_actor(friend, &authority.0, origin) {
                present.push((actor.id, uuid));
                actors.push(actor);
            }
        }
        entities.set_players(&present);
        let changes = entities.tick(&mut actors, dt, &batch, globals.tick_rate.client_timing(), globals.clock_ticks(), bright_outside);
        drop(actors);
        entities.present_level_events(&terrain.scene);
        globals.weather = entities.weather_data.or(globals.weather);
        let summoned = std::mem::take(&mut entities.command_summons);
        if !summoned.is_empty() {
            let answers = minecraft_survival::commands::summon_answers(&globals.chat, &globals.game_rules, summoned);
            show_chat(&mut ui.chat, inventory_ui, &world.packs, &minecraft_survival::commands::reaching(&globals.chat, &answers, minecraft_survival::commands::HOST));
            relayed.delivered.extend(answers);
        }
        let mob_ticks = (entities.client_ticks() - ticks_before) as u32;
        for _ in 0..mob_ticks {
            let rain = entities.rain_tick(&terrain.scene, eye);
            for sound in entities.animate_tick(&terrain.scene, feet).into_iter().chain(rain) {
                if let Some(sounds) = sounds.as_mut() {
                    sounds.play(&world.packs, sound.event, Some(at(sound.at)), sound.volume, sound.pitch);
                }
            }
            entities.campfire_tick(&terrain.scene, |pos| containers.campfire_food(pos));
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
                if !minecraft_terrain::scene::Scene::block(&terrain.scene, pos).is_some_and(minecraft_terrain::openers::Openers::counted) {
                    openers.removed(pos);
                }
                if !minecraft_terrain::scene::Scene::block(&terrain.scene, pos).is_some_and(|b| minecraft_terrain::containers::Station::of(b).is_some() || minecraft_terrain::campfire::is_campfire(b)) {
                    minecraft_survival::level::spill(containers, entities, pos);
                }
            }
            terrain.stream.record_edits(&terrain.scene, &positions);
            terrain.stream.mark_edited(&terrain.scene, &positions);
        }
        for (item, at, velocities) in std::mem::take(&mut entities.item_crumbs) {
            if let Some(texture) = minecraft_terrain::pack::ResourceId::parse(&item)
                .ok()
                .and_then(|id| minecraft_terrain::model::item_particle_texture(&world.packs, &id).ok().flatten())
                .filter(|texture| world.atlas.contains(texture))
            {
                mining.item_crumbs(&world.packs, at.as_vec3(), &velocities, &texture);
            }
        }
        let potion_crumbs = std::mem::take(&mut entities.potion_crumbs);
        if !potion_crumbs.is_empty()
            && let Some(texture) = minecraft_terrain::pack::ResourceId::parse(minecraft_terrain::thrown::SPLASH_POTION)
                .ok()
                .and_then(|id| minecraft_terrain::model::item_particle_texture(&world.packs, &id).ok().flatten())
                .filter(|texture| world.atlas.contains(texture))
        {
            for center in potion_crumbs {
                mining.potion_crumbs(&world.packs, center.as_vec3(), &texture);
            }
        }
        for (pos, block) in std::mem::take(&mut entities.destroyed) {
            mining.burst(&world.packs, &terrain.scene, &world.atlas, pos, &block);
            if let (Some(sounds), Some(kind)) = (sounds.as_mut(), terrain.scene.sound_type(&block)) {
                let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                sounds.play(&world.packs, &kind.break_sound, Some(at(centre)), (kind.volume + 1.0) / 2.0, kind.pitch * 0.8);
            }
        }
        crate::minecraft_players::struck(player, entities, &mut authority.0, local.0, origin, ps, alive, &teleports, sounds, &world.packs);
        for item in std::mem::take(&mut entities.broken_items) {
            mining.break_burst(&world.packs, &world.atlas, glam::DVec3::from_array(eye).as_vec3(), mc_yaw, ps.viewangles[0], &item);
        }
        if let Some(told) = crate::minecraft_players::settle(player, entities, globals, &mut authority.0, local.0, origin, alive, totem_popped, inventory_ui.language(), sounds, &world.packs) {
            crate::minecraft_chat::show(&mut ui.chat, std::slice::from_ref(&told), &HashMap::new());
            relayed.delivered.push(minecraft_commands::Outgoing { to: minecraft_commands::Recipient::All, message: told });
        }
        let minecraft_survival::player::Player { survival, mode: player_mode, death_watch, inventory, selected, hand, ender, .. } = &mut *player;
        ui.vitals = alive.then(|| mc_vitals(survival.vitals()));
        ui.fire_frames = fire_frames.clone();
        // The inventory: MW2 guns as items, the HUD's clicks, the hotbar's
        // gun, and what the HUD shows.
        let owned = crate::minecraft_players::owned_weapons(ps, &authority.0);
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
        let operator = ui.chat.command_level >= 2 && settings.as_deref().is_some_and(|settings| settings.minecraft_operator_items);
        inventory_ui.creative.prepare(!player_mode.consumes_items(), operator);
        inventory_ui.creative.set_item_tags(&globals.chat.data().item_tags);
        let known = |id: &str| globals.chat.registries().items.iter().any(|item| item == id);
        let creative_thrown = inventory_ui.creative.apply(&mut ui, player_mode, inventory, &known);
        inventory_ui.creative.hotbar_key(&mut ui, player_mode, inventory, &known);
        let thrown: Vec<_> = creative_thrown.into_iter().chain(containers.with_ender(ender, |containers| inventory_ui.apply_input(&mut ui, inventory, selected, containers, &mut || roll))).collect();
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
        let spectator = player_mode.mode == minecraftoss_player::GameMode::Spectator;
        let mut opener_events = Vec::new();
        for (opened, open) in [(was_open.clone().filter(|o| now_open.as_ref() != Some(o)), false), (now_open.filter(|o| was_open.as_ref() != Some(o)), true)] {
            let Some(opened) = opened else { continue };
            opener_events.extend(minecraft_survival::level::toggle_openers(openers, &terrain.scene, &opened.cells, local_uuid, open, spectator));
        }
        if entities.ticks() % minecraft_terrain::openers::RECHECK_TICKS == 0 {
            let key = minecraft_survival::level::opener_key(local_uuid);
            let open_cells = inventory_ui.block().map_or_else(Vec::new, |opened| opened.cells.clone());
            opener_events.extend(openers.recheck(|player, pos| player != key || open_cells.contains(&pos)));
        }
        for event in opener_events {
            let outcome = minecraft_survival::level::opener_outcome(&terrain.scene, &event);
            let (cell, id, count) = outcome.block_event;
            entities.lids.block_event(&terrain.scene, cell, id, count);
            if let Some(sound) = outcome.sound {
                let pitch = minecraft_terrain::openers::pitch(entities.random_float());
                if let Some(sounds) = sounds.as_mut() {
                    sounds.play(&world.packs, sound.event, Some(at(sound.position)), sound.volume, pitch);
                }
            }
            if let Some((cell, state)) = outcome.state {
                write_blocks(terrain, &globals.registries, shapes, shape_ids, entities, vec![(cell, Some(state))]);
            }
        }
        let found = entities.take_experience(host_id).saturating_add(inventory_ui.take_experience()) as f32;
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
        crate::minecraft_players::drop_on_death(death_watch, inventory, hand, entities);
        crate::minecraft_inventory::throw(&mut entities.world_items, thrown, &thrower);
        ui.weapon_request = inventory_ui.weapon_request(inventory, selected, ps.weapon).filter(|_| player_mode.interacts());
        let refused = ui.weapon_request.filter(|&weapon| sky_bits(&terrain.environment) != 0 && sim::voxel::needs_sky(authority.0.weapon_script_name(weapon)));
        if refused.is_some() && *refused_air != refused {
            show_chat(&mut ui.chat, inventory_ui, &world.packs, &[minecraft_survival::switch::air_support_unavailable()]);
        }
        *refused_air = refused;
        if refused.is_some() {
            ui.weapon_request = None;
        }
        inventory_ui.animation_tick = atlas_animation.ticks();
        containers.with_ender(ender, |containers| inventory_ui.publish(&mut ui, inventory, *selected, &world.packs, &mut images, containers));

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
            && !moved.holding
            && offhand_stack.as_ref().is_some_and(|stack| {
                minecraft_survival::offhand::has_use(stack, placeable(stack), &survival.status.food, player_mode.mode) || (containers.cooks_on_campfire(stack, &entities.recipes) && aims_at_campfire())
            });
        ui.offhand_busy = alive && !moved.holding && (survival.eating().is_some() || hand.blocking.is_some() || hand.charging.is_some());
        ui.using_item = alive && (survival.eating().is_some() || hand.blocking.is_some() || hand.charging.is_some());
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
                let charging = hand.charging.filter(|charging| charging.slot == slot);
                let using = hand.blocking.is_some_and(|blocking| blocking.slot == slot) || charging.is_some();
                let display = minecraft_terrain::pack::ResourceId::parse(&stack.id)
                    .ok()
                    .and_then(|id| minecraft_terrain::model::item_hand_transform(&world.packs, &id, arm < 0.0, using).ok())
                    .unwrap_or(glam::Mat4::IDENTITY);
                let display = match charging {
                    Some(charging) => crate::minecraft_hand::trident_charge_transform(charging.ticks as f32 + (hand.clock / TICK_SECONDS) as f32, arm) * display,
                    None => display,
                };
                let eat = survival
                    .eating()
                    .filter(|eating| eating.slot == slot)
                    .map(|eating| crate::minecraft_hand::eat_transform(eating.remaining_ticks() as f32, (hand.clock / TICK_SECONDS) as f32, eating.info.consume_ticks, arm));
                let swung = if using || eat.is_some() || hand.swing_offhand != (slot == minecraft_survival::offhand::OFFHAND) { 0.0 } else { swing };
                let glint = minecraft_terrain::glint::has_glint(&stack);
                posed.push((eat.unwrap_or(glam::Mat4::IDENTITY) * crate::minecraft_hand::item_pose(display, swung, 0.0, arm), inventory_ui.dials.key(&stack.id).to_owned(), glint));
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

        sim::voxel::set_mob_boxes(entities.shot_boxes());
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
        for (pos, apart) in entities.lids.take_apart_changes() {
            terrain.stream.draw_apart(pos, apart);
        }
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
    let minecraft_survival::player::Player { survival, mode: player_mode, inventory, selected, hand, .. } = &mut *player;
    if alive {
        match minecraft_survival::gear::body_gear(inventory, *selected, player_mode) {
            Some(gear) => body_gear.set(local.0.0, gear),
            None => body_gear.remove(local.0.0),
        }
    } else if dead {
        body_gear.died(local.0.0);
    } else {
        body_gear.remove(local.0.0);
    }
    let mut cracks = minecraft_survival::level::shot_cracks(shots);
    cracks.extend(hand.dig.crack());
    cracks.extend(friends.iter().filter(|friend| friend.dimension == *dimension).filter_map(|friend| friend.hand.dig.crack()));
    view.cracks = mining.crack_mesh(&terrain.scene, &terrain.stream.states, &world.packs, &cracks);
    let lava = (alive && sim::voxel::eye_in_lava(ps.origin, ps.view_height_current)).then(|| {
        let fire_resistant = survival.status.effect(minecraftoss_player::survival::EffectKind::FireResistance).is_some();
        (player_mode.mode == minecraftoss_player::GameMode::Spectator, fire_resistant)
    });
    let weather = entities.as_ref().map_or_else(minecraft_terrain::server::WeatherView::default, |e| e.weather);
    let inputs = SkyInputs {
        eye,
        forward,
        aspect,
        dt,
        clock_ticks: globals.default_clock_ticks(*dimension),
        game_time: globals.day.ticks,
        rain_level: weather.rain_level,
        thunder_level: weather.thunder_level,
        lava,
        world_fog: ui.boss_bars.iter().any(|bar| bar.world_fog),
    };
    present_sky(&mut view, terrain, &globals.registries, light, (environment_accumulator, environment_primed), &inputs);
    present_clouds(&mut view, world, cloud_center, eye, globals.day.ticks);
    present_light_volume(&mut view, light, block, light_volume_at, light_volume_age);
}

#[derive(Clone, Copy)]
pub(crate) struct SkyInputs {
    pub(crate) eye: [f64; 3],
    pub(crate) forward: glam::Vec3,
    pub(crate) aspect: f32,
    pub(crate) dt: f64,
    pub(crate) clock_ticks: i64,
    pub(crate) game_time: f64,
    pub(crate) rain_level: f32,
    pub(crate) thunder_level: f32,
    pub(crate) lava: Option<(bool, bool)>,
    pub(crate) world_fog: bool,
}

pub(crate) fn present_sky(view: &mut MinecraftWorldView, terrain: &mut minecraft_survival::level::Terrain, registries: &Registries, light: &SkyLight, (accumulator, primed): (&mut f64, &mut bool), inputs: &SkyInputs) {
    let SkyInputs { eye, forward, aspect, dt, clock_ticks, game_time, rain_level, thunder_level, lava, world_fog } = *inputs;
    let eye_block = (eye[0].floor() as i32, eye[1].floor() as i32, eye[2].floor() as i32);
    view.eye_light = [
        f32::from(light.get(eye_block)),
        f32::from(light.get_block(eye_block)),
    ];
    let rains_here = terrain
        .scene
        .noise_biome((eye_block.0 >> 2, eye_block.1 >> 2, eye_block.2 >> 2))
        .is_some_and(|biome| registries.biomes.get(biome).has_precipitation);
    terrain
        .environment
        .update_rain_fog(rain_level, light.get(eye_block), rains_here, (dt * 20.0) as f32);
    *accumulator += dt;
    if !*primed || *accumulator >= TICK_SECONDS {
        *accumulator = (*accumulator % TICK_SECONDS).min(TICK_SECONDS);
        let scene = &terrain.scene;
        terrain.environment.tick(
            clock_ticks,
            rain_level,
            thunder_level,
            eye,
            |x, y, z| scene.noise_biome((x, y, z)).map_or(0, |id| id.0),
            !*primed,
        );
        *primed = true;
    }
    let partial_tick = (*accumulator / TICK_SECONDS).clamp(0.0, 1.0) as f32;
    let sky = terrain.environment.sky_state(&View {
        partial_tick,
        forward,
        camera_y: eye[1] as f32,
        render_distance: VIEW_DISTANCE as u32,
        rain_level,
        thunder_level,
    });
    let render_distance = VIEW_DISTANCE as f32 * 16.0;
    let sky = match lava {
        Some((spectator, fire_resistant)) => minecraft_terrain::lava_fog::in_lava(sky, spectator, fire_resistant, render_distance),
        None => sky,
    };
    let right = forward.cross(glam::Vec3::Y).normalize_or(glam::Vec3::X);
    let up = right.cross(forward).normalize_or(glam::Vec3::Y);
    let put = |v: glam::Vec3| [v.x, v.y, v.z, 0.0];
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
        [aspect, sky.cloud_alpha, sky.star_brightness, sky.star_angle],
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
                _ if world_fog => 2.0,
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
    view.end_look.portal_time = crate::minecraft_end_look::portal_time(game_time);
}

pub(crate) fn present_clouds(view: &mut MinecraftWorldView, world: &Loaded, cloud_center: &mut Option<(i32, i32)>, eye: [f64; 3], game_time: f64) {
    // Clouds, rebuilt when the camera crosses a cloud cell.
    if let Some(mask) = &world.cloud_mask {
        let center = mask.center(eye[0] as f32, eye[2] as f32, game_time);
        if *cloud_center != Some(center) {
            *cloud_center = Some(center);
            let mesh = mask.build(center, eye[1] as f32);
            view.clouds = Some(Arc::new((mesh.vertices, mesh.indices)));
        }
    }
}

pub(crate) fn present_light_volume(view: &mut MinecraftWorldView, light: &SkyLight, block: (i32, i32, i32), light_volume_at: &mut Option<[i32; 3]>, light_volume_age: &mut u32) {
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

pub(crate) fn view_forward(viewangles: [f32; 3]) -> glam::Vec3 {
    let (pitch, yaw) = (viewangles[0].to_radians(), viewangles[1].to_radians());
    let map_forward = [pitch.cos() * yaw.cos(), pitch.cos() * yaw.sin(), -pitch.sin()];
    glam::Vec3::new(map_forward[0], map_forward[2], -map_forward[1])
}

pub(crate) fn cull_camera(drawn_from: Option<&Transform>, origin: [f64; 3], eye: [f64; 3], forward: glam::Vec3, aspect: f32) -> CullCamera {
    // Culled from the camera that draws: the player's eye, or while
    // skating the Skate camera (a frame behind, so with room to spare).
    let skate_camera = drawn_from.map(|t| {
        let at = sim::voxel::to_block(origin, t.translation.to_array());
        let ahead = t.rotation * Vec3::NEG_Z;
        (
            glam::DVec3::new(at[0], at[1], at[2]),
            glam::Vec3::new(ahead.x, ahead.z, -ahead.y).normalize_or(forward),
        )
    });
    let (cull_at, cull_forward) = skate_camera.unwrap_or((glam::DVec3::new(eye[0], eye[1], eye[2]), forward));
    CullCamera {
        position: cull_at,
        forward: cull_forward,
        fov_degrees: if skate_camera.is_some() { 120.0 } else { 90.0 },
        aspect,
        yaw_degrees: (-cull_forward.x).atan2(cull_forward.z).to_degrees(),
        pitch_degrees: (-cull_forward.y).asin().to_degrees(),
    }
}

type ColumnLights = Vec<((i32, i32), Option<Arc<SkyLight>>)>;

pub(crate) fn terrain_frame(terrain: &mut minecraft_survival::level::Terrain, world: &Loaded, view: &mut MinecraftWorldView, camera: &CullCamera) -> ColumnLights {
    let update = terrain
        .stream
        .frame(&terrain.scene, camera, FADE_MILLIS, &world.atlas, &world.packs);
    view.uploads.extend(update.uploads);
    view.removed.extend(update.removed);
    view.visible = update.visible;
    update.lights
}

pub(crate) fn animate_atlas(view: &mut MinecraftWorldView, animation: &mut AtlasAnimation, clock: &mut f64, dt: f64, atlas: &Atlas) {
    *clock += dt;
    if *clock >= TICK_SECONDS {
        *clock %= TICK_SECONDS;
        let patches = animation.tick(atlas);
        view.atlas_patches.retain(|queued| patches.iter().all(|patch| patch.origin != queued.origin));
        view.atlas_patches.extend(patches);
    }
}

pub(crate) fn apply_streamed(
    registries: &Registries,
    shapes: &mut HashMap<BlockStateId, u16>,
    shape_ids: &mut HashMap<(Vec<[u32; 6]>, u16), u16>,
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
    containers: &mut minecraft_terrain::containers::Containers,
    light: &mut Option<SkyLight>,
    gate: &mut crate::minecraft_spawn_gate::SpawnGate,
    view: &mut MinecraftWorldView,
    at: [f64; 3],
    others: &[(i32, i32, i32)],
    aspect: f32,
    now_ms: u64,
) {
    let [x, y, z] = at;
    let spawn_block = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let here: Vec<(i32, i32, i32)> = std::iter::once(spawn_block).chain(others.iter().copied()).collect();
    let streamed = minecraft_survival::level::stream_tick(terrain, entities, containers, &here, Some(spawn_block));
    apply_streamed(registries, shapes, shape_ids, streamed);
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
    observe_spawn(terrain, gate, at, camera.position, now_ms);
}

pub(crate) struct HostView<'a> {
    pub(crate) server: &'a mut minecraft_survival::server::Server,
    pub(crate) local: &'a minecraft_net::PlayerUuid,
    pub(crate) origins: &'a HashMap<Dimension, [f64; 3]>,
    pub(crate) shown: Option<Dimension>,
    pub(crate) queues: &'a mut crate::minecraft_players::VoxelQueues,
    pub(crate) shapes: &'a mut HashMap<BlockStateId, u16>,
    pub(crate) shape_ids: &'a mut HashMap<(Vec<[u32; 6]>, u16), u16>,
    pub(crate) packs: &'a PackStack,
    pub(crate) atlas: &'a Atlas,
    pub(crate) language: Option<&'a HashMap<String, String>>,
    pub(crate) mining: &'a mut crate::minecraft_mining::Mining,
    pub(crate) sounds: Option<&'a mut crate::minecraft_sounds::Sounds>,
    pub(crate) light: Option<&'a SkyLight>,
}

pub(crate) fn host_view(runtime: &mut Runtime) -> Option<HostView<'_>> {
    let Runtime { server, local, origins, shown, queues, shapes, shape_ids, world, inventory_ui, mining, sounds, light, .. } = runtime;
    let world = world.as_ref()?;
    Some(HostView { server: server.as_mut()?, local: local.as_ref()?, origins, shown: *shown, queues, shapes, shape_ids, packs: &world.packs, atlas: &world.atlas, language: inventory_ui.language(), mining, sounds: sounds.as_mut(), light: light.as_ref() })
}

pub(crate) fn show_lines(runtime: &Runtime, chat: &mut frame::McChat, lines: &[minecraft_commands::Clientbound]) {
    if let Some(world) = runtime.world.as_ref().filter(|_| !lines.is_empty()) {
        show_chat(chat, &runtime.inventory_ui, &world.packs, lines);
    }
}

pub(crate) fn presence_of(player: &minecraft_survival::player::Player, ps: Option<&playerstate_iw4::PlayerState>, origin: [f64; 3]) -> minecraft_survival::commands::Presence {
    let alive = ps.is_some_and(|ps| ps.pm_type == 0) && player.arrival.is_none();
    let pose = ps
        .filter(|_| alive)
        .map(|ps| minecraft_commands::Pose { feet: sim::voxel::to_block(origin, ps.origin), yaw: minecraft_yaw(ps.viewangles[1]), pitch: ps.viewangles[0] })
        .or_else(|| player.place.filter(|place| place.dimension == player.dimension).map(|place| minecraft_commands::Pose { feet: place.feet, yaw: place.yaw, pitch: place.pitch }));
    minecraft_survival::commands::Presence { pose, alive }
}

pub(crate) fn friends_presence(players: &std::collections::BTreeMap<minecraft_net::PlayerUuid, minecraft_survival::player::Player>, authority: &sim::SimWorld, origins: &HashMap<Dimension, [f64; 3]>, host: &minecraft_net::PlayerUuid) -> Vec<(minecraft_net::PlayerUuid, minecraft_survival::commands::Presence)> {
    players
        .iter()
        .filter(|(uuid, _)| *uuid != host)
        .filter_map(|(uuid, player)| {
            let origin = *origins.get(&player.dimension)?;
            Some((uuid.clone(), presence_of(player, authority.player(sim::ClientId(player.id.0)), origin)))
        })
        .collect()
}

pub(crate) fn place_friend(player: &mut minecraft_survival::player::Player, authority: &mut sim::SimWorld, id: sim::ClientId, ps: &playerstate_iw4::PlayerState, origin: [f64; 3]) {
    let chunk = |block: f64| (block.floor() as i32) >> 4;
    if let Some(place) = player.arrival
        && !sim::voxel::chunks_ready_around(chunk(place.feet[0]), chunk(place.feet[2]), crate::minecraft_spawn_gate::SPAWN_CHUNK_RADIUS)
    {
        return;
    }
    place_player(player, authority, id, ps, origin, &mut crate::minecraft_spawn_gate::SpawnGate::default(), 0);
}

pub(crate) fn place_player(
    player: &mut minecraft_survival::player::Player,
    authority: &mut sim::SimWorld,
    id: sim::ClientId,
    ps: &playerstate_iw4::PlayerState,
    origin: [f64; 3],
    gate: &mut crate::minecraft_spawn_gate::SpawnGate,
    now_ms: u64,
) {
    let Some(place) = player.arrival.take() else {
        let feet = sim::voxel::to_block(origin, ps.origin);
        player.place = Some(minecraft_survival::player_nbt::Place {
            dimension: player.dimension,
            feet,
            yaw: minecraft_yaw(ps.viewangles[1]),
            pitch: ps.viewangles[0],
            motion: player_motion(ps.velocity),
            on_ground: ps.ground_entity_num != playerstate_iw4::ENTITYNUM_NONE,
            fall_distance: player.steps.air_peak.map_or(0.0, |peak| (peak - feet[1]).max(0.0)),
        });
        return;
    };
    authority.teleport_facing(id, sim::voxel::to_map(origin, place.feet), [place.pitch, minecraft_yaw(place.yaw), 0.0]);
    authority.push_velocity(id, arrival_velocity(place.motion));
    if player.mode.abilities.flying {
        authority.set_minecraft_abilities(id, ability_bits(&player.mode) | playerstate_iw4::pm_flags::MC_FLYING);
    }
    player.steps.last = None;
    player.portal.last_feet = None;
    player.steps.air_peak = (place.fall_distance > 0.0).then_some(place.feet[1] + place.fall_distance);
    let chunk = |block: f64| (block.floor() as i32) >> 4;
    if !gate.waiting() && !sim::voxel::chunks_ready_around(chunk(place.feet[0]), chunk(place.feet[2]), crate::minecraft_spawn_gate::SPAWN_CHUNK_RADIUS) {
        gate.start(now_ms);
    }
    player.place = Some(place);
}

fn player_motion(velocity: [f32; 3]) -> [f64; 3] {
    sim::voxel::to_block([0.0; 3], velocity).map(|v| v * TICK_SECONDS)
}

fn arrival_velocity(motion: [f64; 3]) -> [f32; 3] {
    sim::voxel::to_map([0.0; 3], motion.map(|v| v / TICK_SECONDS))
}

pub(crate) fn observe_spawn(terrain: &minecraft_survival::level::Terrain, gate: &mut crate::minecraft_spawn_gate::SpawnGate, origin: [f64; 3], eye: glam::DVec3, now_ms: u64) {
    if !gate.waiting() {
        return;
    }
    let spawn_chunk = ((origin[0].floor() as i32) >> 4, (origin[2].floor() as i32) >> 4);
    let chunks_ready = sim::voxel::chunks_ready_around(spawn_chunk.0, spawn_chunk.1, crate::minecraft_spawn_gate::SPAWN_CHUNK_RADIUS);
    let eye = eye.floor();
    let section_visible = terrain.stream.section_compiled_and_visible((eye.x as i32, eye.y as i32, eye.z as i32), FADE_MILLIS);
    gate.observe(now_ms, chunks_ready, section_visible);
}

pub(crate) fn shape_id(
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

pub(crate) fn write_blocks(
    terrain: &mut minecraft_survival::level::Terrain,
    registries: &Registries,
    shapes: &mut HashMap<BlockStateId, u16>,
    shape_ids: &mut HashMap<(Vec<[u32; 6]>, u16), u16>,
    entities: &mut minecraft_survival::entities::Entities,
    writes: Vec<((i32, i32, i32), Option<minecraft_terrain::scene::Block>)>,
) {
    for (at, block) in &writes {
        let state = block.as_ref().and_then(|b| terrain.stream.states.state_of(b));
        let shape = state.map_or(0, |state| shape_id(shapes, shape_ids, registries, state));
        sim::voxel::set_block_shape(at.0, at.1, at.2, shape);
    }
    minecraft_survival::level::write_blocks(terrain, entities, writes);
}

pub(crate) fn ability_bits(mode: &minecraft_terrain::game_mode::PlayerMode) -> u32 {
    use playerstate_iw4::pm_flags::{MC_INVULNERABLE, MC_MAY_FLY, MC_NOCLIP};
    [(mode.abilities.invulnerable, MC_INVULNERABLE), (mode.abilities.may_fly, MC_MAY_FLY), (mode.noclip(), MC_NOCLIP)]
        .into_iter()
        .filter(|&(on, _)| on)
        .fold(0, |bits, (_, bit)| bits | bit)
}

pub(crate) fn sky_bits(environment: &minecraft_terrain::environment::DimensionEnvironment) -> u32 {
    if environment.presentation.has_ceiling { playerstate_iw4::pm_flags::MC_NO_SKY } else { 0 }
}

fn in_remote_killstreak(presented: &net::PresentedSnapshot, local: sim::ClientId) -> bool {
    let missile = presented.snapshot().and_then(|snapshot| snapshot.meta.for_client(local)).is_some_and(|meta| meta.remote_missile.is_some());
    missile || presented.player(local).is_some_and(|ps| remote_view(ps.pm_type, ps.other_flags))
}

pub(crate) fn remote_view(pm_type: i32, other_flags: u32) -> bool {
    pm_type == playerstate_iw4::PM_TYPE_NORMAL_LINKED || other_flags & playerstate_iw4::other_flags::AC130 != 0
}

fn ui_game_mode(mode: minecraftoss_player::GameMode) -> frame::McGameMode {
    match mode {
        minecraftoss_player::GameMode::Survival => frame::McGameMode::Survival,
        minecraftoss_player::GameMode::Creative => frame::McGameMode::Creative,
        minecraftoss_player::GameMode::Adventure => frame::McGameMode::Adventure,
        minecraftoss_player::GameMode::Spectator => frame::McGameMode::Spectator,
    }
}

pub(crate) fn mc_vitals(vitals: minecraft_survival::survival::Vitals) -> frame::McVitals {
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
        withered: vitals.withered,
    }
}

fn draw_body_gear(
    mut view: ResMut<MinecraftWorldView>,
    mut runtime: NonSendMut<Runtime>,
    posed: Res<crate::anim::minecraft_gear::PosedGearFrame>,
    gear: Res<crate::anim::minecraft_gear::McBodyGear>,
    puppet: Res<frame::InventoryPuppet>,
) {
    view.puppet_gear = Default::default();
    let Runtime { world: Some(world), light: Some(light), gear_items, .. } = &mut *runtime else {
        return;
    };
    let meshes = crate::anim::minecraft_gear::gear_meshes(&posed, &gear, view.origin, puppet.active.then_some(puppet.client), gear_items, &world.packs, &world.atlas, light);
    let (bytes, indices) = &mut view.particles;
    let base = (bytes.len() / std::mem::size_of::<minecraft_terrain::mesh::SectionVertex>()) as u32;
    bytes.extend_from_slice(bytemuck::cast_slice(&minecraft_terrain::mesh::section_vertices(&meshes.world)));
    indices.extend(meshes.world.indices.iter().map(|i| i + base));
    view.puppet_gear = (bytemuck::cast_slice(&minecraft_terrain::mesh::section_vertices(&meshes.puppet)).to_vec(), meshes.puppet.indices);
}

fn save_before_exit(mut exit: MessageReader<AppExit>, mut runtime: NonSendMut<Runtime>) {
    if exit.read().last().is_some() {
        close_world(&mut runtime, true);
    }
}

fn close_world(runtime: &mut Runtime, blocking: bool) -> bool {
    let mut server = runtime.server.take();
    let local = runtime.local.take();
    if let (Some(server), Some(local)) = (server.as_mut(), local.as_ref()) {
        for level in server.levels.values_mut() {
            minecraft_survival::level::leave(level, local);
        }
    }
    runtime.shown = None;
    runtime.origins.clear();
    runtime.switch_screen = None;
    runtime.loading_backgrounds = None;
    runtime.refused_air = None;
    runtime.queues = Default::default();
    runtime.relayed = Default::default();
    if let (Some(server), Some(local)) = (server.as_mut(), local.as_ref())
        && server.globals.save.is_none()
        && let Some(player) = server.players.get_mut(local)
    {
        runtime.carried_survival = Some(std::mem::take(&mut player.survival));
    }
    let world = runtime.world.take();
    runtime.gear_items = Default::default();
    let loading = runtime.loading.take();
    let loader = runtime.loader.take();
    let open = world.is_some() || loading.is_some();
    if server.is_some() || open || loader.is_some() {
        let close = move || save_and_close(server, world, loading, loader);
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
) {
    let started = std::time::Instant::now();
    for level in server.iter_mut().flat_map(|server| server.levels.values_mut()) {
        if let Some(entities) = level.entities.as_mut() {
            minecraft_survival::level::save_server_block_entities(entities, &mut level.terrain, true);
        }
        drop(level.entities.take());
    }
    if let Some(loader) = loader {
        let _ = loader.join();
    }
    drop(loading);
    let (Some(world), Some(server)) = (world, server) else {
        return;
    };
    drop(world);
    server.save_and_close(started);
}

fn awaiting_level(runtime: &Runtime) -> bool {
    let (Some(server), Some(local)) = (runtime.server.as_ref(), runtime.local.as_ref()) else {
        return false;
    };
    server.players.get(local).is_some_and(|player| !server.levels.contains_key(&player.dimension) || runtime.shown != Some(player.dimension))
}

fn follow_levels(runtime: &mut Runtime, view: &mut MinecraftWorldView, now_ms: u64) -> Vec<(Dimension, String)> {
    let (Some(server), Some(local)) = (runtime.server.as_mut(), runtime.local.as_ref()) else {
        return Vec::new();
    };
    let ticks = std::mem::take(&mut server.globals.level_ticks);
    server.follow_weather_commands();
    for closed in server.close_empty_levels(ticks) {
        runtime.origins.remove(&closed);
    }
    server.prepare_respawns();
    server.open_levels(VIEW_DISTANCE);
    let failed: Vec<(Dimension, String)> = server.finish_opening().into_iter().filter_map(|(dimension, opened)| opened.err().map(|error| (dimension, error))).collect();
    minecraft_survival::portal_forcer::arrive_through_portals(server);
    let Some(player) = server.players.get(local) else {
        return failed;
    };
    let dimension = player.dimension;
    let Some(level) = server.levels.get_mut(&dimension).filter(|_| runtime.shown != Some(dimension)) else {
        return failed;
    };
    let (x, y, z) = level.terrain.stream.player_spawn;
    let arrived = player.arrival.or(player.place).filter(|place| place.dimension == dimension).map(|place| place.feet);
    let origin = *runtime.origins.entry(dimension).or_insert_with(|| match dimension {
        Dimension::Overworld => [x, y, z],
        _ => arrived.unwrap_or([x, y, z]),
    });
    if let Some(entities) = level.entities.as_mut() {
        entities.set_players(&[(u64::from(player.id.0), local.clone())]);
    }
    sim::voxel::switch_level(origin);
    view.origin = origin;
    view.generation += 1;
    view.uploads.clear();
    view.removed.clear();
    view.visible.clear();
    view.particles = Default::default();
    view.entity_meshes = Default::default();
    view.cracks = Default::default();
    view.clouds = None;
    view.light_volume = None;
    runtime.shapes.clear();
    runtime.shape_ids.clear();
    let chunks = level.terrain.stream.reshow(&level.terrain.scene);
    apply_streamed(&server.globals.registries, &mut runtime.shapes, &mut runtime.shape_ids, (chunks, Vec::new()));
    runtime.light = Some(SkyLight::streamed());
    runtime.light_volume_at = None;
    runtime.cloud_center = None;
    runtime.environment_accumulator = 0.0;
    runtime.environment_primed = false;
    runtime.mining = Default::default();
    runtime.minimap = Default::default();
    runtime.spawn_gate.start(now_ms);
    runtime.shown = Some(dimension);
    diag::info!(World, "Minecraft level shown: {} with {:?} at map origin", dimension.dimension_type(), origin);
    failed
}

fn local_switching(runtime: &Runtime) -> bool {
    let (Some(server), Some(local)) = (runtime.server.as_ref(), runtime.local.as_ref()) else {
        return false;
    };
    server.players.get(local).is_some_and(|player| player.switching.is_some())
}

fn leave_level(
    level: &mut minecraft_survival::level::Level,
    player: &mut minecraft_survival::player::Player,
    ui: &mut frame::MinecraftUi,
    inventory_ui: &mut crate::minecraft_inventory::InventoryUi,
) {
    let minecraft_survival::player::Player { id, uuid, inventory, selected, ender, survival, hand, steps, place, .. } = player;
    ui.inventory_open = false;
    if let Some(entities) = level.entities.as_mut() {
        let experience = entities.leave(u64::from(id.0), inventory, *selected, place.map(|place| place.feet));
        let _ = survival.gain_experience(experience);
        let roll = entities.random_float();
        let thrown = level.containers.with_ender(ender, |containers| inventory_ui.apply_input(ui, inventory, selected, containers, &mut || roll));
        if let Some(place) = place.as_ref() {
            let thrower = crate::minecraft_inventory::Thrower {
                eye: glam::DVec3::new(place.feet[0], place.feet[1] + crate::minecraft_spawn_gate::SPAWN_EYE_HEIGHT, place.feet[2]),
                yaw: place.yaw,
                pitch: place.pitch,
            };
            crate::minecraft_inventory::throw(&mut entities.world_items, thrown, &thrower);
        }
        let mut writes = Vec::new();
        for event in level.openers.leave(minecraft_survival::level::opener_key(uuid)) {
            let outcome = minecraft_survival::level::opener_outcome(&level.terrain.scene, &event);
            let (cell, id, count) = outcome.block_event;
            entities.lids.block_event(&level.terrain.scene, cell, id, count);
            writes.extend(outcome.state.map(|(cell, state)| (cell, Some(state))));
        }
        if !writes.is_empty() {
            minecraft_survival::level::write_blocks(&mut level.terrain, entities, writes);
        }
    }
    survival.stop_eating();
    hand.blocking = None;
    hand.charging = None;
    hand.swing = None;
    hand.recent.clear();
    steps.air_peak = None;
    steps.last = None;
}

fn stop(runtime: &mut Runtime, view: &mut MinecraftWorldView) {
    if close_world(runtime, false) || view.active {
        sim::voxel::deactivate();
        runtime.spawn_gate = Default::default();
        clear_view(view);
    }
}

pub(crate) fn clear_view(view: &mut MinecraftWorldView) {
    view.active = false;
    view.atlas = None;
    view.atlas_patches.clear();
    view.uploads.clear();
    view.removed.clear();
    view.visible.clear();
    view.celestial = None;
    view.crack_texture = None;
    view.glint_texture = None;
    view.armor_glint_texture = None;
    view.end_look = Default::default();
    view.particles = Default::default();
    view.puppet_gear = Default::default();
    view.entity_meshes = Default::default();
    view.hand = Default::default();
    view.cracks = Default::default();
    view.clouds = None;
    view.light_volume = None;
    view.generation += 1;
}

#[cfg(test)]
mod remote_tests {
    use super::remote_view;

    #[test]
    fn a_linked_player_or_one_in_the_ac130_is_in_a_remote_killstreak() {
        assert!(!remote_view(0, 0), "on foot");
        assert!(remote_view(playerstate_iw4::PM_TYPE_NORMAL_LINKED, 0), "the chopper gunner links the player to the helicopter");
        assert!(remote_view(0, playerstate_iw4::other_flags::AC130));
        assert!(!remote_view(playerstate_iw4::PM_TYPE_DEAD, 0));
        assert!(!remote_view(0, playerstate_iw4::other_flags::PLAYER));
    }
}

#[cfg(test)]
mod yaw_tests {
    use super::minecraft_yaw;

    fn same_angle(a: f32, b: f32) -> bool {
        (a - b).rem_euclid(360.0).min((b - a).rem_euclid(360.0)) < 1e-3
    }

    #[test]
    fn the_yaw_turns_from_mw2_to_minecraft_and_back_with_the_same_formula() {
        for (mw2, minecraft) in [(-90.0, 0.0), (0.0, -90.0), (90.0, 180.0), (180.0, 90.0)] {
            assert!(same_angle(minecraft_yaw(mw2), minecraft), "{mw2} -> {}", minecraft_yaw(mw2));
            assert!(same_angle(minecraft_yaw(minecraft), mw2), "{minecraft} -> {}", minecraft_yaw(minecraft));
        }
        for mw2 in [-170.0, -33.5, 12.25, 135.0] {
            assert!(same_angle(minecraft_yaw(minecraft_yaw(mw2)), mw2), "{mw2}");
        }
    }
}

#[cfg(test)]
mod arrival_tests {
    use super::{arrival_velocity, player_motion};

    #[test]
    fn an_arrival_gives_back_the_motion_a_departure_saved() {
        for velocity in [[180.0, -36.0, 72.0], [-500.0, 12.5, -300.0], [0.0, 0.0, 900.0]] {
            let motion = player_motion(velocity);
            let back = arrival_velocity(motion);
            for axis in 0..3 {
                assert!((back[axis] - velocity[axis]).abs() < 1e-3, "{velocity:?} -> {motion:?} -> {back:?}");
            }
        }
        let east_up_south = arrival_velocity([0.5, 0.25, 0.1]);
        assert!((east_up_south[0] - 360.0).abs() < 1e-3 && (east_up_south[1] + 72.0).abs() < 1e-3 && (east_up_south[2] - 180.0).abs() < 1e-3, "{east_up_south:?}");
    }

    #[test]
    fn no_motion_leaves_the_player_still() {
        assert_eq!(arrival_velocity([0.0; 3]), [0.0; 3]);
        assert_eq!(player_motion([0.0; 3]), [0.0; 3]);
    }
}
