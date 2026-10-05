use std::collections::HashMap;
use std::sync::Arc;

use minecraftoss_player::crafting::{CookingKind, RecipeBook};
use minecraftoss_player::furnace::Furnace;
use minecraftoss_player::inventory::{Inventory, ItemStack};

use crate::crafting::{move_into, result_order};
use crate::scene::{Block, BlockPos};
use crate::stations::StationBook;

pub const CHEST_SLOTS: usize = 27;
const HOTBAR: usize = 9;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Station {
    Chest,
    Barrel,
    Oven(CookingKind),
    Brewer,
}

impl Station {
    pub fn of(block: &Block) -> Option<Self> {
        if block.id.namespace != "minecraft" {
            return None;
        }
        match block.id.path.as_str() {
            "chest" | "trapped_chest" => Some(Self::Chest),
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
        }
    }
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
}

#[derive(Default)]
pub struct Containers {
    chests: HashMap<BlockPos, Vec<Option<ItemStack>>>,
    ovens: HashMap<BlockPos, Oven>,
    brewers: HashMap<BlockPos, Brewer>,
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

impl Containers {
    pub fn set_book(&mut self, book: Arc<StationBook>) {
        self.book = book;
    }

    pub fn oven(&self, pos: BlockPos) -> Option<&Oven> {
        self.ovens.get(&pos)
    }

    pub fn brewer(&self, pos: BlockPos) -> Option<&Brewer> {
        self.brewers.get(&pos)
    }

    pub fn remove(&mut self, pos: BlockPos) -> (Vec<ItemStack>, f32) {
        if let Some(cells) = self.chests.remove(&pos) {
            return (cells.into_iter().flatten().collect(), 0.0);
        }
        if let Some(brewer) = self.brewers.remove(&pos) {
            return (brewer.slots.into_iter().flatten().collect(), 0.0);
        }
        match self.ovens.remove(&pos) {
            Some(mut oven) => (oven.furnace.take_contents(), oven.experience),
            None => (Vec::new(), 0.0),
        }
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
        if index >= cells.len() || hotbar >= HOTBAR {
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
        inventory.distribute_external(&indices, right, &mut cells, |index, stack| {
            accepts(&book, opened, index, stack, &recipes) && limit(opened, index) > 1
        });
        self.store(opened, cells, &inventory.recipes);
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
}
