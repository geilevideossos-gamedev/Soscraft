//! The player's inventory on the Minecraft map: MinecraftOSS's vanilla
//! `Inventory` (slots, stacking, container clicks, the 2x2 crafting grid),
//! published for the HUD to draw with item icons and names made the way the
//! viewer makes them. The player's MW2 guns are items in it too: selecting
//! one on the hotbar raises that gun.
use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use frame::{McChoice, McClick, McOven, McScreen, McSlot, McStack, MinecraftUi};
use glam::DVec3;
use minecraft_net::messages::{MenuAction, MenuSlot};
use minecraft_survival::guns::{WEAPON_PREFIX, weapon_of};
use minecraft_survival::menus::{Books, Clicked, Menu, Open, Screen};
use minecraft_survival::player::Player;
use minecraft_terrain::containers::{Containers, Opened, Station};
use minecraft_terrain::stations::{Bench, Context, StationBook, Workshop};
use minecraft_terrain::crafting::{self, Grid};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::BlockPos;
use minecraftoss_player::crafting::CookingKind;
use minecraftoss_player::inventory::{Inventory, ItemStack};

const ICON_COLUMNS: u32 = 16;
const ICON_ATLAS_LIMIT: u32 = 8192;
const OFFHAND: usize = 40;

#[derive(Default)]
pub(crate) struct InventoryUi {
    atlas: Option<image::RgbaImage>,
    handle: Option<Handle<Image>>,
    cells: HashMap<String, u32>,
    failed: HashSet<String>,
    language: Option<HashMap<String, String>>,
    dirty: bool,
    follow: minecraft_survival::guns::Follow,
    book: std::sync::Arc<StationBook>,
    enchantments: Option<std::sync::Arc<minecraftoss_entities::enchant::Enchantments>>,
    gui: Option<crate::minecraft_gui::GuiAssets>,
    gui_failed: Option<u64>,
    icon_scale: u32,
    icon_rows: u32,
    pub(crate) animation_tick: u64,
    animated: AnimatedIcons,
    pub(crate) creative: crate::minecraft_creative::CreativeUi,
    pub(crate) dials: minecraft_terrain::dials::Dials,
    book_source: Option<BookSource>,
    lectern_actions: Vec<(BlockPos, minecraft_terrain::lectern::LecternAction)>,
    book_edits: Vec<(usize, Vec<String>, Option<String>)>,
    pub(crate) fork: crate::minecraft_fork_items::ForkLook,
    recipe_book: crate::minecraft_recipe_book::RecipeBookUi,
    unlocked: Option<HashSet<String>>,
    seen: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BookSource {
    Lectern(BlockPos),
    Held(usize),
}

#[derive(Default)]
struct AnimatedIcons {
    sheet: Option<image::RgbaImage>,
    handle: Option<Handle<Image>>,
    cells: HashMap<String, u32>,
    tick: u64,
    dirty: bool,
    moved: bool,
}

impl AnimatedIcons {
    fn clear(&mut self) {
        self.sheet = None;
        self.cells.clear();
        self.moved = true;
    }

    fn add(&mut self, id: String, rendered: &image::RgbaImage, icon: u32) -> bool {
        let cell = self.cells.len() as u32;
        let limit = ICON_ATLAS_LIMIT / icon;
        if cell >= limit {
            return false;
        }
        let capacity = self.sheet.as_ref().map_or(0, |sheet| sheet.width() / icon);
        if cell >= capacity {
            let mut next = image::RgbaImage::new((cell + 1).next_power_of_two().min(limit) * icon, icon);
            if let Some(old) = self.sheet.as_ref() {
                image::imageops::replace(&mut next, old, 0, 0);
            }
            self.sheet = Some(next);
            self.moved = true;
            self.dirty = true;
        }
        self.cells.insert(id, cell);
        self.draw(cell, rendered, icon);
        true
    }

    fn draw(&mut self, cell: u32, rendered: &image::RgbaImage, icon: u32) {
        let Some(sheet) = self.sheet.as_mut() else { return };
        let x = cell * icon;
        if image::imageops::crop_imm(sheet, x, 0, icon, icon).to_image() != *rendered {
            image::imageops::replace(sheet, rendered, i64::from(x), 0);
            self.dirty = true;
        }
    }

    fn publish(&mut self, ui: &mut MinecraftUi, images: &mut Assets<Image>, icon: u32) {
        if self.moved {
            self.moved = false;
            ui.animated_icon_rects.clear();
            let width = self.sheet.as_ref().map_or(1, image::RgbaImage::width) as f32;
            for (id, &cell) in &self.cells {
                let x = (cell * icon) as f32;
                ui.animated_icon_rects.insert(id.clone(), [x / width, 0.0, (x + icon as f32) / width, 1.0]);
            }
        }
        if !self.dirty {
            return;
        }
        let Some(sheet) = self.sheet.as_ref() else { return };
        self.dirty = false;
        let image = Image {
            sampler: ImageSampler::nearest(),
            ..Image::new(
                Extent3d { width: sheet.width(), height: sheet.height(), depth_or_array_layers: 1 },
                TextureDimension::D2,
                sheet.as_raw().clone(),
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::default(),
            )
        };
        match self.handle.as_ref() {
            Some(handle) => {
                let _ = images.insert(handle.id(), image);
            }
            None => self.handle = Some(images.add(image)),
        }
        ui.animated_icons = self.handle.clone();
    }
}

/// What the player holds and where it looks, for throwing items.
pub(crate) struct Thrower {
    pub eye: DVec3,
    /// Minecraft yaw and pitch, in degrees.
    pub yaw: f32,
    pub pitch: f32,
}

impl InventoryUi {
    /// Keeps the player's MW2 guns in the inventory: each gun the player has
    /// is one item, placed on the first free hotbar slot when it arrives, and
    /// a gun the player no longer has leaves.
    pub(crate) fn sync_weapons(&mut self, inventory: &mut Inventory, owned: &[u32]) {
        minecraft_survival::guns::sync_weapons(inventory, owned);
    }

    pub(crate) fn show_menu(&mut self, ui: &mut MinecraftUi, menu: &Menu) {
        ui.screen = screen_for(Some(&menu.open));
        if !matches!(menu.open, Open::Table(_)) {
            ui.screen_title = self.resolve(&menu.title);
        }
        ui.inventory_open = true;
        ui.screen_opened = true;
    }

    pub(crate) fn resolve(&self, text: &minecraft_commands::Text) -> String {
        let lang = |key: &str| self.language.as_ref().and_then(|language| language.get(key).cloned());
        text.resolve(&lang).text
    }

    pub(crate) fn lectern(&self) -> Option<BlockPos> {
        match self.book_source {
            Some(BookSource::Lectern(pos)) => Some(pos),
            _ => None,
        }
    }

    pub(crate) fn show_held_book(&mut self, ui: &mut MinecraftUi, slot: usize, book: frame::McBook) -> bool {
        if ui.inventory_open {
            return false;
        }
        self.book_source = Some(BookSource::Held(slot));
        ui.screen = McScreen::Lectern;
        ui.inventory_open = true;
        ui.screen_opened = true;
        ui.book = Some(book);
        true
    }

    pub(crate) fn show_book_editor(&mut self, ui: &mut MinecraftUi, slot: usize, pages: Vec<String>, author: &str) -> bool {
        if ui.inventory_open {
            return false;
        }
        let mut pages: Vec<String> = pages.into_iter().map(|page| page.chars().take(minecraft_survival::books::PAGE_LENGTH).collect()).collect();
        if pages.is_empty() {
            pages.push(String::new());
        }
        let cursor = pages[0].len();
        self.book_source = Some(BookSource::Held(slot));
        ui.screen = McScreen::BookEdit;
        ui.inventory_open = true;
        ui.screen_opened = true;
        ui.book = None;
        ui.book_edit = Some(frame::McBookEdit { pages, cursor, highlight: cursor, author: author.to_owned(), ..Default::default() });
        true
    }

    fn book_screen(ui: &MinecraftUi, shown: McScreen) -> McScreen {
        match (ui.book_edit.is_some(), shown) {
            (true, McScreen::BookEdit | McScreen::BookSign) => shown,
            (true, _) => McScreen::BookEdit,
            (false, _) => McScreen::Lectern,
        }
    }

