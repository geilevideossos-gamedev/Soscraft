use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::item_catalog::ItemCatalog;

use crate::scene::{Block, BlockPos};

pub const ITEM_CATALOG: &str = "artifacts/item-catalog/26.3.json";

const TABLE_REACH: f64 = 8.5;
const HOTBAR: usize = 9;
const STORAGE: usize = 36;
const OFFHAND: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grid {
    Inventory,
    Table,
}

impl Grid {
    pub fn size(self) -> usize {
        match self {
            Grid::Inventory => 4,
            Grid::Table => 9,
        }
    }

    pub fn player_slots(self) -> usize {
        match self {
            Grid::Inventory => 41,
            Grid::Table => STORAGE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    Player(usize),
    Grid(usize),
    Result,
}

pub fn find_item_catalog(root: Option<&Path>, jar: &Path) -> Option<PathBuf> {
    [
        root.map(|root| root.join(ITEM_CATALOG)),
        jar.parent().map(|parent| parent.join(ITEM_CATALOG)),
        std::env::var_os("MINECRAFTOSS_ROOT").map(|root| Path::new(&root).join(ITEM_CATALOG)),
        Some(PathBuf::from(ITEM_CATALOG)),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.is_file())
}

pub fn load_recipes(
    jar: &Path,
    catalog: Option<&Path>,
) -> Result<(RecipeBook, Option<anyhow::Error>)> {
    let book = RecipeBook::from_jar(jar)?;
    let Some(path) = catalog else {
        return Ok((
            book,
            Some(anyhow::anyhow!(
                "no {ITEM_CATALOG} beside the Minecraft data"
            )),
        ));
    };
    match ItemCatalog::from_path(path) {
        Ok(catalog) => Ok((book.with_item_catalog(Arc::new(catalog)), None)),
        Err(error) => Ok((book, Some(error))),
    }
}

pub fn output(inventory: &Inventory, grid: Grid) -> Option<ItemStack> {
    match grid {
        Grid::Inventory => inventory.crafting_output(),
        Grid::Table => inventory.workbench_output(),
    }
}

pub fn click_cell(inventory: &mut Inventory, grid: Grid, index: usize, right: bool, shift: bool) {
    if index >= grid.size() {
        return;
    }
    if shift {
        quick_move_from_grid(inventory, grid, index);
        return;
    }
    match grid {
        Grid::Inventory => inventory.click_crafting_slot(index, right, false),
        Grid::Table => inventory.click_workbench_slot(index, right, false),
    }
}

pub fn click_result(inventory: &mut Inventory, grid: Grid, shift: bool) -> bool {
    if shift {
        craft_all(inventory, grid) > 0
    } else {
        take(inventory, grid)
    }
}

pub fn take(inventory: &mut Inventory, grid: Grid) -> bool {
    let Some(result) = output(inventory, grid) else {
        return false;
    };
    if let Some(carried) = inventory.cursor.as_ref() {
        let room = limit(&inventory.recipes, carried).saturating_sub(carried.count);
        if !carried.same_item(&result) || room < result.count {
            return false;
        }
    }
    take_into_cursor(inventory, grid)
}

pub fn craft_all(inventory: &mut Inventory, grid: Grid) -> u32 {
    let Some(first) = output(inventory, grid) else {
        return 0;
    };
    let carried = inventory.cursor.take();
    let order = result_order();
    let mut crafted = 0;
    while let Some(result) = output(inventory, grid) {
        if !result.same_item(&first) {
            break;
        }
        let mut stack = result.clone();
        stack.max = limit(&inventory.recipes, &stack);
        let mut slots = inventory.slots.clone();
        move_into(&mut slots, &mut stack, &order);
        if stack.count > 0 {
            break;
        }
        let before = std::mem::replace(&mut inventory.slots, slots);
        if !take_into_cursor(inventory, grid) {
            inventory.slots = before;
            break;
        }
        inventory.cursor = None;
        inventory.notice_item_changed(&result);
        crafted += 1;
    }
    inventory.cursor = carried;
    crafted
}

pub fn quick_move_from_grid(inventory: &mut Inventory, grid: Grid, index: usize) -> bool {
    if index >= grid.size() {
        return false;
    }
    let Some(mut stack) = cells_mut(inventory, grid)[index].take() else {
        return false;
    };
    let changed = stack.clone();
    stack.max = limit(&inventory.recipes, &stack);
    let order: Vec<usize> = (HOTBAR..STORAGE).chain(0..HOTBAR).collect();
    let moved = move_into(&mut inventory.slots, &mut stack, &order);
    if stack.count > 0 {
        cells_mut(inventory, grid)[index] = Some(stack);
    }
    if moved {
        inventory.notice_item_changed(&changed);
    }
    moved
}

pub fn quick_move_into_table(inventory: &mut Inventory, index: usize) -> bool {
    if index >= STORAGE {
        return false;
    }
    let Some(mut stack) = inventory.slots[index].take() else {
        return false;
    };
    let changed = stack.clone();
    stack.max = limit(&inventory.recipes, &stack);
    let cells: Vec<usize> = (0..Grid::Table.size()).collect();
    let into_grid = move_into(&mut inventory.workbench, &mut stack, &cells);
    let mut into_slots = false;
    if stack.count > 0 {
        let order: Vec<usize> = if index < HOTBAR {
            (HOTBAR..STORAGE).collect()
        } else {
            (0..HOTBAR).collect()
        };
        into_slots = move_into(&mut inventory.slots, &mut stack, &order);
    }
    if stack.count > 0 {
        inventory.slots[index] = Some(stack);
    }
    if into_slots {
        inventory.notice_item_changed(&changed);
    }
    into_grid || into_slots
}

pub fn spread(inventory: &mut Inventory, grid: Grid, targets: &[Spot], right: bool) {
    let Some(carried) = inventory.cursor.as_ref() else {
        return;
    };
    let max = limit(&inventory.recipes, carried);
    let players = inventory.slots.len();
    let mut indices = Vec::new();
    for spot in targets {
        let (index, target) = match *spot {
            Spot::Player(slot) if slot < grid.player_slots() => (slot, inventory.slots.get(slot)),
            Spot::Grid(cell) if cell < grid.size() => {
                (players + cell, cells(inventory, grid).get(cell))
            }
            _ => continue,
        };
        if armor_slot(index).is_some_and(|name| inventory.recipes.equipment_slot(carried) != Some(name)) {
            continue;
        }
        let fits = target.is_some_and(|target| {
            target
                .as_ref()
                .is_none_or(|stack| stack.same_item(carried) && stack.count <= max)
        });
        if fits && !indices.contains(&index) {
            indices.push(index);
        }
    }
    indices.truncate(usize::from(carried.count));
    if indices.is_empty() {
        return;
    }
    inventory.distribute_crafting(&indices, right, grid == Grid::Table);
}

pub fn gather(inventory: &mut Inventory, grid: Grid, target: Spot, right: bool) {
    let occupied = match target {
        Spot::Player(index) => inventory.slots.get(index).is_none_or(Option::is_some),
        Spot::Grid(cell) => cells(inventory, grid).get(cell).is_none_or(Option::is_some),
        Spot::Result => true,
    };
    if occupied {
        return;
    }
    let Some(mut carried) = inventory.cursor.take() else {
        return;
    };
    let max = limit(&inventory.recipes, &carried);
    let mut order: Vec<Spot> = (0..grid.size()).map(Spot::Grid).collect();
    if grid == Grid::Inventory {
        order.extend([39, 38, 37, 36].map(Spot::Player));
    }
    order.extend((HOTBAR..STORAGE).chain(0..HOTBAR).map(Spot::Player));
    if grid == Grid::Inventory {
        order.push(Spot::Player(40));
    }
    if right {
        order.reverse();
    }
    for pass in 0..2 {
        for spot in &order {
            if carried.count >= max {
                break;
            }
            let slot = match *spot {
                Spot::Player(index) => &mut inventory.slots[index],
                Spot::Grid(cell) => &mut cells_mut(inventory, grid)[cell],
                Spot::Result => continue,
            };
            let Some(stack) = slot.as_mut() else {
                continue;
            };
            if !stack.same_item(&carried) || (pass == 0 && stack.count >= max) {
                continue;
            }
            let moved = stack.count.min(max - carried.count);
            stack.count -= moved;
            carried.count += moved;
            if stack.count == 0 {
                *slot = None;
            }
        }
    }
    inventory.cursor = Some(carried);
}

pub fn swap(inventory: &mut Inventory, grid: Grid, cell: Option<usize>, hotbar: usize) -> bool {
    if hotbar >= HOTBAR && hotbar != OFFHAND {
        return false;
    }
    let Some(index) = cell else {
        if inventory.slots[hotbar].is_some() {
            return false;
        }
        let Some(result) = output(inventory, grid) else {
            return false;
        };
        let carried = inventory.cursor.take();
        let mut placed = result.clone();
        placed.max = limit(&inventory.recipes, &placed);
        let mut slots = inventory.slots.clone();
        slots[hotbar] = Some(placed);
        let before = std::mem::replace(&mut inventory.slots, slots);
        let taken = take_into_cursor(inventory, grid);
        if taken {
            inventory.cursor = None;
            inventory.notice_item_changed(&result);
        } else {
            inventory.slots = before;
        }
        inventory.cursor = carried;
        return taken;
    };
    if index >= grid.size() {
        return false;
    }
    let cells = match grid {
        Grid::Inventory => &mut inventory.crafting[..],
        Grid::Table => &mut inventory.workbench[..],
    };
    if cells[index].is_none() && inventory.slots[hotbar].is_none() {
        return false;
    }
    std::mem::swap(&mut cells[index], &mut inventory.slots[hotbar]);
    if let Some(stack) = inventory.slots[hotbar].clone() {
        inventory.notice_item_changed(&stack);
    }
    true
}

pub fn throw_cell(inventory: &mut Inventory, grid: Grid, cell: usize, whole: bool) -> Option<ItemStack> {
    let slot = cells_mut(inventory, grid).get_mut(cell)?;
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

pub fn throw_result(inventory: &mut Inventory, grid: Grid, whole: bool) -> Vec<ItemStack> {
    let mut thrown = Vec::new();
    if inventory.cursor.is_some() {
        return thrown;
    }
    let Some(first) = output(inventory, grid) else {
        return thrown;
    };
    while output(inventory, grid).is_some_and(|next| next.same_item(&first)) {
        if !take_into_cursor(inventory, grid) {
            break;
        }
        thrown.extend(inventory.cursor.take());
        if !whole {
            break;
        }
    }
    thrown
}

pub fn has_loose_items(inventory: &Inventory) -> bool {
    inventory.cursor.is_some()
        || inventory
            .crafting
            .iter()
            .chain(inventory.workbench.iter())
            .any(Option::is_some)
}

pub fn close(inventory: &mut Inventory, selected: usize) -> Vec<ItemStack> {
    let mut loose: Vec<ItemStack> = inventory.cursor.take().into_iter().collect();
    loose.extend(inventory.crafting.iter_mut().filter_map(Option::take));
    loose.extend(inventory.workbench.iter_mut().filter_map(Option::take));
    let mut thrown: Vec<ItemStack> = loose
        .into_iter()
        .filter_map(|stack| inventory.add_item(stack, selected))
        .collect();
    thrown.extend(inventory.take_pending_drops());
    thrown
}

pub fn is_crafting_table(block: &Block) -> bool {
    block.id.namespace == "minecraft" && block.id.path == "crafting_table"
}

pub fn opens_table(crouching: bool, holding: bool) -> bool {
    !(crouching && holding)
}

pub fn within_table_reach(eye: [f64; 3], pos: BlockPos) -> bool {
    let gap = |eye: f64, low: i32| {
        let low = f64::from(low);
        eye - eye.clamp(low, low + 1.0)
    };
    let (dx, dy, dz) = (gap(eye[0], pos.0), gap(eye[1], pos.1), gap(eye[2], pos.2));
    dx * dx + dy * dy + dz * dz < TABLE_REACH * TABLE_REACH
}

pub fn kit(spec: &str) -> Vec<(String, u32)> {
    spec.split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            let (id, count) = match entry.split_once('*') {
                Some((id, count)) => (id.trim(), count.trim().parse::<u32>().ok()?),
                None => (entry, 1),
            };
            if id.is_empty() || count == 0 {
                return None;
            }
            let id = if id.split('+').next().unwrap_or(id).contains(':') {
                id.to_owned()
            } else {
                format!("minecraft:{id}")
            };
            Some((id, count))
        })
        .collect()
}

pub fn give_kit(inventory: &mut Inventory, spec: &str) -> usize {
    let mut given = 0;
    for (entry, mut count) in kit(spec) {
        let (id, enchantments) = kit_enchantments(&entry);
        let max = u32::from(inventory.recipes.max_stack(id).max(1));
        while count > 0 {
            let Some(slot) = (HOTBAR..STORAGE).find(|&slot| inventory.slots[slot].is_none()) else {
                return given;
            };
            let size = count.min(max);
            count -= size;
            let mut stack = inventory.recipes.stack(id, size as u8);
            if !enchantments.is_empty() {
                let mut components = serde_json::Map::new();
                for (enchantment, level) in &enchantments {
                    minecraftoss_entities::enchant::add_enchantment(id, &mut components, enchantment, *level);
                }
                stack.components = Some(serde_json::Value::Object(components));
            }
            inventory.slots[slot] = Some(stack);
            given += 1;
        }
    }
    given
}

pub fn kit_enchantments(entry: &str) -> (&str, Vec<(String, i32)>) {
    let mut parts = entry.split('+');
    let id = parts.next().unwrap_or(entry).trim();
    let enchantments = parts
        .filter_map(|part| {
            let (name, level) = match part.split_once('=') {
                Some((name, level)) => (name.trim(), level.trim().parse::<i32>().ok()?),
                None => (part.trim(), 1),
            };
            if name.is_empty() {
                return None;
            }
            let name = if name.contains(':') {
                name.to_owned()
            } else {
                format!("minecraft:{name}")
            };
            Some((name, level))
        })
        .collect();
    (id, enchantments)
}

fn cells(inventory: &Inventory, grid: Grid) -> &[Option<ItemStack>] {
    match grid {
        Grid::Inventory => &inventory.crafting,
        Grid::Table => &inventory.workbench,
    }
}

fn cells_mut(inventory: &mut Inventory, grid: Grid) -> &mut [Option<ItemStack>] {
    match grid {
        Grid::Inventory => &mut inventory.crafting,
        Grid::Table => &mut inventory.workbench,
    }
}

fn take_into_cursor(inventory: &mut Inventory, grid: Grid) -> bool {
    match grid {
        Grid::Inventory => inventory.take_crafting_output(false),
        Grid::Table => inventory.take_workbench_output(false),
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

pub(crate) fn result_order() -> Vec<usize> {
    (0..HOTBAR).rev().chain((HOTBAR..STORAGE).rev()).collect()
}

fn limit(recipes: &RecipeBook, stack: &ItemStack) -> u8 {
    if stack.components.is_none() {
        stack.max.min(recipes.max_stack(&stack.id))
    } else {
        stack.max
    }
}

pub(crate) fn move_into(
    cells: &mut [Option<ItemStack>],
    stack: &mut ItemStack,
    order: &[usize],
) -> bool {
    let before = stack.count;
    for &index in order {
        if stack.count == 0 {
            break;
        }
        if let Some(target) = cells[index].as_mut() {
            if target.same_item(stack) {
                let room = stack.max.min(target.max).saturating_sub(target.count);
                let moved = room.min(stack.count);
                target.count += moved;
                stack.count -= moved;
            }
        }
    }
    for &index in order {
        if stack.count == 0 {
            break;
        }
        if cells[index].is_none() {
            let moved = stack.count.min(stack.max);
            cells[index] = Some(ItemStack {
                count: moved,
                ..stack.clone()
            });
            stack.count -= moved;
        }
    }
    stack.count < before
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn kit_entries_carry_enchantments() {
        let (id, enchantments) = kit_enchantments("minecraft:diamond_pickaxe+silk_touch+minecraft:efficiency=5");
        assert_eq!(id, "minecraft:diamond_pickaxe");
        assert_eq!(
            enchantments,
            vec![("minecraft:silk_touch".to_owned(), 1), ("minecraft:efficiency".to_owned(), 5)]
        );
        assert_eq!(kit_enchantments("minecraft:ice"), ("minecraft:ice", vec![]));
        assert_eq!(kit("diamond_pickaxe+minecraft:silk_touch,iron_sword+sharpness=5*2"), vec![("minecraft:diamond_pickaxe+minecraft:silk_touch".to_owned(), 1), ("minecraft:iron_sword+sharpness=5".to_owned(), 2)]);
        let mut inventory = Inventory::default();
        assert_eq!(give_kit(&mut inventory, "ice*2,diamond_pickaxe+silk_touch"), 2);
        assert_eq!(inventory.slots[HOTBAR].as_ref().map(|s| (s.id.as_str(), s.count, s.components.is_none())), Some(("minecraft:ice", 2, true)));
        let pickaxe = inventory.slots[HOTBAR + 1].as_ref().unwrap();
        assert_eq!(pickaxe.id, "minecraft:diamond_pickaxe");
        assert_eq!(pickaxe.components, Some(json!({"minecraft:enchantments": {"minecraft:silk_touch": 1}})));
    }

    fn write_jar(path: &Path, recipes: &[(&str, Value)], tags: &[(&str, Value)]) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let entries = recipes
            .iter()
            .map(|(name, value)| (format!("data/minecraft/recipe/{name}.json"), value))
            .chain(
                tags.iter()
                    .map(|(name, value)| (format!("data/minecraft/tags/item/{name}.json"), value)),
            );
        for (name, value) in entries {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(value.to_string().as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    fn shaped(pattern: &[&str], key: Value, result: &str, count: u8) -> Value {
        json!({
            "type": "minecraft:crafting_shaped",
            "pattern": pattern,
            "key": key,
            "result": {"id": result, "count": count}
        })
    }

    fn shapeless(ingredients: &[&str], result: &str, count: u8) -> Value {
        json!({
            "type": "minecraft:crafting_shapeless",
            "ingredients": ingredients,
            "result": {"id": result, "count": count}
        })
    }

    fn recipes() -> Vec<(&'static str, Value)> {
        let planks = "#minecraft:planks";
        let stick = "minecraft:stick";
        vec![
            (
                "cake",
                shapeless(
                    &["minecraft:milk_bucket", "minecraft:sugar", "minecraft:egg"],
                    "minecraft:cake",
                    1,
                ),
            ),
            (
                "flint_and_steel",
                shapeless(
                    &["minecraft:flint", "minecraft:iron_nugget"],
                    "minecraft:flint_and_steel",
                    1,
                ),
            ),
            (
                "flint_arrow",
                shapeless(&["minecraft:flint"], "minecraft:arrow", 2),
            ),
            (
                "honey_block",
                shaped(
                    &["HH", "HH"],
                    json!({"H": "minecraft:honey_bottle"}),
                    "minecraft:honey_block",
                    1,
                ),
            ),
            (
                "oak_planks",
                shapeless(&["#minecraft:logs"], "minecraft:oak_planks", 4),
            ),
            (
                "stick",
                shaped(&["#", "#"], json!({"#": planks}), "minecraft:stick", 4),
            ),
            (
                "stone_slab",
                shaped(
                    &["SSS"],
                    json!({"S": "minecraft:stone"}),
                    "minecraft:stone_slab",
                    6,
                ),
            ),
            (
                "wooden_axe",
                shaped(
                    &["PP", "PS", " S"],
                    json!({"P": planks, "S": stick}),
                    "minecraft:wooden_axe",
                    1,
                ),
            ),
            (
                "wooden_pickaxe",
                shaped(
                    &["PPP", " S ", " S "],
                    json!({"P": planks, "S": stick}),
                    "minecraft:wooden_pickaxe",
                    1,
                ),
            ),
        ]
    }

    fn tags() -> Vec<(&'static str, Value)> {
        vec![
            (
                "logs",
                json!({"values": ["minecraft:oak_log", "#minecraft:birch_logs"]}),
            ),
            ("birch_logs", json!({"values": ["minecraft:birch_log"]})),
            (
                "planks",
                json!({"values": ["minecraft:oak_planks", "minecraft:birch_planks"]}),
            ),
        ]
    }

    fn catalog_item(max_stack: u8) -> Value {
        json!({
            "fuel_component": false,
            "burn_ticks": 0,
            "speed_bits": "3f800000",
            "speed_decimal": "1.0",
            "max_stack": max_stack
        })
    }

    fn catalog_json() -> Value {
        let mut milk = catalog_item(1);
        milk["remainder"] = json!({"id": "minecraft:bucket", "count": 1});
        let mut honey = catalog_item(16);
        honey["remainder"] = json!({"id": "minecraft:glass_bottle", "count": 1});
        let mut helmet = catalog_item(1);
        helmet["equippable_slot"] = json!("head");
        json!({
            "schema_version": 1,
            "minecraft_version": "26.3",
            "items": {
                "minecraft:bucket": catalog_item(16),
                "minecraft:egg": catalog_item(16),
                "minecraft:cake": catalog_item(1),
                "minecraft:wooden_pickaxe": catalog_item(1),
                "minecraft:wooden_axe": catalog_item(1),
                "minecraft:flint_and_steel": catalog_item(1),
                "minecraft:milk_bucket": milk,
                "minecraft:honey_bottle": honey,
                "minecraft:iron_helmet": helmet
            }
        })
    }

    fn write_data(dir: &Path) -> PathBuf {
        let jar = dir.join("client.jar");
        write_jar(&jar, &recipes(), &tags());
        let catalog = dir.join(ITEM_CATALOG);
        std::fs::create_dir_all(catalog.parent().unwrap()).unwrap();
        std::fs::write(&catalog, catalog_json().to_string()).unwrap();
        jar
    }

    fn inventory() -> Inventory {
        let dir = crate::test_dir::tempdir().unwrap();
        let jar = write_data(dir.path());
        let catalog = find_item_catalog(Some(dir.path()), &jar);
        let (book, warning) = load_recipes(&jar, catalog.as_deref()).unwrap();
        assert!(warning.is_none());
        Inventory::default().with_recipes(book)
    }

    fn stack(inventory: &Inventory, id: &str, count: u8) -> Option<ItemStack> {
        Some(inventory.recipes.stack(id, count))
    }

    fn fill(inventory: &mut Inventory, id: &str) {
        let item = stack(inventory, id, 64);
        for slot in &mut inventory.slots[..STORAGE] {
            slot.clone_from(&item);
        }
    }

    fn put(inventory: &mut Inventory, grid: Grid, cells: &[usize], id: &str, count: u8) {
        for &cell in cells {
            let item = stack(inventory, id, count);
            cells_mut(inventory, grid)[cell] = item;
        }
    }

    fn id_of(slot: &Option<ItemStack>) -> Option<&str> {
        slot.as_ref().map(|stack| stack.id.as_str())
    }

    fn count_of(slot: &Option<ItemStack>) -> u8 {
        slot.as_ref().map_or(0, |stack| stack.count)
    }

    fn total(stacks: &[ItemStack], id: &str) -> u32 {
        stacks
            .iter()
            .filter(|stack| stack.id == id)
            .map(|stack| u32::from(stack.count))
            .sum()
    }

    fn output_id(inventory: &Inventory, grid: Grid) -> Option<String> {
        output(inventory, grid).map(|stack| stack.id)
    }

    #[test]
    fn shaped_recipe_matches_anywhere_in_the_grid() {
        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[2], "minecraft:oak_planks", 1);
        put(&mut inv, Grid::Table, &[5], "minecraft:birch_planks", 1);
        let made = output(&inv, Grid::Table).unwrap();
        assert_eq!((made.id.as_str(), made.count), ("minecraft:stick", 4));

        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[4, 7], "minecraft:oak_planks", 1);
        assert_eq!(
            output_id(&inv, Grid::Table).as_deref(),
            Some("minecraft:stick")
        );

        let mut inv = inventory();
        put(
            &mut inv,
            Grid::Inventory,
            &[1, 3],
            "minecraft:oak_planks",
            1,
        );
        assert_eq!(
            output_id(&inv, Grid::Inventory).as_deref(),
            Some("minecraft:stick")
        );
    }

    #[test]
    fn shaped_recipe_with_an_extra_item_does_not_match() {
        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[1, 4, 8], "minecraft:oak_planks", 1);
        assert_eq!(output(&inv, Grid::Table), None);

        let mut inv = inventory();
        put(
            &mut inv,
            Grid::Inventory,
            &[0, 2],
            "minecraft:oak_planks",
            1,
        );
        put(&mut inv, Grid::Inventory, &[1], "minecraft:birch_log", 1);
        assert_eq!(output(&inv, Grid::Inventory), None);
    }

    #[test]
    fn recipes_wider_than_two_only_craft_in_the_table() {
        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[3, 4, 5], "minecraft:stone", 1);
        let made = output(&inv, Grid::Table).unwrap();
        assert_eq!((made.id.as_str(), made.count), ("minecraft:stone_slab", 6));

        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[6, 7, 8], "minecraft:stone", 1);
        assert_eq!(
            output_id(&inv, Grid::Table).as_deref(),
            Some("minecraft:stone_slab")
        );

        let mut inv = inventory();
        put(&mut inv, Grid::Inventory, &[0, 1], "minecraft:stone", 1);
        assert_eq!(output(&inv, Grid::Inventory), None);

        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[0, 1, 2], "minecraft:oak_planks", 1);
        put(&mut inv, Grid::Table, &[4, 7], "minecraft:stick", 1);
        assert_eq!(
            output_id(&inv, Grid::Table).as_deref(),
            Some("minecraft:wooden_pickaxe")
        );
    }

    #[test]
    fn shaped_recipe_matches_its_mirror_image_but_not_upside_down() {
        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[0, 1, 3], "minecraft:oak_planks", 1);
        put(&mut inv, Grid::Table, &[4, 7], "minecraft:stick", 1);
        assert_eq!(
            output_id(&inv, Grid::Table).as_deref(),
            Some("minecraft:wooden_axe")
        );

        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[0, 1, 4], "minecraft:oak_planks", 1);
        put(&mut inv, Grid::Table, &[3, 6], "minecraft:stick", 1);
        assert_eq!(
            output_id(&inv, Grid::Table).as_deref(),
            Some("minecraft:wooden_axe")
        );

        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[1, 4], "minecraft:stick", 1);
        put(&mut inv, Grid::Table, &[3, 6, 7], "minecraft:oak_planks", 1);
        assert_eq!(output(&inv, Grid::Table), None);
    }

    #[test]
    fn shapeless_recipe_matches_in_any_order() {
        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[8], "minecraft:milk_bucket", 1);
        put(&mut inv, Grid::Table, &[0], "minecraft:sugar", 1);
        put(&mut inv, Grid::Table, &[4], "minecraft:egg", 1);
        assert_eq!(
            output_id(&inv, Grid::Table).as_deref(),
            Some("minecraft:cake")
        );

        let mut inv = inventory();
        put(&mut inv, Grid::Inventory, &[3], "minecraft:milk_bucket", 1);
        put(&mut inv, Grid::Inventory, &[0], "minecraft:egg", 1);
        put(&mut inv, Grid::Inventory, &[1], "minecraft:sugar", 1);
        assert_eq!(
            output_id(&inv, Grid::Inventory).as_deref(),
            Some("minecraft:cake")
        );
    }

    #[test]
    fn shapeless_recipe_accepts_any_item_of_a_nested_tag() {
        for log in ["minecraft:birch_log", "minecraft:oak_log"] {
            let mut inv = inventory();
            put(&mut inv, Grid::Inventory, &[2], log, 1);
            let made = output(&inv, Grid::Inventory).unwrap();
            assert_eq!((made.id.as_str(), made.count), ("minecraft:oak_planks", 4));
        }
        let mut inv = inventory();
        put(&mut inv, Grid::Inventory, &[2], "minecraft:spruce_log", 1);
        assert_eq!(output(&inv, Grid::Inventory), None);
    }

    #[test]
    fn shapeless_recipe_with_an_extra_or_missing_item_does_not_match() {
        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[0], "minecraft:milk_bucket", 1);
        put(&mut inv, Grid::Table, &[1], "minecraft:sugar", 1);
        put(&mut inv, Grid::Table, &[2, 3], "minecraft:egg", 1);
        assert_eq!(output(&inv, Grid::Table), None);

        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[0], "minecraft:milk_bucket", 1);
        put(&mut inv, Grid::Table, &[2], "minecraft:egg", 1);
        assert_eq!(output(&inv, Grid::Table), None);
    }

    #[test]
    fn milk_bucket_leaves_a_bucket_in_its_cell() {
        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[0], "minecraft:milk_bucket", 1);
        put(&mut inv, Grid::Table, &[1], "minecraft:sugar", 1);
        put(&mut inv, Grid::Table, &[2], "minecraft:egg", 1);
        assert!(take(&mut inv, Grid::Table));
        assert_eq!(id_of(&inv.cursor), Some("minecraft:cake"));
        let bucket = inv.workbench[0].as_ref().unwrap();
        assert_eq!(
            (bucket.id.as_str(), bucket.count, bucket.max),
            ("minecraft:bucket", 1, 16)
        );
        assert!(inv.workbench[1..].iter().all(Option::is_none));
        assert_eq!(output(&inv, Grid::Table), None);
    }

    #[test]
    fn remainder_goes_to_the_inventory_when_its_cell_still_holds_items() {
        let mut inv = inventory();
        put(
            &mut inv,
            Grid::Inventory,
            &[0, 1, 2, 3],
            "minecraft:honey_bottle",
            2,
        );
        assert!(take(&mut inv, Grid::Inventory));
        assert_eq!(id_of(&inv.cursor), Some("minecraft:honey_block"));
        assert!(inv.crafting.iter().all(|cell| count_of(cell) == 1));
        assert_eq!(id_of(&inv.slots[0]), Some("minecraft:glass_bottle"));
        assert_eq!(count_of(&inv.slots[0]), 4);
        assert!(inv.take_pending_drops().is_empty());
    }

    #[test]
    fn remainder_with_no_room_anywhere_waits_to_be_dropped() {
        let mut inv = inventory();
        fill(&mut inv, "minecraft:dirt");
        put(
            &mut inv,
            Grid::Inventory,
            &[0, 1, 2, 3],
            "minecraft:honey_bottle",
            2,
        );
        assert!(take(&mut inv, Grid::Inventory));
        assert_eq!(inv.count("minecraft:glass_bottle"), 0);
        assert_eq!(
            total(&inv.take_pending_drops(), "minecraft:glass_bottle"),
            4
        );
    }

    #[test]
    fn a_number_key_craft_keeps_its_remainders_beside_the_result() {
        let mut inv = inventory();
        put(
            &mut inv,
            Grid::Inventory,
            &[0, 1, 2, 3],
            "minecraft:honey_bottle",
            2,
        );
        assert!(swap(&mut inv, Grid::Inventory, None, 0));
        assert_eq!(id_of(&inv.slots[0]), Some("minecraft:honey_block"));
        assert_eq!(inv.count("minecraft:glass_bottle"), 4);
        assert!(inv.take_pending_drops().is_empty());
    }

    #[test]
    fn craft_all_crafts_until_an_input_runs_out() {
        let mut inv = inventory();
        put(&mut inv, Grid::Inventory, &[0], "minecraft:oak_log", 10);
        assert_eq!(craft_all(&mut inv, Grid::Inventory), 10);
        assert_eq!(id_of(&inv.slots[8]), Some("minecraft:oak_planks"));
        assert_eq!(count_of(&inv.slots[8]), 40);
        assert!(inv.crafting.iter().all(Option::is_none));
    }

    #[test]
    fn craft_all_merges_hotbar_right_to_left_then_main_bottom_up_and_skips_the_offhand() {
        let mut inv = inventory();
        inv.slots[3] = stack(&inv, "minecraft:oak_planks", 60);
        inv.slots[20] = stack(&inv, "minecraft:oak_planks", 62);
        inv.slots[40] = stack(&inv, "minecraft:oak_planks", 10);
        put(&mut inv, Grid::Inventory, &[0], "minecraft:oak_log", 3);
        assert_eq!(craft_all(&mut inv, Grid::Inventory), 3);
        assert_eq!(count_of(&inv.slots[3]), 64);
        assert_eq!(count_of(&inv.slots[20]), 64);
        assert_eq!(count_of(&inv.slots[8]), 6);
        assert_eq!(count_of(&inv.slots[40]), 10);
    }

    #[test]
    fn craft_all_fills_empty_hotbar_slots_before_the_main_inventory() {
        let mut inv = inventory();
        fill(&mut inv, "minecraft:dirt");
        inv.slots[1] = None;
        for slot in &mut inv.slots[HOTBAR..STORAGE] {
            *slot = None;
        }
        put(&mut inv, Grid::Table, &[0, 1, 2], "minecraft:oak_planks", 3);
        put(&mut inv, Grid::Table, &[4, 7], "minecraft:stick", 3);
        assert_eq!(craft_all(&mut inv, Grid::Table), 3);
        for slot in [1, 35, 34] {
            assert_eq!(id_of(&inv.slots[slot]), Some("minecraft:wooden_pickaxe"));
            assert_eq!(count_of(&inv.slots[slot]), 1);
        }
        assert!(inv.slots[33].is_none());
    }

    #[test]
    fn craft_all_stops_before_a_result_that_would_not_fit_whole() {
        let mut inv = inventory();
        fill(&mut inv, "minecraft:dirt");
        inv.slots[5] = stack(&inv, "minecraft:oak_planks", 56);
        put(&mut inv, Grid::Inventory, &[0], "minecraft:oak_log", 5);
        assert_eq!(craft_all(&mut inv, Grid::Inventory), 2);
        assert_eq!(count_of(&inv.slots[5]), 64);
        assert_eq!(count_of(&inv.crafting[0]), 3);
        assert!(inv.take_pending_drops().is_empty());
    }

    #[test]
    fn craft_all_stops_when_the_result_changes() {
        let mut inv = inventory();
        put(&mut inv, Grid::Inventory, &[0], "minecraft:flint", 3);
        put(&mut inv, Grid::Inventory, &[1], "minecraft:iron_nugget", 1);
        assert_eq!(craft_all(&mut inv, Grid::Inventory), 1);
        assert_eq!(inv.count("minecraft:flint_and_steel"), 1);
        assert_eq!(count_of(&inv.crafting[0]), 2);
        assert_eq!(inv.count("minecraft:arrow"), 0);
    }

    #[test]
    fn craft_all_leaves_the_carried_stack_on_the_cursor() {
        let mut inv = inventory();
        inv.cursor = stack(&inv, "minecraft:oak_planks", 3);
        put(&mut inv, Grid::Inventory, &[0], "minecraft:oak_log", 2);
        assert_eq!(craft_all(&mut inv, Grid::Inventory), 2);
        assert_eq!(count_of(&inv.cursor), 3);
        assert_eq!(count_of(&inv.slots[8]), 8);
    }

    #[test]
    fn craft_all_places_each_result_before_its_remainders() {
        let mut inv = inventory();
        fill(&mut inv, "minecraft:dirt");
        inv.slots[0] = None;
        put(
            &mut inv,
            Grid::Inventory,
            &[0, 1, 2, 3],
            "minecraft:honey_bottle",
            2,
        );
        assert_eq!(craft_all(&mut inv, Grid::Inventory), 2);
        assert_eq!(id_of(&inv.slots[0]), Some("minecraft:honey_block"));
        assert_eq!(count_of(&inv.slots[0]), 2);
        assert!(
            inv.crafting
                .iter()
                .all(|cell| id_of(cell) == Some("minecraft:glass_bottle") && count_of(cell) == 1)
        );
        assert_eq!(
            total(&inv.take_pending_drops(), "minecraft:glass_bottle"),
            4
        );
    }

    #[test]
    fn shift_click_on_a_grid_cell_merges_then_fills_main_before_hotbar() {
        let mut inv = inventory();
        inv.slots[4] = stack(&inv, "minecraft:oak_log", 60);
        put(&mut inv, Grid::Table, &[2], "minecraft:oak_log", 10);
        assert!(quick_move_from_grid(&mut inv, Grid::Table, 2));
        assert_eq!(count_of(&inv.slots[4]), 64);
        assert_eq!(count_of(&inv.slots[9]), 6);
        assert!(inv.workbench[2].is_none());
    }

    #[test]
    fn shift_click_on_a_grid_cell_keeps_what_does_not_fit() {
        let mut inv = inventory();
        fill(&mut inv, "minecraft:dirt");
        inv.slots[30] = stack(&inv, "minecraft:oak_log", 62);
        put(&mut inv, Grid::Table, &[6], "minecraft:oak_log", 5);
        assert!(quick_move_from_grid(&mut inv, Grid::Table, 6));
        assert_eq!(count_of(&inv.slots[30]), 64);
        assert_eq!(count_of(&inv.workbench[6]), 3);
        assert!(!quick_move_from_grid(&mut inv, Grid::Table, 6));
    }

    #[test]
    fn shift_click_in_the_table_puts_inventory_stacks_into_the_grid_first() {
        let mut inv = inventory();
        inv.slots[12] = stack(&inv, "minecraft:oak_log", 10);
        inv.slots[13] = stack(&inv, "minecraft:oak_log", 5);
        inv.slots[14] = stack(&inv, "minecraft:iron_helmet", 1);
        assert!(quick_move_into_table(&mut inv, 12));
        assert_eq!(count_of(&inv.workbench[0]), 10);
        assert!(quick_move_into_table(&mut inv, 13));
        assert_eq!(count_of(&inv.workbench[0]), 15);
        assert!(quick_move_into_table(&mut inv, 14));
        assert_eq!(id_of(&inv.workbench[1]), Some("minecraft:iron_helmet"));
        assert!(inv.slots[39].is_none());
    }

    #[test]
    fn shift_click_in_a_full_table_moves_between_main_and_hotbar() {
        let mut inv = inventory();
        put(
            &mut inv,
            Grid::Table,
            &[0, 1, 2, 3, 5, 6, 7, 8],
            "minecraft:stone",
            1,
        );
        put(&mut inv, Grid::Table, &[4], "minecraft:oak_log", 60);
        inv.slots[20] = stack(&inv, "minecraft:oak_log", 10);
        assert!(quick_move_into_table(&mut inv, 20));
        assert_eq!(count_of(&inv.workbench[4]), 64);
        assert_eq!(count_of(&inv.slots[0]), 6);
        assert!(inv.slots[20].is_none());

        inv.slots[3] = stack(&inv, "minecraft:birch_log", 5);
        assert!(quick_move_into_table(&mut inv, 3));
        assert_eq!(id_of(&inv.slots[9]), Some("minecraft:birch_log"));
        assert!(inv.slots[3].is_none());
    }

    #[test]
    fn shift_click_in_the_table_keeps_what_fits_nowhere() {
        let mut inv = inventory();
        put(
            &mut inv,
            Grid::Table,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8],
            "minecraft:stone",
            1,
        );
        fill(&mut inv, "minecraft:dirt");
        for slot in &mut inv.slots[HOTBAR..STORAGE] {
            *slot = None;
        }
        inv.slots[20] = stack(&inv, "minecraft:oak_log", 5);
        assert!(!quick_move_into_table(&mut inv, 20));
        assert_eq!(count_of(&inv.slots[20]), 5);
    }

    #[test]
    fn dragging_over_grid_cells_shares_the_carried_stack() {
        let mut inv = inventory();
        inv.cursor = stack(&inv, "minecraft:oak_log", 9);
        let targets = [
            Spot::Grid(0),
            Spot::Grid(4),
            Spot::Grid(8),
            Spot::Player(20),
            Spot::Grid(4),
            Spot::Result,
        ];
        spread(&mut inv, Grid::Table, &targets, false);
        for cell in [0, 4, 8] {
            assert_eq!(count_of(&inv.workbench[cell]), 2);
        }
        assert_eq!(count_of(&inv.slots[20]), 2);
        assert_eq!(count_of(&inv.cursor), 1);

        let mut inv = inventory();
        inv.cursor = stack(&inv, "minecraft:oak_log", 3);
        spread(&mut inv, Grid::Table, &[Spot::Grid(1), Spot::Grid(2)], true);
        assert_eq!(count_of(&inv.workbench[1]), 1);
        assert_eq!(count_of(&inv.workbench[2]), 1);
        assert_eq!(count_of(&inv.cursor), 1);
    }

    #[test]
    fn dragging_ignores_slots_the_screen_does_not_show() {
        let mut inv = inventory();
        inv.cursor = stack(&inv, "minecraft:oak_log", 4);
        spread(
            &mut inv,
            Grid::Table,
            &[Spot::Player(40), Spot::Grid(1)],
            false,
        );
        assert_eq!(count_of(&inv.workbench[1]), 4);
        assert!(inv.slots[40].is_none());

        let mut inv = inventory();
        inv.cursor = stack(&inv, "minecraft:oak_log", 4);
        spread(
            &mut inv,
            Grid::Inventory,
            &[Spot::Grid(4), Spot::Player(40)],
            false,
        );
        assert_eq!(count_of(&inv.slots[40]), 4);
        assert!(inv.crafting.iter().all(Option::is_none));
    }

    #[test]
    fn left_drag_over_more_slots_than_items_gives_one_each_and_leaves_no_empty_stacks() {
        let mut inv = inventory();
        inv.cursor = stack(&inv, "minecraft:dirt", 2);
        let targets: Vec<Spot> = (9..14).map(Spot::Player).collect();
        spread(&mut inv, Grid::Inventory, &targets, false);
        assert_eq!(count_of(&inv.slots[9]), 1);
        assert_eq!(count_of(&inv.slots[10]), 1);
        assert!(inv.slots[11..14].iter().all(Option::is_none));
        assert!(inv.cursor.is_none());
        assert!(inv.slots.iter().flatten().all(|stack| stack.count > 0));
    }

    #[test]
    fn closing_returns_the_cursor_and_both_grids_to_the_inventory() {
        let mut inv = inventory();
        inv.cursor = stack(&inv, "minecraft:dirt", 3);
        put(&mut inv, Grid::Inventory, &[2], "minecraft:oak_log", 2);
        put(&mut inv, Grid::Table, &[5], "minecraft:stone", 7);
        assert!(has_loose_items(&inv));
        assert!(close(&mut inv, 0).is_empty());
        assert_eq!(id_of(&inv.slots[0]), Some("minecraft:dirt"));
        assert_eq!(id_of(&inv.slots[1]), Some("minecraft:oak_log"));
        assert_eq!(id_of(&inv.slots[2]), Some("minecraft:stone"));
        assert!(!has_loose_items(&inv));
    }

    #[test]
    fn closing_fills_the_selected_slot_first() {
        let mut inv = inventory();
        inv.slots[0] = stack(&inv, "minecraft:oak_log", 10);
        inv.slots[6] = stack(&inv, "minecraft:oak_log", 10);
        inv.cursor = stack(&inv, "minecraft:oak_log", 5);
        assert!(close(&mut inv, 6).is_empty());
        assert_eq!(count_of(&inv.slots[6]), 15);
        assert_eq!(count_of(&inv.slots[0]), 10);
    }

    #[test]
    fn closing_a_full_inventory_hands_back_what_does_not_fit() {
        let mut inv = inventory();
        fill(&mut inv, "minecraft:dirt");
        inv.slots[4] = stack(&inv, "minecraft:oak_log", 62);
        inv.cursor = stack(&inv, "minecraft:oak_log", 3);
        put(&mut inv, Grid::Table, &[0], "minecraft:stone", 7);
        let thrown = close(&mut inv, 0);
        assert_eq!(total(&thrown, "minecraft:oak_log"), 1);
        assert_eq!(total(&thrown, "minecraft:stone"), 7);
        assert_eq!(count_of(&inv.slots[4]), 64);
        assert!(!has_loose_items(&inv));
    }

    #[test]
    fn closing_also_hands_back_remainders_waiting_to_drop() {
        let mut inv = inventory();
        fill(&mut inv, "minecraft:dirt");
        put(
            &mut inv,
            Grid::Inventory,
            &[0, 1, 2, 3],
            "minecraft:honey_bottle",
            2,
        );
        assert!(take(&mut inv, Grid::Inventory));
        let thrown = close(&mut inv, 0);
        assert_eq!(total(&thrown, "minecraft:honey_block"), 1);
        assert_eq!(total(&thrown, "minecraft:honey_bottle"), 4);
        assert_eq!(total(&thrown, "minecraft:glass_bottle"), 4);
        for id in [
            "minecraft:honey_block",
            "minecraft:honey_bottle",
            "minecraft:glass_bottle",
        ] {
            assert_eq!(inv.count(id), 0);
        }
    }

    #[test]
    fn q_on_the_result_crafts_and_drops_one_result_and_ctrl_q_crafts_all() {
        let mut inv = inventory();
        put(&mut inv, Grid::Inventory, &[0], "minecraft:oak_log", 3);
        let one = throw_result(&mut inv, Grid::Inventory, false);
        assert_eq!(total(&one, "minecraft:oak_planks"), 4);
        assert_eq!(count_of(&inv.crafting[0]), 2);
        let all = throw_result(&mut inv, Grid::Inventory, true);
        assert_eq!(total(&all, "minecraft:oak_planks"), 8);
        assert!(inv.crafting[0].is_none());
        assert!(inv.cursor.is_none());
        put(&mut inv, Grid::Inventory, &[0], "minecraft:flint", 3);
        put(&mut inv, Grid::Inventory, &[1], "minecraft:iron_nugget", 1);
        let changed = throw_result(&mut inv, Grid::Inventory, true);
        assert_eq!(total(&changed, "minecraft:flint_and_steel"), 1);
        assert_eq!(total(&changed, "minecraft:arrow"), 0);
    }

    #[test]
    fn q_on_a_grid_cell_drops_one_or_the_whole_stack() {
        let mut inv = inventory();
        put(&mut inv, Grid::Table, &[4], "minecraft:stick", 5);
        assert_eq!(throw_cell(&mut inv, Grid::Table, 4, false).map(|s| s.count), Some(1));
        assert_eq!(count_of(&inv.workbench[4]), 4);
        assert_eq!(throw_cell(&mut inv, Grid::Table, 4, true).map(|s| s.count), Some(4));
        assert!(inv.workbench[4].is_none());
        assert!(throw_cell(&mut inv, Grid::Table, 4, true).is_none());
    }

    #[test]
    fn f_swaps_a_grid_cell_with_the_offhand() {
        let mut inv = inventory();
        put(&mut inv, Grid::Inventory, &[2], "minecraft:stick", 5);
        assert!(swap(&mut inv, Grid::Inventory, Some(2), OFFHAND));
        assert_eq!(id_of(&inv.slots[OFFHAND]), Some("minecraft:stick"));
        assert!(inv.crafting[2].is_none());
        assert!(!swap(&mut inv, Grid::Inventory, Some(2), 41));
    }

    #[test]
    fn the_item_catalog_sets_stack_limits_and_remainders() {
        let mut inv = inventory();
        assert_eq!(inv.recipes.max_stack("minecraft:bucket"), 16);
        assert_eq!(
            inv.recipes.crafting_remainder("minecraft:milk_bucket"),
            stack(&inv, "minecraft:bucket", 1)
        );
        put(&mut inv, Grid::Table, &[0, 1, 2], "minecraft:oak_planks", 1);
        put(&mut inv, Grid::Table, &[4, 7], "minecraft:stick", 1);
        assert_eq!(output(&inv, Grid::Table).unwrap().max, 1);
    }
}
