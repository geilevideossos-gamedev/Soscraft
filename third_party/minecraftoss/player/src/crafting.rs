//! Shaped and shapeless crafting against recipes and item tags loaded from the
//! player's external, pinned Minecraft data JAR. No vanilla data is bundled.
use crate::{inventory::ItemStack, item_catalog::ItemCatalog};
use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
    sync::Arc,
};
use zip::ZipArchive;

#[derive(Clone, Debug)]
struct Ingredient(Vec<String>);
impl Ingredient {
    fn parse(value: &Value) -> Option<Self> {
        match value {
            Value::String(id) => Some(Self(vec![id.clone()])),
            Value::Array(options) => {
                let ids = options
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                (!ids.is_empty()).then_some(Self(ids))
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
enum Kind {
    Shaped(Vec<Vec<Option<Ingredient>>>),
    Shapeless(Vec<Ingredient>),
    FireworkStar {
        shapes: Vec<(String, Ingredient)>,
        trail: Ingredient,
        twinkle: Ingredient,
        fuel: Ingredient,
        dye: Ingredient,
    },
    FireworkStarFade {
        target: Ingredient,
        dye: Ingredient,
    },
    FireworkRocket {
        shell: Ingredient,
        fuel: Ingredient,
        star: Ingredient,
    },
    ShieldDecoration {
        banner: Ingredient,
        target: Ingredient,
    },
}

const DYE_COLORS: [(&str, i32); 16] = [
    ("white", 15790320),
    ("orange", 15435844),
    ("magenta", 12801229),
    ("light_blue", 6719955),
    ("yellow", 14602026),
    ("lime", 4312372),
    ("pink", 14188952),
    ("gray", 4408131),
    ("light_gray", 11250603),
    ("cyan", 2651799),
    ("purple", 8073150),
    ("blue", 2437522),
    ("brown", 5320730),
    ("green", 3887386),
    ("red", 11743532),
    ("black", 1973019),
];
const FIREWORK_SHAPES: [&str; 5] = ["small_ball", "large_ball", "star", "creeper", "burst"];
const DYE: &str = "minecraft:dye";
const FIREWORK_EXPLOSION: &str = "minecraft:firework_explosion";
const FIREWORKS: &str = "minecraft:fireworks";
const DEFAULT_FLIGHT: u64 = 1;
const MAX_FUEL: u64 = 3;
const BANNER_PATTERNS: &str = "minecraft:banner_patterns";
const BASE_COLOR: &str = "minecraft:base_color";

#[derive(Debug)]
struct Recipe {
    id: String,
    category: String,
    group: Option<String>,
    kind: Kind,
    result: ItemStack,
}

#[derive(Clone, Copy, Debug)]
pub struct CraftingRecipeRef<'a> {
    pub id: &'a str,
    pub category: &'a str,
    pub group: Option<&'a str>,
    pub result: &'a ItemStack,
    pub width: usize,
    pub height: usize,
    pub ingredients: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SmeltingRecipe {
    pub result: ItemStack,
    pub cooking_ticks: u32,
    pub experience: f32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CookingKind {
    #[default]
    Furnace,
    BlastFurnace,
    Smoker,
}
impl CookingKind {
    pub fn block_id(self) -> &'static str {
        match self {
            Self::Furnace => "furnace",
            Self::BlastFurnace => "blast_furnace",
            Self::Smoker => "smoker",
        }
    }
    pub fn from_block_id(id: &str) -> Option<Self> {
        match id {
            "furnace" => Some(Self::Furnace),
            "blast_furnace" => Some(Self::BlastFurnace),
            "smoker" => Some(Self::Smoker),
            _ => None,
        }
    }
    /// The default CookingFuel speed provider is conditional on the furnace
    /// block: the smoker and blast furnace match the FAST_FURNACE predicate.
    pub fn default_speed_multiplier(self) -> f32 {
        match self {
            Self::Furnace => 1.0,
            Self::BlastFurnace | Self::Smoker => 2.0,
        }
    }
    pub fn default_burn_divisor(self) -> u32 {
        match self {
            Self::Furnace => 1,
            Self::BlastFurnace | Self::Smoker => 2,
        }
    }
}

#[derive(Debug)]
struct SmeltingEntry {
    id: String,
    kind: CookingKind,
    category: String,
    group: Option<String>,
    ingredient: Ingredient,
    recipe: SmeltingRecipe,
}

#[derive(Clone, Debug)]
enum AutoUnlockCriterion {
    ChangedItem(Ingredient),
    OccupiedSlots(usize),
}

#[derive(Debug)]
struct AutoUnlockRule {
    recipe_id: String,
    criterion: AutoUnlockCriterion,
}

#[derive(Clone, Copy, Debug)]
pub struct CookingRecipeRef<'a> {
    pub id: &'a str,
    pub category: &'a str,
    pub group: Option<&'a str>,
    pub result: &'a ItemStack,
}

#[derive(Default, Debug)]
pub struct RecipeBook {
    recipes: Vec<Recipe>,
    smelting: Vec<SmeltingEntry>,
    tags: HashMap<String, Vec<String>>,
    auto_unlocks: Vec<AutoUnlockRule>,
    display_indices: HashMap<String, Vec<u32>>,
    item_catalog: Option<Arc<ItemCatalog>>,
    jar_ids: HashSet<String>,
    reserved_slots: Option<fn(&str) -> Option<usize>>,
}
impl RecipeBook {
    pub fn from_jar(path: &Path) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("open data JAR {}", path.display()))?;
        let mut archive = ZipArchive::new(file)?;
        let mut tags = HashMap::new();
        let mut raw_recipes = Vec::new();
        let mut raw_advancements = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name().to_owned();
            if !name.ends_with(".json") {
                continue;
            }
            let is_recipe = name.starts_with("data/minecraft/recipe/");
            let is_tag = name.starts_with("data/minecraft/tags/item/");
            let is_recipe_advancement = name.starts_with("data/minecraft/advancement/recipes/");
            if !is_recipe && !is_tag && !is_recipe_advancement {
                continue;
            }
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            let value: Value =
                serde_json::from_str(&content).with_context(|| format!("parse {name}"))?;
            if is_recipe {
                raw_recipes.push((name, value));
            } else if is_recipe_advancement {
                raw_advancements.push(value);
            } else if let Some(values) = value.get("values").and_then(Value::as_array) {
                let id = name
                    .trim_start_matches("data/minecraft/tags/item/")
                    .trim_end_matches(".json");
                let entries = values
                    .iter()
                    .filter_map(|v| v.as_str().or_else(|| v.get("id").and_then(Value::as_str)))
                    .map(str::to_owned)
                    .collect();
                tags.insert(format!("minecraft:{id}"), entries);
            }
        }
        raw_recipes.sort_by(|a, b| a.0.cmp(&b.0));
        let jar_ids = raw_recipes
            .iter()
            .map(|(name, _)| {
                format!(
                    "minecraft:{}",
                    name.trim_start_matches("data/minecraft/recipe/")
                        .trim_end_matches(".json")
                )
            })
            .collect();
        let mut recipes = Vec::new();
        let mut smelting = Vec::new();
        for (name, value) in raw_recipes {
            let cooking_kind = match value.get("type").and_then(Value::as_str) {
                Some("minecraft:smelting") => Some(CookingKind::Furnace),
                Some("minecraft:blasting") => Some(CookingKind::BlastFurnace),
                Some("minecraft:smoking") => Some(CookingKind::Smoker),
                _ => None,
            };
            if let Some(kind) = cooking_kind {
                if let Some(ingredient) = value.get("ingredient").and_then(Ingredient::parse) {
                    if let Some(result) = parse_result(&value) {
                        smelting.push(SmeltingEntry {
                            id: format!(
                                "minecraft:{}",
                                name.trim_start_matches("data/minecraft/recipe/")
                                    .trim_end_matches(".json")
                            ),
                            kind,
                            category: value
                                .get("category")
                                .and_then(Value::as_str)
                                .unwrap_or("misc")
                                .to_owned(),
                            group: value
                                .get("group")
                                .and_then(Value::as_str)
                                .filter(|group| !group.is_empty())
                                .map(str::to_owned),
                            ingredient,
                            recipe: SmeltingRecipe {
                                result,
                                cooking_ticks: value
                                    .get("cookingtime")
                                    .and_then(Value::as_u64)
                                    .unwrap_or(200)
                                    as u32,
                                experience: value
                                    .get("experience")
                                    .and_then(Value::as_f64)
                                    .unwrap_or(0.0)
                                    as f32,
                            },
                        });
                    }
                }
            } else if let Some(mut recipe) = parse_recipe(&value) {
                recipe.id = format!(
                    "minecraft:{}",
                    name.trim_start_matches("data/minecraft/recipe/")
                        .trim_end_matches(".json")
                );
                recipe.category = value
                    .get("category")
                    .and_then(Value::as_str)
                    .unwrap_or("misc")
                    .to_owned();
                recipe.group = value
                    .get("group")
                    .and_then(Value::as_str)
                    .filter(|group| !group.is_empty())
                    .map(str::to_owned);
                recipes.push(recipe);
            }
        }
        let known_ids = recipes
            .iter()
            .map(|recipe| recipe.id.as_str())
            .chain(smelting.iter().map(|entry| entry.id.as_str()))
            .collect::<HashSet<_>>();
        let auto_unlocks = raw_advancements
            .iter()
            .flat_map(|advancement| auto_unlock_rules(advancement, &known_ids))
            .collect();
        Ok(Self {
            recipes,
            smelting,
            tags,
            auto_unlocks,
            display_indices: HashMap::new(),
            item_catalog: None,
            jar_ids,
            reserved_slots: None,
        })
    }

