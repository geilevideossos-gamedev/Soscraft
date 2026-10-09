use std::collections::HashMap;
use std::sync::{Arc, mpsc};

use frame::{McClick, McCreativeTab, McScreen, McSlot, McTabKind, MinecraftUi};
use minecraft_survival::creative::{self, CreativeSlot, GridClick, SetSlot};
use minecraft_terrain::creative_search::Searched;
use minecraft_terrain::creative_tabs::{CreativeTab, CreativeTabs, Row, TabEntry, TabKind};
use minecraft_terrain::creative_variants::VariantData;
use minecraft_terrain::game_mode::PlayerMode;
use minecraft_terrain::hotbars::{self, Hotbars};
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};

#[derive(Clone, Debug, PartialEq)]
struct Built {
    tab: String,
    query: String,
    operator: bool,
    hotbars: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shown {
    stack: ItemStack,
    locked: bool,
    name: Option<String>,
}

pub(crate) struct SearchList {
    key: (bool, bool),
    stacks: Vec<ItemStack>,
    lines: Vec<Vec<String>>,
}

pub(crate) fn search_lines(
    stack: &ItemStack,
    language: Option<&HashMap<String, String>>,
    variants: Option<&VariantData>,
) -> Vec<String> {
    let Some(language) = language else {
        return vec![stack.id.clone()];
    };
    let text = |key: String| language.get(&key).cloned();
    let item = stack.id.rsplit(':').next().unwrap_or(&stack.id);
    let potion = minecraft_terrain::stations::potion_of(stack).and_then(|potion| {
        let effect = potion
            .rsplit(':')
            .next()?
            .trim_start_matches("long_")
            .trim_start_matches("strong_");
        text(format!("item.minecraft.{item}.effect.{effect}"))
    });
    let mut lines = vec![
        potion.unwrap_or_else(|| minecraft_terrain::item_icons::item_name(language, &stack.id)),
    ];
    for (id, level) in minecraft_terrain::stations::enchantments_of(stack) {
        let name =
            text(format!("enchantment.{}", id.replacen(':', ".", 1))).unwrap_or_else(|| id.clone());
        let single = variants.is_some_and(|data| {
            data.enchantments
                .iter()
                .any(|(known, max)| *known == id && *max == 1)
        });
        lines.push(if level == 1 && single {
            name
        } else {
            format!("{name} {}", minecraft_terrain::stations::roman(level))
        });
    }
    let component = |key: &str| {
        stack
            .components
            .as_ref()
            .and_then(|components| components.get(key))
    };
    if let Some(instrument) = component("minecraft:instrument").and_then(serde_json::Value::as_str)
    {
        lines.extend(text(format!(
            "instrument.{}",
            instrument.replacen(':', ".", 1)
        )));
    }
    if let Some(painting) =
        component("minecraft:painting/variant").and_then(serde_json::Value::as_str)
    {
        let key = format!("painting.{}", painting.replacen(':', ".", 1));
        lines.extend(text(format!("{key}.title")));
        lines.extend(text(format!("{key}.author")));
    }
    if let Some(flight) =
        component("minecraft:fireworks").and_then(|fireworks| fireworks.get("flight_duration"))
    {
        let label =
            text("item.minecraft.firework_rocket.flight".to_owned()).unwrap_or_else(String::new);
        lines.push(format!("{label} {flight}"));
    }
    lines
}

pub(crate) fn search_items(
    query: &str,
    search: &SearchList,
    item_tags: &HashMap<String, Vec<String>>,
) -> Vec<usize> {
    let searched: Vec<Searched> = search
        .stacks
        .iter()
        .zip(&search.lines)
        .map(|(stack, lines)| Searched {
            id: &stack.id,
            lines,
            tags: item_tags.get(&stack.id).map_or(&[][..], Vec::as_slice),
        })
        .collect();
    minecraft_terrain::creative_search::search(query, &searched)
}

pub(crate) fn tabs_of(
    tabs: &CreativeTabs,
    operator: bool,
    language: Option<&HashMap<String, String>>,
) -> HashMap<String, Vec<String>> {
    let mut found: HashMap<String, Vec<String>> = HashMap::new();
    for tab in tabs
        .shown(operator)
        .filter(|tab| tab.kind == TabKind::Category)
    {
        let title = tab_view(tab, language).title;
        for entry in tab.entries.iter().filter(|entry| entry.in_search) {
            let titles = found.entry(entry.item.clone()).or_default();
            if titles.last() != Some(&title) {
                titles.push(title.clone());
            }
        }
    }
    found
}

pub(crate) const SAVE_KEY: &str = "C";
pub(crate) const LOAD_KEY: &str = "X";

pub(crate) fn hotbar_path() -> std::path::PathBuf {
    std::path::Path::new("iw4l-artifacts").join(hotbars::FILE)
}

pub(crate) fn format_text(template: &str, args: &[&str]) -> String {
    let mut out = template.to_owned();
    for (index, arg) in args.iter().enumerate() {
        out = out.replace(&format!("%{}$s", index + 1), arg);
    }
    for arg in args {
        if !out.contains("%s") {
            break;
        }
        out = out.replacen("%s", arg, 1);
    }
    out
}

#[derive(Default)]
pub(crate) struct CreativeUi {
    started: bool,
    loading: Option<mpsc::Receiver<(CreativeTabs, Option<VariantData>)>>,
    variants: Option<Arc<VariantData>>,
    tabs: Option<Arc<CreativeTabs>>,
    published: Option<(bool, bool)>,
    built: Option<Built>,
    items: Vec<Option<Shown>>,
    pending_tabs: Option<Vec<(McCreativeTab, Option<ItemStack>)>>,
    pending_items: Option<Vec<Option<ItemStack>>>,
    creative_mode: bool,
    operator: bool,
    search: Option<SearchList>,
    item_tags: HashMap<String, Vec<String>>,
    tag_names: Vec<String>,
    hotbars: Option<Hotbars>,
    hotbars_version: u64,
    texts: Option<(String, String)>,
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

pub(crate) fn entry_stacks(
    entries: &[&TabEntry],
    recipes: &RecipeBook,
    variants: Option<&VariantData>,
) -> Vec<ItemStack> {
    let plain: std::collections::HashSet<&str> = entries
        .iter()
        .filter(|entry| entry.variants.is_none())
        .map(|entry| entry.item.as_str())
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut expanded = std::collections::HashSet::new();
    let mut out = Vec::new();
    for entry in entries {
        let Some(kind) = entry.variants.as_ref() else {
            out.push(stack_of(recipes, &entry.item));
            continue;
        };
        match variants.and_then(|data| data.expand(kind)) {
            Some(list) => {
                for components in list {
                    if expanded.insert((entry.item.clone(), components.to_string())) {
                        let mut stack = ItemStack::new(entry.item.as_str(), 1);
                        stack.components = Some(components);
                        stack.max = creative::max_stack(recipes, &stack);
                        out.push(stack);
                    }
                }
            }
            None => {
                if !plain.contains(entry.item.as_str()) && seen.insert(entry.item.as_str()) {
                    out.push(stack_of(recipes, &entry.item));
                }
            }
        }
    }
    out
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
                        let tabs = minecraft_terrain::creative_tabs::creative_tabs(&jar);
                        let variants = VariantData::read(&jar)
                            .map_err(|error| eprintln!("creative variants unavailable: {error:#}"))
                            .ok();
                        let _ = send.send((tabs, variants));
                    });
                if spawned.is_ok() {
                    self.loading = Some(receive);
                }
            }
        }
        if let Some(receive) = self.loading.as_ref()
            && let Ok((tabs, variants)) = receive.try_recv()
        {
            self.loading = None;
            self.tabs = Some(Arc::new(tabs));
            self.variants = variants.map(Arc::new);
            self.search = None;
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
        if self.texts.is_none()
            && let Some(language) = language
        {
            let text = |key: &str| language.get(key).cloned().unwrap_or_else(|| key.to_owned());
            self.texts = Some((text("inventory.hotbarInfo"), text("inventory.hotbarSaved")));
            self.built = None;
        }
        let Some(tabs) = self.tabs.clone() else {
            return;
        };
        let wanted = (self.operator, language.is_some());
        if self.published != Some(wanted) {
            self.published = Some(wanted);
            self.built = None;
            ui.creative.tabs_of = tabs_of(&tabs, self.operator, language);
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
        let Some(selected) = ui.creative.selected_tab().cloned() else {
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
            hotbars: self.hotbars_version,
        };
        if self.built.as_ref() == Some(&built) {
            return;
        }
        let unlocked = |stack: ItemStack| {
            Some(Shown {
                stack,
                locked: false,
                name: None,
            })
        };
        let mut tags = Vec::new();
        let shown: Vec<Option<Shown>> = match selected.kind {
            McTabKind::Category => tabs
                .tab(&selected.id)
                .map(|tab| {
                    entry_stacks(
                        &tab.displayed().collect::<Vec<_>>(),
                        recipes,
                        self.variants.as_deref(),
                    )
                })
                .unwrap_or_else(Vec::new)
                .into_iter()
                .map(unlocked)
                .collect(),
            McTabKind::Search => {
                self.ensure_search(&tabs, recipes, language);
                let Some(search) = self.search.as_ref() else {
                    return;
                };
                let found = search_items(&built.query, search, &self.item_tags);
                let visible = minecraft_terrain::creative_search::visible_tags(
                    &built.query,
                    self.tag_names.iter().map(String::as_str),
                );
                tags = found
                    .iter()
                    .map(|&index| {
                        let own = self.item_tags.get(&search.stacks[index].id);
                        visible
                            .iter()
                            .filter(|tag| own.is_some_and(|own| own.contains(tag)))
                            .cloned()
                            .collect()
                    })
                    .collect();
                found
                    .into_iter()
                    .map(|index| unlocked(search.stacks[index].clone()))
                    .collect()
            }
            McTabKind::Hotbar => {
                let info = self.texts.as_ref().map(|texts| texts.0.clone());
                let hotbars = self.hotbars().clone();
                let mut shown = Vec::new();
                for row in 0..hotbars::GROUPS {
                    if hotbars.is_empty(row) {
                        shown.extend((0..hotbars::SIZE).map(|column| {
                            (column == row).then(|| Shown {
                                stack: stack_of(recipes, "minecraft:paper"),
                                locked: true,
                                name: Some(format_text(
                                    info.as_deref().unwrap_or("inventory.hotbarInfo"),
                                    &[SAVE_KEY, &(row + 1).to_string()],
                                )),
                            })
                        }));
                    } else {
                        shown.extend(hotbars.row(row).iter().map(|stack| {
                            stack.clone().map(|stack| Shown {
                                stack,
                                locked: false,
                                name: None,
                            })
                        }));
                    }
                }
                shown
            }
            McTabKind::Inventory => Vec::new(),
        };
        ui.creative.tags = tags;
        self.pending_items = Some(
            shown
                .iter()
                .map(|shown| shown.as_ref().map(|shown| shown.stack.clone()))
                .collect(),
        );
        self.items = shown;
        self.built = Some(built);
    }

    pub(crate) fn set_item_tags(
        &mut self,
        item_tags: &std::collections::BTreeMap<String, Vec<String>>,
    ) {
        if !self.tag_names.is_empty() || item_tags.is_empty() {
            return;
        }
        self.tag_names = item_tags.keys().cloned().collect();
        for (tag, items) in item_tags {
            for item in items {
                self.item_tags
                    .entry(item.clone())
                    .or_default()
                    .push(tag.clone());
            }
        }
        self.built = None;
    }

    fn ensure_search(
        &mut self,
        tabs: &CreativeTabs,
        recipes: &RecipeBook,
        language: Option<&HashMap<String, String>>,
    ) {
        let key = (self.operator, language.is_some());
        if self.search.as_ref().is_none_or(|search| search.key != key) {
            let entries = tabs.search(self.operator);
            let stacks = entry_stacks(
                &entries.iter().collect::<Vec<_>>(),
                recipes,
                self.variants.as_deref(),
            );
            let lines = stacks
                .iter()
                .map(|stack| search_lines(stack, language, self.variants.as_deref()))
                .collect();
            self.search = Some(SearchList { key, stacks, lines });
        }
    }

    fn hotbars(&mut self) -> &mut Hotbars {
        self.hotbars
            .get_or_insert_with(|| Hotbars::read(&hotbar_path()))
    }

    pub(crate) fn name_items(&self, items: &mut [Option<frame::McStack>]) {
        for (item, shown) in items.iter_mut().zip(&self.items) {
            if let (Some(item), Some(name)) = (
                item.as_mut(),
                shown.as_ref().and_then(|shown| shown.name.clone()),
            ) {
                item.name = Some(name);
            }
        }
    }

    pub(crate) fn hotbar_key(
        &mut self,
        ui: &mut MinecraftUi,
        mode: &PlayerMode,
        inventory: &mut Inventory,
        known: &dyn Fn(&str) -> bool,
    ) {
        let Some((index, load)) = ui.creative.hotbar_key.take() else {
            return;
        };
        if !self.creative_mode || index >= hotbars::GROUPS {
            return;
        }
        if load {
            let stacks = self.hotbars().row(index).to_vec();
            let (_, refused) =
                creative::apply(mode, inventory, creative::load_hotbar(&stacks), known);
            for why in refused {
                diag::warn!(World, "Minecraft saved hotbar slot refused: {why:?}");
            }
            return;
        }
        let stacks: Vec<Option<ItemStack>> = inventory.slots[..hotbars::SIZE]
            .iter()
            .map(|stack| {
                stack
                    .clone()
                    .filter(|stack| minecraft_survival::guns::weapon_of(stack).is_none())
            })
            .collect();
        self.hotbars().store(index, &stacks);
        if let Err(error) = self.hotbars().write(&hotbar_path()) {
            diag::warn!(World, "Failed to save creative mode options: {error}");
        }
        self.hotbars_version += 1;
        let saved = self
            .texts
            .as_ref()
            .map_or("inventory.hotbarSaved", |texts| texts.1.as_str());
        ui.creative.saved = Some(format_text(saved, &[LOAD_KEY, &(index + 1).to_string()]));
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
        let shown = self.items.get(index).and_then(Option::as_ref);
        if shown.is_some_and(|shown| shown.locked) {
            return Vec::new();
        }
        let recipes = inventory.recipes.clone();
        let max = |stack: &ItemStack| creative::max_stack(&recipes, stack);
        creative::grid_click(
            inventory.cursor.as_ref(),
            shown.map(|shown| &shown.stack),
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
        let stacks = entry_stacks(
            &entries.iter().collect::<Vec<_>>(),
            &RecipeBook::default(),
            None,
        );
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

    fn category(id: &str, title: &str, items: &[&str], operator: bool) -> CreativeTab {
        CreativeTab {
            id: id.into(),
            title: title.into(),
            row: Row::Top,
            column: 0,
            kind: TabKind::Category,
            aligned_right: false,
            show_title: true,
            scroll_bar: true,
            background: "items".into(),
            icon: None,
            operator,
            entries: items.iter().map(|item| entry(item, None)).collect(),
        }
    }

    #[test]
    fn items_know_the_shown_tabs_that_hold_them() {
        let tabs = CreativeTabs {
            tabs: vec![
                category(
                    "minecraft:building_blocks",
                    "itemGroup.buildingBlocks",
                    &["minecraft:stone", "minecraft:stone"],
                    false,
                ),
                category(
                    "minecraft:natural_blocks",
                    "itemGroup.natural",
                    &["minecraft:stone", "minecraft:dirt"],
                    false,
                ),
                category(
                    "minecraft:op_blocks",
                    "itemGroup.op",
                    &["minecraft:stone"],
                    true,
                ),
            ],
            ..CreativeTabs::default()
        };
        let found = tabs_of(&tabs, false, None);
        assert_eq!(
            found["minecraft:stone"],
            ["itemGroup.buildingBlocks", "itemGroup.natural"]
        );
        assert_eq!(found["minecraft:dirt"], ["itemGroup.natural"]);
        assert_eq!(tabs_of(&tabs, true, None)["minecraft:stone"].len(), 3);
    }

    #[test]
    fn the_search_list_filters_by_name_and_by_tag() {
        let recipes = RecipeBook::default();
        let stacks: Vec<ItemStack> = ["minecraft:oak_log", "minecraft:white_wool"]
            .iter()
            .map(|id| stack_of(&recipes, id))
            .collect();
        let mut language = HashMap::new();
        language.insert("block.minecraft.oak_log".to_owned(), "Oak Log".to_owned());
        language.insert(
            "block.minecraft.white_wool".to_owned(),
            "White Wool".to_owned(),
        );
        let lines = stacks
            .iter()
            .map(|stack| search_lines(stack, Some(&language), None))
            .collect();
        let search = SearchList {
            key: (false, true),
            stacks,
            lines,
        };
        let mut tags = HashMap::new();
        tags.insert(
            "minecraft:oak_log".to_owned(),
            vec!["minecraft:logs".to_owned()],
        );
        assert_eq!(search_items("wool", &search, &tags), [1]);
        assert_eq!(search_items("#logs", &search, &tags), [0]);
        assert_eq!(search_items("", &search, &tags), [0, 1]);
    }

    #[test]
    fn hotbar_texts_fill_their_keys_like_vanilla() {
        assert_eq!(
            format_text("Save hotbar with %1$s+%2$s", &[SAVE_KEY, "3"]),
            "Save hotbar with C+3"
        );
        assert_eq!(
            format_text("Saved hotbar (restore with %1$s+%2$s)", &[LOAD_KEY, "1"]),
            "Saved hotbar (restore with X+1)"
        );
        assert_eq!(format_text("%s and %s", &["a", "b"]), "a and b");
    }

    #[test]
    fn the_saved_hotbars_tab_shows_a_locked_paper_for_each_empty_row() {
        let mut creative = CreativeUi {
            tabs: Some(Arc::new(CreativeTabs {
                tabs: vec![CreativeTab {
                    kind: TabKind::Hotbar,
                    ..category("minecraft:hotbar", "itemGroup.hotbar", &[], false)
                }],
                ..CreativeTabs::default()
            })),
            hotbars: Some(Hotbars::default()),
            ..CreativeUi::default()
        };
        let mut stacks = vec![None; 9];
        stacks[4] = Some(ItemStack::new("minecraft:torch", 16));
        creative.hotbars().store(1, &stacks);
        let mut ui = MinecraftUi::default();
        let recipes = RecipeBook::default();
        creative.refresh(&mut ui, &recipes, None);
        ui.creative.tabs = creative
            .take_tabs()
            .unwrap()
            .into_iter()
            .map(|(tab, _)| tab)
            .collect();
        creative.refresh(&mut ui, &recipes, None);
        let items = creative.take_items().unwrap();
        assert_eq!(items.len(), 81);
        assert_eq!(
            items[0].as_ref().map(|stack| stack.id.as_str()),
            Some("minecraft:paper")
        );
        assert!(items[1..9].iter().all(Option::is_none));
        assert_eq!(items[9 + 4], Some(ItemStack::new("minecraft:torch", 16)));
        assert_eq!(items[9 + 1], None);
        assert_eq!(
            items[18 + 2].as_ref().map(|stack| stack.id.as_str()),
            Some("minecraft:paper")
        );
        assert!(creative.items[0].as_ref().is_some_and(|shown| shown.locked));
        assert_eq!(
            creative.items[20]
                .as_ref()
                .and_then(|shown| shown.name.clone())
                .as_deref(),
            Some("inventory.hotbarInfo")
        );
        let inventory = Inventory::default();
        assert!(
            creative
                .grid(
                    &inventory,
                    0,
                    GridClick::Pick {
                        right: false,
                        shift: false
                    }
                )
                .is_empty()
        );
        assert_eq!(
            creative
                .grid(
                    &inventory,
                    13,
                    GridClick::Pick {
                        right: false,
                        shift: true
                    }
                )
                .len(),
            1
        );
    }

    #[test]
    fn expanded_variants_replace_their_entry_and_unexpanded_ones_stay_plain() {
        use minecraft_terrain::creative_tabs::Variants;
        let data = VariantData {
            potions: vec![
                "minecraft:water".to_owned(),
                "minecraft:swiftness".to_owned(),
            ],
            enchantments: vec![
                ("minecraft:mending".to_owned(), 1),
                ("minecraft:sharpness".to_owned(), 5),
            ],
            ..VariantData::default()
        };
        let entries = [
            entry("minecraft:potion", Some(Variants::Potions)),
            entry(
                "minecraft:enchanted_book",
                Some(Variants::EnchantedBooks { all_levels: false }),
            ),
            entry("minecraft:suspicious_stew", Some(Variants::SuspiciousStews)),
            entry("minecraft:white_banner", None),
            entry("minecraft:white_banner", Some(Variants::OminousBanner)),
        ];
        let stacks = entry_stacks(
            &entries.iter().collect::<Vec<_>>(),
            &RecipeBook::default(),
            Some(&data),
        );
        let shown: Vec<(&str, Option<String>)> = stacks
            .iter()
            .map(|stack| {
                (
                    stack.id.as_str(),
                    stack.components.as_ref().map(ToString::to_string),
                )
            })
            .collect();
        assert_eq!(
            shown,
            [
                (
                    "minecraft:potion",
                    Some(
                        r#"{"minecraft:potion_contents":{"potion":"minecraft:water"}}"#.to_owned()
                    )
                ),
                (
                    "minecraft:potion",
                    Some(
                        r#"{"minecraft:potion_contents":{"potion":"minecraft:swiftness"}}"#
                            .to_owned()
                    )
                ),
                (
                    "minecraft:enchanted_book",
                    Some(r#"{"minecraft:stored_enchantments":{"minecraft:mending":1}}"#.to_owned())
                ),
                (
                    "minecraft:enchanted_book",
                    Some(
                        r#"{"minecraft:stored_enchantments":{"minecraft:sharpness":5}}"#.to_owned()
                    )
                ),
                ("minecraft:suspicious_stew", None),
                ("minecraft:white_banner", None),
            ]
        );
        let mut language = HashMap::new();
        language.insert(
            "item.minecraft.potion.effect.swiftness".to_owned(),
            "Potion of Swiftness".to_owned(),
        );
        language.insert(
            "item.minecraft.enchanted_book".to_owned(),
            "Enchanted Book".to_owned(),
        );
        language.insert(
            "enchantment.minecraft.sharpness".to_owned(),
            "Sharpness".to_owned(),
        );
        language.insert(
            "enchantment.minecraft.mending".to_owned(),
            "Mending".to_owned(),
        );
        assert_eq!(
            search_lines(&stacks[1], Some(&language), Some(&data)),
            ["Potion of Swiftness"]
        );
        assert_eq!(
            search_lines(&stacks[3], Some(&language), Some(&data)),
            ["Enchanted Book", "Sharpness V"]
        );
        assert_eq!(
            search_lines(&stacks[2], Some(&language), Some(&data)),
            ["Enchanted Book", "Mending"]
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
