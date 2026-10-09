use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{json, Value};

use minecraftoss_player::crafting::{CookingKind, RecipeBook};
use minecraftoss_player::furnace::Furnace;
use minecraftoss_player::inventory::{Inventory, ItemStack};

use crate::campfire::Campfire;
use crate::crafting::{move_into, result_order};
use crate::scene::{Block, BlockPos};
use crate::stations::StationBook;

pub const CHEST_SLOTS: usize = 27;
const HOTBAR: usize = 9;
const OFFHAND: usize = 40;
const STORAGE: usize = 36;
const INPUT: usize = 0;
const FUEL: usize = 1;
const OUTPUT: usize = 2;
const BREWER_SLOTS: usize = 5;
const BOTTLES: usize = 3;
const REAGENT: usize = 3;
const BLAZE: usize = 4;
const BREW_TICKS: u32 = 400;
const FUEL_USES: u32 = 20;
pub const CONTAINER: &str = "minecraft:container";
const CUSTOM_NAME: &str = "minecraft:custom_name";
const COPPER_CHESTS: [&str; 4] = [
    "copper_chest",
    "exposed_copper_chest",
    "weathered_copper_chest",
    "oxidized_copper_chest",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Station {
    Chest,
    Barrel,
    Oven(CookingKind),
    Brewer,
    Ender,
    Shulker,
}

impl Station {
    pub fn of(block: &Block) -> Option<Self> {
        if block.id.namespace != "minecraft" {
            return None;
        }
        match block.id.path.as_str() {
            "chest" | "trapped_chest" => Some(Self::Chest),
            path if COPPER_CHESTS.contains(&path.strip_prefix("waxed_").unwrap_or(path)) => {
                Some(Self::Chest)
            }
            "ender_chest" => Some(Self::Ender),
            path if is_shulker_box(path) => Some(Self::Shulker),
            "barrel" => Some(Self::Barrel),
            "furnace" => Some(Self::Oven(CookingKind::Furnace)),
            "smoker" => Some(Self::Oven(CookingKind::Smoker)),
            "blast_furnace" => Some(Self::Oven(CookingKind::BlastFurnace)),
            "brewing_stand" => Some(Self::Brewer),
            _ => None,
        }
    }

    pub fn title(self, double: bool) -> &'static str {
        match self {
            Self::Chest if double => "LARGE CHEST",
            Self::Chest => "CHEST",
            Self::Barrel => "BARREL",
            Self::Oven(CookingKind::Furnace) => "FURNACE",
            Self::Oven(CookingKind::Smoker) => "SMOKER",
            Self::Oven(CookingKind::BlastFurnace) => "BLAST FURNACE",
            Self::Brewer => "BREWING STAND",
            Self::Ender => "ENDER CHEST",
            Self::Shulker => "SHULKER BOX",
        }
    }
}

pub fn is_shulker_box(id: &str) -> bool {
    let path = id.strip_prefix("minecraft:").unwrap_or(id);
    path == "shulker_box" || path.ends_with("_shulker_box")
}

pub fn chest_sound(path: &str, open: bool) -> &'static str {
    let copper = COPPER_CHESTS
        .iter()
        .position(|chest| *chest == path.strip_prefix("waxed_").unwrap_or(path));
    match (copper, open) {
        (Some(2), true) => "minecraft:block.copper_chest_weathered.open",
        (Some(2), false) => "minecraft:block.copper_chest_weathered.close",
        (Some(3), true) => "minecraft:block.copper_chest_oxidized.open",
        (Some(3), false) => "minecraft:block.copper_chest_oxidized.close",
        (Some(_), true) => "minecraft:block.copper_chest.open",
        (Some(_), false) => "minecraft:block.copper_chest.close",
        (None, true) => "minecraft:block.chest.open",
        (None, false) => "minecraft:block.chest.close",
    }
}

pub fn lid_cell(pos: BlockPos, facing: &str) -> Option<BlockPos> {
    let (dx, dy, dz) = match facing {
        "up" => (0, 1, 0),
        "down" => (0, -1, 0),
        "north" => (0, 0, -1),
        "south" => (0, 0, 1),
        "west" => (-1, 0, 0),
        "east" => (1, 0, 0),
        _ => return None,
    };
    Some((pos.0 + dx, pos.1 + dy, pos.2 + dz))
}

pub fn lid_blocked(facing: &str, neighbour: &[[f64; 6]]) -> bool {
    let mut lid = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    match facing {
        "up" => lid[4] = 0.5,
        "down" => lid[1] = 0.5,
        "north" => lid[2] = 0.5,
        "south" => lid[5] = 0.5,
        "west" => lid[0] = 0.5,
        "east" => lid[3] = 0.5,
        _ => return false,
    }
    let lid: [f64; 6] = std::array::from_fn(|k| lid[k] + if k < 3 { 1e-6 } else { -1e-6 });
    neighbour
        .iter()
        .any(|b| (0..3).all(|k| b[k] < lid[k + 3] && lid[k] < b[k + 3]))
}

pub fn lid_obstructed(pos: BlockPos, facing: &str, boxes_at: impl Fn(BlockPos) -> Vec<[f64; 6]>) -> bool {
    let Some(lid) = lid_cell(pos, facing) else {
        return false;
    };
    let mut boxes = boxes_at(lid);
    boxes.extend(
        boxes_at((lid.0, lid.1 - 1, lid.2))
            .into_iter()
            .map(|b| [b[0], b[1] - 1.0, b[2], b[3], b[4] - 1.0, b[5]]),
    );
    lid_blocked(facing, &boxes)
}

pub fn stored_items(stack: &ItemStack) -> Vec<(usize, ItemStack)> {
    let Some(list) = stack
        .components
        .as_ref()
        .and_then(|components| components.get(CONTAINER))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let slot = usize::try_from(entry.get("slot")?.as_u64()?).ok()?;
            let item = entry.get("item")?;
            let count = item.get("count").map_or(Some(1), Value::as_u64)?;
            let mut stored = ItemStack::new(item.get("id")?.as_str()?, count.clamp(1, 255) as u8);
            stored.components = item.get("components").cloned();
            Some((slot, stored))
        })
        .collect()
}

fn container_component(cells: &[Option<ItemStack>]) -> Value {
    Value::Array(
        cells
            .iter()
            .enumerate()
            .filter_map(|(slot, cell)| {
                let stack = cell.as_ref()?;
                let mut item = json!({"id": stack.id, "count": stack.count});
                if let Some(components) = &stack.components {
                    item["components"] = components.clone();
                }
                Some(json!({"slot": slot, "item": item}))
            })
            .collect(),
    )
}

fn property<'a>(block: &'a Block, name: &str) -> Option<&'a str> {
    block.properties.get(name).map(String::as_str)
}

pub fn chest_partner(pos: BlockPos, block: &Block) -> Option<BlockPos> {
    let (dx, dz) = match (property(block, "facing")?, property(block, "type")?) {
        ("north", "left") | ("south", "right") => (1, 0),
        ("north", "right") | ("south", "left") => (-1, 0),
        ("east", "left") | ("west", "right") => (0, 1),
        ("east", "right") | ("west", "left") => (0, -1),
        _ => return None,
    };
    Some((pos.0 + dx, pos.1, pos.2 + dz))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    pub station: Station,
    pub cells: Vec<BlockPos>,
}

impl Opened {
    pub fn new(pos: BlockPos, block: &Block, partner: Option<&Block>) -> Option<Self> {
        let station = Station::of(block)?;
        let mut cells = vec![pos];
        if let (Station::Chest, Some(other), Some(at)) =
            (station, partner, chest_partner(pos, block))
        {
            let paired = other.id == block.id
                && property(other, "facing") == property(block, "facing")
                && chest_partner(at, other) == Some(pos);
            if paired && property(block, "type") == Some("right") {
                cells.push(at);
            } else if paired {
                cells.insert(0, at);
            }
        }
        Some(Self { station, cells })
    }

    pub fn rows(&self) -> usize {
        match self.station {
            Station::Oven(_) | Station::Brewer => 0,
            _ => 3 * self.cells.len(),
        }
    }

    pub fn size(&self) -> usize {
        match self.station {
            Station::Oven(_) => 3,
            Station::Brewer => BREWER_SLOTS,
            _ => CHEST_SLOTS * self.cells.len(),
        }
    }

