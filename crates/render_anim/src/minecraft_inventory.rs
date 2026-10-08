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
use minecraft_terrain::containers::{Containers, Opened, Station};
use minecraft_terrain::stations::{Bench, Context, StationBook, Taken, Workshop};
use minecraft_terrain::crafting::{self, Grid, Spot};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::BlockPos;
use minecraftoss_player::crafting::CookingKind;
use minecraftoss_player::inventory::{Inventory, ItemStack};

/// Items that are MW2 guns: `iw4:weapon/<weapon index>`.
const WEAPON_PREFIX: &str = "iw4:weapon/";
const ICON_COLUMNS: u32 = 16;
const ICON_ATLAS_LIMIT: u32 = 8192;
const OFFHAND: usize = 40;

pub(crate) fn weapon_of(stack: &ItemStack) -> Option<u32> {
    stack.id.strip_prefix(WEAPON_PREFIX)?.parse().ok()
}

fn weapon_item(weapon: u32) -> ItemStack {
    let mut stack = ItemStack::new(format!("{WEAPON_PREFIX}{weapon}"), 1);
    stack.max = 1;
    stack
}

#[derive(Default)]
pub(crate) struct InventoryUi {
    atlas: Option<image::RgbaImage>,
    handle: Option<Handle<Image>>,
    cells: HashMap<String, u32>,
    failed: HashSet<String>,
    language: Option<HashMap<String, String>>,
    dirty: bool,
    /// The gun last asked for, until the player state holds it.
    pending_weapon: Option<u32>,
    last_selected: Option<usize>,
    table: Option<BlockPos>,
    block: Option<Opened>,
    experience: u32,
    bench: Option<(BlockPos, Bench)>,
    pub(crate) level: u32,
    pub(crate) shelves: i32,
    seed: i32,
    spent: u32,
    pub(crate) used: Vec<(&'static str, BlockPos)>,
    pub(crate) anvil_used: Option<BlockPos>,
    book: std::sync::Arc<StationBook>,
    enchantments: Option<std::sync::Arc<minecraftoss_entities::enchant::Enchantments>>,
    gui: Option<crate::minecraft_gui::GuiAssets>,
    gui_failed: Option<u64>,
    icon_scale: u32,
    icon_rows: u32,
    pub(crate) animation_tick: u64,
    animated: AnimatedIcons,
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
        let keep = |stack: &Option<ItemStack>| {
            stack.as_ref().is_none_or(|s| weapon_of(s).is_none_or(|w| owned.contains(&w)))
        };
        for slot in inventory.slots.iter_mut().chain(inventory.crafting.iter_mut()).chain(inventory.workbench.iter_mut()) {
            if !keep(slot) {
                *slot = None;
            }
        }
        if !keep(&inventory.cursor) {
            inventory.cursor = None;
        }
        let held: HashSet<u32> = inventory
            .slots
            .iter()
            .chain(inventory.crafting.iter())
            .chain(inventory.workbench.iter())
            .chain(std::iter::once(&inventory.cursor))
            .filter_map(|s| s.as_ref().and_then(weapon_of))
            .collect();
        for &weapon in owned {
            if held.contains(&weapon) {
                continue;
            }
            let free = (0..frame::minecraft_ui::MC_HOTBAR)
                .chain(frame::minecraft_ui::MC_HOTBAR..36)
                .find(|&i| inventory.slots[i].is_none());
            if let Some(i) = free {
                inventory.slots[i] = Some(weapon_item(weapon));
            }
        }
    }

    pub(crate) fn table(&self) -> Option<BlockPos> {
        self.table
    }

    pub(crate) fn open_table(&mut self, ui: &mut MinecraftUi, pos: BlockPos) {
        if ui.inventory_open {
            return;
        }
        self.table = Some(pos);
        ui.inventory_open = true;
        ui.screen = McScreen::CraftingTable;
        ui.screen_opened = true;
    }

    pub(crate) fn block(&self) -> Option<&Opened> {
        self.block.as_ref()
    }

