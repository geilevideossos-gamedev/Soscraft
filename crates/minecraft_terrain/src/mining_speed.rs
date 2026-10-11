use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use minecraftoss_player::Block;
use minecraftoss_player::inventory::ItemStack;
use serde_json::Value;

use crate::class_file::{self, ClassFile, Instruction};
use crate::transform::BlockFacts;

pub const BLOCKS_CLASS: &str = "net/minecraft/world/level/block/Blocks";
pub const BLOCK_ITEM_IDS_CLASS: &str = "net/minecraft/references/BlockItemIds";
pub const BLOCK_IDS_CLASS: &str = "net/minecraft/references/BlockIds";
const PROPERTIES_CLASS: &str = "net/minecraft/world/level/block/state/BlockBehaviour$Properties";
const RETURNS_PROPERTIES: &str =
    ")Lnet/minecraft/world/level/block/state/BlockBehaviour$Properties;";
const REQUIRES_TOOL: &str = "requiresCorrectToolForDrops";
const BLOCK: &str = "Lnet/minecraft/world/level/block/Block;";
const COLORS: &str = "Lnet/minecraft/world/level/block/ColorCollection;";
const COPPERS: &str = "Lnet/minecraft/world/level/block/WeatheringCopperCollection;";
const BY_STATE: &str = "Lnet/minecraft/world/level/block/WeatheringCopperCollection$ByState;";
const BLOCK_TYPES: [&str; 3] = [BLOCK, COLORS, COPPERS];
const COPIES: [&str; 6] = [
    "ofFullCopy",
    "ofLegacyCopy",
    "registerStair",
    "registerLegacyStair",
    "registerSlab",
    "registerWall",
];
const REF_INVOKE_STATIC: u8 = 6;
pub const DYES: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];
pub const WEATHER_PREFIXES: [&str; 4] = ["", "exposed_", "weathered_", "oxidized_"];
const WAXED: &str = "waxed_";
const ENCHANTMENT_FOLDER: &str = "data/minecraft/enchantment/";
const ENCHANTMENTS: &str = "minecraft:enchantments";
const ATTRIBUTES: &str = "minecraft:attributes";
const TOOL: &str = "minecraft:tool";
const NO_TOOL: &str = "!minecraft:tool";
pub const MINING_EFFICIENCY: &str = "minecraft:mining_efficiency";
pub const SUBMERGED_MINING_SPEED: &str = "minecraft:submerged_mining_speed";
pub const BLOCK_BREAK_SPEED: &str = "minecraft:block_break_speed";
const MINING_ATTRIBUTES: [&str; 3] = [MINING_EFFICIENCY, SUBMERGED_MINING_SPEED, BLOCK_BREAK_SPEED];
pub const MAINHAND: &str = "mainhand";
pub const OFFHAND: &str = "offhand";
pub const ARMOR_SLOTS: [&str; 4] = ["feet", "legs", "chest", "head"];
const MATERIALS: [(&str, f32, &str); 7] = [
    ("wooden", 2.0, "minecraft:incorrect_for_wooden_tool"),
    ("stone", 4.0, "minecraft:incorrect_for_stone_tool"),
    ("copper", 5.0, "minecraft:incorrect_for_copper_tool"),
    ("iron", 6.0, "minecraft:incorrect_for_iron_tool"),
    ("diamond", 8.0, "minecraft:incorrect_for_diamond_tool"),
    ("golden", 12.0, "minecraft:incorrect_for_gold_tool"),
    ("netherite", 9.0, "minecraft:incorrect_for_netherite_tool"),
];
const DIGGERS: [&str; 4] = ["pickaxe", "axe", "shovel", "hoe"];
const COBWEB: &str = "minecraft:cobweb";
const SWORD_INSTANTLY_MINES: &str = "minecraft:sword_instantly_mines";
const SWORD_EFFICIENT: &str = "minecraft:sword_efficient";
const SHEARS_EXTREME: &str = "minecraft:shears_extreme_breaking_speed";
const SHEARS_MAJOR: &str = "minecraft:shears_major_breaking_speed";
const SHEARS_MINOR: &str = "minecraft:shears_minor_breaking_speed";
const NORMAL_DIVISOR: f32 = 30.0;
const WRONG_TOOL_DIVISOR: f32 = 100.0;
const UNBREAKABLE: f32 = -1.0;

#[derive(Clone, Debug, PartialEq)]
enum LevelValue {
    Constant(f32),
    Linear {
        base: f32,
        per_level_above_first: f32,
    },
    LevelsSquared {
        added: f32,
    },
    Clamped {
        value: Box<LevelValue>,
        min: f32,
        max: f32,
    },
}

impl LevelValue {
    fn parse(value: &Value) -> Option<Self> {
        if let Some(constant) = value.as_f64() {
            return Some(Self::Constant(constant as f32));
        }
        let number = |key: &str| value[key].as_f64().map(|n| n as f32);
        Some(match value["type"].as_str()? {
            "minecraft:constant" => Self::Constant(number("value")?),
            "minecraft:linear" => Self::Linear {
                base: number("base")?,
                per_level_above_first: number("per_level_above_first")?,
            },
            "minecraft:levels_squared" => Self::LevelsSquared {
                added: number("added")?,
            },
            "minecraft:clamped" => Self::Clamped {
                value: Box::new(Self::parse(&value["value"])?),
                min: number("min")?,
                max: number("max")?,
            },
            _ => return None,
        })
    }

