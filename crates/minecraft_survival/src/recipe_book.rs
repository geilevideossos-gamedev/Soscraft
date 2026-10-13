use std::collections::BTreeSet;

use minecraft_net::messages::RecipeBookView;
use minecraftoss_player::inventory::{Inventory, RecipePlacement};

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

pub fn place(inventory: &mut Inventory, table: bool, recipe: &str, all: bool) -> RecipePlacement {
    if !minecraft_terrain::recipe_book::always_known(recipe)
        && !inventory.unlocked_recipes.contains(recipe)
    {
        return RecipePlacement::Blocked;
    }
    inventory.place_known_crafting_recipe(recipe, table, all)
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

    use minecraft_net::messages::{Click, MenuAction};
    use minecraft_terrain::containers::{Containers, Opened, Station};
    use minecraft_terrain::stations::StationBook;

    use super::*;
    use crate::menus::{self, Books, Open};

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

    fn placing() -> Inventory {
        let planks = json!({"type": "minecraft:crafting_shapeless", "category": "building", "ingredients": ["minecraft:oak_log"], "result": {"id": "minecraft:oak_planks", "count": 4}});
        let mut recipes = minecraft_terrain::fork_recipes::recipes();
        recipes.push(("minecraft:oak_planks".to_owned(), planks));
        Inventory::default().with_recipes(RecipeBook::default().with_extra(recipes))
    }

    fn stack(id: &str, count: u8) -> Option<ItemStack> {
        Some(ItemStack::new(id, count))
    }

    fn cells(grid: &[Option<ItemStack>]) -> Vec<Option<(&str, u8)>> {
        grid.iter()
            .map(|stack| stack.as_ref().map(|stack| (stack.id.as_str(), stack.count)))
            .collect()
    }

    fn total(inventory: &Inventory, id: &str) -> u32 {
        inventory
            .slots
            .iter()
            .flatten()
            .filter(|stack| stack.id == id)
            .map(|stack| u32::from(stack.count))
            .sum()
    }

    fn player(id: u32, name: &str, inventory: Inventory) -> crate::player::Player {
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let id = minecraft_commands::PlayerId(id);
        chat.join(id, name);
        crate::player::Player::new(
            chat.profile(id).unwrap(),
            minecraft_terrain::game_mode::PlayerMode::default(),
            crate::survival::Survival::default(),
            inventory,
        )
    }

    fn place_action(recipe: &str, all: bool) -> MenuAction {
        MenuAction::PlaceRecipe {
            recipe: recipe.to_owned(),
            all,
        }
    }

    fn apply(player: &mut crate::player::Player, action: &MenuAction) {
        let stations = StationBook::default();
        let books = Books {
            stations: &stations,
            enchantments: None,
            guns: None,
        };
        menus::apply(player, &mut Containers::default(), books, action, 0.5);
    }

    #[test]
    fn placing_a_recipe_clears_the_grid_then_fills_it_and_another_click_adds_a_set() {
        let mut inventory = placing();
        inventory.slots[5] = stack("minecraft:iron_ingot", 10);
        let mut host = player(0, "Steve", inventory);
        assert!(menus::open(
            &mut host,
            Open::Table((0, 64, 0)),
            minecraft_commands::Text::literal("Crafting"),
            0
        ));
        host.inventory.workbench[0] = stack("minecraft:dirt", 3);
        apply(&mut host, &place_action("iw4l:gun_barrel", false));
        let iron = Some(("minecraft:iron_ingot", 1));
        assert_eq!(
            cells(&host.inventory.workbench),
            [None, None, None, iron, iron, iron, None, None, None]
        );
        assert_eq!(total(&host.inventory, "minecraft:dirt"), 3);
        assert_eq!(total(&host.inventory, "minecraft:iron_ingot"), 7);
        apply(&mut host, &place_action("iw4l:gun_barrel", false));
        let iron = Some(("minecraft:iron_ingot", 2));
        assert_eq!(cells(&host.inventory.workbench)[3..6], [iron, iron, iron]);
        assert_eq!(total(&host.inventory, "minecraft:iron_ingot"), 4);
        let made = host
            .inventory
            .recipes
            .matching(&host.inventory.workbench, 3, 3);
        assert_eq!(
            made.map(|stack| (stack.id, stack.count)),
            Some(("iw4l:gun_barrel".to_owned(), 2))
        );
    }

    #[test]
    fn shift_places_as_many_sets_as_the_inventory_holds() {
        let mut inventory = placing();
        inventory.slots[0] = stack("minecraft:copper_ingot", 5);
        inventory.slots[9] = stack("minecraft:iron_nugget", 20);
        inventory.slots[20] = stack("minecraft:gunpowder", 64);
        inventory.slots[21] = stack("minecraft:gunpowder", 64);
        let mut host = player(0, "Steve", inventory);
        apply(&mut host, &place_action("iw4l:rifle_ammo", true));
        assert_eq!(
            cells(&host.inventory.crafting),
            [
                Some(("minecraft:copper_ingot", 5)),
                Some(("minecraft:iron_nugget", 5)),
                Some(("minecraft:gunpowder", 5)),
                None
            ]
        );
        assert_eq!(total(&host.inventory, "minecraft:copper_ingot"), 0);
        assert_eq!(total(&host.inventory, "minecraft:iron_nugget"), 15);
        assert_eq!(total(&host.inventory, "minecraft:gunpowder"), 123);
        let mut many = placing();
        for slot in 0..4 {
            many.slots[slot] = stack("minecraft:copper_ingot", 64);
            many.slots[slot + 9] = stack("minecraft:iron_nugget", 64);
            many.slots[slot + 18] = stack("minecraft:gunpowder", 64);
        }
        assert_eq!(
            place(&mut many, false, "iw4l:rifle_ammo", true),
            RecipePlacement::Placed
        );
        assert_eq!(
            cells(&many.crafting),
            [
                Some(("minecraft:copper_ingot", 64)),
                Some(("minecraft:iron_nugget", 64)),
                Some(("minecraft:gunpowder", 64)),
                None
            ]
        );
    }

    #[test]
    fn a_missing_ingredient_places_nothing_and_asks_for_the_ghost() {
        let mut inventory = placing();
        inventory.slots[0] = stack("minecraft:iron_ingot", 4);
        inventory.slots[1] = stack("minecraft:iron_nugget", 4);
        inventory.slots[2] = stack("minecraft:gunpowder", 1);
        let mut named = ItemStack::new("minecraft:gunpowder", 5);
        named.components = Some(json!({"minecraft:custom_name": "Dust"}));
        inventory.slots[3] = Some(named);
        inventory.crafting[3] = stack("minecraft:dirt", 2);
        assert_eq!(
            place(&mut inventory, false, "iw4l:frag_grenade", false),
            RecipePlacement::Ghost
        );
        assert_eq!(cells(&inventory.crafting), [None, None, None, None]);
        assert_eq!(total(&inventory, "minecraft:dirt"), 2);
        assert_eq!(total(&inventory, "minecraft:iron_ingot"), 4);
        assert_eq!(total(&inventory, "minecraft:gunpowder"), 6);
        inventory.slots[4] = stack("minecraft:oak_log", 2);
        assert_eq!(
            place(&mut inventory, false, "minecraft:oak_planks", false),
            RecipePlacement::Blocked
        );
        assert!(inventory.unlock_recipe("minecraft:oak_planks"));
        assert_eq!(
            place(&mut inventory, false, "minecraft:oak_planks", false),
            RecipePlacement::Placed
        );
        assert_eq!(
            cells(&inventory.crafting)[0],
            Some(("minecraft:oak_log", 1))
        );
        assert_eq!(
            place(&mut inventory, false, "iw4l:m4", false),
            RecipePlacement::Blocked
        );
        assert_eq!(
            place(&mut inventory, false, "iw4l:nothing", false),
            RecipePlacement::Blocked
        );
    }

    #[test]
    fn a_friend_places_through_its_click_and_only_in_a_crafting_screen() {
        let mut inventory = placing();
        inventory.slots[7] = stack("minecraft:copper_ingot", 2);
        inventory.slots[8] = stack("minecraft:iron_nugget", 2);
        inventory.slots[30] = stack("minecraft:gunpowder", 2);
        let mut friend = player(1, "Alex", inventory);
        let stations = StationBook::default();
        let books = Books {
            stations: &stations,
            enchantments: None,
            guns: None,
        };
        let mut containers = Containers::default();
        let chest = Opened {
            station: Station::Chest,
            cells: vec![(4, 64, 4)],
        };
        assert!(menus::open(
            &mut friend,
            Open::Block(chest),
            minecraft_commands::Text::literal("Chest"),
            0
        ));
        let id = friend.menus.current().id;
        let message = |menu: u8, recipe: &str| Click {
            menu,
            state: 0,
            action: place_action(recipe, false),
        };
        let mut click = |friend: &mut crate::player::Player, message: Click| {
            menus::click(friend, &mut containers, books, &message, 0.5);
        };
        click(&mut friend, message(id, "iw4l:rifle_ammo"));
        assert_eq!(cells(&friend.inventory.crafting), [None, None, None, None]);
        assert!(friend.menus.current().resends());
        menus::close(&mut friend);
        let long = format!("iw4l:{}", "a".repeat(menus::RECIPE_ID_LIMIT));
        click(&mut friend, message(menus::INVENTORY_MENU, &long));
        assert!(friend.menus.current().resends());
        click(
            &mut friend,
            message(menus::INVENTORY_MENU, "iw4l:rifle_ammo"),
        );
        assert_eq!(
            cells(&friend.inventory.crafting)[..3],
            [
                Some(("minecraft:copper_ingot", 1)),
                Some(("minecraft:iron_nugget", 1)),
                Some(("minecraft:gunpowder", 1))
            ]
        );
        assert_eq!(total(&friend.inventory, "minecraft:copper_ingot"), 1);
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
