use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};

use crate::class_file::{self, ClassFile, Instruction};

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

#[derive(Clone, Debug, Default)]
pub struct MiningBook {
    requires_tool: HashSet<String>,
    warnings: Vec<String>,
}

impl MiningBook {
    pub fn from_jar(jar: &Path) -> Result<Self> {
        let file =
            std::fs::File::open(jar).with_context(|| format!("cannot open {}", jar.display()))?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut class = |name: &str| -> Result<ClassFile> {
            let mut entry = archive
                .by_name(&format!("{name}.class"))
                .with_context(|| format!("no {name} in the jar"))?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            ClassFile::parse(&bytes)
        };
        let mut book = Self::default();
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
}
