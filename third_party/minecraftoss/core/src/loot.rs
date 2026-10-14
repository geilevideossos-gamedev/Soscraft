//! Loot tables (26.3 `LootTable`, `LootPool`, the pool entries, the
//! condition and function types the vanilla data uses, and the int and float
//! context providers), read from the data pack.
//!
//! Tables with a `random_sequence` draw from that per-world named sequence
//! (`RandomSequences`), others from the caller's random. Constructs not
//! ported yet make the evaluation fail with a message instead of inventing
//! loot.

use crate::datapack::DataPack;
use crate::ident::Identifier;
use crate::item::ItemStack;
use crate::random::{LegacyRandom, RandomSource, XoroshiroRandom};
use crate::{BlockStateId, Registries};
use serde_json::Value as Json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub trait Enchanting: Send + Sync + std::fmt::Debug {
    fn enchant_with_levels(&self, random: &mut dyn RandomSource, item: &str, cost: i32, options: Option<&Json>) -> Vec<(String, i32)>;
    fn enchant_randomly(&self, random: &mut dyn RandomSource, item: &str, options: Option<&Json>, only_compatible: bool) -> Option<(String, i32)>;
}

/// What a loot evaluation knows (`LootParams`).
#[derive(Clone, Debug, Default)]
pub struct LootParams {
    pub origin: Option<[f64; 3]>,
    /// The block broken or dropped (`BLOCK_STATE`).
    pub block_state: Option<BlockStateId>,
    /// The tool (`TOOL`); `Some(empty)` for blocks broken without one.
    pub tool: Option<ItemStack>,
    /// `THIS_ENTITY` is present.
    pub this_entity: bool,
    /// `EXPLOSION_RADIUS`.
    pub explosion_radius: Option<f32>,
    pub luck: f32,
    pub enchanting: Option<Arc<dyn Enchanting>>,
    pub biome: Option<String>,
}

/// Parsed loot tables and predicates, loaded on first use.
pub struct LootTables {
    tables: Mutex<HashMap<String, Option<Json>>>,
    predicates: Mutex<HashMap<String, Option<Json>>>,
}

impl Default for LootTables {
    fn default() -> Self {
        Self { tables: Mutex::new(HashMap::new()), predicates: Mutex::new(HashMap::new()) }
    }
}

type Result<T> = std::result::Result<T, String>;

struct Context<'a> {
    registries: &'a Registries,
    tables: &'a LootTables,
    params: &'a LootParams,
    random: &'a mut dyn RandomSource,
    /// Tables being evaluated (`LootContext.visitedElements`).
    visiting: Vec<String>,
}

impl XoroshiroRandom {
    /// `RandomSequence(seed, key)`: the world seed, unmixed, xored with the
    /// key's MD5, then mixed.
    pub fn for_sequence(world_seed: i64, key: &str) -> Self {
        const SILVER: u64 = 0x6a09_e667_f3bc_c909;
        const GOLDEN: u64 = 0x9e37_79b9_7f4a_7c15;
        let lo = (world_seed as u64) ^ SILVER;
        let hi = lo.wrapping_add(GOLDEN);
        let digest = md5::compute(key.as_bytes());
        let hash_lo = u64::from_be_bytes(digest[0..8].try_into().expect("MD5 half"));
        let hash_hi = u64::from_be_bytes(digest[8..16].try_into().expect("MD5 half"));
        let mix = |mut z: u64| {
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        XoroshiroRandom::from_state(mix(lo ^ hash_lo), mix(hi ^ hash_hi))
    }
}

/// Per-world named random sequences (`RandomSequences`).
#[derive(Debug, Default)]
pub struct RandomSequences {
    pub world_seed: i64,
    sequences: HashMap<String, XoroshiroRandom>,
}

impl RandomSequences {
    pub fn new(world_seed: i64) -> Self {
        Self { world_seed, sequences: HashMap::new() }
    }

    pub fn get(&mut self, key: &str) -> &mut XoroshiroRandom {
        let seed = self.world_seed;
        self.sequences.entry(key.to_owned()).or_insert_with(|| XoroshiroRandom::for_sequence(seed, key))
    }
}

/// A chain with one more modifier inside it.
fn with_inner(modifier: Option<&Json>, chain: &[Json]) -> Vec<Json> {
    let mut out: Vec<Json> = modifier.cloned().into_iter().collect();
    out.extend(chain.iter().cloned());
    out
}

fn id_of(text: &str) -> String {
    if text.contains(':') { text.to_owned() } else { format!("minecraft:{text}") }
}

impl LootTables {
    fn load(map: &Mutex<HashMap<String, Option<Json>>>, pack: &DataPack, kind: &str, id: &str) -> Option<Json> {
        let mut map = map.lock().expect("loot cache");
        map.entry(id.to_owned())
            .or_insert_with(|| Identifier::parse(id).ok().and_then(|ident| pack.read_json(kind, &ident).ok()))
            .clone()
    }

    fn table(&self, registries: &Registries, id: &str) -> Option<Json> {
        Self::load(&self.tables, &registries.datapack, "loot_table", id)
    }

    fn predicate(&self, registries: &Registries, id: &str) -> Option<Json> {
        Self::load(&self.predicates, &registries.datapack, "predicate", id)
    }

    /// `LootTable.getRandomItems(params)`: the table's own sequence when it
    /// names one, else `random`, with stacks split to their size.
    pub fn roll(
        &self,
        registries: &Registries,
        id: &str,
        params: &LootParams,
        sequences: &mut RandomSequences,
        random: &mut dyn RandomSource,
    ) -> Result<Vec<ItemStack>> {
        let Some(table) = self.table(registries, id) else { return Ok(Vec::new()) };
        let random = match table.get("random_sequence").and_then(Json::as_str) {
            Some(sequence) => sequences.get(sequence) as &mut dyn RandomSource,
            None => random,
        };
        self.roll_with(registries, id, &table, params, random)
    }

    pub fn roll_seeded(
        &self,
        registries: &Registries,
        id: &str,
        params: &LootParams,
        seed: i64,
        sequences: &mut RandomSequences,
        random: &mut dyn RandomSource,
    ) -> Result<Vec<ItemStack>> {
        if seed == 0 {
            return self.roll(registries, id, params, sequences, random);
        }
        let Some(table) = self.table(registries, id) else { return Ok(Vec::new()) };
        self.roll_with(registries, id, &table, params, &mut LegacyRandom::new(seed))
    }

    fn roll_with(&self, registries: &Registries, id: &str, table: &Json, params: &LootParams, random: &mut dyn RandomSource) -> Result<Vec<ItemStack>> {
        let mut out = Vec::new();
        let mut context = Context { registries, tables: self, params, random, visiting: Vec::new() };
        context.table_items(id, table, &[], &mut out)?;
        // `createStackSplitter`.
        let mut split = Vec::new();
        for stack in out {
            let max = registries.items.max_stack(&stack.id);
            if stack.count < max {
                split.push(stack);
            } else {
                let mut count = stack.count;
                while count > 0 {
                    let mut part = stack.clone();
                    part.count = count.min(max);
                    count -= part.count;
                    split.push(part);
                }
            }
        }
        Ok(split)
    }

