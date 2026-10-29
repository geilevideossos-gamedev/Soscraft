use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::{Player, Pos, World};
use serde_json::{Map, Value, json};

use crate::bucket::{Change, ItemUse, Sound};
use crate::terrain::Dimension;

pub const COMPASS: &str = "minecraft:compass";
pub const LODESTONE: &str = "minecraft:lodestone";
pub const TRACKER: &str = "minecraft:lodestone_tracker";
pub const LOCK_SOUND: &str = "minecraft:item.lodestone_compass.lock";
pub const NAME_KEY: &str = "item.minecraft.lodestone_compass";
const USE_REACH: f64 = 4.5;
const WORLD_HORIZONTAL_LIMIT: i32 = 30_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub dimension: String,
    pub pos: Pos,
}

impl Target {
    pub fn new(dimension: Dimension, pos: Pos) -> Self {
        Self {
            dimension: dimension.dimension_type().to_owned(),
            pos,
        }
    }

    pub fn in_dimension(&self, dimension: Dimension) -> bool {
        Dimension::parse(&self.dimension) == Some(dimension)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracker {
    pub target: Option<Target>,
    pub tracked: bool,
}

fn flag(value: &Value) -> Option<bool> {
    value.as_bool().or_else(|| value.as_i64().map(|v| v != 0))
}

fn read_target(value: &Value) -> Option<Target> {
    let dimension = value.get("dimension")?.as_str()?.to_owned();
    let pos = value.get("pos")?.as_array()?;
    if pos.len() != 3 {
        return None;
    }
    let coord = |i: usize| pos[i].as_i64().and_then(|v| i32::try_from(v).ok());
    Some(Target {
        dimension,
        pos: (coord(0)?, coord(1)?, coord(2)?),
    })
}

pub fn is_lodestone_compass(stack: &ItemStack) -> bool {
    stack.id == COMPASS
        && stack
            .components
            .as_ref()
            .is_some_and(|components| components.get(TRACKER).is_some())
}

pub fn tracker(stack: &ItemStack) -> Option<Tracker> {
    if stack.id != COMPASS {
        return None;
    }
    let value = stack.components.as_ref()?.get(TRACKER)?;
    Some(Tracker {
        target: value.get("target").and_then(read_target),
        tracked: value.get("tracked").and_then(flag).unwrap_or(true),
    })
}

pub fn name_key(stack: &ItemStack) -> Option<&'static str> {
    is_lodestone_compass(stack).then_some(NAME_KEY)
}

pub fn set_tracker(stack: &mut ItemStack, tracker: &Tracker) {
    let mut fields = Map::new();
    if let Some(target) = &tracker.target {
        let (x, y, z) = target.pos;
        fields.insert(
            "target".to_owned(),
            json!({"dimension": target.dimension, "pos": [x, y, z]}),
        );
    }
    if !tracker.tracked {
        fields.insert("tracked".to_owned(), Value::Bool(false));
    }
    let components = stack
        .components
        .get_or_insert_with(|| Value::Object(Map::new()));
    if !components.is_object() {
        *components = Value::Object(Map::new());
    }
    if let Some(map) = components.as_object_mut() {
        map.insert(TRACKER.to_owned(), Value::Object(fields));
    }
}

pub fn with_tracker(stack: &ItemStack, target: Target) -> ItemStack {
    let mut copy = stack.clone();
    copy.id = COMPASS.to_owned();
    copy.count = 1;
    set_tracker(
        &mut copy,
        &Tracker {
            target: Some(target),
            tracked: true,
        },
    );
    copy
}

pub fn lock_use(player: &Player, world: &impl World, held: &ItemStack) -> Option<ItemUse> {
    if held.id != COMPASS {
        return None;
    }
    let hit = player.target(world, USE_REACH)?;
    if world.block(hit.pos)?.id != LODESTONE {
        return None;
    }
    Some(ItemUse {
        change: Change::Lodestone(hit.pos),
        result: None,
        wear: 0,
        consumed: false,
        sounds: vec![Sound::at_block(LOCK_SOUND, hit.pos)],
    })
}

pub fn lock(
    inventory: &mut Inventory,
    (slot, selected): (usize, usize),
    target: Target,
    infinite: bool,
) -> Option<ItemStack> {
    let held = inventory
        .slots
        .get_mut(slot)?
        .as_mut()
        .filter(|held| held.id == COMPASS)?;
    if !infinite && held.count == 1 {
        set_tracker(
            held,
            &Tracker {
                target: Some(target),
                tracked: true,
            },
        );
        return None;
    }
    let copy = with_tracker(held, target);
    if !infinite {
        held.count -= 1;
    }
    inventory.add_item(copy, selected)
}

pub fn tracked_targets(inventory: &Inventory, dimension: Dimension) -> Vec<Pos> {
    let mut targets: Vec<Pos> = inventory
        .slots
        .iter()
        .flatten()
        .filter_map(tracker)
        .filter(|tracker| tracker.tracked)
        .filter_map(|tracker| tracker.target)
        .filter(|target| target.in_dimension(dimension))
        .map(|target| target.pos)
        .collect();
    targets.sort_unstable();
    targets.dedup();
    targets
}

pub fn forget(inventory: &mut Inventory, dimension: Dimension, pos: Pos) -> bool {
    let mut forgot = false;
    for stack in inventory.slots.iter_mut().flatten() {
        let Some(tracker) = tracker(stack) else {
            continue;
        };
        let aimed = tracker
            .target
            .as_ref()
            .is_some_and(|target| target.in_dimension(dimension) && target.pos == pos);
        if tracker.tracked && aimed {
            set_tracker(
                stack,
                &Tracker {
                    target: None,
                    tracked: true,
                },
            );
            forgot = true;
        }
    }
    forgot
}

pub fn in_world_bounds((x, y, z): Pos, min_y: i32, max_y: i32) -> bool {
    (min_y..=max_y).contains(&y)
        && (-WORLD_HORIZONTAL_LIMIT..WORLD_HORIZONTAL_LIMIT).contains(&x)
        && (-WORLD_HORIZONTAL_LIMIT..WORLD_HORIZONTAL_LIMIT).contains(&z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use minecraftoss_player::Block;
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

    fn below(id: &str) -> Blocks {
        let mut world = Blocks::default();
        world.set_block((0, 0, 0), Some(Block::new(id)));
        world
    }

    fn looking_down() -> Player {
        let mut player = Player::new(DVec3::new(0.5, 1.0, 0.5));
        player.pitch = 90.0;
        player
    }

    fn holding(count: u8) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new(COMPASS, count));
        inventory
    }

