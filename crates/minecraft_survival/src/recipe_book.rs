use std::collections::BTreeSet;

use minecraft_net::messages::RecipeBookView;
use minecraftoss_player::inventory::Inventory;

use crate::player_nbt::SavedBook;

pub const SETTINGS: [&str; 8] = [
    "isGuiOpen",
    "isFilteringCraftable",
    "isFurnaceGuiOpen",
    "isFurnaceFilteringCraftable",
    "isBlastingFurnaceGuiOpen",
    "isBlastingFurnaceFilteringCraftable",
    "isSmokerGuiOpen",
    "isSmokerFilteringCraftable",
];
const OPEN: usize = 0;
const FILTERING: usize = 1;

fn saved(id: &str) -> bool {
    !minecraft_terrain::recipe_book::always_known(id)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Book {
    pub settings: [bool; 8],
    pub highlight: BTreeSet<String>,
    snapshot: BTreeSet<String>,
    shown: bool,
}

impl Book {
    pub fn restore(file: Option<&SavedBook>, inventory: &mut Inventory) -> Self {
        let mut book = Self::default();
        if let Some(file) = file {
            book.settings = file.settings;
            for id in file.recipes.iter().filter(|id| saved(id)) {
                if inventory.recipes.has_recipe(id) && !inventory.unlock_recipe(id) {
                    inventory.unlocked_recipes.insert(id.clone());
                }
            }
            book.highlight = file
                .to_be_displayed
                .iter()
                .filter(|id| saved(id) && inventory.recipes.has_recipe(id))
                .cloned()
                .collect();
        }
        book.snapshot = inventory.unlocked_recipes.iter().cloned().collect();
        book
    }

    pub fn update(&mut self, inventory: &Inventory) {
        if inventory.unlocked_recipes.len() == self.snapshot.len() {
            return;
        }
        self.highlight = self.pending(inventory);
        self.snapshot = inventory.unlocked_recipes.iter().cloned().collect();
    }

    pub fn pending(&self, inventory: &Inventory) -> BTreeSet<String> {
        let mut highlight = self.highlight.clone();
        highlight.extend(
            inventory
                .unlocked_recipes
                .iter()
                .filter(|id| saved(id) && !self.snapshot.contains(*id))
                .cloned(),
        );
        highlight
    }

    pub fn saved(&self, inventory: &Inventory) -> SavedBook {
        let mut recipes: Vec<String> = inventory
            .unlocked_recipes
            .iter()
            .filter(|id| saved(id))
            .cloned()
            .collect();
        recipes.sort();
        SavedBook {
            recipes,
            to_be_displayed: self.pending(inventory).into_iter().collect(),
            settings: self.settings,
        }
    }

    pub fn crafting(&self) -> (bool, bool) {
        (self.settings[OPEN], self.settings[FILTERING])
    }

    pub fn set_crafting(&mut self, open: bool, filtering: bool) {
        self.settings[OPEN] = open;
        self.settings[FILTERING] = filtering;
    }

    pub fn seen(&mut self, ids: &[String]) {
        for id in ids {
            self.highlight.remove(id);
        }
    }

    pub fn view(&self, inventory: &Inventory) -> RecipeBookView {
        let saved = self.saved(inventory);
        let (open, filtering) = self.crafting();
        RecipeBookView {
            recipes: saved.recipes,
            highlight: saved.to_be_displayed,
            open,
            filtering,
        }
    }
}

pub fn notice_carried(inventory: &mut Inventory) {
    let carried: Vec<_> = inventory.slots.iter().flatten().cloned().collect();
    for stack in &carried {
        inventory.notice_item_changed(stack);
    }
}

pub fn follow_screen(
    book: &mut Book,
    inventory: &Inventory,
    (open, filtering): (&mut bool, &mut bool),
    seen: &[String],
) {
    book.update(inventory);
    if book.shown {
        book.set_crafting(*open, *filtering);
    } else {
        (*open, *filtering) = book.crafting();
        book.shown = true;
    }
    book.seen(seen);
}

pub fn follow_friend(
    book: &mut Book,
    inventory: &Inventory,
    settings: Option<(bool, bool)>,
    seen: &[String],
) {
    book.update(inventory);
    if let Some((open, filtering)) = settings {
        book.set_crafting(open, filtering);
    }
    book.seen(seen);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use minecraftoss_player::crafting::RecipeBook;
    use minecraftoss_player::inventory::ItemStack;
    use serde_json::json;

    use super::*;

    fn inventory() -> Inventory {
        let planks = json!({"type": "minecraft:crafting_shapeless", "category": "building", "ingredients": ["minecraft:oak_log"], "result": {"id": "minecraft:oak_planks", "count": 4}});
        let ammo = json!({"type": "minecraft:crafting_shapeless", "category": "equipment", "ingredients": ["minecraft:gunpowder", "minecraft:copper_ingot"], "result": {"id": "iw4l:rifle_ammo", "count": 16}});
        let mut inventory = Inventory::default();
        inventory.recipes = Arc::new(RecipeBook::default().with_extra(vec![
            ("minecraft:oak_planks".to_owned(), planks.clone()),
            ("minecraft:stick".to_owned(), planks),
            ("iw4l:rifle_ammo".to_owned(), ammo),
        ]));
        inventory
    }

    fn file(recipes: &[&str], shown: &[&str]) -> SavedBook {
        SavedBook {
            recipes: recipes.iter().map(|id| (*id).to_owned()).collect(),
            to_be_displayed: shown.iter().map(|id| (*id).to_owned()).collect(),
            settings: [true, false, true, true, false, false, false, true],
        }
    }

    #[test]
    fn a_saved_book_comes_back_without_unknown_or_fork_recipes() {
        let mut inventory = inventory();
        let saved = file(
            &["minecraft:oak_planks", "minecraft:gone", "iw4l:rifle_ammo"],
            &["minecraft:oak_planks", "minecraft:gone"],
        );
        let book = Book::restore(Some(&saved), &mut inventory);
        assert_eq!(
            inventory.unlocked_recipes,
            ["minecraft:oak_planks".to_owned()].into()
        );
        assert_eq!(book.crafting(), (true, false));
        let back = book.saved(&inventory);
        assert_eq!(back.recipes, ["minecraft:oak_planks"]);
        assert_eq!(back.to_be_displayed, ["minecraft:oak_planks"]);
        assert_eq!(back.settings, saved.settings);
        let fresh = Book::restore(None, &mut Inventory::default());
        assert_eq!(fresh.crafting(), (false, false));
    }

    #[test]
    fn new_unlocks_wait_to_be_displayed_until_the_book_shows_them() {
        let mut inventory = inventory();
        let mut book = Book::restore(Some(&file(&["minecraft:oak_planks"], &[])), &mut inventory);
        assert!(inventory.unlock_recipe("minecraft:stick"));
        assert_eq!(
            book.pending(&inventory),
            ["minecraft:stick".to_owned()].into()
        );
        book.update(&inventory);
        assert_eq!(book.highlight, ["minecraft:stick".to_owned()].into());
        let view = book.view(&inventory);
        assert_eq!(view.recipes, ["minecraft:oak_planks", "minecraft:stick"]);
        assert_eq!(view.highlight, ["minecraft:stick"]);
        assert!(view.open);
        book.seen(&[
            "minecraft:stick".to_owned(),
            "minecraft:oak_planks".to_owned(),
        ]);
        assert!(book.highlight.is_empty());
        assert!(book.saved(&inventory).to_be_displayed.is_empty());
    }

    #[test]
    fn the_host_screen_takes_the_saved_settings_once_then_writes_them_back() {
        let mut inventory = inventory();
        let mut book = Book::restore(Some(&file(&[], &[])), &mut inventory);
        let (mut open, mut filtering) = (false, true);
        follow_screen(&mut book, &inventory, (&mut open, &mut filtering), &[]);
        assert_eq!((open, filtering), (true, false));
        filtering = true;
        follow_screen(&mut book, &inventory, (&mut open, &mut filtering), &[]);
        assert_eq!(book.crafting(), (true, true));
        assert_eq!(book.settings[2..], [true, true, false, false, false, true]);
        follow_friend(&mut book, &inventory, Some((false, false)), &[]);
        assert_eq!(book.crafting(), (false, false));
        follow_friend(&mut book, &inventory, None, &[]);
        assert_eq!(book.crafting(), (false, false));
    }

    #[test]
    fn items_carried_on_joining_unlock_their_recipes() {
        let mut inventory = Inventory::default();
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        inventory.recipes = Arc::new(RecipeBook::from_jar(&jar).unwrap());
        inventory.slots[12] = Some(ItemStack::new("minecraft:oak_log", 3));
        let book = Book::restore(None, &mut inventory);
        notice_carried(&mut inventory);
        assert!(inventory.unlocked_recipes.contains("minecraft:oak_planks"));
        assert!(book.pending(&inventory).contains("minecraft:oak_planks"));
    }
}