    /// A block's own loot table (`blocks/<name>`).
    pub fn block_drops(
        &self,
        registries: &Registries,
        state: BlockStateId,
        params: &LootParams,
        sequences: &mut RandomSequences,
        random: &mut dyn RandomSource,
    ) -> Result<Vec<ItemStack>> {
        let blocks = &registries.blocks;
        let name = blocks.block(blocks.block_of(state)).name.as_str().to_owned();
        let (namespace, path) = name.split_once(':').unwrap_or(("minecraft", &name));
        let mut params = params.clone();
        params.block_state = Some(state);
        let own = format!("{namespace}:blocks/{path}");
        let table = if self.table(registries, &own).is_none() && path.contains("wall_") {
            format!("{namespace}:blocks/{}", path.replacen("wall_", "", 1))
        } else {
            own
        };
        self.roll(registries, &table, &params, sequences, random)
    }
}

impl Context<'_> {
    /// `LootTable.getRandomItemsRaw`. `chain` holds the modifiers that
    /// decorate the output, innermost first; each stack runs through them as
    /// soon as it is made, as vanilla's decorated consumers do.
    fn table_items(&mut self, id: &str, table: &Json, chain: &[Json], out: &mut Vec<ItemStack>) -> Result<()> {
        if self.visiting.iter().any(|v| v == id) {
            return Ok(());
        }
        self.visiting.push(id.to_owned());
        let chain = with_inner(table.get("modifier"), chain);
        for pool in table.get("pools").and_then(Json::as_array).into_iter().flatten() {
            self.pool_items(pool, &chain, out)?;
        }
        self.visiting.pop();
        Ok(())
    }

    /// Runs a new stack through a modifier chain into the output.
    fn emit(&mut self, mut stack: ItemStack, chain: &[Json], out: &mut Vec<ItemStack>) -> Result<()> {
        for modifier in chain {
            stack = self.apply_modifier(Some(modifier), stack)?;
        }
        out.push(stack);
        Ok(())
    }

    /// `LootPool.addRandomItems`.
    fn pool_items(&mut self, pool: &Json, chain: &[Json], out: &mut Vec<ItemStack>) -> Result<()> {
        if !self.condition_opt(pool.get("condition"))? {
            return Ok(());
        }
        let rolls = self.int(pool.get("rolls").ok_or("pool without rolls")?)?;
        let bonus = match pool.get("bonus_rolls") {
            Some(b) => self.float(b)?,
            None => 0.0,
        };
        let count = rolls + (bonus * self.params.luck).floor() as i32;
        let chain = with_inner(pool.get("modifier"), chain);
        for _ in 0..count {
            self.add_random_item(pool, &chain, out)?;
        }
        Ok(())
    }

    /// `LootPool.addRandomItem`: expand the entries, then pick by weight.
    fn add_random_item(&mut self, pool: &Json, chain: &[Json], out: &mut Vec<ItemStack>) -> Result<()> {
        let mut valid: Vec<(Json, Vec<Json>, i32)> = Vec::new();
        for entry in pool.get("entries").and_then(Json::as_array).into_iter().flatten() {
            let mut expanded = Vec::new();
            self.expand(entry, &mut Vec::new(), &mut expanded)?;
            for (leaf, modifiers) in expanded {
                let weight = self.weight(&leaf);
                if weight > 0 {
                    valid.push((leaf, modifiers, weight));
                }
            }
        }
        let total: i32 = valid.iter().map(|v| v.2).sum();
        if total == 0 || valid.is_empty() {
            return Ok(());
        }
        let chosen = if valid.len() == 1 {
            0
        } else {
            let mut index = self.random.next_i32_bound(total);
            let mut chosen = valid.len() - 1;
            for (i, v) in valid.iter().enumerate() {
                index -= v.2;
                if index < 0 {
                    chosen = i;
                    break;
                }
            }
            chosen
        };
        let (leaf, modifiers, _) = valid.swap_remove(chosen);
        // The leaf's modifier, then its enclosing composites' from the
        // innermost out, then the pool's and the table's.
        let mut full: Vec<Json> = leaf.get("modifier").cloned().into_iter().collect();
        full.extend(modifiers.iter().rev().cloned());
        full.extend(chain.iter().cloned());
        self.create_items(&leaf, &full, out)
    }

    fn weight(&self, leaf: &Json) -> i32 {
        let weight = leaf.get("weight").and_then(Json::as_i64).unwrap_or(1) as f32;
        let quality = leaf.get("quality").and_then(Json::as_i64).unwrap_or(0) as f32;
        ((weight + quality * self.params.luck).floor() as i32).max(0)
    }

    /// `LootPoolEntryContainer.expand`: a condition gate, then the entry's
    /// own expansion; composite entries' modifiers wrap their children.
    fn expand(&mut self, entry: &Json, modifiers: &mut Vec<Json>, out: &mut Vec<(Json, Vec<Json>)>) -> Result<bool> {
        if !self.condition_opt(entry.get("condition"))? {
            return Ok(false);
        }
        let kind = entry.get("type").and_then(Json::as_str).unwrap_or("minecraft:item");
        match kind.trim_start_matches("minecraft:") {
            "item" | "empty" | "loot_table" | "dynamic" => {
                out.push((entry.clone(), modifiers.clone()));
                Ok(true)
            }
            "alternatives" | "group" | "sequence" => {
                let children: Vec<Json> = entry.get("children").and_then(Json::as_array).cloned().unwrap_or_default();
                let pushed = entry.get("modifier").cloned();
                if let Some(m) = &pushed {
                    modifiers.push(m.clone());
                }
                let result = match kind.trim_start_matches("minecraft:") {
                    "alternatives" => {
                        let mut any = false;
                        for child in &children {
                            if self.expand(child, modifiers, out)? {
                                any = true;
                                break;
                            }
                        }
                        any
                    }
                    "group" => {
                        for child in &children {
                            self.expand(child, modifiers, out)?;
                        }
                        true
                    }
                    _ => {
                        let mut all = true;
                        for child in &children {
                            if !self.expand(child, modifiers, out)? {
                                all = false;
                                break;
                            }
                        }
                        all
                    }
                };
                if pushed.is_some() {
                    modifiers.pop();
                }
                Ok(result)
            }
            other => Err(format!("loot entry type {other} is not supported")),
        }
    }

    /// `createItemStack` of a leaf entry.
    fn create_items(&mut self, leaf: &Json, chain: &[Json], out: &mut Vec<ItemStack>) -> Result<()> {
        let kind = leaf.get("type").and_then(Json::as_str).unwrap_or("minecraft:item");
        match kind.trim_start_matches("minecraft:") {
            "item" => {
                let name = leaf.get("name").and_then(Json::as_str).ok_or("item entry without name")?;
                self.emit(ItemStack::new(&id_of(name), 1), chain, out)
            }
            "empty" => Ok(()),
            "loot_table" => {
                let value = leaf.get("value").ok_or("loot_table entry without value")?;
                match value {
                    Json::String(id) => {
                        let id = id_of(id);
                        let table = self.tables.table(self.registries, &id).ok_or_else(|| format!("missing loot table {id}"))?;
                        self.table_items(&id, &table, chain, out)
                    }
                    inline => self.table_items("<inline>", &inline.clone(), chain, out),
                }
            }
            other => Err(format!("loot entry type {other} is not supported")),
        }
    }

    // ---- providers -------------------------------------------------------------

