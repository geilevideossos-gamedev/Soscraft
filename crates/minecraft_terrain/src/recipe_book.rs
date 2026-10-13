use std::collections::{HashMap, HashSet};

use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::ItemStack;

use crate::creative_search::{Searched, search};

pub const PER_PAGE: usize = 20;
pub const STORAGE: usize = 36;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Search,
    Equipment,
    Building,
    Misc,
    Redstone,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Search,
        Tab::Equipment,
        Tab::Building,
        Tab::Misc,
        Tab::Redstone,
    ];
    pub const SEARCHED: [Tab; 4] = [Tab::Equipment, Tab::Building, Tab::Misc, Tab::Redstone];

    pub fn of_category(category: &str) -> Tab {
        match category {
            "equipment" => Tab::Equipment,
            "building" => Tab::Building,
            "redstone" => Tab::Redstone,
            _ => Tab::Misc,
        }
    }

    pub fn index(self) -> usize {
        Tab::ALL.iter().position(|tab| *tab == self).unwrap_or(0)
    }

    pub fn at(index: usize) -> Tab {
        Tab::ALL.get(index).copied().unwrap_or(Tab::Search)
    }

    pub fn icons(self) -> &'static [&'static str] {
        match self {
            Tab::Search => &["minecraft:compass"],
            Tab::Equipment => &["minecraft:iron_axe", "minecraft:golden_sword"],
            Tab::Building => &["minecraft:bricks"],
            Tab::Misc => &["minecraft:lava_bucket", "minecraft:apple"],
            Tab::Redstone => &["minecraft:redstone"],
        }
    }
}

pub type Cells = Vec<Option<Vec<String>>>;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: String,
    pub tab: Tab,
    pub group: Option<String>,
    pub result: ItemStack,
    pub table: Cells,
    pub small: Option<Cells>,
}

impl Entry {
    pub fn grid(&self, table: bool) -> Option<&Cells> {
        if table {
            Some(&self.table)
        } else {
            self.small.as_ref()
        }
    }