    pub(crate) fn open_block(&mut self, ui: &mut MinecraftUi, opened: Opened) -> bool {
        if ui.inventory_open {
            return false;
        }
        ui.screen = screen_of(&opened);
        ui.screen_title = self.text(station_title(&opened));
        self.block = Some(opened);
        ui.inventory_open = true;
        ui.screen_opened = true;
        true
    }

    pub(crate) fn language(&self) -> Option<&HashMap<String, String>> {
        self.language.as_ref()
    }

    fn text(&self, key: &str) -> String {
        self.language.as_ref().and_then(|language| language.get(key).cloned()).unwrap_or_else(|| key.to_owned())
    }

    pub(crate) fn take_experience(&mut self) -> u32 {
        std::mem::take(&mut self.experience)
    }

    pub(crate) fn take_spent_levels(&mut self) -> u32 {
        std::mem::take(&mut self.spent)
    }

    pub(crate) fn set_station_data(
        &mut self,
        book: std::sync::Arc<StationBook>,
        enchantments: Option<std::sync::Arc<minecraftoss_entities::enchant::Enchantments>>,
        seed: i32,
    ) {
        self.book = book;
        self.enchantments = enchantments;
        self.seed = seed;
    }

    pub(crate) fn bench(&self) -> Option<(BlockPos, Workshop)> {
        self.bench.as_ref().map(|(pos, bench)| (*pos, bench.kind))
    }

    pub(crate) fn open_bench(&mut self, ui: &mut MinecraftUi, pos: BlockPos, kind: Workshop) -> bool {
        if ui.inventory_open {
            return false;
        }
        self.bench = Some((pos, Bench::new(kind)));
        ui.screen = bench_screen(kind);
        ui.screen_title = self.text(bench_title(kind));
        ui.inventory_open = true;
        ui.screen_opened = true;
        true
    }

    fn context<'a>(&'a self, recipes: &'a minecraftoss_player::crafting::RecipeBook) -> Context<'a> {
        Context {
            recipes,
            book: &self.book,
            enchantments: self.enchantments.as_deref(),
            bookshelves: self.shelves,
            seed: self.seed,
            level: self.level,
        }
    }

    fn taken(&mut self, pos: BlockPos, kind: Workshop, taken: Taken, roll: f32) {
        self.spent += taken.levels;
        self.level = self.level.saturating_sub(taken.levels);
        self.experience += taken.experience;
        if !taken.sound.is_empty() && kind != Workshop::Anvil {
            self.used.push((taken.sound, pos));
        }
        if taken.new_seed {
            self.seed = (roll * i32::MAX as f32) as i32;
        }
        if kind == Workshop::Anvil {
            self.anvil_used = Some(pos);
        }
    }