    /// A `ContextIntProvider`.
    fn int(&mut self, value: &Json) -> Result<i32> {
        if let Some(n) = value.as_f64() {
            return Ok(n as i32);
        }
        let kind = value.get("type").and_then(Json::as_str).unwrap_or("minecraft:constant");
        match kind.trim_start_matches("minecraft:") {
            "constant" => Ok(value.get("value").and_then(Json::as_f64).unwrap_or(0.0) as i32),
            "uniform" => {
                let min = self.int(value.get("min").ok_or("uniform without min")?)?;
                let max = self.int(value.get("max").ok_or("uniform without max")?)?;
                Ok(if min >= max { min } else { self.random.next_i32_bound(max - min + 1) + min })
            }
            "binomial" => {
                let n = self.int(value.get("n").ok_or("binomial without n")?)?;
                let p = self.float(value.get("p").ok_or("binomial without p")?)?;
                let mut result = 0;
                for _ in 0..n {
                    if self.random.next_f32() < p {
                        result += 1;
                    }
                }
                Ok(result)
            }
            other => Err(format!("int provider {other} is not supported")),
        }
    }

    /// A `ContextFloatProvider`.
    fn float(&mut self, value: &Json) -> Result<f32> {
        if let Some(n) = value.as_f64() {
            return Ok(n as f32);
        }
        let kind = value.get("type").and_then(Json::as_str).unwrap_or("minecraft:constant");
        match kind.trim_start_matches("minecraft:") {
            "constant" => Ok(value.get("value").and_then(Json::as_f64).unwrap_or(0.0) as f32),
            "uniform" => {
                let min = self.float(value.get("min").ok_or("uniform without min")?)?;
                let max = self.float(value.get("max").ok_or("uniform without max")?)?;
                Ok(if min >= max { min } else { self.random.next_f32() * (max - min) + min })
            }
            other => Err(format!("float provider {other} is not supported")),
        }
    }

    // ---- conditions ------------------------------------------------------------

    fn condition_opt(&mut self, condition: Option<&Json>) -> Result<bool> {
        match condition {
            None => Ok(true),
            Some(c) => self.condition(c),
        }
    }

