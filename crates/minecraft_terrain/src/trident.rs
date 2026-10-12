use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::thrown::{PlayerBody, Throw, shot_direction};
use glam::{DVec3, Quat};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::potion::PotionTarget;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Block, Pos, World};
use std::collections::BTreeMap;

pub const TEXTURE: &str = "minecraft:entity/trident/trident";
pub const TEXTURE_SIZE: [f32; 2] = [32.0, 32.0];
pub const ITEM: &str = "minecraft:trident";
pub const DAMAGE: f32 = 8.0;
pub const DAMAGE_TYPE: &str = "minecraft:trident";
pub const POWER: f32 = 2.5;
pub const CHARGE_TICKS: u32 = 10;
pub const GRAVITY: f64 = 0.05;
pub const EYE_HEIGHT: f64 = 1.62;
pub const THROW_SOUND: &str = "minecraft:item.trident.throw";
pub const HIT_SOUND: &str = "minecraft:item.trident.hit";
pub const HIT_GROUND_SOUND: &str = "minecraft:item.trident.hit_ground";
pub const RETURN_SOUND: &str = "minecraft:item.trident.return";
const RIPTIDE_SOUNDS: [&str; 3] = [
    "minecraft:item.trident.riptide_1",
    "minecraft:item.trident.riptide_2",
    "minecraft:item.trident.riptide_3",
];
const HALF: f64 = 0.25;
const DESPAWN_TICKS: i32 = 1200;
const SHAKE_TICKS: i32 = 7;
const FALL_CHECK: f64 = 0.06;
const SPREAD: f64 = 0.0172275;
pub const UNKNOWN_OWNER: u64 = u64::MAX;
const ENTITY: &str = "minecraft:trident";

pub fn enchantment(stack: &ItemStack, id: &str) -> i32 {
    crate::stations::enchantments_of(stack)
        .into_iter()
        .find(|(name, _)| name == id)
        .map_or(0, |(_, level)| level)
}

pub fn loyalty(stack: &ItemStack) -> i32 {
    enchantment(stack, "minecraft:loyalty").clamp(0, 127)
}

pub fn riptide_strength(stack: &ItemStack) -> f32 {
    match enchantment(stack, "minecraft:riptide") {
        level if level > 0 => 1.5 + 0.75 * (level - 1) as f32,
        _ => 0.0,
    }
}

pub fn throw_sound(stack: &ItemStack) -> &'static str {
    match enchantment(stack, "minecraft:riptide") {
        level if level > 0 => RIPTIDE_SOUNDS[(level.min(3) - 1) as usize],
        _ => THROW_SOUND,
    }
}

pub fn next_damage_breaks(inventory: &Inventory, stack: &ItemStack) -> bool {
    inventory
        .recipes
        .durability(stack)
        .is_some_and(|(damage, max)| damage + 1 >= max)
}

