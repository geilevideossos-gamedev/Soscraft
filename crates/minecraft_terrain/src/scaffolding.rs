use glam::DVec3;
use minecraftoss_player::inventory::Inventory;
use minecraftoss_player::{Block, Face, Hit, Player, Pos, World};

use crate::bottle::is_water_source;
use crate::doors::facing;

pub const SCAFFOLDING: &str = "minecraft:scaffolding";
pub const STABILITY_MAX_DISTANCE: i32 = 7;
const REACH: f64 = 5.0;
const HORIZONTAL: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

fn is_scaffolding(block: Option<&Block>) -> bool {
    block.is_some_and(|b| b.id == SCAFFOLDING)
}

fn distance_of(block: &Block) -> i32 {
    block
        .property("distance")
        .and_then(|d| d.parse().ok())
        .unwrap_or(STABILITY_MAX_DISTANCE)
}

fn below(pos: Pos) -> Pos {
    (pos.0, pos.1 - 1, pos.2)
}

fn step(pos: Pos, face: Face) -> Pos {
    let (dx, dy, dz) = face.offset();
    (pos.0 + dx, pos.1 + dy, pos.2 + dz)
}

fn opposite(face: Face) -> Face {
    match face {
        Face::Down => Face::Up,
        Face::Up => Face::Down,
        Face::North => Face::South,
        Face::South => Face::North,
        Face::West => Face::East,
        Face::East => Face::West,
    }
}

fn horizontal(face: Face) -> bool {
    !matches!(face, Face::Down | Face::Up)
}

fn facing_face(direction: &str) -> Face {
    match direction {
        "north" => Face::North,
        "south" => Face::South,
        "west" => Face::West,
        _ => Face::East,
    }
}

pub fn nearest(look: DVec3) -> Face {
    let mut best = (Face::North, 0.0);
    for (face, dot) in [
        (Face::Down, -look.y),
        (Face::Up, look.y),
        (Face::North, -look.z),
        (Face::South, look.z),
        (Face::West, -look.x),
        (Face::East, look.x),
    ] {
        if dot > best.1 {
            best = (face, dot);
        }
    }
    best.0
}

pub fn distance(world: &impl World, pos: Pos, sturdy: &dyn Fn(Pos) -> bool) -> i32 {
    let mut distance = match world.block(below(pos)) {
        Some(block) if block.id == SCAFFOLDING => distance_of(&block),
        _ if sturdy(below(pos)) => return 0,
        _ => STABILITY_MAX_DISTANCE,
    };
    for (dx, dz) in HORIZONTAL {
        if let Some(side) = world
            .block((pos.0 + dx, pos.1, pos.2 + dz))
            .filter(|b| b.id == SCAFFOLDING)
        {
            distance = distance.min(distance_of(&side) + 1);
            if distance == 1 {
                break;
            }
        }
    }
    distance
}

pub fn state(world: &impl World, pos: Pos, sturdy: &dyn Fn(Pos) -> bool) -> Block {
    let distance = distance(world, pos, sturdy);
    let bottom = distance > 0 && !is_scaffolding(world.block(below(pos)).as_ref());
    let waterlogged = world.block(pos).is_some_and(|b| is_water_source(&b));
    let flag = |on: bool| if on { "true" } else { "false" };
    Block::new(SCAFFOLDING)
        .with("distance", &distance.to_string())
        .with("bottom", flag(bottom))
        .with("waterlogged", flag(waterlogged))
}