    pub fn fits(&self, table: bool) -> bool {
        self.grid(table).is_some()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Collection {
    pub tab: Tab,
    pub recipes: Vec<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Known {
    pub entries: Vec<Entry>,
    pub collections: Vec<Collection>,
}

pub fn always_known(id: &str) -> bool {
    crate::fork_items::is_fork(id)
}

pub fn known(book: &RecipeBook, unlocked: &HashSet<String>) -> Known {
    let entries: Vec<Entry> = book
        .crafting_recipes()
        .filter(|recipe| unlocked.contains(recipe.id) || always_known(recipe.id))
        .filter_map(|recipe| {
            let table = book.crafting_grid_candidates(recipe.id, 3, 3)?;
            Some(Entry {
                id: recipe.id.to_owned(),
                tab: Tab::of_category(recipe.category),
                group: recipe.group.map(str::to_owned),
                result: recipe.result.clone(),
                table,
                small: book.crafting_grid_candidates(recipe.id, 2, 2),
            })
        })
        .collect();
    let collections = collections(&entries);
    Known {
        entries,
        collections,
    }
}

pub fn collections(entries: &[Entry]) -> Vec<Collection> {
    let mut out: Vec<Collection> = Vec::new();
    let mut groups: HashMap<(Tab, &str), usize> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        match entry.group.as_deref() {
            None => out.push(Collection {
                tab: entry.tab,
                recipes: vec![index],
            }),
            Some(group) => match groups.get(&(entry.tab, group)) {
                Some(&at) => out[at].recipes.push(index),
                None => {
                    groups.insert((entry.tab, group), out.len());
                    out.push(Collection {
                        tab: entry.tab,
                        recipes: vec![index],
                    });
                }
            },
        }
    }
    out
}

pub fn in_tab(known: &Known, tab: Tab) -> Vec<usize> {
    let tabs: &[Tab] = match tab {
        Tab::Search => &Tab::SEARCHED,
        _ => std::slice::from_ref(&tab),
    };
    tabs.iter()
        .flat_map(|wanted| {
            known
                .collections
                .iter()
                .enumerate()
                .filter(move |(_, collection)| collection.tab == *wanted)
                .map(|(index, _)| index)
        })
        .collect()
}

pub fn counts<'a>(stacks: impl IntoIterator<Item = &'a Option<ItemStack>>) -> HashMap<String, u32> {
    let mut counts = HashMap::new();
    for stack in stacks.into_iter().flatten() {
        if stack.components.is_none() && stack.count > 0 {
            *counts.entry(stack.id.clone()).or_insert(0) += u32::from(stack.count);
        }
    }
    counts
}

pub fn craftable(cells: &Cells, counts: &HashMap<String, u32>) -> bool {
    let needs: Vec<&[String]> = cells.iter().flatten().map(Vec::as_slice).collect();
    let mut holders: HashMap<&str, Vec<usize>> = HashMap::new();
    (0..needs.len()).all(|cell| {
        let mut seen = HashSet::new();
        augment(cell, &needs, counts, &mut holders, &mut seen)
    })
}

fn augment<'a>(
    cell: usize,
    needs: &[&'a [String]],
    counts: &HashMap<String, u32>,
    holders: &mut HashMap<&'a str, Vec<usize>>,
    seen: &mut HashSet<&'a str>,
) -> bool {
    for id in needs[cell] {
        let id = id.as_str();
        let capacity = counts.get(id).copied().unwrap_or(0) as usize;
        if capacity == 0 || !seen.insert(id) {
            continue;
        }
        let held = holders.entry(id).or_default();
        if held.len() < capacity {
            held.push(cell);
            return true;
        }
        let others = held.clone();
        for other in others {
            if augment(other, needs, counts, holders, seen) {
                let held = holders.entry(id).or_default();
                if let Some(at) = held.iter().position(|&taken| taken == other) {
                    held[at] = cell;
                }
                return true;
            }
        }
    }
    false
}

pub fn matches(known: &Known, query: &str, lines: impl Fn(&ItemStack) -> Vec<String>) -> Vec<bool> {
    let lines: Vec<Vec<String>> = known
        .entries
        .iter()
        .map(|entry| lines(&entry.result))
        .collect();
    let searched: Vec<Searched> = known
        .entries
        .iter()
        .zip(&lines)
        .map(|(entry, lines)| Searched {
            id: &entry.result.id,
            lines,
            tags: &[],
        })
        .collect();
    let mut found = vec![false; known.entries.len()];
    for index in search(query.trim(), &searched) {
        found[index] = true;
    }
    found
}

pub struct View<'a> {
    pub tab: Tab,
    pub table: bool,
    pub filtering: bool,
    pub craftable: &'a [bool],
    pub matches: Option<&'a [bool]>,
}

impl View<'_> {
    pub fn selected(&self, known: &Known, collection: usize) -> Vec<usize> {
        known.collections[collection]
            .recipes
            .iter()
            .copied()
            .filter(|&entry| known.entries[entry].fits(self.table))
            .collect()
    }

    pub fn has_craftable(&self, known: &Known, collection: usize) -> bool {
        self.selected(known, collection)
            .iter()
            .any(|&entry| self.craftable.get(entry).copied().unwrap_or(false))
    }

    pub fn shown(&self, known: &Known) -> Vec<usize> {
        in_tab(known, self.tab)
            .into_iter()
            .filter(|&collection| !self.selected(known, collection).is_empty())
            .filter(|&collection| {
                self.matches.is_none_or(|found| {
                    known.collections[collection]
                        .recipes
                        .iter()
                        .any(|&entry| found.get(entry).copied().unwrap_or(false))
                })
            })
            .filter(|&collection| !self.filtering || self.has_craftable(known, collection))
            .collect()
    }

    pub fn tab_visible(&self, known: &Known, tab: Tab) -> bool {
        tab == Tab::Search
            || in_tab(known, tab)
                .into_iter()
                .any(|collection| !self.selected(known, collection).is_empty())
    }
}

