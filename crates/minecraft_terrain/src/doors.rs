use glam::DVec3;
use minecraftoss_player::{Block, Face, Player, Pos, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opening {
    Door,
    Trapdoor,
    FenceGate,
}

impl Opening {
    pub fn of(id: &str) -> Option<Self> {
        if id.ends_with("_trapdoor") {
            Some(Self::Trapdoor)
        } else if id.ends_with("_door") {
            Some(Self::Door)
        } else if id.ends_with("_fence_gate") {
            Some(Self::FenceGate)
        } else {
            None
        }
    }
}

const FULL: [f64; 6] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];

pub fn facing(yaw: f64) -> &'static str {
    match ((yaw / 90.0 + 0.5).floor() as i64).rem_euclid(4) {
        0 => "south",
        1 => "west",
        2 => "north",
        _ => "east",
    }
}

fn step(direction: &str) -> (i32, i32) {
    match direction {
        "south" => (0, 1),
        "west" => (-1, 0),
        "north" => (0, -1),
        _ => (1, 0),
    }
}

fn clockwise(direction: &str) -> &'static str {
    match direction {
        "north" => "east",
        "east" => "south",
        "south" => "west",
        _ => "north",
    }
}

fn counter_clockwise(direction: &str) -> &'static str {
    match direction {
        "north" => "west",
        "west" => "south",
        "south" => "east",
        _ => "north",
    }
}

fn opposite(direction: &str) -> &'static str {
    clockwise(clockwise(direction))
}

fn face_name(face: Face) -> &'static str {
    match face {
        Face::Down => "down",
        Face::Up => "up",
        Face::North => "north",
        Face::South => "south",
        Face::West => "west",
        Face::East => "east",
    }
}

fn beside(pos: Pos, direction: &str) -> Pos {
    let (dx, dz) = step(direction);
    (pos.0 + dx, pos.1, pos.2 + dz)
}

fn above(pos: Pos) -> Pos {
    (pos.0, pos.1 + 1, pos.2)
}

pub fn replaceable(block: Option<&Block>) -> bool {
    block.is_none_or(|b| {
        let id = b.id.rsplit(':').next().unwrap_or(&b.id);
        matches!(
            id,
            "air"
                | "cave_air"
                | "void_air"
                | "water"
                | "lava"
                | "short_grass"
                | "tall_grass"
                | "fern"
                | "large_fern"
                | "dead_bush"
                | "seagrass"
                | "tall_seagrass"
                | "vine"
                | "fire"
                | "soul_fire"
                | "light"
                | "glow_lichen"
                | "hanging_roots"
                | "crimson_roots"
                | "warped_roots"
                | "nether_sprouts"
                | "structure_void"
        ) || (id == "snow" && b.property("layers").is_none_or(|l| l == "1"))
    })
}

pub fn sturdy_top(world: &impl World, pos: Pos) -> bool {
    world
        .collision_boxes(pos)
        .iter()
        .any(|b| b[0] <= 0.0 && b[2] <= 0.0 && b[3] >= 1.0 && b[5] >= 1.0 && b[4] >= 1.0)
}

fn full_block(world: &impl World, pos: Pos) -> bool {
    world.collision_boxes(pos) == [FULL]
}

fn source_water(block: Option<&Block>) -> bool {
    block.is_some_and(|b| b.id == "minecraft:water" && b.property("level").is_none_or(|l| l == "0"))
}

