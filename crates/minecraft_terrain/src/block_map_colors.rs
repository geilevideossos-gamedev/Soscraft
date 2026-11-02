use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result, anyhow, bail};
use minecraftoss_core::block::{BlockId, BlockRegistry, BlockStateId};

use crate::class_file::{self, ClassFile, Constant, Handle, Instruction, Member};
use crate::map_picture::{MAP_COLOR_CLASS, MapPalette};

pub const BLOCKS_CLASS: &str = "net/minecraft/world/level/block/Blocks";
pub const DYE_COLOR_CLASS: &str = "net/minecraft/world/item/DyeColor";
pub const BLOCK_ITEM_IDS_CLASS: &str = "net/minecraft/references/BlockItemIds";
pub const BLOCK_IDS_CLASS: &str = "net/minecraft/references/BlockIds";
const PROPERTIES_CLASS: &str = "net/minecraft/world/level/block/state/BlockBehaviour$Properties";
const COLOR_COLLECTION_CLASS: &str = "net/minecraft/world/level/block/ColorCollection";
const COPPER_COLLECTION_CLASS: &str = "net/minecraft/world/level/block/WeatheringCopperCollection";
const COPPER_STATES_CLASS: &str =
    "net/minecraft/world/level/block/WeatheringCopperCollection$ByState";
const WEATHER_STATE_CLASS: &str = "net/minecraft/world/level/block/WeatheringCopper$WeatherState";
const BLOCK_ITEM_ID_CLASS: &str = "net/minecraft/references/BlockItemId";
const BLOCK_STATE_CLASS: &str = "net/minecraft/world/level/block/state/BlockState";
const LAMBDA_FACTORY_CLASS: &str = "java/lang/invoke/LambdaMetafactory";
const BLOCK_ITEM_ID: &str = "Lnet/minecraft/references/BlockItemId;";
const RESOURCE_KEY: &str = "Lnet/minecraft/resources/ResourceKey;";
const COLOR_COLLECTION: &str = "Lnet/minecraft/world/level/block/ColorCollection;";
const COPPER_COLLECTION: &str = "Lnet/minecraft/world/level/block/WeatheringCopperCollection;";
const WEATHER_STATES: [&str; 4] = ["UNAFFECTED", "EXPOSED", "WEATHERED", "OXIDIZED"];
const NONE: &str = "NONE";
const ACC_STATIC: u16 = 0x0008;
const REF_INVOKE_VIRTUAL: u8 = 5;
const REF_INVOKE_STATIC: u8 = 6;
const REF_INVOKE_INTERFACE: u8 = 9;
const MAX_DEPTH: usize = 48;
const MAX_STEPS: usize = 1 << 20;

const ACONST_NULL: u8 = 0x01;
const LCONST_0: u8 = 0x09;
const DCONST_1: u8 = 0x0f;
const ILOAD: u8 = 0x15;
const ALOAD_3: u8 = 0x2d;
const IALOAD: u8 = 0x2e;
const LALOAD: u8 = 0x2f;
const SALOAD: u8 = 0x35;
const ISTORE: u8 = 0x36;
const ASTORE_3: u8 = 0x4e;
const IASTORE: u8 = 0x4f;
const SASTORE: u8 = 0x56;
const POP: u8 = 0x57;
const DUP: u8 = 0x59;
const SWAP: u8 = 0x5f;
const IADD: u8 = 0x60;
const ISUB: u8 = 0x64;
const IMUL: u8 = 0x68;
const IFEQ: u8 = 0x99;
const IFNE: u8 = 0x9a;
const IFLT: u8 = 0x9b;
const IFGE: u8 = 0x9c;
const IFGT: u8 = 0x9d;
const IFLE: u8 = 0x9e;
const IF_ICMPEQ: u8 = 0x9f;
const IF_ICMPNE: u8 = 0xa0;
const IF_ICMPLT: u8 = 0xa1;
const IF_ICMPGE: u8 = 0xa2;
const IF_ICMPGT: u8 = 0xa3;
const IF_ICMPLE: u8 = 0xa4;
const IF_ACMPEQ: u8 = 0xa5;
const IF_ACMPNE: u8 = 0xa6;
const IRETURN: u8 = 0xac;
const ARETURN: u8 = 0xb0;
const RETURN: u8 = 0xb1;
const NEWARRAY: u8 = 0xbc;
const ANEWARRAY: u8 = 0xbd;
const ARRAYLENGTH: u8 = 0xbe;
const ATHROW: u8 = 0xbf;
const CHECKCAST: u8 = 0xc0;
const INSTANCEOF: u8 = 0xc1;
const IFNULL: u8 = 0xc6;
const IFNONNULL: u8 = 0xc7;

pub trait BlockStates {
    fn blocks(&self) -> usize;
    fn states(&self) -> usize;
    fn name(&self, block: usize) -> String;
    fn states_of(&self, block: usize) -> Vec<u16>;
    fn default_of(&self, block: usize) -> u16;
    fn setting(&self, state: u16, property: &str) -> Option<String>;
}

impl BlockStates for BlockRegistry {
    fn blocks(&self) -> usize {
        self.block_count()
    }

    fn states(&self) -> usize {
        self.state_count()
    }

    fn name(&self, block: usize) -> String {
        self.block(BlockId(block as u16)).name.to_string()
    }

    fn states_of(&self, block: usize) -> Vec<u16> {
        self.block(BlockId(block as u16))
            .states()
            .map(|state| state.0)
            .collect()
    }

    fn default_of(&self, block: usize) -> u16 {
        self.block(BlockId(block as u16)).default_state().0
    }

