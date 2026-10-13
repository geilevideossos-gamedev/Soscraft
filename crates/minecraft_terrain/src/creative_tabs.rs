use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};

use crate::class_file::{
    BIPUSH, ClassFile, GETFIELD, GETSTATIC, GOTO, GOTO_W, ICONST_5, ICONST_M1, IINC, INVOKEDYNAMIC,
    INVOKEINTERFACE, INVOKESPECIAL, INVOKESTATIC, INVOKEVIRTUAL, Instruction, LDC, LDC_W, LDC2_W,
    LOOKUPSWITCH, Member, Method, NEW, PUTFIELD, PUTSTATIC, SIPUSH, TABLESWITCH, instructions,
    parameters, returns_value,
};

pub const TABS_CLASS: &str = "net/minecraft/world/item/CreativeModeTabs";
pub const ITEMS_CLASS: &str = "net/minecraft/world/item/Items";
pub const BLOCKS_CLASS: &str = "net/minecraft/world/level/block/Blocks";
const REFERENCES: &str = "net/minecraft/references/";
const ITEM_DEFINITIONS: &str = "assets/minecraft/items/";
const OUTPUT: &str = "net/minecraft/world/item/CreativeModeTab$Output";
const PARAMETERS: &str = "net/minecraft/world/item/CreativeModeTab$ItemDisplayParameters";
const BUILDER: &str = "net/minecraft/world/item/CreativeModeTab$Builder";
const TAB: &str = "net/minecraft/world/item/CreativeModeTab";
const ROW: &str = "net/minecraft/world/item/CreativeModeTab$Row";
const TYPE: &str = "net/minecraft/world/item/CreativeModeTab$Type";
const VISIBILITY: &str = "net/minecraft/world/item/CreativeModeTab$TabVisibility";
const COLORS: &str = "net/minecraft/world/level/block/ColorCollection";
const COPPER: &str = "net/minecraft/world/level/block/WeatheringCopperCollection";
const BY_STATE: &str = "net/minecraft/world/level/block/WeatheringCopperCollection$ByState";
const DYE_COLOR: &str = "net/minecraft/world/item/DyeColor";
const ITEM_STACK: &str = "net/minecraft/world/item/ItemStack";
const LAMBDA_FACTORY: &str = "java/lang/invoke/LambdaMetafactory";
const ITEM_DESCRIPTOR: &str = "Lnet/minecraft/world/item/Item;";
const BLOCK_DESCRIPTOR: &str = "Lnet/minecraft/world/level/block/Block;";
const COLORS_DESCRIPTOR: &str = "Lnet/minecraft/world/level/block/ColorCollection;";
const COPPER_DESCRIPTOR: &str = "Lnet/minecraft/world/level/block/WeatheringCopperCollection;";
const DYE_COLORS: [&str; 16] = [
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
const COPPER_STATES: [&str; 4] = ["", "exposed_", "weathered_", "oxidized_"];
const BY_STATE_ACCESSORS: [&str; 4] = ["unaffected", "exposed", "weathered", "oxidized"];
const DEPTH_LIMIT: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabKind {
    Category,
    Hotbar,
    Search,
    Inventory,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Variants {
    Potions,
    EnchantedBooks { all_levels: bool },
    Instruments { tag: String },
    SuspiciousStews,
    OminousBottles,
    Fireworks,
    Paintings { tag: String, in_tag: bool },
    OminousBanner,
    TestBlockModes,
    LightLevels,
    FullClip,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabEntry {
    pub item: String,
    pub variants: Option<Variants>,
    pub in_tab: bool,
    pub in_search: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreativeTab {
    pub id: String,
    pub title: String,
    pub row: Row,
    pub column: u8,
    pub kind: TabKind,
    pub aligned_right: bool,
    pub show_title: bool,
    pub scroll_bar: bool,
    pub background: String,
    pub icon: Option<String>,
    pub operator: bool,
    pub entries: Vec<TabEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CreativeTabs {
    pub tabs: Vec<CreativeTab>,
    pub fallback: Option<String>,
    pub skipped: Vec<String>,
}

impl CreativeTab {
    pub fn displayed(&self) -> impl Iterator<Item = &TabEntry> {
        self.entries.iter().filter(|entry| entry.in_tab)
    }

    pub fn item_ids(&self) -> Vec<&str> {
        self.displayed().map(|entry| entry.item.as_str()).collect()
    }
}

impl CreativeTabs {
    pub fn tab(&self, id: &str) -> Option<&CreativeTab> {
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn shown(&self, operator: bool) -> impl Iterator<Item = &CreativeTab> {
        self.tabs.iter().filter(move |tab| {
            (operator || !tab.operator)
                && (tab.kind != TabKind::Category || tab.displayed().next().is_some())
        })
    }

    pub fn search(&self, operator: bool) -> Vec<TabEntry> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let sources = self
            .tabs
            .iter()
            .filter(|tab| tab.kind == TabKind::Category && (operator || !tab.operator))
            .flat_map(|tab| tab.entries.iter().filter(|entry| entry.in_search))
            .chain(
                self.tabs
                    .iter()
                    .filter(|tab| tab.kind == TabKind::Search)
                    .flat_map(|tab| tab.entries.iter()),
            );
        for entry in sources {
            if seen.insert((entry.item.clone(), entry.variants.clone())) {
                out.push(TabEntry {
                    in_tab: true,
                    in_search: true,
                    ..entry.clone()
                });
            }
        }
        out
    }
}

pub fn creative_tabs(jar: &Path) -> CreativeTabs {
    match read_creative_tabs(jar) {
        Ok(tabs) => {
            if !tabs.skipped.is_empty() {
                eprintln!(
                    "creative tabs skipped {} entries: {}",
                    tabs.skipped.len(),
                    tabs.skipped.join("; ")
                );
            }
            tabs
        }
        Err(error) => {
            let reason = format!("{error:#}");
            eprintln!("creative tabs fall back to registry order: {reason}");
            let items = Jar::open(jar)
                .map(|mut jar| {
                    let known = jar.item_ids();
                    registry_order(&mut |name| jar.class(name), &known)
                })
                .unwrap_or_else(|_| Vec::new());
            fallback_tabs(items, reason)
        }
    }
}

pub fn read_creative_tabs(jar: &Path) -> Result<CreativeTabs> {
    let mut jar = Jar::open(jar)?;
    let known = jar.item_ids();
    read_tabs(&mut |name| jar.class(name), &known)
}

struct Jar {
    archive: zip::ZipArchive<std::fs::File>,
}

impl Jar {
    fn open(path: &Path) -> Result<Jar> {
        let file =
            std::fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
        Ok(Jar {
            archive: zip::ZipArchive::new(file)?,
        })
    }

    fn class(&mut self, name: &str) -> Option<Vec<u8>> {
        let mut entry = self.archive.by_name(&format!("{name}.class")).ok()?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }

    fn item_ids(&self) -> HashSet<String> {
        self.archive
            .file_names()
            .filter_map(|path| path.strip_prefix(ITEM_DEFINITIONS)?.strip_suffix(".json"))
            .filter(|name| !name.contains('/'))
            .map(|name| format!("minecraft:{name}"))
            .collect()
    }
}

pub fn read_tabs(
    load: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    known: &HashSet<String>,
) -> Result<CreativeTabs> {
    if known.is_empty() {
        bail!("no item ids to check the tabs against");
    }
    let class = parse(load, TABS_CLASS)?;
    let names = Names::read(load, known)?;
    let mut machine = Machine::new(&class, &names, known);
    if let Some(init) = class.method("<clinit>") {
        machine
            .run(init, Vec::new())
            .context("CreativeModeTabs.<clinit>")?;
    }
    let bootstrap = class
        .method("bootstrap")
        .ok_or_else(|| anyhow!("CreativeModeTabs has no bootstrap method"))?;
    machine
        .run(bootstrap, vec![Value::Unknown])
        .context("CreativeModeTabs.bootstrap")?;
    if machine.registered.is_empty() {
        bail!("CreativeModeTabs.bootstrap registers no tabs");
    }
    let mut tabs = Vec::new();
    for (id, draft) in machine.registered.clone() {
        let draft = machine.drafts[draft].clone();
        let icon = match &draft.icon {
            Some(icon) => match machine.apply(icon, Vec::new()) {
                Ok(Value::Item(item, _)) if known.contains(&item) => Some(item),
                Ok(other) => {
                    machine
                        .skipped
                        .push(format!("{id}: the icon is not an item ({other:?})"));
                    None
                }
                Err(error) => {
                    machine.skipped.push(format!("{id}: icon: {error:#}"));
                    None
                }
            },
            None => None,
        };
        let mut entries = Vec::new();
        let mut operator = false;
        if let (Some(display), true) = (&draft.display, draft.kind != TabKind::Search) {
            machine.emitted.clear();
            machine.asked_permission = false;
            match machine.apply(display, vec![Value::Parameters, Value::Output]) {
                Ok(_) => {
                    entries = std::mem::take(&mut machine.emitted);
                    operator = machine.asked_permission;
                }
                Err(error) => machine.skipped.push(format!("{id}: {error:#}")),
            }
        }
        let title = draft.title.unwrap_or_else(|| id.clone());
        tabs.push(CreativeTab {
            id,
            title,
            row: draft.row.unwrap_or(Row::Top),
            column: draft.column,
            kind: draft.kind,
            aligned_right: draft.aligned_right,
            show_title: draft.show_title,
            scroll_bar: draft.scroll_bar,
            background: draft.background.unwrap_or_else(|| "items".to_owned()),
            icon,
            operator,
            entries,
        });
    }
    if !tabs
        .iter()
        .any(|tab| tab.kind == TabKind::Category && tab.displayed().next().is_some())
    {
        bail!("no creative tab has any item");
    }
    Ok(CreativeTabs {
        tabs,
        fallback: None,
        skipped: machine.skipped,
    })
}

pub fn registry_order(
    load: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    known: &HashSet<String>,
) -> Vec<String> {
    let mut order = Names::read(load, known)
        .map(|names| names.order)
        .unwrap_or_else(|_| Vec::new());
    let listed: HashSet<&String> = order.iter().collect();
    let mut rest: Vec<String> = known
        .iter()
        .filter(|id| !listed.contains(id))
        .cloned()
        .collect();
    rest.sort();
    order.extend(rest);
    order
}

pub fn fallback_tabs(items: Vec<String>, reason: impl Into<String>) -> CreativeTabs {
    let layout = [
        (
            "building_blocks",
            "buildingBlocks",
            Row::Top,
            0,
            TabKind::Category,
        ),
        (
            "colored_blocks",
            "coloredBlocks",
            Row::Top,
            1,
            TabKind::Category,
        ),
        ("natural_blocks", "natural", Row::Top, 2, TabKind::Category),
        (
            "functional_blocks",
            "functional",
            Row::Top,
            3,
            TabKind::Category,
        ),
        (
            "redstone_blocks",
            "redstone",
            Row::Top,
            4,
            TabKind::Category,
        ),
        ("hotbar", "hotbar", Row::Top, 5, TabKind::Hotbar),
        ("search", "search", Row::Top, 6, TabKind::Search),
        (
            "tools_and_utilities",
            "tools",
            Row::Bottom,
            0,
            TabKind::Category,
        ),
        ("combat", "combat", Row::Bottom, 1, TabKind::Category),
        (
            "food_and_drinks",
            "foodAndDrink",
            Row::Bottom,
            2,
            TabKind::Category,
        ),
        (
            "ingredients",
            "ingredients",
            Row::Bottom,
            3,
            TabKind::Category,
        ),
        ("spawn_eggs", "spawnEggs", Row::Bottom, 4, TabKind::Category),
        ("op_blocks", "op", Row::Bottom, 5, TabKind::Category),
        ("inventory", "inventory", Row::Bottom, 6, TabKind::Inventory),
    ];
    let tabs = layout
        .into_iter()
        .map(|(id, title, row, column, kind)| CreativeTab {
            id: format!("minecraft:{id}"),
            title: format!("itemGroup.{title}"),
            row,
            column,
            kind,
            aligned_right: column >= 5,
            show_title: kind != TabKind::Inventory,
            scroll_bar: kind != TabKind::Inventory,
            background: match kind {
                TabKind::Search => "item_search",
                TabKind::Inventory => "inventory",
                _ => "items",
            }
            .to_owned(),
            icon: None,
            operator: id == "op_blocks",
            entries: if kind == TabKind::Search {
                items
                    .iter()
                    .map(|item| TabEntry {
                        item: item.clone(),
                        variants: None,
                        in_tab: true,
                        in_search: true,
                    })
                    .collect()
            } else {
                Vec::new()
            },
        })
        .collect();
    CreativeTabs {
        tabs,
        fallback: Some(reason.into()),
        skipped: Vec::new(),
    }
}

fn parse(load: &mut dyn FnMut(&str) -> Option<Vec<u8>>, name: &str) -> Result<ClassFile> {
    let bytes = load(name).ok_or_else(|| anyhow!("{name}.class is not in the jar"))?;
    ClassFile::parse(&bytes).with_context(|| format!("{name}.class"))
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            if !out.is_empty() && !out.ends_with('_') && name.chars().any(|c| c.is_lowercase()) {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn color_index(name: &str) -> Option<usize> {
    let name = snake(name);
    DYE_COLORS.iter().position(|color| *color == name)
}

fn namespaced(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Ids {
    One(String),
    Colors(Vec<String>),
    Copper(Vec<String>, Vec<String>),
}

impl Ids {
    fn flatten(&self) -> Vec<String> {
        match self {
            Ids::One(id) => vec![id.clone()],
            Ids::Colors(ids) => ids.clone(),
            Ids::Copper(weathering, waxed) => {
                weathering.iter().chain(waxed.iter()).cloned().collect()
            }
        }
    }
}

fn colored(base: &str) -> Ids {
    Ids::Colors(
        DYE_COLORS
            .iter()
            .map(|color| namespaced(&format!("{color}_{base}")))
            .collect(),
    )
}

fn copper(names: &[String]) -> Ids {
    let weathering = COPPER_STATES
        .iter()
        .zip(names)
        .map(|(state, name)| namespaced(&format!("{state}{name}")))
        .collect();
    let waxed = COPPER_STATES
        .iter()
        .zip(names)
        .map(|(state, name)| namespaced(&format!("waxed_{state}{name}")))
        .collect();
    Ids::Copper(weathering, waxed)
}

#[derive(Clone, Debug, Default)]
struct Definition {
    strings: Vec<String>,
    states: Option<Vec<String>>,
    uses: Vec<String>,
}

fn definitions(class: &ClassFile) -> Result<HashMap<String, Definition>> {
    let mut out = HashMap::new();
    let Some(code) = class.method("<clinit>").and_then(|m| m.code.as_ref()) else {
        return Ok(out);
    };
    let mut current = Definition::default();
    for ins in instructions(code)? {
        match ins.op {
            LDC | LDC_W => {
                if let Some(text) = class.string(ins.index) {
                    current.strings.push(text.to_owned());
                }
            }
            GETSTATIC => {
                if let Some(member) = class.member(ins.index) {
                    if member.class == class.name {
                        current.uses.push(member.name);
                    }
                }
            }
            INVOKESPECIAL => {
                if let Some(member) = class.member(ins.index) {
                    if member.class == BY_STATE
                        && member.name == "<init>"
                        && current.strings.len() >= 4
                    {
                        let at = current.strings.len() - 4;
                        current.states = Some(current.strings[at..].to_vec());
                    }
                }
            }
            PUTSTATIC => {
                if let Some(member) = class.member(ins.index) {
                    if member.class == class.name {
                        out.insert(member.name, std::mem::take(&mut current));
                    }
                }
            }
            _ => {}
        }
    }
    let inherited: Vec<(String, Vec<String>)> = out
        .iter()
        .filter(|(_, definition)| definition.states.is_none())
        .filter_map(|(name, definition)| {
            let states = definition
                .uses
                .iter()
                .find_map(|used| out.get(used).and_then(|d| d.states.clone()))?;
            Some((name.clone(), states))
        })
        .collect();
    for (name, states) in inherited {
        if let Some(definition) = out.get_mut(&name) {
            definition.states = Some(states);
        }
    }
    Ok(out)
}

struct Names {
    items: HashMap<String, Ids>,
    blocks: HashMap<String, Ids>,
    order: Vec<String>,
}

impl Names {
    fn read(
        load: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
        known: &HashSet<String>,
    ) -> Result<Names> {
        let mut references: HashMap<String, HashMap<String, Definition>> = HashMap::new();
        let items = parse(load, ITEMS_CLASS)?;
        let (items, fields) = owner_ids(&items, load, &mut references, known)?;
        let blocks = match parse(load, BLOCKS_CLASS) {
            Ok(blocks) => owner_ids(&blocks, load, &mut references, known)?.0,
            Err(_) => HashMap::new(),
        };
        let mut seen = HashSet::new();
        let mut order = Vec::new();
        for field in fields {
            for id in items.get(&field).into_iter().flat_map(Ids::flatten) {
                if known.contains(&id) && seen.insert(id.clone()) {
                    order.push(id);
                }
            }
        }
        Ok(Names {
            items,
            blocks,
            order,
        })
    }
}

type Owned = (HashMap<String, Ids>, Vec<String>);

fn owner_ids(
    class: &ClassFile,
    load: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    references: &mut HashMap<String, HashMap<String, Definition>>,
    known: &HashSet<String>,
) -> Result<Owned> {
    let code = class
        .method("<clinit>")
        .and_then(|m| m.code.as_ref())
        .ok_or_else(|| anyhow!("{} has no static initializer", class.name))?;
    let mut map = HashMap::new();
    let mut fields = Vec::new();
    let mut reference: Option<Member> = None;
    for ins in instructions(code)? {
        match ins.op {
            GETSTATIC => {
                if let Some(member) = class.member(ins.index) {
                    if reference.is_none() && member.class.starts_with(REFERENCES) {
                        reference = Some(member);
                    }
                }
            }
            PUTSTATIC => {
                let Some(field) = class.member(ins.index) else {
                    continue;
                };
                if field.class != class.name {
                    continue;
                }
                let definition = match &reference {
                    Some(r) => {
                        if !references.contains_key(&r.class) {
                            let parsed = match load(&r.class) {
                                Some(bytes) => definitions(&ClassFile::parse(&bytes)?)?,
                                None => HashMap::new(),
                            };
                            references.insert(r.class.clone(), parsed);
                        }
                        references.get(&r.class).and_then(|d| d.get(&r.name))
                    }
                    None => None,
                };
                if let Some(ids) = resolve(&field, reference.as_ref(), definition, known) {
                    map.insert(field.name.clone(), ids);
                    fields.push(field.name);
                }
                reference = None;
            }
            _ => {}
        }
    }
    Ok((map, fields))
}

fn resolve(
    field: &Member,
    reference: Option<&Member>,
    definition: Option<&Definition>,
    known: &HashSet<String>,
) -> Option<Ids> {
    let lower = field.name.to_ascii_lowercase();
    let last = definition.and_then(|d| d.strings.last()).cloned();
    match field.descriptor.as_str() {
        COLORS_DESCRIPTOR => Some(colored(&last.unwrap_or(lower))),
        COPPER_DESCRIPTOR => {
            let names = definition
                .and_then(|d| d.states.clone())
                .unwrap_or_else(|| vec![last.unwrap_or(lower); 4]);
            Some(copper(&names))
        }
        ITEM_DESCRIPTOR | BLOCK_DESCRIPTOR => std::iter::once(lower)
            .chain(
                definition
                    .into_iter()
                    .flat_map(|d| d.strings.iter().rev().cloned()),
            )
            .chain(reference.map(|r| r.name.to_ascii_lowercase()))
            .map(|candidate| namespaced(&candidate))
            .find(|id| known.contains(id))
            .map(Ids::One),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Unknown,
    Int(i32),
    Str(String),
    New(usize),
    Array(usize),
    Item(String, Option<Variants>),
    Missing(String),
    Colors(Vec<String>),
    Copper(Vec<String>, Vec<String>),
    States(Vec<String>),
    Color(String),
    ColorList(Vec<String>),
    Lambda(Box<Lambda>),
    Output,
    Parameters,
    Holder,
    Lookup(String),
    Key(String),
    Tag(String),
    Row(Row),
    Kind(TabKind),
    Visibility(bool, bool),
    Texture(String),
    Builder(usize),
}

#[derive(Clone, Debug, PartialEq)]
struct Lambda {
    target: Member,
    kind: u8,
    captured: Vec<Value>,
}

#[derive(Clone, Debug)]
struct Draft {
    row: Option<Row>,
    column: u8,
    title: Option<String>,
    icon: Option<Lambda>,
    display: Option<Lambda>,
    aligned_right: bool,
    show_title: bool,
    scroll_bar: bool,
    kind: TabKind,
    background: Option<String>,
}

struct Machine<'a> {
    class: &'a ClassFile,
    names: &'a Names,
    known: &'a HashSet<String>,
    statics: HashMap<String, Value>,
    arrays: Vec<Vec<Value>>,
    allocated: Vec<String>,
    drafts: Vec<Draft>,
    registered: Vec<(String, usize)>,
    emitted: Vec<TabEntry>,
    skipped: Vec<String>,
    holder_is: i32,
    tag: Option<String>,
    asked_permission: bool,
    depth: usize,
}

fn pop(stack: &mut Vec<Value>) -> Result<Value> {
    stack
        .pop()
        .ok_or_else(|| anyhow!("the operand stack runs empty"))
}

fn pops(stack: &mut Vec<Value>, n: usize) -> Result<Vec<Value>> {
    if stack.len() < n {
        bail!("the operand stack runs empty");
    }
    Ok(stack.split_off(stack.len() - n))
}

fn compare(op: u8, a: i32, b: i32) -> bool {
    match op {
        0x99 | 0x9f => a == b,
        0x9a | 0xa0 => a != b,
        0x9b | 0xa1 => a < b,
        0x9c | 0xa2 => a >= b,
        0x9d | 0xa3 => a > b,
        _ => a <= b,
    }
}

impl<'a> Machine<'a> {
    fn new(class: &'a ClassFile, names: &'a Names, known: &'a HashSet<String>) -> Machine<'a> {
        Machine {
            class,
            names,
            known,
            statics: HashMap::new(),
            arrays: Vec::new(),
            allocated: Vec::new(),
            drafts: Vec::new(),
            registered: Vec::new(),
            emitted: Vec::new(),
            skipped: Vec::new(),
            holder_is: 0,
            tag: None,
            asked_permission: false,
            depth: 0,
        }
    }

    fn run(&mut self, method: &Method, args: Vec<Value>) -> Result<Value> {
        if self.depth >= DEPTH_LIMIT {
            bail!("{} nests too deeply", method.name);
        }
        let code = method
            .code
            .as_ref()
            .ok_or_else(|| anyhow!("{} has no code", method.name))?;
        let program = instructions(code)?;
        let mut slots = Vec::new();
        let mut slot = 0usize;
        for parameter in parameters(&method.descriptor) {
            slots.push(slot);
            slot += if parameter == "J" || parameter == "D" {
                2
            } else {
                1
            };
        }
        let mut locals = vec![Value::Unknown; slot.max(method.max_locals as usize)];
        for (slot, value) in slots.into_iter().zip(args) {
            locals[slot] = value;
        }
        self.depth += 1;
        let result = self
            .execute(&program, locals)
            .with_context(|| method.name.clone());
        self.depth -= 1;
        result
    }

    fn execute(&mut self, program: &[Instruction], mut locals: Vec<Value>) -> Result<Value> {
        let positions: HashMap<usize, usize> = program
            .iter()
            .enumerate()
            .map(|(i, ins)| (ins.at, i))
            .collect();
        let mut stack: Vec<Value> = Vec::new();
        let mut pc = 0;
        while pc < program.len() {
            let ins = program[pc];
            let mut next = pc + 1;
            match ins.op {
                0x00 | 0xc0 => {}
                0x01 | 0x09..=0x0f | LDC2_W => stack.push(Value::Unknown),
                ICONST_M1..=ICONST_5 | BIPUSH | SIPUSH => stack.push(Value::Int(ins.value)),
                LDC | LDC_W => stack.push(match self.class.string(ins.index) {
                    Some(text) => Value::Str(text.to_owned()),
                    None => match self.class.integer(ins.index) {
                        Some(value) => Value::Int(value),
                        None => Value::Unknown,
                    },
                }),
                0x15..=0x2d => stack.push(
                    locals
                        .get(ins.index as usize)
                        .cloned()
                        .unwrap_or(Value::Unknown),
                ),
                0x2e..=0x35 => {
                    let index = pop(&mut stack)?;
                    let array = pop(&mut stack)?;
                    stack.push(match (array, index) {
                        (Value::Array(id), Value::Int(i)) => self
                            .arrays
                            .get(id)
                            .and_then(|a| a.get(i as usize))
                            .cloned()
                            .unwrap_or(Value::Unknown),
                        _ => Value::Unknown,
                    });
                }
                0x36..=0x4e => {
                    let value = pop(&mut stack)?;
                    let slot = ins.index as usize;
                    if slot >= locals.len() {
                        locals.resize(slot + 1, Value::Unknown);
                    }
                    locals[slot] = value;
                }
                0x4f..=0x56 => {
                    let value = pop(&mut stack)?;
                    let index = pop(&mut stack)?;
                    let array = pop(&mut stack)?;
                    if let (Value::Array(id), Value::Int(i)) = (array, index) {
                        if let Some(slot) =
                            self.arrays.get_mut(id).and_then(|a| a.get_mut(i as usize))
                        {
                            *slot = value;
                        }
                    }
                }
                0x57 => {
                    pop(&mut stack)?;
                }
                0x58 => {
                    pops(&mut stack, 2)?;
                }
                0x59 => {
                    let top = pop(&mut stack)?;
                    stack.push(top.clone());
                    stack.push(top);
                }
                0x5a => {
                    let a = pop(&mut stack)?;
                    let b = pop(&mut stack)?;
                    stack.extend([a.clone(), b, a]);
                }
                0x5b => {
                    let a = pop(&mut stack)?;
                    let b = pop(&mut stack)?;
                    let c = pop(&mut stack)?;
                    stack.extend([a.clone(), c, b, a]);
                }
                0x5c => {
                    let top = pops(&mut stack, 2)?;
                    stack.extend(top.iter().cloned());
                    stack.extend(top);
                }
                0x5d => {
                    let top = pops(&mut stack, 2)?;
                    let c = pop(&mut stack)?;
                    stack.extend(top.iter().cloned());
                    stack.push(c);
                    stack.extend(top);
                }
                0x5e => {
                    let top = pops(&mut stack, 2)?;
                    let under = pops(&mut stack, 2)?;
                    stack.extend(top.iter().cloned());
                    stack.extend(under);
                    stack.extend(top);
                }
                0x5f => {
                    let a = pop(&mut stack)?;
                    let b = pop(&mut stack)?;
                    stack.extend([a, b]);
                }
                0x60..=0x73 | 0x78..=0x83 | 0x94..=0x98 => {
                    let b = pop(&mut stack)?;
                    let a = pop(&mut stack)?;
                    stack.push(match (ins.op, a, b) {
                        (0x60, Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_add(b)),
                        (0x64, Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_sub(b)),
                        (0x68, Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_mul(b)),
                        _ => Value::Unknown,
                    });
                }
                0x74..=0x77 | 0x85..=0x93 | 0xbc | 0xc1 => {
                    pop(&mut stack)?;
                    stack.push(Value::Unknown);
                }
                IINC => {
                    if let Some(Value::Int(value)) = locals.get_mut(ins.index as usize) {
                        *value = value.wrapping_add(ins.value);
                    } else if let Some(slot) = locals.get_mut(ins.index as usize) {
                        *slot = Value::Unknown;
                    }
                }
                0x99..=0x9e | 0xc6 | 0xc7 => {
                    let value = pop(&mut stack)?;
                    if let (Value::Int(a), 0x99..=0x9e) = (&value, ins.op) {
                        if ins.value as usize > ins.at && compare(ins.op, *a, 0) {
                            next = self.jump(&positions, ins.value)?;
                        }
                    }
                }
                0x9f..=0xa6 => {
                    let b = pop(&mut stack)?;
                    let a = pop(&mut stack)?;
                    if let (Value::Int(a), Value::Int(b), 0x9f..=0xa4) = (a, b, ins.op) {
                        if ins.value as usize > ins.at && compare(ins.op, a, b) {
                            next = self.jump(&positions, ins.value)?;
                        }
                    }
                }
                GOTO | GOTO_W => {
                    if ins.value as usize > ins.at {
                        next = self.jump(&positions, ins.value)?;
                    }
                }
                0xa8 | 0xa9 | 0xc9 => bail!("subroutines are not supported"),
                TABLESWITCH | LOOKUPSWITCH => bail!("switches are not supported"),
                0xac..=0xb0 => return pop(&mut stack),
                0xb1 | 0xbf => return Ok(Value::Unknown),
                GETSTATIC => {
                    let member = self.member(ins.index)?;
                    stack.push(self.get_static(&member));
                }
                PUTSTATIC => {
                    let member = self.member(ins.index)?;
                    let value = pop(&mut stack)?;
                    if member.class == self.class.name {
                        self.statics.insert(member.name, value);
                    }
                }
                GETFIELD => {
                    pop(&mut stack)?;
                    stack.push(Value::Unknown);
                }
                PUTFIELD => {
                    pops(&mut stack, 2)?;
                }
                INVOKEVIRTUAL | INVOKESPECIAL | INVOKESTATIC | INVOKEINTERFACE => {
                    let member = self.member(ins.index)?;
                    let args = pops(&mut stack, parameters(&member.descriptor).len())?;
                    let receiver = match ins.op {
                        INVOKESTATIC => None,
                        _ => Some(pop(&mut stack)?),
                    };
                    if member.name == "<init>" {
                        if let Some(Value::New(id)) = receiver {
                            let built = match (self.allocated.get(id), args.first()) {
                                (Some(class), Some(Value::Item(item, variants)))
                                    if class == ITEM_STACK =>
                                {
                                    Value::Item(item.clone(), variants.clone())
                                }
                                _ => Value::Unknown,
                            };
                            for value in stack.iter_mut().chain(locals.iter_mut()) {
                                if *value == Value::New(id) {
                                    *value = built.clone();
                                }
                            }
                        }
                        pc = next;
                        continue;
                    }
                    let result = self.invoke(&member, receiver, args)?;
                    if returns_value(&member.descriptor) {
                        stack.push(result);
                    }
                }
                INVOKEDYNAMIC => {
                    let dynamic = self
                        .class
                        .invoke_dynamic(ins.index)
                        .ok_or_else(|| anyhow!("invokedynamic #{} is unreadable", ins.index))?;
                    let captured = pops(&mut stack, parameters(&dynamic.descriptor).len())?;
                    let handle = (dynamic.bootstrap.member.class == LAMBDA_FACTORY)
                        .then(|| dynamic.arguments.get(1).and_then(|i| self.class.handle(*i)))
                        .flatten();
                    stack.push(match handle {
                        Some(handle) => Value::Lambda(Box::new(Lambda {
                            target: handle.member,
                            kind: handle.kind,
                            captured,
                        })),
                        None => Value::Unknown,
                    });
                }
                NEW => {
                    let class = self.class.class_name(ins.index).unwrap_or("").to_owned();
                    self.allocated.push(class);
                    stack.push(Value::New(self.allocated.len() - 1));
                }
                0xbd => {
                    let count = pop(&mut stack)?;
                    stack.push(match count {
                        Value::Int(n) if (0..=4096).contains(&n) => {
                            self.arrays.push(vec![Value::Unknown; n as usize]);
                            Value::Array(self.arrays.len() - 1)
                        }
                        _ => Value::Unknown,
                    });
                }
                0xbe => {
                    let array = pop(&mut stack)?;
                    stack.push(match array {
                        Value::Array(id) => Value::Int(self.arrays[id].len() as i32),
                        _ => Value::Unknown,
                    });
                }
                0xc2 | 0xc3 => {
                    pop(&mut stack)?;
                }
                0xc5 => {
                    let dimensions = program[pc].value.max(0) as usize;
                    pops(&mut stack, dimensions.max(1))?;
                    stack.push(Value::Unknown);
                }
                other => bail!("opcode {other:#04x} is not supported"),
            }
            pc = next;
        }
        Ok(Value::Unknown)
    }

    fn jump(&self, positions: &HashMap<usize, usize>, target: i32) -> Result<usize> {
        positions
            .get(&(target as usize))
            .copied()
            .ok_or_else(|| anyhow!("a jump lands inside an instruction"))
    }

    fn member(&self, index: u16) -> Result<Member> {
        self.class
            .member(index)
            .ok_or_else(|| anyhow!("constant #{index} is not a member"))
    }

    fn get_static(&self, member: &Member) -> Value {
        let name = member.name.as_str();
        let lower = name.to_ascii_lowercase();
        let ids = |map: &HashMap<String, Ids>| match map.get(name) {
            Some(Ids::One(id)) => Value::Item(id.clone(), None),
            Some(Ids::Colors(ids)) => Value::Colors(ids.clone()),
            Some(Ids::Copper(weathering, waxed)) => {
                Value::Copper(weathering.clone(), waxed.clone())
            }
            None => Value::Missing(format!("{}.{name}", member.class)),
        };
        match member.class.as_str() {
            class if class == self.class.name => {
                self.statics.get(name).cloned().unwrap_or(Value::Unknown)
            }
            ITEMS_CLASS => ids(&self.names.items),
            BLOCKS_CLASS => ids(&self.names.blocks),
            DYE_COLOR => Value::Color(lower),
            ROW => match name {
                "TOP" => Value::Row(Row::Top),
                "BOTTOM" => Value::Row(Row::Bottom),
                _ => Value::Unknown,
            },
            TYPE => match name {
                "CATEGORY" => Value::Kind(TabKind::Category),
                "HOTBAR" => Value::Kind(TabKind::Hotbar),
                "SEARCH" => Value::Kind(TabKind::Search),
                "INVENTORY" => Value::Kind(TabKind::Inventory),
                _ => Value::Unknown,
            },
            VISIBILITY => match name {
                "PARENT_AND_SEARCH_TABS" => Value::Visibility(true, true),
                "PARENT_TAB_ONLY" => Value::Visibility(true, false),
                "SEARCH_TAB_ONLY" => Value::Visibility(false, true),
                _ => Value::Unknown,
            },
            "net/minecraft/core/registries/Registries" => Value::Key(namespaced(&lower)),
            class if class.ends_with("Tags") => Value::Tag(namespaced(&lower)),
            _ => Value::Unknown,
        }
    }

    fn apply(&mut self, lambda: &Lambda, args: Vec<Value>) -> Result<Value> {
        let mut all = lambda.captured.clone();
        all.extend(args);
        if lambda.target.class == self.class.name {
            let class = self.class;
            let method = class
                .method_with(&lambda.target.name, &lambda.target.descriptor)
                .ok_or_else(|| anyhow!("{} is missing", lambda.target.name))?;
            return self.run(method, all);
        }
        let receiver = match lambda.kind {
            5 | 7 | 9 if !all.is_empty() => Some(all.remove(0)),
            _ => None,
        };
        self.invoke(&lambda.target.clone(), receiver, all)
    }

    fn invoke(
        &mut self,
        member: &Member,
        receiver: Option<Value>,
        args: Vec<Value>,
    ) -> Result<Value> {
        let name = member.name.as_str();
        if member.class == self.class.name {
            if name.starts_with("generate") {
                self.generate(name, &args);
                return Ok(Value::Unknown);
            }
            let class = self.class;
            let method = class
                .method_with(name, &member.descriptor)
                .ok_or_else(|| anyhow!("{name} is missing"))?;
            let mut all: Vec<Value> = receiver.into_iter().collect();
            all.extend(args);
            return self.run(method, all);
        }
        let first = args.first().cloned().unwrap_or(Value::Unknown);
        Ok(match (member.class.as_str(), name) {
            (OUTPUT, "accept") => {
                let (in_tab, in_search) = match args.get(1) {
                    Some(Value::Visibility(tab, search)) => (*tab, *search),
                    _ => (true, true),
                };
                self.accept(first, in_tab, in_search);
                Value::Unknown
            }
            (OUTPUT, "acceptAll") => {
                self.skipped
                    .push("a collection of stacks the reader cannot follow".to_owned());
                Value::Unknown
            }
            (TAB, "builder") => {
                let row = match first {
                    Value::Row(row) => Some(row),
                    _ => None,
                };
                let column = match args.get(1) {
                    Some(Value::Int(column)) => (*column).clamp(0, 255) as u8,
                    _ => 0,
                };
                self.drafts.push(Draft {
                    row,
                    column,
                    title: None,
                    icon: None,
                    display: None,
                    aligned_right: false,
                    show_title: true,
                    scroll_bar: true,
                    kind: TabKind::Category,
                    background: None,
                });
                Value::Builder(self.drafts.len() - 1)
            }
            (TAB, "createTextureLocation") => match first {
                Value::Str(name) => Value::Texture(name),
                _ => Value::Unknown,
            },
            (BUILDER, _) => self.build(receiver, name, first),
            ("net/minecraft/core/Registry", "register") => {
                if let (Some(Value::Key(id)), Some(Value::Builder(draft))) =
                    (args.get(1), args.get(2))
                {
                    self.registered.push((id.clone(), *draft));
                }
                args.get(2).cloned().unwrap_or(Value::Unknown)
            }
            ("net/minecraft/network/chat/Component", "translatable") => first,
            ("net/minecraft/resources/Identifier", "withDefaultNamespace") => match first {
                Value::Str(path) => Value::Str(namespaced(&path)),
                _ => Value::Unknown,
            },
            ("net/minecraft/resources/ResourceKey", "create") => match args.get(1) {
                Some(Value::Str(id)) => Value::Key(namespaced(id)),
                _ => Value::Unknown,
            },
            ("java/util/List", "of") => match first {
                Value::Array(id) if args.len() == 1 => self.color_list(&self.arrays[id].clone()),
                _ => self.color_list(&args),
            },
            ("java/util/List" | "java/lang/Iterable", "forEach") => {
                if let (Some(Value::ColorList(colors)), Value::Lambda(lambda)) = (&receiver, &first)
                {
                    for color in colors.clone() {
                        self.apply(lambda, vec![Value::Color(color)])?;
                    }
                }
                Value::Unknown
            }
            ("java/util/Objects", "requireNonNull") => first,
            ("java/util/function/Consumer", "accept") => match receiver {
                Some(Value::Lambda(lambda)) => {
                    self.apply(&lambda, args)?;
                    Value::Unknown
                }
                _ => Value::Unknown,
            },
            ("java/util/function/Supplier", "get") => match receiver {
                Some(Value::Lambda(lambda)) => self.apply(&lambda, Vec::new())?,
                _ => Value::Unknown,
            },
            ("java/util/Optional", "ifPresent") => {
                if let Value::Lambda(lambda) = &first {
                    self.apply(lambda, vec![receiver.unwrap_or(Value::Unknown)])?;
                }
                Value::Unknown
            }
            ("net/minecraft/core/HolderLookup$Provider", "lookup" | "lookupOrThrow") => match first
            {
                Value::Key(key) => Value::Lookup(key),
                _ => Value::Unknown,
            },
            (PARAMETERS, "hasPermissions") => {
                self.asked_permission = true;
                Value::Unknown
            }
            (COLORS, "forEach") => {
                if let (Some(Value::Colors(ids)), Value::Lambda(lambda)) = (&receiver, &first) {
                    for id in ids.clone() {
                        self.apply(lambda, vec![Value::Item(id, None)])?;
                    }
                }
                Value::Unknown
            }
            (COLORS, "pick") => match (&receiver, &first) {
                (Some(Value::Colors(ids)), Value::Color(color)) => color_index(color)
                    .and_then(|i| ids.get(i))
                    .map(|id| Value::Item(id.clone(), None))
                    .unwrap_or(Value::Unknown),
                _ => Value::Unknown,
            },
            (COLORS, accessor) => match (&receiver, color_index(accessor)) {
                (Some(Value::Colors(ids)), Some(i)) if args.is_empty() => ids
                    .get(i)
                    .map(|id| Value::Item(id.clone(), None))
                    .unwrap_or(Value::Unknown),
                _ => Value::Unknown,
            },
            (COPPER, "forEach") => {
                if let (Some(Value::Copper(weathering, waxed)), Value::Lambda(lambda)) =
                    (&receiver, &first)
                {
                    for id in weathering
                        .iter()
                        .chain(waxed.iter())
                        .cloned()
                        .collect::<Vec<_>>()
                    {
                        self.apply(lambda, vec![Value::Item(id, None)])?;
                    }
                }
                Value::Unknown
            }
            (COPPER, "weathering") => match receiver {
                Some(Value::Copper(weathering, _)) => Value::States(weathering),
                _ => Value::Unknown,
            },
            (COPPER, "waxed") => match receiver {
                Some(Value::Copper(_, waxed)) => Value::States(waxed),
                _ => Value::Unknown,
            },
            (BY_STATE, "forEach") => {
                if let (Some(Value::States(ids)), Value::Lambda(lambda)) = (&receiver, &first) {
                    for id in ids.clone() {
                        self.apply(lambda, vec![Value::Item(id, None)])?;
                    }
                }
                Value::Unknown
            }
            (BY_STATE, accessor) => match (
                &receiver,
                BY_STATE_ACCESSORS.iter().position(|a| *a == accessor),
            ) {
                (Some(Value::States(ids)), Some(i)) => ids
                    .get(i)
                    .map(|id| Value::Item(id.clone(), None))
                    .unwrap_or(Value::Unknown),
                _ => Value::Unknown,
            },
            ("net/minecraft/world/entity/raid/Raid", "getOminousBannerInstance") => Value::Item(
                "minecraft:white_banner".to_owned(),
                Some(Variants::OminousBanner),
            ),
            ("net/minecraft/world/level/block/TestBlock", "setModeOnStack") => {
                with_variants(first, Variants::TestBlockModes)
            }
            ("net/minecraft/world/level/block/LightBlock", "setLightOnStack") => {
                with_variants(first, Variants::LightLevels)
            }
            ("net/minecraft/core/Holder", "is") => {
                if let Value::Tag(tag) = first {
                    self.tag = Some(tag);
                }
                Value::Int(self.holder_is)
            }
            _ => Value::Unknown,
        })
    }

    fn color_list(&self, values: &[Value]) -> Value {
        let colors: Option<Vec<String>> = values
            .iter()
            .map(|value| match value {
                Value::Color(color) => Some(color.clone()),
                _ => None,
            })
            .collect();
        match colors {
            Some(colors) if !colors.is_empty() => Value::ColorList(colors),
            _ => Value::Unknown,
        }
    }

    fn build(&mut self, receiver: Option<Value>, name: &str, argument: Value) -> Value {
        let Some(Value::Builder(id)) = receiver else {
            return Value::Unknown;
        };
        let draft = &mut self.drafts[id];
        match (name, argument) {
            ("title", Value::Str(title)) => draft.title = Some(title),
            ("icon", Value::Lambda(lambda)) => draft.icon = Some(*lambda),
            ("displayItems", Value::Lambda(lambda)) => draft.display = Some(*lambda),
            ("alignedRight", _) => draft.aligned_right = true,
            ("hideTitle", _) => draft.show_title = false,
            ("noScrollBar", _) => draft.scroll_bar = false,
            ("type", Value::Kind(kind)) => draft.kind = kind,
            ("backgroundTexture", Value::Texture(texture)) => draft.background = Some(texture),
            _ => {}
        }
        Value::Builder(id)
    }

    fn accept(&mut self, value: Value, in_tab: bool, in_search: bool) {
        match value {
            Value::Item(item, variants) => self.emit(item, variants, in_tab, in_search),
            Value::Missing(field) => self.skipped.push(format!("{field} has no known item id")),
            other => self.skipped.push(format!(
                "an accepted value the reader cannot follow ({other:?})"
            )),
        }
    }

    fn emit(&mut self, item: String, variants: Option<Variants>, in_tab: bool, in_search: bool) {
        if !self.known.contains(&item) {
            self.skipped.push(format!("{item} is not a known item"));
            return;
        }
        self.emitted.push(TabEntry {
            item,
            variants,
            in_tab,
            in_search,
        });
    }

    fn generate(&mut self, name: &str, args: &[Value]) {
        let (in_tab, in_search) = args
            .iter()
            .find_map(|arg| match arg {
                Value::Visibility(tab, search) => Some((*tab, *search)),
                _ => None,
            })
            .unwrap_or((true, true));
        let item = args.iter().find_map(|arg| match arg {
            Value::Item(item, None) => Some(item.clone()),
            _ => None,
        });
        let tag = args.iter().find_map(|arg| match arg {
            Value::Tag(tag) => Some(tag.clone()),
            _ => None,
        });
        let generated = match name {
            "generatePotionEffectTypes" => item.map(|item| (Variants::Potions, item)),
            "generateEnchantmentBookTypesOnlyMaxLevel" => Some((
                Variants::EnchantedBooks { all_levels: false },
                "minecraft:enchanted_book".to_owned(),
            )),
            "generateEnchantmentBookTypesAllLevels" => Some((
                Variants::EnchantedBooks { all_levels: true },
                "minecraft:enchanted_book".to_owned(),
            )),
            "generateInstrumentTypes" => item
                .zip(tag)
                .map(|(item, tag)| (Variants::Instruments { tag }, item)),
            "generateSuspiciousStews" => Some((
                Variants::SuspiciousStews,
                "minecraft:suspicious_stew".to_owned(),
            )),
            "generateOminousBottles" => Some((
                Variants::OminousBottles,
                "minecraft:ominous_bottle".to_owned(),
            )),
            "generateFireworksAllDurations" => {
                Some((Variants::Fireworks, "minecraft:firework_rocket".to_owned()))
            }
            "generatePresetPaintings" => self
                .paintings(args)
                .map(|variants| (variants, "minecraft:painting".to_owned())),
            _ => None,
        };
        match generated {
            Some((variants, item)) => self.emit(item, Some(variants), in_tab, in_search),
            None => self
                .skipped
                .push(format!("{name} is a generator the reader cannot follow")),
        }
    }

    fn paintings(&mut self, args: &[Value]) -> Option<Variants> {
        let predicate = args.iter().find_map(|arg| match arg {
            Value::Lambda(lambda) => Some((**lambda).clone()),
            _ => None,
        })?;
        self.tag = None;
        let inside = self.test(&predicate, 1)?;
        let outside = self.test(&predicate, 0)?;
        let tag = self.tag.take()?;
        match (inside, outside) {
            (true, false) => Some(Variants::Paintings { tag, in_tag: true }),
            (false, true) => Some(Variants::Paintings { tag, in_tag: false }),
            _ => None,
        }
    }

    fn test(&mut self, predicate: &Lambda, holder_is: i32) -> Option<bool> {
        self.holder_is = holder_is;
        match self.apply(predicate, vec![Value::Holder]) {
            Ok(Value::Int(value)) => Some(value != 0),
            _ => None,
        }
    }
}

fn with_variants(value: Value, variants: Variants) -> Value {
    match value {
        Value::Item(item, _) => Value::Item(item, Some(variants)),
        _ => Value::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM_IDS: &str = "net/minecraft/references/ItemIds";
    const KEY: &str = "Lnet/minecraft/resources/ResourceKey;";
    const VISIBILITY_DESCRIPTOR: &str = "Lnet/minecraft/world/item/CreativeModeTab$TabVisibility;";
    const BUILDER_DESCRIPTOR: &str = "Lnet/minecraft/world/item/CreativeModeTab$Builder;";
    const OUTPUT_DESCRIPTOR: &str = "Lnet/minecraft/world/item/CreativeModeTab$Output;";

    #[derive(Default)]
    struct Code(Vec<u8>);

    impl Code {
        fn op(&mut self, op: u8) -> &mut Code {
            self.0.push(op);
            self
        }

        fn with(&mut self, op: u8, index: u16) -> &mut Code {
            self.0.push(op);
            self.0.extend(index.to_be_bytes());
            self
        }

        fn ldc(&mut self, index: u16) -> &mut Code {
            self.with(LDC_W, index)
        }

        fn interface(&mut self, index: u16, count: u8) -> &mut Code {
            self.with(INVOKEINTERFACE, index);
            self.0.extend([count, 0]);
            self
        }

        fn dynamic(&mut self, index: u16) -> &mut Code {
            self.with(INVOKEDYNAMIC, index);
            self.0.extend([0, 0]);
            self
        }

        fn here(&self) -> usize {
            self.0.len()
        }

        fn branch(&mut self, op: u8) -> usize {
            let at = self.here();
            self.with(op, 0);
            at
        }

        fn land(&mut self, at: usize) {
            let offset = (self.here() - at) as i16;
            self.0[at + 1..at + 3].copy_from_slice(&offset.to_be_bytes());
        }

        fn back(&mut self, op: u8, target: usize) {
            let offset = (target as i64 - self.here() as i64) as i16;
            self.with(op, offset as u16);
        }
    }

    struct Class {
        name: u16,
        pool: Vec<u8>,
        count: u16,
        methods: Vec<u8>,
        method_count: u16,
        bootstraps: Vec<(u16, u16)>,
    }

    impl Class {
        fn new(name: &str) -> Class {
            let mut class = Class {
                name: 0,
                pool: Vec::new(),
                count: 0,
                methods: Vec::new(),
                method_count: 0,
                bootstraps: Vec::new(),
            };
            class.name = class.class(name);
            class
        }

        fn add(&mut self, entry: &[u8]) -> u16 {
            self.count += 1;
            self.pool.extend_from_slice(entry);
            self.count
        }

        fn utf8(&mut self, text: &str) -> u16 {
            let mut entry = vec![1];
            entry.extend((text.len() as u16).to_be_bytes());
            entry.extend(text.as_bytes());
            self.add(&entry)
        }

        fn pair(&mut self, tag: u8, a: u16, b: u16) -> u16 {
            let mut entry = vec![tag];
            entry.extend(a.to_be_bytes());
            entry.extend(b.to_be_bytes());
            self.add(&entry)
        }

        fn class(&mut self, name: &str) -> u16 {
            let name = self.utf8(name);
            let mut entry = vec![7];
            entry.extend(name.to_be_bytes());
            self.add(&entry)
        }

        fn string(&mut self, text: &str) -> u16 {
            let text = self.utf8(text);
            let mut entry = vec![8];
            entry.extend(text.to_be_bytes());
            self.add(&entry)
        }

        fn member(&mut self, tag: u8, owner: &str, name: &str, descriptor: &str) -> u16 {
            let owner = self.class(owner);
            let name = self.utf8(name);
            let descriptor = self.utf8(descriptor);
            let nat = self.pair(12, name, descriptor);
            self.pair(tag, owner, nat)
        }

        fn field(&mut self, owner: &str, name: &str, descriptor: &str) -> u16 {
            self.member(9, owner, name, descriptor)
        }

        fn method_ref(&mut self, owner: &str, name: &str, descriptor: &str) -> u16 {
            self.member(10, owner, name, descriptor)
        }

        fn interface(&mut self, owner: &str, name: &str, descriptor: &str) -> u16 {
            self.member(11, owner, name, descriptor)
        }

        fn handle(&mut self, kind: u8, member: u16) -> u16 {
            let mut entry = vec![15, kind];
            entry.extend(member.to_be_bytes());
            self.add(&entry)
        }

        fn lambda(&mut self, name: &str, descriptor: &str, kind: u8, target: u16) -> u16 {
            let factory = self.method_ref(
                LAMBDA_FACTORY,
                "metafactory",
                "()Ljava/lang/invoke/CallSite;",
            );
            let factory = self.handle(6, factory);
            let target = self.handle(kind, target);
            self.bootstraps.push((factory, target));
            let bootstrap = (self.bootstraps.len() - 1) as u16;
            let name = self.utf8(name);
            let descriptor = self.utf8(descriptor);
            let nat = self.pair(12, name, descriptor);
            self.pair(18, bootstrap, nat)
        }

        fn method(&mut self, name: &str, descriptor: &str, code: &Code) {
            let name = self.utf8(name);
            let descriptor = self.utf8(descriptor);
            let attribute = self.utf8("Code");
            self.method_count += 1;
            self.methods.extend([0, 0x0a]);
            self.methods.extend(name.to_be_bytes());
            self.methods.extend(descriptor.to_be_bytes());
            self.methods.extend([0, 1]);
            self.methods.extend(attribute.to_be_bytes());
            self.methods
                .extend(((12 + code.0.len()) as u32).to_be_bytes());
            self.methods.extend([0, 8, 0, 8]);
            self.methods.extend((code.0.len() as u32).to_be_bytes());
            self.methods.extend(&code.0);
            self.methods.extend([0, 0, 0, 0]);
        }

        fn bytes(mut self) -> Vec<u8> {
            let attribute = self.utf8("BootstrapMethods");
            let mut out = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65];
            out.extend((self.count + 1).to_be_bytes());
            out.extend(&self.pool);
            out.extend([0, 0x21]);
            out.extend(self.name.to_be_bytes());
            out.extend([0, 0, 0, 0, 0, 0]);
            out.extend(self.method_count.to_be_bytes());
            out.extend(&self.methods);
            out.extend([0, 1]);
            out.extend(attribute.to_be_bytes());
            out.extend(((2 + self.bootstraps.len() * 8) as u32).to_be_bytes());
            out.extend((self.bootstraps.len() as u16).to_be_bytes());
            for (factory, target) in &self.bootstraps {
                out.extend(factory.to_be_bytes());
                out.extend([0, 2, 0, 0]);
                out.extend(target.to_be_bytes());
            }
            out
        }
    }

    fn reference_class() -> Vec<u8> {
        let mut c = Class::new(ITEM_IDS);
        let mut code = Code::default();
        let create = c.method_ref(ITEM_IDS, "create", &format!("(Ljava/lang/String;){KEY}"));
        let colored = c.method_ref(
            ITEM_IDS,
            "createSimpleColored",
            &format!("(Ljava/lang/String;){COLORS_DESCRIPTOR}"),
        );
        let simple_copper = c.method_ref(
            ITEM_IDS,
            "createSimpleCopper",
            &format!("(Ljava/lang/String;){COPPER_DESCRIPTOR}"),
        );
        let pair = c.method_ref(
            "net/minecraft/references/BlockItemId",
            "create",
            "(Ljava/lang/String;Ljava/lang/String;)Lnet/minecraft/references/BlockItemId;",
        );
        let by_state = c.class(BY_STATE);
        let by_state_init = c.method_ref(
            BY_STATE,
            "<init>",
            "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V",
        );
        let same = c.method_ref(
            COPPER,
            "same",
            &format!("(L{BY_STATE};){COPPER_DESCRIPTOR}"),
        );
        for (text, field) in [
            ("stone", "STONE"),
            ("cut_sandstone_slab", "CUT_SANDSTONE_SLAB"),
            ("ghost", "GHOST"),
        ] {
            let text = c.string(text);
            let field = c.field(ITEM_IDS, field, KEY);
            code.ldc(text)
                .with(INVOKESTATIC, create)
                .with(PUTSTATIC, field);
        }
        let wool = c.string("wool");
        let field = c.field(ITEM_IDS, "WOOL", COLORS_DESCRIPTOR);
        code.ldc(wool)
            .with(INVOKESTATIC, colored)
            .with(PUTSTATIC, field);
        let names = c.field(ITEM_IDS, "COPPER_NAMES", &format!("L{BY_STATE};"));
        code.with(NEW, by_state).op(0x59);
        for text in ["copper_block", "copper", "copper", "copper"] {
            let text = c.string(text);
            code.ldc(text);
        }
        code.with(INVOKESPECIAL, by_state_init)
            .with(PUTSTATIC, names);
        let field = c.field(ITEM_IDS, "COPPER_BLOCK", COPPER_DESCRIPTOR);
        code.with(GETSTATIC, names)
            .with(INVOKESTATIC, same)
            .with(PUTSTATIC, field);
        let cut_copper = c.string("cut_copper");
        let field = c.field(ITEM_IDS, "CUT_COPPER", COPPER_DESCRIPTOR);
        code.ldc(cut_copper)
            .with(INVOKESTATIC, simple_copper)
            .with(PUTSTATIC, field);
        let stem = c.string("melon_stem");
        let seeds = c.string("melon_seeds");
        let field = c.field(ITEM_IDS, "MELON", "Lnet/minecraft/references/BlockItemId;");
        code.ldc(stem)
            .ldc(seeds)
            .with(INVOKESTATIC, pair)
            .with(PUTSTATIC, field);
        code.op(0xb1);
        c.method("<clinit>", "()V", &code);
        c.bytes()
    }

    fn items_class() -> Vec<u8> {
        let mut c = Class::new(ITEMS_CLASS);
        let mut code = Code::default();
        let register = c.method_ref(
            ITEMS_CLASS,
            "registerItem",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        );
        let block_item = "Lnet/minecraft/references/BlockItemId;";
        for (reference, reference_type, field, field_type) in [
            ("STONE", KEY, "STONE", ITEM_DESCRIPTOR),
            ("WOOL", COLORS_DESCRIPTOR, "WOOL", COLORS_DESCRIPTOR),
            (
                "CUT_SANDSTONE_SLAB",
                KEY,
                "CUT_STANDSTONE_SLAB",
                ITEM_DESCRIPTOR,
            ),
            (
                "COPPER_BLOCK",
                COPPER_DESCRIPTOR,
                "COPPER_BLOCK",
                COPPER_DESCRIPTOR,
            ),
            (
                "CUT_COPPER",
                COPPER_DESCRIPTOR,
                "CUT_COPPER",
                COPPER_DESCRIPTOR,
            ),
            ("MELON", block_item, "MELON_SEEDS", ITEM_DESCRIPTOR),
            ("GHOST", KEY, "GHOST", ITEM_DESCRIPTOR),
        ] {
            let reference = c.field(ITEM_IDS, reference, reference_type);
            let field = c.field(ITEMS_CLASS, field, field_type);
            code.with(GETSTATIC, reference)
                .with(INVOKESTATIC, register)
                .with(PUTSTATIC, field);
        }
        for field in ["POTION", "TEST_BLOCK", "PAINTING"] {
            let field = c.field(ITEMS_CLASS, field, ITEM_DESCRIPTOR);
            code.op(0x01).with(PUTSTATIC, field);
        }
        code.op(0xb1);
        c.method("<clinit>", "()V", &code);
        c.bytes()
    }

    fn blocks_class() -> Vec<u8> {
        let mut c = Class::new(BLOCKS_CLASS);
        let mut code = Code::default();
        let bricks = c.field(BLOCKS_CLASS, "BRICKS", BLOCK_DESCRIPTOR);
        code.op(0x01).with(PUTSTATIC, bricks).op(0xb1);
        c.method("<clinit>", "()V", &code);
        c.bytes()
    }

    fn tabs_class() -> Vec<u8> {
        let mut c = Class::new(TABS_CLASS);
        let registries = "net/minecraft/core/registries/Registries";
        let parameters = format!("L{PARAMETERS};");

        let mut code = Code::default();
        let tab_registry = c.field(registries, "CREATIVE_MODE_TAB", KEY);
        let with_namespace = c.method_ref(
            "net/minecraft/resources/Identifier",
            "withDefaultNamespace",
            "(Ljava/lang/String;)Lnet/minecraft/resources/Identifier;",
        );
        let key_create = c.method_ref(
            "net/minecraft/resources/ResourceKey",
            "create",
            &format!("({KEY}Lnet/minecraft/resources/Identifier;){KEY}"),
        );
        code.with(GETSTATIC, tab_registry)
            .op(0x2a)
            .with(INVOKESTATIC, with_namespace)
            .with(INVOKESTATIC, key_create)
            .op(0xb0);
        c.method("createKey", &format!("(Ljava/lang/String;){KEY}"), &code);

        let mut code = Code::default();
        let create_key = c.method_ref(
            TABS_CLASS,
            "createKey",
            &format!("(Ljava/lang/String;){KEY}"),
        );
        let texture = c.method_ref(
            TAB,
            "createTextureLocation",
            "(Ljava/lang/String;)Lnet/minecraft/resources/Identifier;",
        );
        let search_background = c.field(
            TABS_CLASS,
            "SEARCH_BACKGROUND",
            "Lnet/minecraft/resources/Identifier;",
        );
        let item_search = c.string("item_search");
        code.ldc(item_search)
            .with(INVOKESTATIC, texture)
            .with(PUTSTATIC, search_background);
        let mut keys = Vec::new();
        for (name, field) in [("building", "BUILDING"), ("search", "SEARCH"), ("op", "OP")] {
            let name = c.string(name);
            let field = c.field(TABS_CLASS, field, KEY);
            code.ldc(name)
                .with(INVOKESTATIC, create_key)
                .with(PUTSTATIC, field);
            keys.push(field);
        }
        code.op(0xb1);
        c.method("<clinit>", "()V", &code);

        let builder = c.method_ref(TAB, "builder", &format!("(L{ROW};I){BUILDER_DESCRIPTOR}"));
        let title = c.method_ref(
            BUILDER,
            "title",
            &format!("(Lnet/minecraft/network/chat/Component;){BUILDER_DESCRIPTOR}"),
        );
        let translatable = c.interface(
            "net/minecraft/network/chat/Component",
            "translatable",
            "(Ljava/lang/String;)Lnet/minecraft/network/chat/MutableComponent;",
        );
        let icon = c.method_ref(
            BUILDER,
            "icon",
            &format!("(Ljava/util/function/Supplier;){BUILDER_DESCRIPTOR}"),
        );
        let generator = "Lnet/minecraft/world/item/CreativeModeTab$DisplayItemsGenerator;";
        let display = c.method_ref(
            BUILDER,
            "displayItems",
            &format!("({generator}){BUILDER_DESCRIPTOR}"),
        );
        let right = c.method_ref(BUILDER, "alignedRight", &format!("(){BUILDER_DESCRIPTOR}"));
        let kind = c.method_ref(BUILDER, "type", &format!("(L{TYPE};){BUILDER_DESCRIPTOR}"));
        let background = c.method_ref(
            BUILDER,
            "backgroundTexture",
            &format!("(Lnet/minecraft/resources/Identifier;){BUILDER_DESCRIPTOR}"),
        );
        let build = c.method_ref(BUILDER, "build", &format!("()L{TAB};"));
        let register = c.interface(
            "net/minecraft/core/Registry",
            "register",
            &format!("(Lnet/minecraft/core/Registry;{KEY}Ljava/lang/Object;)Ljava/lang/Object;"),
        );
        let top = c.field(ROW, "TOP", &format!("L{ROW};"));
        let bottom = c.field(ROW, "BOTTOM", &format!("L{ROW};"));
        let search_type = c.field(TYPE, "SEARCH", &format!("L{TYPE};"));
        let dye = c.class(DYE_COLOR);
        let light_gray = c.field(DYE_COLOR, "LIGHT_GRAY", &format!("L{DYE_COLOR};"));
        let white = c.field(DYE_COLOR, "WHITE", &format!("L{DYE_COLOR};"));
        let list_of = c.interface(
            "java/util/List",
            "of",
            "([Ljava/lang/Object;)Ljava/util/List;",
        );
        let icon_target = c.method_ref(TABS_CLASS, "lambda$icon", &format!("()L{ITEM_STACK};"));
        let icon_lambda = c.lambda("get", "()Ljava/util/function/Supplier;", 6, icon_target);
        let building_target = c.method_ref(
            TABS_CLASS,
            "lambda$building",
            &format!("(Ljava/util/List;{parameters}{OUTPUT_DESCRIPTOR})V"),
        );
        let building_lambda = c.lambda(
            "accept",
            &format!("(Ljava/util/List;){generator}"),
            6,
            building_target,
        );
        let op_target = c.method_ref(
            TABS_CLASS,
            "lambda$op",
            &format!("({parameters}{OUTPUT_DESCRIPTOR})V"),
        );
        let op_lambda = c.lambda("accept", &format!("(){generator}"), 6, op_target);
        let titles: Vec<u16> = ["itemGroup.building", "itemGroup.search", "itemGroup.op"]
            .into_iter()
            .map(|text| c.string(text))
            .collect();

        let mut code = Code::default();
        code.op(0x05)
            .with(0xbd, dye)
            .op(0x59)
            .op(0x03)
            .with(GETSTATIC, light_gray)
            .op(0x53)
            .op(0x59)
            .op(0x04)
            .with(GETSTATIC, white)
            .op(0x53)
            .with(INVOKESTATIC, list_of)
            .op(0x4c);
        code.op(0x2a)
            .with(GETSTATIC, keys[0])
            .with(GETSTATIC, top)
            .op(0x03)
            .with(INVOKESTATIC, builder)
            .ldc(titles[0])
            .with(INVOKESTATIC, translatable)
            .with(INVOKEVIRTUAL, title)
            .dynamic(icon_lambda)
            .with(INVOKEVIRTUAL, icon)
            .op(0x2b)
            .dynamic(building_lambda)
            .with(INVOKEVIRTUAL, display)
            .with(INVOKEVIRTUAL, build)
            .with(INVOKESTATIC, register)
            .op(0x57);
        code.op(0x2a)
            .with(GETSTATIC, keys[1])
            .with(GETSTATIC, top)
            .op(BIPUSH)
            .op(6)
            .with(INVOKESTATIC, builder)
            .ldc(titles[1])
            .with(INVOKESTATIC, translatable)
            .with(INVOKEVIRTUAL, title)
            .with(GETSTATIC, search_background)
            .with(INVOKEVIRTUAL, background)
            .with(INVOKEVIRTUAL, right)
            .with(GETSTATIC, search_type)
            .with(INVOKEVIRTUAL, kind)
            .with(INVOKEVIRTUAL, build)
            .with(INVOKESTATIC, register)
            .op(0x57);
        code.op(0x2a)
            .with(GETSTATIC, keys[2])
            .with(GETSTATIC, bottom)
            .op(0x08)
            .with(INVOKESTATIC, builder)
            .ldc(titles[2])
            .with(INVOKESTATIC, translatable)
            .with(INVOKEVIRTUAL, title)
            .with(INVOKEVIRTUAL, right)
            .dynamic(op_lambda)
            .with(INVOKEVIRTUAL, display)
            .with(INVOKEVIRTUAL, build)
            .with(INVOKESTATIC, register)
            .op(0x57)
            .op(0xb1);
        c.method(
            "bootstrap",
            &format!("(Lnet/minecraft/core/Registry;)L{TAB};"),
            &code,
        );

        let stack = c.class(ITEM_STACK);
        let stack_init = c.method_ref(
            ITEM_STACK,
            "<init>",
            "(Lnet/minecraft/world/level/ItemLike;)V",
        );
        let bricks = c.field(BLOCKS_CLASS, "BRICKS", BLOCK_DESCRIPTOR);
        let mut code = Code::default();
        code.with(NEW, stack)
            .op(0x59)
            .with(GETSTATIC, bricks)
            .with(INVOKESPECIAL, stack_init)
            .op(0xb0);
        c.method("lambda$icon", &format!("()L{ITEM_STACK};"), &code);

        let accept = c.interface(OUTPUT, "accept", "(Lnet/minecraft/world/level/ItemLike;)V");
        let accept_stack = c.interface(OUTPUT, "accept", &format!("(L{ITEM_STACK};)V"));
        let accept_ref = c.lambda(
            "accept",
            &format!("({OUTPUT_DESCRIPTOR})Ljava/util/function/Consumer;"),
            9,
            accept,
        );
        let both = c.field(VISIBILITY, "PARENT_AND_SEARCH_TABS", VISIBILITY_DESCRIPTOR);
        let mut code = Code::default();
        let stone = c.field(ITEMS_CLASS, "STONE", ITEM_DESCRIPTOR);
        code.op(0x2c).with(GETSTATIC, stone).interface(accept, 2);
        let wool = c.field(ITEMS_CLASS, "WOOL", COLORS_DESCRIPTOR);
        let register_colored = c.method_ref(
            TABS_CLASS,
            "registerColoredItems",
            &format!("({OUTPUT_DESCRIPTOR}Ljava/util/List;{COLORS_DESCRIPTOR})V"),
        );
        code.op(0x2c)
            .op(0x2a)
            .with(GETSTATIC, wool)
            .with(INVOKESTATIC, register_colored);
        let copper_block = c.field(ITEMS_CLASS, "COPPER_BLOCK", COPPER_DESCRIPTOR);
        let waxed = c.method_ref(COPPER, "waxed", &format!("()L{BY_STATE};"));
        let non_null = c.method_ref(
            "java/util/Objects",
            "requireNonNull",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        );
        let state_each = c.method_ref(BY_STATE, "forEach", "(Ljava/util/function/Consumer;)V");
        code.with(GETSTATIC, copper_block)
            .with(INVOKEVIRTUAL, waxed)
            .op(0x2c)
            .op(0x59)
            .with(INVOKESTATIC, non_null)
            .op(0x57)
            .dynamic(accept_ref)
            .with(INVOKEVIRTUAL, state_each);
        for field in ["CUT_STANDSTONE_SLAB", "MELON_SEEDS", "GHOST"] {
            let field = c.field(ITEMS_CLASS, field, ITEM_DESCRIPTOR);
            code.op(0x2c).with(GETSTATIC, field).interface(accept, 2);
        }
        let potion = c.field(ITEMS_CLASS, "POTION", ITEM_DESCRIPTOR);
        let potions = c.method_ref(
            TABS_CLASS,
            "generatePotionEffectTypes",
            &format!(
                "({OUTPUT_DESCRIPTOR}Lnet/minecraft/core/HolderLookup;{ITEM_DESCRIPTOR}{VISIBILITY_DESCRIPTOR}Lnet/minecraft/world/flag/FeatureFlagSet;)V"
            ),
        );
        code.op(0x2c)
            .op(0x01)
            .with(GETSTATIC, potion)
            .with(GETSTATIC, both)
            .op(0x01)
            .with(INVOKESTATIC, potions);
        let holders = c.method_ref(
            PARAMETERS,
            "holders",
            "()Lnet/minecraft/core/HolderLookup$Provider;",
        );
        let enchantments = c.field(registries, "ENCHANTMENT", KEY);
        let lookup = c.interface(
            "net/minecraft/core/HolderLookup$Provider",
            "lookup",
            &format!("({KEY})Ljava/util/Optional;"),
        );
        let books_target = c.method_ref(
            TABS_CLASS,
            "lambda$books",
            &format!("({OUTPUT_DESCRIPTOR}Lnet/minecraft/core/HolderLookup$RegistryLookup;)V"),
        );
        let books = c.lambda(
            "accept",
            &format!("({OUTPUT_DESCRIPTOR})Ljava/util/function/Consumer;"),
            6,
            books_target,
        );
        let if_present = c.method_ref(
            "java/util/Optional",
            "ifPresent",
            "(Ljava/util/function/Consumer;)V",
        );
        code.op(0x2b)
            .with(INVOKEVIRTUAL, holders)
            .with(GETSTATIC, enchantments)
            .interface(lookup, 2)
            .op(0x2c)
            .dynamic(books)
            .with(INVOKEVIRTUAL, if_present);
        let cut_copper = c.field(ITEMS_CLASS, "CUT_COPPER", COPPER_DESCRIPTOR);
        let copper_each = c.method_ref(COPPER, "forEach", "(Ljava/util/function/Consumer;)V");
        code.with(GETSTATIC, cut_copper)
            .op(0x2c)
            .dynamic(accept_ref)
            .with(INVOKEVIRTUAL, copper_each)
            .op(0xb1);
        c.method(
            "lambda$building",
            &format!("(Ljava/util/List;{parameters}{OUTPUT_DESCRIPTOR})V"),
            &code,
        );

        let color_target = c.method_ref(
            TABS_CLASS,
            "lambda$color",
            &format!("({OUTPUT_DESCRIPTOR}{COLORS_DESCRIPTOR}L{DYE_COLOR};)V"),
        );
        let color_lambda = c.lambda(
            "accept",
            &format!("({OUTPUT_DESCRIPTOR}{COLORS_DESCRIPTOR})Ljava/util/function/Consumer;"),
            6,
            color_target,
        );
        let list_each = c.interface(
            "java/util/List",
            "forEach",
            "(Ljava/util/function/Consumer;)V",
        );
        let mut code = Code::default();
        code.op(0x2b)
            .op(0x2a)
            .op(0x2c)
            .dynamic(color_lambda)
            .interface(list_each, 2)
            .op(0xb1);
        c.method(
            "registerColoredItems",
            &format!("({OUTPUT_DESCRIPTOR}Ljava/util/List;{COLORS_DESCRIPTOR})V"),
            &code,
        );

        let pick = c.method_ref(
            COLORS,
            "pick",
            &format!("(L{DYE_COLOR};)Ljava/lang/Object;"),
        );
        let like = c.class("net/minecraft/world/level/ItemLike");
        let mut code = Code::default();
        code.op(0x2a)
            .op(0x2b)
            .op(0x2c)
            .with(INVOKEVIRTUAL, pick)
            .with(0xc0, like)
            .interface(accept, 2)
            .op(0xb1);
        c.method(
            "lambda$color",
            &format!("({OUTPUT_DESCRIPTOR}{COLORS_DESCRIPTOR}L{DYE_COLOR};)V"),
            &code,
        );

        let tab_only = c.field(VISIBILITY, "PARENT_TAB_ONLY", VISIBILITY_DESCRIPTOR);
        let search_only = c.field(VISIBILITY, "SEARCH_TAB_ONLY", VISIBILITY_DESCRIPTOR);
        let book_descriptor = format!(
            "({OUTPUT_DESCRIPTOR}Lnet/minecraft/core/HolderLookup;{VISIBILITY_DESCRIPTOR})V"
        );
        let max = c.method_ref(
            TABS_CLASS,
            "generateEnchantmentBookTypesOnlyMaxLevel",
            &book_descriptor,
        );
        let all = c.method_ref(
            TABS_CLASS,
            "generateEnchantmentBookTypesAllLevels",
            &book_descriptor,
        );
        let mut code = Code::default();
        code.op(0x2a)
            .op(0x2b)
            .with(GETSTATIC, tab_only)
            .with(INVOKESTATIC, max)
            .op(0x2a)
            .op(0x2b)
            .with(GETSTATIC, search_only)
            .with(INVOKESTATIC, all)
            .op(0xb1);
        c.method(
            "lambda$books",
            &format!("({OUTPUT_DESCRIPTOR}Lnet/minecraft/core/HolderLookup$RegistryLookup;)V"),
            &code,
        );

        let permitted = c.method_ref(PARAMETERS, "hasPermissions", "()Z");
        let test_block = c.field(ITEMS_CLASS, "TEST_BLOCK", ITEM_DESCRIPTOR);
        let set_mode = c.method_ref(
            "net/minecraft/world/level/block/TestBlock",
            "setModeOnStack",
            &format!(
                "(L{ITEM_STACK};Lnet/minecraft/world/level/block/state/properties/TestBlockMode;)L{ITEM_STACK};"
            ),
        );
        let unplaceable_target = c.method_ref(
            TABS_CLASS,
            "lambda$unplaceable",
            "(Lnet/minecraft/core/Holder;)Z",
        );
        let predicate = c.lambda(
            "test",
            "()Ljava/util/function/Predicate;",
            6,
            unplaceable_target,
        );
        let paintings = c.method_ref(
            TABS_CLASS,
            "generatePresetPaintings",
            &format!(
                "({OUTPUT_DESCRIPTOR}Lnet/minecraft/core/HolderLookup$Provider;Lnet/minecraft/core/HolderLookup$RegistryLookup;Ljava/util/function/Predicate;{VISIBILITY_DESCRIPTOR})V"
            ),
        );
        let mut code = Code::default();
        code.op(0x2a).with(INVOKEVIRTUAL, permitted);
        let refused = code.branch(0x99);
        code.op(0x03).op(0x3d);
        let top = code.here();
        code.op(0x1c).op(0x07);
        let done = code.branch(0xa2);
        code.op(0x2b)
            .with(NEW, stack)
            .op(0x59)
            .with(GETSTATIC, test_block)
            .with(INVOKESPECIAL, stack_init)
            .op(0x01)
            .with(INVOKESTATIC, set_mode)
            .interface(accept_stack, 2)
            .op(IINC)
            .op(2)
            .op(1);
        code.back(GOTO, top);
        code.land(done);
        code.op(0x2b)
            .op(0x01)
            .op(0x01)
            .dynamic(predicate)
            .with(GETSTATIC, both)
            .with(INVOKESTATIC, paintings);
        code.land(refused);
        code.op(0xb1);
        c.method(
            "lambda$op",
            &format!("({parameters}{OUTPUT_DESCRIPTOR})V"),
            &code,
        );

        let placeable = c.field(
            "net/minecraft/tags/PaintingVariantTags",
            "PLACEABLE",
            "Lnet/minecraft/tags/TagKey;",
        );
        let is = c.interface(
            "net/minecraft/core/Holder",
            "is",
            "(Lnet/minecraft/tags/TagKey;)Z",
        );
        let mut code = Code::default();
        code.op(0x2a).with(GETSTATIC, placeable).interface(is, 2);
        let tagged = code.branch(0x9a);
        code.op(0x04);
        let out = code.branch(GOTO);
        code.land(tagged);
        code.op(0x03);
        code.land(out);
        code.op(0xac);
        c.method(
            "lambda$unplaceable",
            "(Lnet/minecraft/core/Holder;)Z",
            &code,
        );
        c.bytes()
    }

    fn classes() -> HashMap<String, Vec<u8>> {
        HashMap::from([
            (ITEM_IDS.to_owned(), reference_class()),
            (ITEMS_CLASS.to_owned(), items_class()),
            (BLOCKS_CLASS.to_owned(), blocks_class()),
            (TABS_CLASS.to_owned(), tabs_class()),
        ])
    }

    fn copper_ids(base: &str, unaffected: &str) -> Vec<String> {
        let names = [
            unaffected.to_owned(),
            base.to_owned(),
            base.to_owned(),
            base.to_owned(),
        ];
        copper(&names).flatten()
    }

    fn known() -> HashSet<String> {
        let mut known: HashSet<String> = [
            "stone",
            "white_wool",
            "light_gray_wool",
            "cut_sandstone_slab",
            "melon_seeds",
            "potion",
            "enchanted_book",
            "test_block",
            "painting",
            "bricks",
        ]
        .into_iter()
        .map(namespaced)
        .collect();
        known.extend(copper_ids("copper", "copper_block"));
        known.extend(copper_ids("cut_copper", "cut_copper"));
        known
    }

    fn read(classes: &HashMap<String, Vec<u8>>, known: &HashSet<String>) -> Result<CreativeTabs> {
        read_tabs(&mut |name| classes.get(name).cloned(), known)
    }

    fn entry(item: &str, variants: Option<Variants>, in_tab: bool, in_search: bool) -> TabEntry {
        TabEntry {
            item: namespaced(item),
            variants,
            in_tab,
            in_search,
        }
    }

    #[test]
    fn copper_and_color_ids_follow_the_vanilla_prefixes() {
        assert_eq!(
            copper_ids("copper", "copper_block"),
            [
                "copper_block",
                "exposed_copper",
                "weathered_copper",
                "oxidized_copper",
                "waxed_copper_block",
                "waxed_exposed_copper",
                "waxed_weathered_copper",
                "waxed_oxidized_copper",
            ]
            .map(namespaced)
        );
        let Ids::Colors(wool) = colored("wool") else {
            panic!("not a color collection");
        };
        assert_eq!(wool.len(), 16);
        assert_eq!(wool[0], "minecraft:white_wool");
        assert_eq!(wool[3], "minecraft:light_blue_wool");
        assert_eq!(wool[15], "minecraft:black_wool");
        assert_eq!(color_index("lightBlue"), Some(3));
        assert_eq!(color_index("LIGHT_GRAY"), Some(8));
        assert_eq!(color_index("light_gray"), Some(8));
        assert_eq!(color_index("white"), Some(0));
        assert_eq!(color_index("asList"), None);
        assert_eq!(namespaced("minecraft:stone"), "minecraft:stone");
    }

    #[test]
    fn synthetic_tabs_are_read_with_their_layout_icons_and_items() {
        let tabs = read(&classes(), &known()).unwrap();
        assert_eq!(tabs.fallback, None);
        let ids: Vec<&str> = tabs.tabs.iter().map(|tab| tab.id.as_str()).collect();
        assert_eq!(
            ids,
            ["minecraft:building", "minecraft:search", "minecraft:op"]
        );

        let building = &tabs.tabs[0];
        assert_eq!(building.title, "itemGroup.building");
        assert_eq!((building.row, building.column), (Row::Top, 0));
        assert_eq!(building.kind, TabKind::Category);
        assert!(!building.aligned_right && building.show_title && building.scroll_bar);
        assert_eq!(building.background, "items");
        assert_eq!(building.icon.as_deref(), Some("minecraft:bricks"));
        assert!(!building.operator);
        let mut expected = vec![
            entry("stone", None, true, true),
            entry("light_gray_wool", None, true, true),
            entry("white_wool", None, true, true),
        ];
        for id in &copper_ids("copper", "copper_block")[4..] {
            expected.push(entry(id, None, true, true));
        }
        expected.extend([
            entry("cut_sandstone_slab", None, true, true),
            entry("melon_seeds", None, true, true),
            entry("potion", Some(Variants::Potions), true, true),
            entry(
                "enchanted_book",
                Some(Variants::EnchantedBooks { all_levels: false }),
                true,
                false,
            ),
            entry(
                "enchanted_book",
                Some(Variants::EnchantedBooks { all_levels: true }),
                false,
                true,
            ),
        ]);
        for id in copper_ids("cut_copper", "cut_copper") {
            expected.push(entry(&id, None, true, true));
        }
        assert_eq!(building.entries, expected);
        assert_eq!(building.item_ids().len(), 19);
        assert_eq!(
            tabs.skipped,
            ["net/minecraft/world/item/Items.GHOST has no known item id"]
        );

        let search = &tabs.tabs[1];
        assert_eq!(
            (search.row, search.column, search.kind),
            (Row::Top, 6, TabKind::Search)
        );
        assert!(search.aligned_right);
        assert_eq!(search.background, "item_search");
        assert!(search.entries.is_empty() && search.icon.is_none());

        let op = &tabs.tabs[2];
        assert_eq!(
            (op.row, op.column, op.kind),
            (Row::Bottom, 5, TabKind::Category)
        );
        assert!(op.aligned_right && op.operator);
        assert_eq!(
            op.entries,
            [
                entry("test_block", Some(Variants::TestBlockModes), true, true),
                entry(
                    "painting",
                    Some(Variants::Paintings {
                        tag: "minecraft:placeable".to_owned(),
                        in_tag: false,
                    }),
                    true,
                    true,
                ),
            ]
        );
    }

    #[test]
    fn search_joins_the_category_tabs_and_hides_operator_items() {
        let tabs = read(&classes(), &known()).unwrap();
        let everything = tabs.search(true);
        let plain = tabs.search(false);
        assert_eq!(plain.len(), 19);
        assert_eq!(everything.len(), 21);
        assert!(
            everything
                .iter()
                .all(|entry| entry.in_tab && entry.in_search)
        );
        assert!(
            plain
                .iter()
                .all(|entry| entry.item != "minecraft:test_block")
        );
        assert_eq!(
            plain
                .iter()
                .filter(|entry| entry.item == "minecraft:enchanted_book")
                .map(|entry| entry.variants.clone())
                .collect::<Vec<_>>(),
            [Some(Variants::EnchantedBooks { all_levels: true })]
        );
        let shown = |operator| {
            tabs.shown(operator)
                .map(|tab| tab.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(shown(false), ["minecraft:building", "minecraft:search"]);
        assert_eq!(
            shown(true),
            ["minecraft:building", "minecraft:search", "minecraft:op"]
        );
        let mut doubled = tabs.clone();
        doubled.tabs[2]
            .entries
            .push(entry("stone", None, true, true));
        assert_eq!(doubled.search(true).len(), 21);
    }

    #[test]
    fn unknown_items_are_skipped_and_logged() {
        let mut known = known();
        known.remove("minecraft:white_wool");
        known.remove("minecraft:bricks");
        let tabs = read(&classes(), &known).unwrap();
        let building = &tabs.tabs[0];
        assert!(building.icon.is_none());
        assert!(!building.item_ids().contains(&"minecraft:white_wool"));
        assert!(
            tabs.skipped
                .contains(&"minecraft:white_wool is not a known item".to_owned())
        );
        assert!(
            tabs.skipped
                .iter()
                .any(|line| line.starts_with("minecraft:building: the icon"))
        );
    }

    #[test]
    fn a_missing_tabs_class_falls_back_to_registry_order() {
        let mut classes = classes();
        classes.remove(TABS_CLASS);
        let known = known();
        let error = read(&classes, &known).unwrap_err();
        assert!(format!("{error:#}").contains("CreativeModeTabs.class is not in the jar"));
        assert!(read(&HashMap::new(), &HashSet::new()).is_err());

        let order = registry_order(&mut |name| classes.get(name).cloned(), &known);
        let mut expected: Vec<String> = [
            "stone",
            "white_wool",
            "light_gray_wool",
            "cut_sandstone_slab",
        ]
        .into_iter()
        .map(namespaced)
        .collect();
        expected.extend(copper_ids("copper", "copper_block"));
        expected.extend(copper_ids("cut_copper", "cut_copper"));
        expected.extend(
            [
                "melon_seeds",
                "potion",
                "test_block",
                "painting",
                "bricks",
                "enchanted_book",
            ]
            .into_iter()
            .map(namespaced),
        );
        assert_eq!(order, expected);
        let without_items = registry_order(&mut |_| None, &known);
        assert_eq!(without_items.len(), known.len());
        assert_eq!(without_items[0], "minecraft:bricks");

        let tabs = fallback_tabs(order.clone(), "no tabs class");
        assert_eq!(tabs.fallback.as_deref(), Some("no tabs class"));
        assert_eq!(tabs.tabs.len(), 14);
        let shown: Vec<&str> = tabs.shown(true).map(|tab| tab.id.as_str()).collect();
        assert_eq!(
            shown,
            [
                "minecraft:hotbar",
                "minecraft:search",
                "minecraft:inventory"
            ]
        );
        let search: Vec<String> = tabs
            .search(false)
            .into_iter()
            .map(|entry| entry.item)
            .collect();
        assert_eq!(search, order);
        let lost = creative_tabs(Path::new("no/such/client.jar"));
        assert!(lost.fallback.is_some());
        assert!(lost.search(true).is_empty());
    }

    #[test]
    fn the_real_tabs_match_vanilla() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let tabs = read_creative_tabs(&jar).unwrap();
        assert_eq!(tabs.skipped, Vec::<String>::new());
        let layout = fallback_tabs(Vec::new(), "");
        assert_eq!(tabs.tabs.len(), layout.tabs.len());
        for (tab, expected) in tabs.tabs.iter().zip(&layout.tabs) {
            assert_eq!(
                (
                    &tab.id,
                    &tab.title,
                    tab.row,
                    tab.column,
                    tab.kind,
                    tab.aligned_right,
                    tab.show_title,
                    tab.scroll_bar,
                    &tab.background,
                    tab.operator,
                ),
                (
                    &expected.id,
                    &expected.title,
                    expected.row,
                    expected.column,
                    expected.kind,
                    expected.aligned_right,
                    expected.show_title,
                    expected.scroll_bar,
                    &expected.background,
                    expected.operator,
                )
            );
        }
        let icons: Vec<&str> = tabs
            .tabs
            .iter()
            .map(|tab| tab.icon.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(
            icons,
            [
                "bricks",
                "cyan_wool",
                "grass_block",
                "oak_sign",
                "redstone",
                "bookshelf",
                "compass",
                "diamond_pickaxe",
                "netherite_sword",
                "golden_apple",
                "iron_ingot",
                "creeper_spawn_egg",
                "command_block",
                "chest",
            ]
            .map(namespaced)
        );
        let counts: Vec<usize> = tabs.tabs.iter().map(|tab| tab.entries.len()).collect();
        assert_eq!(
            counts,
            [475, 278, 254, 238, 70, 0, 0, 149, 79, 47, 153, 89, 13, 0]
        );
        let ids = |id: &str| tabs.tab(id).unwrap().item_ids();
        assert_eq!(
            ids("minecraft:building_blocks")[..4],
            [
                "oak_log",
                "oak_wood",
                "stripped_oak_log",
                "stripped_oak_wood"
            ]
            .map(namespaced)
        );
        assert_eq!(
            ids("minecraft:colored_blocks")[..5],
            [
                "white_wool",
                "light_gray_wool",
                "gray_wool",
                "black_wool",
                "brown_wool",
            ]
            .map(namespaced)
        );
        let building = ids("minecraft:building_blocks");
        let copper_at = building
            .iter()
            .position(|id| *id == "minecraft:copper_block")
            .unwrap();
        assert_eq!(
            building[copper_at..copper_at + 4],
            [
                "copper_block",
                "exposed_copper",
                "weathered_copper",
                "oxidized_copper",
            ]
            .map(namespaced)
        );
        assert!(building.contains(&"minecraft:waxed_oxidized_cut_copper_slab"));
        let variants = |id: &str| {
            tabs.tab(id)
                .unwrap()
                .entries
                .iter()
                .filter_map(|entry| {
                    let variants = entry.variants.clone()?;
                    Some((entry.item.clone(), variants, entry.in_tab, entry.in_search))
                })
                .collect::<Vec<_>>()
        };
        let placeable = |in_tag| Variants::Paintings {
            tag: "minecraft:placeable".to_owned(),
            in_tag,
        };
        assert_eq!(
            variants("minecraft:functional_blocks"),
            [
                (namespaced("painting"), placeable(true), true, true),
                (
                    namespaced("white_banner"),
                    Variants::OminousBanner,
                    true,
                    true
                ),
            ]
        );
        let horns = Variants::Instruments {
            tag: "minecraft:goat_horns".to_owned(),
        };
        assert_eq!(
            variants("minecraft:tools_and_utilities"),
            [
                (
                    namespaced("firework_rocket"),
                    Variants::Fireworks,
                    true,
                    true
                ),
                (namespaced("goat_horn"), horns, true, true),
            ]
        );
        assert_eq!(
            variants("minecraft:combat"),
            [
                (
                    namespaced("firework_rocket"),
                    Variants::Fireworks,
                    true,
                    true
                ),
                (namespaced("tipped_arrow"), Variants::Potions, true, true),
            ]
        );
        assert_eq!(
            variants("minecraft:food_and_drinks"),
            [
                (
                    namespaced("suspicious_stew"),
                    Variants::SuspiciousStews,
                    true,
                    true
                ),
                (
                    namespaced("ominous_bottle"),
                    Variants::OminousBottles,
                    true,
                    true
                ),
                (namespaced("potion"), Variants::Potions, true, true),
                (namespaced("splash_potion"), Variants::Potions, true, true),
                (
                    namespaced("lingering_potion"),
                    Variants::Potions,
                    true,
                    true
                ),
            ]
        );
        let books = |all_levels| Variants::EnchantedBooks { all_levels };
        assert_eq!(
            variants("minecraft:ingredients"),
            [
                (namespaced("enchanted_book"), books(false), true, false),
                (namespaced("enchanted_book"), books(true), false, true),
            ]
        );
        assert_eq!(
            variants("minecraft:op_blocks"),
            [
                (
                    namespaced("test_block"),
                    Variants::TestBlockModes,
                    true,
                    true
                ),
                (namespaced("light"), Variants::LightLevels, true, true),
                (namespaced("painting"), placeable(false), true, true),
            ]
        );
        assert_eq!(tabs.search(true).len(), 1638);
        assert_eq!(tabs.search(false).len(), 1625);
        assert_eq!(tabs.shown(false).count(), 13);

        let mut archive = Jar::open(&jar).unwrap();
        let known = archive.item_ids();
        let listed: HashSet<String> = tabs
            .search(true)
            .into_iter()
            .map(|entry| entry.item)
            .collect();
        let mut absent: Vec<&str> = known
            .iter()
            .filter(|id| !listed.contains(*id))
            .map(String::as_str)
            .collect();
        absent.sort();
        let maps = [
            "abandoned_camp",
            "buried_ancient_city",
            "buried_mineshaft",
            "buried_treasure",
            "buried_trial_chambers",
            "desert_pyramid",
            "desert_village",
            "jungle_pyramid",
            "ocean_monument",
            "plains_village",
            "savanna_village",
            "snowy_village",
            "swamp_hut",
            "taiga_village",
            "warm_ocean_ruins",
            "woodland_mansion",
        ];
        let mut expected: Vec<String> = maps
            .iter()
            .map(|map| namespaced(&format!("{map}_map")))
            .chain(
                [
                    "air",
                    "ender_dragon_spawn_egg",
                    "filled_map",
                    "knowledge_book",
                    "petrified_oak_slab",
                    "wither_spawn_egg",
                    "written_book",
                ]
                .into_iter()
                .map(namespaced),
            )
            .collect();
        expected.sort();
        assert_eq!(absent, expected);

        let order = registry_order(&mut |name| archive.class(name), &known);
        assert_eq!(order.len(), known.len());
        assert_eq!(
            order[..4],
            ["air", "stone", "granite", "polished_granite"].map(namespaced)
        );
    }
}
