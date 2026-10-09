use std::collections::HashMap;
use std::sync::{Arc, mpsc};

use frame::{McClick, McCreativeTab, McScreen, McSlot, McTabKind, MinecraftUi};
use minecraft_survival::creative::{self, CreativeSlot, GridClick, SetSlot};
use minecraft_terrain::creative_tabs::{CreativeTab, CreativeTabs, Row, TabEntry, TabKind};
use minecraft_terrain::game_mode::PlayerMode;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};

#[derive(Clone, Debug, PartialEq)]
struct Built {
    tab: String,
    query: String,
    operator: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shown {
    stack: ItemStack,
    locked: bool,
}

#[derive(Default)]
pub(crate) struct CreativeUi {
    started: bool,
    loading: Option<mpsc::Receiver<CreativeTabs>>,
    tabs: Option<Arc<CreativeTabs>>,
    published: Option<(bool, bool)>,
    built: Option<Built>,
    items: Vec<Option<Shown>>,
    pending_tabs: Option<Vec<(McCreativeTab, Option<ItemStack>)>>,
    pending_items: Option<Vec<Option<ItemStack>>>,
    creative_mode: bool,
    operator: bool,
}

pub(crate) fn kind_of(kind: TabKind) -> McTabKind {
    match kind {
        TabKind::Category => McTabKind::Category,
        TabKind::Hotbar => McTabKind::Hotbar,
        TabKind::Search => McTabKind::Search,
        TabKind::Inventory => McTabKind::Inventory,
    }
}

pub(crate) fn stack_of(recipes: &RecipeBook, id: &str) -> ItemStack {
    let mut stack = ItemStack::new(id, 1);
    stack.max = creative::max_stack(recipes, &stack);
    stack
}

pub(crate) fn entry_stacks(entries: &[&TabEntry], recipes: &RecipeBook) -> Vec<ItemStack> {
    let plain: std::collections::HashSet<&str> = entries
        .iter()
        .filter(|entry| entry.variants.is_none())
        .map(|entry| entry.item.as_str())
        .collect();
    let mut seen = std::collections::HashSet::new();
    entries
        .iter()
        .filter(|entry| entry.variants.is_none() || !plain.contains(entry.item.as_str()))
        .filter(|entry| entry.variants.is_none() || seen.insert(entry.item.as_str()))
        .map(|entry| stack_of(recipes, &entry.item))
        .collect()
}

pub(crate) fn tab_view(
    tab: &CreativeTab,
    language: Option<&HashMap<String, String>>,
) -> McCreativeTab {
    McCreativeTab {
        id: tab.id.clone(),
        title: language
            .and_then(|language| language.get(&tab.title))
            .cloned()
            .unwrap_or_else(|| tab.title.clone()),
        top: tab.row == Row::Top,
        column: tab.column,
        aligned_right: tab.aligned_right,
        kind: kind_of(tab.kind),
        show_title: tab.show_title,
        scroll_bar: tab.scroll_bar,
        background: tab.background.clone(),
        icon: None,
    }
}

impl CreativeUi {
    pub(crate) fn prepare(&mut self, creative_mode: bool, operator: bool) {
        self.creative_mode = creative_mode;
        self.operator = operator;
        if !self.started {
            self.started = true;
            if let Some(jar) = crate::minecraft_world::data_jar() {
                let (send, receive) = mpsc::channel();
                let spawned = std::thread::Builder::new()
                    .name("minecraft-creative-tabs".into())
                    .spawn(move || {
                        let _ = send.send(minecraft_terrain::creative_tabs::creative_tabs(&jar));
                    });
                if spawned.is_ok() {
                    self.loading = Some(receive);
                }
            }
        }
        if let Some(receive) = self.loading.as_ref()
            && let Ok(tabs) = receive.try_recv()
        {
            self.loading = None;
            self.tabs = Some(Arc::new(tabs));
            self.published = None;
            self.built = None;
        }
    }

    pub(crate) fn screen(&self, ui: &MinecraftUi) -> Option<McScreen> {
        if !self.creative_mode {
            return None;
        }
        ui.creative
            .selected_tab()
            .map(|tab| McScreen::Creative(tab.kind))
    }

