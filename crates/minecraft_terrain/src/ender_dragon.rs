use std::collections::BTreeMap;

use glam::DVec3;
use minecraftoss_core::entity_data::wrap_degrees;
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::mth::{atan2, cos, sin};
use minecraftoss_player::rng::LegacyRandom;

use crate::end_crystal::{DamageKind, EndDamage};

pub const ENDER_DRAGON: &str = "minecraft:ender_dragon";
pub const MAX_HEALTH: f32 = 200.0;
pub const TRACKING_RANGE: f64 = 160.0;
pub const FIRST_ID: u64 = 1 << 40;
pub const HEAD: usize = 0;
pub const NECK: usize = 1;
pub const BODY: usize = 2;
pub const PART_SIZES: [(f64, f64); 8] = [
    (1.0, 1.0),
    (3.0, 3.0),
    (5.0, 3.0),
    (2.0, 2.0),
    (2.0, 2.0),
    (2.0, 2.0),
    (4.0, 2.0),
    (4.0, 2.0),
];
pub const HISTORY_SENT: usize = 26;
pub const CONTACT_DAMAGE: f32 = 10.0;
pub const SOUND_VOLUME: f32 = 5.0;
pub const AMBIENT_SOUND: &str = "minecraft:entity.ender_dragon.ambient";
pub const HURT_SOUND: &str = "minecraft:entity.ender_dragon.hurt";
pub const DEATH_SOUND: &str = "minecraft:entity.generic.death";
pub const FLAP_SOUND: &str = "minecraft:entity.ender_dragon.flap";
pub const GROWL_SOUND: &str = "minecraft:entity.ender_dragon.growl";
pub const DRAGON_DEATH_SOUND: &str = "minecraft:entity.ender_dragon.death";
pub const FIRST_KILL_EXPERIENCE: i32 = 12000;
pub const LATER_KILL_EXPERIENCE: i32 = 500;
pub const DEATH_TICKS: i32 = 200;
const DEG: f32 = 0.017_453_292;
const NODE_COUNT: usize = 24;
const NODE_ADJACENCY: [u32; NODE_COUNT] = [
    6146, 8197, 8202, 16404, 32808, 32848, 65696, 131392, 131712, 263424, 526848, 525313, 1581057,
    3166214, 2138120, 6373424, 4358208, 12910976, 9044480, 9706496, 15216640, 13688832, 11763712,
    8257536,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub y: f64,
    pub y_rot: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlightHistory {
    samples: [Sample; 64],
    head: i32,
}

impl Default for FlightHistory {
    fn default() -> Self {
        Self {
            samples: [Sample { y: 0.0, y_rot: 0.0 }; 64],
            head: -1,
        }
    }
}

pub fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

pub fn rot_lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * wrap_degrees(b - a)
}

pub fn wrap_degrees_f64(angle: f64) -> f64 {
    let mut a = angle % 360.0;
    if a >= 180.0 {
        a -= 360.0;
    }
    if a < -180.0 {
        a += 360.0;
    }
    a
}

impl FlightHistory {
    pub fn record(&mut self, y: f64, y_rot: f32) {
        let sample = Sample { y, y_rot };
        if self.head < 0 {
            self.samples = [sample; 64];
        }
        self.head += 1;
        if self.head == 64 {
            self.head = 0;
        }
        self.samples[self.head as usize] = sample;
    }

    pub fn get(&self, back: usize) -> Sample {
        self.samples[((self.head - back as i32) & 63) as usize]
    }

    pub fn recent(&self, count: usize) -> Vec<Sample> {
        (0..count).map(|back| self.get(back)).collect()
    }
}