    fn setting(&self, state: u16, property: &str) -> Option<String> {
        self.property(BlockStateId(state), property)
            .map(str::to_owned)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMapColors {
    ids: Vec<u8>,
}

impl BlockMapColors {
    pub fn from_ids(ids: Vec<u8>) -> Self {
        Self { ids }
    }

    pub fn read(
        load: &mut dyn FnMut(&str) -> Option<ClassFile>,
        palette: &MapPalette,
        states: &dyn BlockStates,
    ) -> Result<Self> {
        let dyes = dye_table(&load(DYE_COLOR_CLASS).ok_or_else(|| missing(DYE_COLOR_CLASS))?)?;
        let mut ids = HashMap::new();
        for holder in [BLOCK_ITEM_IDS_CLASS, BLOCK_IDS_CLASS] {
            let class = load(holder).ok_or_else(|| missing(holder))?;
            for (field, value) in id_table(&class, &dyes)? {
                ids.insert((holder.to_owned(), field), value);
            }
        }
        let mut machine = Machine {
            load,
            states,
            classes: HashMap::new(),
            methods: HashMap::new(),
            dyes,
            ids,
            statics: HashMap::new(),
            props: Vec::new(),
            registered: Vec::new(),
            properties: HashMap::new(),
            switches: HashMap::new(),
            defaults: HashMap::new(),
        };
        machine
            .run(BLOCKS_CLASS, "<clinit>", "()V", Vec::new(), 0)
            .context("running Blocks.<clinit>")?;
        let count = states.blocks();
        if machine.registered.len() != count {
            bail!(
                "Blocks registers {} blocks and the catalog has {count}",
                machine.registered.len()
            );
        }
        for (block, registered) in machine.registered.iter().enumerate() {
            if let Some(name) = &registered.name {
                let expected = states.name(block);
                if *name != expected {
                    bail!("block {block} is {name} in the jar and {expected} in the catalog");
                }
            }
        }
        let mut ids = vec![0u8; states.states()];
        let mut seen = vec![false; ids.len()];
        for block in 0..count {
            for state in states.states_of(block) {
                let name = machine
                    .color_at(block, state, 0)
                    .with_context(|| format!("the map colour of {}", states.name(block)))?;
                let id = palette.id(&name).ok_or_else(|| {
                    anyhow!("{} has the unknown map colour {name}", states.name(block))
                })?;
                let at = usize::from(state);
                match (ids.get_mut(at), seen.get_mut(at)) {
                    (Some(slot), Some(done)) if !*done => {
                        *slot = id;
                        *done = true;
                    }
                    _ => bail!("state {state} of {} is out of place", states.name(block)),
                }
            }
        }
        if let Some(state) = seen.iter().position(|done| !done) {
            bail!("state {state} belongs to no block");
        }
        Ok(Self { ids })
    }

    pub fn read_jar(jar: &Path, states: &dyn BlockStates) -> Result<Self> {
        let file =
            std::fs::File::open(jar).with_context(|| format!("cannot open {}", jar.display()))?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut load = |name: &str| -> Option<ClassFile> {
            let mut entry = archive.by_name(&format!("{name}.class")).ok()?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).ok()?;
            ClassFile::parse(&bytes).ok()
        };
        let palette = MapPalette::from_class(
            &load(MAP_COLOR_CLASS).ok_or_else(|| missing(MAP_COLOR_CLASS))?,
        )?;
        Self::read(&mut load, &palette, states)
    }

    pub fn id(&self, state: BlockStateId) -> u8 {
        self.ids.get(usize::from(state.0)).copied().unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

fn missing(class: &str) -> anyhow::Error {
    anyhow!("no {class} in the jar")
}

#[derive(Clone, Debug, PartialEq)]
enum Color {
    Named(String),
    DefaultOf(usize),
}

#[derive(Clone, Debug)]
enum Rule {
    Unset,
    Fixed(Color),
    ByState(Rc<Closure>),
}

#[derive(Debug)]
struct Closure {
    handle: Handle,
    captured: Vec<Value>,
}

#[derive(Clone, Debug)]
enum Value {
    Opaque,
    Null,
    Int(i32),
    Id(Option<String>),
    Constant(String, String),
    Color(Color),
    Props(usize),
    Block(usize),
    Colors(Rc<Vec<Value>>),
    Copper(Rc<Vec<Value>>),
    ByWeather(Rc<Vec<Value>>),
    Closure(Rc<Closure>),
    State(u16),
    Setting(String),
    SwitchMap(String, String),
    Ordinal(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Dye {
    name: String,
    map: String,
    terracotta: String,
}

struct Registered {
    name: Option<String>,
    rule: Rule,
}

struct Code {
    class: Rc<ClassFile>,
    instructions: Vec<Instruction>,
    bytes: Vec<u8>,
    max_locals: usize,
    is_static: bool,
}

enum Flow {
    Next,
    Jump(i32),
    Return(Value),
}

struct Frame {
    locals: Vec<Value>,
    stack: Vec<Value>,
}

impl Frame {
    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| anyhow!("the stack is empty"))
    }

    fn int(&mut self) -> Result<i32> {
        match self.pop()? {
            Value::Int(value) => Ok(value),
            other => bail!("expected an int, found {other:?}"),
        }
    }

    fn take(&mut self, count: usize) -> Result<Vec<Value>> {
        let at = self
            .stack
            .len()
            .checked_sub(count)
            .ok_or_else(|| anyhow!("the stack holds fewer than {count} values"))?;
        Ok(self.stack.split_off(at))
    }

    fn local(&mut self, index: u16) -> Result<&mut Value> {
        self.locals
            .get_mut(usize::from(index))
            .ok_or_else(|| anyhow!("no local {index}"))
    }
}

enum Assignment {
    Alias(Member),
    Named(String),
}

struct Machine<'a> {
    load: &'a mut dyn FnMut(&str) -> Option<ClassFile>,
    states: &'a dyn BlockStates,
    classes: HashMap<String, Option<Rc<ClassFile>>>,
    methods: HashMap<(String, String, String), Rc<Code>>,
    dyes: Vec<Dye>,
    ids: HashMap<(String, String), Value>,
    statics: HashMap<String, Value>,
    props: Vec<Rule>,
    registered: Vec<Registered>,
    properties: HashMap<(String, String), String>,
    switches: HashMap<(String, String), HashMap<String, i32>>,
    defaults: HashMap<usize, String>,
}

impl Machine<'_> {
    fn class(&mut self, name: &str) -> Result<Rc<ClassFile>> {
        if let Some(found) = self.classes.get(name) {
            return found.clone().ok_or_else(|| missing(name));
        }
        let class = (self.load)(name).map(Rc::new);
        self.classes.insert(name.to_owned(), class.clone());
        class.ok_or_else(|| missing(name))
    }

    fn code(&mut self, class: &str, name: &str, descriptor: &str) -> Result<Rc<Code>> {
        let key = (class.to_owned(), name.to_owned(), descriptor.to_owned());
        if let Some(code) = self.methods.get(&key) {
            return Ok(code.clone());
        }
        let owner = self.class(class)?;
        let method = owner
            .method_with(name, descriptor)
            .ok_or_else(|| anyhow!("no method {class}.{name}{descriptor}"))?;
        let bytes = method
            .code
            .clone()
            .ok_or_else(|| anyhow!("{class}.{name} has no code"))?;
        let code = Rc::new(Code {
            instructions: class_file::instructions(&bytes)?,
            bytes,
            max_locals: usize::from(method.max_locals),
            is_static: method.access & ACC_STATIC != 0,
            class: owner.clone(),
        });
        self.methods.insert(key, code.clone());
        Ok(code)
    }

    fn run(
        &mut self,
        class: &str,
        name: &str,
        descriptor: &str,
        arguments: Vec<Value>,
        depth: usize,
    ) -> Result<Value> {
        if depth > MAX_DEPTH {
            bail!("calls nest deeper than {MAX_DEPTH}");
        }
        let code = self.code(class, name, descriptor)?;
        let mut kinds = class_file::parameters(descriptor).into_iter();
        let mut frame = Frame {
            locals: Vec::with_capacity(code.max_locals),
            stack: Vec::new(),
        };
        for (at, value) in arguments.into_iter().enumerate() {
            let receiver = !code.is_static && at == 0;
            let wide = !receiver && matches!(kinds.next().as_deref(), Some("J" | "D"));
            frame.locals.push(value);
            if wide {
                frame.locals.push(Value::Opaque);
            }
        }
        let size = code.max_locals.max(frame.locals.len());
        frame.locals.resize(size, Value::Opaque);
        let mut pc = 0;
        for _ in 0..MAX_STEPS {
            let instruction = *code
                .instructions
                .get(pc)
                .ok_or_else(|| anyhow!("{class}.{name} runs off its end"))?;
            pc += 1;
            let flow = self
                .step(&code, &mut frame, instruction, depth)
                .with_context(|| format!("{class}.{name} at {}", instruction.at))?;
            match flow {
                Flow::Next => {}
                Flow::Jump(target) => {
                    pc = code
                        .instructions
                        .binary_search_by_key(&(target as usize), |instruction| instruction.at)
                        .map_err(|_| anyhow!("{class}.{name} jumps into an instruction"))?;
                }
                Flow::Return(value) => return Ok(value),
            }
        }
        bail!("{class}.{name} runs longer than {MAX_STEPS} steps")
    }

    fn step(
        &mut self,
        code: &Code,
        frame: &mut Frame,
        instruction: Instruction,
        depth: usize,
    ) -> Result<Flow> {
        let class = &code.class;
        let index = instruction.index;
        match instruction.op {
            ACONST_NULL => frame.stack.push(Value::Null),
            class_file::ICONST_M1..=class_file::ICONST_5
            | class_file::BIPUSH
            | class_file::SIPUSH => frame.stack.push(Value::Int(instruction.value)),
            LCONST_0..=DCONST_1 => frame.stack.push(Value::Opaque),
            class_file::LDC | class_file::LDC_W | class_file::LDC2_W => {
                frame.stack.push(match class.constant(index) {
                    Some(Constant::Integer(value)) => Value::Int(*value),
                    _ => Value::Opaque,
                })
            }
            ILOAD..=ALOAD_3 => {
                let value = frame.local(index)?.clone();
                frame.stack.push(value);
            }
            IALOAD => {
                let at = frame.pop()?;
                let array = frame.pop()?;
                frame.stack.push(match (array, at) {
                    (Value::SwitchMap(owner, field), Value::Ordinal(constant)) => {
                        Value::Int(self.switch_case(&owner, &field, &constant)?)
                    }
                    _ => Value::Opaque,
                });
            }
            LALOAD..=SALOAD => {
                frame.take(2)?;
                frame.stack.push(Value::Opaque);
            }
            ISTORE..=ASTORE_3 => {
                let value = frame.pop()?;
                *frame.local(index)? = value;
            }
            IASTORE..=SASTORE => {
                frame.take(3)?;
            }
            POP => {
                frame.pop()?;
            }
            DUP => {
                let top = frame
                    .stack
                    .last()
                    .cloned()
                    .ok_or_else(|| anyhow!("dup on an empty stack"))?;
                frame.stack.push(top);
            }
            SWAP => {
                let top = frame.pop()?;
                let below = frame.pop()?;
                frame.stack.push(top);
                frame.stack.push(below);
            }
            IADD | ISUB | IMUL => {
                let right = frame.int()?;
                let left = frame.int()?;
                frame.stack.push(Value::Int(match instruction.op {
                    IADD => left.wrapping_add(right),
                    ISUB => left.wrapping_sub(right),
                    _ => left.wrapping_mul(right),
                }));
            }
            class_file::IINC => match frame.local(index)? {
                Value::Int(value) => *value = value.wrapping_add(instruction.value),
                other => bail!("iinc on {other:?}"),
            },
            IFEQ..=IFLE => {
                let value = frame.int()?;
                let taken = match instruction.op {
                    IFEQ => value == 0,
                    IFNE => value != 0,
                    IFLT => value < 0,
                    IFGE => value >= 0,
                    IFGT => value > 0,
                    _ => value <= 0,
                };
                if taken {
                    return Ok(Flow::Jump(instruction.value));
                }
            }
            IF_ICMPEQ..=IF_ICMPLE => {
                let right = frame.int()?;
                let left = frame.int()?;
                let taken = match instruction.op {
                    IF_ICMPEQ => left == right,
                    IF_ICMPNE => left != right,
                    IF_ICMPLT => left < right,
                    IF_ICMPGE => left >= right,
                    IF_ICMPGT => left > right,
                    _ => left <= right,
                };
                if taken {
                    return Ok(Flow::Jump(instruction.value));
                }
            }
            IF_ACMPEQ | IF_ACMPNE => {
                let right = frame.pop()?;
                let left = frame.pop()?;
                if same(&left, &right)? == (instruction.op == IF_ACMPEQ) {
                    return Ok(Flow::Jump(instruction.value));
                }
            }
            IFNULL | IFNONNULL => {
                let null = match frame.pop()? {
                    Value::Null => true,
                    Value::Opaque => bail!("cannot tell whether an unknown value is null"),
                    _ => false,
                };
                if null == (instruction.op == IFNULL) {
                    return Ok(Flow::Jump(instruction.value));
                }
            }
            class_file::GOTO | class_file::GOTO_W => return Ok(Flow::Jump(instruction.value)),
            class_file::TABLESWITCH | class_file::LOOKUPSWITCH => {
                let key = frame.int()?;
                return Ok(Flow::Jump(switch_target(&code.bytes, instruction.at, key)?));
            }
            IRETURN..=ARETURN => return Ok(Flow::Return(frame.pop()?)),
            RETURN => return Ok(Flow::Return(Value::Opaque)),
            class_file::GETSTATIC => {
                let member = member(class, index)?;
                let value = self.get_static(&member)?;
                frame.stack.push(value);
            }
            class_file::PUTSTATIC => {
                let member = member(class, index)?;
                let value = frame.pop()?;
                if member.class == BLOCKS_CLASS {
                    self.statics.insert(member.name, value);
                }
            }
            class_file::GETFIELD => {
                frame.pop()?;
                frame.stack.push(Value::Opaque);
            }
            class_file::PUTFIELD => {
                frame.take(2)?;
            }
            class_file::INVOKEVIRTUAL
            | class_file::INVOKESPECIAL
            | class_file::INVOKESTATIC
            | class_file::INVOKEINTERFACE => {
                let member = member(class, index)?;
                let receiver = usize::from(instruction.op != class_file::INVOKESTATIC);
                let arguments =
                    frame.take(class_file::parameters(&member.descriptor).len() + receiver)?;
                let value = self.invoke(&member, arguments, depth)?;
                if class_file::returns_value(&member.descriptor) {
                    frame.stack.push(value);
                }
            }
            class_file::INVOKEDYNAMIC => {
                let dynamic = class
                    .invoke_dynamic(index)
                    .ok_or_else(|| anyhow!("constant {index} is not a dynamic call"))?;
                let captured = frame.take(class_file::parameters(&dynamic.descriptor).len())?;
                let handle = if dynamic.bootstrap.member.class == LAMBDA_FACTORY_CLASS {
                    dynamic
                        .arguments
                        .get(1)
                        .and_then(|&argument| class.handle(argument))
                } else {
                    None
                };
                frame.stack.push(match handle {
                    Some(handle) => Value::Closure(Rc::new(Closure { handle, captured })),
                    None => Value::Opaque,
                });
            }
            class_file::NEW => frame.stack.push(Value::Opaque),
            NEWARRAY | ANEWARRAY | ARRAYLENGTH | INSTANCEOF => {
                frame.pop()?;
                frame.stack.push(Value::Opaque);
            }
            CHECKCAST => {}
            ATHROW => bail!("the code throws"),
            op => bail!("unsupported opcode {op:#04x}"),
        }
        Ok(Flow::Next)
    }

    fn get_static(&mut self, member: &Member) -> Result<Value> {
        if member.class == BLOCKS_CLASS {
            return self
                .statics
                .get(&member.name)
                .cloned()
                .ok_or_else(|| anyhow!("Blocks.{} is read before it is set", member.name));
        }
        if member.class == MAP_COLOR_CLASS {
            return Ok(Value::Color(Color::Named(member.name.clone())));
        }
        if member.class == COLOR_COLLECTION_CLASS && member.name == "VALUES" {
            return Ok(Value::Colors(Rc::new(self.dye_values())));
        }
        if member.class == BLOCK_ITEM_IDS_CLASS || member.class == BLOCK_IDS_CLASS {
            return self
                .ids
                .get(&(member.class.clone(), member.name.clone()))
                .cloned()
                .ok_or_else(|| anyhow!("no block id {}.{}", member.class, member.name));
        }
        if member.name.starts_with("$SwitchMap$") {
            return Ok(Value::SwitchMap(member.class.clone(), member.name.clone()));
        }
        if member.descriptor.starts_with('L') {
            return Ok(Value::Constant(member.class.clone(), member.name.clone()));
        }
        Ok(Value::Opaque)
    }

    fn dye_values(&self) -> Vec<Value> {
        self.dyes
            .iter()
            .map(|dye| Value::Constant(DYE_COLOR_CLASS.to_owned(), dye.name.clone()))
            .collect()
    }

    fn dye(&self, name: &str) -> Result<&Dye> {
        self.dyes
            .iter()
            .find(|dye| dye.name == name)
            .ok_or_else(|| anyhow!("no dye colour {name}"))
    }

    fn invoke(
        &mut self,
        member: &Member,
        mut arguments: Vec<Value>,
        depth: usize,
    ) -> Result<Value> {
        let functional = member.class.starts_with("java/util/function/")
            || member
                .class
                .starts_with("org/apache/commons/lang3/function/");
        if functional {
            if let Some(Value::Closure(closure)) = arguments.first() {
                let closure = closure.clone();
                arguments.remove(0);
                return self.apply(&closure, arguments, depth);
            }
        }
        match member.class.as_str() {
            BLOCKS_CLASS if member.name == "register" => self.register(arguments),
            BLOCKS_CLASS => self.run(
                BLOCKS_CLASS,
                &member.name,
                &member.descriptor,
                arguments,
                depth + 1,
            ),
            PROPERTIES_CLASS => self.properties_call(member, arguments),
            DYE_COLOR_CLASS => self.dye_call(member, arguments),
            COLOR_COLLECTION_CLASS => self.color_collection_call(member, arguments, depth),
            COPPER_COLLECTION_CLASS | COPPER_STATES_CLASS => {
                self.copper_call(member, arguments, depth)
            }
            BLOCK_ITEM_ID_CLASS if member.name == "block" => {
                Ok(arguments.into_iter().next().unwrap_or(Value::Opaque))
            }
            BLOCK_STATE_CLASS if member.name == "getValue" => self.setting(arguments, depth),
            "java/lang/Integer" if member.name == "intValue" => match arguments.first() {
                Some(Value::Setting(value)) => Ok(Value::Int(
                    value
                        .parse()
                        .with_context(|| format!("{value} is not a number"))?,
                )),
                other => bail!("intValue on {other:?}"),
            },
            "java/lang/Boolean" if member.name == "booleanValue" => match arguments.first() {
                Some(Value::Setting(value)) => Ok(Value::Int(i32::from(value == "true"))),
                other => bail!("booleanValue on {other:?}"),
            },
            _ => Ok(match (member.name.as_str(), arguments.first()) {
                ("defaultMapColor", Some(Value::Block(block))) => {
                    Value::Color(Color::DefaultOf(*block))
                }
                ("ordinal", Some(Value::Constant(_, name))) => Value::Ordinal(name.clone()),
                _ => Value::Opaque,
            }),
        }
    }

    fn apply(&mut self, closure: &Closure, arguments: Vec<Value>, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            bail!("calls nest deeper than {MAX_DEPTH}");
        }
        let mut all = closure.captured.clone();
        all.extend(arguments);
        match closure.handle.kind {
            REF_INVOKE_VIRTUAL | REF_INVOKE_STATIC | REF_INVOKE_INTERFACE => {
                self.invoke(&closure.handle.member, all, depth + 1)
            }
            _ => Ok(Value::Opaque),
        }
    }

