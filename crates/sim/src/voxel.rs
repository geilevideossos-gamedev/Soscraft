//! Collision against a Minecraft world. While a Minecraft map is loaded, world
//! traces against the map it stands in for test block collision shapes
//! instead of that map's brushes and meshes.
//!
//! Minecraft space is blocks with Y up. A map unit point `(X, Y, Z)` is block
//! point `origin + (X, Z, -Y) / BLOCK`: X stays east, Z up becomes Y up, and
//! the left-handed-looking swap of Y and Z is a proper rotation.
use std::collections::HashMap;
use std::sync::RwLock;

use crate::world::SimBrush;

/// Map units per block: the 70-unit soldier stands about two blocks tall,
/// and a block is below the 39-unit jump, so one block can be jumped onto.
pub const BLOCK: f32 = 36.0;
/// Pulled back from every hit, as IW4 traces keep off surfaces.
const SURFACE_CLIP_EPSILON: f32 = 0.125;
const SOLID: u32 = 1;
/// `SURF_TYPE` bits for a stone-like surface.
const STONE_SURFACE: u32 = 17 << 20;

/// One chunk column of shape ids, x fastest then z then y.
pub struct VoxelChunk {
    pub min_y: i32,
    pub height: i32,
    pub shapes: Vec<u16>,
}

/// The block world traces run against, and where it sits in map space.
#[derive(Default)]
pub struct VoxelWorld {
    brushes: usize,
    origin: [f64; 3],
    chunks: HashMap<(i32, i32), VoxelChunk>,
    /// Collision boxes of each shape id, in block space `[min, max]`; id 0 is
    /// empty.
    shapes: Vec<Vec<[f32; 6]>>,
    fluids: Vec<u16>,
}

static WORLD: RwLock<Option<VoxelWorld>> = RwLock::new(None);
/// Bumped whenever the block world's collision changes.
static REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The block box each recent revision touched, oldest first, so a reader
/// can tell whether the changes since its own revision reach a region.
static CHANGES: std::sync::Mutex<std::collections::VecDeque<(u64, [i32; 3], [i32; 3])>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());
const CHANGE_LOG: usize = 256;
const EVERYWHERE: ([i32; 3], [i32; 3]) = ([i32::MIN; 3], [i32::MAX; 3]);

fn bump() {
    bump_region(EVERYWHERE.0, EVERYWHERE.1);
}

fn bump_region(min: [i32; 3], max: [i32; 3]) {
    let Ok(mut log) = CHANGES.lock() else {
        REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return;
    };
    let revision = REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    log.push_back((revision, min, max));
    while log.len() > CHANGE_LOG {
        log.pop_front();
    }
}

/// A chunk column's block box.
fn chunk_region(x: i32, z: i32) -> ([i32; 3], [i32; 3]) {
    ([x * 16, i32::MIN, z * 16], [x * 16 + 15, i32::MAX, z * 16 + 15])
}

/// Whether any change after `since` touched the blocks within `radius`
/// across and `depth` up or down of map point `centre`, or a block next to
/// them (a neighbour decides which faces show). When the log no longer
/// reaches back to `since`, it answers yes.
pub fn changed_near(since: u64, centre: [f32; 3], radius: i32, depth: i32) -> bool {
    if revision() <= since {
        return false;
    }
    let origin = match WORLD.read() {
        Ok(world) => match world.as_ref() {
            Some(world) => world.origin,
            None => return true,
        },
        Err(_) => return true,
    };
    let Ok(log) = CHANGES.lock() else {
        return true;
    };
    if log.front().is_none_or(|(revision, ..)| *revision > since + 1) {
        return true;
    }
    let c = to_block(origin, centre);
    let (cx, cy, cz) = (c[0].floor() as i32, c[1].floor() as i32, c[2].floor() as i32);
    let lo = [cx - radius - 1, cy - depth - 1, cz - radius - 1];
    let hi = [cx + radius + 1, cy + depth + 1, cz + radius + 1];
    log.iter()
        .filter(|(revision, ..)| *revision > since)
        .any(|(_, min, max)| (0..3).all(|i| min[i] <= hi[i] && max[i] >= lo[i]))
}

/// Changes with every change to the block world's collision.
pub fn revision() -> u64 {
    REVISION.load(std::sync::atomic::Ordering::Relaxed)
}

/// What the authoritative game did to the block world this tick, for the
/// world's owner to apply.
#[derive(Clone, Copy, Debug)]
pub enum VoxelEvent {
    /// A bullet struck this block, with the damage it would have done: the
    /// weapon's, at that range, after penetration.
    Shot { block: [i32; 3], damage: f32 },
    /// An explosion went off here, in blocks.
    Explosion { center: [f64; 3], power: f32 },
    /// A bullet struck the mob with this key, with the damage it would have
    /// done at that range, from this block point.
    MobShot { key: u64, damage: f32, from: [f64; 3] },
    MobBlast { key: u64, damage: f32, center: [f64; 3], impact: f32, attacker: Option<u32>, slow_ticks: i32 },
    /// A bullet's path through the air, in blocks, up to what stopped it:
    /// blocks without collision on it (grass, flowers) take its damage.
    Ray { from: [f64; 3], to: [f64; 3], damage: f32 },
}

static EVENTS: std::sync::Mutex<Vec<VoxelEvent>> = std::sync::Mutex::new(Vec::new());

/// A bullet impact at map point `end` on a surface facing `normal`: the
/// block behind the surface.
pub fn push_shot(end: [f32; 3], normal: [f32; 3], damage: f32) {
    let Ok(world) = WORLD.read() else {
        return;
    };
    let Some(world) = world.as_ref() else {
        return;
    };
    let p = to_block(world.origin, end);
    // Into the surface, past the trace's pull-back.
    let n = [f64::from(normal[0]), f64::from(normal[2]), -f64::from(normal[1])];
    let block = std::array::from_fn(|k| (p[k] - n[k] * 0.05).floor() as i32);
    if let Ok(mut events) = EVENTS.lock() {
        events.push(VoxelEvent::Shot { block, damage });
    }
}

/// An explosion at map point `origin`.
pub fn push_explosion(origin: [f32; 3], power: f32) {
    let Ok(world) = WORLD.read() else {
        return;
    };
    let Some(world) = world.as_ref() else {
        return;
    };
    let center = to_block(world.origin, origin);
    if let Ok(mut events) = EVENTS.lock() {
        events.push(VoxelEvent::Explosion { center, power });
    }
}

/// The world's mobs as bullets see them: a key and a block-space box
/// `[min, max]` each.
static MOB_BOXES: RwLock<Vec<(u64, [f64; 6])>> = RwLock::new(Vec::new());

pub fn set_mob_boxes(boxes: Vec<(u64, [f64; 6])>) {
    if let Ok(mut held) = MOB_BOXES.write() {
        *held = boxes;
    }
}

/// The nearest mob on the map-space segment `start`..`end`: its key, the
/// distance to it in map units and how far up its box the bullet struck
/// (0 at the feet, 1 at the top).
pub(crate) fn mob_on_segment(start: [f32; 3], end: [f32; 3]) -> Option<(u64, f32, f32)> {
    let world = WORLD.read().ok()?;
    let origin = world.as_ref()?.origin;
    let a = to_block(origin, start);
    let b = to_block(origin, end);
    let d: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
    let boxes = MOB_BOXES.read().ok()?;
    let mut best: Option<(u64, f64, f64)> = None;
    for &(key, bb) in boxes.iter() {
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut hit = true;
        for k in 0..3 {
            if d[k].abs() < 1e-12 {
                if a[k] < bb[k] || a[k] > bb[k + 3] {
                    hit = false;
                    break;
                }
                continue;
            }
            let (u, v) = ((bb[k] - a[k]) / d[k], (bb[k + 3] - a[k]) / d[k]);
            t0 = t0.max(u.min(v));
            t1 = t1.min(u.max(v));
            if t0 > t1 {
                hit = false;
                break;
            }
        }
        if hit && best.is_none_or(|(_, t, _)| t0 < t) {
            let y = a[1] + d[1] * t0;
            let up = ((y - bb[1]) / (bb[4] - bb[1]).max(1e-6)).clamp(0.0, 1.0);
            best = Some((key, t0, up));
        }
    }
    let length = f64::from(
        ((end[0] - start[0]).powi(2) + (end[1] - start[1]).powi(2) + (end[2] - start[2]).powi(2)).sqrt(),
    );
    best.map(|(key, t, up)| (key, (t * length) as f32, up as f32))
}

/// A bullet's path from map point `start` to `end`.
pub(crate) fn push_ray(start: [f32; 3], end: [f32; 3], damage: f32) {
    let Ok(world) = WORLD.read() else {
        return;
    };
    let Some(world) = world.as_ref() else {
        return;
    };
    let (from, to) = (to_block(world.origin, start), to_block(world.origin, end));
    if let Ok(mut events) = EVENTS.lock() {
        events.push(VoxelEvent::Ray { from, to, damage });
    }
}

/// MW2's hit location for a strike this far up a mob's box, as a standing
/// soldier's body divides: head, neck, upper and lower torso, legs.
pub(crate) fn mob_hitloc(up: f32) -> u8 {
    match up {
        u if u > 0.87 => 2,
        u if u > 0.8 => 3,
        u if u > 0.62 => 4,
        u if u > 0.45 => 5,
        u if u > 0.22 => 12,
        _ => 14,
    }
}

/// The world's mobs as aim assist sees them: each box's centre in map
/// units and its half width.
/// Each mob's key and box in map space, mins then maxs.
pub fn mob_targets() -> Vec<(u64, [f32; 3], [f32; 3])> {
    let Ok(world) = WORLD.read() else {
        return Vec::new();
    };
    let Some(origin) = world.as_ref().map(|w| w.origin) else {
        return Vec::new();
    };
    let Ok(boxes) = MOB_BOXES.read() else {
        return Vec::new();
    };
    boxes
        .iter()
        .map(|(key, b)| {
            let a = to_map(origin, [b[0], b[1], b[2]]);
            let c = to_map(origin, [b[3], b[4], b[5]]);
            let mins = [a[0].min(c[0]), a[1].min(c[1]), a[2].min(c[2])];
            let maxs = [a[0].max(c[0]), a[1].max(c[1]), a[2].max(c[2])];
            (*key, mins, maxs)
        })
        .collect()
}

pub(crate) fn push_mob_shot(key: u64, damage: f32, from: [f32; 3]) {
    let Ok(world) = WORLD.read() else {
        return;
    };
    let Some(world) = world.as_ref() else {
        return;
    };
    let from = to_block(world.origin, from);
    if let Ok(mut events) = EVENTS.lock() {
        events.push(VoxelEvent::MobShot { key, damage, from });
    }
}

pub(crate) fn push_mob_blast(key: u64, damage: f32, origin: [f32; 3], impact: f32, attacker: Option<u32>, slow_ticks: i32) {
    let Ok(world) = WORLD.read() else {
        return;
    };
    let Some(world) = world.as_ref() else {
        return;
    };
    let center = to_block(world.origin, origin);
    if let Ok(mut events) = EVENTS.lock() {
        events.push(VoxelEvent::MobBlast { key, damage, center, impact, attacker, slow_ticks });
    }
}

/// Damage the world's mobs dealt the players: client, amount, and where it
/// came from in map space.
static PLAYER_DAMAGE: std::sync::Mutex<Vec<PlayerDamage>> = std::sync::Mutex::new(Vec::new());

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MinecraftDeath {
    pub message: String,
    pub killer: String,
}