    fn book_button(&mut self, ui: &mut MinecraftUi, source: BookSource, button: usize) {
        match source {
            BookSource::Lectern(pos) => self.lectern_actions.push((pos, minecraft_terrain::lectern::LecternAction::Button(button as i32))),
            BookSource::Held(_) => {
                if let Some(book) = ui.book.as_mut() {
                    let last = book.pages.len().saturating_sub(1);
                    book.page = match button {
                        1 => book.page.saturating_sub(1),
                        2 => (book.page + 1).min(last),
                        _ => book.page,
                    };
                }
            }
        }
    }

    fn close_book(&mut self, ui: &mut MinecraftUi) {
        if let Some(source) = self.book_source.take() {
            ui.book = None;
            ui.book_edit = None;
            if let BookSource::Lectern(pos) = source {
                self.lectern_actions.push((pos, minecraft_terrain::lectern::LecternAction::Close));
            }
        }
    }

    pub(crate) fn show_lectern(&mut self, ui: &mut MinecraftUi, pos: BlockPos, book: frame::McBook) -> bool {
        if self.lectern() != Some(pos) {
            if ui.inventory_open {
                return false;
            }
            self.book_source = Some(BookSource::Lectern(pos));
            ui.screen = McScreen::Lectern;
            ui.inventory_open = true;
            ui.screen_opened = true;
        }
        ui.book = Some(book);
        true
    }

    pub(crate) fn lectern_closed(&mut self, ui: &mut MinecraftUi, pos: BlockPos) {
        if self.lectern() == Some(pos) {
            self.book_source = None;
            ui.book = None;
            ui.inventory_open = false;
        }
    }

    pub(crate) fn take_lectern_actions(&mut self) -> Vec<(BlockPos, minecraft_terrain::lectern::LecternAction)> {
        std::mem::take(&mut self.lectern_actions)
    }

    pub(crate) fn take_book_edits(&mut self) -> Vec<(usize, Vec<String>, Option<String>)> {
        std::mem::take(&mut self.book_edits)
    }

    pub(crate) fn book_clicks(&mut self, ui: &mut MinecraftUi) -> bool {
        let Some(source) = self.book_source else {
            return false;
        };
        for click in std::mem::take(&mut ui.clicks) {
            match click {
                McClick::Choose(button) => self.book_button(ui, source, button),
                McClick::Close => ui.inventory_open = false,
                McClick::EditBook { pages, title } => {
                    if let BookSource::Held(slot) = source {
                        self.book_edits.push((slot, pages, title));
                        ui.inventory_open = false;
                    }
                }
                _ => {}
            }
        }
        if !ui.inventory_open {
            self.close_book(ui);
        } else {
            ui.screen = Self::book_screen(ui, ui.screen);
        }
        true
    }

    pub(crate) fn language(&self) -> Option<&HashMap<String, String>> {
        self.language.as_ref()
    }

    pub(crate) fn language_and_fork(
        &mut self,
    ) -> (
        Option<&HashMap<String, String>>,
        &mut crate::minecraft_fork_items::ForkLook,
    ) {
        (self.language.as_ref(), &mut self.fork)
    }

    pub(crate) fn set_station_data(
        &mut self,
        book: std::sync::Arc<StationBook>,
        enchantments: Option<std::sync::Arc<minecraftoss_entities::enchant::Enchantments>>,
    ) {
        self.book = book;
        self.enchantments = enchantments;
    }

    pub(crate) fn apply_input(
        &mut self,
        ui: &mut MinecraftUi,
        player: &mut Player,
        containers: &mut Containers,
        books: Books<'_>,
        roll: f32,
    ) -> Clicked {
        let mut clicked = Clicked::default();
        let shown = ui.screen;
        for click in std::mem::take(&mut ui.clicks) {
            if let Some(source) = self.book_source {
                match &click {
                    McClick::Choose(button) => {
                        self.book_button(ui, source, *button);
                        continue;
                    }
                    McClick::EditBook { pages, title } => {
                        if let BookSource::Held(slot) = source {
                            let Player { inventory, name, .. } = &mut *player;
                            minecraft_survival::books::edit_book(inventory, slot, pages, title.as_deref(), name);
                            ui.inventory_open = false;
                            self.close_book(ui);
                        }
                        continue;
                    }
                    McClick::Close => self.close_book(ui),
                    _ => continue,
                }
            }
            match click {
                McClick::Close => clicked.thrown.extend(minecraft_survival::menus::close(player)),
                McClick::Rename(name) => minecraft_survival::menus::rename(player, &name, self.language.as_ref()),
                other => {
                    if let Some(action) = action_of(&other) {
                        clicked.add(minecraft_survival::menus::apply(player, containers, books, &action, roll));
                    }
                }
            }
        }
        if !ui.inventory_open {
            clicked.thrown.extend(minecraft_survival::menus::close(player));
            self.close_book(ui);
        }
        ui.screen = screen_for(player.menus.open.as_ref().map(|menu| &menu.open));
        if self.book_source.is_some() {
            ui.screen = Self::book_screen(ui, shown);
        }
        let Player { inventory, selected, .. } = player;
        clicked.thrown.extend(inventory.take_pending_drops());
        let _ = inventory.take_stat_events();
        if let Some(slot) = ui.select.take() {
            *selected = slot.min(frame::minecraft_ui::MC_HOTBAR - 1);
        }
        if let Some(whole) = ui.drop_selected.take()
            && let Some(stack) = inventory.drop_selected(*selected, whole)
        {
            clicked.thrown.push(stack);
        }
        clicked.thrown = minecraft_survival::menus::keep_guns(inventory, *selected, std::mem::take(&mut clicked.thrown));
        clicked
    }

    /// The gun the hotbar selection asks for, if the player holds another;
    /// and the selection follows a gun MW2 raised on its own.
    pub(crate) fn weapon_request(
        &mut self,
        inventory: &Inventory,
        guns: &[Option<u32>; minecraft_survival::guns::HOTBAR],
        selected: &mut usize,
        held: u32,
    ) -> Option<u32> {
        self.follow.request_with(inventory, guns, selected, held)
    }

    pub(crate) fn set_unlocked(&mut self, unlocked: HashSet<String>) {
        self.unlocked = Some(unlocked);
    }

    pub(crate) fn take_seen(&mut self) -> Vec<String> {
        std::mem::take(&mut self.seen)
    }

    pub(crate) fn set_weapons(
        &mut self,
        ui: &mut MinecraftUi,
        weapons: Option<&assets::PreparedWeapons>,
        strings: Option<&assets::PreparedLocalizedStrings>,
    ) {
        self.fork.set_weapons(weapons.map(|weapons| &weapons.0));
        if let Some(language) = self.language.as_mut()
            && self.fork.name_items(language, strings.map(|strings| &strings.0))
        {
            ui.names.retain(|id, _| !minecraft_terrain::fork_items::is_fork(id));
        }
    }