pub fn pages(collections: usize) -> usize {
    collections.div_ceil(PER_PAGE)
}

pub fn page_of(page: usize, pages: usize) -> usize {
    if page >= pages { 0 } else { page }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn shaped(pattern: &[&str], key: serde_json::Value, result: &str) -> serde_json::Value {
        json!({"type": "minecraft:crafting_shaped", "category": "equipment", "pattern": pattern, "key": key, "result": {"id": result, "count": 1}})
    }

    fn book() -> RecipeBook {
        RecipeBook::default().with_extra(vec![
            (
                "minecraft:stick".to_owned(),
                json!({"type": "minecraft:crafting_shaped", "category": "misc", "group": "sticks", "pattern": ["#", "#"], "key": {"#": ["minecraft:oak_planks", "minecraft:birch_planks"]}, "result": {"id": "minecraft:stick", "count": 4}}),
            ),
            (
                "minecraft:bamboo_stick".to_owned(),
                json!({"type": "minecraft:crafting_shaped", "category": "misc", "group": "sticks", "pattern": ["#", "#"], "key": {"#": "minecraft:bamboo"}, "result": {"id": "minecraft:stick", "count": 1}}),
            ),
            (
                "minecraft:chest".to_owned(),
                json!({"type": "minecraft:crafting_shaped", "category": "misc", "pattern": ["###", "# #", "###"], "key": {"#": ["minecraft:oak_planks", "minecraft:birch_planks"]}, "result": {"id": "minecraft:chest", "count": 1}}),
            ),
            (
                "minecraft:iron_sword".to_owned(),
                shaped(&["I", "I", "S"], json!({"I": "minecraft:iron_ingot", "S": "minecraft:stick"}), "minecraft:iron_sword"),
            ),
            (
                "minecraft:bricks".to_owned(),
                json!({"type": "minecraft:crafting_shaped", "category": "building", "pattern": ["BB", "BB"], "key": {"B": "minecraft:brick"}, "result": {"id": "minecraft:bricks", "count": 1}}),
            ),
            (
                "iw4l:rifle_ammo".to_owned(),
                json!({"type": "minecraft:crafting_shapeless", "category": "equipment", "group": "iw4l_ammo", "ingredients": ["minecraft:copper_ingot", "minecraft:iron_nugget", "minecraft:gunpowder"], "result": {"id": "iw4l:rifle_ammo", "count": 16}}),
            ),
            (
                "iw4l:pistol_ammo".to_owned(),
                json!({"type": "minecraft:crafting_shapeless", "category": "equipment", "group": "iw4l_ammo", "ingredients": ["minecraft:copper_ingot", "minecraft:gunpowder"], "result": {"id": "iw4l:pistol_ammo", "count": 24}}),
            ),
            (
                "iw4l:m4".to_owned(),
                json!({"type": "minecraft:crafting_shaped", "category": "equipment", "group": "iw4l_assault_rifles", "pattern": ["BKF", " MX"], "key": {"B": "iw4l:gun_barrel", "K": "minecraft:iron_block", "F": "minecraft:repeater", "M": "iw4l:firing_mechanism", "X": "minecraft:iron_ingot"}, "result": {"id": "iw4l:m4", "count": 1}}),
            ),
        ])
    }

    fn unlocked(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    fn stacks(items: &[(&str, u8)]) -> Vec<Option<ItemStack>> {
        items
            .iter()
            .map(|(id, count)| Some(ItemStack::new(*id, *count)))
            .collect()
    }

    #[test]
    fn only_unlocked_vanilla_recipes_and_every_fork_recipe_are_known() {
        let known = known(&book(), &unlocked(&["minecraft:stick", "minecraft:chest"]));
        let ids: Vec<&str> = known
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "minecraft:stick",
                "minecraft:chest",
                "iw4l:rifle_ammo",
                "iw4l:pistol_ammo",
                "iw4l:m4"
            ]
        );
        let tabs: Vec<Tab> = known.entries.iter().map(|entry| entry.tab).collect();
        assert_eq!(
            tabs,
            [
                Tab::Misc,
                Tab::Misc,
                Tab::Equipment,
                Tab::Equipment,
                Tab::Equipment
            ]
        );
        assert!(known.entries[0].fits(false) && known.entries[0].fits(true));
        assert!(!known.entries[1].fits(false) && known.entries[1].fits(true));
        assert!(!known.entries[4].fits(false));
        assert_eq!(Tab::of_category("building"), Tab::Building);
        assert_eq!(Tab::of_category("redstone"), Tab::Redstone);
        assert_eq!(Tab::of_category("whatever"), Tab::Misc);
    }

    #[test]
    fn recipes_of_one_group_and_tab_share_one_collection_at_the_first_member() {
        let known = known(
            &book(),
            &unlocked(&[
                "minecraft:stick",
                "minecraft:bamboo_stick",
                "minecraft:chest",
                "minecraft:iron_sword",
                "minecraft:bricks",
            ]),
        );
        let shape: Vec<(Tab, Vec<&str>)> = known
            .collections
            .iter()
            .map(|collection| {
                (
                    collection.tab,
                    collection
                        .recipes
                        .iter()
                        .map(|&entry| known.entries[entry].id.as_str())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                (Tab::Misc, vec!["minecraft:stick", "minecraft:bamboo_stick"]),
                (Tab::Misc, vec!["minecraft:chest"]),
                (Tab::Equipment, vec!["minecraft:iron_sword"]),
                (Tab::Building, vec!["minecraft:bricks"]),
                (Tab::Equipment, vec!["iw4l:rifle_ammo", "iw4l:pistol_ammo"]),
                (Tab::Equipment, vec!["iw4l:m4"]),
            ]
        );
        let titles = |tab| {
            in_tab(&known, tab)
                .into_iter()
                .map(|collection| {
                    known.entries[known.collections[collection].recipes[0]]
                        .id
                        .clone()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            titles(Tab::Search),
            [
                "minecraft:iron_sword",
                "iw4l:rifle_ammo",
                "iw4l:m4",
                "minecraft:bricks",
                "minecraft:stick",
                "minecraft:chest"
            ]
        );
        assert_eq!(titles(Tab::Redstone), Vec::<String>::new());
    }

    #[test]
    fn a_recipe_is_craftable_only_when_every_cell_gets_its_own_item() {
        let known = known(&book(), &unlocked(&["minecraft:chest", "minecraft:stick"]));
        let chest = &known.entries[1].table;
        let held = counts(&stacks(&[
            ("minecraft:oak_planks", 5),
            ("minecraft:birch_planks", 2),
        ]));
        assert!(!craftable(chest, &held));
        let held = counts(&stacks(&[
            ("minecraft:oak_planks", 5),
            ("minecraft:birch_planks", 3),
        ]));
        assert!(craftable(chest, &held));
        let ammo = &known.entries[2].table;
        assert!(!craftable(
            ammo,
            &counts(&stacks(&[
                ("minecraft:copper_ingot", 1),
                ("minecraft:gunpowder", 1)
            ]))
        ));
        assert!(craftable(
            ammo,
            &counts(&stacks(&[
                ("minecraft:copper_ingot", 1),
                ("minecraft:gunpowder", 1),
                ("minecraft:iron_nugget", 1)
            ]))
        ));
        let mut named = ItemStack::new("minecraft:iron_nugget", 1);
        named.components = Some(json!({"minecraft:custom_name": "x"}));
        let held = counts(&[
            Some(ItemStack::new("minecraft:copper_ingot", 1)),
            Some(ItemStack::new("minecraft:gunpowder", 1)),
            Some(named),
        ]);
        assert!(!craftable(ammo, &held));
    }

    #[test]
    fn the_shown_list_keeps_fitting_matching_and_craftable_collections() {
        let known = known(
            &book(),
            &unlocked(&[
                "minecraft:stick",
                "minecraft:bamboo_stick",
                "minecraft:chest",
                "minecraft:iron_sword",
                "minecraft:bricks",
            ]),
        );
        let held = counts(&stacks(&[
            ("minecraft:bamboo", 2),
            ("minecraft:iron_ingot", 2),
            ("minecraft:stick", 1),
        ]));
        let craftable: Vec<bool> = known
            .entries
            .iter()
            .map(|entry| craftable(&entry.table, &held))
            .collect();
        let first = |list: Vec<usize>| -> Vec<String> {
            list.into_iter()
                .map(|collection| {
                    known.entries[known.collections[collection].recipes[0]]
                        .id
                        .clone()
                })
                .collect()
        };
        let small = View {
            tab: Tab::Search,
            table: false,
            filtering: false,
            craftable: &craftable,
            matches: None,
        };
        assert_eq!(
            first(small.shown(&known)),
            ["iw4l:rifle_ammo", "minecraft:bricks", "minecraft:stick"]
        );
        assert!(!small.tab_visible(&known, Tab::Redstone));
        assert!(small.tab_visible(&known, Tab::Search));
        assert!(small.tab_visible(&known, Tab::Building));
        let table = View {
            table: true,
            filtering: true,
            ..small
        };
        assert_eq!(
            first(table.shown(&known)),
            ["minecraft:iron_sword", "minecraft:stick"]
        );
        let sticks = known
            .collections
            .iter()
            .position(|collection| collection.recipes.len() == 2 && collection.tab == Tab::Misc)
            .unwrap();
        assert_eq!(table.selected(&known, sticks).len(), 2);
        assert!(table.has_craftable(&known, sticks));
        let found = matches(&known, "Stick", |stack| {
            vec![stack.id.replace("minecraft:", "")]
        });
        let searching = View {
            matches: Some(&found),
            filtering: false,
            ..table
        };
        assert_eq!(first(searching.shown(&known)), ["minecraft:stick"]);
        let by_id = matches(&known, "iw4l:", |_| Vec::new());
        assert_eq!(by_id.iter().filter(|found| **found).count(), 3);
    }

    #[test]
    fn twenty_recipes_fill_a_page_and_a_page_past_the_end_goes_back_to_the_first() {
        assert_eq!(pages(0), 0);
        assert_eq!(pages(20), 1);
        assert_eq!(pages(21), 2);
        assert_eq!(page_of(1, 2), 1);
        assert_eq!(page_of(2, 2), 0);
        assert_eq!(page_of(0, 0), 0);
        assert_eq!(Tab::at(3), Tab::Misc);
        assert_eq!(Tab::Redstone.index(), 4);
        assert_eq!(
            Tab::Equipment.icons(),
            ["minecraft:iron_axe", "minecraft:golden_sword"]
        );
    }

    #[test]
    fn a_shaped_recipe_is_centred_in_the_table_only_when_it_is_narrow() {
        let known = known(&book(), &unlocked(&["minecraft:iron_sword"]));
        let sword = &known.entries[0];
        let ids: Vec<Option<&str>> = sword
            .table
            .iter()
            .map(|cell| cell.as_ref().map(|ids| ids[0].as_str()))
            .collect();
        assert_eq!(
            ids,
            [
                None,
                Some("minecraft:iron_ingot"),
                None,
                None,
                Some("minecraft:iron_ingot"),
                None,
                None,
                Some("minecraft:stick"),
                None
            ]
        );
        assert!(sword.small.is_none());
    }
}