    fn register(&mut self, arguments: Vec<Value>) -> Result<Value> {
        let name = match arguments.first() {
            Some(Value::Id(name)) => name.clone(),
            _ => None,
        };
        let rule = match arguments.last() {
            Some(Value::Props(props)) => self.props[*props].clone(),
            other => bail!("a block is registered with {other:?} instead of properties"),
        };
        self.registered.push(Registered { name, rule });
        Ok(Value::Block(self.registered.len() - 1))
    }

    fn new_props(&mut self, rule: Rule) -> Value {
        self.props.push(rule);
        Value::Props(self.props.len() - 1)
    }

    fn properties_call(&mut self, member: &Member, arguments: Vec<Value>) -> Result<Value> {
        match member.name.as_str() {
            "of" => Ok(self.new_props(Rule::Unset)),
            "ofLegacyCopy" | "ofFullCopy" => match arguments.first() {
                Some(Value::Block(block)) => {
                    let rule = self.registered[*block].rule.clone();
                    Ok(self.new_props(rule))
                }
                other => bail!("{} of {other:?}", member.name),
            },
            "mapColor" => {
                let [Value::Props(props), color] = arguments.as_slice() else {
                    bail!("mapColor with {arguments:?}");
                };
                let rule = match color {
                    Value::Color(color) => Rule::Fixed(color.clone()),
                    Value::Constant(class, dye) if class == DYE_COLOR_CLASS => {
                        Rule::Fixed(Color::Named(self.dye(dye)?.map.clone()))
                    }
                    Value::Closure(closure) => Rule::ByState(closure.clone()),
                    other => bail!("mapColor with {other:?}"),
                };
                self.props[*props] = rule;
                Ok(Value::Props(*props))
            }
            _ => {
                let builds = member
                    .descriptor
                    .ends_with(&format!(")L{PROPERTIES_CLASS};"));
                Ok(match arguments.first() {
                    Some(Value::Props(props)) if builds => Value::Props(*props),
                    _ => Value::Opaque,
                })
            }
        }
    }