    /// Publishes the inventory for the HUD, with icons for every item in it.
    pub(crate) fn publish(
        &mut self,
        ui: &mut MinecraftUi,
        inventory: &Inventory,
        selected: usize,
        packs: &PackStack,
        images: &mut Assets<Image>,
        (screen, level, containers): (&Screen, u32, &Containers),
    ) {
        if self.language.is_none() {
            self.language = Some(minecraft_terrain::item_icons::language(packs).unwrap_or_else(|_| HashMap::new()));
        }
        let generation = packs.generation;
        if self.gui.as_ref().is_none_or(|gui| gui.generation != generation) && self.gui_failed != Some(generation) {
            let empty = HashMap::new();
            let language = self.language.as_ref().unwrap_or(&empty);
            self.gui = crate::minecraft_gui::GuiAssets::load(packs, language, crate::minecraft_world::data_jar().as_deref(), images);
            if self.gui.is_none() {
                self.gui_failed = Some(generation);
            }
        }
        ui.gui = self.gui.as_ref().map(|gui| gui.gui.clone());
        ui.glint = self.gui.as_ref().and_then(|gui| gui.glint.clone());
        self.creative.refresh(ui, &inventory.recipes, self.language.as_ref());
        let language = self
            .language
            .get_or_insert_with(|| minecraft_terrain::item_icons::language(packs).unwrap_or_default());
        let mut stacks = Vec::new();
        let water = minecraft_terrain::bottle::is_water_bottle;
        let water_name = language
            .get(minecraft_terrain::bottle::WATER_NAME_KEY)
            .cloned()
            .unwrap_or_else(|| "Water Bottle".to_owned());
        let lang: &HashMap<String, String> = language;
        let enchant_table = self.enchantments.clone();
        let specials = (self.fork.special(inventory, Grid::Inventory), self.fork.special(inventory, Grid::Table));
        let gun_rules = self.fork.rules();
        let fork = &mut self.fork;
        let mut convert = |stack: &Option<ItemStack>| {
            stack.as_ref().map(|s| {
                let icon = minecraft_terrain::item_tint::of_stack(s);
                stacks.push(icon.clone());
                let weapon = weapon_of(s).or_else(|| fork.weapon(s));
                let mut shown = McStack {
                    id: s.id.clone(),
                    count: s.count,
                    weapon,
                    damage: inventory.recipes.durability(s),
                    max: stack_limit(&inventory.recipes, s),
                    named: minecraft_terrain::stations::custom_name(s).is_some(),
                    name: display_name(lang, s).or_else(|| water(s).then(|| water_name.clone())),
                    attack: main_hand_attack(&inventory.recipes, s),
                    enchantments: minecraft_terrain::stations::enchantments_of(s)
                        .into_iter()
                        .map(|(id, level)| {
                            let key = format!("enchantment.{}", id.replacen(':', ".", 1));
                            let single = enchant_table.as_ref().and_then(|table| table.index(&id).map(|i| table.list[i].max_level == 1)).unwrap_or(false);
                            let name = lang.get(&key).cloned().unwrap_or(id);
                            if level == 1 && single { name } else { format!("{name} {}", minecraft_terrain::stations::roman(level)) }
                        })
                        .collect(),
                    contents: contents_lines(lang, s),
                    more: more_line(lang, s),
                    glint: minecraft_terrain::glint::has_glint(s),
                    icon,
                    lore: book_lines(lang, s),
                    clip: fork.clip_bar(s, weapon),
                };
                shown.enchantments.splice(0..0, fork.lines(s, weapon, lang));
                shown
            })
        };
        ui.slots = inventory.slots.iter().take(frame::minecraft_ui::MC_INVENTORY_SLOTS).map(&mut convert).collect();
        let table = matches!(screen.open, Some(Open::Table(_)));
        let table_open = ui.inventory_open && table;
        let small_open = ui.inventory_open && !table;
        ui.crafting = std::array::from_fn(|i| convert(&inventory.crafting[i]));
        ui.result = if small_open { convert(&specials.0.or_else(|| crafting::output(inventory, Grid::Inventory))) } else { None };
        ui.workbench = std::array::from_fn(|i| convert(&inventory.workbench[i]));
        ui.workbench_result = if table_open { convert(&specials.1.or_else(|| crafting::output(inventory, Grid::Table))) } else { None };
        let opened = match &screen.open {
            Some(Open::Block(opened)) if ui.inventory_open => Some(opened.clone()),
            _ => None,
        };
        let (mut choices, mut chosen, mut cost, mut name) = (Vec::new(), None, None, None);
        let mut shown_merchant = None;
        let bench = match &screen.open {
            Some(Open::Bench(_, bench)) if ui.inventory_open => Some(bench),
            _ => None,
        };
        let ctx = Context {
            recipes: &inventory.recipes,
            book: &self.book,
            enchantments: self.enchantments.as_deref(),
            bookshelves: screen.shelves,
            seed: screen.seed,
            level,
            guns: gun_rules.as_ref().map(|rules| rules as &dyn minecraft_terrain::gun_mods::GunRules),
        };
        let (container, output, data) = match (opened.as_ref(), bench) {
            (None, Some(bench)) => {
                let offer = bench.offer(&ctx);
                let costs: Vec<u32> = offer.choices.iter().map(|choice| choice.cost).collect();
                let runes = match (bench.kind, self.gui.as_ref()) {
                    (Workshop::Enchanting, Some(gui)) => crate::minecraft_gui::enchanting_runes(&gui.words, &gui.gui, screen.seed, &costs),
                    _ => Vec::new(),
                };
                choices = offer
                    .choices
                    .iter()
                    .enumerate()
                    .map(|(i, choice)| McChoice {
                        icon: choice.item.as_ref().and_then(|item| convert(&Some(item.clone()))),
                        label: choice.clue.as_ref().map_or_else(String::new, |(id, level)| {
                            let key = format!("enchantment.{}", id.replacen(':', ".", 1));
                            let name = lang.get(&key).cloned().unwrap_or_else(|| id.clone());
                            let single = enchant_table.as_ref().and_then(|table| table.index(id).map(|i| table.list[i].max_level == 1)).unwrap_or(false);
                            if *level == 1 && single { name } else { format!("{name} {}", minecraft_terrain::stations::roman(*level)) }
                        }),
                        cost: choice.cost,
                        enabled: choice.enabled,
                        runes: runes.get(i).map_or_else(String::new, Clone::clone),
                    })
                    .collect();
                chosen = bench.chosen;
                cost = (offer.cost > 0).then_some(offer.cost);
                name = bench.name.clone();
                (bench.cells.iter().map(&mut convert).collect(), convert(&offer.result), [0; 4])
            }
            (Some(opened), _) => {
                let cells = &screen.cells;
                match opened.station {
                    Station::Brewer => (cells.iter().map(&mut convert).collect(), None, screen.data),
                    Station::Oven(_) if cells.len() >= 3 => (cells[..2].iter().map(&mut convert).collect(), convert(&cells[2]), screen.data),
                    _ => (cells.iter().map(&mut convert).collect(), None, [0; 4]),
                }
            }
            (None, None) => match &screen.open {
                Some(Open::Merchant(merchant)) if ui.inventory_open => {
                    let view = &merchant.view;
                    let offers = view
                        .offers
                        .iter()
                        .map(|offer| {
                            let [cost_a, cost_b, result] = offer_stacks(&inventory.recipes, &view.max_stacks, offer);
                            frame::McOffer {
                                cost_a: convert(&cost_a),
                                base_count: u8::try_from(offer.buy.count.max(1)).unwrap_or(u8::MAX),
                                cost_b: convert(&cost_b),
                                result: convert(&result),
                                out_of_stock: offer.out_of_stock(),
                            }
                        })
                        .collect();
                    shown_merchant = Some(frame::McMerchant {
                        offers,
                        level: view.level,
                        xp: view.xp,
                        future_xp: view.future_xp,
                        show_progress: !view.trader,
                        can_restock: !view.trader,
                    });
                    chosen = merchant.selected;
                    (merchant.view.payment.iter().map(&mut convert).collect(), convert(&merchant.view.result), [0; 4])
                }
                _ => (Vec::new(), None, [0; 4]),
            },
        };
        ui.choices = choices;
        ui.chosen = chosen;
        ui.cost = cost;
        ui.name = name;
        ui.container = container;
        ui.output = output;
        ui.data = data;
        ui.merchant = shown_merchant;
        let book_grid = crate::minecraft_recipe_book::grid_of(ui.screen).filter(|_| ui.inventory_open);
        let unlocked = self.unlocked.as_ref().unwrap_or(&inventory.unlocked_recipes);
        if let Some(built) = self.recipe_book.build(&ui.recipe_book, inventory, unlocked, book_grid, lang) {
            self.seen = built.buttons.iter().flatten().map(|choice| choice.id.clone()).collect();
            let book = &mut ui.recipe_book;
            book.tabs = built.tabs;
            book.pages = built.pages;
            book.buttons = built
                .buttons
                .into_iter()
                .map(|choices| frame::McRecipeButton {
                    recipes: choices
                        .into_iter()
                        .filter_map(|choice| {
                            Some(frame::McRecipeOption {
                                result: convert(&Some(choice.result))?,
                                id: choice.id,
                                craftable: choice.craftable,
                                cells: choice.cells.iter().map(|cell| cell.as_ref().map(|stacks| stacks.iter().filter_map(|stack| convert(&Some(stack.clone()))).collect())).collect(),
                            })
                        })
                        .collect(),
                })
                .collect();
            book.ghost_cells = built.ghost.into_iter().map(|(slot, stacks)| (slot, stacks.into_iter().filter_map(|stack| convert(&Some(stack))).collect())).collect();
            book.tab_icons = built.tab_icons.into_iter().map(|icons| icons.into_iter().filter_map(|stack| convert(&Some(stack))).collect()).collect();
        }
        ui.cursor = convert(&inventory.cursor);
        let mut creative_converted = 0;
        if let Some(items) = self.creative.take_items() {
            creative_converted += items.iter().flatten().count();
            ui.creative.items = items.iter().map(&mut convert).collect();
            self.creative.name_items(&mut ui.creative.items);
        }
        if let Some(tabs) = self.creative.take_tabs() {
            creative_converted += tabs.iter().filter(|(_, icon)| icon.is_some()).count();
            ui.creative.tabs = tabs.into_iter().map(|(tab, icon)| frame::McCreativeTab { icon: convert(&icon), ..tab }).collect();
        }
        stacks.truncate(stacks.len() - creative_converted);
        stacks.extend(self.creative.icon_ids(ui));
        stacks.extend(ui.recipe_book.icon_ids().map(str::to_owned));
        self.dials.watch(stacks.iter().map(String::as_str));
        ui.armor = inventory.armor_value();
        ui.droppable = inventory.cursor.as_ref().filter(|_| ui.inventory_open).map_or_else(Vec::new, |carried| {
            droppable(ui.screen, carried, inventory, opened.as_ref(), bench, containers, &ctx)
        });
        ui.selected = selected;
        let scale = ui.gui_scale.clamp(1, 16);
        if scale != self.icon_scale {
            self.icon_scale = scale;
            self.atlas = None;
            self.icon_rows = 0;
            self.cells.clear();
            self.failed.clear();
            self.animated.clear();
            ui.icon_rects.clear();
        }
        let icon = 16 * scale;
        let max_rows = ICON_ATLAS_LIMIT / icon;
        let mut grown = false;
        let animation_tick = self.animation_tick;
        if self.animated.tick != animation_tick {
            self.animated.tick = animation_tick;
            let shown: Vec<String> = if ui.inventory_open {
                stacks.clone()
            } else {
                let hotbar = inventory.slots.iter().take(frame::minecraft_ui::MC_HOTBAR);
                hotbar.chain(inventory.slots.get(OFFHAND)).chain(std::iter::once(&inventory.cursor)).flatten().map(minecraft_terrain::item_tint::of_stack).collect()
            };
            let mut refreshed = HashSet::new();
            for id in &shown {
                let Some(&cell) = self.animated.cells.get(id) else { continue };
                if !refreshed.insert(id) {
                    continue;
                }
                let clock = minecraft_terrain::item_icon::SpriteClock::at(animation_tick);
                let Ok(Some(rendered)) = minecraft_terrain::item_icons::item_icon_at(packs, &icon_source(&self.dials, id).1, icon as usize, &clock) else { continue };
                let rendered = image::imageops::resize(&rendered, icon, icon, image::imageops::FilterType::Nearest);
                self.animated.draw(cell, &rendered, icon);
            }
        }
        for id in stacks {
            if id.starts_with(WEAPON_PREFIX) {
                continue;
            }
            let (base, source) = icon_source(&self.dials, &id);
            if !ui.names.contains_key(base) {
                ui.names.insert(base.to_owned(), minecraft_terrain::item_icons::item_name(language, base));
            }
            if minecraft_terrain::fork_items::get(&id).is_some_and(|row| row.icon == minecraft_terrain::fork_items::Icon::KillIcon) {
                continue;
            }
            if self.cells.contains_key(&id) || self.failed.contains(&id) || self.animated.cells.contains_key(&id) {
                continue;
            }
            if self.fork.waiting(&id) {
                continue;
            }
            let cell = self.cells.len() as u32;
            if cell >= ICON_COLUMNS * max_rows {
                diag::warn!(World, "Minecraft item icon atlas full: no icon for `{id}`");
                self.failed.insert(id);
                continue;
            }
            let clock = minecraft_terrain::item_icon::SpriteClock::at(animation_tick);
            let made = self.fork.icon(packs, &id, icon as usize, &clock);
            match made.unwrap_or_else(|| minecraft_terrain::item_icons::item_icon_at(packs, &source, icon as usize, &clock)) {
                Ok(Some(rendered)) => {
                    if clock.animated() && self.animated.add(id.clone(), &image::imageops::resize(&rendered, icon, icon, image::imageops::FilterType::Nearest), icon) {
                        continue;
                    }
                    let row = cell / ICON_COLUMNS;
                    if row >= self.icon_rows {
                        let rows = (self.icon_rows * 2).max(row + 1).min(max_rows);
                        let mut next = image::RgbaImage::new(ICON_COLUMNS * icon, rows * icon);
                        if let Some(old) = self.atlas.as_ref() {
                            image::imageops::replace(&mut next, old, 0, 0);
                        }
                        self.atlas = Some(next);
                        self.icon_rows = rows;
                    }
                    let Some(atlas) = self.atlas.as_mut() else { continue };
                    let (x, y) = ((cell % ICON_COLUMNS) * icon, row * icon);
                    let rendered = image::imageops::resize(&rendered, icon, icon, image::imageops::FilterType::Nearest);
                    image::imageops::replace(atlas, &rendered, i64::from(x), i64::from(y));
                    self.cells.insert(id, cell);
                    grown = true;
                    self.dirty = true;
                }
                Ok(None) => {
                    diag::warn!(World, "Minecraft item icon: no model for `{id}`");
                    self.failed.insert(id);
                }
                Err(error) => {
                    diag::warn!(World, "Minecraft item icon for `{id}` failed: {error}");
                    self.failed.insert(id);
                }
            }
        }
        if grown {
            let (w, h) = ((ICON_COLUMNS * icon) as f32, (self.icon_rows * icon).max(1) as f32);
            for (id, &cell) in &self.cells {
                let (x, y) = (((cell % ICON_COLUMNS) * icon) as f32, ((cell / ICON_COLUMNS) * icon) as f32);
                ui.icon_rects.insert(id.clone(), [x / w, y / h, (x + icon as f32) / w, (y + icon as f32) / h]);
            }
        }
        if self.dirty
            && let Some(atlas) = self.atlas.as_ref()
        {
            self.dirty = false;
            let image = Image {
                sampler: ImageSampler::nearest(),
                ..Image::new(
                    Extent3d { width: atlas.width(), height: atlas.height(), depth_or_array_layers: 1 },
                    TextureDimension::D2,
                    atlas.as_raw().clone(),
                    TextureFormat::Rgba8Unorm,
                    RenderAssetUsages::default(),
                )
            };
            match self.handle.as_ref() {
                Some(handle) => {
                    let _ = images.insert(handle.id(), image);
                }
                None => self.handle = Some(images.add(image)),
            }
            ui.icons = self.handle.clone();
        }
        self.animated.publish(ui, images, icon);
    }
}

fn icon_source<'a>(dials: &minecraft_terrain::dials::Dials, icon: &'a str) -> (&'a str, String) {
    let (base, potion) = minecraft_terrain::item_tint::split(icon);
    (minecraft_terrain::dials::item_of(base), minecraft_terrain::item_tint::key(dials.key(base), potion))
}