    pub fn has_recipe(&self, id: &str) -> bool {
        self.jar_ids.contains(id)
            || self.recipes.iter().any(|recipe| recipe.id == id)
            || self.smelting.iter().any(|entry| entry.id == id)
    }

    /// Load display IDs observed from the pinned server recipe manager. The
    /// client HashMap iterates known displays by these integer-key buckets.
    pub fn with_display_catalog(mut self, path: &Path) -> Result<Self> {
        let value: Value = serde_json::from_reader(
            File::open(path).with_context(|| format!("open display catalog {}", path.display()))?,
        )?;
        anyhow::ensure!(
            value.get("schema_version").and_then(Value::as_u64) == Some(1)
                && value.get("minecraft_version").and_then(Value::as_str) == Some("26.3"),
            "unsupported recipe display catalog {}",
            path.display()
        );
        let recipes = value
            .get("recipes")
            .and_then(Value::as_object)
            .context("recipe display catalog missing recipes")?;
        let mut seen = HashSet::new();
        for (id, displays) in recipes {
            let indices = displays
                .as_array()
                .context("recipe display list must be an array")?
                .iter()
                .map(|display| {
                    display
                        .get("index")
                        .and_then(Value::as_u64)
                        .and_then(|index| u32::try_from(index).ok())
                        .context("invalid recipe display index")
                })
                .collect::<Result<Vec<_>>>()?;
            for index in &indices {
                anyhow::ensure!(
                    seen.insert(*index),
                    "duplicate recipe display index {index}"
                );
            }
            self.display_indices.insert(id.clone(), indices);
        }
        Ok(self)
    }

    pub fn with_extra(mut self, recipes: impl IntoIterator<Item = (String, Value)>) -> Self {
        let vanilla = u32::try_from(self.crafting_recipes().count() + self.smelting.len()).unwrap_or(u32::MAX);
        let mut next = self
            .display_indices
            .values()
            .flatten()
            .max()
            .map_or(0, |index| index.saturating_add(1))
            .max(vanilla);
        for (id, value) in recipes {
            let Some(mut recipe) = parse_recipe(&value) else {
                continue;
            };
            if self.recipes.iter().any(|known| known.id == id) {
                continue;
            }
            recipe.id = id.clone();
            self.recipes.push(recipe);
            self.display_indices.insert(id, vec![next]);
            next = next.saturating_add(1);
        }
        self
    }

    pub fn display_indices(&self, id: &str) -> Option<&[u32]> {
        self.display_indices.get(id).map(Vec::as_slice)
    }