pub fn hinge(world: &impl World, pos: Pos, id: &str, facing: &str, click: DVec3) -> &'static str {
    let (ccw, cw) = (counter_clockwise(facing), clockwise(facing));
    let full = |p: Pos| i32::from(full_block(world, p));
    let balance = full(beside(pos, cw)) + full(beside(above(pos), cw))
        - full(beside(pos, ccw))
        - full(beside(above(pos), ccw));
    let lower_door = |p: Pos| {
        world
            .block(p)
            .is_some_and(|b| b.id == id && b.property("half") == Some("lower"))
    };
    let (door_ccw, door_cw) = (lower_door(beside(pos, ccw)), lower_door(beside(pos, cw)));
    if (door_ccw && !door_cw) || balance > 0 {
        return "right";
    }
    if (door_cw && !door_ccw) || balance < 0 {
        return "left";
    }
    let (sx, sz) = step(facing);
    let (cx, cz) = (click.x - f64::from(pos.0), click.z - f64::from(pos.2));
    if (sx < 0 && cz < 0.5) || (sx > 0 && cz > 0.5) || (sz < 0 && cx > 0.5) || (sz > 0 && cx < 0.5)
    {
        "right"
    } else {
        "left"
    }
}

pub fn door(
    world: &impl World,
    pos: Pos,
    id: &str,
    yaw: f64,
    click: DVec3,
    sturdy: &dyn Fn(Pos) -> bool,
) -> Option<(Block, Block)> {
    if pos.1 >= world.max_y() || !replaceable(world.block(above(pos)).as_ref()) {
        return None;
    }
    if !sturdy((pos.0, pos.1 - 1, pos.2)) {
        return None;
    }
    let facing = facing(yaw);
    let lower = Block::new(id)
        .with("facing", facing)
        .with("half", "lower")
        .with("hinge", hinge(world, pos, id, facing, click))
        .with("open", "false")
        .with("powered", "false");
    let upper = lower.clone().with("half", "upper");
    Some((lower, upper))
}

pub fn trapdoor(
    world: &impl World,
    pos: Pos,
    id: &str,
    yaw: f64,
    face: Face,
    click_y: f64,
) -> Block {
    let (facing, half) = match face {
        Face::Up | Face::Down => (
            opposite(facing(yaw)),
            if face == Face::Up { "bottom" } else { "top" },
        ),
        _ => (
            face_name(face),
            if click_y - f64::from(pos.1) > 0.5 {
                "top"
            } else {
                "bottom"
            },
        ),
    };
    let waterlogged = source_water(world.block(pos).as_ref());
    Block::new(id)
        .with("facing", facing)
        .with("half", half)
        .with("open", "false")
        .with("powered", "false")
        .with("waterlogged", if waterlogged { "true" } else { "false" })
}

pub fn fence_gate(world: &impl World, pos: Pos, id: &str, yaw: f64) -> Block {
    let facing = facing(yaw);
    let wall = |p: Pos| world.block(p).is_some_and(|b| b.id.ends_with("_wall"));
    let in_wall = if matches!(facing, "north" | "south") {
        wall(beside(pos, "west")) || wall(beside(pos, "east"))
    } else {
        wall(beside(pos, "north")) || wall(beside(pos, "south"))
    };
    Block::new(id)
        .with("facing", facing)
        .with("in_wall", if in_wall { "true" } else { "false" })
        .with("open", "false")
        .with("powered", "false")
}

pub fn place(
    player: &Player,
    world: &mut impl World,
    id: &str,
    sturdy: &dyn Fn(Pos) -> bool,
) -> Option<Pos> {
    let opening = Opening::of(id)?;
    let hit = player.target(world, 5.0)?;
    let (dx, dy, dz) = hit.face.offset();
    let pos = (hit.pos.0 + dx, hit.pos.1 + dy, hit.pos.2 + dz);
    match opening {
        Opening::Door => {
            let (lower, upper) = door(world, pos, id, player.yaw, hit.point, sturdy)?;
            let placed = player.place_target_with(world, lower)?;
            world.set_block(above(placed), Some(upper));
            Some(placed)
        }
        Opening::Trapdoor => {
            let block = trapdoor(world, pos, id, player.yaw, hit.face, hit.point.y);
            player.place_target_with(world, block)
        }
        Opening::FenceGate => {
            let block = fence_gate(world, pos, id, player.yaw);
            player.place_target_with(world, block)
        }
    }
}

pub struct Outlined<'a, W>(pub &'a W);