fn stack_limit(recipes: &minecraftoss_player::crafting::RecipeBook, stack: &ItemStack) -> u8 {
    if stack.components.is_none() {
        stack.max.min(recipes.max_stack(&stack.id))
    } else {
        stack.max
    }
}

pub(crate) fn screen_for(open: Option<&Open>) -> McScreen {
    match open {
        Some(Open::Table(_)) => McScreen::CraftingTable,
        Some(Open::Block(opened)) => screen_of(opened),
        Some(Open::Bench(_, bench)) => bench_screen(bench.kind),
        Some(Open::Merchant(_)) => McScreen::Merchant,
        Some(Open::Mount(mount)) => McScreen::Mount { columns: u8::try_from(mount.columns).unwrap_or(u8::MAX), saddle: mount.saddle, armor: mount.armor, llama: mount.llama },
        Some(Open::Inventory) | None => McScreen::Inventory,
    }
}

fn menu_slot(slot: McSlot) -> Option<MenuSlot> {
    Some(match slot {
        McSlot::Inventory(index) => MenuSlot::Inventory(u8::try_from(index).ok()?),
        McSlot::Crafting(index) => MenuSlot::Crafting(u8::try_from(index).ok()?),
        McSlot::Result => MenuSlot::Result,
        McSlot::Workbench(index) => MenuSlot::Workbench(u8::try_from(index).ok()?),
        McSlot::WorkbenchResult => MenuSlot::WorkbenchResult,
        McSlot::Container(index) => MenuSlot::Container(u16::try_from(index).ok()?),
        McSlot::Output => MenuSlot::Output,
        McSlot::Creative(_) | McSlot::Trash => return None,
    })
}