    /// Pinned 26.3 recipe advancements mostly use one inventory_changed
    /// predicate. InventoryChangeTrigger tests a single predicate against the
    /// changed stack, and a slots-only predicate against current occupancy.
    pub fn auto_unlocks_for_change<'a>(
        &'a self,
        changed_item: &'a ItemStack,
        occupied_slots: usize,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.auto_unlocks.iter().filter_map(move |rule| {
            let matches = match &rule.criterion {
                AutoUnlockCriterion::ChangedItem(ingredient) => {
                    self.matches_ingredient(ingredient, &changed_item.id)
                }
                AutoUnlockCriterion::OccupiedSlots(minimum) => occupied_slots >= *minimum,
            };
            matches.then_some(rule.recipe_id.as_str())
        })
    }

    pub fn with_item_catalog(mut self, catalog: Arc<ItemCatalog>) -> Self {
        for recipe in &mut self.recipes {
            recipe.result.max = catalog.max_stack(&recipe.result.id);
        }
        for entry in &mut self.smelting {
            entry.recipe.result.max = catalog.max_stack(&entry.recipe.result.id);
        }
        self.item_catalog = Some(catalog);
        self
    }

    pub fn with_reserved_slots(mut self, slot: fn(&str) -> Option<usize>) -> Self {
        self.reserved_slots = Some(slot);
        self
    }

    pub fn reserved_slot(&self, id: &str) -> Option<usize> {
        self.reserved_slots.and_then(|slot| slot(id))
    }

    pub fn item_catalog(&self) -> Option<&ItemCatalog> {
        self.item_catalog.as_deref()
    }

    pub fn max_stack(&self, id: &str) -> u8 {
        self.item_catalog()
            .map_or(64, |catalog| catalog.max_stack(id))
    }

    /// A stack's explicit component patch takes precedence over its default
    /// item component. A null patch removes the default component.
    pub fn equipment_slot<'a>(&'a self, stack: &'a ItemStack) -> Option<&'a str> {
        if let Some(value) = stack
            .components
            .as_ref()
            .and_then(|components| components.get("minecraft:equippable"))
        {
            return value.get("slot").and_then(Value::as_str);
        }
        self.item_catalog()
            .and_then(|catalog| catalog.get(&stack.id))
            .and_then(|item| item.equippable_slot.as_deref())
    }

    pub fn equipment_asset<'a>(&'a self, stack: &'a ItemStack) -> Option<&'a str> {
        if let Some(value) = stack
            .components
            .as_ref()
            .and_then(|components| components.get("minecraft:equippable"))
        {
            return value.get("asset_id").and_then(Value::as_str);
        }
        self.item_catalog()
            .and_then(|catalog| catalog.get(&stack.id))
            .and_then(|item| item.equipment_asset.as_deref())
    }

    pub fn armor_points(&self, stack: &ItemStack, slot: &str) -> f64 {
        if stack
            .components
            .as_ref()
            .is_some_and(|components| components.get("minecraft:attribute_modifiers").is_some())
        {
            return 0.0;
        }
        self.item_catalog()
            .and_then(|catalog| catalog.get(&stack.id))
            .filter(|item| item.equippable_slot.as_deref() == Some(slot))
            .map_or(0.0, |item| item.armor_points)
    }

    /// A stack's attribute modifiers: its component patch's when it has
    /// one, else the item's defaults.
    pub fn item_modifiers(&self, stack: &ItemStack) -> Vec<crate::item_catalog::AttributeModifier> {
        use crate::item_catalog::{AttributeModifier, ModifierOperation};
        if let Some(patch) = stack.components.as_ref().and_then(|components| components.get("minecraft:attribute_modifiers")) {
            // The list form, or an object holding `modifiers`.
            let list = patch.as_array().or_else(|| patch.get("modifiers").and_then(Value::as_array));
            return list
                .into_iter()
                .flatten()
                .filter_map(|m| {
                    Some(AttributeModifier {
                        attribute: m.get("type").or_else(|| m.get("attribute")).and_then(Value::as_str)?.to_owned(),
                        id: m.get("id").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        amount: m.get("amount").and_then(Value::as_f64)?,
                        operation: ModifierOperation::parse(m.get("operation").and_then(Value::as_str)?)?,
                        slot: m.get("slot").and_then(Value::as_str).unwrap_or("any").to_owned(),
                    })
                })
                .collect();
        }
        self.item_catalog().and_then(|catalog| catalog.get(&stack.id)).map(|item| item.attribute_modifiers.clone()).unwrap_or_default()
    }

    /// The player's `ATTACK_DAMAGE` and `ATTACK_SPEED` (1 and 4 in
    /// `Player.createAttributes`) with the main hand's modifiers.
    pub fn attack_attributes(&self, held: Option<&ItemStack>) -> (f64, f64) {
        use crate::item_catalog::attribute_value;
        let modifiers = held.map(|stack| self.item_modifiers(stack)).unwrap_or_default();
        let of = |attribute: &'static str| modifiers.iter().filter(move |m| m.in_main_hand() && m.attribute == attribute);
        (attribute_value(1.0, of("minecraft:attack_damage"), (0.0, 2048.0)), attribute_value(4.0, of("minecraft:attack_speed"), (0.0, 1024.0)))
    }

    /// The `weapon` component: durability lost per attack and how long a
    /// hit disables a shield.
    pub fn weapon(&self, stack: &ItemStack) -> Option<(u32, f32)> {
        if let Some(patch) = stack.components.as_ref().and_then(|components| components.get("minecraft:weapon")) {
            let per_attack = patch.get("item_damage_per_attack").and_then(Value::as_u64).unwrap_or(1) as u32;
            let disable = patch.get("disable_blocking_for_seconds").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            return Some((per_attack, disable));
        }
        self.item_catalog().and_then(|catalog| catalog.get(&stack.id)).and_then(|item| item.weapon)
    }

    /// The armor toughness and knockback resistance a worn piece adds, by
    /// its material (`ArmorMaterials`: diamond 2 toughness; netherite 3 and
    /// 0.1 knockback resistance).
    pub fn armor_toughness(&self, stack: &ItemStack, slot: &str) -> (f64, f64) {
        if stack
            .components
            .as_ref()
            .is_some_and(|components| components.get("minecraft:attribute_modifiers").is_some())
        {
            return (0.0, 0.0);
        }
        let asset = self
            .item_catalog()
            .and_then(|catalog| catalog.get(&stack.id))
            .filter(|item| item.equippable_slot.as_deref() == Some(slot))
            .and_then(|item| item.equipment_asset.as_deref());
        match asset {
            Some("minecraft:diamond") => (2.0, 0.0),
            Some("minecraft:netherite") => (3.0, 0.1),
            _ => (0.0, 0.0),
        }
    }

    /// Returns the damage and maximum damage for a damageable stack, honoring
    /// explicit component patches and the UNBREAKABLE component.
    pub fn durability(&self, stack: &ItemStack) -> Option<(u32, u32)> {
        let defaults = self
            .item_catalog()
            .and_then(|catalog| catalog.get(&stack.id));
        let patch = stack.components.as_ref();
        if patch
            .and_then(|components| components.get("minecraft:unbreakable"))
            .is_some_and(|value| !value.is_null())
        {
            return None;
        }
        let max_damage = match patch.and_then(|components| components.get("minecraft:max_damage")) {
            Some(value) => u32::try_from(value.as_u64()?).ok()?,
            None => defaults.map_or(0, |item| item.max_damage),
        };
        if max_damage == 0 {
            return None;
        }
        let damage = match patch.and_then(|components| components.get("minecraft:damage")) {
            Some(value) => u32::try_from(value.as_u64()?).ok()?.min(max_damage),
            None => 0,
        };
        Some((damage, max_damage))
    }

    /// Default or stack-patched TOOL damage for one mined block.
    pub fn mining_tool_wear(&self, stack: &ItemStack) -> Option<(u32, u32)> {
        let (_, max_damage) = self.durability(stack)?;
        let defaults = self
            .item_catalog()
            .and_then(|catalog| catalog.get(&stack.id));
        let damage_per_block = match stack
            .components
            .as_ref()
            .and_then(|components| components.get("minecraft:tool"))
        {
            Some(value) => u32::try_from(value.get("damage_per_block")?.as_u64()?).ok()?,
            None => defaults.map_or(0, |item| item.damage_per_block),
        };
        if damage_per_block == 0 {
            return None;
        }
        Some((max_damage, damage_per_block))
    }

    pub fn stack(&self, id: &str, count: u8) -> ItemStack {
        let mut stack = ItemStack::new(id, count);
        stack.max = self.max_stack(id);
        stack
    }

    pub fn fuel_ticks(&self, id: &str) -> u32 {
        if let Some(catalog) = self.item_catalog() {
            return catalog.get(id).map_or(0, |item| item.burn_ticks);
        }
        crate::furnace::fuel_ticks(id)
    }

    pub fn fuel_speed(&self, id: &str) -> f32 {
        self.item_catalog()
            .and_then(|catalog| catalog.get(id))
            .map_or(1.0, |item| item.speed_multiplier)
    }

    pub fn is_fuel(&self, id: &str) -> bool {
        if let Some(catalog) = self.item_catalog() {
            return catalog.get(id).is_some_and(|item| item.fuel_component);
        }
        crate::furnace::fuel_ticks(id) > 0
    }

    pub fn crafting_remainder(&self, id: &str) -> Option<ItemStack> {
        if let Some(catalog) = self.item_catalog() {
            let (id, count) = catalog.get(id)?.remainder.as_ref()?;
            return Some(self.stack(id, *count));
        }
        (id == "minecraft:lava_bucket").then(|| ItemStack::new("minecraft:bucket", 1))
    }

    pub fn count(&self) -> usize {
        self.recipes.len()
    }

    pub fn crafting_recipes(&self) -> impl Iterator<Item = CraftingRecipeRef<'_>> {
        self.recipes.iter().filter_map(|recipe| {
            let (width, height, ingredients) = match &recipe.kind {
                Kind::Shaped(rows) => (
                    rows[0].len(),
                    rows.len(),
                    rows.iter().flatten().filter(|cell| cell.is_some()).count(),
                ),
                Kind::Shapeless(items) => (0, 0, items.len()),
                _ => return None,
            };
            Some(CraftingRecipeRef {
                id: &recipe.id,
                category: &recipe.category,
                group: recipe.group.as_deref(),
                result: &recipe.result,
                width,
                height,
                ingredients,
            })
        })
    }

    pub fn crafting_grid_candidates(
        &self,
        recipe_id: &str,
        grid_width: usize,
        grid_height: usize,
    ) -> Option<Vec<Option<Vec<String>>>> {
        let recipe = self.recipes.iter().find(|recipe| recipe.id == recipe_id)?;
        let mut grid = vec![None; grid_width * grid_height];
        match &recipe.kind {
            Kind::Shaped(rows) => {
                let (width, height) = (rows[0].len(), rows.len());
                if width > grid_width || height > grid_height {
                    return None;
                }
                // 26.3 PlaceRecipeHelper centers a dimension only when the
                // recipe is smaller than half the grid dimension.
                let offset_x = if (width as f32) < grid_width as f32 / 2.0 {
                    (grid_width - width) / 2
                } else {
                    0
                };
                let offset_y = if (height as f32) < grid_height as f32 / 2.0 {
                    (grid_height - height) / 2
                } else {
                    0
                };
                for (y, row) in rows.iter().enumerate() {
                    for (x, ingredient) in row.iter().enumerate() {
                        if let Some(ingredient) = ingredient {
                            grid[(y + offset_y) * grid_width + x + offset_x] =
                                Some(self.ingredient_items(ingredient));
                        }
                    }
                }
            }
            Kind::Shapeless(ingredients) => {
                if ingredients.len() > grid.len() {
                    return None;
                }
                for (slot, ingredient) in ingredients.iter().enumerate() {
                    grid[slot] = Some(self.ingredient_items(ingredient));
                }
            }
            _ => return None,
        }
        Some(grid)
    }

    fn ingredient_items(&self, ingredient: &Ingredient) -> Vec<String> {
        fn expand(
            book: &RecipeBook,
            candidate: &str,
            visited: &mut HashSet<String>,
            result: &mut Vec<String>,
        ) {
            if let Some(tag) = candidate.strip_prefix('#') {
                if visited.insert(tag.to_owned()) {
                    if let Some(entries) = book.tags.get(tag) {
                        for entry in entries {
                            expand(book, entry, visited, result);
                        }
                    }
                    visited.remove(tag);
                }
            } else if !result.iter().any(|id| id == candidate) {
                result.push(candidate.to_owned());
            }
        }
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        for candidate in &ingredient.0 {
            expand(self, candidate, &mut visited, &mut result);
        }
        result
    }

    pub fn smelting_count(&self) -> usize {
        self.cooking_count(CookingKind::Furnace)
    }

    pub fn cooking_count(&self, kind: CookingKind) -> usize {
        self.smelting
            .iter()
            .filter(|entry| entry.kind == kind)
            .count()
    }

    pub fn cooking_recipes(&self, kind: CookingKind) -> impl Iterator<Item = CookingRecipeRef<'_>> {
        self.smelting
            .iter()
            .filter(move |entry| entry.kind == kind)
            .map(|entry| CookingRecipeRef {
                id: &entry.id,
                category: &entry.category,
                group: entry.group.as_deref(),
                result: &entry.recipe.result,
            })
    }

    pub fn cooking_recipe_accepts(
        &self,
        kind: CookingKind,
        recipe_id: &str,
        item_id: &str,
    ) -> bool {
        self.smelting
            .iter()
            .find(|entry| entry.kind == kind && entry.id == recipe_id)
            .is_some_and(|entry| self.matches_ingredient(&entry.ingredient, item_id))
    }

    /// First display item for the cooking input. The ingredient choices in
    /// the external recipe and item tags retain their pack order.
    pub fn cooking_display_ingredient(
        &self,
        kind: CookingKind,
        recipe_id: &str,
    ) -> Option<ItemStack> {
        let ingredient = &self
            .smelting
            .iter()
            .find(|entry| entry.kind == kind && entry.id == recipe_id)?
            .ingredient;
        ingredient.0.iter().find_map(|candidate| {
            self.first_ingredient_item(candidate, &mut HashSet::new())
                .map(|id| self.stack(id, 1))
        })
    }

    fn first_ingredient_item<'a>(
        &'a self,
        candidate: &'a str,
        visited: &mut HashSet<String>,
    ) -> Option<&'a str> {
        let Some(tag) = candidate.strip_prefix('#') else {
            return Some(candidate);
        };
        if !visited.insert(tag.to_owned()) {
            return None;
        }
        let item = self
            .tags
            .get(tag)?
            .iter()
            .find_map(|entry| self.first_ingredient_item(entry, visited));
        visited.remove(tag);
        item
    }

    pub fn smelting_for(&self, input: &ItemStack) -> Option<&SmeltingRecipe> {
        self.cooking_for(CookingKind::Furnace, input)
    }

    pub fn cooking_for(&self, kind: CookingKind, input: &ItemStack) -> Option<&SmeltingRecipe> {
        self.cooking_entry_for(kind, input).map(|(_, recipe)| recipe)
    }

    pub fn cooking_entry_for(&self, kind: CookingKind, input: &ItemStack) -> Option<(&str, &SmeltingRecipe)> {
        self.smelting
            .iter()
            .filter(|entry| entry.kind == kind)
            .find(|entry| self.matches_ingredient(&entry.ingredient, &input.id))
            .map(|entry| (entry.id.as_str(), &entry.recipe))
    }

    pub fn cooking_recipe(&self, id: &str) -> Option<&SmeltingRecipe> {
        self.smelting.iter().find(|entry| entry.id == id).map(|entry| &entry.recipe)
    }

    pub fn matching(
        &self,
        grid: &[Option<ItemStack>],
        width: usize,
        height: usize,
    ) -> Option<ItemStack> {
        if grid.len() != width * height {
            return None;
        }
        self.recipes.iter().find_map(|recipe| match &recipe.kind {
            Kind::Shaped(_) | Kind::Shapeless(_) => {
                let matched = self.recipe_matches(recipe, grid, width, height);
                matched.then(|| recipe.result.clone())
            }
            _ => self.special_result(recipe, grid),
        })
    }

    pub fn matching_id(
        &self,
        grid: &[Option<ItemStack>],
        width: usize,
        height: usize,
    ) -> Option<&str> {
        if grid.len() != width * height {
            return None;
        }
        self.recipes
            .iter()
            .find(|recipe| self.recipe_matches(recipe, grid, width, height))
            .map(|recipe| recipe.id.as_str())
    }

    pub fn matches_crafting_id(
        &self,
        recipe_id: &str,
        grid: &[Option<ItemStack>],
        width: usize,
        height: usize,
    ) -> bool {
        self.recipes
            .iter()
            .find(|recipe| recipe.id == recipe_id)
            .is_some_and(|recipe| self.recipe_matches(recipe, grid, width, height))
    }

    fn recipe_matches(
        &self,
        recipe: &Recipe,
        grid: &[Option<ItemStack>],
        width: usize,
        height: usize,
    ) -> bool {
        match &recipe.kind {
            Kind::Shaped(pattern) => self.matches_shaped(pattern, grid, width, height),
            Kind::Shapeless(ingredients) => self.matches_shapeless(ingredients, grid),
            _ => self.special_result(recipe, grid).is_some(),
        }
    }

    fn special_result(&self, recipe: &Recipe, grid: &[Option<ItemStack>]) -> Option<ItemStack> {
        let items = grid.iter().filter_map(Option::as_ref).collect::<Vec<_>>();
        match &recipe.kind {
            Kind::FireworkStar {
                shapes,
                trail,
                twinkle,
                fuel,
                dye,
            } => {
                if items.len() < 2 {
                    return None;
                }
                let (mut shape, mut has_trail, mut has_twinkle, mut has_fuel) =
                    (None, false, false, false);
                let mut colors = Vec::new();
                for stack in items {
                    if self.matches_ingredient(twinkle, &stack.id) {
                        if has_twinkle {
                            return None;
                        }
                        has_twinkle = true;
                    } else if self.matches_ingredient(trail, &stack.id) {
                        if has_trail {
                            return None;
                        }
                        has_trail = true;
                    } else if self.matches_ingredient(fuel, &stack.id) {
                        if has_fuel {
                            return None;
                        }
                        has_fuel = true;
                    } else if let Some(color) = dye_color(stack)
                        .filter(|_| self.matches_ingredient(dye, &stack.id))
                    {
                        colors.push(color);
                    } else if let Some((name, _)) = shapes
                        .iter()
                        .find(|(_, ingredient)| self.matches_ingredient(ingredient, &stack.id))
                    {
                        if shape.is_some() {
                            return None;
                        }
                        shape = Some(name.as_str());
                    } else {
                        return None;
                    }
                }
                if !has_fuel || colors.is_empty() {
                    return None;
                }
                let mut explosion = serde_json::Map::new();
                explosion.insert("shape".to_owned(), Value::from(shape.unwrap_or("small_ball")));
                explosion.insert("colors".to_owned(), Value::from(colors));
                if has_trail {
                    explosion.insert("has_trail".to_owned(), Value::from(1));
                }
                if has_twinkle {
                    explosion.insert("has_twinkle".to_owned(), Value::from(1));
                }
                let mut star = recipe.result.clone();
                set_component(&mut star, FIREWORK_EXPLOSION, Some(Value::Object(explosion)));
                Some(star)
            }
            Kind::FireworkStarFade { target, dye } => {
                if items.len() < 2 {
                    return None;
                }
                let mut star = None;
                let mut fades = Vec::new();
                for stack in items {
                    if let Some(color) =
                        dye_color(stack).filter(|_| self.matches_ingredient(dye, &stack.id))
                    {
                        fades.push(color);
                    } else if self.matches_ingredient(target, &stack.id) {
                        if star.is_some() {
                            return None;
                        }
                        star = Some(stack);
                    } else {
                        return None;
                    }
                }
                let star = star.filter(|_| !fades.is_empty())?;
                let mut faded = transmuted(&recipe.result, star);
                let mut explosion = faded
                    .components
                    .as_ref()
                    .and_then(|patch| patch.get(FIREWORK_EXPLOSION))
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_else(|| {
                        serde_json::Map::from_iter([(
                            "shape".to_owned(),
                            Value::from("small_ball"),
                        )])
                    });
                explosion.insert("fade_colors".to_owned(), Value::from(fades));
                set_component(&mut faded, FIREWORK_EXPLOSION, Some(Value::Object(explosion)));
                Some(faded)
            }
            Kind::FireworkRocket { shell, fuel, star } => {
                if items.len() < 2 {
                    return None;
                }
                let (mut has_shell, mut flight) = (false, 0);
                let mut explosions = Vec::new();
                for stack in items {
                    if self.matches_ingredient(shell, &stack.id) {
                        if has_shell {
                            return None;
                        }
                        has_shell = true;
                    } else if self.matches_ingredient(fuel, &stack.id) {
                        flight += 1;
                        if flight > MAX_FUEL {
                            return None;
                        }
                    } else if self.matches_ingredient(star, &stack.id) {
                        explosions.extend(
                            stack
                                .components
                                .as_ref()
                                .and_then(|patch| patch.get(FIREWORK_EXPLOSION))
                                .cloned(),
                        );
                    } else {
                        return None;
                    }
                }
                if !has_shell || flight == 0 {
                    return None;
                }
                let fireworks = (flight != DEFAULT_FLIGHT || !explosions.is_empty()).then(|| {
                    let mut fireworks = serde_json::Map::new();
                    fireworks.insert("flight_duration".to_owned(), Value::from(flight));
                    if !explosions.is_empty() {
                        fireworks.insert("explosions".to_owned(), Value::Array(explosions));
                    }
                    Value::Object(fireworks)
                });
                let mut rockets = recipe.result.clone();
                set_component(&mut rockets, FIREWORKS, fireworks);
                Some(rockets)
            }
            Kind::ShieldDecoration { banner, target } => {
                if items.len() != 2 {
                    return None;
                }
                let (mut flag, mut shield) = (None, None);
                for stack in items {
                    if let Some(color) =
                        banner_color(&stack.id).filter(|_| self.matches_ingredient(banner, &stack.id))
                    {
                        if flag.is_some() {
                            return None;
                        }
                        flag = Some((stack, color));
                    } else if self.matches_ingredient(target, &stack.id) {
                        if shield.is_some() || banner_patterns(stack).is_some() {
                            return None;
                        }
                        shield = Some(stack);
                    } else {
                        return None;
                    }
                }
                let ((flag, color), shield) = (flag?, shield?);
                let mut decorated = transmuted(&recipe.result, shield);
                set_component(&mut decorated, BANNER_PATTERNS, banner_patterns(flag));
                set_component(&mut decorated, BASE_COLOR, Some(Value::from(color)));
                Some(decorated)
            }
            Kind::Shaped(_) | Kind::Shapeless(_) => None,
        }
    }

    fn matches_ingredient(&self, ingredient: &Ingredient, id: &str) -> bool {
        ingredient
            .0
            .iter()
            .any(|candidate| self.matches_id(candidate, id, &mut HashSet::new()))
    }

    fn matches_id(&self, candidate: &str, id: &str, visited: &mut HashSet<String>) -> bool {
        if let Some(tag) = candidate.strip_prefix('#') {
            if !visited.insert(tag.to_owned()) {
                return false;
            }
            let matched = self.tags.get(tag).is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| self.matches_id(entry, id, visited))
            });
            visited.remove(tag);
            matched
        } else {
            candidate == id
        }
    }

    /// Resolve an item tag from the pinned gameplay data JAR, including nested tags.
    pub fn item_in_tag(&self, tag: &str, id: &str) -> bool {
        self.matches_id(&format!("#{tag}"), id, &mut HashSet::new())
    }

    fn matches_shaped(
        &self,
        pattern: &[Vec<Option<Ingredient>>],
        grid: &[Option<ItemStack>],
        width: usize,
        height: usize,
    ) -> bool {
        let ph = pattern.len();
        let pw = pattern.first().map_or(0, Vec::len);
        if pw == 0 || ph == 0 || pw > width || ph > height {
            return false;
        }
        for oy in 0..=height - ph {
            for ox in 0..=width - pw {
                for mirror in [false, true] {
                    let mut matched = true;
                    for y in 0..height {
                        for x in 0..width {
                            let expected = if x >= ox && x < ox + pw && y >= oy && y < oy + ph {
                                let px = if mirror { pw - 1 - (x - ox) } else { x - ox };
                                pattern[y - oy][px].as_ref()
                            } else {
                                None
                            };
                            let actual = grid[y * width + x].as_ref();
                            if !match (expected, actual) {
                                (None, None) => true,
                                (Some(ingredient), Some(stack)) => {
                                    self.matches_ingredient(ingredient, &stack.id)
                                }
                                _ => false,
                            } {
                                matched = false;
                                break;
                            }
                        }
                    }
                    if matched {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn matches_shapeless(&self, ingredients: &[Ingredient], grid: &[Option<ItemStack>]) -> bool {
        let items = grid.iter().filter_map(Option::as_ref).collect::<Vec<_>>();
        if items.len() != ingredients.len() {
            return false;
        }
        fn assign(
            book: &RecipeBook,
            ingredients: &[Ingredient],
            items: &[&ItemStack],
            index: usize,
            used: u16,
        ) -> bool {
            if index == ingredients.len() {
                return true;
            }
            items.iter().enumerate().any(|(slot, stack)| {
                used & (1 << slot) == 0
                    && book.matches_ingredient(&ingredients[index], &stack.id)
                    && assign(book, ingredients, items, index + 1, used | (1 << slot))
            })
        }
        assign(self, ingredients, &items, 0, 0)
    }
}

fn auto_unlock_rules(advancement: &Value, known_ids: &HashSet<&str>) -> Vec<AutoUnlockRule> {
    let Some(requirements) = advancement
        .get("requirements")
        .and_then(Value::as_array)
        .filter(|groups| groups.len() == 1)
        .and_then(|groups| groups[0].as_array())
    else {
        return Vec::new();
    };
    let Some(criteria) = advancement.get("criteria").and_then(Value::as_object) else {
        return Vec::new();
    };
    let rewards = advancement
        .get("rewards")
        .and_then(|rewards| rewards.get("recipes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|id| known_ids.contains(id))
        .collect::<Vec<_>>();
    let mut rules = Vec::new();
    for requirement in requirements.iter().filter_map(Value::as_str) {
        let Some(condition) = criteria
            .get(requirement)
            .filter(|criterion| {
                criterion.get("trigger").and_then(Value::as_str)
                    == Some("minecraft:inventory_changed")
            })
            .and_then(|criterion| criterion.get("conditions"))
        else {
            continue;
        };
        let criterion = if let Some(item) = condition
            .get("items")
            .and_then(Value::as_array)
            .filter(|items| items.len() == 1)
            .and_then(|items| items[0].get("items"))
            .and_then(Ingredient::parse)
        {
            AutoUnlockCriterion::ChangedItem(item)
        } else if let Some(minimum) = condition
            .get("slots")
            .and_then(|slots| slots.get("occupied"))
            .and_then(|occupied| occupied.get("min"))
            .and_then(Value::as_u64)
            .and_then(|minimum| usize::try_from(minimum).ok())
        {
            AutoUnlockCriterion::OccupiedSlots(minimum)
        } else {
            continue;
        };
        for id in &rewards {
            rules.push(AutoUnlockRule {
                recipe_id: (*id).to_owned(),
                criterion: criterion.clone(),
            });
        }
    }
    rules
}

fn parse_recipe(value: &Value) -> Option<Recipe> {
    let result = parse_result(value)?;
    let kind = match value.get("type")?.as_str()? {
        "minecraft:crafting_shaped" => {
            let key = value.get("key")?.as_object()?;
            let pattern = value.get("pattern")?.as_array()?;
            let rows = pattern
                .iter()
                .map(|row| {
                    row.as_str()?
                        .chars()
                        .map(|symbol| {
                            if symbol == ' ' {
                                Some(None)
                            } else {
                                Some(Some(Ingredient::parse(key.get(&symbol.to_string())?)?))
                            }
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .collect::<Option<Vec<_>>>()?;
            if rows.is_empty() || rows.iter().any(|row| row.len() != rows[0].len()) {
                return None;
            }
            Kind::Shaped(trim_pattern(rows)?)
        }
        "minecraft:crafting_shapeless" => Kind::Shapeless(
            value
                .get("ingredients")?
                .as_array()?
                .iter()
                .map(Ingredient::parse)
                .collect::<Option<Vec<_>>>()?,
        ),
        "minecraft:crafting_special_firework_star" => Kind::FireworkStar {
            shapes: value
                .get("shapes")?
                .as_object()?
                .iter()
                .map(|(shape, ingredient)| {
                    FIREWORK_SHAPES.contains(&shape.as_str()).then_some(())?;
                    Some((shape.clone(), Ingredient::parse(ingredient)?))
                })
                .collect::<Option<Vec<_>>>()?,
            trail: Ingredient::parse(value.get("trail")?)?,
            twinkle: Ingredient::parse(value.get("twinkle")?)?,
            fuel: Ingredient::parse(value.get("fuel")?)?,
            dye: Ingredient::parse(value.get("dye")?)?,
        },
        "minecraft:crafting_special_firework_star_fade" => Kind::FireworkStarFade {
            target: Ingredient::parse(value.get("target")?)?,
            dye: Ingredient::parse(value.get("dye")?)?,
        },
        "minecraft:crafting_special_firework_rocket" => Kind::FireworkRocket {
            shell: Ingredient::parse(value.get("shell")?)?,
            fuel: Ingredient::parse(value.get("fuel")?)?,
            star: Ingredient::parse(value.get("star")?)?,
        },
        "minecraft:crafting_special_shielddecoration" => Kind::ShieldDecoration {
            banner: Ingredient::parse(value.get("banner")?)?,
            target: Ingredient::parse(value.get("target")?)?,
        },
        _ => return None,
    };
    Some(Recipe {
        id: String::new(),
        category: value.get("category").and_then(Value::as_str).unwrap_or("misc").to_owned(),
        group: value.get("group").and_then(Value::as_str).filter(|group| !group.is_empty()).map(str::to_owned),
        kind,
        result,
    })
}

fn parse_result(value: &Value) -> Option<ItemStack> {
    let result = value.get("result")?;
    let id = result.get("id")?.as_str()?;
    let count = u8::try_from(result.get("count").and_then(Value::as_u64).unwrap_or(1)).ok()?;
    let mut stack = ItemStack::new(id, count);
    stack.components = result
        .get("components")
        .filter(|components| components.as_object().is_some_and(|map| !map.is_empty()))
        .cloned();
    Some(stack)
}

fn dye_color(stack: &ItemStack) -> Option<i32> {
    let patch = stack.components.as_ref();
    if patch.is_some_and(|patch| patch.get(format!("!{DYE}")).is_some()) {
        return None;
    }
    let name = match patch.and_then(|patch| patch.get(DYE)) {
        Some(dye) => dye.as_str()?,
        None => stack.id.strip_prefix("minecraft:")?.strip_suffix("_dye")?,
    };
    DYE_COLORS
        .iter()
        .find(|(dye, _)| *dye == name)
        .map(|(_, color)| *color)
}

fn banner_color(id: &str) -> Option<&'static str> {
    let name = id.strip_prefix("minecraft:")?.strip_suffix("_banner")?;
    DYE_COLORS
        .iter()
        .find(|(dye, _)| *dye == name)
        .map(|(dye, _)| *dye)
}

fn banner_patterns(stack: &ItemStack) -> Option<Value> {
    stack
        .components
        .as_ref()?
        .get(BANNER_PATTERNS)
        .filter(|patterns| patterns.as_array().is_some_and(|layers| !layers.is_empty()))
        .cloned()
}

fn set_component(stack: &mut ItemStack, key: &str, value: Option<Value>) {
    let mut patch = match stack.components.take() {
        Some(Value::Object(patch)) => patch,
        _ => serde_json::Map::new(),
    };
    patch.remove(&format!("!{key}"));
    match value {
        Some(value) => patch.insert(key.to_owned(), value),
        None => patch.remove(key),
    };
    stack.components = (!patch.is_empty()).then_some(Value::Object(patch));
}

fn transmuted(result: &ItemStack, input: &ItemStack) -> ItemStack {
    let mut stack = result.clone();
    if let Some(Value::Object(patch)) = &input.components {
        for (key, value) in patch {
            set_component(&mut stack, key, Some(value.clone()));
        }
    }
    stack
}

fn trim_pattern(rows: Vec<Vec<Option<Ingredient>>>) -> Option<Vec<Vec<Option<Ingredient>>>> {
    let mut top = rows.len();
    let mut bottom = 0;
    let mut left = rows[0].len();
    let mut right = 0;
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if cell.is_some() {
                top = top.min(y);
                bottom = bottom.max(y + 1);
                left = left.min(x);
                right = right.max(x + 1);
            }
        }
    }
    (top < bottom && left < right).then(|| {
        rows[top..bottom]
            .iter()
            .map(|row| row[left..right].to_vec())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Inventory;
    #[test]
    fn shaped_recipe_matches_offset_and_mirror() {
        let mut book = RecipeBook::default();
        book.recipes.push(
            parse_recipe(&serde_json::json!({
                "type":"minecraft:crafting_shaped",
                "key":{"A":"minecraft:stone","B":"minecraft:dirt"},
                "pattern":[" AB "],"result":{"id":"minecraft:bricks"}
            }))
            .unwrap(),
        );
        let grid = [
            None,
            None,
            Some(ItemStack::new("minecraft:dirt", 1)),
            Some(ItemStack::new("minecraft:stone", 1)),
        ];
        assert_eq!(book.matching(&grid, 2, 2).unwrap().id, "minecraft:bricks");
    }
    #[test]
    fn shapeless_recipe_expands_item_tag() {
        let mut book = RecipeBook::default();
        book.tags
            .insert("minecraft:logs".into(), vec!["minecraft:oak_log".into()]);
        book.recipes.push(
            parse_recipe(&serde_json::json!({
                "type":"minecraft:crafting_shapeless", "ingredients":["#minecraft:logs"],
                "result":{"id":"minecraft:oak_planks","count":4}
            }))
            .unwrap(),
        );
        let grid = [
            None,
            Some(ItemStack::new("minecraft:oak_log", 1)),
            None,
            None,
        ];
        assert_eq!(book.matching(&grid, 2, 2).unwrap().count, 4);
    }
    #[test]
    fn taking_result_consumes_inputs_and_preserves_item_counts() {
        let mut book = RecipeBook::default();
        book.tags.insert(
            "minecraft:oak_logs".into(),
            vec!["minecraft:oak_log".into()],
        );
        book.recipes.push(
            parse_recipe(&serde_json::json!({
                "type":"minecraft:crafting_shapeless", "ingredients":["#minecraft:oak_logs"],
                "result":{"id":"minecraft:oak_planks","count":4}
            }))
            .unwrap(),
        );
        let mut inv = Inventory::default().with_recipes(book);
        inv.crafting[0] = Some(ItemStack::new("minecraft:oak_log", 2));
        assert_eq!(inv.crafting_output().unwrap().count, 4);
        assert!(inv.take_crafting_output(false));
        assert_eq!(inv.count("minecraft:oak_log"), 1);
        assert_eq!(inv.count("minecraft:oak_planks"), 4);
        assert!(inv.take_crafting_output(false));
        assert_eq!(inv.count("minecraft:oak_log"), 0);
        assert_eq!(inv.count("minecraft:oak_planks"), 8);
        assert!(!inv.take_crafting_output(false));
    }
    #[test]
    fn three_by_three_furnace_consumes_eight_cobblestone() {
        let mut book = RecipeBook::default();
        book.recipes.push(
            parse_recipe(&serde_json::json!({
                "type":"minecraft:crafting_shaped",
                "key":{"#":"minecraft:cobblestone"},
                "pattern":["###","# #","###"],
                "result":{"id":"minecraft:furnace"}
            }))
            .unwrap(),
        );
        let mut inv = Inventory::default().with_recipes(book);
        for index in [0, 1, 2, 3, 5, 6, 7, 8] {
            inv.workbench[index] = Some(ItemStack::new("minecraft:cobblestone", 1));
        }
        assert_eq!(inv.workbench_output().unwrap().id, "minecraft:furnace");
        assert!(inv.take_workbench_output(false));
        assert_eq!(inv.count("minecraft:cobblestone"), 0);
        assert_eq!(inv.count("minecraft:furnace"), 1);
        assert!(inv.workbench_output().is_none());
    }
    #[test]
    fn closing_workbench_returns_unused_inputs() {
        let mut inv = Inventory::default();
        inv.workbench[0] = Some(ItemStack::new("minecraft:cobblestone", 12));
        inv.workbench[8] = Some(ItemStack::new("minecraft:oak_log", 3));
        assert!(inv.settle_workbench().is_empty());
        assert_eq!(inv.count("minecraft:cobblestone"), 12);
        assert_eq!(inv.count("minecraft:oak_log"), 3);
        assert!(inv.workbench.iter().all(Option::is_none));
    }
    #[test]
    fn a_recipe_result_keeps_its_components() {
        let effects = serde_json::json!({"minecraft:suspicious_stew_effects": [{"duration": 60, "id": "minecraft:fire_resistance"}]});
        let recipe = parse_recipe(&serde_json::json!({
            "type": "minecraft:crafting_shapeless",
            "ingredients": ["minecraft:bowl", "minecraft:allium"],
            "result": {"id": "minecraft:suspicious_stew", "components": effects.clone()}
        }))
        .unwrap();
        assert_eq!(recipe.result.components.as_ref(), Some(&effects));
        let plain = parse_recipe(&serde_json::json!({
            "type": "minecraft:crafting_shapeless",
            "ingredients": ["minecraft:bowl"],
            "result": {"id": "minecraft:mushroom_stew", "components": {}}
        }))
        .unwrap();
        assert_eq!(plain.result.components, None);
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = RecipeBook::from_jar(&jar).unwrap();
        let mut grid: [Option<ItemStack>; 9] = std::array::from_fn(|_| None);
        grid[0] = Some(ItemStack::new("minecraft:bowl", 1));
        grid[1] = Some(ItemStack::new("minecraft:red_mushroom", 1));
        grid[2] = Some(ItemStack::new("minecraft:brown_mushroom", 1));
        grid[4] = Some(ItemStack::new("minecraft:allium", 1));
        let stew = book.matching(&grid, 3, 3).unwrap();
        assert_eq!(stew.id, "minecraft:suspicious_stew");
        assert_eq!(stew.components, Some(effects));
        grid[4] = Some(ItemStack::new("minecraft:oxeye_daisy", 1));
        let regeneration = book.matching(&grid, 3, 3).unwrap();
        assert_eq!(
            regeneration.components.as_ref().and_then(|c| c["minecraft:suspicious_stew_effects"][0]["id"].as_str()),
            Some("minecraft:regeneration")
        );
        let mut planks: [Option<ItemStack>; 4] = std::array::from_fn(|_| None);
        planks[0] = Some(ItemStack::new("minecraft:oak_log", 1));
        assert_eq!(book.matching(&planks, 2, 2).map(|stack| stack.components), Some(None));
    }

    fn firework_book() -> RecipeBook {
        let mut book = RecipeBook::default();
        book.tags.insert(
            "minecraft:dyes".into(),
            DYE_COLORS.iter().map(|(name, _)| format!("minecraft:{name}_dye")).collect(),
        );
        book.tags.insert(
            "minecraft:skulls".into(),
            vec!["minecraft:creeper_head".into(), "minecraft:zombie_head".into()],
        );
        for (id, recipe) in [
            ("minecraft:firework_rocket", serde_json::json!({
                "type": "minecraft:crafting_special_firework_rocket", "shell": "minecraft:paper",
                "fuel": "minecraft:gunpowder", "star": "minecraft:firework_star",
                "result": {"id": "minecraft:firework_rocket", "count": 3}
            })),
            ("minecraft:firework_rocket_simple", serde_json::json!({
                "type": "minecraft:crafting_shapeless", "ingredients": ["minecraft:gunpowder", "minecraft:paper"],
                "result": {"id": "minecraft:firework_rocket", "count": 3}
            })),
            ("minecraft:firework_star", serde_json::json!({
                "type": "minecraft:crafting_special_firework_star", "dye": "#minecraft:dyes", "fuel": "minecraft:gunpowder",
                "shapes": {"burst": "minecraft:feather", "creeper": "#minecraft:skulls", "large_ball": "minecraft:fire_charge", "star": "minecraft:gold_nugget"},
                "trail": "minecraft:diamond", "twinkle": "minecraft:glowstone_dust", "result": {"id": "minecraft:firework_star"}
            })),
            ("minecraft:firework_star_fade", serde_json::json!({
                "type": "minecraft:crafting_special_firework_star_fade", "dye": "#minecraft:dyes",
                "target": "minecraft:firework_star", "result": {"id": "minecraft:firework_star"}
            })),
        ] {
            let mut recipe = parse_recipe(&recipe).unwrap();
            recipe.id = id.to_owned();
            book.recipes.push(recipe);
        }
        book
    }

    fn item(id: &str) -> ItemStack {
        ItemStack::new(format!("minecraft:{id}"), 1)
    }

    fn with(mut stack: ItemStack, components: Value) -> ItemStack {
        stack.components = Some(components);
        stack
    }

    fn table(items: impl IntoIterator<Item = ItemStack>) -> Vec<Option<ItemStack>> {
        let mut grid = items.into_iter().map(Some).collect::<Vec<_>>();
        grid.resize(9, None);
        grid
    }

    fn crafted(book: &RecipeBook, ids: &[&str]) -> Option<ItemStack> {
        book.matching(&table(ids.iter().map(|id| item(id))), 3, 3)
    }

    fn star(explosion: Value) -> ItemStack {
        with(item("firework_star"), serde_json::json!({"minecraft:firework_explosion": explosion}))
    }

    #[test]
    fn firework_stars_take_their_shape_colours_trail_and_twinkle_from_the_grid() {
        let book = firework_book();
        let red = crafted(&book, &["gunpowder", "red_dye", "fire_charge", "diamond"]).unwrap();
        assert_eq!((red.id.as_str(), red.count), ("minecraft:firework_star", 1));
        assert_eq!(
            red.components,
            Some(serde_json::json!({"minecraft:firework_explosion": {"shape": "large_ball", "colors": [11743532], "has_trail": 1}}))
        );
        let explosion = |ids: &[&str]| {
            crafted(&book, ids).and_then(|star| star.components).map(|patch| patch["minecraft:firework_explosion"].clone())
        };
        assert_eq!(
            explosion(&["blue_dye", "gunpowder", "red_dye", "creeper_head", "glowstone_dust"]),
            Some(serde_json::json!({"shape": "creeper", "colors": [2437522, 11743532], "has_twinkle": 1}))
        );
        assert_eq!(explosion(&["lime_dye", "gunpowder"]), Some(serde_json::json!({"shape": "small_ball", "colors": [4312372]})));
        assert_eq!(explosion(&["gunpowder", "white_dye", "white_dye", "feather"]), Some(serde_json::json!({"shape": "burst", "colors": [15790320, 15790320]})));
        assert_eq!(
            explosion(&["gold_nugget", "yellow_dye", "gunpowder", "diamond", "glowstone_dust"]),
            Some(serde_json::json!({"shape": "star", "colors": [14602026], "has_trail": 1, "has_twinkle": 1}))
        );
        for refused in [
            &["gunpowder"][..],
            &["red_dye", "fire_charge"],
            &["gunpowder", "fire_charge", "diamond"],
            &["gunpowder", "gunpowder", "red_dye"],
            &["gunpowder", "red_dye", "feather", "gold_nugget"],
            &["gunpowder", "red_dye", "diamond", "diamond"],
            &["gunpowder", "red_dye", "glowstone_dust", "glowstone_dust"],
            &["gunpowder", "red_dye", "stick"],
        ] {
            assert_eq!(crafted(&book, refused), None, "{refused:?}");
        }
        let lime = with(item("red_dye"), serde_json::json!({"minecraft:dye": "lime"}));
        let made = book.matching(&table([item("gunpowder"), lime]), 3, 3).unwrap();
        assert_eq!(made.components.unwrap()["minecraft:firework_explosion"]["colors"], serde_json::json!([4312372]));
        let undyed = with(item("red_dye"), serde_json::json!({"!minecraft:dye": {}}));
        assert_eq!(book.matching(&table([item("gunpowder"), undyed]), 3, 3), None);
    }

    #[test]
    fn firework_rockets_fly_one_level_per_gunpowder_with_their_stars_in_grid_order() {
        let book = firework_book();
        let yellow = serde_json::json!({"shape": "star", "colors": [14602026], "fade_colors": [2437522]});
        let lime = serde_json::json!({"shape": "creeper", "colors": [4312372], "has_twinkle": 1});
        let mut grid = table([]);
        grid[0] = Some(star(lime.clone()));
        grid[2] = Some(item("paper"));
        grid[4] = Some(item("gunpowder"));
        grid[7] = Some(star(yellow.clone()));
        grid[8] = Some(item("gunpowder"));
        let rockets = book.matching(&grid, 3, 3).unwrap();
        assert_eq!((rockets.id.as_str(), rockets.count), ("minecraft:firework_rocket", 3));
        assert_eq!(
            rockets.components,
            Some(serde_json::json!({"minecraft:fireworks": {"flight_duration": 2, "explosions": [lime, yellow]}}))
        );
        assert_eq!(book.matching_id(&grid, 3, 3), Some("minecraft:firework_rocket"));
        assert_eq!(
            crafted(&book, &["paper", "gunpowder", "gunpowder", "gunpowder"]).and_then(|rockets| rockets.components),
            Some(serde_json::json!({"minecraft:fireworks": {"flight_duration": 3}}))
        );
        let plain = crafted(&book, &["gunpowder", "paper"]).unwrap();
        assert_eq!((plain.id.as_str(), plain.count, plain.components), ("minecraft:firework_rocket", 3, None));
        let blank = book.matching(&table([item("paper"), item("gunpowder"), item("firework_star")]), 3, 3).unwrap();
        assert_eq!((blank.count, blank.components), (3, None));
        for refused in [
            &["paper", "gunpowder", "gunpowder", "gunpowder", "gunpowder"][..],
            &["paper"],
            &["paper", "paper", "gunpowder"],
            &["paper", "firework_star"],
            &["gunpowder", "firework_star"],
            &["paper", "gunpowder", "stick"],
        ] {
            assert_eq!(crafted(&book, refused), None, "{refused:?}");
        }
    }

    #[test]
    fn dyes_give_a_firework_star_its_fade_colours() {
        let book = firework_book();
        let mut red = star(serde_json::json!({"shape": "large_ball", "colors": [11743532], "has_trail": 1}));
        red.components.as_mut().unwrap()["minecraft:custom_name"] = serde_json::json!("Red");
        red.count = 5;
        let faded = book.matching(&table([red.clone(), item("blue_dye")]), 3, 3).unwrap();
        assert_eq!((faded.id.as_str(), faded.count), ("minecraft:firework_star", 1));
        assert_eq!(
            faded.components,
            Some(serde_json::json!({
                "minecraft:custom_name": "Red",
                "minecraft:firework_explosion": {"shape": "large_ball", "colors": [11743532], "fade_colors": [2437522], "has_trail": 1}
            }))
        );
        let twice = book.matching(&table([item("white_dye"), faded, item("black_dye")]), 3, 3).unwrap();
        assert_eq!(twice.components.unwrap()["minecraft:firework_explosion"]["fade_colors"], serde_json::json!([15790320, 1973019]));
        assert_eq!(
            crafted(&book, &["firework_star", "lime_dye"]).and_then(|star| star.components),
            Some(serde_json::json!({"minecraft:firework_explosion": {"shape": "small_ball", "fade_colors": [4312372]}}))
        );
        assert_eq!(book.matching(&table([red.clone()]), 3, 3), None);
        assert_eq!(book.matching(&table([red.clone(), red.clone(), item("blue_dye")]), 3, 3), None);
        assert_eq!(book.matching(&table([red, item("blue_dye"), item("stick")]), 3, 3), None);
    }

    #[test]
    fn special_recipes_stay_out_of_the_recipe_book_but_craft_from_the_grid() {
        let book = firework_book();
        let listed = book.crafting_recipes().map(|recipe| recipe.id.to_owned()).collect::<Vec<_>>();
        assert_eq!(listed, ["minecraft:firework_rocket_simple"]);
        assert!(book.crafting_grid_candidates("minecraft:firework_star", 3, 3).is_none());
        assert!(book.crafting_grid_candidates("minecraft:firework_rocket_simple", 3, 3).is_some());
        assert!(book.has_recipe("minecraft:firework_star_fade"));
        let book = book.with_extra([("fork:gun".to_owned(), serde_json::json!({
            "type": "minecraft:crafting_shapeless", "ingredients": ["minecraft:stick"], "result": {"id": "fork:gun"}
        }))]);
        assert_eq!(book.display_indices("fork:gun"), Some(&[1][..]));
        let mut inventory = Inventory::default().with_recipes(book);
        assert!(!inventory.unlock_recipe("minecraft:firework_rocket"));
        let red = serde_json::json!({"shape": "large_ball", "colors": [11743532], "has_trail": 1});
        inventory.workbench[0] = Some(ItemStack::new("minecraft:paper", 2));
        inventory.workbench[1] = Some(ItemStack::new("minecraft:gunpowder", 4));
        let mut stars = star(red.clone());
        stars.count = 2;
        inventory.workbench[3] = Some(stars);
        assert!(inventory.take_workbench_output(false));
        let rockets = inventory.cursor.clone().unwrap();
        assert_eq!(rockets.count, 3);
        assert_eq!(rockets.components, Some(serde_json::json!({"minecraft:fireworks": {"flight_duration": 1, "explosions": [red]}})));
        assert_eq!(
            inventory.workbench.iter().map(|cell| cell.as_ref().map_or(0, |stack| stack.count)).collect::<Vec<_>>(),
            [1, 3, 0, 1, 0, 0, 0, 0, 0]
        );
        assert!(inventory.unlocked_recipes.is_empty());
    }

    #[test]
    fn the_jar_firework_recipes_craft_stars_and_rockets() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = RecipeBook::from_jar(&jar).unwrap();
        assert!(book.crafting_recipes().all(|recipe| !recipe.id.starts_with("minecraft:firework_star")));
        let red = crafted(&book, &["gunpowder", "red_dye", "fire_charge", "diamond"]).unwrap();
        let explosion = serde_json::json!({"shape": "large_ball", "colors": [11743532], "has_trail": 1});
        assert_eq!(red.components, Some(serde_json::json!({"minecraft:firework_explosion": explosion.clone()})));
        assert_eq!(
            crafted(&book, &["gunpowder", "white_dye", "skeleton_skull"]).and_then(|star| star.components),
            Some(serde_json::json!({"minecraft:firework_explosion": {"shape": "creeper", "colors": [15790320]}}))
        );
        let rockets = book.matching(&table([item("paper"), item("gunpowder"), red.clone(), item("gunpowder")]), 3, 3).unwrap();
        assert_eq!(rockets.count, 3);
        assert_eq!(
            rockets.components,
            Some(serde_json::json!({"minecraft:fireworks": {"flight_duration": 2, "explosions": [explosion]}}))
        );
        let plain = crafted(&book, &["paper", "gunpowder"]).unwrap();
        assert_eq!((plain.id.as_str(), plain.count, plain.components), ("minecraft:firework_rocket", 3, None));
        let faded = book.matching(&table([red, item("blue_dye")]), 3, 3).unwrap();
        assert_eq!(faded.components.unwrap()["minecraft:firework_explosion"]["fade_colors"], serde_json::json!([2437522]));
        assert_eq!(crafted(&book, &["paper", "gunpowder", "gunpowder", "gunpowder", "gunpowder"]), None);
    }

    fn shield_book() -> RecipeBook {
        let mut book = RecipeBook::default();
        book.tags.insert(
            "minecraft:banners".into(),
            DYE_COLORS.iter().map(|(name, _)| format!("minecraft:{name}_banner")).collect(),
        );
        let mut recipe = parse_recipe(&serde_json::json!({
            "type": "minecraft:crafting_special_shielddecoration", "banner": "#minecraft:banners",
            "target": "minecraft:shield", "result": {"id": "minecraft:shield"}
        }))
        .unwrap();
        recipe.id = "minecraft:shield_decoration".to_owned();
        book.recipes.push(recipe);
        book
    }

    #[test]
    fn a_banner_and_an_unpatterned_shield_and_nothing_else_make_a_patterned_shield() {
        let book = shield_book();
        assert_eq!(book.crafting_recipes().count(), 0);
        assert!(book.crafting_grid_candidates("minecraft:shield_decoration", 3, 3).is_none());
        let black = crafted(&book, &["shield", "black_banner"]).unwrap();
        assert_eq!(
            (black.id.as_str(), black.count, black.components),
            ("minecraft:shield", 1, Some(serde_json::json!({"minecraft:base_color": "black"})))
        );
        let layers = serde_json::json!([{"pattern": "minecraft:creeper", "color": "black"}, {"pattern": "minecraft:border", "color": "red"}]);
        let white = with(item("white_banner"), serde_json::json!({"minecraft:banner_patterns": layers.clone()}));
        let mut grid = table([]);
        grid[4] = Some(item("shield"));
        grid[8] = Some(white.clone());
        assert_eq!(
            book.matching(&grid, 3, 3).and_then(|shield| shield.components),
            Some(serde_json::json!({"minecraft:base_color": "white", "minecraft:banner_patterns": layers.clone()}))
        );
        let emptied = with(item("shield"), serde_json::json!({"minecraft:banner_patterns": [], "minecraft:base_color": "red"}));
        assert_eq!(
            book.matching(&table([item("lime_banner"), emptied]), 3, 3).and_then(|shield| shield.components),
            Some(serde_json::json!({"minecraft:base_color": "lime"}))
        );
        let removed = with(item("shield"), serde_json::json!({"!minecraft:banner_patterns": {}}));
        assert_eq!(
            book.matching(&table([removed, white.clone()]), 3, 3).and_then(|shield| shield.components),
            Some(serde_json::json!({"minecraft:base_color": "white", "minecraft:banner_patterns": layers.clone()}))
        );
        let patterned = with(
            item("shield"),
            serde_json::json!({"minecraft:base_color": "red", "minecraft:banner_patterns": [{"pattern": "minecraft:cross", "color": "yellow"}]}),
        );
        assert_eq!(book.matching(&table([patterned, item("black_banner")]), 3, 3), None);
        for refused in [
            &["shield"][..],
            &["black_banner"],
            &["shield", "shield", "black_banner"],
            &["shield", "black_banner", "white_banner"],
            &["black_banner", "white_banner"],
            &["shield", "shield"],
            &["shield", "stick"],
            &["stick", "black_banner"],
            &["shield", "black_banner", "stick"],
        ] {
            assert_eq!(crafted(&book, refused), None, "{refused:?}");
        }
    }

    #[test]
    fn a_decorated_shield_keeps_its_name_damage_and_enchantments() {
        let book = shield_book();
        let worn = with(
            item("shield"),
            serde_json::json!({
                "minecraft:custom_name": "Wall", "minecraft:damage": 30,
                "minecraft:enchantments": {"minecraft:unbreaking": 3}, "minecraft:repair_cost": 1
            }),
        );
        let layers = serde_json::json!([{"pattern": "minecraft:creeper", "color": "black"}]);
        let banner = with(
            item("white_banner"),
            serde_json::json!({"minecraft:banner_patterns": layers.clone(), "minecraft:custom_name": "Flag"}),
        );
        let decorated = book.matching(&table([worn, banner]), 3, 3).unwrap();
        assert_eq!(
            decorated.components,
            Some(serde_json::json!({
                "minecraft:custom_name": "Wall", "minecraft:damage": 30,
                "minecraft:enchantments": {"minecraft:unbreaking": 3}, "minecraft:repair_cost": 1,
                "minecraft:banner_patterns": layers, "minecraft:base_color": "white"
            }))
        );
        let mut inventory = Inventory::default().with_recipes(book);
        inventory.crafting[1] = Some(item("black_banner"));
        inventory.crafting[2] = Some(item("shield"));
        assert!(inventory.take_crafting_output(false));
        assert_eq!(inventory.cursor.as_ref().and_then(|shield| shield.components.clone()), Some(serde_json::json!({"minecraft:base_color": "black"})));
        assert!(inventory.crafting.iter().all(Option::is_none));
    }

    #[test]
    fn the_jar_shield_decoration_takes_a_banner() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = RecipeBook::from_jar(&jar).unwrap();
        assert!(book.crafting_recipes().all(|recipe| recipe.id != "minecraft:shield_decoration"));
        let black = crafted(&book, &["black_banner", "shield"]).unwrap();
        assert_eq!((black.id.as_str(), black.components), ("minecraft:shield", Some(serde_json::json!({"minecraft:base_color": "black"}))));
        let layers = serde_json::json!([{"pattern": "minecraft:creeper", "color": "black"}]);
        let white = with(item("white_banner"), serde_json::json!({"minecraft:banner_patterns": layers.clone()}));
        assert_eq!(
            book.matching(&table([item("shield"), white]), 3, 3).and_then(|shield| shield.components),
            Some(serde_json::json!({"minecraft:base_color": "white", "minecraft:banner_patterns": layers}))
        );
        assert_eq!(crafted(&book, &["black_banner", "shield", "stick"]), None);
    }

    #[test]
    fn extra_recipes_get_display_indices_after_vanilla() {
        let mut book = RecipeBook::default();
        for (id, pattern) in [("minecraft:a", "AA"), ("minecraft:b", "BB")] {
            let mut recipe = parse_recipe(&serde_json::json!({
                "type": "minecraft:crafting_shaped",
                "key": {"A": "minecraft:stone", "B": "minecraft:dirt"},
                "pattern": [pattern],
                "result": {"id": id}
            }))
            .unwrap();
            recipe.id = id.to_owned();
            book.recipes.push(recipe);
        }
        book.display_indices.insert("minecraft:a".into(), vec![7]);
        book.display_indices.insert("minecraft:b".into(), vec![40]);
        let gun = serde_json::json!({
            "type": "minecraft:crafting_shaped", "category": "equipment", "group": "fork_guns",
            "key": {"I": "minecraft:iron_ingot", "G": "minecraft:gunpowder"},
            "pattern": ["IG"], "result": {"id": "fork:gun"}
        });
        let ammo = serde_json::json!({
            "type": "minecraft:crafting_shapeless", "category": "equipment",
            "ingredients": ["minecraft:gunpowder", "minecraft:copper_ingot"], "result": {"id": "fork:ammo", "count": 16}
        });
        let book = book.with_extra([
            ("fork:gun".to_owned(), gun),
            ("minecraft:a".to_owned(), ammo.clone()),
            ("fork:broken".to_owned(), serde_json::json!({"type": "minecraft:crafting_shaped"})),
            ("fork:ammo".to_owned(), ammo),
        ]);
        assert_eq!(book.count(), 4);
        assert_eq!(book.display_indices("fork:gun"), Some(&[41][..]));
        assert_eq!(book.display_indices("fork:ammo"), Some(&[42][..]));
        assert_eq!(book.display_indices("minecraft:a"), Some(&[7][..]));
        assert_eq!(book.display_indices("fork:broken"), None);
        let gun = book.crafting_recipes().find(|recipe| recipe.id == "fork:gun").unwrap();
        assert_eq!((gun.category, gun.group, gun.width, gun.height), ("equipment", Some("fork_guns"), 2, 1));
        let ammo = book.crafting_recipes().find(|recipe| recipe.id == "fork:ammo").unwrap();
        assert_eq!((ammo.category, ammo.group, ammo.result.count), ("equipment", None, 16));
        let grid = [
            Some(ItemStack::new("minecraft:gunpowder", 1)),
            Some(ItemStack::new("minecraft:iron_ingot", 1)),
            None,
            None,
        ];
        assert_eq!(book.matching(&grid, 2, 2).map(|stack| stack.id), Some("fork:gun".to_owned()));
        let bare = RecipeBook::default().with_extra([("fork:gun".to_owned(), serde_json::json!({
            "type": "minecraft:crafting_shapeless", "ingredients": ["minecraft:stick"], "result": {"id": "fork:gun"}
        }))]);
        assert_eq!(bare.display_indices("fork:gun"), Some(&[0][..]));
        let mut inventory = Inventory::default().with_recipes(book);
        inventory.notice_item_changed(&ItemStack::new("minecraft:gunpowder", 1));
        assert!(inventory.unlocked_recipes.is_empty());
    }

    #[test]
    fn every_recipe_of_the_jar_and_every_extra_recipe_is_known() {
        let book = RecipeBook::default().with_extra([("fork:gun".to_owned(), serde_json::json!({
            "type": "minecraft:crafting_shapeless", "ingredients": ["minecraft:stick"], "result": {"id": "fork:gun"}
        }))]);
        assert!(book.has_recipe("fork:gun"));
        assert!(!book.has_recipe("minecraft:oak_planks"));
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = RecipeBook::from_jar(&jar).unwrap();
        assert!(book.has_recipe("minecraft:oak_planks"));
        assert!(book.has_recipe("minecraft:iron_ingot_from_smelting_raw_iron"));
        assert!(book.has_recipe("minecraft:andesite_slab_from_andesite_stonecutting"));
        assert!(!book.has_recipe("minecraft:andesite_slab_from_andesite_stonecutting_x"));
        assert!(!book.has_recipe("minecraft:recipe/oak_planks"));
    }

    #[test]
    fn pinned_jar_furnace_recipe_matches_when_available() {
        let jar = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../harness/.gradle/loom-cache/minecraftMaven/net/minecraft/minecraft-common-1fad6b3808/26.3/minecraft-common-1fad6b3808-26.3.jar",
        );
        if !jar.exists() {
            return;
        }
        let book = RecipeBook::from_jar(&jar).unwrap();
        let mut grid: [Option<ItemStack>; 9] = std::array::from_fn(|_| None);
        for index in [0, 1, 2, 3, 5, 6, 7, 8] {
            grid[index] = Some(ItemStack::new("minecraft:cobblestone", 1));
        }
        assert_eq!(
            book.matching(&grid, 3, 3).map(|item| item.id),
            Some("minecraft:furnace".into())
        );
    }
}
