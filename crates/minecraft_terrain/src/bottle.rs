use minecraftoss_player::food::{FoodInfo, FoodUse};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::{Block, Face, Player, Pos, World};
use serde_json::{Value, json};

pub const GLASS_BOTTLE: &str = "minecraft:glass_bottle";
pub const POTION: &str = "minecraft:potion";
pub const FILL_SOUND: &str = "minecraft:item.bottle.fill";
pub const EMPTY_SOUND: &str = "minecraft:item.bottle.empty";
pub const SPLASH_SOUND: &str = "minecraft:entity.generic.splash";
pub const WATER_NAME_KEY: &str = "item.minecraft.potion.effect.water";
const DRINK_TICKS: u32 = 32;
const REACH: f64 = 4.5;

pub fn is_water_source(block: &Block) -> bool {
    match block.id.as_str() {
        "minecraft:water" => block.property("level").is_none_or(|level| level == "0"),
        "minecraft:kelp"
        | "minecraft:kelp_plant"
        | "minecraft:seagrass"
        | "minecraft:tall_seagrass"
        | "minecraft:bubble_column" => true,
        _ => block.property("waterlogged") == Some("true"),
    }
}

pub fn water_bottle() -> ItemStack {
    let mut stack = ItemStack::new(POTION, 1);
    stack.max = 1;
    stack.components = Some(json!({"minecraft:potion_contents": {"potion": "minecraft:water"}}));
    stack
}

pub fn is_water_bottle(stack: &ItemStack) -> bool {
    stack.id == POTION
        && stack
            .components
            .as_ref()
            .and_then(|c| c.get("minecraft:potion_contents"))
            .is_some_and(|contents| {
                contents.as_str() == Some("minecraft:water")
                    || contents.get("potion").and_then(Value::as_str) == Some("minecraft:water")
            })
}

pub fn filled_result(
    inventory: &mut Inventory,
    slot: usize,
    filled: ItemStack,
) -> Option<ItemStack> {
    let held = inventory.slots.get_mut(slot)?.as_mut()?;
    held.count = held.count.saturating_sub(1);
    if held.count == 0 {
        inventory.slots[slot] = Some(filled);
        return None;
    }
    inventory.add_item(filled, slot)
}

fn level(block: &Block) -> u8 {
    block
        .property("level")
        .and_then(|l| l.parse().ok())
        .unwrap_or(1)
}

pub fn cauldron_use(
    block: &Block,
    held: &ItemStack,
) -> Option<(Option<Block>, ItemStack, &'static str)> {
    let glass = ItemStack::new(GLASS_BOTTLE, 1);
    match block.id.as_str() {
        "minecraft:cauldron" if is_water_bottle(held) => Some((
            Some(Block::new("minecraft:water_cauldron").with("level", "1")),
            glass,
            EMPTY_SOUND,
        )),
        "minecraft:water_cauldron" if is_water_bottle(held) && level(block) < 3 => {
            let next = (level(block) + 1).to_string();
            Some((Some(block.clone().with("level", &next)), glass, EMPTY_SOUND))
        }
        "minecraft:water_cauldron" if held.id == GLASS_BOTTLE => {
            let left = level(block).saturating_sub(1);
            let next = if left == 0 {
                Block::new("minecraft:cauldron")
            } else {
                block.clone().with("level", &left.to_string())
            };
            Some((Some(next), water_bottle(), FILL_SOUND))
        }
        _ => None,
    }
}

pub fn mud_target(block: &Block, face: Face, held: &ItemStack) -> bool {
    is_water_bottle(held)
        && face != Face::Down
        && matches!(
            block.id.as_str(),
            "minecraft:dirt" | "minecraft:coarse_dirt" | "minecraft:rooted_dirt"
        )
}