impl<W: World> World for Outlined<'_, W> {
    fn block(&self, pos: Pos) -> Option<Block> {
        self.0.block(pos).map(|b| {
            if b.id.ends_with("_fence_gate") && b.property("open") == Some("true") {
                b.with("open", "false")
            } else {
                b
            }
        })
    }

    fn set_block(&mut self, _pos: Pos, _block: Option<Block>) {}

    fn collision_boxes(&self, pos: Pos) -> Vec<[f64; 6]> {
        self.0.collision_boxes(pos)
    }
}

fn wood_set(id: &str) -> &'static str {
    let path = id.rsplit(':').next().unwrap_or(id);
    if path.starts_with("cherry_") {
        "cherry_wood"
    } else if path.starts_with("bamboo_") {
        "bamboo_wood"
    } else if path.starts_with("crimson_") || path.starts_with("warped_") {
        "nether_wood"
    } else if path.contains("copper_") {
        "copper"
    } else if path.starts_with("iron_") {
        "iron"
    } else {
        "wooden"
    }
}

pub struct Sound {
    pub event: String,
    pub volume: f32,
    pub pitch: f32,
    pub pitch_spread: f32,
}

pub fn toggle_sound(block: &Block) -> Option<Sound> {
    let path = block.id.rsplit(':').next().unwrap_or(&block.id);
    let flag = |name: &str| block.property(name) == Some("true");
    let swing = |kind: &str| {
        let set = wood_set(&block.id);
        let open = if flag("open") { "close" } else { "open" };
        Sound {
            event: format!("minecraft:block.{set}_{kind}.{open}"),
            volume: 1.0,
            pitch: 0.9,
            pitch_spread: 0.1,
        }
    };
    match Opening::of(&block.id) {
        Some(Opening::Door) => return Some(swing("door")),
        Some(Opening::Trapdoor) => return Some(swing("trapdoor")),
        Some(Opening::FenceGate) => {
            let set = match wood_set(&block.id) {
                "wooden" => "",
                set => set,
            };
            let open = if flag("open") { "close" } else { "open" };
            let event = if set.is_empty() {
                format!("minecraft:block.fence_gate.{open}")
            } else {
                format!("minecraft:block.{set}_fence_gate.{open}")
            };
            return Some(Sound {
                event,
                volume: 1.0,
                pitch: 0.9,
                pitch_spread: 0.1,
            });
        }
        None => {}
    }
    if path == "lever" {
        return Some(Sound {
            event: "minecraft:block.lever.click".to_owned(),
            volume: 0.3,
            pitch: if flag("powered") { 0.5 } else { 0.6 },
            pitch_spread: 0.0,
        });
    }
    if path == "comparator" {
        let subtract = block.property("mode") != Some("subtract");
        return Some(Sound {
            event: "minecraft:block.comparator.click".to_owned(),
            volume: 0.3,
            pitch: if subtract { 0.55 } else { 0.5 },
            pitch_spread: 0.0,
        });
    }
    if path == "note_block" {
        let note = block
            .property("note")
            .and_then(|n| n.parse::<i32>().ok())
            .map_or(0, |n| (n + 1) % 25);
        let instrument = block.property("instrument").unwrap_or("harp");
        return Some(Sound {
            event: format!("minecraft:block.note_block.{instrument}"),
            volume: 3.0,
            pitch: 2.0_f32.powf((note - 12) as f32 / 12.0),
            pitch_spread: 0.0,
        });
    }
    if path.ends_with("_button") && !flag("powered") {
        return Some(Sound {
            event: format!(
                "minecraft:block.{}_button.click_on",
                button_set(path, &block.id)
            ),
            volume: 1.0,
            pitch: 1.0,
            pitch_spread: 0.0,
        });
    }
    None
}

fn button_set(path: &str, id: &str) -> &'static str {
    if path.starts_with("stone_") || path.starts_with("polished_blackstone_") {
        "stone"
    } else {
        wood_set(id)
    }
}