impl MinecraftDeath {
    pub fn line(&self, victim: &str) -> String {
        self.message.replace("%1$s", victim)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlayerDamage {
    pub(crate) client: u32,
    pub(crate) amount: i32,
    pub(crate) from: Option<[f32; 3]>,
    pub(crate) death: Option<MinecraftDeath>,
}

pub fn push_player_damage(client: u32, amount: i32, from: Option<[f32; 3]>, death: Option<MinecraftDeath>) {
    if let Ok(mut damage) = PLAYER_DAMAGE.lock() {
        damage.push(PlayerDamage { client, amount, from, death });
    }
}

pub fn player_damage_pending() -> bool {
    PLAYER_DAMAGE.lock().is_ok_and(|damage| !damage.is_empty())
}

pub(crate) fn take_player_damage() -> Vec<PlayerDamage> {
    PLAYER_DAMAGE.lock().map(|mut d| std::mem::take(&mut *d)).unwrap_or_default()
}

static MINECRAFT_DEATHS: std::sync::Mutex<Vec<(u32, String)>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn push_minecraft_death(client: u32, killer: String) {
    if let Ok(mut deaths) = MINECRAFT_DEATHS.lock() {
        deaths.push((client, killer));
    }
}

pub fn take_minecraft_deaths() -> Vec<(u32, String)> {
    MINECRAFT_DEATHS.lock().map_or_else(|_| Vec::new(), |mut deaths| std::mem::take(&mut *deaths))
}

static TEAM_SWITCHES: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn push_team_switch(client: u32) {
    if let Ok(mut clients) = TEAM_SWITCHES.lock() {
        clients.push(client);
    }
}

pub fn take_team_switches() -> Vec<u32> {
    TEAM_SWITCHES.lock().map_or_else(|_| Vec::new(), |mut clients| std::mem::take(&mut *clients))
}

static DEATH_PROTECTED: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());
static TOTEM_POPS: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

pub fn set_death_protected(client: u32, protected: bool) {
    if let Ok(mut clients) = DEATH_PROTECTED.lock() {
        clients.retain(|&c| c != client);
        if protected {
            clients.push(client);
        }
    }
}

pub(crate) fn spend_death_protection(client: u32) -> bool {
    let spent = DEATH_PROTECTED.lock().is_ok_and(|mut clients| {
        let before = clients.len();
        clients.retain(|&c| c != client);
        clients.len() < before
    });
    if spent && let Ok(mut pops) = TOTEM_POPS.lock() {
        pops.push(client);
    }
    spent
}

pub fn take_totem_pops() -> Vec<u32> {
    TOTEM_POPS.lock().map_or_else(|_| Vec::new(), |mut pops| std::mem::take(&mut *pops))
}

pub fn take_events() -> Vec<VoxelEvent> {
    EVENTS.lock().map(|mut events| std::mem::take(&mut *events)).unwrap_or_default()
}

/// Sets one block's collision shape id, as `set_chunk` lays them out.
pub fn set_block_shape(x: i32, y: i32, z: i32, shape: u16) {
    if let Ok(mut world) = WORLD.write()
        && let Some(world) = world.as_mut()
        && let Some(chunk) = world.chunks.get_mut(&(x >> 4, z >> 4))
    {
        let ly = y - chunk.min_y;
        if ly >= 0 && ly < chunk.height {
            let index = ((ly * 16 + (z & 15)) * 16 + (x & 15)) as usize;
            if let Some(slot) = chunk.shapes.get_mut(index) {
                *slot = shape;
                bump_region([x, y, z], [x, y, z]);
            }
        }
    }
}

/// Starts replacing world collision for the map whose brush table is
/// `brushes`, with the block world's `origin` block at map origin.
pub fn activate(brushes: &[SimBrush], origin: [f64; 3], shapes: Vec<Vec<[f32; 6]>>) {
    if let Ok(mut world) = WORLD.write() {
        *world = Some(VoxelWorld {
            brushes: brushes.as_ptr() as usize,
            origin,
            chunks: HashMap::new(),
            fluids: vec![0; shapes.len()],
            shapes,
        });
    }
    set_level_switch(LevelSwitch::Never);
    bump();
}

pub fn switch_level(origin: [f64; 3]) {
    if let Ok(mut world) = WORLD.write()
        && let Some(world) = world.as_mut()
    {
        world.switch(origin);
    }
    let _ = take_events();
    set_mob_boxes(Vec::new());
    set_level_switch(LevelSwitch::Pending);
    bump();
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LevelSwitch {
    #[default]
    Never,
    Pending,
    At { ms: i32, models: u32 },
}

impl LevelSwitch {
    fn stamped(self, ms: i32, next_model: u32) -> Self {
        match self {
            Self::Pending => Self::At { ms, models: crate::script::host::entities::SPAWNED_PRESENCE_BASE.saturating_add(next_model) },
            other => other,
        }
    }

    fn fired_before(self, born_ms: i32) -> bool {
        match self {
            Self::Never => false,
            Self::Pending => true,
            Self::At { ms, .. } => born_ms < ms,
        }
    }

    fn spawned_before(self, model: crate::ScriptModelId) -> bool {
        let wire = model.to_wire();
        wire >= crate::script::host::entities::SPAWNED_PRESENCE_BASE
            && match self {
                Self::Never => false,
                Self::Pending => true,
                Self::At { models, .. } => wire < models,
            }
    }
}

static LEVEL_SWITCH: std::sync::Mutex<LevelSwitch> = std::sync::Mutex::new(LevelSwitch::Never);

fn set_level_switch(switch: LevelSwitch) {
    if let Ok(mut held) = LEVEL_SWITCH.lock() {
        *held = switch;
    }
}

fn level_switch() -> LevelSwitch {
    LEVEL_SWITCH.lock().map_or(LevelSwitch::Never, |held| *held)
}

pub(crate) fn stamp_level_switch(ms: i32, next_model: u32) {
    if let Ok(mut held) = LEVEL_SWITCH.lock() {
        *held = held.stamped(ms, next_model);
    }
}

pub(crate) fn fired_before_switch(born_ms: i32) -> bool {
    level_switch().fired_before(born_ms)
}

pub(crate) fn spawned_before_switch(inflictor: Option<crate::ScriptModelId>) -> bool {
    inflictor.is_some_and(|model| level_switch().spawned_before(model))
}

const NEEDS_SKY: [&str; 11] = [
    "airdrop_marker",
    "airdrop_mega_marker",
    "airdrop_sentry_marker",
    "killstreak_ac130",
    "killstreak_harrier_airstrike",
    "killstreak_helicopter",
    "killstreak_helicopter_flares",
    "killstreak_helicopter_minigun",
    "killstreak_precision_airstrike",
    "killstreak_predator_missile",
    "killstreak_stealth_airstrike",
];

pub fn needs_sky(weapon: &str) -> bool {
    let weapon = weapon.to_ascii_lowercase();
    NEEDS_SKY.contains(&weapon.strip_suffix("_mp").unwrap_or(&weapon))
}

pub fn refused_without_sky(pm_flags: u32, weapon: &str) -> bool {
    pm_flags & playerstate_iw4::pm_flags::MC_NO_SKY != 0 && needs_sky(weapon)
}

pub fn deactivate() {
    if let Ok(mut world) = WORLD.write() {
        *world = None;
    }
    set_level_switch(LevelSwitch::Never);
    let _ = take_events();
    let _ = take_player_damage();
    let _ = take_totem_pops();
    let _ = take_minecraft_deaths();
    let _ = take_team_switches();
    if let Ok(mut clients) = DEATH_PROTECTED.lock() {
        clients.clear();
    }
    set_mob_boxes(Vec::new());
}

/// Adds shape ids to the table and returns the first new id.
pub fn add_shapes(more: Vec<Vec<[f32; 6]>>) -> Option<u16> {
    let mut world = WORLD.write().ok()?;
    let world = world.as_mut()?;
    let first = world.shapes.len() as u16;
    world.fluids.extend(std::iter::repeat_n(0, more.len()));
    world.shapes.extend(more);
    Some(first)
}

pub const WATER: u16 = 0x10;
pub const LAVA: u16 = 0x20;
pub const FLUID_AMOUNT: u16 = 0x0f;
pub const CLIMBABLE: u16 = 0x40;
pub const SCAFFOLDING: u16 = 0x80;
pub const SCAFFOLDING_BOTTOM: u16 = 0x100;
const SCAFFOLDING_SLAB: [[f32; 6]; 1] = [[0.0, 0.0, 0.0, 1.0, 0.125, 1.0]];
const ABOVE_EPSILON: f64 = 9.999_999_747_378_752e-6;
pub const STUCK_BERRY_BUSH: u16 = 0x200;
pub const STUCK_COBWEB: u16 = 0x400;
const BERRY_BUSH_SLOWDOWN: [f32; 3] = [0.8, 0.8, 0.75];
const COBWEB_SLOWDOWN: [f32; 3] = [0.25, 0.25, 0.05];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mover {
    pub feet: f32,
    pub descending: bool,
}

impl Mover {
    pub fn of(ps: &playerstate_iw4::PlayerState, cmd: &playerstate_iw4::UserCmd) -> Self {
        Self { feet: ps.origin[2], descending: movement_iw4::sneaking(ps, cmd) }
    }
}

pub fn add_fluid_shape(boxes: Vec<[f32; 6]>, fluid: u16) -> Option<u16> {
    let mut world = WORLD.write().ok()?;
    let world = world.as_mut()?;
    let id = world.shapes.len() as u16;
    world.shapes.push(boxes);
    world.fluids.push(fluid);
    Some(id)
}

pub fn set_chunk(x: i32, z: i32, chunk: VoxelChunk) {
    if let Ok(mut world) = WORLD.write()
        && let Some(world) = world.as_mut()
    {
        world.chunks.insert((x, z), chunk);
    }
    let (min, max) = chunk_region(x, z);
    bump_region(min, max);
}

pub fn remove_chunk(x: i32, z: i32) {
    if let Ok(mut world) = WORLD.write()
        && let Some(world) = world.as_mut()
    {
        world.chunks.remove(&(x, z));
    }
    let (min, max) = chunk_region(x, z);
    bump_region(min, max);
}

/// The faces of the block world's collision boxes within `radius` blocks
/// across and `depth` blocks up or down of map point `centre`, as triangles
/// in map units wound counterclockwise seen from outside. A face against a
/// full neighbouring block is left out; blocks without collision (grass,
/// flowers) have none.
pub fn collision_triangles(centre: [f32; 3], radius: i32, depth: i32) -> Vec<[[f32; 3]; 3]> {
    let Ok(world) = WORLD.read() else {
        return Vec::new();
    };
    let Some(world) = world.as_ref() else {
        return Vec::new();
    };
    let full: Vec<bool> = world.shapes.iter().map(|b| b.len() == 1 && b[0] == [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]).collect();
    let is_full = |x: i32, y: i32, z: i32| -> bool {
        let Some(chunk) = world.chunks.get(&(x >> 4, z >> 4)) else {
            return false;
        };
        let ly = y - chunk.min_y;
        if ly < 0 || ly >= chunk.height {
            return false;
        }
        let id = chunk.shapes[((ly * 16 + (z & 15)) * 16 + (x & 15)) as usize];
        full.get(usize::from(id)).copied().unwrap_or(false)
    };
    let c = to_block(world.origin, centre);
    let (cx, cy, cz) = (c[0].floor() as i32, c[1].floor() as i32, c[2].floor() as i32);
    let mut out = Vec::new();
    for x in cx - radius..=cx + radius {
        for z in cz - radius..=cz + radius {
            for y in cy - depth..=cy + depth {
                let boxes = world.shape_at(x, y, z);
                if boxes.is_empty() {
                    continue;
                }
                let base = [f64::from(x), f64::from(y), f64::from(z)];
                for b in boxes {
                    let lo = [f64::from(b[0]), f64::from(b[1]), f64::from(b[2])];
                    let hi = [f64::from(b[3]), f64::from(b[4]), f64::from(b[5])];
                    for axis in 0..3 {
                        // Tangents whose cross product is the axis.
                        let (u, v) = [(1, 2), (2, 0), (0, 1)][axis];
                        for high in [false, true] {
                            let at = if high { hi[axis] } else { lo[axis] };
                            let mut step = [0; 3];
                            step[axis] = if high { 1 } else { -1 };
                            let on_edge = if high { at >= 1.0 } else { at <= 0.0 };
                            if on_edge && is_full(x + step[0], y + step[1], z + step[2]) {
                                continue;
                            }
                            let corner = |a: f64, b: f64| {
                                let mut p = [0.0; 3];
                                p[axis] = at;
                                p[u] = a;
                                p[v] = b;
                                to_map(world.origin, std::array::from_fn(|k| base[k] + p[k]))
                            };
                            let mut quad = [
                                corner(lo[u], lo[v]),
                                corner(hi[u], lo[v]),
                                corner(hi[u], hi[v]),
                                corner(lo[u], hi[v]),
                            ];
                            if !high {
                                quad.reverse();
                            }
                            out.push([quad[0], quad[1], quad[2]]);
                            out.push([quad[0], quad[2], quad[3]]);
                        }
                    }
                }
            }
        }
    }
    out
}

/// Whether traces against `brushes` go to the block world.
pub(crate) fn active_for(brushes: &[SimBrush]) -> bool {
    WORLD
        .read()
        .ok()
        .is_some_and(|world| world.as_ref().is_some_and(|w| w.brushes == brushes.as_ptr() as usize))
}

/// Whether any block world is standing in for a map.
pub fn active() -> bool {
    WORLD.read().ok().is_some_and(|world| world.is_some())
}

pub fn chunks_ready_around(cx: i32, cz: i32, radius: i32) -> bool {
    WORLD.read().ok().is_some_and(|world| world.as_ref().is_some_and(|w| w.has_chunks_around(cx, cz, radius)))
}

pub const RESPAWN_RADIUS: i32 = 10;
const SPAWN_SEARCH_UP: i32 = 8;
const SPAWN_SEARCH_DOWN: i32 = 16;

pub fn awaiting_ground(origin: [f32; 3]) -> bool {
    WORLD.read().ok().is_some_and(|world| world.as_ref().is_some_and(|w| w.awaits_ground(origin)))
}

pub(crate) fn spawn_clear_of(taken: impl Fn([f32; 3]) -> bool) -> [f32; 3] {
    let spawn = [0.0; 3];
    if !taken(spawn) {
        return spawn;
    }
    WORLD.read().ok().and_then(|world| world.as_ref().and_then(|w| w.free_spawn_spot(&taken))).unwrap_or(spawn)
}

pub fn spawn_origin(sessionstate: &str) -> Option<[f32; 3]> {
    spawn_origin_for(active(), sessionstate)
}

fn spawn_origin_for(active: bool, sessionstate: &str) -> Option<[f32; 3]> {
    (active && sessionstate != "dead").then_some([0.0; 3])
}

/// Map point to block point.
pub fn to_block(origin: [f64; 3], p: [f32; 3]) -> [f64; 3] {
    let s = f64::from(BLOCK);
    [
        origin[0] + f64::from(p[0]) / s,
        origin[1] + f64::from(p[2]) / s,
        origin[2] - f64::from(p[1]) / s,
    ]
}

/// Block point to map point.
pub fn to_map(origin: [f64; 3], b: [f64; 3]) -> [f32; 3] {
    let s = f64::from(BLOCK);
    [
        ((b[0] - origin[0]) * s) as f32,
        (-(b[2] - origin[2]) * s) as f32,
        ((b[1] - origin[1]) * s) as f32,
    ]
}

impl VoxelWorld {
    fn has_chunks_around(&self, cx: i32, cz: i32, radius: i32) -> bool {
        (cx - radius..=cx + radius).all(|x| (cz - radius..=cz + radius).all(|z| self.chunks.contains_key(&(x, z))))
    }

    fn switch(&mut self, origin: [f64; 3]) {
        self.origin = origin;
        self.chunks.clear();
        self.shapes = vec![Vec::new()];
        self.fluids = vec![0];
    }

    fn awaits_ground(&self, origin: [f32; 3]) -> bool {
        let feet = to_block(self.origin, origin).map(|c| c.floor() as i32);
        !self.has_chunks_around(feet[0] >> 4, feet[2] >> 4, 0)
    }

    fn free_spawn_spot(&self, taken: &impl Fn([f32; 3]) -> bool) -> Option<[f32; 3]> {
        let mins = [-HULL_HALF_WIDTH, -HULL_HALF_WIDTH, 0.0];
        let maxs = [HULL_HALF_WIDTH, HULL_HALF_WIDTH, STANCE_HEIGHTS[0]];
        let spawn = self.origin.map(|c| c.floor() as i32);
        (1..=RESPAWN_RADIUS)
            .flat_map(|r| (-r..=r).flat_map(move |dx| (-r..=r).map(move |dz| (dx, dz))).filter(move |&(dx, dz)| dx.abs().max(dz.abs()) == r))
            .find_map(|(dx, dz)| {
                let (x, z) = (spawn[0] + dx, spawn[2] + dz);
                let (y, top) = (spawn[1] - SPAWN_SEARCH_DOWN..=spawn[1] + SPAWN_SEARCH_UP)
                    .rev()
                    .find_map(|y| self.shape_at(x, y, z).iter().map(|b| b[4]).reduce(f32::max).map(|top| (y, top)))?;
                let feet = [f64::from(x) + 0.5, f64::from(y) + f64::from(top), f64::from(z) + 0.5];
                let spot = to_map(self.origin, feet);
                let clear = trace_in(self, spot, spot, mins, maxs, None).startsolid == 0;
                let wet = (0..2).any(|up| self.fluid_at(x, feet[1].floor() as i32 + up, z) != 0);
                (clear && !wet && !taken(spot)).then_some(spot)
            })
    }

    fn shape_at(&self, x: i32, y: i32, z: i32) -> &[[f32; 6]] {
        let Some(chunk) = self.chunks.get(&(x >> 4, z >> 4)) else {
            return &[];
        };
        let ly = y - chunk.min_y;
        if ly < 0 || ly >= chunk.height {
            return &[];
        }
        let index = ((ly * 16 + (z & 15)) * 16 + (x & 15)) as usize;
        let id = chunk.shapes.get(index).copied().unwrap_or(0);
        self.shapes.get(usize::from(id)).map_or(&[], Vec::as_slice)
    }

    fn shape_for(&self, x: i32, y: i32, z: i32, mover: Option<Mover>) -> &[[f32; 6]] {
        let boxes = self.shape_at(x, y, z);
        let Some(mover) = mover else {
            return boxes;
        };
        let flags = self.fluid_at(x, y, z);
        if flags & SCAFFOLDING == 0 {
            return boxes;
        }
        let bottom = self.origin[1] + f64::from(mover.feet) / f64::from(BLOCK);
        if bottom > f64::from(y) + 1.0 - ABOVE_EPSILON && !mover.descending {
            boxes
        } else if flags & SCAFFOLDING_BOTTOM != 0 && bottom > f64::from(y) - ABOVE_EPSILON {
            &SCAFFOLDING_SLAB
        } else {
            &[]
        }
    }

    fn climb_sample(&self, feet: [f64; 3]) -> movement_iw4::ClimbSample {
        let flags = self.fluid_at(feet[0].floor() as i32, feet[1].floor() as i32, feet[2].floor() as i32);
        movement_iw4::ClimbSample { on: flags & CLIMBABLE != 0, scaffolding: flags & SCAFFOLDING != 0 }
    }
}

struct Sweep {
    first: Option<(f64, [f64; 3])>,
    start_solid: bool,
    end_solid: bool,
}

impl VoxelWorld {
    /// A block-space box `[lo, hi]` about `a` moved to `b`: the first hit as
    /// a fraction of the move and its normal, or whether it starts inside.
    fn sweep(&self, a: [f64; 3], b: [f64; 3], lo: [f64; 3], hi: [f64; 3], test_start: bool, mover: Option<Mover>) -> Sweep {
        const INSIDE: f64 = 1e-4;
        let delta: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
        let reach_lo: [i32; 3] = std::array::from_fn(|k| (a[k].min(b[k]) + lo[k] - 1.0).floor() as i32);
        let reach_hi: [i32; 3] = std::array::from_fn(|k| (a[k].max(b[k]) + hi[k] + 1.0).floor() as i32);
        let mut out = Sweep {
            first: None,
            start_solid: false,
            end_solid: false,
        };
        let mut best_t = f64::MAX;
        for y in reach_lo[1]..=reach_hi[1] {
            for z in reach_lo[2]..=reach_hi[2] {
                for x in reach_lo[0]..=reach_hi[0] {
                    for shape in self.shape_for(x, y, z, mover) {
                        // The shape grown by the moving box.
                        let bmin = [
                            f64::from(x) + f64::from(shape[0]) - hi[0],
                            f64::from(y) + f64::from(shape[1]) - hi[1],
                            f64::from(z) + f64::from(shape[2]) - hi[2],
                        ];
                        let bmax = [
                            f64::from(x) + f64::from(shape[3]) - lo[0],
                            f64::from(y) + f64::from(shape[4]) - lo[1],
                            f64::from(z) + f64::from(shape[5]) - lo[2],
                        ];
                        let inside = |p: [f64; 3]| {
                            (0..3).all(|k| p[k] > bmin[k] + INSIDE && p[k] < bmax[k] - INSIDE)
                        };
                        if inside(a) {
                            if test_start {
                                out.start_solid = true;
                                out.end_solid |= inside(b);
                            }
                            continue;
                        }
                        let (mut t_in, mut t_out, mut axis) = (0.0f64, 1.0f64, None);
                        let mut hit = true;
                        for k in 0..3 {
                            if delta[k].abs() < 1e-12 {
                                if a[k] <= bmin[k] || a[k] >= bmax[k] {
                                    hit = false;
                                    break;
                                }
                                continue;
                            }
                            let (t0, t1) = ((bmin[k] - a[k]) / delta[k], (bmax[k] - a[k]) / delta[k]);
                            let (near, far) = (t0.min(t1), t0.max(t1));
                            if near > t_in {
                                t_in = near;
                                axis = Some(k);
                            }
                            t_out = t_out.min(far);
                            if t_in >= t_out {
                                hit = false;
                                break;
                            }
                        }
                        if hit
                            && let Some(k) = axis
                            && t_in < best_t
                        {
                            best_t = t_in;
                            let mut normal = [0.0; 3];
                            normal[k] = -delta[k].signum();
                            out.first = Some((t_in, normal));
                        }
                    }
                }
            }
        }
        out
    }
}

/// A box swept from `start` to `end` in map space against the block world.
pub(crate) fn trace(
    start: [f32; 3],
    end: [f32; 3],
    mins: [f32; 3],
    maxs: [f32; 3],
) -> trace_iw4::Trace {
    trace_as(start, end, mins, maxs, None)
}

pub(crate) fn trace_as(start: [f32; 3], end: [f32; 3], mins: [f32; 3], maxs: [f32; 3], mover: Option<Mover>) -> trace_iw4::Trace {
    let open = trace_iw4::Trace {
        fraction: 1.0,
        endpos: end,
        ..trace_iw4::Trace::default()
    };
    let Ok(guard) = WORLD.read() else {
        return open;
    };
    let Some(world) = guard.as_ref() else {
        return open;
    };
    trace_in(world, start, end, mins, maxs, mover)
}

pub fn camera_reach(eye: [f32; 3], back: [f32; 3], distance: f32) -> f32 {
    let Ok(guard) = WORLD.read() else {
        return distance;
    };
    match guard.as_ref() {
        Some(world) => camera_reach_in(world, eye, back, distance),
        None => distance,
    }
}

fn camera_reach_in(world: &VoxelWorld, eye: [f32; 3], back: [f32; 3], distance: f32) -> f32 {
    let mut reach = distance;
    for corner in 0..8 {
        let offset = [corner & 1, (corner >> 1) & 1, (corner >> 2) & 1].map(|bit| (bit * 2 - 1) as f32 * 0.1 * BLOCK);
        let start: [f32; 3] = std::array::from_fn(|k| eye[k] + offset[k]);
        let end: [f32; 3] = std::array::from_fn(|k| start[k] + back[k] * reach);
        let hit = trace_in(world, start, end, [0.0; 3], [0.0; 3], None);
        if hit.fraction >= 1.0 {
            continue;
        }
        let at: [f32; 3] = std::array::from_fn(|k| start[k] + (end[k] - start[k]) * hit.fraction);
        let dist = (0..3).map(|k| (at[k] - eye[k]) * (at[k] - eye[k])).sum::<f32>().sqrt();
        if dist < reach {
            reach = dist;
        }
    }
    reach
}

pub(crate) fn ground_within(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3], drop: f32, mover: Option<Mover>) -> bool {
    WORLD.read().ok().is_some_and(|guard| guard.as_ref().is_some_and(|world| ground_in(world, origin, mins, maxs, drop, mover)))
}

fn ground_in(world: &VoxelWorld, origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3], drop: f32, mover: Option<Mover>) -> bool {
    let below = [mins[0], mins[1], mins[2] - drop];
    let feet = [maxs[0], maxs[1], mins[2]];
    trace_in(world, origin, origin, below, feet, mover).startsolid != 0
}

