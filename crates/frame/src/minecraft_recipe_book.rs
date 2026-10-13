use crate::{McSlot, McStack};

pub const MC_RECIPE_TABS: usize = 5;
pub const MC_RECIPES_PER_PAGE: usize = 20;
pub const MC_RECIPE_CYCLE_TICKS: f64 = 30.0;

#[derive(Clone, Debug, PartialEq)]
pub struct McRecipeOption {
    pub id: String,
    pub result: McStack,
    pub craftable: bool,
    pub cells: Vec<Option<Vec<McStack>>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McRecipeButton {
    pub recipes: Vec<McRecipeOption>,
}

impl McRecipeButton {
    pub fn craftable(&self) -> bool {
        self.recipes.iter().any(|recipe| recipe.craftable)
    }

    pub fn selected(&self, filtering: bool) -> Vec<&McRecipeOption> {
        self.recipes
            .iter()
            .filter(|recipe| !filtering || recipe.craftable)
            .collect()
    }

    pub fn alternatives(&self, filtering: bool) -> Vec<&McRecipeOption> {
        let craftable = self.recipes.iter().filter(|recipe| recipe.craftable);
        let missing = self
            .recipes
            .iter()
            .filter(|recipe| !recipe.craftable && !filtering);
        craftable.chain(missing).collect()
    }

    pub fn shown(&self, filtering: bool, ticks: f64) -> Option<&McRecipeOption> {
        let selected = self.selected(filtering);
        let index = cycle(ticks, selected.len())?;
        selected.get(index).copied()
    }

    pub fn same_result(&self, filtering: bool) -> bool {
        let selected = self.selected(filtering);
        selected.windows(2).all(|pair| {
            pair[0].result.id == pair[1].result.id && pair[0].result.name == pair[1].result.name
        })
    }
}

pub fn cycle(ticks: f64, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some((ticks / MC_RECIPE_CYCLE_TICKS).max(0.0) as usize % len)
}

#[derive(Clone, Debug, PartialEq)]
pub struct McRecipeBook {
    pub open: bool,
    pub filtering: bool,
    pub tab: usize,
    pub page: usize,
    pub query: String,
    pub cursor: usize,
    pub highlight: usize,
    pub focused: bool,
    pub ghost: Option<String>,
    pub overlay: Option<usize>,
    pub tabs: [bool; MC_RECIPE_TABS],
    pub tab_icons: Vec<Vec<McStack>>,
    pub pages: usize,
    pub buttons: Vec<McRecipeButton>,
    pub ghost_cells: Vec<(McSlot, Vec<McStack>)>,
}

impl Default for McRecipeBook {
    fn default() -> Self {
        Self {
            open: false,
            filtering: false,
            tab: 0,
            page: 0,
            query: String::new(),
            cursor: 0,
            highlight: 0,
            focused: false,
            ghost: None,
            overlay: None,
            tabs: [true, false, false, false, false],
            tab_icons: Vec::new(),
            pages: 0,
            buttons: Vec::new(),
            ghost_cells: Vec::new(),
        }
    }
}

impl McRecipeBook {
    pub fn reopen(&mut self) {
        self.tab = 0;
        self.page = 0;
        self.query.clear();
        self.cursor = 0;
        self.highlight = 0;
        self.focused = false;
        self.ghost = None;
        self.overlay = None;
    }

    pub fn select_tab(&mut self, tab: usize) {
        if tab != self.tab {
            self.tab = tab;
            self.page = 0;
            self.overlay = None;
        }
    }

    pub fn turn(&mut self, forward: bool) {
        if forward && self.page + 1 < self.pages {
            self.page += 1;
        } else if !forward && self.page > 0 {
            self.page -= 1;
        }
        self.overlay = None;
    }

    pub fn shown_tab(&self) -> usize {
        if self.tabs.get(self.tab).copied().unwrap_or(false) {
            self.tab
        } else {
            0
        }
    }

    pub fn icon_ids(&self) -> impl Iterator<Item = &str> {
        let buttons = self
            .buttons
            .iter()
            .flat_map(|button| &button.recipes)
            .flat_map(|recipe| {
                std::iter::once(&recipe.result).chain(recipe.cells.iter().flatten().flatten())
            });
        let ghosts = self.ghost_cells.iter().flat_map(|(_, stacks)| stacks);
        self.tab_icons
            .iter()
            .flatten()
            .chain(buttons)
            .chain(ghosts)
            .map(|stack| stack.id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: &str) -> McStack {
        McStack {
            id: id.to_owned(),
            count: 1,
            weapon: None,
            damage: None,
            max: 64,
            named: false,
            name: None,
            attack: None,
            enchantments: Vec::new(),
            contents: Vec::new(),
            more: None,
            glint: false,
        }
    }

    fn option(id: &str, result: &str, craftable: bool) -> McRecipeOption {
        McRecipeOption {
            id: id.to_owned(),
            result: stack(result),
            craftable,
            cells: Vec::new(),
        }
    }

    #[test]
    fn a_button_shows_its_craftable_recipes_first_and_hides_the_rest_while_filtering() {
        let button = McRecipeButton {
            recipes: vec![
                option("a", "minecraft:oak_planks", false),
                option("b", "minecraft:birch_planks", true),
                option("c", "minecraft:spruce_planks", false),
            ],
        };
        assert!(button.craftable());
        let ids = |list: Vec<&McRecipeOption>| {
            list.iter()
                .map(|recipe| recipe.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(button.selected(false)), ["a", "b", "c"]);
        assert_eq!(ids(button.selected(true)), ["b"]);
        assert_eq!(ids(button.alternatives(false)), ["b", "a", "c"]);
        assert_eq!(ids(button.alternatives(true)), ["b"]);
        assert_eq!(button.shown(false, 0.0).unwrap().id, "a");
        assert_eq!(button.shown(false, 31.0).unwrap().id, "b");
        assert_eq!(button.shown(false, 95.0).unwrap().id, "a");
        assert_eq!(button.shown(true, 95.0).unwrap().id, "b");
        assert!(!button.same_result(false));
        let empty = McRecipeButton::default();
        assert!(!empty.craftable());
        assert!(empty.shown(false, 10.0).is_none());
        let sticks = McRecipeButton {
            recipes: vec![
                option("a", "minecraft:stick", true),
                option("b", "minecraft:stick", true),
            ],
        };
        assert!(sticks.same_result(false));
    }

    #[test]
    fn turning_pages_stays_inside_the_book_and_a_new_tab_starts_at_its_first_page() {
        let mut book = McRecipeBook {
            pages: 3,
            overlay: Some(2),
            ..McRecipeBook::default()
        };
        book.turn(false);
        assert_eq!(book.page, 0);
        assert_eq!(book.overlay, None);
        book.turn(true);
        book.turn(true);
        book.turn(true);
        assert_eq!(book.page, 2);
        book.select_tab(1);
        assert_eq!((book.tab, book.page), (1, 0));
        book.page = 1;
        book.select_tab(1);
        assert_eq!(book.page, 1);
        assert_eq!(book.shown_tab(), 0);
        book.tabs[1] = true;
        assert_eq!(book.shown_tab(), 1);
        book.query = "stick".to_owned();
        book.ghost = Some("minecraft:stick".to_owned());
        book.open = true;
        book.filtering = true;
        book.reopen();
        assert_eq!(
            (
                book.tab,
                book.page,
                book.query.as_str(),
                book.ghost.as_deref()
            ),
            (0, 0, "", None)
        );
        assert!(book.open && book.filtering);
    }
}
