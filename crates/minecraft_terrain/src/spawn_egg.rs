use crate::bucket::{aim, offset};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::{Block, Face, Player, Pos, World};
use std::collections::VecDeque;

pub const SPAWN_EGG_SUFFIX: &str = "_spawn_egg";
const FALLBACK_WIDTH: f64 = 0.6;
const EPSILON: f64 = 1.0e-7;
const SPAWNERS: [&str; 2] = ["minecraft:spawner", "minecraft:trial_spawner"];
const LIQUIDS: [&str; 2] = ["minecraft:water", "minecraft:lava"];

#[derive(Clone, Debug, PartialEq)]
pub enum EggUse {
    Spawner(Pos),
    Spawn { kind: String, position: [f64; 3] },
}

#[derive(Default)]
pub struct EggsWaiting(VecDeque<(usize, String, bool)>);

impl EggsWaiting {
    pub fn send(&mut self, inventory: &Inventory, slot: usize) -> bool {
        self.send_from(inventory, slot, true)
    }

    pub fn send_from(&mut self, inventory: &Inventory, slot: usize, spends: bool) -> bool {
        let Some(egg) = inventory.slots[slot].as_ref() else {
            return false;
        };
        let waiting = self.0.iter().filter(|(at, ..)| *at == slot).count();
        if usize::from(egg.count) <= waiting {
            return false;
        }
        self.0.push_back((slot, egg.id.clone(), spends));
        true
    }

    pub fn answer(&mut self, inventory: &mut Inventory, hatched: bool) -> Option<String> {
        let (slot, egg, spends) = self.0.pop_front()?;
        if hatched && spends {
            let slots = &mut inventory.slots;
            let holds = |stack: &Option<ItemStack>| stack.as_ref().is_some_and(|s| s.id == egg);
            let index = if holds(&slots[slot]) {
                Some(slot)
            } else {
                slots.iter().position(holds)
            };
            if let Some(index) = index {
                let left = slots[index].as_mut().map(|stack| {
                    stack.count -= 1;
                    stack.count
                });
                if left == Some(0) {
                    slots[index] = None;
                }
            }
        }
        Some(egg)
    }
}

pub fn spawned_kind(item: &str) -> Option<String> {
    let kind = item.strip_suffix(SPAWN_EGG_SUFFIX)?;
    let name = kind.rsplit(':').next().unwrap_or(kind);
    (!name.is_empty()).then(|| kind.to_owned())
}

pub fn is_spawner(block: &Block) -> bool {
    SPAWNERS.contains(&block.id.as_str())
}

pub fn egg_use(player: &Player, world: &impl World, item: &str) -> Option<EggUse> {
    let kind = spawned_kind(item)?;
    match aim(player, world, false) {
        Some(hit) => {
            if world.block(hit.pos).is_some_and(|block| is_spawner(&block)) {
                return Some(EggUse::Spawner(hit.pos));
            }
            let at = if has_collision(world, hit.pos) {
                offset(hit.pos, hit.face.offset())
            } else {
                hit.pos
            };
            let move_down = at != hit.pos && hit.face == Face::Up;
            let width = mob_width(&kind);
            let rise = spawn_rise(world, at, move_down, width);
            Some(EggUse::Spawn {
                kind,
                position: [
                    f64::from(at.0) + 0.5,
                    f64::from(at.1) + rise,
                    f64::from(at.2) + 0.5,
                ],
            })
        }
        None => {
            let hit = aim(player, world, true)?;
            let liquid = world
                .block(hit.pos)
                .is_some_and(|block| LIQUIDS.contains(&block.id.as_str()));
            liquid.then(|| EggUse::Spawn {
                kind,
                position: [
                    f64::from(hit.pos.0) + 0.5,
                    f64::from(hit.pos.1),
                    f64::from(hit.pos.2) + 0.5,
                ],
            })
        }
    }
}

fn mob_width(kind: &str) -> f64 {
    minecraftoss_world::natural_spawner::type_width(kind)
        .or_else(|| {
            (kind == "minecraft:iron_golem").then_some(minecraftoss_entities::iron_golem::WIDTH)
        })
        .or_else(|| {
            (kind == "minecraft:cave_spider").then(|| {
                minecraftoss_entities::spider::Spider::cave(glam::DVec3::ZERO)
                    .body
                    .width
            })
        })
        .map_or(FALLBACK_WIDTH, f64::from)
}

fn solid(b: &[f64; 6]) -> bool {
    b[3] > b[0] && b[4] > b[1] && b[5] > b[2]
}

fn has_collision(world: &impl World, pos: Pos) -> bool {
    world.collision_boxes(pos).iter().any(solid)
}