    fn bench_click(
        &mut self,
        click: &McClick,
        inventory: &mut Inventory,
        roll: &mut dyn FnMut() -> f32,
        thrown: &mut Vec<ItemStack>,
    ) -> bool {
        let Some((pos, mut bench)) = self.bench.take() else {
            return false;
        };
        let recipes = inventory.recipes.clone();
        let kind = bench.kind;
        let mut handled = true;
        let mut taken: Option<Taken> = None;
        {
            let ctx = self.context(&recipes);
            match click {
                McClick::Throw { slot: McSlot::Container(index), whole } => {
                    if inventory.cursor.is_none() {
                        thrown.extend(bench.throw(*index, *whole));
                    }
                }
                McClick::Throw { slot: McSlot::Output, whole } => {
                    let first = bench.offer(&ctx).result;
                    while inventory.cursor.is_none() {
                        let mut random = minecraftoss_player::rng::LegacyRandom::new((roll() * u32::MAX as f32) as u64);
                        let Some(step) = bench.take(false, inventory, &ctx, &mut random) else { break };
                        thrown.extend(inventory.cursor.take());
                        let total = taken.get_or_insert_with(Taken::default);
                        total.levels += step.levels;
                        total.experience += step.experience;
                        total.sound = step.sound;
                        total.new_seed |= step.new_seed;
                        let next = bench.offer(&ctx).result;
                        if !*whole || next.as_ref().zip(first.as_ref()).is_none_or(|(next, first)| !next.same_item(first)) {
                            break;
                        }
                    }
                }
                McClick::Gather { .. } => bench.gather(inventory, false),
                McClick::Slot { slot: McSlot::Inventory(index), shift: true, .. } => {
                    bench.quick_move_from_player(*index, inventory, &ctx);
                }
                McClick::Slot { slot: McSlot::Container(index), right, shift } => {
                    bench.click(*index, *right, *shift, inventory, &ctx);
                }
                McClick::Slot { slot: McSlot::Output, shift, .. } => {
                    let mut random = minecraftoss_player::rng::LegacyRandom::new((roll() * u32::MAX as f32) as u64);
                    taken = bench.take(*shift, inventory, &ctx, &mut random);
                }
                McClick::Choose(index) => taken = bench.choose(*index, inventory, &ctx),
                McClick::Rename(name) => {
                    let default = bench.cells[0].as_ref().map(|s| {
                        self.language.as_ref().map_or_else(|| s.id.clone(), |language| minecraft_terrain::item_icons::item_name(language, &s.id))
                    });
                    let named = bench.cells[0].as_ref().and_then(minecraft_terrain::stations::custom_name);
                    if named.is_none() && default.as_deref() == Some(name.as_str()) {
                        bench.name = None;
                    } else {
                        bench.rename(name);
                    }
                }
                McClick::Swap { slot: McSlot::Container(index), hotbar } => bench.swap(*index, *hotbar, inventory, &ctx),
                McClick::Spread { slots, right } => {
                    let players: Vec<usize> = slots.iter().filter_map(|slot| if let McSlot::Inventory(i) = slot { Some(*i) } else { None }).collect();
                    let cells: Vec<usize> = slots.iter().filter_map(|slot| if let McSlot::Container(i) = slot { Some(*i) } else { None }).collect();
                    bench.spread(&players, &cells, *right, inventory, &ctx);
                }
                _ => handled = false,
            }
        }
        if let Some(taken) = taken {
            self.taken(pos, kind, taken, roll());
        }
        self.bench = Some((pos, bench));
        handled
    }

    fn close_bench(&mut self, inventory: &mut Inventory, selected: usize) -> Vec<ItemStack> {
        let Some((_, mut bench)) = self.bench.take() else {
            return Vec::new();
        };
        bench.close().into_iter().filter_map(|stack| inventory.add_item(stack, selected)).collect()
    }