fn trace_in(world: &VoxelWorld, start: [f32; 3], end: [f32; 3], mins: [f32; 3], maxs: [f32; 3], mover: Option<Mover>) -> trace_iw4::Trace {
    let open = trace_iw4::Trace {
        fraction: 1.0,
        endpos: end,
        ..trace_iw4::Trace::default()
    };
    let s = f64::from(BLOCK);
    let a = to_block(world.origin, start);
    let b = to_block(world.origin, end);
    // The box in block space: map X, Z and -Y extents.
    let lo = [
        f64::from(mins[0]) / s,
        f64::from(mins[2]) / s,
        -f64::from(maxs[1]) / s,
    ];
    let hi = [
        f64::from(maxs[0]) / s,
        f64::from(maxs[2]) / s,
        -f64::from(mins[1]) / s,
    ];
    let delta = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    // Long traces (bullets) are tested in short segments, nearest first, so
    // the cells tested stay near the ray.
    let segments = (length / 4.0).ceil().max(1.0) as usize;
    let mut best_t = 1.0f64;
    let mut best_normal = [0.0f64; 3];
    let mut start_solid = false;
    let mut end_solid = false;
    for segment in 0..segments {
        let (t0, t1) = (
            segment as f64 / segments as f64,
            (segment + 1) as f64 / segments as f64,
        );
        let sa: [f64; 3] = std::array::from_fn(|k| a[k] + delta[k] * t0);
        let sb: [f64; 3] = std::array::from_fn(|k| a[k] + delta[k] * t1);
        let hit = world.sweep(sa, sb, lo, hi, segment == 0, mover);
        if segment == 0 && hit.start_solid {
            start_solid = true;
            end_solid = segments == 1 && hit.end_solid;
            break;
        }
        if let Some((t, normal)) = hit.first {
            best_t = t0 + (t1 - t0) * t;
            best_normal = normal;
            break;
        }
    }
    if start_solid {
        return trace_iw4::Trace {
            fraction: 0.0,
            endpos: start,
            startsolid: 1,
            allsolid: u8::from(end_solid),
            contents: SOLID,
            surface_flags: STONE_SURFACE,
            hit_type: trace_iw4::HITTYPE_ENTITY,
            hit_id: trace_iw4::ENTITYNUM_WORLD,
            ..trace_iw4::Trace::default()
        };
    }
    if best_t >= 1.0 {
        return open;
    }
    let pulled = if length > 0.0 {
        (best_t - f64::from(SURFACE_CLIP_EPSILON) / (length * s)).max(0.0)
    } else {
        0.0
    };
    let fraction = pulled as f32;
    let endpos = std::array::from_fn(|k| start[k] + (end[k] - start[k]) * fraction);
    let n = best_normal;
    trace_iw4::Trace {
        fraction,
        endpos,
        normal: [n[0] as f32, -(n[2] as f32), n[1] as f32],
        contents: SOLID,
        surface_flags: STONE_SURFACE,
        walkable: u8::from(n[1] > 0.7),
        hit_type: trace_iw4::HITTYPE_ENTITY,
        hit_id: trace_iw4::ENTITYNUM_WORLD,
        ..trace_iw4::Trace::default()
    }
}

