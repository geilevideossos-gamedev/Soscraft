use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, PoisonError};

use minecraft_commands::Text;
use minecraft_net::messages::{
    Click, CreativeTarget, MenuAction, MenuExtras, MenuKind, MenuSlot, MenuView, Nbt, NetOffer,
};
use minecraft_net::{Dimension, PlayerUuid, ToClient};
use minecraft_terrain::containers::{Containers, Opened, Station};
use minecraft_terrain::crafting::{self, Grid, Spot};
use minecraft_terrain::mount_inventory::MountInventory;
use minecraft_terrain::gun_mods::GunRules;
use minecraft_terrain::maps::{LootExplorer, SEARCH_TIME};
use minecraft_terrain::openers::RECHECK_TICKS;
use minecraft_terrain::scene::{BlockPos, HandcraftedScene, Scene};
use minecraft_terrain::server::{MerchantOp, MerchantView};
use minecraft_terrain::stations::{Bench, Context, StationBook, Taken, Workshop};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::enchant::Enchantments;
use minecraftoss_entities::trading::{ItemCost, MerchantOffer, TradeItem};
use minecraftoss_entities::villager::Profession;
use minecraftoss_player::crafting::{CookingKind, RecipeBook};
use minecraftoss_player::inventory::{Inventory, ItemStack};

use crate::actions::{Body, Effect};
use crate::creative::{CreativeSlot, set_slot};
use crate::data::GameData;
use crate::entities::{Entities, MerchantNews};
use crate::gun_crafting::Place;
use crate::guns::{HOTBAR, fits, is_gun, weapon_of};
use crate::level::Terrain;
use crate::offhand::OFFHAND;
use crate::player::Player;
use crate::server::Globals;
use crate::views::{slot_nbt, view_stack};

pub const INVENTORY_MENU: u8 = 0;
pub const MENU_IDS: u8 = 100;
pub const STATE_MASK: u16 = 0x7fff;
pub const PLAYER_SLOTS: usize = 45;
pub const MENU_INPUTS: usize = 256;
pub const RECIPE_ID_LIMIT: usize = 256;

#[derive(Clone, Debug)]
pub enum Open {
    Inventory,
    Table(BlockPos),
    Block(Opened),
    Bench(BlockPos, Bench),
    Merchant(MerchantScreen),
    Mount(MountInventory),
}

#[derive(Clone, Debug)]
pub struct MerchantScreen {
    pub view: MerchantView,
    pub selected: Option<usize>,
    pub held: VecDeque<MenuAction>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MerchantShown {
    pub opened: bool,
    pub closed: bool,
    pub thrown: Vec<ItemStack>,
}

#[derive(Clone, Debug, PartialEq)]
struct Sent {
    slots: Vec<Option<Nbt>>,
    carried: Option<Nbt>,
    data: Vec<(u8, i32)>,
    extras: MenuExtras,
}

#[derive(Clone, Debug)]
pub struct Menu {
    pub id: u8,
    pub state: u16,
    pub dimension: Dimension,
    pub open: Open,
    pub title: Text,
    pub shelves: i32,
    resend: bool,
    sent: Option<Sent>,
}

impl Menu {
    pub fn new(id: u8, dimension: Dimension, open: Open, title: Text, shelves: i32) -> Self {
        Self {
            id,
            state: 0,
            dimension,
            open,
            title,
            shelves,
            resend: false,
            sent: None,
        }
    }

    pub fn block(&self) -> Option<&Opened> {
        match &self.open {
            Open::Block(opened) => Some(opened),
            _ => None,
        }
    }

    pub fn resends(&self) -> bool {
        self.resend
    }
}

#[derive(Clone, Debug)]
pub struct Menus {
    pub open: Option<Menu>,
    pub inventory: Menu,
    counter: u8,
    told: Option<u8>,
    merchant_ops: Vec<MerchantOp>,
    merchant_waiting: bool,
    mount_updates: Vec<MountInventory>,
    mount_key: bool,
}

impl Default for Menus {
    fn default() -> Self {
        Self {
            open: None,
            inventory: Menu::new(
                INVENTORY_MENU,
                Dimension::Overworld,
                Open::Inventory,
                Text::translate("container.crafting", Vec::new()),
                0,
            ),
            counter: 0,
            told: None,
            merchant_ops: Vec::new(),
            merchant_waiting: false,
            mount_updates: Vec::new(),
            mount_key: false,
        }
    }
}

impl Menus {
    pub fn current(&self) -> &Menu {
        self.open.as_ref().unwrap_or(&self.inventory)
    }

    fn current_mut(&mut self) -> &mut Menu {
        self.open.as_mut().unwrap_or(&mut self.inventory)
    }

    pub fn block(&self) -> Option<&Opened> {
        self.open.as_ref().and_then(Menu::block)
    }

    fn next_id(&mut self) -> u8 {
        self.counter = self.counter % MENU_IDS + 1;
        self.counter
    }

    fn merchant_busy(&self) -> bool {
        self.merchant_waiting || !self.merchant_ops.is_empty()
    }
}

#[derive(Clone, Copy)]
pub struct Books<'a> {
    pub stations: &'a StationBook,
    pub enchantments: Option<&'a Enchantments>,
    pub guns: Option<&'a dyn GunRules>,
}

impl<'a> Books<'a> {
    pub fn of(data: &'a GameData) -> Self {
        Self {
            stations: &data.stations,
            enchantments: data.enchantments.as_deref(),
            guns: None,
        }
    }