pub(crate) fn action_of(click: &McClick) -> Option<MenuAction> {
    Some(match click {
        McClick::Slot { slot, right, shift } => MenuAction::Slot { slot: menu_slot(*slot)?, right: *right, shift: *shift },
        McClick::Outside { right } => MenuAction::Outside { right: *right },
        McClick::Gather { slot, right } => MenuAction::Gather { slot: menu_slot(*slot)?, right: *right },
        McClick::Swap { slot, hotbar } => MenuAction::Swap { slot: menu_slot(*slot)?, hotbar: u8::try_from(*hotbar).ok()? },
        McClick::Spread { slots, right } => MenuAction::Spread { slots: slots.iter().filter_map(|slot| menu_slot(*slot)).collect(), right: *right },
        McClick::Throw { slot, whole } => MenuAction::Throw { slot: menu_slot(*slot)?, whole: *whole },
        McClick::Choose(index) => MenuAction::Choose(u16::try_from(*index).ok()?),
        McClick::PlaceRecipe { recipe, all } => MenuAction::PlaceRecipe { recipe: recipe.clone(), all: *all },
        McClick::Close | McClick::Rename(_) | McClick::Clone(_) | McClick::EditBook { .. } => return None,
    })
}

fn armor_slot(index: usize) -> Option<&'static str> {
    match index {
        36 => Some("feet"),
        37 => Some("legs"),
        38 => Some("chest"),
        39 => Some("head"),
        _ => None,
    }
}

fn droppable(
    screen: McScreen,
    carried: &ItemStack,
    inventory: &Inventory,
    opened: Option<&Opened>,
    bench: Option<&Bench>,
    containers: &Containers,
    ctx: &Context<'_>,
) -> Vec<(McSlot, u8)> {
    let max = stack_limit(&inventory.recipes, carried);
    let gun = minecraft_survival::guns::is_gun(carried);
    let mut out: Vec<(McSlot, u8)> = (0..36).map(|i| (McSlot::Inventory(i), max)).collect();
    match screen {
        McScreen::Inventory => {
            for index in 36..40 {
                if armor_slot(index).is_some_and(|slot| inventory.recipes.equipment_slot(carried) == Some(slot)) {
                    out.push((McSlot::Inventory(index), 1));
                }
            }
            if !gun {
                out.push((McSlot::Inventory(OFFHAND), max));
            }
            out.extend((0..4).map(|i| (McSlot::Crafting(i), max)));
        }
        McScreen::CraftingTable => out.extend((0..9).map(|i| (McSlot::Workbench(i), max))),
        McScreen::Merchant => out.extend((0..2).map(|i| (McSlot::Container(i), max))),
        McScreen::Mount { columns, saddle, armor, llama } => {
            let columns = usize::from(columns);
            let mount = minecraft_terrain::mount_inventory::MountInventory { vehicle: 0, columns, saddle, armor, llama, cells: vec![None; minecraft_terrain::mount_inventory::CHEST + columns * 3] };
            out.extend((0..mount.cells.len()).filter_map(|i| Some((McSlot::Container(i), mount.capacity(i, carried)?.min(max)))));
        }
        McScreen::Creative(frame::McTabKind::Inventory) => {
            for index in 36..40 {
                if armor_slot(index).is_some_and(|slot| inventory.recipes.equipment_slot(carried) == Some(slot)) {
                    out.push((McSlot::Inventory(index), 1));
                }
            }
            if !gun {
                out.push((McSlot::Inventory(OFFHAND), max));
            }
        }
        _ => {}
    }
    if let Some(opened) = opened {
        for index in 0..opened.size() {
            if let Some(capacity) = containers.accepts(opened, index, carried, &inventory.recipes) {
                out.push((McSlot::Container(index), capacity.min(max)));
            }
        }
    }
    if let Some(bench) = bench {
        for index in 0..bench.cells.len() {
            if let Some(capacity) = bench.accepts_stack(index, carried, ctx) {
                out.push((McSlot::Container(index), capacity.min(max)));
            }
        }
    }
    out
}

const SHOWN_CONTENTS: usize = 5;

fn stored_in_order(stack: &ItemStack) -> Vec<ItemStack> {
    let mut stored = minecraft_terrain::containers::stored_items(stack);
    stored.sort_by_key(|(slot, _)| *slot);
    stored.into_iter().map(|(_, item)| item).collect()
}

fn translated(language: &HashMap<String, String>, keys: [&str; 2], fallback: &str, arguments: &[String]) -> String {
    let template = keys.iter().find_map(|key| language.get(*key)).map_or(fallback, String::as_str);
    arguments.iter().fold(template.to_owned(), |text, argument| text.replacen("%s", argument, 1))
}

fn contents_lines(language: &HashMap<String, String>, stack: &ItemStack) -> Vec<String> {
    stored_in_order(stack)
        .iter()
        .take(SHOWN_CONTENTS)
        .map(|item| {
            let name = display_name(language, item).unwrap_or_else(|| minecraft_terrain::item_icons::item_name(language, &item.id));
            translated(language, ["item.container.item_count", "container.shulkerBox.itemCount"], "%s x %s", &[item.count.to_string(), name])
        })
        .collect()
}

fn more_line(language: &HashMap<String, String>, stack: &ItemStack) -> Option<String> {
    let rest = stored_in_order(stack).len().checked_sub(SHOWN_CONTENTS).filter(|rest| *rest > 0)?;
    Some(translated(language, ["item.container.more_items", "container.shulkerBox.more"], "and %s more...", &[rest.to_string()]))
}

fn main_hand_attack(recipes: &minecraftoss_player::crafting::RecipeBook, stack: &ItemStack) -> Option<[f32; 2]> {
    let attack = |m: &minecraftoss_player::item_catalog::AttributeModifier| {
        m.in_main_hand() && matches!(m.attribute.as_str(), "minecraft:attack_damage" | "minecraft:attack_speed")
    };
    recipes.item_modifiers(stack).iter().any(attack).then(|| {
        let (damage, speed) = recipes.attack_attributes(Some(stack));
        [damage as f32, speed as f32]
    })
}

fn display_name(language: &HashMap<String, String>, stack: &ItemStack) -> Option<String> {
    if let Some(name) = minecraft_terrain::stations::custom_name(stack) {
        return Some(name);
    }
    let title = written_content(stack).and_then(|content| content.get("title"));
    if let Some(title) = title.map(|title| title.get("raw").unwrap_or(title)).and_then(serde_json::Value::as_str).filter(|title| !title.trim().is_empty()) {
        return Some(title.to_owned());
    }
    if let Some(name) = minecraft_terrain::lodestone::name_key(stack).and_then(|key| language.get(key)) {
        return Some(name.clone());
    }
    let potion = minecraft_terrain::stations::potion_of(stack)?;
    let effect = potion.rsplit(':').next()?.trim_start_matches("long_").trim_start_matches("strong_");
    let item = stack.id.rsplit(':').next()?;
    language.get(&format!("item.minecraft.{item}.effect.{effect}")).cloned()
}

fn offer_stacks(recipes: &minecraftoss_player::crafting::RecipeBook, max_stacks: &[(String, i32)], offer: &minecraftoss_entities::trading::MerchantOffer) -> [Option<ItemStack>; 3] {
    let stack = |id: &str, count: i32, components: Option<serde_json::Value>| ItemStack {
        id: id.to_owned(),
        count: u8::try_from(count.max(1)).unwrap_or(u8::MAX),
        max: recipes.max_stack(id),
        components,
    };
    let max_a = max_stacks.iter().find(|(id, _)| *id == offer.buy.id).map_or_else(|| i32::from(recipes.max_stack(&offer.buy.id)), |&(_, max)| max);
    let cost_a = stack(&offer.buy.id, offer.cost_a_count(max_a), offer.buy.components.clone());
    let cost_b = offer.buy_b.as_ref().map(|cost| stack(&cost.id, cost.count, cost.components.clone()));
    let result = stack(&offer.sell.id, offer.sell.count, (!offer.sell.components.is_empty()).then(|| serde_json::Value::Object(offer.sell.components.clone())));
    [Some(cost_a), cost_b, Some(result)]
}