pub const HULL_HALF_WIDTH: f32 = 0.3 * BLOCK;
pub const STANCE_HEIGHTS: [f32; 3] = [1.8 * BLOCK, 1.5 * BLOCK, movement_iw4::PRONE_MAXS_Z];
pub const DAMAGE_PER_HARDNESS: f32 = 160.0;
const PUSH_GAP: f64 = 1e-3;
const STEP_OUT: f64 = 1.05;
const SIDE_OUT: f64 = 1.0;
const CLIMB_OUT: f64 = 3.0;
const WALL_REACH: f32 = 0.05;

pub fn player_hull(ps: &playerstate_iw4::PlayerState) -> ([f32; 3], [f32; 3]) {
    let height = match movement_iw4::stance_surface_type(ps) {
        movement_iw4::StanceSurface::Stand => STANCE_HEIGHTS[0],
        movement_iw4::StanceSurface::Prone => STANCE_HEIGHTS[2],
        movement_iw4::StanceSurface::Crouch | movement_iw4::StanceSurface::LastStand => STANCE_HEIGHTS[1],
    };
    (
        [-HULL_HALF_WIDTH, -HULL_HALF_WIDTH, 0.0],
        [HULL_HALF_WIDTH, HULL_HALF_WIDTH, height],
    )
}

pub fn move_bounds(bounds: movement_iw4::MoveBounds) -> movement_iw4::MoveBounds {
    movement_iw4::MoveBounds {
        mins: [-HULL_HALF_WIDTH, -HULL_HALF_WIDTH, bounds.mins[2]],
        maxs: [HULL_HALF_WIDTH, HULL_HALF_WIDTH, STANCE_HEIGHTS[0]],
        heights: STANCE_HEIGHTS,
        ..bounds
    }
}

pub fn area_bounds(bounds: movement_iw4::MoveBounds) -> movement_iw4::MoveBounds {
    if bounds.heights != STANCE_HEIGHTS {
        return bounds;
    }
    let stance = STANCE_HEIGHTS.iter().position(|&h| h == bounds.maxs[2]).unwrap_or(0);
    let (mins, maxs) = (crate::bullet_collision::PLAYER_MINS, crate::bullet_collision::PLAYER_MAXS);
    movement_iw4::MoveBounds {
        mins: [mins[0], mins[1], bounds.mins[2]],
        maxs: [maxs[0], maxs[1], movement_iw4::STANCE_HEIGHTS[stance]],
        heights: movement_iw4::STANCE_HEIGHTS,
        ..bounds
    }
}

fn block_bounds(mins: [f32; 3], maxs: [f32; 3]) -> ([f64; 3], [f64; 3]) {
    let s = f64::from(BLOCK);
    (
        [f64::from(mins[0]) / s, f64::from(mins[2]) / s, -f64::from(maxs[1]) / s],
        [f64::from(maxs[0]) / s, f64::from(maxs[2]) / s, -f64::from(mins[1]) / s],
    )
}

pub fn body_box(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> Option<[f64; 6]> {
    let world = WORLD.read().ok()?;
    let at = to_block(world.as_ref()?.origin, origin);
    let (lo, hi) = block_bounds(mins, maxs);
    Some([at[0] + lo[0], at[1] + lo[1], at[2] + lo[2], at[0] + hi[0], at[1] + hi[1], at[2] + hi[2]])
}

pub fn boxes_meet(a: [f64; 6], b: [f64; 6]) -> bool {
    (0..3).all(|k| a[k] < b[k + 3] && b[k] < a[k + 3])
}

pub(crate) fn push_out(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3], mover: Option<Mover>) -> Option<[f32; 3]> {
    let guard = WORLD.read().ok()?;
    let world = guard.as_ref()?;
    let at = to_block(world.origin, origin);
    let (lo, hi) = block_bounds(mins, maxs);
    let body_lo = std::array::from_fn(|k| at[k] + lo[k]);
    let body_hi = std::array::from_fn(|k| at[k] + hi[k]);
    let d = world.way_out(body_lo, body_hi, mover)?;
    Some(to_map(world.origin, std::array::from_fn(|k| at[k] + d[k])))
}

impl VoxelWorld {
    fn overlapping(&self, lo: [f64; 3], hi: [f64; 3], mover: Option<Mover>) -> Vec<[f64; 6]> {
        const INSIDE: f64 = 1e-4;
        let mut out = Vec::new();
        for y in lo[1].floor() as i32 - 1..=hi[1].floor() as i32 {
            for z in lo[2].floor() as i32 - 1..=hi[2].floor() as i32 {
                for x in lo[0].floor() as i32 - 1..=hi[0].floor() as i32 {
                    for shape in self.shape_for(x, y, z, mover) {
                        let cell = [f64::from(x), f64::from(y), f64::from(z)];
                        let b: [f64; 6] = std::array::from_fn(|k| cell[k % 3] + f64::from(shape[k]));
                        if (0..3).all(|k| b[k + 3] - lo[k] > INSIDE && hi[k] - b[k] > INSIDE) {
                            out.push(b);
                        }
                    }
                }
            }
        }
        out
    }

    fn escape(&self, lo: [f64; 3], hi: [f64; 3], axis: usize, sign: f64, limit: f64, mover: Option<Mover>) -> Option<f64> {
        let mut d = 0.0;
        loop {
            let l: [f64; 3] = std::array::from_fn(|k| if k == axis { lo[k] + sign * d } else { lo[k] });
            let h: [f64; 3] = std::array::from_fn(|k| if k == axis { hi[k] + sign * d } else { hi[k] });
            let inside = self.overlapping(l, h, mover);
            if inside.is_empty() {
                return Some(d);
            }
            let need = inside
                .iter()
                .map(|b| if sign > 0.0 { b[axis + 3] - lo[axis] } else { hi[axis] - b[axis] })
                .fold(f64::MIN, f64::max)
                + PUSH_GAP;
            if need <= d || need > limit {
                return None;
            }
            d = need;
        }
    }

    fn way_out(&self, lo: [f64; 3], hi: [f64; 3], mover: Option<Mover>) -> Option<[f64; 3]> {
        let inside = self.overlapping(lo, hi, mover);
        if inside.is_empty() {
            return None;
        }
        let along = |axis: usize, sign: f64, d: f64| {
            let mut v = [0.0; 3];
            v[axis] = sign * d;
            v
        };
        let middle = (lo[1] + hi[1]) * 0.5;
        let ceiling = inside.iter().any(|b| b[1] > middle);
        let mut best = if ceiling {
            self.escape(lo, hi, 1, -1.0, SIDE_OUT, mover).map(|d| (d, along(1, -1.0, d)))
        } else {
            self.escape(lo, hi, 1, 1.0, STEP_OUT, mover).map(|d| (d, along(1, 1.0, d)))
        };
        for (axis, sign) in [(0, 1.0), (0, -1.0), (2, 1.0), (2, -1.0)] {
            if let Some(d) = self.escape(lo, hi, axis, sign, SIDE_OUT, mover)
                && best.is_none_or(|(b, _)| d < b)
            {
                best = Some((d, along(axis, sign, d)));
            }
        }
        best.map(|(_, v)| v)
            .or_else(|| self.escape(lo, hi, 1, 1.0, CLIMB_OUT, mover).filter(|_| !ceiling).map(|d| along(1, 1.0, d)))
    }

    fn fluid_at(&self, x: i32, y: i32, z: i32) -> u16 {
        let Some(chunk) = self.chunks.get(&(x >> 4, z >> 4)) else {
            return 0;
        };
        let ly = y - chunk.min_y;
        if ly < 0 || ly >= chunk.height {
            return 0;
        }
        let id = chunk.shapes.get(((ly * 16 + (z & 15)) * 16 + (x & 15)) as usize).copied().unwrap_or(0);
        self.fluids.get(usize::from(id)).copied().unwrap_or(0)
    }

    fn fluid_top(&self, x: i32, y: i32, z: i32, kind: u16) -> Option<f64> {
        let fluid = self.fluid_at(x, y, z);
        if fluid & kind == 0 {
            return None;
        }
        let covered = self.fluid_at(x, y + 1, z) & kind != 0;
        Some(f64::from(y) + if covered { 1.0 } else { f64::from(fluid & FLUID_AMOUNT) / 9.0 })
    }

    fn fluid_covers(&self, point: [f64; 3], kind: u16) -> bool {
        self.fluid_top(point[0].floor() as i32, point[1].floor() as i32, point[2].floor() as i32, kind)
            .is_some_and(|top| top >= point[1])
    }

    fn fluid_depth(&self, at: [f64; 3], lo: [f64; 3], hi: [f64; 3], kind: u16) -> f64 {
        const SHRINK: f64 = 0.001;
        let min: [f64; 3] = std::array::from_fn(|k| at[k] + lo[k] + SHRINK);
        let max: [f64; 3] = std::array::from_fn(|k| at[k] + hi[k] - SHRINK);
        let mut depth = 0.0f64;
        for x in min[0].floor() as i32..max[0].ceil() as i32 {
            for y in min[1].floor() as i32..max[1].ceil() as i32 {
                for z in min[2].floor() as i32..max[2].ceil() as i32 {
                    if let Some(top) = self.fluid_top(x, y, z, kind)
                        && top >= min[1]
                    {
                        depth = depth.max(top - min[1]);
                    }
                }
            }
        }
        depth
    }

    fn water_sample(&self, at: [f64; 3], lo: [f64; 3], hi: [f64; 3], eye_height: f64) -> movement_iw4::WaterSample {
        const EYE_DROP: f64 = 0.111_111_11;
        const HEAD: f64 = 0.9;
        let depth = self.fluid_depth(at, lo, hi, WATER);
        let (x, z) = (at[0].floor() as i32, at[2].floor() as i32);
        let eye_y = at[1] + eye_height - EYE_DROP;
        let eye = self.fluid_top(x, eye_y.floor() as i32, z, WATER).is_some_and(|top| top > eye_y);
        let head = self.fluid_at(x, (at[1] + HEAD).floor() as i32, z) & WATER != 0;
        movement_iw4::WaterSample {
            depth: (depth * f64::from(BLOCK)) as f32,
            eye,
            head,
        }
    }