pub fn sample_at(samples: &[Sample], back: usize, partial: f32) -> Sample {
    let get = |i: usize| {
        samples
            .get(i)
            .or(samples.last())
            .copied()
            .unwrap_or(Sample { y: 0.0, y_rot: 0.0 })
    };
    let (newer, older) = (get(back), get(back + 1));
    Sample {
        y: lerp(f64::from(partial), older.y, newer.y),
        y_rot: rot_lerp(partial, older.y_rot, newer.y_rot),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    HoldingPattern,
    Dying,
    Hovering,
}

impl Phase {
    pub fn id(self) -> i32 {
        match self {
            Phase::HoldingPattern => 0,
            Phase::Dying => 9,
            Phase::Hovering => 10,
        }
    }

    pub fn by_id(id: i32) -> Self {
        match id {
            9 => Phase::Dying,
            10 => Phase::Hovering,
            _ => Phase::HoldingPattern,
        }
    }

    pub fn is_sitting(self) -> bool {
        self == Phase::Hovering
    }

    pub fn fly_speed(self) -> f32 {
        match self {
            Phase::HoldingPattern => 0.6,
            Phase::Dying => 3.0,
            Phase::Hovering => 1.0,
        }
    }
}

pub fn turn_speed(velocity: DVec3) -> f32 {
    let speed = velocity.x.hypot(velocity.z) as f32 + 1.0;
    0.7 / speed.min(40.0) / speed
}

pub fn normalize(v: DVec3) -> DVec3 {
    let length = v.length();
    if length < f64::from(1.0e-5_f32) {
        DVec3::ZERO
    } else {
        v / length
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Node {
    fn distance_sqr(self, other: Node) -> f32 {
        let (dx, dy, dz) = (
            (other.x - self.x) as f32,
            (other.y - self.y) as f32,
            (other.z - self.z) as f32,
        );
        dx * dx + dy * dy + dz * dz
    }

    fn distance(self, other: Node) -> f32 {
        self.distance_sqr(other).sqrt()
    }
}

pub fn node_column(index: usize) -> (i32, i32, i32) {
    let i = index as f32;
    let (radius, step, at, lift) = if index < 12 {
        (60.0, 0.261_799_4_f32, i, 5)
    } else if index < 20 {
        (40.0, 0.392_699_1_f32, i - 12.0, 15)
    } else {
        (20.0, 0.785_398_2_f32, i - 20.0, 5)
    };
    let angle = 2.0 * (-std::f32::consts::PI + step * at);
    let x = (radius * cos(f64::from(angle))).floor() as i32;
    let z = (radius * sin(f64::from(angle))).floor() as i32;
    (x, z, lift)
}

pub fn nodes(height: impl Fn(i32, i32) -> i32) -> [Node; NODE_COUNT] {
    std::array::from_fn(|index| {
        let (x, z, lift) = node_column(index);
        Node {
            x,
            y: 73.max(height(x, z) + lift),
            z,
        }
    })
}

pub fn closest_node(nodes: &[Node; NODE_COUNT], at: DVec3, crystals_gone: bool) -> usize {
    let here = Node {
        x: at.x.floor() as i32,
        y: at.y.floor() as i32,
        z: at.z.floor() as i32,
    };
    let start = if crystals_gone { 12 } else { 0 };
    let mut best = 10000.0_f32;
    let mut found = 0;
    for (index, node) in nodes.iter().enumerate().skip(start) {
        let distance = node.distance_sqr(here);
        if distance < best {
            best = distance;
            found = index;
        }
    }
    found
}

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub nodes: Vec<Node>,
    pub next: usize,
}

impl Path {
    pub fn is_done(&self) -> bool {
        self.next >= self.nodes.len()
    }
}

#[derive(Clone, Copy, Default)]
struct Search {
    g: f32,
    h: f32,
    f: f32,
    came_from: Option<usize>,
    closed: bool,
    heap: i32,
}

struct Heap {
    items: Vec<usize>,
}

impl Heap {
    fn insert(&mut self, node: usize, search: &mut [Search; NODE_COUNT]) {
        self.items.push(node);
        search[node].heap = self.items.len() as i32 - 1;
        self.up(self.items.len() - 1, search);
    }

    fn pop(&mut self, search: &mut [Search; NODE_COUNT]) -> usize {
        let top = self.items[0];
        let last = self.items.pop().expect("a node to pop");
        if !self.items.is_empty() {
            self.items[0] = last;
            self.down(0, search);
        }
        search[top].heap = -1;
        top
    }

    fn change_cost(&mut self, node: usize, cost: f32, search: &mut [Search; NODE_COUNT]) {
        let old = search[node].f;
        search[node].f = cost;
        if cost < old {
            self.up(search[node].heap as usize, search);
        } else {
            self.down(search[node].heap as usize, search);
        }
    }

    fn up(&mut self, mut at: usize, search: &mut [Search; NODE_COUNT]) {
        let node = self.items[at];
        let cost = search[node].f;
        while at > 0 {
            let parent = (at - 1) >> 1;
            let above = self.items[parent];
            if cost >= search[above].f {
                break;
            }
            self.items[at] = above;
            search[above].heap = at as i32;
            at = parent;
        }
        self.items[at] = node;
        search[node].heap = at as i32;
    }

    fn down(&mut self, mut at: usize, search: &mut [Search; NODE_COUNT]) {
        let node = self.items[at];
        let cost = search[node].f;
        loop {
            let left = 1 + (at << 1);
            let right = left + 1;
            if left >= self.items.len() {
                break;
            }
            let left_node = self.items[left];
            let left_cost = search[left_node].f;
            let (right_node, right_cost) = if right >= self.items.len() {
                (None, f32::INFINITY)
            } else {
                (Some(self.items[right]), search[self.items[right]].f)
            };
            if left_cost < right_cost {
                if left_cost >= cost {
                    break;
                }
                self.items[at] = left_node;
                search[left_node].heap = at as i32;
                at = left;
            } else {
                let Some(right_node) = right_node.filter(|_| right_cost < cost) else {
                    break;
                };
                self.items[at] = right_node;
                search[right_node].heap = at as i32;
                at = right;
            }
        }
        self.items[at] = node;
        search[node].heap = at as i32;
    }
}

pub fn find_path(
    nodes: &[Node; NODE_COUNT],
    from: usize,
    to: usize,
    crystals_gone: bool,
) -> Option<Path> {
    let mut search = [Search {
        heap: -1,
        ..Search::default()
    }; NODE_COUNT];
    let target = nodes[to];
    search[from].h = nodes[from].distance(target);
    search[from].f = search[from].h;
    let mut open = Heap { items: Vec::new() };
    open.insert(from, &mut search);
    let mut closest = from;
    let start = if crystals_gone { 12 } else { 0 };
    while !open.items.is_empty() {
        let current = open.pop(&mut search);
        if nodes[current] == target {
            return Some(reconstruct(nodes, &search, current));
        }
        if nodes[current].distance(target) < nodes[closest].distance(target) {
            closest = current;
        }
        search[current].closed = true;
        for next in start..NODE_COUNT {
            if NODE_ADJACENCY[current] & (1 << next) == 0 || search[next].closed {
                continue;
            }
            let g = search[current].g + nodes[current].distance(nodes[next]);
            let open_already = search[next].heap >= 0;
            if open_already && g >= search[next].g {
                continue;
            }
            search[next].came_from = Some(current);
            search[next].g = g;
            search[next].h = nodes[next].distance(target);
            if open_already {
                let cost = search[next].g + search[next].h;
                open.change_cost(next, cost, &mut search);
            } else {
                search[next].f = search[next].g + search[next].h;
                open.insert(next, &mut search);
            }
        }
    }
    (closest != from).then(|| reconstruct(nodes, &search, closest))
}

fn reconstruct(nodes: &[Node; NODE_COUNT], search: &[Search; NODE_COUNT], end: usize) -> Path {
    let mut list = vec![nodes[end]];
    let mut at = end;
    while let Some(previous) = search[at].came_from {
        list.insert(0, nodes[previous]);
        at = previous;
    }
    Path {
        nodes: list,
        next: 0,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Holding {
    pub path: Option<Path>,
    pub target: Option<DVec3>,
    pub clockwise: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerBody {
    pub id: u64,
    pub feet: DVec3,
    pub height: f64,
    pub attackable: bool,
}

pub struct DragonEnv<'a> {
    pub height: &'a dyn Fn(i32, i32) -> i32,
    pub podium_top: &'a dyn Fn() -> i32,
    pub alive_crystals: Option<i32>,
    pub players: &'a [PlayerBody],
    pub crystals: &'a [(u64, DVec3)],
    pub death_experience: Option<i32>,
}

pub fn death_experience(
    fight_dragon: bool,
    previously_killed: bool,
    mob_drops: bool,
) -> Option<i32> {
    mob_drops.then_some(if fight_dragon && !previously_killed {
        FIRST_KILL_EXPERIENCE
    } else {
        LATER_KILL_EXPERIENCE
    })
}

pub fn experience_share(total: i32, part: f32) -> i32 {
    (total as f32 * part).floor() as i32
}

pub fn heard_at(event: &str, position: DVec3, camera: DVec3) -> DVec3 {
    if event != DRAGON_DEATH_SOUND {
        return position;
    }
    let center = position.floor() + DVec3::splat(0.5);
    camera + normalize(center - camera) * 2.0
}

pub const CRYSTAL_RANGE: f64 = 32.0;
pub const WIDTH: f64 = 16.0;
pub const HEIGHT: f64 = 8.0;
pub const CRYSTAL_DESTROYED_DAMAGE: f32 = 10.0;

pub fn crystal_in_reach(dragon: DVec3, crystal: DVec3) -> bool {
    let half = WIDTH / 2.0 + CRYSTAL_RANGE;
    let reach = (
        dragon - DVec3::new(half, CRYSTAL_RANGE, half),
        dragon + DVec3::new(half, HEIGHT + CRYSTAL_RANGE, half),
    );
    boxes_meet(reach, crate::end_crystal::pick_box(crystal))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DragonEvents {
    pub sounds: Vec<(&'static str, DVec3, f32, f32)>,
    pub player_hits: Vec<(u64, f32, DVec3)>,
    pub experience: Vec<(DVec3, i32)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hurt {
    pub applied: bool,
    pub full: bool,
    pub died: bool,
}

pub fn part_damage(part: usize, amount: f32) -> f32 {
    if part == HEAD || part == NECK {
        amount
    } else {
        amount / 4.0 + amount.min(1.0)
    }
}

pub fn hurts_the_dragon(damage: &EndDamage) -> bool {
    damage.player.is_some() || damage.kind == DamageKind::Explosion
}

pub fn blast_damage(feet: DVec3, center: DVec3, radius: f32) -> Option<f32> {
    let reach = f64::from(radius * 2.0);
    if reach < f64::from(1.0e-5_f32) {
        return None;
    }
    let distance = feet.distance(center) / reach;
    if distance > 1.0 {
        return None;
    }
    let impact = 1.0 - distance;
    Some(((impact * impact + impact) / 2.0 * 7.0 * reach + 1.0) as f32)
}

#[derive(Clone, Debug)]
pub struct EnderDragon {
    pub id: u64,
    pub uuid: [i32; 4],
    pub position: DVec3,
    pub previous_position: DVec3,
    pub velocity: DVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_rot_a: f32,
    pub health: f32,
    pub flap_time: f32,
    pub o_flap_time: f32,
    pub in_wall: bool,
    pub death_time: i32,
    pub history: FlightHistory,
    pub parts: [DVec3; 8],
    pub phase: Phase,
    pub holding: Holding,
    pub hover_target: Option<DVec3>,
    pub death_target: Option<DVec3>,
    pub hurt_time: i32,
    pub damage_cooldown: i32,
    pub last_hurt: f32,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub sitting_damage_received: f32,
    pub no_ai: bool,
    pub removed: bool,
    pub fight_origin: (i32, i32, i32),
    nodes: Option<[Node; NODE_COUNT]>,
    pub nearest_crystal: Option<u64>,
    pub random: LegacyRandom,
    tag: Tag,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DragonView {
    pub id: u64,
    pub uuid: [i32; 4],
    pub position: DVec3,
    pub previous_position: DVec3,
    pub y_rot: f32,
    pub flap_time: f32,
    pub o_flap_time: f32,
    pub history: Vec<Sample>,
    pub health: f32,
    pub hurt_time: i32,
    pub death_time: i32,
    pub sitting: bool,
    pub parts: [DVec3; 8],
    pub tick_count: i32,
    pub nearest_crystal: Option<u64>,
}

fn doubles(tag: &Tag, key: &str) -> Option<DVec3> {
    let Some(Tag::List(values)) = tag.get(key) else {
        return None;
    };
    let value = |i: usize| values.get(i).and_then(Tag::as_f64);
    Some(DVec3::new(value(0)?, value(1)?, value(2)?))
}

fn floats(tag: &Tag, key: &str) -> Option<(f32, f32)> {
    let Some(Tag::List(values)) = tag.get(key) else {
        return None;
    };
    let value = |i: usize| values.get(i).and_then(Tag::as_f64).map(|v| v as f32);
    Some((value(0)?, value(1)?))
}

pub fn part_box(position: DVec3, part: usize) -> (DVec3, DVec3) {
    let (width, height) = PART_SIZES[part];
    let half = width / 2.0;
    (
        position - DVec3::new(half, 0.0, half),
        position + DVec3::new(half, height, half),
    )
}

pub fn boxes_meet(a: (DVec3, DVec3), b: (DVec3, DVec3)) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
}

impl EnderDragon {
    pub fn from_tag(tag: &Tag, id: u64) -> Option<Self> {
        if tag.get("id").and_then(Tag::as_str) != Some(ENDER_DRAGON) {
            return None;
        }
        let position = doubles(tag, "Pos")?;
        let uuid: [i32; 4] = match tag.get("UUID") {
            Some(Tag::IntArray(uuid)) => uuid.as_slice().try_into().ok()?,
            _ => return None,
        };
        let (y_rot, x_rot) = floats(tag, "Rotation").unwrap_or((0.0, 0.0));
        let int = |key: &str| tag.get(key).and_then(Tag::as_i64).map(|v| v as i32);
        let seed = uuid.iter().fold(0x00D2_A6F1_u64, |h, &w| {
            (h ^ u64::from(w as u32)).wrapping_mul(0x100_0000_01B3)
        });
        let mut dragon = Self {
            id,
            uuid,
            position,
            previous_position: position,
            velocity: doubles(tag, "Motion").unwrap_or(DVec3::ZERO),
            y_rot,
            x_rot,
            y_rot_a: 0.0,
            health: tag
                .get("Health")
                .and_then(Tag::as_f64)
                .map_or(MAX_HEALTH, |health| health as f32),
            flap_time: 0.0,
            o_flap_time: 0.0,
            in_wall: false,
            death_time: int("DragonDeathTime").unwrap_or(0),
            history: FlightHistory::default(),
            parts: [position; 8],
            phase: Phase::Hovering,
            holding: Holding::default(),
            hover_target: None,
            death_target: None,
            hurt_time: int("HurtTime").unwrap_or(0),
            damage_cooldown: 0,
            last_hurt: 0.0,
            tick_count: 0,
            ambient_sound_time: -80,
            sitting_damage_received: tag
                .get("sitting_damage_received")
                .and_then(Tag::as_f64)
                .map_or(0.0, |v| v as f32),
            no_ai: int("NoAI").unwrap_or(0) != 0,
            removed: false,
            fight_origin: (0, 0, 0),
            nodes: None,
            nearest_crystal: None,
            random: LegacyRandom::new(seed),
            tag: tag.clone(),
        };
        if let Some(phase) = int("DragonPhase") {
            dragon.set_phase(Phase::by_id(phase));
        }
        Some(dragon)
    }

    pub fn to_tag(&self) -> Tag {
        let mut tag = self.tag.clone();
        if let Tag::Compound(map) = &mut tag {
            map.insert(
                "Pos".to_owned(),
                Tag::List(self.position.to_array().map(Tag::Double).to_vec()),
            );
            map.insert(
                "Motion".to_owned(),
                Tag::List(self.velocity.to_array().map(Tag::Double).to_vec()),
            );
            map.insert(
                "Rotation".to_owned(),
                Tag::List(vec![Tag::Float(self.y_rot), Tag::Float(self.x_rot)]),
            );
            map.insert("Health".to_owned(), Tag::Float(self.health));
            map.insert("DragonPhase".to_owned(), Tag::Int(self.phase.id()));
            map.insert("DragonDeathTime".to_owned(), Tag::Int(self.death_time));
            map.insert(
                "sitting_damage_received".to_owned(),
                Tag::Float(self.sitting_damage_received),
            );
            map.insert("HurtTime".to_owned(), Tag::Short(self.hurt_time as i16));
        }
        tag
    }

    pub fn set_phase(&mut self, phase: Phase) {
        if self.phase == phase {
            return;
        }
        self.phase = phase;
        match phase {
            Phase::HoldingPattern => {
                self.holding = Holding {
                    clockwise: self.holding.clockwise,
                    ..Holding::default()
                }
            }
            Phase::Hovering => self.hover_target = None,
            Phase::Dying => self.death_target = None,
        }
    }

    pub fn dead_or_dying(&self) -> bool {
        self.health <= 0.0
    }

    pub fn hurt_part(
        &mut self,
        part: usize,
        damage: &EndDamage,
        events: &mut DragonEvents,
    ) -> Hurt {
        if self.phase == Phase::Dying {
            return Hurt::default();
        }
        let amount = part_damage(part, damage.amount);
        if amount < 0.01 || !hurts_the_dragon(damage) {
            return Hurt::default();
        }
        let before = self.health;
        let hurt = self.really_hurt(amount, events);
        if self.phase.is_sitting() {
            self.sitting_damage_received += before - self.health;
            if self.sitting_damage_received > 0.25 * MAX_HEALTH {
                self.sitting_damage_received = 0.0;
                self.set_phase(Phase::HoldingPattern);
            }
        }
        hurt
    }

    fn really_hurt(&mut self, amount: f32, events: &mut DragonEvents) -> Hurt {
        if self.dead_or_dying() {
            return Hurt::default();
        }
        let full = if self.damage_cooldown > 10 {
            if amount <= self.last_hurt {
                return Hurt::default();
            }
            self.health -= amount - self.last_hurt;
            self.last_hurt = amount;
            false
        } else {
            self.last_hurt = amount;
            self.damage_cooldown = 20;
            self.health -= amount;
            self.hurt_time = 10;
            true
        };
        let died = self.dead_or_dying();
        if full {
            let pitch = self.voice_pitch();
            let sound = if died { DEATH_SOUND } else { HURT_SOUND };
            events
                .sounds
                .push((sound, self.position, SOUND_VOLUME, pitch));
        }
        if died && !self.phase.is_sitting() {
            self.health = 1.0;
            self.set_phase(Phase::Dying);
        }
        Hurt {
            applied: true,
            full,
            died,
        }
    }

    pub fn blast(
        &mut self,
        center: DVec3,
        radius: f32,
        player: Option<u64>,
        events: &mut DragonEvents,
    ) {
        let mut hits = Vec::new();
        if let Some(amount) = blast_damage(self.position, center, radius) {
            hits.push((BODY, amount));
        }
        for (part, &feet) in self.parts.iter().enumerate() {
            if let Some(amount) = blast_damage(feet, center, radius) {
                hits.push((part, amount));
            }
        }
        for (part, amount) in hits {
            self.hurt_part(part, &EndDamage::explosion(amount, player), events);
        }
    }

    pub fn voice_pitch(&mut self) -> f32 {
        (self.random.next_float() - self.random.next_float()) * 0.2 + 1.0
    }

    fn head_y_offset(&self) -> f32 {
        if self.phase.is_sitting() {
            return -1.0;
        }
        (self.history.get(5).y - self.history.get(0).y) as f32
    }

    pub fn view(&self) -> DragonView {
        DragonView {
            id: self.id,
            uuid: self.uuid,
            position: self.position,
            previous_position: self.previous_position,
            y_rot: self.y_rot,
            flap_time: self.flap_time,
            o_flap_time: self.o_flap_time,
            history: self.history.recent(HISTORY_SENT),
            health: self.health,
            hurt_time: self.hurt_time,
            death_time: self.death_time,
            sitting: self.phase.is_sitting(),
            parts: self.parts,
            tick_count: self.tick_count,
            nearest_crystal: self.nearest_crystal,
        }
    }

    fn check_crystals(&mut self, crystals: &[(u64, DVec3)]) {
        if let Some(nearest) = self.nearest_crystal {
            if crystals.iter().all(|(id, _)| *id != nearest) {
                self.nearest_crystal = None;
            } else if self.tick_count % 10 == 0 && self.health < MAX_HEALTH {
                self.health = (self.health + 1.0).clamp(0.0, MAX_HEALTH);
            }
        }
        if self.random.next_int(10) == 0 {
            let mut best = f64::MAX;
            let mut nearest = None;
            for &(id, position) in crystals {
                if !crystal_in_reach(self.position, position) {
                    continue;
                }
                let distance = position.distance_squared(self.position);
                if distance < best {
                    best = distance;
                    nearest = Some(id);
                }
            }
            self.nearest_crystal = nearest;
        }
    }

    pub fn crystal_destroyed(
        &mut self,
        crystal: u64,
        player: Option<u64>,
        events: &mut DragonEvents,
    ) -> Hurt {
        if self.nearest_crystal != Some(crystal) {
            return Hurt::default();
        }
        self.hurt_part(
            HEAD,
            &EndDamage::explosion(CRYSTAL_DESTROYED_DAMAGE, player),
            events,
        )
    }

    pub fn tick(&mut self, env: &DragonEnv) -> DragonEvents {
        let mut events = DragonEvents::default();
        self.previous_position = self.position;
        self.tick_count += 1;
        if self.hurt_time > 0 {
            self.hurt_time -= 1;
        }
        if self.damage_cooldown > 0 {
            self.damage_cooldown -= 1;
        }
        if self.dead_or_dying() {
            self.tick_death(env, &mut events);
        }
        if !self.dead_or_dying() && !self.removed {
            let roll = self.random.next_int(1000) as i32;
            let time = self.ambient_sound_time;
            self.ambient_sound_time += 1;
            if roll < time {
                self.ambient_sound_time = -80;
                let pitch = self.voice_pitch();
                events
                    .sounds
                    .push((AMBIENT_SOUND, self.position, SOUND_VOLUME, pitch));
            }
        }
        self.ai_step(env, &mut events);
        events
    }

    fn tick_death(&mut self, env: &DragonEnv, events: &mut DragonEvents) {
        self.death_time += 1;
        if let Some(total) = env
            .death_experience
            .filter(|_| self.death_time > 150 && self.death_time % 5 == 0)
        {
            events
                .experience
                .push((self.position, experience_share(total, 0.08)));
        }
        if self.death_time == 1 {
            events
                .sounds
                .push((DRAGON_DEATH_SOUND, self.position, SOUND_VOLUME, 1.0));
        }
        let rise = DVec3::new(0.0, 0.1, 0.0);
        self.position += rise;
        for part in &mut self.parts {
            *part += rise;
        }
        if self.death_time >= DEATH_TICKS {
            if let Some(total) = env.death_experience {
                events
                    .experience
                    .push((self.position, experience_share(total, 0.2)));
            }
            self.removed = true;
        }
    }

    fn ai_step(&mut self, env: &DragonEnv, events: &mut DragonEvents) {
        self.o_flap_time = self.flap_time;
        if self.dead_or_dying() {
            return;
        }
        self.check_crystals(env.crystals);
        let velocity = self.velocity;
        let mut flap = 0.2 / (velocity.x.hypot(velocity.z) as f32 * 10.0 + 1.0);
        flap *= 2.0_f64.powf(velocity.y) as f32;
        if self.phase.is_sitting() {
            self.flap_time += 0.1;
        } else if self.in_wall {
            self.flap_time += flap * 0.5;
        } else {
            self.flap_time += flap;
        }
        self.y_rot = wrap_degrees(self.y_rot);
        if self.no_ai {
            self.flap_time = 0.5;
            return;
        }
        self.history.record(self.position.y, self.y_rot);
        self.phase_tick(env);
        let target = match self.phase {
            Phase::HoldingPattern => self.holding.target,
            Phase::Hovering => self.hover_target,
            Phase::Dying => self.death_target,
        };
        if let Some(target) = target {
            self.fly_toward(target);
        }
        self.place_parts(events, env);
    }

    fn phase_tick(&mut self, env: &DragonEnv) {
        match self.phase {
            Phase::HoldingPattern => {
                let distance = self
                    .holding
                    .target
                    .map_or(0.0, |target| target.distance_squared(self.position));
                if !(100.0..=22500.0).contains(&distance) {
                    self.find_new_target(env);
                }
            }
            Phase::Hovering => {
                if self.hover_target.is_none() {
                    self.hover_target = Some(self.position);
                }
            }
            Phase::Dying => {
                let target = *self.death_target.get_or_insert_with(|| {
                    let (x, _, z) = self.fight_origin;
                    DVec3::new(
                        f64::from(x) + 0.5,
                        f64::from((env.podium_top)()),
                        f64::from(z) + 0.5,
                    )
                });
                let distance = target.distance_squared(self.position);
                self.health = if !(100.0..=22500.0).contains(&distance) {
                    0.0
                } else {
                    1.0
                };
            }
        }
    }

    fn crystals_gone(env: &DragonEnv) -> bool {
        env.alive_crystals.is_none_or(|alive| alive == 0)
    }

    fn find_new_target(&mut self, env: &DragonEnv) {
        if self.holding.path.as_ref().is_none_or(Path::is_done) {
            let nodes = *self
                .nodes
                .get_or_insert_with(|| nodes(|x, z| (env.height)(x, z)));
            let gone = Self::crystals_gone(env);
            let start = closest_node(&nodes, self.position, gone);
            let mut goal = start as i32;
            if self.random.next_int(8) == 0 {
                self.holding.clockwise = !self.holding.clockwise;
                goal += 6;
            }
            if self.holding.clockwise {
                goal += 1;
            } else {
                goal -= 1;
            }
            if env.alive_crystals.is_none_or(|alive| alive < 0) {
                goal -= 12;
                goal &= 7;
                goal += 12;
            } else {
                goal %= 12;
                if goal < 0 {
                    goal += 12;
                }
            }
            self.holding.path = find_path(&nodes, start, goal as usize, gone);
            if let Some(path) = self.holding.path.as_mut() {
                path.next += 1;
            }
        }
        self.navigate_to_next_node();
    }

    fn navigate_to_next_node(&mut self) {
        let Some(path) = self.holding.path.as_mut().filter(|path| !path.is_done()) else {
            return;
        };
        let node = path.nodes[path.next];
        path.next += 1;
        let base = f64::from(node.y);
        let y = loop {
            let y = f64::from(node.y as f32 + self.random.next_float() * 20.0);
            if y >= base {
                break y;
            }
        };
        self.holding.target = Some(DVec3::new(f64::from(node.x), y, f64::from(node.z)));
    }

    fn fly_toward(&mut self, target: DVec3) {
        let dx = target.x - self.position.x;
        let mut dy = target.y - self.position.y;
        let dz = target.z - self.position.z;
        let distance = dx * dx + dy * dy + dz * dz;
        let speed = self.phase.fly_speed();
        let horizontal = (dx * dx + dz * dz).sqrt();
        if horizontal > 0.0 {
            dy = (dy / horizontal).clamp(-f64::from(speed), f64::from(speed));
        }
        self.velocity += DVec3::new(0.0, dy * 0.01, 0.0);
        self.y_rot = wrap_degrees(self.y_rot);
        let toward = normalize(target - self.position);
        let facing = normalize(DVec3::new(
            f64::from(sin(f64::from(self.y_rot * DEG))),
            self.velocity.y,
            f64::from(-cos(f64::from(self.y_rot * DEG))),
        ));
        let along = ((facing.dot(toward) as f32 + 0.5) / 1.5).max(0.0);
        let small = f64::from(1.0e-5_f32);
        if dx.abs() > small || dz.abs() > small {
            let turn = wrap_degrees(180.0 - atan2(dx, dz) as f32 * 57.295_776 - self.y_rot)
                .clamp(-50.0, 50.0);
            self.y_rot_a *= 0.8;
            self.y_rot_a += turn * turn_speed(self.velocity);
            self.y_rot += self.y_rot_a * 0.1;
        }
        let near = (2.0 / (distance + 1.0)) as f32;
        let push = 0.06 * (along * near + (1.0 - near));
        self.velocity += DVec3::new(
            f64::from(push * sin(f64::from(self.y_rot * DEG))),
            0.0,
            f64::from(-push * cos(f64::from(self.y_rot * DEG))),
        );
        let step = if self.in_wall {
            self.velocity * f64::from(0.8_f32)
        } else {
            self.velocity
        };
        self.position += step;
        let heading = normalize(self.velocity);
        let drag = 0.8 + 0.15 * (heading.dot(facing) + 1.0) / 2.0;
        self.velocity *= DVec3::new(drag, f64::from(0.91_f32), drag);
    }

    fn place_parts(&mut self, events: &mut DragonEvents, env: &DragonEnv) {
        let at = |offset: DVec3| self.position + offset;
        let tilt = (self.history.get(5).y - self.history.get(10).y) as f32 * 10.0 * DEG;
        let (tilt_cos, tilt_sin) = (cos(f64::from(tilt)), sin(f64::from(tilt)));
        let yaw = self.y_rot * DEG;
        let (yaw_sin, yaw_cos) = (sin(f64::from(yaw)), cos(f64::from(yaw)));
        let mut parts = self.parts;
        parts[BODY] = at(DVec3::new(
            f64::from(yaw_sin * 0.5),
            0.0,
            f64::from(-yaw_cos * 0.5),
        ));
        parts[6] = at(DVec3::new(
            f64::from(yaw_cos * 4.5),
            2.0,
            f64::from(yaw_sin * 4.5),
        ));
        parts[7] = at(DVec3::new(
            f64::from(yaw_cos * -4.5),
            2.0,
            f64::from(yaw_sin * -4.5),
        ));
        if self.hurt_time <= 0 {
            for part in [HEAD, NECK] {
                let (low, high) = part_box(self.parts[part], part);
                let reach = (low - DVec3::ONE, high + DVec3::ONE);
                for player in env.players.iter().filter(|player| player.attackable) {
                    let half = 0.3;
                    let body = (
                        player.feet - DVec3::new(half, 0.0, half),
                        player.feet + DVec3::new(half, player.height, half),
                    );
                    if boxes_meet(reach, body) {
                        events
                            .player_hits
                            .push((player.id, CONTACT_DAMAGE, self.position));
                    }
                }
            }
        }
        let head_yaw = self.y_rot * DEG - self.y_rot_a * 0.01;
        let (head_sin, head_cos) = (sin(f64::from(head_yaw)), cos(f64::from(head_yaw)));
        let lift = self.head_y_offset();
        parts[HEAD] = at(DVec3::new(
            f64::from(head_sin * 6.5 * tilt_cos),
            f64::from(lift + tilt_sin * 6.5),
            f64::from(-head_cos * 6.5 * tilt_cos),
        ));
        parts[NECK] = at(DVec3::new(
            f64::from(head_sin * 5.5 * tilt_cos),
            f64::from(lift + tilt_sin * 5.5),
            f64::from(-head_cos * 5.5 * tilt_cos),
        ));
        let root = self.history.get(5);
        for k in 0..3 {
            let sample = self.history.get(12 + k * 2);
            let turn = self.y_rot * DEG
                + wrap_degrees_f64(f64::from(sample.y_rot - root.y_rot)) as f32 * DEG;
            let (turn_sin, turn_cos) = (sin(f64::from(turn)), cos(f64::from(turn)));
            let length = (k + 1) as f32 * 2.0;
            parts[3 + k] = at(DVec3::new(
                f64::from(-(yaw_sin * 1.5 + turn_sin * length) * tilt_cos),
                sample.y - root.y - f64::from((length + 1.5) * tilt_sin) + 1.5,
                f64::from((yaw_cos * 1.5 + turn_cos * length) * tilt_cos),
            ));
        }
        self.parts = parts;
    }
}

pub struct Dragons {
    pub list: Vec<EnderDragon>,
    next_id: u64,
}

impl Default for Dragons {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            next_id: FIRST_ID,
        }
    }
}

impl Dragons {
    pub fn add(&mut self, tag: &Tag) -> Option<u64> {
        let dragon = EnderDragon::from_tag(tag, self.next_id + 1)?;
        self.next_id += 1;
        self.list.push(dragon);
        Some(self.next_id)
    }

    pub fn fresh_tag(position: DVec3, y_rot: f32, random: &mut LegacyRandom) -> Tag {
        let most = (random.next_long() & !0xF000) | 0x4000;
        let least = (random.next_long() & 0x3FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000;
        let uuid = [
            (most >> 32) as i32,
            most as i32,
            (least >> 32) as i32,
            least as i32,
        ];
        Tag::Compound(BTreeMap::from([
            ("id".to_owned(), Tag::String(ENDER_DRAGON.to_owned())),
            (
                "Pos".to_owned(),
                Tag::List(position.to_array().map(Tag::Double).to_vec()),
            ),
            (
                "Rotation".to_owned(),
                Tag::List(vec![Tag::Float(y_rot), Tag::Float(0.0)]),
            ),
            ("UUID".to_owned(), Tag::IntArray(uuid.to_vec())),
            ("Health".to_owned(), Tag::Float(MAX_HEALTH)),
        ]))
    }

    pub fn by_uuid(&self, uuid: [i32; 4]) -> Option<&EnderDragon> {
        self.list.iter().find(|dragon| dragon.uuid == uuid)
    }

    pub fn tags_where(&self, keep: impl Fn(DVec3) -> bool) -> Vec<Tag> {
        self.list
            .iter()
            .filter(|dragon| !dragon.removed && keep(dragon.position))
            .map(EnderDragon::to_tag)
            .collect()
    }

    pub fn remove_where(&mut self, remove: impl Fn(DVec3) -> bool) {
        self.list.retain(|dragon| !remove(dragon.position));
    }

    pub fn views(&self) -> Vec<DragonView> {
        self.list.iter().map(EnderDragon::view).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(_: i32, _: i32) -> i32 {
        62
    }

    fn env<'a>(
        height: &'a dyn Fn(i32, i32) -> i32,
        top: &'a dyn Fn() -> i32,
        alive: Option<i32>,
    ) -> DragonEnv<'a> {
        DragonEnv {
            height,
            podium_top: top,
            alive_crystals: alive,
            players: &[],
            crystals: &[],
            death_experience: None,
        }
    }

    fn dragon_at(position: DVec3, y_rot: f32) -> EnderDragon {
        let mut random = LegacyRandom::new(5);
        let tag = Dragons::fresh_tag(position, y_rot, &mut random);
        let mut dragon = EnderDragon::from_tag(&tag, FIRST_ID + 1).unwrap();
        dragon.set_phase(Phase::HoldingPattern);
        dragon
    }

    #[test]
    fn the_flight_history_keeps_sixty_four_samples_newest_first() {
        let mut history = FlightHistory::default();
        history.record(10.0, 5.0);
        assert_eq!(
            history.get(0),
            Sample {
                y: 10.0,
                y_rot: 5.0
            }
        );
        assert_eq!(
            history.get(63),
            Sample {
                y: 10.0,
                y_rot: 5.0
            },
            "the first sample fills it"
        );
        for i in 1..70 {
            history.record(10.0 + f64::from(i), 0.0);
        }
        assert_eq!(history.get(0).y, 79.0);
        assert_eq!(history.get(5).y, 74.0);
        assert_eq!(history.get(64).y, 79.0, "it wraps");
        let recent = history.recent(3);
        let half = sample_at(&recent, 0, 0.5);
        assert_eq!(half.y, 78.5);
        assert_eq!(rot_lerp(0.5, 170.0, -170.0), 180.0, "the short way round");
    }

    #[test]
    fn the_twenty_four_nodes_ring_the_island_at_sixty_forty_and_twenty_blocks() {
        let nodes = nodes(flat);
        for (index, radius) in [(0, 60), (12, 40), (20, 20)] {
            assert!(
                (nodes[index].x - radius).abs() <= 1 && nodes[index].z.abs() <= 1,
                "{index}: {:?}",
                nodes[index]
            );
        }
        assert_eq!(nodes[0].y, 73, "never below 73");
        let high = nodes_at(|_, _| 80);
        assert_eq!((high[0].y, high[12].y, high[20].y), (85, 95, 85));
        for (i, node) in nodes.iter().enumerate() {
            let radius = f64::from(node.x).hypot(f64::from(node.z));
            let ring = if i < 12 {
                60.0
            } else if i < 20 {
                40.0
            } else {
                20.0
            };
            assert!((radius - ring).abs() < 1.5, "{i}: {radius}");
        }
        assert_eq!(closest_node(&nodes, DVec3::new(59.0, 80.0, 0.0), false), 0);
        assert_eq!(
            closest_node(&nodes, DVec3::new(59.0, 80.0, 0.0), true),
            12,
            "without crystals only the inner rings"
        );
    }

    fn nodes_at(height: impl Fn(i32, i32) -> i32) -> [Node; NODE_COUNT] {
        nodes(height)
    }

    #[test]
    fn a_path_follows_the_node_graph_to_its_goal() {
        let nodes = nodes(flat);
        let path = find_path(&nodes, 0, 1, false).expect("a path");
        assert_eq!(path.nodes.first(), Some(&nodes[0]));
        assert_eq!(path.nodes.last(), Some(&nodes[1]));
        for pair in path.nodes.windows(2) {
            let from = nodes.iter().position(|n| *n == pair[0]).unwrap();
            let to = nodes.iter().position(|n| *n == pair[1]).unwrap();
            assert!(NODE_ADJACENCY[from] & (1 << to) != 0, "{from} -> {to}");
        }
        let across = find_path(&nodes, 0, 6, false).expect("across the ring");
        assert_eq!(across.nodes.last(), Some(&nodes[6]));
        assert!(across.nodes.len() > 2);
        let inner = find_path(&nodes, 12, 13, true).expect("an inner path");
        assert!(
            inner
                .nodes
                .iter()
                .all(|node| nodes.iter().position(|n| n == node).unwrap() >= 12)
        );
    }

    #[test]
    fn a_holding_dragon_circles_the_island_between_its_nodes() {
        let mut dragon = dragon_at(DVec3::new(0.5, 128.0, 0.5), 37.0);
        let height = flat;
        let top = || 64;
        let env = env(&height, &top, Some(10));
        let mut lowest = f64::MAX;
        let mut farthest: f64 = 0.0;
        let mut targets = std::collections::HashSet::new();
        for _ in 0..3000 {
            dragon.tick(&env);
            lowest = lowest.min(dragon.position.y);
            farthest = farthest.max(dragon.position.x.hypot(dragon.position.z));
            if let Some(target) = dragon.holding.target {
                targets.insert((target.x as i32, target.z as i32));
            }
        }
        assert!(
            targets.len() >= 4,
            "it moves on from node to node: {targets:?}"
        );
        assert!(lowest > 60.0, "it stays in the air: {lowest}");
        assert!(farthest < 100.0, "it stays over the island: {farthest}");
        assert!(dragon.flap_time > 10.0, "its wings beat");
        let body = dragon.parts[BODY] - dragon.position;
        assert!(body.length() < 1.0);
        let wing = dragon.parts[6] - dragon.position;
        assert!((wing.y - 2.0).abs() < 1e-9 && (wing.x.hypot(wing.z) - 4.5).abs() < 1e-3);
        let head = dragon.parts[HEAD] - dragon.position;
        assert!(head.x.hypot(head.z) > 3.0, "the head reaches out in front");
    }

    #[test]
    fn a_dragon_without_ai_stays_put_and_a_summoned_one_hovers() {
        let mut random = LegacyRandom::new(1);
        let tag = Dragons::fresh_tag(DVec3::new(0.0, 100.0, 0.0), 0.0, &mut random);
        let mut summoned = EnderDragon::from_tag(&tag, FIRST_ID).unwrap();
        assert_eq!(summoned.phase, Phase::Hovering, "vanilla's default phase");
        let height = flat;
        let top = || 64;
        let env = env(&height, &top, None);
        for _ in 0..200 {
            summoned.tick(&env);
        }
        assert!(
            summoned.position.distance(DVec3::new(0.0, 100.0, 0.0)) < 10.0,
            "{:?}",
            summoned.position
        );
        let mut frozen = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        frozen.no_ai = true;
        frozen.tick(&env);
        assert_eq!(frozen.position, DVec3::new(0.0, 100.0, 0.0));
        assert_eq!(frozen.flap_time, 0.5);
    }

    #[test]
    fn a_dragon_keeps_its_phase_health_and_uuid_through_a_save() {
        let mut dragon = dragon_at(DVec3::new(3.0, 90.0, -4.0), 12.0);
        dragon.health = 150.0;
        dragon.death_time = 0;
        let tag = dragon.to_tag();
        assert_eq!(tag.get("DragonPhase"), Some(&Tag::Int(0)));
        assert_eq!(tag.get("Health"), Some(&Tag::Float(150.0)));
        let back = EnderDragon::from_tag(&tag, 9).unwrap();
        assert_eq!(back.uuid, dragon.uuid);
        assert_eq!(back.phase, Phase::HoldingPattern);
        assert_eq!(back.health, 150.0);
        assert_eq!(back.position, dragon.position);
        assert_eq!(back.y_rot, 12.0);
        let mut dragons = Dragons::default();
        let id = dragons.add(&tag).unwrap();
        assert!(id > FIRST_ID, "dragon ids never meet the entity world's");
        assert_eq!(dragons.by_uuid(dragon.uuid).map(|d| d.id), Some(id));
        assert_eq!(
            Phase::by_id(3),
            Phase::HoldingPattern,
            "phases not ported yet fall back"
        );
    }

    #[test]
    fn only_the_head_and_neck_take_full_damage_and_only_from_players_and_explosions() {
        assert_eq!(part_damage(HEAD, 20.0), 20.0);
        assert_eq!(part_damage(NECK, 20.0), 20.0);
        assert_eq!(part_damage(BODY, 20.0), 6.0);
        assert_eq!(part_damage(6, 0.5), 0.625);
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        let mut events = DragonEvents::default();
        let shot = dragon.hurt_part(HEAD, &EndDamage::attack(20.0, 0), &mut events);
        assert_eq!(
            shot,
            Hurt {
                applied: true,
                full: true,
                died: false
            }
        );
        assert_eq!(dragon.health, 180.0);
        assert_eq!(dragon.hurt_time, 10);
        assert_eq!(events.sounds.len(), 1);
        assert_eq!(events.sounds[0].0, HURT_SOUND);
        let unowned = EndDamage {
            amount: 20.0,
            player: None,
            kind: DamageKind::Attack,
        };
        dragon.damage_cooldown = 0;
        assert!(
            !dragon.hurt_part(HEAD, &unowned, &mut events).applied,
            "no player, no damage"
        );
        assert!(
            dragon
                .hurt_part(BODY, &EndDamage::explosion(20.0, None), &mut events)
                .applied,
            "explosions always hurt it"
        );
        assert_eq!(dragon.health, 174.0);
    }

    #[test]
    fn hits_within_the_damage_cooldown_only_count_their_extra_damage() {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        let mut events = DragonEvents::default();
        dragon.hurt_part(HEAD, &EndDamage::attack(8.0, 0), &mut events);
        assert!(
            !dragon
                .hurt_part(HEAD, &EndDamage::attack(5.0, 0), &mut events)
                .applied
        );
        let more = dragon.hurt_part(HEAD, &EndDamage::attack(12.0, 0), &mut events);
        assert!(more.applied && !more.full);
        assert_eq!(dragon.health, 188.0);
        assert_eq!(events.sounds.len(), 1, "a partial hit makes no sound");
        dragon.damage_cooldown = 10;
        assert!(
            dragon
                .hurt_part(HEAD, &EndDamage::attack(3.0, 0), &mut events)
                .full
        );
        assert_eq!(dragon.health, 185.0);
    }

    #[test]
    fn a_killing_blow_sends_a_flying_dragon_to_die_and_a_hovering_one_down_at_once() {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        dragon.health = 5.0;
        let mut events = DragonEvents::default();
        let blow = dragon.hurt_part(HEAD, &EndDamage::attack(10.0, 0), &mut events);
        assert!(blow.died);
        assert_eq!((dragon.health, dragon.phase), (1.0, Phase::Dying));
        assert_eq!(events.sounds[0].0, DEATH_SOUND);
        assert!(
            !dragon
                .hurt_part(HEAD, &EndDamage::attack(50.0, 0), &mut events)
                .applied,
            "a dying dragon takes no more"
        );
        let height = flat;
        let top = || 64;
        for _ in 0..400 {
            dragon.tick(&env(&height, &top, None));
        }
        assert!(
            dragon.removed,
            "it rises and goes 200 ticks after its health runs out"
        );
        let mut random = LegacyRandom::new(3);
        let tag = Dragons::fresh_tag(DVec3::new(0.0, 100.0, 0.0), 0.0, &mut random);
        let mut hovering = EnderDragon::from_tag(&tag, FIRST_ID).unwrap();
        hovering.health = 5.0;
        hovering.hurt_part(HEAD, &EndDamage::attack(10.0, 0), &mut events);
        assert!(
            hovering.health <= 0.0 && hovering.phase == Phase::Hovering,
            "a sitting dragon dies where it is"
        );
    }

    fn die(total: Option<i32>) -> (Vec<DragonEvents>, EnderDragon) {
        let mut dragon = dragon_at(DVec3::new(0.0, 70.0, 0.0), 0.0);
        dragon.health = 0.0;
        let height = flat;
        let top = || 64;
        let env = DragonEnv {
            death_experience: total,
            ..env(&height, &top, None)
        };
        let mut ticks = Vec::new();
        while !dragon.removed {
            ticks.push(dragon.tick(&env));
        }
        (ticks, dragon)
    }

    #[test]
    fn the_death_adds_up_to_12000_experience_on_the_first_kill_and_500_after() {
        assert_eq!(death_experience(true, false, true), Some(12000));
        assert_eq!(death_experience(true, true, true), Some(500));
        assert_eq!(
            death_experience(false, false, true),
            Some(500),
            "a dragon outside the fight"
        );
        assert_eq!(death_experience(true, false, false), None, "mob_drops off");
        for (total, share, last) in [(12000, 960, 2400), (500, 40, 100)] {
            let (ticks, dragon) = die(Some(total));
            assert_eq!(ticks.len(), 200);
            assert_eq!(dragon.death_time, DEATH_TICKS);
            let awards: Vec<(usize, i32)> = ticks
                .iter()
                .enumerate()
                .flat_map(|(tick, events)| {
                    events.experience.iter().map(move |(_, xp)| (tick + 1, *xp))
                })
                .collect();
            let mut expected: Vec<(usize, i32)> =
                (155..=200).step_by(5).map(|tick| (tick, share)).collect();
            expected.push((200, last));
            assert_eq!(awards, expected);
            assert_eq!(awards.iter().map(|(_, xp)| xp).sum::<i32>(), total);
        }
        let (ticks, _) = die(None);
        assert!(ticks.iter().all(|events| events.experience.is_empty()));
    }

    #[test]
    fn the_dying_dragon_roars_once_and_rises_twenty_blocks() {
        let (ticks, dragon) = die(Some(500));
        let roars: Vec<usize> = ticks
            .iter()
            .enumerate()
            .filter(|(_, events)| {
                events
                    .sounds
                    .iter()
                    .any(|sound| sound.0 == DRAGON_DEATH_SOUND)
            })
            .map(|(tick, _)| tick + 1)
            .collect();
        assert_eq!(roars, [1]);
        assert_eq!(ticks[0].sounds[0].2, SOUND_VOLUME);
        assert!(
            (dragon.position.y - 90.0).abs() < 1e-6,
            "{:?}",
            dragon.position
        );
        let last = ticks[199].experience[1].0;
        assert!(
            (last.y - 90.0).abs() < 1e-6,
            "the last share comes after the last rise"
        );
    }

    #[test]
    fn the_death_roar_is_heard_two_blocks_from_the_camera_toward_the_dragon() {
        let camera = DVec3::new(100.5, 50.62, 0.5);
        let heard = heard_at(DRAGON_DEATH_SOUND, DVec3::new(0.2, 80.7, 0.9), camera);
        let toward = (DVec3::new(0.5, 80.5, 0.5) - camera).normalize();
        assert!((heard - (camera + toward * 2.0)).length() < 1e-9);
        let elsewhere = DVec3::new(3.0, 4.0, 5.0);
        assert_eq!(heard_at(HURT_SOUND, elsewhere, camera), elsewhere);
    }

    #[test]
    fn a_sitting_dragon_takes_off_after_a_quarter_of_its_health() {
        let mut random = LegacyRandom::new(4);
        let tag = Dragons::fresh_tag(DVec3::new(0.0, 100.0, 0.0), 0.0, &mut random);
        let mut dragon = EnderDragon::from_tag(&tag, FIRST_ID).unwrap();
        assert!(dragon.phase.is_sitting());
        let mut events = DragonEvents::default();
        dragon.hurt_part(HEAD, &EndDamage::attack(30.0, 0), &mut events);
        assert_eq!(dragon.phase, Phase::Hovering);
        dragon.damage_cooldown = 0;
        dragon.hurt_part(HEAD, &EndDamage::attack(30.0, 0), &mut events);
        assert_eq!(
            dragon.phase,
            Phase::HoldingPattern,
            "past 50 damage it takes off"
        );
        assert_eq!(dragon.sitting_damage_received, 0.0);
    }

    #[test]
    fn a_blast_hurts_the_dragon_once_through_its_nearest_part() {
        assert_eq!(blast_damage(DVec3::ZERO, DVec3::ZERO, 6.0), Some(85.0));
        assert_eq!(
            blast_damage(DVec3::new(12.5, 0.0, 0.0), DVec3::ZERO, 6.0),
            None
        );
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        let height = flat;
        let top = || 64;
        dragon.tick(&env(&height, &top, None));
        let mut events = DragonEvents::default();
        let head = dragon.parts[HEAD];
        dragon.blast(head, 6.0, None, &mut events);
        assert!(
            (dragon.health - (200.0 - 85.0)).abs() < 1e-3,
            "{}",
            dragon.health
        );
        assert_eq!(events.sounds.len(), 1);
    }

    #[test]
    fn a_crystal_within_32_blocks_heals_the_dragon_a_point_every_ten_ticks() {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        dragon.no_ai = true;
        dragon.health = 150.0;
        let height = flat;
        let top = || 64;
        let near = [
            (7, DVec3::new(30.0, 90.0, 0.0)),
            (8, DVec3::new(20.0, 95.0, 0.0)),
        ];
        let env = DragonEnv {
            height: &height,
            podium_top: &top,
            alive_crystals: None,
            players: &[],
            crystals: &near,
            death_experience: None,
        };
        for _ in 0..200 {
            dragon.tick(&env);
            if dragon.nearest_crystal.is_some() {
                break;
            }
        }
        assert_eq!(
            dragon.nearest_crystal,
            Some(8),
            "the nearest crystal in reach"
        );
        let before = dragon.health;
        for _ in 0..100 {
            dragon.tick(&env);
        }
        assert!(
            (dragon.health - before - 10.0).abs() <= 1.0,
            "{before} -> {}",
            dragon.health
        );
        assert_eq!(dragon.view().nearest_crystal, Some(8));
        dragon.health = MAX_HEALTH;
        for _ in 0..20 {
            dragon.tick(&env);
        }
        assert_eq!(dragon.health, MAX_HEALTH, "never above its maximum");
        let gone = DragonEnv {
            height: &height,
            podium_top: &top,
            alive_crystals: None,
            players: &[],
            crystals: &[],
            death_experience: None,
        };
        dragon.tick(&gone);
        assert_eq!(
            dragon.nearest_crystal, None,
            "a destroyed crystal drops the beam"
        );
        assert!(crystal_in_reach(DVec3::ZERO, DVec3::new(40.5, 0.0, 0.0)));
        assert!(!crystal_in_reach(DVec3::ZERO, DVec3::new(41.5, 0.0, 0.0)));
        assert!(!crystal_in_reach(DVec3::ZERO, DVec3::new(0.0, -35.0, 0.0)));
    }

    #[test]
    fn destroying_the_crystal_that_heals_it_blasts_the_dragon_s_head_for_10() {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        let mut events = DragonEvents::default();
        assert!(
            !dragon.crystal_destroyed(3, Some(0), &mut events).applied,
            "not its crystal"
        );
        dragon.nearest_crystal = Some(3);
        assert!(
            dragon.crystal_destroyed(3, None, &mut events).applied,
            "an explosion hurts it with or without a player"
        );
        assert_eq!(dragon.health, 190.0);
    }

    #[test]
    fn the_head_and_neck_strike_an_attackable_player_they_touch() {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        let height = flat;
        let top = || 64;
        dragon.tick(&env(&height, &top, None));
        let head = dragon.parts[HEAD];
        let players = [
            PlayerBody {
                id: 3,
                feet: head + DVec3::new(0.5, 0.0, 0.0),
                height: 1.8,
                attackable: true,
            },
            PlayerBody {
                id: 4,
                feet: head + DVec3::new(0.5, 0.0, 0.0),
                height: 1.8,
                attackable: false,
            },
        ];
        let env = DragonEnv {
            height: &height,
            podium_top: &top,
            alive_crystals: None,
            players: &players,
            crystals: &[],
            death_experience: None,
        };
        let events = dragon.tick(&env);
        assert!(
            events
                .player_hits
                .iter()
                .any(|hit| hit.0 == 3 && hit.1 == CONTACT_DAMAGE)
        );
        assert!(
            events.player_hits.iter().all(|hit| hit.0 != 4),
            "creative players are left alone"
        );
        dragon.hurt_time = 5;
        assert!(
            dragon.tick(&env).player_hits.is_empty(),
            "not while it was hurt recently"
        );
    }
}
