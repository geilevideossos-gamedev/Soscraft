use minecraftoss_player::inventory::{Inventory, ItemStack};

pub const UNBREAKING: &str = "minecraft:unbreaking";
pub const STEERING_ITEM: &str = "minecraft:warped_fungus_on_a_stick";
pub const STEERING_LEFTOVER: &str = "minecraft:fishing_rod";
pub const BOOST_COST: u32 = 1;

pub fn wear(
    inventory: &mut Inventory,
    slot: usize,
    amount: u32,
    mut roll: impl FnMut() -> f32,
) -> bool {
    let Some(stack) = inventory.slots.get(slot).cloned().flatten() else {
        return false;
    };
    let unbreaking = crate::trident::enchantment(&stack, UNBREAKING);
    let spare = unbreaking as f32 / (unbreaking + 1) as f32;
    let lost = (0..amount)
        .filter(|_| !(unbreaking > 0 && roll() < spare))
        .count() as u32;
    inventory.wear_tool(slot, lost)
}

pub fn wear_and_convert(
    inventory: &mut Inventory,
    slot: usize,
    amount: u32,
    roll: impl FnMut() -> f32,
    into: &str,
) -> bool {
    let Some(before) = inventory.slots.get(slot).cloned().flatten() else {
        return false;
    };
    if !wear(inventory, slot, amount, roll) {
        return false;
    }
    let mut leftover = ItemStack::new(into, 1);
    leftover.components = before.components.and_then(|components| {
        let mut map = components.as_object()?.clone();
        map.remove("minecraft:damage");
        (!map.is_empty()).then_some(serde_json::Value::Object(map))
    });
    inventory.slots[slot] = Some(leftover);
    true
}

pub fn spend_boost(
    inventory: &mut Inventory,
    slot: usize,
    infinite: bool,
    roll: impl FnMut() -> f32,
) -> bool {
    if infinite {
        return false;
    }
    wear_and_convert(inventory, slot, BOOST_COST, roll, STEERING_LEFTOVER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::rng::LegacyRandom;

    fn stick(components: serde_json::Value) -> Inventory {
        let mut stack = ItemStack::new(STEERING_ITEM, 1);
        let mut map = components.as_object().cloned().unwrap_or_default();
        map.insert("minecraft:max_damage".to_owned(), 100.into());
        stack.components = Some(serde_json::Value::Object(map));
        let mut inventory = Inventory::default();
        inventory.slots[2] = Some(stack);
        inventory
    }

    fn damage(inventory: &Inventory) -> Option<u64> {
        inventory.slots[2]
            .as_ref()?
            .components
            .as_ref()?
            .get("minecraft:damage")?
            .as_u64()
    }

    #[test]
    fn a_boost_costs_one_of_a_hundred() {
        let mut inventory = stick(serde_json::json!({}));
        let mut random = LegacyRandom::new(3);
        assert!(!spend_boost(&mut inventory, 2, false, || random.next_float()));
        assert_eq!(damage(&inventory), Some(1));
        for _ in 0..97 {
            assert!(!spend_boost(&mut inventory, 2, false, || random.next_float()));
        }
        assert_eq!(damage(&inventory), Some(98));
        assert_eq!(inventory.slots[2].as_ref().unwrap().id, STEERING_ITEM);
    }

    #[test]
    fn unbreaking_three_spares_three_in_four() {
        let mut random = LegacyRandom::new(7);
        let mut lost = 0;
        for _ in 0..4000 {
            let mut inventory = stick(serde_json::json!({
                "minecraft:enchantments": { "minecraft:unbreaking": 3 }
            }));
            spend_boost(&mut inventory, 2, false, || random.next_float());
            lost += damage(&inventory).unwrap_or(0);
        }
        assert!((900..=1100).contains(&lost), "{lost}");
    }

    #[test]
    fn the_hundredth_boost_leaves_an_enchanted_fishing_rod() {
        let mut inventory = stick(serde_json::json!({
            "minecraft:damage": 99,
            "minecraft:enchantments": { "minecraft:mending": 1 },
            "minecraft:custom_name": "Steer"
        }));
        let mut random = LegacyRandom::new(1);
        assert!(spend_boost(&mut inventory, 2, false, || random.next_float()));
        let rod = inventory.slots[2].as_ref().expect("a fishing rod is left");
        assert_eq!((rod.id.as_str(), rod.count), (STEERING_LEFTOVER, 1));
        assert_eq!(damage(&inventory), None);
        let components = rod.components.as_ref().unwrap();
        assert_eq!(
            components.get("minecraft:enchantments"),
            Some(&serde_json::json!({ "minecraft:mending": 1 }))
        );
        assert_eq!(
            components.get("minecraft:custom_name"),
            Some(&serde_json::json!("Steer"))
        );
    }

    #[test]
    fn creative_boosts_cost_nothing() {
        let mut inventory = stick(serde_json::json!({ "minecraft:damage": 99 }));
        assert!(!spend_boost(&mut inventory, 2, true, || 0.0));
        assert_eq!(damage(&inventory), Some(99));
        assert_eq!(inventory.slots[2].as_ref().unwrap().id, STEERING_ITEM);
    }
}
