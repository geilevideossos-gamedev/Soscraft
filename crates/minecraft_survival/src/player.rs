use std::sync::Arc;

use minecraft_commands::PlayerId;
use minecraft_commands::server::Profile;
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::game_mode::PlayerMode;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::Inventory;

use crate::death::DeathWatch;
use crate::survival::Survival;

pub struct Player {
    pub uuid: PlayerUuid,
    pub id: PlayerId,
    pub name: String,
    pub dimension: Dimension,
    pub survival: Survival,
    pub mode: PlayerMode,
    pub death_watch: DeathWatch,
    pub inventory: Inventory,
    pub selected: usize,
}

impl Player {
    pub fn new(
        profile: &Profile,
        mode: PlayerMode,
        survival: Survival,
        inventory: Inventory,
    ) -> Self {
        Self {
            uuid: PlayerUuid(profile.uuid.clone()),
            id: profile.id,
            name: profile.name.clone(),
            dimension: Dimension::Overworld,
            survival,
            mode,
            death_watch: DeathWatch::default(),
            inventory,
            selected: 0,
        }
    }
}

pub fn starting_inventory(recipes: Arc<RecipeBook>, kit: Option<&str>) -> Inventory {
    let mut inventory = Inventory::default();
    inventory.recipes = recipes;
    if let Some(kit) = kit {
        let given = minecraft_terrain::crafting::give_kit(&mut inventory, kit);
        diag::info!(
            World,
            "Minecraft kit: {given} stacks from IW4L_MINECRAFT_KIT"
        );
    }
    inventory
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::HOST;
    use minecraft_commands::CommandServer;
    use minecraft_terrain::game_mode::GameMode;
    use minecraftoss_player::inventory::ItemStack;

    #[test]
    fn a_joining_player_starts_in_the_overworld_with_the_survival_it_carries() {
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        let mut carried = Survival::default();
        carried.observe(100, 100);
        carried.status.health = 7.0;
        let mut inventory = Inventory::default();
        inventory.slots[3] = Some(ItemStack::new("minecraft:stone", 5));
        let player = Player::new(
            chat.profile(HOST).unwrap(),
            PlayerMode::new(GameMode::Creative),
            carried,
            inventory,
        );
        assert_eq!(
            player.uuid,
            PlayerUuid(minecraft_commands::offline_uuid("Steve"))
        );
        assert_eq!(player.id, HOST);
        assert_eq!(player.name, "Steve");
        assert_eq!(player.dimension, Dimension::Overworld);
        assert_eq!(player.survival.status.health, 7.0);
        assert_eq!(player.mode.mode, GameMode::Creative);
        assert!(!player.death_watch.switched_teams());
        assert_eq!(
            player.inventory.slots[3].as_ref().map(|stack| stack.count),
            Some(5)
        );
        assert_eq!(player.selected, 0);
    }

    #[test]
    fn the_kit_fills_the_backpack_and_the_inventory_uses_the_world_recipes() {
        let recipes = Arc::new(RecipeBook::default());
        let empty = starting_inventory(recipes.clone(), None);
        assert!(Arc::ptr_eq(&empty.recipes, &recipes));
        assert!(empty.slots.iter().all(Option::is_none));
        let kit = starting_inventory(recipes.clone(), Some("cobblestone*70,diamond_sword"));
        assert!(Arc::ptr_eq(&kit.recipes, &recipes));
        let stacks: Vec<(usize, &str, u8)> = kit
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, stack)| stack.as_ref().map(|s| (slot, s.id.as_str(), s.count)))
            .collect();
        assert_eq!(
            stacks,
            vec![
                (9, "minecraft:cobblestone", 64),
                (10, "minecraft:cobblestone", 6),
                (11, "minecraft:diamond_sword", 1)
            ]
        );
    }
}