    /// Applies the HUD's clicks, selection and drops with vanilla's rules.
    /// Stacks thrown out of the inventory are returned, except guns, which
    /// stay.
    pub(crate) fn apply_input(
        &mut self,
        ui: &mut MinecraftUi,
        inventory: &mut Inventory,
        selected: &mut usize,
        containers: &mut Containers,
        roll: &mut dyn FnMut() -> f32,
    ) -> Vec<ItemStack> {
        let mut thrown = Vec::new();
        for click in std::mem::take(&mut ui.clicks) {
            let grid = if self.table.is_some() { Grid::Table } else { Grid::Inventory };
            if keeps_gun(ui, &click) {
                continue;
            }
            if self.bench.is_some() {
                if click == McClick::Close {
                    thrown.extend(self.close_bench(inventory, *selected));
                } else if self.bench_click(&click, inventory, roll, &mut thrown) {
                    continue;
                }
            }
            if let Some(opened) = self.block.clone() {
                let cell = |slot: McSlot| match slot {
                    McSlot::Container(index) => Some(index),
                    McSlot::Output => Some(2),
                    _ => None,
                };
                match &click {
                    McClick::Slot { slot: McSlot::Inventory(index), right: _, shift: true } => {
                        containers.quick_move_from_player(&opened, inventory, *index);
                        continue;
                    }
                    McClick::Slot { slot, right, shift } if cell(*slot).is_some() => {
                        let index = cell(*slot).unwrap_or(0);
                        self.experience += containers.click(&opened, inventory, index, *right, *shift, roll()).experience;
                        continue;
                    }
                    McClick::Gather { right, .. } => {
                        containers.gather(&opened, inventory, *right);
                        continue;
                    }
                    McClick::Swap { slot, hotbar } if cell(*slot).is_some() => {
                        let index = cell(*slot).unwrap_or(0);
                        self.experience += containers.swap(&opened, inventory, index, *hotbar, roll()).experience;
                        continue;
                    }
                    McClick::Throw { slot, whole } if cell(*slot).is_some() => {
                        if inventory.cursor.is_none() {
                            let (stack, changed) = containers.throw(&opened, inventory, cell(*slot).unwrap_or(0), *whole, roll());
                            thrown.extend(stack);
                            self.experience += changed.experience;
                        }
                        continue;
                    }
                    McClick::Spread { slots, right } => {
                        let players: Vec<usize> = slots.iter().filter_map(|slot| if let McSlot::Inventory(i) = slot { Some(*i) } else { None }).collect();
                        let cells: Vec<usize> = slots.iter().filter_map(|slot| cell(*slot)).collect();
                        containers.spread(&opened, inventory, &players, &cells, *right);
                        continue;
                    }
                    McClick::Close => self.block = None,
                    _ => {}
                }
            }
            match click {
                McClick::Slot { slot: McSlot::Inventory(index), right, shift } => {
                    if shift && grid == Grid::Table {
                        crafting::quick_move_into_table(inventory, index);
                    } else if let Some(stack) = inventory.click(Some(index), right, shift) {
                        thrown.push(stack);
                    }
                }
                McClick::Slot { slot, right, shift } => match spot(slot, grid) {
                    Some(Spot::Grid(cell)) => crafting::click_cell(inventory, grid, cell, right, shift),
                    Some(Spot::Result) => {
                        crafting::click_result(inventory, grid, shift);
                    }
                    _ => {}
                },
                McClick::Outside { right } => {
                    if let Some(stack) = inventory.click(None, right, false) {
                        thrown.push(stack);
                    }
                }
                McClick::Gather { slot, right } => {
                    if let Some(target) = spot(slot, grid) {
                        crafting::gather(inventory, grid, target, right);
                    }
                }
                McClick::Swap { slot: McSlot::Inventory(index), hotbar } => inventory.number_swap(index, hotbar),
                McClick::Swap { slot, hotbar } => {
                    match spot(slot, grid) {
                        Some(Spot::Grid(cell)) => crafting::swap(inventory, grid, Some(cell), hotbar),
                        Some(Spot::Result) => crafting::swap(inventory, grid, None, hotbar),
                        _ => false,
                    };
                }
                McClick::Spread { slots, right } => {
                    let spots: Vec<Spot> = slots.into_iter().filter_map(|slot| spot(slot, grid)).collect();
                    crafting::spread(inventory, grid, &spots, right);
                }
                McClick::Throw { slot, whole } => {
                    if inventory.cursor.is_some() {
                        continue;
                    }
                    match spot(slot, grid) {
                        Some(Spot::Player(index)) => thrown.extend(throw_slot(inventory, index, whole)),
                        Some(Spot::Grid(cell)) => thrown.extend(crafting::throw_cell(inventory, grid, cell, whole)),
                        Some(Spot::Result) => thrown.extend(crafting::throw_result(inventory, grid, whole)),
                        None => {}
                    }
                }
                McClick::Choose(_) | McClick::Rename(_) => {}
                McClick::Close => {
                    self.table = None;
                    thrown.extend(crafting::close(inventory, *selected));
                }
            }
        }
        if !ui.inventory_open {
            self.table = None;
            self.block = None;
            thrown.extend(self.close_bench(inventory, *selected));
            if crafting::has_loose_items(inventory) {
                thrown.extend(crafting::close(inventory, *selected));
            }
        }
        ui.screen = match (&self.table, &self.block, &self.bench) {
            (Some(_), _, _) => McScreen::CraftingTable,
            (None, Some(opened), _) => screen_of(opened),
            (None, None, Some((_, bench))) => bench_screen(bench.kind),
            (None, None, None) => McScreen::Inventory,
        };
        thrown.extend(inventory.take_pending_drops());
        let _ = inventory.take_stat_events();
        if let Some(slot) = ui.select.take() {
            *selected = slot.min(frame::minecraft_ui::MC_HOTBAR - 1);
        }
        if let Some(whole) = ui.drop_selected.take()
            && let Some(stack) = inventory.drop_selected(*selected, whole)
        {
            thrown.push(stack);
        }
        // A gun never leaves the inventory.
        let mut out = Vec::new();
        for stack in thrown {
            if weapon_of(&stack).is_some() {
                if let Some(back) = inventory.add_item(stack, *selected) {
                    inventory.cursor = Some(back);
                }
            } else {
                out.push(stack);
            }
        }
        out
    }