    /// `LootItemCondition.test`.
    fn condition(&mut self, condition: &Json) -> Result<bool> {
        if let Some(reference) = condition.as_str() {
            let id = id_of(reference);
            let predicate = self.tables.predicate(self.registries, &id).ok_or_else(|| format!("missing predicate {id}"))?;
            return self.condition(&predicate);
        }
        let kind = condition.get("type").and_then(Json::as_str).ok_or("condition without type")?;
        match kind.trim_start_matches("minecraft:") {
            "survives_explosion" => match self.params.explosion_radius {
                Some(radius) => Ok(self.random.next_f32() <= 1.0 / radius),
                None => Ok(true),
            },
            "inverted" => Ok(!self.condition(condition.get("term").ok_or("inverted without term")?)?),
            "any_of" => {
                for term in condition.get("terms").and_then(Json::as_array).into_iter().flatten() {
                    if self.condition(term)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            "all_of" => {
                for term in condition.get("terms").and_then(Json::as_array).into_iter().flatten() {
                    if !self.condition(term)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            "random_chance" => {
                let chance = self.float(condition.get("chance").ok_or("random_chance without chance")?)?;
                Ok(self.random.next_f32() < chance)
            }
            "table_bonus" => {
                let enchantment = condition.get("enchantment").and_then(Json::as_str).unwrap_or_default();
                let level = self.tool_enchantment(enchantment);
                let chances: Vec<f32> = condition.get("chances").and_then(Json::as_array).into_iter().flatten().filter_map(Json::as_f64).map(|c| c as f32).collect();
                let chance = chances.get((level as usize).min(chances.len().saturating_sub(1))).copied().unwrap_or(0.0);
                Ok(self.random.next_f32() < chance)
            }
            "match_block" => Ok(self.match_block(condition)),
            "location_check" => self.location_check(condition),
            "match_tool" => {
                let Some(tool) = &self.params.tool else { return Ok(false) };
                match condition.get("predicate") {
                    None => Ok(true),
                    Some(predicate) => self.item_predicate(tool, predicate),
                }
            }
            "entity_properties" => {
                if !self.params.this_entity {
                    return Ok(false);
                }
                match condition.get("predicate") {
                    Some(Json::Object(map)) if map.is_empty() => Ok(true),
                    None => Ok(true),
                    _ => Err("entity property predicates are not supported".to_owned()),
                }
            }
            other => Err(format!("loot condition {other} is not supported")),
        }
    }

    /// `MatchBlock`: the block (id, list or tag) and state properties.
    fn match_block(&self, condition: &Json) -> bool {
        let Some(state) = self.params.block_state else { return false };
        let blocks = &self.registries.blocks;
        let name = blocks.block(blocks.block_of(state)).name.as_str();
        let block_ok = match condition.get("blocks") {
            None => true,
            Some(Json::String(s)) if s.starts_with('#') => {
                let tag = id_of(&s[1..]);
                self.registries.block_tags.id(&tag).is_some_and(|t| self.registries.block_in_tag(state, t))
            }
            Some(Json::String(s)) => id_of(s) == name,
            Some(Json::Array(list)) => list.iter().filter_map(Json::as_str).any(|s| id_of(s) == name),
            _ => false,
        };
        if !block_ok {
            return false;
        }
        for (key, want) in condition.get("state").and_then(Json::as_object).into_iter().flatten() {
            let Some(have) = blocks.property(state, key) else { return false };
            let ok = match want {
                Json::String(v) => have == v,
                Json::Bool(b) => have == if *b { "true" } else { "false" },
                Json::Number(n) => have == n.to_string(),
                Json::Object(range) => {
                    let value: i64 = match have.parse() {
                        Ok(v) => v,
                        Err(_) => return false,
                    };
                    range.get("min").and_then(Json::as_i64).is_none_or(|m| value >= m) && range.get("max").and_then(Json::as_i64).is_none_or(|m| value <= m)
                }
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        true
    }

    fn location_check(&self, condition: &Json) -> Result<bool> {
        let offset = ["offsetX", "offsetY", "offsetZ"].iter().any(|key| condition.get(*key).and_then(Json::as_i64).is_some_and(|v| v != 0));
        if offset {
            return Err("location_check with an offset is not supported".to_owned());
        }
        if self.params.origin.is_none() {
            return Ok(false);
        }
        let Some(predicate) = condition.get("predicate").and_then(Json::as_object) else { return Ok(true) };
        for (key, wanted) in predicate {
            match key.as_str() {
                "biomes" => {
                    let Some(biome) = &self.params.biome else { return Ok(false) };
                    let holds = match wanted {
                        Json::String(s) if s.starts_with('#') => {
                            let tag = id_of(&s[1..]);
                            let id = self.registries.biomes.id(biome);
                            match (id, self.registries.biome_tags.id(&tag)) {
                                (Some(id), Some(tag)) => self.registries.biome_in_tag(id, tag),
                                _ => false,
                            }
                        }
                        Json::String(s) => id_of(s) == *biome,
                        Json::Array(list) => list.iter().filter_map(Json::as_str).any(|s| id_of(s) == *biome),
                        _ => false,
                    };
                    if !holds {
                        return Ok(false);
                    }
                }
                other => return Err(format!("location predicate {other} is not supported")),
            }
        }
        Ok(true)
    }

    /// `ItemPredicate.test` for the parts vanilla loot uses.
    fn item_predicate(&self, stack: &ItemStack, predicate: &Json) -> Result<bool> {
        if stack.is_empty() && (predicate.get("items").is_some() || predicate.get("predicates").is_some()) {
            return Ok(false);
        }
        if let Some(items) = predicate.get("items") {
            let ok = match items {
                Json::String(s) if s.starts_with('#') => return Err("item tag predicates are not supported".to_owned()),
                Json::String(s) => id_of(s) == stack.id,
                Json::Array(list) => list.iter().filter_map(Json::as_str).any(|s| id_of(s) == stack.id),
                _ => false,
            };
            if !ok {
                return Ok(false);
            }
        }
        if let Some(predicates) = predicate.get("predicates").and_then(Json::as_object) {
            for (key, value) in predicates {
                match key.as_str() {
                    "minecraft:enchantments" => {
                        for wanted in value.as_array().into_iter().flatten() {
                            let enchantment = wanted.get("enchantments").and_then(Json::as_str).unwrap_or_default();
                            let level = self.tool_enchantment(enchantment);
                            let min = wanted.get("levels").and_then(|l| l.get("min")).and_then(Json::as_i64).unwrap_or(1);
                            if (level as i64) < min {
                                return Ok(false);
                            }
                        }
                    }
                    "minecraft:map_id" => {
                        if stack.components.as_ref().and_then(|c| c.get("minecraft:map_id")).is_none() {
                            return Ok(false);
                        }
                    }
                    other => return Err(format!("item sub-predicate {other} is not supported")),
                }
            }
        }
        Ok(true)
    }

    /// The tool's level of an enchantment (`minecraft:enchantments` component).
    fn tool_enchantment(&self, enchantment: &str) -> i32 {
        let Some(tool) = &self.params.tool else { return 0 };
        let id = id_of(enchantment);
        tool.components
            .as_ref()
            .and_then(|c| c.get("minecraft:enchantments"))
            .and_then(|e| e.get(&id))
            .and_then(crate::nbt::Tag::as_i64)
            .unwrap_or(0) as i32
    }

    // ---- functions ---------------------------------------------------------------

    /// `LootItemFunction.apply` for a `modifier` (one function or a list).
    fn apply_modifier(&mut self, modifier: Option<&Json>, mut stack: ItemStack) -> Result<ItemStack> {
        match modifier {
            None => Ok(stack),
            Some(Json::Array(list)) => {
                for function in list {
                    stack = self.apply_function(function, stack)?;
                }
                Ok(stack)
            }
            Some(function) => self.apply_function(function, stack),
        }
    }

    fn apply_function(&mut self, function: &Json, mut stack: ItemStack) -> Result<ItemStack> {
        if let Some(condition) = function.get("condition") {
            if !self.condition(condition)? {
                return Ok(stack);
            }
        }
        let kind = function.get("type").and_then(Json::as_str).ok_or("function without type")?;
        match kind.trim_start_matches("minecraft:") {
            "set_count" => {
                let add = function.get("add").and_then(Json::as_bool).unwrap_or(false);
                let base = if add { stack.count } else { 0 };
                stack.count = base + self.int(function.get("count").ok_or("set_count without count")?)?;
            }
            "explosion_decay" => {
                if let Some(radius) = self.params.explosion_radius {
                    let probability = 1.0 / radius;
                    let mut result = 0;
                    for _ in 0..stack.count {
                        if self.random.next_f32() <= probability {
                            result += 1;
                        }
                    }
                    stack.count = result;
                }
            }
            "apply_bonus" => {
                if self.params.tool.is_some() {
                    let enchantment = function.get("enchantment").and_then(Json::as_str).unwrap_or_default();
                    let level = self.tool_enchantment(enchantment);
                    let formula = function.get("formula").and_then(Json::as_str).unwrap_or_default();
                    stack.count = match formula.trim_start_matches("minecraft:") {
                        "ore_drops" => {
                            if level > 0 {
                                let bonus = (self.random.next_i32_bound(level + 2) - 1).max(0);
                                stack.count * (bonus + 1)
                            } else {
                                stack.count
                            }
                        }
                        "uniform_bonus_count" => {
                            let multiplier = function.get("parameters").and_then(|p| p.get("bonusMultiplier")).and_then(Json::as_i64).unwrap_or(1) as i32;
                            stack.count + self.random.next_i32_bound(multiplier * level + 1)
                        }
                        "binomial_with_bonus_count" => {
                            let parameters = function.get("parameters");
                            let extra = parameters.and_then(|p| p.get("extra")).and_then(Json::as_i64).unwrap_or(0) as i32;
                            let probability = parameters.and_then(|p| p.get("probability")).and_then(Json::as_f64).unwrap_or(0.0) as f32;
                            let mut count = stack.count;
                            for _ in 0..level + extra {
                                if self.random.next_f32() < probability {
                                    count += 1;
                                }
                            }
                            count
                        }
                        other => return Err(format!("bonus formula {other} is not supported")),
                    };
                }
            }
            "limit_count" => {
                let limit = function.get("limit").ok_or("limit_count without limit")?;
                let (min, max) = match limit {
                    Json::Number(n) => (n.as_i64(), n.as_i64()),
                    other => (other.get("min").and_then(Json::as_i64), other.get("max").and_then(Json::as_i64)),
                };
                if let Some(min) = min {
                    stack.count = stack.count.max(min as i32);
                }
                if let Some(max) = max {
                    stack.count = stack.count.min(max as i32);
                }
            }
            // Components copied from a block entity or state: the copies are
            // cosmetic for drops without either.
            "copy_components" | "copy_state" => {
                if self.params.block_state.is_none() {
                    return Err(format!("{kind} without a block"));
                }
            }
            "enchant_with_levels" => {
                let params = self.params;
                if let Some(enchanting) = &params.enchanting {
                    let cost = self.int(function.get("levels").ok_or("enchant_with_levels without levels")?)?;
                    let chosen = enchanting.enchant_with_levels(&mut *self.random, &stack.id, cost, function.get("options"));
                    stack = enchanted(stack, &chosen);
                }
            }
            "enchant_randomly" => {
                let params = self.params;
                if let Some(enchanting) = &params.enchanting {
                    let only_compatible = function.get("only_compatible").and_then(Json::as_bool).unwrap_or(true);
                    if let Some(chosen) = enchanting.enchant_randomly(&mut *self.random, &stack.id, function.get("options"), only_compatible) {
                        stack = enchanted(stack, &[chosen]);
                    }
                }
            }
            "set_damage" => {
                let max = match stack.components.as_ref().and_then(|c| c.get("minecraft:max_damage")).and_then(crate::nbt::Tag::as_i64) {
                    Some(max) => Some(max as i32),
                    None => self.registries.items.max_damage(&stack.id),
                };
                let unbreakable = stack.components.as_ref().and_then(|c| c.get("minecraft:unbreakable")).is_some();
                if let Some(max) = max.filter(|max| *max > 0 && !unbreakable) {
                    let current = stack.components.as_ref().and_then(|c| c.get("minecraft:damage")).and_then(crate::nbt::Tag::as_i64).unwrap_or(0) as i32;
                    let add = function.get("add").and_then(Json::as_bool).unwrap_or(false);
                    let kept = if add { 1.0 - current as f32 / max as f32 } else { 0.0 };
                    let fraction = 1.0 - (self.float(function.get("damage").ok_or("set_damage without damage")?)? + kept).clamp(0.0, 1.0);
                    let damage = ((fraction * max as f32).floor() as i32).clamp(0, max);
                    set_component(&mut stack, "minecraft:damage", (damage != 0).then_some(crate::nbt::Tag::Int(damage)));
                }
            }
            "set_potion" => {
                let potion = function.get("id").and_then(Json::as_str).ok_or("set_potion without id")?;
                let mut contents = match stack.components.as_ref().and_then(|c| c.get("minecraft:potion_contents")) {
                    Some(crate::nbt::Tag::Compound(map)) => map.clone(),
                    _ => Default::default(),
                };
                contents.insert("potion".to_owned(), crate::nbt::Tag::String(id_of(potion)));
                set_component(&mut stack, "minecraft:potion_contents", Some(crate::nbt::Tag::Compound(contents)));
            }
            "set_stew_effect" => {
                let effects = function.get("effects").and_then(Json::as_array).cloned().unwrap_or_default();
                if stack.id == "minecraft:suspicious_stew" && !effects.is_empty() {
                    let entry = &effects[self.random.next_i32_bound(effects.len() as i32) as usize];
                    let effect = id_of(entry.get("type").and_then(Json::as_str).ok_or("stew effect without type")?);
                    let mut duration = self.int(entry.get("duration").ok_or("stew effect without duration")?)?;
                    if !matches!(effect.as_str(), "minecraft:instant_health" | "minecraft:instant_damage" | "minecraft:saturation") {
                        duration *= 20;
                    }
                    let mut added = std::collections::BTreeMap::new();
                    added.insert("id".to_owned(), crate::nbt::Tag::String(effect));
                    if duration != 160 {
                        added.insert("duration".to_owned(), crate::nbt::Tag::Int(duration));
                    }
                    let mut list = match stack.components.as_ref().and_then(|c| c.get("minecraft:suspicious_stew_effects")) {
                        Some(crate::nbt::Tag::List(list)) => list.clone(),
                        _ => Vec::new(),
                    };
                    list.push(crate::nbt::Tag::Compound(added));
                    set_component(&mut stack, "minecraft:suspicious_stew_effects", Some(crate::nbt::Tag::List(list)));
                }
            }
            "set_ominous_bottle_amplifier" => {
                let amplifier = self.int(function.get("amplifier").ok_or("set_ominous_bottle_amplifier without amplifier")?)?.clamp(0, 4);
                set_component(&mut stack, "minecraft:ominous_bottle_amplifier", (amplifier != 0).then_some(crate::nbt::Tag::Int(amplifier)));
            }
            "set_enchantments" => {
                if stack.id == "minecraft:book" {
                    stack.id = "minecraft:enchanted_book".to_owned();
                }
                let key = if stack.id == "minecraft:enchanted_book" { "minecraft:stored_enchantments" } else { "minecraft:enchantments" };
                let mut map = match stack.components.as_ref().and_then(|c| c.get(key)) {
                    Some(crate::nbt::Tag::Compound(map)) => map.clone(),
                    _ => Default::default(),
                };
                let add = function.get("add").and_then(Json::as_bool).unwrap_or(false);
                for (id, level) in function.get("enchantments").and_then(Json::as_object).into_iter().flatten() {
                    let id = id_of(id);
                    let base = if add { map.get(&id).and_then(crate::nbt::Tag::as_i64).unwrap_or(0) as i32 } else { 0 };
                    let level = (base + self.int(level)?).clamp(0, 255);
                    if level > 0 {
                        map.insert(id, crate::nbt::Tag::Int(level));
                    } else {
                        map.remove(&id);
                    }
                }
                set_component(&mut stack, key, (!map.is_empty()).then_some(crate::nbt::Tag::Compound(map)));
            }
            "set_components" => {
                for (key, value) in function.get("components").and_then(Json::as_object).into_iter().flatten() {
                    match key.strip_prefix('!') {
                        Some(removed) => set_component(&mut stack, &id_of(removed), None),
                        None => set_component(&mut stack, &id_of(key), Some(json_tag(value))),
                    }
                }
            }
            "set_name" => {
                if let Some(name) = function.get("name") {
                    let target = function.get("target").and_then(Json::as_str).unwrap_or("custom_name");
                    let key = if target == "item_name" { "minecraft:item_name" } else { "minecraft:custom_name" };
                    set_component(&mut stack, key, Some(json_tag(name)));
                }
            }
            "set_instrument" => {
                let options = self.holder_set("tags/instrument", function.get("options").ok_or("set_instrument without options")?)?;
                if !options.is_empty() {
                    let instrument = options[self.random.next_i32_bound(options.len() as i32) as usize].clone();
                    set_component(&mut stack, "minecraft:instrument", Some(crate::nbt::Tag::String(instrument)));
                }
            }
            "filtered" => {
                let filter = function.get("item_filter").ok_or("filtered without item_filter")?;
                let branch = if self.item_predicate(&stack, filter)? { function.get("on_pass") } else { function.get("on_fail") };
                stack = self.apply_modifier(branch, stack)?;
            }
            "discard" => stack = ItemStack::empty(),
            other => return Err(format!("loot function {other} is not supported")),
        }
        Ok(stack)
    }

    fn holder_set(&self, tags: &str, value: &Json) -> Result<Vec<String>> {
        let mut out = Vec::new();
        match value {
            Json::String(s) if s.starts_with('#') => self.tag_members(tags, &id_of(&s[1..]), &mut out, 0)?,
            Json::String(s) => out.push(id_of(s)),
            Json::Array(list) => out.extend(list.iter().filter_map(Json::as_str).map(id_of)),
            _ => return Err("holder set is neither an id, a tag nor a list".to_owned()),
        }
        Ok(out)
    }

    fn tag_members(&self, tags: &str, tag: &str, out: &mut Vec<String>, depth: u32) -> Result<()> {
        if depth > 8 {
            return Err(format!("tag {tag} nests too deep"));
        }
        let ident = Identifier::parse(tag).map_err(|_| format!("bad tag {tag}"))?;
        let json = self.registries.datapack.read_json(tags, &ident)?;
        for value in json.get("values").and_then(Json::as_array).into_iter().flatten() {
            let Some(id) = value.as_str().or_else(|| value.get("id").and_then(Json::as_str)) else { continue };
            match id.strip_prefix('#') {
                Some(nested) => self.tag_members(tags, &id_of(nested), out, depth + 1)?,
                None => out.push(id_of(id)),
            }
        }
        Ok(())
    }
}

fn set_component(stack: &mut ItemStack, key: &str, value: Option<crate::nbt::Tag>) {
    use crate::nbt::Tag;
    match value {
        Some(value) => {
            if let Tag::Compound(map) = stack.components.get_or_insert_with(|| Tag::Compound(Default::default())) {
                map.insert(key.to_owned(), value);
            }
        }
        None => {
            if let Some(Tag::Compound(map)) = &mut stack.components {
                map.remove(key);
                if map.is_empty() {
                    stack.components = None;
                }
            }
        }
    }
}

fn json_tag(value: &Json) -> crate::nbt::Tag {
    use crate::nbt::Tag;
    match value {
        Json::Object(map) => Tag::Compound(map.iter().map(|(k, v)| (k.clone(), json_tag(v))).collect()),
        Json::Array(list) => Tag::List(list.iter().map(json_tag).collect()),
        Json::String(s) => Tag::String(s.clone()),
        Json::Bool(b) => Tag::Byte(i8::from(*b)),
        Json::Number(n) => match n.as_i64() {
            Some(v) => i32::try_from(v).map_or(Tag::Long(v), Tag::Int),
            None => Tag::Double(n.as_f64().unwrap_or(0.0)),
        },
        Json::Null => Tag::Compound(Default::default()),
    }
}

fn enchanted(mut stack: ItemStack, chosen: &[(String, i32)]) -> ItemStack {
    use crate::nbt::Tag;
    if stack.id == "minecraft:book" {
        stack = ItemStack::new("minecraft:enchanted_book", 1);
    }
    let key = if stack.id == "minecraft:enchanted_book" { "minecraft:stored_enchantments" } else { "minecraft:enchantments" };
    for (id, level) in chosen.iter().filter(|(_, level)| *level > 0) {
        let components = stack.components.get_or_insert_with(|| Tag::Compound(Default::default()));
        let Tag::Compound(components) = components else { continue };
        let map = components.entry(key.to_owned()).or_insert_with(|| Tag::Compound(Default::default()));
        let Tag::Compound(map) = map else { continue };
        let current = map.get(id).and_then(Tag::as_i64).unwrap_or(0) as i32;
        map.insert(id.clone(), Tag::Int(current.max((*level).min(255))));
    }
    stack
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::AnyRandom;
    use crate::registries::DataPaths;

    #[test]
    fn vanilla_block_tables_roll() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let mut sequences = RandomSequences::new(1);
        let mut random = AnyRandom::new(true, 5);
        let params = LootParams { tool: Some(ItemStack::empty()), ..LootParams::default() };
        for name in ["melon", "cobweb", "torch", "oak_leaves", "gravel", "wheat", "diamond_ore", "acacia_slab", "short_grass"] {
            let state = registries.blocks.parse_state(&format!("minecraft:{name}")).unwrap();
            let drops = registries.loot.block_drops(&registries, state, &params, &mut sequences, &mut random);
            assert!(drops.is_ok(), "{name}: {drops:?}");
        }
    }

    #[derive(Debug)]
    struct Fixed;

    impl Enchanting for Fixed {
        fn enchant_with_levels(&self, random: &mut dyn RandomSource, _item: &str, cost: i32, options: Option<&Json>) -> Vec<(String, i32)> {
            assert_eq!(options.and_then(Json::as_str), Some("#minecraft:on_random_loot"));
            random.next_i32_bound(2);
            vec![("minecraft:sharpness".to_owned(), cost / 10), ("minecraft:unbreaking".to_owned(), 3)]
        }

        fn enchant_randomly(&self, _random: &mut dyn RandomSource, item: &str, _options: Option<&Json>, only_compatible: bool) -> Option<(String, i32)> {
            (item == "minecraft:book" && only_compatible).then(|| ("minecraft:mending".to_owned(), 1))
        }
    }

    fn apply(registries: &Registries, params: &LootParams, function: Json, stack: ItemStack) -> Result<ItemStack> {
        let tables = LootTables::default();
        let mut random = LegacyRandom::new(9);
        let mut context = Context { registries, tables: &tables, params, random: &mut random, visiting: Vec::new() };
        context.apply_function(&function, stack)
    }

    #[test]
    fn the_enchanting_functions_write_vanilla_components() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let params = LootParams { enchanting: Some(Arc::new(Fixed)), ..LootParams::default() };
        let levels = serde_json::json!({"type": "minecraft:enchant_with_levels", "levels": 30, "options": "#minecraft:on_random_loot"});
        let sword = apply(&registries, &params, levels.clone(), ItemStack::new("minecraft:diamond_sword", 1)).unwrap();
        assert_eq!(sword.id, "minecraft:diamond_sword");
        let enchantments = sword.components.as_ref().and_then(|c| c.get("minecraft:enchantments")).unwrap();
        assert_eq!(enchantments.get("minecraft:sharpness").and_then(crate::nbt::Tag::as_i64), Some(3));
        assert_eq!(enchantments.get("minecraft:unbreaking").and_then(crate::nbt::Tag::as_i64), Some(3));
        let book = apply(&registries, &params, levels, ItemStack::new("minecraft:book", 4)).unwrap();
        assert_eq!((book.id.as_str(), book.count), ("minecraft:enchanted_book", 1));
        let stored = book.components.as_ref().and_then(|c| c.get("minecraft:stored_enchantments")).unwrap();
        assert_eq!(stored.get("minecraft:sharpness").and_then(crate::nbt::Tag::as_i64), Some(3));
        assert!(book.components.as_ref().and_then(|c| c.get("minecraft:enchantments")).is_none());
        let randomly = serde_json::json!({"type": "minecraft:enchant_randomly", "options": "#minecraft:on_random_loot"});
        let book = apply(&registries, &params, randomly.clone(), ItemStack::new("minecraft:book", 1)).unwrap();
        assert_eq!(book.id, "minecraft:enchanted_book");
        assert_eq!(book.components.as_ref().and_then(|c| c.get("minecraft:stored_enchantments")).and_then(|e| e.get("minecraft:mending")).and_then(crate::nbt::Tag::as_i64), Some(1));
        let stick = apply(&registries, &params, randomly, ItemStack::new("minecraft:stick", 2)).unwrap();
        assert_eq!((stick.id.as_str(), stick.count, stick.components.is_none()), ("minecraft:stick", 2, true), "nothing to choose leaves the stack alone");
    }

    #[test]
    fn unknown_functions_still_fail() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let params = LootParams::default();
        let map = serde_json::json!({"type": "minecraft:exploration_map", "destination": "#minecraft:on_treasure_maps"});
        assert!(apply(&registries, &params, map, ItemStack::new("minecraft:map", 1)).is_err());
        let levels = serde_json::json!({"type": "minecraft:enchant_with_levels", "levels": 30});
        let sword = apply(&registries, &params, levels, ItemStack::new("minecraft:iron_sword", 1)).unwrap();
        assert!(sword.components.is_none(), "without an engine the stack passes through");
    }

    fn apply_seeded(registries: &Registries, params: &LootParams, function: &Json, stack: ItemStack, seed: i64) -> Result<ItemStack> {
        let tables = LootTables::default();
        let mut random = LegacyRandom::new(seed.wrapping_mul(-7_046_029_254_386_353_131));
        let mut context = Context { registries, tables: &tables, params, random: &mut random, visiting: Vec::new() };
        context.apply_function(function, stack)
    }

    fn check(registries: &Registries, params: &LootParams, condition: Json) -> Result<bool> {
        let tables = LootTables::default();
        let mut random = LegacyRandom::new(3);
        let mut context = Context { registries, tables: &tables, params, random: &mut random, visiting: Vec::new() };
        context.condition(&condition)
    }

    fn component<'a>(stack: &'a ItemStack, key: &str) -> Option<&'a crate::nbt::Tag> {
        stack.components.as_ref().and_then(|c| c.get(key))
    }

    fn level(stack: &ItemStack, key: &str, enchantment: &str) -> Option<i64> {
        component(stack, key).and_then(|e| e.get(enchantment)).and_then(crate::nbt::Tag::as_i64)
    }

    #[test]
    fn set_damage_wears_an_iron_sword_by_its_max_damage() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let params = LootParams::default();
        assert_eq!(registries.items.max_damage("minecraft:iron_sword"), Some(250));
        let half = serde_json::json!({"type": "minecraft:set_damage", "damage": 0.5});
        let sword = apply(&registries, &params, half.clone(), ItemStack::new("minecraft:iron_sword", 1)).unwrap();
        assert_eq!(component(&sword, "minecraft:damage").and_then(crate::nbt::Tag::as_i64), Some(125));
        let more = serde_json::json!({"type": "minecraft:set_damage", "damage": 0.25, "add": true});
        let worn = apply(&registries, &params, more, sword.clone()).unwrap();
        assert_eq!(component(&worn, "minecraft:damage").and_then(crate::nbt::Tag::as_i64), Some(62), "a quarter more durability on a half-worn sword");
        let mended = apply(&registries, &params, serde_json::json!({"type": "minecraft:set_damage", "damage": 1.0}), sword).unwrap();
        assert!(mended.components.is_none(), "full durability is the default damage of 0");
        let ranged = serde_json::json!({"type": "minecraft:set_damage", "damage": {"type": "minecraft:uniform", "min": 0.8, "max": 1.0}});
        for seed in 0..20 {
            let sword = apply_seeded(&registries, &params, &ranged, ItemStack::new("minecraft:diamond_sword", 1), seed).unwrap();
            let damage = component(&sword, "minecraft:damage").and_then(crate::nbt::Tag::as_i64).unwrap_or(0);
            assert!((0..=312).contains(&damage), "{damage}");
        }
        let stick = apply(&registries, &params, half, ItemStack::new("minecraft:stick", 1)).unwrap();
        assert!(stick.components.is_none(), "an item without durability stays as it is");
    }

