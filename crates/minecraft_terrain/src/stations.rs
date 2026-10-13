use std::io::Read;
use std::path::Path;

use anyhow::Result;
use minecraftoss_entities::enchant::{Enchantments, add_enchantment};
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::rng::{LegacyRandom, LootRandom};
use serde_json::{Map, Value};

use crate::containers::pick;
use crate::crafting::{move_into, result_order};
use crate::gun_mods::{self, GunRules};
use crate::scene::{Block, BlockPos};

const HOTBAR: usize = 9;
const STORAGE: usize = 36;
const OFFHAND: usize = 40;
pub const TOO_EXPENSIVE: u32 = 40;
const NAME_LIMIT: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workshop {
    Stonecutter,
    Smithing,
    Grindstone,
    Anvil,
    Enchanting,
}

impl Workshop {
    pub fn of(block: &Block) -> Option<Self> {
        if block.id.namespace != "minecraft" {
            return None;
        }
        match block.id.path.as_str() {
            "stonecutter" => Some(Self::Stonecutter),
            "smithing_table" => Some(Self::Smithing),
            "grindstone" => Some(Self::Grindstone),
            "anvil" | "chipped_anvil" | "damaged_anvil" => Some(Self::Anvil),
            "enchanting_table" => Some(Self::Enchanting),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Stonecutter => "STONECUTTER",
            Self::Smithing => "SMITHING TABLE",
            Self::Grindstone => "GRINDSTONE",
            Self::Anvil => "ANVIL",
            Self::Enchanting => "ENCHANTING TABLE",
        }
    }

    pub fn inputs(self) -> usize {
        match self {
            Self::Stonecutter => 1,
            Self::Smithing => 3,
            Self::Grindstone | Self::Anvil | Self::Enchanting => 2,
        }
    }
}

#[derive(Clone, Debug)]
enum Ingredient {
    Items(Vec<String>),
    Tag(String),
}

impl Ingredient {
    fn parse(value: &Value) -> Option<Self> {
        match value {
            Value::String(id) => Some(match id.strip_prefix('#') {
                Some(tag) => Self::Tag(tag.to_owned()),
                None => Self::Items(vec![id.clone()]),
            }),
            Value::Array(list) => Some(Self::Items(
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            )),
            Value::Object(object) => object.get("item").and_then(Self::parse),
            _ => None,
        }
    }

    fn matches(&self, id: &str, recipes: &RecipeBook) -> bool {
        match self {
            Self::Items(ids) => ids.iter().any(|item| item == id),
            Self::Tag(tag) => recipes.item_in_tag(tag, id),
        }
    }
}

#[derive(Clone, Debug)]
enum Smithed {
    Transform(String),
    Trim(String),
}

#[derive(Clone, Debug)]
struct Smithing {
    template: Ingredient,
    base: Ingredient,
    addition: Ingredient,
    result: Smithed,
}

#[derive(Clone, Debug)]
struct Brewing {
    item: String,
    potion: String,
    reagent: Ingredient,
    output: String,
    components: Value,
}

#[derive(Clone, Debug, Default)]
pub struct StationBook {
    stonecutting: Vec<(Ingredient, String, u8)>,
    smithing: Vec<Smithing>,
    brewing: Vec<Brewing>,
    campfire: Vec<(Ingredient, String, u32)>,
}

pub fn potion_of(stack: &ItemStack) -> Option<&str> {
    stack
        .components
        .as_ref()?
        .get("minecraft:potion_contents")?
        .get("potion")?
        .as_str()
}

impl StationBook {
    pub fn from_jar(jar: &Path) -> Result<Self> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(jar)?)?;
        let mut entries = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name().to_owned();
            if !name.starts_with("data/minecraft/recipe/") || !name.ends_with(".json") {
                continue;
            }
            let mut text = String::new();
            entry.read_to_string(&mut text)?;
            entries.push((name, serde_json::from_str::<Value>(&text)?));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut book = Self::default();
        for (_, json) in entries {
            book.add(&json);
        }
        Ok(book)
    }

    fn add(&mut self, json: &Value) {
        let result_id = json["result"]["id"].as_str().map(str::to_owned);
        match json["type"].as_str() {
            Some("minecraft:stonecutting") => {
                if let (Some(ingredient), Some(id)) =
                    (Ingredient::parse(&json["ingredient"]), result_id)
                {
                    let count = json["result"]["count"].as_u64().unwrap_or(1).min(64) as u8;
                    self.stonecutting.push((ingredient, id, count));
                }
            }
            Some(kind @ ("minecraft:smithing_transform" | "minecraft:smithing_trim")) => {
                let parts = (
                    Ingredient::parse(&json["template"]),
                    Ingredient::parse(&json["base"]),
                    Ingredient::parse(&json["addition"]),
                );
                let result = if kind == "minecraft:smithing_trim" {
                    json["pattern"]
                        .as_str()
                        .map(|p| Smithed::Trim(p.to_owned()))
                } else {
                    result_id.map(Smithed::Transform)
                };
                if let ((Some(template), Some(base), Some(addition)), Some(result)) =
                    (parts, result)
                {
                    self.smithing.push(Smithing {
                        template,
                        base,
                        addition,
                        result,
                    });
                }
            }
            Some("minecraft:brewing") => {
                let input = &json["input"];
                let fields = (
                    input["item"].as_str(),
                    input["potion_contents"]["potions"].as_str(),
                    Ingredient::parse(&json["reagent"]),
                    json["output"]["id"].as_str(),
                );
                if let (Some(item), Some(potion), Some(reagent), Some(output)) = fields {
                    self.brewing.push(Brewing {
                        item: item.to_owned(),
                        potion: potion.to_owned(),
                        reagent,
                        output: output.to_owned(),
                        components: json["output"]["components"].clone(),
                    });
                }
            }
            Some("minecraft:campfire_cooking") => {
                let ticks = json["cookingtime"].as_u64().and_then(|t| u32::try_from(t).ok());
                if let (Some(ingredient), Some(id), Some(ticks)) =
                    (Ingredient::parse(&json["ingredient"]), result_id, ticks)
                {
                    self.campfire.push((ingredient, id, ticks));
                }
            }
            _ => {}
        }
    }

    pub fn campfire_cooking(&self, id: &str, recipes: &RecipeBook) -> Option<(ItemStack, u32)> {
        self.campfire
            .iter()
            .find(|(ingredient, ..)| ingredient.matches(id, recipes))
            .map(|(_, result, ticks)| (recipes.stack(result, 1), *ticks))
    }

    pub fn stonecutter_results(&self, input: &str, recipes: &RecipeBook) -> Vec<ItemStack> {
        self.stonecutting
            .iter()
            .filter(|(ingredient, ..)| ingredient.matches(input, recipes))
            .map(|(_, id, count)| recipes.stack(id, *count))
            .collect()
    }

    fn smithed(
        &self,
        template: &ItemStack,
        base: &ItemStack,
        addition: &ItemStack,
        recipes: &RecipeBook,
    ) -> Option<ItemStack> {
        let recipe = self.smithing.iter().find(|r| {
            r.template.matches(&template.id, recipes)
                && r.base.matches(&base.id, recipes)
                && r.addition.matches(&addition.id, recipes)
        })?;
        match &recipe.result {
            Smithed::Transform(id) => Some(ItemStack {
                id: id.clone(),
                count: 1,
                max: recipes.max_stack(id).max(1),
                components: base.components.clone(),
            }),
            Smithed::Trim(pattern) => {
                let material = trim_material(&addition.id)?;
                let mut components = object(base.components.clone());
                let trim = serde_json::json!({"material": material, "pattern": pattern});
                if components.get("minecraft:trim") == Some(&trim) {
                    return None;
                }
                components.insert("minecraft:trim".to_owned(), trim);
                Some(ItemStack {
                    count: 1,
                    components: Some(Value::Object(components)),
                    ..base.clone()
                })
            }
        }
    }

    fn smithing_slot(&self, slot: usize, id: &str, recipes: &RecipeBook) -> bool {
        self.smithing.iter().any(|r| match slot {
            0 => r.template.matches(id, recipes),
            1 => r.base.matches(id, recipes),
            _ => r.addition.matches(id, recipes),
        })
    }

    pub fn brew(
        &self,
        bottle: &ItemStack,
        reagent: &str,
        recipes: &RecipeBook,
    ) -> Option<ItemStack> {
        let potion = potion_of(bottle)?;
        let recipe = self.brewing.iter().find(|r| {
            r.item == bottle.id && r.potion == potion && r.reagent.matches(reagent, recipes)
        })?;
        let components = recipe.components.as_object().filter(|c| !c.is_empty());
        Some(ItemStack {
            id: recipe.output.clone(),
            count: 1,
            max: 1,
            components: components.map(|c| Value::Object(c.clone())),
        })
    }

    pub fn is_reagent(&self, id: &str, recipes: &RecipeBook) -> bool {
        self.brewing.iter().any(|r| r.reagent.matches(id, recipes))
    }
}

