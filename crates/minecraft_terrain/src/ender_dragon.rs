use std::collections::BTreeMap;

use glam::DVec3;
use minecraftoss_core::entity_data::wrap_degrees;
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::mth::{atan2, cos, sin};
use minecraftoss_player::rng::LegacyRandom;

use crate::effect_cloud::{Breath, EffectCloud};
use crate::end_crystal::{DamageKind, EndDamage};
use crate::potions::PotionEffect;

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
pub const FLAME_BREATH_TICKS: i32 = 10;
pub const AMBIENT_SOUND: &str = "minecraft:entity.ender_dragon.ambient";
pub const HURT_SOUND: &str = "minecraft:entity.ender_dragon.hurt";
pub const DEATH_SOUND: &str = "minecraft:entity.generic.death";
pub const FLAP_SOUND: &str = "minecraft:entity.ender_dragon.flap";
pub const GROWL_SOUND: &str = "minecraft:entity.ender_dragon.growl";
pub const DRAGON_DEATH_SOUND: &str = "minecraft:entity.ender_dragon.death";
pub const FIRST_KILL_EXPERIENCE: i32 = 12000;
pub const LATER_KILL_EXPERIENCE: i32 = 500;
pub const DEATH_TICKS: i32 = 200;
pub const EYE_HEIGHT: f64 = 6.8;
pub const SHOOT_SOUND: &str = "minecraft:entity.ender_dragon.shoot";
pub const SHOOT_VOLUME: f32 = 10.0;
pub const FIREBALL_CHARGE: i32 = 5;
pub const FLAME_RADIUS: f32 = 5.0;
pub const FLAME_DURATION: i32 = 200;
pub const FLAME_TICKS: i32 = 200;
pub const FLAME_ATTACKS: i32 = 4;
pub const ROAR_TICKS: i32 = 40;
pub const SCAN_IDLE_TICKS: i32 = 100;
pub const SCAN_ATTACK_RANGE: f64 = 20.0;
pub const SCAN_CHARGE_RANGE: f64 = 150.0;
pub const CHARGE_RECOVERY_TICKS: i32 = 10;
pub const WING_DAMAGE: f32 = 5.0;
pub const WING_PUSH: f64 = 4.0;
pub const KNOCKBACK: f64 = 0.4;
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
    StrafePlayer,
    LandingApproach,
    Landing,
    Takeoff,
    SittingFlaming,
    SittingScanning,
    SittingAttacking,
    ChargingPlayer,
    Dying,
    Hovering,
}

impl Phase {
    pub fn id(self) -> i32 {
        match self {
            Phase::HoldingPattern => 0,
            Phase::StrafePlayer => 1,
            Phase::LandingApproach => 2,
            Phase::Landing => 3,
            Phase::Takeoff => 4,
            Phase::SittingFlaming => 5,
            Phase::SittingScanning => 6,
            Phase::SittingAttacking => 7,
            Phase::ChargingPlayer => 8,
            Phase::Dying => 9,
            Phase::Hovering => 10,
        }
    }

    pub fn by_id(id: i32) -> Self {
        match id {
            1 => Phase::StrafePlayer,
            2 => Phase::LandingApproach,
            3 => Phase::Landing,
            4 => Phase::Takeoff,
            5 => Phase::SittingFlaming,
            6 => Phase::SittingScanning,
            7 => Phase::SittingAttacking,
            8 => Phase::ChargingPlayer,
            9 => Phase::Dying,
            10 => Phase::Hovering,
            _ => Phase::HoldingPattern,
        }
    }

    pub fn is_sitting(self) -> bool {
        matches!(
            self,
            Phase::SittingFlaming
                | Phase::SittingScanning
                | Phase::SittingAttacking
                | Phase::Hovering
        )
    }

    pub fn fly_speed(self) -> f32 {
        match self {
            Phase::Landing => 1.5,
            Phase::ChargingPlayer | Phase::Dying => 3.0,
            Phase::Hovering => 1.0,
            _ => 0.6,
        }
    }