    pub(crate) fn refresh(
        &mut self,
        ui: &mut MinecraftUi,
        recipes: &RecipeBook,
        language: Option<&HashMap<String, String>>,
    ) {
        let Some(tabs) = self.tabs.clone() else {
            return;
        };
        let wanted = (self.operator, language.is_some());
        if self.published != Some(wanted) {
            self.published = Some(wanted);
            self.built = None;
            self.pending_tabs = Some(
                tabs.shown(self.operator)
                    .map(|tab| {
                        (
                            tab_view(tab, language),
                            tab.icon.as_deref().map(|icon| stack_of(recipes, icon)),
                        )
                    })
                    .collect(),
            );
            return;
        }
        if ui.screen == McScreen::Inventory
            && let Some(screen) = self.screen(ui)
        {
            ui.screen = screen;
        }
        let Some(selected) = ui.creative.selected_tab() else {
            return;
        };
        let built = Built {
            tab: selected.id.clone(),
            query: if selected.kind == McTabKind::Search {
                ui.creative.query.clone()
            } else {
                String::new()
            },
            operator: self.operator,
        };
        if self.built.as_ref() == Some(&built) {
            return;
        }
        let shown: Vec<Option<Shown>> = match selected.kind {
            McTabKind::Category => tabs
                .tab(&selected.id)
                .map(|tab| entry_stacks(&tab.displayed().collect::<Vec<_>>(), recipes))
                .unwrap_or_default()
                .into_iter()
                .map(|stack| {
                    Some(Shown {
                        stack,
                        locked: false,
                    })
                })
                .collect(),
            McTabKind::Search => {
                let entries = tabs.search(self.operator);
                entry_stacks(&entries.iter().collect::<Vec<_>>(), recipes)
                    .into_iter()
                    .map(|stack| {
                        Some(Shown {
                            stack,
                            locked: false,
                        })
                    })
                    .collect()
            }
            McTabKind::Hotbar | McTabKind::Inventory => Vec::new(),
        };
        self.pending_items = Some(
            shown
                .iter()
                .map(|shown| shown.as_ref().map(|shown| shown.stack.clone()))
                .collect(),
        );
        self.items = shown;
        self.built = Some(built);
    }

    pub(crate) fn take_tabs(&mut self) -> Option<Vec<(McCreativeTab, Option<ItemStack>)>> {
        self.pending_tabs.take()
    }

    pub(crate) fn take_items(&mut self) -> Option<Vec<Option<ItemStack>>> {
        self.pending_items.take()
    }

    pub(crate) fn icon_ids(&self, ui: &MinecraftUi) -> Vec<String> {
        if !matches!(ui.screen, McScreen::Creative(_)) || !ui.inventory_open {
            return Vec::new();
        }
        let first = ui.creative.first_shown();
        let window = frame::minecraft_creative::MC_CREATIVE_COLUMNS
            * frame::minecraft_creative::MC_CREATIVE_ROWS;
        let tabs = ui.creative.tabs.iter().filter_map(|tab| tab.icon.as_ref());
        let items = ui.creative.items.iter().skip(first).take(window).flatten();
        tabs.chain(items).map(|stack| stack.id.clone()).collect()
    }

    fn grid(&self, inventory: &Inventory, index: usize, click: GridClick) -> Vec<SetSlot> {
        let Some(shown) = self.items.get(index) else {
            return Vec::new();
        };
        if shown.as_ref().is_some_and(|shown| shown.locked) {
            return Vec::new();
        }
        let recipes = inventory.recipes.clone();
        let max = |stack: &ItemStack| creative::max_stack(&recipes, stack);
        creative::grid_click(
            inventory.cursor.as_ref(),
            shown.as_ref().map(|shown| &shown.stack),
            click,
            &max,
        )
    }