    fn at(&self, level: i32) -> f32 {
        match self {
            Self::Constant(value) => *value,
            Self::Linear {
                base,
                per_level_above_first,
            } => base + per_level_above_first * (level - 1) as f32,
            Self::LevelsSquared { added } => level.wrapping_mul(level) as f32 + added,
            Self::Clamped { value, min, max } => value.at(level).clamp(*min, *max),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    AddValue,
    AddMultipliedBase,
    AddMultipliedTotal,
}

#[derive(Clone, Debug, PartialEq)]
struct AttributeEffect {
    attribute: String,
    operation: Operation,
    amount: LevelValue,
}

impl AttributeEffect {
    fn parse(value: &Value) -> Option<Self> {
        let attribute = value["attribute"].as_str()?;
        if !MINING_ATTRIBUTES.contains(&attribute) {
            return None;
        }
        let operation = match value["operation"].as_str()? {
            "add_value" => Operation::AddValue,
            "add_multiplied_base" => Operation::AddMultipliedBase,
            "add_multiplied_total" => Operation::AddMultipliedTotal,
            _ => return None,
        };
        Some(Self {
            attribute: attribute.to_owned(),
            operation,
            amount: LevelValue::parse(&value["amount"])?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct MiningBook {
    requires_tool: HashSet<String>,
    warnings: Vec<String>,
    attributes: HashMap<String, Vec<AttributeEffect>>,
    slots: HashMap<String, Vec<String>>,
}

impl MiningBook {
    pub fn from_jar(jar: &Path) -> Result<Self> {
        let file =
            std::fs::File::open(jar).with_context(|| format!("cannot open {}", jar.display()))?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut enchantments = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let Some(path) = entry
                .name()
                .strip_prefix(ENCHANTMENT_FOLDER)
                .and_then(|name| name.strip_suffix(".json"))
                .map(str::to_owned)
            else {
                continue;
            };
            let mut text = String::new();
            entry.read_to_string(&mut text)?;
            enchantments.push((format!("minecraft:{path}"), serde_json::from_str(&text)?));
        }
        let mut class = |name: &str| -> Result<ClassFile> {
            let mut entry = archive
                .by_name(&format!("{name}.class"))
                .with_context(|| format!("no {name} in the jar"))?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            ClassFile::parse(&bytes)
        };
        let mut book = Self::default().with_enchantments(enchantments);
        let scan = (|| -> Result<(HashSet<String>, Vec<String>)> {
            let blocks = class(BLOCKS_CLASS)?;
            let ids = [class(BLOCK_ITEM_IDS_CLASS)?, class(BLOCK_IDS_CLASS)?];
            tool_blocks(&blocks, &ids)
        })();
        match scan {
            Ok((requires_tool, skipped)) => {
                book.requires_tool = requires_tool;
                book.warnings = skipped;
            }
            Err(error) => book.warnings.push(format!(
                "the blocks that need the right tool could not be read, every block drops by hand: {error:#}"
            )),
        }
        Ok(book)
    }

    pub fn with_tool_blocks<I: IntoIterator<Item = S>, S: Into<String>>(blocks: I) -> Self {
        Self {
            requires_tool: blocks.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    pub fn with_enchantments(mut self, enchantments: Vec<(String, Value)>) -> Self {
        for (id, json) in enchantments {
            let effects: Vec<AttributeEffect> = json["effects"][ATTRIBUTES]
                .as_array()
                .map(|entries| entries.iter().filter_map(AttributeEffect::parse).collect())
                .unwrap_or_default();
            if effects.is_empty() {
                continue;
            }
            if let Some(slots) = json["slots"].as_array() {
                let slots = slots
                    .iter()
                    .filter_map(|slot| slot.as_str().map(str::to_owned))
                    .collect();
                self.slots.insert(id.clone(), slots);
            }
            self.attributes.insert(id, effects);
        }
        self
    }

    fn fits_slot(&self, enchantment: &str, slot: &str) -> bool {
        self.slots.get(enchantment).is_none_or(|groups| {
            groups.iter().any(|group| match group.as_str() {
                "any" => true,
                "hand" => slot == MAINHAND || slot == OFFHAND,
                "armor" => ARMOR_SLOTS.contains(&slot),
                group => group == slot,
            })
        })
    }

    pub fn attribute(
        &self,
        attribute: &str,
        base: f64,
        range: (f64, f64),
        equipment: &[(&str, Option<&ItemStack>)],
    ) -> f64 {
        let mut added = 0.0;
        let mut multiplied_base = 0.0;
        let mut multiplied_total = 1.0;
        for &(slot, stack) in equipment {
            for (enchantment, level) in stack.map(enchantment_levels).unwrap_or_default() {
                if level <= 0 || !self.fits_slot(&enchantment, slot) {
                    continue;
                }
                for effect in self.attributes.get(&enchantment).into_iter().flatten() {
                    if effect.attribute != attribute {
                        continue;
                    }
                    let amount = f64::from(effect.amount.at(level));
                    match effect.operation {
                        Operation::AddValue => added += amount,
                        Operation::AddMultipliedBase => multiplied_base += amount,
                        Operation::AddMultipliedTotal => multiplied_total *= 1.0 + amount,
                    }
                }
            }
        }
        let value = base + added;
        let value = value + value * multiplied_base;
        (value * multiplied_total).clamp(range.0, range.1)
    }

    pub fn requires_tool(&self, block: &str) -> bool {
        self.requires_tool.contains(block)
    }

    pub fn tool_block_count(&self) -> usize {
        self.requires_tool.len()
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

pub fn dyed_ids(base: &str) -> Vec<String> {
    DYES.iter()
        .map(|dye| format!("minecraft:{dye}_{base}"))
        .collect()
}

pub fn weathered_names(base: &str) -> [String; 4] {
    WEATHER_PREFIXES.map(|prefix| format!("{prefix}{base}"))
}

pub fn copper_ids(names: &[String; 4]) -> Vec<String> {
    let mut out: Vec<String> = names
        .iter()
        .map(|name| format!("minecraft:{name}"))
        .collect();
    out.extend(names.iter().map(|name| format!("minecraft:{WAXED}{name}")));
    out
}

fn static_init(class: &ClassFile) -> Result<Vec<Instruction>> {
    let code = class
        .method("<clinit>")
        .and_then(|method| method.code.as_ref())
        .with_context(|| format!("{} has no static initializer", class.name))?;
    class_file::instructions(code)
}

pub fn id_fields(class: &ClassFile) -> Result<(HashMap<String, Vec<String>>, Vec<String>)> {
    let mut fields = HashMap::new();
    let mut skipped = Vec::new();
    let mut by_state: HashMap<String, Vec<String>> = HashMap::new();
    let mut strings: Vec<String> = Vec::new();
    let mut calls: Vec<String> = Vec::new();
    let mut state_names: Option<Vec<String>> = None;
    for instruction in static_init(class)? {
        match instruction.op {
            class_file::LDC | class_file::LDC_W => {
                if let Some(text) = class.string(instruction.index) {
                    strings.push(text.to_owned());
                }
            }
            class_file::GETSTATIC => {
                if let Some(member) = class.member(instruction.index) {
                    if member.descriptor == BY_STATE {
                        state_names = by_state.get(&member.name).cloned();
                    }
                }
            }
            class_file::INVOKESTATIC | class_file::INVOKEVIRTUAL | class_file::INVOKESPECIAL => {
                if let Some(member) = class.member(instruction.index) {
                    calls.push(member.name);
                }
            }
            class_file::PUTSTATIC => {
                let Some(member) = class.member(instruction.index) else {
                    continue;
                };
                let called = |name: &str| calls.iter().any(|call| call == name);
                let ids = if member.descriptor == BY_STATE {
                    by_state.insert(member.name.clone(), std::mem::take(&mut strings));
                    None
                } else if member.descriptor == COLORS && called("createSimpleColored") {
                    strings.first().map(|base| dyed_ids(base))
                } else if member.descriptor == COPPERS && called("createSimpleCopper") {
                    strings
                        .first()
                        .map(|base| copper_ids(&weathered_names(base)))
                } else if member.descriptor == COPPERS && called("same") {
                    state_names
                        .take()
                        .filter(|names| names.len() == 4)
                        .map(|names| {
                            copper_ids(&std::array::from_fn(|at| {
                                format!("{}{}", WEATHER_PREFIXES[at], names[at])
                            }))
                        })
                } else if called("create") {
                    strings.first().map(|id| vec![format!("minecraft:{id}")])
                } else {
                    None
                };
                match ids {
                    Some(ids) => {
                        fields.insert(member.name, ids);
                    }
                    None if member.descriptor != BY_STATE => skipped.push(format!(
                        "{}.{} has an id this reader does not know",
                        class.name, member.name
                    )),
                    None => {}
                }
                strings.clear();
                calls.clear();
                state_names = None;
            }
            _ => {}
        }
    }
    Ok((fields, skipped))
}

#[derive(Default)]
struct Found {
    requires_tool: bool,
    copies: Vec<String>,
}

fn reach(
    class: &ClassFile,
    code: &[Instruction],
    visited: &mut HashSet<(String, String)>,
    found: &mut Found,
) -> Result<()> {
    let mut source: Option<String> = None;
    for instruction in code {
        match instruction.op {
            class_file::GETSTATIC => {
                let Some(member) = class.member(instruction.index) else {
                    continue;
                };
                if member.class == BLOCKS_CLASS && BLOCK_TYPES.contains(&member.descriptor.as_str())
                {
                    source = Some(member.name);
                }
            }
            class_file::INVOKEVIRTUAL
            | class_file::INVOKESPECIAL
            | class_file::INVOKESTATIC
            | class_file::INVOKEINTERFACE => {
                let Some(member) = class.member(instruction.index) else {
                    continue;
                };
                if member.class == PROPERTIES_CLASS && member.name == REQUIRES_TOOL {
                    found.requires_tool = true;
                }
                if (member.class == BLOCKS_CLASS || member.class == PROPERTIES_CLASS)
                    && COPIES.contains(&member.name.as_str())
                {
                    found.copies.extend(source.clone());
                }
                if instruction.op == class_file::INVOKESTATIC
                    && member.class == BLOCKS_CLASS
                    && member.descriptor.ends_with(RETURNS_PROPERTIES)
                {
                    follow(class, &member.name, &member.descriptor, visited, found)?;
                }
            }
            class_file::INVOKEDYNAMIC => {
                let Some(dynamic) = class.invoke_dynamic(instruction.index) else {
                    continue;
                };
                for argument in dynamic.arguments {
                    let Some(handle) = class.handle(argument) else {
                        continue;
                    };
                    if handle.kind == REF_INVOKE_STATIC && handle.member.class == BLOCKS_CLASS {
                        follow(
                            class,
                            &handle.member.name,
                            &handle.member.descriptor,
                            visited,
                            found,
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn follow(
    class: &ClassFile,
    name: &str,
    descriptor: &str,
    visited: &mut HashSet<(String, String)>,
    found: &mut Found,
) -> Result<()> {
    if !visited.insert((name.to_owned(), descriptor.to_owned())) {
        return Ok(());
    }
    let Some(code) = class
        .method_with(name, descriptor)
        .and_then(|method| method.code.as_ref())
    else {
        return Ok(());
    };
    reach(class, &class_file::instructions(code)?, visited, found)
}

fn inherits(
    field: &str,
    segments: &HashMap<String, (Option<String>, Found)>,
    seen: &mut HashSet<String>,
) -> bool {
    if !seen.insert(field.to_owned()) {
        return false;
    }
    let Some((_, found)) = segments.get(field) else {
        return false;
    };
    found.requires_tool
        || found
            .copies
            .iter()
            .any(|source| inherits(source, segments, seen))
}

pub fn tool_blocks(
    blocks: &ClassFile,
    id_classes: &[ClassFile],
) -> Result<(HashSet<String>, Vec<String>)> {
    let mut ids: HashMap<String, Vec<String>> = HashMap::new();
    let mut warnings = Vec::new();
    for class in id_classes {
        let (fields, skipped) = id_fields(class)?;
        warnings.extend(skipped);
        ids.extend(
            fields
                .into_iter()
                .map(|(field, list)| (format!("{}.{field}", class.name), list)),
        );
    }
    let id_names: Vec<&str> = id_classes.iter().map(|class| class.name.as_str()).collect();
    let code = static_init(blocks)?;
    let mut segments: HashMap<String, (Option<String>, Found)> = HashMap::new();
    let mut start = 0;
    for (at, instruction) in code.iter().enumerate() {
        if instruction.op != class_file::PUTSTATIC {
            continue;
        }
        let Some(member) = blocks.member(instruction.index) else {
            continue;
        };
        if member.class != BLOCKS_CLASS || !BLOCK_TYPES.contains(&member.descriptor.as_str()) {
            continue;
        }
        let segment = &code[start..=at];
        start = at + 1;
        let id = segment.iter().find_map(|instruction| {
            if instruction.op != class_file::GETSTATIC {
                return None;
            }
            let member = blocks.member(instruction.index)?;
            id_names
                .contains(&member.class.as_str())
                .then(|| format!("{}.{}", member.class, member.name))
        });
        let mut found = Found::default();
        reach(blocks, segment, &mut HashSet::new(), &mut found)?;
        segments.insert(member.name, (id, found));
    }
    let mut out = HashSet::new();
    for (field, (id, _)) in &segments {
        if !inherits(field, &segments, &mut HashSet::new()) {
            continue;
        }
        match id.as_ref().and_then(|id| ids.get(id)) {
            Some(list) => out.extend(list.iter().cloned()),
            None => warnings.push(format!("Blocks.{field} needs the right tool but has no id")),
        }
    }
    if out.is_empty() {
        anyhow::bail!("no block in Blocks needs the right tool");
    }
    Ok((out, warnings))
}

pub fn enchantment_levels(stack: &ItemStack) -> Vec<(String, i32)> {
    let Some(levels) = stack
        .components
        .as_ref()
        .and_then(|components| components.get(ENCHANTMENTS))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };
    levels
        .iter()
        .filter_map(|(id, level)| Some((namespaced(id), i32::try_from(level.as_i64()?).ok()?)))
        .collect()
}

fn namespaced(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuleBlocks {
    Tag(String),
    Blocks(Vec<String>),
}

impl RuleBlocks {
    fn parse(value: &Value) -> Option<Self> {
        match value {
            Value::String(text) => Some(match text.strip_prefix('#') {
                Some(tag) => Self::Tag(namespaced(tag)),
                None => Self::Blocks(vec![namespaced(text)]),
            }),
            Value::Array(ids) => ids
                .iter()
                .map(|id| id.as_str().map(namespaced))
                .collect::<Option<Vec<_>>>()
                .map(Self::Blocks),
            _ => None,
        }
    }

    fn holds(&self, facts: &impl BlockFacts, block: &Block) -> bool {
        match self {
            Self::Tag(tag) => facts.in_tag(Some(block), tag),
            Self::Blocks(ids) => ids.iter().any(|id| *id == block.id),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolRule {
    pub blocks: RuleBlocks,
    pub speed: Option<f32>,
    pub correct: Option<bool>,
}

impl ToolRule {
    fn mines_and_drops(blocks: RuleBlocks, speed: f32) -> Self {
        Self {
            blocks,
            speed: Some(speed),
            correct: Some(true),
        }
    }

    fn denies_drops(blocks: RuleBlocks) -> Self {
        Self {
            blocks,
            speed: None,
            correct: Some(false),
        }
    }

    fn override_speed(blocks: RuleBlocks, speed: f32) -> Self {
        Self {
            blocks,
            speed: Some(speed),
            correct: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    pub rules: Vec<ToolRule>,
    pub default_speed: f32,
    pub damage_per_block: u32,
    pub destroys_in_creative: bool,
}

impl Tool {
    fn parse(value: &Value) -> Self {
        let rules = value["rules"]
            .as_array()
            .map(|rules| {
                rules
                    .iter()
                    .filter_map(|rule| {
                        Some(ToolRule {
                            blocks: RuleBlocks::parse(&rule["blocks"])?,
                            speed: rule["speed"].as_f64().map(|speed| speed as f32),
                            correct: rule["correct_for_drops"].as_bool(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            rules,
            default_speed: value["default_mining_speed"].as_f64().unwrap_or(1.0) as f32,
            damage_per_block: value["damage_per_block"]
                .as_u64()
                .and_then(|damage| u32::try_from(damage).ok())
                .unwrap_or(1),
            destroys_in_creative: value["can_destroy_blocks_in_creative"]
                .as_bool()
                .unwrap_or(true),
        }
    }

    fn digger(speed: f32, incorrect: &str, kind: &str) -> Self {
        Self {
            rules: vec![
                ToolRule::denies_drops(RuleBlocks::Tag(incorrect.to_owned())),
                ToolRule::mines_and_drops(
                    RuleBlocks::Tag(format!("minecraft:mineable/{kind}")),
                    speed,
                ),
            ],
            default_speed: 1.0,
            damage_per_block: 1,
            destroys_in_creative: true,
        }
    }

    fn sword() -> Self {
        Self {
            rules: vec![
                ToolRule::mines_and_drops(RuleBlocks::Blocks(vec![COBWEB.to_owned()]), 15.0),
                ToolRule::override_speed(
                    RuleBlocks::Tag(SWORD_INSTANTLY_MINES.to_owned()),
                    f32::MAX,
                ),
                ToolRule::override_speed(RuleBlocks::Tag(SWORD_EFFICIENT.to_owned()), 1.5),
            ],
            default_speed: 1.0,
            damage_per_block: 2,
            destroys_in_creative: false,
        }
    }

    fn shears() -> Self {
        Self {
            rules: vec![
                ToolRule::mines_and_drops(RuleBlocks::Blocks(vec![COBWEB.to_owned()]), 15.0),
                ToolRule::override_speed(RuleBlocks::Tag(SHEARS_EXTREME.to_owned()), 15.0),
                ToolRule::override_speed(RuleBlocks::Tag(SHEARS_MAJOR.to_owned()), 5.0),
                ToolRule::override_speed(RuleBlocks::Tag(SHEARS_MINOR.to_owned()), 2.0),
            ],
            default_speed: 1.0,
            damage_per_block: 1,
            destroys_in_creative: true,
        }
    }

    fn weapon() -> Self {
        Self {
            rules: Vec::new(),
            default_speed: 1.0,
            damage_per_block: 2,
            destroys_in_creative: false,
        }
    }

    pub fn speed(&self, facts: &impl BlockFacts, block: &Block) -> f32 {
        self.rules
            .iter()
            .find_map(|rule| {
                let speed = rule.speed?;
                rule.blocks.holds(facts, block).then_some(speed)
            })
            .unwrap_or(self.default_speed)
    }

    pub fn correct(&self, facts: &impl BlockFacts, block: &Block) -> bool {
        self.rules
            .iter()
            .find_map(|rule| {
                let correct = rule.correct?;
                rule.blocks.holds(facts, block).then_some(correct)
            })
            .unwrap_or(false)
    }
}

pub fn tool_of(stack: Option<&ItemStack>) -> Option<Tool> {
    let stack = stack?;
    if let Some(components) = stack.components.as_ref() {
        if components.get(NO_TOOL).is_some() {
            return None;
        }
        if let Some(tool) = components.get(TOOL) {
            return Some(Tool::parse(tool));
        }
    }
    let name = stack.id.strip_prefix("minecraft:")?;
    match name {
        "shears" => return Some(Tool::shears()),
        "mace" | "trident" => return Some(Tool::weapon()),
        _ => {}
    }
    let (material, kind) = name.rsplit_once('_')?;
    let &(_, speed, incorrect) = MATERIALS
        .iter()
        .find(|(prefix, _, _)| *prefix == material)?;
    if kind == "sword" {
        return Some(Tool::sword());
    }
    DIGGERS
        .contains(&kind)
        .then(|| Tool::digger(speed, incorrect, kind))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Digger<'a> {
    pub held: Option<&'a ItemStack>,
    pub offhand: Option<&'a ItemStack>,
    pub armor: [Option<&'a ItemStack>; 4],
    pub haste: Option<u8>,
    pub conduit: Option<u8>,
    pub fatigue: Option<u8>,
    pub eye_in_water: bool,
    pub on_ground: bool,
}

impl<'a> Digger<'a> {
    pub fn equipment(&self) -> Vec<(&'static str, Option<&'a ItemStack>)> {
        let mut out = vec![(MAINHAND, self.held), (OFFHAND, self.offhand)];
        out.extend(ARMOR_SLOTS.iter().copied().zip(self.armor));
        out
    }
}

fn attribute(
    book: Option<&MiningBook>,
    attribute: &str,
    base: f64,
    range: (f64, f64),
    digger: &Digger,
) -> f64 {
    match book {
        Some(book) => book.attribute(attribute, base, range, &digger.equipment()),
        None => base,
    }
}

pub fn correct_for_drops(
    book: Option<&MiningBook>,
    facts: &impl BlockFacts,
    block: &Block,
    held: Option<&ItemStack>,
) -> bool {
    !book.is_some_and(|book| book.requires_tool(&block.id))
        || tool_of(held).is_some_and(|tool| tool.correct(facts, block))
}

pub fn destroy_speed(
    book: Option<&MiningBook>,
    facts: &impl BlockFacts,
    block: &Block,
    digger: &Digger,
) -> f32 {
    let mut speed = tool_of(digger.held).map_or(1.0, |tool| tool.speed(facts, block));
    if speed > 1.0 {
        speed += attribute(book, MINING_EFFICIENCY, 0.0, (0.0, 1024.0), digger) as f32;
    }
    if digger.haste.is_some() || digger.conduit.is_some() {
        let amplifier = digger.haste.unwrap_or(0).max(digger.conduit.unwrap_or(0));
        speed *= 1.0 + (i32::from(amplifier) + 1) as f32 * 0.2;
    }
    if let Some(amplifier) = digger.fatigue {
        speed *= 0.3_f64.powf(f64::from(amplifier) + 1.0) as f32;
    }
    speed *= attribute(book, BLOCK_BREAK_SPEED, 1.0, (0.0, 1024.0), digger) as f32;
    if digger.eye_in_water {
        speed *= attribute(book, SUBMERGED_MINING_SPEED, 0.2, (0.0, 20.0), digger) as f32;
    }
    if !digger.on_ground {
        speed /= 5.0;
    }
    speed
}

pub fn destroy_progress(
    book: Option<&MiningBook>,
    facts: &impl BlockFacts,
    block: &Block,
    hardness: f32,
    digger: &Digger,
) -> f32 {
    if hardness == UNBREAKABLE {
        return 0.0;
    }
    let divisor = if correct_for_drops(book, facts, block, digger.held) {
        NORMAL_DIVISOR
    } else {
        WRONG_TOOL_DIVISOR
    };
    destroy_speed(book, facts, block, digger) / hardness / divisor
}

pub fn ticks_to_break(progress: f32) -> Option<u32> {
    if progress.is_nan() || progress <= 0.0 {
        return None;
    }
    let mut sum = 0.0_f32;
    let mut ticks = 0;
    while sum < 1.0 {
        let next = sum + progress;
        if next == sum {
            return None;
        }
        sum = next;
        ticks += 1;
    }
    Some(ticks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dyed_ids_follow_the_dye_colour_order() {
        let ids = dyed_ids("concrete");
        assert_eq!(ids.len(), 16);
        assert_eq!(ids[0], "minecraft:white_concrete");
        assert_eq!(ids[3], "minecraft:light_blue_concrete");
        assert_eq!(ids[15], "minecraft:black_concrete");
    }

    #[test]
    fn copper_ids_have_every_weather_state_and_its_waxed_twin() {
        assert_eq!(
            copper_ids(&weathered_names("cut_copper_stairs")),
            [
                "minecraft:cut_copper_stairs",
                "minecraft:exposed_cut_copper_stairs",
                "minecraft:weathered_cut_copper_stairs",
                "minecraft:oxidized_cut_copper_stairs",
                "minecraft:waxed_cut_copper_stairs",
                "minecraft:waxed_exposed_cut_copper_stairs",
                "minecraft:waxed_weathered_cut_copper_stairs",
                "minecraft:waxed_oxidized_cut_copper_stairs",
            ]
        );
    }

    #[test]
    fn the_copper_block_uses_its_special_names() {
        let names = ["copper_block", "copper", "copper", "copper"].map(String::from);
        let special: [String; 4] =
            std::array::from_fn(|at| format!("{}{}", WEATHER_PREFIXES[at], names[at]));
        assert_eq!(
            copper_ids(&special),
            [
                "minecraft:copper_block",
                "minecraft:exposed_copper",
                "minecraft:weathered_copper",
                "minecraft:oxidized_copper",
                "minecraft:waxed_copper_block",
                "minecraft:waxed_exposed_copper",
                "minecraft:waxed_weathered_copper",
                "minecraft:waxed_oxidized_copper",
            ]
        );
    }

    #[test]
    fn a_book_made_from_a_list_answers_for_it() {
        let book = MiningBook::with_tool_blocks(["minecraft:stone"]);
        assert!(book.requires_tool("minecraft:stone"));
        assert!(!book.requires_tool("minecraft:dirt"));
        assert_eq!(book.tool_block_count(), 1);
        assert!(MiningBook::default().warnings().is_empty());
    }

    fn jar() -> Option<std::path::PathBuf> {
        let root = std::env::var_os("MINECRAFTOSS_ROOT")?;
        let jar = Path::new(&root).join("client.jar");
        jar.is_file().then_some(jar)
    }

    #[test]
    fn the_jar_names_the_blocks_that_need_the_right_tool() {
        let Some(jar) = jar() else {
            return;
        };
        let book = MiningBook::from_jar(&jar).unwrap();
        assert!(book.warnings().is_empty(), "{:?}", book.warnings());
        assert!(book.tool_block_count() > 400, "{}", book.tool_block_count());
        let needs = [
            "stone",
            "netherrack",
            "obsidian",
            "cobweb",
            "snow",
            "snow_block",
            "iron_ore",
            "deepslate_iron_ore",
            "cobbled_deepslate",
            "stone_slab",
            "stone_stairs",
            "cobblestone_wall",
            "iron_chain",
            "iron_trapdoor",
            "furnace",
            "anvil",
            "ancient_debris",
            "end_stone",
            "basalt",
            "white_concrete",
            "black_concrete_stairs",
            "red_terracotta",
            "white_glazed_terracotta",
            "copper_block",
            "waxed_exposed_copper",
            "cut_copper",
            "waxed_oxidized_cut_copper_stairs",
        ];
        for block in needs {
            assert!(book.requires_tool(&format!("minecraft:{block}")), "{block}");
        }
        let free = [
            "dirt",
            "ice",
            "packed_ice",
            "oak_log",
            "glass",
            "white_wool",
            "iron_door",
            "bell",
            "lantern",
            "ender_chest",
            "brewing_stand",
            "copper_door",
            "stone_button",
            "stone_pressure_plate",
            "piston",
            "conduit",
            "crafter",
            "heavy_core",
            "powder_snow",
            "bedrock",
        ];
        for block in free {
            assert!(
                !book.requires_tool(&format!("minecraft:{block}")),
                "{block}"
            );
        }
    }

    #[test]
    fn every_block_that_needs_the_right_tool_is_a_real_block() {
        let Some(jar) = jar() else {
            return;
        };
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        let book = MiningBook::from_jar(&jar).unwrap();
        let mut missing: Vec<&String> = book
            .requires_tool
            .iter()
            .filter(|id| registries.blocks.block_by_name(id).is_none())
            .collect();
        missing.sort();
        assert!(missing.is_empty(), "{missing:?}");
    }

    struct Facts;

    impl BlockFacts for Facts {
        fn in_tag(&self, block: Option<&Block>, tag: &str) -> bool {
            let Some(block) = block else {
                return false;
            };
            let members: &[&str] = match tag {
                "minecraft:mineable/pickaxe" => &["stone", "netherrack", "obsidian", "iron_ore"],
                "minecraft:mineable/axe" => &["oak_log"],
                "minecraft:mineable/shovel" => &["dirt"],
                "minecraft:incorrect_for_wooden_tool" => &["obsidian", "iron_ore"],
                "minecraft:incorrect_for_stone_tool" | "minecraft:incorrect_for_iron_tool" => {
                    &["obsidian"]
                }
                "minecraft:sword_instantly_mines" => &["bamboo"],
                "minecraft:sword_efficient" => &["oak_leaves"],
                "minecraft:shears_extreme_breaking_speed" => &["oak_leaves"],
                "minecraft:shears_major_breaking_speed" => &["white_wool"],
                _ => &[],
            };
            members
                .iter()
                .any(|member| block.id == format!("minecraft:{member}"))
        }

        fn has_property(&self, _: &Block, _: &str) -> bool {
            false
        }
    }

    fn hardness(block: &str) -> f32 {
        match block {
            "stone" => 1.5,
            "netherrack" => 0.4,
            "obsidian" => 50.0,
            "iron_ore" => 3.0,
            "cobweb" => 4.0,
            "bamboo" => 1.0,
            "white_wool" => 0.8,
            "oak_log" => 2.0,
            "dirt" => 0.5,
            "torch" => 0.0,
            "bedrock" => -1.0,
            _ => panic!("no hardness for {block}"),
        }
    }

    fn enchantment(attribute: &str, operation: &str, amount: Value, slot: &str) -> Value {
        serde_json::json!({
            "effects": {
                "minecraft:attributes": [{
                    "amount": amount,
                    "attribute": attribute,
                    "id": "minecraft:enchantment.test",
                    "operation": operation,
                }]
            },
            "slots": [slot],
        })
    }

    fn book() -> MiningBook {
        MiningBook::with_tool_blocks(
            ["stone", "netherrack", "obsidian", "iron_ore", "cobweb"]
                .map(|block| format!("minecraft:{block}")),
        )
        .with_enchantments(vec![
            (
                "minecraft:efficiency".to_owned(),
                enchantment(
                    MINING_EFFICIENCY,
                    "add_value",
                    serde_json::json!({"type": "minecraft:levels_squared", "added": 1.0}),
                    "mainhand",
                ),
            ),
            (
                "minecraft:aqua_affinity".to_owned(),
                enchantment(
                    SUBMERGED_MINING_SPEED,
                    "add_multiplied_total",
                    serde_json::json!({
                        "type": "minecraft:linear",
                        "base": 4.0,
                        "per_level_above_first": 4.0
                    }),
                    "head",
                ),
            ),
        ])
    }

    fn stack(id: &str, enchantments: &[(&str, i64)]) -> ItemStack {
        let mut stack = ItemStack::new(format!("minecraft:{id}"), 1);
        if !enchantments.is_empty() {
            let levels: serde_json::Map<String, Value> = enchantments
                .iter()
                .map(|(id, level)| (format!("minecraft:{id}"), Value::from(*level)))
                .collect();
            stack.components = Some(serde_json::json!({ "minecraft:enchantments": levels }));
        }
        stack
    }

    fn grounded(held: Option<&ItemStack>) -> Digger<'_> {
        Digger {
            held,
            on_ground: true,
            ..Digger::default()
        }
    }

    fn progress(block: &str, digger: &Digger) -> f32 {
        destroy_progress(
            Some(&book()),
            &Facts,
            &Block::new(format!("minecraft:{block}")),
            hardness(block),
            digger,
        )
    }

    fn ticks(block: &str, digger: &Digger) -> Option<u32> {
        ticks_to_break(progress(block, digger))
    }

    fn correct(block: &str, held: Option<&ItemStack>) -> bool {
        correct_for_drops(
            Some(&book()),
            &Facts,
            &Block::new(format!("minecraft:{block}")),
            held,
        )
    }

    #[test]
    fn netherrack_breaks_at_once_with_an_efficient_or_golden_pickaxe() {
        let efficiency_five = stack("diamond_pickaxe", &[("efficiency", 5)]);
        let five = progress("netherrack", &grounded(Some(&efficiency_five)));
        assert!((five - 2.8333).abs() < 1e-3, "{five}");
        assert_eq!(ticks_to_break(five), Some(1));
        let two = stack("diamond_pickaxe", &[("efficiency", 2)]);
        assert_eq!(ticks("netherrack", &grounded(Some(&two))), Some(1));
        let one = stack("diamond_pickaxe", &[("efficiency", 1)]);
        assert_eq!(ticks("netherrack", &grounded(Some(&one))), Some(2));
        let plain = stack("diamond_pickaxe", &[]);
        assert_eq!(ticks("netherrack", &grounded(Some(&plain))), Some(2));
        let golden = stack("golden_pickaxe", &[]);
        assert_eq!(progress("netherrack", &grounded(Some(&golden))), 1.0);
        assert_eq!(ticks("netherrack", &grounded(Some(&golden))), Some(1));
        let wooden = stack("wooden_pickaxe", &[]);
        assert_eq!(ticks("netherrack", &grounded(Some(&wooden))), Some(6));
        assert_eq!(ticks("netherrack", &grounded(None)), Some(41));
        assert!(!correct("netherrack", None));
        assert!(correct("netherrack", Some(&wooden)));
    }

    #[test]
    fn stone_follows_efficiency_and_the_mining_effects() {
        let pickaxe = stack("diamond_pickaxe", &[("efficiency", 5)]);
        let digger = grounded(Some(&pickaxe));
        assert_eq!(ticks("stone", &grounded(None)), Some(151));
        assert!(!correct("stone", None));
        let plain = stack("diamond_pickaxe", &[]);
        assert_eq!(ticks("stone", &grounded(Some(&plain))), Some(6));
        assert_eq!(ticks("stone", &digger), Some(2));
        let with = |haste, conduit, fatigue| Digger {
            haste,
            conduit,
            fatigue,
            ..digger
        };
        assert_eq!(ticks("stone", &with(Some(1), None, None)), Some(1));
        assert_eq!(ticks("stone", &with(None, Some(0), None)), Some(2));
        assert_eq!(ticks("stone", &with(Some(0), Some(1), None)), Some(1));
        assert_eq!(ticks("stone", &with(None, None, Some(0))), Some(5));
        let tired = progress("stone", &with(None, None, Some(2)));
        assert!((tired - 0.0204).abs() < 1e-4, "{tired}");
        assert_eq!(ticks_to_break(tired), Some(50));
    }

    #[test]
    fn water_and_air_slow_mining_unless_aqua_affinity_helps() {
        let pickaxe = stack("diamond_pickaxe", &[("efficiency", 5)]);
        let helmet = stack("diamond_helmet", &[("aqua_affinity", 1)]);
        let digger = |eye_in_water, on_ground, head| Digger {
            held: Some(&pickaxe),
            armor: [None, None, None, head],
            eye_in_water,
            on_ground,
            ..Digger::default()
        };
        assert_eq!(ticks("stone", &digger(true, true, None)), Some(7));
        assert_eq!(ticks("stone", &digger(true, false, None)), Some(34));
        assert_eq!(ticks("stone", &digger(true, false, Some(&helmet))), Some(7));
        assert_eq!(ticks("stone", &digger(false, false, None)), Some(7));
    }

    #[test]
    fn hard_blocks_need_the_right_tier() {
        let efficient = stack("diamond_pickaxe", &[("efficiency", 5)]);
        assert_eq!(ticks("obsidian", &grounded(Some(&efficient))), Some(45));
        assert!(correct("obsidian", Some(&efficient)));
        let iron = stack("iron_pickaxe", &[]);
        assert!(!correct("obsidian", Some(&iron)));
        assert_eq!(
            progress("obsidian", &grounded(Some(&iron))),
            6.0 / 50.0 / 100.0
        );
        let wooden = stack("wooden_pickaxe", &[]);
        assert_eq!(ticks("iron_ore", &grounded(Some(&wooden))), Some(151));
        assert!(!correct("iron_ore", Some(&wooden)));
        let stone = stack("stone_pickaxe", &[]);
        assert_eq!(ticks("iron_ore", &grounded(Some(&stone))), Some(23));
        assert!(correct("iron_ore", Some(&stone)));
    }

    #[test]
    fn swords_shears_and_axes_have_their_own_blocks() {
        let sword = stack("diamond_sword", &[]);
        let shears = stack("shears", &[]);
        assert_eq!(ticks("cobweb", &grounded(Some(&sword))), Some(8));
        assert!(correct("cobweb", Some(&sword)));
        assert_eq!(ticks("cobweb", &grounded(Some(&shears))), Some(8));
        assert!(correct("cobweb", Some(&shears)));
        assert_eq!(ticks("cobweb", &grounded(None)), Some(401));
        assert!(!correct("cobweb", None));
        let bamboo = progress("bamboo", &grounded(Some(&sword)));
        assert!(bamboo.is_finite() && bamboo >= 1.0, "{bamboo}");
        assert_eq!(ticks("white_wool", &grounded(Some(&shears))), Some(5));
        let axe = stack("iron_axe", &[]);
        assert_eq!(ticks("oak_log", &grounded(Some(&axe))), Some(10));
        let pickaxe = stack("diamond_pickaxe", &[("efficiency", 5)]);
        assert_eq!(ticks("dirt", &grounded(Some(&pickaxe))), Some(15));
        assert_eq!(ticks("dirt", &grounded(None)), Some(15));
    }

    #[test]
    fn free_and_unbreakable_blocks() {
        let tired = Digger {
            fatigue: Some(2),
            ..grounded(None)
        };
        assert_eq!(ticks("torch", &tired), Some(1));
        assert_eq!(progress("bedrock", &grounded(None)), 0.0);
        assert_eq!(ticks("bedrock", &grounded(None)), None);
    }

    #[test]
    fn efficiency_counts_only_in_the_main_hand() {
        let pickaxe = stack("diamond_pickaxe", &[]);
        let spare = stack("diamond_pickaxe", &[("efficiency", 5)]);
        let helmet = stack("diamond_helmet", &[("efficiency", 5)]);
        let digger = Digger {
            offhand: Some(&spare),
            armor: [None, None, None, Some(&helmet)],
            ..grounded(Some(&pickaxe))
        };
        assert_eq!(
            book().attribute(MINING_EFFICIENCY, 0.0, (0.0, 1024.0), &digger.equipment()),
            0.0
        );
        assert_eq!(ticks("stone", &digger), Some(6));
    }

    #[test]
    fn tools_come_from_the_item_or_its_tool_component() {
        let tool = |id: &str| tool_of(Some(&stack(id, &[])));
        assert_eq!(tool("netherite_shovel").unwrap().damage_per_block, 1);
        assert_eq!(tool("copper_axe").unwrap().rules.len(), 2);
        let sword = tool("golden_sword").unwrap();
        assert_eq!(
            (sword.damage_per_block, sword.destroys_in_creative),
            (2, false)
        );
        assert_eq!(tool("mace").unwrap().rules, Vec::new());
        assert_eq!(tool("trident").unwrap().damage_per_block, 2);
        assert!(tool("shears").unwrap().destroys_in_creative);
        assert_eq!(tool("iron_spear"), None);
        assert_eq!(tool("stick"), None);
        assert_eq!(tool_of(None), None);
        let mut custom = stack("stick", &[]);
        custom.components = Some(serde_json::json!({
            "minecraft:tool": {
                "rules": [
                    {"blocks": "#minecraft:mineable/shovel", "speed": 20.0, "correct_for_drops": true},
                    {"blocks": ["minecraft:stone", "cobweb"], "correct_for_drops": false}
                ],
                "default_mining_speed": 3.0,
                "damage_per_block": 4
            }
        }));
        let custom_tool = tool_of(Some(&custom)).unwrap();
        assert_eq!(
            custom_tool.speed(&Facts, &Block::new("minecraft:dirt")),
            20.0
        );
        assert_eq!(
            custom_tool.speed(&Facts, &Block::new("minecraft:stone")),
            3.0
        );
        assert!(!custom_tool.correct(&Facts, &Block::new("minecraft:cobweb")));
        assert_eq!(
            (
                custom_tool.damage_per_block,
                custom_tool.destroys_in_creative
            ),
            (4, true)
        );
        let mut removed = stack("diamond_pickaxe", &[]);
        removed.components = Some(serde_json::json!({"!minecraft:tool": {}}));
        assert_eq!(tool_of(Some(&removed)), None);
    }

    #[test]
    fn without_a_book_every_block_drops_and_nothing_is_enchanted() {
        let pickaxe = stack("diamond_pickaxe", &[("efficiency", 5)]);
        let stone = Block::new("minecraft:stone");
        assert!(correct_for_drops(None, &Facts, &stone, None));
        assert_eq!(
            destroy_speed(None, &Facts, &stone, &grounded(Some(&pickaxe))),
            8.0
        );
    }

    #[test]
    fn level_values_and_slot_groups_follow_vanilla() {
        let squared = LevelValue::parse(
            &serde_json::json!({"type": "minecraft:levels_squared", "added": 1.0}),
        )
        .unwrap();
        assert_eq!(
            [1, 2, 3, 4, 5].map(|level| squared.at(level)),
            [2.0, 5.0, 10.0, 17.0, 26.0]
        );
        let clamped = LevelValue::parse(&serde_json::json!({
            "type": "minecraft:clamped",
            "value": {"type": "minecraft:linear", "base": 1.0, "per_level_above_first": 2.0},
            "min": 0.0,
            "max": 4.0
        }))
        .unwrap();
        assert_eq!([1, 2, 3].map(|level| clamped.at(level)), [1.0, 3.0, 4.0]);
        assert_eq!(
            LevelValue::parse(&serde_json::json!(2.5)),
            Some(LevelValue::Constant(2.5))
        );
        let book = MiningBook::default().with_enchantments(vec![(
            "minecraft:test".to_owned(),
            enchantment(
                BLOCK_BREAK_SPEED,
                "add_multiplied_base",
                serde_json::json!(0.5),
                "hand",
            ),
        )]);
        let held = stack("stick", &[("test", 1)]);
        assert_eq!(
            book.attribute(
                BLOCK_BREAK_SPEED,
                1.0,
                (0.0, 1024.0),
                &[(OFFHAND, Some(&held))]
            ),
            1.5
        );
        assert_eq!(
            book.attribute(
                BLOCK_BREAK_SPEED,
                1.0,
                (0.0, 1024.0),
                &[("chest", Some(&held))]
            ),
            1.0
        );
    }

    #[test]
    fn efficiency_and_aqua_affinity_read_from_the_jar() {
        let Some(jar) = jar() else {
            return;
        };
        let book = MiningBook::from_jar(&jar).unwrap();
        let pickaxe = stack("diamond_pickaxe", &[("efficiency", 5)]);
        let helmet = stack("diamond_helmet", &[("aqua_affinity", 1)]);
        let digger = Digger {
            held: Some(&pickaxe),
            armor: [None, None, None, Some(&helmet)],
            ..Digger::default()
        };
        let equipment = digger.equipment();
        assert_eq!(
            book.attribute(MINING_EFFICIENCY, 0.0, (0.0, 1024.0), &equipment),
            26.0
        );
        assert_eq!(
            book.attribute(SUBMERGED_MINING_SPEED, 0.2, (0.0, 20.0), &equipment),
            1.0
        );
        assert_eq!(
            book.attribute(BLOCK_BREAK_SPEED, 1.0, (0.0, 1024.0), &equipment),
            1.0
        );
    }
}