    pub fn contains(&self, pos: BlockPos) -> bool {
        self.cells.contains(&pos)
    }
}

#[derive(Clone, Debug)]
pub struct Oven {
    pub furnace: Furnace,
    pub experience: f32,
}

fn cooking_ticks(base: u32, speed: f32) -> u32 {
    if speed > 0.0 {
        (base as f32 / speed).ceil() as u32
    } else {
        base
    }
}

impl Oven {
    pub fn new(kind: CookingKind) -> Self {
        Self {
            furnace: Furnace::new(kind),
            experience: 0.0,
        }
    }

    pub fn is_lit(&self) -> bool {
        self.furnace.is_lit()
    }

    pub fn tick(&mut self, recipes: &RecipeBook) -> bool {
        let lit = self.is_lit();
        let input = self.furnace.slots[INPUT].clone();
        let done = self.furnace.completed_recipes;
        self.furnace.tick(recipes);
        if self.furnace.completed_recipes > done {
            let kind = self.furnace.kind;
            if let Some(recipe) = input
                .as_ref()
                .and_then(|input| recipes.cooking_for(kind, input))
            {
                self.experience += recipe.experience;
            }
        }
        lit != self.is_lit()
    }

    fn refresh(&mut self, before: &Option<ItemStack>, recipes: &RecipeBook) {
        let same = match (before, &self.furnace.slots[INPUT]) {
            (Some(a), Some(b)) => a.same_item(b),
            _ => false,
        };
        if same {
            return;
        }
        let furnace = &mut self.furnace;
        furnace.cook_progress = 0;
        furnace.cook_total = furnace.slots[INPUT]
            .as_ref()
            .and_then(|stack| recipes.cooking_for(furnace.kind, stack))
            .map_or(0, |recipe| {
                cooking_ticks(recipe.cooking_ticks, furnace.speed_multiplier)
            });
    }

    pub fn take_experience(&mut self, roll: f32) -> u32 {
        let total = std::mem::take(&mut self.experience);
        let whole = total.floor();
        let part = total - whole;
        whole as u32 + u32::from(part > 0.0 && roll < part)
    }

    pub fn progress(&self) -> [f32; 2] {
        let f = &self.furnace;
        let lit = if f.lit_remaining > 0 && f.lit_total > 0 {
            f.lit_remaining as f32 / f.lit_total as f32
        } else {
            0.0
        };
        let cook = if f.cook_total > 0 {
            (f.cook_progress as f32 / f.cook_total as f32).min(1.0)
        } else {
            0.0
        };
        [lit, cook]
    }

    pub fn data(&self) -> [i32; 4] {
        let f = &self.furnace;
        [f.lit_remaining, f.lit_total, f.cook_progress, f.cook_total].map(|n| n.min(i32::MAX as u32) as i32)
    }
}