fn trim_material(addition: &str) -> Option<String> {
    let path = addition.strip_prefix("minecraft:")?;
    let material = match path {
        "amethyst_shard" => "amethyst",
        "copper_ingot" => "copper",
        "diamond" => "diamond",
        "emerald" => "emerald",
        "gold_ingot" => "gold",
        "iron_ingot" => "iron",
        "lapis_lazuli" => "lapis",
        "netherite_ingot" => "netherite",
        "quartz" => "quartz",
        "redstone" => "redstone",
        "resin_brick" => "resin",
        _ => return None,
    };
    Some(format!("minecraft:{material}"))
}

fn object(components: Option<Value>) -> Map<String, Value> {
    match components {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn int_component(stack: &ItemStack, key: &str) -> i64 {
    stack
        .components
        .as_ref()
        .and_then(|c| c.get(key))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

fn enchantment_key(stack: &ItemStack) -> &'static str {
    if stack.id == "minecraft:enchanted_book" {
        "minecraft:stored_enchantments"
    } else {
        "minecraft:enchantments"
    }
}

pub fn enchantments_of(stack: &ItemStack) -> Vec<(String, i32)> {
    let mut out: Vec<(String, i32)> = stack
        .components
        .as_ref()
        .and_then(|c| c.get(enchantment_key(stack)))
        .and_then(Value::as_object)
        .map_or_else(Vec::new, |map| {
            map.iter()
                .filter_map(|(id, level)| level.as_i64().map(|l| (id.clone(), l as i32)))
                .collect()
        });
    out.sort();
    out
}

fn set_enchantments(stack: &mut ItemStack, list: &[(String, i32)]) {
    let key = enchantment_key(stack);
    let mut components = object(stack.components.take());
    components.remove(key);
    if !list.is_empty() {
        let map: Map<String, Value> = list
            .iter()
            .map(|(id, level)| (id.clone(), Value::from(*level)))
            .collect();
        components.insert(key.to_owned(), Value::Object(map));
    }
    stack.components = (!components.is_empty()).then_some(Value::Object(components));
}

fn set_int(stack: &mut ItemStack, key: &str, value: Option<i64>) {
    let mut components = object(stack.components.take());
    match value {
        Some(value) => {
            components.insert(key.to_owned(), Value::from(value));
        }
        None => {
            components.remove(key);
        }
    }
    stack.components = (!components.is_empty()).then_some(Value::Object(components));
}

pub fn custom_name(stack: &ItemStack) -> Option<String> {
    let name = stack.components.as_ref()?.get("minecraft:custom_name")?;
    match name {
        Value::String(text) => Some(text.clone()),
        other => other.get("text").and_then(Value::as_str).map(str::to_owned),
    }
}

pub fn repair_material(item: &str) -> Option<&'static str> {
    let path = item.strip_prefix("minecraft:")?;
    let armor = ["_helmet", "_chestplate", "_leggings", "_boots"]
        .iter()
        .any(|part| path.ends_with(part));
    let tool = ["_sword", "_pickaxe", "_axe", "_shovel", "_hoe", "_spear"]
        .iter()
        .any(|part| path.ends_with(part));
    let material = path.split('_').next()?;
    Some(match (material, armor, tool) {
        ("leather", true, _) => "minecraft:repairs_leather_armor",
        ("chainmail", true, _) => "minecraft:repairs_chain_armor",
        ("iron", true, _) => "minecraft:repairs_iron_armor",
        ("golden", true, _) => "minecraft:repairs_gold_armor",
        ("diamond", true, _) => "minecraft:repairs_diamond_armor",
        ("netherite", true, _) => "minecraft:repairs_netherite_armor",
        ("copper", true, _) => "minecraft:repairs_copper_armor",
        ("turtle", true, _) => "minecraft:repairs_turtle_helmet",
        ("wooden", _, true) => "minecraft:wooden_tool_materials",
        ("stone", _, true) => "minecraft:stone_tool_materials",
        ("iron", _, true) => "minecraft:iron_tool_materials",
        ("golden", _, true) => "minecraft:gold_tool_materials",
        ("diamond", _, true) => "minecraft:diamond_tool_materials",
        ("netherite", _, true) => "minecraft:netherite_tool_materials",
        ("copper", _, true) => "minecraft:copper_tool_materials",
        _ => return None,
    })
}

fn repairs(item: &str, material: &str, recipes: &RecipeBook) -> bool {
    match item {
        "minecraft:elytra" => material == "minecraft:phantom_membrane",
        "minecraft:mace" => material == "minecraft:breeze_rod",
        "minecraft:shield" => recipes.item_in_tag("minecraft:wooden_tool_materials", material),
        "minecraft:wolf_armor" => material == "minecraft:armadillo_scute",
        _ => repair_material(item).is_some_and(|tag| recipes.item_in_tag(tag, material)),
    }
}

pub struct Context<'a> {
    pub recipes: &'a RecipeBook,
    pub book: &'a StationBook,
    pub enchantments: Option<&'a Enchantments>,
    pub bookshelves: i32,
    pub seed: i32,
    pub level: u32,
    pub guns: Option<&'a dyn GunRules>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Choice {
    pub item: Option<ItemStack>,
    pub cost: u32,
    pub clue: Option<(String, i32)>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Offer {
    pub result: Option<ItemStack>,
    pub cost: u32,
    pub choices: Vec<Choice>,
    pub repair_items: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Taken {
    pub levels: u32,
    pub experience: u32,
    pub sound: &'static str,
    pub new_seed: bool,
}

#[derive(Clone, Debug)]
pub struct Bench {
    pub kind: Workshop,
    pub cells: Vec<Option<ItemStack>>,
    pub chosen: Option<usize>,
    pub name: Option<String>,
}

impl Bench {
    pub fn new(kind: Workshop) -> Self {
        Self {
            kind,
            cells: vec![None; kind.inputs()],
            chosen: None,
            name: None,
        }
    }

    pub fn offer(&self, ctx: &Context<'_>) -> Offer {
        match self.kind {
            Workshop::Stonecutter => {
                let choices: Vec<Choice> = self.cells[0].as_ref().map_or_else(Vec::new, |input| {
                    ctx.book
                        .stonecutter_results(&input.id, ctx.recipes)
                        .into_iter()
                        .map(|item| Choice {
                            item: Some(item),
                            enabled: true,
                            ..Choice::default()
                        })
                        .collect()
                });
                let result = self
                    .chosen
                    .and_then(|i| choices.get(i))
                    .and_then(|c| c.item.clone());
                Offer {
                    result,
                    choices,
                    ..Offer::default()
                }
            }
            Workshop::Smithing => {
                let result = match (&self.cells[0], &self.cells[1], &self.cells[2]) {
                    (Some(t), Some(b), Some(a)) => ctx.book.smithed(t, b, a, ctx.recipes),
                    _ => None,
                };
                Offer {
                    result,
                    ..Offer::default()
                }
            }
            Workshop::Grindstone => Offer {
                result: grind(&self.cells[0], &self.cells[1], ctx),
                ..Offer::default()
            },
            Workshop::Anvil => anvil(&self.cells[0], &self.cells[1], self.name.as_deref(), ctx),
            Workshop::Enchanting => Offer {
                choices: enchanting_offers(&self.cells[0], &self.cells[1], ctx),
                ..Offer::default()
            },
        }
    }

    fn accepts(&self, index: usize, stack: &ItemStack, ctx: &Context<'_>) -> bool {
        accepts(self.kind, index, stack, ctx)
    }

    fn input_changed(&mut self, before: &Option<ItemStack>) {
        let same = match (before, &self.cells[0]) {
            (Some(a), Some(b)) => a.same_item(b),
            _ => false,
        };
        if !same {
            self.chosen = None;
            if self.kind == Workshop::Anvil {
                self.name = None;
            }
        }
    }

    pub fn swap(
        &mut self,
        index: usize,
        hotbar: usize,
        inventory: &mut Inventory,
        ctx: &Context<'_>,
    ) {
        if index >= self.cells.len() || (hotbar >= HOTBAR && hotbar != OFFHAND) {
            return;
        }
        let before = self.cells[0].clone();
        if let Some(item) = inventory.slots[hotbar].as_ref() {
            if !self.accepts(index, item, ctx) {
                return;
            }
        }
        std::mem::swap(&mut self.cells[index], &mut inventory.slots[hotbar]);
        self.input_changed(&before);
    }

    pub fn spread(
        &mut self,
        players: &[usize],
        cells: &[usize],
        right: bool,
        inventory: &mut Inventory,
        ctx: &Context<'_>,
    ) {
        let base = inventory.slots.len();
        let indices: Vec<usize> = players
            .iter()
            .copied()
            .filter(|&slot| slot < STORAGE)
            .chain(cells.iter().map(|&cell| base + cell))
            .collect();
        let before = self.cells[0].clone();
        let kind = self.kind;
        inventory.distribute_limited(&indices, right, &mut self.cells, |index, stack| {
            accepts(kind, index, stack, ctx).then(|| cell_limit(kind, index))
        });
        self.input_changed(&before);
    }

    pub fn click(
        &mut self,
        index: usize,
        right: bool,
        shift: bool,
        inventory: &mut Inventory,
        ctx: &Context<'_>,
    ) {
        if index >= self.cells.len() {
            return;
        }
        let before = self.cells[0].clone();
        if shift {
            if let Some(mut stack) = self.cells[index].take() {
                let order: Vec<usize> = (HOTBAR..STORAGE).chain(0..HOTBAR).collect();
                move_into(&mut inventory.slots, &mut stack, &order);
                self.cells[index] = (stack.count > 0).then_some(stack);
            }
        } else {
            let may_place = inventory
                .cursor
                .as_ref()
                .is_some_and(|carried| self.accepts(index, carried, ctx));
            pick(
                &mut self.cells[index],
                &mut inventory.cursor,
                right,
                may_place,
                cell_limit(self.kind, index),
            );
        }
        self.input_changed(&before);
    }

    pub fn quick_move_from_player(
        &mut self,
        slot: usize,
        inventory: &mut Inventory,
        ctx: &Context<'_>,
    ) {
        if slot >= STORAGE {
            return;
        }
        let Some(mut stack) = inventory.slots[slot].take() else {
            return;
        };
        let before = self.cells[0].clone();
        if self.kind == Workshop::Enchanting && stack.id != "minecraft:lapis_lazuli" {
            if self.cells[0].is_none() && self.accepts(0, &stack, ctx) {
                self.cells[0] = Some(ItemStack {
                    count: 1,
                    ..stack.clone()
                });
                stack.count -= 1;
            }
            inventory.slots[slot] = (stack.count > 0).then_some(stack);
            self.input_changed(&before);
            return;
        }
        let cuts = self.kind != Workshop::Stonecutter
            || !ctx
                .book
                .stonecutter_results(&stack.id, ctx.recipes)
                .is_empty();
        let targets: Vec<usize> = (0..self.cells.len())
            .filter(|&i| cuts && self.accepts(i, &stack, ctx))
            .collect();
        if targets.is_empty() {
            let order: Vec<usize> = if slot >= HOTBAR {
                (0..HOTBAR).collect()
            } else {
                (HOTBAR..STORAGE).collect()
            };
            move_into(&mut inventory.slots, &mut stack, &order);
        } else {
            move_into(&mut self.cells, &mut stack, &targets);
        }
        if stack.count > 0 {
            if inventory.slots[slot].is_none() {
                inventory.slots[slot] = Some(stack);
            } else {
                move_into(&mut inventory.slots, &mut stack, &result_order());
            }
        }
        self.input_changed(&before);
    }

    pub fn choose(
        &mut self,
        index: usize,
        inventory: &mut Inventory,
        ctx: &Context<'_>,
    ) -> Option<Taken> {
        match self.kind {
            Workshop::Stonecutter => {
                let count = self.cells[0].as_ref().map_or(0, |input| {
                    ctx.book.stonecutter_results(&input.id, ctx.recipes).len()
                });
                if index < count {
                    self.chosen = Some(index);
                }
                None
            }
            Workshop::Enchanting => {
                let offers = enchanting_offers(&self.cells[0], &self.cells[1], ctx);
                let offer = offers.get(index)?;
                if !offer.enabled {
                    return None;
                }
                let item = self.cells[0].clone()?;
                let list = enchant_list(&item, index, offer.cost as i32, ctx);
                if list.is_empty() {
                    return None;
                }
                let book = item.id == "minecraft:book";
                let mut result = if book {
                    ItemStack {
                        id: "minecraft:enchanted_book".to_owned(),
                        count: 1,
                        max: 1,
                        components: item.components.clone(),
                    }
                } else {
                    item.clone()
                };
                let mut components = object(result.components.take());
                if let Some(enchantments) = ctx.enchantments {
                    for (e, level) in list {
                        add_enchantment(
                            &result.id,
                            &mut components,
                            &enchantments.list[e].id,
                            level,
                        );
                    }
                }
                result.components = Some(Value::Object(components));
                self.cells[0] = Some(result);
                if let Some(lapis) = self.cells[1].as_mut() {
                    lapis.count = lapis.count.saturating_sub(index as u8 + 1);
                }
                if self.cells[1].as_ref().is_some_and(|l| l.count == 0) {
                    self.cells[1] = None;
                }
                let _ = inventory;
                Some(Taken {
                    levels: index as u32 + 1,
                    sound: "minecraft:block.enchantment_table.use",
                    new_seed: true,
                    ..Taken::default()
                })
            }
            _ => None,
        }
    }

    pub fn take(
        &mut self,
        shift: bool,
        inventory: &mut Inventory,
        ctx: &Context<'_>,
        random: &mut dyn LootRandom,
    ) -> Option<Taken> {
        let mut taken: Option<Taken> = None;
        loop {
            let offer = self.offer(ctx);
            let Some(result) = offer.result.clone() else {
                break;
            };
            if self.kind == Workshop::Anvil
                && (offer.cost == 0 || ctx.level < offer.cost || offer.cost >= TOO_EXPENSIVE)
            {
                break;
            }
            if shift {
                let mut trial = inventory.slots.clone();
                let mut moving = result.clone();
                move_into(&mut trial, &mut moving, &result_order());
                if moving.count > 0 {
                    break;
                }
                inventory.slots = trial;
            } else {
                match inventory.cursor.as_mut() {
                    None => inventory.cursor = Some(result.clone()),
                    Some(carried)
                        if carried.same_item(&result)
                            && carried.count + result.count <= carried.max =>
                    {
                        carried.count += result.count;
                    }
                    _ => break,
                }
            }
            inventory.record_crafted(&result);
            let step = self.consume(&offer, ctx, random);
            let total = taken.get_or_insert_with(Taken::default);
            total.levels += step.levels;
            total.experience += step.experience;
            total.sound = step.sound;
            if !shift || !matches!(self.kind, Workshop::Stonecutter | Workshop::Smithing) {
                break;
            }
        }
        taken
    }

    fn consume(&mut self, offer: &Offer, ctx: &Context<'_>, random: &mut dyn LootRandom) -> Taken {
        let shrink = |cell: &mut Option<ItemStack>, by: u8| {
            if let Some(stack) = cell.as_mut() {
                stack.count = stack.count.saturating_sub(by);
                if stack.count == 0 {
                    *cell = None;
                }
            }
        };
        match self.kind {
            Workshop::Stonecutter => {
                shrink(&mut self.cells[0], 1);
                if self.cells[0].is_none() {
                    self.chosen = None;
                }
                Taken {
                    sound: "minecraft:ui.stonecutter.take_result",
                    ..Taken::default()
                }
            }
            Workshop::Smithing => {
                for cell in &mut self.cells {
                    shrink(cell, 1);
                }
                Taken {
                    sound: "minecraft:block.smithing_table.use",
                    ..Taken::default()
                }
            }
            Workshop::Grindstone => {
                let experience = grind_experience(&self.cells[0], &self.cells[1], ctx, random);
                self.cells[0] = None;
                self.cells[1] = None;
                Taken {
                    experience,
                    sound: "minecraft:block.grindstone.use",
                    ..Taken::default()
                }
            }
            Workshop::Anvil => {
                self.cells[0] = None;
                if offer.repair_items > 0 {
                    shrink(&mut self.cells[1], offer.repair_items);
                } else {
                    self.cells[1] = None;
                }
                self.name = None;
                Taken {
                    levels: offer.cost,
                    sound: "minecraft:block.anvil.use",
                    ..Taken::default()
                }
            }
            Workshop::Enchanting => Taken::default(),
        }
    }

    pub fn rename(&mut self, name: &str) {
        self.name = Some(name.chars().take(NAME_LIMIT).collect());
    }

    pub fn accepts_stack(&self, index: usize, stack: &ItemStack, ctx: &Context<'_>) -> Option<u8> {
        if index >= self.cells.len() || !self.accepts(index, stack, ctx) {
            return None;
        }
        Some(cell_limit(self.kind, index).min(stack.max))
    }

    pub fn throw(&mut self, index: usize, whole: bool) -> Option<ItemStack> {
        let before = self.cells[0].clone();
        let cell = self.cells.get_mut(index)?;
        let stack = cell.as_mut()?;
        let count = if whole { stack.count } else { 1 };
        let thrown = ItemStack {
            count,
            ..stack.clone()
        };
        stack.count -= count;
        if stack.count == 0 {
            *cell = None;
        }
        self.input_changed(&before);
        Some(thrown)
    }

    pub fn gather(&mut self, inventory: &mut Inventory, right: bool) {
        let Some(mut carried) = inventory.cursor.take() else {
            return;
        };
        let before = self.cells[0].clone();
        let mut order: Vec<(bool, usize)> = (0..self.cells.len())
            .map(|index| (true, index))
            .chain((HOTBAR..STORAGE).chain(0..HOTBAR).map(|slot| (false, slot)))
            .collect();
        if right {
            order.reverse();
        }
        for pass in 0..2 {
            for &(cell, index) in &order {
                if carried.count >= carried.max {
                    break;
                }
                let source = if cell {
                    &mut self.cells[index]
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
        self.input_changed(&before);
    }

    pub fn close(&mut self) -> Vec<ItemStack> {
        self.chosen = None;
        self.name = None;
        self.cells.iter_mut().filter_map(Option::take).collect()
    }
}

fn cell_limit(kind: Workshop, index: usize) -> u8 {
    if kind == Workshop::Enchanting && index == 0 {
        1
    } else {
        u8::MAX
    }
}

fn accepts(kind: Workshop, index: usize, stack: &ItemStack, ctx: &Context<'_>) -> bool {
    match (kind, index) {
        (Workshop::Smithing, slot) => ctx.book.smithing_slot(slot, &stack.id, ctx.recipes),
        (Workshop::Grindstone, _) => {
            ctx.recipes.durability(stack).is_some()
                || !enchantments_of(stack).is_empty()
                || gun_mods::is_gun(stack)
        }
        (Workshop::Enchanting, 0) => stack.max == 1 || stack.id == "minecraft:book",
        (Workshop::Enchanting, _) => stack.id == "minecraft:lapis_lazuli",
        _ => true,
    }
}

fn is_curse(id: &str, ctx: &Context<'_>) -> bool {
    ctx.enchantments
        .is_some_and(|e| e.tag_contains("minecraft:curse", id))
}

fn without_non_curses(mut stack: ItemStack, ctx: &Context<'_>) -> ItemStack {
    let kept: Vec<(String, i32)> = enchantments_of(&stack)
        .into_iter()
        .filter(|(id, _)| is_curse(id, ctx))
        .collect();
    set_enchantments(&mut stack, &kept);
    if stack.id == "minecraft:enchanted_book" && kept.is_empty() {
        stack.id = "minecraft:book".to_owned();
    }
    let mut cost = 0i64;
    for _ in &kept {
        cost = cost * 2 + 1;
    }
    set_int(
        &mut stack,
        "minecraft:repair_cost",
        (cost > 0).then_some(cost),
    );
    stack
}

fn stripped_gun(
    a: &Option<ItemStack>,
    b: &Option<ItemStack>,
    ctx: &Context<'_>,
) -> Option<Option<(ItemStack, usize)>> {
    let gun = [a, b].into_iter().flatten().find(|stack| gun_mods::is_gun(stack))?;
    let alone = a.is_none() || b.is_none();
    Some(
        ctx.guns
            .filter(|_| alone)
            .and_then(|rules| gun_mods::strip(gun, rules)),
    )
}

fn grind(a: &Option<ItemStack>, b: &Option<ItemStack>, ctx: &Context<'_>) -> Option<ItemStack> {
    if let Some(stripped) = stripped_gun(a, b, ctx) {
        return stripped.map(|(bare, _)| bare);
    }
    match (a, b) {
        (None, None) => None,
        (Some(one), None) | (None, Some(one)) => {
            if one.count > 1 || enchantments_of(one).is_empty() {
                None
            } else {
                Some(without_non_curses(one.clone(), ctx))
            }
        }
        (Some(a), Some(b)) => {
            if a.count > 1 || b.count > 1 || a.id != b.id {
                return None;
            }
            let mut result = ItemStack {
                count: 1,
                ..a.clone()
            };
            match (ctx.recipes.durability(a), ctx.recipes.durability(b)) {
                (Some((da, ma)), Some((db, mb))) => {
                    let max = ma.max(mb);
                    let total = (ma - da) + (mb - db) + max * 5 / 100;
                    set_int(
                        &mut result,
                        "minecraft:damage",
                        Some(i64::from(max.saturating_sub(total))),
                    );
                }
                _ => {
                    if a.max < 2 || a.components != b.components {
                        return None;
                    }
                    result.count = 2;
                }
            }
            let mut merged = enchantments_of(&result);
            for (id, level) in enchantments_of(b) {
                if is_curse(&id, ctx) && !merged.iter().any(|(m, _)| *m == id) {
                    merged.push((id, level));
                }
            }
            set_enchantments(&mut result, &merged);
            Some(without_non_curses(result, ctx))
        }
    }
}

fn grind_experience(
    a: &Option<ItemStack>,
    b: &Option<ItemStack>,
    ctx: &Context<'_>,
    random: &mut dyn LootRandom,
) -> u32 {
    let removed = stripped_gun(a, b, ctx).map(|stripped| stripped.map_or(0, |(_, removed)| removed));
    let Some(enchantments) = ctx.enchantments.filter(|_| removed.is_none()) else {
        return attachment_experience(removed.unwrap_or(0), random);
    };
    let mut total = 0i32;
    for stack in [a, b].into_iter().flatten() {
        for (id, level) in enchantments_of(stack) {
            if is_curse(&id, ctx) {
                continue;
            }
            if let Some(index) = enchantments.index(&id) {
                total += enchantments.list[index].min_cost(level);
            }
        }
    }
    if total <= 0 {
        return 0;
    }
    let half = (total + 1) / 2;
    (half + random.next_int(half as u32) as i32) as u32
}

const ATTACHMENT_COST: i32 = 5;

fn attachment_experience(removed: usize, random: &mut dyn LootRandom) -> u32 {
    let total = i32::try_from(removed).unwrap_or(0) * ATTACHMENT_COST;
    if total <= 0 {
        return 0;
    }
    let half = (total + 1) / 2;
    (half + random.next_int(half as u32) as i32) as u32
}

fn anvil(
    left: &Option<ItemStack>,
    right: &Option<ItemStack>,
    name: Option<&str>,
    ctx: &Context<'_>,
) -> Offer {
    let empty = Offer::default();
    let Some(left) = left else {
        return empty;
    };
    let mut result = left.clone();
    let mut enchants = enchantments_of(&result);
    let mut cost = 0i64;
    let base = int_component(left, "minecraft:repair_cost")
        + right
            .as_ref()
            .map_or(0, |r| int_component(r, "minecraft:repair_cost"));
    let mut repair_items = 0u8;
    if let Some(right) = right {
        let book = right.id == "minecraft:enchanted_book";
        let durability = ctx.recipes.durability(&result);
        if durability.is_some() && repairs(&left.id, &right.id, ctx.recipes) {
            let (mut damage, max) = durability.unwrap_or((0, 1));
            let mut step = damage.min(max / 4);
            if step == 0 {
                return empty;
            }
            while step > 0 && repair_items < right.count {
                damage -= step;
                cost += 1;
                repair_items += 1;
                step = damage.min(max / 4);
            }
            set_int(&mut result, "minecraft:damage", Some(i64::from(damage)));
        } else {
            if !book && (right.id != result.id || durability.is_none()) {
                return empty;
            }
            if let (Some((damage, max)), false) = (durability, book) {
                let left_left = max - damage;
                let right_left = ctx.recipes.durability(right).map_or(0, |(d, m)| m - d);
                let combined = left_left + right_left + max * 12 / 100;
                let new_damage = max.saturating_sub(combined);
                if new_damage < damage {
                    set_int(&mut result, "minecraft:damage", Some(i64::from(new_damage)));
                    cost += 2;
                }
            }
            let mut applied = false;
            let mut refused = false;
            if let Some(enchantments) = ctx.enchantments {
                for (id, level) in enchantments_of(right) {
                    let Some(index) = enchantments.index(&id) else {
                        continue;
                    };
                    let current = enchants
                        .iter()
                        .find(|(e, _)| *e == id)
                        .map_or(0, |(_, l)| *l);
                    let mut next = if current == level {
                        level + 1
                    } else {
                        level.max(current)
                    };
                    let mut fits = enchantments.list[index].can_enchant(&left.id)
                        || left.id == "minecraft:enchanted_book";
                    for (other, _) in &enchants {
                        if let Some(o) = enchantments.index(other) {
                            if o != index && !enchantments.compatible(index, o) {
                                fits = false;
                                cost += 1;
                            }
                        }
                    }
                    if !fits {
                        refused = true;
                        continue;
                    }
                    applied = true;
                    next = next.min(enchantments.list[index].max_level);
                    match enchants.iter_mut().find(|(e, _)| *e == id) {
                        Some(entry) => entry.1 = next,
                        None => enchants.push((id.clone(), next)),
                    }
                    let mut anvil_cost = i64::from(enchantments.list[index].anvil_cost);
                    if book {
                        anvil_cost = (anvil_cost / 2).max(1);
                    }
                    cost += anvil_cost * i64::from(next);
                    if left.count > 1 {
                        cost = 40;
                    }
                }
            }
            if refused && !applied {
                return empty;
            }
        }
    }
    let mut rename = 0i64;
    match name {
        Some(text) if text.trim().is_empty() => {
            if custom_name(left).is_some() {
                rename = 1;
                cost += 1;
                let mut components = object(result.components.take());
                components.remove("minecraft:custom_name");
                result.components = (!components.is_empty()).then_some(Value::Object(components));
            }
        }
        Some(text) if custom_name(left).as_deref() != Some(text) && !text.is_empty() => {
            rename = 1;
            cost += 1;
            let mut components = object(result.components.take());
            components.insert("minecraft:custom_name".to_owned(), Value::from(text));
            result.components = Some(Value::Object(components));
        }
        _ => {}
    }
    if cost <= 0 {
        return empty;
    }
    let mut total = (base + cost).clamp(0, i64::from(u32::MAX)) as u32;
    if rename == cost && rename > 0 && total >= TOO_EXPENSIVE {
        total = TOO_EXPENSIVE - 1;
    }
    let mut prior = int_component(&result, "minecraft:repair_cost");
    if let Some(right) = right {
        prior = prior.max(int_component(right, "minecraft:repair_cost"));
    }
    if rename != cost || rename == 0 {
        prior = prior * 2 + 1;
    }
    set_int(
        &mut result,
        "minecraft:repair_cost",
        (prior > 0).then_some(prior),
    );
    set_enchantments(&mut result, &enchants);
    Offer {
        result: (total < TOO_EXPENSIVE).then_some(result),
        cost: total,
        choices: Vec::new(),
        repair_items,
    }
}

fn enchant_cost(random: &mut LegacyRandom, slot: i32, shelves: i32, enchantable: i32) -> i32 {
    if enchantable <= 0 {
        return 0;
    }
    let shelves = shelves.min(15);
    let j =
        random.next_int(8) as i32 + 1 + (shelves >> 1) + random.next_int(shelves as u32 + 1) as i32;
    match slot {
        0 => (j / 3).max(1),
        1 => j * 2 / 3 + 1,
        _ => j.max(shelves * 2),
    }
}

fn enchant_list(item: &ItemStack, slot: usize, cost: i32, ctx: &Context<'_>) -> Vec<(usize, i32)> {
    let Some(enchantments) = ctx.enchantments else {
        return Vec::new();
    };
    let mut random = LegacyRandom::new((i64::from(ctx.seed) + slot as i64) as u64);
    let source = enchantments.source(Some(&Value::from("#minecraft:in_enchanting_table")));
    let mut list = enchantments.select(&mut random, &item.id, cost, &source);
    if item.id == "minecraft:book" && list.len() > 1 {
        let drop = random.next_int(list.len() as u32) as usize;
        list.remove(drop);
    }
    list
}

fn enchanting_offers(
    item: &Option<ItemStack>,
    lapis: &Option<ItemStack>,
    ctx: &Context<'_>,
) -> Vec<Choice> {
    let Some(enchantments) = ctx.enchantments else {
        return Vec::new();
    };
    let Some(item) = item.as_ref().filter(|i| enchantments_of(i).is_empty()) else {
        return Vec::new();
    };
    let enchantable = enchantments.enchantable(&item.id);
    if enchantable <= 0 {
        return Vec::new();
    }
    let mut random = LegacyRandom::new(i64::from(ctx.seed) as u64);
    let costs: Vec<i32> = (0..3)
        .map(|slot| {
            let cost = enchant_cost(&mut random, slot, ctx.bookshelves, enchantable);
            if cost < slot + 1 { 0 } else { cost }
        })
        .collect();
    let lapis = lapis.as_ref().map_or(0, |l| u32::from(l.count));
    costs
        .into_iter()
        .enumerate()
        .map(|(slot, cost)| {
            let list = if cost > 0 {
                enchant_list(item, slot, cost, ctx)
            } else {
                Vec::new()
            };
            let clue = if list.is_empty() {
                None
            } else {
                let mut pick = LegacyRandom::new((i64::from(ctx.seed) + slot as i64) as u64);
                let (e, level) = list[pick.next_int(list.len() as u32) as usize];
                Some((enchantments.list[e].id.clone(), level))
            };
            let cost = cost.max(0) as u32;
            Choice {
                item: None,
                cost,
                enabled: cost > 0 && !list.is_empty() && ctx.level >= cost && lapis > slot as u32,
                clue,
            }
        })
        .collect()
}

pub fn bookshelves(
    table: BlockPos,
    provides: impl Fn(BlockPos) -> bool,
    transmits: impl Fn(BlockPos) -> bool,
) -> i32 {
    let mut count = 0;
    for dy in 0..=1 {
        for dx in -2..=2i32 {
            for dz in -2..=2i32 {
                if dx.abs() != 2 && dz.abs() != 2 {
                    continue;
                }
                let at = (table.0 + dx, table.1 + dy, table.2 + dz);
                let between = (table.0 + dx / 2, table.1 + dy, table.2 + dz / 2);
                if provides(at) && transmits(between) {
                    count += 1;
                }
            }
        }
    }
    count
}

pub fn roman(level: i32) -> &'static str {
    match level {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        7 => "VII",
        8 => "VIII",
        9 => "IX",
        10 => "X",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use serde_json::json;

    use super::*;

    fn jar(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("client.jar");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let entries = [
            (
                "data/minecraft/recipe/stone_bricks_from_stone_stonecutting.json",
                json!({"type": "minecraft:stonecutting", "ingredient": "minecraft:stone", "result": {"id": "minecraft:stone_bricks"}}),
            ),
            (
                "data/minecraft/recipe/stone_slab_from_stone_stonecutting.json",
                json!({"type": "minecraft:stonecutting", "ingredient": "minecraft:stone", "result": {"id": "minecraft:stone_slab", "count": 2}}),
            ),
            (
                "data/minecraft/recipe/netherite_sword_smithing.json",
                json!({"type": "minecraft:smithing_transform", "template": "minecraft:netherite_upgrade_smithing_template", "base": "minecraft:diamond_sword", "addition": "#minecraft:netherite_tool_materials", "result": {"id": "minecraft:netherite_sword"}}),
            ),
            (
                "data/minecraft/recipe/brewing/potion_water_nether_wart.json",
                json!({"type": "minecraft:brewing", "input": {"item": "minecraft:potion", "potion_contents": {"potions": "minecraft:water"}}, "reagent": {"item": "minecraft:nether_wart"}, "output": {"id": "minecraft:potion", "components": {"minecraft:potion_contents": {"potion": "minecraft:awkward"}}}}),
            ),
            (
                "data/minecraft/recipe/cooked_beef_from_campfire_cooking.json",
                json!({"type": "minecraft:campfire_cooking", "ingredient": "minecraft:beef", "result": {"id": "minecraft:cooked_beef"}, "cookingtime": 600, "experience": 0.35}),
            ),
            (
                "data/minecraft/recipe/cooked_chicken_from_campfire_cooking.json",
                json!({"type": "minecraft:campfire_cooking", "ingredient": "minecraft:chicken", "result": {"id": "minecraft:cooked_chicken"}}),
            ),
            (
                "data/minecraft/tags/item/netherite_tool_materials.json",
                json!({"values": ["minecraft:netherite_ingot"]}),
            ),
            (
                "data/minecraft/tags/item/iron_tool_materials.json",
                json!({"values": ["minecraft:iron_ingot"]}),
            ),
        ];
        for (name, value) in entries {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(value.to_string().as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    fn setup() -> (StationBook, RecipeBook) {
        let dir = crate::test_dir::tempdir().unwrap();
        let path = jar(dir.path());
        (
            StationBook::from_jar(&path).unwrap(),
            RecipeBook::from_jar(&path).unwrap(),
        )
    }

    fn ctx<'a>(book: &'a StationBook, recipes: &'a RecipeBook, level: u32) -> Context<'a> {
        Context {
            recipes,
            book,
            enchantments: None,
            bookshelves: 0,
            seed: 0,
            level,
            guns: None,
        }
    }

    fn damaged(id: &str, damage: i64, max: i64) -> ItemStack {
        let mut stack = ItemStack::new(id, 1);
        stack.max = 1;
        stack.components = Some(json!({"minecraft:damage": damage, "minecraft:max_damage": max}));
        stack
    }

    #[test]
    fn campfire_recipes_give_the_cooked_item_and_its_time() {
        let (book, recipes) = setup();
        let (cooked, ticks) = book.campfire_cooking("minecraft:beef", &recipes).unwrap();
        assert_eq!((cooked.id.as_str(), cooked.count, ticks), ("minecraft:cooked_beef", 1, 600));
        assert!(book.campfire_cooking("minecraft:chicken", &recipes).is_none(), "cookingtime is a required field");
        assert!(book.campfire_cooking("minecraft:stone", &recipes).is_none());
        assert!(book.campfire_cooking("minecraft:cooked_beef", &recipes).is_none());
    }

    #[test]
    fn vanilla_campfires_cook_nine_foods_in_thirty_seconds() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = StationBook::from_jar(&jar).unwrap();
        let recipes = RecipeBook::from_jar(&jar).unwrap();
        let foods = [
            ("beef", "cooked_beef"),
            ("chicken", "cooked_chicken"),
            ("cod", "cooked_cod"),
            ("mutton", "cooked_mutton"),
            ("porkchop", "cooked_porkchop"),
            ("rabbit", "cooked_rabbit"),
            ("salmon", "cooked_salmon"),
            ("potato", "baked_potato"),
            ("kelp", "dried_kelp"),
        ];
        for (raw, done) in foods {
            let (cooked, ticks) = book.campfire_cooking(&format!("minecraft:{raw}"), &recipes).unwrap();
            assert_eq!((cooked.id, ticks), (format!("minecraft:{done}"), 600));
        }
        assert_eq!(book.campfire.len(), foods.len());
        assert!(book.campfire_cooking("minecraft:raw_iron", &recipes).is_none());
    }

    #[test]
    fn workshops_come_from_their_blocks() {
        assert_eq!(
            Workshop::of(&Block::new("minecraft:chipped_anvil")),
            Some(Workshop::Anvil)
        );
        assert_eq!(
            Workshop::of(&Block::new("minecraft:stonecutter")),
            Some(Workshop::Stonecutter)
        );
        assert_eq!(Workshop::of(&Block::new("minecraft:stone")), None);
        assert_eq!(Workshop::Smithing.inputs(), 3);
    }

    #[test]
    fn a_stonecutter_offers_its_cuts_and_cuts_until_the_stone_runs_out() {
        let (book, recipes) = setup();
        let ctx = ctx(&book, &recipes, 0);
        let mut inventory = Inventory::default();
        let mut bench = Bench::new(Workshop::Stonecutter);
        bench.cells[0] = Some(recipes.stack("minecraft:stone", 3));
        let offer = bench.offer(&ctx);
        let ids: Vec<String> = offer
            .choices
            .iter()
            .filter_map(|c| c.item.as_ref().map(|i| i.id.clone()))
            .collect();
        assert_eq!(ids, vec!["minecraft:stone_bricks", "minecraft:stone_slab"]);
        assert!(offer.result.is_none());
        bench.choose(1, &mut inventory, &ctx);
        assert_eq!(bench.offer(&ctx).result.map(|r| r.count), Some(2));
        let mut random = LegacyRandom::new(1);
        let taken = bench.take(true, &mut inventory, &ctx, &mut random).unwrap();
        assert_eq!(taken.sound, "minecraft:ui.stonecutter.take_result");
        assert_eq!(inventory.slots[8].as_ref().map(|s| s.count), Some(6));
        assert!(bench.cells[0].is_none());
        assert_eq!(bench.chosen, None);
    }

    #[test]
    fn a_smithing_table_upgrades_to_netherite_keeping_components() {
        let (book, recipes) = setup();
        let ctx = ctx(&book, &recipes, 0);
        let mut bench = Bench::new(Workshop::Smithing);
        bench.cells[0] = Some(recipes.stack("minecraft:netherite_upgrade_smithing_template", 1));
        bench.cells[1] = Some(damaged("minecraft:diamond_sword", 10, 1561));
        assert!(bench.offer(&ctx).result.is_none());
        bench.cells[2] = Some(recipes.stack("minecraft:netherite_ingot", 2));
        let result = bench.offer(&ctx).result.unwrap();
        assert_eq!(result.id, "minecraft:netherite_sword");
        assert_eq!(int_component(&result, "minecraft:damage"), 10);
        let mut inventory = Inventory::default();
        let mut random = LegacyRandom::new(1);
        bench.take(false, &mut inventory, &ctx, &mut random);
        assert!(bench.cells[0].is_none() && bench.cells[1].is_none());
        assert_eq!(bench.cells[2].as_ref().map(|s| s.count), Some(1));
    }

    #[test]
    fn a_grindstone_combines_two_worn_tools_with_a_bonus() {
        let (book, recipes) = setup();
        let ctx = ctx(&book, &recipes, 0);
        let mut bench = Bench::new(Workshop::Grindstone);
        bench.cells[0] = Some(damaged("minecraft:iron_pickaxe", 200, 250));
        bench.cells[1] = Some(damaged("minecraft:iron_pickaxe", 150, 250));
        let result = bench.offer(&ctx).result.unwrap();
        assert_eq!(
            int_component(&result, "minecraft:damage"),
            250 - (50 + 100 + 12)
        );
        bench.cells[1] = Some(damaged("minecraft:iron_sword", 0, 250));
        assert!(bench.offer(&ctx).result.is_none());
        bench.cells[1] = None;
        assert!(
            bench.offer(&ctx).result.is_none(),
            "one plain tool gives nothing"
        );
    }

    #[test]
    fn an_anvil_repairs_with_materials_and_renames_for_levels() {
        let (book, recipes) = setup();
        let mut bench = Bench::new(Workshop::Anvil);
        bench.cells[0] = Some(damaged("minecraft:iron_pickaxe", 200, 250));
        bench.cells[1] = Some(recipes.stack("minecraft:iron_ingot", 5));
        let offer = bench.offer(&ctx(&book, &recipes, 0));
        let result = offer.result.clone().unwrap();
        assert_eq!(
            int_component(&result, "minecraft:damage"),
            200 - 62 * 3 - 14
        );
        assert_eq!((offer.cost, offer.repair_items), (4, 4));
        assert_eq!(int_component(&result, "minecraft:repair_cost"), 1);
        let mut inventory = Inventory::default();
        let mut random = LegacyRandom::new(1);
        assert!(
            bench
                .take(false, &mut inventory, &ctx(&book, &recipes, 3), &mut random)
                .is_none(),
            "three levels do not pay for four"
        );
        let taken = bench
            .take(false, &mut inventory, &ctx(&book, &recipes, 4), &mut random)
            .unwrap();
        assert_eq!(taken.levels, 4);
        assert_eq!(bench.cells[1].as_ref().map(|s| s.count), Some(1));
        let mut named = Bench::new(Workshop::Anvil);
        named.cells[0] = Some(recipes.stack("minecraft:stone", 1));
        named.rename("Rock");
        let offer = named.offer(&ctx(&book, &recipes, 0));
        assert_eq!(offer.cost, 1);
        assert_eq!(
            offer.result.as_ref().and_then(custom_name).as_deref(),
            Some("Rock")
        );
        named.rename("");
        assert!(named.offer(&ctx(&book, &recipes, 0)).result.is_none());
    }

    #[test]
    fn brewing_follows_the_jar_recipes() {
        let (book, recipes) = setup();
        let water = crate::bottle::water_bottle();
        let awkward = book
            .brew(&water, "minecraft:nether_wart", &recipes)
            .unwrap();
        assert_eq!(potion_of(&awkward), Some("minecraft:awkward"));
        assert!(
            book.brew(&awkward, "minecraft:nether_wart", &recipes)
                .is_none()
        );
        assert!(book.is_reagent("minecraft:nether_wart", &recipes));
        assert!(!book.is_reagent("minecraft:dirt", &recipes));
    }

    #[test]
    fn bench_cells_throw_swap_with_the_offhand_and_gather_into_the_cursor() {
        let (book, recipes) = setup();
        let ctx = ctx(&book, &recipes, 0);
        let mut inventory = Inventory::default();
        let mut bench = Bench::new(Workshop::Stonecutter);
        bench.cells[0] = Some(recipes.stack("minecraft:stone", 5));
        assert_eq!(bench.throw(0, false).map(|s| s.count), Some(1));
        assert_eq!(bench.cells[0].as_ref().map(|s| s.count), Some(4));
        inventory.slots[OFFHAND] = Some(recipes.stack("minecraft:stone", 2));
        bench.swap(0, OFFHAND, &mut inventory, &ctx);
        assert_eq!(bench.cells[0].as_ref().map(|s| s.count), Some(2));
        assert_eq!(inventory.slots[OFFHAND].as_ref().map(|s| s.count), Some(4));
        inventory.slots[12] = Some(recipes.stack("minecraft:stone", 7));
        inventory.cursor = Some(recipes.stack("minecraft:stone", 1));
        bench.gather(&mut inventory, false);
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(10));
        assert!(bench.cells[0].is_none() && inventory.slots[12].is_none());
        assert_eq!(inventory.slots[OFFHAND].as_ref().map(|s| s.count), Some(4));
        assert_eq!(bench.throw(0, true), None);
        let mut enchanting = Bench::new(Workshop::Enchanting);
        let sword = damaged("minecraft:diamond_sword", 0, 1561);
        assert_eq!(enchanting.accepts_stack(0, &sword, &ctx), Some(1));
        assert_eq!(enchanting.accepts_stack(1, &sword, &ctx), None);
        enchanting.cells[1] = Some(recipes.stack("minecraft:lapis_lazuli", 3));
        assert_eq!(enchanting.throw(1, true).map(|s| s.count), Some(3));
    }

    #[test]
    fn the_enchanting_slot_takes_one_item_by_click_drag_and_shift_click() {
        let (book, recipes) = setup();
        let ctx = ctx(&book, &recipes, 0);
        let mut inventory = Inventory::default();
        let mut bench = Bench::new(Workshop::Enchanting);
        inventory.cursor = Some(recipes.stack("minecraft:book", 4));
        bench.click(0, false, false, &mut inventory, &ctx);
        assert_eq!(bench.cells[0].as_ref().map(|s| s.count), Some(1));
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(3));
        bench.cells[0] = None;
        bench.spread(&[9], &[0], false, &mut inventory, &ctx);
        assert_eq!(bench.cells[0].as_ref().map(|s| s.count), Some(1));
        assert_eq!(inventory.slots[9].as_ref().map(|s| s.count), Some(1));
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(1));
        bench.cells[0] = None;
        inventory.cursor = None;
        inventory.slots[12] = Some(recipes.stack("minecraft:book", 5));
        bench.quick_move_from_player(12, &mut inventory, &ctx);
        assert_eq!(bench.cells[0].as_ref().map(|s| s.count), Some(1));
        assert_eq!(inventory.slots[12].as_ref().map(|s| s.count), Some(4));
        bench.quick_move_from_player(12, &mut inventory, &ctx);
        assert_eq!(inventory.slots[12].as_ref().map(|s| s.count), Some(4));
        assert!(inventory.slots[..9].iter().all(Option::is_none));
    }

    #[test]
    fn bookshelves_count_on_the_ring_with_air_between() {
        let shelf = |(x, _, z): BlockPos| x.abs() == 2 || z.abs() == 2;
        assert_eq!(bookshelves((0, 0, 0), shelf, |_| true), 32);
        assert_eq!(bookshelves((0, 0, 0), shelf, |(x, _, _)| x != 1), 22);
        assert_eq!(roman(3), "III");
    }
}