    fn stuck_in(&self, at: [f64; 3], lo: [f64; 3], hi: [f64; 3]) -> Option<[f32; 3]> {
        let shrink = f64::from(1.0e-5_f32);
        let min: [f64; 3] = std::array::from_fn(|k| at[k] + lo[k] + shrink);
        let max: [f64; 3] = std::array::from_fn(|k| at[k] + hi[k] - shrink);
        let mut touched = 0;
        for x in min[0].floor() as i32..max[0].ceil() as i32 {
            for y in min[1].floor() as i32..max[1].ceil() as i32 {
                for z in min[2].floor() as i32..max[2].ceil() as i32 {
                    touched |= self.fluid_at(x, y, z);
                }
            }
        }
        if touched & STUCK_COBWEB != 0 {
            Some(COBWEB_SLOWDOWN)
        } else if touched & STUCK_BERRY_BUSH != 0 {
            Some(BERRY_BUSH_SLOWDOWN)
        } else {
            None
        }
    }
}

pub fn water(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3], view_height: f32) -> movement_iw4::WaterSample {
    let Ok(guard) = WORLD.read() else {
        return movement_iw4::WaterSample::default();
    };
    let Some(world) = guard.as_ref() else {
        return movement_iw4::WaterSample::default();
    };
    let (lo, hi) = block_bounds(mins, maxs);
    world.water_sample(to_block(world.origin, origin), lo, hi, f64::from(view_height / BLOCK))
}

