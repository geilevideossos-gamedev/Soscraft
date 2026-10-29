use std::collections::BTreeMap;

use glam::{DVec3, Mat4, Vec3};
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::rng::LegacyRandom;

use crate::scene::BlockPos;

pub const ITEM_FRAME: &str = "minecraft:item_frame";
pub const GLOW_ITEM_FRAME: &str = "minecraft:glow_item_frame";
pub const NUM_ROTATIONS: i32 = 8;
pub const DEPTH: f64 = 0.0625;
pub const WIDTH: f64 = 0.75;
pub const HANG_OFFSET: f64 = 0.46875;
pub const CHECK_INTERVAL: i32 = 100;
pub const TRACKING_RANGE: f64 = 160.0;
pub const DROP_STEP: f32 = 0.15;
pub const FRAME_TARGET: u64 = u64::MAX / 4;
pub const GLOW_FRAME_BRIGHTNESS: f32 = 5.0;
pub const FULL_BRIGHT: f32 = 15.0;
const ITEM_DEPTH: f32 = 0.4375;
const INVISIBLE_ITEM_DEPTH: f32 = 0.5;
const ITEM_SCALE: f32 = 0.5;
const MODEL_Z_OFFSET: f32 = 1.0 / 512.0;
const BLOCK_POS_RANGE: i64 = 16;
const BELOW_WORLD: f64 = 64.0;
const BORDER: f64 = 29_999_984.0;
const USE_REACH: f64 = 4.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameSound {
    Place,
    AddItem,
    RotateItem,
    RemoveItem,
    Break,
}

impl FrameSound {
    pub fn event(self, glow: bool) -> &'static str {
        match (self, glow) {
            (Self::Place, false) => "minecraft:entity.item_frame.place",
            (Self::AddItem, false) => "minecraft:entity.item_frame.add_item",
            (Self::RotateItem, false) => "minecraft:entity.item_frame.rotate_item",
            (Self::RemoveItem, false) => "minecraft:entity.item_frame.remove_item",
            (Self::Break, false) => "minecraft:entity.item_frame.break",
            (Self::Place, true) => "minecraft:entity.glow_item_frame.place",
            (Self::AddItem, true) => "minecraft:entity.glow_item_frame.add_item",
            (Self::RotateItem, true) => "minecraft:entity.glow_item_frame.rotate_item",
            (Self::RemoveItem, true) => "minecraft:entity.glow_item_frame.remove_item",
            (Self::Break, true) => "minecraft:entity.glow_item_frame.break",
        }
    }
}

pub fn step(facing: Direction) -> DVec3 {
    let (x, y, z) = facing.offset();
    DVec3::new(f64::from(x), f64::from(y), f64::from(z))
}

pub fn horizontal(facing: Direction) -> bool {
    !matches!(facing, Direction::Up | Direction::Down)
}

pub fn data_2d(facing: Direction) -> Option<i32> {
    match facing {
        Direction::South => Some(0),
        Direction::West => Some(1),
        Direction::North => Some(2),
        Direction::East => Some(3),
        Direction::Up | Direction::Down => None,
    }
}

pub fn facing_of_id(id: i64) -> Direction {
    Direction::ALL[(id % 6).unsigned_abs() as usize]
}

pub fn boxes_meet((a, b): (DVec3, DVec3), (c, d): (DVec3, DVec3)) -> bool {
    a.x < d.x && b.x > c.x && a.y < d.y && b.y > c.y && a.z < d.z && b.z > c.z
}

pub fn frame_item_id(glow: bool) -> &'static str {
    if glow { GLOW_ITEM_FRAME } else { ITEM_FRAME }
}

pub fn hanging_box(centre: DVec3, facing: Direction) -> (DVec3, DVec3) {
    let along = step(facing).abs();
    let size = DVec3::splat(WIDTH) + along * (DEPTH - WIDTH);
    (centre - size / 2.0, centre + size / 2.0)
}