    pub fn with_guns(self, guns: Option<&'a dyn GunRules>) -> Self {
        Self { guns, ..self }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clicked {
    pub thrown: Vec<ItemStack>,
    pub experience: u32,
    pub used: Vec<(&'static str, BlockPos)>,
    pub anvil: Option<BlockPos>,
}

impl Clicked {
    pub fn add(&mut self, other: Clicked) {
        self.thrown.extend(other.thrown);
        self.experience += other.experience;
        self.used.extend(other.used);
        self.anvil = other.anvil.or(self.anvil);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuInput {
    Click(Click),
    Rename(String),
    Close(u8),
    OpenMount,
    Creative {
        slot: CreativeTarget,
        stack: Option<Nbt>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct Screen {
    pub open: Option<Open>,
    pub cells: Vec<Option<ItemStack>>,
    pub data: [i32; 4],
    pub seed: i32,
    pub shelves: i32,
}

fn context<'a>(
    recipes: &'a RecipeBook,
    books: Books<'a>,
    shelves: i32,
    seed: i32,
    level: u32,
) -> Context<'a> {
    Context {
        recipes,
        book: books.stations,
        enchantments: books.enchantments,
        bookshelves: shelves,
        seed,
        level,
        guns: books.guns,
    }
}

pub fn station_title(opened: &Opened) -> &'static str {
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

pub fn bench_title(kind: Workshop) -> &'static str {
    match kind {
        Workshop::Stonecutter => "container.stonecutter",
        Workshop::Smithing => "container.upgrade",
        Workshop::Grindstone => "container.grindstone_title",
        Workshop::Anvil => "container.repair",
        Workshop::Enchanting => "container.enchant",
    }
}

pub fn open(player: &mut Player, open: Open, title: Text, shelves: i32) -> bool {
    if player.menus.open.is_some() {
        return false;
    }
    let id = player.menus.next_id();
    player.menus.open = Some(Menu::new(id, player.dimension, open, title, shelves));
    true
}

#[allow(clippy::too_many_arguments)]
pub fn open_station(
    player: &mut Player,
    containers: &mut Containers,
    terrain: &Terrain,
    globals: &Globals,
    recipes: &RecipeBook,
    table: Option<BlockPos>,
    block: Option<Opened>,
    bench: Option<(BlockPos, Workshop)>,
) -> bool {
    if let Some(pos) = table {
        return open(
            player,
            Open::Table(pos),
            Text::translate("container.crafting", Vec::new()),
            0,
        );
    }
    if let Some(opened) = block {
        let storage = matches!(opened.station, Station::Chest | Station::Barrel);
        for &pos in opened.cells.iter().filter(|_| storage) {
            let explorer = containers
                .is_new(pos)
                .then(|| loot_explorer(terrain, globals, player.dimension));
            if let Some(explorer) = &explorer
                && let Some(chunk) = terrain
                    .scene
                    .generated_chunk((pos.0.div_euclid(16), pos.2.div_euclid(16)))
                && let Some(entity) = chunk
                    .block_entities
                    .entities
                    .get(&pos)
                    .or_else(|| chunk.block_entities.pending.get(&pos))
                && let Some((items, seed)) = minecraft_terrain::containers::structure_loot_in(
                    &globals.registries,
                    entity,
                    pos,
                    globals.seed,
                    recipes,
                    globals.data.enchantments.clone(),
                    Some(terrain.stream.states.biome_name(pos, |at| {
                        terrain.scene.generated_chunk(at).map(|c| &**c)
                    })),
                    Some(explorer.clone()),
                )
            {
                keep_explored(globals, explorer);
                containers.fill(pos, items, seed);
            }
        }
        let title = containers.custom_name(opened.cells[0]).map_or_else(
            || Text::translate(station_title(&opened), Vec::new()),
            Text::literal,
        );
        return open(player, Open::Block(opened), title, 0);
    }
    let Some((pos, kind)) = bench else {
        return false;
    };
    let shelves = if kind == Workshop::Enchanting {
        let block_at = |at: BlockPos| Scene::block(&terrain.scene, at);
        minecraft_terrain::stations::bookshelves(
            pos,
            |at| block_at(at).is_some_and(|b| b.id.key() == "minecraft:bookshelf"),
            |at| {
                block_at(at).is_none_or(|b| {
                    terrain.stream.states.state_of(b).is_some_and(|state| {
                        globals
                            .registries
                            .blocks
                            .is(state, minecraftoss_core::block::flags::REPLACEABLE)
                    })
                })
            },
        )
    } else {
        0
    };
    open(
        player,
        Open::Bench(pos, Bench::new(kind)),
        Text::translate(bench_title(kind), Vec::new()),
        shelves,
    )
}

fn loot_explorer(terrain: &Terrain, globals: &Globals, dimension: Dimension) -> Arc<LootExplorer> {
    let first_id = globals
        .maps
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .next_id();
    Arc::new(LootExplorer::new(
        terrain.stream.world_gen(),
        dimension.dimension_type(),
        first_id,
    ))
}

fn keep_explored(globals: &Globals, explorer: &LootExplorer) {
    let explored = explorer.take();
    for destination in &explored.timed_out {
        diag::warn!(
            World,
            "Minecraft explorer map search for {destination} stopped after {} s",
            SEARCH_TIME.as_secs()
        );
    }
    let mut maps = globals.maps.lock().unwrap_or_else(PoisonError::into_inner);
    for (id, map) in explored.maps {
        maps.insert(id, map);
    }
}

pub fn still_valid(menu: &Menu, scene: &HandcraftedScene, eye: [f64; 3]) -> bool {
    let near = |pos: BlockPos| crafting::within_table_reach(eye, pos);
    match &menu.open {
        Open::Inventory => true,
        Open::Table(pos) => {
            near(*pos) && Scene::block(scene, *pos).is_some_and(crafting::is_crafting_table)
        }
        Open::Block(opened) => opened.cells.iter().all(|&pos| {
            near(pos) && Scene::block(scene, pos).and_then(Station::of) == Some(opened.station)
        }),
        Open::Bench(pos, bench) => {
            near(*pos) && Scene::block(scene, *pos).and_then(Workshop::of) == Some(bench.kind)
        }
        Open::Merchant(_) | Open::Mount(_) => true,
    }
}

fn slot_index(slot: MenuSlot) -> Option<usize> {
    match slot {
        MenuSlot::Container(index) => Some(usize::from(index)),
        MenuSlot::Output => Some(2),
        _ => None,
    }
}

fn spot(slot: MenuSlot, grid: Grid) -> Option<Spot> {
    match slot {
        MenuSlot::Inventory(index) => Some(Spot::Player(usize::from(index))),
        MenuSlot::Crafting(cell) if grid == Grid::Inventory => Some(Spot::Grid(usize::from(cell))),
        MenuSlot::Result if grid == Grid::Inventory => Some(Spot::Result),
        MenuSlot::Workbench(cell) if grid == Grid::Table => Some(Spot::Grid(usize::from(cell))),
        MenuSlot::WorkbenchResult if grid == Grid::Table => Some(Spot::Result),
        _ => None,
    }
}

fn player_slots(slots: &[MenuSlot]) -> Vec<usize> {
    slots
        .iter()
        .filter_map(|slot| match slot {
            MenuSlot::Inventory(index) => Some(usize::from(*index)),
            _ => None,
        })
        .collect()
}

fn throw_slot(inventory: &mut Inventory, index: usize, whole: bool) -> Option<ItemStack> {
    let slot = inventory.slots.get_mut(index)?;
    let stack = slot.as_mut()?;
    let count = if whole { stack.count } else { 1 };
    let thrown = ItemStack {
        count,
        ..stack.clone()
    };
    stack.count -= count;
    if stack.count == 0 {
        *slot = None;
    }
    Some(thrown)
}

pub fn keep_guns(
    inventory: &mut Inventory,
    selected: usize,
    thrown: Vec<ItemStack>,
) -> Vec<ItemStack> {
    let mut out = Vec::new();
    for stack in thrown {
        if weapon_of(&stack).is_some() {
            if let Some(back) = inventory.add_item(stack, selected) {
                inventory.cursor = Some(back);
            }
        } else {
            out.push(stack);
        }
    }
    out
}

fn stack_at(
    menu: Option<&Menu>,
    inventory: &Inventory,
    containers: &mut Containers,
    slot: MenuSlot,
) -> Option<ItemStack> {
    match slot {
        MenuSlot::Inventory(index) => inventory.slots.get(usize::from(index))?.clone(),
        MenuSlot::Crafting(index) => inventory.crafting.get(usize::from(index))?.clone(),
        MenuSlot::Workbench(index) => inventory.workbench.get(usize::from(index))?.clone(),
        MenuSlot::Container(index) => match &menu?.open {
            Open::Block(opened) => containers.cells(opened).get(usize::from(index))?.clone(),
            Open::Bench(_, bench) => bench.cells.get(usize::from(index))?.clone(),
            Open::Mount(mount) => mount.cells.get(usize::from(index))?.clone(),
            _ => None,
        },
        MenuSlot::Result | MenuSlot::WorkbenchResult | MenuSlot::Output => None,
    }
}

fn keeps_gun(
    menu: Option<&Menu>,
    inventory: &Inventory,
    containers: &mut Containers,
    action: &MenuAction,
) -> bool {
    let offhand = MenuSlot::Inventory(OFFHAND as u8);
    let gun = |stack: Option<ItemStack>| stack.is_some_and(|stack| weapon_of(&stack).is_some());
    let any_gun = |stack: Option<ItemStack>| stack.is_some_and(|stack| is_gun(&stack));
    match action {
        MenuAction::Swap { slot, hotbar } if usize::from(*hotbar) == OFFHAND => {
            any_gun(stack_at(menu, inventory, containers, *slot))
        }
        MenuAction::Swap { slot, hotbar } if *slot == offhand => {
            any_gun(inventory.slots.get(usize::from(*hotbar)).cloned().flatten())
        }
        MenuAction::Slot {
            slot, shift: false, ..
        } if *slot == offhand => any_gun(inventory.cursor.clone()),
        MenuAction::Spread { slots, .. }
            if slots.contains(&offhand) && any_gun(inventory.cursor.clone()) =>
        {
            true
        }
        MenuAction::Throw { slot, .. } => gun(stack_at(menu, inventory, containers, *slot)),
        MenuAction::Slot {
            slot: MenuSlot::Container(_),
            shift: false,
            ..
        }
        | MenuAction::Outside { .. } => gun(inventory.cursor.clone()),
        MenuAction::Slot {
            slot: slot @ MenuSlot::Inventory(_),
            shift: true,
            ..
        } if menu.is_some_and(|menu| matches!(menu.open, Open::Block(_) | Open::Bench(..) | Open::Mount(_))) => {
            gun(stack_at(menu, inventory, containers, *slot))
        }
        MenuAction::Swap {
            slot: MenuSlot::Container(_),
            hotbar,
        } => gun(inventory.slots.get(usize::from(*hotbar)).cloned().flatten()),
        MenuAction::Spread { slots, .. } => {
            gun(inventory.cursor.clone())
                && slots
                    .iter()
                    .any(|slot| matches!(slot, MenuSlot::Container(_)))
        }
        _ => false,
    }
}

fn misfits(inventory: &Inventory, action: &MenuAction) -> bool {
    let misfit =
        |slot: usize, stack: Option<&ItemStack>| stack.is_some_and(|stack| !fits(slot, stack));
    let held = |slot: usize| inventory.slots.get(slot).and_then(Option::as_ref);
    match action {
        MenuAction::Slot {
            slot: MenuSlot::Inventory(index),
            shift: false,
            ..
        } => misfit(usize::from(*index), inventory.cursor.as_ref()),
        MenuAction::Swap {
            slot: MenuSlot::Inventory(index),
            hotbar,
        } => {
            let (index, hotbar) = (usize::from(*index), usize::from(*hotbar));
            misfit(index, held(hotbar)) || misfit(hotbar, held(index))
        }
        _ => false,
    }
}

fn fitted_spread(inventory: &Inventory, action: &MenuAction) -> Option<MenuAction> {
    let MenuAction::Spread { slots, right } = action else {
        return None;
    };
    let carried = inventory.cursor.as_ref()?;
    let fitting = |slot: &MenuSlot| match slot {
        MenuSlot::Inventory(index) => fits(usize::from(*index), carried),
        _ => true,
    };
    let kept: Vec<MenuSlot> = slots.iter().copied().filter(fitting).collect();
    (kept.len() < slots.len()).then_some(MenuAction::Spread {
        slots: kept,
        right: *right,
    })
}

fn book_grid(menu: Option<&Menu>) -> Option<bool> {
    match menu.map(|menu| &menu.open) {
        None | Some(Open::Inventory) => Some(false),
        Some(Open::Table(_)) => Some(true),
        _ => None,
    }
}

fn valid_slot(menu: Option<&Menu>, containers: &mut Containers, slot: MenuSlot) -> bool {
    let size = match menu.map(|menu| &menu.open) {
        Some(Open::Block(opened)) => containers.cells(opened).len(),
        Some(Open::Bench(_, bench)) => bench.cells.len(),
        Some(Open::Merchant(_)) => 3,
        Some(Open::Mount(mount)) => mount.cells.len(),
        _ => 0,
    };
    let table = matches!(menu.map(|menu| &menu.open), Some(Open::Table(_)));
    match slot {
        MenuSlot::Inventory(index) => usize::from(index) < PLAYER_SLOTS,
        MenuSlot::Crafting(index) => !table && usize::from(index) < 4,
        MenuSlot::Result => !table,
        MenuSlot::Workbench(index) => table && usize::from(index) < 9,
        MenuSlot::WorkbenchResult => table,
        MenuSlot::Container(index) => usize::from(index) < size,
        MenuSlot::Output => size > 0,
    }
}

fn valid(menu: Option<&Menu>, containers: &mut Containers, action: &MenuAction) -> bool {
    let mut slot = |slot: MenuSlot| valid_slot(menu, containers, slot);
    match action {
        MenuAction::Slot { slot: at, .. }
        | MenuAction::Gather { slot: at, .. }
        | MenuAction::Throw { slot: at, .. } => slot(*at),
        MenuAction::Swap { slot: at, hotbar } => {
            (usize::from(*hotbar) < HOTBAR || usize::from(*hotbar) == OFFHAND) && slot(*at)
        }
        MenuAction::Spread { slots, .. } => {
            slots.len() <= PLAYER_SLOTS + 64 && slots.iter().all(|at| slot(*at))
        }
        MenuAction::Outside { .. } | MenuAction::Choose(_) => true,
        MenuAction::PlaceRecipe { recipe, .. } => {
            recipe.len() <= RECIPE_ID_LIMIT && book_grid(menu).is_some()
        }
    }
}

fn bench_action(
    bench: &mut Bench,
    action: &MenuAction,
    inventory: &mut Inventory,
    ctx: &Context<'_>,
    roll: f32,
    thrown: &mut Vec<ItemStack>,
) -> (bool, Option<Taken>) {
    let mut taken: Option<Taken> = None;
    let random = || minecraftoss_player::rng::LegacyRandom::new((roll * u32::MAX as f32) as u64);
    match action {
        MenuAction::Throw {
            slot: MenuSlot::Container(index),
            whole,
        } => {
            if inventory.cursor.is_none() {
                thrown.extend(bench.throw(usize::from(*index), *whole));
            }
        }
        MenuAction::Throw {
            slot: MenuSlot::Output,
            whole,
        } => {
            let first = bench.offer(ctx).result;
            while inventory.cursor.is_none() {
                let Some(step) = bench.take(false, inventory, ctx, &mut random()) else {
                    break;
                };
                thrown.extend(inventory.cursor.take());
                let total = taken.get_or_insert_with(Taken::default);
                total.levels += step.levels;
                total.experience += step.experience;
                total.sound = step.sound;
                total.new_seed |= step.new_seed;
                let next = bench.offer(ctx).result;
                if !*whole
                    || next
                        .as_ref()
                        .zip(first.as_ref())
                        .is_none_or(|(next, first)| !next.same_item(first))
                {
                    break;
                }
            }
        }
        MenuAction::Gather { .. } => bench.gather(inventory, false),
        MenuAction::Slot {
            slot: MenuSlot::Inventory(index),
            shift: true,
            ..
        } => bench.quick_move_from_player(usize::from(*index), inventory, ctx),
        MenuAction::Slot {
            slot: MenuSlot::Container(index),
            right,
            shift,
        } => bench.click(usize::from(*index), *right, *shift, inventory, ctx),
        MenuAction::Slot {
            slot: MenuSlot::Output,
            shift,
            ..
        } => taken = bench.take(*shift, inventory, ctx, &mut random()),
        MenuAction::Choose(index) => taken = bench.choose(usize::from(*index), inventory, ctx),
        MenuAction::Swap {
            slot: MenuSlot::Container(index),
            hotbar,
        } => bench.swap(usize::from(*index), usize::from(*hotbar), inventory, ctx),
        MenuAction::Spread { slots, right } => {
            let players = player_slots(slots);
            let cells: Vec<usize> = slots
                .iter()
                .filter_map(|slot| match slot {
                    MenuSlot::Container(index) => Some(usize::from(*index)),
                    _ => None,
                })
                .collect();
            bench.spread(&players, &cells, *right, inventory, ctx);
        }
        _ => return (false, taken),
    }
    (true, taken)
}

fn block_action(
    opened: &Opened,
    action: &MenuAction,
    inventory: &mut Inventory,
    containers: &mut Containers,
    roll: f32,
    clicked: &mut Clicked,
) -> bool {
    match action {
        MenuAction::Slot {
            slot: MenuSlot::Inventory(index),
            shift: true,
            ..
        } => {
            containers.quick_move_from_player(opened, inventory, usize::from(*index));
        }
        MenuAction::Slot { slot, right, shift } if slot_index(*slot).is_some() => {
            let index = slot_index(*slot).unwrap_or(0);
            clicked.experience += containers
                .click(opened, inventory, index, *right, *shift, roll)
                .experience;
        }
        MenuAction::Gather { right, .. } => containers.gather(opened, inventory, *right),
        MenuAction::Swap { slot, hotbar } if slot_index(*slot).is_some() => {
            let index = slot_index(*slot).unwrap_or(0);
            clicked.experience += containers
                .swap(opened, inventory, index, usize::from(*hotbar), roll)
                .experience;
        }
        MenuAction::Throw { slot, whole } if slot_index(*slot).is_some() => {
            if inventory.cursor.is_none() {
                let index = slot_index(*slot).unwrap_or(0);
                let (stack, changed) = containers.throw(opened, inventory, index, *whole, roll);
                clicked.thrown.extend(stack);
                clicked.experience += changed.experience;
            }
        }
        MenuAction::Spread { slots, right } => {
            let players = player_slots(slots);
            let cells: Vec<usize> = slots.iter().filter_map(|slot| slot_index(*slot)).collect();
            containers.spread(opened, inventory, &players, &cells, *right);
        }
        _ => return false,
    }
    true
}

fn grid_action(
    action: &MenuAction,
    inventory: &mut Inventory,
    grid: Grid,
    guns: Option<&dyn GunRules>,
    thrown: &mut Vec<ItemStack>,
) {
    let special = |inventory: &mut Inventory, place| {
        guns.and_then(|rules| crate::gun_crafting::take(inventory, grid, rules, place))
    };
    match action {
        MenuAction::Slot {
            slot: MenuSlot::Inventory(index),
            right,
            shift,
        } => {
            let index = usize::from(*index);
            if *shift && grid == Grid::Table {
                crafting::quick_move_into_table(inventory, index);
            } else if !(*shift && crate::guns::grenade_quick_move(inventory, index))
                && let Some(stack) = inventory.click(Some(index), *right, *shift)
            {
                thrown.push(stack);
            }
        }
        MenuAction::Slot { slot, right, shift } => match spot(*slot, grid) {
            Some(Spot::Grid(cell)) => crafting::click_cell(inventory, grid, cell, *right, *shift),
            Some(Spot::Result) => {
                let place = if *shift { Place::Inventory } else { Place::Cursor };
                match special(inventory, place) {
                    Some(rest) => thrown.extend(rest),
                    None => {
                        crafting::click_result(inventory, grid, *shift);
                    }
                }
            }
            _ => {}
        },
        MenuAction::Outside { right } => {
            if let Some(stack) = inventory.click(None, *right, false) {
                thrown.push(stack);
            }
        }
        MenuAction::Gather { slot, right } => {
            if let Some(target) = spot(*slot, grid) {
                crafting::gather(inventory, grid, target, *right);
            }
        }
        MenuAction::Swap {
            slot: MenuSlot::Inventory(index),
            hotbar,
        } => inventory.number_swap(usize::from(*index), usize::from(*hotbar)),
        MenuAction::Swap { slot, hotbar } => {
            let hotbar = usize::from(*hotbar);
            match spot(*slot, grid) {
                Some(Spot::Grid(cell)) => crafting::swap(inventory, grid, Some(cell), hotbar),
                Some(Spot::Result) => match special(inventory, Place::Hotbar(hotbar)) {
                    Some(rest) => {
                        thrown.extend(rest);
                        true
                    }
                    None => crafting::swap(inventory, grid, None, hotbar),
                },
                _ => false,
            };
        }
        MenuAction::Spread { slots, right } => {
            let spots: Vec<Spot> = slots.iter().filter_map(|slot| spot(*slot, grid)).collect();
            crafting::spread(inventory, grid, &spots, *right);
        }
        MenuAction::Throw { slot, whole } => {
            if inventory.cursor.is_some() {
                return;
            }
            match spot(*slot, grid) {
                Some(Spot::Player(index)) => thrown.extend(throw_slot(inventory, index, *whole)),
                Some(Spot::Grid(cell)) => {
                    thrown.extend(crafting::throw_cell(inventory, grid, cell, *whole))
                }
                Some(Spot::Result) => match special(inventory, Place::Ground) {
                    Some(rest) => thrown.extend(rest),
                    None => thrown.extend(crafting::throw_result(inventory, grid, *whole)),
                },
                None => {}
            }
        }
        MenuAction::Choose(_) | MenuAction::PlaceRecipe { .. } => {}
    }
}

fn merchant_op(action: &MenuAction) -> Option<MerchantOp> {
    Some(match action {
        MenuAction::Slot {
            slot: MenuSlot::Container(index @ (0 | 1)),
            right,
            shift,
        } => MerchantOp::Payment {
            slot: usize::from(*index),
            right: *right,
            shift: *shift,
        },
        MenuAction::Slot {
            slot: MenuSlot::Container(2) | MenuSlot::Output,
            shift,
            ..
        } => MerchantOp::Result { shift: *shift },
        MenuAction::Slot {
            slot: MenuSlot::Inventory(index),
            shift: true,
            ..
        } => MerchantOp::QuickMove(usize::from(*index)),
        MenuAction::Choose(index) => MerchantOp::Select(i32::from(*index)),
        _ => return None,
    })
}

fn merchant_local(action: &MenuAction) -> bool {
    let player = |slot: &MenuSlot| matches!(slot, MenuSlot::Inventory(_));
    match action {
        MenuAction::Slot { slot, .. }
        | MenuAction::Gather { slot, .. }
        | MenuAction::Swap { slot, .. }
        | MenuAction::Throw { slot, .. } => player(slot),
        MenuAction::Spread { slots, .. } => slots.iter().all(player),
        MenuAction::Outside { .. } => true,
        MenuAction::Choose(_) | MenuAction::PlaceRecipe { .. } => false,
    }
}

fn mount_action(
    mount: &mut MountInventory,
    inventory: &mut Inventory,
    action: &MenuAction,
    thrown: &mut Vec<ItemStack>,
) {
    match action {
        MenuAction::Slot {
            slot: MenuSlot::Inventory(index),
            shift: true,
            ..
        } => mount.quick_move_from_player(inventory, usize::from(*index)),
        MenuAction::Slot {
            slot: MenuSlot::Container(index),
            right,
            shift,
        } => mount.click(inventory, usize::from(*index), *right, *shift),
        MenuAction::Swap {
            slot: MenuSlot::Container(index),
            hotbar,
        } => mount.swap(inventory, usize::from(*index), usize::from(*hotbar)),
        MenuAction::Throw {
            slot: MenuSlot::Container(index),
            whole,
        } => {
            if inventory.cursor.is_none() {
                thrown.extend(mount.throw(usize::from(*index), *whole));
            }
        }
        _ if merchant_local(action) => grid_action(action, inventory, Grid::Inventory, None, thrown),
        _ => {}
    }
}

fn merchant_action(
    screen: &mut MerchantScreen,
    ops: &mut Vec<MerchantOp>,
    inventory: &mut Inventory,
    action: &MenuAction,
    busy: bool,
    thrown: &mut Vec<ItemStack>,
) {
    if busy {
        screen.held.push_back(action.clone());
        return;
    }
    if let Some(op) = merchant_op(action) {
        if let MerchantOp::Select(index) = op {
            screen.selected = usize::try_from(index).ok();
        }
        ops.push(op);
    } else if merchant_local(action) {
        grid_action(action, inventory, Grid::Inventory, None, thrown);
    }
}

pub fn apply(
    player: &mut Player,
    containers: &mut Containers,
    books: Books<'_>,
    action: &MenuAction,
    roll: f32,
) -> Clicked {
    let mut clicked = Clicked::default();
    let Player {
        menus,
        inventory,
        selected,
        survival,
        ender,
        enchantment_seed,
        ..
    } = player;
    let fitted = fitted_spread(inventory, action);
    let action = fitted.as_ref().unwrap_or(action);
    containers.with_ender(ender, |containers| {
        if keeps_gun(menus.open.as_ref(), inventory, containers, action)
            || misfits(inventory, action)
        {
            return;
        }
        if let MenuAction::PlaceRecipe { recipe, all } = action {
            if let Some(table) = book_grid(menus.open.as_ref()) {
                crate::recipe_book::place(inventory, table, recipe, *all);
            }
            return;
        }
        if let Some(Menu {
            open: Open::Merchant(screen),
            ..
        }) = menus.open.as_mut()
        {
            let busy =
                menus.merchant_waiting || !menus.merchant_ops.is_empty() || !screen.held.is_empty();
            merchant_action(
                screen,
                &mut menus.merchant_ops,
                inventory,
                action,
                busy,
                &mut clicked.thrown,
            );
            return;
        }
        if let Some(Menu {
            open: Open::Mount(mount),
            ..
        }) = menus.open.as_mut()
        {
            let before = mount.cells.clone();
            mount_action(mount, inventory, action, &mut clicked.thrown);
            if mount.cells != before {
                menus.mount_updates.push(mount.clone());
            }
            return;
        }
        let grid = match menus.open.as_ref().map(|menu| &menu.open) {
            Some(Open::Table(_)) => Grid::Table,
            _ => Grid::Inventory,
        };
        if let Some(Menu {
            open: Open::Bench(pos, bench),
            shelves,
            ..
        }) = menus.open.as_mut()
        {
            let recipes = inventory.recipes.clone();
            let level = survival.status.experience_level;
            let ctx = context(&recipes, books, *shelves, *enchantment_seed, level);
            let (handled, taken) =
                bench_action(bench, action, inventory, &ctx, roll, &mut clicked.thrown);
            if let Some(taken) = taken {
                survival.spend_levels(taken.levels);
                clicked.experience += taken.experience;
                if !taken.sound.is_empty() && bench.kind != Workshop::Anvil {
                    clicked.used.push((taken.sound, *pos));
                }
                if taken.new_seed {
                    *enchantment_seed = (roll * i32::MAX as f32) as i32;
                }
                if bench.kind == Workshop::Anvil {
                    clicked.anvil = Some(*pos);
                }
            }
            if handled {
                return;
            }
        }
        if let Some(opened) = menus.open.as_ref().and_then(Menu::block).cloned()
            && block_action(&opened, action, inventory, containers, roll, &mut clicked)
        {
            return;
        }
        grid_action(action, inventory, grid, books.guns, &mut clicked.thrown);
    });
    let mut thrown = std::mem::take(&mut clicked.thrown);
    thrown.extend(inventory.take_pending_drops());
    let _ = inventory.take_stat_events();
    clicked.thrown = keep_guns(inventory, *selected, thrown);
    clicked
}

pub fn click(
    player: &mut Player,
    containers: &mut Containers,
    books: Books<'_>,
    click: &Click,
    roll: f32,
) -> Clicked {
    let ignored = player.dead
        || player.mode.mode == minecraftoss_player::GameMode::Spectator
        || click.menu != player.menus.current().id
        || player
            .menus
            .open
            .as_ref()
            .is_some_and(|menu| menu.dimension != player.dimension);
    if ignored {
        if click.menu == player.menus.current().id {
            player.menus.current_mut().resend = true;
        }
        return Clicked::default();
    }
    let ok = {
        let Player { menus, ender, .. } = &mut *player;
        containers.with_ender(ender, |containers| {
            valid(menus.open.as_ref(), containers, &click.action)
        })
    };
    let stale = click.state != player.menus.current().state;
    let clicked = if ok {
        apply(player, containers, books, &click.action, roll)
    } else {
        Clicked::default()
    };
    if stale || !ok {
        player.menus.current_mut().resend = true;
    }
    clicked
}

pub fn rename(player: &mut Player, name: &str, language: Option<&HashMap<String, String>>) {
    let Some(Menu {
        open: Open::Bench(_, bench),
        ..
    }) = player.menus.open.as_mut()
    else {
        return;
    };
    if bench.kind != Workshop::Anvil {
        return;
    }
    let default = bench.cells[0].as_ref().map(|stack| {
        language.map_or_else(
            || stack.id.clone(),
            |language| minecraft_terrain::item_icons::item_name(language, &stack.id),
        )
    });
    let named = bench.cells[0]
        .as_ref()
        .and_then(minecraft_terrain::stations::custom_name);
    if named.is_none() && default.as_deref() == Some(name) {
        bench.name = None;
    } else {
        bench.rename(name);
    }
}

pub fn close(player: &mut Player) -> Vec<ItemStack> {
    if let Some(Menu {
        open: Open::Merchant(_),
        ..
    }) = player.menus.open
    {
        player.menus.open = None;
        player.menus.merchant_ops.push(MerchantOp::Close);
        return Vec::new();
    }
    let mut thrown = Vec::new();
    if let Some(Menu {
        open: Open::Bench(_, mut bench),
        ..
    }) = player.menus.open.take()
    {
        let selected = player.selected;
        thrown.extend(
            bench
                .close()
                .into_iter()
                .filter_map(|stack| player.inventory.add_item(stack, selected)),
        );
    }
    if crafting::has_loose_items(&player.inventory) && !player.menus.merchant_busy() {
        thrown.extend(crafting::close(&mut player.inventory, player.selected));
    }
    thrown.extend(player.inventory.take_pending_drops());
    keep_guns(&mut player.inventory, player.selected, thrown)
}

pub fn close_requested(player: &mut Player, menu: u8) -> Vec<ItemStack> {
    if menu != player.menus.current().id {
        return Vec::new();
    }
    if player.menus.told == Some(menu) {
        player.menus.told = None;
    }
    close(player)
}

pub fn take_inputs(
    player: &mut Player,
    containers: &mut Containers,
    books: Books<'_>,
    inputs: Vec<MenuInput>,
    language: Option<&HashMap<String, String>>,
    known: &dyn Fn(&str) -> bool,
    mut roll: impl FnMut() -> f32,
) -> Clicked {
    let mut clicked = Clicked::default();
    for input in inputs {
        match input {
            MenuInput::Click(click) => {
                clicked.add(self::click(player, containers, books, &click, roll()));
            }
            MenuInput::Rename(name) => rename(player, &name, language),
            MenuInput::Close(menu) => clicked.thrown.extend(close_requested(player, menu)),
            MenuInput::OpenMount => player.menus.mount_key = true,
            MenuInput::Creative { slot, stack } => {
                clicked
                    .thrown
                    .extend(creative_set(player, slot, stack.as_ref(), known));
            }
        }
    }
    clicked
}

fn merchant_title(view: &MerchantView) -> Text {
    if let Some(name) = view.name.as_ref() {
        return Text::literal(name.clone());
    }
    if view.trader {
        return Text::translate("entity.minecraft.wandering_trader", Vec::new());
    }
    let profession = view
        .profession
        .rsplit(':')
        .next()
        .unwrap_or(view.profession);
    Text::translate(
        format!("entity.minecraft.villager.{profession}"),
        Vec::new(),
    )
}

fn merchant_news(
    player: &mut Player,
    news: Vec<MerchantNews>,
    shows: bool,
    waiting: bool,
) -> MerchantShown {
    let mut shown = MerchantShown::default();
    let dimension = player.dimension;
    let menus = &mut player.menus;
    for news in news {
        let other = menus
            .open
            .as_ref()
            .is_some_and(|menu| !matches!(menu.open, Open::Merchant(_)));
        let screen = match menus.open.as_mut() {
            Some(Menu {
                open: Open::Merchant(screen),
                ..
            }) => Some(screen),
            _ => None,
        };
        match news {
            MerchantNews::Opened(view) => match screen {
                Some(screen) if screen.view.villager == view.villager => screen.view = view,
                None if shows && !other => {
                    let id = menus.next_id();
                    let title = merchant_title(&view);
                    let screen = MerchantScreen {
                        view,
                        selected: None,
                        held: VecDeque::new(),
                    };
                    menus.open = Some(Menu::new(id, dimension, Open::Merchant(screen), title, 0));
                    shown.opened = true;
                }
                _ => menus.merchant_ops.push(MerchantOp::Close),
            },
            MerchantNews::Answer(update) => {
                if let Some(screen) = screen {
                    match update.view {
                        Some(view) => screen.view = view,
                        None => {
                            menus.open = None;
                            shown.closed = true;
                        }
                    }
                }
            }
            MerchantNews::Closing => {
                if screen.is_some() {
                    menus.open = None;
                    menus.merchant_ops.push(MerchantOp::Close);
                    shown.closed = true;
                }
            }
        }
    }
    menus.merchant_waiting = waiting;
    shown
}

fn pump_merchant(
    player: &mut Player,
    mut send: impl FnMut(MerchantOp, &Inventory, usize),
) -> Vec<ItemStack> {
    let Player {
        menus,
        inventory,
        selected,
        ..
    } = player;
    let mut thrown = Vec::new();
    while !menus.merchant_waiting {
        if !menus.merchant_ops.is_empty() {
            let op = menus.merchant_ops.remove(0);
            send(op, inventory, *selected);
            menus.merchant_waiting = true;
            continue;
        }
        let Some(Menu {
            open: Open::Merchant(screen),
            ..
        }) = menus.open.as_mut()
        else {
            break;
        };
        let Some(action) = screen.held.pop_front() else {
            break;
        };
        merchant_action(
            screen,
            &mut menus.merchant_ops,
            inventory,
            &action,
            false,
            &mut thrown,
        );
    }
    thrown.extend(inventory.take_pending_drops());
    keep_guns(inventory, *selected, thrown)
}

pub fn merchant(
    player: &mut Player,
    entities: &mut Entities,
    feet: [f64; 3],
    shows: bool,
    alive: bool,
) -> MerchantShown {
    let id = u64::from(player.id.0);
    let news = entities.take_merchant(id);
    let waiting = entities.awaits_merchant(id);
    let mut shown = merchant_news(player, news, shows && alive, waiting);
    if !alive {
        close_merchant_now(player, entities, feet);
        return shown;
    }
    shown.thrown = pump_merchant(player, |op, inventory, selected| {
        entities.merchant(id, op, inventory, selected, feet);
    });
    shown
}

pub fn close_merchant_now(player: &mut Player, entities: &mut Entities, feet: [f64; 3]) {
    let id = u64::from(player.id.0);
    let open = matches!(
        player.menus.open.as_ref().map(|menu| &menu.open),
        Some(Open::Merchant(_))
    );
    if open {
        player.menus.open = None;
    }
    if !open && !player.menus.merchant_busy() && !entities.awaits_merchant(id) {
        return;
    }
    entities.close_merchant_now(id, &mut player.inventory, player.selected, feet);
    player.menus.merchant_ops.clear();
    player.menus.merchant_waiting = false;
}

fn creative_set(
    player: &mut Player,
    slot: CreativeTarget,
    stack: Option<&Nbt>,
    known: &dyn Fn(&str) -> bool,
) -> Option<ItemStack> {
    if player.dead {
        return None;
    }
    let slot = match slot {
        CreativeTarget::Menu(index) => CreativeSlot::Menu(usize::from(index)),
        CreativeTarget::Carried => CreativeSlot::Carried,
        CreativeTarget::Drop => CreativeSlot::Drop,
    };
    let stack = match stack {
        Some(nbt) => Some(view_stack(nbt, &player.inventory.recipes)?),
        None => None,
    };
    match set_slot(&player.mode, &mut player.inventory, slot, stack, known) {
        Ok(drop) if slot == CreativeSlot::Drop => {
            drop.filter(|_| player.drop_throttle.allow(&player.name))
        }
        Ok(drop) => drop,
        Err(refused) => {
            diag::debug!(World, "creative slot set from {} refused: {refused:?}", player.name);
            None
        }
    }
}

fn oven_data(data: [i32; 4]) -> Vec<(u8, i32)> {
    data.iter()
        .enumerate()
        .map(|(index, value)| (index as u8, *value))
        .collect()
}

fn contents(
    menu: &Menu,
    inventory: &Inventory,
    containers: &mut Containers,
) -> (Vec<Option<ItemStack>>, [i32; 4]) {
    match &menu.open {
        Open::Inventory => (inventory.crafting.to_vec(), [0; 4]),
        Open::Table(_) => (inventory.workbench.to_vec(), [0; 4]),
        Open::Block(opened) => {
            let cells = containers.cells(opened);
            let data = match opened.station {
                Station::Oven(_) => containers.oven(opened.cells[0]).map(|oven| oven.data()),
                Station::Brewer => containers
                    .brewer(opened.cells[0])
                    .map(|brewer| brewer.data()),
                _ => None,
            };
            (cells, data.unwrap_or([0; 4]))
        }
        Open::Bench(_, bench) => {
            let chosen = bench.chosen.map_or(-1, |chosen| chosen as i32);
            (bench.cells.clone(), [chosen, 0, 0, 0])
        }
        Open::Merchant(screen) => {
            let mut cells = screen.view.payment.to_vec();
            cells.push(screen.view.result.clone());
            (cells, [0; 4])
        }
        Open::Mount(mount) => (mount.cells.clone(), [0; 4]),
    }
}

pub fn screen(player: &mut Player, containers: &mut Containers) -> Screen {
    let Player {
        menus,
        inventory,
        ender,
        enchantment_seed,
        ..
    } = player;
    let Some(menu) = menus.open.as_ref() else {
        return Screen {
            seed: *enchantment_seed,
            ..Screen::default()
        };
    };
    let (cells, data) =
        containers.with_ender(ender, |containers| contents(menu, inventory, containers));
    Screen {
        open: Some(menu.open.clone()),
        cells,
        data,
        seed: *enchantment_seed,
        shelves: menu.shelves,
    }
}

fn kind_of(open: &Open) -> MenuKind {
    match open {
        Open::Inventory => MenuKind::Inventory,
        Open::Table(_) => MenuKind::CraftingTable,
        Open::Block(opened) => match opened.station {
            Station::Oven(CookingKind::Furnace) => MenuKind::Furnace,
            Station::Oven(CookingKind::Smoker) => MenuKind::Smoker,
            Station::Oven(CookingKind::BlastFurnace) => MenuKind::BlastFurnace,
            Station::Brewer => MenuKind::BrewingStand,
            Station::Ender => MenuKind::EnderChest,
            Station::Shulker => MenuKind::ShulkerBox,
            Station::Chest | Station::Barrel => MenuKind::Chest {
                rows: opened.rows() as u8,
            },
        },
        Open::Bench(_, bench) => match bench.kind {
            Workshop::Stonecutter => MenuKind::Stonecutter,
            Workshop::Smithing => MenuKind::Smithing,
            Workshop::Grindstone => MenuKind::Grindstone,
            Workshop::Anvil => MenuKind::Anvil,
            Workshop::Enchanting => MenuKind::Enchanting,
        },
        Open::Merchant(_) => MenuKind::Merchant,
        Open::Mount(_) => MenuKind::Mount,
    }
}

fn net_item(id: &str, count: i32, components: Option<&serde_json::Value>) -> Nbt {
    let mut map = std::collections::BTreeMap::from([
        ("id".to_owned(), Tag::String(id.to_owned())),
        ("count".to_owned(), Tag::Int(count)),
    ]);
    if let Some(components) = components.and_then(minecraft_terrain::item_nbt::components_tag) {
        map.insert("components".to_owned(), components);
    }
    Nbt(minecraftoss_core::nbt::write(&Tag::Compound(map), ""))
}

fn item_of(nbt: &Nbt) -> Option<(String, i32, Option<serde_json::Value>)> {
    let tag = minecraftoss_core::nbt::read(&nbt.0).ok()?;
    let id = tag.get("id")?.as_str()?.to_owned();
    let count = tag.get("count").and_then(Tag::as_i64).unwrap_or(1);
    let components = tag
        .get("components")
        .and_then(minecraft_terrain::item_nbt::saved_components)
        .map(|components| minecraft_terrain::item_nbt::tag_json(&components));
    Some((id, i32::try_from(count).ok()?, components))
}

pub fn net_offer(offer: &MerchantOffer) -> NetOffer {
    let sold = (!offer.sell.components.is_empty())
        .then(|| serde_json::Value::Object(offer.sell.components.clone()));
    NetOffer {
        cost_a: net_item(&offer.buy.id, offer.buy.count, offer.buy.components.as_ref()),
        cost_b: offer
            .buy_b
            .as_ref()
            .map(|cost| net_item(&cost.id, cost.count, cost.components.as_ref())),
        result: net_item(&offer.sell.id, offer.sell.count, sold.as_ref()),
        uses: offer.uses,
        max_uses: offer.max_uses,
        xp: offer.xp,
        special_price: offer.special_price,
        demand: offer.demand,
        price_multiplier: offer.price_multiplier,
        reward_exp: offer.reward_exp,
    }
}

pub fn offer_of(offer: &NetOffer) -> Option<MerchantOffer> {
    let cost = |nbt: &Nbt| {
        item_of(nbt).map(|(id, count, components)| ItemCost {
            id,
            count,
            components,
        })
    };
    let (id, count, components) = item_of(&offer.result)?;
    let components = match components {
        Some(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    Some(MerchantOffer {
        buy: cost(&offer.cost_a)?,
        buy_b: offer.cost_b.as_ref().and_then(cost),
        sell: TradeItem {
            id,
            count,
            components,
        },
        uses: offer.uses,
        max_uses: offer.max_uses,
        reward_exp: offer.reward_exp,
        special_price: offer.special_price,
        demand: offer.demand,
        price_multiplier: offer.price_multiplier,
        xp: offer.xp,
    })
}

fn merchant_extras(screen: &MerchantScreen) -> MenuExtras {
    let view = &screen.view;
    MenuExtras::Merchant {
        villager: view.villager,
        profession: view.profession.to_owned(),
        level: view.level,
        xp: view.xp,
        future_xp: view.future_xp,
        show_progress: !view.trader,
        selected: screen
            .selected
            .and_then(|selected| u16::try_from(selected).ok()),
        offers: view.offers.iter().map(net_offer).collect(),
    }
}

fn mirror_merchant(view: &MenuView, cells: &[Option<ItemStack>]) -> Option<Open> {
    let MenuExtras::Merchant {
        villager,
        profession,
        level,
        xp,
        future_xp,
        show_progress,
        selected,
        offers,
    } = &view.extras
    else {
        return None;
    };
    let profession = Profession::ALL
        .iter()
        .map(|known| known.id())
        .find(|id| id == profession)
        .unwrap_or(Profession::None.id());
    let cell = |index: usize| cells.get(index).cloned().flatten();
    Some(Open::Merchant(MerchantScreen {
        view: MerchantView {
            villager: *villager,
            profession,
            level: *level,
            xp: *xp,
            offers: offers.iter().filter_map(offer_of).collect(),
            payment: [cell(0), cell(1)],
            result: cell(2),
            future_xp: *future_xp,
            max_stacks: Vec::new(),
            name: None,
            trader: !*show_progress,
        },
        selected: selected.map(usize::from),
        held: VecDeque::new(),
    }))
}

struct Viewer<'a> {
    inventory: &'a Inventory,
    books: Books<'a>,
    seed: i32,
    level: u32,
}

fn snapshot(menu: &Menu, viewer: &Viewer<'_>, containers: &mut Containers, carried: bool) -> Sent {
    let (cells, data) = contents(menu, viewer.inventory, containers);
    let data = match &menu.open {
        Open::Block(opened) if matches!(opened.station, Station::Oven(_) | Station::Brewer) => {
            oven_data(data)
        }
        Open::Bench(_, bench) if bench.kind == Workshop::Stonecutter => vec![(0, data[0])],
        _ => Vec::new(),
    };
    let extras = match &menu.open {
        Open::Bench(_, bench) if bench.kind == Workshop::Enchanting => MenuExtras::Enchanting {
            seed: viewer.seed,
            shelves: menu.shelves,
        },
        Open::Bench(_, bench) if bench.kind == Workshop::Anvil => {
            let ctx = context(
                &viewer.inventory.recipes,
                viewer.books,
                menu.shelves,
                viewer.seed,
                viewer.level,
            );
            MenuExtras::Anvil {
                cost: i32::try_from(bench.offer(&ctx).cost).unwrap_or(i32::MAX),
                name: bench.name.clone(),
            }
        }
        Open::Merchant(screen) => merchant_extras(screen),
        Open::Mount(mount) => MenuExtras::Mount {
            mount: mount.vehicle,
            columns: u8::try_from(mount.columns).unwrap_or(u8::MAX),
            saddle: mount.saddle,
            armor: mount.armor,
            llama: mount.llama,
        },
        _ => MenuExtras::None,
    };
    Sent {
        slots: cells
            .iter()
            .map(|cell| cell.as_ref().and_then(slot_nbt))
            .collect(),
        carried: viewer
            .inventory
            .cursor
            .as_ref()
            .filter(|_| carried)
            .and_then(slot_nbt),
        data,
        extras,
    }
}

fn view_of(menu: &Menu, sent: &Sent) -> MenuView {
    MenuView {
        menu: menu.id,
        kind: kind_of(&menu.open),
        title: menu.title.clone(),
        state: menu.state,
        slots: sent.slots.clone(),
        carried: sent.carried.clone(),
        data: sent.data.clone(),
        extras: sent.extras.clone(),
    }
}

fn sync(
    menu: &mut Menu,
    viewer: &Viewer<'_>,
    containers: &mut Containers,
    carried: bool,
    out: &mut Vec<ToClient>,
) {
    let now = snapshot(menu, viewer, containers, carried);
    let before = menu.sent.take();
    let resend = std::mem::take(&mut menu.resend);
    if before.as_ref() == Some(&now) && !resend {
        menu.sent = before;
        return;
    }
    match before.filter(|before| {
        !resend && before.extras == now.extras && before.slots.len() == now.slots.len()
    }) {
        Some(before) => {
            if before.slots != now.slots {
                menu.state = (menu.state + 1) & STATE_MASK;
            }
            let slots = now
                .slots
                .iter()
                .enumerate()
                .filter(|(index, slot)| before.slots.get(*index) != Some(slot))
                .map(|(index, slot)| (index as i16, slot.clone()))
                .collect();
            let data = now
                .data
                .iter()
                .filter(|entry| !before.data.contains(entry))
                .copied()
                .collect();
            out.push(ToClient::MenuSlots {
                menu: menu.id,
                state: menu.state,
                slots,
                carried: (before.carried != now.carried).then(|| now.carried.clone()),
                data,
            });
        }
        None => {
            menu.state = (menu.state + 1) & STATE_MASK;
            out.push(ToClient::MenuOpen(view_of(menu, &now)));
        }
    }
    menu.sent = Some(now);
}

pub fn changes(
    player: &mut Player,
    containers: &mut Containers,
    books: Books<'_>,
) -> Vec<ToClient> {
    let mut out = Vec::new();
    let Player {
        menus,
        inventory,
        ender,
        survival,
        enchantment_seed,
        ..
    } = player;
    if let Some(told) = menus.told
        && menus.open.as_ref().map(|menu| menu.id) != Some(told)
    {
        out.push(ToClient::MenuClose { menu: told });
        menus.told = None;
    }
    let viewer = Viewer {
        inventory,
        books,
        seed: *enchantment_seed,
        level: survival.status.experience_level,
    };
    let station = menus.open.is_some();
    containers.with_ender(ender, |containers| {
        sync(
            &mut menus.inventory,
            &viewer,
            containers,
            !station,
            &mut out,
        );
        if let Some(menu) = menus.open.as_mut() {
            sync(menu, &viewer, containers, true, &mut out);
            menus.told = Some(menu.id);
        }
    });
    out
}

pub fn finish(
    entities: &mut Entities,
    terrain: &mut Terrain,
    body: &mut dyn Body,
    used: Vec<(&'static str, BlockPos)>,
    anvil: Option<BlockPos>,
) -> Vec<Effect> {
    let centre = |pos: BlockPos| {
        [
            f64::from(pos.0) + 0.5,
            f64::from(pos.1) + 0.5,
            f64::from(pos.2) + 0.5,
        ]
    };
    let mut effects = Vec::new();
    for (event, pos) in used {
        let pitch = entities.random_float() * 0.1 + 0.9;
        effects.push(Effect::Sound {
            event: event.to_owned(),
            at: Some(centre(pos)),
            volume: 1.0,
            pitch,
            spread: 0.0,
        });
    }
    if let Some(pos) = anvil {
        let block = Scene::block(&terrain.scene, pos).cloned();
        let worn = entities.random_float() < 0.12;
        let next = block
            .as_ref()
            .filter(|_| worn)
            .map(|b| match b.id.key().as_str() {
                "minecraft:anvil" => Some("minecraft:chipped_anvil"),
                "minecraft:chipped_anvil" => Some("minecraft:damaged_anvil"),
                _ => None,
            });
        let event = if next == Some(None) {
            "minecraft:block.anvil.destroy"
        } else {
            "minecraft:block.anvil.use"
        };
        if let Some(next) = next {
            let state = next.map(|id| {
                let mut worn = minecraft_terrain::scene::Block::new(id);
                worn.properties = block
                    .as_ref()
                    .map_or_else(Default::default, |b| b.properties.clone());
                worn
            });
            body.write_blocks(terrain, entities, vec![(pos, state)]);
        }
        let pitch = entities.random_float() * 0.1 + 0.9;
        effects.push(Effect::Sound {
            event: event.to_owned(),
            at: Some(centre(pos)),
            volume: 1.0,
            pitch,
            spread: 0.0,
        });
    }
    effects
}

pub struct Lids<'a> {
    pub entities: &'a mut Entities,
    pub terrain: &'a mut Terrain,
    pub openers: &'a mut minecraft_terrain::openers::Openers,
    pub body: &'a mut dyn Body,
}

pub fn lids(
    lids: Lids<'_>,
    (uuid, player, feet): (&PlayerUuid, u64, [f64; 3]),
    was: Option<&Opened>,
    now: Option<&Opened>,
    spectator: bool,
) -> Vec<Effect> {
    let mut events = Vec::new();
    for (opened, open) in [
        (was.filter(|was| now != Some(*was)), false),
        (now.filter(|now| was != Some(*now)), true),
    ] {
        let Some(opened) = opened else { continue };
        let counted = opened.cells.iter().any(|&pos| {
            Scene::block(&lids.terrain.scene, pos)
                .is_some_and(minecraft_terrain::openers::Openers::counted)
        });
        if open && counted {
            lids.entities.anger_piglins(player, feet, true);
        }
        events.extend(crate::level::toggle_openers(
            lids.openers,
            &lids.terrain.scene,
            &opened.cells,
            uuid,
            open,
            spectator,
        ));
    }
    lid_effects(lids.entities, lids.terrain, lids.body, events)
}

pub fn recheck_lids(
    lids: Lids<'_>,
    open_cells: impl Fn(u128) -> Option<Vec<BlockPos>>,
) -> Vec<Effect> {
    if !lids.entities.ticks().is_multiple_of(RECHECK_TICKS) {
        return Vec::new();
    }
    let events = lids
        .openers
        .recheck(|player, pos| open_cells(player).is_none_or(|cells| cells.contains(&pos)));
    lid_effects(lids.entities, lids.terrain, lids.body, events)
}

fn lid_effects(
    entities: &mut Entities,
    terrain: &mut Terrain,
    body: &mut dyn Body,
    events: Vec<minecraft_terrain::openers::OpenerEvent>,
) -> Vec<Effect> {
    let mut effects = Vec::new();
    for event in events {
        let outcome = crate::level::opener_outcome(&terrain.scene, &event);
        let (cell, id, count) = outcome.block_event;
        entities.lid_event(&terrain.scene, cell, id, count);
        if let Some((pos, viewers)) = outcome.trapped {
            entities.chest_openers(pos, viewers);
        }
        if let Some(sound) = outcome.sound {
            let pitch = minecraft_terrain::openers::pitch(entities.random_float());
            effects.push(Effect::Sound {
                event: sound.event.to_owned(),
                at: Some(sound.position),
                volume: sound.volume,
                pitch,
                spread: 0.0,
            });
        }
        if let Some((cell, state)) = outcome.state {
            body.write_blocks(terrain, entities, vec![(cell, Some(state))]);
        }
    }
    effects
}

fn mirror_open(view: &MenuView, cells: &[Option<ItemStack>], data: [i32; 4]) -> Option<Open> {
    let at = |index: i32| (0, -1000 - index, 0);
    let block = |station: Station, count: i32| {
        Some(Open::Block(Opened {
            station,
            cells: (0..count).map(at).collect(),
        }))
    };
    let bench = |kind: Workshop| {
        let mut bench = Bench::new(kind);
        for (cell, stack) in bench.cells.iter_mut().zip(cells) {
            *cell = stack.clone();
        }
        if kind == Workshop::Stonecutter {
            bench.chosen = usize::try_from(data[0]).ok();
        }
        if let MenuExtras::Anvil { name, .. } = &view.extras {
            bench.name = name.clone();
        }
        Some(Open::Bench(at(0), bench))
    };
    match view.kind {
        MenuKind::Inventory => None,
        MenuKind::CraftingTable => Some(Open::Table(at(0))),
        MenuKind::Chest { rows } => block(Station::Chest, i32::from(rows / 3).max(1)),
        MenuKind::EnderChest => block(Station::Ender, 1),
        MenuKind::ShulkerBox => block(Station::Shulker, 1),
        MenuKind::Furnace => block(Station::Oven(CookingKind::Furnace), 1),
        MenuKind::Smoker => block(Station::Oven(CookingKind::Smoker), 1),
        MenuKind::BlastFurnace => block(Station::Oven(CookingKind::BlastFurnace), 1),
        MenuKind::BrewingStand => block(Station::Brewer, 1),
        MenuKind::Enchanting => bench(Workshop::Enchanting),
        MenuKind::Anvil => bench(Workshop::Anvil),
        MenuKind::Grindstone => bench(Workshop::Grindstone),
        MenuKind::Smithing => bench(Workshop::Smithing),
        MenuKind::Stonecutter => bench(Workshop::Stonecutter),
        MenuKind::Merchant => mirror_merchant(view, cells),
        MenuKind::Mount => mirror_mount(view, cells),
        MenuKind::Loom
        | MenuKind::Cartography
        | MenuKind::Hopper
        | MenuKind::Dispenser
        | MenuKind::Dropper
        | MenuKind::Beacon => None,
    }
}

fn mirror_mount(view: &MenuView, cells: &[Option<ItemStack>]) -> Option<Open> {
    let MenuExtras::Mount {
        mount,
        columns,
        saddle,
        armor,
        llama,
    } = view.extras
    else {
        return None;
    };
    let columns = usize::from(columns);
    let mut cells = cells.to_vec();
    cells.resize(minecraft_terrain::mount_inventory::CHEST + columns * 3, None);
    Some(Open::Mount(MountInventory {
        vehicle: mount,
        columns,
        saddle,
        armor,
        llama,
        cells,
    }))
}

pub fn mount_title(world: &minecraftoss_entities::world::EntityWorld, vehicle: u64) -> Text {
    let kind = world
        .entity_type(vehicle)
        .unwrap_or("minecraft:horse")
        .rsplit(':')
        .next()
        .unwrap_or("horse");
    Text::translate(format!("entity.minecraft.{kind}"), Vec::new())
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MountShown {
    pub opened: bool,
    pub closed: bool,
    pub thrown: Vec<ItemStack>,
}

pub fn mount(
    player: &mut Player,
    entities: &mut Entities,
    eye: [f64; 3],
    riding_key: bool,
    alive: bool,
) -> MountShown {
    let id = u64::from(player.id.0);
    let mut shown = MountShown::default();
    let mut wanted = entities.take_mount_menus(id);
    let riding_key = std::mem::take(&mut player.menus.mount_key) || riding_key;
    if riding_key
        && let Some(vehicle) = player.riding.map(|ride| ride.vehicle)
        && entities.view.world.horse_opens_inventory(vehicle, id)
    {
        wanted.push(vehicle);
    }
    for vehicle in wanted {
        if !alive
            || player.menus.open.is_some()
            || entities
                .mount_viewer(vehicle)
                .is_some_and(|viewer| viewer != id)
        {
            continue;
        }
        let Some(inventory) =
            MountInventory::of(&entities.view.world, vehicle, &player.inventory.recipes)
        else {
            continue;
        };
        let title = mount_title(&entities.view.world, vehicle);
        shown.opened |= open(player, Open::Mount(inventory), title, 0);
    }
    let invalid = match player.menus.open.as_ref().map(|menu| &menu.open) {
        Some(Open::Mount(inventory)) => {
            !alive
                || !inventory.still_valid(&entities.view.world, glam::DVec3::from_array(eye))
        }
        _ => false,
    };
    if invalid {
        shown.thrown = close(player);
        shown.closed = true;
    }
    for update in std::mem::take(&mut player.menus.mount_updates) {
        entities.set_mount_inventory(&update);
    }
    let viewing = match player.menus.open.as_ref().map(|menu| &menu.open) {
        Some(Open::Mount(inventory)) => Some(inventory.vehicle),
        _ => None,
    };
    entities.set_mount_viewer(id, viewing);
    shown
}

pub fn stacks(slots: &[Option<Nbt>], recipes: &RecipeBook) -> Vec<Option<ItemStack>> {
    slots
        .iter()
        .map(|slot| slot.as_ref().and_then(|nbt| view_stack(nbt, recipes)))
        .collect()
}

pub fn mirror(view: Option<&MenuView>, recipes: &RecipeBook) -> Screen {
    let Some(view) = view else {
        return Screen::default();
    };
    let cells = stacks(&view.slots, recipes);
    let mut data = [0; 4];
    for &(index, value) in &view.data {
        if let Some(slot) = data.get_mut(usize::from(index)) {
            *slot = value;
        }
    }
    let (seed, shelves) = match view.extras {
        MenuExtras::Enchanting { seed, shelves } => (seed, shelves),
        _ => (0, 0),
    };
    Screen {
        open: mirror_open(view, &cells, data),
        cells,
        data,
        seed,
        shelves,
    }
}

pub fn apply_slots(
    view: &mut MenuView,
    state: u16,
    slots: Vec<(i16, Option<Nbt>)>,
    carried: Option<Option<Nbt>>,
    data: Vec<(u8, i32)>,
) {
    view.state = state;
    for (index, slot) in slots {
        if let Some(cell) = usize::try_from(index)
            .ok()
            .and_then(|index| view.slots.get_mut(index))
        {
            *cell = slot;
        }
    }
    if let Some(carried) = carried {
        view.carried = carried;
    }
    for (index, value) in data {
        match view.data.iter_mut().find(|(at, _)| *at == index) {
            Some(entry) => entry.1 = value,
            None => view.data.push((index, value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_commands::{CommandServer, PlayerId};
    use minecraft_terrain::game_mode::{GameMode, PlayerMode};

    use crate::survival::Survival;

    fn player(chat: &mut CommandServer, id: u32, name: &str) -> Player {
        let id = PlayerId(id);
        chat.join(id, name);
        Player::new(
            chat.profile(id).unwrap(),
            PlayerMode::default(),
            Survival::default(),
            Inventory::default(),
        )
    }

    fn books(book: &StationBook) -> Books<'_> {
        Books {
            stations: book,
            enchantments: None,
            guns: None,
        }
    }

    fn chest() -> Opened {
        Opened {
            station: Station::Chest,
            cells: vec![(4, 64, 4)],
        }
    }

    fn shown(nbt: &Option<Nbt>) -> Option<(String, u8)> {
        nbt.as_ref()
            .and_then(|nbt| view_stack(nbt, &RecipeBook::default()))
            .map(|stack| (stack.id, stack.count))
    }

    fn slot(index: u8, shift: bool) -> MenuAction {
        MenuAction::Slot {
            slot: MenuSlot::Inventory(index),
            right: false,
            shift,
        }
    }

    fn cell(index: u16) -> MenuAction {
        MenuAction::Slot {
            slot: MenuSlot::Container(index),
            right: false,
            shift: false,
        }
    }

    fn opened_views(out: &[ToClient]) -> Vec<MenuView> {
        out.iter()
            .filter_map(|message| match message {
                ToClient::MenuOpen(view) => Some(view.clone()),
                _ => None,
            })
            .collect()
    }

    fn friend_click(
        friend: &mut Player,
        containers: &mut Containers,
        book: &StationBook,
        (menu, state): (u8, u16),
        action: MenuAction,
    ) -> Clicked {
        let message = Click {
            menu,
            state,
            action,
        };
        click(friend, containers, books(book), &message, 0.5)
    }

    #[test]
    fn two_players_in_one_chest_see_each_others_clicks() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let mut friend = player(&mut chat, 1, "Alex");
        let book = StationBook::default();
        let mut containers = Containers::default();
        assert!(open(
            &mut host,
            Open::Block(chest()),
            Text::literal("Chest"),
            0
        ));
        assert!(open(
            &mut friend,
            Open::Block(chest()),
            Text::literal("Chest"),
            0
        ));
        let first = changes(&mut friend, &mut containers, books(&book));
        let mut views = opened_views(&first);
        assert_eq!(
            views.iter().map(|view| view.menu).collect::<Vec<_>>(),
            vec![INVENTORY_MENU, 1]
        );
        assert_eq!(views[1].kind, MenuKind::Chest { rows: 3 });
        assert_eq!(views[1].slots.len(), 27);
        assert!(changes(&mut friend, &mut containers, books(&book)).is_empty());
        host.inventory.slots[0] = Some(ItemStack::new("minecraft:dirt", 10));
        apply(
            &mut host,
            &mut containers,
            books(&book),
            &slot(0, true),
            0.5,
        );
        assert!(host.inventory.slots[0].is_none());
        let seen = changes(&mut friend, &mut containers, books(&book));
        let [
            ToClient::MenuSlots {
                menu: 1,
                state,
                slots,
                carried: None,
                data,
            },
        ] = &seen[..]
        else {
            panic!("{seen:?}");
        };
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].0, 0);
        assert_eq!(shown(&slots[0].1), Some(("minecraft:dirt".to_owned(), 10)));
        apply_slots(&mut views[1], *state, slots.clone(), None, data.clone());
        let clicked = friend_click(&mut friend, &mut containers, &book, (1, *state), cell(0));
        assert_eq!(clicked, Clicked::default());
        let carried = friend.inventory.cursor.as_ref();
        assert_eq!(carried.map(|stack| stack.count), Some(10));
        assert!(screen(&mut host, &mut containers).cells[0].is_none());
        let after = changes(&mut friend, &mut containers, books(&book));
        let ToClient::MenuSlots {
            state,
            slots,
            carried,
            data,
            ..
        } = after[0].clone()
        else {
            panic!("{after:?}");
        };
        assert_eq!(slots, vec![(0, None)]);
        assert_eq!(
            shown(carried.as_ref().unwrap()),
            Some(("minecraft:dirt".to_owned(), 10))
        );
        apply_slots(&mut views[1], state, slots, carried, data);
        let menu = friend.menus.open.as_ref().unwrap();
        let fresh = view_of(menu, menu.sent.as_ref().unwrap());
        assert_eq!(views[1], fresh, "the friend's copy matches the host");
    }

    #[test]
    fn a_stale_state_resends_everything() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = player(&mut chat, 1, "Alex");
        let book = StationBook::default();
        let mut containers = Containers::default();
        open(&mut friend, Open::Block(chest()), Text::literal("Chest"), 0);
        let _ = changes(&mut friend, &mut containers, books(&book));
        let state = friend.menus.current().state;
        friend.inventory.slots[3] = Some(ItemStack::new("minecraft:dirt", 4));
        let stale = (1, state.wrapping_sub(1));
        friend_click(&mut friend, &mut containers, &book, stale, slot(3, true));
        assert!(
            friend.inventory.slots[3].is_none(),
            "a stale click still acts"
        );
        assert!(friend.menus.current().resends());
        let out = changes(&mut friend, &mut containers, books(&book));
        let views = opened_views(&out);
        assert_eq!(views.len(), 1, "{out:?}");
        assert_eq!(views[0].menu, 1);
        assert_eq!(
            shown(&views[0].slots[0]),
            Some(("minecraft:dirt".to_owned(), 4))
        );
        assert_ne!(views[0].state, state);
        let state = views[0].state;
        friend_click(&mut friend, &mut containers, &book, (1, state), cell(0));
        let out = changes(&mut friend, &mut containers, books(&book));
        assert!(
            matches!(out[..], [ToClient::MenuSlots { menu: 1, .. }]),
            "{out:?}"
        );
    }

    #[test]
    fn a_burning_furnace_does_not_make_a_click_stale() {
        use minecraftoss_core::nbt::Tag;
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = player(&mut chat, 1, "Alex");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let pos = (4, 64, 4);
        let tag = Tag::Compound(std::collections::BTreeMap::from([
            ("id".to_owned(), Tag::String("minecraft:furnace".to_owned())),
            ("lit_time_remaining".to_owned(), Tag::Int(100)),
            ("lit_total_time".to_owned(), Tag::Int(100)),
        ]));
        containers.import(pos, &tag, &RecipeBook::default());
        let furnace = Opened {
            station: Station::Oven(CookingKind::Furnace),
            cells: vec![pos],
        };
        open(
            &mut friend,
            Open::Block(furnace),
            Text::literal("Furnace"),
            0,
        );
        let _ = changes(&mut friend, &mut containers, books(&book));
        let state = friend.menus.current().state;
        containers.tick(&RecipeBook::default());
        let burning = changes(&mut friend, &mut containers, books(&book));
        let [
            ToClient::MenuSlots {
                menu: 1,
                state: sent,
                slots,
                data,
                ..
            },
        ] = &burning[..]
        else {
            panic!("{burning:?}");
        };
        assert!(slots.is_empty(), "{slots:?}");
        assert_eq!(data, &vec![(0, 99)]);
        assert_eq!(*sent, state, "a data change keeps the state, as vanilla's");
        friend.inventory.slots[3] = Some(ItemStack::new("minecraft:coal", 2));
        friend_click(
            &mut friend,
            &mut containers,
            &book,
            (1, state),
            slot(3, true),
        );
        assert!(
            !friend.menus.current().resends(),
            "the click saw the latest state"
        );
    }

    #[test]
    fn a_click_on_a_closed_menu_or_from_another_dimension_is_ignored() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = player(&mut chat, 1, "Alex");
        let book = StationBook::default();
        let mut containers = Containers::default();
        friend.inventory.slots[0] = Some(ItemStack::new("minecraft:dirt", 4));
        open(&mut friend, Open::Block(chest()), Text::literal("Chest"), 0);
        let _ = changes(&mut friend, &mut containers, books(&book));
        let state = friend.menus.current().state;
        friend_click(
            &mut friend,
            &mut containers,
            &book,
            (7, state),
            slot(0, true),
        );
        assert!(friend.inventory.slots[0].is_some(), "not the open menu");
        friend_click(&mut friend, &mut containers, &book, (1, state), cell(200));
        assert!(friend.menus.current().resends(), "a slot past the chest");
        assert!(close_requested(&mut friend, 1).is_empty());
        assert!(friend.menus.open.is_none());
        friend_click(
            &mut friend,
            &mut containers,
            &book,
            (1, state),
            slot(0, true),
        );
        assert!(friend.inventory.slots[0].is_some(), "the chest is closed");
        assert!(
            changes(&mut friend, &mut containers, books(&book))
                .iter()
                .all(|message| !matches!(message, ToClient::MenuClose { .. })),
            "the friend closed it, so no close goes back"
        );
        open(&mut friend, Open::Block(chest()), Text::literal("Chest"), 0);
        let _ = changes(&mut friend, &mut containers, books(&book));
        let open_menu = (friend.menus.current().id, friend.menus.current().state);
        assert_eq!(open_menu.0, 2);
        friend.dimension = Dimension::Nether;
        friend_click(
            &mut friend,
            &mut containers,
            &book,
            open_menu,
            slot(0, true),
        );
        assert!(
            friend.inventory.slots[0].is_some(),
            "opened in the Overworld"
        );
        friend.dimension = Dimension::Overworld;
        friend.dead = true;
        friend_click(
            &mut friend,
            &mut containers,
            &book,
            open_menu,
            slot(0, true),
        );
        assert!(friend.inventory.slots[0].is_some(), "a dead player");
        friend.dead = false;
        let _ = changes(&mut friend, &mut containers, books(&book));
        let open_menu = (friend.menus.current().id, friend.menus.current().state);
        friend_click(
            &mut friend,
            &mut containers,
            &book,
            open_menu,
            slot(0, true),
        );
        assert!(friend.inventory.slots[0].is_none());
        let _ = close(&mut friend);
        let out = changes(&mut friend, &mut containers, books(&book));
        assert!(out.contains(&ToClient::MenuClose { menu: 2 }), "{out:?}");
    }

    #[test]
    fn closing_with_a_full_inventory_drops_the_overflow() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = player(&mut chat, 1, "Alex");
        for slot in (0..36).chain([OFFHAND]) {
            friend.inventory.slots[slot] = Some(ItemStack::new("minecraft:stone", 64));
        }
        friend.inventory.crafting[2] = Some(ItemStack::new("minecraft:dirt", 3));
        friend.inventory.cursor = Some(ItemStack::new("minecraft:dirt", 2));
        let thrown = close_requested(&mut friend, INVENTORY_MENU);
        let dirt: u32 = thrown
            .iter()
            .filter(|stack| stack.id == "minecraft:dirt")
            .map(|stack| u32::from(stack.count))
            .sum();
        assert_eq!(dirt, 5);
        assert!(friend.inventory.crafting.iter().all(Option::is_none));
        assert!(friend.inventory.cursor.is_none());
        let at = (0, 64, 0);
        let mut anvil = Bench::new(Workshop::Anvil);
        anvil.cells[0] = Some(ItemStack::new("minecraft:iron_sword", 1));
        open(
            &mut friend,
            Open::Bench(at, anvil.clone()),
            Text::literal("Anvil"),
            0,
        );
        let thrown = close(&mut friend);
        assert_eq!(thrown.len(), 1);
        assert_eq!(thrown[0].id, "minecraft:iron_sword");
        friend.inventory.slots[5] = None;
        open(
            &mut friend,
            Open::Bench(at, anvil),
            Text::literal("Anvil"),
            0,
        );
        assert!(close(&mut friend).is_empty(), "room for it");
        assert_eq!(
            friend.inventory.slots[5]
                .as_ref()
                .map(|stack| stack.id.as_str()),
            Some("minecraft:iron_sword")
        );
    }

    #[test]
    fn an_anvil_rename_applies() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = player(&mut chat, 1, "Alex");
        let book = StationBook::default();
        let mut containers = Containers::default();
        friend.survival.status.experience_level = 5;
        friend.inventory.slots[0] = Some(ItemStack::new("minecraft:iron_sword", 1));
        let pos = (2, 64, 2);
        let bench = Open::Bench(pos, Bench::new(Workshop::Anvil));
        open(&mut friend, bench, Text::literal("Anvil"), 0);
        apply(
            &mut friend,
            &mut containers,
            books(&book),
            &slot(0, true),
            0.5,
        );
        assert!(friend.inventory.slots[0].is_none());
        let _ = changes(&mut friend, &mut containers, books(&book));
        rename(&mut friend, "Blade", None);
        let out = changes(&mut friend, &mut containers, books(&book));
        let views = opened_views(&out);
        assert_eq!(
            views.last().map(|view| view.extras.clone()),
            Some(MenuExtras::Anvil {
                cost: 1,
                name: Some("Blade".to_owned()),
            }),
            "{out:?}"
        );
        let output = MenuAction::Slot {
            slot: MenuSlot::Output,
            right: false,
            shift: false,
        };
        let clicked = apply(&mut friend, &mut containers, books(&book), &output, 0.5);
        assert_eq!(clicked.anvil, Some(pos));
        let named = friend.inventory.cursor.as_ref();
        assert_eq!(
            named.and_then(minecraft_terrain::stations::custom_name),
            Some("Blade".to_owned())
        );
        assert_eq!(friend.survival.status.experience_level, 4);
        let _ = close(&mut friend);
        let mut anvil = Bench::new(Workshop::Anvil);
        anvil.cells[0] = Some(ItemStack::new("minecraft:iron_sword", 1));
        open(
            &mut friend,
            Open::Bench(pos, anvil),
            Text::literal("Anvil"),
            0,
        );
        rename(&mut friend, "Blade", None);
        rename(&mut friend, "minecraft:iron_sword", None);
        let Some(Menu {
            open: Open::Bench(_, bench),
            ..
        }) = friend.menus.open.as_ref()
        else {
            panic!("the anvil is open");
        };
        assert_eq!(bench.name, None, "the item's own name is no rename");
    }

    #[test]
    fn dying_with_items_in_the_grid_drops_them() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = player(&mut chat, 1, "Alex");
        friend.inventory.slots[5] = Some(ItemStack::new("minecraft:dirt", 1));
        friend.inventory.crafting[1] = Some(ItemStack::new("minecraft:oak_planks", 2));
        friend.inventory.cursor = Some(ItemStack::new("minecraft:stick", 1));
        let mut grindstone = Bench::new(Workshop::Grindstone);
        grindstone.cells[0] = Some(ItemStack::new("minecraft:iron_sword", 1));
        let open_grindstone = Open::Bench((0, 64, 0), grindstone);
        open(&mut friend, open_grindstone, Text::literal("Grindstone"), 0);
        assert!(close(&mut friend).is_empty());
        let drops = crate::death::death_drops(&mut friend.inventory, |_| false);
        let mut ids: Vec<&str> = drops.iter().map(|stack| stack.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            vec![
                "minecraft:dirt",
                "minecraft:iron_sword",
                "minecraft:oak_planks",
                "minecraft:stick"
            ]
        );
    }

    #[test]
    fn the_friend_mirrors_a_chest_a_table_and_a_stonecutter() {
        let recipes = RecipeBook::default();
        let mut view = MenuView {
            menu: 3,
            kind: MenuKind::Chest { rows: 6 },
            title: Text::literal("Big"),
            state: 1,
            slots: vec![None; 54],
            carried: None,
            data: Vec::new(),
            extras: MenuExtras::None,
        };
        view.slots[53] = slot_nbt(&ItemStack::new("minecraft:dirt", 7));
        let screen = mirror(Some(&view), &recipes);
        let Some(Open::Block(opened)) = &screen.open else {
            panic!("{:?}", screen.open);
        };
        assert_eq!(
            (opened.station, opened.rows(), opened.size()),
            (Station::Chest, 6, 54)
        );
        assert_eq!(screen.cells[53].as_ref().map(|stack| stack.count), Some(7));
        view.kind = MenuKind::CraftingTable;
        let table = mirror(Some(&view), &recipes);
        assert!(matches!(table.open, Some(Open::Table(_))));
        view.kind = MenuKind::Stonecutter;
        view.slots = vec![slot_nbt(&ItemStack::new("minecraft:stone", 1)), None];
        view.data = vec![(0, 2)];
        let Some(Open::Bench(_, bench)) = mirror(Some(&view), &recipes).open else {
            panic!("a stonecutter");
        };
        assert_eq!(bench.chosen, Some(2));
        assert!(bench.cells[0].is_some());
        assert!(mirror(None, &recipes).open.is_none());
    }

    #[test]
    fn a_named_villager_s_trading_screen_takes_its_name_as_the_title() {
        let named = MerchantView {
            name: Some("Bob".to_owned()),
            ..trade_view(9)
        };
        assert_eq!(merchant_title(&named), Text::literal("Bob"));
        assert_eq!(
            merchant_title(&trade_view(9)),
            Text::translate("entity.minecraft.villager.farmer", Vec::new())
        );
        let trader = MerchantView {
            trader: true,
            profession: "minecraft:none",
            ..trade_view(9)
        };
        assert_eq!(
            merchant_title(&trader),
            Text::translate("entity.minecraft.wandering_trader", Vec::new())
        );
        let screen = MerchantScreen {
            view: trader,
            selected: None,
            held: VecDeque::new(),
        };
        let MenuExtras::Merchant { show_progress, .. } = merchant_extras(&screen) else {
            panic!("a merchant's extras")
        };
        assert!(!show_progress, "a wandering trader has no experience bar");
    }

    fn trade_view(villager: u64) -> MerchantView {
        MerchantView {
            villager,
            profession: "minecraft:farmer",
            level: 1,
            xp: 0,
            offers: Vec::new(),
            payment: [None, None],
            result: None,
            future_xp: 0,
            max_stacks: Vec::new(),
            name: None,
            trader: false,
        }
    }

    fn trading(chat: &mut CommandServer) -> Player {
        let mut steve = player(chat, 0, "Steve");
        let shown = merchant_news(
            &mut steve,
            vec![MerchantNews::Opened(trade_view(9))],
            true,
            false,
        );
        assert!(shown.opened && !shown.closed);
        steve
    }

    fn sent(player: &mut Player) -> Vec<MerchantOp> {
        let mut ops = Vec::new();
        let _ = pump_merchant(player, |op, _, _| ops.push(op));
        ops
    }

    fn answer(view: Option<MerchantView>) -> MerchantNews {
        MerchantNews::Answer(minecraft_terrain::server::MerchantUpdate {
            view,
            slots: Vec::new(),
            cursor: None,
            closing: false,
        })
    }

    #[test]
    fn a_villager_s_trading_screen_turns_clicks_into_trades() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut steve = trading(&mut chat);
        let menu = steve.menus.open.as_ref().expect("the trading screen is open");
        assert!(matches!(menu.open, Open::Merchant(_)));
        assert_eq!(
            menu.title,
            Text::translate("entity.minecraft.villager.farmer", Vec::new())
        );
        let book = StationBook::default();
        let mut containers = Containers::default();
        let pay_right = MenuAction::Slot {
            slot: MenuSlot::Container(1),
            right: true,
            shift: false,
        };
        let take_all = MenuAction::Slot {
            slot: MenuSlot::Output,
            right: false,
            shift: true,
        };
        let expected = [
            (MenuAction::Choose(3), MerchantOp::Select(3)),
            (
                cell(0),
                MerchantOp::Payment {
                    slot: 0,
                    right: false,
                    shift: false,
                },
            ),
            (
                pay_right,
                MerchantOp::Payment {
                    slot: 1,
                    right: true,
                    shift: false,
                },
            ),
            (cell(2), MerchantOp::Result { shift: false }),
            (take_all, MerchantOp::Result { shift: true }),
            (slot(14, true), MerchantOp::QuickMove(14)),
        ];
        for (action, op) in expected {
            let _ = apply(&mut steve, &mut containers, books(&book), &action, 0.5);
            assert_eq!(sent(&mut steve), [op], "{action:?}");
            let shown = merchant_news(&mut steve, vec![answer(Some(trade_view(9)))], true, false);
            assert!(!shown.opened && !shown.closed);
        }
        let Some(Menu {
            open: Open::Merchant(screen),
            ..
        }) = steve.menus.open.as_ref()
        else {
            panic!("still trading");
        };
        assert_eq!(screen.selected, Some(3));
        steve.inventory.slots[2] = Some(ItemStack::new("minecraft:wheat", 5));
        let _ = apply(&mut steve, &mut containers, books(&book), &slot(2, false), 0.5);
        assert!(sent(&mut steve).is_empty(), "a plain click in the inventory stays here");
        assert_eq!(
            steve.inventory.cursor.as_ref().map(|stack| stack.count),
            Some(5)
        );
        let _ = apply(&mut steve, &mut containers, books(&book), &slot(2, false), 0.5);
        assert!(steve.inventory.cursor.is_none());
        assert!(close(&mut steve).is_empty());
        assert!(steve.menus.open.is_none());
        assert_eq!(sent(&mut steve), [MerchantOp::Close]);
    }