pub fn can_start(inventory: &Inventory, stack: &ItemStack, wet: bool) -> bool {
    stack.id == ITEM
        && !next_damage_breaks(inventory, stack)
        && (riptide_strength(stack) <= 0.0 || wet)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Release {
    Nothing,
    Throw(ItemStack),
    Riptide { strength: f32, sound: &'static str },
}

fn wear(inventory: &mut Inventory, slot: usize, random: &mut LegacyRandom) {
    crate::durability::wear(inventory, slot, 1, || random.next_float());
}

pub fn release(
    inventory: &mut Inventory,
    slot: usize,
    used_ticks: u32,
    creative: bool,
    wet: bool,
    random: &mut LegacyRandom,
) -> Release {
    if used_ticks < CHARGE_TICKS {
        return Release::Nothing;
    }
    let Some(stack) = inventory.slots.get(slot).cloned().flatten() else {
        return Release::Nothing;
    };
    let strength = riptide_strength(&stack);
    if stack.id != ITEM || (strength > 0.0 && !wet) || next_damage_breaks(inventory, &stack) {
        return Release::Nothing;
    }
    if !creative {
        wear(inventory, slot, random);
    }
    if strength > 0.0 {
        return Release::Riptide {
            strength,
            sound: throw_sound(&stack),
        };
    }
    let Some(mut thrown) = inventory.slots.get(slot).cloned().flatten() else {
        return Release::Nothing;
    };
    thrown.count = 1;
    if !creative {
        if let Some(held) = inventory.slots[slot].as_mut() {
            held.count = held.count.saturating_sub(1);
            if held.count == 0 {
                inventory.slots[slot] = None;
            }
        }
    }
    Release::Throw(thrown)
}

pub fn riptide_push(yaw: f32, pitch: f32, strength: f32) -> DVec3 {
    let direction = shot_direction(yaw, pitch);
    let length = direction.length();
    if length <= 0.0 {
        DVec3::ZERO
    } else {
        direction / length * f64::from(strength)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pickup {
    Allowed,
    CreativeOnly,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TridentOwner {
    pub eye: DVec3,
    pub alive: bool,
    pub spectator: bool,
    pub bounds: (DVec3, DVec3),
}

impl TridentOwner {
    pub fn of(body: &PlayerBody) -> Self {
        Self {
            eye: body.feet + DVec3::Y * EYE_HEIGHT,
            alive: body.alive,
            spectator: body.spectator,
            bounds: body.bounds(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TridentEvent {
    Struck {
        target: u64,
        velocity: DVec3,
    },
    Sound {
        event: &'static str,
        at: DVec3,
        volume: f32,
        pitch: f32,
    },
    Dropped {
        at: DVec3,
    },
}

#[derive(Clone, Debug)]
pub struct ThrownTrident {
    pub id: i32,
    pub owner: u64,
    pub owner_uuid: Option<u128>,
    pub stack: ItemStack,
    pub pickup: Pickup,
    pub loyalty: i32,
    pub foil: bool,
    pub position: DVec3,
    pub previous: DVec3,
    pub velocity: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub previous_yaw: f32,
    pub previous_pitch: f32,
    pub in_ground: bool,
    pub in_ground_time: i32,
    pub life: i32,
    pub shake: i32,
    pub dealt_damage: bool,
    pub no_physics: bool,
    pub left_owner: bool,
    pub returning: i32,
    pub tick_count: i32,
    pub alive: bool,
    last_state: Option<Block>,
    random: LegacyRandom,
}

fn lerp_rotation(mut from: f32, to: f32) -> f32 {
    while to - from < -180.0 {
        from -= 360.0;
    }
    while to - from >= 180.0 {
        from += 360.0;
    }
    from + 0.2 * (to - from)
}

fn heading(v: DVec3) -> (f32, f32) {
    let horizontal = (v.x * v.x + v.z * v.z).sqrt();
    (
        v.x.atan2(v.z).to_degrees() as f32,
        v.y.atan2(horizontal).to_degrees() as f32,
    )
}

fn sweep(min: DVec3, max: DVec3, by: DVec3) -> (DVec3, DVec3) {
    (min + by.min(DVec3::ZERO), max + by.max(DVec3::ZERO))
}

fn meets(a: (DVec3, DVec3), b: (DVec3, DVec3)) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
}

fn signum(v: f64) -> f64 {
    if v == 0.0 { v } else { v.signum() }
}

impl ThrownTrident {
    pub fn launch(
        throw: &Throw,
        stack: ItemStack,
        creative: bool,
        random: &mut LegacyRandom,
    ) -> Self {
        let start = throw.eye - DVec3::new(0.0, f64::from(0.1_f32), 0.0);
        let direction = shot_direction(throw.yaw, throw.pitch);
        let length = direction.length();
        let unit = if length < f64::from(1.0e-5_f32) {
            DVec3::ZERO
        } else {
            direction / length
        };
        let mut triangle = || SPREAD * (random.next_double() - random.next_double());
        let spread = DVec3::new(triangle(), triangle(), triangle());
        let mut velocity = (unit + spread) * f64::from(POWER);
        let (yaw, pitch) = heading(velocity);
        let motion = throw.motion;
        velocity += DVec3::new(
            motion.x,
            if throw.on_ground { 0.0 } else { motion.y },
            motion.z,
        );
        Self {
            id: 0,
            owner: throw.owner,
            owner_uuid: None,
            loyalty: loyalty(&stack),
            foil: crate::glint::has_glint(&stack),
            stack,
            pickup: if creative {
                Pickup::CreativeOnly
            } else {
                Pickup::Allowed
            },
            position: start,
            previous: start,
            velocity,
            yaw,
            pitch,
            previous_yaw: yaw,
            previous_pitch: pitch,
            in_ground: false,
            in_ground_time: 0,
            life: 0,
            shake: 0,
            dealt_damage: false,
            no_physics: false,
            left_owner: false,
            returning: 0,
            tick_count: 0,
            alive: true,
            last_state: None,
            random: LegacyRandom::new(random.next_long()),
        }
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        let p = self.position;
        (
            DVec3::new(p.x - HALF, p.y, p.z - HALF),
            DVec3::new(p.x + HALF, p.y + 2.0 * HALF, p.z + HALF),
        )
    }

    fn margin(&self) -> f64 {
        f64::from(0.0_f32.max(0.3_f32.min((self.tick_count - 2) as f32 / 20.0)))
    }

    fn cell(&self) -> Pos {
        (
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        )
    }

    fn inside_block(&self, world: &impl World) -> bool {
        let p = self.position;
        minecraftoss_player::collision_boxes_at(world, self.cell())
            .iter()
            .any(|(min, max)| {
                min.x <= p.x
                    && p.x <= max.x
                    && min.y <= p.y
                    && p.y <= max.y
                    && min.z <= p.z
                    && p.z <= max.z
            })
    }

    fn should_fall(&self, world: &impl World) -> bool {
        let reach = DVec3::splat(FALL_CHECK);
        let (low, high) = (self.position - reach, self.position + reach);
        for x in low.x.floor() as i32..=high.x.floor() as i32 {
            for y in low.y.floor() as i32..=high.y.floor() as i32 {
                for z in low.z.floor() as i32..=high.z.floor() as i32 {
                    if minecraftoss_player::collision_boxes_at(world, (x, y, z))
                        .into_iter()
                        .any(|b| meets((low, high), b))
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn despawns(&self) -> bool {
        !(self.pickup == Pickup::Allowed && self.loyalty > 0)
    }

    fn come_back(&mut self, owner: TridentOwner, events: &mut Vec<TridentEvent>) -> bool {
        if !owner.alive || owner.spectator {
            if self.pickup == Pickup::Allowed {
                events.push(TridentEvent::Dropped {
                    at: self.position + DVec3::new(0.0, f64::from(0.1_f32), 0.0),
                });
            }
            self.alive = false;
            return false;
        }
        self.no_physics = true;
        let towards = owner.eye - self.position;
        let loyalty = f64::from(self.loyalty);
        self.position.y += towards.y * 0.015 * loyalty;
        let length = towards.length();
        let pull = if length < f64::from(1.0e-5_f32) {
            DVec3::ZERO
        } else {
            towards / length * (0.05 * loyalty)
        };
        self.velocity = self.velocity * 0.95 + pull;
        if self.returning == 0 {
            events.push(TridentEvent::Sound {
                event: RETURN_SOUND,
                at: self.position,
                volume: 10.0,
                pitch: 1.0,
            });
        }
        self.returning += 1;
        true
    }

    fn stuck(&mut self, world: &impl World) {
        if world.block(self.cell()) != self.last_state && self.should_fall(world) {
            self.in_ground = false;
            let (a, b, c) = (
                self.random.next_float() * 0.2,
                self.random.next_float() * 0.2,
                self.random.next_float() * 0.2,
            );
            self.velocity *= DVec3::new(f64::from(a), f64::from(b), f64::from(c));
            self.life = 0;
        } else if self.despawns() {
            self.life += 1;
            if self.life >= DESPAWN_TICKS {
                self.alive = false;
            }
        }
        self.in_ground_time += 1;
    }

    fn struck(&self, targets: &[PotionTarget], from: DVec3, end: DVec3) -> Option<(u64, DVec3)> {
        if self.dealt_damage {
            return None;
        }
        let (min, max) = self.bounds();
        let (min, max) = sweep(min, max, self.velocity);
        let search = (min - DVec3::ONE, max + DVec3::ONE);
        let grow = DVec3::splat(self.margin());
        targets
            .iter()
            .filter(|t| !(t.owner && !self.left_owner) && meets((t.min, t.max), search))
            .filter_map(|t| {
                minecraftoss_entities::sight::clip_box(t.min - grow, t.max + grow, from, end)
                    .map(|at| (from.distance_squared(at), t.id, at))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id, at)| (id, at))
    }

    pub fn tick(
        &mut self,
        world: &impl World,
        targets: &[PotionTarget],
        owner: Option<TridentOwner>,
    ) -> Vec<TridentEvent> {
        let mut events = Vec::new();
        if !self.alive {
            return events;
        }
        self.tick_count += 1;
        self.previous = self.position;
        self.previous_yaw = self.yaw;
        self.previous_pitch = self.pitch;
        if self.in_ground_time > 4 {
            self.dealt_damage = true;
        }
        if let Some(owner) =
            owner.filter(|_| self.loyalty > 0 && (self.dealt_damage || self.no_physics))
        {
            if !self.come_back(owner, &mut events) {
                return events;
            }
        }
        let physics = !self.no_physics;
        if physics && self.inside_block(world) {
            self.velocity = DVec3::ZERO;
            self.in_ground = true;
        }
        if self.shake > 0 {
            self.shake -= 1;
        }
        if self.in_ground && physics {
            self.stuck(world);
            return events;
        }
        self.in_ground_time = 0;
        let (yaw, pitch) = heading(self.velocity);
        let yaw = if physics {
            yaw
        } else {
            heading(-self.velocity).0
        };
        self.pitch = lerp_rotation(self.pitch, pitch);
        self.yaw = lerp_rotation(self.yaw, yaw);
        if !self.left_owner {
            let (min, max) = self.bounds();
            let (min, max) = sweep(min, max, self.velocity);
            let reach = (min - DVec3::ONE, max + DVec3::ONE);
            self.left_owner = owner.is_none_or(|o| !meets(o.bounds, reach));
        }
        if physics {
            let from = self.position;
            let to = from + self.velocity;
            let block = minecraftoss_entities::sight::clip(world, from, to);
            let end = block.map_or(to, |(_, at)| at);
            if let Some((target, at)) = self.struck(targets, from, end) {
                self.position = at;
                events.push(TridentEvent::Struck {
                    target,
                    velocity: self.velocity,
                });
                self.dealt_damage = true;
                events.push(TridentEvent::Sound {
                    event: HIT_SOUND,
                    at,
                    volume: 1.0,
                    pitch: 1.0,
                });
                self.velocity *= DVec3::new(-0.01, -0.1, -0.01);
                let turn = 170.0 + self.random.next_float() * 20.0;
                self.yaw += turn;
                self.previous_yaw += turn;
            } else if let Some((pos, at)) = block {
                self.last_state = world.block(pos);
                let sign = DVec3::new(
                    signum(self.velocity.x),
                    signum(self.velocity.y),
                    signum(self.velocity.z),
                );
                self.position = at - sign * f64::from(0.05_f32);
                self.velocity = DVec3::ZERO;
                let pitch = 1.2 / (self.random.next_float() * 0.2 + 0.9);
                events.push(TridentEvent::Sound {
                    event: HIT_GROUND_SOUND,
                    at: self.position,
                    volume: 1.0,
                    pitch,
                });
                self.in_ground = true;
                self.shake = SHAKE_TICKS;
            } else {
                self.position = to;
            }
        } else {
            self.position += self.velocity;
        }
        self.velocity *= f64::from(0.99_f32);
        if physics && !self.in_ground {
            self.velocity.y -= GRAVITY;
        }
        if self.position.y < f64::from(world.min_y() - 64) {
            self.alive = false;
        }
        events
    }

    pub fn touched_by(&self, player: &PlayerBody) -> bool {
        let (min, max) = player.bounds();
        let grow = DVec3::new(1.0, 0.5, 1.0);
        player.alive && !player.spectator && meets((min - grow, max + grow), self.bounds())
    }

    pub fn taken_by(&self, player: u64, owner_here: bool) -> Option<Option<ItemStack>> {
        if !self.alive
            || (player != self.owner && owner_here)
            || !(self.in_ground || self.no_physics)
            || self.shake > 0
        {
            return None;
        }
        match self.pickup {
            Pickup::Allowed => Some(Some(self.stack.clone())),
            Pickup::CreativeOnly => Some(None),
        }
    }

    pub fn to_tag(&self, owner: Option<u128>) -> Tag {
        let doubles =
            |v: DVec3| Tag::List(vec![Tag::Double(v.x), Tag::Double(v.y), Tag::Double(v.z)]);
        let mut item = BTreeMap::new();
        item.insert("id".to_owned(), Tag::String(self.stack.id.clone()));
        item.insert(
            "count".to_owned(),
            Tag::Int(i32::from(self.stack.count.max(1))),
        );
        if let Some(components) = &self.stack.components {
            item.insert(
                "components".to_owned(),
                crate::item_nbt::json_tag(components),
            );
        }
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(ENTITY.to_owned()));
        map.insert("Pos".to_owned(), doubles(self.position));
        map.insert("Motion".to_owned(), doubles(self.velocity));
        map.insert(
            "Rotation".to_owned(),
            Tag::List(vec![Tag::Float(self.yaw), Tag::Float(self.pitch)]),
        );
        if let Some(owner) = owner {
            map.insert(
                "Owner".to_owned(),
                Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(owner).to_vec()),
            );
        }
        map.insert("LeftOwner".to_owned(), Tag::Byte(i8::from(self.left_owner)));
        map.insert("HasBeenShot".to_owned(), Tag::Byte(1));
        map.insert(
            "life".to_owned(),
            Tag::Short(self.life.clamp(0, 32767) as i16),
        );
        map.insert(
            "shake".to_owned(),
            Tag::Byte(self.shake.clamp(0, 127) as i8),
        );
        map.insert("inGround".to_owned(), Tag::Byte(i8::from(self.in_ground)));
        map.insert(
            "pickup".to_owned(),
            Tag::Byte(match self.pickup {
                Pickup::Allowed => 1,
                Pickup::CreativeOnly => 2,
            }),
        );
        map.insert("damage".to_owned(), Tag::Double(2.0));
        map.insert("crit".to_owned(), Tag::Byte(0));
        map.insert("PierceLevel".to_owned(), Tag::Byte(0));
        map.insert(
            "SoundEvent".to_owned(),
            Tag::String(HIT_GROUND_SOUND.to_owned()),
        );
        map.insert(
            "DealtDamage".to_owned(),
            Tag::Byte(i8::from(self.dealt_damage)),
        );
        map.insert("item".to_owned(), Tag::Compound(item));
        if let Some(state) = &self.last_state {
            let mut block = BTreeMap::new();
            block.insert("Name".to_owned(), Tag::String(state.id.clone()));
            if !state.properties.is_empty() {
                block.insert(
                    "Properties".to_owned(),
                    Tag::Compound(
                        state
                            .properties
                            .iter()
                            .map(|(k, v)| (k.clone(), Tag::String(v.clone())))
                            .collect(),
                    ),
                );
            }
            map.insert("inBlockState".to_owned(), Tag::Compound(block));
        }
        Tag::Compound(map)
    }

    pub fn from_tag(tag: &Tag) -> Option<Self> {
        if tag.get("id").and_then(Tag::as_str) != Some(ENTITY) {
            return None;
        }
        let triple = |key: &str| -> Option<DVec3> {
            let list = tag.get(key)?.as_list()?;
            let at = |i: usize| match list.get(i)? {
                Tag::Double(v) => Some(*v),
                Tag::Float(v) => Some(f64::from(*v)),
                _ => None,
            };
            Some(DVec3::new(at(0)?, at(1)?, at(2)?))
        };
        let number = |key: &str| -> i64 {
            match tag.get(key) {
                Some(Tag::Byte(v)) => i64::from(*v),
                Some(Tag::Short(v)) => i64::from(*v),
                Some(Tag::Int(v)) => i64::from(*v),
                Some(Tag::Long(v)) => *v,
                _ => 0,
            }
        };
        let position = triple("Pos")?;
        let item = tag.get("item")?;
        let mut stack = ItemStack::new(item.get("id").and_then(Tag::as_str).unwrap_or(ITEM), 1);
        stack.max = 1;
        stack.components = item.get("components").map(crate::item_nbt::tag_json);
        let rotation = tag.get("Rotation").and_then(Tag::as_list);
        let angle = |i: usize| match rotation.and_then(|r| r.get(i)) {
            Some(Tag::Float(v)) => *v,
            _ => 0.0,
        };
        let last_state = tag.get("inBlockState").and_then(|state| {
            let mut block = Block::new(state.get("Name")?.as_str()?);
            if let Some(Tag::Compound(properties)) = state.get("Properties") {
                for (key, value) in properties {
                    if let Some(value) = value.as_str() {
                        block = block.with(key, value);
                    }
                }
            }
            Some(block)
        });
        let owner_uuid = tag
            .get("Owner")
            .and_then(Tag::as_ints)
            .and_then(|ints| <[i32; 4]>::try_from(ints).ok())
            .map(minecraftoss_entities::gossip::uuid_from_ints);
        Some(Self {
            id: 0,
            owner: UNKNOWN_OWNER,
            owner_uuid,
            loyalty: loyalty(&stack),
            foil: crate::glint::has_glint(&stack),
            stack,
            pickup: if number("pickup") == 2 {
                Pickup::CreativeOnly
            } else {
                Pickup::Allowed
            },
            position,
            previous: position,
            velocity: triple("Motion").unwrap_or(DVec3::ZERO),
            yaw: angle(0),
            pitch: angle(1),
            previous_yaw: angle(0),
            previous_pitch: angle(1),
            in_ground: number("inGround") != 0,
            in_ground_time: 0,
            life: number("life") as i32,
            shake: number("shake") as i32,
            dealt_damage: number("DealtDamage") != 0,
            no_physics: false,
            left_owner: number("LeftOwner") != 0,
            returning: 0,
            tick_count: 0,
            alive: true,
            last_state,
            random: LegacyRandom::new(
                position.x.to_bits() ^ position.y.to_bits().rotate_left(21) ^ position.z.to_bits(),
            ),
        })
    }

    pub fn view(&self) -> TridentView {
        TridentView {
            previous: self.previous,
            position: self.position,
            previous_yaw: self.previous_yaw,
            yaw: self.yaw,
            previous_pitch: self.previous_pitch,
            pitch: self.pitch,
            foil: self.foil,
            owner: self.owner,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelCube {
    pub uv: [f32; 2],
    pub origin: [f32; 3],
    pub size: [f32; 3],
    pub mirror: bool,
}

pub const MODEL: [ModelCube; 5] = [
    ModelCube {
        uv: [0.0, 6.0],
        origin: [-0.5, 2.0, -0.5],
        size: [1.0, 25.0, 1.0],
        mirror: false,
    },
    ModelCube {
        uv: [4.0, 0.0],
        origin: [-1.5, 0.0, -0.5],
        size: [3.0, 2.0, 1.0],
        mirror: false,
    },
    ModelCube {
        uv: [4.0, 3.0],
        origin: [-2.5, -3.0, -0.5],
        size: [1.0, 4.0, 1.0],
        mirror: false,
    },
    ModelCube {
        uv: [0.0, 0.0],
        origin: [-0.5, -4.0, -0.5],
        size: [1.0, 4.0, 1.0],
        mirror: false,
    },
    ModelCube {
        uv: [4.0, 3.0],
        origin: [1.5, -3.0, -0.5],
        size: [1.0, 4.0, 1.0],
        mirror: true,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TridentView {
    pub previous: DVec3,
    pub position: DVec3,
    pub previous_yaw: f32,
    pub yaw: f32,
    pub previous_pitch: f32,
    pub pitch: f32,
    pub foil: bool,
    pub owner: u64,
}

impl TridentView {
    pub fn at(&self, partial: f32) -> DVec3 {
        self.previous
            .lerp(self.position, f64::from(partial.clamp(0.0, 1.0)))
    }

    pub fn rotation(&self, partial: f32) -> Quat {
        let partial = partial.clamp(0.0, 1.0);
        let yaw = self.previous_yaw + (self.yaw - self.previous_yaw) * partial;
        let pitch = self.previous_pitch + (self.pitch - self.previous_pitch) * partial;
        Quat::from_rotation_y((yaw - 90.0).to_radians())
            * Quat::from_rotation_z((pitch + 90.0).to_radians())
    }

    pub fn tip(&self, partial: f32) -> DVec3 {
        self.at(partial)
            + (self.rotation(partial) * glam::Vec3::new(0.0, -4.0 / 16.0, 0.0)).as_dvec3()
    }
}

pub fn texture() -> Option<ResourceId> {
    ResourceId::parse(TEXTURE).ok()
}

pub fn append_tridents<'a>(
    mesh: &mut ChunkMesh,
    tridents: impl IntoIterator<Item = &'a TridentView>,
    partial: f32,
    atlas: &Atlas,
    light: &SkyLight,
) {
    let Some(texture) = texture().filter(|id| atlas.contains(id)) else {
        return;
    };
    let region = atlas.region(&texture);
    let flip = Quat::from_rotation_z(std::f32::consts::PI);
    for trident in tridents {
        let at = trident.at(partial);
        let cell = (
            at.x.floor() as i32,
            at.y.floor() as i32,
            at.z.floor() as i32,
        );
        let (sky, block) = (light.get(cell) as f32, light.get_block(cell) as f32);
        let rotation = trident.rotation(partial);
        let first = mesh.vertices.len() as u32;
        for cube in MODEL {
            let to = [
                cube.origin[0] + cube.size[0],
                cube.origin[1] + cube.size[1],
                cube.origin[2] + cube.size[2],
            ];
            crate::cow_render::cube_scaled(
                mesh,
                at,
                rotation,
                glam::Vec3::ONE,
                region,
                sky,
                block,
                cube.origin,
                to,
                cube.uv,
                [0.0, 24.016, 0.0],
                flip,
                [1.0; 3],
                TEXTURE_SIZE,
                None,
                cube.mirror,
            );
        }
        if trident.foil {
            mesh.glint.push(first..mesh.vertices.len() as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn floor() -> Blocks {
        let mut world = Blocks::default();
        for x in -3..=3 {
            for z in -3..=60 {
                world.set_block((x, 60, z), Some(Block::new("minecraft:stone")));
            }
        }
        world
    }

    fn trident(components: serde_json::Value) -> ItemStack {
        let mut stack = ItemStack::new(ITEM, 1);
        stack.max = 1;
        stack.components = Some(components);
        stack
    }

    fn worn(damage: i32) -> ItemStack {
        trident(serde_json::json!({"minecraft:max_damage": 250, "minecraft:damage": damage}))
    }

    fn held(stack: ItemStack) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(stack);
        inventory
    }

    fn damage(stack: Option<&ItemStack>) -> Option<u64> {
        stack
            .and_then(|s| s.components.as_ref())
            .and_then(|c| c.get("minecraft:damage"))
            .and_then(serde_json::Value::as_u64)
    }

    fn launched(stack: ItemStack, pitch: f32, owner: u64) -> ThrownTrident {
        let mut random = LegacyRandom::new(12);
        let components = stack.components.clone();
        let throw = Throw {
            owner,
            item: ITEM,
            components: components.as_ref(),
            eye: DVec3::new(0.5, 62.62, 0.5),
            yaw: 0.0,
            pitch,
            motion: DVec3::ZERO,
            on_ground: true,
        };
        ThrownTrident::launch(&throw, stack, false, &mut random)
    }

    #[test]
    fn released_before_10_ticks_does_not_throw() {
        let mut random = LegacyRandom::new(1);
        let mut inventory = held(worn(5));
        assert_eq!(
            release(&mut inventory, 0, 9, false, false, &mut random),
            Release::Nothing
        );
        assert_eq!(damage(inventory.slots[0].as_ref()), Some(5));
        let Release::Throw(thrown) = release(&mut inventory, 0, 10, false, false, &mut random)
        else {
            panic!("a full charge throws");
        };
        assert_eq!(thrown.count, 1);
        assert_eq!(
            damage(Some(&thrown)),
            Some(6),
            "a throw costs one durability"
        );
        assert!(inventory.slots[0].is_none());
        let mut creative = held(worn(5));
        assert!(matches!(
            release(&mut creative, 0, 30, true, false, &mut random),
            Release::Throw(_)
        ));
        assert_eq!(damage(creative.slots[0].as_ref()), Some(5));
    }

    #[test]
    fn a_trident_one_use_from_breaking_is_not_thrown() {
        let mut random = LegacyRandom::new(2);
        let inventory = held(worn(249));
        assert!(!can_start(
            &inventory,
            inventory.slots[0].as_ref().unwrap(),
            true
        ));
        let mut inventory = held(worn(249));
        assert_eq!(
            release(&mut inventory, 0, 40, false, false, &mut random),
            Release::Nothing
        );
        assert!(inventory.slots[0].is_some());
    }

    #[test]
    fn riptide_needs_water_or_rain_and_launches_instead_of_throwing() {
        let mut random = LegacyRandom::new(3);
        let riptide =
            trident(serde_json::json!({"minecraft:enchantments": {"minecraft:riptide": 3}}));
        let inventory = held(riptide.clone());
        assert!(!can_start(&inventory, &riptide, false));
        assert!(can_start(&inventory, &riptide, true));
        let mut inventory = held(riptide.clone());
        assert_eq!(
            release(&mut inventory, 0, 20, false, false, &mut random),
            Release::Nothing
        );
        assert_eq!(
            release(&mut inventory, 0, 20, false, true, &mut random),
            Release::Riptide {
                strength: 3.0,
                sound: "minecraft:item.trident.riptide_3"
            }
        );
        assert!(inventory.slots[0].is_some());
        assert!((riptide_push(0.0, -90.0, 3.0) - DVec3::new(0.0, 3.0, 0.0)).length() < 1e-5);
        let first =
            trident(serde_json::json!({"minecraft:enchantments": {"minecraft:riptide": 1}}));
        assert_eq!(riptide_strength(&first), 1.5);
        assert_eq!(throw_sound(&worn(0)), THROW_SOUND);
    }

    #[test]
    fn a_trident_flies_at_two_and_a_half_and_sticks_where_it_lands() {
        let world = floor();
        let mut thrown = launched(worn(0), 0.0, 1);
        assert!((thrown.velocity.length() - 2.5).abs() < 0.1);
        assert!((thrown.position.y - (62.62 - f64::from(0.1_f32))).abs() < 1e-9);
        let mut sounds = Vec::new();
        for _ in 0..100 {
            for event in thrown.tick(&world, &[], None) {
                if let TridentEvent::Sound { event, .. } = event {
                    sounds.push(event);
                }
            }
            if thrown.in_ground {
                break;
            }
        }
        assert!(thrown.in_ground);
        assert_eq!(sounds, vec![HIT_GROUND_SOUND]);
        assert!(thrown.position.y > 61.0 && thrown.position.y < 61.1);
        assert!(thrown.pitch < 0.0, "{}", thrown.pitch);
        let still = thrown.position;
        for _ in 0..20 {
            thrown.tick(&world, &[], None);
        }
        assert_eq!(thrown.position, still);
        for _ in 0..1200 {
            thrown.tick(&world, &[], None);
        }
        assert!(!thrown.alive);
    }

    #[test]
    fn a_trident_strikes_a_mob_once_and_drops_back() {
        let world = floor();
        let mut thrown = launched(worn(0), 0.0, 1);
        let zombie = PotionTarget {
            id: 7,
            min: DVec3::new(0.2, 61.0, 6.0),
            max: DVec3::new(0.8, 62.95, 6.6),
            owner: false,
        };
        let mut struck = Vec::new();
        for _ in 0..60 {
            for event in thrown.tick(&world, &[zombie], None) {
                if let TridentEvent::Struck { target, velocity } = event {
                    struck.push((target, velocity));
                }
            }
        }
        assert_eq!(struck.len(), 1);
        assert_eq!(struck[0].0, 7);
        assert!(struck[0].1.z > 2.0);
        assert!(thrown.dealt_damage && thrown.in_ground);
        assert!(thrown.position.z < 6.0, "{:?}", thrown.position);
    }

    fn loyal(level: i32) -> ItemStack {
        trident(serde_json::json!({"minecraft:enchantments": {"minecraft:loyalty": level}}))
    }

    fn owner_at(feet: DVec3, alive: bool) -> TridentOwner {
        TridentOwner::of(&PlayerBody {
            id: 1,
            feet,
            alive,
            spectator: false,
        })
    }

    #[test]
    fn loyalty_pulls_it_back_to_its_thrower() {
        let world = floor();
        let mut thrown = launched(loyal(3), 20.0, 1);
        assert_eq!(thrown.loyalty, 3);
        let owner = owner_at(DVec3::new(0.5, 61.0, 0.5), true);
        let mut returning_sounds = 0;
        let mut closest = f64::MAX;
        for _ in 0..200 {
            for event in thrown.tick(&world, &[], Some(owner)) {
                if matches!(
                    event,
                    TridentEvent::Sound {
                        event: RETURN_SOUND,
                        ..
                    }
                ) {
                    returning_sounds += 1;
                }
            }
            if thrown.no_physics {
                closest = closest.min(thrown.position.distance(owner.eye));
            }
        }
        assert!(thrown.no_physics, "it let go of the ground");
        assert_eq!(returning_sounds, 1);
        assert!(closest < 1.0, "{closest}");
        assert!(thrown.alive);
        assert_eq!(
            thrown.taken_by(1, true).flatten().map(|s| s.id),
            Some(ITEM.to_owned())
        );
    }

    #[test]
    fn a_loyal_trident_never_despawns_but_drops_when_its_thrower_dies() {
        let world = floor();
        let mut thrown = launched(loyal(1), 0.0, 1);
        for _ in 0..1500 {
            thrown.tick(&world, &[], None);
        }
        assert!(thrown.alive && thrown.in_ground);
        let events = thrown.tick(&world, &[], Some(owner_at(DVec3::ZERO, false)));
        assert!(
            matches!(events.as_slice(), [TridentEvent::Dropped { .. }]),
            "{events:?}"
        );
        assert!(!thrown.alive);
    }

    #[test]
    fn a_stuck_trident_saves_as_the_vanilla_entity_and_loads_back() {
        let world = floor();
        let mut thrown = launched(loyal(2), 30.0, 1);
        while !thrown.in_ground {
            thrown.tick(&world, &[], None);
        }
        let tag = thrown.to_tag(Some(0x1234_5678_9abc_def0_1122_3344_5566_7788));
        assert_eq!(
            tag.get("id").and_then(Tag::as_str),
            Some("minecraft:trident")
        );
        assert_eq!(tag.get("inGround"), Some(&Tag::Byte(1)));
        assert_eq!(
            tag.get("inBlockState")
                .and_then(|b| b.get("Name"))
                .and_then(Tag::as_str),
            Some("minecraft:stone")
        );
        let back = ThrownTrident::from_tag(&tag).expect("a trident");
        assert_eq!(back.owner, UNKNOWN_OWNER);
        assert_eq!(
            back.owner_uuid,
            Some(0x1234_5678_9abc_def0_1122_3344_5566_7788)
        );
        assert_eq!(
            (back.position, back.in_ground, back.shake, back.loyalty),
            (thrown.position, true, thrown.shake, 2)
        );
        assert_eq!(back.stack, thrown.stack);
        assert_eq!(back.pickup, Pickup::Allowed);
        let mut back = back;
        for _ in 0..10 {
            back.tick(&world, &[], None);
        }
        assert!(
            back.in_ground && back.alive,
            "it stays stuck in the same stone"
        );
        assert!(ThrownTrident::from_tag(&Tag::Compound(BTreeMap::new())).is_none());
        let ownerless = back.to_tag(None);
        assert_eq!(ownerless.get("Owner"), None, "no thrower is made up");
        assert_eq!(
            ThrownTrident::from_tag(&ownerless).map(|t| t.owner_uuid),
            Some(None)
        );
    }

    #[test]
    fn a_settled_trident_is_its_throwers_while_the_thrower_is_here() {
        let world = floor();
        let mut thrown = launched(worn(3), 30.0, 4);
        while !thrown.in_ground {
            thrown.tick(&world, &[], None);
        }
        assert_eq!(thrown.taken_by(4, true), None);
        for _ in 0..7 {
            thrown.tick(&world, &[], None);
        }
        assert_eq!(
            thrown.taken_by(4, true).flatten().map(|s| s.id),
            Some(ITEM.to_owned())
        );
        assert_eq!(thrown.taken_by(5, true), None);
        assert_eq!(
            thrown.taken_by(5, false).flatten().map(|s| s.id),
            Some(ITEM.to_owned()),
            "anyone takes it while its thrower is not in the world"
        );
        let near = PlayerBody {
            id: 4,
            feet: thrown.position + DVec3::new(1.2, -0.5, 0.0),
            alive: true,
            spectator: false,
        };
        let far = PlayerBody {
            feet: thrown.position + DVec3::new(3.0, 0.0, 0.0),
            ..near
        };
        assert!(thrown.touched_by(&near) && !thrown.touched_by(&far));
        thrown.pickup = Pickup::CreativeOnly;
        assert_eq!(thrown.taken_by(4, true), Some(None));
    }

    #[test]
    fn a_creative_loyal_trident_coming_back_is_taken_without_a_copy() {
        let world = floor();
        let mut random = LegacyRandom::new(12);
        let throw = Throw {
            owner: 1,
            item: ITEM,
            components: None,
            eye: DVec3::new(0.5, 62.62, 0.5),
            yaw: 0.0,
            pitch: 20.0,
            motion: DVec3::ZERO,
            on_ground: true,
        };
        let mut thrown = ThrownTrident::launch(&throw, loyal(3), true, &mut random);
        let owner = owner_at(DVec3::new(0.5, 61.0, 0.5), true);
        for _ in 0..200 {
            thrown.tick(&world, &[], Some(owner));
            if thrown.no_physics && thrown.shake == 0 {
                break;
            }
        }
        assert!(thrown.no_physics, "it is on its way back");
        assert_eq!(
            thrown.taken_by(1, true),
            Some(None),
            "a creative throw leaves the trident in the hand and adds no second one"
        );
    }

    fn view(yaw: f32, pitch: f32) -> TridentView {
        TridentView {
            previous: DVec3::new(0.0, 64.0, 0.0),
            position: DVec3::new(0.0, 64.0, 0.0),
            previous_yaw: yaw,
            yaw,
            previous_pitch: pitch,
            pitch,
            foil: false,
            owner: 0,
        }
    }

    #[test]
    fn the_spikes_lead_along_the_heading() {
        let south = view(0.0, 0.0).tip(1.0) - DVec3::new(0.0, 64.0, 0.0);
        assert!(
            (south - DVec3::new(0.0, 0.0, 0.25)).length() < 1e-5,
            "{south:?}"
        );
        let east = view(90.0, 0.0).tip(1.0) - DVec3::new(0.0, 64.0, 0.0);
        assert!(
            (east - DVec3::new(0.25, 0.0, 0.0)).length() < 1e-5,
            "{east:?}"
        );
        let up = view(0.0, 90.0).tip(1.0) - DVec3::new(0.0, 64.0, 0.0);
        assert!((up - DVec3::new(0.0, 0.25, 0.0)).length() < 1e-5, "{up:?}");
    }

    #[test]
    fn the_model_is_a_pole_a_base_and_three_spikes_on_a_32_sheet() {
        let length: f32 = MODEL.iter().map(|c| c.size[1]).fold(0.0, f32::max);
        assert_eq!(length, 25.0);
        let low = MODEL
            .iter()
            .map(|c| c.origin[1])
            .fold(f32::INFINITY, f32::min);
        let high = MODEL
            .iter()
            .map(|c| c.origin[1] + c.size[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!((low, high), (-4.0, 27.0));
        assert_eq!(MODEL.iter().filter(|c| c.mirror).count(), 1);
        assert_eq!(texture().map(|id| id.key()), Some(TEXTURE.to_owned()));
    }
}