fn written_content(stack: &ItemStack) -> Option<&serde_json::Value> {
    stack.components.as_ref()?.get(minecraft_survival::books::WRITTEN_BOOK_CONTENT)
}

fn book_lines(language: &HashMap<String, String>, stack: &ItemStack) -> Vec<String> {
    let Some(content) = written_content(stack) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    if let Some(author) = content.get("author").and_then(serde_json::Value::as_str).filter(|author| !author.trim().is_empty()) {
        let lang = |key: &str| Some(language.get(key).map_or("by %1$s", String::as_str).to_owned());
        let by = minecraft_commands::Text::translate("book.byAuthor", vec![minecraft_commands::Text::literal(author)]);
        lines.push(by.resolve(&lang).text);
    }
    let generation = content.get("generation").and_then(serde_json::Value::as_i64).unwrap_or(0).clamp(0, 3) as usize;
    let fallback = ["Original", "Copy of original", "Copy of a copy", "Tattered"][generation];
    lines.push(language.get(&format!("book.generation.{generation}")).map_or(fallback, String::as_str).to_owned());
    lines
}

fn bench_screen(kind: Workshop) -> McScreen {
    match kind {
        Workshop::Stonecutter => McScreen::Stonecutter,
        Workshop::Smithing => McScreen::Smithing,
        Workshop::Grindstone => McScreen::Grindstone,
        Workshop::Anvil => McScreen::Anvil,
        Workshop::Enchanting => McScreen::Enchanting,
    }
}

fn screen_of(opened: &Opened) -> McScreen {
    match opened.station {
        Station::Oven(CookingKind::Furnace) => McScreen::Furnace(McOven::Furnace),
        Station::Oven(CookingKind::Smoker) => McScreen::Furnace(McOven::Smoker),
        Station::Oven(CookingKind::BlastFurnace) => McScreen::Furnace(McOven::BlastFurnace),
        Station::Brewer => McScreen::Brewing,
        Station::Shulker => McScreen::ShulkerBox,
        _ => McScreen::Chest { rows: opened.rows() as u8 },
    }
}