fn overlaps(a: &[f64; 6], b: &[f64; 6]) -> bool {
    (0..3).all(|axis| a[axis] < b[axis + 3] && b[axis] < a[axis + 3])
}

pub fn spawn_rise(world: &impl World, at: Pos, move_down: bool, width: f64) -> f64 {
    let (x, y, z) = (f64::from(at.0), f64::from(at.1), f64::from(at.2));
    let drop = if move_down { 2.0 } else { 1.0 };
    let region = [x, y + 1.0 - drop, z, x + 1.0, y + 1.0, z + 1.0];
    let half = width / 2.0;
    let feet = y + 1.0;
    let footprint = [
        x + 0.5 - half,
        z + 0.5 - half,
        x + 0.5 + half,
        z + 0.5 + half,
    ];
    let mut travel = -drop;
    let lowest = at.1 - if move_down { 2 } else { 1 } - 1;
    for cy in lowest..=at.1 {
        for cx in at.0 - 1..=at.0 + 1 {
            for cz in at.2 - 1..=at.2 + 1 {
                let corner = [f64::from(cx), f64::from(cy), f64::from(cz)];
                for local in world.collision_boxes((cx, cy, cz)) {
                    let b = [
                        local[0] + corner[0],
                        local[1] + corner[1],
                        local[2] + corner[2],
                        local[3] + corner[0],
                        local[4] + corner[1],
                        local[5] + corner[2],
                    ];
                    let under = b[0] < footprint[2] - EPSILON
                        && b[3] > footprint[0] + EPSILON
                        && b[2] < footprint[3] - EPSILON
                        && b[5] > footprint[1] + EPSILON;
                    if solid(&b) && overlaps(&b, &region) && under && b[4] <= feet + EPSILON {
                        travel = travel.max(b[4] - feet);
                    }
                }
            }
        }
    }
    1.0 + travel
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use std::collections::HashMap;

    const FULL: [f64; 6] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    const BOTTOM_SLAB: [f64; 6] = [0.0, 0.0, 0.0, 1.0, 0.5, 1.0];
    const FENCE_POST: [f64; 6] = [0.375, 0.0, 0.375, 0.625, 1.5, 0.625];
    const LADDER_EAST: [f64; 6] = [0.0, 0.0, 0.0, 0.1875, 1.0, 1.0];

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

        fn collision_boxes(&self, pos: Pos) -> Vec<[f64; 6]> {
            match self.block(pos).as_ref().map(|b| b.id.as_str()) {
                Some("minecraft:stone" | "minecraft:spawner") => vec![FULL],
                Some("minecraft:smooth_stone_slab") => vec![BOTTOM_SLAB],
                Some("minecraft:oak_fence") => vec![FENCE_POST],
                Some("minecraft:ladder") => vec![LADDER_EAST],
                Some("minecraft:snow") => vec![[0.0, 0.0, 0.0, 1.0, 0.0, 1.0]],
                _ => Vec::new(),
            }
        }
    }

    fn world(blocks: &[(Pos, &str)]) -> Blocks {
        let mut world = Blocks::default();
        for &(pos, id) in blocks {
            world.set_block(pos, Some(Block::new(id)));
        }
        world
    }

    fn floor(y: i32) -> Vec<(Pos, &'static str)> {
        let mut blocks = Vec::new();
        for x in -3..=3 {
            for z in -3..=3 {
                blocks.push(((x, y, z), "minecraft:stone"));
            }
        }
        blocks
    }

    fn looking_down_at(x: i32, z: i32, eye_y: f64) -> Player {
        let mut player = Player::new(DVec3::new(
            f64::from(x) + 0.5,
            eye_y - 1.62,
            f64::from(z) + 0.5,
        ));
        player.pitch = 90.0;
        player
    }

    fn looking_east(eye: DVec3) -> Player {
        let mut player = Player::new(eye - DVec3::new(0.0, 1.62, 0.0));
        player.yaw = -90.0;
        player.pitch = 0.0;
        player
    }

    fn spawned(use_: Option<EggUse>) -> (String, [f64; 3]) {
        match use_ {
            Some(EggUse::Spawn { kind, position }) => (kind, position),
            other => panic!("expected a spawn, got {other:?}"),
        }
    }

    fn eggs(stacks: &[(usize, u8)]) -> Inventory {
        let mut inventory = Inventory::default();
        for &(slot, count) in stacks {
            inventory.slots[slot] = Some(ItemStack::new("minecraft:zombie_spawn_egg", count));
        }
        inventory
    }

    #[test]
    fn a_stack_sends_no_more_eggs_than_it_holds() {
        let inventory = eggs(&[(0, 2)]);
        let mut waiting = EggsWaiting::default();
        assert!(waiting.send(&inventory, 0));
        assert!(waiting.send(&inventory, 0));
        assert!(!waiting.send(&inventory, 0));
        assert!(!waiting.send(&inventory, 1));
    }

    #[test]
    fn a_hatched_egg_is_used_and_a_failed_one_kept() {
        let mut inventory = eggs(&[(0, 2)]);
        let mut waiting = EggsWaiting::default();
        waiting.send(&inventory, 0);
        waiting.send(&inventory, 0);
        assert_eq!(
            waiting.answer(&mut inventory, false).as_deref(),
            Some("minecraft:zombie_spawn_egg")
        );
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(2));
        waiting.answer(&mut inventory, true);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(1));
        assert_eq!(waiting.answer(&mut inventory, true), None);
    }

    #[test]
    fn a_creative_egg_hatches_without_being_used() {
        let mut inventory = eggs(&[(0, 1)]);
        let mut waiting = EggsWaiting::default();
        assert!(waiting.send_from(&inventory, 0, false));
        assert!(!waiting.send_from(&inventory, 0, false));
        assert_eq!(
            waiting.answer(&mut inventory, true).as_deref(),
            Some("minecraft:zombie_spawn_egg")
        );
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(1));
    }

    #[test]
    fn the_last_egg_empties_its_slot_or_the_slot_it_moved_to() {
        let mut inventory = eggs(&[(0, 1)]);
        let mut waiting = EggsWaiting::default();
        waiting.send(&inventory, 0);
        waiting.answer(&mut inventory, true);
        assert!(inventory.slots[0].is_none());
        let mut inventory = eggs(&[(0, 1)]);
        waiting.send(&inventory, 0);
        inventory.slots.swap(0, 5);
        waiting.answer(&mut inventory, true);
        assert!(inventory.slots[0].is_none());
        assert!(inventory.slots[5].is_none());
    }

    #[test]
    fn egg_names_the_mob_it_hatches() {
        assert_eq!(
            spawned_kind("minecraft:zombie_spawn_egg").as_deref(),
            Some("minecraft:zombie")
        );
        assert_eq!(
            spawned_kind("minecraft:iron_golem_spawn_egg").as_deref(),
            Some("minecraft:iron_golem")
        );
        assert_eq!(spawned_kind("minecraft:egg"), None);
        assert_eq!(spawned_kind("minecraft:_spawn_egg"), None);
    }

    #[test]
    fn eggs_on_a_floor_spawn_on_top_of_the_clicked_block() {
        let world = world(&floor(0));
        let player = looking_down_at(0, 0, 3.0);
        let (kind, position) = spawned(egg_use(&player, &world, "minecraft:cow_spawn_egg"));
        assert_eq!(kind, "minecraft:cow");
        assert_eq!(position, [0.5, 1.0, 0.5]);
    }

    #[test]
    fn blocks_without_collision_take_the_mob_in_their_own_cell() {
        let mut blocks = floor(0);
        blocks.push(((0, 1, 0), "minecraft:short_grass"));
        let world = world(&blocks);
        let player = looking_down_at(0, 0, 3.0);
        let (_, position) = spawned(egg_use(&player, &world, "minecraft:zombie_spawn_egg"));
        assert_eq!(position, [0.5, 1.0, 0.5]);
    }

    #[test]
    fn a_wall_face_spawns_beside_it_without_dropping_more_than_one_block() {
        let mut blocks = floor(0);
        blocks.push(((2, 1, 0), "minecraft:stone"));
        blocks.push(((2, 2, 0), "minecraft:stone"));
        let world = world(&blocks);
        let player = looking_east(DVec3::new(0.5, 2.5, 0.5));
        let (_, position) = spawned(egg_use(&player, &world, "minecraft:pig_spawn_egg"));
        assert_eq!(position, [1.5, 2.0, 0.5]);
    }

    #[test]
    fn a_wall_face_over_a_slab_stands_the_mob_on_the_slab() {
        let mut blocks = floor(0);
        blocks.push(((2, 1, 0), "minecraft:stone"));
        blocks.push(((1, 1, 0), "minecraft:smooth_stone_slab"));
        let world = world(&blocks);
        let player = looking_east(DVec3::new(0.5, 1.75, 0.5));
        let (_, position) = spawned(egg_use(&player, &world, "minecraft:pig_spawn_egg"));
        assert_eq!(position, [1.5, 1.5, 0.5]);
    }

    #[test]
    fn the_top_of_a_slab_seats_the_mob_on_the_slab() {
        let mut blocks = floor(0);
        blocks.push(((0, 1, 0), "minecraft:smooth_stone_slab"));
        let world = world(&blocks);
        let player = looking_down_at(0, 0, 3.5);
        let (_, position) = spawned(egg_use(&player, &world, "minecraft:creeper_spawn_egg"));
        assert_eq!(position, [0.5, 1.5, 0.5]);
    }

    #[test]
    fn the_top_of_a_fence_seats_the_mob_on_the_post() {
        let mut blocks = floor(0);
        blocks.push(((0, 1, 0), "minecraft:oak_fence"));
        let world = world(&blocks);
        let player = looking_down_at(0, 0, 4.0);
        let (_, position) = spawned(egg_use(&player, &world, "minecraft:skeleton_spawn_egg"));
        assert_eq!(position, [0.5, 2.5, 0.5]);
    }

    #[test]
    fn rise_follows_entity_type_get_y_offset() {
        let mut blocks = floor(0);
        blocks.push(((5, 1, 5), "minecraft:smooth_stone_slab"));
        let world = world(&blocks);
        assert_eq!(spawn_rise(&world, (0, 1, 0), false, 0.6), 0.0);
        assert_eq!(spawn_rise(&world, (0, 1, 0), true, 0.6), 0.0);
        assert_eq!(spawn_rise(&world, (0, 2, 0), false, 0.6), 0.0);
        assert_eq!(spawn_rise(&world, (0, 2, 0), true, 0.6), -1.0);
        assert_eq!(spawn_rise(&world, (5, 1, 5), false, 0.6), 0.5);
        assert_eq!(spawn_rise(&world, (5, 2, 5), true, 0.6), -0.5);
    }

    #[test]
    fn only_wide_mobs_land_on_a_ladder_edge() {
        let world = world(&[((0, 0, 0), "minecraft:ladder")]);
        assert_eq!(spawn_rise(&world, (0, 1, 0), true, 1.4), 0.0);
        assert_eq!(spawn_rise(&world, (0, 1, 0), true, 0.6), -1.0);
    }

    #[test]
    fn a_post_rising_through_the_feet_does_not_lift_the_mob() {
        let mut blocks = floor(0);
        blocks.push(((0, 1, 0), "minecraft:oak_fence"));
        let world = world(&blocks);
        assert_eq!(spawn_rise(&world, (0, 1, 0), true, 0.6), 0.0);
        assert_eq!(spawn_rise(&world, (0, 1, 0), false, 0.6), 0.0);
    }

    #[test]
    fn an_iron_golem_is_as_wide_as_its_body() {
        assert!((mob_width("minecraft:iron_golem") - 1.4).abs() < 1.0e-6);
        assert!((mob_width("minecraft:cow") - 0.9).abs() < 1.0e-6);
        assert!((mob_width("minecraft:cave_spider") - 0.7).abs() < 1.0e-6);
        assert_eq!(mob_width("minecraft:villager"), FALLBACK_WIDTH);
    }

    #[test]
    fn shapes_of_neighbouring_cells_outside_the_cell_are_ignored() {
        let world = world(&[((1, 0, 0), "minecraft:oak_fence")]);
        assert_eq!(spawn_rise(&world, (0, 1, 0), true, 1.4), -1.0);
    }

    #[test]
    fn zero_height_snow_counts_as_no_collision() {
        let mut blocks = floor(0);
        blocks.push(((0, 1, 0), "minecraft:snow"));
        let world = world(&blocks);
        assert!(!has_collision(&world, (0, 1, 0)));
    }

    #[test]
    fn spawners_are_retargeted_not_hatched_beside() {
        let mut blocks = floor(0);
        blocks.push(((0, 1, 0), "minecraft:spawner"));
        let world = world(&blocks);
        let player = looking_down_at(0, 0, 4.0);
        assert_eq!(
            egg_use(&player, &world, "minecraft:zombie_spawn_egg"),
            Some(EggUse::Spawner((0, 1, 0)))
        );
    }

    #[test]
    fn eggs_aimed_at_a_liquid_source_hatch_in_it() {
        let world = world(&[((0, 0, 0), "minecraft:water")]);
        let player = looking_down_at(0, 0, 2.5);
        let (kind, position) = spawned(egg_use(&player, &world, "minecraft:glow_squid_spawn_egg"));
        assert_eq!(kind, "minecraft:glow_squid");
        assert_eq!(position, [0.5, 0.0, 0.5]);
    }

    #[test]
    fn nothing_in_reach_or_other_items_do_nothing() {
        let player = looking_down_at(0, 0, 3.0);
        let far = world(&floor(-10));
        assert_eq!(egg_use(&player, &far, "minecraft:zombie_spawn_egg"), None);
        let near = world(&floor(0));
        assert_eq!(egg_use(&player, &near, "minecraft:egg"), None);
    }
}
