use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use frame::{McRecipeBook, McScreen, McSlot};
use minecraft_terrain::recipe_book::{self, Known, Tab, View};
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};

pub(crate) struct Choice {
    pub id: String,
    pub result: ItemStack,
    pub craftable: bool,
    pub cells: Vec<Option<Vec<ItemStack>>>,
}

#[derive(Default)]
pub(crate) struct Built {
    pub tabs: [bool; frame::minecraft_recipe_book::MC_RECIPE_TABS],
    pub pages: usize,
    pub buttons: Vec<Vec<Choice>>,
    pub ghost: Vec<(McSlot, Vec<ItemStack>)>,
    pub tab_icons: Vec<Vec<ItemStack>>,
}

#[derive(Default)]
pub(crate) struct RecipeBookUi {
    known: Option<(usize, usize, Known)>,
    built: Option<u64>,
}

pub(crate) fn grid_of(screen: McScreen) -> Option<bool> {
    match screen {
        McScreen::Inventory => Some(false),
        McScreen::CraftingTable => Some(true),
        _ => None,
    }
}

fn grid_slot(table: bool, index: usize) -> McSlot {
    if table {
        McSlot::Workbench(index)
    } else {
        McSlot::Crafting(index)
    }
}

fn stacks(book: &RecipeBook, ids: &[String]) -> Vec<ItemStack> {
    ids.iter().map(|id| book.stack(id, 1)).collect()
}

fn cells(book: &RecipeBook, cells: &recipe_book::Cells) -> Vec<Option<Vec<ItemStack>>> {
    cells
        .iter()
        .map(|cell| cell.as_ref().map(|ids| stacks(book, ids)))
        .collect()
}

