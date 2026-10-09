use minecraft_terrain::body_gear::BodyGear;
use minecraft_terrain::game_mode::PlayerMode;
use minecraftoss_player::inventory::Inventory;

use crate::guns::weapon_of;

pub fn body_gear(inventory: &Inventory, selected: usize, mode: &PlayerMode) -> Option<BodyGear> {
    mode.interacts().then(|| {
        minecraft_terrain::body_gear::body_gear(&inventory.slots, selected, |stack| {
            weapon_of(stack).is_some()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guns::weapon_item;
    use minecraft_terrain::game_mode::GameMode;
    use minecraftoss_player::inventory::ItemStack;

    fn kitted() -> Inventory {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(weapon_item(5));
        inventory.slots[1] = Some(ItemStack::new("minecraft:cobblestone", 64));
        inventory.slots[39] = Some(ItemStack::new("minecraft:iron_helmet", 1));
        inventory.slots[40] = Some(ItemStack::new("minecraft:torch", 16));
        inventory
    }

    #[test]
    fn a_player_shows_the_selected_item_the_offhand_and_the_worn_armour() {
        let inventory = kitted();
        let mode = PlayerMode::new(GameMode::Survival);
        let gear = body_gear(&inventory, 1, &mode).unwrap();
        assert_eq!(
            gear.main.map(|item| item.id).as_deref(),
            Some("minecraft:cobblestone")
        );
        assert_eq!(
            gear.off.map(|item| item.id).as_deref(),
            Some("minecraft:torch")
        );
        assert_eq!(
            gear.armor[0].as_ref().map(|item| item.id.as_str()),
            Some("minecraft:iron_helmet")
        );
        assert!(gear.hides_gun);
    }

    #[test]
    fn a_selected_mw2_gun_stays_the_bodys_gun() {
        let inventory = kitted();
        let gear = body_gear(&inventory, 0, &PlayerMode::new(GameMode::Creative)).unwrap();
        assert!(gear.main.is_none());
        assert!(!gear.hides_gun);
        assert!(gear.off.is_some());
    }

    #[test]
    fn a_spectator_shows_no_gear() {
        let inventory = kitted();
        assert!(body_gear(&inventory, 1, &PlayerMode::new(GameMode::Spectator)).is_none());
    }
}
