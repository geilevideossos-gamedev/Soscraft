//! Local Skate gameplay adapter. Rendering and match ownership remain in IW4L.
pub mod collision;
pub mod rails;
pub mod rig;
use bevy::prelude::*;
use frame::{AppScreen, SkateMode};
use skate_host::bridge::{CollisionBuilder, InputFrame, Pose, PreparedCollision, Session};
use std::sync::{Arc, Mutex, mpsc};

enum Job {
    Activate(u64, Vec3, f32, f32),
    Step(u64, f32, InputFrame, f32),
    Suspend,
}
enum Reply {
    Ready,
    Activated(u64, Pose, u128),
    Pose(u64, Pose),
    Error(String),
}
#[derive(Resource, Default)]
struct Host {
    send: Option<mpsc::Sender<Job>>,
    receive: Option<Mutex<mpsc::Receiver<Reply>>>,
    clip: Option<Arc<asset_world::ClipCollision>>,
    ready: bool,
    enter_requested: bool,
    activating: bool,
    epoch: u64,
    pad_packet: u32,
    previous_buttons: u16,
    input_suspended: bool,
    logged_tick: u64,
    preloaded_banks: bool,
    notice_at: Option<f64>,
    teleport_bit: Option<u32>,
    flick: FlickState,
}

const SKATING_OFF: &str = "Skating is off (Options > Controls > Minecraft)";
const NO_SKATE_DATA: &str = "Skate 3 data is not set up: restart to select your default.xex";
const NOTICE_SECONDS: f64 = 3.0;

fn refused_toggle(skating: bool, assets: bool) -> Option<&'static str> {
    if !skating {
        Some(SKATING_OFF)
    } else if !assets {
        Some(NO_SKATE_DATA)
    } else {
        None
    }
}

const WIPEOUT_FALL: f32 = 0.05;

fn airborne(state: &str, was_airborne: bool, drop: f32) -> bool {
    match state {
        "PhysicsAir" | "KnownAir" | "PhysicsAirSecondary" | "BipedAir" => true,
        "WipeoutGround" => was_airborne && drop > WIPEOUT_FALL,
        _ => false,
    }
}

fn teleported(last: Option<u32>, now: u32) -> bool {
    last.is_some_and(|last| last != now)
}

pub fn register(app: &mut App) {
    app.init_resource::<SkateMode>()
        .init_resource::<Host>()
        .add_systems(
            Update,
            update
                .after(frame::PresentedPublished)
                .before(crate::sync_camera_from_presented)
                .before(render_scene::GfxSceneAdd)
                .in_set(frame::ClientSet::Present),
        );
}

fn preload_assets(mode: &mut SkateMode) {
    let Some(root) = std::env::var_os("IW4L_SKATE_ASSETS") else {
        return;
    };
    mode.preload_pending = true;
    if let Err(e) = std::thread::Builder::new()
        .name("skate-preload".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let start = std::time::Instant::now();
            match Session::preload(std::path::Path::new(&root)) {
                Ok(()) => diag::info!(
                    World,
                    "Skate animation banks preloaded in {}ms",
                    start.elapsed().as_millis()
                ),
                Err(e) => diag::warn!(World, "Skate preload: {e}"),
            }
        })
    {
        diag::warn!(World, "Skate preload thread: {e}");
    }
}

/// Blocks across either way of the skater that a Minecraft world's collision
/// covers, blocks up and down, and how far the skater goes before it is
/// rebuilt around them.
const BLOCK_RADIUS: i32 = 40;
const BLOCK_DEPTH: i32 = 20;
const BLOCK_RECENTRE: f32 = 14.0;

/// A Minecraft world's collision around map point `centre`, for Skate. Its
/// block edges become grind rails the same way any map's lips do: the same
/// detector that walks MW2 collision walks the block faces it is handed.
fn block_collision(
    builder: &CollisionBuilder,
    centre: Vec3,
) -> Result<(PreparedCollision, usize), String> {
    let mut map_triangles: Vec<[Vec3; 3]> =
        sim::voxel::collision_triangles(centre.to_array(), BLOCK_RADIUS, BLOCK_DEPTH)
            .into_iter()
            .map(|t| t.map(Vec3::from_array))
            .collect();
    if map_triangles.is_empty() {
        return Err("no blocks around the skater yet".into());
    }
    let started = std::time::Instant::now();
    let ramps = sim::voxel::skate_ramps(centre.to_array(), BLOCK_RADIUS, BLOCK_DEPTH);
    diag::debug!(
        World,
        "Skate block ramps: {} triangles in {} ms",
        ramps.len(),
        started.elapsed().as_millis(),
    );
    map_triangles.extend(ramps.into_iter().map(|t| t.map(Vec3::from_array)));
    let (mut found, census) = rails::find(&map_triangles, rails::Limits::BLOCKS);
    let track = sim::voxel::track_rails(centre.to_array(), BLOCK_RADIUS, BLOCK_DEPTH);
    diag::debug!(
        World,
        "Skate block rails: {} walkable edges, {} lips, {} runs, {} rails, {} track rails",
        census.candidates,
        census.lips,
        census.runs,
        census.rails,
        track.len(),
    );
    found.extend(
        track
            .into_iter()
            .map(|rail| rail.into_iter().map(Vec3::from_array).collect()),
    );
    let walls = sim::voxel::missing_chunk_walls(centre.to_array(), BLOCK_RADIUS, BLOCK_DEPTH);
    if !walls.is_empty() {
        diag::debug!(
            World,
            "Skate chunk walls: {} triangles at chunks not loaded yet",
            walls.len(),
        );
    }
    map_triangles.extend(walls.into_iter().map(|t| t.map(Vec3::from_array)));
    let triangles: Vec<[[f32; 3]; 3]> = map_triangles
        .iter()
        .map(|t| t.map(|p| collision::to_skate(p).to_array()))
        .collect();
    let rails: Vec<Vec<[f32; 3]>> = found
        .into_iter()
        .map(|rail| {
            rail.into_iter()
                .map(|p| collision::to_skate(p).to_array())
                .collect()
        })
        .collect();
    let n = triangles.len();
    Ok((builder.build(triangles, rails)?, n))
}

