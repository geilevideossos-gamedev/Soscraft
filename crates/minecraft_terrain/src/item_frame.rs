use std::collections::BTreeMap;

use glam::DVec3;
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
const BLOCK_POS_RANGE: i64 = 16;
const BELOW_WORLD: f64 = 64.0;
const BORDER: f64 = 29_999_984.0;

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
pub struct FrameView {
    pub id: u64,
    pub glow: bool,
    pub pos: BlockPos,
    pub facing: Direction,
    pub position: DVec3,
    pub item: Option<String>,
    pub glint: bool,
    pub rotation: i32,
    pub invisible: bool,
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
    let mut stack = ItemStack::new(id, 1);
    stack.components = tag
        .get("components")
        .and_then(crate::item_nbt::saved_components)
        .map(|components| crate::item_nbt::tag_json(&components));
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
        let along = step(self.facing).abs();
        let size = DVec3::splat(WIDTH) + along * (DEPTH - WIDTH);
        let centre = self.position();
        (centre - size / 2.0, centre + size / 2.0)
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
            glint: self.item.as_ref().is_some_and(crate::glint::has_glint),
            rotation: self.rotation,
            invisible: self.invisible,
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

    pub fn views(&self) -> Vec<FrameView> {
        self.list.iter().map(ItemFrame::view).collect()
    }
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
}
