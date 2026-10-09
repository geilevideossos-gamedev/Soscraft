use glam::{DVec3, Vec2, Vec3};
use minecraftoss_entities::health::DamageResult;
use minecraftoss_entities::monster_ai::Target;
use minecraftoss_entities::movement::Body;
use minecraftoss_entities::potion::{Potion, PotionTarget, ThrownPotion};
use minecraftoss_entities::tempt::PlayerCandidate;
use minecraftoss_entities::world::{DamageSourceKind, EntityWorld};
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Face, Pos, World};

use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;

pub const GRAVITY: f64 = 0.03;
pub const POWER: f32 = 1.5;
pub const INACCURACY: f32 = 1.0;
pub const PEARL_DAMAGE: f32 = 5.0;
pub const PEARL_COOLDOWN: u32 = 20;
const KNOCKBACK: f64 = 0.4;
const THROWN: &str = "minecraft:thrown";
pub const PLAYER_TARGET: u64 = u64::MAX / 2;
const PLAYER_HALF_WIDTH: f64 = 0.3;
const PLAYER_HEIGHT: f64 = 1.8;
const HIDDEN_TICKS: i32 = 2;
const HIDDEN_NEAR: f64 = 12.25;
const CHICK_NUDGE: f64 = 0.25;
pub const BOTTLE_COLOR: u32 = 0x385DC6;
pub const BREAK_SOUND: &str = "minecraft:entity.splash_potion.break";
const FACE_EPSILON: f64 = 1.0e-5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Throwable {
    Snowball,
    Egg,
    EnderPearl,
    ExperienceBottle,
    SplashPotion,
    LingeringPotion,
}

impl Throwable {
    pub fn of(id: &str) -> Option<Self> {
        match id {
            "minecraft:snowball" => Some(Self::Snowball),
            "minecraft:egg" | "minecraft:blue_egg" | "minecraft:brown_egg" => Some(Self::Egg),
            "minecraft:ender_pearl" => Some(Self::EnderPearl),
            "minecraft:experience_bottle" => Some(Self::ExperienceBottle),
            "minecraft:splash_potion" => Some(Self::SplashPotion),
            "minecraft:lingering_potion" => Some(Self::LingeringPotion),
            _ => None,
        }
    }

    pub fn sound(self) -> &'static str {
        match self {
            Self::Snowball => "minecraft:entity.snowball.throw",
            Self::Egg => "minecraft:entity.egg.throw",
            Self::EnderPearl => "minecraft:entity.ender_pearl.throw",
            Self::ExperienceBottle => "minecraft:entity.experience_bottle.throw",
            Self::SplashPotion => "minecraft:entity.splash_potion.throw",
            Self::LingeringPotion => "minecraft:entity.lingering_potion.throw",
        }
    }

    pub fn pitch_offset(self) -> f32 {
        match self {
            Self::Snowball | Self::Egg | Self::EnderPearl => 0.0,
            Self::ExperienceBottle | Self::SplashPotion | Self::LingeringPotion => -20.0,
        }
    }

    pub fn power(self) -> f32 {
        match self {
            Self::Snowball | Self::Egg | Self::EnderPearl => POWER,
            Self::ExperienceBottle => 0.7,
            Self::SplashPotion | Self::LingeringPotion => 0.5,
        }
    }

    pub fn inaccuracy(self) -> f32 {
        INACCURACY
    }

    pub fn gravity(self) -> f64 {
        match self {
            Self::Snowball | Self::Egg | Self::EnderPearl => GRAVITY,
            Self::ExperienceBottle => 0.07,
            Self::SplashPotion | Self::LingeringPotion => minecraftoss_entities::potion::GRAVITY,
        }
    }
}

pub fn bottle_experience(random: &mut LegacyRandom) -> i32 {
    3 + random.next_int(5) as i32 + random.next_int(5) as i32
}

pub fn break_pitch(random: f32) -> f32 {
    random * 0.1 + 0.9
}

pub fn sound_cell(at: DVec3) -> [f64; 3] {
    (at.floor() + DVec3::splat(0.5)).to_array()
}

pub fn douse_cells(block: Pos, face: Face) -> [Pos; 6] {
    let (dx, dy, dz) = face.offset();
    let (x, y, z) = (block.0 + dx, block.1 + dy, block.2 + dz);
    [(x, y, z), block, (x, y, z - 1), (x + 1, y, z), (x, y, z + 1), (x - 1, y, z)]
}

pub fn face_normal(face: Face) -> DVec3 {
    let (x, y, z) = face.offset();
    DVec3::new(f64::from(x), f64::from(y), f64::from(z))
}

fn nearest_face(v: DVec3) -> Face {
    let mut best = (Face::Down, f64::MIN);
    for face in [Face::Down, Face::Up, Face::North, Face::South, Face::West, Face::East] {
        let dot = face_normal(face).dot(v);
        if dot > best.1 {
            best = (face, dot);
        }
    }
    best.0
}

