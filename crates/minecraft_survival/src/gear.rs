use minecraft_net::messages::{GearItem, GearView};
use minecraft_terrain::body_gear::BodyGear;
use minecraft_terrain::game_mode::PlayerMode;
use minecraftoss_player::inventory::Inventory;

use crate::guns::weapon_of;
use crate::player::Player;

pub fn body_gear(inventory: &Inventory, selected: usize, mode: &PlayerMode) -> Option<BodyGear> {
    mode.interacts().then(|| {
        minecraft_terrain::body_gear::body_gear(&inventory.slots, selected, |stack| {
            weapon_of(stack).is_some()
        })
    })
}

fn item_view(item: &minecraft_terrain::body_gear::GearItem) -> GearItem {
    GearItem {
        id: item.id.clone(),
        glint: item.glint,
        dye: item.dye,
    }
}

fn item_of(view: &GearItem) -> minecraft_terrain::body_gear::GearItem {
    minecraft_terrain::body_gear::GearItem {
        id: view.id.clone(),
        glint: view.glint,
        dye: view.dye,
    }
}

pub fn gear_view(gear: &BodyGear) -> GearView {
    GearView {
        main: gear.main.as_ref().map(item_view),
        off: gear.off.as_ref().map(item_view),
        armor: gear
            .armor
            .each_ref()
            .map(|item| item.as_ref().map(item_view)),
        hides_gun: gear.hides_gun,
    }
}

pub fn gear_of(view: &GearView) -> BodyGear {
    BodyGear {
        main: view.main.as_ref().map(item_of),
        off: view.off.as_ref().map(item_of),
        armor: view.armor.each_ref().map(|item| item.as_ref().map(item_of)),
        hides_gun: view.hides_gun,
    }
}

pub fn player_gear(player: &Player) -> GearView {
    body_gear(&player.inventory, player.selected, &player.mode)
        .as_ref()
        .map_or_else(GearView::default, gear_view)
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

    #[test]
    fn the_gear_sent_to_friends_comes_back_the_same_and_carries_the_elytra() {
        let mut inventory = kitted();
        inventory.slots[38] = Some(ItemStack::new("minecraft:elytra", 1));
        let gear = body_gear(&inventory, 1, &PlayerMode::new(GameMode::Survival)).unwrap();
        let view = gear_view(&gear);
        assert_eq!(
            view.armor[1].as_ref().map(|item| item.id.as_str()),
            Some("minecraft:elytra")
        );
        assert_eq!(gear_of(&view), gear);
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        chat.join(crate::commands::HOST, "Steve");
        let profile = chat.profile(crate::commands::HOST).unwrap();
        let mut player = Player::new(
            profile,
            PlayerMode::new(GameMode::Survival),
            crate::survival::Survival::default(),
            inventory,
        );
        player.selected = 1;
        assert_eq!(player_gear(&player), view);
        player.mode = PlayerMode::new(GameMode::Spectator);
        assert_eq!(player_gear(&player), GearView::default());
        assert!(gear_of(&GearView::default()).is_bare());
    }
}