pub fn lava(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> f32 {
    let Ok(guard) = WORLD.read() else {
        return 0.0;
    };
    let Some(world) = guard.as_ref() else {
        return 0.0;
    };
    let (lo, hi) = block_bounds(mins, maxs);
    (world.fluid_depth(to_block(world.origin, origin), lo, hi, LAVA) * f64::from(BLOCK)) as f32
}

pub fn climb(origin: [f32; 3]) -> movement_iw4::ClimbSample {
    let Ok(guard) = WORLD.read() else {
        return movement_iw4::ClimbSample::default();
    };
    guard.as_ref().map_or_else(movement_iw4::ClimbSample::default, |world| world.climb_sample(to_block(world.origin, origin)))
}

pub fn eye_in_lava(origin: [f32; 3], view_height: f32) -> bool {
    let Ok(guard) = WORLD.read() else {
        return false;
    };
    let Some(world) = guard.as_ref() else {
        return false;
    };
    let mut eye = to_block(world.origin, origin);
    eye[1] += f64::from(view_height / BLOCK);
    world.fluid_covers(eye, LAVA)
}

pub fn stuck(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> Option<[f32; 3]> {
    let guard = WORLD.read().ok()?;
    let world = guard.as_ref()?;
    let (lo, hi) = block_bounds(mins, maxs);
    world.stuck_in(to_block(world.origin, origin), lo, hi)
}

pub fn touches_wall(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> bool {
    let Ok(guard) = WORLD.read() else {
        return false;
    };
    guard.as_ref().is_some_and(|world| touches_wall_in(world, origin, mins, maxs))
}

fn touches_wall_in(world: &VoxelWorld, origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> bool {
    let reach = WALL_REACH * BLOCK;
    let wide = [mins[0] - reach, mins[1] - reach, mins[2] + reach];
    let tall = [maxs[0] + reach, maxs[1] + reach, maxs[2] - reach];
    trace_in(world, origin, origin, wide, tall, None).startsolid != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: [f32; 6] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    const FENCE: [f32; 6] = [0.375, 0.0, 0.375, 0.625, 1.5, 0.625];

    fn world(blocks: &[([i32; 3], u16)]) -> VoxelWorld {
        let mut chunk = VoxelChunk { min_y: -16, height: 64, shapes: vec![0; 64 * 256] };
        for &([x, y, z], shape) in blocks {
            chunk.shapes[(((y + 16) * 16 + z) * 16 + x) as usize] = shape;
        }
        let mut chunks = HashMap::new();
        chunks.insert((0, 0), chunk);
        VoxelWorld {
            brushes: 0,
            origin: [0.0; 3],
            chunks,
            shapes: vec![Vec::new(), vec![FULL], vec![FENCE], Vec::new(), Vec::new()],
            fluids: vec![0, 0, 0, WATER | 8, WATER | 4],
        }
    }

    const SOURCE: u16 = 3;
    const FLOWING: u16 = 4;

    #[test]
    fn a_level_switch_drops_the_old_level_and_waits_for_ground_at_the_new_origin() {
        let mut w = world(&[([1, 0, 1], 1)]);
        w.brushes = 7;
        assert!(!w.awaits_ground([0.0; 3]));
        w.switch([100.5, 49.0, 0.5]);
        assert!(w.chunks.is_empty());
        assert_eq!(w.origin, [100.5, 49.0, 0.5]);
        assert_eq!((w.shapes.len(), w.fluids.len()), (1, 1));
        assert!(w.shapes[0].is_empty());
        assert!(w.awaits_ground([0.0; 3]));
        assert_eq!(to_block(w.origin, [36.0, 0.0, 0.0]), [101.5, 49.0, 0.5]);
        assert_eq!(w.brushes, 7, "the map's brushes stay");
    }

    #[test]
    fn only_air_support_needs_the_sky() {
        for weapon in [
            "killstreak_predator_missile_mp",
            "killstreak_precision_airstrike_mp",
            "killstreak_stealth_airstrike_mp",
            "killstreak_harrier_airstrike_mp",
            "killstreak_helicopter_mp",
            "killstreak_helicopter_flares_mp",
            "killstreak_helicopter_minigun_mp",
            "killstreak_ac130_mp",
            "airdrop_marker_mp",
            "airdrop_mega_marker_mp",
            "airdrop_sentry_marker_mp",
            "KILLSTREAK_AC130_MP",
        ] {
            assert!(needs_sky(weapon), "{weapon}");
        }
        for weapon in ["killstreak_uav_mp", "killstreak_counter_uav_mp", "killstreak_emp_mp", "killstreak_nuke_mp", "frag_grenade_mp", "m4_mp", "ac130_105mm_mp", ""] {
            assert!(!needs_sky(weapon), "{weapon}");
        }
        let no_sky = playerstate_iw4::pm_flags::MC_NO_SKY | playerstate_iw4::pm_flags::MC_INVULNERABLE;
        assert!(refused_without_sky(no_sky, "killstreak_predator_missile_mp"));
        assert!(!refused_without_sky(no_sky, "killstreak_uav_mp"));
        assert!(!refused_without_sky(no_sky, "m4_mp"));
        assert!(!refused_without_sky(playerstate_iw4::pm_flags::MC_INVULNERABLE, "killstreak_predator_missile_mp"), "under the sky");
    }

    #[test]
    fn ordnance_from_before_a_level_switch_is_left_behind() {
        let base = crate::script::host::entities::SPAWNED_PRESENCE_BASE;
        let model = |serial: u32| crate::ScriptModelId::from_wire(base + serial);
        let map_model = crate::ScriptModelId::from_wire(12);
        let never = LevelSwitch::Never;
        assert!(!never.fired_before(0) && !never.spawned_before(model(0)));
        assert_eq!(never.stamped(5_000, 3), never, "no switch, nothing to stamp");
        let pending = LevelSwitch::Pending;
        assert!(pending.fired_before(i32::MAX) && pending.spawned_before(model(u32::from(u16::MAX))));
        assert!(!pending.spawned_before(map_model), "the map's own models never left a level");
        let at = pending.stamped(5_000, 3);
        assert_eq!(at, LevelSwitch::At { ms: 5_000, models: base + 3 });
        assert_eq!(at.stamped(9_000, 7), at, "the first tick after the switch keeps its stamp");
        assert!(at.fired_before(4_950), "a grenade thrown a tick before the switch");
        assert!(!at.fired_before(5_000), "a projectile born in the switch's own tick");
        assert!(!at.fired_before(7_000));
        assert!(at.spawned_before(model(2)), "a bomb spawned before the switch");
        assert!(!at.spawned_before(model(3)) && !at.spawned_before(model(40)));
        assert!(!at.spawned_before(map_model));
    }

    fn pool(depth: i32) -> VoxelWorld {
        let mut blocks = Vec::new();
        for x in 0..16 {
            for z in 0..16 {
                blocks.push(([x, 0, z], 1));
                for y in 1..=depth {
                    blocks.push(([x, y, z], SOURCE));
                }
            }
        }
        world(&blocks)
    }

    fn sample(w: &VoxelWorld, feet: [f64; 3], eye: f64) -> movement_iw4::WaterSample {
        let half = f64::from(HULL_HALF_WIDTH / BLOCK);
        w.water_sample(feet, [-half, 0.0, -half], [half, 70.0 / 36.0, half], eye)
    }

    #[test]
    fn standing_in_a_deep_pool_is_wet_to_the_eyes() {
        let w = pool(3);
        let s = sample(&w, [8.5, 1.0, 8.5], 60.0 / 36.0);
        assert!(s.eye && s.head);
        assert!((f64::from(s.depth) / 36.0 - (3.0 - 1.001)).abs() < 1e-4, "{s:?}");
    }

    #[test]
    fn a_surface_source_is_eight_ninths_deep() {
        let w = pool(1);
        let s = sample(&w, [8.5, 1.0, 8.5], 60.0 / 36.0);
        assert!(!s.eye && s.head);
        assert!((f64::from(s.depth) / 36.0 - (8.0 / 9.0 - 0.001)).abs() < 1e-4, "{s:?}");
    }

    #[test]
    fn flowing_water_is_shallower_and_dry_land_is_dry() {
        let w = world(&[([4, 0, 4], 1), ([4, 1, 4], FLOWING)]);
        let s = sample(&w, [4.5, 1.0, 4.5], 60.0 / 36.0);
        assert!((f64::from(s.depth) / 36.0 - (4.0 / 9.0 - 0.001)).abs() < 1e-4, "{s:?}");
        let dry = sample(&w, [10.5, 1.0, 10.5], 60.0 / 36.0);
        assert_eq!(dry, movement_iw4::WaterSample::default());
    }

    #[test]
    fn chunks_ready_around_needs_every_column() {
        let mut w = VoxelWorld::default();
        let column = || VoxelChunk { min_y: 0, height: 1, shapes: vec![0; 256] };
        for x in -3..=3 {
            for z in -3..=3 {
                if (x, z) != (2, -3) {
                    w.chunks.insert((x, z), column());
                }
            }
        }
        assert!(!w.has_chunks_around(0, 0, 3));
        assert!(w.has_chunks_around(0, 0, 2));
        assert!(w.has_chunks_around(-1, 1, 0));
        assert!(!w.has_chunks_around(2, -3, 0));
        w.chunks.insert((2, -3), column());
        assert!(w.has_chunks_around(0, 0, 3));
        assert!(!w.has_chunks_around(1, 0, 3));
    }

    #[test]
    fn a_player_waits_for_ground_wherever_its_chunk_is_missing() {
        let mut w = world(&[]);
        w.origin = [7.5, 64.0, 3.5];
        assert!(!w.awaits_ground([0.0; 3]));
        assert!(!w.awaits_ground(to_map(w.origin, [15.9, -200.0, 0.1])));
        assert!(!w.awaits_ground(to_map(w.origin, [7.5, 64.0, 13.9])));
        assert!(w.awaits_ground(to_map(w.origin, [-0.5, 64.0, 8.0])));
        assert!(w.awaits_ground(to_map(w.origin, [7.5, 64.0, -7.0])));
        assert!(w.awaits_ground(to_map(w.origin, [8.0, 64.0, 16.5])));
        assert!(w.awaits_ground(to_map(w.origin, [40.0, 64.0, 3.5])));
        assert!(w.awaits_ground(to_map(w.origin, [2000.5, 120.0, 2000.5])));
        w.chunks.insert((125, 125), VoxelChunk { min_y: 0, height: 1, shapes: vec![0; 256] });
        assert!(!w.awaits_ground(to_map(w.origin, [2000.5, 120.0, 2000.5])));
        assert!(w.awaits_ground(to_map(w.origin, [2016.5, 120.0, 2000.5])));
    }

    #[test]
    fn a_taken_spawn_moves_to_the_nearest_dry_free_column() {
        let mut blocks = Vec::new();
        for x in 0..16 {
            for z in 0..16 {
                blocks.push(([x, 10, z], 1));
            }
        }
        blocks.push(([6, 11, 6], SOURCE));
        let mut w = world(&blocks);
        w.origin = [7.5, 11.0, 7.5];
        let nobody = |_: [f32; 3]| false;
        let spot = w.free_spawn_spot(&nobody).unwrap();
        assert_eq!([spot[0], spot[1]], [-36.0, 0.0]);
        assert!(spot[2].abs() < 0.2, "{spot:?}");
        let crowded = |spot: [f32; 3]| crate::spawn::occupied(&[[-36.0, 0.0, 0.0], [-36.0, -36.0, 0.0]], spot);
        assert_eq!(w.free_spawn_spot(&crowded).map(|s| [s[0], s[1]]), Some([0.0, 36.0]));
    }

    #[test]
    fn the_minecraft_spawn_takes_every_live_or_watching_spawn_but_not_a_corpse() {
        for state in ["playing", "spectator", "intermission"] {
            assert_eq!(spawn_origin_for(true, state), Some([0.0; 3]), "{state}");
            assert_eq!(spawn_origin_for(false, state), None, "{state}");
        }
        assert_eq!(spawn_origin_for(true, "dead"), None);
    }

    #[test]
    fn lava_has_its_own_depth_and_is_never_water() {
        const LAVA_SOURCE: u16 = 5;
        const LAVA_FLOWING: u16 = 6;
        let mut w = world(&[
            ([4, 0, 4], 1),
            ([4, 1, 4], LAVA_SOURCE),
            ([4, 2, 4], LAVA_SOURCE),
            ([8, 0, 8], 1),
            ([8, 1, 8], LAVA_FLOWING),
        ]);
        w.shapes.extend([Vec::new(), Vec::new()]);
        w.fluids.extend([LAVA | 8, LAVA | 2]);
        let half = f64::from(HULL_HALF_WIDTH / BLOCK);
        let (lo, hi) = ([-half, 0.0, -half], [half, 1.8, half]);
        let deep = w.fluid_depth([4.5, 1.0, 4.5], lo, hi, LAVA);
        assert!((deep - (1.0 + 8.0 / 9.0 - 0.001)).abs() < 1e-9, "{deep}");
        assert_eq!(sample(&w, [4.5, 1.0, 4.5], 60.0 / 36.0), movement_iw4::WaterSample::default());
        let shallow = w.fluid_depth([8.5, 1.0, 8.5], lo, hi, LAVA);
        assert!((shallow - (2.0 / 9.0 - 0.001)).abs() < 1e-9, "{shallow}");
        assert!(shallow <= 0.4);
        assert!(w.fluid_covers([4.5, 2.85, 4.5], LAVA) && !w.fluid_covers([4.5, 2.95, 4.5], LAVA));
        assert!(!w.fluid_covers([4.5, 1.5, 4.5], WATER) && !w.fluid_covers([8.5, 1.5, 8.5], LAVA));
        assert_eq!(pool(2).fluid_depth([8.5, 1.0, 8.5], lo, hi, LAVA), 0.0);
    }

    #[test]
    fn berry_bushes_and_cobwebs_slow_a_body_that_overlaps_them() {
        const BUSH: u16 = 5;
        const WEB: u16 = 6;
        let mut w = world(&[([4, 0, 4], 1), ([4, 1, 4], BUSH), ([8, 1, 8], WEB), ([8, 2, 9], BUSH)]);
        w.shapes.extend([Vec::new(), Vec::new()]);
        w.fluids.extend([STUCK_BERRY_BUSH, STUCK_COBWEB]);
        let half = f64::from(HULL_HALF_WIDTH / BLOCK);
        let (lo, hi) = ([-half, 0.0, -half], [half, 1.8, half]);
        assert_eq!(w.stuck_in([4.5, 1.0, 4.5], lo, hi), Some(BERRY_BUSH_SLOWDOWN));
        assert_eq!(w.stuck_in([4.5, 2.0, 4.5], lo, hi), None, "standing over a bush is not in it");
        assert_eq!(w.stuck_in([5.3, 1.0, 4.5], lo, hi), None, "touching a bush is not being in it");
        assert_eq!(w.stuck_in([5.29, 1.0, 4.5], lo, hi), Some(BERRY_BUSH_SLOWDOWN));
        assert_eq!(w.stuck_in([4.5, -0.7, 4.5], lo, hi), Some(BERRY_BUSH_SLOWDOWN), "the head counts too");
        assert_eq!(w.stuck_in([8.5, 1.5, 8.9], lo, hi), Some(COBWEB_SLOWDOWN), "a web holds tighter than a bush");
        assert_eq!(w.stuck_in([12.5, 1.0, 12.5], lo, hi), None);
    }

    #[test]
    fn the_eyes_go_under_just_below_the_surface() {
        let w = pool(2);
        let low = sample(&w, [8.5, 1.0, 8.5], 0.9);
        assert!(low.eye);
        let high = sample(&w, [8.5, 2.0, 8.5], 1.0);
        assert!(!high.eye);
    }

    fn body(feet: [f64; 3], height: f64) -> ([f64; 3], [f64; 3]) {
        let half = f64::from(HULL_HALF_WIDTH / BLOCK);
        ([feet[0] - half, feet[1], feet[2] - half], [feet[0] + half, feet[1] + height, feet[2] + half])
    }

    #[test]
    fn blocks_are_world_ground_for_movement() {
        let w = world(&[([4, 0, 4], 1)]);
        let (mins, maxs) = ([-15.0, -15.0, 0.0], [15.0, 15.0, 70.0]);
        let above = to_map([0.0; 3], [4.5, 1.0 + 0.25 / 36.0, 4.5]);
        let below = to_map([0.0; 3], [4.5, 1.0 - 0.25 / 36.0, 4.5]);
        let ground = trace_in(&w, above, below, mins, maxs, None);
        assert!(ground.fraction < 1.0);
        assert_eq!(ground.walkable, 1);
        assert_eq!((ground.hit_type, ground.hit_id), (trace_iw4::HITTYPE_ENTITY, trace_iw4::ENTITYNUM_WORLD));
        let sunk = to_map([0.0; 3], [4.5, 0.5, 4.5]);
        let inside = trace_in(&w, sunk, sunk, mins, maxs, None);
        assert_eq!(inside.startsolid, 1);
        assert_eq!((inside.hit_type, inside.hit_id), (trace_iw4::HITTYPE_ENTITY, trace_iw4::ENTITYNUM_WORLD));
        let (high, low) = (to_map([0.0; 3], [10.5, 5.0, 10.5]), to_map([0.0; 3], [10.5, 4.0, 10.5]));
        let air = trace_in(&w, high, low, mins, maxs, None);
        assert_eq!((air.fraction, air.hit_type), (1.0, 0));
    }

    #[test]
    fn a_vanilla_sized_body_fits_trapdoor_gaps() {
        const OPEN: [f32; 6] = [0.0, 0.0, 0.0, 0.1875, 1.0, 1.0];
        const TOP: [f32; 6] = [0.0, 0.8125, 0.0, 1.0, 1.0, 1.0];
        let mut blocks = vec![([4, 4, 4], 5), ([10, 0, 10], 1), ([10, 2, 10], 6), ([10, 3, 10], 1)];
        for x in 2..=6 {
            for z in 2..=6 {
                if (x, z) != (4, 4) {
                    blocks.push(([x, 4, z], 1));
                }
            }
        }
        let mut w = world(&blocks);
        w.shapes.extend([vec![OPEN], vec![TOP]]);
        w.fluids.extend([0, 0]);
        let mw2 = movement_iw4::MoveBounds {
            mins: [-15.0, -15.0, 0.0],
            maxs: [15.0, 15.0, 70.0],
            tracemask: 0,
            heights: movement_iw4::STANCE_HEIGHTS,
        };
        let vanilla = move_bounds(mw2);
        assert!((vanilla.maxs[0] - 10.8).abs() < 1e-4 && (vanilla.maxs[2] - 64.8).abs() < 1e-4);
        assert!((vanilla.heights[1] - 54.0).abs() < 1e-4);
        let drop = |b: movement_iw4::MoveBounds| {
            let (high, low) = (to_map([0.0; 3], [4.6, 6.0, 4.5]), to_map([0.0; 3], [4.6, 2.0, 4.5]));
            trace_in(&w, high, low, b.mins, b.maxs, None).fraction
        };
        assert_eq!(drop(vanilla), 1.0);
        assert!(drop(mw2) < 1.0);
        let stand = |b: movement_iw4::MoveBounds| {
            let at = to_map([0.0; 3], [10.5, 1.01, 10.5]);
            trace_in(&w, at, at, b.mins, b.maxs, None).startsolid
        };
        assert_eq!(stand(vanilla), 0);
        assert_eq!(stand(mw2), 1);
        let crouched = movement_iw4::MoveBounds { maxs: [vanilla.maxs[0], vanilla.maxs[1], STANCE_HEIGHTS[1]], ..vanilla };
        assert_eq!(area_bounds(crouched).maxs, [15.0, 15.0, 50.0]);
        assert_eq!(area_bounds(mw2), mw2);
    }

    #[test]
    fn a_body_clear_of_blocks_is_left_alone() {
        let w = world(&[([4, 0, 4], 1)]);
        let (lo, hi) = body([4.5, 1.0 + 1e-3, 4.5], 70.0 / 36.0);
        assert_eq!(w.way_out(lo, hi, None), None);
    }

    #[test]
    fn a_body_sunk_into_a_block_below_is_lifted_onto_it() {
        let w = world(&[([4, 0, 4], 1)]);
        let (lo, hi) = body([4.5, 0.9, 4.5], 70.0 / 36.0);
        let d = w.way_out(lo, hi, None).expect("a way out");
        assert_eq!((d[0], d[2]), (0.0, 0.0));
        assert!((0.9 + d[1] - (1.0 + PUSH_GAP)).abs() < 1e-9);
    }

    #[test]
    fn a_lift_blocked_overhead_goes_sideways_instead() {
        let w = world(&[([4, 0, 4], 1), ([4, 2, 4], 1), ([4, 3, 4], 1)]);
        let (lo, hi) = body([4.3, 0.95, 4.5], 1.2);
        let d = w.way_out(lo, hi, None).expect("a way out");
        assert_eq!(d[1], 0.0);
        assert!(d[0] < 0.0);
        assert!(w.overlapping(std::array::from_fn(|k| lo[k] + d[k]), std::array::from_fn(|k| hi[k] + d[k]), None).is_empty());
    }

    #[test]
    fn the_shortest_way_out_wins() {
        let w = world(&[([5, 1, 4], 1), ([5, 2, 4], 1)]);
        let (lo, hi) = body([4.75, 1.0, 4.5], 70.0 / 36.0);
        let d = w.way_out(lo, hi, None).expect("a way out");
        assert!(d[0] < 0.0 && d[0] > -0.1);
        assert_eq!((d[1], d[2]), (0.0, 0.0));
    }

    #[test]
    fn a_fence_taller_than_its_cell_still_counts() {
        let w = world(&[([4, 0, 4], 2)]);
        let (lo, hi) = body([4.5, 1.2, 4.5], 70.0 / 36.0);
        assert_eq!(w.overlapping(lo, hi, None).len(), 1);
        let d = w.way_out(lo, hi, None).expect("a way out");
        assert!((1.2 + d[1] - (1.5 + PUSH_GAP)).abs() < 1e-9);
    }

    #[test]
    fn a_body_walled_in_is_not_moved() {
        let mut blocks = Vec::new();
        for y in 0..6 {
            for x in 3..6 {
                for z in 3..6 {
                    blocks.push(([x, y, z], 1));
                }
            }
        }
        let w = world(&blocks);
        let (lo, hi) = body([4.5, 1.0, 4.5], 70.0 / 36.0);
        assert_eq!(w.way_out(lo, hi, None), None);
    }

    #[test]
    fn a_head_in_the_ceiling_goes_down_not_through_it() {
        let w = world(&[([4, 0, 4], 1), ([4, 3, 4], 1)]);
        let (lo, hi) = body([4.5, 1.2, 4.5], 70.0 / 36.0);
        let d = w.way_out(lo, hi, None).expect("a way out");
        assert!(d[1] < 0.0 && d[1] > -0.2, "{d:?}");
        let mut tunnel = Vec::new();
        for z in 0..9 {
            for x in 3..=5 {
                tunnel.push(([x, -1, z], 1));
                tunnel.push(([x, 1, z], 1));
            }
            tunnel.push(([3, 0, z], 1));
            tunnel.push(([5, 0, z], 1));
        }
        let w = world(&tunnel);
        let (lo, hi) = body([4.5, 0.0, 4.5], 50.0 / 36.0);
        assert_eq!(w.way_out(lo, hi, None), None);
    }

    #[test]
    fn a_body_against_a_wall_touches_it_but_not_the_floor_below_or_open_air() {
        let w = world(&[([4, 0, 4], 1), ([5, 1, 4], 1), ([5, 2, 4], 1)]);
        let (mins, maxs) = player_hull(&playerstate_iw4::PlayerState::ZERO);
        let at = |x: f64| to_map([0.0; 3], [x, 1.0 + 1e-3, 4.5]);
        assert!(touches_wall_in(&w, at(4.7 - 0.125 / 36.0), mins, maxs));
        assert!(!touches_wall_in(&w, at(4.5), mins, maxs));
        assert!(!touches_wall_in(&w, to_map([0.0; 3], [4.5, 1.0, 4.5]), mins, maxs));
        assert!(!touches_wall_in(&w, to_map([0.0; 3], [10.5, 5.0, 10.5]), mins, maxs));
    }

    #[test]
    fn touching_boxes_do_not_meet() {
        assert!(!boxes_meet([0.0, 0.0, 0.0, 1.0, 1.0, 1.0], [1.0, 0.0, 0.0, 2.0, 1.0, 1.0]));
        assert!(boxes_meet([0.0, 0.0, 0.0, 1.0, 1.0, 1.0], [0.99, 0.5, 0.5, 2.0, 1.0, 1.0]));
    }

    const STABLE: [[f32; 6]; 5] = [
        [0.0, 0.875, 0.0, 1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0, 0.125, 1.0, 0.125],
        [0.875, 0.0, 0.0, 1.0, 1.0, 0.125],
        [0.0, 0.0, 0.875, 0.125, 1.0, 1.0],
        [0.875, 0.0, 0.875, 1.0, 1.0, 1.0],
    ];
    const TOWER: u16 = 5;
    const HANGING: u16 = 6;
    const LADDER: u16 = 7;

    fn scaffolding(blocks: &[([i32; 3], u16)]) -> VoxelWorld {
        let mut w = world(blocks);
        w.shapes.extend([STABLE.to_vec(), STABLE.to_vec(), vec![[0.0, 0.0, 0.8125, 1.0, 1.0, 1.0]]]);
        w.fluids.extend([CLIMBABLE | SCAFFOLDING, CLIMBABLE | SCAFFOLDING | SCAFFOLDING_BOTTOM, CLIMBABLE]);
        w
    }

    fn mover(feet: f64, descending: bool) -> Option<Mover> {
        Some(Mover { feet: (feet * 36.0) as f32, descending })
    }

    const HULL: ([f32; 3], [f32; 3]) = ([-10.8, -10.8, 0.0], [10.8, 10.8, 64.8]);

    fn drop_onto(w: &VoxelWorld, from: f64, to: f64, who: Option<Mover>) -> f32 {
        let (high, low) = (to_map([0.0; 3], [4.5, from, 4.5]), to_map([0.0; 3], [4.5, to, 4.5]));
        trace_in(w, high, low, HULL.0, HULL.1, who).fraction
    }

    #[test]
    fn scaffolding_holds_up_a_player_above_it_unless_they_sneak() {
        let w = scaffolding(&[([4, 0, 4], 1), ([4, 1, 4], TOWER), ([4, 2, 4], TOWER)]);
        let above = 3.0 + 0.25 / 36.0;
        assert!(drop_onto(&w, above, 2.9, mover(above, false)) < 1.0);
        assert_eq!(drop_onto(&w, above, 2.9, mover(above, true)), 1.0);
        assert!(drop_onto(&w, above, 2.9, None) < 1.0);
        let inside = 2.5;
        assert_eq!(drop_onto(&w, inside, 2.0 + 0.01, mover(inside, false)), 1.0);
        assert!(drop_onto(&w, inside, 1.9, mover(inside, false)) < 1.0);
        assert_eq!(drop_onto(&w, 1.2, 1.0 + 0.01, mover(1.2, true)), 1.0);
    }

    #[test]
    fn scaffolding_lets_a_player_walk_in_from_the_side() {
        let w = scaffolding(&[([4, 0, 4], 1), ([5, 1, 4], TOWER), ([5, 2, 4], TOWER)]);
        let feet = 1.0 + 0.25 / 36.0;
        let (from, to) = (to_map([0.0; 3], [4.5, feet, 4.5]), to_map([0.0; 3], [5.5, feet, 4.5]));
        assert_eq!(trace_in(&w, from, to, HULL.0, HULL.1, mover(feet, false)).fraction, 1.0);
        assert!(trace_in(&w, from, to, HULL.0, HULL.1, None).fraction < 1.0);
        let (lo, hi) = body([5.5, feet, 4.5], 1.8);
        assert_eq!(w.way_out(lo, hi, mover(feet, false)), None);
        assert!(w.way_out(lo, hi, None).is_some());
    }

    #[test]
    fn hanging_scaffolding_has_a_floor_slab_for_players_not_below_it() {
        let w = scaffolding(&[([4, 3, 4], HANGING)]);
        assert!((drop_onto(&w, 3.5, 3.0, mover(3.5, false)) - 0.75).abs() < 0.01);
        assert!(drop_onto(&w, 4.5, 3.0, mover(4.5, true)) < 1.0);
        let rising = (to_map([0.0; 3], [4.5, 1.0, 4.5]), to_map([0.0; 3], [4.5, 3.5, 4.5]));
        assert_eq!(trace_in(&w, rising.0, rising.1, HULL.0, HULL.1, mover(1.0, false)).fraction, 1.0);
        let tower = scaffolding(&[([4, 3, 4], TOWER)]);
        assert_eq!(drop_onto(&tower, 3.5, 3.0, mover(3.5, false)), 1.0);
    }

    #[test]
    fn the_block_at_the_feet_decides_climbing() {
        let w = scaffolding(&[([4, 1, 4], TOWER), ([6, 1, 6], LADDER)]);
        let tower = w.climb_sample([4.5, 1.2, 4.5]);
        assert_eq!((tower.on, tower.scaffolding), (true, true));
        let ladder = w.climb_sample([6.5, 1.0, 6.5]);
        assert_eq!((ladder.on, ladder.scaffolding), (true, false));
        assert_eq!(w.climb_sample([4.5, 2.0 + 1e-3, 4.5]), movement_iw4::ClimbSample::default());
        assert_eq!(w.fluid_depth([4.5, 1.0, 4.5], [-0.3, 0.0, -0.3], [0.3, 1.8, 0.3], WATER), 0.0);
    }

    #[test]
    fn the_hull_follows_the_stance() {
        let mut ps = playerstate_iw4::PlayerState::ZERO;
        ps.view_height_target = movement_iw4::view_height::STAND;
        assert_eq!(player_hull(&ps).1[2], STANCE_HEIGHTS[0]);
        ps.view_height_target = movement_iw4::view_height::CROUCH;
        assert_eq!(player_hull(&ps).1[2], STANCE_HEIGHTS[1]);
        ps.view_height_target = movement_iw4::view_height::PRONE;
        assert_eq!(player_hull(&ps).1[2], 30.0);
        assert_eq!(player_hull(&ps).1[0], 0.3 * BLOCK);
    }

    struct Pond(VoxelWorld);

    impl movement_iw4::CollisionBackend for Pond {
        fn trace(&self, input: movement_iw4::GroundTraceInput) -> trace_iw4::Trace {
            trace_in(&self.0, input.start, input.end, input.mins, input.maxs, None)
        }

        fn water(&self, origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3], view_height: f32) -> movement_iw4::WaterSample {
            let (lo, hi) = block_bounds(mins, maxs);
            self.0.water_sample(to_block(self.0.origin, origin), lo, hi, f64::from(view_height / BLOCK))
        }

        fn lava(&self, origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> f32 {
            let (lo, hi) = block_bounds(mins, maxs);
            (self.0.fluid_depth(to_block(self.0.origin, origin), lo, hi, LAVA) * f64::from(BLOCK)) as f32
        }

        fn ground_within(&self, origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3], drop: f32) -> Option<bool> {
            Some(ground_in(&self.0, origin, mins, maxs, drop, None))
        }
    }

    const LAVA_POOL: u16 = 5;
    const POND_EDGE: i32 = 12;
    const POND_SURFACE: f64 = 2.0 + 8.0 / 9.0;

    fn pond(fluid: u16, depth: i32, bank_top: i32) -> Pond {
        let mut blocks = Vec::new();
        for x in 0..16 {
            for z in 0..16 {
                blocks.push(([x, 0, z], 1));
                let (top, block) = if x < POND_EDGE { (depth, fluid) } else { (bank_top - 1, 1) };
                for y in 1..=top {
                    blocks.push(([x, y, z], block));
                }
            }
        }
        let mut w = world(&blocks);
        w.shapes.push(Vec::new());
        w.fluids.push(LAVA | 8);
        Pond(w)
    }

    fn swimmer(feet: [f64; 3]) -> playerstate_iw4::PlayerState {
        crate::world::spawn_player_state(to_map([0.0; 3], feet), [0.0; 3])
    }

    fn swim(pond: &Pond, ps: &mut playerstate_iw4::PlayerState, buttons: u32, forwardmove: i8, msec: i32, seconds: i32) -> Vec<[f64; 3]> {
        drive(pond, ps, buttons, forwardmove, msec, seconds * 1000 / msec)
    }

    fn drive(pond: &Pond, ps: &mut playerstate_iw4::PlayerState, buttons: u32, forwardmove: i8, msec: i32, frames: i32) -> Vec<[f64; 3]> {
        let mut old_buttons = 0;
        let mut path = Vec::new();
        for _ in 0..frames {
            let context = crate::step::pmove_context(old_buttons, (1.0, 1.0, 1.0), false, false, 0.005, 0.005, 0, false, true, false, 0, 0, 0, false, false);
            let context = movement_iw4::PmoveSingleContext { bounds: move_bounds(context.bounds), ..context };
            let mut cmd = playerstate_iw4::UserCmd { server_time: ps.command_time + msec, buttons, forwardmove, ..playerstate_iw4::UserCmd::default() };
            movement_iw4::pmove(ps, &mut cmd, context, pond, &movement_iw4::FlatMantleAnimLength::default(), &movement_iw4::ZeroMantleRootDelta);
            old_buttons = buttons;
            path.push(to_block([0.0; 3], ps.origin));
        }
        path
    }

    #[test]
    fn holding_jump_lifts_a_swimmer_off_the_floor_of_deep_water_or_lava_to_float_at_the_surface() {
        for fluid in [SOURCE, LAVA_POOL] {
            for msec in [50, 8] {
                let pond = pond(fluid, 2, 1);
                let mut ps = swimmer([6.5, 1.5, 8.5]);
                let sunk = swim(&pond, &mut ps, 0, 0, msec, 3);
                assert!((sunk[sunk.len() - 1][1] - 1.0).abs() < 0.01, "fluid {fluid} msec {msec}: {:?}", sunk.last());
                assert_eq!(ps.ground_entity_num, i32::from(trace_iw4::ENTITYNUM_WORLD), "fluid {fluid} msec {msec}");
                let held = swim(&pond, &mut ps, playerstate_iw4::buttons::JUMP, 0, msec, 8);
                let floating = &held[held.len() * 5 / 8..];
                assert!(
                    floating.iter().all(|p| (p[1] - POND_SURFACE).abs() < 0.5),
                    "fluid {fluid} msec {msec}: {:?}",
                    floating.iter().map(|p| p[1]).fold((f64::MAX, f64::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)))
                );
            }
        }
    }

    #[test]
    fn a_swimmer_holding_jump_climbs_out_onto_a_bank_one_block_above_the_floor_or_level_with_the_water() {
        for fluid in [SOURCE, LAVA_POOL] {
            for (depth, bank_top) in [(1, 2), (2, 3)] {
                for msec in [50, 8] {
                    let pond = pond(fluid, depth, bank_top);
                    let mut ps = swimmer([10.5, 1.5, 8.5]);
                    swim(&pond, &mut ps, 0, 0, msec, 3);
                    assert_eq!(ps.ground_entity_num, i32::from(trace_iw4::ENTITYNUM_WORLD), "fluid {fluid} depth {depth} msec {msec}");
                    let path = swim(&pond, &mut ps, playerstate_iw4::buttons::JUMP, 127, msec, 6);
                    let ashore = path.iter().any(|p| p[0] > f64::from(POND_EDGE) + 0.3 && (p[1] - f64::from(bank_top)).abs() < 0.01);
                    assert!(ashore, "fluid {fluid} depth {depth} msec {msec}: {:?}", path.last());
                }
            }
        }
    }

    const LEDGE_EDGE: i32 = 6;
    const BOTTOM_SLAB: u16 = 5;

    fn ledge(below_edge: Option<u16>) -> Pond {
        let mut blocks = Vec::new();
        for x in 0..16 {
            for z in 0..16 {
                blocks.push(([x, -1, z], 1));
                match (x < LEDGE_EDGE, below_edge) {
                    (true, _) => blocks.push(([x, 0, z], 1)),
                    (false, Some(lower)) => blocks.push(([x, 0, z], lower)),
                    (false, None) => {}
                }
            }
        }
        let mut w = world(&blocks);
        w.shapes.push(vec![[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]);
        w.fluids.push(0);
        Pond(w)
    }

    #[test]
    fn sneaking_stops_at_a_ledge_higher_than_a_step_but_walking_falls_off() {
        for msec in [50, 8] {
            let cliff = ledge(None);
            let mut sneaker = swimmer([3.5, 1.0, 8.5]);
            let path = swim(&cliff, &mut sneaker, playerstate_iw4::buttons::CROUCH, 127, msec, 4);
            assert!(path.iter().all(|p| (p[1] - 1.0).abs() < 0.01), "msec {msec}: {:?}", path.iter().map(|p| p[1]).fold(f64::MAX, f64::min));
            let end = path[path.len() - 1];
            assert!(end[0] > f64::from(LEDGE_EDGE) && end[0] < f64::from(LEDGE_EDGE) + 0.3, "msec {msec}: {end:?}");
            let mut walker = swimmer([3.5, 1.0, 8.5]);
            let path = swim(&cliff, &mut walker, 0, 127, msec, 4);
            assert!(path.iter().any(|p| p[1] < 0.1), "msec {msec}: {:?}", path.last());
        }
    }

    #[test]
    fn sneaking_stops_at_a_ledge_while_wading_in_water_or_lava() {
        for fluid in [SOURCE, LAVA_POOL + 1] {
            for msec in [50, 8] {
                let Pond(mut w) = ledge(None);
                w.shapes.push(Vec::new());
                w.fluids.push(LAVA | 8);
                let chunk = w.chunks.get_mut(&(0, 0)).unwrap();
                for x in 0..LEDGE_EDGE {
                    for z in 0..16 {
                        chunk.shapes[(((1 + 16) * 16 + z) * 16 + x) as usize] = fluid;
                    }
                }
                let wading = Pond(w);
                let mut sneaker = swimmer([5.3, 1.2, 8.5]);
                swim(&wading, &mut sneaker, 0, 0, msec, 1);
                let path = swim(&wading, &mut sneaker, playerstate_iw4::buttons::CROUCH, 127, msec, 6);
                assert!(path.iter().all(|p| (p[1] - 1.0).abs() < 0.01), "fluid {fluid} msec {msec}: {:?}", path.iter().map(|p| p[1]).fold(f64::MAX, f64::min));
                let end = path[path.len() - 1];
                assert!(end[0] > f64::from(LEDGE_EDGE) && end[0] < f64::from(LEDGE_EDGE) + 0.3, "fluid {fluid} msec {msec}: {end:?}");
                let mut walker = swimmer([5.3, 1.2, 8.5]);
                swim(&wading, &mut walker, 0, 0, msec, 1);
                let path = swim(&wading, &mut walker, 0, 127, msec, 6);
                assert!(path.iter().any(|p| p[1] < 0.1), "fluid {fluid} msec {msec}: {:?}", path.last());
            }
        }
    }

    #[test]
    fn sneaking_walks_down_onto_a_slab_within_the_step() {
        for msec in [50, 8] {
            let steps = ledge(Some(BOTTOM_SLAB));
            let mut sneaker = swimmer([3.5, 1.0, 8.5]);
            let path = swim(&steps, &mut sneaker, playerstate_iw4::buttons::CROUCH, 127, msec, 2);
            assert!(path.iter().any(|p| p[0] > f64::from(LEDGE_EDGE) + 1.0 && (p[1] - 0.5).abs() < 0.01), "msec {msec}: {:?}", path.last());
        }
    }

    #[test]
    fn a_totem_saves_one_death_and_reports_it() {
        let client = 7_000_001;
        assert!(!spend_death_protection(client));
        set_death_protected(client, true);
        set_death_protected(client, true);
        assert!(spend_death_protection(client));
        assert!(!spend_death_protection(client));
        assert_eq!(take_totem_pops().iter().filter(|&&c| c == client).count(), 1);
        set_death_protected(client, true);
        set_death_protected(client, false);
        assert!(!spend_death_protection(client));
    }

    #[test]
    fn a_minecraft_death_rides_with_its_damage_and_names_the_victim() {
        let client = 7_000_002;
        let death = MinecraftDeath { message: "%1$s was shot by Skeleton".into(), killer: "Skeleton".into() };
        push_player_damage(client, 20, None, Some(death.clone()));
        let queued: Vec<PlayerDamage> = take_player_damage().into_iter().filter(|d| d.client == client).collect();
        assert_eq!(queued, vec![PlayerDamage { client, amount: 20, from: None, death: Some(death.clone()) }]);
        assert_eq!(death.line("Sos"), "Sos was shot by Skeleton");
        push_minecraft_death(client, death.killer);
        let deaths: Vec<(u32, String)> = take_minecraft_deaths().into_iter().filter(|(c, _)| *c == client).collect();
        assert_eq!(deaths, vec![(client, "Skeleton".to_owned())]);
        assert!(take_minecraft_deaths().iter().all(|(c, _)| *c != client));
    }

    #[test]
    fn a_team_switch_is_taken_once() {
        let client = 7_000_003;
        push_team_switch(client);
        assert_eq!(take_team_switches().into_iter().filter(|&c| c == client).count(), 1);
        assert!(!take_team_switches().contains(&client));
    }

    fn wall(x: i32) -> VoxelWorld {
        let mut blocks = Vec::new();
        for y in 0..12 {
            for z in 0..16 {
                blocks.push(([x, y, z], 1));
            }
        }
        world(&blocks)
    }

    const BACK_X: [f32; 3] = [-1.0, 0.0, 0.0];

    #[test]
    fn the_camera_keeps_its_full_distance_in_the_open() {
        let w = world(&[]);
        let eye = to_map([0.0; 3], [8.5, 5.5, 8.5]);
        assert_eq!(camera_reach_in(&w, eye, BACK_X, 4.0 * BLOCK), 4.0 * BLOCK);
    }

    #[test]
    fn the_camera_stops_at_a_wall_behind_the_eye() {
        let w = wall(5);
        let eye = to_map([0.0; 3], [8.5, 5.5, 8.5]);
        let reach = camera_reach_in(&w, eye, BACK_X, 4.0 * BLOCK);
        assert!((reach - 2.5 * BLOCK).abs() < 0.2 * BLOCK, "{reach}");
    }

    #[test]
    fn the_camera_never_goes_through_a_wall_right_behind_the_eye() {
        let w = wall(5);
        let eye = to_map([0.0; 3], [6.05, 5.5, 8.5]);
        let reach = camera_reach_in(&w, eye, BACK_X, 4.0 * BLOCK);
        assert!(reach > 0.14 * BLOCK && reach < 0.18 * BLOCK, "{reach}");
    }

    #[test]
    fn the_corner_rays_catch_a_block_the_centre_ray_misses() {
        let w = world(&[([6, 5, 9], 1)]);
        let eye = to_map([0.0; 3], [8.5, 5.5, 8.95]);
        let end: [f32; 3] = std::array::from_fn(|k| eye[k] + BACK_X[k] * 4.0 * BLOCK);
        assert_eq!(trace_in(&w, eye, end, [0.0; 3], [0.0; 3], None).fraction, 1.0);
        let reach = camera_reach_in(&w, eye, BACK_X, 4.0 * BLOCK);
        assert!((reach - 1.5 * BLOCK).abs() < 0.1 * BLOCK, "{reach}");
    }

    fn floor() -> Pond {
        let mut blocks = Vec::new();
        for x in 0..16 {
            for z in 0..16 {
                blocks.push(([x, 0, z], 1));
            }
        }
        Pond(world(&blocks))
    }

    fn flier(feet: [f64; 3], flags: u32) -> playerstate_iw4::PlayerState {
        let mut ps = swimmer(feet);
        ps.ground_entity_num = playerstate_iw4::ENTITYNUM_NONE;
        ps.pm_flags |= flags;
        ps
    }

    const FLYING: u32 = playerstate_iw4::pm_flags::MC_MAY_FLY | playerstate_iw4::pm_flags::MC_FLYING;

    #[test]
    fn a_flier_sinking_onto_stone_lands_and_stops_flying() {
        for msec in [50, 8] {
            let mut ps = flier([8.5, 4.0, 8.5], FLYING);
            let path = drive(&floor(), &mut ps, playerstate_iw4::buttons::CROUCH, 0, msec, 2000 / msec);
            assert!((path[path.len() - 1][1] - 1.0).abs() < 0.01, "msec {msec}: {:?}", path.last());
            assert_eq!(ps.pm_flags & playerstate_iw4::pm_flags::MC_FLYING, 0, "msec {msec}");
            assert_eq!(ps.ground_entity_num, i32::from(trace_iw4::ENTITYNUM_WORLD), "msec {msec}");
        }
    }

    #[test]
    fn a_flier_hovering_just_above_the_floor_keeps_flying() {
        for msec in [50, 8] {
            let mut ps = flier([8.5, 1.5, 8.5], FLYING);
            let path = drive(&floor(), &mut ps, 0, 0, msec, 2000 / msec);
            assert!(path.iter().all(|p| (p[1] - 1.5).abs() < 1e-3), "msec {msec}: {:?}", path.last());
            assert_ne!(ps.pm_flags & playerstate_iw4::pm_flags::MC_FLYING, 0, "msec {msec}");
        }
    }

    #[test]
    fn a_flier_stops_at_a_stone_wall() {
        for msec in [50, 8] {
            let mut ps = flier([6.5, 5.0, 8.5], FLYING);
            let path = drive(&Pond(wall(10)), &mut ps, 0, 127, msec, 2000 / msec);
            let end = path[path.len() - 1];
            assert!(end[0] > 9.6 && end[0] < 9.71, "msec {msec}: {end:?}");
            assert!((end[1] - 5.0).abs() < 1e-3, "msec {msec}: {end:?}");
            assert_ne!(ps.pm_flags & playerstate_iw4::pm_flags::MC_FLYING, 0, "msec {msec}");
        }
    }

    #[test]
    fn a_spectator_flies_through_a_stone_wall_and_the_floor_and_never_lands() {
        let spectator = FLYING | playerstate_iw4::pm_flags::MC_NOCLIP;
        let mut blocks = Vec::new();
        for x in 10..13 {
            for y in 0..12 {
                for z in 0..16 {
                    blocks.push(([x, y, z], 1));
                }
            }
        }
        let thick = Pond(world(&blocks));
        for msec in [50, 8] {
            let mut ghost = flier([6.5, 5.0, 8.5], spectator);
            let path = drive(&thick, &mut ghost, 0, 127, msec, 2000 / msec);
            let end = path[path.len() - 1];
            assert!(end[0] > 13.5, "msec {msec}: {end:?}");
            assert!((end[1] - 5.0).abs() < 1e-3, "msec {msec}: {end:?}");
            assert!(path.iter().any(|p| p[0] > 10.5 && p[0] < 12.5), "msec {msec}");
            assert_eq!(ghost.ground_entity_num, playerstate_iw4::ENTITYNUM_NONE, "msec {msec}");
            assert_ne!(ghost.pm_flags & playerstate_iw4::pm_flags::MC_FLYING, 0, "msec {msec}");

            let mut walled = flier([6.5, 5.0, 8.5], FLYING);
            let path = drive(&thick, &mut walled, 0, 127, msec, 2000 / msec);
            assert!(path[path.len() - 1][0] < 9.71, "msec {msec}: {:?}", path.last());

            let mut sinker = flier([8.5, 3.0, 8.5], spectator);
            let path = drive(&floor(), &mut sinker, playerstate_iw4::buttons::CROUCH, 0, msec, 2000 / msec);
            assert!(path[path.len() - 1][1] < -5.0, "msec {msec}: {:?}", path.last());
            assert_eq!(sinker.ground_entity_num, playerstate_iw4::ENTITYNUM_NONE, "msec {msec}");
            assert_ne!(sinker.pm_flags & playerstate_iw4::pm_flags::MC_FLYING, 0, "msec {msec}");
        }
    }

    #[test]
    fn a_double_tap_on_the_ground_takes_off_only_for_a_player_who_may_fly() {
        for (flags, flies) in [(playerstate_iw4::pm_flags::MC_MAY_FLY, true), (0, false)] {
            let ground = floor();
            let mut ps = flier([8.5, 1.0, 8.5], flags);
            drive(&ground, &mut ps, 0, 0, 50, 20);
            assert_eq!(ps.ground_entity_num, i32::from(trace_iw4::ENTITYNUM_WORLD));
            drive(&ground, &mut ps, playerstate_iw4::buttons::MC_FLY_TOGGLE, 0, 50, 1);
            let path = drive(&ground, &mut ps, 0, 0, 50, 4);
            assert_eq!(ps.pm_flags & playerstate_iw4::pm_flags::MC_FLYING != 0, flies);
            assert_eq!(path[path.len() - 1][1] > 1.4, flies, "{:?}", path.last());
        }
    }
}