/// Throws stacks from the player's eye, as `Player.drop` does.
pub(crate) fn throw(items: &mut minecraftoss_player::items::WorldItems, stacks: Vec<ItemStack>, thrower: &Thrower) {
    for stack in stacks {
        items.toss(stack, thrower.eye, thrower.yaw, thrower.pitch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_clicks_become_menu_actions_and_the_rest_stay_on_the_screen() {
        let spread = McClick::Spread { slots: vec![McSlot::Inventory(3), McSlot::Creative(2), McSlot::Container(40)], right: true };
        assert_eq!(action_of(&spread), Some(MenuAction::Spread { slots: vec![MenuSlot::Inventory(3), MenuSlot::Container(40)], right: true }));
        let swap = McClick::Swap { slot: McSlot::Workbench(8), hotbar: 40 };
        assert_eq!(action_of(&swap), Some(MenuAction::Swap { slot: MenuSlot::Workbench(8), hotbar: 40 }));
        assert_eq!(action_of(&McClick::Choose(2)), Some(MenuAction::Choose(2)));
        let output = McClick::Throw { slot: McSlot::Output, whole: true };
        assert_eq!(action_of(&output), Some(MenuAction::Throw { slot: MenuSlot::Output, whole: true }));
        let place = McClick::PlaceRecipe { recipe: "iw4l:m4".to_owned(), all: true };
        assert_eq!(action_of(&place), Some(MenuAction::PlaceRecipe { recipe: "iw4l:m4".to_owned(), all: true }));
        for kept in [McClick::Close, McClick::Rename("Blade".to_owned()), McClick::Clone(McSlot::Inventory(0)), McClick::Slot { slot: McSlot::Creative(5), right: false, shift: false }] {
            assert_eq!(action_of(&kept), None, "{kept:?}");
        }
        assert_eq!(screen_for(None), McScreen::Inventory);
        assert_eq!(screen_for(Some(&Open::Table((0, 0, 0)))), McScreen::CraftingTable);
        let chest = Opened { station: Station::Chest, cells: vec![(0, 0, 0), (1, 0, 0)] };
        assert_eq!(screen_for(Some(&Open::Block(chest))), McScreen::Chest { rows: 6 });
    }

    #[test]
    fn a_trading_offer_shows_its_discounted_first_cost_its_second_cost_and_what_it_sells() {
        use minecraftoss_entities::trading::{ItemCost, MerchantOffer, TradeItem};
        let recipes = minecraftoss_player::crafting::RecipeBook::default();
        let mut sells = serde_json::Map::new();
        sells.insert("minecraft:stored_enchantments".to_owned(), serde_json::json!({"minecraft:mending": 1}));
        let offer = MerchantOffer {
            buy: ItemCost { id: "minecraft:emerald".to_owned(), count: 10, components: None },
            buy_b: Some(ItemCost { id: "minecraft:book".to_owned(), count: 1, components: None }),
            sell: TradeItem { id: "minecraft:enchanted_book".to_owned(), count: 1, components: sells },
            uses: 0,
            max_uses: 12,
            reward_exp: true,
            special_price: -3,
            demand: 0,
            price_multiplier: 0.2,
            xp: 1,
        };
        let [a, b, result] = offer_stacks(&recipes, &[("minecraft:emerald".to_owned(), 64)], &offer);
        assert_eq!(a.map(|s| (s.id, s.count)), Some(("minecraft:emerald".to_owned(), 7)));
        assert_eq!(b.map(|s| (s.id, s.count)), Some(("minecraft:book".to_owned(), 1)));
        let result = result.unwrap();
        assert_eq!((result.id.as_str(), result.count), ("minecraft:enchanted_book", 1));
        assert!(result.components.is_some_and(|c| c.get("minecraft:stored_enchantments").is_some()));
        let pricey = MerchantOffer { special_price: 80, buy_b: None, ..offer };
        let [a, b, _] = offer_stacks(&recipes, &[("minecraft:emerald".to_owned(), 64)], &pricey);
        assert_eq!(a.map(|s| s.count), Some(64));
        assert!(b.is_none());
    }

    #[test]
    fn a_written_book_is_named_by_its_title_with_author_and_generation_lines() {
        use minecraft_survival::books::{WRITABLE_BOOK, WRITTEN_BOOK, WRITTEN_BOOK_CONTENT};
        let language: HashMap<String, String> = [
            ("book.byAuthor", "by %1$s"),
            ("book.generation.0", "Original"),
            ("book.generation.1", "Copy of original"),
            ("book.generation.2", "Copy of a copy"),
            ("book.generation.3", "Tattered"),
        ]
        .into_iter()
        .map(|(key, text)| (key.to_owned(), text.to_owned()))
        .collect();
        let book = |content: serde_json::Value| {
            let mut stack = ItemStack::new(WRITTEN_BOOK, 1);
            stack.components = Some(serde_json::json!({WRITTEN_BOOK_CONTENT: content}));
            stack
        };
        let signed = book(serde_json::json!({"title": {"raw": "Diary"}, "author": "Alex", "pages": [{"raw": "a"}], "resolved": true}));
        assert_eq!(display_name(&language, &signed).as_deref(), Some("Diary"));
        assert!(minecraft_terrain::stations::custom_name(&signed).is_none(), "the title is not a custom name, so it is not italic");
        assert_eq!(book_lines(&language, &signed), ["by Alex", "Original"]);
        let copied = book(serde_json::json!({"title": "Plain", "author": "  ", "generation": 2}));
        assert_eq!(display_name(&language, &copied).as_deref(), Some("Plain"));
        assert_eq!(book_lines(&language, &copied), ["Copy of a copy"]);
        let tattered = book(serde_json::json!({"title": {"raw": " "}, "author": "Owner", "generation": 3}));
        assert_eq!(display_name(&language, &tattered), None, "a blank title keeps the item name");
        assert_eq!(book_lines(&language, &tattered), ["by Owner", "Tattered"]);
        let mut renamed = signed.clone();
        renamed.components.as_mut().unwrap()["minecraft:custom_name"] = serde_json::json!("Secret");
        assert_eq!(display_name(&language, &renamed).as_deref(), Some("Secret"));
        assert_eq!(book_lines(&HashMap::new(), &copied), ["Copy of a copy"]);
        assert_eq!(book_lines(&HashMap::new(), &signed), ["by Alex", "Original"]);
        let mut quill = ItemStack::new(WRITABLE_BOOK, 1);
        assert!(book_lines(&language, &quill).is_empty());
        quill.components = Some(serde_json::json!({"minecraft:writable_book_content": {"pages": [{"raw": "x"}]}}));
        assert_eq!(display_name(&language, &quill), None);
        assert!(book_lines(&language, &quill).is_empty());
    }

    #[test]
    fn a_lodestone_compass_is_named_and_glints_and_a_custom_name_comes_first() {
        use minecraft_terrain::lodestone::{self, Target, COMPASS, NAME_KEY};
        let language: HashMap<String, String> = [(NAME_KEY.to_owned(), "Lodestone Compass".to_owned())].into_iter().collect();
        let plain = ItemStack::new(COMPASS, 1);
        assert_eq!(display_name(&language, &plain), None);
        let mut tracked = lodestone::with_tracker(&plain, Target::new(minecraft_net::Dimension::Overworld, (1, 2, 3)));
        assert_eq!(display_name(&language, &tracked).as_deref(), Some("Lodestone Compass"));
        assert!(minecraft_terrain::glint::has_glint(&tracked));
        let mut inventory = minecraftoss_player::inventory::Inventory::default();
        inventory.slots[0] = Some(tracked.clone());
        lodestone::forget(&mut inventory, minecraft_net::Dimension::Overworld, (1, 2, 3));
        let lost = inventory.slots[0].clone().unwrap();
        assert_eq!(display_name(&language, &lost).as_deref(), Some("Lodestone Compass"), "a broken lodestone keeps the name");
        tracked.components.as_mut().unwrap()["minecraft:custom_name"] = serde_json::json!("Home");
        assert_eq!(display_name(&language, &tracked).as_deref(), Some("Home"));
    }

    #[test]
    fn two_potions_share_an_id_and_a_name_but_not_an_icon() {
        let potion = |name: &str| {
            let mut stack = ItemStack::new("minecraft:potion", 1);
            stack.components = Some(serde_json::json!({"minecraft:potion_contents": {"potion": name}}));
            stack
        };
        let (healing, poison) = (potion("minecraft:healing"), potion("minecraft:poison"));
        let (healing_icon, poison_icon) = (minecraft_terrain::item_tint::of_stack(&healing), minecraft_terrain::item_tint::of_stack(&poison));
        assert_eq!(healing.id, poison.id);
        assert_ne!(healing_icon, poison_icon);
        let dials = minecraft_terrain::dials::Dials::default();
        assert_eq!(icon_source(&dials, &healing_icon), ("minecraft:potion", "minecraft:potion~f82423".to_owned()));
        assert_eq!(icon_source(&dials, &poison_icon), ("minecraft:potion", "minecraft:potion~87a363".to_owned()));
        assert_eq!(icon_source(&dials, "minecraft:clock"), ("minecraft:clock", "minecraft:clock".to_owned()));
    }

    #[test]
    fn a_lodestone_compass_icon_is_keyed_by_its_target_but_named_by_its_item() {
        use minecraft_terrain::lodestone::{self, Target, COMPASS};
        let plain = ItemStack::new(COMPASS, 1);
        let at = |pos| minecraft_terrain::item_tint::of_stack(&lodestone::with_tracker(&plain, Target::new(minecraft_net::Dimension::Overworld, pos)));
        let (first, second) = (at((1, 2, 3)), at((9, 9, 9)));
        assert_ne!(first, second);
        assert_ne!(first, minecraft_terrain::item_tint::of_stack(&plain));
        let dials = minecraft_terrain::dials::Dials::default();
        assert_eq!(icon_source(&dials, &first), (COMPASS, COMPASS.to_owned()), "named as a compass, drawn as one until its needle is stepped");
    }

    #[test]
    fn a_shulker_box_tooltip_names_five_stacks_in_slot_order_and_counts_the_rest() {
        let mut language = HashMap::new();
        language.insert("container.shulkerBox.itemCount".to_owned(), "%s x %s".to_owned());
        language.insert("container.shulkerBox.more".to_owned(), "and %s more...".to_owned());
        language.insert("block.minecraft.stone".to_owned(), "Stone".to_owned());
        language.insert("item.minecraft.diamond".to_owned(), "Diamond".to_owned());
        let entries: Vec<serde_json::Value> = [(20, "minecraft:diamond", 3), (0, "minecraft:stone", 64), (4, "minecraft:stone", 1), (5, "minecraft:stone", 2), (7, "minecraft:stone", 3), (9, "minecraft:stone", 4), (26, "minecraft:diamond", 1)]
            .into_iter()
            .map(|(slot, id, count)| serde_json::json!({"slot": slot, "item": {"id": id, "count": count}}))
            .collect();
        let mut stack = ItemStack::new("minecraft:shulker_box", 1);
        stack.components = Some(serde_json::json!({"minecraft:container": entries}));
        assert_eq!(contents_lines(&language, &stack), ["64 x Stone", "1 x Stone", "2 x Stone", "3 x Stone", "4 x Stone"]);
        assert_eq!(more_line(&language, &stack).as_deref(), Some("and 2 more..."));
        let empty = ItemStack::new("minecraft:shulker_box", 1);
        assert!(contents_lines(&language, &empty).is_empty());
        assert_eq!(more_line(&language, &empty), None);
    }

    #[test]
    fn a_friends_lectern_screen_sends_its_buttons_and_closes_once() {
        use minecraft_terrain::lectern::LecternAction;
        let mut inventory_ui = InventoryUi::default();
        let mut ui = MinecraftUi::default();
        assert!(!inventory_ui.book_clicks(&mut ui), "no lectern open");
        let pos = (4, 64, -2);
        let book = frame::McBook { pages: vec!["one".to_owned()], page: 0, take: false };
        assert!(inventory_ui.show_lectern(&mut ui, pos, book));
        ui.clicks = vec![McClick::Choose(2), McClick::Choose(101)];
        assert!(inventory_ui.book_clicks(&mut ui));
        assert_eq!(inventory_ui.take_lectern_actions(), [(pos, LecternAction::Button(2)), (pos, LecternAction::Button(101))]);
        assert_eq!(ui.screen, McScreen::Lectern);
        ui.clicks = vec![McClick::Close];
        assert!(inventory_ui.book_clicks(&mut ui));
        assert_eq!(inventory_ui.take_lectern_actions(), [(pos, LecternAction::Close)]);
        assert!(!ui.inventory_open && ui.book.is_none() && inventory_ui.lectern().is_none());
        assert!(!inventory_ui.book_clicks(&mut ui));
    }

    #[test]
    fn a_lectern_screen_sends_its_buttons_and_closes_once() {
        use minecraft_terrain::lectern::LecternAction;
        let mut inventory_ui = InventoryUi::default();
        let mut ui = MinecraftUi::default();
        let profile = minecraft_commands::server::Profile { id: minecraft_commands::PlayerId(0), name: "Steve".to_owned(), uuid: "00000000-0000-0000-0000-000000000001".to_owned() };
        let mut player = Player::new(&profile, Default::default(), Default::default(), Inventory::default());
        let mut containers = Containers::default();
        let data = minecraft_survival::data::GameData::default();
        let pos = (4, 64, -2);
        let book = |page: usize| frame::McBook { pages: vec!["one".to_owned(), "two".to_owned()], page, take: true };
        assert!(inventory_ui.show_lectern(&mut ui, pos, book(0)));
        assert!(ui.inventory_open && ui.screen_opened);
        assert_eq!(ui.screen, McScreen::Lectern);
        ui.clicks = vec![McClick::Choose(2), McClick::Slot { slot: McSlot::Inventory(0), right: false, shift: false }, McClick::Choose(3)];
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert_eq!(ui.screen, McScreen::Lectern);
        assert_eq!(inventory_ui.take_lectern_actions(), [(pos, LecternAction::Button(2)), (pos, LecternAction::Button(3))]);
        ui.screen_opened = false;
        assert!(inventory_ui.show_lectern(&mut ui, pos, book(1)));
        assert!(!ui.screen_opened);
        assert_eq!(ui.book.as_ref().map(|book| book.page), Some(1));
        assert!(!inventory_ui.show_lectern(&mut ui, (0, 0, 0), book(0)));
        ui.inventory_open = false;
        ui.clicks = vec![McClick::Close];
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert_eq!(inventory_ui.take_lectern_actions(), [(pos, LecternAction::Close)]);
        assert_eq!(ui.screen, McScreen::Inventory);
        assert!(ui.book.is_none() && inventory_ui.lectern().is_none());
        assert!(inventory_ui.show_lectern(&mut ui, pos, book(0)));
        inventory_ui.lectern_closed(&mut ui, pos);
        assert!(!ui.inventory_open && ui.book.is_none());
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert!(inventory_ui.take_lectern_actions().is_empty());
        assert!(inventory_ui.show_lectern(&mut ui, pos, book(0)));
        ui.inventory_open = false;
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert_eq!(inventory_ui.take_lectern_actions(), [(pos, LecternAction::Close)]);
    }

    #[test]
    fn a_held_book_turns_pages_locally_and_closes_without_a_lectern_action() {
        let mut inventory_ui = InventoryUi::default();
        let mut ui = MinecraftUi::default();
        let profile = minecraft_commands::server::Profile { id: minecraft_commands::PlayerId(0), name: "Steve".to_owned(), uuid: "00000000-0000-0000-0000-000000000001".to_owned() };
        let mut player = Player::new(&profile, Default::default(), Default::default(), Inventory::default());
        let mut containers = Containers::default();
        let data = minecraft_survival::data::GameData::default();
        let book = frame::McBook { pages: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()], page: 0, take: false };
        assert!(!inventory_ui.book_clicks(&mut ui), "no book open");
        assert!(inventory_ui.show_held_book(&mut ui, OFFHAND, book.clone()));
        assert!(ui.inventory_open && ui.screen_opened);
        assert_eq!(ui.screen, McScreen::Lectern);
        assert_eq!(inventory_ui.lectern(), None, "a held book is not a lectern");
        let page = |ui: &MinecraftUi| ui.book.as_ref().map(|book| book.page);
        ui.clicks = vec![McClick::Choose(1), McClick::Choose(2), McClick::Choose(2), McClick::Choose(2), McClick::Choose(3)];
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert_eq!(page(&ui), Some(2), "turns stop at the last page and Take Book is ignored");
        assert_eq!(ui.screen, McScreen::Lectern);
        assert!(inventory_ui.take_lectern_actions().is_empty());
        ui.clicks = vec![McClick::Choose(1)];
        assert!(inventory_ui.book_clicks(&mut ui));
        assert_eq!(page(&ui), Some(1));
        assert!(inventory_ui.take_lectern_actions().is_empty());
        ui.inventory_open = false;
        ui.clicks = vec![McClick::Close];
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert!(inventory_ui.take_lectern_actions().is_empty(), "no lectern action on close");
        assert!(ui.book.is_none());
        assert_eq!(ui.screen, McScreen::Inventory);
        assert!(!inventory_ui.book_clicks(&mut ui));
        ui.inventory_open = true;
        assert!(!inventory_ui.show_held_book(&mut ui, 0, book), "not over another screen");
        assert!(ui.book.is_none());
    }

    #[test]
    fn a_held_book_edit_reaches_the_inventory_slot() {
        use minecraft_survival::books::{WRITABLE_BOOK, WRITABLE_BOOK_CONTENT, WRITTEN_BOOK, WRITTEN_BOOK_CONTENT};
        let mut inventory_ui = InventoryUi::default();
        let mut ui = MinecraftUi::default();
        let profile = minecraft_commands::server::Profile { id: minecraft_commands::PlayerId(0), name: "Steve".to_owned(), uuid: "00000000-0000-0000-0000-000000000001".to_owned() };
        let mut player = Player::new(&profile, Default::default(), Default::default(), Inventory::default());
        let mut containers = Containers::default();
        let data = minecraft_survival::data::GameData::default();
        player.inventory.slots[2] = Some(ItemStack::new(WRITABLE_BOOK, 1));
        let empty = frame::McBook { pages: Vec::new(), page: 0, take: false };
        assert!(inventory_ui.show_held_book(&mut ui, 2, empty.clone()));
        ui.clicks = vec![McClick::EditBook { pages: vec!["Hello".to_owned(), "World".to_owned()], title: None }];
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert!(!ui.inventory_open && ui.book.is_none());
        assert!(inventory_ui.take_lectern_actions().is_empty());
        let stack = player.inventory.slots[2].clone().expect("the quill");
        assert_eq!(stack.id, WRITABLE_BOOK);
        assert_eq!(stack.components, Some(serde_json::json!({WRITABLE_BOOK_CONTENT: {"pages": [{"raw": "Hello"}, {"raw": "World"}]}})));
        assert!(inventory_ui.show_held_book(&mut ui, 2, empty.clone()));
        ui.clicks = vec![McClick::EditBook { pages: vec!["Hello".to_owned()], title: Some("Diary".to_owned()) }, McClick::Close];
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        let signed = player.inventory.slots[2].clone().expect("the signed book");
        assert_eq!(signed.id, WRITTEN_BOOK);
        assert_eq!(signed.components, Some(serde_json::json!({WRITTEN_BOOK_CONTENT: {"title": {"raw": "Diary"}, "author": "Steve", "pages": [{"raw": "Hello"}], "resolved": true}})));
        let pos = (4, 64, -2);
        player.inventory.slots[3] = Some(ItemStack::new(WRITABLE_BOOK, 1));
        assert!(inventory_ui.show_lectern(&mut ui, pos, empty));
        ui.clicks = vec![McClick::EditBook { pages: vec!["x".to_owned()], title: None }];
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert_eq!(inventory_ui.lectern(), Some(pos), "a lectern book is not edited");
        assert!(ui.inventory_open);
        assert_eq!(player.inventory.slots[3], Some(ItemStack::new(WRITABLE_BOOK, 1)));
    }

    #[test]
    fn a_book_and_quill_opens_the_editor_and_keeps_its_screen() {
        use minecraft_survival::books::{WRITABLE_BOOK, WRITABLE_BOOK_CONTENT};
        let mut inventory_ui = InventoryUi::default();
        let mut ui = MinecraftUi::default();
        let profile = minecraft_commands::server::Profile { id: minecraft_commands::PlayerId(0), name: "Steve".to_owned(), uuid: "00000000-0000-0000-0000-000000000001".to_owned() };
        let mut player = Player::new(&profile, Default::default(), Default::default(), Inventory::default());
        let mut containers = Containers::default();
        let data = minecraft_survival::data::GameData::default();
        player.inventory.slots[OFFHAND] = Some(ItemStack::new(WRITABLE_BOOK, 1));
        assert!(inventory_ui.show_book_editor(&mut ui, OFFHAND, Vec::new(), "Steve"));
        assert_eq!(ui.screen, McScreen::BookEdit);
        assert!(ui.inventory_open && ui.screen_opened);
        let edit = ui.book_edit.clone().expect("the editor");
        assert_eq!((edit.pages, edit.page, edit.cursor, edit.author.as_str()), (vec![String::new()], 0, 0, "Steve"));
        assert!(!inventory_ui.show_book_editor(&mut ui, 0, Vec::new(), "Steve"), "not over another screen");
        ui.screen = McScreen::BookSign;
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert_eq!(ui.screen, McScreen::BookSign, "the sign screen stays");
        ui.screen = McScreen::BookEdit;
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert_eq!(ui.screen, McScreen::BookEdit);
        ui.clicks = vec![McClick::Close];
        ui.inventory_open = false;
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert!(ui.book_edit.is_none());
        assert_eq!(player.inventory.slots[OFFHAND], Some(ItemStack::new(WRITABLE_BOOK, 1)), "escape keeps nothing");
        let long = "a".repeat(1100);
        assert!(inventory_ui.show_book_editor(&mut ui, OFFHAND, vec![long, "two".to_owned()], "Steve"));
        let edit = ui.book_edit.clone().expect("the editor");
        assert_eq!((edit.pages[0].len(), edit.cursor, edit.pages[1].as_str()), (1024, 1024, "two"));
        ui.clicks = vec![McClick::EditBook { pages: vec!["Hi".to_owned()], title: None }, McClick::Close];
        let _ = inventory_ui.apply_input(&mut ui, &mut player, &mut containers, Books::of(&data), 0.5);
        assert!(!ui.inventory_open && ui.book_edit.is_none());
        let stack = player.inventory.slots[OFFHAND].clone().expect("the quill");
        assert_eq!(stack.components, Some(serde_json::json!({WRITABLE_BOOK_CONTENT: {"pages": [{"raw": "Hi"}]}})));
    }
}