    pub(crate) fn apply(
        &mut self,
        ui: &mut MinecraftUi,
        mode: &PlayerMode,
        inventory: &mut Inventory,
        known: &dyn Fn(&str) -> bool,
    ) -> Vec<ItemStack> {
        let McScreen::Creative(kind) = ui.screen else {
            return Vec::new();
        };
        let mut dropped = Vec::new();
        let mut rest = Vec::new();
        for click in std::mem::take(&mut ui.clicks) {
            let sets = match &click {
                McClick::Slot {
                    slot: McSlot::Creative(index),
                    right,
                    shift,
                } => self.grid(
                    inventory,
                    *index,
                    GridClick::Pick {
                        right: *right,
                        shift: *shift,
                    },
                ),
                McClick::Swap {
                    slot: McSlot::Creative(index),
                    hotbar,
                } => self.grid(inventory, *index, GridClick::Swap(*hotbar)),
                McClick::Throw {
                    slot: McSlot::Creative(index),
                    whole,
                } => self.grid(inventory, *index, GridClick::Throw { whole: *whole }),
                McClick::Clone(McSlot::Creative(index)) => {
                    self.grid(inventory, *index, GridClick::Clone)
                }
                McClick::Clone(McSlot::Inventory(index)) => match (
                    &inventory.cursor,
                    inventory.slots.get(*index).cloned().flatten(),
                ) {
                    (None, Some(stack)) => {
                        let max = creative::max_stack(&inventory.recipes, &stack);
                        vec![(
                            CreativeSlot::Carried,
                            Some(ItemStack {
                                count: max,
                                ..stack
                            }),
                        )]
                    }
                    _ => Vec::new(),
                },
                McClick::Clone(_)
                | McClick::Gather {
                    slot: McSlot::Creative(_),
                    ..
                } => Vec::new(),
                McClick::Slot {
                    slot: McSlot::Trash,
                    shift,
                    ..
                } => creative::trash_click(*shift),
                McClick::Slot {
                    slot: McSlot::Inventory(index),
                    shift: true,
                    ..
                } if kind != McTabKind::Inventory => creative::hotbar_quick_move(*index),
                _ => {
                    rest.push(click);
                    continue;
                }
            };
            let (thrown, refused) = creative::apply(mode, inventory, sets, known);
            dropped.extend(thrown);
            for why in refused {
                diag::warn!(World, "Minecraft creative slot refused: {why:?}");
            }
        }
        ui.clicks = rest;
        dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(item: &str, variants: Option<minecraft_terrain::creative_tabs::Variants>) -> TabEntry {
        TabEntry {
            item: item.to_owned(),
            variants,
            in_tab: true,
            in_search: true,
        }
    }

    #[test]
    fn variant_entries_show_their_plain_item_once_until_expanded() {
        use minecraft_terrain::creative_tabs::Variants;
        let entries = [
            entry("minecraft:painting", None),
            entry(
                "minecraft:painting",
                Some(Variants::Paintings {
                    tag: "minecraft:placeable".into(),
                    in_tag: true,
                }),
            ),
            entry("minecraft:potion", Some(Variants::Potions)),
            entry("minecraft:stone", None),
            entry("minecraft:potion", Some(Variants::Potions)),
        ];
        let stacks = entry_stacks(&entries.iter().collect::<Vec<_>>(), &RecipeBook::default());
        let ids: Vec<&str> = stacks.iter().map(|stack| stack.id.as_str()).collect();
        assert_eq!(
            ids,
            ["minecraft:painting", "minecraft:potion", "minecraft:stone"]
        );
        assert!(
            stacks
                .iter()
                .all(|stack| stack.count == 1 && stack.max == 64)
        );
    }

    #[test]
    fn tabs_take_their_title_from_the_language() {
        let tab = CreativeTab {
            id: "minecraft:building_blocks".into(),
            title: "itemGroup.buildingBlocks".into(),
            row: Row::Bottom,
            column: 3,
            kind: TabKind::Inventory,
            aligned_right: true,
            show_title: false,
            scroll_bar: false,
            background: "inventory".into(),
            icon: Some("minecraft:chest".into()),
            operator: false,
            entries: Vec::new(),
        };
        let mut language = HashMap::new();
        language.insert(
            "itemGroup.buildingBlocks".to_owned(),
            "Building Blocks".to_owned(),
        );
        let view = tab_view(&tab, Some(&language));
        assert_eq!(view.title, "Building Blocks");
        assert!(!view.top && view.aligned_right && !view.show_title && !view.scroll_bar);
        assert_eq!(view.kind, McTabKind::Inventory);
        assert_eq!(tab_view(&tab, None).title, "itemGroup.buildingBlocks");
    }
}