impl RecipeBookUi {
    pub(crate) fn build(
        &mut self,
        state: &McRecipeBook,
        inventory: &Inventory,
        grid: Option<bool>,
        language: &HashMap<String, String>,
    ) -> Option<Built> {
        let Some(table) = grid.filter(|_| state.open || state.ghost.is_some()) else {
            return self.built.take().map(|_| Built::default());
        };
        let book = &inventory.recipes;
        let key = (Arc::as_ptr(book) as usize, inventory.unlocked_recipes.len());
        if self
            .known
            .as_ref()
            .is_none_or(|(ptr, unlocked, _)| (*ptr, *unlocked) != key)
        {
            self.known = Some((
                key.0,
                key.1,
                recipe_book::known(book, &inventory.unlocked_recipes),
            ));
            self.built = None;
        }
        let grid_cells: &[Option<ItemStack>] = if table {
            &inventory.workbench
        } else {
            &inventory.crafting
        };
        let counts = recipe_book::counts(
            inventory
                .slots
                .iter()
                .take(recipe_book::STORAGE)
                .chain(grid_cells),
        );
        let mut held: Vec<(&String, &u32)> = counts.iter().collect();
        held.sort();
        let mut hasher = DefaultHasher::new();
        (
            key,
            table,
            state.open,
            state.filtering,
            state.tab,
            state.page,
            &state.query,
            &state.ghost,
            state.overlay,
            &held,
            language.len(),
        )
            .hash(&mut hasher);
        let hash = hasher.finish();
        if self.built == Some(hash) {
            return None;
        }
        self.built = Some(hash);
        let known = &self.known.as_ref()?.2;
        let craftable: Vec<bool> = known
            .entries
            .iter()
            .map(|entry| entry.fits(table) && recipe_book::craftable(&entry.table, &counts))
            .collect();
        let found = (!state.query.trim().is_empty()).then(|| {
            recipe_book::matches(known, &state.query, |stack| {
                vec![minecraft_terrain::item_icons::item_name(
                    language, &stack.id,
                )]
            })
        });
        let mut view = View {
            tab: Tab::Search,
            table,
            filtering: state.filtering,
            craftable: &craftable,
            matches: found.as_deref(),
        };
        let tabs = Tab::ALL.map(|tab| view.tab_visible(known, tab));
        let wanted = Tab::at(state.tab);
        if tabs[wanted.index()] {
            view.tab = wanted;
        }
        let shown = view.shown(known);
        let pages = recipe_book::pages(shown.len());
        let page = recipe_book::page_of(state.page, pages);
        let buttons = shown
            .iter()
            .skip(page * recipe_book::PER_PAGE)
            .take(recipe_book::PER_PAGE)
            .enumerate()
            .map(|(button, &collection)| {
                view.selected(known, collection)
                    .into_iter()
                    .map(|entry| {
                        let recipe = &known.entries[entry];
                        Choice {
                            id: recipe.id.clone(),
                            result: recipe.result.clone(),
                            craftable: craftable[entry],
                            cells: if state.overlay == Some(button) {
                                cells(book, &recipe.table)
                            } else {
                                Vec::new()
                            },
                        }
                    })
                    .collect()
            })
            .collect();
        let mut ghost = Vec::new();
        if let Some(recipe) = state
            .ghost
            .as_ref()
            .and_then(|id| known.entries.iter().find(|entry| &entry.id == id))
            && let Some(grid) = recipe.grid(table)
        {
            for (index, cell) in grid.iter().enumerate() {
                if let Some(ids) = cell {
                    ghost.push((grid_slot(table, index), stacks(book, ids)));
                }
            }
            let result = if table {
                McSlot::WorkbenchResult
            } else {
                McSlot::Result
            };
            ghost.push((result, vec![recipe.result.clone()]));
        }
        Some(Built {
            tabs,
            pages,
            buttons,
            ghost,
            tab_icons: Tab::ALL
                .iter()
                .map(|tab| tab.icons().iter().map(|id| book.stack(id, 1)).collect())
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn inventory() -> Inventory {
        let mut inventory = Inventory::default();
        inventory.recipes = Arc::new(RecipeBook::default().with_extra(vec![
            (
                "minecraft:stick".to_owned(),
                json!({"type": "minecraft:crafting_shaped", "category": "misc", "pattern": ["#", "#"], "key": {"#": "minecraft:oak_planks"}, "result": {"id": "minecraft:stick", "count": 4}}),
            ),
            (
                "iw4l:rifle_ammo".to_owned(),
                json!({"type": "minecraft:crafting_shapeless", "category": "equipment", "group": "iw4l_ammo", "ingredients": ["minecraft:copper_ingot", "minecraft:iron_nugget", "minecraft:gunpowder"], "result": {"id": "iw4l:rifle_ammo", "count": 16}}),
            ),
        ]));
        inventory
    }

    #[test]
    fn the_book_lists_fork_recipes_always_and_vanilla_ones_once_unlocked() {
        let mut inventory = inventory();
        let mut ui = RecipeBookUi::default();
        let mut state = McRecipeBook {
            open: true,
            ..McRecipeBook::default()
        };
        let language = HashMap::new();
        assert!(ui.build(&state, &inventory, None, &language).is_none());
        let built = ui
            .build(&state, &inventory, Some(false), &language)
            .unwrap();
        assert_eq!(built.buttons.len(), 1);
        assert_eq!(built.buttons[0][0].id, "iw4l:rifle_ammo");
        assert!(!built.buttons[0][0].craftable);
        assert_eq!(built.tabs, [true, true, false, false, false]);
        assert_eq!(built.pages, 1);
        assert!(
            ui.build(&state, &inventory, Some(false), &language)
                .is_none()
        );
        inventory.unlock_recipe("minecraft:stick");
        inventory.slots[0] = Some(ItemStack::new("minecraft:oak_planks", 2));
        let built = ui
            .build(&state, &inventory, Some(false), &language)
            .unwrap();
        let ids: Vec<(&str, bool)> = built
            .buttons
            .iter()
            .map(|button| (button[0].id.as_str(), button[0].craftable))
            .collect();
        assert_eq!(ids, [("iw4l:rifle_ammo", false), ("minecraft:stick", true)]);
        state.filtering = true;
        let built = ui
            .build(&state, &inventory, Some(false), &language)
            .unwrap();
        assert_eq!(built.buttons.len(), 1);
        state.filtering = false;
        state.tab = Tab::Redstone.index();
        let built = ui
            .build(&state, &inventory, Some(false), &language)
            .unwrap();
        assert_eq!(built.buttons.len(), 2, "a hidden tab shows the search tab");
        state.tab = Tab::Misc.index();
        state.query = "rifle".to_owned();
        let built = ui
            .build(&state, &inventory, Some(false), &language)
            .unwrap();
        assert!(built.buttons.is_empty());
        state.tab = 0;
        let built = ui
            .build(&state, &inventory, Some(false), &language)
            .unwrap();
        assert_eq!(built.buttons.len(), 1);
        assert!(built.buttons[0][0].cells.is_empty());
        state.overlay = Some(0);
        let built = ui
            .build(&state, &inventory, Some(false), &language)
            .unwrap();
        assert_eq!(built.buttons[0][0].cells.len(), 9);
        assert_eq!(built.tab_icons[1].len(), 2);
    }

    #[test]
    fn a_clicked_recipe_shows_its_ghost_in_the_open_grid() {
        let mut inventory = inventory();
        inventory.unlock_recipe("minecraft:stick");
        let mut ui = RecipeBookUi::default();
        let state = McRecipeBook {
            ghost: Some("minecraft:stick".to_owned()),
            ..McRecipeBook::default()
        };
        let language = HashMap::new();
        let built = ui
            .build(&state, &inventory, Some(false), &language)
            .unwrap();
        let ghost: Vec<(McSlot, &str)> = built
            .ghost
            .iter()
            .map(|(slot, stacks)| (*slot, stacks[0].id.as_str()))
            .collect();
        assert_eq!(
            ghost,
            [
                (McSlot::Crafting(0), "minecraft:oak_planks"),
                (McSlot::Crafting(2), "minecraft:oak_planks"),
                (McSlot::Result, "minecraft:stick")
            ]
        );
        assert_eq!(built.ghost[2].1[0].count, 4);
        let built = ui.build(&state, &inventory, Some(true), &language).unwrap();
        let slots: Vec<McSlot> = built.ghost.iter().map(|(slot, _)| *slot).collect();
        assert_eq!(
            slots,
            [
                McSlot::Workbench(1),
                McSlot::Workbench(4),
                McSlot::WorkbenchResult
            ]
        );
        assert_eq!(grid_of(McScreen::CraftingTable), Some(true));
        assert_eq!(grid_of(McScreen::Anvil), None);
        assert!(ui.build(&state, &inventory, None, &language).is_some());
        assert!(ui.build(&state, &inventory, None, &language).is_none());
    }
}
