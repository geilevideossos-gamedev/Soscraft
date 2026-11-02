//! `SculkPatchFeature` with the world-generation `SculkSpreader`.

use super::multiface::{Spreader, DEFAULT_ORDER};
use super::{int, shuffle, Placeable};
use crate::feature::blocks::FluidType;
use crate::feature::kinds::simple::can_attach_to;
use crate::feature::{Ctx, Library};
use minecraftoss_core::pos::Direction;
use minecraftoss_core::random::WorldgenRandom;
use minecraftoss_core::tags::TagId;
use minecraftoss_core::{BlockId, BlockPos, BlockStateId};
use serde_json::Value;

pub fn parse(lib: &mut Library, kind: &str, json: &Value) -> Option<Result<Box<dyn Placeable>, String>> {
    match kind {
        "sculk_patch" => Some(SculkPatch::parse(lib, json).map(|f| Box::new(f) as Box<dyn Placeable>)),
        _ => None,
    }
}

// `SculkSpreader.createWorldGenSpreader`.
const GROWTH_SPAWN_COST: i32 = 50;
const NO_GROWTH_RADIUS: i32 = 1;
const CHARGE_DECAY_RATE: i32 = 5;
const ADDITIONAL_DECAY_RATE: i32 = 10;
const LEVEL_GROWTH_SPAWN_COST: i32 = 10;
const LEVEL_NO_GROWTH_RADIUS: i32 = 4;
const LEVEL_CHARGE_DECAY_RATE: i32 = 10;
const LEVEL_ADDITIONAL_DECAY_RATE: i32 = 5;
pub const MAX_CHARGE: i32 = 1000;
pub const MAX_CURSORS: usize = 32;
pub const MAX_CURSOR_DISTANCE: i32 = 1024;
pub const CHARGE_EVENT: i32 = 3006;
pub const SPREAD_SOUND: &str = "minecraft:block.sculk.spread";

#[derive(Debug)]
struct SculkPatch {
    charge_count: i32,
    amount_per_charge: i32,
    spread_attempts: i32,
    growth_rounds: i32,
    spread_rounds: i32,
    spreader: SculkSpreader,
}