    fn dye_call(&mut self, member: &Member, arguments: Vec<Value>) -> Result<Value> {
        let Some(Value::Constant(_, name)) = arguments.first() else {
            return Ok(Value::Opaque);
        };
        let dye = self.dye(name)?;
        Ok(match member.name.as_str() {
            "getMapColor" => Value::Color(Color::Named(dye.map.clone())),
            "getTerracottaColor" => Value::Color(Color::Named(dye.terracotta.clone())),
            "ordinal" => Value::Ordinal(name.clone()),
            _ => Value::Opaque,
        })
    }

    fn color_collection_call(
        &mut self,
        member: &Member,
        arguments: Vec<Value>,
        depth: usize,
    ) -> Result<Value> {
        let dyes = self.dyes.len();
        match member.name.as_str() {
            "registerBlocks" => {
                let ids = group(arguments.first(), dyes)?;
                let register = closure(arguments.get(1))?;
                let props = closure(arguments.last())?;
                let mut out = Vec::with_capacity(dyes);
                for (dye, id) in self.dye_values().into_iter().zip(ids.iter()) {
                    let properties = self.apply(&props, vec![dye], depth)?;
                    out.push(self.apply(
                        &register,
                        vec![id.clone(), Value::Opaque, properties],
                        depth,
                    )?);
                }
                Ok(Value::Colors(Rc::new(out)))
            }
            "zipMap" => {
                let left = group(arguments.first(), dyes)?;
                let right = group(arguments.get(1), dyes)?;
                let function = closure(arguments.get(2))?;
                let mut out = Vec::with_capacity(dyes);
                for (left, right) in left.iter().zip(right.iter()) {
                    out.push(self.apply(&function, vec![left.clone(), right.clone()], depth)?);
                }
                Ok(Value::Colors(Rc::new(out)))
            }
            "map" => {
                let values = group(arguments.first(), dyes)?;
                let function = closure(arguments.get(1))?;
                let mut out = Vec::with_capacity(dyes);
                for value in values.iter() {
                    out.push(self.apply(&function, vec![value.clone()], depth)?);
                }
                Ok(Value::Colors(Rc::new(out)))
            }
            "pick" => {
                let values = group(arguments.first(), dyes)?;
                let at = match arguments.get(1) {
                    Some(Value::Constant(_, name)) => {
                        self.dyes.iter().position(|dye| dye.name == *name)
                    }
                    _ => None,
                }
                .ok_or_else(|| anyhow!("pick with {:?}", arguments.get(1)))?;
                Ok(values[at].clone())
            }
            _ => Ok(Value::Opaque),
        }
    }

    fn copper_call(
        &mut self,
        member: &Member,
        arguments: Vec<Value>,
        depth: usize,
    ) -> Result<Value> {
        let states = WEATHER_STATES.len();
        match (member.class.as_str(), member.name.as_str()) {
            (COPPER_COLLECTION_CLASS, "registerBlocks") => {
                let ids = group(arguments.first(), 2 * states)?;
                let register = closure(arguments.get(1))?;
                let props = closure(arguments.last())?;
                let mut out = Vec::with_capacity(ids.len());
                for (at, id) in ids.iter().enumerate() {
                    let state = Value::Constant(
                        WEATHER_STATE_CLASS.to_owned(),
                        WEATHER_STATES[at % states].to_owned(),
                    );
                    let properties = self.apply(&props, vec![state], depth)?;
                    out.push(self.apply(
                        &register,
                        vec![id.clone(), Value::Opaque, properties],
                        depth,
                    )?);
                }
                Ok(Value::Copper(Rc::new(out)))
            }
            (COPPER_COLLECTION_CLASS, "weathering" | "waxed") => {
                let all = group(arguments.first(), 2 * states)?;
                let half = if member.name == "waxed" {
                    &all[states..]
                } else {
                    &all[..states]
                };
                Ok(Value::ByWeather(Rc::new(half.to_vec())))
            }
            (COPPER_STATES_CLASS, "pick") => {
                let values = group(arguments.first(), states)?;
                let at = match arguments.get(1) {
                    Some(Value::Constant(_, name)) => {
                        WEATHER_STATES.iter().position(|state| state == name)
                    }
                    _ => None,
                }
                .ok_or_else(|| anyhow!("pick with {:?}", arguments.get(1)))?;
                Ok(values[at].clone())
            }
            _ => Ok(Value::Opaque),
        }
    }