pub fn struck_face(world: &impl World, block: Pos, at: DVec3, motion: DVec3) -> Face {
    let mut best: Option<(f64, Face)> = None;
    for (min, max) in minecraftoss_player::collision_boxes_at(world, block) {
        for axis in 0..3 {
            let (plane, face) = if motion[axis] > 0.0 {
                (min[axis], [Face::West, Face::Down, Face::North][axis])
            } else if motion[axis] < 0.0 {
                (max[axis], [Face::East, Face::Up, Face::South][axis])
            } else {
                continue;
            };
            let within = (0..3).filter(|&o| o != axis).all(|o| at[o] >= min[o] - FACE_EPSILON && at[o] <= max[o] + FACE_EPSILON);
            let gap = (at[axis] - plane).abs();
            if within && gap < FACE_EPSILON && best.is_none_or(|(g, _)| gap < g) {
                best = Some((gap, face));
            }
        }
    }
    best.map_or_else(|| nearest_face(-motion), |(_, face)| face)
}

pub fn throw_pitch(random: f32) -> f32 {
    0.4 / (random * 0.4 + 0.8)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerBody {
    pub id: u64,
    pub feet: DVec3,
    pub alive: bool,
    pub spectator: bool,
}

impl PlayerBody {
    pub fn of(candidate: &PlayerCandidate) -> Self {
        Self { id: candidate.id, feet: candidate.position, alive: candidate.alive, spectator: candidate.spectator }
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        let half = DVec3::new(PLAYER_HALF_WIDTH, 0.0, PLAYER_HALF_WIDTH);
        (self.feet - half, self.feet + half + DVec3::Y * PLAYER_HEIGHT)
    }

    pub fn targetable(&self) -> bool {
        self.alive && !self.spectator
    }
}

pub fn target_feet(target: &PotionTarget) -> DVec3 {
    DVec3::new((target.min.x + target.max.x) / 2.0, target.min.y, (target.min.z + target.max.z) / 2.0)
}

pub fn targets_by_id(targets: &[PotionTarget], id: u64) -> Option<DVec3> {
    targets.iter().find(|t| t.id == id).map(target_feet)
}

pub fn struck_player(id: u64) -> Option<u64> {
    id.checked_sub(PLAYER_TARGET)
}

pub fn player_targets(players: &[PlayerBody], owner: u64) -> Vec<PotionTarget> {
    players
        .iter()
        .filter(|p| p.targetable())
        .map(|p| {
            let (min, max) = p.bounds();
            PotionTarget { id: PLAYER_TARGET + p.id, min, max, owner: p.id == owner }
        })
        .collect()
}

pub fn owner_box(players: &[PlayerBody], owner: u64) -> Option<(DVec3, DVec3)> {
    players.iter().find(|p| p.id == owner && p.targetable()).map(PlayerBody::bounds)
}

pub struct Throw<'a> {
    pub owner: u64,
    pub item: &'a str,
    pub components: Option<&'a serde_json::Value>,
    pub eye: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub motion: DVec3,
    pub on_ground: bool,
}

pub fn chick_variant(item: &str) -> &'static str {
    match item {
        "minecraft:blue_egg" => "minecraft:cold",
        "minecraft:brown_egg" => "minecraft:warm",
        _ => "minecraft:temperate",
    }
}

pub fn chicks(random: &mut LegacyRandom) -> usize {
    if random.next_int(8) != 0 {
        0
    } else if random.next_int(32) == 0 {
        4
    } else {
        1
    }
}

pub fn shot_direction(yaw: f32, pitch: f32) -> DVec3 {
    shot_direction_offset(yaw, pitch, 0.0)
}

pub fn shot_direction_offset(yaw: f32, pitch: f32, offset: f32) -> DVec3 {
    let (y, x, lifted) = (yaw.to_radians(), pitch.to_radians(), (pitch + offset).to_radians());
    DVec3::new(
        f64::from(-y.sin() * x.cos()),
        f64::from(-lifted.sin()),
        f64::from(y.cos() * x.cos()),
    )
}

#[derive(Clone, Debug)]
pub struct Thrown {
    pub item: String,
    pub kind: Throwable,
    pub owner: u64,
    pub components: Option<serde_json::Value>,
    pub previous: DVec3,
    flight: ThrownPotion,
    random: LegacyRandom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landed {
    pub at: DVec3,
    pub before: DVec3,
    pub mob: Option<u64>,
    pub velocity: DVec3,
    pub block: Option<Pos>,
    pub face: Option<Face>,
}

impl Thrown {
    pub fn throw(
        item: &str,
        eye: DVec3,
        yaw: f32,
        pitch: f32,
        motion: DVec3,
        on_ground: bool,
        random: &mut LegacyRandom,
    ) -> Option<Self> {
        Self::launch(&Throw { owner: 0, item, components: None, eye, yaw, pitch, motion, on_ground }, random)
    }

