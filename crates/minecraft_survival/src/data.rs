use std::path::{Path, PathBuf};
use std::sync::Arc;

use minecraft_terrain::stations::StationBook;
use minecraftoss_entities::enchant::Enchantments;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::loot::LootBook;

#[derive(Default)]
pub struct GameData {
    pub jar: Option<PathBuf>,
    pub recipes: Arc<RecipeBook>,
    pub loot: Option<Arc<LootBook>>,
    pub stations: Arc<StationBook>,
    pub enchantments: Option<Arc<Enchantments>>,
    pub protection: Option<Arc<minecraft_terrain::protection::ProtectionBook>>,
}

impl GameData {
    pub fn load(root: Option<&Path>, jar: Option<PathBuf>) -> Self {
        let mut data = Self::default();
        let Some(jar) = jar else {
            diag::warn!(World, "Minecraft data JAR not found: no loot or recipes");
            return data;
        };
        let catalog = minecraft_terrain::crafting::find_item_catalog(root, &jar);
        match minecraft_terrain::crafting::load_recipes(&jar, catalog.as_deref()) {
            Ok((recipes, catalog_error)) => {
                match (catalog_error, catalog.as_deref()) {
                    (Some(error), _) => diag::warn!(
                        World,
                        "Minecraft item catalog unavailable, stacks of 64 and no crafting remainders: {error:#}"
                    ),
                    (None, Some(path)) => diag::info!(
                        World,
                        "Minecraft recipes: {} with the item catalog {}",
                        recipes.count(),
                        path.display()
                    ),
                    (None, None) => {}
                }
                data.recipes = Arc::new(recipes);
            }
            Err(error) => diag::warn!(World, "Minecraft recipes unavailable: {error:#}"),
        }
        match LootBook::from_jar(&jar) {
            Ok(book) => data.loot = Some(Arc::new(book)),
            Err(error) => diag::warn!(World, "Minecraft block loot unavailable: {error:#}"),
        }
        match StationBook::from_jar(&jar) {
            Ok(book) => data.stations = Arc::new(book),
            Err(error) => diag::warn!(World, "Minecraft station recipes unavailable: {error:#}"),
        }
        match minecraft_terrain::protection::ProtectionBook::from_jar(&jar) {
            Ok(book) => data.protection = Some(Arc::new(book)),
            Err(error) => diag::warn!(World, "Minecraft armour protection unavailable: {error:#}"),
        }
        if let Some(catalog) = catalog.as_deref() {
            match minecraftoss_entities::trading::TradeBook::from_jar(&jar, catalog) {
                Ok(book) => data.enchantments = Some(Arc::new(book.enchantments)),
                Err(error) => diag::warn!(World, "Minecraft enchantments unavailable: {error:#}"),
            }
        }
        data.jar = Some(jar);
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_jar_there_is_no_loot_and_no_recipe() {
        let data = GameData::load(None, None);
        assert!(data.jar.is_none());
        assert_eq!(data.recipes.count(), 0);
        assert!(data.loot.is_none());
        assert!(data.enchantments.is_none());
        assert!(data.protection.is_none());
    }

    #[test]
    fn an_unreadable_jar_is_kept_for_the_level_server() {
        let jar = std::env::temp_dir().join("minecraft-survival-data-missing.jar");
        let data = GameData::load(None, Some(jar.clone()));
        assert_eq!(data.jar, Some(jar));
        assert_eq!(data.recipes.count(), 0);
        assert!(data.loot.is_none());
        assert!(data.enchantments.is_none());
    }
}