pub fn target(
    world: &impl World,
    hit: &Hit,
    look: DVec3,
    facing: &str,
    sneaking: bool,
    sturdy: &dyn Fn(Pos) -> bool,
    replaceable: &dyn Fn(Option<&Block>) -> bool,
) -> Option<Pos> {
    let inside = hit.distance <= 0.0;
    let face = if inside {
        opposite(nearest(look))
    } else {
        hit.face
    };
    let clicked = world.block(hit.pos);
    let replace_clicked = is_scaffolding(clicked.as_ref()) || replaceable(clicked.as_ref());
    let clicked_pos = if replace_clicked {
        hit.pos
    } else {
        step(hit.pos, face)
    };
    let in_world = |pos: Pos| pos.1 >= world.min_y() && pos.1 <= world.max_y();
    let at_clicked = world.block(clicked_pos);
    if !is_scaffolding(at_clicked.as_ref()) {
        let free = replace_clicked || replaceable(at_clicked.as_ref());
        let held = distance(world, clicked_pos, sturdy) != STABILITY_MAX_DISTANCE;
        return (free && held && in_world(clicked_pos)).then_some(clicked_pos);
    }
    let direction = if sneaking {
        if inside { opposite(face) } else { face }
    } else if face == Face::Up {
        facing_face(facing)
    } else {
        Face::Up
    };
    let mut pos = step(clicked_pos, direction);
    let mut sideways = 0;
    while sideways < STABILITY_MAX_DISTANCE {
        if !in_world(pos) {
            return None;
        }
        let block = world.block(pos);
        if !is_scaffolding(block.as_ref()) {
            return replaceable(block.as_ref()).then_some(pos);
        }
        pos = step(pos, direction);
        if horizontal(direction) {
            sideways += 1;
        }
    }
    None
}