    /// The gun the hotbar selection asks for, if the player holds another;
    /// and the selection follows a gun MW2 raised on its own.
    pub(crate) fn weapon_request(
        &mut self,
        inventory: &Inventory,
        selected: &mut usize,
        held: u32,
    ) -> Option<u32> {
        let selected_gun = inventory.slots[*selected].as_ref().and_then(weapon_of);
        let changed = self.last_selected != Some(*selected);
        self.last_selected = Some(*selected);
        if self.pending_weapon == Some(held) {
            self.pending_weapon = None;
        }
        if changed {
            if let Some(gun) = selected_gun.filter(|&gun| gun != held) {
                self.pending_weapon = Some(gun);
            }
        } else if self.pending_weapon.is_none()
            && selected_gun.is_some_and(|gun| gun != held)
            && let Some(slot) = (0..frame::minecraft_ui::MC_HOTBAR)
                .find(|&i| inventory.slots[i].as_ref().and_then(weapon_of) == Some(held))
        {
            // MW2 changed weapons (out of ammo, a pickup): the hotbar follows.
            *selected = slot;
            self.last_selected = Some(slot);
        }
        self.pending_weapon
    }

    /// Publishes the inventory for the HUD, with icons for every item in it.
    pub(crate) fn publish(
        &mut self,
        ui: &mut MinecraftUi,
        inventory: &Inventory,
        selected: usize,
        packs: &PackStack,
        images: &mut Assets<Image>,
        containers: &mut Containers,
    ) {
        if self.language.is_none() {
            self.language = Some(minecraft_terrain::item_icons::language(packs).unwrap_or_else(|_| HashMap::new()));
        }
        let generation = packs.generation;
        if self.gui.as_ref().is_none_or(|gui| gui.generation != generation) && self.gui_failed != Some(generation) {
            let empty = HashMap::new();
            let language = self.language.as_ref().unwrap_or(&empty);
            self.gui = crate::minecraft_gui::GuiAssets::load(packs, language, crate::minecraft_entities::data_jar().as_deref(), images);
            if self.gui.is_none() {
                self.gui_failed = Some(generation);
            }
        }
        ui.gui = self.gui.as_ref().map(|gui| gui.gui.clone());
        ui.glint = self.gui.as_ref().and_then(|gui| gui.glint.clone());
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
        let mut convert = |stack: &Option<ItemStack>| {
            stack.as_ref().map(|s| {
                stacks.push(s.id.clone());
                McStack {
                    id: s.id.clone(),
                    count: s.count,
                    weapon: weapon_of(s),
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
                }
            })
        };
        ui.slots = inventory.slots.iter().take(frame::minecraft_ui::MC_INVENTORY_SLOTS).map(&mut convert).collect();
        let table_open = ui.inventory_open && self.table.is_some();
        let small_open = ui.inventory_open && self.table.is_none();
        ui.crafting = std::array::from_fn(|i| convert(&inventory.crafting[i]));
        ui.result = if small_open { convert(&crafting::output(inventory, Grid::Inventory)) } else { None };
        ui.workbench = std::array::from_fn(|i| convert(&inventory.workbench[i]));
        ui.workbench_result = if table_open { convert(&crafting::output(inventory, Grid::Table)) } else { None };
        let opened = self.block.clone().filter(|_| ui.inventory_open);
        let (mut choices, mut chosen, mut cost, mut name) = (Vec::new(), None, None, None);
        let bench = self.bench.as_ref().filter(|_| ui.inventory_open).map(|(_, bench)| bench);
        let ctx = Context {
            recipes: &inventory.recipes,
            book: &self.book,
            enchantments: self.enchantments.as_deref(),
            bookshelves: self.shelves,
            seed: self.seed,
            level: self.level,
        };
        let (container, output, data) = match (opened.as_ref(), bench) {
            (None, Some(bench)) => {
                let offer = bench.offer(&ctx);
                let costs: Vec<u32> = offer.choices.iter().map(|choice| choice.cost).collect();
                let runes = match (bench.kind, self.gui.as_ref()) {
                    (Workshop::Enchanting, Some(gui)) => crate::minecraft_gui::enchanting_runes(&gui.words, &gui.gui, self.seed, &costs),
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
                let cells = containers.cells(opened);
                match opened.station {
                    Station::Brewer => (cells.iter().map(&mut convert).collect(), None, containers.brewer(opened.cells[0]).map_or([0; 4], |brewer| brewer.data())),
                    Station::Oven(_) => (
                        cells[..2].iter().map(&mut convert).collect(),
                        convert(&cells[2]),
                        containers.oven(opened.cells[0]).map_or([0; 4], |oven| oven.data()),
                    ),
                    _ => (cells.iter().map(&mut convert).collect(), None, [0; 4]),
                }
            }
            (None, None) => (Vec::new(), None, [0; 4]),
        };
        ui.choices = choices;
        ui.chosen = chosen;
        ui.cost = cost;
        ui.name = name;
        ui.container = container;
        ui.output = output;
        ui.data = data;
        ui.cursor = convert(&inventory.cursor);
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
            let shown: Vec<&String> = if ui.inventory_open {
                stacks.iter().collect()
            } else {
                let hotbar = inventory.slots.iter().take(frame::minecraft_ui::MC_HOTBAR);
                hotbar.chain(inventory.slots.get(OFFHAND)).chain(std::iter::once(&inventory.cursor)).flatten().map(|stack| &stack.id).collect()
            };
            let mut refreshed = HashSet::new();
            for id in shown {
                let Some(&cell) = self.animated.cells.get(id) else { continue };
                if !refreshed.insert(id) {
                    continue;
                }
                let clock = minecraft_terrain::item_icon::SpriteClock::at(animation_tick);
                let Ok(Some(rendered)) = minecraft_terrain::item_icons::item_icon_at(packs, id, icon as usize, &clock) else { continue };
                let rendered = image::imageops::resize(&rendered, icon, icon, image::imageops::FilterType::Nearest);
                self.animated.draw(cell, &rendered, icon);
            }
        }
        for id in stacks {
            if id.starts_with(WEAPON_PREFIX) {
                continue;
            }
            if !ui.names.contains_key(&id) {
                ui.names.insert(id.clone(), minecraft_terrain::item_icons::item_name(language, &id));
            }
            if self.cells.contains_key(&id) || self.failed.contains(&id) || self.animated.cells.contains_key(&id) {
                continue;
            }
            let cell = self.cells.len() as u32;
            if cell >= ICON_COLUMNS * max_rows {
                diag::warn!(World, "Minecraft item icon atlas full: no icon for `{id}`");
                self.failed.insert(id);
                continue;
            }
            let clock = minecraft_terrain::item_icon::SpriteClock::at(animation_tick);
            match minecraft_terrain::item_icons::item_icon_at(packs, &id, icon as usize, &clock) {
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

fn stack_limit(recipes: &minecraftoss_player::crafting::RecipeBook, stack: &ItemStack) -> u8 {
    if stack.components.is_none() {
        stack.max.min(recipes.max_stack(&stack.id))
    } else {
        stack.max
    }
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
    let gun = weapon_of(carried).is_some();
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

fn throw_slot(inventory: &mut Inventory, index: usize, whole: bool) -> Option<ItemStack> {
    let slot = inventory.slots.get_mut(index)?;
    let stack = slot.as_mut()?;
    let count = if whole { stack.count } else { 1 };
    let thrown = ItemStack { count, ..stack.clone() };
    stack.count -= count;
    if stack.count == 0 {
        *slot = None;
    }
    Some(thrown)
}

fn published(ui: &MinecraftUi, slot: McSlot) -> Option<&McStack> {
    match slot {
        McSlot::Inventory(i) => ui.slots.get(i)?.as_ref(),
        McSlot::Crafting(i) => ui.crafting.get(i)?.as_ref(),
        McSlot::Result => ui.result.as_ref(),
        McSlot::Workbench(i) => ui.workbench.get(i)?.as_ref(),
        McSlot::WorkbenchResult => ui.workbench_result.as_ref(),
        McSlot::Container(i) => ui.container.get(i)?.as_ref(),
        McSlot::Output => ui.output.as_ref(),
    }
}

fn keeps_gun(ui: &MinecraftUi, click: &McClick) -> bool {
    let gun = |stack: Option<&McStack>| stack.is_some_and(|stack| stack.weapon.is_some());
    match click {
        McClick::Swap { slot, hotbar: OFFHAND } => gun(published(ui, *slot)),
        McClick::Swap { slot: McSlot::Inventory(OFFHAND), hotbar } => gun(ui.slots.get(*hotbar).and_then(Option::as_ref)),
        McClick::Slot { slot: McSlot::Inventory(OFFHAND), shift: false, .. } => gun(ui.cursor.as_ref()),
        McClick::Throw { slot, .. } => gun(published(ui, *slot)),
        _ => false,
    }
}

fn station_title(opened: &Opened) -> &'static str {
    match opened.station {
        Station::Chest if opened.cells.len() > 1 => "container.chestDouble",
        Station::Chest => "container.chest",
        Station::Barrel => "container.barrel",
        Station::Oven(CookingKind::Furnace) => "container.furnace",
        Station::Oven(CookingKind::Smoker) => "container.smoker",
        Station::Oven(CookingKind::BlastFurnace) => "container.blast_furnace",
        Station::Brewer => "container.brewing",
        Station::Ender => "container.enderchest",
        Station::Shulker => "container.shulkerBox",
    }
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

fn bench_title(kind: Workshop) -> &'static str {
    match kind {
        Workshop::Stonecutter => "container.stonecutter",
        Workshop::Smithing => "container.upgrade",
        Workshop::Grindstone => "container.grindstone_title",
        Workshop::Anvil => "container.repair",
        Workshop::Enchanting => "container.enchant",
    }
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
    let potion = minecraft_terrain::stations::potion_of(stack)?;
    let effect = potion.rsplit(':').next()?.trim_start_matches("long_").trim_start_matches("strong_");
    let item = stack.id.rsplit(':').next()?;
    language.get(&format!("item.minecraft.{item}.effect.{effect}")).cloned()
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

fn spot(slot: McSlot, grid: Grid) -> Option<Spot> {
    match slot {
        McSlot::Inventory(index) => Some(Spot::Player(index)),
        McSlot::Crafting(cell) if grid == Grid::Inventory => Some(Spot::Grid(cell)),
        McSlot::Result if grid == Grid::Inventory => Some(Spot::Result),
        McSlot::Workbench(cell) if grid == Grid::Table => Some(Spot::Grid(cell)),
        McSlot::WorkbenchResult if grid == Grid::Table => Some(Spot::Result),
        _ => None,
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
}