    pub fn launch(throw: &Throw, random: &mut LegacyRandom) -> Option<Self> {
        let kind = Throwable::of(throw.item)?;
        let start = throw.eye - DVec3::new(0.0, 0.1, 0.0);
        let direction = shot_direction_offset(throw.yaw, throw.pitch, kind.pitch_offset());
        let mut flight = ThrownPotion::shoot(
            Potion::Healing,
            PLAYER_TARGET + throw.owner,
            start,
            direction,
            kind.power(),
            kind.inaccuracy(),
            random,
        );
        let motion = throw.motion;
        flight.velocity += DVec3::new(motion.x, if throw.on_ground { 0.0 } else { motion.y }, motion.z);
        Some(Self {
            item: throw.item.to_owned(),
            kind,
            owner: throw.owner,
            components: throw.components.cloned(),
            previous: start,
            flight,
            random: LegacyRandom::new(random.next_long()),
        })
    }

    pub fn position(&self) -> DVec3 {
        self.flight.position
    }

    pub fn velocity(&self) -> DVec3 {
        self.flight.velocity
    }

    pub fn alive(&self) -> bool {
        self.flight.alive
    }

    pub fn yaw(&self) -> f32 {
        self.flight.yaw
    }

    pub fn age(&self) -> i32 {
        self.flight.tick_count
    }

    pub fn pitch(&self) -> f32 {
        self.flight.pitch
    }

    pub fn margin(&self) -> f64 {
        self.flight.margin()
    }

    pub fn contents(&self) -> crate::potions::PotionContents {
        crate::potions::PotionContents::from_components(self.components.as_ref())
    }

    pub fn tint(&self) -> Option<u32> {
        matches!(self.kind, Throwable::SplashPotion | Throwable::LingeringPotion).then(|| self.contents().color())
    }

    pub fn view(&self) -> ThrownView {
        ThrownView {
            item: self.item.clone(),
            owner: self.owner,
            previous: self.previous,
            position: self.position(),
            age: self.age(),
            yaw: self.yaw(),
            pitch: self.pitch(),
            tint: self.tint(),
        }
    }

    pub fn random(&mut self) -> &mut LegacyRandom {
        &mut self.random
    }