fn shrink(cell: &mut Option<ItemStack>, by: u8) {
    if let Some(stack) = cell.as_mut() {
        stack.count = stack.count.saturating_sub(by);
        if stack.count == 0 {
            *cell = None;
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Brewer {
    pub slots: [Option<ItemStack>; BREWER_SLOTS],
    pub brew_time: u32,
    pub fuel: u32,
    ingredient: Option<String>,
    shown: [bool; BOTTLES],
}

impl Brewer {
    fn brewable(&self, book: &StationBook, recipes: &RecipeBook) -> bool {
        let Some(reagent) = self.slots[REAGENT].as_ref() else {
            return false;
        };
        book.is_reagent(&reagent.id, recipes)
            && self.slots[..BOTTLES]
                .iter()
                .flatten()
                .any(|bottle| book.brew(bottle, &reagent.id, recipes).is_some())
    }

    pub fn tick(&mut self, book: &StationBook, recipes: &RecipeBook) -> (bool, Option<ItemStack>) {
        let blaze = self.slots[BLAZE]
            .as_ref()
            .is_some_and(|s| s.id == "minecraft:blaze_powder");
        if self.fuel == 0 && blaze {
            self.fuel = FUEL_USES;
            shrink(&mut self.slots[BLAZE], 1);
        }
        let brewable = self.brewable(book, recipes);
        let reagent = self.slots[REAGENT].as_ref().map(|s| s.id.clone());
        if self.brew_time > 0 {
            self.brew_time -= 1;
            if self.brew_time == 0 && brewable {
                return (true, self.brew(book, recipes));
            }
            if !brewable || reagent != self.ingredient {
                self.brew_time = 0;
            }
        } else if brewable && self.fuel > 0 {
            self.fuel -= 1;
            self.brew_time = BREW_TICKS;
            self.ingredient = reagent;
        }
        (false, None)
    }

    fn brew(&mut self, book: &StationBook, recipes: &RecipeBook) -> Option<ItemStack> {
        let reagent = self.slots[REAGENT].clone()?;
        for bottle in &mut self.slots[..BOTTLES] {
            if let Some(next) = bottle
                .as_ref()
                .and_then(|b| book.brew(b, &reagent.id, recipes))
            {
                *bottle = Some(next);
            }
        }
        shrink(&mut self.slots[REAGENT], 1);
        let rest = recipes.crafting_remainder(&reagent.id)?;
        if self.slots[REAGENT].is_none() {
            self.slots[REAGENT] = Some(rest);
            None
        } else {
            Some(rest)
        }
    }

    pub fn bottles(&self) -> [bool; BOTTLES] {
        std::array::from_fn(|i| self.slots[i].is_some())
    }

    pub fn progress(&self) -> [f32; 2] {
        let brewing = if self.brew_time > 0 {
            (BREW_TICKS - self.brew_time) as f32 / BREW_TICKS as f32
        } else {
            0.0
        };
        [self.fuel as f32 / FUEL_USES as f32, brewing]
    }

    pub fn data(&self) -> [i32; 4] {
        [self.brew_time as i32, self.fuel as i32, BREW_TICKS as i32, FUEL_USES as i32]
    }
}

#[derive(Default)]
pub struct Containers {
    chests: HashMap<BlockPos, Vec<Option<ItemStack>>>,
    ender: Vec<Option<ItemStack>>,
    names: HashMap<BlockPos, Value>,
    ovens: HashMap<BlockPos, Oven>,
    brewers: HashMap<BlockPos, Brewer>,
    campfires: HashMap<BlockPos, Campfire>,
    book: Arc<StationBook>,
}

#[derive(Default)]
pub struct Ticked {
    pub props: Vec<(BlockPos, &'static str, bool)>,
    pub brewed: Vec<BlockPos>,
    pub drops: Vec<(BlockPos, ItemStack)>,
}

pub struct Changed {
    pub experience: u32,
}

fn shuffle<T>(list: &mut [T], random: &mut minecraftoss_player::rng::LegacyRandom) {
    for i in (2..=list.len()).rev() {
        let j = random.next_int(i as u32) as usize;
        list.swap(i - 1, j);
    }
}

fn shuffle_and_split(
    mut items: Vec<ItemStack>,
    slots: usize,
    random: &mut minecraftoss_player::rng::LegacyRandom,
) -> Vec<ItemStack> {
    items.retain(|stack| stack.count > 0);
    let (mut many, mut out): (Vec<ItemStack>, Vec<ItemStack>) =
        items.into_iter().partition(|stack| stack.count > 1);
    while slots.saturating_sub(out.len() + many.len()) > 0 && !many.is_empty() {
        let index = random.next_int(many.len() as u32) as usize;
        let mut stack = many.remove(index);
        let half = u32::from(stack.count / 2).max(1);
        let taken = random.next_int(half) as u8 + 1;
        let split = ItemStack {
            count: taken,
            ..stack.clone()
        };
        stack.count -= taken;
        for part in [stack, split] {
            if part.count == 0 {
                continue;
            }
            if part.count > 1 && random.next_int(2) == 0 {
                many.push(part);
            } else {
                out.push(part);
            }
        }
    }
    out.extend(many);
    shuffle(&mut out, random);
    out
}

pub fn structure_loot(
    registries: &minecraftoss_core::Registries,
    entity: &minecraftoss_core::nbt::Tag,
    pos: BlockPos,
    world_seed: i64,
    recipes: &RecipeBook,
) -> Option<(Vec<ItemStack>, u64)> {
    let table = entity.get("LootTable")?.as_str()?;
    let seed = entity
        .get("LootTableSeed")
        .and_then(minecraftoss_core::nbt::Tag::as_i64)
        .filter(|seed| *seed != 0)
        .unwrap_or_else(|| {
            world_seed
                ^ (i64::from(pos.0) * 3_129_871)
                ^ (i64::from(pos.2) * 116_129_781)
                ^ i64::from(pos.1)
        });
    let params = minecraftoss_core::loot::LootParams {
        origin: Some([
            f64::from(pos.0) + 0.5,
            f64::from(pos.1) + 0.5,
            f64::from(pos.2) + 0.5,
        ]),
        ..Default::default()
    };
    let mut sequences = minecraftoss_core::loot::RandomSequences::new(world_seed);
    let mut random = minecraftoss_core::random::LegacyRandom::new(seed);
    let stacks = registries
        .loot
        .roll(registries, table, &params, &mut sequences, &mut random)
        .ok()?;
    let items = stacks
        .into_iter()
        .filter(|stack| stack.count > 0)
        .map(|stack| ItemStack {
            count: stack.count.clamp(1, 255) as u8,
            max: recipes.max_stack(&stack.id),
            components: stack.components.as_ref().map(crate::item_nbt::tag_json),
            id: stack.id,
        })
        .collect();
    Some((items, seed as u64))
}

impl Containers {
    pub fn set_book(&mut self, book: Arc<StationBook>) {
        self.book = book;
    }

    pub fn is_new(&self, pos: BlockPos) -> bool {
        !self.chests.contains_key(&pos)
    }

    pub fn fill(&mut self, pos: BlockPos, items: Vec<ItemStack>, seed: u64) {
        let mut random = minecraftoss_player::rng::LegacyRandom::new(seed);
        let mut slots: Vec<usize> = (0..CHEST_SLOTS).collect();
        shuffle(&mut slots, &mut random);
        let items = shuffle_and_split(items, slots.len(), &mut random);
        let mut cells = vec![None; CHEST_SLOTS];
        for item in items {
            let Some(slot) = slots.pop() else {
                break;
            };
            cells[slot] = Some(item);
        }
        self.chests.insert(pos, cells);
    }

    pub fn oven(&self, pos: BlockPos) -> Option<&Oven> {
        self.ovens.get(&pos)
    }

    pub fn brewer(&self, pos: BlockPos) -> Option<&Brewer> {
        self.brewers.get(&pos)
    }

    pub fn place_shulker(&mut self, pos: BlockPos, stack: &ItemStack, recipes: &RecipeBook) {
        let mut cells = vec![None; CHEST_SLOTS];
        for (slot, mut stored) in stored_items(stack) {
            if let Some(cell) = cells.get_mut(slot) {
                stored.max = recipes.max_stack(&stored.id);
                *cell = Some(stored);
            }
        }
        self.chests.insert(pos, cells);
        match stack.components.as_ref().and_then(|c| c.get(CUSTOM_NAME)) {
            Some(name) => self.names.insert(pos, name.clone()),
            None => self.names.remove(&pos),
        };
    }

    pub fn take_shulker(&mut self, pos: BlockPos) -> Option<Value> {
        let mut components = serde_json::Map::new();
        if let Some(cells) = self
            .chests
            .remove(&pos)
            .filter(|cells| cells.iter().any(Option::is_some))
        {
            components.insert(CONTAINER.to_owned(), container_component(&cells));
        }
        if let Some(name) = self.names.remove(&pos) {
            components.insert(CUSTOM_NAME.to_owned(), name);
        }
        (!components.is_empty()).then_some(Value::Object(components))
    }

    pub fn custom_name(&self, pos: BlockPos) -> Option<String> {
        match self.names.get(&pos)? {
            Value::String(text) => Some(text.clone()),
            other => other.get("text").and_then(Value::as_str).map(str::to_owned),
        }
    }

    pub fn remove(&mut self, pos: BlockPos) -> (Vec<ItemStack>, f32) {
        self.names.remove(&pos);
        if let Some(cells) = self.chests.remove(&pos) {
            return (cells.into_iter().flatten().collect(), 0.0);
        }
        if let Some(brewer) = self.brewers.remove(&pos) {
            return (brewer.slots.into_iter().flatten().collect(), 0.0);
        }
        if let Some(campfire) = self.campfires.remove(&pos) {
            return (campfire.take_contents(), 0.0);
        }
        match self.ovens.remove(&pos) {
            Some(mut oven) => (oven.furnace.take_contents(), oven.experience),
            None => (Vec::new(), 0.0),
        }
    }

    pub fn campfires(&self) -> impl Iterator<Item = (BlockPos, &Campfire)> {
        self.campfires.iter().map(|(pos, campfire)| (*pos, campfire))
    }

    pub fn place_food(&mut self, pos: BlockPos, stack: &ItemStack, recipes: &RecipeBook) -> Option<bool> {
        let (_, ticks) = self.book.campfire_cooking(&stack.id, recipes)?;
        Some(self.campfires.entry(pos).or_default().place(stack, ticks))
    }

    pub fn cooks_on_campfire(&self, stack: &ItemStack, recipes: &RecipeBook) -> bool {
        self.book.campfire_cooking(&stack.id, recipes).is_some()
    }

    pub fn cook(&mut self, lit: impl Fn(BlockPos) -> Option<bool>, recipes: &RecipeBook) -> Vec<(BlockPos, ItemStack)> {
        let mut positions: Vec<BlockPos> = self.campfires.keys().copied().collect();
        positions.sort_unstable();
        let mut popped = Vec::new();
        for pos in positions {
            let (Some(campfire), Some(lit)) = (self.campfires.get_mut(&pos), lit(pos)) else {
                continue;
            };
            let book = &self.book;
            let done = campfire.tick(lit, |item| book.campfire_cooking(&item.id, recipes).map(|(cooked, _)| cooked));
            popped.extend(done.into_iter().map(|stack| (pos, stack)));
        }
        self.campfires.retain(|_, campfire| !campfire.is_empty());
        popped
    }

    pub fn tick(&mut self, recipes: &RecipeBook) -> Ticked {
        let mut ticked = Ticked::default();
        for (pos, oven) in &mut self.ovens {
            if oven.tick(recipes) {
                ticked.props.push((*pos, "lit", oven.is_lit()));
            }
        }
        for (pos, brewer) in &mut self.brewers {
            let (brewed, drop) = brewer.tick(&self.book, recipes);
            if brewed {
                ticked.brewed.push(*pos);
            }
            if let Some(stack) = drop {
                ticked.drops.push((*pos, stack));
            }
            let bottles = brewer.bottles();
            for (i, name) in ["has_bottle_0", "has_bottle_1", "has_bottle_2"]
                .into_iter()
                .enumerate()
            {
                if bottles[i] != brewer.shown[i] {
                    ticked.props.push((*pos, name, bottles[i]));
                }
            }
            brewer.shown = bottles;
        }
        ticked
            .props
            .sort_unstable_by_key(|(pos, name, _)| (*pos, *name));
        ticked.brewed.sort_unstable();
        ticked
    }

    pub fn cells(&mut self, opened: &Opened) -> Vec<Option<ItemStack>> {
        match opened.station {
            Station::Oven(kind) => self
                .ovens
                .entry(opened.cells[0])
                .or_insert_with(|| Oven::new(kind))
                .furnace
                .slots
                .to_vec(),
            Station::Brewer => self
                .brewers
                .entry(opened.cells[0])
                .or_default()
                .slots
                .to_vec(),
            Station::Ender => {
                self.ender.resize(CHEST_SLOTS, None);
                self.ender.clone()
            }
            _ => opened
                .cells
                .iter()
                .flat_map(|pos| {
                    self.chests
                        .entry(*pos)
                        .or_insert_with(|| vec![None; CHEST_SLOTS])
                        .clone()
                })
                .collect(),
        }
    }

    fn store(&mut self, opened: &Opened, cells: Vec<Option<ItemStack>>, recipes: &RecipeBook) {
        match opened.station {
            Station::Oven(kind) => {
                let oven = self
                    .ovens
                    .entry(opened.cells[0])
                    .or_insert_with(|| Oven::new(kind));
                let before = oven.furnace.slots[INPUT].clone();
                for (slot, cell) in oven.furnace.slots.iter_mut().zip(cells) {
                    *slot = cell;
                }
                oven.refresh(&before, recipes);
            }
            Station::Brewer => {
                let brewer = self.brewers.entry(opened.cells[0]).or_default();
                for (slot, cell) in brewer.slots.iter_mut().zip(cells) {
                    *slot = cell;
                }
            }
            Station::Ender => self.ender = cells,
            _ => {
                for (pos, half) in opened.cells.iter().zip(cells.chunks(CHEST_SLOTS)) {
                    self.chests.insert(*pos, half.to_vec());
                }
            }
        }
    }

    fn take_output(&mut self, opened: &Opened, roll: f32) -> u32 {
        match opened.station {
            Station::Oven(_) => self
                .ovens
                .get_mut(&opened.cells[0])
                .map_or(0, |oven| oven.take_experience(roll)),
            _ => 0,
        }
    }

    pub fn click(
        &mut self,
        opened: &Opened,
        inventory: &mut Inventory,
        index: usize,
        right: bool,
        shift: bool,
        roll: f32,
    ) -> Changed {
        let mut cells = self.cells(opened);
        if index >= cells.len() {
            return Changed { experience: 0 };
        }
        let output = is_output(opened, index);
        let had = cells[index].clone();
        if shift {
            if let Some(mut stack) = cells[index].take() {
                let order = if output || !matches!(opened.station, Station::Oven(_)) {
                    result_order()
                } else {
                    (HOTBAR..STORAGE).chain(0..HOTBAR).collect()
                };
                move_into(&mut inventory.slots, &mut stack, &order);
                cells[index] = (stack.count > 0).then_some(stack);
            }
        } else {
            let may_place = inventory.cursor.as_ref().is_some_and(|carried| {
                accepts(&self.book, opened, index, carried, &inventory.recipes)
            });
            pick(
                &mut cells[index],
                &mut inventory.cursor,
                right,
                may_place,
                limit(opened, index),
            );
        }
        let taken = output && cells[index] != had;
        if let Some(stack) = had.as_ref().filter(|_| taken) {
            inventory.record_crafted(stack);
        }
        self.store(opened, cells, &inventory.recipes);
        Changed {
            experience: if taken {
                self.take_output(opened, roll)
            } else {
                0
            },
        }
    }

    pub fn quick_move_from_player(
        &mut self,
        opened: &Opened,
        inventory: &mut Inventory,
        slot: usize,
    ) {
        if slot >= STORAGE {
            return;
        }
        let Some(mut stack) = inventory.slots[slot].take() else {
            return;
        };
        let mut cells = self.cells(opened);
        match opened.station {
            Station::Oven(kind) => {
                let recipes = inventory.recipes.clone();
                if recipes.cooking_for(kind, &stack).is_some() {
                    move_into(&mut cells, &mut stack, &[INPUT]);
                } else if recipes.is_fuel(&stack.id) {
                    move_into(&mut cells, &mut stack, &[FUEL]);
                } else {
                    let order: Vec<usize> = if slot >= HOTBAR {
                        (0..HOTBAR).collect()
                    } else {
                        (HOTBAR..STORAGE).collect()
                    };
                    move_into(&mut inventory.slots, &mut stack, &order);
                }
            }
            Station::Brewer => {
                let recipes = inventory.recipes.clone();
                let reagent = self.book.is_reagent(&stack.id, &recipes);
                let bottle = accepts(&self.book, opened, 0, &stack, &recipes);
                let before = stack.count;
                if stack.id == "minecraft:blaze_powder" {
                    move_into(&mut cells, &mut stack, &[BLAZE]);
                }
                if stack.count == before && reagent {
                    move_into(&mut cells, &mut stack, &[REAGENT]);
                } else if stack.count == before && bottle {
                    for cell in &mut cells[..BOTTLES] {
                        if stack.count > 0 && cell.is_none() {
                            *cell = Some(ItemStack {
                                count: 1,
                                ..stack.clone()
                            });
                            stack.count -= 1;
                        }
                    }
                } else if stack.count == before {
                    let order: Vec<usize> = if slot >= HOTBAR {
                        (0..HOTBAR).collect()
                    } else {
                        (HOTBAR..STORAGE).collect()
                    };
                    move_into(&mut inventory.slots, &mut stack, &order);
                }
            }
            Station::Shulker if is_shulker_box(&stack.id) => {}
            _ => {
                let order: Vec<usize> = (0..cells.len()).collect();
                move_into(&mut cells, &mut stack, &order);
            }
        }
        if stack.count > 0 {
            if inventory.slots[slot].is_none() {
                inventory.slots[slot] = Some(stack);
            } else {
                move_into(&mut inventory.slots, &mut stack, &result_order());
            }
        }
        self.store(opened, cells, &inventory.recipes);
    }

    pub fn swap(
        &mut self,
        opened: &Opened,
        inventory: &mut Inventory,
        index: usize,
        hotbar: usize,
        roll: f32,
    ) -> Changed {
        let mut cells = self.cells(opened);
        let none = Changed { experience: 0 };
        if index >= cells.len() || (hotbar >= HOTBAR && hotbar != OFFHAND) {
            return none;
        }
        let output = is_output(opened, index);
        let held = inventory.slots[hotbar].clone();
        let taken = match (&held, &cells[index]) {
            (None, None) => return none,
            (None, Some(_)) => {
                inventory.slots[hotbar] = cells[index].take();
                true
            }
            (Some(item), None) => {
                if !accepts(&self.book, opened, index, item, &inventory.recipes) {
                    return none;
                }
                let cap = limit(opened, index);
                if item.count > cap {
                    cells[index] = Some(ItemStack {
                        count: cap,
                        ..item.clone()
                    });
                    shrink(&mut inventory.slots[hotbar], cap);
                } else {
                    cells[index] = inventory.slots[hotbar].take();
                }
                false
            }
            (Some(item), Some(_)) => {
                if !accepts(&self.book, opened, index, item, &inventory.recipes)
                    || item.count > limit(opened, index)
                {
                    return none;
                }
                std::mem::swap(&mut cells[index], &mut inventory.slots[hotbar]);
                true
            }
        };
        self.store(opened, cells, &inventory.recipes);
        Changed {
            experience: if taken && output {
                self.take_output(opened, roll)
            } else {
                0
            },
        }
    }

    pub fn spread(
        &mut self,
        opened: &Opened,
        inventory: &mut Inventory,
        players: &[usize],
        cells_at: &[usize],
        right: bool,
    ) {
        let mut cells = self.cells(opened);
        let base = inventory.slots.len();
        let indices: Vec<usize> = players
            .iter()
            .copied()
            .filter(|&slot| slot < STORAGE)
            .chain(cells_at.iter().map(|&cell| base + cell))
            .collect();
        let recipes = inventory.recipes.clone();
        let book = self.book.clone();
        inventory.distribute_limited(&indices, right, &mut cells, |index, stack| {
            accepts(&book, opened, index, stack, &recipes).then(|| limit(opened, index))
        });
        self.store(opened, cells, &inventory.recipes);
    }

    pub fn accepts(
        &self,
        opened: &Opened,
        index: usize,
        stack: &ItemStack,
        recipes: &RecipeBook,
    ) -> Option<u8> {
        (index < opened.size()
            && !is_output(opened, index)
            && accepts(&self.book, opened, index, stack, recipes))
        .then(|| limit(opened, index).min(stack.max))
    }

    pub fn throw(
        &mut self,
        opened: &Opened,
        inventory: &mut Inventory,
        index: usize,
        whole: bool,
        roll: f32,
    ) -> (Option<ItemStack>, Changed) {
        let mut cells = self.cells(opened);
        let Some(stack) = cells.get_mut(index).and_then(Option::as_mut) else {
            return (None, Changed { experience: 0 });
        };
        let count = if whole { stack.count } else { 1 };
        let thrown = ItemStack {
            count,
            ..stack.clone()
        };
        shrink(&mut cells[index], count);
        let output = is_output(opened, index);
        if output {
            inventory.record_crafted(&thrown);
        }
        self.store(opened, cells, &inventory.recipes);
        let experience = if output {
            self.take_output(opened, roll)
        } else {
            0
        };
        (Some(thrown), Changed { experience })
    }

    pub fn gather(&mut self, opened: &Opened, inventory: &mut Inventory, right: bool) {
        let Some(mut carried) = inventory.cursor.take() else {
            return;
        };
        let mut cells = self.cells(opened);
        let order: Vec<(bool, usize)> = (0..cells.len())
            .filter(|&index| !is_output(opened, index))
            .map(|index| (true, index))
            .chain((HOTBAR..STORAGE).chain(0..HOTBAR).map(|slot| (false, slot)))
            .collect();
        let order: Vec<(bool, usize)> = if right {
            order.into_iter().rev().collect()
        } else {
            order
        };
        for pass in 0..2 {
            for &(cell, index) in &order {
                if carried.count >= carried.max {
                    break;
                }
                let source = if cell {
                    &mut cells[index]
                } else {
                    &mut inventory.slots[index]
                };
                let Some(stack) = source.as_mut() else {
                    continue;
                };
                if !stack.same_item(&carried) || (pass == 0 && stack.count >= stack.max) {
                    continue;
                }
                let moved = stack.count.min(carried.max - carried.count);
                carried.count += moved;
                stack.count -= moved;
                if stack.count == 0 {
                    *source = None;
                }
            }
        }
        inventory.cursor = Some(carried);
        self.store(opened, cells, &inventory.recipes);
    }
}

fn is_output(opened: &Opened, index: usize) -> bool {
    matches!(opened.station, Station::Oven(_)) && index == OUTPUT
}

fn limit(opened: &Opened, index: usize) -> u8 {
    if opened.station == Station::Brewer && index < BOTTLES {
        1
    } else {
        u8::MAX
    }
}

fn accepts(
    book: &StationBook,
    opened: &Opened,
    index: usize,
    stack: &ItemStack,
    recipes: &RecipeBook,
) -> bool {
    match (opened.station, index) {
        (Station::Oven(_), OUTPUT) => false,
        (Station::Oven(_), FUEL) => recipes.is_fuel(&stack.id) || stack.id == "minecraft:bucket",
        (Station::Brewer, REAGENT) => book.is_reagent(&stack.id, recipes),
        (Station::Brewer, BLAZE) => stack.id == "minecraft:blaze_powder",
        (Station::Shulker, _) => !is_shulker_box(&stack.id),
        (Station::Brewer, _) => matches!(
            stack.id.as_str(),
            "minecraft:potion"
                | "minecraft:splash_potion"
                | "minecraft:lingering_potion"
                | "minecraft:glass_bottle"
        ),
        _ => true,
    }
}

pub(crate) fn pick(
    cell: &mut Option<ItemStack>,
    cursor: &mut Option<ItemStack>,
    right: bool,
    may_place: bool,
    limit: u8,
) {
    match (cell.as_mut(), cursor.as_mut()) {
        (None, None) => {}
        (None, Some(carried)) => {
            if may_place {
                let moved = if right { 1 } else { carried.count }
                    .min(carried.max)
                    .min(limit);
                *cell = Some(ItemStack {
                    count: moved,
                    ..carried.clone()
                });
                carried.count -= moved;
                if carried.count == 0 {
                    *cursor = None;
                }
            }
        }
        (Some(stack), None) => {
            let moved = if right {
                stack.count.div_ceil(2)
            } else {
                stack.count
            };
            *cursor = Some(ItemStack {
                count: moved,
                ..stack.clone()
            });
            stack.count -= moved;
            if stack.count == 0 {
                *cell = None;
            }
        }
        (Some(stack), Some(carried)) => {
            if may_place && stack.same_item(carried) {
                let room = stack.max.min(limit).saturating_sub(stack.count);
                let moved = if right { 1 } else { carried.count }.min(room);
                stack.count += moved;
                carried.count -= moved;
                if carried.count == 0 {
                    *cursor = None;
                }
            } else if may_place && carried.count <= limit {
                std::mem::swap(cell, cursor);
            } else if stack.same_item(carried) && carried.count + stack.count <= carried.max {
                carried.count += stack.count;
                *cell = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use serde_json::json;

    use super::*;

    fn book() -> RecipeBook {
        let dir = crate::test_dir::tempdir().unwrap();
        let jar = dir.path().join("client.jar");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        let recipes = [
            (
                "iron_ingot",
                json!({"type": "minecraft:smelting", "ingredient": "minecraft:raw_iron", "result": {"id": "minecraft:iron_ingot"}, "cookingtime": 200, "experience": 0.7}),
            ),
            (
                "iron_ingot_blasting",
                json!({"type": "minecraft:blasting", "ingredient": "minecraft:raw_iron", "result": {"id": "minecraft:iron_ingot"}, "cookingtime": 100, "experience": 0.7}),
            ),
        ];
        for (name, value) in recipes {
            zip.start_file(
                format!("data/minecraft/recipe/{name}.json"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(value.to_string().as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        RecipeBook::from_jar(&jar).unwrap()
    }

    fn player() -> Inventory {
        Inventory::default().with_recipes(book())
    }

    fn block(id: &str, props: &[(&str, &str)]) -> Block {
        let mut block = Block::new(id);
        for (k, v) in props {
            block = block.with(k, v);
        }
        block
    }

    fn furnace_opened() -> Opened {
        Opened::new((0, 64, 0), &block("minecraft:furnace", &[]), None).unwrap()
    }

    #[test]
    fn stations_and_double_chests_follow_vanilla() {
        assert_eq!(
            Station::of(&block("minecraft:trapped_chest", &[])),
            Some(Station::Chest)
        );
        assert_eq!(
            Station::of(&block("minecraft:smoker", &[])),
            Some(Station::Oven(CookingKind::Smoker))
        );
        assert_eq!(Station::of(&block("minecraft:stone", &[])), None);
        let left = block("minecraft:chest", &[("facing", "north"), ("type", "left")]);
        let right = block("minecraft:chest", &[("facing", "north"), ("type", "right")]);
        assert_eq!(chest_partner((5, 64, 5), &left), Some((6, 64, 5)));
        assert_eq!(chest_partner((6, 64, 5), &right), Some((5, 64, 5)));
        let opened = Opened::new((5, 64, 5), &left, Some(&right)).unwrap();
        assert_eq!(opened.cells, vec![(6, 64, 5), (5, 64, 5)]);
        assert_eq!((opened.rows(), opened.size()), (6, 54));
        let single = Opened::new((5, 64, 5), &left, None).unwrap();
        assert_eq!((single.rows(), single.size()), (3, 27));
        assert_eq!(Station::Chest.title(true), "LARGE CHEST");
    }

    #[test]
    fn chests_keep_their_items_and_shift_moves_vanilla_ways() {
        let mut store = Containers::default();
        let mut inventory = player();
        let opened = Opened::new((0, 64, 0), &block("minecraft:barrel", &[]), None).unwrap();
        inventory.slots[12] = Some(inventory.recipes.stack("minecraft:dirt", 30));
        store.quick_move_from_player(&opened, &mut inventory, 12);
        assert!(inventory.slots[12].is_none());
        assert_eq!(store.cells(&opened)[0].as_ref().map(|s| s.count), Some(30));
        store.click(&opened, &mut inventory, 0, false, true, 0.0);
        assert_eq!(
            inventory.slots[8].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:dirt"),
            "out of a chest a stack goes to the last hotbar slot first"
        );
        store.click(&opened, &mut inventory, 0, false, false, 0.0);
        assert!(inventory.cursor.is_none());
        inventory.cursor = Some(inventory.recipes.stack("minecraft:stone", 9));
        store.click(&opened, &mut inventory, 4, true, false, 0.0);
        assert_eq!(store.cells(&opened)[4].as_ref().map(|s| s.count), Some(1));
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(8));
        store.click(&opened, &mut inventory, 4, false, false, 0.0);
        assert_eq!(store.cells(&opened)[4].as_ref().map(|s| s.count), Some(9));
        let (dropped, _) = store.remove((0, 64, 0));
        assert_eq!(dropped.len(), 1);
        assert!(store.cells(&opened).iter().all(Option::is_none));
    }

    #[test]
    fn a_double_chest_spreads_over_both_halves() {
        let mut store = Containers::default();
        let mut inventory = player();
        let left = block("minecraft:chest", &[("facing", "east"), ("type", "left")]);
        let right = block("minecraft:chest", &[("facing", "east"), ("type", "right")]);
        let opened = Opened::new((0, 64, 0), &left, Some(&right)).unwrap();
        inventory.cursor = Some(inventory.recipes.stack("minecraft:stone", 10));
        store.spread(&opened, &mut inventory, &[], &[26, 27], false);
        let cells = store.cells(&opened);
        assert_eq!(cells[26].as_ref().map(|s| s.count), Some(5));
        assert_eq!(cells[27].as_ref().map(|s| s.count), Some(5));
        let other = Opened::new((0, 64, 1), &right, Some(&left)).unwrap();
        assert_eq!(store.cells(&other), cells);
        inventory.cursor = Some(inventory.recipes.stack("minecraft:stone", 1));
        store.gather(&opened, &mut inventory, false);
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(11));
    }

    #[test]
    fn a_furnace_smelts_with_fuel_and_pays_experience_on_taking() {
        let mut store = Containers::default();
        let mut inventory = player();
        let opened = furnace_opened();
        inventory.slots[10] = Some(inventory.recipes.stack("minecraft:raw_iron", 3));
        inventory.slots[11] = Some(inventory.recipes.stack("minecraft:coal", 1));
        inventory.slots[12] = Some(inventory.recipes.stack("minecraft:dirt", 5));
        store.quick_move_from_player(&opened, &mut inventory, 10);
        store.quick_move_from_player(&opened, &mut inventory, 11);
        store.quick_move_from_player(&opened, &mut inventory, 12);
        assert_eq!(
            inventory.slots[0].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:dirt"),
            "neither smeltable nor fuel goes to the hotbar"
        );
        let cells = store.cells(&opened);
        assert_eq!(cells[0].as_ref().map(|s| s.count), Some(3));
        assert_eq!(
            cells[1].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:coal")
        );
        inventory.cursor = Some(inventory.recipes.stack("minecraft:dirt", 1));
        store.click(&opened, &mut inventory, 1, false, false, 0.0);
        assert_eq!(
            inventory.cursor.as_ref().map(|s| s.id.as_str()),
            Some("minecraft:dirt"),
            "the fuel slot refuses what does not burn"
        );
        inventory.cursor = None;
        let mut lit = Vec::new();
        for _ in 0..400 {
            lit.extend(store.tick(&inventory.recipes).props);
        }
        assert_eq!(lit, vec![((0, 64, 0), "lit", true)]);
        let oven = store.oven((0, 64, 0)).unwrap();
        assert_eq!(oven.furnace.slots[2].as_ref().map(|s| s.count), Some(2));
        assert!((oven.experience - 1.4).abs() < 1e-4);
        let changed = store.click(&opened, &mut inventory, 2, false, true, 0.9);
        assert_eq!(changed.experience, 1);
        assert_eq!(
            inventory.slots[8].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:iron_ingot")
        );
        let blast = Opened::new((1, 64, 0), &block("minecraft:blast_furnace", &[]), None).unwrap();
        inventory.slots[13] = Some(inventory.recipes.stack("minecraft:raw_iron", 1));
        inventory.slots[14] = Some(inventory.recipes.stack("minecraft:coal", 1));
        store.quick_move_from_player(&blast, &mut inventory, 13);
        store.quick_move_from_player(&blast, &mut inventory, 14);
        for _ in 0..101 {
            store.tick(&inventory.recipes);
        }
        let blast_oven = store.oven((1, 64, 0)).unwrap();
        assert_eq!(
            blast_oven.furnace.slots[2].as_ref().map(|s| s.count),
            Some(1)
        );
    }

    #[test]
    fn a_brewing_stand_brews_with_blaze_powder_while_closed() {
        let dir = crate::test_dir::tempdir().unwrap();
        let jar = dir.path().join("client.jar");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        zip.start_file(
            "data/minecraft/recipe/brewing/potion_water_nether_wart.json",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        let recipe = json!({"type": "minecraft:brewing", "input": {"item": "minecraft:potion", "potion_contents": {"potions": "minecraft:water"}}, "reagent": {"item": "minecraft:nether_wart"}, "output": {"id": "minecraft:potion", "components": {"minecraft:potion_contents": {"potion": "minecraft:awkward"}}}});
        zip.write_all(recipe.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();
        let mut store = Containers::default();
        store.set_book(Arc::new(StationBook::from_jar(&jar).unwrap()));
        let mut inventory = Inventory::default().with_recipes(RecipeBook::from_jar(&jar).unwrap());
        let pos = (3, 64, 3);
        let opened = Opened::new(pos, &block("minecraft:brewing_stand", &[]), None).unwrap();
        assert_eq!(opened.size(), 5);
        for slot in 10..13 {
            inventory.slots[slot] = Some(crate::bottle::water_bottle());
        }
        inventory.slots[14] = Some(inventory.recipes.stack("minecraft:nether_wart", 2));
        inventory.slots[15] = Some(inventory.recipes.stack("minecraft:blaze_powder", 1));
        for slot in [15, 14, 10, 11, 12] {
            store.quick_move_from_player(&opened, &mut inventory, slot);
        }
        let cells = store.cells(&opened);
        assert!(cells[..3].iter().all(Option::is_some));
        assert_eq!(cells[3].as_ref().map(|s| s.count), Some(2));
        assert_eq!(
            cells[4].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:blaze_powder")
        );
        let first = store.tick(&inventory.recipes);
        assert_eq!(first.props.len(), 3);
        assert_eq!(store.brewer(pos).unwrap().fuel, 19);
        let mut brewed = Vec::new();
        for _ in 0..400 {
            brewed.extend(store.tick(&inventory.recipes).brewed);
        }
        assert_eq!(brewed, vec![pos]);
        let cells = store.cells(&opened);
        assert!(
            cells[..3]
                .iter()
                .all(|b| b.as_ref().and_then(crate::stations::potion_of)
                    == Some("minecraft:awkward"))
        );
        assert_eq!(cells[3].as_ref().map(|s| s.count), Some(1));
        assert!(cells[4].is_none(), "the powder became twenty uses of fuel");
        store.click(&opened, &mut inventory, 0, false, false, 0.0);
        inventory.cursor = Some(inventory.recipes.stack("minecraft:glass_bottle", 4));
        store.click(&opened, &mut inventory, 0, false, false, 0.0);
        assert_eq!(store.cells(&opened)[0].as_ref().map(|s| s.count), Some(1));
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(3));
    }

    #[test]
    fn structure_loot_spreads_over_random_slots_once() {
        let mut store = Containers::default();
        let recipes = book();
        let pos = (9, 40, 9);
        assert!(store.is_new(pos));
        store.fill(
            pos,
            vec![
                recipes.stack("minecraft:bread", 6),
                recipes.stack("minecraft:iron_ingot", 1),
            ],
            42,
        );
        assert!(!store.is_new(pos));
        let opened = Opened::new(pos, &block("minecraft:chest", &[]), None).unwrap();
        let cells = store.cells(&opened);
        let used: Vec<&ItemStack> = cells.iter().flatten().collect();
        assert!(used.len() >= 2);
        let bread: u32 = used
            .iter()
            .filter(|s| s.id == "minecraft:bread")
            .map(|s| u32::from(s.count))
            .sum();
        assert_eq!(bread, 6);
        assert!(used.iter().any(|s| s.id == "minecraft:iron_ingot"));
    }

    #[test]
    fn a_dungeon_chest_rolls_its_jar_loot_table() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            "LootTable".to_owned(),
            minecraftoss_core::nbt::Tag::String("minecraft:chests/simple_dungeon".to_owned()),
        );
        map.insert(
            "LootTableSeed".to_owned(),
            minecraftoss_core::nbt::Tag::Long(12345),
        );
        let entity = minecraftoss_core::nbt::Tag::Compound(map);
        let recipes = book();
        let (items, seed) = structure_loot(&registries, &entity, (5, 30, 5), 7, &recipes).unwrap();
        assert_eq!(seed, 12345);
        assert!(!items.is_empty(), "a dungeon chest has loot");
        let plain = minecraftoss_core::nbt::Tag::Compound(std::collections::BTreeMap::new());
        assert!(structure_loot(&registries, &plain, (5, 30, 5), 7, &recipes).is_none());
    }

    #[test]
    fn experience_rounds_by_chance() {
        let mut oven = Oven::new(CookingKind::Furnace);
        oven.experience = 2.25;
        assert_eq!(oven.take_experience(0.2), 3);
        oven.experience = 2.25;
        assert_eq!(oven.take_experience(0.3), 2);
        assert_eq!(oven.take_experience(0.0), 0);
    }

    #[test]
    fn number_keys_swap_with_the_hotbar_and_outputs_only_give() {
        let mut store = Containers::default();
        let mut inventory = player();
        let opened = furnace_opened();
        inventory.slots[3] = Some(inventory.recipes.stack("minecraft:dirt", 4));
        let changed = store.swap(&opened, &mut inventory, 2, 3, 0.0);
        assert_eq!(changed.experience, 0);
        assert!(
            store.cells(&opened)[2].is_none(),
            "nothing goes into an output"
        );
        store.swap(&opened, &mut inventory, 0, 3, 0.0);
        assert_eq!(
            store.cells(&opened)[0].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:dirt")
        );
        assert!(inventory.slots[3].is_none());
    }

    #[test]
    fn q_throws_one_or_a_whole_stack_out_of_a_container() {
        let mut store = Containers::default();
        let mut inventory = player();
        let opened = Opened::new((0, 64, 0), &block("minecraft:barrel", &[]), None).unwrap();
        inventory.cursor = Some(inventory.recipes.stack("minecraft:dirt", 5));
        store.click(&opened, &mut inventory, 3, false, false, 0.0);
        let (one, _) = store.throw(&opened, &mut inventory, 3, false, 0.0);
        assert_eq!(one.map(|s| s.count), Some(1));
        assert_eq!(store.cells(&opened)[3].as_ref().map(|s| s.count), Some(4));
        let (rest, _) = store.throw(&opened, &mut inventory, 3, true, 0.0);
        assert_eq!(rest.map(|s| s.count), Some(4));
        assert!(store.cells(&opened)[3].is_none());
        assert!(store.throw(&opened, &mut inventory, 3, true, 0.0).0.is_none());
    }

    #[test]
    fn f_swaps_a_cell_with_the_offhand_and_drags_only_where_items_fit() {
        let mut store = Containers::default();
        let mut inventory = player();
        let opened = furnace_opened();
        inventory.slots[OFFHAND] = Some(inventory.recipes.stack("minecraft:raw_iron", 3));
        store.swap(&opened, &mut inventory, 0, OFFHAND, 0.0);
        assert_eq!(
            store.cells(&opened)[0].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:raw_iron")
        );
        assert!(inventory.slots[OFFHAND].is_none());
        let iron = inventory.recipes.stack("minecraft:raw_iron", 1);
        assert_eq!(
            store.accepts(&opened, 0, &iron, &inventory.recipes),
            Some(iron.max)
        );
        assert_eq!(store.accepts(&opened, 1, &iron, &inventory.recipes), None);
        assert_eq!(store.accepts(&opened, 2, &iron, &inventory.recipes), None);
        let brewer =
            Opened::new((1, 64, 0), &block("minecraft:brewing_stand", &[]), None).unwrap();
        let bottle = inventory.recipes.stack("minecraft:glass_bottle", 16);
        assert_eq!(
            store.accepts(&brewer, 0, &bottle, &inventory.recipes),
            Some(1)
        );
        assert_eq!(store.accepts(&brewer, 4, &bottle, &inventory.recipes), None);
    }

    #[test]
    fn a_drag_puts_one_bottle_in_each_brewing_slot_and_counts_full_slots() {
        let mut store = Containers::default();
        let mut inventory = player();
        let brewer =
            Opened::new((1, 64, 0), &block("minecraft:brewing_stand", &[]), None).unwrap();
        inventory.cursor = Some(inventory.recipes.stack("minecraft:glass_bottle", 5));
        store.spread(&brewer, &mut inventory, &[], &[0, 1, 2], false);
        let cells = store.cells(&brewer);
        assert!(cells[..BOTTLES]
            .iter()
            .all(|cell| cell.as_ref().map(|s| s.count) == Some(1)));
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(2));
        let barrel = Opened::new((2, 64, 0), &block("minecraft:barrel", &[]), None).unwrap();
        inventory.cursor = Some(inventory.recipes.stack("minecraft:dirt", 64));
        store.click(&barrel, &mut inventory, 0, false, false, 0.0);
        inventory.cursor = Some(inventory.recipes.stack("minecraft:dirt", 30));
        store.spread(&barrel, &mut inventory, &[], &[0, 1, 2], false);
        let counts: Vec<u8> = store.cells(&barrel)[..3]
            .iter()
            .map(|cell| cell.as_ref().map_or(0, |s| s.count))
            .collect();
        assert_eq!(counts, vec![64, 10, 10]);
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(10));
    }

    #[test]
    fn ender_chests_shulker_boxes_and_copper_chests_open_as_vanilla_stations() {
        assert_eq!(Station::of(&block("minecraft:ender_chest", &[])), Some(Station::Ender));
        assert_eq!(Station::of(&block("minecraft:shulker_box", &[])), Some(Station::Shulker));
        assert_eq!(Station::of(&block("minecraft:lime_shulker_box", &[])), Some(Station::Shulker));
        for copper in ["minecraft:copper_chest", "minecraft:waxed_oxidized_copper_chest"] {
            assert_eq!(Station::of(&block(copper, &[])), Some(Station::Chest), "{copper}");
        }
        let ender = Opened::new((0, 64, 0), &block("minecraft:ender_chest", &[]), None).unwrap();
        let shulker = Opened::new((0, 64, 0), &block("minecraft:red_shulker_box", &[]), None).unwrap();
        assert_eq!((ender.rows(), ender.size()), (3, 27));
        assert_eq!((shulker.rows(), shulker.size()), (3, 27));
        let left = block("minecraft:waxed_copper_chest", &[("facing", "north"), ("type", "left")]);
        let right = block("minecraft:waxed_copper_chest", &[("facing", "north"), ("type", "right")]);
        assert_eq!(Opened::new((5, 64, 5), &left, Some(&right)).unwrap().size(), 54);
        assert_eq!(chest_sound("waxed_weathered_copper_chest", true), "minecraft:block.copper_chest_weathered.open");
        assert_eq!(chest_sound("exposed_copper_chest", false), "minecraft:block.copper_chest.close");
        assert_eq!(chest_sound("trapped_chest", true), "minecraft:block.chest.open");
    }

    #[test]
    fn every_ender_chest_opens_the_players_one_ender_inventory() {
        let mut store = Containers::default();
        let mut inventory = player();
        let here = Opened::new((0, 64, 0), &block("minecraft:ender_chest", &[]), None).unwrap();
        let there = Opened::new((40, 70, -9), &block("minecraft:ender_chest", &[]), None).unwrap();
        inventory.slots[12] = Some(inventory.recipes.stack("minecraft:diamond", 5));
        store.quick_move_from_player(&here, &mut inventory, 12);
        assert_eq!(store.cells(&there)[0].as_ref().map(|s| s.count), Some(5));
        let (spilled, _) = store.remove((0, 64, 0));
        assert!(spilled.is_empty(), "breaking an ender chest drops nothing of the inventory");
        assert_eq!(store.cells(&here)[0].as_ref().map(|s| s.count), Some(5));
        let chest = Opened::new((0, 64, 0), &block("minecraft:chest", &[]), None).unwrap();
        assert!(store.cells(&chest).iter().all(Option::is_none));
    }

    #[test]
    fn a_shulker_box_refuses_shulker_boxes() {
        let mut store = Containers::default();
        let mut inventory = player();
        let opened = Opened::new((0, 64, 0), &block("minecraft:shulker_box", &[]), None).unwrap();
        let boxed = inventory.recipes.stack("minecraft:blue_shulker_box", 1);
        assert_eq!(store.accepts(&opened, 0, &boxed, &inventory.recipes), None);
        inventory.cursor = Some(boxed.clone());
        store.click(&opened, &mut inventory, 0, false, false, 0.0);
        assert!(store.cells(&opened)[0].is_none());
        assert_eq!(inventory.cursor.as_ref().map(|s| s.id.as_str()), Some("minecraft:blue_shulker_box"));
        inventory.cursor = None;
        inventory.slots[15] = Some(boxed);
        store.quick_move_from_player(&opened, &mut inventory, 15);
        assert!(store.cells(&opened).iter().all(Option::is_none));
        assert!(inventory.slots[15].is_some(), "a shift click leaves the box where it was");
        inventory.slots[2] = inventory.slots[15].take();
        store.swap(&opened, &mut inventory, 0, 2, 0.0);
        assert!(store.cells(&opened)[0].is_none());
        let bundle = inventory.recipes.stack("minecraft:bundle", 1);
        assert!(store.accepts(&opened, 0, &bundle, &inventory.recipes).is_some());
        let chest = Opened::new((1, 64, 0), &block("minecraft:chest", &[]), None).unwrap();
        assert!(store.accepts(&chest, 0, &inventory.recipes.stack("minecraft:shulker_box", 1), &inventory.recipes).is_some());
    }

    #[test]
    fn a_broken_shulker_box_keeps_its_contents_and_name_for_the_next_placing() {
        let mut store = Containers::default();
        let mut inventory = player();
        let pos = (2, 64, 2);
        let opened = Opened::new(pos, &block("minecraft:shulker_box", &[]), None).unwrap();
        let mut sword = inventory.recipes.stack("minecraft:iron_sword", 1);
        sword.components = Some(json!({"minecraft:damage": 4}));
        inventory.cursor = Some(sword);
        store.click(&opened, &mut inventory, 3, false, false, 0.0);
        inventory.cursor = Some(inventory.recipes.stack("minecraft:dirt", 40));
        store.click(&opened, &mut inventory, 26, false, false, 0.0);
        let components = store.take_shulker(pos).unwrap();
        assert!(store.cells(&opened).iter().all(Option::is_none), "nothing is left to spill");
        assert_eq!(
            components,
            json!({"minecraft:container": [
                {"slot": 3, "item": {"id": "minecraft:iron_sword", "count": 1, "components": {"minecraft:damage": 4}}},
                {"slot": 26, "item": {"id": "minecraft:dirt", "count": 40}}
            ]})
        );
        let mut item = inventory.recipes.stack("minecraft:shulker_box", 1);
        item.components = Some(json!({"minecraft:container": components["minecraft:container"], "minecraft:custom_name": "Loot"}));
        let elsewhere = (9, 70, 9);
        store.place_shulker(elsewhere, &item, &inventory.recipes);
        let placed = Opened::new(elsewhere, &block("minecraft:shulker_box", &[]), None).unwrap();
        let cells = store.cells(&placed);
        assert_eq!(cells[3].as_ref().and_then(|s| s.components.clone()), Some(json!({"minecraft:damage": 4})));
        assert_eq!(cells[26].as_ref().map(|s| (s.count, s.max)), Some((40, 64)));
        assert_eq!(store.custom_name(elsewhere).as_deref(), Some("Loot"));
        let kept = store.take_shulker(elsewhere).unwrap();
        assert_eq!(kept["minecraft:custom_name"], json!("Loot"));
        assert_eq!(store.custom_name(elsewhere), None);
        store.place_shulker(pos, &inventory.recipes.stack("minecraft:shulker_box", 1), &inventory.recipes);
        assert_eq!(store.take_shulker(pos), None, "an empty unnamed box drops plain and stacks with new ones");
    }

    #[test]
    fn a_campfire_takes_four_foods_cooks_them_while_lit_and_spills_the_rest() {
        let dir = crate::test_dir::tempdir().unwrap();
        let jar = dir.path().join("client.jar");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        zip.start_file(
            "data/minecraft/recipe/cooked_beef_from_campfire_cooking.json",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        let recipe = json!({"type": "minecraft:campfire_cooking", "ingredient": "minecraft:beef", "result": {"id": "minecraft:cooked_beef"}, "cookingtime": 600});
        zip.write_all(recipe.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();
        let recipes = RecipeBook::from_jar(&jar).unwrap();
        let mut store = Containers::default();
        store.set_book(Arc::new(StationBook::from_jar(&jar).unwrap()));
        let (lit, unlit, unloaded) = ((1, 64, 1), (2, 64, 1), (3, 64, 1));
        let beef = recipes.stack("minecraft:beef", 64);
        assert_eq!(store.place_food(lit, &recipes.stack("minecraft:stick", 1), &recipes), None);
        assert_eq!(store.place_food(lit, &recipes.stack("minecraft:cooked_beef", 1), &recipes), None);
        assert_eq!(store.campfires().count(), 0);
        for _ in 0..4 {
            assert_eq!(store.place_food(lit, &beef, &recipes), Some(true));
        }
        assert_eq!(store.place_food(lit, &beef, &recipes), Some(false));
        assert_eq!(store.place_food(unlit, &beef, &recipes), Some(true));
        assert_eq!(store.place_food(unloaded, &beef, &recipes), Some(true));
        let mut popped = Vec::new();
        for tick in 0..599 {
            let loaded = tick < 300;
            popped.extend(store.cook(|pos| (loaded || pos != unloaded).then_some(pos == lit || pos == unloaded), &recipes));
        }
        assert!(popped.is_empty());
        assert_eq!(store.campfires().find(|(pos, _)| *pos == unloaded).map(|(_, campfire)| campfire.progress(0)), Some(300));
        popped.extend(store.cook(|pos| (pos != unloaded).then_some(pos == lit), &recipes));
        assert_eq!(popped.len(), 4);
        assert!(popped.iter().all(|(pos, stack)| *pos == lit && stack.id == "minecraft:cooked_beef" && stack.count == 1));
        let mut left: Vec<_> = store.campfires().map(|(pos, _)| pos).collect();
        left.sort_unstable();
        assert_eq!(left, vec![unlit, unloaded]);
        assert_eq!(store.remove(unloaded).0.len(), 1);
        let (spilled, experience) = store.remove(unlit);
        assert_eq!((spilled.len(), spilled[0].id.as_str(), experience), (1, "minecraft:beef", 0.0));
        assert_eq!(store.campfires().count(), 0);
    }

    #[test]
    fn a_shulker_box_opens_only_with_room_for_its_lid() {
        let full = [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]];
        let bottom_slab = [[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]];
        let top_slab = [[0.0, 0.5, 0.0, 1.0, 1.0, 1.0]];
        assert!(lid_blocked("up", &full));
        assert!(!lid_blocked("up", &[]));
        assert!(lid_blocked("up", &bottom_slab));
        assert!(!lid_blocked("up", &top_slab), "the lid only needs the lower half above");
        assert!(lid_blocked("down", &top_slab));
        assert!(!lid_blocked("down", &bottom_slab));
        let thin_wall_north = [[0.0, 0.0, 0.8125, 1.0, 1.0, 1.0]];
        assert!(lid_blocked("north", &thin_wall_north));
        assert!(!lid_blocked("south", &thin_wall_north));
        assert!(!lid_blocked("east", &[[0.5, 0.0, 0.0, 1.0, 1.0, 1.0]]));
        assert_eq!(lid_cell((3, 64, 3), "west"), Some((2, 64, 3)));
        assert_eq!(lid_cell((3, 64, 3), "up"), Some((3, 65, 3)));
        let fence = |at: BlockPos| if at == (3, 63, 2) { vec![[0.375, 0.0, 0.375, 0.625, 1.5, 0.625]] } else { Vec::new() };
        assert!(lid_obstructed((3, 64, 3), "north", fence), "a fence below the lid reaches up into it");
        assert!(!lid_obstructed((3, 64, 3), "south", fence));
        let shulker_below_the_lid = |at: BlockPos| if at == (3, 64, 3) { full.to_vec() } else { Vec::new() };
        assert!(!lid_obstructed((3, 64, 3), "up", shulker_below_the_lid));
        assert!(lid_obstructed((3, 64, 3), "up", |at: BlockPos| if at == (3, 65, 3) { bottom_slab.to_vec() } else { Vec::new() }));
    }
}