#[derive(Clone, Debug)]
pub struct SculkSpreader {
    world_generation: bool,
    growth_spawn_cost: i32,
    no_growth_radius: i32,
    charge_decay_rate: i32,
    additional_decay_rate: i32,
    sculk: BlockStateId,
    sculk_block: BlockId,
    vein: BlockId,
    sensor: BlockStateId,
    shrieker: BlockStateId,
    replaceable_spread: TagId,
    replaceable: TagId,
    growth_inhibitors: TagId,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpreadEffects {
    pub sounds: Vec<(BlockPos, &'static str)>,
    pub level_events: Vec<(BlockPos, i32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Behaviour {
    Sculk,
    Vein,
    Default,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub pos: BlockPos,
    pub charge: i32,
    pub update_delay: i32,
    pub decay_delay: i32,
    /// `MultifaceBlock.availableFaces` in `Direction` order, or `None`.
    pub facings: Option<Vec<Direction>>,
}

impl Cursor {
    pub fn new(pos: BlockPos, charge: i32) -> Self {
        Self { pos, charge, update_delay: 0, decay_delay: 1, facings: None }
    }

    fn unreasonable(&self, origin: BlockPos) -> bool {
        (self.pos.x - origin.x).abs().max((self.pos.y - origin.y).abs()).max((self.pos.z - origin.z).abs()) > MAX_CURSOR_DISTANCE
    }
}

pub fn add_cursors(cursors: &mut Vec<Cursor>, pos: BlockPos, mut charge: i32) {
    while charge > 0 {
        let current = charge.min(MAX_CHARGE);
        if cursors.len() < MAX_CURSORS {
            cursors.push(Cursor::new(pos, current));
        }
        charge -= current;
    }
}

pub fn pack_facings(facings: &[Direction]) -> i32 {
    facings.iter().fold(0, |packed, &direction| packed | 1 << Direction::ALL.iter().position(|&d| d == direction).unwrap_or(0))
}

/// `ChargeCursor.NON_CORNER_NEIGHBOURS` in `betweenClosed` order.
fn non_corner_neighbours() -> Vec<(i32, i32, i32)> {
    let mut out = Vec::with_capacity(18);
    for z in -1..=1 {
        for y in -1..=1 {
            for x in -1..=1 {
                if (x == 0 || y == 0 || z == 0) && (x, y, z) != (0, 0, 0) {
                    out.push((x, y, z));
                }
            }
        }
    }
    out
}

impl SculkPatch {
    fn parse(lib: &mut Library, json: &Value) -> Result<Self, String> {
        Ok(Self {
            charge_count: int(json, "charge_count")?,
            amount_per_charge: int(json, "amount_per_charge")?,
            spread_attempts: int(json, "spread_attempts")?,
            growth_rounds: int(json, "growth_rounds")?,
            spread_rounds: int(json, "spread_rounds")?,
            spreader: SculkSpreader::new(&lib.registries, true)?,
        })
    }

    fn can_spread_from(&self, ctx: &Ctx, origin: BlockPos) -> bool {
        let start = ctx.block(origin);
        if self.spreader.behaviour(ctx, start) != Behaviour::Default {
            return true;
        }
        if !ctx.is_air(start) && !(start == ctx.lib.blocks.water) {
            return false;
        }
        Direction::ALL.iter().any(|&d| ctx.registries().blocks.state(ctx.block(origin.relative(d, 1))).collision_full_block)
    }
}

impl SculkSpreader {
    pub fn new(r: &minecraftoss_core::Registries, world_generation: bool) -> Result<Self, String> {
        let s = |n: &str| r.blocks.parse_state(n);
        let (growth_spawn_cost, no_growth_radius, charge_decay_rate, additional_decay_rate) = if world_generation {
            (GROWTH_SPAWN_COST, NO_GROWTH_RADIUS, CHARGE_DECAY_RATE, ADDITIONAL_DECAY_RATE)
        } else {
            (LEVEL_GROWTH_SPAWN_COST, LEVEL_NO_GROWTH_RADIUS, LEVEL_CHARGE_DECAY_RATE, LEVEL_ADDITIONAL_DECAY_RATE)
        };
        let spread_tag = if world_generation { "minecraft:sculk_replaceable_world_gen" } else { "minecraft:sculk_replaceable" };
        Ok(Self {
            world_generation,
            growth_spawn_cost,
            no_growth_radius,
            charge_decay_rate,
            additional_decay_rate,
            sculk: s("minecraft:sculk")?,
            sculk_block: r.blocks.block_by_name("minecraft:sculk").ok_or("no sculk")?,
            vein: r.blocks.block_by_name("minecraft:sculk_vein").ok_or("no sculk vein")?,
            sensor: s("minecraft:sculk_sensor")?,
            shrieker: s(if world_generation { "minecraft:sculk_shrieker[can_summon=true]" } else { "minecraft:sculk_shrieker[can_summon=false]" })?,
            replaceable_spread: r.block_tags.require(spread_tag)?,
            replaceable: r.block_tags.require("minecraft:sculk_replaceable")?,
            growth_inhibitors: r.block_tags.require("minecraft:sculk_growth_inhibitors")?,
        })
    }

    fn vein_spreader(&self) -> Spreader {
        Spreader { block: self.vein, sculk: true, types: DEFAULT_ORDER }
    }

    fn same_space_spreader(&self) -> Spreader {
        Spreader { block: self.vein, sculk: true, types: &[super::multiface::SpreadType::SamePosition] }
    }

    fn behaviour(&self, ctx: &Ctx, state: BlockStateId) -> Behaviour {
        let block = ctx.registries().blocks.block_of(state);
        if block == self.sculk_block {
            Behaviour::Sculk
        } else if block == self.vein {
            Behaviour::Vein
        } else {
            Behaviour::Default
        }
    }

    fn faces(ctx: &Ctx, state: BlockStateId) -> Vec<Direction> {
        Direction::ALL.into_iter().filter(|&d| Spreader::has_face(ctx, state, d)).collect()
    }

    fn attempt_spread_vein(&self, ctx: &mut Ctx, behaviour: Behaviour, pos: BlockPos, state: BlockStateId, facings: &Option<Vec<Direction>>) -> bool {
        match behaviour {
            Behaviour::Sculk | Behaviour::Vein => self.vein_spreader().spread_all(ctx, state, pos, self.world_generation) > 0,
            Behaviour::Default => match facings {
                None => {
                    let current = ctx.block(pos);
                    self.same_space_spreader().spread_all(ctx, current, pos, self.world_generation) > 0
                }
                Some(faces) if !faces.is_empty() => {
                    if !ctx.is_air(state) && ctx.fluid(state) != FluidType::Water {
                        return false;
                    }
                    self.regrow(ctx, pos, state, faces)
                }
                Some(_) => self.vein_spreader().spread_all(ctx, state, pos, self.world_generation) > 0,
            },
        }
    }

    /// `SculkVeinBlock.regrow`.
    fn regrow(&self, ctx: &mut Ctx, pos: BlockPos, existing: BlockStateId, faces: &[Direction]) -> bool {
        let mut state = ctx.registries().blocks.block(self.vein).default_state();
        let mut any = false;
        for &face in faces {
            if can_attach_to(ctx, face, ctx.block(pos.relative(face, 1))) {
                state = ctx.with(state, face.name(), "true");
                any = true;
            }
        }
        if !any {
            return false;
        }
        if ctx.fluid(existing) != FluidType::Empty {
            state = ctx.with(state, "waterlogged", "true");
        }
        ctx.set_block_update(pos, state);
        true
    }

    /// `SculkVeinBlock.onDischarged`.
    fn vein_discharged(&self, ctx: &mut Ctx, mut state: BlockStateId, pos: BlockPos) {
        if ctx.registries().blocks.block_of(state) != self.vein {
            return;
        }
        for dir in Direction::ALL {
            if Spreader::has_face(ctx, state, dir) && ctx.registries().blocks.block_of(ctx.block(pos.relative(dir, 1))) == self.sculk_block {
                state = ctx.with(state, dir.name(), "false");
            }
        }
        if Self::faces(ctx, state).is_empty() {
            state = if ctx.fluid_at(pos) == FluidType::Empty { ctx.lib.blocks.air } else { ctx.lib.blocks.water };
        }
        ctx.set_block_update(pos, state);
    }

    fn discharged(&self, ctx: &mut Ctx, behaviour: Behaviour, state: BlockStateId, pos: BlockPos) {
        if behaviour == Behaviour::Vein {
            self.vein_discharged(ctx, state, pos);
        }
    }

    fn use_charge(&self, ctx: &mut Ctx, random: &mut WorldgenRandom, behaviour: Behaviour, cursor: &Cursor, origin: BlockPos, spread_veins: bool, effects: &mut SpreadEffects) -> i32 {
        match behaviour {
            Behaviour::Default => {
                if cursor.decay_delay > 0 {
                    cursor.charge
                } else {
                    0
                }
            }
            Behaviour::Vein => {
                if spread_veins && self.place_sculk(ctx, random, cursor.pos, effects) {
                    cursor.charge - 1
                } else if random.next_i32_bound(self.charge_decay_rate) == 0 {
                    (cursor.charge as f32 * 0.5).floor() as i32
                } else {
                    cursor.charge
                }
            }
            Behaviour::Sculk => {
                let charge = cursor.charge;
                if charge == 0 || random.next_i32_bound(self.charge_decay_rate) != 0 {
                    return charge;
                }
                let pos = cursor.pos;
                let close = pos.dist_sqr(origin) < f64::from(self.no_growth_radius * self.no_growth_radius);
                if !close && self.can_place_growth(ctx, pos) {
                    if random.next_i32_bound(self.growth_spawn_cost) < charge {
                        let growth = pos.above();
                        let shrieker = random.next_i32_bound(11) == 0;
                        let mut state = if shrieker { self.shrieker } else { self.sensor };
                        if ctx.property(state, "waterlogged").is_some() && ctx.fluid_at(growth) != FluidType::Empty {
                            state = ctx.with(state, "waterlogged", "true");
                        }
                        ctx.set_block_update(growth, state);
                        let placed = if shrieker { "minecraft:block.sculk_shrieker.place" } else { "minecraft:block.sculk_sensor.place" };
                        effects.sounds.push((pos, placed));
                    }
                    (charge - self.growth_spawn_cost).max(0)
                } else if random.next_i32_bound(self.additional_decay_rate) != 0 {
                    charge
                } else {
                    charge - if close { 1 } else { decay_penalty(pos, origin, charge, self.no_growth_radius) }
                }
            }
        }
    }

    fn can_place_growth(&self, ctx: &Ctx, pos: BlockPos) -> bool {
        let above = ctx.block(pos.above());
        if !(ctx.is_air(above) || above == ctx.lib.blocks.water) {
            return false;
        }
        let mut count = 0;
        for y in 0..=2 {
            for z in -4..=4 {
                for x in -4..=4 {
                    if ctx.in_tag(ctx.block(pos.offset(x, y, z)), self.growth_inhibitors) {
                        count += 1;
                        if count > 2 {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// `SculkVeinBlock.attemptPlaceSculk`.
    fn place_sculk(&self, ctx: &mut Ctx, random: &mut WorldgenRandom, pos: BlockPos, effects: &mut SpreadEffects) -> bool {
        let state = ctx.block(pos);
        let mut supports = Direction::ALL;
        shuffle(&mut supports, random);
        for support in supports {
            if !Spreader::has_face(ctx, state, support) {
                continue;
            }
            let support_pos = pos.relative(support, 1);
            if !ctx.in_tag(ctx.block(support_pos), self.replaceable_spread) {
                continue;
            }
            ctx.set_block_update(support_pos, self.sculk);
            effects.sounds.push((support_pos, SPREAD_SOUND));
            self.vein_spreader().spread_all(ctx, self.sculk, support_pos, self.world_generation);
            let skip = support.opposite();
            for dir in Direction::ALL {
                if dir == skip {
                    continue;
                }
                let vein_pos = support_pos.relative(dir, 1);
                let vein = ctx.block(vein_pos);
                if ctx.registries().blocks.block_of(vein) == self.vein {
                    self.vein_discharged(ctx, vein, vein_pos);
                }
            }
            return true;
        }
        false
    }

    fn has_substrate_access(&self, ctx: &Ctx, state: BlockStateId, pos: BlockPos) -> bool {
        if ctx.registries().blocks.block_of(state) != self.vein {
            return false;
        }
        Direction::ALL.iter().any(|&d| Spreader::has_face(ctx, state, d) && ctx.in_tag(ctx.block(pos.relative(d, 1)), self.replaceable))
    }

    fn movement_unobstructed(ctx: &Ctx, from: BlockPos, to: BlockPos) -> bool {
        if from.dist_manhattan(to) == 1 {
            return true;
        }
        let (dx, dy, dz) = (to.x - from.x, to.y - from.y, to.z - from.z);
        let dir = |axis, neg: bool| Direction::from_axis(axis, !neg);
        let x = dir(minecraftoss_core::pos::Axis::X, dx < 0);
        let y = dir(minecraftoss_core::pos::Axis::Y, dy < 0);
        let z = dir(minecraftoss_core::pos::Axis::Z, dz < 0);
        let open = |d: Direction| {
            let test = from.relative(d, 1);
            !ctx.registries().blocks.is_face_sturdy(ctx.block(test), d.opposite(), minecraftoss_core::SupportType::Full)
        };
        if dx == 0 {
            open(y) || open(z)
        } else if dy == 0 {
            open(x) || open(z)
        } else {
            open(x) || open(y)
        }
    }

    fn movement_pos(&self, ctx: &Ctx, random: &mut WorldgenRandom, pos: BlockPos, origin: BlockPos) -> Option<BlockPos> {
        let mut offsets = non_corner_neighbours();
        shuffle(&mut offsets, random);
        let mut found = pos;
        for (x, y, z) in offsets {
            let neighbour = pos.offset(x, y, z);
            let d2 = (origin.x - neighbour.x).pow(2) + (origin.z - neighbour.z).pow(2);
            if self.world_generation && d2 > 144 {
                continue;
            }
            let state = ctx.block(neighbour);
            if self.behaviour(ctx, state) != Behaviour::Default && Self::movement_unobstructed(ctx, pos, neighbour) {
                found = neighbour;
                if self.has_substrate_access(ctx, state, neighbour) {
                    break;
                }
            }
        }
        (found != pos).then_some(found)
    }

    fn update(&self, ctx: &mut Ctx, random: &mut WorldgenRandom, cursor: &mut Cursor, origin: BlockPos, spread_veins: bool, effects: &mut SpreadEffects) {
        if cursor.charge <= 0 {
            return;
        }
        if cursor.update_delay > 0 {
            cursor.update_delay -= 1;
            return;
        }
        let mut state = ctx.block(cursor.pos);
        let mut behaviour = self.behaviour(ctx, state);
        if spread_veins && self.attempt_spread_vein(ctx, behaviour, cursor.pos, state, &cursor.facings) {
            if behaviour != Behaviour::Sculk {
                state = ctx.block(cursor.pos);
                behaviour = self.behaviour(ctx, state);
            }
            effects.sounds.push((cursor.pos, SPREAD_SOUND));
        }
        cursor.charge = self.use_charge(ctx, random, behaviour, cursor, origin, spread_veins, effects);
        if cursor.charge <= 0 {
            self.discharged(ctx, behaviour, state, cursor.pos);
            return;
        }
        match self.movement_pos(ctx, random, cursor.pos, origin) {
            Some(transfer) => {
                self.discharged(ctx, behaviour, state, cursor.pos);
                cursor.pos = transfer;
                state = ctx.block(transfer);
            }
            None if self.world_generation => {
                self.discharged(ctx, behaviour, state, cursor.pos);
                cursor.charge = 0;
                return;
            }
            None => {}
        }
        if self.behaviour(ctx, state) != Behaviour::Default {
            cursor.facings = Some(if ctx.registries().blocks.block_of(state) == self.vein { Self::faces(ctx, state) } else { Vec::new() });
        }
        cursor.decay_delay = if behaviour == Behaviour::Default { (cursor.decay_delay - 1).max(0) } else { 1 };
        cursor.update_delay = 1;
    }

    pub fn shrieker(&self) -> BlockStateId {
        self.shrieker
    }

    pub fn update_cursors(&self, ctx: &mut Ctx, random: &mut WorldgenRandom, cursors: &mut Vec<Cursor>, origin: BlockPos, spread_veins: bool, effects: &mut SpreadEffects) {
        let mut kept: Vec<Cursor> = Vec::new();
        let mut first: Vec<(BlockPos, usize)> = Vec::new();
        let mut charges: Vec<(BlockPos, i32)> = Vec::new();
        for mut cursor in cursors.drain(..) {
            if cursor.unreasonable(origin) {
                continue;
            }
            self.update(ctx, random, &mut cursor, origin, spread_veins, effects);
            if cursor.charge <= 0 {
                effects.level_events.push((cursor.pos, 0));
                continue;
            }
            match charges.iter_mut().find(|(pos, _)| *pos == cursor.pos) {
                Some((_, charge)) => *charge += cursor.charge,
                None => charges.push((cursor.pos, cursor.charge)),
            }
            let Some(index) = first.iter().find(|(pos, _)| *pos == cursor.pos).map(|&(_, index)| index) else {
                first.push((cursor.pos, kept.len()));
                kept.push(cursor);
                continue;
            };
            if !self.world_generation && cursor.charge + kept[index].charge <= MAX_CHARGE {
                let existing = &mut kept[index];
                existing.charge += cursor.charge;
                existing.update_delay = existing.update_delay.min(cursor.update_delay);
                continue;
            }
            let lighter = cursor.charge < kept[index].charge;
            kept.push(cursor);
            if lighter {
                let at = kept.len() - 1;
                if let Some(entry) = first.iter_mut().find(|(pos, _)| *pos == kept[at].pos) {
                    entry.1 = at;
                }
            }
        }
        for (pos, charge) in charges {
            let facings = first.iter().find(|(at, _)| *at == pos).and_then(|&(_, index)| kept[index].facings.as_ref());
            if let Some(facings) = facings.filter(|_| charge > 0) {
                let size = ((f64::from(charge)).ln_1p() / 2.299999952316284) as i32 + 1;
                effects.level_events.push((pos, (size << 6) + pack_facings(facings)));
            }
        }
        *cursors = kept;
    }
}

fn decay_penalty(pos: BlockPos, origin: BlockPos, charge: i32, no_growth_radius: i32) -> i32 {
    let outer = (pos.dist_sqr(origin).sqrt() as f32 - no_growth_radius as f32).powi(2);
    let max_reach = ((24 - no_growth_radius) * (24 - no_growth_radius)) as f32;
    let factor = 1.0f32.min(outer / max_reach);
    1.max((charge as f32 * factor * 0.5) as i32)
}

impl Placeable for SculkPatch {
    fn place(&self, ctx: &mut Ctx, random: &mut WorldgenRandom, origin: BlockPos) -> bool {
        if !self.can_spread_from(ctx, origin) {
            return false;
        }
        let mut cursors: Vec<Cursor> = Vec::new();
        let mut effects = SpreadEffects::default();
        for round in 0..self.spread_rounds + self.growth_rounds {
            for _ in 0..self.charge_count {
                let mut charge = self.amount_per_charge;
                while charge > 0 {
                    let current = charge.min(1000);
                    if cursors.len() < MAX_CURSORS {
                        cursors.push(Cursor::new(origin, current));
                    }
                    charge -= current;
                }
            }
            let spread_veins = round < self.spread_rounds;
            for _ in 0..self.spread_attempts {
                if cursors.is_empty() {
                    continue;
                }
                let mut kept = Vec::with_capacity(cursors.len());
                for mut cursor in cursors.drain(..) {
                    let far = (cursor.pos.x - origin.x).abs().max((cursor.pos.y - origin.y).abs()).max((cursor.pos.z - origin.z).abs()) > 1024;
                    if far {
                        continue;
                    }
                    self.spreader.update(ctx, random, &mut cursor, origin, spread_veins, &mut effects);
                    if cursor.charge > 0 {
                        kept.push(cursor);
                    }
                }
                cursors = kept;
            }
            cursors.clear();
        }
        true
    }
}