    pub fn turn_speed(self, velocity: DVec3) -> f32 {
        if self != Phase::Landing {
            return turn_speed(velocity);
        }
        let speed = velocity.x.hypot(velocity.z) as f32 + 1.0;
        speed.min(40.0) / speed
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
    last: Option<Node>,
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
            return Some(reconstruct(nodes, &search, current, last));
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
    (closest != from).then(|| reconstruct(nodes, &search, closest, last))
}

pub fn strafe_lift(dx: f64, dz: f64) -> f64 {
    (f64::from(0.4_f32) + (dx * dx + dz * dz).sqrt() / 80.0 - 1.0).min(10.0)
}

pub fn block_center(at: DVec3) -> DVec3 {
    at.floor() + DVec3::splat(0.5)
}

fn reconstruct(
    nodes: &[Node; NODE_COUNT],
    search: &[Search; NODE_COUNT],
    end: usize,
    last: Option<Node>,
) -> Path {
    let mut list = vec![nodes[end]];
    let mut at = end;
    while let Some(previous) = search[at].came_from {
        list.insert(0, nodes[previous]);
        at = previous;
    }
    list.extend(last);
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Strafe {
    pub path: Option<Path>,
    pub target: Option<DVec3>,
    pub attack_target: Option<u64>,
    pub fireball_charge: i32,
    pub clockwise: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Course {
    pub path: Option<Path>,
    pub target: Option<DVec3>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flame {
    pub ticks: i32,
    pub count: i32,
    pub cloud: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Charge {
    pub target: Option<DVec3>,
    pub time_since_charge: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerBody {
    pub id: u64,
    pub feet: DVec3,
    pub height: f64,
    pub eye_height: f64,
    pub attackable: bool,
    pub crouching: bool,
}

impl PlayerBody {
    pub fn eye(&self) -> DVec3 {
        self.feet + DVec3::new(0.0, self.eye_height, 0.0)
    }

    pub fn visibility(&self) -> f64 {
        if self.crouching { 0.8 } else { 1.0 }
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        let half = 0.3;
        (
            self.feet - DVec3::new(half, 0.0, half),
            self.feet + DVec3::new(half, self.height, half),
        )
    }
}

pub struct DragonEnv<'a> {
    pub height: &'a dyn Fn(i32, i32) -> i32,
    pub podium_top: &'a dyn Fn() -> i32,
    pub alive_crystals: Option<i32>,
    pub players: &'a [PlayerBody],
    pub crystals: &'a [(u64, DVec3)],
    pub death_experience: Option<i32>,
    pub sees: &'a dyn Fn(DVec3, DVec3) -> bool,
    pub empty: &'a dyn Fn(i32, i32, i32) -> bool,
    pub mobs: &'a [(u64, (DVec3, DVec3))],
    pub wall: &'a dyn Fn(i32, i32, i32) -> Wall,
    pub griefing: bool,
    pub peaceful: bool,
}

impl DragonEnv<'_> {
    pub fn targets(&self) -> &[PlayerBody] {
        if self.peaceful { &[] } else { self.players }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wall {
    Pass,
    Immune,
    Breaks,
    Stays,
}

pub fn wing_push(center: DVec3, feet: DVec3) -> DVec3 {
    let (dx, dz) = (feet.x - center.x, feet.z - center.z);
    let distance = (dx * dx + dz * dz).max(0.1);
    DVec3::new(
        dx / distance * WING_PUSH,
        f64::from(0.2_f32),
        dz / distance * WING_PUSH,
    )
}

pub fn knockback(velocity: DVec3, on_ground: bool, xd: f64, zd: f64) -> DVec3 {
    if xd * xd + zd * zd < f64::from(1.0e-5_f32) {
        return velocity;
    }
    let length = (xd * xd + zd * zd).sqrt();
    let (dx, dz) = (xd / length * KNOCKBACK, zd / length * KNOCKBACK);
    let y = if on_ground {
        (velocity.y / 2.0 + KNOCKBACK).min(KNOCKBACK)
    } else {
        velocity.y
    };
    DVec3::new(velocity.x / 2.0 - dx, y, velocity.z / 2.0 - dz)
}

pub fn view_vector(x_rot: f32, y_rot: f32) -> DVec3 {
    let pitch = f64::from(x_rot * DEG);
    let yaw = f64::from(-y_rot * DEG);
    let (yaw_cos, yaw_sin) = (cos(yaw), sin(yaw));
    let (pitch_cos, pitch_sin) = (cos(pitch), sin(pitch));
    DVec3::new(
        f64::from(yaw_sin * pitch_cos),
        f64::from(-pitch_sin),
        f64::from(yaw_cos * pitch_cos),
    )
}

pub fn nearest_player(
    players: &[PlayerBody],
    at: DVec3,
    accept: impl Fn(&PlayerBody) -> bool,
) -> Option<PlayerBody> {
    let mut best: Option<(f64, PlayerBody)> = None;
    for player in players.iter().filter(|player| player.attackable) {
        if !accept(player) {
            continue;
        }
        let distance = player.feet.distance_squared(at);
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, *player));
        }
    }
    best.map(|(_, player)| player)
}

pub fn distance_to_center_sqr(block: (i32, i32, i32), at: DVec3) -> f64 {
    let center = DVec3::new(
        f64::from(block.0) + 0.5,
        f64::from(block.1) + 0.5,
        f64::from(block.2) + 0.5,
    );
    center.distance_squared(at)
}

fn circle_goal(
    random: &mut LegacyRandom,
    start: usize,
    clockwise: &mut bool,
    inner: bool,
) -> usize {
    let mut goal = start as i32;
    if random.next_int(8) == 0 {
        *clockwise = !*clockwise;
        goal += 6;
    }
    if *clockwise {
        goal += 1;
    } else {
        goal -= 1;
    }
    if inner {
        goal -= 12;
        goal &= 7;
        goal += 12;
    } else {
        goal %= 12;
        if goal < 0 {
            goal += 12;
        }
    }
    goal as usize
}

fn next_waypoint(random: &mut LegacyRandom, path: &mut Option<Path>) -> Option<DVec3> {
    let path = path.as_mut().filter(|path| !path.is_done())?;
    let node = path.nodes[path.next];
    path.next += 1;
    let base = f64::from(node.y);
    let y = loop {
        let y = f64::from(node.y as f32 + random.next_float() * 20.0);
        if y >= base {
            break y;
        }
    };
    Some(DVec3::new(f64::from(node.x), y, f64::from(node.z)))
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
    pub player_pushes: Vec<(u64, DVec3)>,
    pub mob_hits: Vec<(u64, DVec3, f32)>,
    pub broken: Vec<(i32, i32, i32)>,
    pub wall_bursts: Vec<(i32, i32, i32)>,
    pub experience: Vec<(DVec3, i32)>,
    pub fireballs: Vec<(DVec3, DVec3)>,
    pub clouds: Vec<EffectCloud>,
    pub breath: Vec<DragonBreath>,
    pub explosions: Vec<(DVec3, bool)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreathKind {
    Landing,
    Flame,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragonBreath {
    pub head: DVec3,
    pub look: DVec3,
    pub motion: DVec3,
    pub kind: BreathKind,
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
    pub strafe: Strafe,
    pub pending_strafe: Option<u64>,
    pub approach: Course,
    pub landing_target: Option<DVec3>,
    pub takeoff: Course,
    pub takeoff_first_tick: bool,
    pub flame: Flame,
    pub flame_out: bool,
    pub scanning_time: i32,
    pub attacking_ticks: i32,
    pub charge: Charge,
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
    pub phase: i32,
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
            strafe: Strafe::default(),
            pending_strafe: None,
            approach: Course::default(),
            landing_target: None,
            takeoff: Course::default(),
            takeoff_first_tick: false,
            flame: Flame::default(),
            flame_out: false,
            scanning_time: 0,
            attacking_ticks: 0,
            charge: Charge::default(),
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
        if self.phase == Phase::SittingFlaming && self.flame.cloud {
            self.flame.cloud = false;
            self.flame_out = true;
        }
        self.phase = phase;
        match phase {
            Phase::HoldingPattern => {
                self.holding = Holding {
                    clockwise: self.holding.clockwise,
                    ..Holding::default()
                }
            }
            Phase::StrafePlayer => {
                self.strafe = Strafe {
                    clockwise: self.strafe.clockwise,
                    ..Strafe::default()
                }
            }
            Phase::LandingApproach => self.approach = Course::default(),
            Phase::Landing => self.landing_target = None,
            Phase::Takeoff => {
                self.takeoff_first_tick = true;
                self.takeoff = Course::default();
            }
            Phase::SittingFlaming => {
                self.flame.ticks = 0;
                self.flame.count += 1;
            }
            Phase::SittingScanning => self.scanning_time = 0,
            Phase::SittingAttacking => self.attacking_ticks = 0,
            Phase::ChargingPlayer => self.charge = Charge::default(),
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
        if let Some(from) = damage
            .from
            .filter(|_| hurt.full && damage.kind == DamageKind::Attack && !self.phase.is_sitting())
        {
            self.velocity = knockback(
                self.velocity,
                false,
                from.x - self.position.x,
                from.z - self.position.z,
            );
        }
        if self.phase.is_sitting() {
            self.sitting_damage_received += before - self.health;
            if self.sitting_damage_received > 0.25 * MAX_HEALTH {
                self.sitting_damage_received = 0.0;
                self.set_phase(Phase::Takeoff);
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
            phase: self.phase.id(),
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
        let hurt = if self.nearest_crystal == Some(crystal) {
            self.hurt_part(
                HEAD,
                &EndDamage::explosion(CRYSTAL_DESTROYED_DAMAGE, player),
                events,
            )
        } else {
            Hurt::default()
        };
        if self.phase == Phase::HoldingPattern {
            self.pending_strafe = player.or(self.pending_strafe);
        }
        hurt
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
        if !self.removed {
            self.ai_step(env, &mut events);
        }
        events
    }

    fn death_burst(&mut self) -> DVec3 {
        let a = (self.random.next_float() - 0.5) * 8.0;
        let b = (self.random.next_float() - 0.5) * 4.0;
        let c = (self.random.next_float() - 0.5) * 8.0;
        self.position + DVec3::new(f64::from(a), 2.0 + f64::from(b), f64::from(c))
    }

    fn tick_death(&mut self, env: &DragonEnv, events: &mut DragonEvents) {
        self.death_time += 1;
        if (180..=200).contains(&self.death_time) {
            let at = self.death_burst();
            events.explosions.push((at, true));
        }
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
            let at = self.death_burst();
            events.explosions.push((at, false));
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
        if let Some(id) = self.pending_strafe.take() {
            let player = env.targets().iter().find(|player| player.id == id);
            if let Some(player) = player.filter(|player| player.attackable) {
                if self.phase == Phase::HoldingPattern {
                    self.strafe_player(player, env);
                }
            }
        }
        let before = self.phase;
        self.phase_tick(env, events);
        if self.phase != before {
            self.phase_tick(env, events);
        }
        if self.phase == Phase::Landing {
            let breath = self.breath(env, BreathKind::Landing, self.velocity);
            events.breath.push(breath);
        }
        let target = match self.phase {
            Phase::HoldingPattern => self.holding.target,
            Phase::StrafePlayer => self.strafe.target,
            Phase::LandingApproach => self.approach.target,
            Phase::Landing => self.landing_target,
            Phase::Takeoff => self.takeoff.target,
            Phase::ChargingPlayer => self.charge.target,
            Phase::SittingFlaming | Phase::SittingScanning | Phase::SittingAttacking => None,
            Phase::Hovering => self.hover_target,
            Phase::Dying => self.death_target,
        };
        if let Some(target) = target {
            self.fly_toward(target);
        }
        self.place_parts(events, env);
    }

    fn phase_tick(&mut self, env: &DragonEnv, events: &mut DragonEvents) {
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
            Phase::StrafePlayer => self.strafe_tick(env, events),
            Phase::LandingApproach => {
                let distance = self
                    .approach
                    .target
                    .map_or(0.0, |target| target.distance_squared(self.position));
                if !(100.0..=22500.0).contains(&distance) {
                    self.approach_new_target(env);
                }
            }
            Phase::Landing => self.landing_tick(env),
            Phase::Takeoff => self.takeoff_tick(env),
            Phase::SittingFlaming => self.flaming_tick(env, events),
            Phase::SittingScanning => self.scanning_tick(env),
            Phase::SittingAttacking => {
                let ticks = self.attacking_ticks;
                self.attacking_ticks += 1;
                if ticks >= ROAR_TICKS {
                    self.set_phase(Phase::SittingFlaming);
                }
            }
            Phase::ChargingPlayer => self.charge_tick(),
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

    fn node_list(&mut self, env: &DragonEnv) -> [Node; NODE_COUNT] {
        *self
            .nodes
            .get_or_insert_with(|| nodes(|x, z| (env.height)(x, z)))
    }

    pub fn podium(&self, env: &DragonEnv) -> (i32, i32, i32) {
        let (x, _, z) = self.fight_origin;
        (x, (env.height)(x, z), z)
    }

    fn find_new_target(&mut self, env: &DragonEnv) {
        if self.holding.path.as_ref().is_some_and(Path::is_done) {
            let podium = self.podium(env);
            let alive = env.alive_crystals.unwrap_or(0);
            if self.random.next_int((alive + 3).max(1) as u32) == 0 {
                self.set_phase(Phase::LandingApproach);
                return;
            }
            let at = DVec3::new(
                f64::from(podium.0),
                f64::from(podium.1),
                f64::from(podium.2),
            );
            if let Some(player) = nearest_player(env.targets(), at, |_| true) {
                let odds = distance_to_center_sqr(podium, player.feet) / 512.0;
                if self.random.next_int((odds + 2.0) as i32 as u32) == 0
                    || self.random.next_int((alive + 2).max(1) as u32) == 0
                {
                    self.strafe_player(&player, env);
                    return;
                }
            }
        }
        if self.holding.path.as_ref().is_none_or(Path::is_done) {
            let nodes = self.node_list(env);
            let gone = Self::crystals_gone(env);
            let start = closest_node(&nodes, self.position, gone);
            let inner = env.alive_crystals.is_none_or(|alive| alive < 0);
            let goal = circle_goal(&mut self.random, start, &mut self.holding.clockwise, inner);
            self.holding.path = find_path(&nodes, start, goal, gone, None);
            if let Some(path) = self.holding.path.as_mut() {
                path.next += 1;
            }
        }
        if let Some(target) = next_waypoint(&mut self.random, &mut self.holding.path) {
            self.holding.target = Some(target);
        }
    }

    fn strafe_player(&mut self, player: &PlayerBody, env: &DragonEnv) {
        self.set_phase(Phase::StrafePlayer);
        self.strafe.attack_target = Some(player.id);
        let nodes = self.node_list(env);
        let gone = Self::crystals_gone(env);
        let start = closest_node(&nodes, self.position, gone);
        let goal = closest_node(&nodes, player.feet, gone);
        let (x, z) = (player.feet.x.floor() as i32, player.feet.z.floor() as i32);
        let (dx, dz) = (
            f64::from(x) - self.position.x,
            f64::from(z) - self.position.z,
        );
        let last = Node {
            x,
            y: (player.feet.y + strafe_lift(dx, dz)).floor() as i32,
            z,
        };
        self.strafe.path = find_path(&nodes, start, goal, gone, Some(last));
        if let Some(path) = self.strafe.path.as_mut() {
            path.next += 1;
            if let Some(target) = next_waypoint(&mut self.random, &mut self.strafe.path) {
                self.strafe.target = Some(target);
            }
        }
    }

    fn strafe_tick(&mut self, env: &DragonEnv, events: &mut DragonEvents) {
        let target = self
            .strafe
            .attack_target
            .and_then(|id| env.players.iter().find(|player| player.id == id))
            .copied();
        let Some(target) = target else {
            self.set_phase(Phase::HoldingPattern);
            return;
        };
        if self.strafe.path.as_ref().is_some_and(Path::is_done) {
            let (dx, dz) = (
                target.feet.x - self.position.x,
                target.feet.z - self.position.z,
            );
            self.strafe.target = Some(target.feet + DVec3::new(0.0, strafe_lift(dx, dz), 0.0));
        }
        let distance = self
            .strafe
            .target
            .map_or(0.0, |at| at.distance_squared(self.position));
        if !(100.0..=22500.0).contains(&distance) {
            self.strafe_new_target(env);
        }
        let near = target.feet.distance_squared(self.position) < 4096.0;
        if !near || !(env.sees)(self.eye(), target.eye()) {
            if self.strafe.fireball_charge > 0 {
                self.strafe.fireball_charge -= 1;
            }
            return;
        }
        self.strafe.fireball_charge += 1;
        let angle = self.facing_angle(target.feet);
        if self.strafe.fireball_charge < FIREBALL_CHARGE || !(0.0..10.0).contains(&angle) {
            return;
        }
        let view = view_vector(self.x_rot, self.y_rot);
        let head = self.parts[HEAD];
        let from = DVec3::new(head.x - view.x, head.y + 1.0, head.z - view.z);
        let aim = target.feet + DVec3::new(0.0, target.height * 0.5, 0.0) - from;
        let pitch = self.voice_pitch();
        events.sounds.push((
            SHOOT_SOUND,
            block_center(self.position),
            SHOOT_VOLUME,
            pitch,
        ));
        events.fireballs.push((from, normalize(aim)));
        self.strafe.fireball_charge = 0;
        if let Some(path) = self.strafe.path.as_mut() {
            path.next = path.next.max(path.nodes.len());
        }
        self.set_phase(Phase::HoldingPattern);
    }

    fn strafe_new_target(&mut self, env: &DragonEnv) {
        if self.strafe.path.as_ref().is_none_or(Path::is_done) {
            let nodes = self.node_list(env);
            let gone = Self::crystals_gone(env);
            let start = closest_node(&nodes, self.position, gone);
            let inner = env.alive_crystals.is_none_or(|alive| alive <= 0);
            let goal = circle_goal(&mut self.random, start, &mut self.strafe.clockwise, inner);
            self.strafe.path = find_path(&nodes, start, goal, gone, None);
            if let Some(path) = self.strafe.path.as_mut() {
                path.next += 1;
            }
        }
        if let Some(target) = next_waypoint(&mut self.random, &mut self.strafe.path) {
            self.strafe.target = Some(target);
        }
    }

    fn approach_new_target(&mut self, env: &DragonEnv) {
        if self.approach.path.as_ref().is_none_or(Path::is_done) {
            let nodes = self.node_list(env);
            let gone = Self::crystals_gone(env);
            let start = closest_node(&nodes, self.position, gone);
            let podium = self.podium(env);
            let at = DVec3::new(
                f64::from(podium.0),
                f64::from(podium.1),
                f64::from(podium.2),
            );
            let toward = match nearest_player(env.targets(), at, |_| true) {
                Some(player) => {
                    let away = normalize(DVec3::new(player.feet.x, 0.0, player.feet.z));
                    DVec3::new(-away.x * 40.0, 105.0, -away.z * 40.0)
                }
                None => DVec3::new(40.0, f64::from(podium.1), 0.0),
            };
            let goal = closest_node(&nodes, toward, gone);
            let last = Node {
                x: podium.0,
                y: podium.1,
                z: podium.2,
            };
            self.approach.path = find_path(&nodes, start, goal, gone, Some(last));
            if let Some(path) = self.approach.path.as_mut() {
                path.next += 1;
            }
        }
        if let Some(target) = next_waypoint(&mut self.random, &mut self.approach.path) {
            self.approach.target = Some(target);
        }
        if self.approach.path.as_ref().is_some_and(Path::is_done) {
            self.set_phase(Phase::Landing);
        }
    }

    fn landing_tick(&mut self, env: &DragonEnv) {
        let podium = self.podium(env);
        let target = *self.landing_target.get_or_insert(DVec3::new(
            f64::from(podium.0) + 0.5,
            f64::from(podium.1),
            f64::from(podium.2) + 0.5,
        ));
        if target.distance_squared(self.position) < 1.0 {
            self.flame.count = 0;
            self.set_phase(Phase::SittingScanning);
        }
    }

    fn takeoff_tick(&mut self, env: &DragonEnv) {
        if self.takeoff_first_tick || self.takeoff.path.is_none() {
            self.takeoff_first_tick = false;
            self.takeoff_new_target(env);
        } else if distance_to_center_sqr(self.podium(env), self.position) >= 100.0 {
            self.set_phase(Phase::HoldingPattern);
        }
    }

    fn takeoff_new_target(&mut self, env: &DragonEnv) {
        let nodes = self.node_list(env);
        let gone = Self::crystals_gone(env);
        let start = closest_node(&nodes, self.position, gone);
        let look = self.head_look_vector(env);
        let ahead = DVec3::new(-look.x * 40.0, 105.0, -look.z * 40.0);
        let mut goal = closest_node(&nodes, ahead, gone) as i32;
        if env.alive_crystals.is_none_or(|alive| alive <= 0) {
            goal -= 12;
            goal &= 7;
            goal += 12;
        } else {
            goal %= 12;
            if goal < 0 {
                goal += 12;
            }
        }
        self.takeoff.path = find_path(&nodes, start, goal as usize, gone, None);
        if let Some(path) = self.takeoff.path.as_mut() {
            path.next += 1;
        }
        if let Some(target) = next_waypoint(&mut self.random, &mut self.takeoff.path) {
            self.takeoff.target = Some(target);
        }
    }

    pub fn head_look_vector(&self, env: &DragonEnv) -> DVec3 {
        match self.phase {
            Phase::Landing | Phase::Takeoff => {
                let distance = distance_to_center_sqr(self.podium(env), self.position).sqrt();
                let far = (distance as f32 / 4.0).max(1.0);
                view_vector(-(6.0 / far) * 1.5 * 5.0, self.y_rot)
            }
            phase if phase.is_sitting() => view_vector(-45.0, self.y_rot),
            _ => view_vector(self.x_rot, self.y_rot),
        }
    }

    fn breath(&self, env: &DragonEnv, kind: BreathKind, motion: DVec3) -> DragonBreath {
        DragonBreath {
            head: self.parts[HEAD] + DVec3::new(0.0, PART_SIZES[HEAD].1 * 0.5, 0.0),
            look: self.head_look_vector(env).normalize_or_zero(),
            motion,
            kind,
        }
    }

    fn flaming_tick(&mut self, env: &DragonEnv, events: &mut DragonEvents) {
        self.flame.ticks += 1;
        if self.flame.ticks % 2 == 0 && self.flame.ticks < FLAME_BREATH_TICKS {
            let breath = self.breath(env, BreathKind::Flame, DVec3::ZERO);
            events.breath.push(breath);
        }
        if self.flame.ticks >= FLAME_TICKS {
            if self.flame.count >= FLAME_ATTACKS {
                self.set_phase(Phase::Takeoff);
            } else {
                self.set_phase(Phase::SittingScanning);
            }
            return;
        }
        if self.flame.ticks != 10 {
            return;
        }
        let head = self.parts[HEAD];
        let away = normalize(DVec3::new(
            head.x - self.position.x,
            0.0,
            head.z - self.position.z,
        ));
        let x = head.x + away.x * f64::from(FLAME_RADIUS) / 2.0;
        let z = head.z + away.z * f64::from(FLAME_RADIUS) / 2.0;
        let start = head.y + 0.5;
        let mut y = start;
        while (env.empty)(x.floor() as i32, y.floor() as i32, z.floor() as i32) {
            y -= 1.0;
            if y < 0.0 {
                y = start;
                break;
            }
        }
        let y = f64::from(y.floor() as i32 + 1);
        events.clouds.push(EffectCloud::dragon_breath(
            DVec3::new(x, y, z),
            FLAME_RADIUS,
            FLAME_DURATION,
            0.0,
            PotionEffect::new("minecraft:instant_damage", 0, 0),
            Breath::Flame(self.id),
        ));
        self.flame.cloud = true;
    }

    fn scanning_tick(&mut self, env: &DragonEnv) {
        self.scanning_time += 1;
        let (here, eye) = (self.position, self.eye());
        let in_view = |player: &PlayerBody, range: f64| {
            let reach = (range * player.visibility()).max(2.0);
            player.feet.distance_squared(here) <= reach * reach && (env.sees)(eye, player.eye())
        };
        let near = nearest_player(env.targets(), here, |player| {
            (player.feet.y - here.y).abs() <= 10.0 && in_view(player, SCAN_ATTACK_RANGE)
        });
        if let Some(player) = near {
            if self.scanning_time > 25 {
                self.set_phase(Phase::SittingAttacking);
                return;
            }
            let angle = self.facing_angle(player.feet);
            if angle < 0.0 || angle > 10.0 {
                let head = self.parts[HEAD];
                let (dx, dz) = (player.feet.x - head.x, player.feet.z - head.z);
                let turn = wrap_degrees_f64(
                    180.0 - atan2(dx, dz) * 57.295_776_367_187_5 - f64::from(self.y_rot),
                )
                .clamp(-100.0, 100.0);
                self.y_rot_a *= 0.8;
                let distance = (dx * dx + dz * dz).sqrt() as f32 + 1.0;
                let capped = distance.min(40.0);
                self.y_rot_a += turn as f32 * (0.7 / capped / distance);
                self.y_rot += self.y_rot_a;
            }
        } else if self.scanning_time >= SCAN_IDLE_TICKS {
            let charged = nearest_player(env.targets(), here, |player| {
                in_view(player, SCAN_CHARGE_RANGE)
            });
            self.set_phase(Phase::Takeoff);
            if let Some(player) = charged {
                self.set_phase(Phase::ChargingPlayer);
                self.charge.target = Some(player.feet);
            }
        }
    }

    fn charge_tick(&mut self) {
        let Some(target) = self.charge.target else {
            self.set_phase(Phase::HoldingPattern);
            return;
        };
        if self.charge.time_since_charge > 0 {
            let time = self.charge.time_since_charge;
            self.charge.time_since_charge += 1;
            if time >= CHARGE_RECOVERY_TICKS {
                self.set_phase(Phase::HoldingPattern);
                return;
            }
        }
        let distance = target.distance_squared(self.position);
        if !(100.0..=22500.0).contains(&distance) {
            self.charge.time_since_charge += 1;
        }
    }

    pub fn eye(&self) -> DVec3 {
        self.position + DVec3::new(0.0, EYE_HEIGHT, 0.0)
    }

    fn facing_angle(&self, toward: DVec3) -> f32 {
        let to = normalize(DVec3::new(
            toward.x - self.position.x,
            0.0,
            toward.z - self.position.z,
        ));
        let yaw = f64::from(self.y_rot * DEG);
        let facing = normalize(DVec3::new(f64::from(sin(yaw)), 0.0, f64::from(-cos(yaw))));
        let dot = facing.dot(to) as f32;
        (f64::from(dot).acos() * 57.295_776_367_187_5) as f32 + 0.5
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
            self.y_rot_a += turn * self.phase.turn_speed(self.velocity);
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

    fn knock_back(
        &self,
        reach: (DVec3, DVec3),
        center: DVec3,
        env: &DragonEnv,
        events: &mut DragonEvents,
    ) {
        let damage = if self.phase.is_sitting() {
            0.0
        } else {
            WING_DAMAGE
        };
        for player in env.players.iter().filter(|player| player.attackable) {
            if boxes_meet(reach, player.bounds()) {
                events
                    .player_pushes
                    .push((player.id, wing_push(center, player.feet)));
                if damage > 0.0 {
                    events.player_hits.push((player.id, damage, self.position));
                }
            }
        }
        for &(id, bounds) in env.mobs {
            if boxes_meet(reach, bounds) {
                let feet = DVec3::new(
                    (bounds.0.x + bounds.1.x) / 2.0,
                    bounds.0.y,
                    (bounds.0.z + bounds.1.z) / 2.0,
                );
                events.mob_hits.push((id, wing_push(center, feet), damage));
            }
        }
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
            let center = parts[BODY];
            for wing in [6, 7] {
                let (low, high) = part_box(parts[wing], wing);
                let reach = (
                    low - DVec3::new(4.0, 4.0, 4.0),
                    high + DVec3::new(4.0, 0.0, 4.0),
                );
                self.knock_back(reach, center, env, events);
            }
            for part in [HEAD, NECK] {
                let (low, high) = part_box(self.parts[part], part);
                let reach = (low - DVec3::ONE, high + DVec3::ONE);
                for player in env.players.iter().filter(|player| player.attackable) {
                    if boxes_meet(reach, player.bounds()) {
                        events
                            .player_hits
                            .push((player.id, CONTACT_DAMAGE, self.position));
                    }
                }
                for &(id, bounds) in env.mobs {
                    if boxes_meet(reach, bounds) {
                        events.mob_hits.push((id, DVec3::ZERO, CONTACT_DAMAGE));
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
        let mut in_wall = false;
        for part in [HEAD, NECK, BODY] {
            in_wall |= self.check_walls(part_box(self.parts[part], part), env, events);
        }
        self.in_wall = in_wall;
    }

    fn check_walls(
        &mut self,
        area: (DVec3, DVec3),
        env: &DragonEnv,
        events: &mut DragonEvents,
    ) -> bool {
        let (low, high) = (area.0.floor(), area.1.floor());
        let (x0, y0, z0) = (low.x as i32, low.y as i32, low.z as i32);
        let (x1, y1, z1) = (high.x as i32, high.y as i32, high.z as i32);
        let mut hit = false;
        let mut removed = false;
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    if events.broken.contains(&(x, y, z)) {
                        continue;
                    }
                    match (env.wall)(x, y, z) {
                        Wall::Pass => {}
                        Wall::Immune => hit = true,
                        _ if !env.griefing => hit = true,
                        Wall::Breaks => {
                            events.broken.push((x, y, z));
                            removed = true;
                        }
                        Wall::Stays => {}
                    }
                }
            }
        }
        if removed {
            let x = x0 + self.random.next_int((x1 - x0 + 1) as u32) as i32;
            let y = y0 + self.random.next_int((y1 - y0 + 1) as u32) as i32;
            let z = z0 + self.random.next_int((z1 - z0 + 1) as u32) as i32;
            events.wall_bursts.push((x, y, z));
        }
        hit
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
            sees: &|_, _| true,
            empty: &|_, _, _| true,
            mobs: &[],
            wall: &|_, _, _| Wall::Pass,
            griefing: true,
            peaceful: false,
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
        let path = find_path(&nodes, 0, 1, false, None).expect("a path");
        assert_eq!(path.nodes.first(), Some(&nodes[0]));
        assert_eq!(path.nodes.last(), Some(&nodes[1]));
        for pair in path.nodes.windows(2) {
            let from = nodes.iter().position(|n| *n == pair[0]).unwrap();
            let to = nodes.iter().position(|n| *n == pair[1]).unwrap();
            assert!(NODE_ADJACENCY[from] & (1 << to) != 0, "{from} -> {to}");
        }
        let across = find_path(&nodes, 0, 6, false, None).expect("across the ring");
        assert_eq!(across.nodes.last(), Some(&nodes[6]));
        assert!(across.nodes.len() > 2);
        let inner = find_path(&nodes, 12, 13, true, None).expect("an inner path");
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
        for id in 0..=10 {
            assert_eq!(Phase::by_id(id).id(), id, "vanilla's phase ids");
        }
        assert_eq!(
            Phase::by_id(11),
            Phase::HoldingPattern,
            "unknown ids fall back"
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
            from: None,
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
    fn a_dying_dragon_explodes_every_tick_and_bursts_from_tick_180_to_200() {
        let (ticks, dragon) = die(None);
        assert_eq!(ticks.len(), 200);
        for (tick, events) in ticks.iter().enumerate() {
            let death_time = tick as i32 + 1;
            let emitters: Vec<DVec3> = events
                .explosions
                .iter()
                .filter(|(_, emitter)| *emitter)
                .map(|(at, _)| *at)
                .collect();
            let plain: Vec<DVec3> = events
                .explosions
                .iter()
                .filter(|(_, emitter)| !*emitter)
                .map(|(at, _)| *at)
                .collect();
            assert_eq!(
                emitters.len(),
                usize::from((180..=200).contains(&death_time)),
                "{death_time}"
            );
            assert_eq!(
                plain.len(),
                usize::from(death_time < DEATH_TICKS),
                "aiStep does not run once it is removed, {death_time}"
            );
            let near = |at: DVec3, feet_y: f64| {
                at.x.abs() <= 4.0 && at.z.abs() <= 4.0 && (at.y - feet_y - 2.0).abs() <= 2.0 + 1e-9
            };
            for at in emitters {
                assert!(near(at, 70.0 + 0.1 * f64::from(death_time - 1)), "{at:?}");
            }
            for at in plain {
                assert!(near(at, 70.0 + 0.1 * f64::from(death_time)), "{at:?}");
            }
        }
        let all: Vec<&(DVec3, bool)> = ticks.iter().flat_map(|events| &events.explosions).collect();
        assert_eq!(all.iter().filter(|(_, emitter)| !*emitter).count(), 199);
        assert_eq!(all.iter().filter(|(_, emitter)| *emitter).count(), 21);
        assert!(dragon.removed);
    }

    #[test]
    fn the_dying_dragon_s_random_draws_three_floats_a_tick() {
        let mut dragon = dragon_at(DVec3::new(0.0, 70.0, 0.0), 0.0);
        dragon.health = 0.0;
        let mut mirror = dragon.random.clone();
        let height = flat;
        let top = || 64;
        let env = env(&height, &top, None);
        let mut burst = |at: DVec3| {
            let a = (mirror.next_float() - 0.5) * 8.0;
            let b = (mirror.next_float() - 0.5) * 4.0;
            let c = (mirror.next_float() - 0.5) * 8.0;
            at + DVec3::new(f64::from(a), 2.0 + f64::from(b), f64::from(c))
        };
        while !dragon.removed {
            let before = dragon.position;
            let events = dragon.tick(&env);
            let mut expected = Vec::new();
            if (180..=200).contains(&dragon.death_time) {
                expected.push((burst(before), true));
            }
            if !dragon.removed {
                expected.push((burst(dragon.position), false));
            }
            assert_eq!(events.explosions, expected, "{}", dragon.death_time);
        }
        assert_eq!(dragon.random.next_int(1 << 30), mirror.next_int(1 << 30));
    }

    fn assert_close(actual: DVec3, expected: DVec3) {
        assert!((actual - expected).length() < 1e-9, "{actual:?} {expected:?}");
    }

    #[test]
    fn a_hit_knocks_a_flying_dragon_away_from_the_attacker() {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        dragon.velocity = DVec3::new(0.6, -0.2, 0.4);
        let mut events = DragonEvents::default();
        let from = DVec3::new(-3.0, 90.0, 4.0);
        let hit = EndDamage::attack(10.0, 0).from_position(from);
        assert!(dragon.hurt_part(6, &hit, &mut events).full);
        let (dx, dz) = (-3.0 / 5.0 * KNOCKBACK, 4.0 / 5.0 * KNOCKBACK);
        assert_close(dragon.velocity, DVec3::new(0.3 - dx, -0.2, 0.2 - dz));
        let before = dragon.velocity;
        dragon.damage_cooldown = 0;
        assert!(
            dragon
                .hurt_part(HEAD, &EndDamage::attack(10.0, 0), &mut events)
                .full
        );
        assert_eq!(dragon.velocity, before, "a hit from nowhere does not push");
        dragon.damage_cooldown = 0;
        dragon.health = 5.0;
        dragon.velocity = DVec3::ZERO;
        let killing = EndDamage::attack(10.0, 0).from_position(DVec3::new(0.0, 0.0, -2.0));
        assert!(dragon.hurt_part(HEAD, &killing, &mut events).died);
        assert_eq!(dragon.phase, Phase::Dying);
        assert_close(dragon.velocity, DVec3::new(0.0, 0.0, KNOCKBACK));
    }

    #[test]
    fn a_sitting_or_dying_dragon_is_not_knocked_back() {
        let mut events = DragonEvents::default();
        let hit = EndDamage::attack(10.0, 0).from_position(DVec3::new(8.0, 64.0, 0.0));
        let mut sitting = dragon_at(DVec3::new(0.0, 64.0, 0.0), 0.0);
        sitting.set_phase(Phase::SittingScanning);
        sitting.velocity = DVec3::new(0.1, 0.0, 0.1);
        assert!(sitting.hurt_part(HEAD, &hit, &mut events).full);
        assert_eq!(sitting.velocity, DVec3::new(0.1, 0.0, 0.1));
        let mut dying = dragon_at(DVec3::new(0.0, 64.0, 0.0), 0.0);
        dying.set_phase(Phase::Dying);
        dying.velocity = DVec3::new(0.1, 0.0, 0.1);
        assert!(!dying.hurt_part(HEAD, &hit, &mut events).applied);
        assert_eq!(dying.velocity, DVec3::new(0.1, 0.0, 0.1));
    }

    #[test]
    fn an_explosion_does_not_knock_the_dragon_back() {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        dragon.velocity = DVec3::new(0.2, 0.0, 0.0);
        let mut events = DragonEvents::default();
        let blast = EndDamage::explosion(10.0, Some(0)).from_position(DVec3::new(5.0, 100.0, 0.0));
        assert!(dragon.hurt_part(HEAD, &blast, &mut events).full);
        assert_eq!(dragon.velocity, DVec3::new(0.2, 0.0, 0.0));
    }

    #[test]
    fn only_a_full_hit_knocks_back_not_a_partial_one_in_the_cooldown() {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        let mut events = DragonEvents::default();
        let from = DVec3::new(10.0, 100.0, 0.0);
        dragon.velocity = DVec3::ZERO;
        assert!(
            dragon
                .hurt_part(HEAD, &EndDamage::attack(10.0, 0).from_position(from), &mut events)
                .full
        );
        assert_close(dragon.velocity, DVec3::new(-KNOCKBACK, 0.0, 0.0));
        dragon.velocity = DVec3::ZERO;
        let partial = dragon.hurt_part(HEAD, &EndDamage::attack(15.0, 0).from_position(from), &mut events);
        assert!(partial.applied && !partial.full);
        assert_eq!(dragon.velocity, DVec3::ZERO, "a partial hit in the cooldown");
        let wasted = dragon.hurt_part(HEAD, &EndDamage::attack(5.0, 0).from_position(from), &mut events);
        assert!(!wasted.applied);
        assert_eq!(dragon.velocity, DVec3::ZERO);
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
        assert_eq!(dragon.phase, Phase::Takeoff, "past 50 damage it takes off");
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
            sees: &|_, _| true,
            empty: &|_, _, _| true,
            mobs: &[],
            wall: &|_, _, _| Wall::Pass,
            griefing: true,
            peaceful: false,
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
            sees: &|_, _| true,
            empty: &|_, _, _| true,
            mobs: &[],
            wall: &|_, _, _| Wall::Pass,
            griefing: true,
            peaceful: false,
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
                eye_height: 1.62,
                attackable: true,
                crouching: false,
            },
            PlayerBody {
                id: 4,
                feet: head + DVec3::new(0.5, 0.0, 0.0),
                height: 1.8,
                eye_height: 1.62,
                attackable: false,
                crouching: false,
            },
        ];
        let env = DragonEnv {
            height: &height,
            podium_top: &top,
            alive_crystals: None,
            players: &players,
            crystals: &[],
            death_experience: None,
            sees: &|_, _| true,
            empty: &|_, _, _| true,
            mobs: &[],
            wall: &|_, _, _| Wall::Pass,
            griefing: true,
            peaceful: false,
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

    fn cells(area: (DVec3, DVec3)) -> Vec<(i32, i32, i32)> {
        let (low, high) = (area.0.floor(), area.1.floor());
        let mut cells = Vec::new();
        for x in low.x as i32..=high.x as i32 {
            for y in low.y as i32..=high.y as i32 {
                for z in low.z as i32..=high.z as i32 {
                    cells.push((x, y, z));
                }
            }
        }
        cells
    }

    #[test]
    fn the_dragon_breaks_what_its_head_neck_and_body_fly_through_but_not_immune_blocks() {
        let height = flat;
        let top = || 64;
        let mut dragon = dragon_at(DVec3::new(0.5, 100.0, 0.5), 0.0);
        dragon.tick(&env(&height, &top, None));
        let solid = |_: i32, _: i32, _: i32| Wall::Breaks;
        let mut breaking = dragon.clone();
        let events = breaking.tick(&DragonEnv {
            wall: &solid,
            ..env(&height, &top, None)
        });
        let mut expected = Vec::new();
        for part in [HEAD, NECK, BODY] {
            for cell in cells(part_box(breaking.parts[part], part)) {
                if !expected.contains(&cell) {
                    expected.push(cell);
                }
            }
        }
        assert_eq!(events.broken, expected, "each block once, head first");
        assert_eq!(
            events.wall_bursts.len(),
            3,
            "a burst for each part that broke some"
        );
        for (burst, part) in events.wall_bursts.iter().zip([HEAD, NECK, BODY]) {
            assert!(cells(part_box(breaking.parts[part], part)).contains(burst));
        }
        assert!(!breaking.in_wall, "breaking through does not slow it");
        let body = cells(part_box(dragon.parts[BODY], BODY));
        let one = body[body.len() / 2];
        let single = move |x: i32, y: i32, z: i32| {
            if (x, y, z) == one {
                Wall::Breaks
            } else {
                Wall::Pass
            }
        };
        let mut lone = dragon.clone();
        let events = lone.tick(&DragonEnv {
            wall: &single,
            ..env(&height, &top, None)
        });
        assert_eq!(events.broken, vec![one]);
        assert_eq!(events.wall_bursts.len(), 1);
        for (wall, griefing) in [
            (Wall::Immune, true),
            (Wall::Breaks, false),
            (Wall::Stays, false),
        ] {
            let blocks = move |_: i32, _: i32, _: i32| wall;
            let mut stuck = dragon.clone();
            let events = stuck.tick(&DragonEnv {
                wall: &blocks,
                griefing,
                ..env(&height, &top, None)
            });
            assert!(events.broken.is_empty() && events.wall_bursts.is_empty());
            assert!(stuck.in_wall, "{wall:?} with griefing {griefing} holds it");
        }
        let water = |_: i32, _: i32, _: i32| Wall::Stays;
        let mut wet = dragon.clone();
        let events = wet.tick(&DragonEnv {
            wall: &water,
            ..env(&height, &top, None)
        });
        assert!(
            events.broken.is_empty() && !wet.in_wall,
            "water neither breaks nor holds"
        );
        let start = wet.position;
        let mut free = wet.clone();
        free.in_wall = false;
        wet.in_wall = true;
        wet.tick(&env(&height, &top, None));
        free.tick(&env(&height, &top, None));
        assert!(
            wet.position.distance(start) < free.position.distance(start),
            "a dragon in a wall flies at 0.8 of its speed"
        );
    }

    #[test]
    fn the_wing_push_and_the_knockback_match_vanilla() {
        let push = wing_push(DVec3::new(0.0, 70.0, 0.0), DVec3::new(2.0, 64.0, 0.0));
        assert_eq!(push, DVec3::new(2.0, f64::from(0.2_f32), 0.0));
        let close = wing_push(DVec3::ZERO, DVec3::new(0.1, 0.0, 0.0));
        assert!((close.x - 4.0).abs() < 1e-12, "{close:?}");
        let far = wing_push(DVec3::ZERO, DVec3::new(3.0, 0.0, 4.0));
        assert!((far - DVec3::new(0.48, f64::from(0.2_f32), 0.64)).length() < 1e-12);
        let thrown = knockback(DVec3::new(0.2, -0.1, 0.0), true, -1.0, 0.0);
        assert!(
            (thrown - DVec3::new(0.5, 0.35, 0.0)).length() < 1e-12,
            "{thrown:?}"
        );
        let airborne = knockback(DVec3::new(0.0, 0.3, 0.6), false, 0.0, 2.0);
        assert!(
            (airborne - DVec3::new(0.0, 0.3, -0.1)).length() < 1e-12,
            "{airborne:?}"
        );
    }

    #[test]
    fn the_wings_throw_players_and_mobs_and_hurt_them_only_in_flight() {
        let height = flat;
        let top = || 64;
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        dragon.tick(&env(&height, &top, None));
        let right = dragon.parts[6] + DVec3::new(1.0, -2.0, 0.0);
        let left = dragon.parts[7] + DVec3::new(-1.0, -2.0, 0.0);
        let head = dragon.parts[HEAD];
        let players = [player(3, right, true), player(4, right, false)];
        let reach = |feet: DVec3| {
            (
                feet - DVec3::new(0.3, 0.0, 0.3),
                feet + DVec3::new(0.3, 2.9, 0.3),
            )
        };
        let mobs = [(9, reach(left)), (10, reach(head))];
        let flying = DragonEnv {
            players: &players,
            mobs: &mobs,
            ..env(&height, &top, None)
        };
        let events = dragon.tick(&flying);
        let center = dragon.parts[BODY];
        assert_eq!(
            events.player_pushes,
            vec![(3, wing_push(center, right))],
            "creative players stay put"
        );
        assert!(events.player_pushes[0].1.x > 0.5);
        assert!(
            events
                .player_hits
                .contains(&(3, WING_DAMAGE, dragon.position))
        );
        let thrown = events.mob_hits.iter().find(|hit| hit.0 == 9).unwrap();
        assert_eq!(*thrown, (9, wing_push(center, left), WING_DAMAGE));
        assert!(thrown.1.x < -0.5, "thrown away from the body");
        assert!(events.mob_hits.contains(&(10, DVec3::ZERO, CONTACT_DAMAGE)));
        dragon.hurt_time = 3;
        let events = dragon.tick(&flying);
        assert!(events.player_pushes.is_empty() && events.mob_hits.is_empty());
        let mut sitting = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        sitting.set_phase(Phase::SittingScanning);
        sitting.tick(&env(&height, &top, None));
        let right = sitting.parts[6] + DVec3::new(1.0, -2.0, 0.0);
        let players = [player(3, right, true)];
        let events = sitting.tick(&DragonEnv {
            players: &players,
            ..env(&height, &top, None)
        });
        assert_eq!(
            events.player_pushes.len(),
            1,
            "a sitting dragon still pushes"
        );
        assert!(
            events.player_hits.iter().all(|hit| hit.1 != WING_DAMAGE),
            "but its wings do no harm"
        );
    }

    fn player(id: u64, feet: DVec3, attackable: bool) -> PlayerBody {
        PlayerBody {
            id,
            feet,
            height: 1.8,
            eye_height: 1.62,
            attackable,
            crouching: false,
        }
    }

    fn with_players<'a>(
        base: DragonEnv<'a>,
        players: &'a [PlayerBody],
        sees: &'a dyn Fn(DVec3, DVec3) -> bool,
    ) -> DragonEnv<'a> {
        DragonEnv {
            players,
            sees,
            ..base
        }
    }

    #[test]
    fn the_view_vector_and_strafe_lift_match_vanilla() {
        let ahead = view_vector(0.0, 0.0);
        assert!((ahead - DVec3::Z).length() < 1e-6, "{ahead:?}");
        let west = view_vector(0.0, 90.0);
        assert!((west - DVec3::NEG_X).length() < 1e-6, "{west:?}");
        let up = view_vector(-45.0, 0.0);
        assert!((up.y - 0.707_106_8).abs() < 1e-3 && (up.z - 0.707_106_8).abs() < 1e-3);
        assert!((strafe_lift(80.0, 0.0) - f64::from(0.4_f32)).abs() < 1e-12);
        assert_eq!(strafe_lift(0.0, 1000.0), 10.0);
        assert!((strafe_lift(0.0, 0.0) + 0.6).abs() < 1e-6);
        assert_eq!(
            block_center(DVec3::new(-0.2, 64.9, 3.5)),
            DVec3::new(-0.5, 64.5, 3.5)
        );
    }

    #[test]
    fn a_path_can_end_at_a_node_of_its_own() {
        let nodes = nodes(flat);
        let last = Node { x: 7, y: 70, z: -3 };
        let path = find_path(&nodes, 0, 1, false, Some(last)).expect("a path");
        assert_eq!(path.nodes.last(), Some(&last));
        assert_eq!(path.nodes[path.nodes.len() - 2], nodes[1]);
        let same = find_path(&nodes, 4, 4, false, Some(last)).expect("from a node to itself");
        assert_eq!(same.nodes, vec![nodes[4], last]);
    }

    #[test]
    fn a_circling_dragon_at_the_end_of_its_path_strafes_the_nearest_attackable_player() {
        let height = flat;
        let top = || 64;
        let sees = |_: DVec3, _: DVec3| true;
        let players = [
            player(3, DVec3::new(2.0, 63.0, 1.0), false),
            player(4, DVec3::new(40.0, 63.0, 0.0), true),
            player(5, DVec3::new(90.0, 63.0, 0.0), true),
        ];
        assert_eq!(
            nearest_player(&players, DVec3::new(0.0, 63.0, 0.0), |_| true).map(|p| p.id),
            Some(4),
            "creative players are never picked"
        );
        assert_eq!(
            distance_to_center_sqr((0, 63, 0), DVec3::new(0.5, 63.5, 2.5)),
            4.0
        );
        let mut strafed = None;
        for seed in 0..20 {
            let mut dragon = dragon_at(DVec3::new(0.5, 100.0, 0.5), seed as f32 * 18.0);
            dragon.random = LegacyRandom::new(seed);
            let env = with_players(env(&height, &top, Some(10)), &players, &sees);
            for _ in 0..2000 {
                dragon.tick(&env);
                if dragon.phase == Phase::StrafePlayer {
                    strafed = Some(dragon);
                    break;
                }
            }
            if strafed.is_some() {
                break;
            }
        }
        let dragon = strafed.expect("sooner or later it strafes a player");
        assert_eq!(dragon.strafe.attack_target, Some(4));
        let path = dragon.strafe.path.as_ref().expect("a path to the player");
        let last = path.nodes.last().expect("a last node");
        assert_eq!((last.x, last.z), (40, 0), "it ends over the player");
        assert!((53..=73).contains(&last.y), "{last:?}");
        let mut alone = dragon_at(DVec3::new(0.5, 100.0, 0.5), 0.0);
        let creative = [players[0]];
        let env = with_players(env(&height, &top, Some(10)), &creative, &sees);
        for _ in 0..3000 {
            alone.tick(&env);
            assert_ne!(alone.phase, Phase::StrafePlayer, "nobody to strafe");
        }
    }

    fn strafing(target: DVec3) -> EnderDragon {
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        dragon.set_phase(Phase::StrafePlayer);
        dragon.strafe.attack_target = Some(4);
        dragon.strafe.target = Some(target);
        dragon
    }

    #[test]
    fn a_strafing_dragon_spits_a_fireball_after_five_ticks_with_the_player_in_sight_ahead() {
        let height = flat;
        let top = || 64;
        let sees = |_: DVec3, _: DVec3| true;
        let players = [player(4, DVec3::new(0.0, 90.0, -40.0), true)];
        let env = with_players(env(&height, &top, Some(10)), &players, &sees);
        let mut dragon = strafing(DVec3::new(0.0, 100.0, -80.0));
        let mut shots = Vec::new();
        for tick in 1..=10 {
            let before = (dragon.parts[HEAD], dragon.y_rot, dragon.position);
            let events = dragon.tick(&env);
            if !events.fireballs.is_empty() {
                shots.push((tick, events, before));
                break;
            }
            assert_eq!(dragon.strafe.fireball_charge, tick);
        }
        let (tick, events, (head, y_rot, at)) = shots.pop().expect("a fireball");
        assert_eq!(tick, FIREBALL_CHARGE);
        assert_eq!(dragon.phase, Phase::HoldingPattern, "then it circles again");
        let (from, direction) = events.fireballs[0];
        let view = view_vector(0.0, wrap_degrees(y_rot));
        assert_eq!(
            from,
            DVec3::new(head.x - view.x, head.y + 1.0, head.z - view.z)
        );
        let aim = (DVec3::new(0.0, 90.9, -40.0) - from).normalize();
        assert!((direction - aim).length() < 1e-9);
        let shot = events
            .sounds
            .iter()
            .find(|sound| sound.0 == SHOOT_SOUND)
            .expect("the shoot sound");
        assert_eq!(shot.2, SHOOT_VOLUME);
        assert_eq!(shot.1, block_center(at));
    }

    #[test]
    fn without_sight_or_out_of_range_or_facing_away_the_charge_runs_down() {
        let height = flat;
        let top = || 64;
        let blind = |_: DVec3, _: DVec3| false;
        let near = [player(4, DVec3::new(0.0, 90.0, -40.0), true)];
        let dark = with_players(env(&height, &top, Some(10)), &near, &blind);
        let mut dragon = strafing(DVec3::new(0.0, 100.0, -80.0));
        dragon.strafe.fireball_charge = 3;
        for expected in [2, 1, 0, 0] {
            assert!(dragon.tick(&dark).fireballs.is_empty());
            assert_eq!(dragon.strafe.fireball_charge, expected);
        }
        let sees = |_: DVec3, _: DVec3| true;
        let far = [player(4, DVec3::new(0.0, 90.0, -70.0), true)];
        let distant = with_players(env(&height, &top, Some(10)), &far, &sees);
        let mut dragon = strafing(DVec3::new(0.0, 100.0, -80.0));
        for _ in 0..10 {
            assert!(
                dragon.tick(&distant).fireballs.is_empty(),
                "64 blocks or more away"
            );
        }
        let behind = [player(4, DVec3::new(0.0, 90.0, 40.0), true)];
        let back = with_players(env(&height, &top, Some(10)), &behind, &sees);
        let mut dragon = strafing(DVec3::new(0.0, 100.0, -80.0));
        for _ in 0..3 {
            assert!(dragon.tick(&back).fireballs.is_empty(), "not behind it");
        }
        assert!(
            dragon.strafe.fireball_charge >= 3,
            "the charge still builds"
        );
    }

    #[test]
    fn a_strafe_without_its_player_falls_back_to_circling() {
        let height = flat;
        let top = || 64;
        let mut dragon = strafing(DVec3::new(0.0, 100.0, -80.0));
        dragon.tick(&env(&height, &top, Some(10)));
        assert_eq!(dragon.phase, Phase::HoldingPattern);
        let tag = strafing(DVec3::ZERO).to_tag();
        assert_eq!(tag.get("DragonPhase"), Some(&Tag::Int(1)));
        let mut loaded = EnderDragon::from_tag(&tag, 3).unwrap();
        assert_eq!(loaded.phase, Phase::StrafePlayer);
        loaded.tick(&env(&height, &top, Some(10)));
        assert_eq!(
            loaded.phase,
            Phase::HoldingPattern,
            "a saved strafe has no player"
        );
    }

    #[test]
    fn breaking_a_crystal_makes_a_circling_dragon_strafe_the_player_behind_it() {
        let height = flat;
        let top = || 64;
        let sees = |_: DVec3, _: DVec3| true;
        let players = [
            player(4, DVec3::new(30.0, 70.0, 0.0), true),
            player(5, DVec3::new(-30.0, 70.0, 0.0), false),
        ];
        let env = with_players(env(&height, &top, Some(10)), &players, &sees);
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        let mut events = DragonEvents::default();
        dragon.crystal_destroyed(9, Some(4), &mut events);
        assert_eq!(
            dragon.phase,
            Phase::HoldingPattern,
            "it turns on its next tick"
        );
        dragon.tick(&env);
        assert_eq!(dragon.phase, Phase::StrafePlayer);
        assert_eq!(dragon.strafe.attack_target, Some(4));
        let mut calm = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        calm.crystal_destroyed(9, Some(5), &mut events);
        calm.tick(&env);
        assert_eq!(calm.phase, Phase::HoldingPattern, "not a creative player");
        let mut busy = strafing(DVec3::new(0.0, 100.0, -80.0));
        busy.strafe.attack_target = Some(4);
        busy.crystal_destroyed(9, Some(4), &mut events);
        assert_eq!(busy.pending_strafe, None, "only a circling dragon turns");
    }

    fn ground(_: i32, y: i32, _: i32) -> bool {
        y >= 62
    }

    fn podium_top() -> i32 {
        62
    }

    fn podium_env<'a>(
        players: &'a [PlayerBody],
        sees: &'a dyn Fn(DVec3, DVec3) -> bool,
        alive: Option<i32>,
    ) -> DragonEnv<'a> {
        DragonEnv {
            height: &flat,
            podium_top: &podium_top,
            alive_crystals: alive,
            players,
            crystals: &[],
            death_experience: None,
            sees,
            empty: &ground,
            mobs: &[],
            wall: &|_, _, _| Wall::Pass,
            griefing: true,
            peaceful: false,
        }
    }

    fn phases_of(dragon: &mut EnderDragon, env: &DragonEnv, ticks: usize) -> Vec<(usize, Phase)> {
        let mut seen = vec![(0, dragon.phase)];
        for tick in 1..=ticks {
            dragon.tick(env);
            if seen.last().map(|(_, phase)| *phase) != Some(dragon.phase) {
                seen.push((tick, dragon.phase));
            }
        }
        seen
    }

    #[test]
    fn the_new_phases_keep_vanilla_speeds_and_sitting() {
        assert_eq!(Phase::Landing.fly_speed(), 1.5);
        assert_eq!(Phase::ChargingPlayer.fly_speed(), 3.0);
        assert_eq!(Phase::LandingApproach.fly_speed(), 0.6);
        assert_eq!(Phase::Takeoff.fly_speed(), 0.6);
        let sitting: Vec<Phase> = (0..=10)
            .map(Phase::by_id)
            .filter(|phase| phase.is_sitting())
            .collect();
        assert_eq!(
            sitting,
            [
                Phase::SittingFlaming,
                Phase::SittingScanning,
                Phase::SittingAttacking,
                Phase::Hovering
            ]
        );
        let slow = DVec3::new(0.3, 0.0, 0.4);
        assert_eq!(Phase::Landing.turn_speed(slow), 1.0);
        assert!((Phase::Takeoff.turn_speed(slow) - 0.7 / 1.5 / 1.5).abs() < 1e-6);
    }

    #[test]
    fn a_circling_dragon_at_the_end_of_its_path_sometimes_turns_to_land() {
        let sees = |_: DVec3, _: DVec3| true;
        let env = podium_env(&[], &sees, Some(0));
        let mut dragon = dragon_at(DVec3::new(0.5, 100.0, 0.5), 0.0);
        let seen = phases_of(&mut dragon, &env, 3000);
        assert_eq!(
            seen.get(1).map(|(_, phase)| *phase),
            Some(Phase::LandingApproach),
            "{seen:?}"
        );
    }

    #[test]
    fn a_landing_dragon_flies_in_perches_on_the_podium_and_takes_off_with_nobody_near() {
        let sees = |_: DVec3, _: DVec3| true;
        let env = podium_env(&[], &sees, Some(0));
        let mut dragon = dragon_at(DVec3::new(0.5, 110.0, 60.5), 0.0);
        dragon.set_phase(Phase::LandingApproach);
        let mut sat_at = None;
        let mut seen = vec![(0, dragon.phase)];
        for tick in 1..=4000 {
            dragon.tick(&env);
            if seen.last().map(|(_, phase)| *phase) != Some(dragon.phase) {
                seen.push((tick, dragon.phase));
                if dragon.phase == Phase::SittingScanning {
                    sat_at = Some(dragon.position);
                }
            }
            if dragon.phase == Phase::HoldingPattern {
                break;
            }
        }
        let order: Vec<Phase> = seen.iter().map(|(_, phase)| *phase).collect();
        assert_eq!(
            order,
            [
                Phase::LandingApproach,
                Phase::Landing,
                Phase::SittingScanning,
                Phase::Takeoff,
                Phase::HoldingPattern
            ],
            "{seen:?}"
        );
        let sat = sat_at.expect("it sat down");
        assert!(
            sat.distance_squared(DVec3::new(0.5, 62.0, 0.5)) < 1.0,
            "on the podium: {sat:?}"
        );
        assert_eq!(seen[3].0 - seen[2].0, SCAN_IDLE_TICKS as usize - 1);
        assert!(
            distance_to_center_sqr((0, 62, 0), dragon.position) >= 100.0,
            "it leaves the podium before circling"
        );
    }

    #[test]
    fn a_landing_dragon_breathes_every_tick_from_its_head() {
        let sees = |_: DVec3, _: DVec3| true;
        let env = podium_env(&[], &sees, Some(0));
        let mut dragon = dragon_at(DVec3::new(0.5, 110.0, 60.5), 0.0);
        dragon.set_phase(Phase::LandingApproach);
        let mut landing_ticks = 0;
        for _ in 1..=4000 {
            let head = dragon.parts[HEAD];
            let events = dragon.tick(&env);
            if dragon.phase == Phase::Landing {
                landing_ticks += 1;
                assert_eq!(events.breath.len(), 1);
                let breath = events.breath[0];
                assert_eq!(breath.kind, BreathKind::Landing);
                assert_eq!(breath.head, head + DVec3::new(0.0, 0.5, 0.0));
                assert!((breath.look.length() - 1.0).abs() < 1e-9);
                assert!(breath.look.y > 0.0, "the landing head looks up, so its breath falls");
            } else {
                assert!(events.breath.is_empty(), "{:?}", dragon.phase);
            }
            if dragon.phase == Phase::SittingScanning {
                break;
            }
        }
        assert!(landing_ticks > 5, "{landing_ticks}");
    }

    #[test]
    fn a_flaming_dragon_breathes_on_its_second_to_eighth_flame_tick() {
        let sees = |_: DVec3, _: DVec3| true;
        let players = [player(4, DVec3::new(8.0, 63.0, 0.5), true)];
        let env = podium_env(&players, &sees, Some(0));
        let mut dragon = perched();
        let mut flame_started = None;
        let mut puffs: Vec<Vec<usize>> = Vec::new();
        for tick in 1..=1400 {
            let head = dragon.parts[HEAD];
            let before = dragon.phase;
            let events = dragon.tick(&env);
            if dragon.phase == Phase::SittingFlaming && before != Phase::SittingFlaming {
                flame_started = Some(tick);
                puffs.push(Vec::new());
            }
            for breath in &events.breath {
                assert_eq!(breath.kind, BreathKind::Flame);
                assert_eq!(breath.motion, DVec3::ZERO);
                assert_eq!(breath.head, head + DVec3::new(0.0, 0.5, 0.0));
                assert!((breath.look - view_vector(-45.0, dragon.y_rot).normalize()).length() < 1e-9);
                puffs.last_mut().unwrap().push(tick - flame_started.unwrap());
            }
            if dragon.phase == Phase::Takeoff {
                break;
            }
        }
        assert_eq!(puffs.len(), 4);
        for flame in puffs {
            assert_eq!(flame, [1, 3, 5, 7]);
        }
    }

    fn perched() -> EnderDragon {
        let mut dragon = dragon_at(DVec3::new(0.5, 62.0, 0.5), -90.0);
        dragon.set_phase(Phase::SittingScanning);
        dragon
    }

    #[test]
    fn a_perched_dragon_turns_to_a_player_roars_and_breathes_flame_four_times() {
        let sees = |_: DVec3, _: DVec3| true;
        let players = [player(4, DVec3::new(8.0, 63.0, 0.5), true)];
        let env = podium_env(&players, &sees, Some(0));
        let mut dragon = perched();
        let before = dragon.y_rot;
        let mut clouds = Vec::new();
        let mut seen = vec![(0, dragon.phase)];
        let mut flames_out = 0;
        for tick in 1..=1400 {
            let events = dragon.tick(&env);
            clouds.extend(events.clouds.into_iter().map(|cloud| (tick, cloud)));
            if dragon.flame_out {
                dragon.flame_out = false;
                flames_out += 1;
            }
            if seen.last().map(|(_, phase)| *phase) != Some(dragon.phase) {
                seen.push((tick, dragon.phase));
            }
            if tick == 10 {
                assert_ne!(dragon.y_rot, before, "it turns toward the player");
            }
            if dragon.phase == Phase::Takeoff {
                break;
            }
        }
        let order: Vec<Phase> = seen.iter().map(|(_, phase)| *phase).collect();
        let mut expected = vec![Phase::SittingScanning];
        for _ in 0..3 {
            expected.extend([
                Phase::SittingAttacking,
                Phase::SittingFlaming,
                Phase::SittingScanning,
            ]);
        }
        expected.extend([
            Phase::SittingAttacking,
            Phase::SittingFlaming,
            Phase::Takeoff,
        ]);
        assert_eq!(order, expected, "{seen:?}");
        assert_eq!(seen[1].0, 26, "it roars after 25 ticks of watching");
        assert_eq!(seen[2].0 - seen[1].0, ROAR_TICKS as usize, "the roar");
        assert_eq!(seen[3].0 - seen[2].0, FLAME_TICKS as usize - 1);
        assert_eq!(clouds.len(), 4, "one flame each time");
        assert_eq!(flames_out, 4, "each flame goes out when it stops");
        let (tick, flame) = &clouds[0];
        assert_eq!(*tick, seen[2].0 + 9, "the flame comes on its tenth tick");
        assert_eq!(flame.breath, Some(Breath::Flame(dragon.id)));
        assert_eq!(
            (flame.radius, flame.duration),
            (FLAME_RADIUS, FLAME_DURATION)
        );
        assert_eq!(flame.radius_per_tick, 0.0);
        assert_eq!(flame.position.y, 62.0, "on the ground in front of the head");
        assert_eq!(
            crate::potions::instant_amount(
                &flame.effects[0],
                false,
                crate::effect_cloud::INSTANT_SCALE
            ),
            Some(crate::potions::PlayerPotion::Harm {
                amount: 3.0,
                by: None
            }),
            "Harming I at half strength"
        );
    }

    #[test]
    fn a_perched_dragon_with_nobody_near_charges_a_player_it_sees_within_150() {
        let sees = |_: DVec3, _: DVec3| true;
        let blind = |_: DVec3, _: DVec3| false;
        let far = [player(4, DVec3::new(60.0, 70.0, 0.5), true)];
        let mut dragon = perched();
        let seen = phases_of(&mut dragon, &podium_env(&far, &sees, Some(0)), 100);
        assert_eq!(
            seen,
            [(0, Phase::SittingScanning), (100, Phase::ChargingPlayer)]
        );
        assert_eq!(dragon.charge.target, Some(far[0].feet));
        let mut unseen = perched();
        let seen = phases_of(&mut unseen, &podium_env(&far, &blind, Some(0)), 100);
        assert_eq!(seen, [(0, Phase::SittingScanning), (100, Phase::Takeoff)]);
        let crouched = [PlayerBody {
            crouching: true,
            ..player(4, DVec3::new(17.0, 63.0, 0.5), true)
        }];
        assert_eq!(crouched[0].visibility(), 0.8);
        let mut sneaked = perched();
        let seen = phases_of(&mut sneaked, &podium_env(&crouched, &sees, Some(0)), 30);
        assert_eq!(
            seen,
            [(0, Phase::SittingScanning)],
            "a crouching player 17 blocks off is beyond 20 times 0.8"
        );
    }

    #[test]
    fn a_charging_dragon_turns_back_ten_ticks_after_it_reaches_its_target() {
        let sees = |_: DVec3, _: DVec3| true;
        let env = podium_env(&[], &sees, Some(0));
        let mut dragon = dragon_at(DVec3::new(0.5, 80.0, 0.5), 0.0);
        dragon.set_phase(Phase::ChargingPlayer);
        let target = DVec3::new(0.5, 70.0, -50.0);
        dragon.charge.target = Some(target);
        let mut reached = None;
        for tick in 1..=400 {
            dragon.tick(&env);
            if reached.is_none() && dragon.charge.time_since_charge > 0 {
                reached = Some(tick);
                assert!(dragon.position.distance_squared(target) < 100.0);
            }
            if dragon.phase == Phase::HoldingPattern {
                let at = reached.expect("it reached its target first");
                assert!((at + 5..=at + 10).contains(&tick), "{at} {tick}");
                break;
            }
        }
        assert_eq!(dragon.phase, Phase::HoldingPattern);
        let mut lost = dragon_at(DVec3::new(0.5, 80.0, 0.5), 0.0);
        lost.set_phase(Phase::ChargingPlayer);
        lost.tick(&env);
        assert_eq!(lost.phase, Phase::HoldingPattern, "no target, no charge");
    }

    #[test]
    fn hurting_a_flaming_dragon_past_50_puts_its_flame_out_and_it_takes_off() {
        let sees = |_: DVec3, _: DVec3| true;
        let players = [player(4, DVec3::new(8.0, 63.0, 0.5), true)];
        let env = podium_env(&players, &sees, Some(0));
        let mut dragon = perched();
        dragon.set_phase(Phase::SittingFlaming);
        for _ in 0..12 {
            dragon.tick(&env);
        }
        assert!(dragon.flame.cloud);
        let mut events = DragonEvents::default();
        dragon.hurt_part(HEAD, &EndDamage::attack(30.0, 4), &mut events);
        dragon.damage_cooldown = 0;
        dragon.hurt_part(HEAD, &EndDamage::attack(30.0, 4), &mut events);
        assert_eq!(dragon.phase, Phase::Takeoff);
        assert!(dragon.flame_out && !dragon.flame.cloud);
        let look = dragon.head_look_vector(&env);
        let far =
            (distance_to_center_sqr((0, 62, 0), dragon.position).sqrt() as f32 / 4.0).max(1.0);
        assert_eq!(look, view_vector(-(6.0 / far) * 7.5, dragon.y_rot));
        dragon.set_phase(Phase::SittingScanning);
        assert_eq!(
            dragon.head_look_vector(&env),
            view_vector(-45.0, dragon.y_rot),
            "a sitting head looks down"
        );
    }

    #[test]
    fn in_peaceful_the_dragon_strafes_watches_and_charges_nobody() {
        let height = flat;
        let top = || 64;
        let sees = |_: DVec3, _: DVec3| true;
        let players = [player(4, DVec3::new(40.0, 63.0, 0.0), true)];
        let calm = DragonEnv {
            peaceful: true,
            ..with_players(env(&height, &top, Some(10)), &players, &sees)
        };
        assert!(calm.targets().is_empty());
        for seed in 0..5 {
            let mut dragon = dragon_at(DVec3::new(0.5, 100.0, 0.5), seed as f32 * 18.0);
            dragon.random = LegacyRandom::new(seed);
            for _ in 0..2000 {
                dragon.tick(&calm);
                assert_ne!(dragon.phase, Phase::StrafePlayer, "nobody to strafe");
            }
        }
        let mut dragon = dragon_at(DVec3::new(0.0, 100.0, 0.0), 0.0);
        let mut events = DragonEvents::default();
        dragon.crystal_destroyed(9, Some(4), &mut events);
        dragon.tick(&calm);
        assert_eq!(
            dragon.phase,
            Phase::HoldingPattern,
            "a lost crystal turns it on nobody"
        );
        let near = [player(4, DVec3::new(8.0, 63.0, 0.5), true)];
        let mut watching = perched();
        let seen = phases_of(&mut watching, &podium_env(&near, &sees, Some(0)), 30);
        assert_eq!(seen[1], (26, Phase::SittingAttacking));
        let podium = DragonEnv {
            peaceful: true,
            ..podium_env(&near, &sees, Some(0))
        };
        let mut calm_perch = perched();
        let seen = phases_of(&mut calm_perch, &podium, 100);
        assert_eq!(
            seen,
            [(0, Phase::SittingScanning), (100, Phase::Takeoff)],
            "no roar at the player beside it and no charge"
        );
    }
}