    fn setting(&mut self, arguments: Vec<Value>, depth: usize) -> Result<Value> {
        let [Value::State(state), Value::Constant(owner, field)] = arguments.as_slice() else {
            bail!("getValue with {arguments:?}");
        };
        let property = self.property_name(owner, field, depth)?;
        self.states
            .setting(*state, &property)
            .map(Value::Setting)
            .ok_or_else(|| anyhow!("state {state} has no property {property}"))
    }

    fn property_name(&mut self, owner: &str, field: &str, depth: usize) -> Result<String> {
        let key = (owner.to_owned(), field.to_owned());
        if let Some(name) = self.properties.get(&key) {
            return Ok(name.clone());
        }
        if depth > MAX_DEPTH {
            bail!("the property {owner}.{field} is aliased too often");
        }
        let class = self.class(owner)?;
        let name = match assignment(&class, field)? {
            Some(Assignment::Alias(source)) => {
                self.property_name(&source.class, &source.name, depth + 1)?
            }
            Some(Assignment::Named(name)) => name,
            None => match &class.super_name {
                Some(parent) => self.property_name(parent, field, depth + 1)?,
                None => bail!("no property {owner}.{field}"),
            },
        };
        self.properties.insert(key, name.clone());
        Ok(name)
    }

    fn switch_case(&mut self, owner: &str, field: &str, constant: &str) -> Result<i32> {
        let key = (owner.to_owned(), field.to_owned());
        if !self.switches.contains_key(&key) {
            let class = self.class(owner)?;
            let cases = switch_cases(&class, field)?;
            self.switches.insert(key.clone(), cases);
        }
        self.switches[&key]
            .get(constant)
            .copied()
            .ok_or_else(|| anyhow!("{owner}.{field} has no case for {constant}"))
    }

    fn color_at(&mut self, block: usize, state: u16, depth: usize) -> Result<String> {
        if depth > MAX_DEPTH {
            bail!("map colours refer to each other too deeply");
        }
        let rule = self
            .registered
            .get(block)
            .map(|registered| registered.rule.clone())
            .ok_or_else(|| anyhow!("no block {block}"))?;
        let color = match rule {
            Rule::Unset => return Ok(NONE.to_owned()),
            Rule::Fixed(color) => color,
            Rule::ByState(closure) => {
                match self.apply(&closure, vec![Value::State(state)], depth)? {
                    Value::Color(color) => color,
                    other => bail!("the map colour of state {state} is {other:?}"),
                }
            }
        };
        self.color_name(&color, depth + 1)
    }

    fn color_name(&mut self, color: &Color, depth: usize) -> Result<String> {
        match color {
            Color::Named(name) => Ok(name.clone()),
            Color::DefaultOf(block) => {
                if let Some(name) = self.defaults.get(block) {
                    return Ok(name.clone());
                }
                let state = self.states.default_of(*block);
                let name = self.color_at(*block, state, depth)?;
                self.defaults.insert(*block, name.clone());
                Ok(name)
            }
        }
    }
}

fn member(class: &ClassFile, index: u16) -> Result<Member> {
    class
        .member(index)
        .ok_or_else(|| anyhow!("constant {index} is not a member"))
}

fn group(value: Option<&Value>, size: usize) -> Result<Rc<Vec<Value>>> {
    match value {
        Some(Value::Colors(values) | Value::Copper(values) | Value::ByWeather(values))
            if values.len() == size =>
        {
            Ok(values.clone())
        }
        other => bail!("expected a collection of {size}, found {other:?}"),
    }
}

fn closure(value: Option<&Value>) -> Result<Rc<Closure>> {
    match value {
        Some(Value::Closure(closure)) => Ok(closure.clone()),
        other => bail!("expected a lambda, found {other:?}"),
    }
}

fn same(left: &Value, right: &Value) -> Result<bool> {
    Ok(match (left, right) {
        (Value::Setting(value), Value::Constant(_, name))
        | (Value::Constant(_, name), Value::Setting(value)) => value.eq_ignore_ascii_case(name),
        (Value::Constant(left_class, left), Value::Constant(right_class, right)) => {
            left_class == right_class && left == right
        }
        (Value::Block(left), Value::Block(right)) => left == right,
        (Value::Null, Value::Null) => true,
        (Value::Null, other) | (other, Value::Null) if !matches!(other, Value::Opaque) => false,
        _ => bail!("cannot compare {left:?} with {right:?}"),
    })
}

fn static_init(class: &ClassFile) -> Result<Vec<Instruction>> {
    match class
        .method("<clinit>")
        .and_then(|method| method.code.as_deref())
    {
        Some(code) => class_file::instructions(code),
        None => Ok(Vec::new()),
    }
}

fn dye_table(class: &ClassFile) -> Result<Vec<Dye>> {
    let own = format!("L{};", class.name);
    let mut colors: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for instruction in static_init(class)? {
        match instruction.op {
            class_file::GETSTATIC => {
                if let Some(member) = class.member(instruction.index) {
                    if member.class == MAP_COLOR_CLASS {
                        colors.push(member.name);
                    }
                }
            }
            class_file::PUTSTATIC => {
                if let Some(member) = class.member(instruction.index) {
                    if member.descriptor == own && colors.len() >= 2 {
                        out.push(Dye {
                            name: member.name,
                            map: colors[0].clone(),
                            terracotta: colors[1].clone(),
                        });
                    }
                }
                colors.clear();
            }
            _ => {}
        }
    }
    if out.is_empty() {
        bail!("{} makes no dye colours", class.name);
    }
    Ok(out)
}