    pub fn tick(
        &mut self,
        world: &impl World,
        targets: &[PotionTarget],
        owner: Option<(DVec3, DVec3)>,
    ) -> Option<Landed> {
        self.previous = self.flight.position;
        self.flight.velocity.y += minecraftoss_entities::potion::GRAVITY - self.kind.gravity();
        let hit = self.flight.tick(world, targets, owner)?;
        let velocity = self.flight.velocity;
        Some(Landed {
            at: hit.location,
            before: self.previous,
            mob: hit.entity,
            velocity,
            block: hit.block,
            face: hit.block.map(|block| struck_face(world, block, hit.location, velocity)),
        })
    }
}

fn target(id: u64, body: &Body) -> PotionTarget {
    let half = f64::from(body.width) / 2.0;
    let p = body.position;
    PotionTarget {
        id,
        min: DVec3::new(p.x - half, p.y, p.z - half),
        max: DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
        owner: false,
    }
}

pub fn targets(world: &EntityWorld) -> Vec<PotionTarget> {
    let mut out = Vec::new();
    out.extend(
        world
            .bats()
            .iter()
            .filter(|e| e.bat.health > 0.0)
            .map(|e| target(e.id, &e.bat.body)),
    );
    out.extend(
        world
            .zombies()
            .iter()
            .filter(|e| e.zombie.health > 0.0)
            .map(|e| target(e.id, &e.zombie.body)),
    );
    out.extend(
        world
            .skeletons()
            .iter()
            .filter(|e| e.skeleton.health > 0.0)
            .map(|e| target(e.id, &e.skeleton.body)),
    );
    out.extend(
        world
            .creepers()
            .iter()
            .filter(|e| e.creeper.health > 0.0 && !e.creeper.exploded)
            .map(|e| target(e.id, &e.creeper.body)),
    );
    out.extend(
        world
            .spiders()
            .iter()
            .filter(|e| e.spider.health > 0.0)
            .map(|e| target(e.id, &e.spider.body)),
    );
    out.extend(
        world
            .slimes()
            .iter()
            .filter(|e| e.slime.health > 0.0)
            .map(|e| target(e.id, &e.slime.body)),
    );
    out.extend(
        world
            .endermen()
            .iter()
            .filter(|e| e.enderman.health > 0.0)
            .map(|e| target(e.id, &e.enderman.body)),
    );
    out.extend(
        world
            .witches()
            .iter()
            .filter(|e| e.witch.health > 0.0)
            .map(|e| target(e.id, &e.witch.body)),
    );
    out.extend(
        world
            .iron_golems()
            .iter()
            .filter(|e| e.golem.health > 0.0)
            .map(|e| target(e.id, &e.golem.body)),
    );
    out.extend(
        world
            .wolves()
            .iter()
            .filter(|e| e.wolf.health > 0.0)
            .map(|e| target(e.id, &e.wolf.body)),
    );
    out.extend(
        world
            .villagers()
            .iter()
            .filter(|e| e.villager.health > 0.0)
            .map(|e| target(e.id, &e.villager.body)),
    );
    out.extend(
        world
            .cows()
            .iter()
            .filter(|e| e.cow.health > 0.0)
            .map(|e| target(e.id, &e.cow.body)),
    );
    out.extend(
        world
            .sheep()
            .iter()
            .filter(|e| e.health > 0.0)
            .map(|e| target(e.id, &e.body)),
    );
    out.extend(
        world
            .pigs()
            .iter()
            .filter(|e| e.pig.health > 0.0)
            .map(|e| target(e.id, &e.pig.body)),
    );
    out.extend(
        world
            .chickens()
            .iter()
            .filter(|e| e.chicken.health > 0.0)
            .map(|e| target(e.id, &e.chicken.body)),
    );
    out
}

fn knockback(body: &mut Body, random: &mut LegacyRandom, power: f64, mut xd: f64, mut zd: f64) {
    while xd * xd + zd * zd < f64::from(1.0e-5_f32) {
        xd = (random.next_double() - random.next_double()) * 0.01;
        zd = (random.next_double() - random.next_double()) * 0.01;
    }
    let length = (xd * xd + zd * zd).sqrt();
    let (dx, dz) = (xd / length * power, zd / length * power);
    let v = body.velocity;
    let y = if body.on_ground {
        (v.y / 2.0 + power).min(0.4)
    } else {
        v.y
    };
    body.velocity = DVec3::new(v.x / 2.0 - dx, y, v.z / 2.0 - dz);
    body.needs_sync = true;
}

pub fn hurt(
    world: &mut EntityWorld,
    id: u64,
    velocity: DVec3,
    player: u64,
    random: &mut LegacyRandom,
) -> bool {
    if let Some(e) = world.enderman_mut(id) {
        e.dodge_pending = true;
        return false;
    }
    let by = Some(Target::Player(player));
    let time = world.game_time();
    let projectile = DamageSourceKind::Projectile;
    let blame =
        |hit: &DamageResult, tick: i32| hit.applied.then_some((Target::Player(player), tick));
    let (hit, resists) = if let Some(e) = world.bat_mut(id) {
        (e.hurt(0.0), false)
    } else if let Some(e) = world.zombie_mut(id) {
        let hit = e.hurt(0.0);
        if let (Some(ai), Some(blamed)) = (e.ai.as_deref_mut(), blame(&hit, e.tick_count)) {
            ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.skeleton_mut(id) {
        let hit = e.hurt(0.0);
        if let (Some(ai), Some(blamed)) = (e.ai.as_deref_mut(), blame(&hit, e.tick_count)) {
            ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.creeper_mut(id) {
        let hit = e.hurt(0.0);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.spider_mut(id) {
        let hit = e.hurt(0.0);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.slime_mut(id) {
        let hit = e.hurt(0.0);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.witch_mut(id) {
        let hit = e.hurt(0.0, false, false);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, false)
    } else if let Some(e) = world.iron_golem_mut(id) {
        let hit = e.hurt(0.0);
        if let Some(blamed) = blame(&hit, e.tick_count) {
            e.ai.state.hurt_by = Some(blamed);
        }
        (hit, true)
    } else if let Some(e) = world.wolf_mut(id) {
        (e.hurt_from(0.0, THROWN, by, time), false)
    } else if let Some(e) = world.villager_mut(id) {
        (
            e.hurt_from(0.0, THROWN, Some(PLAYER_TARGET + player), time),
            false,
        )
    } else if let Some(e) = world.cow_mut(id) {
        (e.hurt(0.0, projectile), false)
    } else if let Some(e) = world.sheep_mut(id) {
        (e.hurt(0.0, projectile), false)
    } else if let Some(e) = world.pig_mut(id) {
        (e.hurt(0.0, projectile), false)
    } else if let Some(e) = world.chicken_mut(id) {
        (e.hurt_with_source(0.0, projectile), false)
    } else {
        return false;
    };
    if hit.applied {
        world.villager_hurt_by(id, PLAYER_TARGET + player);
    }
    if hit.applied && hit.full && !resists {
        if let Some(body) = world.body_mut(id) {
            knockback(body, random, KNOCKBACK, -velocity.x, -velocity.z);
        }
    }
    hit.applied
}

pub fn shown(age: i32, at: DVec3, camera: DVec3) -> bool {
    age >= HIDDEN_TICKS || at.distance_squared(camera) >= HIDDEN_NEAR
}

pub fn hatch_point(at: DVec3, before: DVec3) -> DVec3 {
    let back = before - at;
    if back.length_squared() < 1.0e-12 {
        at
    } else {
        at + back.normalize() * CHICK_NUDGE
    }
}

pub fn texture(item: &str) -> Option<ResourceId> {
    let path = item.strip_prefix("minecraft:")?;
    ResourceId::parse(&format!("minecraft:item/{path}")).ok()
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThrownView {
    pub item: String,
    pub owner: u64,
    pub previous: DVec3,
    pub position: DVec3,
    pub age: i32,
    pub yaw: f32,
    pub pitch: f32,
    pub tint: Option<u32>,
}

impl ThrownView {
    pub fn at(&self, partial: f32) -> DVec3 {
        self.previous.lerp(self.position, f64::from(partial.clamp(0.0, 1.0)))
    }
}

fn tint_color(rgb: u32) -> [f32; 4] {
    let [r, g, b] = crate::spell_particles::rgb(rgb);
    [r, g, b, 1.0]
}

fn layers(view: &ThrownView) -> Vec<(ResourceId, [f32; 4])> {
    match (view.tint, texture(&view.item)) {
        (Some(tint), Some(bottle)) => match ResourceId::parse("minecraft:item/potion_overlay") {
            Ok(overlay) => vec![(overlay, tint_color(tint)), (bottle, [1.0; 4])],
            Err(_) => Vec::new(),
        },
        (None, Some(item)) => vec![(item, [1.0; 4])],
        (_, None) => Vec::new(),
    }
}

pub fn append_views<'a>(mesh: &mut ChunkMesh, views: impl IntoIterator<Item = &'a ThrownView>, partial: f32, camera: DVec3, atlas: &Atlas, forward: Vec3, light: &SkyLight) {
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    for view in views {
        let at = view.at(partial);
        if !shown(view.age, at, camera) {
            continue;
        }
        let layers = layers(view);
        if layers.iter().any(|(id, _)| !atlas.contains(id)) {
            continue;
        }
        let cell = (at.x.floor() as i32, at.y.floor() as i32, at.z.floor() as i32);
        let (sky, block) = (light.get(cell) as f32, light.get_block(cell) as f32);
        let center = at.as_vec3() + up * 0.125;
        for (index, (id, color)) in layers.iter().enumerate() {
            let [u0, v0, u1, v1] = atlas.region(id);
            let nudge = forward * (index as f32 * 0.002);
            let start = mesh.vertices.len() as u32;
            for (corner, uv) in [(Vec2::new(-0.25, -0.25), [u0, v1]), (Vec2::new(0.25, -0.25), [u1, v1]), (Vec2::new(0.25, 0.25), [u1, v0]), (Vec2::new(-0.25, 0.25), [u0, v0])] {
                let point = center + right * corner.x + up * corner.y - nudge;
                mesh.vertices.push(Vertex { position: point.to_array(), uv, color: *color, sky_light: sky, block_light: block });
            }
            mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
}

pub fn append_sprites<'a>(
    mesh: &mut ChunkMesh,
    items: impl IntoIterator<Item = (&'a str, DVec3)>,
    atlas: &Atlas,
    forward: Vec3,
    light: &SkyLight,
) {
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    for (item, at) in items {
        let Some(id) = texture(item).filter(|id| atlas.contains(id)) else {
            continue;
        };
        let cell = (
            at.x.floor() as i32,
            at.y.floor() as i32,
            at.z.floor() as i32,
        );
        let (sky, block) = (light.get(cell) as f32, light.get_block(cell) as f32);
        let center = at.as_vec3() + up * 0.125;
        let [u0, v0, u1, v1] = atlas.region(&id);
        let start = mesh.vertices.len() as u32;
        for (corner, uv) in [
            (Vec2::new(-0.25, -0.25), [u0, v1]),
            (Vec2::new(0.25, -0.25), [u1, v1]),
            (Vec2::new(0.25, 0.25), [u1, v0]),
            (Vec2::new(-0.25, 0.25), [u0, v0]),
        ] {
            let point = center + right * corner.x + up * corner.y;
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                uv,
                color: [1.0; 4],
                sky_light: sky,
                block_light: block,
            });
        }
        mesh.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};
    use std::collections::HashMap;

    #[derive(Default)]
    struct Blocks(HashMap<Pos, Block>);

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.0.get(&pos).cloned()
        }

        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.0.insert(pos, block),
                None => self.0.remove(&pos),
            };
        }
    }

    fn throw(item: &str, pitch: f32) -> Thrown {
        let mut random = LegacyRandom::new(42);
        Thrown::throw(
            item,
            DVec3::new(0.5, 70.0, 0.5),
            0.0,
            pitch,
            DVec3::ZERO,
            true,
            &mut random,
        )
        .expect("a throwable")
    }

    #[test]
    fn each_throwable_item_names_its_kind() {
        assert_eq!(
            Throwable::of("minecraft:experience_bottle"),
            Some(Throwable::ExperienceBottle)
        );
        assert_eq!(
            Throwable::of("minecraft:snowball"),
            Some(Throwable::Snowball)
        );
        assert_eq!(Throwable::of("minecraft:brown_egg"), Some(Throwable::Egg));
        assert_eq!(
            Throwable::of("minecraft:ender_pearl"),
            Some(Throwable::EnderPearl)
        );
        assert_eq!(Throwable::of("minecraft:stone"), None);
        assert_eq!(chick_variant("minecraft:blue_egg"), "minecraft:cold");
        assert_eq!(chick_variant("minecraft:egg"), "minecraft:temperate");
    }

    #[test]
    fn a_throw_leaves_from_under_the_eye_at_one_and_a_half_blocks_a_tick() {
        let thrown = throw("minecraft:snowball", 0.0);
        assert!((thrown.position().y - 69.9).abs() < 1e-9);
        let speed = thrown.velocity().length();
        assert!((speed - 1.5).abs() < 0.05, "{speed}");
        assert!(thrown.velocity().z > 1.4);
    }

    #[test]
    fn a_throw_carries_the_throwers_motion_but_not_its_fall_on_the_ground() {
        let mut random = LegacyRandom::new(1);
        let still = Thrown::throw(
            "minecraft:egg",
            DVec3::ZERO,
            0.0,
            0.0,
            DVec3::ZERO,
            false,
            &mut random.clone(),
        )
        .unwrap();
        let moving = Thrown::throw(
            "minecraft:egg",
            DVec3::ZERO,
            0.0,
            0.0,
            DVec3::new(0.2, -0.5, 0.0),
            true,
            &mut random,
        )
        .unwrap();
        assert!((moving.velocity().x - still.velocity().x - 0.2).abs() < 1e-9);
        assert!((moving.velocity().y - still.velocity().y).abs() < 1e-9);
    }

    #[test]
    fn thrown_items_fall_under_vanilla_gravity_and_drag() {
        let world = Blocks::default();
        let mut thrown = throw("minecraft:snowball", 0.0);
        let before = thrown.velocity();
        assert!(thrown.tick(&world, &[], None).is_none());
        let expected = (before.y - GRAVITY) * f64::from(0.99_f32);
        assert!((thrown.velocity().y - expected).abs() < 1e-9);
        assert!((thrown.velocity().z - before.z * f64::from(0.99_f32)).abs() < 1e-9);
    }

    #[test]
    fn a_pearl_lands_where_it_was_before_the_tick_it_struck() {
        let mut world = Blocks::default();
        for x in -2..=2 {
            for z in -2..=40 {
                world.set_block((x, 60, z), Some(Block::new("minecraft:stone")));
            }
        }
        let mut thrown = throw("minecraft:ender_pearl", 0.0);
        let mut landed = None;
        for _ in 0..200 {
            if let Some(hit) = thrown.tick(&world, &[], None) {
                landed = Some(hit);
                break;
            }
        }
        let landed = landed.expect("the pearl lands");
        assert!((landed.at.y - 61.0).abs() < 1e-6, "{landed:?}");
        assert!(landed.before.y > landed.at.y);
        assert!(!thrown.alive());
    }

    #[test]
    fn a_thrown_item_strikes_a_mob_in_its_way() {
        let world = Blocks::default();
        let mut thrown = throw("minecraft:snowball", 0.0);
        let cow = PotionTarget {
            id: 7,
            min: DVec3::new(0.0, 69.0, 3.0),
            max: DVec3::new(1.0, 71.0, 4.0),
            owner: false,
        };
        let mut landed = None;
        for _ in 0..10 {
            if let Some(hit) = thrown.tick(&world, &[cow], None) {
                landed = Some(hit);
                break;
            }
        }
        assert_eq!(landed.and_then(|l| l.mob), Some(7));
    }

    #[test]
    fn about_one_egg_in_eight_hatches_and_one_hatch_in_thirty_two_is_four_chicks() {
        let mut random = LegacyRandom::new(9);
        let rolls: Vec<usize> = (0..256_000).map(|_| chicks(&mut random)).collect();
        let hatched = rolls.iter().filter(|&&n| n > 0).count() as f64 / rolls.len() as f64;
        let fours = rolls.iter().filter(|&&n| n == 4).count() as f64 / rolls.len() as f64;
        assert!((hatched - 1.0 / 8.0).abs() < 0.005, "{hatched}");
        assert!((fours - 1.0 / 256.0).abs() < 0.001, "{fours}");
        assert!(rolls.iter().all(|&n| matches!(n, 0 | 1 | 4)));
    }

    #[test]
    fn a_fresh_throw_is_not_drawn_in_the_throwers_face() {
        let camera = DVec3::new(0.0, 70.0, 0.0);
        assert!(!shown(1, DVec3::new(0.0, 70.0, 0.5), camera));
        assert!(shown(2, DVec3::new(0.0, 70.0, 0.5), camera));
        assert!(shown(1, DVec3::new(0.0, 70.0, 4.0), camera));
    }

    #[test]
    fn chicks_hatch_off_the_face_the_egg_struck() {
        let at = DVec3::new(5.0, 64.5, 2.0);
        let hatched = hatch_point(at, DVec3::new(4.0, 64.5, 2.0));
        assert!((hatched.x - 4.75).abs() < 1e-9);
        assert_eq!(hatch_point(at, at), at);
    }

    #[test]
    fn the_throw_sound_is_pitched_down_at_random() {
        assert!((throw_pitch(0.0) - 0.5).abs() < 1e-6);
        assert!((throw_pitch(1.0) - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn snowballs_eggs_and_pearls_leave_level_at_one_and_a_half_under_light_gravity() {
        for kind in [Throwable::Snowball, Throwable::Egg, Throwable::EnderPearl] {
            assert_eq!((kind.pitch_offset(), kind.power(), kind.inaccuracy(), kind.gravity()), (0.0, 1.5, 1.0, 0.03), "{kind:?}");
        }
    }

    #[test]
    fn a_pitch_offset_lifts_only_the_rise_of_the_throw() {
        let level = shot_direction_offset(0.0, 0.0, -20.0);
        assert!((level.y - 20.0_f32.to_radians().sin() as f64).abs() < 1e-6);
        assert!((level.z - 1.0).abs() < 1e-6);
        let flat = shot_direction(90.0, 0.0);
        assert!((flat.x + 1.0).abs() < 1e-6 && flat.y.abs() < 1e-6);
    }

    #[test]
    fn a_throw_remembers_its_thrower() {
        let mut random = LegacyRandom::new(3);
        let throw = Throw { owner: 9, item: "minecraft:snowball", components: None, eye: DVec3::ZERO, yaw: 0.0, pitch: 0.0, motion: DVec3::ZERO, on_ground: true };
        assert_eq!(Thrown::launch(&throw, &mut random).map(|t| t.owner), Some(9));
        assert_eq!(throw_owner("minecraft:egg"), 0);
    }

    fn throw_owner(item: &str) -> u64 {
        throw(item, 0.0).owner
    }

    #[test]
    fn players_are_struck_by_their_id_and_the_thrower_is_marked() {
        let players = [
            PlayerBody { id: 0, feet: DVec3::ZERO, alive: true, spectator: false },
            PlayerBody { id: 1, feet: DVec3::X, alive: true, spectator: false },
            PlayerBody { id: 2, feet: DVec3::Z, alive: true, spectator: true },
            PlayerBody { id: 3, feet: DVec3::Y, alive: false, spectator: false },
        ];
        let targets = player_targets(&players, 1);
        assert_eq!(targets.iter().map(|t| (struck_player(t.id), t.owner)).collect::<Vec<_>>(), vec![(Some(0), false), (Some(1), true)]);
        assert_eq!(targets[1].min, DVec3::new(0.7, 0.0, -0.3));
        assert_eq!(targets[1].max, DVec3::new(1.3, 1.8, 0.3));
        assert_eq!(owner_box(&players, 2), None);
        assert_eq!(struck_player(7), None);
    }

    #[test]
    fn a_throw_straight_up_spares_its_thrower_until_it_falls_back_on_it() {
        let world = Blocks::default();
        let mut random = LegacyRandom::new(5);
        let throw = Throw { owner: 4, item: "minecraft:snowball", components: None, eye: DVec3::new(0.0, 70.0, 0.0), yaw: 0.0, pitch: -90.0, motion: DVec3::ZERO, on_ground: true };
        let mut thrown = Thrown::launch(&throw, &mut random).unwrap();
        let thrower = PotionTarget { id: PLAYER_TARGET + 4, min: DVec3::new(-5.0, 68.38, -5.0), max: DVec3::new(5.0, 70.18, 5.0), owner: true };
        let mut landed = None;
        for _ in 0..300 {
            if let Some(hit) = thrown.tick(&world, &[thrower], Some((thrower.min, thrower.max))) {
                landed = Some(hit);
                break;
            }
        }
        let landed = landed.expect("it comes back down");
        assert_eq!(landed.mob.and_then(struck_player), Some(4));
        assert!(thrown.age() > 50, "{}", thrown.age());
    }

    #[test]
    fn a_bottle_o_enchanting_leaves_20_degrees_up_at_0_7_and_falls_faster() {
        let kind = Throwable::ExperienceBottle;
        assert_eq!((kind.pitch_offset(), kind.power(), kind.gravity()), (-20.0, 0.7, 0.07));
        let bottle = throw("minecraft:experience_bottle", 0.0);
        let v = bottle.velocity();
        assert!((v.length() - 0.7).abs() < 0.03, "{v:?}");
        assert!((v.y / v.z - 20.0_f64.to_radians().sin()).abs() < 0.05, "{v:?}");
        let world = Blocks::default();
        let mut bottle = bottle;
        let before = bottle.velocity();
        bottle.tick(&world, &[], None);
        assert!((bottle.velocity().y - (before.y - 0.07) * f64::from(0.99_f32)).abs() < 1e-9);
    }

    #[test]
    fn a_splash_potion_leaves_20_degrees_up_at_half_speed_and_shows_its_colour() {
        let kind = Throwable::SplashPotion;
        assert_eq!(Throwable::of("minecraft:splash_potion"), Some(kind));
        assert_eq!((kind.pitch_offset(), kind.power(), kind.gravity()), (-20.0, 0.5, 0.05));
        assert_eq!(kind.sound(), "minecraft:entity.splash_potion.throw");
        let mut random = LegacyRandom::new(8);
        let components = serde_json::json!({"minecraft:potion_contents": {"potion": "minecraft:poison"}});
        let poison = Throw { owner: 2, item: "minecraft:splash_potion", components: Some(&components), eye: DVec3::new(0.0, 70.0, 0.0), yaw: 0.0, pitch: 0.0, motion: DVec3::ZERO, on_ground: true };
        let potion = Thrown::launch(&poison, &mut random).unwrap();
        assert!((potion.velocity().length() - 0.5).abs() < 0.03);
        let view = potion.view();
        assert_eq!((view.tint, view.owner, view.item.as_str()), (Some(8889187), 2, "minecraft:splash_potion"));
        let layered = layers(&view);
        assert_eq!(layered.iter().map(|(id, _)| id.key()).collect::<Vec<_>>(), vec!["minecraft:item/potion_overlay".to_owned(), "minecraft:item/splash_potion".to_owned()]);
        assert_eq!(layered[1].1, [1.0; 4]);
        assert_eq!(throw_owner("minecraft:snowball"), 0);
        assert_eq!(throw("minecraft:snowball", 0.0).view().tint, None);
        let plain = throw("minecraft:splash_potion", 0.0);
        assert_eq!(plain.view().tint, Some(BOTTLE_COLOR));
    }

    #[test]
    fn a_lingering_potion_flies_like_a_splash_potion() {
        let kind = Throwable::LingeringPotion;
        assert_eq!(Throwable::of("minecraft:lingering_potion"), Some(kind));
        assert_eq!((kind.pitch_offset(), kind.power(), kind.gravity()), (-20.0, 0.5, 0.05));
        assert_eq!(kind.sound(), "minecraft:entity.lingering_potion.throw");
        assert_eq!(throw("minecraft:lingering_potion", 0.0).view().tint, Some(BOTTLE_COLOR));
    }

    #[test]
    fn bottle_experience_is_3_to_11() {
        let mut random = LegacyRandom::new(17);
        let rolls: Vec<i32> = (0..20_000).map(|_| bottle_experience(&mut random)).collect();
        assert_eq!(rolls.iter().min(), Some(&3));
        assert_eq!(rolls.iter().max(), Some(&11));
        let mean = rolls.iter().sum::<i32>() as f64 / rolls.len() as f64;
        assert!((mean - 7.0).abs() < 0.1, "{mean}");
        assert!((break_pitch(0.0) - 0.9).abs() < 1e-6 && (break_pitch(1.0) - 1.0).abs() < 1e-6);
        assert_eq!(sound_cell(DVec3::new(-0.2, 64.9, 3.0)), [-0.5, 64.5, 3.5]);
    }

    #[test]
    fn water_douses_the_face_cell_the_struck_block_and_the_four_sides() {
        assert_eq!(douse_cells((0, 64, 0), Face::Up), [(0, 65, 0), (0, 64, 0), (0, 65, -1), (1, 65, 0), (0, 65, 1), (-1, 65, 0)]);
        assert_eq!(douse_cells((3, 64, 0), Face::West)[..2], [(2, 64, 0), (3, 64, 0)]);
    }

    #[test]
    fn a_throw_lands_on_the_face_it_met() {
        let mut world = Blocks::default();
        world.set_block((0, 69, 5), Some(Block::new("minecraft:stone")));
        for x in -2..=2 {
            for z in -2..=40 {
                world.set_block((x, 50, z), Some(Block::new("minecraft:stone")));
            }
        }
        let mut level = throw("minecraft:snowball", 0.0);
        let mut wall = None;
        for _ in 0..20 {
            if let Some(hit) = level.tick(&world, &[], None) {
                wall = Some(hit);
                break;
            }
        }
        let wall = wall.expect("it meets the wall");
        assert_eq!((wall.block, wall.face), (Some((0, 69, 5)), Some(Face::North)));
        let mut down = throw("minecraft:snowball", 90.0);
        let mut floor = None;
        for _ in 0..40 {
            if let Some(hit) = down.tick(&world, &[], None) {
                floor = Some(hit);
                break;
            }
        }
        assert_eq!(floor.and_then(|f| f.face), Some(Face::Up));
        assert_eq!(face_normal(Face::West), DVec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn sprites_come_from_the_item_textures() {
        assert_eq!(
            texture("minecraft:ender_pearl").map(|id| id.key()),
            Some("minecraft:item/ender_pearl".to_owned())
        );
    }
}