pub fn release_sound(before: &Block, after: &Block) -> Option<Sound> {
    let path = after.id.rsplit(':').next().unwrap_or(&after.id);
    let released = before.id == after.id
        && path.ends_with("_button")
        && before.property("powered") == Some("true")
        && after.property("powered") == Some("false");
    released.then(|| Sound {
        event: format!(
            "minecraft:block.{}_button.click_off",
            button_set(path, &after.id)
        ),
        volume: 1.0,
        pitch: 1.0,
        pitch_spread: 0.0,
    })
}

pub fn powered_toggle(before: &Block, after: &Block) -> bool {
    Opening::of(&after.id).is_some()
        && before.id == after.id
        && after.property("half") != Some("upper")
        && before.property("open") != after.property("open")
        && before.property("powered") != after.property("powered")
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
            for z in -3..=3 {
                world.set_block((x, 0, z), Some(Block::new("minecraft:stone")));
            }
        }
        world
    }

    const OAK: &str = "minecraft:oak_door";

    #[test]
    fn the_players_yaw_gives_vanilla_horizontal_directions() {
        assert_eq!(facing(0.0), "south");
        assert_eq!(facing(44.0), "south");
        assert_eq!(facing(46.0), "west");
        assert_eq!(facing(180.0), "north");
        assert_eq!(facing(-90.0), "east");
        assert_eq!(facing(270.0), "east");
    }

    #[test]
    fn a_door_places_both_halves_facing_the_player() {
        let world = floor();
        let (lower, upper) = door(
            &world,
            (0, 1, 0),
            OAK,
            180.0,
            DVec3::new(0.2, 1.0, 0.5),
            &|p| sturdy_top(&world, p),
        )
        .expect("room for a door");
        assert_eq!(lower.property("facing"), Some("north"));
        assert_eq!(lower.property("half"), Some("lower"));
        assert_eq!(upper.property("half"), Some("upper"));
        assert_eq!(upper.property("hinge"), lower.property("hinge"));
        assert_eq!(lower.property("open"), Some("false"));
    }

    #[test]
    fn a_door_needs_room_above_and_ground_below() {
        let mut world = floor();
        assert!(
            door(
                &world,
                (0, 2, 0),
                OAK,
                0.0,
                DVec3::new(0.5, 2.0, 0.5),
                &|p| sturdy_top(&world, p)
            )
            .is_none()
        );
        world.set_block((0, 2, 0), Some(Block::new("minecraft:dirt")));
        assert!(
            door(
                &world,
                (0, 1, 0),
                OAK,
                0.0,
                DVec3::new(0.5, 1.0, 0.5),
                &|p| sturdy_top(&world, p)
            )
            .is_none()
        );
        world.set_block((0, 2, 0), Some(Block::new("minecraft:short_grass")));
        assert!(
            door(
                &world,
                (0, 1, 0),
                OAK,
                0.0,
                DVec3::new(0.5, 1.0, 0.5),
                &|p| sturdy_top(&world, p)
            )
            .is_some()
        );
    }

    #[test]
    fn the_hinge_follows_where_the_player_clicked() {
        let world = floor();
        let facing_north = |x: f64| hinge(&world, (0, 1, 0), OAK, "north", DVec3::new(x, 1.0, 0.5));
        assert_eq!(facing_north(0.8), "right");
        assert_eq!(facing_north(0.2), "left");
        let facing_east = |z: f64| hinge(&world, (0, 1, 0), OAK, "east", DVec3::new(0.5, 1.0, z));
        assert_eq!(facing_east(0.8), "right");
        assert_eq!(facing_east(0.2), "left");
    }

    #[test]
    fn the_hinge_goes_against_the_wall() {
        let mut world = floor();
        world.set_block((1, 1, 0), Some(Block::new("minecraft:stone")));
        assert_eq!(
            hinge(&world, (0, 1, 0), OAK, "north", DVec3::new(0.2, 1.0, 0.5)),
            "right"
        );
        let mut world = floor();
        world.set_block((-1, 1, 0), Some(Block::new("minecraft:stone")));
        world.set_block((-1, 2, 0), Some(Block::new("minecraft:stone")));
        assert_eq!(
            hinge(&world, (0, 1, 0), OAK, "north", DVec3::new(0.8, 1.0, 0.5)),
            "left"
        );
    }

    #[test]
    fn a_door_beside_another_makes_a_double_door() {
        let mut world = floor();
        world.set_block(
            (-1, 1, 0),
            Some(Block::new(OAK).with("half", "lower").with("hinge", "left")),
        );
        assert_eq!(
            hinge(&world, (0, 1, 0), OAK, "north", DVec3::new(0.2, 1.0, 0.5)),
            "right"
        );
    }

    #[test]
    fn a_trapdoor_takes_the_clicked_side_or_the_players_back() {
        let world = floor();
        let id = "minecraft:oak_trapdoor";
        let side = trapdoor(&world, (1, 1, 0), id, 0.0, Face::East, 1.7);
        assert_eq!(side.property("facing"), Some("east"));
        assert_eq!(side.property("half"), Some("top"));
        let low = trapdoor(&world, (1, 1, 0), id, 0.0, Face::East, 1.2);
        assert_eq!(low.property("half"), Some("bottom"));
        let floor_click = trapdoor(&world, (0, 1, 0), id, 0.0, Face::Up, 1.0);
        assert_eq!(floor_click.property("facing"), Some("north"));
        assert_eq!(floor_click.property("half"), Some("bottom"));
        let ceiling = trapdoor(&world, (0, 1, 0), id, 90.0, Face::Down, 2.0);
        assert_eq!(ceiling.property("facing"), Some("east"));
        assert_eq!(ceiling.property("half"), Some("top"));
    }

    #[test]
    fn a_trapdoor_in_still_water_is_waterlogged() {
        let mut world = floor();
        world.set_block(
            (0, 1, 0),
            Some(Block::new("minecraft:water").with("level", "0")),
        );
        let block = trapdoor(
            &world,
            (0, 1, 0),
            "minecraft:oak_trapdoor",
            0.0,
            Face::Up,
            1.0,
        );
        assert_eq!(block.property("waterlogged"), Some("true"));
    }

    #[test]
    fn a_fence_gate_between_walls_sits_lower() {
        let mut world = floor();
        world.set_block((-1, 1, 0), Some(Block::new("minecraft:cobblestone_wall")));
        let gate = fence_gate(&world, (0, 1, 0), "minecraft:oak_fence_gate", 180.0);
        assert_eq!(gate.property("facing"), Some("north"));
        assert_eq!(gate.property("in_wall"), Some("true"));
        let across = fence_gate(&world, (0, 1, 0), "minecraft:oak_fence_gate", 90.0);
        assert_eq!(across.property("in_wall"), Some("false"));
    }

    #[test]
    fn placing_a_door_through_a_player_sets_both_halves() {
        let mut world = floor();
        let mut player = Player::new(DVec3::new(0.5, 1.0, 3.5));
        player.yaw = 180.0;
        player.pitch = 35.0;
        let support = Blocks(world.0.clone());
        let placed = place(&player, &mut world, OAK, &|p| sturdy_top(&support, p))
            .expect("a door is placed");
        let upper = world.block(above(placed)).expect("an upper half");
        assert_eq!(upper.property("half"), Some("upper"));
        assert_eq!(
            world
                .block(placed)
                .and_then(|b| b.property("half").map(str::to_owned))
                .as_deref(),
            Some("lower")
        );
    }

    #[test]
    fn open_fence_gates_can_still_be_aimed_at() {
        let mut world = floor();
        world.set_block(
            (0, 1, 1),
            Some(
                Block::new("minecraft:oak_fence_gate")
                    .with("facing", "north")
                    .with("open", "true"),
            ),
        );
        let mut player = Player::new(DVec3::new(0.5, 1.0, 3.5));
        player.yaw = 180.0;
        player.pitch = 20.0;
        assert_ne!(player.target(&world, 5.0).map(|h| h.pos), Some((0, 1, 1)));
        assert_eq!(
            player.target(&Outlined(&world), 5.0).map(|h| h.pos),
            Some((0, 1, 1))
        );
    }

    #[test]
    fn toggling_sounds_follow_the_block_set() {
        let oak = Block::new("minecraft:oak_door").with("open", "false");
        assert_eq!(
            toggle_sound(&oak).map(|s| s.event).as_deref(),
            Some("minecraft:block.wooden_door.open")
        );
        let iron = Block::new("minecraft:iron_trapdoor").with("open", "true");
        assert_eq!(
            toggle_sound(&iron).map(|s| s.event).as_deref(),
            Some("minecraft:block.iron_trapdoor.close")
        );
        let cherry = Block::new("minecraft:cherry_fence_gate").with("open", "false");
        assert_eq!(
            toggle_sound(&cherry).map(|s| s.event).as_deref(),
            Some("minecraft:block.cherry_wood_fence_gate.open")
        );
        let birch = Block::new("minecraft:birch_fence_gate").with("open", "true");
        assert_eq!(
            toggle_sound(&birch).map(|s| s.event).as_deref(),
            Some("minecraft:block.fence_gate.close")
        );
        let copper = Block::new("minecraft:waxed_oxidized_copper_door").with("open", "false");
        assert_eq!(
            toggle_sound(&copper).map(|s| s.event).as_deref(),
            Some("minecraft:block.copper_door.open")
        );
        let crimson = Block::new("minecraft:crimson_trapdoor").with("open", "false");
        assert_eq!(
            toggle_sound(&crimson).map(|s| s.event).as_deref(),
            Some("minecraft:block.nether_wood_trapdoor.open")
        );
        let button = Block::new("minecraft:stone_button").with("powered", "false");
        assert_eq!(
            toggle_sound(&button).map(|s| s.event).as_deref(),
            Some("minecraft:block.stone_button.click_on")
        );
        assert!(
            toggle_sound(&Block::new("minecraft:oak_button").with("powered", "true")).is_none()
        );
        assert!(toggle_sound(&Block::new("minecraft:stone")).is_none());
    }

    #[test]
    fn comparators_click_note_blocks_play_and_buttons_click_off() {
        let compare = Block::new("minecraft:comparator").with("mode", "compare");
        let click = toggle_sound(&compare).expect("a comparator click");
        assert_eq!(
            (click.event.as_str(), click.pitch),
            ("minecraft:block.comparator.click", 0.55)
        );
        let note = Block::new("minecraft:note_block")
            .with("note", "11")
            .with("instrument", "bell");
        let played = toggle_sound(&note).expect("a note");
        assert_eq!(played.event, "minecraft:block.note_block.bell");
        assert!((played.pitch - 1.0).abs() < 1e-6);
        let pressed = Block::new("minecraft:oak_button").with("powered", "true");
        let released = pressed.clone().with("powered", "false");
        assert_eq!(
            release_sound(&pressed, &released)
                .map(|s| s.event)
                .as_deref(),
            Some("minecraft:block.wooden_button.click_off")
        );
        assert!(release_sound(&released, &pressed).is_none());
    }

    #[test]
    fn only_redstone_toggles_are_heard_from_the_server() {
        let closed = Block::new("minecraft:iron_door")
            .with("half", "lower")
            .with("open", "false")
            .with("powered", "false");
        let opened = closed.clone().with("open", "true").with("powered", "true");
        assert!(powered_toggle(&closed, &opened));
        assert!(!powered_toggle(
            &closed.clone().with("half", "upper"),
            &opened.clone().with("half", "upper")
        ));
        let by_hand = closed.clone().with("open", "true");
        assert!(!powered_toggle(&closed, &by_hand));
    }
}