fn id_table(class: &ClassFile, dyes: &[Dye]) -> Result<Vec<(String, Value)>> {
    let id = |path: Option<String>| Value::Id(path.map(|path| format!("minecraft:{path}")));
    let mut strings: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for instruction in static_init(class)? {
        match instruction.op {
            class_file::LDC | class_file::LDC_W => {
                if let Some(text) = class.string(instruction.index) {
                    strings.push(text.to_owned());
                }
            }
            class_file::PUTSTATIC => {
                let first = strings.first().cloned();
                strings.clear();
                let Some(member) = class.member(instruction.index) else {
                    continue;
                };
                let value = match member.descriptor.as_str() {
                    BLOCK_ITEM_ID | RESOURCE_KEY => id(first),
                    COLOR_COLLECTION => Value::Colors(Rc::new(
                        dyes.iter()
                            .map(|dye| {
                                let name = dye.name.to_ascii_lowercase();
                                id(first.as_ref().map(|base| format!("{name}_{base}")))
                            })
                            .collect(),
                    )),
                    COPPER_COLLECTION => {
                        Value::Copper(Rc::new(vec![id(None); 2 * WEATHER_STATES.len()]))
                    }
                    _ => continue,
                };
                out.push((member.name, value));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn assignment(class: &ClassFile, field: &str) -> Result<Option<Assignment>> {
    let mut strings: Vec<String> = Vec::new();
    let mut last: Option<Instruction> = None;
    for instruction in static_init(class)? {
        match instruction.op {
            class_file::LDC | class_file::LDC_W => {
                if let Some(text) = class.string(instruction.index) {
                    strings.push(text.to_owned());
                }
            }
            class_file::PUTSTATIC => {
                let assigns = class
                    .member(instruction.index)
                    .is_some_and(|member| member.class == class.name && member.name == field);
                if assigns {
                    let alias = last
                        .filter(|previous| previous.op == class_file::GETSTATIC)
                        .and_then(|previous| class.member(previous.index));
                    if let Some(source) = alias {
                        return Ok(Some(Assignment::Alias(source)));
                    }
                    return match strings.first() {
                        Some(name) => Ok(Some(Assignment::Named(name.clone()))),
                        None => bail!("{}.{field} is not a named property", class.name),
                    };
                }
                strings.clear();
            }
            _ => {}
        }
        last = Some(instruction);
    }
    Ok(None)
}

fn switch_cases(class: &ClassFile, field: &str) -> Result<HashMap<String, i32>> {
    let instructions = static_init(class)?;
    let mut out = HashMap::new();
    for window in instructions.windows(5) {
        let [map, constant, ordinal, case, store] = window else {
            continue;
        };
        let reads_map = map.op == class_file::GETSTATIC
            && class
                .member(map.index)
                .is_some_and(|member| member.class == class.name && member.name == field);
        let takes_ordinal = ordinal.op == class_file::INVOKEVIRTUAL
            && class
                .member(ordinal.index)
                .is_some_and(|member| member.name == "ordinal");
        let pushes_case = matches!(
            case.op,
            class_file::ICONST_M1..=class_file::ICONST_5 | class_file::BIPUSH | class_file::SIPUSH
        );
        if !reads_map
            || constant.op != class_file::GETSTATIC
            || !takes_ordinal
            || !pushes_case
            || store.op != IASTORE
        {
            continue;
        }
        if let Some(member) = class.member(constant.index) {
            out.insert(member.name, case.value);
        }
    }
    if out.is_empty() {
        bail!("{}.{field} is not a switch map", class.name);
    }
    Ok(out)
}

fn switch_target(code: &[u8], at: usize, key: i32) -> Result<i32> {
    let base = (at + 4) & !3;
    let int = |offset: usize| -> Result<i32> {
        let bytes = code
            .get(offset..offset + 4)
            .ok_or_else(|| anyhow!("the switch at {at} is cut off"))?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    };
    let default = int(base)?;
    let offset = match code.get(at).copied() {
        Some(class_file::TABLESWITCH) => {
            let low = int(base + 4)?;
            let high = int(base + 8)?;
            if (low..=high).contains(&key) {
                int(base + 12 + (key - low) as usize * 4)?
            } else {
                default
            }
        }
        Some(class_file::LOOKUPSWITCH) => {
            let pairs = usize::try_from(int(base + 4)?).context("a negative switch size")?;
            let mut found = default;
            for pair in 0..pairs {
                if int(base + 8 + pair * 8)? == key {
                    found = int(base + 12 + pair * 8)?;
                    break;
                }
            }
            found
        }
        other => bail!("no switch at {at}: {other:?}"),
    };
    Ok(at as i32 + offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class_file::{Bootstrap, Method};

    const PROPS: &str = "Lnet/minecraft/world/level/block/state/BlockBehaviour$Properties;";
    const MAP_COLOR: &str = "Lnet/minecraft/world/level/material/MapColor;";
    const BLOCK: &str = "Lnet/minecraft/world/level/block/Block;";
    const LOG_CLASS: &str = "test/Log";
    const PILLAR_CLASS: &str = "test/Pillar";
    const STATE_PROPERTIES_CLASS: &str = "test/StateProperties";
    const PROPERTY: &str = "Ltest/Property;";
    const ALOAD_0: u8 = 0x2a;
    const ALOAD_1: u8 = 0x2b;
    const ALOAD_2: u8 = 0x2c;

    #[derive(Default)]
    struct Pool(Vec<Constant>);

    impl Pool {
        fn add(&mut self, constant: Constant) -> u16 {
            if let Some(at) = self.0.iter().skip(1).position(|c| *c == constant) {
                return at as u16 + 1;
            }
            if self.0.is_empty() {
                self.0.push(Constant::Unused);
            }
            self.0.push(constant);
            (self.0.len() - 1) as u16
        }

        fn utf8(&mut self, text: &str) -> u16 {
            self.add(Constant::Utf8(text.to_owned()))
        }

        fn class(&mut self, name: &str) -> u16 {
            let name = self.utf8(name);
            self.add(Constant::Class(name))
        }

        fn string(&mut self, text: &str) -> u16 {
            let text = self.utf8(text);
            self.add(Constant::String(text))
        }

        fn nat(&mut self, name: &str, descriptor: &str) -> u16 {
            let name = self.utf8(name);
            let descriptor = self.utf8(descriptor);
            self.add(Constant::NameAndType(name, descriptor))
        }

        fn field(&mut self, class: &str, name: &str, descriptor: &str) -> u16 {
            let class = self.class(class);
            let nat = self.nat(name, descriptor);
            self.add(Constant::Field(class, nat))
        }

        fn method(&mut self, class: &str, name: &str, descriptor: &str) -> u16 {
            let class = self.class(class);
            let nat = self.nat(name, descriptor);
            self.add(Constant::Method(class, nat))
        }

        fn handle(&mut self, class: &str, name: &str, descriptor: &str) -> u16 {
            let method = self.method(class, name, descriptor);
            self.add(Constant::MethodHandle(REF_INVOKE_STATIC, method))
        }

        fn dynamic(&mut self, bootstrap: u16, descriptor: &str) -> u16 {
            let nat = self.nat("apply", descriptor);
            self.add(Constant::InvokeDynamic(bootstrap, nat))
        }
    }

    #[derive(Default)]
    struct Asm(Vec<u8>);

    impl Asm {
        fn op(&mut self, op: u8) -> &mut Self {
            self.0.push(op);
            self
        }

        fn wide(&mut self, op: u8, index: u16) -> &mut Self {
            self.0.push(op);
            self.0.extend(index.to_be_bytes());
            self
        }

        fn dynamic(&mut self, index: u16) -> &mut Self {
            self.wide(class_file::INVOKEDYNAMIC, index);
            self.0.extend([0, 0]);
            self
        }
    }

    fn method(name: &str, descriptor: &str, code: &Asm) -> Method {
        Method {
            access: ACC_STATIC,
            name: name.to_owned(),
            descriptor: descriptor.to_owned(),
            max_locals: 4,
            code: Some(code.0.clone()),
        }
    }

    fn class(
        name: &str,
        parent: Option<&str>,
        pool: Pool,
        methods: Vec<Method>,
        bootstraps: Vec<Bootstrap>,
    ) -> ClassFile {
        ClassFile {
            name: name.to_owned(),
            super_name: parent.map(str::to_owned),
            pool: pool.0,
            fields: Vec::new(),
            methods,
            bootstraps,
        }
    }

    fn statics(name: &str, parent: Option<&str>, build: impl Fn(&mut Pool, &mut Asm)) -> ClassFile {
        let mut pool = Pool::default();
        let mut code = Asm::default();
        build(&mut pool, &mut code);
        code.op(RETURN);
        let init = method("<clinit>", "()V", &code);
        class(name, parent, pool, vec![init], Vec::new())
    }

    fn blocks() -> ClassFile {
        let mut pool = Pool::default();
        let factory = pool.handle(
            LAMBDA_FACTORY_CLASS,
            "metafactory",
            "(Ljava/lang/invoke/MethodHandles$Lookup;)Ljava/lang/invoke/CallSite;",
        );
        let shape = pool.utf8("()V");
        let lambda = |pool: &mut Pool, name: &str, descriptor: &str| Bootstrap {
            method: factory,
            arguments: vec![shape, pool.handle(BLOCKS_CLASS, name, descriptor), shape],
        };
        let register_descriptor =
            format!("({BLOCK_ITEM_ID}Ljava/util/function/Function;{PROPS}){BLOCK}");
        let log_lambda = format!("({MAP_COLOR}{MAP_COLOR}L{BLOCK_STATE_CLASS};){MAP_COLOR}");
        let wool_lambda = format!("(L{DYE_COLOR_CLASS};){PROPS}");
        let bootstraps = vec![
            lambda(&mut pool, "lambda$log", &log_lambda),
            lambda(&mut pool, "register", &register_descriptor),
            lambda(&mut pool, "lambda$wool", &wool_lambda),
        ];
        let of = pool.method(PROPERTIES_CLASS, "of", &format!("(){PROPS}"));
        let fixed = pool.method(
            PROPERTIES_CLASS,
            "mapColor",
            &format!("({MAP_COLOR}){PROPS}"),
        );
        let by_state = pool.method(
            PROPERTIES_CLASS,
            "mapColor",
            &format!("(Ljava/util/function/Function;){PROPS}"),
        );
        let by_dye = pool.method(
            PROPERTIES_CLASS,
            "mapColor",
            &format!("(L{DYE_COLOR_CLASS};){PROPS}"),
        );
        let no_occlusion = pool.method(PROPERTIES_CLASS, "noOcclusion", &format!("(){PROPS}"));
        let legacy_copy = pool.method(
            PROPERTIES_CLASS,
            "ofLegacyCopy",
            &format!("(Lnet/minecraft/world/level/block/state/BlockBehaviour;){PROPS}"),
        );
        let register = pool.method(BLOCKS_CLASS, "register", &register_descriptor);
        let default_color = pool.method(
            "net/minecraft/world/level/block/Block",
            "defaultMapColor",
            &format!("(){MAP_COLOR}"),
        );
        let log_props = format!("({MAP_COLOR}{MAP_COLOR}){PROPS}");
        let log_properties = pool.method(BLOCKS_CLASS, "logProperties", &log_props);
        let register_colors = pool.method(
            COLOR_COLLECTION_CLASS,
            "registerBlocks",
            &format!(
                "({COLOR_COLLECTION}Lorg/apache/commons/lang3/function/TriFunction;Ljava/util/function/BiFunction;Ljava/util/function/Function;){COLOR_COLLECTION}"
            ),
        );
        let get_value = pool.method(
            BLOCK_STATE_CLASS,
            "getValue",
            "(Lnet/minecraft/world/level/block/state/properties/Property;)Ljava/lang/Comparable;",
        );
        let log_dynamic = pool.dynamic(
            0,
            &format!("({MAP_COLOR}{MAP_COLOR})Ljava/util/function/Function;"),
        );
        let register_dynamic = pool.dynamic(1, "()Lorg/apache/commons/lang3/function/TriFunction;");
        let wool_dynamic = pool.dynamic(2, "()Ljava/util/function/Function;");
        let color = |pool: &mut Pool, name: &str| pool.field(MAP_COLOR_CLASS, name, MAP_COLOR);
        let id = |pool: &mut Pool, name: &str, descriptor: &str| {
            pool.field(BLOCK_ITEM_IDS_CLASS, name, descriptor)
        };
        let own = |pool: &mut Pool, name: &str, descriptor: &str| {
            pool.field(BLOCKS_CLASS, name, descriptor)
        };
        let mut init = Asm::default();
        init.wide(class_file::GETSTATIC, id(&mut pool, "STONE", BLOCK_ITEM_ID))
            .op(ACONST_NULL)
            .wide(class_file::INVOKESTATIC, of)
            .wide(class_file::GETSTATIC, color(&mut pool, "STONE"))
            .wide(class_file::INVOKEVIRTUAL, fixed)
            .wide(class_file::INVOKESTATIC, register)
            .wide(class_file::PUTSTATIC, own(&mut pool, "STONE", BLOCK));
        init.wide(
            class_file::GETSTATIC,
            id(&mut pool, "STONE_SLAB", BLOCK_ITEM_ID),
        )
        .op(ACONST_NULL)
        .wide(class_file::GETSTATIC, own(&mut pool, "STONE", BLOCK))
        .wide(class_file::INVOKESTATIC, legacy_copy)
        .wide(class_file::INVOKEVIRTUAL, no_occlusion)
        .wide(class_file::INVOKESTATIC, register)
        .op(POP);
        init.wide(
            class_file::GETSTATIC,
            id(&mut pool, "OAK_LOG", BLOCK_ITEM_ID),
        )
        .op(ACONST_NULL)
        .wide(class_file::GETSTATIC, color(&mut pool, "WOOD"))
        .wide(class_file::GETSTATIC, color(&mut pool, "PODZOL"))
        .wide(class_file::INVOKESTATIC, log_properties)
        .wide(class_file::INVOKESTATIC, register)
        .wide(class_file::PUTSTATIC, own(&mut pool, "OAK_LOG", BLOCK));
        init.wide(
            class_file::GETSTATIC,
            id(&mut pool, "OAK_PLANKS", BLOCK_ITEM_ID),
        )
        .op(ACONST_NULL)
        .wide(class_file::INVOKESTATIC, of)
        .wide(class_file::GETSTATIC, own(&mut pool, "OAK_LOG", BLOCK))
        .wide(class_file::INVOKEVIRTUAL, default_color)
        .wide(class_file::INVOKEVIRTUAL, fixed)
        .wide(class_file::INVOKESTATIC, register)
        .op(POP);
        init.wide(
            class_file::GETSTATIC,
            id(&mut pool, "WOOL", COLOR_COLLECTION),
        )
        .dynamic(register_dynamic)
        .op(ACONST_NULL)
        .dynamic(wool_dynamic)
        .wide(class_file::INVOKESTATIC, register_colors)
        .wide(
            class_file::PUTSTATIC,
            own(&mut pool, "WOOL", COLOR_COLLECTION),
        )
        .op(RETURN);
        let mut log = Asm::default();
        log.wide(class_file::INVOKESTATIC, of)
            .op(ALOAD_0)
            .op(ALOAD_1)
            .dynamic(log_dynamic)
            .wide(class_file::INVOKEVIRTUAL, by_state)
            .op(ARETURN);
        let mut axis = Asm::default();
        axis.op(ALOAD_2)
            .wide(
                class_file::GETSTATIC,
                pool.field(LOG_CLASS, "AXIS", PROPERTY),
            )
            .wide(class_file::INVOKEVIRTUAL, get_value)
            .wide(
                class_file::GETSTATIC,
                pool.field("test/Axis", "Y", "Ltest/Axis;"),
            )
            .wide(IF_ACMPNE, 5)
            .op(ALOAD_0)
            .op(ARETURN)
            .op(ALOAD_1)
            .op(ARETURN);
        let mut wool = Asm::default();
        wool.wide(class_file::INVOKESTATIC, of)
            .op(ALOAD_0)
            .wide(class_file::INVOKEVIRTUAL, by_dye)
            .op(ARETURN);
        let methods = vec![
            method("<clinit>", "()V", &init),
            method("logProperties", &log_props, &log),
            method("lambda$log", &log_lambda, &axis),
            method("lambda$wool", &wool_lambda, &wool),
        ];
        class(
            BLOCKS_CLASS,
            Some("java/lang/Object"),
            pool,
            methods,
            bootstraps,
        )
    }

    fn jar() -> HashMap<String, ClassFile> {
        let dyes = statics(DYE_COLOR_CLASS, None, |pool, code| {
            let own = format!("L{DYE_COLOR_CLASS};");
            for (dye, map, terracotta) in [
                ("WHITE", "SNOW", "TERRACOTTA_WHITE"),
                ("RED", "COLOR_RED", "TERRACOTTA_RED"),
            ] {
                code.wide(
                    class_file::GETSTATIC,
                    pool.field(MAP_COLOR_CLASS, map, MAP_COLOR),
                )
                .wide(
                    class_file::GETSTATIC,
                    pool.field(MAP_COLOR_CLASS, terracotta, MAP_COLOR),
                )
                .wide(
                    class_file::PUTSTATIC,
                    pool.field(DYE_COLOR_CLASS, dye, &own),
                );
            }
        });
        let ids = statics(BLOCK_ITEM_IDS_CLASS, None, |pool, code| {
            for (field, path, descriptor) in [
                ("STONE", "stone", BLOCK_ITEM_ID),
                ("STONE_SLAB", "stone_slab", BLOCK_ITEM_ID),
                ("OAK_LOG", "oak_log", BLOCK_ITEM_ID),
                ("OAK_PLANKS", "oak_planks", BLOCK_ITEM_ID),
                ("WOOL", "wool", COLOR_COLLECTION),
            ] {
                code.wide(class_file::LDC_W, pool.string(path)).wide(
                    class_file::PUTSTATIC,
                    pool.field(BLOCK_ITEM_IDS_CLASS, field, descriptor),
                );
            }
        });
        let pillar = statics(PILLAR_CLASS, None, |pool, code| {
            code.wide(
                class_file::GETSTATIC,
                pool.field(STATE_PROPERTIES_CLASS, "AXIS", PROPERTY),
            )
            .wide(
                class_file::PUTSTATIC,
                pool.field(PILLAR_CLASS, "AXIS", PROPERTY),
            );
        });
        let named = statics(STATE_PROPERTIES_CLASS, None, |pool, code| {
            code.wide(class_file::LDC_W, pool.string("axis"))
                .wide(class_file::LDC_W, pool.class("test/Axis"))
                .wide(
                    class_file::PUTSTATIC,
                    pool.field(STATE_PROPERTIES_CLASS, "AXIS", PROPERTY),
                );
        });
        let log = class(
            LOG_CLASS,
            Some(PILLAR_CLASS),
            Pool::default(),
            Vec::new(),
            Vec::new(),
        );
        [
            blocks(),
            dyes,
            ids,
            statics(BLOCK_IDS_CLASS, None, |_, _| {}),
            pillar,
            named,
            log,
        ]
        .into_iter()
        .map(|class| (class.name.clone(), class))
        .collect()
    }

    struct Catalog {
        blocks: Vec<(&'static str, Vec<u16>, u16)>,
        axes: HashMap<u16, &'static str>,
    }

    impl Catalog {
        fn new() -> Self {
            Catalog {
                blocks: vec![
                    ("minecraft:stone", vec![0], 0),
                    ("minecraft:stone_slab", vec![1], 1),
                    ("minecraft:oak_log", vec![2, 3, 4], 3),
                    ("minecraft:oak_planks", vec![5], 5),
                    ("minecraft:white_wool", vec![6], 6),
                    ("minecraft:red_wool", vec![7], 7),
                ],
                axes: HashMap::from([(2, "x"), (3, "y"), (4, "z")]),
            }
        }
    }

    impl BlockStates for Catalog {
        fn blocks(&self) -> usize {
            self.blocks.len()
        }

        fn states(&self) -> usize {
            self.blocks.iter().map(|(_, states, _)| states.len()).sum()
        }

        fn name(&self, block: usize) -> String {
            self.blocks[block].0.to_owned()
        }

        fn states_of(&self, block: usize) -> Vec<u16> {
            self.blocks[block].1.clone()
        }

        fn default_of(&self, block: usize) -> u16 {
            self.blocks[block].2
        }

        fn setting(&self, state: u16, property: &str) -> Option<String> {
            (property == "axis")
                .then(|| self.axes.get(&state))
                .flatten()
                .map(|value| (*value).to_owned())
        }
    }

    fn palette() -> MapPalette {
        MapPalette::with_colors([
            (0, "NONE", 0),
            (8, "SNOW", 0xFFFFFF),
            (11, "STONE", 0x707070),
            (13, "WOOD", 0x8F7748),
            (28, "COLOR_RED", 0x993333),
            (34, "PODZOL", 0x815631),
        ])
    }

    fn read(classes: &HashMap<String, ClassFile>, catalog: &Catalog) -> Result<BlockMapColors> {
        let mut load = |name: &str| classes.get(name).cloned();
        BlockMapColors::read(&mut load, &palette(), catalog)
    }

    #[test]
    fn blocks_take_their_map_colour_from_their_properties_like_vanilla() {
        let colors = read(&jar(), &Catalog::new()).unwrap();
        let ids: Vec<u8> = (0..8).map(|state| colors.id(BlockStateId(state))).collect();
        assert_eq!(ids, [11, 11, 34, 13, 34, 13, 8, 28]);
        assert_eq!(colors.len(), 8);
        assert_eq!(colors.id(BlockStateId(99)), 0);
    }

    #[test]
    fn a_block_without_a_map_colour_is_none_and_the_catalog_must_match_the_jar() {
        let mut classes = jar();
        let mut catalog = Catalog::new();
        catalog.blocks[1].0 = "minecraft:andesite";
        let error = format!("{:#}", read(&classes, &catalog).unwrap_err());
        assert!(error.contains("minecraft:stone_slab"), "{error}");
        catalog = Catalog::new();
        catalog.blocks.pop();
        let error = format!("{:#}", read(&classes, &catalog).unwrap_err());
        assert!(error.contains("registers 6 blocks"), "{error}");
        let blocks = classes.get_mut(BLOCKS_CLASS).unwrap();
        let fixed = blocks
            .pool
            .iter()
            .position(|constant| *constant == Constant::Utf8("mapColor".to_owned()))
            .unwrap();
        blocks.pool[fixed] = Constant::Utf8("lightLevel".to_owned());
        let colors = read(&classes, &Catalog::new()).unwrap();
        assert_eq!(colors.id(BlockStateId(0)), 0);
        assert_eq!(colors.id(BlockStateId(3)), 0);
    }

    #[test]
    fn switches_jump_to_their_case_or_their_default() {
        let mut code = vec![0u8; 3];
        code.push(class_file::TABLESWITCH);
        code.extend(40i32.to_be_bytes());
        code.extend(1i32.to_be_bytes());
        code.extend(2i32.to_be_bytes());
        code.extend(10i32.to_be_bytes());
        code.extend(20i32.to_be_bytes());
        assert_eq!(switch_target(&code, 3, 1).unwrap(), 13);
        assert_eq!(switch_target(&code, 3, 2).unwrap(), 23);
        assert_eq!(switch_target(&code, 3, 7).unwrap(), 43);
        let mut code = vec![class_file::LOOKUPSWITCH, 0, 0, 0];
        code.extend(30i32.to_be_bytes());
        code.extend(2i32.to_be_bytes());
        code.extend(5i32.to_be_bytes());
        code.extend(8i32.to_be_bytes());
        code.extend(9i32.to_be_bytes());
        code.extend(12i32.to_be_bytes());
        assert_eq!(switch_target(&code, 0, 9).unwrap(), 12);
        assert_eq!(switch_target(&code, 0, 5).unwrap(), 8);
        assert_eq!(switch_target(&code, 0, 6).unwrap(), 30);
        assert!(switch_target(&code[..8], 0, 9).is_err());
    }

    #[test]
    fn every_26_3_block_state_gets_its_vanilla_map_colour() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = Path::new(&root).join("client.jar");
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        if !jar.is_file() || !paths.block_catalog.is_file() {
            return;
        }
        let blocks = BlockRegistry::load(&paths.block_catalog).unwrap();
        let colors = BlockMapColors::read_jar(&jar, &blocks).unwrap();
        assert_eq!(colors.len(), blocks.state_count());
        let id = |text: &str| colors.id(blocks.parse_state(text).unwrap());
        assert_eq!(id("minecraft:air"), 0);
        assert_eq!(id("minecraft:grass_block"), 1);
        assert_eq!(id("minecraft:sand"), 2);
        assert_eq!(id("minecraft:stone"), 11);
        assert_eq!(id("minecraft:water"), 12);
        assert_eq!(id("minecraft:oak_log[axis=y]"), 13);
        assert_eq!(id("minecraft:oak_log[axis=x]"), 34);
        assert_eq!(id("minecraft:oak_log[axis=z]"), 34);
        assert_eq!(id("minecraft:oak_planks"), 13);
        assert_eq!(id("minecraft:red_bed[part=head]"), 3);
        assert_eq!(id("minecraft:red_bed[part=foot]"), 28);
        assert_eq!(id("minecraft:white_wool"), 8);
        assert_eq!(id("minecraft:red_terracotta"), 50);
        assert_eq!(id("minecraft:wheat[age=0]"), 7);
        assert_eq!(id("minecraft:wheat[age=7]"), 18);
        assert_eq!(id("minecraft:copper_block"), 15);
        assert_eq!(id("minecraft:exposed_copper"), 44);
        assert_eq!(id("minecraft:oxidized_copper"), 55);
        assert_eq!(id("minecraft:waxed_oxidized_copper"), 55);
        assert_eq!(id("minecraft:oxidized_lightning_rod"), 55);
        assert_eq!(id("minecraft:waxed_exposed_lightning_rod"), 44);
        assert_eq!(id("minecraft:oxidized_copper_bars"), 0);
        assert_eq!(id("minecraft:iron_bars"), 0);
        assert_eq!(id("minecraft:barrier[waterlogged=true]"), 12);
        assert_eq!(id("minecraft:barrier[waterlogged=false]"), 0);
        assert_eq!(id("minecraft:light[waterlogged=true]"), 12);
        let mut checked = 0;
        for (_, block) in blocks.blocks() {
            for state in block.states() {
                assert!(usize::from(colors.id(state)) < 62);
                checked += 1;
            }
        }
        assert_eq!(checked, blocks.state_count());
    }
}