/// One retained session per map. Leaving skating only pauses this worker;
/// collision, decoded animation banks, graphs and the rig remain resident.
fn preload_map(host: &mut Host, clip: Arc<asset_world::ClipCollision>) -> Result<(), String> {
    let root =
        std::env::var_os("IW4L_SKATE_ASSETS").ok_or("IW4L_SKATE_ASSETS is not configured")?;
    rig::reference().ok_or("Skate rig.json could not be loaded")?;
    assets::bot_model::local_skate_board().ok_or("Skate board.json could not be loaded")?;
    let (send, receive) = mpsc::channel();
    let (publish, results) = mpsc::channel();
    let geometry = clip.clone();
    std::thread::Builder::new()
        .name("iw4l-skate".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let result = (|| -> Result<(), String> {
                let start = std::time::Instant::now();
                let world = collision::extract(&geometry);
                let mut session = Session::new(
                    std::path::Path::new(&root),
                    world.triangles,
                    world.rails,
                    [0., 0., 0.],
                    0.,
                )?;
                diag::info!(
                    World,
                    "Skate map session preloaded in {}ms",
                    start.elapsed().as_millis()
                );
                if publish.send(Reply::Ready).is_err() {
                    return Ok(());
                }
                // On a Minecraft world the collision streams: built around
                // the skater off this thread and swapped in as they move or
                // the blocks change.
                let builder = session.collision_builder();
                let (build_send, build_jobs) = mpsc::channel::<Vec3>();
                let (built_send, built) = mpsc::channel::<(u64, Vec3, Result<(PreparedCollision, usize), String>)>();
                std::thread::Builder::new()
                    .name("iw4l-skate-blocks".into())
                    .spawn(move || {
                        while let Ok(mut centre) = build_jobs.recv() {
                            while let Ok(newer) = build_jobs.try_recv() {
                                centre = newer;
                            }
                            let revision = sim::voxel::revision();
                            let prepared = block_collision(&builder, centre);
                            if built_send.send((revision, centre, prepared)).is_err() {
                                break;
                            }
                        }
                    })
                    .map_err(|e| e.to_string())?;
                let mut blocks: Option<(u64, Vec3)> = None;
                let mut building = false;
                let mut requested = std::time::Instant::now();
                let mut skater_at: Option<Vec3> = None;
                let mut accumulated = 0.;
                let mut epoch = 0;
                while let Ok(job) = receive.recv() {
                    match job {
                        Job::Activate(new_epoch, spawn, yaw, aspect_ratio) => {
                            epoch = new_epoch;
                            accumulated = 0.;
                            let start = std::time::Instant::now();
                            session.set_aspect_ratio(aspect_ratio);
                            if sim::voxel::active() {
                                let revision = sim::voxel::revision();
                                match block_collision(&session.collision_builder(), spawn) {
                                    Ok((prepared, n)) => {
                                        session.install_collision(prepared)?;
                                        blocks = Some((revision, spawn));
                                        diag::info!(World, "Skate: {n} block collision triangles around the spawn");
                                    }
                                    Err(e) => diag::warn!(World, "Skate block collision: {e}"),
                                }
                                skater_at = Some(spawn);
                            }
                            let p = session.activate(
                                collision::to_skate(spawn).to_array(),
                                yaw.to_radians() + std::f32::consts::FRAC_PI_2,
                            )?;
                            if publish
                                .send(Reply::Activated(epoch, p, start.elapsed().as_millis()))
                                .is_err()
                            {
                                break;
                            }
                        }
                        Job::Suspend => {
                            accumulated = 0.;
                            session.suspend_input();
                        }
                        Job::Step(request, dt, input, aspect_ratio) => {
                            if request != epoch {
                                continue;
                            }
                            if let Ok((revision, centre, prepared)) = built.try_recv() {
                                building = false;
                                match prepared {
                                    Ok((prepared, _)) => {
                                        session.install_collision(prepared)?;
                                        blocks = Some((revision, centre));
                                    }
                                    Err(e) => diag::warn!(World, "Skate block collision: {e}"),
                                }
                            }
                            if sim::voxel::active()
                                && !building
                                && let Some(at) = skater_at
                            {
                                let far = blocks.is_none_or(|(_, centre)| {
                                    let d = (at - centre) / sim::voxel::BLOCK;
                                    d.truncate().length() > BLOCK_RECENTRE || d.z.abs() > BLOCK_DEPTH as f32 * 0.5
                                });
                                // Only changes that reach the blocks it covers: chunks
                                // stream in and out far away the whole time.
                                let changed = requested.elapsed().as_secs_f32() > 0.25
                                    && blocks.is_some_and(|(revision, centre)| {
                                        sim::voxel::changed_near(revision, centre.to_array(), BLOCK_RADIUS, BLOCK_DEPTH)
                                    });
                                if (far || changed) && build_send.send(at).is_ok() {
                                    building = true;
                                    requested = std::time::Instant::now();
                                }
                            }
                            session.set_aspect_ratio(aspect_ratio);
                            session.collect(input, dt);
                            accumulated = (accumulated + dt).min(0.15);
                            let mut advanced = false;
                            // The native camera can change the simulation period.
                            while accumulated >= session.period() {
                                accumulated -= session.period();
                                session.advance()?;
                                advanced = true;
                            }
                            if advanced {
                                let p = session.pose();
                                if !p.root.is_finite() || p.bones.iter().any(|b| !b.is_finite()) {
                                    return Err("Skate published a non-finite pose".into());
                                }
                                skater_at = Some(collision::from_skate(p.root.w_axis.truncate()));
                                if publish.send(Reply::Pose(epoch, p)).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(())
            })();
            if let Err(e) = result {
                let _ = publish.send(Reply::Error(e));
            }
        })
        .map_err(|e| e.to_string())?;
    host.send = Some(send);
    host.receive = Some(Mutex::new(results));
    host.clip = Some(clip);
    host.ready = false;
    Ok(())
}

fn stop(host: &mut Host, mode: &mut SkateMode, authority: &mut net::AuthorityWorld) {
    authority
        .0
        .set_external_motion(sim::ClientId(mode.client), false);
    host.enter_requested = false;
    host.activating = false;
    host.epoch = host.epoch.wrapping_add(1);
    if let Some(send) = &host.send {
        let _ = send.send(Job::Suspend);
    }
    mode.active = false;
    mode.airborne = false;
    mode.entering = false;
    mode.camera = None;
    mode.bones.clear();
    mode.status.clear();
    diag::info!(World, "Skate mode stopped; map session retained");
}

fn present(mode: &mut SkateMode, p: Pose, authority: &mut net::AuthorityWorld) {
    let b = collision::basis();
    let mut root = b * p.root * b.inverse();
    root.w_axis = collision::from_skate(p.root.w_axis.truncate()).extend(1.);
    let drop = mode.root.w_axis.z - root.w_axis.z;
    mode.root = root;
    mode.bones = p.bones;
    mode.names = p.names;
    mode.tick = p.tick;
    mode.airborne = airborne(&p.state, mode.airborne, drop);
    mode.status = p.state;
    mode.camera = p.camera.map(|(position, basis, fov)| {
        (
            Transform::from_translation(collision::from_skate(position)).looking_to(
                b.transform_vector3(basis.z_axis).normalize(),
                b.transform_vector3(basis.y_axis).normalize(),
            ),
            fov,
        )
    });
    authority.0.set_origin(
        sim::ClientId(mode.client),
        root.w_axis.truncate().to_array(),
    );
}

fn update(
    time: Res<Time>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    screen: Res<AppScreen>,
    local: Res<net::LocalPresentClient>,
    presented: Res<net::PresentedSnapshot>,
    clip: Res<crate::DynEntPhysClip>,
    mut authority: Option<ResMut<net::AuthorityWorld>>,
    mut mode: ResMut<SkateMode>,
    mut host: ResMut<Host>,
    (gamepads, active): (Query<&bevy::input::gamepad::Gamepad>, Option<Res<frame::ActivePad>>),
    minecraft: Option<Res<frame::MinecraftUi>>,
    settings: Option<Res<frame::GameSettings>>,
    (keys, mouse_buttons, motion, devices): KeyboardInput<'_>,
) {
    let Some(authority) = authority.as_deref_mut() else {
        return;
    };
    if let Some(bit) = authority.0.player(local.0).map(|ps| ps.e_flags & playerstate_iw4::eflags::TELEPORT)
        && teleported(host.teleport_bit.replace(bit), bit)
        && (mode.active || host.enter_requested || host.activating)
    {
        stop(&mut host, &mut mode, authority);
    }
    let enabled = settings.is_some_and(|settings| settings.skating);
    let assets = std::env::var_os("IW4L_SKATE_ASSETS").is_some();
    let now = time.elapsed_secs_f64();
    if host.notice_at.is_some_and(|at| now - at >= NOTICE_SECONDS) {
        host.notice_at = None;
        mode.status.clear();
    }
    if enabled && !host.preloaded_banks {
        host.preloaded_banks = true;
        preload_assets(&mut mode);
    }
    let aspect_ratio = windows
        .single()
        .map(|w| w.width() / w.height().max(1.))
        .unwrap_or(16. / 9.);
    let ps = presented.player(local.0);
    let alive = ps.is_some_and(|p| p.pm_type == 0) && *screen == AppScreen::InGame;
    let same_map = host
        .clip
        .as_ref()
        .is_none_or(|a| clip.0.as_ref().is_some_and(|b| Arc::ptr_eq(a, b)));
    let board_ok = minecraft.as_deref().is_none_or(|ui| !ui.active || ui.board_allowed);
    if (mode.active || host.enter_requested || host.activating) && (!alive || !same_map || !board_ok) {
        stop(&mut host, &mut mode, authority);
    }
    if !same_map {
        host.send = None;
        host.receive = None;
        host.clip = None;
        host.ready = false;
        mode.preloaded = false;
        mode.preload_pending = enabled && assets;
    }
    if !enabled && (host.send.is_some() || host.clip.is_some() || mode.preloaded || mode.preload_pending) {
        if mode.active || host.enter_requested || host.activating {
            stop(&mut host, &mut mode, authority);
        }
        let had_session = host.send.is_some();
        host.send = None;
        host.receive = None;
        host.clip = None;
        host.ready = false;
        mode.preloaded = false;
        mode.preload_pending = false;
        if had_session {
            diag::info!(World, "Skating turned off: map session dropped; decoded animation banks stay cached until exit");
        }
    }
    // This runs during map preparation/class selection, without waiting for J.
    if host.clip.is_none()
        && enabled
        && assets
        && let Some(geometry) = clip.0.clone()
    {
        mode.preload_pending = true;
        host.clip = Some(geometry.clone()); // A failed load retries on a new map, never every frame.
        if let Err(e) = preload_map(&mut host, geometry) {
            diag::warn!(World, "Skate map preload: {e}");
            mode.preload_pending = false;
            host.notice_at = None;
            mode.status = e;
        }
    }
    // Skating reads the same controller as the rest of the game, whatever
    // kind it is, converted to the Xbox layout the skate input expects.
    let pad = active.and_then(|active| active.0).and_then(|entity| gamepads.get(entity).ok());
    host.pad_packet = host.pad_packet.wrapping_add(1);
    let screen_open = minecraft.as_deref().is_some_and(|ui| ui.active && (ui.inventory_open || ui.chat.open));
    let packet = host.pad_packet;
    let typing = screen_open || devices.as_deref().is_some_and(|devices| !devices.focused);
    let input = match (pad, keys.as_deref(), mouse_buttons.as_deref()) {
        (Some(_), ..) if screen_open => InputFrame::from_pad(0, [0; 2], [0; 2], [0; 2], packet),
        (Some(pad), ..) => pad_frame(pad, packet),
        (None, Some(keys), Some(mouse)) if !typing => {
            let delta = motion
                .as_deref()
                .map_or([0.0; 2], |motion| motion.delta.to_array());
            keyboard_frame(
                key_sample(keys, mouse),
                delta,
                &mut host.flick,
                time.delta_secs(),
            )
            .frame(packet)
        }
        (None, ..) => {
            host.flick = FlickState::default();
            InputFrame::from_pad(0, [0; 2], [0; 2], [0; 2], packet)
        }
    };
    mode.keyboard = pad.is_none();
    mode.controller = input.controller();
    host.previous_buttons = input.buttons();

    let mut replies = Vec::new();
    if let Some(receiver) = &host.receive {
        let receiver = receiver.lock().unwrap();
        loop {
            match receiver.try_recv() {
                Ok(reply) => replies.push(reply),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    replies.push(Reply::Error("Skate worker disconnected".into()));
                    break;
                }
            }
        }
    }
    for reply in replies {
        match reply {
            Reply::Ready => {
                host.ready = true;
                mode.preloaded = true;
                mode.preload_pending = false;
                diag::info!(World, "Skate ready before toggle");
            }
            Reply::Activated(epoch, p, ms) if epoch == host.epoch && host.activating && alive => {
                host.activating = false;
                mode.entering = false;
                mode.active = true;
                host.input_suspended = false;
                authority.0.set_external_motion(local.0, true);
                present(&mut mode, p, authority);
                diag::info!(World, "Skate activation from retained session: {ms}ms");
            }
            Reply::Pose(epoch, p) if epoch == host.epoch && mode.active => {
                if p.tick / 120 != host.logged_tick / 120 {
                    diag::info!(
                        World,
                        "Skate tick={} speed={:.2} state={}",
                        p.tick,
                        p.velocity.length(),
                        p.state
                    );
                    host.logged_tick = p.tick;
                }
                present(&mut mode, p, authority);
            }
            Reply::Error(e) => {
                stop(&mut host, &mut mode, authority);
                host.send = None;
                host.receive = None;
                host.ready = false;
                mode.preloaded = false;
                mode.preload_pending = false;
                // The worker is gone; prepare a fresh session for this map
                // so the player can skate again rather than for the rest of
                // the match being left without one.
                host.clip = None;
                diag::warn!(World, "Skate stopped: {e}; preparing a new session");
                host.notice_at = None;
                mode.status = e;
                return;
            }
            _ => {}
        }
    }
    if std::mem::take(&mut mode.toggle_requested) && alive {
        if mode.active || host.enter_requested || host.activating {
            stop(&mut host, &mut mode, authority);
            return;
        }
        if let Some(notice) = refused_toggle(enabled, assets) {
            mode.status = notice.to_owned();
            host.notice_at = Some(now);
            return;
        }
        if !board_ok {
            return;
        }
        if host.send.is_none() {
            diag::warn!(World, "Skate session unavailable: {}", mode.status);
            return;
        }
        host.enter_requested = true;
        mode.entering = true;
        mode.client = local.0.0;
    }
    if host.enter_requested
        && host.ready
        && let Some(ps) = ps.filter(|_| alive)
    {
        host.epoch = host.epoch.wrapping_add(1);
        host.enter_requested = false;
        host.activating = true;
        if let Some(send) = &host.send {
            let _ = send.send(Job::Activate(
                host.epoch,
                Vec3::from_array(ps.origin) + Vec3::Z * 2.,
                ps.viewangles[1],
                aspect_ratio,
            ));
        }
    }
    if !mode.active {
        return;
    }
    if mode.input_blocked {
        if !host.input_suspended {
            if let Some(send) = &host.send {
                let _ = send.send(Job::Suspend);
            }
        }
        host.input_suspended = true;
        return;
    }
    host.input_suspended = false;
    if let Some(send) = &host.send {
        if send
            .send(Job::Step(
                host.epoch,
                time.delta_secs().min(0.1),
                input,
                aspect_ratio,
            ))
            .is_err()
        {
            stop(&mut host, &mut mode, authority);
        }
    }
}

/// A controller's state in XInput's layout, for the skate input.
fn pad_frame(pad: &bevy::input::gamepad::Gamepad, packet: u32) -> InputFrame {
    use bevy::input::gamepad::GamepadButton as B;
    const BITS: [(B, u16); 14] = [
        (B::DPadUp, 0x0001),
        (B::DPadDown, 0x0002),
        (B::DPadLeft, 0x0004),
        (B::DPadRight, 0x0008),
        (B::Start, 0x0010),
        (B::Select, 0x0020),
        (B::LeftThumb, 0x0040),
        (B::RightThumb, 0x0080),
        (B::LeftTrigger, 0x0100),
        (B::RightTrigger, 0x0200),
        (B::South, 0x1000),
        (B::East, 0x2000),
        (B::West, 0x4000),
        (B::North, 0x8000),
    ];
    let buttons = BITS
        .iter()
        .filter(|(button, _)| pad.pressed(*button))
        .fold(0, |bits, (_, bit)| bits | bit);
    // An analog trigger reports its travel; a digital one only pressed.
    let trigger = |button: B| {
        let value = pad
            .get(button)
            .unwrap_or(if pad.pressed(button) { 1.0 } else { 0.0 });
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    let axis = |v: f32| (v.clamp(-1.0, 1.0) * 32767.0).round() as i16;
    let (left, right) = (pad.left_stick(), pad.right_stick());
    InputFrame::from_pad(
        buttons,
        [trigger(B::LeftTrigger2), trigger(B::RightTrigger2)],
        [axis(left.x), axis(left.y)],
        [axis(right.x), axis(right.y)],
        packet,
    )
}

const STICK: f32 = 32767.0;
const FLICK_PER_PIXEL: f32 = 600.0;
const FLICK_SETTLE: f32 = 0.1;

#[derive(Clone, Copy, Default)]
struct KeySample {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
    space: bool,
    shift: bool,
    ctrl: bool,
    tab: bool,
    z: bool,
    v: bool,
    left_mouse: bool,
    right_mouse: bool,
}

type KeyboardInput<'w> = (
    Option<Res<'w, ButtonInput<KeyCode>>>,
    Option<Res<'w, ButtonInput<MouseButton>>>,
    Option<Res<'w, bevy::input::mouse::AccumulatedMouseMotion>>,
    Option<Res<'w, frame::InputDevices>>,
);

#[derive(Default)]
struct FlickState {
    stick: [f32; 2],
}

#[derive(Debug, PartialEq)]
struct PadState {
    buttons: u16,
    triggers: [u8; 2],
    left: [i16; 2],
    right: [i16; 2],
}

impl PadState {
    fn frame(&self, packet: u32) -> InputFrame {
        InputFrame::from_pad(self.buttons, self.triggers, self.left, self.right, packet)
    }
}

fn key_sample(keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> KeySample {
    KeySample {
        w: keys.pressed(KeyCode::KeyW),
        a: keys.pressed(KeyCode::KeyA),
        s: keys.pressed(KeyCode::KeyS),
        d: keys.pressed(KeyCode::KeyD),
        space: keys.pressed(KeyCode::Space),
        shift: keys.pressed(KeyCode::ShiftLeft),
        ctrl: keys.pressed(KeyCode::ControlLeft),
        tab: keys.pressed(KeyCode::Tab),
        z: keys.pressed(KeyCode::KeyZ),
        v: keys.pressed(KeyCode::KeyV),
        left_mouse: mouse.pressed(MouseButton::Left),
        right_mouse: mouse.pressed(MouseButton::Right),
    }
}

fn keyboard_frame(keys: KeySample, mouse: [f32; 2], state: &mut FlickState, dt: f32) -> PadState {
    let settle = STICK * dt / FLICK_SETTLE;
    for (stick, moved) in state.stick.iter_mut().zip([mouse[0], -mouse[1]]) {
        let eased = stick.signum() * (stick.abs() - settle).max(0.0);
        let push = moved * FLICK_PER_PIXEL;
        let base = if eased * push < 0.0 { 0.0 } else { eased };
        *stick = (base + push).clamp(-STICK, STICK);
    }
    let buttons = [
        (keys.space, 0x1000),
        (keys.ctrl, 0x2000),
        (keys.shift, 0x4000),
        (keys.tab, 0x8000),
        (keys.z, 0x0100),
        (keys.v, 0x0200),
    ]
    .iter()
    .filter(|(held, _)| *held)
    .fold(0, |bits, (_, bit)| bits | bit);
    let axis = |plus: bool, minus: bool| (i16::from(plus) - i16::from(minus)) * 32767;
    let trigger = |held: bool| if held { 255 } else { 0 };
    PadState {
        buttons,
        triggers: [trigger(keys.right_mouse), trigger(keys.left_mouse)],
        left: [axis(keys.d, keys.a), axis(keys.w, keys.s)],
        right: state.stick.map(|v| v.round() as i16),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toggle_is_refused_while_skating_is_off_or_its_data_is_missing() {
        assert_eq!(refused_toggle(false, true), Some(SKATING_OFF));
        assert_eq!(refused_toggle(false, false), Some(SKATING_OFF));
        assert_eq!(refused_toggle(true, false), Some(NO_SKATE_DATA));
        assert_eq!(refused_toggle(true, true), None);
    }

    #[test]
    fn airborne_states_are_the_air_states() {
        for state in ["PhysicsAir", "KnownAir", "PhysicsAirSecondary", "BipedAir"] {
            assert!(airborne(state, false, 0.0), "{state}");
        }
        for state in [
            "PhysicsGround",
            "SlideGround",
            "GrindFiftyFifty",
            "GrindBoardslide",
            "WipeoutGround",
            "BipedGround",
            "OffBoardPushing",
            "LandingOnDeck",
            "HandPlant",
            "",
        ] {
            assert!(!airborne(state, false, 10.0), "{state}");
        }
    }

    #[test]
    fn a_wipeout_in_the_air_falls_until_it_lands() {
        assert!(airborne("WipeoutGround", true, 6.0), "a bail in the air keeps falling");
        assert!(!airborne("WipeoutGround", true, 0.0), "the ragdoll has hit the ground");
        assert!(!airborne("WipeoutGround", true, -2.0), "it bounced up off the ground");
        assert!(!airborne("WipeoutGround", false, 6.0), "a bail on the ground tumbling downhill is no fall");
        assert!(!airborne("PhysicsGround", true, 6.0));
        let mut was = true;
        for drop in [9.0, 12.0, 15.0, 0.0, 3.0, 4.0] {
            was = airborne("WipeoutGround", was, drop);
        }
        assert!(!was, "once landed, a wipeout stays on the ground");
    }

    #[test]
    fn a_teleport_flip_picks_the_board_up() {
        assert!(!teleported(None, 0));
        assert!(!teleported(None, 2));
        assert!(!teleported(Some(0), 0));
        assert!(!teleported(Some(2), 2));
        assert!(teleported(Some(0), 2));
        assert!(teleported(Some(2), 0));
    }

    fn keyboard(keys: KeySample) -> PadState {
        keyboard_frame(keys, [0.0; 2], &mut FlickState::default(), 1.0 / 60.0)
    }

    #[test]
    fn w_pushes_the_left_stick_forward() {
        let w = KeySample { w: true, ..KeySample::default() };
        assert_eq!(keyboard(w).left, [0, 32767]);
        assert_eq!(keyboard(KeySample { s: true, ..w }).left, [0, 0]);
        assert_eq!(keyboard(KeySample { s: true, ..KeySample::default() }).left, [0, -32767]);
        assert_eq!(keyboard(KeySample { d: true, ..w }).left, [32767, 32767]);
        assert_eq!(keyboard(KeySample { a: true, ..KeySample::default() }).left, [-32767, 0]);
        let pad = keyboard(w);
        assert_eq!((pad.buttons, pad.triggers, pad.right), (0, [0; 2], [0; 2]));
    }

    #[test]
    fn a_mouse_flick_reaches_the_right_stick_and_settles() {
        let mut flick = FlickState::default();
        let none = KeySample::default();
        let down = keyboard_frame(none, [0.0, 60.0], &mut flick, 1.0 / 60.0);
        assert_eq!(down.right, [0, -32767], "a quick move down pulls the stick down");
        let up = keyboard_frame(none, [0.0, -60.0], &mut flick, 1.0 / 60.0);
        assert_eq!(up.right, [0, 32767], "flicking back up goes straight up");
        let right = keyboard_frame(none, [20.0, 0.0], &mut flick, 0.01);
        assert_eq!(right.right[0], 12000);
        let mut last = right.right;
        for _ in 0..5 {
            last = keyboard_frame(none, [0.0; 2], &mut flick, 0.01).right;
        }
        assert_eq!(last[0], 0, "the sideways nudge is gone");
        assert!((13000..=13200).contains(&last[1]), "four tenths of the way up 0.06 s after the flick: {last:?}");
        for _ in 0..5 {
            last = keyboard_frame(none, [0.0; 2], &mut flick, 0.01).right;
        }
        assert_eq!(last, [0, 0], "centred 0.1 s after the mouse stops");

        let mut slow = FlickState::default();
        for _ in 0..200 {
            last = keyboard_frame(none, [1.0, 1.0], &mut slow, 1.0 / 144.0).right;
        }
        assert!(last.iter().all(|v| v.unsigned_abs() < 8192), "a slow drift stays inside the dead zone: {last:?}");
    }

    #[test]
    fn space_presses_a_and_the_mouse_buttons_pull_the_triggers() {
        let none = KeySample::default();
        assert_eq!(keyboard(KeySample { space: true, ..none }).buttons, 0x1000);
        assert_eq!(keyboard(KeySample { ctrl: true, ..none }).buttons, 0x2000);
        assert_eq!(keyboard(KeySample { shift: true, ..none }).buttons, 0x4000);
        assert_eq!(keyboard(KeySample { tab: true, ..none }).buttons, 0x8000);
        assert_eq!(keyboard(KeySample { z: true, ..none }).buttons, 0x0100);
        assert_eq!(keyboard(KeySample { v: true, ..none }).buttons, 0x0200);
        assert_eq!(keyboard(KeySample { left_mouse: true, ..none }).triggers, [0, 255]);
        assert_eq!(keyboard(KeySample { right_mouse: true, ..none }).triggers, [255, 0]);
        let all = KeySample {
            w: true,
            a: true,
            s: true,
            d: true,
            space: true,
            shift: true,
            ctrl: true,
            tab: true,
            z: true,
            v: true,
            left_mouse: true,
            right_mouse: true,
        };
        assert_eq!(keyboard(all).buttons & 0x00ff, 0, "no Start, Back, d-pad or stick click");
    }

    #[test]
    fn no_key_gives_a_centred_frame() {
        let pad = keyboard(KeySample::default());
        assert_eq!(pad, PadState { buttons: 0, triggers: [0; 2], left: [0; 2], right: [0; 2] });
        let frame = pad.frame(7);
        assert_eq!(frame.controller(), Some(0), "a keyboard frame is a connected controller");
        assert_eq!(frame.buttons(), 0);
    }

    fn to_map(p: [f64; 3]) -> Vec3 {
        Vec3::from_array(sim::voxel::to_map([0.0; 3], p))
    }

    fn box_triangles(b: [f32; 6]) -> Vec<[Vec3; 3]> {
        let b = b.map(f64::from);
        let mut tris = Vec::new();
        for axis in 0..3 {
            let (u, v) = [(1, 2), (2, 0), (0, 1)][axis];
            for high in [false, true] {
                let corner = |a: usize, c: usize| {
                    let mut p = [0.0; 3];
                    p[axis] = if high { b[axis + 3] } else { b[axis] };
                    p[u] = b[u + 3 * a];
                    p[v] = b[v + 3 * c];
                    to_map(p)
                };
                let mut quad = [corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)];
                if !high {
                    quad.reverse();
                }
                tris.push([quad[0], quad[1], quad[2]]);
                tris.push([quad[0], quad[2], quad[3]]);
            }
        }
        tris
    }

    fn rails_near_the_block(block: [f32; 6], with_fillets: bool) -> usize {
        let full = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let mut world: std::collections::HashMap<[i32; 3], Vec<[f32; 6]>> =
            std::collections::HashMap::new();
        for x in -4..=4 {
            for z in -4..=4 {
                world.insert([x, -1, z], vec![full]);
            }
        }
        world.insert([0, 0, 0], vec![block]);
        let ground = [
            Vec3::new(-200., -200., 0.),
            Vec3::new(200., -200., 0.),
            Vec3::new(200., 200., 0.),
            Vec3::new(-200., 200., 0.),
        ];
        let mut tris = vec![
            [ground[0], ground[1], ground[2]],
            [ground[0], ground[2], ground[3]],
        ];
        tris.extend(box_triangles(block));
        if with_fillets {
            let fillets = sim::voxel::riser_fillets(
                |x, y, z| world.get(&[x, y, z]).map_or(&[][..], Vec::as_slice),
                [-2, -1, -2],
                [2, 2, 2],
            );
            tris.extend(fillets.into_iter().map(|t| t.map(to_map)));
        }
        let (found, _) = rails::find(&tris, rails::Limits::BLOCKS);
        found
            .iter()
            .filter(|rail| rail.iter().any(|p| p.x.abs() < 80. && p.y.abs() < 80.))
            .count()
    }

    fn rails_over(blocks: &[([i32; 3], &[[f32; 6]])], limits: rails::Limits) -> Vec<Vec<Vec3>> {
        let full = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let mut world: std::collections::HashMap<[i32; 3], Vec<[f32; 6]>> =
            std::collections::HashMap::new();
        for x in -4..=4 {
            for z in -4..=6 {
                world.insert([x, -1, z], vec![full]);
            }
        }
        let ground = [
            Vec3::new(-300., -300., 0.),
            Vec3::new(300., -300., 0.),
            Vec3::new(300., 300., 0.),
            Vec3::new(-300., 300., 0.),
        ];
        let mut tris = vec![
            [ground[0], ground[1], ground[2]],
            [ground[0], ground[2], ground[3]],
        ];
        for &(at, boxes) in blocks {
            world.insert(at, boxes.to_vec());
            for b in boxes {
                let [x, y, z] = at.map(|v| v as f32);
                tris.extend(box_triangles([
                    b[0] + x,
                    b[1] + y,
                    b[2] + z,
                    b[3] + x,
                    b[4] + y,
                    b[5] + z,
                ]));
            }
        }
        let fillets = sim::voxel::riser_fillets(
            |x, y, z| world.get(&[x, y, z]).map_or(&[][..], Vec::as_slice),
            [-3, -1, -3],
            [3, 4, 5],
        );
        tris.extend(fillets.into_iter().map(|t| t.map(to_map)));
        rails::find(&tris, limits)
            .0
            .into_iter()
            .filter(|rail| rail.iter().all(|p| p.x.abs() < 250. && p.y.abs() < 250.))
            .collect()
    }

    fn turned(blocks: &[([i32; 3], Vec<[f32; 6]>)]) -> Vec<([i32; 3], Vec<[f32; 6]>)> {
        blocks
            .iter()
            .map(|([x, y, z], boxes)| {
                let boxes = boxes
                    .iter()
                    .map(|b| [1.0 - b[5], b[1], b[0], 1.0 - b[2], b[4], b[3]])
                    .collect();
                ([-z - 1, *y, *x], boxes)
            })
            .collect()
    }

    #[test]
    fn the_sides_of_a_stair_ramp_grind_like_a_hubba() {
        let full = vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]];
        let stair = vec![[0.0, 0.0, 0.0, 1.0, 0.5, 1.0], [0.0, 0.5, 0.5, 1.0, 1.0, 1.0]];
        let mut staircase = vec![
            ([0, 0, 0], stair.clone()),
            ([0, 0, 1], full.clone()),
            ([0, 1, 1], stair.clone()),
            ([0, 0, 2], full.clone()),
            ([0, 1, 2], full.clone()),
            ([0, 2, 2], stair),
        ];
        for facing in 0..4 {
            let blocks: Vec<([i32; 3], &[[f32; 6]])> =
                staircase.iter().map(|(at, b)| (*at, b.as_slice())).collect();
            let sloped = |limits: rails::Limits| -> Vec<Vec<Vec3>> {
                rails_over(&blocks, limits)
                    .into_iter()
                    .filter(|rail| (rail[0].z - rail[rail.len() - 1].z).abs() > 9.)
                    .collect()
            };
            assert!(sloped(rails::Limits::MW2).is_empty(), "{facing}");
            let hubbas = sloped(rails::Limits::BLOCKS);
            assert_eq!(hubbas.len(), 2, "{facing}: {hubbas:?}");
            for rail in &hubbas {
                let side = |k: usize| rail.iter().all(|p| (p[k] - rail[0][k]).abs() < 0.01);
                assert!(side(0) || side(1), "{facing}: {rail:?}");
                let len = rail.windows(2).map(|w| w[0].distance(w[1])).sum::<f32>();
                assert!(len > 2.5 * 36. * std::f32::consts::SQRT_2, "{facing}: {len}");
            }
            if facing == 0 {
                let mut sides: Vec<f32> = hubbas.iter().map(|rail| rail[0].x).collect();
                sides.sort_by(f32::total_cmp);
                assert!(sides[0].abs() < 0.01 && (sides[1] - 36.).abs() < 0.01, "{sides:?}");
            }
            staircase = turned(&staircase);
        }
    }

    #[test]
    fn a_fence_line_is_one_rail_down_its_middle() {
        let fence = [[0.375, 0.0, 0.375, 0.625, 1.5, 0.625], [0.0, 0.0, 0.375, 1.0, 1.5, 0.625]];
        let line: Vec<([i32; 3], &[[f32; 6]])> = (0..6).map(|x| ([x, 0, 0], &fence[..])).collect();
        let rails: Vec<Vec<Vec3>> = rails_over(&line, rails::Limits::BLOCKS)
            .into_iter()
            .filter(|rail| rail.iter().all(|p| (p.z - 54.).abs() < 0.01))
            .collect();
        assert_eq!(rails.len(), 1, "{rails:?}");
        assert!(rails[0].iter().all(|p| (p.y + 18.).abs() < 0.01), "{rails:?}");
    }

    #[test]
    fn a_slab_on_the_ground_is_no_longer_a_lip() {
        let slab = [0.0, 0.0, 0.0, 1.0, 0.5, 1.0];
        let full = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        assert!(rails_near_the_block(slab, false) > 0, "a bare slab edge is a lip");
        assert_eq!(rails_near_the_block(slab, true), 0);
        assert!(rails_near_the_block(full, true) > 0, "a full block keeps its lips");
    }
}