pub fn place(
    player: &Player,
    world: &mut impl World,
    inventory: &mut Inventory,
    sneaking: bool,
    sturdy: &dyn Fn(Pos) -> bool,
    replaceable: &dyn Fn(Option<&Block>) -> bool,
) -> Option<Pos> {
    let held = inventory.slots.get(player.selected)?.as_ref()?;
    if held.id != SCAFFOLDING || held.count == 0 {
        return None;
    }
    let hit = player.target(world, REACH)?;
    let pos = target(
        world,
        &hit,
        player.look(),
        facing(player.yaw),
        sneaking,
        sturdy,
        replaceable,
    )?;
    let block = state(world, pos, sturdy);
    world.set_block(pos, Some(block));
    let slot = &mut inventory.slots[player.selected];
    if let Some(stack) = slot.as_mut() {
        stack.count -= 1;
        if stack.count == 0 {
            *slot = None;
        }
    }
    Some(pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doors::replaceable;
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

    fn ground(world: &Blocks) -> impl Fn(Pos) -> bool {
        let stone: std::collections::HashSet<Pos> = world
            .0
            .iter()
            .filter(|(_, b)| b.id == "minecraft:stone")
            .map(|(pos, _)| *pos)
            .collect();
        move |pos| stone.contains(&pos)
    }

    fn scaffold(distance: i32) -> Block {
        Block::new(SCAFFOLDING).with("distance", &distance.to_string())
    }

    fn tower(height: i32) -> Blocks {
        let mut world = Blocks::default();
        for x in -3..=3 {
            for z in -3..=3 {
                world.set_block((x, 0, z), Some(Block::new("minecraft:stone")));
            }
        }
        for y in 1..=height {
            world.set_block((0, y, 0), Some(scaffold(0)));
        }
        world
    }

    fn hit(pos: Pos, face: Face, distance: f64) -> Hit {
        Hit {
            pos,
            face,
            distance,
            point: DVec3::ZERO,
        }
    }

    const LEVEL: DVec3 = DVec3::new(0.0, 0.0, 1.0);

    #[test]
    fn clicking_a_column_side_stacks_on_its_top() {
        let world = tower(4);
        let sturdy = ground(&world);
        let side = hit((0, 2, 0), Face::West, 2.0);
        assert_eq!(
            target(&world, &side, LEVEL, "east", false, &sturdy, &replaceable),
            Some((0, 5, 0))
        );
    }

    #[test]
    fn clicking_a_column_top_extends_it_the_way_the_player_faces() {
        let mut world = tower(1);
        let sturdy = ground(&world);
        let top = hit((0, 1, 0), Face::Up, 2.0);
        assert_eq!(
            target(&world, &top, LEVEL, "south", false, &sturdy, &replaceable),
            Some((0, 1, 1))
        );
        for z in 1..=7 {
            world.set_block((0, 1, z), Some(scaffold(z.min(7))));
        }
        assert_eq!(
            target(&world, &top, LEVEL, "south", false, &sturdy, &replaceable),
            None
        );
        world.set_block((0, 1, 7), None);
        assert_eq!(
            target(&world, &top, LEVEL, "south", false, &sturdy, &replaceable),
            Some((0, 1, 7))
        );
    }

    #[test]
    fn sneaking_places_against_the_clicked_face() {
        let world = tower(2);
        let sturdy = ground(&world);
        let side = hit((0, 2, 0), Face::West, 2.0);
        assert_eq!(
            target(&world, &side, LEVEL, "east", true, &sturdy, &replaceable),
            Some((-1, 2, 0))
        );
    }

    #[test]
    fn from_inside_a_column_it_builds_up_or_where_the_player_looks() {
        let world = tower(3);
        let sturdy = ground(&world);
        let inside = hit((0, 2, 0), Face::North, 0.0);
        assert_eq!(
            target(
                &world,
                &inside,
                LEVEL,
                "south",
                false,
                &sturdy,
                &replaceable
            ),
            Some((0, 4, 0))
        );
        let down = DVec3::new(0.0, -1.0, 0.1);
        assert_eq!(
            target(&world, &inside, down, "south", false, &sturdy, &replaceable),
            Some((0, 2, 1))
        );
        assert_eq!(
            target(&world, &inside, LEVEL, "south", true, &sturdy, &replaceable),
            Some((0, 2, 1))
        );
        assert_eq!(nearest(DVec3::new(0.3, -0.9, 0.2)), Face::Down);
        assert_eq!(nearest(DVec3::new(-0.8, 0.1, 0.5)), Face::West);
    }

    #[test]
    fn unsupported_spots_and_the_build_limit_refuse() {
        let mut world = Blocks::default();
        world.set_block((0, 10, 0), Some(Block::new("minecraft:stone")));
        let sturdy = ground(&world);
        let floating = hit((0, 10, 0), Face::East, 2.0);
        assert_eq!(
            target(
                &world,
                &floating,
                LEVEL,
                "east",
                false,
                &sturdy,
                &replaceable
            ),
            None
        );
        let top = hit((0, 10, 0), Face::Up, 2.0);
        assert_eq!(
            target(&world, &top, LEVEL, "east", false, &sturdy, &replaceable),
            Some((0, 11, 0))
        );
        world.set_block((0, 318, 0), Some(Block::new("minecraft:stone")));
        world.set_block((0, 319, 0), Some(scaffold(0)));
        let roof = hit((0, 319, 0), Face::East, 2.0);
        assert_eq!(
            target(&world, &roof, LEVEL, "east", false, &sturdy, &replaceable),
            None
        );
    }

    #[test]
    fn replaceable_plants_are_taken_by_the_registry_rule() {
        let mut world = tower(1);
        world.set_block((2, 1, 0), Some(Block::new("minecraft:leaf_litter")));
        let sturdy = ground(&world);
        let litter = hit((2, 1, 0), Face::Up, 2.0);
        let registry = |b: Option<&Block>| {
            replaceable(b) || b.is_some_and(|b| b.id == "minecraft:leaf_litter")
        };
        assert_eq!(
            target(&world, &litter, LEVEL, "east", false, &sturdy, &replaceable),
            None
        );
        assert_eq!(
            target(&world, &litter, LEVEL, "east", false, &sturdy, &registry),
            Some((2, 1, 0))
        );
    }

    #[test]
    fn placed_scaffolding_takes_the_vanilla_state() {
        let mut world = tower(1);
        world.set_block((0, 1, 1), Some(scaffold(1)));
        world.set_block(
            (0, 1, 2),
            Some(Block::new("minecraft:water").with("level", "0")),
        );
        world.set_block((1, 0, 1), None);
        let sturdy = ground(&world);
        let on_ground = state(&world, (2, 1, 2), &sturdy);
        assert_eq!(on_ground.property("distance"), Some("0"));
        assert_eq!(on_ground.property("bottom"), Some("false"));
        let hanging = state(&world, (0, 2, 1), &sturdy);
        assert_eq!(hanging.property("distance"), Some("1"));
        assert_eq!(hanging.property("bottom"), Some("false"));
        let wet = state(&world, (0, 1, 2), &sturdy);
        assert_eq!(wet.property("waterlogged"), Some("true"));
        assert_eq!(wet.property("distance"), Some("0"));
        let reaching = state(&world, (1, 1, 1), &sturdy);
        assert_eq!(reaching.property("distance"), Some("2"));
        assert_eq!(reaching.property("bottom"), Some("true"));
        assert_eq!(reaching.property("waterlogged"), Some("false"));
        assert_eq!(
            distance(&Blocks::default(), (0, 50, 0), &|_| false),
            STABILITY_MAX_DISTANCE
        );
    }
}