    fn overworld(pos: Pos) -> Target {
        Target::new(Dimension::Overworld, pos)
    }

    fn compasses(inventory: &Inventory) -> Vec<(usize, u8, Option<Target>)> {
        inventory
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, stack)| {
                let stack = stack.as_ref().filter(|stack| stack.id == COMPASS)?;
                Some((slot, stack.count, tracker(stack).and_then(|t| t.target)))
            })
            .collect()
    }

    #[test]
    fn a_compass_on_a_lodestone_locks_with_its_sound_and_on_stone_does_nothing() {
        let compass = ItemStack::new(COMPASS, 8);
        let used = lock_use(&looking_down(), &below(LODESTONE), &compass).expect("a lodestone");
        assert_eq!(used.change, Change::Lodestone((0, 0, 0)));
        assert!(!used.consumed && used.result.is_none() && used.wear == 0);
        assert_eq!(used.sounds, vec![Sound::at_block(LOCK_SOUND, (0, 0, 0))]);
        assert_eq!(
            lock_use(&looking_down(), &below("minecraft:stone"), &compass),
            None
        );
        let clock = ItemStack::new("minecraft:clock", 1);
        assert_eq!(lock_use(&looking_down(), &below(LODESTONE), &clock), None);
        let recovery = ItemStack::new("minecraft:recovery_compass", 1);
        assert_eq!(
            lock_use(&looking_down(), &below(LODESTONE), &recovery),
            None
        );
    }

    #[test]
    fn a_single_compass_tracks_in_place() {
        let mut inventory = holding(1);
        assert_eq!(
            lock(&mut inventory, (0, 0), overworld((4, 64, -2)), false),
            None
        );
        assert_eq!(
            compasses(&inventory),
            [(0, 1, Some(overworld((4, 64, -2))))]
        );
        let stack = inventory.slots[0].as_ref().unwrap();
        assert_eq!(
            stack.components,
            Some(
                json!({TRACKER: {"target": {"dimension": "minecraft:overworld", "pos": [4, 64, -2]}}})
            )
        );
        assert!(crate::glint::has_glint(stack));
        assert_eq!(name_key(stack), Some(NAME_KEY));
        assert_eq!(name_key(&ItemStack::new(COMPASS, 1)), None);
    }

    #[test]
    fn a_stack_of_three_leaves_two_plain_and_adds_one_tracked() {
        let mut inventory = holding(3);
        inventory.slots[1] = Some(ItemStack::new("minecraft:stone", 64));
        assert_eq!(
            lock(&mut inventory, (0, 0), overworld((1, 2, 3)), false),
            None
        );
        assert_eq!(
            compasses(&inventory),
            [(0, 2, None), (2, 1, Some(overworld((1, 2, 3))))]
        );
    }

    #[test]
    fn a_full_inventory_hands_the_tracked_compass_back_to_drop() {
        let mut inventory = holding(3);
        for slot in (1..36).chain([40]) {
            inventory.slots[slot] = Some(ItemStack::new("minecraft:stone", 64));
        }
        let dropped =
            lock(&mut inventory, (0, 0), overworld((1, 2, 3)), false).expect("no room for it");
        assert_eq!((dropped.id.as_str(), dropped.count), (COMPASS, 1));
        assert_eq!(
            tracker(&dropped).and_then(|t| t.target),
            Some(overworld((1, 2, 3)))
        );
        assert_eq!(compasses(&inventory), [(0, 2, None)]);
    }

    #[test]
    fn creative_keeps_every_compass_and_adds_a_tracked_one() {
        let mut inventory = holding(3);
        assert_eq!(
            lock(&mut inventory, (0, 0), overworld((1, 2, 3)), true),
            None
        );
        assert_eq!(
            compasses(&inventory),
            [(0, 3, None), (1, 1, Some(overworld((1, 2, 3))))]
        );
        let mut single = holding(1);
        lock(&mut single, (0, 0), overworld((1, 2, 3)), true);
        assert_eq!(
            compasses(&single),
            [(0, 1, None), (1, 1, Some(overworld((1, 2, 3))))]
        );
    }

    #[test]
    fn relinking_moves_the_target_and_keeps_other_components() {
        let mut inventory = holding(1);
        inventory.slots[0].as_mut().unwrap().components =
            Some(json!({"minecraft:custom_name": "Home"}));
        lock(&mut inventory, (0, 0), overworld((1, 2, 3)), false);
        lock(
            &mut inventory,
            (0, 0),
            Target::new(Dimension::Nether, (8, 40, 8)),
            false,
        );
        let stack = inventory.slots[0].as_ref().unwrap();
        assert_eq!(
            tracker(stack),
            Some(Tracker {
                target: Some(Target::new(Dimension::Nether, (8, 40, 8))),
                tracked: true
            })
        );
        assert_eq!(
            stack.components.as_ref().unwrap()["minecraft:custom_name"],
            "Home"
        );
        let mut two = holding(2);
        lock(&mut two, (0, 0), overworld((1, 2, 3)), false);
        let tracked = two.slots[1].clone().unwrap();
        two.slots[1] = None;
        two.slots[0] = Some(ItemStack {
            count: 2,
            ..tracked
        });
        lock(&mut two, (0, 0), overworld((9, 9, 9)), false);
        assert_eq!(
            compasses(&two),
            [
                (0, 1, Some(overworld((1, 2, 3)))),
                (1, 1, Some(overworld((9, 9, 9))))
            ]
        );
        let mut stone = Inventory::default();
        stone.slots[0] = Some(ItemStack::new("minecraft:stone", 1));
        assert_eq!(lock(&mut stone, (0, 0), overworld((1, 2, 3)), false), None);
        assert!(stone.slots[0].as_ref().unwrap().components.is_none());
    }

    #[test]
    fn forget_clears_the_target_but_keeps_tracking_the_name_and_the_glint() {
        let mut inventory = holding(1);
        lock(&mut inventory, (0, 0), overworld((1, 2, 3)), false);
        inventory.slots[5] = Some(with_tracker(
            &ItemStack::new(COMPASS, 1),
            Target::new(Dimension::Nether, (1, 2, 3)),
        ));
        assert!(!forget(&mut inventory, Dimension::Overworld, (1, 2, 4)));
        assert!(forget(&mut inventory, Dimension::Overworld, (1, 2, 3)));
        let stack = inventory.slots[0].as_ref().unwrap();
        assert_eq!(
            tracker(stack),
            Some(Tracker {
                target: None,
                tracked: true
            })
        );
        assert_eq!(stack.components, Some(json!({TRACKER: {}})));
        assert!(crate::glint::has_glint(stack));
        assert_eq!(name_key(stack), Some(NAME_KEY));
        assert_eq!(
            compasses(&inventory)[1].2,
            Some(Target::new(Dimension::Nether, (1, 2, 3))),
            "the other dimension's lodestone is kept"
        );
        assert!(!forget(&mut inventory, Dimension::Overworld, (1, 2, 3)));
    }

    #[test]
    fn only_tracked_targets_in_the_players_dimension_are_checked() {
        let mut inventory = Inventory::default();
        let plain = ItemStack::new(COMPASS, 1);
        inventory.slots[0] = Some(with_tracker(&plain, overworld((1, 2, 3))));
        inventory.slots[1] = Some(with_tracker(&plain, overworld((1, 2, 3))));
        inventory.slots[2] = Some(with_tracker(&plain, overworld((-5, 70, 9))));
        inventory.slots[3] = Some(with_tracker(
            &plain,
            Target::new(Dimension::End, (0, 60, 0)),
        ));
        let mut untracked = with_tracker(&plain, overworld((7, 7, 7)));
        set_tracker(
            &mut untracked,
            &Tracker {
                target: Some(overworld((7, 7, 7))),
                tracked: false,
            },
        );
        inventory.slots[4] = Some(untracked);
        inventory.slots[40] = Some(with_tracker(&plain, overworld((0, 0, 0))));
        assert_eq!(
            tracked_targets(&inventory, Dimension::Overworld),
            [(-5, 70, 9), (0, 0, 0), (1, 2, 3)]
        );
        assert_eq!(tracked_targets(&inventory, Dimension::End), [(0, 60, 0)]);
        assert!(tracked_targets(&inventory, Dimension::Nether).is_empty());
        assert!(!forget(&mut inventory, Dimension::Overworld, (7, 7, 7)));
    }

    #[test]
    fn a_tracker_reads_vanillas_forms() {
        let stack = |value: Value| ItemStack {
            components: Some(json!({ TRACKER: value })),
            ..ItemStack::new(COMPASS, 1)
        };
        let saved = stack(
            json!({"target": {"dimension": "minecraft:the_nether", "pos": [1, -2, 3]}, "tracked": 0}),
        );
        assert_eq!(
            tracker(&saved),
            Some(Tracker {
                target: Some(Target::new(Dimension::Nether, (1, -2, 3))),
                tracked: false
            })
        );
        assert_eq!(
            tracker(&stack(json!({"tracked": true}))),
            Some(Tracker {
                target: None,
                tracked: true
            })
        );
        assert_eq!(
            tracker(&stack(
                json!({"target": {"dimension": "minecraft:overworld", "pos": [1, 2]}})
            )),
            Some(Tracker {
                target: None,
                tracked: true
            })
        );
        let mut clock = stack(json!({}));
        clock.id = "minecraft:clock".to_owned();
        assert_eq!(tracker(&clock), None);
        assert_eq!(name_key(&clock), None);
    }

    #[test]
    fn a_lodestone_outside_the_build_height_or_the_border_is_out_of_bounds() {
        assert!(in_world_bounds((0, -64, 0), -64, 319));
        assert!(in_world_bounds((29_999_999, 319, -30_000_000), -64, 319));
        assert!(!in_world_bounds((0, 320, 0), -64, 319));
        assert!(!in_world_bounds((0, -65, 0), -64, 319));
        assert!(!in_world_bounds((30_000_000, 64, 0), -64, 319));
        assert!(!in_world_bounds((0, 64, -30_000_001), -64, 319));
    }
}