    #[test]
    fn clicks_wait_for_the_answer_to_the_click_before_them_and_follow_in_order() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut steve = trading(&mut chat);
        let book = StationBook::default();
        let mut containers = Containers::default();
        steve.inventory.slots[2] = Some(ItemStack::new("minecraft:wheat", 5));
        for action in [cell(0), slot(2, false), slot(9, true), MenuAction::Choose(1)] {
            let _ = apply(&mut steve, &mut containers, books(&book), &action, 0.5);
        }
        assert!(
            steve.inventory.cursor.is_none(),
            "the held inventory click waits too"
        );
        assert_eq!(
            sent(&mut steve),
            [MerchantOp::Payment {
                slot: 0,
                right: false,
                shift: false
            }]
        );
        assert!(sent(&mut steve).is_empty(), "one trade click at a time");
        let _ = merchant_news(&mut steve, Vec::new(), true, true);
        assert!(sent(&mut steve).is_empty(), "the answer has not come");
        let _ = merchant_news(&mut steve, vec![answer(Some(trade_view(9)))], true, false);
        assert_eq!(sent(&mut steve), [MerchantOp::QuickMove(9)]);
        assert_eq!(
            steve.inventory.cursor.as_ref().map(|stack| stack.count),
            Some(5),
            "the inventory click ran once the answer came"
        );
        let _ = merchant_news(&mut steve, vec![answer(Some(trade_view(9)))], true, false);
        assert_eq!(sent(&mut steve), [MerchantOp::Select(1)]);
        let _ = merchant_news(&mut steve, vec![answer(Some(trade_view(9)))], true, false);
        assert!(sent(&mut steve).is_empty());
        let _ = apply(&mut steve, &mut containers, books(&book), &cell(1), 0.5);
        assert!(close(&mut steve).is_empty());
        assert!(
            steve.inventory.cursor.is_some(),
            "the cursor waits for the server's close"
        );
        assert_eq!(
            sent(&mut steve),
            [MerchantOp::Payment {
                slot: 1,
                right: false,
                shift: false
            }]
        );
        assert!(close(&mut steve).is_empty());
        assert!(
            steve.inventory.cursor.is_some(),
            "a close while a click is out leaves the cursor alone"
        );
        let _ = merchant_news(&mut steve, vec![answer(Some(trade_view(9)))], true, false);
        assert_eq!(sent(&mut steve), [MerchantOp::Close]);
    }

    #[test]
    fn the_server_closes_the_trading_screen_and_a_player_who_cannot_show_it_turns_it_down() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut steve = trading(&mut chat);
        let shown = merchant_news(&mut steve, vec![MerchantNews::Closing], true, false);
        assert!(shown.closed && steve.menus.open.is_none());
        assert_eq!(sent(&mut steve), [MerchantOp::Close], "the client answers with a close");
        let _ = merchant_news(&mut steve, vec![answer(None)], true, false);
        let reopened = vec![MerchantNews::Opened(trade_view(9))];
        assert!(merchant_news(&mut steve, reopened, true, false).opened);
        let shown = merchant_news(&mut steve, vec![answer(None)], true, false);
        assert!(shown.closed && steve.menus.open.is_none(), "the villager went away");
        assert!(sent(&mut steve).is_empty());
        let mut alex = player(&mut chat, 1, "Alex");
        let shown = merchant_news(
            &mut alex,
            vec![MerchantNews::Opened(trade_view(9))],
            false,
            false,
        );
        assert!(!shown.opened && alex.menus.open.is_none());
        assert_eq!(sent(&mut alex), [MerchantOp::Close]);
        let mut busy = player(&mut chat, 2, "Kai");
        busy.menus.open = Some(Menu::new(
            5,
            Dimension::Overworld,
            Open::Table((0, 64, 0)),
            Text::literal("Crafting"),
            0,
        ));
        let _ = merchant_news(
            &mut busy,
            vec![MerchantNews::Opened(trade_view(9))],
            true,
            false,
        );
        assert!(matches!(
            busy.menus.open.as_ref().map(|menu| &menu.open),
            Some(Open::Table(_))
        ));
        assert_eq!(sent(&mut busy), [MerchantOp::Close]);
    }
    fn offer(buy: (&str, i32), sell: (&str, i32), uses: i32) -> MerchantOffer {
        MerchantOffer {
            buy: ItemCost {
                id: buy.0.to_owned(),
                count: buy.1,
                components: None,
            },
            buy_b: None,
            sell: TradeItem {
                id: sell.0.to_owned(),
                count: sell.1,
                components: serde_json::Map::new(),
            },
            uses,
            max_uses: 12,
            reward_exp: true,
            special_price: 0,
            demand: 0,
            price_multiplier: 0.05,
            xp: 2,
        }
    }

    #[test]
    fn an_offer_crosses_the_network_with_its_costs_components_and_prices() {
        let mut book = offer(("minecraft:emerald", 10), ("minecraft:enchanted_book", 1), 3);
        book.buy_b = Some(ItemCost {
            id: "minecraft:book".to_owned(),
            count: 1,
            components: None,
        });
        book.sell.components.insert(
            "minecraft:stored_enchantments".to_owned(),
            serde_json::json!({"minecraft:mending": 1}),
        );
        book.buy.components = Some(serde_json::json!({"minecraft:custom_name": "Coin"}));
        book.special_price = -3;
        book.demand = 5;
        book.reward_exp = false;
        book.xp = 10;
        let back = offer_of(&net_offer(&book)).expect("the offer");
        assert_eq!(back, book);
        assert_eq!(back.cost_a_count(64), book.cost_a_count(64));
        let sold_out = offer(("minecraft:wheat", 20), ("minecraft:emerald", 1), 12);
        let back = offer_of(&net_offer(&sold_out)).expect("the offer");
        assert!(back.out_of_stock());
        assert_eq!(back, sold_out);
    }

    #[test]
    fn a_horse_inventory_turns_clicks_into_level_updates_and_a_friend_mirrors_it() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut alex = player(&mut chat, 1, "Alex");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let donkey = MountInventory {
            vehicle: 33,
            columns: 5,
            saddle: true,
            armor: false,
            llama: false,
            cells: vec![None; 17],
        };
        assert!(open(
            &mut alex,
            Open::Mount(donkey),
            Text::translate("entity.minecraft.donkey", Vec::new()),
            0
        ));
        alex.inventory.slots[0] = Some(ItemStack::new("minecraft:saddle", 1));
        alex.inventory.slots[9] = Some(ItemStack::new("minecraft:wheat", 16));
        let shift = |slot: u8| MenuAction::Slot {
            slot: MenuSlot::Inventory(slot),
            right: false,
            shift: true,
        };
        let _ = apply(&mut alex, &mut containers, books(&book), &shift(0), 0.5);
        let _ = apply(&mut alex, &mut containers, books(&book), &shift(9), 0.5);
        let _ = apply(
            &mut alex,
            &mut containers,
            books(&book),
            &MenuAction::Slot {
                slot: MenuSlot::Inventory(20),
                right: false,
                shift: false,
            },
            0.5,
        );
        let updates = std::mem::take(&mut alex.menus.mount_updates);
        assert_eq!(updates.len(), 2, "only the clicks that changed the donkey go to the level");
        assert!(updates[1].saddled() && updates[1].items()[0].as_ref().is_some_and(|s| s.count == 16));
        assert!(alex.inventory.slots[0].is_none() && alex.inventory.slots[9].is_none());
        let out = changes(&mut alex, &mut containers, books(&book));
        let views = opened_views(&out);
        let view = views.last().expect("the horse inventory is sent");
        assert_eq!((view.kind, view.slots.len()), (MenuKind::Mount, 17));
        assert_eq!(
            view.extras,
            MenuExtras::Mount {
                mount: 33,
                columns: 5,
                saddle: true,
                armor: false,
                llama: false
            }
        );
        let recipes = RecipeBook::default();
        let Some(Open::Mount(mirrored)) = mirror(Some(view), &recipes).open else {
            panic!("the friend mirrors the horse inventory");
        };
        assert_eq!((mirrored.vehicle, mirrored.columns, mirrored.cells.len()), (33, 5, 17));
        assert!(mirrored.saddled());
        assert!(close(&mut alex).is_empty());
        assert!(alex.menus.open.is_none());
    }

    #[test]
    fn a_horse_inventory_opens_for_one_player_at_a_time() {
        use minecraftoss_entities::cow::Cow;
        use minecraftoss_entities::horse::{HorseKind, HorseState};
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        let Ok(terrain) = crate::level::Terrain::open(
            std::sync::Arc::new(registries),
            0,
            2,
            Dimension::Overworld,
            None,
            None,
        ) else {
            return;
        };
        let data = GameData::default();
        let mut entities = Entities::new(
            &terrain.stream,
            Dimension::Overworld,
            0,
            &data,
            None,
            None,
        );
        let mut horse = HorseState::new(HorseKind::Donkey);
        horse.tame = true;
        horse.chest = true;
        horse.resize_inventory();
        let mut cow = Cow::new(glam::DVec3::new(0.5, 200.0, 0.5));
        cow.max_health = 20.0;
        cow.health = 20.0;
        let donkey = entities.view.world.spawn_horse(cow, horse, true);
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut steve = player(&mut chat, 0, "Steve");
        let mut alex = player(&mut chat, 1, "Alex");
        let eye = [2.5, 201.6, 0.5];
        entities.want_mount_menu(1, donkey);
        assert!(mount(&mut alex, &mut entities, eye, false, true).opened);
        assert_eq!(entities.mount_viewer(donkey), Some(1));
        entities.want_mount_menu(0, donkey);
        let refused = mount(&mut steve, &mut entities, eye, false, true);
        assert!(
            !refused.opened && steve.menus.open.is_none(),
            "a second player would get a copy of the same chest"
        );
        assert!(!mount(&mut alex, &mut entities, eye, false, true).closed);
        assert!(close(&mut alex).is_empty());
        let _ = mount(&mut alex, &mut entities, eye, false, true);
        assert_eq!(entities.mount_viewer(donkey), None, "closing lets it go");
        entities.want_mount_menu(0, donkey);
        assert!(mount(&mut steve, &mut entities, eye, false, true).opened);
        assert_eq!(entities.mount_viewer(donkey), Some(0));
        let mut inventory = Inventory::default();
        let _ = entities.leave(0, &mut inventory, 0, None);
        assert_eq!(
            entities.mount_viewer(donkey),
            None,
            "leaving the level lets it go"
        );
    }

    #[test]
    fn a_friend_s_trading_screen_is_sent_and_mirrored_with_its_offers() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut alex = player(&mut chat, 1, "Alex");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let mut view = trade_view(9);
        let mut cheap = offer(("minecraft:wheat", 20), ("minecraft:emerald", 1), 0);
        cheap.special_price = -4;
        view.offers = vec![
            cheap,
            offer(("minecraft:emerald", 1), ("minecraft:bread", 6), 12),
        ];
        view.level = 2;
        view.xp = 15;
        let shown = merchant_news(&mut alex, vec![MerchantNews::Opened(view.clone())], true, false);
        assert!(shown.opened, "a friend's screen opens on the host");
        let out = changes(&mut alex, &mut containers, books(&book));
        let views = opened_views(&out);
        let first = views.last().expect("the trading screen is sent");
        assert_eq!(first.kind, MenuKind::Merchant);
        assert_eq!(first.slots.len(), 3);
        let MenuExtras::Merchant {
            villager,
            profession,
            level,
            xp,
            offers,
            selected: None,
            ..
        } = &first.extras
        else {
            panic!("{:?}", first.extras);
        };
        assert_eq!((*villager, profession.as_str(), *level, *xp), (9, "minecraft:farmer", 2, 15));
        assert_eq!(offers.len(), 2);
        let recipes = RecipeBook::default();
        let Some(Open::Merchant(mirrored)) = mirror(Some(first), &recipes).open else {
            panic!("the friend mirrors a trading screen");
        };
        assert_eq!(mirrored.view.offers, view.offers);
        assert_eq!(mirrored.view.profession, "minecraft:farmer");
        assert_eq!((mirrored.view.level, mirrored.view.xp), (2, 15));
        assert!(mirrored.view.offers[1].out_of_stock());
        assert_eq!(mirrored.view.offers[0].cost_a_count(64), 16);
        let _ = apply(
            &mut alex,
            &mut containers,
            books(&book),
            &MenuAction::Choose(1),
            0.5,
        );
        assert_eq!(sent(&mut alex), [MerchantOp::Select(1)]);
        let mut paid = view.clone();
        paid.payment[0] = Some(ItemStack::new("minecraft:emerald", 1));
        paid.result = Some(ItemStack::new("minecraft:bread", 6));
        paid.future_xp = 17;
        let _ = merchant_news(&mut alex, vec![answer(Some(paid))], true, false);
        let out = changes(&mut alex, &mut containers, books(&book));
        let views = opened_views(&out);
        let resent = views.last().expect("new extras resend the screen");
        assert_eq!(resent.menu, first.menu);
        let Some(Open::Merchant(mirrored)) = mirror(Some(resent), &recipes).open else {
            panic!("still trading");
        };
        assert_eq!(mirrored.selected, Some(1));
        assert_eq!(mirrored.view.future_xp, 17);
        assert_eq!(
            mirrored.view.payment[0].as_ref().map(|stack| stack.id.as_str()),
            Some("minecraft:emerald")
        );
        assert_eq!(
            mirrored.view.result.as_ref().map(|stack| stack.count),
            Some(6)
        );
        let message = Click {
            menu: resent.menu,
            state: resent.state,
            action: MenuAction::Slot {
                slot: MenuSlot::Output,
                right: false,
                shift: true,
            },
        };
        let _ = click(&mut alex, &mut containers, books(&book), &message, 0.5);
        assert_eq!(
            sent(&mut alex),
            [MerchantOp::Result { shift: true }],
            "the friend's click reaches the same trade"
        );
        assert!(close_requested(&mut alex, resent.menu).is_empty());
        let _ = merchant_news(&mut alex, vec![answer(None)], true, false);
        assert_eq!(sent(&mut alex), [MerchantOp::Close]);
        let out = changes(&mut alex, &mut containers, books(&book));
        assert!(
            out.iter()
                .all(|message| !matches!(message, ToClient::MenuClose { .. })),
            "the friend closed it, so no close goes back"
        );
        let _ = merchant_news(&mut alex, vec![MerchantNews::Opened(view)], true, false);
        let out = changes(&mut alex, &mut containers, books(&book));
        let again = opened_views(&out).last().map(|view| view.menu).expect("reopened");
        let shown = merchant_news(&mut alex, vec![MerchantNews::Closing], true, false);
        assert!(shown.closed);
        let out = changes(&mut alex, &mut containers, books(&book));
        assert!(out.contains(&ToClient::MenuClose { menu: again }), "{out:?}");
    }

    #[test]
    fn killstreak_items_cannot_go_into_a_chest() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let killstreak = crate::guns::weapon_item(12);
        host.inventory.slots[0] = Some(killstreak.clone());
        host.inventory.slots[1] = Some(ItemStack::new("minecraft:dirt", 4));
        assert!(open(
            &mut host,
            Open::Block(chest()),
            Text::literal("Chest"),
            0
        ));
        let mut act = |host: &mut Player, action: MenuAction| {
            apply(host, &mut containers, books(&book), &action, 0.5)
        };
        act(&mut host, slot(0, true));
        assert_eq!(host.inventory.slots[0], Some(killstreak.clone()));
        let swap = MenuAction::Swap {
            slot: MenuSlot::Container(3),
            hotbar: 0,
        };
        act(&mut host, swap);
        assert_eq!(host.inventory.slots[0], Some(killstreak.clone()));
        act(&mut host, slot(0, false));
        assert_eq!(host.inventory.cursor, Some(killstreak.clone()));
        act(&mut host, cell(5));
        let spread = MenuAction::Spread {
            slots: vec![MenuSlot::Container(6), MenuSlot::Container(7)],
            right: false,
        };
        act(&mut host, spread);
        let dropped = act(&mut host, MenuAction::Outside { right: false });
        assert!(dropped.thrown.is_empty(), "the cursor drop keeps it");
        assert_eq!(host.inventory.cursor, Some(killstreak.clone()));
        act(&mut host, slot(0, false));
        assert_eq!(host.inventory.slots[0], Some(killstreak));
        act(&mut host, slot(1, true));
        assert!(host.inventory.slots[1].is_none(), "other items still go in");
        let cells = containers.cells(&chest());
        let held: Vec<&str> = cells.iter().flatten().map(|s| s.id.as_str()).collect();
        assert_eq!(held, vec!["minecraft:dirt"]);
    }

    #[test]
    fn fork_guns_drop_with_q_go_into_chests_and_stay_out_of_the_offhand() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let mut m4 = ItemStack::new("iw4l:m4", 1);
        m4.components = Some(serde_json::json!({"minecraft:custom_data": {"iw4l": {"clip": 17}}}));
        let usp = ItemStack::new("iw4l:usp", 1);
        let shield = ItemStack::new("iw4l:riotshield", 1);
        host.inventory.slots[0] = Some(m4.clone());
        host.inventory.slots[1] = Some(usp.clone());
        host.inventory.slots[2] = Some(shield.clone());
        let mut act = |host: &mut Player, action: MenuAction| {
            apply(host, &mut containers, books(&book), &action, 0.5)
        };
        let offhand = MenuSlot::Inventory(OFFHAND as u8);
        act(&mut host, slot(1, false));
        act(&mut host, MenuAction::Slot { slot: offhand, right: false, shift: false });
        assert_eq!(host.inventory.cursor, Some(usp.clone()));
        let spread = MenuAction::Spread {
            slots: vec![offhand, MenuSlot::Inventory(20)],
            right: true,
        };
        act(&mut host, spread);
        assert_eq!(host.inventory.cursor, Some(usp.clone()));
        act(&mut host, slot(1, false));
        act(&mut host, MenuAction::Swap { slot: MenuSlot::Inventory(2), hotbar: OFFHAND as u8 });
        act(&mut host, MenuAction::Swap { slot: offhand, hotbar: 2 });
        assert_eq!(host.inventory.slots[2], Some(shield));
        assert!(host.inventory.slots[OFFHAND].is_none());
        let thrown = act(&mut host, MenuAction::Throw { slot: MenuSlot::Inventory(0), whole: false });
        assert_eq!(thrown.thrown, vec![m4]);
        assert!(host.inventory.slots[0].is_none());
        assert!(open(&mut host, Open::Block(chest()), Text::literal("Chest"), 0));
        let mut act = |host: &mut Player, action: MenuAction| {
            apply(host, &mut containers, books(&book), &action, 0.5)
        };
        act(&mut host, slot(1, true));
        assert!(host.inventory.slots[1].is_none());
        act(&mut host, slot(2, false));
        let dropped = act(&mut host, MenuAction::Outside { right: false });
        assert_eq!(dropped.thrown.len(), 1);
        assert!(host.inventory.cursor.is_none());
        let cells = containers.cells(&chest());
        assert_eq!(cells.iter().flatten().cloned().collect::<Vec<_>>(), vec![usp]);
    }

    fn held_at(player: &Player, slot: usize) -> Option<(&str, u8)> {
        player.inventory.slots[slot]
            .as_ref()
            .map(|stack| (stack.id.as_str(), stack.count))
    }

    fn carried(player: &Player) -> Option<(&str, u8)> {
        player
            .inventory
            .cursor
            .as_ref()
            .map(|stack| (stack.id.as_str(), stack.count))
    }

    fn put_with_cursor(host: &mut Player, slot: u8, id: &str) {
        let book = StationBook::default();
        let mut containers = Containers::default();
        host.inventory.cursor = Some(ItemStack::new(id, 10));
        let action = MenuAction::Slot {
            slot: MenuSlot::Inventory(slot),
            right: false,
            shift: false,
        };
        apply(host, &mut containers, books(&book), &action, 0.5);
    }

    #[test]
    fn only_a_lethal_grenade_goes_in_the_lethal_slot() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        for refused in ["minecraft:dirt", "iw4l:flash_grenade", "iw4l:m4", "iw4:weapon/7"] {
            put_with_cursor(&mut host, 43, refused);
            assert_eq!(held_at(&host, 43), None, "{refused}");
            assert_eq!(carried(&host), Some((refused, 10)));
        }
        for lethal in ["iw4l:semtex", "iw4l:throwing_knife", "iw4l:claymore", "iw4l:c4", "iw4l:tactical_insertion"] {
            host.inventory.slots[43] = None;
            put_with_cursor(&mut host, 43, lethal);
            assert_eq!(held_at(&host, 43), Some((lethal, 10)));
            assert_eq!(carried(&host), None);
        }
        assert!(crate::guns::fits(43, &ItemStack::new("iw4l:frag_grenade", 64)));
        assert!(!crate::guns::fits(43, &ItemStack::new("iw4l:frag_grenade", 65)));
        assert!(crate::guns::fits(20, &ItemStack::new("minecraft:dirt", 64)));
    }

    #[test]
    fn only_a_tactical_goes_in_the_tactical_slot() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        for refused in ["minecraft:dirt", "iw4l:frag_grenade", "iw4l:rifle_ammo", "iw4l:tactical_insertion"] {
            put_with_cursor(&mut host, 44, refused);
            assert_eq!(held_at(&host, 44), None, "{refused}");
            assert_eq!(carried(&host), Some((refused, 10)));
        }
        for tactical in [
            "iw4l:flash_grenade",
            "iw4l:concussion_grenade",
            "iw4l:smoke_grenade",
        ] {
            host.inventory.slots[44] = None;
            put_with_cursor(&mut host, 44, tactical);
            assert_eq!(held_at(&host, 44), Some((tactical, 10)));
        }
        for body in [41, 42] {
            put_with_cursor(&mut host, body, "minecraft:saddle");
            assert_eq!(held_at(&host, usize::from(body)), None);
        }
    }

    #[test]
    fn shift_click_moves_a_grenade_to_its_slot_and_back_out() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let mut act = |host: &mut Player, action: MenuAction| {
            apply(host, &mut containers, books(&book), &action, 0.5);
        };
        host.inventory.slots[20] = Some(ItemStack::new("iw4l:semtex", 40));
        host.inventory.slots[21] = Some(ItemStack::new("iw4l:semtex", 40));
        host.inventory.slots[3] = Some(ItemStack::new("iw4l:flash_grenade", 7));
        act(&mut host, slot(20, true));
        assert_eq!(held_at(&host, 43), Some(("iw4l:semtex", 40)));
        assert_eq!(held_at(&host, 20), None);
        act(&mut host, slot(21, true));
        assert_eq!(held_at(&host, 43), Some(("iw4l:semtex", 64)));
        assert_eq!(held_at(&host, 21), Some(("iw4l:semtex", 16)));
        act(&mut host, slot(21, true));
        assert_eq!(held_at(&host, 0), Some(("iw4l:semtex", 16)), "a full slot");
        act(&mut host, slot(3, true));
        assert_eq!(held_at(&host, 44), Some(("iw4l:flash_grenade", 7)));
        host.inventory.slots[0] = None;
        host.inventory.slots[30] = Some(ItemStack::new("iw4l:frag_grenade", 5));
        act(&mut host, slot(30, true));
        assert_eq!(held_at(&host, 0), Some(("iw4l:frag_grenade", 5)), "another lethal");
        assert_eq!(held_at(&host, 43), Some(("iw4l:semtex", 64)));
        act(&mut host, slot(43, true));
        assert_eq!(held_at(&host, 43), None);
        assert_eq!(held_at(&host, 9), Some(("iw4l:semtex", 64)));
        for main in 9..36 {
            host.inventory.slots[main].get_or_insert(ItemStack::new("minecraft:stone", 64));
        }
        act(&mut host, slot(44, true));
        assert_eq!(held_at(&host, 44), None);
        assert_eq!(held_at(&host, 1), Some(("iw4l:flash_grenade", 7)));
        host.inventory.slots[22] = Some(ItemStack::new("iw4l:smoke_grenade", 3));
        assert!(open(&mut host, Open::Block(chest()), Text::literal("Chest"), 0));
        let mut act = |host: &mut Player, action: MenuAction| {
            apply(host, &mut containers, books(&book), &action, 0.5);
        };
        act(&mut host, slot(22, true));
        assert_eq!(held_at(&host, 44), None, "a chest takes it instead");
        assert_eq!(held_at(&host, 22), None);
    }

    #[test]
    fn a_number_key_cannot_swap_dirt_into_a_grenade_slot() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let mut act = |host: &mut Player, slot: u8, hotbar: u8| {
            let action = MenuAction::Swap {
                slot: MenuSlot::Inventory(slot),
                hotbar,
            };
            apply(host, &mut containers, books(&book), &action, 0.5);
        };
        host.inventory.slots[2] = Some(ItemStack::new("minecraft:dirt", 3));
        act(&mut host, 43, 2);
        assert_eq!(held_at(&host, 43), None);
        assert_eq!(held_at(&host, 2), Some(("minecraft:dirt", 3)));
        host.inventory.slots[43] = Some(ItemStack::new("iw4l:frag_grenade", 9));
        act(&mut host, 43, 2);
        assert_eq!(held_at(&host, 43), Some(("iw4l:frag_grenade", 9)));
        host.inventory.slots[2] = Some(ItemStack::new("iw4l:semtex", 4));
        act(&mut host, 43, 2);
        assert_eq!(held_at(&host, 43), Some(("iw4l:semtex", 4)));
        assert_eq!(held_at(&host, 2), Some(("iw4l:frag_grenade", 9)));
        host.inventory.slots[2] = None;
        act(&mut host, 43, 2);
        assert_eq!(held_at(&host, 43), None);
        assert_eq!(held_at(&host, 2), Some(("iw4l:semtex", 4)));
        host.inventory.slots[OFFHAND] = Some(ItemStack::new("iw4l:smoke_grenade", 2));
        act(&mut host, 44, OFFHAND as u8);
        assert_eq!(held_at(&host, 44), Some(("iw4l:smoke_grenade", 2)));
        assert_eq!(held_at(&host, OFFHAND), None);
    }

    #[test]
    fn a_drag_spreads_only_into_grenade_slots_that_fit() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let spread = MenuAction::Spread {
            slots: vec![
                MenuSlot::Inventory(43),
                MenuSlot::Inventory(44),
                MenuSlot::Inventory(20),
            ],
            right: false,
        };
        host.inventory.cursor = Some(ItemStack::new("minecraft:dirt", 10));
        apply(&mut host, &mut containers, books(&book), &spread, 0.5);
        assert_eq!(held_at(&host, 20), Some(("minecraft:dirt", 10)));
        assert_eq!((held_at(&host, 43), held_at(&host, 44)), (None, None));
        host.inventory.slots[20] = None;
        host.inventory.cursor = Some(ItemStack::new("iw4l:frag_grenade", 10));
        apply(&mut host, &mut containers, books(&book), &spread, 0.5);
        assert_eq!(held_at(&host, 43), Some(("iw4l:frag_grenade", 5)));
        assert_eq!(held_at(&host, 20), Some(("iw4l:frag_grenade", 5)));
        assert_eq!((held_at(&host, 44), carried(&host)), (None, None));
    }

    #[test]
    fn a_friends_click_on_slot_44_is_valid() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = player(&mut chat, 1, "Alex");
        let book = StationBook::default();
        let mut containers = Containers::default();
        let _ = changes(&mut friend, &mut containers, books(&book));
        friend.inventory.cursor = Some(ItemStack::new("iw4l:flash_grenade", 12));
        let menu = (INVENTORY_MENU, friend.menus.current().state);
        friend_click(&mut friend, &mut containers, &book, menu, slot(44, false));
        assert_eq!(held_at(&friend, 44), Some(("iw4l:flash_grenade", 12)));
        assert!(!friend.menus.current().resends());
        friend.inventory.cursor = Some(ItemStack::new("minecraft:dirt", 1));
        let menu = (INVENTORY_MENU, friend.menus.current().state);
        friend_click(&mut friend, &mut containers, &book, menu, slot(45, false));
        assert!(friend.menus.current().resends(), "no slot 45");
        assert_eq!(carried(&friend), Some(("minecraft:dirt", 1)));
    }

    fn creative_player(chat: &mut CommandServer, mode: GameMode) -> Player {
        let mut friend = player(chat, 1, "Alex");
        friend.mode = PlayerMode::new(mode);
        friend
    }

    fn known(id: &str) -> bool {
        id.starts_with("minecraft:") && id != "minecraft:nope"
    }

    fn nbt(id: &str, count: u8) -> Option<Nbt> {
        let mut stack = ItemStack::new(id, count);
        if id.ends_with("_sword") {
            stack.components = Some(serde_json::json!({"minecraft:max_stack_size": 1}));
        }
        slot_nbt(&stack)
    }

    fn set(slot: CreativeTarget, stack: Option<Nbt>) -> MenuInput {
        MenuInput::Creative { slot, stack }
    }

    fn take(friend: &mut Player, containers: &mut Containers, inputs: Vec<MenuInput>) -> Clicked {
        let book = StationBook::default();
        take_inputs(
            friend,
            containers,
            books(&book),
            inputs,
            None,
            &known,
            || 0.5,
        )
    }

    fn held(slot: &Option<ItemStack>) -> Option<(&str, u8)> {
        slot.as_ref().map(|stack| (stack.id.as_str(), stack.count))
    }

    #[test]
    fn a_creative_friends_slot_set_reaches_their_inventory() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = creative_player(&mut chat, GameMode::Creative);
        let mut containers = Containers::default();
        let book = StationBook::default();
        assert!(!changes(&mut friend, &mut containers, books(&book)).is_empty());
        let clicked = take(
            &mut friend,
            &mut containers,
            vec![
                set(CreativeTarget::Menu(36), nbt("minecraft:stone", 64)),
                set(CreativeTarget::Menu(5), nbt("minecraft:diamond_helmet", 1)),
            ],
        );
        assert_eq!(clicked, Clicked::default());
        assert_eq!(held(&friend.inventory.slots[0]), Some(("minecraft:stone", 64)));
        assert_eq!(
            held(&friend.inventory.slots[39]),
            Some(("minecraft:diamond_helmet", 1))
        );
        changes(&mut friend, &mut containers, books(&book));
        take(
            &mut friend,
            &mut containers,
            vec![set(CreativeTarget::Carried, nbt("minecraft:torch", 16))],
        );
        let sent = changes(&mut friend, &mut containers, books(&book));
        let [
            ToClient::MenuSlots {
                menu: INVENTORY_MENU,
                carried: Some(carried),
                ..
            },
        ] = &sent[..]
        else {
            panic!("{sent:?}");
        };
        assert_eq!(shown(carried), Some(("minecraft:torch".to_owned(), 16)));
    }

    #[test]
    fn a_survival_or_spectator_friends_creative_sets_are_refused() {
        for mode in [GameMode::Survival, GameMode::Adventure, GameMode::Spectator] {
            let mut chat = CommandServer::new(crate::commands::HOST);
            let mut friend = creative_player(&mut chat, mode);
            let mut containers = Containers::default();
            let clicked = take(
                &mut friend,
                &mut containers,
                vec![
                    set(CreativeTarget::Menu(36), nbt("minecraft:stone", 64)),
                    set(CreativeTarget::Carried, nbt("minecraft:stone", 1)),
                    set(CreativeTarget::Drop, nbt("minecraft:stone", 1)),
                ],
            );
            assert!(clicked.thrown.is_empty(), "{mode:?}");
            assert!(friend.inventory.slots.iter().all(Option::is_none), "{mode:?}");
            assert!(friend.inventory.cursor.is_none(), "{mode:?}");
        }
    }

    #[test]
    fn creative_sets_refuse_unknown_items_bad_slots_and_overfull_stacks() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = creative_player(&mut chat, GameMode::Creative);
        let mut containers = Containers::default();
        let weapon = slot_nbt(&crate::guns::weapon_item(7));
        assert!(weapon.is_some());
        let clicked = take(
            &mut friend,
            &mut containers,
            vec![
                set(CreativeTarget::Menu(36), nbt("minecraft:nope", 1)),
                set(CreativeTarget::Menu(0), nbt("minecraft:stone", 1)),
                set(CreativeTarget::Menu(46), nbt("minecraft:stone", 1)),
                set(CreativeTarget::Menu(37), nbt("minecraft:stone", 65)),
                set(CreativeTarget::Menu(38), nbt("minecraft:diamond_sword", 2)),
                set(CreativeTarget::Menu(39), weapon),
                set(CreativeTarget::Carried, Some(Nbt(vec![10, 0]))),
            ],
        );
        assert!(clicked.thrown.is_empty());
        assert!(friend.inventory.slots.iter().all(Option::is_none));
        assert!(friend.inventory.crafting.iter().all(Option::is_none));
        assert!(friend.inventory.cursor.is_none());
        friend.inventory.slots[2] = Some(ItemStack::new("minecraft:stone", 5));
        take(
            &mut friend,
            &mut containers,
            vec![
                set(CreativeTarget::Menu(38), Some(Nbt(vec![1, 2, 3]))),
                set(CreativeTarget::Menu(39), nbt("minecraft:diamond_sword", 1)),
            ],
        );
        assert_eq!(held(&friend.inventory.slots[2]), Some(("minecraft:stone", 5)));
        assert_eq!(
            held(&friend.inventory.slots[3]),
            Some(("minecraft:diamond_sword", 1))
        );
        take(
            &mut friend,
            &mut containers,
            vec![set(CreativeTarget::Menu(38), None)],
        );
        assert!(friend.inventory.slots[2].is_none());
    }

    #[test]
    fn a_creative_drop_is_thrown_not_stored() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = creative_player(&mut chat, GameMode::Creative);
        let mut containers = Containers::default();
        let clicked = take(
            &mut friend,
            &mut containers,
            vec![
                set(CreativeTarget::Drop, nbt("minecraft:cobblestone", 64)),
                set(CreativeTarget::Drop, None),
            ],
        );
        assert_eq!(
            clicked
                .thrown
                .iter()
                .map(|stack| (stack.id.as_str(), stack.count))
                .collect::<Vec<_>>(),
            vec![("minecraft:cobblestone", 64)]
        );
        assert!(friend.inventory.slots.iter().all(Option::is_none));
        assert!(friend.inventory.cursor.is_none());
    }

    #[test]
    fn creative_drops_are_throttled_like_vanilla() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = creative_player(&mut chat, GameMode::Creative);
        let mut containers = Containers::default();
        let drops = |count: usize| {
            (0..count)
                .map(|_| set(CreativeTarget::Drop, nbt("minecraft:cobblestone", 1)))
                .collect::<Vec<_>>()
        };
        let clicked = take(&mut friend, &mut containers, drops(75));
        assert_eq!(clicked.thrown.len(), 74);
        assert_eq!(friend.drop_throttle.count, 1480);
        friend.drop_throttle.tick(1);
        assert_eq!(take(&mut friend, &mut containers, drops(2)).thrown.len(), 1);
        assert_eq!(friend.drop_throttle.count, 1499);
        friend.drop_throttle.tick(19);
        assert!(take(&mut friend, &mut containers, drops(1)).thrown.is_empty());
        friend.drop_throttle.tick(1);
        assert_eq!(take(&mut friend, &mut containers, drops(2)).thrown.len(), 1);
        let mut throttle = crate::creative::DropThrottle { count: 5 };
        throttle.tick(9);
        assert_eq!(throttle.count, 0);
        let clicked = take(
            &mut friend,
            &mut containers,
            vec![set(CreativeTarget::Menu(36), nbt("minecraft:stone", 64))],
        );
        assert!(clicked.thrown.is_empty());
        assert_eq!(held(&friend.inventory.slots[0]), Some(("minecraft:stone", 64)));
    }

    #[test]
    fn creative_sets_and_clicks_apply_in_the_order_sent() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = creative_player(&mut chat, GameMode::Creative);
        let mut containers = Containers::default();
        let book = StationBook::default();
        changes(&mut friend, &mut containers, books(&book));
        let state = friend.menus.current().state;
        take(
            &mut friend,
            &mut containers,
            vec![
                set(CreativeTarget::Carried, nbt("minecraft:torch", 16)),
                MenuInput::Click(Click {
                    menu: INVENTORY_MENU,
                    state,
                    action: slot(3, false),
                }),
            ],
        );
        assert_eq!(held(&friend.inventory.slots[3]), Some(("minecraft:torch", 16)));
        assert!(friend.inventory.cursor.is_none());
    }

    #[test]
    fn a_dead_friends_creative_set_is_ignored() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut friend = creative_player(&mut chat, GameMode::Creative);
        let mut containers = Containers::default();
        friend.dead = true;
        let clicked = take(
            &mut friend,
            &mut containers,
            vec![
                set(CreativeTarget::Menu(36), nbt("minecraft:stone", 64)),
                set(CreativeTarget::Drop, nbt("minecraft:stone", 64)),
            ],
        );
        assert!(clicked.thrown.is_empty());
        assert!(friend.inventory.slots[0].is_none());
    }
}
