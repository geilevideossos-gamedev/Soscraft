use crate::{Block, Face, Pos, World, mth, source_water_at};

const HEADS: [&str; 7] = [
    "skeleton_skull",
    "wither_skeleton_skull",
    "zombie_head",
    "creeper_head",
    "dragon_head",
    "piglin_head",
    "player_head",
];
const COPPER_CHESTS: [&str; 4] = [
    "copper_chest",
    "exposed_copper_chest",
    "weathered_copper_chest",
    "oxidized_copper_chest",
];
const HORIZONTAL: [Face; 4] = [Face::South, Face::West, Face::North, Face::East];
const SOLID_SIZE: f64 = 0.7291666666666666;

pub(crate) struct View {
    pub(crate) yaw: f64,
    pub(crate) pitch: f64,
    pub(crate) face: Face,
    pub(crate) crouching: bool,
    pub(crate) replace_clicked: bool,
}

pub(crate) enum Placement {
    Other,
    Refused,
    Placed(Block, Option<(Pos, Block)>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mounted {
    Banner,
    Sign,
    Head,
    HangingSign,
}

pub(crate) fn block_entity_placement(
    world: &impl World,
    pos: Pos,
    block: &Block,
    view: &View,
) -> Placement {
    let Some(path) = block.id.strip_prefix("minecraft:") else {
        return Placement::Other;
    };
    let placed =
        |state: Option<Block>| state.map_or(Placement::Refused, |b| Placement::Placed(b, None));
    if let Some((kind, wall_path)) = wall_variant(path) {
        let wall = Block::new(format!("minecraft:{wall_path}"));
        return placed(standing_or_wall(world, pos, block, wall, kind, view));
    }
    let water = if source_water_at(world, pos) {
        "true"
    } else {
        "false"
    };
    let facing = horizontal(view.yaw);
    match path {
        "ender_chest" => placed(Some(
            block
                .clone()
                .with("facing", name(opposite(facing)))
                .with("waterlogged", water),
        )),
        "decorated_pot" => placed(Some(
            block
                .clone()
                .with("facing", name(facing))
                .with("cracked", "false")
                .with("waterlogged", water),
        )),
        "lectern" => placed(Some(
            block
                .clone()
                .with("facing", name(opposite(facing)))
                .with("has_book", "false")
                .with("powered", "false"),
        )),
        "bell" => placed(bell(world, pos, block, view)),
        "ladder" => placed(ladder(world, pos, block, view, water)),
        _ if path == "shulker_box" || path.ends_with("_shulker_box") => {
            placed(Some(block.clone().with("facing", name(view.face))))
        }
        _ if path.ends_with("_bed") => {
            let head = step(pos, facing);
            if !replaceable(world, head) {
                return Placement::Refused;
            }
            let foot = block
                .clone()
                .with("facing", name(facing))
                .with("part", "foot")
                .with("occupied", "false");
            let head_block = foot.clone().with("part", "head");
            Placement::Placed(foot, Some((head, head_block)))
        }
        _ => Placement::Other,
    }
}

pub(crate) fn chest(
    world: &impl World,
    pos: Pos,
    block: &Block,
    view: &View,
) -> (Block, Option<(Pos, Block)>) {
    let water = if source_water_at(world, pos) {
        "true"
    } else {
        "false"
    };
    let single_facing = |at: Pos| {
        world
            .block(at)
            .filter(|other| {
                chests_connect(&block.id, &other.id) && other.property("type") == Some("single")
            })
            .and_then(|other| other.property("facing").and_then(face_named))
    };
    let mut facing = opposite(horizontal(view.yaw));
    let mut kind = "single";
    let mut partner = None;
    if view.crouching && axis(view.face) != 1 {
        let clicked = step(pos, opposite(view.face));
        if let Some(other) = single_facing(clicked).filter(|f| axis(*f) != axis(view.face)) {
            facing = other;
            kind = if counter_clockwise(other) == opposite(view.face) {
                "right"
            } else {
                "left"
            };
            partner = Some(clicked);
        }
    }
    if partner.is_none() && !view.crouching {
        for (side, side_kind) in [
            (clockwise(facing), "left"),
            (counter_clockwise(facing), "right"),
        ] {
            if single_facing(step(pos, side)) == Some(facing) {
                kind = side_kind;
                partner = Some(step(pos, side));
                break;
            }
        }
    }
    let mut placed = block
        .clone()
        .with("facing", name(facing))
        .with("type", kind)
        .with("waterlogged", water);
    let joined = partner.and_then(|at| {
        let other = world.block(at)?;
        let id = joined_chest(&block.id, &other.id);
        placed.id.clone_from(&id);
        let mut half = other.with("type", if kind == "left" { "right" } else { "left" });
        half.id = id;
        Some((at, half))
    });
    (placed, joined)
}

pub(crate) fn pairs_as_chest(id: &str) -> bool {
    matches!(id, "minecraft:chest" | "minecraft:trapped_chest") || copper_chest(id).is_some()
}

pub(crate) fn chests_connect(placed: &str, other: &str) -> bool {
    placed == other || (copper_chest(placed).is_some() && copper_chest(other).is_some())
}

pub(crate) fn joined_chest(placed: &str, other: &str) -> String {
    let (Some((placed_waxed, placed_level)), Some((other_waxed, other_level))) =
        (copper_chest(placed), copper_chest(other))
    else {
        return placed.to_owned();
    };
    let (waxed, level) = if placed_level <= other_level {
        (placed_waxed, placed_level)
    } else {
        (other_waxed, other_level)
    };
    let prefix = if waxed && placed_waxed == other_waxed {
        "waxed_"
    } else {
        ""
    };
    format!("minecraft:{prefix}{}", COPPER_CHESTS[level])
}

fn copper_chest(id: &str) -> Option<(bool, usize)> {
    let path = id.strip_prefix("minecraft:")?;
    let (waxed, path) = match path.strip_prefix("waxed_") {
        Some(rest) => (true, rest),
        None => (false, path),
    };
    let level = COPPER_CHESTS.iter().position(|chest| *chest == path)?;
    Some((waxed, level))
}

fn wall_variant(path: &str) -> Option<(Mounted, String)> {
    if path.contains("wall_") {
        return None;
    }
    if let Some(wood) = path.strip_suffix("_hanging_sign") {
        return Some((Mounted::HangingSign, format!("{wood}_wall_hanging_sign")));
    }
    if let Some(wood) = path.strip_suffix("_sign") {
        return Some((Mounted::Sign, format!("{wood}_wall_sign")));
    }
    if let Some(color) = path.strip_suffix("_banner") {
        return Some((Mounted::Banner, format!("{color}_wall_banner")));
    }
    if HEADS.contains(&path) {
        let (kind, part) = path.rsplit_once('_')?;
        return Some((Mounted::Head, format!("{kind}_wall_{part}")));
    }
    None
}

fn standing_or_wall(
    world: &impl World,
    pos: Pos,
    standing: &Block,
    wall: Block,
    kind: Mounted,
    view: &View,
) -> Option<Block> {
    let wall = wall_state(world, pos, wall, kind, view);
    let attachment = if kind == Mounted::HangingSign {
        Face::Up
    } else {
        Face::Down
    };
    for direction in looking_directions(view) {
        if direction == opposite(attachment) {
            continue;
        }
        let (state, on_wall) = if direction == attachment {
            (
                Some(standing_state(world, pos, standing, kind, view)),
                false,
            )
        } else {
            (wall.clone(), true)
        };
        if let Some(state) = state {
            if can_place(world, pos, &state, kind, on_wall) {
                return Some(state);
            }
        }
    }
    None
}

fn standing_state(
    world: &impl World,
    pos: Pos,
    block: &Block,
    kind: Mounted,
    view: &View,
) -> Block {
    let water = if source_water_at(world, pos) {
        "true"
    } else {
        "false"
    };
    let facing_away = segment(view.yaw as f32 + 180.0).to_string();
    match kind {
        Mounted::Banner => block.clone().with("rotation", &facing_away),
        Mounted::Sign => block
            .clone()
            .with("rotation", &facing_away)
            .with("waterlogged", water),
        Mounted::Head => block
            .clone()
            .with("rotation", &segment(view.yaw as f32).to_string())
            .with("powered", "false"),
        Mounted::HangingSign => {
            let (attached, rotation) = hanging_sign(world, pos, view);
            block
                .clone()
                .with("attached", if attached { "true" } else { "false" })
                .with("rotation", &rotation.to_string())
                .with("waterlogged", water)
        }
    }
}

fn hanging_sign(world: &impl World, pos: Pos, view: &View) -> (bool, i32) {
    let up = step(pos, Face::Up);
    let direction = horizontal(view.yaw);
    let mut attached = !covered(world, up, Face::Down, 0, 16) || view.crouching;
    let above = world
        .block(up)
        .filter(|above| above.id.ends_with("_hanging_sign"));
    if let Some(above) = above.filter(|_| !view.crouching) {
        let below_sign = match above.property("facing") {
            Some(facing) => face_named(facing),
            None => above
                .property("rotation")
                .and_then(|rotation| rotation.parse::<i32>().ok())
                .and_then(segment_direction),
        };
        if below_sign.is_some_and(|other| axis(other) == axis(direction)) {
            attached = false;
        }
    }
    let rotation = if attached {
        segment(view.yaw as f32 + 180.0)
    } else {
        direction_segment(opposite(direction))
    };
    (attached, rotation)
}

fn wall_state(
    world: &impl World,
    pos: Pos,
    wall: Block,
    kind: Mounted,
    view: &View,
) -> Option<Block> {
    let water = if source_water_at(world, pos) {
        "true"
    } else {
        "false"
    };
    for direction in looking_directions(view) {
        if axis(direction) == 1 {
            continue;
        }
        if kind == Mounted::HangingSign && axis(direction) == axis(view.face) {
            continue;
        }
        let state = wall.clone().with("facing", name(opposite(direction)));
        let fits = match kind {
            Mounted::Banner | Mounted::Sign => solid(world, step(pos, direction)),
            Mounted::Head => !replaceable(world, step(pos, direction)),
            Mounted::HangingSign => wall_hanging_sign_fits(world, pos, opposite(direction)),
        };
        if fits {
            return Some(match kind {
                Mounted::Banner => state,
                Mounted::Head => state.with("powered", "false"),
                Mounted::Sign | Mounted::HangingSign => state.with("waterlogged", water),
            });
        }
    }
    None
}

fn can_place(world: &impl World, pos: Pos, state: &Block, kind: Mounted, on_wall: bool) -> bool {
    match (kind, on_wall) {
        (Mounted::Banner | Mounted::Sign, false) => solid(world, step(pos, Face::Down)),
        (Mounted::Banner | Mounted::Sign, true) => state
            .property("facing")
            .and_then(face_named)
            .is_some_and(|facing| solid(world, step(pos, opposite(facing)))),
        (Mounted::HangingSign, false) => covered(world, step(pos, Face::Up), Face::Down, 7, 9),
        (Mounted::HangingSign, true) => state
            .property("facing")
            .and_then(face_named)
            .is_some_and(|facing| wall_hanging_sign_fits(world, pos, facing)),
        (Mounted::Head, _) => true,
    }
}

fn wall_hanging_sign_fits(world: &impl World, pos: Pos, facing: Face) -> bool {
    let attaches = |side: Face, toward: Face| {
        let at = step(pos, side);
        match world
            .block(at)
            .filter(|block| block.id.ends_with("_wall_hanging_sign"))
        {
            Some(other) => other
                .property("facing")
                .and_then(face_named)
                .is_some_and(|other| axis(other) == axis(facing)),
            None => covered(world, at, toward, 0, 16),
        }
    };
    attaches(clockwise(facing), counter_clockwise(facing))
        || attaches(counter_clockwise(facing), clockwise(facing))
}

fn ladder(world: &impl World, pos: Pos, block: &Block, view: &View, water: &str) -> Option<Block> {
    let clicked = world.block(step(pos, opposite(view.face)));
    if !view.replace_clicked
        && clicked.is_some_and(|clicked| {
            clicked.id == block.id && clicked.property("facing") == Some(name(view.face))
        })
    {
        return None;
    }
    looking_directions(view)
        .into_iter()
        .filter(|direction| axis(*direction) != 1)
        .find(|direction| world.sturdy_face(step(pos, *direction), opposite(*direction)))
        .map(|direction| {
            block
                .clone()
                .with("facing", name(opposite(direction)))
                .with("waterlogged", water)
        })
}

fn bell(world: &impl World, pos: Pos, block: &Block, view: &View) -> Option<Block> {
    let base = block.clone().with("powered", "false");
    if axis(view.face) == 1 {
        let attachment = if view.face == Face::Down {
            "ceiling"
        } else {
            "floor"
        };
        let state = base
            .with("attachment", attachment)
            .with("facing", name(horizontal(view.yaw)));
        return bell_survives(world, pos, &state).then_some(state);
    }
    let between = |a: Face, b: Face| {
        covered(world, step(pos, a), opposite(a), 0, 16)
            && covered(world, step(pos, b), opposite(b), 0, 16)
    };
    let double = if axis(view.face) == 0 {
        between(Face::West, Face::East)
    } else {
        between(Face::North, Face::South)
    };
    let state = base.with("facing", name(opposite(view.face))).with(
        "attachment",
        if double { "double_wall" } else { "single_wall" },
    );
    if bell_survives(world, pos, &state) {
        return Some(state);
    }
    let floor = covered(world, step(pos, Face::Down), Face::Up, 0, 16);
    let state = state.with("attachment", if floor { "floor" } else { "ceiling" });
    bell_survives(world, pos, &state).then_some(state)
}

fn bell_survives(world: &impl World, pos: Pos, state: &Block) -> bool {
    match state.property("attachment") {
        Some("floor") => covered(world, step(pos, Face::Down), Face::Up, 0, 16),
        Some("ceiling") => covered(world, step(pos, Face::Up), Face::Down, 7, 9),
        _ => state
            .property("facing")
            .and_then(face_named)
            .is_some_and(|facing| covered(world, step(pos, facing), opposite(facing), 0, 16)),
    }
}

fn looking_directions(view: &View) -> [Face; 6] {
    let mut faces = nearest(view.yaw, view.pitch);
    if view.replace_clicked {
        return faces;
    }
    let toward = opposite(view.face);
    if let Some(index) = faces.iter().position(|face| *face == toward) {
        faces[..=index].rotate_right(1);
    }
    faces
}

fn nearest(yaw: f64, pitch: f64) -> [Face; 6] {
    let pitch = pitch as f32 * 0.017453292;
    let yaw = -(yaw as f32) * 0.017453292;
    let sin_pitch = mth::sin(f64::from(pitch));
    let cos_pitch = mth::cos(f64::from(pitch));
    let sin_yaw = mth::sin(f64::from(yaw));
    let cos_yaw = mth::cos(f64::from(yaw));
    let east = sin_yaw > 0.0;
    let up = -sin_pitch > 0.0;
    let south = cos_yaw > 0.0;
    let east_amount = if east { sin_yaw } else { -sin_yaw };
    let up_amount = if up { -sin_pitch } else { sin_pitch };
    let south_amount = if south { cos_yaw } else { -cos_yaw };
    let x_amount = east_amount * cos_pitch;
    let z_amount = south_amount * cos_pitch;
    let x = if east { Face::East } else { Face::West };
    let y = if up { Face::Up } else { Face::Down };
    let z = if south { Face::South } else { Face::North };
    let [first, second, third] = if east_amount > south_amount {
        if up_amount > x_amount {
            [y, x, z]
        } else if z_amount > up_amount {
            [x, z, y]
        } else {
            [x, y, z]
        }
    } else if up_amount > z_amount {
        [y, z, x]
    } else if x_amount > up_amount {
        [z, x, y]
    } else {
        [z, y, x]
    };
    [
        first,
        second,
        third,
        opposite(third),
        opposite(second),
        opposite(first),
    ]
}

fn horizontal(yaw: f64) -> Face {
    HORIZONTAL[((yaw / 90.0 + 0.5).floor() as i64).rem_euclid(4) as usize]
}

fn segment(degrees: f32) -> i32 {
    ((degrees * (16.0 / 360.0) + 0.5).floor() as i32) & 15
}

fn direction_segment(face: Face) -> i32 {
    HORIZONTAL
        .iter()
        .position(|horizontal| *horizontal == face)
        .map_or(0, |index| index as i32 * 4)
}

fn segment_direction(segment: i32) -> Option<Face> {
    match segment {
        0 => Some(Face::North),
        4 => Some(Face::East),
        8 => Some(Face::South),
        12 => Some(Face::West),
        _ => None,
    }
}

fn replaceable(world: &impl World, pos: Pos) -> bool {
    world
        .block(pos)
        .is_none_or(|block| world.replaceable(&block))
}

fn solid(world: &impl World, pos: Pos) -> bool {
    let boxes = world.collision_boxes(pos);
    if boxes.is_empty() {
        return false;
    }
    let size = [0, 1, 2].map(|axis| {
        let low = boxes
            .iter()
            .map(|bounds| bounds[axis])
            .fold(f64::INFINITY, f64::min);
        let high = boxes
            .iter()
            .map(|bounds| bounds[axis + 3])
            .fold(f64::NEG_INFINITY, f64::max);
        high - low
    });
    (size[0] + size[1] + size[2]) / 3.0 >= SOLID_SIZE || size[1] >= 1.0
}

fn covered(world: &impl World, pos: Pos, face: Face, from_pixel: u8, to_pixel: u8) -> bool {
    covers(&world.collision_boxes(pos), face, from_pixel, to_pixel)
}

pub fn authored_sturdy_face(block: &Block, boxes: &[[f64; 6]], face: Face) -> bool {
    let id = block.id.as_str();
    !id.ends_with("_leaves")
        && (matches!(id, "minecraft:soul_sand" | "minecraft:mud") || covers(boxes, face, 0, 16))
}

fn covers(boxes: &[[f64; 6]], face: Face, from_pixel: u8, to_pixel: u8) -> bool {
    let normal = axis(face);
    let positive = matches!(face, Face::Up | Face::South | Face::East);
    let (u, v) = match normal {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let touching: Vec<&[f64; 6]> = boxes
        .iter()
        .filter(|bounds| {
            if positive {
                bounds[normal + 3] >= 1.0
            } else {
                bounds[normal] <= 0.0
            }
        })
        .collect();
    (from_pixel..to_pixel).all(|i| {
        (from_pixel..to_pixel).all(|j| {
            let a = (f64::from(i) + 0.5) / 16.0;
            let b = (f64::from(j) + 0.5) / 16.0;
            touching.iter().any(|bounds| {
                bounds[u] <= a && a <= bounds[u + 3] && bounds[v] <= b && b <= bounds[v + 3]
            })
        })
    })
}

fn step(pos: Pos, face: Face) -> Pos {
    let (dx, dy, dz) = face.offset();
    (pos.0 + dx, pos.1 + dy, pos.2 + dz)
}

fn axis(face: Face) -> usize {
    match face {
        Face::West | Face::East => 0,
        Face::Down | Face::Up => 1,
        Face::North | Face::South => 2,
    }
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

fn clockwise(face: Face) -> Face {
    match face {
        Face::North => Face::East,
        Face::East => Face::South,
        Face::South => Face::West,
        Face::West => Face::North,
        other => other,
    }
}

fn counter_clockwise(face: Face) -> Face {
    opposite(clockwise(face))
}

fn name(face: Face) -> &'static str {
    match face {
        Face::Down => "down",
        Face::Up => "up",
        Face::North => "north",
        Face::South => "south",
        Face::West => "west",
        Face::East => "east",
    }
}

fn face_named(name: &str) -> Option<Face> {
    Some(match name {
        "down" => Face::Down,
        "up" => Face::Up,
        "north" => Face::North,
        "south" => Face::South,
        "west" => Face::West,
        "east" => Face::East,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Blocks(BTreeMap<Pos, Block>);

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

    fn view(yaw: f64, pitch: f64, face: Face) -> View {
        View {
            yaw,
            pitch,
            face,
            crouching: false,
            replace_clicked: false,
        }
    }

    fn state(
        world: &Blocks,
        pos: Pos,
        id: &str,
        view: &View,
    ) -> Option<(Block, Option<(Pos, Block)>)> {
        match block_entity_placement(world, pos, &Block::new(id), view) {
            Placement::Placed(block, partner) => Some((block, partner)),
            Placement::Refused => None,
            Placement::Other => panic!("{id} has no placement rule"),
        }
    }

    #[test]
    fn looking_order_matches_direction_ordered_by_nearest() {
        assert_eq!(
            nearest(0.0, 0.0),
            [
                Face::South,
                Face::Down,
                Face::West,
                Face::East,
                Face::Up,
                Face::North
            ]
        );
        assert_eq!(nearest(180.0, 60.0)[..2], [Face::Down, Face::North]);
        assert_eq!(nearest(-90.0, -10.0)[0], Face::East);
        let clicked_side = view(0.0, 60.0, Face::North);
        assert_eq!(
            looking_directions(&clicked_side)[..2],
            [Face::South, Face::Down]
        );
        let replaced_grass = View {
            replace_clicked: true,
            ..clicked_side
        };
        assert_eq!(looking_directions(&replaced_grass), nearest(0.0, 60.0));
        assert_eq!(horizontal(0.0), Face::South);
        assert_eq!(horizontal(-90.0), Face::East);
        assert_eq!(horizontal(135.0), Face::North);
        assert_eq!(segment(180.0), 8);
        assert_eq!(segment(-11.0), 0);
        assert_eq!(segment(-12.0), 15);
    }

    #[test]
    fn banners_signs_and_heads_stand_on_floors_and_hang_on_walls() {
        let mut world = Blocks::default();
        world.set_block((0, 0, 0), Some(Block::new("minecraft:stone")));
        world.set_block((0, 1, -1), Some(Block::new("minecraft:stone")));
        let on_top = view(180.0, 70.0, Face::Up);
        let (banner, _) = state(&world, (0, 1, 0), "minecraft:red_banner", &on_top).unwrap();
        assert_eq!(banner.id, "minecraft:red_banner");
        assert_eq!(banner.property("rotation"), Some("0"));
        let (skull, _) = state(&world, (0, 1, 0), "minecraft:skeleton_skull", &on_top).unwrap();
        assert_eq!(skull.property("rotation"), Some("8"));
        let on_wall = view(180.0, 10.0, Face::South);
        let (wall, _) = state(&world, (0, 1, 0), "minecraft:red_banner", &on_wall).unwrap();
        assert_eq!(wall.id, "minecraft:red_wall_banner");
        assert_eq!(wall.property("facing"), Some("south"));
        let (sign, _) = state(&world, (0, 1, 0), "minecraft:oak_sign", &on_wall).unwrap();
        assert_eq!(sign.id, "minecraft:oak_wall_sign");
        let (head, _) = state(
            &world,
            (0, 1, 0),
            "minecraft:wither_skeleton_skull",
            &on_wall,
        )
        .unwrap();
        assert_eq!(head.id, "minecraft:wither_skeleton_wall_skull");
        assert_eq!(head.property("facing"), Some("south"));
        assert!(state(&world, (5, 5, 5), "minecraft:white_banner", &on_wall).is_none());
        let (floating, _) = state(&world, (5, 5, 5), "minecraft:zombie_head", &on_wall).unwrap();
        assert_eq!(floating.id, "minecraft:zombie_head");
    }

    #[test]
    fn ladders_face_away_from_the_wall_they_are_placed_on() {
        let mut world = Blocks::default();
        world.set_block((0, 1, 0), Some(Block::new("minecraft:stone")));
        for (yaw, face, pos, facing) in [
            (180.0, Face::South, (0, 1, 1), "south"),
            (0.0, Face::North, (0, 1, -1), "north"),
            (90.0, Face::East, (1, 1, 0), "east"),
            (-90.0, Face::West, (-1, 1, 0), "west"),
        ] {
            let (ladder, _) = state(&world, pos, "minecraft:ladder", &view(yaw, 10.0, face)).unwrap();
            assert_eq!(ladder.property("facing"), Some(facing), "clicked {facing}");
            assert_eq!(ladder.property("waterlogged"), Some("false"));
        }
        world.set_block((0, 0, 1), Some(Block::new("minecraft:stone")));
        let on_floor_facing_wall = view(180.0, 60.0, Face::Up);
        let (ladder, _) = state(&world, (0, 1, 1), "minecraft:ladder", &on_floor_facing_wall).unwrap();
        assert_eq!(ladder.property("facing"), Some("south"));
        let on_floor_facing_away = view(0.0, 60.0, Face::Up);
        let (ladder, _) = state(&world, (0, 1, 1), "minecraft:ladder", &on_floor_facing_away).unwrap();
        assert_eq!(ladder.property("facing"), Some("south"));
        assert!(state(&world, (5, 1, 5), "minecraft:ladder", &on_floor_facing_wall).is_none());
        world.set_block((0, 1, 1), Some(Block::new("minecraft:water").with("level", "0")));
        let flooded = View {
            replace_clicked: true,
            ..view(180.0, 10.0, Face::South)
        };
        let (ladder, _) = state(&world, (0, 1, 1), "minecraft:ladder", &flooded).unwrap();
        assert_eq!(ladder.property("waterlogged"), Some("true"));
    }

    #[test]
    fn ladders_need_a_sturdy_wall_not_just_a_colliding_one() {
        let side = view(180.0, 10.0, Face::South);
        for (wall, placed) in [
            ("minecraft:oak_leaves", false),
            ("minecraft:soul_sand", true),
            ("minecraft:mud", true),
            ("minecraft:stone_slab", false),
        ] {
            let mut world = Blocks::default();
            world.set_block((0, 1, 0), Some(Block::new(wall)));
            let ladder = state(&world, (0, 1, 1), "minecraft:ladder", &side);
            assert_eq!(ladder.is_some(), placed, "{wall}");
        }
    }

    #[test]
    fn a_ladder_cannot_be_placed_on_the_front_of_another_ladder() {
        let mut world = Blocks::default();
        world.set_block((0, 1, 0), Some(Block::new("minecraft:stone")));
        world.set_block(
            (0, 1, 1),
            Some(Block::new("minecraft:ladder").with("facing", "south")),
        );
        let front = view(180.0, 10.0, Face::South);
        assert!(state(&world, (0, 1, 2), "minecraft:ladder", &front).is_none());
        let side = view(-90.0, 10.0, Face::West);
        assert!(state(&world, (-1, 1, 1), "minecraft:ladder", &side).is_none());
        world.set_block((-1, 1, 0), Some(Block::new("minecraft:stone")));
        let (beside, _) = state(&world, (-1, 1, 1), "minecraft:ladder", &side).unwrap();
        assert_eq!(beside.property("facing"), Some("south"));
    }

    #[test]
    fn hanging_signs_hang_from_ceilings_and_walls() {
        let mut world = Blocks::default();
        world.set_block((0, 3, 0), Some(Block::new("minecraft:stone")));
        let below = view(90.0, -60.0, Face::Down);
        let (ceiling, _) = state(&world, (0, 2, 0), "minecraft:oak_hanging_sign", &below).unwrap();
        assert_eq!(ceiling.id, "minecraft:oak_hanging_sign");
        assert_eq!(ceiling.property("attached"), Some("false"));
        assert_eq!(ceiling.property("rotation"), Some("12"));
        world.set_block((1, 1, 0), Some(Block::new("minecraft:stone")));
        let side = view(-90.0, 0.0, Face::West);
        let (wall, _) = state(&world, (0, 1, 0), "minecraft:oak_hanging_sign", &side).unwrap();
        assert_eq!(wall.id, "minecraft:oak_wall_hanging_sign");
        assert_eq!(wall.property("facing"), Some("north"));
    }

    #[test]
    fn beds_chests_pots_shulkers_lecterns_and_bells_take_their_vanilla_states() {
        let mut world = Blocks::default();
        world.set_block((0, 0, 0), Some(Block::new("minecraft:stone")));
        let north = view(180.0, 30.0, Face::Up);
        let (foot, head) = state(&world, (0, 1, 0), "minecraft:red_bed", &north).unwrap();
        assert_eq!(
            (foot.property("facing"), foot.property("part")),
            (Some("north"), Some("foot"))
        );
        let (head_pos, head) = head.unwrap();
        assert_eq!(head_pos, (0, 1, -1));
        assert_eq!(head.property("part"), Some("head"));
        world.set_block((0, 1, -1), Some(Block::new("minecraft:stone")));
        assert!(state(&world, (0, 1, 0), "minecraft:red_bed", &north).is_none());
        let (ender, _) = state(&world, (0, 1, 0), "minecraft:ender_chest", &north).unwrap();
        assert_eq!(ender.property("facing"), Some("south"));
        let (pot, _) = state(&world, (0, 1, 0), "minecraft:decorated_pot", &north).unwrap();
        assert_eq!(pot.property("facing"), Some("north"));
        let (lectern, _) = state(&world, (0, 1, 0), "minecraft:lectern", &north).unwrap();
        assert_eq!(lectern.property("facing"), Some("south"));
        let side = view(180.0, 0.0, Face::West);
        let (shulker, _) = state(&world, (0, 1, 0), "minecraft:blue_shulker_box", &side).unwrap();
        assert_eq!(shulker.property("facing"), Some("west"));
        let (bell, _) = state(&world, (0, 1, 0), "minecraft:bell", &north).unwrap();
        assert_eq!(
            (bell.property("attachment"), bell.property("facing")),
            (Some("floor"), Some("north"))
        );
        world.set_block((1, 1, 0), Some(Block::new("minecraft:stone")));
        world.set_block((-1, 1, 0), Some(Block::new("minecraft:stone")));
        let (between, _) = state(&world, (0, 1, 0), "minecraft:bell", &side).unwrap();
        assert_eq!(
            (between.property("attachment"), between.property("facing")),
            (Some("double_wall"), Some("east"))
        );
        assert!(pairs_as_chest("minecraft:waxed_oxidized_copper_chest"));
        assert!(!pairs_as_chest("minecraft:ender_chest"));
        assert!(chests_connect(
            "minecraft:copper_chest",
            "minecraft:waxed_weathered_copper_chest"
        ));
        assert!(!chests_connect(
            "minecraft:chest",
            "minecraft:trapped_chest"
        ));
        assert_eq!(
            joined_chest(
                "minecraft:waxed_oxidized_copper_chest",
                "minecraft:waxed_exposed_copper_chest"
            ),
            "minecraft:waxed_exposed_copper_chest"
        );
        assert_eq!(
            joined_chest(
                "minecraft:waxed_copper_chest",
                "minecraft:weathered_copper_chest"
            ),
            "minecraft:copper_chest"
        );
    }
}