pub trait FrameWorld {
    fn legacy_solid(&self, pos: BlockPos) -> bool;
    fn is_diode(&self, pos: BlockPos) -> bool;
    fn collides(&self, low: DVec3, high: DVec3) -> bool;
    fn inside_border(&self, low: DVec3, high: DVec3) -> bool {
        low.x > -BORDER && high.x < BORDER && low.z > -BORDER && high.z < BORDER
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemFrame {
    pub id: u64,
    pub glow: bool,
    pub pos: BlockPos,
    pub facing: Direction,
    pub item: Option<ItemStack>,
    pub rotation: i32,
    pub drop_chance: f32,
    pub invisible: bool,
    pub fixed: bool,
    pub invulnerable: bool,
    pub check_ticks: i32,
    pub tag: Tag,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrameAction {
    Use {
        held: Option<ItemStack>,
        infinite: bool,
    },
    Attack {
        infinite: bool,
    },
    Shot {
        infinite: bool,
    },
    Blast {
        infinite: bool,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameOutcome {
    pub handled: bool,
    pub consumed: bool,
    pub sounds: Vec<FrameSound>,
    pub drops: Vec<ItemStack>,
    pub removed: bool,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FrameView {
    pub id: u64,
    pub glow: bool,
    pub pos: BlockPos,
    pub facing: Direction,
    pub position: DVec3,
    pub item: Option<String>,
    pub potion: Option<u32>,
    pub look: String,
    pub glint: bool,
    pub rotation: i32,
    pub invisible: bool,
    pub fixed: bool,
}

fn doubles(tag: &Tag, key: &str) -> Option<DVec3> {
    let Some(Tag::List(values)) = tag.get(key) else {
        return None;
    };
    let value = |i: usize| values.get(i).and_then(Tag::as_f64);
    Some(DVec3::new(value(0)?, value(1)?, value(2)?))
}

fn flag(tag: &Tag, key: &str) -> bool {
    tag.get(key)
        .and_then(Tag::as_i64)
        .is_some_and(|value| value != 0)
}

fn saved_block_pos(tag: &Tag) -> Option<BlockPos> {
    match tag.get("block_pos")? {
        Tag::IntArray(values) if values.len() == 3 => Some((values[0], values[1], values[2])),
        _ => None,
    }
}

fn closer_than(a: BlockPos, b: BlockPos) -> bool {
    let d = |p: i32, q: i32| (i64::from(p) - i64::from(q)).pow(2);
    d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2) < BLOCK_POS_RANGE * BLOCK_POS_RANGE
}

pub fn stack_of_tag(tag: &Tag) -> Option<ItemStack> {
    let id = tag.get("id")?.as_str()?;
    let count = tag.get("count").map_or(Some(1), Tag::as_i64)?;
    if count <= 0 || id == "minecraft:air" {
        return None;
    }
    let components = tag
        .get("components")
        .and_then(crate::item_nbt::saved_components);
    let (id, components) = crate::item_nbt::from_disk(id, components);
    let mut stack = ItemStack::new(id, 1);
    stack.components = components.map(|components| crate::item_nbt::tag_json(&components));
    Some(stack)
}

impl ItemFrame {
    pub fn from_tag(tag: &Tag, id: u64) -> Option<Self> {
        let glow = match tag.get("id").and_then(Tag::as_str)? {
            ITEM_FRAME => false,
            GLOW_ITEM_FRAME => true,
            _ => return None,
        };
        let position = doubles(tag, "Pos")?;
        let containing = (
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        );
        let pos = saved_block_pos(tag)
            .filter(|&saved| closer_than(saved, containing))
            .unwrap_or(containing);
        Some(Self {
            id,
            glow,
            pos,
            facing: tag
                .get("Facing")
                .and_then(Tag::as_i64)
                .map_or(Direction::Down, facing_of_id),
            item: tag.get("Item").and_then(stack_of_tag),
            rotation: tag
                .get("ItemRotation")
                .and_then(Tag::as_i64)
                .map_or(0, |rotation| i32::from(rotation as i8) % NUM_ROTATIONS),
            drop_chance: tag
                .get("ItemDropChance")
                .and_then(Tag::as_f64)
                .map_or(1.0, |chance| chance as f32),
            invisible: flag(tag, "Invisible"),
            fixed: flag(tag, "Fixed"),
            invulnerable: flag(tag, "Invulnerable"),
            check_ticks: 0,
            tag: tag.clone(),
        })
    }

    pub fn to_tag(&self) -> Tag {
        let mut tag = self.tag.clone();
        if let Tag::Compound(map) = &mut tag {
            let (yaw, pitch) = self.yaw_pitch();
            map.insert(
                "Pos".to_owned(),
                Tag::List(self.position().to_array().map(Tag::Double).to_vec()),
            );
            map.insert(
                "Rotation".to_owned(),
                Tag::List(vec![Tag::Float(yaw), Tag::Float(pitch)]),
            );
            map.insert(
                "block_pos".to_owned(),
                Tag::IntArray(vec![self.pos.0, self.pos.1, self.pos.2]),
            );
            map.insert("Facing".to_owned(), Tag::Byte(self.facing.index() as i8));
            match self.item.as_ref().and_then(crate::item_nbt::stack_tag) {
                Some(item) => {
                    map.insert("Item".to_owned(), item);
                }
                None => {
                    map.remove("Item");
                }
            }
            map.insert("ItemRotation".to_owned(), Tag::Byte(self.rotation as i8));
            map.insert("ItemDropChance".to_owned(), Tag::Float(self.drop_chance));
            map.insert("Invisible".to_owned(), Tag::Byte(i8::from(self.invisible)));
            map.insert("Fixed".to_owned(), Tag::Byte(i8::from(self.fixed)));
        }
        tag
    }

    pub fn uuid(&self) -> Option<[i32; 4]> {
        match self.tag.get("UUID") {
            Some(Tag::IntArray(uuid)) => uuid.as_slice().try_into().ok(),
            _ => None,
        }
    }

    pub fn position(&self) -> DVec3 {
        let centre = DVec3::new(
            f64::from(self.pos.0) + 0.5,
            f64::from(self.pos.1) + 0.5,
            f64::from(self.pos.2) + 0.5,
        );
        centre - step(self.facing) * HANG_OFFSET
    }

    pub fn bounding_box(&self) -> (DVec3, DVec3) {
        hanging_box(self.position(), self.facing)
    }

    pub fn pop_box(&self) -> (DVec3, DVec3) {
        self.bounding_box()
    }

    pub fn yaw_pitch(&self) -> (f32, f32) {
        match data_2d(self.facing) {
            Some(data) => ((data * 90) as f32, 0.0),
            None => (
                0.0,
                if self.facing == Direction::Up {
                    -90.0
                } else {
                    90.0
                },
            ),
        }
    }

    pub fn support(&self) -> BlockPos {
        let (x, y, z) = self.facing.offset();
        (self.pos.0 - x, self.pos.1 - y, self.pos.2 - z)
    }

    pub fn supported(&self, world: &impl FrameWorld) -> bool {
        let (low, high) = self.pop_box();
        if world.collides(low, high) || !world.inside_border(low, high) {
            return false;
        }
        let support = self.support();
        world.legacy_solid(support) || (horizontal(self.facing) && world.is_diode(support))
    }

    pub fn analog_output(&self) -> i32 {
        if self.item.is_some() {
            self.rotation % NUM_ROTATIONS + 1
        } else {
            0
        }
    }

    pub fn drop_point(&self) -> DVec3 {
        let (x, _, z) = self.facing.offset();
        let along = |step: i32| f64::from(step as f32 * DROP_STEP);
        self.position() + DVec3::new(along(x), 0.0, along(z))
    }

    pub fn frame_item(&self) -> ItemStack {
        let mut stack = ItemStack::new(frame_item_id(self.glow), 1);
        if let Some(name) = self.tag.get("CustomName") {
            let mut components = serde_json::Map::new();
            components.insert(
                "minecraft:custom_name".to_owned(),
                crate::item_nbt::tag_json(name),
            );
            stack.components = Some(serde_json::Value::Object(components));
        }
        stack
    }

    pub fn view(&self) -> FrameView {
        FrameView {
            id: self.id,
            glow: self.glow,
            pos: self.pos,
            facing: self.facing,
            position: self.position(),
            item: self.item.as_ref().map(|stack| stack.id.clone()),
            potion: self
                .item
                .as_ref()
                .and_then(|stack| crate::potions::stack_color(stack.components.as_ref())),
            look: self
                .item
                .as_ref()
                .map(|stack| crate::item_look::ItemLook::of(stack.components.as_ref(), None).suffix())
                .unwrap_or_default(),
            glint: self.item.as_ref().is_some_and(crate::glint::has_glint),
            rotation: self.rotation,
            invisible: self.invisible,
            fixed: self.fixed,
        }
    }
}

pub struct Frames {
    pub list: Vec<ItemFrame>,
    next_id: u64,
    random: LegacyRandom,
    uuids: LegacyRandom,
}

impl Default for Frames {
    fn default() -> Self {
        Self {
            list: Vec::new(),
            next_id: 0,
            random: LegacyRandom::new(0x00F2_A3E5_17E3),
            uuids: LegacyRandom::new(crate::end_crystal::session_seed()),
        }
    }
}

impl Frames {
    pub fn add(&mut self, tag: &Tag) -> Option<u64> {
        let frame = ItemFrame::from_tag(tag, self.next_id + 1)?;
        self.next_id += 1;
        self.list.push(frame);
        Some(self.next_id)
    }

    pub fn fresh_tag(&mut self, glow: bool, pos: BlockPos, facing: Direction) -> Tag {
        let most = (self.uuids.next_long() & !0xF000) | 0x4000;
        let least = (self.uuids.next_long() & 0x3FFF_FFFF_FFFF_FFFF) | 0x8000_0000_0000_0000;
        let uuid = [
            (most >> 32) as i32,
            most as i32,
            (least >> 32) as i32,
            least as i32,
        ];
        let centre = DVec3::new(
            f64::from(pos.0) + 0.5,
            f64::from(pos.1) + 0.5,
            f64::from(pos.2) + 0.5,
        ) - step(facing) * HANG_OFFSET;
        Tag::Compound(BTreeMap::from([
            ("id".to_owned(), Tag::String(frame_item_id(glow).to_owned())),
            (
                "Pos".to_owned(),
                Tag::List(centre.to_array().map(Tag::Double).to_vec()),
            ),
            ("UUID".to_owned(), Tag::IntArray(uuid.to_vec())),
            (
                "block_pos".to_owned(),
                Tag::IntArray(vec![pos.0, pos.1, pos.2]),
            ),
            ("Facing".to_owned(), Tag::Byte(facing.index() as i8)),
        ]))
    }

    pub fn get(&self, id: u64) -> Option<&ItemFrame> {
        self.list.iter().find(|frame| frame.id == id)
    }

    pub fn coexists(&self, frame: &ItemFrame) -> bool {
        let pop = frame.pop_box();
        !self.list.iter().any(|other| {
            other.id != frame.id
                && other.facing == frame.facing
                && boxes_meet(other.bounding_box(), pop)
        })
    }

    pub fn survives(&self, frame: &ItemFrame, world: &impl FrameWorld) -> bool {
        frame.fixed || (frame.supported(world) && self.coexists(frame))
    }

    pub fn tick(&mut self, world: &impl FrameWorld, min_y: i32) -> Vec<ItemFrame> {
        let floor = f64::from(min_y) - BELOW_WORLD;
        self.list.retain(|frame| frame.position().y >= floor);
        let mut popped = Vec::new();
        let mut index = 0;
        while index < self.list.len() {
            let due = self.list[index].check_ticks >= CHECK_INTERVAL;
            self.list[index].check_ticks += 1;
            if due {
                self.list[index].check_ticks = 0;
                if !self.survives(&self.list[index], world) {
                    popped.push(self.list.remove(index));
                    continue;
                }
            }
            index += 1;
        }
        popped
    }

    pub fn drops(
        &mut self,
        frame: &ItemFrame,
        with_frame: bool,
        entity_drops: bool,
        infinite: bool,
    ) -> Vec<ItemStack> {
        if frame.fixed || !entity_drops || infinite {
            return Vec::new();
        }
        let mut drops = Vec::new();
        if with_frame {
            drops.push(frame.frame_item());
        }
        if let Some(item) = &frame.item {
            if self.random.next_float() < frame.drop_chance {
                drops.push(item.clone());
            }
        }
        drops
    }

    pub fn act(
        &mut self,
        id: u64,
        action: &FrameAction,
        entity_drops: bool,
    ) -> Option<(ItemFrame, FrameOutcome)> {
        let index = self.list.iter().position(|frame| frame.id == id)?;
        let mut outcome = FrameOutcome::default();
        let (infinite, explosion) = match action {
            FrameAction::Use { held, infinite } => {
                let frame = &mut self.list[index];
                if frame.fixed {
                    return Some((frame.clone(), outcome));
                }
                if frame.item.is_some() {
                    frame.rotation = (frame.rotation + 1) % NUM_ROTATIONS;
                    outcome.sounds.push(FrameSound::RotateItem);
                    outcome.handled = true;
                } else if let Some(held) = held.as_ref().filter(|stack| stack.count > 0) {
                    let mut one = held.clone();
                    one.count = 1;
                    frame.item = Some(one);
                    outcome.sounds.push(FrameSound::AddItem);
                    outcome.consumed = !infinite;
                    outcome.handled = true;
                }
                return Some((frame.clone(), outcome));
            }
            FrameAction::Attack { infinite } | FrameAction::Shot { infinite } => (*infinite, false),
            FrameAction::Blast { infinite } => (*infinite, true),
        };
        let frame = self.list[index].clone();
        if (frame.fixed || frame.invulnerable) && !infinite {
            return Some((frame, outcome));
        }
        outcome.handled = true;
        if !frame.fixed && !explosion && frame.item.is_some() {
            outcome.drops = self.drops(&frame, false, entity_drops, infinite);
            outcome.sounds.push(FrameSound::RemoveItem);
            self.list[index].item = None;
            return Some((self.list[index].clone(), outcome));
        }
        outcome.sounds.push(FrameSound::Break);
        outcome.drops = self.drops(&frame, true, entity_drops, infinite);
        outcome.removed = true;
        self.list.remove(index);
        Some((frame, outcome))
    }

    pub fn tags_where(&self, keep: impl Fn(DVec3) -> bool) -> Vec<Tag> {
        self.list
            .iter()
            .filter(|frame| keep(frame.position()))
            .map(ItemFrame::to_tag)
            .collect()
    }

    pub fn remove_where(&mut self, remove: impl Fn(DVec3) -> bool) {
        self.list.retain(|frame| !remove(frame.position()));
    }

    pub fn targets(&self) -> Vec<minecraftoss_entities::potion::PotionTarget> {
        self.list
            .iter()
            .map(|frame| {
                let (min, max) = frame.bounding_box();
                minecraftoss_entities::potion::PotionTarget {
                    id: FRAME_TARGET + frame.id,
                    min,
                    max,
                    owner: false,
                }
            })
            .collect()
    }

    pub fn views(&self) -> Vec<FrameView> {
        self.list.iter().map(ItemFrame::view).collect()
    }
}

pub fn frame_pose(view: &FrameView) -> Mat4 {
    let centre = Vec3::new(
        view.pos.0 as f32 + 0.5,
        view.pos.1 as f32 + 0.5,
        view.pos.2 as f32 + 0.5,
    );
    let (x_rot, y_rot) = match data_2d(view.facing) {
        Some(data) => (0.0, 180.0 - (data * 90) as f32),
        None => (-90.0 * view.facing.offset().1 as f32, 180.0),
    };
    Mat4::from_translation(centre)
        * Mat4::from_rotation_x(x_rot.to_radians())
        * Mat4::from_rotation_y(y_rot.to_radians())
}

pub fn model_pose(view: &FrameView) -> Mat4 {
    frame_pose(view)
        * Mat4::from_translation(Vec3::new(0.0, 0.0, -MODEL_Z_OFFSET))
        * Mat4::from_translation(Vec3::splat(-0.5))
}

pub fn item_pose(view: &FrameView) -> Mat4 {
    let depth = if view.invisible {
        INVISIBLE_ITEM_DEPTH
    } else {
        ITEM_DEPTH
    };
    let turn = view.rotation as f32 * 360.0 / NUM_ROTATIONS as f32;
    frame_pose(view)
        * Mat4::from_translation(Vec3::new(0.0, 0.0, depth))
        * Mat4::from_rotation_z(turn.to_radians())
        * Mat4::from_scale(Vec3::splat(ITEM_SCALE))
}

pub fn light(view: &FrameView, sky: f32, block: f32) -> ((f32, f32), (f32, f32)) {
    if view.glow {
        (
            (sky, block.max(GLOW_FRAME_BRIGHTNESS)),
            (FULL_BRIGHT, FULL_BRIGHT),
        )
    } else {
        ((sky, block), (sky, block))
    }
}

pub fn tracked(view: &FrameView, camera: DVec3) -> bool {
    let (dx, dz) = (view.position.x - camera.x, view.position.z - camera.z);
    dx * dx + dz * dz <= TRACKING_RANGE * TRACKING_RANGE
}

pub fn struck_frame(target: u64) -> Option<u64> {
    (FRAME_TARGET..crate::thrown::PLAYER_TARGET)
        .contains(&target)
        .then(|| target - FRAME_TARGET)
}

pub fn use_passes(view: &FrameView, holding: bool) -> bool {
    view.fixed || (view.item.is_none() && !holding)
}

pub fn on_ray<'a>(
    frames: impl IntoIterator<Item = &'a FrameView>,
    eye: DVec3,
    look: DVec3,
    reach: f64,
) -> Option<(u64, f64)> {
    frames
        .into_iter()
        .filter_map(|view| {
            crate::end_crystal::ray_box(eye, look, reach, hanging_box(view.position, view.facing))
                .map(|distance| (view.id, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

pub fn frame_item_use(id: u64) -> crate::bucket::ItemUse {
    crate::bucket::ItemUse {
        change: crate::bucket::Change::Frame(id),
        result: None,
        wear: 0,
        consumed: true,
        sounds: Vec::new(),
    }
}

pub fn hung_at(clicked: BlockPos, face: Direction) -> BlockPos {
    let (x, y, z) = face.offset();
    (clicked.0 + x, clicked.1 + y, clicked.2 + z)
}

pub fn frame_use(
    player: &minecraftoss_player::Player,
    world: &impl minecraftoss_player::World,
    held: &ItemStack,
) -> Option<crate::bucket::ItemUse> {
    let glow = match held.id.as_str() {
        ITEM_FRAME => false,
        GLOW_ITEM_FRAME => true,
        _ => return None,
    };
    let hit = player.target(world, USE_REACH)?;
    Some(crate::bucket::ItemUse {
        change: crate::bucket::Change::Server(
            hit.pos,
            crate::server::ItemOnBlock::HangFrame {
                face: Direction::ALL[hit.face as usize],
                glow,
            },
        ),
        result: None,
        wear: 0,
        consumed: true,
        sounds: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_tag(glow: bool, pos: BlockPos, facing: Direction) -> Tag {
        let mut frames = Frames::default();
        frames.fresh_tag(glow, pos, facing)
    }

    fn with(mut tag: Tag, entries: &[(&str, Tag)]) -> Tag {
        if let Tag::Compound(map) = &mut tag {
            for (key, value) in entries {
                map.insert((*key).to_owned(), value.clone());
            }
        }
        tag
    }

    fn item(id: &str) -> Tag {
        Tag::Compound(BTreeMap::from([
            ("id".to_owned(), Tag::String(id.to_owned())),
            ("count".to_owned(), Tag::Int(1)),
        ]))
    }

    struct Blocks {
        solid: Vec<BlockPos>,
        diodes: Vec<BlockPos>,
    }

    impl FrameWorld for Blocks {
        fn legacy_solid(&self, pos: BlockPos) -> bool {
            self.solid.contains(&pos)
        }

        fn is_diode(&self, pos: BlockPos) -> bool {
            self.diodes.contains(&pos)
        }

        fn collides(&self, low: DVec3, high: DVec3) -> bool {
            self.solid.iter().any(|&(x, y, z)| {
                let cell = DVec3::new(f64::from(x), f64::from(y), f64::from(z));
                boxes_meet((cell, cell + DVec3::ONE), (low, high))
            })
        }
    }

    #[test]
    fn a_frame_round_trips_its_vanilla_tag() {
        let tag = with(
            frame_tag(true, (3, 70, -5), Direction::East),
            &[
                ("Item", item("minecraft:elytra")),
                ("ItemRotation", Tag::Byte(5)),
                ("ItemDropChance", Tag::Float(0.25)),
                ("Invisible", Tag::Byte(1)),
                ("Fixed", Tag::Byte(1)),
                ("CustomName", Tag::String("Ship".to_owned())),
            ],
        );
        let mut frames = Frames::default();
        let id = frames.add(&tag).unwrap();
        let frame = frames.get(id).unwrap().clone();
        assert!(frame.glow);
        assert_eq!((frame.pos, frame.facing), ((3, 70, -5), Direction::East));
        assert_eq!(
            frame.item.as_ref().map(|s| (s.id.as_str(), s.count)),
            Some(("minecraft:elytra", 1))
        );
        assert_eq!((frame.rotation, frame.drop_chance), (5, 0.25));
        assert!(frame.invisible && frame.fixed && !frame.invulnerable);
        let saved = frame.to_tag();
        assert_eq!(saved.get("Facing"), Some(&Tag::Byte(5)));
        assert_eq!(
            saved.get("block_pos"),
            Some(&Tag::IntArray(vec![3, 70, -5]))
        );
        assert_eq!(saved.get("ItemRotation"), Some(&Tag::Byte(5)));
        assert_eq!(saved.get("ItemDropChance"), Some(&Tag::Float(0.25)));
        assert_eq!(saved.get("Invisible"), Some(&Tag::Byte(1)));
        assert_eq!(saved.get("Fixed"), Some(&Tag::Byte(1)));
        assert_eq!(
            saved.get("CustomName"),
            Some(&Tag::String("Ship".to_owned()))
        );
        assert_eq!(saved.get("UUID"), tag.get("UUID"));
        assert_eq!(
            saved
                .get("Item")
                .and_then(|i| i.get("id"))
                .and_then(Tag::as_str),
            Some("minecraft:elytra")
        );
        assert_eq!(
            saved.get("Rotation"),
            Some(&Tag::List(vec![Tag::Float(270.0), Tag::Float(0.0)]))
        );
        let mut again = Frames::default();
        let back = again.add(&saved).unwrap();
        let mut reloaded = again.get(back).unwrap().clone();
        reloaded.id = frame.id;
        reloaded.tag = frame.tag.clone();
        assert_eq!(reloaded, frame);
        let mut empty = frame.clone();
        empty.item = None;
        assert_eq!(
            empty.to_tag().get("Item"),
            None,
            "an empty frame saves no item"
        );
        assert_eq!(
            frame.frame_item().components,
            Some(serde_json::json!({"minecraft:custom_name": "Ship"}))
        );
        let none = Tag::Compound(BTreeMap::from([(
            "id".to_owned(),
            Tag::String("minecraft:painting".to_owned()),
        )]));
        assert_eq!(Frames::default().add(&none), None);
    }

    #[test]
    fn a_gun_in_a_frame_is_saved_as_its_carrier_and_comes_back() {
        let mut frames = Frames::default();
        let id = frames
            .add(&frame_tag(false, (1, 64, 1), Direction::North))
            .unwrap();
        let mut frame = frames.get(id).unwrap().clone();
        let mut gun = ItemStack::new("iw4l:m4", 1);
        gun.components = Some(serde_json::json!({
            "minecraft:custom_data": {"iw4l": {"attachments": ["acog"], "clip": 30}}
        }));
        frame.item = Some(gun.clone());
        let saved = frame.to_tag();
        assert_eq!(
            saved
                .get("Item")
                .and_then(|i| i.get("id"))
                .and_then(Tag::as_str),
            Some("minecraft:carrot_on_a_stick")
        );
        let mut again = Frames::default();
        let back = again.add(&saved).unwrap();
        let loaded = again.get(back).unwrap().item.clone().unwrap();
        assert_eq!((loaded.id, loaded.components), (gun.id, gun.components));
    }

    #[test]
    fn the_box_sits_against_the_support_for_every_facing() {
        for facing in Direction::ALL {
            let mut frames = Frames::default();
            frames.add(&frame_tag(false, (10, 64, -3), facing)).unwrap();
            let frame = &frames.list[0];
            let (low, high) = frame.bounding_box();
            let size = high - low;
            let along = step(facing);
            for axis in 0..3 {
                let expected = if along[axis] != 0.0 { DEPTH } else { WIDTH };
                assert!(
                    (size[axis] - expected).abs() < 1e-12,
                    "{facing:?} axis {axis}: {size}"
                );
            }
            let support = frame.support();
            let (sx, sy, sz) = facing.offset();
            assert_eq!(support, (10 - sx, 64 - sy, -3 - sz));
            let face = DVec3::new(10.5, 64.5, -2.5) - along * 0.5;
            let back = (frame.position() - along * DEPTH / 2.0 - face).dot(along);
            assert!(back.abs() < 1e-12, "{facing:?} lies on the support's face");
            let (yaw, pitch) = frame.yaw_pitch();
            match facing {
                Direction::Up => assert_eq!((yaw, pitch), (0.0, -90.0)),
                Direction::Down => assert_eq!((yaw, pitch), (0.0, 90.0)),
                _ => assert_eq!((yaw, pitch), ((data_2d(facing).unwrap() * 90) as f32, 0.0)),
            }
            let point = frame.drop_point() - frame.position();
            let vanilla = |step: f64| f64::from(step as f32 * 0.15f32);
            let expected = DVec3::new(vanilla(along.x), 0.0, vanilla(along.z));
            assert!(point.abs_diff_eq(expected, 1e-12), "{facing:?}: {point}");
        }
    }

    #[test]
    fn defaults_fill_missing_keys() {
        let tag = Tag::Compound(BTreeMap::from([
            ("id".to_owned(), Tag::String(ITEM_FRAME.to_owned())),
            (
                "Pos".to_owned(),
                Tag::List(vec![
                    Tag::Double(1.5),
                    Tag::Double(65.03125),
                    Tag::Double(-0.5),
                ]),
            ),
        ]));
        let mut frames = Frames::default();
        frames.add(&tag).unwrap();
        let frame = &frames.list[0];
        assert_eq!(frame.facing, Direction::Down);
        assert_eq!(frame.pos, (1, 65, -1));
        assert_eq!(frame.item, None);
        assert_eq!((frame.rotation, frame.drop_chance), (0, 1.0));
        assert!(!frame.glow && !frame.invisible && !frame.fixed && !frame.invulnerable);
        assert_eq!(facing_of_id(-1), Direction::Up);
        assert_eq!(facing_of_id(8), Direction::North);
        let negative = with(tag.clone(), &[("ItemRotation", Tag::Byte(-3))]);
        frames.add(&negative).unwrap();
        assert_eq!(
            frames.list[1].rotation, -3,
            "Java's remainder keeps the sign"
        );
        let air = with(tag, &[("Item", item("minecraft:air"))]);
        frames.add(&air).unwrap();
        assert_eq!(frames.list[2].item, None);
    }

    #[test]
    fn a_far_block_pos_is_ignored() {
        let tag = frame_tag(false, (0, 64, 0), Direction::North);
        let near = with(
            tag.clone(),
            &[("block_pos", Tag::IntArray(vec![0, 64, 15]))],
        );
        let far = with(tag, &[("block_pos", Tag::IntArray(vec![0, 64, 16]))]);
        let mut frames = Frames::default();
        frames.add(&near).unwrap();
        frames.add(&far).unwrap();
        assert_eq!(frames.list[0].pos, (0, 64, 15));
        assert_eq!(
            frames.list[1].pos,
            (0, 64, 0),
            "the block holding its position"
        );
    }

    #[test]
    fn frames_check_every_101_ticks() {
        let mut frames = Frames::default();
        frames
            .add(&frame_tag(false, (0, 64, 0), Direction::North))
            .unwrap();
        let nothing = Blocks {
            solid: Vec::new(),
            diodes: Vec::new(),
        };
        for _ in 0..100 {
            assert!(frames.tick(&nothing, -64).is_empty());
        }
        let popped = frames.tick(&nothing, -64);
        assert_eq!(popped.len(), 1, "the 101st tick checks");
        assert!(frames.list.is_empty());
        let wall = Blocks {
            solid: vec![(0, 64, 1)],
            diodes: Vec::new(),
        };
        frames
            .add(&frame_tag(false, (0, 64, 0), Direction::North))
            .unwrap();
        for _ in 0..101 {
            assert!(frames.tick(&wall, -64).is_empty());
        }
        assert_eq!(frames.list[0].check_ticks, 0);
        for _ in 0..100 {
            assert!(frames.tick(&nothing, -64).is_empty());
        }
        assert_eq!(
            frames.tick(&nothing, -64).len(),
            1,
            "then every 101 ticks again"
        );
        frames
            .add(&frame_tag(false, (0, -129, 0), Direction::North))
            .unwrap();
        frames.tick(&wall, -64);
        assert!(frames.list.is_empty(), "below the world it is gone at once");
    }

    #[test]
    fn support_needs_a_solid_block_or_a_diode_on_a_wall() {
        let frames = Frames::default();
        let wall = ItemFrame::from_tag(&frame_tag(false, (0, 64, 0), Direction::North), 1).unwrap();
        let floor = ItemFrame::from_tag(&frame_tag(false, (0, 64, 0), Direction::Up), 2).unwrap();
        let solid = Blocks {
            solid: vec![(0, 64, 1), (0, 63, 0)],
            diodes: Vec::new(),
        };
        assert!(frames.survives(&wall, &solid) && frames.survives(&floor, &solid));
        let diodes = Blocks {
            solid: Vec::new(),
            diodes: vec![(0, 64, 1), (0, 63, 0)],
        };
        assert!(
            frames.survives(&wall, &diodes),
            "a repeater holds a wall frame"
        );
        assert!(!frames.survives(&floor, &diodes), "but not a floor frame");
        let none = Blocks {
            solid: Vec::new(),
            diodes: Vec::new(),
        };
        assert!(!frames.survives(&wall, &none));
    }

    #[test]
    fn a_block_inside_the_frame_pops_it() {
        let frames = Frames::default();
        let wall = ItemFrame::from_tag(&frame_tag(false, (0, 64, 0), Direction::North), 1).unwrap();
        let filled = Blocks {
            solid: vec![(0, 64, 1), (0, 64, 0)],
            diodes: Vec::new(),
        };
        assert!(!frames.survives(&wall, &filled));
        let far = ItemFrame::from_tag(&frame_tag(false, (29_999_984, 64, 0), Direction::North), 3)
            .unwrap();
        let border = Blocks {
            solid: vec![(29_999_984, 64, 1)],
            diodes: Vec::new(),
        };
        assert!(!frames.survives(&far, &border), "past the world border");
    }

    #[test]
    fn a_fixed_frame_never_pops() {
        let mut frames = Frames::default();
        let tag = with(
            frame_tag(false, (0, 64, 0), Direction::North),
            &[("Fixed", Tag::Byte(1))],
        );
        frames.add(&tag).unwrap();
        let filled = Blocks {
            solid: vec![(0, 64, 0)],
            diodes: Vec::new(),
        };
        for _ in 0..303 {
            assert!(frames.tick(&filled, -64).is_empty());
        }
        let fixed = frames.list[0].clone();
        assert!(
            frames.drops(&fixed, true, true, false).is_empty(),
            "nor drops anything"
        );
    }

    #[test]
    fn two_frames_on_one_face_overlap_but_a_floor_frame_shares_the_cell() {
        let mut frames = Frames::default();
        frames
            .add(&frame_tag(false, (0, 64, 0), Direction::North))
            .unwrap();
        let again =
            ItemFrame::from_tag(&frame_tag(false, (0, 64, 0), Direction::North), 9).unwrap();
        let floor = ItemFrame::from_tag(&frame_tag(false, (0, 64, 0), Direction::Up), 10).unwrap();
        let beside =
            ItemFrame::from_tag(&frame_tag(false, (1, 64, 0), Direction::North), 11).unwrap();
        assert!(!frames.coexists(&again));
        assert!(frames.coexists(&floor));
        assert!(frames.coexists(&beside));
    }

    #[test]
    fn drops_follow_the_rules_and_the_drop_chance() {
        let mut frames = Frames::default();
        let tag = with(
            frame_tag(true, (0, 64, 0), Direction::North),
            &[("Item", item("minecraft:carrot"))],
        );
        frames.add(&tag).unwrap();
        let frame = frames.list[0].clone();
        let ids = |stacks: Vec<ItemStack>| stacks.into_iter().map(|s| s.id).collect::<Vec<_>>();
        assert_eq!(
            ids(frames.drops(&frame, true, true, false)),
            [GLOW_ITEM_FRAME, "minecraft:carrot"]
        );
        assert_eq!(
            ids(frames.drops(&frame, false, true, false)),
            ["minecraft:carrot"]
        );
        assert!(
            frames.drops(&frame, true, false, false).is_empty(),
            "entity_drops off"
        );
        assert!(
            frames.drops(&frame, true, true, true).is_empty(),
            "creative"
        );
        let mut never = frame.clone();
        never.drop_chance = 0.0;
        assert_eq!(
            ids(frames.drops(&never, true, true, false)),
            [GLOW_ITEM_FRAME]
        );
    }

    fn view_of(facing: Direction, rotation: i32, invisible: bool, glow: bool) -> FrameView {
        let mut frame = ItemFrame::from_tag(&frame_tag(glow, (4, 70, -9), facing), 1).unwrap();
        frame.rotation = rotation;
        frame.invisible = invisible;
        frame.item = Some(ItemStack::new("minecraft:elytra", 1));
        frame.view()
    }

    #[test]
    fn a_framed_potion_view_carries_its_colour() {
        let mut frame = ItemFrame::from_tag(&frame_tag(false, (4, 70, -9), Direction::North), 1).unwrap();
        let mut potion = ItemStack::new("minecraft:potion", 1);
        potion.components = Some(serde_json::json!({"minecraft:potion_contents": {"potion": "minecraft:night_vision"}}));
        frame.item = Some(potion);
        let view = frame.view();
        assert_eq!(view.item.as_deref(), Some("minecraft:potion"));
        assert_eq!(view.potion, Some(0xC2FF66));
        frame.item = Some(ItemStack::new("minecraft:potion", 1));
        assert_eq!(frame.view().potion, None);
        assert_eq!(view_of(Direction::North, 0, false, false).potion, None);
    }

    #[test]
    fn a_framed_ominous_banner_or_loaded_crossbow_view_carries_its_look() {
        let mut frame = ItemFrame::from_tag(&frame_tag(false, (4, 70, -9), Direction::North), 1).unwrap();
        assert_eq!(frame.view().look, "");
        let banner = stack_of_tag(&minecraftoss_world::natural_spawner::spawning::ominous_banner()).unwrap();
        frame.item = Some(banner.clone());
        let view = frame.view();
        assert_eq!(view.item.as_deref(), Some("minecraft:white_banner"));
        let look = crate::item_look::ItemLook::parse(&view.look);
        assert_eq!(look, crate::item_look::ItemLook::of(banner.components.as_ref(), None));
        assert_eq!(look.layers.len(), 8);
        assert_eq!(format!("{}{}", view.item.unwrap(), view.look), crate::item_tint::of_stack(&banner));
        let mut crossbow = ItemStack::new("minecraft:crossbow", 1);
        crossbow.components = Some(serde_json::json!({"minecraft:charged_projectiles": [{"id": "minecraft:arrow"}]}));
        frame.item = Some(crossbow);
        assert_eq!(frame.view().look, "|charge=arrow");
        assert_eq!(view_of(Direction::North, 0, false, false).look, "");
    }

    #[test]
    fn a_framed_banner_draws_its_pattern_layers() {
        let (_temp, packs) = crate::item_icons::tests::banner_pack();
        let textures = ["banner_base", "base", "stripe"].map(|name| crate::pack::ResourceId::parse(&format!("minecraft:entity/banner/{name}")).unwrap());
        let atlas = crate::mesh::make_atlas(&packs, textures.to_vec()).unwrap();
        let light = crate::lighting::SkyLight::build(&crate::scene::HandcraftedScene::new());
        let mut frame = ItemFrame::from_tag(&frame_tag(false, (0, 1, 0), Direction::North), 1).unwrap();
        frame.invisible = true;
        let mut visuals = crate::mesh::ItemVisuals::default();
        let mut faces = |stack: ItemStack| {
            frame.item = Some(stack);
            let mut mesh = crate::mesh::ChunkMesh::default();
            visuals.append_frames(&mut mesh, &[frame.view()], &packs, &atlas, &light).unwrap();
            mesh.faces
        };
        let mut banner = ItemStack::new("minecraft:white_banner", 1);
        assert_eq!(faces(banner.clone()), 18, "pole, bar and flag");
        banner.components = Some(serde_json::json!({"minecraft:banner_patterns": [{"pattern": "minecraft:stripe", "color": "red"}]}));
        assert_eq!(faces(banner), 24, "and one layer on the flag");
    }

    #[test]
    fn frame_pose_puts_the_back_against_the_wall_for_all_six_facings() {
        for facing in Direction::ALL {
            let view = view_of(facing, 0, false, false);
            let pose = model_pose(&view);
            let back = pose.transform_point3(Vec3::new(0.5, 0.5, 15.75 / 16.0));
            let along = step(facing).as_vec3();
            let support_face = Vec3::new(4.5, 70.5, -8.5) - along * 0.5;
            let expected = support_face + along * (DEPTH as f32 / 4.0 + MODEL_Z_OFFSET);
            assert!(
                back.abs_diff_eq(expected, 1e-5),
                "{facing:?}: {back} vs {expected}"
            );
            let outward = pose.transform_vector3(-Vec3::Z).normalize();
            assert!(
                outward.abs_diff_eq(along, 1e-5),
                "{facing:?} faces out: {outward}"
            );
            let item = item_pose(&view).transform_point3(Vec3::ZERO);
            let expected = Vec3::new(4.5, 70.5, -8.5) - along * ITEM_DEPTH;
            assert!(
                item.abs_diff_eq(expected, 1e-5),
                "{facing:?}: the item lies on the back"
            );
        }
    }

    #[test]
    fn the_item_turns_45_degrees_per_rotation_about_the_facing() {
        for facing in Direction::ALL {
            let along = step(facing).as_vec3();
            let start = item_pose(&view_of(facing, 0, false, false));
            let first = start.transform_vector3(Vec3::X).normalize();
            for rotation in 1..NUM_ROTATIONS {
                let turned = item_pose(&view_of(facing, rotation, false, false));
                let now = turned.transform_vector3(Vec3::X).normalize();
                let angle = first.angle_between(now).to_degrees();
                let expected = 45.0 * rotation.min(NUM_ROTATIONS - rotation) as f32;
                assert!(
                    (angle - expected).abs() < 1e-3,
                    "{facing:?} {rotation}: {angle}"
                );
                assert!(
                    now.dot(along).abs() < 1e-5,
                    "it turns in the plane of the frame"
                );
                let scale = turned.transform_vector3(Vec3::X).length();
                assert!((scale - 0.5).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn an_invisible_frame_moves_the_item_to_the_wall() {
        let view = view_of(Direction::North, 0, true, false);
        let item = item_pose(&view).transform_point3(Vec3::ZERO);
        assert!(item.abs_diff_eq(Vec3::new(4.5, 70.5, -8.0), 1e-5), "{item}");
    }

    #[test]
    fn a_glow_frame_raises_its_block_light_to_five_and_lights_its_item_fully() {
        let glow = view_of(Direction::North, 0, false, true);
        assert_eq!(light(&glow, 3.0, 2.0), ((3.0, 5.0), (15.0, 15.0)));
        assert_eq!(light(&glow, 3.0, 9.0), ((3.0, 9.0), (15.0, 15.0)));
        let plain = view_of(Direction::North, 0, false, false);
        assert_eq!(light(&plain, 3.0, 2.0), ((3.0, 2.0), (3.0, 2.0)));
    }

    #[test]
    fn frames_are_drawn_within_ten_chunks() {
        let view = view_of(Direction::North, 0, false, false);
        assert!(tracked(&view, view.position + DVec3::new(159.0, 80.0, 0.0)));
        assert!(!tracked(
            &view,
            view.position + DVec3::new(120.0, 0.0, 120.0)
        ));
    }

    #[test]
    fn frames_mesh_the_pack_model_and_their_item() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = crate::pack::PackStack::open(vec![pack]).unwrap();
        let textures = [
            "block/item_frame",
            "block/glow_item_frame",
            "block/birch_planks",
            "item/elytra",
        ]
        .map(|path| crate::pack::ResourceId::parse(&format!("minecraft:{path}")).unwrap());
        let atlas = crate::mesh::make_atlas(&packs, textures.to_vec()).unwrap();
        let light = crate::lighting::SkyLight::build(&crate::scene::HandcraftedScene::default());
        let mut visuals = crate::mesh::ItemVisuals::default();
        let mut empty = view_of(Direction::North, 0, false, false);
        empty.item = None;
        let mut frame = crate::mesh::ChunkMesh::default();
        visuals
            .append_frames(&mut frame, &[empty.clone()], &packs, &atlas, &light)
            .unwrap();
        assert!(frame.faces > 0, "the frame model");
        let uv_in = |mesh: &crate::mesh::ChunkMesh, texture: &crate::pack::ResourceId| {
            let [u0, v0, u1, v1] = atlas.region(texture);
            mesh.vertices
                .iter()
                .any(|v| (u0..=u1).contains(&v.uv[0]) && (v0..=v1).contains(&v.uv[1]))
        };
        assert!(uv_in(&frame, &textures[0]) && uv_in(&frame, &textures[2]));
        assert!(!uv_in(&frame, &textures[1]));
        let mut full = crate::mesh::ChunkMesh::default();
        visuals
            .append_frames(
                &mut full,
                &[view_of(Direction::North, 0, false, false)],
                &packs,
                &atlas,
                &light,
            )
            .unwrap();
        assert!(full.faces > frame.faces, "the elytra adds its own faces");
        assert!(uv_in(&full, &textures[3]));
        let mut glowing = crate::mesh::ChunkMesh::default();
        visuals
            .append_frames(
                &mut glowing,
                &[view_of(Direction::Up, 0, false, true)],
                &packs,
                &atlas,
                &light,
            )
            .unwrap();
        assert!(uv_in(&glowing, &textures[1]));
        assert!(
            glowing.vertices.iter().any(|v| v.block_light == 15.0),
            "the item of a glow frame is full bright"
        );
        let mut hidden = crate::mesh::ChunkMesh::default();
        let mut invisible = view_of(Direction::North, 0, true, false);
        visuals
            .append_frames(&mut hidden, &[invisible.clone()], &packs, &atlas, &light)
            .unwrap();
        assert_eq!(
            hidden.faces,
            full.faces - frame.faces,
            "an invisible frame shows only its item"
        );
        invisible.glint = true;
        let mut shiny = crate::mesh::ChunkMesh::default();
        visuals
            .append_frames(&mut shiny, &[invisible], &packs, &atlas, &light)
            .unwrap();
        assert_eq!(shiny.glint, vec![0..shiny.vertices.len() as u32]);
    }

    struct One(BlockPos, minecraftoss_player::Block);

    impl minecraftoss_player::World for One {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            (pos == self.0).then(|| self.1.clone())
        }

        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
    }

    #[test]
    fn frames_hang_on_all_six_faces_with_vanilla_rotations() {
        let stone = minecraftoss_player::Block::new("minecraft:stone");
        let mut player = minecraftoss_player::Player::new(DVec3::new(0.5, 3.0, 0.5));
        player.pitch = 90.0;
        let floor = One((0, 1, 0), stone.clone());
        let used =
            frame_use(&player, &floor, &ItemStack::new(ITEM_FRAME, 4)).expect("aimed at the floor");
        let up = crate::server::ItemOnBlock::HangFrame {
            face: Direction::Up,
            glow: false,
        };
        assert_eq!(used.change, crate::bucket::Change::Server((0, 1, 0), up));
        assert!(used.consumed && used.result.is_none() && used.sounds.is_empty());
        let mut facing_north = minecraftoss_player::Player::new(DVec3::new(0.5, 0.0, 2.5));
        facing_north.yaw = 180.0;
        let wall = One((0, 1, 0), stone);
        let used = frame_use(&facing_north, &wall, &ItemStack::new(GLOW_ITEM_FRAME, 1))
            .expect("aimed at the wall");
        let south = crate::server::ItemOnBlock::HangFrame {
            face: Direction::South,
            glow: true,
        };
        assert_eq!(used.change, crate::bucket::Change::Server((0, 1, 0), south));
        assert_eq!(
            frame_use(
                &facing_north,
                &wall,
                &ItemStack::new("minecraft:painting", 1)
            ),
            None
        );
        let expected = [
            (Direction::Down, (0.0, 90.0)),
            (Direction::Up, (0.0, -90.0)),
            (Direction::North, (180.0, 0.0)),
            (Direction::South, (0.0, 0.0)),
            (Direction::West, (90.0, 0.0)),
            (Direction::East, (270.0, 0.0)),
        ];
        for (face, rotation) in expected {
            let pos = hung_at((5, 64, 5), face);
            let frame = ItemFrame::from_tag(&frame_tag(false, pos, face), 1).unwrap();
            assert_eq!(
                frame.support(),
                (5, 64, 5),
                "{face:?} hangs on the clicked block"
            );
            assert_eq!(frame.yaw_pitch(), rotation, "{face:?}");
        }
    }

    fn filled(glow: bool, item_id: Option<&str>) -> (Frames, u64) {
        let mut frames = Frames::default();
        let mut tag = frame_tag(glow, (0, 64, 0), Direction::North);
        if let Some(item_id) = item_id {
            tag = with(tag, &[("Item", item(item_id))]);
        }
        let id = frames.add(&tag).unwrap();
        (frames, id)
    }

    fn held(id: &str, count: u8) -> FrameAction {
        FrameAction::Use {
            held: Some(ItemStack::new(id, count)),
            infinite: false,
        }
    }

    #[test]
    fn an_empty_frame_takes_one_item_and_a_full_frame_turns_it() {
        let (mut frames, id) = filled(false, None);
        let empty_hand = FrameAction::Use {
            held: None,
            infinite: false,
        };
        let (_, passed) = frames.act(id, &empty_hand, true).unwrap();
        assert_eq!(passed, FrameOutcome::default(), "an empty hand passes");
        let (frame, took) = frames.act(id, &held("minecraft:compass", 5), true).unwrap();
        assert!(took.handled && took.consumed && !took.removed);
        assert_eq!(took.sounds, [FrameSound::AddItem]);
        assert_eq!(
            frame.item.as_ref().map(|s| (s.id.as_str(), s.count)),
            Some(("minecraft:compass", 1))
        );
        let (frame, turned) = frames.act(id, &held("minecraft:stick", 1), true).unwrap();
        assert!(turned.handled && !turned.consumed);
        assert_eq!(turned.sounds, [FrameSound::RotateItem]);
        assert_eq!(
            (frame.rotation, frame.item.unwrap().id.as_str()),
            (1, "minecraft:compass")
        );
        let (_, turned) = frames.act(id, &empty_hand, true).unwrap();
        assert!(turned.handled, "an empty hand turns a full frame too");
        let (mut creative, id) = filled(false, None);
        let infinite = FrameAction::Use {
            held: Some(ItemStack::new("minecraft:compass", 1)),
            infinite: true,
        };
        let (frame, took) = creative.act(id, &infinite, true).unwrap();
        assert!(
            took.handled && !took.consumed && frame.item.is_some(),
            "creative keeps its stack"
        );
        assert_eq!(Frames::default().act(9, &empty_hand, true), None);
    }

    #[test]
    fn eight_turns_bring_the_item_back() {
        let (mut frames, id) = filled(false, Some("minecraft:arrow"));
        let rotations: Vec<i32> = (0..8)
            .map(|_| {
                frames
                    .act(id, &held("minecraft:stick", 1), true)
                    .unwrap()
                    .0
                    .rotation
            })
            .collect();
        assert_eq!(rotations, [1, 2, 3, 4, 5, 6, 7, 0]);
    }

    #[test]
    fn a_fixed_frame_ignores_use_and_survival_hits_but_not_creative() {
        let (mut frames, id) = filled(false, Some("minecraft:elytra"));
        frames.list[0].fixed = true;
        for action in [
            held("minecraft:stick", 1),
            FrameAction::Attack { infinite: false },
            FrameAction::Shot { infinite: false },
            FrameAction::Blast { infinite: false },
        ] {
            let (frame, outcome) = frames.act(id, &action, true).unwrap();
            assert_eq!(outcome, FrameOutcome::default(), "{action:?}");
            assert_eq!((frame.rotation, frame.item.is_some()), (0, true));
        }
        let (_, outcome) = frames
            .act(id, &FrameAction::Attack { infinite: true }, true)
            .unwrap();
        assert!(
            outcome.removed && outcome.drops.is_empty(),
            "creative breaks it outright, with nothing dropped"
        );
        assert_eq!(outcome.sounds, [FrameSound::Break]);
        let (mut guarded, id) = filled(false, Some("minecraft:elytra"));
        guarded.list[0].invulnerable = true;
        let (_, outcome) = guarded
            .act(id, &FrameAction::Attack { infinite: false }, true)
            .unwrap();
        assert_eq!(
            outcome,
            FrameOutcome::default(),
            "an invulnerable frame ignores survival hits"
        );
        let (_, outcome) = guarded
            .act(id, &FrameAction::Attack { infinite: true }, true)
            .unwrap();
        assert_eq!(outcome.sounds, [FrameSound::RemoveItem]);
    }

    #[test]
    fn a_hit_takes_the_item_out_and_a_second_hit_breaks_the_frame() {
        for hit in [
            FrameAction::Attack { infinite: false },
            FrameAction::Shot { infinite: false },
        ] {
            let (mut frames, id) = filled(false, Some("minecraft:elytra"));
            let (frame, first) = frames.act(id, &hit, true).unwrap();
            assert!(first.handled && !first.removed && frame.item.is_none());
            assert_eq!(first.sounds, [FrameSound::RemoveItem]);
            assert_eq!(
                first
                    .drops
                    .iter()
                    .map(|s| s.id.as_str())
                    .collect::<Vec<_>>(),
                ["minecraft:elytra"]
            );
            let (_, second) = frames.act(id, &hit, true).unwrap();
            assert!(second.removed);
            assert_eq!(second.sounds, [FrameSound::Break]);
            assert_eq!(
                second
                    .drops
                    .iter()
                    .map(|s| s.id.as_str())
                    .collect::<Vec<_>>(),
                [ITEM_FRAME]
            );
            assert!(frames.list.is_empty());
        }
    }

    #[test]
    fn creative_hits_drop_nothing() {
        let (mut frames, id) = filled(false, Some("minecraft:elytra"));
        let creative = FrameAction::Attack { infinite: true };
        let (frame, first) = frames.act(id, &creative, true).unwrap();
        assert!(
            frame.item.is_none() && first.drops.is_empty(),
            "the item vanishes"
        );
        let (_, second) = frames.act(id, &creative, true).unwrap();
        assert!(
            second.removed && second.drops.is_empty(),
            "and so does the frame"
        );
    }

    #[test]
    fn entity_drops_off_drops_nothing() {
        let (mut frames, id) = filled(false, Some("minecraft:elytra"));
        let hit = FrameAction::Attack { infinite: false };
        let (frame, first) = frames.act(id, &hit, false).unwrap();
        assert!(
            frame.item.is_none()
                && first.drops.is_empty()
                && first.sounds == [FrameSound::RemoveItem]
        );
        let (_, second) = frames.act(id, &hit, false).unwrap();
        assert!(second.removed && second.drops.is_empty());
    }

    #[test]
    fn drop_chance_zero_keeps_the_item_from_dropping() {
        let (mut frames, id) = filled(false, Some("minecraft:elytra"));
        frames.list[0].drop_chance = 0.0;
        let (frame, first) = frames
            .act(id, &FrameAction::Attack { infinite: false }, true)
            .unwrap();
        assert!(
            frame.item.is_none() && first.drops.is_empty(),
            "the item is gone, not dropped"
        );
    }

    #[test]
    fn a_blast_breaks_a_full_frame_with_both_drops() {
        let (mut frames, id) = filled(true, Some("minecraft:elytra"));
        let (_, outcome) = frames
            .act(id, &FrameAction::Blast { infinite: false }, true)
            .unwrap();
        assert!(outcome.removed);
        assert_eq!(outcome.sounds, [FrameSound::Break]);
        assert_eq!(
            outcome
                .drops
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            [GLOW_ITEM_FRAME, "minecraft:elytra"]
        );
    }

    #[test]
    fn drops_appear_a_step_out_from_the_wall() {
        for facing in Direction::ALL {
            let frame = ItemFrame::from_tag(&frame_tag(false, (0, 64, 0), facing), 1).unwrap();
            let out = frame.drop_point() - DVec3::new(0.5, 64.5, 0.5);
            let along = step(facing);
            let expected =
                along * -HANG_OFFSET + DVec3::new(along.x, 0.0, along.z) * f64::from(DROP_STEP);
            assert!(out.abs_diff_eq(expected, 1e-6), "{facing:?}: {out}");
        }
    }

    #[test]
    fn glow_frames_use_their_own_sounds() {
        let sounds = [
            FrameSound::Place,
            FrameSound::AddItem,
            FrameSound::RotateItem,
            FrameSound::RemoveItem,
            FrameSound::Break,
        ];
        for sound in sounds {
            let plain = sound.event(false);
            let glow = sound.event(true);
            assert!(plain.starts_with("minecraft:entity.item_frame."));
            assert_eq!(glow, plain.replace("item_frame", "glow_item_frame"));
        }
        let (mut frames, id) = filled(true, None);
        let (frame, outcome) = frames.act(id, &held("minecraft:carrot", 1), true).unwrap();
        assert!(frame.glow);
        assert_eq!(
            outcome.sounds[0].event(frame.glow),
            "minecraft:entity.glow_item_frame.add_item"
        );
    }

    #[test]
    fn the_crosshair_picks_the_nearest_frame_box_and_a_use_passes_only_on_an_empty_hand_and_frame()
    {
        let (frames, _) = filled(false, None);
        let views = frames.views();
        let eye = DVec3::new(0.5, 64.5, -3.0);
        let (id, distance) = on_ray(&views, eye, DVec3::Z, 5.0).unwrap();
        assert_eq!(id, views[0].id);
        assert!((distance - (3.0 + 0.9375)).abs() < 1e-9, "{distance}");
        assert_eq!(on_ray(&views, eye, DVec3::Z, 3.5), None);
        assert_eq!(
            on_ray(&views, eye + DVec3::X * 0.4, DVec3::Z, 5.0),
            None,
            "the rim ends 0.125 in"
        );
        assert!(use_passes(&views[0], false) && !use_passes(&views[0], true));
        let mut full = views[0].clone();
        full.item = Some("minecraft:carrot".to_owned());
        assert!(!use_passes(&full, false));
        full.fixed = true;
        assert!(use_passes(&full, true));
        assert_eq!(frame_item_use(7).change, crate::bucket::Change::Frame(7));
    }
}