    #[test]
    fn set_ominous_bottle_amplifier_rolls_zero_to_four() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let params = LootParams::default();
        let function = serde_json::json!({"type": "minecraft:set_ominous_bottle_amplifier", "amplifier": {"type": "minecraft:uniform", "min": 0, "max": 4}});
        let mut seen = [false; 5];
        for seed in 0..200 {
            let bottle = apply_seeded(&registries, &params, &function, ItemStack::new("minecraft:ominous_bottle", 1), seed).unwrap();
            let amplifier = component(&bottle, "minecraft:ominous_bottle_amplifier").and_then(crate::nbt::Tag::as_i64).unwrap_or(0);
            assert!((0..=4).contains(&amplifier));
            if amplifier == 0 {
                assert!(bottle.components.is_none(), "level 0 is the bottle's default");
            }
            seen[amplifier as usize] = true;
        }
        assert_eq!(seen, [true; 5]);
        let high = serde_json::json!({"type": "minecraft:set_ominous_bottle_amplifier", "amplifier": 9});
        let bottle = apply(&registries, &params, high, ItemStack::new("minecraft:ominous_bottle", 1)).unwrap();
        assert_eq!(component(&bottle, "minecraft:ominous_bottle_amplifier").and_then(crate::nbt::Tag::as_i64), Some(4));
    }

    #[test]
    fn set_enchantments_sets_or_adds_levels_and_stores_them_on_books() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let params = LootParams::default();
        let set = serde_json::json!({"type": "minecraft:set_enchantments", "enchantments": {"minecraft:sharpness": 3, "knockback": 2}});
        let sword = apply(&registries, &params, set.clone(), ItemStack::new("minecraft:iron_sword", 1)).unwrap();
        assert_eq!(level(&sword, "minecraft:enchantments", "minecraft:sharpness"), Some(3));
        assert_eq!(level(&sword, "minecraft:enchantments", "minecraft:knockback"), Some(2));
        let add = serde_json::json!({"type": "minecraft:set_enchantments", "enchantments": {"minecraft:sharpness": 2}, "add": true});
        let sharper = apply(&registries, &params, add, sword.clone()).unwrap();
        assert_eq!(level(&sharper, "minecraft:enchantments", "minecraft:sharpness"), Some(5));
        let reset = serde_json::json!({"type": "minecraft:set_enchantments", "enchantments": {"minecraft:sharpness": 1}});
        let blunter = apply(&registries, &params, reset, sharper).unwrap();
        assert_eq!(level(&blunter, "minecraft:enchantments", "minecraft:sharpness"), Some(1));
        assert_eq!(level(&blunter, "minecraft:enchantments", "minecraft:knockback"), Some(2), "unnamed enchantments stay");
        let clear = serde_json::json!({"type": "minecraft:set_enchantments", "enchantments": {"minecraft:sharpness": 0, "minecraft:knockback": 0}});
        assert!(apply(&registries, &params, clear, sword).unwrap().components.is_none());
        let book = apply(&registries, &params, set, ItemStack::new("minecraft:book", 2)).unwrap();
        assert_eq!((book.id.as_str(), book.count), ("minecraft:enchanted_book", 2));
        assert_eq!(level(&book, "minecraft:stored_enchantments", "minecraft:sharpness"), Some(3));
        assert!(component(&book, "minecraft:enchantments").is_none());
    }

    #[test]
    fn set_components_sets_and_removes_components() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let params = LootParams::default();
        let worn = apply(&registries, &params, serde_json::json!({"type": "minecraft:set_damage", "damage": 0.5}), ItemStack::new("minecraft:iron_chestplate", 1)).unwrap();
        assert!(component(&worn, "minecraft:damage").is_some());
        let trim = serde_json::json!({"type": "minecraft:set_components", "components": {"minecraft:trim": {"material": "minecraft:copper", "pattern": "minecraft:flow"}, "!minecraft:damage": {}}});
        let plate = apply(&registries, &params, trim, worn).unwrap();
        let trim = component(&plate, "minecraft:trim").unwrap();
        assert_eq!(trim.get("material").and_then(crate::nbt::Tag::as_str), Some("minecraft:copper"));
        assert_eq!(trim.get("pattern").and_then(crate::nbt::Tag::as_str), Some("minecraft:flow"));
        assert!(component(&plate, "minecraft:damage").is_none());
    }

    #[test]
    fn potions_stews_names_horns_and_filters_write_vanilla_components() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let params = LootParams::default();
        let potion = apply(&registries, &params, serde_json::json!({"type": "minecraft:set_potion", "id": "minecraft:strong_regeneration"}), ItemStack::new("minecraft:potion", 1)).unwrap();
        assert_eq!(component(&potion, "minecraft:potion_contents").and_then(|c| c.get("potion")).and_then(crate::nbt::Tag::as_str), Some("minecraft:strong_regeneration"));
        let stew = serde_json::json!({"type": "minecraft:set_stew_effect", "effects": [{"type": "minecraft:night_vision", "duration": 7}]});
        let saturation = serde_json::json!({"type": "minecraft:set_stew_effect", "effects": [{"type": "minecraft:saturation", "duration": 7}]});
        let bowl = apply(&registries, &params, stew.clone(), ItemStack::new("minecraft:suspicious_stew", 1)).unwrap();
        let bowl = apply(&registries, &params, saturation, bowl).unwrap();
        let effects = component(&bowl, "minecraft:suspicious_stew_effects").and_then(crate::nbt::Tag::as_list).unwrap();
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0].get("id").and_then(crate::nbt::Tag::as_str), Some("minecraft:night_vision"));
        assert_eq!(effects[0].get("duration").and_then(crate::nbt::Tag::as_i64), Some(140));
        assert_eq!(effects[1].get("duration").and_then(crate::nbt::Tag::as_i64), Some(7), "instant effects keep their ticks");
        let eight = serde_json::json!({"type": "minecraft:set_stew_effect", "effects": [{"type": "minecraft:night_vision", "duration": 8}]});
        let plain = apply(&registries, &params, eight, ItemStack::new("minecraft:suspicious_stew", 1)).unwrap();
        assert!(component(&plain, "minecraft:suspicious_stew_effects").and_then(crate::nbt::Tag::as_list).unwrap()[0].get("duration").is_none(), "160 ticks is the default");
        assert!(apply(&registries, &params, stew, ItemStack::new("minecraft:bowl", 1)).unwrap().components.is_none());
        let name = serde_json::json!({"type": "minecraft:set_name", "name": {"translate": "filled_map.swamp_camp_map"}, "target": "item_name"});
        let named = apply(&registries, &params, name, ItemStack::new("minecraft:map", 1)).unwrap();
        assert_eq!(component(&named, "minecraft:item_name").and_then(|n| n.get("translate")).and_then(crate::nbt::Tag::as_str), Some("filled_map.swamp_camp_map"));
        let custom = apply(&registries, &params, serde_json::json!({"type": "minecraft:set_name", "name": "Loot"}), ItemStack::new("minecraft:map", 1)).unwrap();
        assert_eq!(component(&custom, "minecraft:custom_name").and_then(crate::nbt::Tag::as_str), Some("Loot"));
        let horns = serde_json::json!({"type": "minecraft:set_instrument", "options": "#minecraft:regular_goat_horns"});
        let regular = ["minecraft:ponder_goat_horn", "minecraft:sing_goat_horn", "minecraft:seek_goat_horn", "minecraft:feel_goat_horn"];
        let mut seen = std::collections::HashSet::new();
        for seed in 0..40 {
            let horn = apply_seeded(&registries, &params, &horns, ItemStack::new("minecraft:goat_horn", 1), seed).unwrap();
            let instrument = component(&horn, "minecraft:instrument").and_then(crate::nbt::Tag::as_str).unwrap().to_owned();
            assert!(regular.contains(&instrument.as_str()), "{instrument}");
            seen.insert(instrument);
        }
        assert_eq!(seen.len(), 4);
        let filtered = serde_json::json!({"type": "minecraft:filtered", "item_filter": {"predicates": {"minecraft:map_id": {}}}, "on_fail": {"type": "minecraft:discard"}});
        assert!(apply(&registries, &params, filtered.clone(), ItemStack::new("minecraft:map", 1)).unwrap().is_empty());
        let mut drawn = ItemStack::new("minecraft:filled_map", 1);
        set_component(&mut drawn, "minecraft:map_id", Some(crate::nbt::Tag::Int(4)));
        assert_eq!(apply(&registries, &params, filtered, drawn.clone()).unwrap(), drawn);
    }

    #[test]
    fn location_checks_read_the_biome_at_the_origin() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let swamp = LootParams { origin: Some([0.5, 64.5, 0.5]), biome: Some("minecraft:swamp".to_owned()), ..LootParams::default() };
        let check_biomes = |params: &LootParams, biomes: Json| check(&registries, params, serde_json::json!({"type": "minecraft:location_check", "predicate": {"biomes": biomes}})).unwrap();
        assert!(check_biomes(&swamp, serde_json::json!("minecraft:swamp")));
        assert!(!check_biomes(&swamp, serde_json::json!("minecraft:sulfur_caves")));
        assert!(check_biomes(&swamp, serde_json::json!(["minecraft:jungle", "swamp"])));
        assert!(check_biomes(&swamp, serde_json::json!("#minecraft:is_overworld")));
        assert!(!check_biomes(&swamp, serde_json::json!("#minecraft:is_nether")));
        let nowhere = LootParams { biome: Some("minecraft:swamp".to_owned()), ..LootParams::default() };
        assert!(!check_biomes(&nowhere, serde_json::json!("minecraft:swamp")), "no origin, no location");
        let structure = serde_json::json!({"type": "minecraft:location_check", "predicate": {"structures": "minecraft:mansion"}});
        assert!(check(&registries, &swamp, structure).is_err());
    }

    #[derive(Debug)]
    struct Any;

    impl Enchanting for Any {
        fn enchant_with_levels(&self, random: &mut dyn RandomSource, _item: &str, cost: i32, _options: Option<&Json>) -> Vec<(String, i32)> {
            random.next_i32_bound(2);
            vec![("minecraft:unbreaking".to_owned(), (cost / 10).max(1))]
        }

        fn enchant_randomly(&self, random: &mut dyn RandomSource, _item: &str, _options: Option<&Json>, _only_compatible: bool) -> Option<(String, i32)> {
            random.next_i32_bound(2);
            Some(("minecraft:mending".to_owned(), 1))
        }
    }

    fn functions_in(value: &Json, out: &mut Vec<Json>) {
        match value {
            Json::Object(map) => {
                for (key, inner) in map {
                    if matches!(key.as_str(), "modifier" | "functions" | "on_pass" | "on_fail") {
                        match inner {
                            Json::Array(list) => out.extend(list.iter().cloned()),
                            one => out.push(one.clone()),
                        }
                    }
                    functions_in(inner, out);
                }
            }
            Json::Array(list) => list.iter().for_each(|inner| functions_in(inner, out)),
            _ => {}
        }
    }

    fn json_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                json_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "json") {
                out.push(path);
            }
        }
    }

    #[test]
    fn every_chest_spawner_equipment_and_gameplay_table_rolls_without_an_unsupported_function() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let params = LootParams {
            origin: Some([0.5, 64.5, 0.5]),
            this_entity: true,
            tool: Some(ItemStack::new("minecraft:fishing_rod", 1)),
            enchanting: Some(Arc::new(Any)),
            biome: Some("minecraft:plains".to_owned()),
            ..LootParams::default()
        };
        let root = paths.datapack.join("data/minecraft/loot_table");
        let mut tables = 0;
        for folder in ["chests", "spawners", "equipment", "gameplay"] {
            let mut files = Vec::new();
            json_files(&root.join(folder), &mut files);
            for file in files {
                let relative = file.strip_prefix(&root).unwrap().with_extension("");
                let id = format!("minecraft:{}", relative.to_string_lossy().replace('\\', "/"));
                let table: Json = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
                let mut functions = Vec::new();
                functions_in(&table, &mut functions);
                let kind = |f: &Json| f.get("type").and_then(Json::as_str).unwrap_or_default().to_owned();
                for function in &functions {
                    for item in ["minecraft:iron_sword", "minecraft:book", "minecraft:suspicious_stew", "minecraft:map"] {
                        if let Err(error) = apply(&registries, &params, function.clone(), ItemStack::new(item, 1)) {
                            assert!(!error.starts_with("loot function") || kind(function) == "minecraft:exploration_map", "{id}: {error}");
                        }
                    }
                }
                let maps = functions.iter().any(|f| kind(f) == "minecraft:exploration_map");
                for seed in 0..8 {
                    let mut sequences = RandomSequences::new(seed);
                    let mut random = LegacyRandom::new(seed);
                    match registries.loot.roll(&registries, &id, &params, &mut sequences, &mut random) {
                        Ok(_) => {}
                        Err(error) if maps && error.contains("exploration_map") => {}
                        Err(error) if folder == "gameplay" && !error.starts_with("loot function") => {}
                        Err(error) => panic!("{id}: {error}"),
                    }
                }
                tables += 1;
            }
        }
        assert!(tables > 90, "{tables} tables");
    }
}