pub fn drink(slot: usize, stack: &ItemStack) -> FoodUse {
    FoodUse {
        slot,
        stack: stack.clone(),
        info: FoodInfo {
            nutrition: 0,
            saturation: 0.0,
            can_always_eat: true,
            consume_ticks: DRINK_TICKS,
            animation: "drink".to_owned(),
            sound: "entity.generic.drink".to_owned(),
            particles: false,
            remainder: Some(ItemStack::new(GLASS_BOTTLE, 1)),
        },
        elapsed_ticks: 0,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BottleUse {
    Block {
        pos: Pos,
        block: Option<Block>,
        result: ItemStack,
        sounds: Vec<&'static str>,
    },
    Fill {
        result: ItemStack,
    },
    Drink,
}

pub fn bottle_use(player: &Player, world: &impl World, held: &ItemStack) -> Option<BottleUse> {
    let glass = held.id == GLASS_BOTTLE;
    let water = is_water_bottle(held);
    if !glass && !water {
        return None;
    }
    if let Some(hit) = player.target(world, REACH) {
        if let Some(block) = world.block(hit.pos) {
            if !player.crouching {
                if let Some((block, result, sound)) = cauldron_use(&block, held) {
                    return Some(BottleUse::Block {
                        pos: hit.pos,
                        block,
                        result,
                        sounds: vec![sound],
                    });
                }
            }
            if mud_target(&block, hit.face, held) {
                return Some(BottleUse::Block {
                    pos: hit.pos,
                    block: Some(Block::new("minecraft:mud")),
                    result: ItemStack::new(GLASS_BOTTLE, 1),
                    sounds: vec![SPLASH_SOUND, EMPTY_SOUND],
                });
            }
        }
    }
    if water {
        return Some(BottleUse::Drink);
    }
    let hit = player.target_source_fluid(world, REACH)?;
    world
        .block(hit.pos)
        .filter(is_water_source)
        .map(|_| BottleUse::Fill {
            result: water_bottle(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
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

    fn looking_down_at(world_block: Option<(Pos, Block)>, under: &str) -> (Player, Blocks) {
        let mut world = Blocks::default();
        world.set_block((0, 0, 0), Some(Block::new(under)));
        if let Some((pos, block)) = world_block {
            world.set_block(pos, Some(block));
        }
        let mut player = Player::new(DVec3::new(0.5, 2.0, 0.5));
        player.pitch = 90.0;
        (player, world)
    }

    #[test]
    fn kelp_seagrass_and_waterlogged_blocks_hold_water_but_flowing_water_does_not() {
        assert!(is_water_source(
            &Block::new("minecraft:water").with("level", "0")
        ));
        assert!(!is_water_source(
            &Block::new("minecraft:water").with("level", "3")
        ));
        assert!(is_water_source(&Block::new("minecraft:kelp_plant")));
        assert!(is_water_source(&Block::new("minecraft:tall_seagrass")));
        assert!(is_water_source(
            &Block::new("minecraft:oak_stairs").with("waterlogged", "true")
        ));
        assert!(!is_water_source(&Block::new("minecraft:stone")));
    }

    #[test]
    fn a_last_bottle_becomes_the_water_bottle_and_others_go_to_the_inventory() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new(GLASS_BOTTLE, 1));
        assert_eq!(filled_result(&mut inventory, 0, water_bottle()), None);
        assert!(inventory.slots[0].as_ref().is_some_and(is_water_bottle));
        inventory.slots[1] = Some(ItemStack::new(GLASS_BOTTLE, 3));
        assert_eq!(filled_result(&mut inventory, 1, water_bottle()), None);
        assert_eq!(inventory.slots[1].as_ref().map(|s| s.count), Some(2));
        let filled = inventory
            .slots
            .iter()
            .flatten()
            .filter(|s| is_water_bottle(s))
            .count();
        assert_eq!(filled, 2);
    }

    #[test]
    fn water_bottles_never_stack() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(water_bottle());
        inventory.slots[1] = Some(ItemStack::new(GLASS_BOTTLE, 2));
        filled_result(&mut inventory, 1, water_bottle());
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(1));
        assert_eq!(water_bottle().max, 1);
    }

    #[test]
    fn cauldrons_fill_and_empty_a_third_at_a_time() {
        let bottle = water_bottle();
        let glass = ItemStack::new(GLASS_BOTTLE, 1);
        let (one, back, sound) = cauldron_use(&Block::new("minecraft:cauldron"), &bottle).unwrap();
        let one = one.unwrap();
        assert_eq!(
            (one.id.as_str(), one.property("level")),
            ("minecraft:water_cauldron", Some("1"))
        );
        assert_eq!((back.id.as_str(), sound), (GLASS_BOTTLE, EMPTY_SOUND));
        let (two, ..) = cauldron_use(&one, &bottle).unwrap();
        let three = cauldron_use(two.as_ref().unwrap(), &bottle)
            .unwrap()
            .0
            .unwrap();
        assert_eq!(three.property("level"), Some("3"));
        assert!(cauldron_use(&three, &bottle).is_none());
        let (two_again, filled, sound) = cauldron_use(&three, &glass).unwrap();
        assert_eq!(two_again.unwrap().property("level"), Some("2"));
        assert!(is_water_bottle(&filled) && sound == FILL_SOUND);
        let empty = cauldron_use(&one, &glass).unwrap().0.unwrap();
        assert_eq!(empty.id, "minecraft:cauldron");
        assert!(cauldron_use(&Block::new("minecraft:cauldron"), &glass).is_none());
    }

    #[test]
    fn water_on_dirt_makes_mud_but_not_from_below() {
        let bottle = water_bottle();
        assert!(mud_target(&Block::new("minecraft:dirt"), Face::Up, &bottle));
        assert!(mud_target(
            &Block::new("minecraft:rooted_dirt"),
            Face::North,
            &bottle
        ));
        assert!(!mud_target(
            &Block::new("minecraft:dirt"),
            Face::Down,
            &bottle
        ));
        assert!(!mud_target(
            &Block::new("minecraft:stone"),
            Face::Up,
            &bottle
        ));
        let glass = ItemStack::new(GLASS_BOTTLE, 1);
        assert!(!mud_target(&Block::new("minecraft:dirt"), Face::Up, &glass));
    }

    #[test]
    fn a_glass_bottle_fills_from_a_source_it_looks_at() {
        let water = Block::new("minecraft:water").with("level", "0");
        let (player, world) = looking_down_at(Some(((0, 1, 0), water)), "minecraft:stone");
        let held = ItemStack::new(GLASS_BOTTLE, 4);
        assert!(matches!(
            bottle_use(&player, &world, &held),
            Some(BottleUse::Fill { .. })
        ));
        let flowing = Block::new("minecraft:water").with("level", "2");
        let (player, world) = looking_down_at(Some(((0, 1, 0), flowing)), "minecraft:stone");
        assert_eq!(bottle_use(&player, &world, &held), None);
    }

    #[test]
    fn a_water_bottle_on_dirt_is_mud_and_elsewhere_a_drink() {
        let (player, world) = looking_down_at(None, "minecraft:dirt");
        match bottle_use(&player, &world, &water_bottle()) {
            Some(BottleUse::Block { pos, block, .. }) => {
                assert_eq!(pos, (0, 0, 0));
                assert_eq!(block.map(|b| b.id).as_deref(), Some("minecraft:mud"));
            }
            other => panic!("{other:?}"),
        }
        let (player, world) = looking_down_at(None, "minecraft:stone");
        assert_eq!(
            bottle_use(&player, &world, &water_bottle()),
            Some(BottleUse::Drink)
        );
    }

    #[test]
    fn crouching_skips_the_cauldron_and_drinks() {
        let (mut player, world) = looking_down_at(
            Some((
                (0, 1, 0),
                Block::new("minecraft:water_cauldron").with("level", "1"),
            )),
            "minecraft:stone",
        );
        assert!(matches!(
            bottle_use(&player, &world, &water_bottle()),
            Some(BottleUse::Block { .. })
        ));
        player.crouching = true;
        assert_eq!(
            bottle_use(&player, &world, &water_bottle()),
            Some(BottleUse::Drink)
        );
    }

    #[test]
    fn drinking_takes_32_ticks_and_leaves_the_bottle() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(water_bottle());
        let mut food = minecraftoss_player::survival::FoodData::default();
        let mut using = drink(0, inventory.slots[0].as_ref().unwrap());
        let mut finished = 0;
        for tick in 1..=32 {
            if matches!(
                using.tick(
                    &mut inventory,
                    &mut food,
                    minecraftoss_player::GameMode::Survival
                ),
                minecraftoss_player::food::FoodUseTick::Finished { .. }
            ) {
                finished = tick;
            }
        }
        assert_eq!(finished, 32);
        assert_eq!(
            inventory.slots[0].as_ref().map(|s| s.id.as_str()),
            Some(GLASS_BOTTLE)
        );
    }
}
